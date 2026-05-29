# Invariant Slice — S1: Mithril import authority (Ade-side consume + verify + bind)

> **Scope decisions (locked, 2026-05-29):**
> - **OI-Y.1 → resolved.** STM multisig verification lives in the RED mithril-client
>   (documented acquisition infra); BLUE only binds content + recomputes the seed
>   artifact hash + records the cert hash as provenance. Mithril is **never** a BLUE
>   trust root ([[feedback-mithril-is-peer-infra-not-ade-authority]]).
> - **OI-S1.1 → (A) provenance-binding only.** Ade keeps importing the decoded seed via
>   the documented path; S1 binds a verified Mithril certificate's certified point to
>   that seed artifact (cert hash → anchor provenance, fail-closed on mismatch).
>   `RO-MITHRIL-IMPORT-01` moves declared→**partial** (provenance, not yet seed-source).
>   Seed-from-Mithril (option B) is a follow-on gated on a one-time artifact-type spike
>   + forward-replay (out of this cluster).

## §2 Slice Header

- **Slice Name:** Mithril import authority — Ade-side snapshot consumption, verification, and seed-provenance binding into `BootstrapAnchor`.
- **Cluster:** PHASE4-N-Y — Mithril-Anchored Bootstrap, Network Forward-Sync & WAL Recovery.
- **Status:** Merged.
- **Cluster Exit Criteria Addressed** (verbatim from `cluster.md` §5):
  - [ ] **CE-Y-2.** Mithril binding is a pure deterministic predicate over the closed field-set; tests `mithril_anchor_binding_is_deterministic` + `mithril_anchor_rejects_field_mismatch` (each field flipped → fail-closed) pass.
  - [ ] **CE-Y-3.** Mithril-sourced state enters **only** via `bootstrap_initial_state`; grep gate `ci_check_mithril_uses_bootstrap_initial_state.sh` (positive `bootstrap_initial_state(`; negative: no second storage-init path; no `trait *Anchor`).
  - [ ] **CE-Y-4.** Storage does not initialize on an unverified/mismatched anchor; test `mithril_import_fail_closed_blocks_storage_init` passes.
  - [ ] **CE-Y-15** *(partial — S1 portion only):* `CN-MITHRIL-01`/`DC-MITHRIL-01` introduced; `RO-MITHRIL-IMPORT-01` declared→**partial**; `CN-ANCHOR-01`/`DC-ANCHOR-01`/`CN-SEED-01` `strengthened_in += "PHASE4-N-Y"`.
  - *(CE-Y-5..14, 16, 17 are out of scope for S1.)*
- **Slice Dependencies:** none (builds only on already-merged `bootstrap_anchor`, `seed_import`, `bootstrap_initial_state`).

## §3 Implementation Instruction (AI)

Implement exactly what §5/§9/§10 specify. Do not add seed-from-Mithril decode, forward-replay, or any second bootstrap path. Do not re-implement STM multisig verification anywhere under BLUE crate paths. If the Mithril artifact's certified-point/immutable-range field names are ambiguous at implementation time, stop and ask — do not guess. §12 is the only proof of completion. Commit messages carry the project model-attribution trailer; no AI-authoring language in source comments.

## §4 Intent

Make it impossible to bootstrap from a Mithril-sourced seed **without** a verified binding between the Mithril certificate's certified chain point and the Ade-recomputed seed artifact — recorded as closed provenance in the `BootstrapAnchor`. The Mithril STM multisig is verified by the RED mithril-client (acquisition infra), **never** trusted as a BLUE authority; Ade's authoritative act is the deterministic content-binding + fail-closed predicate.

## §5 Scope

- **Modules / crates:**
  - **BLUE** — `ade_ledger::bootstrap_anchor` (extend the closed `BootstrapAnchor` with a closed `SeedProvenance` enum; schema bump `SCHEMA_VERSION 1→2`, additive + version-gated); the BLUE binding predicate `verify_mithril_binding`.
  - **RED** — new `ade_runtime::mithril_import` shell (invoke/parse the mithril-client-verified snapshot + certificate; extract certificate hash, certified point, immutable range, genesis hash, network magic).
  - **RED (CE/wiring)** — `ade_runtime::bootstrap_anchor::mint` (`MintInputs` gains the provenance) and the `bootstrap_initial_state` call site consume the result.
- **State machines affected:** none.
- **Persistence impact:** `BootstrapAnchor` canonical CBOR schema `1→2` (additive, version-gated decode). No WAL change.
- **Network-visible impact:** none.
- **Out of scope (explicit):** forward-sync (S2), recovery (S3), Conway genesis (S4), differential evidence (S5), any historical/ImmutableDB replay, seed-bytes-from-Mithril decode (OI-S1.1 option B).

## §6 Execution Boundary (TCB color)

- **BLUE:** `ade_ledger::bootstrap_anchor::{anchor, encode/decode}` (extended closed type + the binding predicate); `ade_crypto` (Blake2b recompute of `seed_artifact_hash`).
- **GREEN:** none new (the `bootstrap_initial_state` authority is reused — CE locus, not law).
- **RED:** `ade_runtime::mithril_import` (mithril-client invocation/parse, file I/O); `ade_runtime::bootstrap_anchor::mint` (`MintInputs` extension); the `ade_node` bootstrap call site.

Color is fully resolved. The STM multisig verification stays in the RED mithril-client; BLUE verifies **content-binding** {network magic, genesis hash, certified point, era, immutable range} + **recomputes** `seed_artifact_hash`; it records the cert hash as provenance but never treats the STM signature as a BLUE trust root. CI gate includes a negative grep: no `mithril`/STM-verify import under BLUE crate paths.

## §7 Invariants Preserved

[[CN-ANCHOR-01]] (anchor remains the sole closed provenance record), [[DC-ANCHOR-01]] (canonical CBOR, version-gated decode — `bootstrap_anchor_decode_rejects_unknown_version` still passes), [[CN-SEED-01]] (the cardano-cli JSON seed path unchanged), [[CN-NODE-01]] (single bootstrap authority), [[T-DET-01]] (determinism), [[CN-WAL-01]]/[[DC-WAL-01]]..[[DC-WAL-03]] (WAL untouched), and the BLUE forbidden-pattern + `ci_check_dependency_boundary.sh` / `ci_check_hash_uses_wire_bytes.sh` gates.

## §8 Invariants Strengthened or Introduced

**One family — Mithril import** (the only family this slice strengthens):
- **Introduces `CN-MITHRIL-01`** — a Mithril-sourced seed may bootstrap only after the certified point + immutable range + genesis hash + network magic bind to the anchor and the recomputed `seed_artifact_hash` matches; mismatch fails closed before storage init.
- **Introduces `DC-MITHRIL-01`** — the binding predicate is a pure deterministic function over the closed field-set (no I/O, no clock); the cert hash is recorded as closed `SeedProvenance`, not consumed as a trust root.
- Side-effect strengthenings (recorded via `strengthened_in`, not the slice's family): [[CN-ANCHOR-01]], [[DC-ANCHOR-01]] (schema extended + provenance variant), [[CN-SEED-01]] (Mithril as an alternate seed-provenance source). [[RO-MITHRIL-IMPORT-01]] declared→**partial**.

## §9 Design Summary

A closed `SeedProvenance` enum makes the seed's origin explicit and illegal states unrepresentable:

```text
enum SeedProvenance {
    CardanoCliJson,
    Mithril { certificate_hash: Hash32, certified_point: SeedPoint, immutable_range: (u64, u64) },
}
```

`BootstrapAnchor` gains `seed_provenance: SeedProvenance`; `encode_bootstrap_anchor` bumps to `SCHEMA_VERSION 2` (additive 8th field), decode rejects unknown versions (existing `bootstrap_anchor_decode_rejects_unknown_version` semantics extend to reject v3+). The BLUE binding predicate `verify_mithril_binding(provenance, anchor_fields, recomputed_seed_artifact_hash) -> Result<(), MithrilImportError>` is pure and total. The RED `mithril_import` shell shells out to the verified mithril-client and parses its output into the provenance fields — it performs no semantic decision.

## §10 Changes Introduced

- **Types:** new closed `SeedProvenance` enum; new closed `MithrilImportError` enum; `BootstrapAnchor` +1 field (`seed_provenance`); `MintInputs` +provenance.
- **State transitions:** none (pure binding predicate).
- **Persistence:** anchor CBOR `v1→v2` (additive, version-gated).
- **Removal/refactors:** none.

## §11 Replay / Crash / Epoch Validation

- **Replay tests added/updated:** `bootstrap_anchor_round_trips_via_canonical_cbor` extended for v2; `bootstrap_anchor_encode_two_runs_byte_identical` covers the new field; `bootstrap_anchor_decode_rejects_unknown_version` still passes (rejects v3+). New `mithril_anchor_binding_is_deterministic` (two-run byte-identical minted v2 anchor + predicate verdict).
- **Crash/restart:** `mithril_import_fail_closed_blocks_storage_init` proves a mismatched binding halts before any storage write (no partial state).
- **Epoch boundary:** not applicable.

## §12 Mechanical Acceptance Criteria

- [ ] `mithril_anchor_binding_is_deterministic` — pure predicate; two runs byte-identical.
- [ ] `mithril_anchor_rejects_field_mismatch` — each of {network magic, genesis hash, certified point, immutable range, seed_artifact_hash} flipped → distinct `MithrilImportError` variant, fail-closed.
- [ ] `mithril_import_fail_closed_blocks_storage_init` — mismatch → no `bootstrap_initial_state` storage init.
- [ ] `bootstrap_anchor_v2_round_trips_and_rejects_unknown_version` — v2 round-trips; v3+ rejected.
- [ ] `ci/ci_check_mithril_uses_bootstrap_initial_state.sh` — positive `bootstrap_initial_state(`; negatives: no `trait *Anchor`, no second storage-init path, no `mithril`/STM-verify import under BLUE crate paths.
- [ ] `cargo test --workspace` clean; carry-forward gates pass (`ci_check_bootstrap_anchor_closure.sh`, `ci_check_seed_import_closure.sh`, `ci_check_hash_uses_wire_bytes.sh`, `ci_check_dependency_boundary.sh`).

## §13 Failure Modes

Closed `MithrilImportError` {`NetworkMagicMismatch`, `GenesisHashMismatch`, `CertifiedPointMismatch`, `ImmutableRangeMismatch`, `SeedArtifactHashMismatch`, `UnsupportedArtifactType`}. All **fail-fast** (affect bootstrap/anchor → halt deterministically; never partial). No replay impact (failure occurs before any authoritative write). Error variants carry only non-secret primitives.

## §14 Hard Prohibitions

**Inherited (cluster §7):** no `GenesisAnchor`/`MithrilAnchor` trait or plugin seam; no cardano-node storage-layout mimicry; no re-encoding hash-critical bytes; no storage init before a verified anchor; no implementation-fact promotion to law; no full-N2N/N2C claim.

**Slice-specific:**
- **No STM multisig verification re-implemented in BLUE**; Ade never treats the Mithril STM cert as a BLUE trust root ([[feedback-mithril-is-peer-infra-not-ade-authority]]).
- **No Mithril substitution** for `admit_via_block_validity` or for the BLUE seed/anchor authority — Mithril only *adds provenance*.
- **No claim of "Ade supports Mithril"** from a peer-acceleration pass; closure requires Ade-side consume+verify+bind here.
- **No seed-bytes-from-Mithril decode and no historical/ImmutableDB replay** (OI-S1.1 option B is out of scope).
- No `String`/`anyhow`/formatted errors, `HashMap`/`HashSet`, wall-clock, or float in the BLUE binding predicate.

## §15 Explicit Non-Goals

No forward-sync, recovery, Conway genesis, or differential harness (S2–S5). No seed decode from a Mithril artifact. No new protocol versions. No performance work. No feature flags. No preparation for behavior not required here.

## §16 Completion Checklist

- [ ] All new state is replay-derivable (anchor v2 round-trips).
- [ ] All new data is canonically encoded (version-gated CBOR).
- [ ] All failure modes are deterministic (closed `MithrilImportError`, fail-fast).
- [ ] No TODOs/placeholders in the BLUE binding predicate.
- [ ] CI enforces the strengthened invariant (`ci_check_mithril_uses_bootstrap_initial_state.sh` + the named tests).
- [ ] Replay-equivalence tests pass across runs.

## §17 Review Notes

- Invariant risk considered: that Mithril provenance silently becomes a trust root — mitigated by the BLUE/RED split + the negative CI grep.
- Assumption challenged: "Mithril replaces the seed source" — narrowed to provenance-binding (OI-S1.1=A); seed-from-Mithril is a separate gated follow-on.
- Follow-up implied: S2 (forward-sync) consumes the anchor produced here; the seed-from-Mithril spike (option B) if/when prioritized.

## §18 Authority Reminder

Planning/review aid only. Correctness is defined by the invariant registry + CI. On conflict: normative documents + CI enforcement win.
