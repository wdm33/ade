#!/usr/bin/env bash
set -uo pipefail

# EPOCH-CONSENSUS-VIEW S3e (DC-EVIEW-07): the bound, immutable EpochConsensusView. Emit
# the next-epoch view from the finalized snapshot, BOUND to all of {network, era, epoch,
# source chain point, checkpoint commitment, nonce, snapshot phase, canonical-bytes
# hash}. A view missing/mismatching any binding is INERT (matches() requires all + the
# canonical-hash verify). The canonical encoding round-trips (WAL-recordable, replay-
# equivalent). Observe-only; no live wiring (activation is DC-EVIEW-08).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$REPO_ROOT"
FAILED=0; fail() { echo "FAIL: $1"; FAILED=1; }
V=crates/ade_ledger/src/reduced_epoch_view.rs

test -f "$V" || fail "the EpochConsensusView module ($V) is missing"

# (1) the bound record carries all 8 bindings.
grep -qE 'pub struct EpochConsensusView' "$V" || fail "EpochConsensusView missing"
for f in network_magic era epoch source_point checkpoint_commitment nonce snapshot_phase canonical_hash; do
    grep -qE "$f" "$V" || fail "EpochConsensusView is missing the $f binding"
done

# (2) bind computes the canonical-bytes hash over every binding + the stake distribution.
grep -qE 'pub fn bind' "$V" || fail "the bind constructor missing"
grep -qF 'blake2b_256(&canonical_bytes(' "$V" \
    || fail "bind does not compute the canonical-bytes hash"

# (3) a view is INERT unless ALL bindings match AND the canonical hash verifies.
grep -qE 'pub fn matches' "$V" || fail "matches (binding validation) missing"
grep -qF 'self.verify_canonical_hash()' "$V" \
    || fail "matches does not require verify_canonical_hash (a tampered view must be inert)"
grep -qE 'pub fn verify_canonical_hash' "$V" || fail "verify_canonical_hash missing"

# (4) the canonical encoding round-trips (replay-equivalent / WAL-recordable).
grep -qE 'pub fn canonical_bytes' "$V" || fail "canonical_bytes (round-trippable encoding) missing"

# (5) the load-bearing proofs.
for t in bind_is_deterministic_and_self_verifies matches_exact_bindings_and_rejects_mismatch \
         canonical_hash_is_binding_sensitive canonical_bytes_reproduce_the_hash \
         tampered_view_fails_verification; do
    grep -qE "fn $t" "$V" || fail "the $t proof is missing"
done

# (6) observe-only: no live wiring of the view.
if grep -rqE 'EpochConsensusView' \
    crates/ade_node/src/node_lifecycle.rs crates/ade_node/src/node_sync.rs \
    crates/ade_runtime/src/admission/ crates/ade_runtime/src/forward_sync/ 2>/dev/null; then
    fail "EpochConsensusView is referenced on the live path -- S3e is observe-only (activation is DC-EVIEW-08)"
fi

if (( FAILED == 0 )); then
    echo "OK: EpochConsensusView binding (DC-EVIEW-07; 8-binding immutable view, canonical-hash identity, inert-unless-bound+verified, round-trippable; observe-only)"
fi
exit $FAILED
