// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN loader for the committed Conway-576 block-validity corpus.
//!
//! Non-authoritative test infrastructure: it reads the small committed corpus
//! at `corpus/validity/conway_epoch576/` and assembles a `ConwayValidityCorpus`
//! for the B1 block-validity replay tests. `BTreeMap`/`Vec` only — no `HashMap`.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::Deserialize;

/// A two-place rational (numerator / denominator) as stored in the corpus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct CorpusRatio {
    pub numer: u64,
    pub denom: u64,
}

/// One issuing pool's leadership-relevant inputs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorpusPool {
    /// Pool id (blake2b-224 of the issuer vkey), 28 bytes.
    pub pool_id: [u8; 28],
    /// Registered pool VRF key hash (blake2b-256 of the VRF vkey), 32 bytes.
    pub vrf_keyhash: [u8; 32],
    /// Set-snapshot active-stake fraction (individualPoolStake).
    pub sigma: CorpusRatio,
}

/// The committed Conway-576 positive-validation corpus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConwayValidityCorpus {
    /// Raw block CBORs (era-tagged `[era, block]` envelopes), one per block.
    pub blocks: Vec<Vec<u8>>,
    /// Epoch nonce (eta0) for epoch 576.
    pub epoch_nonce: [u8; 32],
    /// Active-slots coefficient.
    pub asc: CorpusRatio,
    /// Issuing pools, keyed by pool id for deterministic lookup.
    pub pools: BTreeMap<[u8; 28], CorpusPool>,
}

/// Errors raised while loading the committed corpus.
#[derive(Debug)]
pub enum CorpusLoadError {
    Io(std::io::Error),
    Json(serde_json::Error),
    BadHexLen { field: &'static str, got: usize },
    BadHexDigit { field: &'static str },
}

impl std::fmt::Display for CorpusLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CorpusLoadError::Io(e) => write!(f, "io: {e}"),
            CorpusLoadError::Json(e) => write!(f, "json: {e}"),
            CorpusLoadError::BadHexLen { field, got } => {
                write!(f, "bad hex length for {field}: {got}")
            }
            CorpusLoadError::BadHexDigit { field } => write!(f, "bad hex digit in {field}"),
        }
    }
}

impl std::error::Error for CorpusLoadError {}

#[derive(Deserialize)]
struct PoolJson {
    pool_id: String,
    vrf_keyhash: String,
    sigma: CorpusRatio,
}

#[derive(Deserialize)]
struct ConsensusInputsJson {
    epoch_nonce: String,
    active_slots_coeff: CorpusRatio,
    pools: Vec<PoolJson>,
}

fn decode_hex<const N: usize>(s: &str, field: &'static str) -> Result<[u8; N], CorpusLoadError> {
    let bytes = s.as_bytes();
    if bytes.len() != N * 2 {
        return Err(CorpusLoadError::BadHexLen {
            field,
            got: bytes.len(),
        });
    }
    let mut out = [0u8; N];
    for (i, slot) in out.iter_mut().enumerate() {
        let hi = hex_val(bytes[i * 2], field)?;
        let lo = hex_val(bytes[i * 2 + 1], field)?;
        *slot = (hi << 4) | lo;
    }
    Ok(out)
}

fn hex_val(c: u8, field: &'static str) -> Result<u8, CorpusLoadError> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        b'A'..=b'F' => Ok(c - b'A' + 10),
        _ => Err(CorpusLoadError::BadHexDigit { field }),
    }
}

/// Repo-root `corpus/validity/conway_epoch576/` from this crate's manifest dir.
fn corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("corpus")
        .join("validity")
        .join("conway_epoch576")
}

impl ConwayValidityCorpus {
    /// Load the committed corpus from `corpus/validity/conway_epoch576/`.
    ///
    /// One file under `blocks/` may hold several concatenated `[era, block]`
    /// envelopes; each envelope becomes one entry in `blocks`. Blocks are read
    /// in sorted filename order, then concatenation order within a file, for a
    /// deterministic stream.
    pub fn load() -> Result<Self, CorpusLoadError> {
        let dir = corpus_dir();

        let json_text = std::fs::read_to_string(dir.join("consensus_inputs.json"))
            .map_err(CorpusLoadError::Io)?;
        let parsed: ConsensusInputsJson =
            serde_json::from_str(&json_text).map_err(CorpusLoadError::Json)?;

        let epoch_nonce = decode_hex::<32>(&parsed.epoch_nonce, "epoch_nonce")?;

        let mut pools: BTreeMap<[u8; 28], CorpusPool> = BTreeMap::new();
        for p in parsed.pools {
            let pool_id = decode_hex::<28>(&p.pool_id, "pool_id")?;
            let vrf_keyhash = decode_hex::<32>(&p.vrf_keyhash, "vrf_keyhash")?;
            pools.insert(
                pool_id,
                CorpusPool {
                    pool_id,
                    vrf_keyhash,
                    sigma: p.sigma,
                },
            );
        }

        let blocks = load_blocks(&dir.join("blocks"))?;

        Ok(ConwayValidityCorpus {
            blocks,
            epoch_nonce,
            asc: parsed.active_slots_coeff,
            pools,
        })
    }
}

/// Read every `*.cbor` under `dir` in sorted order and split each into its
/// constituent era-tagged block envelopes.
fn load_blocks(dir: &std::path::Path) -> Result<Vec<Vec<u8>>, CorpusLoadError> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(CorpusLoadError::Io)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "cbor").unwrap_or(false))
        .collect();
    files.sort();

    let mut blocks = Vec::new();
    for f in files {
        let data = std::fs::read(&f).map_err(CorpusLoadError::Io)?;
        for env in split_envelopes(&data) {
            blocks.push(env);
        }
    }
    Ok(blocks)
}

/// Split a buffer of one or more concatenated `[era, block]` CBOR envelopes
/// into the raw bytes of each whole envelope.
fn split_envelopes(data: &[u8]) -> Vec<Vec<u8>> {
    use ade_codec::cbor::envelope::decode_block_envelope;

    let mut out = Vec::new();
    let mut pos = 0usize;
    while pos < data.len() {
        // The envelope decoder rejects trailing bytes, so feed it exactly one
        // envelope by first locating the envelope end via its inner block end.
        match decode_block_envelope(&data[pos..]) {
            Ok(env) => {
                out.push(data[pos..pos + env.block_end].to_vec());
                pos += env.block_end;
            }
            Err(_) => {
                // A multi-envelope file makes the whole-buffer decode fail with
                // trailing bytes; fall back to manual cursor splitting.
                return split_envelopes_manual(data);
            }
        }
    }
    out
}

fn split_envelopes_manual(data: &[u8]) -> Vec<Vec<u8>> {
    use ade_codec::cbor::{self, ContainerEncoding};

    let mut out = Vec::new();
    let mut pos = 0usize;
    while pos < data.len() {
        let rest = &data[pos..];
        let mut o = 0usize;
        match cbor::read_array_header(rest, &mut o) {
            Ok(ContainerEncoding::Definite(2, _)) => {}
            _ => break,
        }
        if cbor::read_uint(rest, &mut o).is_err() {
            break;
        }
        match cbor::skip_item(rest, &mut o) {
            Ok(_) => {
                out.push(rest[..o].to_vec());
                pos += o;
            }
            Err(_) => break,
        }
    }
    out
}
