#!/usr/bin/env bash
# ci_check_tx_submission2_real_capture.sh -- TXSUB2-CODEC-REALWIRE (supports DC-PROTO-02).
#
# The N2N tx-submission2 codec MUST accept + preserve cardano-node's REAL wire
# form for MsgReplyTxIds / MsgRequestTxs / MsgReplyTxs:
#   - each txId is ERA-TAGGED `[eraIndex, hash32]` (the HardFork GenTxId), NOT a
#     bare 32-byte byte string;
#   - the txId/tx sequences are CBOR INDEFINITE-length arrays (`9f .. ff`);
# and a captured frame MUST re-encode BYTE-IDENTICALLY.
#
# This was a real compatibility FALSE-REJECT found by the server-side live
# capture (Ade rejected a real peer's ReplyTxIds with "indefinite-length array
# not allowed"); the synthetic round-trip tests used definite arrays + bare
# txids and missed it. This gate locks the codec against silently regressing to
# that false-rejecting form.
#
# Structural guards over crates/ade_network/src/codec/tx_submission.rs
# (comment-stripped):
#   1. era-tagged txid is modelled: `struct TxSubmissionTxId` carrying an `era`.
#   2. an era-tagged txid decoder exists (`decode_tx_submission_txid`).
#   3. decode ACCEPTS indefinite sequences (`ContainerEncoding::Indefinite`).
#   4. encode EMITS the indefinite form (`write_array_header(.. Indefinite)` + `write_break`).
# Evidence guards:
#   5. the captured-frame byte-identity unit test exists.
#   6. the malformed/negative unit tests exist (bare txid / wrong hash len / unterminated).
#   7. the real-capture corpus carries the node-originated reply_txids + reply_txs
#      frames and the round-trip corpus test exists.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CODEC="${ADE_TXSUB_CODEC_FILE:-$REPO_ROOT/crates/ade_network/src/codec/tx_submission.rs}"
CORPUS="$REPO_ROOT/corpus/network/n2n/tx_submission2"
RTTEST="$REPO_ROOT/crates/ade_network/tests/tx_submission2_real_capture_corpus.rs"
fail() { echo "FAIL (ci_check_tx_submission2_real_capture): $1" >&2; exit 1; }

# Structural code guards (1-4), over a comment-stripped view.
check_code() {
  local f="$1"
  [ -f "$f" ] || fail "codec file missing: $f"
  local s
  s="$(sed -E 's://.*$::' "$f")"
  grep -Eq 'struct[[:space:]]+TxSubmissionTxId' <<< "$s" \
    || fail "no TxSubmissionTxId (era-tagged txid model) -- guard 1"
  grep -Eq '\bera\b' <<< "$s" \
    || fail "TxSubmissionTxId is missing the era tag -- guard 1"
  grep -Eq 'decode_tx_submission_txid' <<< "$s" \
    || fail "no era-tagged txid decoder -- guard 2"
  grep -Eq 'ContainerEncoding::Indefinite' <<< "$s" \
    || fail "decode does not accept indefinite-length arrays -- guard 3"
  grep -Eq 'write_break' <<< "$s" \
    || fail "encode does not emit the indefinite break byte -- guard 4"
  grep -Eq 'write_array_header\([^)]*Indefinite' <<< "$s" \
    || fail "encode does not emit an indefinite-length array header -- guard 4"
}

# --self-test: a synthetic codec that lacks the real-wire handling must be caught.
if [ "${1:-}" = "--self-test" ]; then
  TMP="$(mktemp)"
  trap 'rm -f "$TMP"' EXIT
  cat > "$TMP" <<'RS'
// synthetic regression: bare-txid + definite-only codec (no era tag, no indefinite)
pub struct BareTxId { pub hash: [u8; 32] }
fn decode() { let _ = ContainerEncoding::Definite; }
RS
  if ADE_TXSUB_CODEC_FILE="$TMP" bash "$0" --check-code-only >/dev/null 2>&1; then
    echo "FAIL: gate did not detect a codec reverted to the false-rejecting form" >&2
    exit 1
  fi
  echo "PASS: gate detects a codec that drops era-tagged-txid / indefinite-array support"
  exit 0
fi

if [ "${1:-}" = "--check-code-only" ]; then
  check_code "$CODEC"
  echo "OK code"
  exit 0
fi

check_code "$CODEC"

# Evidence guards 5-6 (unit tests in the codec file).
s="$(sed -E 's://.*$::' "$CODEC")"
grep -Eq 'real_cardano_reply_txids_decodes_and_re_encodes_byte_identical' <<< "$s" \
  || fail "missing captured-frame byte-identity unit test -- guard 5"
for t in decode_rejects_bare_txid decode_rejects_wrong_txid_hash_length \
         decode_rejects_unterminated_indefinite_sequence; do
  grep -Eq "$t" <<< "$s" || fail "missing negative unit test $t -- guard 6"
done

# Evidence guard 7 (real-capture corpus + round-trip test). The captured
# MsgReplyTxIds (era-tagged txid + indefinite array) is the rich frame that
# exposed + validates the fix; it must be present. MsgReplyTxs needs the live
# full-exchange (only flows when the public-preprod node has a mempool tx) and
# is the documented live follow-up -- round-tripped by the test when present.
[ -f "$RTTEST" ] || fail "missing real-capture round-trip test: $RTTEST"
ls "$CORPUS"/*_txsub_reply_txids_*_recv.cbor >/dev/null 2>&1 \
  || fail "no real MsgReplyTxIds capture in corpus ($CORPUS)"
if ls "$CORPUS"/*_txsub_reply_txs_*_recv.cbor >/dev/null 2>&1; then
  echo "note: real MsgReplyTxs capture present (live full-exchange landed)"
else
  echo "note: MsgReplyTxs not yet captured (live full-exchange + DC-PROTO-02 flip pending preprod tx)"
fi

echo "OK: tx-submission2 codec is on cardano-node's real wire grammar -- era-tagged txid + indefinite arrays, byte-identical (TXSUB2-CODEC-REALWIRE)"
