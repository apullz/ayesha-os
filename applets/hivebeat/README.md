# hivebeat (◕‿◕✿)

a live-coding music synth that runs in a terminal. you type a pattern, it loops forever, you edit it while it plays, and the changes land on the next cycle — no glitches. square/saw/fm/pad melodies, euclidean drum grooves, chiptune to ambient — all from a tiny language of our own.

```
 _     _           _                _
| |__ (_)_   _____| |__   ___  __ _| |_
| '_ \| \ \ / / _ \ '_ \ / _ \/ _` | __|
| | | | |\ V /  __/ |_) |  __/ (_| | |_
|_| |_|_| \_/ \___|_.__/ \___|\__,_|\__|
              ...the hive is humming
```

- **live**: run it in a real termux shell and hear it through your phone.
- **offline**: render to wav anywhere — even inside proot — and play it with any music app.

---

## demo — hear it (◕‿◕)

[![play](https://img.shields.io/badge/▶_play_embedded_player-hivebeat--demo-3dff88?style=for-the-badge)](https://ayesha-os.github.io/hivebeat/)

a full 8-bit chiptune, **rendered by hivebeat's own engine** — 170 bpm, euclidean drums, arp leads, build → drop → breakdown → final drop:

- ▶ [**embedded player** (with section-highlighting timeline)](https://ayesha-os.github.io/hivebeat/)
- [🎵 sabre_demo.wav](tracks/sabre_demo.wav) — 39s, 22050hz mono, 1.7 MB
- [source: track.py](track.py) — the whole track is just hivebeat DSL lines

```
python3 track.py   # regenerate tracks/sabre_demo.wav in one shot
```

---

## quickstart

### live

```
bash setup_termux.sh        # pkg install python python-numpy pulseaudio; starts the daemon + TCP bridge
./hivebeat                  # the live repl — start typing patterns
```

live audio works from real termux *and* from inside proot. the repl streams PCM to a bundled pulse-simple player (`hivepipe`) over **ssh into the real termux side** — proot↔pulse sockets are cursed (SCM_CREDENTIALS never survive the proot boundary, and termux's TCP module never greets), but a natively-running client works every time. `setup_termux.sh` installs `hivepipe` into termux and makes sure sshd is up; the runit `pulseaudio` service keeps the daemon alive.

> **if it's silent** (backend falls back to `null sink`): check the daemon and sshd on the termux side:
> ```
> sv status sshd
> sv restart $PREFIX/var/service/pulseaudio   # or: sv start $PREFIX/var/service/pulseaudio
> pkill -9 -x pulseaudio && sv start $PREFIX/var/service/pulseaudio   # nuke stray daemons
> ```
> the repl tries ssh → TCP bridge → unix socket in order, so a healthy daemon + sshd is all it needs.

### offline (works anywhere with python + numpy)

```
python3 render.py out.wav 8 'p1.square("c4 e4 g4 a4").dur(0.25)'
./play.sh out.wav 8 'drums.kick(euclid(3,8)).dur(0.25)' 'p1.saw("c2 g2").dur(1)'
```

---

## tutorial — the hive language

every player is a **looping pattern**. the syntax is a method chain — `name` first, then the instrument, then any params:

```
name.instrument("token token token").param(value).param(value)
```

(and the older `name >> instrument("...", param=value)` form still works too)

### 1. your first notes

```
p1.square("c4 e4 g4 a4").dur(0.25)
```

- `p1` is just a name — any identifier works (`lead`, `bass`, `x`, ...).
- `square` is the instrument.
- the string is the **pattern**: space-separated tokens, each token = one note.
- `dur` = how long each note lasts, in beats. `dur(0.25)` = sixteenth notes.
- the pattern loops forever. re-type `p1.square(...)` with anything and it swaps in on the next cycle.

### 2. notes, octaves, accidentals, rests

```
p1.square("c4 e4 g4 a4 g4 e4 c4").dur(0.5)   # note + octave
p2.saw("c#4 d#4 f#4 g#4").dur(0.5)           # sharps and flats: # or b
p3.saw("e2 a2 d3 g3").dur(1)                 # octaves move the pitch register
p1.square("c4 . e4 rest g4 _").dur(0.5)      # rests: .  rest  r  _
```

- bare `e` (no octave) defaults to octave 4.
- rests are `.`, `rest`, `r`, or `_` — they eat their `dur` slot but play nothing.

### 3. timing: `dur`, `step`, `delay`, `amp`

```
p1.square("c4 e4 g4").dur(1)                 # each note 1 beat (legato)
p1.square("c4 e4 g4").dur(0.5).step(1)       # note is half a beat, then 0.5 beat of silence
bass.saw("c2 g2").dur(1).delay(1)            # shift the whole pattern by 1 beat (call & response!)
hat.hat("x x x x").dur(0.25).amp(0.6)        # quieter
```

- `dur` — note length in beats (can also be a list, see below).
- `step` — time *between* note starts. default: 1 beat melodic, 0.5 beat drums.
- `delay` — how many beats in before the first note (offset the cycle start).
- `amp` — overall volume of this player (1.0 = full).

### 4. cycling lists — make patterns *move*

```
p1.square("c4 e4 g4 a4").dur([0.25, 0.25, 0.5])   # durations cycle: 0.25 0.25 0.5 0.25 0.25 0.5 ...
bass.saw("c2 g2 a1 e2").dur([1, 0.5]).amp([0.9, 0.5])   # lists cycle independently
```

any of `dur`, `step`, `amp` can be a list — it repeats round-robin forever. this is the cheapest way to make a pattern breathe.

### 5. chords

```
p1.fm("[c4 e4 g4] [d4 f4 a4] [a3 c4 e4] [g3 b3 d4]").dur(0.5)
```

brackets `[ ]` group notes into one chord event — all played at once. a chord is one token and eats one `dur` slot.

### 6. drums

```
drums.kick("x . x . x x . .").dur(0.5)     # x = hit, everything else = rest
snare.snare(".. x .. x").dur(0.5)
hat.hat("x x x x x x x x").dur(0.25)
```

drum instruments are `kick`, `snare`, `hat`. the pattern is `x` (hit) or `.` (rest). the instrument name is the *sound*; the player name is whatever you want (yes, `drums.kick(...)` is the normal shape).

### 7. euclidean rhythms — instant grooves

```
drums.kick(euclid(3, 8)).dur(0.25)   # 3 hits spread as evenly as possible over 8 steps
hat.hat(euclid(5, 8)).dur(0.125)
snare.snare(euclid(3, 4)).dur(0.5)
```

`euclid(hits, steps)` returns a rhythm — bjorklund's algorithm. swap the numbers and the groove breathes differently. classics:

| hits/steps | vibe |
|---|---|
| `euclid(3, 8)` | the ubiquitous tech-kick |
| `euclid(5, 8)` | swung hats |
| `euclid(3, 4)` | snare-on-the-3 |
| `euclid(7, 16)` | off-kilter percussion |
| `euclid(11, 16)` | almost-in-4/4 chaos |

### 8. every instrument & its knobs

| instrument | sound | extra params (defaults) |
|---|---|---|
| `square` | chiptune pulse | `duty=0.5`, `detune`, `attack=0.003`, `tau=0.2`, `gain=0.25` |
| `saw` | detuned saw stack | `detune=0.006`, `attack=0.005`, `tau=0.2`, `gain=0.2` |
| `fm` | FM bell/pluck | `ratio=2.0`, `index=4.0`, `attack=0.01`, `tau=0.35`, `gain=0.25` |
| `pad` | slow detuned strings | `detune=0.012`, `attack=0.08`, `tau=0.6`, `gain=0.12` |
| `kick` | pitch-drop thump | `f0=110`, `f1=40`, `tau=0.08`, `gain=1.0` |
| `snare` | noise + 185hz ring | `gain=0.9` |
| `hat` | highpassed noise | `gain=0.4` |

```
p1.fm("c4 e4 g4").ratio(3).index(2).tau(0.8)    # softer, wobblier bell
p1.square("c4 e4 g4").duty(0.25)                 # thin, bright pulse
p1.pad("c3 [e3 g3]").dur(4).attack(0.3)          # slow evolving wash
```

### 9. live controls

```
bpm(128)              # change tempo — the clock resyncs, players keep looping
stop                  # (or hush) — silence everything, keep the repl alive
?                     # help
exit / quit / ctrl-c  # leave
```

re-typing any `name.instrument(...)` replaces that player on the next cycle boundary — the key live-coding move. build up one layer at a time, then morph them:

```
drums.kick(euclid(3,8)).dur(0.25)
hat.hat(euclid(5,8)).dur(0.125)
bass.saw("c2 g2 a1 e2").dur(0.5)
bpm(140)
bass.saw("c2 g2 c3 a1").dur([0.5,0.5,1])
lead.fm("[c4 e4 g4] [d4 f4 a4]").dur(0.5)
stop
```

---

## command reference (the whole language)

```
name.instrument("pattern").param(value).param(value)   define/replace a looping player
  instrument:  square | saw | fm | pad | kick | snare | hat
  pattern:     notes (c4, e, g#3, bb2) · rests (. rest r _) · chords ([c4 e4 g4])
               drums use x (hit) / . (rest) · euclid(hits, steps) for drum patterns
  params:      dur (number|list) · step (number|list) · delay (number) · amp (number|list)
               + per-instrument knobs from the table above
               (legacy form:  name >> instrument("pattern", param=value)  also works)

bpm(number)      set tempo
stop | hush      silence all players
? | help         print help
exit | quit      leave (ctrl-c too)
```

## render.py & play.sh (offline)

```
python3 render.py out.wav SECONDS "player line" ["player line" ...]
./play.sh out.wav SECONDS "player line" [...]     # render, then try to auto-open it on android
```

---

## architecture

| module | job |
|---|---|
| `audio.py` | sinks — `PacatSink` (live → `hivepipe` on real termux over ssh, with TCP/unix fallbacks), `WavSink` (offline), `NullSink` (realtime silent clock) |
| `live.py` | `Engine` — sample-accurate cycle scheduler + tanh limiter. players swap at cycle boundaries |
| `dsl.py` | the parser — patterns, chords, euclid, cycling params → `PlayerDef` |
| `instruments.py` | stateless numpy synths (envelope + oscillator, nothing stateful → glitch-free by construction) |
| `pattern.py` | note→freq math + bjorklund euclidean rhythm generator |

the clock is the sorensen & gardner "programming with time" model: events are scheduled at absolute sample positions, so timing is sample-accurate and editing mid-loop never corrupts the beat.

## why no supercollider / tidal / sonic-pi?

they all need the scsynth server, which doesn't run on termux/android. so hivebeat is its own engine — pure python + numpy, portable to anything with a sound path.

## known limits (v0.1)

- mono synthesis (stereo only at the sink)
- no custom envelopes yet, no MIDI, no recording of live sessions
- live audio works from proot via ssh-pipe: the repl streams PCM to `hivepipe` running natively on the termux side (proot↔pulse sockets die on SCM_CREDENTIALS, so nothing beats a real termux client)

## development

```
python3 beep.py     # smoke test 1: renders a 1s beep, verifies it's non-silent
python3 drift.py    # smoke test 2: realtime clock drift over 10s (expect <50ms)
```

---

## ayesha-os integration

vendored into the ayesha-os monorepo as `applets/hivebeat/` (plain copy, no git history). registered in `ayesha.json` and launched by the engine's `manage_applet` runner with `python repl.py` (foreground — it takes over the terminal like a tui).

platform notes:

- the repl guards its `import readline` (not shipped with cpython on windows), so it boots everywhere — history is best-effort, live line editing just degrades gracefully where readline is missing.
- the repl needs numpy: `pip install -r requirements.txt` (numpy>=1.21) before first run. the vendored copy skips `__pycache__`/`.venv` in the dist bundle, so install into the applet dir or a venv.
- live audio expects the termux pulseaudio path (`hivepipe` / `pacat`); on platforms without it the repl degrades to the realtime `null` sink (synthesis runs, nothing audible).
- windows has no pulseaudio — use offline rendering, which works anywhere with python + numpy:
- termux (real termux shell, not proot): `bash setup_termux.sh` installs python + numpy + pulseaudio, starts the daemon and installs the `hivepipe` player into `$PREFIX/bin`.

offline render:

```
python render.py out.wav 8 'p1.square("c4 e4 g4 a4").dur(0.25)'
```
