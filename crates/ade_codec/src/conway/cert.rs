// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use crate::cbor::{self, ContainerEncoding};
use crate::error::CodecError;
use ade_types::conway::cert::{ConwayCert, DRep};
use ade_types::shelley::cert::StakeCredential;
use ade_types::tx::{Coin, PoolId};
use ade_types::EpochNo;
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
        2 => {
            let credential = decode_stake_credential(data, offset)?;
            let pool_id = read_pool_id(data, offset)?;
            ConwayCert::StakeDelegation { credential, pool_id }
        }
        3 => {
            // Reuse the single shared pool_params decoder (era-stable, PO-1).
            let cert = crate::shelley::cert::read_pool_registration_cert(data, offset)?;
            ConwayCert::PoolRegistration(cert)
        }
        4 => {
            let pool_id = read_pool_id(data, offset)?;
            let (epoch, _) = cbor::read_uint(data, offset)?;
            ConwayCert::PoolRetirement {
                pool_id,
                epoch: EpochNo(epoch),
            }
        }
        5 | 6 => ConwayCert::RemovedInConway { tag },
        7 => {
            let credential = decode_stake_credential(data, offset)?;
            let deposit = read_coin(data, offset)?;
            ConwayCert::AccountRegistrationDeposit { credential, deposit }
        }
        8 => {
            let credential = decode_stake_credential(data, offset)?;
            let refund = read_coin(data, offset)?;
            ConwayCert::AccountUnregistrationDeposit { credential, refund }
        }
        9 => {
            let credential = decode_stake_credential(data, offset)?;
            let drep = decode_drep(data, offset)?;
            ConwayCert::VoteDelegation { credential, drep }
        }
        10 => {
            let credential = decode_stake_credential(data, offset)?;
            let pool_id = read_pool_id(data, offset)?;
            let drep = decode_drep(data, offset)?;
            ConwayCert::StakeVoteDelegation {
                credential,
                pool_id,
                drep,
            }
        }
        11 => {
            let credential = decode_stake_credential(data, offset)?;
            let pool_id = read_pool_id(data, offset)?;
            let deposit = read_coin(data, offset)?;
            ConwayCert::StakeRegistrationDelegation {
                credential,
                pool_id,
                deposit,
            }
        }
        12 => {
            let credential = decode_stake_credential(data, offset)?;
            let drep = decode_drep(data, offset)?;
            let deposit = read_coin(data, offset)?;
            ConwayCert::VoteRegistrationDelegation {
                credential,
                drep,
                deposit,
            }
        }
        13 => {
            let credential = decode_stake_credential(data, offset)?;
            let pool_id = read_pool_id(data, offset)?;
            let drep = decode_drep(data, offset)?;
            let deposit = read_coin(data, offset)?;
            ConwayCert::StakeVoteRegistrationDelegation {
                credential,
                pool_id,
                drep,
                deposit,
            }
        }
        14 => {
            let cold_credential = decode_stake_credential(data, offset)?;
            let hot_credential = decode_stake_credential(data, offset)?;
            ConwayCert::AuthCommitteeHot {
                cold_credential,
                hot_credential,
            }
        }
        15 => {
            let cold_credential = decode_stake_credential(data, offset)?;
            // anchor/nil: consumed by the trailing cert-array skip below.
            ConwayCert::ResignCommitteeCold { cold_credential }
        }
        16 => {
            let drep_credential = decode_stake_credential(data, offset)?;
            let deposit = read_coin(data, offset)?;
            // anchor/nil: consumed by the trailing cert-array skip below.
            ConwayCert::DRepRegistration {
                drep_credential,
                deposit,
            }
        }
        17 => {
            let drep_credential = decode_stake_credential(data, offset)?;
            let refund = read_coin(data, offset)?;
            ConwayCert::DRepUnregistration {
                drep_credential,
                refund,
            }
        }
        18 => {
            let drep_credential = decode_stake_credential(data, offset)?;
            // anchor/nil: consumed by the trailing cert-array skip below.
            ConwayCert::DRepUpdate { drep_credential }
        }
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
    let cred_type_offset = *offset;
    let (cred_type, _) = cbor::read_uint(data, offset)?;
    let hash = read_hash28(data, offset)?;
    match cred_type {
        0 => Ok(StakeCredential::KeyHash(hash)),
        1 => Ok(StakeCredential::ScriptHash(hash)),
        _ => Err(CodecError::InvalidCborStructure {
            offset: cred_type_offset,
            detail: "unknown stake credential type",
        }),
    }
}

/// Decode a `drep = [0, addr_keyhash // 1, script_hash // 2 // 3]` delegation
/// target into the closed [`DRep`] grammar. No catch-all: an unknown DRep
/// variant tag rejects deterministically.
fn decode_drep(data: &[u8], offset: &mut usize) -> Result<DRep, CodecError> {
    let enc = cbor::read_array_header(data, offset)?;
    match enc {
        ContainerEncoding::Definite(n, _) if n >= 1 => {}
        _ => {
            return Err(CodecError::InvalidCborStructure {
                offset: *offset,
                detail: "drep must be a non-empty definite-length array",
            });
        }
    }
    let variant_offset = *offset;
    let (variant, _) = cbor::read_uint(data, offset)?;
    let drep = match variant {
        0 => DRep::KeyHash(read_hash28(data, offset)?),
        1 => DRep::ScriptHash(read_hash28(data, offset)?),
        2 => DRep::AlwaysAbstain,
        3 => DRep::AlwaysNoConfidence,
        _ => {
            return Err(CodecError::InvalidCborStructure {
                offset: variant_offset,
                detail: "unknown drep variant tag",
            });
        }
    };
    Ok(drep)
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
