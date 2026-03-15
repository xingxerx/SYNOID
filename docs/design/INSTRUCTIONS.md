# SYNOID — Instruction Manual
**Version:** Beta (v0.1.1) | **Copyright © 2026 xingxerx. All Rights Reserved.**

---

## Table of Contents
1. [What is SYNOID?](#what-is-synoid)
2. [System Requirements](#system-requirements)
3. [Prerequisites](#prerequisites)
4. [Installation & Build](#installation--build)
5. [Configuration](#configuration)
6. [Running SYNOID](#running-synoid)
7. [CLI Command Reference](#cli-command-reference)
8. [AI Creative Intent Examples](#ai-creative-intent-examples)
9. [Module Overview](#module-overview)
10. [Dashboard & GUI](#dashboard--gui)
11. [Cyberdefense & Security](#cyberdefense--security)
12. [Recovery System](#recovery-system)
13. [Code Standards & Architecture](#code-standards--architecture)
14. [Testing](#testing)
15. [Troubleshooting](#troubleshooting)
16. [License](#license)

---

## What is SYNOID?

SYNOID is an **Agentic Video Production Kernel** written in Rust. It combines high-performance video editing, AI reasoning, and infinite-resolution vector upscaling into a single autonomous tool — controlled entirely through natural language intent.

**Key differentiators:**
- Natural language creative intent → fully edited video output
- Vector-based infinite resolution upscaling (no quality ceiling)
- Built-in cyberdefense (Sentinel + IntegrityGuard)
- Modular "Brain-Cortex" AI architecture with Neuroplasticity & Style Learning
- Multi-provider LLM support (Ollama / OpenAI-compatible APIs / Google AI Studio)

---

## System Requirements

| Component | Minimum | Recommended |
|-----------|---------|-------------|
| **RAM** | 16 GB | 32 GB (for 4K) |
| **CPU** | AMD Ryzen 5 (6c/12t) | Intel i9-7900X (10c/20t) |
| **GPU** | NVIDIA GTX 1050 | GTX 1080Ti / RTX series |
| **Storage (OS)** | SSD | NVMe SSD |
| **Storage (Footage)** | 3–4× source file size | HDD 7200 RPM minimum |

> **Note:** If less than 16 GB RAM is detected, SYNOID automatically enters **Degraded Performance Mode** (LSP optimizations enabled, cache priming disabled).

---

## Prerequisites

Before building, install the following tools:

### 1. Rust Toolchain
```bash
rustc --version
cargo --version
```
Install via [rustup.rs](https://rustup.rs) if not present.

### 2. FFmpeg
Must be installed and available in your system `PATH`.
```bash
ffmpeg -version
```

### 3. yt-dlp (for YouTube features)
```bash
pip install yt-dlp
```
Or install the standalone binary and place it in your `PATH`.

### 4. Optional: Ollama (for local LLM)
Needed if using local AI models instead of cloud APIs.
```bash
ollama serve
```

---

## Installation & Build

```bash
# 1. Clone the repository
git clone https://github.com/ooples/token-optimizer-mcp.git
cd SYNOID

# 2. Build the release binary
cargo build --release
```

> The first build will take several minutes to compile all dependencies. Subsequent builds are incremental and much faster.

---

## Configuration

Create a `.env` file in the project root:

```env
# Local Ollama endpoint (default)
SYNOID_API_URL=http://localhost:11434/v1

# Optional: Google AI Studio for vision analysis
# GOOGLE_API_KEY=your_key_here
```

**Environment variables:**

| Variable | Default | Description |
|----------|---------|-------------|
| `SYNOID_API_URL` | `http://localhost:11434/v1` | LLM API endpoint (Ollama or OpenAI-compatible) |
| `SYNOID_INSTANCE_ID` | *(auto-set)* | Instance identifier for multi-instance runs |

---

## Running SYNOID

### Launch the GUI (Command Center)
```bash
cargo run --release -- gui
```
Opens the full-featured web dashboard. Navigate to `http://localhost:3000` in your browser.

### Run a Second Instance
```bash
cargo run --release -- gui --port 3001
```

---

## CLI Command Reference

### YouTube Download & Process
```bash
cargo run --release -- youtube \
  --url "https://youtu.be/VIDEO_ID" \
  --intent "make it cinematic" \
  --output "./output.mp4" \
  --chunk-minutes 10 \
  --login chrome          # Optional: borrow browser cookies for auth
```

### Trim / Clip a Video
```bash
cargo run --release -- clip \
  --input "my_video.mp4" \
  --start 30 \
  --end 90 \
  --output "clipped.mp4"
```

### Vectorize Video (Infinite Resolution)
```bash
cargo run --release -- vectorize \
  --input "my_video.mp4" \
  --output "./vectors"
```

### Upscale Video (2× to 4K)
```bash
cargo run --release -- upscale \
  --input "input.mp4" \
  --output "upscaled.mp4" \
  --scale 2.0
```

### Autonomous Research
```bash
cargo run --release -- research \
  --topic "video editing tips 2026" \
  --limit 5
```

### Voice Engine

**Record a voice sample:**
```bash
cargo run --release -- voice --record 10 --output "sample.wav"
```

**Clone a voice profile:**
```bash
cargo run --release -- voice --clone "sample.wav" --profile "MyVoice"
```

**Text-to-Speech with a cloned profile:**
```bash
cargo run --release -- voice --speak "Hello World" --profile "MyVoice"
```

### Cyberdefense Sentinel
```bash
cargo run --release -- guard --mode file --watch "./important_files"
```

---

## AI Creative Intent Examples

SYNOID accepts free-form natural language as a creative directive. Below are example intents you can pass via the GUI or CLI.

### Viral Clip Generation
> *"Utilize the Research module to source and download the top 5 most engaging YouTube videos related to popular games. Analyze their pacing and engagement style. Feed this analysis into the Academy module to establish a new Style Library. Apply these patterns to my raw footage — do not make random cuts. Instead, identify and bleep all cuss words, apply studio-quality EQ and compression to the voice tracks, and generate 100% accurate on-screen captions. Structure the final video using the Engagement Consolidator."*

### Automated Journalism
> *"Research 'Universal Editing Tips 2026', find the top 5 relevant videos, and generate a summary script."*

### Clean + Caption
> *"Remove all dead air and silence from my footage, normalize the audio, and add accurate captions."*

---

## Module Overview

| Module | Location | Description |
|--------|----------|-------------|
| **Brain** | `src/agent/brain.rs` | Intent classification, AI orchestration, Neuroplasticity |
| **Motor Cortex** | `src/agent/motor_cortex.rs` | Action executor, FFmpeg pipeline builder |
| **GPT/OSS Bridge** | `src/agent/gpt_oss_bridge.rs` | Multi-provider LLM connectivity (Ollama, OpenAI, Gemini) |
| **Smart Editor** | `src/agent/smart_editor.rs` | Silence cutting, cinematic looks, dynamic clip sequencing |
| **Vector Engine** | `src/agent/vector_engine.rs` | Raster → SVG → Raster (infinite upscaling pipeline) |
| **Vision Tools** | `src/agent/vision_tools.rs` | Scene detection, subject tracking, frame annotation |
| **Audio Tools** | `src/agent/audio_tools.rs` | Transient analysis, EBU R128 loudness, EQ/compression |
| **Source Tools** | `src/agent/source_tools.rs` | yt-dlp wrapper for YouTube downloading |
| **Voice Engine** | `src/agent/voice/` | TTS, voice cloning, audio recording via Candle transformers |
| **Cyberdefense** | `src/agent/defense/` | Sentinel (process monitoring) + IntegrityGuard (file watching) |
| **Academy** | `src/agent/academy/` | Style Library, autonomous learning from analyzed videos |
| **Recovery** | `src/agent/recovery.rs` | Crash-proof state persistence (`recovery_manifest.json`) |
| **Unified Pipeline** | `src/agent/unified_pipeline.rs` | Orchestrates multi-stage end-to-end workflows |
| **Upscale Engine** | `src/agent/upscale_engine.rs` | SeedVR2 / Real-ESRGAN / Lanczos upscaling |
| **Multicam** | `src/agent/multicam.rs` | AI multicam sync & SmartSwitch |
| **Edit Graph** | `src/engine/graph.rs` | DAG-based node graph (ComfyUI-style pipeline) |

---

## Dashboard & GUI

The web-based **Command Center** is available at `http://localhost:3000` after launching the GUI.

**Sidebar navigation includes:**
- **Project Overview** — Active tasks, team presence, sprint stats
- **AI Co-pilot** — Natural language intent input
- **Focus Tasks** — Current task board
- **Omni Viewer** — Media browser and preview
- **Analytics** — Project metrics
- **Settings / Sign Out**

The dashboard supports real-time team activity feeds, search across projects and files, and AI-powered task suggestions.

---

## Cyberdefense & Security

SYNOID includes a built-in security layer:

- **Sentinel** — Monitors system processes for anomalies.
- **IntegrityGuard** — Watches specified file directories for unauthorized changes.
- **IO Shield** (`src/agent/io_shield.rs`) — All file operations must be routed through this module.

> **Rule:** Any feature that performs file I/O must go through the Sentinel/IO Shield modules. Never write raw file paths without validation.

---

## Recovery System

SYNOID implements a "Black Box" crash-recovery system:

- On any interruption (crash or `Ctrl-C`), the current render state is automatically serialized to:
  ```
  .synoid/cortex_cache/recovery_manifest.json
  ```
- On next launch, SYNOID detects this manifest and offers to **resume from the last verified frame**.

**Manifest fields:** project name, last frame index, last active intent, hardware state, timestamp, and completed chunk paths.

---

## Code Standards & Architecture

### Actor Model
Cross-module communication uses `tokio::sync::mpsc` channels.

### Engine Pattern
Every engine (video, vector, voice) implements the `Plugin` trait.

### Error Handling
- Libraries: use `thiserror` for typed error enums.
- Binaries: use `anyhow` for flexible error propagation.

### Async Rules
- Runtime: `tokio`
- Never call blocking operations inside `async` functions.

### FFmpeg Safety
- Always validate input before spawning FFmpeg processes.
- All FFI calls must be wrapped in `unsafe` blocks with documentation.

### Feature Addition Checklist
When adding any new feature, always provide:
- [ ] CLI flag (in `main.rs`)
- [ ] GUI menu item (in `window.rs`)
- [ ] Config file support

### Refactoring Rule
Prefer moving code to new crates/modules over letting any single file exceed **500 lines**.

---

## Testing

```bash
# Run unit tests
cargo test

# Run integration tests (requires ffmpeg in PATH)
cargo test --features integration

# FFmpeg-dependent tests are marked #[ignore] by default
cargo test -- --ignored
```

---

## Troubleshooting

| Problem | Solution |
|---------|----------|
| **Build errors** | Ensure all system libraries and `pkg-config` are installed. |
| **FFmpeg not found** | Add the FFmpeg binary folder to your system `PATH`. |
| **yt-dlp not found** | Run `pip install yt-dlp` or place the standalone binary in `PATH`. |
| **Memory issues during 4K upscale** | Reduce thread count via `rayon` config in `src/main.rs`. |
| **Hive Mind offline** | Start Ollama with `ollama serve`. SYNOID continues in degraded mode without it. |
| **Dashboard not loading** | Check that port `3000` is not in use. Run a second instance on `--port 3001`. |
| **Recovery manifest detected on startup** | SYNOID will prompt to resume the last session. Delete `.synoid/cortex_cache/recovery_manifest.json` to start fresh. |

---

## License

SYNOID is distributed under the **SYNOID Shared Improvement License (SIL) v1.0**.

- ✅ Personal use and study permitted.
- ✅ Improvements must be shared back under the same license.
- ❌ Commercial use, resale, or redistribution without explicit written permission is prohibited.
- ❌ Forking to create competing products is prohibited.

**© 2026 xingxerx. All Rights Reserved.**
