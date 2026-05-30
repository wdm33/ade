# PHASE4-N-F-A / A5 — SCOPING (not a slice): Producer Recovered-State Lifecycle

> **This is a SCOPING document, not a slice doc.** Grounding A5 against
> the real producer code (2026-05-30) showed A5 is **not** a slice-sized
> call-site swap — it is a **successor lifecycle cluster**. No
> production-path code is written until that cluster is scoped via
> `/invariants` + `/cluster-plan` with per-slice docs reviewed.
>
> **Proposed successor cluster: `PHASE4-N-F-C — Producer Recovered-State
> Lifecycle`** (use the correct next cluster ID at plan time). A5 is
> retired as a slice id; its intent becomes this cluster.

## 0. Reset the goal (normative)

We are **not** trying to make `produce_mode` *look like* it consumes
`SeedEpochConsensusInputs`. We are trying to make the **BA-02 producer**
consume **Ade-owned recovered state**, where the seed-epoch consensus
inputs were established during verified bootstrap, persisted as
anchor-bound Ade state, WAL-proven, restored + verified on warm-start,
projected through A4, and only then used for production.

**The thing we are trying to accomplish:**
```
verified bootstrap / documented seed extraction
  → composer persists SeedEpochConsensusInputs sidecar (A2)
  → WAL records SeedEpochConsensusInputsImported provenance (A3a, commit point)
  → production recovery / warm-start restores AND verifies the sidecar (A3b, fail-closed)
  → A4 projects recovered sidecar → PoolDistrView / ExpectedVrfInput
  → produce_mode forges block #(tip+1) from recovered selected tip + recovered consensus inputs
  → Haskell peer acceptance is the ONLY BA-02 proof
```

**The thing we must NOT do (forbidden — semantic laundering):**
```
--consensus-inputs-path at forge time
  → convert bundle into a SeedEpochConsensusInputs-shaped object
  → call the A4 projection
  → claim produce consumes recovered Ade state
```
The distinction is **provenance, not shape**: the type is identical;
what differs is whether the bytes came from a verified bootstrap that
persisted + WAL-proved + replayed them, or from an operator file at forge
time. The second is a disguised operator bundle and is banned; CI must
make it unrepresentable.

**Key line (preserve):** A5 exists because A1–A4 created the
recovered-state *capability*, but **no production mode currently threads
those capabilities together.** The task is to design that production
lifecycle **without smuggling the forge-time operator bundle back in.**

---

## 1. What production paths exist today?

- **`Mode::Produce` → `run_produce_mode`** (`produce_mode.rs:93`) — the
  BA-02 producer. At `produce_mode.rs:184-204` it **cold-starts** from
  operator files:
  - imports UTxO from `--json-seed-path` (`import_cardano_cli_json_utxo`);
  - imports consensus inputs from `--consensus-inputs-path`
    (`import_live_consensus_inputs`) → the bundle;
  - builds `pool_distr_view_from_consensus_inputs(&bundle)` →
    `PoolDistrView` (forge-time);
  - cold-starts `bootstrap_initial_state` with an `InMemoryChainDb` +
    `genesis_initial` (`SeedEpochConsensusSource::NotRequired`);
  - forges via `run_real_forge`.
  It has **no** `FileWalStore`, **no** `PersistentChainDb`, **no**
  `BootstrapAnchor` mint/persistence, **no** sidecar `put`/`get`, **no**
  composer call, **no** warm-start. It never persists → nothing to
  recover. *(Two "anchor" false-positives to pre-empt: the producer
  passes a zero-hash `BootstrapAnchorHash(Hash32([0u8;32]))` placeholder
  to `make_schedule_for_imported_window` (`produce_mode.rs:447`) — a null
  schedule arg, not a real anchor; and it holds a `GenesisAnchor`
  (`coordinator.rs:48`) which is a **forge timing/KES-period** anchor
  (slot-zero time, KES periods), NOT the verified-bootstrap
  `BootstrapAnchor` identity/provenance object the recovered-state
  lifecycle is keyed by. The producer touches neither real
  `BootstrapAnchor`.)*
- **`Mode::Admission` → `run_admission_inner`** (`admission/bootstrap.rs`)
  — has SOME lifecycle infra: `mint(MintInputs{…})` (anchor) +
  `FileWalStore::open` + `run_admission`. But it **still** sources
  consensus inputs from `--consensus-inputs-path` and does **not** call
  the composers or persist the sidecar.
- **`Mode::WireOnly` / `Mode::KeyGenKes`** — not relevant to recovered
  consensus-input production.

## 2. Which lifecycle pieces exist but are disconnected?

- **`PersistentChainDb`** (`chaindb/persistent.rs:85`) — redb-backed,
  implements **both** `ChainDb` (`:195`) and `SnapshotStore` (`:463`).
  Exists; the producer does not use it.
- **Verified-bootstrap composers** (`genesis_bootstrap`,
  `mithril_bootstrap`) — persist the `SeedEpochConsensusInputs` sidecar
  (A2) + append the WAL provenance (A3a). **Only test callers.**
- **A3a WAL provenance** — `WalEntry::SeedEpochConsensusInputsImported`
  + `RecoveredBootstrapProvenance` + replay. Shipped (`c507159`).
- **A3b warm-start verification** —
  `bootstrap_initial_state(RequiredFromRecoveredProvenance)`, fail-closed,
  `BootstrapState`. Shipped (`104982d`). **Not production-wired.**
- **A4 projection** — `PoolDistrView::from_seed_epoch_consensus_inputs`.
  Shipped (`8b60524`). `produce_mode` does not consume it.
- **`recover_node_state`** (`recovery/restart.rs:114`) — WAL-replay
  recovery helper. **Test-only; MUST NOT be used as production
  evidence.**
- **`run_node_until_shutdown`** (`node.rs`) — orchestrator entry.
  **Test-only callers; not production-wired.**

## 3. Target lifecycle required

**First run (verified bootstrap / documented seed extraction):**
```
operator seed (cli/Mithril) → composer (genesis/mithril)
  → mint BootstrapAnchor
  → bootstrap_initial_state over a PERSISTENT SnapshotStore
  → persist SeedEpochConsensusInputs sidecar (A2)
  → append WAL provenance (A3a) — the commit point
```
**Subsequent runs (warm-start recovery):**
```
open PersistentChainDb + FileWalStore
  → replay WAL → RecoveredBootstrapProvenance (A3a)
  → bootstrap_initial_state(RequiredFromRecoveredProvenance) (A3b, fail-closed:
       missing sidecar / missing WAL / hash mismatch / anchor mismatch / duplicate)
  → recovered SeedEpochConsensusInputs
```
**Produce:**
```
recovered selected tip + recovered SeedEpochConsensusInputs
  → PoolDistrView::from_seed_epoch_consensus_inputs (A4) + eta0 from recovered chain_dep → ExpectedVrfInput
  → run_real_forge → block #(tip+1)
  → Haskell peer acceptance = the only BA-02 proof
```
Produce **no longer** uses `--consensus-inputs-path` as bounty-primary
input; any remaining use is diagnostic/fixture-only (§6) and cannot emit
BA-02 evidence.

## 4. Required sub-slices

- **C1 — Choose the production owner of the lifecycle.** Decide whether
  `Mode::Admission` evolves into the bootstrap/sync/produce owner, or a
  new producer lifecycle mode owns `PersistentChainDb` + WAL + anchor +
  recovery. Must respect CN-NODE-01 (one bootstrap authority — do not
  spawn a second). This decision gates C2–C4.
- **C2 — Production bootstrap composition wiring.** Wire the verified-
  bootstrap composers into the chosen production lifecycle. This is where
  sidecar `put` (A2) + WAL provenance append (A3a) become
  **production-real** (their first non-test caller).
- **C3 — Production warm-start recovery.** Open
  `PersistentChainDb`/`SnapshotStore` + `FileWalStore`, replay provenance,
  verify sidecar, call
  `bootstrap_initial_state(RequiredFromRecoveredProvenance)` (A3b).
  Decide the first-run-vs-warm-start branch.
- **C4 — Produce handoff.** Build the forge base (`ForgeBase` /
  `ForgeRequestContext`) from the **recovered selected tip** + recovered
  `SeedEpochConsensusInputs`. **No `InMemoryChainDb` cold-start for
  bounty-primary production.** Swaps the bounty-primary call site
  (`produce_mode.rs:197`) to the A4 projection of the recovered surface.
  **Closes CE-A-4b.**
- **C5 — Diagnostic fence.** Fence `--consensus-inputs-path` and seed-graft
  paths as diagnostic-only: no BA-02 evidence, no sync claim, no
  durability claim. Extend `ci_check_consensus_input_provenance.sh` so the
  producer forge path references the recovered surface only, and so no
  shape-swap populator exists anywhere in the tree (the §0 ban, made
  mechanical).
- **C6 — BA-02 evidence correlation.** A peer-acceptance-only manifest
  correlating the forged block hash ↔ the Haskell peer accept log. The
  only BA-02 success artifact.

## 5. Claims NOT satisfied until this cluster lands

- "The BA-02 producer consumes recovered Ade state" — NOT until C4.
- "Produce no longer depends on `--consensus-inputs-path`" — NOT until C5.
- **CE-A-4b** (bounty-primary call site consumes the recovered surface) —
  assigned here (C4), NOT satisfied by N-F-A A1–A4.
- "Production warm-start restores the seed-epoch inputs" — A3b proved the
  *capability* on the authority surface; the *production path* proof is C3.
- BA-02 itself (Haskell peer accepts an Ade-forged block from recovered
  state) — NOT until C6.

## 6. What remains diagnostic-only

- `--consensus-inputs-path` / `import_live_consensus_inputs` — survive
  **only** as a diagnostic/fixture or first-run *extraction* source
  (verified-bootstrap origin), never as the forge-time bounty-primary
  input. CI-fenced (C5).
- Seed-graft / seed-point paths — diagnostic-only; cannot emit BA-02
  evidence, a sync claim, or a durability claim.
- `recover_node_state`, `run_node_until_shutdown` — remain test/capability
  surfaces; not production evidence unless a slice genuinely wires them
  and says so.

## 7. Hard non-goals

- **No shape-swap** from operator bundle → `SeedEpochConsensusInputs` (§0).
- No direct UTXO-HD / LedgerDB decoder.
- No mainnet Byron→Conway historical replay.
- No cross-epoch production.
- No forged-block durability beyond what N-U provides.
- No claim that this satisfies BA-01, BA-03, BA-04, or BA-09.
- No claim that `run_node_until_shutdown` or `recover_node_state` is
  production-wired unless the code actually wires it.

## 8. Proof obligations

- **Production lifecycle:** the production path proves
  `bootstrap → sidecar persist → WAL provenance → warm-start restore →
  projection → produce` (not a test-only helper).
- **Fail-closed:** missing sidecar / missing WAL / hash mismatch / anchor
  mismatch / duplicate provenance all fail closed on the production path.
- **Replay-equivalence:** produce-from-recovered-state is byte-identical
  under replay for a fixed selected tip, slot, and keys.
- **BA-02:** Haskell peer acceptance is the only BA-02 success proof.
- **Containment:** CI prevents the diagnostic bundle path from emitting
  bounty-primary evidence (no shape-swap populator; forge path consumes
  the recovered surface only).

## 9. Dependency on N-F-A A1–A4 (all committed + tested)

- **A1** (`c13c2e9`): `SeedEpochConsensusInputs` + sole codec.
- **A2** (`f6bf50f`): anchor-keyed sidecar `put/get` + composer population + CI gate.
- **A3a** (`c507159`): WAL provenance entry + `RecoveredBootstrapProvenance` + replay.
- **A3b** (`104982d`): `bootstrap_initial_state` warm-start verification (`RequiredFromRecoveredProvenance`, fail-closed) + `BootstrapState`.
- **A4** (`8b60524`): `PoolDistrView::from_seed_epoch_consensus_inputs` projection (CE-A-4a).
The successor cluster consumes all five; it adds no new BLUE authority,
only the production lifecycle that threads them.

## 10. Immediate next steps (process)

1. **Commit this scoping doc** (standalone, no code). ✔ supersedes the
   first scoping draft.
2. **N-F-A cluster-close decision** — A1–A4 are committed + tested (the
   capability half is complete). Recommend closing N-F-A now as a
   "recovered-state capability cluster": rework cluster.md CE-A-4 →
   **CE-A-4a (done, A4)** / **CE-A-4b (successor, C4)**, regenerate the
   four grounding docs, promote candidate rules
   (CN-CINPUT-01/02, DC-CINPUT-01/02). Decide at `/cluster-close`.
3. **Successor cluster** — run `/invariants` then `/cluster-plan` for
   `PHASE4-N-F-C — Producer Recovered-State Lifecycle`; §0 is its primary
   safety invariant. Per-slice docs C1..C6 before any code.
