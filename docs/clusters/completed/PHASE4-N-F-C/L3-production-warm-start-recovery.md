# Slice PHASE4-N-F-C / L3 — Production warm-start recovery

> Fills the WarmStart arm of the `--mode node` lifecycle owner (`crates/ade_node/src/node_lifecycle.rs`,
> marker `PHASE4-N-F-C-LIFECYCLE-OWNER`). L1 left that arm a typed fail-closed stub
> (`NodeLifecycleError::WarmStartRecoveryNotWired`, exit 40). L3 replaces it with a REAL production
> warm-start recovery that, on a non-empty persistent store, replays the WAL, recovers the bootstrap
> provenance, and reconstructs the verified `BootstrapState` — including the recovered
> `SeedEpochConsensusInputs` — through the single `bootstrap_initial_state` authority with
> `RequiredFromRecoveredProvenance`, failing closed on every provenance defect with NO bundle fallback.
> Authority doc: `cluster.md`. Plan: `../../planning/phase4-n-f-c-cluster-slice-plan.md`. Invariant
> sketch: `../../planning/phase4-n-f-c-invariants.md`.

## 2. Slice Header
- **Slice Name:** Wire the WarmStart arm of `run_node_lifecycle` to the production warm-start recovery
  chain — `FileWalStore::read_all` → `replay_from_anchor` → `RecoveredBootstrapProvenance` →
  `bootstrap_initial_state(RequiredFromRecoveredProvenance)` → recovered `BootstrapState` (with
  `seed_epoch_consensus_inputs: Some(..)`) — over the owner's `PersistentChainDb` + `FileWalStore`;
  fail closed (typed, non-zero exit, NO bundle/genesis/cold fallback) on missing WAL provenance,
  missing sidecar, sidecar hash mismatch, anchor mismatch, or duplicate provenance.
- **Cluster:** PHASE4-N-F-C — Build the real Ade node lifecycle.
- **Status:** Proposed.
- **Cluster Exit Criteria Addressed:** CE-L-3.
- **Slice Dependencies:** L1 (the `--mode node` owner skeleton + the on-disk first-run/warm-start branch +
  the `lifecycle_owner` CI gate this slice refines), L2 (the first-run bootstrap that persists the anchor
  lineage + sidecar + WAL provenance the warm-start recovers — consumed unchanged; **not modified**).

## 3. Implementation Instruction (AI)
Implement §10 only — the WarmStart arm + the `lifecycle_owner`-gate refinement + the hermetic
fixture/tests. Do NOT touch L2 (FirstRun / Mithril composition), L4 (peer fetch → apply), L5 (produce /
consume fence), or L6 (BA-02). Do NOT modify `mithril_bootstrap.rs`, `produce_mode.rs`, or `admission`.
Do NOT call `recover_node_state` (it hardcodes `SeedEpochConsensusSource::NotRequired` and is the
test/capability helper — wiring it would NOT recover the sidecar; see §9.3). `replay_from_anchor`,
`bootstrap_initial_state`, `restore_seed_epoch_consensus_inputs`, and the A1 codec already exist and are
consumed verbatim — **add no new BLUE authority** and do not change `bootstrap.rs` / `replay.rs`. Resolve
the two entry obligations in §9.0 before coding the arm. No code, no registry edits, and no grounding-doc
regeneration are part of THIS doc commit. Commit (later, for the implementation) with the
model-attribution trailer.

## 4. Intent
Make the recovered `SeedEpochConsensusInputs` surface a *production* fact rather than a capability: the
warm-start path that restores it — through the single `bootstrap_initial_state` authority, verified
fail-closed against the WAL-replayed `RecoveredBootstrapProvenance` — is exercised by the `--mode node`
production owner, not a test-only helper. After `persist (L2) → WAL → restore (L3)` the recovered record
is byte-identical to what was persisted, or the node halts. This provides the evidence intended to flip
**DC-CINPUT-01** from partial → enforced at cluster close, subject to L4/L5 not weakening the
recovered-state path. No registry status is changed in this slice.

## 5. Scope
- **Modules / crates:**
  - `ade_node::node_lifecycle` (RED) — the WarmStart arm: obtain the bootstrap anchor fingerprint (§9.0
    W2), `wal.read_all()`, build the preserved-block-bytes map from the chaindb, `replay_from_anchor`,
    the deterministic chaindb→WAL-tail reconciliation, `bootstrap_initial_state(BootstrapInputs { …,
    genesis_initial: None, seed_epoch_consensus_source: RequiredFromRecoveredProvenance(provenance) })`,
    map the closed error surface to typed fail-closed `NodeLifecycleError`, exit.
  - `ci/ci_check_lifecycle_owner_uses_bootstrap_initial_state.sh` — **refined** (see §9.4): the WarmStart
    arm may now call `bootstrap_initial_state(` directly; add the positive WarmStart requirement
    (`RequiredFromRecoveredProvenance`), the no-`recover_node_state(` fence, and the containment that
    `RequiredFromRecoveredProvenance` is constructed only in the owner; keep the cold/genesis fences.
  - `ade_ledger::wal::replay` + `ade_runtime::bootstrap` (BLUE/authority) — **consumed unchanged**
    (`replay_from_anchor`, `ReplayOutcome`, `RecoveredBootstrapProvenance`, `bootstrap_initial_state`,
    `restore_seed_epoch_consensus_inputs`, the A1 codec).
- **State machines affected:** none new. The WarmStart arm composes existing authorities.
- **Persistence impact:** READS the WAL, the anchor-keyed sidecar, and the chaindb/snapshot. The only
  WRITE is the deterministic, idempotent `chaindb.rollback_to_slot(wal_tail_slot)` reconciliation that the
  proven recovery path already performs (no new persisted format; a no-op in the clean L3 fixture case).
  W2 may add the smallest explicit anchor-retrieval surface if none exists (§9.0).
- **Network-visible impact:** none (L3 does not sync — L4).
- **Out of scope:** L2 / L4 / L5 / L6; any change to `bootstrap.rs`, `replay.rs`, `mithril_bootstrap.rs`,
  `produce_mode.rs`, `admission`, or `recover_node_state`; any new BLUE authority/type; orphan-block crash
  recovery proof (that is L4c).

## 6. Execution Boundary (TCB color)
- **BLUE (reuse only — no change):** `ade_ledger::wal::replay::{replay_from_anchor, ReplayOutcome,
  RecoveredBootstrapProvenance}`; the `bootstrap_initial_state` single authority + its
  `restore_seed_epoch_consensus_inputs` verify chain (hash → A1 decode → anchor/epoch binding →
  byte-identity re-encode) + the A1 `decode/encode_seed_epoch_consensus_inputs` codec.
- **GREEN (reuse only):** `classify_start` (the on-disk first-run/warm-start branch decision, already
  shipped in L1; unchanged).
- **RED:** the WarmStart arm wiring in `node_lifecycle.rs` (store/WAL reads, the anchor-fingerprint read,
  block-bytes assembly, the reconciliation write, the `bootstrap_initial_state` call, error mapping, exit).
- **CI:** the refined `ci_check_lifecycle_owner_uses_bootstrap_initial_state.sh`.

## 7. Invariants Preserved
- `CN-NODE-01` — initial state still flows ONLY through the single `bootstrap_initial_state`; the
  WarmStart arm calls that one authority (no second bootstrap/recovery/storage-init path; it does NOT
  call `recover_node_state`).
- `CN-CINPUT-01` — sole `SeedEpochConsensusInputs` A1 codec (reused; no second codec).
- `CN-CINPUT-02` — the sidecar **populate** path stays contained to the verified-bootstrap composers; L3
  only RESTORES (reads + verifies) the sidecar, it never constructs/puts/encodes it.
- `DC-CINPUT-02a` — the `PoolDistrView`/`ExpectedVrfInput` projection determinism is untouched (consumption
  is L5).
- `CN-ANCHOR-01` / `DC-ANCHOR-01` — recovery binds to exactly one `BootstrapAnchor` lineage; the
  anchor-fp seeds `replay_from_anchor` and keys the sidecar.
- `CN-STORE-02` — the owner uses the persistent `PersistentChainDb` + `FileWalStore` (no `InMemoryChainDb`).
- L1 mode-closure; the L2 FirstRun arm + the diagnostic `produce_mode` / `admission` paths — unchanged.

## 8. Invariants Strengthened or Introduced
- **Provides the evidence for the cluster-close flip of `DC-CINPUT-01` (registry id `DC-CINPUT-01`) from
  partial → enforced**, subject to L4/L5 not weakening the recovered-state path: the
  `Anchor + WAL → recovered SeedEpochConsensusInputs` chain is now exercised by a **production entry
  point** (`--mode node` WarmStart), byte-identical, fail-closed. The supporting tests are the new
  `node_lifecycle` persist→recover byte-identity positive + the five fail-closed negatives (§11/§12).
  **No registry status is flipped inside L3.**
- **Evidence also accrues to** (recorded only at `/cluster-close`, never in-slice): `T-REC-01`, `T-REC-02`
  (recovery determinism), `DC-WAL-03` (WAL replay-equivalence now driven by the production owner),
  `CN-NODE-01` (warm-start arm proven to use the single authority), `CN-STORE-02`, `CN-ANCHOR-01`,
  `DC-ANCHOR-01`.
- No registry edit in-slice: `strengthened_in += "PHASE4-N-F-C"` and any `tests`/`ci_scripts` appends are
  recorded at cluster close (consistent with L1/L2).

## 9. Design Summary

### 9.0 Two entry obligations surfaced by grounding in `bootstrap_initial_state` (resolve before coding)
Reading the real authority at `ddc84be` revealed two facts the one-line L3 chain hides. Both are intrinsic
to "production warm-start recovery" and must be resolved at the start of implementation:

- **(W1) The sidecar restore is reachable ONLY when a tip or snapshot is persisted.**
  `bootstrap_initial_state` restores the sidecar *only* in its warm-start branch
  (`tip.is_some() || !snapshot_slots.is_empty()`); its cold-start branch (`tip.is_none() &&
  snapshot_slots.is_empty()`) returns `seed_epoch_consensus_inputs: None` regardless of the source
  (`bootstrap.rs:172–186`). **L2's first run is a cold-start that persists the sidecar + WAL provenance
  but NO tip and NO snapshot.** So an L2-only store both (a) re-classifies as `FirstRun` (no tip, no
  snapshots) and (b) would cold-start again even if forced to warm-start — the
  `RequiredFromRecoveredProvenance` restore is never entered. In production, the tip/snapshot that makes
  warm-start fire is produced by **L4** (durable block apply).
  **L3's scope (resolved):** *L3 proves the warm-start recovery path over a valid constructed warm-start
  precondition; L4c later proves that normal peer fetch + durable apply creates that precondition
  naturally.* Concretely, L3 ships a **minimal committed warm-start fixture** (a *constructed warm-start
  precondition*: anchor lineage + WAL provenance + sidecar + a minimal stored snapshot at the seed slot,
  mirroring the A3b warm-start setup already in `bootstrap.rs` tests, lines ~700–920) and drives the
  WarmStart arm over it. This fixture is a legitimate proof input — it is *the valid persisted warm-start
  precondition*, not fabricated evidence — because L3's job is to prove the recovery transition, not that
  L2 alone reaches warm-startable state. L3 does not change L2 and does not depend on L4 at runtime; the
  success log must not imply an L2-only store warm-starts.
- **(W2) The owner must obtain the bootstrap anchor fingerprint from a source independent of the WAL
  provenance entry — mandatory.** `replay_from_anchor(anchor_initial_ledger_fp, …)` takes the anchor fp as
  input and *validates* that the WAL provenance entry's `anchor_fp` equals it (`replay.rs:138`). If the
  only way to get `anchor_fp` is to read it from the WAL entry being checked, the mismatch check is
  circular. So L3 must read the anchor fp from an **independent persisted `BootstrapAnchor` / anchor
  pointer**, never from the provenance entry. Confirm at implement time that L2's bootstrap persists the
  anchor retrievably; **if no such retrieval surface exists, adding the smallest explicit anchor-retrieval
  surface needed for recovery is in scope for L3** — warm-start cannot be correct without it, and it stays
  within the single-authority / one-anchor-lineage invariants (CN-ANCHOR-01, CN-NODE-01).

### 9.1 The WarmStart composition (grounded in the verified signatures)
The arm mirrors the *structure* of the proven `recover_node_state` (`recovery/restart.rs:114`) but differs
in the one decisive input — it passes `RequiredFromRecoveredProvenance` instead of `NotRequired`:

```
let anchor_fp = <read persisted BootstrapAnchor initial_ledger_fingerprint>;   // §9.0 W2 (independent source)
let entries = wal.read_all()?;                                                   // FileWalStore::read_all
let block_bytes = <preserved bytes for each AdmitBlock, from chaindb.get_block_by_hash>;
let replay: ReplayOutcome = replay_from_anchor(&anchor_fp, &entries, &block_bytes)?;  // BLUE
let provenance = replay.provenance.ok_or(<fail-closed: no provenance>)?;         // RecoveredBootstrapProvenance
chaindb.rollback_to_slot(wal_tail_slot)?;                                        // deterministic, idempotent
let state: BootstrapState = bootstrap_initial_state(BootstrapInputs {
    chaindb, snapshot_store: chaindb, era_schedule, ledger_view,
    genesis_initial: None,                                                       // warm-start: unused
    seed_epoch_consensus_source: SeedEpochConsensusSource::RequiredFromRecoveredProvenance(provenance),
})?;                                                                             // single authority
debug_assert!(state.seed_epoch_consensus_inputs.is_some());                      // restored + verified
```

`bootstrap_initial_state` internally runs `restore_seed_epoch_consensus_inputs` — the 5-step fail-closed
verify (sidecar present → `blake2b_256` hash == provenance → A1 `decode` → anchor+epoch binding →
byte-identity re-encode), `bootstrap.rs:247–298`. There is **no `--consensus-inputs-path` fallback** inside
it. The owner maps each `BootstrapError`/`WalError` to a typed `NodeLifecycleError` and a non-zero exit.

### 9.2 Post-recovery behavior
On success the recovered `BootstrapState` (ledger + chain_dep + tip + `Some(seed_epoch_consensus_inputs)`)
is held in memory. L3 does NOT sync (L4) or produce (L5), so the WarmStart arm logs an honest record
("warm-start recovery complete; anchor_fp=…, epoch=…, recovered tip=…; sync/produce not wired (L4/L5); no
block produced") and **exits 0**. Exit 0 states only "warm-start recovered a verified, byte-identical
state"; never "the node ran" or "a block was produced." The record must not imply an L2-only store
warm-starts (§9.0 W1).

### 9.3 Why not `recover_node_state`
`recover_node_state` already does read_all → `replay_from_anchor` → reconcile → `bootstrap_initial_state`,
but it hardcodes `seed_epoch_consensus_source: SeedEpochConsensusSource::NotRequired` (`restart.rs:197`),
so it returns `seed_epoch_consensus_inputs: None` — it does **not** recover the sidecar. It is the
test/capability helper (per its own comment + the cluster doc). L3 therefore BUILDS the owner-level
equivalent with `RequiredFromRecoveredProvenance`; it does not call, change, or "wire" `recover_node_state`
(the cluster's overclaim fence). `recover_node_state` stays exactly as-is.

### 9.4 L1 gate refinement (must, not optional)
The L1 `ci_check_lifecycle_owner_uses_bootstrap_initial_state.sh` guard (b) currently forbids
`bootstrap_initial_state\(` in the owner (it carries an explicit "L3 warm-start will call
bootstrap_initial_state directly; re-refine this guard then" note), and guard (c-neg) forbids the bare
token `genesis_initial`. L3 REFINES (does not blanket-remove):
- guard (b): drop `bootstrap_initial_state\(` from the forbidden set; KEEP `InMemoryChainDb` +
  `materialize_rolled_back_state\(` forbidden (the owner delegates materialization to the authority).
- guard (c-neg): forbid `genesis_initial: Some` (a cold/genesis seed) rather than the field name, so the
  WarmStart arm may write `genesis_initial: None`.
- **new positive (WarmStart):** the owner constructs `RequiredFromRecoveredProvenance` (the warm-start
  uses the recovered surface), and `replay_from_anchor(` appears on the owner's warm-start path.
- **new fence (overclaim):** the owner does NOT call `recover_node_state(`.
- **new containment:** `SeedEpochConsensusSource::RequiredFromRecoveredProvenance` is *constructed* only in
  the owner / recovery module (not in `produce_mode` — guard (e) already covers that; nor anywhere else).
- guards (a), (d), (e) unchanged. Comments/`#[cfg(test)]` are stripped before the negative greps as today.

## 10. Changes Introduced
### Types
- Additional fail-closed `NodeLifecycleError` variants mapping the warm-start surface: no WAL provenance
  recovered, WAL replay error (`ChainBreak` / `BlockBytesMissing` / `DuplicateProvenance` /
  `ProvenanceAnchorMismatch`), sidecar restore/verify failure (`SeedConsensusSidecarMissing` /
  `SeedConsensusHashMismatch` / `SeedConsensusBindingMismatch` / `SeedConsensusSidecarDecode`), anchor-read
  failure. A candidate `EXIT_NODE_WARM_START_RECOVERY_FAILED = 42` (distinct from 40 unwired / 41 Mithril).
  No new BLUE/canonical type.
### State Transitions
- WarmStart arm: from L1's `Err(WarmStartRecoveryNotWired)` stub to the real recovery (§9.1). No new
  authoritative transition (reuses `replay_from_anchor` + `bootstrap_initial_state`).
### Persistence
- No new format. Reads WAL/sidecar/snapshot; the only write is the existing idempotent
  `rollback_to_slot` reconciliation. W2 may add a minimal anchor-retrieval read surface (§9.0).
### Removal / Refactors
- None to `bootstrap.rs` / `replay.rs` / `recovery/restart.rs` / `mithril_bootstrap.rs` / `produce_mode` /
  `admission`. The L1 WarmStart stub arm is replaced; the L1 gate is refined (§9.4).

## 11. Replay, Crash, and Epoch Validation
- **Replay (reused, preserved):** the BLUE replay determinism is already covered by
  `replay_from_anchor_two_runs_byte_identical`, `replay_from_anchor_three_entry_chain_ok`,
  `replay_from_anchor_catches_chain_break`, `replay_from_anchor_catches_missing_block_bytes`,
  `replay_from_anchor_empty_wal_returns_anchor_fp` (`ade_ledger/src/wal/replay.rs`); the BLUE restore
  verify chain by the A3b warm-start tests in `ade_runtime/src/bootstrap.rs` (sidecar-missing,
  hash-mismatch, anchor-binding-mismatch, epoch-binding-mismatch). L3 must keep all green.
- **Replay (new, this slice):** `warm_start_recovers_seed_epoch_consensus_inputs_byte_identical` (in
  `crates/ade_node/src/node_lifecycle.rs` tests): construct the warm-start precondition (per §9.0 W1 — the
  minimal committed warm-start fixture: anchor + WAL provenance + sidecar + a minimal stored snapshot at
  the seed slot), drop the handles, re-open, drive the WarmStart arm, and assert the recovered
  `BootstrapState.seed_epoch_consensus_inputs` is `Some` and its A1-re-encoded bytes equal the persisted
  sidecar bytes, and the recovered tip matches.
- **Crash/restart:** the persist → drop handles → re-open → recover sequence IS the clean restart proof.
  Orphan-block (crash-mid-apply, durable-before-tip) recovery is L4c, not L3.
- **Epoch:** single seed epoch; the recovered `provenance.epoch_no` binds the sidecar to that epoch inside
  the verify chain.

## 12. Mechanical Acceptance Criteria
- [ ] Refined `ci/ci_check_lifecycle_owner_uses_bootstrap_initial_state.sh` passes: WarmStart arm may call
      `bootstrap_initial_state(`; owner constructs `RequiredFromRecoveredProvenance` and calls
      `replay_from_anchor(`; owner calls NO `recover_node_state(`, builds no `InMemoryChainDb`, calls no
      `materialize_rolled_back_state(`, writes no `genesis_initial: Some`; `RequiredFromRecoveredProvenance`
      constructed only in the owner; FirstRun composer requirement (`bootstrap_from_mithril_snapshot(`) and
      the single-authority count (guard d) still hold.
- [ ] `ci/ci_check_consensus_input_provenance.sh` (CN-CINPUT-02) stays green (owner restores, never
      populates the sidecar).
- [ ] `ci/ci_check_node_mode_closure.sh` + `ci/ci_check_bootstrap_closure.sh` stay green.
- [ ] Positive: `warm_start_recovers_seed_epoch_consensus_inputs_byte_identical` passes (recovered sidecar
      bytes-identical to persisted; recovered tip matches; exit 0).
- [ ] Negatives — each returns a typed `NodeLifecycleError` and a non-zero exit, with NO bundle/genesis/cold
      fallback:
      - `warm_start_fails_closed_missing_wal_provenance` (no `SeedEpochConsensusInputsImported` entry ⇒
        `replay.provenance == None` ⇒ fail closed)
      - `warm_start_fails_closed_missing_sidecar` (`BootstrapError::SeedConsensusSidecarMissing`)
      - `warm_start_fails_closed_sidecar_hash_mismatch` (`BootstrapError::SeedConsensusHashMismatch`)
      - `warm_start_fails_closed_anchor_mismatch` (`WalError::ProvenanceAnchorMismatch` and/or
        `BootstrapError::SeedConsensusBindingMismatch`)
      - `warm_start_fails_closed_duplicate_provenance` (`WalError::DuplicateProvenance`)
- [ ] `cargo build` + scoped `ade_node` / `ade_runtime` / `ade_ledger` tests + the named gates pass. Full
      `ade_testkit` corpus/oracle lane is NOT an L3 gate (times out ~600s on clean HEAD).

## 13. Failure Modes (all fail-closed, typed, no fallback)
No WAL provenance recovered; WAL replay error (chain break / missing block bytes / duplicate provenance /
anchor mismatch); sidecar missing / hash mismatch / decode failure / anchor-or-epoch binding mismatch /
byte-identity mismatch; anchor-read failure (§9.0 W2). Every case is a deterministic non-zero exit with a
typed `NodeLifecycleError`. **No genesis branch, no `--consensus-inputs-path` bundle, no tip-bundle, no
cold `produce_mode` fallback** is reachable from the WarmStart arm.

## 14. Hard Prohibitions
**Inherited (cluster):** no genesis / `--consensus-inputs-path` bundle / tip-bundle / cold fallback on
recovery; no second bootstrap/recovery/storage-init authority (CN-NODE-01); no shape-swap; no new BLUE
authority/type; no native Mithril decode; no `HashMap`/clock/float/async in BLUE.
**Slice-specific (from the L3 brief):** no forge; no sync; no Mithril first-run changes (no edit to L2 /
`mithril_bootstrap.rs`); no `produce_mode` changes; no bundle fallback; **no `recover_node_state` overclaim**
— do not call it or claim it is production-wired; fail closed on missing WAL, missing sidecar, hash
mismatch, anchor mismatch, duplicate provenance; no change to `bootstrap.rs` / `replay.rs`; no registry
promotion or status flip; no grounding-doc regeneration.

## 15. Explicit Non-Goals
No peer fetch → apply (L4); no produce / consume-side fence (L5); no BA-02 evidence (L6); no orphan-block
crash recovery proof (L4c); no L2/FirstRun change; no `recover_node_state` adoption; no registry append or
status flip; no grounding-doc refresh. L3 does not make any live-preprod claim.

## 16. Completion Checklist
- [ ] §9.0 W1 + W2 resolved (warm-start precondition built as a minimal committed warm-start fixture;
      anchor fp read from an independent persisted source).
- [ ] WarmStart arm composes `read_all → replay_from_anchor → bootstrap_initial_state(
      RequiredFromRecoveredProvenance)` over the owner's persistent stores; recovers
      `BootstrapState.seed_epoch_consensus_inputs: Some(..)` byte-identical; success ⇒ exit 0 with an honest
      "no block produced" record.
- [ ] WarmStart fails closed (typed, non-zero, no fallback) on the five named defects.
- [ ] L1 `lifecycle_owner` gate refined per §9.4; CN-CINPUT-02 + mode/bootstrap-closure gates stay green.
- [ ] `recover_node_state` unchanged and uncalled by the owner; `produce_mode` / `admission` / L2 FirstRun
      unchanged.
- [ ] DC-CINPUT-01 cluster-close-flip evidence: the positive + five negatives committed and green (no
      registry status flipped in-slice).
- [ ] `cargo build` + scoped tests + named gates pass (full corpus lane excluded).

## 17. Review Notes
- **Invariant risk considered:** that the WarmStart arm becomes a second bootstrap/recovery authority. It
  does not — it calls the single `bootstrap_initial_state` and does not touch `recover_node_state`; the gate
  refinement (§9.4) makes that mechanical.
- **Assumption challenged (W1):** "an L2 first run is directly warm-start-recoverable." It is NOT — L2's
  cold-start persists no tip/snapshot, so the sidecar-restore branch isn't entered until blocks are applied
  (L4). L3 proves the warm-start recovery path over a valid constructed warm-start precondition; L4c later
  proves normal peer fetch + durable apply creates that precondition naturally. This is not carry-forward:
  L3 fully ships and tests the recovery authority path.
- **Assumption challenged (W2):** the anchor fp must come from an independent persisted source, not the WAL
  provenance entry, or the anchor-mismatch check is vacuous.
- **Follow-up slices implied:** L4 (peer fetch → durable apply, which makes warm-start naturally reachable),
  L4c (sync → kill → recover same-tip proof), L5 (produce from the recovered surface — DC-CINPUT-02b /
  CN-CINPUT-03).
