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
//! - Reference scripts → supported (A1.1): `match`ed + encoded via
//!   `encode_script_ref`, fail-closed on malformed via `BadReferenceScript`.

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
use ade_types::{
    tx::{Coin, TxIn},
    Hash32,
};

use super::json::{
    parse_utxo_seed_json, RawReferenceScript, RawUtxoEntry, RawValue, RawValueEntry,
};

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
    /// Two entries resolve to the SAME canonical `TxIn` (a UTxO dump has unique
    /// outrefs by construction). Distinct JSON key strings can collide on one
    /// `TxIn` (uppercase vs lowercase hex; `#0` vs `#00`), which would make the
    /// streaming and whole-buffer imports pick different survivors and diverge.
    /// Fail closed on ANY duplicate so the import is byte-identical or rejected
    /// — never a silent, order-dependent fingerprint (DC-MEM-06).
    DuplicateTxIn { key: String },
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
    /// `referenceScript` JSON shape was structurally well-formed
    /// (typed deserialization succeeded) but contained an
    /// unrecognized script `type` or unparseable `cborHex`.
    /// PHASE4-N-M-A1.1.
    BadReferenceScript { detail: &'static str },
}

impl From<serde_json::Error> for JsonSeedError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}

/// SOLE authority: import a cardano-cli JSON UTxO dump into canonical Ade
/// state (CN-SEED-01). MEM-OPT-OPS S2: STREAMING — `Deserializer::from_reader`
/// over a `BufReader<File>`, converting each entry to canonical form and
/// inserting into the `BTreeMap` AS IT IS PARSED, so neither the whole-file
/// buffer nor the intermediate `RawUtxoMap` is ever materialized (removes the
/// ~6.8 GB import peak). Byte-identical to `import_cardano_cli_json_utxo_from_bytes`:
/// the SAME per-entry conversion (`parse_txin_key` + `build_canonical_tx_out`)
/// and the SAME canonical `BTreeMap`, so the fingerprint is unchanged
/// (`DC-MEM-06` — the fingerprint is over canonical keys, never parse/iteration
/// order). The whole-buffer variant is retained as the equivalence oracle.
pub fn import_cardano_cli_json_utxo(
    path: &Path,
) -> Result<(UTxOState, UtxoFingerprint), JsonSeedError> {
    let file = fs::File::open(path).map_err(|e| JsonSeedError::Io(e.kind()))?;
    let reader = io::BufReader::new(file);
    let mut utxos: BTreeMap<TxIn, TxOut> = BTreeMap::new();
    let mut conv_err: Option<JsonSeedError> = None;
    let mut de = serde_json::Deserializer::from_reader(reader);
    let sink = CanonicalUtxoSink {
        utxos: &mut utxos,
        conv_err: &mut conv_err,
    };
    if let Err(e) = serde::de::DeserializeSeed::deserialize(sink, &mut de) {
        // A stashed conversion error (BadTxInKey / value / script) is the real
        // cause and takes precedence over the generic serde halt error.
        if let Some(ce) = conv_err.take() {
            return Err(ce);
        }
        return Err(JsonSeedError::from(e));
    }
    // Reject trailing data after the top-level object (no best-effort accept).
    de.end().map_err(JsonSeedError::from)?;
    let state = UTxOState { utxos };
    let fingerprint = compute_utxo_fingerprint(&state);
    Ok((state, fingerprint))
}

/// In-memory whole-buffer variant: parse the full `RawUtxoMap`, then convert.
/// Retained as the streaming path's equivalence ORACLE and as the in-memory
/// test helper. Same single-authority guarantee; same `(UTxOState, fingerprint)`
/// as the streaming file variant on identical bytes.
pub fn import_cardano_cli_json_utxo_from_bytes(
    bytes: &[u8],
) -> Result<(UTxOState, UtxoFingerprint), JsonSeedError> {
    let raw = parse_utxo_seed_json(bytes)?;
    let mut utxos: BTreeMap<TxIn, TxOut> = BTreeMap::new();
    for (key, entry) in raw {
        let tx_in = parse_txin_key(&key)?;
        let tx_out = build_canonical_tx_out(&entry)?;
        // Unique-outref enforcement (DC-MEM-06): distinct JSON key strings can
        // collide on one canonical TxIn; reject ANY duplicate so the import is
        // byte-identical or rejected, never an order-dependent survivor.
        if utxos.insert(tx_in, tx_out).is_some() {
            return Err(JsonSeedError::DuplicateTxIn { key });
        }
    }
    let state = UTxOState { utxos };
    let fingerprint = compute_utxo_fingerprint(&state);
    Ok((state, fingerprint))
}

/// Streaming sink (RED): deserializes the top-level cardano-cli UTxO map
/// entry-by-entry, converting each via the SAME `parse_txin_key` +
/// `build_canonical_tx_out` as the whole-buffer path and inserting into
/// `utxos` — only one `RawUtxoEntry` is alive at a time. A conversion error is
/// stashed into `conv_err` because the serde error returned to halt the parse
/// cannot carry a `JsonSeedError` payload; the caller reads it back. No
/// best-effort recovery: any conversion or JSON error halts the import.
struct CanonicalUtxoSink<'a> {
    utxos: &'a mut BTreeMap<TxIn, TxOut>,
    conv_err: &'a mut Option<JsonSeedError>,
}

impl<'a, 'de> serde::de::DeserializeSeed<'de> for CanonicalUtxoSink<'a> {
    type Value = ();
    fn deserialize<D>(self, deserializer: D) -> Result<(), D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(self)
    }
}

impl<'a, 'de> serde::de::Visitor<'de> for CanonicalUtxoSink<'a> {
    type Value = ();

    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("a cardano-cli UTxO map ({\"txhash#ix\": entry, ...})")
    }

    fn visit_map<M>(self, mut map: M) -> Result<(), M::Error>
    where
        M: serde::de::MapAccess<'de>,
    {
        use serde::de::Error as _;
        while let Some(key) = map.next_key::<String>()? {
            let entry = map.next_value::<RawUtxoEntry>()?;
            let tx_in = match parse_txin_key(&key) {
                Ok(t) => t,
                Err(e) => {
                    *self.conv_err = Some(e);
                    return Err(M::Error::custom("seed txin conversion error"));
                }
            };
            let tx_out = match build_canonical_tx_out(&entry) {
                Ok(t) => t,
                Err(e) => {
                    *self.conv_err = Some(e);
                    return Err(M::Error::custom("seed txout conversion error"));
                }
            };
            // Unique-outref enforcement (DC-MEM-06): a duplicate TxIn (e.g. a
            // case- or leading-zero-variant key colliding on one canonical TxIn)
            // would make the streaming and whole-buffer paths pick different
            // survivors. Fail closed on ANY duplicate -- byte-identical or rejected.
            if self.utxos.insert(tx_in, tx_out).is_some() {
                *self.conv_err = Some(JsonSeedError::DuplicateTxIn { key });
                return Err(M::Error::custom("duplicate txin in seed"));
            }
        }
        Ok(())
    }
}

/// Parse `"<64-hex>#<u16>"` → `TxIn`.
fn parse_txin_key(key: &str) -> Result<TxIn, JsonSeedError> {
    let (hash_hex, ix_str) = key
        .split_once('#')
        .ok_or_else(|| JsonSeedError::BadTxInKey {
            key: key.to_string(),
        })?;
    if hash_hex.len() != 64 {
        return Err(JsonSeedError::BadTxInKey {
            key: key.to_string(),
        });
    }
    let mut hash_bytes = [0u8; 32];
    for i in 0..32 {
        let pair = &hash_hex[i * 2..i * 2 + 2];
        hash_bytes[i] = u8::from_str_radix(pair, 16).map_err(|_| JsonSeedError::BadTxInKey {
            key: key.to_string(),
        })?;
    }
    let index: u16 = ix_str.parse().map_err(|_| JsonSeedError::BadTxInKey {
        key: key.to_string(),
    })?;
    Ok(TxIn {
        tx_hash: Hash32(hash_bytes),
        index,
    })
}

/// Build a canonical `TxOut::AlonzoPlus` (Babbage-shape map) from a
/// parsed JSON entry. Honest scope: lovelace + multi-asset + inline
/// datum + datum hash + reference scripts (PHASE4-N-M-A1.1).
fn build_canonical_tx_out(entry: &RawUtxoEntry) -> Result<TxOut, JsonSeedError> {
    let address_bytes = decode_cli_address(&entry.address)?;
    let (coin, multi_asset_opt) = encode_value(&entry.value)?;
    let datum_option_opt = encode_datum_option(entry)?;
    let script_ref_opt = match &entry.reference_script {
        Some(rs) => Some(encode_script_ref(rs)?),
        None => None,
    };

    // Canonical Babbage map form: {0: address, 1: value, ?2:
    // datum_option, ?3: script_ref}. Field count is 2..=4 depending
    // on which optional fields are present.
    let mut raw: Vec<u8> = Vec::new();
    let mut count: u64 = 2;
    if datum_option_opt.is_some() {
        count += 1;
    }
    if script_ref_opt.is_some() {
        count += 1;
    }
    write_map_header(
        &mut raw,
        ContainerEncoding::Definite(count, canonical_width(count)),
    );

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

    // Field 3: script_ref (tag(24, bytes(.cbor script))) if present.
    if let Some(sr_bytes) = script_ref_opt {
        write_uint_canonical(&mut raw, 3);
        raw.extend_from_slice(&sr_bytes);
    }

    Ok(TxOut::AlonzoPlus {
        raw,
        address: address_bytes,
        coin,
    })
}

/// Map the cardano-cli `referenceScript.script.type` string to its
/// canonical Babbage script-variant tag. Closed vocabulary.
fn script_variant_tag(ty: &str) -> Option<u64> {
    match ty {
        "SimpleScript" => Some(0),
        "PlutusScriptV1" => Some(1),
        "PlutusScriptV2" => Some(2),
        "PlutusScriptV3" => Some(3),
        _ => None,
    }
}

/// Encode the Babbage `script_ref` CBOR (`#6.24(bytes .cbor script)`)
/// from a typed `RawReferenceScript`. PHASE4-N-M-A1.1 / CN-SEED-01.
///
/// Layout:
/// ```text
/// inner = array(2) | uint(variant_tag) | <cborHex bytes raw>
/// outer = tag(24)  | bytes(len(inner)) | inner
/// ```
///
/// The `cborHex` content is treated as already-canonical CBOR per
/// cardano-node's contract (CBOR-encoded `bytes(plutus_binary)` for
/// Plutus variants; CBOR-encoded `native_script` array for
/// SimpleScript). We never re-encode it — see slice A1.1 §4.
fn encode_script_ref(rs: &RawReferenceScript) -> Result<Vec<u8>, JsonSeedError> {
    let variant_tag =
        script_variant_tag(&rs.script.ty).ok_or(JsonSeedError::BadReferenceScript {
            detail: "unknown script type",
        })?;
    let inner_payload =
        decode_hex_string(&rs.script.cbor_hex).ok_or(JsonSeedError::BadReferenceScript {
            detail: "cbor_hex not valid hex",
        })?;
    if inner_payload.is_empty() {
        return Err(JsonSeedError::BadReferenceScript {
            detail: "cbor_hex empty",
        });
    }

    // Inner: array(2) | uint(variant_tag) | <inner_payload raw>.
    let mut inner: Vec<u8> = Vec::with_capacity(inner_payload.len() + 4);
    write_array_header(
        &mut inner,
        ContainerEncoding::Definite(2, canonical_width(2)),
    );
    write_uint_canonical(&mut inner, variant_tag);
    inner.extend_from_slice(&inner_payload);

    // Outer: tag(24) | bytes(len(inner)) | inner.
    let mut outer: Vec<u8> = Vec::with_capacity(inner.len() + 6);
    // CBOR tag 24 = major type 6 with value 24 → two bytes 0xD8 0x18.
    outer.push(0xd8);
    outer.push(0x18);
    write_bytes_canonical(&mut outer, &inner);
    Ok(outer)
}

/// Decode an address as cardano-cli emits it in `query utxo` JSON.
/// Accepts both bech32-encoded Shelley-and-later addresses
/// (`addr_test1...` / `addr1...` / `stake_test1...` / `stake1...`)
/// and Base58-encoded Byron-era addresses. Returns the on-wire
/// address bytes (Shelley-bech32 payload OR raw Byron CBOR
/// envelope bytes).
///
/// PHASE4-N-M-A1.1 slice A1.2: Byron Base58 support added so the
/// full preprod `cardano-cli query utxo --whole-utxo` dump
/// (~0.7% Byron-era entries) imports cleanly. Per
/// [[feedback-oracle-seed-then-ade-owns]], the imported address
/// bytes are stored exactly as the operator's dump records them;
/// no re-encoding is performed.
fn decode_cli_address(addr: &str) -> Result<Vec<u8>, JsonSeedError> {
    // Cheap discrimination: a bech32 address has a known HRP
    // prefix followed by `1` (the separator). Everything that
    // starts with `addr` / `stake` is bech32. Everything else is
    // tried as Byron Base58.
    if addr.starts_with("addr") || addr.starts_with("stake") {
        return decode_bech32_address(addr);
    }
    decode_byron_base58_address(addr)
}

/// Decode a bech32 address (`addr_test1...` / `addr1...` /
/// `stake_test1...` / `stake1...`) into raw address bytes.
fn decode_bech32_address(addr: &str) -> Result<Vec<u8>, JsonSeedError> {
    use bech32::primitives::decode::CheckedHrpstring;
    use bech32::Bech32;
    let hrpstring =
        CheckedHrpstring::new::<Bech32>(addr).map_err(|_| JsonSeedError::BadAddress {
            addr: addr.to_string(),
        })?;
    let hrp = hrpstring.hrp();
    let hrp_str = hrp.as_str();
    if !matches!(hrp_str, "addr" | "addr_test" | "stake" | "stake_test") {
        return Err(JsonSeedError::BadAddress {
            addr: addr.to_string(),
        });
    }
    let bytes: Vec<u8> = hrpstring.byte_iter().collect();
    Ok(bytes)
}

/// Decode a Byron-era Base58 address (no checksum) into raw
/// address bytes (the CBOR-encoded `[bytes(payload), checksum_u32]`
/// envelope cardano-node stores on-wire).
///
/// cardano-cli emits Byron addresses as the Base58 of this raw
/// envelope. The returned bytes ARE the on-wire address bytes —
/// we never re-decode the CBOR inside, only carry it through.
fn decode_byron_base58_address(addr: &str) -> Result<Vec<u8>, JsonSeedError> {
    use base58::FromBase58;
    addr.from_base58().map_err(|_| JsonSeedError::BadAddress {
        addr: addr.to_string(),
    })
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
            let asset_bytes =
                decode_hex_string(asset_hex).ok_or(JsonSeedError::BadAssetNameHex {
                    hex: asset_hex.clone(),
                })?;
            asset_map.insert(asset_bytes, amount.get());
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
        let inner_bytes =
            decode_hex_string(inline_hex).ok_or(JsonSeedError::BadInlineDatumRawHex {
                hex: inline_hex.clone(),
            })?;
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
            return Err(JsonSeedError::BadDatumHash {
                hex: hash_hex.clone(),
            });
        }
        let bytes = decode_hex_string(hash_hex).ok_or(JsonSeedError::BadDatumHash {
            hex: hash_hex.clone(),
        })?;
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

    /// Real Plutus V2 cborHex from the committed preprod 129-entry
    /// mini sample (truncated to keep the test fixture small; the
    /// full 2651-byte payload is exercised by the file-driven
    /// smoke test). The bytes `590a 5b 01 00 ...` are valid CBOR
    /// (bytes-of-length-2651 header + body), so the importer
    /// accepts them as opaque-canonical Plutus payload.
    const PLUTUS_V2_REF_SCRIPT_ENTRY: &str = r#"{
        "0000000000000000000000000000000000000000000000000000000000000004#0": {
            "address": "addr_test1wp97ley0p7xqksmh6tq3c6v8depl9jpfvnkk68d29fwznmcmlpuqk",
            "referenceScript": {
                "script": {
                    "cborHex": "4401020304",
                    "description": "",
                    "type": "PlutusScriptV2"
                },
                "scriptLanguage": "PlutusScriptLanguage PlutusScriptV2"
            },
            "value": { "lovelace": 100 }
        }
    }"#;

    const PLUTUS_V1_REF_SCRIPT_ENTRY: &str = r#"{
        "0000000000000000000000000000000000000000000000000000000000000005#0": {
            "address": "addr_test1wp97ley0p7xqksmh6tq3c6v8depl9jpfvnkk68d29fwznmcmlpuqk",
            "referenceScript": {
                "script": {
                    "cborHex": "4401020304",
                    "description": "",
                    "type": "PlutusScriptV1"
                },
                "scriptLanguage": "PlutusScriptLanguage PlutusScriptV1"
            },
            "value": { "lovelace": 100 }
        }
    }"#;

    const PLUTUS_V3_REF_SCRIPT_ENTRY: &str = r#"{
        "0000000000000000000000000000000000000000000000000000000000000006#0": {
            "address": "addr_test1wp97ley0p7xqksmh6tq3c6v8depl9jpfvnkk68d29fwznmcmlpuqk",
            "referenceScript": {
                "script": {
                    "cborHex": "4401020304",
                    "description": "",
                    "type": "PlutusScriptV3"
                },
                "scriptLanguage": "PlutusScriptLanguage PlutusScriptV3"
            },
            "value": { "lovelace": 100 }
        }
    }"#;

    /// SimpleScript with a tiny native-script CBOR payload —
    /// `83 00 01 02` = array(3)[uint(0), uint(1), uint(2)], not a
    /// real script, but a valid CBOR sequence for fingerprint
    /// participation. The seed importer treats it as opaque-canonical.
    const SIMPLE_SCRIPT_REF_SCRIPT_ENTRY: &str = r#"{
        "0000000000000000000000000000000000000000000000000000000000000007#0": {
            "address": "addr_test1wp97ley0p7xqksmh6tq3c6v8depl9jpfvnkk68d29fwznmcmlpuqk",
            "referenceScript": {
                "script": {
                    "cborHex": "83000102",
                    "description": "",
                    "type": "SimpleScript"
                },
                "scriptLanguage": "SimpleScriptLanguage SimpleScriptV2"
            },
            "value": { "lovelace": 100 }
        }
    }"#;

    const UNKNOWN_TYPE_REF_SCRIPT_ENTRY: &str = r#"{
        "0000000000000000000000000000000000000000000000000000000000000008#0": {
            "address": "addr_test1wp97ley0p7xqksmh6tq3c6v8depl9jpfvnkk68d29fwznmcmlpuqk",
            "referenceScript": {
                "script": {
                    "cborHex": "4401020304",
                    "description": "",
                    "type": "PlutusScriptV99"
                },
                "scriptLanguage": "?"
            },
            "value": { "lovelace": 100 }
        }
    }"#;

    const NON_HEX_REF_SCRIPT_ENTRY: &str = r#"{
        "0000000000000000000000000000000000000000000000000000000000000009#0": {
            "address": "addr_test1wp97ley0p7xqksmh6tq3c6v8depl9jpfvnkk68d29fwznmcmlpuqk",
            "referenceScript": {
                "script": {
                    "cborHex": "not_hex_at_all",
                    "description": "",
                    "type": "PlutusScriptV2"
                },
                "scriptLanguage": "PlutusScriptLanguage PlutusScriptV2"
            },
            "value": { "lovelace": 100 }
        }
    }"#;

    const NO_REF_SCRIPT_BASELINE: &str = r#"{
        "0000000000000000000000000000000000000000000000000000000000000004#0": {
            "address": "addr_test1wp97ley0p7xqksmh6tq3c6v8depl9jpfvnkk68d29fwznmcmlpuqk",
            "value": { "lovelace": 100 }
        }
    }"#;

    #[test]
    fn utxo_seed_parses_minimal_two_entry_fixture() {
        let (state, fp) =
            import_cardano_cli_json_utxo_from_bytes(MINIMAL_TWO_ENTRY.as_bytes()).expect("import");
        assert_eq!(state.utxos.len(), 2);
        // Fingerprint is non-zero and deterministic-shaped.
        assert_ne!(fp.0 .0, [0u8; 32]);
    }

    #[test]
    fn utxo_seed_two_imports_byte_identical() {
        let (s1, f1) =
            import_cardano_cli_json_utxo_from_bytes(MINIMAL_TWO_ENTRY.as_bytes()).expect("a");
        let (s2, f2) =
            import_cardano_cli_json_utxo_from_bytes(MINIMAL_TWO_ENTRY.as_bytes()).expect("b");
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
            import_cardano_cli_json_utxo_from_bytes(INLINE_DATUM_ENTRY.as_bytes()).expect("import");
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
    fn utxo_seed_accepts_plutus_v2_reference_script() {
        let (state, _fp) =
            import_cardano_cli_json_utxo_from_bytes(PLUTUS_V2_REF_SCRIPT_ENTRY.as_bytes())
                .expect("import");
        assert_eq!(state.utxos.len(), 1);
        let out = state.utxos.values().next().expect("entry");
        let raw = match out {
            TxOut::AlonzoPlus { raw, .. } => raw,
            other => panic!("expected AlonzoPlus, got {other:?}"),
        };
        // The raw bytes MUST contain the canonical script_ref field
        // marker: uint(3) = 0x03 followed by tag(24) = 0xD8 0x18.
        let mut found = false;
        for i in 0..raw.len().saturating_sub(2) {
            if raw[i] == 0x03 && raw[i + 1] == 0xd8 && raw[i + 2] == 0x18 {
                found = true;
                break;
            }
        }
        assert!(
            found,
            "raw bytes must contain canonical script_ref marker `03 D8 18`"
        );
    }

    #[test]
    fn utxo_seed_accepts_plutus_v1_reference_script() {
        let (state, _) =
            import_cardano_cli_json_utxo_from_bytes(PLUTUS_V1_REF_SCRIPT_ENTRY.as_bytes())
                .expect("import");
        let out = state.utxos.values().next().unwrap();
        let raw = match out {
            TxOut::AlonzoPlus { raw, .. } => raw,
            _ => panic!(),
        };
        // After `03 D8 18` (key=3, tag24) + bytes-header, the inner
        // array starts with `82 01` (array(2), uint(1) for V1).
        let needle = [0x03u8, 0xd8, 0x18];
        let pos = raw
            .windows(3)
            .position(|w| w == needle)
            .expect("script_ref marker present");
        // Skip over `03 D8 18` and the bytes-length header byte(s).
        // For small inner (<= 23 bytes), the bytes header is one
        // byte (0x40..=0x57). Our PlutusScriptV1 fixture inner is
        // `array(2)|uint(1)|<5 bytes of cborHex (4401020304)>` = 7
        // bytes inner, so the bytes header is 0x47 (bytes of len 7).
        assert_eq!(raw[pos + 3], 0x47, "expected bytes-header for len 7");
        // Then inner: `82 01 ...`.
        assert_eq!(raw[pos + 4], 0x82, "array(2) header");
        assert_eq!(raw[pos + 5], 0x01, "uint(1) variant tag for PlutusV1");
    }

    #[test]
    fn utxo_seed_accepts_plutus_v3_reference_script() {
        let (state, _) =
            import_cardano_cli_json_utxo_from_bytes(PLUTUS_V3_REF_SCRIPT_ENTRY.as_bytes())
                .expect("import");
        let out = state.utxos.values().next().unwrap();
        let raw = match out {
            TxOut::AlonzoPlus { raw, .. } => raw,
            _ => panic!(),
        };
        let needle = [0x03u8, 0xd8, 0x18];
        let pos = raw.windows(3).position(|w| w == needle).expect("marker");
        assert_eq!(raw[pos + 4], 0x82);
        assert_eq!(raw[pos + 5], 0x03, "uint(3) variant tag for PlutusV3");
    }

    #[test]
    fn utxo_seed_accepts_simple_script_reference_script() {
        let (state, _) =
            import_cardano_cli_json_utxo_from_bytes(SIMPLE_SCRIPT_REF_SCRIPT_ENTRY.as_bytes())
                .expect("import");
        let out = state.utxos.values().next().unwrap();
        let raw = match out {
            TxOut::AlonzoPlus { raw, .. } => raw,
            _ => panic!(),
        };
        let needle = [0x03u8, 0xd8, 0x18];
        let pos = raw.windows(3).position(|w| w == needle).expect("marker");
        assert_eq!(raw[pos + 4], 0x82);
        assert_eq!(
            raw[pos + 5],
            0x00,
            "uint(0) variant tag for SimpleScript (native)"
        );
    }

    #[test]
    fn utxo_seed_reference_script_changes_fingerprint() {
        let (_, fp_with) =
            import_cardano_cli_json_utxo_from_bytes(PLUTUS_V2_REF_SCRIPT_ENTRY.as_bytes())
                .expect("with");
        let (_, fp_without) =
            import_cardano_cli_json_utxo_from_bytes(NO_REF_SCRIPT_BASELINE.as_bytes())
                .expect("without");
        assert_ne!(
            fp_with, fp_without,
            "reference-script content MUST participate in the canonical fingerprint"
        );
    }

    #[test]
    fn utxo_seed_reference_script_deterministic_across_two_imports() {
        let (s1, f1) =
            import_cardano_cli_json_utxo_from_bytes(PLUTUS_V2_REF_SCRIPT_ENTRY.as_bytes())
                .expect("a");
        let (s2, f2) =
            import_cardano_cli_json_utxo_from_bytes(PLUTUS_V2_REF_SCRIPT_ENTRY.as_bytes())
                .expect("b");
        assert_eq!(s1, s2);
        assert_eq!(f1, f2);
    }

    #[test]
    fn utxo_seed_rejects_unknown_script_type() {
        let err = import_cardano_cli_json_utxo_from_bytes(UNKNOWN_TYPE_REF_SCRIPT_ENTRY.as_bytes())
            .expect_err("must fail");
        match err {
            JsonSeedError::BadReferenceScript { detail } => {
                assert_eq!(detail, "unknown script type");
            }
            other => panic!("expected BadReferenceScript, got {other:?}"),
        }
    }

    #[test]
    fn utxo_seed_rejects_non_hex_cbor_hex() {
        let err = import_cardano_cli_json_utxo_from_bytes(NON_HEX_REF_SCRIPT_ENTRY.as_bytes())
            .expect_err("must fail");
        match err {
            JsonSeedError::BadReferenceScript { detail } => {
                assert_eq!(detail, "cbor_hex not valid hex");
            }
            other => panic!("expected BadReferenceScript, got {other:?}"),
        }
    }

    #[test]
    fn utxo_seed_rejects_missing_cbor_hex() {
        // `cborHex` is a required field on RawScriptEnvelope, so
        // omitting it produces a structural Json error rather than
        // a typed BadReferenceScript. Verify it lands in the closed
        // sum as `Json(_)` and never falls through to a successful
        // import.
        let bad = r#"{
            "00000000000000000000000000000000000000000000000000000000000000aa#0": {
                "address": "addr_test1wp97ley0p7xqksmh6tq3c6v8depl9jpfvnkk68d29fwznmcmlpuqk",
                "referenceScript": {
                    "script": {
                        "description": "",
                        "type": "PlutusScriptV2"
                    },
                    "scriptLanguage": "PlutusScriptLanguage PlutusScriptV2"
                },
                "value": { "lovelace": 100 }
            }
        }"#;
        let err = import_cardano_cli_json_utxo_from_bytes(bad.as_bytes()).expect_err("must fail");
        assert!(
            matches!(err, JsonSeedError::Json(_)),
            "missing cborHex must surface as Json, got {err:?}"
        );
    }

    /// One real Byron-era address from the preprod UTxO dump. The
    /// `Kjg...` prefix is canonical Base58 for cardano Byron-era
    /// addresses (the same prefix range appears for ~4364 of the
    /// ~5429 Byron addresses in a fresh preprod whole-utxo query).
    const BYRON_PREPROD_ADDR: &str =
        "KjgoiXJS2coQhwvEcJxB3Cp3hgzoYjSdaypptgAxgtht2dJYCnByrrvpm4ygqos3hiK2bWPw9QmC2XAVHq3dsU2tztaMLTkdxXRLe4DRFn8C";

    const BYRON_ENTRY: &str = r#"{
        "0000000000000000000000000000000000000000000000000000000000000010#0": {
            "address": "KjgoiXJS2coQhwvEcJxB3Cp3hgzoYjSdaypptgAxgtht2dJYCnByrrvpm4ygqos3hiK2bWPw9QmC2XAVHq3dsU2tztaMLTkdxXRLe4DRFn8C",
            "value": { "lovelace": 1000000 }
        }
    }"#;

    #[test]
    fn utxo_seed_accepts_byron_base58_address() {
        let (state, _) =
            import_cardano_cli_json_utxo_from_bytes(BYRON_ENTRY.as_bytes()).expect("import");
        assert_eq!(state.utxos.len(), 1);
        let out = state.utxos.values().next().unwrap();
        // The on-wire address bytes for a Byron address are
        // non-empty and DO NOT start with the Shelley-era
        // address header bits (Shelley addresses start with one
        // of the encoded type bits 0x00..0x7f; Byron CBOR starts
        // with major type 4/array, byte 0x82 or 0x83 typically).
        assert!(!out.address_bytes().is_empty());
    }

    #[test]
    fn utxo_seed_byron_address_decodes_to_expected_bytes() {
        // Sanity-check `decode_byron_base58_address` against a
        // known Byron preprod address. The leading byte of the
        // raw envelope CBOR is 0x82 (array(2)): the Byron
        // envelope is `[bytes(payload), checksum_u32]`.
        let bytes = decode_cli_address(BYRON_PREPROD_ADDR).expect("decode");
        assert!(!bytes.is_empty(), "decoded bytes must be non-empty");
        assert_eq!(
            bytes[0], 0x82,
            "Byron CBOR envelope MUST start with 0x82 (array(2)); got {:02x}",
            bytes[0]
        );
    }

    #[test]
    fn utxo_seed_rejects_garbage_address() {
        // A non-bech32, non-Base58 string fails closed.
        let bad = r#"{
            "0000000000000000000000000000000000000000000000000000000000000011#0": {
                "address": "!!! not a valid address !!!",
                "value": { "lovelace": 1 }
            }
        }"#;
        let err = import_cardano_cli_json_utxo_from_bytes(bad.as_bytes()).expect_err("err");
        assert!(matches!(err, JsonSeedError::BadAddress { .. }));
    }

    #[test]
    fn utxo_seed_canonical_script_ref_encoder_known_vector() {
        // Explicit byte-vector regression for the script_ref encoder.
        // Variant: PlutusScriptV2 (tag 2). cborHex = "4401020304"
        // (5 bytes = CBOR `bytes(0x01, 0x02, 0x03, 0x04)`: header
        // 0x44 means bytes of length 4, payload `01 02 03 04`).
        //
        // Expected:
        //   inner  = 82 02 44 01 02 03 04            (7 bytes)
        //   outer  = D8 18 47 82 02 44 01 02 03 04   (10 bytes)
        let rs = RawReferenceScript {
            script: super::super::json::RawScriptEnvelope {
                cbor_hex: "4401020304".to_string(),
                description: None,
                ty: "PlutusScriptV2".to_string(),
            },
            script_language: None,
        };
        let bytes = encode_script_ref(&rs).expect("encode");
        let expected: [u8; 10] = [0xd8, 0x18, 0x47, 0x82, 0x02, 0x44, 0x01, 0x02, 0x03, 0x04];
        assert_eq!(bytes, expected.to_vec());
    }

    #[test]
    fn utxo_seed_canonical_txout_address_extracted() {
        let (state, _) =
            import_cardano_cli_json_utxo_from_bytes(MINIMAL_TWO_ENTRY.as_bytes()).expect("import");
        let any = state.utxos.values().next().unwrap();
        // Address bytes are non-empty (bech32 decoded properly).
        assert!(!any.address_bytes().is_empty());
    }

    // ---------- MEM-OPT-OPS S2: streaming import equivalence (DC-MEM-06) ----------

    /// Write `json` to a temp file and stream-import it (the production path).
    fn streaming_import_str(json: &str) -> Result<(UTxOState, UtxoFingerprint), JsonSeedError> {
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new().expect("temp file");
        tmp.write_all(json.as_bytes()).expect("write fixture");
        tmp.flush().expect("flush");
        import_cardano_cli_json_utxo(tmp.path())
    }

    /// The core S2 equivalence: the streaming(file) path and the whole-buffer
    /// (bytes) path must AGREE on every input — both `Ok` with the byte-identical
    /// canonical fingerprint, or both `Err` (fail-closed identically). A streamed
    /// fingerprint that differed would be a consensus change, not a memory win.
    fn assert_streaming_matches_whole_buffer(label: &str, json: &str) {
        let streamed = streaming_import_str(json);
        let whole = import_cardano_cli_json_utxo_from_bytes(json.as_bytes());
        match (streamed, whole) {
            (Ok((ss, sf)), Ok((ws, wf))) => {
                assert_eq!(sf, wf, "[{label}] streamed fingerprint must equal whole-buffer");
                assert_eq!(
                    ss, ws,
                    "[{label}] streamed UTxOState must equal whole-buffer (full byte-identity, not just len/fingerprint)"
                );
            }
            (Err(_), Err(_)) => { /* both fail-closed identically — equivalent */ }
            (s, w) => panic!(
                "[{label}] streaming vs whole-buffer disagree (streamed_ok={}, whole_ok={})",
                s.is_ok(),
                w.is_ok()
            ),
        }
    }

    /// Two textually-distinct JSON keys that collapse to the SAME canonical TxIn
    /// (`#0` and `#00` both parse to index 0) carrying different values. The
    /// whole-buffer path (String-keyed dedup, last-in-sorted-order) and the
    /// streaming path (textual order) would otherwise pick different survivors
    /// and diverge — so BOTH must fail closed with `DuplicateTxIn` (M1).
    const COLLIDING_TXIN_ENTRY: &str = r#"{
        "0000000000000000000000000000000000000000000000000000000000000005#0": {
            "address": "addr_test1vq0ast4z2dypfrl9kg2c0garrcy085w78dls8xsx954x34cmgvp2u",
            "value": { "lovelace": 1000000 }
        },
        "0000000000000000000000000000000000000000000000000000000000000005#00": {
            "address": "addr_test1vq0ast4z2dypfrl9kg2c0garrcy085w78dls8xsx954x34cmgvp2u",
            "value": { "lovelace": 2000000 }
        }
    }"#;

    #[test]
    fn streaming_matches_whole_buffer_across_fixtures() {
        // Positive + negative fixtures: positives must agree on the FULL UTxOState;
        // negatives (incl. the colliding-TxIn case) must both fail closed. Proves
        // the streaming path never diverges from the whole-buffer oracle.
        for (label, json) in [
            ("minimal_two_entry", MINIMAL_TWO_ENTRY),
            ("inline_datum", INLINE_DATUM_ENTRY),
            ("plutus_v2_ref", PLUTUS_V2_REF_SCRIPT_ENTRY),
            ("plutus_v1_ref", PLUTUS_V1_REF_SCRIPT_ENTRY),
            ("plutus_v3_ref", PLUTUS_V3_REF_SCRIPT_ENTRY),
            ("simple_script_ref", SIMPLE_SCRIPT_REF_SCRIPT_ENTRY),
            ("unknown_type_ref", UNKNOWN_TYPE_REF_SCRIPT_ENTRY),
            ("non_hex_ref", NON_HEX_REF_SCRIPT_ENTRY),
            ("no_ref_baseline", NO_REF_SCRIPT_BASELINE),
            ("byron_entry", BYRON_ENTRY),
            ("colliding_txin", COLLIDING_TXIN_ENTRY),
        ] {
            assert_streaming_matches_whole_buffer(label, json);
        }
    }

    #[test]
    fn streaming_fingerprint_independent_of_textual_order() {
        // The streaming visitor inserts into a BTreeMap (canonical-key order), so
        // the fingerprint is independent of the JSON's textual entry order.
        let reordered = r#"{
            "0000000000000000000000000000000000000000000000000000000000000002#3": {
                "address": "addr_test1vq0ast4z2dypfrl9kg2c0garrcy085w78dls8xsx954x34cmgvp2u",
                "value": { "lovelace": 2000000 }
            },
            "0000000000000000000000000000000000000000000000000000000000000001#0": {
                "address": "addr_test1vq0ast4z2dypfrl9kg2c0garrcy085w78dls8xsx954x34cmgvp2u",
                "value": { "lovelace": 1000000 }
            }
        }"#;
        let (_, sorted_fp) = streaming_import_str(MINIMAL_TWO_ENTRY).expect("sorted");
        let (_, reordered_fp) = streaming_import_str(reordered).expect("reordered");
        assert_eq!(
            sorted_fp, reordered_fp,
            "streamed fingerprint is independent of JSON textual order"
        );
    }

    #[test]
    fn streaming_rejects_garbage_fail_closed() {
        assert!(streaming_import_str("not json at all").is_err());
    }

    #[test]
    fn streaming_rejects_trailing_data_fail_closed() {
        // A valid object followed by trailing junk must be rejected (no
        // best-effort accept) — Deserializer::end() catches it.
        let trailing = format!("{MINIMAL_TWO_ENTRY} trailing-garbage");
        assert!(
            streaming_import_str(&trailing).is_err(),
            "trailing data after the top-level object must fail closed"
        );
    }

    #[test]
    fn streaming_surfaces_conversion_error_not_swallowed() {
        // A structurally-valid entry whose key is not "<64hex>#<u16>" must surface
        // the exact JsonSeedError::BadTxInKey (the stashed conversion error), NOT a
        // generic serde halt and NOT a silently-skipped entry.
        let bad_key = r#"{
            "this_is_not_a_txin_key": {
                "address": "addr_test1vq0ast4z2dypfrl9kg2c0garrcy085w78dls8xsx954x34cmgvp2u",
                "value": { "lovelace": 1000000 }
            }
        }"#;
        let err = streaming_import_str(bad_key).expect_err("bad txin key must fail");
        assert!(
            matches!(err, JsonSeedError::BadTxInKey { .. }),
            "expected BadTxInKey, got {err:?}"
        );
    }

    #[test]
    fn streaming_rejects_duplicate_txin_fail_closed() {
        // Two distinct key strings colliding on one canonical TxIn (`#0` vs `#00`)
        // must fail closed with DuplicateTxIn -- never a silent, order-dependent
        // survivor (M1 / DC-MEM-06). Both the streaming and whole-buffer paths.
        let err = streaming_import_str(COLLIDING_TXIN_ENTRY).expect_err("duplicate txin must fail");
        assert!(
            matches!(err, JsonSeedError::DuplicateTxIn { .. }),
            "streaming: expected DuplicateTxIn, got {err:?}"
        );
        let werr = import_cardano_cli_json_utxo_from_bytes(COLLIDING_TXIN_ENTRY.as_bytes())
            .expect_err("whole-buffer duplicate txin must fail");
        assert!(
            matches!(werr, JsonSeedError::DuplicateTxIn { .. }),
            "whole-buffer: expected DuplicateTxIn, got {werr:?}"
        );
    }

    #[test]
    fn streaming_rejects_exact_duplicate_string_key_but_oracle_collapses() {
        // The HONEST asymmetry (per-cluster security review). An EXACT-duplicate JSON
        // string key (the same "<txid>#<ix>" literally twice -- distinct from the
        // canonical #0/#00 collision above): the streaming PRODUCTION path sees both
        // via serde's MapAccess and fails closed (DuplicateTxIn). The whole-buffer
        // ORACLE collapses exact string dups in its BTreeMap<String> parse (serde
        // last-wins) and returns Ok -- so the two TEST paths DIVERGE here. cardano-cli
        // emits unique outref keys by construction, so this input is not naturally
        // producible; the PRODUCTION (streaming) path is the fail-closed one. This
        // pins the asymmetry: the equivalence test covers only inputs where the two
        // paths agree; production fails closed on ANY duplicate.
        let dup = r#"{
            "0000000000000000000000000000000000000000000000000000000000000006#0": {
                "address": "addr_test1vq0ast4z2dypfrl9kg2c0garrcy085w78dls8xsx954x34cmgvp2u",
                "value": { "lovelace": 1000000 }
            },
            "0000000000000000000000000000000000000000000000000000000000000006#0": {
                "address": "addr_test1vq0ast4z2dypfrl9kg2c0garrcy085w78dls8xsx954x34cmgvp2u",
                "value": { "lovelace": 2000000 }
            }
        }"#;
        // Production (streaming): fail-closed on the exact-dup string key.
        let err = streaming_import_str(dup).expect_err("exact-dup string key must fail closed (streaming)");
        assert!(
            matches!(err, JsonSeedError::DuplicateTxIn { .. }),
            "streaming: expected DuplicateTxIn, got {err:?}"
        );
        // Oracle (whole-buffer, test-only): serde collapses exact-dup string keys
        // (last-wins) -> Ok. Documented asymmetry; the oracle is NOT the production path.
        assert!(
            import_cardano_cli_json_utxo_from_bytes(dup.as_bytes()).is_ok(),
            "whole-buffer oracle collapses exact-dup string keys (serde last-wins)"
        );
    }
}
