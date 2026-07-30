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
                        // Check that parent is within sandbox before allowing early return
                        let parent_canonical = parent.canonicalize().unwrap_or_else(|_| parent.to_path_buf());
                        if parent_canonical.starts_with(&root_canonical) {
                            return Ok(resolved);
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

    pub fn root(&self) -> &Path {
        &self.root
    }
}
