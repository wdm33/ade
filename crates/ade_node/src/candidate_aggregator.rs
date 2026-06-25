// Core Contract:
// - Deterministic: same inputs => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - No I/O, no store reads, no durable mutation

//! GREEN — BLUE-safe candidate construction (PHASE4-N-AO S2, `DC-NODE-35`).
//!
//! A PURE projection: given a fork anchor, the chain-dep AT that anchor, and a
//! peer's candidate header inputs, validate each header through the BLUE
//! `validate_and_apply_header` authority and assemble a [`CandidateFragment`]
//! for the BLUE fork-choice selector (`select_best_chain`, consumed live in S3).
//!
//! Hard boundary (`DC-NODE-35` / OQ-AO-6 -> GREEN): this module performs **no
//! store reads, no materialization, no selection, no block-fetch, no WAL, and no
//! durable mutation**. It mints nothing — every `ValidatedHeaderSummary` placed
//! in a fragment is `validate_and_apply_header` output, or the candidate is
//! rejected (fail-closed).
//!
//! Forward obligation to S3: the live RED driver MUST obtain `anchor_chain_dep`
//! by a read-only `materialize_rolled_back_state` from Ade's durable stored fork
//! anchor before calling here — peer-supplied fork state must never reach this
//! core.

use ade_core::consensus::candidate::{CandidateFragment, TiebreakerView};
use ade_core::consensus::era_schedule::EraSchedule;
use ade_core::consensus::errors::HeaderValidationError;
use ade_core::consensus::events::{BlockDistance, Point};
use ade_core::consensus::header_summary::HeaderInput;
use ade_core::consensus::header_validate::validate_and_apply_header;
use ade_core::consensus::ledger_view::LedgerView;
use ade_core::consensus::praos_state::PraosChainDepState;
use ade_types::BlockNo;

/// Closed failure surface for candidate construction. **Fail-closed:** a
/// candidate that does not fully validate produces NO fragment — never a minted
/// one.
#[derive(Debug)]
pub enum CandidateBuildError {
    /// A candidate header (0-based `index` in the supplied list) was rejected by
    /// the BLUE `validate_and_apply_header` authority.
    HeaderInvalid {
        index: usize,
        error: HeaderValidationError,
    },
    /// A candidate fragment must carry at least one header above the anchor.
    EmptyHeaders,
}

/// Build ONE peer's [`CandidateFragment`] by validating its headers above the
/// fork anchor. PURE: no I/O, no store, no materialize, no selection.
///
/// - `anchor` / `anchor_block_no` — the fork point (a block on Ade's durable
///   chain; supplied by the S3 driver, never peer data).
/// - `anchor_chain_dep` — the chain-dep AT that point (the S3 driver obtains it
///   by a read-only `materialize_rolled_back_state`; never peer data).
/// - `current_tip_block_no` — Ade's selector tip block number (a supplied
///   value, not a store read), used only to derive `rollback_depth`.
/// - `headers` — the candidate's header inputs above the anchor, in order.
///
/// Each `HeaderInput` is validated in order through `validate_and_apply_header`,
/// evolving a working chain-dep seeded from `anchor_chain_dep`; the resulting
/// `ValidatedHeaderSummary`s — and ONLY those — populate the fragment. Any
/// validation failure fails closed (`HeaderInvalid`); zero headers fail closed
/// (`EmptyHeaders`).
#[allow(clippy::too_many_arguments)]
pub fn build_candidate_fragment(
    anchor: Point,
    anchor_block_no: BlockNo,
    current_tip_block_no: BlockNo,
    anchor_chain_dep: &PraosChainDepState,
    headers: &[HeaderInput],
    ledger_view: &dyn LedgerView,
    era_schedule: &EraSchedule,
) -> Result<CandidateFragment, CandidateBuildError> {
    if headers.is_empty() {
        return Err(CandidateBuildError::EmptyHeaders);
    }

    // Thread a working chain-dep through the BLUE header authority, collecting
    // ONLY its validated summaries. Seeded from the supplied anchor chain-dep.
    let mut chain_dep = anchor_chain_dep.clone();
    let mut summaries = Vec::with_capacity(headers.len());
    for (index, header) in headers.iter().enumerate() {
        let applied = validate_and_apply_header(&chain_dep, header, ledger_view, era_schedule)
            .map_err(|error| CandidateBuildError::HeaderInvalid { index, error })?;
        chain_dep = applied.new_state;
        summaries.push(applied.summary);
    }

    // The candidate tip is the last validated header. Its TiebreakerView is the
    // fragment's select_view — derived from validated values only (slot, issuer,
    // op-cert counter, leader-VRF prefix), never a peer claim. Block-number-first
    // ordering and eligibility are select_best_chain's (S3); here we only project.
    let select_view = match summaries.last() {
        Some(tip) => {
            let mut leader_vrf_output_first_8 = [0u8; 8];
            leader_vrf_output_first_8.copy_from_slice(&tip.vrf_leader_output.0[0..8]);
            TiebreakerView {
                slot: tip.slot,
                issuer_hash: tip.issuer_pool.clone(),
                op_cert_counter: tip.op_cert_counter,
                leader_vrf_output_first_8,
            }
        }
        // Unreachable: headers is non-empty (checked above) and the loop pushes
        // one summary per header. Fail closed rather than panic.
        None => return Err(CandidateBuildError::EmptyHeaders),
    };

    // rollback_depth = how many blocks Ade rolls back from its current tip to the
    // fork anchor. select_best_chain k-bounds it (S3); we only compute it.
    let rollback_depth = BlockDistance(current_tip_block_no.0.saturating_sub(anchor_block_no.0));

    Ok(CandidateFragment {
        anchor,
        anchor_block_no,
        headers: summaries,
        select_view,
        rollback_depth,
    })
}

/// Assemble the candidate set for `select_best_chain` in a DETERMINISTIC order,
/// independent of peer / arrival order. PURE.
///
/// `select_best_chain` is arrival-order-independent (`CN-CONS-01`), so the order
/// does not change the selected tip; a canonical order is required only so the
/// construction is deterministic. The order key is the candidate tip's canonical
/// identity: (tip block number, then `TiebreakerView` field order). This is a
/// total sort key, NOT the Praos preference (that lives in `select_best_chain`).
pub fn assemble_candidate_set(mut fragments: Vec<CandidateFragment>) -> Vec<CandidateFragment> {
    fragments.sort_by(|a, b| {
        let a_tip = a.anchor_block_no.0.saturating_add(a.headers.len() as u64);
        let b_tip = b.anchor_block_no.0.saturating_add(b.headers.len() as u64);
        a_tip
            .cmp(&b_tip)
            .then_with(|| tiebreaker_sort_key(&a.select_view, &b.select_view))
    });
    fragments
}

/// Total deterministic order over `TiebreakerView` fields — a canonical sort key
/// for the candidate set, NOT the Praos preference.
fn tiebreaker_sort_key(a: &TiebreakerView, b: &TiebreakerView) -> std::cmp::Ordering {
    a.slot
        .0
        .cmp(&b.slot.0)
        .then_with(|| a.issuer_hash.0.cmp(&b.issuer_hash.0))
        .then_with(|| a.op_cert_counter.cmp(&b.op_cert_counter))
        .then_with(|| a.leader_vrf_output_first_8.cmp(&b.leader_vrf_output_first_8))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    use std::collections::BTreeMap;

    use ade_core::consensus::era_schedule::{BootstrapAnchorHash, EraSummary};
    use ade_core::consensus::header_summary::HeaderVrf;
    use ade_core::consensus::praos_state::Nonce;
    use ade_core::consensus::vrf_cert::{vrf_input, ActiveSlotsCoeff, VrfRole};
    use ade_crypto::vrf::{VrfProof, VrfVerificationKey};
    use ade_testkit::consensus::ledger_view_stub::{
        EpochStakeFixture, LedgerViewStub, PoolFixture,
    };
    use ade_types::{CardanoEra, EpochNo, Hash28, Hash32, SlotNo};
    use cardano_crypto::vrf::VrfDraft03;

    // Fixtures mirror ade_runtime::consensus::chain_selector's tests: a single
    // Shelley era, one pool with asc = 1/1 (every VRF output trivially leads), and
    // TPraos headers signed with a deterministic VRF keypair.

    fn schedule() -> EraSchedule {
        let eras = vec![EraSummary {
            era: CardanoEra::Shelley,
            start_slot: SlotNo(0),
            start_epoch: EpochNo(0),
            slot_length_ms: 1_000,
            epoch_length_slots: 432_000,
            safe_zone_slots: 129_600,
        }];
        EraSchedule::new(BootstrapAnchorHash(Hash32([0u8; 32])), 0, eras).expect("schedule")
    }

    fn pool() -> Hash28 {
        Hash28([0xAA; 28])
    }

    fn keypair() -> ([u8; 64], VrfVerificationKey) {
        let (sk, vk_bytes) = VrfDraft03::keypair_from_seed(&[7u8; 32]);
        (sk, VrfVerificationKey(vk_bytes))
    }

    fn prove(sk: &[u8; 64], slot: SlotNo, epoch_nonce: &Nonce, role: VrfRole) -> VrfProof {
        let alpha = vrf_input(slot, epoch_nonce, role);
        VrfProof(VrfDraft03::prove(sk, &alpha).expect("prove"))
    }

    fn ledger(vk: VrfVerificationKey) -> LedgerViewStub {
        let mut pools = BTreeMap::new();
        pools.insert(
            pool(),
            PoolFixture {
                active_stake: 1,
                vrf_keyhash: ade_crypto::blake2b::blake2b_256(&vk.0),
            },
        );
        let mut stub = LedgerViewStub::new().with_epoch(
            EpochNo(0),
            EpochStakeFixture {
                total_active_stake: 1,
                asc: ActiveSlotsCoeff { numer: 1, denom: 1 },
                pools: pools.clone(),
            },
        );
        stub = stub.with_epoch(
            EpochNo(1),
            EpochStakeFixture {
                total_active_stake: 1,
                asc: ActiveSlotsCoeff { numer: 1, denom: 1 },
                pools,
            },
        );
        stub
    }

    /// A genesis-like anchor chain-dep (no blocks applied yet). The seed-epoch
    /// nonce basis is constant, so candidate headers validate against the same
    /// epoch_nonce regardless of how many have been applied within the epoch.
    fn anchor_chain_dep() -> PraosChainDepState {
        let mut s = PraosChainDepState::empty();
        s.epoch_nonce = Nonce(Hash32([0xCD; 32]));
        s.evolving_nonce = Nonce(Hash32([0xEE; 32]));
        s.candidate_nonce = Nonce(Hash32([0xCD; 32]));
        s
    }

    fn header_at(
        sk: &[u8; 64],
        vk: &VrfVerificationKey,
        epoch_nonce: &Nonce,
        slot: SlotNo,
        block_no: BlockNo,
        op_cert_counter: u64,
    ) -> HeaderInput {
        HeaderInput {
            slot,
            block_no,
            prev_hash: Hash32([0u8; 32]),
            body_hash: Hash32([0x55; 32]),
            issuer_pool: pool(),
            op_cert_kes_period: 0,
            op_cert_counter,
            vrf_vk: vk.clone(),
            vrf: HeaderVrf::Tpraos {
                nonce_proof: prove(sk, slot, epoch_nonce, VrfRole::NonceContribution),
                leader_proof: prove(sk, slot, epoch_nonce, VrfRole::LeaderEligibility),
            },
            kes: None,
        }
    }

    fn anchor_point() -> Point {
        Point {
            slot: SlotNo(0),
            hash: Hash32([0u8; 32]),
        }
    }

    #[test]
    fn build_candidate_fragment_assembles_from_validated_headers() {
        let (sk, vk) = keypair();
        let cd = anchor_chain_dep();
        // Two candidate headers above the genesis anchor (block 0): blocks 1, 2.
        // op_cert_counter is non-decreasing per block (BLUE op-cert monotonicity).
        let h1 = header_at(&sk, &vk, &cd.epoch_nonce, SlotNo(1), BlockNo(1), 1);
        let h2 = header_at(&sk, &vk, &cd.epoch_nonce, SlotNo(2), BlockNo(2), 2);

        let frag = build_candidate_fragment(
            anchor_point(),
            BlockNo(0),
            BlockNo(5),
            &cd,
            &[h1, h2],
            &ledger(vk),
            &schedule(),
        )
        .expect("valid candidate builds a fragment");

        assert_eq!(frag.anchor_block_no, BlockNo(0));
        assert_eq!(frag.headers.len(), 2, "both validated headers present");
        assert_eq!(frag.headers[0].block_no, BlockNo(1));
        assert_eq!(frag.headers[1].block_no, BlockNo(2));
        // rollback_depth = current_tip (5) - anchor (0) = 5.
        assert_eq!(frag.rollback_depth, BlockDistance(5));
        // select_view is the tip header's tiebreaker (block 2 @ slot 2, counter 2).
        assert_eq!(frag.select_view.slot, SlotNo(2));
        assert_eq!(frag.select_view.op_cert_counter, 2);
        assert_eq!(frag.select_view.issuer_hash, pool());
    }

    #[test]
    fn build_candidate_fragment_rejects_invalid_header_fails_closed() {
        let (sk, vk) = keypair();
        let cd = anchor_chain_dep();
        let mut bad = header_at(&sk, &vk, &cd.epoch_nonce, SlotNo(1), BlockNo(1), 1);
        // Corrupt the leader VRF proof -> validate_and_apply_header fails the VRF.
        if let HeaderVrf::Tpraos { leader_proof, .. } = &mut bad.vrf {
            leader_proof.0[0] ^= 0xFF;
        }

        let err = build_candidate_fragment(
            anchor_point(),
            BlockNo(0),
            BlockNo(5),
            &cd,
            &[bad],
            &ledger(vk),
            &schedule(),
        )
        .expect_err("an invalid candidate header must fail closed -- no minted fragment");
        assert!(
            matches!(err, CandidateBuildError::HeaderInvalid { index: 0, .. }),
            "got {err:?}"
        );
    }

    #[test]
    fn build_candidate_fragment_empty_headers_fails_closed() {
        let (_sk, vk) = keypair();
        let cd = anchor_chain_dep();
        let err = build_candidate_fragment(
            anchor_point(),
            BlockNo(0),
            BlockNo(5),
            &cd,
            &[],
            &ledger(vk),
            &schedule(),
        )
        .expect_err("zero headers must fail closed");
        assert!(matches!(err, CandidateBuildError::EmptyHeaders));
    }

    #[test]
    fn build_candidate_fragment_two_runs_byte_identical() {
        let (sk, vk) = keypair();
        let cd = anchor_chain_dep();
        let mk = || {
            let h1 = header_at(&sk, &vk, &cd.epoch_nonce, SlotNo(1), BlockNo(1), 1);
            build_candidate_fragment(
                anchor_point(),
                BlockNo(0),
                BlockNo(5),
                &cd,
                &[h1],
                &ledger(vk.clone()),
                &schedule(),
            )
            .unwrap()
        };
        assert_eq!(mk(), mk(), "same inputs => byte-identical fragment");
    }

    #[test]
    fn assemble_candidate_set_ordering_is_arrival_independent() {
        let (sk, vk) = keypair();
        let cd = anchor_chain_dep();
        // frag_a: candidate tip at block 1; frag_b: candidate tip at block 2.
        let h_a = header_at(&sk, &vk, &cd.epoch_nonce, SlotNo(1), BlockNo(1), 1);
        let h_b1 = header_at(&sk, &vk, &cd.epoch_nonce, SlotNo(1), BlockNo(1), 1);
        let h_b2 = header_at(&sk, &vk, &cd.epoch_nonce, SlotNo(2), BlockNo(2), 2);
        let frag_a = build_candidate_fragment(
            anchor_point(),
            BlockNo(0),
            BlockNo(5),
            &cd,
            &[h_a],
            &ledger(vk.clone()),
            &schedule(),
        )
        .unwrap();
        let frag_b = build_candidate_fragment(
            anchor_point(),
            BlockNo(0),
            BlockNo(5),
            &cd,
            &[h_b1, h_b2],
            &ledger(vk.clone()),
            &schedule(),
        )
        .unwrap();

        let set1 = assemble_candidate_set(vec![frag_a.clone(), frag_b.clone()]);
        let set2 = assemble_candidate_set(vec![frag_b, frag_a]);
        assert_eq!(
            set1, set2,
            "candidate-set order must be independent of input/arrival order"
        );
        // Sorted by tip block number: frag_a (tip 1) before frag_b (tip 2).
        assert_eq!(set1[0].headers.len(), 1);
        assert_eq!(set1[1].headers.len(), 2);
    }
}
