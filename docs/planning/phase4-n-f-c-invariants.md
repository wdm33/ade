# PHASE4-N-F-C — Build the real Ade node lifecycle — Invariant Sketch

> **IDD Part I artifact.** Planning only — no code.
> Successor to PHASE4-N-F-A (recovered seed-epoch consensus-input CAPABILITY).
> Normative scoping source: `docs/clusters/completed/PHASE4-N-F-A/A5-SCOPING-producer-recovered-state-lifecycle.md`.
> **Revised 2026-05-30** to align with the post-reset plan: one `--mode node` lifecycle owner,
> Mithril-only first-run bootstrap (fail-closed), and the L1–L6 slice structure. Authority docs:
> `docs/clusters/PHASE4-N-F-C/cluster.md`, `docs/clusters/PHASE4-N-F-C/C1-production-lifecycle-owner.md`
> (the L1 slice), `docs/planning/phase4-n-f-c-cluster-slice-plan.md`.
>
> **What changed from the original sketch (all code-verified):** the owner question is resolved — a
> dedicated `--mode node` owner, NOT an evolved `Mode::Admission` (reading the code disproved the
> premise that admission routes through `bootstrap_initial_state`). L3 BUILDS the production warm-start
> (it does not "wire `recover_node_state`", which passes `NotRequired` and stays test/capability). L4
> INTEGRATES the real peer BlockFetch source (`admission::wire_pump`) with the durable
> `forward_sync::pump_block` apply engine (zero production callers today). "bounty-primary / bounty
> lane" wording replaced by "Ade node lifecycle path".

## Framing: is this a pure `canonical input → canonical output`?

**The authoritative kernel is.** The node-lifecycle forge is a pure transition:

```
(recovered_selected_tip, recovered_SeedEpochConsensusInputs, slot, eta0_from_recovered_chain_dep, vrf_proof, kes_signature)
  → Result<ForgedBlock /* = block #(tip+1) */, ForgeError>
```

**But the cluster as a whole is a lifecycle, not a single transition.** It is a **wiring / composition
cluster**: it threads already-proven BLUE authorities (seed-cinput codec, WAL replay, warm-start verify
chain, the A4 projection, the forge engine) through a single RED production owner (`--mode node`). It
**adds no new BLUE authority** (A5 §9) — every "reuse X" was verified against the actual function bodies.
So most invariants here are *strengthenings of existing rules* and *containment / composition
discipline*, not new BLUE semantics.

The genuine nondeterminism is the **external BA-02 proof** (a Haskell peer accepting the forged block)
and **wall-clock slot timing** — both already RED and already fenced; neither enters the authoritative
core. They are flagged below so they stay out of BLUE.

The forbidden move (A5 §0) — *shape-swapping the operator bundle into a `SeedEpochConsensusInputs`-shaped
object at forge time and calling the projection on it* — is the central illegal state this cluster must
make **unrepresentable**. The distinction is **provenance, not shape**.

**Lifecycle rule (locked):** *Mithril gets Ade started. After bootstrap, WAL + `PersistentChainDb`
explain all Ade state.* Mithril is queried exactly once (first run only); the first-run-vs-warm-start
branch is a pure function of on-disk state.

---

## 1. What must always be true

1. **(Single bootstrap authority — strengthen CN-NODE-01.)** The `--mode node` lifecycle owner reaches
   initial state **only** through the one `bootstrap_initial_state`. First-run and warm-start are two
   branches of that one function; wiring them must not spawn a second bootstrap / storage-init path.
2. **(One lifecycle owner — resolved: `--mode node`.)** Exactly one production lifecycle owner owns the
   path from Mithril-certified first-run / warm-start recovery to produce handoff, without a second
   bootstrap authority. Resolved as a dedicated `--mode node` (NOT an evolved `Mode::Admission`, NOT cold
   `produce_mode`); `produce_mode` and `admission` are demoted to diagnostic. (L1.)
3. **(Mithril-only first run, fail-closed — new.)** No real Mithril provenance ⇒ no first-run bootstrap.
   cardano-cli UTxO / consensus-input extraction is valid **only** when bound to Mithril-certified /
   Mithril-restored state **and** `verify_mithril_binding` succeeds (before any state is admitted). No
   genesis branch, no `--consensus-inputs-path` substitute, no tip-bundle, no cold-`produce_mode`
   substitute. (L2.)
4. **(Producer consumes recovered state — new, CE-A-4b → DC-CINPUT-02b.)** On the node-lifecycle forge
   path, the forge base (`PoolDistrView` + eta0) is derived from the **recovered**
   `SeedEpochConsensusInputs` via `PoolDistrView::from_seed_epoch_consensus_inputs`, where "recovered"
   means: established at Mithril-certified bootstrap → sidecar-persisted → WAL-proven → restored +
   verified on warm-start. (L5.)
5. **(Production lifecycle is real — strengthen DC-CINPUT-01 partial→enforced.)** The full chain
   `Mithril bootstrap → sidecar persist → WAL provenance → warm-start restore → projection → produce` is
   exercised by a **production entry point** (`--mode node`), not a test-only helper. `recover_node_state`
   / `run_node_until_shutdown` stay test / capability surfaces unless a slice genuinely wires them and
   says so. (L3 builds the production warm-start path; it does not adopt `recover_node_state`, which
   passes `NotRequired` and ignores the recovered provenance.)
6. **(Persistence is durable.)** The owner uses `PersistentChainDb` + `FileWalStore` (not
   `InMemoryChainDb`) on the node-lifecycle path: a first run persists something a subsequent run can
   recover. (L1/L2.)
7. **(Recovered artifacts are anchor-bound — strengthen CN-STORE-02 / CN-ANCHOR-01 / DC-ANCHOR-01.)** The
   recovered `SeedEpochConsensusInputs`, the WAL, and the selected tip all bind to exactly one
   `BootstrapAnchor` lineage; the forge base is built from that single lineage.
8. **(Peer-fetch → durable validated apply — new.)** Blocks reach the selected tip via the real peer
   `BlockFetch` source (`admission::wire_pump`) feeding the durable `forward_sync::pump_block` apply
   engine (validated apply → `PersistentChainDb` bytes + WAL `AdmitBlock`, durable-before-tip). No
   "sync-to-tip" is claimed until that fetch→apply integration exists; `ade_core_interop::follow` is NOT
   validating sync. (L4.)
9. **(Forge symmetry holds — carry CN-FORGE-01..04.)** The forged block still round-trips through the
   same BLUE decode / `self_accept` / Praos-VRF authorities; recovered-state sourcing changes the
   *inputs*, never the forge / validate symmetry.
10. **(Replay-equivalence — strengthen T-REC-01 / T-REC-02, DC-WAL-03, DC-FORGE-01, DC-SYNC-01.)**
    Produce-from-recovered-state is byte-identical under replay for a fixed (selected tip, slot, recovered
    inputs, keys); `sync → kill → recover` reaches the same selected tip byte-identically.
11. **(Diagnostic paths are inert for the node lifecycle — new, CN-CINPUT-03.)** `--consensus-inputs-path`
    / `import_live_consensus_inputs` / seed-graft survive only as diagnostic / fixture; they cannot emit
    BA-02 evidence, a sync claim, or a durability claim on the node-lifecycle path.

## 2. What must never be possible

1. **Shape-swap laundering (the §0 ban, made mechanical).** No path may convert an operator bundle
   (`--consensus-inputs-path`) into a `SeedEpochConsensusInputs` at forge time and feed it to the
   projection / forge base. No "shape-swap populator" may exist *anywhere* in the tree.
2. **First-run bootstrap without verified Mithril provenance.** cardano-cli-extracted state used as a
   first-run seed unless it is bound to Mithril-certified / restored state and `verify_mithril_binding`
   succeeds. No genesis / bundle / tip-bundle / cold fallback at first run or on recovery failure.
3. **Forge-time population of the recovered surface** (already fenced populate-side by CN-CINPUT-02; this
   cluster adds the **CONSUME-side fence**, CN-CINPUT-03): the producer forge path may reference the
   recovered surface **only** as a consumer of warm-start output — never construct / put / encode it, and
   only a verified-bootstrap / recovery-authorized lifecycle site may pass
   `SeedEpochConsensusSource::RequiredFromRecoveredProvenance`.
4. **Cold-start as the node-lifecycle path.** No `InMemoryChainDb` cold-start / `LedgerState::new` /
   synthetic forge base on the node-lifecycle path (retires CN-PROD-03's cold-start-only scope for it).
5. **Bundle fallback on recovery failure.** Missing sidecar / missing WAL / hash mismatch / anchor
   mismatch / duplicate provenance must fail closed — never silently fall back to a bundle.
6. **A second bootstrap / recovery / storage-init authority** (CN-NODE-01 violation).
7. **Admission left as only a verdict/comparison engine for the node lifecycle**, or
   `ade_core_interop::follow` treated as validating sync.
8. **BA-02 claimed from anything but a real Haskell-peer accept.** Self-accept, lagging verdicts, or
   "block_received" never count as BA-02.

## 3. What must remain identical across executions (deterministic surface)

- The recovered `SeedEpochConsensusInputs` bytes after `persist → WAL → restore`.
- The `PoolDistrView` + `ExpectedVrfInput` projected from a fixed recovered record (DC-CINPUT-02a).
- The forged block bytes for a fixed (selected tip, slot, recovered inputs, VRF proof, KES signature).
- The first-run-vs-warm-start **branch decision** for a fixed on-disk state (a pure function of what is
  persisted, not wall-clock or env).
- The selected tip reached by `pump_block` over a fixed ordered block sequence (durable-before-tip).

## 4. What must be replay-equivalent

- **(Anchor + WAL → post-state + provenance.)** Replaying `(BootstrapAnchor + WAL)` yields byte-identical
  recovered `SeedEpochConsensusInputs` *and* selected tip (DC-WAL-03 + DC-CINPUT-01 extended to the
  production path).
- **(Sync → kill → recover.)** After `pump_block` advances the tip, a kill + warm-start (L3) recovers the
  same selected tip byte-identically (DC-SYNC-01 + the L4c proof).
- **(Recovered state → forged block.)** Replaying produce over a fixed recovered base + canonical forge
  inputs yields a byte-identical `ForgedBlock` (DC-FORGE-01 family, T-REC-01).
- **(Whole lifecycle.)** Mithril first-run (persist) → warm-start (recover) → sync → produce is
  replay-equivalent to a direct produce from the same canonical inputs — shutdown/resume identity
  extended to the producer.

## 5. State transitions in scope

```
T1 (first run — Mithril-only production bootstrap; L2):
  (empty_persistent_store, Mithril-certified seed {manifest + cardano-cli-extracted UTxO/cinputs
     bound to certified state}, operator seed_point)
    → verify_mithril_binding (fail-closed) → bootstrap_from_mithril_snapshot
    → Result<(persisted_state{PersistentChainDb + FileWalStore, sidecar + WAL provenance,
              one BootstrapAnchor lineage}, BootstrapState), MithrilBootstrapError | BootstrapError>
  — first NON-TEST caller of the sidecar put + WAL-provenance append.

T1′ (branch decision; L1):
  (on_disk_state) → FirstRun | WarmStart      (pure function of persisted state)

T2 (subsequent run — production warm-start recovery; L3, BUILD not wire):
  (PersistentChainDb, FileWalStore)
    → WalStore::read_all → replay_from_anchor → RecoveredBootstrapProvenance
    → bootstrap_initial_state(RequiredFromRecoveredProvenance)
    → Result<BootstrapState{recovered SeedEpochConsensusInputs, recovered tip}, BootstrapError>
  — fail-closed (missing sidecar / WAL / hash / anchor / duplicate); no bundle fallback.

T3 (peer-fetch → durable validated apply; L4 = L4a+L4b+L4c):
  (peer BlockFetch stream via admission::wire_pump)            [L4a: ordered block bytes]
    → pump_block (validated apply, durable-before-tip)          [L4b: PersistentChainDb bytes + WAL AdmitBlock]
    → selected recoverable tip                                  [L4c: kill→recover reaches same tip]

T4 (produce handoff; L5 — closes CE-A-4b):
  (recovered_tip, recovered_SeedEpochConsensusInputs, slot, eta0)
    → ForgeBase / ForgeRequestContext         (via PoolDistrView::from_seed_epoch_consensus_inputs of the RECOVERED surface)

T5 (forge; carries CN-FORGE-01..04):
  (ForgeBase, slot, vrf_proof, kes_sig) → Result<ForgeSucceeded{block #(tip+1)} | ForgeNotLeader | ForgeFailed, _>

T6 (BA-02 evidence correlation; L6 — RED, non-authoritative):
  (forged_block_hash, haskell_peer_accept_log) → BA02Manifest | NoEvidence
```

## 6. TCB color hypothesis

- **BLUE (no new authority — reuse only, verified):** seed-cinput codec, WAL `replay_from_anchor` /
  `ReplayOutcome`, the `restore_seed_epoch_consensus_inputs` verify chain (inside `bootstrap_initial_state`),
  `PoolDistrView::from_seed_epoch_consensus_inputs`, `admit_via_block_validity`, forge / `self_accept` /
  Praos-VRF. **No new BLUE module.**
- **GREEN:** the first-run-vs-warm-start branch decision (pure over on-disk state); the
  `forward_sync::reducer` admit-plan (reused); any lifecycle sequencing reducer.
- **RED:** the `--mode node` owner / driver (opens `PersistentChainDb` / `FileWalStore`, anchor mint/bind,
  the L4a block-fetch source extracted from `admission::wire_pump`, the L4b `pump_block` driver, slot
  loop, peer I/O); the L6 BA-02 evidence correlator; the diagnostic-fence CLI handling.
- **CI (enforcement, not a color):** extend `ci_check_consensus_input_provenance.sh` with the CONSUME-side
  fence + "no shape-swap populator anywhere" (CN-CINPUT-03); a gate asserting the node-lifecycle forge
  base derives from the recovered surface (DC-CINPUT-02b); repaired `ci_check_node_mode_closure.sh` +
  `ci_check_bootstrap_closure.sh` (both stale-RED on `main`); candidate
  `ci_check_lifecycle_owner_uses_bootstrap_initial_state.sh`.

## 7. Resolved decisions + remaining open questions

**Resolved since the original sketch (code-verified):**
- **Owner = dedicated `--mode node`** (NOT evolved `Mode::Admission` — admission does not route through
  `bootstrap_initial_state`; its banner is stale). `produce_mode` / `admission` demoted to diagnostic.
- **First-run source = Mithril only, fail-closed** (genesis/bundle/cold/tip-bundle all forbidden).
- **L3 builds** the production warm-start; `recover_node_state` stays test/capability.
- **L4 integrates** `admission::wire_pump` BlockFetch → `forward_sync::pump_block` (split L4a/L4b/L4c).
- **CN-CINPUT-03 / DC-CINPUT-02b** are the candidate new rules (below), promoted at close.

**Still open (resolve during implementation / close):**
1. **Branch-decision home & color** — GREEN reducer in `ade_runtime` vs. inside the owner with a banner
   (L1 resolves).
2. **CONSUME-side fence shape** — extend `ci_check_consensus_input_provenance.sh` (preferred, per the
   CN-CINPUT-02 open_obligation) vs. a standalone gate (L5/C5).
3. **BA-02 environment** — the live peer-accept run is gated on the Mithril operational prerequisite
   (now met against the existing preprod db) **plus** explicit operator green-light; **no live run
   without it**. C1 private testnet vs C2 preprod docker peer to be chosen at L6.

---

## Proposed registry entries (APPROVED as candidates — NOT yet appended)

The registry uses the project's schema (`id` / `tier` / `statement` / `source` / `cross_ref` /
`code_locus` / `tests` / `ci_script` / `status` / `strengthened_in`). `tests` / `ci_script` /
`code_locus` stay empty until slices implement enforcement; both rules remain `status = "declared"`
until their wiring slice lands (DC-CINPUT-02b until L5 wires the producer; CN-CINPUT-03 until L5 extends
the containment gate). **Append happens at cluster scope / close, not in `/invariants`.**

**`DC-CINPUT-02b` (producer consumption — CE-A-4b) — approved, declared:**
```toml
[[rules]]
id = "DC-CINPUT-02b"
tier = "derived"
statement = """
Producer consumption of the recovered surface: on the Ade node-lifecycle forge
path, the forge base (PoolDistrView + eta0) is derived ONLY from a recovered
SeedEpochConsensusInputs — established at a Mithril-certified bootstrap,
sidecar-persisted, WAL-proven, and restored+verified on warm-start — projected
via PoolDistrView::from_seed_epoch_consensus_inputs. No forge-time operator
bundle (--consensus-inputs-path) may be shape-swapped into the recovered type
and fed to the projection. Provenance, not shape.
"""
source = "docs/clusters/PHASE4-N-F-C/cluster.md (L5); docs/clusters/completed/PHASE4-N-F-A/A5-SCOPING-producer-recovered-state-lifecycle.md §0,§4"
cross_ref = ["CN-CINPUT-02", "DC-CINPUT-01", "DC-CINPUT-02a", "CN-PROD-03", "CN-NODE-01"]
code_locus = ""
tests = []
ci_script = ""
status = "declared"
strengthened_in = []
```

**`CN-CINPUT-03` (CONSUME-side fence + no shape-swap populator) — approved, declared:**
```toml
[[rules]]
id = "CN-CINPUT-03"
tier = "constraint"
statement = """
CONSUME-side containment: the producer forge path references the recovered
SeedEpochConsensusInputs ONLY as a warm-start consumer — it never constructs,
encodes, or puts the sidecar, and only a verified-bootstrap/recovery-authorized
lifecycle site may pass SeedEpochConsensusSource::RequiredFromRecoveredProvenance.
No shape-swap populator (operator bundle -> SeedEpochConsensusInputs) exists
anywhere in the tree. Diagnostic --consensus-inputs-path / seed-graft paths emit
no BA-02 evidence. Enforced by extending the data-flow-resistant containment gate.
"""
source = "docs/clusters/PHASE4-N-F-C/cluster.md (L5); docs/clusters/completed/PHASE4-N-F-A/A5-SCOPING-producer-recovered-state-lifecycle.md §4,§0"
cross_ref = ["CN-CINPUT-02", "DC-CINPUT-02b", "CN-NODE-01"]
code_locus = ""
tests = []
ci_script = ""
status = "declared"
strengthened_in = []
```

**Strengthenings at close (no new IDs — `strengthened_in += "PHASE4-N-F-C"`):** `DC-CINPUT-01`
(partial→enforced once the production warm-start lands), `DC-CINPUT-02a` (consumption now exercised),
`CN-NODE-01`, `CN-PROD-03` (cold-start scope retired for the node-lifecycle path), `CN-STORE-02`,
`CN-ANCHOR-01`, `DC-ANCHOR-01`, `T-REC-01`, `T-REC-02`, `DC-WAL-03`, `DC-FORGE-01`, `DC-SYNC-01` (L4b
first production driver of `pump_block`). A BA-02 evidence-manifest obligation (L6) is expected to
strengthen the existing `RO-LIVE-01` / `RO-LIVE-05` family rather than mint a new RO — resolve which at
close.
