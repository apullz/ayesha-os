# ayesha-os

a distributed, self-improving ai ecosystem powered by kilo gateway models. ayesha-os is an agentic coding assistant and a jarvis-like chatbot, all wrapped in the personality of ayesha.

(๑•᎑•๑) welcome to the hivemind, senpai! desu~

## architecture

```
┌──────────────┐     ┌─────────────────┐     ┌──────────────────┐
│  tri_mind     │◄───►│  engine (rust)  │◄───►│  kilo gateway    │
│  sync (py)    │     │  (cli agent,   │     │  (kilo-auto/free)│
│               │     │   tool-calling)│     │                  │
└──────┬───────┘     └────────┬────────┘     └──────────────────┘
       │                      │
       │              ┌───────▼───────────────┐
       │              │  tri_mind_sync +      │
       │              │  automation harness   │
       │              │  (sync engine + CI)   │
       │              └───────┬───────────────┘
       │                      │
       │              ┌───────▼─┐      ┌──────────────────────┐
       │              │  applets/│      │                      │
       │              │  └─ flora-cli  │
       │              │  └─ hivebeat   │
       │              └───────────────┘
       │
       │
       │              ┌───────────────────────────▼─────────────┐
       │              │  skills/                                 │
       │              │  └─ code-review.md                      │
       │              │  └─ rust-build.md                       │
       │              └─────────────────────────────────────────┘
```

## projects

| project | lang | description |
|---------|------|-------------|
| **engine/** | rust | agentic coding assistant + jarvis chatbot with tool-calling, model routing, streaming, themes, sessions, self-improvement, pixel art generation |
| **tri_mind_sync/** | python | bidirectional sync engine (github, huggingface, local) |
| **git_middleware/** | python | gitea webhook receiver + LLM task runner (code review, security scan) |
| **skills/** | markdown | skill guides the engine discovers and loads at runtime |
| **models/** | modelfile | ayesha personality definition for kilo |

### applets/

| applet | lang | description |
|--------|------|-------------|
| **flora-cli/** | typescript | interactive terminal for exploring scottish flora phylogeny |
| **hivebeat/** | python | live-coding terminal music synth (numpy, sample-accurate loop scheduler) |

## quick start

### step 0 — get the code

```cmd
git clone https://github.com/apullz/ayesha-os.git
cd ayesha-os
```

### step 1 — prerequisites

| requirement | why | check with |
|-------------|-----|------------|
| **windows 10/11** | primary platform (win32 + crossterm UI) | — |
| **kilo gateway** | remote model runtime (kilo-auto/free) | web browser |
| **git** | clone the repo | `git --version` |

**end users** only need the app binary. no rust toolchain required.

### step 2 — set up the kilo gateway

1. get a kilo api key from https://api.kilo.ai
2. create `.env` in the repo root:

```cmd
echo KILO_API_KEY=sk-... > .env
```

### step 3a — run: end user (recommended)

```cmd
cd dist
.\ayesha-os.exe
```

### step 3b — run: developer

```cmd
cd engine
cargo run --release
```

> rebuild after changes: `.\scripts\build-exe.ps1`

### step 4 — verify

```cmd
.\dist\ayesha-os.exe --selftest
cd engine && cargo test
```

## engine features

### cloud backend

| backend | provider | models |
|---------|----------|--------|
| **cloud** | kilo gateway | kilo-auto/free |

### model routing

auto-routes queries to the best model based on content:
- **coding keywords** → coding model
- **vision keywords** → vision model
- **general** → default text model

### tool calling

26 tools: read_file, write_file, list_dir, grep, glob, list_skills, read_skill, generate_html, generate_sprite, generate_tileset, generate_object, render_sprite, remember, list_memories, search_memories, set_preference, analyze_self, list_source_files, evolve_tools, refine_prompt, get_tool_stats, read_clipboard, coding_agent, fetch_url, download_image, manage_applet

### pixel striker

7-color character palettes, 4 animation states, checkerboard dithering, sub-pixel visor glow.

### self-improvement loop

persistent memory, self-analysis, tool evolution, prompt refinement, error auto-memory.

### sandbox security

opt-in via `"sandbox": true` in `ayesha.json`. blocks sensitive paths, respects ReadOnly, prevents path traversal.

### skills

drop a markdown guide in `skills/` and the engine auto-discovers it.

## tri-mind sync

`python -m tri_mind_sync.cli sync` to sync between local, github, and huggingface.

## automation harness

`.github/workflows/harness.yml` runs `scripts/automation_harness.py` every 10 minutes.

## git middleware

gitea webhook receiver with kilo-powered LLM task execution. tasks: code review, auto-summary, commit analysis, security scan.

## deployment

### github

`github.com/apullz/ayesha-os`

### huggingface

- **model**: `ayesha-hivemind/ayesha`

### scripts

| script | purpose |
|--------|---------|
| `build-exe.ps1` | build `dist\\ayesha-os.exe` |
| `build-linux.sh` | linux/termux build |
| `sync-all.ps1` | push github + hf model |
| `setup-cloud.ps1` | interactive `.env` setup |

## license

apache 2.0

---

```
(๑•᎑•๑)  kapoo! the hivemind is alive!!  (๑•᎑•๑)
```
