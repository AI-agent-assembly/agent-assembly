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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // BPF compilation is Linux-only. On macOS/Windows the build script is a no-op;
    // the userspace constants in lib.rs are gated with the same cfg predicate.
    #[cfg(target_os = "linux")]
    {
        use std::{env, path::PathBuf, process::Command};

        let manifest_dir = env::var("CARGO_MANIFEST_DIR")?;
        let probes_dir = PathBuf::from(&manifest_dir).join("../aa-ebpf-probes");
        let out_dir = env::var("OUT_DIR")?;
        // Mirror the path layout used by aya-build: OUT_DIR/<package-name>/…
        // lib.rs embeds the binary at OUT_DIR/aa-ebpf-probes/bpfel-unknown-none/release/aa-hello
        let target_dir = PathBuf::from(&out_dir).join("aa-ebpf-probes");

        // Re-run this script whenever the probes source changes.
        println!("cargo:rerun-if-changed={}", probes_dir.display());

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
            .status()?;

        if !status.success() {
            return Err("BPF probe compilation failed — see cargo output above".into());
        }
    }
    Ok(())
}
