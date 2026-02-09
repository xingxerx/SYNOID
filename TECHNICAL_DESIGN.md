# SYNOID AI Video Production Kernel - Technical Design Document

## 1. System Overview
SYNOID is an "Agentic Video Production Kernel" designed to revolutionize content creation by shifting the paradigm from manual frame manipulation to "Creative Intent." Users provide natural language directives, and the system's AI agents execute the technical tasks. The kernel is built on Rust for high performance and memory safety, leveraging a modular architecture to handle video production, vectorization, voice synthesis, and cyberdefense.

## 2. System Architecture
The application is structured into specific modules orchestrated by a central kernel.

### Directory Structure
- **src/main.rs**: The Command Line Interface (CLI) entry point.
- **src/window.rs**: The Native GUI implementation (using `eframe`).
- **src/agent/**: The core intelligence modules.
    - **brain.rs**: Intent processor (translates natural language to technical commands).
    - **motor_cortex.rs**: Execution engine (runs FFmpeg, downloads files, etc.).
    - **vector_engine.rs** & **vector_video.rs**: Handles raster-to-vector conversion for infinite resolution.
    - **voice/**: Handles Text-to-Speech (TTS) and voice cloning.
    - **defense/**: The cyberdefense sentinel system.

### Key Dependencies
- **vtracer**: Vectorizes raster video frames into SVG.
- **resvg**: Renders SVGs back into video frames.
- **rayon**: Enables parallel processing for frame operations.
- **candle-**: Provides machine learning inference for local AI models.
- **rodio/cpal**: Handles audio input and output.
- **eframe**: Powers the native GUI.
- **ffmpeg**: Required system dependency for video manipulation.
- **yt-dlp**: Required system dependency for downloading content.

## 3. Core Feature Modules

### Module A: Video Production (The "Motor Cortex")
Handles physical manipulation of video files based on professional workflow principles (Ingest -> Rough Cut -> Output).
- **Ingest**: Integrates with YouTube via `synoid-core youtube` to download and process video based on intent.
- **Assembly**: Smart Clipping extracts segments using timestamps.
- **Compression**: Optimizes file size without significant quality loss.
- **Embodied Editing**: Generates FFmpeg commands from natural language intent.

### Module B: Infinite Resolution Engine
Bypasses resolution limits by converting pixels to vectors.
- **Vectorization**: Converts raster frames to SVG, making them resolution-independent.
- **Vector Upscaling**: Scales SVGs infinitely to re-render at 4K, 8K, etc.
- **Safety Limit**: Implements a "16K Safety Limit" (16384px) to prevent system crashes during extreme upscaling.

### Module C: Neural Synthesis (Audio Engine)
Ensures audio is a primary driver of storytelling.
- **Voice Cloning**: Captures voice samples to create reusable "Voice Profiles."
- **Neural TTS**: Generates speech from text using cloned profiles.
- **HuggingFace Integration**: Automatically downloads state-of-the-art TTS models.

### Module D: AI Brain & Intent Processing
Interprets user commands and learns styles.
- **Intent Understanding**: Deconstructs directives (e.g., "fix the video") into actionable steps.
- **Style Learning**: Analyzes reference videos to learn editing styles (pacing, color grading).
- **Semantic Segmentation**: Identifies "boring" vs. "action" segments for automated rough cuts.

### Module E: Cyberdefense Sentinel
Ensures system integrity during operation.
- **Active Defense**: Monitors for unauthorized system activity.
- **File Integrity**: Watches specific directories for unauthorized changes.

## 4. Workflows

### Reproducing the "Creative Intent" Workflow
1.  **User Prompt**: "Edit this video, fix the video... make out the user's voice..."
2.  **System Action**:
    -   **Analyze Audio**: Extract audio track.
    -   **Transcribe**: Generate transcript via STT.
    -   **Enhance Audio**: Apply EQ/Compression/Denoise.
    -   **Upscale Video**: Pass frames through `vector_engine`.
    -   **Render**: Combine upscaled video with enhanced audio.

### Ethical AI Considerations
-   **Watermarking**: Implement SynthID or similar to prevent misuse.
-   **False Memories**: acknowledge that AI-edited media can implant false memories; design for transparency.

## 5. Hardware Requirements
-   **CPU**: High core count (Intel i9 / AMD Ryzen 9) recommended for parallel processing.
-   **GPU**: NVIDIA GTX 1080Ti / Titan Xp or better for neural synthesis.
-   **RAM**: Minimum 16GB, 32GB+ recommended for 4K workflows.
