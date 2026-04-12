#!/usr/bin/env bash
set -euo pipefail

# Pallas types must not leak outside the ade_plutus quarantine.
#
# ade_plutus wraps aiken_uplc, which transitively depends on
# pallas-addresses / pallas-codec / pallas-crypto / pallas-primitives /
# pallas-traverse. Per Phase 3 cluster plan (Cluster P-B quarantine policy),
# no pallas-originated type may appear in the public API of any Ade
# crate. This script grep-scans Ade's source outside ade_plutus and
# fails if any `pallas_` or `pallas-` reference is found.
#
# Discharged slice-entry obligation: O-29.2 (see
# docs/active/S-29_obligation_discharge.md).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

# All Ade crates except the ade_plutus quarantine.
QUARANTINED_CRATES=(
    "ade_codec"
    "ade_types"
    "ade_crypto"
    "ade_core"
    "ade_ledger"
    "ade_testkit"
    "ade_runtime"
    "ade_node"
)

FAILED=0

for crate in "${QUARANTINED_CRATES[@]}"; do
    SRC_DIR="$REPO_ROOT/crates/$crate"
    if [ ! -d "$SRC_DIR" ]; then
        continue
    fi

    # Scan source AND Cargo.toml. Both are Ade-authored and must not
    # reference pallas.
    matches=$(grep -rn -E 'pallas[_-][a-z]+' "$SRC_DIR" \
        --include='*.rs' \
        --include='Cargo.toml' 2>/dev/null | \
        grep -v ':[0-9]*:\s*//' | \
        grep -v ':[0-9]*:\s*#' || true)

    if [ -n "$matches" ]; then
        echo "FAIL: pallas reference found in $crate (quarantined; only ade_plutus may depend on pallas-*):"
        echo "$matches"
        FAILED=1
    fi
done

if [ "$FAILED" -eq 0 ]; then
    echo "PASS: pallas-* types confined to ade_plutus quarantine"
    exit 0
else
    exit 1
fi
