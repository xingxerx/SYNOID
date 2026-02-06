# SYNOID Codebase Audit & Breakdown

**Date:** October 2023
**Status:** Alpha / Prototype
**Auditor:** Jules (AI Agent)

## 1. Executive Summary
SYNOID is an ambitious "Agentic Video Production Kernel" written in Rust. It aims to combine video processing (FFmpeg), AI reasoning (LLMs), and infinite resolution upscaling (Vectorization) into a single autonomous tool.

The current codebase establishes a strong architectural skeleton but relies heavily on stubs and mock data for its advanced "AI" features. The core media processing capabilities (downloading, simple trimming, vectorization) are functional.

## 2. Component Breakdown

### ✅ What is Working

*   **CLI Infrastructure (`main.rs`)**:
    *   Robust command-line interface using `clap`.
    *   Commands for `youtube`, `clip`, `compress`, `vectorize`, `upscale`, and `voice` are wired up.
*   **Vector Engine (`src/agent/vector_engine.rs`)**:
    *   **Functional**: Can convert raster video frames to SVG using `vtracer` and re-render them at higher resolutions.
    *   **Limitation**: CPU-only. Heavy performance cost.
*   **Source Tools (`src/agent/source_tools.rs`)**:
    *   **Functional**: Effective wrapper around `yt-dlp` for downloading YouTube videos with optional browser cookie authentication.
    *   **Functional**: `ffprobe` integration for getting video duration.
*   **Cyberdefense (`src/agent/defense/`)**:
    *   **Functional**: `Sentinel` (process monitoring) and `IntegrityGuard` (file watching) have working implementations for basic system monitoring.
*   **Voice Engine (`src/agent/voice/`)**:
    *   **Functional**: Audio recording and basic Text-to-Speech (TTS) via `candle` transformers are implemented.

### ⚠️ What Needs Updating / Missing

*   **The "Brain" (`src/agent/brain.rs`, `gpt_oss_bridge.rs`)**:
    *   **Status**: Heuristic-based.
    *   **Issue**: The "Complex Request" handler is a stub. It prints logs but does not connect to any actual LLM (Local or Cloud). It cannot truly "reason" about intents yet.
*   **Vision Tools (`src/agent/vision_tools.rs`)**:
    *   **Status**: Mocked.
    *   **Issue**: `scan_visual` attempts to call `ffprobe` for scene detection but ignores the output and returns hardcoded "dummy" scenes. Real scene detection is missing.
*   **Motor Cortex (`src/agent/motor_cortex.rs`)**:
    *   **Status**: Basic.
    *   **Issue**: "Embodied" editing is limited to applying a fixed crop and LUT. It cannot dynamically cut or rearrange video segments based on content.
*   **GPU Acceleration**:
    *   **Status**: Disabled/Stubbed.
    *   **Issue**: `Commands::Gpu` is a print stub. `upscale_video_cuda` returns a "not supported" error. The `cudarc` dependency is commented out in `Cargo.toml`.

## 3. Architecture Overview

```
src/
├── main.rs              # Entry point & CLI routing
├── agent/
│   ├── brain.rs         # Intent classification (Rules + LLM)
│   ├── motor_cortex.rs  # Action executor (FFmpeg builder)
│   ├── vector_engine.rs # Raster -> Vector -> Raster pipeline
│   ├── voice/           # Audio/TTS subsystem
│   ├── defense/         # Security subsystem
│   └── ..._tools.rs     # Helpers for Vision, Source, Audio
```

## 4. Roadmap & Recommendations

To move from "Prototype" to "Product", the following steps are recommended:

1.  **Connect the Brain**:
    *   Implement real HTTP calls in `gpt_oss_bridge.rs` to connect to a local OpenAI-compatible API (e.g., Ollama).
    *   *Action Plan*: Immediate priority.

2.  **Enable Real Vision**:
    *   Parse `ffprobe` scene detection output in `vision_tools.rs` to give the agent actual eyes.
    *   *Action Plan*: Immediate priority.

3.  **Expand Motor Cortex**:
    *   Implement an `EditGraph` that can sequence multiple clips, not just filter a single one.

4.  **Hardware Acceleration**:
    *   Re-enable `cudarc` or use `wgpu` for hardware-accelerated vector rendering and inference.
