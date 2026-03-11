# SYNOID Isolated Instance Launcher v1.1
# Ensures Absolute Isolation for Autonomous Learning loops.

param(
    [int]$port = 3001
)

Write-Host "🚀 Starting Isolated SYNOID Instance on Port $port..." -ForegroundColor Cyan
Write-Host "📁 Build Directory: target_$port" -ForegroundColor Gray
Write-Host "🔧 Cargo Home: $PSScriptRoot\target_$port\.cargo" -ForegroundColor Gray

# Set environment for Absolute Isolation
$env:CARGO_TARGET_DIR = "target_$port"
$env:CARGO_HOME = "$PSScriptRoot\target_$port\.cargo"
$env:SYNOID_INSTANCE_ID = "$port"
$env:SYNOID_ENABLE_SENTINEL = "true"

# Launch in GUI mode with watch protection
# We ignore build artifacts, caches, and AI internal state files to prevent infinite restart loops
cargo watch -i "target*" -i "Download*" -i "cortex_cache*" -i "synoid_settings*" -i "synoid_intent*" -i "brain_memory.json" -i "editing_strategy.json" -x "run --release -- gui --port $port"
