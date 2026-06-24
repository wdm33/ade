// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Native MemPack decoder for cardano-node V2 (utxohd-mem) LedgerDB `tables` TxOut values
//! (MITHRIL-VERIFIED-ANCHOR-IMPORT, Stage 2). The `tables` map is CBOR (`array(1)` → indefinite map
//! of `TxIn → TxOut`), but each TxOut value's CONTENT is MemPack — a compact non-CBOR format. This
//! module is the grounded compatibility decoder for that content, pinned to cardano-ledger-conway
//! 1.22.1.0 / mempack 0.2.1.0 (cardano-node 11.0.1). See `reference_mempack_txout_layout`.
//!
//! Non-negotiables enforced here:
//! - NO host-endianness assumptions: every multi-byte integer is read with an explicit LE/BE routine.
//! - CONSUME-EXACTLY at every nesting boundary (`expect_consumed`); trailing bytes are terminal.
//! - NO opaque preservation fallback: any unknown tag / address form / script language / value tag is
//!   a structured TERMINAL error, never a skip-or-keep-bytes.

use std::collections::BTreeMap;

use ade_codec::cbor::{read_array_header, read_bytes, read_map_header, ContainerEncoding};
use ade_crypto::blake2b::blake2b_256;
use ade_types::tx::Coin;
use ade_types::{Hash28, Hash32};

use crate::value::AssetName;

/// The Conway era index in the HardFork telescope (matches the Stage-1 `state` decode's era).
const CONWAY_ERA_INDEX: usize = 6;

/// Why a `tables` / MemPack decode fails. Every variant is TERMINAL (structured fail-closed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TablesDecodeError {
    /// A truncated / out-of-bounds read.
    UnexpectedEof { what: &'static str },
    /// A MemPack blob had trailing bytes after its fields (consume-exactly violated).
    TrailingBytes { what: &'static str, left: usize },
    /// An over-long or non-minimal VarLen.
    BadVarLen,
    /// A `Maybe` tag other than 0x00 / 0x01.
    BadMaybeTag { what: &'static str, tag: u8 },
    /// An unsupported TxOut constructor tag (not 0..=5).
    UnsupportedTxOutTag(u8),
    /// An unsupported / unrecognized address form.
    UnsupportedAddress(String),
    /// An unsupported value tag (not ada-only / multi-asset).
    UnsupportedValueTag(u8),
    /// An unsupported datum option tag.
    UnsupportedDatumTag(u8),
    /// An unsupported script tag or Plutus language byte.
    UnsupportedScript(String),
    /// The CBOR `tables` framing was malformed.
    MalformedTables(String),
    /// The snapshot era is not Conway (this MemPack layout is Conway-pinned).
    UnsupportedEra { decoded: String },
}

pub type R<T> = Result<T, TablesDecodeError>;

/// A bounded, explicit-endianness MemPack reader over one TxOut value's bytes (or a sub-slice). Every
/// read is bounds-checked (fail-closed on EOF). Multi-byte integers use explicit LE routines (the
/// MemPack `Word*` instances are host-LE; we never rely on the host being LE). Hash bytes are read
/// raw (PackedBytes are big-endian = natural on-chain order).
pub struct MemPackReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> MemPackReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        MemPackReader { data, pos: 0 }
    }

    pub fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    fn take(&mut self, n: usize, what: &'static str) -> R<&'a [u8]> {
        if n > self.remaining() {
            return Err(TablesDecodeError::UnexpectedEof { what });
        }
        let s = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }

    pub fn read_u8(&mut self, what: &'static str) -> R<u8> {
        Ok(self.take(1, what)?[0])
    }

    pub fn read_tag(&mut self, what: &'static str) -> R<u8> {
        self.read_u8(what)
    }

    /// MemPack `VarLen`: BIG-ENDIAN 7-bit groups, bit7 = continuation (NOT LEB128). Used for all
    /// lengths and the bare coin. Rejects overflow and a non-minimal leading-zero group.
    pub fn read_varlen(&mut self) -> R<u64> {
        let first = self.read_u8("varlen")?;
        // A leading 0x80 (a zero group that continues) is non-minimal -> reject (over-long).
        if first == 0x80 {
            return Err(TablesDecodeError::BadVarLen);
        }
        let mut val: u64 = (first & 0x7f) as u64;
        if first & 0x80 == 0 {
            return Ok(val);
        }
        loop {
            let b = self.read_u8("varlen")?;
            if val > (u64::MAX >> 7) {
                return Err(TablesDecodeError::BadVarLen);
            }
            val = (val << 7) | (b & 0x7f) as u64;
            if b & 0x80 == 0 {
                return Ok(val);
            }
        }
    }

    pub fn read_u16_le(&mut self) -> R<u16> {
        let b = self.take(2, "u16")?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    pub fn read_u32_le(&mut self) -> R<u32> {
        let b = self.take(4, "u32")?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub fn read_u64_le(&mut self) -> R<u64> {
        let b = self.take(8, "u64")?;
        let mut a = [0u8; 8];
        a.copy_from_slice(b);
        Ok(u64::from_le_bytes(a))
    }

    /// `n` raw bytes (BE hash bytes / address bytes are copied as-is).
    pub fn read_bytes(&mut self, n: usize, what: &'static str) -> R<&'a [u8]> {
        self.take(n, what)
    }

    /// A VarLen length followed by that many raw bytes (MemPack ShortByteString / ByteString).
    pub fn read_var_bytes(&mut self, what: &'static str) -> R<&'a [u8]> {
        let n = self.read_varlen()? as usize;
        self.take(n, what)
    }

    /// A MemPack `Maybe`: 1 tag byte (0x00 = Nothing, 0x01 = Just + value via `f`).
    pub fn read_maybe<T>(
        &mut self,
        what: &'static str,
        f: impl FnOnce(&mut Self) -> R<T>,
    ) -> R<Option<T>> {
        match self.read_u8(what)? {
            0x00 => Ok(None),
            0x01 => Ok(Some(f(self)?)),
            tag => Err(TablesDecodeError::BadMaybeTag { what, tag }),
        }
    }

    /// Consume-exactly: the reader MUST be at end-of-input. A non-empty remainder is terminal.
    pub fn expect_consumed(&self, what: &'static str) -> R<()> {
        if self.remaining() != 0 {
            return Err(TablesDecodeError::TrailingBytes {
                what,
                left: self.remaining(),
            });
        }
        Ok(())
    }
}

/// Recognize + validate a Shelley/Byron on-wire address from its header byte + length. Fail-closed
/// on an unknown form or inconsistent length (no opaque keep-bytes). The bytes ARE the on-wire form
/// (stored as-is); this only proves they are a form Ade understands.
pub(crate) fn validate_address_form(bytes: &[u8]) -> R<()> {
    let header = *bytes
        .first()
        .ok_or_else(|| TablesDecodeError::UnsupportedAddress("empty address".into()))?;
    let kind = header >> 4;
    let network = header & 0x0f;
    let len = bytes.len();
    let need_net = || -> R<()> {
        if network > 1 {
            Err(TablesDecodeError::UnsupportedAddress(format!(
                "bad network id {network}"
            )))
        } else {
            Ok(())
        }
    };
    match kind {
        // base: header(1) + payment(28) + stake(28)
        0x0..=0x3 => {
            need_net()?;
            if len != 57 {
                return Err(TablesDecodeError::UnsupportedAddress(format!(
                    "base address len {len} != 57"
                )));
            }
        }
        // pointer: header(1) + payment(28) + pointer (>= 3 nat varints)
        0x4 | 0x5 => {
            need_net()?;
            if len < 1 + 28 + 3 {
                return Err(TablesDecodeError::UnsupportedAddress(format!(
                    "pointer address len {len} < 32"
                )));
            }
        }
        // enterprise: header(1) + payment(28)
        0x6 | 0x7 => {
            need_net()?;
            if len != 29 {
                return Err(TablesDecodeError::UnsupportedAddress(format!(
                    "enterprise address len {len} != 29"
                )));
            }
        }
        // Byron: variable CBOR, delimited by the outer length
        0x8 => {}
        // reward: header(1) + stake(28)
        0xe | 0xf => {
            need_net()?;
            if len != 29 {
                return Err(TablesDecodeError::UnsupportedAddress(format!(
                    "reward address len {len} != 29"
                )));
            }
        }
        other => {
            return Err(TablesDecodeError::UnsupportedAddress(format!(
                "unknown address kind {other:#x}"
            )))
        }
    }
    Ok(())
}

/// Decode a `CompactAddr` (TxOut tags 0/1/4/5): a VarLen length + the raw on-wire address bytes.
/// Returns the validated on-wire bytes.
pub(crate) fn read_compact_addr(r: &mut MemPackReader) -> R<Vec<u8>> {
    let bytes = r.read_var_bytes("compact_addr")?;
    validate_address_form(bytes)?;
    Ok(bytes.to_vec())
}

/// MemPack `Credential 'Staking` = 1 tag byte (0x00 ScriptHashObj / 0x01 KeyHashObj) + 28 hash bytes
/// in NATURAL order (PackedBytes, big-endian writes = on-chain order). Returns (is_script, hash).
fn read_staking_credential(r: &mut MemPackReader) -> R<(bool, [u8; 28])> {
    let tag = r.read_u8("cred_tag")?;
    let is_script = match tag {
        0x00 => true,
        0x01 => false,
        other => {
            return Err(TablesDecodeError::UnsupportedAddress(format!(
                "staking credential tag {other}"
            )))
        }
    };
    let mut hash = [0u8; 28];
    hash.copy_from_slice(r.read_bytes(28, "cred_hash")?);
    Ok((is_script, hash))
}

/// Reconstruct the on-wire BASE address from a tag-2/3 `Addr28Extra` (the payment credential hash +
/// flags) given the already-decoded staking credential. PAYMENT hash: `Addr28Extra` = 4 host-LE
/// Word64; `read_u64_le` then `to_be_bytes` recovers the natural hash bytes (the BE→LE double-flip).
/// STAKE hash: natural order (read by [`read_staking_credential`]). Flags in w3 low 32 bits:
/// bit0 = payment-is-keyhash, bit1 = is-mainnet. tag-2/3 is ALWAYS a base address + ada-only.
fn read_addr28_base_address(
    r: &mut MemPackReader,
    stake_is_script: bool,
    stake_hash: &[u8; 28],
) -> R<Vec<u8>> {
    let a = r.read_u64_le()?;
    let b = r.read_u64_le()?;
    let c = r.read_u64_le()?;
    let d = r.read_u64_le()?;
    let mut payment = [0u8; 28];
    payment[0..8].copy_from_slice(&a.to_be_bytes());
    payment[8..16].copy_from_slice(&b.to_be_bytes());
    payment[16..24].copy_from_slice(&c.to_be_bytes());
    payment[24..28].copy_from_slice(&((d >> 32) as u32).to_be_bytes());
    let flags = (d & 0xffff_ffff) as u32;
    let payment_is_script = (flags & 0b01) == 0; // bit0 set => keyhash
    let is_mainnet = (flags & 0b10) != 0; // bit1
                                          // CIP-19 base header: bit5 = stake-is-script, bit4 = payment-is-script, bits[3..0] = network.
                                          // (the exact bit assignment is a PO#1 cross-check against cardano-cli query utxo.)
    let header = ((stake_is_script as u8) << 5)
        | ((payment_is_script as u8) << 4)
        | (is_mainnet as u8);
    let mut addr = Vec::with_capacity(57);
    addr.push(header);
    addr.extend_from_slice(&payment);
    addr.extend_from_slice(stake_hash);
    validate_address_form(&addr)?;
    Ok(addr)
}

fn rd_u64_le(b: &[u8]) -> u64 {
    let mut a = [0u8; 8];
    a.copy_from_slice(b);
    u64::from_le_bytes(a)
}

fn rep_slice(rep: &[u8], off: usize, len: usize) -> R<&[u8]> {
    let end = off
        .checked_add(len)
        .ok_or(TablesDecodeError::UnexpectedEof { what: "ma_rep" })?;
    rep.get(off..end)
        .ok_or(TablesDecodeError::UnexpectedEof { what: "ma_rep" })
}

/// A faithfully-decoded output value: lovelace + the multi-asset bundle with FULL Word64 quantities.
///
/// Cardano output multi-asset quantities are Word64 (0 ..= 2^64-1). They are kept as `u64` here with
/// NO truncation, saturation, or i64 cast (a persisted/imported snapshot quantity is never lost —
/// DC-MITHRIL-05, tier=true). Ade's i64 `MultiAsset` cannot hold the upper half; routing these
/// outputs through it is a RELEASE BLOCKER for full ledger validation, tracked as a downstream
/// obligation, not handled in this snapshot-decoder slice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxOutValue {
    pub coin: Coin,
    pub assets: BTreeMap<Hash28, BTreeMap<AssetName, u64>>,
}

impl TxOutValue {
    pub fn ada_only(coin: Coin) -> Self {
        TxOutValue {
            coin,
            assets: BTreeMap::new(),
        }
    }
    pub fn is_ada_only(&self) -> bool {
        self.assets.is_empty()
    }
}

/// Decode a CompactValue (CompactForm MaryValue): 1 tag byte (0x00 ada-only / 0x01 multi-asset) +
/// a bare VarLen coin [+ VarLen numMA + a VarLen-prefixed flat rep]. Faithful u64 quantities.
fn read_compact_value(r: &mut MemPackReader) -> R<TxOutValue> {
    match r.read_u8("value_tag")? {
        0x00 => Ok(TxOutValue::ada_only(Coin(r.read_varlen()?))),
        0x01 => {
            let coin = Coin(r.read_varlen()?);
            let n = r.read_varlen()? as usize;
            let rep = r.read_var_bytes("ma_rep")?;
            Ok(TxOutValue {
                coin,
                assets: decode_multiasset_rep(rep, n)?,
            })
        }
        other => Err(TablesDecodeError::UnsupportedValueTag(other)),
    }
}

/// A standalone `CompactForm Coin` = a tag byte (0x00) + a VarLen coin. The tag-2/3 ada-only TxOut
/// uses this standalone form; `CompactValue` inlines the coin and skips the tag.
fn read_compact_coin(r: &mut MemPackReader) -> R<Coin> {
    match r.read_u8("coin_tag")? {
        0x00 => Ok(Coin(r.read_varlen()?)),
        other => Err(TablesDecodeError::MalformedTables(format!(
            "compact coin tag {other}"
        ))),
    }
}

/// Decode the flat multi-asset rep into Ade's MultiAsset. `n` triples. Regions (ABSOLUTE byte offsets
/// into `rep`, words host-LE): A=quantity@8i (Word64), B=policy-off@8n+2i (Word16), C=name-off@10n+2i
/// (Word16), D=policy-ids(28B)@12n, E=asset-names after D. A shared asset name repeats its C offset;
/// its length is the next DISTINCT C offset minus this one (nubOrd), or the rep end for the last.
fn decode_multiasset_rep(rep: &[u8], n: usize) -> R<BTreeMap<Hash28, BTreeMap<AssetName, u64>>> {
    let mut name_offs = Vec::with_capacity(n);
    for i in 0..n {
        let b = rep_slice(rep, 10 * n + 2 * i, 2)?;
        name_offs.push(u16::from_le_bytes([b[0], b[1]]) as usize);
    }
    let mut distinct = name_offs.clone();
    distinct.sort_unstable();
    distinct.dedup();
    let mut ma: BTreeMap<Hash28, BTreeMap<AssetName, u64>> = BTreeMap::new();
    for i in 0..n {
        // FAITHFUL Word64 -> u64: never truncated, saturated, or cast to i64 (DC-MITHRIL-05).
        let qty = rd_u64_le(rep_slice(rep, 8 * i, 8)?);
        let pb = rep_slice(rep, 8 * n + 2 * i, 2)?;
        let policy_off = u16::from_le_bytes([pb[0], pb[1]]) as usize;
        let mut policy = [0u8; 28];
        policy.copy_from_slice(rep_slice(rep, policy_off, 28)?);
        let name_off = name_offs[i];
        let name_end = distinct
            .iter()
            .copied()
            .find(|&d| d > name_off)
            .unwrap_or(rep.len());
        let name = rep_slice(rep, name_off, name_end.saturating_sub(name_off))?.to_vec();
        if name.len() > 32 {
            return Err(TablesDecodeError::MalformedTables(format!(
                "asset name len {} > 32",
                name.len()
            )));
        }
        ma.entry(Hash28(policy))
            .or_default()
            .insert(AssetName(name), qty);
    }
    Ok(ma)
}

/// A fully-decoded V2 `tables` TxOut. The structured fields are for validation + canonical evidence;
/// the inline-datum and reference-script ORIGINAL wire bytes are PRESERVED (hash-sensitive — never
/// re-encoded into a substitute).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedTxOut {
    /// On-wire address bytes (header + credential hashes).
    pub address: Vec<u8>,
    /// Output value (coin + multi-asset, faithful u64 quantities).
    pub value: TxOutValue,
    /// Datum (none / hash / preserved inline bytes).
    pub datum: DatumField,
    /// Reference script (preserved wire bytes + type), if present.
    pub script: Option<ScriptField>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DatumField {
    None,
    /// A datum hash (32 bytes, canonical order).
    Hash([u8; 32]),
    /// An inline datum — the PRESERVED CBOR-of-Plutus-Data bytes.
    Inline(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptField {
    /// A native (Timelock) script — the PRESERVED memoized CBOR bytes.
    Native(Vec<u8>),
    /// A Plutus script (version 1/2/3) — the PRESERVED flat script bytes.
    Plutus { version: u8, bytes: Vec<u8> },
}

/// Reconstruct a tag-3 `DataHash32`: 4 host-LE Word64 (same BE→LE double-flip as Addr28Extra).
fn read_data_hash32(r: &mut MemPackReader) -> R<[u8; 32]> {
    let mut h = [0u8; 32];
    for k in 0..4 {
        h[k * 8..k * 8 + 8].copy_from_slice(&r.read_u64_le()?.to_be_bytes());
    }
    Ok(h)
}

/// A natural-order (PackedBytes big-endian) 32-byte hash (tag-1 DataHash; tag-5 DatumHash).
fn read_hash32_be(r: &mut MemPackReader, what: &'static str) -> R<[u8; 32]> {
    let mut h = [0u8; 32];
    h.copy_from_slice(r.read_bytes(32, what)?);
    Ok(h)
}

/// The tag-5 `Datum` option: 1 tag byte (0 none / 1 hash+32B / 2 inline + VarLen bytes).
fn read_datum_option(r: &mut MemPackReader) -> R<DatumField> {
    match r.read_u8("datum_tag")? {
        0x00 => Ok(DatumField::None),
        0x01 => Ok(DatumField::Hash(read_hash32_be(r, "datum_hash")?)),
        0x02 => Ok(DatumField::Inline(r.read_var_bytes("inline_datum")?.to_vec())),
        other => Err(TablesDecodeError::UnsupportedDatumTag(other)),
    }
}

/// The tag-5 reference `Script`: AlonzoScript tag (0 native MemoBytes / 1 Plutus + Conway language
/// byte 0=V1 / 1=V2 / 2=V3 + VarLen flat bytes). Original wire bytes preserved.
fn read_script(r: &mut MemPackReader) -> R<ScriptField> {
    match r.read_u8("script_tag")? {
        0x00 => Ok(ScriptField::Native(r.read_var_bytes("native_script")?.to_vec())),
        0x01 => {
            let version = match r.read_u8("plutus_lang")? {
                0 => 1,
                1 => 2,
                2 => 3,
                other => {
                    return Err(TablesDecodeError::UnsupportedScript(format!(
                        "plutus language byte {other}"
                    )))
                }
            };
            Ok(ScriptField::Plutus {
                version,
                bytes: r.read_var_bytes("plutus_script")?.to_vec(),
            })
        }
        other => Err(TablesDecodeError::UnsupportedScript(format!(
            "alonzo script tag {other}"
        ))),
    }
}

/// Decode one V2 `tables` TxOut value (the MemPack blob) via the 6-way constructor tag. CONSUME-
/// EXACTLY: the whole value blob must be consumed (trailing bytes are terminal).
pub fn read_txout(value: &[u8]) -> R<DecodedTxOut> {
    let mut r = MemPackReader::new(value);
    let out = match r.read_u8("txout_tag")? {
        0 => DecodedTxOut {
            address: read_compact_addr(&mut r)?,
            value: read_compact_value(&mut r)?,
            datum: DatumField::None,
            script: None,
        },
        1 => {
            let address = read_compact_addr(&mut r)?;
            let value = read_compact_value(&mut r)?;
            DecodedTxOut {
                address,
                value,
                datum: DatumField::Hash(read_hash32_be(&mut r, "datahash")?),
                script: None,
            }
        }
        2 => {
            let (sis, sh) = read_staking_credential(&mut r)?;
            let address = read_addr28_base_address(&mut r, sis, &sh)?;
            DecodedTxOut {
                address,
                value: TxOutValue::ada_only(read_compact_coin(&mut r)?),
                datum: DatumField::None,
                script: None,
            }
        }
        3 => {
            let (sis, sh) = read_staking_credential(&mut r)?;
            let address = read_addr28_base_address(&mut r, sis, &sh)?;
            let coin = read_compact_coin(&mut r)?;
            DecodedTxOut {
                address,
                value: TxOutValue::ada_only(coin),
                datum: DatumField::Hash(read_data_hash32(&mut r)?),
                script: None,
            }
        }
        4 => {
            let address = read_compact_addr(&mut r)?;
            let value = read_compact_value(&mut r)?;
            DecodedTxOut {
                address,
                value,
                datum: DatumField::Inline(r.read_var_bytes("inline_datum")?.to_vec()),
                script: None,
            }
        }
        5 => {
            let address = read_compact_addr(&mut r)?;
            let value = read_compact_value(&mut r)?;
            let datum = read_datum_option(&mut r)?;
            let script = Some(read_script(&mut r)?);
            DecodedTxOut {
                address,
                value,
                datum,
                script,
            }
        }
        other => return Err(TablesDecodeError::UnsupportedTxOutTag(other)),
    };
    r.expect_consumed("txout")?;
    Ok(out)
}

/// Canonical deterministic serialization of a decoded TxOut (the unit hashed into the whole-tables
/// commitment). Big-endian fixed widths + BTreeMap-sorted assets => identical TxOut serializes
/// identically; the preserved datum/script wire bytes are included verbatim.
fn canonical_txout_bytes(o: &DecodedTxOut) -> Vec<u8> {
    let mut v = Vec::with_capacity(64 + o.address.len());
    v.extend_from_slice(&(o.address.len() as u32).to_be_bytes());
    v.extend_from_slice(&o.address);
    v.extend_from_slice(&o.value.coin.0.to_be_bytes());
    v.extend_from_slice(&(o.value.assets.len() as u32).to_be_bytes());
    for (policy, names) in &o.value.assets {
        v.extend_from_slice(&policy.0);
        v.extend_from_slice(&(names.len() as u32).to_be_bytes());
        for (name, qty) in names {
            v.extend_from_slice(&(name.0.len() as u32).to_be_bytes());
            v.extend_from_slice(&name.0);
            v.extend_from_slice(&qty.to_be_bytes()); // faithful u64
        }
    }
    match &o.datum {
        DatumField::None => v.push(0),
        DatumField::Hash(h) => {
            v.push(1);
            v.extend_from_slice(h);
        }
        DatumField::Inline(b) => {
            v.push(2);
            v.extend_from_slice(&(b.len() as u32).to_be_bytes());
            v.extend_from_slice(b);
        }
    }
    match &o.script {
        None => v.push(0),
        Some(ScriptField::Native(b)) => {
            v.push(1);
            v.extend_from_slice(&(b.len() as u32).to_be_bytes());
            v.extend_from_slice(b);
        }
        Some(ScriptField::Plutus { version, bytes }) => {
            v.push(2);
            v.push(*version);
            v.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
            v.extend_from_slice(bytes);
        }
    }
    v
}

/// The deterministic outcome of decoding a whole `tables` file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TablesDecodeSummary {
    pub count: usize,
    pub tag_counts: [u64; 6],
    /// blake2b chain over the canonical (sorted) `(TxIn ++ canonical TxOut)` stream.
    pub commitment: Hash32,
}

/// Decode the whole V2 `tables` CBOR map into a deterministic canonical-UTxO commitment, BOUND to the
/// era decoded from the SAME snapshot's `state` (the Stage-1 NES). PO#2: `state_era_index` is that
/// era — REQUIRE Conway before interpreting the tables as this MemPack layout (never the tables file
/// or a CLI flag). Streams in the map's canonical (ascending TxIn) order, asserting it. `max_entries`
/// caps the work for tests (None = the whole file). Fail-closed: any TxOut decode error halts.
pub fn decode_tables_commitment(
    tables: &[u8],
    state_era_index: usize,
    max_entries: Option<usize>,
) -> R<TablesDecodeSummary> {
    if state_era_index != CONWAY_ERA_INDEX {
        return Err(TablesDecodeError::UnsupportedEra {
            decoded: format!("state era index {state_era_index} (require Conway)"),
        });
    }
    let mut o = 0usize;
    match read_array_header(tables, &mut o)
        .map_err(|e| TablesDecodeError::MalformedTables(format!("{e:?}")))?
    {
        ContainerEncoding::Definite(1, _) => {}
        other => {
            return Err(TablesDecodeError::MalformedTables(format!(
                "tables outer array != 1: {other:?}"
            )))
        }
    }
    let _ = read_map_header(tables, &mut o)
        .map_err(|e| TablesDecodeError::MalformedTables(format!("{e:?}")))?;
    let mut running = blake2b_256(b"ade-v2-ledgerdb-tables-utxo-commitment-v1");
    let mut count = 0usize;
    let mut tag_counts = [0u64; 6];
    let mut prev: Option<Vec<u8>> = None;
    loop {
        if o >= tables.len() || tables[o] == 0xff {
            break;
        }
        let (txin, _) = read_bytes(tables, &mut o)
            .map_err(|e| TablesDecodeError::MalformedTables(format!("{e:?}")))?;
        let (val, _) = read_bytes(tables, &mut o)
            .map_err(|e| TablesDecodeError::MalformedTables(format!("{e:?}")))?;
        if let Some(p) = &prev {
            if &txin <= p {
                return Err(TablesDecodeError::MalformedTables(
                    "tables map keys not in ascending (canonical) order".into(),
                ));
            }
        }
        prev = Some(txin.clone());
        let out = read_txout(&val)?;
        let tag = val[0] as usize;
        if tag < 6 {
            tag_counts[tag] += 1;
        }
        let entry = canonical_txout_bytes(&out);
        let mut chained = Vec::with_capacity(32 + txin.len() + entry.len());
        chained.extend_from_slice(&running.0);
        chained.extend_from_slice(&txin);
        chained.extend_from_slice(&entry);
        running = blake2b_256(&chained);
        count += 1;
        if let Some(m) = max_entries {
            if count >= m {
                break;
            }
        }
    }
    Ok(TablesDecodeSummary {
        count,
        tag_counts,
        commitment: running,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn varlen_big_endian_7bit_matches_real_coin() {
        // The oracle's real-value coin: bytes e8 af 30 -> 1,710,000 lovelace.
        let mut r = MemPackReader::new(&[0xe8, 0xaf, 0x30]);
        assert_eq!(r.read_varlen().unwrap(), 1_710_000);
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn varlen_small_and_boundary() {
        assert_eq!(MemPackReader::new(&[0x00]).read_varlen().unwrap(), 0);
        assert_eq!(MemPackReader::new(&[0x7f]).read_varlen().unwrap(), 127);
        assert_eq!(MemPackReader::new(&[0x81, 0x00]).read_varlen().unwrap(), 128);
        assert_eq!(
            MemPackReader::new(&[0xff, 0x7f]).read_varlen().unwrap(),
            (0x7f << 7) | 0x7f
        );
    }

    #[test]
    fn varlen_rejects_non_minimal_leading_zero() {
        // 0x80 0x00 would decode 0 in two bytes -> non-minimal -> terminal.
        assert_eq!(
            MemPackReader::new(&[0x80, 0x00]).read_varlen(),
            Err(TablesDecodeError::BadVarLen)
        );
    }

    #[test]
    fn varlen_rejects_overflow() {
        // 11 continuation groups overflow u64.
        let over = [0xff; 11];
        assert_eq!(
            MemPackReader::new(&over).read_varlen(),
            Err(TablesDecodeError::BadVarLen)
        );
    }

    #[test]
    fn explicit_le_words_are_host_independent() {
        let mut r = MemPackReader::new(&[0x01, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(r.read_u64_le().unwrap(), 1);
        let mut r = MemPackReader::new(&[0x34, 0x12]);
        assert_eq!(r.read_u16_le().unwrap(), 0x1234);
    }

    #[test]
    fn maybe_and_consume_exactly() {
        let mut r = MemPackReader::new(&[0x00]);
        assert_eq!(r.read_maybe("m", |_| Ok(())).unwrap(), None);
        r.expect_consumed("m").unwrap();

        let mut r = MemPackReader::new(&[0x01, 0xab]);
        assert_eq!(
            r.read_maybe("m", |rr| rr.read_u8("v")).unwrap(),
            Some(0xab)
        );
        r.expect_consumed("m").unwrap();

        // trailing bytes -> terminal
        let mut r = MemPackReader::new(&[0x00, 0x99]);
        r.read_u8("x").unwrap();
        assert!(matches!(
            r.expect_consumed("x"),
            Err(TablesDecodeError::TrailingBytes { left: 1, .. })
        ));
    }

    #[test]
    fn eof_is_terminal_not_panic() {
        let mut r = MemPackReader::new(&[0x05]);
        r.read_u8("a").unwrap();
        assert!(matches!(
            r.read_u8("b"),
            Err(TablesDecodeError::UnexpectedEof { what: "b" })
        ));
        assert!(matches!(
            MemPackReader::new(&[0x00, 0x00]).read_u64_le(),
            Err(TablesDecodeError::UnexpectedEof { .. })
        ));
    }

    #[test]
    fn compact_addr_forms_validate_and_reject() {
        // base (kind 0, testnet net 0): header 0x00 + 56 hash bytes = 57; CompactAddr = VarLen(57)+bytes
        let mut base = vec![0x00u8];
        base.extend_from_slice(&[0xab; 56]);
        let mut blob = vec![57u8]; // 57 < 128 -> single VarLen byte
        blob.extend_from_slice(&base);
        assert_eq!(
            read_compact_addr(&mut MemPackReader::new(&blob)).unwrap(),
            base
        );
        // enterprise (kind 6) len 29, reward (kind 14) len 29 both validate
        let mut ent = vec![0x60u8];
        ent.extend_from_slice(&[0xcd; 28]);
        validate_address_form(&ent).unwrap();
        let mut rew = vec![0xe0u8];
        rew.extend_from_slice(&[0x11; 28]);
        validate_address_form(&rew).unwrap();
        // wrong length, unknown kind, bad network -> all terminal (no opaque keep-bytes)
        assert!(matches!(
            validate_address_form(&[0x00, 0x01, 0x02]),
            Err(TablesDecodeError::UnsupportedAddress(_))
        ));
        assert!(matches!(
            validate_address_form(&[0x90; 29]),
            Err(TablesDecodeError::UnsupportedAddress(_))
        ));
        let mut badnet = vec![0x05u8];
        badnet.extend_from_slice(&[0u8; 56]);
        assert!(matches!(
            validate_address_form(&badnet),
            Err(TablesDecodeError::UnsupportedAddress(_))
        ));
    }

    #[test]
    fn addr28_base_address_reconstruction_round_trip() {
        // A known payment hash + flags packed exactly as cardano-ledger's Addr28Extra (4 host-LE
        // Word64), then reconstructed -> the same hash + a well-formed base address.
        let pay: [u8; 28] = std::array::from_fn(|i| (i as u8) + 1);
        let stake: [u8; 28] = std::array::from_fn(|i| (i as u8) + 100);
        let a = u64::from_be_bytes(pay[0..8].try_into().unwrap());
        let b = u64::from_be_bytes(pay[8..16].try_into().unwrap());
        let c = u64::from_be_bytes(pay[16..24].try_into().unwrap());
        let last4 = u32::from_be_bytes(pay[24..28].try_into().unwrap());
        let flags: u32 = 0b01; // payment-is-keyhash, testnet
        let d = ((last4 as u64) << 32) | flags as u64;
        let mut disk = Vec::new();
        for w in [a, b, c, d] {
            disk.extend_from_slice(&w.to_le_bytes());
        }
        let mut r = MemPackReader::new(&disk);
        let addr = read_addr28_base_address(&mut r, false, &stake).unwrap();
        assert_eq!(addr.len(), 57);
        assert_eq!(&addr[1..29], &pay, "payment hash recovered");
        assert_eq!(&addr[29..57], &stake, "stake hash recovered");
        // stake keyhash + payment keyhash + testnet -> base header 0x00.
        assert_eq!(addr[0], 0x00);
        r.expect_consumed("addr28").unwrap();
    }

    #[test]
    fn staking_credential_tag_is_fail_closed() {
        assert_eq!(
            read_staking_credential(&mut MemPackReader::new(
                &[&[0x01u8][..], &[0xab; 28]].concat()
            ))
            .unwrap(),
            (false, [0xab; 28])
        );
        assert!(matches!(
            read_staking_credential(&mut MemPackReader::new(&[0x02; 29])),
            Err(TablesDecodeError::UnsupportedAddress(_))
        ));
    }

    #[test]
    fn compact_value_ada_only_and_multiasset() {
        // ada-only: tag 0x00 + VarLen coin (e8 af 30 = 1,710,000)
        let blob = [0x00u8, 0xe8, 0xaf, 0x30];
        let v = read_compact_value(&mut MemPackReader::new(&blob)).unwrap();
        assert_eq!(v.coin, Coin(1_710_000));
        assert!(v.is_ada_only());

        // multi-asset: 1 triple (policy 0xaa.., name "TOKE", qty 1_000_000)
        let policy = [0xaau8; 28];
        let name = b"TOKE";
        let mut rep = Vec::new();
        rep.extend_from_slice(&1_000_000u64.to_le_bytes()); // A: quantity @ 0
        rep.extend_from_slice(&12u16.to_le_bytes()); // B: policy off @ 8 -> D@12n=12
        rep.extend_from_slice(&40u16.to_le_bytes()); // C: name off @ 10 -> E@12+28=40
        rep.extend_from_slice(&policy); // D @ 12
        rep.extend_from_slice(name); // E @ 40
        assert_eq!(rep.len(), 44);
        let mut blob = vec![0x01u8, 0x0a, 0x01, rep.len() as u8]; // tag, coin=10, numMA=1, repLen=44
        blob.extend_from_slice(&rep);
        let v = read_compact_value(&mut MemPackReader::new(&blob)).unwrap();
        assert_eq!(v.coin, Coin(10));
        let names = &v.assets[&Hash28(policy)];
        assert_eq!(names[&AssetName(name.to_vec())], 1_000_000);
    }

    #[test]
    fn multiasset_quantities_preserved_exactly_as_u64_no_i64_cast() {
        // 3 assets under one policy with quantities at and ABOVE the i64 ceiling — preserved exactly.
        let policy = [0xaau8; 28];
        let quantities: [u64; 3] = [i64::MAX as u64, i64::MAX as u64 + 1, u64::MAX];
        let n = 3usize;
        let mut rep = Vec::new();
        for q in quantities {
            rep.extend_from_slice(&q.to_le_bytes()); // A: quantities @ 0 (24 bytes)
        }
        for _ in 0..n {
            rep.extend_from_slice(&36u16.to_le_bytes()); // B: policy offsets @ 24 -> D @ 12n=36
        }
        rep.extend_from_slice(&64u16.to_le_bytes()); // C: name offsets @ 30 -> E @ 36+28=64
        rep.extend_from_slice(&65u16.to_le_bytes());
        rep.extend_from_slice(&66u16.to_le_bytes());
        rep.extend_from_slice(&policy); // D @ 36
        rep.extend_from_slice(b"ABC"); // E @ 64
        assert_eq!(rep.len(), 67);
        let mut blob = vec![0x01u8, 0x00, n as u8, rep.len() as u8]; // tag, coin=0, numMA=3, repLen=67
        blob.extend_from_slice(&rep);
        let v = read_compact_value(&mut MemPackReader::new(&blob)).unwrap();
        let a = &v.assets[&Hash28(policy)];
        assert_eq!(a[&AssetName(b"A".to_vec())], 9_223_372_036_854_775_807); // i64::MAX
        assert_eq!(a[&AssetName(b"B".to_vec())], 9_223_372_036_854_775_808); // i64::MAX + 1 (> i64::MAX)
        assert_eq!(a[&AssetName(b"C".to_vec())], u64::MAX); // full Word64 preserved
    }

    #[test]
    fn coin_varlen_overflow_is_terminal() {
        // an over-long VarLen coin overflows u64 -> terminal BadVarLen (no truncation/wraparound).
        let mut blob = vec![0x00u8]; // ada-only value tag
        blob.extend_from_slice(&[0xff; 11]); // 11 continuation groups overflow u64
        assert!(matches!(
            read_compact_value(&mut MemPackReader::new(&blob)),
            Err(TablesDecodeError::BadVarLen)
        ));
    }

    #[test]
    fn txout_dispatch_tag0_tag5_and_fail_closed() {
        // tag 0: enterprise addr (0x60 + 28 = 29) + ada-only value (coin 10)
        let mut addr = vec![0x60u8];
        addr.extend_from_slice(&[0xcd; 28]);
        let mut t0 = vec![0x00u8, 29u8];
        t0.extend_from_slice(&addr);
        t0.extend_from_slice(&[0x00, 0x0a]); // value: ada-only, coin 10
        let o = read_txout(&t0).unwrap();
        assert_eq!(o.address, addr);
        assert_eq!(o.value.coin, Coin(10));
        assert_eq!(o.datum, DatumField::None);
        assert!(o.script.is_none());

        // tag 5: base addr (57) + ada value + datum none + Plutus-V2 reference script
        let mut base = vec![0x00u8];
        base.extend_from_slice(&[0xab; 56]);
        let mut t5 = vec![0x05u8, 57u8];
        t5.extend_from_slice(&base);
        t5.extend_from_slice(&[0x00, 0x05]); // value: ada-only, coin 5
        t5.push(0x00); // datum option: none
        t5.extend_from_slice(&[0x01, 0x01, 0x03, 0xaa, 0xbb, 0xcc]); // script: Plutus, V2, 3 bytes
        let o = read_txout(&t5).unwrap();
        assert_eq!(o.value.coin, Coin(5));
        assert_eq!(o.datum, DatumField::None);
        assert_eq!(
            o.script,
            Some(ScriptField::Plutus {
                version: 2,
                bytes: vec![0xaa, 0xbb, 0xcc]
            })
        );

        // consume-exactly: a trailing byte is terminal
        let mut bad = t0.clone();
        bad.push(0x99);
        assert!(matches!(
            read_txout(&bad),
            Err(TablesDecodeError::TrailingBytes { .. })
        ));
        // unknown TxOut tag is terminal
        assert!(matches!(
            read_txout(&[0x06, 0x00]),
            Err(TablesDecodeError::UnsupportedTxOutTag(6))
        ));
    }

    #[test]
    fn tables_commitment_deterministic_era_bound_and_sorted() {
        let mk = |txid: u8, ix: u16, coin: u8| -> Vec<u8> {
            let mut e = vec![0x58u8, 34u8]; // key bytes(34)
            e.extend(vec![txid; 32]);
            e.extend_from_slice(&ix.to_be_bytes());
            let mut val = vec![0x00u8, 29u8, 0x60u8]; // txout tag0, CompactAddr len 29, header
            val.extend_from_slice(&[0xcd; 28]);
            val.extend_from_slice(&[0x00, coin]); // ada-only value
            e.push(0x58);
            e.push(val.len() as u8);
            e.extend_from_slice(&val);
            e
        };
        let mut tables = vec![0x81u8, 0xbf]; // array(1), indefinite map
        tables.extend(mk(0x01, 0, 10));
        tables.extend(mk(0x02, 0, 20));
        tables.push(0xff);
        let s1 = decode_tables_commitment(&tables, 6, None).unwrap();
        assert_eq!(s1.count, 2);
        assert_eq!(s1.tag_counts[0], 2);
        // deterministic: same bytes + era -> identical commitment
        assert_eq!(
            s1.commitment,
            decode_tables_commitment(&tables, 6, None).unwrap().commitment
        );
        // PO#2 era binding: a non-Conway state era is terminal
        assert!(matches!(
            decode_tables_commitment(&tables, 5, None),
            Err(TablesDecodeError::UnsupportedEra { .. })
        ));
        // non-canonical (unsorted) keys are terminal
        let mut bad = vec![0x81u8, 0xbf];
        bad.extend(mk(0x02, 0, 20));
        bad.extend(mk(0x01, 0, 10));
        bad.push(0xff);
        assert!(matches!(
            decode_tables_commitment(&bad, 6, None),
            Err(TablesDecodeError::MalformedTables(_))
        ));
    }
}
