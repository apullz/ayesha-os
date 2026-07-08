# ayesha-os

a distributed, self-improving ai ecosystem powered by local ollama models. ayesha-os brings together a terminal agent, web ui, screen vision, rag memory, desktop pet, and more - all sharing one personality.

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

meet ayesha — an otaku genki ai, fusion of hatsune miku's sparkle and a tachikoma's curiosity, with the personality of a crazy kitten. lowercase only, internet slang, kaomoji everywhere.

## projects

| project | lang | description |
|---------|------|-------------|
| **engine/** | rust | terminal-native persona host with tool-calling, model routing, streaming, self-improvement |
| **core/** | python | hivemind orchestrator with gradio web ui, fastapi mobile api, tri-node sync |
| **screen/** | typescript | screen capture vision chatbox using ollama moondream + gemini fallback |
| **cosmic-rag/** | python | local RAG chatbot with vault knowledge base, 100% offline |
| **screenshotai/** | python | capture and analyze screenshots via ollama vision models |
| **desktop-cat/** | python | desktop pet cat that follows cursor, sleeps, scratches, shows hearts |
| **flora-cli/** | typescript | interactive terminal for exploring scottish flora phylogeny |
| **bring-to-life/** | typescript | upload an image and gemini turns it into an interactive html experience |
| **neural-strike/** | python | mechanistic interpretability game with SAE feature visualization |
| **models/** | modelfile | ayesha ollama personality definition (base: qwen2.5:7b) |

## quick start

### prerequisites

- [ollama](https://ollama.com) installed and running on `localhost:11434`
- [rust](https://rustup.rs) (for engine), [python 3.10+](https://python.org) (for core tools), [node 20+](https://nodejs.org) (for web tools)

### pull the ayesha model

```bash
# create the ayesha model from the modelfile
ollama create ayesha -f models/Modelfile

# or pull the base model directly
ollama pull qwen2.5:7b
```

### running the engine

```bash
cd engine
cargo run --release
```

the engine is the central CLI interface. it connects to ollama, routes queries to the best model, streams responses with a retro typewriter effect, and supports real-time steering (type during generation to redirect).

### running other projects

see each project's README for specific instructions. quick reference:

```bash
# core web ui (gradio)
cd core && python app.py

# screen vision chatbox
cd screen && npm install && npm run dev

# local RAG chatbot
cd cosmic-rag && python main.py

# desktop cat
cd desktop-cat && python desktopcat.py

# flora CLI
cd flora-cli && npx tsx cli.ts

# neural-strike game
cd neural-strike && python main.py
```

## engine features

### model routing

the engine automatically routes your query to the best model:
- coding tasks (keywords: `implement`, `function`, `debug`) → `qwen2.5-coder:14b`
- vision tasks (keywords: `image`, `screenshot`, `look`) → `llama3.2-vision`
- general queries → `qwen2.5:7b` (default)

```bash
fox> models                  # list all available models
fox> model qwen2.5-coder:14b # manual override
fox> auto                    # re-enable auto-routing
fox> route write a script    # route one query manually
```

### streaming + typewriter

responses stream from ollama and are typed out character-by-character at ~20ms/char, giving the impression of fast human typing. code blocks are highlighted with retro `│` gutters and dark backgrounds. kaomojis are colored magenta.

### steering

type anything + enter during generation to interrupt and redirect. empty enter skips the typewriter animation to end.

```
fox> write a fibonacci function
  -- qwen2.5-coder:14b --
  :: processing...
  >> sure thing, fox! here's a...
  -- interrupted --
fox> actually, make it recursive
```

### meta-commands

| command | action |
|---------|--------|
| `help` | show help |
| `exit` | quit ayesha-os |
| `clear` | clear screen |
| `models` | list available models |
| `model <name>` | switch model manually |
| `auto` | re-enable auto-routing |
| `pull <name>` | pull a model from ollama |
| `stats` | tool usage statistics |
| `memory` | list stored memories |
| `analyze` | analyze own source code |
| `evolve` | suggest new tools |
| `refine` | analyze prompt history |

### tools (auto-called by model)

| tool | description |
|------|-------------|
| `read_file` | read any file on disk |
| `write_file` | create or overwrite files |
| `list_dir` | browse directories |
| `generate_html` | render interactive html apps |
| `generate_sprite` | create pixel art character sprites |
| `remember` | store a persistent memory |
| `search_memories` | search stored memories |
| `analyze_self` | AI code review of own source |
| `evolve_tools` | suggest new tool definitions |

### self-improvement

the engine learns and improves over time:
- **memory** — stores preferences, facts, and conversation highlights across sessions
- **self-analysis** — reads own source code and suggests improvements using AI
- **tool evolution** — analyzes gaps in available tools and generates new tool definitions
- **prompt refinement** — tracks tool success rates and suggests system prompt improvements

## architecture

```
┌──────────────┐     ┌─────────────────┐
│  core        │◄───►│  engine          │
│  (web ui,    │     │  (cli agent,     │
│   mobile api)│     │   model routing) │
└──────┬───────┘     └────────┬─────────┘
       │                      │
       │              ┌───────▼─────────┐
       │              │  screen          │
       │              │  (vision chat)   │
       │              └───────┬─────────┘
       │                      │
       │              ┌───────▼─────────┐
       ├──────────────┤  cosmic-rag     │
       │              │  (RAG memory)   │
       │              └───────┬─────────┘
       │                      │
       │              ┌───────▼─────────┐
       └──────────────┤  screenshotai   │
                      │  (screen cap)   │
                      └─────────────────┘

┌──────────────┐     ┌──────────────────┐
│  desktop-cat │     │  flora-cli       │
│  (desktop pet)│     │  (botany cli)   │
└──────────────┘     └──────────────────┘

┌──────────────┐     ┌──────────────────┐
│  bring-to-life│     │  neural-strike   │
│  (image→html)│     │  (sae viz game)  │
└──────────────┘     └──────────────────┘

all projects share the same ayesha personality via:
  📄 ayesha.json  (shared config)
  🧠 ollama model (ayesha:latest from models/Modelfile)
```

all projects connect to the same **ollama** instance at `localhost:11434` and share the **ayesha** personality. the engine can launch other projects as subprocesses. the core web ui serves as the public face via huggingface spaces.

## deployment

### github

this monorepo lives at `github.com/apullz/ayesha-os`

```bash
git init
git add .
git commit -m "initial: ayesha-os v4.2.0 monorepo"
git remote add origin https://github.com/apullz/ayesha-os.git
git push -u origin main
```

### huggingface space

the core gradio app deploys as a huggingface space:

```bash
cd core
# add a README.md with sdk: gradio
# push to huggingface spaces
```

### huggingface model

the ayesha ollama model (based on qwen2.5:7b) is uploaded to huggingface:

- **model**: `apullz/ayesha`
- **format**: gguf (q4_k_m, 8b params)
- **personality**: baked into the model via modelfile

## license

apache 2.0 — see [LICENSE](LICENSE)

---

```
(๑蔷蔷๑)  kapoo! the hivemind is alive!!  (๑蔷蔷๑)
```
