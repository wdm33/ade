# Cluster/Slice Plan — Ade · PHASE4-N-F-C (Producer Recovered-State Lifecycle)

> **IDD Part I / cluster-planning artifact.** Overall ordered plan only — full cluster doc is `/cluster-doc PHASE4-N-F-C`, slice docs are `/slice-doc`.
> Source invariant sketch: `docs/planning/phase4-n-f-c-invariants.md` (committed `61a51ca`).
> Scoping source: `docs/clusters/completed/PHASE4-N-F-A/A5-SCOPING-producer-recovered-state-lifecycle.md` (C1–C6).
> Status: **confirmed** (2026-05-30). This is a per-cluster plan; it does **not** replace the overarching Phase-4 plan at `docs/active/phase_4_cluster_plan.md`.

## Cluster Index (Dependency Order)

This is **one cluster** (`PHASE4-N-F-C`), six slices, ordered by safety not delivery. It carries
**no new BLUE authority** (A5 §9) — it threads the proven N-F-A capability (A1 codec, A3a replay,
A3b warm-start verify, A4 projection) through a single production owner. BA-02's *live* flip is
operator-gated and is **not** a mechanical cluster CE (it is a declared obligation on the RO-LIVE
family), consistent with how N-S / N-R handled CN-CONS-06 / RO-LIVE-01.

1. `PHASE4-N-F-C` — Producer Recovered-State Lifecycle — primary invariant: **the BA-02 producer
   forges from Ade-owned *recovered* state (verified bootstrap → persisted → WAL-proven →
   warm-start-restored → A4-projected), never from a forge-time operator bundle; provenance, not
   shape.**

---

## Cluster `PHASE4-N-F-C` — Producer Recovered-State Lifecycle

- **Primary invariant:** Exactly one production lifecycle owner threads
  `verified-bootstrap/recovery → produce` without a second bootstrap authority (CN-NODE-01); the
  bounty-primary forge base derives **only** from the recovered `SeedEpochConsensusInputs`
  (DC-CINPUT-02b); the shape-swap of an operator bundle into that surface is unrepresentable
  (CN-CINPUT-03).

- **TCB partition:**
  - **BLUE (reuse only — no new authority):** `ade_ledger::{seed_consensus_inputs, wal,
    consensus_view, producer::*, block_validity}`, `ade_core::consensus::{leader_check, vrf_cert,
    leader_schedule}`. The A1 codec, A3a replay, A4 projection, forge / `self_accept` are consumed
    verbatim.
  - **GREEN:** the first-run-vs-warm-start **branch decision** (pure over persisted state) + any
    lifecycle sequencing reducer, in `ade_runtime` (mirroring the `forward_sync::reducer` GREEN
    pattern).
  - **RED:** the production lifecycle **owner / driver** — the chosen `ade_node` mode + `ade_runtime`
    plumbing that opens `PersistentChainDb` + `FileWalStore`, mints / binds the `BootstrapAnchor`,
    runs recovery, runs the slot loop and peer I/O; plus the C6 BA-02 evidence correlator and the
    diagnostic-fence CLI handling.
  - **CI (enforcement):** extend `ci_check_consensus_input_provenance.sh` (CN-CINPUT-03) + a
    forge-base-provenance assertion (DC-CINPUT-02b).

- **Cluster Exit Criteria:**
  - **CE-C-1** — Production owner chosen via a **committed call-graph comparison** of Option A (evolve
    `Mode::Admission` — already has `mint` + `FileWalStore::open` + `run_admission`) vs Option B
    (dedicated producer lifecycle mode owning `PersistentChainDb` + `FileWalStore` + `BootstrapAnchor`
    + recovery + produce); the chosen owner routes initial state **solely** through
    `bootstrap_initial_state` (CN-NODE-01 preserved, no second authority); any option leaving
    `produce_mode` cold-starting from operator files as the **bounty-primary** path is **rejected**.
  - **CE-C-2** — The chosen owner is the **first non-test caller** of a verified-bootstrap composer
    (`genesis_bootstrap` / `mithril_bootstrap`): production sidecar `put` (A2) + WAL provenance append
    (A3a) over a **`PersistentChainDb` + `FileWalStore`** (not `InMemoryChainDb`). First-run
    documented seed extraction is **bootstrap-time only** and **must not remain a forge-time input**
    once recovery / produce wiring lands (C3 / C4) — it is an extraction source for the persisted
    recovered surface, never the forge-time bounty-primary input.
  - **CE-C-3** — Production warm-start recovery: owner opens the persistent store + WAL, replays
    provenance, and restores + verifies via `bootstrap_initial_state(RequiredFromRecoveredProvenance)`
    — **fail-closed on all five modes** (missing sidecar / missing WAL / hash mismatch / anchor
    mismatch / duplicate), **no bundle fallback** (flips **DC-CINPUT-01 partial→enforced**).
  - **CE-C-4** — Produce handoff: the bounty-primary forge base (`PoolDistrView` + eta0) is built from
    the **recovered** `SeedEpochConsensusInputs` via `PoolDistrView::from_seed_epoch_consensus_inputs`
    (closes **CE-A-4b / DC-CINPUT-02b**); no `InMemoryChainDb` cold-start on the bounty-primary path.
  - **CE-C-5** — Consume-side containment: `ci_check_consensus_input_provenance.sh` extended so (neg)
    no shape-swap populator exists anywhere + only a verified-bootstrap / recovery-authorized
    lifecycle site passes `RequiredFromRecoveredProvenance`, and (pos) the producer forge path
    references the recovered surface only; diagnostic `--consensus-inputs-path` / seed-graft emit
    **no** BA-02 evidence (**CN-CINPUT-03**).
  - **CE-C-6** — BA-02 evidence-correlation **harness + manifest schema** exist; a **dry-run over a
    synthetic peer-accept log** yields a valid manifest. *(The live Haskell-peer-accept flip is a
    **declared operator-gated obligation** on the RO-LIVE family — explicitly **not** a mechanical CE;
    no BA-02 claim until real peer-accept evidence exists.)*
  - **CE-C-7 (replay)** — `produce-from-recovered-state` replay-equivalence: same recovered base +
    canonical forge inputs → **byte-identical `ForgedBlock`** (T-REC-01, DC-FORGE-01);
    `first-run(persist) → warm-start(recover) → produce` is replay-equivalent to a direct produce from
    the same canonical inputs (shutdown / resume identity extended to the producer).

- **Slices** (safety order; note the C5 / C4 split):
  - **C1 — Choose & stand up the production lifecycle owner** — invariant: one owner, routed solely
    through `bootstrap_initial_state`, no second bootstrap authority; produce-from-operator-files-cold
    rejected as bounty-primary — addresses: CE-C-1 — TCB: **RED** (owner skeleton) + decision doc.
    *Deliverable: committed A-vs-B call-graph comparison + the chosen owner module boundary, leaving
    the existing diagnostic `produce_mode` intact and correct.*
  - **C2 — Production bootstrap composition** — invariant: first non-test caller of the A2 put + A3a
    provenance append over a persistent store; documented seed extraction is bootstrap-time only —
    addresses: CE-C-2 — TCB: **RED** driver + **GREEN** merge (reused) + **BLUE** codec (reused).
  - **C3 — Production warm-start recovery** — invariant: open persistent store + WAL, restore + verify
    fail-closed, no bundle fallback — addresses: CE-C-3 (flips DC-CINPUT-01) — TCB: **GREEN** branch
    decision + **RED** store / WAL open + **BLUE** A3b verify chain (reused).
  - **C5n — Consume-side *negative* fence (lands before the consumer)** — invariant: no shape-swap
    populator anywhere; only an authorized lifecycle site may pass `RequiredFromRecoveredProvenance`
    — addresses: CE-C-5 (negative half) — TCB: **CI**. *Vacuously green at merge (no consumer yet)
    but forbids the §0 forbidden move from ever existing.*
  - **C4 — Produce handoff + consume-side *positive* assertion** — invariant: bounty-primary forge
    base derives from the recovered surface via A4; producer forge path references the recovered
    surface only — addresses: CE-C-4 + CE-C-5 (positive half) + CE-C-7 — TCB: **RED** handoff +
    **BLUE** A4 projection / forge (reused) + **CI** (positive assertion). *The consumer is fenced
    from the moment it exists.*
  - **C6 — BA-02 evidence correlation** — invariant: forged-block-hash ↔ peer-accept-log correlation
    is a closed manifest; dry-run produces a valid manifest; live flip operator-gated — addresses:
    CE-C-6 — TCB: **RED** correlator (non-authoritative).

- **Replay obligations (scoped — NOT a full-corpus run):**
  - **No new canonical BLUE type, no new authoritative transition** (A5 §9) — so no new replay-corpus
    *format*.
  - **Acceptance is scoped to producer-from-recovered-state fixtures + the affected crates / gates**,
    **not** a full `ade_testkit` oracle / corpus replay. The full `ade_testkit` corpus/oracle suite
    is known to time out (~600s) on clean HEAD for environmental reasons; until that test-hygiene lane
    is fixed, this cluster's replay acceptance is the new producer-from-recovered-state fixtures plus
    targeted tests in the touched crates (`ade_runtime`, `ade_node`, and the `ade_testkit` producer /
    recovery harness modules only) — never the whole-workspace oracle replay as a default gate.
  - **New replay fixtures (scoped):** a production-lifecycle fixture exercising
    `persist → recover → produce` (CE-C-7), added alongside / extending the existing
    `shutdown_resume_identity`, `wal_replay_from_anchor`, and producer-from-recovered harness tests.
    Run them targeted (e.g. `cargo test -p ade_runtime`, `cargo test -p ade_node`, and the specific
    `ade_testkit` producer / recovery harness tests), not the timing-out full `cargo test -p
    ade_testkit` corpus lane.
  - **Determinism guard:** the first-run-vs-warm-start branch decision must be proven a pure function
    of persisted state (no wall-clock / env) — it is GREEN and replay-checked, not BLUE.

- **Registry impact (at close, not now):**
  - **Promote** (currently declared candidates in the invariant sketch): `DC-CINPUT-02b` (C4),
    `CN-CINPUT-03` (C5).
  - **Strengthen** (`strengthened_in += "PHASE4-N-F-C"`): `CN-NODE-01`, `CN-PROD-03` (cold-start scope
    retired for bounty-primary), `DC-CINPUT-01` (→ enforced), `DC-CINPUT-02a` (consumption
    exercised), `CN-STORE-02`, `CN-ANCHOR-01`, `DC-ANCHOR-01`, `T-REC-01`, `T-REC-02`, `DC-WAL-03`,
    `DC-FORGE-01`.
  - **Operator-gated obligation:** BA-02 live flip declared on the **RO-LIVE** family
    (`blocked_until_operator_pass_executed`), not satisfied by this close.

- **Plan-level forbidden moves (encoded guardrails):**
  1. Do **not** patch cold `produce_mode` directly into a fake recovered-state consumer.
  2. Do **not** shape-swap `--consensus-inputs-path` into `SeedEpochConsensusInputs` (provenance, not
     shape) — CN-CINPUT-03 makes it unrepresentable.
  3. Do **not** claim BA-02 until real Haskell-peer-acceptance evidence exists.
  4. Do **not** introduce a second bootstrap / recovery / storage-init authority (CN-NODE-01).
  5. Do **not** merge C4 (consumer) before C5n (negative fence) — no unfenced consume window.

---

## Complete-work-only check

- Every CE above is reachable by the slices in **this** cluster — **except** the BA-02 *live* flip,
  which is deliberately **not** a CE (it is an operator-gated obligation). The cluster closes
  mechanically with CE-C-1…C-7 green.
- No carry-forward: CE-A-4b is closed **here** (C4), not deferred.
- Each slice leaves the system correct: C1 leaves the diagnostic `produce_mode` working; C2 / C3 add
  a persistent owner without breaking it; C5n fences before C4 opens the consume path; C6 is
  non-authoritative tooling.

## C1 call-graph inputs (for the first-gate decision; from the A5 grounding)

- **Option A — evolve `Mode::Admission`:** `admission/bootstrap.rs run_admission_inner` already has
  `mint(MintInputs{…})` + `FileWalStore::open` + `run_admission`, but still sources consensus inputs
  from `--consensus-inputs-path` and does **not** call the composers or persist the sidecar.
- **Option B — dedicated producer lifecycle mode:** owns `PersistentChainDb`
  (`chaindb/persistent.rs:85`, impl `ChainDb:195` + `SnapshotStore:463`) + `FileWalStore` +
  `BootstrapAnchor` + recovery (`recovery/restart.rs:114 recover_node_state`, today test-only;
  `node.rs run_node_until_shutdown`, today test-only) + produce.
- **Reject criterion:** `produce_mode.rs:93 run_produce_mode` cold-starts at `:184-204` from operator
  files (`--json-seed-path`, `--consensus-inputs-path`) over an `InMemoryChainDb`; the zero-hash
  anchor at `:447` and the `GenesisAnchor` at `coordinator.rs:48` are timing/KES placeholders, **not**
  the verified-bootstrap `BootstrapAnchor`. Any option leaving this as the bounty-primary path is
  rejected.
