# SYNOID Codebase Audit & Breakdown

**Date:** February 2026
**Status:** Beta
**Auditor:** Jules (AI Agent)

## 1. Executive Summary
SYNOID is an ambitious "Agentic Video Production Kernel" written in Rust. It combines video processing (FFmpeg), AI reasoning (LLMs), and infinite resolution upscaling (Vectorization) into a single autonomous tool.

The codebase has evolved significantly and the core "AI" features, including the Brain and Vision tools, now utilize real processing pipelines instead of mocks.

## 2. Component Breakdown

### ✅ What is Working

*   **CLI Infrastructure (`main.rs`)**:
    *   Robust command-line interface using `clap`.
    *   Commands for video manipulation, YouTube downloading, research, and unified processing.
*   **The "Brain" & LLM Bridge (`src/agent/brain.rs`, `gpt_oss_bridge.rs`)**:
    *   **Functional**: Connects to OpenAI-compatible APIs (like Ollama) for orchestration, planning, and dynamic orchestration. 
*   **Vision & Audio Tools (`src/agent/vision_tools.rs`, `audio_tools.rs`)**:
    *   **Functional**: Video scene detection via `ffprobe` and center-of-mass subject tracking are live. Audio transient analysis utilizes `ebur128`.
*   **Motor Cortex & Smart Editor (`src/agent/motor_cortex.rs`, `smart_editor.rs`)**:
    *   **Functional**: Dynamically sequences clips, cuts silence ("ruthless" routing), and applies cinematic looks using high-order logical evaluation.
*   **Source Tools (`src/agent/source_tools.rs`)**:
    *   **Functional**: Effective wrapper around `yt-dlp` for downloading YouTube videos with optional browser cookie authentication.
*   **Cyberdefense (`src/agent/defense/`)**:
    *   **Functional**: `Sentinel` (process monitoring) and `IntegrityGuard` (file watching) have working implementations for basic system monitoring.
*   **Voice Engine (`src/agent/voice/`)**:
    *   **Functional**: Audio recording and basic Text-to-Speech (TTS) via `candle` transformers are implemented.

### ⚠️ What Needs Updating / Missing

*   **GPU Acceleration**:
    *   While CUDA is referenced, portions remain simulated or fall back to CPU. Expanding robust native `cudarc` pipelines is an ongoing objective.
*   **Smart Editor Modularity**:
    *   The `smart_editor.rs` sits at a large ~43KB file size. It should be split into focused sub-modules (scene ops, filter ops, transition ops) in a future refactor.

## 3. Architecture Overview

```
src/
├── main.rs              # Entry point & CLI routing
├── agent/
│   ├── brain.rs         # Intent classification & Agent execution
│   ├── motor_cortex.rs  # Action executor (FFmpeg builder)
│   ├── gpt_oss_bridge.rs# OpenAI-compatible API connectivity
│   ├── vector_engine.rs # Raster -> Vector -> Raster pipeline
│   ├── voice/           # Audio/TTS subsystem
│   ├── defense/         # Security subsystem
│   └── ..._tools.rs     # Helpers for Vision, Source, Audio
```

## 4. Roadmap & Recommendations

To propel further to 1.0, the following steps are recommended:

1.  **Smart Editor Refactoring**:
    *   Split `smart_editor.rs` into specialized files as features scale up.
2.  **Hardware Acceleration Deep Integration**:
    *   Expand `cudarc` usage explicitly throughout all encoding/decoding loops, moving beyond basic neural heuristics.
3.  **Advanced Motor Cortex Control**:
    *   Develop a more robust topological `EditGraph` implementation if non-linear edit graphs become overly complex for the current JSON plan interface.
