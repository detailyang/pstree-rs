fn main() {
    // On macOS the libc crate links -liconv. In a Nix environment the system
    // libiconv is not on the default linker search path, so we locate it via
    // the NIX_LIBICONV_PATH env var (set by the Nix shell / devShell), or fall
    // back to a best-effort search under /nix/store.
    #[cfg(target_os = "macos")]
    {
        if let Ok(path) = std::env::var("NIX_LIBICONV_PATH") {
            println!("cargo:rustc-link-search=native={}/lib", path);
            return;
        }

        // If we are inside a Nix build (or nix develop), SDKROOT or similar
        // vars may not be set, but we can scan for a known pattern.
        if let Ok(entries) = std::fs::read_dir("/nix/store") {
            let mut candidates: Vec<std::path::PathBuf> = entries
                .flatten()
                .filter_map(|e| {
                    let name = e.file_name();
                    let s = name.to_string_lossy();
                    if s.contains("libiconv") && !s.ends_with(".drv") {
                        let lib = e.path().join("lib");
                        if lib.join("libiconv.dylib").exists() {
                            return Some(lib);
                        }
                    }
                    None
                })
                .collect();

            // Prefer the macOS system-version package (109.x) over GNU libiconv.
            candidates.sort_by(|a, b| {
                let a_sys = a.to_string_lossy().contains("109");
                let b_sys = b.to_string_lossy().contains("109");
                b_sys.cmp(&a_sys)
            });

            if let Some(lib_dir) = candidates.first() {
                println!("cargo:rustc-link-search=native={}", lib_dir.display());
            }
        }
    }
}
