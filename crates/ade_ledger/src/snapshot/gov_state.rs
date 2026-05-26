// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! BLUE encoders/decoders for `ProtocolParameters`,
//! `ConwayOnlyDepositParams`, and `ConwayGovState` (PHASE4-N-J S5).
//!
//! Compact encoding: each top-level structure is an array(N) with
//! per-field ordering matching `ade_ledger::fingerprint`. Options
//! encode as `null | bytes/array`. Rationals encode as
//! `array(2)[num(int), den(int)]`. The GovAction enum carries a
//! 7-variant tag with per-variant payload.

use std::collections::{BTreeMap, BTreeSet};

use ade_codec::cbor::{
    canonical_width, read_any_int, read_array_header, read_bytes, read_map_header,
    write_array_header, write_bytes_canonical, write_map_header, write_null, write_uint_canonical,
    ContainerEncoding, IntWidth, MAJOR_NEGATIVE,
};
use ade_codec::CodecError;
use ade_types::conway::cert::DRep;
use ade_types::conway::governance::{GovAction, GovActionId, GovActionState, Vote};
use ade_types::shelley::cert::StakeCredential;
use ade_types::tx::Coin;
use ade_types::{EpochNo, Hash28, Hash32};

use crate::pparams::{ConwayOnlyDepositParams, ProtocolParameters};
use crate::rational::Rational;
use crate::state::ConwayGovState;

use super::error::{SnapshotDecodeError, StructuralReason};

// ---------------------------------------------------------------------------
// ProtocolParameters (24 fields)
// ---------------------------------------------------------------------------

const PPARAMS_FIELDS: u64 = 24;

pub fn encode_pparams(pp: &ProtocolParameters) -> Vec<u8> {
    let mut buf = Vec::new();
    write_array_header(
        &mut buf,
        ContainerEncoding::Definite(PPARAMS_FIELDS, canonical_width(PPARAMS_FIELDS)),
    );
    write_uint_canonical(&mut buf, pp.min_fee_a.0);
    write_uint_canonical(&mut buf, pp.min_fee_b.0);
    write_uint_canonical(&mut buf, pp.max_block_body_size as u64);
    write_uint_canonical(&mut buf, pp.max_tx_size as u64);
    write_uint_canonical(&mut buf, pp.max_block_header_size as u64);
    write_uint_canonical(&mut buf, pp.key_deposit.0);
    write_uint_canonical(&mut buf, pp.pool_deposit.0);
    write_uint_canonical(&mut buf, pp.e_max as u64);
    write_uint_canonical(&mut buf, pp.n_opt as u64);
    write_rational(&mut buf, &pp.pool_influence);
    write_rational(&mut buf, &pp.monetary_expansion);
    write_rational(&mut buf, &pp.treasury_growth);
    write_uint_canonical(&mut buf, pp.protocol_major as u64);
    write_uint_canonical(&mut buf, pp.protocol_minor as u64);
    write_uint_canonical(&mut buf, pp.min_utxo_value.0);
    write_uint_canonical(&mut buf, pp.min_pool_cost.0);
    write_rational(&mut buf, &pp.decentralization);
    write_uint_canonical(&mut buf, pp.collateral_percent as u64);
    write_uint_canonical(&mut buf, pp.max_tx_ex_units_mem);
    write_uint_canonical(&mut buf, pp.max_tx_ex_units_cpu);
    write_uint_canonical(&mut buf, pp.network_id as u64);
    write_opt_bytes(&mut buf, pp.cost_models_cbor.as_deref());
    // Final 2 reserved fields: placeholders for forward-compatible extension.
    write_uint_canonical(&mut buf, 0);
    write_uint_canonical(&mut buf, 0);
    buf
}

pub fn decode_pparams(bytes: &[u8]) -> Result<ProtocolParameters, SnapshotDecodeError> {
    let mut o = 0usize;
    expect_array(bytes, &mut o, PPARAMS_FIELDS)?;
    let min_fee_a = Coin(read_u64(bytes, &mut o)?);
    let min_fee_b = Coin(read_u64(bytes, &mut o)?);
    let max_block_body_size = read_u64(bytes, &mut o)? as u32;
    let max_tx_size = read_u64(bytes, &mut o)? as u32;
    let max_block_header_size = read_u64(bytes, &mut o)? as u32;
    let key_deposit = Coin(read_u64(bytes, &mut o)?);
    let pool_deposit = Coin(read_u64(bytes, &mut o)?);
    let e_max = read_u64(bytes, &mut o)? as u32;
    let n_opt = read_u64(bytes, &mut o)? as u32;
    let pool_influence = read_rational(bytes, &mut o)?;
    let monetary_expansion = read_rational(bytes, &mut o)?;
    let treasury_growth = read_rational(bytes, &mut o)?;
    let protocol_major = read_u64(bytes, &mut o)? as u32;
    let protocol_minor = read_u64(bytes, &mut o)? as u32;
    let min_utxo_value = Coin(read_u64(bytes, &mut o)?);
    let min_pool_cost = Coin(read_u64(bytes, &mut o)?);
    let decentralization = read_rational(bytes, &mut o)?;
    let collateral_percent = read_u64(bytes, &mut o)? as u16;
    let max_tx_ex_units_mem = read_u64(bytes, &mut o)?;
    let max_tx_ex_units_cpu = read_u64(bytes, &mut o)?;
    let network_id = read_u64(bytes, &mut o)? as u8;
    let cost_models_cbor = read_opt_bytes(bytes, &mut o)?;
    // Reserved fields.
    let _ = read_u64(bytes, &mut o)?;
    let _ = read_u64(bytes, &mut o)?;
    Ok(ProtocolParameters {
        min_fee_a,
        min_fee_b,
        max_block_body_size,
        max_tx_size,
        max_block_header_size,
        key_deposit,
        pool_deposit,
        e_max,
        n_opt,
        pool_influence,
        monetary_expansion,
        treasury_growth,
        protocol_major,
        protocol_minor,
        min_utxo_value,
        min_pool_cost,
        decentralization,
        collateral_percent,
        max_tx_ex_units_mem,
        max_tx_ex_units_cpu,
        network_id,
        cost_models_cbor,
    })
}

// ---------------------------------------------------------------------------
// ConwayOnlyDepositParams
// ---------------------------------------------------------------------------

pub fn encode_conway_deposit_params(p: &ConwayOnlyDepositParams) -> Vec<u8> {
    let mut buf = Vec::new();
    write_array_header(&mut buf, ContainerEncoding::Definite(3, IntWidth::Inline));
    write_uint_canonical(&mut buf, p.drep_deposit.0);
    write_uint_canonical(&mut buf, p.gov_action_deposit.0);
    write_uint_canonical(&mut buf, p.drep_activity);
    buf
}

pub fn decode_conway_deposit_params(
    bytes: &[u8],
) -> Result<ConwayOnlyDepositParams, SnapshotDecodeError> {
    let mut o = 0usize;
    expect_array(bytes, &mut o, 3)?;
    let drep_deposit = Coin(read_u64(bytes, &mut o)?);
    let gov_action_deposit = Coin(read_u64(bytes, &mut o)?);
    let drep_activity = read_u64(bytes, &mut o)?;
    Ok(ConwayOnlyDepositParams {
        drep_deposit,
        gov_action_deposit,
        drep_activity,
    })
}

// ---------------------------------------------------------------------------
// ConwayGovState
// ---------------------------------------------------------------------------

const GOV_FIELDS: u64 = 9;

pub fn encode_gov_state(g: &ConwayGovState) -> Vec<u8> {
    let mut buf = Vec::new();
    write_array_header(
        &mut buf,
        ContainerEncoding::Definite(GOV_FIELDS, canonical_width(GOV_FIELDS)),
    );
    // proposals (Vec)
    write_array_header(
        &mut buf,
        ContainerEncoding::Definite(
            g.proposals.len() as u64,
            canonical_width(g.proposals.len() as u64),
        ),
    );
    for p in &g.proposals {
        write_gov_action_state(&mut buf, p);
    }
    // committee (map)
    write_map_header(
        &mut buf,
        ContainerEncoding::Definite(
            g.committee.len() as u64,
            canonical_width(g.committee.len() as u64),
        ),
    );
    for (cred, expiry) in &g.committee {
        write_stake_credential(&mut buf, cred);
        write_uint_canonical(&mut buf, *expiry);
    }
    // committee_quorum (num, den)
    write_array_header(&mut buf, ContainerEncoding::Definite(2, IntWidth::Inline));
    write_uint_canonical(&mut buf, g.committee_quorum.0);
    write_uint_canonical(&mut buf, g.committee_quorum.1);
    // drep_expiry (map)
    write_map_header(
        &mut buf,
        ContainerEncoding::Definite(
            g.drep_expiry.len() as u64,
            canonical_width(g.drep_expiry.len() as u64),
        ),
    );
    for (cred, expiry) in &g.drep_expiry {
        write_stake_credential(&mut buf, cred);
        write_uint_canonical(&mut buf, *expiry);
    }
    // gov_action_lifetime
    write_uint_canonical(&mut buf, g.gov_action_lifetime);
    // vote_delegations (map)
    write_map_header(
        &mut buf,
        ContainerEncoding::Definite(
            g.vote_delegations.len() as u64,
            canonical_width(g.vote_delegations.len() as u64),
        ),
    );
    for (cred, drep) in &g.vote_delegations {
        write_stake_credential(&mut buf, cred);
        write_drep(&mut buf, drep);
    }
    // pool_voting_thresholds (Vec of (num, den))
    write_array_header(
        &mut buf,
        ContainerEncoding::Definite(
            g.pool_voting_thresholds.len() as u64,
            canonical_width(g.pool_voting_thresholds.len() as u64),
        ),
    );
    for (n, d) in &g.pool_voting_thresholds {
        write_array_header(&mut buf, ContainerEncoding::Definite(2, IntWidth::Inline));
        write_uint_canonical(&mut buf, *n);
        write_uint_canonical(&mut buf, *d);
    }
    // drep_voting_thresholds
    write_array_header(
        &mut buf,
        ContainerEncoding::Definite(
            g.drep_voting_thresholds.len() as u64,
            canonical_width(g.drep_voting_thresholds.len() as u64),
        ),
    );
    for (n, d) in &g.drep_voting_thresholds {
        write_array_header(&mut buf, ContainerEncoding::Definite(2, IntWidth::Inline));
        write_uint_canonical(&mut buf, *n);
        write_uint_canonical(&mut buf, *d);
    }
    // committee_hot_keys (map)
    write_map_header(
        &mut buf,
        ContainerEncoding::Definite(
            g.committee_hot_keys.len() as u64,
            canonical_width(g.committee_hot_keys.len() as u64),
        ),
    );
    for (hot, cold) in &g.committee_hot_keys {
        write_stake_credential(&mut buf, hot);
        write_stake_credential(&mut buf, cold);
    }
    buf
}

pub fn decode_gov_state(bytes: &[u8]) -> Result<ConwayGovState, SnapshotDecodeError> {
    let mut o = 0usize;
    expect_array(bytes, &mut o, GOV_FIELDS)?;
    // proposals
    let n_proposals = read_array_len(bytes, &mut o)?;
    let mut proposals = Vec::with_capacity(n_proposals as usize);
    for _ in 0..n_proposals {
        proposals.push(read_gov_action_state(bytes, &mut o)?);
    }
    // committee
    let n_committee = read_map_len(bytes, &mut o)?;
    let mut committee: BTreeMap<StakeCredential, u64> = BTreeMap::new();
    for _ in 0..n_committee {
        let cred = read_stake_credential(bytes, &mut o)?;
        let expiry = read_u64(bytes, &mut o)?;
        committee.insert(cred, expiry);
    }
    // committee_quorum
    expect_array(bytes, &mut o, 2)?;
    let q_num = read_u64(bytes, &mut o)?;
    let q_den = read_u64(bytes, &mut o)?;
    // drep_expiry
    let n_drep_expiry = read_map_len(bytes, &mut o)?;
    let mut drep_expiry: BTreeMap<StakeCredential, u64> = BTreeMap::new();
    for _ in 0..n_drep_expiry {
        let cred = read_stake_credential(bytes, &mut o)?;
        let expiry = read_u64(bytes, &mut o)?;
        drep_expiry.insert(cred, expiry);
    }
    let gov_action_lifetime = read_u64(bytes, &mut o)?;
    // vote_delegations
    let n_vote_delegations = read_map_len(bytes, &mut o)?;
    let mut vote_delegations: BTreeMap<StakeCredential, DRep> = BTreeMap::new();
    for _ in 0..n_vote_delegations {
        let cred = read_stake_credential(bytes, &mut o)?;
        let drep = read_drep(bytes, &mut o)?;
        vote_delegations.insert(cred, drep);
    }
    // pool_voting_thresholds
    let n_pvt = read_array_len(bytes, &mut o)?;
    let mut pool_voting_thresholds = Vec::with_capacity(n_pvt as usize);
    for _ in 0..n_pvt {
        expect_array(bytes, &mut o, 2)?;
        let n = read_u64(bytes, &mut o)?;
        let d = read_u64(bytes, &mut o)?;
        pool_voting_thresholds.push((n, d));
    }
    // drep_voting_thresholds
    let n_dvt = read_array_len(bytes, &mut o)?;
    let mut drep_voting_thresholds = Vec::with_capacity(n_dvt as usize);
    for _ in 0..n_dvt {
        expect_array(bytes, &mut o, 2)?;
        let n = read_u64(bytes, &mut o)?;
        let d = read_u64(bytes, &mut o)?;
        drep_voting_thresholds.push((n, d));
    }
    // committee_hot_keys
    let n_hot = read_map_len(bytes, &mut o)?;
    let mut committee_hot_keys: BTreeMap<StakeCredential, StakeCredential> = BTreeMap::new();
    for _ in 0..n_hot {
        let hot = read_stake_credential(bytes, &mut o)?;
        let cold = read_stake_credential(bytes, &mut o)?;
        committee_hot_keys.insert(hot, cold);
    }
    Ok(ConwayGovState {
        proposals,
        committee,
        committee_quorum: (q_num, q_den),
        drep_expiry,
        gov_action_lifetime,
        vote_delegations,
        pool_voting_thresholds,
        drep_voting_thresholds,
        committee_hot_keys,
    })
}

// ---------------------------------------------------------------------------
// GovAction + GovActionState
// ---------------------------------------------------------------------------

fn write_gov_action_state(buf: &mut Vec<u8>, s: &GovActionState) {
    write_array_header(buf, ContainerEncoding::Definite(9, IntWidth::Inline));
    write_gov_action_id(buf, &s.action_id);
    // committee_votes Vec<(cred, vote)>
    write_array_header(
        buf,
        ContainerEncoding::Definite(
            s.committee_votes.len() as u64,
            canonical_width(s.committee_votes.len() as u64),
        ),
    );
    for (cred, vote) in &s.committee_votes {
        write_array_header(buf, ContainerEncoding::Definite(2, IntWidth::Inline));
        write_stake_credential(buf, cred);
        write_vote(buf, *vote);
    }
    // drep_votes
    write_array_header(
        buf,
        ContainerEncoding::Definite(
            s.drep_votes.len() as u64,
            canonical_width(s.drep_votes.len() as u64),
        ),
    );
    for (cred, vote) in &s.drep_votes {
        write_array_header(buf, ContainerEncoding::Definite(2, IntWidth::Inline));
        write_stake_credential(buf, cred);
        write_vote(buf, *vote);
    }
    // spo_votes
    write_array_header(
        buf,
        ContainerEncoding::Definite(
            s.spo_votes.len() as u64,
            canonical_width(s.spo_votes.len() as u64),
        ),
    );
    for (hash, vote) in &s.spo_votes {
        write_array_header(buf, ContainerEncoding::Definite(2, IntWidth::Inline));
        write_bytes_canonical(buf, &hash.0);
        write_vote(buf, *vote);
    }
    write_uint_canonical(buf, s.deposit.0);
    write_bytes_canonical(buf, &s.return_addr);
    write_gov_action(buf, &s.gov_action);
    write_uint_canonical(buf, s.proposed_in.0);
    write_uint_canonical(buf, s.expires_after.0);
}

fn read_gov_action_state(
    bytes: &[u8],
    o: &mut usize,
) -> Result<GovActionState, SnapshotDecodeError> {
    expect_array(bytes, o, 9)?;
    let action_id = read_gov_action_id(bytes, o)?;
    let n_cv = read_array_len(bytes, o)?;
    let mut committee_votes = Vec::with_capacity(n_cv as usize);
    for _ in 0..n_cv {
        expect_array(bytes, o, 2)?;
        let cred = read_stake_credential(bytes, o)?;
        let vote = read_vote(bytes, o)?;
        committee_votes.push((cred, vote));
    }
    let n_dv = read_array_len(bytes, o)?;
    let mut drep_votes = Vec::with_capacity(n_dv as usize);
    for _ in 0..n_dv {
        expect_array(bytes, o, 2)?;
        let cred = read_stake_credential(bytes, o)?;
        let vote = read_vote(bytes, o)?;
        drep_votes.push((cred, vote));
    }
    let n_sv = read_array_len(bytes, o)?;
    let mut spo_votes = Vec::with_capacity(n_sv as usize);
    for _ in 0..n_sv {
        expect_array(bytes, o, 2)?;
        let (h, _) = read_bytes(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
        if h.len() != 28 {
            return Err(SnapshotDecodeError::Structural {
                reason: StructuralReason::Hash28LengthMismatch,
            });
        }
        let mut arr = [0u8; 28];
        arr.copy_from_slice(&h);
        let vote = read_vote(bytes, o)?;
        spo_votes.push((Hash28(arr), vote));
    }
    let deposit = Coin(read_u64(bytes, o)?);
    let (return_addr, _) = read_bytes(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
    let gov_action = read_gov_action(bytes, o)?;
    let proposed_in = EpochNo(read_u64(bytes, o)?);
    let expires_after = EpochNo(read_u64(bytes, o)?);
    Ok(GovActionState {
        action_id,
        committee_votes,
        drep_votes,
        spo_votes,
        deposit,
        return_addr,
        gov_action,
        proposed_in,
        expires_after,
    })
}

fn write_gov_action(buf: &mut Vec<u8>, a: &GovAction) {
    write_array_header(buf, ContainerEncoding::Definite(2, IntWidth::Inline));
    match a {
        GovAction::ParameterChange {
            prev_action,
            update,
            policy_hash,
        } => {
            write_uint_canonical(buf, 0);
            write_array_header(buf, ContainerEncoding::Definite(3, IntWidth::Inline));
            write_opt_gov_action_id(buf, prev_action.as_ref());
            write_bytes_canonical(buf, update);
            write_opt_hash28(buf, policy_hash.as_ref());
        }
        GovAction::HardForkInitiation {
            prev_action,
            protocol_version,
        } => {
            write_uint_canonical(buf, 1);
            write_array_header(buf, ContainerEncoding::Definite(2, IntWidth::Inline));
            write_opt_gov_action_id(buf, prev_action.as_ref());
            write_array_header(buf, ContainerEncoding::Definite(2, IntWidth::Inline));
            write_uint_canonical(buf, protocol_version.0);
            write_uint_canonical(buf, protocol_version.1);
        }
        GovAction::TreasuryWithdrawals {
            withdrawals,
            policy_hash,
        } => {
            write_uint_canonical(buf, 2);
            write_array_header(buf, ContainerEncoding::Definite(2, IntWidth::Inline));
            write_array_header(
                buf,
                ContainerEncoding::Definite(
                    withdrawals.len() as u64,
                    canonical_width(withdrawals.len() as u64),
                ),
            );
            for (addr, coin) in withdrawals {
                write_array_header(buf, ContainerEncoding::Definite(2, IntWidth::Inline));
                write_bytes_canonical(buf, addr);
                write_uint_canonical(buf, coin.0);
            }
            write_opt_hash28(buf, policy_hash.as_ref());
        }
        GovAction::NoConfidence { prev_action } => {
            write_uint_canonical(buf, 3);
            write_array_header(buf, ContainerEncoding::Definite(1, IntWidth::Inline));
            write_opt_gov_action_id(buf, prev_action.as_ref());
        }
        GovAction::UpdateCommittee {
            prev_action,
            removed,
            added,
            threshold,
        } => {
            write_uint_canonical(buf, 4);
            write_array_header(buf, ContainerEncoding::Definite(4, IntWidth::Inline));
            write_opt_gov_action_id(buf, prev_action.as_ref());
            write_array_header(
                buf,
                ContainerEncoding::Definite(
                    removed.len() as u64,
                    canonical_width(removed.len() as u64),
                ),
            );
            for cred in removed {
                write_stake_credential(buf, cred);
            }
            write_map_header(
                buf,
                ContainerEncoding::Definite(
                    added.len() as u64,
                    canonical_width(added.len() as u64),
                ),
            );
            for (cred, epoch) in added {
                write_stake_credential(buf, cred);
                write_uint_canonical(buf, *epoch);
            }
            write_array_header(buf, ContainerEncoding::Definite(2, IntWidth::Inline));
            write_uint_canonical(buf, threshold.0);
            write_uint_canonical(buf, threshold.1);
        }
        GovAction::NewConstitution { prev_action, raw } => {
            write_uint_canonical(buf, 5);
            write_array_header(buf, ContainerEncoding::Definite(2, IntWidth::Inline));
            write_opt_gov_action_id(buf, prev_action.as_ref());
            write_bytes_canonical(buf, raw);
        }
        GovAction::InfoAction => {
            write_uint_canonical(buf, 6);
            write_array_header(buf, ContainerEncoding::Definite(0, IntWidth::Inline));
        }
    }
}

fn read_gov_action(bytes: &[u8], o: &mut usize) -> Result<GovAction, SnapshotDecodeError> {
    expect_array(bytes, o, 2)?;
    let tag = read_u64(bytes, o)?;
    match tag {
        0 => {
            expect_array(bytes, o, 3)?;
            let prev_action = read_opt_gov_action_id(bytes, o)?;
            let (update, _) = read_bytes(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
            let policy_hash = read_opt_hash28(bytes, o)?;
            Ok(GovAction::ParameterChange {
                prev_action,
                update,
                policy_hash,
            })
        }
        1 => {
            expect_array(bytes, o, 2)?;
            let prev_action = read_opt_gov_action_id(bytes, o)?;
            expect_array(bytes, o, 2)?;
            let major = read_u64(bytes, o)?;
            let minor = read_u64(bytes, o)?;
            Ok(GovAction::HardForkInitiation {
                prev_action,
                protocol_version: (major, minor),
            })
        }
        2 => {
            expect_array(bytes, o, 2)?;
            let n_w = read_array_len(bytes, o)?;
            let mut withdrawals = Vec::with_capacity(n_w as usize);
            for _ in 0..n_w {
                expect_array(bytes, o, 2)?;
                let (addr, _) = read_bytes(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
                let coin = Coin(read_u64(bytes, o)?);
                withdrawals.push((addr, coin));
            }
            let policy_hash = read_opt_hash28(bytes, o)?;
            Ok(GovAction::TreasuryWithdrawals {
                withdrawals,
                policy_hash,
            })
        }
        3 => {
            expect_array(bytes, o, 1)?;
            let prev_action = read_opt_gov_action_id(bytes, o)?;
            Ok(GovAction::NoConfidence { prev_action })
        }
        4 => {
            expect_array(bytes, o, 4)?;
            let prev_action = read_opt_gov_action_id(bytes, o)?;
            let n_r = read_array_len(bytes, o)?;
            let mut removed: BTreeSet<StakeCredential> = BTreeSet::new();
            for _ in 0..n_r {
                removed.insert(read_stake_credential(bytes, o)?);
            }
            let n_a = read_map_len(bytes, o)?;
            let mut added: BTreeMap<StakeCredential, u64> = BTreeMap::new();
            for _ in 0..n_a {
                let cred = read_stake_credential(bytes, o)?;
                let epoch = read_u64(bytes, o)?;
                added.insert(cred, epoch);
            }
            expect_array(bytes, o, 2)?;
            let n = read_u64(bytes, o)?;
            let d = read_u64(bytes, o)?;
            Ok(GovAction::UpdateCommittee {
                prev_action,
                removed,
                added,
                threshold: (n, d),
            })
        }
        5 => {
            expect_array(bytes, o, 2)?;
            let prev_action = read_opt_gov_action_id(bytes, o)?;
            let (raw, _) = read_bytes(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
            Ok(GovAction::NewConstitution { prev_action, raw })
        }
        6 => {
            expect_array(bytes, o, 0)?;
            Ok(GovAction::InfoAction)
        }
        _ => Err(SnapshotDecodeError::Structural {
            reason: StructuralReason::EraTagOutOfRange,
        }),
    }
}

// ---------------------------------------------------------------------------
// Field helpers
// ---------------------------------------------------------------------------

fn write_rational(buf: &mut Vec<u8>, r: &Rational) {
    write_array_header(buf, ContainerEncoding::Definite(2, IntWidth::Inline));
    write_int_i128(buf, r.numerator());
    write_int_i128(buf, r.denominator());
}

fn read_rational(bytes: &[u8], o: &mut usize) -> Result<Rational, SnapshotDecodeError> {
    expect_array(bytes, o, 2)?;
    let n = read_int_i128(bytes, o)?;
    let d = read_int_i128(bytes, o)?;
    Rational::new(n, d).ok_or(SnapshotDecodeError::Structural {
        reason: StructuralReason::ArrayLengthMismatch,
    })
}

fn write_int_i128(buf: &mut Vec<u8>, v: i128) {
    if v >= 0 {
        write_uint_canonical(buf, v as u64);
    } else {
        let positive: u64 = ((-1i128) - v) as u64;
        let width = ade_codec::cbor::canonical_width(positive);
        ade_codec::cbor::write_argument(buf, MAJOR_NEGATIVE, positive, width);
    }
}

fn read_int_i128(bytes: &[u8], o: &mut usize) -> Result<i128, SnapshotDecodeError> {
    let (v, is_neg, _w) = read_any_int(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
    if is_neg {
        Ok((-1i128) - (v as i128))
    } else {
        Ok(v as i128)
    }
}

fn write_opt_bytes(buf: &mut Vec<u8>, b: Option<&[u8]>) {
    match b {
        Some(x) => write_bytes_canonical(buf, x),
        None => write_null(buf),
    }
}

fn read_opt_bytes(bytes: &[u8], o: &mut usize) -> Result<Option<Vec<u8>>, SnapshotDecodeError> {
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
    Ok(Some(b))
}

fn write_opt_hash28(buf: &mut Vec<u8>, h: Option<&Hash28>) {
    match h {
        Some(x) => write_bytes_canonical(buf, &x.0),
        None => write_null(buf),
    }
}

fn read_opt_hash28(bytes: &[u8], o: &mut usize) -> Result<Option<Hash28>, SnapshotDecodeError> {
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
    if b.len() != 28 {
        return Err(SnapshotDecodeError::Structural {
            reason: StructuralReason::Hash28LengthMismatch,
        });
    }
    let mut arr = [0u8; 28];
    arr.copy_from_slice(&b);
    Ok(Some(Hash28(arr)))
}

fn write_gov_action_id(buf: &mut Vec<u8>, id: &GovActionId) {
    write_array_header(buf, ContainerEncoding::Definite(2, IntWidth::Inline));
    write_bytes_canonical(buf, &id.tx_hash.0);
    write_uint_canonical(buf, id.index as u64);
}

fn read_gov_action_id(bytes: &[u8], o: &mut usize) -> Result<GovActionId, SnapshotDecodeError> {
    expect_array(bytes, o, 2)?;
    let (h, _) = read_bytes(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
    if h.len() != 32 {
        return Err(SnapshotDecodeError::Structural {
            reason: StructuralReason::Hash32LengthMismatch,
        });
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&h);
    let idx = read_u64(bytes, o)?;
    Ok(GovActionId {
        tx_hash: Hash32(arr),
        index: idx as u32,
    })
}

fn write_opt_gov_action_id(buf: &mut Vec<u8>, id: Option<&GovActionId>) {
    match id {
        Some(x) => write_gov_action_id(buf, x),
        None => write_null(buf),
    }
}

fn read_opt_gov_action_id(
    bytes: &[u8],
    o: &mut usize,
) -> Result<Option<GovActionId>, SnapshotDecodeError> {
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
    Ok(Some(read_gov_action_id(bytes, o)?))
}

fn write_stake_credential(buf: &mut Vec<u8>, c: &StakeCredential) {
    write_array_header(buf, ContainerEncoding::Definite(2, IntWidth::Inline));
    let (variant, hash): (u64, &Hash28) = match c {
        StakeCredential::KeyHash(h) => (0, h),
        StakeCredential::ScriptHash(h) => (1, h),
    };
    write_uint_canonical(buf, variant);
    write_bytes_canonical(buf, &hash.0);
}

fn read_stake_credential(
    bytes: &[u8],
    o: &mut usize,
) -> Result<StakeCredential, SnapshotDecodeError> {
    expect_array(bytes, o, 2)?;
    let variant = read_u64(bytes, o)?;
    let (h, _) = read_bytes(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
    if h.len() != 28 {
        return Err(SnapshotDecodeError::Structural {
            reason: StructuralReason::Hash28LengthMismatch,
        });
    }
    let mut arr = [0u8; 28];
    arr.copy_from_slice(&h);
    match variant {
        0 => Ok(StakeCredential::KeyHash(Hash28(arr))),
        1 => Ok(StakeCredential::ScriptHash(Hash28(arr))),
        _ => Err(SnapshotDecodeError::Structural {
            reason: StructuralReason::EraTagOutOfRange,
        }),
    }
}

fn write_drep(buf: &mut Vec<u8>, d: &DRep) {
    write_array_header(buf, ContainerEncoding::Definite(2, IntWidth::Inline));
    match d {
        DRep::KeyHash(h) => {
            write_uint_canonical(buf, 0);
            write_bytes_canonical(buf, &h.0);
        }
        DRep::ScriptHash(h) => {
            write_uint_canonical(buf, 1);
            write_bytes_canonical(buf, &h.0);
        }
        DRep::AlwaysAbstain => {
            write_uint_canonical(buf, 2);
            write_null(buf);
        }
        DRep::AlwaysNoConfidence => {
            write_uint_canonical(buf, 3);
            write_null(buf);
        }
    }
}

fn read_drep(bytes: &[u8], o: &mut usize) -> Result<DRep, SnapshotDecodeError> {
    expect_array(bytes, o, 2)?;
    let variant = read_u64(bytes, o)?;
    match variant {
        0 | 1 => {
            let (h, _) = read_bytes(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
            if h.len() != 28 {
                return Err(SnapshotDecodeError::Structural {
                    reason: StructuralReason::Hash28LengthMismatch,
                });
            }
            let mut arr = [0u8; 28];
            arr.copy_from_slice(&h);
            if variant == 0 {
                Ok(DRep::KeyHash(Hash28(arr)))
            } else {
                Ok(DRep::ScriptHash(Hash28(arr)))
            }
        }
        2 | 3 => {
            // null placeholder
            if *o >= bytes.len() || bytes[*o] != 0xF6 {
                return Err(SnapshotDecodeError::Structural {
                    reason: StructuralReason::UnexpectedNonNull,
                });
            }
            *o += 1;
            if variant == 2 {
                Ok(DRep::AlwaysAbstain)
            } else {
                Ok(DRep::AlwaysNoConfidence)
            }
        }
        _ => Err(SnapshotDecodeError::Structural {
            reason: StructuralReason::EraTagOutOfRange,
        }),
    }
}

fn write_vote(buf: &mut Vec<u8>, v: Vote) {
    let tag: u64 = match v {
        Vote::No => 0,
        Vote::Yes => 1,
        Vote::Abstain => 2,
    };
    write_uint_canonical(buf, tag);
}

fn read_vote(bytes: &[u8], o: &mut usize) -> Result<Vote, SnapshotDecodeError> {
    let v = read_u64(bytes, o)?;
    match v {
        0 => Ok(Vote::No),
        1 => Ok(Vote::Yes),
        2 => Ok(Vote::Abstain),
        _ => Err(SnapshotDecodeError::Structural {
            reason: StructuralReason::EraTagOutOfRange,
        }),
    }
}

fn read_u64(bytes: &[u8], o: &mut usize) -> Result<u64, SnapshotDecodeError> {
    let (v, _is_neg, _w) = read_any_int(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
    Ok(v)
}

fn read_array_len(bytes: &[u8], o: &mut usize) -> Result<u64, SnapshotDecodeError> {
    match read_array_header(bytes, o).map_err(SnapshotDecodeError::Cbor)? {
        ContainerEncoding::Definite(n, _) => Ok(n),
        _ => Err(SnapshotDecodeError::Structural {
            reason: StructuralReason::ArrayLengthMismatch,
        }),
    }
}

fn read_map_len(bytes: &[u8], o: &mut usize) -> Result<u64, SnapshotDecodeError> {
    match read_map_header(bytes, o).map_err(SnapshotDecodeError::Cbor)? {
        ContainerEncoding::Definite(n, _) => Ok(n),
        _ => Err(SnapshotDecodeError::Structural {
            reason: StructuralReason::ArrayLengthMismatch,
        }),
    }
}

fn expect_array(bytes: &[u8], o: &mut usize, expected_len: u64) -> Result<(), SnapshotDecodeError> {
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

    #[test]
    fn pparams_round_trip_default() {
        let pp = ProtocolParameters::default();
        let bytes = encode_pparams(&pp);
        let decoded = decode_pparams(&bytes).expect("decode");
        assert_eq!(decoded, pp);
    }

    #[test]
    fn pparams_round_trip_with_cost_models() {
        let mut pp = ProtocolParameters::default();
        pp.cost_models_cbor = Some(vec![0xAB, 0xCD, 0xEF]);
        let bytes = encode_pparams(&pp);
        let decoded = decode_pparams(&bytes).expect("decode");
        assert_eq!(decoded, pp);
    }

    #[test]
    fn pparams_encode_deterministic_across_runs() {
        let pp = ProtocolParameters::default();
        let a = encode_pparams(&pp);
        let b = encode_pparams(&pp);
        assert_eq!(a, b);
    }

    #[test]
    fn conway_deposit_params_round_trip() {
        let p = ConwayOnlyDepositParams {
            drep_deposit: Coin(500_000_000),
            gov_action_deposit: Coin(100_000_000_000),
            drep_activity: 100,
        };
        let bytes = encode_conway_deposit_params(&p);
        let decoded = decode_conway_deposit_params(&bytes).expect("decode");
        assert_eq!(decoded, p);
    }

    fn make_gov_state() -> ConwayGovState {
        let mut g = ConwayGovState {
            proposals: Vec::new(),
            committee: BTreeMap::new(),
            committee_quorum: (2, 3),
            drep_expiry: BTreeMap::new(),
            gov_action_lifetime: 6,
            vote_delegations: BTreeMap::new(),
            pool_voting_thresholds: vec![(1, 2), (51, 100)],
            drep_voting_thresholds: vec![(67, 100)],
            committee_hot_keys: BTreeMap::new(),
        };
        g.committee
            .insert(StakeCredential::KeyHash(Hash28([0xAA; 28])), 600);
        g.drep_expiry
            .insert(StakeCredential::KeyHash(Hash28([0xBB; 28])), 700);
        g.vote_delegations.insert(
            StakeCredential::KeyHash(Hash28([0xCC; 28])),
            DRep::AlwaysAbstain,
        );
        g.vote_delegations.insert(
            StakeCredential::ScriptHash(Hash28([0xDD; 28])),
            DRep::KeyHash(Hash28([0xEE; 28])),
        );
        g.committee_hot_keys.insert(
            StakeCredential::KeyHash(Hash28([0xF0; 28])),
            StakeCredential::ScriptHash(Hash28([0xF1; 28])),
        );
        // Add one proposal of each GovAction variant.
        for (i, action) in [
            GovAction::InfoAction,
            GovAction::ParameterChange {
                prev_action: None,
                update: vec![0x01, 0x02],
                policy_hash: Some(Hash28([0x11; 28])),
            },
            GovAction::HardForkInitiation {
                prev_action: Some(GovActionId {
                    tx_hash: Hash32([0x22; 32]),
                    index: 1,
                }),
                protocol_version: (10, 0),
            },
            GovAction::TreasuryWithdrawals {
                withdrawals: vec![
                    (vec![0x33; 29], Coin(1_000_000)),
                    (vec![0x34; 29], Coin(2_000_000)),
                ],
                policy_hash: None,
            },
            GovAction::NoConfidence { prev_action: None },
            GovAction::UpdateCommittee {
                prev_action: None,
                removed: {
                    let mut s = BTreeSet::new();
                    s.insert(StakeCredential::KeyHash(Hash28([0x44; 28])));
                    s
                },
                added: {
                    let mut m = BTreeMap::new();
                    m.insert(StakeCredential::ScriptHash(Hash28([0x45; 28])), 800);
                    m
                },
                threshold: (3, 5),
            },
            GovAction::NewConstitution {
                prev_action: None,
                raw: vec![0x55, 0x66],
            },
        ]
        .into_iter()
        .enumerate()
        {
            g.proposals.push(GovActionState {
                action_id: GovActionId {
                    tx_hash: Hash32([i as u8; 32]),
                    index: i as u32,
                },
                committee_votes: vec![(
                    StakeCredential::KeyHash(Hash28([0x80 + i as u8; 28])),
                    Vote::Yes,
                )],
                drep_votes: vec![(
                    StakeCredential::ScriptHash(Hash28([0x90 + i as u8; 28])),
                    Vote::No,
                )],
                spo_votes: vec![(Hash28([0xA0 + i as u8; 28]), Vote::Abstain)],
                deposit: Coin(100_000_000_000),
                return_addr: vec![0xB0 + i as u8; 29],
                gov_action: action,
                proposed_in: EpochNo(576 + i as u64),
                expires_after: EpochNo(582 + i as u64),
            });
        }
        g
    }

    #[test]
    fn gov_state_round_trip_empty() {
        let g = ConwayGovState {
            proposals: Vec::new(),
            committee: BTreeMap::new(),
            committee_quorum: (0, 0),
            drep_expiry: BTreeMap::new(),
            gov_action_lifetime: 0,
            vote_delegations: BTreeMap::new(),
            pool_voting_thresholds: Vec::new(),
            drep_voting_thresholds: Vec::new(),
            committee_hot_keys: BTreeMap::new(),
        };
        let bytes = encode_gov_state(&g);
        let decoded = decode_gov_state(&bytes).expect("decode");
        assert_eq!(decoded, g);
    }

    #[test]
    fn gov_state_round_trip_all_gov_action_variants() {
        let g = make_gov_state();
        let bytes = encode_gov_state(&g);
        let decoded = decode_gov_state(&bytes).expect("decode");
        assert_eq!(decoded, g);
    }

    #[test]
    fn gov_state_encode_deterministic_across_runs() {
        let g = make_gov_state();
        let a = encode_gov_state(&g);
        let b = encode_gov_state(&g);
        assert_eq!(a, b);
    }
}
