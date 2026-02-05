# SYNOID

<div align="center">

**Agentic Video Production Kernel**

*Autonomous AI-powered video editing, voice cloning, and infinite resolution upscaling*

[![Rust](https://img.shields.io/badge/Rust-1.70+-orange?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-Proprietary-blue?style=flat-square)]()
[![Platform](https://img.shields.io/badge/Platform-Windows%20|%20Linux-green?style=flat-square)]()

</div>

---

## 🌌 Overview

**SYNOID** is an advanced **Agentic Video Production Kernel** designed to revolutionize content creation through autonomous AI. Unlike traditional tools that require manual frame-by-frame manipulation, SYNOID understands **creative intent**, allowing users to direct complex video production workflows using natural language.

Built on a high-performance **Rust** foundation, SYNOID integrates a suite of cutting-edge technologies into a single, cohesive workstation:
- **Semantic Understanding**: Deconstructs video content to identify "boring" vs "action" segments based on your directives.
- **Infinite Resolution**: Converts raster footage into resolution-independent vector graphics for limitless upscaling.
- **Neural Synthesis**: Clones voices and generates neural speech for dynamic audio production.
- **Active Defense**: Protects your workspace with a background sentinel that monitors for unauthorized system activity.

Whether you are repurposing long-form content, restoring legacy footage, or building automated media pipelines, SYNOID provides the intelligent infrastructure to execute with precision and speed.

---

## ✨ Features

### 🎬 Video Production
- **YouTube Integration** — Download and process videos with custom creative intent
- **Smart Clipping** — Extract segments with precise timestamps
- **Compression** — Target-size compression without quality loss
- **Embodied Editing** — AI understands your intent and generates FFmpeg commands

### 🔎 Infinite Resolution Engine
- **Vector Upscaling** — Convert raster video to SVG, scale infinitely, re-render at any resolution
- **Vectorization** — Export video frames as resolution-independent SVGs
- **16K Safety Limit** — Automatic safeguards for extreme upscales

### 🗣️ Voice Cloning & Neural TTS
- **Voice Recording** — Capture voice samples directly
- **Voice Profiles** — Create reusable speaker embeddings
- **Neural TTS** — Generate speech with cloned or default voices
- **HuggingFace Integration** — Download state-of-the-art TTS models

### 🛡️ Cyberdefense Sentinel
- **Process Monitoring** — Detect suspicious system activity
- **File Integrity** — Watch directories for unauthorized changes
- **Continuous Guard** — Real-time system protection

### 🧠 AI Brain
- **Intent Processing** — Natural language command understanding
- **Style Learning** — Analyze and replicate editing styles
- **Edit Suggestions** — AI-powered video improvement recommendations

---

## 🚀 Quick Start

### Prerequisites
- **Rust** 1.70+
- **FFmpeg** (in PATH)
- **yt-dlp** (for YouTube features)

### Build
```bash
cargo build --release
```

### Run GUI
```bash
cargo run --release -- gui
```

---

## 📖 CLI Commands

### Video Processing
```bash
# Download YouTube video
synoid-core youtube --url "https://youtube.com/watch?v=..." --intent "make it cinematic"

# Clip video segment
synoid-core clip --input video.mp4 --start 10.0 --duration 30.0

# Compress to target size
synoid-core compress --input video.mp4 --size 25.0 --output small.mp4
```

### Vector Engine
```bash
# Vectorize video to SVG frames
synoid-core vectorize --input video.mp4 --output ./svg_frames

# Infinite upscale (2x)
synoid-core upscale --input video.mp4 --scale 2.0 --output upscaled.mp4
```

### Voice Engine
```bash
# Record voice sample
synoid-core voice --record 10 --output my_voice.wav

# Download TTS model
synoid-core voice --download

# Clone voice and create profile
synoid-core voice --clone sample.wav --profile "MyVoice"

# Generate speech with cloned voice
synoid-core voice --speak "Hello world" --profile "MyVoice" --output speech.wav
```

### Cyberdefense
```bash
# Start Sentinel (all monitors)
synoid-core guard --mode all --watch ./important_files

# Process monitoring only
synoid-core guard --mode sys
```

### AI Brain
```bash
# Direct brain command
synoid-core run --request "analyze this video for pacing"

# Embodied agent
synoid-core embody --input raw.mp4 --intent "add dramatic color grading" --output final.mp4

# Learn editing style
synoid-core learn --input reference.mp4 --name "cinematic"

# Get edit suggestions
synoid-core suggest --input draft.mp4
```
Try This command for the Creative Intent if you don't have one " Edit this video,  fix the video first create the transcript of the video try your best to make our the user's voice. Up scale and enhance the user's voice so it's more audible so we can hear better "
---

## 🏗️ Architecture

```
src/
├── main.rs              # CLI entry point
├── window.rs            # eframe GUI
└── agent/
    ├── brain.rs         # AI intent processor
    ├── motor_cortex.rs  # Execution engine
    ├── vector_engine.rs # SVG vectorization & upscaling
    ├── vector_video.rs  # Animated SVG video engine
    ├── voice/           # Voice cloning & TTS
    ├── defense/         # Cyberdefense sentinel
    ├── production_tools.rs
    ├── source_tools.rs
    ├── vision_tools.rs
    └── audio_tools.rs
```

---

## 🔧 Configuration

Set environment variables in `.env`:
```env
SYNOID_API_URL=http://localhost:11434/v1
```

---

## 📦 Dependencies

| Crate | Purpose |
|-------|---------|
| `vtracer` | Raster to vector conversion |
| `resvg` | SVG rendering engine |
| `rayon` | Parallel frame processing |
| `candle-*` | Neural network inference |
| `rodio/cpal` | Audio I/O |
| `eframe` | Native GUI |
| `clap` | CLI argument parsing |

---

## 📜 License

**Proprietary** — © 2026 Xing_The_Creator | SYNOID

All rights reserved. Unauthorized copying, modification, or distribution is prohibited.

---

<div align="center">

**Built with 🦀 Rust for maximum performance**

</div>
