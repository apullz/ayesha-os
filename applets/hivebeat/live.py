import numpy as np


class Engine:
    """the hive: sample-accurate cycle scheduler + mixer."""

    def __init__(self, sr=48000):
        self.sr = sr
        self.bpm = 120.0
        self.players = {}
        self.sample_pos = 0
        self._beat_s = 60.0 / self.bpm

    @property
    def beat_samples(self):
        return self._beat_s * self.sr

    def set_bpm(self, bpm):
        self.bpm = float(bpm)
        self._beat_s = 60.0 / self.bpm
        for pd in list(self.players.values()):
            pd.precompute(self._beat_s, self.sr)

    def set_player(self, name, pdef):
        pdef.precompute(self._beat_s, self.sr)
        self.players[name] = pdef

    def hush(self):
        self.players.clear()

    def tick(self, frames):
        """render `frames` samples starting at current sample_pos,
        return float32 mono buffer."""
        start = self.sample_pos
        end = start + frames
        out = np.zeros(frames, dtype=np.float64)
        sr = self.sr
        for pd in list(self.players.values()):
            if not pd.cycle_events or pd.cycle_samples <= 0:
                continue
            cs = pd.cycle_samples
            c0 = start // cs
            c1 = (end - 1) // cs
            for c in range(c0, c1 + 1):
                base = c * cs
                for (se, de, ev) in pd.cycle_events:
                    s = base + se
                    if s >= end or s + de <= start:
                        continue
                    lo = max(start, s)
                    hi = min(end, s + de)
                    if lo >= hi:
                        continue
                    seg = hi - lo
                    t = np.arange(seg, dtype=np.float64) / sr + (lo - s) / sr
                    durs = de / sr
                    freq, is_rest = ev
                    if is_rest:
                        continue
                    if freq is None:
                        y = pd.instrument(None, t, durs, pd.amp, pd.params)
                    elif isinstance(freq, list):
                        y = np.zeros(seg)
                        n = max(len(freq), 1)
                        for f in freq:
                            y += pd.instrument(f, t, durs, pd.amp / n, pd.params)
                    else:
                        y = pd.instrument(freq, t, durs, pd.amp, pd.params)
                    out[lo - start:hi - start] += y
        self.sample_pos = end
        out = np.tanh(out) * 0.85
        return out.astype(np.float32)
