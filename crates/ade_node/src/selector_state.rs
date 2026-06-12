// Core Contract:
// - Deterministic: same inputs => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - No I/O, no store reads, no durable mutation

//! GREEN — selector-state projection for live fork-choice dispatch
//! (PHASE4-N-AO S3, `DC-NODE-36`).
//!
//! Pure projections the RED dispatch driver (`run_participant_sync`) uses to
//! build the `ChainSelectorState` from **local durable authority** and to carry
//! a provisional fork-choice decision toward S4.
//!
//! `project_tiebreaker` derives the `TiebreakerView` of a block **from Ade's
//! own already-admitted durable tip bytes** — local durable authority, NOT
//! peer-derived, NOT revalidated, NOT minted. The leader value it projects
//! (`praos_leader_value` for Conway/Praos) is bit-identical to the one
//! `validate_and_apply_header` places in a `ValidatedHeaderSummary`, so the
//! current tip and the (S2-validated) candidates compare on the same basis.
//!
//! `PendingForkSwitch` is the **provisional** decision S3 emits on a fork-choice
//! win — consumed by S4, which fetches + validates the replacement branch and
//! applies the rollback. S3 never applies it (the hard S3/S4 boundary).

use ade_core::consensus::candidate::{CandidateFragment, TiebreakerView};
use ade_core::consensus::events::Point;
use ade_core::consensus::header_summary::{HeaderInput, HeaderVrf};
use ade_core::consensus::praos_leader_value;
use ade_types::{BlockNo, Hash32, SlotNo};

/// Closed failure surface for the selector-state projection. Fail-closed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectorProjectError {
    /// The durable tip is a legacy TPraos header. Unsupported on the Conway
    /// participant path (the live venue is Praos); fail closed rather than
    /// project an inconsistent tiebreaker.
    UnsupportedTpraosTip,
}

/// Project the `TiebreakerView` of a header from its already-decoded inputs.
/// PURE. For Conway/Praos the leader value is `praos_leader_value(output)` —
/// the same value `validate_and_apply_header` records in the summary, so the
/// current tip and S2-validated candidates are comparable. The caller supplies
/// a header decoded from Ade's **own durable** tip bytes (local authority).
pub fn project_tiebreaker(header: &HeaderInput) -> Result<TiebreakerView, SelectorProjectError> {
    let leader_value = match &header.vrf {
        HeaderVrf::Praos { output, .. } => praos_leader_value(output),
        HeaderVrf::Tpraos { .. } => return Err(SelectorProjectError::UnsupportedTpraosTip),
    };
    let mut leader_vrf_output_first_8 = [0u8; 8];
    leader_vrf_output_first_8.copy_from_slice(&leader_value.0[0..8]);
    Ok(TiebreakerView {
        slot: header.slot,
        issuer_hash: header.issuer_pool.clone(),
        op_cert_counter: header.op_cert_counter,
        leader_vrf_output_first_8,
    })
}

/// A fork anchor — the durable chain point a competing branch forks from.
/// Bound to Ade's **durable stored** `(slot, hash, block_no)` by the RED driver
/// (`get_block_by_hash`), never peer-supplied (`DC-NODE-29` discipline).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForkAnchor {
    pub slot: SlotNo,
    pub hash: Hash32,
    pub block_no: BlockNo,
}

/// A PROVISIONAL fork-choice decision emitted by S3 on a `ChainSelected` win —
/// consumed by S4. S3 sets this and the `DC-NODE-28` forge fence but **applies
/// nothing**: no body-fetch, no rollback-commit, no `WalEntry::RollBack`. The
/// current durable chain is preserved until S4 fetches + validates the
/// replacement branch as a complete candidate branch (FC-6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingForkSwitch {
    /// The durable-bound fork point the replacement branch forks from.
    pub fork_anchor: ForkAnchor,
    /// The peer whose candidate won fork-choice (the branch S4 fetches from).
    pub winning_peer: String,
    /// The winning candidate (S2-validated header summaries above the anchor).
    pub winning_candidate: CandidateFragment,
    /// PHASE4-N-AO S6 (CE-AO-6): the selected winner's tip point `(slot, block
    /// hash)`, retained from the competing block S3 decoded. It is the
    /// **`BlockFetch RequestRange` upper endpoint** (`fork_anchor → winner_tip`)
    /// the live fetch asks the winning peer for.
    ///
    /// **`winner_tip` is a fetch endpoint ONLY — it is NOT adoption authority.**
    /// The fetched body must still bind to the S3-selected `ValidatedHeaderSummary`
    /// and pass S4 `prevalidate_branch` before any rollback is committed. A peer
    /// that serves a different body for this endpoint is rejected by S4
    /// (`BodyHeaderMismatch`), not adopted.
    pub winner_tip: Point,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    use ade_core::consensus::header_summary::HeaderVrf;
    use ade_crypto::vrf::{VrfOutput, VrfProof, VrfVerificationKey};
    use ade_types::Hash28;

    fn praos_header(slot: u64, block_no: u64, counter: u64, output_byte: u8) -> HeaderInput {
        HeaderInput {
            slot: SlotNo(slot),
            block_no: BlockNo(block_no),
            body_hash: Hash32([0x55; 32]),
            issuer_pool: Hash28([0xAA; 28]),
            op_cert_kes_period: 0,
            op_cert_counter: counter,
            vrf_vk: VrfVerificationKey([0u8; 32]),
            vrf: HeaderVrf::Praos {
                proof: VrfProof([0u8; 80]),
                output: VrfOutput([output_byte; 64]),
            },
            kes: None,
        }
    }

    #[test]
    fn project_tiebreaker_praos_matches_leader_value() {
        let h = praos_header(7, 51, 3, 0x11);
        let tb = project_tiebreaker(&h).expect("praos tip projects");
        // The vrf field is the FIRST 8 bytes of praos_leader_value(output) -- the
        // same leader value validate_and_apply_header records, so the current tip
        // and S2-validated candidates compare on the same basis.
        let expected = praos_leader_value(&VrfOutput([0x11; 64]));
        let mut want = [0u8; 8];
        want.copy_from_slice(&expected.0[0..8]);
        assert_eq!(tb.slot, SlotNo(7));
        assert_eq!(tb.op_cert_counter, 3);
        assert_eq!(tb.issuer_hash, Hash28([0xAA; 28]));
        assert_eq!(tb.leader_vrf_output_first_8, want);
    }

    #[test]
    fn project_tiebreaker_is_deterministic() {
        let h = praos_header(7, 51, 3, 0x22);
        assert_eq!(project_tiebreaker(&h), project_tiebreaker(&h));
    }

    #[test]
    fn project_tiebreaker_tpraos_fails_closed() {
        let h = HeaderInput {
            slot: SlotNo(7),
            block_no: BlockNo(51),
            body_hash: Hash32([0x55; 32]),
            issuer_pool: Hash28([0xAA; 28]),
            op_cert_kes_period: 0,
            op_cert_counter: 1,
            vrf_vk: VrfVerificationKey([0u8; 32]),
            vrf: HeaderVrf::Tpraos {
                nonce_proof: VrfProof([0u8; 80]),
                leader_proof: VrfProof([0u8; 80]),
            },
            kes: None,
        };
        assert_eq!(
            project_tiebreaker(&h),
            Err(SelectorProjectError::UnsupportedTpraosTip)
        );
    }
}
