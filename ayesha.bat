@echo off
cd /d "C:\ayesha-os\engine"
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul 2>&1
set RUSTFLAGS=-Awarnings
rustup run stable-x86_64-pc-windows-msvc cargo run --release
