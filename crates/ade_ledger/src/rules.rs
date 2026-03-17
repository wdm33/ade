// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use ade_codec::byron;
use ade_types::CardanoEra;
use crate::error::{LedgerError, RuleNotYetEnforcedError, RuleName};
use crate::state::LedgerState;

/// Apply a block to ledger state, dispatching by era.
///
/// Byron blocks are validated via `crate::byron::validate_byron_block`.
/// Byron EBBs pass through (no transactions).
/// Shelley through Mary return `RuleNotYetEnforced` until their slices land.
pub fn apply_block(
    state: &LedgerState,
    era: CardanoEra,
    block_cbor: &[u8],
) -> Result<LedgerState, LedgerError> {
    match era {
        CardanoEra::ByronEbb => {
            // EBBs contain no transactions — pass-through, state unchanged
            Ok(state.clone())
        }
        CardanoEra::ByronRegular => {
            let preserved = byron::decode_byron_regular_block(block_cbor)?;
            let block = preserved.decoded();
            crate::byron::validate_byron_block(state, block)
        }
        _ => Err(LedgerError::RuleNotYetEnforced(RuleNotYetEnforcedError {
            era,
            rule: RuleName::ApplyBlock,
        })),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn apply_block_byron_ebb_passes_through() {
        // Build a minimal valid Byron EBB block CBOR:
        // array(3) [header, body, extra]
        // EBB blocks contain no transactions — state passes through unchanged.
        // We use a known-good EBB from the codec test that already loaded it.
        // Here we just verify that any Byron EBB returns Ok with state unchanged.
        let state = LedgerState::new(CardanoEra::ByronEbb);

        // Construct minimal EBB inner block bytes via ade_codec encoding
        use ade_codec::traits::{AdeEncode, CodecContext};
        use ade_types::byron::block::{ByronEbbBlock, ByronEbbHeader};
        use ade_types::Hash32;

        let ebb = ByronEbbBlock {
            header: ByronEbbHeader {
                protocol_magic: 764824073,
                prev_hash: Hash32([0u8; 32]),
                body_proof: Hash32([0u8; 32]),
                epoch: 0,
                chain_difficulty: 0,
                extra_data: vec![0x81, 0xa0],
            },
            body: vec![0x80],  // empty array
            extra: vec![0xa0], // empty map
        };
        let ctx = CodecContext {
            era: CardanoEra::ByronEbb,
        };
        let mut buf = Vec::new();
        ebb.ade_encode(&mut buf, &ctx).unwrap();

        let result = apply_block(&state, CardanoEra::ByronEbb, &buf);
        assert!(result.is_ok());
        // State unchanged — no transactions in EBB
        assert_eq!(result.unwrap(), state);
    }

    #[test]
    fn apply_block_shelley_returns_not_yet_enforced() {
        let state = LedgerState::new(CardanoEra::Shelley);
        let result = apply_block(&state, CardanoEra::Shelley, &[]);
        assert!(matches!(
            result,
            Err(LedgerError::RuleNotYetEnforced(RuleNotYetEnforcedError {
                era: CardanoEra::Shelley,
                rule: RuleName::ApplyBlock,
            }))
        ));
    }

    #[test]
    fn apply_block_deterministic() {
        let state = LedgerState::new(CardanoEra::Mary);
        let result1 = apply_block(&state, CardanoEra::Mary, &[0x83, 0x01, 0x02]);
        let result2 = apply_block(&state, CardanoEra::Mary, &[0x83, 0x01, 0x02]);
        assert_eq!(result1, result2);
    }
}
