#!/usr/bin/env bash
set -uo pipefail

# EPOCH-CONSENSUS-VIEW ECA-B1 (DC-EPOCH-16): rolling Praos chain-dep nonce evolution on the live
# follow path. ONE indivisible per-header HeaderContribution {slot, prev_block_hash, vrf_nonce_output,
# freeze_boundary} computes evolving'/lab'/candidate'; the epoch tick computes
# epoch_nonce' = candidate (X) last_epoch_block_nonce with the previous/last-epoch-block rotation and
# evolving/candidate carried UNCHANGED (no reset). The combine operand is an explicit optional that
# fails closed (never fabricated); the durable chain-dep snapshot is backward-compatible. The
# separable CandidateFreeze is retired.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$REPO_ROOT"
FAILED=0; fail() { echo "FAIL: $1"; FAILED=1; }
NONCE=crates/ade_core/src/consensus/nonce.rs
ERRORS=crates/ade_core/src/consensus/errors.rs
CHAIN_DEP=crates/ade_ledger/src/snapshot/chain_dep.rs
SEED=crates/ade_runtime/src/mithril_native_assembly.rs

# (1) ONE indivisible per-header input carries the 4 authoritative fields.
for f in 'prev_block_hash: Hash32,' 'vrf_nonce_output: VrfOutput,' 'freeze_boundary: SlotNo,'; do
    grep -qF "$f" "$NONCE" || fail "HeaderContribution is missing the field: $f"
done

# (2) CandidateFreeze is RETIRED -- no variant declaration or construction may exist (a prose
#     mention that it was retired is allowed).
if grep -qE 'CandidateFreeze[[:space:]]*\{|NonceInput::CandidateFreeze' "$NONCE"; then
    fail "CandidateFreeze must be retired (no variant or construction; folded into the per-header step)"
fi

# (3) the per-header step maintains lab + tracks/freezes the candidate from the single input.
HC="$(awk '/fn apply_header_contribution/{f=1} f{print} f&&/^}/{exit}' "$NONCE")"
grep -qF 'lab_next = Nonce(prev_block_hash.clone())' <<< "$HC" \
    || fail "apply_header_contribution does not set lab' = Nonce(prev_block_hash)"
grep -qF 'slot.0 < freeze_boundary.0' <<< "$HC" \
    || fail "apply_header_contribution does not gate the candidate freeze on the freeze_boundary"

# (4) the epoch tick: combine + rotation, evolving & candidate carried through UNCHANGED (no reset).
EB="$(awk '/fn apply_epoch_boundary/{f=1} f{print} f&&/^}/{exit}' "$NONCE")"
grep -qF 'combine(&state.candidate_nonce, last_epoch_block_nonce)' <<< "$EB" \
    || fail "the epoch tick does not combine candidate with last_epoch_block_nonce"
grep -qF 'last_epoch_block_nonce = Some(state.lab_nonce.clone())' <<< "$EB" \
    || fail "the epoch tick does not rotate last_epoch_block_nonce <- lab"
if grep -qE 'new_state\.evolving_nonce =|new_state\.candidate_nonce =' <<< "$EB"; then
    fail "the epoch tick must NOT reset evolving or candidate (Praos carries them through)"
fi

# (5) explicit operand presence: fail closed on an absent operand -- never fabricate.
grep -qF 'MissingLastEpochBlockNonce' "$ERRORS" \
    || fail "NonceEvolutionError::MissingLastEpochBlockNonce is missing"
grep -qF 'ok_or(NonceEvolutionError::MissingLastEpochBlockNonce)' <<< "$EB" \
    || fail "the epoch tick does not fail closed on an absent last_epoch_block_nonce"

# (6) backward-compatible durable chain-dep snapshot: write array(10), accept legacy array(9).
grep -qF 'const FIELDS: u64 = 10;' "$CHAIN_DEP" \
    || fail "the chain-dep snapshot must encode the array(10) form"
grep -qF 'const FIELDS_LEGACY: u64 = 9;' "$CHAIN_DEP" \
    || fail "the chain-dep snapshot must declare the legacy array(9) arity"
grep -qF 'n == FIELDS || n == FIELDS_LEGACY' "$CHAIN_DEP" \
    || fail "the chain-dep decoder must accept exactly the current OR legacy arity"

# (7) FirstRun seeds the combine operand from the imported snapshot (no fabrication).
grep -qF 'last_epoch_block_nonce: Some(to_core(&n.last_epoch_block))' "$SEED" \
    || fail "the native bootstrap does not seed last_epoch_block_nonce from the imported snapshot"

# (8) the hermetic proofs exist (incl. the mandatory bridge-equivalence assertion).
for t in \
    seeded_chain_dep_tick_reproduces_bridge_eta0 \
    epoch_boundary_combines_candidate_with_last_epoch_block_nonce \
    epoch_boundary_does_not_reset_evolving_or_candidate \
    epoch_boundary_fails_closed_on_missing_operand \
    header_contribution_freezes_candidate_at_freeze_boundary \
    chain_dep_legacy_array9_decodes_to_none ; do
    grep -rqF "fn $t" crates/ || fail "the DC-EPOCH-16 proof '$t' is missing"
done

# (9) LIVE WIRING (B2): RSW = ceil(4k/f) lives in the canonical era geometry (derived by the parser
#     from the venue k), header_validate READS it from there (not a LedgerView method, which is
#     retired), and the boundary tick is DRIVEN on the follow path with a fail-closed bridge eta0
#     cross-check. The warm-start (no-k) path stays inert via CANDIDATE_FREEZE_INERT until B4.
HV=crates/ade_core/src/consensus/header_validate.rs
ERA=crates/ade_core/src/consensus/era_schedule.rs
SYNC=crates/ade_node/src/node_sync.rs
LIFECYCLE=crates/ade_node/src/node_lifecycle.rs
GENESIS=crates/ade_runtime/src/consensus/genesis_parser.rs
PROFILE=crates/ade_node/src/bootstrap_export.rs
grep -qF 'randomness_stabilisation_window_slots' "$ERA" \
    || fail "EraSummary must carry the RSW field (randomness_stabilisation_window_slots)"
grep -qF 'security_param' "$PROFILE" \
    || fail "NetworkProfile must carry the venue securityParam (k) for RSW = ceil(4k/f)"
grep -qF 'pub fn praos_rsw_slots' "$ERA" \
    || fail "the single RSW source of truth (praos_rsw_slots) must live in ade_core"
grep -qF 'praos_rsw_slots' "$GENESIS" \
    || fail "the genesis parser must derive RSW via the shared praos_rsw_slots helper"
grep -qF 'praos_rsw_slots' "$LIFECYCLE" \
    || fail "the live --network schedule builder must derive RSW via the shared praos_rsw_slots helper"
grep -qF 'randomness_stabilisation_window_slots' "$HV" \
    || fail "header_validate must read RSW from the era geometry"
grep -qF 'CANDIDATE_FREEZE_INERT' "$HV" \
    || fail "the warm-start (no-k) inert sentinel CANDIDATE_FREEZE_INERT is missing"
if grep -rqF 'fn randomness_stabilisation_window' crates/; then
    fail "the deferred LedgerView::randomness_stabilisation_window method must be retired (RSW now lives in the era geometry)"
fi
grep -qF 'NonceInput::EpochBoundary' "$SYNC" \
    || fail "the boundary tick is not driven on the live follow path (node_sync)"
grep -qF 'bridge eta0' "$SYNC" \
    || fail "the boundary tick does not cross-check the ECA-5 bridge eta0 (fail-closed)"

if (( FAILED == 0 )); then
    echo "OK: rolling Praos nonce evolution on the follow path (DC-EPOCH-16; indivisible per-header step, combine + rotation no-reset, fail-closed operand, backward-compat snapshot, bridge-equivalence proven; live RSW freeze from the era geometry + boundary tick driven with a fail-closed bridge cross-check; CandidateFreeze + the deferred LedgerView RSW method retired)"
fi
exit $FAILED
