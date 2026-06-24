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
//! `encoded_multi_asset` = map(P) of bytes(28) policy → map(A) of bytes asset_name →
//! uint (the OUTPUT asset quantity, the non-negative Word64 domain). A quantity
//! ≤ i64::MAX encodes identically to the prior signed form; a quantity > i64::MAX
//! round-trips faithfully as a CBOR unsigned int.
//!
//! BTreeMap traversal everywhere — deterministic.

use std::collections::BTreeMap;

use ade_codec::cbor::{
    canonical_width, read_any_int, read_array_header, read_bytes, read_map_header,
    write_array_header, write_bytes_canonical, write_map_header, write_uint_canonical,
    ContainerEncoding, IntWidth,
};
use ade_codec::CodecError;
use ade_types::address::Address;
use ade_types::mary::value::OutputAssetQuantity;
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

/// Canonical single-`TxOut` byte form (the snapshot value encoding) — the storage
/// value for the on-disk UTxO anchor (MEM-OPT-UTXO-DISK S2b). Deterministic, and the
/// SAME bytes the snapshot encoder writes, so an anchor entry and a snapshot entry
/// for one output are byte-identical (hence fingerprint-identical).
pub fn encode_tx_out_canonical(tx_out: &TxOut) -> Vec<u8> {
    let mut buf = Vec::new();
    write_tx_out(&mut buf, tx_out);
    buf
}

/// Decode a single canonical `TxOut` (the inverse of [`encode_tx_out_canonical`]).
/// Fails closed on trailing bytes — an anchor value is exactly one output.
pub fn decode_tx_out_canonical(bytes: &[u8]) -> Result<TxOut, SnapshotDecodeError> {
    let mut o = 0usize;
    let out = read_tx_out(bytes, &mut o)?;
    if o != bytes.len() {
        return Err(SnapshotDecodeError::Structural {
            reason: StructuralReason::ArrayLengthMismatch,
        });
    }
    Ok(out)
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
            // Output quantity = canonical CBOR unsigned int (Word64). Byte-identical
            // to the prior signed encoding for any quantity ≤ i64::MAX.
            write_uint_canonical(buf, qty.0);
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
    let mut ma: BTreeMap<Hash28, BTreeMap<AssetName, OutputAssetQuantity>> = BTreeMap::new();
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
        let mut assets: BTreeMap<AssetName, OutputAssetQuantity> = BTreeMap::new();
        for _ in 0..n_assets {
            let (name_bytes, _w) = read_bytes(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
            let qty = read_output_quantity(bytes, o)?;
            assets.insert(AssetName(name_bytes), qty);
        }
        ma.insert(Hash28(pol), assets);
    }
    Ok(MultiAsset(ma))
}

/// Read one OUTPUT asset quantity (the non-negative Word64 domain) as a canonical
/// CBOR unsigned int. A negative CBOR integer in an output position is malformed —
/// outputs cannot carry a signed quantity — so it is a structured terminal error,
/// never coerced to a wrapped or saturated value.
fn read_output_quantity(
    bytes: &[u8],
    o: &mut usize,
) -> Result<OutputAssetQuantity, SnapshotDecodeError> {
    let (v, is_neg, _w) = read_any_int(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
    if is_neg {
        return Err(SnapshotDecodeError::Structural {
            reason: StructuralReason::NegativeAssetQuantity,
        });
    }
    Ok(OutputAssetQuantity(v))
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
            a.insert(AssetName(vec![0x01, 0x02]), OutputAssetQuantity(42));
            a.insert(AssetName(vec![0xFF]), OutputAssetQuantity(7));
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
    fn utxo_state_word64_multi_asset_quantity_round_trips() {
        // Output quantities are the full non-negative Word64 domain, including the
        // upper half that i64 could not represent. Every boundary value round-trips
        // exactly: 0, 1, i64::MAX, i64::MAX+1, u64::MAX.
        let mut ma = MultiAsset::new();
        ma.0.insert(Hash28([0xAA; 28]), {
            let mut a = BTreeMap::new();
            a.insert(AssetName(vec![0x01]), OutputAssetQuantity(0));
            a.insert(AssetName(vec![0x02]), OutputAssetQuantity(1));
            a.insert(AssetName(vec![0x03]), OutputAssetQuantity(i64::MAX as u64));
            a.insert(AssetName(vec![0x04]), OutputAssetQuantity(i64::MAX as u64 + 1));
            a.insert(AssetName(vec![0x05]), OutputAssetQuantity(u64::MAX));
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
    fn utxo_state_negative_output_quantity_is_rejected() {
        // A persisted snapshot whose multi-asset quantity is a NEGATIVE CBOR integer
        // is malformed for an output position and must fail closed — never coerced.
        // Hand-craft a state with one quantity == 1, then patch its encoding to a
        // CBOR negative one (0x20 == nint 0 == -1).
        let mut ma = MultiAsset::new();
        let mut a = BTreeMap::new();
        a.insert(AssetName(vec![0x01]), OutputAssetQuantity(1));
        ma.0.insert(Hash28([0xAA; 28]), a);
        let mut s = UTxOState::new();
        s.utxos.insert(
            tx_in(0xAA, 0),
            TxOut::ShelleyMary {
                address: vec![0xAA; 29],
                value: Value {
                    coin: Coin(0),
                    multi_asset: ma,
                },
            },
        );
        let mut bytes = encode_utxo_state(&s);
        // The sole 0x01 quantity byte (CBOR uint 1) is the last byte; flip it to a
        // negative one (major-1, value 0 => -1).
        let last = bytes.len() - 1;
        assert_eq!(bytes[last], 0x01, "the quantity byte to patch");
        bytes[last] = 0x20;
        assert!(matches!(
            decode_utxo_state(&bytes),
            Err(SnapshotDecodeError::Structural {
                reason: StructuralReason::NegativeAssetQuantity
            })
        ));
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

    #[test]
    fn tx_out_canonical_roundtrips_and_rejects_trailing_bytes() {
        // MEM-OPT-UTXO-DISK S2b: the single-TxOut anchor value codec round-trips,
        // and fails closed on trailing bytes (an anchor value is EXACTLY one output,
        // never a prefix of a longer buffer).
        let out = byron_out(0x42, 7777);
        let bytes = encode_tx_out_canonical(&out);
        match decode_tx_out_canonical(&bytes) {
            Ok(decoded) => assert_eq!(decoded, out, "TxOut codec must round-trip"),
            Err(e) => panic!("round-trip decode failed: {e:?}"),
        }
        let mut trailing = bytes.clone();
        trailing.push(0x00);
        assert!(
            decode_tx_out_canonical(&trailing).is_err(),
            "a trailing byte must be rejected, never silently ignored"
        );
    }

    #[test]
    fn representable_quantity_encodes_byte_identical_golden() {
        // BYTE-IDENTITY: a representable output quantity (≤ i64::MAX) encodes as a
        // canonical CBOR unsigned int — exactly what the prior signed encoding wrote
        // for a non-negative value. This golden pins the multi-asset value bytes so a
        // regression that changed the encoding for a representable value is caught.
        let mut ma = MultiAsset::new();
        let mut a = BTreeMap::new();
        // 500_000 = 0x0007A120 → CBOR uint: 0x1a 00 07 a1 20.
        a.insert(AssetName(vec![0xAB]), OutputAssetQuantity(500_000));
        ma.0.insert(Hash28([0x11; 28]), a);
        let mut buf = Vec::new();
        write_multi_asset(&mut buf, &ma);
        let mut expected = Vec::new();
        expected.push(0xA1); // map(1) policies
        expected.push(0x58); // bytes, 1-byte len follows
        expected.push(28);
        expected.extend_from_slice(&[0x11; 28]);
        expected.push(0xA1); // map(1) assets
        expected.push(0x41); // bytes(1)
        expected.push(0xAB);
        expected.extend_from_slice(&[0x1A, 0x00, 0x07, 0xA1, 0x20]); // uint 500_000
        assert_eq!(buf, expected, "representable quantity must be byte-identical");

        // And it round-trips through the reader.
        let mut o = 0usize;
        let decoded = read_multi_asset(&buf, &mut o).expect("decode");
        assert_eq!(decoded, ma);
        assert_eq!(o, buf.len());
    }

    #[test]
    fn stage2_mempack_word64_output_survives_snapshot_recovery() {
        // The real Stage-2 → value-model seam: a native MemPack TxOut whose multi-asset
        // quantity is > i64::MAX is decoded (DC-MITHRIL-05, faithful u64), promoted into
        // the authoritative ledger Value / UTxOState, persisted to a snapshot, recovered,
        // and the quantity is preserved EXACTLY across the whole round-trip.
        use crate::ledgerdb_tables::{read_txout, TxOutValue};

        // Build a tag-0 MemPack TxOut: enterprise addr + multi-asset value with one
        // asset of quantity u64::MAX.
        let policy = [0x22u8; 28];
        let name = b"BIG";
        let mut rep = Vec::new();
        rep.extend_from_slice(&u64::MAX.to_le_bytes()); // A: quantity @ 0
        rep.extend_from_slice(&12u16.to_le_bytes()); // B: policy off @ 8 → D @ 12n=12
        rep.extend_from_slice(&40u16.to_le_bytes()); // C: name off @ 10 → E @ 12+28=40
        rep.extend_from_slice(&policy); // D @ 12
        rep.extend_from_slice(name); // E @ 40
        let mut value_blob = vec![0x01u8, 0x00, 0x01, rep.len() as u8]; // tag, coin=0, numMA=1, repLen
        value_blob.extend_from_slice(&rep);

        let mut addr = vec![0x60u8]; // enterprise, testnet
        addr.extend_from_slice(&[0xcd; 28]);
        let mut blob = vec![0x00u8, addr.len() as u8];
        blob.extend_from_slice(&addr);
        blob.extend_from_slice(&value_blob);

        let decoded = read_txout(&blob).expect("mempack decode");
        let tv: &TxOutValue = &decoded.value;
        assert_eq!(tv.assets[&Hash28(policy)][&AssetName(name.to_vec())], u64::MAX);

        // Promote the faithful u64 TxOutValue into the authoritative ledger MultiAsset.
        let mut ledger_ma = MultiAsset::new();
        for (pol, names) in &tv.assets {
            let mut inner = BTreeMap::new();
            for (nm, q) in names {
                inner.insert(nm.clone(), OutputAssetQuantity(*q));
            }
            ledger_ma.0.insert(pol.clone(), inner);
        }
        let tx_out = TxOut::ShelleyMary {
            address: decoded.address.clone(),
            value: Value {
                coin: tv.coin,
                multi_asset: ledger_ma,
            },
        };
        let mut state = UTxOState::new();
        state.utxos.insert(tx_in(0x22, 0), tx_out);

        // Persist → recover; the u64::MAX quantity must survive exactly.
        let snapshot = encode_utxo_state(&state);
        let recovered = decode_utxo_state(&snapshot).expect("snapshot recovery");
        assert_eq!(recovered, state);
        match recovered.utxos.get(&tx_in(0x22, 0)) {
            Some(TxOut::ShelleyMary { value, .. }) => assert_eq!(
                value.multi_asset.0[&Hash28(policy)][&AssetName(name.to_vec())],
                OutputAssetQuantity(u64::MAX)
            ),
            other => panic!("expected ShelleyMary output, got {other:?}"),
        }
    }
}
