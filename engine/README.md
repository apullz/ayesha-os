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
║     agentic coding assistant + jarvis chatbot                 ║
║                                                              ║
╚══════════════════════════════════════════════════════════════╝

  ── R E L E A S E   I N F O ──

  Title    : ayesha-engine
  Version  : v4.2.0
  Type     : CLI AI Agent / Coding Assistant / Persona Host
  Language : Rust (edition 2021)
  Target   : x86_64-pc-windows-msvc
  Features : streaming, thinking blocks, model routing,
             tool execution, command palette, retro UI,
             pixel art generation, cloud model support,
             self-improvement loop, persistent memory

  ── D E S C R I P T I O N ──

  the engine is the heart of ayesha-os. it functions as both
  an agentic coding assistant and a jarvis-like
  chatbot, all wrapped in the ayesha personality.

  connects to local kilo + cloud providers (openrouter,
  kilo), routes queries to the best model, streams
  responses with a retro cyberpunk typewriter effect, detects
  <think> reasoning blocks, and supports real-time steering.

  ── F E A T U R E S ──

  DUAL BACKEND
    local:   kilo @ kilo gateway
    cloud:   openrouter (archived), kilo
    models:  auto-detected from kilo + cloud config

  MODEL ROUTING
    auto-detects coding/vision queries by keyword
    coding -> kilo-auto/free
    vision -> kilo-auto/free
    manual: /model <name>, /route <query>, /auto

  TRUE STREAMING
    tokens print as they arrive from kilo
    <think> blocks render dimmed
    ~20ms/char typewriter effect

  STEERING
    type during generation to interrupt and redirect
    Ctrl+C to abort, Ctrl+M / Ctrl+P open interactive applet menu
    Shift+Up/Down to cycle applets

  TOOL EXECUTION (26 tools)
    file ops:    read_file, write_file, list_dir, grep, glob
    skills:      list_skills, read_skill
    generation:  generate_html, generate_sprite,
                 generate_tileset, generate_object, render_sprite
    memory:      remember, list_memories, search_memories,
                 set_preference
    analysis:    analyze_self, list_source_files, evolve_tools,
                 refine_prompt, get_tool_stats
    network:     fetch_url, download_image
    clipboard:   read_clipboard
    coding:      coding_agent (read/write/edit/analyze/modify/suggest)
    applets:     manage_applet

  PIXEL STRIKER (built-in sprite engine)
    7-color character palettes
    4 animation states: idle, walk_left, walk_right, hacking_active
    checkerboard dithering for soft shadows
    sub-pixel visor glow effects
    circuit-line highlights for tech aesthetic

  SELF-IMPROVEMENT LOOP
    persistent memory (~/.ayesha/memory.json)
    self-analysis (scans .rs for issues)
    tool evolution (generates new tool definitions)
    prompt refinement (analyzes success rates)
    error auto-memory (tool failures stored automatically)

  SANDBOX SECURITY (opt-in)
    strict enforcement is OFF by default (legacy permissive behavior);
    enable it with "sandbox": true in ayesha.json
    when enabled, blocks sensitive paths: .env, .ssh, .gnupg, .aws
    when enabled, write_file respects the ReadOnly attribute
    path traversal prevention
    all file ops through Sandbox::resolve()

  COMMAND PALETTE
    type / to see all commands overlayed on terminal
    filterable, box-drawing character frame

  RETRO CYBERPUNK UI
    green-on-black theme with box-drawing borders
    rainbow ASCII banner on startup
    kaomoji coloring in responses
    code block formatting with background coloring

  APPLET MANAGER
    Ctrl+M / Ctrl+P open interactive arrow-key menu
    type to filter applets, Enter launches, x stops
    auto-installs npm dependencies if needed
    Shift+Up/Down for quick applet cycling

  CLOUD INTEGRATION
    OpenRouter: free tier models (nemotron, llama, deepseek, qwen)
    OpenCode: big-pickle model
    API keys from .env (OPENROUTER_API_KEY, OPENCODE_API_KEY)
    SSE streaming with tool-call reconstruction

  PERSISTENT STATE
    config.json: user_name, engine config
    ~/.ayesha/memory.json: memories, preferences, facts
    ~/.ayesha/prompt_history.json: tool usage history

  ── B U I L D   R E Q U I R E M E N T S ──

  Rust 1.80+ (stable, x86_64-pc-windows-msvc)
  kilo running on kilo gateway
  models: ayesha (custom), kilo-auto/free, kilo-auto/free
  Windows SDK with rc.exe (for icon embedding)

  ── B U I L D ──

  cargo build --release

  or use the build script:
  powershell -ExecutionPolicy Bypass -File ..\scripts\build-exe.ps1

  this creates a standalone dist\ayesha-os.exe with bundled
  config, models, and applets.

  linux / termux (no msvc/vcvars/rc.exe needed — build.rs is a
  no-op off-windows, reqwest uses rustls-tls):
    bash ../scripts/build-linux.sh        # -> dist/ayesha-os (bundled the same way)

  on termux: pkg install rust first; the script auto-detects
  $TERMUX_VERSION and installs hivepipe -> $PREFIX/bin for
  hivebeat pulse audio.

  ── U S A G E ──

  $ cargo run --release

  > write a fibonacci function
    >> sure thing! here's a recursive implementation...

  > what's on my screen?
    >> i can see a terminal window with...

  > analyze tools.rs
    >> here's my analysis of the code...

  type / for command palette
  type Ctrl+C or /exit to quit

  ── C O M M A N D S ──

  /                 show command palette overlay
  /help             show help
  /exit             quit (saves memory + prompt history)
  /clear            clear screen
  /models           list available models (local + cloud)
  /model <name>     switch model
  /auto             re-enable auto-routing
  /pull <name>      pull a model
  /route <query>    route one query manually
  /apps             list registered applets
  /run <name>       launch an applet
  /stop <name>      stop an applet
  /stats            tool usage stats with bar charts
  /memory           list memories
  /analyze          analyze own source code
  /evolve           suggest new tools
  /refine           analyze prompt history
  /sync             push changes to github + huggingface
  /name <you>       set user name

  ── F I L E S ──

  src/
    main.rs             entry point, agent loop, commands
    kilo.rs           kilo client, streaming, tool defs
    cloud.rs            cloud client (openrouter/kilo)
    ui.rs               retro terminal rendering
    tools.rs            tool definitions + dispatch
    sandbox.rs          file I/O sandbox + security
    model_registry.rs   model discovery + routing
    memory.rs           persistent memory store
    self_analysis.rs    code analysis engine
    tool_evolution.rs   tool gap analysis + generation
    prompt_refinement.rs prompt history + refinement
    coding_agent.rs     multi-action coding tool
    applet_manager.rs   applet process management
    pixel_striker/      built-in pixel art engine
      mod.rs            module root
      palette.rs        color palette system
      character.rs      character sprite renderer
      renderer.rs       sprite sheet renderer
      tileset.rs        terrain tileset (stub)
      object.rs         game object sprites (stub)
  config.json           user name + engine config
  Cargo.toml            dependencies
  build.rs              icon embedding (rc.exe)

  ── D E P E N D E N C I E S ──

  runtime:
    tokio            async runtime
    reqwest          HTTP client for kilo + cloud APIs
    serde/serde_json JSON parsing
    crossterm        terminal control + raw mode
    anyhow           error handling
    colored          terminal colors
    dirs             user directory paths
    image            pixel art PNG generation
    arboard          clipboard access (text + images)

  build:
    (none — uses rc.exe directly from Windows SDK)

  ── C L O U D   M O D E L S ──

  provider     model                              context   caps
  ─────────────────────────────────────────────────────────────
  openrouter   nvidia/nemotron-3-super:free       1M       all
  openrouter   meta-llama/llama-3.3-70b:free      131K     gen+code
  openrouter   deepseek/deepseek-r1:free          64K      think
  openrouter   qwen/qwen-2.5-coder-32b:free       32K      code
  kilo     kilo-auto/free                200K     code

  ── N O T E S ──

  - defaults to model "ayesha" (custom from Modelfile)
  - config.json stores user_name — edit manually or /name
  - steering uses stdin thread + mpsc channel
  - cloud models require API keys in .env
  - icon embedded via rc.exe + build.rs (CARGO_MANIFEST_DIR)

  ── G R E E T S ──

  rust community for the best systems language.
  kilo team for local AI infrastructure.
  kilo team for the gateway.
  you, for reading this far. kapoo!! desu-ne!! :3
