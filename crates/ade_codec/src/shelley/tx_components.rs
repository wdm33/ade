// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Conway transaction component splitter (BLUE).
//!
//! Splits a Conway transaction `[body, witness_set, is_valid, aux_data_or_nil]`
//! into preserved-byte slices for body, witness set, the boolean validity
//! flag, and the optional auxiliary data. The four slices reference the
//! input buffer; nothing is re-encoded.
//!
//! Pure, total, deterministic. Reused by the BLUE block-forge body
//! assembler (`ade_ledger::producer::forge`) and the body-hash parity
//! gate (S4).

use crate::cbor::{self, ContainerEncoding};
use crate::error::CodecError;

/// Preserved-byte projection of a Conway `[body, witness_set, is_valid,
/// aux_data_or_nil]` transaction. Byte slices alias the input buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxComponents<'a> {
    pub body_bytes: &'a [u8],
    pub witness_set_bytes: &'a [u8],
    pub is_valid: bool,
    /// `None` when the slot is encoded as CBOR null; otherwise the
    /// preserved CBOR bytes of the auxiliary data item.
    pub aux_data_bytes: Option<&'a [u8]>,
}

/// Split a Conway transaction CBOR slice into its four preserved-byte
/// components. The input MUST be a definite array of exactly four
/// elements with no trailing bytes; either condition produces a typed
/// [`CodecError`] reject.
pub fn split_conway_tx_components(tx_cbor: &[u8]) -> Result<TxComponents<'_>, CodecError> {
    let mut offset = 0usize;
    let enc = cbor::read_array_header(tx_cbor, &mut offset)?;
    match enc {
        ContainerEncoding::Definite(4, _) => {}
        _ => {
            return Err(CodecError::InvalidCborStructure {
                offset,
                detail: "Conway transaction must be a definite array of 4 elements",
            });
        }
    }

    let (body_start, body_end) = cbor::skip_item(tx_cbor, &mut offset)?;
    let (ws_start, ws_end) = cbor::skip_item(tx_cbor, &mut offset)?;

    let is_valid = cbor::read_bool(tx_cbor, &mut offset)?;

    let aux_start = offset;
    let (aux_lo, aux_hi) = cbor::skip_item(tx_cbor, &mut offset)?;
    let aux_bytes = &tx_cbor[aux_lo..aux_hi];
    let aux_data_bytes = if aux_bytes == [0xf6] {
        None
    } else {
        Some(&tx_cbor[aux_start..aux_hi])
    };

    if offset != tx_cbor.len() {
        return Err(CodecError::TrailingBytes {
            consumed: offset,
            total: tx_cbor.len(),
        });
    }

    Ok(TxComponents {
        body_bytes: &tx_cbor[body_start..body_end],
        witness_set_bytes: &tx_cbor[ws_start..ws_end],
        is_valid,
        aux_data_bytes,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::cbor::{write_array_header, write_bool, write_null, IntWidth};

    fn synth_tx(body: &[u8], ws: &[u8], is_valid: bool, aux: Option<&[u8]>) -> Vec<u8> {
        let mut buf = Vec::new();
        write_array_header(&mut buf, ContainerEncoding::Definite(4, IntWidth::Inline));
        buf.extend_from_slice(body);
        buf.extend_from_slice(ws);
        write_bool(&mut buf, is_valid);
        match aux {
            None => write_null(&mut buf),
            Some(a) => buf.extend_from_slice(a),
        }
        buf
    }

    #[test]
    fn split_conway_tx_components_round_trips() {
        let body = b"\xa0"; // empty map — opaque body stand-in
        let ws = b"\xa0"; // empty map
        let aux = b"\xa0"; // empty map — non-nil aux
        let tx = synth_tx(body, ws, true, Some(aux));
        let comps = split_conway_tx_components(&tx).unwrap();
        assert_eq!(comps.body_bytes, body);
        assert_eq!(comps.witness_set_bytes, ws);
        assert!(comps.is_valid);
        assert_eq!(comps.aux_data_bytes, Some(&aux[..]));

        // Re-assemble: 4-element array header + the four preserved slices.
        let mut rebuilt = Vec::new();
        write_array_header(
            &mut rebuilt,
            ContainerEncoding::Definite(4, IntWidth::Inline),
        );
        rebuilt.extend_from_slice(comps.body_bytes);
        rebuilt.extend_from_slice(comps.witness_set_bytes);
        write_bool(&mut rebuilt, comps.is_valid);
        rebuilt.extend_from_slice(comps.aux_data_bytes.unwrap());
        assert_eq!(rebuilt, tx);

        // Nil aux variant.
        let tx_nil = synth_tx(body, ws, false, None);
        let comps_nil = split_conway_tx_components(&tx_nil).unwrap();
        assert!(!comps_nil.is_valid);
        assert_eq!(comps_nil.aux_data_bytes, None);
    }

    #[test]
    fn split_conway_tx_components_rejects_short_array() {
        let mut tx = Vec::new();
        write_array_header(&mut tx, ContainerEncoding::Definite(3, IntWidth::Inline));
        tx.extend_from_slice(b"\xa0\xa0\xf5");
        let err = split_conway_tx_components(&tx).unwrap_err();
        match err {
            CodecError::InvalidCborStructure { .. } => {}
            other => panic!("expected InvalidCborStructure, got {:?}", other),
        }
    }

    #[test]
    fn split_conway_tx_components_rejects_trailing_garbage() {
        let mut tx = synth_tx(b"\xa0", b"\xa0", true, None);
        tx.push(0x00);
        let err = split_conway_tx_components(&tx).unwrap_err();
        match err {
            CodecError::TrailingBytes { .. } => {}
            other => panic!("expected TrailingBytes, got {:?}", other),
        }
    }
}
