# PHASE4-N-F-E — Slice S3a: Forge-tick replay-equivalence

> **Status:** slice doc (IDD Part IV). Companion to `cluster.md` (S3a row) and
> `../../planning/phase4-n-f-e-cluster-slice-plan.md`. Code-verified against
> HEAD `98b488a` (S2) at authoring.

> **Slice S3a in one line:** prove that the self-accept-only forge tick is
> **replay-deterministic** — same recovered state + same ordered feed + same
> injected clock tick schedule + same shutdown schedule ⇒ byte-identical tips,
> WAL, checkpoints, the forge-attempt sequence, and forged block bytes for any
> `ForgeSucceeded` outcomes, across two clean runs.

## 1. Slice identity
- **Cluster:** PHASE4-N-F-E (forge-tick on the relay spine, hermetic).
- **Slice:** S3a — forge-tick replay-equivalence.
- **Module:** `crates/ade_node::node_sync` **test module only** — no production
  code change. Reuses the S2 wiring (`run_relay_loop` + `ForgeActivation` +
  `hermetic_forge_outcomes`) and the `l5_*` / `s2_*` forge fixtures.

## 2. Cluster Exit Criteria addressed (verbatim)
- **CE-E-6** — forge-tick replay-equivalence: candidate test
  `relay_loop_forge_two_runs_byte_identical` asserts byte-identical tips + WAL +
  checkpoints + forge-attempt sequence + forged bytes across two clean runs over
  identical inputs.

(CE-E-1/E-2 = S1; CE-E-3/E-4/E-5 = S2; **CE-E-7 (single-epoch/KES fail-closed) is S3b — explicitly out of S3a scope**.)

## 3. Intent (invariant impact)
Lands the **replay clause of `DC-NODE-05`**: the forge tick's observable output
(the ordered sequence of `CoordinatorEvent` forge attempts and, for any
`ForgeSucceeded`, the forged block bytes) is a pure, deterministic function of
the canonical inputs (recovered state, feed, injected clock tick schedule,
shutdown schedule). This extends the N-F-D relay-loop replay anchor (`T-REC-03`,
`relay_loop_two_clean_runs_byte_identical`) from sync-only to the forge branch —
the wall-clock observation is canonicalized to `SlotNo` before crossing, so two
clean runs over identical inputs are byte-identical. No new authoritative
durable state and no new corpus: the WAL / tip / checkpoint surfaces are
relay-derived and unchanged by forge; S3a asserts they are byte-identical across
both runs, and the load-bearing identity is the forge-attempt sequence + forged
bytes.

## 4. Pre-conditions
- S2 (`98b488a`): `run_relay_loop` forge branch + `ForgeActivation` (with
  `hermetic_forge_outcomes: Vec<CoordinatorEvent>`) wired; the evolved
  containment gate green; `ade_node` lib-test green (130).
- `CoordinatorEvent` derives `PartialEq`/`Eq` (incl. `ForgedBlockArtifact { slot,
  hash, bytes }`), so `assert_eq!` byte-compares forged bytes.
- `forge_one_from_recovered` is deterministic for fixed inputs (DC-CINPUT-02b,
  `forge_from_recovered_is_deterministic_across_two_runs`); `DeterministicClock`
  + same `l5_synth_shell` seeds ⇒ identical forge inputs.

## 5. Implementation boundary
- **Test-only.** Add one hermetic two-run test to `node_sync`'s test module; no
  production code, no gate, no registry edit.
- **Inner `run_once`** (async): fresh `PersistentChainDb` + `FileWalStore`; an
  **open WirePump** source (Continuing); a `DeterministicClock` over a **fixed
  multi-tick schedule** (e.g. `[100_000, 200_000, 300_000]` → slots 100/200/300,
  each `Due` by monotonic increase ⇒ a 3-attempt forge sequence); a
  `ForgeActivation` over `l5_recovered_state(Some(l5_recovered_inputs()))` +
  `s2_coordinator_state()` + `l5_synth_shell(<fixed seeds>)`; run via
  `tokio::join!(run_relay_loop(..., Some(&mut act)), async { sd_tx.send(true) })`
  (forge ticks run synchronously, park at `Idle`, shutdown halts). Return the
  owned `(tip, wal_image: String, checkpoint_slots, hermetic_forge_outcomes)`.
- **Two runs over identical inputs** → assert byte-identity of **all four**:
  tip (slot+hash), WAL `Debug` image, checkpoint slots, and the
  `Vec<CoordinatorEvent>` forge-attempt sequence — which carries the forged
  block bytes for any `ForgeSucceeded`. Assert the forge-attempt sequence is
  non-empty and that its entries are outcomes returned by the fenced
  `forge_one_from_recovered` path (so the identity is not vacuous and cannot
  pass on a synthetic/stub-populated vector).

## 6. TCB color
- **Test only** — RED test orchestration over the pure GREEN planner + the
  deterministic forge handoff. No BLUE/GREEN/RED production module changes.

## 7. Invariants preserved (must not weaken)
- `DC-NODE-05` (S2 wiring), `CN-NODE-02`, `DC-SYNC-02` — `run_relay_loop` and
  `run_node_sync` are unchanged; the forge advances no tip; the containment gate
  is untouched.
- `T-REC-03` — the N-F-D relay replay evidence
  (`relay_loop_two_clean_runs_byte_identical`) stays green (re-run in §11).
- `DC-CINPUT-02b` / `CN-CINPUT-03` — leadership still projected only from the
  recovered surface inside the fenced forge.
- All BLUE invariants — untouched.

## 8. Invariants strengthened (one family: DC-NODE-05)
- `DC-NODE-05` (`declared`) — S3a discharges its **replay clause** (CE-E-6) via
  `relay_loop_forge_two_runs_byte_identical`, and correspondingly strengthens
  `T-REC-03` (loop-as-replay now covers the forge-attempt sequence + forged
  bytes). **No registry edit in S3a:** `DC-NODE-05` stays `declared` and the
  `T-REC-03 strengthened_in += "PHASE4-N-F-E"` append is deferred to cluster
  close, when **all** of CE-E-1..7 are green (S3b still owes CE-E-7). No status
  flip yet.

## 9. Replay / determinism obligations
- The whole point of S3a. Two clean runs over identical canonical inputs ⇒
  byte-identical tips/WAL/checkpoints + forge-attempt sequence + forged block
  bytes (for any `ForgeSucceeded`).
- Determinism guard: `DeterministicClock` (no wall-clock/`Instant`), fixed shell
  seeds, no rand/float, `BTreeMap`-ordered recovered surface; the wall-clock
  observation is canonicalized to `SlotNo` before crossing. **In-memory two-run,
  no on-disk corpus** (mirrors N-F-D S3a — "no new durability law").

## 10. (n/a — folded into §11/§12)

## 11. Replay / crash / epoch validation (tests by name)
- **New:** `relay_loop_forge_two_runs_byte_identical` (node_sync test module) —
  the two-run forge replay-equivalence test above.
- **Re-run green (regression):** `relay_loop_two_clean_runs_byte_identical`
  (T-REC-03 relay evidence) and the S2 forge tests
  (`relay_loop_forge_tick_attempts_forge_advances_no_tip`,
  `relay_loop_forge_slot_derived_via_clock_seam`,
  `relay_loop_without_producer_material_matches_nfd_relay`).
- **No crash/epoch validation in S3a** — crash recovery is N-F-D's domain;
  off-epoch/KES fail-closed is S3b.

## 12. Mechanical acceptance criteria
- [ ] `cargo test -p ade_node --lib relay_loop_forge_two_runs_byte_identical`
      passes: two runs over identical (recovered state, feed, injected clock
      ticks, shutdown schedule) are byte-identical in tip, WAL image, checkpoint
      slots, the `Vec<CoordinatorEvent>` forge-attempt sequence, and the forged
      block bytes for any `ForgeSucceeded` outcomes; the forge-attempt sequence
      is non-empty and contains outcomes returned by the fenced
      `forge_one_from_recovered` path.
- [ ] `cargo test -p ade_node --lib relay_loop_two_clean_runs_byte_identical`
      stays green (T-REC-03 relay evidence intact with the forge branch present).
- [ ] `cargo test -p ade_node --lib` green (full target, 0 failed, count grown by 1).
- [ ] `cargo build -p ade_node` clean; `rustfmt --edition 2021` on
      `node_sync.rs`.
- [ ] Gates unchanged + green: `ci_check_node_run_loop_containment.sh`,
      `ci_check_node_sync_via_pump.sh`, `ci_check_loop_planner_closed.sh` (S3a
      adds no production/gate change).

## 13. Forbidden in this slice (inherits the cluster Forbidden list)
- No production code change; no gate change; no registry edit (DC-NODE-05 stays
  `declared`).
- No fail-closed / KES-out-of-range / off-epoch / single-epoch-containment proof
  — that is **S3b** (CE-E-7).
- No BLUE change; no `run_node_sync` change; no durable apply / tip mutation /
  serve / admit of a forged block.
- No live peer, BA-02, RO-LIVE claim. No on-disk replay corpus entry.

## 14. Slice completion checklist
- [ ] `relay_loop_forge_two_runs_byte_identical` added to `node_sync` test
      module (with the `run_once` two-run helper).
- [ ] `cargo test -p ade_node --lib` green; T-REC-03 relay evidence + S2 forge
      tests re-run green; `rustfmt` applied; the three gates pass.
- [ ] Slice doc committed standalone (`docs:`) before implementation; impl
      committed (`feat:` / `test:`) after green.

## Authority
Registry IDs `DC-NODE-05` (replay clause; stays `declared`), `T-REC-03`
(strengthened in spirit, registry append deferred to close), `CN-NODE-02` /
`DC-SYNC-02` / `DC-CINPUT-02b` (preserved). The cluster doc `cluster.md` and
`docs/ade-invariant-registry.toml` are authoritative; this slice doc refines, it
does not override.
