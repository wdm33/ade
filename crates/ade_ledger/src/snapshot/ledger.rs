// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! BLUE `LedgerState` snapshot encoder/decoder — assembles S1-S5
//! sub-state encoders into the full ledger snapshot bytes
//! (PHASE4-N-J S6).
//!
//! Wire shape:
//! ```text
//! array(9) [
//!   uint  era,                      // 7 (Conway-only; pre-Conway rejected)
//!   uint  max_lovelace_supply,
//!   bool  track_utxo,               // harness flag; round-tripped
//!   bytes utxo_state_encoded,       // S2 output
//!   bytes cert_state_encoded,       // S3 output
//!   bytes epoch_state_encoded,      // S4 output
//!   bytes pparams_encoded,          // S5 output
//!   null | bytes gov_state_encoded,
//!   null | bytes conway_deposit_params_encoded,
//! ]
//! ```
//!
//! Sub-state bodies ride inside `bstr` containers so the outer
//! decoder can hand each slice to its specialized decoder without
//! sharing offset state.

use ade_codec::cbor::{
    canonical_width, read_any_int, read_array_header, read_bool, read_bytes, write_array_header,
    write_bool, write_bytes_canonical, write_null, write_uint_canonical, ContainerEncoding,
};
use ade_codec::CodecError;
use ade_types::CardanoEra;

use crate::state::LedgerState;

use super::cert_state::{decode_cert_state, encode_cert_state};
use super::epoch_state::{decode_epoch_state, encode_epoch_state};
use super::error::{SnapshotDecodeError, SnapshotEncodeError, StructuralReason};
use super::gov_state::{
    decode_conway_deposit_params, decode_gov_state, decode_pparams,
    encode_conway_deposit_params, encode_gov_state, encode_pparams,
};
use super::utxo_state::{decode_utxo_state, encode_utxo_state};

const FIELDS: u64 = 9;

pub fn encode_ledger_state(state: &LedgerState) -> Result<Vec<u8>, SnapshotEncodeError> {
    if (state.era as u8) < (CardanoEra::Conway as u8) {
        return Err(SnapshotEncodeError::EraNotSupported { era: state.era });
    }
    let mut buf = Vec::new();
    write_array_header(
        &mut buf,
        ContainerEncoding::Definite(FIELDS, canonical_width(FIELDS)),
    );
    write_uint_canonical(&mut buf, state.era as u64);
    write_uint_canonical(&mut buf, state.max_lovelace_supply);
    write_bool(&mut buf, state.track_utxo);
    write_bytes_canonical(&mut buf, &encode_utxo_state(&state.utxo_state));
    write_bytes_canonical(&mut buf, &encode_cert_state(&state.cert_state));
    write_bytes_canonical(&mut buf, &encode_epoch_state(&state.epoch_state));
    write_bytes_canonical(&mut buf, &encode_pparams(&state.protocol_params));
    match &state.gov_state {
        Some(g) => write_bytes_canonical(&mut buf, &encode_gov_state(g)),
        None => write_null(&mut buf),
    }
    match &state.conway_deposit_params {
        Some(p) => write_bytes_canonical(&mut buf, &encode_conway_deposit_params(p)),
        None => write_null(&mut buf),
    }
    Ok(buf)
}

pub fn decode_ledger_state(bytes: &[u8]) -> Result<LedgerState, SnapshotDecodeError> {
    let mut o = 0usize;
    expect_array(bytes, &mut o, FIELDS)?;
    let era_u = read_u64(bytes, &mut o)?;
    let era = decode_era(era_u)?;
    if (era as u8) < (CardanoEra::Conway as u8) {
        return Err(SnapshotDecodeError::EraNotSupported { era });
    }
    let max_lovelace_supply = read_u64(bytes, &mut o)?;
    let track_utxo = read_bool(bytes, &mut o).map_err(SnapshotDecodeError::Cbor)?;
    let utxo_state = {
        let (b, _) = read_bytes(bytes, &mut o).map_err(SnapshotDecodeError::Cbor)?;
        decode_utxo_state(&b)?
    };
    let cert_state = {
        let (b, _) = read_bytes(bytes, &mut o).map_err(SnapshotDecodeError::Cbor)?;
        decode_cert_state(&b)?
    };
    let epoch_state = {
        let (b, _) = read_bytes(bytes, &mut o).map_err(SnapshotDecodeError::Cbor)?;
        decode_epoch_state(&b)?
    };
    let protocol_params = {
        let (b, _) = read_bytes(bytes, &mut o).map_err(SnapshotDecodeError::Cbor)?;
        decode_pparams(&b)?
    };
    let gov_state = read_opt_bstr(bytes, &mut o, decode_gov_state)?;
    let conway_deposit_params = read_opt_bstr(bytes, &mut o, decode_conway_deposit_params)?;
    Ok(LedgerState {
        utxo_state,
        epoch_state,
        protocol_params,
        era,
        track_utxo,
        cert_state,
        max_lovelace_supply,
        gov_state,
        conway_deposit_params,
    })
}

fn decode_era(tag: u64) -> Result<CardanoEra, SnapshotDecodeError> {
    match tag {
        0 => Ok(CardanoEra::ByronEbb),
        1 => Ok(CardanoEra::ByronRegular),
        2 => Ok(CardanoEra::Shelley),
        3 => Ok(CardanoEra::Allegra),
        4 => Ok(CardanoEra::Mary),
        5 => Ok(CardanoEra::Alonzo),
        6 => Ok(CardanoEra::Babbage),
        7 => Ok(CardanoEra::Conway),
        _ => Err(SnapshotDecodeError::Structural {
            reason: StructuralReason::EraTagOutOfRange,
        }),
    }
}

fn read_opt_bstr<T, F>(
    bytes: &[u8],
    o: &mut usize,
    decode_fn: F,
) -> Result<Option<T>, SnapshotDecodeError>
where
    F: FnOnce(&[u8]) -> Result<T, SnapshotDecodeError>,
{
    if *o >= bytes.len() {
        return Err(SnapshotDecodeError::Cbor(CodecError::UnexpectedEof {
            offset: *o,
            needed: 1,
        }));
    }
    if bytes[*o] == 0xF6 {
        *o += 1;
        return Ok(None);
    }
    let (b, _) = read_bytes(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
    Ok(Some(decode_fn(&b)?))
}

fn read_u64(bytes: &[u8], o: &mut usize) -> Result<u64, SnapshotDecodeError> {
    let (v, _is_neg, _w) = read_any_int(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
    Ok(v)
}

fn expect_array(
    bytes: &[u8],
    o: &mut usize,
    expected_len: u64,
) -> Result<(), SnapshotDecodeError> {
    let enc = read_array_header(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
    match enc {
        ContainerEncoding::Definite(n, _) if n == expected_len => Ok(()),
        _ => Err(SnapshotDecodeError::Structural {
            reason: StructuralReason::ArrayLengthMismatch,
        }),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    use ade_types::address::Address;
    use ade_types::shelley::cert::StakeCredential;
    use ade_types::tx::{Coin, PoolId, TxIn};
    use ade_types::{EpochNo, Hash28, Hash32, SlotNo};

    use crate::epoch::{MarkSnapshot, SetSnapshot, GoSnapshot, StakeSnapshot};
    use crate::fingerprint::fingerprint;
    use crate::pparams::ConwayOnlyDepositParams;
    use crate::state::ConwayGovState;
    use crate::utxo::TxOut;
    use crate::value::Value;

    fn empty_conway() -> LedgerState {
        LedgerState::new(CardanoEra::Conway)
    }

    fn populated_conway() -> LedgerState {
        let mut s = empty_conway();
        s.max_lovelace_supply = 45_000_000_000_000_000;
        s.track_utxo = true;
        s.utxo_state.utxos.insert(
            TxIn {
                tx_hash: Hash32([0x11; 32]),
                index: 0,
            },
            TxOut::ShelleyMary {
                address: vec![0x22; 29],
                value: Value {
                    coin: Coin(1_500_000),
                    multi_asset: Default::default(),
                },
            },
        );
        s.cert_state
            .delegation
            .registrations
            .insert(StakeCredential::KeyHash(Hash28([0x33; 28])), Coin(2_000_000));
        s.cert_state.delegation.delegations.insert(
            StakeCredential::KeyHash(Hash28([0x33; 28])),
            PoolId(Hash28([0x44; 28])),
        );
        s.epoch_state.epoch = EpochNo(580);
        s.epoch_state.slot = SlotNo(164_000_000);
        s.epoch_state.reserves = Coin(13_888_022_852_926_644);
        s.epoch_state.treasury = Coin(1_434_657_232_801_879);
        s.epoch_state.epoch_fees = Coin(8_321_001_400);
        let mut mark_snap = StakeSnapshot::new();
        mark_snap
            .delegations
            .insert(Hash28([0x33; 28]), (PoolId(Hash28([0x44; 28])), Coin(100)));
        mark_snap.pool_stakes.insert(PoolId(Hash28([0x44; 28])), Coin(100));
        s.epoch_state.snapshots.mark = MarkSnapshot(mark_snap.clone());
        s.epoch_state.snapshots.set = SetSnapshot(mark_snap.clone());
        s.epoch_state.snapshots.go = GoSnapshot(mark_snap);
        s.epoch_state
            .block_production
            .insert(PoolId(Hash28([0x44; 28])), 7);
        s.gov_state = Some(ConwayGovState {
            proposals: Vec::new(),
            committee: BTreeMap::new(),
            committee_quorum: (2, 3),
            drep_expiry: BTreeMap::new(),
            gov_action_lifetime: 6,
            vote_delegations: BTreeMap::new(),
            pool_voting_thresholds: vec![(1, 2)],
            drep_voting_thresholds: vec![(67, 100)],
            committee_hot_keys: BTreeMap::new(),
        });
        s.conway_deposit_params = Some(ConwayOnlyDepositParams {
            drep_deposit: Coin(500_000_000),
            gov_action_deposit: Coin(100_000_000_000),
            drep_activity: 20,
        });
        // Bonus: address that triggers Byron variant on round-trip.
        s.utxo_state.utxos.insert(
            TxIn {
                tx_hash: Hash32([0x55; 32]),
                index: 1,
            },
            TxOut::Byron {
                address: Address::Byron(vec![0x66; 24]),
                coin: Coin(2_000_000),
            },
        );
        s
    }

    #[test]
    fn ledger_state_round_trip_empty_conway() {
        let s = empty_conway();
        let bytes = encode_ledger_state(&s).expect("encode");
        let decoded = decode_ledger_state(&bytes).expect("decode");
        assert_eq!(decoded, s);
    }

    #[test]
    fn ledger_state_round_trip_populated_conway() {
        let s = populated_conway();
        let bytes = encode_ledger_state(&s).expect("encode");
        let decoded = decode_ledger_state(&bytes).expect("decode");
        assert_eq!(decoded, s);
    }

    #[test]
    fn ledger_state_encode_deterministic_across_runs() {
        let s = populated_conway();
        let a = encode_ledger_state(&s).expect("encode a");
        let b = encode_ledger_state(&s).expect("encode b");
        assert_eq!(a, b);
    }

    #[test]
    fn encode_then_decode_roundtrips_via_fingerprint() {
        let s = populated_conway();
        let bytes = encode_ledger_state(&s).expect("encode");
        let decoded = decode_ledger_state(&bytes).expect("decode");
        assert_eq!(fingerprint(&decoded), fingerprint(&s));
    }

    #[test]
    fn pre_conway_era_is_structurally_rejected_on_encode() {
        for era in [
            CardanoEra::ByronEbb,
            CardanoEra::ByronRegular,
            CardanoEra::Shelley,
            CardanoEra::Allegra,
            CardanoEra::Mary,
            CardanoEra::Alonzo,
            CardanoEra::Babbage,
        ] {
            let s = LedgerState::new(era);
            match encode_ledger_state(&s) {
                Err(SnapshotEncodeError::EraNotSupported { era: e }) => assert_eq!(e, era),
                other => panic!("expected EraNotSupported for {era:?}, got {other:?}"),
            }
        }
    }

    #[test]
    fn pre_conway_era_is_structurally_rejected_on_decode() {
        // Hand-craft array(9) with era=2 (Shelley).
        let mut s = empty_conway();
        s.max_lovelace_supply = 1;
        let mut bytes = encode_ledger_state(&s).expect("encode");
        // Replace the era field (immediately after array header). For Conway=7
        // encoded as 0x07 (single byte uint), patch to 0x02.
        // Array header for definite array(9) = 0x89; era follows as next byte.
        assert_eq!(bytes[0], 0x89);
        assert_eq!(bytes[1], 0x07);
        bytes[1] = 0x02;
        match decode_ledger_state(&bytes) {
            Err(SnapshotDecodeError::EraNotSupported { era }) => {
                assert_eq!(era, CardanoEra::Shelley)
            }
            other => panic!("expected EraNotSupported, got {other:?}"),
        }
    }

    #[test]
    fn decode_rejects_unknown_era_tag() {
        let s = empty_conway();
        let mut bytes = encode_ledger_state(&s).expect("encode");
        // era=7 → patch to 9 (unknown). 9 ≤ 23 so still 1-byte inline.
        assert_eq!(bytes[1], 0x07);
        bytes[1] = 0x09;
        match decode_ledger_state(&bytes) {
            Err(SnapshotDecodeError::Structural {
                reason: StructuralReason::EraTagOutOfRange,
            }) => {}
            other => panic!("expected EraTagOutOfRange, got {other:?}"),
        }
    }
}
