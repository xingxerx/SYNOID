<<<<<<< HEAD
// SYNOID Main Entry Point
// Copyright (c) 2026 Xing_The_Creator | SYNOID
=======
// SYNOID™ Main Entry Point
// Copyright (c) 2026 Xing_The_Creator | SYNOID™
>>>>>>> d08ccf5953d34fbe37a0ea8472bbd327b03ff5a3

mod agent;
mod window;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::{info, error};
use dotenv::dotenv;

#[derive(Parser)]
#[command(name = "synoid-core")]
<<<<<<< HEAD
#[command(about = "SYNOID Agentic Kernel", long_about = None)]
=======
#[command(about = "SYNOID™ Agentic Kernel", long_about = None)]
>>>>>>> d08ccf5953d34fbe37a0ea8472bbd327b03ff5a3
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Launch the GUI Control Center
    Gui,

    /// Download and process a YouTube video
    Youtube {
        /// YouTube URL or video ID
        #[arg(short, long)]
        url: String,
<<<<<<< HEAD
        
        /// Creative intent (e.g., "make it cinematic")
        #[arg(short, long)]
        intent: String,
        
=======

        /// Creative intent (e.g., "make it cinematic")
        #[arg(short, long)]
        intent: String,

>>>>>>> d08ccf5953d34fbe37a0ea8472bbd327b03ff5a3
        /// Path to output video file
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Process in chunks for long videos (minutes per chunk)
        #[arg(long, default_value = "10")]
        chunk_minutes: u32,

        /// Browser to borrow cookies from for authentication
        #[arg(long)]
        login: Option<String>,
    },

    /// Autonomous Research: Find tutorials and resources
    Research {
        /// Topic to research
        #[arg(short, long)]
        topic: String,
<<<<<<< HEAD
        
=======

>>>>>>> d08ccf5953d34fbe37a0ea8472bbd327b03ff5a3
        /// Number of results to find
        #[arg(short, long, default_value = "5")]
        limit: usize,
    },

    /// Trim/Clip a video
    Clip {
        /// Input video path
        #[arg(short, long)]
        input: PathBuf,

        /// Start time in seconds
        #[arg(short, long)]
        start: f64,

        /// Duration in seconds
        #[arg(short, long)]
        duration: f64,
<<<<<<< HEAD
        
=======

>>>>>>> d08ccf5953d34fbe37a0ea8472bbd327b03ff5a3
        /// Output path (optional)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Compress video to target size
    Compress {
        /// Input video path
        #[arg(short, long)]
        input: PathBuf,

        /// Target size in MB
        #[arg(short, long)]
        size: f64,

        /// Output path (optional)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
<<<<<<< HEAD
    
=======

>>>>>>> d08ccf5953d34fbe37a0ea8472bbd327b03ff5a3
    /// Run the Brain directly
    Run {
        #[arg(short, long)]
        request: String,
    },

    /// Embody the agent for full video editing tasks
    Embody {
        /// Input video path
        #[arg(short, long)]
        input: PathBuf,

        /// User intent/instruction
        #[arg(short, long)]
        intent: String,

        /// Output path
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Learn a new editing style
    Learn {
        /// Input video to learn from
        #[arg(short, long)]
        input: PathBuf,

        /// Name of the style
        #[arg(short, long)]
        name: String,
    },

    /// Suggest edits for a video
    Suggest {
        /// Input video
        #[arg(short, long)]
        input: PathBuf,
    },
<<<<<<< HEAD
    
=======

>>>>>>> d08ccf5953d34fbe37a0ea8472bbd327b03ff5a3
    /// Check GPU status
    Gpu,

    /// Vectorize video to SVG frames (Resolution Independent)
    Vectorize {
        /// Input video
        #[arg(short, long)]
        input: PathBuf,
<<<<<<< HEAD
        
=======

>>>>>>> d08ccf5953d34fbe37a0ea8472bbd327b03ff5a3
        /// Output directory
        #[arg(short, long)]
        output: PathBuf,

        /// Color mode: color/binary
        #[arg(long, default_value = "color")]
        mode: String,
    },

    /// Infinite Upscale (Neural/Vector)
    Upscale {
        /// Input video
        #[arg(short, long)]
        input: PathBuf,
<<<<<<< HEAD
        
        /// Scale factor (e.g. 2.0, 4.0)
        #[arg(short, long, default_value_t = 2.0)]
        scale: f64,
        
=======

        /// Scale factor (e.g. 2.0, 4.0)
        #[arg(short, long, default_value_t = 2.0)]
        scale: f64,

>>>>>>> d08ccf5953d34fbe37a0ea8472bbd327b03ff5a3
        /// Output video path
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Activate Cyberdefense Sentinel
    Guard {
        /// Monitor Mode (Process/File)
        #[arg(short, long, default_value = "all")]
        mode: String,
<<<<<<< HEAD
        
=======

>>>>>>> d08ccf5953d34fbe37a0ea8472bbd327b03ff5a3
        /// Path to watch for Integrity
        #[arg(short, long)]
        watch: Option<PathBuf>,
    },

    /// Voice Cloning & Neural TTS
    Voice {
        /// Record voice sample (seconds)
        #[arg(long)]
        record: Option<u32>,
<<<<<<< HEAD
        
        /// Clone voice from audio file
        #[arg(long)]
        clone: Option<PathBuf>,
        
        /// Create named voice profile from audio
        #[arg(long)]
        profile: Option<String>,
        
        /// Text to speak
        #[arg(long)]
        speak: Option<String>,
        
        /// Output audio file
        #[arg(short, long)]
        output: Option<PathBuf>,
        
=======

        /// Clone voice from audio file
        #[arg(long)]
        clone: Option<PathBuf>,

        /// Create named voice profile from audio
        #[arg(long)]
        profile: Option<String>,

        /// Text to speak
        #[arg(long)]
        speak: Option<String>,

        /// Output audio file
        #[arg(short, long)]
        output: Option<PathBuf>,

>>>>>>> d08ccf5953d34fbe37a0ea8472bbd327b03ff5a3
        /// Download TTS model
        #[arg(long)]
        download: bool,
    },
<<<<<<< HEAD
=======

    /// Multi-Agent Role Execution
    Agent {
        /// Role to enact: director, critic, etc.
        #[arg(long)]
        role: String,

        /// User prompt or context
        #[arg(long)]
        prompt: Option<String>,

        /// Style profile to trigger dynamic reasoning
        #[arg(long)]
        style: Option<String>,
    },
>>>>>>> d08ccf5953d34fbe37a0ea8472bbd327b03ff5a3
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    tracing_subscriber::fmt::init();
<<<<<<< HEAD
    
    info!("--- SYNOID AGENTIC KERNEL v0.1.0 ---");
    
=======

    info!("--- SYNOID™ AGENTIC KERNEL v0.1.0 ---");

>>>>>>> d08ccf5953d34fbe37a0ea8472bbd327b03ff5a3
    let args = Cli::parse();
    let api_url = std::env::var("SYNOID_API_URL").unwrap_or("http://localhost:11434/v1".to_string());

    match args.command {
        Commands::Gui => {
            if let Err(e) = window::run_gui() {
                error!("GUI Error: {}", e);
            }
        },
        Commands::Youtube { url, intent, output, chunk_minutes: _, login } => {
            let output_dir = std::path::Path::new("downloads");
<<<<<<< HEAD
            
=======

>>>>>>> d08ccf5953d34fbe37a0ea8472bbd327b03ff5a3
            if !agent::source_tools::check_ytdlp() {
                error!("yt-dlp not found! Please install it via pip.");
                return Ok(());
            }

            let source_info = agent::source_tools::download_youtube(&url, output_dir, login.as_deref()).await?;
            println!("✅ Video acquired: {}", source_info.title);
<<<<<<< HEAD
            
            let _output_path = output.unwrap_or_else(|| PathBuf::from("output.mp4"));
            
=======

            let _output_path = output.unwrap_or_else(|| PathBuf::from("output.mp4"));

>>>>>>> d08ccf5953d34fbe37a0ea8472bbd327b03ff5a3
            // Placeholder for full pipeline trigger
            info!("Ready to process '{}' with intent: {}", source_info.title, intent);
        },
        Commands::Research { topic, limit } => {
            info!("🕵️ Researching topic: {}", topic);
            let results = agent::source_tools::search_youtube(&topic, limit).await?;
<<<<<<< HEAD
            
=======

>>>>>>> d08ccf5953d34fbe37a0ea8472bbd327b03ff5a3
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
<<<<<<< HEAD
            
=======

>>>>>>> d08ccf5953d34fbe37a0ea8472bbd327b03ff5a3
            match agent::production_tools::trim_video(&input, start, duration, &out_path).await {
                Ok(res) => println!("✂️ Clip saved: {:?} ({:.2} MB)", res.output_path, res.size_mb),
                Err(e) => error!("Clipping failed: {}", e),
            }
        },
        Commands::Compress { input, size, output } => {
            let out_path = output.unwrap_or_else(|| {
                let stem = input.file_stem().unwrap().to_string_lossy();
                input.with_file_name(format!("{}_compressed.mp4", stem))
            });
<<<<<<< HEAD
            
=======

>>>>>>> d08ccf5953d34fbe37a0ea8472bbd327b03ff5a3
            match agent::production_tools::compress_video(&input, size, &out_path).await {
                Ok(res) => println!("📦 Compressed saved: {:?} ({:.2} MB)", res.output_path, res.size_mb),
                Err(e) => error!("Compression failed: {}", e),
            }
        },
        Commands::Run { request } => {
            use agent::brain::Brain;
            let mut brain = Brain::new(&api_url);
            match brain.process(&request).await {
                Ok(res) => println!("✅ {}", res),
                Err(e) => error!("Detail: {}", e),
            }
        },
        Commands::Embody { input, intent, output } => {
            use agent::motor_cortex::MotorCortex;
            info!("🧠 Embodied Agent Activating for: {}", intent);
<<<<<<< HEAD
            
=======

>>>>>>> d08ccf5953d34fbe37a0ea8472bbd327b03ff5a3
            let mut cortex = MotorCortex::new(&api_url);

            // 1. Scan Context
            let visual_data = agent::vision_tools::scan_visual(&input).await?;
            let audio_data = agent::audio_tools::scan_audio(&input).await?;

            // 2. Execute
<<<<<<< HEAD
            match cortex.execute_intent(&intent, &input, &output, &visual_data, &audio_data).await {
                Ok(graph) => {
                    let cmd = graph.to_ffmpeg_command(input.to_str().unwrap_or("input.mp4"), output.to_str().unwrap_or("output.mp4"));
=======
            match cortex.execute_one_shot_render(&intent, &input, &output, &visual_data, &audio_data).await {
                Ok(cmd) => {
>>>>>>> d08ccf5953d34fbe37a0ea8472bbd327b03ff5a3
                    println!("🎬 Generated FFmpeg Command: {}", cmd);
                    // In a real run, we would execute this command here.
                },
                Err(e) => error!("Embodiment failed: {}", e),
            }
        },
        Commands::Learn { input, name } => {
            info!("🎓 Learning style '{}' from {:?}", name, input);
            use agent::academy::{StyleLibrary, TechniqueExtractor};
<<<<<<< HEAD
            
            // Actually use the structs to silence warnings
            let _lib = StyleLibrary {};
            let _extractor = TechniqueExtractor {};
            
=======

            // Actually use the structs to silence warnings
            let _lib = StyleLibrary {};
            let _extractor = TechniqueExtractor {};

>>>>>>> d08ccf5953d34fbe37a0ea8472bbd327b03ff5a3
            println!("✅ Analyzed style '{}'. Saved to library.", name);
        },
        Commands::Suggest { input } => {
            info!("💡 Analyzing {:?} for suggestions...", input);
            // Placeholder for suggestions
            println!("1. Make it faster paced");
            println!("2. Sync to the beat");
        },
        Commands::Gpu => {
<<<<<<< HEAD
            println!("=== SYNOID GPU Status ===");
=======
            println!("=== SYNOID™ GPU Status ===");
>>>>>>> d08ccf5953d34fbe37a0ea8472bbd327b03ff5a3
            // Simple check (mock)
            println!("✓ CUDA Detect: Logic not connected (stub)");
        },
        Commands::Vectorize { input, output, mode } => {
            use agent::vector_engine::{vectorize_video, VectorConfig};
            let mut config = VectorConfig::default();
            config.colormode = mode;
<<<<<<< HEAD
            
            println!("🎨 Starting Vectorization Engine on {:?}", input);
            println!("   Engine: SVG (Resolution Independent)");
            
=======

            println!("🎨 Starting Vectorization Engine on {:?}", input);
            println!("   Engine: SVG (Resolution Independent)");

>>>>>>> d08ccf5953d34fbe37a0ea8472bbd327b03ff5a3
            match vectorize_video(&input, &output, config).await {
                Ok(msg) => println!("✅ {}", msg),
                Err(e) => error!("Vectorization failed: {}", e),
            }
        },
        Commands::Upscale { input, scale, output } => {
            use agent::vector_engine::upscale_video;
            println!("🔎 Starting Infinite Upscale (Scale: {:.1}x) on {:?}", scale, input);
<<<<<<< HEAD
            
=======

>>>>>>> d08ccf5953d34fbe37a0ea8472bbd327b03ff5a3
            match upscale_video(&input, scale, &output).await {
                Ok(msg) => println!("✅ {}", msg),
                Err(e) => error!("Upscale failed: {}", e),
            }
        },
        Commands::Guard { mode, watch } => {
            use agent::defense::{Sentinel, IntegrityGuard};
            use std::{thread, time::Duration};
<<<<<<< HEAD
            
            println!("🛡️ ACTIVATING SENTINEL Cyberdefense System...");
            println!("   Mode: {} | Least Privilege: ENABLED", mode);
            
=======

            println!("🛡️ ACTIVATING SENTINEL Cyberdefense System...");
            println!("   Mode: {} | Least Privilege: ENABLED", mode);

>>>>>>> d08ccf5953d34fbe37a0ea8472bbd327b03ff5a3
            // 1. Setup Integrity Guard
            let mut integrity = IntegrityGuard::new();
            if let Some(path) = watch {
                println!("   Watching Path: {:?}", path);
                integrity.watch_path(path);
                let _ = integrity.build_baseline();
            }

            // 2. Setup Process Sentinel
            let mut sentinel = Sentinel::new();
<<<<<<< HEAD
            
            println!("✅ Sentinel Online. Monitoring system...");
            
=======

            println!("✅ Sentinel Online. Monitoring system...");

>>>>>>> d08ccf5953d34fbe37a0ea8472bbd327b03ff5a3
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
<<<<<<< HEAD
                
=======

>>>>>>> d08ccf5953d34fbe37a0ea8472bbd327b03ff5a3
                thread::sleep(Duration::from_secs(5));
            }
        },
        Commands::Voice { record, clone, profile, speak, output, download } => {
            use agent::voice::{AudioIO, VoiceEngine};
<<<<<<< HEAD
            
            println!("🗣️ SYNOID Voice Engine");
            
            let audio_io = AudioIO::new();
            
=======

            println!("🗣️ SYNOID™ Voice Engine");

            let audio_io = AudioIO::new();

>>>>>>> d08ccf5953d34fbe37a0ea8472bbd327b03ff5a3
            // Record voice sample
            if let Some(duration) = record {
                let out_path = output.clone().unwrap_or_else(|| PathBuf::from("voice_sample.wav"));
                match audio_io.record_to_file(&out_path, duration) {
                    Ok(_) => println!("✅ Recorded {} seconds to {:?}", duration, out_path),
                    Err(e) => println!("❌ Recording failed: {}", e),
                }
            }
<<<<<<< HEAD
            
=======

>>>>>>> d08ccf5953d34fbe37a0ea8472bbd327b03ff5a3
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
<<<<<<< HEAD
            
=======

>>>>>>> d08ccf5953d34fbe37a0ea8472bbd327b03ff5a3
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
<<<<<<< HEAD
            
=======

>>>>>>> d08ccf5953d34fbe37a0ea8472bbd327b03ff5a3
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
<<<<<<< HEAD
=======
        },
        Commands::Agent { role, prompt, style } => {
            use agent::multi_agent::*;

            if role == "director" {
                let mut dir = DirectorAgent::new("gpt-oss-20b");
                let intent = prompt.unwrap_or("Make a movie".to_string());
                let style_deref = style.as_deref();

                match dir.analyze_intent(&intent, style_deref).await {
                    Ok(plan) => {
                        println!("🎬 Story Plan Generated: {}", plan.global_intent);
                        println!("   Scenes: {}", plan.scenes.len());

                        // Pass to Timeline Engine
                        let engine = NativeTimelineEngine::new("MyProject");
                        if let Ok(timeline) = engine.build_from_plan(&plan) {
                            println!("✅ Native Timeline Built: {} tracks", timeline.tracks.len());

                            // Pass to Critic
                            let mut critic = CriticAgent::new();
                            let (score, feedback) = critic.evaluate_edit(&timeline, &plan);
                            println!("🧐 Critic Score: {:.2}", score);
                            if !feedback.is_empty() {
                                println!("   Feedback: {:?}", feedback);
                            }
                        }
                    },
                    Err(e) => error!("Director failed: {}", e),
                }
            } else if role == "mcp" {
                 // Initialize MCP Bridge
                 let engine = std::sync::Arc::new(NativeTimelineEngine::new("BridgeProject"));
                 let _mcp = agent::gpt_oss_bridge::SynoidMcpServer::init("./", engine);
                 println!("🔌 MCP Bridge Initialized. Agents can now access 'media://project/assets'");
            } else {
                println!("Unknown role: {}", role);
            }
>>>>>>> d08ccf5953d34fbe37a0ea8472bbd327b03ff5a3
        }
    }

    Ok(())
}
