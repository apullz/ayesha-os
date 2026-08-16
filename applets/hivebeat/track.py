"""sabrepulse-style 8-bit chiptune demo — rendered with hivebeat's own engine.

usage:
    python3 track.py            # renders tracks/sabre_demo.wav (22050hz mono)
"""

import os

import numpy as np

from dsl import parse_line
from live import Engine

SR = 22050
BPM = 170
HERE = os.path.dirname(os.path.abspath(__file__))

# harmonic loop: | C | G | A | F |  (one bar each = 4 beats)
BASS = ("c2 . c2 . c2 . c2 . "     # C
        "g1 . g1 . g1 . g1 . "     # G
        "a1 . a1 . a1 . a1 . "     # A
        "f1 . f1 . f1 . f1 . ")    # F

ARP = ("c4 e4 g4 c5 e5 g5 c5 e4 "  # C
       "g4 b4 d5 g5 d5 b5 g4 b4 "  # G
       "a4 c5 e5 a5 e5 c5 a4 c5 "  # A
       "f4 a4 c5 f5 c5 a4 f4 a4 ")  # F

LEAD = ("c5 e5 g5 e5 a5 c6 a5 g5 "  # C
        "g5 b5 d6 b5 d6 b5 g5 b5 "  # G
        "a5 c6 e6 c6 e6 c6 a5 c6 "  # A
        "f5 a5 c6 a5 c6 a5 f5 a5 ")  # F

PULSE = ("c3 . c3 . c3 . c3 . "    # C
         "g2 . g2 . g2 . g2 . "    # G
         "a2 . a2 . a2 . a2 . "    # A
         "f2 . f2 . f2 . f2 . ")   # F

INTRO = [
    f'arp.square("{ARP}").dur(0.125).step(0.25).amp(0.5)',
    f'bass.saw("{BASS}").dur(0.25).step(0.5).amp(0.7)',
    'hat.hat(euclid(4,16)).dur(0.03).step(0.25).amp(0.5)',
]

BUILD = [
    f'arp.square("{ARP}").dur(0.125).step(0.25).amp(0.7)',
    f'bass.saw("{BASS}").dur(0.25).step(0.5).amp(0.8)',
    'kick.kick("x x x x").dur(0.3).step(1.0)',
    'snare.snare(". x . x").dur(0.2).step(1.0)',
    'hat.hat(euclid(8,16)).dur(0.03).step(0.25).amp(0.6)',
]

DROP = [
    f'bass.saw("{BASS}").dur(0.25).step(0.5).amp(1.0)',
    f'lead.square("{LEAD}").dur(0.06).step(0.25).amp(0.65)',
    f'lead2.saw("{PULSE}").dur(0.125).step(0.5).amp(0.4)',
    'kick.kick(euclid(12,16)).dur(0.08).step(0.25)',
    'snare.snare(". x . x").dur(0.18).step(1.0)',
    'hat.hat(euclid(10,16)).dur(0.04).step(0.25).amp(0.7)',
]

BREAKDOWN = [
    'pad.pad("[c4 e4 g4] [g3 b3 d4] [a3 c4 e4] [f3 a3 c4]").dur(3.8).step(4.0).amp(0.8)',
    f'arp.square("{ARP}").dur(0.125).step(0.25).amp(0.45)',
    'kick.kick("x . . .").dur(0.3).step(1.0).amp(0.9)',
    'snare.snare(euclid(6,16)).dur(0.1).step(0.25).amp(0.5)',
]

FINAL = [
    f'bass.saw("{BASS}").dur(0.25).step(0.5).amp(1.0)',
    f'lead.square("{LEAD}").dur(0.06).step(0.25).amp(0.75)',
    f'lead2.saw("{PULSE}").dur(0.125).step(0.5).amp(0.45)',
    f'arp.square("{ARP}").dur(0.125).step(0.25).amp(0.3)',
    'kick.kick(euclid(12,16)).dur(0.08).step(0.25)',
    'snare.snare(". x . x").dur(0.18).step(1.0)',
    'hat.hat(euclid(10,16)).dur(0.04).step(0.25).amp(0.75)',
]

OUTRO = [
    'pad.pad("[c4 e4 g4] [g3 b3 d4] [a3 c4 e4] [f3 a3 c4]").dur(3.8).step(4.0).amp(0.7)',
    f'arp.square("{ARP}").dur(0.125).step(0.25).amp(0.35)',
    f'bass.saw("{BASS}").dur(0.25).step(0.5).amp(0.5)',
]

SECTIONS = [
    ('intro', 8, INTRO),
    ('build', 8, BUILD),
    ('drop', 32, DROP),
    ('breakdown', 16, BREAKDOWN),
    ('final', 32, FINAL),
    ('outro', 16, OUTRO),
]

XFADE = 0.008


def render_section(lines, beats):
    eng = Engine(sr=SR)
    eng.set_bpm(BPM)
    for line in lines:
        pdef = parse_line(line)
        eng.set_player(pdef.name, pdef)
    frames = int(beats * 60.0 / BPM * SR)
    out = np.empty(frames, dtype=np.float32)
    done = 0
    while done < frames:
        n = min(4096, frames - done)
        out[done:done + n] = eng.tick(n)
        done += n
    return out


def assemble():
    parts = [render_section(lines, beats) for _, beats, lines in SECTIONS]
    xf = int(XFADE * SR)
    blobs = [parts[0]]
    for nxt in parts[1:]:
        cur = blobs[-1]
        n = min(xf, len(cur), len(nxt))
        tail = cur[-n:].copy()
        head = nxt[:n].copy()
        ramp = np.linspace(0.0, 1.0, n)
        tail *= (1 - ramp)
        head *= ramp
        cur = cur[:-n]
        mid = tail + head
        blobs[-1] = cur
        blobs.append(mid)
        blobs.append(nxt[n:])
    audio = np.concatenate(blobs)

    fade = int(3.0 * SR)
    if fade < len(audio):
        audio[-fade:] *= np.linspace(1.0, 0.0, fade)
    return np.clip(audio, -1.0, 1.0)


def write_wav(path, audio):
    data = (audio * 32767.0).astype(np.int16).tobytes()
    with open(path, 'wb') as f:
        f.write(b'RIFF')
        import struct
        f.write(struct.pack('<I', 36 + len(data)))
        f.write(b'WAVEfmt ')
        f.write(struct.pack('<IHHIIHH', 16, 1, 1, SR, SR * 2, 2, 16))
        f.write(b'data')
        f.write(struct.pack('<I', len(data)))
        f.write(data)


def main():
    audio = assemble()
    out = os.path.join(HERE, 'tracks', 'sabre_demo.wav')
    os.makedirs(os.path.dirname(out), exist_ok=True)
    write_wav(out, audio)
    print(f'  {len(audio) / SR:.1f}s of chipfire -> {out} '
          f'({os.path.getsize(out) / 1e6:.2f} MB)')


if __name__ == '__main__':
    main()
