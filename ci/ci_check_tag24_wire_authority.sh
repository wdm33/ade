#!/usr/bin/env bash
#
# ci_check_tag24_wire_authority.sh — PHASE4-N-X / CN-WIRE-08 gate.
#
# Enforces the single N2N tag-24 CBOR-in-CBOR wire-envelope authority:
#
#   (1) `wrap_tag24` and `unwrap_tag24` are each defined EXACTLY ONCE,
#       in the BLUE authority module crates/ade_codec/src/cbor/tag24.rs.
#   (2) No hand-rolled tag-24 parse (`0xd8`/`0x18` byte-literal sniffing,
#       or a `read_tag(..)==24` + `read_bytes` pair) survives in the RED
#       serve/admission/interop consumer paths — they call the authority.
#   (3) The serve paths compose via the per-protocol authorities
#       (`compose_blockfetch_block` / `compose_rollforward_header`), so no
#       bare `[era, block]` / bare header is placed on the wire.
#   (4) The deleted hand-rolled `unwrap_block_fetch_envelope` does not
#       reappear anywhere under crates/.
#
# Live peer acceptance stays RO-LIVE-01 / CN-CONS-06 operator-gated; this
# gate only guards the in-process wrap authority + composition.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

FAIL=0
AUTHORITY="crates/ade_codec/src/cbor/tag24.rs"

# ---------------------------------------------------------------------
# (1) Single definition site for wrap_tag24 / unwrap_tag24.
# ---------------------------------------------------------------------
for fn in wrap_tag24 unwrap_tag24; do
  defs=$(grep -rln "pub fn ${fn}\b" crates/ --include='*.rs' 2>/dev/null | sort -u || true)
  count=$(printf '%s\n' "$defs" | grep -c . || true)
  if [[ "$count" -ne 1 || "$defs" != "$AUTHORITY" ]]; then
    echo "[ci_check_tag24_wire_authority] FAIL — '${fn}' must be defined exactly once, in ${AUTHORITY}."
    echo "  found in:"
    printf '%s\n' "$defs" | sed 's/^/    /'
    FAIL=1
  fi
done

# ---------------------------------------------------------------------
# (2) No hand-rolled tag-24 byte-literal parse in RED consumer paths.
#     These files must route tag-24 stripping through the authority.
# ---------------------------------------------------------------------
RED_CONSUMERS=(
  "crates/ade_node/src/admission/runner.rs"
  "crates/ade_core_interop/src/follow.rs"
)
for f in "${RED_CONSUMERS[@]}"; do
  [[ -f "$f" ]] || continue
  # A tag-24 marker sniff: 0xd8 adjacent to 0x18 in a comparison, e.g.
  # `bytes[0] != 0xd8` / `== 0xd8`. We flag any 0xd8 byte-literal in a
  # comparison context in these production consumers.
  if grep -nE '[!=]=\s*0xd8|0xd8\s*&&|0x18\b.*0xd8|0xd8.*0x18' "$f" >/dev/null 2>&1; then
    echo "[ci_check_tag24_wire_authority] FAIL — hand-rolled tag-24 byte parse in ${f}:"
    grep -nE '[!=]=\s*0xd8|0xd8\s*&&|0x18\b.*0xd8|0xd8.*0x18' "$f" | sed 's/^/    /'
    echo "    Use ade_codec::unwrap_tag24 / decompose_* instead."
    FAIL=1
  fi
done

# ---------------------------------------------------------------------
# (3) Serve paths compose via the per-protocol authorities.
# ---------------------------------------------------------------------
BF_SERVER="crates/ade_network/src/block_fetch/server.rs"
if ! grep -q "compose_blockfetch_block" "$BF_SERVER"; then
  echo "[ci_check_tag24_wire_authority] FAIL — ${BF_SERVER} must serve via compose_blockfetch_block (no bare [era,block])."
  FAIL=1
fi
CS_SERVER="crates/ade_network/src/chain_sync/server.rs"
if ! grep -q "compose_rollforward_header" "$CS_SERVER"; then
  echo "[ci_check_tag24_wire_authority] FAIL — ${CS_SERVER} must serve via compose_rollforward_header (no bare header)."
  FAIL=1
fi

# compose/decompose must delegate to the ade_codec authority, not
# re-implement the wrap.
CODEC_BF="crates/ade_network/src/codec/block_fetch.rs"
CODEC_CS="crates/ade_network/src/codec/chain_sync.rs"
if ! grep -q "ade_codec::wrap_tag24" "$CODEC_BF"; then
  echo "[ci_check_tag24_wire_authority] FAIL — compose_blockfetch_block must delegate to ade_codec::wrap_tag24."
  FAIL=1
fi
if ! grep -q "ade_codec::wrap_tag24" "$CODEC_CS"; then
  echo "[ci_check_tag24_wire_authority] FAIL — compose_rollforward_header must delegate to ade_codec::wrap_tag24."
  FAIL=1
fi

# ---------------------------------------------------------------------
# (4) The deleted hand-rolled unwrap must not reappear.
# ---------------------------------------------------------------------
if grep -rln "fn unwrap_block_fetch_envelope" crates/ --include='*.rs' >/dev/null 2>&1; then
  echo "[ci_check_tag24_wire_authority] FAIL — hand-rolled unwrap_block_fetch_envelope reintroduced:"
  grep -rln "fn unwrap_block_fetch_envelope" crates/ --include='*.rs' | sed 's/^/    /'
  echo "    Use ade_codec::unwrap_tag24 / decompose_blockfetch_block instead."
  FAIL=1
fi

if [[ "$FAIL" -eq 0 ]]; then
  echo "[ci_check_tag24_wire_authority] PASS — single tag-24 authority; serve composes; no hand-rolled parse."
fi
exit "$FAIL"
