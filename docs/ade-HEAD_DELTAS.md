# Ade — HEAD Deltas (Changes Since Baseline)

> **Status:** Living architectural document. Regenerated; not hand-edited.
>
> Regenerate with `/head-deltas <baseline>` after every cluster close. Baseline is recorded in `.idd-config.json` `head_deltas_baseline`.

> Baseline: `273c887` (no tag, 2026-05-29 04:20:00 +0700)
> HEAD: `3b78008` (Close PHASE4-N-Y — Mithril-anchored bootstrap, network forward-sync & WAL recovery, 2026-05-29 14:59:36 +0700)
> 13 commits, 47 files changed, +6273 / -257 lines

This window is three pieces:

1. The **PHASE4-N-X tail** (`86ddc4d` + `c83f2ba`) — **no code or behavior change.** `86ddc4d` is the N-X *close*-pass grounding-doc refresh (CODEMAP / SEAMS / HEAD_DELTAS / TRACEABILITY regenerated at the N-X HEAD, cluster docs archived). `c83f2ba` seeded the operator-pass live-leg C1 scoping follow-on as a planning note. These two account for several of the `docs/` rows in `git diff --stat`.
2. The **PHASE4-N-Y scope** (`461c912`) — the cluster spec + slice plan + cluster/slice docs (planning only).
3. The **PHASE4-N-Y implementation** — the only code change in the window: S1 Mithril import authority + seed provenance (`9a97d34`); the DC-STORE-09 anchor-constant disambiguation fix (`51b6c4f`); S2 durable network forward-sync (`a42bfe2`); S3 end-to-end crash-recovery wiring (`09a49ed`); S4 Conway-genesis bootstrap source (`bb0d1fe`); S5 observable-surface compatibility evidence (`4b747cb`); the two security-HIGH remediations caught at cluster-close — S6 recovery↔WAL-tail reconciliation (`cb7da89`) and S7 real (non-tautological) Mithril binding (`dc8fc8c`); the registry close (`fb2c312`); and the cluster close (`3b78008`).

> **Baseline bump (this close):** on the PHASE4-N-Y close, `.idd-config.json` `head_deltas_baseline` should be bumped from `273c887` to **`3b78008`** so the next cluster narrates from this point. (That config edit is made separately, outside this regeneration.)

---

## 1. Commit Log

Verbatim from `git log --oneline --no-merges 273c887..HEAD`, newest-first. Type is the conventional-commits prefix on the subject; no editorial.

| Hash | Type | Summary |
|------|------|---------|
| `3b78008` | — | Close PHASE4-N-Y — Mithril-anchored bootstrap, network forward-sync & WAL recovery |
| `fb2c312` | docs | PHASE4-N-Y close — 6 new rules enforced + strengthenings |
| `dc8fc8c` | fix | PHASE4-N-Y S7 — real (non-tautological) Mithril binding (HIGH) |
| `cb7da89` | fix | PHASE4-N-Y S6 — recovery reconciles chaindb to the WAL tail (HIGH) |
| `4b747cb` | feat | PHASE4-N-Y S5 — observable-surface compatibility evidence |
| `bb0d1fe` | feat | PHASE4-N-Y S4 — Conway-genesis bootstrap source |
| `51b6c4f` | fix | disambiguate anchor schema-version constant (DC-STORE-09) |
| `09a49ed` | feat | PHASE4-N-Y S3 — end-to-end crash recovery wiring |
| `a42bfe2` | feat | PHASE4-N-Y S2 — durable network forward-sync lifecycle |
| `9a97d34` | feat | PHASE4-N-Y S1 — Ade-side Mithril import authority + seed provenance |
| `461c912` | docs | scope PHASE4-N-Y Mithril-anchored bootstrap + forward-sync + WAL recovery |
| `c83f2ba` | docs | seed the operator-pass live-leg C1 scoping follow-on |
| `86ddc4d` | docs | refresh CODEMAP/SEAMS/HEAD_DELTAS/TRACEABILITY for PHASE4-N-X |

Type histogram: feat ×4, fix ×3, docs ×5. Unclassified by prefix: 1 — `3b78008` ("Close PHASE4-N-Y …") carries no conventional-commits prefix; its diff is the cluster-close pass (registry/grounding-doc refresh + cluster-doc archive), so it is `docs`-by-scope. (`51b6c4f` and the two `fix(...)` HIGH commits all carry a `fix:` prefix.)

---

## 2. New Modules

Eight new modules this window, all PHASE4-N-Y. Three BLUE (authoritative core), four RED (shell), one GREEN (evidence). Colors follow `.idd-config.json` `core_paths`: `ade_ledger` is BLUE; `ade_runtime` is RED; `ade_testkit` is neither core nor shell (GREEN test-evidence).

| Module | Color | Purpose | Key sub-paths | Added in (cluster/slice) |
|--------|-------|---------|---------------|--------------------------|
| `ade_ledger::bootstrap_anchor::binding` | BLUE | The pure `verify_mithril_binding` predicate (CN-MITHRIL-01 / DC-MITHRIL-01): cross-checks a Mithril manifest's attested `{network_magic, genesis_hash, certified_point, certificate_hash}` against the independently-minted `BootstrapAnchor`; fails closed (typed `MithrilImportError`) on any field mismatch before storage init. Never re-verifies the STM multisig. | `binding.rs` (`verify_mithril_binding`, `MithrilManifestReport`, closed `MithrilImportError`) | PHASE4-N-Y / `9a97d34` (S1), hardened `dc8fc8c` (S7) |
| `ade_ledger::genesis_source` | BLUE | The pure Conway-genesis → canonical-initial-state transform (DC-GENESIS-SRC-01): turns a controlled `ConwayGenesisConfig` into the `(LedgerState, PraosChainDepState)` cold-start pair the single bootstrap authority feeds into its `genesis_initial` branch. Conway-only — non-Conway fails closed. | `genesis_source.rs` (`genesis_initial_state`, `ConwayGenesisConfig`, `GenesisInitialFund`, `GenesisSourceError`) | PHASE4-N-Y / `bb0d1fe` (S4) |
| `ade_runtime::mithril_import` | RED | The mithril-client import shell (S1): consumes a mithril-client-verified snapshot manifest and maps it to `SeedProvenance::Mithril` + the observed anchor field-set. Makes no semantic decision (the BLUE predicate decides) and never re-verifies the STM multisig. | `mithril_import/mod.rs`, `importer.rs` (`import_mithril_manifest`, `MithrilManifestError`, `MithrilProvenanceImport`), `json.rs` | PHASE4-N-Y / `9a97d34` (S1) |
| `ade_runtime::forward_sync` | RED+GREEN | Durable network forward-sync lifecycle (DC-SYNC-01), two-driver split mirroring `session`/`mux_pump`. The GREEN `reducer` composes the BLUE admit chokepoint and emits a closed `SyncEffect` plan whose `AdvanceTip` is unreachable until `StoreBlockBytes` + `AppendWal` precede it; the RED `pump` applies the plan in order against `ChainDb` + WAL and fail-closes (`TipBeforeDurable`) on any out-of-order apply. | `forward_sync/mod.rs`, `reducer.rs` (GREEN — `forward_sync_step`, `SyncEffect`, `AdmitPlan`, `ForwardSyncState`), `pump.rs` (RED — `pump_block`, `PumpError`, `PumpTip`, `SnapshotSink`) | PHASE4-N-Y / `a42bfe2` (S2) |
| `ade_runtime::genesis_bootstrap` | RED | The Conway-genesis bootstrap entry (S4): composes the RED `genesis_parser` (file read), the BLUE `genesis_source` transform, and the single closed `bootstrap_initial_state` authority; mints the `BootstrapAnchor` with `SeedProvenance::CardanoCliJson`. Introduces no parallel storage-init path and no `*Anchor` trait/plugin seam. | `genesis_bootstrap.rs` | PHASE4-N-Y / `bb0d1fe` (S4) |
| `ade_runtime::recovery::restart` | RED | Node-binary restart recovery wiring (S3): composes the existing authorities (`WalStore::read_all` → BLUE `replay_from_anchor` → warm-start `bootstrap_initial_state`) into a single end-to-end crash-recovery entry — no second recovery engine. S6 extended it to reconcile the ChainDB to the WAL tail so a torn `put_block`/wal-append crash cannot incorporate an un-WAL'd orphan. | `recovery/restart.rs`; `recovery/mod.rs` (the pre-existing snapshot+forward-replay primitive, `recovery.rs` → `recovery/mod.rs` directory promotion) | PHASE4-N-Y / `09a49ed` (S3), extended `cb7da89` (S6) |
| `ade_testkit::harness::sync_diff` | GREEN | Observable-surface differential harness for the snapshot→tip sync window (DC-COMPAT-01): compares Ade's per-block verdict, selected tip hash, block hash, and `query utxo`-style UTxO set against committed oracle fixtures. **Never** compares Ade's internal ledger `fingerprint` to a Haskell/serialized-state hash; deterministic over committed fixtures. | `harness/sync_diff.rs` (`BlockVerdict`, observable-surface diff types) | PHASE4-N-Y / `4b747cb` (S5) |

**Module promotion (not a new module):** `crates/ade_runtime/src/recovery.rs` → `crates/ade_runtime/src/recovery/mod.rs` (a directory promotion, `git` rename score `R099` — content preserved) to make room for the new `recovery/restart.rs` sibling.

**New non-source artifacts (corpus):** `corpus/sync/preprod_snapshot_to_tip_synthetic/` (the synthetic snapshot→tip oracle: `README.md` + `oracle_observable.toml`) and `corpus/sync/regressions/` (`README.md` — the per-mismatch regression-fixture home named by RO-SYNC-EVIDENCE-01). These back the S5 `sync_diff` harness and the evidence-manifest schema gate.

**Cross-reference (CODEMAP @ `3b78008`):** CODEMAP was regenerated at this same HEAD (`3b78008`, PHASE4-N-Y close) and **does** catalogue these modules — `ade_ledger::bootstrap_anchor::binding`, `genesis_source`, `forward_sync`, `mithril_import`, `genesis_bootstrap`, `recovery::restart`, and the GREEN `sync_diff` harness all appear in the BLUE/RED/GREEN tables and the `ade_ledger` / `ade_runtime` authority rows. No CODEMAP staleness on §2 this window.

---

## 3. Modules Modified

Modules that existed at baseline with non-trivial changes. The N-X tail (piece 1) and the N-Y scoping (piece 2) touched no code, so they produce no §3 entry. Every entry below is PHASE4-N-Y.

| Module | Scope | Key changes |
|--------|-------|-------------|
| `ade_ledger::bootstrap_anchor` (`anchor.rs`, `error.rs`, `mod.rs`) | +180 / -27 lines across 3 files | **N-Y (S1 + DC-STORE-09 fix + S7):** the anchor record is extended with the closed `SeedProvenance` enum (`CardanoCliJson` / `Mithril { certificate_hash, certified_point, immutable_range }`) carried in the canonical CBOR framing. The wire schema constant is both **renamed and bumped: `SCHEMA_VERSION` → `ANCHOR_SCHEMA_VERSION`, value `1 → 2`** (additive, version-gated — `decode_bootstrap_anchor` still rejects an unknown version fail-fast; the unknown-version negative test now splices a fresh v2 encoding). The rename (`51b6c4f`) disambiguates from the snapshot-framing `SCHEMA_VERSION` that `ci_check_snapshot_encoder_closure.sh` (DC-STORE-09) requires to live only in `snapshot/framing.rs` — the gate's generic-name grep was flagging the anchor's distinct, legitimate constant. The new `binding` and `error` surfaces are declared in `mod.rs`. |
| `ade_runtime::bootstrap_anchor` (RED) | +10 / -2 lines | **N-Y (S1):** the RED anchor-minting shell records the new `SeedProvenance` variant for a Mithril-sourced seed (the observed anchor field-set fed to the BLUE binding predicate). |
| `ade_runtime::recovery` (`recovery/mod.rs`) | +4 lines | **N-Y (S3):** the pre-existing snapshot+forward-replay primitive gains the `restart` submodule declaration (the new end-to-end restart wiring lives in the sibling `restart.rs`, §2). |
| `ade_node::admission::bootstrap` | +2 lines | **N-Y (S1/S4):** the admission bootstrap path threads the extended provenance/anchor surface (the storage-init chokepoint is unchanged — Mithril and genesis routes both enter the single `bootstrap_initial_state` authority). |
| `ade_ledger` / `ade_runtime` `lib.rs` | +4 lines total | Module declarations for the new submodules: `ade_ledger` exposes `pub mod genesis_source`; `ade_runtime` exposes `pub mod forward_sync`, `pub mod genesis_bootstrap`, `pub mod mithril_import`. |
| `ade_testkit::harness` (`mod.rs`) | +1 line | **N-Y (S5):** declares the new `sync_diff` GREEN harness submodule (§2). |

### Strengthenings recorded this window (registry `strengthened_in`)

Not new rules — fourteen cross-cutting invariant strengthenings PHASE4-N-Y carried forward (see §7), grouped by sub-system:

- **Durability / determinism core** — `T-DET-01`, `DC-STORE-01`, `DC-STORE-02`, `DC-STORE-03`, `DC-STORE-05`, `DC-CONS-20` (the durable-before-tip ordering and replay-equivalence now extend to the network forward-sync path + recovery↔WAL-tail reconciliation).
- **Bootstrap / anchor / seed authority** — `CN-NODE-01`, `CN-SEED-01`, `CN-ANCHOR-01`, `DC-ANCHOR-01`, `CN-GENESIS-01` (Mithril and Conway-genesis routes both enter the single closed bootstrap authority; the anchor schema is version-gated at v2).
- **WAL replay** — `DC-WAL-01`, `DC-WAL-02`, `DC-WAL-03` (the recovery wiring exercises the BLUE WAL replay integrity gates end-to-end).

(`RO-MITHRIL-IMPORT-01` also gained `PHASE4-N-Y` in `strengthened_in`, but its headline change is a **status flip** `declared → partial`; it is narrated as such in §7 rather than counted among the fourteen strengthenings.)

---

## 4. Feature Flags

No feature-flag deltas this window. **No `Cargo.toml`** (workspace root or any member) was modified between `273c887` and `3b78008`, so no `[features]` table, `optionalDependencies`, build tag, or `extras_require` changed. No `compile_error!`-coupled flag was introduced or removed.

---

## 5. CI Checks

Every CI check added or materially modified since baseline. CI scripts live as `ci/ci_check_*.sh` (no `.github/workflows` in this repo yet, per `.idd-config.json` `ci_dirs`). Count: **99 → 103** (+4 new, 0 modified, 0 removed).

### PHASE4-N-Y checks

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci_check_mithril_uses_bootstrap_initial_state.sh` | New (`9a97d34`, S1) | Mithril import enters the single closed bootstrap authority only (CN-MITHRIL-01) and never re-verifies the STM multisig in BLUE (DC-MITHRIL-01). Positive: the workspace references `bootstrap_initial_state(`. Negatives: no `trait *Anchor` plugin seam (`BootstrapAnchor` stays a struct); no second `pub fn bootstrap_initial_state` outside the one authority module; no `mithril`/STM-verify crate import under any BLUE crate path. Also gates `genesis_source` routing (DC-GENESIS-SRC-01). |
| `ci_check_forward_sync_chokepoint_only.sh` | New (`a42bfe2`, S2) | Forward-sync chokepoint-only + GREEN reducer purity (DC-SYNC-01). The GREEN reducer holds no I/O state (no tokio/redb/SystemTime/rand/HashMap/float); admits blocks only through the BLUE chokepoint (transitively via `receive_apply`); redb writes + WAL appends live in the RED pump, never the reducer; `AdvanceTip` is emitted only from the durable `AdmitPlan` constructor. |
| `ci_check_no_haskell_fingerprint_equality.sh` | New (`4b747cb`, S5) | Cardano compatibility proven only on observable surfaces (DC-COMPAT-01). A negative grep over the test tree: fails if any line is an equality assertion that pairs Ade's `fingerprint` with a Haskell/oracle serialized-state-hash token. Precise enough to allow the legitimate Ade-vs-Ade internal cross-path fingerprint equality (S4's `genesis_path_fp_equals_snapshot_path_fp`). |
| `ci_check_sync_evidence_manifest_schema.sh` | New (`4b747cb`, S5) | The committed snapshot→tip sync-evidence manifest schema (RO-SYNC-EVIDENCE-01, mirroring CN-OPERATOR-EVIDENCE-01). When a `docs/clusters/PHASE4-N-Y/CE-Y-SYNC-LIVE_*.toml` manifest exists, verify every required field is present and the referenced fixture's sha256 matches `fixture_file_sha256`. **Vacuously satisfied** until the operator-witnessed two-Haskell-node live pass commits a manifest — the live capture is operator action, never executed or fabricated in CI. |

**Cross-reference (all four grounding docs @ `3b78008`):** CODEMAP, TRACEABILITY, and SEAMS were all regenerated in the same PHASE4-N-Y close pass as this HEAD_DELTAS, at HEAD `3b78008`. TRACEABILITY binds the four new gates (`ci_check_mithril_uses_bootstrap_initial_state` / `ci_check_forward_sync_chokepoint_only` / `ci_check_no_haskell_fingerprint_equality` / `ci_check_sync_evidence_manifest_schema`) and the six new rule IDs (CN-MITHRIL-01, DC-MITHRIL-01, DC-SYNC-01, DC-GENESIS-SRC-01, DC-COMPAT-01, RO-SYNC-EVIDENCE-01) to their registry `ci_script`/`tests` entries; SEAMS records the new closed surfaces (`SeedProvenance`, `SyncEffect`, etc.). No cross-doc staleness this window. *(The four were generated concurrently; an interim draft of this note flagged TRACEABILITY/SEAMS as stale because they had not yet been written when this doc was drafted — corrected here.)*

---

## 6. Canonical Type Registry Delta

n/a — `.idd-config.json` `canonical_type_registry` is `null`. Canonical-type rules live inline in the invariant registry under family **T**; no family-T entries were added or removed this window.

For reference, the structural canonical-type count rose **446 → 452** (+6, all in `ade_ledger`) per the CODEMAP grep inventory — the new BLUE types `GenesisInitialFund`, `ConwayGenesisConfig`, `GenesisSourceError` (`genesis_source`), `MithrilManifestReport`, `MithrilImportError` (`bootstrap_anchor::binding`), and the `SeedProvenance` enum. This is a count delta, not a registry delta (there is no canonical-type registry file).

---

## 7. Normative / Invariant Rule Delta

Source: `docs/ade-invariant-registry.toml` (the project's canonical append-only invariant registry; `invariant_registry` in `.idd-config.json`). Counts by `^[[rules]]` entries.

- Rules at baseline (`273c887`): **292**
- Rules at HEAD (`3b78008`): **298**
- Net additions: **6** (all introduced and enforced inside this window)
- Removals: **0** (append-only discipline upheld).

### New rules

| ID | Tier | Cluster | One-line summary |
|----|------|---------|------------------|
| `CN-MITHRIL-01` | constraint | N-Y | A Mithril-sourced seed may bootstrap only after a verified binding cross-checks the manifest's attested `{network_magic, genesis_hash, certified_point, certificate_hash}` against the **independently** `--json-seed`-minted `BootstrapAnchor`, failing closed before storage init; the STM multisig is the RED mithril-client's job, never a BLUE trust root, and no mithril/STM crate is imported under any BLUE path. Status `enforced` (S1 + S7). |
| `DC-MITHRIL-01` | derived | N-Y | `verify_mithril_binding` is a pure, total, deterministic BLUE predicate (no I/O, clock, HashMap, float, or String errors); each field divergence maps to a distinct closed `MithrilImportError` variant; the compared sides come from two independent origins — never a value vs itself. Status `enforced`. |
| `DC-SYNC-01` | derived | N-Y | During network forward-sync a block's preserved wire bytes + WAL entry MUST be durable before the tip advances; admission is chokepoint-only; `AdvanceTip` is constructible only after `StoreBlockBytes` + `AppendWal` (single durable() emit site) and the RED pump fail-closes (`TipBeforeDurable`) out of order; recovery reconciles the chaindb to the WAL tail so a torn crash cannot incorporate an un-WAL'd orphan (S6). Status `enforced`. |
| `DC-GENESIS-SRC-01` | derived | N-Y | A controlled genesis enters initial state only through the single `bootstrap_initial_state` authority; the genesis→initial-state transform is a pure deterministic BLUE function; a non-Conway genesis fails closed (`GenesisSourceError::NonConwayEra`) — no Byron→Conway historical replay path; no `*Anchor` trait/plugin seam. Status `enforced`. |
| `DC-COMPAT-01` | derived | N-Y | Cardano compatibility is proven only on observable surfaces (per-block verdict, tip/block hashes, `query utxo`, transcripts) with version-pinned fixtures; asserting Ade's internal `fingerprint` == a Haskell serialized-state hash is FORBIDDEN and CI-blocked; the only valid fingerprint equality is internal Ade-vs-Ade (genesis-path == snapshot-path). Status `enforced`. |
| `RO-SYNC-EVIDENCE-01` | release | N-Y | A committed snapshot→tip sync-evidence manifest carries the closed schema (oracle versions, chain point, fixture refs, sha256, result) and its sha256 cross-checks the committed fixture (vacuous until a manifest is committed, mirroring CN-OPERATOR-EVIDENCE-01); each discovered Haskell mismatch becomes a named regression fixture under `corpus/sync/regressions/`; the two-Haskell-node private-Conway-testnet live leg is operator-witnessed. Status `partial`. |

### Status flip (release obligation)

- **RO-MITHRIL-IMPORT-01** — `status: declared → partial`; `open_obligation: "blocked_until_mithril_import_cluster" → "blocked_until_mithril_import_wiring_slice"` (rewritten to record that N-Y S1/S7 introduced the Ade-side **provenance** binding — OI-S1.1 scope A — but NOT seed-bytes-from-Mithril); `strengthened_in: [] → ["PHASE4-N-Y"]`. Remaining for `enforced`: (a) seed-bytes-from-Mithril decode (option B), (b) a wired production composition site with a CI gate asserting the bound anchor's `seed_point` originates from the `--json-seed` UTxO extraction (a re-tautologization hazard flagged in the S7 security re-review), (c) a committed reproducible Mithril fixture + CI/release evidence.

### Modified rules (strengthenings)

Fourteen rules had `PHASE4-N-Y` appended to `strengthened_in`; no statement was weakened. Grouped by sub-system (full list — see §3):

- **Durability / determinism:** `T-DET-01`, `DC-STORE-01`, `DC-STORE-02`, `DC-STORE-03`, `DC-STORE-05`, `DC-CONS-20`.
- **Bootstrap / anchor / seed:** `CN-NODE-01`, `CN-SEED-01`, `CN-ANCHOR-01`, `DC-ANCHOR-01`, `CN-GENESIS-01`.
- **WAL replay:** `DC-WAL-01`, `DC-WAL-02`, `DC-WAL-03`.

### Two security-HIGH findings caught at cluster-close and remediated

Both were surfaced by the per-cluster security review against the full N-Y diff and fixed before close — they are recorded here because they materially shaped two of the new rules:

- **S6 (`cb7da89`) — torn-write recovery reconciliation (HIGH).** The original S2/S3 wiring could, on a power-loss crash between `put_block` and the WAL append, leave the chaindb tip ahead of the WAL; warm-start recovery (which derives tip from stored blocks) would silently incorporate the un-admitted orphan and break replay-equivalence. Fix: recovery reconciles the ChainDB to the WAL tail, dropping any block past the durable WAL frontier. This is now the second clause of **DC-SYNC-01** (`recovery_torn_put_block_before_wal_append_drops_orphan`).
- **S7 (`dc8fc8c`) — tautological Mithril binding (HIGH).** The S1 binding compared a value to itself, so a snapshot certified at a different chain point than the seed dump would bind successfully. Fix: the binding now cross-checks the Mithril manifest report against the **independently** `--json-seed`-minted anchor — two genuinely different origins. This is the `attack_rationale` of **CN-MITHRIL-01** and the "never a value vs itself" clause of **DC-MITHRIL-01** (`mithril_binding_rejects_certified_point_other_than_seed_point`).

### Honest residual

N-Y proves, in-process and over committed fixtures: the Mithril provenance binding cross-check, the Conway-genesis cold-start through the single authority, the durable-before-tip forward-sync ordering, the crash-recovery↔WAL-tail reconciliation, and observable-surface (never internal-hash) compatibility. It does **not** prove a live snapshot→tip pass against real Haskell peers, nor seed-bytes-from-Mithril. The snapshot→tip live capture (`RO-SYNC-EVIDENCE-01`, CE-Y-16) is operator-witnessed and `blocked_until_operator_pass_executed`; full Mithril import (`RO-MITHRIL-IMPORT-01`) is `partial` and `blocked_until_mithril_import_wiring_slice`. Neither is a code gap in this cluster's shipped scope.
