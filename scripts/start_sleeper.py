#!/usr/bin/env python3
"""
Start the autonomous sleeper plugin in background.
user runs this before sleep; it commits + pushes every 30 min for 8h.
"""
import subprocess, sys, os
from pathlib import Path

SCRIPT = Path(__file__).parent / "autonomous_sleeper.py"

def main():
    print("starting autonomous sleeper (8h background mode)...", flush=True)
    print("it will sweep for ollama refs + push commits every 30 min.", flush=True)
    print("to stop: pkill -f autonomous_sleeper", flush=True)
    r = subprocess.run(
        [sys.executable, str(SCRIPT)],
        cwd=str(Path(__file__).parent.parent),
        stdout=sys.stdout,
        stderr=sys.stderr,
    )
    sys.exit(r.returncode)

if __name__ == "__main__":
    main()
