// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use crate::cbor::{self, ContainerEncoding};
use crate::error::CodecError;
use ade_types::allegra::script::NativeScript;
use ade_types::Hash28;

/// Decode a NativeScript from CBOR.
///
/// Wire format: `array(2) [constructor_tag, payload]`
/// - Tag 0: Sig — `[0, key_hash(28)]`
/// - Tag 1: All — `[1, [scripts...]]`
/// - Tag 2: Any — `[2, [scripts...]]`
/// - Tag 3: MOfN — `[3, n, [scripts...]]` (array(3)), `n` a signed int64
/// - Tag 4: InvalidBefore — `[4, slot]`
/// - Tag 5: InvalidHereafter — `[5, slot]`
pub fn decode_native_script(
    data: &[u8],
    offset: &mut usize,
) -> Result<NativeScript, CodecError> {
    let enc = cbor::read_array_header(data, offset)?;
    let arr_len = match enc {
        ContainerEncoding::Definite(n, _) => n,
        _ => {
            return Err(CodecError::InvalidCborStructure {
                offset: *offset,
                detail: "native script must be definite-length array",
            });
        }
    };

    let (tag, _) = cbor::read_uint(data, offset)?;

    match tag {
        0 => {
            // Sig: [0, key_hash]
            let (hash_bytes, _) = cbor::read_bytes(data, offset)?;
            if hash_bytes.len() != 28 {
                return Err(CodecError::InvalidLength {
                    offset: *offset - hash_bytes.len(),
                    detail: "native script Sig key hash must be 28 bytes",
                });
            }
            let mut arr = [0u8; 28];
            arr.copy_from_slice(&hash_bytes);
            Ok(NativeScript::Sig(Hash28(arr)))
        }
        1 => {
            // All: [1, [scripts...]]
            let scripts = decode_script_array(data, offset)?;
            Ok(NativeScript::All(scripts))
        }
        2 => {
            // Any: [2, [scripts...]]
            let scripts = decode_script_array(data, offset)?;
            Ok(NativeScript::Any(scripts))
        }
        3 => {
            // MOfN: [3, n, [scripts...]]
            if arr_len != 3 {
                return Err(CodecError::InvalidCborStructure {
                    offset: *offset,
                    detail: "native script MOfN must be array(3)",
                });
            }
            let n = read_threshold(data, offset)?;
            let scripts = decode_script_array(data, offset)?;
            Ok(NativeScript::MOfN(n, scripts))
        }
        4 => {
            // InvalidBefore: [4, slot]
            let (slot, _) = cbor::read_uint(data, offset)?;
            Ok(NativeScript::InvalidBefore(slot))
        }
        5 => {
            // InvalidHereafter: [5, slot]
            let (slot, _) = cbor::read_uint(data, offset)?;
            Ok(NativeScript::InvalidHereafter(slot))
        }
        _ => Err(CodecError::InvalidCborStructure {
            offset: *offset,
            detail: "unknown native script constructor tag",
        }),
    }
}

/// Read a `script_n_of_k` threshold: CDDL `int64`, decoded by cardano-ledger as a signed `Int`.
///
/// Either CBOR integer major type is accepted; a magnitude outside `i64` is refused, as cborg's
/// `Int` decoder refuses it.
fn read_threshold(data: &[u8], offset: &mut usize) -> Result<i64, CodecError> {
    let start = *offset;
    let (arg, negative, _) = cbor::read_any_int(data, offset)?;
    let Ok(magnitude) = i64::try_from(arg) else {
        return Err(CodecError::InvalidCborStructure {
            offset: start,
            detail: "native script MOfN threshold outside int64",
        });
    };
    // Major type 1 encodes -1 - arg; with arg <= i64::MAX this is at least i64::MIN.
    Ok(if negative { -1 - magnitude } else { magnitude })
}

fn decode_script_array(
    data: &[u8],
    offset: &mut usize,
) -> Result<Vec<NativeScript>, CodecError> {
    let enc = cbor::read_array_header(data, offset)?;
    match enc {
        ContainerEncoding::Definite(n, _) => {
            let mut scripts = Vec::with_capacity(n as usize);
            for _ in 0..n {
                scripts.push(decode_native_script(data, offset)?);
            }
            Ok(scripts)
        }
        ContainerEncoding::Indefinite => {
            let mut scripts = Vec::new();
            while !cbor::is_break(data, *offset)? {
                scripts.push(decode_native_script(data, offset)?);
            }
            *offset += 1;
            Ok(scripts)
        }
    }
}

/// Decode an array of NativeScripts from CBOR (top-level array in witness set key 1).
pub fn decode_native_scripts(
    data: &[u8],
    offset: &mut usize,
) -> Result<Vec<NativeScript>, CodecError> {
    decode_script_array(data, offset)
}
