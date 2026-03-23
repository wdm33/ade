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
        block_production: std::collections::BTreeMap::new(),
        epoch_fees: Coin(0),
    };

    Ok(LedgerState {
        utxo_state,
        epoch_state,
        protocol_params,
        era: CardanoEra::Shelley,
        track_utxo: old_state.track_utxo,
        cert_state: old_state.cert_state.clone(),
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
        track_utxo: old_state.track_utxo,
        cert_state: old_state.cert_state.clone(),
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
        track_utxo: old_state.track_utxo,
        cert_state: old_state.cert_state.clone(),
    })
}

/// Translate Mary ledger state to Alonzo.
///
/// Mary -> Alonzo adds Plutus infrastructure:
/// - UTxO set carries over (Mary outputs are valid in Alonzo)
/// - Protocol parameters carry over (Alonzo-specific params like
///   cost models are loaded from Alonzo genesis, not from translation)
/// - Epoch state carries over with snapshots
/// - Era tag changes to Alonzo
///
/// Note: Alonzo genesis parameters (cost models, execution unit limits,
/// collateral percentage) are loaded from the genesis file, not derived
/// from the Mary state. This translation only handles the era boundary.
pub fn translate_mary_to_alonzo(
    old_state: &LedgerState,
) -> Result<LedgerState, LedgerError> {
    if old_state.era != CardanoEra::Mary {
        return Err(LedgerError::Translation(TranslationError {
            from_era: old_state.era,
            to_era: CardanoEra::Alonzo,
            reason: TranslationFailureReason::InvalidSourceState,
        }));
    }

    Ok(LedgerState {
        utxo_state: old_state.utxo_state.clone(),
        epoch_state: old_state.epoch_state.clone(),
        protocol_params: old_state.protocol_params.clone(),
        era: CardanoEra::Alonzo,
        track_utxo: old_state.track_utxo,
        cert_state: old_state.cert_state.clone(),
    })
}

/// Translate Alonzo ledger state to Babbage.
///
/// Alonzo -> Babbage adds inline datums, reference scripts, reference inputs:
/// - UTxO set carries over (Alonzo outputs are valid in Babbage)
/// - Protocol parameters carry over
/// - Epoch state carries over with snapshots
/// - Era tag changes to Babbage
pub fn translate_alonzo_to_babbage(
    old_state: &LedgerState,
) -> Result<LedgerState, LedgerError> {
    if old_state.era != CardanoEra::Alonzo {
        return Err(LedgerError::Translation(TranslationError {
            from_era: old_state.era,
            to_era: CardanoEra::Babbage,
            reason: TranslationFailureReason::InvalidSourceState,
        }));
    }

    Ok(LedgerState {
        utxo_state: old_state.utxo_state.clone(),
        epoch_state: old_state.epoch_state.clone(),
        protocol_params: old_state.protocol_params.clone(),
        era: CardanoEra::Babbage,
        track_utxo: old_state.track_utxo,
        cert_state: old_state.cert_state.clone(),
    })
}

/// Translate Babbage ledger state to Conway.
///
/// Babbage -> Conway adds governance:
/// - UTxO set carries over (Babbage outputs are valid in Conway)
/// - Protocol parameters carry over (Conway governance parameters
///   like DRep thresholds are loaded from Conway genesis)
/// - Epoch state carries over with snapshots
/// - Era tag changes to Conway
///
/// Note: Initial governance state (empty proposals, initial constitutional
/// committee, initial constitution, empty DRep state) comes from the
/// Conway genesis file, not from translation of Babbage state.
pub fn translate_babbage_to_conway(
    old_state: &LedgerState,
) -> Result<LedgerState, LedgerError> {
    if old_state.era != CardanoEra::Babbage {
        return Err(LedgerError::Translation(TranslationError {
            from_era: old_state.era,
            to_era: CardanoEra::Conway,
            reason: TranslationFailureReason::InvalidSourceState,
        }));
    }

    Ok(LedgerState {
        utxo_state: old_state.utxo_state.clone(),
        epoch_state: old_state.epoch_state.clone(),
        protocol_params: old_state.protocol_params.clone(),
        era: CardanoEra::Conway,
        track_utxo: old_state.track_utxo,
        cert_state: old_state.cert_state.clone(),
    })
}

/// Dispatch a translation by era pair.
///
/// Pure function: `(old_state) -> new_state`.
/// Deterministic: same input always produces the same output.
///
/// This is the single entry point for all era translations.
/// The caller (epoch boundary logic or test harness) determines
/// WHEN to call it; this function only determines WHAT happens.
pub fn translate_era(
    old_state: &LedgerState,
    target_era: CardanoEra,
) -> Result<LedgerState, LedgerError> {
    match (old_state.era, target_era) {
        (e, CardanoEra::Shelley) if e.is_byron() => translate_byron_to_shelley(old_state),
        (CardanoEra::Shelley, CardanoEra::Allegra) => translate_shelley_to_allegra(old_state),
        (CardanoEra::Allegra, CardanoEra::Mary) => translate_allegra_to_mary(old_state),
        (CardanoEra::Mary, CardanoEra::Alonzo) => translate_mary_to_alonzo(old_state),
        (CardanoEra::Alonzo, CardanoEra::Babbage) => translate_alonzo_to_babbage(old_state),
        (CardanoEra::Babbage, CardanoEra::Conway) => translate_babbage_to_conway(old_state),
        _ => Err(LedgerError::Translation(TranslationError {
            from_era: old_state.era,
            to_era: target_era,
            reason: TranslationFailureReason::InvalidSourceState,
        })),
    }
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
    use crate::delegation::CertState;
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
                block_production: std::collections::BTreeMap::new(),
                epoch_fees: Coin(0),
            },
            protocol_params: ProtocolParameters::default(),
            era: CardanoEra::ByronRegular,
            track_utxo: false,
            cert_state: CertState::new(),
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
                block_production: std::collections::BTreeMap::new(),
                epoch_fees: Coin(0),
            },
            protocol_params: ProtocolParameters::default(),
            era: CardanoEra::Shelley,
            track_utxo: false,
            cert_state: CertState::new(),
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
                block_production: std::collections::BTreeMap::new(),
                epoch_fees: Coin(0),
            },
            protocol_params: ProtocolParameters::default(),
            era: CardanoEra::Allegra,
            track_utxo: false,
            cert_state: CertState::new(),
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
            track_utxo: false,
            cert_state: CertState::new(),
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

    fn make_mary_state() -> LedgerState {
        LedgerState {
            utxo_state: UTxOState::new(),
            epoch_state: EpochState {
                epoch: EpochNo(251),
                slot: SlotNo(23_068_800),
                snapshots: SnapshotState::new(),
                reserves: Coin(45_000_000_000_000_000),
                treasury: Coin(0),
                block_production: std::collections::BTreeMap::new(),
                epoch_fees: Coin(0),
            },
            protocol_params: ProtocolParameters::default(),
            era: CardanoEra::Mary,
            track_utxo: false,
            cert_state: CertState::new(),
        }
    }

    fn make_alonzo_state() -> LedgerState {
        LedgerState {
            utxo_state: UTxOState::new(),
            epoch_state: EpochState {
                epoch: EpochNo(290),
                slot: SlotNo(39_916_975),
                snapshots: SnapshotState::new(),
                reserves: Coin(45_000_000_000_000_000),
                treasury: Coin(0),
                block_production: std::collections::BTreeMap::new(),
                epoch_fees: Coin(0),
            },
            protocol_params: ProtocolParameters::default(),
            era: CardanoEra::Alonzo,
            track_utxo: false,
            cert_state: CertState::new(),
        }
    }

    fn make_babbage_state() -> LedgerState {
        LedgerState {
            utxo_state: UTxOState::new(),
            epoch_state: EpochState {
                epoch: EpochNo(365),
                slot: SlotNo(72_316_896),
                snapshots: SnapshotState::new(),
                reserves: Coin(45_000_000_000_000_000),
                treasury: Coin(0),
                block_production: std::collections::BTreeMap::new(),
                epoch_fees: Coin(0),
            },
            protocol_params: ProtocolParameters::default(),
            era: CardanoEra::Babbage,
            track_utxo: false,
            cert_state: CertState::new(),
        }
    }

    // -----------------------------------------------------------------------
    // Mary -> Alonzo
    // -----------------------------------------------------------------------

    #[test]
    fn mary_to_alonzo_basic() {
        let mary = make_mary_state();
        let alonzo = translate_mary_to_alonzo(&mary).unwrap();
        assert_eq!(alonzo.era, CardanoEra::Alonzo);
        assert_eq!(alonzo.epoch_state.epoch, mary.epoch_state.epoch);
        assert_eq!(alonzo.protocol_params, mary.protocol_params);
    }

    #[test]
    fn mary_to_alonzo_preserves_utxo() {
        let mut mary = make_mary_state();
        let tx_in = TxIn { tx_hash: Hash32([0xee; 32]), index: 0 };
        let tx_out = TxOut::ShelleyMary {
            address: vec![0x01],
            value: crate::value::Value::from_coin(Coin(4_000_000)),
        };
        mary.utxo_state = utxo_insert(&mary.utxo_state, tx_in.clone(), tx_out.clone());
        let alonzo = translate_mary_to_alonzo(&mary).unwrap();
        assert_eq!(alonzo.utxo_state.len(), 1);
    }

    #[test]
    fn mary_to_alonzo_rejects_non_mary() {
        let state = make_shelley_state();
        assert!(translate_mary_to_alonzo(&state).is_err());
    }

    // -----------------------------------------------------------------------
    // Alonzo -> Babbage
    // -----------------------------------------------------------------------

    #[test]
    fn alonzo_to_babbage_basic() {
        let alonzo = make_alonzo_state();
        let babbage = translate_alonzo_to_babbage(&alonzo).unwrap();
        assert_eq!(babbage.era, CardanoEra::Babbage);
        assert_eq!(babbage.epoch_state.epoch, alonzo.epoch_state.epoch);
    }

    #[test]
    fn alonzo_to_babbage_rejects_non_alonzo() {
        let state = make_mary_state();
        assert!(translate_alonzo_to_babbage(&state).is_err());
    }

    // -----------------------------------------------------------------------
    // Babbage -> Conway
    // -----------------------------------------------------------------------

    #[test]
    fn babbage_to_conway_basic() {
        let babbage = make_babbage_state();
        let conway = translate_babbage_to_conway(&babbage).unwrap();
        assert_eq!(conway.era, CardanoEra::Conway);
        assert_eq!(conway.epoch_state.epoch, babbage.epoch_state.epoch);
    }

    #[test]
    fn babbage_to_conway_rejects_non_babbage() {
        let state = make_alonzo_state();
        assert!(translate_babbage_to_conway(&state).is_err());
    }

    // -----------------------------------------------------------------------
    // translate_era dispatch
    // -----------------------------------------------------------------------

    #[test]
    fn translate_era_dispatches_correctly() {
        let byron = make_byron_state();
        assert_eq!(translate_era(&byron, CardanoEra::Shelley).unwrap().era, CardanoEra::Shelley);

        let shelley = make_shelley_state();
        assert_eq!(translate_era(&shelley, CardanoEra::Allegra).unwrap().era, CardanoEra::Allegra);

        let allegra = make_allegra_state();
        assert_eq!(translate_era(&allegra, CardanoEra::Mary).unwrap().era, CardanoEra::Mary);

        let mary = make_mary_state();
        assert_eq!(translate_era(&mary, CardanoEra::Alonzo).unwrap().era, CardanoEra::Alonzo);

        let alonzo = make_alonzo_state();
        assert_eq!(translate_era(&alonzo, CardanoEra::Babbage).unwrap().era, CardanoEra::Babbage);

        let babbage = make_babbage_state();
        assert_eq!(translate_era(&babbage, CardanoEra::Conway).unwrap().era, CardanoEra::Conway);
    }

    #[test]
    fn translate_era_rejects_invalid_pairs() {
        let shelley = make_shelley_state();
        // Can't go backwards
        assert!(translate_era(&shelley, CardanoEra::ByronRegular).is_err());
        // Can't skip eras
        assert!(translate_era(&shelley, CardanoEra::Mary).is_err());
    }

    // -----------------------------------------------------------------------
    // Full chain
    // -----------------------------------------------------------------------

    #[test]
    fn full_translation_chain_byron_through_conway() {
        let byron = make_byron_state();
        let shelley = translate_byron_to_shelley(&byron).unwrap();
        let allegra = translate_shelley_to_allegra(&shelley).unwrap();
        let mary = translate_allegra_to_mary(&allegra).unwrap();
        let alonzo = translate_mary_to_alonzo(&mary).unwrap();
        let babbage = translate_alonzo_to_babbage(&alonzo).unwrap();
        let conway = translate_babbage_to_conway(&babbage).unwrap();

        assert_eq!(conway.era, CardanoEra::Conway);
        assert_eq!(conway.epoch_state.epoch, byron.epoch_state.epoch);
    }

    #[test]
    fn full_chain_via_dispatch() {
        let mut state = make_byron_state();
        for target in [
            CardanoEra::Shelley,
            CardanoEra::Allegra,
            CardanoEra::Mary,
            CardanoEra::Alonzo,
            CardanoEra::Babbage,
            CardanoEra::Conway,
        ] {
            state = translate_era(&state, target).unwrap();
        }
        assert_eq!(state.era, CardanoEra::Conway);
    }

    #[test]
    fn translation_chain_is_deterministic() {
        let byron = make_byron_state();

        let chain1 = translate_era(
            &translate_era(
                &translate_era(
                    &translate_era(
                        &translate_era(
                            &translate_era(&byron, CardanoEra::Shelley).unwrap(),
                            CardanoEra::Allegra,
                        ).unwrap(),
                        CardanoEra::Mary,
                    ).unwrap(),
                    CardanoEra::Alonzo,
                ).unwrap(),
                CardanoEra::Babbage,
            ).unwrap(),
            CardanoEra::Conway,
        ).unwrap();

        let chain2 = translate_era(
            &translate_era(
                &translate_era(
                    &translate_era(
                        &translate_era(
                            &translate_era(&byron, CardanoEra::Shelley).unwrap(),
                            CardanoEra::Allegra,
                        ).unwrap(),
                        CardanoEra::Mary,
                    ).unwrap(),
                    CardanoEra::Alonzo,
                ).unwrap(),
                CardanoEra::Babbage,
            ).unwrap(),
            CardanoEra::Conway,
        ).unwrap();

        assert_eq!(chain1, chain2);
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
