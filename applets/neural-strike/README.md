╔══════════════════════════════════════════════════════════════╗
║                                                              ║
║   ███╗   ██╗███████╗██╗   ██╗██████╗  █████╗ ██╗           ║
║   ████╗  ██║██╔════╝██║   ██║██╔══██╗██╔══██╗██║           ║
║   ██╔██╗ ██║█████╗  ██║   ██║██████╔╝███████║██║           ║
║   ██║╚██╗██║██╔══╝  ██║   ██║██╔══██╗██╔══██║██║           ║
║   ██║ ╚████║███████╗╚██████╔╝██║  ██║██║  ██║███████╗      ║
║   ╚═╝  ╚═══╝╚══════╝ ╚═════╝ ╚═╝  ╚═╝╚═╝  ╚═╝╚══════╝      ║
║                                                              ║
║         ███████╗████████╗██████╗ ██╗██╗  ██╗███████╗        ║
║         ██╔════╝╚══██╔══╝██╔══██╗██║██║ ██╔╝██╔════╝        ║
║         ███████╗   ██║   ██████╔╝██║█████╔╝ █████╗          ║
║         ╚════██║   ██║   ██╔══██╗██║██╔═██╗ ██╔══╝          ║
║         ███████║   ██║   ██║  ██║██║██║  ██╗███████╗        ║
║         ╚══════╝   ╚═╝   ╚═╝  ╚═╝╚═╝╚═╝  ╚═╝╚══════╝        ║
║                                                              ║
║        neural-strike  ::  v1.0                               ║
║        "latent territory — a sae interpretability game"      ║
║                                                              ║
╚══════════════════════════════════════════════════════════════╝

  ── R E L E A S E   I N F O ──

  Title    : NEURAL-STRIKE: LATENT TERRITORY
  Version  : v1.0
  Type     : Educational Game / Mech Interp Tool
  Engine   : Python 3.10+ / PyQt6
  Model    : Gemma-2-2B (local feature analysis)
  Data     : LOCAL ONLY (network stubbed since 07/2026)
  Status   : requires pre-downloaded data exports

  ── D E S C R I P T I O N ──

  neural-strike is a visual exploration game for mechanistic
  interpretability. it renders SAE (Sparse Autoencoder)
  feature activations as a territory war game — different
  "gangs" of features compete for latent space territory.

  the game visualizes how individual neurons activate for
  different concepts (toxicity, geography, sentiment) using
  UMAP projections of internal model activations.

  features:
  - real-time UMAP viewport of feature space
  - token scanner showing activating tokens
  - feature inspector with top-activating tokens
  - CRT overlay + glitch effects for cyberpunk aesthetic
  - territory capture mechanics (game-ified interpretability)

  ── I N S T A L L ──

  pip install PyQt6 numpy Pillow requests
  python main.py

  # optional: download feature data exports into data/exports/

  ── U S A G E ──

  the app opens with a territory view of feature space.
  each colored region represents a "gang" of features.
  click on features to inspect their top-activating tokens.
  use the token scanner to see how the model processes text.

  the steer() API and neuronpedia fetch are disabled in
  local-only mode. pre-downloaded JSON exports in
  data/exports/ will populate the feature explorer.

  ── F I L E S ──

  main.py                 entry point
  config.py               colors, paths, cluster definitions
  data/
    neuronpedia_client.py  local-only cache client (no network)
    database.py            sqlite feature storage
    exported_features.json pre-cached feature data
  ui/
    main_window.py         main PyQt6 window
    umap_viewport.py       2D feature space visualization
    token_scanner.py       matrix-rain token display
    feature_inspector.py   feature detail panel
    effects.py             CRT, glitch, data-stream effects

  ── N O T E S ──

  this app analyzes gemma-2-2b features — the gemma name
  here refers to the model being studied, not the ollama
  assistant model. this is correct and intentional.

  the neuronpedia.org API calls were stubbed out in july
  2026 to make this fully local. to use with data, place
  JSON exports in data/exports/ or data/cache/.

  ── G R E E T S ──

  neuronpedia for the feature datasets. the SAE community
  for making interpretability accessible. gemma team for
  the open model. territory captured. desu-ne.
