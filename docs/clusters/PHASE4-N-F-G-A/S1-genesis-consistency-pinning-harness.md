# PHASE4-N-F-G-A — Slice S1: Genesis-consistency pinning harness

> **Status:** slice doc (IDD Part IV). Companion to `cluster.md` (S1 row + CE-G-A-1) and
> `../../planning/phase4-n-f-g-{invariants,cluster-slice-plan}.md`. Code-verified against HEAD
> `58809947`/`f08b12ca`.
>
> **Slice S1 in one line:** prove — hermetically, against a committed
> private-genesis-derived leadership reference fixture — that the WarmStart-recovered seed epoch
> is **genesis-consistent on both surfaces** (eta0 from `chain_dep`; stake/ASC/per-pool
> VRF-keyhash from `SeedEpochConsensusInputs`), that Ade's `praos_vrf_input` + leader-threshold
> inputs match the reference, and that the pre-seed→recover round-trip is faithful — **or stop
> and insert S1b.**

## 1. Slice identity
- **Cluster:** PHASE4-N-F-G-A (forge fidelity on the node spine). The **first hard fork** —
  gates S2–S4 and all of G-B/G-C.
- **Slice:** S1 — genesis-consistency pinning harness.
- **Module:** a **GREEN `ade_testkit` pinning harness** (fixture loader + pinning helpers) + its
  tests; a **committed private-genesis-derived leadership reference fixture** (reference-data
  discipline). **No production code change**, no new BLUE authority, no gate that changes
  production behavior.

## 2. Cluster Exit Criteria addressed (verbatim)
- **CE-G-A-1** — genesis-consistency: candidate harness/tests
  `pinning_recovered_eta0_matches_genesis_fixture`,
  `pinning_recovered_stake_asc_vrf_matches_genesis_fixture`,
  `pinning_praos_vrf_input_and_threshold_match_fixture`,
  `pinning_preseed_warmstart_roundtrip_faithful` pass against a **committed reference fixture**;
  comparisons are observable/derived-surface only (existing
  `ci_check_no_haskell_fingerprint_equality.sh` / DC-COMPAT-01 holds). *(gates S2–S4 + all
  G-B/G-C work.)*

(CE-G-A-2 = S2, CE-G-A-3 = S3, CE-G-A-4 = S4 — explicitly out of S1 scope.)

## 3. Intent (invariant impact)
Establish the **genesis-consistency assurance** of the recovered-surface forge: that the
leadership inputs the node forge path consumes (eta0 = recovered `PraosChainDepState.epoch_nonce`;
stake/ASC/per-pool VRF-keyhash = recovered `SeedEpochConsensusInputs`) are **byte-faithful to a
genesis-derived reference**, and that Ade's leader-eligibility computation (`praos_vrf_input` +
`query_leader_schedule` threshold inputs) **matches** that reference. This is the proof that,
given a genesis-consistent operator bundle, Ade will compute leadership the way a from-genesis
peer does — the precondition for any peer to accept an Ade-forged block. It is the **gate**:
until it passes, no serve (G-B) or live (G-C) work is sound.

## 4. Pre-conditions (verified at HEAD)
- `consensus::vrf_cert::praos_vrf_input(slot, &Nonce) -> [u8;32]` (`vrf_cert.rs:131`),
  `ActiveSlotsCoeff{numer,denom}` (`:252`) — BLUE, validation-side corpus-proven.
- `consensus::leader_schedule::query_leader_schedule(...)` (`leader_schedule.rs:79`) — BLUE
  threshold `1-(1-f)^σ`.
- `SeedEpochConsensusInputs{epoch_no, active_slots_coeff, total_active_stake, pool_distribution:
  BTreeMap<Hash28, PoolEntry>}`; `PoolEntry{active_stake, vrf_keyhash: Hash32}`
  (`seed_consensus_inputs.rs`, `consensus_view.rs`) — carries stake/ASC/vrf, **not** eta0.
- `PraosChainDepState.epoch_nonce: Nonce` (`praos_state.rs:118`) — **this is eta0**.
- `bootstrap::warm_start_recovery` /
  `bootstrap_initial_state(SeedEpochConsensusSource::RequiredFromRecoveredProvenance)`
  (`node_lifecycle.rs:728`, `bootstrap.rs:64,229`) — RED single-authority recovery; the path the
  harness drives.
- `admission::seed_to_snapshot(utxo, chain_dep, slot, store)` (`seed_to_snapshot.rs:55`) — RED
  proven extraction, reused to pre-seed the test store.
- Warm-start recovery is already exercised hermetically by N-F-A/N-F-C tests (the harness reuses
  that infrastructure).

## 5. Implementation boundary (test + fixture + harness; reuse, no production change)
- **Committed reference fixture** — a committed private-genesis-derived leadership reference
  fixture for this proof (genesis config + expected `{eta0, per-pool active_stake,
  total_active_stake, ASC numer/denom, per-pool vrf_keyhash}` + expected `praos_vrf_input(slot,·)`
  for sample slots + expected threshold inputs). **Provenance documented** (reference-data
  discipline: genesis-derived, extracted offline once from a real cardano-cli/cardano-node on the
  committed private genesis — not Ade-derived, not a live operator-pass). Stored as committed
  reference data under `ade_testkit` (exact path set at implement-slice).
- **GREEN harness** (`ade_testkit`) — a fixture loader + pinning helpers that (1) build the
  fixture's `UTxOState` + `chain_dep` (with eta0) + `SeedEpochConsensusInputs`; (2) pre-seed a
  fresh test store via `seed_to_snapshot` + the seed-epoch sidecar + provenance WAL append
  (reusing the existing composer mechanism); (3) run `warm_start_recovery` → recovered
  `BootstrapState`; (4) expose the recovered (eta0, stake/ASC/vrf) + Ade's `praos_vrf_input` /
  threshold inputs for assertion.
- **Tests** assert the four pinning claims (below) against the fixture. **No production code, no
  new BLUE authority, no second bootstrap** (CN-NODE-01 — recovery via the single existing
  authority).

## 6. TCB color (execution boundary)
- **GREEN:** the `ade_testkit` pinning harness (fixture loader + pinning helpers) — deterministic
  glue, affects no authoritative output.
- **RED (orchestration, reused):** `bootstrap::warm_start_recovery` + `admission::seed_to_snapshot`
  + the store pre-seed — driven by the harness, unmodified.
- **BLUE (consumed, unchanged):** `praos_vrf_input`, `query_leader_schedule`,
  `from_seed_epoch_consensus_inputs`, `PraosChainDepState` — read-only.
- **No production module changes; no ambiguous colors.** (Mirrors N-F-E S3a: test orchestration
  over GREEN/BLUE.)

## 7. Invariants preserved (must not weaken) — by registry ID
- `CN-NODE-01` — recovery via the single `bootstrap_initial_state`/`warm_start_recovery`
  authority; the pre-seed reuses `seed_to_snapshot`, **no new bootstrap authority, no second
  recovered state, no Mithril call**.
- `DC-CINPUT-02b` / `CN-CINPUT-03` — leadership inputs still come only from the recovered surface
  (guard d); the harness fabricates no production `SeedEpochConsensusInputs` (it constructs *test*
  fixture inputs and drives the real recovery).
- `DC-COMPAT-01` — comparisons are observable/derived surfaces (VRF-input bytes, stake numbers,
  ASC fraction, vrf_keyhash, threshold inputs) — **never** an
  Ade-internal-fingerprint-vs-Haskell-serialized-state-hash equality.
- `CN-NODE-02` / `DC-NODE-05` — `run_relay_loop` / `run_node_sync` / the forge path are
  **untouched**; the relay-loop containment gate is unchanged.
- All BLUE invariants — untouched (read-only consumption).

## 8. Invariants strengthened (one family: recovered-surface genesis-consistency)
- `DC-NODE-05` — S1 discharges the **genesis-consistency clause** of the recovered-surface forge
  (CE-G-A-1): the recovered leadership inputs are proven genesis-faithful and Ade's leader
  computation matches a genesis-derived reference. **No registry edit in S1** — the
  `strengthened_in += "PHASE4-N-F-G-A"` append (DC-NODE-05; and DC-EPOCH-03's `declared→enforced`)
  is **deferred to cluster close**, when CE-G-A-1..4 are all green (per the N-F-E S3a pattern). No
  status flip in this slice.

## 9. Reference fixture & provenance (S1-specific, load-bearing)
**The fixture is evidence input, not runtime authority.** It may prove that Ade's recovered
surfaces match a committed private-genesis-derived reference, but it must **not** become an
alternate production source of eta0, stake, ASC, or VRF keyhash — those come only from the
recovered surface via the single bootstrap authority (guard d / CN-NODE-01).

The fixture's authority is **reference-data provenance**, not Ade self-derivation: Ade does
**not** reimplement the Haskell genesis→initial-nonce rule (per the C1 §4a "extract-once, don't
re-derive" finding). The committed expected `{eta0, stake, ASC, vrf_keyhash}` are genesis-derived
values obtained offline (cardano-cli `query protocol-state` / `stake-snapshot` against the
committed private genesis, or a captured reference). **Honest split:** formula-agreement
(`praos_vrf_input`/threshold) and recovery-faithfulness are hermetically provable with the
committed fixture; the genesis-*derivation* authority is the fixture's documented provenance. If
a genesis-faithful reference cannot be established, the genesis-consistency claim cannot be made →
contingency (§13).

## 10. Replay / determinism obligations
The harness is deterministic: `BTreeMap`-ordered recovered surface, no wall-clock/rand/float
(recovery + pinning are pure functions of the fixture). Two runs of each pinning test over the
committed fixture are byte-identical. The pre-seed→recover round-trip extends the existing N-F-A
pre-seed→WarmStart-recover replay-equivalence to the genesis-consistency fixture. **No new
on-disk replay corpus, no new canonical type.**

## 11. Replay / crash / epoch validation (tests by name)
- **New (the four pinning tests):**
  - `pinning_recovered_eta0_matches_genesis_fixture` — recovered `chain_dep.epoch_nonce` ==
    fixture eta0.
  - `pinning_recovered_stake_asc_vrf_matches_genesis_fixture` — recovered
    `SeedEpochConsensusInputs` (per-pool `active_stake`, `total_active_stake`, ASC `numer/denom`,
    per-pool `vrf_keyhash`) == fixture.
  - `pinning_praos_vrf_input_and_threshold_match_fixture` — Ade's `praos_vrf_input(slot,
    recovered_eta0)` (sample slots) **and** `query_leader_schedule` threshold inputs
    (stake_fraction, ASC) == fixture expected.
  - `pinning_preseed_warmstart_roundtrip_faithful` — pre-seed (via `seed_to_snapshot` + sidecar +
    provenance WAL) → `warm_start_recovery` → recovered state is byte-faithful to the fixture
    inputs.
- **No crash/epoch validation in S1** — epoch-boundary fail-closed is S4 (CE-G-A-4); crash
  recovery is N-F-A/N-F-D's domain.

## 12. Mechanical acceptance criteria
- [ ] `cargo test -p ade_testkit <the four pinning tests>` pass against the committed reference
      fixture (two-run-stable; values match the genesis-derived expected).
- [ ] `ci_check_no_haskell_fingerprint_equality.sh` stays green (DC-COMPAT-01 — derived/observable
      comparisons only, no state-hash equality).
- [ ] Candidate `ci_check_genesis_consistency_fixture_present.sh` — the reference fixture is
      committed and the harness references it (mirrors the existing `ci_check_*_corpus_present.sh`
      pattern).
- [ ] `cargo build` clean on touched crates; `rustfmt` applied.
- [ ] Gates unchanged + green: `ci_check_node_run_loop_containment.sh` (untouched),
      `ci_check_consensus_input_provenance.sh` (guard d), `ci_check_operator_forge_no_secret_leak.sh`,
      `ci_check_lifecycle_owner_uses_bootstrap_initial_state.sh` (no new bootstrap).
- [ ] Acceptance scoped to touched crates (`ade_testkit` + consumed
      `ade_core`/`ade_ledger`/`ade_runtime`) — **not** the full `ade_testkit` corpus/oracle lane
      (times out ~600s on clean HEAD).

## 13. Failure modes & the hard-fork contingency (S1b)
Each is **fail-fast** (the test fails → the slice does not close → STOP, do not proceed to S2–S4
or any G-B/G-C work):
- `pinning_praos_vrf_input_and_threshold_match_fixture` fails → **Ade's leader-eligibility formula
  diverges** from the genesis-derived reference → **S1b: formula remediation**.
- `pinning_preseed_warmstart_roundtrip_faithful` fails → **recovery corrupts** the
  genesis-consistent inputs → **S1b: recovery fix**.
- the recovered surfaces structurally cannot carry a genesis-derived value (a needed field absent)
  → **S1b: private-net bootstrap/anchor path**.
- a genesis-faithful reference fixture cannot be established (no offline reference extraction) →
  **STOP; re-scope** — the genesis-consistency claim is unprovable as scoped.

## 14. Hard prohibitions (inherits the cluster Forbidden list)
- No production code change, no new BLUE authority/canonical type, no gate that changes production
  behavior.
- No second bootstrap / no Mithril call / no parallel recovery (CN-NODE-01); recovery only via the
  single existing authority.
- No fabricated *production* `SeedEpochConsensusInputs`/eta0/pparams/pool_id; no bundle token on
  the forge path (guard d). **The committed fixture must not become an alternate production source
  of eta0/stake/ASC/vrf (§9).**
- **No state-hash / internal-fingerprint equality** against a Haskell reference (DC-COMPAT-01) —
  derived/observable surfaces only.
- No serve / serve-handoff / live-feed / `WirePump` / RO-LIVE / BA-02 work (G-B/G-C).
- Do not relax `ci_check_node_run_loop_containment.sh`.
- **Hard line:** if genesis-consistency cannot be proven against the committed fixture — **stop
  and insert S1b**; do not proceed.

## 15. Explicit non-goals
No S2 ingress / S3 slot-alignment / S4 epoch-fail-closed work. No serve handoff, live feed,
operator pass, or peer acceptance. No offline genesis→eta0 derivation in Ade (extract-once
reference data is the source). No cross-epoch / nonce-roll. No mainnet-fidelity pparams.

## 16. Completion checklist
- [ ] Committed private-genesis-derived leadership reference fixture in place with documented
      genesis-derivation provenance.
- [ ] GREEN `ade_testkit` pinning harness + the four pinning tests added; all pass two-run-stable
      against the fixture.
- [ ] DC-COMPAT-01 gate green; candidate fixture-present gate green; the four carried gates green.
- [ ] `cargo test` scoped to touched crates green (count grown); `rustfmt` applied.
- [ ] Slice doc committed standalone (`docs:`) before implementation; impl committed
      (`test:`/`feat:`) after green. No registry edit (deferred to cluster close).
- [ ] **If any pinning claim fails → STOP and open S1b; do not proceed to S2–S4 / G-B / G-C.**

## Authority
Registry IDs `DC-NODE-05` (genesis-consistency clause discharged; registry append deferred to
close), `CN-NODE-01` / `DC-CINPUT-02b` / `CN-CINPUT-03` / `DC-COMPAT-01` / `CN-NODE-02`
(preserved). The cluster doc `cluster.md` and `docs/ade-invariant-registry.toml` are
authoritative; this slice doc refines, it does not override.
