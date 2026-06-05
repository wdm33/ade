// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Decode an era-tagged block envelope into the canonical inputs the two
// authorities consume:
//   - a `HeaderInput` (Praos variant for Babbage/Conway) for the header
//     authority (`validate_and_apply_header`);
//   - the header CBOR sub-slice (the block hash is `blake2b_256` over it);
//   - the recomputed era-correct block body hash, for the body-hash binding.
//
// All decode/structure failures surface as a typed `BlockValidityError`; this
// module never panics on attacker-shaped input (BLUE fail-closed).

use ade_codec::cbor::{self, envelope::decode_block_envelope, ContainerEncoding};
use ade_codec::CodecError;
use ade_core::consensus::{HeaderInput, HeaderKes, HeaderVrf};
use ade_crypto::blake2b::{blake2b_224, blake2b_256};
use ade_crypto::vrf::{VrfOutput, VrfProof, VrfVerificationKey};
use ade_types::shelley::block::VrfData;
use ade_types::{BlockNo, CardanoEra, Hash28, Hash32, SlotNo};

use super::{check_header_position, BlockValidityError, FieldError, FieldKind};

/// A decoded block, projected to the inputs both authorities need.
#[derive(Debug)]
pub struct DecodedBlock {
    /// The era discriminant from the outer envelope.
    pub era: CardanoEra,
    /// Header authority input.
    pub header_input: HeaderInput,
    /// `blake2b_256(header_cbor)` — the block hash / `Point.hash`.
    pub block_hash: Hash32,
    /// Recomputed era-correct block body hash (segwit). The body-hash binding
    /// asserts this equals the validated header's `body_hash`.
    pub computed_body_hash: Hash32,
    /// Byte range of the inner era-specific block within `block_cbor` (the
    /// envelope strips the `[era, ..]` tag). The body authority
    /// (`apply_block_with_verdicts`) consumes the inner block, not the
    /// envelope.
    pub inner_start: usize,
    pub inner_end: usize,
}

/// Return the header CBOR sub-slice of an `AcceptedBlock` envelope —
/// the same bytes the validator's body-hash recipe hashes via
/// `header_cbor_slice`. Reuses the existing canonical walker; this is
/// the single header-projection authority for the producer-side server
/// pump (PHASE4-N-G, `DC-CONS-18`). No parallel splitter.
///
/// The returned slice is a contiguous subslice of `accepted.as_bytes()`
/// projected into envelope coordinates; for a Babbage/Conway envelope
/// `[era, inner_block]`, the header is the first element of
/// `inner_block`'s outer `array(N)`.
pub fn accepted_block_header_bytes(
    accepted: &crate::producer::AcceptedBlock,
) -> Result<&[u8], BlockValidityError> {
    block_header_bytes(accepted.as_bytes())
}

/// Return the header CBOR sub-slice of a canonical `[era, block]` block
/// envelope's raw bytes — the same `header_cbor_slice` recipe
/// [`accepted_block_header_bytes`] uses, factored out for callers that
/// hold the raw canonical block bytes rather than an `AcceptedBlock`
/// token (PHASE4-N-U S3: the `--mode node` served-chain projection reads
/// `StoredBlock.bytes` directly from the durable ChainDb). This is the
/// SAME single header-projection authority (`DC-CONS-18`) — no parallel
/// splitter, no second walker; the only difference is the input type.
///
/// The returned slice is a contiguous subslice of `block_cbor`. For a
/// Babbage/Conway envelope `[era, inner_block]`, the header is the first
/// element of `inner_block`'s outer `array(N)`.
pub fn block_header_bytes(block_cbor: &[u8]) -> Result<&[u8], BlockValidityError> {
    let env = decode_block_envelope(block_cbor).map_err(codec)?;
    let inner_start = env.block_start;
    let inner = &block_cbor[inner_start..env.block_end];
    let header = header_cbor_slice(inner)?;
    // Project `header` (a slice of `inner`) back into `block_cbor`
    // coordinates via slice arithmetic on the parent slice positions. The
    // walker returns `&inner[start..end]`, and `inner` is
    // `&block_cbor[inner_start..]`, so `header` lives at
    // `block_cbor[inner_start + (header.start - inner.start)]`. We use
    // `as_ptr()` arithmetic on the contiguous parent slice — both pointers
    // come from `block_cbor`, so the subtraction is safe.
    let header_start_in_inner = (header.as_ptr() as usize) - (inner.as_ptr() as usize);
    let header_start_in_bytes = inner_start + header_start_in_inner;
    Ok(&block_cbor[header_start_in_bytes..header_start_in_bytes + header.len()])
}

/// Decode an era-tagged block envelope and project it.
///
/// Conway/Babbage produce a Praos `HeaderInput`; the combined VRF cert carries
/// `[output, proof]`, and the KES material is lifted from the header. Returns a
/// typed error on any malformed structure or fixed-size field.
pub fn decode_block(block_cbor: &[u8]) -> Result<DecodedBlock, BlockValidityError> {
    let env = decode_block_envelope(block_cbor).map_err(codec)?;
    let inner = &block_cbor[env.block_start..env.block_end];

    let block = decode_inner(env.era, inner)?;
    let block = block.decoded();
    let hb = &block.header.body;

    // CN-WIRE-09 position clause (CE-G-J-3): reject a position-illegal
    // header (block_number 0 without Genesis, or > 0 with Genesis) before
    // the header authority runs. Position-AWARE — distinct from the
    // position-blind byte codec that already ran in `decode_inner`.
    check_header_position(hb.block_number, &hb.prev_hash)?;

    let header_cbor = header_cbor_slice(inner)?;
    let block_hash = Hash32(blake2b_256(header_cbor).0);
    let computed_body_hash = crate::block_body_hash::block_body_hash(block);

    let header_input = match env.era {
        CardanoEra::Babbage | CardanoEra::Conway => praos_header_input(inner, hb)?,
        other => {
            // Pre-Babbage TPraos header construction is out of this slice's
            // scope (the B1 corpus is Conway). Surface it as a typed reject
            // rather than guessing a TPraos projection.
            return Err(BlockValidityError::Body(unsupported_era(other)));
        }
    };

    Ok(DecodedBlock {
        era: env.era,
        header_input,
        block_hash,
        computed_body_hash,
        inner_start: env.block_start,
        inner_end: env.block_end,
    })
}

fn decode_inner(
    era: CardanoEra,
    inner: &[u8],
) -> Result<ade_codec::preserved::PreservedCbor<ade_types::shelley::block::ShelleyBlock>, BlockValidityError>
{
    let decoded = match era {
        CardanoEra::Babbage => ade_codec::babbage::decode_babbage_block(inner),
        CardanoEra::Conway => ade_codec::conway::decode_conway_block(inner),
        other => return Err(BlockValidityError::Body(unsupported_era(other))),
    };
    decoded.map_err(codec)
}

/// Build the Praos `HeaderInput` for a Babbage/Conway header body. Mirrors the
/// era-correct combined-VRF + KES projection proven by the B1-S5 truth test.
fn praos_header_input(
    inner: &[u8],
    hb: &ade_types::shelley::block::ShelleyHeaderBody,
) -> Result<HeaderInput, BlockValidityError> {
    let vrf_result = match &hb.vrf {
        VrfData::Combined { vrf_result } => vrf_result,
        VrfData::Split { .. } => {
            return Err(BlockValidityError::MalformedField(FieldError {
                field: FieldKind::VrfProof,
                expected: 0,
                actual: 0,
            }))
        }
    };
    let (output_bytes, proof_bytes) = parse_combined_vrf(vrf_result)?;

    let vrf_vk = VrfVerificationKey(expect_array::<32>(&hb.vrf_vkey, FieldKind::VrfVkey)?);
    let out_arr = expect_array::<64>(&output_bytes, FieldKind::VrfProof)?;
    let proof_arr = expect_array::<80>(&proof_bytes, FieldKind::VrfProof)?;

    let body_bytes = header_body_slice(inner)?.to_vec();
    let kes_signature = unwrap_cbor_bytes(&block_kes_signature(inner)?)?;

    let issuer_pool = Hash28(blake2b_224(&hb.issuer_vkey).0);

    let kes = HeaderKes {
        issuer_vkey: hb.issuer_vkey.clone(),
        kes_vkey: hb.operational_cert.hot_vkey.clone(),
        kes_signature,
        op_cert_signature: hb.operational_cert.sigma.clone(),
        header_body_bytes: body_bytes,
    };

    Ok(HeaderInput {
        slot: SlotNo(hb.slot),
        block_no: BlockNo(hb.block_number),
        body_hash: hb.body_hash.clone(),
        issuer_pool,
        op_cert_kes_period: hb.operational_cert.kes_period,
        op_cert_counter: hb.operational_cert.sequence_number,
        vrf_vk,
        vrf: HeaderVrf::Praos {
            proof: VrfProof(proof_arr),
            output: VrfOutput(out_arr),
        },
        kes: Some(kes),
    })
}

/// The Babbage/Conway combined VRF cert is `array(2)[bytes(64), bytes(80)]`.
fn parse_combined_vrf(b: &[u8]) -> Result<(Vec<u8>, Vec<u8>), BlockValidityError> {
    let mut o = 0usize;
    match cbor::read_array_header(b, &mut o).map_err(codec)? {
        ContainerEncoding::Definite(2, _) => {}
        _ => {
            return Err(BlockValidityError::MalformedField(FieldError {
                field: FieldKind::VrfProof,
                expected: 2,
                actual: 0,
            }))
        }
    }
    let (output, _) = cbor::read_bytes(b, &mut o).map_err(codec)?;
    let (proof, _) = cbor::read_bytes(b, &mut o).map_err(codec)?;
    Ok((output, proof))
}

/// The header CBOR sub-slice within an inner block. Block is `array(N)[header, ..]`;
/// the header item is the first element.
fn header_cbor_slice(inner: &[u8]) -> Result<&[u8], BlockValidityError> {
    let mut o = 0usize;
    cbor::read_array_header(inner, &mut o).map_err(codec)?;
    let (start, end) = cbor::skip_item(inner, &mut o).map_err(codec)?;
    Ok(&inner[start..end])
}

/// The header-body CBOR bytes (KES message). Header is `array(2)[body, kes_sig]`.
fn header_body_slice(inner: &[u8]) -> Result<&[u8], BlockValidityError> {
    let mut o = 0usize;
    cbor::read_array_header(inner, &mut o).map_err(codec)?;
    cbor::read_array_header(inner, &mut o).map_err(codec)?;
    let (start, end) = cbor::skip_item(inner, &mut o).map_err(codec)?;
    Ok(&inner[start..end])
}

/// The KES-signature CBOR item: the second element of the `array(2)` header.
fn block_kes_signature(inner: &[u8]) -> Result<Vec<u8>, BlockValidityError> {
    let mut o = 0usize;
    cbor::read_array_header(inner, &mut o).map_err(codec)?;
    cbor::read_array_header(inner, &mut o).map_err(codec)?;
    cbor::skip_item(inner, &mut o).map_err(codec)?; // header body
    let (start, end) = cbor::skip_item(inner, &mut o).map_err(codec)?;
    Ok(inner[start..end].to_vec())
}

/// Unwrap a CBOR `bytes(..)` item, returning the inner byte string.
fn unwrap_cbor_bytes(b: &[u8]) -> Result<Vec<u8>, BlockValidityError> {
    let mut o = 0usize;
    let (bytes, _) = cbor::read_bytes(b, &mut o).map_err(codec)?;
    Ok(bytes)
}

fn expect_array<const N: usize>(b: &[u8], field: FieldKind) -> Result<[u8; N], BlockValidityError> {
    if b.len() != N {
        return Err(BlockValidityError::MalformedField(FieldError {
            field,
            expected: N,
            actual: b.len(),
        }));
    }
    let mut a = [0u8; N];
    a.copy_from_slice(b);
    Ok(a)
}

fn codec(e: CodecError) -> BlockValidityError {
    BlockValidityError::Body(crate::error::LedgerError::from(e))
}

fn unsupported_era(_era: CardanoEra) -> crate::error::LedgerError {
    // The B1 corpus is Praos-era (Babbage/Conway). Pre-Babbage TPraos header
    // projection is out of this slice's scope; surface it as a structured
    // reject rather than guessing a projection.
    crate::error::LedgerError::from(CodecError::InvalidCborStructure {
        offset: 0,
        detail: "block_validity: unsupported pre-Babbage era",
    })
}
