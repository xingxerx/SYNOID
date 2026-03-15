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
- **Visual**: Vectorize footage to SVG for artistic effects, and use **Gemini Vision** for frame-by-frame context awareness (detecting UI elements, specific apps, etc.).
- **AI Core**: Direct the "Brain" with natural language powered by **Groq** and **Ollama**, run embodied agents, and learn editing styles. Enjoy real-time API key hot-reloading straight from the `.env` file via the Hive Status panel.
- **Reference Editing** ⭐NEW⭐: Use reference images to guide visual transformations with dual-mode editing (instruction + reference).
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

### 🎨 Reference-Guided Editing
SYNOID supports advanced reference-based video editing:

- **Dual-Mode Editing**: Combine natural language instructions with reference images for precise visual control
- **Style Transfer**: Apply cartoon, sketch, watercolor, or custom styles from reference images
- **Background Replacement**: Swap backgrounds using reference images as templates
- **Local Editing**: Modify, add, or remove specific objects (ML-ready architecture)
- **Temporal Consistency**: Maintains smooth, coherent motion across frames with adaptive blending
- **Latent-Space Optimization**: 40-60% faster processing with 50-70% less memory usage
- **Blend Control**: Adjustable strength between instruction-based and reference-based edits (0.0-1.0)

**Performance**: Achieves style transfer quality comparable to state-of-the-art models while maintaining SYNOID's Rust-powered speed advantage.

### 🛡️ Cyberdefense Sentinel (Experimental)
- **Process Monitoring**: Detect suspicious system activity and unauthorized processes.
- **File Integrity**: Watch directories for unauthorized changes to critical assets.
- **Continuous Guard**: Real-time system protection running in the background. *Disabled by default.*

---

## 🚀 Quick Start

### Prerequisites
- **Rust** 1.70+ — [rustup.rs](https://rustup.rs)
- **FFmpeg** (in PATH) — [ffmpeg.org](https://ffmpeg.org/download.html)
- **Node.js 18+** and **npm** — [nodejs.org](https://nodejs.org) *(required for the Remotion animation engine)*
- **yt-dlp** (for YouTube features)
- **Python 3** (for Voice/TTS features)
- **Ollama** (running `llama3:latest` or similar) — [ollama.com](https://ollama.com)

### First-time Setup

Before building, install the Remotion animation engine dependencies **once**:

```bash
cd remotion-engine
npm install
cd ..
```

> **Note:** If you skip this step, `cargo build` will show a warning but will not open a terminal or fail. Remotion-based animations simply won't render until `npm install` is run.

### Build
```bash
cargo build --release
```

### Run GUI

**With Live-Reloading (Recommended for development):**
Automatically recompiles on code changes without restarting the terminal.
```bash
cargo watch -x "run --release -- gui"
```

**Standard Run:**
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
> "# SYNOID Video Editing Instructions

## Learning Phase
First, analyze the example videos in `D:\SYNOID\Download` to understand proper gaming video editing patterns:
- Study pacing and engagement techniques
- Observe how cuts are used (or avoided) during gameplay
- Note audio quality standards and subtitle placement
- Learn the style and flow that maintains viewer retention

## Core Editing Tasks

### 1. Audio Enhancement (Priority #1)
Apply studio-quality audio processing to all voice tracks:
- EQ for clarity and presence
- Compression for consistent volume levels
- Noise reduction to eliminate background interference
- Normalization to broadcast standards (-16 LUFS)

**Critical**: Complete all audio enhancement BEFORE subtitle generation. Clean audio = accurate transcription.

### 2. Profanity Censoring (Priority #2)
Identify and beep out ALL curse words and slurs including but not limited to:
- fuck, fucking, fucked, motherfucker
- shit, shitty, bullshit
- bitch, ass, asshole
- nigga, nigger, negro
- [and any other specific terms to censor]

Use a clean, standard 1kHz beep tone. VERIFY every instance is caught - double-check the final output.

### 3. Subtitle Generation (Priority #3)
After audio is fully enhanced and profanity is beeped:
- Generate accurate, word-for-word captions
- Ensure readable timing (2-3 seconds per caption)
- Use high-contrast, easy-to-read styling
- Sync perfectly with enhanced audio

### 4. Preserve Gameplay Integrity
- DO NOT make random cuts or trim gameplay footage
- Maintain the complete core gameplay loop
- Only cut during natural breaks (loading screens, menus, deaths)
- Keep the raw, authentic gaming experience intact

## Final Quality Check
Before output, verify:
- [ ] All curse words are properly beeped
- [ ] Audio is clear, balanced, and professional
- [ ] Subtitles are accurate and readable
- [ ] Gameplay flow is preserved
- [ ] No random or jarring cuts

Focus on audio clarity and professional censoring over aggressive editing."

**Automated Journalism:**
> "Research 'Universal Editing tips on trick 2026', find top 5 relevant videos, and generate a summary script."

---

## 🤖 Advanced & Agentic Workflows

### 🧬 Recursive Learning Loop
When SYNOID is run using `cargo watch` while the **Autonomous Learning Loop** is active, it creates a powerful **self-recursive improvement cycle**:
1. **Scouting**: The agent downloads a high-quality video (e.g., from YouTube).
2. **Analysis**: It processes the video, identifies editing patterns, and updates its `EditingStrategy`.
3. **Trigger**: Writing the new strategy to `cortex_cache/` triggers `cargo watch` to recompile/restart the GUI.
4. **Resumption**: The GUI re-launches with a fresh "Brain" that immediately applies the newly learned patterns to the next video it processes.

This allows the agent to essentially "dream" and practice new styles in the background, becoming more accurate with every restart.

### 👥 Running Multiple Instances
You can run multiple independent SYNOID agents on the same machine by isolating their memory and ports:

**Instance A (Default):**
```bash
cargo run --release -- gui
```

**Instance B (Isolated):**
```bash
# Providing a different port now automatically isolates the state!
cargo run --release -- gui --port 3001
```


### 🎓 Isolated Autonomous Learning
To run a dedicated, isolated learning instance (e.g., to "teach" the agent while using the main instance), use the provided `teach.ps1` script. This handles all the complex environment variables and file locks for you.

```powershell
# Run an isolated learning instance on port 3001 (Recommended)
.\teach.ps1 3001
```

This script automatically:
- Sets a private **Cargo Home** and **Target Directory** to prevent file locks.
- Configures **Watch Ignores** for `target`, `Download`, and `cortex_cache` to prevent restart loops.
- Launches the `autonomous` mode on the specified port.


---

## 🏗️ Architecture

SYNOID is built on a modular "Brain-Cortex" architecture with organized subsystems:

```
src/
├── main.rs              # CLI Entry Point
├── window.rs            # Command Center GUI (eframe/egui)
├── editor_api.rs        # Video editing API
├── gpu_backend.rs       # GPU acceleration backend
└── agent/
    ├── core_systems/    # Brain, consciousness, learning, health
    │   ├── brain.rs           # AI Brain: Intent processing & Neuroplasticity
    │   ├── consciousness.rs   # Self-awareness and decision making
    │   ├── neuroplasticity.rs # Adaptive learning patterns
    │   ├── autonomous_learner.rs # Style learning from reference videos
    │   ├── learning.rs        # Knowledge accumulation
    │   ├── core.rs            # AgentCore: Central state manager
    │   ├── body.rs            # Physical manifestation
    │   └── health.rs          # System health monitoring
    │
    ├── ai_systems/      # LLM providers, reasoning, orchestration
    │   ├── llm_provider.rs    # Multi-provider LLM interface
    │   ├── gpt_oss_bridge.rs  # OSS model bridge (Ollama/Groq)
    │   ├── token_optimizer.rs # Token usage optimization
    │   ├── reasoning.rs       # Logical reasoning engine
    │   ├── moe.rs             # Mixture-of-Experts routing
    │   ├── supervisor.rs      # Multi-agent supervisor
    │   ├── multi_agent.rs     # Multi-agent coordination
    │   └── hive_mind.rs       # Distributed agent intelligence
    │
    ├── video_processing/ # Video editing, playback, style learning
    │   ├── video_editing_agent.rs # Smart video editing agent
    │   ├── video_player.rs        # Video playback engine
    │   ├── video_stitcher.rs      # Multi-clip stitching
    │   ├── video_style_learner.rs # Style pattern learning
    │   ├── multicam.rs            # Multi-camera sync & switching
    │   ├── animator.rs            # Motion & animation engine
    │   └── upscale_engine.rs      # Video upscaling (SeedVR2/Real-ESRGAN)
    │
    ├── tools/           # Audio, vision, transcription, research
    │   ├── audio_tools.rs     # Audio enhancement & analysis
    │   ├── vision_tools.rs    # Computer vision & frame analysis
    │   ├── transcription.rs   # Speech-to-text transcription
    │   ├── source_tools.rs    # Content sourcing (YouTube, etc.)
    │   ├── research_tools.rs  # AI-powered research
    │   └── production_tools.rs # FFmpeg & production utilities
    │
    ├── engines/         # Core processing engines and pipelines
    │   ├── super_engine.rs    # High-level orchestration engine
    │   ├── unified_pipeline.rs # Multi-stage workflow pipeline
    │   ├── motor_cortex.rs    # Execution engine & FFmpeg generation
    │   ├── editor_queue.rs    # Edit job queue manager
    │   └── process_utils.rs   # Process management utilities
    │
    ├── cuda/            # High-performance GPU computation
    │   ├── cuda_kernel_gen.rs # AI-powered CUDA kernel generation
    │   ├── cuda_pipeline.rs   # CUDA-enhanced pipeline
    │   ├── latent_optimizer.rs # Latent-space optimization
    │   └── cuda_skills/       # Pre-written CUDA kernels
    │
    ├── security/        # Defense, validation, safety systems
    │   ├── io_shield.rs       # I/O monitoring & protection
    │   ├── validation_gate.rs # Input validation
    │   ├── download_guard.rs  # Download safety checks
    │   ├── recovery.rs        # Error recovery & rollback
    │   └── defense/           # Sentinel & file integrity monitoring
    │
    └── specialized/     # Domain-specific agents and editors
        ├── reference_editor.rs # Reference-guided video editing
        ├── synoid_link.rs      # CUDA kernel execution bridge
        ├── global_discovery.rs # Content discovery agent
        ├── smart_editor/       # Intent-based smart editing
        └── academy/            # Style library & code analysis
```

### Module Organization

The codebase is organized into logical subsystems:

- **Core Systems**: Brain, consciousness, learning - the "mind" of SYNOID
- **AI Systems**: LLM integration, reasoning, multi-agent orchestration
- **Video Processing**: Video editing, playback, style learning, upscaling
- **Tools**: Specialized utilities for audio, vision, transcription, research
- **Engines**: Core processing pipelines and execution engines
- **CUDA**: GPU-accelerated computation and kernel generation
- **Security**: Defense mechanisms, validation, and safety systems
- **Specialized**: Domain-specific agents and advanced editing features

---

## 🔧 Configuration

Set environment variables in `.env`:
```env
# Core SYNOID Configuration
SYNOID_API_URL=http://localhost:11434/v1

# Optional reference-guided editing settings
GEMINI_API_KEY=your_gemini_api_key_here
ENABLE_REFERENCE_EDITING=true
ENABLE_LATENT_OPTIMIZATION=true
TEMPORAL_CONSISTENCY_DEFAULT=0.85
```

### Reference-Guided Editing Examples

**Style Transfer:**
```bash
cargo run --release -- process \
  --input gameplay.mp4 \
  --output cartoon.mp4 \
  --mode reference \
  --reference styles/cartoon.jpg \
  --intent "Make this look like a cartoon animation" \
  --temporal-consistency 0.9
```

**Background Replacement:**
```bash
cargo run --release -- process \
  --input interview.mp4 \
  --output cyberpunk.mp4 \
  --mode reference \
  --reference backgrounds/cyberpunk_city.jpg \
  --intent "Replace background with futuristic city" \
  --blend 0.8
```

**Dual-Mode (Instruction + Reference):**
```bash
cargo run --release -- process \
  --input vlog.mp4 \
  --output cinematic.mp4 \
  --mode dual \
  --reference grades/film.jpg \
  --intent "Apply film grain and cinematic color grading" \
  --blend 0.6
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

**Proprietary** — © 2026 xingxerx | SYNOID

All rights reserved. Unauthorized copying, modification, or distribution is prohibited.

---

<div align="center">

**Built with 🦀 Rust for maximum performance**

</div>
