# SYNOID

<div align="center">

**Agentic Video Production Kernel**

*Autonomous AI-powered video editing, voice cloning, and vector stylization*

[![Rust](https://img.shields.io/badge/Rust-1.70+-orange?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-Proprietary-blue?style=flat-square)]()
[![Platform](https://img.shields.io/badge/Platform-Windows%20|%20Linux-green?style=flat-square)]()

</div>

---

## 🌌 Overview

**SYNOID** is an advanced **Agentic Video Production Kernel** designed to revolutionize content creation through autonomous AI. Unlike traditional tools that require manual frame-by-frame manipulation, SYNOID understands **creative intent**, allowing users to direct complex video production workflows using natural language.

Built on a high-performance **Rust** foundation, SYNOID integrates a suite of cutting-edge technologies into a single, cohesive **Command Center**:
- **Semantic Understanding**: Deconstructs video content to identify "boring" vs "action" segments based on your directives.
- **Vector Stylization / Artistic Upscaling**: Converts raster footage into resolution-independent vector graphics for unique artistic upscaling (best for animation/graphics).
- **Neural Synthesis (Experimental)**: Interfaces with external tools to clone voices and generate neural speech.
- **Active Defense (Optional)**: Can monitor your workspace with a background sentinel for unauthorized system activity (experimental).
- **Neuroplasticity**: The system learns from your edits and rendering successes to optimize future performance.

Whether you are repurposing long-form content, restoring legacy footage, or building automated media pipelines, SYNOID provides the intelligent infrastructure to execute with precision and speed.

---

## ✨ Features

### 🎬 Command Center GUI
The heart of SYNOID is the **Command Center**, a premium dark-mode interface organizing all capabilities into a streamlined workflow:

- **Media**: Upload videos (YouTube/Local), clip segments, and strictly compress files without quality loss.
- **Visual**: Vectorize footage to SVG for artistic effects.
- **AI Core**: Direct the "Brain" with natural language, run embodied agents, and learn editing styles.
- **Voice Studio**: Unified interface to record samples and generate speech (Simulated/Experimental).
- **Security**: Monitor system integrity and active processes with the Cyberdefense Sentinel (Experimental).
- **Research**: AI-powered topic research and video sourcing.

### ⚡ Viral Video Transformation
Transform raw footage into high-retention content automatically:
- **Ruthless Editing**: Automatically detects and trims dead air, hesitation, and silence (-40dB threshold).
- **Audio Enhancement**: Applies studio-quality EQ, compression, and normalization to voice tracks.
- **Engagement Consolidator**: Intelligently structures video for maximum viewer retention.

### 🎭 Funny Mode & Smart Transitions
- **Funny Mode**: Injects AI commentary and detects humorous moments. *Requires local Ollama setup and external Python scripts.*
- **Smart Transitions**: Analyzes scene motion to select the perfect transition:
    - *High Motion* → **Wipe/Slide**
    - *Medium Motion* → **Mix/Crossfade**
    - *Speech/Dialogue* → **Seamless Cut**
    - *Static/Low Motion* → **ZoomPan**

### 🔎 Vector Stylization Engine
- **Vector Upscaling**: Convert raster video to SVG, scale infinitely, re-render at any resolution. *Note: This produces a "vector art" style, not photorealistic super-resolution.*
- **Vectorization**: Export video frames as resolution-independent SVGs.
- **16K Safety Limit**: Automatic safeguards for extreme upscales.

### 🛡️ Cyberdefense Sentinel (Experimental)
- **Process Monitoring**: Detect suspicious system activity and unauthorized processes.
- **File Integrity**: Watch directories for unauthorized changes to critical assets.
- **Continuous Guard**: Real-time system protection running in the background. *Disabled by default.*

---

## 🚀 Quick Start

### Prerequisites
- **Rust** 1.70+
- **FFmpeg** (in PATH)
- **yt-dlp** (for YouTube features)
- **Python 3** (for Voice/TTS features)
- **Ollama** (running `deepseek-r1:14b` or similar)

### Build
```bash
cargo build --release
```

### Run GUI
```bash
cargo run --release -- gui
```

---

## 📖 Usage

Launch the **Command Center**:

```bash
cargo run --release -- gui
```

### 🧠 Creative Intent Examples

**Viral Clip Generation:**
> Take this raw footage and apply ruthless editing to remove all silence. Enhance the audio for podcast quality and make it punchy. Ensure the final cut maintains a rhythm suitable for a 40-50 minute duration. This operation falls under the Video Production module, specifically utilizing the Smart Editor for semantic intent processing and Production Tools for audio enhancement. Remove any curse words or inappropiate languages and replace it with a funny sound effect.

**Automated Journalism:**
> "Research 'Universal Editing tips on trick 2026', find top 5 relevant videos, and generate a summary script."

---

## 🏗️ Architecture

SYNOID is built on a modular "Brain-Cortex" architecture:

```
src/
├── main.rs              # CLI Entry Point
├── window.rs            # Command Center GUI (eframe/egui)
└── agent/
    ├── core.rs          # AgentCore: The central state manager ("The Ghost")
    ├── brain.rs         # AI Brain: Intent processing & Neuroplasticity
    ├── motor_cortex.rs  # MotorCortex: Execution engine & FFmpeg generation
    ├── unified_pipeline.rs # Pipeline: Orchestrates multi-stage workflows
    ├── vector_engine.rs # Vector Engine: SVG conversion & upscaling
    ├── voice/           # Voice Engine: Cloning, TTS, Transcription
    ├── defense/         # Sentinel: Cyberdefense & Integrity monitoring
    ├── academy/         # Style Library & Learning
    └── tools/           # Vision, Audio, Source, Production tools
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
| `candle-*` | Neural network inference (Voice/LLM) |
| `rodio/cpal` | Audio I/O |
| `eframe` | Native GUI (Command Center) |
| `symphonia` | Audio decoding/analysis |
| `axum` | Web Server & Dashboard API |

---

## 📜 License

**Proprietary** — © 2026 Xing_The_Creator | SYNOID

All rights reserved. Unauthorized copying, modification, or distribution is prohibited.

---

<div align="center">

**Built with 🦀 Rust for maximum performance**

</div>
