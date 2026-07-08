use std::fs;
use std::path::PathBuf;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct CodeAnalysis {
    pub file: String,
    pub issues: Vec<Issue>,
    pub suggestions: Vec<String>,
    pub complexity_score: u8,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Issue {
    pub severity: String,
    pub line: Option<usize>,
    pub description: String,
    pub fix: String,
}

pub struct SelfAnalyzer {
    project_root: PathBuf,
}

impl SelfAnalyzer {
    pub fn new(project_root: PathBuf) -> Self {
        Self { project_root }
    }

    pub fn source_files(&self) -> Vec<PathBuf> {
        let src_dir = self.project_root.join("src");
        let mut files = Vec::new();
        if let Ok(entries) = fs::read_dir(&src_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "rs").unwrap_or(false) {
                    files.push(path);
                }
            }
        }
        // Also check subdirectories
        if let Ok(entries) = fs::read_dir(&src_dir) {
            for entry in entries.flatten() {
                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    let sub = entry.path();
                    if let Ok(sub_entries) = fs::read_dir(&sub) {
                        for sub_entry in sub_entries.flatten() {
                            let path = sub_entry.path();
                            if path.extension().map(|e| e == "rs").unwrap_or(false) {
                                files.push(path);
                            }
                        }
                    }
                }
            }
        }
        files
    }

    pub fn read_source(&self, file: &PathBuf) -> Result<String> {
        Ok(fs::read_to_string(file)?)
    }

    pub fn full_source_dump(&self) -> Result<String> {
        let files = self.source_files();
        let mut dump = String::new();
        for file in &files {
            let relative = file.strip_prefix(&self.project_root)
                .unwrap_or(file)
                .to_string_lossy();
            let content = self.read_source(file)?;
            dump.push_str(&format!("=== {} ({} lines) ===\n", relative, content.lines().count()));
            dump.push_str(&content);
            dump.push_str("\n\n");
        }
        Ok(dump)
    }

    pub fn analyze_for_improvements(&self, source: &str) -> Vec<Issue> {
        let mut issues = Vec::new();

        for (i, line) in source.lines().enumerate() {
            let trimmed = line.trim();

            // Unused imports
            if trimmed.starts_with("use ") && trimmed.contains(';') && !trimmed.contains("::") {
                issues.push(Issue {
                    severity: "info".to_string(),
                    line: Some(i + 1),
                    description: format!("potentially unused import: {}", trimmed),
                    fix: "remove if not used".to_string(),
                });
            }

            // Unwrap usage
            if trimmed.contains(".unwrap()") && !trimmed.starts_with("//") {
                issues.push(Issue {
                    severity: "warning".to_string(),
                    line: Some(i + 1),
                    description: "using unwrap() which can panic".to_string(),
                    fix: "use .unwrap_or_default() or proper error handling".to_string(),
                });
            }

            // Long lines
            if line.len() > 120 {
                issues.push(Issue {
                    severity: "style".to_string(),
                    line: Some(i + 1),
                    description: format!("line exceeds 120 chars ({})", line.len()),
                    fix: "break into multiple lines".to_string(),
                });
            }

            // TODO/FIXME
            if trimmed.contains("TODO") || trimmed.contains("FIXME") {
                issues.push(Issue {
                    severity: "info".to_string(),
                    line: Some(i + 1),
                    description: "unresolved TODO/FIXME".to_string(),
                    fix: "address or create issue".to_string(),
                });
            }

            // Empty catch
            if trimmed == "Ok(())" || trimmed == "{}" {
                issues.push(Issue {
                    severity: "info".to_string(),
                    line: Some(i + 1),
                    description: "empty result/block".to_string(),
                    fix: "add comment explaining intentional no-op".to_string(),
                });
            }
        }

        issues
    }

    pub fn generate_improvement_prompt(&self, source: &str, file_name: &str) -> String {
        let issues = self.analyze_for_improvements(source);
        let issue_list: Vec<String> = issues.iter().map(|i| {
            format!("[{}] line {:?}: {} → fix: {}", i.severity, i.line, i.description, i.fix)
        }).collect();

        format!(
            r#"analyze this rust source file and suggest concrete improvements:

file: {}
lines: {}

issues found ({}):
{}

source:
```
{}
```

suggest:
1. performance improvements
2. better error handling
3. code simplification
4. missing functionality
5. security concerns

output a numbered list of specific, actionable improvements. be concise."#,
            file_name,
            source.lines().count(),
            issues.len(),
            issue_list.join("\n"),
            source,
        )
    }
}
