// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! BLUE header-position legality — the single authority for the
//! `block_number` <-> `prev_hash` coupling (CN-WIRE-09 position clause,
//! CE-G-J-3, PHASE4-N-F-G-J S3).
//!
//! A genesis-successor block (`block_number 0` on a from-genesis chain)
//! MUST carry `PrevHash::Genesis` (CBOR null); a non-genesis block
//! (`block_number > 0`) MUST carry `PrevHash::Block(hash32)`.
//!
//! This rule is position-AWARE and lives ONLY here, called from
//! `decode_block`. It is deliberately separate from the position-BLIND
//! byte codec (`ade_codec`), which decodes `null -> Genesis` and
//! `hash32 -> Block` without consulting `block_number`. The codec
//! defines the byte representation; this function is the admission
//! authority for the coupling. The position decision is never the byte
//! decoder's to make.

use ade_types::shelley::block::PrevHash;

use super::BlockValidityError;

/// Reject a header whose `block_number`/`prev_hash` pair is
/// position-illegal: `block_number 0` <=> `PrevHash::Genesis`;
/// `block_number > 0` <=> `PrevHash::Block`. Pure, total, deterministic.
///
/// On violation, surfaces [`BlockValidityError::HeaderPositionInvalid`]
/// (coarse class `HeaderInvalid`); fail-fast, no `String` payload.
pub fn check_header_position(
    block_number: u64,
    prev_hash: &PrevHash,
) -> Result<(), BlockValidityError> {
    let prev_is_genesis = matches!(prev_hash, PrevHash::Genesis);
    // Legal iff the genesis-successor position agrees with the genesis
    // predecessor: `(block_number == 0)` is true exactly when the
    // predecessor is `Genesis`.
    if (block_number == 0) == prev_is_genesis {
        Ok(())
    } else {
        Err(BlockValidityError::HeaderPositionInvalid {
            block_number,
            prev_is_genesis,
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use ade_types::Hash32;

    #[test]
    fn header_position_zero_requires_genesis_ok() {
        assert!(check_header_position(0, &PrevHash::Genesis).is_ok());
    }

    #[test]
    fn header_position_zero_with_block_is_rejected() {
        let err = check_header_position(0, &PrevHash::Block(Hash32([0x11; 32])))
            .expect_err("block_number 0 with a Block predecessor is position-illegal");
        match err {
            BlockValidityError::HeaderPositionInvalid {
                block_number,
                prev_is_genesis,
            } => {
                assert_eq!(block_number, 0);
                assert!(!prev_is_genesis);
            }
            other => panic!("expected HeaderPositionInvalid, got {other:?}"),
        }
        // The coarse, oracle-comparable class folds into HeaderInvalid —
        // no new BlockRejectClass is introduced.
        assert_eq!(
            check_header_position(0, &PrevHash::Block(Hash32([0x11; 32])))
                .unwrap_err()
                .class(),
            crate::block_validity::BlockRejectClass::HeaderInvalid
        );
    }

    #[test]
    fn header_position_nonzero_requires_block_ok() {
        assert!(check_header_position(7, &PrevHash::Block(Hash32([0x22; 32]))).is_ok());
    }

    #[test]
    fn header_position_nonzero_with_genesis_is_rejected() {
        let err = check_header_position(7, &PrevHash::Genesis)
            .expect_err("block_number > 0 with a Genesis predecessor is position-illegal");
        match err {
            BlockValidityError::HeaderPositionInvalid {
                block_number,
                prev_is_genesis,
            } => {
                assert_eq!(block_number, 7);
                assert!(prev_is_genesis);
            }
            other => panic!("expected HeaderPositionInvalid, got {other:?}"),
        }
    }
}
