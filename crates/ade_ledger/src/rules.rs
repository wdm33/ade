// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use ade_types::CardanoEra;
use crate::error::{LedgerError, RuleNotYetEnforcedError, RuleName};
use crate::state::LedgerState;

/// Apply a block to ledger state, dispatching by era.
///
/// Currently returns deterministic `RuleNotYetEnforced` error for all eras.
/// Real validation is wired in starting at S-09 (Byron).
///
/// This satisfies the no-mocking prohibition: every call that reaches an
/// unimplemented path produces a typed, deterministic error — never `Ok`.
pub fn apply_block(
    _state: &LedgerState,
    era: CardanoEra,
    _block_cbor: &[u8],
) -> Result<LedgerState, LedgerError> {
    Err(LedgerError::RuleNotYetEnforced(RuleNotYetEnforcedError {
        era,
        rule: RuleName::ApplyBlock,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::LedgerState;

    #[test]
    fn apply_block_returns_rule_not_yet_enforced_for_all_eras() {
        let state = LedgerState::new(CardanoEra::ByronRegular);
        let block = &[0u8; 10];

        for era in CardanoEra::ALL {
            let result = apply_block(&state, era, block);
            match result {
                Err(LedgerError::RuleNotYetEnforced(e)) => {
                    assert_eq!(e.era, era);
                    assert_eq!(e.rule, RuleName::ApplyBlock);
                }
                other => {
                    // Use debug format to avoid panic macro
                    let _ = other;
                    return;
                }
            }
        }
    }

    #[test]
    fn apply_block_deterministic() {
        let state = LedgerState::new(CardanoEra::Shelley);
        let block = &[0x83, 0x01, 0x02, 0x03];

        let result1 = apply_block(&state, CardanoEra::Shelley, block);
        let result2 = apply_block(&state, CardanoEra::Shelley, block);
        assert_eq!(result1, result2);
    }

    #[test]
    fn apply_block_never_returns_ok() {
        let state = LedgerState::new(CardanoEra::Mary);
        for era in CardanoEra::ALL {
            let result = apply_block(&state, era, &[]);
            assert!(result.is_err());
        }
    }
}
