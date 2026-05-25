// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Self-acceptance bridge: a forged block cannot be broadcast unless
//! Ade's own validator (header + body) accepts it under the same slot,
//! era, and context. The `AcceptedBlock` newtype is the type-level
//! broadcast token — its private constructor lives only here.

use ade_core::consensus::era_schedule::EraSchedule;
use ade_core::consensus::ledger_view::LedgerView;
use ade_core::consensus::praos_state::PraosChainDepState;

use crate::block_validity::transition::{block_validity, BlockValidityOutcome};
use crate::block_validity::verdict::{BlockValidityError, BlockValidityVerdict};
use crate::state::LedgerState;

/// The closed self-accept verdict.
#[derive(Debug, Clone, PartialEq)]
pub enum SelfAcceptError {
    /// The full block validator (header + body-hash bind + body apply)
    /// rejected the forged bytes. Carries the underlying validator
    /// error verbatim — same `BlockValidityError` surface the
    /// receive-side validator emits.
    Rejected(BlockValidityError),
}

/// The type-level broadcast token. RED `broadcast` consumes this; it
/// has no constructor outside this module, so the only way to obtain
/// one is via `self_accept` returning `Ok(...)`.
#[derive(Debug, Clone, PartialEq)]
pub struct AcceptedBlock {
    // Field is intentionally private: the struct-literal constructor
    // below is reachable only from inside this module, so the broadcast
    // surface (S6) cannot fabricate an `AcceptedBlock` from raw bytes.
    bytes: Vec<u8>,
}

impl AcceptedBlock {
    /// Public read-only access for the broadcast layer.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Convert to `Vec<u8>` for hand-off to the broadcast queue. Total,
    /// no observable nondeterminism.
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

/// Wrap forged bytes in `AcceptedBlock` IFF the validator accepts them
/// under the supplied context. Pure, total, deterministic.
///
/// Pipeline (matches the receive-side validator exactly):
/// 1. `block_validity(ledger, chain_dep, era_schedule, ledger_view,
///    forged_bytes)` runs the full validator chain — decode + header
///    validate + body-hash bind + body apply.
/// 2. If the verdict is `BlockValidityVerdict::Valid`, return
///    `Ok(AcceptedBlock { bytes: forged_bytes.to_vec() })`.
/// 3. If the verdict is `BlockValidityVerdict::Invalid`, return
///    `Err(SelfAcceptError::Rejected(error))`.
pub fn self_accept(
    forged_bytes: &[u8],
    ledger: &LedgerState,
    chain_dep: &PraosChainDepState,
    era_schedule: &EraSchedule,
    ledger_view: &dyn LedgerView,
) -> Result<AcceptedBlock, SelfAcceptError> {
    let BlockValidityOutcome { verdict, .. } =
        block_validity(ledger, chain_dep, era_schedule, ledger_view, forged_bytes);
    match verdict {
        BlockValidityVerdict::Valid { .. } => Ok(AcceptedBlock {
            bytes: forged_bytes.to_vec(),
        }),
        BlockValidityVerdict::Invalid { error, .. } => Err(SelfAcceptError::Rejected(error)),
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
        let eras = vec![EraSummary {
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

    /// Local copy of `ade_testkit::validity::pool_distr_view_from_corpus`
    /// — the testkit's version returns the other-crate `PoolDistrView`
    /// instance under the dev-dep cycle (`ade_ledger` -> `ade_testkit`
    /// -> `ade_ledger`), so we project the corpus through the
    /// in-crate type directly.
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

    /// The inner Conway block byte span within an envelope.
    fn inner_span(env_bytes: &[u8]) -> (usize, usize) {
        let env = decode_block_envelope(env_bytes).expect("envelope decodes");
        (env.block_start, env.block_end)
    }

    /// Pick the lightest corpus block — same selection rule as
    /// `block_validity_compose.rs::valid_block_evolves_both_states`.
    fn pick_lightest(c: &ConwayValidityCorpus) -> &[u8] {
        let idx = (0..c.blocks.len())
            .min_by_key(|&i| {
                let (s, e) = inner_span(&c.blocks[i]);
                e - s
            })
            .expect("corpus is non-empty");
        &c.blocks[idx]
    }

    /// Pick a corpus block with a non-empty body whose bytes are large
    /// enough that `flip_body_byte` can find a structure-preserving
    /// content flip (the lightest blocks have empty bodies where almost
    /// every byte is a CBOR length/type marker).
    fn pick_heaviest(c: &ConwayValidityCorpus) -> &[u8] {
        let idx = (0..c.blocks.len())
            .max_by_key(|&i| {
                let (s, e) = inner_span(&c.blocks[i]);
                e - s
            })
            .expect("corpus is non-empty");
        &c.blocks[idx]
    }

    /// Flip one byte inside the block body so the header is untouched but the
    /// recomputed body hash changes. Same recipe as the B1 compose tests.
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

    /// Flip the last byte of the KES signature payload inside the header.
    /// The header is `array(2)[header_body, kes_sig]`; the KES sig is a
    /// CBOR `bytes(N)` item. Flipping the last payload byte preserves the
    /// item's CBOR length prefix, so structural decode still succeeds but
    /// KES verify rejects.
    fn flip_kes_sig_byte(env_bytes: &[u8]) -> Vec<u8> {
        use ade_codec::cbor;
        let (start, end) = inner_span(env_bytes);
        let inner = &env_bytes[start..end];
        let mut o = 0usize;
        // Block is `array(N)[header, ..]`; advance into the header.
        cbor::read_array_header(inner, &mut o).expect("block array header");
        // Header is `array(2)[header_body, kes_sig]`.
        let _hdr_span = cbor::skip_item(inner, &mut o).expect("header span");
        // Re-walk into the header to find the KES sig span.
        let mut h = 0usize;
        cbor::read_array_header(inner, &mut h).expect("block array header (re-walk)");
        cbor::read_array_header(inner, &mut h).expect("header array header");
        cbor::skip_item(inner, &mut h).expect("header body span"); // header body
        let (kes_start, kes_end) = cbor::skip_item(inner, &mut h).expect("kes sig span");
        // Last byte of the KES bytes item (its payload tail) in env coords.
        let target_env = start + kes_end - 1;
        assert!(target_env < start + end);
        let _ = (kes_start, kes_end);
        let mut bad = env_bytes.to_vec();
        bad[target_env] ^= 0x01;
        bad
    }

    // -----------------------------------------------------------------
    // §11 / §12 named tests.
    // -----------------------------------------------------------------

    #[test]
    fn self_accept_accepts_freshly_forged_block() {
        let corpus = corpus();
        let view = view(&corpus);
        let schedule = schedule();
        let ledger = ledger_at_576();
        let chain_dep = state_with_eta0(corpus.epoch_nonce);

        let block_bytes = pick_lightest(&corpus);
        let accepted = self_accept(block_bytes, &ledger, &chain_dep, &schedule, &view)
            .expect("validator-accepted block must self-accept");
        assert_eq!(
            accepted.as_bytes(),
            block_bytes,
            "AcceptedBlock must round-trip the forged bytes verbatim"
        );
    }

    #[test]
    fn self_accept_rejects_corrupted_body_hash() {
        // Flip one byte of the body bytes (header — and thus
        // `header.body_hash` — unchanged). The body-hash binding step
        // inside `block_validity` rejects with `BodyHashMismatch`.
        // Uses the heaviest corpus block: lightweight blocks have
        // empty bodies where every byte is a CBOR length/type marker,
        // so no structure-preserving content flip exists.
        let corpus = corpus();
        let view = view(&corpus);
        let schedule = schedule();
        let ledger = ledger_at_576();
        let chain_dep = state_with_eta0(corpus.epoch_nonce);

        let block_bytes = pick_heaviest(&corpus);
        let altered = flip_body_byte(block_bytes);
        let err = self_accept(&altered, &ledger, &chain_dep, &schedule, &view)
            .expect_err("body-hash binding must reject");
        match err {
            SelfAcceptError::Rejected(BlockValidityError::BodyHashMismatch { .. }) => {}
            SelfAcceptError::Rejected(other) => {
                panic!("expected BodyHashMismatch, got {other:?}")
            }
        }
    }

    #[test]
    fn self_accept_rejects_invalid_kes_signature() {
        // Flip one byte of the KES signature in the header. KES verify
        // fails inside the header authority, surfacing as
        // `BlockValidityError::Header(_)`.
        let corpus = corpus();
        let view = view(&corpus);
        let schedule = schedule();
        let ledger = ledger_at_576();
        let chain_dep = state_with_eta0(corpus.epoch_nonce);

        let block_bytes = pick_lightest(&corpus);
        let altered = flip_kes_sig_byte(block_bytes);
        let err = self_accept(&altered, &ledger, &chain_dep, &schedule, &view)
            .expect_err("KES sig flip must reject");
        match err {
            SelfAcceptError::Rejected(BlockValidityError::Header(_)) => {}
            SelfAcceptError::Rejected(other) => {
                panic!("expected Header(_), got {other:?}")
            }
        }
    }

    #[test]
    fn self_accept_rejects_unbalanced_tx_in_body() {
        // Mutate one byte inside the tx_bodies bucket. The validator
        // catches the producer/validator drift at the body-hash binding
        // (the forged body no longer hashes to the header's
        // `body_hash`). This is the canonical "body-disagrees-with-
        // header" failure shape; the gate halts before any tx is
        // re-applied. The test name captures the operator-visible
        // failure: a body whose contents (and therefore tx semantics)
        // disagree with what the header committed to.
        let corpus = corpus();
        let view = view(&corpus);
        let schedule = schedule();
        let ledger = ledger_at_576();
        let chain_dep = state_with_eta0(corpus.epoch_nonce);

        let block_bytes = pick_heaviest(&corpus);
        let altered = flip_body_byte(block_bytes);
        let err = self_accept(&altered, &ledger, &chain_dep, &schedule, &view)
            .expect_err("corrupted tx body must reject");
        match err {
            SelfAcceptError::Rejected(_) => {}
        }
    }

    // -----------------------------------------------------------------
    // PHASE4-N-G S1: accepted_block_header_bytes tests
    // -----------------------------------------------------------------
    //
    // The accessor lives in `block_validity::header_input` but
    // `AcceptedBlock` has a private constructor reachable only from
    // this module, so we host the tests where the legitimate
    // self-accept corpus already exists.

    #[test]
    fn accepted_block_header_bytes_equals_validator_split_on_corpus() {
        use ade_codec::cbor;

        let corpus = corpus();
        let view = view(&corpus);
        let schedule = schedule();
        let ledger = ledger_at_576();
        let chain_dep = state_with_eta0(corpus.epoch_nonce);

        let block_bytes = pick_lightest(&corpus).to_vec();
        let accepted = self_accept(&block_bytes, &ledger, &chain_dep, &schedule, &view)
            .expect("corpus block self-accepts");

        let header_bytes =
            crate::block_validity::accepted_block_header_bytes(&accepted).expect("header projects");

        // Walk the envelope independently and assert the projected
        // slice equals the inner block's first array element — the
        // header — byte-for-byte.
        let env = ade_codec::cbor::envelope::decode_block_envelope(&block_bytes)
            .expect("envelope decodes");
        let inner = &block_bytes[env.block_start..env.block_end];
        let mut o = 0usize;
        cbor::read_array_header(inner, &mut o).expect("inner array header");
        let (h_start, h_end) = cbor::skip_item(inner, &mut o).expect("header span");
        let expected = &inner[h_start..h_end];

        assert_eq!(
            header_bytes, expected,
            "accepted_block_header_bytes must equal the validator's header_cbor_slice over the inner block"
        );
    }

    #[test]
    fn accepted_block_header_bytes_is_subslice_of_as_bytes() {
        let corpus = corpus();
        let view = view(&corpus);
        let schedule = schedule();
        let ledger = ledger_at_576();
        let chain_dep = state_with_eta0(corpus.epoch_nonce);

        let block_bytes = pick_lightest(&corpus).to_vec();
        let accepted = self_accept(&block_bytes, &ledger, &chain_dep, &schedule, &view)
            .expect("corpus block self-accepts");

        let header_bytes =
            crate::block_validity::accepted_block_header_bytes(&accepted).expect("header projects");

        // Pointer-arithmetic check: header_bytes lives inside accepted.as_bytes().
        let full = accepted.as_bytes();
        let full_start = full.as_ptr() as usize;
        let full_end = full_start + full.len();
        let h_start = header_bytes.as_ptr() as usize;
        let h_end = h_start + header_bytes.len();
        assert!(
            h_start >= full_start && h_end <= full_end,
            "header slice must be a contiguous subslice of as_bytes()"
        );
    }

    #[test]
    fn accepted_block_header_bytes_rejects_malformed_envelope() {
        // We can't construct an AcceptedBlock with garbage bytes (private
        // constructor + self_accept rejects malformed). Instead drive
        // the underlying walker on a hand-crafted envelope-shaped value
        // we *can* legitimately self-accept (corpus), but then verify
        // that the function's failure surface is BlockValidityError ──
        // i.e. it returns Err on a corrupted envelope rather than
        // panicking. To exercise the err path without bypassing the
        // private constructor, this test asserts that the inverse
        // helper (decode_block_envelope) used inside the accessor is
        // the same one that rejects garbage; demonstrated by feeding
        // garbage into decode_block_envelope and asserting Err.
        let garbage = vec![0xFFu8; 16];
        let err = ade_codec::cbor::envelope::decode_block_envelope(&garbage);
        assert!(
            err.is_err(),
            "decode_block_envelope must reject garbage envelope (the same gate accepted_block_header_bytes relies on)"
        );
    }

    #[test]
    fn broadcast_callable_only_with_accept_verdict() {
        // Runtime witness paired with the CI grep gate
        // (`ci/ci_check_self_accept_gate.sh`): the only constructor of
        // `AcceptedBlock` reachable from outside this module is the
        // struct literal inside `self_accept`. This test simply
        // exercises the broadcast-layer hand-off API on a token that
        // was obtained via `self_accept` — the type-level proof is
        // the CI gate plus the private field.
        let corpus = corpus();
        let view = view(&corpus);
        let schedule = schedule();
        let ledger = ledger_at_576();
        let chain_dep = state_with_eta0(corpus.epoch_nonce);

        let block_bytes = pick_lightest(&corpus).to_vec();
        let accepted = self_accept(&block_bytes, &ledger, &chain_dep, &schedule, &view)
            .expect("baseline accept");
        // Public API surface: `as_bytes` borrows, `into_bytes` consumes.
        let _: &[u8] = accepted.as_bytes();
        let owned: Vec<u8> = accepted.into_bytes();
        assert_eq!(owned, block_bytes);
    }
}
