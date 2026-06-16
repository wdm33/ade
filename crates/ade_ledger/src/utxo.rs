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
use crate::utxo_overlay::OverlayUtxo;
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

    /// The raw address bytes of this output. For Shelley+ outputs these
    /// are the on-wire address bytes whose header byte classifies the
    /// payment credential (key-hash vs script-hash). Byron addresses
    /// return their raw legacy bytes (no Shelley payment credential).
    pub fn address_bytes(&self) -> &[u8] {
        match self {
            TxOut::Byron { address, .. } => address.as_bytes(),
            TxOut::ShelleyMary { address, .. } => address,
            TxOut::AlonzoPlus { address, .. } => address,
        }
    }
}

/// Minimal UTxO state — an overlay-backed store (MEM-OPT-UTXO-DISK S2a): an
/// `Arc`-shared anchor + a bounded overlay, so a clone is O(overlay) and a mutation
/// is an overlay append (no full-map clone). Iteration is canonical `TxIn` order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UTxOState {
    pub utxos: OverlayUtxo,
}

impl UTxOState {
    pub fn new() -> Self {
        UTxOState {
            utxos: OverlayUtxo::new(),
        }
    }

    /// Build from a full UTxO map (seed import / snapshot decode): the map becomes
    /// the shared anchor; the overlay starts empty.
    pub fn from_map(utxos: BTreeMap<TxIn, TxOut>) -> Self {
        UTxOState {
            utxos: OverlayUtxo::from_map(utxos),
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

/// Membership view over a UTxO set — the minimum an input-presence check needs
/// (no value materialization). Implemented for the production stores AND any
/// `BTreeMap<TxIn, V>`, so a test fixture can use a set-like `BTreeMap<TxIn, ()>`
/// for pure membership while production resolves through the on-disk-capable seam.
pub trait UtxoMembership {
    /// `true` iff a live entry exists for `tx_in`.
    fn contains_key(&self, tx_in: &TxIn) -> bool;
}

/// MEM-OPT-UTXO-DISK S1: the seam for a swappable UTxO backend. The authoritative
/// lookup returns an OWNED value, never a borrow into storage, so a later on-disk
/// backend (S2) can resolve inputs without leaking storage lifetimes into the
/// validity rules. Every `UtxoStore` is a [`UtxoMembership`] — membership is
/// answerable without materializing the value.
pub trait UtxoStore: UtxoMembership {
    /// Resolve an input to its output, BY VALUE. `None` if absent.
    fn get(&self, tx_in: &TxIn) -> Option<TxOut>;
}

impl<V> UtxoMembership for BTreeMap<TxIn, V> {
    fn contains_key(&self, tx_in: &TxIn) -> bool {
        BTreeMap::contains_key(self, tx_in)
    }
}
impl UtxoMembership for OverlayUtxo {
    fn contains_key(&self, tx_in: &TxIn) -> bool {
        OverlayUtxo::contains_key(self, tx_in)
    }
}
impl UtxoMembership for UTxOState {
    fn contains_key(&self, tx_in: &TxIn) -> bool {
        self.utxos.contains_key(tx_in)
    }
}

impl UtxoStore for UTxOState {
    fn get(&self, tx_in: &TxIn) -> Option<TxOut> {
        self.utxos.get(tx_in)
    }
}

impl UtxoStore for OverlayUtxo {
    fn get(&self, tx_in: &TxIn) -> Option<TxOut> {
        OverlayUtxo::get(self, tx_in)
    }
}

/// The in-memory map is also a `UtxoStore` — lets test fixtures keep building plain
/// `BTreeMap`s while the validation chain resolves through the abstract seam.
impl UtxoStore for BTreeMap<TxIn, TxOut> {
    fn get(&self, tx_in: &TxIn) -> Option<TxOut> {
        BTreeMap::get(self, tx_in).cloned()
    }
}

/// Insert a UTxO — pure, returns new state. The clone is O(overlay) (the anchor
/// `Arc` is shared) and the insert is an overlay append (O(1)) — no full-map clone
/// (MEM-OPT-UTXO-DISK S2a).
pub fn utxo_insert(state: &UTxOState, tx_in: TxIn, tx_out: TxOut) -> UTxOState {
    let mut new_store = state.utxos.clone();
    new_store.insert(tx_in, tx_out);
    UTxOState { utxos: new_store }
}

/// Delete a UTxO — returns new state + the consumed output, or InputNotFoundError.
/// The clone is O(overlay) and the delete is an overlay tombstone (O(1)) — no
/// full-map clone (MEM-OPT-UTXO-DISK S2a). The structured `InputNotFound` shape and
/// the resolved value are byte-identical to the BTreeMap model.
pub fn utxo_delete(
    state: &UTxOState,
    tx_in: &TxIn,
) -> Result<(UTxOState, TxOut), LedgerError> {
    let mut new_store = state.utxos.clone();
    let tx_out = new_store.remove(tx_in).ok_or_else(|| {
        LedgerError::InputNotFound(InputNotFoundError {
            tx_in: tx_in.clone(),
        })
    })?;

    Ok((UTxOState { utxos: new_store }, tx_out))
}

/// Lookup a UTxO — no mutation. Returns an OWNED value (S1: the swappable-backend
/// interface; an on-disk backend cannot hand out a borrow into storage). The clone
/// is a single `TxOut`, never a map clone; the resolved VALUE is identical, so no
/// verdict / fingerprint / failure-shape change.
pub fn utxo_lookup(state: &UTxOState, tx_in: &TxIn) -> Option<TxOut> {
    state.get(tx_in)
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
        assert_eq!(utxo_lookup(&state2, &tx_in), Some(tx_out));
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

    #[test]
    fn owned_lookup_returns_stored_value_and_does_not_mutate() {
        // MEM-OPT-UTXO-DISK S1: the owned interface returns a value EQUAL to the
        // stored entry (so every resolved output feeding validation + the
        // fingerprint is identical to the borrow it replaces), and a lookup never
        // mutates the store.
        let tx_in = make_tx_in(0x77, 3);
        let tx_out = make_byron_out(2_500_000);
        let state = utxo_insert(&UTxOState::new(), tx_in.clone(), tx_out.clone());
        let before = state.clone();
        assert_eq!(utxo_lookup(&state, &tx_in), Some(tx_out.clone()));
        assert_eq!(UtxoStore::get(&state, &tx_in), Some(tx_out));
        assert_eq!(state, before, "a lookup must not mutate the store");
        assert_eq!(utxo_lookup(&state, &make_tx_in(0x99, 0)), None);
    }
}
