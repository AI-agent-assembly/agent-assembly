# Version Compatibility Matrix

This document tracks which versions of `aa-runtime` are compatible with each SDK version. Update this file whenever any component version changes — see [CI enforcement](#ci-enforcement) below.

> **CI enforcement for SDK version changes is pending cross-repo CI integration.** Until then, SDK version bumps must be accompanied by a manual update to this file.

---

## Compatibility Matrix

| `aa-runtime` | Python SDK (`aa-ffi-python`) | Node.js SDK (`aa-ffi-node`) | Go SDK | Protocol Version |
|---|---|---|---|---|
| v0.0.1 | v0.0.1 ✓ | v0.0.1 ✓ | v0.0.1 ✓ | protocol/v1 |

**Legend:**
- ✓ Compatible — fully supported
- ⚠️ Partial — works with known limitations (see notes)
- ✗ Incompatible — do not use together

---

## Minimum Supported Runtime Version per SDK

| SDK | Minimum `aa-runtime` Version |
|---|---|
| Python SDK v0.0.1 | aa-runtime v0.0.1 |
| Node.js SDK v0.0.1 | aa-runtime v0.0.1 |
| Go SDK v0.0.1 | aa-runtime v0.0.1 |

---

## Supported Protocol Versions per Runtime

A runtime version may support multiple protocol versions to allow SDK upgrades without simultaneous runtime upgrades.

| `aa-runtime` Version | Supported Protocol Versions |
|---|---|
| v0.0.1 | protocol/v1 |

---

## CI Enforcement

A CI check (`compat-matrix-check`) enforces that this file is updated whenever version-carrying files change in a pull request.

**Currently enforced (monorepo scope):**
- `Cargo.toml` (workspace root)
- `crates/*/Cargo.toml` (all crate manifests)

**Deferred — pending cross-repo CI integration:**
- `sdk/python/pyproject.toml` (Python SDK)
- `sdk/node/package.json` (Node.js SDK)
- `sdk/go/go.mod` (Go SDK)

Until cross-repo CI exists, SDK version bumps require a **manual update** to this file before merging.

---

## How to Update This File

When bumping a component version:

1. Add a new row to the [Compatibility Matrix](#compatibility-matrix) table for the new version combination.
2. Update the [Minimum Supported Runtime Version](#minimum-supported-runtime-version-per-sdk) table if the minimum changes.
3. Update the [Supported Protocol Versions](#supported-protocol-versions-per-runtime) table if the runtime adds or drops protocol version support.
4. Commit the change in the same PR as the version bump.

See [versioning.md](versioning.md) for the full versioning and deprecation policy.

---

## Workspace changes (non-version bumps)

| PR / Ticket | Change | Compatibility impact |
|---|---|---|
| AAASM-107 | Added `conformance` workspace crate (test infrastructure, not shipped) | None — internal tooling only |
| AAASM-39 | Added `aa-ebpf-common` workspace crate (shared eBPF types, not shipped standalone) | None — internal shared types only |
| AAASM-37  | Added `aa-ebpf-common` workspace crate (no_std shared eBPF event types, not shipped as a public API) | None — internal kernel/userspace bridge only |
