// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! BLUE receive-side admission token (PHASE4-N-H S1).
//!
//! [`AdmittedBlock`] is the type-level admission gate: its sole
//! constructor is [`admit_via_block_validity`], which wraps
//! [`crate::block_validity::block_validity`] and returns
//! `Ok(AdmittedBlock)` only when the verdict is
//! [`BlockValidityVerdict::Valid`]. The inner field is private; the
//! tuple-struct constructor is module-private; no public path exists
//! from raw bytes to an `AdmittedBlock` outside this site.
//!
//! Distinct from [`crate::producer::AcceptedBlock`] (producer-side
//! broadcast token). The two tokens are deliberately separate so
//! cross-use is mechanically impossible:
//!   - producer broadcast: takes `AcceptedBlock`, not `AdmittedBlock`
//!   - receive admission: takes `AdmittedBlock`, not `AcceptedBlock`
//!
//! See `docs/planning/receive-side-bridge-invariants.md` ¬P-6.

use ade_core::consensus::era_schedule::EraSchedule;
use ade_core::consensus::ledger_view::LedgerView;
use ade_core::consensus::praos_state::PraosChainDepState;

use crate::block_validity::transition::{block_validity, BlockValidityOutcome};
use crate::block_validity::verdict::{BlockValidityError, BlockValidityVerdict};
use crate::state::LedgerState;

/// The receive-side admission token. ChainDb-write wrappers consume
/// this; no constructor exists outside this module, so a wrapper
/// cannot fabricate an `AdmittedBlock` from raw bytes.
#[derive(Debug, Clone, PartialEq)]
pub struct AdmittedBlock {
    // Private field; the only construction site is
    // `admit_via_block_validity` below.
    bytes: Vec<u8>,
}

impl AdmittedBlock {
    /// Read-only access for ChainDb-write wrappers.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Consume the token, yielding the validated bytes.
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

/// The outcome of receive-side admission: an `AdmittedBlock` token
/// plus the evolved sub-states (ledger + chain_dep) the receive
/// reducer hands back to its caller.
#[derive(Debug, Clone, PartialEq)]
pub struct AdmittedOutcome {
    pub admitted: AdmittedBlock,
    pub ledger: LedgerState,
    pub chain_dep: PraosChainDepState,
}

/// Admit peer-supplied block bytes via `block_validity`. The only
/// public constructor of `AdmittedBlock`.
///
/// On `Valid`: returns `Ok(AdmittedOutcome)` carrying the
/// `AdmittedBlock` token + the evolved ledger + the evolved
/// chain_dep state.
///
/// On `Invalid`: returns `Err(BlockValidityError)` with the upstream
/// rejection verbatim; the caller's states are unchanged (the input
/// states were borrowed; no token is produced).
pub fn admit_via_block_validity(
    block_bytes: &[u8],
    ledger: &LedgerState,
    chain_dep: &PraosChainDepState,
    era_schedule: &EraSchedule,
    ledger_view: &dyn LedgerView,
) -> Result<AdmittedOutcome, BlockValidityError> {
    let BlockValidityOutcome {
        verdict,
        ledger: new_ledger,
        chain_dep: new_chain_dep,
    } = block_validity(ledger, chain_dep, era_schedule, ledger_view, block_bytes);
    match verdict {
        BlockValidityVerdict::Valid { .. } => Ok(AdmittedOutcome {
            admitted: AdmittedBlock {
                bytes: block_bytes.to_vec(),
            },
            ledger: new_ledger,
            chain_dep: new_chain_dep,
        }),
        BlockValidityVerdict::Invalid { error, .. } => Err(error),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    use std::collections::BTreeMap;

    use ade_codec::cbor::envelope::decode_block_envelope;
    use ade_core::consensus::vrf_cert::ActiveSlotsCoeff;
    use ade_core::consensus::{BootstrapAnchorHash, EraSummary, Nonce};
    use ade_testkit::validity::ConwayValidityCorpus;
    use ade_types::{CardanoEra, EpochNo, Hash28, Hash32, SlotNo};

    use crate::block_validity::decode_block;
    use crate::consensus_view::{PoolDistrView, PoolEntry};

    const EPOCH_576: EpochNo = EpochNo(576);
    const EPOCH_577_START: u64 = 163_900_800;
    const MAINNET_EPOCH_LENGTH: u64 = 432_000;

    fn schedule() -> EraSchedule {
        let start_576 = EPOCH_577_START - MAINNET_EPOCH_LENGTH;
        EraSchedule::new(
            BootstrapAnchorHash(Hash32([0u8; 32])),
            0,
            vec![EraSummary {
                era: CardanoEra::Conway,
                start_slot: SlotNo(start_576),
                start_epoch: EPOCH_576,
                slot_length_ms: 1_000,
                epoch_length_slots: MAINNET_EPOCH_LENGTH as u32,
                safe_zone_slots: MAINNET_EPOCH_LENGTH as u32,
            }],
        )
        .expect("schedule")
    }

    fn corpus_view() -> (ConwayValidityCorpus, PoolDistrView) {
        let c = ConwayValidityCorpus::load().expect("corpus loads");
        let total = c.pd_total_active_stake;
        let asc = ActiveSlotsCoeff {
            numer: c.asc.numer as u32,
            denom: c.asc.denom as u32,
        };
        let mut pools: BTreeMap<Hash28, PoolEntry> = BTreeMap::new();
        for (pool_id, p) in &c.pools {
            let scale = total / p.sigma.denom;
            pools.insert(
                Hash28(*pool_id),
                PoolEntry {
                    active_stake: p.sigma.numer * scale,
                    vrf_keyhash: Hash32(p.vrf_keyhash),
                },
            );
        }
        (c, PoolDistrView::new(EPOCH_576, total, asc, pools))
    }

    fn ledger_at_576() -> LedgerState {
        let mut l = LedgerState::new(CardanoEra::Conway);
        l.epoch_state.epoch = EPOCH_576;
        l
    }

    fn chain_dep_with_eta0(eta0: [u8; 32]) -> PraosChainDepState {
        let mut s = PraosChainDepState::empty();
        s.epoch_nonce = Nonce(Hash32(eta0));
        s.evolving_nonce = Nonce(Hash32(eta0));
        s
    }

    fn pick_lightest(c: &ConwayValidityCorpus) -> &[u8] {
        let idx = (0..c.blocks.len())
            .min_by_key(|&i| {
                let env = decode_block_envelope(&c.blocks[i]).expect("env");
                env.block_end - env.block_start
            })
            .expect("non-empty");
        &c.blocks[idx]
    }

    fn pick_heaviest(c: &ConwayValidityCorpus) -> &[u8] {
        let idx = (0..c.blocks.len())
            .max_by_key(|&i| {
                let env = decode_block_envelope(&c.blocks[i]).expect("env");
                env.block_end - env.block_start
            })
            .expect("non-empty");
        &c.blocks[idx]
    }

    fn flip_body_byte(env_bytes: &[u8]) -> Vec<u8> {
        let env = decode_block_envelope(env_bytes).expect("env");
        let start = env.block_start;
        let end = env.block_end;
        let base = decode_block(env_bytes).expect("base decodes");
        for idx in (start..end).rev() {
            let mut bad = env_bytes.to_vec();
            bad[idx] ^= 0x01;
            if let Ok(d) = decode_block(&bad) {
                if d.computed_body_hash != base.computed_body_hash {
                    return bad;
                }
            }
        }
        panic!("no structure-preserving body flip found")
    }

    #[test]
    fn admit_via_block_validity_accepts_corpus_block() {
        let (c, view) = corpus_view();
        let schedule = schedule();
        let ledger = ledger_at_576();
        let chain_dep = chain_dep_with_eta0(c.epoch_nonce);
        let bytes = pick_lightest(&c).to_vec();
        let outcome = admit_via_block_validity(&bytes, &ledger, &chain_dep, &schedule, &view)
            .expect("validator-accepted block must admit");
        assert_eq!(outcome.admitted.as_bytes(), &bytes[..]);
    }

    #[test]
    fn admit_via_block_validity_rejects_corrupted_body() {
        let (c, view) = corpus_view();
        let schedule = schedule();
        let ledger = ledger_at_576();
        let chain_dep = chain_dep_with_eta0(c.epoch_nonce);
        let bytes = pick_heaviest(&c);
        let altered = flip_body_byte(bytes);
        let err = admit_via_block_validity(&altered, &ledger, &chain_dep, &schedule, &view)
            .expect_err("corrupted body must reject");
        match err {
            BlockValidityError::BodyHashMismatch { .. } => {}
            other => panic!("expected BodyHashMismatch, got {other:?}"),
        }
    }

    #[test]
    fn admitted_block_as_bytes_is_subslice_of_input() {
        // The token's as_bytes() returns a slice of the token's own
        // owned Vec — not literally the input slice (we copy in
        // `admit_via_block_validity`). Verify the bytes are byte-
        // identical to the input and the slice is contained within
        // the token's internal vec.
        let (c, view) = corpus_view();
        let schedule = schedule();
        let ledger = ledger_at_576();
        let chain_dep = chain_dep_with_eta0(c.epoch_nonce);
        let bytes = pick_lightest(&c).to_vec();
        let outcome = admit_via_block_validity(&bytes, &ledger, &chain_dep, &schedule, &view)
            .expect("admit");
        let got = outcome.admitted.as_bytes();
        assert_eq!(got, &bytes[..]);
        let owned = outcome.admitted.into_bytes();
        assert_eq!(owned, bytes);
    }
}
