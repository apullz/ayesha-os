Set-Location "C:\ayesha-os\engine"
$env:RUSTFLAGS = "-Awarnings"
$vcvars = Join-Path ${env:ProgramFiles(x86)} "Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
cmd /c "`"$vcvars`" >nul 2>&1 && set RUSTFLAGS=-Awarnings && cargo run --release 2>nul"
