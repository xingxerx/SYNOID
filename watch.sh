#!/bin/bash
# Smart Cargo Watch Script for SYNOID
# Only reloads when actual source code changes (not media files, logs, or build artifacts)
# Includes debouncing to prevent rapid reloads and graceful shutdown handling

echo -e "\033[36m🔍 SYNOID Smart Watch Mode\033[0m"
echo -e "\033[90m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\033[0m"
echo -e "\033[32m✓ Watching Rust source files (.rs, .toml)\033[0m"
echo -e "\033[32m✓ Ignoring: media files, logs, build artifacts\033[0m"
echo -e "\033[32m✓ Debounced: 2s delay after file changes\033[0m"
echo -e "\033[32m✓ Graceful shutdown enabled\033[0m"
echo -e "\033[90m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\033[0m\n"

# Use cargo-watch with smart filtering
# --why: Show which file triggered the rebuild
# --clear: Clear screen between rebuilds
# --delay: Wait 2 seconds after detecting changes (debouncing)
# --ignore: Ignore patterns from .watchignore file
# --watch: Only watch specific directories

cargo watch \
    --why \
    --clear \
    --delay 2 \
    --ignore "*.mp4" \
    --ignore "*.avi" \
    --ignore "*.mov" \
    --ignore "*.mkv" \
    --ignore "*.mp3" \
    --ignore "*.wav" \
    --ignore "*.png" \
    --ignore "*.jpg" \
    --ignore "*.log" \
    --ignore "*.tmp" \
    --ignore "Download/*" \
    --ignore "output/*" \
    --ignore "target/*" \
    --ignore ".git/*" \
    --ignore "*.md" \
    --watch src \
    --watch Cargo.toml \
    --watch rust-toolchain.toml \
    -x "run --release --bin synoid-core -- gui"

# Alternative: If you want to watch only specific file types
# Uncomment the following and comment out the above:

# cargo watch \
#     --why \
#     --clear \
#     --delay 2 \
#     --glob "**/*.rs" \
#     --glob "**/*.toml" \
#     -x "run --release --bin synoid-core -- gui"
