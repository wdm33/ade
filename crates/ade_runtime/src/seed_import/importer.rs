// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN cardano-cli JSON UTxO importer (PHASE4-N-M-A S1).
//!
//! Sole authority `import_cardano_cli_json_utxo` for converting a
//! cardano-cli `query utxo --whole-utxo` JSON file into an Ade
//! canonical `(UTxOState, UtxoFingerprint)` pair.
//!
//! Honest scope (per cluster doc):
//! - Conway-era Babbage-shape outputs supported (map-form CBOR).
//! - Lovelace + multi-asset values.
//! - Inline datum (via `inlineDatumRaw` hex) + datum hash.
//! - Reference scripts → fail-fast `UnsupportedTxOutFeature`.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

use ade_codec::cbor::{
    canonical_width, write_array_header, write_bytes_canonical, write_map_header,
    write_uint_canonical, ContainerEncoding,
};
use ade_crypto::blake2b::blake2b_256;
use ade_ledger::utxo::{TxOut, UTxOState};
use ade_types::{tx::{Coin, TxIn}, Hash32};

use super::json::{parse_utxo_seed_json, RawUtxoEntry, RawValue, RawValueEntry};

/// Closed `UtxoFingerprint` newtype. Blake2b-256 over canonical
/// CBOR `map(N) [TxIn → TxOut_raw_bytes]` in BTreeMap order.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UtxoFingerprint(pub Hash32);

/// Closed JSON-seed error sum.
#[derive(Debug)]
pub enum JsonSeedError {
    /// IO failure reading the seed file.
    Io(io::ErrorKind),
    /// JSON parse failure.
    Json(serde_json::Error),
    /// TxIn key did not match `<64-hex>#<u16>` shape.
    BadTxInKey { key: String },
    /// Address bech32 decode failed or unknown HRP.
    BadAddress { addr: String },
    /// Datum hash hex was not 64 chars / valid hex.
    BadDatumHash { hex: String },
    /// inlineDatumRaw hex was not valid hex.
    BadInlineDatumRawHex { hex: String },
    /// Value field had a non-uint, non-asset-map entry.
    BadValueShape { detail: &'static str },
    /// A multi-asset entry's asset name was not valid hex.
    BadAssetNameHex { hex: String },
    /// A feature this slice deliberately does NOT support.
    /// Per [[feedback-shell-must-not-overstate-semantic-truth]],
    /// fail-fast rather than silently produce a partial seed.
    UnsupportedTxOutFeature { feature: &'static str },
}

impl From<serde_json::Error> for JsonSeedError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}

/// SOLE authority: import a cardano-cli JSON UTxO dump into
/// canonical Ade state. CN-SEED-01.
pub fn import_cardano_cli_json_utxo(
    path: &Path,
) -> Result<(UTxOState, UtxoFingerprint), JsonSeedError> {
    let bytes = fs::read(path).map_err(|e| JsonSeedError::Io(e.kind()))?;
    import_cardano_cli_json_utxo_from_bytes(&bytes)
}

/// In-memory variant (used by tests + by the file variant above).
/// Same single-authority guarantee; the file variant is a
/// one-line wrapper.
pub fn import_cardano_cli_json_utxo_from_bytes(
    bytes: &[u8],
) -> Result<(UTxOState, UtxoFingerprint), JsonSeedError> {
    let raw = parse_utxo_seed_json(bytes)?;
    let mut utxos: BTreeMap<TxIn, TxOut> = BTreeMap::new();
    for (key, entry) in raw {
        let tx_in = parse_txin_key(&key)?;
        let tx_out = build_canonical_tx_out(&entry)?;
        utxos.insert(tx_in, tx_out);
    }
    let state = UTxOState { utxos };
    let fingerprint = compute_utxo_fingerprint(&state);
    Ok((state, fingerprint))
}

/// Parse `"<64-hex>#<u16>"` → `TxIn`.
fn parse_txin_key(key: &str) -> Result<TxIn, JsonSeedError> {
    let (hash_hex, ix_str) = key.split_once('#').ok_or_else(|| {
        JsonSeedError::BadTxInKey { key: key.to_string() }
    })?;
    if hash_hex.len() != 64 {
        return Err(JsonSeedError::BadTxInKey { key: key.to_string() });
    }
    let mut hash_bytes = [0u8; 32];
    for i in 0..32 {
        let pair = &hash_hex[i * 2..i * 2 + 2];
        hash_bytes[i] = u8::from_str_radix(pair, 16)
            .map_err(|_| JsonSeedError::BadTxInKey { key: key.to_string() })?;
    }
    let index: u16 = ix_str
        .parse()
        .map_err(|_| JsonSeedError::BadTxInKey { key: key.to_string() })?;
    Ok(TxIn {
        tx_hash: Hash32(hash_bytes),
        index,
    })
}

/// Build a canonical `TxOut::AlonzoPlus` (Babbage-shape map) from a
/// parsed JSON entry. Honest scope: lovelace + multi-asset + inline
/// datum + datum hash; reference scripts fail-fast.
fn build_canonical_tx_out(entry: &RawUtxoEntry) -> Result<TxOut, JsonSeedError> {
    if entry.reference_script.is_some() {
        return Err(JsonSeedError::UnsupportedTxOutFeature {
            feature: "referenceScript",
        });
    }
    let address_bytes = decode_bech32_address(&entry.address)?;
    let (coin, multi_asset_opt) = encode_value(&entry.value)?;
    let datum_option_opt = encode_datum_option(entry)?;

    // Canonical Babbage map form: {0: address, 1: value, ?2:
    // datum_option, ?3: script_ref}. We don't support script_ref
    // in this slice, so the field count is 2 or 3.
    let mut raw: Vec<u8> = Vec::new();
    let mut count: u64 = 2;
    if datum_option_opt.is_some() {
        count += 1;
    }
    write_map_header(&mut raw, ContainerEncoding::Definite(count, canonical_width(count)));

    // Field 0: address.
    write_uint_canonical(&mut raw, 0);
    write_bytes_canonical(&mut raw, &address_bytes);

    // Field 1: value (uint coin if no MA; else [coin, ma_bytes]).
    write_uint_canonical(&mut raw, 1);
    if let Some(ma_bytes) = multi_asset_opt {
        write_array_header(&mut raw, ContainerEncoding::Definite(2, canonical_width(2)));
        write_uint_canonical(&mut raw, coin.0);
        raw.extend_from_slice(&ma_bytes);
    } else {
        write_uint_canonical(&mut raw, coin.0);
    }

    // Field 2: datum_option (already-canonical CBOR bytes if present).
    if let Some(d_bytes) = datum_option_opt {
        write_uint_canonical(&mut raw, 2);
        raw.extend_from_slice(&d_bytes);
    }

    Ok(TxOut::AlonzoPlus {
        raw,
        address: address_bytes,
        coin,
    })
}

/// Decode a bech32 address (`addr_test1...` / `addr1...` /
/// `stake_test1...` / `stake1...`) into raw address bytes (the
/// on-wire address representation).
fn decode_bech32_address(addr: &str) -> Result<Vec<u8>, JsonSeedError> {
    use bech32::primitives::decode::CheckedHrpstring;
    use bech32::Bech32;
    let hrpstring = CheckedHrpstring::new::<Bech32>(addr)
        .map_err(|_| JsonSeedError::BadAddress { addr: addr.to_string() })?;
    let hrp = hrpstring.hrp();
    let hrp_str = hrp.as_str();
    if !matches!(hrp_str, "addr" | "addr_test" | "stake" | "stake_test") {
        return Err(JsonSeedError::BadAddress { addr: addr.to_string() });
    }
    let bytes: Vec<u8> = hrpstring.byte_iter().collect();
    Ok(bytes)
}

/// Encode a parsed `value` field into `(coin, Option<multi_asset_bytes>)`.
/// The multi-asset bytes are canonical CBOR `map(N) [policy_id_bytes
/// → map(M) [asset_name_bytes → uint amount]]` in BTreeMap order.
fn encode_value(value: &RawValue) -> Result<(Coin, Option<Vec<u8>>), JsonSeedError> {
    let lovelace = match value.get("lovelace") {
        Some(RawValueEntry::Lovelace(n)) => *n,
        Some(_) => {
            return Err(JsonSeedError::BadValueShape {
                detail: "lovelace key was not a bare uint",
            });
        }
        None => 0,
    };
    let coin = Coin(lovelace);

    // Build the multi-asset sub-map (everything except "lovelace").
    let mut policies: BTreeMap<Vec<u8>, BTreeMap<Vec<u8>, u64>> = BTreeMap::new();
    for (key, entry) in value {
        if key == "lovelace" {
            continue;
        }
        let policy_bytes = decode_hex_string(key).ok_or(JsonSeedError::BadValueShape {
            detail: "policy id key was not valid hex",
        })?;
        let assets = match entry {
            RawValueEntry::Assets(m) => m,
            RawValueEntry::Lovelace(_) => {
                return Err(JsonSeedError::BadValueShape {
                    detail: "non-lovelace value was a bare uint",
                });
            }
        };
        let mut asset_map: BTreeMap<Vec<u8>, u64> = BTreeMap::new();
        for (asset_hex, amount) in assets {
            let asset_bytes = decode_hex_string(asset_hex).ok_or(
                JsonSeedError::BadAssetNameHex { hex: asset_hex.clone() },
            )?;
            asset_map.insert(asset_bytes, *amount);
        }
        policies.insert(policy_bytes, asset_map);
    }

    if policies.is_empty() {
        return Ok((coin, None));
    }

    // Canonical CBOR encode the policy map.
    let mut buf: Vec<u8> = Vec::new();
    let outer_count = policies.len() as u64;
    write_map_header(
        &mut buf,
        ContainerEncoding::Definite(outer_count, canonical_width(outer_count)),
    );
    for (policy, assets) in &policies {
        write_bytes_canonical(&mut buf, policy);
        let inner_count = assets.len() as u64;
        write_map_header(
            &mut buf,
            ContainerEncoding::Definite(inner_count, canonical_width(inner_count)),
        );
        for (asset, amount) in assets {
            write_bytes_canonical(&mut buf, asset);
            write_uint_canonical(&mut buf, *amount);
        }
    }
    Ok((coin, Some(buf)))
}

/// Encode the datum field of a JSON UTxO entry into the Babbage
/// `datum_option` CBOR shape:
/// - `[0, hash32]` for a datum hash.
/// - `[1, tagged_cbor_bytes(inline_datum_raw)]` for an inline
///   datum. The inline datum is wrapped in CBOR tag 24 per the
///   Babbage spec.
fn encode_datum_option(entry: &RawUtxoEntry) -> Result<Option<Vec<u8>>, JsonSeedError> {
    // Inline datum takes precedence over hash if both are present.
    if let Some(inline_hex) = &entry.inline_datum_raw {
        let inner_bytes = decode_hex_string(inline_hex).ok_or(
            JsonSeedError::BadInlineDatumRawHex { hex: inline_hex.clone() },
        )?;
        let mut buf: Vec<u8> = Vec::new();
        // [1, tag(24, bytes(inner))]
        write_array_header(&mut buf, ContainerEncoding::Definite(2, canonical_width(2)));
        write_uint_canonical(&mut buf, 1);
        // CBOR tag 24 = major type 6, value 24 → 0xd8 0x18
        buf.push(0xd8);
        buf.push(0x18);
        write_bytes_canonical(&mut buf, &inner_bytes);
        return Ok(Some(buf));
    }
    if let Some(hash_hex) = &entry.datumhash {
        if hash_hex.len() != 64 {
            return Err(JsonSeedError::BadDatumHash { hex: hash_hex.clone() });
        }
        let bytes = decode_hex_string(hash_hex)
            .ok_or(JsonSeedError::BadDatumHash { hex: hash_hex.clone() })?;
        let mut buf: Vec<u8> = Vec::new();
        // [0, bytes(32)]
        write_array_header(&mut buf, ContainerEncoding::Definite(2, canonical_width(2)));
        write_uint_canonical(&mut buf, 0);
        write_bytes_canonical(&mut buf, &bytes);
        return Ok(Some(buf));
    }
    Ok(None)
}

/// Compute the canonical `UtxoFingerprint` for a `UTxOState`:
/// Blake2b-256 over canonical CBOR `map(N) [TxIn → TxOut_raw]` in
/// BTreeMap order.
fn compute_utxo_fingerprint(state: &UTxOState) -> UtxoFingerprint {
    let mut buf: Vec<u8> = Vec::new();
    let count = state.utxos.len() as u64;
    write_map_header(
        &mut buf,
        ContainerEncoding::Definite(count, canonical_width(count)),
    );
    for (tx_in, tx_out) in &state.utxos {
        // Key: canonical [tx_hash, index].
        write_array_header(&mut buf, ContainerEncoding::Definite(2, canonical_width(2)));
        write_bytes_canonical(&mut buf, &tx_in.tx_hash.0);
        write_uint_canonical(&mut buf, tx_in.index as u64);
        // Value: the canonical raw bytes of the TxOut.
        let value_bytes: Vec<u8> = match tx_out {
            TxOut::AlonzoPlus { raw, .. } => raw.clone(),
            TxOut::Byron { address, coin } => {
                let mut b = Vec::new();
                write_array_header(&mut b, ContainerEncoding::Definite(2, canonical_width(2)));
                write_bytes_canonical(&mut b, address.as_bytes());
                write_uint_canonical(&mut b, coin.0);
                b
            }
            TxOut::ShelleyMary { address, value } => {
                let mut b = Vec::new();
                write_array_header(&mut b, ContainerEncoding::Definite(2, canonical_width(2)));
                write_bytes_canonical(&mut b, address);
                write_uint_canonical(&mut b, value.coin.0);
                b
            }
        };
        buf.extend_from_slice(&value_bytes);
    }
    UtxoFingerprint(blake2b_256(&buf))
}

/// Decode a lowercase / mixed-case hex string into raw bytes.
fn decode_hex_string(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    let bytes = s.as_bytes();
    for i in 0..s.len() / 2 {
        let hi = hex_nibble(bytes[i * 2])?;
        let lo = hex_nibble(bytes[i * 2 + 1])?;
        out.push((hi << 4) | lo);
    }
    Some(out)
}

fn hex_nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    const MINIMAL_TWO_ENTRY: &str = r#"{
        "0000000000000000000000000000000000000000000000000000000000000001#0": {
            "address": "addr_test1vq0ast4z2dypfrl9kg2c0garrcy085w78dls8xsx954x34cmgvp2u",
            "value": { "lovelace": 1000000 }
        },
        "0000000000000000000000000000000000000000000000000000000000000002#3": {
            "address": "addr_test1vq0ast4z2dypfrl9kg2c0garrcy085w78dls8xsx954x34cmgvp2u",
            "value": { "lovelace": 2000000 }
        }
    }"#;

    const INLINE_DATUM_ENTRY: &str = r#"{
        "0000000000000000000000000000000000000000000000000000000000000003#0": {
            "address": "addr_test1wp97ley0p7xqksmh6tq3c6v8depl9jpfvnkk68d29fwznmcmlpuqk",
            "inlineDatumRaw": "581c7f5055adc0fddd13ee66d565d1a2ae552be4a9fcdd6835613fbb872f",
            "value": { "lovelace": 10000000 }
        }
    }"#;

    const REF_SCRIPT_ENTRY: &str = r#"{
        "0000000000000000000000000000000000000000000000000000000000000004#0": {
            "address": "addr_test1vq0ast4z2dypfrl9kg2c0garrcy085w78dls8xsx954x34cmgvp2u",
            "referenceScript": { "anything": true },
            "value": { "lovelace": 100 }
        }
    }"#;

    #[test]
    fn utxo_seed_parses_minimal_two_entry_fixture() {
        let (state, fp) =
            import_cardano_cli_json_utxo_from_bytes(MINIMAL_TWO_ENTRY.as_bytes())
                .expect("import");
        assert_eq!(state.utxos.len(), 2);
        // Fingerprint is non-zero and deterministic-shaped.
        assert_ne!(fp.0 .0, [0u8; 32]);
    }

    #[test]
    fn utxo_seed_two_imports_byte_identical() {
        let (s1, f1) =
            import_cardano_cli_json_utxo_from_bytes(MINIMAL_TWO_ENTRY.as_bytes())
                .expect("a");
        let (s2, f2) =
            import_cardano_cli_json_utxo_from_bytes(MINIMAL_TWO_ENTRY.as_bytes())
                .expect("b");
        assert_eq!(s1, s2);
        assert_eq!(f1, f2);
    }

    #[test]
    fn utxo_seed_btree_order_independent_of_json_order() {
        let order_a = r#"{
            "0000000000000000000000000000000000000000000000000000000000000002#0": {
                "address": "addr_test1vq0ast4z2dypfrl9kg2c0garrcy085w78dls8xsx954x34cmgvp2u",
                "value": { "lovelace": 200 }
            },
            "0000000000000000000000000000000000000000000000000000000000000001#0": {
                "address": "addr_test1vq0ast4z2dypfrl9kg2c0garrcy085w78dls8xsx954x34cmgvp2u",
                "value": { "lovelace": 100 }
            }
        }"#;
        let order_b = r#"{
            "0000000000000000000000000000000000000000000000000000000000000001#0": {
                "address": "addr_test1vq0ast4z2dypfrl9kg2c0garrcy085w78dls8xsx954x34cmgvp2u",
                "value": { "lovelace": 100 }
            },
            "0000000000000000000000000000000000000000000000000000000000000002#0": {
                "address": "addr_test1vq0ast4z2dypfrl9kg2c0garrcy085w78dls8xsx954x34cmgvp2u",
                "value": { "lovelace": 200 }
            }
        }"#;
        let (_, fa) = import_cardano_cli_json_utxo_from_bytes(order_a.as_bytes()).unwrap();
        let (_, fb) = import_cardano_cli_json_utxo_from_bytes(order_b.as_bytes()).unwrap();
        assert_eq!(fa, fb);
    }

    #[test]
    fn utxo_seed_rejects_unparseable_json() {
        let err = import_cardano_cli_json_utxo_from_bytes(b"not json").expect_err("must err");
        assert!(matches!(err, JsonSeedError::Json(_)));
    }

    #[test]
    fn utxo_seed_rejects_bad_txin_key() {
        let bad = r#"{
            "not_a_valid_key": {
                "address": "addr_test1vq0ast4z2dypfrl9kg2c0garrcy085w78dls8xsx954x34cmgvp2u",
                "value": { "lovelace": 1 }
            }
        }"#;
        let err = import_cardano_cli_json_utxo_from_bytes(bad.as_bytes()).expect_err("err");
        assert!(matches!(err, JsonSeedError::BadTxInKey { .. }));
    }

    #[test]
    fn utxo_seed_rejects_bad_address() {
        let bad = r#"{
            "0000000000000000000000000000000000000000000000000000000000000001#0": {
                "address": "not_a_bech32_address",
                "value": { "lovelace": 1 }
            }
        }"#;
        let err = import_cardano_cli_json_utxo_from_bytes(bad.as_bytes()).expect_err("err");
        assert!(matches!(err, JsonSeedError::BadAddress { .. }));
    }

    #[test]
    fn utxo_seed_inline_datum_entry_round_trips() {
        let (state, fp) =
            import_cardano_cli_json_utxo_from_bytes(INLINE_DATUM_ENTRY.as_bytes())
                .expect("import");
        assert_eq!(state.utxos.len(), 1);
        // The fingerprint differs from a lovelace-only entry at the
        // same TxIn (datum_option bytes change the canonical CBOR).
        let no_datum = r#"{
            "0000000000000000000000000000000000000000000000000000000000000003#0": {
                "address": "addr_test1wp97ley0p7xqksmh6tq3c6v8depl9jpfvnkk68d29fwznmcmlpuqk",
                "value": { "lovelace": 10000000 }
            }
        }"#;
        let (_, fp_no_datum) =
            import_cardano_cli_json_utxo_from_bytes(no_datum.as_bytes()).expect("no datum");
        assert_ne!(fp, fp_no_datum);
    }

    #[test]
    fn utxo_seed_reference_script_fails_fast() {
        let err = import_cardano_cli_json_utxo_from_bytes(REF_SCRIPT_ENTRY.as_bytes())
            .expect_err("must fail");
        match err {
            JsonSeedError::UnsupportedTxOutFeature { feature } => {
                assert_eq!(feature, "referenceScript");
            }
            other => panic!("expected UnsupportedTxOutFeature, got {other:?}"),
        }
    }

    #[test]
    fn utxo_seed_canonical_txout_address_extracted() {
        let (state, _) =
            import_cardano_cli_json_utxo_from_bytes(MINIMAL_TWO_ENTRY.as_bytes())
                .expect("import");
        let any = state.utxos.values().next().unwrap();
        // Address bytes are non-empty (bech32 decoded properly).
        assert!(!any.address_bytes().is_empty());
    }
}
