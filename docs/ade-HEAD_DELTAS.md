# HEAD Deltas — Ade

> **Status:** Living architectural document. Regenerated; not hand-edited.
> Regenerate via `/head-deltas <baseline>`. Baseline is declared in
> `.idd-config.json` (`head_deltas_baseline`).

> Baseline: `d509f02` (Phase 3 handoff snapshot, 2026-04-15)
> HEAD: `78da6c9` (chore(ci): close Phase 4 N-D CI gap — 3 new scripts, 9 rules enforced, 2026-05-19)
> 14 commits, 47 files changed, +7449 / −96 lines

The delta covers six threads of work, in roughly this proportion of the
change budget:

1. **Phase 4 cluster N-D (ChainDB persistence)** — the substantive code
   work. Slices S-33 through S-36 shipped end-to-end as `feat(phase-4):`
   commits. S-37 (1,000-kill-9 durability stress harness) has code
   landed under a chore commit but its obligation-discharge doc remains
   uncommitted; the slice is not-yet-closed and CE-N-D-1 is the gating
   obligation.
2. **Phase 2C close-out / CE-73 reclassification** — single commit
   splitting CE-73 into a Tier-2 semantic gate (now enforced via new
   `ci_check_hfc_translation.sh`) and an explicit Tier-4 bytes non-goal.
3. **IDD canonicalization** — four `chore(idd)` commits that make the
   repo legible to the global IDD slash commands: `.idd-config.json`,
   registry rename (`constitution_registry.toml` → `docs/ade-invariant-registry.toml`),
   cluster N-D moved into `docs/clusters/PHASE4-N-D/`, repo-local
   commit-msg trailer hook.
4. **Grounding-doc generation + ripple** — `a87c3a3` produced the first
   cuts of CODEMAP, SEAMS, HEAD_DELTAS, and TRACEABILITY at the
   canonical `docs/ade-*.md` paths; `f0b0fd6` refreshed HEAD_DELTAS and
   SEAMS after the BLUE-scope closure.
5. **BLUE-list drift closure** — `5b70bee` extended six CI scripts from
   a 4-crate (or 5-crate for `dependency_boundary`) BLUE scope to the
   full 6-crate scope declared in `.idd-config.json`, then `c8fa37f`
   refreshed CODEMAP and TRACEABILITY to remove 14 `_(scope gap)_`
   markers across 13 rules.
6. **Phase 4 N-D CI gap closure** — `78da6c9` added three new RED-scope
   CI scripts (`ci_check_chaindb_contract.sh`,
   `ci_check_recovery_contract.sh`, `ci_check_chaindb_crash_safety.sh`)
   for the N-D recovery surface and flipped nine registry rules from
   `declared` → `enforced` (T-REC-01, T-REC-02, DC-STORE-01,
   DC-STORE-02, DC-STORE-03, DC-STORE-05, CN-STORE-03, CN-STORE-04,
   CN-STORE-05). DC-STORE-04 was left `declared` with an explanatory
   Tier-5-divergence comment block — a comment edit, not a rule edit,
   per the IDD no-weakening discipline.

---

## 1. Commit Log

| Hash | Type | Summary |
|------|------|---------|
| `78da6c9` | chore | chore(ci): close Phase 4 N-D CI gap — 3 new scripts, 9 rules enforced |
| `f0b0fd6` | chore | chore(idd): refresh HEAD_DELTAS + SEAMS to align with BLUE-scope closure |
| `c8fa37f` | chore | chore(idd): refresh CODEMAP + TRACEABILITY after BLUE-list drift closure |
| `5b70bee` | chore | chore(ci): close BLUE-list drift — extend 6 CI scripts to full BLUE scope |
| `a87c3a3` | chore | chore(idd): generate four grounding docs (CODEMAP, SEAMS, HEAD_DELTAS, TRACEABILITY) |
| `3eddcbb` | chore | chore(idd): add .idd-config.json — opt the repo into IDD enforcement |
| `76c1f64` | chore | chore(idd): move in-flight cluster N-D into canonical clusters layout |
| `39865f6` | chore | chore(idd): update active-doc + CI refs to canonical registry path |
| `2047c42` | chore | chore(idd): commit-msg hook + CLAUDE.md trailer-override note |
| `5eecc8a` | feat | feat(phase-4): snapshot + forward-replay recovery (S-36) |
| `e52fe9f` | feat | feat(phase-4): SnapshotStore trait + impls (S-35) |
| `fb4a5d4` | feat | feat(phase-4): persistent ChainDb backed by redb (S-34) |
| `994203b` | feat | feat(phase-4): begin cluster N-D — ChainDb trait + InMemoryChainDb (S-33) |
| `9b15378` | feat | feat(phase-2c): reclassify CE-73 — semantic enforced, bytes Tier 4 non-goal |

Verbatim from `git log d509f02..HEAD`. Aggregation is in §3 and §5.

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

CODEMAP cross-reference: all three new modules are entered in
`docs/ade-CODEMAP.md` §RED at HEAD.

---

## 3. Modules Modified

| Module | Scope | Key changes |
|--------|-------|-------------|
| `ade_runtime` (root crate) | +13 files, +2163 lines | Crate went from empty shell to substantive RED module set. New top-level submodules `chaindb` and `recovery` exported from `lib.rs`. New bin target `chaindb_kill_target` and integration test `tests/stress_kill_harness.rs` (S-37 in-flight). New deps: `ade_types` (path), `redb = "2"`, `tempfile = "3"` (dev). All landed across S-33 / S-34 / S-35 / S-36 / S-37. Tier isolation continuously verified: `rg "redb\|rocksdb\|sled\|sqlite" crates/ade_runtime/src/` matches only `chaindb/persistent.rs`. |
| `ade_ledger` | +1 file, +73 lines | Single change: 10 new unit tests for `decode_invalid_tx_indices` in `plutus_eval.rs` covering empty/definite/indefinite CBOR arrays, duplicate-collapse via BTreeSet, malformed headers, non-uint truncation in both definite and indefinite paths. Shipped piggybacked on the CE-73 reclassification commit (`9b15378`). No production-code change in this crate. |

No other crate had non-trivial source changes since baseline. `ade_codec`,
`ade_types`, `ade_crypto`, `ade_plutus`, `ade_core`, `ade_testkit`,
and `ade_node` were untouched by code commits. `ade_plutus`'s
`evaluator.rs` is **referenced** by the BLUE-scope CI extension
(see §5) as the named chokepoint for `PlutusScript::from_cbor` but
the source file itself was not modified.

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
in this repo). At baseline there were 15 scripts. At HEAD there are 19:
the CE-73 reclassification added one (`ci_check_hfc_translation.sh`),
and the Phase 4 N-D CI gap closure (`78da6c9`) added three more
(`ci_check_chaindb_contract.sh`, `ci_check_recovery_contract.sh`,
`ci_check_chaindb_crash_safety.sh`). The prior CODEMAP refresh at
`c8fa37f` recorded the count as 16 (15 baseline + 1 from CE-73); the
next CODEMAP refresh will move 16 → 19. One new repo-local git hook
also shipped, six scripts had their BLUE-scope arrays extended, and
one had a path-only registry edit. Grouped by cluster.

### CE-73 reclassification (Phase 2C close-out)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_hfc_translation.sh` | **New** (`9b15378`) | CE-73-semantic gate: runs the three HFC ledger-side translation proof surfaces — `translation_summary_proof` (22/22 encoding-independent fields match oracle at Allegra→Mary), `translation_comparison_surface`, `transition_proof_surface`. Authoritative test for invariant `DC-EPOCH-02` (status flipped `partial → enforced` in the registry in the same commit). |

### IDD canonicalization (post-Phase-4-N-D)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_constitution_coverage.sh` | Modified (`39865f6`) | Path-only edit: `REGISTRY` and Python `REGISTRY_PATH` default now point at `docs/ade-invariant-registry.toml` (was `constitution_registry.toml`). What it enforces is unchanged: registry coverage against `PLAN_DOC` and `CLASSIFICATION_TABLE`. CI-script verified locally at 147 entries. |
| `ci/git-hooks/commit-msg` | **New** (`2047c42`) | Local git hook (not a CI script proper): rejects commit messages lacking a `Co-Authored-By: Claude ...` trailer. Activated per clone via `git config core.hooksPath ci/git-hooks`. Skip-conditions: `Merge`/`Revert` first lines and merge-message file present. Bypass via `--no-verify`. Repo-local exception to the global no-AI-attribution rule, scoped to commit messages only. |

### BLUE-list drift closure (`5b70bee`)

Surface drift surfaced by the grounding-doc generation step: six CI
scripts hard-coded a narrower `BLUE_CRATES` array than the 6-crate set
declared in `.idd-config.json` `core_paths`. Five scripts scanned only
4 crates (`ade_codec`, `ade_types`, `ade_crypto`, `ade_core`);
`ci_check_dependency_boundary.sh` scanned 5 (missing `ade_plutus`
only). All six were extended to the full 6-crate set (`ade_codec`,
`ade_types`, `ade_crypto`, `ade_core`, `ade_ledger`, `ade_plutus`).

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_module_headers.sh` | Modified — BLUE-scope (`5b70bee`) | `// Core Contract:` first-line header on every `.rs` in BLUE crates. Scope now includes `ade_ledger` and `ade_plutus`; in-script header comment updated to match. Enforces `T-BUILD-01`. |
| `ci/ci_check_no_semantic_cfg.sh` | Modified — BLUE-scope (`5b70bee`) | No semantic `#[cfg(...)]` in BLUE crate `src/`. Scope extended to `ade_ledger` + `ade_plutus`. Enforces `T-BUILD-01`. |
| `ci/ci_check_no_signing_in_blue.sh` | Modified — BLUE-scope (`5b70bee`) | No `SigningKey`/signing primitives in BLUE crates. Scope extended to `ade_ledger` + `ade_plutus`. Enforces `T-KEY-01`. |
| `ci/ci_check_hash_uses_wire_bytes.sh` | Modified — BLUE-scope (`5b70bee`) | All hashing in BLUE goes via wire-byte fingerprint surfaces. Scope extended to `ade_ledger` + `ade_plutus`. Enforces `T-ENC-01`, `DC-CBOR-02`. |
| `ci/ci_check_ingress_chokepoints.sh` | Modified — BLUE-scope + named-chokepoint registry growth (`5b70bee`) | No raw CBOR decoding outside named chokepoints in BLUE. Scope extended; the script also now lists `PlutusScript::from_cbor` (in `ade_plutus`) alongside the per-era block decoders (in `ade_codec`), and Check 3 explicitly allowlists `crates/ade_plutus/src/evaluator.rs` because Plutus script CBOR is a distinct ingress surface from block CBOR (decoded via aiken/pallas). Named-chokepoint registry grew 10 → 11. Enforces `T-INGRESS-01`, `DC-INGRESS-01`. |
| `ci/ci_check_dependency_boundary.sh` | Modified — BLUE-scope (`5b70bee`) | BLUE crates must not depend on RED crates. Scope previously held 5 crates and was missing `ade_plutus` only; extended to 6. Enforces `T-BOUND-02`. |

All six scripts pass at extended scope. The follow-up commit `c8fa37f`
re-ran CODEMAP and TRACEABILITY generation against the new scope,
removing 14 `_(scope gap)_` markers across 13 rules (`T-ENC-01`,
`T-BUILD-01`, `T-BOUND-02`, `T-INGRESS-01`, `T-KEY-01`, `DC-CBOR-02`,
`DC-CRYPTO-02`, `CN-WIRE-01`, `CN-WIRE-06`, `CN-PLUTUS-04`,
`CN-CRYPTO-01`, `CN-BUILD-01`, `CN-BUILD-02`). The fully-enforced
rule count rose by ~13 (CI-with-caveat → CI-no-caveat); code/tests
gaps were not touched by this commit.

### Phase 4 N-D CI gap closure (`78da6c9`)

TRACEABILITY surfaced that the Phase 4 N-D recovery surface had landed
code in `ade_runtime::{chaindb,recovery}` and tests in
`tests/stress_kill_harness.rs`, but no dedicated CI scripts existed
to drive them — leaving nine recovery/storage rules in a "code +
tests filled, `ci_script` empty" partial-enforcement state. This
commit added three RED-scope CI scripts, one per surface, and flipped
those nine rules to `enforced` in the same atomic edit.

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_chaindb_contract.sh` | **New** (`78da6c9`) | Runs `cargo test -p ade_runtime --lib chaindb::` — the 8-test bundle exercising the `run_contract_tests` and `run_snapshot_contract_tests` suites against both `InMemoryChainDb` and `PersistentChainDb`. Enforces `DC-STORE-02` (append-only finalized provenance), `DC-STORE-03` (atomic snapshots), `CN-STORE-04` (atomic checkpoints), `CN-STORE-05` (finalized provenance append-only). |
| `ci/ci_check_recovery_contract.sh` | **New** (`78da6c9`) | Runs `cargo test -p ade_runtime --lib recovery::` — the 6-test bundle covering `recover_from_snapshot_and_replay_forward`, `recover_from_genesis_when_no_snapshot`, `no_starting_point_error`, `apply_failure_surfaces_with_slot`, `snapshot_decode_failure_surfaces_as_error`, `snapshot_with_no_post_blocks_is_ok`. Enforces `T-REC-01` (recovery is replay-equivalent), `T-REC-02` (all authoritative state derivable by replay), `DC-STORE-05` (recovery is snapshot + forward replay). |
| `ci/ci_check_chaindb_crash_safety.sh` | **New** (`78da6c9`) | Runs the **smoke variant** of the subprocess-SIGKILL harness (`stress_kill_smoke`, 10 iterations) plus the `snapshot_table_intact_after_kill_loop` post-kill integrity check, plus `persistent_passes_crash_safety_with_no_kill`. The **1,000-iteration closure-gate variant** (`stress_kill_1000`) remains `#[ignore]` in the harness and is run manually for CE-N-D-1 closure evidence — not invoked on every CI run. Enforces `T-REC-01`, `DC-STORE-01` (recovery from power-loss is replay-equivalent), `CN-STORE-03` (crash recovery == clean replay). |

All three pass at HEAD; the commit body records:
`cargo test -p ade_runtime --lib chaindb::` → 8 passed,
`cargo test -p ade_runtime --lib recovery::` → 6 passed,
`cargo test -p ade_runtime --test stress_kill_harness stress_kill_smoke`
→ 1 passed (plus `snapshot_table_intact_after_kill_loop`).

TRACEABILITY cross-reference: `ci_check_hfc_translation.sh` is the
enforcement for `DC-EPOCH-02`; the six BLUE-scope-extended scripts
are the enforcement for the 13 rules above; the three new N-D scripts
are the enforcement for the 9 rules listed in §7. The next
`/traceability` refresh will retire the prior audit's "9 rules with
empty `ci_script`" gap-count line.

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
- Strengthenings (`declared` / `partial` → `enforced`):
  - **`DC-EPOCH-02`** (CE-73 reclassification, commit `9b15378`):
    `status` `partial` → `enforced`; `ci_script` `""` →
    `"ci/ci_check_hfc_translation.sh"`; `authority_surface` rewritten
    to reference the CE-73-semantic (closed, Tier 2) vs CE-73-bytes
    (explicit Tier 4 non-goal) split; three evidence references added.
  - **`T-REC-01`** (Phase 4 N-D CI gap closure, commit `78da6c9`):
    `status` `declared` → `enforced`; `code_locus` populated with
    `crates/ade_runtime/src/recovery.rs,
    crates/ade_runtime/src/chaindb/crash_safety.rs`; nine `tests`
    enumerated; `ci_script` populated with
    `ci/ci_check_recovery_contract.sh, ci/ci_check_chaindb_crash_safety.sh`.
  - **`T-REC-02`** (`78da6c9`): `declared` → `enforced`; bound to
    `crates/ade_runtime/src/recovery.rs`, two tests,
    `ci/ci_check_recovery_contract.sh`.
  - **`DC-STORE-01`** (`78da6c9`): `declared` → `enforced`; bound to
    `crates/ade_runtime/src/chaindb/crash_safety.rs` and
    `crates/ade_runtime/tests/stress_kill_harness.rs`, four tests,
    `ci/ci_check_chaindb_crash_safety.sh`.
  - **`DC-STORE-02`** (`78da6c9`): `declared` → `enforced`; bound to
    `crates/ade_runtime/src/chaindb/persistent.rs,
    crates/ade_runtime/src/chaindb/contract.rs`, three tests,
    `ci/ci_check_chaindb_contract.sh`.
  - **`DC-STORE-03`** (`78da6c9`): `declared` → `enforced`; bound to
    `crates/ade_runtime/src/chaindb/snapshot_contract.rs,
    crates/ade_runtime/src/chaindb/persistent.rs`, four tests,
    `ci/ci_check_chaindb_contract.sh`.
  - **`DC-STORE-05`** (`78da6c9`): `declared` → `enforced`; bound to
    `crates/ade_runtime/src/recovery.rs`, four tests,
    `ci/ci_check_recovery_contract.sh`.
  - **`CN-STORE-03`** (`78da6c9`): `declared` → `enforced`; bound to
    `crates/ade_runtime/src/chaindb/crash_safety.rs` and
    `crates/ade_runtime/tests/stress_kill_harness.rs`, four tests,
    `ci/ci_check_chaindb_crash_safety.sh`.
  - **`CN-STORE-04`** (`78da6c9`): `declared` → `enforced`; bound to
    `crates/ade_runtime/src/chaindb/snapshot_contract.rs,
    crates/ade_runtime/src/chaindb/persistent.rs`, four tests,
    `ci/ci_check_chaindb_contract.sh`.
  - **`CN-STORE-05`** (`78da6c9`): `declared` → `enforced`; bound to
    `crates/ade_runtime/src/chaindb/persistent.rs,
    crates/ade_runtime/src/chaindb/contract.rs`, three tests,
    `ci/ci_check_chaindb_contract.sh`.
- Annotated but unchanged:
  - **`DC-STORE-04`** (`78da6c9`): 12-line comment block appended
    *above* the rule entry explaining that the rule names
    cardano-node's literal three-DB topology (ImmutableDB +
    VolatileDB + LedgerDB) and that Ade has deliberately diverged
    per CE-79 Tier 5 (single redb-backed store with logical
    separation via key prefixes). The semantic guarantees
    DC-STORE-04 names — append-only finalized data, atomic
    snapshots — survive and are now enforced by DC-STORE-02 and
    DC-STORE-03 (both flipped to `enforced` in the same commit).
    The rule entry itself (status, code_locus, tests, ci_script)
    is **not modified** — the comment block is metadata above the
    `[[rules]]` table header. IDD discipline forbids weakening a
    rule in place; reclassification (if pursued) requires an
    explicit new strengthening entry (e.g., `DC-STORE-04A`).
  - 13 further rules had their TRACEABILITY rows annotated by
    `c8fa37f` to remove `_(scope gap)_` markers — these are
    TRACEABILITY-document edits, not registry edits.

Net strengthening this delta: **10 rules** flipped `declared`/`partial`
→ `enforced` (1 from `9b15378` + 9 from `78da6c9`). All are permitted
strengthenings, not weakenings. No rule IDs were retired or reassigned.
Family counts unchanged (T=30, DC=37, CN=64, RO=6, OP=7; remaining 3
attributed to test-stub / placeholder families per `/traceability`
report).

Normative-doc rule extraction (the `normative_docs` list in
`.idd-config.json`) is approximate and not regenerated here — the
structured registry is the authoritative source.

---

## Anomalies and Cross-Reference Warnings

- **S-37 in-flight at HEAD.** Slice S-37 code landed in `2047c42`
  (`chaindb_kill_target.rs`, `tests/stress_kill_harness.rs`) under a
  chore commit, not a `feat(phase-4)` commit. The slice's
  obligation-discharge doc remains untracked in the working tree
  (`docs/active/S-37_obligation_discharge.md`). The smoke variant of
  the harness is now wired into CI by `ci_check_chaindb_crash_safety.sh`
  (`78da6c9`), but the 1,000-iteration closure-gate variant remains
  `#[ignore]` and is the gating obligation for CE-N-D-1. Treat the
  slice as not-yet-closed until that variant is run for closure
  evidence.
- **CODEMAP stale on CI-script count.** CODEMAP §"CI enforcement"
  table header at HEAD still reads "(16 scripts under `ci/`)"; the
  actual count at HEAD is 19 after `78da6c9`'s three new RED-scope
  scripts. Flagged for the next `/codemap` run.
- **TRACEABILITY stale on 9 rule status flips.** The prior
  TRACEABILITY at HEAD (refreshed by `c8fa37f`) was generated before
  `78da6c9` flipped the 9 N-D rules to `enforced`. The audit's gap
  counts will need to be regenerated. Commit body of `78da6c9`
  explicitly calls this out.
- **SEAMS stale on PlutusScript::from_cbor — closed.** The previous
  HEAD_DELTAS flagged this; `f0b0fd6` refreshed SEAMS to add
  `PlutusScript::from_cbor` to §3 closed registries (16 → 17) and
  §4 frozen contracts (9 → 10). No longer an anomaly.
- **`ade_core` is BLUE by config but empty.** Acknowledged in CODEMAP
  callout and TRACEABILITY; treated as a CE-79 Tier-4 non-goal
  (no enforcement to perform, flagged so a reviewer doesn't mistake
  the empty crate for missing work). Not new in this delta.
- **`ade_node` MUST NOT list is forward-looking.** Binary is a
  hello-world stub; no authority surface exercised yet. Cluster N-E
  (ledger + runtime composition) will activate it. Not new in this delta.
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
