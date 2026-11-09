# Cluster/Slice Plan — Ade · PHASE4-N-F-C (Build the real Ade node lifecycle)

> **REVISED (code-verified 2026-05-30).** This replaces the earlier C1–C6 / "evolve `Mode::Admission`
> (Option A)" plan, which rested on a false premise (admission does not route through
> `bootstrap_initial_state`). Owner decision resolved: a dedicated **`--mode node`** lifecycle owner.
> Authority doc: `docs/clusters/PHASE4-N-F-C/cluster.md`. L1 slice: `docs/clusters/PHASE4-N-F-C/C1-production-lifecycle-owner.md`.
> Invariant sketch (still valid): `docs/planning/phase4-n-f-c-invariants.md`. Does not replace the
> overarching Phase-4 plan at `docs/active/phase_4_cluster_plan.md`.

## Cluster Index

1. `PHASE4-N-F-C` — Build the real Ade node lifecycle — primary invariant: the Ade node lifecycle forges
   from Ade-owned recovered state (Mithril-certified first-run → persist → WAL-proven → warm-start
   restored → produce), never a forge-time operator bundle; one `--mode node` owner, no second bootstrap
   authority.

## The one lifecycle

```
FIRST RUN (empty store):  Mithril-certified bootstrap ONLY → verify_mithril_binding (fail-closed)
                            → bootstrap_from_mithril_snapshot → PersistentChainDb + FileWalStore
                            → one BootstrapAnchor lineage → sidecar + WAL provenance
WARM START (non-empty):   WAL replay → RecoveredBootstrapProvenance
                            → bootstrap_initial_state(RequiredFromRecoveredProvenance) → recovered state
BOTH →                    peer BlockFetch → validated durable apply → selected recoverable tip
                            → forge base = recovered tip + PoolDistrView::from_seed_epoch_consensus_inputs
                            → run_real_forge → success ONLY on real Haskell-peer acceptance
```

**Mithril gets Ade started. After bootstrap, WAL + PersistentChainDb explain all Ade state.**

## Locked rules
- **Mithril-only first run, fail-closed.** No real Mithril provenance ⇒ no first-run bootstrap. cardano-cli
  extraction is valid only when bound to Mithril-certified/restored state AND `verify_mithril_binding`
  succeeds. No genesis / `--consensus-inputs-path` / tip-bundle / cold-`produce_mode` fallback.
- Mithril = provenance (not byte source); native UTXO-HD decode is a Tier-4 non-goal.
- Mithril queried once (first run); WAL + persisted state are sole authority after the anchor.
- No shape-swap; no second bootstrap authority; no BA-02 claim without a real peer accept.
- Scope: single-epoch, single-shot until forged-block durability (N-U).

## Slices (safety order)

- **L1 — Lifecycle owner skeleton + branch (`--mode node`)** — owner over `PersistentChainDb` +
  `FileWalStore`; first-run-vs-warm-start = pure fn of on-disk state; both arms route through
  `bootstrap_initial_state`; repair the two stale CN-NODE-01 gates — addresses CE-L-1 — TCB: RED owner +
  GREEN branch + CI. *Hermetic.*
- **L2 — Mithril-only first-run bootstrap (live)** — `bootstrap_from_mithril_snapshot` first production
  caller; `verify_mithril_binding` before admit; fail-closed; sidecar + WAL provenance; one anchor lineage
  — addresses CE-L-2 — TCB: RED driver + GREEN merge (reused) + BLUE codec (reused). *Live leg gated on the
  Mithril operational prerequisite; hermetic fixture proves fail-closed wiring.*
- **L3 — Production warm-start recovery (BUILD, not wire)** — `replay_from_anchor` →
  `RequiredFromRecoveredProvenance` → recovered `BootstrapState`, fail-closed, no fallback;
  `recover_node_state` stays test/capability unless explicitly changed — addresses CE-L-3 (flips
  DC-CINPUT-01 → enforced) — TCB: RED store/WAL open + BLUE replay/verify (reused) + new production wiring.
  *Hermetic.*
- **L4 — Peer BlockFetch → durable validated apply (SPLIT; not hidden under "sync")** — addresses CE-L-4:
  - **L4a** — block-fetch source abstraction extracted from `admission::wire_pump` (N2N
    `RequestRange`→`Block{bytes}`), decoupled from the verdict loop — TCB: RED.
  - **L4b** — feed L4a blocks into `forward_sync::pump_block` (first production caller): validated apply →
    `PersistentChainDb` bytes + WAL `AdmitBlock`, durable-before-tip — TCB: RED driver + GREEN reducer
    (reused) + BLUE admit (reused).
  - **L4c** — selected-tip recovery proof: kill after L4b advances, warm-start (L3) recovers the same tip
    byte-identically — TCB: hermetic crash/restart test.
- **L5 — Produce from recovered selected tip + recovered consensus inputs** — forge base from recovered
  tip (L4c) + recovered `SeedEpochConsensusInputs` (L3) via the A4 projection; reuse `run_real_forge`;
  delete the cold forge base from the node lifecycle path; CN-CINPUT-03 + DC-CINPUT-02b — addresses CE-L-5
  — TCB: RED handoff + BLUE A4/forge (reused) + CI. *Hermetic.*
- **L6 — Peer-acceptance evidence** — closed manifest (forged-block-hash ↔ peer accept log); synthetic
  dry-run mechanical; live accept gated — addresses CE-L-6 — TCB: RED correlator. *Live leg gated.*

## Plan-level forbidden moves
1. First-run bootstrap without verified Mithril provenance.
2. cardano-cli-extracted first-run seed not bound to Mithril-certified/restored state.
3. Genesis / `--consensus-inputs-path` / tip-bundle / cold fallback at first-run or recovery.
4. A second bootstrap/recovery/storage-init authority (CN-NODE-01).
5. Shape-swap of an operator bundle into `SeedEpochConsensusInputs`.
6. Leaving `admission` as only a verdict engine for the node lifecycle, or treating
   `ade_core_interop::follow` as validating sync.
7. Claiming "sync-to-tip" before L4a+L4b+L4c connect; claiming BA-02 before a real peer accept.

## Replay obligations (scoped)
No new canonical type / authoritative transition. New scoped fixtures: L3 `persist→recover`; L4c
`sync→kill→recover`; L5 `produce-from-recovered`. Acceptance scoped to `ade_runtime`, `ade_node`, and the
specific `ade_testkit` harness tests — NOT the full corpus lane (times out ~600s on clean HEAD).

## Operational prerequisite (live L2 / L6)
Required, not optional: a real Mithril-certified preprod artifact (`mithril-client` certified download →
peer restore → cardano-cli extraction, genesis-key-verified). Hermetic slices proceed without it; the live
L2 binding + L6 accept do not. See cluster.md "Operational prerequisite".

## Registry impact (at close)
Promote `DC-CINPUT-02b` (L5) + `CN-CINPUT-03` (L5). Strengthen `CN-NODE-01`, `CN-PROD-03`, `DC-CINPUT-01`
(→enforced), `DC-CINPUT-02a`, `CN-STORE-02`, `T-REC-01/02`, `DC-WAL-03`, `DC-FORGE-01`, `DC-SYNC-01`.
BA-02 stays an operator-gated RO-LIVE obligation.
