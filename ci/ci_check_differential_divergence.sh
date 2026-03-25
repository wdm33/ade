#!/usr/bin/env bash
set -euo pipefail

# CE-75: Verify zero differential divergence on non-Plutus blocks.
#
# Runs the differential ledger harness on the expanded corpus
# (1,500+ blocks per era, 10,500 total across all 7 eras).
# Reports zero divergence on all non-Plutus-dependent blocks.
#
# Version-scoped to cardano-node 10.6.2.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

echo "=== CE-75: Differential Divergence ==="
echo "Running 10,500-block replay across all 7 eras..."

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

if [ "$FAILED" -eq 0 ]; then
    echo "PASS: CE-75 differential divergence — zero divergence across 10,500 blocks (7 eras)"
    exit 0
else
    echo "FAIL: CE-75 differential divergence — divergence detected"
    exit 1
fi
