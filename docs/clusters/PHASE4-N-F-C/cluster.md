# Cluster PHASE4-N-F-C — Producer Recovered-State Lifecycle

> **Status: OPEN (planned 2026-05-30).** Successor to the PHASE4-N-F-A *capability* cluster. This is the **production-wiring** half: thread the proven N-F-A recovered-state capability (A1 codec, A3a replay, A3b warm-start verify, A4 projection) through ONE production lifecycle owner so the BA-02 producer forges from Ade-owned *recovered* state — never a forge-time operator bundle.
> **Core safety line (binding, from A5 §0):** recovered state must come from verified bootstrap → persist → WAL → warm-start restore; the forge-time shape-swap of `--consensus-inputs-path` into `SeedEpochConsensusInputs` is **forbidden and must be CI-unrepresentable**. Provenance, not shape.
> **Sources:** `docs/planning/phase4-n-f-c-invariants.md` (`61a51ca`) · `docs/planning/phase4-n-f-c-cluster-slice-plan.md` (`0e0b15f`) · `docs/clusters/completed/PHASE4-N-F-A/A5-SCOPING-producer-recovered-state-lifecycle.md` (C1–C6) · guardrail memory `feedback_produce_subordinate_to_sync_spine`, `feedback_shell_must_not_overstate_semantic_truth`.

## Primary invariant
The bounty-primary producer derives its forge base (`PoolDistrView` + eta0) **only** from a recovered `SeedEpochConsensusInputs` — one established at verified bootstrap, sidecar-persisted, WAL-proven, and warm-start-restored+verified — projected via `PoolDistrView::from_seed_epoch_consensus_inputs`; a forge-time operator bundle shape-swapped into that surface is unrepresentable, and exactly one production lifecycle owner threads bootstrap/recovery→produce without a second bootstrap authority. *(Candidate `DC-CINPUT-02b` (producer consumption) + `CN-CINPUT-03` (consume-side fence); strengthens `CN-NODE-01`, `CN-PROD-03`, `DC-CINPUT-01`, `DC-CINPUT-02a`, `CN-STORE-02`, `CN-ANCHOR-01`/`DC-ANCHOR-01`, `T-REC-01`/`T-REC-02`, `DC-WAL-03`, `DC-FORGE-01`.)*

This cluster **adds no new BLUE authority** (A5 §9). The forge, codec, replay, projection, and verify chains all already exist and are consumed verbatim; the cluster builds the RED/GREEN production lifecycle that threads them and the CI fences that keep the threading honest.

## Normative anchors
- `docs/planning/phase4-n-f-c-invariants.md` — the confirmed invariant sketch (§1 always-true, §2 never-possible, §5 transitions, the two candidate rules).
- `docs/clusters/completed/PHASE4-N-F-A/A5-SCOPING-producer-recovered-state-lifecycle.md` — §0 goal/forbidden, §3 target lifecycle, §4 C1–C6, §7 hard non-goals, §8 proof obligations.
- Registry: `CN-NODE-01` (single bootstrap authority), `CN-CINPUT-01`/`CN-CINPUT-02` (sole codec + populate-side/forge-time fence — the CONSUME-side fence is owed here), `DC-CINPUT-01` (warm-start verification — `partial`, production path owed here), `DC-CINPUT-02a` (projection equivalence — consumption owed here), `CN-PROD-03` (current cold-start forge base), `CN-PROD-04`/`CN-FORGE-01..04` (forge/serve symmetry), `CN-ANCHOR-01`/`DC-ANCHOR-01`, `CN-STORE-02`, `DC-WAL-03`, `T-REC-01`/`T-REC-02`, `DC-FORGE-01`, the RO-LIVE family (BA-02 operator-gated).

## Entry conditions (what shipped clusters guarantee)
- **N-F-A (A1–A4):** `SeedEpochConsensusInputs` + sole codec; anchor-fp-keyed sidecar `put_/get_seed_epoch_consensus_inputs` on `SnapshotStore`; `WalEntry::SeedEpochConsensusInputsImported` (tag 3) + `replay_from_anchor → ReplayOutcome{recovered_provenance}`; `bootstrap_initial_state(RequiredFromRecoveredProvenance)` warm-start restore+verify (fail-closed, 5 typed errors) at `bootstrap.rs:159/229/247`; `PoolDistrView::from_seed_epoch_consensus_inputs` projection. All proven at the **authority surface** (tests `bootstrap.rs:695`, `seed_consensus_merge.rs:197`); **none production-wired**.
- **N-K/N-T/N-Y:** single `bootstrap_initial_state` authority (`bootstrap.rs:159`); `PersistentChainDb` impl `ChainDb` + `SnapshotStore` (`chaindb/persistent.rs:85/195/463/565`); `FileWalStore::open`; `recover_node_state` (`recovery/restart.rs:114`, **test-only**); `run_node_until_shutdown` (`node.rs:145`, **test-only callers**); verified-bootstrap composers `genesis_bootstrap`/`mithril_bootstrap` (sidecar+provenance tail, **test callers only**).
- **N-Q/N-R/N-S/N-W/N-X:** real forge (`run_real_forge`), `self_accept`, Praos VRF (`leader_vrf_input`), KES-signs-real-pre-image, tag-24 serve, `Mode::{WireOnly, Admission, KeyGenKes, Produce}` closed (`cli.rs:27`).
- **Current bounty-primary gap:** `run_produce_mode` (`produce_mode.rs:93`) cold-starts from `--consensus-inputs-path` (`import_live_consensus_inputs:188` → `pool_distr_view_from_consensus_inputs:197` → `InMemoryChainDb::new():198`) and passes `SeedEpochConsensusSource::NotRequired:215`. **CE-A-4b is owed here.**

## Exit criteria (mechanical, CI-verifiable)
New test/check names are **candidate** (created by the slice that owns them); existing artifacts are named as-is.

- **CE-C-1 — production owner stood up & routed through the single authority (gate; C1).** Purely-mechanical spine:
  (a) `ci/ci_check_node_mode_closure.sh` green — the `Mode` set stays closed after the owner change (no wildcard dispatch arm);
  (b) `ci/ci_check_bootstrap_closure.sh` green — `bootstrap_initial_state` remains the sole bootstrap pub fn (CN-NODE-01, no second authority);
  (c) candidate gate `ci/ci_check_lifecycle_owner_uses_bootstrap_initial_state.sh` green — the chosen owner obtains initial state **solely** via `bootstrap_initial_state`.
  *(The Option A vs B call-graph comparison and the rejection of any produce-from-operator-files-cold bounty-primary option are a committed rationale artifact — `docs/clusters/PHASE4-N-F-C/C1-DECISION-production-owner.md`, produced by the C1 slice — NOT a CI gate. The mechanical spine above is the formal exit criterion.)*
- **CE-C-2 — production bootstrap composition (C2).** Candidate test `production_owner_persists_seed_epoch_sidecar_over_persistent_store` proves the owner is the **first non-test caller** of a verified-bootstrap composer over `PersistentChainDb` + `FileWalStore` (sidecar `put` + WAL provenance append); `ci/ci_check_consensus_input_provenance.sh` stays green (populate-side still contained), extended with a candidate guard that documented-seed extraction is **bootstrap-time only** (no forge-time residue).
- **CE-C-3 — production warm-start recovery (C3); flips `DC-CINPUT-01` `partial`→`enforced`.** Candidate test `production_warm_start_recovers_seed_epoch_inputs_byte_identical` (production entry, distinct from the authority-surface `warm_start_restores_seed_epoch_consensus_inputs_byte_identical`), plus the five fail-closed tests `production_recovery_fails_closed_on_{missing_sidecar,missing_wal,hash_mismatch,anchor_mismatch,duplicate_provenance}` — **no bundle fallback** on any.
- **CE-C-4 — produce handoff (C4); closes `CE-A-4b`, enforces candidate `DC-CINPUT-02b`.** Candidate test `produce_forge_base_derives_from_recovered_surface` (bounty-primary forge base = `PoolDistrView::from_seed_epoch_consensus_inputs(recovered)`) + `produce_bounty_primary_uses_persistent_chaindb` (no `InMemoryChainDb` on the bounty-primary path).
- **CE-C-5 — consume-side containment (C5); enforces candidate `CN-CINPUT-03`.** `ci/ci_check_consensus_input_provenance.sh` extended with consume-side guards: (neg) no shape-swap populator anywhere in the tree + only a verified-bootstrap/recovery-authorized lifecycle site passes `RequiredFromRecoveredProvenance`; (pos) the producer forge path references the recovered surface only; diagnostic `--consensus-inputs-path`/seed-graft emit **no** BA-02 evidence — gate green.
- **CE-C-6 — BA-02 evidence harness (C6).** Candidate test `ba02_manifest_dry_run_over_synthetic_accept_log` proves the closed BA-02 manifest type correlates forged-block-hash ↔ peer-accept-log over a committed synthetic fixture. *(The **live** Haskell-peer-accept flip is a declared operator-gated obligation on the RO-LIVE family — explicitly NOT a mechanical CE.)*
- **CE-C-7 — replay-equivalence (scoped).** Candidate tests `produce_from_recovered_state_replay_identical` (same recovered base + canonical forge inputs → byte-identical `ForgedBlock`; T-REC-01, DC-FORGE-01) and `first_run_then_warmstart_then_produce_equals_direct_produce` — run **targeted** (`cargo test -p ade_runtime`, `cargo test -p ade_node`, and the specific `ade_testkit` producer/recovery harness tests), **not** the timing-out full `ade_testkit` corpus/oracle lane.

## Expected slice types (safety order — note the C5/C4 split)
- **C1 — Production lifecycle owner selection + skeleton** — RED owner module + committed decision doc `C1-DECISION-production-owner.md`; routes solely through `bootstrap_initial_state`; leaves diagnostic `produce_mode` intact. *(CE-C-1)*
- **C2 — Production bootstrap composition** — RED driver wiring the verified-bootstrap composer over `PersistentChainDb`+`FileWalStore` (first non-test caller); GREEN merge + BLUE codec reused; bootstrap-time-only extraction guard. *(CE-C-2)*
- **C3 — Production warm-start recovery** — GREEN first-run-vs-warm-start branch + RED store/WAL open; BLUE A3b verify chain reused; fail-closed, no fallback. *(CE-C-3)*
- **C5n — Consume-side *negative* fence** — CI extension to `ci_check_consensus_input_provenance.sh` (no shape-swap populator; only authorized site passes `RequiredFromRecoveredProvenance`). Lands **before** C4. *(CE-C-5 negative half)*
- **C4 — Produce handoff + consume-side *positive* assertion** — RED handoff building the forge base from the recovered surface; BLUE A4/forge reused; CI positive assertion. Consumer fenced from the moment it exists. *(CE-C-4 + CE-C-5 positive half + CE-C-7)*
- **C6 — BA-02 evidence correlation** — RED closed manifest type + correlator + synthetic dry-run; live flip operator-gated. *(CE-C-6)*

## TCB color map (FC/IS partition)
- **BLUE (reuse only — no new authority):** `ade_ledger::{seed_consensus_inputs, wal, consensus_view::from_seed_epoch_consensus_inputs, producer::*, block_validity}`, `ade_core::consensus::{leader_check, vrf_cert, leader_schedule}`.
- **GREEN:** the first-run-vs-warm-start branch decision + lifecycle sequencing reducer (`ade_runtime`, GREEN-by-content, mirroring `forward_sync::reducer`). Must be a pure function of persisted state.
- **RED:** the lifecycle owner/driver (the chosen `ade_node` mode + `ade_runtime` plumbing opening `PersistentChainDb`/`FileWalStore`, anchor mint/bind, recovery, slot loop, peer I/O), the C6 evidence correlator, the diagnostic-fence CLI handling.
- **CI (enforcement, not a color):** `ci_check_consensus_input_provenance.sh` (extended), candidate `ci_check_lifecycle_owner_uses_bootstrap_initial_state.sh`.
- **Open color (C1/C2 resolve):** whether the lifecycle-owner *home* is an evolved `Mode::Admission` (Option A) or a dedicated producer lifecycle mode (Option B). This is the cluster's first gate and changes where the RED driver lives — it does **not** change any color assignment above.

## Forbidden during this cluster
- Patching cold `produce_mode` directly into a fake recovered-state consumer.
- Shape-swapping `--consensus-inputs-path`/`import_live_consensus_inputs` output into `SeedEpochConsensusInputs` and feeding it to the A4 projection (provenance, not shape).
- A second bootstrap/recovery/storage-init authority (CN-NODE-01).
- `InMemoryChainDb` cold-start as the bounty-primary forge base.
- A bundle fallback on any recovery failure (must fail closed, typed).
- Merging C4 (consumer) before C5n (negative fence) — no unfenced consume window.
- Claiming BA-02 from anything but a real Haskell-peer accept; claiming `recover_node_state`/`run_node_until_shutdown` is production-wired unless the slice actually wires it.
- No new BLUE authority, no new canonical type, no `HashMap`/clock/float/async in BLUE.

## Invariants strengthened / candidate new rules
- **Candidate new (in the invariant sketch, declared, not yet appended):** `DC-CINPUT-02b` (producer consumption of the recovered surface — C4), `CN-CINPUT-03` (consume-side fence / no shape-swap populator — C5). Promoted at `/cluster-close`, not before.
- **Strengthenings (`strengthened_in += "PHASE4-N-F-C"` at close):** `CN-NODE-01` (lifecycle owner routes through the one authority), `CN-PROD-03` (cold-start scope retired for the bounty-primary path), `DC-CINPUT-01` (`partial`→`enforced`), `DC-CINPUT-02a` (consumption now exercised), `CN-STORE-02`, `CN-ANCHOR-01`/`DC-ANCHOR-01`, `T-REC-01`/`T-REC-02`, `DC-WAL-03`, `DC-FORGE-01`. BA-02 live flip declared on the RO-LIVE family (`blocked_until_operator_pass_executed`).

## Replay obligations (scoped — NOT a full-corpus run)
- No new canonical BLUE type and no new authoritative transition (A5 §9) → no new replay-corpus *format*.
- **New scoped fixture:** a production-lifecycle `persist → recover → produce` fixture (CE-C-7), added alongside/extending `warm_start_restores_seed_epoch_consensus_inputs_byte_identical`, `wal_replay_from_anchor`, and the producer-from-recovered harness tests.
- **Acceptance is scoped** to producer-from-recovered-state fixtures + the touched crates (`ade_runtime`, `ade_node`, and the specific `ade_testkit` producer/recovery harness tests) — **never** the whole-workspace `ade_testkit` oracle/corpus lane as a default gate (known to time out ~600s on clean HEAD for environmental reasons until the test-hygiene lane is fixed).
- Determinism guard: the first-run-vs-warm-start branch decision must be proven a pure function of persisted state (no wall-clock/env) — GREEN, replay-checked, not BLUE.

## The C1 first gate (must be resolved before C2–C4 assume a wiring path)
Decide the production lifecycle owner; reject any option that leaves `produce_mode` cold-starting from operator files as the bounty-primary path. The decision + call-graph comparison is recorded in the committed rationale artifact `docs/clusters/PHASE4-N-F-C/C1-DECISION-production-owner.md` (produced by the C1 slice; not a CI gate — CE-C-1 is the mechanical spine above).
- **Option A — evolve `Mode::Admission`** (`admission/bootstrap.rs:114 run_admission_inner`): already has `mint:151` + `FileWalStore::open:181` + `bootstrap_initial_state` warm-start, but still `import_live_consensus_inputs:194` and does not call the composers/persist the sidecar.
- **Option B — dedicated producer lifecycle mode** owning `PersistentChainDb` (`chaindb/persistent.rs:85`, `ChainDb:195`+`SnapshotStore:463`+`put_seed_epoch_consensus_inputs:565`) + `FileWalStore` + `BootstrapAnchor` + recovery (`recovery/restart.rs:114`, `node.rs:145` — both test-only today) + produce.
- **Reject criterion (grounded):** `run_produce_mode` (`produce_mode.rs:93`) cold-starts at `:188–215` over `InMemoryChainDb` with `SeedEpochConsensusSource::NotRequired`; the `coordinator.rs` `GenesisAnchor` and the `:447` zero-hash anchor are timing/KES placeholders, **not** the verified-bootstrap `BootstrapAnchor`. Any option leaving this as bounty-primary is rejected.

## Non-goals (A5 §7)
No new BLUE authority. No direct UTXO-HD/LedgerDB decoder. No mainnet Byron→Conway historical replay. No cross-epoch production. No forged-block durability beyond N-U. No BA-01/BA-03/BA-04/BA-09 claim. No live BA-02 run without explicit operator green-light. No grounding-doc regeneration (that's `/cluster-close`).
