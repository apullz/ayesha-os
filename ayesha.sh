#!/usr/bin/env bash
# ayesha.sh — run the built binary. the exe/ELF IS the app (delivery rule).
cd "$(dirname "$0")"
exec ./dist/ayesha-os "$@"
