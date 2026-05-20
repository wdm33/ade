// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use crate::cbor::{self, ContainerEncoding};
use crate::error::CodecError;
use ade_types::conway::cert::ConwayCert;
use ade_types::shelley::cert::StakeCredential;
use ade_types::tx::{Coin, PoolId};
use ade_types::Hash28;

/// Decode a Conway certificate array (tx body key 4) into the closed
/// [`ConwayCert`] grammar over CDDL tags `0..18`.
///
/// This decoder is a closed grammar: it has no fail-open catch-all arm. An
/// unknown tag (`>= 19`) rejects with [`CodecError::UnknownCertTag`]; malformed
/// CBOR rejects with the existing structured `CodecError` variants. Tags 5/6
/// (genesis-key-delegation / MIR, removed in Conway) decode to a distinct
/// marker variant, never an accept.
pub fn decode_conway_certs(data: &[u8]) -> Result<Vec<ConwayCert>, CodecError> {
    let mut offset = 0;

    // Handle optional tag(258) set wrapping (Conway sets).
    if offset < data.len() && cbor::peek_major(data, offset)? == 6 {
        let _ = cbor::read_tag(data, &mut offset)?;
    }

    let enc = cbor::read_array_header(data, &mut offset)?;
    let certs = match enc {
        ContainerEncoding::Definite(n, _) => {
            // Bound the preallocation by the remaining input: a CBOR array of `n`
            // elements needs at least `n` bytes, so capping at `data.len()`
            // cannot under-allocate for valid input and defangs a crafted huge
            // count (the loop still validates every element, hitting
            // UnexpectedEof when the data runs out).
            let mut certs = Vec::with_capacity((n as usize).min(data.len()));
            for _ in 0..n {
                certs.push(decode_single_cert(data, &mut offset)?);
            }
            certs
        }
        ContainerEncoding::Indefinite => {
            let mut certs = Vec::new();
            while !cbor::is_break(data, offset)? {
                certs.push(decode_single_cert(data, &mut offset)?);
            }
            offset += 1; // consume the break byte
            certs
        }
    };

    // Closed grammar: `data` is the exact CBOR item for tx-body key 4. Trailing
    // bytes after the cert array are malformed input, rejected — not silently
    // ignored (parity with `decode_withdrawals`).
    if offset != data.len() {
        return Err(CodecError::TrailingBytes {
            consumed: offset,
            total: data.len(),
        });
    }

    Ok(certs)
}

fn decode_single_cert(data: &[u8], offset: &mut usize) -> Result<ConwayCert, CodecError> {
    let cert_start = *offset;
    let enc = cbor::read_array_header(data, offset)?;
    match enc {
        ContainerEncoding::Definite(n, _) if n >= 1 => {}
        _ => {
            return Err(CodecError::InvalidCborStructure {
                offset: *offset,
                detail: "certificate must be a non-empty definite-length array",
            });
        }
    }

    let tag_offset = *offset;
    let (tag, _) = cbor::read_uint(data, offset)?;

    let cert = match tag {
        0 => {
            let credential = decode_stake_credential(data, offset)?;
            ConwayCert::AccountRegistration { credential }
        }
        1 => {
            let credential = decode_stake_credential(data, offset)?;
            ConwayCert::AccountUnregistration { credential }
        }
        2 => ConwayCert::StakeDelegation,
        3 => {
            let pool_id = read_pool_id(data, offset)?;
            ConwayCert::PoolRegistration { pool_id }
        }
        4 => ConwayCert::PoolRetirement,
        5 | 6 => ConwayCert::RemovedInConway { tag },
        7 => {
            let _credential = decode_stake_credential(data, offset)?;
            let deposit = read_coin(data, offset)?;
            ConwayCert::AccountRegistrationDeposit { deposit }
        }
        8 => {
            let _credential = decode_stake_credential(data, offset)?;
            let refund = read_coin(data, offset)?;
            ConwayCert::AccountUnregistrationDeposit { refund }
        }
        9 => ConwayCert::VoteDelegation,
        10 => ConwayCert::StakeVoteDelegation,
        11 => {
            let _credential = decode_stake_credential(data, offset)?;
            let _pool = read_hash28(data, offset)?;
            let deposit = read_coin(data, offset)?;
            ConwayCert::StakeRegistrationDelegation { deposit }
        }
        12 => {
            let _credential = decode_stake_credential(data, offset)?;
            skip_drep(data, offset)?;
            let deposit = read_coin(data, offset)?;
            ConwayCert::VoteRegistrationDelegation { deposit }
        }
        13 => {
            let _credential = decode_stake_credential(data, offset)?;
            let _pool = read_hash28(data, offset)?;
            skip_drep(data, offset)?;
            let deposit = read_coin(data, offset)?;
            ConwayCert::StakeVoteRegistrationDelegation { deposit }
        }
        14 => ConwayCert::AuthCommitteeHot,
        15 => ConwayCert::ResignCommitteeCold,
        16 => {
            let _drep_credential = decode_stake_credential(data, offset)?;
            let deposit = read_coin(data, offset)?;
            ConwayCert::DRepRegistration { deposit }
        }
        17 => {
            let _drep_credential = decode_stake_credential(data, offset)?;
            let refund = read_coin(data, offset)?;
            ConwayCert::DRepUnregistration { refund }
        }
        18 => ConwayCert::DRepUpdate,
        _ => {
            return Err(CodecError::UnknownCertTag {
                tag,
                offset: tag_offset,
            });
        }
    };

    // Structurally consume any unparsed trailing fields by skipping the whole
    // cert array from its start, landing the cursor at the cert's end.
    *offset = cert_start;
    let (_, end) = cbor::skip_item(data, offset)?;
    *offset = end;

    Ok(cert)
}

fn decode_stake_credential(
    data: &[u8],
    offset: &mut usize,
) -> Result<StakeCredential, CodecError> {
    let enc = cbor::read_array_header(data, offset)?;
    match enc {
        ContainerEncoding::Definite(2, _) => {}
        _ => {
            return Err(CodecError::InvalidCborStructure {
                offset: *offset,
                detail: "stake credential must be array(2)",
            });
        }
    }
    let (_cred_type, _) = cbor::read_uint(data, offset)?;
    let hash = read_hash28(data, offset)?;
    Ok(StakeCredential(hash))
}

fn skip_drep(data: &[u8], offset: &mut usize) -> Result<(), CodecError> {
    let _ = cbor::skip_item(data, offset)?;
    Ok(())
}

fn read_coin(data: &[u8], offset: &mut usize) -> Result<Coin, CodecError> {
    let (v, _) = cbor::read_uint(data, offset)?;
    Ok(Coin(v))
}

fn read_pool_id(data: &[u8], offset: &mut usize) -> Result<PoolId, CodecError> {
    Ok(PoolId(read_hash28(data, offset)?))
}

fn read_hash28(data: &[u8], offset: &mut usize) -> Result<Hash28, CodecError> {
    let (bytes, _) = cbor::read_bytes(data, offset)?;
    if bytes.len() != 28 {
        return Err(CodecError::InvalidLength {
            offset: *offset - bytes.len(),
            detail: "expected 28-byte hash",
        });
    }
    let mut arr = [0u8; 28];
    arr.copy_from_slice(&bytes);
    Ok(Hash28(arr))
}
