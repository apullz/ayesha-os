#!/usr/bin/env pwsh
# sync-all.ps1 — push monorepo + HF model repo + HF Space in one shot
# Usage: .\scripts\sync-all.ps1
# Requires: $env:HF_TOKEN for HuggingFace access

$ROOT = Split-Path -Parent (Split-Path -Parent $PSCommandPath)
$HF_MODEL = "$ROOT\_hf-model"
$HF_SPACE = "$ROOT\_hf-space"
$HF_TOKEN = $env:HF_TOKEN

function Clone-HFRepo($url, $dest) {
    if (Test-Path "$dest\.git") { return $true }
    if (Test-Path $dest) { Remove-Item -Recurse -Force $dest }
    Write-Host ">>> Cloning $dest..." -ForegroundColor Cyan
    if ($HF_TOKEN) {
        $authedUrl = $url -replace "https://", "https://hf:$HF_TOKEN@"
        git clone $authedUrl $dest 2>&1
    } else {
        git clone $url $dest 2>&1
    }
    return (Test-Path "$dest\.git")
}

# ── 1. Clone HF repos ──
$modelOk = Clone-HFRepo "https://huggingface.co/ayesha-hivemind/ayesha" $HF_MODEL
$spaceOk = Clone-HFRepo "https://huggingface.co/spaces/ayesha-hivemind/ayesha-hivemind" $HF_SPACE

# ── 2. Sync HF model repo ──
if ($modelOk) {
    Write-Host "`n>>> Syncing HF model repo..." -ForegroundColor Yellow
    Push-Location $HF_MODEL
    Copy-Item "$ROOT\models\Modelfile" "$HF_MODEL\" -Force
    Copy-Item "$ROOT\ayesha.json" "$HF_MODEL\" -Force
    Copy-Item "$ROOT\scripts\space-app.py" "$HF_MODEL\app.py" -Force
    git add -A
    git diff --cached --quiet
    if ($LASTEXITCODE -ne 0) {
        git commit -m "sync $(Get-Date -Format 'yyyy-MM-dd HH:mm')"
        git push
        if ($LASTEXITCODE -eq 0) {
            Write-Host "  ✅ HF model pushed" -ForegroundColor Green
        } else {
            Write-Host "  ❌ HF model push failed" -ForegroundColor Red
        }
    } else {
        Write-Host "  nothing to commit" -ForegroundColor DarkGray
    }
    Pop-Location
} else {
    Write-Host "`n>>> Skipping HF model (no token or clone failed)" -ForegroundColor DarkYellow
}

# ── 3. Sync HF Space repo ──
if ($spaceOk) {
    Write-Host "`n>>> Syncing HF Space repo..." -ForegroundColor Yellow
    Push-Location $HF_SPACE
    Copy-Item "$ROOT\core\app.py" "$HF_SPACE\" -Force
    Copy-Item "$ROOT\core\ayesha_hive_client.py" "$HF_SPACE\" -Force
    Copy-Item "$ROOT\core\ayesha_mobile_api.py" "$HF_SPACE\" -Force
    Copy-Item "$ROOT\models\Modelfile" "$HF_SPACE\" -Force
    Copy-Item "$ROOT\scripts\space-app.py" "$HF_SPACE\app.py" -Force
    git add -A
    git diff --cached --quiet
    if ($LASTEXITCODE -ne 0) {
        git commit -m "sync $(Get-Date -Format 'yyyy-MM-dd HH:mm')"
        git push
        if ($LASTEXITCODE -eq 0) {
            Write-Host "  ✅ HF Space pushed" -ForegroundColor Green
        } else {
            Write-Host "  ❌ HF Space push failed" -ForegroundColor Red
        }
    } else {
        Write-Host "  nothing to commit" -ForegroundColor DarkGray
    }
    Pop-Location
} else {
    Write-Host "`n>>> Skipping HF Space (no token or clone failed)" -ForegroundColor DarkYellow
}

# ── 4. Push GitHub monorepo ──
Write-Host "`n>>> Pushing GitHub monorepo..." -ForegroundColor Yellow
Push-Location $ROOT
git add -A
git diff --cached --quiet
if ($LASTEXITCODE -ne 0) {
    git commit -m "sync $(Get-Date -Format 'yyyy-MM-dd HH:mm')"
    git push origin master
    if ($LASTEXITCODE -eq 0) {
        Write-Host "  ✅ GitHub pushed" -ForegroundColor Green
    } else {
        Write-Host "  ❌ GitHub push failed" -ForegroundColor Red
    }
} else {
    Write-Host "  nothing to commit" -ForegroundColor DarkGray
}
Pop-Location

Write-Host "`n=== All synced! ===" -ForegroundColor Green
