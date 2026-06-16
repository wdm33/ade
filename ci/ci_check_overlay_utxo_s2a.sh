#!/usr/bin/env bash
set -uo pipefail

# MEM-OPT-UTXO-DISK S2a (CE-UD-2, DC-MEM-07 partial): the UTxO set is an overlay-
# backed store -- an Arc-shared anchor + a BOUNDED in-memory overlay -- so a clone
# is O(overlay) and a mutation is an overlay append (NEVER a full-map clone). The
# anchor is still in memory here (S2a de-risks the clone-model change); the on-disk
# redb anchor is S2b. A static assertion over the live tree.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$REPO_ROOT"
FAILED=0; fail() { echo "FAIL: $1"; FAILED=1; }
OV=crates/ade_ledger/src/utxo_overlay.rs
U=crates/ade_ledger/src/utxo.rs
FP=crates/ade_ledger/src/fingerprint.rs

# (1) the overlay representation: an Arc-shared anchor + a tombstoned overlay.
grep -qE 'pub struct OverlayUtxo' "$OV" || fail "OverlayUtxo type is missing"
grep -qE 'anchor: Arc<BTreeMap<TxIn, TxOut>>' "$OV" \
    || fail "OverlayUtxo anchor is not the Arc-shared BTreeMap (cheap clone)"
grep -qE 'overlay: BTreeMap<TxIn, Option<TxOut>>' "$OV" \
    || fail "OverlayUtxo overlay is not the tombstoned diff map (Some=insert/None=delete)"

# (2) the overlay is BOUNDED with compaction (DC-MEM-07 partial -- the in-memory
#     diff cannot grow without folding into the anchor).
grep -qE 'pub const MAX_OVERLAY_ENTRIES' "$OV" || fail "the overlay bound (MAX_OVERLAY_ENTRIES) is missing"
grep -qE 'fn compact\(&mut self\)' "$OV" || fail "overlay compaction (fold into anchor) is missing"

# (3) UTxOState now holds the OverlayUtxo, NOT a raw BTreeMap -- the representation
#     change that makes the clone cheap.
grep -qE 'pub utxos: OverlayUtxo' "$U" || fail "UTxOState.utxos is not the OverlayUtxo store"
if grep -qE 'pub utxos: BTreeMap<TxIn, TxOut>' "$U"; then
    fail "UTxOState.utxos is still a raw BTreeMap (the S2a representation change did not land)"
fi

# (4) mutation is overlay-append: utxo_insert/utxo_delete clone the store (cheap)
#     and insert/remove on it; they must NOT rebuild a fresh BTreeMap per mutation.
grep -qE 'let mut new_store = state\.utxos\.clone\(\);' "$U" \
    || fail "utxo_insert/utxo_delete no longer clone the overlay store (O(overlay))"
if grep -qE 'let mut new_utxos = state\.utxos\.clone\(\);' "$U"; then
    fail "utxo_insert/utxo_delete still clone a raw BTreeMap (O(n) per mutation)"
fi

# (5) the validation chain resolves through the abstract seam (backend-agnostic):
#     the era validators take &impl UtxoStore, membership goes via UtxoMembership.
grep -qE 'pub trait UtxoMembership' "$U" || fail "the UtxoMembership seam is missing"
grep -qE 'pub trait UtxoStore: UtxoMembership' "$U" \
    || fail "UtxoStore is not a UtxoMembership supertrait"
for f in alonzo babbage conway plutus_eval; do
    grep -qE 'utxo: &impl crate::utxo::UtxoStore' "crates/ade_ledger/src/$f.rs" \
        || fail "$f.rs validator does not resolve through &impl UtxoStore"
done
grep -qE 'utxo: &impl crate::utxo::UtxoMembership' crates/ade_ledger/src/late_era_validation.rs \
    || fail "check_inputs_present does not use the UtxoMembership seam"

# (6) the replay-equivalence proofs exist: the overlay matches a BTreeMap across a
#     sequence, and an overlay-split state fingerprints identically to a direct build.
grep -qE 'fn overlay_matches_btreemap_across_a_sequence' "$OV" \
    || fail "the overlay/BTreeMap equivalence proof is missing"
grep -qE 'fn s2a_overlay_split_fingerprints_identically_to_direct_build' "$FP" \
    || fail "the overlay-split fingerprint-equivalence proof is missing"

# (7) S2a is IN-MEMORY only -- the on-disk redb anchor is S2b. No redb in the BLUE
#     ledger crate yet.
if grep -qE '^\s*redb\b' crates/ade_ledger/Cargo.toml; then
    fail "ade_ledger now depends on redb -- that is S2b (on-disk anchor), not S2a"
fi

if (( FAILED == 0 )); then
    echo "OK: overlay UTxO store (S2a; Arc anchor + bounded overlay; O(overlay) clone; &impl UtxoStore seam; replay-equivalent; in-memory only)"
fi
exit $FAILED
