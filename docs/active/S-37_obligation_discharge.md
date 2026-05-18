# Slice S-37 Entry Obligation Discharge

> **Status:** Discharged. S-37 implementation may begin.
>
> **Authority Level:** Slice-entry proof discharge (per Phase 4 cluster
> plan §"Cluster N-D — Chain DB & persistence").
>
> **Cluster:** N-D, slice 5 (FINAL). Closes **CE-N-D-1** (chain DB
> survives 1,000 random kill-9 events with zero corruption).
>
> **Predecessors:** S-33 ✓ (trait), S-34 ✓ (redb), S-35 ✓ (snapshots),
> S-36 ✓ (recovery).

This slice ships the durability stress harness: subprocess-based
fault injection that SIGKILLs a worker mid-write to validate the
crash-window invariants from S-34's `run_crash_safety_tests`. The
slice closes cluster N-D and produces the first Phase 4 CE closure
evidence.

---

## Slice scope

**In:**
- A `chaindb_kill_target` binary in `ade_runtime` — opens the
  persistent chaindb and writes synthetic blocks in a loop. The
  victim of `kill -9`.
- An integration test `stress_kill_harness` that orchestrates: spawn
  → sleep → SIGKILL → wait → reopen → verify invariants → repeat.
- A CI-safe smoke variant (10 iterations).
- A closure-grade `#[ignore]`'d gate variant (1,000 iterations).
- CE-N-D-1 closure evidence: smoke green in CI, gate green when
  manually invoked.

**Out:**
- Stress harness for `SnapshotStore` operations (snapshots are write-
  heavy but the same redb transaction discipline applies; if smoke
  reveals issues, add separate snapshot stress later).
- Multi-process concurrent-writer tests (single-writer model from
  S-33 holds; concurrent-writer is out of scope for cluster N-D
  entirely).
- Performance benchmarks under fault injection (CE-N-D-2/3 are
  separate evidence streams).
- ASAN / fuzz harnesses (helpful but not gating CE-N-D-1).

---

## O-37.1 — Why subprocess, not in-process?

**Obligation:** S-34 already shipped a `KillStrategy` trait. Why does
S-37 introduce a separate harness instead of implementing
`KillStrategy` for SIGKILL?

### Answer

**You cannot in-process `kill -9` your own process and expect
test-runner behavior.** The Rust process running the harness IS the
test runner; SIGKILL'ing it terminates the test, not just the chaindb
operation. The S-34 `KillStrategy` interface is preserved as a
deterministic in-process simulation (`abort_call` style — useful for
unit tests of error paths) but it does not exercise the OS-level
crash window the redb manifest commits to surviving.

Subprocess is the only way to get a real OS-level SIGKILL between
fsync and reopen.

### Implementation pattern

- Worker binary (`chaindb_kill_target`) opens the chaindb and writes
  blocks in a loop. It exits cleanly on no signal but is expected
  to be SIGKILL'd from outside.
- Harness (an integration test) spawns the worker, sleeps for a
  pseudo-random duration to vary the crash window, sends SIGKILL via
  `Child::kill` (which is SIGKILL on Unix), waits for exit, reopens
  the chaindb, and verifies invariants.
- One iteration = one full spawn-kill-reopen cycle.

---

## O-37.2 — Sleep duration policy

**Obligation:** What's the kill timing? Random sleep would violate
the "no true randomness" Core Contract rule.

### Answer

**Deterministic LCG-style sequence parametrized by iteration index.**

```rust
fn delay_ms(iter: u64) -> u64 {
    // Small mix of values that exercise different crash windows:
    //  - 0ms → kill before worker has begun any I/O
    //  - 5ms → during db open
    //  - 25ms → after a few writes
    //  - 100ms → after many writes
    const TABLE: [u64; 8] = [0, 1, 5, 10, 25, 50, 100, 200];
    TABLE[(iter as usize) % TABLE.len()]
}
```

This is reproducible from the iteration index — same seed, same
sequence — and exercises the four interesting crash windows (no-op,
during-open, mid-write, late). No clock dependency, no RNG state.

### Iteration count

| Variant | Iterations | When run |
|---|---|---|
| Smoke (`stress_kill_smoke`) | 10 | Every `cargo test` |
| Gate (`stress_kill_1000`) | 1,000 | `#[ignore]`'d; manual run for CE-N-D-1 closure evidence |

10-iteration smoke at ~125ms average per iter is ~1-2s test runtime
— acceptable for CI. 1,000 iterations is ~2-5min — acceptable for
a manual gate run.

---

## O-37.3 — Invariants checked per iteration

**Obligation:** What does "zero corruption" mean operationally? What
does the harness assert after each kill cycle?

### Answer

After each spawn-kill-wait, reopen the chaindb and assert:

1. **Open succeeds.** Reopen returns `Ok(_)`, never `Err(Corruption)`.
   This is the headline corruption-free invariant.
2. **Schema is intact.** No `SchemaMismatch`, no Corruption from
   missing magic.
3. **Tip is consistent.** `tip()` returns `Ok(Some(_))` (after the
   first iteration that committed at least one block) or `Ok(None)`.
4. **Tip block is fully readable.** `get_block_by_slot(tip.slot)`
   and `get_block_by_hash(tip.hash)` both return matching data.
5. **No partial writes visible.** Every block returned by
   `iter_from_slot(0)` has its slot in the hash index too. (Full
   index consistency check; redb's atomic transaction guarantee
   should make this trivially hold, but we verify.)
6. **No phantom blocks.** Every entry in the hash index has a
   corresponding block in the slot index.

If any assertion fails, the test panics with iteration index +
specific invariant. The fault is preserved by leaving the chaindb
file in `target/` for inspection.

---

## O-37.4 — Worker block-write strategy

**Obligation:** What does the worker actually do? Random writes,
sequential, mixed?

### Answer

**Sequential block writes by slot, hash derived from slot.**

```rust
let hash = {
    let mut h = [0u8; 32];
    h[..8].copy_from_slice(&slot.to_le_bytes());
    Hash32(h)
};
```

Hash derived from slot makes idempotent re-puts deterministic — if
the worker restarts and tries to write slot 5 again, the hash is the
same, so `put_block` succeeds (idempotent path). Different starting
state per iteration is what matters for crash-window coverage, not
random data.

The worker's slot counter starts from `tip().slot.0 + 1` if a tip
exists, else from 1. This means each iteration writes net-new blocks
on top of the prior iteration's surviving state — over 1,000
iterations we accumulate a sizable corpus and exercise the recovery
of larger and larger files.

Block size: 64 bytes. Small enough to write fast, large enough to
exercise multi-block fsync.

---

## O-37.5 — How CE-N-D-1 closes

**Obligation:** What's the formal closure criterion?

### Answer

CE-N-D-1: "chain DB survives 1,000 random kill-9 events on a
synthetic workload with zero corruption (checksum-verified)."

Closure evidence:
- `cargo test -p ade_runtime --test stress_kill_harness stress_kill_1000 -- --ignored --nocapture`
  runs the gate variant (1,000 iterations) end-to-end without panic.
- The output is captured to a log file under `target/`.
- The `phase_4_status.md` (forthcoming) cites the log as evidence.

Initial closure run is the first time the gate is invoked manually.
Re-runs are not required unless the harness or persistent impl
changes. The smoke variant in CI catches regressions between manual
gate runs.

---

## O-37.6 — Acceptance gate for S-37

1. `cargo build --workspace` clean.
2. `cargo test -p ade_runtime` green:
   - All 14 existing tests still pass.
   - New `stress_kill_smoke` (10 iter) ✓
   - `stress_kill_1000` is `#[ignore]`'d, runs manually.
3. `cargo clippy -p ade_runtime --all-targets` clean.
4. Tier isolation holds:
   `rg "redb|rocksdb|sled|sqlite" crates/ade_runtime/src/` returns
   only `persistent.rs`.
5. Manual gate run: 1,000 iterations green. Log archived.

---

## Forbidden patterns for S-37

- **No in-process SIGKILL.** Subprocess only.
- **No clock-based timing.** Iteration-indexed deterministic delay table.
- **No "best-effort" pass." Any reopen failure or invariant
  violation fails the iteration loud and clear.
- **No suppressing test output during the gate run.** The 1,000-iter
  log is closure evidence and must be readable.
- **No platform conditionals** beyond what's necessary. Linux-first;
  if macOS works it's a bonus, but Linux is the closure target.

---

## Out of scope (explicitly)

- Snapshot stress harness (defer; same redb transaction discipline).
- Concurrent-writer scenarios (out of scope for cluster N-D).
- Power-loss simulation deeper than process kill (e.g.,
  filesystem-level fsync interception). redb commits to fsync;
  trusting the OS at that boundary is appropriate.
- Performance under fault injection (CE-N-D-2/3 are separate).
- Recovery-after-corruption tools (out of scope; on real corruption,
  operator restores from backup).

---

## Authority Reminder

This discharge is a planning artifact. Authority for CE-N-D-1
closure rests on the `phase_4_status.md` evidence pointer once the
gate run lands.
