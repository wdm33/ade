// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Admission obligations that need neither the UTxO nor protocol parameters (DC-TXV-09, DC-VAL-07).
//!
//! Each check is a pure function of the transaction or block bytes, the slot and the network id, and
//! equals cardano-ledger's predicate (Babbage `babbageUtxoValidation`, which Conway runs in both
//! `isValid` shapes, and BBODY's body-size check) or is weaker than it. None reads state, so a rejection
//! here is always a transaction or block Cardano rejects.

use ade_codec::cbor::{self, ContainerEncoding};
use ade_types::babbage::tx::BabbageTxOut;
use ade_types::conway::tx::ConwayTxBody;
use ade_types::shelley::block::ShelleyBlock;
use ade_types::SlotNo;

use super::required_signers::walk_withdrawal_accounts;
use super::script_witnesses::validity_interval;

/// Cardano's limit on a bootstrap output's extra attributes (`validateOutputBootAddrAttrsTooBig`).
pub const MAX_BOOTSTRAP_ATTRIBUTES_SIZE: usize = 64;

/// Which output of a transaction a check refers to: Cardano's `allOutputs` is the outputs plus the
/// collateral return.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputPosition {
    Output(usize),
    CollateralReturn,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UtxoFreeRejection {
    /// `OutsideValidityIntervalUTxO`.
    OutsideValidityInterval {
        invalid_before: Option<u64>,
        invalid_hereafter: Option<u64>,
        slot: u64,
    },
    /// `WrongNetwork`.
    WrongNetworkInOutput { output: OutputPosition, network: u8, expected: u8 },
    /// `WrongNetworkWithdrawal`.
    WrongNetworkWithdrawal { network: u8, expected: u8 },
    /// `WrongNetworkInTxBody`.
    WrongNetworkInTxBody { declared: u8, expected: u8 },
    /// `OutputBootAddrAttrsTooBig`.
    BootstrapAttributesTooBig { output: OutputPosition, size: usize },
    /// A bootstrap output whose attributes cannot be read; Cardano's address decoder rejects it too.
    MalformedBootstrapAddress { output: OutputPosition },
    /// An output with no address bytes.
    EmptyOutputAddress { output: OutputPosition },
    /// A withdrawal map that cannot be walked, or a reward account with no header byte.
    MalformedWithdrawals,
    /// BBODY `WrongBlockBodySizeBBODY`.
    WrongBlockBodySize { declared: u64, actual: u64 },
}

impl core::fmt::Display for UtxoFreeRejection {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{self:?}")
    }
}

/// `inInterval slot (txvldt tx)`: `invalid_before <= slot` when set, `slot < invalid_hereafter` when set.
pub fn check_validity_interval(body: &ConwayTxBody, slot: SlotNo) -> Result<(), UtxoFreeRejection> {
    let vi = validity_interval(body);
    let after_start = vi.invalid_before.is_none_or(|b| b <= slot.0);
    let before_end = vi.invalid_hereafter.is_none_or(|h| slot.0 < h);
    if after_start && before_end {
        Ok(())
    } else {
        Err(UtxoFreeRejection::OutsideValidityInterval {
            invalid_before: vi.invalid_before,
            invalid_hereafter: vi.invalid_hereafter,
            slot: slot.0,
        })
    }
}

/// The slot-free obligations: output, withdrawal and body network ids, and bootstrap attribute size.
pub fn check_tx_networks(body: &ConwayTxBody, network_id: u8) -> Result<(), UtxoFreeRejection> {
    let outputs = body
        .outputs
        .iter()
        .enumerate()
        .map(|(i, o)| (OutputPosition::Output(i), o))
        .chain(body.collateral_return.iter().map(|o| (OutputPosition::CollateralReturn, o)));
    let mut bootstrap: Vec<(OutputPosition, BootstrapAttributes)> = Vec::new();
    for (position, out) in outputs {
        let network = output_network(position, out, &mut bootstrap)?;
        if network != network_id {
            return Err(UtxoFreeRejection::WrongNetworkInOutput {
                output: position,
                network,
                expected: network_id,
            });
        }
    }

    if let Some(w) = &body.withdrawals {
        let mut wrong: Option<u8> = None;
        let mut malformed = false;
        walk_withdrawal_accounts(w, &mut |account| match account.first() {
            Some(h) if h & 0x0f != network_id => {
                wrong.get_or_insert(h & 0x0f);
            }
            Some(_) => {}
            None => malformed = true,
        })
        .map_err(|_| UtxoFreeRejection::MalformedWithdrawals)?;
        if malformed {
            return Err(UtxoFreeRejection::MalformedWithdrawals);
        }
        if let Some(network) = wrong {
            return Err(UtxoFreeRejection::WrongNetworkWithdrawal { network, expected: network_id });
        }
    }

    if let Some(declared) = body.network_id {
        if declared != network_id {
            return Err(UtxoFreeRejection::WrongNetworkInTxBody { declared, expected: network_id });
        }
    }

    for (position, attrs) in bootstrap {
        let size = attrs.derivation_payload_len + attrs.unknown_len;
        if size > MAX_BOOTSTRAP_ATTRIBUTES_SIZE {
            return Err(UtxoFreeRejection::BootstrapAttributesTooBig { output: position, size });
        }
    }
    Ok(())
}

/// Both per-transaction obligations at the block's slot.
pub fn check_utxo_free_tx(
    body: &ConwayTxBody,
    slot: SlotNo,
    network_id: u8,
) -> Result<(), UtxoFreeRejection> {
    check_validity_interval(body, slot)?;
    check_tx_networks(body, network_id)
}

/// BBODY: the header's declared body size equals the length of the four preserved body segments.
pub fn check_declared_body_size(block: &ShelleyBlock) -> Result<(), UtxoFreeRejection> {
    let actual = (block.tx_bodies.len()
        + block.witness_sets.len()
        + block.metadata.len()
        + block.invalid_txs.as_ref().map_or(0, |b| b.len())) as u64;
    let declared = block.header.body.body_size;
    if declared == actual {
        Ok(())
    } else {
        Err(UtxoFreeRejection::WrongBlockBodySize { declared, actual })
    }
}

/// An address's network id as cardano-ledger's `getNetwork` reads it: the header nibble for a Shelley
/// address; for a bootstrap address, testnet (0) iff its attributes carry the network magic (key 2),
/// else mainnet (1).
pub fn address_network(address: &[u8]) -> Result<u8, AddressNetworkError> {
    let first = *address.first().ok_or(AddressNetworkError::Empty)?;
    if first >> 4 == 0x8 {
        let attrs = read_bootstrap_attributes(address).ok_or(AddressNetworkError::MalformedBootstrap)?;
        Ok(if attrs.has_network_magic { 0 } else { 1 })
    } else {
        Ok(first & 0x0f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressNetworkError {
    Empty,
    MalformedBootstrap,
}

fn output_network(
    position: OutputPosition,
    out: &BabbageTxOut,
    bootstrap: &mut Vec<(OutputPosition, BootstrapAttributes)>,
) -> Result<u8, UtxoFreeRejection> {
    let first = *out
        .address
        .first()
        .ok_or(UtxoFreeRejection::EmptyOutputAddress { output: position })?;
    if first >> 4 == 0x8 {
        let attrs = read_bootstrap_attributes(&out.address)
            .ok_or(UtxoFreeRejection::MalformedBootstrapAddress { output: position })?;
        let network = if attrs.has_network_magic { 0 } else { 1 };
        bootstrap.push((position, attrs));
        Ok(network)
    } else {
        Ok(first & 0x0f)
    }
}

/// What the ledger reads from a bootstrap address's attributes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BootstrapAttributes {
    /// Key 2 present (`NetworkTestnet`).
    pub has_network_magic: bool,
    /// Length of the derivation-path payload: the inner bytes of key 1's value.
    pub derivation_payload_len: usize,
    /// Total length of the values of every other key (`unknownAttributesLength`).
    pub unknown_len: usize,
}

/// Read a bootstrap address `[tag24(bytes [root, attrs, type]), crc]`. Lenient: no CRC and no
/// canonical-length checks, so it accepts everything Cardano's decoder accepts.
pub fn read_bootstrap_attributes(address: &[u8]) -> Option<BootstrapAttributes> {
    let mut o = 0;
    if !matches!(cbor::read_array_header(address, &mut o).ok()?, ContainerEncoding::Definite(2, _)) {
        return None;
    }
    let (tag, _) = cbor::read_tag(address, &mut o).ok()?;
    if tag != 24 {
        return None;
    }
    let (payload, _) = cbor::read_bytes(address, &mut o).ok()?;
    cbor::skip_item(address, &mut o).ok()?; // crc
    if o != address.len() {
        return None;
    }

    let mut p = 0;
    if !matches!(cbor::read_array_header(&payload, &mut p).ok()?, ContainerEncoding::Definite(3, _)) {
        return None;
    }
    cbor::skip_item(&payload, &mut p).ok()?; // root
    let enc = cbor::read_map_header(&payload, &mut p).ok()?;
    let mut attrs = BootstrapAttributes {
        has_network_magic: false,
        derivation_payload_len: 0,
        unknown_len: 0,
    };
    let mut entry = |p: &mut usize| -> Option<()> {
        let (key, _) = cbor::read_uint(&payload, p).ok()?;
        let (value, _) = cbor::read_bytes(&payload, p).ok()?;
        match key {
            1 => {
                let mut v = 0;
                let (inner, _) = cbor::read_bytes(&value, &mut v).ok()?;
                attrs.derivation_payload_len = inner.len();
            }
            2 => attrs.has_network_magic = true,
            _ => attrs.unknown_len += value.len(),
        }
        Some(())
    };
    match enc {
        ContainerEncoding::Definite(n, _) => {
            for _ in 0..n {
                entry(&mut p)?;
            }
        }
        ContainerEncoding::Indefinite => {
            while !cbor::is_break(&payload, p).ok()? {
                entry(&mut p)?;
            }
            p += 1;
        }
    }
    cbor::skip_item(&payload, &mut p).ok()?; // type
    Some(attrs)
}
