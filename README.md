# ayesha-os

a distributed, self-improving ai ecosystem powered by local ollama models. ayesha-os is an agentic coding assistant and a jarvis-like chatbot, all wrapped in the personality of ayesha — an otaku genki ai. **v4.5.0**: rust engine with themes, sessions, skills & streaming syntax highlighting; an expo/react-native mobile chat app; a dockerized huggingface bot space; and a CI automation harness.

```
                       _     
                      | |    
  __ _ _   _  ___  ___| |__   __ _ ______ ___  ___ 
 / _` | | | |/ _ \/ __| '_ \ / _` |______/ _ \/ __|
| (_| | |_| |  __/\__ \ | | | (_| |     | (_) \__ \
 \__,_|\__, |\___||___/_| |_|\__,_|      \___/|___/
        __/ |                                       
       |___/                                        
```

## architecture

```
┌──────────────┐     ┌─────────────────┐     ┌──────────────────┐
│  core        │◄───►│  engine (rust)  │◄───►│  ollama / cloud  │
│  (web ui,    │     │  (cli agent,    │     │  (local models   │
│   mobile api)│     │   tool-calling) │     │   + openrouter)  │
└──────┬───────┘     └────────┬────────┘     └──────────────────┘
       │                      │
       │              ┌───────▼───────────────┐
       │              │  tri_mind_sync +      │
       │              │  automation harness   │
       │              │  (sync engine + CI)   │
       │              └───────┬───────────────┘
       │                      │
       │              ┌───────▼─────────┐      ┌──────────────────────┐
       │              │  applets/       │      │  ayesha-bot-mobile   │
       │              │  └─ flora-cli   │      │  (expo / react       │
       │              └─────────────────┘      │   native chat client)│
                                               └─────────┬────────────┘
       │                                                │
       │                    ┌───────────────────────────▼─────────────┐
       │                    │  hf space apullz/ayesha-bot             │
       │                    │  (docker + ollama nemotron-3-nano +     │
       │                    │   gradio, streams via SSE)              │
       │                    └─────────────────────────────────────────┘
```

## projects

| project | lang | description |
|---------|------|-------------|
| **engine/** | rust | agentic coding assistant + jarvis chatbot with tool-calling, model routing, streaming (syntax-highlighted), themes, sessions, self-improvement, pixel art generation |
| **ayesha-bot-mobile/** | typescript (expo) | pastel "magical chat" mobile client that streams from the hf bot space via gradio SSE |
| **_hf-ayesha-bot/** | docker + gradio | huggingface space that runs ollama (`nemotron-3-nano:4b`) as the `ayesha` personality and serves it behind a phone-frame chat overlay |
| **tri_mind_sync/** | python | bidirectional sync engine (github, huggingface, local) |
| **git_middleware/** | python | gitea webhook receiver + LLM task runner (code review, security scan) |
| **skills/** | markdown | skill guides the engine discovers and loads at runtime (`list_skills` / `read_skill`) |
| **models/** | modelfile | ayesha ollama personality definition |

### applets/

| applet | lang | description |
|--------|------|-------------|
| **flora-cli/** | typescript | interactive terminal for exploring scottish flora phylogeny |
| **hivebeat/** | python | live-coding terminal music synth (numpy, sample-accurate loop scheduler) — `python repl.py`; live audio via termux pulse, offline wav render anywhere |

## quick start

### step 0 — get the code

```cmd
git clone https://github.com/apullz/ayesha-os.git
cd ayesha-os
```

already have it / got a release zip? just skip ahead.

### step 1 — prerequisites

| requirement | why | check with |
|-------------|-----|------------|
| **windows 10/11** | the primary supported platform (win32 + crossterm UI) | — |
| **ollama** | local model runtime (API on `localhost:11434`) | `ollama --version` |
| **git** | clone the repo | `git --version` |

**end users** only need the app binary (step 3a / step 3c) — no rust toolchain required.
**developers** who rebuild from source also need:

- **rust toolchain** (stable) — `rustup --version`
- **visual studio 2022 build tools** — provides the `vcvars64.bat` that `ayesha.bat` loads

### step 2 — install the ayesha model

1. make sure ollama is running: `ollama list` (should not error)
2. pull the base model the personality is built on:

```cmd
ollama pull mistral-nemo:12b
```

3. build the ayesha personality from the modelfile (from the repo root):

```cmd
ollama create ayesha -f models/Modelfile
```

4. verify the model exists:

```cmd
ollama list
::  ayesha:latest  should be listed
```

### step 3a — run: end user (recommended)

```cmd
cd dist
.\ayesha-os.exe
```

`dist\ayesha-os.exe` (windows) and `dist/ayesha-os` (linux/termux) are the app — a
self-contained build with config, models, and applets bundled in. no rust, no build
tools needed.

> don't have `dist\`? rebuild it (developer path below) or grab a release.

### step 3b — run: developer

iterating on the code? run straight from source (needs rust + vs build tools):

```cmd
ayesha.bat
```

or manually:

```cmd
cd engine
cargo run --release
```

> rebuild the exe after any engine/applet change with `.\scripts\build-exe.ps1`
> (linux/termux twin: `./scripts/build-linux.sh`)

### step 3c — run: linux & termux

on linux (and termux on android) the same engine builds to an elf:

```bash
./scripts/build-linux.sh     # compile release + bundle dist/ (auto-detects termux)
./ayesha.sh                  # launch dist/ayesha-os
```

or `cd dist && ./ayesha-os` directly — `dist/ayesha-os` is the linux app, a
self-contained build with config, models, and applets bundled in, just like the
exe on windows.

termux prerequisites: `pkg install rust` (plus `nodejs` for flora-cli; hivebeat
needs `python python-numpy pulseaudio` — run `bash applets/hivebeat/setup_termux.sh`
for the audio setup, which the build script also reminds you about). clipboard
tools (`read_clipboard`, ctrl+v vision) are no-ops on termux — there's no android
clipboard backend — and `--headless` / `--selftest` run fine with no display.

for local models on termux either `pkg install ollama` (native aarch64) or point
`ayesha.json`'s `ollama.endpoint` at a remote box (`"http://<host>:11434"`). cloud
keys work the same as windows: `./scripts/setup-cloud.sh`.

### step 4 — (optional) enable free cloud models

cloud backends (openrouter, kilo gateway) are free and handy when local models are slow.
set the keys once:

```powershell
.\scripts\setup-cloud.ps1
```

this writes `.env` with `OPENROUTER_API_KEY` (it opens the key
pages in your browser for you). then, inside the engine:

```cmd
fox> models                          # list everything (local + cloud)
fox> model nvidia/nemotron-3-super:free
fox> auto                            # back to auto-routing
```

### step 5 — verify it works

```cmd
.\dist\ayesha-os.exe --selftest      # headless E2E smoke test, windows (exit 0 = ok)
./dist/ayesha-os --selftest          # ...and the linux/termux elf (no display needed)
cd engine && cargo test              # 191 unit tests, 1 warning (dead code)
```

then just start chatting, fox: `fox> write me a haiku` (￣▽￣)

## engine features

the engine is the heart of ayesha-os — an agentic coding assistant with a full persona.

### dual backend: local + cloud

| backend | provider | models |
|---------|----------|--------|
| **local** | ollama @ `localhost:11434` | ayesha, qwen2.5-coder:14b, llama3.2-vision |
| **cloud** | openrouter (free tier) | nvidia/nemotron-3-super:free, meta-llama/llama-3.3-70b-instruct:free, deepseek-r1:free, qwen-2.5-coder-32b:free, xiaomi/mimo-v2.5, xiaomi/mimo-v2.5-pro |
| **cloud** | kilo | kilo-auto/free |

```bash
fox> models                  # list all available models (local + cloud)
fox> model deepseek-r1:free  # switch to a cloud model
fox> auto                    # re-enable auto-routing
```

### model routing

auto-routes queries to the best model based on content:
- **coding keywords** (code, implement, function, debug, refactor) → coding model
- **vision keywords** (image, screenshot, look, picture) → vision model
- **general** → default text model

### agentic tool calling

the model can autonomously call tools to complete tasks (26 tools):

| tool | description |
|------|-------------|
| `read_file` | read any file on disk (path sandbox is opt-in, see below) |
| `write_file` | create or overwrite files |
| `list_dir` | browse directories |
| `grep` | recursive text search (case-insensitive substring, `path:line:` results) |
| `glob` | find files by pattern (`**`, `*`, `?`), recursive |
| `list_skills` | list available skills from the `skills/` folder |
| `read_skill` | load a skill's instructions and follow them |
| `generate_html` | generate self-contained interactive html apps |
| `generate_sprite` | create pixel art character sprite sheets |
| `generate_tileset` | create terrain tilesets (grass, desert, water, snow) |
| `generate_object` | create item/object sprites (tree, rock, chest, potion) |
| `render_sprite` | generate interactive HTML sprite viewer with CRT effects |
| `remember` | store persistent memories with categories and importance |
| `list_memories` | browse stored memories |
| `search_memories` | search memories by keyword |
| `set_preference` | store user preferences across sessions |
| `analyze_self` | AI-powered code review of own source files |
| `list_source_files` | list all source files with line counts |
| `evolve_tools` | analyze gaps and suggest new tool definitions |
| `refine_prompt` | analyze tool usage history and suggest prompt improvements |
| `get_tool_stats` | display per-tool success rates with bar charts |
| `read_clipboard` | read system clipboard (text or image) |
| `coding_agent` | multi-action coding tool (read/write/edit/analyze/modify/suggest) |
| `fetch_url` | download any file (html/json/binary) from a URL to a local path (100 MB cap) |
| `download_image` | download an image from a URL, validate it's really an image, auto-pick extension |
| `manage_applet` | list / launch / stop applets from inside the engine |

### streaming + steering

responses stream in real-time with retro typewriter effect. a `lowercase-proxy` formatter keeps output on-brand (all-lowercase outside code fences, text kaomojis preserved), and a code renderer paints fenced code blocks and inline `` `code` `` spans with syntax highlighting (keywords, strings, numbers, comments, functions, types). type during generation to interrupt and redirect:

```
fox> write a fibonacci function
  >> sure thing, fox! here's a...
  -- interrupted --
fox> actually, make it recursive
```

### themes

six color themes — `kook`, `cyberpunk`, `sakura`, `win95`, `mono`, `monokai++` (default) — each with 12 role colors + 9 syntax-token colors:

```bash
fox> theme            # list themes
fox> theme sakura     # switch to a theme
```

generate a theme from any image: `python scripts/theme-from-image.py --image shot.png`

### sessions

conversations auto-save to `sessions/default.json` and can be snapshotted and resumed:

```bash
fox> save my-idea      # save the conversation
fox> sessions          # list auto-saved sessions
fox> resume my-idea    # resume a saved session
fox> newsession        # start fresh
fox> export chat.md    # export as markdown
```

### skills

drop a markdown guide in `skills/` and the engine will auto-discover it (parses YAML frontmatter for `name`/`description`), surface it via `list_skills`, and hint the model to call `read_skill` when a request matches. bundled: `code-review`, `rust-build`.

### meta-commands

| command | action |
|---------|--------|
| `help` | show help |
| `exit` | quit (saves memory + prompt history) |
| `clear` | clear screen |
| `reset` | wipe conversation + memory |
| `models` | list available models (local + cloud) |
| `model <name>` | switch model manually |
| `toolmodel [name]` | view / switch the tool-calling model |
| `auto` | re-enable auto-routing |
| `route <query>` | route one query manually |
| `pull <name>` | pull a model from ollama |
| `theme [name]` | list / switch color theme |
| `apps` | list registered applets |
| `run <name>` | launch an applet |
| `stop <name>` | stop an applet |
| `stats` | tool usage statistics |
| `memory` | list stored memories |
| `skills` | list available skills |
| `history [n]` | show last n messages |
| `compact` | trim history to system + ~8 messages |
| `save [path]` | save conversation to JSON |
| `load [path]` | load a conversation from JSON |
| `sessions` | list auto-saved sessions |
| `resume [name]` | resume a saved session |
| `newsession` | start a fresh conversation |
| `system` | print the current system prompt |
| `export [path]` | export conversation as markdown |
| `ping` | latency test to the active model |
| `joke` | random programmer joke |
| `time` | current UTC time |
| `uptime` | session uptime |
| `config [key=value]` | view / edit config |
| `analyze` | analyze own source code |
| `evolve` | suggest new tools |
| `refine` | analyze prompt history |
| `sync` | push changes to github + huggingface |
| `name <you>` | set your display name |
| `/` | open command palette |

**Tab** autocompletes slash commands and applet names (repeat to cycle, double-Tab lists all). **Ctrl+M** or **Ctrl+P** opens the interactive applet menu — navigate with **↑/↓**, type to filter, **Enter** to launch, **x** to stop the selected applet, **Esc** to exit. **Shift+Up/Down** cycles through applets.

### pixel striker (built-in sprite engine)

generates pixel art programmatically with:
- 7-color character palettes (skin, hair, shirt, pants, shoes, visor, circuit)
- 4 animation states: idle (breathing bob), walk_left, walk_right, hacking_active
- checkerboard dithering for soft shadows
- sub-pixel visor glow effects
- circuit-line highlights for tech aesthetic

### self-improvement loop

the engine learns and evolves:
- **persistent memory** — stores preferences, facts, conversation highlights, and errors across sessions at `~/.ayesha/memory.json`
- **self-analysis** — scans own `.rs` source for unused imports, `unwrap()` usage, long lines, TODOs
- **tool evolution** — identifies missing tools and generates definitions + implementation skeletons
- **prompt refinement** — tracks tool success rates and suggests system prompt changes
- **error auto-memory** — tool failures automatically stored as memories
- **191 unit tests** — `cargo test` (1 dead-code warning); `ayesha-os --selftest` runs a headless E2E smoke test

### sandbox security

strict path blocking is **opt-in**: set `"sandbox": true` in `ayesha.json`. by default the sandbox is permissive (legacy behavior — it resolves paths but does not block sensitive files). with `"sandbox": true`:

- blocks access to sensitive paths: `.env`, `.ssh`, `.gnupg`, `.aws`, `.password`, `.secret`, `.token`
- `write_file` respects the ReadOnly attribute instead of auto-clearing it
- path traversal prevention via canonicalization
- all file operations go through `Sandbox::resolve()`

## core features

### gradio web ui

```bash
cd core && python app.py
```

personality engine with three-layer response system (computer/otacon/win95) served via gradio.

### fastapi mobile api

```bash
cd core && python ayesha_mobile_api.py
```

REST API for mobile apps with:
- device registration + heartbeat
- hive status broadcasting
- personality config read/write
- WebSocket real-time updates
- Android-specific endpoints (`/api/android/init`, `/api/android/session`)
- tri-node mind integration — broadcasts fan out to all hivemind nodes
- `/api/hive/*` — list active sisters, shared key/value config store
- background 30s hive sync + heartbeat loop

### hivemind client

`ayesha_hive_client.py` — client library for instances to register with, sync, and discover sister instances in the hivemind.

## tri-mind sync

bidirectional sync between three nodes:

| node | purpose |
|------|---------|
| **local** | your development machine |
| **github** | version control + collaboration |
| **huggingface** | public model + space hosting |

```bash
python -m tri_mind_sync.cli status   # check sync state
python -m tri_mind_sync.cli sync     # run full sync
python -m tri_mind_sync.cli scan     # detect local changes
python -m tri_mind_sync.cli push     # push to github
python -m tri_mind_sync.cli watch    # continuous sync loop
```

or use the all-in-one script:

```powershell
.\scripts\sync-all.ps1    # push github + hf model + hf space
```

## applets detail

### flora-cli

interactive TypeScript terminal for exploring scottish flora phylogeny. uses ollama for natural language queries about plant taxonomy.

### hivebeat

live-coding terminal music synth — type pattern lines, they loop forever, edits land on the next cycle boundary (sorensen & gardner "programming with time" scheduler, sample-accurate). pure python + numpy, square/saw/fm/pad melodies + euclidean drums:

```
python repl.py    # the live repl: p1.square("c4 e4 g4 a4").dur(0.25), bpm(140), stop
python render.py out.wav 8 'p1.saw("c2 g2 [c3 e3 g3]").dur(1)'   # offline wav, works anywhere
```

live audio streams to a bundled pulse-simple player (`hivepipe`) on termux/proot; without a pulse path the repl falls back to a realtime silent clock. on windows, `readline` isn't in cpython, so use the `render.py` path (numpy-only).

## mobile app

`ayesha-bot-mobile/` — an **expo / react-native** chat client for the ayesha bot:

- pastel "magical chat" UI with 3 tabs: **Chat**, **Stars** (saved conversations), **Settings**
- streams replies from the `apullz/ayesha-bot` hf space via gradio SSE v3 (`/gradio_api/call/respond`)
- haptics on send, animated sparkles, on-device history via AsyncStorage (no accounts)
- run with `npx expo start` inside `ayesha-bot-mobile/`

## hf bot space

`_hf-ayesha-bot/` — the dockerized **`apullz/ayesha-bot`** huggingface space that powers the mobile app:

- `FROM ollama/ollama:latest` — bakes `nemotron-3-nano:4b` in as the `ayesha` personality at image build time
- gradio `ChatInterface` on port 7860 streaming via `/api/chat`, wrapped in a custom phone-frame overlay (`overlay.html` / `theme.css` / `chat.js`)
- deploy with the `Dockerfile` + `Modelfile` in that folder

## automation harness

`.github/workflows/harness.yml` runs `scripts/automation_harness.py` in CI **every 10 minutes** (plus manual dispatch). the harness:

- polls `.automation/tasks/` for `*.task.json` jobs and runs them non-interactively (command / sync / test / lint)
- writes files atomically (temp → fsync → SHA-256 → rename) with backup/restore on failure
- lints changed files per language (`.py`, `.rs` via `cargo check`, `.ts/.tsx` via `tsc`, `.json`, `.toml`, `.ps1`, `.sh`)
- auto-commits and pushes any results back as `chore(harness): ... [skip ci]`

## git middleware

gitea webhook receiver with LLM-powered task execution:

| endpoint | description |
|----------|-------------|
| `POST /webhook/gitea` | receive push/PR/release webhooks |
| `POST /task` | on-demand LLM task execution |
| `POST /reload` | hot-reload `config.json` |
| `GET /config` | router / model mapping summary |
| `GET /health` | server health check |

tasks: code review, auto-summary, commit analysis, security scan. uses ollama with task-specific prompt templates. `python main.py test` runs a built-in test suite (router mapping, HMAC verification, webhook mapping).

## deployment

### github

this monorepo lives at `github.com/apullz/ayesha-os`

### huggingface

- **model**: `apullz/ayesha` — ayesha personality modelfile
- **space**: `apullz/ayesha-hivemind` — gradio web ui
- **space**: `apullz/ayesha-bot` — dockerized chat backend powering the mobile app

```powershell
$env:HF_TOKEN = "hf_..."
.\scripts\sync-all.ps1
```

### scripts

| script | purpose |
|--------|---------|
| `build-exe.ps1` | build `dist\ayesha-os.exe` with config, models, applets bundled |
| `build-linux.sh` | linux/termux twin — builds `dist/ayesha-os`, installs hivepipe on termux |
| `sync-all.ps1` | push github + hf model + hf space |
| `setup-cloud.ps1` | interactive `.env` setup for openrouter cloud keys |
| `setup-cloud.sh` | bash twin of `setup-cloud.ps1` (linux/termux) |
| `launcher.py` | system-tray applet launcher (pystray) |
| `automation_harness.py` | zero-touch task queue / lint / self-correct loop |
| `import-takeout-memories.py` | distill google takeout activity into ayesha memories |
| `theme-from-image.py` | extract a color palette from an image → `ayesha.json` theme |
| `space-app.py` | static themed gradio demo for the hf space |

### building the standalone exe / elf

windows (powershell):

```powershell
.\scripts\build-exe.ps1
```

creates `dist\ayesha-os.exe` with bundled config, models, and applets.

linux / termux (bash):

```bash
./scripts/build-linux.sh
```

creates `dist/ayesha-os` with the same bundle — and when run inside termux it
auto-detects `$TERMUX_VERSION` and installs the hivebeat audio player
(`hivepipe`) into `$PREFIX/bin` for you.

## license

apache 2.0

---

```
(๑蔷蔷๑)  kapoo! the hivemind is alive!!  (๑蔷蔷๑)
```
