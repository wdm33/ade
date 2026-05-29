# Ade — HEAD Deltas (Changes Since Baseline)

> **Status:** Living architectural document. Regenerated; not hand-edited.
>
> Regenerate with `/head-deltas <baseline>` after every cluster close. Baseline is recorded in `.idd-config.json` `head_deltas_baseline`.

> Baseline: `3b78008` (no tag, 2026-05-29 14:59:36 +0700)
> HEAD: `67d1ccc` (Close PHASE4-N-Z — Mithril production-bootstrap wiring + seed-point independence, 2026-05-29 16:57:41 +0700)
> 9 commits, 13 files changed, +1692 / -560 lines

This window is a short **PHASE4-N-Y post-close housekeeping tail** (`3b78008` → `5db9aae`) followed by the **PHASE4-N-Z cluster** (`5db9aae` → `67d1ccc`). The prior baseline (`3b78008`) was the PHASE4-N-Y close; per the per-cluster bump model the N-Y cluster body is now archived in git history + its cluster docs, and this doc narrates only the post-N-Y span.

The window is two pieces:

1. The **N-Y post-close tail** (`f0d0bf9`, `5db9aae`, `588a554`) — **no source-code behavior change** beyond a registry pointer repair + a new drift-guard gate. `f0d0bf9` adds the first `.github/workflows` entry (an ade-atlas notify); `5db9aae` repairs three stale `code_locus` pointers left by the N-Y `recovery.rs → recovery/mod.rs` promotion and adds `ci_check_registry_code_locus_exists.sh`; `588a554` re-reproduced the four grounding docs at `5db9aae`. This tail is narrated in **§8**.
2. The **PHASE4-N-Z cluster** (`9b4177f` scope, `f2b1562` S1, `bccec39` gate-hardening, `c876022` registry, `67d1ccc` close) — Mithril production-bootstrap wiring + seed-point independence. One new RED module, one new BLUE-call-order CI gate, one new derived rule, narrated in §§2–7.

> **Baseline bump:** the PHASE4-N-Z close is a cluster close and **does** warrant a bump of `.idd-config.json` `head_deltas_baseline` from `3b78008` to **`67d1ccc`**. The next cluster close re-bumps from there.

> **Grounding-doc coherence:** all four docs (CODEMAP, SEAMS, TRACEABILITY, HEAD_DELTAS) were regenerated together in the PHASE4-N-Z close pass at HEAD `67d1ccc` — they all reflect `ade_runtime::mithril_bootstrap`, `DC-MITHRIL-02`, and `ci_check_mithril_seed_point_independence.sh` (CI 105, registry 299). No cross-doc staleness this window. *(They were generated concurrently; an interim draft of this note flagged the other three as stale because they had not yet been written when this doc was drafted — corrected here.)*

---

## 1. Commit Log

Verbatim from `git log --oneline --no-merges 3b78008..67d1ccc`, newest-first. Type is the conventional-commits prefix on the subject; no editorial.

| Hash | Type | Summary |
|------|------|---------|
| `67d1ccc` | — | Close PHASE4-N-Z — Mithril production-bootstrap wiring + seed-point independence |
| `c876022` | docs | PHASE4-N-Z close — DC-MITHRIL-02 enforced + RO-MITHRIL-IMPORT-01 item (b) closed |
| `bccec39` | fix | harden Mithril seed-point independence gate against laundering (N-Z review BLOCK) |
| `f2b1562` | feat | PHASE4-N-Z S1 — Mithril production bootstrap + seed-point independence gate |
| `9b4177f` | docs | scope PHASE4-N-Z Mithril production-bootstrap + seed-point independence |
| `588a554` | docs | re-reproduce CODEMAP/TRACEABILITY/SEAMS/HEAD_DELTAS at 5db9aae |
| `5db9aae` | fix | repair recovery.rs code_locus drift + add code-locus existence gate |
| `f0d0bf9` | ci | notify ade-atlas to rebuild on grounding-doc changes |
| `3ddbc9a` | docs | refresh CODEMAP/TRACEABILITY/SEAMS/HEAD_DELTAS for PHASE4-N-Y |

Type histogram: docs ×4, fix ×2, feat ×1, ci ×1. Unclassified by prefix: 1 — `67d1ccc` ("Close PHASE4-N-Z …") carries no conventional-commits prefix; its diff is the cluster-close pass (registry + grounding-doc refresh + cluster-doc archive), so it is `docs`-by-scope.

(`3ddbc9a` is the N-Y close-pass grounding refresh — it sits at the very start of this window because the prior baseline `3b78008` is its predecessor; it touched only `docs/` and carries no N-Z content.)

---

## 2. New Modules

One new module this window, all PHASE4-N-Z.

| Module | Color | Purpose | Key sub-paths | Added in (cluster/slice) |
|--------|-------|---------|---------------|--------------------------|
| `ade_runtime::mithril_bootstrap` | RED | The wired **production** Mithril-snapshot bootstrap entry (S1) — closes RO-MITHRIL-IMPORT-01 item (b). A composition-only RED shell that routes a Mithril-sourced seed through the **same** single closed bootstrap authority `bootstrap_initial_state` (CN-NODE-01), never a parallel storage-init path; mirrors `genesis_bootstrap` in shape. The load-bearing discipline (DC-MITHRIL-02): the anchor's `seed_point` is minted from the **operator-provided** `MithrilSeedPointInputs`, an origin structurally independent of the manifest; the manifest import only populates `SeedProvenance::Mithril`. `verify_mithril_binding` then cross-checks the manifest's attested `certified_point` against the independently-supplied `anchor.seed_point` and fails closed **before** any `bootstrap_initial_state` call. | `mithril_bootstrap.rs` (`bootstrap_from_mithril_snapshot`, `MithrilSeedPointInputs`, closed `MithrilBootstrapError` { `Import` / `Binding` / bootstrap-authority variants }) | PHASE4-N-Z / `f2b1562` (S1), gate hardened `bccec39` |

**Cross-reference (CODEMAP @ `67d1ccc`): STALE.** CODEMAP was last regenerated at `5db9aae` (N-Y tail) and does **not** yet catalogue `ade_runtime::mithril_bootstrap`. Run `/codemap` to add it to the RED authority table alongside the sibling `genesis_bootstrap` / `mithril_import` rows.

No new corpus / non-source artifacts this window.

---

## 3. Modules Modified

Modules that existed at baseline with non-trivial changes. The N-Y tail repaired registry pointers and added a docs-notify workflow but touched no `crates/**/*.rs` behavior; the N-Z scope (`9b4177f`) is planning-only. The only source change is the new-module declaration.

| Module | Scope | Key changes |
|--------|-------|-------------|
| `ade_runtime` `lib.rs` | +1 line | **N-Z (S1):** declares the new RED submodule — `pub mod mithril_bootstrap;` (the entry itself is the new module in §2). No other `crates/**/*.rs` file changed in the window. |

### Strengthenings recorded this window (registry `strengthened_in`)

Not new rules — two cross-cutting invariant strengthenings PHASE4-N-Z carried forward (see §7):

- **`CN-MITHRIL-01`** — the verify-before-bootstrap call-order is now mechanically enforced on the wired production composition (the new gate asserts `verify_mithril_binding(` precedes `bootstrap_initial_state(`).
- **`RO-MITHRIL-IMPORT-01`** — item (b) (a wired production composition site with a CI gate asserting seed-point independence) is **closed**; the rule stays `partial` pending items (a) seed-bytes-from-Mithril decode and (c) a committed reproducible fixture + live evidence.

---

## 4. Feature Flags

No feature-flag deltas this window. **No `Cargo.toml`** (workspace root or any member) was modified between `3b78008` and `67d1ccc`, so no `[features]` table, `optionalDependencies`, build tag, or `extras_require` changed. No `compile_error!`-coupled flag was introduced or removed.

---

## 5. CI Checks

Every CI check added or materially modified since baseline. Enforcement gates live as `ci/ci_check_*.sh`. Count of `ci/ci_check_*.sh`: **103 → 105** (+2 new — 1 in the N-Y tail, 1 in N-Z; 0 modified, 0 removed). A non-gating `.github/workflows/notify-atlas.yml` was also added in the tail (§8).

### PHASE4-N-Z checks

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci_check_mithril_seed_point_independence.sh` | New (`f2b1562`, S1; hardened `bccec39`) | Mithril bootstrap seed-point independence (DC-MITHRIL-02) + verify-before-bootstrap call-order (CN-MITHRIL-01, strengthened), on the production composition `bootstrap_from_mithril_snapshot`. **(a)** Positive call-order: `verify_mithril_binding(` appears before `bootstrap_initial_state(` in source. **(b)** Negative source-origin: the `MintInputs` `seed_slot:` / `seed_block_hash:` RHS does not mention a manifest-origin token (`report`, `.certified_point`, `provenance`, `SeedProvenance::Mithril`, `import.`). **(c)** Containment (added in `bccec39`): the production body must reference the import only as whole values — any `import.report.<field>` drill, any `import.provenance.<field>` drill, or any mention of `certified_point` fails closed, so a one-hop local or mutate-before-mint cannot launder a manifest point into the seed_point path. Strips the `#[cfg(test)]` module + line comments before grepping. |

### PHASE4-N-Y tail check

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci_check_registry_code_locus_exists.sh` | New (`5db9aae`) | Registry `code_locus` drift-guard (see §8). Loads the registry with `python3` + `tomllib`, extracts every `crates/**.rs` and `ci/**.sh` token from each rule's `code_locus`, skips glob-containing tokens and `docs/` paths, and fails closed if any cited code/gate path does not exist on disk. |

**Cross-reference (TRACEABILITY @ `67d1ccc`): STALE for the N-Z gate.** TRACEABILITY was last regenerated at `5db9aae` and binds `ci_check_registry_code_locus_exists.sh` only via the N-Y-tail refresh; it does **not** yet bind `ci_check_mithril_seed_point_independence.sh` to DC-MITHRIL-02 / CN-MITHRIL-01. Run `/traceability` to add the binding. The `.github/workflows/notify-atlas.yml` workflow is *not* an invariant gate and is correctly absent from TRACEABILITY (it enforces nothing); see §8.

---

## 6. Canonical Type Registry Delta

n/a — `.idd-config.json` `canonical_type_registry` is `null`. Canonical-type rules live inline in the invariant registry under family **T**; no family-T entries were added or removed this window.

For reference, the N-Z module introduces the RED composition types `MithrilSeedPointInputs` (the operator-side independent seed-point struct) and the closed `MithrilBootstrapError` sum. These are RED-shell composition types, not BLUE canonical types, and there is no canonical-type registry file to delta against.

---

## 7. Normative / Invariant Rule Delta

Source: `docs/ade-invariant-registry.toml` (the project's canonical append-only invariant registry; `invariant_registry` in `.idd-config.json`). Counts by `^[[rules]]` entries.

- Rules at baseline (`3b78008`): **298**
- Rules at HEAD (`67d1ccc`): **299**
- Net additions: **1** (`DC-MITHRIL-02`, introduced and enforced inside the N-Z cluster body)
- Removals: **0** (append-only discipline upheld).

### New rule

| ID | Tier | Cluster | One-line summary |
|----|------|---------|------------------|
| `DC-MITHRIL-02` | derived | N-Z | For Mithril bootstrap, the `BootstrapAnchor` `seed_point` MUST be derived from the operator-provided independent seed-point extraction inputs, **not** from the Mithril manifest. The manifest may populate provenance/attestation fields (`SeedProvenance::Mithril`), but `verify_mithril_binding` MUST compare two structurally independent origins and fail closed on mismatch. In the production composition the manifest import may be referenced only as whole values (`import.provenance` → `seed_provenance`; `&import.report` → the verify call); the import's point-bearing fields must never be drilled into or laundered (via a local binding or a mutate-before-mint) into the anchor's `seed_point`. Status `enforced` (S1, gate hardened by the review BLOCK remediation). |

### Status flips / closures (release obligation)

- **RO-MITHRIL-IMPORT-01** — stays `status: partial`; `strengthened_in: ["PHASE4-N-Y"] → ["PHASE4-N-Y", "PHASE4-N-Z"]`. Item **(b)** (a wired production composition site with a CI gate asserting seed-point independence) is now **CLOSED** by `bootstrap_from_mithril_snapshot` + DC-MITHRIL-02 + the data-flow-resistant containment gate. `open_obligation` rewritten to `blocked_until_mithril_seed_bytes_and_fixture`; remaining for `enforced`: (a) seed-bytes-from-Mithril decode (option B) — needs a Mithril artifact-type spike + forward-replay; (c) a committed reproducible Mithril fixture + CI/release evidence.

### Modified rule (strengthening)

- **`CN-MITHRIL-01`** — `strengthened_in: [] → ["PHASE4-N-Z"]`; no statement weakened. The verify-before-bootstrap call-order it requires is now mechanically asserted on the wired production composition by `ci_check_mithril_seed_point_independence.sh` guard (a). (`DC-MITHRIL-01` is also cross-referenced by the new rule but its own statement/strengthened_in is unchanged this window.)

### Security-HIGH finding caught at cluster-close and remediated (anomaly worth recording)

Surfaced by the per-cluster (per-slice) security review against the S1 diff and fixed before close:

- **`bccec39` — laundering-bypassable independence gate (IDD-review BLOCK).** The first cut of `ci_check_mithril_seed_point_independence.sh` (guard (b)) only inspected the literal RHS of the `seed_slot` / `seed_block_hash` assignment lines. A one-hop local (`let q = import.report.certified_point.slot; … seed_slot: q,`) or a mutate-before-mint would re-collapse the two origins into a value-vs-itself comparison while guard (b) stayed green — re-introducing exactly the tautological-binding class that the N-Y S7 HIGH remediation had closed at the code level. The remediation hardened the gate with guard (c) **containment**: in the production composition the manifest import may be referenced *only* as whole values; any `import.report.<field>` drill, `import.provenance.<field>` drill, or mention of `certified_point` fails the gate. The laundering class is now structurally CI-blocked, not merely line-local. This is the `attack_rationale` of DC-MITHRIL-02.

### Honest residual

N-Z proves, in-process and mechanically: the Mithril-sourced seed enters the single closed bootstrap authority, the anchor's seed_point originates from an operator-independent origin (not the manifest), the binding verifies before storage init, and the laundering class is gated. It does **not** prove seed-bytes-from-Mithril decode (option B) nor commit a reproducible Mithril fixture / live evidence — both remain in `RO-MITHRIL-IMPORT-01` (`partial`, `blocked_until_mithril_seed_bytes_and_fixture`). Neither is a code gap in this cluster's shipped scope.

---

## 8. Post-Close Tail (`3b78008` → `5db9aae`)

**Not a cluster.** Three housekeeping commits after the PHASE4-N-Y close — a CI notify workflow, a registry drift-guard, and a grounding-doc re-reproduce. No source-code (`crates/**/*.rs`) behavior change, no new invariant rule.

| Hash | Type | Summary |
|------|------|---------|
| `588a554` | docs | `docs(grounding):` re-reproduce CODEMAP/TRACEABILITY/SEAMS/HEAD_DELTAS at 5db9aae |
| `5db9aae` | fix | `fix(registry):` repair recovery.rs code_locus drift + add code-locus existence gate |
| `f0d0bf9` | ci | `ci:` notify ade-atlas to rebuild on grounding-doc changes |

### `f0d0bf9` — CI notify workflow (first `.github/workflows` in the repo)

Adds `.github/workflows/notify-atlas.yml` (+42 lines) — the **first** GitHub Actions workflow in this repo, which until now had only `ci/ci_check_*.sh` scripts (`.idd-config.json` `ci_dirs` is still `["ci"]`; this workflow is outside that list). On a push to `main` that touches any of the five grounding artifacts (`ade-{CODEMAP,SEAMS,HEAD_DELTAS,TRACEABILITY}.md` + `ade-invariant-registry.toml`), or on `workflow_dispatch`, it `repository_dispatch`-es an `ade-docs-updated` event to `wdm33/ade-atlas` so the dashboard rebuilds. A **clean no-op** until the `ATLAS_DISPATCH_TOKEN` secret is set; `ade-atlas` also has a daily cron fallback. Permissions are `contents: read`; it gates nothing in this repo (it is a notify, not a check).

> **§5 cross-reference note:** this workflow is *not* an invariant gate and is *not* referenced by any TRACEABILITY rule — by design (it enforces nothing). When `.github/workflows` is added to `.idd-config.json` `ci_dirs`, the CI-check inventory tooling should continue to classify `notify-atlas.yml` as a non-gating workflow, not an enforcement script.

### `5db9aae` — registry code_locus drift repair + existence gate

Two coupled changes, **0 new rules** (registry stays **298** at this point in the window):

1. **Repaired three stale `code_locus` pointers** in `docs/ade-invariant-registry.toml` left behind by the N-Y S3 `recovery.rs → recovery/mod.rs` directory promotion. All three pointed at the now-nonexistent `crates/ade_runtime/src/recovery.rs`:
   - **`T-REC-01`** (true; recovery replay-equivalence) → now `recovery/mod.rs, recovery/restart.rs, chaindb/crash_safety.rs`.
   - **`T-REC-02`** (true; all authoritative state derivable by replay) → now `recovery/mod.rs, recovery/restart.rs`.
   - **`DC-STORE-05`** (derived; recovery is snapshot + forward replay, not full genesis replay) → now `recovery/mod.rs, recovery/restart.rs`.

   This is a pointer repair, **not** a rule statement / `tests` / `ci_script` / `status` change or strengthening — append-only discipline is untouched.

2. **New CI gate `ci/ci_check_registry_code_locus_exists.sh`** — the drift-guard that would have caught the above (see §5). Raises the `ci/ci_check_*.sh` count to 104 at this point in the window.

### `588a554` — grounding-doc re-reproduce at `5db9aae`

Re-reproduced CODEMAP / TRACEABILITY / SEAMS / HEAD_DELTAS at `5db9aae` (a pure regeneration; the prior HEAD_DELTAS window narrated `273c887 → 5db9aae`). All four were subsequently regenerated again at the N-Z close (`67d1ccc`) — see the grounding-doc coherence note in the header; there is no cross-doc staleness at this HEAD.

> **Anomaly check:** removals — 0 (registry rule count unchanged across the tail; no `code_locus` / `tests` / `ci_script` array element removed, only a path token rewritten in place). No discipline violation.
