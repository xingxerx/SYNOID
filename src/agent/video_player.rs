use std::io::Read;
use std::process::{Command, Stdio, Child};
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
    playing: bool,
}

impl VideoPlayer {
    pub fn new(path: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let width = 640;
        let height = 360;
        let fps = 30.0;

        let mut child = Command::new("ffmpeg")
            .arg("-i").arg(path)
            .arg("-f").arg("image2pipe")
            .arg("-pix_fmt").arg("rgb24")
            .arg("-vcodec").arg("rawvideo")
            .arg("-s").arg(format!("{}x{}", width, height))
            .arg("-r").arg(fps.to_string())
            .arg("-")
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;

        let mut stdout = child.stdout.take().expect("Failed to grab stdout");
        let (tx, rx) = sync_channel(5);

        let frame_size = width * height * 3;

        thread::spawn(move || {
            let mut buffer = vec![0u8; frame_size];
            loop {
                match stdout.read_exact(&mut buffer) {
                    Ok(_) => {
                        if tx.send(buffer.clone()).is_err() {
                            break; // receiver dropped
                        }
                    }
                    Err(_) => break, // EOF or error
                }
            }
        });

        // Also spawn a detatched audio player
        let _audio_process = Command::new("ffplay")
            .arg("-nodisp")
            .arg("-autoexit")
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
        let _ = Command::new("pkill").arg("ffplay").spawn();
        #[cfg(target_os = "windows")]
        let _ = Command::new("taskkill").arg("/F").arg("/IM").arg("ffplay.exe").spawn();
    }

    pub fn get_next_frame(&mut self) -> Option<&Vec<u8>> {
        if !self.playing {
            return self.current_frame.as_ref();
        }

        let now = Instant::now();
        let frame_duration = Duration::from_secs_f64(1.0 / self.fps);

        if let Some(last) = self.last_frame_time {
            if now.duration_since(last) < frame_duration {
                return self.current_frame.as_ref();
            }
        }

        match self.receiver.try_recv() {
            Ok(frame) => {
                self.current_frame = Some(frame);
                self.last_frame_time = Some(now);
                self.current_frame.as_ref()
            }
            Err(TryRecvError::Empty) => {
                // Wait for ffmpeg to catch up
                self.current_frame.as_ref()
            }
            Err(TryRecvError::Disconnected) => {
                self.playing = false;
                self.current_frame.as_ref()
            }
        }
    }
}

impl Drop for VideoPlayer {
    fn drop(&mut self) {
        self.stop();
    }
}
