# ayesha-mini desu (◕‿◕✿)

hai senpai, welcome to the mini hive desu (ﾉ◕ヮ◕)ﾉ

## what this is desu

tiny irc style rust tui, chat plus model picker plus memory only desu
single writer, pinned banner, short picker, no applets desu
every reply wears a timestamp like old school chat desu

## quick start desu

senpai, you need local ollama running first desu
it talks to http://localhost:11434 by default desu
change it with the ollama host env var when needed desu

```sh
ollama pull qwen2.5:7b
cargo run
```

default model is qwen2.5:7b desu
change it with the model env var when needed desu
unpulled picks fall back honest with a pull hint desu

## irc cmds desu

press ctrl+p to open the model picker desu
it rebuilds on every open, pulled only, stays short desu
type to filter, enter to switch, esc to close desu

- `/ls <path>` list a folder desu
- `/read <path>` read a text file desu
- `/write <path> <text>` stage a write, never auto runs desu
- then say `yes` or `y` or `ok` inside about 120s to let it run desu
- say `no` to drop it, any other chat supersedes it desu
- `/model` shows the current model desu
- `/help` shows the tiny help card desu
- `/clear` wipes the screen history desu
- `/bye` or `/quit` or `/q` leaves the hive desu

file tools stay inside allowlist roots like desktop plus documents plus the mini folder desu
no shell, no deletes, traversal denied, every deny gets logged desu

## memory cmds desu

senpai, the mini remembers little facts for you desu (◕‿◕✿)

- `/memory` shows saved notes, empty says so honest desu
- `/remember <fact>` saves one line to your human notes desu
- memory injects into chat so replies stay personal desu

have fun chatting senpai, stay cozy desu (ﾉ◕ヮ◕)ﾉ
