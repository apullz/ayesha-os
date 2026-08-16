╔═══════════════════════════════════════════════╗
║       ayesha-os  ::  STATUS REPORT  ::         ║
║          "[RELEASE] v4.5.0"                   ║
╚═══════════════════════════════════════════════╝

  ── L E G E N D ──

  [✔] working
  [!] works with caveats
  [✖] broken / not started
  [→] requires local ollama model

  ── P R O J E C T S ──

  engine/          [✔]  Agentic coding assistant + jarvis chatbot.
                         CLI persona host with streaming,
                         model routing, tool-calling (26 tools
                         incl. grep / glob search, skills,
                         fetch_url / download_image for
                         image & file downloads),
                         cloud support (openrouter/opencode,
                         incl. xiaomi/mimo-v2.5 + big-pickle),
                         skills/ folder with markdown guides
                         (list_skills + read_skill tools),
                         pixel striker sprite engine,
                         self-improvement loop, command palette,
                         opencode-style compact UI,
                         autocomplete (Tab cycling for commands
                         and applets), 203 unit tests, 1 pre-existing dead-code
                         warning.
                         └ 30 slash commands:
                            help, clear, models, auto, sync, apps,
                            run, stop, model, toolmodel, pull, route,
                            name, exit, stats, history, compact,
                            save, load, system, export, ping,
                            joke, time, uptime, config,
                            memory, analyze, evolve, refine, mode
                         └ model: ayesha (qwen2.5-coder:14b base)
                         └ --selftest for headless E2E verification

  tri_mind_sync/   [✔]  Bidirectional sync engine
                         (github, huggingface, local).
                         Fixed: now pushes modified files.

  git_middleware/   [✔]  Gitea webhook receiver + LLM task runner.
                         Code review, security scan, auto-summary.

  flora-cli/       [→]  Scottish flora phylo explorer.
                         └ replaced gemini with ollama 07/2026
                         └ OLLAMA_HOST env var now respected
                         └ multi-turn ask (conversation memory)
                         └ history command shows chat history
                         └ tree --depth=N limits display depth
                         └ fixed double-escaped ANSI codes
                         └ fixed package.json name + metadata

  models/          [✔]  Modelfile for ayesha personality.
                           └ FROM qwen2.5-coder:14b

  hivebeat/        [!]  Live-coding terminal music synth (numpy).
                           └ repl.py interactive loop (foreground applet)
                           └ sample-accurate cycle scheduler + tanh limiter
                           └ live audio: termux pulse (hivepipe/pacat) —
                             falls back to null sink without it
                           └ windows: cpython lacks readline, use
                             render.py for offline wav (numpy-only)

  launcher/        [✔]  In-engine interactive applet switcher.
                          └ Ctrl+M / Ctrl+P open arrow-key menu
                          └ ↑/↓ navigate, type to filter, Enter launch
                          └ x stops selected applet, Esc/back exits
                          └ /run, /stop, /apps slash commands
                          └ foreground applets (flora-cli, hivebeat) run in
                            the current window as pages; ctrl+p switches back
                          └ scripts/launcher.py (tray companion)

  ── L I N U X  /  T E R M U X ──

  linux/           [✔]  engine builds natively to an elf (no windows-only crates,
                          rustls-tls, build.rs no-op off-windows):
                          └ ./scripts/build-linux.sh -> dist/ayesha-os
                            (config + models + applets bundled, node_modules kept)
                          └ launcher: ./ayesha.sh (twin of ayesha.bat)
                          └ read_clipboard works on x11/wayland (arboard)

  termux/          [✔]  builds on-device (pkg install rust) with the same script:
                          └ auto-detects $TERMUX_VERSION, installs
                            hivepipe -> $PREFIX/bin for hivebeat audio
                          └ audio setup: bash applets/hivebeat/setup_termux.sh
                          └ clipboard tools are no-ops (no android backend)
                          └ --headless / --selftest work headlessly

  ── K N O W N   I S S U E S ──

  ── S E C U R I T Y   N O T E S ──

  * no API keys committed — all .env* in .gitignore
  * git author changed to noreply email
  * 0.0.0.0 binds changed to 127.0.0.1
  * no personal paths or system info in source
  * sandbox blocks sensitive paths (.env, .ssh, .gnupg, .aws)

  ── M O D E L S   N E E D E D ──

  ollama pull qwen2.5-coder:14b  # default text + coding model
  ollama pull llama3.2-vision    # vision tasks
  ollama create ayesha -f models/Modelfile  # custom personality

  ── C L O U D   M O D E L S   (f r e e) ──

  openrouter:   nvidia/nemotron-3-super:free
                meta-llama/llama-3.3-70b-instruct:free
                deepseek/deepseek-r1:free
                qwen/qwen-2.5-coder-32b-instruct:free
                xiaomi/mimo-v2.5            (1M ctx, tools, vision)
                xiaomi/mimo-v2.5-pro        (1M ctx, agentic, thinking)
  opencode:     opencode/big-pickle

  ── D E P L O Y M E N T ──

  github:       git push origin master
  huggingface:  $env:HF_TOKEN = "hf_..."; .\scripts\sync-all.ps1
  standalone:   .\scripts\build-exe.ps1 -> dist\ayesha-os.exe
                ./scripts/build-linux.sh -> dist/ayesha-os   (linux / termux)
