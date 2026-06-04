# Invariant Slice — PHASE4-N-F-G-N S1: persist eta0 in the seed-epoch sidecar + WarmStart overlay

> **Status:** Planning artifact (non-normative). Normative authority is the registry + CI.

## §2 Slice Header
- **Slice:** PHASE4-N-F-G-N S1 — add `epoch_nonce` to the persisted `SeedEpochConsensusInputs` (versioned,
  fail-closed), carry it from `LiveConsensusInputsCanonical` through the admission merge, recover it at
  WarmStart, and overlay it onto the forge `PraosChainDepState` — so the forge signs the header VRF over the
  real recovered eta0, not the snapshot's `Nonce::ZERO`.
- **Cluster:** PHASE4-N-F-G-N — WarmStart forge eta0 from the recovered seed-epoch consensus input.
- **Status:** planned.
- **CE addressed:** CE-G-N-1 (the mechanical persist + recover + overlay + fail-closed). [S2 = store regen + live C1.]

## §3 Dependencies
- Proven defect: forge instrument logged `eta0 = Nonce::ZERO`; the C1 follower expects `953a4c34…`.
- S1-recon: `SeedEpochConsensusInputs` (`ade_ledger/src/seed_consensus_inputs.rs`) OMITS `epoch_nonce`; the
  imported `LiveConsensusInputsCanonical` has it; the merge (`seed_consensus_merge.rs`) drops it; the snapshot
  `chain_dep` carries ZERO; `bootstrap_initial_state` (bootstrap.rs:234) returns the ZERO chain_dep.
- `warm_start_recovery` (node_lifecycle.rs:1239) already passes `RequiredFromRecoveredProvenance` (the sidecar
  is restored at recovery — it just lacks eta0 today).

## §4 Intent (invariant impact)
Make eta0 an explicit, persisted, recovered consensus input — not a snapshot placeholder. Enforces `T-REC-04`
+ `DC-CINPUT-03`. NO VRF crypto/variant change.

## §5 Scope / What is built
1. **`SeedEpochConsensusInputs.epoch_nonce`** — new field (a `Nonce`), required at construction.
2. **Versioned, fail-closed CBOR** — bump the sidecar schema version; the decoder fails closed on the schema
   change (old version → `UnknownVersion` / a structured `SeedEpochConsensusInputsMissingEpochNonce`), NEVER a
   default-to-zero. CN-CINPUT-01 stays the sole encoder.
3. **Merge persists it** — `merge_seed_epoch_consensus_inputs` carries `LiveConsensusInputsCanonical.epoch_nonce`
   into the persisted record.
4. **WarmStart overlay** — `bootstrap_initial_state` (warm-start, `RequiredFromRecoveredProvenance` path) sets
   the recovered `chain_dep.epoch_nonce` (+ `evolving_nonce`) from the recovered sidecar's `epoch_nonce`,
   before returning `BootstrapState`. Snapshot chain_dep structure otherwise preserved.
5. **Regression tests:** (a) persist→recover round-trips eta0 byte-identically; (b) ZERO-nonce snapshot +
   `953a4c34…` sidecar ⇒ recovered forge `chain_dep.epoch_nonce == 953a4c34…`; (c) an old/missing-eta0 sidecar
   fails closed with the structured error (no ZERO fallback).
6. **Registry + CI:** `T-REC-04` + `DC-CINPUT-03` → enforced; a CI gate asserts the sidecar carries eta0, the
   merge persists it, the overlay applies it, and the fail-closed path exists.

**Out of scope:** the C1 store regeneration + live confirmation (S2); any VRF crypto change; the admission
runner's own chain_dep (line 247, already correct); the feed-side KeepAlive.

## §6 Execution Boundary (TCB color)
`ade_ledger` sidecar codec (versioned closed CBOR, fail-closed) + `ade_runtime::bootstrap` recovery assembly
(deterministic). The forge consumes the corrected `chain_dep` through the UNCHANGED BLUE Praos VRF path.

## §11 Replay / Crash / Epoch Validation
Persist→recover of eta0 is deterministic (same canonical inputs ⇒ byte-identical sidecar ⇒ same recovered
chain_dep). Covered by the round-trip + recovery regression tests. No new authoritative transition.

## §12 Mechanical Acceptance Criteria
- [ ] `SeedEpochConsensusInputs` has an `epoch_nonce` field.
- [ ] The sidecar codec is versioned; the schema change is fail-closed (old/missing eta0 → structured error,
  never default-zero).
- [ ] `merge_seed_epoch_consensus_inputs` persists `epoch_nonce` from `LiveConsensusInputsCanonical`.
- [ ] `bootstrap_initial_state` overlays the recovered sidecar `epoch_nonce` onto the forge `chain_dep`.
- [ ] Regression: imported eta0 `953a4c34…` + ZERO snapshot + sidecar round-trip ⇒ recovered `chain_dep.epoch_nonce == 953a4c34…`.
- [ ] Old/missing sidecar eta0 fails closed (`SeedEpochConsensusInputsMissingEpochNonce` / version mismatch).
- [ ] `T-REC-04` + `DC-CINPUT-03` enforced; CI gate present.
- [ ] No regression: `ade_ledger` seed-consensus suite + `ade_runtime` bootstrap/recovery suite + `ade_node`
  warm-start tests pass.

## §14 Hard Prohibitions
- no genesis-derived nonce; no manual / C1-only override; no VRF variant change; no private-only branch;
- no hiding eta0 only in the snapshot; no default-to-zero / accept-both fallback (old sidecars fail closed);
- no weakening of `self_accept`; no RO-LIVE flip; no acceptance claim without the follower log through `correlate`.

## §15 Explicit Non-Goals
The C1 store regeneration + live confirmation (S2); any VRF crypto change; the feed-side KeepAlive; durable
block-1+ progression.
