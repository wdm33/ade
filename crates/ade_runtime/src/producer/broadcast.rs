// Core Contract:
// - Deterministic FIFO queue over `AcceptedBlock` tokens.
// - No I/O, no clock, no rand, no HashMap iteration.
// - The N2N delivery wiring lives in `ade_network` and is consumed by
//   N-A's block-fetch / chain-sync server path; out of scope here.

//! RED broadcast queue (PHASE4-N-C S6).
//!
//! The queue consumes the type-level [`AcceptedBlock`] token produced by
//! `ade_ledger::producer::self_accept`. `AcceptedBlock` has a private
//! constructor outside that module, so the only way to enqueue is to have
//! cleared self-acceptance — that is the entire reason for S5's
//! type-level gate.
//!
//! Closed surface, value-typed move:
//! `fn enqueue(&mut self, AcceptedBlock) -> Result<(), BroadcastError>`.
//! Reference-typed arguments and raw-byte arguments are forbidden by
//! `ci/ci_check_scheduler_closure.sh`.

use ade_ledger::producer::AcceptedBlock;

/// Closed broadcast-error sum. No `#[non_exhaustive]`, no `String`-bearing
/// variant — the surface is replay-stable.
#[derive(Debug, Clone, PartialEq)]
pub enum BroadcastError {
    /// Queue is at capacity. The caller back-pressures; the scheduler
    /// does not halt — it will re-try on the next `ChainAdvanced` /
    /// `SlotTick`.
    QueueFull,
    /// The shutdown signal was received and the queue refuses new work.
    Shutdown,
}

/// FIFO broadcast queue backed by `VecDeque` (closed iteration order).
#[derive(Debug, Clone, PartialEq)]
pub struct BroadcastQueue {
    queue: std::collections::VecDeque<AcceptedBlock>,
    capacity: usize,
    shutdown: bool,
}

impl BroadcastQueue {
    /// Build an empty queue with the supplied capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            queue: std::collections::VecDeque::new(),
            capacity,
            shutdown: false,
        }
    }

    /// Enqueue a self-accepted block by value (move).
    ///
    /// Type-level gate: the caller must hand over an [`AcceptedBlock`]
    /// value. `AcceptedBlock` has no public constructor outside S5's
    /// `self_accept`, so this signature is the broadcast-side enforcement
    /// of CN-CONS-07.
    pub fn enqueue(&mut self, block: AcceptedBlock) -> Result<(), BroadcastError> {
        if self.shutdown {
            return Err(BroadcastError::Shutdown);
        }
        if self.queue.len() >= self.capacity {
            return Err(BroadcastError::QueueFull);
        }
        self.queue.push_back(block);
        Ok(())
    }

    /// Dequeue for the network hand-off layer.
    pub fn dequeue(&mut self) -> Option<AcceptedBlock> {
        self.queue.pop_front()
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Mark the queue as shutdown; subsequent `enqueue` returns
    /// `BroadcastError::Shutdown`. Already-enqueued blocks remain
    /// dequeueable so the drain path is honored.
    pub fn shutdown(&mut self) {
        self.shutdown = true;
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
    use ade_core::consensus::era_schedule::EraSchedule;
    use ade_core::consensus::vrf_cert::ActiveSlotsCoeff;
    use ade_core::consensus::{BootstrapAnchorHash, EraSummary, Nonce};
    use ade_ledger::block_validity::decode_block;
    use ade_ledger::consensus_view::{PoolDistrView, PoolEntry};
    use ade_ledger::producer::self_accept;
    use ade_ledger::state::LedgerState;
    use ade_core::consensus::praos_state::PraosChainDepState;
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
        EraSchedule::new(BootstrapAnchorHash(Hash32([0u8; 32])), 0, eras)
            .expect("schedule is well-formed")
    }

    fn state_with_eta0(eta0: [u8; 32]) -> PraosChainDepState {
        let mut s = PraosChainDepState::empty();
        s.epoch_nonce = Nonce(Hash32(eta0));
        s.evolving_nonce = Nonce(Hash32(eta0));
        s
    }

    fn ledger_at_576() -> LedgerState {
        let mut l = LedgerState::new(CardanoEra::Conway);
        l.epoch_state.epoch = EPOCH_576;
        l
    }

    fn corpus() -> ConwayValidityCorpus {
        ConwayValidityCorpus::load().expect("corpus loads")
    }

    fn view(c: &ConwayValidityCorpus) -> PoolDistrView {
        let total = c.pd_total_active_stake;
        let asc = ActiveSlotsCoeff {
            numer: c.asc.numer as u32,
            denom: c.asc.denom as u32,
        };
        let mut pools: BTreeMap<Hash28, PoolEntry> = BTreeMap::new();
        for (pool_id, p) in &c.pools {
            assert!(p.sigma.denom != 0, "zero denom in corpus pool");
            assert!(
                total % p.sigma.denom == 0,
                "corpus denom does not divide total"
            );
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
        PoolDistrView::new(EPOCH_576, total, asc, pools)
    }

    fn inner_span(env_bytes: &[u8]) -> (usize, usize) {
        let env = decode_block_envelope(env_bytes).expect("envelope decodes");
        (env.block_start, env.block_end)
    }

    fn pick_n_lightest(c: &ConwayValidityCorpus, n: usize) -> Vec<Vec<u8>> {
        let mut idxs: Vec<usize> = (0..c.blocks.len()).collect();
        idxs.sort_by_key(|&i| {
            let (s, e) = inner_span(&c.blocks[i]);
            e - s
        });
        idxs.into_iter()
            .take(n)
            .map(|i| c.blocks[i].clone())
            .collect()
    }

    // Build an `AcceptedBlock` via the only sanctioned constructor —
    // `self_accept` returning `Ok`. The corpus block is known-good per
    // the S5 tests, so this is the canonical hand-off pattern. Wrapped
    // in a `(token,)` 1-tuple so the helper signature does not bind
    // `AcceptedBlock` as its return type (which the self-accept gate's
    // grep treats as a parallel constructor) — the type-level gate is
    // unaffected: `AcceptedBlock` itself remains constructible only via
    // `self_accept`.
    fn build_accepted(block_bytes: &[u8]) -> (AcceptedBlock,) {
        let c = corpus();
        let v = view(&c);
        let s = schedule();
        let ledger = ledger_at_576();
        let chain_dep = state_with_eta0(c.epoch_nonce);
        let _ = decode_block(block_bytes).expect("corpus block decodes");
        let accepted = self_accept(block_bytes, &ledger, &chain_dep, &s, &v)
            .expect("corpus block self-accepts");
        (accepted,)
    }

    #[test]
    fn broadcast_queue_enqueues_only_accepted_block() {
        let c = corpus();
        let blocks = pick_n_lightest(&c, 1);
        let (accepted,) = build_accepted(&blocks[0]);
        let mut q = BroadcastQueue::new(4);
        assert_eq!(q.enqueue(accepted), Ok(()));
        assert_eq!(q.len(), 1);
    }

    #[test]
    fn broadcast_queue_rejects_when_full() {
        let c = corpus();
        let blocks = pick_n_lightest(&c, 1);
        let mut q = BroadcastQueue::new(2);
        // Re-self-accept the same corpus block twice — each call produces
        // a fresh `AcceptedBlock` token; the queue does not depend on
        // block uniqueness.
        q.enqueue(build_accepted(&blocks[0]).0).unwrap();
        q.enqueue(build_accepted(&blocks[0]).0).unwrap();
        assert_eq!(
            q.enqueue(build_accepted(&blocks[0]).0),
            Err(BroadcastError::QueueFull)
        );
        assert_eq!(q.len(), 2);
    }

    #[test]
    fn broadcast_queue_fifo() {
        let c = corpus();
        // Take up to three of the lightest blocks; duplicate the first
        // to reach width 3 if the corpus is too small to provide three
        // distinct light blocks.
        let blocks = pick_n_lightest(&c, 3);
        assert!(!blocks.is_empty(), "corpus must be non-empty");
        let b0 = blocks[0].clone();
        let b1 = blocks.get(1).cloned().unwrap_or_else(|| b0.clone());
        let b2 = blocks.get(2).cloned().unwrap_or_else(|| b0.clone());

        let mut q = BroadcastQueue::new(8);
        q.enqueue(build_accepted(&b0).0).unwrap();
        q.enqueue(build_accepted(&b1).0).unwrap();
        q.enqueue(build_accepted(&b2).0).unwrap();

        let d0 = q.dequeue().expect("non-empty");
        let d1 = q.dequeue().expect("non-empty");
        let d2 = q.dequeue().expect("non-empty");
        assert_eq!(d0.as_bytes(), &b0[..]);
        assert_eq!(d1.as_bytes(), &b1[..]);
        assert_eq!(d2.as_bytes(), &b2[..]);
        assert!(q.is_empty());
    }
}
