#!/usr/bin/env bash
# CE-B1/CE-B2 (PHASE4-N-AE.B) — recovered/forge-parent intersectability, Option B:
# FindIntersect-only, PROOF-GATED, NO synthetic bytes.
#
# The serve may advertise a recovered/forged parent as a FindIntersect point ONLY
# IF it can prove the point is the parent of a real servable successor — the
# `prev_hash` of the EARLIEST servable StoredBlock. It must NEVER synthesize a
# StoredBlock or serve bytes for the projected point (BlockFetch refuses
# structurally because the point is not a StoredBlock).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

PROJ="crates/ade_runtime/src/network/served_chain_projection.rs"
fail() { echo "FAIL ci_check_recovered_anchor_intersectable: $1" >&2; exit 1; }
[ -f "$PROJ" ] || fail "missing $PROJ"

# Production code only (exclude the #[cfg(test)] module).
PROD="$(awk '/#\[cfg\(test\)\]/{exit} {print}' "$PROJ")"

# (1) The proof-gated forge-parent projection exists AND is used (def + >=1 call
#     in `intersect`).
n_proof="$(printf '%s\n' "$PROD" | grep -c 'earliest_servable_block_prev_hash' || true)"
[ "$n_proof" -ge 2 ] || fail "the proof-gated forge-parent projection (earliest_servable_block_prev_hash) is missing or unused (found $n_proof; need def + call)"

# (2) The projection is gated on a REAL successor's prev_hash (PrevHash::Block),
#     never an unconditional / zero / synthetic anchor.
printf '%s\n' "$PROD" | grep -q 'PrevHash::Block' || fail "the projection is not gated on a real prev_hash (PrevHash::Block)"

# (3) NO synthetic bytes / no fake StoredBlock: the PRODUCTION serve projection is
#     read-only — it must NOT construct a StoredBlock or write/put a block/snapshot.
if printf '%s\n' "$PROD" | grep -nE 'StoredBlock[[:space:]]*\{|\.put_block|\.put_snapshot' | grep -q .; then
  fail "the serve projection constructs/writes a block (synthetic bytes / fake StoredBlock forbidden — Option B is FindIntersect-only)"
fi

# (4) BlockFetch bytes still come ONLY from real stored bytes via the single decode
#     authority — serve_range derives hashes from decode_block of stored bytes, so a
#     projected (non-StoredBlock) point yields no bytes.
printf '%s\n' "$PROD" | awk '/fn serve_range/,/^    }/' | grep -q 'decode_block' \
  || fail "serve_range no longer derives served hashes from real stored bytes via decode_block"

echo "OK ci_check_recovered_anchor_intersectable: FindIntersect-only forge-parent projection, proof-gated on the earliest servable StoredBlock's prev_hash; no synthetic StoredBlock / no bytes served for the projected point."
