// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use ade_types::tx::Coin;

/// Protocol parameters — initially minimal (fee coefficients for Byron/Shelley).
///
/// Expanded in S-16 with full Shelley–Mary parameter set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolParameters {
    /// Fee coefficient 'a' (per-byte fee, in lovelace).
    pub min_fee_a: Coin,
    /// Fee constant 'b' (fixed fee, in lovelace).
    pub min_fee_b: Coin,
    /// Minimum UTxO value (in lovelace).
    pub min_utxo_value: Coin,
    /// Maximum transaction size in bytes.
    pub max_tx_size: u32,
}

impl Default for ProtocolParameters {
    fn default() -> Self {
        // Byron mainnet defaults
        ProtocolParameters {
            min_fee_a: Coin(43_946_000_000), // Byron: 43946000000 per tx-byte (scaled)
            min_fee_b: Coin(155_381_000_000_000), // Byron: 155381000000000 fixed
            min_utxo_value: Coin(0),
            max_tx_size: 16_384,
        }
    }
}
