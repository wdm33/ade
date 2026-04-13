// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Canonical per-component fingerprint of the ledger state.
//!
//! Produces a deterministic `Blake2b-256` hash per major sub-state
//! (era, UTxO, certificates, epoch, snapshots, protocol parameters,
//! governance) plus a combined rollup. Two states with the same
//! semantic content always produce the same fingerprint; any change
//! to a tracked field flips exactly one component hash plus the
//! combined rollup.
//!
//! This is Ade's own canonical format — it is NOT a Haskell-compatible
//! encoding of cardano-node's `ExtLedgerState`. The encoding is chosen
//! for determinism, compactness, and straightforward divergence
//! localization, not byte-parity with any external implementation.
//!
//! Intended consumers:
//! - `CE-74` determinism CI (hash `combined` before/after replay)
//! - External differential harnesses (consume component hashes to
//!   localize divergence without parsing full state)
//! - Golden regression tests

use ade_codec::cbor::{
    write_argument, write_array_header, write_bytes_canonical, write_map_header, write_null,
    write_uint_canonical, ContainerEncoding, IntWidth, MAJOR_NEGATIVE,
};
use ade_crypto::blake2b::blake2b_256;
use ade_types::conway::cert::DRep;
use ade_types::conway::governance::{GovAction, GovActionId, GovActionState, Vote};
use ade_types::shelley::cert::StakeCredential;
use ade_types::tx::{Coin, PoolId, TxIn};
use ade_types::{CardanoEra, Hash28, Hash32};

use crate::delegation::{CertState, PoolParams as CertPoolParams};
use crate::epoch::{SnapshotState, StakeSnapshot};
use crate::pparams::ProtocolParameters;
use crate::rational::Rational;
use crate::state::{ConwayGovState, EpochState, LedgerState};
use crate::utxo::{TxOut, UTxOState};
use crate::value::{MultiAsset, Value};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Version of the fingerprint schema. Bump on any encoding change.
const FINGERPRINT_VERSION: u64 = 1;

/// Per-component fingerprint of a ledger state.
///
/// Each component hash is `Blake2b-256` over a canonical CBOR encoding of
/// one sub-state. `combined` is `Blake2b-256` over the concatenation of
/// the seven component hashes in declared order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerFingerprint {
    pub era: Hash32,
    pub utxo: Hash32,
    pub cert: Hash32,
    pub epoch: Hash32,
    pub snapshots: Hash32,
    pub pparams: Hash32,
    pub governance: Hash32,
    pub combined: Hash32,
}

impl LedgerFingerprint {
    /// Hex representation of the combined rollup hash (64 characters).
    pub fn combined_hex(&self) -> String {
        format!("{}", self.combined)
    }
}

/// Compute the per-component fingerprint of a ledger state.
///
/// Pure function: same input always produces the same output.
/// The `track_utxo` flag is deliberately excluded — it is a harness
/// control, not a property of the ledger itself.
pub fn fingerprint(state: &LedgerState) -> LedgerFingerprint {
    let era = fingerprint_era(state.era, state.max_lovelace_supply);
    let utxo = fingerprint_utxo(&state.utxo_state);
    let cert = fingerprint_cert(&state.cert_state);
    let epoch = fingerprint_epoch(&state.epoch_state);
    let snapshots = fingerprint_snapshots(&state.epoch_state.snapshots);
    let pparams = fingerprint_pparams(&state.protocol_params);
    let governance = fingerprint_governance(state.gov_state.as_ref());
    let combined = rollup(&[
        &era,
        &utxo,
        &cert,
        &epoch,
        &snapshots,
        &pparams,
        &governance,
    ]);
    LedgerFingerprint {
        era,
        utxo,
        cert,
        epoch,
        snapshots,
        pparams,
        governance,
        combined,
    }
}

// ---------------------------------------------------------------------------
// Component encoders
// ---------------------------------------------------------------------------

fn fingerprint_era(era: CardanoEra, max_lovelace_supply: u64) -> Hash32 {
    let mut buf = Vec::new();
    write_component_header(&mut buf, b"ade/fp/era");
    write_array_canonical(&mut buf, 2);
    write_uint_canonical(&mut buf, era as u64);
    write_uint_canonical(&mut buf, max_lovelace_supply);
    blake2b_256(&buf)
}

fn fingerprint_utxo(utxo: &UTxOState) -> Hash32 {
    let mut buf = Vec::new();
    write_component_header(&mut buf, b"ade/fp/utxo");
    // BTreeMap iteration is sorted by key, giving canonical order.
    write_map_canonical(&mut buf, utxo.utxos.len() as u64);
    for (tx_in, tx_out) in &utxo.utxos {
        write_tx_in(&mut buf, tx_in);
        write_tx_out(&mut buf, tx_out);
    }
    blake2b_256(&buf)
}

fn fingerprint_cert(cert: &CertState) -> Hash32 {
    let mut buf = Vec::new();
    write_component_header(&mut buf, b"ade/fp/cert");
    write_array_canonical(&mut buf, 5);

    // registrations: credential -> deposit
    write_map_canonical(&mut buf, cert.delegation.registrations.len() as u64);
    for (cred, deposit) in &cert.delegation.registrations {
        write_stake_credential(&mut buf, cred);
        write_coin(&mut buf, *deposit);
    }

    // delegations: credential -> pool
    write_map_canonical(&mut buf, cert.delegation.delegations.len() as u64);
    for (cred, pool) in &cert.delegation.delegations {
        write_stake_credential(&mut buf, cred);
        write_pool_id(&mut buf, pool);
    }

    // rewards: credential -> coin
    write_map_canonical(&mut buf, cert.delegation.rewards.len() as u64);
    for (cred, coin) in &cert.delegation.rewards {
        write_stake_credential(&mut buf, cred);
        write_coin(&mut buf, *coin);
    }

    // pools: pool_id -> params
    write_map_canonical(&mut buf, cert.pool.pools.len() as u64);
    for (pool_id, params) in &cert.pool.pools {
        write_pool_id(&mut buf, pool_id);
        write_cert_pool_params(&mut buf, params);
    }

    // retiring: pool_id -> epoch
    write_map_canonical(&mut buf, cert.pool.retiring.len() as u64);
    for (pool_id, epoch) in &cert.pool.retiring {
        write_pool_id(&mut buf, pool_id);
        write_uint_canonical(&mut buf, epoch.0);
    }

    blake2b_256(&buf)
}

fn fingerprint_epoch(epoch: &EpochState) -> Hash32 {
    let mut buf = Vec::new();
    write_component_header(&mut buf, b"ade/fp/epoch");
    // Fields: epoch, slot, reserves, treasury, block_production, epoch_fees.
    // Snapshots are intentionally excluded — they have their own component.
    write_array_canonical(&mut buf, 6);
    write_uint_canonical(&mut buf, epoch.epoch.0);
    write_uint_canonical(&mut buf, epoch.slot.0);
    write_coin(&mut buf, epoch.reserves);
    write_coin(&mut buf, epoch.treasury);
    write_map_canonical(&mut buf, epoch.block_production.len() as u64);
    for (pool_id, count) in &epoch.block_production {
        write_pool_id(&mut buf, pool_id);
        write_uint_canonical(&mut buf, *count);
    }
    write_coin(&mut buf, epoch.epoch_fees);
    blake2b_256(&buf)
}

fn fingerprint_snapshots(snapshots: &SnapshotState) -> Hash32 {
    let mut buf = Vec::new();
    write_component_header(&mut buf, b"ade/fp/snapshots");
    write_array_canonical(&mut buf, 3);
    write_stake_snapshot(&mut buf, &snapshots.mark.0);
    write_stake_snapshot(&mut buf, &snapshots.set.0);
    write_stake_snapshot(&mut buf, &snapshots.go.0);
    blake2b_256(&buf)
}

fn fingerprint_pparams(pp: &ProtocolParameters) -> Hash32 {
    let mut buf = Vec::new();
    write_component_header(&mut buf, b"ade/fp/pparams");
    write_array_canonical(&mut buf, 17);
    write_coin(&mut buf, pp.min_fee_a);
    write_coin(&mut buf, pp.min_fee_b);
    write_uint_canonical(&mut buf, pp.max_block_body_size as u64);
    write_uint_canonical(&mut buf, pp.max_tx_size as u64);
    write_uint_canonical(&mut buf, pp.max_block_header_size as u64);
    write_coin(&mut buf, pp.key_deposit);
    write_coin(&mut buf, pp.pool_deposit);
    write_uint_canonical(&mut buf, pp.e_max as u64);
    write_uint_canonical(&mut buf, pp.n_opt as u64);
    write_rational(&mut buf, &pp.pool_influence);
    write_rational(&mut buf, &pp.monetary_expansion);
    write_rational(&mut buf, &pp.treasury_growth);
    write_uint_canonical(&mut buf, pp.protocol_major as u64);
    write_uint_canonical(&mut buf, pp.protocol_minor as u64);
    write_coin(&mut buf, pp.min_utxo_value);
    write_coin(&mut buf, pp.min_pool_cost);
    write_rational(&mut buf, &pp.decentralization);
    blake2b_256(&buf)
}

fn fingerprint_governance(gov: Option<&ConwayGovState>) -> Hash32 {
    let mut buf = Vec::new();
    write_component_header(&mut buf, b"ade/fp/governance");
    match gov {
        None => write_null(&mut buf),
        Some(g) => {
            write_array_canonical(&mut buf, 9);

            // 1. proposals
            write_array_canonical(&mut buf, g.proposals.len() as u64);
            for proposal in &g.proposals {
                write_gov_action_state(&mut buf, proposal);
            }

            // 2. committee
            write_map_canonical(&mut buf, g.committee.len() as u64);
            for (cred, expiry) in &g.committee {
                write_hash28(&mut buf, cred);
                write_uint_canonical(&mut buf, *expiry);
            }

            // 3. committee_quorum
            write_array_canonical(&mut buf, 2);
            write_uint_canonical(&mut buf, g.committee_quorum.0);
            write_uint_canonical(&mut buf, g.committee_quorum.1);

            // 4. drep_expiry
            write_map_canonical(&mut buf, g.drep_expiry.len() as u64);
            for (cred, expiry) in &g.drep_expiry {
                write_hash28(&mut buf, cred);
                write_uint_canonical(&mut buf, *expiry);
            }

            // 5. gov_action_lifetime
            write_uint_canonical(&mut buf, g.gov_action_lifetime);

            // 6. vote_delegations
            write_map_canonical(&mut buf, g.vote_delegations.len() as u64);
            for (cred, drep) in &g.vote_delegations {
                write_hash28(&mut buf, cred);
                write_drep(&mut buf, drep);
            }

            // 7. pool_voting_thresholds
            write_array_canonical(&mut buf, g.pool_voting_thresholds.len() as u64);
            for (num, den) in &g.pool_voting_thresholds {
                write_array_canonical(&mut buf, 2);
                write_uint_canonical(&mut buf, *num);
                write_uint_canonical(&mut buf, *den);
            }

            // 8. drep_voting_thresholds
            write_array_canonical(&mut buf, g.drep_voting_thresholds.len() as u64);
            for (num, den) in &g.drep_voting_thresholds {
                write_array_canonical(&mut buf, 2);
                write_uint_canonical(&mut buf, *num);
                write_uint_canonical(&mut buf, *den);
            }

            // 9. committee_hot_keys
            write_map_canonical(&mut buf, g.committee_hot_keys.len() as u64);
            for (hot, cold) in &g.committee_hot_keys {
                write_hash28(&mut buf, hot);
                write_hash28(&mut buf, cold);
            }
        }
    }
    blake2b_256(&buf)
}

// ---------------------------------------------------------------------------
// Rollup and structural helpers
// ---------------------------------------------------------------------------

fn rollup(hashes: &[&Hash32]) -> Hash32 {
    let mut buf = Vec::with_capacity(32 * hashes.len());
    for h in hashes {
        buf.extend_from_slice(&h.0);
    }
    blake2b_256(&buf)
}

fn write_component_header(buf: &mut Vec<u8>, domain: &[u8]) {
    // Each component starts with: array(3) [bstr domain, uint version, <body>]
    write_array_canonical(buf, 3);
    write_bytes_canonical(buf, domain);
    write_uint_canonical(buf, FINGERPRINT_VERSION);
}

fn write_array_canonical(buf: &mut Vec<u8>, count: u64) {
    write_array_header(
        buf,
        ContainerEncoding::Definite(count, canonical_width(count)),
    );
}

fn write_map_canonical(buf: &mut Vec<u8>, count: u64) {
    write_map_header(
        buf,
        ContainerEncoding::Definite(count, canonical_width(count)),
    );
}

fn canonical_width(value: u64) -> IntWidth {
    if value < 24 {
        IntWidth::Inline
    } else if value < 0x100 {
        IntWidth::I8
    } else if value < 0x10000 {
        IntWidth::I16
    } else if value < 0x1_0000_0000 {
        IntWidth::I32
    } else {
        IntWidth::I64
    }
}

// ---------------------------------------------------------------------------
// Primitive writers
// ---------------------------------------------------------------------------

fn write_coin(buf: &mut Vec<u8>, coin: Coin) {
    write_uint_canonical(buf, coin.0);
}

fn write_hash32(buf: &mut Vec<u8>, hash: &Hash32) {
    write_bytes_canonical(buf, &hash.0);
}

fn write_hash28(buf: &mut Vec<u8>, hash: &Hash28) {
    write_bytes_canonical(buf, &hash.0);
}

fn write_pool_id(buf: &mut Vec<u8>, pool: &PoolId) {
    write_hash28(buf, &pool.0);
}

fn write_stake_credential(buf: &mut Vec<u8>, cred: &StakeCredential) {
    write_hash28(buf, &cred.0);
}

fn write_i64_cbor(buf: &mut Vec<u8>, value: i64) {
    if value >= 0 {
        write_uint_canonical(buf, value as u64);
    } else {
        // CBOR nint major 1 encodes -1 - n. For any i64 value (including MIN),
        // -(value + 1) is representable in i64 and non-negative.
        let n = (-(value + 1)) as u64;
        write_argument(buf, MAJOR_NEGATIVE, n, canonical_width(n));
    }
}

/// Encode an `i128` as CBOR major 0 / 1, clamping to `u64` range.
///
/// All ledger-level Rationals (protocol parameter numerators/denominators)
/// fit comfortably in `i64`. Clamping to `u64::MAX` at the extreme is
/// defensive only.
fn write_i128_cbor(buf: &mut Vec<u8>, value: i128) {
    if value >= 0 {
        let clamped = value.min(u64::MAX as i128) as u64;
        write_uint_canonical(buf, clamped);
    } else {
        // n = -1 - value; clamp to u64::MAX if |value| exceeds representable.
        let n_i128 = -1i128 - value;
        let clamped = if n_i128 > u64::MAX as i128 {
            u64::MAX
        } else {
            n_i128 as u64
        };
        write_argument(buf, MAJOR_NEGATIVE, clamped, canonical_width(clamped));
    }
}

// ---------------------------------------------------------------------------
// Composite writers
// ---------------------------------------------------------------------------

fn write_tx_in(buf: &mut Vec<u8>, tx_in: &TxIn) {
    write_array_canonical(buf, 2);
    write_hash32(buf, &tx_in.tx_hash);
    write_uint_canonical(buf, tx_in.index as u64);
}

fn write_tx_out(buf: &mut Vec<u8>, tx_out: &TxOut) {
    match tx_out {
        TxOut::Byron { address, coin } => {
            write_array_canonical(buf, 3);
            write_uint_canonical(buf, 0); // variant tag
            write_bytes_canonical(buf, address.as_bytes());
            write_coin(buf, *coin);
        }
        TxOut::ShelleyMary { address, value } => {
            write_array_canonical(buf, 3);
            write_uint_canonical(buf, 1); // variant tag
            write_bytes_canonical(buf, address);
            write_value(buf, value);
        }
        TxOut::AlonzoPlus { raw, address, coin } => {
            // Fingerprint: tag 2 + address + coin + raw-len + raw.
            // Including raw makes the fingerprint sensitive to
            // datum_hash / datum_option / script_ref changes — a
            // Plutus tx writing a new script-ref is now distinguishable
            // from one that doesn't, which is the whole point of
            // preserving the raw bytes.
            write_array_canonical(buf, 5);
            write_uint_canonical(buf, 2); // variant tag
            write_bytes_canonical(buf, address);
            write_coin(buf, *coin);
            write_uint_canonical(buf, raw.len() as u64);
            buf.extend_from_slice(raw);
        }
    }
}

fn write_value(buf: &mut Vec<u8>, value: &Value) {
    write_array_canonical(buf, 2);
    write_coin(buf, value.coin);
    write_multi_asset(buf, &value.multi_asset);
}

fn write_multi_asset(buf: &mut Vec<u8>, ma: &MultiAsset) {
    write_map_canonical(buf, ma.0.len() as u64);
    for (policy, assets) in &ma.0 {
        write_hash28(buf, policy);
        write_map_canonical(buf, assets.len() as u64);
        for (asset_name, qty) in assets {
            write_bytes_canonical(buf, &asset_name.0);
            write_i64_cbor(buf, *qty);
        }
    }
}

fn write_cert_pool_params(buf: &mut Vec<u8>, p: &CertPoolParams) {
    write_array_canonical(buf, 7);
    write_pool_id(buf, &p.pool_id);
    write_hash32(buf, &p.vrf_hash);
    write_coin(buf, p.pledge);
    write_coin(buf, p.cost);
    write_array_canonical(buf, 2);
    write_uint_canonical(buf, p.margin.0);
    write_uint_canonical(buf, p.margin.1);
    write_bytes_canonical(buf, &p.reward_account);
    write_array_canonical(buf, p.owners.len() as u64);
    for owner in &p.owners {
        write_hash28(buf, owner);
    }
}

fn write_stake_snapshot(buf: &mut Vec<u8>, snap: &StakeSnapshot) {
    write_array_canonical(buf, 2);
    write_map_canonical(buf, snap.delegations.len() as u64);
    for (cred, (pool, coin)) in &snap.delegations {
        write_hash28(buf, cred);
        write_array_canonical(buf, 2);
        write_pool_id(buf, pool);
        write_coin(buf, *coin);
    }
    write_map_canonical(buf, snap.pool_stakes.len() as u64);
    for (pool, coin) in &snap.pool_stakes {
        write_pool_id(buf, pool);
        write_coin(buf, *coin);
    }
}

fn write_rational(buf: &mut Vec<u8>, r: &Rational) {
    write_array_canonical(buf, 2);
    write_i128_cbor(buf, r.numerator());
    write_i128_cbor(buf, r.denominator());
}

fn write_drep(buf: &mut Vec<u8>, drep: &DRep) {
    match drep {
        DRep::KeyHash(h) => {
            write_array_canonical(buf, 2);
            write_uint_canonical(buf, 0);
            write_hash28(buf, h);
        }
        DRep::ScriptHash(h) => {
            write_array_canonical(buf, 2);
            write_uint_canonical(buf, 1);
            write_hash28(buf, h);
        }
        DRep::AlwaysAbstain => {
            write_array_canonical(buf, 1);
            write_uint_canonical(buf, 2);
        }
        DRep::AlwaysNoConfidence => {
            write_array_canonical(buf, 1);
            write_uint_canonical(buf, 3);
        }
    }
}

// ---------------------------------------------------------------------------
// Governance writers
// ---------------------------------------------------------------------------

fn write_gov_action_state(buf: &mut Vec<u8>, s: &GovActionState) {
    write_array_canonical(buf, 9);
    write_gov_action_id(buf, &s.action_id);
    write_vote_list(buf, &s.committee_votes);
    write_vote_list(buf, &s.drep_votes);
    write_vote_list(buf, &s.spo_votes);
    write_coin(buf, s.deposit);
    write_bytes_canonical(buf, &s.return_addr);
    write_gov_action(buf, &s.gov_action);
    write_uint_canonical(buf, s.proposed_in.0);
    write_uint_canonical(buf, s.expires_after.0);
}

fn write_gov_action_id(buf: &mut Vec<u8>, id: &GovActionId) {
    write_array_canonical(buf, 2);
    write_hash32(buf, &id.tx_hash);
    write_uint_canonical(buf, id.index as u64);
}

fn write_vote_list(buf: &mut Vec<u8>, votes: &[(Hash28, Vote)]) {
    // Vec<(Hash28, Vote)> insertion order is not a state-level invariant;
    // sort by credential for canonical encoding.
    let mut sorted: Vec<(&Hash28, Vote)> = votes.iter().map(|(h, v)| (h, *v)).collect();
    sorted.sort_by(|(a, _), (b, _)| a.cmp(b));
    write_array_canonical(buf, sorted.len() as u64);
    for (hash, vote) in sorted {
        write_array_canonical(buf, 2);
        write_hash28(buf, hash);
        write_uint_canonical(buf, vote_tag(vote));
    }
}

fn vote_tag(vote: Vote) -> u64 {
    match vote {
        Vote::No => 0,
        Vote::Yes => 1,
        Vote::Abstain => 2,
    }
}

fn write_gov_action(buf: &mut Vec<u8>, action: &GovAction) {
    match action {
        GovAction::ParameterChange {
            prev_action,
            update,
            policy_hash,
        } => {
            write_array_canonical(buf, 4);
            write_uint_canonical(buf, 0);
            write_optional_gov_action_id(buf, prev_action.as_ref());
            write_bytes_canonical(buf, update);
            write_optional_hash28(buf, policy_hash.as_ref());
        }
        GovAction::HardForkInitiation {
            prev_action,
            protocol_version,
        } => {
            write_array_canonical(buf, 3);
            write_uint_canonical(buf, 1);
            write_optional_gov_action_id(buf, prev_action.as_ref());
            write_array_canonical(buf, 2);
            write_uint_canonical(buf, protocol_version.0);
            write_uint_canonical(buf, protocol_version.1);
        }
        GovAction::TreasuryWithdrawals {
            withdrawals,
            policy_hash,
        } => {
            write_array_canonical(buf, 3);
            write_uint_canonical(buf, 2);
            // Sort withdrawals by address bytes for determinism.
            let mut sorted: Vec<(&Vec<u8>, Coin)> =
                withdrawals.iter().map(|(a, c)| (a, *c)).collect();
            sorted.sort_by(|(a, _), (b, _)| a.cmp(b));
            write_array_canonical(buf, sorted.len() as u64);
            for (addr, coin) in sorted {
                write_array_canonical(buf, 2);
                write_bytes_canonical(buf, addr);
                write_coin(buf, coin);
            }
            write_optional_hash28(buf, policy_hash.as_ref());
        }
        GovAction::NoConfidence { prev_action } => {
            write_array_canonical(buf, 2);
            write_uint_canonical(buf, 3);
            write_optional_gov_action_id(buf, prev_action.as_ref());
        }
        GovAction::UpdateCommittee { prev_action, raw } => {
            write_array_canonical(buf, 3);
            write_uint_canonical(buf, 4);
            write_optional_gov_action_id(buf, prev_action.as_ref());
            write_bytes_canonical(buf, raw);
        }
        GovAction::NewConstitution { prev_action, raw } => {
            write_array_canonical(buf, 3);
            write_uint_canonical(buf, 5);
            write_optional_gov_action_id(buf, prev_action.as_ref());
            write_bytes_canonical(buf, raw);
        }
        GovAction::InfoAction => {
            write_array_canonical(buf, 1);
            write_uint_canonical(buf, 6);
        }
    }
}

fn write_optional_gov_action_id(buf: &mut Vec<u8>, id: Option<&GovActionId>) {
    match id {
        None => write_null(buf),
        Some(id) => write_gov_action_id(buf, id),
    }
}

fn write_optional_hash28(buf: &mut Vec<u8>, hash: Option<&Hash28>) {
    match hash {
        None => write_null(buf),
        Some(h) => write_hash28(buf, h),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn fingerprint_is_deterministic() {
        let s = LedgerState::new(CardanoEra::Shelley);
        let f1 = fingerprint(&s);
        let f2 = fingerprint(&s);
        assert_eq!(f1, f2);
    }

    #[test]
    fn different_eras_have_different_era_hashes() {
        let s_shelley = LedgerState::new(CardanoEra::Shelley);
        let s_allegra = LedgerState::new(CardanoEra::Allegra);
        let f_shelley = fingerprint(&s_shelley);
        let f_allegra = fingerprint(&s_allegra);
        assert_ne!(f_shelley.era, f_allegra.era);
        assert_ne!(f_shelley.combined, f_allegra.combined);
    }

    #[test]
    fn treasury_change_isolates_to_epoch_component() {
        let s1 = LedgerState::new(CardanoEra::Shelley);
        let mut s2 = s1.clone();
        s2.epoch_state.treasury = Coin(1);

        let f1 = fingerprint(&s1);
        let f2 = fingerprint(&s2);

        assert_eq!(f1.era, f2.era);
        assert_eq!(f1.utxo, f2.utxo);
        assert_eq!(f1.cert, f2.cert);
        assert_ne!(f1.epoch, f2.epoch);
        assert_eq!(f1.snapshots, f2.snapshots);
        assert_eq!(f1.pparams, f2.pparams);
        assert_eq!(f1.governance, f2.governance);
        assert_ne!(f1.combined, f2.combined);
    }

    #[test]
    fn utxo_insert_isolates_to_utxo_component() {
        let s1 = LedgerState::new(CardanoEra::Shelley);
        let mut s2 = s1.clone();
        s2.utxo_state.utxos.insert(
            TxIn {
                tx_hash: Hash32([0x01; 32]),
                index: 0,
            },
            TxOut::Byron {
                address: ade_types::address::Address::Byron(vec![0xaa]),
                coin: Coin(100),
            },
        );

        let f1 = fingerprint(&s1);
        let f2 = fingerprint(&s2);

        assert_eq!(f1.era, f2.era);
        assert_ne!(f1.utxo, f2.utxo);
        assert_eq!(f1.cert, f2.cert);
        assert_eq!(f1.epoch, f2.epoch);
        assert_eq!(f1.snapshots, f2.snapshots);
        assert_eq!(f1.pparams, f2.pparams);
        assert_eq!(f1.governance, f2.governance);
        assert_ne!(f1.combined, f2.combined);
    }

    #[test]
    fn track_utxo_flag_does_not_affect_fingerprint() {
        let mut s1 = LedgerState::new(CardanoEra::Shelley);
        s1.track_utxo = false;
        let mut s2 = LedgerState::new(CardanoEra::Shelley);
        s2.track_utxo = true;
        assert_eq!(
            fingerprint(&s1),
            fingerprint(&s2),
            "track_utxo is a harness flag, must not be fingerprinted"
        );
    }

    #[test]
    fn governance_absent_vs_present_differs() {
        let s_absent = LedgerState::new(CardanoEra::Conway);
        let mut s_present = LedgerState::new(CardanoEra::Conway);
        s_present.gov_state = Some(ConwayGovState {
            proposals: Vec::new(),
            committee: BTreeMap::new(),
            committee_quorum: (2, 3),
            drep_expiry: BTreeMap::new(),
            gov_action_lifetime: 6,
            vote_delegations: BTreeMap::new(),
            pool_voting_thresholds: Vec::new(),
            drep_voting_thresholds: Vec::new(),
            committee_hot_keys: BTreeMap::new(),
        });

        let f_absent = fingerprint(&s_absent);
        let f_present = fingerprint(&s_present);

        assert_ne!(f_absent.governance, f_present.governance);
        assert_eq!(f_absent.utxo, f_present.utxo);
        assert_eq!(f_absent.pparams, f_present.pparams);
    }

    #[test]
    fn combined_hash_is_64_hex_chars() {
        let s = LedgerState::new(CardanoEra::Shelley);
        let f = fingerprint(&s);
        assert_eq!(f.combined_hex().len(), 64);
    }

    /// Golden fingerprint hashes for every era's empty `LedgerState`.
    ///
    /// These hashes are hard-pinned. Any change to the fingerprint encoding
    /// (or to the default `LedgerState` / `ProtocolParameters` values) must
    /// come with a deliberate schema bump via `FINGERPRINT_VERSION` plus an
    /// update to the expected hex below.
    ///
    /// Captured at FINGERPRINT_VERSION = 1 on an empty `LedgerState::new(era)`
    /// with `ProtocolParameters::default()` (Shelley mainnet genesis values).
    #[test]
    fn golden_empty_state_per_era() {
        let cases: &[(CardanoEra, &str)] = &[
            (
                CardanoEra::ByronEbb,
                "51925421496599b5a16a56e1b6faba1435aa2a4db20638734b3c4af1b562361f",
            ),
            (
                CardanoEra::ByronRegular,
                "a9f5b2235da477cdf875621293c5668940cee267047aaaa73e51c2f4e9269fc0",
            ),
            (
                CardanoEra::Shelley,
                "9ecbf79943422f72aa6ce6086201239ea2e73354464be788226c66ea5abdcea4",
            ),
            (
                CardanoEra::Allegra,
                "4975fae5bcbbff43fcfb22f49b4ef77511f9adc65e12fac636d5602eb85f2c69",
            ),
            (
                CardanoEra::Mary,
                "1355112da8f327da4f92649706a849a6ea94da48e61a0a3864b6040fd52c1aea",
            ),
            (
                CardanoEra::Alonzo,
                "8505c7d61da6f96ca8b6a85389d7656334de112191f12c6920412ddf78469e2a",
            ),
            (
                CardanoEra::Babbage,
                "045b7fb1a74568c0f78210ad0fa7d2cac1dba72ef8f6ac7a425adc082b0008a9",
            ),
            (
                CardanoEra::Conway,
                "4b569a2b7c8e013d9d04202f3def36c8d7c8165954775a2150580b165888a816",
            ),
        ];
        for (era, expected) in cases {
            let s = LedgerState::new(*era);
            let f = fingerprint(&s);
            assert_eq!(
                f.combined_hex(),
                *expected,
                "golden fingerprint drift for {era:?} — bump FINGERPRINT_VERSION and update golden if this is intentional"
            );
        }
    }

    #[test]
    fn component_hashes_are_distinct_for_empty_state() {
        // Domain-separated component headers should produce distinct
        // hashes even when each sub-component is "empty".
        let s = LedgerState::new(CardanoEra::Shelley);
        let f = fingerprint(&s);

        // Collect the seven component hashes — they must all differ.
        let components = [
            &f.era,
            &f.utxo,
            &f.cert,
            &f.epoch,
            &f.snapshots,
            &f.pparams,
            &f.governance,
        ];
        for i in 0..components.len() {
            for j in (i + 1)..components.len() {
                assert_ne!(
                    components[i], components[j],
                    "components {i} and {j} collide for empty state"
                );
            }
        }
    }
}
