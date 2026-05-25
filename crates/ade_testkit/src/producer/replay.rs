// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Producer replay harness for PHASE4-N-C S3 / CE-N-C-3.
//!
//! Drives `ade_ledger::producer::forge::forge_block` over a small corpus
//! of canonical [`ProducerReplayFixture`] entries. The harness proves
//! two properties:
//!
//! - `forge_block_pure_no_io` — running forge twice on the same tick
//!   produces byte-identical outputs (no clock, no rand, no ambient
//!   state).
//! - `forge_block_replay_byte_identical` — for fixtures carrying a
//!   captured `expected_forged`, the tick's forged bytes match the
//!   captured corpus.
//!
//! Fixtures are produced by [`crate::producer::fixtures`]; private-key
//! material never appears in the harness path (the regen helper at
//! `crates/ade_testkit/tests/regen_producer_fixtures.rs` is the only
//! call site that pulls RED signing primitives, and even there the
//! private bytes never leave the helper's stack frame).

use ade_ledger::producer::state::ProducerTick;

/// A single replay fixture: ordered ticks plus expected per-tick output.
///
/// `expected_forged[i]` is the captured bytes of `forge_block(&ticks[i])`'s
/// `Ok` arm when the i-th tick is a positive case; empty `Vec` when the
/// tick is a negative case, in which case `expected_err_tag[i]` carries
/// the expected `ForgeError` discriminant.
pub struct ProducerReplayFixture {
    pub label: &'static str,
    pub ticks: Vec<ProducerTick>,
    pub expected_forged: Vec<Vec<u8>>,
    pub expected_err_tag: Vec<Option<ExpectedErr>>,
}

/// Coarse discriminant for the expected error path; replay matches by
/// variant tag only, not detailed payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpectedErr {
    NotLeader,
    TxSetNotAdmissiblePrefix,
    OpCertRejected,
    MempoolWidthMismatch,
}

/// The S3 fixture set. Currently three entries (positive empty mempool,
/// negative non-leader, negative width-mismatch). Two-tx mempool with
/// trivially-valid Conway txs is deferred per the slice doc's documented
/// deviation (no reachable in-source trivially-valid Conway tx fixture
/// at this slice's HEAD).
pub fn producer_replay_fixtures() -> Vec<ProducerReplayFixture> {
    crate::producer::fixtures::all_fixtures()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use ade_codec::conway::decode_conway_block;
    use ade_ledger::block_body_hash::{block_body_hash, block_body_hash_from_buckets};
    use ade_ledger::producer::forge::forge_block;
    use ade_types::Hash32;

    fn err_tag(err: &ade_ledger::producer::forge::ForgeError) -> ExpectedErr {
        match err {
            ade_ledger::producer::forge::ForgeError::NotLeader { .. } => {
                ExpectedErr::NotLeader
            }
            ade_ledger::producer::forge::ForgeError::OpCertRejected(_) => {
                ExpectedErr::OpCertRejected
            }
            ade_ledger::producer::forge::ForgeError::TxSetNotAdmissiblePrefix {
                ..
            } => ExpectedErr::TxSetNotAdmissiblePrefix,
            ade_ledger::producer::forge::ForgeError::MempoolWidthMismatch { .. } => {
                ExpectedErr::MempoolWidthMismatch
            }
            ade_ledger::producer::forge::ForgeError::MempoolAcceptedMismatch { .. } => {
                ExpectedErr::TxSetNotAdmissiblePrefix
            }
            ade_ledger::producer::forge::ForgeError::BadKesSignatureLength { .. } => {
                ExpectedErr::OpCertRejected
            }
            ade_ledger::producer::forge::ForgeError::TxComponentSplit { .. } => {
                ExpectedErr::TxSetNotAdmissiblePrefix
            }
        }
    }

    #[test]
    fn forge_block_pure_no_io() {
        for fixture in producer_replay_fixtures() {
            for (i, tick) in fixture.ticks.iter().enumerate() {
                let a = forge_block(tick);
                let b = forge_block(tick);
                match (a, b) {
                    (Ok((fa, _)), Ok((fb, _))) => {
                        assert_eq!(
                            fa.bytes, fb.bytes,
                            "fixture {} tick {}: two runs produced different bytes",
                            fixture.label, i,
                        );
                    }
                    (Err(ea), Err(eb)) => {
                        assert_eq!(
                            ea, eb,
                            "fixture {} tick {}: two runs produced different errors",
                            fixture.label, i,
                        );
                    }
                    (a, b) => panic!(
                        "fixture {} tick {}: divergent verdicts {:?} vs {:?}",
                        fixture.label, i, a, b
                    ),
                }
            }
        }
    }

    #[test]
    fn forge_block_replay_byte_identical() {
        for fixture in producer_replay_fixtures() {
            assert_eq!(
                fixture.ticks.len(),
                fixture.expected_forged.len(),
                "fixture {}: ticks vs expected_forged length",
                fixture.label,
            );
            assert_eq!(
                fixture.ticks.len(),
                fixture.expected_err_tag.len(),
                "fixture {}: ticks vs expected_err_tag length",
                fixture.label,
            );
            for (i, tick) in fixture.ticks.iter().enumerate() {
                let expected_bytes = &fixture.expected_forged[i];
                let expected_err = fixture.expected_err_tag[i];
                let got = forge_block(tick);
                if expected_bytes.is_empty() {
                    let err = got.unwrap_err();
                    let tag = err_tag(&err);
                    assert_eq!(
                        Some(tag),
                        expected_err,
                        "fixture {} tick {}: expected err {:?} got {:?}",
                        fixture.label,
                        i,
                        expected_err,
                        err,
                    );
                } else {
                    let (forged, _effects) = got.unwrap();
                    assert_eq!(
                        forged.bytes, *expected_bytes,
                        "fixture {} tick {}: forged bytes diverge from captured corpus",
                        fixture.label, i,
                    );
                }
            }
        }
    }

    #[test]
    fn forged_body_hash_matches_validator_recomputation() {
        for fixture in producer_replay_fixtures() {
            for (i, tick) in fixture.ticks.iter().enumerate() {
                let expected_bytes = &fixture.expected_forged[i];
                if expected_bytes.is_empty() {
                    continue;
                }
                let (forged, _effects) = forge_block(tick).unwrap();

                let decoded = decode_conway_block(&forged.bytes).unwrap();
                let block = decoded.decoded();

                let recomputed = block_body_hash(block);
                assert_eq!(
                    recomputed, block.header.body.body_hash,
                    "fixture {} tick {}: validator recomputation diverges from emitted header.body_hash",
                    fixture.label, i,
                );
                assert_eq!(
                    recomputed, forged.block.header.body.body_hash,
                    "fixture {} tick {}: forged in-memory header.body_hash diverges from validator recomputation",
                    fixture.label, i,
                );
            }
        }
    }

    #[test]
    fn body_encoder_is_single_authority() {
        const _A: fn(&[u8], &[u8], &[u8], Option<&[u8]>) -> Hash32 = block_body_hash_from_buckets;
        const _B: fn(&ade_types::shelley::block::ShelleyBlock) -> Hash32 = block_body_hash;

        let fixture = &producer_replay_fixtures()[0];
        let tick = &fixture.ticks[0];
        let (forged, _) = forge_block(tick).unwrap();
        let block = &forged.block;
        assert_eq!(
            block_body_hash(block),
            block_body_hash_from_buckets(
                &block.tx_bodies,
                &block.witness_sets,
                &block.metadata,
                block.invalid_txs.as_deref(),
            ),
        );
    }
}
