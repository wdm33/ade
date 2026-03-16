// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use crate::error::CodecError;
use ade_types::address::Address;

/// Named decode chokepoint for Cardano addresses.
///
/// Classifies the address by header byte type and returns the
/// appropriate variant. The raw bytes are preserved exactly.
pub fn decode_address(data: &[u8]) -> Result<Address, CodecError> {
    if data.is_empty() {
        return Err(CodecError::UnexpectedEof {
            offset: 0,
            needed: 1,
        });
    }

    let header = data[0];
    let addr_type = header >> 4;

    match addr_type {
        // Base addresses: types 0-3
        0..=3 => Ok(Address::Base(data.to_vec())),
        // Pointer addresses: types 4-5
        4 | 5 => Ok(Address::Pointer(data.to_vec())),
        // Enterprise addresses: types 6-7
        6 | 7 => Ok(Address::Enterprise(data.to_vec())),
        // Byron/Bootstrap: type 8
        8 => Ok(Address::Byron(data.to_vec())),
        // Reward addresses: types 14-15
        14 | 15 => Ok(Address::Reward(data.to_vec())),
        // Unknown
        _ => Err(CodecError::InvalidCborStructure {
            offset: 0,
            detail: "unknown address type in header byte",
        }),
    }
}

/// Encode an address back to bytes. Identity operation — returns stored bytes.
pub fn encode_address(addr: &Address) -> &[u8] {
    addr.as_bytes()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn decode_base_address() {
        // Type 0 address: header byte 0x01 (type 0, network 1)
        let data = vec![0x01; 57]; // 1 + 28 + 28 = 57 bytes for base address
        let addr = decode_address(&data).unwrap();
        assert!(matches!(addr, Address::Base(_)));
        assert_eq!(addr.as_bytes(), &data[..]);
    }

    #[test]
    fn decode_enterprise_address() {
        // Type 6 address: header byte 0x61 (type 6, network 1)
        let mut data = vec![0x61];
        data.extend_from_slice(&[0u8; 28]);
        let addr = decode_address(&data).unwrap();
        assert!(matches!(addr, Address::Enterprise(_)));
    }

    #[test]
    fn decode_reward_address() {
        // Type 14 address: header byte 0xe1 (type 14, network 1)
        let mut data = vec![0xe1];
        data.extend_from_slice(&[0u8; 28]);
        let addr = decode_address(&data).unwrap();
        assert!(matches!(addr, Address::Reward(_)));
    }

    #[test]
    fn decode_byron_address() {
        // Type 8 address: header byte 0x82 (in the tagged CBOR context)
        let data = vec![0x82, 0xd8, 0x18]; // Byron addresses start with 0x82 in raw CBOR
        let addr = decode_address(&data).unwrap();
        assert!(matches!(addr, Address::Byron(_)));
    }

    #[test]
    fn decode_empty_returns_error() {
        assert!(decode_address(&[]).is_err());
    }

    #[test]
    fn round_trip_identity() {
        let data = vec![0x01; 57];
        let addr = decode_address(&data).unwrap();
        assert_eq!(encode_address(&addr), &data[..]);
    }

    #[test]
    fn unknown_type_rejected() {
        // Type 9 is undefined
        let result = decode_address(&[0x91]);
        assert!(result.is_err());
    }
}
