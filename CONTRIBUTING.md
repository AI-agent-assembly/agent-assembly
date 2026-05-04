# Contributing to agent-assembly

Thank you for your interest in contributing! This guide explains how to set up your environment and submit changes.

## Prerequisites

- **Rust stable** (≥ 1.75) — install via [rustup](https://rustup.rs/)
- **cargo-nextest** — `cargo install cargo-nextest`
- **cargo-deny** — `cargo install cargo-deny`
- **Lefthook** — `brew install lefthook` (macOS) or see [install guide](https://github.com/evilmartians/lefthook/blob/master/docs/install.md)

## Setup

```bash
git clone https://github.com/AI-agent-assembly/agent-assembly.git
cd agent-assembly

# Install git hooks (runs fmt, clippy, deny on commit)
lefthook install

# Verify the workspace builds
cargo build --workspace

# Run the test suite
cargo nextest run --workspace
```

## Branch Naming

```
<version>/<ticket-number>/<short-summary>
```

Example: `v0.0.1/AAASM-42/add_agent_registry`

## Commit Style

Use [Gitmoji](https://gitmoji.dev/) prefixed messages:

```
<emoji> (<scope>): <imperative summary>
```

**One commit per logical unit** — one new file, one property change, one function. Keep commits small and bisectable.

Examples:
- `✨ (aa-core): Add AgentId newtype wrapper`
- `🐛 (aa-gateway): Fix policy evaluation order for overlapping rules`
- `🔧 (ci): Add matrix build for MSRV check`

## Adding a new crate

To add a new crate to the workspace:

1. Scaffold the crate with `cargo new --lib aa-<name>` from the repo root.
2. Add `aa-<name>` to the `members` array in the top-level [`Cargo.toml`](Cargo.toml).
3. In the new crate's `Cargo.toml`, inherit workspace metadata:

   ```toml
   [package]
   name = "aa-<name>"
   version.workspace = true
   edition.workspace = true
   license.workspace = true
   repository.workspace = true
   ```

4. Use `[workspace.lints.clippy]` from the top-level `Cargo.toml` — do **not** redefine clippy lints per-crate.
5. If the crate exposes a binary, declare it explicitly under `[[bin]]` (see [`aa-cli/Cargo.toml`](aa-cli/Cargo.toml) for the canonical example).
6. Run `cargo build --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo doc --workspace --no-deps` to confirm the new crate integrates cleanly.
7. Add the crate to the **Crate Map** table and **Repository Layout** tree in [`README.md`](README.md).

## Pull Requests

- Open a PR against `master`.
- Title format: `[<ticket>] <emoji> (<scope>): <summary>`
- Fill in the PR template — all checklist items must be addressed.
- CI must be green before review is requested.
- At least **1 approval** from the Pioneer team is required to merge.

## Developer Certificate of Origin (DCO)

Every commit must be signed off under the [Developer Certificate of Origin v1.1](https://developercertificate.org/) — this licenses your contribution to the project under the [Apache License 2.0](LICENSE).

Sign off by adding a `Signed-off-by` trailer to each commit message:

```
✨ (aa-core): Add AgentId newtype wrapper

Signed-off-by: Jane Doe <jane@example.com>
```

The easiest way is to pass `-s` (or `--signoff`) to `git commit`:

```bash
git commit -s -m "✨ (aa-core): Add AgentId newtype wrapper"
```

PRs whose commits are missing the sign-off trailer will be blocked by the DCO check.

## Code Quality

Pre-commit hooks enforce these automatically on every `git commit`:

| Check | Command | Config |
|---|---|---|
| Formatting | `cargo fmt --all -- --check` | [`rustfmt.toml`](rustfmt.toml) |
| Linting | `cargo clippy --all-targets -- -D warnings` | [`clippy.toml`](clippy.toml) + `[workspace.lints.clippy]` in [`Cargo.toml`](Cargo.toml) |
| Dependencies | `cargo deny check` | [`deny.toml`](deny.toml) |

On `git push`, documentation is also checked: `cargo doc --workspace --no-deps`.

The workspace-level clippy lints (`correctness = deny`, `suspicious = deny`, others `warn`) live in `[workspace.lints.clippy]` of the top-level `Cargo.toml` — do not override them per-crate.

## Build docs locally

Contributor documentation is an [mdBook](https://rust-lang.github.io/mdBook/) rooted at `docs/`. To build or preview it:

```bash
# One-time install (pin matches CI)
cargo install --locked --version 0.5.2 mdbook
cargo install --locked --version 0.17.0 mdbook-mermaid

# Build static HTML into docs/book/
mdbook build docs

# Live-reload preview at http://localhost:3000
mdbook serve docs --open
```

Mermaid diagrams use the `mdbook-mermaid` preprocessor, which is wired in `docs/book.toml`. The `Docs` GitHub Actions workflow runs `mdbook build docs` on every PR that touches `docs/**`, `README.md`, or `CONTRIBUTING.md` and fails the build on errors.

## Reporting Issues

Use the GitHub issue templates:
- **Bug report** — reproducible steps, expected vs actual behaviour, environment.
- **Feature request** — motivation, proposed solution, alternatives considered.

For security issues, see [SECURITY.md](SECURITY.md).
