// Core Contract:
// - Deterministic, no I/O.
// - The REDUCED-VALIDATION-BOUNDARY-PLANE: a track_utxo=false follower crossing a Conway epoch boundary produces
//   a typed reduced projection, NOT a degraded full transition and NOT a fabricated snapshot. See
//   docs/clusters/REDUCED-VALIDATION-BOUNDARY-PLANE/INVARIANTS.md.

//! Typed reduced boundary projection (P1). The authoritative epoch boundary
//! (`rules::apply_epoch_boundary_with_registrations`) is the SOLE producer of RUPD-applied rewards, pots,
//! mark/set/go, governance/pparam enactment, and CE-3d-comparable state — it REQUIRES point-bound
//! `BoundaryBaseStake`. When a reduced follower (`track_utxo=false`) reaches a Conway boundary it lacks those
//! inputs, so instead of building a reward-only stub mark (which the trace proved leaks into recovery
//! persistence + WAL fingerprints) it produces a [`ReducedBoundaryProjection`]: epoch/slot progression + a
//! distinct [`ReducedEpochProgress`] block-window rollover ONLY. Everything authoritative is UNAVAILABLE.
//!
//! The two results are DISTINCT, non-interchangeable types (I-RVB-1): a `ReducedBoundaryProjection` is not a
//! `LedgerState`, cannot be widened into one, cannot be serialized as an accumulator snapshot, and cannot be
//! fingerprinted as full authority.

use ade_types::{EpochNo, SlotNo};

use crate::governance::GovernanceTerminal;
use crate::rules::EpochBoundaryAccounting;
use crate::state::LedgerState;

/// The structural block-production window rolled over at a reduced boundary — the ONLY nesBcur-adjacent fact the
/// reduced plane may record. It is DELIBERATELY not the full `block_production`/`epoch_fees` reward-calculation
/// input: it carries no per-pool counts and no fees, so it CANNOT be fed to RUPD or epoch authority. Converting a
/// reduced follower's window into a full reward input requires a `FullBoundaryStateRequired` failure, never a
/// silent widening (N-RVB-4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReducedBlockWindow {
    /// The reduced plane observed that the previous epoch's block-production window closed at this boundary.
    /// A structural fact (the window rolled), NOT the reward-bearing per-pool nesBprev.
    pub rolled_at_epoch: EpochNo,
}

/// The reduced facts advanced across a `track_utxo=false` Conway boundary: epoch/slot progression + the reduced
/// block-window rollover, and the EXPLICITLY-UNAVAILABLE cert/governance projections. Read-set audited
/// (INVARIANTS §read-set): NOTHING else — no rewards, no pots, no mark/set/go, no POOLREAP, no governance/pparam
/// enactment — is derivable from reduced inputs, so nothing else is present (absent, never stale/empty/inferred).
/// The `cert_projection`/`governance_projection` fields make the absence STRUCTURAL: they are unavailable BY TYPE
/// (a `ReducedCertProjection`/`ReducedGovernanceProjection` cannot hold a full `CertState`/`ConwayGovState`), so a
/// reduced crossing can never present a normal cert/gov surface that looks like advanced ledger state (deviation 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReducedEpochProgress {
    pub epoch: EpochNo,
    pub slot: SlotNo,
    pub reduced_block_window: ReducedBlockWindow,
    /// The certificate/pool-lifecycle capability advanced — always `Unavailable` (no POOLREAP, no cert lifecycle).
    pub cert_projection: ReducedCertProjection,
    /// The governance capability advanced — always `Unavailable` (no ratify/enact, no gov state carried).
    pub governance_projection: ReducedGovernanceProjection,
}

/// The certificate/pool-lifecycle capability a reduced boundary advances. POOLREAP is unavailable in the reduced
/// plane (splitting its pool/delegation cleanup from its reward refund would be a hybrid state matching neither
/// cardano nor the accumulator — N-RVB-3), and a reduced follower does NOT apply certificates across the boundary
/// (verified: `track_utxo=false` within-epoch block processing carries `cert_state` structurally and never
/// evolves it; leadership/forge read the accumulator's `PoolDistrView`, never this). So a reduced boundary claims
/// NO advanced certificate or pool lifecycle. This is a NAMED absence, not a full `CertState` carried "just in
/// case": a reduced projection must not be dereferenceable or promotable to a fully-advanced (post-POOLREAP)
/// lifecycle. `StructuralFields` is reserved for the day a reduced structural check proves it needs a specific
/// field; until then, `Unavailable` (RVBP N-RVB-2/3, deviation 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReducedCertProjection {
    /// No certificate/pool-lifecycle authority — the reduced plane advanced none. The default and only current arm.
    Unavailable,
}

/// The governance capability a reduced boundary advances. A reduced follower ratifies/enacts NO governance action
/// and carries NO `ConwayGovState` across the boundary (that is the accumulator's authority). Like
/// [`ReducedCertProjection`] this is a NAMED absence, not a full/empty `ConwayGovState` carried "just in case": a
/// reduced projection must not be dereferenceable or promotable to enacted governance state. `Unavailable` is the
/// only current arm (RVBP N-RVB-3, deviation 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReducedGovernanceProjection {
    /// No governance authority — the reduced plane ratified/enacted none. The default and only current arm.
    Unavailable,
}

/// A reduced-plane boundary result. A DISTINCT type from the authoritative `FullEpochBoundaryResult` /
/// `LedgerState` — it carries ONLY [`ReducedEpochProgress`] and a [`ReducedCertProjection`] (never a full
/// `CertState`), so it is structurally incapable of holding a snapshot, reward account, pot, governance effect, or
/// an advanced certificate lifecycle. It can never feed `ActiveEpochAuthority`, leadership, forging, CE-3d, or a
/// full-ledger verdict (I-RVB-4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReducedBoundaryProjection {
    /// The reduced facts + the explicitly-unavailable cert/governance projections. There is no separate full
    /// `CertState`/`ConwayGovState` anywhere on this type — the projections live on `progress`, unavailable by type.
    pub progress: ReducedEpochProgress,
}

impl ReducedBoundaryProjection {
    /// Cross a `track_utxo=false` epoch boundary in the reduced plane. Advances ONLY epoch (to `new_epoch`) and
    /// records the block-window rollover; the header slot carries forward as the boundary point. Pure. No reward,
    /// pot, snapshot, POOLREAP, or governance effect is computed, and the certificate lifecycle is
    /// `Unavailable` — the reduced plane lacks the inputs to advance any of these correctly, and a partial/stale
    /// stand-in is forbidden (N-RVB-2/3).
    pub fn cross(from_epoch: EpochNo, new_epoch: EpochNo, slot: SlotNo) -> Self {
        ReducedBoundaryProjection {
            progress: ReducedEpochProgress {
                epoch: new_epoch,
                slot,
                reduced_block_window: ReducedBlockWindow { rolled_at_epoch: from_epoch },
                // Cert AND governance are unavailable BY TYPE at a reduced crossing — no lifecycle, no ratify/enact.
                cert_projection: ReducedCertProjection::Unavailable,
                governance_projection: ReducedGovernanceProjection::Unavailable,
            },
        }
    }
}

/// The authoritative boundary result — the full transition's output: the post-boundary `LedgerState` (post-RUPD
/// rewards, rotated mark/set/go, pots, governance/pparam enactment) plus its `EpochBoundaryAccounting`. This is
/// the ONLY value that may feed `ActiveEpochAuthority`, leadership, forging, CE-3d state comparison, or a
/// full-ledger verdict (I-RVB-4). It is produced solely by the authoritative `EpochAccumulator` boundary; the
/// reduced plane can never construct one.
#[derive(Debug, Clone)]
pub struct FullEpochBoundaryResult {
    pub ledger: LedgerState,
    pub accounting: EpochBoundaryAccounting,
}

/// The two non-interchangeable epoch-boundary results (I-RVB-1). A boundary yields EITHER the authoritative
/// `Full` result OR a `Reduced` projection. There is NO `From<ReducedBoundaryProjection>` and no field access
/// that widens a reduced projection into authority — the ONLY way to obtain the full result is [`require_full`],
/// which fails closed with `FullBoundaryStateRequired` on `Reduced` (N-RVB-4).
///
/// [`require_full`]: LedgerBoundaryVerdict::require_full
#[derive(Debug, Clone)]
pub enum LedgerBoundaryVerdict {
    Full(FullEpochBoundaryResult),
    Reduced(ReducedBoundaryProjection),
}

impl LedgerBoundaryVerdict {
    /// Extract the authoritative full result, or fail closed. A `Reduced` projection can never be silently
    /// widened into epoch authority (gate 6 / I-RVB-1); a caller that needs full state after a reduced boundary
    /// receives `GovernanceTerminal::FullBoundaryStateRequired`, never a fabricated or empty stand-in.
    pub fn require_full(
        self,
        boundary_point: SlotNo,
    ) -> Result<FullEpochBoundaryResult, GovernanceTerminal> {
        match self {
            LedgerBoundaryVerdict::Full(f) => Ok(f),
            LedgerBoundaryVerdict::Reduced(_) => {
                Err(GovernanceTerminal::FullBoundaryStateRequired { boundary_point })
            }
        }
    }
}

/// Verdict capability (I-RVB-3). A reduced-plane structural check (`block_validity`, header/cert continuation)
/// yields `StructuralValidity`: it asserts the block is structurally well-formed, NOT that the full ledger
/// (rewards, governance, pots) was validated. It can NEVER be promoted to `FullLedgerValidity` merely because a
/// boundary crossed without error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedgerValidityCapability {
    StructuralValidity,
    FullLedgerValidity,
}

impl LedgerValidityCapability {
    /// Require a full-ledger verdict, or fail closed. `StructuralValidity` → `FullBoundaryStateRequired` (a
    /// structural pass is never a full-ledger authority claim); `FullLedgerValidity` → `Ok(())`.
    pub fn require_full_ledger(self, boundary_point: SlotNo) -> Result<(), GovernanceTerminal> {
        match self {
            LedgerValidityCapability::FullLedgerValidity => Ok(()),
            LedgerValidityCapability::StructuralValidity => {
                Err(GovernanceTerminal::FullBoundaryStateRequired { boundary_point })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reduced_cross_advances_only_epoch_slot_window() {
        let p = ReducedBoundaryProjection::cross(EpochNo(1340), EpochNo(1341), SlotNo(115_862_416));
        assert_eq!(p.progress.epoch, EpochNo(1341), "epoch advances to the new epoch");
        assert_eq!(p.progress.slot, SlotNo(115_862_416), "the boundary slot carries forward");
        assert_eq!(
            p.progress.reduced_block_window.rolled_at_epoch,
            EpochNo(1340),
            "the reduced block window records the rolled-from epoch, nothing reward-bearing",
        );
    }

    /// The reduced projection is structurally incapable of carrying authoritative state — there is no field for a
    /// snapshot, reward, pot, governance effect, or a full `CertState`. This test documents that the type itself is
    /// the guarantee (I-RVB-1/N-RVB-4): the only public surface is epoch/slot + the reduced window + a cert
    /// projection that is `Unavailable` (never an advanced lifecycle, deviation 2).
    #[test]
    fn reduced_projection_has_no_authority_surface() {
        let p = ReducedBoundaryProjection::cross(EpochNo(1), EpochNo(2), SlotNo(10));
        // The whole type is Copy + carries only ReducedEpochProgress (epoch/slot/window + the two projections) —
        // nothing to convert into a reward input, no full CertState or ConwayGovState to be mistaken for
        // post-POOLREAP lifecycle / enacted governance state.
        let _progress: ReducedEpochProgress = p.progress;
        assert_eq!(
            p.progress.cert_projection,
            ReducedCertProjection::Unavailable,
            "the reduced boundary claims NO advanced certificate/pool lifecycle (never a full CertState)",
        );
        assert_eq!(
            p.progress.governance_projection,
            ReducedGovernanceProjection::Unavailable,
            "the reduced boundary claims NO governance authority (never a full ConwayGovState)",
        );
    }

    /// GATE 6 / I-RVB-1: a reduced boundary result can NEVER be widened into the authoritative full result. The
    /// only extraction door (`require_full`) fails closed with `FullBoundaryStateRequired`, carrying the boundary
    /// point — never a fabricated or empty full state.
    #[test]
    fn reduced_boundary_verdict_never_widens_to_full() {
        let reduced = LedgerBoundaryVerdict::Reduced(ReducedBoundaryProjection::cross(
            EpochNo(1340),
            EpochNo(1341),
            SlotNo(115_862_416),
        ));
        let r = reduced.require_full(SlotNo(115_862_416));
        assert!(
            matches!(
                r,
                Err(GovernanceTerminal::FullBoundaryStateRequired { boundary_point })
                    if boundary_point == SlotNo(115_862_416)
            ),
            "Reduced.require_full must fail closed with FullBoundaryStateRequired at the boundary point",
        );
    }

    /// The authoritative arm opens the door: `Full.require_full` returns the wrapped result unchanged (the sole
    /// value eligible to feed epoch authority).
    #[test]
    fn full_boundary_verdict_yields_the_authoritative_result() {
        let full = LedgerBoundaryVerdict::Full(FullEpochBoundaryResult {
            ledger: LedgerState::new(ade_types::CardanoEra::Shelley),
            accounting: zero_accounting(),
        });
        let r = full.require_full(SlotNo(1));
        assert!(r.is_ok(), "Full.require_full returns the authoritative result");
        assert_eq!(r.unwrap().ledger.era, ade_types::CardanoEra::Shelley, "the wrapped ledger is preserved");
    }

    /// I-RVB-3: a structural pass can NEVER be promoted to a full-ledger verdict just because a boundary crossed
    /// without error; only a genuine `FullLedgerValidity` satisfies a full-ledger requirement.
    #[test]
    fn structural_validity_is_never_promoted_to_full_ledger() {
        assert!(
            matches!(
                LedgerValidityCapability::StructuralValidity.require_full_ledger(SlotNo(9)),
                Err(GovernanceTerminal::FullBoundaryStateRequired { .. })
            ),
            "StructuralValidity must fail closed when a full-ledger verdict is required",
        );
        assert!(
            LedgerValidityCapability::FullLedgerValidity
                .require_full_ledger(SlotNo(9))
                .is_ok(),
            "FullLedgerValidity satisfies a full-ledger requirement",
        );
    }

    /// All-zero `EpochBoundaryAccounting` for constructing a `FullEpochBoundaryResult` in tests (the values are
    /// irrelevant to the capability gate; only the type identity matters).
    fn zero_accounting() -> EpochBoundaryAccounting {
        EpochBoundaryAccounting {
            delta_r1: 0,
            delta_r2: 0,
            delta_t1: 0,
            delta_t2: 0,
            total_reward: 0,
            pool_reward_pot: 0,
            sum_rewards: 0,
            rewarded_pool_count: 0,
            eta_numerator: 0,
            eta_denominator: 0,
            epoch_fees: 0,
            mir_reserves_to_treasury: 0,
            mir_reserves_to_accounts: 0,
            mir_treasury_to_accounts: 0,
        }
    }
}
