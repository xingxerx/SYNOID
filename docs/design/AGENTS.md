# Synoid Agent Configuration

## Project Overview
Agentic Video Production Kernel in Rust. High-performance video editing via
natural language intent. Key differentiator: vector-based infinite upscaling
+ built-in cyberdefense.

## Architecture Patterns
- **Actor Model**: Use tokio::sync::mpsc for cross-module communication
- **Engine Pattern**: Each engine (video/vector/voice) implements Plugin trait
- **Safety First**: All FFI calls wrapped in unsafe blocks with documentation

## Code Standards
- **Error Handling**: Use thiserror for enums, anyhow for binaries
- **Async**: tokio runtime, avoid blocking calls in async functions
- **FFmpeg**: Always validate input before spawning processes

## Testing Strategy
- Unit tests: `cargo test` in each crate
- Integration: `cargo test --features integration`
- FFmpeg tests require `ffmpeg` in PATH (marked with #[ignore])

## Common Tasks for Agents
- Refactoring: Prefer moving code to new crates over large files (>500 lines)
- Features: Always add CLI flag + GUI menu item + config file support
- Security: Any file operations must go through sentinel module
