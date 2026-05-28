// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Mechanical cross-impl adapter: drive every fixture in
//! `producer_replay_fixtures()` through the full producer pipeline
//! and assert structural cross-impl agreement — decode round-trips,
//! body-hash binding via S4's authority, structural field agreement
//! across forge ⊕ decoder. CN-CONS-06's mechanical half lands here;
//! the crypto-level cross-impl claim (cardano-node acceptance over
//! N2N) lives in CE-N-C-8's operator-action live evidence.
//!
//! The synthetic corpus carries all-zero KES / VRF artifacts by
//! design (RED signing is regen-only; replay drives BLUE), so a
//! "passes cardano-node KES/VRF verify" claim is unrepresentable
//! here. The adapter therefore asserts three honest structural
//! properties:
//!
//! 1. `decode_shelley_block_inner(forged.bytes)` returns `Ok(_)`.
//! 2. `block_body_hash(&decoded)` re-binds the emitted
//!    `header.body_hash` (S4's body-hash recipe on the cross-impl
//!    surface).
//! 3. Decoded `header.body_hash`, `operational_cert.sequence_number`,
//!    and `operational_cert.kes_period` match the in-memory
//!    `ForgedBlock.block` the producer constructed (decoder ⊕ encoder
//!    ≈ identity for these load-bearing fields).

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
mod tests {
    use ade_codec::shelley::block::decode_shelley_block_inner;
    use ade_ledger::block_body_hash::block_body_hash;
    use ade_ledger::producer::forge::forge_block;

    use crate::producer::replay::producer_replay_fixtures;

    #[test]
    fn cross_impl_adapter_forged_block_decodes_through_ade_codec() {
        for fixture in producer_replay_fixtures() {
            for (i, expected) in fixture.expected_forged.iter().enumerate() {
                if expected.is_empty() {
                    continue;
                }
                // PHASE4-N-V: expected_forged is now the era-tagged envelope
                // `[era, block]`; strip it before decoding the bare inner block.
                let env =
                    ade_codec::cbor::envelope::decode_block_envelope(expected).unwrap();
                let inner = &expected[env.block_start..env.block_end];
                let mut offset = 0usize;
                let decoded = decode_shelley_block_inner(inner, &mut offset).unwrap_or_else(|e| {
                    panic!(
                        "fixture {} tick {}: decode_shelley_block_inner rejected captured \
                         expected_forged inner bytes: {:?}",
                        fixture.label, i, e
                    )
                });
                assert_eq!(
                    offset,
                    inner.len(),
                    "fixture {} tick {}: decoder left trailing bytes ({} of {} consumed)",
                    fixture.label,
                    i,
                    offset,
                    inner.len(),
                );
                // Force the decoded value to be used so the compiler
                // cannot elide the decode call entirely.
                assert_eq!(
                    decoded.tx_count as usize,
                    fixture.ticks[i].mempool_tx_bytes.len(),
                    "fixture {} tick {}: decoded tx_count diverges from tick mempool width",
                    fixture.label,
                    i,
                );
            }
        }
    }

    #[test]
    fn cross_impl_adapter_forged_block_structurally_agrees_with_decoder() {
        for fixture in producer_replay_fixtures() {
            for (i, expected) in fixture.expected_forged.iter().enumerate() {
                if expected.is_empty() {
                    continue;
                }

                // (1) Decode round-trip on the captured expected_forged
                // bytes.
                // PHASE4-N-V: expected_forged is now the era-tagged envelope
                // `[era, block]`; strip it before decoding the bare inner block.
                let env =
                    ade_codec::cbor::envelope::decode_block_envelope(expected).unwrap();
                let inner = &expected[env.block_start..env.block_end];
                let mut offset = 0usize;
                let decoded = decode_shelley_block_inner(inner, &mut offset).unwrap_or_else(|e| {
                    panic!(
                        "fixture {} tick {}: decode_shelley_block_inner rejected captured \
                         expected_forged inner bytes: {:?}",
                        fixture.label, i, e
                    )
                });
                assert_eq!(
                    offset,
                    inner.len(),
                    "fixture {} tick {}: decoder left trailing bytes ({} of {} consumed)",
                    fixture.label,
                    i,
                    offset,
                    inner.len(),
                );

                // (2) Body-hash binding via S4's canonical recipe — the
                // producer wrote bytes the validator's body-hash recipe
                // accepts.
                let recomputed = block_body_hash(&decoded);
                assert_eq!(
                    recomputed, decoded.header.body.body_hash,
                    "fixture {} tick {}: block_body_hash(&decoded) diverges from emitted \
                     header.body_hash — cross-impl body-hash binding broken",
                    fixture.label, i,
                );

                // (3) Structural field agreement: re-run forge against
                // the tick and compare the load-bearing header fields
                // through the decoder ⊕ encoder surface.
                let (forged, _effects) = forge_block(&fixture.ticks[i]).unwrap_or_else(|e| {
                    panic!(
                        "fixture {} tick {}: forge_block rejected a positive-case tick: {:?}",
                        fixture.label, i, e
                    )
                });
                assert_eq!(
                    decoded.header.body.body_hash, forged.block.header.body.body_hash,
                    "fixture {} tick {}: decoded header.body_hash diverges from forged \
                     in-memory block",
                    fixture.label, i,
                );
                assert_eq!(
                    decoded.header.body.operational_cert.sequence_number,
                    forged.block.header.body.operational_cert.sequence_number,
                    "fixture {} tick {}: decoded opcert sequence_number diverges from forged \
                     in-memory block",
                    fixture.label, i,
                );
                assert_eq!(
                    decoded.header.body.operational_cert.kes_period,
                    forged.block.header.body.operational_cert.kes_period,
                    "fixture {} tick {}: decoded opcert kes_period diverges from forged \
                     in-memory block",
                    fixture.label, i,
                );
            }
        }
    }

    #[test]
    fn cross_impl_adapter_corpus_round_trips_byte_identical() {
        for fixture in producer_replay_fixtures() {
            for (i, expected) in fixture.expected_forged.iter().enumerate() {
                if expected.is_empty() {
                    continue;
                }
                let tick = &fixture.ticks[i];
                let (a, _) = forge_block(tick).unwrap_or_else(|e| {
                    panic!(
                        "fixture {} tick {}: forge_block rejected positive tick: {:?}",
                        fixture.label, i, e
                    )
                });
                let (b, _) = forge_block(tick).unwrap_or_else(|e| {
                    panic!(
                        "fixture {} tick {}: forge_block rejected positive tick on second pass: \
                         {:?}",
                        fixture.label, i, e
                    )
                });
                assert_eq!(
                    a.bytes, b.bytes,
                    "fixture {} tick {}: two forge passes produced different bytes",
                    fixture.label, i,
                );
                assert_eq!(
                    a.bytes, *expected,
                    "fixture {} tick {}: forged bytes diverge from captured expected_forged",
                    fixture.label, i,
                );
            }
        }
    }
}
