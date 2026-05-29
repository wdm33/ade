# Invariant Slice — S7: Real (non-tautological) Mithril binding (HIGH remediation)

> Remediates the cluster-close security BLOCK #2. S1's `verify_mithril_binding`
> compared an "observed" set and a `reported_*` set that the importer populated
> from the **same** manifest values — so every check compared a value to itself and
> could never fail. The "verified binding" (CN-MITHRIL-01 / CE-Y-2) was a structural
> shell. This slice makes the binding cross-check the Mithril manifest against an
> **independent** source: the `BootstrapAnchor` minted from the operator's
> `--json-seed` + genesis (a genuinely different origin than the Mithril cert).

## §2 Slice Header

- **Slice Name:** Mithril binding cross-checks the manifest against the independently-minted seed anchor — same-value-vs-itself tautology removed.
- **Cluster:** PHASE4-N-Y.
- **Status:** Proposed.
- **Cluster Exit Criteria Addressed:** makes CE-Y-2 real (the binding can now actually fail-closed); unblocks CN-MITHRIL-01/DC-MITHRIL-01 → enforced.
- **Slice Dependencies:** S1 (merged).

## §4 Intent

Make it impossible to bind a Mithril certificate to a seed it does not actually attest: the binding MUST verify the Mithril cert's attested `{network_magic, genesis_hash, certified_point}` equal the `--json-seed`-minted `BootstrapAnchor`'s independently-derived `{network_magic, genesis_hash, seed_point}`. The two sides come from different origins (the Mithril cert vs the operator's seed extraction); a mismatch (e.g. a snapshot certified at a different chain point than the seed dump) fails closed.

## §5 Scope

- **BLUE** — `ade_ledger::bootstrap_anchor::binding`: replace the tautological `MithrilAnchorFields` (observed + duplicated `reported_*`) with a single `MithrilManifestReport { network_magic, genesis_hash, certificate_hash, certified_point, immutable_range }` (the manifest-reported side only). Re-shape `verify_mithril_binding(report: &MithrilManifestReport, anchor: &BootstrapAnchor) -> Result<(), MithrilImportError>` to compare the report against the anchor's independent fields. Add a `CertificateHashMismatch` variant (fixes the S1 WARN where a cert-hash mismatch returned `CertifiedPointMismatch`); the `provenance` recorded on the anchor must carry `report.certificate_hash`.
- **RED** — `ade_runtime::mithril_import::importer`: produce `MithrilManifestReport` from the parsed manifest (no fabricated duplicate side); the `SeedProvenance::Mithril` recorded on the anchor is built from the manifest. The binding is invoked at the composition point where BOTH the manifest report AND the `--json-seed`-minted anchor exist.
- **Out of scope:** any seed-bytes-from-Mithril decode (OI-S1.1 option B); a seed_artifact_hash-vs-Mithril check (Mithril attests a chain point, not Ade's canonical seed digest — there is no independent Mithril counterpart to compare it to, so the prior caller-supplied `seed_artifact_hash` tautology is REMOVED, not replaced with another fake).

## §6 Execution Boundary (TCB color)

- **BLUE:** `verify_mithril_binding` + `MithrilManifestReport` + `MithrilImportError` (closed; pure; no I/O/clock).
- **RED:** the importer that parses the manifest into the report; the composition call site.

No new BLUE authority; STM verification stays in the RED mithril-client (unchanged).

## §7 Invariants Preserved

[[CN-ANCHOR-01]]/[[DC-ANCHOR-01]] (anchor shape + v2 provenance unchanged), [[CN-SEED-01]] (the `--json-seed` anchor is the independent source), [[T-DET-01]] (binding is pure/deterministic), and the S1 gate `ci_check_mithril_uses_bootstrap_initial_state.sh` (no STM/mithril import under BLUE; no plugin seam) must stay green.

## §8 Invariants Strengthened or Introduced

**One family — Mithril import.** Makes [[CN-MITHRIL-01]] + [[DC-MITHRIL-01]] **genuinely enforced** (the binding now cross-checks two independent sources and can fail-closed). No new rule ID; this realizes the rules S1 introduced as shells.

## §9 Design Summary

`verify_mithril_binding`:
- `report.network_magic == anchor.network_magic` else `NetworkMagicMismatch`
- `report.genesis_hash == anchor.genesis_hash` else `GenesisHashMismatch`
- `report.certified_point == anchor.seed_point` else `CertifiedPointMismatch` *(the load-bearing cross-check: the Mithril cert's attested point vs the point the seed was extracted at — independent origins)*
- the anchor's `seed_provenance` is `SeedProvenance::Mithril { certificate_hash, .. }` with `certificate_hash == report.certificate_hash` else `CertificateHashMismatch`
- `immutable_range` is recorded as provenance (no independent counterpart; not a tautological self-check).

The importer no longer fabricates a duplicated side; the only inputs are the manifest (→ report + provenance) and the independently-minted anchor.

## §10 Changes Introduced

- **Types:** `MithrilAnchorFields` (with `reported_*` dup) → `MithrilManifestReport`; add `MithrilImportError::CertificateHashMismatch`.
- **Signature:** `verify_mithril_binding(report, anchor)` (was `(provenance, fields, anchor_seed_artifact_hash, recomputed_seed_artifact_hash)`); the caller-supplied seed-hash tautology removed.
- **Persistence:** none (anchor v2 shape unchanged).

## §11 Replay / Crash / Epoch Validation

- **Determinism:** `mithril_anchor_binding_is_deterministic` (two runs, same verdict).
- No crash/epoch surface.

## §12 Mechanical Acceptance Criteria

- [ ] `mithril_binding_rejects_certified_point_other_than_seed_point` — a manifest certified at a point ≠ the `--json-seed` anchor's `seed_point` → `CertifiedPointMismatch` (the real cross-check; this is the test that FAILS under the old tautological code and PASSES now).
- [ ] `mithril_anchor_rejects_field_mismatch` — each of {network_magic, genesis_hash, certified_point, certificate_hash} differing between the manifest report and the independently-minted anchor → its distinct `MithrilImportError` variant.
- [ ] `mithril_anchor_binding_is_deterministic` — pure, two runs identical.
- [ ] `mithril_import_fail_closed_blocks_storage_init` — still passes (mismatch → no storage init).
- [ ] `bootstrap_anchor_v2_round_trips_and_rejects_unknown_version` — still passes.
- [ ] `ci_check_mithril_uses_bootstrap_initial_state.sh` stays green; `cargo test -p ade_ledger bootstrap_anchor` + `-p ade_runtime mithril` clean.

## §13 Failure Modes

Every field divergence between the Mithril manifest and the independently-minted anchor → a distinct closed `MithrilImportError` variant, fail-fast before storage init. No silent accept; no tautological pass.

## §14 Hard Prohibitions

**Inherited (cluster §7).** **Slice-specific:** the binding MUST cross-check two independent sources (manifest vs `--json-seed` anchor) — no comparing a value to itself; Mithril STM cert is never a BLUE trust root; no fabricated/duplicated comparison side; no fake seed-hash self-check; pure BLUE predicate (no I/O/clock/String-errors).

## §15 Explicit Non-Goals

No seed-from-Mithril decode (option B), no forward-replay, no new provenance variant, no protocol surface, no performance work.

## §16 Completion Checklist

- [ ] Tautology removed; binding cross-checks manifest vs independently-minted anchor; distinct error per field incl. CertificateHashMismatch; deterministic; S1 gate green; CN-MITHRIL-01/DC-MITHRIL-01 now genuinely fail-closeable.

## §18 Authority Reminder

Planning aid only; registry + CI authoritative.
