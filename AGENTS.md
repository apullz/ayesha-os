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
│   ├── screen/          typescript — screen capture vision chatbox
│   ├── cosmic-rag/      python — local RAG chatbot
│   ├── desktop-cat/     python — desktop pet cat
│   ├── flora-cli/       typescript — scottish flora phylogeny explorer
│   ├── bring-to-life/   typescript — image to interactive html
│   ├── neural-strike/   python — SAE interpretability game
│   └── screenshotai/    python — screenshot capture + analysis
├── ayesha.json          central config (personality, projects, ollama models)
└── ayesha.bat           shortcut to launch engine
```

## key commands

### engine (rust)
```bash
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
# screen vision chatbox
cd applets/screen/ollama-screen-vision-chatbox && npm install && npm run dev

# cosmic-rag
cd applets/cosmic-rag && python main.py

# desktop-cat (no AI needed)
cd applets/desktop-cat && python desktopcat.py

# flora-cli
cd applets/flora-cli && npx tsx cli.ts

# bring-to-life
cd applets/bring-to-life && npm run dev

# neural-strike
cd applets/neural-strike && python main.py

# screenshotai
cd applets/screenshotai && python main.py
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
ollama pull moondream
ollama create ayesha -f models/Modelfile
```

## gotchas

- no test suites in any project
- `applets/desktop-cat/desktopcat.py` expects `cat.png` sprite in same dir
- python projects have no pyproject.toml — deps in requirements.txt only
- typescript projects need `npm install` before first run
