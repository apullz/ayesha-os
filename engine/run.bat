@echo off
echo ========================================
echo   ayesha-engine: terminal persona host
echo   model: ayesha (ollama)
echo ========================================
cd /d "%~dp0"
set PATH=C:\msys64\mingw64\bin;%USERPROFILE%\.cargo\bin;%PATH%
target\release\ayesha-engine.exe
pause
