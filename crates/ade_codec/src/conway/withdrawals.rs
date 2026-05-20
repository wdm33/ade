// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use std::collections::BTreeMap;

use crate::cbor::{self, ContainerEncoding};
use crate::error::CodecError;
use ade_types::tx::{Coin, RewardAccount};

/// Decode a Conway transaction withdrawals field (tx body key 5) into a
/// deterministic [`BTreeMap`] keyed by [`RewardAccount`].
///
/// This decoder is a closed grammar with no silent-skip arm: every malformed
/// shape rejects with a structured [`CodecError`], never a partial map. The
/// field is a definite-length CBOR map of `bytes => uint`; an indefinite map is
/// rejected. Each key must be exactly 29 bytes. A coin value exceeding
/// `u64::MAX` rejects rather than truncating. Duplicate keys reject (never
/// last-wins). Trailing bytes after the map reject.
///
/// Strict canonical key ordering is not enforced here: the tx body hash already
/// binds the wire byte order, and the `BTreeMap` normalizes iteration order for
/// summation. Duplicate keys are still rejected because they make the map
/// ambiguous.
pub fn decode_withdrawals(data: &[u8]) -> Result<BTreeMap<RewardAccount, Coin>, CodecError> {
    let mut offset = 0;
    let enc = cbor::read_map_header(data, &mut offset)?;
    let count = match enc {
        ContainerEncoding::Definite(n, _) => n,
        ContainerEncoding::Indefinite => {
            return Err(CodecError::InvalidCborStructure {
                offset: 0,
                detail: "withdrawals map must be definite-length",
            });
        }
    };

    let mut map = BTreeMap::new();
    for _ in 0..count {
        let key_offset = offset;
        let (key_bytes, _) = cbor::read_bytes(data, &mut offset)?;
        if key_bytes.len() != 29 {
            return Err(CodecError::InvalidLength {
                offset: key_offset,
                detail: "reward account must be 29 bytes",
            });
        }
        let mut account = [0u8; 29];
        account.copy_from_slice(&key_bytes);
        let account = RewardAccount(account);

        let (value, _) = cbor::read_uint(data, &mut offset)?;

        if map.insert(account, Coin(value)).is_some() {
            return Err(CodecError::DuplicateMapKey { offset: key_offset });
        }
    }

    if offset != data.len() {
        return Err(CodecError::TrailingBytes {
            consumed: offset,
            total: data.len(),
        });
    }

    Ok(map)
}

/// Exact sum of all withdrawal coin values, accumulated in `i128`.
///
/// Each value is at most `u64::MAX` and the entry count is bounded by tx size,
/// so the total cannot exceed `i128`. No float, no rounding, no saturation.
pub fn withdrawals_sum(map: &BTreeMap<RewardAccount, Coin>) -> i128 {
    map.values().map(|coin| i128::from(coin.0)).sum()
}
