// Core Contract:
// - Deterministic: same inputs => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - prevalidate_branch is PURE: no I/O, no store reads, no durable mutation

//! Fork-switch prove core (PHASE4-N-AO S4, `DC-NODE-37`).
//!
//! **A `PendingForkSwitch` is not authority to roll back; it is only authority to
//! *attempt proof* of the selected replacement branch.**
//!
//! S3 (`DC-NODE-36`) decided a fork-choice win and emitted a `PendingForkSwitch`.
//! S4 turns that provisional decision into a durable adoption **only** by proving
//! the complete replacement branch — fetched bodies, bound to the S3-selected
//! headers, linked from the durable fork anchor, and ledger-validated — and the
//! proof STRICTLY precedes the irreversible `commit_rollback`. A failed proof
//! leaves the current durable chain byte-unchanged (FC-6).
//!
//! Colour split:
//! - [`BranchBodySource`] is the RED fetch seam (the body bytes come from the
//!   winning peer; the live `BlockFetch RequestRange` wiring is CE-AO-6).
//! - [`prevalidate_branch`] is GREEN/BLUE-reused and **pure**: it operates on the
//!   already-fetched bodies + the already-materialized anchor state, performing
//!   the body↔header bind, the parent-link proof, and the BLUE `block_validity`
//!   ledger fold. It reads no store and mutates nothing.
//! - The RED driver `node_lifecycle::apply_fork_switch` does the fetch + the
//!   read-only materialize, calls here, and (only on success) adopts via the
//!   existing `apply_chain_event` authorities.

use ade_core::consensus::candidate::CandidateFragment;
use ade_core::consensus::era_schedule::EraSchedule;
use ade_core::consensus::events::Point;
use ade_core::consensus::header_summary::{HeaderVrf, ValidatedHeaderSummary};
use ade_core::consensus::ledger_view::LedgerView;
use ade_core::consensus::praos_leader_value;
use ade_core::consensus::praos_state::PraosChainDepState;
use ade_crypto::vrf::VrfOutput;
use ade_ledger::block_validity::{block_validity, decode_block, BlockValidityVerdict, DecodedBlock};
use ade_ledger::state::LedgerState;
use ade_types::shelley::block::PrevHash;
use ade_types::{Hash32, SlotNo};

use crate::lca_walk::LcaError;
use crate::selector_state::{ForkAnchor, PendingForkSwitch};

/// The RED seam that supplies a winning branch's bodies. Hermetic in tests; the
/// live `BlockFetch RequestRange` anchor→tip wiring is out of S4 scope (CE-AO-6)
/// and must not weaken the prevalidate-before-commit contract.
pub trait BranchBodySource {
    /// Fetch the block body the winning `peer` served at `slot`. A missing /
    /// unavailable body is a proof failure (the branch is not proven), never a
    /// durable mutation.
    fn fetch_body(&self, peer: &str, slot: SlotNo) -> Result<Vec<u8>, FetchError>;
}

/// Closed fetch failure surface. Fail-closed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FetchError {
    /// The peer did not serve a body for the requested point.
    Unavailable,
}

/// A `BranchBodySource` that serves nothing — the relay loop's placeholder until
/// the live `BlockFetch` fetch lands (CE-AO-6). With it, a fork-choice win fails
/// proof closed (the fence stays set; nothing is adopted), never a half-switch.
pub struct NullBranchBodySource;

impl BranchBodySource for NullBranchBodySource {
    fn fetch_body(&self, _peer: &str, _slot: SlotNo) -> Result<Vec<u8>, FetchError> {
        Err(FetchError::Unavailable)
    }
}

/// PHASE4-N-AO S6 (CE-AO-6): a `BranchBodySource` populated from bytes the relay
/// loop pre-fetched live (`BlockFetch RequestRange` from the winning peer). The
/// bridge between the async live fetch and the sync S4 prove seam.
///
/// **It carries BYTES and nothing else** — no verdict, no selection, no fence, no
/// authority. `apply_fork_switch` (S4) is the sole adopter; a lying / short /
/// truncated / Byzantine fetch is rejected by `prove_fork_switch` /
/// `prevalidate_branch` BEFORE any `commit_rollback`. BlockFetch transports bytes;
/// it does not grant truth.
#[derive(Debug, Default)]
pub struct PrefetchedBranchBodies {
    bodies: std::collections::BTreeMap<(String, u64), Vec<u8>>,
}

impl PrefetchedBranchBodies {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a pre-fetched body for `(peer, slot)`. Bytes only.
    pub fn insert(&mut self, peer: &str, slot: SlotNo, bytes: Vec<u8>) {
        self.bodies.insert((peer.to_string(), slot.0), bytes);
    }

    /// How many bodies were pre-fetched (for the short-range / truncation check).
    pub fn len(&self) -> usize {
        self.bodies.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bodies.is_empty()
    }
}

impl BranchBodySource for PrefetchedBranchBodies {
    fn fetch_body(&self, peer: &str, slot: SlotNo) -> Result<Vec<u8>, FetchError> {
        self.bodies
            .get(&(peer.to_string(), slot.0))
            .cloned()
            .ok_or(FetchError::Unavailable)
    }
}

/// Closed proof-failure surface — every variant leaves the current durable chain
/// byte-unchanged (no `commit_rollback`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BranchProofError {
    /// The winning candidate carried no headers — a branch must have ≥1 block.
    EmptyBranch,
    /// The peer served no body for the header at `slot` (or a short branch).
    BodyUnavailable { slot: SlotNo },
    /// The fetched body (0-based `index`) does not match the S3-selected header
    /// (re-derived header fields and/or recomputed body hash differ).
    BodyHeaderMismatch { index: usize },
    /// The fetched body (0-based `index`) does not link from the fork anchor
    /// (its `prev_hash` is not the previous block / the anchor).
    BrokenParentLink { index: usize },
    /// The fetched body (0-based `index`) fails ledger validation atop the
    /// materialized anchor state — caught BEFORE any commit.
    BodyInvalid { index: usize },
    /// The fork anchor is unreachable for materialize (beyond k / retention) —
    /// the independent depth guard, caught BEFORE any commit (`DC-CONS-05`).
    AnchorUnreachable,
}

/// One proven block of the replacement branch: its bytes + the durable tip
/// (slot + hash) the BLUE `block_validity` verdict assigned it (the
/// `ChainSelected.new_tip` the apply will reconcile against).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvenBlock {
    pub bytes: Vec<u8>,
    pub tip: Point,
}

/// A fully-proven replacement branch (anchor→tip), ready for adoption via the
/// existing `apply_chain_event` authorities. Non-empty by construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvenBranch {
    pub blocks: Vec<ProvenBlock>,
}

/// The structured outcome of a fork-switch apply attempt. `ProofFailed` carries
/// the reason; the current durable chain is unchanged and the forge fence is held.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ForkSwitchOutcome {
    /// The branch was proven and durably adopted; the new durable tip and its
    /// validated parent hash (`new_tip_prev` — the prior applied block, or the
    /// fork anchor for a single-block branch). The parent link is capture-only
    /// evidence fidelity for the post-switch branch-continuity verdict
    /// (PHASE4-N-AO S10, DC-EVIDENCE-05); not read by any authority path.
    Adopted { new_tip: Point, new_tip_prev: Hash32 },
    /// The branch was NOT proven; no durable change, the decision retired failed.
    ProofFailed { error: BranchProofError },
}

/// Prove the complete replacement branch — PURE over its inputs (no I/O, no store,
/// no mutation). The caller (the RED driver) has already fetched `bodies` (one per
/// `winning_candidate.headers`, anchor→tip order) and materialized the anchor
/// state `(anchor_ledger, anchor_chain_dep)` read-only.
///
/// Three proofs, all BEFORE the caller's `commit_rollback`:
/// 1. **Bind** — each fetched body's re-derived header field-matches the
///    S3-selected `ValidatedHeaderSummary`, and its recomputed body hash matches.
///    (S3's summary carries no block hash; S4 trusts nothing peer-asserted.)
/// 2. **Link** — `body[0].prev_hash == fork_anchor.hash`; `body[i].prev_hash ==
///    hash(body[i-1])`.
/// 3. **Ledger-validate** — fold BLUE `block_validity` over the bodies from the
///    materialized anchor; any non-`Valid` verdict fails closed.
#[allow(clippy::too_many_arguments)]
pub fn prevalidate_branch(
    fork_anchor: &ForkAnchor,
    winning_candidate: &CandidateFragment,
    bodies: &[Vec<u8>],
    anchor_ledger: &LedgerState,
    anchor_chain_dep: &PraosChainDepState,
    era_schedule: &EraSchedule,
    ledger_view: &dyn LedgerView,
) -> Result<ProvenBranch, BranchProofError> {
    let headers = &winning_candidate.headers;
    if headers.is_empty() {
        return Err(BranchProofError::EmptyBranch);
    }
    // A short branch (fewer bodies than selected headers) is an unavailable body.
    if bodies.len() != headers.len() {
        return Err(BranchProofError::BodyUnavailable {
            slot: headers[bodies.len().min(headers.len() - 1)].slot,
        });
    }

    // (1) bind + (2) link — decode each body, field-match the selected header,
    // and chain prev_hash from the durable fork anchor.
    let mut expected_prev = fork_anchor.hash.clone();
    for (index, (header, body)) in headers.iter().zip(bodies).enumerate() {
        let decoded =
            decode_block(body).map_err(|_| BranchProofError::BodyHeaderMismatch { index })?;
        if !header_matches_summary(&decoded, header) {
            return Err(BranchProofError::BodyHeaderMismatch { index });
        }
        match &decoded.prev_hash {
            PrevHash::Block(h) if *h == expected_prev => {}
            _ => return Err(BranchProofError::BrokenParentLink { index }),
        }
        expected_prev = decoded.block_hash.clone();
    }

    // (3) ledger-validate — fold BLUE block_validity from the materialized anchor.
    // A non-Valid verdict fails closed HERE, before the caller's commit_rollback.
    let mut ledger = anchor_ledger.clone();
    let mut chain_dep = anchor_chain_dep.clone();
    let mut blocks = Vec::with_capacity(bodies.len());
    for (index, body) in bodies.iter().enumerate() {
        let outcome = block_validity(&ledger, &chain_dep, era_schedule, ledger_view, body);
        match outcome.verdict {
            BlockValidityVerdict::Valid { tip, .. } => {
                blocks.push(ProvenBlock {
                    bytes: body.clone(),
                    tip,
                });
                ledger = outcome.ledger;
                chain_dep = outcome.chain_dep;
            }
            BlockValidityVerdict::Invalid { .. } => {
                return Err(BranchProofError::BodyInvalid { index })
            }
        }
    }
    Ok(ProvenBranch { blocks })
}

/// PHASE4-N-AO S11 (DC-NODE-39): closed reason a post-`ForkChoiceWin` competing
/// descendant could not be bridged to the durable adopted tip / a durable stored
/// ancestor within k. **A `MissingBridge` is ONLY a structured fail-closed outcome
/// -- never an adoption path, a rollback target, a candidate anchor, a reason to
/// clear the fence, a reason to skip the missing parent, or a reason to trust the
/// later block.** It HOLDS the forge fence (the fence-resolved predicate refuses
/// while it is `Some`) and refuses the silent no-op the pre-S11 dispatch did. No
/// durable mutation ever occurs on this path. Closed discriminant -- no free-form
/// strings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MissingBridgeReason {
    /// An intermediate header on the competing branch is absent from the cache --
    /// the branch is incomplete (a hole) and cannot reach a durable ancestor.
    BranchGap,
    /// The walk reached genesis without a durable stored ancestor within k.
    NoDurableAncestorWithinK,
    /// The walk exceeded k traversed headers (block depth) before reaching the LCA.
    ExceededK,
    /// A branch-cache entry's map key != its own re-derived hash (peer-claim, not
    /// authority) -- the cache must fail closed, never be trusted.
    CacheSelfBindingViolation,
    /// The durable LCA was found but is unreachable for a read-only materialize
    /// (beyond retention) -- the branch cannot be reconstructed to prove it.
    LcaUnreachable,
}

impl MissingBridgeReason {
    /// Closed discriminator string -- the value emitted to the convergence-evidence
    /// transcript. Mirrors the `LcaError` / materialize causes; never free-form.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::BranchGap => "branch_gap",
            Self::NoDurableAncestorWithinK => "no_durable_ancestor_within_k",
            Self::ExceededK => "exceeded_k",
            Self::CacheSelfBindingViolation => "cache_self_binding_violation",
            Self::LcaUnreachable => "lca_unreachable",
        }
    }
}

/// PHASE4-N-AO S11 (DC-NODE-39): map the closed S7 LCA-walk failure surface to the
/// closed `MissingBridgeReason`. The materialize ("LCA unreachable") failure case
/// is `LcaUnreachable`, set directly at its call site (it is not an `LcaError`).
pub fn map_lca_error(e: &LcaError) -> MissingBridgeReason {
    match e {
        LcaError::BranchGap => MissingBridgeReason::BranchGap,
        LcaError::NoDurableAncestorWithinK => MissingBridgeReason::NoDurableAncestorWithinK,
        LcaError::ExceededK => MissingBridgeReason::ExceededK,
        LcaError::CacheSelfBindingViolation => MissingBridgeReason::CacheSelfBindingViolation,
    }
}

/// PHASE4-N-AO S5 (DC-NODE-28 resolution) + S11 (DC-NODE-39): the forge fence
/// (`pending_reselection`) clears ONLY on a RESOLVED state -- no pending fork-switch
/// decision, no unresolved missing bridge, AND the node is caught up to the followed
/// peer. A proof failure HOLDS the fence (it says "that branch was not proven", not
/// "the disagreement is resolved"); likewise an unresolved `MissingBridge` HOLDS the
/// fence (S11 -- a node that adopted the winner but cannot bridge its descendants
/// must not forge on the stranded tip). The fence clears only here, when the
/// participant loop reaches a resolved no-pending state (the held-then-resolved path;
/// S4 clears the success path directly after reconcile). `caught_up` is the
/// `DC-NODE-15` signal (`forge_followed_tip_admission == CaughtUp`). PURE.
pub fn fork_switch_fence_resolved(
    pending_fork_switch: &Option<PendingForkSwitch>,
    pending_missing_bridge: &Option<MissingBridgeReason>,
    caught_up: bool,
) -> bool {
    pending_fork_switch.is_none() && pending_missing_bridge.is_none() && caught_up
}

/// A fetched body's re-derived header must field-match the S3-selected summary,
/// and its recomputed body hash must match. The leader value is compared on the
/// `praos_leader_value` basis the summary recorded (Conway/Praos); a TPraos tip is
/// unsupported on this venue and never matches.
fn header_matches_summary(decoded: &DecodedBlock, s: &ValidatedHeaderSummary) -> bool {
    let h = &decoded.header_input;
    h.slot == s.slot
        && h.block_no == s.block_no
        && h.body_hash == s.body_hash
        && h.issuer_pool == s.issuer_pool
        && h.op_cert_counter == s.op_cert_counter
        && decoded.computed_body_hash == s.body_hash
        && leader_output_matches(&h.vrf, &s.vrf_leader_output)
}

fn leader_output_matches(vrf: &HeaderVrf, expected: &VrfOutput) -> bool {
    match vrf {
        HeaderVrf::Praos { output, .. } => praos_leader_value(output) == *expected,
        HeaderVrf::Tpraos { .. } => false,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use ade_core::consensus::candidate::TiebreakerView;
    use ade_core::consensus::events::BlockDistance;
    use ade_types::{BlockNo, Hash28, Hash32};

    #[test]
    fn null_source_serves_nothing() {
        assert_eq!(
            NullBranchBodySource.fetch_body("peer-1", SlotNo(7)),
            Err(FetchError::Unavailable)
        );
    }

    fn anchor() -> ForkAnchor {
        ForkAnchor {
            slot: SlotNo(10),
            hash: Hash32([0x11; 32]),
            block_no: BlockNo(5),
        }
    }

    fn empty_candidate() -> CandidateFragment {
        CandidateFragment {
            anchor: Point {
                slot: SlotNo(10),
                hash: Hash32([0x11; 32]),
            },
            anchor_block_no: BlockNo(5),
            headers: vec![],
            select_view: TiebreakerView {
                slot: SlotNo(11),
                issuer_hash: Hash28([0xAA; 28]),
                op_cert_counter: 1,
                leader_vrf_output_first_8: [0u8; 8],
            },
            rollback_depth: BlockDistance(0),
        }
    }

    fn a_switch() -> PendingForkSwitch {
        PendingForkSwitch {
            fork_anchor: anchor(),
            winning_peer: "peer-1".to_string(),
            winning_candidate: empty_candidate(),
            winner_tip: Point {
                slot: SlotNo(11),
                hash: Hash32([0xBB; 32]),
            },
        }
    }

    #[test]
    fn fence_resolved_only_when_no_pending_and_caught_up() {
        // A decision in flight -> NOT resolved, even if caught up.
        assert!(!fork_switch_fence_resolved(&Some(a_switch()), &None, true));
        // No pending but NOT caught up (still behind / disagreeing) -> NOT resolved.
        assert!(!fork_switch_fence_resolved(&None, &None, false));
        // A proof failure leaves pending=None + the fence held; until caught up it
        // stays held (this predicate is the only live clear path besides S4 success).
        assert!(!fork_switch_fence_resolved(&Some(a_switch()), &None, false));
        // PHASE4-N-AO S11 (DC-NODE-39): an unresolved MissingBridge HOLDS the fence
        // even with no pending decision AND caught up -- a stranded post-switch tip
        // must not forge until the bridge arrives (clear-on-progress) or the run ends.
        assert!(!fork_switch_fence_resolved(
            &None,
            &Some(MissingBridgeReason::BranchGap),
            true
        ));
        // RESOLVED: no pending decision, no missing bridge, AND caught up.
        assert!(fork_switch_fence_resolved(&None, &None, true));
    }

    #[test]
    fn missing_bridge_reason_maps_lca_error_to_closed_discriminant() {
        // Every LcaError maps to a closed MissingBridgeReason with a closed
        // discriminator string -- no free-form text reaches the evidence vocabulary.
        assert_eq!(map_lca_error(&LcaError::BranchGap), MissingBridgeReason::BranchGap);
        assert_eq!(
            map_lca_error(&LcaError::NoDurableAncestorWithinK),
            MissingBridgeReason::NoDurableAncestorWithinK
        );
        assert_eq!(map_lca_error(&LcaError::ExceededK), MissingBridgeReason::ExceededK);
        assert_eq!(
            map_lca_error(&LcaError::CacheSelfBindingViolation),
            MissingBridgeReason::CacheSelfBindingViolation
        );
        assert_eq!(MissingBridgeReason::BranchGap.as_str(), "branch_gap");
        assert_eq!(MissingBridgeReason::LcaUnreachable.as_str(), "lca_unreachable");
    }

    #[test]
    fn empty_branch_fails_closed_before_any_apply() {
        // A degenerate winning candidate (no headers) must fail closed BEFORE the
        // driver could ever commit a rollback — never an empty, half-switched apply.
        let stub = ade_testkit::consensus::ledger_view_stub::LedgerViewStub::new();
        let err = prevalidate_branch(
            &anchor(),
            &empty_candidate(),
            &[],
            &LedgerState::new(ade_types::CardanoEra::Conway),
            &PraosChainDepState::empty(),
            &dummy_schedule(),
            &stub,
        )
        .expect_err("empty branch must fail closed");
        assert_eq!(err, BranchProofError::EmptyBranch);
    }

    fn dummy_schedule() -> EraSchedule {
        use ade_core::consensus::era_schedule::{BootstrapAnchorHash, EraSummary};
        use ade_types::{CardanoEra, EpochNo};
        EraSchedule::new(
            BootstrapAnchorHash(Hash32([0u8; 32])),
            0,
            vec![EraSummary {
                era: CardanoEra::Conway,
                start_slot: SlotNo(0),
                start_epoch: EpochNo(0),
                slot_length_ms: 1_000,
                epoch_length_slots: 432_000,
                safe_zone_slots: 129_600,
            }],
        )
        .expect("schedule")
    }
}
