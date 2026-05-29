# Invariant Slice — S1: Mithril production bootstrap + seed-point independence

> Single slice of PHASE4-N-Z. Closes `RO-MITHRIL-IMPORT-01` open-obligation item (b)
> (wired production composition + independence gate). Composition-only — no CLI flag.

## §2 Slice Header

- **Slice Name:** `bootstrap_from_mithril_snapshot` — production composition that binds a Mithril manifest against an anchor minted from operator-provided independent seed-point inputs, fail-closed before storage init.
- **Cluster:** PHASE4-N-Z.
- **Status:** Proposed.
- **Cluster Exit Criteria Addressed (verbatim):** CE-Z-2, CE-Z-3, CE-Z-4, CE-Z-5, CE-Z-6, CE-Z-7 (S1 portion), CE-Z-8.
- **Slice Dependencies:** PHASE4-N-Y S1+S7 (merged) — `verify_mithril_binding`, `MithrilManifestReport`, `import_mithril_manifest`, the `mint` authority.

## §3 Implementation Instruction (AI)

Mirror `crates/ade_runtime/src/genesis_bootstrap.rs::bootstrap_from_conway_genesis` exactly in shape (composition-only RED shell, routes through the single `bootstrap_initial_state`). Reuse the existing BLUE `verify_mithril_binding` + `mint` + RED `import_mithril_manifest` — introduce NO new authority. The one rule that matters: the anchor's `seed_slot`/`seed_block_hash` come from the operator seed-point inputs passed to the function, NEVER from the manifest import. If you find yourself needing a second semantic authority, STOP (OI-Z.1). §12 is the contract. Commit carries the model trailer; no AI-authoring source comments.

## §4 Intent

Make it impossible for a Mithril bootstrap to bind the manifest against a `seed_point` that the manifest itself supplied — the two compared origins must be structurally independent (operator seed-point extraction vs. Mithril manifest), so `verify_mithril_binding` is a real cross-check, and a disagreeing manifest fails closed before any storage init.

## §5 Scope

- **RED (new):** `ade_runtime::bootstrap_from_mithril_snapshot` (new module `mithril_bootstrap.rs` or extend `genesis_bootstrap`'s sibling location). Signature shape:
  `bootstrap_from_mithril_snapshot(seed_point_inputs, seed_state_inputs, manifest_bytes, chaindb, snapshot_store, era_schedule, ledger_view) -> Result<BootstrapOutput, MithrilBootstrapError>`
  where `seed_point_inputs` carries the operator's independent `{seed_slot, seed_block_hash, network_magic, genesis_hash, seed_artifact_hash, utxo_fingerprint, initial_ledger_fingerprint}` and `manifest_bytes` is the Mithril manifest. Steps: (1) `import_mithril_manifest_from_bytes(manifest_bytes)` → `{provenance, report}`; (2) `mint(MintInputs{ seed_slot/seed_block_hash FROM operator inputs, …, seed_provenance: import.provenance })`; (3) `verify_mithril_binding(&import.report, &anchor)` — on `Err`, return fail-closed, do NOT call bootstrap; (4) on `Ok`, `bootstrap_initial_state(...)`.
- **RED:** closed `MithrilBootstrapError` { `Import(MithrilManifestError)`, `Binding(MithrilImportError)`, `Bootstrap(BootstrapError)` }.
- **Reused:** BLUE `verify_mithril_binding` + `mint`; RED `import_mithril_manifest`; GREEN `bootstrap_initial_state`.
- **CI:** new `ci/ci_check_mithril_seed_point_independence.sh`.
- **Out of scope:** any CLI flag; seed-bytes-from-Mithril decode; a reproducible fixture; live evidence; new provenance variant.

## §6 Execution Boundary (TCB color)

- **BLUE (reused, unchanged):** `verify_mithril_binding`, `MithrilManifestReport`, `MithrilImportError`, `mint`/`MintInputs`.
- **GREEN (reused):** `bootstrap_initial_state`.
- **RED (new):** `bootstrap_from_mithril_snapshot` + `MithrilBootstrapError` (composition glue; no semantic decision beyond ordering + fail-closed propagation).

Color resolved — no new BLUE authority (OI-Z.1 expectation holds).

## §7 Invariants Preserved

[[CN-MITHRIL-01]], [[DC-MITHRIL-01]] (binding predicate unchanged), [[CN-ANCHOR-01]] (mint is the sole anchor authority — the composition calls it, doesn't bypass), [[CN-NODE-01]] (single `bootstrap_initial_state`), [[CN-SEED-01]] (operator seed-point extraction is the independent source), [[T-DET-01]], and the gates `ci_check_mithril_uses_bootstrap_initial_state.sh`, `ci_check_bootstrap_anchor_closure.sh`, `ci_check_registry_code_locus_exists.sh`, `ci_check_dependency_boundary.sh`.

## §8 Invariants Strengthened or Introduced

**One family — Mithril import:**
- **Introduces `DC-MITHRIL-02`** — *For Mithril bootstrap, the `BootstrapAnchor` `seed_point` MUST be derived from the operator-provided independent seed-point extraction inputs, not from the Mithril manifest. The Mithril manifest may populate provenance and attestation fields, but the binding check MUST compare two structurally independent origins and fail closed on mismatch.*
- **Strengthens [[CN-MITHRIL-01]]** — the binding is now exercised in a production composition (`bootstrap_from_mithril_snapshot`), not only the predicate's unit tests; fail-closed-before-storage-init is verified end-to-end.
- **Advances [[RO-MITHRIL-IMPORT-01]]** — closes open-obligation item (b) (wired production site + independence gate); status **stays `partial`** (items (a) seed-bytes-from-Mithril and (c) reproducible fixture + live evidence remain open).

## §9 Design Summary

The composition mints the anchor with `seed_point` from the operator inputs and
`seed_provenance` from the manifest, then cross-checks. Because the `seed_point`
(operator) and `report.certified_point` (manifest) come from different parameters,
`verify_mithril_binding`'s `report.certified_point == anchor.seed_point` check is a
genuine agreement test: the operator must have extracted the seed at the point Mithril
attests. A manifest that disagrees fails closed. The gate forbids any wiring that would
collapse the two origins (e.g. `seed_slot: import.report.certified_point.slot`).

## §10 Changes Introduced

- **Types:** closed `MithrilBootstrapError`.
- **Functions:** `bootstrap_from_mithril_snapshot` (RED composition).
- **State transitions / persistence:** none new (reuses mint + bootstrap_initial_state).
- **CI:** `ci_check_mithril_seed_point_independence.sh`.

## §11 Replay / Crash / Epoch Validation

No new replay corpus / canonical types. `verify_mithril_binding` is already pure
(DC-MITHRIL-01); the composition adds deterministic ordering + fail-closed propagation
only. No crash/epoch surface.

## §12 Mechanical Acceptance Criteria

- [ ] `mithril_bootstrap_verifies_before_storage_init` — proves `verify_mithril_binding` runs and must be `Ok` before `bootstrap_initial_state` (call-order).
- [ ] `mithril_bootstrap_fails_closed_on_seed_point_mismatch` — operator seed-point `P` ≠ manifest `certified_point Q` → `MithrilBootstrapError::Binding(CertifiedPointMismatch)`; storage store stays empty (no `bootstrap_initial_state` side effect).
- [ ] `mithril_bootstrap_succeeds_when_seed_point_matches` — agreement → bootstrap proceeds; anchor records `SeedProvenance::Mithril`.
- [ ] `ci/ci_check_mithril_seed_point_independence.sh` — (a) `verify_mithril_binding` precedes `bootstrap_initial_state` in the composition; (b) `seed_slot`/`seed_block_hash` in the composition are NOT assigned from `import.report.*`, `.certified_point`, or `SeedProvenance::Mithril` fields (negative grep); passes.
- [ ] `cargo test -p ade_runtime -p ade_ledger` clean; carry-forward gates green (`ci_check_mithril_uses_bootstrap_initial_state.sh`, `ci_check_bootstrap_anchor_closure.sh`, `ci_check_registry_code_locus_exists.sh`, `ci_check_dependency_boundary.sh`).

## §13 Failure Modes

Closed `MithrilBootstrapError`: `Import` (malformed manifest), `Binding` (any field mismatch — fail-fast, no storage init), `Bootstrap` (downstream). All deterministic; the binding failure occurs before any authoritative write.

## §14 Hard Prohibitions

**Inherited (cluster §7).** **Slice-specific:** the anchor `seed_point` MUST come from the operator seed-point inputs, never the manifest (DC-MITHRIL-02); `verify_mithril_binding` MUST precede `bootstrap_initial_state`; no CLI flag; no new authority; no new provenance variant; no `String`/`anyhow`/float/`HashMap`/clock in touched BLUE.

## §15 Explicit Non-Goals

No `--mithril-manifest` CLI; no seed-bytes-from-Mithril; no fixture/live evidence; no historical replay; no operator runbook (future PHASE4-N-Z+1 / runbook cluster).

## §16 Completion Checklist

- [ ] Composition routes through the single bootstrap authority; verify-before-bootstrap; seed_point from operator inputs only; negative + positive + call-order tests pass; independence gate green.

## §17 Review Notes

Risk: a future edit collapsing the two origins (re-tautologizing) → the source-origin grep gate + the mismatch test guard it. Risk: implementer adds a CLI flag → §7/§15 forbid it this slice. OI-Z.1: halt if a second authority appears.

## §18 Authority Reminder

Planning aid only; registry + CI authoritative.
