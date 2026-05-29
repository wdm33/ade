# PHASE4-N-Z — Mithril production-bootstrap wiring + seed-point independence (cluster doc)

> **Status:** Planning. **Single slice (S1).** Narrow follow-on closing the
> re-tautologization hazard the PHASE4-N-Y S7 security re-review flagged on
> `RO-MITHRIL-IMPORT-01`: the Mithril binding only verifies anything if the two
> compared origins are structurally independent. This cluster adds the production
> composition authority that mints the anchor from **operator-provided seed-point
> inputs** (independent of the Mithril manifest) and gates that independence.
>
> **Predecessor:** PHASE4-N-Y (HEAD `5db9aae`).
> **Process note:** `/invariants` and `/cluster-plan` are deliberately skipped — the
> invariant is already pinned in `RO-MITHRIL-IMPORT-01`'s open obligation; this is a
> single well-understood follow-on, not a new planning cluster. **Composition-only:
> no CLI flag** (mirrors `bootstrap_from_conway_genesis`, which is also composition-only).

## §1 Primary invariant

> For a Mithril-sourced bootstrap, the `BootstrapAnchor`'s top-level `seed_point` is
> derived from the **operator-provided independent seed-point extraction inputs**, never
> from the Mithril manifest. The Mithril manifest may populate the anchor's provenance
> and attestation fields (`SeedProvenance::Mithril`), but `verify_mithril_binding` MUST
> compare two structurally independent origins — the manifest's attested
> `certified_point` against the anchor's independently-supplied `seed_point` — and the
> bootstrap MUST fail closed (no `bootstrap_initial_state`, no storage init) on
> mismatch. Mithril proves a point; it must not be allowed to define both sides of the
> comparison.

Load-bearing guarantees:

1. **Independent origins (DC-MITHRIL-02, new).** The composition mints
   `anchor.seed_slot` / `anchor.seed_block_hash` from the operator seed-point inputs;
   it MUST NOT source them from the manifest import (`import.report.*`,
   `manifest.certified_point`, or `SeedProvenance::Mithril` fields).
2. **Verify-before-bootstrap (CN-MITHRIL-01, strengthened).** `verify_mithril_binding`
   is called and must return `Ok` *before* `bootstrap_initial_state`; a mismatch
   (`MithrilImportError`) halts deterministically with no storage init.
3. **Single closed bootstrap authority (carried).** The Mithril path routes through the
   same `bootstrap_initial_state` (`CN-NODE-01`); no parallel storage-init path, no
   `*Anchor` trait / plugin seam — exactly as `bootstrap_from_conway_genesis`.
4. **Provenance vs. independent field (clarified).** `anchor.seed_provenance` is the
   Mithril variant carrying the manifest's `{certificate_hash, certified_point,
   immutable_range}` (correctly *recording* what Mithril attested); the **independent**
   field the binding cross-checks is the top-level `anchor.seed_point`.

**Non-claims (honest scope).** This cluster does **not** ship a CLI surface
(`--mithril-manifest` is a future operator-UX slice), does **not** decode seed bytes
*from* a Mithril artifact (OI-S1.1 option B), and does **not** commit a reproducible
Mithril fixture or live evidence. `RO-MITHRIL-IMPORT-01` therefore advances but stays
`partial`.

## §2 Normative anchors

- [[RO-MITHRIL-IMPORT-01]] (open-obligation item (b): wired production site +
  independence gate), [[CN-MITHRIL-01]], [[DC-MITHRIL-01]] (the binding predicate),
  [[CN-ANCHOR-01]] (sole anchor mint), [[CN-NODE-01]] (single bootstrap authority),
  [[CN-SEED-01]] (the operator seed-point extraction is the independent source).
- PHASE4-N-Y S7 security re-review (the re-tautologization hazard this closes).

## §3 Entry conditions (already enforced)

- `verify_mithril_binding(report, anchor)` (BLUE, pure) cross-checks the manifest report
  vs the anchor's independent fields (PHASE4-N-Y S7).
- `import_mithril_manifest` (RED) → `MithrilProvenanceImport { provenance, report }`.
- `mint(MintInputs{ seed_slot, seed_block_hash, …, seed_provenance })` is the sole anchor
  authority (CN-ANCHOR-01); the cardano-cli path supplies `seed_slot`/`seed_block_hash`
  from operator seed-point inputs.
- `bootstrap_from_conway_genesis` is the composition-only precedent this slice mirrors.

## §4 Slice index

| Slice | Purpose | Strengthens | Introduces |
|---|---|---|---|
| **S1 — Mithril production bootstrap + independence gate** | New RED `bootstrap_from_mithril_snapshot` composition: mint anchor (seed_point from operator inputs; provenance from manifest) → `verify_mithril_binding` fail-closed → `bootstrap_initial_state`. New CI gate enforcing call-order + source-origin independence + a mandatory fail-closed negative test. | [[CN-MITHRIL-01]], [[CN-ANCHOR-01]], [[CN-NODE-01]]; advances [[RO-MITHRIL-IMPORT-01]] (→ stays `partial`) | **DC-MITHRIL-02** |

## §5 Exit criteria (CI-verifiable)

- [ ] **CE-Z-1.** `docs/clusters/PHASE4-N-Z/{cluster,S1-*}.md` exist.
- [ ] **CE-Z-2.** `bootstrap_from_mithril_snapshot` exists (composition-only, mirrors `bootstrap_from_conway_genesis`); routes through `bootstrap_initial_state`; introduces no `*Anchor` trait / parallel storage-init path.
- [ ] **CE-Z-3 (call-order).** `verify_mithril_binding(...)` is invoked and must be `Ok` **before** `bootstrap_initial_state(...)`; test `mithril_bootstrap_verifies_before_storage_init`.
- [ ] **CE-Z-4 (negative, mandatory).** Operator seed-point `P` ≠ manifest `certified_point Q` → `CertifiedPointMismatch` → **no** `bootstrap_initial_state`, **no** storage init; test `mithril_bootstrap_fails_closed_on_seed_point_mismatch`.
- [ ] **CE-Z-5 (positive).** Matching operator seed-point + manifest → bootstrap proceeds; test `mithril_bootstrap_succeeds_when_seed_point_matches`.
- [ ] **CE-Z-6 (source-origin gate).** `ci/ci_check_mithril_seed_point_independence.sh`: (a) the composition calls `verify_mithril_binding` before `bootstrap_initial_state`; (b) `anchor.seed_slot`/`seed_block_hash` in the composition are sourced from the operator seed-point inputs, **not** from `import.report.*`, `manifest.certified_point`, or `SeedProvenance::Mithril` fields.
- [ ] **CE-Z-7.** Registry: `DC-MITHRIL-02` introduced `enforced`; `CN-MITHRIL-01` `strengthened_in += "PHASE4-N-Z"`; `RO-MITHRIL-IMPORT-01` open-obligation item (b) marked closed but status stays `partial` (items (a) seed-bytes + (c) fixture/live still open).
- [ ] **CE-Z-8.** `cargo test -p ade_runtime -p ade_ledger` clean (touched crates); carry-forward gates pass (`ci_check_mithril_uses_bootstrap_initial_state.sh`, `ci_check_bootstrap_anchor_closure.sh`, `ci_check_registry_code_locus_exists.sh`, `ci_check_dependency_boundary.sh`).

> No human review may substitute for these checks.

## §6 TCB color map (FC/IS partition)

- **BLUE (reused):** `ade_ledger::bootstrap_anchor::binding::verify_mithril_binding` + `MithrilManifestReport` + closed `MithrilImportError`; `bootstrap_anchor` mint authority.
- **GREEN (reused):** `ade_runtime::bootstrap::bootstrap_initial_state` (closed authority).
- **RED (new):** `ade_runtime::bootstrap_from_mithril_snapshot` composition shell (mirrors `genesis_bootstrap`); `ade_runtime::mithril_import` (reused manifest parser).

No new BLUE authority; the binding predicate + mint are unchanged. The composition is RED glue with a closed error surface.

## §7 Forbidden during this cluster (slice inherits)

- **No CLI flag** (`--mithril-manifest`) — composition-only this slice.
- **No sourcing the anchor's `seed_point` from the Mithril manifest** (the whole point — DC-MITHRIL-02).
- No second bootstrap authority / `*Anchor` trait / plugin seam.
- No seed-bytes-*from*-Mithril decode (option B); no historical replay.
- No re-encoding hash-critical bytes; no storage init before a verified binding.
- No new provenance variant (reuse `SeedProvenance::Mithril`).
- No `String`/`anyhow`/float/`HashMap`/clock in any BLUE path touched.

## §8 Replay obligations

None new. No new canonical types (the composition is RED; the anchor shape is unchanged
from N-Y S1). `verify_mithril_binding` is already pure/deterministic (DC-MITHRIL-01).

## §9 Open issues

- **OI-Z.1.** If implementation reveals the composition needs a *second* semantic
  authority (beyond reusing the binding + mint + bootstrap authorities), halt and
  reconsider whether the single-slice framing holds. Expected: pure RED composition, no
  new authority.

---

> **Authority reminder.** Planning aid only. Correctness lives in the invariant registry
> + CI enforcement. On disagreement: normative documents + CI win.
