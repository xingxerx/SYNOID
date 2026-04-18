// SYNOID Main Entry Point
// Copyright (c) 2026 xingxerx_The_Creator | SYNOID

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
    Gui {
        /// Dashboard server port (change to run multiple instances)
        #[arg(short, long, default_value_t = 3000)]
        port: u16,
    },

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
        /// Role to enact: director
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

    /// Learn editing style from videos already in D:\SYNOID\Download (up to 10)
    LearnDownloads,

    /// Autonomous academic research pipeline (AutoResearchClaw-powered)
    AutoResearch {
        /// Research topic or question
        #[arg(short, long)]
        topic: String,

        /// Maximum number of papers to retrieve
        #[arg(short, long, default_value = "15")]
        limit: usize,

        /// Save full results to JSON file
        #[arg(long)]
        save: bool,
    },

    /// Start Autonomous Learning Loop
    Autonomous {
        /// Optional port for instance isolation (e.g., 3001)
        #[arg(short, long)]
        port: Option<u16>,
    },

    /// Start the Dashboard Web Server
    Serve {
        /// Port to run the server on
        #[arg(short, long, default_value_t = 3000)]
        port: u16,
    },

    /// Self-recursing editing strategy optimizer (karpathy/autoresearch-style)
    AutoImprove {
        /// Number of strategy mutations to evaluate per iteration (default: 4)
        #[arg(short, long, default_value_t = 4)]
        candidates: usize,

        /// Stop after N iterations (omit for infinite loop)
        #[arg(short, long)]
        iterations: Option<u64>,

        /// Print status of past runs and exit
        #[arg(long)]
        status: bool,
    },

    /// Run the ReAct (Reason+Act) agentic loop for a multi-step goal.
    ///
    /// The agent iterates Thought → Action → Observation until it reaches an
    /// answer or hits max_iterations. Tools: analyze_video, search_youtube,
    /// learn_style, query_brain, edit_video, run_command (whitelisted).
    React {
        /// Multi-step goal for the agent (e.g. "analyze my latest video and suggest a grade")
        #[arg(short, long)]
        goal: String,

        /// Maximum reasoning iterations (default: 8, scales with neuroplasticity)
        #[arg(long, default_value_t = 8)]
        max_iterations: usize,
    },

    /// Gemma 4 self-improvement harness — give Gemma 4 tools to build and improve SYNOID.
    ///
    /// Gemma 4 iterates Thought → Action → Observation using tools:
    /// read_file, list_files, write_file, search_code, cargo_check, cargo_test, finish.
    Gemma4 {
        /// Task for Gemma 4 (e.g. "improve smart_editor scene detection accuracy")
        #[arg(short, long)]
        task: String,

        /// Maximum tool-use steps before giving up (default: 16)
        #[arg(long, default_value_t = 16)]
        max_steps: usize,

        /// Plan only — print intended actions without writing files or running cargo
        #[arg(long)]
        dry_run: bool,
    },

    /// GEPA self-improvement loop (Goal-Experience-Policy-Agent)
    ///
    /// Runs the background GEPA cycle that replays trajectory episodes,
    /// distils the best editing patterns, and writes improved policies to
    /// the LearningKernel — making every subsequent intelligent_edit better.
    Gepa {
        /// Print trajectory insights report and exit (no loop)
        #[arg(long)]
        insights: bool,

        /// Run a one-shot policy update from stored trajectories and exit
        #[arg(long)]
        update: bool,

        /// Optional port for instance isolation (e.g. 3001)
        #[arg(short, long)]
        port: Option<u16>,

        /// Interval in seconds between background policy update cycles (default: 120)
        #[arg(long, default_value_t = 120)]
        interval: u64,
    },

    /// Transcribe a video and save a clean SRT subtitle file next to it
    Transcribe {
        /// Input video path
        #[arg(short, long)]
        input: PathBuf,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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

    async_main()
}

#[tokio::main]
async fn async_main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
        tracing::debug!(
            "⚠️ Missing dependencies: {:?}. Some features may not work.",
            missing_deps
        );
    }

    let args = Cli::parse();

    // Auto-set Instance ID based on port if in GUI mode and not already set
    if let Commands::Gui { port } = args.command {
        if port != 3000 && std::env::var("SYNOID_INSTANCE_ID").is_err() {
            let instance_id = format!("_{}", port);
            std::env::set_var("SYNOID_INSTANCE_ID", &instance_id);
            info!("🔷 Auto-Isolated Instance: '{}' (port {})", instance_id, port);
        }
    }

    let api_url =
        std::env::var("SYNOID_API_URL").unwrap_or("http://localhost:11434/v1".to_string());
    let instance_id = std::env::var("SYNOID_INSTANCE_ID").unwrap_or_else(|_| "default".to_string());

    // Initialize the Ghost (Agent Core)
    let core = Arc::new(AgentCore::new(&api_url, &instance_id));

    // Connect Brain → GPU/CUDA backend (neuroplasticity-tuned acceleration)
    core.connect_gpu_to_brain().await;
    info!(
        "🧠⚡ Neural-GPU bridge active: {}",
        core.acceleration_status().await
    );

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
    match args.command {
        Commands::Gui { port } => {
            use crate::agent::core_systems::health::HealthMonitor;
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
                server::start_server(port, server_state).await;
            });

            // Auto-learn from downloaded reference videos in the background
            let core_for_learning = core.clone();
            tokio::spawn(async move {
                info!("🎓 Auto-learning from reference videos in Download folder...");
                core_for_learning.learn_from_downloads().await;
            });

            // Launch GUI (Blocking) — pass AgentCore
            info!("🖥️ Launching GUI Command Center...");
            let core_in_gui = core.clone();
            let res = tokio::task::block_in_place(|| window::run_gui(core_in_gui));
            if let Err(e) = res {
                error!("GUI Error: {}", e);
            }

            // Cleanup
            health.stop();
            info!("{}", health.status_report());
            info!("🛑 GUI closed. Shutting down all background tasks...");
            // Graceful shutdown: ensure background video rendering completes before exiting
            info!("⏳ Waiting for active video editing jobs to finish...");
            core.editor_queue.wait_for_completion().await;

            // Force-exit to kill all spawned tokio tasks (server, hive mind poller, health monitor).
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
            core.process_youtube_intent(
                &url,
                &intent,
                output,
                login.as_deref(),
                false,
                chunk_minutes,
                true,
                true,
            )
            .await?;
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
            core.embody_intent(&input, &intent, &output, dry_run, true, true)
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
                            println!(
                                "1. Pace is slow. Consider 'make it faster' or 'cut silence'."
                            );
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
            use crate::agent::core_systems::health::HealthMonitor;
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
                _ => println!("Unknown role: {}. Available: director", role),
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
        Commands::LearnDownloads => {
            info!("🎓 Learning editing style from downloaded reference videos...");
            core.learn_from_downloads().await;
            info!("✅ Style learning complete.");
        }

        Commands::AutoResearch { topic, limit, save } => {
            info!("🔬 Launching AutoResearch pipeline for: {}", topic);
            core.process_auto_research(&topic, limit, save).await?;
        }

        Commands::AutoImprove {
            candidates,
            iterations,
            status,
        } => {
            use synoid_core::agent::auto_improve::AutoImprove;

            if status {
                AutoImprove::print_status();
            } else {
                info!("🧬 AutoImprove: self-recursing strategy optimizer starting...");
                let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

                // Ctrl-C handler
                tokio::spawn(async move {
                    if tokio::signal::ctrl_c().await.is_ok() {
                        info!("[IMPROVE] Ctrl-C received — signalling shutdown");
                        let _ = shutdown_tx.send(true);
                    }
                });

                let mut improver = AutoImprove::new();
                improver.candidates_per_iter = candidates;
                improver.max_iterations = iterations;

                match improver.run(shutdown_rx).await {
                    Ok(()) => info!("✅ AutoImprove finished."),
                    Err(e) => error!("AutoImprove error: {}", e),
                }
            }
        }

        Commands::Autonomous { port } => {
            use agent::autonomous_learner::AutonomousLearner;
            use agent::brain::Brain;
            use tokio::signal;
            use tokio::sync::Mutex;

            if let Some(p) = port {
                if p != 3000 {
                    let instance_id = format!("_{}", p);
                    std::env::set_var("SYNOID_INSTANCE_ID", &instance_id);
                    info!("🔷 Auto-Isolated Learning Instance: '{}' (port {})", instance_id, p);
                }
            }

            info!("🚀 Starting Autonomous Learning Loop...");
            let brain = Arc::new(Mutex::new(Brain::new(&api_url, "llama3:latest", None)));
            let instance_id = std::env::var("SYNOID_INSTANCE_ID").unwrap_or_else(|_| "default".to_string());
            let learner = AutonomousLearner::new(brain, &instance_id);

            learner.start();

            info!("Press Ctrl+C to stop.");
            // We need a longer timeout for downloads if we are being watched
            signal::ctrl_c().await?;
            learner.stop();
            info!("🛑 Autonomous Loop Stopped.");
        }

        Commands::React { goal, max_iterations } => {
            info!("🤖 ReAct Agent starting. Goal: {}", goal);
            let brain = core.brain.lock().await;
            match brain.run_react_goal(&goal, max_iterations).await {
                Ok(answer) => {
                    println!("\n✅ ReAct Answer:\n{}", answer);
                }
                Err(e) => error!("❌ ReAct Agent Error: {}", e),
            }
        }

        Commands::Gemma4 { task, max_steps, dry_run } => {
            use agent::gemma4_harness::Gemma4Harness;
            let work_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let harness = Gemma4Harness::new(&work_dir, dry_run);
            info!("🤖 Gemma4 Harness starting. Task: {}", task);
            if dry_run {
                info!("   [DRY RUN] No files will be written, no cargo invocations.");
            }
            match harness.run_task(&task, max_steps).await {
                Ok(summary) => println!("\n[Gemma4] Done:\n{}", summary),
                Err(e) => error!("❌ Gemma4 Harness error: {}", e),
            }
        }

        Commands::Gepa { insights, update, port, interval } => {
            use agent::brain::Brain;
            use agent::core_systems::gepa::GepaLoop;
            use tokio::signal;
            use tokio::sync::Mutex;

            if let Some(p) = port {
                if p != 3000 {
                    let instance_id = format!("_{}", p);
                    std::env::set_var("SYNOID_INSTANCE_ID", &instance_id);
                    info!("🔷 GEPA Instance: '{}' (port {})", instance_id, p);
                }
            }

            let instance_id = std::env::var("SYNOID_INSTANCE_ID")
                .unwrap_or_else(|_| "default".to_string());
            let brain = Arc::new(Mutex::new(Brain::new(&api_url, "llama3:latest", None)));
            let gepa = Arc::new(GepaLoop::new(brain, &instance_id));

            if insights {
                // Print insights report and exit
                gepa.generate_insights();
                return Ok(());
            }

            if update {
                // One-shot policy update and exit
                info!("[GEPA] Running one-shot policy update...");
                gepa.run_policy_update().await;
                info!("[GEPA] Done.");
                return Ok(());
            }

            // Default: run background improvement loop until Ctrl+C
            info!("🧠 Starting GEPA background loop (interval: {}s)...", interval);
            gepa.clone().start_background_loop(interval);
            info!("Press Ctrl+C to stop.");
            signal::ctrl_c().await?;
            gepa.stop_background_loop();
            info!("🛑 GEPA Loop Stopped.");
        }

        Commands::Transcribe { input } => {
            use synoid_core::agent::tools::production_tools;
            use synoid_core::agent::tools::transcription::{
                TranscriptionEngine, filter_hallucinations, generate_srt,
            };

            info!("[TRANSCRIBE] Input: {:?}", input);

            let tmp_wav = input.with_extension("_whisper_tmp.wav");
            let audio_path = match production_tools::extract_audio_wav(&input, &tmp_wav).await {
                Ok(p) => {
                    info!("[TRANSCRIBE] Audio extracted to {:?}", p);
                    p
                }
                Err(e) => {
                    error!("[TRANSCRIBE] Audio extraction failed: {}. Trying direct input.", e);
                    input.clone()
                }
            };

            let engine = TranscriptionEngine::new(None).await?;
            let segments = engine.transcribe(&audio_path).await?;
            let segments = filter_hallucinations(segments);

            // Clean up temp WAV
            if audio_path == tmp_wav {
                let _ = std::fs::remove_file(&tmp_wav);
            }

            let srt_path = input.with_extension("srt");
            let srt_content = generate_srt(&segments);
            std::fs::write(&srt_path, &srt_content)?;

            info!("[TRANSCRIBE] ✅ {} segments → {:?}", segments.len(), srt_path);
            println!("Saved: {:?}", srt_path);
        }
    }

    Ok(())
}
