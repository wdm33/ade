#!/usr/bin/env bash
set -uo pipefail

# DC-MEM-04 (PHASE4-N-E S2): replaying the same ordered ingress trace
# against the same base ledger state produces a byte-identical sequence
# of (MempoolState, AdmitOutcome) pairs. This gate is a structural
# defense — the byte-identical property itself is proven by the
# integration tests under `crates/ade_testkit/tests/mempool_ingress_replay.rs`.
# The script verifies:
#
#   1. The GREEN harness module exists and exports the four items
#      (wrap_as_ingress, b_track_corpus_as_ingress, replay_ingress_trace,
#      BTrackCase + ExpectedOutcome).
#   2. The harness's `replay_ingress_trace` body uses `mempool_ingress`
#      (the BLUE bridge) — NOT direct `admit` calls — so the replay is
#      genuinely a fold over the BLUE chokepoint.
#   3. The four load-bearing test names referenced by the registry's
#      DC-MEM-04.tests array exist in the integration test file.
#   4. The harness contains no batching / out-of-order helpers — the
#      replay is strictly single-step per OQ-6 (`fold`, not `partition`
#      or `chunk`).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

HARNESS_MOD="$REPO_ROOT/crates/ade_testkit/src/mempool/mod.rs"
HARNESS="$REPO_ROOT/crates/ade_testkit/src/mempool/ingress_replay.rs"
TESTS="$REPO_ROOT/crates/ade_testkit/tests/mempool_ingress_replay.rs"
LIB="$REPO_ROOT/crates/ade_testkit/src/lib.rs"

FAIL=0

# 1. files exist
for f in "$HARNESS_MOD" "$HARNESS" "$TESTS" "$LIB"; do
    [ -f "$f" ] || { echo "FAIL: missing $f"; FAIL=1; }
done
[ "$FAIL" -eq 0 ] || exit 1

# 1a. lib.rs registers the mempool submodule.
if ! grep -qE '^pub mod mempool;' "$LIB"; then
    echo "FAIL: $LIB does not declare 'pub mod mempool;'"
    FAIL=1
fi

# 1b. harness exports the required items via mod.rs.
for sym in 'wrap_as_ingress' 'b_track_corpus_as_ingress' 'replay_ingress_trace' 'BTrackCase' 'ExpectedOutcome'; do
    if ! grep -q "${sym}" "$HARNESS_MOD"; then
        echo "FAIL: $HARNESS_MOD does not re-export ${sym}"
        FAIL=1
    fi
done

# 1c. harness defines the required items.
for sym in 'pub fn wrap_as_ingress' 'pub fn b_track_corpus_as_ingress' 'pub fn replay_ingress_trace' 'pub struct BTrackCase' 'pub enum ExpectedOutcome'; do
    if ! grep -qE "^${sym}" "$HARNESS"; then
        echo "FAIL: $HARNESS missing required item: ${sym}"
        FAIL=1
    fi
done

# 2. replay_ingress_trace uses mempool_ingress (the BLUE bridge), not direct admit.
if ! grep -qE 'mempool_ingress\(' "$HARNESS"; then
    echo "FAIL: $HARNESS does not call mempool_ingress (the BLUE bridge)"
    FAIL=1
fi
# Heuristic: between `pub fn replay_ingress_trace(` and the next top-level `}`,
# no direct `admit(` calls — the replay must go through the bridge.
BODY=$(awk '/^pub fn replay_ingress_trace\(/,/^}$/' "$HARNESS" || true)
if [ -n "$BODY" ] && echo "$BODY" | grep -qE '[^A-Za-z_]admit\('; then
    echo "FAIL: replay_ingress_trace body calls admit() directly — must go through mempool_ingress"
    echo "$BODY" | grep -nE 'admit\('
    FAIL=1
fi

# 3. required test names exist in the integration test file (matches the
#    registry DC-MEM-04.tests array).
for t in ingress_admit_equals_direct_admit_on_b_track_corpus \
         b_track_adversarial_rejections_preserved_through_ingress \
         ingress_trace_replay_byte_identical \
         dependent_pair_through_ingress_admits_b_after_a \
         ingress_trace_source_invariant_n2n_vs_n2c; do
    if ! grep -qE "fn ${t}\b" "$TESTS"; then
        echo "FAIL: $TESTS missing required test function: ${t}"
        FAIL=1
    fi
done

# 4. no batching / out-of-order helpers in the harness — the replay must
#    remain single-step per OQ-6.
if grep -qE '\b(chunks|chunks_exact|partition|par_iter|rayon|tokio::spawn)\b' "$HARNESS"; then
    echo "FAIL: $HARNESS contains batching/parallel helpers — replay must remain single-step (OQ-6)"
    grep -nE '\b(chunks|chunks_exact|partition|par_iter|rayon|tokio::spawn)\b' "$HARNESS"
    FAIL=1
fi

if [ "$FAIL" -eq 0 ]; then
    echo "PASS: DC-MEM-04 mempool ingress replay (harness exports + bridge usage + test surface + single-step fold)"
fi
exit "$FAIL"
