// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// PROPOSAL-PROCEDURES-DECODE PP-S1 (DC-LEDGER-11): closed Conway tx-body
// `proposal_procedures` decoder + encoder. Parallel to
// `ade_codec::conway::cert::decode_conway_certs`:
//   - single sanctioned entry point;
//   - unknown gov_action tag rejects;
//   - structural failures reject;
//   - trailing garbage rejects;
//   - empty set rejects (CIP-1694 requires non-empty);
//   - UpdateCommittee preserves the DC-LEDGER-10 StakeCredential
//     discriminant for both `removed` and `added`.
//
// Scope locks (cluster doc §OQ resolutions):
//   - `Anchor` stays opaque (existing `{ raw: Vec<u8> }` struct).
//   - `ParameterChange.update` stays opaque `Vec<u8>` (OQ-2).
//   - `NewConstitution.raw` stays opaque `Vec<u8>` (OQ-3).
//   - `return_addr` stays raw reward-account `Vec<u8>` (OQ-4).
//   - `voting_procedures` NOT touched (OQ-1).

use std::collections::{BTreeMap, BTreeSet};

use ade_types::conway::governance::{Anchor, GovAction, GovActionId, ProposalProcedure};
use ade_types::shelley::cert::StakeCredential;
use ade_types::tx::Coin;
use ade_types::{Hash28, Hash32};

use crate::cbor::{self, ContainerEncoding, IntWidth};
use crate::error::CodecError;

const NULL_BYTE: u8 = 0xf6;

/// Decode a Conway tx-body `proposal_procedures` field (key 20) into a
/// typed `Vec<ProposalProcedure>`. Closed grammar: no silent-skip arm,
/// every malformed shape rejects with a structured `CodecError`. The
/// outer CBOR item is a non-empty set (per CIP-1694); `Some(vec![])`
/// is a decode-time invariant violation.
///
/// Era-gating stays at the body-decoder layer (key 20 only decodes in
/// Conway+ eras via the existing `ProposalProceduresInPreConway` error);
/// this function trusts that gate and does NOT re-check era.
pub fn decode_proposal_procedures(data: &[u8]) -> Result<Vec<ProposalProcedure>, CodecError> {
    let mut offset = 0;

    // Per CIP-1694 wire form: `proposal_procedures` may be wrapped in
    // tag(258) for canonical sets (parity with `decode_conway_certs`).
    if offset < data.len() && cbor::peek_major(data, offset)? == 6 {
        let _ = cbor::read_tag(data, &mut offset)?;
    }

    let enc = cbor::read_array_header(data, &mut offset)?;
    let procs = match enc {
        ContainerEncoding::Definite(n, _) => {
            if n == 0 {
                return Err(CodecError::InvalidCborStructure {
                    offset,
                    detail: "proposal_procedures set must be non-empty (CIP-1694)",
                });
            }
            let mut procs = Vec::with_capacity((n as usize).min(data.len()));
            for _ in 0..n {
                procs.push(decode_proposal_procedure(data, &mut offset)?);
            }
            procs
        }
        ContainerEncoding::Indefinite => {
            let mut procs = Vec::new();
            while !cbor::is_break(data, offset)? {
                procs.push(decode_proposal_procedure(data, &mut offset)?);
            }
            offset += 1; // consume break
            if procs.is_empty() {
                return Err(CodecError::InvalidCborStructure {
                    offset,
                    detail: "proposal_procedures set must be non-empty (CIP-1694)",
                });
            }
            procs
        }
    };

    // Closed grammar: `data` is the exact CBOR item for tx-body key 20.
    // Trailing bytes after the set are malformed input — rejected,
    // not silently ignored (parity with `decode_conway_certs` and
    // `decode_withdrawals`).
    if offset != data.len() {
        return Err(CodecError::TrailingBytes {
            consumed: offset,
            total: data.len(),
        });
    }

    Ok(procs)
}

/// Encode a typed `Vec<ProposalProcedure>` back to canonical CBOR. The
/// PreservedCbor round-trip property holds for synthetic canonical
/// inputs (and any well-formed input produced by this encoder).
pub fn encode_proposal_procedures(procs: &[ProposalProcedure]) -> Vec<u8> {
    let mut buf = Vec::new();
    let n = procs.len() as u64;
    cbor::write_array_header(
        &mut buf,
        ContainerEncoding::Definite(n, cbor::canonical_width(n)),
    );
    for p in procs {
        encode_proposal_procedure(&mut buf, p);
    }
    buf
}

// ---- per-procedure ----

/// Decode one `ProposalProcedure` = `array(4)[deposit, return_addr, gov_action, anchor]` at `offset`.
/// Public so the ledger-state decoder (`ade_ledger::ledgerdb_state`) reuses the SAME closed
/// `gov_action` grammar when importing the bootstrap `Proposals` OMap — an unknown gov-action variant
/// fails closed identically on both the tx-body and ledger-state paths (no silent skip).
pub fn decode_proposal_procedure(
    data: &[u8],
    offset: &mut usize,
) -> Result<ProposalProcedure, CodecError> {
    let enc = cbor::read_array_header(data, offset)?;
    match enc {
        ContainerEncoding::Definite(4, _) => {}
        _ => {
            return Err(CodecError::InvalidCborStructure {
                offset: *offset,
                detail: "proposal_procedure must be array(4) [deposit, return_addr, gov_action, anchor]",
            });
        }
    }
    let (deposit_val, _) = cbor::read_uint(data, offset)?;
    let deposit = Coin(deposit_val);

    let (return_addr, _) = cbor::read_bytes(data, offset)?;

    let gov_action = decode_gov_action(data, offset)?;

    let anchor = decode_anchor(data, offset)?;

    Ok(ProposalProcedure {
        deposit,
        return_addr,
        gov_action,
        anchor,
    })
}

fn encode_proposal_procedure(buf: &mut Vec<u8>, p: &ProposalProcedure) {
    cbor::write_array_header(buf, ContainerEncoding::Definite(4, IntWidth::Inline));
    cbor::write_uint_canonical(buf, p.deposit.0);
    cbor::write_bytes_canonical(buf, &p.return_addr);
    encode_gov_action(buf, &p.gov_action);
    encode_anchor(buf, &p.anchor);
}

// ---- gov_action (7-variant closed sum) ----

fn decode_gov_action(data: &[u8], offset: &mut usize) -> Result<GovAction, CodecError> {
    let enc = cbor::read_array_header(data, offset)?;
    let arr_len = match enc {
        ContainerEncoding::Definite(n, _) if n >= 1 => n,
        _ => {
            return Err(CodecError::InvalidCborStructure {
                offset: *offset,
                detail: "gov_action must be non-empty definite-length array",
            });
        }
    };

    let tag_offset = *offset;
    let (tag, _) = cbor::read_uint(data, offset)?;

    let action = match (tag, arr_len) {
        (0, 4) => {
            // ParameterChange [0, prev_action/null, pparams_update, policy_hash/null]
            let prev_action = decode_gov_action_id_opt(data, offset)?;
            // OQ-2: `update` stays opaque — capture the raw CBOR bytes verbatim.
            let (start, end) = cbor::skip_item(data, offset)?;
            let update = data[start..end].to_vec();
            let policy_hash = decode_hash28_opt(data, offset)?;
            GovAction::ParameterChange {
                prev_action,
                update,
                policy_hash,
            }
        }
        (1, 3) => {
            // HardForkInitiation [1, prev_action/null, [major, minor]]
            let prev_action = decode_gov_action_id_opt(data, offset)?;
            let pv_enc = cbor::read_array_header(data, offset)?;
            match pv_enc {
                ContainerEncoding::Definite(2, _) => {}
                _ => {
                    return Err(CodecError::InvalidCborStructure {
                        offset: *offset,
                        detail: "protocol_version must be array(2) [major, minor]",
                    });
                }
            }
            let (major, _) = cbor::read_uint(data, offset)?;
            let (minor, _) = cbor::read_uint(data, offset)?;
            GovAction::HardForkInitiation {
                prev_action,
                protocol_version: (major, minor),
            }
        }
        (2, 3) => {
            // TreasuryWithdrawals [2, { reward_account => coin }, policy_hash/null]
            let m_enc = cbor::read_map_header(data, offset)?;
            let count = match m_enc {
                ContainerEncoding::Definite(n, _) => n,
                ContainerEncoding::Indefinite => {
                    return Err(CodecError::InvalidCborStructure {
                        offset: *offset,
                        detail: "treasury withdrawals map must be definite-length",
                    });
                }
            };
            let mut withdrawals = Vec::with_capacity((count as usize).min(data.len()));
            for _ in 0..count {
                let (addr, _) = cbor::read_bytes(data, offset)?;
                let (amount, _) = cbor::read_uint(data, offset)?;
                withdrawals.push((addr, Coin(amount)));
            }
            let policy_hash = decode_hash28_opt(data, offset)?;
            GovAction::TreasuryWithdrawals {
                withdrawals,
                policy_hash,
            }
        }
        (3, 2) => {
            // NoConfidence [3, prev_action/null]
            let prev_action = decode_gov_action_id_opt(data, offset)?;
            GovAction::NoConfidence { prev_action }
        }
        (4, 5) => {
            // UpdateCommittee [4, prev_action/null, set<cold_cred>,
            //                  { cold_cred => epoch_no }, unit_interval]
            let prev_action = decode_gov_action_id_opt(data, offset)?;
            let removed = decode_cold_credential_set(data, offset)?;
            let added = decode_cold_credential_epoch_map(data, offset)?;
            let threshold = decode_unit_interval(data, offset)?;
            GovAction::UpdateCommittee {
                prev_action,
                removed,
                added,
                threshold,
            }
        }
        (5, 3) => {
            // NewConstitution [5, prev_action/null, constitution]
            let prev_action = decode_gov_action_id_opt(data, offset)?;
            // OQ-3: `raw` stays opaque — capture the constitution bytes verbatim.
            let (start, end) = cbor::skip_item(data, offset)?;
            let raw = data[start..end].to_vec();
            GovAction::NewConstitution { prev_action, raw }
        }
        (6, 1) => GovAction::InfoAction,
        // PP-N1: unknown gov_action tag rejects deterministically.
        // PP-5/6: tag-vs-arity mismatch rejects too (e.g. tag 0 with arr_len != 4).
        _ => {
            return Err(CodecError::UnknownCertTag {
                tag,
                offset: tag_offset,
            });
        }
    };

    Ok(action)
}

fn encode_gov_action(buf: &mut Vec<u8>, action: &GovAction) {
    match action {
        GovAction::ParameterChange {
            prev_action,
            update,
            policy_hash,
        } => {
            cbor::write_array_header(buf, ContainerEncoding::Definite(4, IntWidth::Inline));
            cbor::write_uint_canonical(buf, 0);
            encode_gov_action_id_opt(buf, prev_action);
            buf.extend_from_slice(update); // opaque pre-encoded bytes
            encode_hash28_opt(buf, policy_hash);
        }
        GovAction::HardForkInitiation {
            prev_action,
            protocol_version: (major, minor),
        } => {
            cbor::write_array_header(buf, ContainerEncoding::Definite(3, IntWidth::Inline));
            cbor::write_uint_canonical(buf, 1);
            encode_gov_action_id_opt(buf, prev_action);
            cbor::write_array_header(buf, ContainerEncoding::Definite(2, IntWidth::Inline));
            cbor::write_uint_canonical(buf, *major);
            cbor::write_uint_canonical(buf, *minor);
        }
        GovAction::TreasuryWithdrawals {
            withdrawals,
            policy_hash,
        } => {
            cbor::write_array_header(buf, ContainerEncoding::Definite(3, IntWidth::Inline));
            cbor::write_uint_canonical(buf, 2);
            let n = withdrawals.len() as u64;
            cbor::write_map_header(
                buf,
                ContainerEncoding::Definite(n, cbor::canonical_width(n)),
            );
            for (addr, amount) in withdrawals {
                cbor::write_bytes_canonical(buf, addr);
                cbor::write_uint_canonical(buf, amount.0);
            }
            encode_hash28_opt(buf, policy_hash);
        }
        GovAction::NoConfidence { prev_action } => {
            cbor::write_array_header(buf, ContainerEncoding::Definite(2, IntWidth::Inline));
            cbor::write_uint_canonical(buf, 3);
            encode_gov_action_id_opt(buf, prev_action);
        }
        GovAction::UpdateCommittee {
            prev_action,
            removed,
            added,
            threshold,
        } => {
            cbor::write_array_header(buf, ContainerEncoding::Definite(5, IntWidth::Inline));
            cbor::write_uint_canonical(buf, 4);
            encode_gov_action_id_opt(buf, prev_action);
            encode_cold_credential_set(buf, removed);
            encode_cold_credential_epoch_map(buf, added);
            encode_unit_interval(buf, *threshold);
        }
        GovAction::NewConstitution { prev_action, raw } => {
            cbor::write_array_header(buf, ContainerEncoding::Definite(3, IntWidth::Inline));
            cbor::write_uint_canonical(buf, 5);
            encode_gov_action_id_opt(buf, prev_action);
            buf.extend_from_slice(raw); // opaque pre-encoded constitution
        }
        GovAction::InfoAction => {
            cbor::write_array_header(buf, ContainerEncoding::Definite(1, IntWidth::Inline));
            cbor::write_uint_canonical(buf, 6);
        }
    }
}

// ---- nested helpers ----

fn decode_gov_action_id_opt(
    data: &[u8],
    offset: &mut usize,
) -> Result<Option<GovActionId>, CodecError> {
    if peek_null(data, *offset)? {
        *offset += 1;
        return Ok(None);
    }
    let enc = cbor::read_array_header(data, offset)?;
    match enc {
        ContainerEncoding::Definite(2, _) => {}
        _ => {
            return Err(CodecError::InvalidCborStructure {
                offset: *offset,
                detail: "gov_action_id must be array(2) [tx_hash, index]",
            });
        }
    }
    let tx_hash = read_hash32(data, offset)?;
    let (index, _) = cbor::read_uint(data, offset)?;
    Ok(Some(GovActionId {
        tx_hash,
        index: index as u32,
    }))
}

fn encode_gov_action_id_opt(buf: &mut Vec<u8>, opt: &Option<GovActionId>) {
    match opt {
        None => cbor::write_null(buf),
        Some(id) => {
            cbor::write_array_header(buf, ContainerEncoding::Definite(2, IntWidth::Inline));
            cbor::write_bytes_canonical(buf, &id.tx_hash.0);
            cbor::write_uint_canonical(buf, id.index as u64);
        }
    }
}

fn decode_anchor(data: &[u8], offset: &mut usize) -> Result<Anchor, CodecError> {
    // OQ-3-adjacent: anchor stays opaque (`{ raw: Vec<u8> }`); the
    // OUTER frame is still validated as a single CBOR item, so a
    // structurally-invalid anchor rejects at skip_item (PP-N8).
    let (start, end) = cbor::skip_item(data, offset)?;
    Ok(Anchor {
        raw: data[start..end].to_vec(),
    })
}

fn encode_anchor(buf: &mut Vec<u8>, anchor: &Anchor) {
    buf.extend_from_slice(&anchor.raw);
}

fn decode_cold_credential_set(
    data: &[u8],
    offset: &mut usize,
) -> Result<BTreeSet<StakeCredential>, CodecError> {
    // Optional tag(258) wrapper for canonical sets.
    if *offset < data.len() && cbor::peek_major(data, *offset)? == 6 {
        let _ = cbor::read_tag(data, offset)?;
    }
    let enc = cbor::read_array_header(data, offset)?;
    let count = match enc {
        ContainerEncoding::Definite(n, _) => n,
        ContainerEncoding::Indefinite => {
            return Err(CodecError::InvalidCborStructure {
                offset: *offset,
                detail: "committee removed set must be definite-length",
            });
        }
    };
    let mut set = BTreeSet::new();
    for _ in 0..count {
        let cred = decode_stake_credential(data, offset)?;
        set.insert(cred);
    }
    Ok(set)
}

fn encode_cold_credential_set(buf: &mut Vec<u8>, set: &BTreeSet<StakeCredential>) {
    cbor::write_tag(buf, 258, cbor::canonical_width(258));
    let n = set.len() as u64;
    cbor::write_array_header(
        buf,
        ContainerEncoding::Definite(n, cbor::canonical_width(n)),
    );
    for cred in set {
        encode_stake_credential(buf, cred);
    }
}

fn decode_cold_credential_epoch_map(
    data: &[u8],
    offset: &mut usize,
) -> Result<BTreeMap<StakeCredential, u64>, CodecError> {
    let enc = cbor::read_map_header(data, offset)?;
    let count = match enc {
        ContainerEncoding::Definite(n, _) => n,
        ContainerEncoding::Indefinite => {
            return Err(CodecError::InvalidCborStructure {
                offset: *offset,
                detail: "committee added map must be definite-length",
            });
        }
    };
    let mut map = BTreeMap::new();
    for _ in 0..count {
        let cred = decode_stake_credential(data, offset)?;
        let (epoch, _) = cbor::read_uint(data, offset)?;
        map.insert(cred, epoch);
    }
    Ok(map)
}

fn encode_cold_credential_epoch_map(buf: &mut Vec<u8>, map: &BTreeMap<StakeCredential, u64>) {
    let n = map.len() as u64;
    cbor::write_map_header(
        buf,
        ContainerEncoding::Definite(n, cbor::canonical_width(n)),
    );
    for (cred, epoch) in map {
        encode_stake_credential(buf, cred);
        cbor::write_uint_canonical(buf, *epoch);
    }
}

/// Local copy of the cert-decoder StakeCredential shape: array(2) with
/// tag 0 = KeyHash, tag 1 = ScriptHash. Unknown tag rejects. DC-LEDGER-10
/// preserved end-to-end.
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

fn encode_stake_credential(buf: &mut Vec<u8>, cred: &StakeCredential) {
    cbor::write_array_header(buf, ContainerEncoding::Definite(2, IntWidth::Inline));
    match cred {
        StakeCredential::KeyHash(h) => {
            cbor::write_uint_canonical(buf, 0);
            cbor::write_bytes_canonical(buf, &h.0);
        }
        StakeCredential::ScriptHash(h) => {
            cbor::write_uint_canonical(buf, 1);
            cbor::write_bytes_canonical(buf, &h.0);
        }
    }
}

/// `unit_interval = #6.30([uint, uint])`. Accept both the tag-30-wrapped
/// array and the bare 2-element array form for decode parity with the
/// GREEN snapshot loader (`parse_unit_interval`). Encode in canonical
/// tag-30-wrapped form.
fn decode_unit_interval(data: &[u8], offset: &mut usize) -> Result<(u64, u64), CodecError> {
    if cbor::peek_major(data, *offset)? == 6 {
        let _ = cbor::read_tag(data, offset)?;
    }
    let enc = cbor::read_array_header(data, offset)?;
    match enc {
        ContainerEncoding::Definite(2, _) => {}
        _ => {
            return Err(CodecError::InvalidCborStructure {
                offset: *offset,
                detail: "unit_interval must be array(2) [numerator, denominator]",
            });
        }
    }
    let (num, _) = cbor::read_uint(data, offset)?;
    let (den, _) = cbor::read_uint(data, offset)?;
    Ok((num, den))
}

fn encode_unit_interval(buf: &mut Vec<u8>, ui: (u64, u64)) {
    cbor::write_tag(buf, 30, cbor::canonical_width(30));
    cbor::write_array_header(buf, ContainerEncoding::Definite(2, IntWidth::Inline));
    cbor::write_uint_canonical(buf, ui.0);
    cbor::write_uint_canonical(buf, ui.1);
}

fn decode_hash28_opt(data: &[u8], offset: &mut usize) -> Result<Option<Hash28>, CodecError> {
    if peek_null(data, *offset)? {
        *offset += 1;
        return Ok(None);
    }
    Ok(Some(read_hash28(data, offset)?))
}

fn encode_hash28_opt(buf: &mut Vec<u8>, opt: &Option<Hash28>) {
    match opt {
        None => cbor::write_null(buf),
        Some(h) => cbor::write_bytes_canonical(buf, &h.0),
    }
}

fn read_hash28(data: &[u8], offset: &mut usize) -> Result<Hash28, CodecError> {
    let off = *offset;
    let (bytes, _) = cbor::read_bytes(data, offset)?;
    if bytes.len() != 28 {
        return Err(CodecError::InvalidLength {
            offset: off,
            detail: "hash28 must be 28 bytes",
        });
    }
    let mut h = [0u8; 28];
    h.copy_from_slice(&bytes);
    Ok(Hash28(h))
}

fn read_hash32(data: &[u8], offset: &mut usize) -> Result<Hash32, CodecError> {
    let off = *offset;
    let (bytes, _) = cbor::read_bytes(data, offset)?;
    if bytes.len() != 32 {
        return Err(CodecError::InvalidLength {
            offset: off,
            detail: "hash32 must be 32 bytes",
        });
    }
    let mut h = [0u8; 32];
    h.copy_from_slice(&bytes);
    Ok(Hash32(h))
}

fn peek_null(data: &[u8], offset: usize) -> Result<bool, CodecError> {
    if offset >= data.len() {
        return Err(CodecError::UnexpectedEof { offset, needed: 1 });
    }
    Ok(data[offset] == NULL_BYTE)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    fn coin(n: u64) -> Coin {
        Coin(n)
    }

    fn h28(b: u8) -> Hash28 {
        Hash28([b; 28])
    }

    fn h32(b: u8) -> Hash32 {
        Hash32([b; 32])
    }

    fn synthetic_anchor() -> Anchor {
        // Canonical encoding of [text"x", h'aa'*32].
        let mut buf = Vec::new();
        cbor::write_array_header(&mut buf, ContainerEncoding::Definite(2, IntWidth::Inline));
        cbor::write_text_canonical(&mut buf, "x");
        cbor::write_bytes_canonical(&mut buf, &[0xaa; 32]);
        Anchor { raw: buf }
    }

    fn synthetic_constitution_raw() -> Vec<u8> {
        // Canonical encoding of [anchor, null] for NewConstitution.
        let mut buf = Vec::new();
        cbor::write_array_header(&mut buf, ContainerEncoding::Definite(2, IntWidth::Inline));
        buf.extend_from_slice(&synthetic_anchor().raw);
        cbor::write_null(&mut buf);
        buf
    }

    fn synthetic_pparams_update_raw() -> Vec<u8> {
        // Canonical encoding of an empty map `{}` — a degenerate-but-valid
        // pparams update for round-trip purposes. The real pparams update
        // sub-grammar is intentionally opaque here (OQ-2).
        let mut buf = Vec::new();
        cbor::write_map_header(&mut buf, ContainerEncoding::Definite(0, IntWidth::Inline));
        buf
    }

    fn pp(action: GovAction) -> ProposalProcedure {
        ProposalProcedure {
            deposit: coin(100_000_000_000),
            return_addr: vec![0xe0; 29],
            gov_action: action,
            anchor: synthetic_anchor(),
        }
    }

    fn assert_roundtrip(procs: Vec<ProposalProcedure>) {
        let bytes = encode_proposal_procedures(&procs);
        let decoded = decode_proposal_procedures(&bytes).expect("decode");
        assert_eq!(decoded, procs);
        let re_bytes = encode_proposal_procedures(&decoded);
        assert_eq!(re_bytes, bytes, "byte-identical re-encode");
    }

    #[test]
    fn roundtrip_info_action_proposal() {
        assert_roundtrip(vec![pp(GovAction::InfoAction)]);
    }

    #[test]
    fn roundtrip_hard_fork_initiation() {
        assert_roundtrip(vec![pp(GovAction::HardForkInitiation {
            prev_action: Some(GovActionId {
                tx_hash: h32(0xaa),
                index: 1,
            }),
            protocol_version: (10, 0),
        })]);
    }

    #[test]
    fn roundtrip_no_confidence() {
        assert_roundtrip(vec![pp(GovAction::NoConfidence { prev_action: None })]);
    }

    #[test]
    fn roundtrip_treasury_withdrawals() {
        assert_roundtrip(vec![pp(GovAction::TreasuryWithdrawals {
            withdrawals: vec![(vec![0xe1; 29], coin(1_000_000))],
            policy_hash: None,
        })]);
    }

    #[test]
    fn roundtrip_parameter_change() {
        assert_roundtrip(vec![pp(GovAction::ParameterChange {
            prev_action: None,
            update: synthetic_pparams_update_raw(),
            policy_hash: Some(h28(0xbb)),
        })]);
    }

    #[test]
    fn roundtrip_new_constitution() {
        assert_roundtrip(vec![pp(GovAction::NewConstitution {
            prev_action: Some(GovActionId {
                tx_hash: h32(0xcc),
                index: 0,
            }),
            raw: synthetic_constitution_raw(),
        })]);
    }

    #[test]
    fn roundtrip_update_committee() {
        let mut removed = BTreeSet::new();
        removed.insert(StakeCredential::KeyHash(h28(0x01)));
        removed.insert(StakeCredential::ScriptHash(h28(0x01))); // same 28 bytes!
        let mut added = BTreeMap::new();
        added.insert(StakeCredential::KeyHash(h28(0x02)), 500);
        assert_roundtrip(vec![pp(GovAction::UpdateCommittee {
            prev_action: None,
            removed,
            added,
            threshold: (2, 3),
        })]);
    }

    #[test]
    fn roundtrip_multi_procedure() {
        assert_roundtrip(vec![
            pp(GovAction::InfoAction),
            pp(GovAction::NoConfidence { prev_action: None }),
            pp(GovAction::HardForkInitiation {
                prev_action: None,
                protocol_version: (10, 1),
            }),
        ]);
    }

    #[test]
    fn rejects_unknown_gov_action_tag() {
        // Build: outer set with 1 proposal whose gov_action is [99].
        let mut buf = Vec::new();
        cbor::write_array_header(&mut buf, ContainerEncoding::Definite(1, IntWidth::Inline));
        cbor::write_array_header(&mut buf, ContainerEncoding::Definite(4, IntWidth::Inline));
        cbor::write_uint_canonical(&mut buf, 100_000_000_000);
        cbor::write_bytes_canonical(&mut buf, &[0xe0; 29]);
        cbor::write_array_header(&mut buf, ContainerEncoding::Definite(1, IntWidth::Inline));
        cbor::write_uint_canonical(&mut buf, 99);
        buf.extend_from_slice(&synthetic_anchor().raw);

        match decode_proposal_procedures(&buf) {
            Err(CodecError::UnknownCertTag { tag: 99, .. }) => {}
            other => panic!("expected UnknownCertTag(99), got {other:?}"),
        }
    }

    #[test]
    fn rejects_empty_proposal_procedures_set() {
        let mut buf = Vec::new();
        cbor::write_array_header(&mut buf, ContainerEncoding::Definite(0, IntWidth::Inline));
        match decode_proposal_procedures(&buf) {
            Err(CodecError::InvalidCborStructure { detail, .. }) => {
                assert!(detail.contains("non-empty"));
            }
            other => panic!("expected non-empty rejection, got {other:?}"),
        }
    }

    #[test]
    fn rejects_trailing_garbage() {
        let valid = encode_proposal_procedures(&[pp(GovAction::InfoAction)]);
        let mut with_trailing = valid.clone();
        with_trailing.push(0xff);
        match decode_proposal_procedures(&with_trailing) {
            Err(CodecError::TrailingBytes { .. }) => {}
            other => panic!("expected TrailingBytes, got {other:?}"),
        }
    }

    #[test]
    fn rejects_truncated_proposal_procedure() {
        // Array(4) declared but only 3 elements present — read fails on the
        // 4th (anchor), which manifests as UnexpectedEof.
        let mut buf = Vec::new();
        cbor::write_array_header(&mut buf, ContainerEncoding::Definite(1, IntWidth::Inline));
        cbor::write_array_header(&mut buf, ContainerEncoding::Definite(4, IntWidth::Inline));
        cbor::write_uint_canonical(&mut buf, 100_000_000_000);
        cbor::write_bytes_canonical(&mut buf, &[0xe0; 29]);
        cbor::write_array_header(&mut buf, ContainerEncoding::Definite(1, IntWidth::Inline));
        cbor::write_uint_canonical(&mut buf, 6); // InfoAction
        // No anchor — truncated.
        assert!(
            decode_proposal_procedures(&buf).is_err(),
            "truncated proposal_procedure must reject"
        );
    }

    #[test]
    fn rejects_invalid_stake_credential_in_update_committee() {
        // Build an UpdateCommittee with a stake credential whose type tag
        // is neither 0 (KeyHash) nor 1 (ScriptHash) — must reject.
        let mut buf = Vec::new();
        cbor::write_array_header(&mut buf, ContainerEncoding::Definite(1, IntWidth::Inline));
        cbor::write_array_header(&mut buf, ContainerEncoding::Definite(4, IntWidth::Inline));
        cbor::write_uint_canonical(&mut buf, 100_000_000_000);
        cbor::write_bytes_canonical(&mut buf, &[0xe0; 29]);
        // gov_action = [4, null, set(1)[[99, h'..']], {}, [0,1]] — invalid stake-cred tag 99
        cbor::write_array_header(&mut buf, ContainerEncoding::Definite(5, IntWidth::Inline));
        cbor::write_uint_canonical(&mut buf, 4);
        cbor::write_null(&mut buf);
        cbor::write_tag(&mut buf, 258, IntWidth::Inline);
        cbor::write_array_header(&mut buf, ContainerEncoding::Definite(1, IntWidth::Inline));
        cbor::write_array_header(&mut buf, ContainerEncoding::Definite(2, IntWidth::Inline));
        cbor::write_uint_canonical(&mut buf, 99); // bad tag
        cbor::write_bytes_canonical(&mut buf, &[0x01; 28]);
        cbor::write_map_header(&mut buf, ContainerEncoding::Definite(0, IntWidth::Inline));
        cbor::write_tag(&mut buf, 30, IntWidth::Inline);
        cbor::write_array_header(&mut buf, ContainerEncoding::Definite(2, IntWidth::Inline));
        cbor::write_uint_canonical(&mut buf, 0);
        cbor::write_uint_canonical(&mut buf, 1);
        buf.extend_from_slice(&synthetic_anchor().raw);

        match decode_proposal_procedures(&buf) {
            Err(CodecError::InvalidCborStructure { detail, .. }) => {
                assert!(detail.contains("unknown stake credential type"));
            }
            other => panic!("expected unknown-stake-credential reject, got {other:?}"),
        }
    }

    #[test]
    fn update_committee_keeps_stake_credential_discriminant() {
        // DC-LEDGER-10 preservation: a KeyHash(h) and a ScriptHash(h)
        // with the SAME 28 bytes must remain distinct through round-trip.
        let mut removed = BTreeSet::new();
        removed.insert(StakeCredential::KeyHash(h28(0x42)));
        removed.insert(StakeCredential::ScriptHash(h28(0x42)));
        let procs = vec![pp(GovAction::UpdateCommittee {
            prev_action: None,
            removed: removed.clone(),
            added: BTreeMap::new(),
            threshold: (1, 2),
        })];
        let bytes = encode_proposal_procedures(&procs);
        let decoded = decode_proposal_procedures(&bytes).unwrap();
        assert_eq!(decoded.len(), 1);
        match &decoded[0].gov_action {
            GovAction::UpdateCommittee { removed: r, .. } => {
                assert_eq!(r.len(), 2, "KeyHash and ScriptHash of equal bytes are distinct");
                assert!(r.contains(&StakeCredential::KeyHash(h28(0x42))));
                assert!(r.contains(&StakeCredential::ScriptHash(h28(0x42))));
            }
            other => panic!("expected UpdateCommittee, got {other:?}"),
        }
    }
}
