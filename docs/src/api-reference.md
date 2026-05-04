# API Reference

Authoritative API documentation for the Rust crates lives in rustdoc, generated directly from source. This chapter explains how to produce and browse it.

## Generating rustdoc locally

The whole-workspace rustdoc is built with `cargo doc`. The pre-push lefthook hook also runs this command, so the docs are guaranteed to compile on `master`.

```bash
# Build rustdoc for every workspace member without recursing into transitive deps.
cargo doc --workspace --no-deps

# Same, but also opens the index page in the default browser.
cargo doc --workspace --no-deps --open

# Document private items too — useful when working inside a single crate.
cargo doc -p aa-gateway --no-deps --document-private-items --open
```

The HTML output lands in `target/doc/`. Open `target/doc/aa_core/index.html` (or any other crate's index) directly if you'd rather not use `--open`.

> **Note on eBPF crates** — `aa-ebpf*` requires a nightly toolchain to build the BPF target. CI excludes these crates from the standard build matrix and validates them in a dedicated job. For rustdoc on macOS or non-Linux machines, run `cargo doc --workspace --no-deps --exclude aa-ebpf` to skip them.
