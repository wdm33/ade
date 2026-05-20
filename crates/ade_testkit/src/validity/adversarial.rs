// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN adversarial block mutators (PHASE4-B1, B1-S7).
//!
//! Non-authoritative: each mutator takes a real corpus block's envelope bytes
//! (`[era, inner_block]`) and applies a single targeted corruption that violates
//! one spec rule, returning fresh CBOR. The harness then asserts the BLUE
//! `block_validity` authority rejects each mutation fail-closed — never `Valid`.
//!
//! The mutators locate exact field byte-spans by walking the header CBOR with
//! the same `ade_codec::cbor` primitives the BLUE decoder uses, rather than
//! blind byte-flips. The spans are derived from the canonical Babbage/Conway
//! header-body layout:
//!
//!   envelope  = array(2)[ era(uint), inner_block(bytes-free array) ]
//!   inner     = array(N)[ header, tx_bodies, witness_sets, .. ]
//!   header    = array(2)[ header_body, kes_signature ]
//!   body      = array(10)[ block_no, slot, prev_hash, issuer_vkey,
//!                          vrf_vkey, vrf_result, body_size, body_hash,
//!                          op_cert, protocol_version ]
//!   vrf_result= array(2)[ output(bytes 64), proof(bytes 80) ]
//!
//! `BTreeMap`/`Vec` only; no `HashMap`, no float, no clock.

use ade_codec::cbor::{self, ContainerEncoding};
use ade_crypto::blake2b::blake2b_256;
use ade_ledger::block_validity::BlockRejectClass;

/// The named adversarial mutations (B1-S7 §9, M1–M6). Each carries the spec
/// rule it violates (for the README provenance) and the reject class the BLUE
/// authority is expected to assign.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mutation {
    /// M1 — truncate the header VRF proof (80 → 79 bytes).
    TruncateVrfProof,
    /// M2 — tamper the header VRF vkey so `blake2b256(vkey) != registered keyhash`.
    TamperVrfVkey,
    /// M3 — flip a byte in the KES signature.
    FlipKesSignatureByte,
    /// M4 — set the header slot far beyond the EraSchedule forecast horizon.
    SlotBeyondHorizon,
    /// M5 — flip a byte in the block body; header `body_hash` left unchanged.
    FlipBodyByte,
    /// M6 — forge a witness (fabricated `[vkey, sig]`), then patch the header
    /// `body_hash` to the mutated body. The highest-value case: a forged spend.
    ///
    /// FINDING (B1-S7): in this node the KES signature signs the header BODY,
    /// which includes the `body_hash` field, and the whole header pipeline
    /// (including KES, step 7b) runs BEFORE the body-hash gate and body
    /// validation in `block_validity`. Patching `body_hash` to reach body
    /// validation therefore invalidates the KES signature first, so this
    /// mutation lands fail-closed at `HeaderInvalid` (KES) — never `Valid`.
    /// This is correct, secure behavior: the header crypto commits to the body,
    /// so a forged spend cannot pass `block_validity` by any byte mutation of a
    /// real block. (A forged spend that did NOT patch `body_hash` is M5's
    /// `BodyHashMismatch`.) See `corpus/validity/adversarial/README.md` for the
    /// separate, deeper note on Conway body-level witness verification, which is
    /// reachable only by bypassing the header and is out of B1-S7's scope.
    ForgeWitnessPatchHash,
}

impl Mutation {
    /// All mutations, in stable order.
    pub const ALL: [Mutation; 6] = [
        Mutation::TruncateVrfProof,
        Mutation::TamperVrfVkey,
        Mutation::FlipKesSignatureByte,
        Mutation::SlotBeyondHorizon,
        Mutation::FlipBodyByte,
        Mutation::ForgeWitnessPatchHash,
    ];

    /// Stable short name (matches the README rows).
    pub fn name(self) -> &'static str {
        match self {
            Mutation::TruncateVrfProof => "M1_truncate_vrf_proof",
            Mutation::TamperVrfVkey => "M2_tamper_vrf_vkey",
            Mutation::FlipKesSignatureByte => "M3_flip_kes_signature_byte",
            Mutation::SlotBeyondHorizon => "M4_slot_beyond_horizon",
            Mutation::FlipBodyByte => "M5_flip_body_byte",
            Mutation::ForgeWitnessPatchHash => "M6_forge_witness_patch_hash",
        }
    }

    /// The reject class B1-S7 §9 documents for this mutation.
    pub fn expected_class(self) -> BlockRejectClass {
        match self {
            Mutation::TruncateVrfProof => BlockRejectClass::MalformedField,
            Mutation::TamperVrfVkey => BlockRejectClass::HeaderInvalid,
            Mutation::FlipKesSignatureByte => BlockRejectClass::HeaderInvalid,
            Mutation::SlotBeyondHorizon => BlockRejectClass::HeaderInvalid,
            Mutation::FlipBodyByte => BlockRejectClass::BodyHashMismatch,
            // See the `ForgeWitnessPatchHash` doc: KES (which signs the header
            // body incl. `body_hash`) fences this fail-closed at the header.
            Mutation::ForgeWitnessPatchHash => BlockRejectClass::HeaderInvalid,
        }
    }

    /// Apply this mutation to a real corpus block's envelope bytes.
    pub fn apply(self, envelope: &[u8]) -> Result<Vec<u8>, MutateError> {
        match self {
            Mutation::TruncateVrfProof => truncate_vrf_proof(envelope),
            Mutation::TamperVrfVkey => tamper_vrf_vkey(envelope),
            Mutation::FlipKesSignatureByte => flip_kes_signature_byte(envelope),
            Mutation::SlotBeyondHorizon => slot_beyond_horizon(envelope),
            Mutation::FlipBodyByte => flip_body_byte(envelope),
            Mutation::ForgeWitnessPatchHash => forge_witness_patch_hash(envelope),
        }
    }
}

/// Mutator failure — only ever a structural mismatch with the expected real
/// corpus block layout. A real corpus block never produces this.
#[derive(Debug)]
pub enum MutateError {
    Codec(ade_codec::CodecError),
    /// The mutation target span could not be located in the expected layout.
    UnexpectedLayout(&'static str),
}

impl std::fmt::Display for MutateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MutateError::Codec(e) => write!(f, "codec: {e}"),
            MutateError::UnexpectedLayout(d) => write!(f, "unexpected layout: {d}"),
        }
    }
}

impl std::error::Error for MutateError {}

fn codec(e: ade_codec::CodecError) -> MutateError {
    MutateError::Codec(e)
}

/// Byte-span (absolute offsets into the full envelope) of the fields a mutator
/// targets. A `Span` is the half-open range `[start, end)`.
#[derive(Debug, Clone, Copy)]
struct Span {
    start: usize,
    end: usize,
}

/// Located spans within a Babbage/Conway block envelope. All spans are absolute
/// offsets into the full envelope buffer.
struct Layout {
    /// Header-body `slot` uint item.
    slot: Span,
    /// VRF vkey byte-string VALUE (not including the CBOR header).
    vrf_vkey_value: Span,
    /// VRF combined-cert proof byte-string: header span + value span.
    vrf_proof_header: Span,
    vrf_proof_value: Span,
    /// `body_hash` byte-string VALUE (32 bytes).
    body_hash_value: Span,
    /// KES signature item (second element of `array(2)` header).
    kes_signature: Span,
    /// The witness-sets item of the inner block (second-or-later element).
    witness_sets: Span,
    /// The whole tx_bodies / witness_sets / metadata / invalid segments needed
    /// to recompute the era-correct body hash for M6.
    tx_bodies: Span,
    metadata: Span,
    invalid_txs: Option<Span>,
}

/// Walk the envelope and locate every span the mutators need. Mirrors the BLUE
/// `header_input` / `shelley::block` decoders' field order.
fn locate(envelope: &[u8]) -> Result<Layout, MutateError> {
    use ade_codec::cbor::envelope::decode_block_envelope;

    let env = decode_block_envelope(envelope).map_err(codec)?;
    let inner_start = env.block_start;
    let inner = &envelope[inner_start..env.block_end];

    // inner = array(N)[ header, tx_bodies, witness_sets, metadata, invalid? ]
    let mut o = 0usize;
    let n = match cbor::read_array_header(inner, &mut o).map_err(codec)? {
        ContainerEncoding::Definite(n, _) => n,
        _ => return Err(MutateError::UnexpectedLayout("inner block not definite array")),
    };

    // Element 0: header (array(2)[ header_body, kes_signature ]). Walk INTO the
    // header to capture the header-body field spans and the KES-signature span,
    // then resume the top-level walk from the position AFTER the whole header.
    let _ = cbor::read_array_header(inner, &mut o).map_err(codec)?; // header array(2)

    // Header body (element 0 of the header) — capture its inner field spans.
    let mut hb_o = o;
    let (slot, vrf_vkey_value, vrf_proof_header, vrf_proof_value, body_hash_value) =
        locate_header_body(inner, &mut hb_o, inner_start)?;
    // Resume the top-level walk by skipping the whole header body, then the KES
    // signature, off the ORIGINAL `o` so positions stay correct regardless of
    // how far the field-span walk advanced.
    let _ = cbor::skip_item(inner, &mut o).map_err(codec)?; // header body (element 0)
    let (kes_lo, kes_hi) = cbor::skip_item(inner, &mut o).map_err(codec)?; // kes sig (element 1)
    let kes_signature = Span {
        start: inner_start + kes_lo,
        end: inner_start + kes_hi,
    };

    // Element 1: tx_bodies.
    let (tx_lo, tx_hi) = cbor::skip_item(inner, &mut o).map_err(codec)?;
    let tx_bodies = Span {
        start: inner_start + tx_lo,
        end: inner_start + tx_hi,
    };
    // Element 2: witness_sets.
    let (ws_lo, ws_hi) = cbor::skip_item(inner, &mut o).map_err(codec)?;
    let witness_sets = Span {
        start: inner_start + ws_lo,
        end: inner_start + ws_hi,
    };
    // Element 3: metadata (auxiliary data map).
    let (md_lo, md_hi) = cbor::skip_item(inner, &mut o).map_err(codec)?;
    let metadata = Span {
        start: inner_start + md_lo,
        end: inner_start + md_hi,
    };
    // Element 4 (Alonzo+): invalid-transactions index set.
    let invalid_txs = if n >= 5 {
        let (iv_lo, iv_hi) = cbor::skip_item(inner, &mut o).map_err(codec)?;
        Some(Span {
            start: inner_start + iv_lo,
            end: inner_start + iv_hi,
        })
    } else {
        None
    };

    Ok(Layout {
        slot,
        vrf_vkey_value,
        vrf_proof_header,
        vrf_proof_value,
        body_hash_value,
        kes_signature,
        witness_sets,
        tx_bodies,
        metadata,
        invalid_txs,
    })
}

/// Walk the header body (array(10)) from `*o` (already inside the header
/// array(2)) and return the spans of: slot, vrf_vkey value, vrf proof
/// header+value, body_hash value. `base` is added to convert inner-relative
/// offsets into absolute envelope offsets.
#[allow(clippy::type_complexity)]
fn locate_header_body(
    inner: &[u8],
    o: &mut usize,
    base: usize,
) -> Result<(Span, Span, Span, Span, Span), MutateError> {
    let _ = cbor::read_array_header(inner, o).map_err(codec)?; // header body array
    let _ = cbor::read_uint(inner, o).map_err(codec)?; // block_no

    // slot (uint) — capture its whole item span.
    let slot_lo = *o;
    let _ = cbor::read_uint(inner, o).map_err(codec)?;
    let slot = Span {
        start: base + slot_lo,
        end: base + *o,
    };

    let _ = cbor::skip_item(inner, o).map_err(codec)?; // prev_hash (bytes32)
    let _ = read_bytes_span(inner, o)?; // issuer_vkey

    // vrf_vkey (bytes) — capture VALUE span.
    let vrf_vkey = read_bytes_span(inner, o)?;

    // vrf_result combined cert: array(2)[ output(bytes64), proof(bytes80) ].
    match cbor::read_array_header(inner, o).map_err(codec)? {
        ContainerEncoding::Definite(2, _) => {}
        _ => return Err(MutateError::UnexpectedLayout("vrf_result not array(2)")),
    }
    let _ = read_bytes_span(inner, o)?; // vrf output
    let proof = read_bytes_span(inner, o)?; // vrf proof

    let _ = cbor::read_uint(inner, o).map_err(codec)?; // body_size

    // body_hash (bytes32) — capture VALUE span.
    let body_hash = read_bytes_span(inner, o)?;

    Ok((
        slot,
        Span {
            start: base + vrf_vkey.value_start,
            end: base + vrf_vkey.value_end,
        },
        Span {
            start: base + proof.header_start,
            end: base + proof.value_start,
        },
        Span {
            start: base + proof.value_start,
            end: base + proof.value_end,
        },
        Span {
            start: base + body_hash.value_start,
            end: base + body_hash.value_end,
        },
    ))
}

/// Inner-relative spans of a CBOR byte-string item: the header (length prefix)
/// and the value.
struct BytesSpan {
    header_start: usize,
    value_start: usize,
    value_end: usize,
}

/// Read a byte-string item, returning its header and value spans (inner-relative).
fn read_bytes_span(data: &[u8], o: &mut usize) -> Result<BytesSpan, MutateError> {
    let header_start = *o;
    let (bytes, _) = cbor::read_bytes(data, o).map_err(codec)?;
    let value_end = *o;
    let value_start = value_end - bytes.len();
    Ok(BytesSpan {
        header_start,
        value_start,
        value_end,
    })
}

// ---- M1: truncate the header VRF proof (80 -> 79 bytes) -------------------

fn truncate_vrf_proof(envelope: &[u8]) -> Result<Vec<u8>, MutateError> {
    let layout = locate(envelope)?;
    let header = layout.vrf_proof_header;
    let value = layout.vrf_proof_value;
    if value.end - value.start != 80 {
        return Err(MutateError::UnexpectedLayout("vrf proof not 80 bytes"));
    }
    // Rewrite the byte-string header to length 79 and drop the last value byte.
    // The CBOR length prefix for 0x4F..0x57 is inline; 79 (0x4F) still fits a
    // single-byte minor for a major-2 string with a 1-byte length argument
    // (0x58 0x4F), matching the original 80's `0x58 0x50` two-byte form.
    let mut out = Vec::with_capacity(envelope.len());
    out.extend_from_slice(&envelope[..header.start]);
    // 0x58 = major 2 (bytes), 1-byte length argument; 0x4F = 79.
    out.push(0x58);
    out.push(79);
    out.extend_from_slice(&envelope[value.start..value.end - 1]);
    out.extend_from_slice(&envelope[value.end..]);
    Ok(out)
}

// ---- M2: tamper the header VRF vkey ---------------------------------------

fn tamper_vrf_vkey(envelope: &[u8]) -> Result<Vec<u8>, MutateError> {
    let layout = locate(envelope)?;
    let v = layout.vrf_vkey_value;
    let mut out = envelope.to_vec();
    // Flip a single value byte so blake2b256(vkey) diverges from the registered
    // keyhash. The length (32) is preserved, so this lands VrfKeyhashMismatch
    // (step 5) rather than a malformed-field reject.
    out[v.start] ^= 0xFF;
    Ok(out)
}

// ---- M3: flip a byte in the KES signature ---------------------------------

fn flip_kes_signature_byte(envelope: &[u8]) -> Result<Vec<u8>, MutateError> {
    let layout = locate(envelope)?;
    let k = layout.kes_signature;
    if k.end <= k.start + 1 {
        return Err(MutateError::UnexpectedLayout("kes signature too short"));
    }
    let mut out = envelope.to_vec();
    // Flip a byte well inside the value (skip the CBOR header byte). The KES
    // signature is NOT part of the header body the VRF binds, so VRF
    // verification still passes and the mutation reaches the KES check.
    out[k.end - 1] ^= 0xFF;
    Ok(out)
}

// ---- M4: header slot far beyond the EraSchedule horizon -------------------

fn slot_beyond_horizon(envelope: &[u8]) -> Result<Vec<u8>, MutateError> {
    let layout = locate(envelope)?;
    let s = layout.slot;
    // Replace the slot uint item with a canonical 8-byte u64 set far past the
    // mainnet Conway forecast horizon. The block_body_size / hashes do not
    // depend on the slot for the M4 reject (forecast-horizon is step 1, before
    // any hashing), so re-encoding only the slot item is sufficient.
    let mut new_slot = Vec::new();
    cbor::write_uint_canonical(&mut new_slot, u64::MAX);
    let mut out = Vec::with_capacity(envelope.len() + new_slot.len());
    out.extend_from_slice(&envelope[..s.start]);
    out.extend_from_slice(&new_slot);
    out.extend_from_slice(&envelope[s.end..]);
    Ok(out)
}

// ---- M5: flip a body byte, header body_hash unchanged ---------------------

fn flip_body_byte(envelope: &[u8]) -> Result<Vec<u8>, MutateError> {
    let layout = locate(envelope)?;
    // Flip a byte inside the LARGEST body segment so the change lands on a data
    // byte (not a structural boundary), keeping the body decodable while
    // changing its content — and thus the recomputed body hash. The header is
    // untouched, so the header↔body binding (step 4) is the gate that rejects
    // it (BodyHashMismatch). For a degenerate empty block (all body segments a
    // single CBOR header byte) the flip lands on that byte, which is still a
    // fail-closed reject (the load-bearing invariant: never `Valid`).
    let mut best = layout.tx_bodies;
    for seg in [layout.witness_sets, layout.metadata]
        .into_iter()
        .chain(layout.invalid_txs)
    {
        if seg.end - seg.start > best.end - best.start {
            best = seg;
        }
    }
    if best.end <= best.start {
        return Err(MutateError::UnexpectedLayout("no body segment to flip"));
    }
    // Offset 3 past the segment's CBOR header is reliably a data byte for a
    // non-trivial segment; clamp for tiny segments.
    let pos = best.start + 3usize.min(best.end - best.start - 1);
    let mut out = envelope.to_vec();
    out[pos] ^= 0xFF;
    Ok(out)
}

// ---- M6: forge a witness AND patch the header body_hash -------------------

fn forge_witness_patch_hash(envelope: &[u8]) -> Result<Vec<u8>, MutateError> {
    let layout = locate(envelope)?;

    // Build a forged witness set: a Shelley/Alonzo+ witness-set map whose key 0
    // (vkeywitnesses) holds one fabricated `[vkey(32), sig(64)]` pair. The vkey
    // and signature are fixed nonzero patterns that cannot verify against any
    // real tx body — the body authority's Ed25519 check (fail-closed via
    // `from_bytes`) must reject the spend.
    let forged_ws = forged_witness_set();

    // Replace the original witness-sets segment with the forged one, then
    // recompute the era-correct body hash over the mutated body and patch the
    // header's body_hash so the block passes the M5 gate and reaches step 5.
    let ws = layout.witness_sets;
    let mut mutated = Vec::with_capacity(envelope.len());
    mutated.extend_from_slice(&envelope[..ws.start]);
    mutated.extend_from_slice(&forged_ws);
    mutated.extend_from_slice(&envelope[ws.end..]);

    // Recompute spans on the mutated envelope (the witness-sets length changed,
    // shifting later offsets) so we patch the right body_hash bytes and read the
    // right segments for the hash.
    let mlayout = locate(&mutated)?;
    let new_body_hash = era_body_hash(&mutated, &mlayout);

    let bh = mlayout.body_hash_value;
    if bh.end - bh.start != 32 {
        return Err(MutateError::UnexpectedLayout("body_hash not 32 bytes"));
    }
    let mut out = mutated;
    out[bh.start..bh.end].copy_from_slice(&new_body_hash);
    Ok(out)
}

/// The era-correct (Alonzo+ segwit) block body hash over the preserved segment
/// bytes — mirrors `header_input::block_body_hash`.
fn era_body_hash(envelope: &[u8], layout: &Layout) -> [u8; 32] {
    let tx = &envelope[layout.tx_bodies.start..layout.tx_bodies.end];
    let ws = &envelope[layout.witness_sets.start..layout.witness_sets.end];
    let md = &envelope[layout.metadata.start..layout.metadata.end];
    let iv: &[u8] = match layout.invalid_txs {
        Some(s) => &envelope[s.start..s.end],
        None => &[],
    };

    let h_tx = blake2b_256(tx).0;
    let h_ws = blake2b_256(ws).0;
    let h_md = blake2b_256(md).0;
    let h_iv = blake2b_256(iv).0;

    let mut concat = [0u8; 128];
    concat[0..32].copy_from_slice(&h_tx);
    concat[32..64].copy_from_slice(&h_ws);
    concat[64..96].copy_from_slice(&h_md);
    concat[96..128].copy_from_slice(&h_iv);
    blake2b_256(&concat).0
}

/// A fabricated witness set: one transaction's worth of vkey witnesses holding
/// a single forged `[vkey(32), sig(64)]` pair. Encoded as `map(1){ 0 => [ pair ] }`.
fn forged_witness_set() -> Vec<u8> {
    let mut pair = Vec::new();
    cbor::write_array_header(
        &mut pair,
        ContainerEncoding::Definite(2, ade_codec::cbor::IntWidth::Inline),
    );
    cbor::write_bytes_canonical(&mut pair, &[0x11u8; 32]); // forged vkey
    cbor::write_bytes_canonical(&mut pair, &[0x22u8; 64]); // forged signature

    let mut vkey_witnesses = Vec::new();
    cbor::write_array_header(
        &mut vkey_witnesses,
        ContainerEncoding::Definite(1, ade_codec::cbor::IntWidth::Inline),
    );
    vkey_witnesses.extend_from_slice(&pair);

    // The block-level witness-sets element is an array, one map per transaction.
    let mut tx_ws = Vec::new();
    cbor::write_map_header(
        &mut tx_ws,
        ContainerEncoding::Definite(1, ade_codec::cbor::IntWidth::Inline),
    );
    cbor::write_uint_canonical(&mut tx_ws, 0); // key 0 = vkeywitnesses
    tx_ws.extend_from_slice(&vkey_witnesses);

    let mut out = Vec::new();
    cbor::write_array_header(
        &mut out,
        ContainerEncoding::Definite(1, ade_codec::cbor::IntWidth::Inline),
    );
    out.extend_from_slice(&tx_ws);
    out
}
