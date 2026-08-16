import re

_PC = {
    'c': 0, 'd': 2, 'e': 4, 'f': 5,
    'g': 7, 'a': 9, 'b': 11,
}

_NOTE_RE = re.compile(r'^([a-gA-G])([#b]?)(\d*)$')


def note_to_freq(tok):
    """'c4', 'c#4', 'bb3', 'g' -> frequency in hz (default octave 4)."""
    m = _NOTE_RE.match(tok.strip())
    if not m:
        raise ValueError(f"'{tok}' is not a note (try 'c4', 'e', 'g#2', 'bb3')")
    letter, acc, octs = m.groups()
    pc = _PC[letter.lower()]
    if acc == '#':
        pc += 1
    elif acc == 'b':
        pc -= 1
    octave = int(octs) if octs else 4
    midi = 12 * (octave + 1) + pc
    return 440.0 * (2.0 ** ((midi - 69) / 12.0))


def _distribute(a, b):
    n = min(len(a), len(b))
    merged = [a[i] + b[i] for i in range(n)]
    if len(a) > len(b):
        return merged, a[n:]
    if len(b) > len(a):
        return merged, b[n:]
    return merged, []


def bjorklund(steps, hits):
    """euclidean rhythm: spread `hits` pulses evenly over `steps`.
    returns list of 0/1."""
    if hits <= 0:
        return [0] * steps
    if hits >= steps:
        return [1] * steps
    a = [[1] for _ in range(hits)]
    b = [[0] for _ in range(steps - hits)]
    while b:
        a, b = _distribute(a, b)
    flat = []
    for g in a:
        flat.extend(g)
    return flat


def euclid(steps, hits):
    """euclidean rhythm as beat tokens: ['x','.','x',...]."""
    return ['x' if v else '.' for v in bjorklund(steps, hits)]
