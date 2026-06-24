// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! MITHRIL-VERIFIED-ANCHOR-INTEGRATION / S1c — Stage-2 `tables` → authoritative `UTxOState`.
//!
//! The Stage-2 MemPack decoder ([`crate::ledgerdb_tables`]) yields a [`DecodedTxOut`] per output with
//! the hash-critical inline-datum / reference-script wire bytes PRESERVED. This module promotes each
//! `DecodedTxOut` into Ade's ledger [`TxOut`] with those bytes embedded VERBATIM (CBOR tag-24), the
//! full Word64 multi-asset quantities carried through with NO i64 conversion, then materializes the
//! whole `tables` map into a canonical-ordered [`UTxOState`] and binds its `fingerprint_utxo_v2` to
//! the one manifest point.
//!
//! Non-negotiables:
//! - The datum / script bytes are the identity Cardano hashes — they are embedded verbatim
//!   (`wrap_tag24`), never re-decoded/re-encoded.
//! - Multi-asset quantities are `u64` end to end (`OutputAssetQuantity(u64)`); never truncated,
//!   saturated, or i64-cast.
//! - FAIL-CLOSED: any unsupported tag / address form / non-ascending TxIn key is a structured terminal
//!   error — never an opaque keep-bytes fallback.

use std::collections::BTreeMap;

use ade_codec::cbor::{
    self, canonical_width, read_array_header, read_bytes, read_map_header, ContainerEncoding,
    IntWidth,
};
use ade_codec::wrap_tag24;
use ade_crypto::blake2b::blake2b_256;
use ade_types::mary::value::OutputAssetQuantity;
use ade_types::tx::TxIn;
use ade_types::{Hash28, Hash32};

use crate::ledgerdb_tables::{
    read_txout, DatumField, DecodedTxOut, ScriptField, TablesDecodeError, TxOutValue,
};
use crate::utxo::{TxOut, UTxOState};
use crate::value::{AssetName, MultiAsset, Value};

/// The Conway era index in the HardFork telescope — the materialization is era-bound to Conway, taken
/// from the SAME snapshot's Stage-1 `state` (never the tables file or a CLI flag).
const CONWAY_ERA_INDEX: usize = 6;

/// The Byron address kind nibble (`header >> 4`). A Byron output materializes to [`TxOut::Byron`].
const BYRON_ADDR_KIND: u8 = 0x8;

/// Why a `DecodedTxOut` → ledger `TxOut` conversion or a whole-`tables` materialization fails. Every
/// variant is TERMINAL (structured fail-closed, never a partial or defaulted emission).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TxOutMaterializeError {
    /// The address bytes are empty (no header byte) — a Byron classification cannot be made.
    EmptyAddress,
    /// The decoded TxOut from Stage-2 was malformed in a way the converter cannot represent.
    Tables(TablesDecodeError),
    /// The CBOR `tables` framing was malformed (outer array / map header / key-value framing).
    MalformedTables(String),
    /// A `tables` TxIn key was not the canonical 34 bytes (32-byte txid + 2-byte big-endian index).
    BadTxInKey { len: usize },
    /// The `tables` map keys were not in canonical ASCENDING order.
    NonAscendingTxIn,
    /// The Stage-1 `state` era is not Conway (this materialization is Conway-pinned).
    UnsupportedEra { decoded: String },
    /// A duplicate TxIn key appeared in the `tables` map (the same outpoint twice).
    DuplicateTxIn,
}

impl From<TablesDecodeError> for TxOutMaterializeError {
    fn from(e: TablesDecodeError) -> Self {
        TxOutMaterializeError::Tables(e)
    }
}

pub type R<T> = Result<T, TxOutMaterializeError>;

/// Convert a Stage-2 [`DecodedTxOut`] into Ade's authoritative ledger [`TxOut`], PURE + deterministic.
///
/// - **No datum AND no script** → [`TxOut::ShelleyMary`] (or [`TxOut::Byron`] when the address header
///   nibble is Byron), with the multi-asset bundle built by wrapping each Stage-2 `u64` quantity into
///   [`OutputAssetQuantity`] — never truncated / saturated / i64-cast.
/// - **Datum OR script present** → [`TxOut::AlonzoPlus`] whose `raw` is the canonical Conway TxOut CBOR
///   map (see [`encode_conway_txout_raw`]); the inline-datum / script bytes are embedded VERBATIM
///   inside a CBOR tag-24, never reconstructed (they are the identity Cardano hashes).
pub fn decoded_txout_to_ledger(o: DecodedTxOut) -> R<TxOut> {
    let coin = o.value.coin;
    let header = *o
        .address
        .first()
        .ok_or(TxOutMaterializeError::EmptyAddress)?;
    let is_byron = (header >> 4) == BYRON_ADDR_KIND;

    if matches!(o.datum, DatumField::None) && o.script.is_none() {
        // Pure-payment output: the structured ShelleyMary/Byron form (no preserved wire bytes needed —
        // there is no datum or script identity to preserve).
        if is_byron {
            return Ok(TxOut::Byron {
                address: ade_types::address::Address::Byron(o.address),
                coin,
            });
        }
        let value = Value {
            coin,
            multi_asset: build_multi_asset(&o.value),
        };
        return Ok(TxOut::ShelleyMary {
            address: o.address,
            value,
        });
    }

    // Datum or script present: the AlonzoPlus byte-preserved form. `raw` is the canonical Conway TxOut
    // CBOR; ade_plutus reads it directly for the ScriptContext.
    let raw = encode_conway_txout_raw(&o);
    Ok(TxOut::AlonzoPlus {
        raw,
        address: o.address,
        coin,
    })
}

/// Build a ledger [`MultiAsset`] from a Stage-2 [`TxOutValue`], wrapping each faithful `u64` quantity
/// into [`OutputAssetQuantity`]. Zero-quantity entries are NOT synthesized; the Stage-2 decode carries
/// the real quantities. BTreeMap order is canonical.
fn build_multi_asset(value: &TxOutValue) -> MultiAsset {
    let mut outer: BTreeMap<Hash28, BTreeMap<AssetName, OutputAssetQuantity>> = BTreeMap::new();
    for (policy, names) in &value.assets {
        let mut inner: BTreeMap<AssetName, OutputAssetQuantity> = BTreeMap::new();
        for (name, qty) in names {
            // FAITHFUL Word64: the full u64 quantity is preserved, never i64-cast/truncated/saturated.
            inner.insert(name.clone(), OutputAssetQuantity(*qty));
        }
        outer.insert(policy.clone(), inner);
    }
    MultiAsset(outer)
}

/// Encode the canonical Conway TxOut CBOR map (keys ascending) for a [`DecodedTxOut`] that carries a
/// datum and/or script. This is the `raw` of [`TxOut::AlonzoPlus`].
///
/// ```text
/// {
///   0: address bytes,
///   1: coin (uint)              | [coin, {policy: {name: qty}}]   (multi-asset),
///   2: <if datum>  Hash(h) -> [0, h]  | Inline(b) -> [1, #6.24(b)],
///   3: <if script> #6.24([type, script_bytes])   (Native -> [0, bytes]; PlutusVn -> [n, bytes]),
/// }
/// ```
///
/// The inline-datum bytes (`b`) and the script bytes are embedded VERBATIM inside the tag-24
/// (`#6.24`, CBOR-encoded-CBOR) via [`wrap_tag24`] — never re-decoded/re-encoded. Quantities are CBOR
/// UNSIGNED ints (u64); the policy/name maps are written in canonical (BTreeMap-sorted) order.
pub fn encode_conway_txout_raw(o: &DecodedTxOut) -> Vec<u8> {
    let has_datum = !matches!(o.datum, DatumField::None);
    let has_script = o.script.is_some();
    let mut count: u64 = 2; // keys 0 (address) + 1 (value) always present
    if has_datum {
        count += 1;
    }
    if has_script {
        count += 1;
    }

    let mut buf = Vec::new();
    cbor::write_map_header(&mut buf, ContainerEncoding::Definite(count, canonical_width(count)));

    // key 0: address bytes.
    cbor::write_uint_canonical(&mut buf, 0);
    cbor::write_bytes_canonical(&mut buf, &o.address);

    // key 1: value (coin uint if ada-only, else [coin, {policy: {name: qty}}]).
    cbor::write_uint_canonical(&mut buf, 1);
    write_value(&mut buf, &o.value);

    // key 2: datum_option (Hash -> [0, h]; Inline -> [1, #6.24(bytes)]).
    if has_datum {
        cbor::write_uint_canonical(&mut buf, 2);
        match &o.datum {
            DatumField::None => {}
            DatumField::Hash(h) => {
                cbor::write_array_header(&mut buf, ContainerEncoding::Definite(2, IntWidth::Inline));
                cbor::write_uint_canonical(&mut buf, 0);
                cbor::write_bytes_canonical(&mut buf, h);
            }
            DatumField::Inline(bytes) => {
                cbor::write_array_header(&mut buf, ContainerEncoding::Definite(2, IntWidth::Inline));
                cbor::write_uint_canonical(&mut buf, 1);
                // The inline datum is CBOR-encoded Plutus Data hashed by the network on its WIRE bytes;
                // embed verbatim in #6.24, never re-encode.
                buf.extend_from_slice(&wrap_tag24(bytes));
            }
        }
    }

    // key 3: reference script #6.24([type, script_bytes]).
    if let Some(script) = &o.script {
        cbor::write_uint_canonical(&mut buf, 3);
        let inner = encode_script_inner(script);
        // The whole `[type, script_bytes]` array is itself wrapped in #6.24 (Conway script_ref form);
        // the script bytes inside are verbatim.
        buf.extend_from_slice(&wrap_tag24(&inner));
    }

    buf
}

/// The inner CBOR `[type, script_bytes]` of a Conway reference script, BEFORE the tag-24 wrap. The
/// type tag is `0` for native, `n` for Plutus version `n` (V1→1, V2→2, V3→3). The script bytes are
/// embedded VERBATIM (the memoized native CBOR / the flat Plutus bytes the network hashes).
fn encode_script_inner(script: &ScriptField) -> Vec<u8> {
    let (tag, bytes): (u64, &[u8]) = match script {
        ScriptField::Native(b) => (0, b),
        ScriptField::Plutus { version, bytes } => (*version as u64, bytes),
    };
    let mut inner = Vec::new();
    cbor::write_array_header(&mut inner, ContainerEncoding::Definite(2, IntWidth::Inline));
    cbor::write_uint_canonical(&mut inner, tag);
    cbor::write_bytes_canonical(&mut inner, bytes);
    inner
}

/// Write the Conway value form: `coin` (uint) when ada-only, else `[coin, {policy: {name: qty}}]`
/// with each `qty` a CBOR UNSIGNED int (u64) and the policy/name maps canonical-sorted (BTreeMap).
fn write_value(buf: &mut Vec<u8>, value: &TxOutValue) {
    if value.is_ada_only() {
        cbor::write_uint_canonical(buf, value.coin.0);
        return;
    }
    cbor::write_array_header(buf, ContainerEncoding::Definite(2, IntWidth::Inline));
    cbor::write_uint_canonical(buf, value.coin.0);
    // multi-asset map: { policy_id: { asset_name: qty } }, canonical (sorted) order.
    cbor::write_map_header(
        buf,
        ContainerEncoding::Definite(value.assets.len() as u64, canonical_width(value.assets.len() as u64)),
    );
    for (policy, names) in &value.assets {
        cbor::write_bytes_canonical(buf, &policy.0);
        cbor::write_map_header(
            buf,
            ContainerEncoding::Definite(names.len() as u64, canonical_width(names.len() as u64)),
        );
        for (name, qty) in names {
            cbor::write_bytes_canonical(buf, &name.0);
            // FAITHFUL Word64: a CBOR unsigned int over the full u64 (a quantity > i64::MAX is a
            // major-0 uint, never a negative/signed encoding).
            cbor::write_uint_canonical(buf, *qty);
        }
    }
}

/// Parse a `tables` TxIn key: 32 txid bytes + 2 big-endian index bytes → [`TxIn`].
fn parse_txin_key(key: &[u8]) -> R<TxIn> {
    if key.len() != 34 {
        return Err(TxOutMaterializeError::BadTxInKey { len: key.len() });
    }
    let mut tx_hash = [0u8; 32];
    tx_hash.copy_from_slice(&key[0..32]);
    let index = u16::from_be_bytes([key[32], key[33]]);
    Ok(TxIn {
        tx_hash: Hash32(tx_hash),
        index,
    })
}

/// Materialize the whole V2 `tables` CBOR map into Ade's authoritative [`UTxOState`], BOUND to the era
/// decoded from the SAME snapshot's `state` (the Stage-1 NES). Iterates the map in canonical ASCENDING
/// TxIn order (asserting it — non-ascending is terminal), parses each TxIn key, decodes each TxOut via
/// the Stage-2 [`read_txout`] and promotes it via [`decoded_txout_to_ledger`], accumulates a canonical
/// `BTreeMap<TxIn, TxOut>` → [`UTxOState::from_map`]. `max_entries` caps the work for tests
/// (None = the whole file). FAIL-CLOSED on any unsupported form.
pub fn materialize_tables_to_utxo(
    tables: &[u8],
    state_era_index: usize,
    max_entries: Option<usize>,
) -> R<UTxOState> {
    if state_era_index != CONWAY_ERA_INDEX {
        return Err(TxOutMaterializeError::UnsupportedEra {
            decoded: format!("state era index {state_era_index} (require Conway)"),
        });
    }
    let mut o = 0usize;
    match read_array_header(tables, &mut o)
        .map_err(|e| TxOutMaterializeError::MalformedTables(format!("{e:?}")))?
    {
        ContainerEncoding::Definite(1, _) => {}
        other => {
            return Err(TxOutMaterializeError::MalformedTables(format!(
                "tables outer array != 1: {other:?}"
            )))
        }
    }
    let _ = read_map_header(tables, &mut o)
        .map_err(|e| TxOutMaterializeError::MalformedTables(format!("{e:?}")))?;

    let mut map: BTreeMap<TxIn, TxOut> = BTreeMap::new();
    let mut count = 0usize;
    let mut prev: Option<Vec<u8>> = None;
    loop {
        if o >= tables.len() || tables[o] == 0xff {
            break;
        }
        let (txin_key, _) = read_bytes(tables, &mut o)
            .map_err(|e| TxOutMaterializeError::MalformedTables(format!("{e:?}")))?;
        let (val, _) = read_bytes(tables, &mut o)
            .map_err(|e| TxOutMaterializeError::MalformedTables(format!("{e:?}")))?;
        if let Some(p) = &prev {
            if &txin_key <= p {
                return Err(TxOutMaterializeError::NonAscendingTxIn);
            }
        }
        prev = Some(txin_key.clone());

        let tx_in = parse_txin_key(&txin_key)?;
        let decoded = read_txout(&val)?;
        let tx_out = decoded_txout_to_ledger(decoded)?;
        if map.insert(tx_in, tx_out).is_some() {
            return Err(TxOutMaterializeError::DuplicateTxIn);
        }
        count += 1;
        if let Some(m) = max_entries {
            if count >= m {
                break;
            }
        }
    }
    Ok(UTxOState::from_map(map))
}

/// The point-coherent binding of the materialized UTxO authority to the one manifest snapshot.
///
/// A deterministic blake2b record over: the manifest certified point hash + the Stage-1
/// `NativeSnapshotNonUtxoState` commitment + the Stage-2 `decode_tables_commitment` + the materialized
/// UTxO `fingerprint_utxo_v2`. The UTxO authority is visible only when all four bind to the same
/// manifest point; a wrong point / wrong Stage-1 / wrong Stage-2 / wrong UTxO is a DIFFERENT record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UtxoBindingRecord {
    pub binding: Hash32,
}

/// Compute the [`UtxoBindingRecord`] over the four commitments. The materialized UTxO's
/// `fingerprint_utxo_v2` is recomputed here from `utxo` (the one authoritative fingerprint).
pub fn bind_utxo_to_manifest(
    manifest_point_hash: &Hash32,
    stage1_nonutxo_commitment: &Hash32,
    stage2_tables_commitment: &Hash32,
    utxo: &UTxOState,
) -> UtxoBindingRecord {
    let utxo_fp = crate::fingerprint::fingerprint_utxo_v2(utxo);
    let mut buf = Vec::with_capacity(4 * 32 + 64);
    buf.extend_from_slice(b"ade-mithril-s1c-utxo-binding-v1");
    buf.extend_from_slice(&manifest_point_hash.0);
    buf.extend_from_slice(&stage1_nonutxo_commitment.0);
    buf.extend_from_slice(&stage2_tables_commitment.0);
    buf.extend_from_slice(&utxo_fp.0);
    UtxoBindingRecord {
        binding: blake2b_256(&buf),
    }
}

/// Verify a materialized UTxO binds to the expected manifest point + Stage-1 + Stage-2 commitments.
/// TERMINAL on mismatch — the UTxO authority is rejected unless it binds to the one manifest point.
pub fn verify_utxo_binding(
    expected: &UtxoBindingRecord,
    manifest_point_hash: &Hash32,
    stage1_nonutxo_commitment: &Hash32,
    stage2_tables_commitment: &Hash32,
    utxo: &UTxOState,
) -> Result<(), UtxoBindingMismatch> {
    let computed = bind_utxo_to_manifest(
        manifest_point_hash,
        stage1_nonutxo_commitment,
        stage2_tables_commitment,
        utxo,
    );
    if computed == *expected {
        Ok(())
    } else {
        Err(UtxoBindingMismatch {
            expected: expected.binding.clone(),
            computed: computed.binding,
        })
    }
}

/// The terminal binding-mismatch error (wrong point / Stage-1 / Stage-2 / UTxO).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UtxoBindingMismatch {
    pub expected: Hash32,
    pub computed: Hash32,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use self::ade_ledger_test_helpers::{find_subslice, ParsedTxOut};
    use crate::ledgerdb_tables::decode_tables_commitment;

    // --- Synthetic MemPack TxOut blobs (the exact Stage-2 `read_txout` layout) ----------------

    /// An enterprise on-wire address (header 0x60 = enterprise/testnet + 28 payment bytes = 29 bytes).
    fn enterprise_addr(fill: u8) -> Vec<u8> {
        let mut a = vec![0x60u8];
        a.extend_from_slice(&[fill; 28]);
        a
    }

    /// A base on-wire address (header 0x00 = base/testnet + 56 credential bytes = 57 bytes).
    fn base_addr() -> Vec<u8> {
        let mut a = vec![0x00u8];
        a.extend_from_slice(&[0xab; 56]);
        a
    }

    /// MemPack ada-only CompactValue: tag 0x00 + a 1-byte VarLen coin (coin < 128).
    fn ada_only_value(coin: u8) -> Vec<u8> {
        vec![0x00, coin]
    }

    /// A MemPack `CompactAddr` prefix: VarLen(len) + the address bytes (len < 128).
    fn compact_addr(addr: &[u8]) -> Vec<u8> {
        let mut v = vec![addr.len() as u8];
        v.extend_from_slice(addr);
        v
    }

    /// tag-0 TxOut: enterprise addr + ada-only value (no datum, no script).
    fn txout_tag0(coin: u8) -> Vec<u8> {
        let mut v = vec![0x00u8];
        v.extend_from_slice(&compact_addr(&enterprise_addr(0xcd)));
        v.extend_from_slice(&ada_only_value(coin));
        v
    }

    /// tag-0 TxOut with a single multi-asset (policy, name "ABC", quantity `qty` — full u64).
    fn txout_tag0_multiasset(policy: [u8; 28], name: &[u8], qty: u64) -> Vec<u8> {
        // CompactValue multi-asset rep for ONE triple: A=qty@0 (8B), B=policy_off@8 -> D@12 (2B),
        // C=name_off@10 -> E@12+28=40 (2B), D=policy@12 (28B), E=name@40.
        let mut rep = Vec::new();
        rep.extend_from_slice(&qty.to_le_bytes()); // A
        rep.extend_from_slice(&12u16.to_le_bytes()); // B
        rep.extend_from_slice(&40u16.to_le_bytes()); // C
        rep.extend_from_slice(&policy); // D
        rep.extend_from_slice(name); // E
        assert_eq!(rep.len(), 40 + name.len());
        // value: tag 0x01 multi-asset + coin 0 + numMA=1 + VarLen(repLen) + rep.
        let mut value = vec![0x01u8, 0x00, 0x01, rep.len() as u8];
        value.extend_from_slice(&rep);
        // txout tag 0: enterprise addr + this value.
        let mut v = vec![0x00u8];
        v.extend_from_slice(&compact_addr(&enterprise_addr(0xee)));
        v.extend_from_slice(&value);
        v
    }

    /// tag-4 TxOut: base addr + ada-only value + an inline datum (VarLen-prefixed wire bytes).
    fn txout_tag4_inline_datum(coin: u8, datum: &[u8]) -> Vec<u8> {
        let mut v = vec![0x04u8];
        v.extend_from_slice(&compact_addr(&base_addr()));
        v.extend_from_slice(&ada_only_value(coin));
        v.push(datum.len() as u8); // VarLen length (datum < 128)
        v.extend_from_slice(datum);
        v
    }

    /// tag-5 TxOut: base addr + ada-only value + datum-none + a Plutus reference script.
    fn txout_tag5_plutus_script(coin: u8, version_byte: u8, script: &[u8]) -> Vec<u8> {
        let mut v = vec![0x05u8];
        v.extend_from_slice(&compact_addr(&base_addr()));
        v.extend_from_slice(&ada_only_value(coin));
        v.push(0x00); // datum option: none
        v.push(0x01); // alonzo script tag: Plutus
        v.push(version_byte); // language byte 0=V1 / 1=V2 / 2=V3
        v.push(script.len() as u8); // VarLen length
        v.extend_from_slice(script);
        v
    }

    /// tag-5 TxOut: base addr + ada-only value + an inline datum + a NATIVE reference script.
    fn txout_tag5_inline_datum_native_script(coin: u8, datum: &[u8], native: &[u8]) -> Vec<u8> {
        let mut v = vec![0x05u8];
        v.extend_from_slice(&compact_addr(&base_addr()));
        v.extend_from_slice(&ada_only_value(coin));
        v.push(0x02); // datum option: inline
        v.push(datum.len() as u8);
        v.extend_from_slice(datum);
        v.push(0x00); // alonzo script tag: native
        v.push(native.len() as u8);
        v.extend_from_slice(native);
        v
    }

    /// Build a `tables` CBOR map (array(1) + indefinite map) from `(txid, ix, txout_value)` triples.
    /// Keys are 34-byte (32 txid + 2 BE ix) CBOR byte strings; values are CBOR byte strings.
    fn tables(entries: &[(u8, u16, Vec<u8>)]) -> Vec<u8> {
        let mut t = vec![0x81u8, 0xbf]; // array(1), indefinite map
        for (txid, ix, val) in entries {
            // key: bytes(34) = 0x58 0x22 || 32*txid || ix(BE)
            t.push(0x58);
            t.push(34);
            t.extend(vec![*txid; 32]);
            t.extend_from_slice(&ix.to_be_bytes());
            // value: bytes(len)
            t.push(0x58);
            t.push(val.len() as u8);
            t.extend_from_slice(val);
        }
        t.push(0xff);
        t
    }

    fn h32(b: u8) -> Hash32 {
        Hash32([b; 32])
    }

    // --- Acceptance tests ---------------------------------------------------------------------

    #[test]
    fn deterministic_utxo_commitment() {
        // Same `tables` + manifest -> byte-identical UTxOState commitment + binding record.
        let t = tables(&[
            (0x01, 0, txout_tag0(10)),
            (0x02, 0, txout_tag4_inline_datum(7, &[0x9f, 0x01, 0x02, 0xff])),
            (0x03, 1, txout_tag5_plutus_script(5, 1, &[0xaa, 0xbb, 0xcc])),
        ]);
        let u1 = materialize_tables_to_utxo(&t, 6, None).unwrap();
        let u2 = materialize_tables_to_utxo(&t, 6, None).unwrap();
        assert_eq!(u1.len(), 3);
        // The authoritative UTxO fingerprint is identical for the same tables.
        assert_eq!(
            crate::fingerprint::fingerprint_utxo_v2(&u1),
            crate::fingerprint::fingerprint_utxo_v2(&u2)
        );
        // The binding record is identical for the same point + Stage-1 + Stage-2 + UTxO.
        let point = h32(0x11);
        let s1 = h32(0x22);
        let s2 = h32(0x33);
        let b1 = bind_utxo_to_manifest(&point, &s1, &s2, &u1);
        let b2 = bind_utxo_to_manifest(&point, &s1, &s2, &u2);
        assert_eq!(b1, b2);
    }

    #[test]
    fn u64_above_i64_max_materializes_persists_recovers_exactly() {
        // A multi-asset output with a quantity ABOVE i64::MAX materializes, persists, and recovers
        // with the EXACT quantity (no i64 cast / truncation / saturation anywhere).
        let policy = [0xaau8; 28];
        let big = i64::MAX as u64 + 123;
        let t = tables(&[(0x01, 0, txout_tag0_multiasset(policy, b"ABC", big))]);
        let utxo = materialize_tables_to_utxo(&t, 6, None).unwrap();
        assert_eq!(utxo.len(), 1);

        // The materialized ShelleyMary output carries the full u64 quantity.
        let tx_in = TxIn {
            tx_hash: Hash32([0x01; 32]),
            index: 0,
        };
        match crate::utxo::utxo_lookup(&utxo, &tx_in).unwrap() {
            TxOut::ShelleyMary { value, .. } => {
                let q = value.multi_asset.0[&Hash28(policy)][&AssetName(b"ABC".to_vec())];
                assert_eq!(q, OutputAssetQuantity(big));
                assert!(big > i64::MAX as u64, "the test quantity must exceed i64::MAX");
            }
            other => panic!("expected ShelleyMary, got {other:?}"),
        }

        // persist -> recover -> EXACT same quantity + identical fingerprint.
        let bytes = crate::snapshot::utxo_state::encode_utxo_state(&utxo);
        let recovered = crate::snapshot::utxo_state::decode_utxo_state(&bytes).unwrap();
        assert_eq!(
            crate::fingerprint::fingerprint_utxo_v2(&utxo),
            crate::fingerprint::fingerprint_utxo_v2(&recovered)
        );
        match crate::utxo::utxo_lookup(&recovered, &tx_in).unwrap() {
            TxOut::ShelleyMary { value, .. } => {
                assert_eq!(
                    value.multi_asset.0[&Hash28(policy)][&AssetName(b"ABC".to_vec())],
                    OutputAssetQuantity(big)
                );
            }
            other => panic!("expected recovered ShelleyMary, got {other:?}"),
        }
    }

    #[test]
    fn datum_and_script_bytes_preserved_verbatim_in_raw() {
        // The inline-datum bytes and the script bytes inside `raw` are BYTE-IDENTICAL to the
        // DecodedTxOut's DatumField::Inline / ScriptField bytes (embedded verbatim in tag-24).
        let datum = vec![0xd8, 0x7a, 0x9f, 0x01, 0x02, 0x03, 0xff]; // arbitrary CBOR-ish bytes
        let native = vec![0x82, 0x00, 0x41, 0x77]; // arbitrary native-script bytes
        let decoded = read_txout(&txout_tag5_inline_datum_native_script(9, &datum, &native)).unwrap();
        // Confirm the decoded fields carry our exact bytes.
        assert_eq!(decoded.datum, DatumField::Inline(datum.clone()));
        assert_eq!(decoded.script, Some(ScriptField::Native(native.clone())));

        let raw = encode_conway_txout_raw(&decoded);

        // The verbatim datum bytes must appear inside `raw` wrapped in #6.24 (d8 18 || bytes(len) ||
        // datum) — find the tag-24 envelope and assert the inner bytes equal `datum` exactly.
        let wrapped_datum = wrap_tag24(&datum);
        assert!(
            find_subslice(&raw, &wrapped_datum).is_some(),
            "inline datum must be embedded verbatim in a #6.24 envelope inside raw"
        );
        // The script inner is [0, bytes(native)] wrapped in #6.24; the native bytes must appear verbatim.
        let script_inner = {
            let mut inner = Vec::new();
            ade_codec::cbor::write_array_header(
                &mut inner,
                ContainerEncoding::Definite(2, IntWidth::Inline),
            );
            ade_codec::cbor::write_uint_canonical(&mut inner, 0);
            ade_codec::cbor::write_bytes_canonical(&mut inner, &native);
            inner
        };
        let wrapped_script = wrap_tag24(&script_inner);
        assert!(
            find_subslice(&raw, &wrapped_script).is_some(),
            "native script must be embedded verbatim in a #6.24 envelope inside raw"
        );
        // And the bare native bytes are present byte-identically.
        assert!(find_subslice(&raw, &native).is_some());
    }

    #[test]
    fn alonzo_plus_raw_round_trips_to_same_fields() {
        // DecodedTxOut -> AlonzoPlus.raw -> parse the raw -> the SAME address / value / datum / script.
        let datum = vec![0x9f, 0x18, 0x2a, 0xff];
        let script = vec![0x46, 0x01, 0x00, 0x00, 0x22, 0x01]; // arbitrary Plutus flat bytes
        let decoded = read_txout(&txout_tag5_inline_datum_native_script(33, &datum, &script)).unwrap();
        let decoded2 = read_txout(&txout_tag5_plutus_script(44, 2, &script)).unwrap(); // PlutusV3

        let ledger = decoded_txout_to_ledger(decoded.clone()).unwrap();
        let raw = match &ledger {
            TxOut::AlonzoPlus { raw, address, coin } => {
                assert_eq!(address, &decoded.address);
                assert_eq!(*coin, decoded.value.coin);
                raw.clone()
            }
            other => panic!("expected AlonzoPlus, got {other:?}"),
        };

        // Parse the canonical Conway TxOut map directly with the public cbor primitives — this proves
        // `raw` is decodable canonical CBOR and recovers the same fields losslessly.
        let parsed = ParsedTxOut::parse(&raw).unwrap();
        assert_eq!(parsed.address, decoded.address);
        assert_eq!(parsed.coin, decoded.value.coin.0);
        // datum_option present, tag 1 (inline), inner == the verbatim datum.
        assert_eq!(parsed.datum_inline.as_deref(), Some(datum.as_slice()));
        // script_ref present, tag 0 (native), inner == the verbatim script.
        assert_eq!(parsed.script_native.as_deref(), Some(script.as_slice()));

        // The Plutus-V3 case round-trips its script bytes + version tag too.
        let ledger3 = decoded_txout_to_ledger(decoded2.clone()).unwrap();
        if let TxOut::AlonzoPlus { raw, .. } = &ledger3 {
            let parsed3 = ParsedTxOut::parse(raw).unwrap();
            assert_eq!(parsed3.script_plutus, Some((3u64, script.clone())));
        } else {
            panic!("expected AlonzoPlus for the Plutus-V3 case");
        }
    }

    #[test]
    fn canonical_txin_ordering_asserted() {
        // Ascending keys materialize; a non-ascending key is TERMINAL (no opaque accept).
        let ok = tables(&[
            (0x01, 0, txout_tag0(10)),
            (0x01, 1, txout_tag0(11)),
            (0x02, 0, txout_tag0(12)),
        ]);
        let u = materialize_tables_to_utxo(&ok, 6, None).unwrap();
        assert_eq!(u.len(), 3);

        let bad = tables(&[(0x02, 0, txout_tag0(20)), (0x01, 0, txout_tag0(10))]);
        assert_eq!(
            materialize_tables_to_utxo(&bad, 6, None),
            Err(TxOutMaterializeError::NonAscendingTxIn)
        );
        // A repeated identical key is also non-ascending (<=) -> terminal.
        let dup = tables(&[(0x01, 0, txout_tag0(10)), (0x01, 0, txout_tag0(11))]);
        assert_eq!(
            materialize_tables_to_utxo(&dup, 6, None),
            Err(TxOutMaterializeError::NonAscendingTxIn)
        );
    }

    #[test]
    fn fail_closed_negatives() {
        // (a) non-Conway state era -> terminal (no interpretation of the tables).
        let t = tables(&[(0x01, 0, txout_tag0(10))]);
        assert!(matches!(
            materialize_tables_to_utxo(&t, 5, None),
            Err(TxOutMaterializeError::UnsupportedEra { .. })
        ));

        // (b) unsupported TxOut tag -> terminal (Tables -> UnsupportedTxOutTag).
        let bad_tag = tables(&[(0x01, 0, vec![0x06, 0x00])]);
        assert!(matches!(
            materialize_tables_to_utxo(&bad_tag, 6, None),
            Err(TxOutMaterializeError::Tables(TablesDecodeError::UnsupportedTxOutTag(6)))
        ));

        // (c) unsupported address form -> terminal (bad length).
        let mut bad_addr_val = vec![0x00u8, 3u8, 0x00, 0x01, 0x02]; // tag0, CompactAddr len 3 (invalid)
        bad_addr_val.extend_from_slice(&[0x00, 0x05]); // value
        let bad_addr = tables(&[(0x01, 0, bad_addr_val)]);
        assert!(matches!(
            materialize_tables_to_utxo(&bad_addr, 6, None),
            Err(TxOutMaterializeError::Tables(TablesDecodeError::UnsupportedAddress(_)))
        ));

        // (d) unsupported value tag -> terminal.
        let mut bad_val = vec![0x00u8];
        bad_val.extend_from_slice(&compact_addr(&enterprise_addr(0xcd)));
        bad_val.extend_from_slice(&[0x07, 0x00]); // value tag 0x07 (invalid)
        let bad_value = tables(&[(0x01, 0, bad_val)]);
        assert!(matches!(
            materialize_tables_to_utxo(&bad_value, 6, None),
            Err(TxOutMaterializeError::Tables(TablesDecodeError::UnsupportedValueTag(7)))
        ));

        // (e) unsupported Plutus language byte -> terminal.
        let mut bad_script = vec![0x05u8];
        bad_script.extend_from_slice(&compact_addr(&base_addr()));
        bad_script.extend_from_slice(&ada_only_value(5));
        bad_script.push(0x00); // datum none
        bad_script.push(0x01); // Plutus
        bad_script.push(0x09); // language byte 9 (invalid)
        bad_script.push(0x01);
        bad_script.push(0xaa);
        let bad_lang = tables(&[(0x01, 0, bad_script)]);
        assert!(matches!(
            materialize_tables_to_utxo(&bad_lang, 6, None),
            Err(TxOutMaterializeError::Tables(TablesDecodeError::UnsupportedScript(_)))
        ));

        // (f) a bad (non-34-byte) TxIn key -> terminal. Build the tables map by hand with a 33-byte key.
        let mut t6 = vec![0x81u8, 0xbf];
        t6.push(0x58);
        t6.push(33); // key bytes(33) — wrong length
        t6.extend(vec![0x01u8; 33]);
        let val = txout_tag0(10);
        t6.push(0x58);
        t6.push(val.len() as u8);
        t6.extend_from_slice(&val);
        t6.push(0xff);
        assert!(matches!(
            materialize_tables_to_utxo(&t6, 6, None),
            Err(TxOutMaterializeError::BadTxInKey { len: 33 })
        ));
    }

    #[test]
    fn binding_is_terminal_on_mismatch() {
        // The UTxO commitment binds to the manifest point + Stage-1 + Stage-2; ANY wrong input is a
        // DIFFERENT record (verify_utxo_binding is terminal).
        let t = tables(&[(0x01, 0, txout_tag0(10)), (0x02, 0, txout_tag0(20))]);
        let utxo = materialize_tables_to_utxo(&t, 6, None).unwrap();
        let point = h32(0xaa);
        let s1 = h32(0xbb);
        let s2 = h32(0xcc);
        let record = bind_utxo_to_manifest(&point, &s1, &s2, &utxo);

        // The correct quadruple verifies.
        assert!(verify_utxo_binding(&record, &point, &s1, &s2, &utxo).is_ok());
        // Wrong point -> terminal.
        assert!(verify_utxo_binding(&record, &h32(0xa0), &s1, &s2, &utxo).is_err());
        // Wrong Stage-1 -> terminal.
        assert!(verify_utxo_binding(&record, &point, &h32(0xb0), &s2, &utxo).is_err());
        // Wrong Stage-2 -> terminal.
        assert!(verify_utxo_binding(&record, &point, &s1, &h32(0xc0), &utxo).is_err());
        // Wrong UTxO (a different set) -> terminal.
        let other = materialize_tables_to_utxo(&tables(&[(0x01, 0, txout_tag0(99))]), 6, None).unwrap();
        assert!(verify_utxo_binding(&record, &point, &s1, &s2, &other).is_err());
    }

    #[test]
    fn persist_recover_identical_fingerprint() {
        // Persist the materialized UTxOState -> recover -> identical fingerprint (and the recovered
        // UTxOState equals the original).
        let t = tables(&[
            (0x01, 0, txout_tag0(10)),
            (0x02, 0, txout_tag4_inline_datum(7, &[0x9f, 0x01, 0xff])),
            (0x03, 2, txout_tag5_plutus_script(5, 2, &[0x46, 0x01, 0x00, 0x00, 0x22])),
        ]);
        let utxo = materialize_tables_to_utxo(&t, 6, None).unwrap();
        let bytes = crate::snapshot::utxo_state::encode_utxo_state(&utxo);
        let recovered = crate::snapshot::utxo_state::decode_utxo_state(&bytes).unwrap();
        assert_eq!(utxo, recovered);
        assert_eq!(
            crate::fingerprint::fingerprint_utxo_v2(&utxo),
            crate::fingerprint::fingerprint_utxo_v2(&recovered),
            "persist -> recover preserves the authoritative fingerprint"
        );
    }

    #[test]
    fn no_datum_no_script_is_shelley_mary_byron_is_byron() {
        // A pure-payment Shelley+ output -> ShelleyMary; a Byron-header address -> Byron.
        let shelley = decoded_txout_to_ledger(read_txout(&txout_tag0(10)).unwrap()).unwrap();
        assert!(matches!(shelley, TxOut::ShelleyMary { .. }));

        // A Byron address: header nibble 0x8. The Stage-2 decoder accepts kind 0x8 (variable length).
        // Build a tag-0 TxOut whose CompactAddr is a Byron form (header 0x82, a few bytes).
        let byron_addr = vec![0x82u8, 0xd8, 0x18, 0x44, 0xaa, 0xbb, 0xcc, 0xdd];
        let mut blob = vec![0x00u8];
        blob.push(byron_addr.len() as u8);
        blob.extend_from_slice(&byron_addr);
        blob.extend_from_slice(&ada_only_value(5));
        let byron = decoded_txout_to_ledger(read_txout(&blob).unwrap()).unwrap();
        match byron {
            TxOut::Byron { address, coin } => {
                assert_eq!(coin.0, 5);
                assert_eq!(address, ade_types::address::Address::Byron(byron_addr));
            }
            other => panic!("expected Byron, got {other:?}"),
        }
    }

    // --- Cross-check: the materialized set agrees with the Stage-2 commitment count -----------

    #[test]
    fn materialized_count_matches_stage2_commitment_count() {
        // The materialization and the Stage-2 `decode_tables_commitment` walk the SAME entries in the
        // SAME order — the counts agree (a cross-check the materialization didn't drop/duplicate).
        let t = tables(&[
            (0x01, 0, txout_tag0(10)),
            (0x02, 0, txout_tag0(20)),
            (0x03, 0, txout_tag5_plutus_script(5, 1, &[0xaa])),
        ]);
        let utxo = materialize_tables_to_utxo(&t, 6, None).unwrap();
        let summary = decode_tables_commitment(&t, 6, None).unwrap();
        assert_eq!(utxo.len(), summary.count);
    }

    // --- Local test helpers (a tiny manual Conway-TxOut-map parser for the round-trip) --------

    mod ade_ledger_test_helpers {
        use ade_codec::cbor::{
            peek_major, read_array_header, read_bytes, read_map_header, read_tag, read_uint,
            ContainerEncoding, MAJOR_ARRAY, MAJOR_UNSIGNED,
        };

        /// Find the first occurrence of `needle` in `haystack` (a byte-identity check helper).
        pub fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
            if needle.is_empty() || needle.len() > haystack.len() {
                return None;
            }
            haystack
                .windows(needle.len())
                .position(|w| w == needle)
        }

        /// A minimally-parsed Conway TxOut map (the inverse of `encode_conway_txout_raw`, via the
        /// public ade_codec cbor primitives), proving `raw` is decodable canonical CBOR.
        pub struct ParsedTxOut {
            pub address: Vec<u8>,
            pub coin: u64,
            pub datum_inline: Option<Vec<u8>>,
            pub script_native: Option<Vec<u8>>,
            pub script_plutus: Option<(u64, Vec<u8>)>,
        }

        impl ParsedTxOut {
            pub fn parse(raw: &[u8]) -> Result<Self, String> {
                let mut o = 0usize;
                let n = match read_map_header(raw, &mut o).map_err(|e| format!("{e:?}"))? {
                    ContainerEncoding::Definite(n, _) => n,
                    _ => return Err("indefinite map".into()),
                };
                let mut address = None;
                let mut coin = 0u64;
                let mut datum_inline = None;
                let mut script_native = None;
                let mut script_plutus = None;
                for _ in 0..n {
                    let (key, _) = read_uint(raw, &mut o).map_err(|e| format!("{e:?}"))?;
                    match key {
                        0 => {
                            let (a, _) = read_bytes(raw, &mut o).map_err(|e| format!("{e:?}"))?;
                            address = Some(a);
                        }
                        1 => {
                            // coin (uint) or [coin, ma] — we only need the coin for the round-trip.
                            let major = peek_major(raw, o).map_err(|e| format!("{e:?}"))?;
                            if major == MAJOR_UNSIGNED {
                                let (v, _) = read_uint(raw, &mut o).map_err(|e| format!("{e:?}"))?;
                                coin = v;
                            } else if major == MAJOR_ARRAY {
                                let _ = read_array_header(raw, &mut o)
                                    .map_err(|e| format!("{e:?}"))?;
                                let (v, _) =
                                    read_uint(raw, &mut o).map_err(|e| format!("{e:?}"))?;
                                coin = v;
                                // skip the multi-asset map.
                                let _ = ade_codec::cbor::skip_item(raw, &mut o)
                                    .map_err(|e| format!("{e:?}"))?;
                            } else {
                                return Err("bad value major".into());
                            }
                        }
                        2 => {
                            // datum_option = [tag, payload]: 0 -> [0, h]; 1 -> [1, #6.24(bytes)].
                            let _ = read_array_header(raw, &mut o).map_err(|e| format!("{e:?}"))?;
                            let (dtag, _) = read_uint(raw, &mut o).map_err(|e| format!("{e:?}"))?;
                            if dtag == 1 {
                                // the inline payload is a #6.24 envelope; unwrap it back to the
                                // verbatim datum bytes.
                                let start = o;
                                let _ = read_tag(raw, &mut o).map_err(|e| format!("{e:?}"))?;
                                let (inner, _) =
                                    read_bytes(raw, &mut o).map_err(|e| format!("{e:?}"))?;
                                let _ = start;
                                datum_inline = Some(inner);
                            } else {
                                // hash form: skip the 32-byte hash.
                                let _ = read_bytes(raw, &mut o).map_err(|e| format!("{e:?}"))?;
                            }
                        }
                        3 => {
                            // script_ref = #6.24([type, script_bytes]); unwrap then parse the inner array.
                            let _ = read_tag(raw, &mut o).map_err(|e| format!("{e:?}"))?;
                            let (inner, _) =
                                read_bytes(raw, &mut o).map_err(|e| format!("{e:?}"))?;
                            let mut io = 0usize;
                            let _ = read_array_header(&inner, &mut io)
                                .map_err(|e| format!("{e:?}"))?;
                            let (stype, _) =
                                read_uint(&inner, &mut io).map_err(|e| format!("{e:?}"))?;
                            let (sbytes, _) =
                                read_bytes(&inner, &mut io).map_err(|e| format!("{e:?}"))?;
                            if stype == 0 {
                                script_native = Some(sbytes);
                            } else {
                                script_plutus = Some((stype, sbytes));
                            }
                        }
                        _ => return Err(format!("unexpected key {key}")),
                    }
                }
                Ok(ParsedTxOut {
                    address: address.ok_or("missing address")?,
                    coin,
                    datum_inline,
                    script_native,
                    script_plutus,
                })
            }
        }
    }
}
