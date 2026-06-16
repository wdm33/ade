#!/usr/bin/env bash
set -uo pipefail

# MEM-OPT-UTXO-DISK S2b (DC-MEM-05): the on-disk redb UTxO anchor is a pure storage
# substitution -- the redb-backed UTxO set yields byte-identical resolved values +
# canonical iteration + canonical UTxO encoding as the in-memory BTreeMap on the
# same per-block deltas. GUARDRAIL (the pre-resolve architecture): the anchor is a
# RED storage backend, NOT a UtxoStore -- BLUE validation consumes a resolved
# in-memory view, never the storage backend, and never causes filesystem I/O.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$REPO_ROOT"
FAILED=0; fail() { echo "FAIL: $1"; FAILED=1; }
A=crates/ade_runtime/src/chaindb/utxo_anchor.rs
S=crates/ade_ledger/src/snapshot/utxo_state.rs

# (1) the redb anchor stores fixed-width-key -> canonical TxOut bytes.
grep -qF 'TableDefinition<&[u8; UTXO_KEY_LEN], &[u8]>' "$A" \
    || fail "anchor table is not (fixed-width key -> bytes)"
grep -qF 'use super::utxo_key::{decode_utxo_key, encode_utxo_key' "$A" \
    || fail "anchor does not use the fixed-width key codec"
grep -qE 'encode_tx_out_canonical|decode_tx_out_canonical' "$A" \
    || fail "anchor does not use the canonical TxOut codec"

# (2) per-block commit is a SINGLE atomic write transaction.
grep -qE 'fn commit_block' "$A" || fail "commit_block (atomic per-block delta apply) missing"
grep -qF 'self.db.begin_write()' "$A" || fail "commit_block is not a redb write transaction"
grep -qE 'txn\.commit\(\)' "$A" || fail "commit_block does not commit the transaction"

# (3) GUARDRAIL: the anchor is NOT a UtxoStore (BLUE never holds the storage backend).
if grep -qE 'impl +(crate::)?(utxo::)?UtxoStore +for +UtxoAnchor' "$A"; then
    fail "UtxoAnchor implements UtxoStore -- the storage backend must NOT be a BLUE-facing resolved view"
fi

# (4) the canonical single-TxOut codec + its fail-closed (trailing-bytes) test.
grep -qE 'pub fn encode_tx_out_canonical' "$S" || fail "encode_tx_out_canonical missing"
grep -qE 'pub fn decode_tx_out_canonical' "$S" || fail "decode_tx_out_canonical missing"
grep -qE 'fn tx_out_canonical_roundtrips_and_rejects_trailing_bytes' "$S" \
    || fail "TxOut codec fail-closed (trailing-bytes) test missing"

# (5) the backend-equivalence proof (DC-MEM-05 storage level) + durability.
grep -qE 'fn redb_anchor_equals_btreemap_across_block_deltas' "$A" \
    || fail "backend-equivalence (redb == BTreeMap) proof missing"
grep -qE 'fn anchor_survives_reopen' "$A" || fail "anchor durability (reopen) proof missing"

# (6) S2b-2c.1a: the anchor stamps its materialized POSITION atomically with the delta
#     (one redb txn) and reconciles to the WAL fail-closed (the roll-forward primitive).
grep -qE 'struct AnchorPosition' "$A" || fail "AnchorPosition (the materialized position marker) is missing"
grep -qF 'meta.insert(ANCHOR_POSITION_KEY' "$A" \
    || fail "commit_block does not stamp the position in the same txn (atomic with the delta)"
grep -qE 'fn read_position' "$A" || fail "read_position (restart reconciliation) is missing"
grep -qE 'fn reconcile' "$A" || fail "reconcile (the WAL roll-forward decision) is missing"
grep -qE 'RecoveryDecision::FailClosed' "$A" || fail "reconcile does not fail closed on ahead/diverged"
grep -qiE 'anchor never leads the WAL|always .{0,3} the WAL' "$A" \
    || fail "the anchor-never-ahead invariant is not documented in the anchor"
grep -qE 'fn commit_stamps_position_atomically_with_the_delta' "$A" \
    || fail "the atomic position+delta proof is missing"
grep -qE 'fn reconcile_decides_consistent_rollforward_and_fail_closed' "$A" \
    || fail "the reconcile-decision proof is missing"

if (( FAILED == 0 )); then
    echo "OK: on-disk redb UTxO anchor (S2b; fixed-width key -> canonical TxOut; atomic delta+position commit; reconcile-to-WAL roll-forward/fail-closed; backend-equivalent to BTreeMap; NOT a UtxoStore)"
fi
exit $FAILED
