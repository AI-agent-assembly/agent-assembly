//! Build script for aa-ebpf.
//!
//! Compiles the `aa-ebpf-probes` BPF crate (targeting `bpfel-unknown-none`)
//! and places the compiled binaries into `OUT_DIR/aa-ebpf-probes/…` so they
//! can be embedded with `aya::include_bytes_aligned!` in the userspace crate.
//!
//! ## Why not `aya_build::build_ebpf`?
//!
//! `aya_build` 0.1.3 runs `cargo build --package <name>` from the *caller's*
//! working directory — it does not use `Package::root_dir` as `current_dir`.
//! `aa-ebpf-probes` is a standalone workspace so cargo cannot resolve it as a
//! package from `aa-ebpf/`.  We invoke cargo directly with an explicit
//! `current_dir` to avoid this limitation.
//!
//! ## Graceful fallback
//!
//! When the nightly toolchain is not installed (e.g. standard CI runners),
//! the build script creates empty stub files so the crate still compiles.
//! Loading these stubs at runtime will fail in `Ebpf::load()`, which the
//! runtime handles via per-loader degradation.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // BPF compilation is Linux-only. On macOS/Windows the build script is a no-op;
    // the userspace constants in lib.rs are gated with the same cfg predicate.
    #[cfg(target_os = "linux")]
    {
        use std::{env, fs, path::PathBuf, process::Command};

        let manifest_dir = env::var("CARGO_MANIFEST_DIR")?;
        let probes_dir = PathBuf::from(&manifest_dir).join("../aa-ebpf-probes");
        let out_dir = env::var("OUT_DIR")?;
        // Mirror the path layout used by aya-build: OUT_DIR/<package-name>/…
        // lib.rs embeds the binary at OUT_DIR/aa-ebpf-probes/bpfel-unknown-none/release/aa-hello
        let target_dir = PathBuf::from(&out_dir).join("aa-ebpf-probes");

        // Re-run this script whenever the probes source changes.
        println!("cargo:rerun-if-changed={}", probes_dir.display());

        // The binary paths that lib.rs expects via include_bytes_aligned!
        let release_dir = target_dir.join("bpfel-unknown-none/release");
        let binaries = ["aa-file-io", "aa-exec-probes", "aa-tls-probes"];

        // Run `cargo build --release` inside the probes workspace.
        // aa-ebpf-probes/.cargo/config.toml sets:
        //   target      = "bpfel-unknown-none"
        //   build-std   = ["core"]   (nightly only; rust-toolchain.toml pins nightly)
        let status = Command::new("rustup")
            .args(["run", "nightly", "cargo", "build", "--release"])
            .arg("--target-dir")
            .arg(&target_dir)
            // Change into the probes workspace so cargo resolves its Cargo.toml.
            .current_dir(&probes_dir)
            // Strip cargo's injected RUSTC wrappers so the probes workspace uses
            // the bare nightly rustc without the parent workspace overlay.
            .env_remove("RUSTC")
            .env_remove("RUSTC_WORKSPACE_WRAPPER")
            .status();

        let build_ok = matches!(status, Ok(s) if s.success());

        if !build_ok {
            eprintln!(
                "cargo:warning=BPF probe compilation failed (nightly toolchain missing?). \
                 Creating empty stubs — eBPF loaders will degrade at runtime."
            );
            // Create empty stub files so include_bytes_aligned! has something to embed.
            fs::create_dir_all(&release_dir)?;
            for name in &binaries {
                let path = release_dir.join(name);
                if !path.exists() {
                    fs::write(&path, b"")?;
                }
            }
        }
    }

    Ok(())
}
