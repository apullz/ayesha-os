// defense-in-depth output enforcement — port of the opencode lowercase-proxy
// (bin/lowercase-proxy.js). guarantees the ayesha format rules at the stream
// level instead of trusting the model's system prompt:
//   1. lowercase everything outside ```code fences```
//   2. strip real emoji characters (text kaomojis like :3 survive untouched)
//   3. keep tool-call payloads out of content (they're parsed separately)
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

/// Non-streaming whole-string enforcement (proxy `makeLowercaser` equivalent).
/// Kept for tests + future non-streaming call sites; the live path uses
/// `LowercaseStreamer`.
#[allow(dead_code)]
pub fn enforce_lowercase(text: &str) -> String {
    if !enabled() {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let parts: Vec<&str> = text.split("```").collect();
    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            out.push_str("```");
        }
        if i % 2 == 1 {
            out.push_str(part);
        } else {
            for c in part.chars() {
                if is_emoji(c) {
                    continue;
                }
                for lc in c.to_lowercase() {
                    out.push(lc);
                }
            }
        }
    }
    out
}

/// Streaming-aware enforcer. Feed it content deltas as they arrive; it emits
/// the displayable (lowercased + emoji-free, fence-preserving) text.
#[derive(Default)]
pub struct LowercaseStreamer {
    in_fence: bool,
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
                self.in_fence = !self.in_fence;
                out.push_str("```");
                i += 3;
            } else {
                let c = chars[i];
                if !self.in_fence {
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

    #[test]
    fn lowercases_plain_text() {
        let s = enforce_lowercase("HELLO WORLD, FOX!");
        assert_eq!(s, "hello world, fox!");
    }

    #[test]
    fn preserves_code_fences() {
        let s = enforce_lowercase("here is the fn:\n```rust\nfn Main() {\n    println!(\"Hello\");\n}\n```\nDONE");
        assert!(s.contains("fn Main()"));
        assert!(s.contains("println!"));
        assert!(s.contains("done"));
        assert!(!s.contains("hello"));
    }

    #[test]
    fn strips_real_emoji_but_keeps_kaomojis() {
        let s = enforce_lowercase("WOW 😀 :3 🙂 ok");
        assert!(!s.contains('😀'));
        assert!(!s.contains('🙂'));
        assert!(s.contains(":3"));
        assert_eq!(s, "wow  :3  ok");
    }

    #[test]
    fn flag_regional_symbols_stripped() {
        let s = enforce_lowercase("JP 🇯🇵 flag");
        assert!(!s.contains('🇯'));
        assert!(s.contains("jp  flag"));
    }

    #[test]
    fn streamer_tracks_fence_across_chunks() {
        let mut st = LowercaseStreamer::new();
        let a = st.feed("here's code: ``");
        assert_eq!(a, "here's code: ");
        let b = st.feed("`fn Main() {\n");
        assert_eq!(b, "```fn Main() {\n");
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
        let mut st = LowercaseStreamer::new();
        assert_eq!(st.feed("HI"), "hi");
        assert_eq!(st.feed(" THERE :3"), " there :3");
        assert_eq!(st.finish(), "");
    }

    #[test]
    fn disabled_enforce_passthrough() {
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
