#!/usr/bin/env bash
#
# ci_check_producer_praos_vrf.sh — PHASE4-N-W S3 gate (CN-FORGE-04).
#
# The producer's leader-eligibility VRF must use the SAME era-correct
# construction the validator runs. For Praos (Babbage/Conway) that is the
# single combined input `praos_vrf_input(slot, eta0)` with the
# `praos_leader_value` range-extension — NOT the TPraos role-tagged
# `vrf_input(..., LeaderEligibility)`. There is exactly ONE era→construction
# authority (`leader_vrf_input`), shared by the producer leader-schedule, the
# leader-check evaluator, and (via the answer) the forge prove-step. No
# fallback accepts both TPraos and Praos inputs for one era.
#
# Guards:
#   1. `leader_vrf_input` (the single era→construction authority) is defined
#      exactly once, in ade_core::consensus::vrf_cert.
#   2. The producer leader path (produce_mode / leader_schedule / leader_check)
#      contains no bare `vrf_input(` call — it routes through leader_vrf_input
#      or the answer's alpha_bytes (no TPraos leader alpha on the producer).
#   3. The TPraos construction is PRESERVED for validation: `vrf_input` +
#      `VrfRole` still exist in vrf_cert (not removed).
#   4. TPraos producer forging fail-closes: `UnsupportedProducerEra` exists in
#      the producer error vocabulary and is used by produce_mode.
#   5. No both-alphas fallback: no file OUTSIDE vrf_cert.rs contains both
#      `praos_vrf_input(` and a bare `vrf_input(` (the dual construction lives
#      only inside the single `leader_vrf_input` authority).
#   6. The Praos eligibility threshold uses `leader_value_for` (era-correct
#      leader value — praos_leader_value for Praos), not the raw VRF output.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

VRF_CERT="crates/ade_core/src/consensus/vrf_cert.rs"
PRODUCER_FILES=(
  "crates/ade_node/src/produce_mode.rs"
  "crates/ade_core/src/consensus/leader_schedule.rs"
  "crates/ade_core/src/consensus/leader_check.rs"
)
FAIL=0

# Guard 1 — single era→construction authority.
DEF_COUNT=$(grep -rl "pub fn leader_vrf_input" crates/ --include='*.rs' | sort -u | wc -l)
DEF_SITE=$(grep -rl "pub fn leader_vrf_input" crates/ --include='*.rs' | sort -u | head -1)
if [[ "$DEF_COUNT" -ne 1 || "$DEF_SITE" != "$VRF_CERT" ]]; then
  echo "[ci_check_producer_praos_vrf] FAIL (G1) — leader_vrf_input must be defined exactly once in $VRF_CERT; found $DEF_COUNT def(s): $DEF_SITE"
  FAIL=1
fi

# Guard 2 — no bare vrf_input( (TPraos leader alpha) on the producer path.
# Strip line comments so a doc reference doesn't trip the gate.
for f in "${PRODUCER_FILES[@]}"; do
  if grep -vE '^\s*//' "$f" | grep -qE '\bvrf_input\('; then
    echo "[ci_check_producer_praos_vrf] FAIL (G2) — bare vrf_input( on the producer leader path in $f; route through leader_vrf_input / the answer's alpha_bytes"
    FAIL=1
  fi
done

# Guard 3 — TPraos construction preserved (validation still supported).
if ! grep -q "pub fn vrf_input" "$VRF_CERT" || ! grep -q "pub enum VrfRole" "$VRF_CERT"; then
  echo "[ci_check_producer_praos_vrf] FAIL (G3) — TPraos vrf_input/VrfRole must remain in $VRF_CERT (validation must not be removed)"
  FAIL=1
fi

# Guard 4 — TPraos producer forging fail-closes.
if ! grep -q "UnsupportedProducerEra" crates/ade_runtime/src/producer/producer_log.rs \
   || ! grep -q "UnsupportedProducerEra" crates/ade_node/src/produce_mode.rs; then
  echo "[ci_check_producer_praos_vrf] FAIL (G4) — UnsupportedProducerEra must exist (producer_log) and be used (produce_mode) to fail-close TPraos forging"
  FAIL=1
fi

# Guard 5 — no both-alphas fallback outside the single authority.
while IFS= read -r f; do
  [[ "$f" == "$VRF_CERT" ]] && continue
  if grep -qE '\bvrf_input\(' "$f"; then
    echo "[ci_check_producer_praos_vrf] FAIL (G5) — both praos_vrf_input( and bare vrf_input( in $f (a both-alphas fallback); the dual construction must live only inside leader_vrf_input"
    FAIL=1
  fi
done < <(grep -rl "praos_vrf_input(" crates/ --include='*.rs' | sort -u)

# Guard 6 — Praos eligibility threshold uses the era-correct leader value.
if ! grep -q "leader_value_for(" crates/ade_core/src/consensus/leader_check.rs; then
  echo "[ci_check_producer_praos_vrf] FAIL (G6) — leader_check must use leader_value_for (praos_leader_value for Praos), not the raw VRF output"
  FAIL=1
fi

if [[ "$FAIL" -ne 0 ]]; then
  echo ""
  echo "  CN-FORGE-04: one era-correct Praos VRF authority; no TPraos leader alpha"
  echo "  on the producer; no both-alphas fallback; TPraos validation preserved."
  exit 1
fi

echo "[ci_check_producer_praos_vrf] PASS (G1-G6)"
