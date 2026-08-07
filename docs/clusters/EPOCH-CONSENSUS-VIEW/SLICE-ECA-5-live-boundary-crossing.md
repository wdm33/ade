# SLICE ECA-5 — Live epoch-boundary crossing (forecast-horizon extension coupled to authority promotion)

> Working/planning doc for the EPOCH-CONSENSUS-VIEW cluster. Competition-secret: keep UNTRACKED
> (consistent with the other EVIEW SLICE-* working docs + EPOCH-CONTINUITY-ACTIVATION-plan.md). Do
> NOT commit. The load-bearing facts mirror memory: `project_epoch_consensus_view`,
> `project_eca_continuity_progress`, `project_native_mithril_judge_flow`,
> `feedback_no_semantic_activation_gate`, `feedback_durable_state_is_replay_authority`.

---

## 0. CONTEXT — read this first (so we do not rebuild what exists or re-derive what's settled)

### 0.1 The live finding that motivates this slice (2026-06-25)

A following node, bootstrapped from a fresh native Mithril snapshot and following the live preview
chain (`ade node run --network preview --bootstrap-mithril … --snapshot-dir … --data-dir … --peer …`),
caught up from the snapshot anchor (slot 115676685, epoch 1338) and **failed at the epoch boundary**:

```
relay run-loop sync step failed (Pump("Receive(Validity(Header(
  OutsideForecastRange(OutsideForecastRange { requested: SlotNo(115689630), horizon: SlotNo(115689600) })
)))")); failing closed (no skip-past, no fallback).
```

`115689600 = 1339 × 86400` — the **epoch 1338→1339 boundary**. The node followed and admitted blocks
correctly *within* epoch 1338 (the leader-VRF η0 fix landed in `886ca138`); it cannot forecast a
header in epoch 1339. This is the user's stated goal ("continuous use that crosses epoch
boundaries") surfacing on its own.

### 0.2 What ALREADY EXISTS (on origin/main — do NOT rebuild it)

The EVIEW/EpochConsensusView machinery is implemented and hermetically green:

- **The self-derived next-epoch VIEW** — `EpochConsensusView` carries `stake_by_pool` +
  per-pool VRF keyhashes + `total_active_stake` + nonce + a protocol-params/profile commitment, all
  folded into a `canonical_hash`. Leadership-complete (`DC-EVIEW-12`); projects exclusively to
  `PoolDistrView` with no live CertState join (`DC-EPOCH-12`). [ECA-0a `ad704f86`, ECA-0b `4614e977`]
- **Automatic activation** — no `EVIEW_ACTIVATION_ARMED`, no flag, no special build. Activation is
  gated only by boundary detection + the idempotent `promoted()` check + the predicate
  (`DC-EPOCH-13`). [ECA-1 `a17c7aab`]
- **The v4 seed sidecar** — `SeedEpochConsensusInputs` schema v4 persists ALL 10 activation-input
  fields incl. `genesis_hash` + `protocol_params_hash` + `active_slots_coeff` + the venue geometry
  (`epoch_start_slot` + `epoch_length_slots`). Recovered from the STORE, never the restart CLI.
  [ECA-2-pre `124c87da`, `DC-CINPUT-06`; geometry `DC-CINPUT-05`]
- **The atomic authority + recovery** — `ActiveEpochAuthority` (`crates/ade_node/src/epoch_activation.rs`):
  ONE owned holder, the SOLE view source for both header validation and leadership, promoted IN PLACE
  at the boundary (Seed → Promoted) by the same path that derives the bound candidate, verifies the
  predicate, and writes the durable activation WAL record before the promotion is visible; warm-start
  re-derives + recovers the same authority or halts. [ECA-2-3-4, `DC-EPOCH-14`, enforced]
- **The live reduced-UTxO checkpoint** — built at bootstrap, advanced per durable admit, reorg
  re-materialize, fail-closed readiness; `derive_stake_by_pool` PROVEN against cardano-node on the
  real 3M-entry preview UTxO (reduction 100% exact, ADE1 exact). [-mat 1–4]

### 0.3 The cardano-ledger proof obligations — RESOLVED (do NOT re-derive)

Researched against cardano-ledger SHA 226b002d / core 1.20.0.0 (= node 11.0.1), cited in
`EPOCH-CONTINUITY-ACTIVATION-plan.md` §3:

- **Snapshot timing**: SNAP runs FIRST, POOLREAP SECOND. The mark for target epoch T = `psStakePools`
  at END of T-2, captured PRE-POOLREAP. Leadership reads the SET snapshot (2-epoch lag → `DC-EPOCH-08`,
  `LEADERSHIP_SNAPSHOT_LAG_EPOCHS`).
- **Inclusion**: pools with `spssNumDelegators > 0` (delegator COUNT, not amount). Stake delegated to
  an unregistered pool is SILENTLY DROPPED (never error/placeholder VRF).
- **VRF source**: `spssVrf = spsVrf`, frozen WITH the stake in the SAME snapshot build (never a live
  read). "Capture VRF from the bound ledger state that produced the stake" is PROVEN.

### 0.4 The GAP this slice closes (newly surfaced — NOT covered by ECA-1..4)

ECA-1..4 promote the **leadership VIEW** (the `authority`) across the boundary. They do **not**
extend the **forecast horizon** (the `EraSchedule`). The two are decoupled today:

- `make_node_schedule` (`crates/ade_node/src/node_lifecycle.rs:3170`) builds **one** `EraSummary` for
  the current (seed) epoch with `start_slot = epoch_start_slot`, `safe_zone_slots = epoch_length_slots`.
  Horizon (`check_forecast_horizon`, `era_schedule.rs:199`) = `start_slot + safe_zone` = the **next
  boundary** exactly. `recovered_node_schedule` (`:3215`) builds it once from the sidecar's CURRENT
  epoch; the relay loop takes `era_schedule: &EraSchedule` **by reference** and never rebuilds it.
- `validate_and_apply_header` (`crates/ade_core/src/consensus/header_validate.rs:71`) checks the
  forecast horizon as **Step 1**, BEFORE the leader view is consulted. So a post-boundary header is
  rejected at the forecast gate even though the authority HAS promoted the N+1 view.

Net: the view crosses; the schedule does not; header validation rejects N+1 at Step 1 →
`OutsideForecastRange`. **The fix is to extend the schedule with the N+1 `EraSummary` atomically with
(and only with) the authority promotion.**

### 0.5 Two wiring prerequisites also surfaced (bounded, part of this slice)

- **Magic source** — the EVIEW input construction (`node_lifecycle.rs:721`) does
  `cli.network_magic.ok_or(MissingFlag("--network-magic"))`. The judge route is `--network preview`
  with NO `--network-magic`, so the EVIEW inputs are unreachable there. Resolve magic from the
  committed `--network` profile (the exact pattern already applied to the live pump in `b0bbaaf5`:
  `resolve_network_magic(cli)`).
- **Relay-only path** — the forge-OFF branch passes `None` for `eview_activation`
  (`node_lifecycle.rs` ~line 575, "forge-off: no EVIEW activation"). My recent relay-only follow port
  (`b0bbaaf5`) made the no-keys path FOLLOW; it must also carry the EVIEW activation so a no-keys node
  crosses the boundary. The forge-ON branch already constructs `eview_inputs` (`:715`); mirror it,
  with the replay-scratch path under `--data-dir` (a warm start has no `--snapshot-dir`).

---

## 0.7 LIVE FINDING (2026-06-25) — activation-trigger DEADLOCK (blocks the live crossing)

The DC-EPOCH-15 forecast-extension mechanism is implemented + hermetically proven (08fa37f6), but the
FIRST live crossing still failed `Header(OutsideForecastRange { requested 115689630, horizon
115689600 })`. Root cause is UPSTREAM of the forecast extension -- a deadlock in the EVIEW activation
TRIGGER:

- `maybe_activate_first_boundary` (epoch_wire.rs:422-430) detects "the seed epoch's window is complete"
  as "the durable tip located into a LATER epoch" (`tip_epoch > seed_epoch`).
- But a FOLLOWER's durable tip can never enter N+1: the first N+1 header fails the forecast gate
  (validate_and_apply_header Step 1, OutsideForecastRange) and is NOT admitted, so the tip stays at the
  last N block. The activation runs POST-admit (maybe_activate_epoch_boundary, after run_node_sync), so
  it never sees a tip in N+1 -> never fires -> the forecast never extends -> the next N+1 header fails
  again. Circular.

So DC-EPOCH-15 (the coupling) is correct but INSUFFICIENT: the crossing also needs the authority to
PROMOTE at the boundary, which the tip-in-N+1 trigger prevents for a follower. The forge-ON producer
likely never hit this (it forges, and historic live runs used legacy inputs); the FOLLOWER path
exercises the boundary trigger for the first time.

FIX DIRECTION (needs confirmation before building): detect "window complete" from the INCOMING N+1
header -- the first block at slot >= the boundary arrives exactly when the durable tip is the last N
block, so the N window IS complete at that instant -- and activate + extend the forecast BEFORE
validating that header. This reworks the activation TRIGGER (boundary detection) + the validation flow
(activate-before-validate the boundary-crossing header). Open questions:
- (a) is "the first header at slot >= boundary, tip still in N" a sound + replay-deterministic "N window
  complete" signal (equivalent to the current tip-in-N+1 signal)?
- (b) the EVIEW window replay still sources the complete seed epoch N (compute_first_window_bounds is
  unchanged) -- only the TRIGGER moves earlier;
- (c) BLUE/RED: the trigger is RED shell (the relay loop / run_node_sync), strictly BEFORE the BLUE
  validate_and_apply_header; no BLUE change.

## 0.8 IMPLEMENTATION PLAN — authority-preparation seam (user direction 2026-06-25)

Frame: fix the authority-preparation boundary for the FIRST post-boundary candidate (not "activate from
a header"). The header supplies only the candidate slot + parent; the promoted view derives EXCLUSIVELY
from durable state. Investigated pieces (all confirmed):
- structural decode = `ade_ledger::block_validity::decode_block(bytes)` -> `{header_input.slot, prev_hash, block_hash}` (BLUE, pub, reusable from ade_node).
- derivation to REUSE unchanged = `epoch_wire::compute_first_window_bounds` + `try_activate_at_boundary`
  (extract durable N window -> verify readiness -> materialize -> activate_at_boundary; derives N+1 from
  the N reduced checkpoint + canonical N window blocks + v4 sidecar geometry + cert-state/lineage).
- the ONLY thing wrong = the TRIGGER (`maybe_activate_first_boundary` epoch_wire.rs:428 `tip_epoch <= seed_epoch -> no-op`).

Steps (in order):
1. **The seam (BLUE determine+derive / GREEN promote)** — new `prepare_authority_for_candidate_slot(inputs:
   &EviewActivationInputs, era_schedule: &EraSchedule, durable_tip_slot, durable_tip_hash, candidate_slot,
   candidate_parent, live: &ReducedUtxoCheckpoint, chaindb, active_view: &mut ActiveEpochAuthority,
   wal_write) -> Result<bool, ActivationError>` in epoch_wire.rs. Ok(true)=promoted (caller extends
   schedule); Ok(false)=no-op; Err=TERMINAL guard. Logic:
   - no-op if already promoted; if durable tip not in seed_epoch N; if candidate in N (not a boundary).
   - TERMINAL guards: candidate epoch > N+1 (skip beyond N+1) -> CandidateSlotSkipsBoundary;
     candidate_parent != durable_tip_hash -> CandidateParentNotDurableTip.
   - else (tip in N + candidate EXACTLY N+1 + parent==tip => the N window is complete): reuse
     compute_first_window_bounds + try_activate_at_boundary over the durable tip as the selected point.
     (window-incomplete / readiness / activation-record-conflict guards already live INSIDE
     try_activate_at_boundary; schedule-geometry match is by-construction in extend_schedule_to_epoch.)
2. **run_node_sync (RED)** — signature gains `eview: Option<&EviewActivationInputs>, reduced_checkpoint:
   Option<&ReducedUtxoCheckpoint>, authority: &mut ActiveEpochAuthority, era_schedule: &mut EraSchedule`
   (drop the separate `ledger_view` -- derive `authority.ledger_view()` per block AFTER the seam). Per
   block: `decode_block` -> if eview+checkpoint present: seam -> if promoted: `extend_schedule_to_epoch`
   -> `ledger_view = authority.ledger_view()` -> `pump_block(.., &era_schedule, ledger_view)`. Seam runs
   strictly BEFORE pump_block (before validation).
3. **extend_schedule_to_epoch** -> `pub(crate)` (so node_sync can call it); update the gate grep.
4. **Relay loop** — pass the new args to run_node_sync; REMOVE the deadlocked post-admit
   `maybe_activate_epoch_boundary` (tip-in-N+1). Warm-start `maybe_recover_promoted_authority` +
   extend_schedule_to_epoch STAYS.
5. **Call sites** — 1 prod (node_lifecycle ~2110) + 6 node_sync tests (pass `None` eview + a seed
   `ActiveEpochAuthority::seed(view)` + `&mut sched`).
6. **New ActivationError variants** — CandidateSlotSkipsBoundary, CandidateParentNotDurableTip.
7. **Tests (6)** — first valid N+1 header promotes+validates; malformed N+1 header cannot promote; forked
   N+1 (parent != tip) -> terminal; far-future (skip beyond N+1) -> terminal; warm-restart before+after
   promotion rebuilds identical schedule; producer eligibility for N+1 uses the SAME prepared authority.
8. **Re-run CE-ECA-5** live (snapshot @1338 -> cross into 1339).

## 0.9 LIVE DIAGNOSIS (2026-06-25) — seam PROVEN; snapshot-lag mismatch -> bootstrap bridge needed

The authority-preparation seam is implemented + PROVEN live (it promotes + extends the forecast). FIVE
layered fixes, each confirmed by the next-deeper failure:
1. trigger deadlock -> `prepare_authority_for_candidate_slot` (epoch_wire);
2. `reduced_checkpoint` None on FirstRun (bound at node_lifecycle:461 BEFORE the bootstrap built it) -> re-open post-bootstrap;
3. readiness lag (checkpoint advanced only post-`run_node_sync`) -> seam advances to durable tip + `seal_window_tail` (advance the marker through the block-free tail to source_window_end);
4. seam IGNORED the `NotYet` outcome -> `Ok(active_view.is_promoted())`;
5. `NotYet(WrongSelectedPoint)` -> `selected_point.slot = bounds.source_window_end` (candidate.source_point = Point{source_window_end, lineage_pin}; epoch_candidate.rs:112).
Result: `outcome=Promoted`, OutsideForecastRange GONE. BUT `new_ep=1340`: the seed-epoch window-replay targets
source+2 (LEADERSHIP_SNAPSHOT_LAG_EPOCHS=2; epoch_wire.rs:290 "source+2"). The node's FIRST boundary is
seed->seed+1 (1338->1339); the 1339 leadership is the bootstrap's MARK snapshot (epoch.rs:100 `set = prev mark`),
NOT the replay. So the 1339 leader-VRF is checked against the 1340 view -> `VrfCert(VerificationFailed)`.
USER DIRECTION: use a distinct BOOTSTRAP BRIDGE for the first crossing (the +2 replay is correct for steady state).

## 0.10 BOOTSTRAP BRIDGE PLAN (user direction 2026-06-25)

Data flow CONFIRMED: MARK snapshot (epoch.rs `StakeSnapshot.pool_stakes`: pool->coin) + pool_params (vrf_keyhash)
= the seed+1 leadership; the 1339 nonce = the candidate nonce (PraosState); MIRROR the native SET extraction
(ledgerdb_state.rs / `read_pool_params`). The 6 pieces:
- **(a)** derive seed+1 leadership at bootstrap (MARK + pool_params -> PoolEntry{active_stake, vrf_keyhash}; total; epoch=seed+1; nonce=candidate) -- mirror the SET extraction.
- **(b)** persist as the bootstrap bridge -- sidecar **v5** (SeedEpochConsensusInputs + next-epoch block: `next_epoch_nonce`, `next_epoch_total_active_stake`, `next_epoch_pool_distribution`; target_epoch=epoch_no+1 derived; venue commitments shared). `SEED_CINPUT_SCHEMA_VERSION` 4->5; FIELDS_OUTER 11->14; encode/decode + all construction sites.
- **(c)** explicit selector: `target == seed_epoch+1` -> bootstrap bridge view; `target >= seed_epoch+2` -> replay-derived `EpochConsensusView`. No wall-clock / peer / mutable-pool input.
- **(d)** first-boundary seam: when target==seed+1, promote the BRIDGE view directly (NOT `compute_first_window_bounds`/`try_activate_at_boundary`, which is +2); extend forecast through seed+1; validate. Keep the +2 replay path for target>=seed+2.
- **(e)** warm-start: restore the bridge (before N+1) / the promoted authority+schedule (after N+1) from the durable sidecar.
- **(f)** 6 acceptance tests (bootstrap@N -> first N+1 header promotes the bridge; N+1 VRF/header validates; N+1->N+2 promotes the replay-derived authority; restart before N+1 restores the bridge; restart after N+1 restores the promoted authority/schedule; no path uses the +2 replay view for N+1) + the LIVE proof.

DO NOT revert the ECA5DIAG diagnostics (node_sync.rs gate marker; epoch_wire.rs seam-enter/bounds/activate markers)
until the bridge lands + all six acceptance points pass live. The 5 seam fixes + `seal_window_tail` + the
diagnostics stay (uncommitted) until then.

## 0.11 CONFIRMED v5 SIDECAR SHAPE + HARD RULES (user 2026-06-25)

```
BootstrapNextEpochAuthority {
  target_epoch            = seed_epoch + 1,
  source_kind             = ImportedMarkSnapshot,   // closed discriminant -- distinguishes from replay-derived
  source_point,                                     // the seed/bootstrap point (binds the bridge to the snapshot)
  source_profile_commitment,                        // network/profile commitment (genesis_hash, magic)
  pool_distribution,                                // MARK-derived pool -> {active_stake, vrf_keyhash}
  total_active_stake,
  epoch_nonce,                                      // the seed+1 leadership nonce (eta0)
  protocol_params_commitment,                       // params + asc commitment
  canonical_commitment,                             // blake2b over the canonical encoding (binding + verification)
}
```

HARD RULES (DC-EPOCH-15 strengthening):
- MARK-derived bridge usable ONLY for seed_epoch+1; replay-derived authority ONLY for seed_epoch+2 and later.
- NO fallback from a missing MARK to nesPd / window-replay / an external oracle.
- sidecar v4 FAILS CLOSED for a native bootstrap that requires the bridge (a TYPED schema-upgrade requirement,
  not corruption); never silently invent defaults.
- warm-start MUST reconstruct the SAME selector decision from the durable v5 bytes (byte-identical bridge).

Build order (proceed straight through; the CBOR decoder is necessary compatibility work, not a pause point):
(a) MARK decode + cross-check -> (b) v5 sidecar (this object) -> (c) selector -> (d) first-boundary prep path
-> (e) warm-start recovery -> (f) hermetic tests + live first-boundary proof.

## 0.12 PIECE (a) DONE + VERIFIED LIVE (2026-06-25)

The MARK decode is implemented in `ledgerdb_state.rs` (`decode_native_nonutxo_state` now decodes
`EpochState.snapshots[0] = ssStakeMark`; the field was previously skipped). The 2-map SnapShot layout was
UNKNOWN -- determined by live probing:
- `SnapShot = array(2)[ ssStake : map(StakeCredential -> [Coin, PoolId]) {stake+delegation COMBINED},
  ssPoolParams : map(PoolId -> PoolParams) {NON-standard, uint-first -- NOT vrf-first like the cert-state} ]`.
- `read_mark_snapshot_pool_distr`: map0 aggregation (`calculatePoolDistr`: sum each delegated credential's
  coin into its pool); the per-pool VRF is taken from the durable cert-state registrations (map1 is skipped --
  its encoding differs); a staked pool with no cert registration (retired) is OMITTED (never a fabricated VRF).
- `NativeSnapshotNonUtxoState.mark_pool_distr` added; the mark<->nesPd VRF cross-check (terminal mismatch).

LIVE VERIFIED (preprod snapshot @1338): `mark_pools=626` (seed+1 leadership), `nes_pools=659` (seed),
`overlap=626` (ALL mark pools in nesPd -> cross-check passes), mark_total ~1.67B ADA, nes_total ~1.673B ADA --
exactly right for adjacent-epoch leadership (same pool set minus churn, near-identical stake). The bootstrap
proceeds past the decode. Temp probes (`describe_deep`, `describe_item`, `ECA5MARK`) remain; remove with the
seam diagnostics at the live proof.

NEXT (b): the v5 sidecar carrying `BootstrapNextEpochAuthority`, built from `s1a.mark_pool_distr` + the
candidate nonce + the bindings, assembled in `mithril_native_assembly.rs::native_consensus_inputs` (mirror the
`s1a.pool_distr` seed-view path with `s1a.mark_pool_distr`).

## 0.13 SLICE DONE + LIVE-PROVEN + PUSHED (2026-06-25, `26565bec`, origin/main)

Pieces (b)-(f) all landed. The full bridge is implemented and the FIRST-BOUNDARY CROSSING is proven live.

- (b) v5 persisted format: `bootstrap_bridge.rs` `BootstrapNextEpochAuthority` + version-gated byte-canonical
  CBOR + canonical commitment (5 codec tests); anchor-fp-keyed `SnapshotStore::put/get_bootstrap_next_epoch_authority`
  (mod + in-memory + persistent). Built + persisted at FirstRun (`native_firstrun.rs`).
- (c)/(d) selector + seam: `prepare_authority_for_candidate_slot` promotes the bridge for seed+1 (REQUIRED, no
  fallback to the +2 replay; `BridgeMissing`/`BridgeEpochMismatch`/`BridgeProjection` terminal). The bound view =
  `EpochConsensusView::bind(epoch=seed+1, phase=Mark, nonce=bridge eta0, stake/vrf from pool_distribution)` ->
  `to_pool_distr_view` -> WAL-record -> `promote`. The relay loop SYNCS `chain_dep.epoch_nonce` to the bridge eta0
  (header_validate reads the chain_dep, NOT the view).
- (e) warm-start: the relay loop reads the bridge from `SnapshotStore` on BOTH the first-run and warm-start
  `EviewActivationInputs` paths (`node_lifecycle.rs`).
- (f) THE eta0 ROOT CAUSE: eta0(N+1) = `blake2b(candidate || lastEpochBlock)` (extraEntropy NeutralNonce on
  preview). `extract_praos_nonces_v2` had **evolving and candidate SWAPPED** -- the candidate is `tail[0]`, not
  `tail[2]`. Reverse-engineered the live node's `eta0(1339)` (`cardano-cli query protocol-state` -> epochNonce
  `911cc60d…fe00`, confirmed verifies a real proof), then scanned every nonce pattern: `blake2b(tail[0] || tail[4])`
  reproduces it. Record order = `[candidate, epoch, evolving, lab, lastEpochBlock]`.

LIVE PROOF (preview, docker `cardano-node-preview`): bootstrap @ epoch 1338 -> promote the **626-pool** MARK bridge
at slot 115689630 -> **admit 1388 epoch-1339 blocks** (slots 115689630..115736952, ~47k slots into 1339) with
**ZERO VrfCert and no fail-close**, clean exit. All 54 ade_ledger+ade_node test groups green (updated
`ledgerdb_nonutxo_hermetic`: a valid array(4) mark fixture + corrected nonce assertions). All ECA5 diagnostics
removed; the two dead probe fns removed.

REMAINING: cluster-close (DC-EPOCH-15 declared -> enforced). CLOSURE SCOPE (user-directed): wording
limited to "forecast extension occurs only after durable authority promotion, AND a Mithril-started
FOLLOWER crosses its first boundary without forecast/VRF failure." Do NOT let closure imply "continuous
producer proven." This is a FOLLOWER compatibility proof; the producer claim is out of scope.

Two DISTINCT next operational proofs (neither proven here):
1. Run keyed ADE1 through a boundary and forge an ELECTED N+1 block + a real Haskell node ADOPTS it
   (bounty-shaped production/adoption).
2. Cross the FOLLOWING boundary, where the REPLAY-derived seed+2 authority -- NOT the imported MARK
   bridge -- must validate + forge (the self-sustaining steady-state pipeline after bootstrap).

## 2. Slice Header

### Slice Name
ECA-5 — Live epoch-boundary crossing: forecast-horizon extension coupled to authority promotion.

### Cluster
EPOCH-CONSENSUS-VIEW (EPOCH-CONTINUITY-ACTIVATION).

### Status
Proposed.

### Cluster Exit Criteria Addressed
- [ ] **CE-ECA-5 (live)**: A following Ade node (relay-only, `--network preview --data-dir … --peer …`,
  no operator keys, no manual intervention, no restart, no external epoch-stake import) crosses a real
  preview epoch boundary N→N+1 — admits block N+1 using its self-derived N+1 view, the forecast
  extended in step — and reaches `agreement` with the peer past the boundary. (The live half;
  operator-run, not a hermetic CI test.)
- [ ] **CE-ECA-5-hermetic**: forecast extension is coupled to authority promotion and replay-deterministic
  (the §12 tests + gate).

### Slice Dependencies
- ECA-0a/0b (leadership-complete `EpochConsensusView` with VRF) — merged.
- ECA-1 (automatic activation, no semantic gate) — merged.
- ECA-2-pre (v4 sidecar) — merged.
- ECA-2-3-4 (atomic authority transition + recovery, `DC-EPOCH-14`) — merged.
- The relay-only follow port (`b0bbaaf5`) + the native-Mithril continuity (`54833173`) + the η0 decode
  fix (`886ca138`) — merged; this slice runs on a node that already follows within an epoch.

---

## 3. Implementation Instruction (AI)

> READ THIS SECTION FIRST.

Implement exactly what §5/§9/§10 specify; no extra refactors. **IMPLEMENT INLINE** (no
slice-implementer/fork/Agent delegation — user standing directive). The forecast extension must be a
deterministic projection of canonical durable state, coupled to the existing `DC-EPOCH-14` authority
promotion — never a flag, never a wall-clock, never a second activation surface. Commit messages carry
the `Co-Authored-By: Claude Opus 4.8 (1M context)` trailer (this repo's override) via `git commit -F`;
NO AI attribution in source/doc bodies. §12 is the only proof of completion. Respect §14/§15.

---

## 4. Intent

Make it impossible for a following node to validate (or reject) a header in epoch N+1 against a
forecast horizon that disagrees with its activated epoch authority: the `EraSchedule`'s forecast
horizon extends to include epoch N+1 **if and only if** the `ActiveEpochAuthority` has promoted the
N+1 view, as one replay-deterministic transition over canonical durable state.

---

## 5. Scope

- **Modules / crates:**
  - `crates/ade_node/src/node_lifecycle.rs` — `run_relay_loop_with_sched`: hold the `EraSchedule` as an
    OWNED, mutable value (not `&EraSchedule`); extend it with the N+1 `EraSummary` at the SAME point the
    authority promotes (`maybe_activate_epoch_boundary` / the recovery re-fire). The forge-OFF branch
    constructs + passes `eview_inputs` (mirror forge-ON). `resolve_network_magic(cli)` for the EVIEW
    input magic on both branches; the replay-scratch path resolves under `--data-dir` when
    `--snapshot-dir` is absent.
  - `crates/ade_core/src/consensus/era_schedule.rs` — if needed, a deterministic `EraSchedule`
    constructor/extension that appends a Conway `EraSummary` for epoch N+1 (`start_slot` = the boundary,
    `start_epoch` = N+1, `epoch_length_slots`/`safe_zone_slots` from the durable geometry). Prefer reusing
    `EraSchedule::new` with the 2-summary vector over a new mutator if it keeps the type immutable.
- **State machines affected:** the relay loop's epoch-authority transition (`DC-EPOCH-14`) — the schedule
  extension joins it as the second half of the SAME transition. No new state machine.
- **Persistence impact:** none new. The activation WAL record (`EpochConsensusViewActivated`) already
  records the promotion; the schedule extension is DERIVED from it + the durable geometry, not separately
  persisted. (Open design Q — see §17 — confirm the extended schedule is recomputed identically on
  warm-start from the activation record, so no new WAL field is required.)
- **Network-visible impact:** none.

**Out of scope:** changing the leader-VRF / view derivation (exists); new protocol versions; multi-era
(non-Conway) HFC transitions; performance.

---

## 6. Execution Boundary

- **BLUE:** `crates/ade_core/src/consensus/era_schedule.rs` (`EraSchedule`, `EraSummary`,
  `check_forecast_horizon`, `locate`) — deterministic forecast authority. Any extension constructor here
  must be pure (no I/O, no clock, no float, no HashMap).
- **GREEN / neither:** `crates/ade_node/src/node_lifecycle.rs` (`run_relay_loop_with_sched`) — `ade_node`
  is neither BLUE nor RED; it orchestrates. It must feed the BLUE `EraSchedule` only DETERMINISTIC inputs
  (the durable geometry + the promoted authority's epoch), never a clock/peer/CLI value.
- **RED:** none introduced. (`--network-magic` resolution reads the committed profile, not the network.)

Rule check: the schedule extension's inputs (boundary slot, N+1 epoch, epoch_length) are all functions
of the durable sidecar geometry + the promoted authority — no RED ingress into the BLUE forecast.

---

## 7. Invariants Preserved

- **DC-EPOCH-14** (atomic authority transition + recovery) — the schedule extension must occur within the
  SAME transition, never as a separate visible step; a failure after the predicate stays terminal.
- **DC-EPOCH-13** (no semantic activation gate) — no flag may decide whether the schedule extends.
- **DC-CINPUT-05** (venue geometry = durable replay authority) — the N+1 `EraSummary` geometry comes from
  the durable sidecar, never the restart CLI/genesis; no hardcoded epoch length.
- **DC-EVIEW-12 / DC-EPOCH-12** (leadership-complete view; exclusive projection) — unchanged.
- **DC-EPOCH-08 / DC-EPOCH-06** (SET snapshot lag = 2; recovery exactness) — unchanged.
- Deterministic replay equivalence; canonical encoding of persisted bytes; single authoritative
  view+schedule per epoch; fail-closed header validation.

---

## 8. Invariants Strengthened or Introduced

- **NEW — DC-EPOCH-15 (forecast horizon ⟺ authority promotion coupling).** The relay loop's
  `EraSchedule` forecast horizon extends past an epoch boundary N→N+1 **if and only if** the
  `ActiveEpochAuthority` has promoted the N+1 view, as one transition: (a) the schedule never forecasts
  into an epoch whose view is not promoted (a header at an unpromoted N+1 slot fails closed
  `OutsideForecastRange`, never accepted on a stale horizon); (b) once promoted, the same epoch's
  headers pass the forecast gate AND resolve leadership/validation through the promoted view (cross-
  consumer identity with `DC-EPOCH-14`); (c) the extended schedule is a replay-deterministic projection
  of the durable geometry + the activation record — every replay of the same durable inputs yields the
  same horizon; (d) the N+1 `EraSummary` geometry derives ONLY from the durable sidecar (`DC-CINPUT-05`),
  never the restart CLI.
- **STRENGTHENED — DC-EPOCH-14** (`strengthened_in += EPOCH-CONTINUITY-ACTIVATION-ECA-5`): the atomic
  promotion now also extends the forecast horizon in the same transition; cross-consumer identity covers
  validation forecast + leader view + schedule.
- **STRENGTHENED — DC-CINPUT-05** (`strengthened_in += …-ECA-5`): `make_node_schedule` is now invoked to
  EXTEND across boundaries from durable geometry, not only to build the seed-epoch single summary.

---

## 9. Design Summary

1. **Own the schedule.** `run_relay_loop_with_sched` takes/holds the `EraSchedule` as an owned `mut`
   value (today `&EraSchedule`). The initial value is the current single-epoch schedule from
   `recovered_node_schedule` (unchanged).
2. **Couple extension to promotion.** At the exact site the authority promotes
   (`maybe_activate_epoch_boundary` succeeds → Seed→Promoted for epoch T) AND on the warm-start recovery
   re-fire (`maybe_recover_promoted_authority` recovers a promoted authority), rebuild the owned schedule
   to include the `EraSummary` for epoch T (`start_slot` = T's epoch_start = the boundary, `start_epoch`
   = T, `epoch_length_slots`/`safe_zone_slots` from the durable sidecar geometry). After rebuild,
   `check_forecast_horizon` admits T's slots (horizon moves to the T+1 boundary) and `locate` maps T's
   slots to T's summary.
3. **Single derivation, no flag.** The promoted epoch T and the geometry are the only inputs; identical
   on live activation and warm-start recovery (so a restart after the boundary recomputes the same
   extended schedule — no new WAL field; see §17 open Q).
4. **Wire the relay-only path.** Mirror the forge-ON `eview_inputs` construction into the forge-OFF
   branch; resolve magic via `resolve_network_magic(cli)` (both branches); replay-scratch path under
   `--data-dir` when `--snapshot-dir` is absent.

---

## 10. Changes Introduced

### Types
- Possibly a pure `EraSchedule` extension constructor in `ade_core` (e.g. `with_appended_era` or reuse
  `EraSchedule::new(anchor, start, vec![seed_era, next_era])`). No new persisted type.

### State Transitions
- The `DC-EPOCH-14` boundary transition gains a second deterministic effect: extend the owned schedule.
  Modified, not new.

### Persistence
- None new (the extension is derived; confirm no WAL field needed in §17).

### Removal / Refactors
- `run_relay_loop_with_sched` signature: `era_schedule: &EraSchedule` → owned `mut` (call sites updated).

---

## 11. Replay, Crash, and Epoch Validation

- **Replay tests:** same durable inputs (sidecar geometry + activation record) → identical extended
  schedule + identical forecast verdicts across runs.
- **Crash/restart:** a restart AFTER the boundary recovers the promoted authority (`DC-EPOCH-14` f) AND
  recomputes the same extended schedule (so the recovered node validates N+1 headers identically). A
  crash BEFORE the durable activation WAL keeps Seed AND the un-extended schedule (N+1 still fails closed).
- **Epoch boundary:** the live CE-ECA-5 crossing on preview; plus a hermetic boundary-replay test in
  `ade_testkit` (`replay_cmd = cargo test -p ade_testkit`).

---

## 12. Mechanical Acceptance Criteria

- [ ] `forecast_extends_only_on_promotion`: a header at an N+1 slot fails `OutsideForecastRange` before
  promotion; after the authority promotes N+1, the same slot passes the forecast gate. (BLUE/relay unit.)
- [ ] `extended_schedule_is_replay_deterministic`: same durable geometry + activation record → byte/shape-
  identical extended `EraSchedule` (horizon + locate) across two builds.
- [ ] `warm_start_recovers_extended_schedule`: restart after the boundary recovers the promoted authority
  AND recomputes the extended schedule; an N+1 header validates post-restart.
- [ ] `forge_off_relay_constructs_eview_inputs`: with the durable reduced checkpoint + v4 sidecar + tip
  present, the forge-OFF path builds `eview_inputs` (not `None`) and resolves magic from the `--network`
  profile (no `--network-magic`).
- [ ] `cross_consumer_forecast_and_view_agree_at_n1`: at an N+1 slot, the forecast horizon and the
  leader/validation view both resolve epoch N+1 with the same active-view canonical hash (extends the
  DC-EPOCH-14 cross-consumer test).
- [ ] CI gate `ci/ci_check_eview_forecast_crossing.sh`: asserts the schedule extension is reachable only
  through the promotion path (negative-grep: no flag/independent schedule-extend surface; the relay loop
  owns the schedule; no hardcoded epoch length).
- [ ] All four crates green; existing EVIEW gates (`ci_check_eview_atomic_authority.sh`,
  `ci_check_eview_automatic_activation.sh`) still pass.

**Live (CE-ECA-5, operator-run, not CI):** a real preview N→N+1 crossing with `agreement` past the
boundary, captured to a transcript (outside the repo).

---

## 13. Failure Modes

- **Unpromoted N+1 header** → `OutsideForecastRange` (fail-closed; the schedule must NOT pre-extend).
  Recoverable: the node keeps following within N until promotion fires.
- **Promotion succeeds but schedule extension fails** (e.g. degenerate geometry) → terminal halt
  (consistent with DC-EPOCH-14's post-predicate terminal posture). Fail-fast; replay-affecting.
- **Restart recomputes a different extended schedule** than the pre-crash one → terminal (recovery
  exactness, DC-EPOCH-06). Fail-fast.
- **Sidecar geometry absent under a live feed** → already `FeedMissingRecoveredConsensusInputs`
  (DC-CINPUT-05); unchanged.

---

## 14. Hard Prohibitions

Inherited cluster prohibitions apply. Slice-specific:
- No flag / build switch / env var deciding whether the schedule extends (DC-EPOCH-13).
- No wall-clock, rand, float, or HashMap in the BLUE `EraSchedule` extension.
- No hardcoded epoch length / venue switch / fallback (DC-CINPUT-05); geometry from the sidecar only.
- No second activation surface: the schedule extension MUST flow from the existing `DC-EPOCH-14`
  promotion, not a parallel detector.
- No live CertState read to build the N+1 view (DC-EPOCH-12); the view is the bound candidate.
- No TODO/placeholder/deferred validation in the relay loop or `era_schedule`.

---

## 15. Explicit Non-Goals

- MUST NOT re-derive or change the `EpochConsensusView` / VRF capture (ECA-0a/0b own it).
- MUST NOT introduce new protocol versions or non-Conway HFC era transitions.
- MUST NOT add configuration switches or optimize performance.
- MUST NOT alter the snapshot-timing / inclusion rules (settled, §0.3).
- MUST NOT make the live CE pass via any manual intervention (no flag-flip-at-boundary).

---

## 16. Completion Checklist

- [ ] Extended schedule is replay-derivable from durable state (no new persisted authority).
- [ ] Any new persisted data (if §17 forces a WAL field) is canonically encoded.
- [ ] All failure modes deterministic; post-predicate failures terminal.
- [ ] No TODO/placeholder in BLUE/relay paths.
- [ ] CI enforces DC-EPOCH-15 (`ci_check_eview_forecast_crossing.sh`); registry entry added.
- [ ] Replay-equivalence + warm-start recovery tests pass.
- [ ] Live CE-ECA-5 transcript captured (preview N→N+1, agreement past the boundary).

---

## 17. Decisions (user 2026-06-25) + Open Questions

**DECIDED — binding (user 2026-06-25):**
- **Q#1 RESOLVED — DERIVE, do not persist.** The extended schedule is derived state; its authoritative
  inputs already exist: durable activation record + recovered promoted EpochConsensusView + v4 sidecar
  geometry + committed network profile = the exact post-boundary schedule. A second WAL field would
  create redundant authority + a new mismatch class. Warm-start reconstructs it deterministically and
  REQUIRES byte-identical schedule identity to the pre-restart one.
- **ATOMIC-SWAP IMPLEMENTATION RULE.** promotion durable -> rebuild the immutable schedule including the
  N+1 summary -> atomically replace the relay-loop-owned schedule -> only THEN allow post-boundary
  validation/forging. NO mutable shared reference that can leave validation using the old horizon after
  authority promotion.
- **TRACKING.** This detailed doc stays UNTRACKED (competition secrecy is intentional). The committed
  normative record is DC-EPOCH-15 (invariant registry) + `SLICE-ECA-5-summary.md`. Venue details,
  timing, commands, and live-capture procedures live ONLY here (untracked).

**HARD PROOF OBLIGATIONS — before any live crossing:**
1. **EraSchedule adjacency (verify, do not assume).** `check_forecast_horizon` reads only `eras.last()`;
   appending the N+1 summary moves the horizon to the N+2 boundary and `locate` must map both N and N+1.
   Verify `EraSchedule::new`/`locate` actually support adjacent same-era (Conway) consecutive-epoch
   summaries — add a dedicated unit test that fails if the constructor/locate assumed a single summary.
2. **Warm-start in BOTH states.** Test warm-start BEFORE promotion (schedule stays single-epoch; an N+1
   header still fails closed) AND AFTER promotion (schedule reconstructed with N+1), proving the
   reconstructed forecast boundary EXACTLY matches the live (pre-restart) one in each state.

**Remaining open:**

3. **Multi-boundary catch-up.** A snapshot far behind the tip crosses MORE than one boundary during
   catch-up. Each crossing must promote + extend in turn (N→N+1, then N+1→N+2…). Confirm the loop
   promotes/extends per boundary (the SET-lag means the window for N+2 is derivable once at N+1). The live
   CE should ideally cross at least one boundary; a multi-boundary catch-up is the stronger proof.
4. **Snapshot-retention interaction.** A long catch-up accumulates 601 MB ledger snapshots (separate
   known gap; see `project_native_mithril_judge_flow`). A near-boundary snapshot crosses before the disk
   fills (as in the live finding). Note for the CE venue choice; the retention fix is a separate slice.

---

## 18. Authority Reminder

This is a planning aid. Correctness is defined by the normative docs + CI. Normative documents and CI
enforcement are authoritative; on conflict, they win.
