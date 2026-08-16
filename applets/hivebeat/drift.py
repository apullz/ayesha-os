"""smoke test 2: clock drift check over 10s (realtime pacing, no audio needed).

usage: python3 drift.py
"""

import sys
import time

from audio import NullSink
from dsl import parse_player
from live import Engine


def main():
    engine = Engine()
    engine.set_player('k', parse_player('kick(euclid(3, 8), dur=0.25)', 'k'))
    engine.set_player('p', parse_player('saw("c4 e4 g4 a4", dur=0.25)', 'p'))
    sink = NullSink(engine)
    t0 = time.monotonic()
    sink.start()
    time.sleep(10.0)
    sink.stop()
    wall = time.monotonic() - t0
    expected = engine.sr * wall
    actual = engine.sample_pos
    drift_ms = (actual - expected) / engine.sr * 1000.0
    print(f'  wall time : {wall:.3f}s')
    print(f'  samples   : {actual} (expected ~{int(expected)})')
    print(f'  drift     : {drift_ms:+.2f} ms')
    ok = abs(drift_ms) < 50.0
    print('  RESULT: ' + ('PASS (clock holds realtime)' if ok else 'FAIL'))
    return 0 if ok else 1


if __name__ == '__main__':
    sys.exit(main())
