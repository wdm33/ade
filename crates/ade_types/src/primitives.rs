// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

/// Slot number within the blockchain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SlotNo(pub u64);

/// Block number (height).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BlockNo(pub u64);

/// Epoch number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EpochNo(pub u64);

/// 28-byte hash (e.g., Blake2b-224 for addresses, credential hashes).
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Hash28(pub [u8; 28]);

/// 32-byte hash (e.g., Blake2b-256 for block hashes, tx IDs).
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Hash32(pub [u8; 32]);

impl core::fmt::Debug for Hash28 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Hash28(")?;
        for byte in &self.0 {
            write!(f, "{byte:02x}")?;
        }
        write!(f, ")")
    }
}

impl core::fmt::Debug for Hash32 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Hash32(")?;
        for byte in &self.0 {
            write!(f, "{byte:02x}")?;
        }
        write!(f, ")")
    }
}

impl core::fmt::Display for Hash28 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for byte in &self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl core::fmt::Display for Hash32 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for byte in &self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_no_ordering() {
        assert!(SlotNo(0) < SlotNo(1));
        assert_eq!(SlotNo(42), SlotNo(42));
    }

    #[test]
    fn hash32_debug_hex() {
        let h = Hash32([0xab; 32]);
        let s = format!("{h:?}");
        assert!(s.starts_with("Hash32("));
        assert!(s.contains("abababab"));
    }

    #[test]
    fn hash28_debug_hex() {
        let h = Hash28([0xcd; 28]);
        let s = format!("{h:?}");
        assert!(s.starts_with("Hash28("));
        assert!(s.contains("cdcdcdcd"));
    }

    #[test]
    fn hash32_display_hex() {
        let mut bytes = [0u8; 32];
        bytes[0] = 0xde;
        bytes[31] = 0xad;
        let h = Hash32(bytes);
        let s = format!("{h}");
        assert_eq!(s.len(), 64);
        assert!(s.starts_with("de"));
        assert!(s.ends_with("ad"));
    }

    #[test]
    fn hash28_display_hex() {
        let mut bytes = [0u8; 28];
        bytes[0] = 0xbe;
        bytes[27] = 0xef;
        let h = Hash28(bytes);
        let s = format!("{h}");
        assert_eq!(s.len(), 56);
        assert!(s.starts_with("be"));
        assert!(s.ends_with("ef"));
    }
}
