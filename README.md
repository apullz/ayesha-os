# ayesha-mini

minimal irc-style rust tui — chat + model picker + memory only.

## run

```sh
cargo run
```

requires ollama at `http://localhost:11434` (`OLLAMA_HOST` override).
default model `qwen2.5:7b` (`AYESHA_MODEL` override). pull first:

```sh
ollama pull qwen2.5:7b
```

## slash cmds

- `/ls <path>` · `/read <path>` · `/write <path> <text>` then `yes`
- `/memory` · `/remember <fact>` · `/model`
- `/help` · `/clear` · `/bye` (`/quit`, `/q`)
