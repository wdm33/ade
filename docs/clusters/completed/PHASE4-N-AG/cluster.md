# PHASE4-N-AG — Single-producer loop continuation after follow-link EOF (DC-NODE-19)

> **Single-invariant cluster (3 code/evidence slices + 1 operator-gated live leg).** The CE-AF-6b
> follow-on deferred from PHASE4-N-AF / DC-NODE-18. Authority source:
> `docs/planning/single-producer-loop-continuation-after-feed-eof-invariants.md`;
> plan `docs/planning/phase4-n-ag-cluster-slice-plan.md`.
> **RUNG-1 ONLY · SINGLE-PRODUCER ONLY · POST-CERTIFICATE-PROMOTION ONLY · DC-CONS-03 UNTOUCHED · BLUE core unchanged.**

## §1 Primary property (DC-NODE-19 — new rule, declared → enforced on close)
In a declared single-producer venue (`VenueRole::SingleProducer`) that has **already** entered the
DC-NODE-18 extend state (`ForgeMode::SingleProducerExtendOwnDurableSpine`, reached only via DC-NODE-15
catch-up + an explicit venue-adoption-certificate promotion), a `LoopState::Ending` caused **solely by
structural feed EOF** (the Ade→relay follow link closing/draining) **must not** terminate the forge loop;
the loop continues forging successors on its own certified durable spine, fenced to the certified
single-producer run, until explicit shutdown / a fatal error / an existing BLUE forge-validity bound / a
competing chain. This **relocates the loop's termination authority off feed-liveness** onto explicit
operator shutdown / fatal error — the same move DC-NODE-09 made for the serve-listener lifetime — while
**preserving DC-NODE-05's deeper invariant** (`pump_block` remains the sole durable tip-advance authority;
available feed work still drains via `SyncOnce` before any `ForgeTick`). A loop-lifecycle refinement,
**not** a fork-choice change.

## §2 The finding (c2t7, what DC-NODE-18 left)
DC-NODE-18 made Ade *able* to forge the successor after certificate promotion (block 11 adopted → RED
adoption certificate → promote → block 12 via extend, no echo → adopted). But the c2t7 run **stopped at 2
blocks**: the relay's idle timeout EOF'd the Ade→relay follow link, `source.is_ended()` flipped
(`node_lifecycle.rs:1137` → `LoopState::Ending`), and the GREEN planner returns `HaltCleanly` on
`Ending` + `NoWorkReady` (`run_loop_planner.rs:145`, the N-F-E clause-3 "the loop never forges past the
feed" rule) → the loop `break`s. The forge loop treats the feed source as a **lifecycle authority**; in a
single-producer venue the relay is a pure follower and the feed going quiet is not a reason to stop
producing. (A *fatal* source error is distinct — it already exits via `Err`/fail-fast, never via `Ending`.)

## §3 The design (resolves OQ-19-1)
The GREEN `plan_loop_step` gains an **explicit 5th content-blind input** `VenuePolicy { HaltOnFeedEnd |
ContinueInSingleProducerExtend }` — the truth stated plainly, **no** RED `LoopState` re-derivation
(OQ-19-6) — making it a **32-case total table**. A GREEN projection `(VenueRole, ForgeMode) → VenuePolicy`
yields `ContinueInSingleProducerExtend` **only** when `venue == SingleProducer` **and** `mode ==
SingleProducerExtendOwnDurableSpine`; otherwise `HaltOnFeedEnd` (verbatim prior). On
`Ending` + `ContinueInSingleProducerExtend`: `NoWorkReady` + `Due` → `ForgeTick`, `NoWorkReady` + `NotDue`
→ `Idle`. Shutdown still wins (highest precedence); `WorkAvailable` still drains via `SyncOnce` first
(DC-NODE-05 clause-2 preserved).

The RED `run_relay_loop_with_sched` derives the policy from `(act.venue_role, act.forge_mode)` and threads
it in; the continuation reuses the DC-NODE-18 `single_producer_forge_decision` /
`SingleProducerFenceReason` fence (+ a **per-continuation venue-certificate re-validation** via
`read_adoption_cert`). Only a clean structural feed EOF continues; a fatal source failure still
`Err`/fail-fast.

**OQ-19-1 resolved (the `Idle`-under-dead-feed wakeup):** when the policy is
`ContinueInSingleProducerExtend` and the feed is ended, the `Idle` arm's cancellation-safe `select!` waits
on the **injected clock's next-tick timer** (so the next due forge slot wakes the loop) **or**
`shutdown.changed()` — **not** on the dead feed's `source.wait_ready()`. The feed EOF is no longer the
lifecycle authority; the injected clock remains the forge-cadence authority; operator shutdown still wins.
Deterministic under the injected clock schedule (T-REC-03): no busy-spin (it awaits the timer), no starved
cadence, no waiting forever on a dead feed. The default `HaltOnFeedEnd` path is byte-unchanged.

## §4 Scope of claim
- **Proves (CE-AF-6b):** a declared single-producer venue keeps forging its own certified durable spine
  **across a follow-link EOF** — sustained **past `k`**: several Ade-forged blocks become **immutable** in
  the relay's ChainDB / ImmutableDB evidence surface across ≥ 1 EOF; warm-start replay byte-identical. This
  is the leg DC-NODE-18 explicitly deferred.
- **Does NOT claim:** follow-link keep-alive / reconnect (OQ-KA — the cousin approach); multi-producer
  fork-choice (rung 2); the epoch-transition (rung-1 criterion 2 — off-epoch still fails closed via
  DC-EPOCH-03); preprod (rung 3); the recover-anchor k=0 edge.

## §5 TCB color map (FC/IS partition)
- **BLUE (unchanged — zero diff):** `ade_runtime::forward_sync::pump` (`pump_block`, DC-NODE-16);
  `ade_ledger` ledger/chain_dep/WAL; `forge_one_from_recovered` (DC-NODE-10/DC-CONS-24); chain selection
  (DC-CONS-03).
- **GREEN:** `ade_node::run_loop_planner` — `plan_loop_step` (+5th `VenuePolicy` input, 32-case total) +
  the `(VenueRole, ForgeMode) → VenuePolicy` projection (pure/total/deterministic);
  `ade_node::node_sync` — the **reused** DC-NODE-18 `single_producer_forge_decision` /
  `SingleProducerFenceReason` fence (unchanged).
- **RED:** `ade_node::node_lifecycle` — `run_relay_loop_with_sched` policy threading + the
  `Idle`-under-feed-end clock-tick wakeup + the per-continuation certificate re-validation.
- **No `cli.rs` change** — DC-NODE-18's `--single-producer-venue` already declares the venue.

## §6 Slices
| Slice | Scope | CE | Registry | TCB |
|---|---|---|---|---|
| **S1** | GREEN `plan_loop_step` 5th `VenuePolicy` input → 32-case total table + `(VenueRole,ForgeMode)→VenuePolicy` projection; default `HaltOnFeedEnd` reduces to the prior 16-case behavior; hermetic totality + reduction tests. | CE-AG-1 | DC-NODE-19 (declared; enforced at close) | GREEN |
| **S2** | RED `run_relay_loop_with_sched` threads the policy; continues past structural feed EOF **only after DC-NODE-18 certificate promotion**; default `Unknown` halts verbatim; fatal source failure still `Err`/fail-fast; reuse the DC-NODE-18 fence + per-continuation cert re-validation; `Idle`-under-dead-feed clock-tick wakeup (OQ-19-1); new gate `ci_check_single_producer_loop_continuation.sh`. | CE-AG-2, CE-AG-3 | — | RED (+reuse GREEN) |
| **S3** | Replay-equivalence over a post-feed-end chain: T-REC-03 two-runs + T-REC-05 kill/warm-start byte-identical; feed-end appends nothing to the WAL. | CE-AG-4 | strengthen T-REC-03, T-REC-05 | tests |
| **S4** | Operator-gated live acceptance = CE-AF-6b: rung1-auto C2-LOCAL — > k blocks settle into the relay ImmutableDB across ≥ 1 follow-link EOF, warm-start byte-identical; commit transcript. | CE-AG-5 | DC-NODE-19 → enforced (with S1–S3) | RED (evidence) |

## §7 Cluster Exit Criteria (mechanical except the operator-gated CE-AG-5)
- **CE-AG-1:** `plan_loop_step` total over **5** inputs — a 32-case exhaustive table test (no wildcard); a
  reduction test proves `VenuePolicy::HaltOnFeedEnd` reproduces the prior 16-case table exactly; the
  `(VenueRole,ForgeMode)→VenuePolicy` projection test; planner stays content-blind (no `SlotNo` in
  `plan_loop_step`). Tests (S1 adds): `plan_loop_step_venue_policy_table_is_total`,
  `plan_loop_step_halt_policy_reduces_to_prior_16`, `venue_policy_projection_is_continue_only_in_extend`.
- **CE-AG-2:** hermetic loop test — in a declared single-producer venue in the extend state, a structural
  feed EOF (`source.is_ended()`) does **not** halt (forges the next successor on the own durable spine); the
  default `Unknown` venue halts verbatim (`HaltCleanly`); a fatal source error returns `Err` (not
  continued). Tests (S2): `single_producer_extend_continues_past_feed_eof`,
  `unknown_venue_still_halts_on_feed_eof`, `fatal_source_error_fails_fast_not_continued`.
- **CE-AG-3:** the continuation **fails closed** on each of the **7** certified-run conditions (§8) by
  reusing the DC-NODE-18 fence + per-continuation cert re-validation (→ `SingleProducerFenceViolation` /
  no continuation); the `Idle`-under-dead-feed wait wakes on the clock tick / shutdown (no busy-spin).
  Tests (S2): `continuation_fails_closed_per_fence_reason` (each `SingleProducerFenceReason` + absent/
  malformed cert), `idle_under_dead_feed_wakes_on_clock_tick`; **gate**
  `ci_check_single_producer_loop_continuation.sh` (fences: explicit 5th `VenuePolicy` input + 32-case
  no-wildcard + default-`HaltCleanly`-preserved + fence-**reused**-not-reimplemented + no-BLUE-token in the
  changed region + clock-bounded `Idle`).
- **CE-AG-4:** replay-equivalence over a chain incl. post-feed-end forges — two clean runs byte-identical +
  kill/warm-start byte-identical (extending the existing `recover_follow_forge_two_runs_byte_identical` /
  `recover_follow_kill_warm_start_chains_from_ledger_fp` patterns to a post-EOF chain); the feed-end event
  appends nothing to the WAL. Tests (S3): `continue_past_eof_two_runs_byte_identical`,
  `continue_past_eof_kill_warm_start_recovers_byte_identical`, `feed_eof_appends_nothing_to_wal`.
- **CE-AG-5 (operator-gated; hard close gate = CE-AF-6b):** committed
  `docs/evidence/phase4-n-ag-loop-continuation.{md,jsonl}` — sustained > k Ade blocks settle into the
  relay's ImmutableDB across ≥ 1 follow-link EOF, warm-start replay byte-identical (rung1-auto, C2-LOCAL).
- **CE-AG-6 (close):** `cargo test -p ade_node` green + the new CI gate green; **DC-NODE-19 declared →
  enforced**; `strengthened_in += PHASE4-N-AG` on DC-NODE-05 / CN-NODE-02 / T-REC-03 / T-REC-05 /
  DC-NODE-18 (DC-CONS-03 untouched); all four grounding docs refreshed **including the CODEMAP + SEAMS
  deferred at the N-AF close** (baseline `f87d0056` → N-AG HEAD).

## §8 Forbidden during this cluster (hard boundaries — the fence)
- No RED `LoopState` re-derivation / "lie" to the planner — venue-awareness is an **explicit GREEN 5th
  input**; the feed-ended truth is stated plainly.
- No numeric "max blind forges" cap — continuation is fenced to the **certified single-producer run** (the
  7 conditions), not a magic number.
- No new durable tip-advance path — `pump_block` stays sole authority (DC-NODE-05 / DC-NODE-12 deeper
  invariant preserved); the forge advances no tip directly.
- No RED signal selecting / replacing / reordering / preferring chains — **DC-CONS-03 untouched**.
- No continuation outside the certified single-producer extend state — each of the **7 certified-run
  conditions fails closed**: (1) not `VenueRole::SingleProducer`; (2) not
  `ForgeMode::SingleProducerExtendOwnDurableSpine`; (3) operator shutdown requested; (4) existing
  forge-validity bounds fail (off-epoch DC-EPOCH-03 / beyond forecast horizon DC-CONS-09 / KES-period
  invalid); (5) a competing chain observed before EOF; (6) relay-producing evidence; (7) the venue/adoption
  certificate is absent or malformed.
- No continuation of a non-EOF `Ending` — a fatal source failure still `Err`/fail-fast.
- No busy-spin under a dead feed — the `Idle` wait is clock-tick / shutdown-bounded.
- **Zero BLUE change** (no new BLUE type/authority); no persisted / replay-visible state from the policy or
  the continuation.
- No keep-alive/reconnect (OQ-KA) / epoch-transition / multi-producer / preprod / k=0-recovery work; no new
  CLI flag.

## §9 Replay obligations
No new canonical type, authoritative state, WAL entry type, or replay-corpus entry. `VenuePolicy` + the
planner decision are RED/GREEN scheduling, **not** persisted/WAL'd → they cannot perturb the deterministic
surface. Each continued successor is a normal `AdmitBlock` via `pump_block` (existing, DC-NODE-12 /
DC-WAL-04). The obligation: **extend** T-REC-03 (loop-as-replay) + T-REC-05 (forged-chain warm-start) to a
chain including post-feed-end forges (CE-AG-4), and prove the feed-end event is **replay-neutral** (no WAL
append). No new durability law, no WAL/schema change. At close: `strengthened_in += PHASE4-N-AG` on
T-REC-03 + T-REC-05.

## §10 Invariants
- **Adds:** `DC-NODE-19` — declared → **enforced on close** (single-producer loop continuation after a
  structural feed EOF; CE-AG-1..5).
- **Preserves:** `DC-NODE-05` (deeper invariant — `pump_block` sole tip authority; feed work drains
  first), `DC-NODE-09` (the precedent — lifetime decoupled from feed-end), `DC-NODE-12`, `DC-NODE-15`,
  `DC-NODE-16`, `DC-NODE-18`, `DC-CONS-03` (**untouched**), `DC-CONS-24`, `DC-NODE-10`, `DC-EPOCH-03`,
  `DC-CONS-09`, `CN-NODE-02`, `T-REC-03`, `T-REC-05`, `DC-WAL-04`, `T-DET-01`.
- **Strengthens (at close):** `DC-NODE-05`, `CN-NODE-02`, `T-REC-03`, `T-REC-05`, `DC-NODE-18`
  (`strengthened_in += PHASE4-N-AG`). `DC-CONS-03` explicitly untouched.

## §11 Close record
**SUPERSEDED-CLOSE 2026-06-08** (partial close — hermetic core complete, live CE re-homed).

- **Hermetic core (CE-AG-1..4) COMPLETE:** S1 GREEN planner VenuePolicy (`b9ef6e69`), S2 RED loop
  continuation past feed-EOF (`46098c8c`), S3 replay-equivalence (`a65e2039`). The 32-case planner
  totality, the 7-condition fail-closed continuation, and the post-feed-end replay-equivalence all
  landed and pass.
- **CE-AG-5 (live sustained proof = CE-AF-6b) SUPERSEDED / RE-HOMED to PHASE4-N-AH CE-AH-6.** The
  DC-NODE-18 cert-promotion mechanism CE-AG-5 relied on to ENTER the extend state was retired by
  DC-NODE-20 (the run-4 finding: the operator cert had leaked from evidence into forge-loop
  authority). The live sustained-past-k proof now runs on the DC-NODE-20 local-tip path and is **MET by
  PHASE4-N-AH run-4** (`docs/evidence/phase4-n-ah-ce-ah-6-close.{md,jsonl}`).
- **DC-NODE-19 stays `declared`/partial — NOT flipped to enforced here.** The planned CE-AG-6 flip was
  gated on the live CE-AG-5, which is superseded. DC-NODE-19 (continue-past-EOF in the extend state)
  remains valid hermetic infrastructure; it is **strengthened by PHASE4-N-AH** (`strengthened_in +=
  PHASE4-N-AH`) — the extend state it continues is now ENTERED via local self-admit (DC-NODE-20), not
  the cert — but it is **NOT overclaimed as independent live architecture** (that is DC-NODE-20).
- **Strengthenings recorded under PHASE4-N-AH** (the live-proven cluster that subsumes this hermetic
  core): DC-NODE-05 / DC-NODE-12 / DC-NODE-15 / DC-NODE-18 / DC-NODE-19 / T-REC-03 / T-REC-05 /
  CN-NODE-02 / CN-NODE-04 carry `strengthened_in += PHASE4-N-AH`; the planned `+= PHASE4-N-AG` is folded
  into N-AH rather than double-credited. **DC-CONS-03 untouched.**
- **Grounding docs** refreshed at the N-AH close (incl. the CODEMAP+SEAMS deferred at the N-AF baseline
  `f87d0056`).
