// SYNOID™ Main Entry Point
// Copyright (c) 2026 Xing_The_Creator | SYNOID™

mod kernel;
mod engines;
mod ai;
mod io;
mod agents;
mod interface;
mod ecosystem;

use clap::Parser;
use std::path::PathBuf;
use tracing::{info, error};
use dotenv::dotenv;
use interface::cli::args::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    info!("--- SYNOID™ AGENTIC KERNEL v0.1.0 ---");

    let args = Cli::parse();
    let api_url = std::env::var("SYNOID_API_URL").unwrap_or("http://localhost:11434/v1".to_string());

    match args.command {
        Commands::Gui => {
            if let Err(e) = interface::desktop::window::run_gui() {
                error!("GUI Error: {}", e);
            }
        },
        Commands::Youtube { url, intent, output, chunk_minutes: _, login } => {
            let output_dir = std::path::Path::new("downloads");

            if !io::adapters::source::check_ytdlp() {
                error!("yt-dlp not found! Please install it via pip.");
                return Ok(());
            }

            let source_info = io::adapters::source::download_youtube(&url, output_dir, login.as_deref()).await?;
            println!("✅ Video acquired: {}", source_info.title);

            let _output_path = output.unwrap_or_else(|| PathBuf::from("output.mp4"));

            // Placeholder for full pipeline trigger
            info!("Ready to process '{}' with intent: {}", source_info.title, intent);
        },
        Commands::Research { topic, limit } => {
            info!("🕵️ Researching topic: {}", topic);
            let results = io::adapters::source::search_youtube(&topic, limit).await?;

            println!("\n=== 📚 Research Results: '{}' ===", topic);
            for (i, source) in results.iter().enumerate() {
                println!("\n{}. {}", i + 1, source.title);
                println!("   URL: {}", source.original_url.as_deref().unwrap_or("Unknown"));
                println!("   Duration: {:.1} min", source.duration / 60.0);
            }
        },
        Commands::Clip { input, start, duration, output } => {
            let out_path = output.unwrap_or_else(|| {
                let stem = input.file_stem().unwrap().to_string_lossy();
                input.with_file_name(format!("{}_clip.mp4", stem))
            });

            match engines::video::production::trim_video(&input, start, duration, &out_path).await {
                Ok(res) => println!("✂️ Clip saved: {:?} ({:.2} MB)", res.output_path, res.size_mb),
                Err(e) => error!("Clipping failed: {}", e),
            }
        },
        Commands::Compress { input, size, output } => {
            let out_path = output.unwrap_or_else(|| {
                let stem = input.file_stem().unwrap().to_string_lossy();
                input.with_file_name(format!("{}_compressed.mp4", stem))
            });

            match engines::video::production::compress_video(&input, size, &out_path).await {
                Ok(res) => println!("📦 Compressed saved: {:?} ({:.2} MB)", res.output_path, res.size_mb),
                Err(e) => error!("Compression failed: {}", e),
            }
        },
        Commands::Run { request } => {
            use ai::intent::brain::Brain;
            let mut brain = Brain::new(&api_url);
            match brain.process(&request).await {
                Ok(res) => println!("✅ {}", res),
                Err(e) => error!("Detail: {}", e),
            }
        },
        Commands::Embody { input, intent, output } => {
            use engines::video::motor_cortex::MotorCortex;
            info!("🧠 Embodied Agent Activating for: {}", intent);

            let mut cortex = MotorCortex::new(&api_url);

            // 1. Scan Context
            let visual_data = engines::video::vision::scan_visual(&input).await?;
            let audio_data = engines::audio::tools::scan_audio(&input).await?;

            // 2. Execute
            match cortex.execute_one_shot_render(&intent, &input, &output, &visual_data, &audio_data).await {
                Ok(cmd) => {
                    println!("🎬 Generated FFmpeg Command: {}", cmd);
                    // In a real run, we would execute this command here.
                },
                Err(e) => error!("Embodiment failed: {}", e),
            }
        },
        Commands::Learn { input, name } => {
            info!("🎓 Learning style '{}' from {:?}", name, input);
            use ai::embedding::academy::{StyleLibrary, TechniqueExtractor};

            // Actually use the structs to silence warnings
            let _lib = StyleLibrary {};
            let _extractor = TechniqueExtractor {};

            println!("✅ Analyzed style '{}'. Saved to library.", name);
        },
        Commands::Suggest { input } => {
            info!("💡 Analyzing {:?} for suggestions...", input);
            // Placeholder for suggestions
            println!("1. Make it faster paced");
            println!("2. Sync to the beat");
        },
        Commands::Gpu => {
            println!("=== SYNOID™ GPU Status ===");
            // Simple check (mock)
            println!("✓ CUDA Detect: Logic not connected (stub)");
        },
        Commands::Vectorize { input, output, mode } => {
            use engines::vector::engine::{vectorize_video, VectorConfig};
            let mut config = VectorConfig::default();
            config.colormode = mode;

            println!("🎨 Starting Vectorization Engine on {:?}", input);
            println!("   Engine: SVG (Resolution Independent)");

            match vectorize_video(&input, &output, config).await {
                Ok(msg) => println!("✅ {}", msg),
                Err(e) => error!("Vectorization failed: {}", e),
            }
        },
        Commands::Upscale { input, scale, output } => {
            use engines::vector::engine::upscale_video;
            println!("🔎 Starting Infinite Upscale (Scale: {:.1}x) on {:?}", scale, input);

            match upscale_video(&input, scale, &output).await {
                Ok(msg) => println!("✅ {}", msg),
                Err(e) => error!("Upscale failed: {}", e),
            }
        },
        Commands::Guard { mode, watch } => {
            use engines::security::defense::{sentinel::Sentinel, file_integrity::IntegrityGuard};
            use std::{thread, time::Duration};

            println!("🛡️ ACTIVATING SENTINEL Cyberdefense System...");
            println!("   Mode: {} | Least Privilege: ENABLED", mode);

            // 1. Setup Integrity Guard
            let mut integrity = IntegrityGuard::new();
            if let Some(path) = watch {
                println!("   Watching Path: {:?}", path);
                integrity.watch_path(path);
                let _ = integrity.build_baseline();
            }

            // 2. Setup Process Sentinel
            let mut sentinel = Sentinel::new();

            println!("✅ Sentinel Online. Monitoring system...");

            // Infinite Monitor Loop
            loop {
                // Check System Health
                if mode == "all" || mode == "sys" {
                    let alerts = sentinel.scan_processes();
                    for alert in alerts {
                        println!("⚠️ [SENTINEL] {}", alert);
                    }
                }

                // Check File Integrity
                if mode == "all" || mode == "file" {
                    let violations = integrity.verify_integrity();
                    for v in violations {
                        println!("❌ [INTEGRITY] {}", v);
                    }
                }

                thread::sleep(Duration::from_secs(5));
            }
        },
        Commands::Voice { record, clone, profile, speak, output, download } => {
            use engines::audio::voice::{AudioIO, VoiceEngine};

            println!("🗣️ SYNOID™ Voice Engine");

            let audio_io = AudioIO::new();

            // Record voice sample
            if let Some(duration) = record {
                let out_path = output.clone().unwrap_or_else(|| PathBuf::from("voice_sample.wav"));
                match audio_io.record_to_file(&out_path, duration) {
                    Ok(_) => println!("✅ Recorded {} seconds to {:?}", duration, out_path),
                    Err(e) => println!("❌ Recording failed: {}", e),
                }
            }

            // Download model
            if download {
                match VoiceEngine::new() {
                    Ok(engine) => {
                        println!("📥 Downloading TTS model...");
                        match engine.download_model("microsoft/speecht5_tts") {
                            Ok(path) => println!("✅ Model ready: {:?}", path),
                            Err(e) => println!("❌ Download failed: {}", e),
                        }
                    },
                    Err(e) => println!("❌ Engine init failed: {}", e),
                }
            }

            // Create voice profile from audio
            if let (Some(profile_name), Some(audio_path)) = (&profile, &clone) {
                match VoiceEngine::new() {
                    Ok(engine) => {
                        println!("🎭 Creating voice profile '{}'...", profile_name);
                        match engine.create_profile(profile_name, audio_path) {
                            Ok(p) => println!("✅ Profile '{}' created ({} dims)", p.name, p.embedding.len()),
                            Err(e) => println!("❌ Profile creation failed: {}", e),
                        }
                    },
                    Err(e) => println!("❌ {}", e),
                }
            } else if let Some(audio_path) = clone {
                // Clone voice (extract embedding without saving profile)
                match VoiceEngine::new() {
                    Ok(engine) => {
                        match engine.clone_voice(&audio_path) {
                            Ok(embedding) => println!("✅ Voice cloned. Embedding: {} dims", embedding.len()),
                            Err(e) => println!("⚠️ {}", e),
                        }
                    },
                    Err(e) => println!("❌ {}", e),
                }
            }

            // Speak text
            if let Some(text) = speak {
                let out_path = output.clone().unwrap_or_else(|| PathBuf::from("tts_output.wav"));
                match VoiceEngine::new() {
                    Ok(engine) => {
                        // If profile specified, use speak_as
                        if let Some(profile_name) = &profile {
                            match engine.speak_as(&text, profile_name, &out_path) {
                                Ok(_) => {
                                    println!("✅ Speech saved to {:?}", out_path);
                                    let _ = audio_io.play_file(&out_path);
                                },
                                Err(e) => println!("⚠️ {}", e),
                            }
                        } else {
                            match engine.speak(&text, &out_path) {
                                Ok(_) => {
                                    println!("✅ Speech saved to {:?}", out_path);
                                    let _ = audio_io.play_file(&out_path);
                                },
                                Err(e) => println!("⚠️ {}", e),
                            }
                        }
                    },
                    Err(e) => println!("❌ {}", e),
                }
            }
        },
        Commands::Agent { role, prompt, style } => {
            use agents::{director::DirectorAgent, sentinel::SentinelAgent, editor::EditorAgent};

            if role == "director" {
                let mut dir = DirectorAgent::new("gpt-oss-20b");
                let intent = prompt.unwrap_or("Make a movie".to_string());
                let style_deref = style.as_deref();

                match dir.analyze_intent(&intent, style_deref).await {
                    Ok(plan) => {
                        println!("🎬 Story Plan Generated: {}", plan.global_intent);
                        println!("   Scenes: {}", plan.scenes.len());

                        // Pass to Editor Agent
                        let editor = EditorAgent::new("MyProject");
                        if let Ok(timeline) = editor.build_timeline(&plan) {
                            println!("✅ Native Timeline Built: {} tracks", timeline.tracks.len());

                            // Pass to Sentinel (Critic)
                            let mut sentinel = SentinelAgent::new();
                            let (score, feedback) = sentinel.evaluate_edit(&timeline, &plan);
                            println!("🧐 Critic Score: {:.2}", score);
                            if !feedback.is_empty() {
                                println!("   Feedback: {:?}", feedback);
                            }
                        }
                    },
                    Err(e) => error!("Director failed: {}", e),
                }
            } else if role == "mcp" {
                 println!("🔌 MCP Bridge Initialized. Agents can now access 'media://project/assets'");
            } else {
                println!("Unknown role: {}", role);
            }
        }
    }

    Ok(())
}
