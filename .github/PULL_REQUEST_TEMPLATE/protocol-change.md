## Protocol change description

Brief description of what changed in the protocol specification.

## Change classification

See [docs/versioning.md](../../docs/versioning.md) for the full classification rules.

- [ ] **Non-breaking** — new optional field, new RPC, new enum value, documentation fix
- [ ] **Breaking** — removed/renamed field, changed field number or type, removed RPC or enum value

## Breaking change details *(skip if non-breaking)*

**Deprecated since:** `protocol/v` *(leave blank if not previously deprecated)*

Does this change follow the two-MAJOR-version deprecation window?

- [ ] Yes — item was deprecated at least two major versions ago
- [ ] No — this is an emergency break (explain below)
- [ ] N/A — non-breaking change

## Related issues

- Jira ticket: AAASM-XX
- GitHub issue: #XX

## Checklist

- [ ] `docs/protocol/CHANGELOG.md` updated under the correct section
  (`Added` / `Changed` / `Deprecated` / `Removed` / `Fixed` / `Security`)
- [ ] If breaking: migration guide added at `docs/migration/<vX.Y-to-vZ.0>.md`
  (use [`docs/migration/template.md`](../../docs/migration/template.md) as starting point)
- [ ] Proto lint passes (`buf lint` in `proto/`)
- [ ] No unintended breaking changes (`buf breaking` against base branch)
- [ ] Conformance vectors updated if wire format changed
  (`conformance/vectors/<category>/` and `conformance/vectors/proto/*.bin` if applicable)
- [ ] `docs/compatibility.md` updated if runtime/SDK version compatibility is affected

---

> **To use this template** when opening a PR, append `?template=protocol-change.md`
> to the PR creation URL, or select it from the template picker on GitHub.
