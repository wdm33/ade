#!/usr/bin/env bash
set -euo pipefail

# CI: enforce that no ChainDb / chain_db symbol appears in BLUE
# ade_core::consensus. Strengthens DC-CORE-01 + DC-CONS-07 (S-B1).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TARGET="$REPO_ROOT/crates/ade_core/src/consensus"

if [ ! -d "$TARGET" ]; then
    echo "FAIL: $TARGET does not exist."
    exit 1
fi

if grep -RnE '\b(ChainDb|chain_db)\b' "$TARGET" > /tmp/ade_chaindb_hits 2>/dev/null; then
    echo "FAIL: ChainDb / chain_db reference in BLUE consensus:"
    cat /tmp/ade_chaindb_hits
    exit 1
fi

echo "OK: no ChainDb / chain_db in ade_core::consensus"
