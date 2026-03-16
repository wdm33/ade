use std::collections::BTreeMap;

use ade_codec::cbor;
use ade_codec::cbor::envelope::decode_block_envelope;
use ade_types::CardanoEra;

use crate::harness::block_diff::BlockFields;
use crate::harness::{Era, HarnessError};

/// Extract `BlockFields` from a Byron block for differential comparison
/// against reference oracle data.
pub fn decode_byron_block_fields(raw_cbor: &[u8]) -> Result<BlockFields, HarnessError> {
    let envelope = decode_block_envelope(raw_cbor)
        .map_err(|e| HarnessError::DecodingError(format!("envelope: {e}")))?;

    let inner = &raw_cbor[envelope.block_start..envelope.block_end];

    match envelope.era {
        CardanoEra::ByronEbb => decode_ebb_fields(inner, raw_cbor),
        CardanoEra::ByronRegular => decode_regular_fields(inner, raw_cbor),
        other => Err(HarnessError::DecodingError(format!(
            "expected Byron era, got {other}"
        ))),
    }
}

fn decode_ebb_fields(inner: &[u8], raw: &[u8]) -> Result<BlockFields, HarnessError> {
    let block = ade_codec::byron::decode_byron_ebb_block(inner)
        .map_err(|e| HarnessError::DecodingError(format!("EBB decode: {e}")))?;

    let h = &block.decoded().header;
    let mut fields = BTreeMap::new();

    fields.insert("era".into(), serde_json::Value::String("byron_ebb".into()));
    fields.insert(
        "protocol_magic".into(),
        serde_json::Value::Number(h.protocol_magic.into()),
    );
    fields.insert(
        "prev_block_hash".into(),
        serde_json::Value::String(hex_encode(&h.prev_hash.0)),
    );
    fields.insert(
        "body_proof".into(),
        serde_json::Value::String(hex_encode(&h.body_proof.0)),
    );
    fields.insert(
        "epoch".into(),
        serde_json::Value::Number(serde_json::Number::from(h.epoch)),
    );
    fields.insert("slot".into(), serde_json::Value::Null);
    fields.insert("block_number".into(), serde_json::Value::Null);
    fields.insert("hfc_era_tag".into(), serde_json::Value::Number(0.into()));
    fields.insert(
        "source_file_size".into(),
        serde_json::Value::Number(raw.len().into()),
    );

    // blake2b-256 of the raw CBOR — requires blake2 crate
    let hash = blake2b_256(raw);
    fields.insert(
        "source_blake2b_256".into(),
        serde_json::Value::String(hex_encode(&hash)),
    );

    Ok(BlockFields {
        era: Era::Byron,
        fields,
    })
}

fn decode_regular_fields(inner: &[u8], raw: &[u8]) -> Result<BlockFields, HarnessError> {
    let block = ade_codec::byron::decode_byron_regular_block(inner)
        .map_err(|e| HarnessError::DecodingError(format!("regular decode: {e}")))?;

    let h = &block.decoded().header;
    let cd = &h.consensus_data;
    let mut fields = BTreeMap::new();

    fields.insert("era".into(), serde_json::Value::String("byron".into()));
    fields.insert(
        "protocol_magic".into(),
        serde_json::Value::Number(h.protocol_magic.into()),
    );
    fields.insert(
        "prev_block_hash".into(),
        serde_json::Value::String(hex_encode(&h.prev_hash.0)),
    );

    // body_proof: parse the 4-element array into JSON
    let body_proof_json = parse_body_proof(&h.body_proof)?;
    fields.insert("body_proof".into(), body_proof_json);

    fields.insert(
        "epoch".into(),
        serde_json::Value::Number(serde_json::Number::from(cd.epoch)),
    );
    fields.insert(
        "slot_in_epoch".into(),
        serde_json::Value::Number(serde_json::Number::from(cd.slot_in_epoch)),
    );
    fields.insert(
        "chain_difficulty".into(),
        serde_json::Value::Number(serde_json::Number::from(cd.chain_difficulty)),
    );
    fields.insert("slot".into(), serde_json::Value::Null);
    fields.insert("block_number".into(), serde_json::Value::Null);
    fields.insert(
        "delegator_pubkey".into(),
        serde_json::Value::String(hex_encode(&cd.delegator_pubkey)),
    );
    fields.insert("hfc_era_tag".into(), serde_json::Value::Number(1.into()));
    fields.insert(
        "source_file_size".into(),
        serde_json::Value::Number(raw.len().into()),
    );

    let hash = blake2b_256(raw);
    fields.insert(
        "source_blake2b_256".into(),
        serde_json::Value::String(hex_encode(&hash)),
    );

    Ok(BlockFields {
        era: Era::Byron,
        fields,
    })
}

/// Parse the opaque body_proof CBOR into a JSON value matching the
/// reference oracle format.
///
/// Body proof is array(4):
///   [0]: array(3) [uint, bytes(32), bytes(32)]
///   [1]: array(3) [uint, bytes(32), bytes(32)]
///   [2]: bytes(32)
///   [3]: bytes(32)
fn parse_body_proof(proof_bytes: &[u8]) -> Result<serde_json::Value, HarnessError> {
    let mut offset = 0;
    let enc = cbor::read_array_header(proof_bytes, &mut offset)
        .map_err(|e| HarnessError::DecodingError(format!("body_proof header: {e}")))?;

    let count = match enc {
        cbor::ContainerEncoding::Definite(n, _) => n,
        _ => {
            return Err(HarnessError::DecodingError(
                "body_proof must be definite array".into(),
            ))
        }
    };

    let mut elements = Vec::new();
    for _ in 0..count {
        let major = cbor::peek_major(proof_bytes, offset)
            .map_err(|e| HarnessError::DecodingError(format!("{e}")))?;

        if major == cbor::MAJOR_ARRAY {
            // Sub-array: [uint, bytes(32), bytes(32)]
            let sub_enc = cbor::read_array_header(proof_bytes, &mut offset)
                .map_err(|e| HarnessError::DecodingError(format!("{e}")))?;
            let sub_count = match sub_enc {
                cbor::ContainerEncoding::Definite(n, _) => n,
                _ => {
                    return Err(HarnessError::DecodingError(
                        "body_proof sub must be definite".into(),
                    ))
                }
            };
            let mut sub_items = Vec::new();
            for _ in 0..sub_count {
                let sub_major = cbor::peek_major(proof_bytes, offset)
                    .map_err(|e| HarnessError::DecodingError(format!("{e}")))?;
                if sub_major == cbor::MAJOR_UNSIGNED {
                    let (val, _) = cbor::read_uint(proof_bytes, &mut offset)
                        .map_err(|e| HarnessError::DecodingError(format!("{e}")))?;
                    sub_items.push(serde_json::Value::Number(serde_json::Number::from(val)));
                } else if sub_major == cbor::MAJOR_BYTES {
                    let (bytes, _) = cbor::read_bytes(proof_bytes, &mut offset)
                        .map_err(|e| HarnessError::DecodingError(format!("{e}")))?;
                    sub_items.push(serde_json::Value::String(hex_encode(&bytes)));
                } else {
                    return Err(HarnessError::DecodingError(format!(
                        "unexpected major type {sub_major} in body_proof sub-array"
                    )));
                }
            }
            elements.push(serde_json::Value::Array(sub_items));
        } else if major == cbor::MAJOR_BYTES {
            let (bytes, _) = cbor::read_bytes(proof_bytes, &mut offset)
                .map_err(|e| HarnessError::DecodingError(format!("{e}")))?;
            elements.push(serde_json::Value::String(hex_encode(&bytes)));
        } else {
            return Err(HarnessError::DecodingError(format!(
                "unexpected major type {major} in body_proof"
            )));
        }
    }

    Ok(serde_json::Value::Array(elements))
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use core::fmt::Write;
        let _ = write!(s, "{byte:02x}");
    }
    s
}

fn blake2b_256(data: &[u8]) -> [u8; 32] {
    use blake2::digest::Digest;
    let mut hasher = blake2::Blake2b::<blake2::digest::consts::U32>::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut output = [0u8; 32];
    output.copy_from_slice(&result);
    output
}
