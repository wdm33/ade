# PHASE4-N-F-G-A — Slice S4: Epoch-boundary forge fail-closed (DC-EPOCH-03)

> **Status:** slice doc (IDD Part IV). Companion to `cluster.md` (S4 row + CE-G-A-4). Code-verified
> against HEAD `2049ec9d`. **The last G-A proof** — when CE-G-A-1,2a,2,3,4 are all green, G-A close
> flips `DC-EPOCH-03` declared→enforced.
>
> **Slice S4 in one line:** on the `--mode node` forge path, a candidate forge slot whose epoch ≠
> the recovered seed epoch **fails closed before any leadership decision or KES signing** — it
> cannot be forged, served, or signed — because the recovered `chain_dep` eta0 is the *seed-epoch*
> nonce and is **stale past the boundary** (a peer-reject class); the forge path never promotes the
> nonce (`NonceInput::{CandidateFreeze, EpochBoundary}` stays undriven here).

## 1. Slice identity
- **Cluster:** PHASE4-N-F-G-A (forge fidelity). Gated behind S1/S2a/S2/S3 (green). S4 owns the
  *epoch-boundary* axis only; S3 owned the clock→slot alignment axis (a different wall).
- **Slice:** S4 — epoch-boundary forge fail-closed (the single-recovered-seed-epoch guard).
- **Modules:** **GREEN** `ade_node::node_sync` (a new pure `forge_epoch_admission` guard + closed
  `ForgeEpochAdmission` sum, alongside `forge_one_from_recovered`); **RED** `ade_node::node_sync`
  `forge_one_from_recovered` (wires the guard *before* `query_leader_schedule`). **Consumes BLUE**
  `EraSchedule::locate`, `PoolDistrView` off-epoch `None`, `recovered.chain_dep.epoch_nonce`. **No
  BLUE change, no new `CoordinatorEvent` variant.**

## 2. Cluster Exit Criteria addressed (verbatim)
- **CE-G-A-4** — epoch fail-closed: candidate tests `node_forge_off_epoch_slot_fails_closed`,
  `node_forge_no_epoch_boundary_promotion_on_forge_path`; candidate gate
  `ci_check_node_forge_single_epoch_fail_closed.sh`. (`DC-EPOCH-03` flips declared→enforced when
  CE-G-A-1..4 are all green.)

(CE-G-A-1 = S1, CE-G-A-2a = S2a, CE-G-A-2 = S2, CE-G-A-3 = S3 — all done/green.)

## 3. Intent (invariant impact)
Make it **impossible for the node forge to produce a signed or served block for a slot outside the
single recovered seed epoch.** Today off-epoch fail-closed is *emergent*: a candidate slot in a
different epoch makes the single-epoch `PoolDistrView` return `None`, so `query_leader_schedule`
returns `Err` and `forge_one_from_recovered` yields `ForgeNotLeader` (node_sync.rs:405-421) — safe,
but **conflated** with a legitimate VRF-lottery loss and decided only *after* entering the
leadership path. S4 makes the boundary **explicit and named**: an `era_schedule.locate(slot)?.epoch
≠ recovered seed epoch` check runs **first**, fails closed with the eta0-staleness rationale, and
never reaches leadership or `run_real_forge` (KES signing). The recovered `chain_dep.epoch_nonce`
(the seed-epoch eta0) is consumed verbatim and **never promoted** — cross-epoch nonce roll is a
separate cluster, not G-A. This is the epoch-boundary wall; S3's was the clock-alignment wall.

## 4. Pre-conditions (verified at HEAD `2049ec9d`)
- **The emergent off-epoch path (no explicit guard today):** `forge_one_from_recovered`
  (node_sync.rs:378-453) projects the single-epoch `PoolDistrView` from the recovered record
  (`:397`), then `query_leader_schedule` (`:405`); off-epoch → the view's per-epoch lookups return
  `None` → `Err` → `ForgeNotLeader { slot, [0;8] }` (`:416-420`). The epoch is decided *inside*
  leadership, not before it.
- **The BLUE single-epoch foundation:** `PoolDistrView` answers only for the epoch it was built for;
  every `LedgerView` method returns `None` when `epoch != self.epoch` (consensus_view.rs:95-115).
- **The canonical slot→epoch mapping:** `EraSchedule::locate(slot) -> Result<EraLocation, HFCError>`
  with `.epoch` (era_schedule.rs:110-148) — the **exact** mapping `query_leader_schedule` uses
  (leader_schedule.rs:89-92). The recovered seed epoch is `SeedEpochConsensusInputs.epoch_no`
  (seed_consensus_inputs.rs:53); in production the forge `era_schedule.start_epoch` is built from it
  (`make_node_schedule(.., EpochNo(epoch))`, node_lifecycle.rs:420-425).
- **eta0 is the seed nonce, never promoted:** the forge consumes `recovered.chain_dep.epoch_nonce`
  as `eta0` (node_sync.rs:435); the node forge path contains **no** `NonceInput::{CandidateFreeze,
  EpochBoundary}` / nonce-roll token (verified absent in `node_sync.rs` + `node_lifecycle.rs`).
- **The existing N-F-E proof S4 hardens:** `forge_tick_off_epoch_slot_fails_closed_local`
  (node_sync.rs:1926-1997, CE-E-7) drives slot 432000 (epoch 1) against a recovered epoch-0 view →
  asserts `ForgeNotLeader` + no tip advance.
- **The forge tick is self-accept-only:** the outcome is pushed to `hermetic_forge_outcomes` and
  `last_forged_slot` advances; **no durable tip / WAL / snapshot moves** (node_lifecycle.rs:688-718,
  DC-NODE-05) — and the relay-loop containment gate forbids serve tokens in the loop body.

## 5. The fix (an explicit GREEN epoch-admission guard, fail-closed before leadership)
1. **GREEN guard** (`node_sync.rs`): add a pure
   `forge_epoch_admission(slot, era_schedule, seed_epoch) -> ForgeEpochAdmission`, where
   `ForgeEpochAdmission` is the closed sum `WithinSeedEpoch | OffEpoch { candidate_epoch:
   Option<EpochNo>, seed_epoch: EpochNo }`. It derives the candidate epoch via the **BLUE**
   `era_schedule.locate(SlotNo(slot))` (reuse — no divergent math): `Ok(loc)` with `loc.epoch ==
   seed_epoch` → `WithinSeedEpoch`; `Ok(loc)` with a different epoch → `OffEpoch { Some(loc.epoch),
   seed_epoch }`; `Err(_)` (unlocatable: before-system-start / after-last-era) → `OffEpoch { None,
   seed_epoch }` (also fail-closed — an unlocatable slot can never be the seed epoch). Pure, no
   I/O / clock / rand / float.
2. **RED integration** (`forge_one_from_recovered`): call `forge_epoch_admission(slot, era_schedule,
   recovered_inputs.epoch_no)` **immediately after** resolving `recovered_inputs`, **before**
   `query_leader_schedule`. On `OffEpoch{..}` → return the existing structured
   `CoordinatorEvent::ForgeNotLeader { slot, vrf_output_fingerprint: [0u8;8] }` (fail closed; **no
   new variant** — the closed GREEN `CoordinatorEvent` surface and `produce_mode` are untouched).
   The off-epoch decision is now explicit and precedes leadership + `run_real_forge` (KES signing),
   so an off-epoch slot **can never be signed, forged, or served**, regardless of what leadership
   would answer. On `WithinSeedEpoch` → the current path is unchanged.
3. **No nonce promotion (locked):** the forge keeps consuming `recovered.chain_dep.epoch_nonce`
   verbatim and drives no `NonceInput` transition; S4 adds none. The gate asserts the forge path
   stays free of `EpochBoundary`/`CandidateFreeze` promotion tokens.

## 6. TCB color (execution boundary)
- **GREEN (added):** `forge_epoch_admission` + the `ForgeEpochAdmission` sum — pure, deterministic,
  no I/O / clock / rand / float. (Local to `ade_node`; not a canonical/BLUE/wire type.)
- **RED (integrated):** `forge_one_from_recovered` — wires the guard before leadership; no new I/O,
  no clock, no key access beyond the existing custody boundary.
- **BLUE (consume only):** `EraSchedule::locate`, `PoolDistrView` off-epoch `None`,
  `recovered.chain_dep.epoch_nonce`, `consensus::nonce` (read of the rationale, no drive). A BLUE
  change is a red flag → reject.

## 7. Invariants preserved (must not weaken) — by registry ID
- `DC-CINPUT-02b` / `CN-CINPUT-03` — leadership view projected ONLY from the recovered surface; the
  guard adds a *pre-check*, it does not change the leadership source or read any bundle.
- `DC-NODE-05` — forge-slot discipline (at most once per slot, never past, subordinate to the sync
  spine, no durable-tip advance): the off-epoch fail-closed advances no tip and forges nothing.
- `CN-NODE-02` / `DC-SYNC-02` — the relay run-loop + `run_node_sync` durable-tip authority unchanged.
- `CN-NODE-01` — no second bootstrap / recovery path.
- `T-DET-01` — no determinism tripwire (the guard is pure; no clock/rand/float).
- The closed GREEN `CoordinatorEvent` surface (no new variant) — `produce_mode` + `apply_event`
  untouched; `ci_check_loop_planner_closed.sh`, `ci_check_node_run_loop_containment.sh`,
  `ci_check_no_independent_forge_codepath.sh`, `ci_check_consensus_input_provenance.sh` (guard d)
  stay green/unchanged.
- `DC-COMPAT-01` — no Ade-internal-fingerprint-vs-Haskell-state-hash equality.

## 8. Invariants strengthened (one family: single-recovered-seed-epoch forge fail-closed)
**Family:** *the recovered-surface node forge is bounded to the single recovered seed epoch — a
candidate slot whose epoch ≠ the recovered seed epoch fails closed explicitly, before leadership or
KES signing, and the recovered seed-epoch eta0 is never promoted on the forge path.*
- `DC-EPOCH-03` — S4 provides this rule's **core mechanical enforcement** (the explicit
  epoch-admission guard + the no-promotion lock + CE-G-A-4's tests/gate). The registry
  status **flip `declared` → `enforced`** is performed at **G-A close** (when CE-G-A-1..4 are all
  green), per the S1/S2a/S2/S3 per-slice pattern — **not in this slice's commit.**
- `DC-NODE-05` — epoch-hardened (the recovered-surface forge now fails closed at the epoch boundary
  too). `strengthened_in += "PHASE4-N-F-G-A"` **appended at G-A close.**
- **No registry edit in this slice.** No status flip in S4's own commit.

## 9. Slice-entry decisions (settled)
- **D-1 — off-epoch outcome surface (DECIDED: reuse `ForgeNotLeader`; add an explicit GREEN guard,
  no new `CoordinatorEvent` variant).** `CoordinatorEvent` is a closed **GREEN** enum shared with
  `produce_mode` (coordinator.rs:192); a new variant would ripple to `apply_event` + every match
  site — not narrow, and the cluster's intent is to *surface the BLUE `None` foundation*, not add a
  type. The forge-tick observable outcome for off-epoch stays the existing structured
  `ForgeNotLeader`; the **honest off-epoch distinction lives in the GREEN guard's own closed
  `ForgeEpochAdmission::OffEpoch` type** (directly unit-testable), and the guard makes the boundary
  *explicit and pre-leadership*. This is exactly "hardens N-F-E's `ForgeNotLeader` into the named
  DC-EPOCH-03 boundary."
- **D-2 — candidate-epoch source (DECIDED: reuse `EraSchedule::locate`).** The guard derives the
  candidate epoch via the same BLUE `era_schedule.locate(slot).epoch` that `query_leader_schedule`
  uses — no fabricated epoch math, no divergence between the guard and leadership. The seed epoch is
  `recovered_inputs.epoch_no` (the recovered surface), not a literal.
- **D-3 — unlocatable slots (DECIDED: fail closed).** `locate` `Err` (before-system-start /
  after-last-era) → `OffEpoch { candidate_epoch: None }` → fail closed. An unlocatable slot can
  never be the seed epoch; it must not forge. (Before-anchor at the clock seam is already S3's
  fail-closed; this is defence-in-depth at the epoch guard.)
- **D-4 — no nonce promotion is a lock, not new behavior.** The forge path already drives no
  `NonceInput` transition; S4 *proves and gates* that, and adds none. Cross-epoch nonce roll
  (`CandidateFreeze`/`EpochBoundary`) is a separate cluster.

## 10. Replay / determinism obligations
`forge_epoch_admission` is a pure function of `(slot, era_schedule, seed_epoch)` — same inputs →
same `ForgeEpochAdmission`. No wall-clock / rand / float. For a fixed injected clock tick schedule +
recovered state, the off-epoch-fail-closed-vs-forge sequence is byte-identical across runs (extends
DC-NODE-05's replay clause; the recovered seed eta0 is consumed unchanged). No new authoritative
state, no new canonical type, no WAL/checkpoint change, no new corpus entry.

## 11. Replay / crash / epoch validation (tests by name)
- **New (the guard, GREEN unit tests in `node_sync` tests):**
  - `forge_epoch_admission_within_seed_epoch_admits` — a slot in the recovered seed epoch →
    `WithinSeedEpoch`.
  - `forge_epoch_admission_off_epoch_fails_closed` — a slot one epoch past the seed epoch →
    `OffEpoch { Some(seed+1), seed }` (distinct from a lottery loss; reuses `era_schedule.locate`).
  - `forge_epoch_admission_unlocatable_fails_closed` — an unlocatable slot → `OffEpoch { None, .. }`.
- **New (the node forge path):**
  - `node_forge_off_epoch_slot_fails_closed` — a forge tick at an off-epoch slot (epoch 1, recovered
    epoch 0) yields the structured `ForgeNotLeader` **via the explicit guard, before leadership**;
    **no `ForgeSucceeded`** (not signed/forged), **no tip / WAL / snapshot change** (not served).
    Hardens N-F-E's `forge_tick_off_epoch_slot_fails_closed_local` into the named DC-EPOCH-03 proof.
  - `node_forge_no_epoch_boundary_promotion_on_forge_path` — an **on-epoch** forge consumes
    `recovered.chain_dep.epoch_nonce` (the seed nonce) as eta0, and the recovered `chain_dep`
    (incl. `epoch_nonce`) is **identical before and after** the forge tick — no nonce roll /
    `EpochBoundary` promotion.
- **Inherited (must stay green):** `forge_tick_off_epoch_slot_fails_closed_local` (CE-E-7),
  `projection_off_epoch_returns_none` (the BLUE foundation).
- **No crash/WAL semantics changed** — crash recovery is N-F-A/N-F-D's domain.

## 12. Mechanical acceptance criteria
- [ ] `cargo test -p ade_node --lib` — the three `forge_epoch_admission_*` guard tests +
      `node_forge_off_epoch_slot_fails_closed` + `node_forge_no_epoch_boundary_promotion_on_forge_path`
      green; the inherited `forge_tick_off_epoch_slot_fails_closed_local` still green.
- [ ] `ci_check_node_forge_single_epoch_fail_closed.sh` green (new gate): `forge_one_from_recovered`
      calls `forge_epoch_admission` and returns the fail-closed outcome **before** the
      `query_leader_schedule` call; the guard derives the epoch via `era_schedule.locate` (not a
      literal); the node forge path contains **no** `NonceInput::EpochBoundary` /
      `NonceInput::CandidateFreeze` / nonce-roll token.
- [ ] `ci_check_node_run_loop_containment.sh` + `ci_check_loop_planner_closed.sh` +
      `ci_check_no_independent_forge_codepath.sh` + `ci_check_consensus_input_provenance.sh`
      green/unchanged.
- [ ] `ci_check_clock_seam.sh` + `ci_check_forbidden_patterns.sh` green (no float / clock / rand
      added; the guard is pure).
- [ ] `cargo build` + `cargo clippy` clean on touched crates; **`rustfmt` applied to the changed
      files only** (no workspace `cargo fmt -p`).
- [ ] Acceptance scoped to touched crates (`ade_node`, consumed `ade_core`/`ade_ledger`) — not the
      full `ade_testkit` corpus lane (times out ~600s on clean HEAD).

## 13. Failure modes
All **fail-fast / fail-closed** (structured, visible without overclaim):
- Candidate slot epoch ≠ recovered seed epoch → `ForgeEpochAdmission::OffEpoch` → structured
  `ForgeNotLeader`, **before** leadership / KES signing — no forge, no sign, no tip; relay loop
  continues syncing.
- Unlocatable candidate slot → `OffEpoch { None }` → same fail-closed.
- No off-epoch `ForgeSucceeded`, no off-epoch signed/served block, may ever be produced.
- The forge path drives no nonce promotion — the recovered seed-epoch eta0 is never rolled.

## 14. Hard prohibitions (inherits the cluster "Forbidden during this cluster" list)
- **No new `CoordinatorEvent` variant** — the closed GREEN outcome surface + `produce_mode` stay
  untouched; off-epoch reuses `ForgeNotLeader`.
- **No fabricated epoch math** — the candidate epoch is `era_schedule.locate(slot).epoch`; the seed
  epoch is `recovered_inputs.epoch_no`. No epoch literal, no divergent mapping.
- **No cross-epoch nonce roll / `EpochBoundary` / `CandidateFreeze` promotion** on the forge path
  (S4 fails closed instead; the nonce-roll cluster is separate).
- **No second bootstrap / Mithril call / new recovered state** (CN-NODE-01).
- **No new BLUE authority / canonical type / WAL/checkpoint format.**
- No **serve / serve-handoff / live-feed / WirePump / RO-LIVE / BA-02** work (G-B/G-C); no durable
  evidence / persistence / peer-acceptance claim.
- **No relay-containment relaxation** — `ci_check_node_run_loop_containment.sh` stays unchanged.
- **No registry edit** (the `DC-EPOCH-03` flip + `DC-NODE-05` append happen at G-A close).
- **Hard line:** if the epoch guard needs a BLUE change, a new outcome variant, a nonce-roll, serve/
  live wiring, or a second bootstrap — **stop and re-scope.**

## 15. Explicit non-goals
No cross-epoch production / nonce rollover (separate `NonceInput::{CandidateFreeze, EpochBoundary}`
cluster). No serve handoff (G-B). No live feed / `WirePump` / operator pass / peer acceptance (G-C,
operator-gated). No new `CoordinatorEvent` variant. No change to `query_leader_schedule` /
`PoolDistrView` / the BLUE off-epoch `None` (consumed as-is). No mainnet-complete multi-epoch forge.
**No registry flip in this slice** (G-A close performs it).

## 16. Completion checklist
- [ ] `forge_epoch_admission` + `ForgeEpochAdmission` added (GREEN, pure); `forge_one_from_recovered`
      wires it **before** `query_leader_schedule` and fails closed on off-epoch via the existing
      `ForgeNotLeader`; no new `CoordinatorEvent` variant; the forge path drives no nonce promotion.
- [ ] All §12 tests green; the new gate + containment/planner/provenance/clock-seam/forbidden gates
      green & unchanged; `cargo test` scoped to touched crates green; `clippy` clean; changed files
      rustfmt'd (no workspace fmt).
- [ ] Slice doc committed standalone (`docs:`) before implementation; impl committed (`feat:`/
      `test:`) after green, model-attribution trailer. **No registry edit** (deferred to G-A close).
- [ ] After S4 lands green → **G-A close** (`DC-EPOCH-03` declared→enforced; the deferred
      `CN-OPCERT-01` / `CN-GENESIS-01` / `DC-LEDGER-10` / `CN-NODE-01` / `DC-CINPUT-02b` /
      `DC-NODE-05` / `DC-NODE-03` strengthenings; grounding-doc refresh).

## Authority
Registry IDs `DC-EPOCH-03` (S4 provides the core enforcement; the `declared`→`enforced` **flip is
performed at G-A close**), `DC-NODE-05` (strengthened — epoch-hardened; append **deferred to G-A
close**); `DC-CINPUT-02b` / `CN-CINPUT-03` / `CN-NODE-02` / `DC-SYNC-02` / `CN-NODE-01` / `T-DET-01`
/ `DC-COMPAT-01` (preserved). The cluster doc `cluster.md` and `docs/ade-invariant-registry.toml`
are authoritative; this slice doc refines, it does not override.
