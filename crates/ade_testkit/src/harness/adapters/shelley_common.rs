use std::collections::BTreeMap;

use ade_codec::cbor::envelope::decode_block_envelope;
use ade_codec::shelley::block::decode_shelley_block_inner;
use ade_types::shelley::block::ShelleyBlock;

use crate::harness::block_diff::BlockFields;
use crate::harness::{Era, HarnessError};

/// Generic block field extractor for Shelley-structured eras (Shelley, Allegra, Mary).
///
/// All three eras share the same block structure: array(4) with 15-field header body.
pub fn decode_post_shelley_block_fields(
    raw_cbor: &[u8],
    era_name: &str,
    era_tag: u64,
    era: Era,
) -> Result<BlockFields, HarnessError> {
    let envelope = decode_block_envelope(raw_cbor)
        .map_err(|e| HarnessError::DecodingError(format!("envelope: {e}")))?;

    let inner = &raw_cbor[envelope.block_start..envelope.block_end];
    let mut offset = 0;
    let block: ShelleyBlock = decode_shelley_block_inner(inner, &mut offset)
        .map_err(|e| HarnessError::DecodingError(format!("{era_name} decode: {e}")))?;

    let hb = &block.header.body;
    let oc = &hb.operational_cert;
    let pv = &hb.protocol_version;

    let mut fields = BTreeMap::new();

    fields.insert("era".into(), serde_json::json!(era_name));
    fields.insert("block_number".into(), serde_json::json!(hb.block_number));
    fields.insert("slot".into(), serde_json::json!(hb.slot));
    fields.insert(
        "prev_block_hash".into(),
        serde_json::json!(hex_encode(&hb.prev_hash.0)),
    );
    fields.insert(
        "issuer_vkey_hash".into(),
        serde_json::json!(hex_encode(&hb.issuer_vkey)),
    );
    fields.insert(
        "vrf_vkey".into(),
        serde_json::json!(hex_encode(&hb.vrf_vkey)),
    );
    fields.insert("body_size".into(), serde_json::json!(hb.body_size));
    fields.insert(
        "body_hash".into(),
        serde_json::json!(hex_encode(&hb.body_hash.0)),
    );

    let oc_json = serde_json::json!({
        "hot_vkey": hex_encode(&oc.hot_vkey),
        "sequence_number": oc.sequence_number,
        "kes_period": oc.kes_period,
    });
    fields.insert("operational_cert".into(), oc_json);

    let pv_json = serde_json::json!({
        "major": pv.major,
        "minor": pv.minor,
    });
    fields.insert("protocol_version".into(), pv_json);

    fields.insert("tx_count".into(), serde_json::json!(block.tx_count));
    fields.insert("hfc_era_tag".into(), serde_json::json!(era_tag));
    fields.insert("source_file_size".into(), serde_json::json!(raw_cbor.len()));

    let hash = blake2b_256(raw_cbor);
    fields.insert(
        "source_blake2b_256".into(),
        serde_json::json!(hex_encode(&hash)),
    );

    Ok(BlockFields { era, fields })
}

pub(crate) fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use core::fmt::Write;
        let _ = write!(s, "{byte:02x}");
    }
    s
}

pub(crate) fn blake2b_256(data: &[u8]) -> [u8; 32] {
    use blake2::digest::Digest;
    let mut hasher = blake2::Blake2b::<blake2::digest::consts::U32>::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut output = [0u8; 32];
    output.copy_from_slice(&result);
    output
}
