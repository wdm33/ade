#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-M-A S5 — honest-scope CI gate (RO-GENESIS-REPLAY-01).
#
# Per memory [[feedback-shell-must-not-overstate-semantic-truth]]
# the cluster does NOT claim "Ade has independently replayed
# genesis → P". The claim is honestly carried as an open
# obligation; this gate prevents a future commit from flipping
# RO-GENESIS-REPLAY-01 to `enforced` without doing the actual
# multi-month work.
#
# Mechanical guards:
#   1. `RO-GENESIS-REPLAY-01` is present in the registry.
#   2. Its `status` is `declared`.
#   3. Its `open_obligation` is
#      `"blocked_until_genesis_replay_cluster"`.
#
# If a future cluster ships actual genesis→P replay, it must:
#   (a) flip status to `enforced`,
#   (b) drop the open_obligation,
#   (c) update this gate to match (or delete it + add the
#       enforcement-side gates).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REGISTRY="$REPO_ROOT/docs/ade-invariant-registry.toml"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

if [[ ! -f "$REGISTRY" ]]; then
    print_fail "missing $REGISTRY"
    exit "$FAILED"
fi

# Extract the RO-GENESIS-REPLAY-01 block (id line + the next
# ~30 lines until the next [[rules]] or EOF).
block=$(awk '
    /^id = "RO-GENESIS-REPLAY-01"/ { capture=1; print; next }
    capture && /^\[\[rules\]\]/ { exit }
    capture { print }
' "$REGISTRY")

if [[ -z "$block" ]]; then
    print_fail "RO-GENESIS-REPLAY-01 missing from registry"
else
    if ! echo "$block" | grep -qE '^status = "declared"'; then
        print_fail "RO-GENESIS-REPLAY-01 status must be \"declared\" (genesis replay not yet implemented)"
    fi
    if ! echo "$block" | grep -qE '^open_obligation = "blocked_until_genesis_replay_cluster"'; then
        print_fail "RO-GENESIS-REPLAY-01 open_obligation must be \"blocked_until_genesis_replay_cluster\""
    fi
fi

if (( FAILED == 0 )); then
    echo "OK: RO-GENESIS-REPLAY-01 honesty preserved (status=declared, open_obligation intact)"
fi
exit $FAILED
