╔══════════════════════════════════════════════════════════════╗
║                                                              ║
║     ███████╗ ███╗   ██╗ ██████╗ ██╗███╗   ██╗███████╗      ║
║     ██╔════╝ ████╗  ██║██╔════╝ ██║████╗  ██║██╔════╝      ║
║     █████╗   ██╔██╗ ██║██║  ███╗██║██╔██╗ ██║█████╗        ║
║     ██╔══╝   ██║╚██╗██║██║   ██║██║██║╚██╗██║██╔══╝        ║
║     ███████╗ ██║ ╚████║╚██████╔╝██║██║ ╚████║███████╗      ║
║     ╚══════╝ ╚═╝  ╚═══╝ ╚═════╝ ╚═╝╚═╝  ╚═══╝╚══════╝      ║
║                                                              ║
║     engine  ::  v4.2.0                                       ║
║     "terminal-native persona host with tool-calling"         ║
║                                                              ║
╚══════════════════════════════════════════════════════════════╝

  ── R E L E A S E   I N F O ──

  Title    : ayesha-engine
  Version  : v4.2.0
  Type     : CLI AI Agent / Persona Host
  Language : Rust (edition 2021)
  Model    : ollama + qwen2.5:7b (default)
  Features : streaming, thinking blocks, model routing,
             tool execution, command palette, retro UI
  Build    : stable-x86_64-pc-windows-gnu

  ── D E S C R I P T I O N ──

  the engine is the central CLI interface of ayesha-os. it
  connects to a local ollama instance, routes user queries to
  the best model, streams responses token-by-token with a
  retro cyberpunk typewriter effect, detects <think> reasoning
  blocks mid-stream, and supports real-time steering (type
  during generation to interrupt and redirect).

  key features:

  * TRUE STREAMING — tokens print as they arrive from ollama,
    no artificial delays. <think> blocks render dimmed.

  * MODEL ROUTING — auto-detects coding/vision queries and
    routes to the optimal model. manual /model override.

  * TOOL EXECUTION — model can call tools: read_file,
    write_file, list_dir, generate_html, generate_sprite,
    remember/search_memories, analyze_self, evolve_tools,
    refine_prompt.

  * COMMAND PALETTE — type / alone to see all commands overlayed
    on the terminal. /help, /clear, /model, /auto, /stats,
    /memory, /analyze, /evolve, /refine, /name, etc.

  * SLASH COMMANDS — /name <you> changes the stored user name
    live. /route <query> routes one query manually.

  * SELF-IMPROVEMENT — the engine can read its own source code
    and suggest improvements, evolve new tools, and refine
    its system prompt based on usage patterns.

  * RETRO CYBERPUNK UI — green-on-black theme, box-drawing
    borders (┌─┐│└┘), ◆▶✔✖ symbols, per-line rainbow ASCII
    banner on startup.

  * USER NAME INJECTION — stored in config.json, injected into
    the system prompt. model is told to use it occasionally
    (not every message).

  ── B U I L D   R E Q U I R E M E N T S ──

  Rust 1.80+ (stable)
  ollama running on localhost:11434
  models pulled: qwen2.5:7b (min), plus opt. qwen2.5-coder:14b

  ── I N S T A L L ──

  cargo run --release
  # or use the prebuilt binary in target/release/

  config.json is created on first run. edit user_name field
  or use /name <you> in-app.

  ── U S A G E ──

  $ cargo run --release

  > write a fibonacci function
    -- qwen2.5-coder:14b --
    >> sure thing! here's a recursive implementation...

  > what's on my screen?
    -- llama3.2-vision --
    >> i can see a terminal window with...

  type / for command palette
  type Ctrl+C or /exit to quit

  ── C O M M A N D S ──

  /                 show command palette overlay
  /help             show help
  /exit             quit
  /clear            clear screen
  /models           list available models
  /model <name>     switch model
  /auto             re-enable auto-routing
  /pull <name>      pull a model
  /route <query>    route one query manually
  /stats            tool usage stats
  /memory           list memories
  /analyze          analyze own source code
  /evolve           suggest new tools
  /refine           analyze prompt history
  /name <you>       set user name

  ── M O D E L   R O U T I N G ──

  the engine auto-routes by keyword detection:

  "implement", "debug", "function"     -> coding model
  "image", "screenshot", "vision"      -> vision model
  everything else                       -> qwen2.5:7b

  models are discovered dynamically from ollama at startup.
  the known_models() list provides sensible defaults.

  ── F I L E S ──

  src/
    main.rs             ~422 lines  entry point, loop, commands
    ollama.rs           ~677 lines  ollama client, streaming
    ui.rs               ~352 lines  retro terminal rendering
    tools.rs            ~240 lines  tool definitions + dispatch
    sandbox.rs           ~70 lines  file I/O sandbox
    model_registry.rs   ~248 lines  model discovery + routing
  config.json                      user name + engine config
  Cargo.toml                       dependencies
  run.bat                          windows launcher
  .cargo/config.toml               link flags

  ── D E P E N D E N C I E S ──

  reqwest          HTTP client for ollama API
  serde/serde_json JSON parsing
  tokio            async runtime
  crossterm        terminal control
  anyhow           error handling
  colored          terminal colors
  chrono           timestamps

  ── N O T E S ──

  - the engine defaults to model "ayesha" (custom from
    Modelfile) but falls back to qwen2.5:7b for routing
  - config.json stores user_name — edit manually or /name
  - steer_channel.rs was removed; steering is inline via
    stdin thread + mpsc channel
  - the windows build uses /SUBSYSTEM:WINDOWS to hide
    the console window behind the custom terminal UI

  ── G R E E T S ──

  rust community for the best systems language.
  ollama team for local AI infrastructure.
  qwen team for the base model.
  you, for reading this far. kapoo!! desu-ne!! :3
