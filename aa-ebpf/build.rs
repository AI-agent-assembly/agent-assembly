//! Build script for aa-ebpf.
//!
//! Compiles the `aa-ebpf-probes` BPF crate (which targets `bpfel-unknown-none`)
//! and copies the resulting ELF objects into `OUT_DIR` so they can be embedded
//! with `aya::include_bytes_aligned!` in the userspace crate.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // BPF compilation is Linux-only: aya-build invokes `rustup run nightly cargo build`
    // targeting `bpfel-unknown-none`. On macOS/Windows the BPF programs are not
    // compiled; the userspace constants in lib.rs are gated the same way.
    #[cfg(target_os = "linux")]
    aya_build::build_ebpf(
        [aya_build::Package {
            // Must match the [package] name in aa-ebpf-probes/Cargo.toml.
            // aya-build passes this as `cargo build --package <name>` and uses
            // it as the subdirectory under OUT_DIR for the compiled artifacts.
            name: "aa-ebpf-probes",
            root_dir: "../aa-ebpf-probes",
            no_default_features: false,
            features: &[],
        }],
        aya_build::Toolchain::Nightly,
    )?;
    Ok(())
}
