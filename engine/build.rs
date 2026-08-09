use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    #[cfg(windows)]
    {
        println!("cargo:rerun-if-changed=src/resources.rc");
        println!("cargo:rerun-if-changed=src/ayesha.ico");

        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let out_dir = std::env::var("OUT_DIR").unwrap();

        let Some(rc_exe) = find_rc_exe() else {
            eprintln!("Warning: could not locate rc.exe - ayesha icon will not be embedded.");
            return;
        };

        let res_path = Path::new(&out_dir).join("resources.res");
        let status = Command::new(&rc_exe)
            .current_dir(&manifest_dir)
            .args(["/fo", res_path.to_str().unwrap(), "src/resources.rc"])
            .status();

        match status {
            Ok(s) if s.success() => {
                println!("cargo:rustc-link-arg={}", res_path.display());
            }
            Ok(s) => {
                eprintln!("Warning: rc.exe exited with {s} - ayesha icon not embedded.");
            }
            Err(e) => {
                eprintln!("Warning: failed to run {}: {e}", rc_exe.display());
            }
        }
    }
}

#[cfg(windows)]
fn find_rc_exe() -> Option<PathBuf> {
    for var in ["RC", "WINRES_RC"] {
        if let Ok(p) = std::env::var(var) {
            let p = PathBuf::from(p);
            if p.is_file() {
                return Some(p);
            }
        }
    }

    if let Some(p) = which("rc.exe") {
        return Some(p);
    }

    let kits = PathBuf::from(r"C:\Program Files (x86)\Windows Kits\10\bin");
    let mut best: Option<PathBuf> = None;
    let mut best_ver = (0u32, 0u32);
    if let Ok(entries) = std::fs::read_dir(&kits) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let ver = parse_ver(&name);
            if ver > best_ver {
                let candidate = entry.path().join("x64").join("rc.exe");
                if candidate.is_file() {
                    best_ver = ver;
                    best = Some(candidate);
                }
            }
        }
    }
    best
}

#[cfg(windows)]
fn parse_ver(s: &str) -> (u32, u32) {
    let parts: Vec<&str> = s.split('.').collect();
    let major = parts.first().and_then(|p| p.parse().ok()).unwrap_or(0);
    let minor = parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(0);
    (major, minor)
}

#[cfg(windows)]
fn which(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}
