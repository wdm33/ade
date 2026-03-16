use std::collections::BTreeMap;

use ade_codec::cbor::envelope::decode_block_envelope;
use ade_codec::shelley::decode_shelley_block;

use crate::harness::block_diff::BlockFields;
use crate::harness::{Era, HarnessError};

/// Extract `BlockFields` from a Shelley block for differential comparison.
pub fn decode_shelley_block_fields(raw_cbor: &[u8]) -> Result<BlockFields, HarnessError> {
    let envelope = decode_block_envelope(raw_cbor)
        .map_err(|e| HarnessError::DecodingError(format!("envelope: {e}")))?;

    let inner = &raw_cbor[envelope.block_start..envelope.block_end];
    let preserved = decode_shelley_block(inner)
        .map_err(|e| HarnessError::DecodingError(format!("shelley decode: {e}")))?;

    let block = preserved.decoded();
    let hb = &block.header.body;
    let oc = &hb.operational_cert;
    let pv = &hb.protocol_version;

    let mut fields = BTreeMap::new();

    fields.insert("era".into(), serde_json::json!("shelley"));
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
    fields.insert("hfc_era_tag".into(), serde_json::json!(2));
    fields.insert("source_file_size".into(), serde_json::json!(raw_cbor.len()));

    let hash = blake2b_256(raw_cbor);
    fields.insert(
        "source_blake2b_256".into(),
        serde_json::json!(hex_encode(&hash)),
    );

    Ok(BlockFields {
        era: Era::Shelley,
        fields,
    })
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
    ade_crypto::blake2b_256(data).0
}
