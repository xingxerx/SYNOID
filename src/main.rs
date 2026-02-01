// SYNOID™ Main Entry Point
// Copyright (c) 2026 Xing_The_Creator | SYNOID™

mod agent;
mod window;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::{info, error};
use dotenv::dotenv;

#[derive(Parser)]
#[command(name = "synoid-core")]
#[command(about = "SYNOID™ Agentic Kernel", long_about = None)]
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
    
    /// Run the Brain directly
    Run {
        #[arg(short, long)]
        request: String,
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    tracing_subscriber::fmt::init();
    
    info!("--- SYNOID™ AGENTIC KERNEL v0.1.0 ---");
    
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
            
            if !agent::source_tools::check_ytdlp() {
                error!("yt-dlp not found! Please install it via pip.");
                return Ok(());
            }

            let source_info = agent::source_tools::download_youtube(&url, output_dir, login.as_deref()).await?;
            println!("✅ Video acquired: {}", source_info.title);
            
            let output_path = output.unwrap_or_else(|| PathBuf::from("output.mp4"));
            
            // Placeholder for full pipeline trigger
            info!("Ready to process '{}' with intent: {}", source_info.title, intent);
        },
        Commands::Research { topic, limit } => {
            info!("🕵️ Researching topic: {}", topic);
            let results = agent::source_tools::search_youtube(&topic, limit).await?;
            
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
        }
    }

    Ok(())
}
