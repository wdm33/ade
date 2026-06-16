//! MEM-OPT-UTXO-DISK S2b: the fixed-width redb key for the on-disk UTxO anchor.
//!
//! `key = txid[32] || BE-u32(index)` (36 bytes). This is PROJECT-INTERNAL STORAGE
//! CANONICAL, **not** a Cardano protocol encoding — it is never used for hashing,
//! tx IDs, block IDs, signatures, or replay fingerprints. It exists ONLY to force
//! redb's byte-sorted iteration order to equal canonical `TxIn` order (`DC-MEM-06`),
//! mechanically:
//!
//! > redb byte order == txid lexicographic order, then index numeric order
//!
//! which is exactly `TxIn::cmp` (derived `Ord` over `{ tx_hash, index }`). A
//! fixed-width key avoids the CBOR array-prefix / integer-width effects that can
//! flip byte ordering vs numeric ordering for larger indices. The `index` field of
//! a `TxIn` is a `u16`; it is zero-extended into the BE-u32 slot (order-preserving,
//! future-proof), and decode rejects any key whose index escapes the `u16` domain.

#![allow(dead_code)] // wired into the redb UTxO anchor in the next S2b step

use ade_types::tx::TxIn;
use ade_types::Hash32;

/// The fixed-width key length: 32-byte txid ++ 4-byte big-endian index.
pub(crate) const UTXO_KEY_LEN: usize = 36;

/// Encode a `TxIn` as the fixed-width storage key (`txid[32] || BE-u32(index)`).
pub(crate) fn encode_utxo_key(tx_in: &TxIn) -> [u8; UTXO_KEY_LEN] {
    let mut key = [0u8; UTXO_KEY_LEN];
    key[..32].copy_from_slice(&tx_in.tx_hash.0);
    key[32..].copy_from_slice(&(tx_in.index as u32).to_be_bytes());
    key
}

/// Decode a fixed-width storage key back to a `TxIn`. Fails closed on a wrong
/// length or an index outside the `u16` `TxIn` domain — the key is never trusted
/// blindly (it is read back from disk).
pub(crate) fn decode_utxo_key(bytes: &[u8]) -> Result<TxIn, UtxoKeyError> {
    if bytes.len() != UTXO_KEY_LEN {
        return Err(UtxoKeyError::WrongLength { found: bytes.len() });
    }
    let mut txid = [0u8; 32];
    txid.copy_from_slice(&bytes[..32]);
    let mut idx = [0u8; 4];
    idx.copy_from_slice(&bytes[32..]);
    let index_u32 = u32::from_be_bytes(idx);
    let index = u16::try_from(index_u32).map_err(|_| UtxoKeyError::IndexOutOfRange { index_u32 })?;
    Ok(TxIn {
        tx_hash: Hash32(txid),
        index,
    })
}

/// A fixed-width key that does not decode to a valid `TxIn` (deterministic reject).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum UtxoKeyError {
    /// The key was not exactly [`UTXO_KEY_LEN`] bytes.
    WrongLength { found: usize },
    /// The BE-u32 index field exceeded the `u16` `TxIn` domain.
    IndexOutOfRange { index_u32: u32 },
}

impl std::fmt::Display for UtxoKeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UtxoKeyError::WrongLength { found } => {
                write!(f, "utxo key must be {UTXO_KEY_LEN} bytes, found {found}")
            }
            UtxoKeyError::IndexOutOfRange { index_u32 } => {
                write!(f, "utxo key index {index_u32} exceeds the u16 TxIn domain")
            }
        }
    }
}
impl std::error::Error for UtxoKeyError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn txin(h: [u8; 32], i: u16) -> TxIn {
        TxIn {
            tx_hash: Hash32(h),
            index: i,
        }
    }

    /// The load-bearing DC-MEM-06 proof: the fixed-width key's byte order equals
    /// `TxIn::cmp` for every pair, INCLUDING the index numeric boundary (255 -> 256,
    /// etc.), which is exactly where a naive/CBOR encoding can flip byte-order vs
    /// numeric-order. Indexes are the `u16` `TxIn` domain (the BE-u32 slot zero-
    /// extends them).
    #[test]
    fn fixed_width_key_order_matches_txin_ord() {
        let txids = [[0x00; 32], [0xAA; 32], [0xBB; 32], [0xFF; 32]];
        let idxs = [0u16, 1, 23, 24, 255, 256, u16::MAX - 1, u16::MAX];
        let mut txins = Vec::new();
        for &h in &txids {
            for &i in &idxs {
                txins.push(txin(h, i));
            }
        }
        // every ordered pair: key byte-order agrees with TxIn::cmp.
        for x in &txins {
            for y in &txins {
                assert_eq!(
                    encode_utxo_key(x).cmp(&encode_utxo_key(y)),
                    x.cmp(y),
                    "key order must equal TxIn::cmp for {x:?} vs {y:?}"
                );
            }
        }
        // sorting by key == sorting by TxIn (the whole-set ordering invariant).
        let mut by_key = txins.clone();
        by_key.sort_by(|a, b| encode_utxo_key(a).cmp(&encode_utxo_key(b)));
        let mut by_txin = txins.clone();
        by_txin.sort();
        assert_eq!(by_key, by_txin, "key sort order must equal TxIn sort order");
    }

    #[test]
    fn key_roundtrip_is_identity() {
        for &i in &[0u16, 1, 255, 256, 24, u16::MAX] {
            let t = txin([0x42; 32], i);
            assert_eq!(decode_utxo_key(&encode_utxo_key(&t)), Ok(t));
        }
    }

    #[test]
    fn malformed_key_length_rejected_deterministically() {
        assert_eq!(
            decode_utxo_key(&[]),
            Err(UtxoKeyError::WrongLength { found: 0 })
        );
        assert_eq!(
            decode_utxo_key(&[0u8; UTXO_KEY_LEN - 1]),
            Err(UtxoKeyError::WrongLength {
                found: UTXO_KEY_LEN - 1
            })
        );
        assert_eq!(
            decode_utxo_key(&[0u8; UTXO_KEY_LEN + 1]),
            Err(UtxoKeyError::WrongLength {
                found: UTXO_KEY_LEN + 1
            })
        );
    }

    #[test]
    fn index_out_of_u16_domain_rejected() {
        // A key whose BE-u32 index field exceeds u16::MAX (top bytes nonzero) is not
        // a valid TxIn key -- reject deterministically rather than truncate.
        let mut key = [0u8; UTXO_KEY_LEN];
        key[32..].copy_from_slice(&70_000u32.to_be_bytes());
        assert_eq!(
            decode_utxo_key(&key),
            Err(UtxoKeyError::IndexOutOfRange { index_u32: 70_000 })
        );
    }
}
