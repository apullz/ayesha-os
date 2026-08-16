"""smoke test 1: render a 1s beep to wav and verify it's non-silent.

usage: python3 beep.py [out.wav]
"""

import os
import sys

import numpy as np

from audio import WavSink
from dsl import parse_player
from live import Engine


def main():
    out = sys.argv[1] if len(sys.argv) > 1 else '/tmp/hivebeat_beep.wav'
    engine = Engine()
    pdef = parse_player('square("c4", dur=1.0)', 'beep')
    engine.set_player('beep', pdef)
    secs = WavSink(engine, out, 1.0).run()
    import wave
    with wave.open(out, 'rb') as w:
        frames = np.frombuffer(w.readframes(w.getnframes()), dtype=np.int16)
    peak = float(np.max(np.abs(frames))) / 32767.0
    ok = peak > 0.05 and abs(secs - 1.0) < 0.01
    print(f'  rendered {secs:.3f}s -> {out}')
    print(f'  peak amplitude: {peak:.3f}')
    print(f'  size: {os.path.getsize(out)} bytes')
    print('  RESULT: ' + ('PASS (beep is audible)' if ok else 'FAIL'))
    return 0 if ok else 1


if __name__ == '__main__':
    sys.exit(main())
