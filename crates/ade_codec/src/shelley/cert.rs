// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use crate::cbor::{self, ContainerEncoding};
use crate::error::CodecError;
use ade_types::{Hash28, EpochNo};
use ade_types::tx::PoolId;

/// Decoded certificate from a Shelley+ transaction body.
///
/// Certificates appear in tx body key 4 as an array of cert values.
/// Each cert is `array(N) [type_tag, ...]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Certificate {
    /// [0, stake_credential] — register a stake credential for rewards.
    StakeRegistration { credential: StakeCredential },
    /// [1, stake_credential] — deregister a stake credential.
    StakeDeregistration { credential: StakeCredential },
    /// [2, stake_credential, pool_hash] — delegate stake to a pool.
    StakeDelegation { credential: StakeCredential, pool: PoolId },
    /// [3, ...pool_params...] — register or update a stake pool.
    PoolRegistration { pool_id: PoolId, raw: Vec<u8> },
    /// [4, pool_hash, epoch] — schedule pool retirement at epoch.
    PoolRetirement { pool_id: PoolId, epoch: EpochNo },
    /// Any other cert type — stored as opaque bytes.
    Other { tag: u64, raw: Vec<u8> },
}

/// Stake credential — either a key hash or a script hash.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StakeCredential {
    /// [0, hash28] — key hash credential.
    KeyHash(Hash28),
    /// [1, hash28] — script hash credential.
    ScriptHash(Hash28),
}

impl StakeCredential {
    /// Extract the 28-byte hash regardless of credential type.
    pub fn hash(&self) -> &Hash28 {
        match self {
            StakeCredential::KeyHash(h) => h,
            StakeCredential::ScriptHash(h) => h,
        }
    }
}

/// Decode a stake credential: `[type_tag, hash28]`.
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

    let (tag, _) = cbor::read_uint(data, offset)?;
    let (hash_bytes, _) = cbor::read_bytes(data, offset)?;

    if hash_bytes.len() != 28 {
        return Err(CodecError::InvalidLength {
            offset: *offset - hash_bytes.len(),
            detail: "stake credential hash must be 28 bytes",
        });
    }

    let mut arr = [0u8; 28];
    arr.copy_from_slice(&hash_bytes);
    let hash = Hash28(arr);

    match tag {
        0 => Ok(StakeCredential::KeyHash(hash)),
        1 => Ok(StakeCredential::ScriptHash(hash)),
        _ => Err(CodecError::InvalidCborStructure {
            offset: *offset,
            detail: "unknown stake credential type",
        }),
    }
}

/// Decode certificates from opaque CBOR bytes (tx body key 4).
///
/// The input is the raw CBOR of the certificates array.
pub fn decode_certificates(data: &[u8]) -> Result<Vec<Certificate>, CodecError> {
    let mut offset = 0;

    // Handle optional tag(258) wrapping (Conway sets)
    let major = (data[offset] >> 5) & 0x7;
    if major == 6 {
        let _ = cbor::read_tag(data, &mut offset)?;
    }

    let enc = cbor::read_array_header(data, &mut offset)?;
    let count = match enc {
        ContainerEncoding::Definite(n, _) => n,
        ContainerEncoding::Indefinite => {
            let mut certs = Vec::new();
            while !cbor::is_break(data, offset)? {
                certs.push(decode_single_certificate(data, &mut offset)?);
            }
            return Ok(certs);
        }
    };

    let mut certs = Vec::with_capacity(count as usize);
    for _ in 0..count {
        certs.push(decode_single_certificate(data, &mut offset)?);
    }
    Ok(certs)
}

fn decode_single_certificate(
    data: &[u8],
    offset: &mut usize,
) -> Result<Certificate, CodecError> {
    let cert_start = *offset;
    let enc = cbor::read_array_header(data, offset)?;
    let _arr_len = match enc {
        ContainerEncoding::Definite(n, _) => n,
        _ => {
            return Err(CodecError::InvalidCborStructure {
                offset: *offset,
                detail: "certificate must be definite-length array",
            });
        }
    };

    let (tag, _) = cbor::read_uint(data, offset)?;

    match tag {
        0 => {
            // StakeRegistration: [0, credential]
            let credential = decode_stake_credential(data, offset)?;
            Ok(Certificate::StakeRegistration { credential })
        }
        1 => {
            // StakeDeregistration: [1, credential]
            let credential = decode_stake_credential(data, offset)?;
            Ok(Certificate::StakeDeregistration { credential })
        }
        2 => {
            // StakeDelegation: [2, credential, pool_hash]
            let credential = decode_stake_credential(data, offset)?;
            let (pool_bytes, _) = cbor::read_bytes(data, offset)?;
            if pool_bytes.len() != 28 {
                return Err(CodecError::InvalidLength {
                    offset: *offset - pool_bytes.len(),
                    detail: "pool hash must be 28 bytes",
                });
            }
            let mut arr = [0u8; 28];
            arr.copy_from_slice(&pool_bytes);
            Ok(Certificate::StakeDelegation {
                credential,
                pool: PoolId(Hash28(arr)),
            })
        }
        3 => {
            // PoolRegistration: [3, pool_hash, vrf_hash, pledge, cost, margin, reward_account, ...]
            // Complex — capture pool_id and store rest as opaque
            let (pool_bytes, _) = cbor::read_bytes(data, offset)?;
            if pool_bytes.len() != 28 {
                return Err(CodecError::InvalidLength {
                    offset: *offset - pool_bytes.len(),
                    detail: "pool registration hash must be 28 bytes",
                });
            }
            let mut arr = [0u8; 28];
            arr.copy_from_slice(&pool_bytes);
            let pool_id = PoolId(Hash28(arr));

            // Skip remaining fields (vrf_hash, pledge, cost, margin, reward_account, owners, relays, metadata)
            let cert_end = skip_to_cert_end(data, offset, cert_start)?;
            let raw = data[cert_start..cert_end].to_vec();

            Ok(Certificate::PoolRegistration { pool_id, raw })
        }
        4 => {
            // PoolRetirement: [4, pool_hash, epoch]
            let (pool_bytes, _) = cbor::read_bytes(data, offset)?;
            if pool_bytes.len() != 28 {
                return Err(CodecError::InvalidLength {
                    offset: *offset - pool_bytes.len(),
                    detail: "pool retirement hash must be 28 bytes",
                });
            }
            let mut arr = [0u8; 28];
            arr.copy_from_slice(&pool_bytes);
            let (epoch_val, _) = cbor::read_uint(data, offset)?;

            Ok(Certificate::PoolRetirement {
                pool_id: PoolId(Hash28(arr)),
                epoch: EpochNo(epoch_val),
            })
        }
        _ => {
            // Other cert types — skip and capture raw
            let cert_end = skip_to_cert_end(data, offset, cert_start)?;
            let raw = data[cert_start..cert_end].to_vec();
            Ok(Certificate::Other { tag, raw })
        }
    }
}

/// Skip remaining fields in a certificate array to find its end offset.
fn skip_to_cert_end(
    data: &[u8],
    offset: &mut usize,
    cert_start: usize,
) -> Result<usize, CodecError> {
    // Re-skip the entire cert from cert_start to find the end.
    let mut skip_off = cert_start;
    let (_start, end) = cbor::skip_item(data, &mut skip_off)?;
    *offset = end;
    Ok(end)
}
