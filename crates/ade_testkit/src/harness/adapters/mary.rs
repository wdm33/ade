use crate::harness::adapters::shelley_common;
use crate::harness::block_diff::BlockFields;
use crate::harness::{Era, HarnessError};

/// Extract `BlockFields` from a Mary block for differential comparison.
pub fn decode_mary_block_fields(raw_cbor: &[u8]) -> Result<BlockFields, HarnessError> {
    shelley_common::decode_post_shelley_block_fields(raw_cbor, "mary", 4, Era::Mary)
}
