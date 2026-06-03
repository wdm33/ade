#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-F-G-J S2 (CN-WIRE-09): the header_body `prev_hash` field is the closed wire
# grammar `$hash32 / null`, with exactly ONE codec authority (ade_codec ShelleyHeaderBody),
# and the `null` (Genesis) grammar is scoped to that header_body field ONLY — it never leaks
# into the chain-sync / block-fetch Point/Tip codec (which stays hash32 / Point::Origin).
#
#   (a) PrevHash is defined once (ade_types); `decode_prev_hash` + the Genesis->null encode
#       arm live in exactly one file (ade_codec/shelley/block.rs) — no parallel/second
#       header_body prev_hash codec.
#   (b) the null/Genesis grammar does not leak into the ade_network Point/Tip codec; the
#       genesis point stays `Point::Origin` (array(0)), not a null-bearing hash field.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CODEC="$REPO_ROOT/crates/ade_codec/src/shelley/block.rs"
TYPE="$REPO_ROOT/crates/ade_types/src/shelley/block.rs"
NET_CODEC_DIR="$REPO_ROOT/crates/ade_network/src/codec"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

for f in "$CODEC" "$TYPE"; do
    [[ -f "$f" ]] || print_fail "missing expected source $f"
done
if (( FAILED != 0 )); then echo "FAIL: ci_check_prevhash_single_wire_authority"; exit 1; fi

# --- (a) single type + single codec authority.
grep -qE 'enum PrevHash' "$TYPE" || print_fail "the PrevHash sum type is not defined in ade_types ($TYPE)"

DEF_FILES="$(grep -rlE 'fn decode_prev_hash' "$REPO_ROOT/crates" --include=*.rs | sort)"
if [[ "$DEF_FILES" != "$CODEC" ]]; then
    print_fail "decode_prev_hash must be defined in EXACTLY one file (ade_codec/shelley/block.rs); found: ${DEF_FILES:-<none>}"
fi

ENC_FILES="$(grep -rlE 'PrevHash::Genesis *=> *cbor::write_null' "$REPO_ROOT/crates" --include=*.rs | sort)"
if [[ "$ENC_FILES" != "$CODEC" ]]; then
    print_fail "the Genesis->CBOR-null header_body encode arm must live only in ade_codec/shelley/block.rs; found: ${ENC_FILES:-<none>}"
fi

# --- (b) the null/Genesis prev_hash grammar must NOT leak into the Point/Tip codec.
if [[ -d "$NET_CODEC_DIR" ]]; then
    LEAK="$(grep -rnE 'PrevHash' "$NET_CODEC_DIR" --include=*.rs 2>/dev/null | head -n1)"
    if [[ -n "$LEAK" ]]; then
        print_fail "PrevHash leaked into the ade_network Point/Tip codec (the null prev_hash grammar is header_body-only): $LEAK"
    fi
    CHAIN_SYNC="$NET_CODEC_DIR/chain_sync.rs"
    if [[ -f "$CHAIN_SYNC" ]] && ! grep -qE 'Point::Origin' "$CHAIN_SYNC"; then
        print_fail "Point::Origin missing from the chain_sync codec — the genesis point must stay array(0), never a null-bearing hash"
    fi
fi

if (( FAILED == 0 )); then
    echo "OK: CN-WIRE-09 — one header_body prev_hash codec authority (ade_codec/shelley/block.rs), PrevHash type in ade_types; the null/Genesis grammar does not leak into the Point/Tip codec (Point::Origin stays array(0))."
fi
exit $FAILED
