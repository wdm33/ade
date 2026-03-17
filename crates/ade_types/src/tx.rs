// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use crate::Hash32;

/// Transaction input reference — identifies a specific output from a previous transaction.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TxIn {
    pub tx_hash: Hash32,
    pub index: u16,
}

/// Lovelace coin amount — smallest unit of ADA.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Coin(pub u64);

impl Coin {
    pub const ZERO: Coin = Coin(0);

    /// Checked addition returning None on overflow.
    pub fn checked_add(self, other: Coin) -> Option<Coin> {
        self.0.checked_add(other.0).map(Coin)
    }

    /// Checked subtraction returning None on underflow.
    pub fn checked_sub(self, other: Coin) -> Option<Coin> {
        self.0.checked_sub(other.0).map(Coin)
    }
}

impl core::fmt::Display for Coin {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 28-byte pool identifier (distinct from Hash28 for type safety).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PoolId(pub crate::Hash28);

impl core::fmt::Display for PoolId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coin_checked_add() {
        assert_eq!(Coin(100).checked_add(Coin(200)), Some(Coin(300)));
        assert_eq!(Coin(u64::MAX).checked_add(Coin(1)), None);
    }

    #[test]
    fn coin_checked_sub() {
        assert_eq!(Coin(300).checked_sub(Coin(100)), Some(Coin(200)));
        assert_eq!(Coin(0).checked_sub(Coin(1)), None);
    }

    #[test]
    fn coin_display() {
        assert_eq!(format!("{}", Coin(1_000_000)), "1000000");
    }

    #[test]
    fn tx_in_ordering() {
        let a = TxIn {
            tx_hash: Hash32([0u8; 32]),
            index: 0,
        };
        let b = TxIn {
            tx_hash: Hash32([0u8; 32]),
            index: 1,
        };
        assert!(a < b);
    }
}
