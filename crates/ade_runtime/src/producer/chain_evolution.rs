// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN linear `ChainEvolution` typestate for live producer-mode
//! (PHASE4-N-T S2).
//!
//! Threads the producer's chain state forward across forges. It is
//! **GREEN by content** inside the RED `ade_runtime` crate — pure,
//! deterministic, no I/O, no `tokio`, no wall clock, no RNG, no
//! `HashMap`/`HashSet`, no floats (`BTreeMap` is allowed via the held
//! `PoolDistrView`).
//!
//! `advance` obtains BOTH the post-state (from BLUE `block_validity`)
//! AND the `AcceptedBlock` broadcast token (from BLUE `self_accept`)
//! against IDENTICAL inputs, then cross-checks the two verdicts via
//! `reconcile_verdicts`. **`ChainEvolution` never constructs an
//! `AcceptedBlock`** — the token has a private constructor and is
//! obtained solely from `self_accept` (CE-T-7). This file contains no
//! `AcceptedBlock` struct-literal constructor.
//!
//! See `docs/clusters/PHASE4-N-T/cluster.md` §1.5 for the doctrine on
//! where the post-state and the token come from, and §6 for the hard
//! prohibitions.

use ade_core::consensus::era_schedule::EraSchedule;
use ade_core::consensus::praos_state::{Nonce, PraosChainDepState};
use ade_ledger::block_validity::decode_block;
use ade_ledger::block_validity::transition::{block_validity, BlockValidityOutcome};
use ade_ledger::block_validity::verdict::BlockValidityVerdict;
use ade_ledger::consensus_view::PoolDistrView;
use ade_ledger::producer::self_accept::{self_accept, AcceptedBlock, SelfAcceptError};
use ade_ledger::state::LedgerState;
use ade_types::Hash32;

use crate::producer::coordinator::ChainTip;

/// The closed `advance` failure surface.
#[derive(Debug, Clone, PartialEq)]
pub enum ChainEvolutionError {
    /// `self_accept` rejected the forged bytes (the validator said
    /// Invalid). Carries the underlying self-accept error verbatim.
    SelfAcceptRejected(SelfAcceptError),
    /// `block_validity` (Valid?) and `self_accept` (Ok?) disagreed on
    /// identical inputs — defense-in-depth against an arg-mismatch bug
    /// in `advance`. Structurally unreachable when `advance` passes
    /// identical args (because `self_accept` calls `block_validity`
    /// internally); the guard makes that contract explicit. Mirrors the
    /// posture of `served_chain_handle::PushError::SenderUnavailable`.
    AuthorityMismatch,
}

/// A pure, linear typestate threading the producer's chain forward.
///
/// Holds fixed-for-the-run `{era_schedule, pool_distr_view, eta0}` and
/// the evolving `{base_ledger, base_chain_dep, tip}`. The
/// `pool_distr_view` is held by value and used as `&dyn LedgerView`
/// (it `impl LedgerView`) — no held trait object, keeping the type a
/// pure GREEN value (OI-T.2).
#[derive(Debug, Clone, PartialEq)]
pub struct ChainEvolution {
    // fixed for the run (single-epoch cold-start scope)
    era_schedule: EraSchedule,
    pool_distr_view: PoolDistrView,
    eta0: Nonce,
    // evolving
    base_ledger: LedgerState,
    base_chain_dep: PraosChainDepState,
    tip: Option<ChainTip>,
}

impl ChainEvolution {
    /// Seed the typestate from the bootstrap triple plus the
    /// fixed-for-the-run context.
    pub fn seed(
        base_ledger: LedgerState,
        base_chain_dep: PraosChainDepState,
        tip: Option<ChainTip>,
        era_schedule: EraSchedule,
        pool_distr_view: PoolDistrView,
        eta0: Nonce,
    ) -> Self {
        ChainEvolution {
            era_schedule,
            pool_distr_view,
            eta0,
            base_ledger,
            base_chain_dep,
            tip,
        }
    }

    pub fn base_ledger(&self) -> &LedgerState {
        &self.base_ledger
    }

    pub fn base_chain_dep(&self) -> &PraosChainDepState {
        &self.base_chain_dep
    }

    pub fn era_schedule(&self) -> &EraSchedule {
        &self.era_schedule
    }

    pub fn pool_distr_view(&self) -> &PoolDistrView {
        &self.pool_distr_view
    }

    pub fn eta0(&self) -> &Nonce {
        &self.eta0
    }

    pub fn tip(&self) -> Option<&ChainTip> {
        self.tip.as_ref()
    }

    /// Block number the next forged block will carry: `tip.block_number
    /// + 1`, or `0` at cold-start (`tip == None`).
    pub fn next_block_number(&self) -> u64 {
        match &self.tip {
            Some(t) => t.block_number + 1,
            None => 0,
        }
    }

    /// Previous block hash to thread into the next header: the current
    /// `tip.block_hash`, or the all-zero hash at cold-start.
    pub fn prev_hash(&self) -> Hash32 {
        match &self.tip {
            Some(t) => Hash32(t.block_hash),
            None => Hash32([0u8; 32]),
        }
    }

    /// Consume the typestate and advance it by one forged block.
    ///
    /// Re-derives the post-state via BLUE `block_validity` and obtains
    /// the `AcceptedBlock` token via BLUE `self_accept`, against
    /// IDENTICAL inputs (same base ledger, base chain_dep, era_schedule,
    /// ledger_view, forged_bytes). The two verdicts are cross-checked by
    /// `reconcile_verdicts`. Consumes `self` (linear typestate — a stale
    /// base cannot be reused).
    pub fn advance(
        self,
        forged_bytes: &[u8],
    ) -> Result<(ChainEvolution, AcceptedBlock), ChainEvolutionError> {
        let outcome: BlockValidityOutcome = block_validity(
            &self.base_ledger,
            &self.base_chain_dep,
            &self.era_schedule,
            &self.pool_distr_view,
            forged_bytes,
        );
        let bv_valid = matches!(outcome.verdict, BlockValidityVerdict::Valid { .. });

        let sa = self_accept(
            forged_bytes,
            &self.base_ledger,
            &self.base_chain_dep,
            &self.era_schedule,
            &self.pool_distr_view,
        );

        reconcile_verdicts(bv_valid, sa.is_ok())?;
        let token = sa.map_err(ChainEvolutionError::SelfAcceptRejected)?;

        // `bv_valid` is true here (the guard agreed with `self_accept`),
        // so `outcome.ledger`/`outcome.chain_dep` are the post-state.
        // `self_accept` already decoded these bytes successfully, so the
        // re-decode is infallible on this path; fail closed on the
        // structurally-unreachable error rather than panicking.
        let decoded = decode_block(forged_bytes)
            .map_err(|_| ChainEvolutionError::AuthorityMismatch)?;
        let new_tip = ChainTip {
            slot: decoded.header_input.slot.0,
            block_hash: decoded.block_hash.0,
            block_number: self.next_block_number(),
        };

        let next = ChainEvolution {
            era_schedule: self.era_schedule,
            pool_distr_view: self.pool_distr_view,
            eta0: self.eta0,
            base_ledger: outcome.ledger,
            base_chain_dep: outcome.chain_dep,
            tip: Some(new_tip),
        };
        Ok((next, token))
    }
}

/// Cross-check the two BLUE authorities: returns `Err(AuthorityMismatch)`
/// iff `block_validity`'s Valid verdict disagrees with `self_accept`'s
/// Ok verdict, else `Ok(())`. The mechanical home of CE-T-6b.
pub fn reconcile_verdicts(
    block_validity_valid: bool,
    self_accept_ok: bool,
) -> Result<(), ChainEvolutionError> {
    if block_validity_valid != self_accept_ok {
        Err(ChainEvolutionError::AuthorityMismatch)
    } else {
        Ok(())
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
    use ade_core::consensus::{BootstrapAnchorHash, EraSummary};
    use ade_ledger::consensus_view::PoolEntry;
    use ade_ledger::fingerprint::fingerprint;
    use ade_testkit::validity::ConwayValidityCorpus;
    use ade_types::{CardanoEra, EpochNo, Hash28, SlotNo};

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
        .expect("schedule is well-formed")
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
            let scale = total / p.sigma.denom;
            pools.insert(
                Hash28(*pool_id),
                PoolEntry {
                    active_stake: p.sigma.numer * scale,
                    vrf_keyhash: Hash32(p.vrf_keyhash),
                },
            );
        }
        PoolDistrView::new(EPOCH_576, total, asc, pools)
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

    fn inner_span(env_bytes: &[u8]) -> (usize, usize) {
        let env = decode_block_envelope(env_bytes).expect("envelope decodes");
        (env.block_start, env.block_end)
    }

    fn pick_lightest(c: &ConwayValidityCorpus) -> &[u8] {
        let idx = (0..c.blocks.len())
            .min_by_key(|&i| {
                let (s, e) = inner_span(&c.blocks[i]);
                e - s
            })
            .expect("corpus is non-empty");
        &c.blocks[idx]
    }

    fn pick_heaviest(c: &ConwayValidityCorpus) -> &[u8] {
        let idx = (0..c.blocks.len())
            .max_by_key(|&i| {
                let (s, e) = inner_span(&c.blocks[i]);
                e - s
            })
            .expect("corpus is non-empty");
        &c.blocks[idx]
    }

    /// Flip one body byte so the header is untouched but the recomputed
    /// body hash changes — mirrors `self_accept.rs::flip_body_byte`.
    fn flip_body_byte(env_bytes: &[u8]) -> Vec<u8> {
        let (start, end) = inner_span(env_bytes);
        let base = decode_block(env_bytes).expect("base block decodes");
        for idx in (start..end).rev() {
            let mut bad = env_bytes.to_vec();
            bad[idx] ^= 0x01;
            if let Ok(d) = decode_block(&bad) {
                if d.computed_body_hash != base.computed_body_hash {
                    return bad;
                }
            }
        }
        panic!("no structure-preserving body flip found");
    }

    fn seed_from_corpus(c: &ConwayValidityCorpus) -> ChainEvolution {
        ChainEvolution::seed(
            ledger_at_576(),
            chain_dep_with_eta0(c.epoch_nonce),
            None,
            schedule(),
            view(c),
            Nonce(Hash32(c.epoch_nonce)),
        )
    }

    #[test]
    fn advance_threads_post_state_forward() {
        let corpus = corpus();
        let seed = seed_from_corpus(&corpus);
        let seed_chain_dep = seed.base_chain_dep().clone();

        let block = pick_lightest(&corpus).to_vec();
        let (evo2, token) = seed.advance(&block).expect("lightest corpus block advances");

        let tip = evo2.tip().expect("tip set after first advance");
        assert_eq!(tip.block_number, 0, "first forged block carries number 0");
        assert_ne!(
            evo2.base_chain_dep(),
            &seed_chain_dep,
            "nonce must evolve across the advance"
        );
        assert_eq!(
            token.as_bytes(),
            &block[..],
            "AcceptedBlock must round-trip the forged bytes verbatim"
        );
    }

    #[test]
    fn advance_two_runs_byte_identical() {
        let corpus = corpus();
        let block = pick_lightest(&corpus).to_vec();

        let (evo_a, _) = seed_from_corpus(&corpus)
            .advance(&block)
            .expect("run A advances");
        let (evo_b, _) = seed_from_corpus(&corpus)
            .advance(&block)
            .expect("run B advances");

        assert_eq!(
            fingerprint(evo_a.base_ledger()).combined,
            fingerprint(evo_b.base_ledger()).combined,
            "two-run post-ledger fingerprints must be byte-identical"
        );
        assert_eq!(
            evo_a.base_chain_dep(),
            evo_b.base_chain_dep(),
            "two-run post-chain_dep must be byte-identical"
        );
        assert_eq!(evo_a.tip(), evo_b.tip(), "two-run tip must be identical");
    }

    #[test]
    fn advance_rejects_invalid_bytes() {
        let corpus = corpus();
        // Heaviest block: lightweight blocks have empty bodies where
        // every byte is a CBOR length/type marker, so no
        // structure-preserving content flip exists.
        let block = pick_heaviest(&corpus).to_vec();
        let altered = flip_body_byte(&block);

        let seed = seed_from_corpus(&corpus);
        let err = seed
            .advance(&altered)
            .expect_err("flipped body byte must be rejected");
        match err {
            ChainEvolutionError::SelfAcceptRejected(_) => {}
            other => panic!("expected SelfAcceptRejected, got {other:?}"),
        }
        // `seed` was consumed by `advance` — no partial advance is
        // representable (linear typestate).
    }

    #[test]
    fn reconcile_verdicts_both_valid_ok() {
        assert_eq!(reconcile_verdicts(true, true), Ok(()));
    }

    #[test]
    fn reconcile_verdicts_both_invalid_ok() {
        assert_eq!(reconcile_verdicts(false, false), Ok(()));
    }

    #[test]
    fn reconcile_verdicts_valid_vs_reject_mismatches() {
        assert_eq!(
            reconcile_verdicts(true, false),
            Err(ChainEvolutionError::AuthorityMismatch)
        );
    }

    #[test]
    fn reconcile_verdicts_reject_vs_valid_mismatches() {
        assert_eq!(
            reconcile_verdicts(false, true),
            Err(ChainEvolutionError::AuthorityMismatch)
        );
    }
}
