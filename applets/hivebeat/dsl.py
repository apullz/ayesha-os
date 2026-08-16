import re

import instruments as I
from pattern import euclid, note_to_freq


class PlayerDef:
    """a looping pattern: instrument + token events + timing."""

    def __init__(self, name, instr_name, instrument, tokens, dur, step, delay,
                 amp, params, is_drum):
        self.name = name
        self.instr_name = instr_name
        self.instrument = instrument
        self.tokens = tokens
        self.dur = dur if isinstance(dur, list) else [dur]
        self.step = step if isinstance(step, list) else [step]
        self.delay = delay
        self.amp = amp
        self.params = params
        self.is_drum = is_drum
        self.cycle_samples = 0
        self.cycle_events = []

    def precompute(self, beat_s, sr):
        beat_samples = beat_s * sr
        cycle_beats = 0.0
        events = []
        start_b = self.delay
        for i, ev in enumerate(self.tokens):
            step_i = float(self.step[i % len(self.step)])
            dur_i = float(self.dur[i % len(self.dur)])
            cycle_beats += step_i
            events.append((start_b, dur_i, ev))
            start_b += step_i
        self.cycle_samples = int(round(cycle_beats * beat_samples))
        self.cycle_events = [
            (int(round(sb * beat_samples)), int(round(db * beat_samples)), ev)
            for (sb, db, ev) in events
        ]

    def describe(self):
        return f"{self.instr_name} · {len(self.tokens)} steps"


_ARG_RE = re.compile(r'([a-z0-9_]+)\s*=\s*(\[[^\]]*\]|[^,]+)')
_INSTR_RE = re.compile(r'^\s*([a-z0-9_]+)\s*\((.*)\)\s*$', re.S)
_NAME_RE = re.compile(r'^[A-Za-z_][A-Za-z0-9_]*$')


def _convert(val):
    val = val.strip()
    if val.startswith('['):
        items = [x.strip() for x in val[1:-1].split(',')]
        return [float(x) for x in items if x]
    if val.startswith('euclid'):
        m = re.match(r'euclid\(\s*(\d+)\s*,\s*(\d+)\s*\)', val)
        if m:
            hits, steps = int(m.group(1)), int(m.group(2))
            return euclid(steps, hits)
    try:
        return float(val) if ('.' in val or 'e' in val.lower()) else int(val)
    except ValueError:
        return val


def _split_calls(s):
    """split a method chain 'instr(a).p1(b).p2(c)' into [('instr','a'),('p1','b')...],
    keeping balanced parens (so euclid(3,8) inside the first call stays intact)."""
    calls = []
    while True:
        s = s.strip()
        if not s:
            break
        m = re.match(r'([a-z0-9_]+)\s*\(', s)
        if not m:
            raise ValueError(f"expected name(...) — got '{s[:24]}'")
        name = m.group(1)
        depth = 0
        j = m.end() - 1
        n = len(s)
        while j < n:
            ch = s[j]
            if ch == '(':
                depth += 1
            elif ch == ')':
                depth -= 1
                if depth == 0:
                    break
            j += 1
        if depth != 0:
            raise ValueError('unbalanced parentheses in ' + s[:24])
        calls.append((name, s[m.end():j]))
        rest = s[j + 1:].strip()
        if not rest:
            break
        if not rest.startswith('.'):
            raise ValueError(f"expected .param(value) after ')', got '{rest[:16]}'")
        s = rest[1:]
    return calls


def _drum_tokens(words):
    return [(None, w not in ('x', 'X')) for w in words]


def _tokenize_melodic(s):
    toks = []
    cur = []
    in_bracket = False
    for w in s.split():
        if w.startswith('['):
            in_bracket = True
        if in_bracket:
            cur.append(w)
            if w.endswith(']'):
                toks.append(' '.join(cur))
                cur = []
                in_bracket = False
        else:
            toks.append(w)
    if cur:
        toks.append(' '.join(cur))
    return toks


def _parse_melodic_words(words):
    out = []
    for w in _tokenize_melodic(' '.join(words)):
        w = w.strip()
        if not w:
            continue
        if w in ('.', 'rest', 'r', '_'):
            out.append((None, True))
        elif w.startswith('[') and w.endswith(']'):
            freqs = [note_to_freq(x) for x in w[1:-1].split()]
            out.append((freqs, False))
        else:
            out.append((note_to_freq(w), False))
    return out


def _build_player(name, instr_name, argstr, kwargs):
    is_drum = instr_name in I.DRUMS
    instrument = I.INSTRUMENTS.get(instr_name)
    if instrument is None:
        have = ', '.join(sorted(I.INSTRUMENTS))
        raise ValueError(f"unknown instrument '{instr_name}' — have: {have}")

    tokens = None
    kw_start = 0
    mq = re.search(r'"([^"]*)"', argstr)
    if mq:
        words = mq.group(1).split()
        tokens = _drum_tokens(words) if is_drum else _parse_melodic_words(words)
        kw_start = mq.end()
    else:
        me = re.search(r'^\s*euclid\(\s*(\d+)\s*,\s*(\d+)\s*\)', argstr)
        if me:
            hits, steps = int(me.group(1)), int(me.group(2))
            tokens = _drum_tokens(euclid(steps, hits))
            kw_start = me.end()
        elif not is_drum and ',' not in argstr:
            words = argstr.split()
            if words:
                tokens = _parse_melodic_words(words)
                kw_start = len(argstr)
    if tokens is None:
        raise ValueError('i need a pattern string: square("c4 e4 g4", dur=0.25)')

    for mm in _ARG_RE.finditer(argstr[kw_start:]):
        kwargs[mm.group(1)] = _convert(mm.group(2))

    step = kwargs.get('step', 0.5 if is_drum else 1.0)
    dur = kwargs.get('dur', 0.3 if is_drum else step)
    delay = kwargs.get('delay', 0.0)
    amp = kwargs.get('amp', 1.0)
    params = {k: v for k, v in kwargs.items() if k not in ('dur', 'step', 'delay', 'amp')}

    return PlayerDef(name, instr_name, instrument, tokens, dur, step, delay,
                     amp, params, is_drum)


def parse_player(rhs, name):
    if not _NAME_RE.match(name):
        raise ValueError(f"'{name}' isn't a valid player name (use p1, bass, kick...)")
    m = _INSTR_RE.match(rhs)
    if not m:
        raise ValueError("expected: instrument(\"c4 e4 g4\", dur=0.25) — got no matching parens")
    instr_name = m.group(1).lower()
    return _build_player(name, instr_name, m.group(2), {})


def parse_chain(rhs, name):
    """method-chain form:  p1.square("c4 e4 g4").dur(0.25).amp(0.8)"""
    if not _NAME_RE.match(name):
        raise ValueError(f"'{name}' isn't a valid player name (use p1, bass, kick...)")
    calls = _split_calls(rhs)
    if not calls:
        raise ValueError('expected  p1.square("c4 e4 g4").dur(0.25)')
    instr_name = calls[0][0].lower()
    kwargs = {}
    for pn, pv in calls[1:]:
        kwargs[pn] = _convert(pv)
    return _build_player(name, instr_name, calls[0][1], kwargs)


def parse_line(line):
    """accept either syntax:
        p1.square("c4 e4 g4").dur(0.25)          (method chain)
        p1 >> square("c4 e4 g4", dur=0.25)       (legacy foxdot form)
    """
    line = line.strip()
    m = re.match(r'^([A-Za-z_][A-Za-z0-9_]*)\s*(>>|\.)', line)
    if not m:
        raise ValueError('expected  p1.square("c4 e4 g4").dur(0.25)   or   p1 >> square("c4 e4 g4", dur=0.25)')
    name = m.group(1)
    rest = line[m.end():]
    if m.group(2) == '>>':
        return parse_player(rest, name)
    return parse_chain(rest, name)
