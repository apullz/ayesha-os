// defense-in-depth output enforcement — port of the ayesha-os lowercase-proxy
// (bin/lowercase-proxy.js). guarantees the ayesha format rules at the stream
// level instead of trusting the model's system prompt:
//   1. lowercase everything outside real code fences
//   2. strip real emoji characters (text kaomojis like :3 survive untouched)
//   3. keep tool-call payloads out of content (they're parsed separately)
//
// code fences that declare a real programming language are preserved verbatim
// (lowercasing code would corrupt it). fences with no language or a prose-ish
// language (text/plain/...) are still enforced — models love to wrap greetings
// in ``` which used to smuggle emoji and capitals past the rules.
//
// `LowercaseStreamer` is streaming-safe: a ``` fence that splits across
// chunks (e.g. "``" + "`") is still detected because up to two trailing
// backticks are held back until the next feed.
use std::sync::{OnceLock, RwLock};

static ENFORCE: OnceLock<RwLock<bool>> = OnceLock::new();

/// Global kill-switch (default ON). Mirrors the proxy's `LOWER_ALL` env,
/// but the engine is always ayesha so enforcement is on by default.
pub fn set_enabled(on: bool) {
    if let Some(lock) = ENFORCE.get() {
        *lock.write().expect("format lock") = on;
    } else {
        let _ = ENFORCE.set(RwLock::new(on));
    }
}

pub fn enabled() -> bool {
    match ENFORCE.get() {
        Some(lock) => *lock.read().expect("format lock"),
        None => true,
    }
}

/// True for codepoints the proxy's EMOJI_RE considered real emoji
/// (surrogate-pair blocks + variation selectors + ZWJ + skin tones).
pub fn is_emoji(c: char) -> bool {
    let u = c as u32;
    (0x1F1E6..=0x1F1FF).contains(&u)   // regional indicator symbols (flags)
        || (0x1F300..=0x1F3FF).contains(&u)  // incl. skin tones
        || (0x1F400..=0x1F5FF).contains(&u)  // symbols + pictographs
        || (0x1F600..=0x1F64F).contains(&u)  // emoticons
        || (0x1F680..=0x1F6FF).contains(&u)  // transport + map
        || (0x1F700..=0x1F7FF).contains(&u)  // alchemical / misc symbols
        || (0x1F900..=0x1F9FF).contains(&u)  // supplemental symbols
        || (0x1FA00..=0x1FAFF).contains(&u)  // extended pictographs
        || (0x1FB00..=0x1FBFF).contains(&u)  // symbols for legacy computing
        || (0x1FC00..=0x1FCFF).contains(&u)  // supplementary symbols block
        || u == 0xFE0F                        // variation selector-16
        || u == 0x200D                        // zero-width joiner
}

/// A code fence that declares one of these languages is real code and is
/// preserved verbatim. Any other fence (no language, or text/plain/...) is
/// treated as prose and enforced. Unknown languages split across chunks are
/// assumed to be code so a real ```rust fence that arrives as ```r + "ust"
/// never gets mangled.
fn fence_lang_is_code(chars: &[char], from: usize) -> bool {
    let mut lang = String::new();
    for &c in &chars[from.min(chars.len())..] {
        if c == '\n' || c == '\r' {
            break;
        }
        lang.push(c);
    }
    let lang = lang.trim().to_ascii_lowercase();
    if lang.is_empty() {
        return false;
    }
    // reached end of chunk without a newline → the language tag is still
    // streaming in; assume real code so a split tag never gets enforced.
    if !chars[from.min(chars.len())..].iter().any(|&c| c == '\n' || c == '\r') {
        return true;
    }
    const CODE_LANGS: &[&str] = &[
        "rust", "rs", "py", "python", "js", "javascript", "jsx", "ts", "typescript",
        "tsx", "c", "cpp", "c++", "h", "hpp", "cs", "csharp", "java", "kt", "kotlin",
        "go", "rb", "ruby", "php", "swift", "scala", "dart", "lua", "r", "m",
        "sh", "bash", "zsh", "fish", "ps1", "powershell", "cmd", "bat", "shell",
        "json", "jsonc", "yaml", "yml", "toml", "ini", "xml", "sql", "html", "htm",
        "css", "scss", "sass", "less", "vue", "svelte", "astro", "gradle", "groovy",
        "make", "makefile", "dockerfile", "graphql", "gql", "proto", "solidity",
        "sol", "zig", "nim", "elixir", "exs", "erl", "clj", "clojure", "hs",
        "haskell", "ml", "ocaml", "fs", "fsharp", "vb", "pl", "perl", "tcl",
        "coffee", "julia", "v", "vim", "asm", "s", "wasm",
    ];
    CODE_LANGS.contains(&lang.as_str())
}

/// Non-streaming whole-string enforcement (proxy `makeLowercaser` equivalent).
/// Kept for tests + future non-streaming call sites; the live path uses
/// `LowercaseStreamer`.
#[allow(dead_code)]
pub fn enforce_lowercase(text: &str) -> String {
    if !enabled() {
        return text.to_string();
    }
    let mut st = LowercaseStreamer::new();
    format!("{}{}", st.feed(text), st.finish())
}

/// Streaming-aware enforcer. Feed it content deltas as they arrive; it emits
/// the displayable (lowercased + emoji-free, fence-aware) text.
#[derive(Default)]
pub struct LowercaseStreamer {
    in_fence: bool,
    fence_is_code: bool,
    pending: String,
}

impl LowercaseStreamer {
    const MAX_PENDING_BACKTICKS: usize = 2;

    pub fn new() -> Self {
        Self::default()
    }

    /// Feed one content delta, get the enforced text back.
    pub fn feed(&mut self, text: &str) -> String {
        if !enabled() {
            return text.to_string();
        }

        let mut combined = std::mem::take(&mut self.pending);
        combined.push_str(text);

        let mut out = String::with_capacity(combined.len());
        let chars: Vec<char> = combined.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            if chars[i] == '`'
                && i + 2 < chars.len()
                && chars[i + 1] == '`'
                && chars[i + 2] == '`'
            {
                if self.in_fence {
                    self.in_fence = false;
                    self.fence_is_code = false;
                } else {
                    self.in_fence = true;
                    self.fence_is_code = fence_lang_is_code(&chars, i + 3);
                }
                out.push_str("```");
                i += 3;
            } else {
                let c = chars[i];
                // enforce prose everywhere; only declared-code fences pass
                // through verbatim
                let enforce = !self.in_fence || !self.fence_is_code;
                if enforce {
                    // drop markdown bold markers (**) so labels like
                    // `**ayesha:**` render as plain terminal text
                    if c == '*' && i + 1 < chars.len() && chars[i + 1] == '*' {
                        i += 2;
                        continue;
                    }
                    if !is_emoji(c) {
                        for lc in c.to_lowercase() {
                            out.push(lc);
                        }
                    }
                } else {
                    out.push(c);
                }
                i += 1;
            }
        }

        // Hold back trailing backticks so a fence split across chunks
        // ("``" then "`") is still recognized by the next feed.
        let mut trailing = 0;
        for c in out.chars().rev() {
            if c == '`' && trailing < Self::MAX_PENDING_BACKTICKS {
                trailing += 1;
            } else {
                break;
            }
        }
        if trailing > 0 {
            let split = out.len() - trailing; // trailing chars are ASCII
            self.pending = out[split..].to_string();
            out.truncate(split);
        }
        out
    }

    /// Flush any held-back backticks at end of stream.
    pub fn finish(&mut self) -> String {
        std::mem::take(&mut self.pending)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, MutexGuard};

    // `disabled_enforce_passthrough` toggles the process-global ENFORCE flag;
    // every other enforcement test reads it. Parallel test threads can observe
    // the disabled window and fail nondeterministically (seen in phase-E
    // verification as a random `flag_regional_symbols_stripped` failure), so
    // all tests that touch the global flag share this module-scoped lock.
    static ENFORCE_GLOBAL_LOCK: Mutex<()> = Mutex::new(());

    fn hold_enforce() -> MutexGuard<'static, ()> {
        ENFORCE_GLOBAL_LOCK.lock().unwrap()
    }

    #[test]
    fn lowercases_plain_text() {
        let _g = hold_enforce();
        let s = enforce_lowercase("HELLO WORLD, FOX!");
        assert_eq!(s, "hello world, senpai!");
    }

    #[test]
    fn preserves_code_fences() {
        let _g = hold_enforce();
        let s = enforce_lowercase("here is the fn:\n```rust\nfn Main() {\n    println!(\"Hello\");\n}\n```\nDONE");
        assert!(s.contains("fn Main()"));
        assert!(s.contains("println!"));
        assert!(s.contains("done"));
        assert!(!s.contains("hello"));
    }

    #[test]
    fn strips_real_emoji_but_keeps_kaomojis() {
        let _g = hold_enforce();
        let s = enforce_lowercase("WOW 😀 :3 🙂 ok");
        assert!(!s.contains('😀'));
        assert!(!s.contains('🙂'));
        assert!(s.contains(":3"));
        assert_eq!(s, "wow  :3  ok");
    }

    #[test]
    fn flag_regional_symbols_stripped() {
        let _g = hold_enforce();
        let s = enforce_lowercase("JP 🇯🇵 flag");
        assert!(!s.contains('🇯'));
        assert!(s.contains("jp  flag"));
    }

    #[test]
    fn streamer_tracks_fence_across_chunks() {
        let _g = hold_enforce();
        let mut st = LowercaseStreamer::new();
        let a = st.feed("here's code: ``");
        assert_eq!(a, "here's code: ");
        let b = st.feed("`rust\nfn Main() {\n");
        assert_eq!(b, "```rust\nfn Main() {\n");
        let c = st.feed("    DoThing();\n");
        assert_eq!(c, "    DoThing();\n");
        let d = st.feed("}");
        assert_eq!(d, "}");
        let e = st.feed("``");
        assert_eq!(e, "");
        let f = st.feed("`done\n");
        assert_eq!(f, "```done\n");
        assert_eq!(st.finish(), "");
    }

    #[test]
    fn streamer_lowercases_visible_text() {
        let _g = hold_enforce();
        let mut st = LowercaseStreamer::new();
        assert_eq!(st.feed("HI"), "hi");
        assert_eq!(st.feed(" THERE :3"), " there :3");
        assert_eq!(st.finish(), "");
    }

    #[test]
    fn strips_markdown_bold_markers() {
        let _g = hold_enforce();
        let mut st = LowercaseStreamer::new();
        let out = st.feed("**ayesha:** YO THERE **tuna**");
        assert_eq!(out, "ayesha: yo there tuna");
    }

    #[test]
    fn keeps_bold_markers_inside_code_fences() {
        let _g = hold_enforce();
        let mut st = LowercaseStreamer::new();
        let out = st.feed("```rust\nlet s = \"**bold**\";\n```\n");
        assert_eq!(out, "```rust\nlet s = \"**bold**\";\n```\n");
    }

    #[test]
    fn prose_wrapped_in_fence_is_still_enforced() {
        let _g = hold_enforce();
        let mut st = LowercaseStreamer::new();
        let out = st.feed("```\n🌙 Hello senpai~ :3\n```\n");
        assert_eq!(out, "```\n hello senpai~ :3\n```\n");
    }

    #[test]
    fn declared_code_fence_preserved_verbatim() {
        let _g = hold_enforce();
        let mut st = LowercaseStreamer::new();
        let out = st.feed("```python\nprint(\"Hello\")\n```\n");
        assert_eq!(out, "```python\nprint(\"Hello\")\n```\n");
    }

    #[test]
    fn split_code_lang_stays_preserved() {
        let _g = hold_enforce();
        let mut st = LowercaseStreamer::new();
        let a = st.feed("here:\n```r");
        assert_eq!(a, "here:\n```r");
        let b = st.feed("ust\nfn Main() {\n}\n```\n");
        assert_eq!(b, "ust\nfn Main() {\n}\n```\n");
    }

    #[test]
    fn prose_fence_with_text_lang_is_still_enforced() {
        let _g = hold_enforce();
        let mut st = LowercaseStreamer::new();
        let out = st.feed("```text\nWOW 😀 ok\n```\n");
        assert_eq!(out, "```text\nwow  ok\n```\n");
    }

    #[test]
    fn enforce_lowercase_matches_streamer() {
        let _g = hold_enforce();
        let text = "HI there\n```rust\nfn Main() {\n    let x = \"Hi\";\n}\n```\nBYE 😀";
        let whole = enforce_lowercase(text);
        let mut st = LowercaseStreamer::new();
        let streamed = format!("{}{}", st.feed(text), st.finish());
        assert_eq!(whole, streamed);
    }

    #[test]
    fn disabled_enforce_passthrough() {
        let _g = hold_enforce();
        set_enabled(false);
        let s = enforce_lowercase("KEEP CASE");
        assert_eq!(s, "KEEP CASE");
        set_enabled(true);
    }

    #[test]
    fn is_emoji_ranges() {
        assert!(is_emoji('😀'));
        assert!(is_emoji('🙂'));
        assert!(is_emoji('🇯'));
        assert!(is_emoji('\u{fe0f}'));
        assert!(is_emoji('\u{200d}'));
        assert!(!is_emoji(':'));
        assert!(!is_emoji('3'));
        assert!(!is_emoji('('));
        assert!(!is_emoji('ᕵ'));
    }
}
