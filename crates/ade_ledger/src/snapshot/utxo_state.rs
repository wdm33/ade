// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! BLUE `UTxOState` snapshot encoder/decoder (PHASE4-N-J S2).
//!
//! Wire shape:
//! ```text
//! map(N) [
//!   array(2)[bytes(32) tx_hash, uint index]  ->  encoded_tx_out
//! ]
//! ```
//!
//! `encoded_tx_out` is a tagged-array:
//! - Byron      (tag 0): array(3)[0, address_array, uint coin]
//! - ShelleyMary(tag 1): array(3)[1, bytes addr, array(2)[uint coin, encoded_multi_asset]]
//! - AlonzoPlus (tag 2): array(3)[2, bytes addr, array(2)[uint coin, bytes raw]]
//!
//! `address_array` (Byron-side) is array(2)[uint variant, bytes payload]
//! where variant ∈ 0..=4 maps to (Base, Pointer, Enterprise, Byron, Reward).
//!
//! `encoded_multi_asset` = map(P) of bytes(28) policy → map(A) of bytes asset_name → int.
//!
//! BTreeMap traversal everywhere — deterministic.

use std::collections::BTreeMap;

use ade_codec::cbor::{
    canonical_width, read_any_int, read_array_header, read_bytes, read_map_header,
    write_array_header, write_bytes_canonical, write_map_header, write_uint_canonical,
    ContainerEncoding, IntWidth, MAJOR_NEGATIVE,
};
use ade_codec::CodecError;
use ade_types::address::Address;
use ade_types::tx::{Coin, TxIn};
use ade_types::{Hash28, Hash32};

use crate::utxo::{TxOut, UTxOState};
use crate::value::{AssetName, MultiAsset, Value};

use super::error::{SnapshotDecodeError, StructuralReason};

pub fn encode_utxo_state(state: &UTxOState) -> Vec<u8> {
    let mut buf = Vec::new();
    write_map_header(
        &mut buf,
        ContainerEncoding::Definite(
            state.utxos.len() as u64,
            canonical_width(state.utxos.len() as u64),
        ),
    );
    for (tx_in, tx_out) in &state.utxos {
        write_tx_in(&mut buf, tx_in);
        write_tx_out(&mut buf, tx_out);
    }
    buf
}

pub fn decode_utxo_state(bytes: &[u8]) -> Result<UTxOState, SnapshotDecodeError> {
    let mut o = 0usize;
    let n = match read_map_header(bytes, &mut o).map_err(SnapshotDecodeError::Cbor)? {
        ContainerEncoding::Definite(n, _) => n,
        _ => {
            return Err(SnapshotDecodeError::Structural {
                reason: StructuralReason::ArrayLengthMismatch,
            })
        }
    };
    let mut utxos: BTreeMap<TxIn, TxOut> = BTreeMap::new();
    for _ in 0..n {
        let tx_in = read_tx_in(bytes, &mut o)?;
        let tx_out = read_tx_out(bytes, &mut o)?;
        utxos.insert(tx_in, tx_out);
    }
    Ok(UTxOState::from_map(utxos))
}

// ---------------------------------------------------------------------------
// TxIn
// ---------------------------------------------------------------------------

fn write_tx_in(buf: &mut Vec<u8>, tx_in: &TxIn) {
    write_array_header(buf, ContainerEncoding::Definite(2, IntWidth::Inline));
    write_bytes_canonical(buf, &tx_in.tx_hash.0);
    write_uint_canonical(buf, tx_in.index as u64);
}

fn read_tx_in(bytes: &[u8], o: &mut usize) -> Result<TxIn, SnapshotDecodeError> {
    expect_array(bytes, o, 2)?;
    let (h, _w) = read_bytes(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
    if h.len() != 32 {
        return Err(SnapshotDecodeError::Structural {
            reason: StructuralReason::Hash32LengthMismatch,
        });
    }
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&h);
    let (idx, _n, _w) = read_any_int(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
    Ok(TxIn {
        tx_hash: Hash32(hash),
        index: idx as u16,
    })
}

// ---------------------------------------------------------------------------
// TxOut
// ---------------------------------------------------------------------------

fn write_tx_out(buf: &mut Vec<u8>, tx_out: &TxOut) {
    write_array_header(buf, ContainerEncoding::Definite(3, IntWidth::Inline));
    match tx_out {
        TxOut::Byron { address, coin } => {
            write_uint_canonical(buf, 0);
            write_address(buf, address);
            write_uint_canonical(buf, coin.0);
        }
        TxOut::ShelleyMary { address, value } => {
            write_uint_canonical(buf, 1);
            write_bytes_canonical(buf, address);
            write_array_header(buf, ContainerEncoding::Definite(2, IntWidth::Inline));
            write_uint_canonical(buf, value.coin.0);
            write_multi_asset(buf, &value.multi_asset);
        }
        TxOut::AlonzoPlus { raw, address, coin } => {
            write_uint_canonical(buf, 2);
            write_bytes_canonical(buf, address);
            write_array_header(buf, ContainerEncoding::Definite(2, IntWidth::Inline));
            write_uint_canonical(buf, coin.0);
            write_bytes_canonical(buf, raw);
        }
    }
}

fn read_tx_out(bytes: &[u8], o: &mut usize) -> Result<TxOut, SnapshotDecodeError> {
    expect_array(bytes, o, 3)?;
    let (tag, _n, _w) = read_any_int(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
    match tag {
        0 => {
            let address = read_address(bytes, o)?;
            let (coin, _n, _w) = read_any_int(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
            Ok(TxOut::Byron {
                address,
                coin: Coin(coin),
            })
        }
        1 => {
            let (addr, _w) = read_bytes(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
            expect_array(bytes, o, 2)?;
            let (coin, _n, _w) = read_any_int(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
            let multi_asset = read_multi_asset(bytes, o)?;
            Ok(TxOut::ShelleyMary {
                address: addr,
                value: Value {
                    coin: Coin(coin),
                    multi_asset,
                },
            })
        }
        2 => {
            let (addr, _w) = read_bytes(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
            expect_array(bytes, o, 2)?;
            let (coin, _n, _w) = read_any_int(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
            let (raw, _w) = read_bytes(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
            Ok(TxOut::AlonzoPlus {
                raw,
                address: addr,
                coin: Coin(coin),
            })
        }
        _ => Err(SnapshotDecodeError::Structural {
            reason: StructuralReason::EraTagOutOfRange,
        }),
    }
}

// ---------------------------------------------------------------------------
// Address (Byron-only)
// ---------------------------------------------------------------------------

fn write_address(buf: &mut Vec<u8>, addr: &Address) {
    write_array_header(buf, ContainerEncoding::Definite(2, IntWidth::Inline));
    let (variant, bytes): (u64, &[u8]) = match addr {
        Address::Base(b) => (0, b),
        Address::Pointer(b) => (1, b),
        Address::Enterprise(b) => (2, b),
        Address::Byron(b) => (3, b),
        Address::Reward(b) => (4, b),
    };
    write_uint_canonical(buf, variant);
    write_bytes_canonical(buf, bytes);
}

fn read_address(bytes: &[u8], o: &mut usize) -> Result<Address, SnapshotDecodeError> {
    expect_array(bytes, o, 2)?;
    let (variant, _n, _w) = read_any_int(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
    let (b, _w) = read_bytes(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
    match variant {
        0 => Ok(Address::Base(b)),
        1 => Ok(Address::Pointer(b)),
        2 => Ok(Address::Enterprise(b)),
        3 => Ok(Address::Byron(b)),
        4 => Ok(Address::Reward(b)),
        _ => Err(SnapshotDecodeError::Structural {
            reason: StructuralReason::EraTagOutOfRange,
        }),
    }
}

// ---------------------------------------------------------------------------
// MultiAsset
// ---------------------------------------------------------------------------

fn write_multi_asset(buf: &mut Vec<u8>, ma: &MultiAsset) {
    write_map_header(
        buf,
        ContainerEncoding::Definite(ma.0.len() as u64, canonical_width(ma.0.len() as u64)),
    );
    for (policy, assets) in &ma.0 {
        write_bytes_canonical(buf, &policy.0);
        write_map_header(
            buf,
            ContainerEncoding::Definite(assets.len() as u64, canonical_width(assets.len() as u64)),
        );
        for (asset_name, qty) in assets {
            write_bytes_canonical(buf, &asset_name.0);
            write_int_i64(buf, *qty);
        }
    }
}

fn read_multi_asset(bytes: &[u8], o: &mut usize) -> Result<MultiAsset, SnapshotDecodeError> {
    let n_policies = match read_map_header(bytes, o).map_err(SnapshotDecodeError::Cbor)? {
        ContainerEncoding::Definite(n, _) => n,
        _ => {
            return Err(SnapshotDecodeError::Structural {
                reason: StructuralReason::ArrayLengthMismatch,
            })
        }
    };
    let mut ma: BTreeMap<Hash28, BTreeMap<AssetName, i64>> = BTreeMap::new();
    for _ in 0..n_policies {
        let (pol_bytes, _w) = read_bytes(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
        if pol_bytes.len() != 28 {
            return Err(SnapshotDecodeError::Structural {
                reason: StructuralReason::Hash28LengthMismatch,
            });
        }
        let mut pol = [0u8; 28];
        pol.copy_from_slice(&pol_bytes);
        let n_assets = match read_map_header(bytes, o).map_err(SnapshotDecodeError::Cbor)? {
            ContainerEncoding::Definite(n, _) => n,
            _ => {
                return Err(SnapshotDecodeError::Structural {
                    reason: StructuralReason::ArrayLengthMismatch,
                })
            }
        };
        let mut assets: BTreeMap<AssetName, i64> = BTreeMap::new();
        for _ in 0..n_assets {
            let (name_bytes, _w) = read_bytes(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
            let qty = read_int_i64(bytes, o)?;
            assets.insert(AssetName(name_bytes), qty);
        }
        ma.insert(Hash28(pol), assets);
    }
    Ok(MultiAsset(ma))
}

fn write_int_i64(buf: &mut Vec<u8>, v: i64) {
    if v >= 0 {
        write_uint_canonical(buf, v as u64);
    } else {
        // Negative integer: major type 1, encoded value = -1 - v.
        // Compute in i128 to handle i64::MIN safely.
        let positive: u64 = ((-1i128) - (v as i128)) as u64;
        let width = ade_codec::cbor::canonical_width(positive);
        ade_codec::cbor::write_argument(buf, MAJOR_NEGATIVE, positive, width);
    }
}

fn read_int_i64(bytes: &[u8], o: &mut usize) -> Result<i64, SnapshotDecodeError> {
    let (v, is_neg, _w) = read_any_int(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
    if is_neg {
        // signed value = -1 - encoded; compute in i128 to handle i64::MIN.
        let signed = ((-1i128) - (v as i128)) as i64;
        Ok(signed)
    } else {
        Ok(v as i64)
    }
}

// ---------------------------------------------------------------------------
// Common helpers
// ---------------------------------------------------------------------------

fn expect_array(bytes: &[u8], o: &mut usize, expected_len: u64) -> Result<(), SnapshotDecodeError> {
    let enc = read_array_header(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
    match enc {
        ContainerEncoding::Definite(n, _) if n == expected_len => Ok(()),
        _ => Err(SnapshotDecodeError::Structural {
            reason: StructuralReason::ArrayLengthMismatch,
        }),
    }
}

// Avoid "unused import" of CodecError; the decode path uses it via map_err.
#[allow(unused_imports)]
use CodecError as _CodecErrorUsed;

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    fn tx_in(b: u8, idx: u16) -> TxIn {
        TxIn {
            tx_hash: Hash32([b; 32]),
            index: idx,
        }
    }

    fn byron_out(b: u8, coin: u64) -> TxOut {
        TxOut::Byron {
            address: Address::Byron(vec![b; 16]),
            coin: Coin(coin),
        }
    }

    fn shelley_out(b: u8, coin: u64) -> TxOut {
        let mut ma = MultiAsset::new();
        ma.0.insert(Hash28([b; 28]), {
            let mut a = BTreeMap::new();
            a.insert(AssetName(vec![0x01, 0x02]), 42);
            a.insert(AssetName(vec![0xFF]), -7);
            a
        });
        TxOut::ShelleyMary {
            address: vec![b; 29],
            value: Value {
                coin: Coin(coin),
                multi_asset: ma,
            },
        }
    }

    fn alonzo_out(b: u8, coin: u64) -> TxOut {
        TxOut::AlonzoPlus {
            raw: vec![b, 0x82, 0x01, 0x02],
            address: vec![b; 29],
            coin: Coin(coin),
        }
    }

    fn make_state() -> UTxOState {
        let mut s = UTxOState::new();
        s.utxos.insert(tx_in(0x10, 0), byron_out(0x10, 1000));
        s.utxos.insert(tx_in(0x20, 1), shelley_out(0x20, 2000));
        s.utxos.insert(tx_in(0x30, 2), alonzo_out(0x30, 3000));
        s
    }

    #[test]
    fn utxo_state_round_trip_empty() {
        let s = UTxOState::new();
        let bytes = encode_utxo_state(&s);
        let decoded = decode_utxo_state(&bytes).expect("decode");
        assert_eq!(decoded, s);
    }

    #[test]
    fn utxo_state_round_trip_all_eras() {
        let s = make_state();
        let bytes = encode_utxo_state(&s);
        let decoded = decode_utxo_state(&bytes).expect("decode");
        assert_eq!(decoded, s);
    }

    #[test]
    fn utxo_state_encode_deterministic_across_runs() {
        let s = make_state();
        let a = encode_utxo_state(&s);
        let b = encode_utxo_state(&s);
        assert_eq!(a, b);
    }

    #[test]
    fn utxo_state_negative_multi_asset_quantity_round_trips() {
        let mut ma = MultiAsset::new();
        ma.0.insert(Hash28([0xAA; 28]), {
            let mut a = BTreeMap::new();
            a.insert(AssetName(vec![0x01]), i64::MIN);
            a.insert(AssetName(vec![0x02]), -1);
            a.insert(AssetName(vec![0x03]), 0);
            a.insert(AssetName(vec![0x04]), 1);
            a.insert(AssetName(vec![0x05]), i64::MAX);
            a
        });
        let tx_out = TxOut::ShelleyMary {
            address: vec![0xAA; 29],
            value: Value {
                coin: Coin(0),
                multi_asset: ma,
            },
        };
        let mut s = UTxOState::new();
        s.utxos.insert(tx_in(0xAA, 0), tx_out);
        let bytes = encode_utxo_state(&s);
        let decoded = decode_utxo_state(&bytes).expect("decode");
        assert_eq!(decoded, s);
    }

    #[test]
    fn utxo_state_iteration_order_is_btreemap() {
        // Insert in unsorted order; verify encoding mirrors sorted order
        // by encoding the same set twice with different insertion order
        // and asserting byte equality.
        let make_a = || {
            let mut s = UTxOState::new();
            s.utxos.insert(tx_in(0x30, 2), alonzo_out(0x30, 3000));
            s.utxos.insert(tx_in(0x10, 0), byron_out(0x10, 1000));
            s.utxos.insert(tx_in(0x20, 1), shelley_out(0x20, 2000));
            s
        };
        let make_b = || {
            let mut s = UTxOState::new();
            s.utxos.insert(tx_in(0x10, 0), byron_out(0x10, 1000));
            s.utxos.insert(tx_in(0x20, 1), shelley_out(0x20, 2000));
            s.utxos.insert(tx_in(0x30, 2), alonzo_out(0x30, 3000));
            s
        };
        let a = encode_utxo_state(&make_a());
        let b = encode_utxo_state(&make_b());
        assert_eq!(
            a, b,
            "encoder must depend only on BTreeMap content, not insert order"
        );
    }
}
