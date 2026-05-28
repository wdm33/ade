# PHASE4-N-T — Invariant Sketch

> produce_mode real-bootstrap composition: replace the
> `SyntheticForgeInputs` shortcut with `bootstrap_initial_state`-derived
> state, thread chain state forward across forges via a linear
> `ChainEvolution` typestate, and wire the `BroadcastBlock` effect to the
> served chain with full forge→serve evidence.
>
> IDD Part I artifact. Produced before `/cluster-plan`. No implementation.
> Baseline HEAD at sketch time: `dbee4d5`.

---

## Scope lock (N-T = Problem 1 only — DECIDED)

N-T is locked to **real in-memory production state**, in one continuous
run. It solves exactly:

> real **BootstrapState** → linear **ChainEvolution** → coherent
> **ForgeRequestContext** → **accepted block** → **push_atomic / served
> evidence**.

Two distinct problems were being conflated because admission mode bundles
them. They are separated here:

- **Problem 1 (N-T):** get the *real* ledger/stake/tip into produce_mode
  at startup (not the synthetic zero-stake placeholder) and keep the chain
  coherent block-to-block in memory.
- **Problem 2 (DEFERRED to a later storage/recovery cluster — "N-U"):**
  durable restart — WAL append of forged blocks, ChainDB block store,
  snapshot cadence, crash → warm-start → recover the forged chain.

State-surface decision:

| Surface | Meaning | In N-T? |
|---|---|---|
| `ChainEvolution` (in memory) | "what chain state am I building on right now?" | **YES** |
| WAL | "what authoritative transitions happened, for replay after crash?" | **NO — defer to N-U** |
| Snapshot / ChainDB | "what full state/block store do I restart from?" | **NO — defer to N-U** |

**WAL-only persistence is explicitly REJECTED for N-T.** `bootstrap_initial_state`
warm-start reads the snapshot/ChainDB path, *not* the WAL (verified in
§0), so appending forged blocks to the WAL without ChainDB/snapshot
persistence would create the *appearance* of durability with no real
warm-start recovery. Do not add it.

**Persistence non-claim (N-T must state plainly, not as a weakness):**
- N-T does **not** claim crash recovery of forged blocks.
- After process restart, any forged-but-not-persisted in-memory chain is
  **not** recovered.
- This is acceptable for the N-T block-acceptance artifact and is **not**
  a substitute for DC-STORE / DC-WAL enforcement.

OQ resolutions baked in: **OQ1** re-run `self_accept` on bytes; **OQ3**
absolute slot in scope; **OQ4** produce_mode constructs the real
`(ledger, chain_dep, era_schedule, ledger_view)` from the operator seed +
consensus-inputs bundle (mirroring admission's *construction*, not its
persistence) and routes it through the **cold-start branch** of
`bootstrap_initial_state` (empty in-memory store ⇒ cold-start ⇒ returns
the seeded `genesis_initial`); warm-start stays test-only until N-U;
**OQ5** new GREEN `ChainEvolution` typestate; **OQ7** cold-start forging
is in scope (a freshly-seeded producer forges from the seeded ledger).
Still open for `/cluster-plan`: **OQ2**, **OQ6**.

---

## 0. Framing from code reading (load-bearing — do not re-derive)

Verified directly against source at HEAD `dbee4d5`:

- **`bootstrap_initial_state` is the SOLE bootstrap `pub fn`**
  (`ade_runtime::bootstrap`, CN-NODE-01). It returns
  `(LedgerState, PraosChainDepState, Option<ChainTip>)` and already has a
  cold-start (genesis) / warm-start (snapshot + replay-forward) branch.
  Signature requires `BootstrapInputs { chaindb, snapshot_store,
  era_schedule, ledger_view, genesis_initial }`.
- **produce_mode bypasses it entirely.** `produce_mode.rs:116` parses
  genesis via `parse_simple_genesis_json`; `:166` builds a
  `SyntheticForgeInputs` via `build_synthetic_forge_context` (`:788`).
  `bootstrap_initial_state` is never called.
- **The node is structurally a never-leader.**
  `build_synthetic_forge_context` sets `stake_fraction = (0,1)`, empty
  `LedgerState::new`, `start_slot=0` era schedule, `block_number=1`,
  `prev_hash=0`. With zero stake, `verify_and_evaluate_leader` always
  returns `NotEligible`, so `RequestForge` always yields
  `ForgeNotLeader`, so **`BroadcastBlock` is unreachable dead code today.**
  ⇒ real bootstrap state (S1/S2) is a *hard precondition* for the
  broadcast path (S4) to ever fire.
- **The coordinator already does the GREEN half.** On `ForgeSucceeded`
  it updates `chain_tip`, emits `BlockForged`, emits
  `BroadcastBlock { artifact }`, increments `broadcast_queue_size`
  (`coordinator.rs:489–509`). The gap is RED-only.
- **`BroadcastBlock` arm is a no-op** (`produce_mode.rs:945`:
  `// N-R-B (B2) wires push_atomic here.`) and `_served_chain_handle`
  is constructed and discarded (`:209`).
- **`ProducerLogEvent::BlockServed { peer_id, slot, hash, bytes_len }`
  already exists** (`producer_log.rs:154`). N-T must *emit* it on the
  serve path, not add it.
- **`push_atomic(accepted: AcceptedBlock) -> ServedTip`** exists
  (`served_chain_handle.rs:101`) and is the single served-chain writer
  (CN-SNAPSHOT-01). `drain_and_admit` (GREEN, `broadcast_to_served.rs:37`)
  is the queue-batch alternative.
- **The existing `ci_check_node_binary_uses_single_bootstrap.sh` polices
  "called more than once"** (TRACEABILITY §CN-NODE-01) — it does NOT
  catch a "called zero times via synthetic bypass." The bypass was
  introduced by N-Q S5 (when produce_mode shipped), so the duplicates-only
  gate was correct at N-K close; N-T closes the new hole.

**Pure-transformation check: PASSES.** Authoritative core is
`forge(base_ledger, chain_dep, slot, keys, genesis, era_schedule,
pool_distr) → (block_bytes, post_state)` (deterministic given keys),
`evolve(prior_evolution, forge_result) → next_evolution` (pure
transition), `served_chain_admit(snap, accepted) → snap'` (pure BLUE).
Sole nondeterminism (wall-clock → absolute slot) is RED and canonicalized
as `slot: u64` before entering BLUE. The concept is understood.

---

## 1. What must always be true

- **A1 — bootstrap-derived forge state.** produce_mode's initial forge
  state `(LedgerState, PraosChainDepState, Option<ChainTip>)` is the value
  returned by `bootstrap_initial_state` — never synthesized inline.
  *(strengthens CN-NODE-01, CN-PROD-02)*
- **A2 — real forge context.** The `ForgeRequestContext` for slot S is
  derived deterministically from the current `ChainEvolution` state +
  genesis + era schedule + the `PoolDistrView` projected from the
  *current* ledger. `base_state`, `chain_dep_state`, `block_number`,
  `prev_hash`, `prev_opcert_counter`, `pool_distr_view`, `eta0` are all
  functions of evolving state, never constants. *(NEW — CN-PROD-03)*
- **A3 — chain-forward continuity.** Every successful forge at slot S
  consumes the post-state of the most recent prior successful forge (or
  the bootstrap state if none): `prev_hash(S)=hash(prev block)`,
  `block_number(S)=block_number(prev)+1`,
  `base_state(S)=post_ledger(prev)`. Single linear extension; no fork off
  a stale base. *(NEW — DC-PROD-03)*
- **A4 — broadcast reaches served chain.** Every
  `BroadcastBlock { artifact }` effect admits the artifact into the served
  `ServedChainSnapshot` via the single `push_atomic` authority before the
  next slot tick is processed. The snapshot peers block-fetch from
  contains every self-accepted forged block. *(NEW — CN-PROD-04; carries
  CN-SNAPSHOT-01)*
- **A5 — evidence completeness.** Every block actually served to a peer
  emits the existing closed `ProducerLogEvent::BlockServed`. The evidence
  log records the full forge→serve lifecycle (`BlockForged`, then on peer
  fetch `BlockServed`). *(carries DC-PROD-01)*
- **A6 — absolute slot from bootstrap tip.** The first slot produce_mode
  forges for is derived from `ChainTip.slot` (warm-start: `tip.slot + 1`)
  or from genesis (cold-start), not a fixed `0`. Wall-clock → absolute
  slot is RED (clock seam) and enters the forge as canonical
  `slot: u64`. *(NEW; cross-ref DC-NODE-03 — per OQ3 decision)*
- **A7 — forge composition unchanged, now reachable.** `run_real_forge`
  and the `self_accept` gate are byte-unchanged and now exercised
  end-to-end against real state. *(carries CN-FORGE-01/02; strengthens
  DC-CONS-18)*

## 2. What must never be possible

- Forging against synthetic/constant base state.
  **`SyntheticForgeInputs` + `build_synthetic_forge_context` are deleted,
  not merely unused.**
- Forging slot S+1 against the pre-forge-S state (stale base / fork off a
  stale tip) — made unrepresentable by the linear `ChainEvolution`
  typestate (OQ5 decision).
- A `BroadcastBlock` effect silently dropped (today's no-op arm).
  Broadcast must push-to-served or fail-closed.
- produce_mode deriving initial state through any path other than
  `bootstrap_initial_state` — the exact bypass the existing
  `ci_check_node_binary_uses_single_bootstrap.sh` (duplicates-only) does
  not catch.
- A second served-chain writer (must remain single `push_atomic` —
  CN-SNAPSHOT-01).
- Emitting `BlockServed` for a block absent from the served snapshot
  (no over-stating — `feedback_shell_must_not_overstate_semantic_truth`).
- More than one block per slot, or forging for slot ≤ current tip slot
  (no retroactive/duplicate forge — already in coordinator; must survive
  real wiring).

## 3. What must remain identical across executions (deterministic surface)

- Forged block bytes for fixed (bootstrap triple, slot sequence,
  KES/VRF/cold keys, genesis, era schedule, pool-distr view).
- The chain-evolution series: ordered
  `(block_number, prev_hash, post_ledger fingerprint, post_chain_dep)`.
- `ServedChainSnapshot.fingerprint()` after a fixed forged-block sequence.
- The `ProducerLogEvent` sequence on the replayable surface. RED metadata
  (wall-clock timestamps, socket addresses) is excluded per DC-PROD-01.

**Nondeterminism canonicalized:** wall-clock → absolute slot (RED clock
seam, DC-NODE-03). The start slot derives from the bootstrap
`ChainTip.slot` (OQ3 decision); the per-slot value enters BLUE as
canonical `slot: u64`.

## 4. What must be replay-equivalent

- **R1 (forge replay).** Same canonical inputs → byte-identical
  forged-block bytes + `BroadcastBlock` artifacts. *Extends DC-CONS-18 /
  DC-PROD-02 over real state.*
- **R2 (served-chain replay).** Same ordered forged-block sequence →
  byte-identical snapshot fingerprint. *Proven for `drain_and_admit`;
  extend to the `push_atomic` path.*
- **R3 (bootstrap→forge replay).** Two runs from the same on-disk state
  bootstrap to byte-identical initial state (already DC via
  `bootstrap_two_runs_produce_byte_identical_state`) AND forge a
  byte-identical first block.

## 5. State transitions in scope

```
T0  Absolute-slot derivation (RED clock seam, NEW — per OQ3)
    (ChainTip | genesis, wall_clock) → slot: u64   (canonical input to T2/T3)

T1  Bootstrap (GREEN, exists, reused unchanged)
    (BootstrapInputs) → Result<(LedgerState, PraosChainDepState, Option<ChainTip>), BootstrapError>

T2  Forge-context derivation (BLUE authorities + GREEN glue, NEW)
    (ChainEvolution, slot, genesis, era_schedule) → Result<ForgeRequestContext, ContextError>
      — pool_distr_view via ade_ledger::consensus_view (BLUE)
      — leader_schedule_answer via ade_core::consensus::leader_schedule (BLUE)

T3  Forge execution (RED→BLUE→RED→BLUE, exists as run_real_forge; now fed real ctx)
    (slot, kes_period, &ForgeRequestContext, &mut ProducerShell)
      → CoordinatorEvent::{ForgeSucceeded | ForgeNotLeader | ForgeFailed}

T4  Chain-forward evolution (GREEN typestate, NEW — per OQ5)
    (ChainEvolution, ForgeSucceeded{artifact, post_state}) → Result<ChainEvolution, EvolveError>
      — rejects out-of-order / stale base; exposes pre-forge base for T5

T5  Broadcast-to-served (RED wrapping BLUE, wire NEW — per OQ1)
    re-run self_accept(artifact.bytes, pre_forge_base_ledger, pre_forge_chain_dep,
                       era_schedule, pool_distr_view) → AcceptedBlock
    then push_atomic(AcceptedBlock) → Result<ServedTip, PushError>
      — ORDERING: T5 re-self_accepts against the PRE-forge base state the block
        was forged on, so it runs against the pre-advance ChainEvolution snapshot
        (before / consistent-with T4's advance).

T6  Serve-to-peer evidence (RED, partly exists)
    on block-fetch dispatch reading served snapshot → emit BlockServed
```

## 6. TCB color hypothesis

- **BLUE (all exist):** `consensus_view` pool-distr projection;
  `leader_schedule` query; `served_chain_admit`; `forge_block` +
  `self_accept`; `verify_and_evaluate_leader`.
- **GREEN:** `bootstrap_initial_state` (already GREEN, CN-NODE-01);
  forge-context assembly glue (T2); **`ChainEvolution` typestate (NEW —
  per OQ5)**; `drain_and_admit` (exists); coordinator (exists).
- **RED:** produce_mode main loop; absolute-slot derivation / wall-clock
  ticker (T0); `push_atomic` (watch channel); evidence file I/O;
  ProducerShell signing.

OQ5 resolved: `ChainEvolution` is a new GREEN linear typestate making
"forge against stale base" structurally unrepresentable.

## 7. Resolved decisions (OQ1 / OQ3 / OQ5)

- **OQ1 → re-run `self_accept` on bytes.** T5 reconstructs the
  `AcceptedBlock` by re-running BLUE `self_accept` on `artifact.bytes`
  against the pre-forge base state, then `push_atomic`. Clean GREEN/RED
  boundary; re-validates (cheap, pure). Adds the ordering constraint in
  T5 above.
- **OQ3 → absolute slot in N-T scope.** N-T derives the start slot from
  `ChainTip.slot` / genesis (T0, A6); couples to the clock seam
  (DC-NODE-03).
- **OQ5 → new GREEN `ChainEvolution` typestate.** Linear typestate;
  illegal-states-unrepresentable.

## 8. Open questions

### Resolved (see Scope lock + §7)
- **OQ1** → re-run `self_accept` on bytes. **OQ3** → absolute slot in
  scope. **OQ4** → cold-start `bootstrap_initial_state` from operator
  seed via empty in-memory store; no persistence; warm-start deferred to
  N-U. **OQ5** → GREEN `ChainEvolution` typestate. **OQ7** → cold-start
  forging in scope.

### Carried into `/cluster-plan`
- **OQ2 (push_atomic vs drain_and_admit).** Coordinator emits one
  `BroadcastBlock` per forge → `push_atomic` is the direct fit;
  `drain_and_admit` stays the N-G queue-batch alternative. Confirm at
  slice planning.
- **OQ6 (CI gate framing).**
  `ci_check_produce_mode_uses_bootstrap_initial_state.sh` enforces a
  positive grep (a `bootstrap_initial_state` call in produce_mode) + a
  negative grep (no `SyntheticForgeInputs` / inline `LedgerState::new`
  forge-base). Lands as a strengthening of BOTH CN-NODE-01 and
  CN-PROD-02. (The strengthening is *single-bootstrap-chokepoint, no
  parallel synthetic path* — NOT a warm-start/durability claim.)

## 9. Proposed registry entries

3 new (`status = "declared"`, `introduced_in = "PHASE4-N-T"`) +
5 strengthenings (`strengthened_in += "PHASE4-N-T"`). IDs append to the
`CN-PROD` / `DC-PROD` topic family (next free: CN-PROD-03, CN-PROD-04,
DC-PROD-03).

**NEW**
- **CN-PROD-03** — Bootstrap-derived forge state. produce_mode's forge
  base state is the `bootstrap_initial_state` triple + ledger-projected
  `PoolDistrView` + bootstrap-tip-derived absolute slot;
  `SyntheticForgeInputs` removed. `cross_ref = [CN-NODE-01, CN-PROD-02,
  CN-FORGE-02, DC-NODE-03]`.
- **CN-PROD-04** — Broadcast reaches served chain. Every `BroadcastBlock`
  admits its artifact to the served snapshot via the single `push_atomic`
  authority (re-self_accept reconstruction) before the next tick; no
  no-op broadcast; `BlockServed` emitted only for served blocks present
  in the snapshot. `cross_ref = [CN-SNAPSHOT-01, DC-PROD-01]`.
- **DC-PROD-03** — Chain-forward continuity + replay. Each forge linearly
  extends the prior post-state via the `ChainEvolution` typestate; same
  (bootstrap triple, slot sequence, keys) → byte-identical
  chain-evolution series and forged bytes. `cross_ref = [T-DET-01,
  DC-CONS-18, DC-PROD-02, CN-FORGE-01]`.

**STRENGTHENINGS (`strengthened_in += "PHASE4-N-T"`)**
- `CN-NODE-01` — sole bootstrap now *used* by `--mode produce`
  (cold-start branch), not bypassed. *(single-chokepoint, no parallel
  synthetic path — NOT a warm-start claim)*
- `CN-PROD-02` — synthetic forge-state shortcut removed.
- `CN-FORGE-01` — ForgeSucceeded path now reachable end-to-end against
  real state.
- `CN-SNAPSHOT-01` — `push_atomic` now actually driven by the broadcast
  arm. *(this is the IN-MEMORY served-chain watch snapshot for
  block-fetch serving — NOT the durable snapshot/ChainDB; no durability
  claim)*
- `DC-CONS-18` — forge transcript equivalence now over real bootstrap
  state.

**EXPLICITLY NOT STRENGTHENED in N-T (durability is Problem 2 / N-U):**
N-T must NOT mark warm-start, WAL replay, or full durability as enforced.
No `DC-WAL-*`, `DC-STORE-*`, `CN-WAL-*`, `CN-STORE-*`, `CN-ANCHOR-*`, or
`CN-SEED-*` strengthening belongs in N-T. Forged-block durability (WAL
append + ChainDB store + snapshot cadence + crash→warm-start recovery) is
a later storage/recovery cluster (provisionally **N-U**).
