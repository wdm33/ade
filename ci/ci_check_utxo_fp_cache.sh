#!/usr/bin/env bash
set -uo pipefail

# MEM-OPT-UTXO-DISK S2b-2c.1b-A.1: cache the UTxO-component fingerprint so the live
# track_utxo=false admission skips the full per-block UTxO scan (the S0 churn). The
# cache keys on a content-generation that bumps on every mutation, so it can NEVER
# serve a stale fingerprint -- post_fp stays byte-identical (replay-equivalent). This
# slice does NOT enable full live UTxO application (that is slice B).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$REPO_ROOT"
FAILED=0; fail() { echo "FAIL: $1"; FAILED=1; }
OV=crates/ade_ledger/src/utxo_overlay.rs
FP=crates/ade_ledger/src/fingerprint.rs
RN=crates/ade_node/src/admission/runner.rs
DOC=docs/clusters/MEM-OPT-UTXO-DISK/S2b-2cA-static-utxo.md

# (1) the generation counter: a field, bumped on insert + remove, with an accessor.
grep -qE 'generation: u64' "$OV" || fail "OverlayUtxo has no generation counter"
grep -qF 'fn generation(&self) -> u64' "$OV" || fail "generation() accessor missing"
test "$(grep -c 'self.generation = self.generation.wrapping_add(1)' "$OV")" -ge 2 \
    || fail "insert/remove do not both bump the generation"

# (2) the precomputed-utxo fingerprint variant + the cache (keyed on the generation).
grep -qE 'pub fn fingerprint_v2_with_utxo' "$FP" || fail "fingerprint_v2_with_utxo missing"
grep -qE 'pub fn fingerprint_utxo_v2' "$FP" || fail "fingerprint_utxo_v2 not public (the cache needs it)"
grep -qE 'pub struct UtxoFpCache' "$FP" || fail "UtxoFpCache missing"
grep -qF 'utxo_state.utxos.generation()' "$FP" || fail "the cache does not key on the UTxO generation"

# (3) the live admission post_fp USES the cache and no longer does the full scan.
grep -qF 'utxo_fp_cache.utxo_fingerprint(' "$RN" || fail "the admission post_fp does not use the cache"
grep -qF 'fingerprint_v2_with_utxo(&next_ledger, utxo_fp)' "$RN" \
    || fail "the admission post_fp does not use the precomputed-utxo fingerprint"
if grep -qE 'let post_fp = fingerprint\(&next_ledger\)' "$RN"; then
    fail "the admission still does the full fingerprint() UTxO scan per block"
fi

# (4) live behavior unchanged: the admission does NOT enable track_utxo=true (A keeps
#     the static-UTxO path; B = LIVE-LEDGER-APPLY is a deliberate later slice).
if grep -rqE 'track_utxo *= *true|track_utxo: *true' crates/ade_node/src/admission/; then
    fail "the live admission enables track_utxo=true -- that is slice B, not A"
fi

# (5) the proofs.
grep -qE 'fn fingerprint_v2_with_precomputed_utxo_equals_full_v2' "$FP" \
    || fail "the variant==full proof missing"
grep -qE 'fn utxo_fp_cache_reuses_while_unchanged_and_recomputes_on_change' "$FP" \
    || fail "the cache reuse/invalidate proof missing"

# (6) the HONEST guardrail doc: A does not enable full live application; B is owed.
test -f "$DOC" || fail "the S2b-2c-A honest-scope doc is missing"
grep -qiE 'does NOT enable full live UTxO application' "$DOC" \
    || fail "the doc does not state A is NOT full live application"
grep -qiE 'remains OWED' "$DOC" \
    || fail "the doc does not state B (full live validation) remains owed"

# (7) S2b-2c.1b-A.2: the EXPLICIT StaticUtxoFp (NOT generation magic) -- the bootstrap
#     constant fp, computed once, fail-closed under track_utxo=true / version mismatch.
grep -qE 'pub struct StaticUtxoFp' "$FP" || fail "StaticUtxoFp missing"
grep -qE 'valid_only_when_track_utxo_false' "$FP" || fail "StaticUtxoFp lacks the track_utxo=false guard field"
grep -qE 'bootstrap_anchor' "$FP" || fail "StaticUtxoFp lacks the bootstrap_anchor"
grep -qE 'fn from_bootstrap_utxo' "$FP" || fail "StaticUtxoFp::from_bootstrap_utxo (compute-once) missing"
grep -qE 'UsedUnderTrackUtxoTrue' "$FP" || fail "StaticUtxoFp does not fail closed under track_utxo=true"
grep -qE 'fn static_utxo_fp_fails_closed_under_track_utxo_true' "$FP" \
    || fail "the StaticUtxoFp fail-closed proof missing"

if (( FAILED == 0 )); then
    echo "OK: cached UTxO fingerprint (S2b-2c.1b-A.1; generation-keyed, never stale; live post_fp skips the per-block scan; track_utxo=false unchanged; B owed)"
fi
exit $FAILED
