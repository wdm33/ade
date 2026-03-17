// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use ade_types::tx::Coin;
use ade_types::CardanoEra;

use crate::epoch::SnapshotState;
use crate::error::{LedgerError, TranslationError, TranslationFailureReason};
use crate::pparams::ProtocolParameters;
use crate::state::{EpochState, LedgerState};

/// Translate Byron ledger state to Shelley.
///
/// This is the hard fork from Byron to Shelley. The key transformations:
/// - UTxO set carries over (Byron outputs remain valid)
/// - Protocol parameters are replaced with Shelley genesis parameters
/// - Epoch state is carried over, snapshot pipeline initialized
/// - Era tag changes to Shelley
///
/// Deterministic: same Byron state always produces the same Shelley state.
pub fn translate_byron_to_shelley(
    old_state: &LedgerState,
) -> Result<LedgerState, LedgerError> {
    // Validate source state is Byron
    if !old_state.era.is_byron() {
        return Err(LedgerError::Translation(TranslationError {
            from_era: old_state.era,
            to_era: CardanoEra::Shelley,
            reason: TranslationFailureReason::InvalidSourceState,
        }));
    }

    // UTxO set carries over unchanged — Byron outputs are valid in Shelley
    let utxo_state = old_state.utxo_state.clone();

    // Initialize Shelley protocol parameters from genesis defaults
    let protocol_params = ProtocolParameters::default();

    // Carry over epoch state, initialize snapshot pipeline
    let epoch_state = EpochState {
        epoch: old_state.epoch_state.epoch,
        slot: old_state.epoch_state.slot,
        snapshots: SnapshotState::new(),
        reserves: initial_shelley_reserves(&utxo_state),
        treasury: Coin(0),
    };

    Ok(LedgerState {
        utxo_state,
        epoch_state,
        protocol_params,
        era: CardanoEra::Shelley,
    })
}

/// Translate Shelley ledger state to Allegra.
///
/// Shelley -> Allegra is a soft-fork style transition:
/// - UTxO set carries over unchanged
/// - Protocol parameters carry over (validity intervals are new in tx body,
///   but the parameter set is compatible)
/// - Epoch state carries over with snapshots
/// - Era tag changes to Allegra
pub fn translate_shelley_to_allegra(
    old_state: &LedgerState,
) -> Result<LedgerState, LedgerError> {
    if old_state.era != CardanoEra::Shelley {
        return Err(LedgerError::Translation(TranslationError {
            from_era: old_state.era,
            to_era: CardanoEra::Allegra,
            reason: TranslationFailureReason::InvalidSourceState,
        }));
    }

    Ok(LedgerState {
        utxo_state: old_state.utxo_state.clone(),
        epoch_state: old_state.epoch_state.clone(),
        protocol_params: old_state.protocol_params.clone(),
        era: CardanoEra::Allegra,
    })
}

/// Translate Allegra ledger state to Mary.
///
/// Allegra -> Mary adds multi-asset support:
/// - UTxO set carries over (Allegra outputs are valid in Mary)
/// - Protocol parameters carry over
/// - Epoch state carries over with snapshots
/// - Era tag changes to Mary
pub fn translate_allegra_to_mary(
    old_state: &LedgerState,
) -> Result<LedgerState, LedgerError> {
    if old_state.era != CardanoEra::Allegra {
        return Err(LedgerError::Translation(TranslationError {
            from_era: old_state.era,
            to_era: CardanoEra::Mary,
            reason: TranslationFailureReason::InvalidSourceState,
        }));
    }

    Ok(LedgerState {
        utxo_state: old_state.utxo_state.clone(),
        epoch_state: old_state.epoch_state.clone(),
        protocol_params: old_state.protocol_params.clone(),
        era: CardanoEra::Mary,
    })
}

/// Compute initial Shelley reserves from the UTxO set.
///
/// Total ADA supply is 45 billion ADA = 45_000_000_000_000_000 lovelace.
/// Reserves = total supply - current UTxO total.
fn initial_shelley_reserves(utxo_state: &crate::utxo::UTxOState) -> Coin {
    const TOTAL_SUPPLY: u64 = 45_000_000_000_000_000;

    let utxo_total: u64 = utxo_state
        .utxos
        .values()
        .map(|out| out.coin().0)
        .fold(0u64, |acc, c| acc.saturating_add(c));

    Coin(TOTAL_SUPPLY.saturating_sub(utxo_total))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use ade_types::address::Address;
    use ade_types::tx::TxIn;
    use ade_types::{EpochNo, Hash32, SlotNo};
    use crate::utxo::{utxo_insert, TxOut, UTxOState};

    fn make_byron_state() -> LedgerState {
        LedgerState {
            utxo_state: UTxOState::new(),
            epoch_state: EpochState {
                epoch: EpochNo(208),
                slot: SlotNo(4_492_800),
                snapshots: SnapshotState::new(),
                reserves: Coin(0),
                treasury: Coin(0),
            },
            protocol_params: ProtocolParameters::default(),
            era: CardanoEra::ByronRegular,
        }
    }

    fn make_shelley_state() -> LedgerState {
        LedgerState {
            utxo_state: UTxOState::new(),
            epoch_state: EpochState {
                epoch: EpochNo(208),
                slot: SlotNo(4_492_800),
                snapshots: SnapshotState::new(),
                reserves: Coin(45_000_000_000_000_000),
                treasury: Coin(0),
            },
            protocol_params: ProtocolParameters::default(),
            era: CardanoEra::Shelley,
        }
    }

    fn make_allegra_state() -> LedgerState {
        LedgerState {
            utxo_state: UTxOState::new(),
            epoch_state: EpochState {
                epoch: EpochNo(236),
                slot: SlotNo(16_588_800),
                snapshots: SnapshotState::new(),
                reserves: Coin(45_000_000_000_000_000),
                treasury: Coin(0),
            },
            protocol_params: ProtocolParameters::default(),
            era: CardanoEra::Allegra,
        }
    }

    // -----------------------------------------------------------------------
    // Byron -> Shelley
    // -----------------------------------------------------------------------

    #[test]
    fn byron_to_shelley_basic() {
        let byron = make_byron_state();
        let shelley = translate_byron_to_shelley(&byron).unwrap();

        assert_eq!(shelley.era, CardanoEra::Shelley);
        assert_eq!(shelley.epoch_state.epoch, EpochNo(208));
        assert_eq!(shelley.epoch_state.slot, SlotNo(4_492_800));
        assert!(shelley.utxo_state.is_empty());
    }

    #[test]
    fn byron_to_shelley_preserves_utxo() {
        let mut state = make_byron_state();
        let tx_in = TxIn {
            tx_hash: Hash32([0xaa; 32]),
            index: 0,
        };
        let tx_out = TxOut::Byron {
            address: Address::Byron(vec![0x82, 0xd8, 0x18]),
            coin: Coin(5_000_000),
        };
        state.utxo_state = utxo_insert(&state.utxo_state, tx_in.clone(), tx_out.clone());

        let shelley = translate_byron_to_shelley(&state).unwrap();
        assert_eq!(shelley.utxo_state.len(), 1);
        assert_eq!(shelley.utxo_state.utxos.get(&tx_in), Some(&tx_out));
    }

    #[test]
    fn byron_to_shelley_computes_reserves() {
        let mut state = make_byron_state();
        let tx_in = TxIn {
            tx_hash: Hash32([0xbb; 32]),
            index: 0,
        };
        let tx_out = TxOut::Byron {
            address: Address::Byron(vec![0x01]),
            coin: Coin(1_000_000_000),
        };
        state.utxo_state = utxo_insert(&state.utxo_state, tx_in, tx_out);

        let shelley = translate_byron_to_shelley(&state).unwrap();
        // reserves = total_supply - utxo_total
        assert_eq!(
            shelley.epoch_state.reserves,
            Coin(45_000_000_000_000_000 - 1_000_000_000)
        );
    }

    #[test]
    fn byron_to_shelley_initializes_shelley_params() {
        let byron = make_byron_state();
        let shelley = translate_byron_to_shelley(&byron).unwrap();

        let defaults = ProtocolParameters::default();
        assert_eq!(shelley.protocol_params, defaults);
    }

    #[test]
    fn byron_to_shelley_rejects_non_byron_source() {
        let state = make_shelley_state();
        let result = translate_byron_to_shelley(&state);
        assert!(matches!(
            result,
            Err(LedgerError::Translation(TranslationError {
                from_era: CardanoEra::Shelley,
                to_era: CardanoEra::Shelley,
                reason: TranslationFailureReason::InvalidSourceState,
            }))
        ));
    }

    #[test]
    fn byron_ebb_translates_to_shelley() {
        let state = LedgerState {
            utxo_state: UTxOState::new(),
            epoch_state: EpochState::new(),
            protocol_params: ProtocolParameters::default(),
            era: CardanoEra::ByronEbb,
        };
        let result = translate_byron_to_shelley(&state);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().era, CardanoEra::Shelley);
    }

    // -----------------------------------------------------------------------
    // Shelley -> Allegra
    // -----------------------------------------------------------------------

    #[test]
    fn shelley_to_allegra_basic() {
        let shelley = make_shelley_state();
        let allegra = translate_shelley_to_allegra(&shelley).unwrap();

        assert_eq!(allegra.era, CardanoEra::Allegra);
        assert_eq!(allegra.epoch_state.epoch, shelley.epoch_state.epoch);
        assert_eq!(allegra.protocol_params, shelley.protocol_params);
    }

    #[test]
    fn shelley_to_allegra_preserves_utxo() {
        let mut shelley = make_shelley_state();
        let tx_in = TxIn {
            tx_hash: Hash32([0xcc; 32]),
            index: 0,
        };
        let tx_out = TxOut::ShelleyMary {
            address: vec![0x01, 0x02],
            value: crate::value::Value::from_coin(Coin(2_000_000)),
        };
        shelley.utxo_state = utxo_insert(&shelley.utxo_state, tx_in.clone(), tx_out.clone());

        let allegra = translate_shelley_to_allegra(&shelley).unwrap();
        assert_eq!(allegra.utxo_state.len(), 1);
        assert_eq!(allegra.utxo_state.utxos.get(&tx_in), Some(&tx_out));
    }

    #[test]
    fn shelley_to_allegra_rejects_non_shelley() {
        let state = make_allegra_state();
        let result = translate_shelley_to_allegra(&state);
        assert!(matches!(
            result,
            Err(LedgerError::Translation(TranslationError {
                from_era: CardanoEra::Allegra,
                to_era: CardanoEra::Allegra,
                reason: TranslationFailureReason::InvalidSourceState,
            }))
        ));
    }

    // -----------------------------------------------------------------------
    // Allegra -> Mary
    // -----------------------------------------------------------------------

    #[test]
    fn allegra_to_mary_basic() {
        let allegra = make_allegra_state();
        let mary = translate_allegra_to_mary(&allegra).unwrap();

        assert_eq!(mary.era, CardanoEra::Mary);
        assert_eq!(mary.epoch_state.epoch, allegra.epoch_state.epoch);
        assert_eq!(mary.protocol_params, allegra.protocol_params);
    }

    #[test]
    fn allegra_to_mary_preserves_utxo() {
        let mut allegra = make_allegra_state();
        let tx_in = TxIn {
            tx_hash: Hash32([0xdd; 32]),
            index: 0,
        };
        let tx_out = TxOut::ShelleyMary {
            address: vec![0x01],
            value: crate::value::Value::from_coin(Coin(3_000_000)),
        };
        allegra.utxo_state = utxo_insert(&allegra.utxo_state, tx_in.clone(), tx_out.clone());

        let mary = translate_allegra_to_mary(&allegra).unwrap();
        assert_eq!(mary.utxo_state.len(), 1);
    }

    #[test]
    fn allegra_to_mary_rejects_non_allegra() {
        let state = make_shelley_state();
        let result = translate_allegra_to_mary(&state);
        assert!(matches!(
            result,
            Err(LedgerError::Translation(TranslationError {
                from_era: CardanoEra::Shelley,
                to_era: CardanoEra::Mary,
                reason: TranslationFailureReason::InvalidSourceState,
            }))
        ));
    }

    // -----------------------------------------------------------------------
    // Full translation chain
    // -----------------------------------------------------------------------

    #[test]
    fn full_translation_chain_byron_through_mary() {
        let byron = make_byron_state();
        let shelley = translate_byron_to_shelley(&byron).unwrap();
        let allegra = translate_shelley_to_allegra(&shelley).unwrap();
        let mary = translate_allegra_to_mary(&allegra).unwrap();

        assert_eq!(mary.era, CardanoEra::Mary);
        assert_eq!(mary.epoch_state.epoch, byron.epoch_state.epoch);
    }

    #[test]
    fn translation_is_deterministic() {
        let byron = make_byron_state();
        let s1 = translate_byron_to_shelley(&byron).unwrap();
        let s2 = translate_byron_to_shelley(&byron).unwrap();
        assert_eq!(s1, s2);

        let a1 = translate_shelley_to_allegra(&s1).unwrap();
        let a2 = translate_shelley_to_allegra(&s2).unwrap();
        assert_eq!(a1, a2);

        let m1 = translate_allegra_to_mary(&a1).unwrap();
        let m2 = translate_allegra_to_mary(&a2).unwrap();
        assert_eq!(m1, m2);
    }

    #[test]
    fn shelley_reserves_correct_for_empty_utxo() {
        let reserves = initial_shelley_reserves(&UTxOState::new());
        assert_eq!(reserves, Coin(45_000_000_000_000_000));
    }

    #[test]
    fn shelley_reserves_saturates_on_overflow() {
        // If UTxO total somehow exceeds supply, reserves should saturate at 0
        let mut utxo = UTxOState::new();
        let tx_in = TxIn {
            tx_hash: Hash32([0xff; 32]),
            index: 0,
        };
        let tx_out = TxOut::Byron {
            address: Address::Byron(vec![0x01]),
            coin: Coin(u64::MAX),
        };
        utxo = utxo_insert(&utxo, tx_in, tx_out);

        let reserves = initial_shelley_reserves(&utxo);
        assert_eq!(reserves, Coin(0));
    }
}
