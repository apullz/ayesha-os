<#
.SYNOPSIS
    Builds ayesha-os monorepo into a single standalone distribution package with ayesha.exe
#>

param(
    [switch]$Release = $true
)

Write-Host "╔═══════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║     AYESHA-OS :: BUILD STANDALONE EXECUTABLE  ║" -ForegroundColor Cyan
Write-Host "╚═══════════════════════════════════════════════╝" -ForegroundColor Cyan

$Root = Split-Path -Parent $PSScriptRoot
Set-Location "$Root\engine"

Write-Host "[1/3] Compiling rust engine (release mode)..." -ForegroundColor Yellow
if ($Release) {
    cargo build --release
} else {
    cargo build
}

if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ Rust build failed!" -ForegroundColor Red
    exit 1
}

$OutputDir = "$Root\dist"
if (!(Test-Path $OutputDir)) {
    New-Item -ItemType Directory -Path $OutputDir | Out-Null
}

$BinPath = if ($Release) {
    if (Test-Path "$Root\engine\target\x86_64-pc-windows-msvc\release\ayesha-os.exe") {
        "$Root\engine\target\x86_64-pc-windows-msvc\release\ayesha-os.exe"
    } else {
        "$Root\engine\target\release\ayesha-os.exe"
    }
} else {
    "$Root\engine\target\debug\ayesha-os.exe"
}
$DestBin = "$OutputDir\ayesha-os.exe"

Write-Host "[2/3] Copying executable and bundling assets..." -ForegroundColor Yellow
Get-Process -Name "ayesha-os", "ayesha-engine" -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 500
Copy-Item -LiteralPath $BinPath -Destination $DestBin -Force

# Copy config and models
if (Test-Path "$Root\ayesha.json") {
    Copy-Item -LiteralPath "$Root\ayesha.json" -Destination "$OutputDir\ayesha.json" -Force
}
if (Test-Path "$Root\models") {
    if (!(Test-Path "$OutputDir\models")) { New-Item -ItemType Directory -Path "$OutputDir\models" | Out-Null }
    Copy-Item -LiteralPath "$Root\models\Modelfile" -Destination "$OutputDir\models\Modelfile" -Force
}

Write-Host "[3/3] Packaging applets directory recursively..." -ForegroundColor Yellow
$AppletsDir = "$Root\applets"
if (Test-Path $AppletsDir) {
    $DestAppletsDir = "$OutputDir\applets"
    if (Test-Path $DestAppletsDir) {
        Remove-Item -LiteralPath $DestAppletsDir -Recurse -Force
    }
    Copy-Item -LiteralPath $AppletsDir -Destination $DestAppletsDir -Recurse -Force
}

Write-Host ""
Write-Host "✔ ayesha-os standalone build complete!" -ForegroundColor Green
Write-Host "  Executable location: $DestBin" -ForegroundColor Cyan
Write-Host "  To run: cd dist && .\ayesha-os.exe" -ForegroundColor Yellow
