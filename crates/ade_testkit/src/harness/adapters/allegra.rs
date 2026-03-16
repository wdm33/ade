use crate::harness::adapters::shelley_common;
use crate::harness::block_diff::BlockFields;
use crate::harness::{Era, HarnessError};

/// Extract `BlockFields` from an Allegra block for differential comparison.
pub fn decode_allegra_block_fields(raw_cbor: &[u8]) -> Result<BlockFields, HarnessError> {
    shelley_common::decode_post_shelley_block_fields(raw_cbor, "allegra", 3, Era::Allegra)
}
