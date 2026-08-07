# EPOCH-CONTINUITY-ACTIVATION — plan / session handoff

Status as of 2026-06-21. Local working doc (do NOT commit — competition secrecy). The
load-bearing facts are mirrored in memory: `project_epoch_consensus_view`,
`project_eview_wire_design`, `feedback_no_semantic_activation_gate`.

---

## 1. Where we are (all on origin/main, HEAD `124c87da`)

The **entire EVIEW path is implemented and hermetically green**, but the live activation is
deliberately INERT behind a dev scaffold that must now be removed (ECA-1).

- **ECA-0a DONE+pushed (`ad704f86`)** — cardano-faithful pool lifecycle in the reduced window
  (`future_pools` re-reg staging, `apply_pool_reap` adopt/reap/clear-delegations at each crossed
  boundary, 6-field cert codec); DC-EVIEW-13; `drive_window_consensus_inputs` surfaces the window-end
  `{stake, pool_params}` (the mark, pre-POOLREAP).
- **ECA-0b DONE+pushed (`4614e977`)** — the leadership-complete, self-contained `EpochConsensusView`
  (stake + per-pool effective VRF + FULL profile commitment, all folded into `canonical_hash`) + the
  exclusive `to_pool_distr_view` projection (fail-closed; no live CertState join, no unbound param);
  the cardano `numDelegators>0` aggregate fix. DC-EVIEW-12 + DC-EPOCH-12 enforced; DC-EVIEW-05/07
  strengthened; IDD + security reviews clean (domain-tag + layout hardening applied). **ECA-0 (the
  prerequisite leadership-completeness gap) is CLOSED.**
- **ECA-1 DONE+pushed (`a17c7aab`)** — removed the `EVIEW_ACTIVATION_ARMED` semantic gate (the const +
  the `armed` param through `EviewActivationInputs::maybe_activate` → `maybe_activate_first_boundary` +
  the `if !armed` short-circuit + the node_lifecycle call). NO replacement flag. Activation is now
  AUTOMATIC: gated only by `era_schedule.locate` boundary detection + the idempotent `promoted()` check +
  the predicate. Live path BYTE-IDENTICAL (relay loop still binds `eview_activation=None` → inert by
  un-wired inputs = ECA-2, never a flag; non-EVIEW keys on the `(Some,Some)` canonical-state guard). New
  invariant **DC-EPOCH-13** (no semantic activation gate) + gate `ci_check_eview_automatic_activation.sh`
  (negative-greps the flag/armed-param/if-!armed gone from crates/); DC-EPOCH-11 strengthened; IDD review
  PASS + security review clean; ade_node lib 366 green. **ECA-2 is next.**
  - PRE-EXISTING DEBT (NOT ECA-1): 3 stale gates `ci_check_eview_{candidate,reduced_utxo_checkpoint,
    view_binding}.sh` FAIL — ECA-0a renamed `drive_window_aggregate`→`drive_window_consensus_inputs`;
    -mat-2c/-wire wired the checkpoint + EpochConsensusView live-but-gated (breaking their "observe-only /
    no-live-wiring" asserts of DC-EVIEW-04/07/10). Registry-vs-gate drift owed reconciliation at
    cluster-close or a hygiene pass (confirmed pre-existing: 15 `ReducedUtxoCheckpoint` refs at HEAD,
    `epoch_candidate.rs` untouched by ECA-1).
- **ECA-2-pre DONE+pushed (`124c87da`; DC-CINPUT-06)** — the ECA-2 investigation found 8/10
  `EviewActivationInputs` fields already recoverable, but `genesis_hash` + `protocol_params_hash`
  were NOT persisted (import-bundle only). Per the user directive, extended `SeedEpochConsensusInputs`
  **v3→v4** to persist them (the durable consensus profile): codec v4 (VERSION=4, FIELDS_OUTER=11);
  the merge copies both from the bundle verbatim; pre-v4 stores fail closed with the TYPED
  `ConsensusInputsSchemaUnsupported{found,required}` at BOTH decode sites (bootstrap + live
  `warm_start_recovery` + an operator reimport message in `report()`); manifest `seed_hash` + WAL
  provenance cover the hashes transitively. DC-CINPUT-05 strengthened; gate; IDD PASS (1 WARN =
  live-path shadowing, FIXED) + security clean; all 4 crates green. **The ECA-2 blocker is RESOLVED —
  all 10 fields now recoverable from canonical durable state.** ECA-2 is next (see §4; the field
  sourcing is confirmed, and the ECA-3 observe-only coupling nuance is noted in memory).

- **-mat 1–4 + shadow** — the live reduced-UTxO checkpoint (build at bootstrap, advance per
  durable admit, reorg re-materialize, fail-closed readiness gate, `derive_stake_by_pool`).
  The live derive is **PROVEN against cardano-node** on the real 3M-entry preview UTxO
  (reduction 100% exact, ADE1 exact). Commits `0ac92cba … bfa0b54a`.
- **-wire 1 … 3b-2** — the dual-path activation: durable window-replay is the SOLE authoritative
  candidate; the live checkpoint is a readiness witness; `try_activate_at_boundary` composes
  extract → readiness → fresh-materialize → `activate_at_boundary` (predicate → WAL → promote).
  Wired into the relay loop but **doubly gated off**. Commits `e14a0e15 … a50a3ee8`.
- Gate `ci/ci_check_eview_live_checkpoint.sh` (19 checks) green; coherence 405 rules; ade_node
  lib 364 / ade_runtime 425 green; the live follow/forge path is **byte-identical**.

---

## 2. The correction that redefines "done" (user, 2026-06-21)

`EVIEW_ACTIVATION_ARMED: bool = false` (+ the `armed` param, + a "controlled-build flip at the
boundary") is a **SEMANTIC GATE** — a runtime/build switch deciding WHETHER consensus
transitions occur. Forbidden as production (closed semantic surfaces; replay must decide
identically). It was acceptable ONLY as temporary scaffold to prove byte-identical behavior. It
must be **removed by the final slice, not preserved**.

A continuous producer activates **automatically** when the deterministic predicate passes over
canonical durable state — no human, no flag, no special build. See
`feedback_no_semantic_activation_gate`.

**Success is NOT** "did someone flip the flag right at one boundary." **Success IS** Ade
remaining continuously operational across 1335→1336 — admits N+1, uses its self-derived view,
forge-ready — with no manual intervention, no restart, no external epoch-stake import.

---

## 3. THE BLOCKER surfaced while starting ECA-3 — a real completeness gap

`PoolDistrView` (serves BOTH header VRF validation AND leadership) needs per-pool
`PoolEntry { active_stake: u64, vrf_keyhash: Hash32 }`
(`crates/ade_ledger/src/consensus_view.rs`). The seed view fills these from the imported
`pool_vrf_keyhashes` map.

**But the EVIEW candidate `EpochConsensusView` carries only `stake_by_pool: BTreeMap<PoolId,
Coin>` — NO VRF keyhashes** (`crates/ade_ledger/src/reduced_epoch_view.rs`). The shadow proof
validated the *stake*; it never touched VRF keyhashes because the candidate has none. So a
`PromotedView` cannot be turned into a working `PoolDistrView` from the candidate alone. **ECA-3
is blocked on this.**

The keys exist in the cert state: `DelegationState`'s pool map → `PoolParams { vrf_hash }`
(`crates/ade_ledger/src/delegation.rs:75,97-99`). The derive advances the cert state over the
window, so **at the window end (the source boundary) the cert state holds exactly the pool
registrations the SET snapshot freezes.**

### DESIGN DECISION — CONFIRMED by user 2026-06-21 (option 1: freeze VRF in the candidate)

**Locked.** The promoted epoch view is SELF-CONTAINED: stake distribution + per-pool VRF
keyhashes + total active stake + epoch/source bindings + a canonical hash over ALL of it.
**Hard rule (the user's words):** for target epoch N+1, every `PoolDistrView` entry derives ONLY
from the canonically-bound `EpochConsensusView` for N+1 — NO post-candidate read of live
`CertState` may supply a missing VRF key (or stake). At derivation, capture pool VRF hashes from
the SAME replayed/bound ledger state that produced the stake snapshot, and bind that source point.

**VRF-timing nuance (load-bearing, user):** do NOT call these "the SET-snapshot value" without
proving the ledger timing — stake has snapshot-lag semantics; the pool registration / VRF
lifecycle may have its own epoch-state timing. PROOF OBLIGATION (in flight, research agent
`af19d4fb`): confirm via cardano-ledger that `PoolDistr`'s VRF keyhash is sourced from the SAME
snapshot's pool-params (frozen WITH the stake), not a separately-timed registration — only then
is "capture from the exact bound ledger state that produced the stake snapshot" proven.

**Inclusion-rule PROOF OBLIGATION (in flight, same agent):** does cardano-ledger `PoolDistr`
include zero-stake registered pools, or only nonzero-stake pools? And a credential delegated to a
pool ABSENT from snapshot pool-params (retired) — DROPPED, errored, or zero-VRF? This sets the
candidate's pool set + the fail-closed-vs-drop semantics (a wrong choice is a LIVE divergence:
halt where cardano-node accepts, or vice versa). Leaning (to confirm, not assume): include exactly
the pools in `stake_by_pool`; match cardano's drop-vs-error for stake-without-params.

**RESOLVED — core (research af19d4fb, cardano-ledger SHA 226b002d / core 1.20.0.0 = node 11.0.1, cited):**
- INCLUSION = pools with `spssNumDelegators > 0` (delegator COUNT, not stake-amount; a pool whose
  delegators sum to 0 is STILL included). `calculatePoolDistr'` SnapShots.hs#L449-465.
- MISSING PARAMS = stake delegated to an UNREGISTERED pool is SILENTLY DROPPED at snapshot-build
  (never error/placeholder VRF). Ade: DROP a delegated-but-unregistered pool, do NOT fail-closed.
  [CORRECTS my earlier fail-closed leaning.]
- VRF source/timing = `spssVrf = spsVrf`, frozen WITH the stake in the SAME snapshot build (never a
  live read) → "capture VRF from the same bound ledger state that produced the stake" PROVEN.
  Leadership reads the SET snapshot (2-epoch lag) — corroborates DC-EPOCH-08 LAG=2.
- Candidate construction (cardano-faithful): kept = {p ∈ aggregated-stake : p ∈ cert_state.pool.pools};
  stake = aggregate|kept; VRF = params[p].vrf_hash; total = Σ kept (recomputed); the two maps share
  keys BY CONSTRUCTION (= DC-EVIEW-12). Open (low-risk): solver-pin inferred; Conway SNAP override not
  exhaustively grepped (era-polymorphic); the 0-stake-with-delegators edge (count-not-amount).

**SHARPENED follow-up (user 2026-06-21; research agent a5f9da, PENDING) — ECA-0 HELD until it lands.**
User's precise gate: "at the exact snapshot/target-epoch rule, which pool registration/RETIREMENT
state + VRF key is authoritative per included pool?" Core answer is ~right, but the EXACT ORDERING is
open: in the EPOCH transition, is SNAP (mark capture) before/after POOLREAP (retirement removal) and
the fPParams→pParams adoption (re-registration / new VRF)? That decides whether a pool
retiring/re-registering AT the source boundary is in the snapshot — and thus whether Ade's window-end
cert state needs a SEPARATE epoch-boundary reap/adoption step (today: per-block cert processing only).
**RESOLVED (research a5f9da, SHA 226b002d, all PROVED): SNAP runs FIRST, POOLREAP SECOND.** Both
`fPParams→pParams` ADOPTION and retirement REMOVAL live inside POOLREAP (after SNAP). So the mark
captured at end of source epoch N = PRE-POOLREAP `psStakePools`:
- a pool retiring AT that boundary (retire-epoch==N+1) is STILL INCLUDED (SNAP precedes the reap);
- a same-epoch re-registration's NEW VRF is NOT yet visible (it's in futureParams; the mark carries
  the OLD VRF — the new VRF governs leadership only +1 epoch later);
- a newly-registered pool IS present (dropped from PoolDistr only if 0 delegators).
- Net authoritative rule for target T: `psStakePools` at END of T-2, captured pre-POOLREAP.
- Window-replay mapping: per-block cert processing == pre-POOLREAP at the SNAP instant (so the FINAL
  mark capture is per-block-only = CORRECT), BUT POOLREAP (adopt futures + reap retired) MUST be
  applied at every CROSSED boundary during replay (else the start-of-epoch state is wrong).

**ADE GAPS VERIFIED (code-read) — bigger than "add a VRF field":**
- G1 (VRF version): `apply_pool_registration` (delegation.rs:247) OVERWRITES `pool.pools[id]`
  IMMEDIATELY on re-registration (incl. the new VRF). Ade has NO `futureStakePoolParams` (grep empty).
  → for a pool that ROTATES its VRF during the source epoch, Ade's mark VRF = NEW, cardano = OLD →
  Ade would REJECT that pool's valid blocks for ~2 epochs. Rare (needs an actual VRF-key change) but a
  true consensus divergence; IN SCOPE for DC-EVIEW-12's "era-correct VRF keyhash".
- G2 (boundary reap in the window): `drive_window_aggregate` advances cert state per-block ONLY
  (advance_cert_state = process_block_certificates); it does NOT call the epoch-boundary POOLREAP
  (rules.rs:1144 reaps, but only on the LIVE apply_epoch_boundary path, NOT the EVIEW window). → a
  window that CROSSES a boundary keeps retired pools + misses future adoption. (Does NOT bite a
  single-epoch window at the final mark instant — per-block-only = correct there.)
- G3 (already correct): the FIRST self-derive (source = the bootstrap epoch, single-epoch window) is
  CORRECT modulo G1 → the immediate ECA-5 first-boundary proof is achievable.

**SCOPE DECISION (awaiting user).** To be CORRECT for DC-EVIEW-12's "era-correct VRF", ECA-0 needs
cert-state LIFECYCLE FIDELITY (G1+G2) — bigger than the view-shape change; it touches the canonical
`PoolState` + the cert-state codec (EVIEW/window path + tests only; the LIVE follow/forge path is
unaffected — track_utxo=false never processes certs). RECOMMEND correctness-first:
- **ECA-0a (lifecycle):** add `future_pools` to PoolState (re-reg stages there, keeping the OLD VRF in
  `pools` until adoption; first-reg still inserts to `pools`); apply POOLREAP (adopt futures + reap
  retired) at each boundary CROSSED inside the window driver; emit the mark from PRE-POOLREAP `pools`.
- **ECA-0b (the view):** `pool_vrf_keyhashes` + `protocol_params_commitment` + canonical hash + the
  cardano-faithful capture (kept = delegated ∩ registered, drop unregistered, total recomputed) +
  DC-EVIEW-12 / DC-EPOCH-12.
- **Alt (faster to first proof):** ECA-0b-narrow now (correct for non-VRF-rotating pools + the
  single-epoch first-derive) + ECA-0a as a documented follow-on for sustained continuity + VRF-rotation
  robustness. Build the moment the scope is chosen.

**ASC REFINEMENT (user 2026-06-21, refinement 2 — supersedes finding #4's "supply ASC at projection"):**
Do NOT put ASC's VALUE in the view (not snapshot state), but DO bind a PROTOCOL/GENESIS-PARAMETERS
COMMITMENT (covering ASC) into the candidate. EpochConsensusView salient set = {stake_by_pool,
pool_vrf_keyhashes, total_active_stake, nonce, source_point, epoch, snapshot_phase,
PROTOCOL_PARAMS_COMMITMENT} (+ existing network_magic/era/checkpoint_commitment). Projection rule:
`PoolDistrView = promoted view + protocol params RESOLVED FROM THE BOUND PROFILE`, with NO live
CertState read AND NO UNBOUND protocol-param read — ASC reaches the projection ONLY via the bound
commitment. So ECA-0 adds TWO bindings: `pool_vrf_keyhashes` + `protocol_params_commitment` (both
folded into canonical_hash). DC-EVIEW-12 covers leadership-completeness (VRF); DC-EPOCH-12 extends to
"no unbound protocol-param read." Build TODO: define the PP-commitment canonical content (≥ ASC; reuse
an existing Ade genesis/profile commitment if one exists) + the projection-time commitment check.

**Invariant IDs — the user's proposed DC-EPOCH-08/09/10 COLLIDE with assigned+pushed IDs**
(08 = ActivationSourceWindow, 09 = derive_candidate, 10 = activate_at_boundary; 11 = mat). IDs are
append-only, never reused. IDD-legal mapping of the user's three:
- user #2 (canonical hash covers stake+VRF+point+epoch+nonce+phase) = a STRENGTHENING of the
  existing **DC-EVIEW-07** → `strengthened_in += EPOCH-CONTINUITY-ACTIVATION`; statement extended
  to include `pool_vrf_keyhashes`. NOT a new ID.
- user #1 (leadership-complete view: every included pool has stake AND an era-correct VRF key) =
  new **DC-EVIEW-12**.
- user #3 (promoted `PoolDistrView` derived EXCLUSIVELY from the promoted view; no live cert-state
  join) = new **DC-EPOCH-12**.

### The two options (RECORD)
- **CHOSEN (option 1) — extend the candidate.** Add `pool_vrf_keyhashes: BTreeMap<PoolId, Hash32>` to
  `EpochConsensusView`, captured from the cert state **at the window end** (frozen-at-boundary,
  with the stake), folded into `canonical_hash` (the view's identity). Exact + replay-
  deterministic. The rebind then builds the complete `PoolDistrView`.
- **REJECTED — join the LIVE cert state at rebind.** The live cert state is at the TIP, not the
  source boundary; a mid-epoch pool VRF re-registration would make leadership use the wrong key.
  This is exactly the "view drifts from the frozen snapshot" class the snapshot-alignment
  correction killed. Do NOT do this.

The recommended fix re-touches: `EpochConsensusView` fields + `canonical_hash` + `bind` +
`verify_canonical_hash`; `derive_candidate` / `drive_window_aggregate` (capture the cert-state
pool params at the window end); the WAL `EpochConsensusViewActivated` record's
`stake_view_canonical_hash`; and the shadow proof should be re-run to also confirm the VRF
keyhashes match cardano-cli's set-snapshot pool VRF hashes (the existing off-repo harness
`~/.cardano-c2-preview/eview-oracle-evidence/eview_checkpoint_shadow.rs` already loads
`pool_vrf` data — extend it).

---

## 4. THE SLICE — EPOCH-CONTINUITY-ACTIVATION (build ECA-1..4 now, hermetic; reserve the boundary for ECA-5)

User directive: build + hermetically prove ECA-1..4 NOW; do not wait to land code at the wall.
Sequence — now: build+prove ECA-1..4. Before boundary: fresh Preview bootstrap/readiness run,
confirm candidate generation + deterministic inputs. At boundary: run the UNCHANGED production
binary, observe automatic activation + continuity, collect ECA-5 evidence.

### ECA-0 (prerequisite, the gap above): make the candidate leadership-complete
Extend `EpochConsensusView` with `pool_vrf_keyhashes` captured at the window end; thread through
the derive + canonical hash + WAL record; extend the shadow harness to confirm VRF keyhashes.

**ECA-0 verified findings (2026-06-21, code-read):**
1. The cert-state CODEC is COMPLETE — `snapshot/cert_state.rs` encode/decode_cert_state serialize
   the full 5-field CertState incl. `pools` (PoolId→PoolParams incl. `vrf_hash`). (Memory said
   "DelegationState only" — imprecise; no codec gap. Bootstrap import CAN carry pre-bootstrap VRF.)
2. `advance_cert_state` carries pool params forward over the window (process_block_certificates).
3. GAP to fix: `drive_window_aggregate` (reduced_window_driver.rs:72) DISCARDS the window-end
   `state.cert_state.pool` — returns only StakeByPool. ECA-0 surfaces the VRF map from the final
   cert state (refactor: a richer driver return; keep `drive_window_aggregate` for its 2 tests).
   Build `pool_vrf_keyhashes` for exactly the `stake.pool_stakes` pools (inclusion rule = research).
4. ASC stays a GENESIS param supplied at PoolDistrView projection — NOT bound in the view (cardano-
   ledger: ASC is a Shelley-genesis global, not snapshot state; user's self-contained list =
   stake+VRF+total+bindings, no ASC). DC-EPOCH-12 worded "no live CERT-STATE read" (ASC=genesis OK).
   [flagged to user; proceed unless corrected]
5. OPERATIONAL (ECA-5, not ECA-0): the bootstrap `.certstate` artifact must be PRODUCED with
   cardano-node `pstate`/`ssPoolParams` (pool params incl. VRF), not just `dstate` — else a
   PRE-bootstrap pool with stake lacks a VRF at the window end. Codec is ready; the extraction
   script must populate pools. (Hermetic ECA-0 tests construct cert states with pool params directly.)

**ECA-0a PROGRESS (2026-06-21, all hermetically green, ade_ledger 636 lib tests 0-fail):**
- DONE (ledger primitives): `PoolState.future_pools`; `apply_pool_registration` stages re-regs (active
  VRF unchanged until adoption), first-reg inserts immediately, re-reg cancels retiring;
  `apply_pool_reap(pool, entered_epoch)` adopt-futures-then-reap(==epoch); 6-field cert-state codec
  round-trips future_pools; 4 lifecycle tests (Pool.hs:266-310 + PoolReap.hs faithful).
- SCOPE CORRECTION (reverted, recorded in slice doc): do NOT add future_pools to `fingerprint.rs`
  (breaks warm-start on every existing store — live cert state is empty under track_utxo=false) NOR
  to the live boundary `apply_epoch_boundary_with_registrations` (inert + untested; EVIEW derives via
  the window driver). Both deferred to a track_utxo=true LIVE-LEDGER-APPLY slice. Bootstrap artifact
  still commits future_pools via the manifest cert_state_hash (the codec).
- REFINED 0a/0b SPLIT: the "richer driver return" (surface window-end pool params/VRF alongside
  StakeByPool) moves INTO ECA-0a — the boundary POOLREAP's effect is on POOL PARAMS (VRF), invisible
  in the delegation-based stake aggregate, so the driver must surface pool params to TEST it. ECA-0b
  then consumes them for the view.
- REMAINING ECA-0a (the driver): `drive_window_aggregate` gains `slots_per_epoch` + applies
  `apply_pool_reap` at each CROSSED boundary + returns `{stake, pool_params}` (mark = the params after
  the last block, before any further reap); thread `slots_per_epoch` through derive_candidate +
  activate_at_boundary + the epoch_wire caller (era_schedule.epoch_length_slots is available at
  epoch_wire.rs:295); multi-epoch tests (re-reg adopted + retirement reaped across a synthetic
  boundary; replay/crash/reorg); CI gate; registry DC-EVIEW-13 + DC-EVIEW-10 strengthening.
- SUSTAINED-CONTINUITY CAVEAT (flag for -wire, NOT ECA-0a): the live DC-EPOCH-08 window is SINGLE-epoch
  from a static bootstrap_state, so the driver's boundary POOLREAP is correct but DORMANT live, and the
  live flow is only correct for the FIRST self-derive (source = bootstrap epoch). Sustained multi-epoch
  continuity needs a -wire decision: model A (maintain the cert state continuously, advance+POOLREAP per
  boundary, symmetric with the reduced-UTXO -mat checkpoint) vs model B (re-derive from bootstrap over
  the full span with boundary POOLREAP each activation). The driver will be correct for either range.
- DELEGATION-CLEARING ADDED to ECA-0a (user-directed 2026-06-21, NOT deferred): apply_pool_reap now
  takes &mut CertState + clears delegations of reaped pools (cardano removeStakePoolDelegations,
  PoolReap.hs:221,239-241) + drops orphan futures; regression test
  `reaped_pool_delegation_cleared_no_silent_reattach_on_reregistration` (delegate C→P, reap P, P
  re-registers, C does NOT reattach). ade_ledger 637 lib tests green. Deposit/treasury/reward stay
  separately scoped WITH PROOF (SNAP-before-POOLREAP `Epoch.hs:292-297` + single-epoch window → no
  effect on the mark at the relevant phase; multi-epoch = CE-71-over-window + -wire).
- PRE-LIVE-BOUNDARY OBLIGATION (user step 4, before ECA-5): inspect the ACTUAL bootstrap `.certstate`
  artifact + REQUIRE all of {active pools, future_pools, retiring, delegations, rewards, VRF hashes}
  present AND bound to the same bootstrap manifest / chain point. Verify from the produced artifact /
  live run — NOT from the repo (the codec supports all six fields after ECA-0a).

**ECA-0b CORRECTIONS (user 2026-06-21, binding before ECA-0b proceeds):**
1. FIX aggregate_pool_stake to cardano's `numDelegators > 0` inclusion (a pool with >=1 delegator is
   included EVEN at 0 stake) — do NOT document the divergence as harmless. Remove the
   `if cred_total.0 == 0 { continue }` skip + `or_insert(0)` so every delegated-to pool gets an entry.
   STRENGTHENS DC-EVIEW-05 (count-not-amount). The registered-filter stays in ECA-0b's intersection.
   Update the `delegated_but_zero_stake_adds_no_pool_entry` test → now ADDS a 0 entry.
2. FULL consensus-profile commitment (NOT ASC-only): `protocol_params_commitment =
   blake2b(genesis_hash || protocol_params_hash || asc)` — reuses the canonical genesis_hash +
   protocol_params_hash (the full profile) + an explicit ASC so the projection VERIFIES the consumed
   param. The projection recomputes from (genesis_hash, pp_hash, asc), checks == bound, then uses asc.
   Binding shape: network, era/PV, target epoch, source point, checkpoint commitment, snapshot phase,
   nonce, stake distribution, VRF mapping, consensus-profile commitment, canonical hash.
3. CERT-STATE CONTINUITY MODEL = **A** (continuously-maintained durable cert-state checkpoint), chosen
   + specified NOW. Contract: advance per durable-admit (lockstep with the WAL + the reduced-UTXO -mat
   checkpoint); recovery = warm-start from the durable checkpoint; rollback = re-materialize the exact
   lineage from the sealed bootstrap baseline (cert deltas are not invertible — the -mat-3 pattern);
   storage = bounded redb (delegations/pools/rewards/future_pools/retiring); pruning = N/A (current
   maintained state; the bootstrap baseline is the reset anchor). Chosen over B (re-derive) for symmetry
   with the reduced-UTXO checkpoint + incremental/bounded cost (B from a fixed bootstrap = O(chain)).
   Built in ECA-3/-wire. NO continuous-operation claim until: (a) the first real boundary crosses
   without intervention AND (b) model A is tested across >=2 boundaries / recovery cycles (ECA-5 proves
   only the first crossing).

### ECA-1: remove the semantic gate
Delete `EVIEW_ACTIVATION_ARMED` + the `armed` param from `maybe_activate_first_boundary` /
`EviewActivationInputs::maybe_activate` / `maybe_activate_epoch_boundary`. NO equivalent flag
replaces it. Eligibility = the preview profile / protocol rules; activation = the deterministic
predicate. (epoch_wire.rs, node_lifecycle.rs.)

### ECA-2: deterministic inputs from canonical state
Replace the `None` binding (node_lifecycle.rs ~line 653, forge-on path) with a deterministic
construction: `seed_bootstrap_state = state.ledger.clone()`; `seed_point = state.tip`;
`seed_epoch`; `nonce = ade_core::consensus::Nonce(.0) -> Hash32` (NOT `.raw` — that's a different
Nonce type); `network_magic` = the RESOLVED magic (cli.network_magic is `Option<u32>` — find the
resolved u32, likely from the canonical consensus inputs `network_magic: u32`); the scratch
path. Non-EVIEW (no cert-state package) stays `None` = byte-identical, keyed on canonical state.

### ECA-3: the atomic authority transition (the CORE — design per the user)
Introduce ONE owned, epoch-versioned holder — NOT mutable state scattered through the loop:
```
enum ActiveEpochAuthority {
    SeedView   { epoch: EpochNo, view: PoolDistrView },
    PromotedView { epoch: EpochNo, view: PoolDistrView, activation_wal_binding: <id> },
}
```
- The relay loop OWNS it (replaces the borrowed `ledger_view: &dyn LedgerView` param at
  node_lifecycle.rs ~1474). Callers construct `SeedView` from the seed `PoolDistrView`.
- BOTH header validation (`run_node_sync`) AND leadership (the forge / `forge_epoch_admission`
  DC-EPOCH-03 wall) resolve `authority.ledger_view()` at each authoritative decision — never a
  retained stale borrow.
- At the wall: verify candidate → persist activation WAL → **atomically** replace the holder
  (Seed→Promoted) → all subsequent reads resolve Promoted. PoolDistrView built from the (now
  leadership-complete) candidate: `PoolEntry{active_stake = stake_by_pool[p], vrf_keyhash =
  pool_vrf_keyhashes[p]}` + ASC (protocol param, e.g. from the seed inputs `active_slots_coeff`)
  + epoch + nonce.
- The forge wall opens BY THE PREDICATE, not a flag.

### ECA-4: warm-start recovery
`recover_active_view` reconstructs the EXACT promoted `ActiveEpochAuthority` from the durable WAL
`EpochConsensusViewActivated` record (epoch_activation.rs already has `recover_active_view` for
`ActiveEpochView` — extend to rebuild the authority incl. the PoolDistrView). A restart mid-N+1
resumes on the self-derived view; no re-import, no re-arm.

### Required invariants (hermetic tests must cover)
- gate removed completely; no equivalent semantic flag.
- activation inputs only from canonical durable state.
- one owned authority supplies BOTH validation and leadership.
- durable WAL record PRECEDES publication.
- recovery reconstructs the exact promoted authority.
- OLD epoch view cannot serve N+1 after promotion; PROMOTED view cannot serve N before the wall.
- same-epoch behavior byte-identical.
- **simulated-boundary test** covering: follow, header validation, leadership, crash-before-WAL,
  crash-after-WAL, crash-after-publication.

### Claim allowed after ECA-1..4 (be precise):
"Production transition mechanism implemented and hermetically proven. Continuous Preview
operation remains UNPROVEN until ECA-5 crosses a real epoch boundary without manual
intervention."

### ECA-5 (at the real 1335→1336 boundary, NOT now)
Run the unchanged production binary; Ade stays alive through the wall, keeps admitting valid N+1
blocks, uses its self-derived active view, stays forge-ready, no manual intervention / restart /
external epoch-stake import. Plus the boundary-aligned stake oracle + leadership-schedule (lag=2)
confirmation. Live target: docker `cardano-node-preview` (magic 2), ADE1 = preview pool
`pool1gv25…`/hex `431549bf…`. Fresh oracle bundle `~/.cardano-c2-preview/ade-inputs-ep1335-fresh.json`.

---

## 5. Key code locations
- `crates/ade_node/src/epoch_wire.rs` — orchestration: EVIEW_ACTIVATION_ARMED (REMOVE),
  EviewActivationInputs (+maybe_activate), maybe_activate_first_boundary (drop `armed`),
  compute_first_window_bounds, try_activate_at_boundary, extract_source_window,
  verify_live_readiness, derive_authoritative_candidate.
- `crates/ade_node/src/node_lifecycle.rs` — the relay loop: `run_relay_loop_with_sched` (12th
  param `eview_activation`, the `ledger_view` param at ~1474 → owned authority),
  `maybe_activate_epoch_boundary` helper (~1456) called at ~1830 after the advance; the seed
  PoolDistrView at :601-613; the forge-on `eview_activation = None` binding at ~653 (→ ECA-2
  construction); the 5 callers (534/810/1376 + node_sync 2397/7183).
- `crates/ade_node/src/epoch_activate.rs` — `activate_at_boundary` (validate→derive→predicate→
  WAL→promote).
- `crates/ade_node/src/epoch_activation.rs` — `ActiveEpochView` (Seed|Promoted),
  `recover_active_view`, `activation_predicate`, the terminal `EpochViewActivationError`.
- `crates/ade_node/src/epoch_candidate.rs` — `derive_candidate` (extend to capture VRF keyhashes).
- `crates/ade_ledger/src/reduced_epoch_view.rs` — `EpochConsensusView` (extend +canonical_hash).
- `crates/ade_ledger/src/consensus_view.rs` — `PoolDistrView::new(epoch, total, asc, pools)`,
  `PoolEntry{active_stake, vrf_keyhash}`, `impl LedgerView for PoolDistrView`,
  `from_seed_epoch_consensus_inputs`.
- `crates/ade_ledger/src/delegation.rs` — `DelegationState`, the pool map → `PoolParams{vrf_hash}`.
- `crates/ade_runtime/src/consensus_inputs/{canonical,importer}.rs` — `network_magic: u32`,
  `pool_vrf_keyhashes`, `active_slots_coeff`.

## 6. Commit / process reminders
- Trailer `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>` via the Write
  tool then `git commit -F <file>` (NEVER `-m`; the scrubber blocks any command embedding the
  attribution pattern OR "Co-Authored-By: Claude …" in a grep).
- Per-slice security review (block on HIGH+) on every consensus-path change; extend the gate +
  the registry (DC-EPOCH-11 family) per slice; keep coherence green.
- Off-repo harnesses only (the shadow harness lives in `~/.cardano-c2-preview/eview-oracle-evidence/`).
