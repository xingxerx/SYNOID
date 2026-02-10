# SYNOID AI Video Production Kernel - Technical Design Document

## 1. Hardware Infrastructure & System Requirements
The SYNOID kernel is architected to handle the high-throughput demands of 4K non-linear editing and neural video synthesis. Hardware selection focuses on maximizing parallelization and minimizing I/O bottlenecks.

### 1.1 Processor (CPU) Selection & Logic
The GPU serves as the primary compute engine for OS overhead and complex calculations. Performance in 4K environments is dictated by core/thread density.
*   **High-End Specification (Intel i9 7900X):** Utilizes a 10-core/20-thread architecture to maximize parallelization during "Image Generation" and "Quality Optimization". Allows simultaneous rendering and background indexing without frame drops.
*   **Entry-Level Specification (AMD Ryzen 5):** Features 6 cores and 12 threads. Serves as the baseline for the kernel's **Degraded Performance Mode**.

### 1.2 Graphics (GPU) & VRAM
The GPU offloads transform-heavy tasks (encoding, scaling, real-time effects).
*   **VRAM Functionality:** Stores 3D assets and frame data. Determines the depth of simultaneous image layers cached before spilling to system memory.
*   **Hardware Tiers:** Supported from NVIDIA GTX 1050 (entry) to GTX 1080Ti/Titan Xp/RTX series. High-end cards are prioritized for professional-grade graphic synthesis.

### 1.3 Memory (RAM) & Storage Architecture
*   **Memory Thresholds:** Minimum 16GB RAM for stable 4K operation.
*   **Degraded Performance Mode:** If <16GB RAM is detected, the kernel must automatically trigger LSP optimizations (ra-multiplex enablement, disable cachePriming).
*   **Storage Throughput:**
    *   **OS/Software:** High-speed SSD mandatory.
    *   **Project Footage:** Mechanical HDDs must be 7200 rpm minimum. Storage capacity should be 3-4x source footage size.

## 2. Generative B-Roll & Context Engine (Veo & Gemini 3 Pro)
*Focused strictly on "funny snippets" and B-roll to enhance existing footage.*

### 2.1 Context-Aware Snippet Generation
Uses **Veo** to generate short, comedic or contextual snippets to visualize key points or inject humor.
*   **Purpose:** To "fit perfectly into the code" (the edit), not to replace the main footage.
*   **Duration Limit:** Snippets are capped at 3-5 seconds to maintain pacing.
*   **Wait-State Logic:** Asynchronous polling (15s interval) for `operation.done`.

### 2.2 Multi-Image Intelligent Processing
Uses **Gemini 3 Pro Image Preview** to analyze *existing* video frames to determine where a snippet is needed.
*   **Contextual Insertion:** Analyzes the user's footage to understand the "setup" before generating the "punchline" snippet.
*   **Response Modalities:** `responseModalities: ["IMAGE"]` for previewing generated assets.

### 2.3 System Performance SLA
*   **Snippet Generation:** 1-2 Minutes (Optimized for speed)

## 3. Intelligent Editing Engine: Logic & Flow
Replaces manual frame-level manipulation with high-level narrative abstractions.

### 3.1 Computational Narrative Logic
*   **Straight Cut:** Instant replacement for rhythmic flow.
*   **Temporal Compression (Jump Cut):** Removes redundant frames to increase velocity.
*   **Cutaway:** Contextual interruption for depth.
*   **Cross-Cutting:** Suspense-driven alternation.

### 3.2 Pacing & Rhythmic Patterns
*   **High-Frequency Cutting:** Short shots for action/kinetic energy.
*   **Introspective Pacing:** Longer shots for emotional weight.

### 3.3 Automated Segmentation & Composition
Uses Bayesian topic segmentation (**BSec**) and shot detection.
*   **BSec Synchronization:** Aligns text transcripts to video frames to generate initial edits from transcript segments.

## 4. Sonic Architecture & Signal Processing
Ensures professional-grade audio consistency.

### 4.1 Loudness & Mastering Standards
*   **Normalization:** Scale clips to -12dB target.
*   **Dialogue Compression:** 2:1 to 4:1 ratio.
*   **Mastering Headroom:** True Peak Limiter set to -1.0dBTP to -1.5dBTP.

### 4.2 Frequency Separation & EQ
*   **High-Pass Filtering:** 80-100Hz on dialogue/SFX.
*   **Subtractive EQ:** Carving frequency "slots" in music to avoid masking dialogue.

### 4.3 Acoustic Environment Simulation
*   **Ambience:** "Room Tone" and ambient beds mixed between -25dB and -40dB.

## 5. Developer Interface & Remote Production Workflow
Separates local driver from production container.

### 5.1 Remote File & LSP Management
Utilizes **Distant** tool protocol for editing on remote containers/servers from local Neovim.

### 5.2 Neovim Configuration (0.11+)
Matches `vim.lsp.config.rust_analyzer` settings:
*   `cmd = { 'rust-analyzer' }`
*   `initializationOptions` passing.

### 5.3 Task-Centered Visual Tools
Integrates "interactively-constructed visual program transformations" to scaffold complex Multimedia API calls.

## 6. System Optimization & Performance Tuning
### 6.1 WSL Performance Optimization
Disable environment variable sharing in `/etc/wsl.conf`: `appendWindowsPath = false`.

### 6.2 LSP Latency Mitigation (Low-End Hardware)
*   **ra-multiplex:** Enable server instance sharing.
*   **Cache Management:** `rust-analyzer.cachePriming.enable = false`.

### 6.3 Preprocessing & cost Control
*   **Pillow Library:** Downsample assets >5MB using `img.thumbnail`.
*   **Optimization:** `optimize=True` for WebP/JPEG conversion.

## 7. Cognitive Impact & System Security
### 7.1 Professional Automation Standards
Addresses needs for Personalization and Voice-based Interaction.

### 7.2 System Security
*   **Identity-Aware Proxy (IAP):** Control access to generation endpoints.
*   **Cloud Run:** Host generative application in isolated environment.

