// lightweight syntax highlighting for the code-block renderer.
// dependency-free: scans a single line and emits ANSI-colored segments
// using the active theme's syntax palette (monokai++ tokens by default).
// it is deliberately simple — good enough to make fenced code readable in
// the terminal without pulling in a full highlighting engine.

use crate::theme::{self, SyntaxRole};

const KEYWORDS: &[&str] = &[
    // common across many languages
    "fn", "let", "mut", "const", "static", "pub", "use", "mod", "impl", "trait",
    "struct", "enum", "match", "if", "else", "for", "while", "loop", "return",
    "break", "continue", "true", "false", "None", "Some", "Ok", "Err",
    "def", "class", "import", "from", "as", "with", "try", "except", "finally",
    "raise", "pass", "lambda", "yield", "global", "nonlocal", "async", "await",
    "function", "var", "let", "new", "typeof", "instanceof", "in", "of",
    "export", "default", "extends", "super", "this", "null", "undefined",
    "interface", "type", "namespace", "declare", "abstract", "private",
    "protected", "public", "readonly", "implements", "enum", "never", "unknown",
    "package", "int", "float", "double", "char", "void", "bool", "byte",
    "short", "long", "unsigned", "signed", "case", "switch", "default",
    "do", "sizeof", "typedef", "union", "goto", "volatile", "extern", "inline",
    "print", "echo", "require", "include", "isset", "empty", "unset", "list",
];

fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_' || c == '$' || c == '@'
}

fn is_ident_cont(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Split a token-ish run off the front of `line`. Returns (token, rest).
fn scan_ident(line: &str) -> (String, &str) {
    let mut end = 0;
    for (i, c) in line.char_indices() {
        if i == 0 {
            if is_ident_start(c) {
                end = c.len_utf8();
            } else {
                break;
            }
        } else if is_ident_cont(c) {
            end = i + c.len_utf8();
        } else {
            break;
        }
    }
    if end == 0 {
        (String::new(), line)
    } else {
        (line[..end].to_string(), &line[end..])
    }
}

/// Highlight one line of source code. Returns the ANSI-colored string
/// (no trailing newline).
pub fn highlight_line(line: &str) -> String {
    let mut out = String::with_capacity(line.len() + 32);
    let mut rest = line;

    while !rest.is_empty() {
        let c = rest.chars().next().unwrap();

        // line comment: rest of line in comment color
        if (c == '/' && rest[1..].starts_with('/'))
            || (c == '#' && !rest[1..].starts_with('!'))
            || (c == '-' && rest[1..].starts_with('-'))
        {
            out.push_str(&format!("{}", theme::paint_syntax(SyntaxRole::Comment, rest)));
            break;
        }

        // string literal (single / double / backtick / triple-quote start)
        if c == '"' || c == '\'' || c == '`' {
            if let Some((s, after)) = scan_string(rest, c) {
                out.push_str(&format!("{}", theme::paint_syntax(SyntaxRole::String, &s)));
                rest = after;
                continue;
            }
        }

        // number literal
        if c.is_ascii_digit() {
            let mut end = c.len_utf8();
            while let Some(next) = rest[end..].chars().next() {
                if next.is_ascii_alphanumeric() || next == '.' || next == '_' {
                    end += next.len_utf8();
                } else {
                    break;
                }
            }
            let num = &rest[..end];
            out.push_str(&format!("{}", theme::paint_syntax(SyntaxRole::Number, num)));
            rest = &rest[end..];
            continue;
        }

        // identifiers: keyword / builtin / function / type
        if is_ident_start(c) {
            let (tok, after) = scan_ident(rest);
            let role = classify_ident(&tok, after);
            out.push_str(&format!("{}", theme::paint_syntax(role, &tok)));
            rest = after;
            continue;
        }

        // operator / punctuation
        if "=+-*/%<>!&|^~?:;.,(){}[]".contains(c) {
            let role = if "=+-*/%<>!&|^~?".contains(c) {
                SyntaxRole::Operator
            } else {
                SyntaxRole::Punctuation
            };
            let ch = c.to_string();
            out.push_str(&format!("{}", theme::paint_syntax(role, &ch)));
            rest = &rest[c.len_utf8()..];
            continue;
        }

        out.push(c);
        rest = &rest[c.len_utf8()..];
    }

    if out.is_empty() {
        line.to_string()
    } else {
        out
    }
}

/// Scan a quoted string starting at the given quote char. Handles escaped
/// quotes. Returns (full_string_segment, rest_of_line_after_string).
fn scan_string(line: &str, quote: char) -> Option<(String, &str)> {
    let mut escaped = false;
    let mut bytes = 0usize;
    for c in line[1..].chars() {
        bytes += c.len_utf8();
        if escaped {
            escaped = false;
            continue;
        }
        if c == '\\' {
            escaped = true;
            continue;
        }
        if c == quote {
            let end = 1 + bytes;
            return Some((line[..end].to_string(), &line[end..]));
        }
    }
    None
}

fn classify_ident(tok: &str, after: &str) -> SyntaxRole {
    if KEYWORDS.contains(&tok) {
        return SyntaxRole::Keyword;
    }
    // function call: identifier immediately followed by `(`
    if let Some(next) = after.chars().next() {
        if next == '(' {
            return SyntaxRole::Function;
        }
    }
    // type-ish: starts uppercase (Rust/TS convention)
    if tok.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
        return SyntaxRole::Type;
    }
    SyntaxRole::Keyword
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme;

    fn strip_ansi(s: &str) -> String {
        let mut out = String::new();
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\u{1b}' {
                // consume until 'm'
                while let Some(n) = chars.next() {
                    if n == 'm' {
                        break;
                    }
                }
            } else {
                out.push(c);
            }
        }
        out
    }

    fn set_monokai() {
        theme::set_active(theme::preset("monokai++").unwrap());
    }

    #[test]
    fn keywords_are_colored() {
        set_monokai();
        let out = highlight_line("fn main()");
        let plain = strip_ansi(&out);
        assert_eq!(plain, "fn main()");
        assert!(out.contains("fn"));
    }

    #[test]
    fn strings_and_numbers_colored() {
        set_monokai();
        let out = highlight_line("let x = \"hi\" + 42;");
        assert_eq!(strip_ansi(&out), "let x = \"hi\" + 42;");
        assert!(out.contains("hi"));
        assert!(out.contains("42"));
    }

    #[test]
    fn comment_colors_rest_of_line() {
        set_monokai();
        let out = highlight_line("x = 1 // done");
        assert_eq!(strip_ansi(&out), "x = 1 // done");
    }

    #[test]
    fn function_call_detected() {
        set_monokai();
        let out = highlight_line("println!(\"ok\")");
        assert_eq!(strip_ansi(&out), "println!(\"ok\")");
    }

    #[test]
    fn empty_and_whitespace_lines_pass_through() {
        set_monokai();
        assert_eq!(highlight_line(""), "");
        assert_eq!(highlight_line("   "), "   ");
    }
}
