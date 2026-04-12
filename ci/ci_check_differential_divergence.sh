#!/usr/bin/env bash
set -euo pipefail

# CE-75: Verify zero differential divergence on non-Plutus blocks.
#
# Two layers:
#   1. Verdict agreement — 10,500 blocks across 7 eras, every block the
#      oracle accepted is accepted by Ade.
#   2. Boundary fingerprint agreement — at each of the 12 proof-grade
#      boundary snapshots, Ade's canonical state fingerprint matches
#      the pinned hash recorded for that snapshot. Detects any change
#      in the state bridge or fingerprint encoding.
#
# Per-block state-hash agreement requires an external live differential
# harness (ShadowBox adapter — out of scope for in-repo CI) and is
# deferred; boundary-level agreement is the in-repo closure surface.
#
# Version-scoped to cardano-node 10.6.2.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

echo "=== CE-75: Differential Divergence ==="
echo "Layer 1/2: 10,500-block verdict agreement across 7 eras..."

cargo test --test differential_replay_all_eras all_eras_replay_summary -- --nocapture 2>&1

echo ""
echo "Running per-era replay (1,500 blocks each)..."

FAILED=0
for era in byron shelley allegra mary alonzo babbage conway; do
    if ! cargo test --test differential_replay_all_eras "${era}_replay_all_1500" -- --nocapture 2>&1; then
        echo "FAIL: ${era} replay diverged"
        FAILED=1
    fi
done

echo ""
echo "Layer 2/2: Boundary fingerprint agreement at 12 proof-grade snapshots..."

if ! cargo test --test boundary_fingerprint_agreement boundary_fingerprint_matches_pins -- --nocapture 2>&1; then
    echo "FAIL: boundary fingerprint drift"
    FAILED=1
fi

if [ "$FAILED" -eq 0 ]; then
    echo "PASS: CE-75 differential divergence — verdict agreement (10,500 blocks) + boundary fingerprint agreement (12 snapshots)"
    exit 0
else
    echo "FAIL: CE-75 differential divergence — divergence detected"
    exit 1
fi
