#!/usr/bin/env python3
"""
Start the autonomous sleeper plugin in background.
while you sleep, ayesha-os self-improves for 8h.
each 30-min cycle: ayesha runs her own agent loop to decide + apply
one improvement to the codebase (source/skill/applet/config),
then cargo test -> cargo build -> --selftest -> commit + force-push to github.
to stop: pkill -f autonomous_sleeper
"""
import subprocess, sys, os
from pathlib import Path

SCRIPT = Path(__file__).parent / "autonomous_sleeper.py"

def main():
    print("starting autonomous sleeper (8h self-improvement mode)...", flush=True)
    print("ayesha will decide + apply improvements, build, test, and sync to github.", flush=True)
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
