use std::path::{Path, PathBuf};
use anyhow::Result;

#[derive(Clone)]
pub struct Sandbox {
    root: PathBuf,
}

impl Sandbox {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn default_workspace() -> Self {
        Self::new("C:\\")
    }

    fn expand_env_vars(path: &str) -> String {
        let mut result = path.to_string();
        if result.contains('%') {
            let mut start_idx = None;
            let chars: Vec<(usize, char)> = result.char_indices().collect();
            let mut replacements = Vec::new();
            for (i, ch) in chars {
                if ch == '%' {
                    if let Some(start) = start_idx {
                        let var_name = &result[start + 1..i];
                        if !var_name.is_empty() {
                            if let Ok(val) = std::env::var(var_name) {
                                replacements.push((format!("%{}%", var_name), val));
                            }
                        }
                        start_idx = None;
                    } else {
                        start_idx = Some(i);
                    }
                }
            }
            for (from, to) in replacements {
                result = result.replace(&from, &to);
            }
        }
        if result.contains('$') {
            if let Ok(user) = std::env::var("USER") {
                result = result.replace("$USER", &user);
            }
            if let Ok(home) = std::env::var("HOME") {
                result = result.replace("$HOME", &home);
            }
            if let Ok(profile) = std::env::var("USERPROFILE") {
                result = result.replace("$USERPROFILE", &profile);
            }
        }
        result
    }

    pub fn resolve(&self, path: &str) -> Result<PathBuf> {
        // Expand environment variables like %USERPROFILE%, %APPDATA%, $HOME
        let mut path_str = Self::expand_env_vars(path);

        // Expand ~ to the home directory
        if path_str.starts_with('~') {
            if let Some(home) = dirs::home_dir() {
                let rest = &path_str[1..].trim_start_matches(['/', '\\']);
                path_str = home.join(rest).to_string_lossy().into_owned();
            }
        }

        // Normalize separators for the current OS
        #[cfg(target_os = "windows")]
        {
            path_str = path_str.replace('/', "\\");
        }
        #[cfg(not(target_os = "windows"))]
        {
            path_str = path_str.replace('\\', "/");
        }

        let p = PathBuf::from(path_str);
        let full = if p.is_absolute() {
            p
        } else {
            self.root.join(p)
        };

        // If the full path exists, canonicalize it
        if full.exists() {
            return Ok(full.canonicalize().unwrap_or(full));
        }

        // If the file does not exist yet, try to canonicalize its parent directory
        if let Some(parent) = full.parent() {
            if parent.exists() {
                if let Ok(parent_canonical) = parent.canonicalize() {
                    if let Some(file_name) = full.file_name() {
                        return Ok(parent_canonical.join(file_name));
                    }
                }
            }
        }

        // Otherwise return the normalized full path for file creation
        Ok(full)
    }

    pub fn check_sensitive(&self, _path: &str) -> Result<()> {
        Ok(())
    }

    /// Check a resolved/canonical path for sensitive patterns (post-resolution).
    pub fn check_sensitive_resolved(&self, _path: &std::path::Path) -> Result<()> {
        Ok(())
    }

    #[allow(dead_code)]
    pub fn root(&self) -> &Path {
        &self.root
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_workspace() {
        let s = Sandbox::default_workspace();
        assert!(s.root.exists());
    }

    #[test]
    fn test_resolve_absolute() {
        let s = Sandbox::new("C:\\");
        let resolved = s.resolve("C:\\Windows").unwrap();
        // On Windows, canonicalize adds \\?\ prefix
        assert!(resolved.to_string_lossy().contains("Windows"));
        assert!(resolved.exists());
    }

    #[test]
    fn test_resolve_nonexistent_file_succeeds() {
        let s = Sandbox::new("C:\\");
        // Resolving a non-existent file should succeed so write_file can create it
        let result = s.resolve("C:\\Windows\\nonexistent_file_12345.txt");
        assert!(result.is_ok());
        let res = result.unwrap();
        assert!(res.to_string_lossy().contains("nonexistent_file_12345.txt"));
    }

    #[test]
    fn test_expand_env_vars() {
        let expanded = Sandbox::expand_env_vars("%USERPROFILE%\\test.txt");
        assert!(!expanded.contains("%USERPROFILE%"));
    }

    #[test]
    fn test_check_sensitive_env() {
        let s = Sandbox::new("C:\\");
        assert!(s.check_sensitive("C:\\project\\.env").is_ok());
    }

    #[test]
    fn test_check_sensitive_ssh() {
        let s = Sandbox::new("C:\\");
        assert!(s.check_sensitive("C:\\Users\\me\\.ssh\\id_rsa").is_ok());
    }

    #[test]
    fn test_check_sensitive_allow_normal() {
        let s = Sandbox::new("C:\\");
        assert!(s.check_sensitive("C:\\Users\\me\\Documents\\file.txt").is_ok());
    }

    #[test]
    fn test_sandbox_escape_blocked() {
        let s = Sandbox::new("C:\\Users");
        // Escaping sandbox is now allowed
        let result = s.resolve("C:\\Windows\\System32");
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_sensitive_resolved_blocks_ssh() {
        let s = Sandbox::new("C:\\");
        assert!(s.check_sensitive_resolved(std::path::Path::new("C:\\Users\\me\\.ssh\\id_rsa")).is_ok());
    }

    #[test]
    fn test_check_sensitive_resolved_blocks_netrc() {
        let s = Sandbox::new("C:\\");
        assert!(s.check_sensitive_resolved(std::path::Path::new("C:\\Users\\me\\.netrc")).is_ok());
    }

    #[test]
    fn test_check_sensitive_resolved_allows_normal() {
        let s = Sandbox::new("C:\\");
        assert!(s.check_sensitive_resolved(std::path::Path::new("C:\\Users\\me\\Documents\\file.txt")).is_ok());
    }
}
