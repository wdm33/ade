#!/usr/bin/env bash
set -euo pipefail

# Run the chaindb crash-safety smoke harness.
#
# Subprocess-driven SIGKILL fault injection: child writes; parent
# kills at a random offset; parent reopens and verifies invariants
# (no Corruption error, schema version intact, tip consistent, slot
# and hash indices coherent, full slot-iter completes without error).
#
# CI variant: stress_kill_smoke (10 iterations, fast — sub-second).
# Closure-gate variant: stress_kill_1000 (1000 iterations, marked
# #[ignore], run manually via:
#   cargo test -p ade_runtime --test stress_kill_harness stress_kill_1000 \
#       --release -- --ignored --nocapture
# ). Most recent gate-run log lives under target/ce-evidence/.
#
# Authoritative invariants:
#   - T-REC-01: Recovery is replay-equivalent (crash variant)
#   - DC-STORE-01: Recovery from power-loss produces replay-
#     equivalent state
#   - CN-STORE-03: Crash recovery must produce the same authoritative
#     state as clean replay
#   - Mechanical acceptance criterion CE-N-D-1

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

if ! cargo test -p ade_runtime --test stress_kill_harness stress_kill_smoke --quiet 2>&1 | tail -10; then
    echo "FAIL: chaindb crash-safety smoke failed"
    exit 1
fi

if ! cargo test -p ade_runtime --test stress_kill_harness snapshot_table_intact_after_kill_loop --quiet 2>&1 | tail -10; then
    echo "FAIL: snapshot-table-after-kill failed"
    exit 1
fi

echo "PASS: chaindb crash-safety smoke (T-REC-01, DC-STORE-01, CN-STORE-03)"
