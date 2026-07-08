#!/usr/bin/env pwsh
# sync-all.ps1 — push monorepo + HF model repo + HF Space in one shot
# Usage: .\scripts\sync-all.ps1

$ROOT = Split-Path -Parent (Split-Path -Parent $PSCommandPath)
$HF_MODEL = "$ROOT\_hf-model"    # git clone of hf.co/apullz/ayesha
$HF_SPACE = "$ROOT\_hf-space"    # git clone of hf.co/spaces/apullz/ayesha-hivemind

# ── 1. Init / clone HF repos if not present ──
if (-not (Test-Path "$HF_MODEL\.git")) {
    Write-Host ">>> Cloning HF model repo..." -ForegroundColor Cyan
    git clone https://huggingface.co/apullz/ayesha $HF_MODEL
}
if (-not (Test-Path "$HF_SPACE\.git")) {
    Write-Host ">>> Cloning HF space repo..." -ForegroundColor Cyan
    git clone https://huggingface.co/spaces/apullz/ayesha-hivemind $HF_SPACE
}

# ── 2. Sync HF model repo ──
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
    Write-Host "  ✅ HF model pushed" -ForegroundColor Green
} else {
    Write-Host "  nothing to commit" -ForegroundColor DarkGray
}
Pop-Location

# ── 3. Sync HF Space repo ──
Write-Host "`n>>> Syncing HF Space repo..." -ForegroundColor Yellow
Push-Location $HF_SPACE
Copy-Item "$ROOT\core\app.py" "$HF_SPACE\" -Force
Copy-Item "$ROOT\core\ayesha_hive_client.py" "$HF_SPACE\" -Force
Copy-Item "$ROOT\core\ayesha_mobile_api.py" "$HF_SPACE\" -Force
Copy-Item "$ROOT\core\ayesha_sync.py" "$HF_SPACE\" -Force
Copy-Item "$ROOT\core\tri_node_mind.py" "$HF_SPACE\" -Force
Copy-Item "$ROOT\models\Modelfile" "$HF_SPACE\" -Force
Copy-Item "$ROOT\scripts\space-app.py" "$HF_SPACE\app.py" -Force
git add -A
git diff --cached --quiet
if ($LASTEXITCODE -ne 0) {
    git commit -m "sync $(Get-Date -Format 'yyyy-MM-dd HH:mm')"
    git push
    Write-Host "  ✅ HF Space pushed" -ForegroundColor Green
} else {
    Write-Host "  nothing to commit" -ForegroundColor DarkGray
}
Pop-Location

# ── 4. Push GitHub monorepo ──
Write-Host "`n>>> Pushing GitHub monorepo..." -ForegroundColor Yellow
Push-Location $ROOT
git add -A
git diff --cached --quiet
if ($LASTEXITCODE -ne 0) {
    git commit -m "sync $(Get-Date -Format 'yyyy-MM-dd HH:mm')"
    git push origin master
    Write-Host "  ✅ GitHub pushed" -ForegroundColor Green
} else {
    Write-Host "  nothing to commit" -ForegroundColor DarkGray
}
Pop-Location

Write-Host "`n=== All synced! ===" -ForegroundColor Green
