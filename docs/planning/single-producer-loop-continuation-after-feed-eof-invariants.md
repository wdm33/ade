# Invariant sketch — Single-producer forge-loop continuation after follow-link EOF (DC-NODE-19)

> IDD Part I artifact (invariants phase). Pre-cluster. No clusters/slices/implementation here.
> The implementable slice for **CE-AF-6b**, deferred from **DC-NODE-18** (PHASE4-N-AF).
> Sibling docs: `sustained-single-producer-forge-invariants.md` (SF-6 / OQ-2 — the follow-link-liveness
> obligation held open), `single-producer-extend-own-spine-invariants.md` (DC-NODE-18).
> Approved framing 2026-06-08 (user), with the OQ-19-2 / OQ-19-5 / OQ-19-6 resolutions + the
> "structural feed EOF" wording correction recorded inline below.

**Concept (one line).** In a declared single-producer venue that has **already** entered the DC-NODE-18
extend state, a `LoopState::Ending` caused **solely by structural feed EOF** (the Ade→relay follow link
closing/draining) must **not** terminate the forge loop; the loop keeps forging successors on its own
certified durable spine, fenced to the certified single-producer run, until explicit shutdown, a fatal
error, an existing BLUE fence, or a competing chain ends it.

**Why this slice (c2t7, live-proven 2026-06-07).** DC-NODE-18 made Ade *able* to forge the successor after
certificate promotion (block 11 → relay adopts → cert → promote → block 12 → relay adopts, no echo). But the
c2t7 run still **stopped at 2 blocks**: the relay's idle timeout EOF'd the Ade→relay follow link, and the
loop treats the feed source as a lifecycle authority. DC-NODE-19 relocates that lifecycle authority — but
**only after** the node is already in the DC-NODE-18 single-producer extend state.

**Pure-transformation framing (the methodology check).** This **is** expressible as `canonical input →
canonical output` and introduces **no new authoritative nondeterminism**. The decision "given (loop-state,
feed-readiness, forge-slot, shutdown, venue-policy) → which `LoopStep`" is the existing pure/total GREEN
`plan_loop_step`; DC-NODE-19 refines exactly two cells (the `Ending` + `SingleProducer` rows). The feed-EOF
is already canonicalized into the closed `LoopState{Continuing,Ending}` + `FeedReason` vocabulary (RED→GREEN,
N-F-G-J). Forged successors depend only on `(recovered state, injected-clock-derived SlotNo, durable tip)` —
**not** on feed-EOF timing — so the durable surface stays clock-determined and replay-equivalent exactly as
DC-NODE-05 / T-REC-03 / T-REC-05 already frame it (relative to the injected clock-tick schedule). The
concept is well-understood.

**The precise gap.** `run_loop_planner::plan_loop_step` (`run_loop_planner.rs:145`):
`LoopState::Ending` + `NoWorkReady` → `HaltCleanly` **unconditionally** (the N-F-E clause-3 rule "the loop
never forges past the feed"). The RED loop sets `LoopState::Ending` when `source.is_ended()`
(`node_lifecycle.rs:1137`). So a clean follow-link EOF → `Ending` → `HaltCleanly` → `break`. That one cell
killed c2t7 at 2 blocks. (A *fatal* source error is a different path — it exits via `Err`/`?` fail-fast,
never via `Ending` — so EOF and fatal-failure are already mechanically distinct.)

---

## 1. What must always be true

- **N19 (the new rule — DC-NODE-19).** In `VenueRole::SingleProducer` **and**
  `ForgeMode::SingleProducerExtendOwnDurableSpine` (the DC-NODE-18 extend state, reached only via DC-NODE-15
  catch-up + an explicit venue-adoption-certificate promotion), a `LoopState::Ending` caused **solely by
  structural feed EOF** does **not** terminate the loop — it continues to `ForgeTick`/`Idle` on the own
  durable spine. The loop terminates only on: (a) explicit operator shutdown, (b) a fatal forge/admit/IO
  error (fail-fast), (c) an existing BLUE fail-closed fence (off-epoch DC-EPOCH-03 / beyond forecast horizon
  DC-CONS-09 / KES period), or (d) a DC-NODE-18 `SingleProducerFenceReason`.
- **DC-NODE-05 preserved (the careful boundary).** The forge advances **no** durable tip directly;
  `run_node_sync → pump_block` stays the **sole** durable tip-advance authority. "Produce subordinate to the
  sync spine" in the **clause-2 sense** (available feed work drains via `SyncOnce` *before* any `ForgeTick`)
  is preserved — only the loop's *termination-on-feed-liveness* relocates. The DC-NODE-05 four-cell contract
  (at-most-once-per-slot, never-past-slot, no-direct-tip-advance) is unchanged.
- **CN-NODE-02 preserved + strengthened.** `--mode node` stays the single live-run lifecycle owner; the
  loop's termination **authority** is now explicit operator shutdown / fatal error — **not** an accidental
  feed EOF. The feed source is demoted from *implicit* lifecycle authority to a pure data source in this
  venue. (Exactly the **DC-NODE-09** move — serve-listener lifetime decoupled from feed-end — now for the
  forge loop.)
- **T-REC-03 / T-REC-05 preserved + extended.** Same recovered state + same ordered feed (incl. its EOF
  marker) + same injected clock-tick schedule + same shutdown schedule → byte-identical authoritative
  outputs, **now including** the successors forged *after* feed-end (T-REC-03); same anchor + same WAL (incl.
  post-feed-end forged `AdmitBlock`s) → byte-identical recovered tip + ledger fingerprint (T-REC-05).
- **DC-NODE-18 / DC-NODE-12 / DC-NODE-10 / DC-CONS-24 preserved.** The extend *forge decision* and
  successor-position derivation are unchanged; DC-NODE-19 only governs whether the *loop keeps cycling* to
  reach those ForgeTicks. Each continued successor is durable **only** via `pump_block`.
- **DC-CONS-03 untouched.** The planner/loop never selects/reorders/prefers chains; continuation never
  reaches a chain selector.
- **Planner stays pure/total.** `plan_loop_step` remains pure/total/deterministic over its (now 5) closed
  inputs; the closed `LoopStep` vocabulary still cannot express an authority decision; `SlotNo` observable
  only in `forge_slot_status`. The table is **32 cases** (2⁵), exhaustive, no wildcard.
- **Default venue verbatim.** `VenueRole::Unknown` (default / non-single-producer) takes the **exact** prior
  precedence (`Ending → HaltCleanly`). Zero change off the declared single-producer path.

## 2. What must never be possible

The continuation is **fenced to the certified single-producer run** (OQ-19-2 resolution: **no numeric
"max blind forges" cap** — an artificial operator policy, not a Cardano semantic invariant). Continuation
**fails closed** (no continuation; verbatim `HaltCleanly` / typed refusal — never a silent forge) if **any**
of these hold:

1. not `VenueRole::SingleProducer`;
2. not `ForgeMode::SingleProducerExtendOwnDurableSpine`;
3. operator shutdown requested;
4. existing forge-validity bounds fail (off-epoch DC-EPOCH-03 / beyond forecast horizon DC-CONS-09 /
   KES-period invalid);
5. a competing chain was observed **before** EOF;
6. relay-producing evidence exists;
7. the venue certificate is absent or malformed.

And, independently, these must never be possible:

- The loop ignoring an operator shutdown — shutdown is **highest precedence**, even mid-continuation.
- A **second** durable tip-advance path — `pump_block` stays sole authority (DC-NODE-05 / DC-NODE-12).
- The feed-end continuation reaching chain selection / fork-choice (DC-CONS-03 untouched).
- Continuing a `LoopState::Ending` that represents a **real shutdown or fatal source failure** (only a clean
  *structural feed EOF* is continued; a fatal source failure exits via `Err`/fail-fast).
- The planner observing chain content / a raw `SlotNo` / a verdict — the 5th input must be a **content-blind**
  yes/no venue-policy projection, never a tip/hash/verdict.
- **Busy-spin.** When `Ending` + `SingleProducer` + nothing due, the loop must `Idle`/wait deterministically
  (next clock tick or shutdown), never hot-loop on a dead feed.

## 3. What must remain identical across executions (deterministic surface)

- The durable post-state after continuing to forge K successors past feed-end: ledger fingerprint,
  `chain_dep`, ChainDb tip, WAL image. (Same surface as DC-NODE-18 — just more of the same deterministic
  blocks.)
- The `LoopStep` decision sequence for a fixed (loop-state stream incl. EOF, sync-status stream,
  forge-slot/clock schedule, shutdown schedule, venue role).
- **Not** on the surface: `followed_peer_tip`, feed-liveness *timing*, wall-clock (only the derived `SlotNo`
  crosses the seam). Feed-EOF timing is RED nondeterminism canonicalized into `LoopState::Ending`; the loop's
  *response* to it (given venue + clock + shutdown) is deterministic. (Replay-equivalence is relative to the
  injected clock-tick schedule, exactly as DC-NODE-05 / T-REC-03 frame it.)

## 4. What must be replay-equivalent

- Same recovered checkpoint + same WAL (own-forged chain, **incl. blocks forged after feed-end**) →
  byte-identical recovered durable tip + ledger fingerprint (**T-REC-05** over a post-feed-end chain).
- Same recovered state + same ordered feed (incl. EOF) + same injected clock schedule + same shutdown →
  byte-identical durable outputs (**T-REC-03** extended): two clean runs byte-identical; kill+warm-start over
  a post-feed-end-forged chain recovers byte-identically.
- The feed-end event appends **nothing** to the WAL (a loop-control signal, replay-neutral); each continued
  successor appends exactly one chained `WalEntry::AdmitBlock` via `pump_block` (DC-WAL-04).

## 5. State transitions in scope

GREEN planner (pure, total — **OQ-19-6 resolution: explicit 5th input**, no RED re-derivation of
`LoopState`):
`(LoopState, SyncStatus, ForgeSlotStatus, ShutdownStatus, VenuePolicy) → LoopStep` — 32 cases, no wildcard.

- `(Ending, NoWorkReady, Due, Running, SingleProducer) → ForgeTick` **(NEW)** — continue past feed-end.
- `(Ending, NoWorkReady, Due, Running, Unknown) → HaltCleanly` (verbatim prior).
- `(Ending, NoWorkReady, NotDue, Running, SingleProducer) → Idle` **(NEW)** — wait for next clock tick /
  shutdown, don't halt.
- `(Ending, WorkAvailable, *, Running, *) → SyncOnce` (UNCHANGED — DC-NODE-05 clause-2, both venues).
- `(*, *, *, ShutdownRequested, *) → HaltCleanly` (UNCHANGED — shutdown wins).
- `(Continuing, …) → …` (UNCHANGED — live-feed behavior identical).

RED driver effects (no new effect path):
- `(extend mode, ForgeTick, durable_tip=current) → Ok((mode advanced, durable tip += N+1 via pump_block))` —
  existing DC-NODE-18/12 transition, now reachable post-feed-end, **only** when the §2 certified-run fence
  passes.
- `(Ending, SingleProducer, NotDue) → Idle → wait{next clock tick, shutdown}` — must not block on a dead
  feed's `wait_ready()` (see OQ-19-1).

Termination (the bounds):
- `(any, ShutdownRequested) → HaltCleanly` · `(forge) → Err(fatal)` (fail-fast) ·
  `(off-epoch/beyond-forecast/KES) → forge fails closed` ·
  `(competing peer block / peer-tip-disagrees / relay-producing / venue|mode|cert fence) →
  SingleProducerFenceViolation`.

## 6. TCB color hypothesis

- **GREEN (the core refinement):** `plan_loop_step` gains a 5th closed content-blind `VenuePolicy` input;
  stays pure/total/deterministic (32-case table). Plus a GREEN projection `VenueRole` (+ `ForgeMode`) →
  `VenuePolicy`. The heart of DC-NODE-19.
- **RED (loop threading):** `run_relay_loop_with_sched` threads the policy in; the `Idle` arm's
  cancellation-safe wait must include a clock-tick wakeup under `Ending` (so a dead feed doesn't starve the
  forge cadence); the shutdown watch stays the lifecycle authority; the certified-run fence reuses the
  DC-NODE-18 `SingleProducerFenceReason` / `single_producer_forge_decision`.
- **BLUE: UNCHANGED.** No BLUE type, no new authority — `pump_block` / `forge_one_from_recovered` /
  `block_validity` / chain selection untouched. (Mirrors DC-NODE-18's GREEN+RED-only shape.)

## 7. Open questions (remaining after the 2026-06-08 approval)

- **OQ-19-1 (Idle-under-dead-feed wakeup — RED design, resolve at /cluster-doc):** restructure the
  cancellation-safe wait so `Ending` + `SingleProducer` + `NotDue` idles to the next clock tick / shutdown,
  not forever on a dead `wait_ready()`. A cluster-doc detail, not a new invariant.
- **OQ-19-2 — RESOLVED (no numeric cap; certified-run fence).** The venue declaration + existing BLUE
  fences + operator shutdown bound the continuation; the §2 seven-condition fail-closed fence keeps it tied
  to the certified single-producer run. No "max blind forges" magic number.
- **OQ-19-3 / OQ-KA — RESOLVED (out of scope).** Follow-link keep-alive / reconnect (so the feed stays alive
  and Ade keeps observing the peer) is the **complementary cousin** approach, **not** DC-NODE-19. DC-NODE-19
  scopes only continuation (don't-die-on-EOF). OQ-KA stays a separate non-blocking diagnostic/slice.
- **OQ-19-4 — RESOLVED (live acceptance bar = CE-AF-6b).** Sustained **> k** Ade blocks settle into the
  relay's ImmutableDB across **≥ 1** follow-link EOF, with warm-start replay byte-identical. Operator-gated
  on the rung1-auto C2-LOCAL harness; committed live transcript required for enforcement.
- **OQ-19-5 — RESOLVED (new rule).** **DC-NODE-19**, not a mutation of DC-NODE-05. At close, append
  `strengthened_in += DC-NODE-19` on: **DC-NODE-05** (feed work still drains before forge; no direct durable
  tip advance), **CN-NODE-02** (lifecycle owner = explicit operator shutdown / fatal error, not accidental
  feed EOF), **T-REC-03** + **T-REC-05** (replay equivalence now covers post-feed-end forged successors), and
  **DC-NODE-18** (the proven extend state is now sustained). **DC-CONS-03** explicitly untouched.
- **OQ-19-6 — RESOLVED (explicit GREEN 5th input).** No RED re-derivation of `LoopState::Continuing` after
  EOF (a semantic lie). The planner states the truth plainly:
  `(LoopState, SyncStatus, ForgeSlotStatus, ShutdownStatus, VenuePolicy) → LoopStep`, reviewable + total +
  CI-checkable.

## Out of scope (recorded elsewhere)

- Follow-link keep-alive / reconnect (OQ-KA) — separate complementary cousin (§7 OQ-19-3).
- Multi-producer fork-choice (rung 2), preprod (rung 3) — §7b ladder discipline; DC-CONS-03 untouched.
- The recover-anchor k=0 snapshot-conflict edge — operationally guarded k≥2.

## Proposed registry entry (appended to `docs/ade-invariant-registry.toml`)

`DC-NODE-19`, `tier = derived`, `status = declared` (enforced only when the slice lands with the planner
32-case totality gate + the certified-run continuation fence + the T-REC-03/T-REC-05 replay proofs over a
post-feed-end chain + the verbatim `VenueRole::Unknown` halt-on-feed-end default + the committed CE-AF-6b
live transcript). RED/GREEN-only, no BLUE change. Not a chain-selection rule.
