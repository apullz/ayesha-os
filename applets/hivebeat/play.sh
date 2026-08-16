#!/usr/bin/env bash
# render some music to wav and try to play it on this android box.
# usage: ./play.sh out.wav 6 'p1.square("c4 e4 g4").dur(0.25)' 'drums.kick(euclid(3,8)).dur(0.25)'
set -e
DIR="$(cd "$(dirname "$0")" && pwd)"
OUT="$1"; SECS="$2"; shift 2
python3 "$DIR/render.py" "$OUT" "$SECS" "$@"
echo "  trying to open it on android..."
am start -a android.intent.action.VIEW -d "file://$OUT" -t "audio/*" >/dev/null 2>&1 \
  && echo "  opened with a music player (if nothing happened, play $OUT manually)" \
  || echo "  couldn't auto-open — play $OUT with any music app"
