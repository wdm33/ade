// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use crate::cbor::{self, ContainerEncoding};
use crate::error::CodecError;
use ade_types::allegra::script::{NativeScript, NativeScriptNode};
use ade_types::Hash28;

/// A composite script whose sub-script list is still being read.
struct Open {
    /// The node to emit once the list closes, given its arity.
    node: fn(i64, u64) -> NativeScriptNode,
    threshold: i64,
    /// Sub-scripts still owed by a definite list; `None` = until a break byte.
    owed: Option<u64>,
    arity: u64,
}

/// Decode a NativeScript from CBOR.
///
/// Wire format: `array(2) [constructor_tag, payload]`
/// - Tag 0: Sig — `[0, key_hash(28)]`
/// - Tag 1: All — `[1, [scripts...]]`
/// - Tag 2: Any — `[2, [scripts...]]`
/// - Tag 3: MOfN — `[3, n, [scripts...]]` (array(3)), `n` a signed int64
/// - Tag 4: InvalidBefore — `[4, slot]`
/// - Tag 5: InvalidHereafter — `[5, slot]`
///
/// Iterative: open composites are kept on a heap stack and nodes are emitted in post-order, so the
/// call stack does not grow with nesting depth (DC-LEDGER-SCRIPT-02). Bytes are read in the same order,
/// and errors raised at the same offsets, as by the recursive decoder this replaced.
pub fn decode_native_script(
    data: &[u8],
    offset: &mut usize,
) -> Result<NativeScript, CodecError> {
    let start = *offset;
    let mut nodes: Vec<NativeScriptNode> = Vec::new();
    let mut open: Vec<Open> = Vec::new();
    loop {
        // One script head; a leaf completes here, a composite opens its sub-script list.
        let mut completed = read_script_head(data, offset, &mut nodes, &mut open)?;
        loop {
            if completed {
                match open.last_mut() {
                    None => {
                        return NativeScript::from_post_order(nodes).map_err(|_| {
                            CodecError::InvalidCborStructure {
                                offset: start,
                                detail: "native script nodes do not form one tree",
                            }
                        });
                    }
                    Some(parent) => parent.arity += 1,
                }
                completed = false;
            }
            let Some(top) = open.last_mut() else {
                // Unreachable: only a completed root leaves the stack empty.
                return Err(CodecError::InvalidCborStructure {
                    offset: *offset,
                    detail: "native script decoder lost its position",
                });
            };
            match top.owed {
                Some(0) => {}
                Some(ref mut n) => {
                    *n -= 1;
                    break;
                }
                None => {
                    if !cbor::is_break(data, *offset)? {
                        break;
                    }
                    *offset += 1;
                }
            }
            if let Some(done) = open.pop() {
                nodes.push((done.node)(done.threshold, done.arity));
                completed = true;
            }
        }
    }
}

/// Read one script's head. A leaf is pushed to `nodes` and `true` returned; a composite's sub-script
/// list header is read, the composite pushed to `open`, and `false` returned.
fn read_script_head(
    data: &[u8],
    offset: &mut usize,
    nodes: &mut Vec<NativeScriptNode>,
    open: &mut Vec<Open>,
) -> Result<bool, CodecError> {
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
            nodes.push(NativeScriptNode::Sig(Hash28(arr)));
            Ok(true)
        }
        1 => {
            // All: [1, [scripts...]]
            open.push(open_list(data, offset, |_, arity| NativeScriptNode::All { arity }, 0)?);
            Ok(false)
        }
        2 => {
            // Any: [2, [scripts...]]
            open.push(open_list(data, offset, |_, arity| NativeScriptNode::Any { arity }, 0)?);
            Ok(false)
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
            open.push(open_list(
                data,
                offset,
                |threshold, arity| NativeScriptNode::MOfN { threshold, arity },
                n,
            )?);
            Ok(false)
        }
        4 => {
            // InvalidBefore: [4, slot]
            let (slot, _) = cbor::read_uint(data, offset)?;
            nodes.push(NativeScriptNode::InvalidBefore(slot));
            Ok(true)
        }
        5 => {
            // InvalidHereafter: [5, slot]
            let (slot, _) = cbor::read_uint(data, offset)?;
            nodes.push(NativeScriptNode::InvalidHereafter(slot));
            Ok(true)
        }
        _ => Err(CodecError::InvalidCborStructure {
            offset: *offset,
            detail: "unknown native script constructor tag",
        }),
    }
}

/// Read a sub-script list header and open its composite.
fn open_list(
    data: &[u8],
    offset: &mut usize,
    node: fn(i64, u64) -> NativeScriptNode,
    threshold: i64,
) -> Result<Open, CodecError> {
    let owed = match cbor::read_array_header(data, offset)? {
        ContainerEncoding::Definite(n, _) => Some(n),
        ContainerEncoding::Indefinite => None,
    };
    Ok(Open { node, threshold, owed, arity: 0 })
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
) -> Result<Vec<(NativeScript, core::ops::Range<usize>)>, CodecError> {
    let enc = cbor::read_array_header(data, offset)?;
    let read_one = |offset: &mut usize| -> Result<(NativeScript, core::ops::Range<usize>), CodecError> {
        let start = *offset;
        let script = decode_native_script(data, offset)?;
        Ok((script, start..*offset))
    };
    match enc {
        ContainerEncoding::Definite(n, _) => {
            let mut scripts = Vec::new();
            for _ in 0..n {
                scripts.push(read_one(offset)?);
            }
            Ok(scripts)
        }
        ContainerEncoding::Indefinite => {
            let mut scripts = Vec::new();
            while !cbor::is_break(data, *offset)? {
                scripts.push(read_one(offset)?);
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
    Ok(decode_script_array(data, offset)?.into_iter().map(|(script, _)| script).collect())
}

/// Like [`decode_native_scripts`], also returning each script's byte range in `data`. A native
/// script's hash is taken over exactly these wire bytes, never over a re-encoding (T-ENC-01).
pub fn decode_native_scripts_preserved(
    data: &[u8],
    offset: &mut usize,
) -> Result<Vec<(NativeScript, core::ops::Range<usize>)>, CodecError> {
    decode_script_array(data, offset)
}
