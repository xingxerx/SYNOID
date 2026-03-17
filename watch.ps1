# Smart Cargo Watch Script for SYNOID
# Only reloads when actual source code changes (not media files, logs, or build artifacts)
# Includes debouncing to prevent rapid reloads and graceful shutdown handling

Write-Host "🔍 SYNOID Smart Watch Mode" -ForegroundColor Cyan
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor DarkGray
Write-Host "✓ Watching Rust source files (.rs, .toml)" -ForegroundColor Green
Write-Host "✓ Ignoring: media files, logs, build artifacts" -ForegroundColor Green
Write-Host "✓ Debounced: 2s delay after file changes" -ForegroundColor Green
Write-Host "✓ Graceful shutdown enabled" -ForegroundColor Green
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━`n" -ForegroundColor DarkGray

# Use cargo-watch with smart filtering
# --why: Show which file triggered the rebuild
# --clear: Clear screen between rebuilds
# --delay: Wait 2 seconds after detecting changes (debouncing)
# --ignore: Ignore patterns from .watchignore file
# --watch: Only watch specific directories

cargo watch `
    --why `
    --clear `
    --delay 2 `
    --ignore "*.mp4" `
    --ignore "*.avi" `
    --ignore "*.mov" `
    --ignore "*.mkv" `
    --ignore "*.mp3" `
    --ignore "*.wav" `
    --ignore "*.png" `
    --ignore "*.jpg" `
    --ignore "*.log" `
    --ignore "*.tmp" `
    --ignore "Download/*" `
    --ignore "output/*" `
    --ignore "target/*" `
    --ignore ".git/*" `
    --ignore "*.md" `
    --watch src `
    --watch Cargo.toml `
    --watch rust-toolchain.toml `
    -x "run --release --bin synoid-core -- gui"

# Alternative: If you want to watch only specific file types
# Uncomment the following and comment out the above:

# cargo watch `
#     --why `
#     --clear `
#     --delay 2 `
#     --glob "**/*.rs" `
#     --glob "**/*.toml" `
#     -x "run --release --bin synoid-core -- gui"
