fn main() {
    #[cfg(windows)]
    {
        println!("cargo:rerun-if-changed=src/ayesha.ico");
        println!("cargo:rerun-if-changed=src/resources.rc");
        
        let out_dir = std::env::var("OUT_DIR").unwrap();
        let rc_paths = [
            r"C:\Program Files (x86)\Windows Kits\10\bin\10.0.26100.0\x64\rc.exe",
            r"C:\Program Files (x86)\Windows Kits\10\bin\10.0.19041.0\x64\rc.exe",
        ];
        
        let mut compiled = false;
        for rc_exe in &rc_paths {
            if std::path::Path::new(rc_exe).exists() && std::path::Path::new("src/resources.rc").exists() {
                let res_path = format!("{}\\resources.res", out_dir);
                let status = std::process::Command::new(rc_exe)
                    .args(&["/fo", &res_path, "src/resources.rc"])
                    .status();
                
                if let Ok(s) = status {
                    if s.success() {
                        println!("cargo:rustc-link-arg={}", res_path);
                        compiled = true;
                        break;
                    }
                }
            }
        }
        if !compiled {
            eprintln!("Warning: could not compile Windows resource icon.");
        }
    }
}
