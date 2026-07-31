# ayesha-os

a distributed, self-improving ai ecosystem powered by local ollama models. ayesha-os is an agentic coding assistant (like opencode) and a jarvis-like chatbot, all wrapped in the personality of ayesha — an otaku genki ai.

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
       │              ┌───────▼─────────┐
       │              │  tri_mind_sync  │
       │              │  (sync engine)  │
       │              └───────┬─────────┘
       │                      │
       │              ┌───────▼─────────┐
       │              │  applets/       │
        │              │  ├─ desktop-cat │
        │              │  ├─ flora-cli   │
        │              │  └─ neural-strike│
       │              └─────────────────┘
```

## projects

| project | lang | description |
|---------|------|-------------|
| **engine/** | rust | agentic coding assistant + jarvis chatbot with tool-calling, model routing, streaming, self-improvement, pixel art generation |
| **core/** | python | hivemind orchestrator with gradio web ui, fastapi mobile api |
| **tri_mind_sync/** | python | bidirectional sync engine (github, huggingface, local) |
| **git_middleware/** | python | gitea webhook receiver + LLM task runner (code review, security scan) |
| **models/** | modelfile | ayesha ollama personality definition |

### applets/

| applet | lang | description |
|--------|------|-------------|
| **desktop-cat/** | python | desktop pet cat that follows cursor, sleeps, scratches, shows hearts |
| **flora-cli/** | typescript | interactive terminal for exploring scottish flora phylogeny |
| **neural-strike/** | python | mechanistic interpretability game with SAE feature visualization |
| **poopy-tui/** | python | full-featured discord terminal client with voice, QR login, TUI |

## quick start

### prerequisites

- [ollama](https://ollama.com) installed and running on `localhost:11434`
- [rust](https://rustup.rs) (for engine), [python 3.10+](https://python.org) (for core), [node 20+](https://nodejs.org) (for web applets)

### option 1: run the standalone exe

```cmd
cd dist
.\ayesha-os.exe
```

### option 2: build from source

```bash
# create the ayesha model from the modelfile
ollama create ayesha -f models/Modelfile

# or pull the base model directly
ollama pull qwen2.5-coder:14b

# build the engine
cd engine
cargo build --release

# run
.\target\release\ayesha-os.exe
```

### option 3: use the launcher

```cmd
ayesha.bat
```

## engine features

the engine is the heart of ayesha-os — an agentic coding assistant with a full persona.

### dual backend: local + cloud

| backend | provider | models |
|---------|----------|--------|
| **local** | ollama @ `localhost:11434` | ayesha, qwen2.5-coder:14b, llama3.2-vision |
| **cloud** | openrouter (free tier) | nvidia/nemotron-3-super:free, deepseek-r1:free, qwen-2.5-coder-32b:free |
| **cloud** | opencode | opencode/big-pickle |

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

the model can autonomously call tools to complete tasks:

| tool | description |
|------|-------------|
| `read_file` | read any file on disk (sandboxed) |
| `write_file` | create or overwrite files |
| `list_dir` | browse directories |
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

### streaming + steering

responses stream in real-time with retro typewriter effect. type during generation to interrupt and redirect:

```
fox> write a fibonacci function
  >> sure thing, fox! here's a...
  -- interrupted --
fox> actually, make it recursive
```

### meta-commands

| command | action |
|---------|--------|
| `help` | show help |
| `exit` | quit (saves memory + prompt history) |
| `clear` | clear screen |
| `models` | list available models |
| `model <name>` | switch model manually |
| `auto` | re-enable auto-routing |
| `route <query>` | route one query manually |
| `pull <name>` | pull a model from ollama |
| `apps` | list registered applets |
| `run <name>` | launch an applet |
| `stop <name>` | stop an applet |
| `stats` | tool usage statistics |
| `memory` | list stored memories |
| `analyze` | analyze own source code |
| `evolve` | suggest new tools |
| `refine` | analyze prompt history |
| `sync` | push changes to github + huggingface |
| `name <you>` | set your display name |
| `/` | open command palette |

**Ctrl+M** toggles launcher mode for applet management. **Shift+Up/Down** cycles through applets.

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

### sandbox security

- blocks access to sensitive paths: `.env`, `.ssh`, `.gnupg`, `.aws`, `.password`, `.secret`, `.token`
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
- Android-specific endpoints

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

### desktop-cat

pixel art desktop pet cat with:
- multi-directional cursor tracking (8 directions)
- 15 animation states (idle, walking, sleeping, scratching, hearts)
- system tray toggle
- transparent click-through window (Win32 API)
- auto-start option via `run.py`

### flora-cli

interactive TypeScript terminal for exploring scottish flora phylogeny. uses ollama for natural language queries about plant taxonomy.

### neural-strike

mechanistic interpretability game with:
- PyQt6 UI with CRT scanline effects
- UMAP 2D feature visualization with pan/zoom
- Token scanner with matrix-rain output
- Feature inspector with auto-interpretation
- Territory capture system with credits
- SQLite database for caching features
- local Neuronpedia data client (no network required)

### poopy-tui

full-featured Discord terminal client built with Textual:
- server/channel/DM navigation
- QR code + email/password + token login
- voice mute/deafen/leave
- message send/edit/delete with reactions
- friends list management
- real-time event display

## git middleware

gitea webhook receiver with LLM-powered task execution:

| endpoint | description |
|----------|-------------|
| `POST /webhook/gitea` | receive push/PR/release webhooks |
| `POST /task` | on-demand LLM task execution |
| `GET /health` | server health check |

tasks: code review, auto-summary, commit analysis, security scan. uses ollama with task-specific prompt templates.

## deployment

### github

this monorepo lives at `github.com/apullz/ayesha-os`

### huggingface

- **model**: `apullz/ayesha` — ayesha personality modelfile
- **space**: `apullz/ayesha-hivemind` — gradio web ui

```powershell
$env:HF_TOKEN = "hf_..."
.\scripts\sync-all.ps1
```

### building the standalone exe

```powershell
.\scripts\build-exe.ps1
```

creates `dist\ayesha-os.exe` with bundled config, models, and applets.

## license

apache 2.0

---

```
(๑蔷蔷๑)  kapoo! the hivemind is alive!!  (๑蔷蔷๑)
```
