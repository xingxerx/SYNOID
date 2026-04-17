use std::io::Read;
use std::process::{Child, Command, Stdio};
use crate::agent::engines::process_utils::CommandExt;
use std::sync::mpsc::{sync_channel, Receiver, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

pub struct VideoPlayer {
    receiver: Receiver<Vec<u8>>,
    process: Option<Child>,
    pub width: usize,
    pub height: usize,
    pub fps: f64,
    last_frame_time: Option<Instant>,
    current_frame: Option<Vec<u8>>,
    pub playing: bool,
}

impl VideoPlayer {
    pub fn new(
        path: &str,
        timestamp: f64,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let width = 640;
        let height = 360;
        let fps = 30.0;

        let mut child = Command::new("ffmpeg")
            .stealth()
            .arg("-hwaccel").arg("none")   // force software decode — hardware decoders can silently fail on piped raw output
            .arg("-ss")
            .arg(format!("{:.3}", timestamp))
            .arg("-i")
            .arg(path)
            .arg("-vf")
            .arg(format!("scale={}:{},format=rgb24", width, height))
            .arg("-r")
            .arg(fps.to_string())
            .arg("-f")
            .arg("rawvideo")
            .arg("-an")   // disable audio (handled by ffplay)
            .arg("-sn")   // disable subtitles
            .arg("-")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let mut stdout = child.stdout.take().expect("Failed to grab stdout");
        let stderr = child.stderr.take().expect("Failed to grab stderr");
        // Increase buffer to prevent frame drops if GUI is slow
        let (tx, rx) = sync_channel(60); // ~2 seconds buffer at 30fps

        let frame_size = width * height * 3;

        // Thread to read stderr - only log errors
        thread::spawn(move || {
            use std::io::BufRead;
            let mut err_reader = std::io::BufReader::new(stderr);
            let mut line = String::new();
            while let Ok(n) = err_reader.read_line(&mut line) {
                if n == 0 {
                    break;
                }
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    let lower = trimmed.to_lowercase();
                    // Log any error/warning from ffmpeg (case-insensitive)
                    if lower.contains("error") || lower.contains("fatal") || lower.contains("invalid")
                        || lower.contains("no such file") || lower.contains("permission denied")
                    {
                        tracing::error!("[VideoPlayer ffmpeg] {}", trimmed);
                    }
                }
                line.clear();
            }
        });

        thread::spawn(move || {
            let mut buffer = vec![0u8; frame_size];
            let mut frame_count = 0;
            loop {
                match stdout.read_exact(&mut buffer) {
                    Ok(_) => {
                        frame_count += 1;
                        if tx.send(buffer.clone()).is_err() {
                            break; // receiver dropped
                        }
                    }
                    Err(e) => {
                        if frame_count == 0 {
                            tracing::error!("[VideoPlayer] Failed to read first frame: {} - ffmpeg may have failed", e);
                        }
                        break; // EOF or error
                    }
                }
            }
        });

        // Also spawn a detatched audio player
        // Note: -vn disables video stream entirely, -sn disables subtitle processing
        let _audio_process = Command::new("ffplay")
            .stealth()
            .arg("-nodisp")
            .arg("-autoexit")
            .arg("-vn")        // Disable video stream (audio only)
            .arg("-sn")        // Disable subtitle processing to prevent console spam
            .arg("-ss")
            .arg(format!("{:.3}", timestamp))
            .arg(path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();

        Ok(Self {
            receiver: rx,
            process: Some(child),
            width,
            height,
            fps,
            last_frame_time: None,
            current_frame: None,
            playing: true,
        })
    }

    pub fn stop(&mut self) {
        if let Some(mut child) = self.process.take() {
            let _ = child.kill();
        }
        self.playing = false;
        // Kill ffplay instances just in case
        let _ = Command::new("pkill").stealth().arg("ffplay").spawn();
        #[cfg(target_os = "windows")]
        let _ = Command::new("taskkill")
            .stealth()
            .arg("/F")
            .arg("/IM")
            .arg("ffplay.exe")
            .spawn();
    }

    pub fn get_next_frame(&mut self) -> Option<(bool, &Vec<u8>)> {
        if !self.playing {
            return self.current_frame.as_ref().map(|f| (false, f));
        }

        let now = Instant::now();
        let frame_duration = Duration::from_secs_f64(1.0 / self.fps);

        // Always try to drain the receiver to prevent buffer buildup
        let mut got_new_frame = false;

        // Drain all available frames (keep the latest one)
        loop {
            match self.receiver.try_recv() {
                Ok(frame) => {
                    self.current_frame = Some(frame);
                    got_new_frame = true;
                }
                Err(TryRecvError::Empty) => {
                    break;
                }
                Err(TryRecvError::Disconnected) => {
                    self.playing = false;
                    break;
                }
            }
        }

        if got_new_frame {
            self.last_frame_time = Some(now);
            self.current_frame.as_ref().map(|f| (true, f))
        } else {
            // Check if we should wait based on frame timing
            if let Some(last) = self.last_frame_time {
                if now.duration_since(last) < frame_duration {
                    return self.current_frame.as_ref().map(|f| (false, f));
                }
            }
            // Return current frame even if no new frame (prevents black screen)
            self.current_frame.as_ref().map(|f| (false, f))
        }
    }
}

impl Drop for VideoPlayer {
    fn drop(&mut self) {
        self.stop();
    }
}
