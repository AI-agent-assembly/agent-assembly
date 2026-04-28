# [AAASM-132] eBPF BPF-Side Cross-Compilation Pipeline Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Set up the shared eBPF cross-compilation pipeline so that BPF programs can be compiled to `bpfel-unknown-none`, embedded into the `aa-ebpf` userspace crate, and validated in CI — unblocking AAASM-37, AAASM-38, AAASM-39.

**Architecture:** A dedicated `aa-ebpf-probes/` crate lives outside the main Cargo workspace (no entry in `workspace.members`). It has its own `rust-toolchain.toml` (nightly) and `.cargo/config.toml` (default target = `bpfel-unknown-none`). `aa-ebpf/build.rs` uses `aya-build` to invoke `cargo build` on the probes crate and embed the compiled bytecode. The main workspace stays on stable throughout.

**Tech Stack:** Rust nightly (BPF side only), aya 0.13, aya-ebpf 0.1, aya-log-ebpf 0.1, aya-build 0.1, GitHub Actions ubuntu-latest.

---

## File Map

| Action | Path | Purpose |
|---|---|---|
| Create | `aa-ebpf-probes/Cargo.toml` | BPF-side crate manifest (nightly, no_std) |
| Create | `aa-ebpf-probes/.cargo/config.toml` | Force `bpfel-unknown-none` as default target |
| Create | `aa-ebpf-probes/rust-toolchain.toml` | Pin nightly channel for BPF compilation |
| Create | `aa-ebpf-probes/src/main.rs` | Hello-world kprobe skeleton |
| Modify | `aa-ebpf/Cargo.toml` | Add aya, aya-log (runtime), aya-build (build-dep) |
| Create | `aa-ebpf/build.rs` | Compile probes crate via aya-build |
| Modify | `aa-ebpf/src/lib.rs` | Expose compiled bytecode via `include_bytes_aligned!` |
| Modify | `.github/workflows/ci.yml` | Add `ebpf-build` job (Linux nightly) |
| Create | `aa-ebpf/README.md` | Build instructions + kernel version requirements |

---

## Task 1: Scaffold `aa-ebpf-probes/Cargo.toml`

**Files:**
- Create: `aa-ebpf-probes/Cargo.toml`

- [ ] **Step 1: Create the file**

```toml
[package]
name = "aa-ebpf-probes"
version = "0.0.1"
edition = "2021"
publish = false

[dependencies]
aya-ebpf = "0.1"
aya-log-ebpf = "0.1"

# BPF programs are no_std binaries; there is no std or test harness.
[[bin]]
name = "aa-hello"
path = "src/main.rs"
```

- [ ] **Step 2: Verify the workspace Cargo.toml does NOT list aa-ebpf-probes**

Open `Cargo.toml` (workspace root) and confirm `aa-ebpf-probes` is absent from `[workspace] members`. It must stay outside the workspace so it can use nightly without affecting the rest of the tree.

- [ ] **Step 3: Commit**

```bash
git add aa-ebpf-probes/Cargo.toml
git commit -m "🔧 (aa-ebpf-probes): Scaffold Cargo.toml with aya-ebpf and aya-log-ebpf"
```

---

## Task 2: Add `.cargo/config.toml` for `bpfel-unknown-none` default target

**Files:**
- Create: `aa-ebpf-probes/.cargo/config.toml`

- [ ] **Step 1: Create the config file**

```toml
[build]
target = "bpfel-unknown-none"
```

This makes `cargo build` inside `aa-ebpf-probes/` default to the BPF target without needing `--target` on every invocation.

- [ ] **Step 2: Commit**

```bash
git add aa-ebpf-probes/.cargo/config.toml
git commit -m "🔧 (aa-ebpf-probes): Add .cargo/config.toml setting bpfel-unknown-none as default target"
```

---

## Task 3: Add `rust-toolchain.toml` pinning nightly

**Files:**
- Create: `aa-ebpf-probes/rust-toolchain.toml`

- [ ] **Step 1: Create the toolchain file**

```toml
[toolchain]
channel = "nightly"
components = ["rust-src"]
targets = ["bpfel-unknown-none"]
```

`rust-src` is required by the BPF target because it needs to build `core` from source (`-Zbuild-std=core`). The `targets` entry pre-installs the `bpfel-unknown-none` cross-compilation target.

- [ ] **Step 2: Commit**

```bash
git add aa-ebpf-probes/rust-toolchain.toml
git commit -m "🔧 (aa-ebpf-probes): Add rust-toolchain.toml pinning nightly with rust-src and bpfel-unknown-none"
```

---

## Task 4: Add hello-world kprobe skeleton

**Files:**
- Create: `aa-ebpf-probes/src/main.rs`

- [ ] **Step 1: Create the BPF program**

```rust
#![no_std]
#![no_main]

use aya_ebpf::{macros::kprobe, programs::ProbeContext};
use aya_log_ebpf::info;

/// Minimal kprobe attached to `__x64_sys_write` — validates the BPF
/// compilation pipeline end-to-end. Replace with real probes in
/// AAASM-37 / AAASM-38 / AAASM-39.
#[kprobe]
pub fn aa_hello(ctx: ProbeContext) -> u32 {
    match try_aa_hello(ctx) {
        Ok(ret) => ret,
        Err(ret) => ret,
    }
}

fn try_aa_hello(ctx: ProbeContext) -> Result<u32, u32> {
    info!(&ctx, "aa-hello: __x64_sys_write intercepted");
    Ok(0)
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
```

- [ ] **Step 2: Verify the file compiles (on Linux with nightly installed)**

If on Linux with nightly available:
```bash
cd aa-ebpf-probes
cargo build --release
# Expected: Compiling aa-ebpf-probes ... Finished release
```

On macOS, skip this step — BPF cross-compilation is Linux-only. The CI job in Task 8 will verify this.

- [ ] **Step 3: Commit**

```bash
git add aa-ebpf-probes/src/main.rs
git commit -m "✨ (aa-ebpf-probes): Add hello-world kprobe skeleton on __x64_sys_write"
```

---

## Task 5: Add aya dependencies to `aa-ebpf/Cargo.toml`

**Files:**
- Modify: `aa-ebpf/Cargo.toml`

Current content:
```toml
[package]
name = "aa-ebpf"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
description = "eBPF-based kernel-level monitoring hooks for Agent Assembly"

[dependencies]
aa-core = { path = "../aa-core" }

[lints]
workspace = true
```

- [ ] **Step 1: Add aya runtime deps and aya-build build-dep**

```toml
[package]
name = "aa-ebpf"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
description = "eBPF-based kernel-level monitoring hooks for Agent Assembly"

[dependencies]
aa-core = { path = "../aa-core" }
aya = { version = "0.13", features = ["async_tokio"] }
aya-log = "0.2"

[build-dependencies]
aya-build = "0.1"

[lints]
workspace = true
```

- [ ] **Step 2: Commit**

```bash
git add aa-ebpf/Cargo.toml
git commit -m "🔧 (aa-ebpf): Add aya, aya-log runtime deps and aya-build build-dependency"
```

---

## Task 6: Add `aa-ebpf/build.rs`

**Files:**
- Create: `aa-ebpf/build.rs`

- [ ] **Step 1: Create the build script**

```rust
//! Build script for aa-ebpf.
//!
//! Compiles the `aa-ebpf-probes` BPF crate (which targets `bpfel-unknown-none`)
//! and makes the resulting bytecode available to the userspace crate via
//! `OUT_DIR`.

fn main() {
    let probes_dir = std::path::PathBuf::from("../aa-ebpf-probes");

    // aya-build invokes `cargo build --target bpfel-unknown-none` inside the
    // probes crate and copies the compiled ELF objects into OUT_DIR so they
    // can be embedded with `include_bytes_aligned!`.
    aya_build::build_ebpf([aya_build::Ebpf {
        source_dir: probes_dir,
        target: None, // reads from aa-ebpf-probes/.cargo/config.toml
    }])
    .expect("failed to compile aa-ebpf-probes BPF programs");
}
```

> **Note:** The exact `aya_build::Ebpf` struct fields may differ slightly between patch versions. Verify against the installed version of `aya-build`. If the struct API differs, use `aya_build::build_ebpf_with_dir(probes_dir)` as a fallback — both forms invoke the same underlying `cargo build`.

- [ ] **Step 2: Run cargo check on aa-ebpf to confirm build.rs parses**

```bash
cargo check -p aa-ebpf
# Expected: either success or "cannot compile `aa-ebpf-probes`" (BPF-side error,
# acceptable on macOS) — NOT a Rust syntax error in build.rs itself.
```

- [ ] **Step 3: Commit**

```bash
git add aa-ebpf/build.rs
git commit -m "🔧 (aa-ebpf): Add build.rs to compile BPF probes crate via aya-build"
```

---

## Task 7: Expose compiled bytecode in `aa-ebpf/src/lib.rs`

**Files:**
- Modify: `aa-ebpf/src/lib.rs`

- [ ] **Step 1: Update lib.rs**

```rust
//! eBPF-based kernel-level monitoring hooks for Agent Assembly.
//!
//! This crate provides kernel-space probes that intercept system calls and
//! network events originating from AI agent processes, feeding data into
//! the governance pipeline without requiring code changes in agents.
//!
//! # Loading BPF programs
//!
//! The compiled BPF bytecode is embedded at build time and exposed as the
//! [`AA_HELLO_BPF`] constant. Pass it to [`aya::Ebpf::load`] to load all
//! programs defined in the `aa-ebpf-probes` crate:
//!
//! ```no_run
//! # #[cfg(target_os = "linux")]
//! # {
//! use aya::Ebpf;
//! use aa_ebpf::AA_HELLO_BPF;
//!
//! let mut bpf = Ebpf::load(AA_HELLO_BPF).expect("failed to load BPF programs");
//! # }
//! ```

/// Compiled BPF bytecode for the `aa-hello` probe program.
///
/// Embedded from `aa-ebpf-probes/src/main.rs` at build time. Pass this to
/// [`aya::Ebpf::load`] to obtain a handle to all programs in the probe crate.
///
/// Only meaningful on Linux — on other platforms the bytes are present but
/// cannot be loaded into the kernel.
#[cfg(target_os = "linux")]
pub static AA_HELLO_BPF: &[u8] = aya::include_bytes_aligned!(
    concat!(env!("OUT_DIR"), "/aa-hello")
);
```

- [ ] **Step 2: Confirm cargo check passes on stable (macOS/Linux userspace)**

```bash
cargo check -p aa-ebpf
# Expected: Finished (or warnings only, no errors)
```

- [ ] **Step 3: Commit**

```bash
git add aa-ebpf/src/lib.rs
git commit -m "✨ (aa-ebpf): Expose compiled BPF bytecode via include_bytes_aligned! in lib.rs"
```

---

## Task 8: Add `ebpf-build` CI job to `ci.yml`

**Files:**
- Modify: `.github/workflows/ci.yml`

- [ ] **Step 1: Add the new job at the end of ci.yml (before the conformance placeholder jobs)**

Find the `conformance:` job in `.github/workflows/ci.yml` and insert the following block immediately before it:

```yaml
  ebpf-build:
    name: eBPF probes build (Linux nightly)
    runs-on: ubuntu-latest
    defaults:
      run:
        working-directory: aa-ebpf-probes
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@nightly
        with:
          targets: bpfel-unknown-none
          components: rust-src
      - uses: Swatinem/rust-cache@e18b497796c12c097a38f9edb9d0641fb99eee32 # v2
        with:
          workspaces: aa-ebpf-probes
      - name: Build eBPF probes
        run: cargo build --release
      - name: Clippy on eBPF probes
        run: cargo clippy --all-targets -- -D warnings
```

This job:
- Runs only on Linux (`ubuntu-latest`)
- Uses nightly Rust with the `bpfel-unknown-none` target and `rust-src` component pre-installed
- Caches the probes crate separately from the workspace cache
- The existing `build` / `test` / `clippy` jobs on stable never enter `aa-ebpf-probes/`, so macOS and stable CI are unaffected

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "🔧 (ci): Add ebpf-build job — Linux nightly runner, bpfel-unknown-none compilation"
```

---

## Task 9: Add `aa-ebpf/README.md` with build instructions

**Files:**
- Create: `aa-ebpf/README.md`

- [ ] **Step 1: Create the README**

```markdown
# aa-ebpf

Kernel-level eBPF monitoring hooks for Agent Assembly (Layer 3 of the three-layer
defense-in-depth architecture).

## Architecture

BPF programs live in a **separate crate** (`aa-ebpf-probes/`) that compiles to the
`bpfel-unknown-none` target under nightly Rust. The userspace crate (`aa-ebpf`, this
crate) loads the compiled bytecode via `aya::Ebpf::load`.

```
aa-ebpf-probes/          ← BPF-side (nightly, bpfel-unknown-none)
    src/main.rs          ← kprobe / uprobe / tracepoint programs
    rust-toolchain.toml  ← nightly pin
    .cargo/config.toml   ← default target = bpfel-unknown-none

aa-ebpf/                 ← userspace (stable, in workspace)
    build.rs             ← compiles aa-ebpf-probes via aya-build
    src/lib.rs           ← loads bytecode, exposes pub API
```

## Prerequisites

| Requirement | Version |
|---|---|
| Rust toolchain | nightly (managed by `aa-ebpf-probes/rust-toolchain.toml`) |
| Linux kernel | 5.8+ (required for BPF ring buffer; 5.15+ recommended for BTF) |
| OS | Linux only — eBPF cannot be loaded on macOS or Windows |

## Building BPF programs

The workspace build (`cargo build --workspace`) does NOT compile BPF programs —
they are compiled by `aa-ebpf`'s `build.rs` when you build `aa-ebpf`.

```bash
# Build the userspace crate (triggers BPF compilation automatically via build.rs)
cargo build -p aa-ebpf

# Build BPF programs directly (for iteration)
cd aa-ebpf-probes
cargo build --release
```

## CI

The `ebpf-build` GitHub Actions job (`.github/workflows/ci.yml`) compiles the BPF
programs on a Linux runner with nightly Rust. macOS CI jobs never enter
`aa-ebpf-probes/` and are unaffected.

## CO-RE / BTF

Compile Once, Run Everywhere (CO-RE) via BTF is **not enabled** in the current
hello-world skeleton. When implementing real probes (AAASM-37 / AAASM-38 / AAASM-39),
generate `vmlinux.h` from the CI runner kernel and check it in:

```bash
bpftool btf dump file /sys/kernel/btf/vmlinux format c > aa-ebpf-probes/src/vmlinux.h
```

Minimum kernel version for CO-RE: **5.8** (BPF ring buffer) / **5.15** (stable BTF).

## Adding new probes

1. Add a new `[[bin]]` entry to `aa-ebpf-probes/Cargo.toml`.
2. Write the BPF program in `aa-ebpf-probes/src/<probe-name>.rs`.
3. Expose the compiled bytes in `aa-ebpf/src/lib.rs` using `include_bytes_aligned!`.
4. Write a loader function in `aa-ebpf/src/lib.rs` that attaches the program.
```

- [ ] **Step 2: Commit**

```bash
git add aa-ebpf/README.md
git commit -m "📝 (aa-ebpf): Add README with build instructions, kernel requirements, and CO-RE notes"
```

---

## Verification Checklist

After all 9 tasks are complete:

- [ ] `cargo check -p aa-ebpf` passes on macOS (stable) — no BPF-side errors
- [ ] `cargo clippy --workspace` passes on stable — no warnings in aa-ebpf
- [ ] `cargo build --workspace` succeeds (aa-ebpf-probes is excluded from workspace)
- [ ] On Linux with nightly: `cd aa-ebpf-probes && cargo build --release` succeeds
- [ ] `git log --oneline` shows exactly 9 new commits, each scoped to one change
- [ ] CI `ebpf-build` job is present in `.github/workflows/ci.yml`

---

## Notes for AAASM-37 / AAASM-38 / AAASM-39

These tickets build on top of this infrastructure. When implementing them:
1. Add a new `[[bin]]` to `aa-ebpf-probes/Cargo.toml` per probe
2. Write the probe in `aa-ebpf-probes/src/<probe>.rs`
3. Expose bytecode and loader in `aa-ebpf/src/lib.rs`
4. BTF/CO-RE decision should be made in those tickets using the notes above
