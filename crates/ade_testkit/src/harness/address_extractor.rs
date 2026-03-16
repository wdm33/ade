use std::collections::BTreeSet;

use ade_codec::cbor;
use ade_codec::cbor::envelope::decode_block_envelope;

use super::HarnessError;

/// Extract unique raw address bytes from a decoded block's transaction outputs.
///
/// For Byron blocks, addresses are in the transaction outputs within the body.
/// For post-Byron blocks, addresses are the first element of each tx output.
///
/// This is a best-effort extractor that works with the opaque body bytes.
/// It returns raw address byte strings as found in transaction outputs.
pub fn extract_addresses(raw_cbor: &[u8]) -> Result<BTreeSet<Vec<u8>>, HarnessError> {
    let env = decode_block_envelope(raw_cbor)
        .map_err(|e| HarnessError::DecodingError(format!("envelope: {e}")))?;

    let inner = &raw_cbor[env.block_start..env.block_end];
    let mut offset = 0;
    let mut addresses = BTreeSet::new();

    if env.era.is_byron() {
        extract_byron_addresses(inner, &mut offset, &mut addresses)?;
    } else {
        extract_post_byron_addresses(inner, &mut offset, &mut addresses)?;
    }

    Ok(addresses)
}

fn extract_byron_addresses(
    data: &[u8],
    offset: &mut usize,
    _addresses: &mut BTreeSet<Vec<u8>>,
) -> Result<(), HarnessError> {
    // Byron block: array(3) [header, body, extra]
    // Body for regular blocks: array(4) [tx_payload, ssc, dlg, upd]
    // tx_payload: array(2) [tx_list, witness_list]
    // tx: array(2) [[inputs], [outputs]]
    // output: array(2) [address, lovelace]
    let enc = cbor::read_array_header(data, offset)
        .map_err(|e| HarnessError::DecodingError(format!("{e}")))?;
    let block_len = match enc {
        cbor::ContainerEncoding::Definite(n, _) => n,
        _ => return Ok(()),
    };
    if block_len < 2 {
        return Ok(());
    }

    // Skip header
    let _ =
        cbor::skip_item(data, offset).map_err(|e| HarnessError::DecodingError(format!("{e}")))?;

    // Try to parse body for tx outputs — this is best-effort
    // Just skip for now; Byron corpus blocks with transactions would need deeper parsing
    // The EBB body is just address hashes, not full addresses
    Ok(())
}

fn extract_post_byron_addresses(
    data: &[u8],
    offset: &mut usize,
    addresses: &mut BTreeSet<Vec<u8>>,
) -> Result<(), HarnessError> {
    // Post-Byron block: array(4 or 5) [header, tx_bodies, witnesses, metadata, ...]
    let enc = cbor::read_array_header(data, offset)
        .map_err(|e| HarnessError::DecodingError(format!("{e}")))?;
    let _block_len = match enc {
        cbor::ContainerEncoding::Definite(n, _) => n,
        _ => return Ok(()),
    };

    // Skip header
    let _ =
        cbor::skip_item(data, offset).map_err(|e| HarnessError::DecodingError(format!("{e}")))?;

    // tx_bodies: array of tx_body maps
    let tx_enc = cbor::read_array_header(data, offset)
        .map_err(|e| HarnessError::DecodingError(format!("{e}")))?;
    let tx_count = match tx_enc {
        cbor::ContainerEncoding::Definite(n, _) => n,
        cbor::ContainerEncoding::Indefinite => {
            // Skip indefinite array
            while !cbor::is_break(data, *offset)
                .map_err(|e| HarnessError::DecodingError(format!("{e}")))?
            {
                let _ = cbor::skip_item(data, offset)
                    .map_err(|e| HarnessError::DecodingError(format!("{e}")))?;
            }
            *offset += 1;
            return Ok(());
        }
    };

    for _ in 0..tx_count {
        extract_addresses_from_tx_body(data, offset, addresses)?;
    }

    Ok(())
}

fn extract_addresses_from_tx_body(
    data: &[u8],
    offset: &mut usize,
    addresses: &mut BTreeSet<Vec<u8>>,
) -> Result<(), HarnessError> {
    // tx_body is a CBOR map: { key: value, ... }
    // Key 1 = outputs
    let map_enc = cbor::read_map_header(data, offset)
        .map_err(|e| HarnessError::DecodingError(format!("{e}")))?;
    let map_count = match map_enc {
        cbor::ContainerEncoding::Definite(n, _) => n,
        cbor::ContainerEncoding::Indefinite => {
            while !cbor::is_break(data, *offset)
                .map_err(|e| HarnessError::DecodingError(format!("{e}")))?
            {
                let _ = cbor::skip_item(data, offset)
                    .map_err(|e| HarnessError::DecodingError(format!("{e}")))?;
                let _ = cbor::skip_item(data, offset)
                    .map_err(|e| HarnessError::DecodingError(format!("{e}")))?;
            }
            *offset += 1;
            return Ok(());
        }
    };

    for _ in 0..map_count {
        let key_start = *offset;
        let major = cbor::peek_major(data, *offset)
            .map_err(|e| HarnessError::DecodingError(format!("{e}")))?;

        if major == cbor::MAJOR_UNSIGNED {
            let (key, _) = cbor::read_uint(data, offset)
                .map_err(|e| HarnessError::DecodingError(format!("{e}")))?;

            if key == 1 {
                // Outputs array
                extract_addresses_from_outputs(data, offset, addresses)?;
            } else {
                let _ = cbor::skip_item(data, offset)
                    .map_err(|e| HarnessError::DecodingError(format!("{e}")))?;
            }
        } else {
            // Skip non-integer key + value
            *offset = key_start;
            let _ = cbor::skip_item(data, offset)
                .map_err(|e| HarnessError::DecodingError(format!("{e}")))?;
            let _ = cbor::skip_item(data, offset)
                .map_err(|e| HarnessError::DecodingError(format!("{e}")))?;
        }
    }

    Ok(())
}

fn extract_addresses_from_outputs(
    data: &[u8],
    offset: &mut usize,
    addresses: &mut BTreeSet<Vec<u8>>,
) -> Result<(), HarnessError> {
    let enc = cbor::read_array_header(data, offset)
        .map_err(|e| HarnessError::DecodingError(format!("{e}")))?;
    let count = match enc {
        cbor::ContainerEncoding::Definite(n, _) => n,
        cbor::ContainerEncoding::Indefinite => {
            while !cbor::is_break(data, *offset)
                .map_err(|e| HarnessError::DecodingError(format!("{e}")))?
            {
                let _ = cbor::skip_item(data, offset)
                    .map_err(|e| HarnessError::DecodingError(format!("{e}")))?;
            }
            *offset += 1;
            return Ok(());
        }
    };

    for _ in 0..count {
        // Each output is either:
        // - array(2 or 3): [address, value, datum_hash?]
        // - map: {0: address, 1: value, ...}
        let major = cbor::peek_major(data, *offset)
            .map_err(|e| HarnessError::DecodingError(format!("{e}")))?;

        if major == cbor::MAJOR_ARRAY {
            let out_enc = cbor::read_array_header(data, offset)
                .map_err(|e| HarnessError::DecodingError(format!("{e}")))?;
            let out_len = match out_enc {
                cbor::ContainerEncoding::Definite(n, _) => n,
                _ => {
                    // Skip remaining
                    return Ok(());
                }
            };

            if out_len >= 1 {
                // First element is address bytes
                let addr_major = cbor::peek_major(data, *offset)
                    .map_err(|e| HarnessError::DecodingError(format!("{e}")))?;
                if addr_major == cbor::MAJOR_BYTES {
                    let (addr_bytes, _) = cbor::read_bytes(data, offset)
                        .map_err(|e| HarnessError::DecodingError(format!("{e}")))?;
                    addresses.insert(addr_bytes);
                } else {
                    let _ = cbor::skip_item(data, offset)
                        .map_err(|e| HarnessError::DecodingError(format!("{e}")))?;
                }
                // Skip remaining elements
                for _ in 1..out_len {
                    let _ = cbor::skip_item(data, offset)
                        .map_err(|e| HarnessError::DecodingError(format!("{e}")))?;
                }
            }
        } else if major == cbor::MAJOR_MAP {
            // Map-format output (Babbage+)
            let out_enc = cbor::read_map_header(data, offset)
                .map_err(|e| HarnessError::DecodingError(format!("{e}")))?;
            let out_count = match out_enc {
                cbor::ContainerEncoding::Definite(n, _) => n,
                _ => return Ok(()),
            };

            for _ in 0..out_count {
                let (key, _) = cbor::read_uint(data, offset)
                    .map_err(|e| HarnessError::DecodingError(format!("{e}")))?;
                if key == 0 {
                    let addr_major = cbor::peek_major(data, *offset)
                        .map_err(|e| HarnessError::DecodingError(format!("{e}")))?;
                    if addr_major == cbor::MAJOR_BYTES {
                        let (addr_bytes, _) = cbor::read_bytes(data, offset)
                            .map_err(|e| HarnessError::DecodingError(format!("{e}")))?;
                        addresses.insert(addr_bytes);
                    } else {
                        let _ = cbor::skip_item(data, offset)
                            .map_err(|e| HarnessError::DecodingError(format!("{e}")))?;
                    }
                } else {
                    let _ = cbor::skip_item(data, offset)
                        .map_err(|e| HarnessError::DecodingError(format!("{e}")))?;
                }
            }
        } else {
            let _ = cbor::skip_item(data, offset)
                .map_err(|e| HarnessError::DecodingError(format!("{e}")))?;
        }
    }

    Ok(())
}
