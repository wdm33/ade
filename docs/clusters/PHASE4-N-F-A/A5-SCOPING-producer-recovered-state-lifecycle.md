# PHASE4-N-F-A / A5 — SCOPING (not a slice): Producer Recovered-State Lifecycle

> **This is a SCOPING document, not a slice doc.** Grounding A5 against
> the real producer code (2026-05-30) showed A5 is **not** a slice-sized
> call-site swap — it is a **successor lifecycle cluster**. This doc
> classifies that successor, enumerates its sub-slices and open
> architectural questions, and fixes the **core safety line** so no
> future slice crosses it. **No production-path code is written until a
> proper cluster plan + per-slice docs are reviewed.**
>
> **Proposed successor cluster name: `PHASE4-N-F-C — Producer
> Recovered-State Lifecycle`** (A5 is retired as a slice id; its intent
> becomes this cluster). If the project prefers strict top-level
> sequence, `PHASE4-N-G` is acceptable — naming is an open question (§7).

## 1. Why A5 is not a slice — the grounding finding

The cluster doc (cluster.md §5) assumed: *"node startup runs
`bootstrap_initial_state` (`node.rs:145`), whose warm-start branch is the
real restart path."* Grounding against code breaks this in three ways:

1. **`node.rs:145` (`run_node_until_shutdown`) is test-only** — all
   callers are `#[cfg(test)]` (established in A3b). Not production-wired.
2. **The bounty-primary producer (`Mode::Produce` → `run_produce_mode`,
   `produce_mode.rs:93`) has ZERO recovered-state infrastructure.** At
   `produce_mode.rs:184-204` it: imports UTxO from `--json-seed-path`,
   imports consensus inputs from `--consensus-inputs-path` (the bundle),
   builds `pool_distr_view_from_consensus_inputs(&bundle)`, and
   **cold-starts** `bootstrap_initial_state` with an `InMemoryChainDb` +
   `genesis_initial` (`SeedEpochConsensusSource::NotRequired`). It has
   **no** `FileWalStore`, **no** persistent `SnapshotStore`, **no**
   `BootstrapAnchor` mint, **no** sidecar `put`/`get`, **no** composer
   call, **no** warm-start. It never persists, so there is nothing to
   recover.
3. **The verified-bootstrap composers** (`genesis_bootstrap`,
   `mithril_bootstrap` — where A2 persists the sidecar and A3a appends
   the WAL provenance) have **only test callers**. They are not wired to
   any production mode.

So the N-F-A pieces (A2 persist → A3a WAL → A3b warm-start recover → A4
project) are individually proven but **live in disconnected components
that no production mode threads together.** Making the producer consume
recovered `SeedEpochConsensusInputs` requires building that entire
lifecycle into (or beside) the producer — the size of N-M-A / N-Q, a
cluster, not a slice.

## 2. THE CORE SAFETY LINE (binding on every successor slice)

The successor cluster MUST preserve this provenance chain:

```
verified bootstrap (composer: genesis/mithril)
  → persisted sidecar (A2, anchor-keyed SnapshotStore)
  → WAL provenance entry (A3a)
  → warm-start recovery (A3b, RequiredFromRecoveredProvenance, fail-closed)
  → recovered SeedEpochConsensusInputs
  → projection (A4, PoolDistrView / ExpectedVrfInput)
  → produce / leader check
```

It MUST NEVER become:

```
forge-time --consensus-inputs-path
  → a SeedEpochConsensusInputs-shaped object
  → produce
```

**The second is semantic laundering** — a disguised operator bundle
wearing recovered-state clothes. The first input is *Ade's own audited,
persisted, recovered state*; the second is the operator-bundle model
N-F-A was built to eliminate. A "shape swap" (convert the forge-time
bundle into a `SeedEpochConsensusInputs` in-process and call it
recovered) is **explicitly forbidden** and is the single thing the
successor's CI containment must make unrepresentable. The distinction is
**provenance, not shape**: the type is identical; what differs is whether
the bytes came from a verified bootstrap that persisted + replayed them,
or from an operator file at forge time.

## 3. Current path vs. required target path

**Current (`run_produce_mode`, cold-start):**
```
--json-seed-path ──► import_cardano_cli_json_utxo ──► seed_ledger
--consensus-inputs-path ──► import_live_consensus_inputs ──► bundle
bundle ──► pool_distr_view_from_consensus_inputs ──► PoolDistrView (forge-time)
(seed_ledger, genesis_chain_dep) ──► bootstrap_initial_state(cold, InMemoryChainDb, NotRequired)
PoolDistrView + base state ──► run_real_forge
```

**Required target (recovered-state lifecycle):**
```
FIRST RUN (verified bootstrap):
  operator seed (cli/Mithril) ──► composer (genesis/mithril)
    ──► mint BootstrapAnchor
    ──► bootstrap_initial_state (persistent SnapshotStore)
    ──► persist SeedEpochConsensusInputs sidecar (A2)
    ──► append WAL provenance (A3a)
SUBSEQUENT RUNS (warm-start):
  open persistent chaindb + FileWalStore
    ──► replay WAL ──► RecoveredBootstrapProvenance (A3a)
    ──► bootstrap_initial_state(RequiredFromRecoveredProvenance) (A3b, fail-closed)
    ──► recovered SeedEpochConsensusInputs
    ──► PoolDistrView::from_seed_epoch_consensus_inputs (A4)
    ──► run_real_forge
```

The gap between these is the successor cluster's whole job.

## 4. Required sub-slices (provisional — the cluster plan refines)

- **C-1 — Producer persistent storage.** Replace the producer's
  `InMemoryChainDb` cold-start with a `PersistentChainDb`
  (`chaindb/persistent.rs:85`, already impls `ChainDb` + `SnapshotStore`)
  rooted at a producer data dir; open-or-create semantics. RED.
- **C-2 — Producer verified-bootstrap entry (first run).** Route the
  producer's first-run startup through the verified-bootstrap composer
  (`genesis_bootstrap` / `mithril_bootstrap`) so it mints the anchor,
  persists the sidecar (A2), and appends the WAL provenance (A3a) — i.e.
  give the composers their first *production* caller. RED composition.
  *(Open: does produce reuse admission's bootstrap, or get its own?)*
- **C-3 — Producer warm-start (subsequent runs).** On restart, open the
  persistent store + `FileWalStore`, replay to a
  `RecoveredBootstrapProvenance`, and call
  `bootstrap_initial_state(RequiredFromRecoveredProvenance)` (A3b),
  fail-closed. Decide the first-run-vs-warm-start branch.
- **C-4 — Producer consumes the recovered projection (CE-A-4b).** Swap
  the bounty-primary call site (`produce_mode.rs:197`,
  `pool_distr_view_from_consensus_inputs`) for
  `PoolDistrView::from_seed_epoch_consensus_inputs(&recovered)` (A4) +
  eta0 from recovered `chain_dep`. This is the line that finally makes
  produce consume recovered state — and only lands AFTER C-1..C-3 give it
  a genuine recovered surface. **Closes CE-A-4b.**
- **C-5 — Containment + fence.** Extend
  `ci_check_consensus_input_provenance.sh` so the producer's forge path
  references the recovered surface, never `--consensus-inputs-path` /
  `import_live_consensus_inputs` / `pool_distr_view_from_consensus_inputs`
  at forge time; and so no "shape-swap" populator exists (the §2 ban,
  made mechanical). The `--consensus-inputs-path` flag survives only for
  the diagnostic/bootstrap-extraction path, fenced.
- **C-6 — Operator-pass evidence.** A produce run from a persistent,
  warm-started store that forges using recovered consensus inputs, with
  a committed transcript — the real end-to-end proof.

## 5. Open architecture questions (resolve in the cluster plan, not here)

1. **Produce vs. Admission convergence.** Admission
   (`admission/bootstrap.rs`) already has a `FileWalStore` + anchor mint
   but no composer/sidecar; produce has neither. Do produce and admission
   converge on one shared "open persistent store + bootstrap-or-recover"
   entry, or stay separate? (CN-NODE-01 says one bootstrap authority —
   this should not spawn a second.)
2. **First-run bootstrap source.** Does the producer bootstrap from
   genesis (`bootstrap_from_conway_genesis`) or Mithril
   (`bootstrap_from_mithril_snapshot`)? Both? CLI-selected?
3. **Where consensus inputs come from at FIRST run.** The composers take
   `seed_consensus_inputs: &LiveConsensusInputsCanonical` — today that is
   the operator extraction. Persisting it at first run is fine
   (verified-bootstrap origin, per A2/cluster §2); the ban is on
   *forge-time* re-import. The cluster plan must state precisely where
   first-run extraction is allowed and how it differs from forge-time.
4. **Data-dir / CLI surface.** New `--data-dir` (persistent root)? How
   does `--consensus-inputs-path` get demoted to first-run-only /
   diagnostic?
5. **Cluster id + sequence** (§7).

## 6. Non-goals (of the successor cluster)

- No new consensus-input *type* or codec (A1 is sole).
- No second bootstrap authority (CN-NODE-01; reuse `bootstrap_initial_state`).
- No stake computation / epoch rotation (persists already-extracted seed-epoch constants).
- No new VRF / KES / forge logic (reuse `run_real_forge`, `leader_vrf_input`).
- **No shape-swap of the forge-time bundle into recovered state (§2).**

## 7. Proof obligations

- **Replay-equivalence:** first-run bootstrap + persist + restart-warm-start
  recovers the SAME `SeedEpochConsensusInputs` byte-identically (extends
  A3b's authority-surface proof to the production producer path).
- **Provenance containment:** CI proves the producer forge path consumes
  ONLY the recovered surface; no forge-time bundle; no shape-swap
  populator anywhere in the tree.
- **CE-A-4b:** the bounty-primary produce call site consumes the recovered
  surface (the clause A4 deferred).
- **Operator-pass:** a committed produce transcript from a warm-started
  persistent store forging on recovered inputs.

## 8. Dependency on N-F-A A1–A4 (all committed)

- **A1** (`8b60524` lineage, `c13c2e9`): `SeedEpochConsensusInputs` + sole codec.
- **A2** (`f6bf50f`): anchor-keyed sidecar `put/get` + composer population + CI gate.
- **A3a** (`c507159`): WAL provenance entry + `RecoveredBootstrapProvenance` + replay.
- **A3b** (`104982d`): `bootstrap_initial_state` warm-start verification (`RequiredFromRecoveredProvenance`, fail-closed) + `BootstrapState`.
- **A4** (`8b60524`): `PoolDistrView::from_seed_epoch_consensus_inputs` projection (CE-A-4a).
The successor cluster consumes all five; it adds no new BLUE authority, only the production lifecycle that threads them.

## 9. Immediate next steps (process)

1. **Commit this scoping doc** (done standalone, no code).
2. **N-F-A cluster-close decision:** A1–A4 are committed + tested; the
   cluster's capability half is complete. Either (a) close N-F-A now as a
   "recovered-state capability cluster" with CE-A-4 reworded to
   CE-A-4a(done)/CE-A-4b(successor) and the successor cluster recorded, or
   (b) keep N-F-A open until the successor lands. **Recommend (a)** — the
   capability is done and proven; the lifecycle is genuinely a separate
   concern with its own invariants. Decide at `/cluster-close`.
3. **Successor cluster:** run `/invariants` then `/cluster-plan` for
   `PHASE4-N-F-C — Producer Recovered-State Lifecycle` (§2 is its primary
   safety invariant). Per-slice docs (C-1..C-6) before any code.
