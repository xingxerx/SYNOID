// SYNOID™ CLI Arguments
// Copyright (c) 2026 Xing_The_Creator | SYNOID™

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "synoid-core")]
#[command(about = "SYNOID™ Agentic Kernel", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
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
        /// Monitor Mode (Process/File)
        #[arg(short, long, default_value = "all")]
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

        /// Download TTS model
        #[arg(long)]
        download: bool,
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
}
