# Testing Smart Watch System

## Quick Test Checklist

### ✅ Test 1: Basic Watch Functionality

1. **Start the smart watch**:
   ```powershell
   .\watch.ps1
   ```

2. **Wait for initial build** to complete and GUI to launch

3. **Make a simple code change** in any `.rs` file (e.g., add a comment)

4. **Save the file**

5. **Observe**:
   - ✅ Terminal shows which file triggered the rebuild
   - ✅ GUI closes gracefully
   - ✅ 2-second delay before rebuild starts
   - ✅ New build compiles
   - ✅ GUI launches again

### ✅ Test 2: File Ignore Verification

1. **Start the smart watch**

2. **Create or modify a media file** in the project:
   ```powershell
   echo "test" > test.mp4
   ```

3. **Observe**:
   - ✅ No rebuild triggered
   - ✅ App continues running

4. **Clean up**:
   ```powershell
   del test.mp4
   ```

### ✅ Test 3: Debouncing

1. **Start the smart watch**

2. **Quickly save multiple files** in succession (within 2 seconds)

3. **Observe**:
   - ✅ Only ONE rebuild occurs after all saves
   - ✅ Rebuild starts ~2 seconds after the last save

### ✅ Test 4: Graceful Shutdown

1. **Start the smart watch**

2. **In the GUI**, start a video processing task if possible

3. **Make a code change and save**

4. **Observe**:
   - ✅ "Graceful shutdown initiated..." message
   - ✅ "Settings saved successfully" message
   - ✅ "Waiting for active video editing jobs..." (if jobs running)
   - ✅ Clean exit before rebuild

### ✅ Test 5: Watch Specific Directories Only

1. **Verify watched directories**:
   ```bash
   # The watch script only watches:
   # - src/
   # - Cargo.toml
   # - rust-toolchain.toml
   ```

2. **Modify a file outside watched dirs** (e.g., `README.md`)

3. **Observe**:
   - ✅ No rebuild triggered

4. **Modify a file inside `src/`**

5. **Observe**:
   - ✅ Rebuild triggered

## Manual Test: Run Smart Watch

### Windows:
```powershell
# Full smart watch with all features
.\watch.ps1

# Expected output:
# 🔍 SYNOID Smart Watch Mode
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# ✓ Watching Rust source files (.rs, .toml)
# ✓ Ignoring: media files, logs, build artifacts
# ✓ Debounced: 2s delay after file changes
# ✓ Graceful shutdown enabled
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

### Linux/WSL:
```bash
# Full smart watch with all features
./watch.sh
```

## Automated Test Script

Here's a PowerShell script to test the ignore patterns:

```powershell
# test-watch.ps1
Write-Host "Testing Smart Watch Ignore Patterns..." -ForegroundColor Cyan

# Create test files that should be ignored
$testFiles = @(
    "test_video.mp4",
    "test_audio.mp3",
    "test_image.png",
    "test.log",
    "Download\test.txt",
    "output\test.txt"
)

foreach ($file in $testFiles) {
    # Create directory if needed
    $dir = Split-Path $file
    if ($dir -and !(Test-Path $dir)) {
        New-Item -ItemType Directory -Path $dir -Force | Out-Null
    }

    # Create test file
    "test" | Out-File $file
    Write-Host "Created: $file" -ForegroundColor Yellow
}

Write-Host "`nThese files should NOT trigger a rebuild." -ForegroundColor Green
Write-Host "Press Enter to clean up test files..." -ForegroundColor Gray
Read-Host

# Cleanup
foreach ($file in $testFiles) {
    if (Test-Path $file) {
        Remove-Item $file -Force
        Write-Host "Deleted: $file" -ForegroundColor Gray
    }
}

Write-Host "`nTest complete!" -ForegroundColor Green
```

## Expected Behavior

### When editing `.rs` files:
```
[Running 'cargo run --release --bin synoid-core -- gui']
[Finished running. Exit status: 0]
[Changed: src/window.rs]
[Running 'cargo run --release --bin synoid-core -- gui']
```

### When editing `.mp4` or `.log` files:
```
[Running 'cargo run --release --bin synoid-core -- gui']
(No rebuild, app keeps running)
```

## Performance Benchmarks

Expected rebuild times (on a modern system):

- **Incremental rebuild** (small change): 3-10 seconds
- **Full rebuild** (Cargo.toml change): 30-60 seconds
- **Debug mode rebuild**: 2-5 seconds

## Troubleshooting Test Failures

### Test 1 Failed (No rebuild on code change)
- Check if file is in `src/` directory
- Verify cargo-watch is installed: `cargo install cargo-watch`
- Try running manually: `cargo watch --why -x "run --bin synoid-core -- gui"`

### Test 2 Failed (Rebuilds on media files)
- Check `.watchignore` exists and contains media patterns
- Verify ignore flags in `watch.ps1`/`watch.sh`
- Try adding more specific patterns

### Test 3 Failed (Multiple rebuilds)
- Check `--delay 2` is set in the watch script
- Increase delay to 3-5 seconds for slower systems
- Verify no other file watchers are running (like IDE auto-formatters)

### Test 4 Failed (Not graceful)
- Check `window.rs:2565-2584` for `on_exit` handler
- Check `main.rs:356-366` for cleanup logic
- Look for panic messages in terminal output

## Success Criteria

All tests pass when:
- ✅ Code changes trigger rebuilds
- ✅ Media/log changes do NOT trigger rebuilds
- ✅ Debouncing prevents rapid rebuilds
- ✅ Graceful shutdown messages appear
- ✅ Settings are saved on exit
- ✅ Video jobs complete before restart

## Next Steps

After testing:
1. Report any issues at: https://github.com/anthropics/synoid/issues
2. Customize watch patterns for your workflow
3. Integrate with your IDE's auto-save settings
4. See [SMART_WATCH.md](SMART_WATCH.md) for advanced configuration
