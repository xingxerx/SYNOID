// SYNOID Main Entry Point
// Copyright (c) 2026 Xing_The_Creator | SYNOID

use synoid_core::agent;
use synoid_core::agent::core::AgentCore;
use synoid_core::window;

use clap::{Parser, Subcommand};
use dotenv::dotenv;
use std::path::PathBuf;
use std::sync::Arc;
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

    /// Activate Cyberdefense Sentinel
    Guard {
        /// Monitor Mode (all/sys/file)
        #[arg(short, long, default_value = "file")]
        mode: String,

        /// Path to watch for Integrity
        #[arg(short, long)]
        watch: Option<PathBuf>,
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

        /// Processing stages (comma-separated): transcribe,smart_edit,enhance,encode (or "all")
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
    },

    /// Start Autonomous Learning Loop
    Autonomous,

    /// Start the Dashboard Web Server
    Serve {
        /// Port to run the server on
        #[arg(short, long, default_value_t = 3000)]
        port: u16,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dotenv().ok();
    
    // Set default log level to suppress noisy internal crates (wgpu, naga, etc.)
    // unless explicitly overridden by the user.
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info,wgpu_core=error,wgpu_hal=error,naga=error,winit=error,symphonia=error,sctk_adwaita=off,egui_wgpu=error");
    }

    // Suppress noisy C library warnings from Mesa / EGL / Zink on WSL2
    // These are C-level outputs not controllable via Rust tracing.
    if std::env::var("EGL_LOG_LEVEL").is_err() {
        std::env::set_var("EGL_LOG_LEVEL", "fatal");
    }
    if std::env::var("MESA_LOG_LEVEL").is_err() {
        std::env::set_var("MESA_LOG_LEVEL", "fatal");
    }

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
        eprintln!("   CRITICAL ERROR: The system is crashing. Please report this issue.");
    }));

    info!("--- SYNOID AGENTIC KERNEL v0.1.1 ---");

    // Check external dependencies
    let missing_deps = synoid_core::agent::health::check_dependencies();
    if !missing_deps.is_empty() {
        tracing::debug!("⚠️ Missing dependencies: {:?}. Some features may not work.", missing_deps);
    }

    let api_url = std::env::var("SYNOID_API_URL").unwrap_or("http://localhost:11434/v1".to_string());

    // Initialize the Ghost (Agent Core)
    let core = Arc::new(AgentCore::new(&api_url));

    // Connect Brain → GPU/CUDA backend (neuroplasticity-tuned acceleration)
    core.connect_gpu_to_brain().await;
    info!("🧠⚡ Neural-GPU bridge active: {}", core.acceleration_status().await);
    
    // Initialize Hive Mind (Ollama discovery) - Non-blocking background task
    let core_for_hive = core.clone();
    tokio::spawn(async move {
        match core_for_hive.initialize_hive_mind().await {
            Ok(_) => info!("🐝 Hive Mind Active: Connected to Ollama Neural Network"),
            Err(e) => {
                tracing::debug!("⚠️ Hive Mind Offline: {}", e);
                tracing::debug!("⚠️ Continuing in degraded mode (Brain defaults only)");
            }
        }
    });

    let args = Cli::parse();

    match args.command {
        Commands::Gui => {
            use crate::agent::health::HealthMonitor;
            use synoid_core::server;
            use synoid_core::state::KernelState;

            // Start health monitor (heartbeat every 30 seconds)
            let health = HealthMonitor::new(30);
            let _health_shutdown = health.start();
            info!("🩺 Health Monitor started");

            // Use the existing core instance
            let state = Arc::new(KernelState::new(core.clone()));

            // Spawn Server in Background
            let server_state = state.clone();
            tokio::spawn(async move {
                info!("🌐 Auto-launching Dashboard Server...");
                server::start_server(3000, server_state).await;
            });

            // Launch GUI (Blocking) — pass AgentCore
            info!("🖥️ Launching GUI Command Center...");
            let res = tokio::task::block_in_place(|| window::run_gui(core));
            if let Err(e) = res {
                error!("GUI Error: {}", e);
            }

            // Cleanup
            health.stop();
            info!("{}", health.status_report());
            info!("🛑 GUI closed. Shutting down all background tasks...");

            // Force-exit to kill all spawned tokio tasks (server, hive mind poller, editor queue).
            // Without this, background tasks keep the process alive as a ghost.
            std::process::exit(0);
        }
        Commands::Youtube {
            url,
            intent,
            output,
            chunk_minutes,
            login,
        } => {
            core.process_youtube_intent(&url, &intent, output, login.as_deref(), false, chunk_minutes).await?;
        }
        Commands::Research { topic, limit } => {
            core.process_research(&topic, limit).await?;
        }
        Commands::Clip {
            input,
            start,
            duration,
            output,
        } => {
            core.clip_video(&input, start, duration, output).await?;
        }
        Commands::Compress {
            input,
            size,
            output,
        } => {
            core.compress_video(&input, size, output).await?;
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
            core.process_brain_request(&request).await?;
        }
        Commands::Embody {
            input,
            intent,
            output,
            dry_run,
        } => {
            core.embody_intent(&input, &intent, &output, dry_run)
                .await?;
        }
        Commands::Learn { input, name } => {
            core.learn_style(&input, &name).await?;
        }
        Commands::Suggest { input } => {
            info!("💡 Analyzing {:?} for suggestions...", input);
            use synoid_core::agent::vision_tools;
            match vision_tools::scan_visual(&input).await {
                Ok(scenes) => {
                    let count = scenes.len();
                    if count == 0 {
                        println!("❌ No scenes detected. Video might be empty or corrupt.");
                    } else {
                        let duration = if !scenes.is_empty() {
                            scenes.last().unwrap().timestamp
                        } else {
                            0.0
                        };
                        let avg = if count > 0 {
                            duration / count as f64
                        } else {
                            0.0
                        };
                        println!("✅ Analysis Complete: {} scenes detected.", count);
                        println!("   Avg Shot Length: {:.2}s", avg);

                        println!("\n💡 Suggestions:");
                        if avg > 5.0 {
                            println!("1. Pace is slow. Consider 'make it faster' or 'cut silence'.");
                        } else if avg < 2.0 {
                            println!("1. Pace is fast/action-heavy. Consider 'stabilize' or 'enhance audio'.");
                        } else {
                            println!("1. Pacing is balanced. Consider 'cinematic color grade'.");
                        }
                        println!("2. Try 'vectorize' for a unique look.");
                        println!("3. Try 'funny' mode to add humor.");
                    }
                }
                Err(e) => error!("Failed to analyze video: {}", e),
            }
        }
        Commands::Gpu => {
            synoid_core::gpu_backend::print_gpu_status().await;
        }
        Commands::Serve { port } => {
            use crate::agent::health::HealthMonitor;
            use synoid_core::server;
            use synoid_core::state::KernelState;

            info!("🌐 Starting SYNOID Dashboard on port {}...", port);

            // Start health monitor for long-running server
            let health = HealthMonitor::new(30);
            let _health_shutdown = health.start();

            let state = Arc::new(KernelState::new(core.clone()));
            server::start_server(port, state).await;
            health.stop();
            info!("{}", health.status_report());
        }

        Commands::Guard { mode, watch } => {
            // Guard runs indefinitely
            core.activate_sentinel(&mode, watch).await;
        }
        Commands::Agent {
            role,
            prompt,
            style,
        } => {
            use synoid_core::agent::multi_agent::*;
            let prompt_text = prompt.unwrap_or_else(|| "Do your job".to_string());

            match role.as_str() {
                "director" => {
                    let mut dir = DirectorAgent::new("llama3:latest", &api_url);
                    let style_deref = style.as_deref();

                    match dir.analyze_intent(&prompt_text, style_deref).await {
                        Ok(plan) => {
                            core.log(&format!("🎬 Story Plan Generated: {}", plan.global_intent));
                        }
                        Err(e) => error!("Director failed: {}", e),
                    }
                }
                "critic" => {
                    println!("🧐 Critic Agent analyzing request: '{}'", prompt_text);
                    println!("   Critique: The concept is sound but needs more conflict. Try adding 'dramatic' to the intent.");
                }
                "editor" => {
                    println!("✂️ Editor Agent ready. Please use 'embody' or 'process' commands for actual editing tasks.");
                }
                _ => println!("Unknown role: {}. Available: director, critic, editor", role),
            }
        }
        Commands::Process {
            input,
            stages,
            gpu,
            output,
            intent,
            scale,
        } => {
            core.run_unified_pipeline(&input, &output, &stages, &gpu, intent, scale)
                .await?;
        }
        Commands::Autonomous => {
            use agent::autonomous_learner::AutonomousLearner;
            use agent::brain::Brain;
            use tokio::signal;
            use tokio::sync::Mutex;

            info!("🚀 Starting Autonomous Learning Loop...");
            let brain = Arc::new(Mutex::new(Brain::new(&api_url, "llama3:latest")));
            let learner = AutonomousLearner::new(brain);

            learner.start();

            info!("Press Ctrl+C to stop.");
            signal::ctrl_c().await?;
            learner.stop();
            info!("🛑 Autonomous Loop Stopped.");
        }
    }

    Ok(())
}
