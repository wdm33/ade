use crate::harness::adapters::shelley_common;
use crate::harness::block_diff::BlockFields;
use crate::harness::{Era, HarnessError};

/// Extract `BlockFields` from a Conway block for differential comparison.
pub fn decode_conway_block_fields(raw_cbor: &[u8]) -> Result<BlockFields, HarnessError> {
    shelley_common::decode_post_shelley_block_fields(raw_cbor, "conway", 7, Era::Conway)
}
