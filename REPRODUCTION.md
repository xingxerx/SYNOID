# Reproducing SYNOID - Step-by-Step Guide

This guide details how to build and run the SYNOID Agentic Video Kernel.

## Prerequisites

1.  **Rust Toolchain**: Ensure you have the latest stable Rust installed.
    ```bash
    rustc --version
    cargo --version
    ```
2.  **FFmpeg**: Must be installed and available in your system PATH.
    ```bash
    ffmpeg -version
    ```
3.  **yt-dlp**: Required for YouTube downloading features.
    ```bash
    pip install yt-dlp
    ```
4.  **Hardware**:
    -   **RAM**: Minimum 16GB (32GB recommended for 4K).
    -   **GPU**: NVIDIA GPU recommended for neural features (CUDA support dependent on crate versions).

## Building the Project

1.  **Clone the Repository** (if not already done).
2.  **Build the Release Binary**:
    ```bash
    cargo build --release
    ```
    *Note: The first build will take some time to compile dependencies.*

## Running the Application

### 1. Launch the GUI
The primary interface is the "Command Center" GUI.
```bash
cargo run --release -- gui
```
-   **Features**: Upload videos, trim, compress, vectorize, upscale, and use AI features via the sidebar.

### 2. Command Line Interface (CLI) Examples

**YouTube Download:**
```bash
cargo run --release -- youtube --url "https://youtu.be/..." --intent "make it cinematic"
```

**Vectorize Video (Infinite Resolution):**
```bash
cargo run --release -- vectorize --input "my_video.mp4" --output "./vectors"
```

**Upscale Video (2x to 4K):**
```bash
cargo run --release -- upscale --input "input.mp4" --output "upscaled.mp4" --scale 2.0
```

**Voice Cloning:**
1.  **Record Sample:**
    ```bash
    cargo run --release -- voice --record 10 --output "sample.wav"
    ```
2.  **Create Profile:**
    ```bash
    cargo run --release -- voice --clone "sample.wav" --profile "MyVoice"
    ```
3.  **Speak (TTS):**
    ```bash
    cargo run --release -- voice --speak "Hello World" --profile "MyVoice"
    ```

**Cyberdefense Sentinel:**
Monitor the system for integrity violations while working.
```bash
cargo run --release -- guard --mode file --watch "./important_files"
```

## Troubleshooting

-   **Build Errors**: Ensure all system dependencies (like `pkg-config` on Linux/Mac, though this is Windows-focused) are met.
-   **FFmpeg Not Found**: Add the folder containing `ffmpeg.exe` to your User PATH environment variable.
-   **Memory Issues**: If upscaling 4K video, ensure you have sufficient RAM or reduce thread count by editing `rayon` configurations in `src/main.rs`.
