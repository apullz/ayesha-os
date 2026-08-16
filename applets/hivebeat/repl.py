import os
import re
try:
    import readline  # not shipped on windows python — history is best-effort
except ImportError:
    readline = None

from audio import make_live_sink, NullSink, WavSink
from dsl import parse_line
from live import Engine

HIST_FILE = os.path.join(os.path.expanduser('~'), '.hivebeat_history')


def setup_history():
    if readline is None:
        return
    try:
        readline.read_history_file(HIST_FILE)
    except (OSError, IOError):
        pass
    readline.set_history_length(200)


def save_history():
    if readline is None:
        return
    try:
        readline.write_history_file(HIST_FILE)
    except (OSError, IOError):
        pass

BANNER = r"""
 _     _           _                _
| |__ (_)_   _____| |__   ___  __ _| |_
| '_ \| \ \ / / _ \ '_ \ / _ \/ _` | __|
| | | | |\ V /  __/ |_) |  __/ (_| | |_
|_| |_|_| \_/ \___|_.__/ \___|\__,_|\__|
              ...the hive is humming
"""

HELP = """
  hivebeat — a live-coding music synth (v0.1)

  players (method-chain syntax; the old p1 >> square(...) also still works):
    p1.square("c4 e4 g4 a4").dur(0.25)     melodic: notes + rests (., rest)
    p1.saw("c2 g2 [c3 e3 g3]").dur(1)      chords: [c3 e3 g3]
    kick.kick("x . x . x x . .").dur(0.5)  drums: x = hit, . = rest
    hat.hat(euclid(3, 8)).dur(0.25)        euclidean rhythm: 3 hits / 8 steps

  chain any params:  .dur() .step() .delay() .amp() + per-instrument
    (fm: ratio, index · saw/square: detune, duty, tau · kick: f0, f1)

  instruments: square  saw  fm  pad  kick  snare  hat

  controls:
    bpm(128)   change tempo (clock resyncs)
    p1.xxx...  reassign a player live (swaps at the next cycle)
    stop/hush  silence everything
    ?          this help
    exit/quit  leave (ctrl-c works too)
"""


def main():
    engine = Engine()
    try:
        sink = make_live_sink(engine)
        sink.start()
        mode = f'live ({getattr(sink, "label", "hivepipe")})'
    except Exception as e:
        sink = NullSink(engine)
        sink.start()
        mode = f'null sink ({e}) — run in a real termux shell for sound, or use render.py'
    setup_history()
    print(BANNER)
    print(f"  audio backend: {mode}")
    print('  try:  p1.square("c4 e4 g4 a4").dur(0.25)')
    print("  (stop/exit to leave · ? for help)")
    try:
        while True:
            try:
                line = input('> ').strip()
            except EOFError:
                break
            if not line:
                continue
            low = line.lower()
            if low in ('?', 'help'):
                print(HELP)
                continue
            if low in ('stop', 'hush'):
                engine.hush()
                print('  shhh... hive went quiet (◕‿◕)')
                continue
            if low in ('exit', 'quit'):
                engine.hush()
                break
            m = re.match(r'^bpm\s*\(\s*(\d+(?:\.\d+)?)\s*\)$', low)
            if m:
                b = float(m.group(1))
                engine.set_bpm(b)
                print(f'  bpm -> {b:g} · clock resynced (＾▽＾)')
                continue
            try:
                pdef = parse_line(line)
            except ValueError as e:
                print(f"  (｡•́︿•̀｡) {e}")
                continue
            engine.set_player(pdef.name, pdef)
            if readline is not None:
                readline.add_history(line)
            print(f'  {pdef.name} -> {pdef.describe()} (hive humming...)')
            continue
            print("  (｡•́︿•̀｡) huh? try  p1.square(\"c4 e4 g4\").dur(0.25)   or   ?")
    except KeyboardInterrupt:
        pass
    finally:
        sink.stop()
        save_history()
    print('\n  bye bye, hive is asleep (￣ω￣)')


if __name__ == '__main__':
    main()
