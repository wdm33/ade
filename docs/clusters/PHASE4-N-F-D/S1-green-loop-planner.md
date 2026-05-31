# PHASE4-N-F-D — Slice S1: GREEN loop planner

> **Status:** slice doc (IDD Part IV). Companion to the cluster doc
> `cluster.md` (S1 row) and the cluster/slice plan
> `../../planning/phase4-n-f-d-cluster-slice-plan.md`. Code-verified against
> HEAD `b1b5102` at authoring.

> **Slice S1 in one line:** add the pure GREEN decision function that selects
> each relay-loop lifecycle step — `plan_loop_step(loop_state, sync_status,
> shutdown_status) -> LoopStep` over a closed `{ SyncOnce, Idle, HaltCleanly }`
> vocabulary — so the RED loop (S2) can be a thin composer that owns no
> authority.

## 1. Slice identity
- **Cluster:** PHASE4-N-F-D (live relay run-loop, hermetic spine).
- **Slice:** S1 — GREEN loop planner.
- **Module (new):** `crates/ade_node::run_loop_planner` (GREEN-by-content,
  `//! GREEN` banner inside the RED `ade_node` crate — precedent:
  `ade_node::ba02_evidence`).
- **Lands tested-but-unwired** (precedent: every N-F-C library surface). S2
  wires it.

## 2. Invariant scope
- **CN-NODE-02 (planner half):** the loop's per-iteration step selection is a
  pure, total, deterministic function over **closed lifecycle-level inputs**;
  it cannot encode a ledger / chain-selection / leadership / forge / evidence
  decision. The closed `LoopStep` vocabulary makes an authority decision
  unrepresentable as a planner output.
- **T-REC-03 (introduced `declared`):** the planner is deterministic — same
  inputs ⇒ same `LoopStep` — a precondition for loop-as-replay byte-identity
  (enforced in S3a).

## 3. Pre-conditions
- N-F-C closed (`b046b7f`): `node_sync` (`run_node_sync` / `NodeBlockSource`)
  exists as a library surface; `node_lifecycle` is the marked lifecycle owner.
- No new dependency: the planner is a self-contained pure module over its own
  closed enums.

## 4. Implementation boundary
- **Closed output:** `enum LoopStep { SyncOnce, Idle, HaltCleanly }` — no
  `#[non_exhaustive]`.
- **Closed inputs (each one orthogonal, lifecycle-level only):**
  - `enum ShutdownStatus { Running, ShutdownRequested }` — operator intent.
  - `enum SyncStatus { WorkAvailable, NoWorkReady }` — momentary readiness:
    is a block ready to pump now (i.e. is `next_block()` expected to make
    progress)? **Not** a block, hash, slot, or verdict — a yes/no.
  - `enum LoopState { Continuing, Ending }` — structural feed liveness: has
    the source feed ended (clean disconnect / closed-and-drained)?
- **The decision table (precedence: shutdown > drain-available > ended > idle):**

  | `shutdown_status` | `sync_status` | `loop_state` | → `LoopStep` |
  |---|---|---|---|
  | `ShutdownRequested` | * | * | `HaltCleanly` |
  | `Running` | `WorkAvailable` | * | `SyncOnce` |
  | `Running` | `NoWorkReady` | `Ending` | `HaltCleanly` |
  | `Running` | `NoWorkReady` | `Continuing` | `Idle` |

  Rationale: a shutdown halts promptly at the next boundary (does not start
  new work); otherwise available work drains first (even while ending);
  a drained+ended feed halts cleanly; an open feed with no work right now
  idles. Total over all 2×2×2 = 8 input combinations (exhaustive `match`, no
  wildcard arm).
- **Signature:** `pub fn plan_loop_step(loop_state: LoopState, sync_status:
  SyncStatus, shutdown_status: ShutdownStatus) -> LoopStep`. Pure: no `&self`,
  no I/O, no clock, no allocation, no `await`.
- **`lib.rs`:** `pub mod run_loop_planner;`.

## 5. Proof obligations (exit criteria — CE-D-1)
- [ ] `plan_loop_step` is a pure free function returning the closed `LoopStep`;
      `match` is exhaustive with no wildcard arm; no input or output type is
      `#[non_exhaustive]`.
- [ ] **Exhaustive decision-table test** `plan_loop_step_decision_table_is_total`
      asserts the mapping for all 8 input combinations.
- [ ] **Determinism test** `plan_loop_step_is_deterministic` asserts repeated
      calls with identical inputs return identical `LoopStep`.
- [ ] **Shutdown-precedence test** `shutdown_halts_even_with_work_available`
      asserts `ShutdownRequested + WorkAvailable + Continuing → HaltCleanly`.
- [ ] New CI gate `ci/ci_check_loop_planner_closed.sh` (exit 0) asserts: the
      module exists with a `//! GREEN` banner; `LoopStep` is defined and not
      `#[non_exhaustive]`; `plan_loop_step` is defined; **no authority/I/O
      token** appears in the module (comments + `#[cfg(test)]` stripped first):
      `pump_block`, `run_node_sync`, `run_real_forge`, `forge_one_from_recovered`,
      `correlate`, `Ba02Manifest`, `ChainDb`, `LedgerState`, `BlockHash`,
      `ChainTip`, `PumpTip`, `SlotNo`, `put_block`, `AdvanceTip`,
      `rollback_to_slot`, `std::fs`, `tokio`, `SystemTime`, `Instant`,
      `HashMap`, `await`, and no wildcard `_ =>` arm.

## 6. TCB color
- **GREEN:** `ade_node::run_loop_planner` (pure decision function + closed
  enums; `//! GREEN` banner). No BLUE change; no RED behavior.

## 7. Forbidden in this slice (inherits the cluster Forbidden list)
- No I/O, no clock, no `await`, no allocation in the planner.
- No authority vocabulary in the planner module (the gate's token ban).
- No wildcard `match` arm; no `#[non_exhaustive]` on any planner type.
- No wiring into the binary run path (that is S2) — S1 lands tested-but-unwired.
- No new canonical type / WAL entry / checkpoint format / BLUE change.

## 8. Replay / determinism obligations
- The planner is a total pure function: same inputs ⇒ byte-identical output.
  This is the determinism precondition T-REC-03 builds on (S3a flips it to
  `enforced`). No corpus entry; proven by unit tests.

## 9. Slice completion checklist
- [ ] `run_loop_planner.rs` written with `//! GREEN` banner + closed enums +
      `plan_loop_step` + the 3 tests.
- [ ] `lib.rs` declares `pub mod run_loop_planner;`.
- [ ] `ci/ci_check_loop_planner_closed.sh` added, executable, exits 0.
- [ ] `cargo build -p ade_node` clean; `cargo test -p ade_node run_loop_planner`
      green; `cargo fmt` applied; the new gate passes.
- [ ] Slice doc committed standalone (docs:) before implementation; impl
      committed (feat:) after green.

## Authority
Registry IDs `CN-NODE-02` (planner half) + `T-REC-03` (`declared`). The cluster
doc `cluster.md` and `docs/ade-invariant-registry.toml` are authoritative; this
slice doc refines, it does not override.
