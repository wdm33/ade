# Invariant Slice — RED loop continuation past feed-EOF (DC-NODE-19 S2)

## 2. Slice Header
**Slice Name:** RED loop continuation + certified-run fence + Idle-under-dead-feed wakeup (DC-NODE-19, PHASE4-N-AG S2)
**Cluster:** PHASE4-N-AG — single-producer loop continuation after follow-link EOF; **rung-1 only, single-producer only**
**Status:** Proposed
**Authority source:** `docs/clusters/PHASE4-N-AG/cluster.md` (§3, §5, CE-AG-2, CE-AG-3); registry `DC-NODE-19` (declared — **not flipped** by this slice)

**Cluster Exit Criteria Addressed:**
- [ ] **CE-AG-2:** in a declared single-producer venue in the extend state, a structural feed-EOF does **not** terminate the loop (continues forging the own durable spine); the default `VenueRole::Unknown` venue halts **verbatim** (`HaltCleanly`). Only a clean structural EOF continues — a fatal source failure still exits via `Err` / fail-fast.
- [ ] **CE-AG-3:** continuation **fails closed** on each of the **7** certified-run conditions (reusing the DC-NODE-18 fence + per-continuation cert re-validation); the `Idle`-under-dead-feed wait wakes on the clock tick / shutdown (no busy-spin).

Exit criteria not listed (CE-AG-1 = S1; CE-AG-4 = S3; CE-AG-5/6 = S4/close) are out of scope.

**Slice Dependencies:** S1 (`b9ef6e69`) — `VenuePolicy` + `plan_loop_step`'s 5th param + the `venue_policy` projection.

## 3. Implementation Instruction (AI)
**Read §§9–10 + the cluster doc + the current `run_relay_loop_with_sched` (the `plan_loop_step` call site, the ForgeTick arm incl. `read_adoption_cert` + `single_producer_forge_decision`, the Idle arm, the clock seam) first.** RED-primary in `node_lifecycle`; reuse the GREEN `venue_policy` + the DC-NODE-18 fence. Thread the real policy **only when forge is active**; branch the Idle wait on `loop_state`; re-validate the cert per continuation tick. **Only a clean structural EOF continues** — a fatal source error stays `Err`/fail-fast. **No BLUE change; no new CLI flag; DC-NODE-19 NOT flipped.** Obey §14/§15; §12 is the only completion proof. Commit carries the repo's model trailer.

## 4. Intent
Make it impossible for a certified single-producer Ade (in the DC-NODE-18 extend state) to terminate its forge loop merely because the follow-link feed EOF'd — **relocating the loop's termination authority from feed-liveness to explicit shutdown / fatal error / a fail-closed certified-run fence** — enforcing DC-NODE-19, while preserving DC-NODE-05's `pump_block`-sole-tip authority and DC-CONS-03's fork-choice authority.

## 5. Scope
- **Modules:** `ade_node::node_lifecycle` (`run_relay_loop_with_sched`: thread `venue_policy` into `plan_loop_step`; the `loop_state`-branched Idle wait; the per-continuation cert re-validation in the ForgeTick arm); `ade_node::node_sync` (**+1** `SingleProducerFenceReason` variant `AdoptionCertificateMissingOrMalformed`; `venue_policy` / `single_producer_forge_decision` / `read_adoption_cert` reused unchanged). New: `ci/ci_check_single_producer_loop_continuation.sh`.
- **State machines:** none new (reuses `ForgeMode` / `VenuePolicy`).
- **Persistence:** none.
- **Network-visible:** none.
- **Out of scope:** the planner table (S1); the replay-equivalence proof (S3); the live run (S4); live-wiring `relay_producing` / `recovered_anchor_k0` (stay hard-wired `false` per INFO-1 — the blind venue **declaration** is the trust anchor); follow-link keep-alive (OQ-KA); flipping `DC-NODE-19`.

## 6. Execution Boundary (TCB color)
- **BLUE (UNCHANGED — zero diff):** `pump_block` (DC-NODE-16); `ade_ledger` ledger/chain_dep/WAL; `forge_one_from_recovered` (DC-NODE-10/DC-CONS-24); chain selection (DC-CONS-03).
- **GREEN:** `ade_node::node_sync` — the reused `venue_policy` + `single_producer_forge_decision` (unchanged) + the additive `SingleProducerFenceReason::AdoptionCertificateMissingOrMalformed` variant; `ade_node::run_loop_planner` (consumed, unchanged).
- **RED:** `ade_node::node_lifecycle` — the loop threading, the `loop_state`-branched Idle wait, the per-continuation `read_adoption_cert` gate.
- **No `cli.rs` change.**

## 7. Invariants Preserved (registry IDs)
`DC-NODE-05` (`pump_block` sole durable tip authority; feed work drains via `SyncOnce` before any `ForgeTick` — clause-2; the loop body advances no tip directly) · `CN-NODE-02` (single live-run lifecycle owner; **shutdown is the termination authority**) · `DC-NODE-12` (continued successors durable only via `pump_block`) · `DC-NODE-15` / `DC-NODE-18` (initial catch-up gate + extend-state forge decision unchanged) · `DC-CONS-03` (**untouched** — no chain selector reached) · `DC-NODE-16` (`pump_block` idempotency) · `DC-EPOCH-03` / `DC-CONS-09` (off-epoch / forecast still fail closed in the forge) · `T-REC-03` (loop-as-replay — continuation deterministic under the injected clock; the Idle sleep paces, never changes outputs) · `T-DET-01`. The default (`Unknown` / forge-`None`) path is byte-unchanged.

## 8. Invariants Strengthened or Introduced
`DC-NODE-19` — the loop now **continues past a structural feed EOF** in the certified single-producer extend venue, fenced to the 7 conditions, with the Idle wait clock-bounded — mechanically enforced by the new tests + `ci_check_single_producer_loop_continuation.sh`. (Declared; flips to `enforced` at cluster close after S3 + S4.) Exactly **one** invariant family (DC-NODE-19 loop continuation). The DC-NODE-05 / CN-NODE-02 `strengthened_in += PHASE4-N-AG` appends (termination authority relocated off feed-liveness) are recorded at close.

## 9. Design Summary
- **Thread the policy.** At the `plan_loop_step` call site, compute `let policy = match forge.as_deref() { Some(act) => venue_policy(act.venue_role, &act.forge_mode), None => VenuePolicy::HaltOnFeedEnd };` and pass it. Forge `None` ⇒ `HaltOnFeedEnd` (relay-only loops never continue). A certified single-producer venue in `SingleProducerExtendOwnDurableSpine` ⇒ `ContinueInSingleProducerExtend` ⇒ the loop ForgeTicks/Idles instead of `HaltCleanly` on EOF.
- **Idle-under-dead-feed (OQ-19-1).** Branch the Idle arm on `loop_state`: `Continuing` ⇒ the existing `select!{ source.wait_ready(), shutdown.changed() }`; `Ending` (only reachable in continue-mode — `Ending` + `HaltOnFeedEnd` ⇒ `HaltCleanly`, never Idle) ⇒ `select!{ tokio::time::sleep(slot-derived bounded interval), shutdown.changed() }`. The interval is derived from the slot cadence (`act.slot_length_ms`), **not** a fixed magic constant — so the wakeup follows the same injected-clock / slot schedule that governs forge eligibility. Wakes on the next slot poll (re-reads `act.clock`, forges the next due slot) or shutdown; no busy-spin; no dead-feed `wait_ready`.
- **Per-continuation cert re-validation (condition 7).** In the ForgeTick arm, when `loop_state == Ending` (a continuation tick) **and** `act.venue_role == SingleProducer` in the extend state, re-read `read_adoption_cert(&act.adoption_cert_path)`; if `None` (absent/malformed) ⇒ record `ForgeRefused::SingleProducerFenceViolation { reason: AdoptionCertificateMissingOrMalformed, … }`, no forge (fail closed). The existing `FirstOwnBlockServed` promotion cert-read is unchanged.
- **Structural-EOF-only.** Only `source.is_ended()` ⇒ `LoopState::Ending` is continued; a fatal source error already exits via the existing `?`/`Err` in the SyncOnce arm — never via `Ending` — so a fatal failure is never continued.
- **Conditions 1–6 reuse existing enforcement:** (1)/(2) `venue_policy` ⇒ `HaltOnFeedEnd` (no continuation); (3) shutdown = planner's highest precedence; (4) off-epoch/forecast/KES fail closed inside the forge; (5)/(6) reuse the existing DC-NODE-18 fence over the **last observed** peer/fence inputs — new post-EOF peer information is **not invented**. If a competing-chain or relay-producing signal was observed **before** EOF, continuation fails closed (`observed_peer_tip` retains the last pre-EOF observation; `relay_producing` stays `false` per INFO-1).

## 10. Changes Introduced
- **Types:** `SingleProducerFenceReason` `+= AdoptionCertificateMissingOrMalformed` (GREEN-defined in `node_sync`; **RED-constructed** by the loop's cert gate — the GREEN `single_producer_forge_decision` is **unchanged**, so DC-NODE-18's 5-reason fence test is unaffected). No other type change.
- **State transitions:** the `plan_loop_step` call receives the real policy (forge active + single-producer extend ⇒ `ContinueInSingleProducerExtend`); the Idle arm branches on `loop_state` (`Ending` ⇒ slot-cadence-bounded timer wait); the continuation ForgeTick re-validates the cert.
- **Persistence:** none (cert/`VenuePolicy`/`ForgeMode` are RED, never WAL'd; `pump_block` durable surface untouched).
- **Removal/Refactors:** the S1 default-pass `VenuePolicy::HaltOnFeedEnd` at the call site is replaced by the computed policy.

## 11. Replay, Crash, and Epoch Validation
- **Replay:** the dedicated post-feed-end byte-identical proof is **CE-AG-4 (S3)** — out of scope here. S2 must not perturb `T-REC-03`: the continuation is deterministic under the injected clock (the sleep paces; the injected clock decides slots), and the cert re-read is RED admissibility (a fixed cert ⇒ same decisions). S2's behavior tests (below) exercise the continuation; S3 adds the two-run + kill/warm-start byte-identity.
- **Crash/restart:** continuation forges go durable via `pump_block` (DC-WAL-04 no-orphan); a mid-continuation crash recovers via the existing warm-start (unchanged); the cert/`VenuePolicy`/`ForgeMode` are RED → re-derived on restart (re-enter warm-start; re-promote only on a fresh cert).
- **Epoch boundary:** **not applicable** — within-epoch only (off-epoch fails closed, DC-EPOCH-03 / condition 4).

## 12. Mechanical Acceptance Criteria
- [ ] `single_producer_extend_continues_past_feed_eof` — a hermetic `run_relay_loop_with_sched` in a declared single-producer venue in the extend state, with a feed that `is_ended()` + an injected clock of further due slots, forges successors **past** the feed-end (the loop does **not** `HaltCleanly` on EOF) (CE-AG-2).
- [ ] `unknown_venue_still_halts_on_feed_eof` — the default `Unknown` venue (and forge `None`) `HaltCleanly`s on feed-end, verbatim prior (CE-AG-2).
- [ ] `fatal_source_error_fails_fast_not_continued` — a fatal source error propagates as `Err` (loop returns `Err`), never continued (CE-AG-2).
- [ ] `continuation_fails_closed_per_fence_reason` — each fail-closed condition on the continuation path → no forge: the DC-NODE-18 reasons via `observed_peer_tip` (competing-chain / peer-tip-disagrees) **and** the new `AdoptionCertificateMissingOrMalformed` (absent/malformed cert) (CE-AG-3).
- [ ] `idle_under_dead_feed_wakes_on_clock_tick` — in continue-mode with a dead feed + `NotDue`, the loop wakes on the clock/shutdown (not parked on `wait_ready`): it makes progress to the next due forge, or terminates promptly on shutdown without hanging (CE-AG-3).
- [ ] `ci/ci_check_single_producer_loop_continuation.sh` (NEW) — fences: the loop threads `venue_policy(act.venue_role, &act.forge_mode)` into `plan_loop_step` (no hard-wired `HaltOnFeedEnd` when forge active); the Idle arm is clock/shutdown-bounded under `Ending` (no `source.wait_ready` on the dead-feed path; no busy-loop); the continuation reuses `single_producer_forge_decision` (no `select_best_chain` / `chain_selector` / `fork_choice`); per-continuation `read_adoption_cert` present; no new durable-tip path; no BLUE token.
- [ ] `ci/ci_check_node_run_loop_containment.sh` green — the loop body still advances the tip **only** via `pump_block` (containment semantically unchanged).
- [ ] `ci/ci_check_loop_planner_closed.sh` green — the S1 planner stays closed/total/content-blind.
- [ ] `ci/ci_check_single_producer_extend_own_spine.sh` green — DC-NODE-18 unaffected (verify the additive fence-reason variant does not break it).
- [ ] `cargo test -p ade_node` green. **DC-NODE-19 NOT flipped** (still `declared`).

## 13. Failure Modes
- `SingleProducerFenceViolation` (incl. the new `AdoptionCertificateMissingOrMalformed`) — **fail-closed**, deterministic, structured; no forge, no state transition, tip unchanged.
- A fatal source error / forge `Failed` — propagated `Err` (**fail-fast**); the loop returns `Err`, **not** continued.

## 14. Hard Prohibitions
**Inherited (cluster §8):** no RED `LoopState` re-derivation / planner "lie" (the planner gets the truthful `loop_state` + the `venue_policy` projection); no numeric "max blind forges" cap; no new durable tip path (`pump_block` sole); no chain-selector reference (**DC-CONS-03 untouched**); no continuation of a non-EOF `Ending` (fatal ⇒ `Err`); no busy-spin; zero BLUE change; no keep-alive / epoch / multi-producer / preprod / k=0 work; no new CLI flag.

**Slice-specific:**
- The Idle-under-dead-feed wait MUST be clock/shutdown-bounded (no `source.wait_ready` on the `Ending` path; no unbounded park; no busy-loop).
- `venue_policy` threaded **only** when forge is active (forge `None` ⇒ `HaltOnFeedEnd`).
- The cert re-validation reuses `read_adoption_cert` (no new cert parser); the cert stays **admissibility-only** (not persisted / replay-visible).
- `single_producer_forge_decision` stays the GREEN fence authority (no reimplemented fork-choice in the loop).
- No flip of `DC-NODE-19`; no determinism tripwire that changes **outputs** (the sleep is pacing only).

## 15. Explicit Non-Goals
The planner table (S1) · the replay-equivalence proof over a post-feed-end chain (S3) · the operator-gated live run (S4) · live-wiring `relay_producing` / `recovered_anchor_k0` (INFO-1) · follow-link keep-alive (OQ-KA) · flipping `DC-NODE-19`.

## 16. Completion Checklist
- [ ] Continuation forges past feed-EOF in the certified venue (test green).
- [ ] Default `Unknown` halts verbatim; fatal source error ⇒ `Err` (tests green).
- [ ] The 7 fail-closed conditions enforced, incl. the new cert reason (test green).
- [ ] Idle clock-bounded — no busy-spin, no hang (test green).
- [ ] New gate + containment gate + planner gate + DC-NODE-18 gate green.
- [ ] `cargo test -p ade_node` green.
- [ ] Zero BLUE change; `pump_block` still sole tip; DC-CONS-03 untouched.
- [ ] `DC-NODE-19` NOT flipped (still `declared`).
