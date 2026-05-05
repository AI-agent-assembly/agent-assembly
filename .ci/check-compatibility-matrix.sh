#!/usr/bin/env bash
# check-compatibility-matrix.sh
#
# Enforces that docs/src/compatibility.md is updated whenever a version-carrying
# file is modified in a pull request.
#
# Usage: run from the repository root on a PR branch.
#   bash .ci/check-compatibility-matrix.sh
#
# Exit codes:
#   0 — check passed (no version files changed, or compatibility.md was updated)
#   1 — check failed (version files changed without updating compatibility.md)

set -euo pipefail

BASE="${GITHUB_BASE_REF:-master}"
MERGE_BASE=$(git merge-base HEAD "origin/${BASE}")
CHANGED=$(git diff --name-only "${MERGE_BASE}"...HEAD)

# Version-carrying files in scope today (monorepo Rust workspace).
#
# TODO: When python-sdk, node-sdk, go-sdk are added to the monorepo (or when
# cross-repo CI coordination exists), extend VERSION_FILES to include:
#   sdk/python/pyproject.toml
#   sdk/node/package.json
#   sdk/go/go.mod
VERSION_FILES=$(echo "${CHANGED}" | grep -E "^(Cargo\.toml|crates/[^/]+/Cargo\.toml)$" || true)
COMPAT_CHANGED=$(echo "${CHANGED}" | grep -E "^docs/src/compatibility\.md$" || true)

if [ -n "${VERSION_FILES}" ] && [ -z "${COMPAT_CHANGED}" ]; then
  echo "──────────────────────────────────────────────────────"
  echo "CI FAILURE: compatibility matrix not updated"
  echo "──────────────────────────────────────────────────────"
  echo ""
  echo "The following version-carrying files were modified in this PR:"
  echo ""
  echo "${VERSION_FILES}" | sed 's/^/  /'
  echo ""
  echo "But docs/src/compatibility.md was NOT updated."
  echo ""
  echo "Please update docs/src/compatibility.md to reflect the version change."
  echo "See docs/src/versioning.md for instructions."
  echo "──────────────────────────────────────────────────────"
  exit 1
fi

echo "Compatibility matrix check passed."
