// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN ChainDb-write adapter (PHASE4-N-H S3).
//!
//! Wraps a borrowed [`ChainDb`] and exposes the BLUE
//! `ChainDbWrite::write_admitted` interface. Decodes the AdmittedBlock
//! bytes once via `ade_ledger::block_validity::decode_block` to
//! extract `(slot, hash)` for the `StoredBlock` key, then calls
//! `ChainDb::put_block`. Maps `ChainDbError` into the BLUE
//! `ChainWriteError` shape.

use ade_ledger::block_validity::decode_block;
use ade_ledger::receive::{AdmittedBlock, ChainDbWrite, ChainWriteError, ChainWriteErrorKind};

use crate::chaindb::{ChainDb, ChainDbError, StoredBlock};

/// GREEN adapter wiring `ChainDbWrite` over any `ChainDb` impl.
pub struct ChainDbWriter<'a, D: ChainDb> {
    pub db: &'a D,
}

impl<'a, D: ChainDb> ChainDbWriter<'a, D> {
    pub fn new(db: &'a D) -> Self {
        Self { db }
    }
}

impl<'a, D: ChainDb> ChainDbWrite for ChainDbWriter<'a, D> {
    fn write_admitted(&mut self, block: AdmittedBlock) -> Result<(), ChainWriteError> {
        let bytes = block.into_bytes();
        // Decode is reachable safely: AdmittedBlock's invariant is that
        // its bytes were already validated by block_validity (which
        // includes decode_block as step 1). A decode failure here
        // would mean the AdmittedBlock invariant was violated.
        let decoded = decode_block(&bytes)
            .map_err(|_| ChainWriteError::Underlying(ChainWriteErrorKind::Other))?;
        let stored = StoredBlock {
            slot: decoded.header_input.slot,
            hash: decoded.block_hash,
            bytes,
        };
        self.db
            .put_block(&stored)
            .map_err(|e| map_chaindb_err(e, stored.slot, stored.hash))
    }
}

fn map_chaindb_err(
    e: ChainDbError,
    slot: ade_types::SlotNo,
    hash: ade_types::Hash32,
) -> ChainWriteError {
    match e {
        ChainDbError::InvalidOperation(_) => ChainWriteError::SlotConflict { slot, hash },
        ChainDbError::Io(_) => ChainWriteError::Underlying(ChainWriteErrorKind::Io),
        _ => ChainWriteError::Underlying(ChainWriteErrorKind::Other),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use crate::chaindb::InMemoryChainDb;

    use std::collections::BTreeMap;

    use ade_codec::cbor::envelope::decode_block_envelope;
    use ade_core::consensus::era_schedule::EraSchedule;
    use ade_core::consensus::praos_state::PraosChainDepState;
    use ade_core::consensus::vrf_cert::ActiveSlotsCoeff;
    use ade_core::consensus::{BootstrapAnchorHash, EraSummary, Nonce};
    use ade_ledger::consensus_view::{PoolDistrView, PoolEntry};
    use ade_ledger::receive::admit_via_block_validity;
    use ade_ledger::state::LedgerState;
    use ade_testkit::validity::ConwayValidityCorpus;
    use ade_types::{CardanoEra, EpochNo, Hash28, Hash32, SlotNo};

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
        let c = ConwayValidityCorpus::load().expect("corpus");
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

    fn fresh_ledger() -> LedgerState {
        let mut l = LedgerState::new(CardanoEra::Conway);
        l.epoch_state.epoch = EPOCH_576;
        l
    }

    fn fresh_chain_dep(eta0: [u8; 32]) -> PraosChainDepState {
        let mut s = PraosChainDepState::empty();
        s.epoch_nonce = Nonce(Hash32(eta0));
        s.evolving_nonce = Nonce(Hash32(eta0));
        s
    }

    fn pick_lightest(c: &ConwayValidityCorpus) -> Vec<u8> {
        let idx = (0..c.blocks.len())
            .min_by_key(|&i| {
                let env = decode_block_envelope(&c.blocks[i]).expect("env");
                env.block_end - env.block_start
            })
            .expect("non-empty");
        c.blocks[idx].clone()
    }

    #[test]
    fn in_memory_chain_write_admits_via_admitted_block_to_chaindb() {
        let (c, view) = corpus_view();
        let schedule = schedule();
        let ledger = fresh_ledger();
        let chain_dep = fresh_chain_dep(c.epoch_nonce);
        let bytes = pick_lightest(&c);
        let outcome = admit_via_block_validity(&bytes, &ledger, &chain_dep, &schedule, &view)
            .expect("admit");
        let db = InMemoryChainDb::new();
        {
            let mut writer = ChainDbWriter::new(&db);
            writer.write_admitted(outcome.admitted).expect("write");
        }
        // The block must be present in the ChainDb at its (slot, hash).
        let tip = db.tip().expect("tip").expect("non-empty");
        let decoded = decode_block(&bytes).expect("decode");
        assert_eq!(tip.slot, decoded.header_input.slot);
        assert_eq!(tip.hash, decoded.block_hash);
        let got = db.get_block_by_hash(&tip.hash).expect("get").expect("present");
        assert_eq!(got.bytes, bytes, "stored bytes must equal corpus bytes");
    }

    #[test]
    fn in_memory_chain_write_recovers_slot_hash_from_bytes() {
        // Same as above; the slot/hash key derivation is the
        // adapter's contract. This separate test pins the derivation
        // step explicitly.
        let (c, view) = corpus_view();
        let schedule = schedule();
        let ledger = fresh_ledger();
        let chain_dep = fresh_chain_dep(c.epoch_nonce);
        let bytes = pick_lightest(&c);
        let outcome = admit_via_block_validity(&bytes, &ledger, &chain_dep, &schedule, &view)
            .expect("admit");
        let decoded = decode_block(&bytes).expect("decode");
        let db = InMemoryChainDb::new();
        {
            let mut writer = ChainDbWriter::new(&db);
            writer.write_admitted(outcome.admitted).expect("write");
        }
        assert!(
            db.get_block_by_slot(decoded.header_input.slot)
                .expect("get")
                .is_some(),
            "block must be findable by slot"
        );
    }
}
