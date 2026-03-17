// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use crate::address::Address;
use crate::tx::Coin;
use crate::Hash32;

/// Byron transaction body — decoded from the tx_payload in the block body.
///
/// Byron tx bodies are CBOR arrays: `[inputs, outputs, attributes]`.
/// Attributes are preserved as opaque bytes for round-trip fidelity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ByronTxBody {
    pub inputs: Vec<ByronTxIn>,
    pub outputs: Vec<ByronTxOut>,
    /// Transaction attributes — opaque CBOR, preserved for round-trip.
    pub attributes: Vec<u8>,
}

/// Byron transaction input — references a previous transaction output.
///
/// Wire format: `[tag(0), tag(24, cbor_bytes([tx_hash, index]))]`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ByronTxIn {
    pub tx_hash: Hash32,
    pub index: u32,
}

/// Byron transaction output — address + coin.
///
/// Wire format: `[address, coin]`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ByronTxOut {
    pub address: Address,
    pub coin: Coin,
}

/// Byron witness — public key witness with extended verification key + signature.
///
/// Wire format: `[type_tag, [xvk, signature]]`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ByronWitness {
    /// Witness type tag (0 = PkWitness, 2 = RedeemWitness).
    pub witness_type: u8,
    /// Extended verification key (64 bytes for PkWitness).
    pub xvk: Vec<u8>,
    /// Ed25519 signature (64 bytes).
    pub signature: Vec<u8>,
}

/// A full Byron transaction as it appears in the block body tx_payload.
///
/// Wire format: `[[tx_body, [witnesses]]]` wrapped in the tx_payload array.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ByronTx {
    pub body: ByronTxBody,
    pub witnesses: Vec<ByronWitness>,
}
