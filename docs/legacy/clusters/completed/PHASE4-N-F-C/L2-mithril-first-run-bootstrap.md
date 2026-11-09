# Slice PHASE4-N-F-C / L2 — Mithril-only first-run bootstrap

> Fills the FirstRun arm of the `--mode node` lifecycle owner stood up in L1
> (`crates/ade_node/src/node_lifecycle.rs`, marker `PHASE4-N-F-C-LIFECYCLE-OWNER`).
> L1 left that arm a typed fail-closed stub (`NodeLifecycleError::FirstRunRequiresMithril`,
> exit 40). L2 replaces it with a REAL Mithril-only first-run bootstrap that persists a
> Mithril-verified recovered surface, and still fails closed when Mithril provenance is
> absent or the binding does not hold. Authority doc: `cluster.md`. Plan:
> `../../planning/phase4-n-f-c-cluster-slice-plan.md`. Invariant sketch:
> `../../planning/phase4-n-f-c-invariants.md`.

## 2. Slice Header
- **Slice Name:** Wire the FirstRun arm of `run_node_lifecycle` to `bootstrap_from_mithril_snapshot`
  (its first non-test caller) over the owner's `PersistentChainDb` + `FileWalStore`, with
  `verify_mithril_binding` fail-closed before any state is admitted; persist the seed-epoch sidecar +
  WAL provenance under one `BootstrapAnchor` lineage; fail closed (no genesis / bundle / cold / tip
  fallback) when Mithril provenance is missing or the binding fails.
- **Cluster:** PHASE4-N-F-C — Build the real Ade node lifecycle.
- **Status:** Proposed.
- **Cluster Exit Criteria Addressed:** CE-L-2.
- **Slice Dependencies:** L1 (the `--mode node` owner skeleton + FirstRun/WarmStart branch + the
  `lifecycle_owner` CI gate this slice refines). Consumes shipped N-F-A/N-Y/N-Z surfaces unchanged.

## 3. Implementation Instruction (AI)
Implement §10 only — the FirstRun arm + its CLI inputs + the L1-gate refinement + the hermetic
fixture/tests. Do NOT touch L3 (warm-start recovery), L4 (peer fetch → apply), L5 (produce / consume
fence), or L6 (BA-02). `bootstrap_from_mithril_snapshot` and every authority it composes already exist
and are consumed verbatim — **add no new BLUE authority** and do not modify `mithril_bootstrap.rs`'s
composition. The live preprod claim is NOT made by this slice's mechanical CEs (see §9.4). Commit with
the model-attribution trailer.

## 4. Intent
Make the Ade node's first run obtain its initial state ONLY from a Mithril-certified bootstrap: the
recovered seed-epoch surface is established iff a real Mithril manifest is present and
`verify_mithril_binding` passes; otherwise the lifecycle fails closed. There is no genesis branch, no
`--consensus-inputs-path`-as-forge-input, no peer-extracted-without-cert path, no tip-bundle, and no
cold `produce_mode` fallback. (CE-A-4b's populate half: this is the first non-test caller of the
sidecar/WAL-provenance populate path; producer CONSUMPTION is L5.)

## 5. Scope
- **Modules / crates:**
  - `ade_node::node_lifecycle` (RED) — the FirstRun arm: assemble the Mithril seed from documented-
    extraction inputs, call `bootstrap_from_mithril_snapshot`, handle its closed error surface, persist,
    exit.
  - `ade_node::cli` (RED) — the FirstRun input flags (manifest + documented-extraction files +
    operator-independent seed point); see §9.2.
  - `ci/ci_check_lifecycle_owner_uses_bootstrap_initial_state.sh` — **refined** (see §9.5): allow the
    Mithril-seed extraction, keep the forge-time/fallback fence; add the positive
    `bootstrap_from_mithril_snapshot` requirement on the owner's FirstRun path.
  - `ade_runtime::{mithril_bootstrap, mithril_import, seed_import, consensus_inputs}` (RED) — **consumed
    unchanged** (`bootstrap_from_mithril_snapshot`, `import_mithril_manifest_from_bytes`,
    `import_cardano_cli_json_utxo`, `import_live_consensus_inputs`).
- **State machines affected:** none new. The FirstRun arm composes existing authorities.
- **Persistence impact:** FIRST production write of the anchor-keyed `SeedEpochConsensusInputs` sidecar
  + its WAL provenance (tag 3) — via `bootstrap_from_mithril_snapshot`'s existing tail, over
  `PersistentChainDb` + `FileWalStore` (not `InMemoryChainDb`). No new persisted format.
- **Network-visible impact:** none (L2 reads operator-supplied files; the live peer restore is an
  operational prerequisite, not node I/O — §9.4).
- **Out of scope:** L3–L6; any change to `mithril_bootstrap.rs` composition; any new BLUE authority;
  native Mithril UTXO-HD/LedgerDB decode.

## 6. Execution Boundary (TCB color)
- **BLUE (reuse only):** `verify_mithril_binding` (the sole binding authority), `encode_seed_epoch_consensus_inputs`,
  the `bootstrap_initial_state` internal authorities. No BLUE change.
- **GREEN (reuse only):** `merge_seed_epoch_consensus_inputs` (inside the composer).
- **RED:** the FirstRun arm wiring + the documented-extraction file reads + `bootstrap_from_mithril_snapshot`
  (RED composer). All file I/O and the composer call are RED.
- **CI:** the refined `lifecycle_owner` gate; the existing `ci_check_consensus_input_provenance.sh`
  (CN-CINPUT-02) stays green — its global containment already restricts the sidecar populate calls to
  the two composers, and L2 calls the composer (not the populate primitives directly).

## 7. Invariants Preserved
- `CN-NODE-01` — initial state still flows ONLY through the single `bootstrap_initial_state`
  (`bootstrap_from_mithril_snapshot` calls it; the owner does not call a second bootstrap authority).
- `CN-CINPUT-01` — sole `SeedEpochConsensusInputs` codec (reused; no second codec).
- `CN-CINPUT-02` — the sidecar populate path stays contained to the verified-bootstrap composers; the
  owner calls the composer, not `.put_seed_epoch_consensus_inputs(` / `merge_…(` /
  `append_seed_epoch_provenance(` directly.
- `CN-ANCHOR-01` / `DC-ANCHOR-01` — one minted anchor lineage; the sidecar is anchor-fp-bound.
- `CN-MITHRIL-01` / `DC-MITHRIL-02` — `verify_mithril_binding` is the sole non-tautological binding
  authority; the anchor seed_point comes from operator-independent inputs, never the manifest.
- L1's mode-closure + the diagnostic `produce_mode` / `admission` paths are unchanged.

## 8. Invariants Strengthened or Introduced
- **Strengthens `CN-NODE-01` (the lifecycle owner now actually obtains state via the authority on the
  FirstRun arm)** and the L1 `lifecycle_owner` gate (positive `bootstrap_from_mithril_snapshot`
  requirement). No registry edit in-slice; `strengthened_in += "PHASE4-N-F-C"` recorded at cluster
  close. `CN-CINPUT-02`/`DC-CINPUT-01` populate-side becomes production-exercised (their first non-test
  caller) — also recorded at close, not in-slice.

## 9. Design Summary

### 9.1 The FirstRun composition (grounded in the verified signature)
`bootstrap_from_mithril_snapshot` (verified at HEAD `4f29947`,
`crates/ade_runtime/src/mithril_bootstrap.rs:117`) is:

```
bootstrap_from_mithril_snapshot(
    seed_point_inputs: &MithrilSeedPointInputs,     // operator-INDEPENDENT seed point (DC-MITHRIL-02)
    seed_ledger:       LedgerState,                 // from documented UTxO extraction
    seed_chain_dep:    PraosChainDepState,          // eta0 from documented consensus-input extraction
    manifest_bytes:    &[u8],                        // the Mithril manifest JSON (provenance)
    seed_consensus_inputs: &LiveConsensusInputsCanonical, // documented consensus-input extraction
    chaindb, snapshot_store, wal, era_schedule, ledger_view,
) -> Result<MithrilBootstrapOutput, MithrilBootstrapError>
```

Internal order (already implemented; not changed by L2): `import_mithril_manifest_from_bytes` →
`mint(anchor from seed_point_inputs)` → **`verify_mithril_binding(report, anchor)` fail-closed BEFORE
any `bootstrap_initial_state`** → `bootstrap_initial_state(genesis_initial = Some((seed_ledger,
seed_chain_dep)), NotRequired)` → `persist_seed_epoch_consensus_inputs` (GREEN merge → A1 encode →
sidecar put → WAL provenance append). Closed error surface `MithrilBootstrapError =
{Import, Binding, Bootstrap, SeedConsensusMerge, SeedConsensusPersist, SeedConsensusProvenanceWal}`.

The L2 FirstRun arm: assemble the five seed args from the documented-extraction inputs, call the
composer over the owner's persistent stores, map every `MithrilBootstrapError` to a fail-closed
`NodeLifecycleError`, and on success persist-and-exit (§9.3).

### 9.2 FirstRun inputs (documented extraction from Mithril-certified state)
The arm requires, all operator-supplied as files/values produced by the documented-interface path
(§9.4): (a) a **Mithril manifest** (Ade `RawMithrilManifest` JSON: `artifact_type`,
`certificate_hash_hex`, `network_magic`, `genesis_hash_hex`, `certified_point{slot, block_hash_hex}`,
`immutable_range{lo,hi}`, `source_command`) → `import_mithril_manifest_from_bytes`; (b) a **UTxO seed**
(cardano-cli `query utxo` JSON) → `import_cardano_cli_json_utxo` → `(UTxOState, UtxoFingerprint)` →
`seed_ledger`; (c) **consensus inputs** (documented extraction: epoch nonce / stake / ASC / epoch
window) → `import_live_consensus_inputs` → `LiveConsensusInputsCanonical` → `seed_consensus_inputs` +
`seed_chain_dep` (eta0); (d) the **operator-independent seed point** (slot, block hash, genesis hash,
network magic, artifact/UTxO/initial-ledger fingerprints) → `MithrilSeedPointInputs`.
**Flag-naming decision (open for review):** reuse the existing canonical importers (file formats are
identical to admission/produce); whether to reuse `--json-seed-path` / `--consensus-inputs-path` or
introduce `--node-*` flag names is an implementation choice, with the binding invariant that these
inputs feed ONLY the Mithril composer on the FirstRun arm (never a forge base — L5 fences that).
Recommendation: a new `--mithril-manifest-path` plus reuse of the existing extraction flags, since the
file formats are unchanged and the provenance comes from the manifest + binding, not the flag name.

### 9.3 Post-bootstrap behavior
On success the recovered surface is durably persisted (sidecar + WAL provenance + anchor + snapshot).
L2 does NOT sync (L4) or produce (L5), so the FirstRun arm logs an explicit success record
("first-run Mithril bootstrap complete; anchor_fp=…, epoch=…; sync/produce not wired (L4/L5); no block
produced") and **exits 0**. A subsequent run is a WarmStart (non-empty store) and is owned by L3 (which
fails closed until wired). Exit 0 here states only "first-run bootstrap succeeded and persisted a
recoverable, Mithril-verified surface" — never "the node ran" or "a block was produced."

### 9.4 Hermetic mechanics proof vs live Mithril claim (REQUIRED distinction)
- **Hermetic fixture (mechanical CE — this slice):** a committed, clearly-labelled throwaway fixture —
  a synthetic `RawMithrilManifest` + a tiny UTxO + tiny consensus inputs + a matching operator seed
  point, constructed so `verify_mithril_binding` PASSES — drives the FirstRun arm over a real
  `PersistentChainDb` + `FileWalStore` (tempdir), proving the WIRING: composer called, binding gate
  runs before admit, sidecar + WAL provenance persisted, one anchor lineage, exit 0. Plus NEGATIVE
  fixtures: absent manifest ⇒ fail closed; binding mismatch (manifest `certified_point` ≠ operator seed
  point) ⇒ fail closed; malformed extraction ⇒ fail closed. **The fixture's cert hashes are synthetic
  and prove NOTHING about real Mithril** — it proves Ade's composition + fail-closed mechanics only. A
  throwaway-fixture comment is mandatory (no `.skey`/real-cert material).
- **Live Mithril artifact (operational prerequisite — NOT a mechanical CE here):** the real preprod
  claim ("Ade first-run-bootstrapped from a real Mithril-certified preprod snapshot") requires the real
  certified artifact. The operational prerequisite is MET (verified: `mithril-client cardano-db verify`
  over the existing preprod db ran all 5 steps + emitted `verified_db_directory` + exited 0, genesis-key
  rooted; snapshot `c6873f15…`, cert `b6b37700…`, epoch 291 / immutable 5758). **Open proof obligation
  for the live leg:** the real aggregator certificate JSON is NOT in Ade's `RawMithrilManifest` shape —
  it gives `signed_entity_type.CardanoDatabase{epoch, immutable_file_number}` + `merkle_root`, not a
  `certified_point{slot, block_hash}`. Constructing a valid Ade manifest for the live leg requires
  DERIVING the certified point (slot + block hash at the certified immutable boundary 5758) from the
  restored certified state (e.g. cardano-cli query of that boundary block). That mapping is itself a
  live-leg proof obligation, operator-gated; it is NOT exercised by the hermetic CE and is NOT claimed
  by L2's mechanical close.
- **Consensus-input binding honesty:** `verify_mithril_binding` ties the ANCHOR (seed point + UTxO
  fingerprint + genesis hash + network magic + certificate hash) to the cert. The CONSENSUS INPUTS
  (PoolDistr/ASC/eta0) are bound INDIRECTLY — by being extracted from the same certified state at the
  same epoch — not by a separate cryptographic check inside the binding. **Design decision for review:**
  add an epoch-consistency check (the `seed_consensus_inputs.epoch_no` must agree with the manifest's
  certified epoch, derivable from `certified_point.slot` via the era schedule) so the consensus inputs'
  tie to the cert is mechanical, not bare operator discipline. Recommended; flagged not pre-coded.

### 9.5 L1 gate refinement (must, not optional)
L1's `ci_check_lifecycle_owner_uses_bootstrap_initial_state.sh` guard (c) currently forbids
`import_live_consensus_inputs` and `consensus_inputs_path` ANYWHERE in the owner — which would block
this slice, because the FirstRun arm legitimately reads consensus inputs at BOOTSTRAP time to seed the
Mithril composer. L2 REFINES (does not blanket-remove) that guard: the consensus-input / UTxO read is
permitted on the FirstRun arm **iff** it feeds `bootstrap_from_mithril_snapshot`; the gate adds a
POSITIVE requirement (the owner's FirstRun path calls `bootstrap_from_mithril_snapshot(`) and keeps the
NEGATIVE fence that the owner calls NO `bootstrap_initial_state(` directly, NO
`bootstrap_from_conway_genesis(`, and builds NO `InMemoryChainDb` / `LedgerState::new(` /
`materialize_rolled_back_state(` forge/cold base. The forge-time consensus-input prohibition is L5's
CN-CINPUT-03 (the owner has no forge path yet in L2).

## 10. Changes Introduced
### Types
- A small closed input struct for the FirstRun Mithril seed (paths + seed-point values), and additional
  fail-closed `NodeLifecycleError` variants mapping the `MithrilBootstrapError` surface (Import /
  Binding / Bootstrap / merge / persist / provenance-WAL) + extraction-read failures + a
  missing-manifest variant. No new BLUE/canonical type.
### State Transitions
- FirstRun arm: from L1's `Err(FirstRunRequiresMithril)` stub to the real composition (§9.1). No new
  authoritative transition (reuses `bootstrap_from_mithril_snapshot`).
### Persistence
- First production write of the anchor-keyed sidecar + WAL provenance (via the existing composer tail).
  No new format.
### Removal / Refactors
- None to `produce_mode` / `admission` / `mithril_bootstrap.rs`. The L1 stub arm is replaced; the L1
  gate is refined (§9.5).

## 11. Replay, Crash, and Epoch Validation
- **Replay:** no new authoritative transition; the sidecar persist/replay round-trip is already covered
  by N-F-A. L2's hermetic fixture asserts the persisted sidecar bytes decode via the A1 codec and the
  WAL provenance entry is present (tag 3). The full `persist → recover` byte-identity is L3's obligation.
- **Crash/restart:** the composer's put-then-WAL-append ordering (a crash between leaves an unrecorded
  sidecar = "not imported", fail-closed on next warm-start) is inherited unchanged; L2 adds no new crash
  window. Production warm-start recovery is L3.
- **Epoch:** single seed epoch; the §9.4 epoch-consistency check (if adopted) asserts the consensus
  inputs' epoch agrees with the certified epoch.

## 12. Mechanical Acceptance Criteria
- [ ] Refined `ci/ci_check_lifecycle_owner_uses_bootstrap_initial_state.sh` passes: owner FirstRun path
      calls `bootstrap_from_mithril_snapshot(`; owner calls no `bootstrap_initial_state(` directly, no
      `bootstrap_from_conway_genesis(`, builds no `InMemoryChainDb` / `LedgerState::new(` /
      `materialize_rolled_back_state(`.
- [ ] `ci/ci_check_consensus_input_provenance.sh` (CN-CINPUT-02) stays green (owner calls the composer,
      not the populate primitives directly).
- [ ] `ci/ci_check_node_mode_closure.sh` + `ci/ci_check_bootstrap_closure.sh` stay green.
- [ ] Hermetic POSITIVE test: FirstRun arm over a tempdir `PersistentChainDb` + `FileWalStore` with the
      synthetic-but-consistent fixture ⇒ `verify_mithril_binding` passes, sidecar + WAL provenance
      persisted (decodes via A1 codec; WAL tag-3 present), one anchor lineage, exit 0.
- [ ] Hermetic NEGATIVE tests: absent manifest ⇒ fail closed; binding mismatch ⇒ fail closed; malformed
      extraction ⇒ fail closed. None falls back to genesis/bundle/cold; each returns a typed
      `NodeLifecycleError` and a non-zero exit.
- [ ] `cargo build` + scoped `ade_node`/`ade_runtime` tests + the relevant gates pass. Full
      `ade_testkit` corpus/oracle lane is NOT an L2 gate (times out ~600s on clean HEAD).

## 13. Failure Modes (all fail-closed, typed, no fallback)
Missing/malformed Mithril manifest; `verify_mithril_binding` mismatch (any of network_magic /
genesis_hash / certified_point / certificate_hash); UTxO or consensus-input extraction read/parse
failure; merge gap (pool in exactly one map); sidecar persist or WAL-provenance write failure. Every
case is a deterministic non-zero exit with a typed `NodeLifecycleError`. **No genesis branch, no
`--consensus-inputs-path`-as-forge-input, no peer-extracted-without-cert path, no tip-bundle, no cold
`produce_mode` fallback** is reachable from the FirstRun arm.

## 14. Hard Prohibitions
**Inherited (cluster):** no first-run bootstrap without verified Mithril provenance; cardano-cli
extraction valid only as documented extraction from Mithril-certified/restored state;
`verify_mithril_binding` must pass before any state is admitted; no genesis / `--consensus-inputs-path`
forge input / tip-bundle / cold fallback; no second bootstrap/recovery/storage-init authority; no native
Mithril UTXO-HD/LedgerDB decode; no new BLUE authority/type; no `HashMap`/clock/float in BLUE.
**Slice-specific:** no L3–L6 work; no modification to `mithril_bootstrap.rs` composition; no
representing the hermetic fixture as a real-Mithril/preprod claim; no live run without the operational
prerequisite + explicit operator green-light; no registry promotion; no grounding-doc regeneration.

## 15. Explicit Non-Goals
No warm-start recovery (L3), no peer fetch → apply (L4), no produce / consume-side fence (L5), no BA-02
evidence (L6). No live preprod bootstrap claim from the hermetic CE. No aggregator-cert → Ade-manifest
auto-mapping (a live-leg operator step, §9.4). No registry append. No grounding-doc refresh.

## 16. Completion Checklist
- [ ] FirstRun arm calls `bootstrap_from_mithril_snapshot` (first non-test caller) over the owner's
      `PersistentChainDb` + `FileWalStore`; binding fail-closed before admit; sidecar + WAL provenance
      persisted; one anchor lineage; success ⇒ exit 0 with an honest "no block produced" record.
- [ ] FirstRun fails closed (typed, non-zero, no fallback) on missing manifest / binding mismatch /
      extraction failure.
- [ ] L1 `lifecycle_owner` gate refined per §9.5; CN-CINPUT-02 + mode/bootstrap-closure gates stay green.
- [ ] Hermetic positive + negative fixtures committed and labelled throwaway/synthetic; they prove
      mechanics only, not a real-Mithril claim.
- [ ] Live-leg proof obligations recorded (aggregator-cert → Ade-manifest certified-point derivation;
      optional epoch-consistency check) — NOT claimed satisfied by this slice.
- [ ] `cargo build` + scoped tests + relevant gates pass (full corpus lane excluded).
