# Invariant Slice — Single-producer successor forge extends the adopted durable spine

## 2. Slice Header

**Slice Name:** Single-producer extend-own-durable-spine (DC-NODE-18)
**Cluster:** PHASE4-N-AF — single-slice; **rung-1 only, single-producer only**
**Status:** Proposed
**Authority source:** `docs/planning/single-producer-extend-own-spine-invariants.md`; registry `DC-NODE-18` (declared → enforced on this slice)

**Cluster Exit Criteria Addressed** (this single-slice cluster's CEs = the DC-NODE-18 acceptance contract):
- [ ] **CE-AF-1** (hermetic): forge mode is an explicit enum `InitialCatchupRequired → CaughtUpToPeerTip{peer_tip} → FirstOwnBlockServed{own_tip, parent_peer_tip} → SingleProducerExtendOwnDurableSpine{adopted_root, current_tip}`; transitions total + deterministic; **no booleans**.
- [ ] **CE-AF-2** (hermetic): promotion into `SingleProducerExtendOwnDurableSpine` requires an explicit RED venue-adoption certificate (operator/harness-supplied) — **never** inferred from self-admit. Self-admit without a certificate keeps the mode at `FirstOwnBlockServed` (no extend).
- [ ] **CE-AF-3** (hermetic): in `SingleProducerExtendOwnDurableSpine`, a ForgeTick forges on `durable_servable_tip` **without** requiring `followed_peer_tip == durable_servable_tip`; the forged successor's parent byte-equals the durable tip (DC-CONS-24 preserved).
- [ ] **CE-AF-4** (hermetic, fail-closed fence): forge refuses `ForgeRefused::SingleProducerFenceViolation{reason, durable_tip, followed_peer_tip, observed_peer_tip, venue_role}` if — venue not declared single-producer · relay producing · a competing peer block beyond `adopted_root` observed · peer tip disagrees with the expected single-producer spine · the recovered anchor is the k=0 snapshot-conflict edge.
- [ ] **CE-AF-5** (replay): warm-start replay of a K≥2 own-forged chain is byte-identical (durable state **and** served chain) — T-REC-05 extended to the chain.
- [ ] **CE-AF-6** (operator-gated LIVE evidence; the DC-NODE-18 live half): committed `rung1-auto.sh` (k≥2) transcript — Ade forges N → relay adopts; Ade forges N+1 **without** relay echo → relay adopts; sustained past k (several Ade blocks in the relay ImmutableDB).

Exit criteria not listed here are out of scope for this slice.

## 3. Implementation Instruction (AI)
**Read §§9–10 + the authority source first.** Implement exactly that. The forge mode is an **explicit enum** (no booleans); promotion to extend-own-spine **requires explicit RED adoption evidence**, never inference; the certificate is **admissibility-only** (not persisted, not replay-visible). Obey §14 prohibitions + §15 non-goals. §12 is the only completion proof. Commit messages carry the repo's model trailer (per `CLAUDE.md`); no other AI references.

## 4. Intent
Make it impossible for a single-producer Ade to stall after one forged block: once caught up to a real peer tip (DC-NODE-15) **and** the relay's adoption of Ade's first successor is explicitly evidenced, the forge gate extends Ade's **own durable adopted spine** — enforcing **DC-NODE-18** — without weakening DC-NODE-15's initial-catch-up gate or DC-CONS-03's chain-selection authority.

## 5. Scope
- **Modules:** `ade_node::node_sync` (new `ForgeMode` enum + GREEN mode/fence classifier + `ForgeRefused` extension + the RED venue-certificate input type); `ade_node::node_lifecycle` (the ForgeTick arm threads + advances the mode, records the typed refusal).
- **State machines:** the new `ForgeMode` (4 states), RED scheduling state.
- **Persistence:** **none new** — reuses `pump_block` durable admit + WAL (DC-NODE-16 / DC-WAL-04 unchanged). `ForgeMode` and the venue certificate are RED scheduling state, **not** persisted/replayed.
- **Network-visible:** **none new** — reuses the existing follow/serve.
- **Out of scope:** multi-producer fork-choice (rung 2); follow-link keep-alive (OQ-KA); preprod (rung 3); the recover-anchor k=0 fix; the slot-2000 epoch-transition gap (rung-1 criterion 2 — within-epoch only here).

## 6. Execution Boundary (TCB color)
- **BLUE (UNCHANGED — zero diff):** `ade_runtime::forward_sync::pump` (`pump_block`, DC-NODE-16); `ade_ledger` ledger/chain_dep/WAL; `forge_one_from_recovered` successor construction (DC-NODE-10/DC-CONS-24); `ade_runtime` chain selection (DC-CONS-03).
- **GREEN:** the `ForgeMode` mode/fence classifier in `node_sync` — pure, total, deterministic (mirrors the existing `forge_followed_tip_admission` GREEN-by-function classifier).
- **RED:** the `node_lifecycle` ForgeTick arm (drives the mode); the venue-certificate input (operator/harness-supplied single-producer declaration + adoption evidence).

## 7. Invariants Preserved (registry IDs)
`DC-NODE-15` (initial caught-up gate still gates the FIRST forge) · `DC-NODE-16` (pump_block idempotency) · `DC-CONS-03` (chain-selection/fork-choice authority — **untouched**) · `DC-NODE-10` / `DC-CONS-24` (successor position + parent-hash byte-equality) · `DC-NODE-12` / `DC-WAL-04` (forged durable admit; no-orphan) · `T-REC-05` / `DC-WAL-02` (recover→follow + forged-chain replay-equivalence; the venue certificate is **not** replay-visible) · `DC-NODE-17` (followed-peer-tip reflects only real peer advertisements; never local inference; admissibility-only) · `T-DET-01` (the GREEN classifier is pure/deterministic).

## 8. Invariants Strengthened or Introduced
`DC-NODE-18` — declared → **enforced**. (This slice's tests + CI gate append to the registry entry; status flips on the hermetic CI **and** the committed CE-AF-6 live transcript.) Exactly one invariant family (DC-NODE forge).

## 9. Design Summary
The ForgeTick gate becomes **mode-aware** via an explicit `ForgeMode` enum (RED loop state; GREEN transition function):
- `InitialCatchupRequired` / `CaughtUpToPeerTip{peer_tip}`: the **existing** DC-NODE-15 path — forge only when `durable_servable_tip == followed_peer_tip`.
- On forging + serving the first successor: → `FirstOwnBlockServed{own_tip, parent_peer_tip}`.
- Promotion → `SingleProducerExtendOwnDurableSpine{adopted_root, current_tip}` fires **only** on an explicit RED venue-adoption certificate (operator/harness-supplied evidence that the relay adopted `own_tip`); never inferred from self-admit.
- In `SingleProducerExtendOwnDurableSpine`: each ForgeTick forges on `current_tip` (the durable spine) without the followed==durable requirement; `current_tip` advances per durable admit. The fence (CE-AF-4) is checked every tick; any violation → `SingleProducerFenceViolation` (no forge, no transition).

**Certificate boundary (load-bearing):** the promotion certificate is **admissibility evidence only**. It MUST NOT be persisted as authoritative chain state and MUST NOT alter replay-visible durable state; it may advance **only** the RED/GREEN forge-mode state.

## 10. Changes Introduced
- **Types:** `ForgeMode` enum (4 variants above); `ForgeRefused::SingleProducerFenceViolation{reason, durable_tip, followed_peer_tip, observed_peer_tip, venue_role}` (new variant, structured/comparable); a `VenueRole` / single-producer venue-certificate input type (RED, admissibility-only — never persisted/replay-visible).
- **State transitions:** the 4 mode transitions (§9), each total + deterministic in the GREEN classifier; the RED ForgeTick arm consumes the classifier's decision (forge | refuse-with-typed-violation | mode-advance).
- **Persistence:** none (ForgeMode + certificate are RED, not WAL'd; the durable surface is untouched).
- **Removal/Refactors:** the unconditional "forge only when durable==followed" requirement is replaced by the mode-aware gate (DC-NODE-15 retained for the initial path).

## 11. Replay, Crash, and Epoch Validation
- **Replay:** new test `extend_own_spine_two_runs_byte_identical` (`ade_node::node_sync` / `node_lifecycle`) — warm-start replay of a K≥2 own-forged chain yields byte-identical durable state + served chain (extends the N-U `recover_follow_*` / `forge_kill_then_warm_start_*` family; T-REC-05). The `ForgeMode` + certificate are RED and not replayed → cannot perturb the deterministic surface.
- **Crash/restart:** a mid-extend crash recovers via the existing WAL (DC-WAL-04 no-orphan); the recovered durable tip resumes the spine; the mode re-derives from on-disk state (re-enters the warm-start path, re-promotes only on a fresh certificate).
- **Epoch boundary:** **not applicable** — within-epoch sustained forging only; the slot-2000 epoch transition (rung-1 criterion 2) is a separate gap, explicitly out of scope (§15).

## 12. Mechanical Acceptance Criteria
- [ ] `forge_mode_transitions_are_total_and_deterministic` (CE-AF-1).
- [ ] `extend_own_spine_promotion_requires_adoption_certificate` — self-admit w/o cert stays `FirstOwnBlockServed`; cert promotes (CE-AF-2).
- [ ] `extend_own_spine_forges_on_durable_tip_without_followed_equality` + parent byte-equality (CE-AF-3).
- [ ] `single_producer_fence_fails_closed` — each of the 5 conditions → `SingleProducerFenceViolation{…}` with populated structured fields (CE-AF-4).
- [ ] `extend_own_spine_two_runs_byte_identical` warm-start replay (CE-AF-5).
- [ ] CI gate `ci/ci_check_single_producer_extend_own_spine.sh` — fences: explicit enum (no mode boolean); promotion-requires-certificate; certificate not persisted/replay-visible; fence fail-closed; the mode never references the chain-selector (DC-CONS-03 untouched).
- [ ] **CE-AF-6 (operator-gated, NOT CI):** committed `docs/evidence/phase4-n-af-extend-own-spine.{md,jsonl}` — the live `rung1-auto.sh` transcript (forge N→adopt; N+1→adopt-without-echo; past k). DC-NODE-18 flips to `enforced` only when the hermetic items **and** this transcript are committed.

## 13. Failure Modes
- `SingleProducerFenceViolation` — **fail-fast**, deterministic, comparable (structured fields); no forge, no state transition, tip unchanged. The **default** when no single-producer certificate is present (fail-closed).
- A forge `Failed` (real invariant/IO) still propagates fail-fast (unchanged).

## 14. Hard Prohibitions
- No booleans for the forge mode (explicit enum only).
- No global config knob that weakens semantics — single-producer is a **venue-scoped** RED certificate only.
- **The promotion certificate must never be persisted as authoritative chain state nor alter replay-visible durable state** (admissibility-only; advances only the RED/GREEN forge-mode state).
- No silent inference of relay adoption (promotion requires explicit RED evidence).
- No RED signal selecting / replacing / reordering / preferring chains (DC-CONS-03 untouched).
- No use in a multi-producer venue / when the relay is producing (fail-closed).
- **Zero BLUE change** (no new BLUE type/authority).
- No determinism tripwires in the GREEN classifier (no wall-clock / float / `HashMap` / `String`/`anyhow` errors).
- No TODOs / placeholders in the gate.

## 15. Explicit Non-Goals
Multi-producer fork-choice (rung 2) · follow-link keep-alive / OQ-KA · preprod acceptance (rung 3) · the recover-anchor k=0 fix · the slot-2000 epoch-transition gap (rung-1 criterion 2).

## 16. Completion Checklist
- [ ] All new durable state is replay-derivable (the spine; `ForgeMode` + certificate are RED, non-persisted).
- [ ] No new persisted bytes (durable/WAL surface untouched).
- [ ] All failure modes deterministic (`SingleProducerFenceViolation`, structured).
- [ ] No TODOs/placeholders in the gate; zero BLUE change.
- [ ] CI enforces DC-NODE-18 (`ci_check_single_producer_extend_own_spine.sh` + the hermetic tests).
- [ ] Replay-equivalence passes (CE-AF-5).
- [ ] CE-AF-6 live transcript committed; DC-NODE-18 flipped declared→enforced.
