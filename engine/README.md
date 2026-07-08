# ayesha-engine

terminal-native persona host with tool-calling for ollama.

## features

- connects to local ollama model `ayesha`
- tool-calling agent loop (read/write/list files)
- generates interactive HTML apps from prompts
- sandboxed file access (restricted to workspace)
- colored terminal output with ayesha's personality

## requirements

- [rust](https://rustup.rs/) (installed)
- [ollama](https://ollama.ai/) running on localhost:11434
- `ayesha` model pulled (`ollama pull ayesha`)

## build

```bash
cargo build --release
```

## run

```bash
# via batch file
run.bat

# or directly
target\release\ayesha-engine.exe
```

## commands

| command | description |
|---------|-------------|
| `help` | show available commands |
| `clear` | clear the screen |
| `model` | show current model |
| `model <name>` | switch to a different model |
| `exit` | quit the application |

## tools

ayesha can use these tools when you ask her to:

- `read_file(path)` - read a file's contents
- `write_file(path, content)` - write content to a file
- `list_dir(path)` - list directory contents
- `generate_html(prompt, path)` - generate an interactive HTML app

## examples

```
fox > read the file src/main.rs
fox > list files in the current directory
fox > create a todo list app and save it to output/todo.html
fox > what's in the config folder?
```

## architecture

```
main.rs      - entry point + agent loop
ollama.rs    - ollama API client + tool definitions
tools.rs     - tool execution (file I/O, HTML generation)
sandbox.rs   - path sandboxing/security
ui.rs        - terminal UI (colors, prompts, banner)
```

## sandbox

all file operations are restricted to `~/Documents/workspace`.
sensitive files (.env, .ssh, etc.) are blocked.

## personality

ayesha is a 33-year-old from japan with a fusion of hatsune miku's sparkle and a tachikoma's spider-like curiosity. she uses:

- lower-case text exclusively
- internet slang from the 1990s-2010s
- kaomojis (:3, >w<, ^_^, etc.)
- references to 'apullz' or 'fox'
