#!/usr/bin/env python3
"""
Ayesha Core Setup Helper
Verifies and installs dependencies
"""

import subprocess
import sys
import platform

def run_command(cmd):
    """Run shell command"""
    try:
        result = subprocess.run(cmd, shell=True, capture_output=True, text=True)
        return result.returncode == 0, result.stdout, result.stderr
    except Exception as e:
        return False, "", str(e)

def main():
    print("═" * 60)
    print("  Ayesha Core - Setup & Build Helper")
    print("═" * 60)
    print()
    
    print("Current System:", platform.system(), platform.release())
    print()
    
    # Check Python version
    print("[1/4] Checking Python version...")
    python_version = f"{sys.version_info.major}.{sys.version_info.minor}"
    print(f"  ✓ Python {python_version}")
    
    # Install requirements
    print("[2/4] Installing Kivy dependencies...")
    success, out, err = run_command("pip install -q -r requirements.txt")
    if success:
        print("  ✓ Dependencies installed")
    else:
        print("  ✗ Failed to install dependencies")
        print(err)
        sys.exit(1)
    
    # Test Kivy
    print("[3/4] Testing Kivy import...")
    try:
        import kivy
        print(f"  ✓ Kivy {kivy.__version__} available")
    except ImportError:
        print("  ✗ Kivy not found")
        sys.exit(1)
    
    print("[4/4] Build options:")
    print()
    print("  Option 1: Run desktop app locally")
    print("    python mobile_app.py")
    print()
    print("  Option 2: Build APK via GitHub Actions (Recommended)")
    print("    - Push this repo to GitHub")
    print("    - Check Actions tab for build progress")
    print("    - Download APK from artifacts")
    print()
    print("  Option 3: Build APK locally with buildozer (advanced)")
    print("    - Requires WSL2 Ubuntu or Linux")
    print("    - buildozer android debug")
    print()
    print("═" * 60)
    print("  Setup complete! Choose your build method above. >w<")
    print("═" * 60)

if __name__ == "__main__":
    main()
