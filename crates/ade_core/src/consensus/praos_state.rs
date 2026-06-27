// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use std::collections::BTreeMap;

use ade_types::{BlockNo, EpochNo, Hash28, Hash32, SlotNo};

use crate::consensus::errors::OpCertCounterError;

/// A 32-byte Praos nonce. Distinct newtype so the type system stops
/// callers from mixing nonces and other Hash32 values.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Nonce(pub Hash32);

impl Nonce {
    pub const ZERO: Nonce = Nonce(Hash32([0u8; 32]));

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0 .0
    }
}

/// (pool_id, kes_period) -> highest observed op-cert issue counter.
///
/// BTreeMap — never HashMap. Insertion / iteration order is
/// deterministic because consumers must replay the same state from
/// the same sequence of headers.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OpCertCounterMap {
    counters: BTreeMap<(Hash28, u64), u64>,
}

impl OpCertCounterMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, pool: &Hash28, kes_period: u64) -> Option<u64> {
        self.counters.get(&(pool.clone(), kes_period)).copied()
    }

    /// Insert `(pool, kes_period, counter)`. Monotonically non-
    /// decreasing — an attempt to insert a counter STRICTLY LESS
    /// than the existing counter returns
    /// `OpCertCounterError::Regression`. Equal counter is OK and
    /// no-op (the same op-cert is re-used across blocks within a
    /// KES period — normal pool operation per the Cardano
    /// protocol).
    ///
    /// PHASE4-N-M-FOLLOW (2026-05-27): the previous semantics
    /// (strict-`<` accept, `<=` reject) over-rejected legitimate
    /// repeated op-certs. The pre-FOLLOW positive Conway corpus
    /// covered 14 different pools each appearing once, so the
    /// bug was masked; sustained live admission against a real
    /// peer surfaced it on the third block (same pool, same KES
    /// period, same counter).
    pub fn upsert_strict(
        &mut self,
        pool: Hash28,
        kes_period: u64,
        counter: u64,
    ) -> Result<(), OpCertCounterError> {
        if let Some(existing) = self.counters.get(&(pool.clone(), kes_period)).copied() {
            if counter < existing {
                return Err(OpCertCounterError::Regression {
                    existing,
                    attempted: counter,
                });
            }
            // counter == existing → same op-cert re-used; no-op.
            // counter > existing → new op-cert rotation; update below.
            if counter == existing {
                return Ok(());
            }
        }
        self.counters.insert((pool, kes_period), counter);
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.counters.len()
    }

    pub fn is_empty(&self) -> bool {
        self.counters.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&(Hash28, u64), &u64)> {
        self.counters.iter()
    }

    /// Insert without checking — for decode paths reconstructing from
    /// a canonical encoding. Decoded entries are assumed already
    /// monotonic by virtue of being a previously-validated state.
    pub(crate) fn insert_unchecked(&mut self, pool: Hash28, kes_period: u64, counter: u64) {
        self.counters.insert((pool, kes_period), counter);
    }
}

/// The complete Praos chain-dep state owned by N-B consensus.
///
/// Five named nonce slots per Ouroboros-consensus PraosChainDepState:
/// evolving / candidate / epoch / previous_epoch / lab.
///
/// `last_epoch_block` tracks the block at the previous epoch boundary
/// (used for nonce candidate-to-epoch promotion).
///
/// `last_slot` tracks the most recent applied header slot.
/// `last_block_no` tracks the corresponding block number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PraosChainDepState {
    pub evolving_nonce: Nonce,
    pub candidate_nonce: Nonce,
    pub epoch_nonce: Nonce,
    pub previous_epoch_nonce: Nonce,
    pub lab_nonce: Nonce,
    pub last_epoch_block: Option<EpochNo>,
    /// The Praos boundary-combine operand: the `lab` nonce of the last
    /// block of the PREVIOUS epoch (`praosStateLastEpochBlockNonce`).
    /// `None` = explicit *unset* — a legacy `array(9)` store or a
    /// pre-seed state. The rolling epoch-boundary combine fails closed
    /// (`MissingLastEpochBlockNonce`) on `None` rather than fabricate a
    /// nonce (DC-EPOCH-16).
    pub last_epoch_block_nonce: Option<Nonce>,
    pub last_slot: Option<SlotNo>,
    pub last_block_no: Option<BlockNo>,
    pub op_cert_counters: OpCertCounterMap,
}

impl PraosChainDepState {
    /// Genesis state: all nonces are the shelley_genesis_hash
    /// (the well-known initial nonce derived from the Shelley
    /// genesis CBOR). Caller supplies it because computing it is
    /// genesis-parser business, not BLUE business.
    pub fn genesis(initial_nonce: Nonce) -> Self {
        Self {
            evolving_nonce: initial_nonce.clone(),
            candidate_nonce: initial_nonce.clone(),
            epoch_nonce: initial_nonce.clone(),
            previous_epoch_nonce: initial_nonce.clone(),
            lab_nonce: initial_nonce,
            last_epoch_block: None,
            last_epoch_block_nonce: None,
            last_slot: None,
            last_block_no: None,
            op_cert_counters: OpCertCounterMap::new(),
        }
    }

    /// PHASE4-N-AN (T-REC-06 / T-REC-04): overlay the recovered seed-epoch eta0
    /// onto the epoch + evolving nonce. The SINGLE eta0-overlay authority used by
    /// BOTH WarmStart bootstrap (the live-admit chain_dep) and rollback
    /// materialization (the replay chain_dep) — so live admit and rollback replay
    /// validate the header VRF against the SAME nonce (replay-equivalence by
    /// construction). eta0 is the recovered canonical input (the seed-epoch
    /// consensus sidecar); the persisted snapshot carries the `Nonce::ZERO`
    /// placeholder, which this overlays. At the seed epoch (no blocks applied
    /// since the seed) the evolving nonce equals eta0, so both are set —
    /// reconstructing `genesis(eta0)`'s nonce basis.
    pub fn overlay_recovered_eta0(&mut self, eta0: &Nonce) {
        // Only supply the recovered eta0 when the snapshot carries the ZERO placeholder — a SEED /
        // cold-start snapshot whose nonce was not persisted. A snapshot captured PAST the seed epoch
        // (a node that CROSSED boundaries before restarting) already persists its real epoch nonce
        // [eta0(N)] and evolving nonce; stamping the seed eta0 over it would CLOBBER the correct nonce
        // and fail the first post-boundary header VRF. NO-OP once a real nonce is present (idempotent).
        if self.epoch_nonce != Nonce::ZERO {
            return;
        }
        self.epoch_nonce = eta0.clone();
        self.evolving_nonce = eta0.clone();
    }

    /// Empty state (all nonces = ZERO, no counters). Used for tests
    /// and for the type-default. NOT a valid runtime state.
    pub fn empty() -> Self {
        Self {
            evolving_nonce: Nonce::ZERO,
            candidate_nonce: Nonce::ZERO,
            epoch_nonce: Nonce::ZERO,
            previous_epoch_nonce: Nonce::ZERO,
            lab_nonce: Nonce::ZERO,
            last_epoch_block: None,
            last_epoch_block_nonce: None,
            last_slot: None,
            last_block_no: None,
            op_cert_counters: OpCertCounterMap::new(),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    fn pool(byte: u8) -> Hash28 {
        Hash28([byte; 28])
    }

    #[test]
    fn op_cert_upsert_rejects_regression() {
        let mut map = OpCertCounterMap::new();
        map.upsert_strict(pool(1), 10, 5).unwrap();
        let err = map.upsert_strict(pool(1), 10, 3);
        assert_eq!(
            err,
            Err(OpCertCounterError::Regression {
                existing: 5,
                attempted: 3,
            })
        );
    }

    #[test]
    fn op_cert_upsert_accepts_equal_counter_as_noop() {
        // PHASE4-N-M-FOLLOW: the same op-cert is legitimately
        // re-used across multiple blocks within a KES period
        // (normal pool operation per the Cardano protocol).
        let mut map = OpCertCounterMap::new();
        map.upsert_strict(pool(2), 7, 4).unwrap();
        assert!(map.upsert_strict(pool(2), 7, 4).is_ok());
        // Counter unchanged after the no-op upsert.
        assert_eq!(map.get(&pool(2), 7), Some(4));
    }

    #[test]
    fn op_cert_upsert_accepts_monotonic_increasing() {
        let mut map = OpCertCounterMap::new();
        assert!(map.upsert_strict(pool(3), 1, 1).is_ok());
        assert!(map.upsert_strict(pool(3), 1, 2).is_ok());
        // Equal counter is OK (re-used op-cert).
        assert!(map.upsert_strict(pool(3), 1, 2).is_ok());
        assert!(map.upsert_strict(pool(3), 1, 100).is_ok());
        assert_eq!(map.get(&pool(3), 1), Some(100));
    }

    #[test]
    fn genesis_state_is_well_formed() {
        let nonce = Nonce(Hash32([0xaa; 32]));
        let s = PraosChainDepState::genesis(nonce.clone());
        assert_eq!(s.evolving_nonce, nonce);
        assert_eq!(s.candidate_nonce, nonce);
        assert_eq!(s.epoch_nonce, nonce);
        assert_eq!(s.previous_epoch_nonce, nonce);
        assert_eq!(s.lab_nonce, nonce);
        assert_eq!(s.last_epoch_block, None);
        assert_eq!(s.last_epoch_block_nonce, None);
        assert_eq!(s.last_slot, None);
        assert_eq!(s.last_block_no, None);
        assert!(s.op_cert_counters.is_empty());
    }

    #[test]
    fn nonce_zero_constant_is_zero_bytes() {
        assert_eq!(Nonce::ZERO.as_bytes(), &[0u8; 32]);
    }
}
