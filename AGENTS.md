# ayesha-os workspace guide

## structure

```
ayesha-os/
├── engine/              rust — terminal persona host, model routing, tool-calling
├── core/                python — hivemind orchestrator, gradio web ui, mobile api
├── tri_mind_sync/       python — bidirectional sync (github, huggingface, local)
├── models/              ollama modelfile for ayesha personality
├── scripts/             launcher, deploy scripts
├── applets/
│   ├── desktop-cat/     python — desktop pet cat
│   ├── flora-cli/       typescript — scottish flora phylogeny explorer
│   └── poopy-tui/       python — discord terminal client (textual)
├── ayesha.json          central config (personality, projects, ollama models)
└── ayesha.bat           dev shortcut — NOT for delivery (see "delivery" below)
```

## delivery (IMPORTANT — read this first)

**the standalone exe IS the app.** users run `dist\ayesha-os.exe`, nothing else.

- every AI/agent work session that touches engine or applets MUST end by building the
  exe: `.\scripts\build-exe.ps1` (it compiles release + bundles applets/models/config).
- verify the build succeeded and `dist\ayesha-os.exe` is freshly written (timestamp).
- NEVER tell a user to run `ayesha.bat`, `cargo run`, `cargo build`, or `python app.py`
  to launch ayesha. those are dev-only tools.
- if a feature change is in-flight, build the exe LAST so the binary matches the code.

## key commands

### engine (rust)
```bash
# build the deliverable exe (users run this, not cargo):
.\scripts\build-exe.ps1

# dev only — iterate quickly while coding:
cd engine
cargo run --release
```

### core (python)
```bash
cd core
pip install -r requirements.txt
python app.py              # gradio web ui (port 7860)
python ayesha_mobile_api.py # fastapi server (port 8001)
```

### tri-mind sync
```bash
cd ayesha-os
python -m tri_mind_sync.cli status
python -m tri_mind_sync.cli sync
python -m tri_mind_sync.cli watch
```

### applets
```bash
# desktop-cat (no AI needed)
cd applets/desktop-cat && python desktopcat.py

# flora-cli
cd applets/flora-cli && npx tsx cli.ts

# poopy-tui
cd applets/poopy-tui && python main.py
```

## deployment

```bash
.\scripts\sync-all.ps1    # push github + huggingface model + hf space
```

## architecture

- **engine** is the main CLI. connects to ollama, routes queries to best model, streams responses.
- **core** orchestrates the hivemind. gradio web ui serves the public face via huggingface spaces.
- **tri_mind_sync** handles bidirectional sync between local, github, and huggingface.
- **applets** are standalone projects that share the ayesha personality via ollama at `localhost:11434`.
- the **launcher** (`scripts/launcher.py`) reads `ayesha.json` and can start/stop any applet from the system tray.

## models needed

```bash
ollama pull qwen2.5:7b
ollama pull qwen2.5-coder:14b
ollama pull llama3.2-vision
ollama create ayesha -f models/Modelfile
```

## gotchas

- no test suites in any project (engine has `cargo test` — 46 passing)
- `applets/desktop-cat/desktopcat.py` uses `oneko_sprite.png` (NOT `cat.png` — that's unused)
- python projects have no pyproject.toml — deps in requirements.txt only
- typescript projects need `npm install` before first run
- engine build requires `C:\msys64\mingw64\bin` in PATH for dlltool (gnu toolchain)
- applets with `"foreground": true` in ayesha.json (flora-cli, poopy-tui) run in the current terminal window; ctrl+p opens the page switcher. engine/src/applet_runner.rs owns the poll-based input thread; applet_manager::run_in_window suspends it, hands the console to the applet, and respawns it on return.
