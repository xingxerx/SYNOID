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

    /// GPU-accelerated unified processing pipeline
    Process {
        /// Input video/audio path
        #[arg(short, long)]
        input: PathBuf,

        /// Processing stages (comma-separated): smart_edit,enhance,encode (or "all")
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
    },

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
    tracing_subscriber::fmt::init();

    // Global panic handler
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

    let missing_deps = synoid_core::agent::health::check_dependencies();
    if !missing_deps.is_empty() {
        tracing::warn!("⚠️ Missing dependencies: {:?}. Some features may not work.", missing_deps);
    }

    let api_url = std::env::var("SYNOID_API_URL").unwrap_or("http://localhost:11434/v1".to_string());

    let core = Arc::new(AgentCore::new(&api_url));

    core.connect_gpu_to_brain().await;
    info!(
        "🧠⚡ Neural-GPU bridge active: {}",
        core.acceleration_status().await
    );

    let args = Cli::parse();

    match args.command {
        Commands::Gui => {
            use crate::agent::health::HealthMonitor;
            use synoid_core::server;
            use synoid_core::state::KernelState;

            let health = HealthMonitor::new(30);
            let _health_shutdown = health.start();
            info!("🩺 Health Monitor started");

            let state = Arc::new(KernelState::new(core.clone()));

            let server_state = state.clone();
            tokio::spawn(async move {
                info!("🌐 Auto-launching Dashboard Server...");
                server::start_server(3000, server_state).await;
            });

            info!("🖥️ Launching GUI Command Center...");
            if let Err(e) = window::run_gui(core) {
                error!("GUI Error: {}", e);
            }

            health.stop();
            info!("{}", health.status_report());
        }
        Commands::Youtube {
            url,
            intent,
            output,
            chunk_minutes,
            login,
        } => {
            core.process_youtube_intent(&url, &intent, output, login.as_deref(), chunk_minutes)
                .await?;
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
                        let duration = scenes.last().unwrap().timestamp;
                        let avg = duration / count as f64;
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
                        println!("2. Try the Embody command with a creative intent.");
                    }
                }
                Err(e) => error!("Failed to analyze video: {}", e),
            }
        }
        Commands::Gpu => {
            synoid_core::gpu_backend::print_gpu_status().await;
        }
        Commands::Process {
            input,
            stages,
            gpu,
            output,
            intent,
        } => {
            core.run_unified_pipeline(&input, &output, &stages, &gpu, intent)
                .await?;
        }
        Commands::Serve { port } => {
            use crate::agent::health::HealthMonitor;
            use synoid_core::server;
            use synoid_core::state::KernelState;

            info!("🌐 Starting SYNOID Dashboard on port {}...", port);

            let health = HealthMonitor::new(30);
            let _health_shutdown = health.start();

            let state = Arc::new(KernelState::new(core.clone()));
            server::start_server(port, state).await;
            health.stop();
            info!("{}", health.status_report());
        }
    }

    Ok(())
}
