# PHASE4-N-F-E — Slice S3b: Single-epoch / KES fail-closed containment

> **Status:** slice doc (IDD Part IV). Companion to `cluster.md` (S3b row) and
> `../../planning/phase4-n-f-e-cluster-slice-plan.md`. Code-verified against
> HEAD `2484dd1` (S3a) at authoring.

> **Slice S3b in one line:** prove the forge tick **fails closed** on an
> unsupported slot — a KES period rotated past the hot key produces a *skip*
> (no forge attempt at all), and an off-epoch / outside-horizon slot is
> represented locally as a structured `ForgeNotLeader` through the existing
> fenced forge path — neither fabricating consensus inputs, signing
> retroactively, advancing the tip, nor serving/admitting anything.

## 1. Slice identity
- **Cluster:** PHASE4-N-F-E (forge-tick on the relay spine, hermetic).
- **Slice:** S3b — single-epoch / KES fail-closed containment.
- **Module:** `crates/ade_node::node_sync` **test module only** — no production
  code change. The two fail-closed behaviors already exist from S2 (the
  `if let Some(kes_period)` skip in `run_relay_loop`; `forge_one_from_recovered`'s
  off-epoch `Err → ForgeNotLeader`); S3b proves them.

## 2. Cluster Exit Criteria addressed (verbatim)
- **CE-E-7** — single-epoch / KES containment: candidate tests
  `forge_tick_off_epoch_slot_fails_closed_local`,
  `forge_tick_rotated_kes_period_skips_no_retroactive_sign` assert a structured
  local skip/fail-closed with no fabricated inputs, no retroactive sign, no tip
  advance, no serve/admit.

(CE-E-1/E-2 = S1; CE-E-3/E-4/E-5 = S2; **CE-E-6 (replay-equivalence) = S3a — explicitly out of S3b scope**.)

## 3. Intent (invariant impact)
Lands the **fail-closed clause of `DC-NODE-05`**: the forge tick is *total* over
slots it cannot legitimately serve. A slot whose KES period has rotated past the
hot key is **skipped before any forge attempt** (no `forge_one_from_recovered`
call, hence no KES signing — no retroactive sign); a slot the recovered
single-epoch view cannot support is **represented locally as `ForgeNotLeader`
through the existing fenced forge path** (the off-epoch / outside-horizon
outcome — an implementation-local structured outcome, not a Cardano semantic
claim). In neither case is a `SeedEpochConsensusInputs` / `pparams` /
`protocol_version` / `pool_id` / KES literal fabricated, the durable tip
advanced, or anything served/admitted. This completes single-epoch containment
as a **cluster-scope** rule (cross-epoch rollover is a separate cluster), not a
permanent Cardano semantic.

## 4. Pre-conditions
- S2 (`98b488a`): `run_relay_loop`'s `ForgeTick` arm skips when
  `kes_period_for_slot` is `None` (no forge, no `last_forged_slot` update);
  `forge_one_from_recovered` returns `ForgeNotLeader` on a leader-schedule
  miss (verified at `node_sync.rs:415`).
- S3a (`2484dd1`): forge replay-equivalence closed; `ade_node` lib green (131).
- `kes_period_for_slot` returns `None` for `slot < kes_anchor_slot` or
  `period > kes_max_period` (verified, `coordinator.rs:160`).

## 5. Implementation boundary
- **Test-only.** Add two hermetic tests to `node_sync`'s test module + a small
  KES-exhausted coordinator fixture (e.g. `kes_max_period = 0`,
  `slots_per_kes_period = 10` so the test slot's period exceeds the max → `None`).
  No production code, no gate, no registry edit, no new vocabulary (outcomes are
  observed via the existing `hermetic_forge_outcomes`).
- **KES-rotated test** (`forge_tick_rotated_kes_period_skips_no_retroactive_sign`):
  `ForgeActivation` over a KES-exhausted coordinator; a `DeterministicClock` tick
  → a `Due` slot whose `kes_period_for_slot` is `None`. Drive the loop (open
  WirePump + `join!` shutdown). Assert: `hermetic_forge_outcomes` is **empty**
  (no `forge_one_from_recovered` attempt → no KES signing → no retroactive sign);
  the tip/WAL/checkpoint surfaces are **unchanged from the pre-tick baseline**
  (the skipped tick changes nothing); `last_forged_slot` is not advanced (see §12
  for how this is observed).
- **Off-epoch test** (`forge_tick_off_epoch_slot_fails_closed_local`):
  `ForgeActivation` over `s2_coordinator_state()` (KES in range) + a
  `DeterministicClock` tick → a slot **outside** the recovered epoch's window
  (≥ `epoch_length_slots`, e.g. slot 432000 for the L5 epoch-0 schedule). Drive
  the loop. Assert: exactly one outcome and it is `CoordinatorEvent::ForgeNotLeader`
  (the structured off-epoch outcome represented locally through the fenced forge
  path — **not** `ForgeSucceeded`); the tip/checkpoint surfaces are unchanged from
  the pre-tick baseline (no advance); nothing served/admitted. (Structural: the
  path constructs no `SeedEpochConsensusInputs` / pparams / pool-id / KES literal
  — guaranteed by the fenced `forge_one_from_recovered` + CN-CINPUT-03 guard (d),
  already enforced; the test asserts the observable non-fabrication: no off-epoch
  `ForgeSucceeded`, no tip change.)

## 6. TCB color
- **Test only** — RED test orchestration over the deterministic forge handoff.
  No BLUE/GREEN/RED production module changes.

## 7. Invariants preserved (must not weaken)
- `DC-NODE-05` (S2 wiring + S3a replay) — `run_relay_loop` /
  `forge_one_from_recovered` unchanged; the forge advances no tip; the
  containment gate untouched.
- `CN-PROD-02` — no retroactive sign past the hot-key KES period (the
  KES-rotated skip is the loop-side proof).
- `DC-CINPUT-02b` / `CN-CINPUT-03` — leadership only from the recovered surface;
  off-epoch is represented as `ForgeNotLeader`, never a fabricated off-epoch forge.
- `T-REC-03` (relay evidence) + S3a replay test + S2 forge tests — all re-run
  green (§11).
- All BLUE invariants — untouched.

## 8. Invariants strengthened (one family: DC-NODE-05)
- `DC-NODE-05` (`declared`) — S3b discharges its **fail-closed clause** (CE-E-7).
  **No registry edit in S3b:** `DC-NODE-05` stays `declared` through S3b
  implementation. With **all** of CE-E-1..7 now green, the `declared → enforced`
  flip and the `strengthened_in += "PHASE4-N-F-E"` appends (CN-NODE-02,
  DC-SYNC-02, T-REC-03, DC-NODE-03, CN-PROD-02, DC-CINPUT-02b) happen **only
  during `/cluster-close`**, not here.

## 9. Replay / determinism obligations
- No replay claim in S3b (S3a closed CE-E-6). These tests are deterministic
  single-run fail-closed assertions: `DeterministicClock`, fixed shell seeds, no
  wall-clock/rand/float. No on-disk corpus.

## 11. Validation (tests by name)
- **New:** `forge_tick_rotated_kes_period_skips_no_retroactive_sign`,
  `forge_tick_off_epoch_slot_fails_closed_local` (node_sync test module).
- **Re-run green (regression / containment still holds):**
  `relay_loop_forge_two_runs_byte_identical` (S3a),
  `relay_loop_forge_tick_attempts_forge_advances_no_tip`,
  `relay_loop_forge_slot_derived_via_clock_seam`,
  `relay_loop_without_producer_material_matches_nfd_relay` (S2),
  `relay_loop_two_clean_runs_byte_identical` (N-F-D T-REC-03 relay evidence).

## 12. Mechanical acceptance criteria
- [ ] `cargo test -p ade_node --lib forge_tick_rotated_kes_period_skips_no_retroactive_sign`
      passes: a `Due` slot with `kes_period_for_slot == None` makes **no** forge
      attempt — `hermetic_forge_outcomes` empty; the tip/WAL/checkpoint surfaces
      are unchanged from the pre-tick baseline; and `last_forged_slot` is not
      advanced — proven **either** by direct inspection of `ForgeActivation`
      after the run **or** by a follow-up due-slot attempt showing the skipped
      slot was not consumed.
- [ ] `cargo test -p ade_node --lib forge_tick_off_epoch_slot_fails_closed_local`
      passes: an off-epoch slot yields exactly one `ForgeNotLeader` (no
      `ForgeSucceeded`); tip/checkpoint surfaces unchanged from baseline; nothing
      served/admitted; no fabricated record.
- [ ] `cargo test -p ade_node --lib relay_loop_forge_two_runs_byte_identical` and
      `cargo test -p ade_node --lib relay_loop_two_clean_runs_byte_identical`
      stay green.
- [ ] `cargo test -p ade_node --lib` green (full target, 0 failed, count grown by 2).
- [ ] `cargo build -p ade_node` clean; `rustfmt --edition 2021` on `node_sync.rs`.
- [ ] Gates unchanged + green: `ci_check_node_run_loop_containment.sh`,
      `ci_check_node_sync_via_pump.sh`, `ci_check_loop_planner_closed.sh`.

## 13. Forbidden in this slice (inherits the cluster Forbidden list)
- No production code change; no gate change; **no registry flip** (DC-NODE-05
  stays `declared` through S3b; flips only at `/cluster-close`).
- No replay-equivalence claim (S3a closed CE-E-6).
- No production operator-key file/config ingestion.
- No new event vocabulary — outcomes observed only via the existing local
  `hermetic_forge_outcomes`.
- No BLUE change; no `run_node_sync` change; no durable apply / tip mutation /
  serve / admit. No live peer, BA-02, RO-LIVE claim.

## 14. Slice completion checklist
- [ ] Two fail-closed tests + KES-exhausted coordinator fixture added to the
      `node_sync` test module.
- [ ] `cargo test -p ade_node --lib` green; S3a replay + S2 forge + T-REC-03
      relay evidence re-run green; `rustfmt` applied; the three gates pass.
- [ ] Slice doc committed standalone (`docs:`) before implementation; impl
      committed (`test:`) after green.
- [ ] **After S3b merges, all CE-E-1..7 are green → `/cluster-close` performs
      the DC-NODE-05 `declared → enforced` flip + the strengthenings + the
      grounding-doc refresh.** (Not in S3b.)

## Authority
Registry IDs `DC-NODE-05` (fail-closed clause; stays `declared` through S3b),
`CN-PROD-02` / `DC-CINPUT-02b` / `CN-CINPUT-03` (preserved). The cluster doc
`cluster.md` and `docs/ade-invariant-registry.toml` are authoritative; this slice
doc refines, it does not override.
