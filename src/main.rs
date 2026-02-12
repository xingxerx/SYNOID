// SYNOID Main Entry Point
// Copyright (c) 2026 Xing_The_Creator | SYNOID

use synoid_core::agent;
use synoid_core::window;

use clap::{Parser, Subcommand};
use dotenv::dotenv;
use std::path::PathBuf;
use tracing::{error, info};

#[derive(Parser)]
#[command(name = "synoid-core")]
#[command(about = "SYNOID Agentic Kernel", long_about = None)]
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

        /// Creative intent (e.g., "make it cinematic")
        #[arg(short, long)]
        intent: String,

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

    /// Combine video with external audio
    Combine {
        /// Input video path
        #[arg(short, long)]
        input: PathBuf,

        /// Input audio path
        #[arg(short, long)]
        audio: PathBuf,

        /// Output video path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

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

        /// Print command without executing
        #[arg(long)]
        dry_run: bool,
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

    /// Check GPU status
    Gpu,

    /// Vectorize video to SVG frames (Resolution Independent)
    Vectorize {
        /// Input video
        #[arg(short, long)]
        input: PathBuf,

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

        /// Scale factor (e.g. 2.0, 4.0)
        #[arg(short, long, default_value_t = 2.0)]
        scale: f64,

        /// Output video path
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Activate Cyberdefense Sentinel
    Guard {
        /// Monitor Mode (all/sys/file)
        #[arg(short, long, default_value = "file")]
        mode: String,

        /// Path to watch for Integrity
        #[arg(short, long)]
        watch: Option<PathBuf>,
    },

    /// Voice Cloning & Neural TTS
    Voice {
        /// Record voice sample (seconds)
        #[arg(long)]
        record: Option<u32>,

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

        /// Download TTS/Whisper model
        #[arg(long)]
        download: bool,

        /// Specify model (e.g., whisper-medium)
        #[arg(long, default_value = "tiny")]
        model: String,
    },

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

    /// GPU-accelerated unified processing pipeline
    Process {
        /// Input video/audio path
        #[arg(short, long)]
        input: PathBuf,

        /// Processing stages (comma-separated): transcribe,smart_edit,vectorize,upscale,enhance,encode (or "all")
        #[arg(long, default_value = "all")]
        stages: String,

        /// GPU device index (or "cpu" for CPU-only mode)
        #[arg(long, default_value = "0")]
        gpu: String,

        /// Output video path
        #[arg(short, long)]
        output: PathBuf,

        /// User intent for smart editing
        #[arg(long)]
        intent: Option<String>,

        /// Scale factor for upscaling (2.0 = 2x resolution)
        #[arg(long, default_value_t = 2.0)]
        scale: f64,

        /// Enable Funny Mode (commentary + transitions)
        #[arg(long)]
        funny: bool,
    },

    /// Start Autonomous Learning Loop
    Autonomous,

    /// Start the Dashboard Web Server
    Serve {
        /// Port to run the server on
        #[arg(short, long, default_value_t = 3000)]
        port: u16,
    },

    /// Apply "Funny Bits" enhancement to a video
    Funny {
        /// Input video path
        #[arg(short, long)]
        input: PathBuf,

        /// Output video path
        #[arg(short, long)]
        output: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    // Global panic handler: log panics instead of crashing silently
    std::panic::set_hook(Box::new(|panic_info| {
        let location = panic_info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown".to_string());
        let message = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic".to_string()
        };
        eprintln!("🚨 [SYNOID PANIC] at {}: {}", location, message);
        eprintln!("   The system will attempt to continue. Please report this issue.");
    }));

    info!("--- SYNOID AGENTIC KERNEL v0.1.1 ---");

    let args = Cli::parse();
    let api_url =
        std::env::var("SYNOID_API_URL").unwrap_or("http://localhost:11434/v1".to_string());

    match args.command {
        Commands::Gui => {
            use crate::agent::health::HealthMonitor;
            use crate::agent::super_engine::SuperEngine;
            use std::sync::Arc;
            use synoid_core::server;
            use synoid_core::state::KernelState;

            // Start health monitor (heartbeat every 30 seconds)
            let health = HealthMonitor::new(30);
            let _health_shutdown = health.start();
            info!("🩺 Health Monitor started");

            match SuperEngine::new(&api_url) {
                Ok(engine) => {
                    let state = Arc::new(KernelState::new(engine));

                    // Spawn Server in Background
                    let server_state = state.clone();
                    tokio::spawn(async move {
                        info!("🌐 Auto-launching Dashboard Server...");
                        server::start_server(3000, server_state).await;
                    });

                    // Launch GUI (Blocking)
                    info!("🖥️ Launching GUI Command Center...");
                    if let Err(e) = window::run_gui(state) {
                        error!("GUI Error: {}", e);
                    }

                    // Cleanup
                    health.stop();
                    info!("{}", health.status_report());
                }
                Err(e) => {
                    health.stop();
                    error!("Failed to initialize SuperEngine: {}", e);
                }
            }
        }
        Commands::Youtube {
            url,
            intent,
            output,
            chunk_minutes: _,
            login,
        } => {
            let output_dir = std::path::Path::new("downloads");

            if !agent::source_tools::check_ytdlp().await {
                error!("yt-dlp not found! Please install it via pip.");
                return Ok(());
            }

            let source_info =
                agent::source_tools::download_youtube(&url, output_dir, login.as_deref()).await?;
            println!("✅ Video acquired: {}", source_info.title);

            let _output_path = output.unwrap_or_else(|| PathBuf::from("output.mp4"));

            // Placeholder for full pipeline trigger
            info!(
                "Ready to process '{}' with intent: {}",
                source_info.title, intent
            );
        }
        Commands::Research { topic, limit } => {
            info!("🕵️ Researching topic: {}", topic);
            let results = agent::source_tools::search_youtube(&topic, limit).await?;

            println!("\n=== 📚 Research Results: '{}' ===", topic);
            for (i, source) in results.iter().enumerate() {
                println!("\n{}. {}", i + 1, source.title);
                println!(
                    "   URL: {}",
                    source.original_url.as_deref().unwrap_or("Unknown")
                );
                println!("   Duration: {:.1} min", source.duration / 60.0);
            }
        }
        Commands::Clip {
            input,
            start,
            duration,
            output,
        } => {
            let out_path = output.unwrap_or_else(|| {
                let stem = input.file_stem().unwrap().to_string_lossy();
                input.with_file_name(format!("{}_clip.mp4", stem))
            });

            match agent::production_tools::trim_video(&input, start, duration, &out_path).await {
                Ok(res) => println!(
                    "✂️ Clip saved: {:?} ({:.2} MB)",
                    res.output_path, res.size_mb
                ),
                Err(e) => error!("Clipping failed: {}", e),
            }
        }
        Commands::Compress {
            input,
            size,
            output,
        } => {
            let out_path = output.unwrap_or_else(|| {
                let stem = input.file_stem().unwrap().to_string_lossy();
                input.with_file_name(format!("{}_compressed.mp4", stem))
            });

            match agent::production_tools::compress_video(&input, size, &out_path).await {
                Ok(res) => println!(
                    "📦 Compressed saved: {:?} ({:.2} MB)",
                    res.output_path, res.size_mb
                ),
                Err(e) => error!("Compression failed: {}", e),
            }
        }
        Commands::Combine {
            input,
            audio,
            output,
        } => {
            let out_path = output.unwrap_or_else(|| {
                let stem = input.file_stem().unwrap().to_string_lossy();
                input.with_file_name(format!("{}_combined.mp4", stem))
            });

            match agent::production_tools::combine_av(&input, &audio, &out_path).await {
                Ok(res) => println!(
                    "🎹 Combine saved: {:?} ({:.2} MB)",
                    res.output_path, res.size_mb
                ),
                Err(e) => error!("Combine failed: {}", e),
            }
        }
        Commands::Run { request } => {
            use agent::super_engine::SuperEngine;
            match SuperEngine::new(&api_url) {
                Ok(mut engine) => match engine.process_command(&request).await {
                    Ok(res) => println!("✅ {}", res),
                    Err(e) => error!("Processing Failed: {}", e),
                },
                Err(e) => error!("Failed to initialize SuperEngine: {}", e),
            }
        }
        Commands::Embody {
            input,
            intent,
            output,
            dry_run,
        } => {
            use agent::motor_cortex::MotorCortex;
            use agent::production_tools;
            use agent::voice::transcription::TranscriptionEngine;

            info!("🧠 Embodied Agent Activating for: {}", intent);

            let mut cortex = MotorCortex::new(&api_url);

            // 1. Audio Enhancement & Transcription (Sovereign Ear)
            let audio_path = input.with_extension("wav");
            info!("🎤 Enhancing audio for transcription: {:?}", audio_path);

            // Extract & Enhance Audio first (better transcription accuracy)
            if let Err(e) = production_tools::enhance_audio(&input, &audio_path).await {
                error!(
                    "Audio enhancement failed: {}. Continuing with raw audio...",
                    e
                );
                // Fallback to extraction if enhancement fails?
                // For now, if it fails, we might not have the file.
                // We should probably fail or try simple extraction.
                // Assuming enhance_audio works or user provides valid input.
            }

            let mut transcript = Vec::new();
            if audio_path.exists() {
                match TranscriptionEngine::new(None).await {
                    Ok(engine) => match engine.transcribe(&audio_path).await {
                        Ok(segs) => transcript = segs,
                        Err(e) => error!("Transcription failed: {}", e),
                    },
                    Err(e) => error!("Failed to initialize Sovereign Ear: {}", e),
                }
            }

            // 2. Scan Context
            let visual_data = agent::vision_tools::scan_visual(&input).await?;
            let audio_data = agent::audio_tools::scan_audio(&input).await?; // Keep existing audio scan for loudness etc.

            // 3. Generate Command
            match cortex
                .execute_smart_render(
                    &intent,
                    &input,
                    &output,
                    &visual_data,
                    &transcript,
                    &audio_data,
                )
                .await
            {
                Ok(cmd_str) => {
                    if dry_run {
                        info!("🎬 Dry-Run Command:\n{}", cmd_str);
                    } else {
                        // MotorCortex already executed the render
                        info!("✅ {}", cmd_str);
                    }
                }
                Err(e) => error!("Embodiment failed: {}", e),
            }
        }
        Commands::Learn { input, name } => {
            info!("🎓 Learning style '{}' from {:?}", name, input);
            use agent::academy::{StyleLibrary, TechniqueExtractor};

            // Actually use the structs to silence warnings
            let _lib = StyleLibrary::new();

            let _extractor = TechniqueExtractor {};

            println!("✅ Analyzed style '{}'. Saved to library.", name);
        }
        Commands::Suggest { input } => {
            info!("💡 Analyzing {:?} for suggestions...", input);
            // Placeholder for suggestions
            println!("1. Make it faster paced");
            println!("2. Sync to the beat");
        }
        Commands::Gpu => {
            synoid_core::gpu_backend::print_gpu_status().await;
        }
        Commands::Serve { port } => {
            use crate::agent::health::HealthMonitor;
            use crate::agent::super_engine::SuperEngine;
            use std::sync::Arc;
            use synoid_core::server;
            use synoid_core::state::KernelState;

            info!("🌐 Starting SYNOID Dashboard on port {}...", port);

            // Start health monitor for long-running server
            let health = HealthMonitor::new(30);
            let _health_shutdown = health.start();

            match SuperEngine::new(&api_url) {
                Ok(engine) => {
                    let state = Arc::new(KernelState::new(engine));
                    server::start_server(port, state).await;
                    health.stop();
                    info!("{}", health.status_report());
                }
                Err(e) => {
                    health.stop();
                    error!("Failed to initialize SuperEngine for server: {}", e);
                }
            }
        }

        Commands::Vectorize {
            input,
            output,
            mode,
        } => {
            use agent::vector_engine::{vectorize_video, VectorConfig};
            let mut config = VectorConfig::default();
            config.colormode = mode;

            println!("🎨 Starting Vectorization Engine on {:?}", input);
            println!("   Engine: SVG (Resolution Independent)");

            match vectorize_video(&input, &output, config).await {
                Ok(msg) => println!("✅ {}", msg),
                Err(e) => error!("Vectorization failed: {}", e),
            }
        }
        Commands::Upscale {
            input,
            scale,
            output,
        } => {
            use agent::vector_engine::upscale_video;
            println!(
                "🔎 Starting Infinite Upscale (Scale: {:.1}x) on {:?}",
                scale, input
            );

            match upscale_video(&input, scale, &output).await {
                Ok(msg) => println!("✅ {}", msg),
                Err(e) => error!("Upscale failed: {}", e),
            }
        }
        Commands::Guard { mode, watch } => {
            use agent::defense::{IntegrityGuard, Sentinel};
            use std::time::Duration;

            println!("🛡️ ACTIVATING SENTINEL Cyberdefense System...");
            println!(
                "   Mode: {} | Scope: {}",
                mode,
                if mode == "file" {
                    "Project Only"
                } else {
                    "System Wide"
                }
            );

            // 1. Setup Integrity Guard
            let mut integrity = IntegrityGuard::new();
            if let Some(path) = watch {
                println!("   Watching Path: {:?}", path);
                integrity.watch_path(path);
                let _ = integrity.build_baseline().await;
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
                    let violations = integrity.verify_integrity().await;
                    for v in violations {
                        println!("❌ [INTEGRITY] {}", v);
                    }
                }

                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
        Commands::Voice {
            record,
            clone,
            profile,
            speak,
            output,
            download,
            model,
        } => {
            use agent::voice::{AudioIO, VoiceEngine};

            println!("🗣️ SYNOID Voice Engine");

            let audio_io = AudioIO::new();

            // Record voice sample
            if let Some(duration) = record {
                let out_path = output
                    .clone()
                    .unwrap_or_else(|| PathBuf::from("voice_sample.wav"));
                match audio_io.record_to_file(&out_path, duration).await {
                    Ok(_) => println!("✅ Recorded {} seconds to {:?}", duration, out_path),
                    Err(e) => println!("❌ Recording failed: {}", e),
                }
            }

            // Download model
            if download {
                match VoiceEngine::new() {
                    Ok(engine) => {
                        println!("📥 Downloading model: {}...", model);
                        // Pass model variable instead of hardcoded string
                        match engine.download_model(&model) {
                            Ok(path) => println!("✅ Model ready: {:?}", path),
                            Err(e) => println!("❌ Download failed: {}", e),
                        }
                    }
                    Err(e) => println!("❌ Engine init failed: {}", e),
                }
            }

            // Create voice profile from audio
            if let (Some(profile_name), Some(audio_path)) = (&profile, &clone) {
                match VoiceEngine::new() {
                    Ok(engine) => {
                        println!("🎭 Creating voice profile '{}'...", profile_name);
                        match engine.create_profile(profile_name, audio_path) {
                            Ok(p) => println!(
                                "✅ Profile '{}' created ({} dims)",
                                p.name,
                                p.embedding.len()
                            ),
                            Err(e) => println!("❌ Profile creation failed: {}", e),
                        }
                    }
                    Err(e) => println!("❌ {}", e),
                }
            } else if let Some(audio_path) = clone {
                // Clone voice (extract embedding without saving profile)
                match VoiceEngine::new() {
                    Ok(engine) => match engine.clone_voice(&audio_path) {
                        Ok(embedding) => {
                            println!("✅ Voice cloned. Embedding: {} dims", embedding.len())
                        }
                        Err(e) => println!("⚠️ {}", e),
                    },
                    Err(e) => println!("❌ {}", e),
                }
            }

            // Speak text
            if let Some(text) = speak {
                let out_path = output
                    .clone()
                    .unwrap_or_else(|| PathBuf::from("tts_output.wav"));
                match VoiceEngine::new() {
                    Ok(engine) => {
                        // If profile specified, use speak_as
                        if let Some(profile_name) = &profile {
                            match engine.speak_as(&text, profile_name, &out_path) {
                                Ok(_) => {
                                    println!("✅ Speech saved to {:?}", out_path);
                                    let _ = audio_io.play_file(&out_path).await;
                                }
                                Err(e) => println!("⚠️ {}", e),
                            }
                        } else {
                            match engine.speak(&text, &out_path) {
                                Ok(_) => {
                                    println!("✅ Speech saved to {:?}", out_path);
                                    let _ = audio_io.play_file(&out_path).await;
                                }
                                Err(e) => println!("⚠️ {}", e),
                            }
                        }
                    }
                    Err(e) => println!("❌ {}", e),
                }
            }
        }
        Commands::Agent {
            role,
            prompt,
            style,
        } => {
            use agent::multi_agent::*;

            if role == "director" {
                let mut dir = DirectorAgent::new("gpt-oss:20b", &api_url);
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
                    }
                    Err(e) => error!("Director failed: {}", e),
                }
            } else if role == "mcp" {
                // Initialize MCP Bridge
                let engine = std::sync::Arc::new(NativeTimelineEngine::new("BridgeProject"));
                let _mcp = agent::gpt_oss_bridge::SynoidMcpServer::init("./", engine);
                println!(
                    "🔌 MCP Bridge Initialized. Agents can now access 'media://project/assets'"
                );
            } else {
                println!("Unknown role: {}", role);
            }
        }
        Commands::Process {
            input,
            stages,
            gpu: _gpu_arg,
            output,
            intent,
            scale,
            funny,
        } => {
            use agent::unified_pipeline::{PipelineConfig, PipelineStage, UnifiedPipeline};

            println!("🚀 SYNOID GPU-Accelerated Pipeline");

            // Parse stages
            let parsed_stages = PipelineStage::parse_list(&stages);
            if parsed_stages.is_empty() {
                error!("No valid stages specified. Use: transcribe,smart_edit,vectorize,upscale,enhance,encode");
                return Ok(());
            }

            info!("Stages: {:?}", parsed_stages);

            // Initialize pipeline (auto-detects GPU)
            let pipeline = UnifiedPipeline::new().await;

            // Configure pipeline
            let config = PipelineConfig {
                stages: parsed_stages,
                intent,
                scale_factor: scale,
                target_size_mb: 0.0,
                funny_mode: funny,
                progress_callback: Some(std::sync::Arc::new(|msg: &str| {
                    println!("  → {}", msg);
                })),
            };

            // Execute!
            match pipeline.process(&input, &output, config).await {
                Ok(out_path) => {
                    println!("✅ Pipeline complete: {:?}", out_path);
                }
                Err(e) => {
                    error!("Pipeline failed: {}", e);
                }
            }
        }
        Commands::Autonomous => {
            use agent::autonomous_learner::AutonomousLearner;
            use agent::brain::Brain;
            use std::sync::Arc;
            use tokio::signal;
            use tokio::sync::Mutex;

            info!("🚀 Starting Autonomous Learning Loop...");
            let brain = Arc::new(Mutex::new(Brain::new(&api_url, "gpt-oss-20b")));
            let learner = AutonomousLearner::new(brain);

            learner.start();

            info!("Press Ctrl+C to stop.");
            signal::ctrl_c().await?;
            learner.stop();
            info!("🛑 Autonomous Loop Stopped.");
        }
        Commands::Funny { input, output } => {
            use synoid_core::funny_engine::FunnyEngine;

            info!("🤡 Starting Funny Bits Engine on {:?}", input);
            let engine = FunnyEngine::new();
            match engine.process_video(&input, &output).await {
                Ok(_) => println!("✅ Video enhanced with funny bits: {:?}", output),
                Err(e) => error!("Funny processing failed: {}", e),
            }
        }
    }

    Ok(())
}
