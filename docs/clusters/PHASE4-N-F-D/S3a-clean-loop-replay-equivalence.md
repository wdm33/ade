# PHASE4-N-F-D — Slice S3a: Clean loop replay-equivalence

> **Status:** slice doc (IDD Part IV). Companion to `cluster.md` (S3a row).
> Builds on S2 (`run_relay_loop`, committed `3756803`). **Test-only slice** —
> no production behavior change; it proves a determinism property of the
> already-shipped loop and flips `T-REC-03` → enforced.

> **Slice S3a in one line:** prove that two clean `run_relay_loop` runs over
> identical inputs produce byte-identical authoritative outputs (tip, WAL,
> checkpoints) — deterministic orchestration absent crash interference.

## 1. Slice identity
- **Cluster:** PHASE4-N-F-D. **Slice:** S3a.
- **Touches:** `ade_node::node_sync` test module only (one new hermetic test).
  No production source change.
- **Cluster Exit Criteria addressed:** CE-D-4.

## 2. Invariant scope
- **T-REC-03 (true) → enforced:** same recovered/bootstrapped state + same
  ordered in-memory block feed + same deterministic shutdown schedule ⇒
  byte-identical tips, WAL, and checkpoints across two clean runs. Extends
  T-REC-01/T-REC-02 from single-shot recovery to continuous relay operation;
  rides the existing recovery laws (snapshot + forward-replay, NOT
  full-genesis) — no new durability law.

## 3. Why test-only
The loop is RED orchestration over the deterministic GREEN planner (S1, pure)
and the deterministic `run_node_sync` → `pump_block` seam (BLUE authority).
Determinism is therefore already structurally present; S3a *proves* it
mechanically rather than adding behavior. The proof is a test, not new code
(consistent with the cluster's "no new corpus / no new authoritative state"
replay obligation).

## 4. Implementation boundary
- New test `relay_loop_two_clean_runs_byte_identical` in the
  `ade_node::node_sync` test module (reuses `corpus_view` / `schedule` /
  `pick_lightest` / `fresh_state`):
  - A helper drives one clean `run_relay_loop` over a fresh `TempDir` store
    set + `fresh_state` + `NodeBlockSource::in_memory` over an ordered
    multi-block feed + `watch(false)`, then returns (tip slot+hash, the full
    `wal.read_all()` image, the captured snapshot slot list).
  - Run it twice over the SAME feed (two independent fresh store sets).
  - Assert: tip slot+hash equal; WAL images equal; snapshot slot lists equal.
- A multi-block feed (≥2 blocks) so the loop iterates more than once — the
  property must hold across iterations, not just a single apply.

## 5. Proof obligations (exit criteria — CE-D-4)
- [ ] `relay_loop_two_clean_runs_byte_identical` passes.
- [ ] `cargo test -p ade_node --lib` green; touched file `rustfmt`-clean.
- [ ] Registry `T-REC-03` status `declared` → `enforced` (`tests` populated;
      `ci_script` references the containment gate that backs the loop).
      `ci_check_registry_code_locus_exists.sh` stays green.

## 6. TCB color
- **Test-only** (harness in the RED `ade_node` test module). No BLUE /
  production change.

## 7. Forbidden (inherits the cluster Forbidden list)
- No production source change (S3a is a determinism proof, not behavior).
- No new authoritative state / canonical type / WAL or checkpoint format.
- No `cargo fmt -p ade_node` (format only the touched file).

## 8. Replay / determinism
- This slice IS the replay-equivalence proof for the loop (T-REC-03). No
  corpus entry; discharged by the two-runs-byte-identical test.

## Authority
Registry ID `T-REC-03` (→ enforced at this slice). `cluster.md` + the
registry are authoritative.
