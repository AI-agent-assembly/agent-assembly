#!/usr/bin/env bash
# check-metadata-drift.sh
#
# ADR 0014 (canonical metadata registry & drift gate) — hardcoded-value lint mode.
# Fails CI if a registry-owned canonical literal drifts back into the tree. The
# canonical values live in the `.github` registry (`metadata/org-profile.yaml` ->
# `metadata/generated/registry.json`); their values are decided by ADR 0007/0008.
#
# Scope (AAASM-4922): the two highest-fan-out canonical values reconciled in this
# repo that have ZERO legitimate occurrence anywhere here, so the lint stays
# false-positive-free:
#   * the `.github` governance-doc branch — canonical is `main`, not `master`
#     (registry `governance.baseline_doc_base`); AAASM-5293/5294 also blesses
#     the `blob/HEAD` form for cross-repo links (a rename redirect doesn't cover
#     `raw.githubusercontent.com`, `git fetch`, or `uses: ...@master`), so both
#     `main` and `HEAD` pass — anything else, including `master`, is drift
#     (AAASM-5297: this gate previously enforced `master` while the registry it
#     cited said `main`, which the literal-`main` pattern never caught because
#     every real link already used the `HEAD` form);
#   * the `.dev` alternate installer host — it serves the script at its host ROOT
#     (a `custom_domain` route, ADR 0007), so `tool.agent-assembly.dev/install.sh`
#     is a wrong path (registry `urls.installer_alt`).
#
# ADRs are excluded: they quote these drifts as examples. The broader org-wide
# orphan-literal audit (repo names, display names, Jira IDs) is owned by the
# `.github` registry widen (ADR 0014 Appendix B item 1), not this repo-local lint.
#
# Usage: bash .ci/check-metadata-drift.sh   (from the repository root).
# Exit 0 — clean; exit 1 — a registry-owned canonical value drifted.
set -euo pipefail

status=0

# $1 = human description, $2 = ERE pattern, $3 = canonical-fix hint.
check() {
  local desc="$1" pat="$2" fix="$3" hits
  hits=$(git grep -nE "$pat" -- \
    ':(exclude)docs/src/adr/*' \
    ':(exclude).ci/check-metadata-drift.sh' || true)
  if [ -n "$hits" ]; then
    echo "──────────────────────────────────────────────────────"
    echo "Metadata drift: ${desc}"
    echo "${hits}"
    echo "Fix: ${fix}"
    echo "──────────────────────────────────────────────────────"
    status=1
  fi
}

# The 'main'/'master' check matches on the branch segment rather than one
# literal, so an unexpected branch fails deliberately instead of evading the
# pattern by not being the one literal it names (AAASM-5297). 'main' and
# 'HEAD' both pass; everything else, 'master' included, fails.
check_github_governance_branch() {
  local desc="'.github' governance link uses a branch other than the registry's canonical 'main' (or the 'HEAD' form)"
  local fix="use .../.github/blob/main/... or .../.github/blob/HEAD/... (registry governance.baseline_doc_base; HEAD form per AAASM-5293/5294)"
  local hits
  hits=$(git grep -nE 'ai-agent-assembly/\.github/blob/[A-Za-z0-9_.-]+' -- \
    ':(exclude)docs/src/adr/*' \
    ':(exclude).ci/check-metadata-drift.sh' 2>/dev/null \
    | grep -vE 'ai-agent-assembly/\.github/blob/(main|HEAD)([^A-Za-z0-9_.-]|$)' || true)
  if [ -n "$hits" ]; then
    echo "──────────────────────────────────────────────────────"
    echo "Metadata drift: ${desc}"
    echo "${hits}"
    echo "Fix: ${fix}"
    echo "──────────────────────────────────────────────────────"
    status=1
  fi
}
check_github_governance_branch

check "'.dev' installer alt carries an '/install.sh' path (it serves at host root)" \
  'tool\.agent-assembly\.dev/install\.sh' \
  "use https://tool.agent-assembly.dev (registry urls.installer_alt, ADR 0007)"

if [ "$status" -eq 0 ]; then
  echo "Metadata-drift check passed."
fi
exit "$status"
