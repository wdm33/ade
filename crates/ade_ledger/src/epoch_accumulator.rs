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
//! reward, the pots, `epoch::rotate_snapshots`, and the single canonical POOLREAP — future-pool
//! adoption, reap, deposit refund, delegation-clear) over a transient UTxO-free `LedgerState` view; the
//! within-epoch cert/governance half reuses `rules::process_block_certificates`; the bootstrap-transient
//! reward seed reuses `delegation::apply_bootstrap_reward_deltas`. The contract is the deterministic
//! orchestration of these single-authority primitives over the accumulator's non-UTxO state — the new
//! stake for the boundary mark comes from `ctx` (the reduced-checkpoint per-credential base-UTxO stake,
//! built into the mark with the held delegation state), never a full UTxO map.
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
//!   UTxO stake. A boundary's new MARK is BUILT from `ctx.boundary_mark` (the per-credential base-UTxO
//!   stake, `sum_base_credential_stake` over the reduced checkpoint at the prior tip) plus the held
//!   `cert_state.delegation`; the accumulator never holds the UTxO set or recomputes stake from it.
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
    bootstrap_rupd_commitment, decode_bootstrap_reward_update, encode_bootstrap_reward_update,
    BootstrapRewardUpdate, BootstrapRupdError,
};
use crate::delegation::{apply_bootstrap_reward_deltas, CertState};
use crate::error::LedgerError;
use crate::pparams::{ConwayOnlyDepositParams, ProtocolParameters};
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
        current_block_production: BTreeMap<PoolId, u64>,
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

        // The bootstrapped epoch-state's block_production is nesBprev (epoch (seed-1)'s blocks) → the prev_*
        // buffer; nesBcur (the current epoch's blocks-so-far at the snapshot point) seeds
        // `epoch_state.block_production`, so the seed→seed+1 boundary counts the whole seed epoch, not just
        // the anchor+1 replayed tail.
        //
        // FEES are asymmetric to blocks: the certified snapshot carries ONE fee pot (epoch (seed)'s fees
        // accrued so far) — the nesBcur-analog, NOT a nesBprev. It STAYS in `epoch_state.epoch_fees` (the
        // live follow adds the seed-to-end tail) so the FULL epoch (seed) fees rotate to nesBprev at the
        // seed boundary and the first native boundary draws them. `prev_epoch_fees` is 0: epoch (seed-1)'s
        // fees are NOT in the snapshot — they ride in the one-shot bootstrap RUPD (which owns the seed
        // boundary's reward), so there is no native nesBprev fee to seed.
        let mut epoch_state = ledger.epoch_state.clone();
        let prev_block_production = std::mem::take(&mut epoch_state.block_production);
        let prev_epoch_fees = Coin(0);
        epoch_state.block_production = current_block_production;

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
    /// The per-credential BASE UTxO stake for a crossed boundary (live: the advancer supplies
    /// `reduced_utxo_checkpoint::sum_base_credential_stake()` over the reduced checkpoint at the prior
    /// tip — item #2b). `cross_epoch_boundary` BUILDS the new MARK snapshot from it plus the held
    /// `cert_state.delegation`, RETAINING the per-credential `delegations` the reward computation reads
    /// (the operator's `op_stake` and each member's pro-rata share); a per-pool mark dropped those, so
    /// member rewards went to zero once it rotated into `go`. `None` is a fail-closed boundary error —
    /// the accumulator, being UTxO-free, has no way to recompute the stake, so a boundary REQUIRES it.
    /// For multiple crossed boundaries (the degenerate empty-epoch case) the same prior-tip stake
    /// applies (no intervening change).
    pub boundary_mark: Option<BTreeMap<StakeCredential, Coin>>,
    /// The network's expected block-producing slots per epoch = `epochLength × activeSlotCoeff`
    /// (preview 86_400 × 1/20 = 4_320; mainnet/preprod 432_000 × 1/20 = 21_600). Feeds the boundary
    /// reward update's monetary-expansion performance factor `eta = min(1, blocksMade / floor((1-d) ×
    /// this))`. The advancer derives it from the era schedule's REAL per-era epoch length, so
    /// expansion is correct on every network (a mainnet constant here under-expanded preview 5×).
    /// Consumed ONLY on a boundary cross; on the within-epoch path (`boundary_mark = None`) a crossing
    /// fail-closes before this is read, so that path carries `0`.
    pub active_slots_per_epoch: u64,
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
    /// A pending one-shot bootstrap RUPD reached a boundary that is NOT its target (target_epoch+1). The
    /// bootstrap exception applies at EXACTLY the seed→seed+1 boundary; at any other boundary it must be
    /// already-consumed-None. Fail closed rather than carry a stale bootstrap reward into a later epoch.
    BootstrapRupdWrongBoundary { rupd_target: u64, boundary: u64 },
    /// The durable bootstrap RUPD's recomputed commitment did not match its bound `canonical_commitment`
    /// (tampered or corrupt durable bytes). Fail closed before applying the pots.
    BootstrapRupdCommitmentMismatch,
    /// Applying the bootstrap RUPD's reserves draw underflowed (delta_reserves > reserves). Fail closed.
    BootstrapRupdReservesUnderflow,
    /// CONWAY-PROPOSAL-DEPOSIT-EXPIRY S3: a tx-body governance field (19 `voting_procedures` / 20
    /// `proposal_procedures`) failed to decode, or a field-20 procedure carried an unknown gov-action
    /// variant, on the within-epoch authority path. Terminal for the governance authority — never a silent
    /// skip or empty default (an unknown variant is `MalformedGovernanceField`, not a dropped proposal).
    /// `tx_index` locates the offending tx.
    MalformedGovernanceField { tx_index: u64 },
    /// CONWAY-PROPOSAL-DEPOSIT-EXPIRY S3 / DC-GOV-01: a tx submitted a governance proposal (field 20) but
    /// the accumulator's imported `gov_action_lifetime` is 0 — the placeholder / un-imported value,
    /// impossible on any real network (govActionLifetime ≥ 1). A proposal's `expires_after = proposed_in +
    /// gov_action_lifetime` is PERSISTED future refund authority; it must never be derived from an unproven
    /// parameter. Fail closed rather than track a proposal with a fabricated (`proposed_in`) expiry. A
    /// properly v7-bootstrapped accumulator imports the era-correct lifetime from the certified curPParams.
    /// `tx_index` locates the submitting tx.
    GovActionLifetimeUnproven { tx_index: u64 },
    /// CONWAY-PROPOSAL-DEPOSIT-EXPIRY S4 / DC-GOV-01: the epoch-boundary deposit-expiry-refund planner
    /// could not prove that EVERY tracked proposal is safe to resolve under Ade's authority — at least one
    /// proposal is potentially ratifiable, malformed, or unsupported. The boundary fails closed with ZERO
    /// mutation (no refund credited, no proposal removed) rather than refund past an unproven disposition.
    /// Carries the structured verdict that tripped it.
    GovDepositRefundTerminal(crate::governance::RefundVerdict),
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

/// Apply the CONWAY-PROPOSAL-DEPOSIT-EXPIRY S4 deposit-expiry refunds at an epoch boundary into `target`
/// (DC-GOV-01). Builds the canonical ratification inputs from the accumulator's governance state + the
/// current (pre-rotation) snapshots, plans the whole-set refund (`governance::plan_deposit_refunds`), and —
/// ONLY on a fully-safe plan — credits each refunded deposit to its return-address reward account and
/// removes the proposal, in `GovActionId` order. A non-`ProvablyUnratifiable` proposal anywhere returns the
/// structured terminal `GovDepositRefundTerminal` with ZERO mutation. Governance-untracked (`None`) is a
/// no-op. The deposit pot is IMPLICIT: removal IS the debit, so Σcredits == Σremoved deposits by construction.
fn apply_gov_deposit_refunds(
    acc: &mut EpochAccumulator,
    target: EpochNo,
) -> Result<(), LedgerTransitionError> {
    // Plan first (immutable borrows of gov_state + snapshots); mutate `acc` only on a clean plan.
    let plan = {
        let gov = match acc.gov_state.as_ref() {
            Some(g) => g,
            None => return Ok(()), // governance not tracked
        };
        // Active DRep stake from the imported vote delegations × the current mark snapshot (mirrors the
        // boundary fn's step 4b). For the accumulator `vote_delegations` is empty ⇒ empty, so the committee
        // gate is the proof (per the S4.0 census).
        let mark = &acc.epoch_state.snapshots.mark;
        let mut drep_stake: crate::governance::DRepStakeDistribution = BTreeMap::new();
        for (cred, drep) in &gov.vote_delegations {
            let stake = mark.0.delegations.get(cred.hash()).map(|(_, c)| c.0).unwrap_or(0);
            if stake > 0 {
                *drep_stake.entry(drep.clone()).or_insert(0) += stake;
            }
        }
        let committee_quorum = crate::rational::Rational::new(
            gov.committee_quorum.0 as i128,
            gov.committee_quorum.1.max(1) as i128,
        )
        .unwrap_or_else(crate::rational::Rational::one);
        crate::governance::plan_deposit_refunds(
            &gov.proposals,
            &drep_stake,
            &acc.epoch_state.snapshots.go.0.pool_stakes,
            &gov.committee,
            &committee_quorum,
            &gov.pool_voting_thresholds,
            &gov.drep_voting_thresholds,
            target.0,
            &gov.committee_hot_keys,
            &gov.drep_expiry,
        )
        .map_err(LedgerTransitionError::GovDepositRefundTerminal)?
    };

    // Apply atomically — nothing above mutated `acc` (a terminal returned with zero mutation). The refund
    // routes EXACTLY as POOLREAP (rules.rs) and reward distribution do: a REGISTERED return-address
    // credential is credited to its reward account; a DEREGISTERED one goes to the TREASURY (cardano's
    // unredeemed path) — never an orphan `rewards` entry (the codebase-wide invariant "every rewards key is
    // registered"). Treasury is credited before the boundary view, so the boundary fn's `treasury + deltas`
    // carries it. GovActionId order (the planner already sorted `removed`).
    for entry in &plan.removed {
        if let Some((cred, deposit)) = &entry.credit {
            if acc.cert_state.delegation.registrations.contains_key(cred) {
                let bal = acc
                    .cert_state
                    .delegation
                    .rewards
                    .entry(cred.clone())
                    .or_insert(Coin(0));
                bal.0 = bal.0.saturating_add(deposit.0);
            } else {
                acc.epoch_state.treasury.0 =
                    acc.epoch_state.treasury.0.saturating_add(deposit.0);
            }
        }
    }
    if !plan.removed.is_empty() {
        if let Some(gov) = acc.gov_state.as_mut() {
            let removed: std::collections::BTreeSet<_> =
                plan.removed.iter().map(|e| e.action_id.clone()).collect();
            gov.proposals.retain(|p| !removed.contains(&p.action_id));
        }
    }
    Ok(())
}

/// Cross ONE epoch boundary into `target`. Reuses the validated `apply_epoch_boundary_with_registrations`
/// for the reward + pots + snapshot rotation, feeding it the held `prev_block_production`/`prev_epoch_fees`
/// (the `nesBprev` reward inputs), then rotates `prev := <just-finished nesBcur>`, `cur := ∅`. The new
/// MARK is BUILT (`build_boundary_mark_snapshot`) from `ctx.boundary_mark` (the per-credential base-UTxO
/// stake) plus the held delegation state — fail-closed if `ctx.boundary_mark` is absent.
pub fn cross_epoch_boundary(
    mut acc: EpochAccumulator,
    target: EpochNo,
    ctx: &SelectedBlockCtx,
) -> Result<EpochAccumulator, LedgerTransitionError> {
    let base_utxo = ctx
        .boundary_mark
        .as_ref()
        .ok_or(LedgerTransitionError::MissingBoundaryStake { epoch: target.0 })?;

    // The eta denominator (`active_slots_per_epoch = epochLength × activeSlotCoeff`) is a REQUIRED
    // canonical boundary input. It is 0 ONLY on the within-epoch sentinel — which already fail-closed
    // just above on the absent mark — so a 0 reaching a real cross is a miswire. Halt deterministically
    // rather than absorb it: `active_slots_per_epoch = 0` would make the reward calc's
    // `expected_blocks = max(1, floor((1-d)·0)) = 1` -> `eta = 1` (FULL monetary expansion), silently
    // over-drawing reserves. (IDD §2/§8: the field's validity is coupled to the mark; a follow-up may
    // group them into a single boundary-only sum-type variant so this is unrepresentable.)
    if ctx.active_slots_per_epoch == 0 {
        return Err(LedgerTransitionError::MissingBoundaryStake { epoch: target.0 });
    }

    // ONE-SHOT BOOTSTRAP RUPD (DC-EPOCH-18 / CE-3d). The law: native is the SOLE reward authority once
    // bootstrap-derived inputs are exhausted; only the FIRST bootstrap-adjacent transition (the seed→
    // seed+1 boundary) may consume manifest-bound historical state from the certified snapshot. That
    // boundary pays epoch (seed-1)'s rewards, which need epoch (seed-1)'s FEES — PRE-SEED, and
    // unreconstructable from post-bootstrap blocks (proven: with fees=0 the native pot under-draws by
    // exactly the snapshot `deltaF`). So here the native reward is forced to ZERO (empty block-production
    // + zero fees → no reserves draw, R=0, the pool loop is skipped; rotation/POOLREAP/enactment still
    // run) and the bound bootstrap RUPD supplies cardano's EXACT reward (pots + rs) EXACTLY ONCE, then is
    // consumed. A pending RUPD at any NON-target boundary fails closed (it may never carry into a later
    // epoch). After consumption every later boundary is native-only.
    let is_seed_boundary = matches!(
        &acc.pending_reward_update,
        Some(rupd) if rupd.target_epoch.0.checked_add(1) == Some(target.0)
    );
    if let Some(rupd) = &acc.pending_reward_update {
        if !is_seed_boundary {
            return Err(LedgerTransitionError::BootstrapRupdWrongBoundary {
                rupd_target: rupd.target_epoch.0,
                boundary: target.0,
            });
        }
    }

    // Build the new PER-CREDENTIAL mark from the supplied base-UTxO stake + the held (post-seed)
    // delegation state. The boundary fn consumes it DIRECTLY as the new mark, so its per-credential
    // `delegations` survive into `go` and pay member rewards two boundaries later.
    let new_mark = build_boundary_mark_snapshot(base_utxo, &acc.cert_state.delegation);

    // Capture the just-finished epoch's nesBcur (it becomes nesBprev after this boundary).
    let finished_blocks = std::mem::take(&mut acc.epoch_state.block_production);
    let finished_fees = acc.epoch_state.epoch_fees;

    // CONWAY-PROPOSAL-DEPOSIT-EXPIRY S4 (DC-GOV-01): the deposit-expiry-refund transition, BEFORE the
    // boundary view. The whole-set planner refunds each EXPIRED + provably-unratifiable proposal's deposit
    // to its return address and removes it; any potentially-ratifiable / malformed / unsupported proposal
    // fails the boundary closed with ZERO mutation. Removing the expired proposals here means the boundary
    // fn's own governance pass (which would otherwise silently DROP the expired set) sees a reduced set and
    // is a no-op for them, and the refund credits flow into the new epoch's snapshot.
    apply_gov_deposit_refunds(&mut acc, target)?;

    // Reward inputs: at the seed boundary, EMPTY (the bootstrap RUPD carries the reward, so the native
    // reward computes exactly zero); otherwise the held nesBprev + prev fees.
    let mut view = acc.as_ledger_view();
    if is_seed_boundary {
        view.epoch_state.block_production = BTreeMap::new();
        view.epoch_state.epoch_fees = Coin(0);
    } else {
        view.epoch_state.block_production = acc.prev_block_production.clone();
        view.epoch_state.epoch_fees = acc.prev_epoch_fees;
    }

    let (new_view, _accounting) = apply_epoch_boundary_with_registrations(
        &view,
        target,
        None,
        Some(&new_mark),
        ctx.active_slots_per_epoch,
    );

    // Read back. `new_view.epoch_state` already has epoch=target, rotated snapshots, updated pots, and
    // block_production/epoch_fees reset to empty/0 (the new epoch's fresh nesBcur). POOLREAP — future-pool
    // adoption, reap (== target), deposit refund, and delegation-clear — now runs INSIDE the boundary fn
    // as the single canonical order, so there is no trailing reap to compose here.
    acc.epoch_state = new_view.epoch_state;
    acc.cert_state = new_view.cert_state;
    acc.gov_state = new_view.gov_state;
    // Rotate the block-production buffers: nesBprev := the just-finished nesBcur.
    acc.prev_block_production = finished_blocks;
    acc.prev_epoch_fees = finished_fees;

    // Apply the one-shot bootstrap RUPD exactly once at the seed boundary, AFTER the native mechanics +
    // zero reward, then consume (None = the durable "consumed" record; the accumulator is persisted).
    // Verify the durable commitment first — fail closed on a tampered/corrupt record.
    if is_seed_boundary {
        let rupd = acc
            .pending_reward_update
            .take()
            .expect("is_seed_boundary implies pending_reward_update is Some");
        let recomputed = bootstrap_rupd_commitment(
            &rupd.manifest_commitment,
            rupd.source_point_slot,
            &rupd.source_point_hash,
            rupd.target_epoch,
            rupd.delta_treasury,
            rupd.delta_reserves,
            &rupd.reward_delta,
        );
        if recomputed != rupd.canonical_commitment {
            return Err(LedgerTransitionError::BootstrapRupdCommitmentMismatch);
        }
        acc.epoch_state.reserves.0 = acc
            .epoch_state
            .reserves
            .0
            .checked_sub(rupd.delta_reserves.0)
            .ok_or(LedgerTransitionError::BootstrapRupdReservesUnderflow)?;
        acc.epoch_state.treasury.0 = acc
            .epoch_state
            .treasury
            .0
            .checked_add(rupd.delta_treasury.0)
            .ok_or(LedgerTransitionError::ArithmeticOverflow)?;
        apply_bootstrap_reward_deltas(&mut acc.cert_state.delegation, &rupd.reward_delta)
            .map_err(|_| LedgerTransitionError::ArithmeticOverflow)?;
    }
    Ok(acc)
}

/// Build the new PER-CREDENTIAL mark [`crate::epoch::StakeSnapshot`] for an epoch boundary from the
/// canonical per-credential BASE UTxO stake (`ctx.boundary_mark`, what
/// `reduced_utxo_checkpoint::sum_base_credential_stake` returns) and the held delegation state. For each
/// registered+delegated credential its instant stake is its base-UTxO coin + its reward-account balance,
/// grouped by its delegated pool — the same inputs `reduced_aggregate::aggregate_pool_stake` consumes,
/// but RETAINING the per-credential `delegations` the reward computation reads (the operator's `op_stake`
/// and each member's pro-rata share). A per-pool mark dropped `delegations`, leaving an empty `go` two
/// boundaries later → zero member rewards. Pure, deterministic (`BTreeMap`), saturating on the supply-
/// bounded stake sums.
fn build_boundary_mark_snapshot(
    base_utxo: &BTreeMap<StakeCredential, Coin>,
    delegation: &crate::delegation::DelegationState,
) -> crate::epoch::StakeSnapshot {
    let mut delegations = BTreeMap::new();
    let mut pool_stakes: BTreeMap<PoolId, Coin> = BTreeMap::new();
    for (cred, pool) in &delegation.delegations {
        let base = base_utxo.get(cred).copied().unwrap_or(Coin(0)).0;
        let reward = delegation.rewards.get(cred).copied().unwrap_or(Coin(0)).0;
        let stake = base.saturating_add(reward);
        delegations.insert(cred.hash().clone(), (pool.clone(), Coin(stake)));
        let entry = pool_stakes.entry(pool.clone()).or_insert(Coin(0));
        entry.0 = entry.0.saturating_add(stake);
    }
    crate::epoch::StakeSnapshot {
        delegations,
        pool_stakes,
    }
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

    // Governance proposals (tx-body field 20) + the vote tripwire (field 19) — CONWAY-PROPOSAL-DEPOSIT-
    // EXPIRY S3 / DC-GOV-01. When governance is tracked (`gov_state` present) this captures each newly
    // submitted proposal's identity (GovActionId = tx-id ‖ procedure index), deposit, return address, and
    // exact expiry, and fail-closes if any selected-chain vote targets a tracked proposal. When governance
    // is not tracked (`None`) the within-epoch governance half is skipped — the same gating
    // `process_block_certificates` applies. Reuses `invalid` (the phase-2-invalid set the fee scan already
    // decoded) as the authority-effect gate, paralleling the cert/withdrawal guard.
    acc.gov_state = match gov_state {
        Some(gov) => Some(apply_block_governance(
            gov,
            &block.tx_bodies,
            block.tx_count,
            &invalid,
            ctx.block_epoch,
        )?),
        None => None,
    };

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
/// set ⇒ script. Returns `None` for a malformed (≠ 29-byte) account. Shared with the boundary
/// POOLREAP refund (`rules::apply_epoch_boundary_with_registrations`) so a script-hash reward account
/// routes by its real discriminant, never a KeyHash projection.
pub(crate) fn reward_account_credential(account: &[u8]) -> Option<StakeCredential> {
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
// Within-epoch governance capture (CONWAY-PROPOSAL-DEPOSIT-EXPIRY S3 / DC-GOV-01)
// ---------------------------------------------------------------------------

/// Capture this block's governance proposals (tx-body field 20) into `gov.proposals` and CAPTURE this
/// block's votes (field 19) into the tracked proposals' vote maps (CRE S2) — the INPUT half of DC-GOV-01.
///
/// A dedicated tx-body walk (mirroring `rules::process_block_certificates`, the cert authority's own
/// walk) rather than an extension of the fee scan: forming a live proposal's `GovActionId` needs the tx
/// body hash, the current epoch, and the `gov_state` to merge into — none of which the fee scan's
/// `TxScan` carries. The fee scan and the SHARED `rules.rs` authority are left untouched.
///
/// For every VALID tx (phase-2-valid), in tx order:
///   1. field 20 (`proposal_procedures`, via the typed closed decoder): each procedure becomes a tracked
///      `GovActionState` whose `action_id = (transaction_id(tx_body) ‖ procedure_index)`, with the
///      submitter's deposit / return address / gov-action verbatim, `proposed_in = current_epoch`,
///      `expires_after = proposed_in + gov_action_lifetime`, and EMPTY vote maps (populated by the
///      field-19 capture below).
///   2. field 19 (`voting_procedures`): each `(voter, gov_action_id, vote)` is applied to the tracked
///      proposal's committee/DRep/SPO vote map (`apply_field19_votes`, CRE S2 — replaces the CPDE S3
///      detect-and-halt tripwire). A vote on an untracked proposal is ignored; a re-vote by the same voter
///      replaces its prior entry; a vote on a proposal this same tx just submitted is applied too (1 runs
///      first). This is CAPTURE, not ratification — the DRep/SPO ratify gates stay inert until CRE S4.
///
/// A phase-2-invalid tx that carries field 19/20 fail-closes (`InvalidTxCarriesAuthorityEffect`), parity
/// with the fee scan's cert/withdrawal guard (cardano discards an invalid tx's body effects; rather than
/// selectively skip a discarded proposal, halt). A malformed field 19/20, an unknown gov-action variant,
/// or an unknown voter/vote discriminant on a tracked proposal is terminal (`MalformedGovernanceField`) —
/// never a silent skip. Pure + total; replaying the same block yields the same `gov`.
fn apply_block_governance(
    mut gov: ConwayGovState,
    tx_bodies: &[u8],
    tx_count: u64,
    invalid: &std::collections::BTreeSet<u64>,
    current_epoch: EpochNo,
) -> Result<ConwayGovState, LedgerTransitionError> {
    use ade_codec::cbor;
    if tx_count == 0 {
        return Ok(gov);
    }
    let data = tx_bodies;
    let mut offset = 0usize;
    let mut tx_index: u64 = 0;
    match cbor::read_array_header(data, &mut offset)
        .map_err(|_| LedgerTransitionError::MalformedBlock)?
    {
        cbor::ContainerEncoding::Definite(n, _) => {
            for _ in 0..n {
                apply_one_tx_governance(
                    &mut gov,
                    data,
                    &mut offset,
                    tx_index,
                    invalid,
                    current_epoch,
                )?;
                tx_index += 1;
            }
        }
        cbor::ContainerEncoding::Indefinite => {
            while !cbor::is_break(data, offset).map_err(|_| LedgerTransitionError::MalformedBlock)? {
                apply_one_tx_governance(
                    &mut gov,
                    data,
                    &mut offset,
                    tx_index,
                    invalid,
                    current_epoch,
                )?;
                tx_index += 1;
            }
        }
    }
    // Mechanical coupling: the tx index space walked here must equal `tx_count` (the count the invalid set
    // was range-checked against), so the per-tx validity gate aligns. Equal by construction — the same
    // durable bytes the fee scan + cert pass already walked — but asserted so a future miswire fails closed.
    if tx_index != tx_count {
        return Err(LedgerTransitionError::MalformedBlock);
    }
    Ok(gov)
}

/// Apply ONE tx's governance effects (field 20 capture, field 19 tripwire) gated by phase-2 validity.
fn apply_one_tx_governance(
    gov: &mut ConwayGovState,
    data: &[u8],
    offset: &mut usize,
    tx_index: u64,
    invalid: &std::collections::BTreeSet<u64>,
    current_epoch: EpochNo,
) -> Result<(), LedgerTransitionError> {
    use ade_codec::cbor;
    use ade_types::conway::governance::{GovActionId, GovActionState};

    let body_start = *offset;
    // Locate field 19 (votes) + field 20 (proposals) value spans in one tx-body walk.
    let mut field19: Option<(usize, usize)> = None;
    let mut field20: Option<(usize, usize)> = None;
    match cbor::read_map_header(data, offset).map_err(|_| LedgerTransitionError::MalformedBlock)? {
        cbor::ContainerEncoding::Definite(n, _) => {
            for _ in 0..n {
                read_one_gov_field(data, offset, &mut field19, &mut field20)?;
            }
        }
        cbor::ContainerEncoding::Indefinite => {
            while !cbor::is_break(data, *offset).map_err(|_| LedgerTransitionError::MalformedBlock)? {
                read_one_gov_field(data, offset, &mut field19, &mut field20)?;
            }
            *offset += 1; // consume break
        }
    }
    let body_end = *offset;

    // Phase-2-invalid txs discard ALL body effects; rather than selectively skip a discarded proposal/vote
    // (gated to the byte-exact boundary work), fail closed — parity with the fee scan's cert/withdrawal guard.
    if invalid.contains(&tx_index) {
        if field19.is_some() || field20.is_some() {
            return Err(LedgerTransitionError::InvalidTxCarriesAuthorityEffect { tx_index });
        }
        return Ok(());
    }

    // (1) Capture submitted proposals (field 20). The GovActionId binds to THIS tx's canonical id.
    if let Some((s, e)) = field20 {
        let procs = ade_codec::conway::governance::decode_proposal_procedures(&data[s..e])
            .map_err(|_| LedgerTransitionError::MalformedGovernanceField { tx_index })?;
        // `expires_after = proposed_in + gov_action_lifetime` is persisted future refund authority; a 0
        // lifetime is the placeholder / un-imported value (impossible on any real network). Refuse to
        // track a proposal with a fabricated expiry — the era-correct lifetime must have been imported
        // from the certified curPParams (the v7 bootstrap). `decode_proposal_procedures` rejects an empty
        // set, so reaching here means at least one proposal WILL be tracked.
        if gov.gov_action_lifetime == 0 {
            return Err(LedgerTransitionError::GovActionLifetimeUnproven { tx_index });
        }
        let tx_hash = ade_crypto::transaction_id(&data[body_start..body_end]);
        for (i, p) in procs.into_iter().enumerate() {
            let index = u32::try_from(i)
                .map_err(|_| LedgerTransitionError::MalformedGovernanceField { tx_index })?;
            let expires_after = current_epoch
                .0
                .checked_add(gov.gov_action_lifetime)
                .ok_or(LedgerTransitionError::ArithmeticOverflow)?;
            gov.proposals.push(GovActionState {
                action_id: GovActionId {
                    tx_hash: tx_hash.clone(),
                    index,
                },
                committee_votes: Vec::new(),
                drep_votes: Vec::new(),
                spo_votes: Vec::new(),
                deposit: p.deposit,
                return_addr: p.return_addr,
                gov_action: p.gov_action,
                proposed_in: current_epoch,
                expires_after: EpochNo(expires_after),
            });
        }
    }

    // (2) Vote CAPTURE (field 19): apply each vote to its tracked proposal's vote map — a live vote no
    // longer HALTS the node (CONWAY-RATIFICATION-AND-ENACTMENT-AUTHORITY S2 replaces the CPDE S3
    // detect-and-halt tripwire). Votes on untracked proposals are ignored; a re-vote replaces the voter's
    // prior entry. This is CAPTURE, not ratification: the DRep/SPO ratify gates stay inert (S1 kept their
    // thresholds/stake out of the live gate), so captured DRep/SPO votes do nothing yet. The committee gate
    // is already live via CPDE, so a captured committee vote that reaches quorum makes a proposal
    // potentially-ratifiable → the CPDE S4 potentially-ratifiable terminal (fail-safe) until the CRE S4
    // activation replaces it with real ratify-then-enact.
    // ⚠ S4 SEQUENCING (per the S2 review): the committee + DRep gates count ONLY Yes, so captured No/Abstain
    // votes are fail-safe (monotone toward the terminal). The SPO gate is NON-MONOTONE — it folds No stake
    // into its denominator — so a captured SPO No could flip a proposal to PROVABLY-unratifiable. That is
    // safe ONLY while S1's SPO threshold stays inert. S4 MUST replace the CPDE deposit-refund with real
    // ratify-then-enact BEFORE (or atomically with) activating the live SPO threshold; otherwise a captured
    // SPO No could drive a wrongful expiry-refund.
    if let Some((s, e)) = field19 {
        apply_field19_votes(gov, &data[s..e])
            .map_err(|_| LedgerTransitionError::MalformedGovernanceField { tx_index })?;
    }
    Ok(())
}

/// Read one tx-body field, recording the VALUE byte-span for key 19 (`voting_procedures`) and key 20
/// (`proposal_procedures`); every other field's value is skipped. Advances `offset` past the value.
fn read_one_gov_field(
    data: &[u8],
    offset: &mut usize,
    field19: &mut Option<(usize, usize)>,
    field20: &mut Option<(usize, usize)>,
) -> Result<(), LedgerTransitionError> {
    use ade_codec::cbor;
    let (key, _) =
        cbor::read_uint(data, offset).map_err(|_| LedgerTransitionError::MalformedBlock)?;
    let (vstart, vend) =
        cbor::skip_item(data, offset).map_err(|_| LedgerTransitionError::MalformedBlock)?;
    match key {
        19 => *field19 = Some((vstart, vend)),
        20 => *field20 = Some((vstart, vend)),
        _ => {}
    }
    Ok(())
}

/// Apply the votes in a `voting_procedures` (tx-body field 19) to the tracked proposals' vote maps
/// (CONWAY-RATIFICATION-AND-ENACTMENT-AUTHORITY S2 — replaces the CPDE S3 detect-and-halt tripwire).
/// CDDL: `{ + voter => { + gov_action_id => voting_procedure } }`; voter = `[voter_type, hash28]`
/// (0/1 = committee-hot keyhash/scripthash, 2/3 = DRep keyhash/scripthash, 4 = stake-pool keyhash);
/// voting_procedure = `[vote, anchor/null]` (0=No, 1=Yes, 2=Abstain). A vote on an UNTRACKED proposal is
/// ignored; a re-vote by the same voter REPLACES its prior entry (the ledger keeps the latest vote).
/// Fail-closed (`Err(())`) on any malformed shape / unknown voter-or-vote discriminant; the field-19 value
/// is exactly one (optionally tag-wrapped) map, trailing bytes rejected.
fn apply_field19_votes(gov: &mut ConwayGovState, data: &[u8]) -> Result<(), ()> {
    use ade_codec::cbor;
    let mut offset = 0usize;
    // Tolerate an optional set/tag wrapper (a plain map starts at major 5).
    if offset < data.len() && cbor::peek_major(data, offset).map_err(|_| ())? == 6 {
        let _ = cbor::read_tag(data, &mut offset).map_err(|_| ())?;
    }
    match cbor::read_map_header(data, &mut offset).map_err(|_| ())? {
        cbor::ContainerEncoding::Definite(n, _) => {
            for _ in 0..n {
                apply_one_voter(gov, data, &mut offset)?;
            }
        }
        cbor::ContainerEncoding::Indefinite => {
            while !cbor::is_break(data, offset).map_err(|_| ())? {
                apply_one_voter(gov, data, &mut offset)?;
            }
            offset += 1;
        }
    }
    if offset != data.len() {
        return Err(());
    }
    Ok(())
}

/// One `voter => { + gov_action_id => voting_procedure }` map entry.
fn apply_one_voter(gov: &mut ConwayGovState, data: &[u8], offset: &mut usize) -> Result<(), ()> {
    use ade_codec::cbor;
    let (voter_type, voter_hash) = read_voter(data, offset)?;
    match cbor::read_map_header(data, offset).map_err(|_| ())? {
        cbor::ContainerEncoding::Definite(m, _) => {
            for _ in 0..m {
                let gid = read_gov_action_id(data, offset)?;
                let vote = read_voting_procedure(data, offset)?;
                apply_one_vote(gov, voter_type, &voter_hash, &gid, vote)?;
            }
        }
        cbor::ContainerEncoding::Indefinite => {
            while !cbor::is_break(data, *offset).map_err(|_| ())? {
                let gid = read_gov_action_id(data, offset)?;
                let vote = read_voting_procedure(data, offset)?;
                apply_one_vote(gov, voter_type, &voter_hash, &gid, vote)?;
            }
            *offset += 1;
        }
    }
    Ok(())
}

/// voter = `array(2)[voter_type(uint), hash28(bytes)]`.
fn read_voter(data: &[u8], offset: &mut usize) -> Result<(u64, ade_types::Hash28), ()> {
    use ade_codec::cbor;
    match cbor::read_array_header(data, offset).map_err(|_| ())? {
        cbor::ContainerEncoding::Definite(2, _) => {}
        _ => return Err(()),
    }
    let (voter_type, _) = cbor::read_uint(data, offset).map_err(|_| ())?;
    // A voter is one of the 5 Conway roles (0/1 committee-hot, 2/3 DRep, 4 SPO). Any other discriminant is a
    // CDDL-invalid field-19 → fail-closed at DECODE, regardless of whether its target is tracked (consistent
    // with read_voting_procedure rejecting an out-of-range vote value; a malformed field-19 is a malformed
    // block, not a per-proposal concern).
    if voter_type > 4 {
        return Err(());
    }
    let (hb, _) = cbor::read_bytes(data, offset).map_err(|_| ())?;
    if hb.len() != 28 {
        return Err(());
    }
    let mut h = [0u8; 28];
    h.copy_from_slice(&hb);
    Ok((voter_type, ade_types::Hash28(h)))
}

/// voting_procedure = `array(n>=1)[vote(uint 0..=2), anchor/null?]` → the decoded `Vote`.
fn read_voting_procedure(
    data: &[u8],
    offset: &mut usize,
) -> Result<ade_types::conway::governance::Vote, ()> {
    use ade_codec::cbor;
    use ade_types::conway::governance::Vote;
    let n = match cbor::read_array_header(data, offset).map_err(|_| ())? {
        cbor::ContainerEncoding::Definite(n, _) => n,
        cbor::ContainerEncoding::Indefinite => return Err(()),
    };
    if n == 0 {
        return Err(());
    }
    let (v, _) = cbor::read_uint(data, offset).map_err(|_| ())?;
    let vote = match v {
        0 => Vote::No,
        1 => Vote::Yes,
        2 => Vote::Abstain,
        _ => return Err(()),
    };
    for _ in 1..n {
        let _ = cbor::skip_item(data, offset).map_err(|_| ())?;
    }
    Ok(vote)
}

/// Apply one decoded vote to its tracked proposal (ignored if untracked); a re-vote REPLACES the voter's
/// prior entry. Fail-closed on an unknown voter discriminant that targets a TRACKED proposal.
fn apply_one_vote(
    gov: &mut ConwayGovState,
    voter_type: u64,
    voter_hash: &ade_types::Hash28,
    gid: &ade_types::conway::governance::GovActionId,
    vote: ade_types::conway::governance::Vote,
) -> Result<(), ()> {
    use ade_types::shelley::cert::StakeCredential;
    let Some(p) = gov.proposals.iter_mut().find(|p| &p.action_id == gid) else {
        return Ok(()); // a vote on an untracked proposal — not ours to record
    };
    match voter_type {
        0 | 1 => {
            let cred = if voter_type == 0 {
                StakeCredential::KeyHash(voter_hash.clone())
            } else {
                StakeCredential::ScriptHash(voter_hash.clone())
            };
            upsert_cred_vote(&mut p.committee_votes, cred, vote);
        }
        2 | 3 => {
            let cred = if voter_type == 2 {
                StakeCredential::KeyHash(voter_hash.clone())
            } else {
                StakeCredential::ScriptHash(voter_hash.clone())
            };
            upsert_cred_vote(&mut p.drep_votes, cred, vote);
        }
        4 => {
            if let Some(e) = p.spo_votes.iter_mut().find(|(h, _)| h == voter_hash) {
                e.1 = vote;
            } else {
                p.spo_votes.push((voter_hash.clone(), vote));
            }
        }
        _ => return Err(()), // unknown voter discriminant on a tracked proposal — fail-closed
    }
    Ok(())
}

/// Upsert a `(credential, vote)`: replace the voter's prior vote or append.
fn upsert_cred_vote(
    votes: &mut Vec<(ade_types::shelley::cert::StakeCredential, ade_types::conway::governance::Vote)>,
    cred: ade_types::shelley::cert::StakeCredential,
    vote: ade_types::conway::governance::Vote,
) {
    if let Some(e) = votes.iter_mut().find(|(c, _)| *c == cred) {
        e.1 = vote;
    } else {
        votes.push((cred, vote));
    }
}

/// Decode `gov_action_id = array(2)[tx_hash(bytes32), index(uint)]` at `offset` (mirrors the proven S1
/// ledger-state reader `ledgerdb_state::nn_read_gov_action_id`). Fail-closed on any malformed shape.
fn read_gov_action_id(
    data: &[u8],
    offset: &mut usize,
) -> Result<ade_types::conway::governance::GovActionId, ()> {
    use ade_codec::cbor;
    match cbor::read_array_header(data, offset).map_err(|_| ())? {
        cbor::ContainerEncoding::Definite(2, _) => {}
        _ => return Err(()),
    }
    let (txid, _) = cbor::read_bytes(data, offset).map_err(|_| ())?;
    if txid.len() != 32 {
        return Err(());
    }
    let mut h = [0u8; 32];
    h.copy_from_slice(&txid);
    let (idx, _) = cbor::read_uint(data, offset).map_err(|_| ())?;
    let index = u32::try_from(idx).map_err(|_| ())?;
    Ok(ade_types::conway::governance::GovActionId {
        tx_hash: ade_types::Hash32(h),
        index,
    })
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

    /// A sample per-credential base-UTxO stake map (`ctx.boundary_mark`) for boundary tests that
    /// exercise the crossing machinery but do not assert on the rotated-in mark's content (`p` names a
    /// credential). The per-credential reward effect is proven by
    /// `cross_epoch_boundary_per_credential_mark_pays_member_rewards`.
    fn sample_mark(p: u8, stake: u64) -> BTreeMap<StakeCredential, Coin> {
        let mut m = BTreeMap::new();
        m.insert(key_cred(p), Coin(stake));
        m
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
            Coin(2_744_247_724_989),
            Coin(3_108_895_499_648),
            &delta,
        );
        acc.pending_reward_update = Some(BootstrapRewardUpdate {
            manifest_commitment: Hash32([0x44; 32]),
            source_point_slot: SlotNo(163_000_000),
            source_point_hash: Hash32([0x66; 32]),
            target_epoch: EpochNo(1339),
            delta_treasury: Coin(2_744_247_724_989),
            delta_reserves: Coin(3_108_895_499_648),
            reward_delta: delta,
            canonical_commitment: commitment,
        });
        acc
    }

    // ----- S2 seed (PO-3 / CE-2f): EpochAccumulator::seed_from_bootstrap_ledger -----

    #[test]
    fn seed_splits_bootstrapped_epoch_state_into_prev_buffers() {
        // The bootstrapped EpochState carries nesBprev in `block_production` (→ the prev_* BLOCK buffer)
        // and the certified epoch fee pot in `epoch_fees`. Blocks split into prev_*; the fee pot is the
        // nesBcur-analog and STAYS in `epoch_fees` (the follow adds the seed-to-end tail). The fee
        // prev-buffer is 0 — epoch (seed-1)'s fees ride in the bootstrap RUPD, not a native nesBprev.
        let boot = populated();
        let rupd = boot.pending_reward_update.clone();
        let ledger = boot.as_ledger_view();
        let nes_bprev = ledger.epoch_state.block_production.clone();
        let fee_pot = ledger.epoch_state.epoch_fees;
        assert!(
            !nes_bprev.is_empty(),
            "the test ledger must carry a non-empty nesBprev"
        );

        // nesBcur = the seed epoch's blocks-so-far at the snapshot point (distinct from nesBprev).
        let nes_bcur: BTreeMap<PoolId, u64> = [(pool(0x77), 4u64)].into_iter().collect();
        let seed = EpochAccumulator::seed_from_bootstrap_ledger(
            &ledger,
            EpochNo(1340),
            rupd.clone(),
            nes_bcur.clone(),
        )
        .expect("seed");

        // nesBprev seeds the boundary-consumed BLOCK buffer; the fee prev-buffer is 0 (epoch (seed-1)'s
        // fees ride in the bootstrap RUPD, not a native nesBprev fee).
        assert_eq!(seed.prev_block_production, nes_bprev);
        assert_eq!(seed.prev_epoch_fees, Coin(0));
        // nesBcur seeds the CURRENT block-production buffer (so the seed→seed+1 boundary counts the whole
        // seed epoch). The certified fee pot is the nesBcur-analog: it STAYS in `epoch_fees` so the follow
        // adds the seed-to-end tail and the FULL epoch (seed) fees rotate to prev at the seed boundary.
        assert!(fee_pot.0 > 0, "the test ledger must carry a non-zero certified fee pot");
        assert_eq!(seed.epoch_state.block_production, nes_bcur);
        assert_eq!(seed.epoch_state.epoch_fees, fee_pot);
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
        let err =
            EpochAccumulator::seed_from_bootstrap_ledger(&ledger, EpochNo(1339), None, BTreeMap::new())
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
        let err = EpochAccumulator::seed_from_bootstrap_ledger(
            &ledger,
            ledger.epoch_state.epoch,
            None,
            BTreeMap::new(),
        )
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
            active_slots_per_epoch: 21_600,
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
            active_slots_per_epoch: 21_600,
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

    // ----- Per-credential boundary mark pays member rewards (S3 item #2a, DC-EPOCH-21) -----

    #[test]
    fn cross_epoch_boundary_per_credential_mark_pays_member_rewards() {
        // The boundary mark is now PER-CREDENTIAL. `ctx.boundary_mark` is the per-credential base-UTxO
        // stake (`sum_base_credential_stake`); `build_boundary_mark_snapshot` turns it into a mark whose
        // `delegations` the reward computation reads — the operator's `op_stake` and each member's
        // pro-rata share. The OLD per-pool mark dropped `delegations` → an empty `go` two boundaries
        // later → ZERO member rewards. Same setup, two `go` snapshots: per-credential PAYS, per-pool ZERO.
        let pool_aa = pool(0xAA);
        let member = key_cred(0xCC); // a delegator (not the operator)
        let op = key_cred(0x0B); // the operator's reward-account credential (key_account(0x0B))
        let base = 1_000_000_000_000u64;

        let mut acc = EpochAccumulator::new(CardanoEra::Conway);
        acc.epoch_state.epoch = EpochNo(500);
        acc.protocol_params = conway_params();
        acc.max_lovelace_supply = 45_000_000_000_000_000;
        acc.epoch_state.reserves = Coin(1_000_000_000_000_000);
        acc.cert_state
            .pool
            .pools
            .insert(pool_aa.clone(), sample_pool_params(0xAA, key_account(0x0B)));
        for c in [member.clone(), op.clone()] {
            acc.cert_state
                .delegation
                .registrations
                .insert(c.clone(), Coin(2_000_000));
            acc.cert_state
                .delegation
                .delegations
                .insert(c, pool_aa.clone());
        }
        // The pool produced blocks in the to-be-rewarded epoch (nesBprev).
        acc.prev_block_production.insert(pool_aa.clone(), 100);

        // The per-credential base-UTxO stake the advancer supplies (item #2b): member + operator.
        let mut base_utxo: BTreeMap<StakeCredential, Coin> = BTreeMap::new();
        base_utxo.insert(member.clone(), Coin(base));
        base_utxo.insert(op.clone(), Coin(base));

        // The builder produces a PER-CREDENTIAL mark: delegations populated, pool_stakes summed.
        let built = build_boundary_mark_snapshot(&base_utxo, &acc.cert_state.delegation);
        assert_eq!(
            built.delegations.get(member.hash()),
            Some(&(pool_aa.clone(), Coin(base))),
            "the member's per-credential stake is carried into the mark"
        );
        assert_eq!(
            built.delegations.get(op.hash()),
            Some(&(pool_aa.clone(), Coin(base))),
            "the operator's per-credential stake is carried into the mark"
        );
        assert_eq!(built.pool_stakes.get(&pool_aa), Some(&Coin(2 * base)));

        let ctx = SelectedBlockCtx {
            era: CardanoEra::Conway,
            block_epoch: EpochNo(501),
            block_slot: SlotNo(0),
            issuer_pool: pool_aa.clone(),
            boundary_mark: Some(base_utxo.clone()),
            active_slots_per_epoch: 21_600,
        };

        // PER-CREDENTIAL `go` (the mark, two boundaries on): member + operator rewards are non-zero.
        let mut acc_pc = acc.clone();
        acc_pc.epoch_state.snapshots.go = GoSnapshot(built.clone());
        let after_pc =
            cross_epoch_boundary(acc_pc, EpochNo(501), &ctx).expect("per-credential boundary");
        let member_reward = after_pc
            .cert_state
            .delegation
            .rewards
            .get(&member)
            .copied()
            .unwrap_or(Coin(0));
        let op_reward = after_pc
            .cert_state
            .delegation
            .rewards
            .get(&op)
            .copied()
            .unwrap_or(Coin(0));
        assert!(
            member_reward.0 > 0,
            "the per-credential mark must pay a member reward, got {member_reward:?}"
        );
        assert!(
            op_reward.0 > 0,
            "the operator's op_stake must drive a non-zero leader reward, got {op_reward:?}"
        );
        // The cross also rotates in a per-credential mark (the wiring uses the builder).
        assert!(
            !after_pc.epoch_state.snapshots.mark.0.delegations.is_empty(),
            "the rotated-in mark is per-credential"
        );

        // CONTRAST: the OLD per-pool mark — same pool_stakes, EMPTY delegations — pays ZERO member
        // rewards on the identical setup. This is the byte-insufficiency the reshape fixes.
        let per_pool_go = StakeSnapshot {
            delegations: BTreeMap::new(),
            pool_stakes: built.pool_stakes.clone(),
        };
        let mut acc_pp = acc.clone();
        acc_pp.epoch_state.snapshots.go = GoSnapshot(per_pool_go);
        let after_pp = cross_epoch_boundary(acc_pp, EpochNo(501), &ctx).expect("per-pool boundary");
        let member_pp = after_pp
            .cert_state
            .delegation
            .rewards
            .get(&member)
            .copied()
            .unwrap_or(Coin(0));
        assert_eq!(
            member_pp.0, 0,
            "a per-pool mark (empty delegations) pays ZERO member rewards"
        );
    }

    // ----- One-shot bootstrap RUPD: APPLIED EXACTLY ONCE at the seed boundary (pots + rs), then consumed -----

    #[test]
    fn pending_reward_update_applied_once_at_seed_boundary() {
        let mut acc = reward_fixture();
        acc.epoch_state.epoch = EpochNo(1339);
        let delta_treasury = Coin(1_000);
        let delta_reserves = Coin(2_000);
        let mut delta = BTreeMap::new();
        delta.insert(key_cred(0xCC), Coin(4_242));
        let commitment = bootstrap_rupd_commitment(
            &Hash32([0x01; 32]),
            SlotNo(1),
            &Hash32([0x02; 32]),
            EpochNo(1339),
            delta_treasury,
            delta_reserves,
            &delta,
        );
        acc.pending_reward_update = Some(BootstrapRewardUpdate {
            manifest_commitment: Hash32([0x01; 32]),
            source_point_slot: SlotNo(1),
            source_point_hash: Hash32([0x02; 32]),
            target_epoch: EpochNo(1339),
            delta_treasury,
            delta_reserves,
            reward_delta: delta,
            canonical_commitment: commitment,
        });
        // The seed boundary (target_epoch+1 == 1340) forces the native reward to zero (empty inputs) and
        // applies the bound RUPD EXACTLY once: pots + rs. Assert the byte-exact pot moves + the rs credit.
        let reserves_before = acc.epoch_state.reserves.0;
        let treasury_before = acc.epoch_state.treasury.0;
        let ctx = SelectedBlockCtx {
            era: CardanoEra::Conway,
            block_epoch: EpochNo(1340),
            block_slot: SlotNo(0),
            issuer_pool: pool(0xAA),
            boundary_mark: Some(sample_mark(0xAA, 1_000_000_000_000)),
            active_slots_per_epoch: 21_600,
        };
        let after = cross_epoch_boundary(acc, EpochNo(1340), &ctx).expect("boundary");
        assert_eq!(
            after.cert_state.delegation.rewards.get(&key_cred(0xCC)),
            Some(&Coin(4_242)),
            "the bootstrap RUPD's rs is credited at the seed boundary"
        );
        assert_eq!(
            after.epoch_state.treasury.0,
            treasury_before + delta_treasury.0,
            "the bootstrap RUPD adds deltaT to treasury"
        );
        assert_eq!(
            after.epoch_state.reserves.0,
            reserves_before - delta_reserves.0,
            "the bootstrap RUPD subtracts delta_reserves from reserves"
        );
        assert!(
            after.pending_reward_update.is_none(),
            "the one-shot bootstrap RUPD is CONSUMED at its target boundary"
        );
    }

    /// ZERO-DOUBLE-COUNT INVARIANT (CONWAY-PROPOSAL-DEPOSIT-EXPIRY / DC-EPOCH-18). The imported fee pot
    /// (the certified snapshot's epoch-(seed) fees) and the bootstrap RUPD (cardano's exact reward for
    /// epoch (seed-1), carrying epoch (seed-1)'s `deltaF`) represent DISTINCT historical epochs' fee
    /// contributions; the seed-adjacent boundary may NEVER count the same fee twice. Proven directly:
    ///   (a) at the seed boundary the native reward draws ZERO fees (`epoch_fees` forced to 0), so the
    ///       imported pot is NOT pulled into the seed-boundary reward;
    ///   (b) the pots move by EXACTLY the RUPD's deltas — no extra fee-driven native draw;
    ///   (c) the imported pot rotates to `prev_epoch_fees` INTACT, to be consumed EXACTLY ONCE at the
    ///       NEXT boundary (seed+1 -> seed+2), never here.
    #[test]
    fn imported_fee_pot_not_double_counted_with_bootstrap_rupd_at_seed_boundary() {
        const IMPORTED_FEES: u64 = 7_777_777;
        let mut acc = reward_fixture();
        acc.epoch_state.epoch = EpochNo(1339);
        // The certified snapshot's epoch-(seed) fee pot (the v5/v6 import seeds this).
        acc.epoch_state.epoch_fees = Coin(IMPORTED_FEES);
        let delta_treasury = Coin(1_000);
        let delta_reserves = Coin(2_000);
        let mut delta = BTreeMap::new();
        delta.insert(key_cred(0xCC), Coin(4_242));
        let commitment = bootstrap_rupd_commitment(
            &Hash32([0x01; 32]),
            SlotNo(1),
            &Hash32([0x02; 32]),
            EpochNo(1339),
            delta_treasury,
            delta_reserves,
            &delta,
        );
        acc.pending_reward_update = Some(BootstrapRewardUpdate {
            manifest_commitment: Hash32([0x01; 32]),
            source_point_slot: SlotNo(1),
            source_point_hash: Hash32([0x02; 32]),
            target_epoch: EpochNo(1339),
            delta_treasury,
            delta_reserves,
            reward_delta: delta,
            canonical_commitment: commitment,
        });
        let reserves_before = acc.epoch_state.reserves.0;
        let treasury_before = acc.epoch_state.treasury.0;
        let ctx = SelectedBlockCtx {
            era: CardanoEra::Conway,
            block_epoch: EpochNo(1340),
            block_slot: SlotNo(0),
            issuer_pool: pool(0xAA),
            boundary_mark: Some(sample_mark(0xAA, 1_000_000_000_000)),
            active_slots_per_epoch: 21_600,
        };
        let after = cross_epoch_boundary(acc, EpochNo(1340), &ctx).expect("boundary");
        // (c) the imported epoch-(seed) fee pot is held INTACT in prev_epoch_fees — consumed exactly
        // once, at the NEXT boundary, NOT here.
        assert_eq!(
            after.prev_epoch_fees.0, IMPORTED_FEES,
            "imported fee pot rotates to prev_epoch_fees INTACT (consumed once, at the next boundary)"
        );
        // (a)+(b) the seed-boundary pots move by EXACTLY the bootstrap RUPD's deltas — the imported fee
        // pot drove NO additional fee-driven native draw (it is not double-counted into this reward).
        assert_eq!(
            after.epoch_state.reserves.0,
            reserves_before - delta_reserves.0,
            "ONLY the RUPD draws reserves at the seed boundary (no fee-driven native draw)"
        );
        assert_eq!(
            after.epoch_state.treasury.0,
            treasury_before + delta_treasury.0,
            "ONLY the RUPD moves treasury at the seed boundary"
        );
        // the new epoch's fresh nesBcur fee accumulator starts at zero (the imported pot lives in prev).
        assert_eq!(after.epoch_state.epoch_fees.0, 0, "new-epoch fee accumulator is fresh");
    }

    #[test]
    fn pending_reward_update_rejected_at_non_target_boundary() {
        let mut acc = reward_fixture();
        acc.epoch_state.epoch = EpochNo(1339);
        let delta = BTreeMap::new();
        let commitment = bootstrap_rupd_commitment(
            &Hash32([0x01; 32]),
            SlotNo(1),
            &Hash32([0x02; 32]),
            EpochNo(1337), // target+1 = 1338, NOT this boundary (1340)
            Coin(0),
            Coin(0),
            &delta,
        );
        acc.pending_reward_update = Some(BootstrapRewardUpdate {
            manifest_commitment: Hash32([0x01; 32]),
            source_point_slot: SlotNo(1),
            source_point_hash: Hash32([0x02; 32]),
            target_epoch: EpochNo(1337),
            delta_treasury: Coin(0),
            delta_reserves: Coin(0),
            reward_delta: delta,
            canonical_commitment: commitment,
        });
        let ctx = SelectedBlockCtx {
            era: CardanoEra::Conway,
            block_epoch: EpochNo(1340),
            block_slot: SlotNo(0),
            issuer_pool: pool(0xAA),
            boundary_mark: Some(sample_mark(0xAA, 1_000_000_000_000)),
            active_slots_per_epoch: 21_600,
        };
        match cross_epoch_boundary(acc, EpochNo(1340), &ctx) {
            Err(LedgerTransitionError::BootstrapRupdWrongBoundary {
                rupd_target: 1337,
                boundary: 1340,
            }) => {}
            other => panic!("expected BootstrapRupdWrongBoundary, got {other:?}"),
        }
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
            active_slots_per_epoch: 21_600,
        };
        match cross_epoch_boundary(acc, EpochNo(501), &ctx) {
            Err(LedgerTransitionError::MissingBoundaryStake { epoch: 501 }) => {}
            other => panic!("expected MissingBoundaryStake, got {other:?}"),
        }
    }

    /// IDD §2/§8 guard (CE-3d): the eta denominator `active_slots_per_epoch` is a REQUIRED boundary
    /// input. A 0 reaching a real cross with a mark PRESENT (a miswire — the within-epoch sentinel
    /// escaping its path) HALTS deterministically instead of silently yielding `eta = 1` (full
    /// monetary expansion, over-drawing reserves).
    #[test]
    fn zero_active_slots_at_boundary_is_fail_closed() {
        let acc = reward_fixture();
        let ctx = SelectedBlockCtx {
            era: CardanoEra::Conway,
            block_epoch: EpochNo(501),
            block_slot: SlotNo(0),
            issuer_pool: pool(0xAA),
            boundary_mark: Some(sample_mark(0xAA, 1_000_000_000_000)), // mark present...
            active_slots_per_epoch: 0,                                 // ...but the eta denom is 0
        };
        match cross_epoch_boundary(acc, EpochNo(501), &ctx) {
            Err(LedgerTransitionError::MissingBoundaryStake { epoch: 501 }) => {}
            other => panic!("expected fail-close on 0 active_slots, got {other:?}"),
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
            active_slots_per_epoch: 21_600,
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
            active_slots_per_epoch: 21_600,
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
            active_slots_per_epoch: 21_600,
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
            active_slots_per_epoch: 21_600,
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
            active_slots_per_epoch: 21_600,
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
            active_slots_per_epoch: 21_600,
        };
        let ctx_e1 = SelectedBlockCtx {
            era: CardanoEra::Conway,
            block_epoch: EpochNo(501), // crosses a boundary
            block_slot: SlotNo(43_100_000),
            issuer_pool: pool(0x88),
            boundary_mark: Some(sample_mark(0x77, 1_000_000_000_000)),
            active_slots_per_epoch: 21_600,
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

    // ----- CONWAY-PROPOSAL-DEPOSIT-EXPIRY S3: within-epoch governance capture + vote tripwire -----

    use ade_types::conway::governance::{
        Anchor, GovAction, GovActionId, GovActionState, ProposalProcedure,
    };

    /// A round-trippable opaque anchor (`[ "x", h'aa..' ]`), mirroring the codec test fixture.
    fn s3_anchor() -> Anchor {
        let mut buf = Vec::new();
        write_array_header(&mut buf, ContainerEncoding::Definite(2, IntWidth::Inline));
        ade_codec::cbor::write_text_canonical(&mut buf, "x");
        write_bytes_canonical(&mut buf, &[0xaa; 32]);
        Anchor { raw: buf }
    }

    /// An `InfoAction` proposal procedure with an observable deposit + return address.
    fn s3_proposal(deposit: u64, return_addr_byte: u8) -> ProposalProcedure {
        ProposalProcedure {
            deposit: Coin(deposit),
            return_addr: vec![return_addr_byte; 29],
            gov_action: GovAction::InfoAction,
            anchor: s3_anchor(),
        }
    }

    /// tx-body field-20 (`proposal_procedures`) value bytes for `procs`.
    fn s3_field20(procs: &[ProposalProcedure]) -> Vec<u8> {
        ade_codec::conway::governance::encode_proposal_procedures(procs)
    }

    /// tx-body field-19 (`voting_procedures`) value bytes: one DRep voter (tag 2) casting a Yes on each
    /// `targets` action id. `voting_procedures = { voter => { gov_action_id => voting_procedure } }`.
    fn s3_field19(targets: &[GovActionId]) -> Vec<u8> {
        let mut buf = Vec::new();
        write_map_header(&mut buf, ContainerEncoding::Definite(1, IntWidth::Inline)); // 1 voter
        // voter = [2, addr_keyhash(28)] (DRep keyhash)
        write_array_header(&mut buf, ContainerEncoding::Definite(2, IntWidth::Inline));
        write_uint_canonical(&mut buf, 2);
        write_bytes_canonical(&mut buf, &[0x11; 28]);
        // { gov_action_id => voting_procedure }
        let n = targets.len() as u64;
        write_map_header(&mut buf, ContainerEncoding::Definite(n, canonical_width(n)));
        for gid in targets {
            // gov_action_id = [tx_hash(32), index]
            write_array_header(&mut buf, ContainerEncoding::Definite(2, IntWidth::Inline));
            write_bytes_canonical(&mut buf, &gid.tx_hash.0);
            write_uint_canonical(&mut buf, gid.index as u64);
            // voting_procedure = [vote, anchor/null] — skipped by the tripwire extractor.
            write_array_header(&mut buf, ContainerEncoding::Definite(2, IntWidth::Inline));
            write_uint_canonical(&mut buf, 1); // Yes
            write_null(&mut buf);
        }
        buf
    }

    /// A tx body map carrying `fields` (which MUST be in ascending key order).
    fn s3_body(fields: &[(u64, Vec<u8>)]) -> Vec<u8> {
        let mut buf = Vec::new();
        let n = fields.len() as u64;
        write_map_header(&mut buf, ContainerEncoding::Definite(n, canonical_width(n)));
        for (k, v) in fields {
            write_uint_canonical(&mut buf, *k);
            buf.extend_from_slice(v);
        }
        buf
    }

    /// Wrap tx bodies into the `tx_bodies` array.
    fn s3_tx_bodies(bodies: &[Vec<u8>]) -> Vec<u8> {
        let mut buf = Vec::new();
        let n = bodies.len() as u64;
        write_array_header(&mut buf, ContainerEncoding::Definite(n, canonical_width(n)));
        for b in bodies {
            buf.extend_from_slice(b);
        }
        buf
    }

    fn s3_empty_gov(lifetime: u64) -> ConwayGovState {
        ConwayGovState {
            proposals: Vec::new(),
            committee: std::collections::BTreeMap::new(),
            committee_quorum: (2, 3),
            drep_expiry: std::collections::BTreeMap::new(),
            gov_action_lifetime: lifetime,
            vote_delegations: std::collections::BTreeMap::new(),
            pool_voting_thresholds: Vec::new(),
            drep_voting_thresholds: Vec::new(),
            committee_hot_keys: std::collections::BTreeMap::new(),
        }
    }

    fn s3_tracked(gov: &mut ConwayGovState, id: GovActionId, expires_after: EpochNo) {
        gov.proposals.push(GovActionState {
            action_id: id,
            committee_votes: Vec::new(),
            drep_votes: Vec::new(),
            spo_votes: Vec::new(),
            deposit: Coin(100_000_000_000),
            return_addr: vec![0xe0; 29],
            gov_action: GovAction::InfoAction,
            proposed_in: EpochNo(1309),
            expires_after,
        });
    }

    #[test]
    fn s3_live_proposal_captured_with_identity_epoch_and_expiry() {
        let p = s3_proposal(100_000_000_000, 0xe0);
        let body = s3_body(&[(20, s3_field20(std::slice::from_ref(&p)))]);
        let expected_tx_hash = ade_crypto::transaction_id(&body);
        let bodies = s3_tx_bodies(std::slice::from_ref(&body));

        let gov =
            apply_block_governance(s3_empty_gov(6), &bodies, 1, &invalid_set(&[]), EpochNo(1309))
                .expect("capture");
        assert_eq!(gov.proposals.len(), 1);
        let gas = &gov.proposals[0];
        // Identity binds to THIS tx's canonical id (= blake2b-256 of the body bytes), index 0.
        assert_eq!(gas.action_id.tx_hash, expected_tx_hash);
        assert_eq!(gas.action_id.index, 0);
        assert_eq!(gas.deposit, Coin(100_000_000_000));
        assert_eq!(gas.return_addr, vec![0xe0; 29]);
        assert_eq!(gas.gov_action, GovAction::InfoAction);
        assert_eq!(gas.proposed_in, EpochNo(1309));
        // expires_after = proposed_in + gov_action_lifetime.
        assert_eq!(gas.expires_after, EpochNo(1309 + 6));
        assert!(
            gas.committee_votes.is_empty()
                && gas.drep_votes.is_empty()
                && gas.spo_votes.is_empty(),
            "a freshly captured proposal carries NO votes (Ade never tallies)"
        );
    }

    #[test]
    fn s3_two_proposals_one_tx_get_sequential_indices_same_txid() {
        let procs = [s3_proposal(1_000, 0xe0), s3_proposal(2_000, 0xe1)];
        let body = s3_body(&[(20, s3_field20(&procs))]);
        let txid = ade_crypto::transaction_id(&body);
        let bodies = s3_tx_bodies(std::slice::from_ref(&body));
        let gov = apply_block_governance(s3_empty_gov(6), &bodies, 1, &invalid_set(&[]), EpochNo(500))
            .expect("capture");
        assert_eq!(gov.proposals.len(), 2);
        assert_eq!(
            gov.proposals[0].action_id,
            GovActionId { tx_hash: txid.clone(), index: 0 }
        );
        assert_eq!(
            gov.proposals[1].action_id,
            GovActionId { tx_hash: txid, index: 1 }
        );
        assert_eq!(gov.proposals[0].deposit, Coin(1_000));
        assert_eq!(gov.proposals[1].deposit, Coin(2_000));
    }

    #[test]
    fn cre_s2_vote_on_tracked_proposal_is_captured() {
        use ade_types::conway::governance::Vote;
        use ade_types::shelley::cert::StakeCredential;
        use ade_types::Hash28;
        let pid = GovActionId { tx_hash: Hash32([0xAB; 32]), index: 0 };
        let mut gov = s3_empty_gov(6);
        s3_tracked(&mut gov, pid.clone(), EpochNo(1339));
        let body = s3_body(&[(19, s3_field19(std::slice::from_ref(&pid)))]);
        let bodies = s3_tx_bodies(std::slice::from_ref(&body));
        // CRE S2: a live vote is CAPTURED, no longer a terminal (replaces the CPDE S3 tripwire).
        let gov = apply_block_governance(gov, &bodies, 1, &invalid_set(&[]), EpochNo(1338))
            .expect("a live vote is captured, not a halt");
        let p = gov.proposals.iter().find(|p| p.action_id == pid).expect("proposal tracked");
        // s3_field19 encodes voter_type 2 (DRep keyhash 0x11..) voting Yes → the tracked proposal's drep_votes.
        assert_eq!(
            p.drep_votes,
            vec![(StakeCredential::KeyHash(Hash28([0x11; 28])), Vote::Yes)],
            "the DRep vote landed in drep_votes"
        );
        assert!(p.committee_votes.is_empty() && p.spo_votes.is_empty(), "only the drep map got the vote");
    }

    #[test]
    fn s3_vote_on_untracked_proposal_is_carried_forward() {
        let pid = GovActionId { tx_hash: Hash32([0xAB; 32]), index: 0 };
        let qid = GovActionId { tx_hash: Hash32([0xCD; 32]), index: 7 };
        let mut gov = s3_empty_gov(6);
        s3_tracked(&mut gov, pid, EpochNo(1339));
        let before = gov.clone();
        let body = s3_body(&[(19, s3_field19(std::slice::from_ref(&qid)))]);
        let bodies = s3_tx_bodies(std::slice::from_ref(&body));
        let after = apply_block_governance(gov, &bodies, 1, &invalid_set(&[]), EpochNo(1338))
            .expect("a vote on an UNtracked proposal is a no-op, never a refund or terminal");
        assert_eq!(after, before, "nothing tracked changed");
    }

    #[test]
    fn cre_s2_cross_tx_same_block_vote_on_just_submitted_proposal_is_captured() {
        use ade_types::conway::governance::Vote;
        use ade_types::shelley::cert::StakeCredential;
        use ade_types::Hash28;
        // tx0 submits P; tx1 (same block) votes on P. P is tracked when tx1 runs ⇒ the vote is CAPTURED.
        let p = s3_proposal(100_000_000_000, 0xe0);
        let tx0 = s3_body(&[(20, s3_field20(std::slice::from_ref(&p)))]);
        let pid = GovActionId { tx_hash: ade_crypto::transaction_id(&tx0), index: 0 };
        let tx1 = s3_body(&[(19, s3_field19(std::slice::from_ref(&pid)))]);
        let bodies = s3_tx_bodies(&[tx0, tx1]);
        let gov = apply_block_governance(s3_empty_gov(6), &bodies, 2, &invalid_set(&[]), EpochNo(500))
            .expect("the vote on the just-submitted proposal is captured");
        let tracked = gov.proposals.iter().find(|q| q.action_id == pid).expect("tracked");
        assert_eq!(
            tracked.drep_votes,
            vec![(StakeCredential::KeyHash(Hash28([0x11; 28])), Vote::Yes)],
            "the cross-tx vote landed on the just-submitted proposal"
        );
    }

    /// field-19 with one voter of `voter_type` (hash = `hash`×28) casting `vote` (0=No,1=Yes,2=Abstain) on
    /// each target.
    fn cre_s2_field19(voter_type: u64, hash: u8, votes: &[(GovActionId, u64)]) -> Vec<u8> {
        let mut buf = Vec::new();
        write_map_header(&mut buf, ContainerEncoding::Definite(1, IntWidth::Inline));
        write_array_header(&mut buf, ContainerEncoding::Definite(2, IntWidth::Inline));
        write_uint_canonical(&mut buf, voter_type);
        write_bytes_canonical(&mut buf, &[hash; 28]);
        let n = votes.len() as u64;
        write_map_header(&mut buf, ContainerEncoding::Definite(n, canonical_width(n)));
        for (gid, vote) in votes {
            write_array_header(&mut buf, ContainerEncoding::Definite(2, IntWidth::Inline));
            write_bytes_canonical(&mut buf, &gid.tx_hash.0);
            write_uint_canonical(&mut buf, gid.index as u64);
            write_array_header(&mut buf, ContainerEncoding::Definite(2, IntWidth::Inline));
            write_uint_canonical(&mut buf, *vote);
            write_null(&mut buf);
        }
        buf
    }

    #[test]
    fn cre_s2_captures_committee_and_spo_votes_by_voter_type() {
        use ade_types::conway::governance::Vote;
        use ade_types::shelley::cert::StakeCredential;
        use ade_types::Hash28;
        let pid = GovActionId { tx_hash: Hash32([0xAB; 32]), index: 0 };
        // committee (voter_type 1 = scripthash) voting No -> committee_votes as ScriptHash.
        let mut gov = s3_empty_gov(6);
        s3_tracked(&mut gov, pid.clone(), EpochNo(1339));
        let body = s3_body(&[(19, cre_s2_field19(1, 0x22, &[(pid.clone(), 0)]))]);
        let bodies = s3_tx_bodies(std::slice::from_ref(&body));
        let gov = apply_block_governance(gov, &bodies, 1, &invalid_set(&[]), EpochNo(1338)).expect("cap");
        let p = gov.proposals.iter().find(|p| p.action_id == pid).unwrap();
        assert_eq!(p.committee_votes, vec![(StakeCredential::ScriptHash(Hash28([0x22; 28])), Vote::No)]);
        assert!(p.drep_votes.is_empty() && p.spo_votes.is_empty());
        // spo (voter_type 4) voting Abstain -> spo_votes as Hash28.
        let mut gov = s3_empty_gov(6);
        s3_tracked(&mut gov, pid.clone(), EpochNo(1339));
        let body = s3_body(&[(19, cre_s2_field19(4, 0x33, &[(pid.clone(), 2)]))]);
        let bodies = s3_tx_bodies(std::slice::from_ref(&body));
        let gov = apply_block_governance(gov, &bodies, 1, &invalid_set(&[]), EpochNo(1338)).expect("cap");
        let p = gov.proposals.iter().find(|p| p.action_id == pid).unwrap();
        assert_eq!(p.spo_votes, vec![(Hash28([0x33; 28]), Vote::Abstain)]);
    }

    #[test]
    fn cre_s2_revote_replaces_prior_vote() {
        use ade_types::conway::governance::Vote;
        use ade_types::shelley::cert::StakeCredential;
        use ade_types::Hash28;
        let pid = GovActionId { tx_hash: Hash32([0xAB; 32]), index: 0 };
        let mut gov = s3_empty_gov(6);
        s3_tracked(&mut gov, pid.clone(), EpochNo(1339));
        // tx0: DRep 0x11 votes No; tx1: same DRep votes Yes -> the latest (Yes) REPLACES (not appended).
        let tx0 = s3_body(&[(19, cre_s2_field19(2, 0x11, &[(pid.clone(), 0)]))]);
        let tx1 = s3_body(&[(19, cre_s2_field19(2, 0x11, &[(pid.clone(), 1)]))]);
        let bodies = s3_tx_bodies(&[tx0, tx1]);
        let gov = apply_block_governance(gov, &bodies, 2, &invalid_set(&[]), EpochNo(1338)).expect("cap");
        let p = gov.proposals.iter().find(|p| p.action_id == pid).unwrap();
        assert_eq!(
            p.drep_votes,
            vec![(StakeCredential::KeyHash(Hash28([0x11; 28])), Vote::Yes)],
            "re-vote replaced the prior entry"
        );
    }

    #[test]
    fn cre_s2_unknown_voter_type_on_tracked_proposal_is_terminal() {
        let pid = GovActionId { tx_hash: Hash32([0xAB; 32]), index: 0 };
        let mut gov = s3_empty_gov(6);
        s3_tracked(&mut gov, pid.clone(), EpochNo(1339));
        let body = s3_body(&[(19, cre_s2_field19(5, 0x11, &[(pid.clone(), 1)]))]); // voter_type 5 = unknown
        let bodies = s3_tx_bodies(std::slice::from_ref(&body));
        let err = apply_block_governance(gov, &bodies, 1, &invalid_set(&[]), EpochNo(1338)).unwrap_err();
        assert!(
            matches!(err, LedgerTransitionError::MalformedGovernanceField { tx_index: 0 }),
            "an unknown voter discriminant on a tracked proposal is terminal, got {err:?}"
        );
    }

    #[test]
    fn s3_invalid_tx_carrying_proposal_is_fail_closed() {
        let p = s3_proposal(1, 0xe0);
        let body = s3_body(&[(20, s3_field20(std::slice::from_ref(&p)))]);
        let bodies = s3_tx_bodies(std::slice::from_ref(&body));
        let err = apply_block_governance(s3_empty_gov(6), &bodies, 1, &invalid_set(&[0]), EpochNo(500))
            .unwrap_err();
        assert!(
            matches!(err, LedgerTransitionError::InvalidTxCarriesAuthorityEffect { tx_index: 0 }),
            "a phase-2-invalid tx's proposal must fail closed, got {err:?}"
        );
    }

    #[test]
    fn s3_invalid_tx_carrying_vote_is_fail_closed() {
        let qid = GovActionId { tx_hash: Hash32([0xCD; 32]), index: 0 };
        let body = s3_body(&[(19, s3_field19(std::slice::from_ref(&qid)))]);
        let bodies = s3_tx_bodies(std::slice::from_ref(&body));
        let err = apply_block_governance(s3_empty_gov(6), &bodies, 1, &invalid_set(&[0]), EpochNo(500))
            .unwrap_err();
        assert!(
            matches!(err, LedgerTransitionError::InvalidTxCarriesAuthorityEffect { tx_index: 0 }),
            "got {err:?}"
        );
    }

    #[test]
    fn s3_malformed_field20_is_fail_closed() {
        // field 20 present but an empty set — violates the CIP-1694 non-empty invariant.
        let body = s3_body(&[(20, vec![0x80])]); // array(0)
        let bodies = s3_tx_bodies(std::slice::from_ref(&body));
        let err = apply_block_governance(s3_empty_gov(6), &bodies, 1, &invalid_set(&[]), EpochNo(500))
            .unwrap_err();
        assert!(
            matches!(err, LedgerTransitionError::MalformedGovernanceField { tx_index: 0 }),
            "a malformed field 20 is terminal, never a silent skip, got {err:?}"
        );
    }

    #[test]
    fn s3_malformed_field19_is_fail_closed() {
        // field 19 present but a bare uint, not a voter map.
        let body = s3_body(&[(19, vec![0x00])]);
        let bodies = s3_tx_bodies(std::slice::from_ref(&body));
        let err = apply_block_governance(s3_empty_gov(6), &bodies, 1, &invalid_set(&[]), EpochNo(500))
            .unwrap_err();
        assert!(
            matches!(err, LedgerTransitionError::MalformedGovernanceField { tx_index: 0 }),
            "got {err:?}"
        );
    }

    #[test]
    fn s3_block_without_governance_fields_is_noop() {
        let body = tx_body_fee(50); // only field 2
        let bodies = s3_tx_bodies(std::slice::from_ref(&body));
        let gov = s3_empty_gov(6);
        let before = gov.clone();
        let after = apply_block_governance(gov, &bodies, 1, &invalid_set(&[]), EpochNo(500))
            .expect("no governance fields ⇒ no change");
        assert_eq!(after, before);
    }

    #[test]
    fn s3_capture_skips_non_gov_fields_and_is_replay_equivalent() {
        // A body carrying a fee (field 2) THEN a proposal (field 20): the pass skips field 2, captures 20.
        let p = s3_proposal(100_000_000_000, 0xe0);
        let body = s3_body(&[(2, cbor_uint(50)), (20, s3_field20(std::slice::from_ref(&p)))]);
        let bodies = s3_tx_bodies(std::slice::from_ref(&body));
        let a = apply_block_governance(s3_empty_gov(6), &bodies, 1, &invalid_set(&[]), EpochNo(1309))
            .expect("a");
        let b = apply_block_governance(s3_empty_gov(6), &bodies, 1, &invalid_set(&[]), EpochNo(1309))
            .expect("b");
        assert_eq!(a, b, "same gov + block + epoch ⇒ identical proposals (replay-equivalent)");
        assert_eq!(a.proposals.len(), 1);
        assert_eq!(a.proposals[0].deposit, Coin(100_000_000_000));
    }

    #[test]
    fn s3_unproven_zero_lifetime_refuses_to_fabricate_expiry() {
        // A 0 gov_action_lifetime is the placeholder / un-imported value (impossible on a real network).
        // Capturing a proposal would fabricate `expires_after = proposed_in`. Refuse — the timing
        // authority must be imported from the certified curPParams (the v7 bootstrap).
        let p = s3_proposal(100_000_000_000, 0xe0);
        let body = s3_body(&[(20, s3_field20(std::slice::from_ref(&p)))]);
        let bodies = s3_tx_bodies(std::slice::from_ref(&body));
        let err = apply_block_governance(s3_empty_gov(0), &bodies, 1, &invalid_set(&[]), EpochNo(1309))
            .unwrap_err();
        assert!(
            matches!(err, LedgerTransitionError::GovActionLifetimeUnproven { tx_index: 0 }),
            "a 0 (unimported) lifetime must fail closed, never fabricate an expiry, got {err:?}"
        );
    }

    #[test]
    fn s3_gov_state_none_is_untracked_and_skipped() {
        // The within-epoch wiring: a `None` gov_state means governance is not tracked, so a block carrying
        // a proposal applies cleanly and leaves gov_state None — the same gating process_block_certificates
        // uses (no silent governance authority on an untracked replay).
        let mut acc = fresh_conway_acc();
        assert!(acc.gov_state.is_none());
        acc.gov_state = None;
        let ctx = SelectedBlockCtx {
            era: CardanoEra::Conway,
            block_epoch: EpochNo(500),
            block_slot: SlotNo(43_000_000),
            issuer_pool: pool(0x77),
            boundary_mark: None,
            active_slots_per_epoch: 21_600,
        };
        let out = apply_selected_block(&acc, RAW_CONWAY_BLOCK, &ctx).expect("apply");
        assert!(out.gov_state.is_none(), "untracked governance stays None");
    }

    // ----- CONWAY-PROPOSAL-DEPOSIT-EXPIRY S4: the boundary deposit-expiry-refund transition -----

    use ade_types::conway::governance::Vote;

    fn s4_tw() -> GovAction {
        GovAction::TreasuryWithdrawals { withdrawals: Vec::new(), policy_hash: None }
    }
    fn s4_gas(
        id: u8, action: GovAction, votes: Vec<(StakeCredential, Vote)>,
        expires: u64, deposit: u64, ra: u8,
    ) -> GovActionState {
        GovActionState {
            action_id: GovActionId { tx_hash: Hash32([id; 32]), index: 0 },
            committee_votes: votes,
            drep_votes: Vec::new(),
            spo_votes: Vec::new(),
            deposit: Coin(deposit),
            return_addr: vec![ra; 29],
            gov_action: action,
            proposed_in: EpochNo(1309),
            expires_after: EpochNo(expires),
        }
    }
    fn s4_acc_with_gov(proposals: Vec<GovActionState>) -> EpochAccumulator {
        let mut acc = EpochAccumulator::new(CardanoEra::Conway);
        acc.epoch_state.epoch = EpochNo(1340);
        // Register the return-address credentials the S4 tests use (0xe0/0xe1) so their refunds route to the
        // reward account (the CE-3d case — its accounts are registered); a deregistered account → treasury.
        for ra in [0xe0u8, 0xe1] {
            acc.cert_state
                .delegation
                .registrations
                .insert(StakeCredential::KeyHash(Hash28([ra; 28])), Coin(2_000_000));
        }
        let mut gov = s3_empty_gov(6); // committee_quorum (2,3)
        gov.committee = [
            (StakeCredential::KeyHash(Hash28([0xC1; 28])), 1400u64),
            (StakeCredential::KeyHash(Hash28([0xC2; 28])), 1400),
            (StakeCredential::KeyHash(Hash28([0xC3; 28])), 1400),
        ]
        .into_iter()
        .collect();
        gov.proposals = proposals;
        acc.gov_state = Some(gov);
        acc
    }

    #[test]
    fn s4_boundary_refunds_expiring_unratifiable_and_carries_the_rest() {
        // 0x01: expiring (1339 < ending 1340) + 0 committee Yes -> refund to 0xe0; 0x02: non-expiring -> carried.
        let mut acc = s4_acc_with_gov(vec![
            s4_gas(0x01, s4_tw(), Vec::new(), 1339, 100_000_000_000, 0xe0),
            s4_gas(0x02, s4_tw(), Vec::new(), 1366, 100_000_000_000, 0xe1),
        ]);
        apply_gov_deposit_refunds(&mut acc, EpochNo(1341)).expect("clean refund");
        assert_eq!(
            acc.cert_state.delegation.rewards.get(&StakeCredential::KeyHash(Hash28([0xe0; 28]))),
            Some(&Coin(100_000_000_000)),
            "the expired proposal's deposit refunds to its return-address reward account",
        );
        let gov = acc.gov_state.as_ref().unwrap();
        assert_eq!(gov.proposals.len(), 1, "expired removed, non-expiring carried");
        assert_eq!(gov.proposals[0].action_id.tx_hash, Hash32([0x02; 32]));
    }

    #[test]
    fn s4_boundary_terminal_on_ratifiable_is_zero_mutation() {
        // A ratifiable proposal (2/3 committee Yes) alongside an expiring one -> the whole boundary terminals
        // with ZERO mutation (no refund credited, no proposal removed).
        let mut acc = s4_acc_with_gov(vec![
            s4_gas(0x01, s4_tw(), Vec::new(), 1339, 100_000_000_000, 0xe0),
            s4_gas(
                0x02, s4_tw(),
                vec![
                    (StakeCredential::KeyHash(Hash28([0xC1; 28])), Vote::Yes),
                    (StakeCredential::KeyHash(Hash28([0xC2; 28])), Vote::Yes),
                ],
                1339, 100_000_000_000, 0xe1,
            ),
        ]);
        let before = acc.clone();
        let err = apply_gov_deposit_refunds(&mut acc, EpochNo(1341)).unwrap_err();
        assert!(
            matches!(err, LedgerTransitionError::GovDepositRefundTerminal(_)),
            "a potentially-ratifiable proposal fails the boundary closed, got {err:?}"
        );
        assert_eq!(acc, before, "a terminal boundary makes ZERO mutation");
    }

    #[test]
    fn s4_boundary_refund_is_replay_equivalent() {
        let acc0 = s4_acc_with_gov(vec![
            s4_gas(0x03, s4_tw(), Vec::new(), 1339, 100_000_000_000, 0xe0),
            s4_gas(0x01, s4_tw(), Vec::new(), 1339, 50_000_000_000, 0xe0),
        ]);
        let (mut a, mut b) = (acc0.clone(), acc0);
        apply_gov_deposit_refunds(&mut a, EpochNo(1341)).expect("a");
        apply_gov_deposit_refunds(&mut b, EpochNo(1341)).expect("b");
        assert_eq!(a, b, "same prior + same boundary ⇒ identical accumulator");
        // both 0xe0 deposits (50k + 100k) accrued to the one return account.
        assert_eq!(
            a.cert_state.delegation.rewards.get(&StakeCredential::KeyHash(Hash28([0xe0; 28]))),
            Some(&Coin(150_000_000_000)),
        );
        assert!(a.gov_state.as_ref().unwrap().proposals.is_empty(), "both expired removed");
    }

    #[test]
    fn s4_boundary_deregistered_return_account_routes_deposit_to_treasury() {
        // The return account 0xee is NOT registered ⇒ the refund goes to TREASURY (cardano's unredeemed
        // path), never an orphan reward entry — matching POOLREAP + reward distribution.
        let mut acc =
            s4_acc_with_gov(vec![s4_gas(0x01, s4_tw(), Vec::new(), 1339, 100_000_000_000, 0xee)]);
        let treasury_before = acc.epoch_state.treasury.0;
        apply_gov_deposit_refunds(&mut acc, EpochNo(1341)).expect("clean refund");
        assert_eq!(
            acc.epoch_state.treasury.0,
            treasury_before + 100_000_000_000,
            "an unregistered return account's deposit goes to treasury",
        );
        assert!(
            acc.cert_state
                .delegation
                .rewards
                .get(&StakeCredential::KeyHash(Hash28([0xee; 28])))
                .is_none(),
            "no orphan reward entry for the unregistered account",
        );
        assert!(
            acc.gov_state.as_ref().unwrap().proposals.is_empty(),
            "the expired proposal is removed regardless of refund destination",
        );
    }
}
