# Multi-candidate fork-choice SELECT — invariant sketch

> **Status:** Invariants sketch (IDD Part I). Planning artifact — **not** a cluster plan, **not** a
> slice doc, **not** a registry declaration. Produced before any cluster, slice, or code work.
> Sibling of `docs/planning/phase4-n-ai-live-fork-choice-invariants.md` (the FOLLOW half).
>
> **Concept:** the rung-2 **SELECT** half of live fork-choice. Ade connects to multiple competing
> producers on the Participant venue, tracks competing candidate chains, runs the **existing** BLUE
> `select_best_chain` over the candidate set (routed-to, never duplicated), durably adopts the
> fork-choice-maximal chain via the **existing** enforced apply authorities, and rolls back its **own**
> adopted chain (`RollbackReason::ForkChoiceWin`) when out-competed — replay-equivalently.
>
> **Flip target:** `CN-CONS-03` (`declared` → enforced). `DC-CONS-03` (the BLUE ordering) is **already
> enforced**; this cluster exercises + strengthens it live, it does not introduce it.
>
> **Predecessor (done, live-proven):** the FOLLOW half — single-best-peer rollback-follow,
> `DC-NODE-23…29`, live-proven by **CE-AI-6** (2026-06-11), unblocked by N-AM keep-alive sustain +
> N-AN `T-REC-06` eta0 overlay.

---

## 0. Framing — the canonical transformation

**Yes, this is expressible as `canonical input → canonical output`** — and the pure core *already
exists*. The authoritative decision is the BLUE total function

```
select_best_chain : (ChainSelectorState, &[CandidateFragment]) → (ChainSelectorState, ChainEvent)
```

already `enforced` (`DC-CONS-03`), already multi-candidate (partitions eligible/ineligible, picks the
fork-choice-maximal tip, k-bounds rollback depth), already arrival-order-independent (`CN-CONS-01`).

The cluster-level transformation is:

> `(durable_chain_state, ordered canonical multi-peer receive-event log)`
>   `→ ordered durable apply-effects (WAL: AdmitBlock | RollBack{ForkChoiceWin})`

Nondeterminism (N peers, arrival timing, network) enters **only** as canonical input — the ordered
receive-event sequence captured in the WAL — exactly as the FOLLOW half established.

**Hypothesis (NOT a normative claim — see §1):**

> SELECT is **primarily RED/GREEN wiring over the existing BLUE selector/apply authority.**

This is a hypothesis the code review supports but has **not yet proven**. It is firmed up by §1 (the
selector + both apply arms + the `ForkChoiceWin` marker are all built and enforced) but it is **not
zero-proof**: candidate construction and peer-identity binding **can become authority if mishandled**
(a peer-minted summary fed to the authoritative selector turns RED peer claims into BLUE authority —
see the proof obligation in §1 and OQ-SELECT-2). We do **not** declare "SELECT is just wiring" as
normative until the aggregator and range-fetch sequencing are specified.

---

## 1. Built foundation vs. true gaps (code-review discovery pass)

The discovery pass (verified file:line anchors in the appendix, §9) reshapes the problem from *"maybe we
need to design chain selection"* to *"the selector and apply machinery mostly exist; the live path is not
feeding them enough structured information."* That is a much better problem.

### Built foundation (verified to exist + be tested/enforced)

- **`select_best_chain` exists and is tested** — BLUE, `enforced` (`DC-CONS-03` / `CN-CONS-01`),
  multi-candidate, k-bounded, arrival-order-independent, density-free. (`ade_core/.../fork_choice.rs:96`)
- **`apply_chain_event` has both a rollback arm and a `ChainSelected` arm** — the rollback arm is live
  (FOLLOW), the `ChainSelected` (roll-forward + reconcile) arm is built and unit-tested but never fed a
  live event. (`ade_node/.../node_lifecycle.rs:2353` / `:2432`)
- **`RollbackReason::ForkChoiceWin` exists** — defined, tagged (tag 0), replay-tested; constructed only
  in tests today (live path builds `PeerRollBackward`). (`ade_ledger/.../wal/event.rs:109`)
- **BlockFetch range request exists** — the `BlockFetchMessage::RequestRange{from,to}` client is present
  and used for the linear tip-follow (`from==to==tip`). (`ade_runtime/.../wire_pump.rs:161,759`)

### True gaps (the live SELECT work)

1. **Participant path fails closed on `NeedsForkChoice`** — *"multi-candidate selection is a later
   multi-peer slice"*; a competing Participant block dead-ends in `Err(UnexpectedRollback)`.
   (`node_lifecycle.rs:2562-2569`)
2. **Peer identity is dropped before candidate tracking** — `AdmissionPeerEvent` carries `peer: String`,
   but the `NodeBlockSource → NodeSyncItem` conversion flattens it (`NodeSyncItem = Block(Vec<u8>) |
   RollBack(Point)`). The live participant loop consumes peer-anonymous items. (`node_sync.rs:77-80,
   :203,:217`)
3. **The live path never calls `select_best_chain`** — its only callers are the tested-but-unwired
   `chain_selector` orchestrator, the unwired `follow.rs`, and an interop bin. `--mode node` never
   reaches it.
4. **No selected-range fetch/apply sequencing** — after a (hypothetical) win, nothing fetches the
   winning branch's **bodies** as a selected range and sequences `RolledBack(fork_anchor)` +
   `ChainSelected(body)×N` through the existing apply arms.
5. **No live candidate aggregation** — N peers already merge into one feed
   (`spawn_live_wire_pump_source`), but there is no per-peer candidate-chain tracking or candidate-set
   assembly.

### Honest BLUE-safety boundary

> **Near-zero new BLUE expected, but not zero-proof yet.**

The likely new authority is mostly GREEN/RED wiring, **but** candidate construction and peer-identity
binding can become authority if mishandled. Two concrete dangers:

- A **peer-minted `ValidatedHeaderSummary`** (the `follow.rs` shortcut, explicitly *"must never leak into
  BLUE or any persisted path"*) fed to the authoritative selector would let a lying peer win fork-choice
  with fabricated `block_no` / VRF / op-cert values.
- A **rollback of Ade's own good chain committed before the winner is validated** would let a
  lying/failing peer induce Ade to abandon a valid chain for one that then fails `pump_block` — the
  H-1 failure mode generalized to the Ade-initiated path.

**Proof obligation (slice-entry, blocking):**

> Demonstrate that the candidate summaries passed to `select_best_chain` are derived **only from
> validated / canonical inputs** (Ade's own `validate_and_apply_header` output, per `DC-NODE-24`), and
> that the path **preserves peer identity without trusting peer claims** — no RED-minted summary, no
> peer-supplied slot/hash/block_no taken as authority, and **no durable rollback of the current chain
> committed until the winning candidate's bodies have validated through `pump_block`.**

*(Coupling note surfaced, not resolved here: validating a competing candidate's headers needs the
chain_dep **at the fork point**, which is a different nonce basis than the current tip — so candidate
construction, header validation, and the fork-switch sequencing are coupled. Named in OQ-SELECT-2/3;
resolved at `/cluster-plan` / slice entry, not now.)*

---

## 2. What must always be true (organized around the fork-choice authority invariant)

- **FC-1 — single selection authority (the central invariant).** `select_best_chain` is the **sole**
  chain-selection authority. Every live multi-peer selection routes *through* it; the wiring adds no
  second selector, no parallel preference, no density ordering, no operator heuristic. *(Strengthens
  `DC-CONS-03` / `CN-CONS-03`; reuses the BLUE authority unchanged.)*
- **FC-2 — multi-candidate determinism / arrival-order independence (live).** The selected tip is a pure
  function of the candidate **set** and the durable state — independent of which peer delivered first, of
  inter-peer interleaving, of wall-clock. *(Strengthens `CN-CONS-01` from the hermetic permutation proof
  to the live multi-peer path.)*
- **FC-3 — Praos ordering.** Block-number first, then `TiebreakerView` (slot, issuer, op-cert counter,
  VRF); density forbidden. *(`DC-CONS-03`, already enforced — now exercised live across a real fork.)*
- **FC-4 — k-bounded rollback.** No reselection rolls back more than k blocks; a candidate forking below
  the immutable tip or deeper than k is ineligible and never adopted. *(`DC-CONS-05` / `DC-CONS-06` —
  reuse.)*
- **FC-5 — a fork-choice win is PROVISIONAL (bodies validate).** A candidate is durably adopted **only**
  when its block **bodies** validate + apply through `pump_block` (the sole roll-forward durable admit).
  No header-only tip advance — selection over header summaries is a decision to *fetch-and-validate*, not
  a durable commit. *(Strengthens `DC-NODE-25`.)*
- **FC-6 — never abandon a validated chain for an unvalidated one.** The durable rollback of Ade's
  current chain MUST NOT commit until the winning candidate's bodies have validated; a failed/lying
  winner leaves Ade's current durable chain unchanged. *(Generalizes the H-1 / `DC-NODE-29` discipline to
  the Ade-initiated `ForkChoiceWin` path.)*
- **FC-7 — Ade-initiated reselection rollback is canonically bound.** When Ade switches from chain A to a
  better chain B, the rollback target (the fork point) resolves against Ade's **durable ChainDb stored
  slot+hash** as the sole authority — never peer-supplied slot, never mixed peer/local authority; fail
  closed before any mutation. *(Extends `DC-NODE-29` from `PeerRollBackward` to `ForkChoiceWin`.)*
- **FC-8 — durable lockstep + reconciliation.** Every applied selection advances/rolls-back ChainDb +
  LedgerState + PraosChainDepState as one structural transition (`DC-CONS-20`), and after every applied
  decision `selector.current_tip == ChainDb::tip` (`DC-NODE-26`). No partial admit, no partial rollback,
  no in-memory decision ahead of the durable spine.
- **FC-9 — no forge across unresolved reselection.** While a fork-choice decision (rollback+reapply) is
  in flight, forging is disabled; the forge never builds on a stale pre-resolution tip. *(`DC-NODE-28` —
  reuse, extended to multi-peer.)*
- **FC-10 — candidate summaries are Ade-validated, never peer-minted.** A `CandidateFragment` fed to
  `select_best_chain` carries header summaries **Ade itself derived from validated headers**
  (`validate_and_apply_header`, per `DC-NODE-24`); a raw `followed_peer_tip` or a peer-trusted minted
  summary (`follow.rs` shape) MUST NOT reach `select_best_chain`. *(The BLUE-safety spine — see §1.)*

---

## 3. What must never be possible

- A second chain selector / parallel selection path / density ordering / operator override reaching the
  durable tip.
- A header-only tip advance (adopting a chain whose bodies Ade hasn't validated through `pump_block`).
- A **RED-minted `ValidatedHeaderSummary`** (the `follow.rs` shortcut) crossing into the authoritative
  selection / adoption path.
- A peer winning fork-choice on **fabricated** `block_no` / VRF / op-cert values (selection over
  unvalidated peer claims).
- **Abandoning a validated chain for an unvalidated winner** — committing the rollback of Ade's current
  durable chain before the winner's bodies validate.
- A Byzantine peer's candidate truncating Ade's durable chain beyond k, or to a peer-chosen fork depth
  (mixed peer/local authority on the rollback target) — the **H-1** failure mode on the Ade-initiated
  path.
- A forge firing on a stale tip while a reselection is pending.
- A **partial fork-switch**: rolled back but not reapplied, or reapplied without the rollback durably
  recorded — leaving ChainDb / ledger / chain_dep out of lockstep.
- A **live-only** Ade-initiated reselection (not durably recorded), so replay would diverge.
- Selection influenced by *which* peer is preferred / first / fastest (arrival- or connection-order
  dependence).

---

## 4. What must remain identical across executions (deterministic surface)

Given the same durable store + the same **ordered canonical receive-event log** (each peer's RollForward
headers, RollBackward points, body deliveries, merged into one canonical total order), the SELECT path
produces the same selected durable tip, the same ledger fingerprint, the same `PraosChainDepState`, and
the same ordered WAL entries (`AdmitBlock` + `RollBack{ForkChoiceWin}`).

The **merge of N peers' event streams into one canonical order** is the determinism-critical surface: the
RED driver observes peers concurrently, but the *authoritative* input must be a canonical total order (the
WAL append order), and selection over the resulting set must be order-independent (FC-2). *(See
OQ-SELECT-1 + OQ-4.)*

---

## 5. What must be replay-equivalent

The ordered live multi-peer receive-event sequence, replayed against the same bootstrap anchor + durable
WAL, produces a **byte-identical** durable tip + ledger fp + `PraosChainDepState` — **including every
Ade-initiated `ForkChoiceWin` reselection**. The durable `WalEntry::RollBack{reason: ForkChoiceWin}`
marker re-invokes the **same** `materialize_rolled_back_state` / `commit_rollback` authority on replay
(not a second rollback impl). *(Extends `DC-NODE-27` from `PeerRollBackward` to `ForkChoiceWin` — the
reason variant already exists.)* This is the contract proving Ade's live fork-choice decisions are a
deterministic function of the canonical input, not of network timing.

---

## 6. State transitions in scope (REUSE vs NEW marked)

- **T-SEL-1 (REUSE)** classify: `(durable_tip, candidate_summary, in_spine) → ReceiveClass`. `DC-NODE-23`
  `classify_receive`, unchanged.
- **T-SEL-2 (REUSE, now wired)** resolve: `(ReceiveClass::Competing, Participant) → NeedsForkChoice`.
  `DC-NODE-24` — unchanged, but its consequent gets *wired* (today fails closed, gap #1).
- **T-SEL-3 (NEW)** aggregate: `(per_peer_candidate_state, peer_id, Ade-validated header) → updated
  candidate set`. Track each peer's competing fragment above the common fork anchor — requires peer
  identity restored (gap #2). The new tracking surface.
- **T-SEL-4 (REUSE BLUE)** select: `(ChainSelectorState, &[CandidateFragment]) → (ChainSelectorState,
  ChainEvent)`. `select_best_chain`, unchanged — now fed a set > 1 (gap #3).
- **T-SEL-5 (NEW sequencing over REUSED apply)** apply a `ForkChoiceWin`: `(ForwardSyncState,
  ChainSelected{new_tip, replaced_tip, fork_anchor}, fetched winner bodies) → ordered[
  RolledBack(fork_anchor); then pump each fetched winner body ] → durable effects`. The fork-switch
  decomposition; each step reuses `apply_chain_event` (both arms exist); the sequencing + the winner-body
  range-fetch is new (gaps #4/#5). Errors (fork beyond k, body invalid, fetch failure, reconciliation
  mismatch) fail closed with **no partial state and no abandonment of the current chain** (FC-6).
- **T-SEL-6 (REUSE)** forge fence: `(pending_reselection) → Option<ForgeRefused::ReselectionPending>`.
  `DC-NODE-28`.
- **T-SEL-7 (REUSE/EXTEND)** replay a `ForkChoiceWin` rollback: `(durable_state,
  WalEntry::RollBack{ForkChoiceWin}) → re-invoke materialize+commit`. `DC-NODE-27` extended.

---

## 7. TCB color hypothesis

- **BLUE (reused, unchanged — hypothesis: ZERO new canonical type, ZERO new authority):**
  `select_best_chain`; `materialize_rolled_back_state` (+ the `T-REC-06` eta0 overlay); `commit_rollback`;
  `pump_block`'s BLUE reducer; `WalEntry::RollBack` codec + `RollbackReason::ForkChoiceWin`; the
  header/VRF/KES validators (`validate_and_apply_header`). *(Not zero-proof — see §1.)*
- **GREEN:** the multi-peer candidate aggregator (per-peer fragments → canonical `&[CandidateFragment]`),
  the validated-header → `CandidateFragment` construction, the reconciliation projection. Pure / total /
  deterministic.
- **RED:** the multi-peer connection + per-peer wire pumps (the sole per-peer pump exists; N peers = N
  pumps, already merged), the peer-identity restoration in the feed, block-fetching the winning
  candidate's bodies as a selected range, the live driver sequencing select→apply, the convergence-
  evidence emission.

The FC/IS direction is inward-only and already set by FOLLOW: **RED observes peers → GREEN aggregates into
a canonical candidate set → BLUE selects → RED/BLUE apply via the enforced chokepoints.** SELECT widens RED
observation (1→N), restores the peer identity the feed currently flattens, and wires the GREEN aggregation
+ BLUE dispatch that FOLLOW left fail-closed.

**Color risk (the not-zero-proof edge):** candidate construction (`CandidateFragment` assembly) and
peer-identity binding are GREEN/RED *if* they consume only validated inputs (FC-10) — but they slide into
*de-facto authority* if they mint or trust peer claims. The proof obligation in §1 is what keeps them
GREEN/RED.

---

## 8. Open questions

### Load-bearing (carry into `/cluster-plan`)

- **OQ-SELECT-1 — peer-identity restoration / where per-peer aggregation lives.** Where does per-peer
  aggregation live so peer identity is **not** flattened? Thread `peer` through `NodeSyncItem` (then
  aggregate downstream), or aggregate *upstream of the flatten*? `AdmissionPeerEvent` carries `peer:
  String`; `NodeSyncItem` discards it.
- **OQ-SELECT-2 — BLUE-safe candidate construction.** How are candidate fragments made BLUE-safe? **No
  RED-minted `ValidatedHeaderSummary` from `follow.rs` may cross into BLUE.** Candidates must come from
  `validate_and_apply_header` output (`DC-NODE-24`). Sub-question: validating a competing fork's headers
  needs the chain_dep **at the fork point** (a different nonce basis) — how is that fork-point state
  obtained without committing a rollback first?
- **OQ-SELECT-3 — selected-range fetch + apply sequencing.** After `select_best_chain` chooses a winner,
  how does the live path **fetch and apply the selected range** through the existing `apply_chain_event`
  arms? Specifically: *who* asks for the range, *from which peer*, *using which fork anchor and selected
  tip*, and *what happens if the peer fails to provide it* (FC-6: current chain unchanged)?

### Carried (secondary, still open)

- **OQ-4 — canonical merge order / WAL-as-input.** Confirm the canonical authoritative input is simply
  **WAL append order** (as FOLLOW establishes), with FC-2 guaranteeing the *selected* tip is invariant to
  the merge — i.e., replay is over the WAL and the WAL append order *is* the canonicalization.
- **OQ-5 — venue / CE shape.** `CN-CONS-03` flips on a live multi-producer convergence capture. Venue:
  the CE-AI-6 multi-pool `cardano-testnet` (magic 42, k=5) extended to **two simultaneous competing
  peers**. Does the CE need both peers *producing* (a standing fork), or partition→heal (CE-AI-6 approach
  A)? (The §5b #8 finding showed Ade *loses* to competing producers today precisely because it fails
  closed — SELECT is what fixes that.)
- **OQ-6 — rule story (recommendation; declared at `/cluster-doc`, NOT now).** **Flip** `CN-CONS-03`
  declared→enforced. **Strengthen** `DC-CONS-03` + `CN-CONS-01` (live multi-candidate), `DC-NODE-27`
  (`ForkChoiceWin` replay), `DC-NODE-29` (Ade-initiated rollback binding), `DC-CONS-20` / `DC-NODE-25` /
  `26` / `28` (multi-peer). **New** DC-NODE-family rules for: peer-identity restoration; multi-peer
  candidate aggregation; the live `select_best_chain` dispatch; BLUE-safe candidate construction;
  selected-range fetch + fork-switch apply sequencing. Exact IDs/count at `/cluster-plan`. **No registry
  entries are created by this sketch.**
- **OQ-7 — parallel release-blocking priority.** Per `feedback_fail_closed_validation`, fail-closed
  **tx-validity agreement + adversarial negatives** is a *separate* release-blocking priority. SELECT is
  the C2 rung-2 robustness gate, not the tx-validity gate. Confirm sequencing (SELECT first per the §7b
  ladder, or interleave).

---

## 9. Appendix — verified code anchors (placeholder inventory)

| # | Placeholder | Location | Kind | Role in SELECT |
|---|---|---|---|---|
| 1 | `NeedsForkChoice \| RefuseSingleProducer => Err(UnexpectedRollback)` — *"multi-candidate selection is a later multi-peer slice"* | `crates/ade_node/src/node_lifecycle.rs:2562-2569` (`run_participant_sync`) | DEAD-END (fail-closed stub) | THE arm SELECT replaces. |
| 2 | `apply_chain_event` **`ChainSelected` arm** | `node_lifecycle.rs:2432-2451` | BUILT-BUT-LIVE-DEAD | Built + tested (`apply_driver_ai_s3.rs`); never fed a live `ChainSelected`. Ready to receive. |
| 3 | `RollbackReason::ForkChoiceWin` (tag 0) | `crates/ade_ledger/src/wal/event.rs:109` | RESERVED | Defined + replay-tested; constructed only in tests. Live builds `PeerRollBackward`. |
| 4 | `select_best_chain` | `crates/ade_core/src/consensus/fork_choice.rs:96` | ENFORCED but LIVE-UNCALLED | Multi-candidate, k-bounded, order-independent; `--mode node` never calls it. |
| 5 | `chain_selector` orchestrator (`process_stream_input` / `process_rollback`) | `crates/ade_runtime/src/consensus/chain_selector.rs` | TESTED-BUT-UNWIRED | No production caller. **Linear single-header fragments only** (`rollback_depth 0`) — a *partial* template, not the multi-peer/forking answer. SEAMS candidate #10. |
| 6 | `ade_core_interop::follow` (`FollowState` / `ingest_rollforward`) | `crates/ade_core_interop/src/follow.rs` | UNWIRED + FORBIDDEN-SHAPE | Per-peer tracker that calls `select_best_chain`, but **mints peer-trusted `ValidatedHeaderSummary`** (*"must never leak into BLUE"*). Shape reference only. |
| 7 | Peer-identity flatten — `NodeSyncItem = Block(Vec<u8>) \| RollBack(Point)` drops `AdmissionPeerEvent`'s `peer: String` | `crates/ade_node/src/node_sync.rs:77-80`, conversion `:203`,`:217` | FLATTEN | N peers → N pumps → one merged channel (`spawn_live_wire_pump_source`); peer identity discarded at `NodeSyncItem`. |
| 8 | BlockFetch `RequestRange{from,to}` wired only for single-block tip follow (`from==to==tip`) | `crates/ade_runtime/src/admission/wire_pump.rs:161,759` | PARTIAL | Range client exists; on-selection range-fetch from a chosen peer does not. |
| 9 | Single-best-peer rollback assumption — *"Origin … unsupported for single-best-peer within k → fail closed"* | `wire_pump.rs:132,542` | ASSUMPTION | Pump rollback handling assumes one followed peer; multi-peer semantics need review. |
| 10 | Stale roadmap comments (*"INERT here"* `:675`; *"Latent until AI-S4"* `node_sync.rs:856`; *"Not yet wired: L4 peer BlockFetch"* `:42`) | various | STALE-COMMENT | Were wired by N-AI / N-F-G-C; cleanup, not gaps. |

**Live dispatch confirmed:** `run_relay_loop_with_sched` routes `venue_role == VenueRole::Participant →
run_participant_sync` (`node_lifecycle.rs:1308-1340`), otherwise `run_node_sync`. The SELECT attach point
(`run_participant_sync`'s `NeedsForkChoice` arm) is live and real; CE-AI-6 exercised this path.
