use std::path::{Path, PathBuf};
use anyhow::{Result, bail};
use dirs::home_dir;

#[derive(Clone)]
pub struct Sandbox {
    root: PathBuf,
}

impl Sandbox {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn default_workspace() -> Self {
        let root = home_dir().unwrap_or_else(|| PathBuf::from("C:\\"));
        Self::new(root)
    }

    pub fn resolve(&self, path: &str) -> Result<PathBuf> {
        let p = Path::new(path);

        let resolved = if p.is_absolute() {
            p.to_path_buf()
        } else {
            self.root.join(p)
        };

        let root_canonical = self.root.canonicalize().unwrap_or_else(|_| self.root.clone());

        let canonical = match resolved.canonicalize() {
            Ok(c) => c,
            Err(_) => {
                if let Some(parent) = resolved.parent() {
                    if parent.exists() {
                        // Only allow if parent canonicalization succeeds — don't trust un-canonical paths
                        match parent.canonicalize() {
                            Ok(parent_canonical) if parent_canonical.starts_with(&root_canonical) => {
                                return Ok(resolved);
                            }
                            _ => {}
                        }
                    }
                }
                bail!("path does not exist: {}", resolved.display());
            }
        };

        if !canonical.starts_with(&root_canonical) {
            bail!(
                "access denied: path '{}' escapes sandbox root '{}'",
                canonical.display(),
                root_canonical.display()
            );
        }

        Ok(canonical)
    }

    pub fn check_sensitive(&self, path: &str) -> Result<()> {
        let lower = path.to_lowercase();
        let blocked = [
            ".env", ".ssh", ".gnupg", ".aws", ".azure",
            "password", "secret", "token", "private_key",
            "id_rsa", "known_hosts", "kubeconfig", "pgpass", "netrc",
        ];

        for pattern in &blocked {
            if lower.contains(pattern) {
                bail!(
                    "access denied: '{}' matches sensitive pattern '{}'",
                    path,
                    pattern
                );
            }
        }

        Ok(())
    }

    /// Check a resolved/canonical path for sensitive patterns (post-resolution).
    pub fn check_sensitive_resolved(&self, path: &std::path::Path) -> Result<()> {
        let s = path.to_string_lossy().to_lowercase();
        let blocked = [
            ".env", ".ssh", ".gnupg", ".aws", ".azure",
            "password", "secret", "token", "private_key",
            "id_rsa", "known_hosts", "kubeconfig", "pgpass", "netrc",
        ];
        for pattern in &blocked {
            if s.contains(pattern) {
                bail!(
                    "access denied: resolved path '{}' matches sensitive pattern '{}'",
                    path.display(),
                    pattern
                );
            }
        }
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
    fn test_resolve_nonexistent_fails() {
        let s = Sandbox::new("C:\\Windows");
        // Should fail because path escapes sandbox
        let result = s.resolve("C:\\Users");
        assert!(result.is_err(), "expected error but got: {:?}", result.ok());
    }

    #[test]
    fn test_check_sensitive_env() {
        let s = Sandbox::new("C:\\");
        assert!(s.check_sensitive("C:\\project\\.env").is_err());
    }

    #[test]
    fn test_check_sensitive_ssh() {
        let s = Sandbox::new("C:\\");
        assert!(s.check_sensitive("C:\\Users\\me\\.ssh\\id_rsa").is_err());
    }

    #[test]
    fn test_check_sensitive_allow_normal() {
        let s = Sandbox::new("C:\\");
        assert!(s.check_sensitive("C:\\Users\\me\\Documents\\file.txt").is_ok());
    }

    #[test]
    fn test_sandbox_escape_blocked() {
        let s = Sandbox::new("C:\\Users");
        let result = s.resolve("C:\\Windows\\System32");
        assert!(result.is_err());
    }

    #[test]
    fn test_check_sensitive_resolved_blocks_ssh() {
        let s = Sandbox::new("C:\\");
        assert!(s.check_sensitive_resolved(std::path::Path::new("C:\\Users\\me\\.ssh\\id_rsa")).is_err());
    }

    #[test]
    fn test_check_sensitive_resolved_blocks_netrc() {
        let s = Sandbox::new("C:\\");
        assert!(s.check_sensitive_resolved(std::path::Path::new("C:\\Users\\me\\.netrc")).is_err());
    }

    #[test]
    fn test_check_sensitive_resolved_allows_normal() {
        let s = Sandbox::new("C:\\");
        assert!(s.check_sensitive_resolved(std::path::Path::new("C:\\Users\\me\\Documents\\file.txt")).is_ok());
    }
}
