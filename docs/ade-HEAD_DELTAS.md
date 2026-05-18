# HEAD Deltas — Ade

> **Status:** Living architectural document. Regenerated; not hand-edited.
> Regenerate via `/head-deltas <baseline>`. Baseline is declared in
> `.idd-config.json` (`head_deltas_baseline`).

> Baseline: `d509f02` (Phase 3 handoff snapshot, 2026-04-15)
> HEAD: `3eddcbb` (chore(idd): add .idd-config.json, 2026-05-19)
> 9 commits, 34 files changed, +4579 / −46 lines

The delta covers three threads of work, in roughly this proportion of the
change budget:

1. **Phase 4 cluster N-D (ChainDB persistence)** — the substantive code work.
   Slices S-33 through S-36 shipped end-to-end; S-37 (stress kill harness)
   is in-flight at HEAD with code landed but obligation-discharge doc
   uncommitted.
2. **Phase 3 close-out / CE-73 reclassification** — single commit splitting
   CE-73 into a Tier-2 semantic gate (now enforced via new CI) and an
   explicit Tier-4 non-goal.
3. **IDD canonicalization** — four `chore(idd)` commits at the tail that
   make the repo legible to the global IDD slash commands: `.idd-config.json`,
   registry rename (`constitution_registry.toml` → `docs/ade-invariant-registry.toml`),
   cluster N-D moved into `docs/clusters/PHASE4-N-D/`, repo-local
   commit-msg trailer hook.

---

## 1. Commit Log

| Hash | Type | Summary |
|------|------|---------|
| `3eddcbb` | chore | chore(idd): add .idd-config.json — opt the repo into IDD enforcement |
| `76c1f64` | chore | chore(idd): move in-flight cluster N-D into canonical clusters layout |
| `39865f6` | chore | chore(idd): update active-doc + CI refs to canonical registry path |
| `2047c42` | chore | chore(idd): commit-msg hook + CLAUDE.md trailer-override note |
| `5eecc8a` | feat | feat(phase-4): snapshot + forward-replay recovery (S-36) |
| `e52fe9f` | feat | feat(phase-4): SnapshotStore trait + impls (S-35) |
| `fb4a5d4` | feat | feat(phase-4): persistent ChainDb backed by redb (S-34) |
| `994203b` | feat | feat(phase-4): begin cluster N-D — ChainDb trait + InMemoryChainDb (S-33) |
| `9b15378` | feat | feat(phase-2c): reclassify CE-73 — semantic enforced, bytes Tier 4 non-goal |

Verbatim from `git log d509f02..HEAD`. Aggregation is in §3.

---

## 2. New Modules

| Module | Color | Purpose | Key sub-paths | Added in (cluster/slice) |
|--------|-------|---------|---------------|--------------------------|
| `ade_runtime::chaindb` | RED | Block-store abstraction and impls. Trait surface is Tier 1 (callers depend on it); backing-store choice and on-disk layout are Tier 5 (deliberate divergence from cardano-node's three-DB pattern). | `mod.rs` (ChainDb trait, SnapshotStore trait re-exports), `types.rs` (StoredBlock, ChainTip), `error.rs` (ChainDbError: Io / Corruption / SchemaMismatch / InvalidOperation), `in_memory.rs` (BTreeMap-backed Mutex-protected impl), `persistent.rs` (redb-backed impl, schema v2, single-file layout), `contract.rs` (`run_contract_tests` — 13 assertions any ChainDb impl must pass), `snapshot_contract.rs` (9-assertion suite for SnapshotStore), `crash_safety.rs` (`run_crash_safety_tests` + KillStrategy fault-injection harness) | PHASE4-N-D / S-33, S-34, S-35 |
| `ade_runtime::recovery` | RED | Composes ChainDb + SnapshotStore into a generic recovery primitive: load latest snapshot, replay blocks forward to chain tip. Generic over a `Recoverable` trait so the runtime stays decoupled from `ade_ledger`. | `recovery.rs` (Recoverable trait, StartingState, RecoveryReport, RecoveryError, `recover<C, S, R>` entry point) | PHASE4-N-D / S-36 |
| `ade_runtime` bin `chaindb_kill_target` | RED | Kill-target child process driver for the 1,000-kill-9 durability stress harness. In-flight at HEAD (slice S-37 / CE-N-D-1). | `src/bin/chaindb_kill_target.rs`, integration test at `tests/stress_kill_harness.rs` | PHASE4-N-D / S-37 (in-flight) |

Workspace-level membership is unchanged — no new crates were added; the
above are new modules within the existing `ade_runtime` crate, which prior
to this delta had effectively no source surface (`lib.rs` had only the
core-contract banner, no `pub mod` declarations, no `[dependencies]`).

The `ade_runtime` crate gained `redb = "2"` as its first runtime dependency
(Tier 5 choice per S-34 §O-34.1: pure Rust, ACID, single-file, MIT/Apache;
aligns with the cluster N-F single-static-binary goal), plus
`tempfile = "3"` as a dev-dependency for persistent-impl tests, plus a
local-path dependency on `ade_types`.

CODEMAP cross-reference: **CODEMAP does not yet exist at this path**
(`docs/ade-CODEMAP.md`). The new modules will need to be added when CODEMAP
is first generated.

---

## 3. Modules Modified

| Module | Scope | Key changes |
|--------|-------|-------------|
| `ade_runtime` (root crate) | +13 files, +2163 lines | Crate went from empty shell to substantive RED module set. New top-level submodules `chaindb` and `recovery` exported from `lib.rs`. New bin target `chaindb_kill_target` and integration test `tests/stress_kill_harness.rs` (S-37 in-flight). New deps: `ade_types` (path), `redb = "2"`, `tempfile = "3"` (dev). All landed across S-33 / S-34 / S-35 / S-36 / S-37. Tier isolation continuously verified: `rg "redb\|rocksdb\|sled\|sqlite" crates/ade_runtime/src/` matches only `chaindb/persistent.rs`. |
| `ade_ledger` | +1 file, +73 lines | Single change: 10 new unit tests for `decode_invalid_tx_indices` in `plutus_eval.rs` covering empty/definite/indefinite CBOR arrays, duplicate-collapse via BTreeSet, malformed headers, non-uint truncation in both definite and indefinite paths. Shipped piggybacked on the CE-73 reclassification commit (`9b15378`). No production-code change in this crate. |

No other crate had non-trivial changes since baseline. `ade_codec`,
`ade_types`, `ade_crypto`, `ade_plutus`, `ade_core`, `ade_testkit`,
and `ade_node` were untouched by code commits (the only modifications
to their tree were the global IDD scaffolding chores, which did not
edit their source).

---

## 4. Feature Flags

No Cargo `[features]` tables exist at HEAD in any workspace crate, and
none existed at baseline. The project does not use Cargo feature flags
as a semantic surface — closed semantic surfaces are encoded in the type
system per the IDD core principles, and conditional compilation is
checked out of BLUE code via `ci/ci_check_no_semantic_cfg.sh`.

No `#[cfg(feature = ...)]` gates appear at either ref. **Status:
unchanged — zero feature flags at baseline, zero at HEAD.**

---

## 5. CI Checks

The CI surface is the shell-script set under `ci/` (no `.github/workflows`
in this repo). All 15 baseline scripts are still present and substantively
unchanged. Two additions and one path-only edit landed in this delta;
group-aligned per cluster.

### CE-73 reclassification (Phase 2C close-out)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_hfc_translation.sh` | **New** (`9b15378`) | CE-73-semantic gate: runs the three HFC ledger-side translation proof surfaces — `translation_summary_proof` (22/22 encoding-independent fields match oracle at Allegra→Mary), `translation_comparison_surface`, `transition_proof_surface`. Authoritative test for invariant `DC-EPOCH-02` (status flipped `partial → enforced` in the registry in the same commit). |

### IDD canonicalization (post-Phase-4-N-D)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_constitution_coverage.sh` | Modified (`39865f6`) | Path-only edit: `REGISTRY` and Python `REGISTRY_PATH` default now point at `docs/ade-invariant-registry.toml` (was `constitution_registry.toml`). What it enforces is unchanged: registry coverage against `PLAN_DOC` and `CLASSIFICATION_TABLE`. CI-script verified locally at 147 entries. |
| `ci/git-hooks/commit-msg` | **New** (`2047c42`) | Local git hook (not a CI script proper): rejects commit messages lacking a `Co-Authored-By: Claude ...` trailer. Activated per clone via `git config core.hooksPath ci/git-hooks`. Skip-conditions: `Merge`/`Revert` first lines and merge-message file present. Bypass via `--no-verify`. Repo-local exception to the global no-AI-attribution rule, scoped to commit messages only. |

TRACEABILITY cross-reference: TRACEABILITY does not yet exist at this
path (`docs/ade-TRACEABILITY.md`). When generated, `ci_check_hfc_translation.sh`
must show as the enforcement for `DC-EPOCH-02`; the registry already
encodes that linkage (`ci_script = "ci/ci_check_hfc_translation.sh"`).

---

## 6. Canonical Type Registry Delta

n/a — `.idd-config.json` `canonical_type_registry` is null. Canonical-type
rules live inline in the invariant registry under family `T`.

---

## 7. Normative Rule Delta

The project's invariant registry tracks structured rules (TOML), not
prose normative-doc rules; this section reports on it. The registry
file was renamed in `2047c42` from `constitution_registry.toml` (repo
root) to `docs/ade-invariant-registry.toml` (98% similarity per
`git diff --find-renames`).

- Rules at baseline: **147** (in `constitution_registry.toml`)
- Rules at HEAD: **147** (in `docs/ade-invariant-registry.toml`)
- Net additions: **0**
- Removals: **0** (expected under append-only discipline; clean)
- Modifications: **1** — `DC-EPOCH-02` (CE-73 reclassification, commit
  `9b15378`):
  - `status`: `partial` → `enforced` (strengthened)
  - `ci_script`: `""` → `"ci/ci_check_hfc_translation.sh"`
  - `authority_surface`: rewritten to reference CE-73-semantic (closed,
    Tier 2) vs CE-73-bytes (explicit Tier 4 non-goal) split, plus the
    deferral of consensus-side HFC to Phase 4 cluster N-B.
  - `evidence`: three references added (`phase_2c_progress_report.md`,
    `CE-73_reclassification.md`, `T-26_hfc_ledger_side.md`).

This is a permitted strengthening, not a weakening — `partial → enforced`
is the legal direction. No rule IDs were retired or reassigned.

Normative-doc rule extraction (the `normative_docs` list in
`.idd-config.json`) is approximate and not regenerated here — the
structured registry is the authoritative source.

---

## Anomalies and Cross-Reference Warnings

- **CODEMAP missing.** `docs/ade-CODEMAP.md` does not yet exist. New
  modules `ade_runtime::chaindb` and `ade_runtime::recovery` (RED) and
  the in-flight `chaindb_kill_target` bin (RED) need entries when
  CODEMAP is first generated.
- **TRACEABILITY missing.** `docs/ade-TRACEABILITY.md` does not yet
  exist. The newly-enforced `DC-EPOCH-02` → `ci_check_hfc_translation.sh`
  edge needs to appear in TRACEABILITY when generated.
- **SEAMS missing.** `docs/ade-SEAMS.md` does not yet exist. The new
  `ChainDb` and `SnapshotStore` traits (Tier 1 surface) and the
  `Recoverable` extension point are SEAMS-relevant.
- **S-37 in-flight at HEAD.** Slice S-37 code landed in `2047c42`
  (`chaindb_kill_target.rs`, `tests/stress_kill_harness.rs`) under a
  chore commit, not a `feat(phase-4)` commit. The slice's
  obligation-discharge doc remains untracked in the working tree
  (`docs/active/S-37_obligation_discharge.md`). Treat the slice as
  not-yet-closed; CE-N-D-1 (1,000-kill-9 durability) is the gating
  obligation.
- No removed canonical types (n/a — no separate registry).
- No removed registry rules (expected: 0; actual: 0).
- No commit subjects in the delta lack a conventional-commits prefix.

---

## Generation Notes

Regenerate via `/head-deltas <baseline>` or by re-running the
`head-deltas-generator` agent with the same baseline. Baseline lives
in `.idd-config.json` `head_deltas_baseline`. Update on next phase
boundary (Phase 4 close, or when cluster N-D fully closes including
S-37 / CE-N-D-1).
