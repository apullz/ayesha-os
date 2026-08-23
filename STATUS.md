# ayesha-os status

last updated: 2026-08-23

## stack

| layer | tech | purpose |
|-------|------|---------|
| cli agent | rust + crossterm | engine/src/main.rs (191 unit tests) |
| tool calling | json schema | 26 tools |
| model routing | auto | coding / vision / general |
| llm provider | kilo gateway | https://api.kilo.ai (kilo-auto/free) |
| sync engine | python | tri_mind_sync/ |
| ci | python | .github/workflows/harness.yml + automation_harness.py |
| applets | ts / py | flora-cli, hivebeat |
| skills | markdown | code-review, rust-build |
| dist | bundled | dist/ayesha-os.exe + config/models |

## backend status

| backend | provider | model | status |
|---------|----------|-------|--------|
| cloud | kilo gateway | kilo-auto/free | active |

legacy local llm fully removed. no daemon needed. all calls go through kilo gateway.

## commands

only exit/quit/q recognized. all meta-commands removed.

## autonomous sleeper

scripts/autonomous_sleeper.py runs 8h self-improvement loop.
start with: python scripts/start_sleeper.py

## removed

- legacy local llm dependency (2026-08-23)
- _hf-ayesha_bot gradio web ui (2026-08-23)
- core/ stale worktree (2026-08-23)
- ayesha-bot-mobile/ expo client (2026-08-23)
- all meta-commands (2026-08-23)
- cloudflare + github providers (2026-08-23)

## legend

- ok = working, - = not ready