# SYNOID CUDA Agent Verification Script
# Quick verification that all CUDA agent components are properly set up

Write-Host "`n🚀 SYNOID CUDA Agent Verification" -ForegroundColor Cyan
Write-Host "==================================`n" -ForegroundColor Cyan

$allPassed = $true

# 1. Check CUDA Skills Files
Write-Host "📁 Checking CUDA Skill Files..." -ForegroundColor Yellow
$skillsDir = "src\agent\cuda\cuda_skills"
$requiredSkills = @(
    "color_grading.cu",
    "gaussian_blur.cu",
    "temporal_denoise.cu",
    "unsharp_mask.cu",
    "mod.rs",
    "cuda_kernel_demo.rs"
)

foreach ($skill in $requiredSkills) {
    $path = Join-Path $skillsDir $skill
    if (Test-Path $path) {
        Write-Host "   ✅ $skill" -ForegroundColor Green
    } else {
        Write-Host "   ❌ MISSING: $skill" -ForegroundColor Red
        $allPassed = $false
    }
}

# 2. Check CUDA Agent Modules
Write-Host "`n📦 Checking CUDA Agent Modules..." -ForegroundColor Yellow
$cudaModules = @(
    "src\agent\cuda\cuda_kernel_gen.rs",
    "src\agent\cuda\cuda_pipeline.rs",
    "src\agent\cuda\latent_optimizer.rs"
)

foreach ($module in $cudaModules) {
    if (Test-Path $module) {
        Write-Host "   ✅ $(Split-Path $module -Leaf)" -ForegroundColor Green
    } else {
        Write-Host "   ❌ MISSING: $module" -ForegroundColor Red
        $allPassed = $false
    }
}

# 3. Check SynoidLink Bridge
Write-Host "`n🔗 Checking SynoidLink Bridge..." -ForegroundColor Yellow
if (Test-Path "src\agent\specialized\synoid_link.rs") {
    Write-Host "   ✅ synoid_link.rs" -ForegroundColor Green
} else {
    Write-Host "   ❌ MISSING: synoid_link.rs" -ForegroundColor Red
    $allPassed = $false
}

# 4. Verify Module Exports
Write-Host "`n🔍 Verifying Module Exports..." -ForegroundColor Yellow
$modContent = Get-Content "src\agent\mod.rs" -Raw

if ($modContent -match "pub mod cuda") {
    Write-Host "   ✅ CUDA module exported in agent/mod.rs" -ForegroundColor Green
} else {
    Write-Host "   ❌ CUDA module NOT exported" -ForegroundColor Red
    $allPassed = $false
}

if ($modContent -match "synoid_link") {
    Write-Host "   ✅ SynoidLink exported in agent/mod.rs" -ForegroundColor Green
} else {
    Write-Host "   ❌ SynoidLink NOT exported" -ForegroundColor Red
    $allPassed = $false
}

# 5. Check Documentation
Write-Host "`n📚 Checking Documentation..." -ForegroundColor Yellow
if (Test-Path "docs\integration\CUDA_AGENT_INTEGRATION.md") {
    Write-Host "   ✅ CUDA_AGENT_INTEGRATION.md" -ForegroundColor Green
} else {
    Write-Host "   ⚠️  Documentation not found" -ForegroundColor Yellow
}

# 6. Build Test
Write-Host "`n🔨 Build Test..." -ForegroundColor Yellow
Write-Host "   Building SYNOID library..." -ForegroundColor Gray

try {
    $buildOutput = cargo build --lib 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-Host "   ✅ Library builds successfully" -ForegroundColor Green
    } else {
        Write-Host "   ❌ Build failed" -ForegroundColor Red
        Write-Host $buildOutput -ForegroundColor DarkRed
        $allPassed = $false
    }
} catch {
    Write-Host "   ❌ Build error: $_" -ForegroundColor Red
    $allPassed = $false
}

# 7. Check System Requirements
Write-Host "`n🖥️  System Requirements Check..." -ForegroundColor Yellow

# Check NVCC
try {
    $nvccVersion = nvcc --version 2>&1 | Select-String "release"
    if ($nvccVersion) {
        Write-Host "   ✅ NVCC (CUDA Compiler): $nvccVersion" -ForegroundColor Green
    }
} catch {
    Write-Host "   ⚠️  NVCC not found - CUDA compilation unavailable" -ForegroundColor Yellow
    Write-Host "      Install CUDA Toolkit for GPU acceleration" -ForegroundColor DarkGray
}

# Check FFmpeg
try {
    $ffmpegVersion = ffmpeg -version 2>&1 | Select-Object -First 1
    if ($ffmpegVersion) {
        Write-Host "   ✅ FFmpeg: $($ffmpegVersion.ToString().Substring(0, [Math]::Min(50, $ffmpegVersion.Length)))" -ForegroundColor Green
    }
} catch {
    Write-Host "   ❌ FFmpeg not found - Required for video processing" -ForegroundColor Red
}

# Check NVIDIA GPU
try {
    $nvidiaSmi = nvidia-smi 2>&1 | Select-Object -First 3
    if ($nvidiaSmi) {
        Write-Host "   ✅ NVIDIA GPU detected" -ForegroundColor Green
    }
} catch {
    Write-Host "   ⚠️  nvidia-smi not found - GPU acceleration unavailable" -ForegroundColor Yellow
}

# 8. Summary
Write-Host "`n" + ("=" * 50) -ForegroundColor Cyan
if ($allPassed) {
    Write-Host "✨ All CUDA Agent checks PASSED!" -ForegroundColor Green
    Write-Host "`n🎯 SYNOID CUDA integration is ready to use." -ForegroundColor Green
    Write-Host "`nNext steps:" -ForegroundColor Cyan
    Write-Host "   1. Run: cargo run --example cuda_agent_demo" -ForegroundColor White
    Write-Host "   2. Run: cargo test --test cuda_agent_test" -ForegroundColor White
    Write-Host "   3. See: docs\integration\CUDA_AGENT_INTEGRATION.md" -ForegroundColor White
} else {
    Write-Host "❌ Some checks FAILED - please review errors above" -ForegroundColor Red
    exit 1
}

Write-Host "`n" -NoNewline
