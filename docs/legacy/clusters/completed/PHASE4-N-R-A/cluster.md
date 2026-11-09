# PHASE4-N-R-A — Real forge composition (cluster doc)

> **Status:** Planning. 4-slice sub-cluster closing the
> `RequestForge → ForgeResult` contract by composing existing
> BLUE primitives (`forge_block`, `self_accept`, VRF
> verification, `is_leader_for_vrf_output`) under a clean
> RED/BLUE split. Replaces the S5 stub forge handler in
> `produce_mode::apply_effects_with_forge_handler`.
>
> **Predecessor:** PHASE4-N-Q (HEAD `c1c4b06`) +
> N-R planning (HEAD `27120b7`).
>
> **Successor:** PHASE4-N-R-B (served snapshot + per-peer
> dispatch).
>
> **Inputs:** [`docs/planning/phase4-n-r-invariants.md`](../../planning/phase4-n-r-invariants.md)
> + [`docs/planning/phase4-n-r-cluster-slice-plan.md`](../../planning/phase4-n-r-cluster-slice-plan.md).

---

## §1 Primary invariant

> The producer-mode forge handler is a closed transition from
> `CoordinatorEvent::RequestForge { slot, kes_period,
> ledger_snapshot_ref, chain_tip }` to exactly one of three
> `ForgeResult` variants — `Succeeded { artifact }`,
> `NotLeader { vrf_output_fingerprint }`, `Failed {
> structured_error }` — with the following load-bearing
> guarantees:
>
> 1. **VRF authority split (DQ-A1).** RED produces the VRF
>    proof using the operator's VRF signing key. BLUE
>    verifies the proof and evaluates leader eligibility from
>    canonical inputs only. **BLUE never sees the VRF / KES /
>    cold signing keys.**
> 2. **Self-accept gate (N1).** `ForgeSucceeded` is
>    emitted only if `self_accept(artifact, chain_tip,
>    ledger_snapshot)` returns `Accepted`. Otherwise the
>    handler MUST emit `ForgeFailed { SelfAcceptRejected }`.
> 3. **KES window gate (N3).** No KES sign occurs with
>    `kes_period` outside the opcert's `[start, start +
>    SUM6_MAX_PERIOD]` window. Out-of-window slot →
>    `ForgeFailed { KesPeriodOutOfRange }`.
> 4. **No retroactive forge (N8).** If wall-clock has
>    advanced past the slot's deadline before the forge
>    handler completes, the coordinator's
>    `SlotMissed { reason: DeadlineExceeded }` path takes
>    precedence; no `BroadcastBlock` effect for the missed
>    slot.
> 5. **Empty-block scope (I9).** The forge handler
>    constructs blocks whose body is the empty transaction
>    set. Mempool integration is a separate cluster; the
>    N-R-A artifact does NOT close the broader TxSubmission
>    obligation.
>
> Replay equivalence (DC-PROD-02 strengthened): for a fixed
> canonical event corpus and fixed `(stake_distribution, eta0,
> opcert public metadata, KES / VRF seeds)`, the
> coordinator's `ProducerLogEvent` stream + the forge
> handler's `ForgeResult` sequence are byte-identical across
> runs.

## §1.5 Doctrine: BLUE/RED split inside the forge handler

The N-Q precedent put the GREEN coordinator + RED key-custody
shell on opposite sides of a closed effect/event boundary. N-R-A
takes the next step: **inside** the RED forge handler, the
work splits BLUE-then-RED-then-BLUE:

```
RED  step 1:  build expected_vrf_input from (slot, eta0)        [canonical bytes]
RED  step 2:  vrf_prove(vrf_sk, expected_vrf_input)
                 -> (vrf_proof, vrf_output)                     [RED signing]
BLUE step 3:  verify_and_evaluate_leader(
                 expected_vrf_input, vrf_vk, vrf_proof,
                 LeaderScheduleAnswer,
              ) -> LeaderCheckVerdict                           [pure verification]
                 |
                 +-- NotEligible -> emit ForgeNotLeader; halt.
                 |
                 +-- Eligible -> continue.
RED  step 4:  kes_sign_at(slot, body_hash, kes_period)
                 -> KesSignature                                [RED signing]
                 |
                 +-- KesPeriodOutOfRange -> emit ForgeFailed; halt.
BLUE step 5:  forge_block(&ProducerTick)
                 -> (ForgedBlock, ForgeEffects)                 [pure construction]
BLUE step 6:  self_accept(forged_bytes, ledger, chain_dep, schedule, view)
                 -> AcceptedBlock | error                       [pure verdict]
                 |
                 +-- error -> emit ForgeFailed { SelfAcceptRejected }; halt.
                 |
                 +-- Accepted -> emit ForgeSucceeded { artifact }.
```

The RED handler is the *orchestrator* of this pipeline; it
calls BLUE primitives but never reads or owns BLUE state. Each
BLUE step is replay-deterministic and consumes only canonical
inputs.

This split is the only configuration that lets N-R-A claim:
- The RED VRF signing key never leaves the producer shell.
- The BLUE leader-check verdict is replayable against a
  fixture corpus without invoking any RED signing primitive.
- A `ForgeSucceeded` outcome is structurally proven by the
  N-R-A test corpus — no synthetic verdict can survive a
  real `self_accept`.

## §2 Scope

### In scope

- **New BLUE module** `ade_core::consensus::leader_check`
  (resolved per OI-A.1 below): a narrow evaluator that
  takes a *caller-provided* `LeaderScheduleAnswer` plus the
  RED-produced VRF proof, verifies the proof, evaluates
  eligibility, and returns a closed two-variant
  `LeaderCheckVerdict` (resolved per OI-A.2).
- **Refactor** `forge_block` (in
  `ade_ledger::producer::forge`) to consume a
  `LeaderCheckVerdict::Eligible` artifact rather than
  calling `is_leader_for_vrf_output` itself. After
  migration: `is_leader_for_vrf_output` is **private to
  `leader_check`** (or deleted outright if no in-module
  caller remains). No external caller may bypass
  `LeaderCheckVerdict` (resolved per the §5 strengthening
  below).
- **Real forge handler** in
  `produce_mode::apply_effects_with_forge_handler`: builds a
  `ProducerTick` from `RequestForge` inputs +
  `producer_shell.{vrf_prove, kes_sign_at}` outputs +
  ledger-snapshot / chain-tip / era-schedule values, calls
  `forge_block` + `self_accept`, maps result to
  `ForgeResult`.
- **Pre-flight proof obligations** (A1 deliverable):
  - **OQ4** — opcert envelope golden fixtures
    (`crates/ade_runtime/tests/fixtures/opcert/`).
  - **OQ7** — Conway genesis golden fixtures
    (`crates/ade_runtime/tests/fixtures/conway_genesis/`).
  - **OQ8** — block-fetch protocol failure reply for
    unknown / partial-overlap ranges (recorded in A1 slice doc).
  - **OQ9** — `dispatch_chain_sync_frame` /
    `dispatch_block_fetch_frame` signature audit (recorded
    in A1 slice doc).
- **Tests:**
  - A2 unit tests on
    `verify_and_evaluate_leader` math (eligible-on-threshold,
    not-eligible-above-threshold, malformed-proof, vk-mismatch).
  - A3 entry-gate test (DQ-A2): `forge_block_accepts_empty_mempool`.
    Failure halts A3.
  - A4 integration tests against a synthetic
    `(stake_distribution, eta0, vrf_sk)` corpus.
- **3 new candidate registry entries** (proposed `declared`
  by A1):
  - `CN-FORGE-01` — `RequestForge → ForgeResult` closed
    surface + self-accept gate.
  - `CN-FORGE-02` — RED/BLUE split for leader-check; BLUE
    never sees VRF signing key.
  - `DC-FORGE-01` — leader-check verify+evaluate determinism
    + replay equivalence over the canonical event corpus.

### Out of scope (deferred to N-R-B, N-R-C, or future clusters)

- `ServedChainSnapshot::push_atomic` integration (N-R-B B2).
- Per-peer dispatch wiring (N-R-B B3).
- Real opcert envelope parser (N-R-C C1; A1 captures
  fixtures only).
- Real Conway genesis parser (N-R-C C2; A1 captures fixtures
  only).
- Mempool integration / non-empty-block forging (future
  TxSubmission cluster).
- Hot-key KES rotation across periods (OP-OPS-04 follow-on).
- Bounty-facing operator-pass evidence (N-R-C C4).

### Honest-scope reminder

A4's integration tests use a **synthetic** stake-distribution
+ eta0 corpus. Real Conway-era ledger snapshots feed into the
real forge handler in N-R-C (via the real genesis parser);
N-R-A proves only that the composition is structurally sound
under canonical inputs.

## §3 Slice index

| Slice | Purpose | Strengthens | Introduces |
|---|---|---|---|
| **A1** | Planning artifacts + 3 registry entries (`declared`) + 4 pre-flight proof obligations captured (OQ4 opcert fixtures, OQ7 genesis fixtures, OQ8 block-fetch failure semantics, OQ9 dispatch signature audit). DQ-A1 module-path clarification settled with user (`ade_core::consensus::leader_check` recommended). **Hard rule:** if A1 bloats beyond capture + write-up, split off into a discrete `N-R-PREFLIGHT` slice. | — (declarative) | `CN-FORGE-01`, `CN-FORGE-02`, `DC-FORGE-01` (all `declared`) |
| **A2** | New BLUE module `ade_core::consensus::leader_check`: `verify_and_evaluate_leader(slot, eta0, vrf_vk, vrf_proof_or_output, leader_schedule_answer) -> LeaderCheckVerdict` + closed two-variant verdict enum (`Eligible { slot, vrf_output, leader_proof }` vs `NotEligible { slot, vrf_output_fingerprint }`) + closed error enum + unit tests on the verify + threshold math. Refactor `forge_block` to consume a `LeaderCheckVerdict::Eligible` artifact. **Hard scope:** the new function MUST NOT depend on `LedgerView`, `EraSchedule`, `ChainDepState`, wall-clock, storage, or RED crates — caller provides the `LeaderScheduleAnswer` derived externally via the authority path. | `DC-FORGE-01` (BLUE side); `T-KEY-01` (BLUE never sees signing keys) | — |
| **A3** | Real forge handler in `produce_mode::apply_effects_with_forge_handler`. **Entry gate:** `forge_block_accepts_empty_mempool` test must pass before any composition wiring; if it fails, halt A3 and revise the `ProducerTick` contract — do NOT patch around it in `produce_mode`. Composition: build `ProducerTick` from `RequestForge` inputs + shell signing outputs → `forge_block` → `self_accept` → `ForgeResult` mapping. | `CN-FORGE-01`; `CN-PROD-02` (real load); `DC-CONS-18` (body-hash binding under real load) | — |
| **A4** | Integration tests against synthetic `(stake_dist, eta0, vrf_sk)` corpus: non-leader slot → `ForgeNotLeader`; leader slot → `ForgeSucceeded` surviving `self_accept`; opcert-period-out-of-range → `ForgeFailed { KesPeriodOutOfRange }`; self-accept failure → `ForgeFailed { SelfAcceptRejected }`. Sub-cluster close. | `CN-FORGE-02` (RED/BLUE split end-to-end); all three N-R-A registry entries flip to `enforced` | — |

## §4 Exit criteria (CI-verifiable)

The sub-cluster is complete only when:

- [ ] **CE-A-1.** `docs/planning/phase4-n-r-{invariants,cluster-slice-plan}.md`
  exist (carry-forward from planning commit).
- [ ] **CE-A-2.** `docs/clusters/PHASE4-N-R-A/{cluster,S1,S2,S3,S4}.md`
  exist.
- [ ] **CE-A-3.** `crates/ade_runtime/tests/fixtures/opcert/`
  contains 4 OQ4 golden fixtures (accepted, malformed
  cborHex, malformed type, wrong arity) + a fixture
  metadata file documenting cardano-node + cardano-cli
  versions used to capture them.
- [ ] **CE-A-4.** `crates/ade_runtime/tests/fixtures/conway_genesis/`
  contains golden fixtures per OQ7 (accepted, missing-required,
  malformed-numeric, extra-inert-key, stringly-int-attempt)
  + the closed-contract behavior table written into A1's
  slice doc.
- [ ] **CE-A-5.** A1 slice doc records the OQ8 block-fetch
  failure reply (verified against `ouroboros-network`
  Haskell reference) + the OQ9 dispatch signature audit.
- [ ] **CE-A-6.** New BLUE module
  `ade_core::consensus::leader_check` exists and exports
  `verify_and_evaluate_leader` + the two-variant
  `LeaderCheckVerdict` (`Eligible { slot, vrf_output,
  leader_proof }` / `NotEligible { slot,
  vrf_output_fingerprint }`) + `LeaderCheckError`.
  `forge_block`'s internal `is_leader_for_vrf_output` call
  is replaced with consumption of a `LeaderCheckVerdict::Eligible`
  artifact.
- [ ] **CE-A-6b.** `verify_and_evaluate_leader` has no
  dependency on `LedgerView`, `EraSchedule`,
  `ChainDepState`, wall-clock, storage, or RED crates.
  Mechanical grep gate over the new module's import set.
- [ ] **CE-A-7.** `verify_and_evaluate_leader` unit tests
  pass: eligible-on-threshold, not-eligible-above-threshold,
  malformed-proof, wrong-vk.
- [ ] **CE-A-8.** `forge_block_accepts_empty_mempool` test
  passes (DQ-A2 entry gate).
- [ ] **CE-A-9.** `produce_mode::apply_effects_with_forge_handler`
  no longer contains the S5 stub (`ForgeNotLeader { vrf_output_fingerprint: [0u8; 8] }`);
  the real composition is in place. Grep gate.
- [ ] **CE-A-10.** A4 integration tests pass: 4-variant
  coverage (NotLeader, Succeeded, KesPeriodOutOfRange,
  SelfAcceptRejected).
- [ ] **CE-A-11.** `CN-FORGE-01`, `CN-FORGE-02`, `DC-FORGE-01`
  flip to `enforced` in `docs/ade-invariant-registry.toml`
  with populated `tests` + `code_locus` fields.
- [ ] **CE-A-12.** `cargo test --workspace --lib` clean
  (no regressions).
- [ ] **CE-A-13.** `ci/ci_check_producer_coordinator_no_secrets.sh`
  still passes (carry-forward from N-Q; BLUE never sees
  signing keys).
- [ ] **CE-A-14.** `T-KEY-01.strengthened_in += "PHASE4-N-R-A"`,
  `DC-CONS-18.strengthened_in += "PHASE4-N-R-A"`,
  `CN-PROD-02.strengthened_in += "PHASE4-N-R-A"` recorded
  at sub-cluster close.

## §5 Hard prohibitions

- **N1 carry-forward.** No `ForgeSucceeded` whose artifact
  fails `self_accept`. The handler MUST emit `ForgeFailed`
  with the verdict as `structured_error`.
- **N3 carry-forward.** No KES sign with `kes_period`
  outside the opcert's window. The shell's existing
  `kes_sign_at` already enforces this at the boundary;
  A3's handler MUST surface the structured error rather
  than silently emit `ForgeNotLeader`.
- **N8 carry-forward.** No retroactive forge. The
  coordinator's `SlotMissed { DeadlineExceeded }` path takes
  precedence over any `BroadcastBlock` for a missed slot.
- **N12 (BLUE-side closure).** No BLUE module imports
  `KesSecret`, `VrfSigningKey`, or `ColdSigningKey`. The
  new `leader_check` module consumes only public-key
  material + canonical inputs. Mechanical grep gate (CI).
- **No mempool integration.** A3 MUST construct a
  `ProducerTick` with `mempool: MempoolState::empty()` +
  `mempool_tx_bytes: vec![]`. Any patch that injects
  transactions is out of N-R-A scope and rejected at review.
- **No silent fallback.** The forge handler MUST return one
  of the three `ForgeResult` variants; ambiguous outcomes
  fail-closed.
- **`is_leader_for_vrf_output` is private to `leader_check`
  (or deleted after migration).** After A2 lands, no
  external caller — including BLUE header-validation paths
  — may bypass `LeaderCheckVerdict`. The helper is either
  scoped `pub(super)`/`pub(crate)` inside `leader_check`,
  or removed entirely if the new module's
  `verify_and_evaluate_leader` covers every existing call
  site. Mechanical grep gate (CI) asserts no module outside
  `leader_check` references the symbol.

## §6 Replay obligations preserved + strengthened

- **DC-PROD-02 (N-Q anchor) — strengthened by A4.**
  The replay-byte-identity claim across two runs of
  `coordinator_step` is now exercised against a real forge
  handler's `ForgeSucceeded` artifacts (not the S5 stub's
  `ForgeNotLeader`-only outputs). The S2 unit test from N-Q
  remains the load-bearing replay anchor; A4 adds an
  integration anchor over `produce_mode`'s composition.
- **T-KEY-01 — strengthened by A2 + A3.**
  BLUE never imports signing-key types; the RED forge
  handler is the sole BLUE-adjacent layer that calls
  signing primitives, and it calls them only via the
  producer shell's existing closed API.
- **DC-CONS-18 — strengthened by A3.**
  Body-hash binding (`header.body_hash` matches the
  blake2b_256 recipe over body buckets) is exercised
  end-to-end under real forge load, not just under the
  synthetic-fixture cross-impl adapter.
- **CN-PROD-02 (N-Q anchor) — strengthened by A3 + A4.**
  KES-period window + no-retroactive-forge are now
  exercised under real forge load; A4's
  `ForgeFailed { KesPeriodOutOfRange }` test is the
  enforcement evidence.

## §7 References

- Predecessor cluster: [`../PHASE4-N-Q/cluster.md`](../PHASE4-N-Q/cluster.md).
- Planning: [`../../planning/phase4-n-r-invariants.md`](../../planning/phase4-n-r-invariants.md)
  + [`../../planning/phase4-n-r-cluster-slice-plan.md`](../../planning/phase4-n-r-cluster-slice-plan.md).
- Existing BLUE primitives composed by N-R-A:
  - `ade_core::consensus::leader_schedule::is_leader_for_vrf_output`
  - `ade_crypto::vrf` (proof verification)
  - `ade_ledger::producer::forge::forge_block`
  - `ade_ledger::producer::self_accept::self_accept`
- N-Q surfaces N-R-A composes on top of:
  - `ade_runtime::producer::producer_shell::{vrf_prove, kes_sign_at}`
  - `ade_runtime::producer::coordinator::{CoordinatorEvent, CoordinatorEffect, ForgeResult}`
  - `ade_node::produce_mode::apply_effects_with_forge_handler`
- Doctrine:
  - [[feedback-hard-closure-gates]] — CE-A-N are hard gates,
    not "best effort".
  - [[feedback-proof-discipline]] — OQ4 / OQ7 / OQ8 / OQ9
    are proof obligations, not assumptions.
  - [[feedback-shell-must-not-overstate-semantic-truth]] —
    the forge handler is the closure of the
    `RequestForge → BroadcastBlock` semantic claim; serving
    bytes to peers is N-R-B's claim.
  - [[feedback-fail-closed-validation]] — `ForgeFailed`
    carries structured errors; no `String` payloads in the
    closed error vocabulary.

---

## §8 Open issues resolved before A2

The three OI-A items from the draft cluster doc are now
resolved by user direction. Recorded here so A1's slice doc
can quote them verbatim:

- **OI-A.1 — `leader_check` crate path. RESOLVED:
  `ade_core::consensus::leader_check`.** The principle from
  DQ-A1 is "do not leave leader eligibility buried inside
  forge construction," not "must be in `ade_ledger`."
  Placing the new BLUE module adjacent to the existing
  `ade_core::consensus::leader_schedule` keeps the authority
  surface coherent. The earlier `ade_ledger`-pathed wording
  is a permitted internal divergence — the authority
  boundary is the load-bearing rule, not the crate name.
- **OI-A.2 — `LeaderCheckVerdict` shape. RESOLVED:
  closed two-variant enum.**
  ```rust
  pub enum LeaderCheckVerdict {
      Eligible {
          slot: SlotNo,
          vrf_output: VrfOutput,
          leader_proof: LeaderProofFingerprint,
      },
      NotEligible {
          slot: SlotNo,
          vrf_output_fingerprint: VrfOutputFingerprint,
      },
  }
  ```
  Exact field names may be adjusted at A2 implementation
  time, but the rule is preserved: only `Eligible`
  exposes full leader material needed for forge;
  `NotEligible` exposes only bounded evidence (a
  fingerprint), never a forge-capable artifact. Bool
  newtypes are rejected — eligibility is not "just a
  boolean," it controls whether downstream code may observe
  forge-capable material.
- **OI-A.3 — `verify_and_evaluate_leader` input shape.
  RESOLVED: caller-provided `LeaderScheduleAnswer`; no
  internal schedule query.** Final signature:
  ```rust
  pub fn verify_and_evaluate_leader(
      slot: SlotNo,
      eta0: Eta0,
      vrf_vk: VrfVerificationKey,
      vrf_proof_or_output: VrfProofOrOutput,
      leader_schedule_answer: &LeaderScheduleAnswer,
  ) -> LeaderCheckVerdict;
  ```
  Caller derives `LeaderScheduleAnswer` from `LedgerView` +
  `EraSchedule` + `ChainDepState` via the authority path
  (`query_leader_schedule`). The new function is a narrow
  BLUE evaluator with **no** dependency on `LedgerView`,
  `EraSchedule`, `ChainDepState`, wall-clock, storage, or
  RED crates. Schedule derivation failures and leader
  evaluation failures stay distinct — divergence
  localization is sharper, fixtures pin one
  `LeaderScheduleAnswer`, and the BLUE evaluator is
  trivially testable.
