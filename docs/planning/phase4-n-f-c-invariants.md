# PHASE4-N-F-C — Producer Recovered-State Lifecycle — Invariant Sketch

> **IDD Part I artifact.** Planning only — no clusters, no slices, no code.
> Successor to PHASE4-N-F-A (recovered seed-epoch consensus-input CAPABILITY).
> Normative scoping source: `docs/clusters/completed/PHASE4-N-F-A/A5-SCOPING-producer-recovered-state-lifecycle.md`.
> Status: **confirmed** (2026-05-30). Next step: `/cluster-plan PHASE4-N-F-C` with **C1 production owner as the first gate**.

## Framing: is this a pure `canonical input → canonical output`?

**The authoritative kernel is.** The bounty-primary forge is a pure transition:

```
(recovered_selected_tip, recovered_SeedEpochConsensusInputs, slot, eta0_from_recovered_chain_dep, vrf_proof, kes_signature)
  → Result<ForgedBlock /* = block #(tip+1) */, ForgeError>
```

**But the cluster as a whole is a lifecycle, not a single transition** — exactly what the A5 scoping
doc concluded. It is a **wiring / composition cluster**: it threads already-proven BLUE authorities
(A1 codec, A3a replay, A3b warm-start verify, A4 projection) through a RED production driver. Per A5
§9 it **adds no new BLUE authority**. So most invariants here are *strengthenings of existing rules*
and *containment / composition discipline*, not new BLUE semantics.

The one genuine nondeterminism is the **external BA-02 proof** (a Haskell peer accepting the forged
block) and **wall-clock slot timing** — both already RED and already fenced; neither enters the
authoritative core. They are flagged explicitly below so they stay out of BLUE.

The forbidden move (A5 §0) — *shape-swapping the operator bundle into a `SeedEpochConsensusInputs`-shaped
object at forge time and calling A4 on it* — is the central illegal state this cluster must make
**unrepresentable**. The distinction is **provenance, not shape**.

---

## 1. What must always be true

1. **(Single bootstrap authority — strengthen CN-NODE-01.)** The chosen production lifecycle owner
   reaches initial state **only** through the one `bootstrap_initial_state`. Wiring a production
   warm-start must not spawn a second bootstrap / storage-init path. Cold-start (first run) and
   warm-start (recovery) remain two branches of the one function.
2. **(One lifecycle owner — C1 requirement, NOT resolved here.)** Exactly one production lifecycle
   owner must own the path from verified bootstrap / recovery to produce handoff, **without creating
   a second bootstrap authority**. *Which* mode owns it is the first cluster-doc / cluster-plan
   decision (see Open Questions §7.1); the invariant states only the requirement.
3. **(Producer consumes recovered state — new, CE-A-4b → DC-CINPUT-02b.)** On the bounty-primary
   forge path, the producer's forge base (`PoolDistrView` + eta0) is derived from the **recovered**
   `SeedEpochConsensusInputs` via `PoolDistrView::from_seed_epoch_consensus_inputs` (A4), where
   "recovered" means: established at verified bootstrap → sidecar-persisted (A2) → WAL-proven (A3a)
   → restored + verified on warm-start (A3b).
4. **(Production lifecycle is real — strengthen DC-CINPUT-01 partial→enforced.)** The full chain
   `bootstrap → sidecar persist → WAL provenance → warm-start restore → projection → produce` is
   exercised by a **production entry point**, not a test-only helper (`recover_node_state` /
   `run_node_until_shutdown` stay test / capability surfaces unless a slice genuinely wires them and
   says so).
5. **(Persistence is durable.)** The production owner uses `PersistentChainDb` + `FileWalStore` (not
   `InMemoryChainDb`) for the bounty-primary path: a first run persists something a subsequent run
   can recover.
6. **(Recovered artifacts are anchor-bound — strengthen CN-STORE-02 / CN-ANCHOR-01 / DC-ANCHOR-01.)**
   The recovered `SeedEpochConsensusInputs`, the WAL, and the selected tip all bind to exactly one
   `BootstrapAnchor` lineage; the forge base is built from that single lineage.
7. **(Forge symmetry holds — carry CN-FORGE-01..04.)** The forged block still round-trips through the
   same BLUE decode / `self_accept` / Praos-VRF authorities; recovered-state sourcing changes the
   *inputs*, never the forge / validate symmetry.
8. **(Replay-equivalence — strengthen T-REC-01 / T-REC-02, DC-WAL-03, DC-FORGE-01.)** Produce-from-
   recovered-state is byte-identical under replay for a fixed (selected tip, slot, recovered inputs,
   keys).
9. **(Diagnostic paths are inert for the bounty — new, CN-CINPUT-03.)** `--consensus-inputs-path` /
   `import_live_consensus_inputs` / seed-graft survive only as diagnostic / fixture / first-run-
   extraction; they cannot emit BA-02 evidence, a sync claim, or a durability claim.

## 2. What must never be possible

1. **Shape-swap laundering (the §0 ban, made mechanical).** No path may convert an operator bundle
   (`--consensus-inputs-path`) into a `SeedEpochConsensusInputs` at forge time and feed it to the A4
   projection / forge base. No "shape-swap populator" may exist *anywhere* in the tree.
2. **Forge-time population of the recovered surface** (already fenced by CN-CINPUT-02 populate-side;
   this cluster adds the **CONSUME-side fence**, CN-CINPUT-03): the producer forge path may reference
   the recovered surface **only** as a consumer of warm-start output — never construct / put / encode
   it, and only a verified-bootstrap / recovery-authorized lifecycle site may pass
   `RequiredFromRecoveredProvenance`.
3. **Cold-start as bounty-primary.** No `InMemoryChainDb` cold-start / `LedgerState::new` / synthetic
   forge base on the bounty-primary production path (retires CN-PROD-03's cold-start-only scope for
   the bounty path).
4. **Bundle fallback on recovery failure.** Missing sidecar / missing WAL / hash mismatch / anchor
   mismatch / duplicate provenance must fail closed — never silently fall back to the forge-time
   bundle.
5. **A second bootstrap / recovery / storage-init authority** (CN-NODE-01 violation).
6. **BA-02 claimed from anything but a real Haskell-peer accept.** Self-accept, lagging verdicts, or
   "block_received" never count as BA-02.

## 3. What must remain identical across executions (deterministic surface)

- The recovered `SeedEpochConsensusInputs` bytes after `persist → WAL → restore` (already A1 / A3a /
  A3b).
- The `PoolDistrView` + `ExpectedVrfInput` projected from a fixed recovered record (A4 /
  DC-CINPUT-02a).
- The forged block bytes for a fixed (selected tip, slot, recovered inputs, VRF proof, KES
  signature).
- The first-run-vs-warm-start **branch decision** for a fixed on-disk state (must be a pure function
  of what's persisted, not wall-clock or env).

## 4. What must be replay-equivalent

- **(Anchor + WAL → post-state + provenance.)** Replaying `(BootstrapAnchor + WAL)` yields
  byte-identical recovered `SeedEpochConsensusInputs` *and* selected tip (DC-WAL-03 + DC-CINPUT-01
  extended to the production path).
- **(Recovered state → forged block.)** Replaying produce over a fixed recovered base + canonical
  forge inputs yields a byte-identical `ForgedBlock` (DC-FORGE-01 family, T-REC-01).
- **(Whole lifecycle.)** First run (persist) followed by warm-start (recover) followed by produce is
  replay-equivalent to a direct produce from the same canonical inputs — the "shutdown / resume
  identity" property extended to the producer.

## 5. State transitions in scope

```
T1 (first run — production bootstrap composition; C2):
  (empty_persistent_store, operator_seed /*cli|mithril*/, anchor)
    → Result<(persisted_state{SnapshotStore+WAL with sidecar+provenance}, BootstrapState), BootstrapError>
  — first NON-TEST caller of the A2 put + A3a append.

T2 (subsequent run — production warm-start recovery; C3):
  (persistent_store, file_wal)
    → Result<(BootstrapState{recovered SeedEpochConsensusInputs, recovered tip}, effects), BootstrapError | NodeRecoveryError>
  — RequiredFromRecoveredProvenance, fail-closed, no bundle fallback.

T2′ (branch decision; C3):
  (on_disk_state) → FirstRun | WarmStart      (pure function of persisted state)

T3 (produce handoff; C4 — closes CE-A-4b):
  (recovered_tip, recovered_SeedEpochConsensusInputs, slot, eta0)
    → ForgeBase / ForgeRequestContext         (via A4 projection of the RECOVERED surface)

T4 (forge; carries CN-FORGE-01..04):
  (ForgeBase, slot, vrf_proof, kes_sig) → Result<ForgeSucceeded{block #(tip+1)} | ForgeNotLeader | ForgeFailed, _>

T5 (BA-02 evidence correlation; C6 — RED, non-authoritative):
  (forged_block_hash, haskell_peer_accept_log) → BA02Manifest | NoEvidence
```

## 6. TCB color hypothesis

- **BLUE (no new authority — reuse only):** A1 codec, A3a replay / `ReplayOutcome`, A3b verify chain,
  A4 projection, forge / `self_accept` / Praos-VRF. **No new BLUE module expected.**
- **GREEN:** the first-run-vs-warm-start branch decision (pure over persisted state); any lifecycle
  plan / reducer that sequences existing authorities (mirroring the forward-sync GREEN reducer
  pattern). **Open:** whether the branch logic is GREEN-in-`ade_runtime` or absorbed into
  `bootstrap_initial_state`.
- **RED:** the production lifecycle owner / driver (opens `PersistentChainDb` / `FileWalStore`, slot
  loop, peer I/O), the C6 BA-02 evidence correlator, the diagnostic-fence CLI handling.
- **CI (enforcement, not a color):** extend `ci_check_consensus_input_provenance.sh` with the
  CONSUME-side fence + "no shape-swap populator anywhere" (CN-CINPUT-03); a new gate (or extension)
  asserting the bounty-primary forge base derives from the recovered surface (DC-CINPUT-02b).

**Open color question:** C1 — *which mode owns the lifecycle* — determines where the RED driver lives
(see Open Questions §7.1).

## 7. Open questions (must resolve before / during `/cluster-plan`)

1. **C1 — production owner (the gating decision; first cluster-doc gate).** Does `Mode::Admission`
   (which already has `mint` + `FileWalStore` + `run_admission`) evolve into the bootstrap / sync /
   produce owner, or does a **new producer lifecycle mode** own `PersistentChainDb` + WAL + anchor +
   recovery + produce handoff? Must respect CN-NODE-01 (no second bootstrap authority). **This
   decision gates C2–C4 and the RED driver's home, and must be made with a call graph and a sub-slice
   plan — not pre-resolved here.** *(Author's later lean, non-binding: do not bolt this into the
   existing cold `produce_mode` as a patch.)*
2. **Branch-decision home & color** (GREEN reducer vs. inside `bootstrap_initial_state`) — see §6.
3. **CONSUME-side fence shape:** the approved CN-CINPUT-03 candidate captures it; resolve at
   cluster-plan whether C5 extends `ci_check_consensus_input_provenance.sh` (per A5 §4 C5 and the
   CN-CINPUT-02 open_obligation) vs. a standalone gate.
4. **Producer-consumption rule:** the approved DC-CINPUT-02b candidate captures CE-A-4b (HEAD_DELTAS
   §0 explicitly deferred this to N-F-C).
5. **Does C3 actually wire `recover_node_state` / `run_node_until_shutdown`,** or build a fresh
   production entry? (Must not claim either is production-wired unless code does it — A5 §7.)
6. **BA-02 environment:** C1 private testnet vs. C2 preprod docker peer for the actual peer-accept run
   (cross-refs the operator-pass C1 scoping — a thin wiring cluster is owed before any live run; **no
   live run without user green-light**).

---

## Proposed registry entries (APPROVED as candidates — NOT yet appended)

The registry uses the project's schema (`id` / `tier` / `statement` / `source` / `cross_ref` /
`code_locus` / `tests` / `ci_script` / `status` / `strengthened_in`). `tests` / `ci_script` /
`code_locus` stay empty until slices implement enforcement; both rules remain `status = "declared"`
until their wiring slice lands (DC-CINPUT-02b until C4 wires the producer; CN-CINPUT-03 until C5
extends the containment gate). **Append happens at cluster scope / close, not in `/invariants`.**

**`DC-CINPUT-02b` (producer consumption — CE-A-4b) — approved, declared:**
```toml
[[rules]]
id = "DC-CINPUT-02b"
tier = "derived"
statement = """
Producer consumption of the recovered surface: on the bounty-primary forge
path, produce-mode derives its forge base (PoolDistrView + eta0) ONLY from a
recovered SeedEpochConsensusInputs — established at verified bootstrap,
sidecar-persisted, WAL-proven, and restored+verified on warm-start — projected
via PoolDistrView::from_seed_epoch_consensus_inputs. No forge-time operator
bundle (--consensus-inputs-path) may be shape-swapped into the recovered type
and fed to the projection. Provenance, not shape.
"""
source = "docs/clusters/completed/PHASE4-N-F-A/A5-SCOPING-producer-recovered-state-lifecycle.md §0,§4(C4)"
cross_ref = ["CN-CINPUT-02", "DC-CINPUT-01", "DC-CINPUT-02a", "CN-PROD-03", "CN-NODE-01"]
code_locus = ""
tests = []
ci_script = ""
status = "declared"
strengthened_in = []
```

**`CN-CINPUT-03` (CONSUME-side fence + no shape-swap populator) — approved, declared (with wording tweak applied):**
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
source = "docs/clusters/completed/PHASE4-N-F-A/A5-SCOPING-producer-recovered-state-lifecycle.md §4(C5),§0"
cross_ref = ["CN-CINPUT-02", "DC-CINPUT-02b", "CN-NODE-01"]
code_locus = ""
tests = []
ci_script = ""
status = "declared"
strengthened_in = []
```

**Strengthenings at close (no new IDs — `strengthened_in += "PHASE4-N-F-C"`):** `DC-CINPUT-01`
(partial→enforced once the production restart path lands), `DC-CINPUT-02a` (consumption now
exercised), `CN-NODE-01`, `CN-PROD-03` (cold-start scope retired for bounty-primary), `CN-STORE-02`,
`T-REC-01`, `T-REC-02`, `DC-WAL-03`, `DC-FORGE-01`. A BA-02 evidence-manifest obligation (C6) is
expected to strengthen the existing `RO-LIVE-01` / `RO-LIVE-05` family rather than mint a new RO —
resolve which at `/cluster-plan`.
