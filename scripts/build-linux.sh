#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════════
#  ayesha-os :: build standalone linux / termux binary
#
#  unix mirror of scripts/build-exe.ps1:
#    - compiles the rust engine in release mode
#    - bundles dist/ = ayesha-os ELF + ayesha.json + models/ + applets/
#    - on termux additionally installs the hivebeat audio player
#
#  the engine is portable by design on unix: build.rs is a no-op
#  off-windows (no rc.exe / vcvars / msvc needed), reqwest uses
#  rustls-tls (no openssl), and there are no windows-only crates.
#
#  usage:  ./scripts/build-linux.sh
# ═══════════════════════════════════════════════════════════════
set -euo pipefail

# ── repo root (this script lives in scripts/) ──
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT/engine"

echo "╔═══════════════════════════════════════════════╗"
echo "║     AYESHA-OS :: BUILD LINUX/TERMUX BINARY    ║"
echo "╚═══════════════════════════════════════════════╝"

# ── [1/3] compile the rust engine (release mode) ──
echo "[1/3] compiling rust engine (release mode)..."
if ! cargo build --release; then
    echo "❌ rust build failed!" >&2
    exit 1
fi

# ── [2/3] copy binary + bundle config, models, applets into dist/ ──
echo "[2/3] copying binary and bundling assets..."
DIST="$ROOT/dist"
mkdir -p "$DIST"

# locate the freshly built elf. termux (host = aarch64-linux-android) and a
# plain linux box both land in target/release/, but a linux cross-build from
# another host lands in target/<triple>/release/ — cover both layouts.
BIN=""
for cand in \
    "$ROOT/engine/target/x86_64-unknown-linux-gnu/release/ayesha-os" \
    "$ROOT/engine/target/aarch64-linux-android/release/ayesha-os" \
    "$ROOT/engine/target/release/ayesha-os"; do
    if [ -f "$cand" ]; then BIN="$cand"; break; fi
done
if [ -z "$BIN" ]; then
    echo "❌ could not find the built binary under engine/target/" >&2
    exit 1
fi

cp "$BIN" "$DIST/ayesha-os"
# strip debug symbols (best-effort — some toolchains have no strip)
if command -v strip >/dev/null 2>&1; then
    strip "$DIST/ayesha-os" 2>/dev/null || true
fi

# config + models
if [ -f "$ROOT/ayesha.json" ]; then
    cp "$ROOT/ayesha.json" "$DIST/ayesha.json"
fi
if [ -d "$ROOT/models" ]; then
    mkdir -p "$DIST/models"
    cp "$ROOT/models/Modelfile" "$DIST/models/"
fi

# ── [3/3] package applets recursively ──
echo "[3/3] packaging applets directory recursively..."
if [ -d "$ROOT/applets" ]; then
    rm -rf "$DIST/applets"
    # node_modules is bundled on purpose so applets (e.g. flora-cli) run
    # instantly offline without a first-launch npm install (matches the ps1).
    if command -v rsync >/dev/null 2>&1; then
        rsync -a \
            --exclude '.git' \
            --exclude '__pycache__' \
            --exclude '.venv' \
            --exclude 'logs' \
            --exclude '.env' \
            --exclude '*.pyc' \
            "$ROOT/applets" "$DIST/"
    else
        # rsync-less fallback: copy then prune (same excludes as above)
        cp -a "$ROOT/applets" "$DIST/"
        find "$DIST/applets" -type d \( -name '.git' -o -name '__pycache__' \
            -o -name '.venv' -o -name 'logs' \) -prune -exec rm -rf {} + 2>/dev/null || true
        find "$DIST/applets" -type f \( -name '*.pyc' -o -name '.env' \) -delete 2>/dev/null || true
    fi
fi

# ── termux extras: hivebeat audio player into $PREFIX/bin ──
#    TERMUX_VERSION is set in every real termux shell (not in proot).
if [ -n "${TERMUX_VERSION:-}" ]; then
    echo ""
    echo "  termux detected (TERMUX_VERSION=${TERMUX_VERSION}) — installing hivepipe audio player..."
    if [ -f "$ROOT/applets/hivebeat/hivepipe" ]; then
        mkdir -p "${PREFIX:-/data/data/com.termux/files/usr}/bin"
        cp "$ROOT/applets/hivebeat/hivepipe" "${PREFIX:-/data/data/com.termux/files/usr}/bin/hivepipe"
        chmod +x "${PREFIX:-/data/data/com.termux/files/usr}/bin/hivepipe"
        echo "  ✔ hivepipe -> ${PREFIX:-/data/data/com.termux/files/usr}/bin/hivepipe"
    else
        echo "  ⚠ applets/hivebeat/hivepipe not found — skipping"
    fi
    echo ""
    echo "  audio not set up yet? run:  bash applets/hivebeat/setup_termux.sh"
    echo "    (pkg install python python-numpy pulseaudio; pulseaudio --start;"
    echo "     pactl load-module module-native-protocol-tcp auth-anonymous=1; sv-enable sshd)"
fi

echo ""
echo "✔ ayesha-os standalone build complete!"
echo "  binary: $DIST/ayesha-os"
echo "  to run: cd dist && ./ayesha-os"
