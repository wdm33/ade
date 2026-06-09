# Invariant Slice AI-S2 — Shared detector + venue-split resolver

> Slice of PHASE4-N-AI (`docs/clusters/PHASE4-N-AI/cluster.md`). AI-S1 (rollback WAL
> durability) is closed + pushed (`cced0214`) — the constitutional guard is satisfied.
> GREEN, pure; **latent until AI-S4 wires it into the receive loop** — merges with no live
> behavior change.

## 2. Slice Header
- **Slice Name:** Shared receive detector (venue-blind) + venue-split resolver.
- **Cluster:** PHASE4-N-AI — live fork-choice wiring (rung-2, single-best-peer).
- **Status:** Proposed.
- **Cluster Exit Criteria Addressed:**
  - [ ] **CE-AI-2** (`DC-NODE-23` detector + `DC-NODE-24` venue-split resolver).
- **Slice Dependencies:** AI-S1 (closed) — cluster slice ordering, not a code dependency.

## 4. Intent
Make the receive-side classification of a peer-origin candidate a single **pure, total,
venue-blind** decision, and make the consequent **explicitly venue-gated** — so "is this a
competing chain?" is decided once and routed by an explicit venue mode
(`SingleProducer → refuse`, `Participant → fork-choice`), never by silent inference. This is
the seam the live loop (AI-S4) and the rung-1 fence share.

## 5. Scope
- **Modules / crates:** GREEN-by-function in `ade_node::node_sync` (sibling to
  `forge_followed_tip_admission` / `single_producer_forge_decision`).
- **State machines affected:** none (pure classifiers).
- **Persistence impact:** none. **Network-visible impact:** none (latent until AI-S4).
- **Out of scope:** live loop wiring + the `in_spine` ChainDb computation + the Participant
  CLI declaration + the orchestrator handoff — all **AI-S4** (RED). The apply path — AI-S3.

## 6. Execution Boundary (TCB color)
- **GREEN:** `ade_node::node_sync` — `classify_receive` (detector), `resolve_disposition`
  (resolver), the new `CandidateSummary` / `ReceiveClass` / `ReceiveDisposition` types, the
  `VenueRole::Participant` variant. Pure / total / deterministic — no I/O, clock, rand, float,
  `HashMap`, ChainDb, or network.
- **BLUE:** none. **RED:** none. *(Live wiring that would make this RED is AI-S4.)*

## 7. Invariants Preserved (registry IDs)
`DC-NODE-20` (SingleProducer fail-closed — byte-unchanged; `single_producer_forge_decision`
untouched), `DC-NODE-15` (initial catch-up gate untouched), `DC-CONS-03` (fork-choice
untouched — the detector/resolver never call `select_best_chain`), `T-DET-01`
(pure/deterministic).

## 8. Invariants Strengthened / Introduced
- **Introduces** `DC-NODE-23` (shared venue-blind detector) + `DC-NODE-24` (venue-split
  resolver) — enforced hermetically by this slice (totality + venue-split + venue-blindness +
  no-chain-selector tests + gate). One family: receive classification + venue routing.
  *(Live exercise is AI-S4; the rules' `tests`/`ci_scripts` append + any status flip happen at
  cluster close.)*

## 9. Design Summary
Two pure functions, split by venue-awareness so venue-blindness is **structural**:

- **Detector (`DC-NODE-23`, venue-blind):**
  `classify_receive(durable_tip: TipPoint, candidate: &CandidateSummary, in_spine: bool) -> ReceiveClass`
  where `ReceiveClass { AlreadyHave, LinearExtend, Competing }`.
  - `AlreadyHave` iff `in_spine` (the candidate is already in Ade's admitted spine /
    own-served lineage — the folded AH-FOLLOW-1 predicate, supplied as a flag; the RED ChainDb
    spine-membership computation is AI-S4).
  - else `LinearExtend` iff `candidate.prev_hash == PrevHash::Block(durable_tip.hash)` **and**
    `candidate.block_no == durable_tip.block_no + 1`.
  - else `Competing`.
  The signature has **no `VenueRole`** (venue-blindness is structural); the body references no
  `select_best_chain` / `fork_choice` / `chain_selector` / ChainDb / clock / rand.

- **Resolver (`DC-NODE-24`, venue-split):**
  `resolve_disposition(class: ReceiveClass, venue: VenueRole) -> ReceiveDisposition` where
  `ReceiveDisposition { AlreadyHave, LinearExtend, RefuseSingleProducer, NeedsForkChoice }`.
  **Total over the closed `VenueRole`; only `Competing` is venue-gated** (the fast paths pass
  through, so no unnecessary chain-selection):

  | class | venue | disposition |
  |---|---|---|
  | `AlreadyHave` | any | `AlreadyHave` |
  | `LinearExtend` | any | `LinearExtend` |
  | `Competing` | `SingleProducer` | `RefuseSingleProducer` |
  | `Competing` | `Participant` | `NeedsForkChoice` |
  | `Competing` | `Unknown` | `RefuseSingleProducer` — a **fail-closed disposition, NOT an inferred SingleProducer venue** (OQ-5) |

  `VenueRole` gains an explicit `Participant` variant; `Unknown` stays the fail-closed default.

- **Candidate input:** new GREEN `CandidateSummary { slot: SlotNo, block_no: BlockNo,
  hash: Hash32, prev_hash: PrevHash }` (reuses `ade_types::PrevHash` — the closed
  `Genesis | Block` wire grammar; a `Genesis` prev_hash can never `LinearExtend` a non-genesis
  tip). `durable_tip` reuses the existing `TipPoint`.

## 10. Changes Introduced
- **Types:** `CandidateSummary`, `ReceiveClass` (closed 3-variant), `ReceiveDisposition`
  (closed 4-variant); `VenueRole::Participant` (additive variant).
- **Functions:** `classify_receive` (detector), `resolve_disposition` (resolver).
- **Mechanical ripple:** any existing **exhaustive** `match` on `VenueRole` gains a
  `Participant` arm (compiler-surfaced; expected to be few — `single_producer_forge_decision`
  uses `!=`, not a match). Each arm preserves current behavior (Participant is not venue-declared
  until AI-S4).
- **No** state transitions, persistence, or BLUE changes.

## 11. Replay / Crash / Epoch Validation
Not applicable — pure stateless classifiers (no replay/crash/epoch surface). Determinism holds
by construction (no nondeterministic inputs); covered by the totality tests in §12.

## 12. Mechanical Acceptance Criteria
- [ ] `classify_already_have_when_in_spine` — `in_spine = true` → `AlreadyHave` (even when not a fresh extension).
- [ ] `classify_linear_extend_on_exact_parent_and_block_no` — `prev_hash == Block(tip.hash)` ∧ `block_no == tip.block_no + 1` → `LinearExtend`.
- [ ] `classify_competing_on_nonmatching_parent`, `classify_competing_on_wrong_block_no`, `classify_competing_on_genesis_prev_hash` → `Competing`.
- [ ] `resolve_singleproducer_competing_refuses` → `RefuseSingleProducer`.
- [ ] `resolve_participant_competing_needs_fork_choice` → `NeedsForkChoice`.
- [ ] `resolve_participant_already_have_and_linear_extend_do_not_call_fork_choice` — under `Participant`, `AlreadyHave` → `AlreadyHave` and `LinearExtend` → `LinearExtend` (only `Competing` becomes `NeedsForkChoice`; the fast path is not routed to fork-choice).
- [ ] `resolve_unknown_venue_fails_closed` — `Competing` + `Unknown` → `RefuseSingleProducer` as a fail-closed disposition, NOT an inferred SingleProducer venue.
- [ ] `resolve_passthrough_already_have_and_linear_extend` — across all venues, the two fast-path classes pass through unchanged.
- [ ] New gate **`ci/ci_check_receive_detector_venue_split.sh`**: `classify_receive` signature has **no `VenueRole`** (venue-blind); neither `classify_receive` nor `resolve_disposition` references `select_best_chain` / `fork_choice` / `chain_selector` / `ChainDb`; `resolve_disposition` is total over `VenueRole` with `Unknown` → refuse; `DC-NODE-20`'s `single_producer_forge_decision` is unchanged (no weakening).
- [ ] `cargo test -p ade_node` green.

## 13. Failure Modes
The classifiers are total (no error path) — every input maps to a defined variant;
`Unknown`/undeclared venue deterministically yields the conservative fail-closed refuse. No
fallible operations.

## 14. Hard Prohibitions
Inherits all eight cluster hard lines (cluster doc §8) — esp. #6 (SingleProducer fail-closed
unchanged), #7 (Participant is the only fork-choice path; raw `followed_peer_tip` never becomes
a candidate). **Slice-specific:**
- No `VenueRole` in the detector signature.
- No `select_best_chain` / `fork_choice` / `chain_selector` reference in either function.
- No ChainDb / network / wall-clock / `HashMap` / float in either function.
- No live loop wiring; no `in_spine` ChainDb computation here (AI-S4); no Participant CLI
  declaration here (AI-S4); no orchestrator handoff.
- `Unknown` venue is NEVER inferred to be a valid configured mode — it fails closed.
- No new BLUE; `DC-CONS-03` untouched.

## 15. Explicit Non-Goals
No live loop wiring / apply driver (AI-S3/S4); no ChainDb spine-membership computation; no
Participant venue declaration/CLI; no orchestrator handoff; no multi-peer; no performance work.

## 16. Completion Checklist
- [ ] `classify_receive` (venue-blind 3-way) + `resolve_disposition` (venue-split 4-way) +
  `CandidateSummary` + `VenueRole::Participant` added.
- [ ] Totality + venue-split + Participant-fast-path + venue-blindness + no-chain-selector tests
  green; `cargo test -p ade_node` green.
- [ ] `ci_check_receive_detector_venue_split.sh` green; `ci_check_single_producer_extend_own_spine.sh`
  (DC-NODE-20) stays green.
- [ ] No BLUE change; `DC-CONS-03` untouched.
