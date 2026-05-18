#!/usr/bin/env bash
set -euo pipefail

# Run the recovery-contract test suite.
#
# Enforces that ade_runtime::recovery::recover() produces a
# replay-equivalent state regardless of starting condition:
#   - From genesis when no snapshot present
#   - From most-recent snapshot + forward replay (DC-STORE-05)
#   - Pre-snapshot blocks correctly skipped during forward replay
#   - Mid-replay errors surface with the failing block's slot
#   - No partial-recovery success (recovery is all-or-nothing)
#
# Authoritative invariants:
#   - T-REC-01: Recovery is replay-equivalent
#   - T-REC-02: All authoritative state derivable by replay from inputs
#   - DC-STORE-05: Recovery is snapshot + forward replay, not full
#     genesis replay

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

if ! cargo test -p ade_runtime --lib recovery:: --quiet 2>&1 | tail -20; then
    echo "FAIL: recovery contract suite failed"
    exit 1
fi

echo "PASS: recovery contract suite (T-REC-01/02, DC-STORE-05)"
