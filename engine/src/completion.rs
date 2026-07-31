/// Tab completion engine — pure, testable prefix-matching + cycle state.

pub struct Completer {
    candidates: Vec<String>,
    last_prefix: String,
    cycle_idx: usize,
    matches: Vec<String>,
}

impl Completer {
    pub fn new(mut candidates: Vec<String>) -> Self {
        candidates.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
        candidates.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
        Self {
            candidates,
            last_prefix: String::new(),
            cycle_idx: 0,
            matches: Vec::new(),
        }
    }

    /// Update the candidate list (e.g. after applet names change).
    pub fn set_candidates(&mut self, mut candidates: Vec<String>) {
        candidates.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
        candidates.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
        self.candidates = candidates;
        self.reset();
    }

    pub fn reset(&mut self) {
        self.last_prefix.clear();
        self.cycle_idx = 0;
        self.matches.clear();
    }

    /// Tab pressed: returns (completed_prefix, show_all_matches).
    /// If `prefix` changed since last Tab, resets cycle. Otherwise advances.
    pub fn complete(&mut self, prefix: &str) -> (Option<String>, Vec<String>) {
        let lower = prefix.to_lowercase();

        if lower != self.last_prefix {
            self.last_prefix = lower.clone();
            self.cycle_idx = 0;
            self.matches = self.candidates.iter()
                .filter(|c| c.to_lowercase().starts_with(&lower))
                .cloned()
                .collect();
        } else {
            self.cycle_idx += 1;
        }

        if self.matches.is_empty() {
            return (None, vec![]);
        }

        if self.matches.len() == 1 {
            return (Some(self.matches[0].clone()), vec![]);
        }

        // Cycle through matches
        let idx = self.cycle_idx % self.matches.len();
        let selected = self.matches[idx].clone();

        // Double-tab: show all matches
        let show_all = self.cycle_idx > 0 && self.cycle_idx % self.matches.len() == 0;

        (Some(selected), if show_all { self.matches.clone() } else { vec![] })
    }

    /// Compute common prefix of all matches (for single-Tab expansion).
    pub fn common_prefix(matches: &[String]) -> Option<String> {
        if matches.is_empty() { return None; }
        let first = &matches[0];
        let mut end = first.len();
        for m in &matches[1..] {
            end = end.min(m.len());
            for (i, (a, b)) in first[..end].chars().zip(m[..end].chars()).enumerate() {
                if a.to_ascii_lowercase() != b.to_ascii_lowercase() {
                    end = i;
                    break;
                }
            }
        }
        Some(first[..end].to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn comp(candidates: &[&str]) -> Completer {
        Completer::new(candidates.iter().map(|s| s.to_string()).collect())
    }

    #[test]
    fn first_tab_selects_first_match() {
        let mut c = comp(&["hi", "help", "hello"]);
        let (sel, _) = c.complete("h");
        assert_eq!(sel.as_deref(), Some("hello"));
    }

    #[test]
    fn second_tab_cycles() {
        let mut c = comp(&["hi", "help", "hello"]);
        let (s1, _) = c.complete("h");
        let (s2, _) = c.complete("h");
        let (s3, _) = c.complete("h");
        let (s4, _) = c.complete("h");
        assert_eq!(s1, Some("hello".to_string()));
        assert_eq!(s2, Some("help".to_string()));
        assert_eq!(s3, Some("hi".to_string()));
        assert_eq!(s4, Some("hello".to_string())); // wraps after 3
    }

    #[test]
    fn no_match_returns_none() {
        let mut c = comp(&["help", "hello"]);
        let (sel, _) = c.complete("xyz");
        assert!(sel.is_none());
    }

    #[test]
    fn single_match_resets() {
        let mut c = comp(&["help"]);
        let (s1, _) = c.complete("h");
        let (s2, _) = c.complete("h");
        assert_eq!(s1, s2); // single match stays same
    }

    #[test]
    fn case_insensitive() {
        let mut c = comp(&["Help", "HELLO"]);
        let (sel, _) = c.complete("HELP");
        assert!(sel.is_some());
    }

    #[test]
    fn prefix_change_resets_cycle() {
        let mut c = comp(&["help", "hello", "hi"]);
        let (s1, _) = c.complete("hel");
        let (s2, _) = c.complete("h"); // prefix changed, resets
        assert!(s1.is_some());
        assert!(s2.is_some());
        // s1 should be "help" or "hello", s2 starts fresh cycle
    }

    #[test]
    fn common_prefix_basic() {
        let matches = vec!["hello".into(), "help".into(), "hero".into()];
        assert_eq!(Completer::common_prefix(&matches).as_deref(), Some("he"));
    }

    #[test]
    fn common_prefix_single() {
        let matches = vec!["hello".into()];
        assert_eq!(Completer::common_prefix(&matches).as_deref(), Some("hello"));
    }

    #[test]
    fn common_prefix_empty() {
        assert!(Completer::common_prefix(&[]).is_none());
    }

    #[test]
    fn applet_name_completion() {
        let mut c = comp(&["flora-cli", "desktop-cat", "poopy-tui"]);
        let (sel, _) = c.complete("fl");
        assert_eq!(sel.as_deref(), Some("flora-cli"));
    }

    #[test]
    fn slash_commands() {
        let mut c = comp(&["help", "clear", "models", "apps", "run", "stop"]);
        let (sel, _) = c.complete("/");
        // All start with nothing — slash isn't in candidates, but "help" etc don't start with "/"
        assert!(sel.is_none());
    }

    #[test]
    fn slash_stripped_before_complete() {
        // The caller strips "/" before calling complete. So "help" matches "h".
        let mut c = comp(&["help", "clear", "models"]);
        let (sel, _) = c.complete("h");
        assert_eq!(sel.as_deref(), Some("help"));
    }

    #[test]
    fn reset_clears_state() {
        let mut c = comp(&["hi", "help", "hello"]);
        let _ = c.complete("h");
        c.reset();
        let (sel, _) = c.complete("h");
        assert_eq!(sel.as_deref(), Some("hello")); // fresh start
    }
}
