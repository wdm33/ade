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
use ade_crypto::utxo_set_commitment::UtxoSetCommitment;
use ade_types::conway::cert::DRep;
use ade_types::conway::governance::{GovAction, GovActionId, GovActionState, Vote};
use ade_types::shelley::cert::StakeCredential;
use ade_types::tx::{Coin, PoolId, TxIn};
use ade_types::{CardanoEra, Hash28, Hash32};

use crate::delegation::{CertState, PoolParams as CertPoolParams};
use crate::epoch::{SnapshotState, StakeSnapshot};
use crate::pparams::{ConwayOnlyDepositParams, ProtocolParameters};
use crate::rational::Rational;
use crate::state::{ConwayGovState, EpochState, LedgerState};
use crate::utxo::{TxOut, UTxOState};
use crate::value::{MultiAsset, Value};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Version of the fingerprint schema. Bump on any encoding change.
const FINGERPRINT_VERSION: u64 = 1;

/// Explicit tag preceding the Conway-only deposit params in the pparams
/// component (PHASE4-B3-S1). Emitted only when the params are present, so the
/// non-Conway pparams encoding is unchanged.
const CONWAY_DEPOSIT_PARAMS_TAG: u64 = 1;

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

/// Compute the v1 (original) per-component fingerprint -- the blake2b-over-sorted-
/// UTxO construction. Retained for HISTORICAL v1 evidence verification; PRODUCTION
/// now uses `fingerprint` (= `fingerprint_v2`, the S1.5b cutover).
///
/// Pure function: same input always produces the same output.
pub fn fingerprint_v1(state: &LedgerState) -> LedgerFingerprint {
    let era = fingerprint_era(state.era, state.max_lovelace_supply);
    let utxo = fingerprint_utxo(&state.utxo_state);
    let cert = fingerprint_cert(&state.cert_state);
    let epoch = fingerprint_epoch(&state.epoch_state);
    let snapshots = fingerprint_snapshots(&state.epoch_state.snapshots);
    let pparams = fingerprint_pparams(&state.protocol_params, state.conway_deposit_params.as_ref());
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

/// The PRODUCTION ledger fingerprint. MEM-OPT-UTXO-DISK S1.5b cutover: now v2
/// (the incremental-capable Ristretto255 set commitment, `fingerprint_v2`). ALL
/// production sites (admission `post_fp`, the seed anchor, snapshot encode/decode,
/// recovery, forward-sync, produce-mode) call this, so they move to v2 TOGETHER.
/// v1 stays as `fingerprint_v1` for HISTORICAL evidence; a v1 store is rejected by
/// a v2 node via the bumped persistent-store schema version (fail-closed).
pub fn fingerprint(state: &LedgerState) -> LedgerFingerprint {
    fingerprint_v2(state)
}

/// MEM-OPT-UTXO-DISK S1.5 fingerprint versions. v1 = the original blake2b-over-
/// sorted UTxO component; v2 = the Ristretto255 set commitment (incremental-
/// capable). S1.5a introduces v2 as the ORACLE without flipping production --
/// `fingerprint` stays v1 until the v2 re-bootstrap (S1.5b). v1 and v2 are
/// EXPLICIT and never silently mixed.
pub const FINGERPRINT_VERSION_V1: u32 = 1;
pub const FINGERPRINT_VERSION_V2: u32 = 2;

/// The v2 ledger fingerprint -- identical to `fingerprint` EXCEPT the UTxO
/// component uses the v2 set commitment (`fingerprint_utxo_v2`). The ORACLE for
/// S1.5b's per-block incremental maintenance. NOT yet the production `post_fp`.
pub fn fingerprint_v2(state: &LedgerState) -> LedgerFingerprint {
    fingerprint_v2_with_utxo(state, fingerprint_utxo_v2(&state.utxo_state))
}

/// MEM-OPT-UTXO-DISK S2b-2c.1b-A: the combined v2 fingerprint computed with a
/// PRECOMPUTED utxo component, instead of scanning the UTxO. While the live admission
/// runs `track_utxo=false` the UTxO is unchanged, so its component is constant and the
/// RED admission loop supplies the cached value (via [`UtxoFpCache`]), skipping the
/// full per-block scan (the S0 churn). The non-utxo components are cheap and always
/// recomputed; the result is byte-identical to [`fingerprint_v2`] exactly when
/// `utxo == fingerprint_utxo_v2(&state.utxo_state)`.
pub fn fingerprint_v2_with_utxo(state: &LedgerState, utxo: Hash32) -> LedgerFingerprint {
    let era = fingerprint_era(state.era, state.max_lovelace_supply);
    let cert = fingerprint_cert(&state.cert_state);
    let epoch = fingerprint_epoch(&state.epoch_state);
    let snapshots = fingerprint_snapshots(&state.epoch_state.snapshots);
    let pparams = fingerprint_pparams(&state.protocol_params, state.conway_deposit_params.as_ref());
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

/// MEM-OPT-UTXO-DISK S2b-2c.1b-A: caches the UTxO-component fingerprint keyed on the
/// UTxO's generation, so the RED admission loop skips the full per-block UTxO scan
/// WHILE the UTxO is unchanged. In the live `track_utxo=false` path the UTxO is cloned
/// unchanged every block, so its generation — and its fingerprint — never change. Any
/// UTxO mutation bumps the generation, so a changed UTxO MISSES the cache and is
/// recomputed: the cache can NEVER serve a stale fingerprint. The returned value is
/// always byte-identical to the full recompute — a pure optimization, replay-safe.
#[derive(Debug, Clone, Default)]
pub struct UtxoFpCache {
    entry: Option<(u64, Hash32)>,
}

impl UtxoFpCache {
    pub fn new() -> Self {
        UtxoFpCache { entry: None }
    }

    /// The UTxO-component fingerprint: reuse iff the generation matches the cached
    /// one; otherwise recompute (the full scan) and refresh the cache.
    pub fn utxo_fingerprint(&mut self, utxo_state: &UTxOState) -> Hash32 {
        let generation = utxo_state.utxos.generation();
        if let Some((cached_gen, fp)) = &self.entry {
            if *cached_gen == generation {
                return fp.clone();
            }
        }
        let fp = fingerprint_utxo_v2(utxo_state);
        self.entry = Some((generation, fp.clone()));
        fp
    }
}

/// MEM-OPT-UTXO-DISK S2b-2c.1b-A.2: the EXPLICIT constant UTxO-component fingerprint
/// for the live `track_utxo=false` path. The bootstrap computes the UTxO component
/// ONCE (before dropping the in-memory UTxO), and the live admission supplies it to
/// `post_fp` directly — so the node retains an explicit fingerprint instead of a
/// 1.9M-entry in-memory UTxO. The UTxO's durability is the EXISTING snapshot; this
/// carries only the committed-once fingerprint, NOT a second copy.
///
/// It is INVALID for `track_utxo=true` (which mutates the UTxO per block and needs a
/// real UTxO state / the redb-WorkingSet path); [`utxo_component`](Self::utxo_component)
/// fails closed there — never a silently stale fingerprint. This is NOT full live
/// UTxO validation; it optimizes the current header/tip-following path only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaticUtxoFp {
    /// The fingerprint version this commits under (must be the production v2).
    pub fingerprint_version: u32,
    /// The bootstrap anchor fingerprint this UTxO component belongs to.
    pub bootstrap_anchor: Hash32,
    /// The constant UTxO-component fingerprint — `fingerprint_utxo_v2` of the
    /// imported UTxO, computed once before the in-memory copy is dropped.
    pub utxo_component_fp: Hash32,
    /// Structural guard: this is ONLY valid while the live path runs `track_utxo=false`.
    pub valid_only_when_track_utxo_false: bool,
}

/// Why a [`StaticUtxoFp`] may not be used (fail-closed, never a stale fingerprint).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StaticUtxoFpError {
    /// Used on a `track_utxo=true` path — the UTxO mutates, so a static component
    /// would be stale; a real UTxO state / the redb-WorkingSet path is required.
    UsedUnderTrackUtxoTrue,
    /// The fingerprint version did not match the production v2.
    VersionMismatch { expected: u32, found: u32 },
}

impl StaticUtxoFp {
    /// Build from the imported UTxO state, BEFORE it is dropped — computing the UTxO
    /// component once under the production v2 fingerprint version.
    pub fn from_bootstrap_utxo(utxo: &UTxOState, bootstrap_anchor: Hash32) -> Self {
        StaticUtxoFp {
            fingerprint_version: FINGERPRINT_VERSION_V2,
            bootstrap_anchor,
            utxo_component_fp: fingerprint_utxo_v2(utxo),
            valid_only_when_track_utxo_false: true,
        }
    }

    /// The constant UTxO-component fingerprint — valid ONLY when `track_utxo` is
    /// false. Fails closed under `track_utxo=true` or a version mismatch, so a stale
    /// static component can never enter an authoritative `post_fp`.
    pub fn utxo_component(&self, track_utxo: bool) -> Result<Hash32, StaticUtxoFpError> {
        if self.fingerprint_version != FINGERPRINT_VERSION_V2 {
            return Err(StaticUtxoFpError::VersionMismatch {
                expected: FINGERPRINT_VERSION_V2,
                found: self.fingerprint_version,
            });
        }
        if track_utxo || !self.valid_only_when_track_utxo_false {
            return Err(StaticUtxoFpError::UsedUnderTrackUtxoTrue);
        }
        Ok(self.utxo_component_fp.clone())
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

/// MEM-OPT-UTXO-DISK S1.5a: the v2 UTxO fingerprint component -- a commutative
/// Ristretto255 set commitment over the SAME canonical (write_tx_in ++
/// write_tx_out) entry bytes as v1, so it can be maintained in O(delta)/block
/// (S1.5b). This is the FULL recompute (the oracle); S1.5b proves the per-block
/// incremental maintenance equals it.
pub fn fingerprint_utxo_v2(utxo: &UTxOState) -> Hash32 {
    let mut commit = UtxoSetCommitment::empty();
    for (tx_in, tx_out) in &utxo.utxos {
        commit.add(&v2_entry_bytes(tx_in, tx_out));
    }
    commit.digest()
}

/// The canonical entry bytes (`write_tx_in ++ write_tx_out`) the v2 commitment
/// hashes -- shared by the full recompute and the incremental maintenance, so
/// they commit IDENTICALLY.
fn v2_entry_bytes(tx_in: &TxIn, tx_out: &TxOut) -> Vec<u8> {
    let mut entry = Vec::new();
    write_tx_in(&mut entry, tx_in);
    write_tx_out(&mut entry, tx_out);
    entry
}

/// MEM-OPT-UTXO-DISK S1.5b: incremental maintenance of the v2 UTxO fingerprint.
/// Wraps the set commitment; `produce`/`spend` update it in O(1) over the SAME
/// canonical entry bytes as `fingerprint_utxo_v2` (the oracle), so the running
/// `digest()` is PROVEN equal to the full recompute after every admitted block
/// (the equivalence test). The on-disk backend (S2) uses this so `post_fp` is
/// O(delta)/block, not O(n).
///
/// Membership is NOT tracked here -- the ledger's validation (`utxo_delete` ->
/// `InputNotFound`) is what fails closed on a bad spend; this mirrors only VALID,
/// post-validation deltas.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IncrementalUtxoFp {
    commit: UtxoSetCommitment,
}

impl IncrementalUtxoFp {
    /// An empty UTxO set's incremental fingerprint.
    pub fn empty() -> Self {
        IncrementalUtxoFp {
            commit: UtxoSetCommitment::empty(),
        }
    }

    /// Record a produced output. Commutative; the exact inverse of `spend`.
    pub fn produce(&mut self, tx_in: &TxIn, tx_out: &TxOut) {
        self.commit.add(&v2_entry_bytes(tx_in, tx_out));
    }

    /// Record a spent input (the `(tx_in, tx_out)` as it lived in the set).
    pub fn spend(&mut self, tx_in: &TxIn, tx_out: &TxOut) {
        self.commit.remove(&v2_entry_bytes(tx_in, tx_out));
    }

    /// The v2 UTxO fingerprint component (== `fingerprint_utxo_v2` for the same set).
    pub fn digest(&self) -> Hash32 {
        self.commit.digest()
    }
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

    // NOTE: `future_pools` is intentionally NOT fingerprinted here. The durable ledger
    // fingerprint covers the live (track_utxo=false) state, whose cert state — and hence
    // future_pools — is always empty; including it would change every existing fingerprint
    // (an empty-map header) and break warm-start for no benefit. The bootstrap cert-state
    // artifact commits to future_pools via the manifest's cert_state_hash (the cert_state
    // codec). A future track_utxo=true LIVE-LEDGER-APPLY slice that makes the durable cert
    // state carry staged re-registrations must add it here under a fingerprint-version bump.

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

fn fingerprint_pparams(
    pp: &ProtocolParameters,
    conway_deposits: Option<&ConwayOnlyDepositParams>,
) -> Hash32 {
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
    // The min-UTxO rule's coin payload only — byte-identical to the prior
    // single-`Coin` field, so the pinned non-Conway pparams fingerprints are
    // unchanged.
    write_coin(&mut buf, pp.min_utxo_rule.coin());
    write_coin(&mut buf, pp.min_pool_cost);
    write_rational(&mut buf, &pp.decentralization);
    // Conway deposit-param migration (PHASE4-B3-S1): the two Conway-only
    // deposit params enter the fingerprint ONLY when present, under an explicit
    // versioned tag appended after the unchanged 17-field body. A non-Conway
    // state carries `None` here, emits nothing, and so its pparams fingerprint
    // is byte-identical to the pre-migration encoding.
    if let Some(c) = conway_deposits {
        write_uint_canonical(&mut buf, CONWAY_DEPOSIT_PARAMS_TAG);
        // PHASE4-B5-S1: array extended 2 -> 3 to include the Conway-only
        // `drep_activity` parameter. Additive and gated — only states carrying
        // `conway_deposit_params` differ; non-Conway / param-absent states emit
        // nothing here and keep their pre-B5 pparams fingerprint byte-identical.
        write_array_canonical(&mut buf, 3);
        write_coin(&mut buf, c.drep_deposit);
        write_coin(&mut buf, c.gov_action_deposit);
        write_uint_canonical(&mut buf, c.drep_activity);
    }
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
                write_stake_credential(&mut buf, cred);
                write_uint_canonical(&mut buf, *expiry);
            }

            // 3. committee_quorum
            write_array_canonical(&mut buf, 2);
            write_uint_canonical(&mut buf, g.committee_quorum.0);
            write_uint_canonical(&mut buf, g.committee_quorum.1);

            // 4. drep_expiry
            write_map_canonical(&mut buf, g.drep_expiry.len() as u64);
            for (cred, expiry) in &g.drep_expiry {
                write_stake_credential(&mut buf, cred);
                write_uint_canonical(&mut buf, *expiry);
            }

            // 5. gov_action_lifetime
            write_uint_canonical(&mut buf, g.gov_action_lifetime);

            // 6. vote_delegations
            write_map_canonical(&mut buf, g.vote_delegations.len() as u64);
            for (cred, drep) in &g.vote_delegations {
                write_stake_credential(&mut buf, cred);
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
                write_stake_credential(&mut buf, hot);
                write_stake_credential(&mut buf, cold);
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
    match cred {
        StakeCredential::KeyHash(h) => {
            write_uint_canonical(buf, 0);
            write_hash28(buf, h);
        }
        StakeCredential::ScriptHash(h) => {
            write_uint_canonical(buf, 1);
            write_hash28(buf, h);
        }
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
            // Output quantity = CBOR unsigned int. For any representable value
            // (≤ i64::MAX) this is byte-identical to the prior signed encoding,
            // and a quantity > i64::MAX now fingerprints faithfully as a u64.
            write_uint_canonical(buf, qty.0);
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
    write_credential_vote_list(buf, &s.committee_votes);
    write_credential_vote_list(buf, &s.drep_votes);
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

fn write_credential_vote_list(buf: &mut Vec<u8>, votes: &[(StakeCredential, Vote)]) {
    // Vec<(StakeCredential, Vote)> insertion order is not a state-level invariant;
    // sort by the discriminated credential's Ord for canonical encoding.
    let mut sorted: Vec<(&StakeCredential, Vote)> =
        votes.iter().map(|(c, v)| (c, *v)).collect();
    sorted.sort_by(|(a, _), (b, _)| a.cmp(b));
    write_array_canonical(buf, sorted.len() as u64);
    for (cred, vote) in sorted {
        write_array_canonical(buf, 2);
        write_stake_credential(buf, cred);
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
        GovAction::UpdateCommittee { prev_action, removed, added, threshold } => {
            // Structured Conway `update_committee` (5 fields), serialized in the
            // internal canonical convention (discriminated cold credentials via
            // write_stake_credential). Replaces the prior opaque [4, prev, bytes]
            // encoding — a deliberate fingerprint migration (T-DET-01). BTreeSet/
            // BTreeMap give deterministic member order.
            write_array_canonical(buf, 5);
            write_uint_canonical(buf, 4);
            write_optional_gov_action_id(buf, prev_action.as_ref());
            write_array_canonical(buf, removed.len() as u64);
            for cred in removed {
                write_stake_credential(buf, cred);
            }
            write_map_canonical(buf, added.len() as u64);
            for (cred, epoch) in added {
                write_stake_credential(buf, cred);
                write_uint_canonical(buf, *epoch);
            }
            write_uint_canonical(buf, threshold.0);
            write_uint_canonical(buf, threshold.1);
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

    // ---- MEM-OPT-UTXO-DISK S1.5a: the v2 fingerprint (Ristretto255 set commitment) ----

    fn byron_out(coin: u64, tag: u8) -> TxOut {
        TxOut::Byron {
            address: ade_types::address::Address::Byron(vec![tag]),
            coin: Coin(coin),
        }
    }

    #[test]
    fn fp_versions_are_explicit() {
        assert_eq!(FINGERPRINT_VERSION_V1, 1);
        assert_eq!(FINGERPRINT_VERSION_V2, 2);
        assert_ne!(FINGERPRINT_VERSION_V1, FINGERPRINT_VERSION_V2);
    }

    #[test]
    fn v1_and_v2_utxo_components_differ_only_the_utxo_changes() {
        // The v2 UTxO component is a DIFFERENT construction -- v1 != v2, never
        // silently interchangeable -- and ONLY the utxo component changes.
        let mut s = LedgerState::new(CardanoEra::Shelley);
        s.utxo_state.utxos.insert(
            TxIn { tx_hash: Hash32([0x07; 32]), index: 1 },
            byron_out(42, 0xbb),
        );
        let f1 = fingerprint_v1(&s);
        let f2 = fingerprint_v2(&s);
        assert_ne!(f1.utxo, f2.utxo, "v1 and v2 UTxO components must differ");
        assert_ne!(f1.combined, f2.combined);
        assert_eq!(f1.era, f2.era);
        assert_eq!(f1.cert, f2.cert);
        assert_eq!(f1.epoch, f2.epoch);
        assert_eq!(f1.snapshots, f2.snapshots);
        assert_eq!(f1.pparams, f2.pparams);
        assert_eq!(f1.governance, f2.governance);
    }

    #[test]
    fn fingerprint_v2_is_deterministic() {
        let s = LedgerState::new(CardanoEra::Conway);
        assert_eq!(fingerprint_v2(&s), fingerprint_v2(&s));
    }

    #[test]
    fn fingerprint_v2_utxo_is_insertion_order_independent() {
        let entries = [
            (TxIn { tx_hash: Hash32([0x01; 32]), index: 0 }, 100u64),
            (TxIn { tx_hash: Hash32([0x02; 32]), index: 5 }, 200),
            (TxIn { tx_hash: Hash32([0x03; 32]), index: 9 }, 300),
        ];
        let mk = |order: &[usize]| {
            let mut s = LedgerState::new(CardanoEra::Conway);
            for &i in order {
                let (txin, coin) = &entries[i];
                s.utxo_state.utxos.insert(txin.clone(), byron_out(*coin, 0xcc));
            }
            fingerprint_v2(&s).utxo
        };
        assert_eq!(mk(&[0, 1, 2]), mk(&[2, 0, 1]));
        assert_eq!(mk(&[0, 1, 2]), mk(&[1, 2, 0]));
    }

    #[test]
    fn incremental_v2_equals_full_recompute_after_each_block() {
        // MEM-OPT-UTXO-DISK S1.5b: maintaining the incremental fingerprint through
        // produce/spend deltas yields the SAME digest as the full recompute (the
        // S1.5a oracle, fingerprint_utxo_v2) after EVERY block.
        let txin = |h: u8, i: u16| TxIn { tx_hash: Hash32([h; 32]), index: i };
        let mut utxo = UTxOState::new();
        let mut inc = IncrementalUtxoFp::empty();
        assert_eq!(inc.digest(), fingerprint_utxo_v2(&utxo));

        // Block 0: produce three.
        for (ti, c, t) in [(txin(0x01, 0), 100u64, 0x01u8), (txin(0x02, 0), 200, 0x02), (txin(0x03, 7), 300, 0x03)] {
            let to = byron_out(c, t);
            utxo.utxos.insert(ti.clone(), to.clone());
            inc.produce(&ti, &to);
        }
        assert_eq!(inc.digest(), fingerprint_utxo_v2(&utxo));

        // Block 1: spend 0x01, produce two.
        let spent0 = (txin(0x01, 0), byron_out(100, 0x01));
        utxo.utxos.remove(&spent0.0);
        inc.spend(&spent0.0, &spent0.1);
        for (ti, c, t) in [(txin(0x10, 0), 40u64, 0x10u8), (txin(0x10, 1), 60, 0x11)] {
            let to = byron_out(c, t);
            utxo.utxos.insert(ti.clone(), to.clone());
            inc.produce(&ti, &to);
        }
        assert_eq!(inc.digest(), fingerprint_utxo_v2(&utxo));

        // Block 2: spend 0x02 and 0x10:0, produce one.
        for (ti, c, t) in [(txin(0x02, 0), 200u64, 0x02u8), (txin(0x10, 0), 40, 0x10)] {
            let to = byron_out(c, t);
            utxo.utxos.remove(&ti);
            inc.spend(&ti, &to);
        }
        let np = (txin(0x20, 3), byron_out(95, 0x20));
        utxo.utxos.insert(np.0.clone(), np.1.clone());
        inc.produce(&np.0, &np.1);
        assert_eq!(inc.digest(), fingerprint_utxo_v2(&utxo));
    }

    #[test]
    fn incremental_produce_spend_is_exact_inverse() {
        let ti = TxIn { tx_hash: Hash32([0x55; 32]), index: 2 };
        let to = byron_out(777, 0x55);
        let mut inc = IncrementalUtxoFp::empty();
        let empty = inc.digest();
        inc.produce(&ti, &to);
        assert_ne!(inc.digest(), empty);
        inc.spend(&ti, &to);
        assert_eq!(inc.digest(), empty, "produce then spend must return to the empty digest");
    }

    #[test]
    fn s2a_overlay_split_fingerprints_identically_to_direct_build() {
        // MEM-OPT-UTXO-DISK S2a: a state reached by OVERLAY mutation (the anchor
        // still holds the now-spent 0x01; the overlay carries an insert of 0x03 and
        // a delete tombstone for 0x01) must fingerprint byte-identically to the same
        // effective set built directly into a fresh anchor (empty overlay). Proves
        // the anchor/overlay split is invisible to the authoritative fingerprint.
        let txin = |h: u8, i: u16| TxIn {
            tx_hash: Hash32([h; 32]),
            index: i,
        };

        let anchor: std::collections::BTreeMap<TxIn, TxOut> = [
            (txin(0x01, 0), byron_out(100, 0x01)),
            (txin(0x02, 0), byron_out(200, 0x02)),
        ]
        .into_iter()
        .collect();
        let s0 = UTxOState::from_map(anchor);
        let s1 = crate::utxo::utxo_insert(&s0, txin(0x03, 0), byron_out(300, 0x03));
        let (s_overlay, _) = crate::utxo::utxo_delete(&s1, &txin(0x01, 0)).unwrap();

        // The same effective set {0x02, 0x03} built directly (anchor only).
        let direct: std::collections::BTreeMap<TxIn, TxOut> = [
            (txin(0x02, 0), byron_out(200, 0x02)),
            (txin(0x03, 0), byron_out(300, 0x03)),
        ]
        .into_iter()
        .collect();
        let s_direct = UTxOState::from_map(direct);

        assert_eq!(s_overlay.len(), 2, "0x01 spent, 0x02 + 0x03 live");
        assert_eq!(
            fingerprint_utxo_v2(&s_overlay),
            fingerprint_utxo_v2(&s_direct),
            "overlay-split state must fingerprint identically to the direct build"
        );
    }

    #[test]
    fn fingerprint_v2_with_precomputed_utxo_equals_full_v2() {
        // MEM-OPT-UTXO-DISK S2b-2c.1b-A: supplying the precomputed utxo component
        // yields the byte-identical combined fingerprint as the full scan.
        let txin = |h: u8, i: u16| TxIn {
            tx_hash: Hash32([h; 32]),
            index: i,
        };
        let mut s = LedgerState::new(CardanoEra::Conway);
        s.utxo_state.utxos.insert(txin(0x01, 0), byron_out(100, 1));
        s.utxo_state.utxos.insert(txin(0x02, 5), byron_out(200, 2));
        let utxo_fp = fingerprint_utxo_v2(&s.utxo_state);
        assert_eq!(
            fingerprint_v2_with_utxo(&s, utxo_fp).combined,
            fingerprint_v2(&s).combined,
            "the precomputed-utxo variant must equal the full v2 fingerprint"
        );
    }

    #[test]
    fn utxo_fp_cache_reuses_while_unchanged_and_recomputes_on_change() {
        // The cache may reuse ONLY while the UTxO is unchanged; any mutation bumps
        // the generation, so a changed UTxO is recomputed (never a stale reuse).
        let txin = |h: u8, i: u16| TxIn {
            tx_hash: Hash32([h; 32]),
            index: i,
        };
        let mut s = LedgerState::new(CardanoEra::Conway);
        s.utxo_state.utxos.insert(txin(0x01, 0), byron_out(100, 1));

        let mut cache = UtxoFpCache::new();
        let fp1 = cache.utxo_fingerprint(&s.utxo_state);
        assert_eq!(fp1, fingerprint_utxo_v2(&s.utxo_state), "first call == full scan");

        // a CLONE keeps the same generation (the live track_utxo=false path) -> reuse.
        let s_clone = s.clone();
        assert_eq!(
            cache.utxo_fingerprint(&s_clone.utxo_state),
            fp1,
            "unchanged UTxO (same generation) reuses the cached fp"
        );

        // a MUTATION bumps the generation -> recompute -> the new, correct fp.
        s.utxo_state.utxos.insert(txin(0x02, 0), byron_out(200, 2));
        let fp2 = cache.utxo_fingerprint(&s.utxo_state);
        assert_ne!(fp2, fp1, "a changed UTxO must NOT reuse the stale fp");
        assert_eq!(
            fp2,
            fingerprint_utxo_v2(&s.utxo_state),
            "the recomputed fp matches the full scan"
        );
    }

    #[test]
    fn static_utxo_fp_commits_to_the_imported_utxo_and_drives_post_fp() {
        let txin = |h: u8, i: u16| TxIn {
            tx_hash: Hash32([h; 32]),
            index: i,
        };
        let mut s = LedgerState::new(CardanoEra::Conway);
        s.utxo_state.utxos.insert(txin(0x01, 0), byron_out(100, 1));
        s.utxo_state.utxos.insert(txin(0x02, 0), byron_out(200, 2));
        let anchor = Hash32([0xab; 32]);
        let sfp = StaticUtxoFp::from_bootstrap_utxo(&s.utxo_state, anchor.clone());
        assert_eq!(sfp.fingerprint_version, FINGERPRINT_VERSION_V2);
        assert_eq!(sfp.bootstrap_anchor, anchor);
        assert_eq!(sfp.utxo_component_fp, fingerprint_utxo_v2(&s.utxo_state));
        // it drives post_fp identically to the full fingerprint (track_utxo=false).
        let utxo_fp = sfp.utxo_component(false).unwrap();
        assert_eq!(
            fingerprint_v2_with_utxo(&s, utxo_fp).combined,
            fingerprint_v2(&s).combined,
            "static-fp post_fp must equal the full fingerprint"
        );
    }

    #[test]
    fn static_utxo_fp_fails_closed_under_track_utxo_true_and_version_mismatch() {
        let mut s = LedgerState::new(CardanoEra::Conway);
        s.utxo_state.utxos.insert(
            TxIn {
                tx_hash: Hash32([0x01; 32]),
                index: 0,
            },
            byron_out(1, 1),
        );
        let sfp = StaticUtxoFp::from_bootstrap_utxo(&s.utxo_state, Hash32([0; 32]));
        // track_utxo=true MUST fail closed -- a static component would be stale.
        assert_eq!(
            sfp.utxo_component(true),
            Err(StaticUtxoFpError::UsedUnderTrackUtxoTrue)
        );
        // a non-v2 fingerprint version MUST fail closed.
        let mut wrong_version = sfp.clone();
        wrong_version.fingerprint_version = FINGERPRINT_VERSION_V1;
        assert!(matches!(
            wrong_version.utxo_component(false),
            Err(StaticUtxoFpError::VersionMismatch { .. })
        ));
    }

    #[test]
    fn fingerprint_v2_with_utxo_ignores_state_utxo_so_empty_ledger_matches_full() {
        // MEM-OPT-UTXO-DISK S2b-2c.1b-A.2 post_fp equivalence: the off-heap path passes
        // the constant UTxO component + an EMPTY in-memory UTxO. post_fp is INDEPENDENT
        // of state.utxo_state (only the passed component + the non-UTxO state matter),
        // so the empty-UTxO live ledger yields the SAME post_fp as the full-UTxO ledger
        // given the same (static) component -- byte-identical to the old full-scan path.
        let txin = |h: u8, i: u16| TxIn {
            tx_hash: Hash32([h; 32]),
            index: i,
        };
        let mut full = LedgerState::new(CardanoEra::Conway);
        full.utxo_state.utxos.insert(txin(0x01, 0), byron_out(100, 1));
        full.utxo_state.utxos.insert(txin(0x02, 7), byron_out(200, 2));
        let empty = LedgerState::new(CardanoEra::Conway);
        let component = fingerprint_utxo_v2(&full.utxo_state);
        assert_eq!(
            fingerprint_v2_with_utxo(&empty, component.clone()).combined,
            fingerprint_v2_with_utxo(&full, component.clone()).combined,
            "fingerprint_v2_with_utxo must use only the passed component, not state.utxo_state"
        );
        assert_eq!(
            fingerprint_v2_with_utxo(&empty, component).combined,
            fingerprint_v2(&full).combined,
            "off-heap (empty UTxO + static fp) post_fp == full-UTxO full-scan post_fp"
        );
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

    /// ENACTMENT-COMMITTEE-WRITEBACK S1: the structured `UpdateCommittee`
    /// encoding fingerprints its removed/added/threshold fields (replacing the
    /// prior opaque-bytes `raw`), and the discriminated cold credential is
    /// serialized — a key-hash and a script-hash member of equal bytes
    /// fingerprint differently (DC-LEDGER-10 / R-1 / T-DET-01).
    #[test]
    fn update_committee_structured_fields_change_fingerprint() {
        use ade_types::conway::governance::{GovAction, GovActionId, GovActionState};
        use ade_types::shelley::cert::StakeCredential;
        use ade_types::EpochNo;

        let mk = |action: GovAction| {
            let mut s = LedgerState::new(CardanoEra::Conway);
            s.gov_state = Some(ConwayGovState {
                proposals: vec![GovActionState {
                    action_id: GovActionId { tx_hash: Hash32([0x07; 32]), index: 0 },
                    committee_votes: Vec::new(),
                    drep_votes: Vec::new(),
                    spo_votes: Vec::new(),
                    deposit: Coin(0),
                    return_addr: Vec::new(),
                    gov_action: action,
                    proposed_in: EpochNo(500),
                    expires_after: EpochNo(506),
                }],
                committee: BTreeMap::new(),
                committee_quorum: (2, 3),
                drep_expiry: BTreeMap::new(),
                gov_action_lifetime: 6,
                vote_delegations: BTreeMap::new(),
                pool_voting_thresholds: Vec::new(),
                drep_voting_thresholds: Vec::new(),
                committee_hot_keys: BTreeMap::new(),
            });
            fingerprint(&s).governance
        };

        let key_member = || {
            let mut added = BTreeMap::new();
            added.insert(StakeCredential::KeyHash(Hash28([0x42; 28])), 600u64);
            GovAction::UpdateCommittee {
                prev_action: None,
                removed: Default::default(),
                added,
                threshold: (2, 3),
            }
        };
        let script_member = || {
            let mut added = BTreeMap::new();
            added.insert(StakeCredential::ScriptHash(Hash28([0x42; 28])), 600u64);
            GovAction::UpdateCommittee {
                prev_action: None,
                removed: Default::default(),
                added,
                threshold: (2, 3),
            }
        };
        let diff_threshold = || {
            let mut added = BTreeMap::new();
            added.insert(StakeCredential::KeyHash(Hash28([0x42; 28])), 600u64);
            GovAction::UpdateCommittee {
                prev_action: None,
                removed: Default::default(),
                added,
                threshold: (1, 2),
            }
        };

        // Same bytes, different key/script discriminant → different fingerprint.
        assert_ne!(mk(key_member()), mk(script_member()));
        // Same member, different threshold → different fingerprint (threshold is
        // captured, not dropped as it was under the opaque `raw` encoding).
        assert_ne!(mk(key_member()), mk(diff_threshold()));
        // Deterministic.
        assert_eq!(mk(key_member()), mk(key_member()));
    }

    /// OQ5-S1: a key-hash credential and a script-hash credential over identical
    /// 28 bytes fingerprint differently — the discriminant is serialized. Proven
    /// on the cert component (registrations) and the governance component
    /// (drep_expiry); the change isolates to the affected component.
    #[test]
    fn discriminant_changes_fingerprint() {
        use ade_types::shelley::cert::StakeCredential;

        let bytes = Hash28([0x42; 28]);

        // Cert component: registration keyed by KeyHash vs ScriptHash.
        let mut s_key = LedgerState::new(CardanoEra::Conway);
        s_key
            .cert_state
            .delegation
            .registrations
            .insert(StakeCredential::KeyHash(bytes.clone()), Coin(2_000_000));
        let mut s_script = LedgerState::new(CardanoEra::Conway);
        s_script
            .cert_state
            .delegation
            .registrations
            .insert(StakeCredential::ScriptHash(bytes.clone()), Coin(2_000_000));

        let f_key = fingerprint(&s_key);
        let f_script = fingerprint(&s_script);
        assert_ne!(
            f_key.cert, f_script.cert,
            "cert fingerprint must distinguish key-hash from script-hash credential"
        );
        assert_ne!(f_key.combined, f_script.combined);

        // Governance component: drep_expiry keyed by KeyHash vs ScriptHash.
        let gov = |cred: StakeCredential| {
            let mut s = LedgerState::new(CardanoEra::Conway);
            let mut drep_expiry = BTreeMap::new();
            drep_expiry.insert(cred, 600u64);
            s.gov_state = Some(ConwayGovState {
                proposals: Vec::new(),
                committee: BTreeMap::new(),
                committee_quorum: (2, 3),
                drep_expiry,
                gov_action_lifetime: 6,
                vote_delegations: BTreeMap::new(),
                pool_voting_thresholds: Vec::new(),
                drep_voting_thresholds: Vec::new(),
                committee_hot_keys: BTreeMap::new(),
            });
            s
        };
        let g_key = fingerprint(&gov(StakeCredential::KeyHash(bytes.clone())));
        let g_script = fingerprint(&gov(StakeCredential::ScriptHash(bytes.clone())));
        assert_ne!(
            g_key.governance, g_script.governance,
            "gov fingerprint must distinguish key-hash from script-hash credential"
        );
        assert_ne!(g_key.combined, g_script.combined);
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
            let f = fingerprint_v1(&s);
            assert_eq!(
                f.combined_hex(),
                *expected,
                "v1 golden fingerprint drift for {era:?} — fingerprint_v1 is the FROZEN historical construction (production is v2 via fingerprint)"
            );
        }
    }

    /// The Conway deposit-param migration must not perturb any non-Conway
    /// state's pparams fingerprint: a state whose `conway_deposit_params` is
    /// `None` emits the identical pre-migration encoding. Pinned per era.
    #[test]
    fn pparams_fingerprint_stable_for_non_conway() {
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
        ];
        for (era, expected) in cases {
            let s = LedgerState::new(*era);
            assert_eq!(s.conway_deposit_params, None);
            let f = fingerprint_v1(&s);
            assert_eq!(
                f.combined_hex(),
                *expected,
                "non-Conway v1 fingerprint for {era:?} must be byte-identical post-migration (fingerprint_v1, frozen historical)"
            );
        }
    }

    /// When the Conway-only deposit params are present, they fold into the
    /// pparams component (and only that component); changing either deposit
    /// field flips the pparams hash, and the present encoding matches a golden.
    #[test]
    fn pparams_fingerprint_includes_conway_deposits_when_present() {
        let base = LedgerState::new(CardanoEra::Conway);
        assert_eq!(base.conway_deposit_params, None);

        let mut with_deposits = base.clone();
        with_deposits.conway_deposit_params = Some(crate::pparams::ConwayOnlyDepositParams {
            drep_deposit: Coin(500_000_000),
            gov_action_deposit: Coin(100_000_000_000),
            drep_activity: 20,
        });

        let f_base = fingerprint_v1(&base);
        let f_dep = fingerprint_v1(&with_deposits);

        // Presence changes the pparams component (and the rollup), nothing else.
        assert_ne!(f_base.pparams, f_dep.pparams);
        assert_ne!(f_base.combined, f_dep.combined);
        assert_eq!(f_base.era, f_dep.era);
        assert_eq!(f_base.utxo, f_dep.utxo);
        assert_eq!(f_base.cert, f_dep.cert);
        assert_eq!(f_base.epoch, f_dep.epoch);
        assert_eq!(f_base.snapshots, f_dep.snapshots);
        assert_eq!(f_base.governance, f_dep.governance);

        // Golden for the Conway-with-deposits combined fingerprint.
        assert_eq!(
            f_dep.combined_hex(),
            // PHASE4-B5-S1 migration: Conway-deposit tag extended 2 -> 3 fields
            // to include `drep_activity`. Regenerated golden (was
            // b69422ef…71d9 at the 2-field encoding).
            "d1803cb7fe953b6edf732e003cb2f80058a59acd3bb7feb694307176466a8827",
            "Conway deposit-param fingerprint golden drift — review the migration diff"
        );

        // Changing a deposit field flips the pparams component.
        let mut mutated = with_deposits.clone();
        mutated.conway_deposit_params = Some(crate::pparams::ConwayOnlyDepositParams {
            drep_deposit: Coin(500_000_001),
            gov_action_deposit: Coin(100_000_000_000),
            drep_activity: 20,
        });
        let f_mut = fingerprint(&mutated);
        assert_ne!(f_dep.pparams, f_mut.pparams);
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
