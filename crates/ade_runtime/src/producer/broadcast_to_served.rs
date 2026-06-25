// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// GREEN producer-side adapter (PHASE4-N-G S5).
//
// Bridges PHASE4-N-C's `BroadcastQueue` to PHASE4-N-G S2's
// `ServedChainSnapshot`. Drains the queue and admits each
// `AcceptedBlock` into the served chain via `served_chain_admit`.
// Pure, no I/O, no clock — observably deterministic over captured
// arrival sequences.
//
// The orchestrator (S6 RED) drives this between slot ticks; the
// reducers (S3 chain-sync, S4 block-fetch) read from the resulting
// snapshot through the `ServedHeaderLookup` / `ServedRangeLookup`
// trait impls in `served_chain_lookups`.

use ade_ledger::producer::{served_chain_admit, ServedChainAdmitError, ServedChainSnapshot};
use ade_runtime_broadcast_export::{AcceptedBlock, BroadcastQueue};

mod ade_runtime_broadcast_export {
    pub use ade_ledger::producer::AcceptedBlock;
    pub use crate::producer::broadcast::BroadcastQueue;
}

/// Drain the broadcast queue, admitting every dequeued AcceptedBlock
/// into the served chain. Returns the updated snapshot, the
/// (drained) queue, and the sequence of admitted-block keys in
/// dequeue order — the third value lets S3's `advance_tip` know what
/// became eligible for announcement.
///
/// Total, pure, deterministic. The function does not consume the
/// queue's shutdown flag.
pub fn drain_and_admit(
    snap: ServedChainSnapshot,
    mut queue: BroadcastQueue,
) -> Result<(ServedChainSnapshot, BroadcastQueue, Vec<AcceptedBlock>), ServedChainAdmitError> {
    let mut snap = snap;
    let mut drained: Vec<AcceptedBlock> = Vec::new();
    while let Some(block) = queue.dequeue() {
        // Admit a clone-equivalent into the snapshot; keep the
        // original AcceptedBlock around so `advance_tip` can be
        // driven without re-decoding. AcceptedBlock derives Clone.
        let owned = block.clone();
        snap = served_chain_admit(snap, owned)?;
        drained.push(block);
    }
    Ok((snap, queue, drained))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use crate::producer::broadcast::BroadcastQueue;

    use std::collections::BTreeMap;

    use ade_codec::cbor::envelope::decode_block_envelope;
    use ade_core::consensus::era_schedule::EraSchedule;
    use ade_core::consensus::praos_state::PraosChainDepState;
    use ade_core::consensus::vrf_cert::ActiveSlotsCoeff;
    use ade_core::consensus::{BootstrapAnchorHash, EraSummary, Nonce};
    use ade_ledger::consensus_view::{PoolDistrView, PoolEntry};
    use ade_ledger::producer::{self_accept, ServedChainSnapshot};
    use ade_ledger::state::LedgerState;
    use ade_testkit::validity::ConwayValidityCorpus;
    use ade_types::{CardanoEra, EpochNo, Hash28, Hash32, SlotNo};

    const EPOCH_576: EpochNo = EpochNo(576);
    const EPOCH_577_START: u64 = 163_900_800;
    const MAINNET_EPOCH_LENGTH: u64 = 432_000;

    fn schedule() -> EraSchedule {
        let start_576 = EPOCH_577_START - MAINNET_EPOCH_LENGTH;
        let eras = vec![EraSummary {
            randomness_stabilisation_window_slots: None,
            era: CardanoEra::Conway,
            start_slot: SlotNo(start_576),
            start_epoch: EPOCH_576,
            slot_length_ms: 1_000,
            epoch_length_slots: MAINNET_EPOCH_LENGTH as u32,
            safe_zone_slots: MAINNET_EPOCH_LENGTH as u32,
        }];
        EraSchedule::new(BootstrapAnchorHash(Hash32([0u8; 32])), 0, eras).expect("schedule")
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
            let active_stake = p.sigma.numer * scale;
            pools.insert(
                Hash28(*pool_id),
                PoolEntry {
                    active_stake,
                    vrf_keyhash: Hash32(p.vrf_keyhash),
                },
            );
        }
        (c, PoolDistrView::new(EPOCH_576, total, asc, pools))
    }

    fn build_accepted_n(n: usize) -> Vec<AcceptedBlock> {
        let (c, view) = corpus_view();
        let mut idxs: Vec<usize> = (0..c.blocks.len()).collect();
        idxs.sort_by_key(|&i| {
            let env = decode_block_envelope(&c.blocks[i]).expect("env");
            env.block_end - env.block_start
        });
        let block_bytes: Vec<Vec<u8>> = idxs.into_iter().take(n).map(|i| c.blocks[i].clone()).collect();
        let schedule = schedule();
        let ledger = {
            let mut l = LedgerState::new(CardanoEra::Conway);
            l.epoch_state.epoch = EPOCH_576;
            l
        };
        let chain_dep = {
            let mut s = PraosChainDepState::empty();
            s.epoch_nonce = Nonce(Hash32(c.epoch_nonce));
            s.evolving_nonce = Nonce(Hash32(c.epoch_nonce));
            s
        };
        block_bytes
            .iter()
            .map(|b| self_accept(b, &ledger, &chain_dep, &schedule, &view).expect("self_accept"))
            .collect()
    }

    #[test]
    fn drain_and_admit_admits_every_queued_block() {
        let blocks = build_accepted_n(2);
        let mut queue = BroadcastQueue::new(4);
        for b in &blocks {
            queue.enqueue(b.clone()).expect("enqueue");
        }
        let snap = ServedChainSnapshot::new();
        let (snap, queue, drained) = drain_and_admit(snap, queue).expect("drain");
        assert_eq!(snap.len(), blocks.len());
        assert_eq!(drained.len(), blocks.len());
        assert!(queue.is_empty());
    }

    #[test]
    fn drain_and_admit_is_deterministic_over_arrival_sequence() {
        let blocks = build_accepted_n(2);
        let run = || -> Hash32 {
            let mut queue = BroadcastQueue::new(4);
            for b in &blocks {
                queue.enqueue(b.clone()).expect("enqueue");
            }
            let (snap, _q, _d) = drain_and_admit(ServedChainSnapshot::new(), queue).unwrap();
            snap.fingerprint()
        };
        let a = run();
        let b = run();
        assert_eq!(a, b, "identical arrival sequence -> identical fingerprint");
    }

    #[test]
    fn drain_and_admit_no_io_no_clock() {
        // Negative test: confirm the function is pure by running it
        // twice in different shapes and asserting identical
        // observable outputs. Stronger guarantees are the CI gate
        // ci/ci_check_broadcast_to_served_purity.sh that forbids
        // imports of std::time / tokio / rand in this module.
        let blocks = build_accepted_n(1);
        let mut q1 = BroadcastQueue::new(4);
        let mut q2 = BroadcastQueue::new(4);
        for b in &blocks {
            q1.enqueue(b.clone()).unwrap();
            q2.enqueue(b.clone()).unwrap();
        }
        let (s1, _, _) = drain_and_admit(ServedChainSnapshot::new(), q1).unwrap();
        let (s2, _, _) = drain_and_admit(ServedChainSnapshot::new(), q2).unwrap();
        assert_eq!(s1.fingerprint(), s2.fingerprint());
    }
}
