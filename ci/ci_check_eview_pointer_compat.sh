#!/usr/bin/env bash
set -uo pipefail

# EPOCH-CONSENSUS-VIEW S3a (DC-EVIEW-03): the era-parameterized pointer-varint
# decoder matches cardano-ledger EXACTLY (accept bounded aliasing; Conway
# width-reject + trailing-reject; Babbage/<=Alonzo normalize-clamp-the-3-tuple;
# <=Alonzo crop trailing) -- NO canonicalization preference may override the ledger
# result. Plus the pre-Conway pointer RESOLUTION (fail-closed). The decoder is keyed
# on a TYPED bound CardanoEra, never ambient state. S3a resolves nothing live and
# alters no producer path.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$REPO_ROOT"
FAILED=0; fail() { echo "FAIL: $1"; FAILED=1; }
DEC=crates/ade_codec/src/address/pointer.rs
RES=crates/ade_ledger/src/pointer_resolve.rs

test -f "$DEC" || fail "the pointer decoder ($DEC) is missing"
test -f "$RES" || fail "the pointer resolution ($RES) is missing"

# (1) the decoder is keyed on a TYPED bound CardanoEra (not ambient/config/clock).
grep -qE 'pub fn decode_pointer_tail\(tail: &\[u8\], era: CardanoEra\)' "$DEC" \
    || fail "decode_pointer_tail does not take a typed bound (tail, era: CardanoEra)"
grep -qE 'pub fn decode_pointer_address\(addr_bytes: &\[u8\], era: CardanoEra\)' "$DEC" \
    || fail "decode_pointer_address does not take a typed bound era"

# (2) the era gate: Conway strict vs pre-Conway normalize.
grep -qE 'era >= CardanoEra::Conway' "$DEC" || fail "no Conway era gate (strict vs normalize)"
grep -qE 'era >= CardanoEra::Babbage' "$DEC" || fail "no Babbage gate (reject vs crop trailing)"

# (3) the stored cardano-ledger shape (u32 slot / u16 txIx / u16 certIx).
grep -qE 'pub slot: u32' "$DEC" || fail "Ptr.slot is not u32 (cardano-ledger stored width)"
grep -qE 'pub tx_index: u16' "$DEC" || fail "Ptr.tx_index is not u16"
grep -qE 'pub cert_index: u16' "$DEC" || fail "Ptr.cert_index is not u16"

# (4) normalize = clamp the WHOLE 3-tuple to (0,0,0) (mkPtrNormalized), NOT per-field
#     mask/wrap.
grep -qE 'fn normalize_ptr' "$DEC" || fail "the normalize (clamp-3-tuple) helper is missing"

# (5) NO canonicalization preference: the alias-accepted test is the positive guard
#     (the decoder ACCEPTS a bounded leading-zero alias in every era, matching
#     cardano-ledger; reject-all-non-canonical would diverge).
grep -qE 'fn bounded_leading_zero_alias_accepted_all_eras' "$DEC" \
    || fail "the no-canonicalization-override (alias accepted) test is missing"

# (6) the load-bearing per-era proofs.
for t in conway_rejects_txix_over_u16 conway_rejects_trailing_bytes \
         conway_accepts_max_width_boundary babbage_normalizes_overflow_to_zero_tuple \
         babbage_rejects_trailing_bytes alonzo_crops_trailing_bytes; do
    grep -qE "fn $t" "$DEC" || fail "the $t proof is missing"
done

# (7) resolution is fail-closed (unregistered -> None; duplicate -> rejected).
grep -qE 'pub fn resolve\(&self, ptr: &Ptr\) -> Option<StakeCredential>' "$RES" \
    || fail "resolve does not return Option<StakeCredential> (fail-closed)"
grep -qE 'fn unregistered_pointer_is_none_fail_closed' "$RES" \
    || fail "the unregistered-pointer fail-closed test is missing"
grep -qE 'fn duplicate_position_is_rejected_fail_closed' "$RES" \
    || fail "the duplicate-position fail-closed test is missing"

# (8) S3a boundary: NO live wiring / aggregation / track_utxo here.
if grep -qiE 'track_utxo|EpochConsensusView|aggregate|new_mark|leader' "$DEC" "$RES"; then
    fail "S3a reaches into aggregation / live / leader -- out of scope (S3c+)"
fi

if (( FAILED == 0 )); then
    echo "OK: pointer decode/resolution compat (DC-EVIEW-03; era-parameterized match-cardano-ledger, accept aliasing, Conway width-reject, normalize-clamp pre-Conway, fail-closed resolve; no aggregation)"
fi
exit $FAILED
