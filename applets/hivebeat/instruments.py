import numpy as np

SR = 48000

DRUMS = frozenset({'kick', 'snare', 'hat'})


def _env(t, dur, attack=0.004, tau=0.15):
    e = np.exp(-np.maximum(t - attack, 0.0) / max(dur * tau + 1e-4, 0.02))
    atk = np.minimum(1.0, t / max(attack, 1e-4))
    end = np.clip((dur - t) / 0.015, 0.0, 1.0)
    return atk * e * end


def square(freq, t, dur, amp, p):
    duty = float(p.get('duty', 0.5))
    ph = (freq * t) % 1.0
    y = np.where(ph < duty, 1.0, -1.0)
    y *= _env(t, dur, attack=p.get('attack', 0.003), tau=p.get('tau', 0.2))
    return y * amp * float(p.get('gain', 0.25))


def saw(freq, t, dur, amp, p):
    ph = (freq * t) % 1.0
    y = 2.0 * ph - 1.0
    det = float(p.get('detune', 0.006))
    if det:
        ph2 = ((freq * (1 + det)) * t) % 1.0
        ph3 = ((freq * (1 - det)) * t) % 1.0
        y = (y + (2 * ph2 - 1) + (2 * ph3 - 1)) / 3.0
    y *= _env(t, dur, attack=p.get('attack', 0.005), tau=p.get('tau', 0.2))
    return y * amp * float(p.get('gain', 0.2))


def fm(freq, t, dur, amp, p):
    ratio = float(p.get('ratio', 2.0))
    index = float(p.get('index', 4.0))
    mod = index * np.sin(2 * np.pi * freq * ratio * t)
    y = np.sin(2 * np.pi * freq * t + mod)
    y *= _env(t, dur, attack=p.get('attack', 0.01), tau=p.get('tau', 0.35))
    return y * amp * float(p.get('gain', 0.25))


def pad(freq, t, dur, amp, p):
    det = float(p.get('detune', 0.012))
    y = np.zeros_like(t)
    for d in (-det, 0.0, det):
        ph = ((freq * (1 + d)) * t) % 1.0
        y += 2 * ph - 1
    y /= 3.0
    y *= _env(t, dur, attack=p.get('attack', 0.08), tau=p.get('tau', 0.6))
    return y * amp * float(p.get('gain', 0.12))


def kick(freq, t, dur, amp, p):
    f0 = float(p.get('f0', 110.0))
    f1 = float(p.get('f1', 40.0))
    tau = float(p.get('tau', 0.08))
    f = f1 + (f0 - f1) * np.exp(-t / tau)
    phase = 2 * np.pi * (f1 * t + (f0 - f1) * tau * (1 - np.exp(-t / tau)))
    y = np.sin(phase)
    y *= np.exp(-t / max(dur * 0.4, 0.03))
    end = np.clip((dur - t) / 0.01, 0.0, 1.0)
    return y * amp * end * float(p.get('gain', 1.0))


def snare(freq, t, dur, amp, p):
    rng = np.random.default_rng()
    noise = rng.standard_normal(len(t))
    w = max(int(0.0004 * SR), 3)
    if len(t) > w:
        kern = np.ones(w) / w
        lp = np.convolve(noise, kern, mode='same')
        noise = noise - lp
    tone = np.sin(2 * np.pi * 185.0 * t)
    y = 0.7 * noise + 0.5 * tone
    y *= np.exp(-t / max(dur * 0.3, 0.03))
    end = np.clip((dur - t) / 0.012, 0.0, 1.0)
    return y * amp * end * float(p.get('gain', 0.9))


def hat(freq, t, dur, amp, p):
    rng = np.random.default_rng()
    noise = rng.standard_normal(len(t))
    w = max(int(0.0003 * SR), 3)
    if len(t) > w:
        kern = np.ones(w) / w
        lp = np.convolve(noise, kern, mode='same')
        noise = noise - lp
    y = noise * np.exp(-t / max(dur * 0.25, 0.02))
    end = np.clip((dur - t) / 0.008, 0.0, 1.0)
    return y * amp * end * float(p.get('gain', 0.4))


INSTRUMENTS = {
    'square': square,
    'saw': saw,
    'fm': fm,
    'pad': pad,
    'kick': kick,
    'snare': snare,
    'hat': hat,
}
