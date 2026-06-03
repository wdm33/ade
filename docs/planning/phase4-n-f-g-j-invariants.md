# PHASE4-N-F-G-J — Node forge when feed is empty/at tip — Invariant Sketch

> Status: invariant sketch (planning artifact, pre-cluster). Declares `DC-NODE-08`
> (semantic scheduling) + `CN-NODE-04` (closed diagnostic event vocabulary).
> Surfaced by the C1 forge dry-run, AFTER PHASE4-N-F-G-I fixed the anchor-lineage
> WarmStart gap and the whole-second-`systemStart` regen fixed `SystemStartParseFailure`.

## §0 The gap (canonical framing)

On a fresh **sole-producer** private net (C1 Conway, magic 42, Ade-dominant stake) Ade
fully WarmStart-recovers (epoch 0), loads KES/VRF/cold/opcert, and wires the live
WirePump feed to the genesis follower node — but the upstream peer has **zero blocks**
(Ade is the only producer), so the WirePump channel disconnects, the feed reaches
`is_ended`, and the closed planner `plan_loop_step` **halts before any `LoopStep::ForgeTick`**.
Ade can never produce the **first** block.

This is a **node-spine producer-scheduling gap**, not a serve (G-H is fine) or admission
failure. The forge-tick gate (`Continuing` feed → `ForgeTick`; `is_ended` feed → halt) is
**correct for joining an already-producing network** (preprod) but a **chicken-egg** on a
net whose first blocks Ade itself must produce.

Expressible as a pure transformation? **Yes** — the only semantic change is to one pure
planner: `plan_loop_step(LoopState, feed_state, slot) → LoopStep`. The forge
(`forge_one_from_recovered`) is reused unchanged. The only nondeterminism is the slot/tick
wall-clock (which real slots arrive) — already a captured canonical input (RED shell),
unchanged by this cluster.

### Load-bearing distinction (OQ1 resolved) — empty/at-tip vs peer-loss/error

Not every feed end is forge-eligible. Use a **closed** distinction:

| Feed state | Forge-eligible? | Policy |
|---|---|---|
| `Empty` / `AtTip` / `NoBlockAvailable` (peer alive, nothing to send; or a clean empty source) | **YES** (subject to §1) | producer tick may run from the recovered base |
| `PeerEndedCleanly` | only if it reduces to a defined clean empty-source case | else halt per existing policy |
| `PeerLost` / `DecodeError` / `ProtocolError` / `SourceInvalid` | **NO** | **fail closed or halt** per existing lifecycle policy — never masked as sole-producer mode |

This prevents masking real sync/network failures as "sole producer mode."

### Producer-intent signal (OQ2 resolved) — general, never a C1 flag

The forge allowance is gated on a **general, non-private** signal that the node is a
producer with a valid base — **all** of:

- `ForgeIntent::On` (complete operator key material), **and**
- a WarmStart **recovered/imported** authoritative base (recovered tip), **and**
- the seed-epoch lineage present, **and**
- valid slot/epoch/KES/opcert guards.

> "If the node is configured to forge and has a valid recovered base, it may **evaluate**
> forge eligibility even when the feed is empty/at-tip." This applies to C1 and future
> single-producer/dev/private nets **without** a private-only `--sole-producer`/C1 branch.

## §1 What must always be true
1. **(NEW, DC-NODE-08)** `--mode node` may run the producer tick from the recovered
   authoritative base when the feed is `Empty|AtTip|NoBlockAvailable`, **only if** (a) the
   base is explicitly WarmStart-recovered/imported (recovered `SeedEpochConsensusInputs`
   surface + recovered tip + seed-epoch lineage), (b) `ForgeIntent::On` with the complete
   operator key set, (c) slot/epoch/KES guards pass (`DC-EPOCH-03` + BLUE leader check +
   KES-period/opcert), (d) the forged block flows through `self_accept → SelfAcceptedHandoff
   (DC-NODE-06) → ServedChainView (DC-NODE-07)`.
2. The forge base is **always** the recovered surface (`CN-CINPUT-03` / `DC-CINPUT-02b`).
3. The forged block is **byte-identical** to the same recovered-base + slot + keys forge
   today (`DC-FORGE-01` / `CN-FORGE-01..04` — the forge composition is untouched).
4. The durable/recovered tip advances **only** through the accepted path; never as a side
   effect of forge scheduling.
5. **(NEW, CN-NODE-04)** `--mode node` emits a **closed, allow-listed diagnostic event
   vocabulary** for feed/forge scheduling, with closed reason/outcome enums.

## §2 What must never be possible
1. Forge from an **unanchored / from-genesis-constructed / stale** base.
2. Forge that **bypasses** `import_live_consensus_inputs` / the recovered surface.
3. A **durable tip advance from forge alone** (without `self_accept → handoff`).
4. **Serve of non-self-accepted bytes** (`DC-NODE-06/07` unweakened).
5. A **co-producer workaround** or a **private-only/C1-only flag/branch**.
6. The forge tick running **off-epoch / off-slot / with stale KES** — the empty-feed
   allowance relaxes **no** guard.
7. A `PeerLost / DecodeError / ProtocolError / SourceInvalid` feed state being treated as
   forge-eligible (masking a real network/sync failure as sole-producer mode).
8. S1 events used as **consensus / acceptance / BA-02** evidence; any **stringly-typed
   authoritative error**; any S1-driven change to behavior/scheduling/authority; the
   **planner reading** any CN-NODE-04 event (emit-only — see §6).
9. **No `RO-LIVE-01/06` flip** — peer ACCEPT remains operator-gated, proven only by the
   peer validation log through `ba02_evidence::correlate`.

## §3 What must remain identical across executions
- The forged block bytes for a given `(recovered base, slot, keys)` (`DC-FORGE-01`).
- The pure planner decision `plan_loop_step(LoopState, feed_state, slot)` — same inputs →
  same `LoopStep`, including the new `Empty|AtTip|NoBlockAvailable + recovered base + due
  slot → ForgeTick` branch and the unchanged `PeerLost|…→ halt` branch.
- The S1 event sequence for a given canonical `(feed-state transitions, slot sequence,
  forge outcomes)` — deterministic given inputs (though not consensus evidence).
- *Not* identical (RED, already so): **which** wall-clock slots arrive — the slot ticker is
  the captured canonical input; this cluster adds wall-clock nowhere else.

## §4 What must be replay-equivalent
- Replaying the same ordered canonical inputs (recovered base + captured slot sequence +
  keys) produces **byte-identical** forged block(s) and the **same** `self_accept → handoff
  → ServedChainView` effects (`DC-NODE-06/07`, `DC-FORGE-01` preserved).
- The new planner branch introduces **no new authoritative state** and **no new canonical
  type** on the forge path — only a different `LoopStep` from the *existing* inputs.
- S1 events are **operational tier** — replay-deterministic-given-inputs but explicitly
  **outside** the consensus-evidence/replay-equivalence weight class (they never gate
  acceptance and are never read back by authority).

## §5 State transitions in scope
- **Planner (the one semantic change), GREEN:**
  `plan_loop_step(LoopState{recovered_base: present, forge_intent: On, …}, feed, slot=S) →`
  - **eligible feed** `feed ∈ {Empty, AtTip, NoBlockAvailable}` and `S` a due leader slot
    and epoch/KES guards pass → `Ok(LoopStep::ForgeTick{from_recovered, slot:S})`
  - **ineligible feed** `feed ∈ {PeerLost, DecodeError, ProtocolError, SourceInvalid}` →
    `Ok(LoopStep::Halt)` / fail-closed **per existing lifecycle policy** (unchanged)
  - otherwise (not leader / guards fail / no recovered base / forge off) →
    `Ok(LoopStep::Idle|Halt)` (unchanged).
- **Forge (reused, unchanged), BLUE-driven:**
  `forge_one_from_recovered(recovered_base, slot:S, keys) → Result<(ForgeOutcome,
  Option<SelfAcceptedHandoff>), ForgeError>` — on success self_accepts → handoff →
  `ServedChainView`.
- **Observability (S1), RED emit over a closed GREEN vocabulary, emit-only:**
  `observe(LoopState, transition) → NodeSchedEvent` (closed; see CN-NODE-04) — pure
  projection, **no** effect on, and **never** consumed by, the planner.

## §6 TCB color hypothesis
- **BLUE (reused, unchanged):** `forge_one_from_recovered`'s forge composition + leader
  check + `self_accept` (`CN-FORGE-*`, `DC-FORGE-01`). *Any BLUE change is a red flag → reject.*
- **GREEN:** `plan_loop_step` / `forge_slot_status` (the pure scheduling planner — the S2
  change lives here); the closed `NodeSchedEvent` vocabulary (S1).
- **RED:** `run_node_sync` / the relay run loop (executes the planner), the `NodeBlockSource`
  feed states + the feed-state classification, the JSONL event emission (S1), the slot
  ticker (unchanged).
- **Emit-only hard line:** the planner (GREEN) may **emit** CN-NODE-04 events; it must
  **never consume** them. Events flow one-directionally planner → log, never log → planner.
- **Open color question:** is `plan_loop_step` today a separable pure fn (GREEN) or embedded
  in the RED loop? If embedded, S2 must extract the decision to keep it deterministic/testable
  — resolve in `/cluster-doc`.

## §7 Open questions (for `/cluster-plan` → `/cluster-doc`)
1. **Feed-state taxonomy (load-bearing):** the exact closed `feed_state` enum and the
   `is_ended`→taxonomy mapping in `NodeBlockSource` — which concrete WirePump end conditions
   map to `NoBlockAvailable`/`AtTip` (eligible) vs `PeerLost`/`ProtocolError` (ineligible).
   The C1 case (channel disconnected because the genesis peer had nothing) must classify as
   an **eligible** empty/at-tip case, **not** `PeerLost`.
2. **Producer-intent sufficiency:** confirm `ForgeIntent::On + recovered base + lineage` is a
   complete, non-stringly signal (no new flag) and where it is read in the planner.
3. **Termination/liveness:** with an eligible empty feed and **not** a leader slot, the loop
   must `Idle` (bounded), not busy-loop; confirm the existing `NoWorkReady` idle/backpressure
   path bounds it and bounds forge attempts.
4. **S1-before-S2 sufficiency:** confirm S1's `forge_tick_skipped{reason}` → `forge_attempted`
   → `forge_result` events are sufficient to prove the S2 before/after without further
   widening.
5. **C1-vs-C2 invariance:** S2 changes the shared path but is exercised only by C1 (preprod's
   feed is already Continuing); confirm S3 (C1 rerun harness) is the regression proof and the
   Continuing-feed path is byte-unchanged for C2/preprod.

## §8 Slice shape (for `/cluster-plan`)
- **S1 — Closed node feed/forge scheduling events** (FIRST; **no behavior change**). Emit the
  closed CN-NODE-04 vocabulary so the S2 change is provable without black-box guessing.
- **S2 — Empty/at-tip recovered-base forge scheduling** (the DC-NODE-08 semantic change),
  proven by the S1 events (`forge_tick_skipped` → `forge_attempted` → `forge_result`).
- **S3 — C1 rerun harness / runbook update** (only if needed).

## Declared registry rules (this sketch)
- `CN-NODE-04` (tier=operational, status=declared) — closed diagnostic event vocabulary.
- `DC-NODE-08` (tier=derived, status=declared) — the empty/at-tip recovered-base forge
  scheduling semantic. Both flip to `enforced` at their slice close (S1 / S2).
