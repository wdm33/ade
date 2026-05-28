# Ade — HEAD Deltas (Changes Since Baseline)

> **Status:** Living architectural document. Regenerated; not hand-edited.
>
> Regenerate with `/head-deltas <baseline>` after every cluster close. Baseline is recorded in `.idd-config.json` `head_deltas_baseline`.

> Baseline: `dbee4d5` (no tag, 2026-05-27 23:18:22 +0700)
> HEAD: `22eef90` (docs(cluster): PHASE4-N-V CLOSURE — post-close correction, 2026-05-28 18:35:14 +0700)
> 16 commits, 33 files changed, +4172 / -192 lines

This window is exactly two closed clusters — **PHASE4-N-T** (produce_mode real-bootstrap composition) followed by **PHASE4-N-V** (forge ⇄ validator codec symmetry) — plus their declared-rule follow-ons (CN-FORGE-04 → PHASE4-N-W; RO-CLOSE-01 close-gate discipline).

---

## 1. Commit Log

Verbatim from `git log --oneline --no-merges dbee4d5..HEAD`, newest-first. Type is the conventional-commits prefix on the subject; no editorial.

| Hash | Type | Summary |
|------|------|---------|
| `22eef90` | docs | PHASE4-N-V CLOSURE — post-close correction (CE-V-8 re-confirmed unmasked) |
| `8f38bd0` | docs | RO-CLOSE-01 — unmasked close-gate release discipline |
| `440d28a` | docs | declare CN-FORGE-04 + PHASE4-N-W Praos VRF follow-on |
| `e93d936` | fix | PHASE4-N-V — update forge golden + decoders for the era envelope |
| `eb72323` | docs | PHASE4-N-V close — envelope fix; ForgeSucceeded deferred to N-W |
| `aadace5` | feat | PHASE4-N-V S3 — honest-fallback test pins the Praos-VRF blocker |
| `1096733` | feat | PHASE4-N-V S2 — forge_block emits enveloped bytes + round-trip gate |
| `e29d655` | feat | PHASE4-N-V S1 — canonical encode_block_envelope |
| `be2f8da` | docs | PHASE4-N-V cluster doc + invariants/plan + 1 declared rule |
| `be748ff` | docs | PHASE4-N-T close — 3 rules enforced + 5 strengthenings + gate fix |
| `e31e636` | feat | PHASE4-N-T S5 — loopback serve test + bootstrap CI gate |
| `b46a0c6` | feat | PHASE4-N-T S4 — BroadcastBlock to served chain via push_atomic |
| `6353dfd` | feat | PHASE4-N-T S3 — wire ChainEvolution + real forge context, delete synthetic |
| `dbf6ea7` | feat | PHASE4-N-T S2 — GREEN ChainEvolution typestate |
| `9f525df` | feat | PHASE4-N-T S1 — produce-mode cold-start from operator seed |
| `a1213d8` | docs | PHASE4-N-T cluster doc + invariants/plan + 3 declared rules |

Type histogram: feat ×8, docs ×7, fix ×1. Unclassified: 0 (every subject carries a conventional-commits prefix).

---

## 2. New Modules

Modules added since baseline (present at HEAD, absent at `dbee4d5`).

| Module | Color | Purpose | Key sub-paths | Added in (cluster/slice) |
|--------|-------|---------|---------------|--------------------------|
| `ade_runtime::producer::chain_evolution` | GREEN (by content, inside RED `ade_runtime`) | Linear `ChainEvolution` typestate threading the producer's chain state forward across forges; `advance` consumes `self` so forging against a stale base is structurally unrepresentable, cross-checks the BLUE `block_validity` post-state against the BLUE `self_accept` token, and never mints `AcceptedBlock`. | `producer/chain_evolution.rs` (`ChainEvolution`, `ChainEvolutionError::AuthorityMismatch`, `advance`/`seed`) | PHASE4-N-T S2 (`dbf6ea7`) |

No new crate or workspace member was added; `ade_codec::cbor::envelope::encode_block_envelope` is a new function in an existing module (see §3), not a new module.

**Cross-reference (CODEMAP @ `22eef90`):** `producer::chain_evolution` appears in CODEMAP (Key modules + the producer authority table, tagged *(N-T)*). No stale-CODEMAP warning.

---

## 3. Modules Modified

Modules that existed at baseline with non-trivial changes. Grouped by cluster. Trivial doc-only or single-line plumbing edits are folded into the relevant module entry.

| Module | Scope | Key changes |
|--------|-------|-------------|
| `ade_node::produce_mode` | +718 / -165 lines, +2 test files | **N-T:** cold-starts via `bootstrap_initial_state` from the operator seed (`--json-seed` + `--consensus-inputs`); deletes `SyntheticForgeInputs` / `build_synthetic_forge_context` (no zero-stake / `LedgerState::new` / constant-prev-hash forge base); seeds and threads the GREEN `ChainEvolution` typestate; `CoordinatorEffect::BroadcastBlock` reconstructs the `AcceptedBlock` via BLUE `self_accept` then admits to the served snapshot through `ServedChainHandle::push_atomic` (fail-closed `BroadcastPushError::SelfAcceptReplayRejected`). New tests `tests/produce_loopback.rs` (loopback serve) and `tests/forge_succeeds.rs` (N-V honest-fallback pinning the Praos-VRF blocker). Introduces enforcement of CN-PROD-03 / CN-PROD-04 / DC-PROD-03. |
| `ade_node::cli` | +76 / -0 lines | **N-T:** CLI surface for produce-mode cold-start from the operator seed (seed + consensus-inputs wiring into the single bootstrap authority). |
| `ade_codec::cbor::envelope` | +40 / -1 lines | **N-V:** adds the canonical block-envelope **encoder** `encode_block_envelope`, symmetric to the long-standing `decode_block_envelope` — emits the era-tagged `[era, block]` form (Conway = discriminant 7). Sole block-envelope encoder in the workspace; round-trips through `decode_block_envelope`. Enforces CN-FORGE-03. |
| `ade_ledger::producer::forge` | +33 / -2 lines, +1 test file | **N-V:** `forge_block` now wraps its output via `ade_codec::encode_block_envelope` so `decode_block(forge_block(tick).bytes)` is `Ok` — fixes the N-T defect where `forge_block` emitted a bare `array(5)` block rejected at offset 0 by `decode_block_envelope`. New corpus-pin test `tests/envelope_corpus_pin.rs`. Strengthens CN-FORGE-01; CN-FORGE-03 enforced here. |
| `ade_runtime::producer` (mod) | +1 / -0 lines | **N-T:** registers the new `chain_evolution` submodule. |
| `ade_testkit::producer` | +31 / -10 lines across 3 files | **N-V:** forge golden (`EXPECTED_FORGED_*`) and decoder/replay fixtures updated for the era envelope so the cross-impl adapter, fixtures, and replay agree with the now-enveloped forge output. Files: `cross_impl_adapter.rs`, `fixtures.rs`, `replay.rs`. |

### Strengthenings recorded this window (registry `strengthened_in`)

These are not new modules but cross-cutting invariant strengthenings the two clusters carried forward:

- **DC-CONS-18** — `strengthened_in += ["PHASE4-N-T", "PHASE4-N-V"]` (single forge/encode authority extended by both the bootstrap forge base and the envelope encoder).
- **CN-NODE-01** — `strengthened_in += ["PHASE4-N-T"]` (produce mode is a second legitimate startup path that also bootstraps via the sole authority).
- **CN-PROD-02** — `strengthened_in += ["PHASE4-N-T"]` (no parallel synthetic forge codepath; synthetic shortcut deleted).
- **CN-FORGE-01** — `strengthened_in += ["PHASE4-N-T"]` (forge composition).
- **CN-SNAPSHOT-01** — `strengthened_in += ["PHASE4-N-T"]` (served-snapshot admission via `push_atomic`).

---

## 4. Feature Flags

No feature-flag deltas this window. No `Cargo.toml` was modified between `dbee4d5` and HEAD, so no `[features]` table, `optionalDependencies`, build tag, or `extras_require` changed. No `compile_error!`-coupled flag was introduced or removed.

---

## 5. CI Checks

Every CI check added or materially modified since baseline. CI scripts live as `ci/ci_check_*.sh` (no `.github/workflows` in this repo yet, per `.idd-config.json` `ci_dirs`). Grouped by cluster.

### PHASE4-N-T checks

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci_check_produce_mode_uses_bootstrap_initial_state.sh` | New (`e31e636`) | produce mode derives its initial forge state from the single `bootstrap_initial_state` authority and seeds the GREEN `ChainEvolution` typestate — never a synthetic shortcut; `produce_mode.rs` contains no `SyntheticForgeInputs`. Closes the "called zero times via a synthetic bypass" hole. Strengthens CN-NODE-01 + CN-PROD-02. |
| `ci_check_node_binary_uses_single_bootstrap.sh` | Modified (`be748ff`) | Re-scoped from "exactly one call site in the whole crate" to "every production `.rs` file calls `bootstrap_initial_state` at most once (no double-bootstrap per path) AND the crate calls it at least once (authority actually used)" — accommodating produce mode as a second legitimate startup path. Enforces CN-NODE-01. |

### PHASE4-N-V checks

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci_check_forge_decode_round_trip.sh` | New (`1096733`) | `forge_block` wraps its output via `ade_codec::encode_block_envelope`, and the forge ⇄ decode round-trip regression test exists, so `decode_block(forge_block(tick).bytes)` is `Ok` (no bare-block forge output, no parallel block serializer). Enforces CN-FORGE-03. |

**Cross-reference (CODEMAP @ `22eef90`):** `ci_check_forge_decode_round_trip.sh` is listed in the CODEMAP CI table mapped to CN-FORGE-03 / N-V. `ci_check_produce_mode_uses_bootstrap_initial_state.sh` and the modified `ci_check_node_binary_uses_single_bootstrap.sh` map to CN-PROD-03 / CN-NODE-01 / CN-PROD-02 in the registry; confirm they are reflected in TRACEABILITY on its next regeneration.

---

## 6. Canonical Type Registry Delta

n/a — `.idd-config.json` `canonical_type_registry` is `null`. Canonical-type rules live inline in the invariant registry under family **T**; no family-T entries were added or removed this window (the six new rules are CN/DC/RO, see §7).

---

## 7. Normative / Invariant Rule Delta

Source: `docs/ade-invariant-registry.toml` (the project's canonical append-only invariant registry; `invariant_registry` in `.idd-config.json`). Counts by `^id = ` lines.

- Rules at baseline (`dbee4d5`): **285**
- Rules at HEAD (`22eef90`): **291**
- Net additions: **+6**
- Removals: **0** (append-only discipline upheld).

### New rules

| ID | Status | Cluster | One-line summary |
|----|--------|---------|------------------|
| `CN-PROD-03` | enforced | N-T | produce_mode's forge base state is derived from the single `bootstrap_initial_state` authority (operator-seeded ledger + projected `PoolDistrView` + eta0 + tip slot); `SyntheticForgeInputs` / synthetic forge base deleted (cold-start branch only; warm-start deferred to N-U). |
| `CN-PROD-04` | enforced | N-T | Every `BroadcastBlock` reconstructs the `AcceptedBlock` via BLUE `self_accept` then admits to the served snapshot via the single `ServedChainHandle::push_atomic`; replay rejection skips `push_atomic` and emits `BroadcastPushError::SelfAcceptReplayRejected`; only self-accepted blocks are served. |
| `DC-PROD-03` | enforced | N-T | Producer chain-forward continuity + replay: GREEN `ChainEvolution` threads each forge's post-state into the next base (stale-base forging structurally unrepresentable); `advance` cross-checks BLUE `block_validity` vs BLUE `self_accept` and fail-closes on `AuthorityMismatch`; in-memory two-run byte-identity (durable replay deferred to N-U). |
| `CN-FORGE-03` | enforced | N-V | Producer/validator codec symmetry: `forge_block` emits the era-tagged `[era, block]` envelope via the single `ade_codec::encode_block_envelope`, so forge output round-trips through the same `decode_block` authority that validates received blocks (fixes the N-T bare-`array(5)` defect rejected at offset 0). |
| `CN-FORGE-04` | declared | N-V | Producer-side Praos VRF construction must match the Conway/Praos validator authority (Praos alpha `blake2b256(slot‖eta0)` + range-extension, **not** the TPraos role-tagged `slot‖eta0‖0x4C`); no construction/verification fallback accepting both. Declared follow-on, scheduled for **PHASE4-N-W**. |
| `RO-CLOSE-01` | enforced | N-V | Unmasked close-gate discipline: any slice changing canonical bytes/encoded forms/golden fixtures must run an unmasked full close gate (`cargo test --workspace`) and use cargo's real exit status as the sole pass/fail authority; piped output is display-only; all consumers of changed canonical output must be audited before closure. |

### Modified rules

The five strengthenings listed in §3 (DC-CONS-18, CN-NODE-01, CN-PROD-02, CN-FORGE-01, CN-SNAPSHOT-01) had cluster IDs appended to `strengthened_in`; no statement was weakened.

### Honest residual

In-process `ForgeSucceeded` end-to-end self-accept remains deferred to **PHASE4-N-W**, pinned by the registry's `forge_to_self_accept_blocked_on_praos_vrf_construction` obligation (TPraos vs. Praos VRF transcript mismatch). `tests/forge_succeeds.rs` is the honest-fallback test that pins this blocker rather than masking it.
