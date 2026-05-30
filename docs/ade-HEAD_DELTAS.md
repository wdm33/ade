# Ade — HEAD Deltas (Changes Since Baseline)

> **Status:** Living architectural document. Regenerated; not hand-edited.
>
> Regenerate with `/head-deltas <baseline>` after every cluster close. Baseline is recorded in `.idd-config.json` `head_deltas_baseline`.

> Baseline: `67d1ccc` (no tag, 2026-05-29 16:57:41 +0700)
> HEAD: `a3a0636` (A5 scoping — sharpen the producer "no anchor" claim, 2026-05-30 13:25:21 +0700)
> 24 commits, 45 files changed, +6433 / -2676 lines

This window narrates the **PHASE4-N-F-A cluster** — the *recovered* seed-epoch consensus-input **CAPABILITY** cluster — preceded by a short **PHASE4-N-Z post-close housekeeping tail**. The prior baseline (`67d1ccc`) was the PHASE4-N-Z close; per the per-cluster bump model the N-Z cluster body is now archived in git history + its cluster docs, and this doc narrates only the post-N-Z span.

The window is two pieces:

1. The **PHASE4-N-Z post-close tail** (`b1c2267` → `d7192e2`, 7 commits) — registry hygiene + a CI-gate retirement/repoint pass, **no `crates/**/*.rs` behavior change**. The load-bearing items: `a2af041` retires the foreign `ci_check_constitution_coverage.sh` gate (a ziranity-v3 import) and folds its coherence checks into the code-locus gate; `bb95e95` repoints `ci_check_producer_corpus_present.sh` at the produce-mode CLI; `d7192e2` reconciles `T-CI-01` after the retirement. The remaining four (`b1c2267` N-Z grounding refresh, `663e001` RO-MITHRIL-IMPORT-01 item-(a) reclassification, `e8bde40` stale-claim hygiene, `d5ecfe5` red-gate decision record) are docs/registry-only. This tail is narrated in **§8**.
2. The **PHASE4-N-F-A cluster** (`5f2f1b6` invariant sketch → `a3a0636` A5 scoping, 17 commits) — the recovered seed-epoch consensus-input CAPABILITY. Two new BLUE-or-GREEN modules in `ade_ledger` / `ade_runtime`, one new RED module, one new BLUE projection fn, one new CI containment gate, four new invariant rules. Narrated in §§2–7.

> **⚠ Generation state — this is a DRY-RUN against an IN-FLIGHT cluster close, written to `docs/ade-HEAD_DELTAS.md.proposed` (not the live doc).** PHASE4-N-F-A has **not formally closed**: there is no cluster-close commit at HEAD. The split, at HEAD `a3a0636`, is:
> - **Committed** through `a3a0636`: all source (A1–A4 `*.rs` + the new CI gate `ci/ci_check_consensus_input_provenance.sh`), all cluster/slice docs (A1, A2, A3a, A3b, A4, A5-SCOPING, cluster.md), and the N-Z-tail registry/CI edits.
> - **Uncommitted working tree** (the close-pass being staged): the **registry promotion** (the four `CN/DC-CINPUT` rules + the carry-forward `strengthened_in` edits, 299 → 303) and the **four grounding-doc regenerations** (`ade-{CODEMAP,SEAMS,TRACEABILITY}.md` + this `HEAD_DELTAS`). `git show HEAD:docs/ade-invariant-registry.toml` is still **299** and the committed CODEMAP/TRACEABILITY/SEAMS carry **no** N-F-A content; the N-F-A registry rules + grounding refresh live only in the working tree.
>
> This doc narrates the **staged close-pass state** (working-tree registry = the close the parent is preparing). The committed-vs-working-tree counts are reconciled inline in §6/§7 and flagged in the §0 anomaly box. The next cluster close should commit the staged registry + four docs together, then re-bump `.idd-config.json` `head_deltas_baseline` from `67d1ccc` to **`a3a0636`** (the parent is handling the config edit separately).

> **Cluster framing — CAPABILITY, not production wiring (load-bearing; do not over-read).** PHASE4-N-F-A proves the *recovered-state surface* end-to-end at the **authority surface**: bootstrap → persist (anchor-keyed sidecar) → WAL provenance → warm-start **verify** → project to `PoolDistrView`. It does **NOT** wire the **producer** to consume that surface: `produce_mode` still **cold-starts** from `--consensus-inputs-path` (`SeedEpochConsensusSource::NotRequired`) and cannot even name the recovered type. **BA-02 is not satisfied by this cluster.** Producer consumption (CE-A-4b) and the production restart path are **deferred to the successor cluster PHASE4-N-F-C**; A5 is the scoping handoff to N-F-C, not an implemented slice.

---

## 0. Anomalies & Cross-Reference Warnings (surface prominently)

Recorded so a reader does not mistake an intentional change for a defect.

| Item | Class | Disposition |
|------|-------|-------------|
| `ade_ledger::wal::event` — `WalEntry::prior_fp()` / `WalEntry::post_fp()` accessor methods **deleted** | Intentional A3a refactor | With two semantic WAL classes (the `AdmitBlock` chain vs. the new non-chaining provenance variant), the two-line accessors were replaced by an explicit `match` at both chain-walk sites. **No canonical *type* was removed** — the new `WalEntry::SeedEpochConsensusInputsImported` variant (wire TAG=3) is *additive* and deliberately does **not** participate in the `prior_fp`/`post_fp` fingerprint chain. Append-only type discipline upheld. |
| `ci/ci_check_constitution_coverage.sh` **removed** (290 lines) | Intentional N-Z-tail cleanup | A foreign ziranity-v3 import; its coherence checks were folded into `ci_check_registry_code_locus_exists.sh` (`a2af041`), and `T-CI-01` was repointed to that gate (`d7192e2`). **Not a lost Ade gate** — net CI-script count is flat (105 → 105: −1 foreign, +1 N-F-A). |
| `T-REC-01` / `T-REC-02` `strengthened_in` **replaced** `["PHASE4-N-R-A"]` → `["PHASE4-N-F-A"]` in the staged registry | **Append-only concern — verify before commit** | Under append-only `strengthened_in` discipline these should read `["PHASE4-N-R-A", "PHASE4-N-F-A"]` (the N-R-A strengthening must not be dropped). The other three carry-forwards (`CN-ANCHOR-01`, `DC-ANCHOR-01`, `CN-NODE-01`) correctly **append**. This is in the *uncommitted* close-pass registry — **fix to append before committing the cluster close.** |
| Committed HEAD grounding docs (`CODEMAP`/`SEAMS`/`TRACEABILITY`) carry **no** N-F-A content | Expected (in-flight close) | The N-F-A refresh is staged in the working tree (CODEMAP wt: 11 `seed_consensus_inputs` hits; committed: 0). Once the staged refresh + registry are committed together, all four docs + the registry will be coherent at the close SHA. **Until then the committed docs are stale w.r.t. N-F-A** — which is the normal pre-close state, not a defect. |
| Committed HEAD `TRACEABILITY` references the now-deleted `ci_check_constitution_coverage.sh` (line 114) | Stale-in-committed-doc, repaired-in-working-tree | The committed TRACEABILITY still cites the retired gate for `T-CI-01`; the working-tree refresh repoints it to `ci_check_registry_code_locus_exists.sh`. Resolved by committing the staged refresh. |

No canonical-type removals. No invariant-rule removals (the registry is `+4 / −0`; the `T-REC` issue is a *strengthening-list* replacement, not a rule deletion). Zero commits without a conventional-commits prefix.

---

## 1. Commit Log

Verbatim from `git log --oneline --no-merges 67d1ccc..HEAD`, newest-first. Type is the conventional-commits prefix on the subject; no editorial. (History uses no merge commits in this span — `--merges` is empty.)

| Hash | Type | Summary |
|------|------|---------|
| `a3a0636` | docs | A5 scoping — sharpen the producer "no anchor" claim |
| `02f3e87` | docs | revise A5 scoping — BA-02 goal-reset + C1 "choose production owner" first |
| `2cf28f0` | docs | A5 scoping — producer recovered-state lifecycle is a successor cluster, not a slice |
| `8b60524` | feat | PHASE4-N-F-A A4 — BLUE projection recovered surface → PoolDistrView |
| `d817240` | docs | add PHASE4-N-F-A A4 slice doc — projection + CE-A-4 split |
| `104982d` | feat | PHASE4-N-F-A A3b — bootstrap warm-start sidecar restore capability |
| `adb4e2a` | docs | revise PHASE4-N-F-A A3b — capability scope, not production wiring |
| `c507159` | feat | PHASE4-N-F-A A3a — WAL seed-epoch-consensus-inputs provenance entry |
| `4d50fb2` | docs | add PHASE4-N-F-A A3 slice docs |
| `f6bf50f` | feat | PHASE4-N-F-A A2 — persist seed-epoch consensus inputs at bootstrap (keyed sidecar) + containment gate |
| `784db97` | docs | revise PHASE4-N-F-A for Option 3 (production warm-start + keyed sidecar) |
| `5dfd3dd` | docs | add PHASE4-N-F-A A2 slice doc |
| `c13c2e9` | feat | PHASE4-N-F-A A1 — SeedEpochConsensusInputs type + sole codec |
| `bd59d71` | docs | add PHASE4-N-F-A A1 slice doc |
| `f3c8143` | docs | add PHASE4-N-F-A cluster doc |
| `31375ec` | docs | add PHASE4-N-F split cluster plan |
| `5f2f1b6` | docs | PHASE4-N-F invariant sketch (BA-02 produce wiring) |
| `d7192e2` | docs | reconcile T-CI-01 after retiring foreign gate |
| `a2af041` | ci | retire foreign constitution-coverage gate, fold coherence checks |
| `bb95e95` | ci | repoint producer corpus gate at produce-mode CLI |
| `d5ecfe5` | docs | record pending red-gate repair decisions |
| `e8bde40` | docs | correct stale produce-mode forge claims + archived-cluster CI path |
| `663e001` | docs | reclassify RO-MITHRIL-IMPORT-01 item (a) — documented-interface, Tier-4 non-goal (decision record) |
| `b1c2267` | docs | refresh CODEMAP/TRACEABILITY/SEAMS/HEAD_DELTAS for PHASE4-N-Z |

Type histogram: **docs ×17, feat ×5, ci ×2**. **Unclassified by prefix: 0** — every commit carries a conventional-commits prefix. The five `feat` commits are the only source-bearing commits (A1, A2, A3a, A3b, A4); all `docs` are cluster/slice/planning/registry docs; the two `ci` are the N-Z-tail gate retirement + repoint.

(`b1c2267` is the N-Z close-pass grounding refresh — it sits at the very start of this window because the baseline `67d1ccc` is its predecessor; it touched only `docs/` and carries no N-F-A content.)

---

## 2. New Modules

Five modules added this window — three new files plus two pre-existing files that gained a first-class new surface — all PHASE4-N-F-A. Colors per the A1/A2/A3a doc-comment self-classification and the project TCB vocabulary.

| Module | Color | Purpose | Key sub-paths | Added in (cluster/slice) |
|--------|-------|---------|---------------|--------------------------|
| `ade_ledger::seed_consensus_inputs` | **BLUE** | The closed, version-gated, byte-canonical **`SeedEpochConsensusInputs`** record of the seed-epoch consensus inputs established during verified bootstrap (per-pool active-stake + registered VRF keyhash distribution, ASC, total active stake) for the single seed `epoch_no`, plus its **sole** CBOR encoder/decoder pair (CN-CINPUT-01). Deterministic, `BTreeMap`-ordered, `SEED_CINPUT_SCHEMA_VERSION = 1` written into the form; decode rejects unknown versions, non-canonical/duplicate pool-map keys, and trailing bytes, and verifies byte-identity re-encode. No `Default`, no `#[non_exhaustive]` — the type system requires every field at construction. Carries `anchor_fp` so the record is self-describing and bound to a `BootstrapAnchor` (fingerprint-keyed sidecar — Option A; the anchor is NOT bumped). | `seed_consensus_inputs.rs` (`SeedEpochConsensusInputs`, `PoolEntry { active_stake, vrf_keyhash }`, `encode_seed_epoch_consensus_inputs` / `decode_seed_epoch_consensus_inputs`, `SEED_CINPUT_SCHEMA_VERSION`) | PHASE4-N-F-A / `c13c2e9` (A1) |
| `ade_runtime::seed_consensus_merge` | **GREEN** | The pure, deterministic, no-I/O **merge transform** (A2) that lifts a verified-bootstrap `LiveConsensusInputsCanonical` (bootstrap-time extraction shape — `pool_distribution` carries only `active_stake`; VRF keyhashes live in a separate `pool_vrf_keyhashes` map) plus the minted anchor fingerprint and seed epoch into the BLUE single-map `SeedEpochConsensusInputs` (whose `PoolEntry` carries both `active_stake` and `vrf_keyhash`). `BTreeMap` only; fails closed on missing VRF or stake for a pool. | `seed_consensus_merge.rs` (`merge_seed_epoch_consensus_inputs`) | PHASE4-N-F-A / `f6bf50f` (A2) |
| `ade_runtime::seed_consensus_provenance` | **RED** | The single shared helper (A3a) that **appends** the closed `WalEntry::SeedEpochConsensusInputsImported` provenance entry **after** the verified-bootstrap composition site has durably `put` the sidecar. RED because it touches the `WalStore` (I/O); the entry it writes — and that entry's codec/replay — are BLUE in `ade_ledger::wal`. The put → append ordering is the commit point (load-bearing). | `seed_consensus_provenance.rs` (`append_seed_epoch_provenance`) | PHASE4-N-F-A / `c507159` (A3a) |
| `ade_ledger::consensus_view` → `PoolDistrView::from_seed_epoch_consensus_inputs` | **BLUE** *(new surface on an existing module)* | The A4 **projection**: a pure BLUE field-map from the recovered `SeedEpochConsensusInputs` onto the leadership-consumed `PoolDistrView` (full `LedgerView` surface: `total_active_stake`, `pool_active_stake`, `pool_vrf_keyhash`, `active_slots_coeff`; single-epoch — off-epoch queries return `None`), proven **equivalent** to the prior operator-bundle projection `pool_distr_view_from_consensus_inputs` for the seed epoch (DC-CINPUT-02a). | `consensus_view.rs` (`PoolDistrView::from_seed_epoch_consensus_inputs`) | PHASE4-N-F-A / `8b60524` (A4) |
| `ade_runtime::bootstrap` → `SeedEpochConsensusSource` + warm-start branch | **BLUE-authority surface** *(new surface on an existing module)* | The A3b **warm-start VERIFICATION CAPABILITY**: a new `SeedEpochConsensusSource` enum (`NotRequired` / `RequiredFromRecoveredProvenance`) gating `bootstrap_initial_state`. The `RequiredFromRecoveredProvenance` branch restores the sidecar and verifies it fail-closed — sidecar present, `blake2b_256 == provenance.sidecar_hash`, A1 decode, `anchor_fp + epoch_no` binding, byte-identity re-encode — exposing the recovered inputs or halting (typed `BootstrapError`, `EXIT_AUTHORITY_FATAL_DECODE`, no bundle fallback). Proven on the authority surface directly; **no production mode is wired to it** (`node.rs` `run_node_until_shutdown` + `recover_node_state` are test-only; `produce_mode` cold-starts). | `bootstrap.rs` (`SeedEpochConsensusSource`, `RequiredFromRecoveredProvenance` warm-start verify branch) | PHASE4-N-F-A / `104982d` (A3b) |

**Cross-reference (CODEMAP):** the **committed** `docs/ade-CODEMAP.md` at HEAD does **not** yet catalogue any of these (0 hits for `seed_consensus_inputs`). The **working-tree** CODEMAP (staged close-pass) **does** — 11 hits `seed_consensus_inputs`, 10 `seed_consensus_merge`, 8 `seed_consensus_provenance`, 23 `SeedEpochConsensusInputs`, 14 `CN-CINPUT`. **Action:** commit the staged CODEMAP refresh with the cluster close; until then the committed CODEMAP is stale w.r.t. N-F-A (expected pre-close state). When committed, verify each module above appears in CODEMAP §BLUE (`seed_consensus_inputs`, `consensus_view`), §GREEN (`seed_consensus_merge`), §RED (`seed_consensus_provenance`), and the `bootstrap` row notes the new `SeedEpochConsensusSource` warm-start branch.

No new corpus / non-source artifacts this window (one new runbook doc, `docs/active/mithril-documented-interface-runbook.md`, accompanies the N-Z-tail RO-MITHRIL-IMPORT-01 item-(a) reclassification — see §8).

---

## 3. Modules Modified

Modules that existed at baseline with non-trivial changes. Grouped by cluster/slice; commit-by-commit paraphrase is avoided.

| Module | Scope | Key changes |
|--------|-------|-------------|
| `ade_ledger::wal` | +6 files touched, ~+500 / −60 lines (`event.rs` +150/−, `replay.rs` +223, `store_trait.rs` +39, `error.rs` +10, `mod.rs` +3) | **N-F-A A3a:** adds the additive closed `WalEntry::SeedEpochConsensusInputsImported` variant (wire **TAG=3**) — a *bootstrap provenance* entry that does **not** participate in the `AdmitBlock` `prior_fp`/`post_fp` fingerprint chain. `replay.rs` reconstructs a typed `RecoveredBootstrapProvenance` view (exactly one per store/anchor; duplicate or anchor-mismatch fails closed; the `AdmitBlock` chain walk is unaffected). `store_trait.rs` extends the WAL store surface; `error.rs` adds the fail-closed variants. **Anomaly (intentional):** `event.rs` **deletes** the `WalEntry::prior_fp()`/`post_fp()` accessor methods, replacing them with an explicit `match` at both chain-walk sites (two semantic WAL classes now exist). No canonical type removed. |
| `ade_runtime::bootstrap` | +583 / − (`bootstrap.rs`) | **N-F-A A2 + A3b:** the verified-bootstrap composition now **merges** (via `merge_seed_epoch_consensus_inputs`), **encodes** (A1 sole encoder), **puts** the anchor-keyed sidecar (`put_seed_epoch_consensus_inputs`), and **appends** the WAL provenance entry (`append_seed_epoch_provenance`) — the populate ordering CN-CINPUT-02 fences. A3b adds the `SeedEpochConsensusSource` gate enum + the `RequiredFromRecoveredProvenance` **warm-start verify** branch (see §2). This is the authority-surface CAPABILITY; no production mode threads it. |
| `ade_runtime::genesis_bootstrap` | +233 / − | **N-F-A A2/A3a:** the genesis composer becomes an *allowed populate site* — it calls `merge_…` + `encode_…` + `.put_seed_epoch_consensus_inputs(` + `append_seed_epoch_provenance(`, exactly as the new containment gate's guard (a) requires. |
| `ade_runtime::mithril_bootstrap` | +210 / − | **N-F-A A2/A3a:** the second allowed populate site — same populate-then-append composition as `genesis_bootstrap` (guard (a) asserts the populator lives at *both* verified-bootstrap composers). (This is the module introduced in the prior N-Z window; this window only extends it with the sidecar populate path.) |
| `ade_ledger::consensus_view` | +113 / − | **N-F-A A4:** adds `PoolDistrView::from_seed_epoch_consensus_inputs` (see §2) + its determinism/off-epoch tests. Existing `pool_distr_view_from_consensus_inputs` (the operator-bundle projection) is unchanged and is the DC-CINPUT-02a equivalence oracle. |
| `ade_runtime::chaindb` | +188 / − across `in_memory.rs` (+32), `mod.rs` (+22), `persistent.rs` (+75), `snapshot_contract.rs` (+59) | **N-F-A A2:** extends the `SnapshotStore` surface with the anchor-fp-keyed `put_seed_epoch_consensus_inputs` / get accessor across the in-memory + persistent impls + the snapshot contract. The keyed sidecar is **disjoint** from the slot-keyed snapshots (asserted by `snapshot_store_keyed_sidecar_is_disjoint_from_slot_snapshots`). |
| `ade_node::produce_mode` | +117 / −, net structural | **N-F-A:** *deliberately unchanged in input source.* Refactored to set `seed_epoch_consensus_source: SeedEpochConsensusSource::NotRequired` explicitly and continues to cold-start from `--consensus-inputs-path`. **Producer does NOT consume the recovered surface** — it cannot name `SeedEpochConsensusInputs`. This is the CN-CINPUT-02 forge-time fence in code form; producer consumption (CE-A-4b) is deferred to N-F-C. |
| `ade_node::node` | +23 / − | **N-F-A A3b:** the recovery scaffolding (`recover_node_state`, `run_node_until_shutdown`) gains the warm-start verify wiring but remains **test-only** — DC-CINPUT-01 is `partial` precisely because no *production* mode threads this path. |
| `ade_runtime::recovery::restart` | +60 / − | **N-F-A A3b:** restart-path replay now reconstructs the `RecoveredBootstrapProvenance` view; exercised on the authority surface, not from a production entry point. |
| Test surfaces (`ade_node/tests/shutdown_resume_identity.rs`, `ade_runtime/tests/wal_replay_from_anchor.rs`) | +25 / − | **N-F-A:** extend shutdown/resume identity + WAL-replay-from-anchor coverage to assert the provenance entry does not perturb the admit-block chain and that warm-start restores byte-identically. |

### Strengthenings recorded this window (staged registry `strengthened_in`)

Not new rules — five cross-cutting invariant strengthenings PHASE4-N-F-A carries forward (see §7). **All five are in the *uncommitted* close-pass registry, not committed HEAD.**

- **`CN-ANCHOR-01`**, **`DC-ANCHOR-01`** — the anchor now binds a fingerprint-keyed seed-epoch sidecar (the recovered surface is anchor-bound). Correctly **appended** (`… + "PHASE4-N-F-A"`).
- **`CN-NODE-01`** — both verified-bootstrap composers route the sidecar populate through the single closed bootstrap authority. Correctly **appended**.
- **`T-REC-01`**, **`T-REC-02`** — recovery replay-equivalence + all-state-derivable-by-replay now cover the provenance entry + warm-start. ⚠ **append-only concern:** the staged registry **replaces** `["PHASE4-N-R-A"]` with `["PHASE4-N-F-A"]` (dropping N-R-A). Must be `["PHASE4-N-R-A", "PHASE4-N-F-A"]` before commit (see §0 / §7).

---

## 4. Feature Flags

No feature-flag deltas this window. **No `Cargo.toml`** (workspace root or any member) was modified between `67d1ccc` and `a3a0636`, so no `[features]` table, `optionalDependencies`, build tag, or `extras_require` changed. No `compile_error!`-coupled flag was introduced or removed.

---

## 5. CI Checks

Every CI check added or materially modified since baseline. Enforcement gates live as `ci/ci_check_*.sh`. Count of `ci/ci_check_*.sh`: **105 → 105** (net 0: **+1 new** in N-F-A, **−1 removed** in the N-Z tail, **2 modified** in the N-Z tail).

### PHASE4-N-F-A check (new)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci_check_consensus_input_provenance.sh` | **New** (`f6bf50f`, A2; extended for the WAL-provenance token by A3a `c507159`) | **CN-CINPUT-02** — the `SeedEpochConsensusInputs` sidecar may be populated **only** on the verified-bootstrap composition path, through the anchor-keyed `SnapshotStore`. A data-flow-resistant **containment** gate (modelled on N-Z's `ci_check_mithril_seed_point_independence.sh` — containment, not a bypassable RHS grep), strips `#[cfg(test)]` + line comments, three guards: **(a) POSITIVE** — each composer (`genesis_bootstrap.rs`, `mithril_bootstrap.rs`) calls `.put_seed_epoch_consensus_inputs(` *and* builds via `merge_seed_epoch_consensus_inputs(` + the A1 sole `encode_…(` *and* appends via `append_seed_epoch_provenance(`; **(b) NEGATIVE forge-time fence** — `produce_mode.rs` (which owns `import_live_consensus_inputs` + `pool_distr_view_from_consensus_inputs` + `--consensus-inputs-path`) names **none** of the sidecar build/put/encode/provenance tokens nor `SeedEpochConsensusInputs`; **(c) GLOBAL containment** — across all production (test-stripped) Rust, any *call* to `.put_seed_epoch_consensus_inputs(` / `merge_…(` / `append_seed_epoch_provenance(` outside the four allow-listed modules fails closed (closes the "hidden second populator anywhere in the tree" class), plus an allow-list sanity floor. |

### PHASE4-N-Z post-close tail checks (retired + modified)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci_check_constitution_coverage.sh` | **Removed** (`a2af041`, −290 lines) | A **foreign ziranity-v3 import**, retired. Its registry-coherence checks were folded into `ci_check_registry_code_locus_exists.sh`; `T-CI-01` was repointed accordingly (`d7192e2`). **Intentional cleanup, not a lost Ade gate.** |
| `ci_check_registry_code_locus_exists.sh` | **Modified** (`a2af041`/`d7192e2`, +74/−) | Absorbs the retired gate's coherence responsibilities: in addition to the original `code_locus`-path-exists drift-guard, it now folds the unique-id + directed-`cross_ref`-target-resolution coherence checks. This is the gate `T-CI-01` now points at. |
| `ci_check_producer_corpus_present.sh` | **Modified** (`bb95e95`, +63/−) | **Repointed at the produce-mode CLI** (`ade_node --mode produce`) — the legacy `live_block_production_session` binary is superseded; the producer-corpus presence gate now tracks the current produce-mode path. (Decision recorded at `docs/planning/producer-corpus-gate-guard3-decision.md`.) |

**Cross-reference (TRACEABILITY):** the new `ci_check_consensus_input_provenance.sh` is **not** in committed HEAD TRACEABILITY (0 N-F-A bindings committed) but **is** in the working-tree refresh (6 hits, bound to CN-CINPUT-02). The committed TRACEABILITY also still cites the **retired** `ci_check_constitution_coverage.sh` (line 114, under `T-CI-01`); the working-tree refresh repoints it to `ci_check_registry_code_locus_exists.sh`. **Action:** commit the staged TRACEABILITY refresh with the cluster close. The new gate maps to **CN-CINPUT-02**; the four CINPUT rules and their tests are all present in the staged TRACEABILITY.

---

## 6. Canonical Type Registry Delta

n/a — `.idd-config.json` `canonical_type_registry` is `null`. Canonical-type rules live inline in the invariant registry under family **T**; **no family-T entries were added or removed** this window. (The `T-REC-01`/`T-REC-02` change is a `strengthened_in`-list edit, not a type add/remove — see §0/§7.)

For reference, N-F-A introduces the BLUE canonical type **`SeedEpochConsensusInputs`** (+ `PoolEntry`) with a single sole codec under CN-CINPUT-01, and the gate enum `SeedEpochConsensusSource`. These are governed by family-CN/DC invariant rules (§7), not a separate canonical-type registry file, so there is nothing to delta against here.

---

## 7. Normative / Invariant Rule Delta

Source: `docs/ade-invariant-registry.toml` (the project's canonical append-only invariant registry; `invariant_registry` in `.idd-config.json`). Counts by `^[[rules]]` entries.

> **Committed-vs-staged reconciliation.** `git show HEAD:…registry.toml` = **299** (committed HEAD has **no** CINPUT rules and the pre-N-F-A `strengthened_in` values). The **working-tree** registry = **303** (the staged close-pass: +4 CINPUT rules + the 5 carry-forward strengthenings). The deltas below describe the **staged** registry — the state the cluster close will commit.

- Rules at baseline (`67d1ccc`): **299**
- Rules at HEAD (committed `a3a0636`): **299** (no registry promotion committed yet)
- Rules at HEAD (**staged working tree**): **303**
- Net additions (staged): **+4** (`CN-CINPUT-01`, `CN-CINPUT-02`, `DC-CINPUT-01`, `DC-CINPUT-02a`)
- Removals: **0** (append-only rule discipline upheld — no rule ID dropped).

### New rules (staged)

| ID | Tier | Status | Cluster | One-line summary |
|----|------|--------|---------|------------------|
| `CN-CINPUT-01` | constraint | **enforced** | N-F-A | `SeedEpochConsensusInputs` is a single closed canonical type with a **sole** deterministic-CBOR, `BTreeMap`-ordered, version-gated, byte-canonical encoder/decoder pair; no second codec may exist. (A1) |
| `CN-CINPUT-02` | constraint | **enforced** | N-F-A | The sidecar is populated **only** on the verified-bootstrap composition path through the anchor-keyed `SnapshotStore`, built via the GREEN merge + A1 sole encoder; the forge-time path must not build/put it nor append its WAL provenance — enforced by the data-flow-resistant containment gate. **Constrains POPULATION + the forge-time fence only; does NOT assert producer CONSUMPTION** — consumption is deferred to N-F-C. (A2; `ci_check_consensus_input_provenance.sh`) |
| `DC-CINPUT-01` | derived | **partial** | N-F-A | **Warm-start VERIFICATION CAPABILITY** (authority surface, not production restart): the import is a replay-reconstructable WAL fact (additive non-chaining `SeedEpochConsensusInputsImported` variant, appended after the put = commit point); replay yields a typed `RecoveredBootstrapProvenance` (one per store/anchor, fail-closed on duplicate/mismatch); `bootstrap_initial_state(RequiredFromRecoveredProvenance)` restores + verifies fail-closed (hash, decode, anchor+epoch binding, byte-identity re-encode) or halts with no fallback. **`partial` by design** — proven on the authority surface; the **production restart path is the open obligation, deferred to N-F-C (C3)**. (A3a + A3b) |
| `DC-CINPUT-02a` | derived | **enforced** | N-F-A | **Projection equivalence:** the recovered `SeedEpochConsensusInputs` projects deterministically to the leadership-consumed `PoolDistrView` (full `LedgerView` surface; single-epoch → off-epoch returns `None`) via `PoolDistrView::from_seed_epoch_consensus_inputs`, **equivalent** to the prior operator-bundle projection for the seed epoch; recovered eta0 drives `leader_vrf_input` identically. Pure BLUE field map. **Covers the projection ONLY; producer CONSUMPTION (CE-A-4b) deferred to N-F-C.** (A4) |

> **`DC-CINPUT-02b` was deliberately NOT promoted** this cluster (it would assert producer consumption; that belongs to N-F-C). The `a`-suffix on `DC-CINPUT-02a` reserves the `02b` ID for the deferred consumption rule under N-F-C.

### Modified rules (strengthenings — staged `strengthened_in`)

Five carry-forwards, **no statement weakened**:

- **`CN-ANCHOR-01`** — `["PHASE4-N-M-A", "PHASE4-N-Y"]` → `+ "PHASE4-N-F-A"` (✓ appended). The anchor now binds the fingerprint-keyed seed-epoch sidecar.
- **`DC-ANCHOR-01`** — `["PHASE4-N-M-A", "PHASE4-N-Y"]` → `+ "PHASE4-N-F-A"` (✓ appended).
- **`CN-NODE-01`** — `["PHASE4-N-K", "PHASE4-N-M-B", "PHASE4-N-T", "PHASE4-N-Y"]` → `+ "PHASE4-N-F-A"` (✓ appended). Sidecar populate routes through the single closed bootstrap authority.
- **`T-REC-01`** — baseline `["PHASE4-N-R-A"]` → staged `["PHASE4-N-F-A"]`. ⚠ **append-only concern — N-R-A dropped; should be `["PHASE4-N-R-A", "PHASE4-N-F-A"]`.**
- **`T-REC-02`** — baseline `["PHASE4-N-R-A"]` → staged `["PHASE4-N-F-A"]`. ⚠ **same append-only concern.**

### Honest residual (cluster scope)

PHASE4-N-F-A proves, mechanically and at the **authority surface**: the seed-epoch consensus inputs are a single closed canonical type with a sole codec (CN-CINPUT-01); they are populated only at the verified-bootstrap composers and the forge-time path is fenced (CN-CINPUT-02); the import is a replay-reconstructable WAL fact and warm-start **verifies** it fail-closed (DC-CINPUT-01); and the recovered surface **projects** equivalently to the leadership `PoolDistrView` (DC-CINPUT-02a). It does **NOT** prove — and does not claim — that the **producer consumes** the recovered surface (CE-A-4b) or that a **production restart mode** threads the warm-start path (DC-CINPUT-01's open obligation). Both are explicitly **deferred to PHASE4-N-F-C**; A5-SCOPING is that handoff. **BA-02 remains unsatisfied by this cluster** — `produce_mode` still cold-starts from `--consensus-inputs-path`.

---

## 8. Post-Close Tail (`b1c2267` → `d7192e2`)

**Not a cluster.** Seven housekeeping commits after the PHASE4-N-Z close — registry hygiene + a CI gate retirement/repoint pass + decision records. **No `crates/**/*.rs` behavior change, no new invariant rule** (registry stays **299** across the tail).

| Hash | Type | Summary |
|------|------|---------|
| `d7192e2` | docs | `docs(registry):` reconcile T-CI-01 after retiring foreign gate |
| `a2af041` | ci | `ci:` retire foreign constitution-coverage gate, fold coherence checks |
| `bb95e95` | ci | `ci:` repoint producer corpus gate at produce-mode CLI |
| `d5ecfe5` | docs | `docs(planning):` record pending red-gate repair decisions |
| `e8bde40` | docs | `docs(hygiene):` correct stale produce-mode forge claims + archived-cluster CI path |
| `663e001` | docs | `docs(registry):` reclassify RO-MITHRIL-IMPORT-01 item (a) — documented-interface, Tier-4 non-goal |
| `b1c2267` | docs | `docs(grounding):` refresh CODEMAP/TRACEABILITY/SEAMS/HEAD_DELTAS for PHASE4-N-Z |

### `a2af041` + `d7192e2` — retire foreign gate, fold coherence, reconcile `T-CI-01`

The N-Z window's grounding pass left a **foreign ziranity-v3 import**, `ci_check_constitution_coverage.sh` (290 lines), in `ci/`. `a2af041` **deletes** it and **folds** its registry-coherence responsibilities (unique ids, directed `cross_ref`-target resolution) into `ci_check_registry_code_locus_exists.sh` (which already did the `code_locus`-path-exists drift-guard). `d7192e2` then **repoints** `T-CI-01`'s `ci_script` from the deleted gate to `ci_check_registry_code_locus_exists.sh` and rewrites its `open_obligation` to record that the coherence checks now support — but do not fully enforce — the rule (decision record: `docs/planning/registry-cross-ref-bidirectional-repair.md`). **This is a path/`ci_script` rewrite in place + a foreign-file deletion — no rule statement / `tests` array / status weakened; append-only discipline intact.** Net `ci/ci_check_*.sh` count is flat (the N-F-A gate `+1` lands separately; tail is `−1` foreign, with the fold being a modify-in-place).

### `bb95e95` — repoint producer corpus gate at produce-mode CLI

`ci_check_producer_corpus_present.sh` is repointed from the legacy `live_block_production_session` binary at the current `ade_node --mode produce` path (the legacy binary is superseded; the registry `CN-CONS-06`/`RO-LIVE-01` code-loci already reflect this). Guard-3 decision recorded at `docs/planning/producer-corpus-gate-guard3-decision.md`.

### `663e001` — RO-MITHRIL-IMPORT-01 item (a) reclassification

Reclassifies item (a) (native decode of Mithril ancillary / UTXO-HD / LedgerDB bytes) as a **Tier-4 non-goal** for the bounty path, satisfied instead by the **documented-interface path** (Mithril-bootstrapped peer → documented `cardano-cli`/query extraction → Ade `seed_import` → CN-MITHRIL-01/DC-MITHRIL-02 binding). Adds the runbook `docs/active/mithril-documented-interface-runbook.md` (+80). The rule stays `partial`; `open_obligation` rewritten to `blocked_until_mithril_documented_evidence` (item (c), a committed reproducible documented-interface fixture/evidence bundle, remains). **A decision record + `open_obligation` rewrite — no rule removed, no status flip to enforced.**

### `e8bde40` / `d5ecfe5` — hygiene + decision records

`e8bde40` corrects stale `produce_mode` forge claims + an archived-cluster CI path in docs and touches `.idd-config.json` (a doc-field correction; **not** the `head_deltas_baseline` bump — that is handled separately at the N-F-A close). `d5ecfe5` records pending red-gate repair decisions (planning doc only). No source behavior, no rule delta.

> **Anomaly check (tail):** removals — registry rule count unchanged across the tail (299); the only "removal" is the **foreign** `ci_check_constitution_coverage.sh` file (intentional cleanup, coherence folded forward) and the `prior_fp()`/`post_fp()` accessor methods land in N-F-A A3a (§0), not the tail. No discipline violation in the tail.
