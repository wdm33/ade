// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use std::collections::BTreeMap;
use ade_types::address::Address;
use ade_types::tx::{Coin, TxIn};
use crate::error::{DuplicateInputError, InputNotFoundError, LedgerError};
use crate::value::Value;

/// Era-polymorphic transaction output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TxOut {
    /// Byron output: address + coin only.
    Byron { address: Address, coin: Coin },
    /// Shelley through Mary output: address + value (coin + optional multi-asset).
    ShelleyMary { address: Vec<u8>, value: Value },
    /// Alonzo/Babbage/Conway output with byte-preserved wire form.
    ///
    /// `raw` holds the exact CBOR slice lifted from the tx body (array
    /// form in Alonzo, array or map form in Babbage+, including any
    /// datum_hash / datum_option / script_ref / multi_asset fields).
    /// `address` and `coin` are extracted at construction for O(1)
    /// access by existing match arms.
    ///
    /// Required by `ade_plutus` evaluation: aiken's ScriptContext
    /// construction needs the full output CBOR (not a reconstruction),
    /// otherwise scripts that read datum hashes / inline datums / script
    /// refs from their inputs fail spuriously.
    AlonzoPlus { raw: Vec<u8>, address: Vec<u8>, coin: Coin },
}

impl TxOut {
    /// Extract the coin amount from any era's output.
    pub fn coin(&self) -> Coin {
        match self {
            TxOut::Byron { coin, .. } => *coin,
            TxOut::ShelleyMary { value, .. } => value.coin,
            TxOut::AlonzoPlus { coin, .. } => *coin,
        }
    }
}

/// Minimal UTxO state — deterministic BTreeMap for ordered iteration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UTxOState {
    pub utxos: BTreeMap<TxIn, TxOut>,
}

impl UTxOState {
    pub fn new() -> Self {
        UTxOState {
            utxos: BTreeMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.utxos.len()
    }

    pub fn is_empty(&self) -> bool {
        self.utxos.is_empty()
    }
}

impl Default for UTxOState {
    fn default() -> Self {
        Self::new()
    }
}

/// Insert a UTxO — pure, returns new state.
pub fn utxo_insert(state: &UTxOState, tx_in: TxIn, tx_out: TxOut) -> UTxOState {
    let mut new_utxos = state.utxos.clone();
    new_utxos.insert(tx_in, tx_out);
    UTxOState { utxos: new_utxos }
}

/// Delete a UTxO — returns new state + the consumed output, or InputNotFoundError.
pub fn utxo_delete(
    state: &UTxOState,
    tx_in: &TxIn,
) -> Result<(UTxOState, TxOut), LedgerError> {
    let tx_out = state
        .utxos
        .get(tx_in)
        .ok_or_else(|| {
            LedgerError::InputNotFound(InputNotFoundError {
                tx_in: tx_in.clone(),
            })
        })?
        .clone();

    let mut new_utxos = state.utxos.clone();
    new_utxos.remove(tx_in);

    Ok((UTxOState { utxos: new_utxos }, tx_out))
}

/// Lookup a UTxO — no mutation.
pub fn utxo_lookup<'a>(state: &'a UTxOState, tx_in: &TxIn) -> Option<&'a TxOut> {
    state.utxos.get(tx_in)
}

/// Check for duplicate inputs in a list.
pub fn check_duplicate_inputs(inputs: &[TxIn]) -> Result<(), LedgerError> {
    let mut seen = std::collections::BTreeSet::new();
    for input in inputs {
        if !seen.insert(input) {
            return Err(LedgerError::DuplicateInput(DuplicateInputError {
                tx_in: input.clone(),
            }));
        }
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use ade_types::Hash32;

    fn make_tx_in(hash_byte: u8, index: u16) -> TxIn {
        TxIn {
            tx_hash: Hash32([hash_byte; 32]),
            index,
        }
    }

    fn make_byron_out(coin: u64) -> TxOut {
        TxOut::Byron {
            address: Address::Byron(vec![0x01, 0x02]),
            coin: Coin(coin),
        }
    }

    #[test]
    fn insert_then_lookup() {
        let state = UTxOState::new();
        let tx_in = make_tx_in(0xaa, 0);
        let tx_out = make_byron_out(1_000_000);

        let state2 = utxo_insert(&state, tx_in.clone(), tx_out.clone());
        assert_eq!(utxo_lookup(&state2, &tx_in), Some(&tx_out));
    }

    #[test]
    fn delete_on_absent_returns_error() {
        let state = UTxOState::new();
        let tx_in = make_tx_in(0xbb, 0);
        let result = utxo_delete(&state, &tx_in);
        assert!(matches!(result, Err(LedgerError::InputNotFound(_))));
    }

    #[test]
    fn delete_then_lookup_returns_none() {
        let state = UTxOState::new();
        let tx_in = make_tx_in(0xcc, 0);
        let tx_out = make_byron_out(500_000);

        let state2 = utxo_insert(&state, tx_in.clone(), tx_out);
        let (state3, consumed) = utxo_delete(&state2, &tx_in).unwrap();
        assert_eq!(consumed.coin(), Coin(500_000));
        assert_eq!(utxo_lookup(&state3, &tx_in), None);
    }

    #[test]
    fn duplicate_inputs_detected() {
        let a = make_tx_in(0xdd, 0);
        let inputs = vec![a.clone(), a];
        let result = check_duplicate_inputs(&inputs);
        assert!(matches!(result, Err(LedgerError::DuplicateInput(_))));
    }

    #[test]
    fn no_duplicate_inputs_passes() {
        let a = make_tx_in(0xee, 0);
        let b = make_tx_in(0xee, 1);
        let inputs = vec![a, b];
        assert!(check_duplicate_inputs(&inputs).is_ok());
    }

    #[test]
    fn utxo_state_deterministic() {
        // Same insertions in same order → same BTreeMap state
        let state = UTxOState::new();
        let s1 = utxo_insert(&state, make_tx_in(0x01, 0), make_byron_out(100));
        let s1 = utxo_insert(&s1, make_tx_in(0x02, 0), make_byron_out(200));

        let state2 = UTxOState::new();
        let s2 = utxo_insert(&state2, make_tx_in(0x01, 0), make_byron_out(100));
        let s2 = utxo_insert(&s2, make_tx_in(0x02, 0), make_byron_out(200));

        assert_eq!(s1, s2);
    }
}
