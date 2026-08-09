use std::fs;
use std::path::{Path, PathBuf};

/// A discoverable skill: a markdown file in `<root>/skills/` with YAML-style
/// frontmatter (`name`, `description`) followed by the skill's instructions.
///
/// ```markdown
/// ---
/// name: my-skill
/// description: Use when the user asks about X
/// ---
/// # instructions
/// follow these steps...
/// ```
#[derive(Debug, Clone)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub content: String,
    pub path: PathBuf,
}

/// Scan `<root>/skills/*.md` for skill definitions.
/// Files without valid frontmatter are skipped.
pub fn discover(root: &Path) -> Vec<Skill> {
    let skills_dir = root.join("skills");
    let entries = match fs::read_dir(&skills_dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    let mut skills: Vec<Skill> = entries
        .flatten()
        .filter(|e| {
            let path = e.path();
            let ext = path.extension().and_then(|x| x.to_str()).unwrap_or("");
            path.is_file() && ext.eq_ignore_ascii_case("md")
        })
        .filter_map(|e| parse_skill(&e.path()).ok())
        .collect();

    skills.sort_by(|a, b| a.name.cmp(&b.name));
    skills
}

fn parse_skill(path: &Path) -> anyhow::Result<Skill> {
    let raw = fs::read_to_string(path)?;
    let content = raw.trim_start_matches('\u{feff}').trim_start_matches('\r');
    let (content, frontmatter) = split_frontmatter(content)?;

    let name = frontmatter
        .lines()
        .find_map(|l| kv(l, "name"))
        .map(|s| s.trim().to_lowercase().replace(' ', "-"))
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow::anyhow!("skill missing 'name' frontmatter"))?;

    let description = frontmatter
        .lines()
        .find_map(|l| kv(l, "description"))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "no description".to_string());

    Ok(Skill {
        name,
        description,
        content: content.to_string(),
        path: path.to_path_buf(),
    })
}

/// Split `---\n...\n---\nrest` into (rest, frontmatter).
fn split_frontmatter(content: &str) -> anyhow::Result<(&str, &str)> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return Err(anyhow::anyhow!("no frontmatter fence"));
    }
    let rest = &trimmed[3..];
    let end = rest
        .find("\n---")
        .ok_or_else(|| anyhow::anyhow!("unterminated frontmatter"))?;
    let front = &rest[..end];
    let body = &rest[end + 4..];
    Ok((body, front))
}

/// Extract a `key: value` line, ignoring comments and blank lines.
fn kv(line: &str, key: &str) -> Option<String> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let (k, v) = line.split_once(':')?;
    if k.trim().eq_ignore_ascii_case(key) {
        Some(v.trim().to_string())
    } else {
        None
    }
}

/// Build the skills hint block injected into the system prompt.
pub fn system_prompt_hint(root: &Path) -> Option<String> {
    let skills = discover(root);
    if skills.is_empty() {
        return None;
    }
    let mut out = String::from(
        "\n\nskills (markdown guides in the skills/ folder). if the user's request matches a skill's description, you MUST call the read_skill tool to load its instructions and follow them exactly:\n",
    );
    for s in &skills {
        out.push_str(&format!("- {}: {}\n", s.name, s.description));
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    fn tmpdir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ayesha-skills-{}-{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_skill(dir: &Path, filename: &str, body: &str) {
        let path = dir.join(filename);
        let mut f = File::create(&path).unwrap();
        f.write_all(body.as_bytes()).unwrap();
    }

    #[test]
    fn discovers_skills_with_frontmatter() {
        let dir = tmpdir("discover");
        fs::create_dir_all(dir.join("skills")).unwrap();
        write_skill(
            &dir.join("skills"),
            "code-review.md",
            "---\nname: code-review\ndescription: Use when the user asks to review code\n---\n# code review checklist\n1. check for bugs",
        );
        write_skill(&dir.join("skills"), "notes.md", "not a skill (no frontmatter)");

        let skills = discover(&dir);
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "code-review");
        assert_eq!(skills[0].description, "Use when the user asks to review code");
        assert!(skills[0].content.contains("checklist"));
    }

    #[test]
    fn frontmatter_split_handles_empty_body() {
        let body = "---\nname: empty\ndescription: x\n---\n";
        let skill = parse_skill_text("empty-body", body);
        assert!(skill.is_some());
    }

    fn parse_skill_text(tag: &str, raw: &str) -> Option<Skill> {
        let dir = tmpdir(&format!("parse-{}", tag));
        fs::create_dir_all(&dir).unwrap();
        write_skill(&dir, "s.md", raw);
        let path = dir.join("s.md");
        parse_skill(&path).ok()
    }

    #[test]
    fn skill_names_are_lowercased_and_dashed() {
        let body = "---\nname: My Skill\ndescription: desc\n---\ncontent";
        let skill = parse_skill_text("dashed", body).unwrap();
        assert_eq!(skill.name, "my-skill");
    }

    #[test]
    fn missing_required_fields_are_rejected() {
        let body = "---\ndescription: no name here\n---\ncontent";
        assert!(parse_skill_text("missing", body).is_none());
    }
}
