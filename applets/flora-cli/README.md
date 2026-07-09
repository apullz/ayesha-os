╔══════════════════════════════════════════════════════════════╗
║                                                              ║
║     ███████╗██╗      ██████╗ ██████╗  █████╗                ║
║     ██╔════╝██║     ██╔═══██╗██╔══██╗██╔══██╗               ║
║     █████╗  ██║     ██║   ██║██████╔╝███████║               ║
║     ██╔══╝  ██║     ██║   ██║██╔══██╗██╔══██║               ║
║     ██║     ███████╗╚██████╔╝██║  ██║██║  ██║               ║
║     ╚═╝     ╚══════╝ ╚═════╝ ╚═╝  ╚═╝╚═╝  ╚═╝               ║
║                                                              ║
║         ██████╗██╗     ██╗                                 ║
║        ██╔════╝██║     ██║                                 ║
║        ██║     ██║     ██║                                 ║
║        ██║     ██║     ██║                                 ║
║        ╚██████╗███████╗██║                                 ║
║         ╚═════╝╚══════╝╚═╝                                 ║
║                                                              ║
║        flora-cli  ::  v1.2                                   ║
║        "scottish flora phylogeny explorer"                   ║
║                                                              ║
╚══════════════════════════════════════════════════════════════╝

  ── R E L E A S E   I N F O ──

  Title    : CALEDONIAN PHYLOGENETIC TERMINAL
  Version  : v1.2
  Type     : Terminal Flora Browser + AI Botanist
  Language : TypeScript / Express / React
  Model    : qwen2.5:7b (via local ollama)
  Status   : LOCAL OLLAMA ONLY (Gemini removed 07/2026)

  ── D E S C R I P T I O N ──

  flora-cli is an interactive terminal emulator for exploring
  the evolutionary tree of scottish flora. navigate taxonomic
  clades like directories (cd, ls), inspect species, and ask
  the "Caledonian Botanist Sage" questions about each plant.

  the phylogeny data covers scottish flora from primitive
  liverworts through flowering plants, with detailed lore
  about traditional uses, gaelic names, and ecological roles.

  the "ask" command sends queries to a local ollama model
  (qwen2.5:7b) with full context about the current taxonomic
  position. results are displayed in the terminal view.

  ── I N S T A L L ──

  npm install
  ollama pull qwen2.5:7b

  ── U S A G E ──

  # CLI mode
  npx tsx cli.ts

  # web server mode
  npm run dev
  # opens at http://127.0.0.1:3000

  available commands: ls, cd, pwd, tree, info, ask, search,
  help, exit

  ── M I G R A T I O N   N O T E S ──

  this app was originally part of Google AI Studio and used
  GEMINI_API_KEY for the botanist AI. as of july 2026:

  - cli.ts: Gemini API call → ollama /api/chat
  - server.ts: GoogleGenAI → direct ollama fetch
  - caledonian-cli.js: deleted (compiled output)
  - .env.example: GEMINI_API_KEY → OLLAMA_HOST
  - package.json: @google/genai removed
  - host: 0.0.0.0 → 127.0.0.1

  ── F I L E S ──

  cli.ts                 terminal UI + command handler
  server.ts              express backend + react dev server
  .env.example           config (no api keys)
  package.json           dependencies (no google deps)
  src/
    types.ts             type definitions
    data/floraData.ts    full scottish flora phylogeny dataset
    components/Terminal.tsx  react terminal emulator

  ── D A T A   S T R U C T U R E ──

  the flora dataset uses a hierarchical tree:
  domain → kingdom → phylum → class → order → family →
  genus → species

  each node has: name, rank, children[], and optional lore
  text for species nodes with traditional uses, etymology,
  and ecological notes.

  ── N O T E S ──

  - the data is embedded directly in floraData.ts (~350 lines)
  - "ask" command context includes current path + active species
  - lore text is displayed when browsing species nodes

  ── G R E E T S ──

  scotland's botanical heritage for the inspiration.
  ollama for keeping the botanist sage alive without google.
  "the rowan tree bears fruit of knowledge" — desu-ne :3
