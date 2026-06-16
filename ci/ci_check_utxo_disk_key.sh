#!/usr/bin/env bash
set -uo pipefail

# MEM-OPT-UTXO-DISK S2b (DC-MEM-06): the on-disk UTxO anchor's storage key is a
# FIXED-WIDTH txid[32] || BE-u32(index), whose byte-sorted order equals canonical
# TxIn order BY CONSTRUCTION -- never relying on CBOR array/integer-width layout.
# Project-internal STORAGE canonical, NOT a Cardano protocol/hash encoding. This is
# the S2b codec foundation; the redb anchor + backend-equivalence corpus is next.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$REPO_ROOT"
FAILED=0; fail() { echo "FAIL: $1"; FAILED=1; }
K=crates/ade_runtime/src/chaindb/utxo_key.rs

# (1) the fixed-width key codec exists with the txid[32] || BE-u32 form.
grep -qF 'const UTXO_KEY_LEN: usize = 36' "$K" || fail "UTXO_KEY_LEN is not 36 (txid[32] || BE-u32)"
grep -qE 'fn encode_utxo_key' "$K" || fail "encode_utxo_key missing"
grep -qF 'key[..32].copy_from_slice(&tx_in.tx_hash.0)' "$K" || fail "key does not lead with the 32-byte txid"
grep -qF 'key[32..].copy_from_slice(&(tx_in.index as u32).to_be_bytes())' "$K" || fail "index is not BE-u32"
grep -qE 'fn decode_utxo_key' "$K" || fail "decode_utxo_key missing"

# (2) the proof gates the slice requires (order==Ord, roundtrip, fail-closed decode).
grep -qE 'fn fixed_width_key_order_matches_txin_ord' "$K" || fail "key-order==TxIn::Ord proof missing"
grep -qE 'fn key_roundtrip_is_identity' "$K" || fail "decode(encode(txin))==txin roundtrip proof missing"
grep -qE 'fn malformed_key_length_rejected_deterministically' "$K" || fail "malformed-length reject proof missing"
grep -qE 'fn index_out_of_u16_domain_rejected' "$K" || fail "out-of-u16-domain reject proof missing"

# (3) the authority wording: the key is storage-internal, NOT a protocol/hash encoding.
grep -qiE 'not.*a Cardano protocol encoding|never used for hashing' "$K" \
    || fail "the 'storage-internal, not a protocol/hash encoding' authority note is missing"

if (( FAILED == 0 )); then
    echo "OK: fixed-width UTxO storage key (S2b; txid[32] || BE-u32; order==TxIn::Ord proven; roundtrip + fail-closed decode; storage-internal, not a protocol encoding)"
fi
exit $FAILED
