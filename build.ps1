#!/usr/bin/env pwsh
# Cross-platform build script for the time command

param(
    [string]$Target = "",
    [switch]$Release = $false,
    [switch]$All = $false,
    [switch]$Help = $false
)

if ($Help) {
    Write-Host "Build script for cross-platform time command"
    Write-Host ""
    Write-Host "Usage: ./build.ps1 [options]"
    Write-Host ""
    Write-Host "Options:"
    Write-Host "  -Target <target>    Specific target to build for"
    Write-Host "  -Release           Build in release mode"
    Write-Host "  -All               Build for all common targets"
    Write-Host "  -Help              Show this help message"
    Write-Host ""
    Write-Host "Common targets:"
    Write-Host "  x86_64-pc-windows-msvc     (Windows 64-bit)"
    Write-Host "  x86_64-unknown-linux-gnu   (Linux 64-bit)"
    Write-Host "  x86_64-apple-darwin        (macOS 64-bit)"
    Write-Host "  aarch64-apple-darwin       (macOS ARM64)"
    Write-Host "  aarch64-pc-windows-msvc    (Windows ARM64)"
    exit 0
}

$BuildFlags = @()
if ($Release) {
    $BuildFlags += "--release"
    $BuildDir = "release"
} else {
    $BuildDir = "debug"
}

$CommonTargets = @(
    "x86_64-pc-windows-msvc",
    "x86_64-unknown-linux-gnu", 
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
    "aarch64-pc-windows-msvc"
)

function Build-Target {
    param([string]$TargetName)
    
    Write-Host "Building for target: $TargetName" -ForegroundColor Green
    
    # Add target if not already installed
    rustup target add $TargetName
    
    # Build
    cargo build --target $TargetName @BuildFlags
    
    $ExeExtension = if ($TargetName -like "*windows*") { ".exe" } else { "" }
    $BinaryPath = "target/$TargetName/$BuildDir/time$ExeExtension"
    
    if (Test-Path $BinaryPath) {
        Write-Host "✓ Successfully built: $BinaryPath" -ForegroundColor Green
        
        # Copy to a more convenient location
        $OutputDir = "binaries/$TargetName"
        New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
        Copy-Item $BinaryPath "$OutputDir/time$ExeExtension"
        Write-Host "✓ Copied to: $OutputDir/time$ExeExtension" -ForegroundColor Cyan
    } else {
        Write-Host "✗ Build failed for $TargetName" -ForegroundColor Red
    }
}

if ($All) {
    Write-Host "Building for all common targets..." -ForegroundColor Yellow
    foreach ($target in $CommonTargets) {
        Build-Target $target
        Write-Host ""
    }
} elseif ($Target) {
    Build-Target $Target
} else {
    # Build for current platform
    Write-Host "Building for current platform..." -ForegroundColor Yellow
    cargo build @BuildFlags
    
    $ExeExtension = if ($IsWindows) { ".exe" } else { "" }
    $BinaryPath = "target/$BuildDir/time$ExeExtension"
    
    if (Test-Path $BinaryPath) {
        Write-Host "✓ Successfully built: $BinaryPath" -ForegroundColor Green
    }
}

Write-Host ""
Write-Host "Build complete! 🎉" -ForegroundColor Green
