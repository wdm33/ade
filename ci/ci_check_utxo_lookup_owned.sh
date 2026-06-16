#!/usr/bin/env bash
set -uo pipefail

# MEM-OPT-UTXO-DISK S1 (CE-UD-1, DC-MEM-09): the authoritative UTxO lookup
# interface returns OWNED values (Option<TxOut>), never a borrow into storage --
# the precondition for a swappable on-disk backend (S2 / DC-MEM-05). A static
# assertion over the live tree. Interface-prep ONLY: NO redb/on-disk backend yet.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$REPO_ROOT"
FAILED=0; fail() { echo "FAIL: $1"; FAILED=1; }
U=crates/ade_ledger/src/utxo.rs

# (1) utxo_lookup returns owned Option<TxOut>, NOT a borrow.
grep -qE 'fn utxo_lookup\(.*\) -> Option<TxOut>' "$U" \
    || fail "utxo_lookup is not the owned form (-> Option<TxOut>)"
if grep -qE 'fn utxo_lookup.*-> Option<&' "$U"; then
    fail "utxo_lookup still returns a borrow (-> Option<&...>)"
fi

# (2) the UtxoStore seam exists with an owned get.
grep -qE 'trait UtxoStore' "$U" || fail "the UtxoStore seam (swappable-backend) is missing"
grep -qE 'fn get\(&self, tx_in: &TxIn\) -> Option<TxOut>' "$U" \
    || fail "UtxoStore::get is not the owned form (-> Option<TxOut>)"

# (3) the production input-resolution sites route through the owned utxo_lookup
#     (no raw .utxos.get() borrow in apply_phase_2_failure / phase1 signers).
grep -qE 'utxo_lookup\(&state\.utxo_state, tx_in\)' crates/ade_ledger/src/phase.rs \
    || fail "phase.rs apply_phase_2_failure does not use the owned utxo_lookup"
grep -qE 'utxo_lookup\(&ledger\.utxo_state, input\)' crates/ade_ledger/src/tx_validity/phase1.rs \
    || fail "phase1.rs required-signers resolution does not use the owned utxo_lookup"

# (4) S1 is interface-prep ONLY -- no redb/on-disk backend in the BLUE ledger crate.
if grep -qE '^\s*redb\b' crates/ade_ledger/Cargo.toml; then
    fail "ade_ledger now depends on redb -- that is S2 (on-disk storage), not S1"
fi

if (( FAILED == 0 )); then
    echo "OK: utxo_lookup owned interface (S1 interface-prep; UtxoStore seam; BTreeMap-only; no storage)"
fi
exit $FAILED
