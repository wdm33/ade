# PHASE4-N-F-D — Slice S3b: Crash-at-boundary recovery equivalence

> **Status:** slice doc (IDD Part IV). Companion to `cluster.md` (S3b row).
> Builds on S2 (`run_relay_loop`, `3756803`) and S3a (`6e0d75e`). **Test-only
> slice** — no production behavior change; it proves a recovery property of
> the already-shipped loop and strengthens the existing recovery laws.

> **Slice S3b in one line:** prove that a kill at a loop-iteration boundary,
> followed by the production warm-start recovery, lands at the same tip as an
> uninterrupted run — a different proof surface from clean replay (S3a).

## 1. Slice identity
- **Cluster:** PHASE4-N-F-D. **Slice:** S3b.
- **Touches:** `ade_node::node_sync` test module only (one new hermetic
  crash/restart test). No production source change.
- **Cluster Exit Criteria addressed:** CE-D-5.

## 2. Invariant scope
- **T-REC-03 (strengthened):** the loop-as-replay property extends to a
  crash at an iteration boundary.
- **T-REC-01, T-REC-02, DC-SYNC-01 (`strengthened_in += "PHASE4-N-F-D"`):**
  the relay loop's advanced tip is durable-before-advance (DC-SYNC-01) and
  recovery-derivable (T-REC-01/T-REC-02) — now exercised end-to-end through
  the production `warm_start_recovery` path after a `run_relay_loop` drive.

## 3. Why test-only
The loop advances the tip only through `run_node_sync → pump_block`, whose
durable-before-tip ordering (DC-SYNC-01) and the L3 `warm_start_recovery`
path are already shipped and proven for the `run_node_sync` driver (the L4c
test `node_sync_kill_then_warm_start_recovers_same_tip`). S3b proves the SAME
guarantee holds when the driver is `run_relay_loop` — a test over existing
authority, not new behavior.

## 4. Implementation boundary
- New test `relay_loop_kill_at_boundary_recovers_same_tip` in the
  `ade_node::node_sync` test module, modeled on the existing L4c test but
  driving the tip via `run_relay_loop` instead of `run_node_sync`:
  1. Seed the sidecar + WAL provenance under one anchor (`Hash32([0xA0;32])`,
     matching `fresh_state`'s `prior_fp`) so the L3 warm-start lineage is
     recoverable — identical to L4c.
  2. Drive `run_relay_loop` over an in-memory block feed + `watch(false)` on
     the persistent stores to a clean halt; capture the advanced tip via
     `ChainDb::tip`.
  3. **Kill:** drop the store handles; reopen at the same paths.
  4. **Recover:** `warm_start_recovery(&chaindb, &wal)` (the production L3
     path).
  5. Assert: the recovered tip slot equals the relay-loop-advanced tip slot.

## 5. Proof obligations (exit criteria — CE-D-5)
- [ ] `relay_loop_kill_at_boundary_recovers_same_tip` passes: a
      relay-loop-advanced tip, after a kill, is recovered through the
      production warm-start path to the same tip.
- [ ] `cargo test -p ade_node --lib` green; touched file `rustfmt`-clean.
- [ ] Registry `strengthened_in += "PHASE4-N-F-D"` appended (append-only, no
      list replaced) to `T-REC-01`, `T-REC-02`, `DC-SYNC-01`.
      `ci_check_registry_code_locus_exists.sh` stays green.

## 6. TCB color
- **Test-only** (harness in the RED `ade_node` test module). No production /
  BLUE change.

## 7. Forbidden (inherits the cluster Forbidden list)
- No production source change. No new authoritative state / format.
- Registry edits are append-only on `strengthened_in` (no list replaced).
- No `cargo fmt -p ade_node` (format only the touched file).

## 8. Replay / determinism
- This slice is the crash-at-boundary half of the loop replay-equivalence
  proof; it rides DC-SYNC-01 / T-REC-01 / T-REC-02 (snapshot + forward-replay,
  not full-genesis) — no new durability law, no new corpus.

## Authority
Registry IDs `T-REC-03` (strengthened) + the `strengthened_in` appends on
`T-REC-01` / `T-REC-02` / `DC-SYNC-01`. `cluster.md` + the registry are
authoritative.
