# PHASE4-N-F-F — Slice S4: Operator-material-backed forge proof + replay-equivalence

> **Status:** slice doc (IDD Part IV). Companion to
> `../../planning/phase4-n-f-f-cluster-slice-plan.md` (S4 row). Code-verified
> against HEAD `217ad15` (S3 merged) at authoring.

> **Slice S4 in one line:** prove that an **operator-material-backed**
> `ForgeActivation` (real keys loaded through the S2 production ingress) driven by
> a *continuing* feed reaches ONLY the fenced `forge_one_from_recovered` path —
> exactly once for the due slot, self-accept-only, with the operator KES key
> actually signing — and is replay-equivalent across runs.

## 1. Slice identity
- **Cluster:** PHASE4-N-F-F (operator-key ingress → forge-on flip).
- **Slice:** S4 — Operator-material-backed forge proof + replay-equivalence.
- **Module (tests only):** `crates/ade_node/src/node_sync.rs` `#[cfg(test)]`
  (where the N-F-E `relay_loop_forge_*` fixtures live: `l5_recovered_state`,
  `s2_coordinator_state`, `l5_era_schedule`, `DeterministicClock`). **No
  production code changes** — S4 is a proof slice over the S1–S3 surface.

## 2. Cluster Exit Criteria addressed (verbatim)
- **CE-F-5** — With a *continuing* hermetic feed + operator-material-backed
  activation + injected clock/shutdown schedule, **each forge attempt reaches only
  the fenced `forge_one_from_recovered` path** (no alternate forge codepath),
  self-accept-only (advances no durable tip; serves/admits/broadcasts/gossips
  nothing), **and the test asserts the expected number of attempts for its fixed
  clock schedule**; the forge-attempt sequence and forged bytes are byte-identical
  across runs (replay-equivalent).
- closes **CE-F-6** — the N-F-E forge-containment gate is green on the full cluster
  diff (no production change in S4; gate untouched).

(CE-F-1 in S1; CE-F-2 in S2; CE-F-3/F-4 in S3.)

## 3. Intent (invariant impact)
Closes the proof half of `CN-NODE-03`: the operator-material-backed activation —
keys loaded through the production ingress (`operator_forge::load_operator_producer_shell`)
— reaches the **same** fenced, self-accept-only forge path the N-F-E hermetic tick
established, and does so **replay-equivalently**. The operator's own derived pool
(`blake2b_224(cold_vk)`) is registered in the recovered surface (asc 1/1), so
leadership resolves over the operator identity and the operator KES key signs —
proving the real cryptographic identity drives the fenced forge, not a synthetic
stand-in. Self-accept-only is re-asserted: no durable tip, no snapshot, no serve.

## 4. Pre-conditions
- S1–S3 merged (`217ad15`): `ForgePaths`, `load_operator_producer_shell`,
  `build_operator_forge_material`, the binary `Some/None` flip.
- The N-F-E forge fixtures exist in `node_sync.rs` tests (`l5_recovered_state`,
  `s2_coordinator_state` — `kes_period_for_slot(100) == 0`, `l5_era_schedule`,
  `fresh_state`, `s2_idle_view`).
- `forge_one_from_recovered` decides leadership over the recovered
  `PoolDistrView`; with asc 1/1 a registered pool is always eligible (so the
  KES-signing path runs deterministically, independent of the VRF lottery).

## 5. Implementation boundary (tests only)
- **`s4_operator_material(dir) -> ForgePaths`** — writes a complete real-format
  operator key set with a **real opcert sigma** (cold key signs
  `hot_vkey‖seq‖kes_period`, the `l5_synth_shell` recipe) so the loaded opcert
  verifies against the cold key.
- **`l5_recovered_inputs_for_pool(pool)`** — recovered seed-epoch inputs
  registering the given pool (asc 1/1).
- **`drive_operator_forge_once(opdir, chaindir) -> Vec<CoordinatorEvent>`** —
  loads the shell via `load_operator_producer_shell`, derives the operator pool,
  registers it, builds a `ForgeActivation` over a *continuing* `from_wire_pump`
  source + a one-tick `DeterministicClock` (→ slot 100, KES period 0), runs
  `run_relay_loop(Some(..))` to a clean shutdown halt, asserts tip-unchanged +
  no-snapshot internally, and returns the in-memory outcomes.
- **No production change.** No new type, no CI gate, no BLUE change.

## 6. TCB color
- **RED test harness** over: reused S2 RED ingress, GREEN `run_loop_planner`,
  GREEN `CoordinatorState::kes_period_for_slot`, BLUE `forge_one_from_recovered`
  (leadership projection + KES verify path) — all unchanged.

## 7. Invariants preserved (must not weaken) — by registry ID
- **DC-NODE-05 / CE-F-6** — the forge tick + the `run_relay_loop` body + the
  containment gate are unchanged; S4 only exercises them with operator material.
- **CN-NODE-02 / DC-SYNC-02** — no tip-advance path added; the forge advances no
  durable tip (re-asserted).
- **CN-CINPUT-03 / DC-CINPUT-02b** — leadership is projected from the recovered
  surface inside the fenced call; S4 registers the operator pool *in* that
  recovered surface, never via a bundle.
- **CN-PROD-02 / CE-F-2** — key custody stays in `ProducerShell`; the test reads
  only public surface (`cold_vk` for the pool id) and never prints/serializes key
  bytes.
- All BLUE invariants — no BLUE crate modified.

## 8. Invariants strengthened (one family: CN-NODE-03)
- **CN-NODE-03** (`declared`) — lands its **proof half**: the operator-material
  forge reaches only the fenced self-accept-only path and is replay-equivalent.
  Contributes **CE-F-5**; with CE-F-1..F-4 already green this is the last CE, so
  the cluster close flips `declared → enforced`.

## 9. Replay / determinism obligations
- `relay_loop_with_operator_material_two_runs_byte_identical` proves the
  forge-attempt sequence + forged bytes are byte-identical across two independent
  runs with the same fixed operator key set + recovered state + clock schedule.
  This is DC-NODE-05's replay clause exercised with operator material (a
  strengthening, not a new law). No corpus entry.

## 10. Mechanical acceptance criteria
- [ ] Test `relay_loop_with_operator_material_forge_reaches_fenced_path` — exactly
      one outcome (`hermetic_forge_outcomes.len() == 1`); the outcome is NOT
      `ForgeNotLeader` (operator pool registered + asc 1/1 ⇒ the operator KES
      signing path runs); tip unchanged; no snapshot persisted.
- [ ] Test `relay_loop_with_operator_material_two_runs_byte_identical` — two
      independent runs produce a byte-identical outcome sequence
      (`format!("{:?}")` equality over the forged outcomes).
- [ ] The shell is loaded ONLY via `operator_forge::load_operator_producer_shell`
      (the production ingress), not a synthetic stand-in; the registered pool is
      the operator's own `blake2b_224(cold_vk)`.
- [ ] `cargo test -p ade_node` green (count > 0); `rustfmt`; **the full N-F-F gate
      set passes unchanged** — `ci_check_node_run_loop_containment.sh`,
      `ci_check_loop_planner_closed.sh`, `ci_check_operator_forge_no_secret_leak.sh`,
      `ci_check_forge_intent_closed.sh`, `ci_check_private_key_custody.sh` (CE-F-6:
      no gate modified by S4).

## 11. Forbidden in this slice (inherits the cluster Forbidden list)
- **No production change** — S4 is a proof slice; no new type / fn / CI gate / BLUE
  change. (Any production gap surfaced ⇒ a scoped fix, not silent test-only
  patching.)
- **No relaxation of any gate** (CE-F-6) — the containment + planner + no-leak
  gates stay byte-identical.
- No key-byte print/serialize/compare in the fixture (read only public surface).
- No durable tip advance / serve / admit / gossip assertion of success — the proof
  is self-accept-only + replay-equivalent; NO live / BA-02 / RO-LIVE claim.

## 12. Slice completion checklist
- [ ] `node_sync.rs` test additions (fixtures + 2 tests); no production change.
- [ ] `cargo test -p ade_node` green; `rustfmt`; full gate set green.
- [ ] Slice doc committed standalone (`docs:`) before impl; impl (`test:`) after green.

## Authority
Registry IDs `CN-NODE-03` (proof half; `declared` → `enforced` at close),
`DC-NODE-05` / `CN-NODE-02` / `DC-SYNC-02` / `CN-CINPUT-03` / `DC-CINPUT-02b` /
`CN-PROD-02` (preserved). The cluster-slice-plan and the invariant registry are
authoritative; this slice doc refines.
