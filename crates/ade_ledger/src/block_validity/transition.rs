// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// `block_validity` composes the two existing authorities — header and body —
// into one total, fail-fast verdict:
//
//   HeaderValid ∧ (body_hash == header.body_hash) ∧ BodyValid
//
// Header is decided first; the body authority NEVER runs if the header fails
// (`DC-VAL-03`). The body-hash binding is a real wired check that runs before
// body application so an altered body is rejected by the cheap hash compare
// (`CN-CONS-04`). On any Invalid outcome the input states are returned
// unchanged — no partial mutation (`DC-VAL-05`). No new validation rules are
// introduced here: this is composition only (`DC-VAL-02`).

use ade_core::consensus::events::Point;
use ade_core::consensus::ledger_view::LedgerView;
use ade_core::consensus::{
    validate_and_apply_header, EraSchedule, LeaderEligibility, PraosChainDepState,
};

use crate::rules::apply_block_with_verdicts;
use crate::state::LedgerState;

use super::header_input::decode_block;
use super::{BlockValidityError, BlockValidityVerdict};

/// The outcome of `block_validity`: the verdict plus the (possibly evolved)
/// states. Both states equal the input clones on any Invalid outcome.
pub struct BlockValidityOutcome {
    pub verdict: BlockValidityVerdict,
    pub ledger: LedgerState,
    pub chain_dep: PraosChainDepState,
}

/// Decide whether `block_cbor` is a valid block atop `(ledger, chain_dep)`.
///
/// Total and fail-fast: the first failing stage produces the verdict and the
/// input states are returned unchanged. A `Valid` verdict returns the states
/// evolved by both authorities. ENFORCES the leader-eligibility (stake) check —
/// the live, untrusted-header entry point.
pub fn block_validity(
    ledger: &LedgerState,
    chain_dep: &PraosChainDepState,
    era_schedule: &EraSchedule,
    ledger_view: &dyn LedgerView,
    block_cbor: &[u8],
) -> BlockValidityOutcome {
    block_validity_with_eligibility(
        ledger,
        chain_dep,
        era_schedule,
        ledger_view,
        block_cbor,
        LeaderEligibility::Enforce,
    )
}

/// `block_validity` for the warm-start / rollback REPLAY of already-admit-validated DURABLE blocks:
/// it TRUSTS each block's leader claim (the durable store is the replay authority) and lacks the
/// per-epoch stake to re-check it. Every other stage (structural, VRF-proof, KES, op-cert, nonce
/// apply, body authority) runs identically, so the reconstructed state is byte-identical to the
/// enforcing path — only the redundant stake-threshold check is skipped (see [`LeaderEligibility`]).
///
/// `pub(crate)`, NOT `pub`: the trust-skipping variant must never be reachable from an untrusted
/// (peer / admission) entry point. Its ONLY caller is `materialize_rolled_back_state` (durable
/// replay); `ci/ci_check_trusted_replay_boundary.sh` enforces the single-caller invariant mechanically.
pub(crate) fn block_validity_trusted_replay(
    ledger: &LedgerState,
    chain_dep: &PraosChainDepState,
    era_schedule: &EraSchedule,
    ledger_view: &dyn LedgerView,
    block_cbor: &[u8],
) -> BlockValidityOutcome {
    block_validity_with_eligibility(
        ledger,
        chain_dep,
        era_schedule,
        ledger_view,
        block_cbor,
        LeaderEligibility::TrustDurable,
    )
}

fn block_validity_with_eligibility(
    ledger: &LedgerState,
    chain_dep: &PraosChainDepState,
    era_schedule: &EraSchedule,
    ledger_view: &dyn LedgerView,
    block_cbor: &[u8],
    leader_eligibility: LeaderEligibility,
) -> BlockValidityOutcome {
    // Step 1: decode (era-tagged envelope → header input + hashes).
    let decoded = match decode_block(block_cbor) {
        Ok(d) => d,
        Err(error) => return invalid(ledger, chain_dep, error),
    };

    // Step 3: header authority. FAIL-FAST — the body authority is not reached
    // if this fails.
    let applied = match validate_and_apply_header(
        chain_dep,
        &decoded.header_input,
        ledger_view,
        era_schedule,
        leader_eligibility,
    ) {
        Ok(a) => a,
        Err(e) => return invalid(ledger, chain_dep, BlockValidityError::Header(e)),
    };

    // Step 4: body-hash binding (wired pre-flight, before body application).
    let header_body_hash = applied.summary.body_hash.clone();
    if decoded.computed_body_hash != header_body_hash {
        return invalid(
            ledger,
            chain_dep,
            BlockValidityError::BodyHashMismatch {
                header: header_body_hash,
                actual: decoded.computed_body_hash,
            },
        );
    }

    // Step 5: body authority. The body authority consumes the INNER block
    // (the envelope's `[era, ..]` tag is stripped).
    let inner = &block_cbor[decoded.inner_start..decoded.inner_end];
    let body = match apply_block_with_verdicts(ledger, decoded.era, inner) {
        Ok(b) => b,
        Err(e) => return invalid(ledger, chain_dep, BlockValidityError::Body(e)),
    };

    // Step 6: Valid — return both evolved states.
    BlockValidityOutcome {
        verdict: BlockValidityVerdict::Valid {
            tip: Point {
                slot: applied.summary.slot,
                hash: decoded.block_hash,
            },
            block_no: applied.summary.block_no,
            body: body.verdict,
        },
        ledger: body.new_state,
        chain_dep: applied.new_state,
    }
}

/// Build an Invalid outcome with the input states cloned unchanged.
fn invalid(
    ledger: &LedgerState,
    chain_dep: &PraosChainDepState,
    error: BlockValidityError,
) -> BlockValidityOutcome {
    let class = error.class();
    BlockValidityOutcome {
        verdict: BlockValidityVerdict::Invalid { class, error },
        ledger: ledger.clone(),
        chain_dep: chain_dep.clone(),
    }
}
