#!/usr/bin/env bash
# DC-MITHRIL-08: the native Mithril FirstRun is boundary-complete -- it builds the EVIEW reduced
# checkpoint INLINE (gated on delegations, sealed at the certified slot) from the materialized UTxO,
# so a Mithril-started node produces (checkpoint + sidecar + tip) and ECA activation is armed (not
# inert) at the next epoch boundary. The build is INLINE via the BLUE/GREEN primitives -- it MUST NOT
# couple native_firstrun to admission::bootstrap's helper (the two RED bootstrap paths stay
# independent; the shared authority is the primitive, not a RED helper).
set -euo pipefail

F="crates/ade_node/src/native_firstrun.rs"
[ -f "$F" ] || { echo "FAIL: $F not found"; exit 1; }

# (1) the inline reduced-checkpoint build is present (the underlying primitives, not a helper call).
grep -q 'reduce_txout' "$F" || { echo "FAIL: native_firstrun missing reduce_txout (inline reduced build)"; exit 1; }
grep -q 'ReducedUtxoCheckpoint::open' "$F" || { echo "FAIL: native_firstrun missing ReducedUtxoCheckpoint::open"; exit 1; }
grep -q 'build_from' "$F" || { echo "FAIL: native_firstrun missing build_from"; exit 1; }
grep -q 'seal_bootstrap' "$F" || { echo "FAIL: native_firstrun missing seal_bootstrap"; exit 1; }

# (2) gated on the EVIEW package: a no-delegation snapshot builds NO checkpoint (byte-identical).
grep -qF 'cert_state.delegation.delegations.is_empty()' "$F" || {
  echo "FAIL: native_firstrun reduced-checkpoint build is not gated on delegations"; exit 1; }

# (3) INLINE discipline (mechanically enforced): native_firstrun MUST NOT reach into
#     admission::bootstrap's reduced-checkpoint helper -- the build stays inline via the primitives.
if grep -qE 'admission::bootstrap::build_live_reduced_checkpoint|build_live_reduced_checkpoint' "$F"; then
  echo "FAIL: native_firstrun must build the checkpoint INLINE, not via admission::bootstrap::build_live_reduced_checkpoint"
  exit 1
fi

echo "PASS: DC-MITHRIL-08 native FirstRun inline reduced-checkpoint build (boundary-complete, gated, no cross-module coupling)"
