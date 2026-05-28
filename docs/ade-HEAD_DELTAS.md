# Ade — HEAD Deltas (Changes Since Baseline)

> **Status:** Living architectural document. Regenerated; not hand-edited.
>
> Regenerate with `/head-deltas <baseline>` after every cluster close. Baseline is recorded in `.idd-config.json` `head_deltas_baseline`.

> Baseline: `22eef90` (no tag, 2026-05-28 18:35:14 +0700)
> HEAD: `01e7e08` (feat(ci): PHASE4-N-W S3 — guard the producer Praos VRF + enforce CN-FORGE-04, 2026-05-29 00:54:20 +0700)
> 8 commits, 90 files changed, +4285 / -6988 lines

This window is exactly two pieces:

1. A **registry/doc hygiene pass** (`459ff90` + `6e91f25`, with the `e681baa` grounding refresh and `d313a5a` handoff note that opened it) — **no code or behavior change**. It reconciled 9 test-name drifts and bound 8 CI scripts in the invariant registry, archived every closed cluster doc (N-Q / N-R-* / N-S-* / N-M-* / N-O, and others) from `docs/clusters/` into `docs/clusters/completed/`, and regenerated TRACEABILITY. This is the source of the large negative line delta and the long list of pure-rename rows in `git diff --stat` (the archived cluster docs are git renames with content unchanged).
2. The **PHASE4-N-W** cluster (`bcce61e` cluster doc + `321025f` S1 + `9ba8bee` S2 + `01e7e08` S3) — producer Praos VRF authority migration. This is the only code change in the window.

> **Baseline bump (this close):** on the PHASE4-N-W close, `.idd-config.json` `head_deltas_baseline` should be bumped from `22eef90` to **`01e7e08`** so the next cluster narrates from this point. (That config edit is made separately, outside this regeneration.)

---

## 1. Commit Log

Verbatim from `git log --oneline --no-merges 22eef90..HEAD`, newest-first. Type is the conventional-commits prefix on the subject; no editorial.

| Hash | Type | Summary |
|------|------|---------|
| `01e7e08` | feat | PHASE4-N-W S3 — guard the producer Praos VRF + enforce CN-FORGE-04 |
| `9ba8bee` | feat | PHASE4-N-W S2 — Praos producer leader-VRF migration |
| `321025f` | feat | PHASE4-N-W S1 — TPraos producer-forge fail-closed |
| `bcce61e` | docs | PHASE4-N-W cluster doc + invariants + plan |
| `6e91f25` | docs | archive the completed registry/doc hygiene-pass handoff note |
| `459ff90` | docs | reconcile test-name drift + bind CI gates + archive closed clusters |
| `d313a5a` | docs | registry & doc hygiene-pass handoff note |
| `e681baa` | docs | refresh CODEMAP/SEAMS/HEAD_DELTAS/TRACEABILITY for N-T + N-V |

Type histogram: feat ×3, docs ×5. Unclassified: 0 (every subject carries a conventional-commits prefix).

---

## 2. New Modules

No new modules, crates, or workspace members were added this window. PHASE4-N-W is an authority migration entirely within existing modules (`ade_core::consensus::vrf_cert`, `ade_types::era`, `ade_node::produce_mode`, `ade_ledger::producer::forge`) — see §3.

The new symbols `ExpectedVrfInput`, `leader_vrf_input`, and `leader_value_for` are new public items inside the existing `ade_core::consensus::vrf_cert` module (re-exported via `consensus::mod`), not a new module.

**Cross-reference (CODEMAP @ `01e7e08`):** no new module to cross-reference; no stale-CODEMAP warning on this axis.

---

## 3. Modules Modified

Modules that existed at baseline with non-trivial changes. The hygiene pass (piece 1) touched no code — it only moved cluster docs and edited the registry/TRACEABILITY, so it produces no §3 entry. Every entry below is PHASE4-N-W.

| Module | Scope | Key changes |
|--------|-------|-------------|
| `ade_core::consensus::vrf_cert` | +58 / -7 lines | **N-W:** introduces the closed two-variant `ExpectedVrfInput { Praos(alpha), Tpraos(alpha) }` and makes `leader_vrf_input(era, slot, eta0)` the **single era→VRF-input construction authority** — Praos eras get `praos_vrf_input` (Praos alpha + range-extension), pre-Praos eras get the TPraos role-tagged `vrf_input(.., LeaderEligibility)`. Adds `leader_value_for(input, output)` which reads the era family from the `ExpectedVrfInput` variant (Praos → `praos_leader_value`, TPraos → identity) so leader-value computation has no silent dual meaning. Re-exported through `consensus::mod`. |
| `ade_core::consensus::leader_check` / `leader_schedule` | +132 / -68 lines across `leader_check.rs`, `leader_schedule.rs`, `mod.rs` | **N-W:** the leader-schedule answer now carries `expected_vrf_input` built via the single `leader_vrf_input` authority; `leader_check` reads the era family from `answer.expected_vrf_input` rather than reconstructing it, and cross-checks `answer.expected_vrf_input == leader_vrf_input(era, slot, eta0)` (prove-over-answer-alpha discipline). |
| `ade_types::era` | +23 / 0 lines | **N-W:** adds `CardanoEra::is_praos()` — `true` only for Babbage and Conway, `false` for Byron/Shelley/Allegra/Mary/Alonzo. This is the predicate the produce-mode era guard branches on. |
| `ade_node::produce_mode` | +30 / -10 lines, +2 test files updated | **N-W:** adds the producer **era guard** — when `!era.is_praos()` the forge fails closed with `ForgeFailureReason::UnsupportedProducerEra` instead of attempting a forge. The RED VRF prove step now proves over `ctx.leader_schedule_answer.expected_vrf_input.alpha_bytes()` (the single authority's bytes — no independent alpha reconstruction in the shell), cross-checking the answer's alpha against the BLUE authority. Tests `tests/forge_handler_variants.rs` and `tests/forge_succeeds.rs` updated to exercise the new fail-closed path and the now-unblocked Praos forge. Enforces CN-FORGE-04; strengthens CN-FORGE-01 + DC-PROD-03. |
| `ade_ledger::producer::forge` | +4 / -2 lines | **N-W:** `ForgeFailureReason` gains the `UnsupportedProducerEra` variant (also surfaced in `ade_runtime::producer::producer_log` / `coordinator` for logging + handling). Plumbing for the produce-mode era guard. |
| `ade_runtime::producer` (coordinator / producer_log / scheduler / tick_assembler) | +18 / -6 lines across 4 files | **N-W:** non-authoritative RED plumbing for the new `UnsupportedProducerEra` reason (log vocabulary entry + coordinator match arm) and minor signature threading for the era-correct VRF input. |
| `ade_testkit::producer::fixtures` | +6 / -1 lines | **N-W:** fixtures updated so the producer pipeline tests build the leader-schedule answer with the era-tagged `expected_vrf_input`. |

### Strengthenings recorded this window (registry `strengthened_in`)

Not new rules — cross-cutting invariant strengthenings PHASE4-N-W carried forward:

- **CN-FORGE-01** — `strengthened_in += ["PHASE4-N-W"]` (forge composition now era-gated through the single VRF-input authority).
- **DC-PROD-03** — `strengthened_in += ["PHASE4-N-W"]` (producer chain-forward continuity strengthened with the era-correct leader-VRF construction).

---

## 4. Feature Flags

No feature-flag deltas this window. No `Cargo.toml` (workspace root or any member) was modified between `22eef90` and `01e7e08`, so no `[features]` table, `optionalDependencies`, build tag, or `extras_require` changed. No `compile_error!`-coupled flag was introduced or removed.

---

## 5. CI Checks

Every CI check added or materially modified since baseline. CI scripts live as `ci/ci_check_*.sh` (no `.github/workflows` in this repo yet, per `.idd-config.json` `ci_dirs`). Count: **97 → 98** (+1 new, 0 modified, 0 removed).

> Note: the hygiene pass (`459ff90`) **bound** 8 already-existing CI scripts into the invariant registry's `ci_scripts` arrays. Those scripts existed at baseline — binding them in the registry is a TRACEABILITY/registry change, not a new CI file. The single net-new CI *file* this window is the PHASE4-N-W gate below.

### PHASE4-N-W checks

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci_check_producer_praos_vrf.sh` | New (`01e7e08`) | The producer-side Praos VRF construction matches the Conway/Praos validator authority: `leader_vrf_input` is the single era→VRF-input construction site; Praos eras use the Praos alpha (`blake2b256(slot‖eta0)` + range-extension), **not** the TPraos role-tagged form; the produce-mode era guard fails closed with `UnsupportedProducerEra` on non-Praos eras; the shell proves over the answer's `expected_vrf_input` bytes with no independent alpha reconstruction. Enforces CN-FORGE-04. |

**Cross-reference (CODEMAP @ `01e7e08` / TRACEABILITY):** `ci_check_producer_praos_vrf.sh` is bound to `CN-FORGE-04` in `docs/ade-invariant-registry.toml` (`ci_script` field). Confirm it appears in the CODEMAP CI table and in TRACEABILITY mapped to CN-FORGE-04 on their next regeneration (TRACEABILITY was regenerated in the hygiene pass at `459ff90`, then CN-FORGE-04 flipped declared→enforced at `01e7e08` — so TRACEABILITY may show CN-FORGE-04 as `declared` until its next regen).

---

## 6. Canonical Type Registry Delta

n/a — `.idd-config.json` `canonical_type_registry` is `null`. Canonical-type rules live inline in the invariant registry under family **T**; no family-T entries were added or removed this window.

---

## 7. Normative / Invariant Rule Delta

Source: `docs/ade-invariant-registry.toml` (the project's canonical append-only invariant registry; `invariant_registry` in `.idd-config.json`). Counts by `^[[rules]]` entries.

- Rules at baseline (`22eef90`): **291**
- Rules at HEAD (`01e7e08`): **291**
- Net additions: **0** (no new IDs)
- Removals: **0** (append-only discipline upheld).

### Status changes (no new IDs)

| ID | Change | Cluster | One-line summary |
|----|--------|---------|------------------|
| `CN-FORGE-04` | `declared` → `enforced` (+ `ci_script` bound) | N-W | Producer-side Praos VRF construction matches the Conway/Praos validator authority (Praos alpha `blake2b256(slot‖eta0)` + range-extension, **not** the TPraos role-tagged `slot‖eta0‖0x4C`); single `leader_vrf_input` construction authority; non-Praos eras fail closed via `UnsupportedProducerEra`. Was the declared follow-on scheduled for PHASE4-N-W at the N-V close; now mechanically enforced by `ci_check_producer_praos_vrf.sh`. |

### Modified rules (strengthenings)

The two strengthenings listed in §3 had `PHASE4-N-W` appended to `strengthened_in`; no statement was weakened:

- **CN-FORGE-01** — `strengthened_in += ["PHASE4-N-W"]`.
- **DC-PROD-03** — `strengthened_in += ["PHASE4-N-W"]`.

### Hygiene-pass registry edits (piece 1)

The `459ff90` hygiene pass reconciled **9 test-name drifts** (registry `tests` arrays brought back into sync with the actual test function names) and **bound 8 CI scripts** into `ci_scripts` arrays for rules that were already enforced by those scripts but not yet referencing them. These are append-only `tests`/`ci_scripts` reconciliations — no rule statement, ID, or status changed in that commit, and no rule was weakened or removed.

### Honest residual

`CN-FORGE-04` was the last declared-rule follow-on from the N-V close; it is now enforced. With the producer Praos VRF migration landed, the TPraos-vs-Praos VRF transcript mismatch that pinned `forge_to_self_accept` is resolved on the construction side. Any remaining end-to-end self-accept obligations are tracked in the registry's per-rule `open_obligation` fields, not here.
