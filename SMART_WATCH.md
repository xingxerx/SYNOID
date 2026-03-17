# 🔍 SYNOID Smart Watch System

## Overview

The Smart Watch system provides intelligent, efficient hot-reloading for SYNOID development by only rebuilding when **actual source code changes**, not when temporary files, media, or logs are modified.

## Features

### ✅ Intelligent File Filtering
- **Watches**: Rust source files (`.rs`), `Cargo.toml`, `rust-toolchain.toml`
- **Ignores**: Media files (video/audio/images), logs, build artifacts, downloads, git files

### ✅ Debouncing (2-second delay)
Prevents rapid rebuilds when you save multiple files in quick succession. The watch waits 2 seconds after detecting changes before rebuilding.

### ✅ Graceful Shutdown
When the app needs to restart:
1. GUI closes cleanly and saves settings
2. Background tasks are notified
3. Active video editing jobs complete
4. Server and health monitors shut down
5. New build starts

### ✅ Clear Feedback
- Shows which file triggered the rebuild
- Clears the terminal for clean output
- Colored status messages

## Quick Start

### Windows (PowerShell)
```powershell
.\watch.ps1
```

### Linux/macOS/WSL
```bash
./watch.sh
```

## How It Works

### File Watching Strategy

The smart watch uses three layers of protection against unnecessary rebuilds:

1. **`.watchignore` file**: Defines patterns to ignore (similar to `.gitignore`)
2. **Script-level ignores**: Explicit `--ignore` flags in the watch scripts
3. **Directory targeting**: Only watches `src/`, `Cargo.toml`, and `rust-toolchain.toml`

### Debouncing

When you save a file, the watcher:
1. Detects the change
2. Waits 2 seconds for additional changes
3. Only then starts the rebuild

This is critical for IDEs that save multiple files when formatting or refactoring.

### Graceful Shutdown Flow

```
┌─────────────────────────┐
│  File Change Detected   │
└───────────┬─────────────┘
            │
            ▼
┌─────────────────────────┐
│   2-Second Debounce     │
└───────────┬─────────────┘
            │
            ▼
┌─────────────────────────┐
│  Send SIGTERM to App    │
└───────────┬─────────────┘
            │
            ▼
┌─────────────────────────┐
│ GUI on_exit() Handler   │
│ - Save settings         │
│ - Lock UI state         │
└───────────┬─────────────┘
            │
            ▼
┌─────────────────────────┐
│ Main.rs Cleanup         │
│ - Stop health monitor   │
│ - Wait for video jobs   │
│ - Exit process          │
└───────────┬─────────────┘
            │
            ▼
┌─────────────────────────┐
│  Cargo Rebuilds         │
└───────────┬─────────────┘
            │
            ▼
┌─────────────────────────┐
│  New Instance Starts    │
└─────────────────────────┘
```

## Configuration Files

### `.watchignore`
Defines patterns for files/directories to ignore:
```
# Media files
*.mp4
*.mp3
*.png

# Build artifacts
/target
/output

# Logs
*.log
```

### `.cargo/config.toml`
Cargo configuration for faster incremental builds:
```toml
[build]
incremental = true
```

## Customization

### Adjust Debounce Delay

Edit `watch.ps1` or `watch.sh` and change the `--delay` parameter:

```bash
--delay 5  # Wait 5 seconds instead of 2
```

### Watch Additional Directories

Add more `--watch` flags:

```bash
--watch src \
--watch Cargo.toml \
--watch custom_dir \
```

### Watch Specific File Types Only

Use `--glob` instead of `--watch`:

```bash
cargo watch \
    --why \
    --clear \
    --delay 2 \
    --glob "**/*.rs" \
    --glob "**/*.toml" \
    -x "run --release --bin synoid-core -- gui"
```

## Troubleshooting

### Problem: App keeps rebuilding constantly

**Cause**: Some process is modifying files in watched directories (like logs or temp files)

**Solution**:
1. Check what files are changing: `cargo watch --why` will show you
2. Add those patterns to `.watchignore`
3. Or use `--ignore "pattern"` in the watch script

### Problem: Changes not triggering rebuild

**Cause**: Your file might match an ignore pattern

**Solution**:
1. Check if the file is listed in `.watchignore`
2. Verify the file is in a watched directory (`src/` by default)
3. Try saving the file again (debounce might be active)

### Problem: Graceful shutdown not working

**Cause**: The app might have panicked or frozen

**Solution**:
1. Check for panic messages in the terminal
2. Use Ctrl+C to force-stop the watch process
3. Restart with `.\watch.ps1` or `./watch.sh`

### Problem: Too slow to rebuild

**Cause**: Release builds are optimized and take time

**Solution**: For faster iteration, use debug mode:

```bash
cargo watch -x "run --bin synoid-core -- gui"
# Note: Removed --release flag
```

## Performance Tips

1. **Use incremental compilation**: Already enabled in `.cargo/config.toml`
2. **Increase debounce delay**: If you're editing many files at once
3. **Watch fewer directories**: Only watch what you're actively changing
4. **Use debug builds during development**: Much faster than release builds

## Integration with IDEs

### VS Code
The watch scripts work great with VS Code's autosave feature. The debouncing ensures that rapid saves during typing don't trigger constant rebuilds.

Recommended `settings.json`:
```json
{
  "files.autoSave": "afterDelay",
  "files.autoSaveDelay": 1000
}
```

### Rust-Analyzer
Rust-analyzer and cargo-watch work together without conflicts. Rust-analyzer provides IDE features while cargo-watch handles rebuilding and running.

## Advanced: Conditional Watching

If you want different watch behaviors for different scenarios:

### Watch for Tests
```bash
cargo watch --clear --delay 2 --watch src -x test
```

### Watch for Specific Module
```bash
cargo watch --clear --delay 2 --watch src/agent/cuda -x "test --lib cuda"
```

### Watch with Compilation Check Only (No Run)
```bash
cargo watch --clear --delay 2 --watch src -x check
```

## See Also

- [cargo-watch documentation](https://github.com/watchexec/cargo-watch)
- [Cargo Incremental Compilation](https://doc.rust-lang.org/cargo/reference/profiles.html#incremental)
- [SYNOID README](README.md)
