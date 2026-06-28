// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! BLUE `EpochAccumulator` + the `apply_selected_block` authority transition contract
//! (LIVE-LEDGER-EPOCH-TRANSITION S1, DC-EPOCH-19).
//!
//! The accumulator is the small, durable, non-UTxO authority a continuously self-sustaining ledger
//! must evolve to derive EVERY future epoch transition (rewards, stake snapshots, pool/cert lifecycle,
//! leadership authority) WITHOUT another Mithril import, a CLI oracle, or an injected authority. The
//! LARGE stake-bearing UTxO substrate stays in the disk-backed reduced checkpoint (MEM-OPT); this
//! carries only the non-UTxO facts. It is, structurally, `LedgerState` MINUS the full UTxO, PLUS the
//! two-buffer block-production model cardano requires.
//!
//! ## The transition contract (the load-bearing artifact — not the struct)
//!
//! `apply_selected_block(prior, block_bytes, ctx) -> next | LedgerTransitionError` is total,
//! deterministic, replay-equivalent: same `prior` + same `block_bytes` + same `ctx` ⇒ a byte-identical
//! `EpochAccumulator`. No wall-clock, rand, HashMap, float, or I/O (BLUE). The only nondeterminism is
//! the canonical block bytes. The order of effects is the cardano NEWEPOCH order, read off the source.
//! Boundary transitions come FIRST, one per crossed boundary `e = P+1 ..= C`:
//!
//! - apply the completed reward update from the held `nesBprev` over the pre-rotation `go` snapshot
//!   (the reward stake; `Tick.hs` `RupdEnv bprev es`, `PulsingReward.hs` `ssStakeGo`);
//! - SNAP: rotate `mark` from the current stake aggregate, `set` from the old `mark`, `go` from old `set`;
//! - POOLREAP: reap retiring pools and adopt staged future-pool re-registrations;
//! - enactment (Conway RATIFY), reset `nesBcur` and `epoch_fees`, then rotate `nesBprev` from the
//!   just-finished `nesBcur`.
//!
//! Then this block's within-epoch effects, in tx/cert order: certificates evolve the cert state;
//! withdrawals zero the named reward account; the issuer increments `block_production[issuer]`
//! (`nesBcur`); fees add into `epoch_fees`.
//!
//! ## The two-buffer block-production model (cardano nesBprev/nesBcur)
//!
//! The reward applied at the boundary INTO epoch `X` consumes blocks of epoch `X-2` — the value held in
//! `nesBprev` while epoch `X-1` was followed — NOT the just-finished `nesBcur`. So the accumulator
//! carries BOTH `epoch_state.block_production` (= `nesBcur`, the live-accumulating current epoch) and
//! `prev_block_production` (= `nesBprev`, the reward input for the next boundary), and likewise
//! `epoch_fees` / `prev_epoch_fees`. At a boundary the contract feeds `prev_*` to the validated
//! `rules::apply_epoch_boundary_with_registrations` (which reads `epoch_state.block_production` as the
//! to-be-rewarded counts) and then rotates `prev := <finished nesBcur>`, `cur := ∅`.
//!
//! ## Reuse, not reimplementation
//!
//! The boundary reuses the byte-exact-verified `rules::apply_epoch_boundary_with_registrations` (the
//! reward, the pots, `epoch::rotate_snapshots`, and the inline retirement) over a transient UTxO-free
//! `LedgerState` view; the within-epoch cert/governance half reuses `rules::process_block_certificates`; the
//! bootstrap-transient reward seed reuses `delegation::apply_bootstrap_reward_deltas`; future-pool
//! adoption reuses `delegation::apply_pool_reap`. The contract is the deterministic orchestration of
//! these single-authority primitives over the accumulator's non-UTxO state — the new stake for the
//! boundary mark comes from `ctx` (the reduced-checkpoint aggregate), never a full UTxO map.
//!
//! ## Field ownership (what the accumulator OWNS, DEFERS, and FORBIDS)
//!
//! The accumulator is precisely `LedgerState − UTxO + {prev-epoch buffers, pending reward update}` —
//! NOT a second full ledger. The boundary between what it owns and what it defers is load-bearing: if
//! it blurs, the accumulator quietly becomes a parallel ledger and drifts from the real transition.
//!
//! - **OWNED (evolved + persisted here):** `epoch_state` *minus its UTxO* (epoch/slot, mark/set/go
//!   snapshots, reserves/treasury pots, `nesBcur` block-production, accumulating fees),
//!   `prev_block_production`/`prev_epoch_fees` (the `nesBprev` reward buffers), `cert_state`
//!   (delegations / reward accounts / pool + future-pool + retirement maps), the consensus-relevant
//!   `protocol_params`, `gov_state`, `conway_deposit_params`, `max_lovelace_supply`, and the
//!   bootstrap-transient `pending_reward_update`.
//! - **DEFERRED to the reduced checkpoint (read via `ctx`, NEVER stored here):** the per-credential
//!   UTxO stake and the per-pool active-stake aggregate. A boundary's new MARK arrives as
//!   `ctx.boundary_mark` (`StakeByPool`, aggregated over the reduced checkpoint at the prior tip); the
//!   accumulator never holds the UTxO set or recomputes stake from it.
//! - **FORBIDDEN (structurally unrepresentable):** a `UTxOState` / full UTxO map; a full per-credential
//!   stake map; anything that would answer a UTxO query without the reduced checkpoint. The type has no
//!   UTxO field, and `as_ledger_view` always materializes an EMPTY UTxO — so "an accumulator holding a
//!   UTxO" cannot be constructed. `ci/ci_check_epoch_accumulator_no_utxo.sh` guards this mechanically.
//!
//! ## The sufficiency invariant (DC-EPOCH-19, the reason this exists)
//!
//! `EpochAccumulator + reduced UTxO checkpoint + canonical selected-chain blocks` must reproduce the
//! SAME future boundary state (rewards, snapshots, leadership authority) as the full Cardano ledger —
//! WITHOUT holding the full UTxO set in live RAM. This is the missing non-UTxO companion that lets the
//! Mithril-bootstrapped, reduced-checkpoint follower run *forever* past the imported snapshot's
//! authority. S1 establishes the contract + its determinism/replay foundation hermetically; the live
//! byte-exact sufficiency is the cluster's CE-3/CE-4 (S3/S6).
//!
//! ## Canonical persistence
//!
//! `encode_epoch_accumulator` / `decode_epoch_accumulator` are the SOLE pub codec pair (the
//! `bootstrap_bridge`/`seed_consensus_inputs` discipline): a pinned version, definite CBOR containers,
//! fail-closed on unknown version / wrong era / trailing bytes / any non-byte-canonical encoding
//! (re-encode != input). Conway-only, matching the `snapshot/` module. No `Default`, no
//! `#[non_exhaustive]`: every field is required at construction.

use std::collections::BTreeMap;

use ade_codec::cbor::{
    canonical_width, read_array_header, read_bytes, read_map_header, read_uint, write_array_header,
    write_bytes_canonical, write_map_header, write_null, write_uint_canonical, ContainerEncoding,
    IntWidth,
};
use ade_types::shelley::cert::StakeCredential;
use ade_types::tx::{Coin, PoolId};
use ade_types::{CardanoEra, EpochNo, Hash28, SlotNo};

use crate::bootstrap_reward_update::{
    decode_bootstrap_reward_update, encode_bootstrap_reward_update, BootstrapRewardUpdate,
    BootstrapRupdError,
};
use crate::delegation::{apply_bootstrap_reward_deltas, apply_pool_reap, CertState};
use crate::error::LedgerError;
use crate::pparams::{ConwayOnlyDepositParams, ProtocolParameters};
use crate::reduced_aggregate::StakeByPool;
use crate::rules::{apply_epoch_boundary_with_registrations, process_block_certificates};
use crate::snapshot::{
    decode_cert_state, decode_conway_deposit_params, decode_epoch_state, decode_gov_state,
    decode_pparams, encode_cert_state, encode_conway_deposit_params, encode_epoch_state,
    encode_gov_state, encode_pparams, SnapshotDecodeError,
};
use crate::state::{ConwayGovState, EpochState, LedgerState};
use crate::utxo::UTxOState;

/// Pinned wire schema version. Decode rejects any other (fail-closed). v1 = the initial
/// LIVE-LEDGER-EPOCH-TRANSITION accumulator.
pub const EPOCH_ACCUMULATOR_SCHEMA_VERSION: u32 = 1;

/// The non-UTxO authority a self-sustaining ledger maintains beside the disk-backed reduced UTxO
/// checkpoint. Closed record — all fields required at construction; no `Default`, no
/// `#[non_exhaustive]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpochAccumulator {
    /// Current-epoch authority: epoch, last applied slot, the mark/set/go stake snapshots, the
    /// reserves/treasury pots, the CURRENT epoch's accumulating block production (`nesBcur`) and
    /// accumulating fees. The large UTxO/stake set is NOT here — it is the reduced checkpoint.
    pub epoch_state: EpochState,
    /// `nesBprev`: the previous epoch's per-pool block counts — the reward input the NEXT boundary
    /// consumes (cardano `Tick.hs` `RupdEnv nesBprev`). NOT the just-finished `nesBcur`.
    pub prev_block_production: BTreeMap<PoolId, u64>,
    /// The previous epoch's accumulated fees, paired with `prev_block_production` for the boundary
    /// reward (the same epoch's counts + fees).
    pub prev_epoch_fees: Coin,
    /// Delegations + reward accounts + pool registrations + future-pool/retirement maps.
    pub cert_state: CertState,
    /// The protocol parameters the reward + leadership transitions read (rho/tau/a0/k/d/deposits/
    /// protocol-major/…). Carried in full — `rules::apply_epoch_boundary_with_registrations` reads a
    /// `ProtocolParameters`, and the canonical codec already exists.
    pub protocol_params: ProtocolParameters,
    /// Conway governance state (proposals/committee/dreps/thresholds). Conway is the live era, so the
    /// accumulator carries the FULL state the RATIFY enactment at the boundary needs. `None` only for a
    /// governance-untracked accumulator.
    pub gov_state: Option<ConwayGovState>,
    /// Conway-only deposit parameters (`drep_deposit`, `gov_action_deposit`, `drep_activity`).
    pub conway_deposit_params: Option<ConwayOnlyDepositParams>,
    /// Maximum lovelace supply — `circulation = max_lovelace_supply - reserves` (PV≥4 `totalStake`).
    pub max_lovelace_supply: u64,
    /// The era. Conway-scoped (the codec rejects pre-Conway), matching the `snapshot/` discipline.
    pub era: CardanoEra,
    /// The bootstrap-transient reward-update seed (DC-EPOCH-18 / Option-B): the snapshot-bound
    /// precomputed reward delta applied EXACTLY ONCE at its `target_epoch→+1` boundary, then cleared.
    /// `None` once the native RUPD takes over (the first boundary whose entire input epoch was
    /// followed).
    pub pending_reward_update: Option<BootstrapRewardUpdate>,
}

/// Fail-closed error from sealing the bootstrap SEED accumulator (LIVE-LEDGER-EPOCH-TRANSITION S2,
/// PO-3 / CE-2f). A seed that does not match the epoch its manifest binds, or a pre-Conway bootstrap, is
/// REFUSED — never a mis-bound or out-of-era accumulator sealed to the durable store.
#[derive(Debug, PartialEq, Eq)]
pub enum SeedError {
    /// The bootstrapped ledger's epoch disagrees with the manifest-declared seed epoch: the seed is not
    /// bound to the certified point it claims. `ledger_epoch` is what the bootstrapped ledger carries;
    /// `expected_epoch` is what the bootstrap manifest / certified point declares.
    SeedEpochMismatch {
        ledger_epoch: u64,
        expected_epoch: u64,
    },
    /// The bootstrapped ledger's era is pre-Conway — the accumulator is Conway-scoped (`era_tag` is the
    /// `CardanoEra as u64`).
    EraNotSupported { era_tag: u64 },
}

impl EpochAccumulator {
    /// A fresh empty Conway-era accumulator (genesis-shaped; the live seeding is S2).
    pub fn new(era: CardanoEra) -> Self {
        EpochAccumulator {
            epoch_state: EpochState::new(),
            prev_block_production: BTreeMap::new(),
            prev_epoch_fees: Coin(0),
            cert_state: CertState::new(),
            protocol_params: ProtocolParameters::default(),
            gov_state: None,
            conway_deposit_params: None,
            max_lovelace_supply: 45_000_000_000_000_000,
            era,
            pending_reward_update: None,
        }
    }

    /// Seal the bootstrap SEED accumulator from the Mithril-bootstrapped `LedgerState` at the certified
    /// point (LIVE-LEDGER-EPOCH-TRANSITION S2, PO-3 / CE-2f). The non-UTxO authority the within-epoch fold
    /// consumes is taken faithfully from the bootstrapped ledger — `cert_state`, `protocol_params`,
    /// `gov_state`, `conway_deposit_params`, `max_lovelace_supply`, `era`, and `epoch_state`'s epoch / slot
    /// / snapshots / pots.
    ///
    /// **The two-buffer split (cardano `nesBprev` / `nesBcur`).** The bootstrapped `EpochState` carries the
    /// PRIOR epoch's per-pool counts in `block_production` (the `s1a` snapshot's `nesBprev`, re-keyed by the
    /// native assembly) and a cold-start `epoch_fees`. Both are boundary-consumed reward INPUTS, so they seed
    /// the `prev_*` buffers. The accumulator's own `nesBcur` accumulators start FRESH-EMPTY: the live follow
    /// counts the current epoch's blocks and fees only from the certified slot forward (the pre-certified
    /// partial is an S3 boundary-gate reconciliation item — in S2 the boundary is structurally excluded, so
    /// these buffers are never consumed and the store's readiness gate fail-closes any authoritative read
    /// until S3).
    ///
    /// **Manifest binding (CE-2f, fail-closed).** `expected_epoch` is the epoch the bootstrap manifest /
    /// certified point declares the seed is for (the same value the `s1a` decode cross-checks). The seal
    /// REFUSES (`SeedEpochMismatch`) unless the ledger's own `epoch_state.epoch` matches it, so a seed can
    /// never be sealed at an epoch other than the one its manifest binds. Pre-Conway is refused too
    /// (`EraNotSupported`); the accumulator is Conway-scoped, matching the `snapshot/` discipline.
    ///
    /// `pending_reward_update` is the DC-EPOCH-18 / Option-B bootstrap reward seed, supplied by the caller
    /// from the same certified snapshot (also boundary-consumed; gate-protected until S3).
    pub fn seed_from_bootstrap_ledger(
        ledger: &LedgerState,
        expected_epoch: EpochNo,
        pending_reward_update: Option<BootstrapRewardUpdate>,
    ) -> Result<Self, SeedError> {
        if (ledger.era as u8) < (CardanoEra::Conway as u8) {
            return Err(SeedError::EraNotSupported {
                era_tag: ledger.era as u64,
            });
        }
        if ledger.epoch_state.epoch.0 != expected_epoch.0 {
            return Err(SeedError::SeedEpochMismatch {
                ledger_epoch: ledger.epoch_state.epoch.0,
                expected_epoch: expected_epoch.0,
            });
        }

        // The bootstrapped epoch-state's block_production is nesBprev and its epoch_fees is the (cold-start)
        // fee pot — both boundary-consumed reward inputs → the prev_* buffers. nesBcur starts fresh-empty.
        let mut epoch_state = ledger.epoch_state.clone();
        let prev_block_production = std::mem::take(&mut epoch_state.block_production);
        let prev_epoch_fees = std::mem::replace(&mut epoch_state.epoch_fees, Coin(0));

        Ok(EpochAccumulator {
            epoch_state,
            prev_block_production,
            prev_epoch_fees,
            cert_state: ledger.cert_state.clone(),
            protocol_params: ledger.protocol_params.clone(),
            gov_state: ledger.gov_state.clone(),
            conway_deposit_params: ledger.conway_deposit_params.clone(),
            max_lovelace_supply: ledger.max_lovelace_supply,
            era: ledger.era,
            pending_reward_update,
        })
    }

    /// Build the transient UTxO-free `LedgerState` view the reused single-authority primitives
    /// (`apply_epoch_boundary_with_registrations`, `process_block_certificates`) consume. The UTxO is
    /// EMPTY by construction (`track_utxo = false`) — those primitives read `snapshots.go`,
    /// `cert_state`, `protocol_params`, `gov_state`, never a UTxO map. Building a transient view (not
    /// holding a `LedgerState`) is what makes "an accumulator with a full UTxO" unrepresentable.
    fn as_ledger_view(&self) -> LedgerState {
        LedgerState {
            utxo_state: UTxOState::new(),
            epoch_state: self.epoch_state.clone(),
            protocol_params: self.protocol_params.clone(),
            era: self.era,
            track_utxo: false,
            cert_state: self.cert_state.clone(),
            max_lovelace_supply: self.max_lovelace_supply,
            gov_state: self.gov_state.clone(),
            conway_deposit_params: self.conway_deposit_params.clone(),
        }
    }
}

/// The per-block transition context. Carries ONLY the canonical, deterministic facts the live follow
/// already has at the tip-advancing call site (era, the block's epoch from the era schedule, the
/// block's slot, the validated issuer pool, and — for a crossed boundary — the new MARK stake
/// aggregate from the reduced checkpoint). NEVER a peer handle, CLI, or wall-clock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedBlockCtx {
    /// The block's era (must match the decoded envelope era).
    pub era: CardanoEra,
    /// The block's epoch (live: `era_schedule.locate(slot).epoch`). The contract crosses every boundary
    /// in `(prior.epoch+1) ..= block_epoch`.
    pub block_epoch: EpochNo,
    /// The block's slot (becomes the accumulator's `last_slot`).
    pub block_slot: SlotNo,
    /// The block's issuer pool (live: the already-validated `header_input.issuer_pool`). Used for
    /// `block_production[issuer] += 1` — the producer is the header issuer; NO leader-schedule lookup.
    pub issuer_pool: PoolId,
    /// The new MARK stake aggregate for a crossed boundary (live: `aggregate_pool_stake` over the
    /// reduced checkpoint at the prior tip). `None` is a fail-closed boundary error — the accumulator,
    /// being UTxO-free, has no way to recompute the mark, so a boundary REQUIRES it. For multiple
    /// crossed boundaries (the degenerate empty-epoch case) the same prior-tip aggregate applies (no
    /// intervening stake change).
    pub boundary_mark: Option<StakeByPool>,
}

/// Closed, fail-closed error sum for the authority transition. A malformed block, an unknown
/// cert/governance variant on the authority path, an arithmetic overflow, a missing required input, or
/// a boundary gap is TERMINAL — never a silent partial accumulator, never a fabricated default.
#[derive(Debug)]
pub enum LedgerTransitionError {
    /// The block bytes failed to decode (envelope or era body).
    MalformedBlock,
    /// The block's declared era is pre-Conway (the accumulator is Conway-scoped) — `era_tag` is the
    /// `CardanoEra as u64`.
    EraNotSupported { era_tag: u64 },
    /// The decoded block era disagrees with `ctx.era`.
    EraMismatch { ctx: u64, block: u64 },
    /// The block's epoch precedes the accumulator's epoch (a backwards/duplicate boundary).
    BoundaryGap { prior_epoch: u64, block_epoch: u64 },
    /// A boundary needs the new-mark stake aggregate but `ctx.boundary_mark` was `None`.
    MissingBoundaryStake { epoch: u64 },
    /// A certificate or governance apply failed on the authority path.
    CertApply(LedgerError),
    /// An arithmetic overflow in a pot / reward / count / fee.
    ArithmeticOverflow,
    /// A phase-2-invalid tx's consumed collateral is not declarable without the UTxO (`total_collateral`,
    /// key 17, absent). The accumulator is UTxO-free, so the byte-exact fee (= collateral, not the declared
    /// `fee`) cannot be computed here. Fail-closed rather than credit a knowingly-wrong fee; the rare
    /// undeclared-collateral case is gated to S3's byte-exact boundary gate.
    InvalidTxCollateralNeedsUtxo { tx_index: u64 },
    /// A phase-2-invalid tx carries certificates (key 4) or withdrawals (key 5). cardano-ledger discards an
    /// invalid tx's body effects (only its collateral is consumed), but the within-epoch cert/withdrawal
    /// paths do not yet skip invalid txs (a behavior shared with the live reduced-window cert
    /// reconstruction, `reduced_advance::advance_cert_state`). Rather than silently apply a discarded
    /// effect, the live within-epoch transition fail-closes; the invalid-tx body-effect skip is S3's
    /// byte-exact-gate item.
    InvalidTxCarriesAuthorityEffect { tx_index: u64 },
}

/// Apply one durable selected-chain block to the accumulator. Total, deterministic, replay-equivalent.
///
/// The order is the cardano NEWEPOCH order (module docs): every crossed boundary first (reward over the
/// held `nesBprev` + pre-rotation `go`, then SNAP/POOLREAP/enactment, then the `nesBprev` rotation),
/// then this block's within-epoch effects (certs, withdrawals, issuer block-production, fees).
pub fn apply_selected_block(
    prior: &EpochAccumulator,
    block_bytes: &[u8],
    ctx: &SelectedBlockCtx,
) -> Result<EpochAccumulator, LedgerTransitionError> {
    let (era, block) = decode_selected_block(block_bytes)?;
    if (era as u8) < (CardanoEra::Conway as u8) {
        return Err(LedgerTransitionError::EraNotSupported {
            era_tag: era as u64,
        });
    }
    if era != ctx.era {
        return Err(LedgerTransitionError::EraMismatch {
            ctx: ctx.era as u64,
            block: era as u64,
        });
    }
    if ctx.block_epoch.0 < prior.epoch_state.epoch.0 {
        return Err(LedgerTransitionError::BoundaryGap {
            prior_epoch: prior.epoch_state.epoch.0,
            block_epoch: ctx.block_epoch.0,
        });
    }

    let mut acc = prior.clone();
    // 1. Boundary transitions first — one per crossed boundary, empty epochs included. `checked_add`
    //    keeps the transition TOTAL on hostile durable state: if `prior.epoch == u64::MAX` then (by the
    //    boundary-gap guard above) `block_epoch == u64::MAX` too, so there is no boundary to cross — an
    //    empty range, never a wrap to `0..=u64::MAX`.
    if let Some(first_boundary) = prior.epoch_state.epoch.0.checked_add(1) {
        for e in first_boundary..=ctx.block_epoch.0 {
            acc = cross_epoch_boundary(acc, EpochNo(e), ctx)?;
        }
    }
    // 2. Within-epoch effects of THIS block.
    acc = apply_within_epoch(acc, &block, era, ctx)?;
    acc.epoch_state.slot = ctx.block_slot;
    Ok(acc)
}

/// Cross ONE epoch boundary into `target`. Reuses the validated `apply_epoch_boundary_with_registrations`
/// for the reward + pots + snapshot rotation, feeding it the held `prev_block_production`/`prev_epoch_fees`
/// (the `nesBprev` reward inputs), then rotates `prev := <just-finished nesBcur>`, `cur := ∅`. The new
/// MARK comes from `ctx.boundary_mark` (the reduced-checkpoint aggregate) — fail-closed if absent.
pub fn cross_epoch_boundary(
    mut acc: EpochAccumulator,
    target: EpochNo,
    ctx: &SelectedBlockCtx,
) -> Result<EpochAccumulator, LedgerTransitionError> {
    let mark = ctx
        .boundary_mark
        .as_ref()
        .ok_or(LedgerTransitionError::MissingBoundaryStake { epoch: target.0 })?;

    // Bootstrap-transient reward seed (DC-EPOCH-18): applied EXACTLY ONCE at its target boundary,
    // BEFORE the native reward, then cleared. After the seam, `pending_reward_update == None` and the
    // native RUPD carries the chain.
    if let Some(rupd) = acc.pending_reward_update.clone() {
        if rupd.target_epoch.0.checked_add(1) == Some(target.0) {
            apply_bootstrap_reward_deltas(&mut acc.cert_state.delegation, &rupd.reward_delta)
                .map_err(|_| LedgerTransitionError::ArithmeticOverflow)?;
            acc.pending_reward_update = None;
        }
    }

    // Capture the just-finished epoch's nesBcur (it becomes nesBprev after this boundary).
    let finished_blocks = std::mem::take(&mut acc.epoch_state.block_production);
    let finished_fees = acc.epoch_state.epoch_fees;

    // Build the boundary view with the REWARD INPUTS = the held nesBprev + prev fees (what the
    // existing boundary fn reads as `epoch_state.block_production` / `epoch_state.epoch_fees`).
    let mut view = acc.as_ledger_view();
    view.epoch_state.block_production = acc.prev_block_production.clone();
    view.epoch_state.epoch_fees = acc.prev_epoch_fees;

    let (new_view, _accounting) =
        apply_epoch_boundary_with_registrations(&view, target, None, Some(mark));

    // POOLREAP completeness: the boundary fn's inline retirement omits future-pool adoption. Adopt the
    // staged re-registrations so the re-registered VRF is active for the next epoch's leadership (S3
    // reconciles the deposit-vs-delegation-clear ordering byte-exactly).
    let mut cert_state = new_view.cert_state;
    apply_pool_reap(&mut cert_state, target);

    // Read back. `new_view.epoch_state` already has epoch=target, rotated snapshots, updated pots, and
    // block_production/epoch_fees reset to empty/0 (the new epoch's fresh nesBcur).
    acc.epoch_state = new_view.epoch_state;
    acc.cert_state = cert_state;
    acc.gov_state = new_view.gov_state;
    // Rotate the block-production buffers: nesBprev := the just-finished nesBcur.
    acc.prev_block_production = finished_blocks;
    acc.prev_epoch_fees = finished_fees;
    Ok(acc)
}

/// Apply this block's within-epoch effects: certificates + governance (the ledger's own authority),
/// the issuer's block-production count (`nesBcur`), the accumulated fees (`nesBcur`), and withdrawals
/// (zero the named reward accounts). Pure; touches only the affected entries.
fn apply_within_epoch(
    mut acc: EpochAccumulator,
    block: &ade_types::shelley::block::ShelleyBlock,
    era: CardanoEra,
    ctx: &SelectedBlockCtx,
) -> Result<EpochAccumulator, LedgerTransitionError> {
    // Phase-2 validity gate (cardano-ledger UTXOS): decode the block's invalid_transactions set once. It
    // gates ALL within-epoch effects — a VALID tx contributes its declared fee + withdrawals (+ certs); a
    // phase-2-INVALID tx contributes ONLY its consumed collateral as fee, every body effect discarded. The
    // decode is FAIL-CLOSED (not the lenient diagnostic `plutus_eval::decode_invalid_tx_indices`): a corrupt
    // or non-canonical `invalid_transactions` field HALTS the transition rather than silently under-report
    // the set, which would apply a discarded tx's fee/certs/withdrawals to authoritative state.
    let invalid =
        decode_invalid_tx_indices_canonical(block.invalid_txs.as_deref(), block.tx_count)?;

    // Fees + withdrawals (and the invalid-tx authority-effect guard) in one tx-body scan, BEFORE the cert
    // pass: an invalid tx carrying certs/withdrawals fail-closes HERE, so the cert reuse below (which walks
    // every tx) never applies a discarded invalid-tx cert.
    let (total_fees, withdrawals) =
        scan_block_tx_effects(block.tx_count, &block.tx_bodies, &invalid)?;

    // Certificates + governance — reuse the single ledger authority (no parallel reimplementation). After
    // the guard above, no invalid tx carries certs, so only valid txs' certs are applied.
    let (cert_state, gov_state) = {
        let view = acc.as_ledger_view();
        process_block_certificates(block, era, &view).map_err(LedgerTransitionError::CertApply)?
    };
    acc.cert_state = cert_state;
    acc.gov_state = gov_state;

    // Issuer block-production (nesBcur) — the producer is the header issuer (ctx), no leader lookup.
    let entry = acc
        .epoch_state
        .block_production
        .entry(ctx.issuer_pool.clone())
        .or_insert(0);
    *entry = entry
        .checked_add(1)
        .ok_or(LedgerTransitionError::ArithmeticOverflow)?;

    // Epoch fees (nesBcur).
    acc.epoch_state.epoch_fees = Coin(
        acc.epoch_state
            .epoch_fees
            .0
            .checked_add(total_fees)
            .ok_or(LedgerTransitionError::ArithmeticOverflow)?,
    );
    // Withdrawals (valid txs only) zero the named reward accounts.
    for cred in withdrawals {
        if let Some(balance) = acc.cert_state.delegation.rewards.get_mut(&cred) {
            *balance = Coin(0);
        }
    }
    Ok(acc)
}

/// Decode the canonical `[era, block]` envelope into `(era, ShelleyBlock)`. Conway-scoped (the live
/// era); a pre-Conway body is rejected by the caller's era gate. A decode failure is `MalformedBlock`.
fn decode_selected_block(
    block_bytes: &[u8],
) -> Result<(CardanoEra, ade_types::shelley::block::ShelleyBlock), LedgerTransitionError> {
    let env = ade_codec::cbor::envelope::decode_block_envelope(block_bytes)
        .map_err(|_| LedgerTransitionError::MalformedBlock)?;
    let inner = block_bytes
        .get(env.block_start..env.block_end)
        .ok_or(LedgerTransitionError::MalformedBlock)?;
    let block = match env.era {
        CardanoEra::Conway => ade_codec::conway::decode_conway_block(inner)
            .map_err(|_| LedgerTransitionError::MalformedBlock)?
            .decoded()
            .clone(),
        // Pre-Conway / future eras are rejected by the contract's Conway-scoped era gate; decode is
        // not attempted (the canonical persistence + live follow target Conway).
        other => {
            return Err(LedgerTransitionError::EraNotSupported {
                era_tag: other as u64,
            })
        }
    };
    Ok((env.era, block))
}

/// Fail-closed CANONICAL decode of a block's `invalid_transactions` field (the phase-2-invalid tx indices)
/// for the BLUE authority path. Distinct from the lenient diagnostic
/// `plutus_eval::decode_invalid_tx_indices` (which returns an empty/truncated set on malformed CBOR): a
/// corrupt or non-canonical field HALTS the transition (`MalformedBlock`) rather than silently
/// UNDER-reporting the invalid set — which would apply a discarded tx's fee/certs/withdrawals to
/// authoritative cert/reward/fee state with no error (IDD §8 "fail fast on invariant risk"; the module's
/// "a malformed block is TERMINAL" contract). The field must be a DEFINITE array of canonical-minimal,
/// strictly-ascending uints (cardano serializes `invalid_transactions` as a sorted set — strict ascent
/// rejects both duplicates and non-canonical ordering), each `< tx_count` (an index past the last tx is a
/// malformed-block signal, not a silent no-op), with no trailing bytes. Absent (`None`) or an empty field
/// is the empty set.
fn decode_invalid_tx_indices_canonical(
    invalid_txs: Option<&[u8]>,
    tx_count: u64,
) -> Result<std::collections::BTreeSet<u64>, LedgerTransitionError> {
    use ade_codec::cbor;
    let bytes = match invalid_txs {
        Some(b) if !b.is_empty() => b,
        _ => return Ok(std::collections::BTreeSet::new()),
    };
    let mut offset = 0usize;
    let n = match cbor::read_array_header(bytes, &mut offset)
        .map_err(|_| LedgerTransitionError::MalformedBlock)?
    {
        cbor::ContainerEncoding::Definite(n, w) => {
            // The array length header must itself be canonical-minimal (honor the `_canonical` name; a
            // non-minimal length cannot under-report the set, but it is non-canonical).
            if w != cbor::canonical_width(n) {
                return Err(LedgerTransitionError::MalformedBlock);
            }
            n
        }
        cbor::ContainerEncoding::Indefinite => return Err(LedgerTransitionError::MalformedBlock),
    };
    let mut out = std::collections::BTreeSet::new();
    let mut prev: Option<u64> = None;
    for _ in 0..n {
        let (idx, width) = cbor::read_uint(bytes, &mut offset)
            .map_err(|_| LedgerTransitionError::MalformedBlock)?;
        // Canonical-minimal uint encoding.
        if width != cbor::canonical_width(idx) {
            return Err(LedgerTransitionError::MalformedBlock);
        }
        // Strictly ascending — rejects duplicates AND unsorted/non-canonical ordering.
        if let Some(p) = prev {
            if idx <= p {
                return Err(LedgerTransitionError::MalformedBlock);
            }
        }
        // In range: an index at/after the last tx is malformed, not a silent under-trigger.
        if idx >= tx_count {
            return Err(LedgerTransitionError::MalformedBlock);
        }
        prev = Some(idx);
        out.insert(idx);
    }
    // No trailing bytes after the array (a closed surface, not a prefix match).
    if offset != bytes.len() {
        return Err(LedgerTransitionError::MalformedBlock);
    }
    Ok(out)
}

/// The per-tx fields the within-epoch scan collects in one body walk. The fee credited to `epoch_fees`
/// depends on the tx's phase-2 validity (decided against the block's `invalid_transactions` set), so the
/// declared fee (key 2) and the consumed collateral (key 17) are BOTH captured, plus whether the tx
/// carries authority effects (certs key 4 / withdrawals key 5) and the withdrawn reward-account creds.
struct TxScan {
    /// Declared fee (key 2). `None` if absent (cardano requires it for a valid tx; treated as 0).
    fee: Option<u64>,
    /// Declared total collateral (key 17) — the consumed fee for a phase-2-invalid tx.
    total_collateral: Option<u64>,
    /// Whether the tx carries a certificate field (key 4).
    has_certs: bool,
    /// Withdrawn reward-account credentials (key 5 map keys).
    withdrawals: Vec<StakeCredential>,
}

/// Scan the block's tx bodies once, returning `(Σ fees, withdrawn reward-account credentials of VALID
/// txs)` with cardano-ledger UTXOS fee semantics. A VALID tx (index ∉ `invalid`) contributes its declared
/// fee (key 2) and its withdrawals; a phase-2-INVALID tx (index ∈ `invalid`) contributes ONLY its consumed
/// collateral (`total_collateral`, key 17) and NONE of its body effects. Fail-closed: an invalid tx whose
/// collateral is not declarable without the UTxO (`total_collateral` absent) is
/// `InvalidTxCollateralNeedsUtxo`; an invalid tx that carries certs (key 4) or withdrawals (key 5) is
/// `InvalidTxCarriesAuthorityEffect` — the discarded-effect byte-semantics are gated to S3, never silently
/// applied, and that cert guard means `process_block_certificates` (which walks every tx) never applies an
/// invalid tx's certs. Malformed CBOR / fee overflow are fail-closed.
fn scan_block_tx_effects(
    tx_count: u64,
    tx_bodies: &[u8],
    invalid: &std::collections::BTreeSet<u64>,
) -> Result<(u64, Vec<StakeCredential>), LedgerTransitionError> {
    use ade_codec::cbor;
    if tx_count == 0 {
        return Ok((0, Vec::new()));
    }
    let data = tx_bodies;
    let mut offset = 0usize;
    let mut total_fees: u64 = 0;
    let mut withdrawals: Vec<StakeCredential> = Vec::new();

    let mut index: u64 = 0;
    match cbor::read_array_header(data, &mut offset)
        .map_err(|_| LedgerTransitionError::MalformedBlock)?
    {
        cbor::ContainerEncoding::Definite(n, _) => {
            for _ in 0..n {
                let tx = scan_one_tx(data, &mut offset)?;
                apply_tx_scan(tx, index, invalid, &mut total_fees, &mut withdrawals)?;
                index += 1;
            }
        }
        cbor::ContainerEncoding::Indefinite => {
            while !cbor::is_break(data, offset)
                .map_err(|_| LedgerTransitionError::MalformedBlock)?
            {
                let tx = scan_one_tx(data, &mut offset)?;
                apply_tx_scan(tx, index, invalid, &mut total_fees, &mut withdrawals)?;
                index += 1;
            }
        }
    }
    // Mechanical coupling check: the tx index space the scan walked (the tx_bodies array length) must
    // equal the `tx_count` the invalid set was range-checked against in `decode_invalid_tx_indices_canonical`.
    // Equal by construction today (both derive from the same durable bytes), but asserted so a future caller
    // passing a mismatched `tx_count` fails closed rather than mis-aligning the validity gate.
    if index != tx_count {
        return Err(LedgerTransitionError::MalformedBlock);
    }
    Ok((total_fees, withdrawals))
}

/// Walk ONE tx body map, collecting the fields the validity-aware fee/withdrawal decision needs.
fn scan_one_tx(data: &[u8], offset: &mut usize) -> Result<TxScan, LedgerTransitionError> {
    use ade_codec::cbor;
    let mut tx = TxScan {
        fee: None,
        total_collateral: None,
        has_certs: false,
        withdrawals: Vec::new(),
    };
    match cbor::read_map_header(data, offset).map_err(|_| LedgerTransitionError::MalformedBlock)? {
        cbor::ContainerEncoding::Definite(n, _) => {
            for _ in 0..n {
                let (key, _) = cbor::read_uint(data, offset)
                    .map_err(|_| LedgerTransitionError::MalformedBlock)?;
                read_one_tx_field(data, offset, key, &mut tx)?;
            }
        }
        cbor::ContainerEncoding::Indefinite => {
            while !cbor::is_break(data, *offset)
                .map_err(|_| LedgerTransitionError::MalformedBlock)?
            {
                let (key, _) = cbor::read_uint(data, offset)
                    .map_err(|_| LedgerTransitionError::MalformedBlock)?;
                read_one_tx_field(data, offset, key, &mut tx)?;
            }
            *offset += 1; // consume break
        }
    }
    Ok(tx)
}

/// Credit the scanned tx to the running totals per its phase-2 validity (cardano-ledger UTXOS): a valid
/// tx adds its declared fee + withdrawals; an invalid tx adds its consumed collateral and discards its
/// body effects, fail-closed on undeclared collateral or any carried authority effect.
fn apply_tx_scan(
    tx: TxScan,
    index: u64,
    invalid: &std::collections::BTreeSet<u64>,
    total_fees: &mut u64,
    withdrawals: &mut Vec<StakeCredential>,
) -> Result<(), LedgerTransitionError> {
    if invalid.contains(&index) {
        // Phase-2-invalid: only the consumed collateral is a fee; every body effect is discarded.
        if tx.has_certs || !tx.withdrawals.is_empty() {
            return Err(LedgerTransitionError::InvalidTxCarriesAuthorityEffect { tx_index: index });
        }
        let collateral = tx
            .total_collateral
            .ok_or(LedgerTransitionError::InvalidTxCollateralNeedsUtxo { tx_index: index })?;
        *total_fees = total_fees
            .checked_add(collateral)
            .ok_or(LedgerTransitionError::ArithmeticOverflow)?;
    } else {
        // Valid: the declared fee (cardano requires key 2; absent ⇒ 0) + the withdrawals.
        let fee = tx.fee.unwrap_or(0);
        *total_fees = total_fees
            .checked_add(fee)
            .ok_or(LedgerTransitionError::ArithmeticOverflow)?;
        withdrawals.extend(tx.withdrawals);
    }
    Ok(())
}

/// Read one tx-body field into `tx`: key 2 = fee, key 4 = certs (presence only — the byte-exact cert
/// authority is `process_block_certificates`), key 5 = withdrawals (collect reward-account creds), key 17
/// = total collateral. Anything else is skipped. Advances `offset` past the value.
fn read_one_tx_field(
    data: &[u8],
    offset: &mut usize,
    key: u64,
    tx: &mut TxScan,
) -> Result<(), LedgerTransitionError> {
    use ade_codec::cbor;
    match key {
        2 => {
            let (fee, _) =
                cbor::read_uint(data, offset).map_err(|_| LedgerTransitionError::MalformedBlock)?;
            tx.fee = Some(fee);
        }
        4 => {
            // Certs present — the byte-exact cert application is process_block_certificates' job; here we
            // only record presence (the invalid-tx guard needs it). Skip the value.
            let _ =
                cbor::skip_item(data, offset).map_err(|_| LedgerTransitionError::MalformedBlock)?;
            tx.has_certs = true;
        }
        5 => {
            let n = match cbor::read_map_header(data, offset)
                .map_err(|_| LedgerTransitionError::MalformedBlock)?
            {
                cbor::ContainerEncoding::Definite(n, _) => n,
                cbor::ContainerEncoding::Indefinite => {
                    return Err(LedgerTransitionError::MalformedBlock)
                }
            };
            for _ in 0..n {
                let (account, _) = cbor::read_bytes(data, offset)
                    .map_err(|_| LedgerTransitionError::MalformedBlock)?;
                // A stake/reward account is ALWAYS `header ‖ 28-byte credential` (29 bytes). A
                // different length is a malformed body — fail closed, never a silently dropped
                // withdrawal (which would leave a reward account un-zeroed).
                let cred = reward_account_credential(&account)
                    .ok_or(LedgerTransitionError::MalformedBlock)?;
                tx.withdrawals.push(cred);
                // skip the coin value
                let _ = cbor::skip_item(data, offset)
                    .map_err(|_| LedgerTransitionError::MalformedBlock)?;
            }
        }
        17 => {
            let (tc, _) =
                cbor::read_uint(data, offset).map_err(|_| LedgerTransitionError::MalformedBlock)?;
            tx.total_collateral = Some(tc);
        }
        _ => {
            let _ =
                cbor::skip_item(data, offset).map_err(|_| LedgerTransitionError::MalformedBlock)?;
        }
    }
    Ok(())
}

/// Project a Shelley reward account (`header ‖ 28-byte credential`) to a `StakeCredential`. The header
/// high nibble distinguishes key-hash stake (`0xE_`) from script-hash stake (`0xF_`): bit 4 (`0x10`)
/// set ⇒ script. Returns `None` for a malformed (≠ 29-byte) account.
fn reward_account_credential(account: &[u8]) -> Option<StakeCredential> {
    if account.len() != 29 {
        return None;
    }
    let mut cred = [0u8; 28];
    cred.copy_from_slice(&account[1..29]);
    if account[0] & 0x10 != 0 {
        Some(StakeCredential::ScriptHash(Hash28(cred)))
    } else {
        Some(StakeCredential::KeyHash(Hash28(cred)))
    }
}

// ---------------------------------------------------------------------------
// Canonical persistence codec (the SOLE pub pair)
// ---------------------------------------------------------------------------

/// Closed error sum for the accumulator codec. Carries only non-secret primitives.
#[derive(Debug)]
pub enum EpochAccumulatorCodecError {
    /// CBOR primitive read error or a non-byte-canonical encoding (re-encode != input).
    MalformedCbor,
    /// Decoded schema version did not match `EPOCH_ACCUMULATOR_SCHEMA_VERSION`.
    UnknownVersion { expected: u32, found: u32 },
    /// Decoded buffer did not match the expected closed CBOR shape.
    Structural { reason: &'static str },
    /// The decoded era is pre-Conway (the accumulator is Conway-scoped) or an unknown tag.
    EraNotSupported { era_tag: u64 },
    /// A composed sub-state (`epoch_state` / `cert_state` / `pparams` / `gov_state` /
    /// `conway_deposit_params`) failed to decode.
    Snapshot(SnapshotDecodeError),
    /// The embedded bootstrap reward update failed to decode.
    BootstrapReward(BootstrapRupdError),
    /// Trailing bytes after the record.
    TrailingBytes { extra: usize },
}

impl From<ade_codec::CodecError> for EpochAccumulatorCodecError {
    fn from(_e: ade_codec::CodecError) -> Self {
        EpochAccumulatorCodecError::MalformedCbor
    }
}
impl From<SnapshotDecodeError> for EpochAccumulatorCodecError {
    fn from(e: SnapshotDecodeError) -> Self {
        EpochAccumulatorCodecError::Snapshot(e)
    }
}
impl From<BootstrapRupdError> for EpochAccumulatorCodecError {
    fn from(e: BootstrapRupdError) -> Self {
        EpochAccumulatorCodecError::BootstrapReward(e)
    }
}

const FIELDS_OUTER: u64 = 11;

/// Canonical CBOR encode. Sole pub encoder. Composes the existing `snapshot/` sub-codecs + the
/// `nesBprev` buffer + the optional bootstrap reward update.
///
/// Wire shape (v1):
/// ```text
/// array(11) [
///   uint        EPOCH_ACCUMULATOR_SCHEMA_VERSION (= 1),
///   uint        era,                              // 7 = Conway (pre-Conway rejected on decode)
///   uint        max_lovelace_supply,
///   bytes       epoch_state_encoded,              // encode_epoch_state (nesBcur + pots + snapshots)
///   bytes       cert_state_encoded,               // encode_cert_state
///   bytes       pparams_encoded,                  // encode_pparams
///   null | bytes gov_state_encoded,               // encode_gov_state
///   null | bytes conway_deposit_params_encoded,   // encode_conway_deposit_params
///   map(N) PoolId(bytes28) -> uint,               // prev_block_production (nesBprev)
///   uint        prev_epoch_fees,
///   null | bytes pending_reward_update_encoded,   // encode_bootstrap_reward_update
/// ]
/// ```
pub fn encode_epoch_accumulator(acc: &EpochAccumulator) -> Vec<u8> {
    let mut buf = Vec::new();
    write_array_header(
        &mut buf,
        ContainerEncoding::Definite(FIELDS_OUTER, canonical_width(FIELDS_OUTER)),
    );
    write_uint_canonical(&mut buf, EPOCH_ACCUMULATOR_SCHEMA_VERSION as u64);
    write_uint_canonical(&mut buf, acc.era as u64);
    write_uint_canonical(&mut buf, acc.max_lovelace_supply);
    write_bytes_canonical(&mut buf, &encode_epoch_state(&acc.epoch_state));
    write_bytes_canonical(&mut buf, &encode_cert_state(&acc.cert_state));
    write_bytes_canonical(&mut buf, &encode_pparams(&acc.protocol_params));
    match &acc.gov_state {
        Some(g) => write_bytes_canonical(&mut buf, &encode_gov_state(g)),
        None => write_null(&mut buf),
    }
    match &acc.conway_deposit_params {
        Some(p) => write_bytes_canonical(&mut buf, &encode_conway_deposit_params(p)),
        None => write_null(&mut buf),
    }
    write_pool_u64_map(&mut buf, &acc.prev_block_production);
    write_uint_canonical(&mut buf, acc.prev_epoch_fees.0);
    match &acc.pending_reward_update {
        Some(r) => write_bytes_canonical(&mut buf, &encode_bootstrap_reward_update(r)),
        None => write_null(&mut buf),
    }
    buf
}

/// Canonical CBOR decode. Sole pub decoder. Fail-closed on unknown version, wrong era, wrong shape, a
/// sub-state decode failure, trailing bytes, or any non-byte-canonical encoding (re-encode != input).
pub fn decode_epoch_accumulator(
    bytes: &[u8],
) -> Result<EpochAccumulator, EpochAccumulatorCodecError> {
    let mut o = 0usize;
    expect_definite_array(bytes, &mut o, FIELDS_OUTER, "outer")?;

    let version = read_u32_field(bytes, &mut o)?;
    if version != EPOCH_ACCUMULATOR_SCHEMA_VERSION {
        return Err(EpochAccumulatorCodecError::UnknownVersion {
            expected: EPOCH_ACCUMULATOR_SCHEMA_VERSION,
            found: version,
        });
    }
    let era_tag = read_u64_field(bytes, &mut o)?;
    let era = decode_era(era_tag)?;
    if (era as u8) < (CardanoEra::Conway as u8) {
        return Err(EpochAccumulatorCodecError::EraNotSupported { era_tag });
    }
    let max_lovelace_supply = read_u64_field(bytes, &mut o)?;

    let epoch_state = {
        let (b, _) = read_bytes(bytes, &mut o)?;
        decode_epoch_state(&b)?
    };
    let cert_state = {
        let (b, _) = read_bytes(bytes, &mut o)?;
        decode_cert_state(&b)?
    };
    let protocol_params = {
        let (b, _) = read_bytes(bytes, &mut o)?;
        decode_pparams(&b)?
    };
    let gov_state = read_opt_bstr(bytes, &mut o, |b| decode_gov_state(b).map_err(Into::into))?;
    let conway_deposit_params = read_opt_bstr(bytes, &mut o, |b| {
        decode_conway_deposit_params(b).map_err(Into::into)
    })?;
    let prev_block_production = read_pool_u64_map(bytes, &mut o)?;
    let prev_epoch_fees = Coin(read_u64_field(bytes, &mut o)?);
    let pending_reward_update = read_opt_bstr(bytes, &mut o, |b| {
        decode_bootstrap_reward_update(b).map_err(Into::into)
    })?;

    if o != bytes.len() {
        return Err(EpochAccumulatorCodecError::TrailingBytes {
            extra: bytes.len() - o,
        });
    }

    let acc = EpochAccumulator {
        epoch_state,
        prev_block_production,
        prev_epoch_fees,
        cert_state,
        protocol_params,
        gov_state,
        conway_deposit_params,
        max_lovelace_supply,
        era,
        pending_reward_update,
    };

    // Byte-canonical backstop: a structurally valid but non-minimally-encoded buffer (a wider uint, a
    // non-canonical map order, a duplicate key) decodes to the same value but re-encodes to different
    // bytes — reject it fail-closed.
    if encode_epoch_accumulator(&acc) != bytes {
        return Err(EpochAccumulatorCodecError::MalformedCbor);
    }
    Ok(acc)
}

fn write_pool_u64_map(buf: &mut Vec<u8>, m: &BTreeMap<PoolId, u64>) {
    write_map_header(
        buf,
        ContainerEncoding::Definite(m.len() as u64, canonical_width(m.len() as u64)),
    );
    for (pool, count) in m {
        write_bytes_canonical(buf, &pool.0 .0);
        write_uint_canonical(buf, *count);
    }
}

fn read_pool_u64_map(
    bytes: &[u8],
    o: &mut usize,
) -> Result<BTreeMap<PoolId, u64>, EpochAccumulatorCodecError> {
    let n = match read_map_header(bytes, o)? {
        ContainerEncoding::Definite(n, _) => n,
        ContainerEncoding::Indefinite => {
            return Err(EpochAccumulatorCodecError::Structural {
                reason: "indefinite-length map not allowed in prev_block_production",
            })
        }
    };
    // Duplicate / non-canonical key order is caught by the re-encode backstop (a BTreeMap collapses
    // duplicates and re-sorts, so re-encode != input); read straight into the map.
    let mut m = BTreeMap::new();
    for _ in 0..n {
        let (b, _) = read_bytes(bytes, o)?;
        if b.len() != 28 {
            return Err(EpochAccumulatorCodecError::Structural {
                reason: "expected 28-byte pool id",
            });
        }
        let mut arr = [0u8; 28];
        arr.copy_from_slice(&b);
        let (count, _) = read_uint(bytes, o)?;
        m.insert(PoolId(Hash28(arr)), count);
    }
    Ok(m)
}

fn read_opt_bstr<T, F>(
    bytes: &[u8],
    o: &mut usize,
    decode_fn: F,
) -> Result<Option<T>, EpochAccumulatorCodecError>
where
    F: FnOnce(&[u8]) -> Result<T, EpochAccumulatorCodecError>,
{
    let head = *bytes
        .get(*o)
        .ok_or(EpochAccumulatorCodecError::Structural {
            reason: "eof at optional field",
        })?;
    if head == 0xF6 {
        *o += 1;
        return Ok(None);
    }
    let (b, _) = read_bytes(bytes, o)?;
    Ok(Some(decode_fn(&b)?))
}

fn decode_era(tag: u64) -> Result<CardanoEra, EpochAccumulatorCodecError> {
    match tag {
        0 => Ok(CardanoEra::ByronEbb),
        1 => Ok(CardanoEra::ByronRegular),
        2 => Ok(CardanoEra::Shelley),
        3 => Ok(CardanoEra::Allegra),
        4 => Ok(CardanoEra::Mary),
        5 => Ok(CardanoEra::Alonzo),
        6 => Ok(CardanoEra::Babbage),
        7 => Ok(CardanoEra::Conway),
        _ => Err(EpochAccumulatorCodecError::EraNotSupported { era_tag: tag }),
    }
}

fn expect_definite_array(
    bytes: &[u8],
    o: &mut usize,
    expected_len: u64,
    label: &'static str,
) -> Result<(), EpochAccumulatorCodecError> {
    match read_array_header(bytes, o)? {
        ContainerEncoding::Definite(n, _) if n == expected_len => Ok(()),
        ContainerEncoding::Definite(_, _) => Err(EpochAccumulatorCodecError::Structural {
            reason: match label {
                "outer" => "outer array has wrong field count",
                _ => "array has wrong field count",
            },
        }),
        ContainerEncoding::Indefinite => Err(EpochAccumulatorCodecError::Structural {
            reason: "indefinite-length array not allowed",
        }),
    }
}

fn read_u32_field(bytes: &[u8], o: &mut usize) -> Result<u32, EpochAccumulatorCodecError> {
    let (n, _w): (u64, IntWidth) = read_uint(bytes, o)?;
    if n > u32::MAX as u64 {
        return Err(EpochAccumulatorCodecError::Structural {
            reason: "u32 field overflowed",
        });
    }
    Ok(n as u32)
}

fn read_u64_field(bytes: &[u8], o: &mut usize) -> Result<u64, EpochAccumulatorCodecError> {
    let (n, _w): (u64, IntWidth) = read_uint(bytes, o)?;
    Ok(n)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use crate::bootstrap_reward_update::bootstrap_rupd_commitment;
    use crate::delegation::PoolParams;
    use crate::epoch::{GoSnapshot, MarkSnapshot, SetSnapshot, SnapshotState, StakeSnapshot};
    use crate::rational::Rational;
    use ade_types::{Hash32, SlotNo};

    // A real Conway block captured from the live preprod peer (public chain data), reused from the
    // ade_node admission fixture (the project's real-interop discipline: prove on a REAL block).
    const RAW_CONWAY_BLOCK: &[u8] =
        include_bytes!("../../ade_node/tests/fixtures/raw_era_block_conway.cbor");

    fn pool(b: u8) -> PoolId {
        PoolId(Hash28([b; 28]))
    }
    fn key_cred(b: u8) -> StakeCredential {
        StakeCredential::KeyHash(Hash28([b; 28]))
    }
    fn key_account(b: u8) -> Vec<u8> {
        // header 0xE0 (key-hash stake, mainnet) + 28 credential bytes.
        let mut a = vec![0xE0u8];
        a.extend_from_slice(&[b; 28]);
        a
    }
    fn rat(n: i128, d: i128) -> Rational {
        Rational::new(n, d).unwrap()
    }

    fn sample_pool_params(p: u8, reward_account: Vec<u8>) -> PoolParams {
        PoolParams {
            pool_id: pool(p),
            vrf_hash: Hash32([p; 32]),
            pledge: Coin(0),
            cost: Coin(0),
            margin: (0, 1),
            reward_account,
            owners: vec![],
        }
    }

    fn conway_params() -> ProtocolParameters {
        let mut pp = ProtocolParameters::default();
        pp.protocol_major = 9;
        pp.monetary_expansion = rat(3, 1000);
        pp.treasury_growth = rat(1, 5);
        pp.pool_influence = rat(3, 10);
        pp.n_opt = 500;
        pp.decentralization = rat(0, 1);
        pp
    }

    fn sample_mark(p: u8, stake: u64) -> StakeByPool {
        let mut pool_stakes = BTreeMap::new();
        pool_stakes.insert(pool(p), Coin(stake));
        StakeByPool {
            pool_stakes,
            total_active_stake: Coin(stake),
        }
    }

    // A fully populated Conway accumulator exercising every codec field.
    fn populated() -> EpochAccumulator {
        let mut acc = EpochAccumulator::new(CardanoEra::Conway);
        acc.epoch_state.epoch = EpochNo(1340);
        acc.epoch_state.slot = SlotNo(164_000_000);
        acc.epoch_state.reserves = Coin(13_888_022_852_926_644);
        acc.epoch_state.treasury = Coin(1_434_657_232_801_879);
        acc.epoch_state.epoch_fees = Coin(8_321_001_400);
        acc.epoch_state.block_production.insert(pool(0x44), 7);
        let mut snap = StakeSnapshot::new();
        snap.delegations
            .insert(Hash28([0x33; 28]), (pool(0x44), Coin(1_000)));
        snap.pool_stakes.insert(pool(0x44), Coin(1_000));
        acc.epoch_state.snapshots = SnapshotState {
            mark: MarkSnapshot(snap.clone()),
            set: SetSnapshot(snap.clone()),
            go: GoSnapshot(snap),
        };
        acc.prev_block_production.insert(pool(0x44), 11);
        acc.prev_block_production.insert(pool(0x45), 2);
        acc.prev_epoch_fees = Coin(123_456);
        acc.cert_state
            .delegation
            .registrations
            .insert(key_cred(0x33), Coin(2_000_000));
        acc.cert_state
            .delegation
            .delegations
            .insert(key_cred(0x33), pool(0x44));
        acc.cert_state
            .delegation
            .rewards
            .insert(key_cred(0x33), Coin(500_000));
        acc.cert_state
            .pool
            .pools
            .insert(pool(0x44), sample_pool_params(0x44, key_account(0x0B)));
        acc.protocol_params = conway_params();
        acc.gov_state = Some(ConwayGovState {
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
        acc.conway_deposit_params = Some(ConwayOnlyDepositParams {
            drep_deposit: Coin(500_000_000),
            gov_action_deposit: Coin(100_000_000_000),
            drep_activity: 20,
        });
        let mut delta = BTreeMap::new();
        delta.insert(key_cred(0x33), Coin(9_999));
        let commitment = bootstrap_rupd_commitment(
            &Hash32([0x44; 32]),
            SlotNo(163_000_000),
            &Hash32([0x66; 32]),
            EpochNo(1339),
            &delta,
        );
        acc.pending_reward_update = Some(BootstrapRewardUpdate {
            manifest_commitment: Hash32([0x44; 32]),
            source_point_slot: SlotNo(163_000_000),
            source_point_hash: Hash32([0x66; 32]),
            target_epoch: EpochNo(1339),
            reward_delta: delta,
            canonical_commitment: commitment,
        });
        acc
    }

    // ----- S2 seed (PO-3 / CE-2f): EpochAccumulator::seed_from_bootstrap_ledger -----

    #[test]
    fn seed_splits_bootstrapped_epoch_state_into_prev_buffers() {
        // The bootstrapped EpochState carries nesBprev in `block_production` and the (cold-start) fee pot
        // in `epoch_fees`; the seed moves BOTH into the prev_* reward buffers and starts nesBcur fresh.
        let boot = populated();
        let rupd = boot.pending_reward_update.clone();
        let ledger = boot.as_ledger_view();
        let nes_bprev = ledger.epoch_state.block_production.clone();
        let fee_pot = ledger.epoch_state.epoch_fees;
        assert!(
            !nes_bprev.is_empty(),
            "the test ledger must carry a non-empty nesBprev"
        );

        let seed =
            EpochAccumulator::seed_from_bootstrap_ledger(&ledger, EpochNo(1340), rupd.clone())
                .expect("seed");

        // nesBprev + the prior fee pot seed the boundary-consumed reward buffers.
        assert_eq!(seed.prev_block_production, nes_bprev);
        assert_eq!(seed.prev_epoch_fees, fee_pot);
        // nesBcur accumulators start FRESH (the live follow counts from the certified slot forward).
        assert!(seed.epoch_state.block_production.is_empty());
        assert_eq!(seed.epoch_state.epoch_fees, Coin(0));
        // The within-epoch-consumed authority is carried faithfully.
        assert_eq!(seed.epoch_state.epoch, EpochNo(1340));
        assert_eq!(seed.epoch_state.slot, ledger.epoch_state.slot);
        assert_eq!(seed.epoch_state.reserves, ledger.epoch_state.reserves);
        assert_eq!(seed.epoch_state.treasury, ledger.epoch_state.treasury);
        assert_eq!(seed.epoch_state.snapshots, ledger.epoch_state.snapshots);
        assert_eq!(seed.cert_state, ledger.cert_state);
        assert_eq!(seed.protocol_params, ledger.protocol_params);
        assert_eq!(seed.gov_state, ledger.gov_state);
        assert_eq!(seed.conway_deposit_params, ledger.conway_deposit_params);
        assert_eq!(seed.max_lovelace_supply, ledger.max_lovelace_supply);
        assert_eq!(seed.era, CardanoEra::Conway);
        // The bootstrap reward seed is carried verbatim (boundary-consumed; gate-protected until S3).
        assert_eq!(seed.pending_reward_update, rupd);
    }

    #[test]
    fn seed_epoch_mismatch_is_fail_closed() {
        // CE-2f: a seed whose ledger epoch differs from the manifest-declared epoch is REFUSED.
        let ledger = populated().as_ledger_view(); // epoch 1340
        let err = EpochAccumulator::seed_from_bootstrap_ledger(&ledger, EpochNo(1339), None)
            .expect_err("a mis-bound seed epoch must fail closed");
        assert_eq!(
            err,
            SeedError::SeedEpochMismatch {
                ledger_epoch: 1340,
                expected_epoch: 1339,
            }
        );
    }

    #[test]
    fn pre_conway_seed_is_refused() {
        // The accumulator is Conway-scoped; a pre-Conway bootstrap is refused before any field is read.
        let mut ledger = EpochAccumulator::new(CardanoEra::Conway).as_ledger_view();
        ledger.era = CardanoEra::Babbage;
        let err =
            EpochAccumulator::seed_from_bootstrap_ledger(&ledger, ledger.epoch_state.epoch, None)
                .expect_err("pre-Conway must be refused");
        assert!(matches!(
            err,
            SeedError::EraNotSupported { era_tag } if era_tag == CardanoEra::Babbage as u64
        ));
    }

    // ----- Codec -----

    #[test]
    fn codec_round_trips_byte_identical_populated() {
        let acc = populated();
        let bytes = encode_epoch_accumulator(&acc);
        let decoded = decode_epoch_accumulator(&bytes).expect("decode");
        assert_eq!(decoded, acc);
        assert_eq!(encode_epoch_accumulator(&decoded), bytes);
    }

    #[test]
    fn codec_round_trips_empty_conway() {
        let acc = EpochAccumulator::new(CardanoEra::Conway);
        let bytes = encode_epoch_accumulator(&acc);
        let decoded = decode_epoch_accumulator(&bytes).expect("decode");
        assert_eq!(decoded, acc);
    }

    #[test]
    fn codec_encode_is_deterministic() {
        let acc = populated();
        assert_eq!(
            encode_epoch_accumulator(&acc),
            encode_epoch_accumulator(&acc)
        );
    }

    #[test]
    fn codec_rejects_unknown_version() {
        let acc = populated();
        let bytes = encode_epoch_accumulator(&acc);
        // outer array header is one byte; the version uint follows. v1 (=0x01) → patch to 0x02.
        let mut buf = bytes.clone();
        assert_eq!(buf[1], 0x01);
        buf[1] = 0x02;
        match decode_epoch_accumulator(&buf) {
            Err(EpochAccumulatorCodecError::UnknownVersion {
                expected: 1,
                found: 2,
            }) => {}
            other => panic!("expected UnknownVersion, got {other:?}"),
        }
    }

    #[test]
    fn codec_rejects_pre_conway_era() {
        let acc = populated();
        let mut buf = encode_epoch_accumulator(&acc);
        // version (byte 1) then era (byte 2). Conway=7 (0x07) → patch to Shelley=2 (0x02).
        assert_eq!(buf[2], 0x07);
        buf[2] = 0x02;
        match decode_epoch_accumulator(&buf) {
            Err(EpochAccumulatorCodecError::EraNotSupported { era_tag: 2 }) => {}
            other => panic!("expected EraNotSupported, got {other:?}"),
        }
    }

    #[test]
    fn codec_rejects_trailing_bytes() {
        let acc = populated();
        let mut buf = encode_epoch_accumulator(&acc);
        buf.push(0x00);
        match decode_epoch_accumulator(&buf) {
            Err(EpochAccumulatorCodecError::TrailingBytes { extra: 1 }) => {}
            other => panic!("expected TrailingBytes, got {other:?}"),
        }
    }

    #[test]
    fn codec_rejects_non_canonical_via_reencode_backstop() {
        // Re-encode the version uint non-minimally: 0x01 (minimal) → 0x18 0x01 (1-byte-argument uint,
        // same value 1). The value decodes correctly but re-encodes minimally, so the byte-canonical
        // backstop (re-encode != input) rejects it fail-closed — the discipline that a valid value MUST
        // have exactly one accepted encoding.
        let bytes = encode_epoch_accumulator(&populated());
        assert_eq!(bytes[1], 0x01, "version is encoded minimally");
        let mut buf = Vec::with_capacity(bytes.len() + 1);
        buf.push(bytes[0]); // outer array header
        buf.push(0x18); // uint, 1-byte argument follows
        buf.push(0x01); // = 1 (non-minimal)
        buf.extend_from_slice(&bytes[2..]);
        match decode_epoch_accumulator(&buf) {
            Err(EpochAccumulatorCodecError::MalformedCbor) => {}
            other => panic!("expected MalformedCbor (non-canonical), got {other:?}"),
        }
    }

    // ----- Two-buffer block-production rotation (the cardano nesBprev/nesBcur model) -----

    #[test]
    fn boundary_rotates_block_production_two_buffer() {
        let mut acc = EpochAccumulator::new(CardanoEra::Conway);
        acc.epoch_state.epoch = EpochNo(100);
        acc.protocol_params = conway_params();
        acc.epoch_state.reserves = Coin(1_000_000_000_000_000);
        // prev (nesBprev) = the to-be-rewarded counts; cur (nesBcur) = the just-finished epoch.
        acc.prev_block_production.insert(pool(0xAA), 5);
        acc.epoch_state.block_production.insert(pool(0xBB), 3);
        acc.prev_epoch_fees = Coin(111);
        acc.epoch_state.epoch_fees = Coin(222);

        let ctx = SelectedBlockCtx {
            era: CardanoEra::Conway,
            block_epoch: EpochNo(101),
            block_slot: SlotNo(0),
            issuer_pool: pool(0xBB),
            boundary_mark: Some(sample_mark(0xBB, 1_000)),
        };
        let after = cross_epoch_boundary(acc, EpochNo(101), &ctx).expect("boundary");

        // After: nesBprev := the just-finished nesBcur ({0xBB:3}); nesBcur reset to empty.
        assert_eq!(after.epoch_state.epoch, EpochNo(101));
        assert_eq!(after.prev_block_production.get(&pool(0xBB)), Some(&3));
        assert_eq!(after.prev_block_production.get(&pool(0xAA)), None);
        assert!(after.epoch_state.block_production.is_empty());
        assert_eq!(after.prev_epoch_fees, Coin(222));
        assert_eq!(after.epoch_state.epoch_fees, Coin(0));
    }

    // ----- THE ORDER PROPERTY: a within-epoch withdrawal, then a boundary, pays a FRESH reward -----

    fn reward_fixture() -> EpochAccumulator {
        let mut acc = EpochAccumulator::new(CardanoEra::Conway);
        acc.epoch_state.epoch = EpochNo(500);
        acc.protocol_params = conway_params();
        acc.max_lovelace_supply = 45_000_000_000_000_000;
        acc.epoch_state.reserves = Coin(1_000_000_000_000_000);
        // The reward consumes prev_block_production (nesBprev) over the go snapshot.
        acc.prev_block_production.insert(pool(0xAA), 100);
        let stake = 1_000_000_000_000u64;
        let mut snap = StakeSnapshot::new();
        snap.delegations
            .insert(Hash28([0xCC; 28]), (pool(0xAA), Coin(stake)));
        snap.pool_stakes.insert(pool(0xAA), Coin(stake));
        acc.epoch_state.snapshots.go = GoSnapshot(snap);
        // C is registered + delegated to pool AA; the operator account (0x0B) is a different cred.
        acc.cert_state
            .delegation
            .registrations
            .insert(key_cred(0xCC), Coin(2_000_000));
        acc.cert_state
            .delegation
            .delegations
            .insert(key_cred(0xCC), pool(0xAA));
        acc.cert_state
            .pool
            .pools
            .insert(pool(0xAA), sample_pool_params(0xAA, key_account(0x0B)));
        acc
    }

    #[test]
    fn within_epoch_withdrawal_then_boundary_pays_fresh_reward() {
        let mut acc = reward_fixture();
        // C earned a reward in a prior epoch...
        acc.cert_state
            .delegation
            .rewards
            .insert(key_cred(0xCC), Coin(777_000));
        // ...then withdraws it within this epoch (the within-epoch effect): the balance is zeroed.
        if let Some(b) = acc.cert_state.delegation.rewards.get_mut(&key_cred(0xCC)) {
            *b = Coin(0);
        }
        assert_eq!(
            acc.cert_state.delegation.rewards.get(&key_cred(0xCC)),
            Some(&Coin(0)),
            "precondition: the withdrawal zeroed the balance"
        );

        let ctx = SelectedBlockCtx {
            era: CardanoEra::Conway,
            block_epoch: EpochNo(501),
            block_slot: SlotNo(0),
            issuer_pool: pool(0xAA),
            boundary_mark: Some(sample_mark(0xAA, 1_000_000_000_000)),
        };
        let after = cross_epoch_boundary(acc, EpochNo(501), &ctx).expect("boundary");

        // The boundary pays a FRESH member reward — the post-withdrawal zero does not suppress it.
        let bal = after
            .cert_state
            .delegation
            .rewards
            .get(&key_cred(0xCC))
            .copied()
            .unwrap_or(Coin(0));
        assert!(
            bal.0 > 0,
            "the boundary must credit a fresh reward to the within-epoch withdrawer, got {bal:?}"
        );
    }

    // ----- Bootstrap-transient reward seed: applied once at its target boundary, then cleared -----

    #[test]
    fn pending_reward_update_applied_once_then_cleared() {
        let mut acc = reward_fixture();
        acc.epoch_state.epoch = EpochNo(1339);
        let mut delta = BTreeMap::new();
        delta.insert(key_cred(0xCC), Coin(4_242));
        let commitment = bootstrap_rupd_commitment(
            &Hash32([0x01; 32]),
            SlotNo(1),
            &Hash32([0x02; 32]),
            EpochNo(1339),
            &delta,
        );
        acc.pending_reward_update = Some(BootstrapRewardUpdate {
            manifest_commitment: Hash32([0x01; 32]),
            source_point_slot: SlotNo(1),
            source_point_hash: Hash32([0x02; 32]),
            target_epoch: EpochNo(1339),
            reward_delta: delta,
            canonical_commitment: commitment,
        });
        // No prior native reward inputs → the credited balance is the seed delta (+ any native, which
        // is small here). Cross into 1340 (= target_epoch 1339 + 1): the seed applies, then clears.
        acc.prev_block_production.clear(); // suppress the native reward so we isolate the seed delta
        let ctx = SelectedBlockCtx {
            era: CardanoEra::Conway,
            block_epoch: EpochNo(1340),
            block_slot: SlotNo(0),
            issuer_pool: pool(0xAA),
            boundary_mark: Some(sample_mark(0xAA, 1_000_000_000_000)),
        };
        let after = cross_epoch_boundary(acc, EpochNo(1340), &ctx).expect("boundary");
        assert_eq!(
            after.cert_state.delegation.rewards.get(&key_cred(0xCC)),
            Some(&Coin(4_242)),
            "the bootstrap seed delta is credited at its target boundary"
        );
        assert!(
            after.pending_reward_update.is_none(),
            "the bootstrap seed is cleared after a single application"
        );
    }

    // ----- Fail-closed boundaries -----

    #[test]
    fn missing_boundary_stake_is_fail_closed() {
        let acc = reward_fixture();
        let ctx = SelectedBlockCtx {
            era: CardanoEra::Conway,
            block_epoch: EpochNo(501),
            block_slot: SlotNo(0),
            issuer_pool: pool(0xAA),
            boundary_mark: None, // the UTxO-free accumulator cannot recompute the mark
        };
        match cross_epoch_boundary(acc, EpochNo(501), &ctx) {
            Err(LedgerTransitionError::MissingBoundaryStake { epoch: 501 }) => {}
            other => panic!("expected MissingBoundaryStake, got {other:?}"),
        }
    }

    #[test]
    fn boundary_gap_is_fail_closed() {
        let mut acc = EpochAccumulator::new(CardanoEra::Conway);
        acc.epoch_state.epoch = EpochNo(500);
        let ctx = SelectedBlockCtx {
            era: CardanoEra::Conway,
            block_epoch: EpochNo(499), // before the accumulator's epoch
            block_slot: SlotNo(0),
            issuer_pool: pool(0xAA),
            boundary_mark: None,
        };
        match apply_selected_block(&acc, RAW_CONWAY_BLOCK, &ctx) {
            Err(LedgerTransitionError::BoundaryGap {
                prior_epoch: 500,
                block_epoch: 499,
            }) => {}
            other => panic!("expected BoundaryGap, got {other:?}"),
        }
    }

    #[test]
    fn apply_at_max_epoch_is_total_no_overflow() {
        // The boundary loop's `epoch + 1` must be TOTAL on hostile durable state. At prior.epoch ==
        // u64::MAX (block in the same epoch ⇒ no boundary) the within-epoch effects run with no
        // overflow panic / wrap-to-`0..=u64::MAX` (the M1 checked_add fix).
        let mut acc = fresh_conway_acc();
        acc.epoch_state.epoch = EpochNo(u64::MAX);
        let ctx = SelectedBlockCtx {
            era: CardanoEra::Conway,
            block_epoch: EpochNo(u64::MAX), // same epoch — no boundary to cross
            block_slot: SlotNo(1),
            issuer_pool: pool(0x77),
            boundary_mark: None,
        };
        let after = apply_selected_block(&acc, RAW_CONWAY_BLOCK, &ctx).expect("total at max epoch");
        assert_eq!(after.epoch_state.epoch, EpochNo(u64::MAX));
        assert_eq!(
            after.epoch_state.block_production.get(&pool(0x77)),
            Some(&1)
        );
    }

    // ----- Reward-account credential discriminant -----

    #[test]
    fn reward_account_credential_decodes_discriminant() {
        // 0xE0 header → key hash; 0xF0 header → script hash.
        let mut key = vec![0xE0u8];
        key.extend_from_slice(&[0x11; 28]);
        assert_eq!(
            reward_account_credential(&key),
            Some(StakeCredential::KeyHash(Hash28([0x11; 28])))
        );
        let mut script = vec![0xF0u8];
        script.extend_from_slice(&[0x22; 28]);
        assert_eq!(
            reward_account_credential(&script),
            Some(StakeCredential::ScriptHash(Hash28([0x22; 28])))
        );
        // A malformed (wrong-length) account yields None (not a panic / fabricated cred).
        assert_eq!(reward_account_credential(&[0xE0, 0x00]), None);
    }

    // ----- Real-block determinism + replay-equivalence (the S1 acceptance) -----

    fn fresh_conway_acc() -> EpochAccumulator {
        let mut acc = EpochAccumulator::new(CardanoEra::Conway);
        acc.epoch_state.epoch = EpochNo(500);
        acc.protocol_params = conway_params();
        acc.epoch_state.reserves = Coin(1_000_000_000_000_000);
        acc
    }

    #[test]
    fn apply_selected_block_on_real_conway_block_is_deterministic() {
        let acc = fresh_conway_acc();
        let ctx = SelectedBlockCtx {
            era: CardanoEra::Conway,
            block_epoch: EpochNo(500), // same epoch — no boundary
            block_slot: SlotNo(43_000_000),
            issuer_pool: pool(0x77),
            boundary_mark: None,
        };
        let a = apply_selected_block(&acc, RAW_CONWAY_BLOCK, &ctx).expect("apply a");
        let b = apply_selected_block(&acc, RAW_CONWAY_BLOCK, &ctx).expect("apply b");
        assert_eq!(
            encode_epoch_accumulator(&a),
            encode_epoch_accumulator(&b),
            "same prior + same block + same ctx ⇒ byte-identical accumulator"
        );
        // The within-epoch effects landed: the issuer's nesBcur incremented.
        assert_eq!(a.epoch_state.block_production.get(&pool(0x77)), Some(&1));
        assert_eq!(a.epoch_state.slot, SlotNo(43_000_000));
    }

    #[test]
    fn apply_selected_block_credits_exactly_one_fee_scan_per_admit() {
        // CE-2b end-to-end: a within-epoch admit credits `epoch_fees` by EXACTLY one tx-body scan's
        // worth — the same total `scan_block_tx_effects` reports for this block — and increments the
        // issuer's nesBcur by exactly one. The store's slot gate proves a *re-announced* block applies
        // nothing (`at_or_before_tip_is_already_applied`); this proves the per-admit delta is one scan
        // (not two, not drifting), tying the fold to the validity-aware scan that CE-2c pins by value.
        let (_era, block) = decode_selected_block(RAW_CONWAY_BLOCK).expect("decode");
        let invalid =
            decode_invalid_tx_indices_canonical(block.invalid_txs.as_deref(), block.tx_count)
                .expect("canonical invalid set");
        let (one_scan_fees, _w) =
            scan_block_tx_effects(block.tx_count, &block.tx_bodies, &invalid).expect("scan");

        let acc0 = fresh_conway_acc();
        let ctx = SelectedBlockCtx {
            era: CardanoEra::Conway,
            block_epoch: EpochNo(500), // same epoch — within-epoch, no boundary
            block_slot: SlotNo(43_000_000),
            issuer_pool: pool(0x77),
            boundary_mark: None,
        };
        let acc1 = apply_selected_block(&acc0, RAW_CONWAY_BLOCK, &ctx).expect("admit 1");
        assert_eq!(
            acc1.epoch_state.epoch_fees.0 - acc0.epoch_state.epoch_fees.0,
            one_scan_fees,
            "one admit credits exactly one fee scan"
        );
        assert_eq!(acc1.epoch_state.block_production.get(&pool(0x77)), Some(&1));

        // A second admit (next within-epoch slot) adds exactly one MORE scan's worth — each admitted
        // block applies once; the BLUE transition is not self-idempotent (the store's slot gate is what
        // rejects re-applying the SAME block), so the delta must repeat, never double-count.
        let ctx2 = SelectedBlockCtx {
            era: CardanoEra::Conway,
            block_epoch: EpochNo(500),
            block_slot: SlotNo(43_000_001),
            issuer_pool: pool(0x77),
            boundary_mark: None,
        };
        let acc2 = apply_selected_block(&acc1, RAW_CONWAY_BLOCK, &ctx2).expect("admit 2");
        assert_eq!(
            acc2.epoch_state.epoch_fees.0 - acc1.epoch_state.epoch_fees.0,
            one_scan_fees,
            "each admitted block adds exactly one scan — no double-count, no drift"
        );
        assert_eq!(acc2.epoch_state.block_production.get(&pool(0x77)), Some(&2));
    }

    #[test]
    fn replay_equivalence_via_durable_checkpoint_across_a_boundary() {
        // Fold [block@E, block@E+1(boundary)]. Folding from the start must equal folding from the
        // durable checkpoint persisted after block 1 — replay-equivalence IS the recovery mechanism.
        let acc0 = fresh_conway_acc();
        let ctx_e = SelectedBlockCtx {
            era: CardanoEra::Conway,
            block_epoch: EpochNo(500),
            block_slot: SlotNo(43_000_000),
            issuer_pool: pool(0x77),
            boundary_mark: None,
        };
        let ctx_e1 = SelectedBlockCtx {
            era: CardanoEra::Conway,
            block_epoch: EpochNo(501), // crosses a boundary
            block_slot: SlotNo(43_100_000),
            issuer_pool: pool(0x88),
            boundary_mark: Some(sample_mark(0x77, 1_000_000_000_000)),
        };

        let acc1 = apply_selected_block(&acc0, RAW_CONWAY_BLOCK, &ctx_e).expect("block 1");
        // Persist + recover at the checkpoint.
        let persisted = encode_epoch_accumulator(&acc1);
        let acc1_recovered = decode_epoch_accumulator(&persisted).expect("recover");
        assert_eq!(acc1_recovered, acc1);

        let from_start =
            apply_selected_block(&acc1, RAW_CONWAY_BLOCK, &ctx_e1).expect("from start");
        let from_ckpt = apply_selected_block(&acc1_recovered, RAW_CONWAY_BLOCK, &ctx_e1)
            .expect("from checkpoint");
        assert_eq!(
            encode_epoch_accumulator(&from_start),
            encode_epoch_accumulator(&from_ckpt),
            "folding from the durable checkpoint must be byte-identical to folding from the start"
        );
        // The boundary actually fired: the epoch advanced and nesBprev rotated in the issuer's count.
        assert_eq!(from_start.epoch_state.epoch, EpochNo(501));
        assert_eq!(from_start.prev_block_production.get(&pool(0x77)), Some(&1));
    }

    #[test]
    fn scan_block_tx_effects_does_not_error_on_real_block() {
        // The real Conway block scan extracts fees + withdrawals without error (the byte-level wiring),
        // gated by the block's real invalid_transactions set decoded through the FAIL-CLOSED canonical
        // decoder (mirrors the live within-epoch path: a real cardano-node block is canonical).
        let (era, block) = decode_selected_block(RAW_CONWAY_BLOCK).expect("decode");
        assert_eq!(era, CardanoEra::Conway);
        let invalid =
            decode_invalid_tx_indices_canonical(block.invalid_txs.as_deref(), block.tx_count)
                .expect("real block invalid_transactions is canonical");
        let (_fees, _withdrawals) =
            scan_block_tx_effects(block.tx_count, &block.tx_bodies, &invalid).expect("scan");
    }

    // ----- Phase-2 validity gate: invalid-tx fee = collateral; body effects fail-closed (PO-1) -----

    /// CBOR uint (canonical minimal width) for the small values used in these tx-body fixtures.
    fn cbor_uint(v: u64) -> Vec<u8> {
        let mut b = Vec::new();
        ade_codec::cbor::write_uint_canonical(&mut b, v);
        b
    }

    /// One tx-body map `{2: fee}` (a plain valid tx) as canonical CBOR.
    fn tx_body_fee(fee: u64) -> Vec<u8> {
        let mut b = vec![0xA1]; // map(1)
        b.extend(cbor_uint(2));
        b.extend(cbor_uint(fee));
        b
    }

    /// One tx-body map `{2: fee, 17: total_collateral}` (a phase-2-invalid tx that declares collateral).
    fn tx_body_fee_collateral(fee: u64, collateral: u64) -> Vec<u8> {
        let mut b = vec![0xA2]; // map(2), canonical key order 2 < 17
        b.extend(cbor_uint(2));
        b.extend(cbor_uint(fee));
        b.extend(cbor_uint(17));
        b.extend(cbor_uint(collateral));
        b
    }

    /// One tx-body map `{2: fee, 4: []}` (carries an empty certs field).
    fn tx_body_fee_certs(fee: u64) -> Vec<u8> {
        let mut b = vec![0xA2]; // map(2), canonical key order 2 < 4
        b.extend(cbor_uint(2));
        b.extend(cbor_uint(fee));
        b.extend(cbor_uint(4));
        b.push(0x80); // empty array (certs present)
        b
    }

    /// Wrap a single tx body in the `tx_bodies` array(1).
    fn tx_bodies_one(body: Vec<u8>) -> Vec<u8> {
        let mut b = vec![0x81]; // array(1)
        b.extend(body);
        b
    }

    fn invalid_set(indices: &[u64]) -> std::collections::BTreeSet<u64> {
        indices.iter().copied().collect()
    }

    #[test]
    fn invalid_tx_fee_is_collateral_not_declared_fee() {
        let bodies = tx_bodies_one(tx_body_fee_collateral(50, 200));
        // VALID (empty invalid set): the declared fee (50) is credited.
        let (fees_valid, _) =
            scan_block_tx_effects(1, &bodies, &invalid_set(&[])).expect("valid scan");
        assert_eq!(fees_valid, 50, "a valid tx credits its declared fee");
        // INVALID (index 0): the consumed collateral (200), NOT the declared fee, is credited.
        let (fees_invalid, _) =
            scan_block_tx_effects(1, &bodies, &invalid_set(&[0])).expect("invalid scan");
        assert_eq!(
            fees_invalid, 200,
            "a phase-2-invalid tx credits its collateral, not its declared fee"
        );
    }

    #[test]
    fn invalid_tx_without_total_collateral_is_fail_closed() {
        // An invalid tx with no key-17 total_collateral: the consumed collateral needs the UTxO, which the
        // accumulator does not have → fail-closed rather than credit a knowingly-wrong fee.
        let bodies = tx_bodies_one(tx_body_fee(50));
        let err = scan_block_tx_effects(1, &bodies, &invalid_set(&[0])).unwrap_err();
        assert!(
            matches!(
                err,
                LedgerTransitionError::InvalidTxCollateralNeedsUtxo { tx_index: 0 }
            ),
            "expected InvalidTxCollateralNeedsUtxo, got {err:?}"
        );
    }

    #[test]
    fn invalid_tx_carrying_certs_is_fail_closed() {
        // cardano discards an invalid tx's certs; the within-epoch cert path does not yet skip them, so
        // rather than silently apply a discarded cert, the transition fail-closes (the skip is S3's gate).
        let bodies = tx_bodies_one(tx_body_fee_certs(50));
        let err = scan_block_tx_effects(1, &bodies, &invalid_set(&[0])).unwrap_err();
        assert!(
            matches!(
                err,
                LedgerTransitionError::InvalidTxCarriesAuthorityEffect { tx_index: 0 }
            ),
            "expected InvalidTxCarriesAuthorityEffect, got {err:?}"
        );
        // The SAME block with the tx VALID is fine: the certs are process_block_certificates' job, and the
        // scan credits the declared fee.
        let (fees, _) = scan_block_tx_effects(1, &bodies, &invalid_set(&[])).expect("valid scan");
        assert_eq!(fees, 50);
    }

    #[test]
    fn valid_tx_fee_is_declared_fee_regression() {
        // Two valid txs: Σ declared fees, no withdrawals.
        let mut bodies = vec![0x82]; // array(2)
        bodies.extend(tx_body_fee(100));
        bodies.extend(tx_body_fee(25));
        let (fees, withdrawals) =
            scan_block_tx_effects(2, &bodies, &invalid_set(&[])).expect("scan");
        assert_eq!(fees, 125);
        assert!(withdrawals.is_empty());
    }

    // ----- Fail-closed canonical decode of invalid_transactions (the review HIGH: a fail-open set would
    // silently under-report and apply a discarded tx's effects to authoritative state) -----

    #[test]
    fn canonical_invalid_set_absent_and_empty_are_empty() {
        assert!(decode_invalid_tx_indices_canonical(None, 3)
            .expect("absent is the empty set")
            .is_empty());
        // A present, empty definite array (0x80) is the empty set.
        assert!(decode_invalid_tx_indices_canonical(Some(&[0x80]), 3)
            .expect("empty array is the empty set")
            .is_empty());
    }

    #[test]
    fn canonical_invalid_set_valid_sorted_in_range() {
        // array [0, 2], tx_count 3 → {0, 2}.
        let set = decode_invalid_tx_indices_canonical(Some(&[0x82, 0x00, 0x02]), 3).expect("ok");
        assert_eq!(set, invalid_set(&[0, 2]));
    }

    #[test]
    fn canonical_invalid_set_fail_closed_on_non_canonical() {
        // Every case that the lenient diagnostic decoder would silently accept / truncate must HALT here.
        let bad: &[(&[u8], &str)] = &[
            (&[0x05], "a bare uint, not an array"),
            (&[0x40], "a bstr, not an array"),
            (&[0x9f, 0x00, 0xff], "indefinite array"),
            (
                &[0x81, 0x40],
                "non-uint entry (empty bstr) — lenient decoder would skip it",
            ),
            (
                &[0x81, 0x18, 0x05],
                "non-minimal uint (5 encoded as 0x18 0x05)",
            ),
            (&[0x82, 0x01, 0x00], "unsorted [1, 0]"),
            (&[0x82, 0x00, 0x00], "duplicate [0, 0]"),
            (&[0x81, 0x00, 0xff], "trailing bytes after [0]"),
        ];
        for (bytes, why) in bad {
            let r = decode_invalid_tx_indices_canonical(Some(bytes), 8);
            assert!(
                matches!(r, Err(LedgerTransitionError::MalformedBlock)),
                "expected MalformedBlock for {why}, got {r:?}"
            );
        }
        // An index at/after the last tx (here 5 with tx_count 3) is a malformed-block signal, not a silent
        // under-trigger.
        let r = decode_invalid_tx_indices_canonical(Some(&[0x81, 0x05]), 3);
        assert!(
            matches!(r, Err(LedgerTransitionError::MalformedBlock)),
            "expected MalformedBlock for an out-of-range index, got {r:?}"
        );
    }
}
