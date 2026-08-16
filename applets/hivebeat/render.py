"""offline render: hivebeat lines -> wav.

usage:
    python3 render.py out.wav 6 'p1.square("c4 e4 g4 a4").dur(0.25)'
    python3 render.py out.wav 6 'drums.kick(euclid(3,8)).dur(0.25)' 'p1.saw("c3 e3 g3").dur(0.5)'
"""

import sys

from audio import WavSink
from dsl import parse_line
from live import Engine


def main(argv):
    if len(argv) < 3:
        print(__doc__)
        return 1
    out_path, seconds, lines = argv[0], float(argv[1]), argv[2:]
    engine = Engine()
    for line in lines:
        line = line.strip()
        if not line:
            continue
        try:
            pdef = parse_line(line)
        except ValueError as e:
            print(f'  (｡•́︿•̀｡) {e}')
            return 1
        engine.set_player(pdef.name, pdef)
        print(f'  {pdef.name} -> {pdef.describe()}')
    seconds_rendered = WavSink(engine, out_path, seconds).run()
    print(f'  rendered {seconds_rendered:.2f}s -> {out_path}')
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
