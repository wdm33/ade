# HEAD Deltas — Ade

> **Status:** Living architectural document. Regenerated; not hand-edited.
> Regenerate via `/head-deltas <baseline>`. Baseline is declared in
> `.idd-config.json` (`head_deltas_baseline`).

> Baseline: `d509f02` (Phase 3 handoff snapshot, 2026-04-15)
> HEAD: `56bfa7b` (feat(phase-4): close CE-N-A-5 — 4 N2C real captures + LSQ/LTS/TxSubmission2 wire-form fixes + condition 4 + 5 + S-A10 evidence script, 2026-05-19)
> 50 commits, 229 files changed, +30,403 / −147 lines

The delta covers seven threads of work, in roughly this proportion of
the change budget:

1. **Phase 4 cluster N-A (network mini-protocols)** — the largest
   substantive code drop. 10 slices (S-A1 through S-A10, with S-A8b /
   S-A8c rework slices) shipped end-to-end as `feat(phase-4):`
   commits. Introduced the new BLUE workspace crate `ade_network`
   with 11 mini-protocol codecs, 8 state machines, the Ouroboros mux
   frame codec, and a RED `session` substrate. Closed CE-N-A-1
   through CE-N-A-5 against pinned cardano-node 11.0.1, including a
   real-capture corpus at `corpus/network/{n2n,n2c}/`. Three wire-form
   codec bugs surfaced by real interop were fixed in flight
   (chain-sync RollForward era-wrap, block-fetch flat RequestRange
   triple, N2C version 0x8000 wire flag), plus an LSQ Acquire /
   AcquireNoPoint split and a LocalTxSubmission / N2N TxSubmission2
   inner-tx HFC envelope fix, plus DoS-hardening on
   `Vec::with_capacity` in eight codecs.
2. **Phase 4 cluster N-D (ChainDB persistence)** — closed in `436b1d7`.
   Slices S-33 through S-37 shipped end-to-end. CE-N-D-1 closure
   evidence (1000/1000 stress-kill iterations) is logged at
   `docs/clusters/completed/PHASE4-N-D/CE-N-D-1_2026-05-19.log`.
3. **Phase 2C close-out / CE-73 reclassification** — single commit
   splitting CE-73 into a Tier-2 semantic gate (now enforced via new
   `ci_check_hfc_translation.sh`) and an explicit Tier-4 bytes non-goal.
4. **IDD canonicalization** — four `chore(idd)` commits that make the
   repo legible to the global IDD slash commands: `.idd-config.json`,
   registry rename (`constitution_registry.toml` → `docs/ade-invariant-registry.toml`),
   cluster N-D moved into `docs/clusters/PHASE4-N-D/`, repo-local
   commit-msg trailer hook.
5. **Grounding-doc generation + ripple** — `a87c3a3` produced the first
   cuts of CODEMAP, SEAMS, HEAD_DELTAS, and TRACEABILITY at the
   canonical `docs/ade-*.md` paths; `f0b0fd6` refreshed HEAD_DELTAS
   and SEAMS after the BLUE-scope closure; `a2c7ac8` refreshed all
   three after the N-D CI closure.
6. **BLUE-list drift closure** — `5b70bee` extended six CI scripts from
   a 4-crate (or 5-crate for `dependency_boundary`) BLUE scope to the
   full 6-crate scope declared in `.idd-config.json`, then `c8fa37f`
   refreshed CODEMAP and TRACEABILITY to remove 14 `_(scope gap)_`
   markers across 13 rules.
7. **Phase 4 N-D CI gap closure** — `78da6c9` added three new RED-scope
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
| `56bfa7b` | feat | feat(phase-4): close CE-N-A-5 — 4 N2C real captures + LSQ/LTS/TxSubmission2 wire-form fixes + condition 4 + 5 + S-A10 evidence script |
| `d977640` | docs | docs(registry): wire S-A9 real-capture tests into PHASE4-N-A invariants |
| `b7cd39d` | feat | feat(phase-4): S-A9 N2C handshake + N2N keep-alive + peer-sharing real captures (3 more protocols + N2C 0x8000 wire-flag fix) |
| `a1b47ec` | feat | feat(phase-4): S-A9 block-fetch real interop + flat-range wire-form fix |
| `ef38212` | feat | feat(phase-4): S-A9 block-fetch codec wrapping fix + capture binary |
| `84d3eab` | feat | feat(phase-4): S-A9 chain-sync real capture + ChainSync codec wrapped-header fix |
| `98d0abe` | feat | feat(phase-4): S-A9 partial — real-capture corpus + handshake against mainnet relays |
| `1ba2d95` | feat | feat(phase-4): S-A8c — version table alignment with cardano-node 11.0.1 |
| `679491f` | docs | docs(phase-4): S-A8c entry obligation discharge — version table alignment with cardano-node 11.0.1 |
| `b7fade3` | feat | feat(phase-4): S-A8b — LocalTxMonitor wire-grammar rework (corrects S-A2/S-A8 misimpl) |
| `affa624` | docs | docs(phase-4): S-A8b entry obligation discharge — LocalTxMonitor wire-grammar rework |
| `9b7b96d` | docs | docs(phase-4): S-A9 + S-A10 entry obligation discharge — corpus replay harness + live interop closure gate |
| `77a02dd` | feat | feat(phase-4): S-A8 — N2C transition authority (4 state machines; structural completion) |
| `20b3554` | docs | docs(phase-4): S-A8 entry obligation discharge — N2C transition authority (4 state machines) |
| `b16329b` | feat | feat(phase-4): S-A7 — keep-alive + peer-sharing transition authority (structural completion) |
| `2cb0e86` | docs | docs(phase-4): S-A7 entry obligation discharge — keep-alive + peer-sharing transition authority |
| `844ae95` | feat | feat(phase-4): S-A6 — tx-submission2 transition authority (closes CE-N-A-4 state-machine portion) |
| `10659d5` | docs | docs(phase-4): S-A6 entry obligation discharge — tx-submission2 transition authority |
| `d702772` | feat | feat(phase-4): S-A5 — block-fetch transition authority (closes CE-N-A-3 state-machine portion) |
| `7078b9b` | docs | docs(phase-4): S-A5 entry obligation discharge — block-fetch transition authority |
| `787da55` | feat | feat(phase-4): S-A4 — chain-sync transition authority (closes CE-N-A-2 state-machine portion) |
| `7fef3a4` | docs | docs(phase-4): S-A4 entry obligation discharge — chain-sync transition authority |
| `ba02f71` | feat | feat(phase-4): S-A3 — handshake version negotiation authority (closes CE-N-A-1 state-machine portion) |
| `6faacd0` | docs | docs(phase-4): S-A3 entry obligation discharge — handshake version negotiation authority |
| `d1d47e9` | feat | feat(phase-4): S-A2 — protocol message codec authority for all 11 mini-protocols |
| `a4aabb9` | docs | docs(phase-4): S-A2 entry obligation discharge — protocol codec authority for all 11 mini-protocols |
| `4fde3a7` | feat | feat(phase-4): S-A1 — ade_network substrate + DC-CORE-01 mechanical gate |
| `22023be` | docs | docs(phase-4): S-A1 entry obligation discharge — mux/framing + sync-only CI gate |
| `6942674` | docs | docs(phase-4): open PHASE4-N-A cluster doc — wire+semantic Tier 1, 10 slices |
| `6ca2ba8` | docs | docs(phase-4): ratify PHASE4-N-A cluster plan (10 slices, authority-aligned) |
| `ae9c473` | docs | docs(phase-4): close N-A invariants §7 decisions + add DC-PROTO-06 |
| `492de56` | docs | docs(phase-4): open PHASE4-N-A — invariant sketch + DC-CORE-01 sync-only rule |
| `436b1d7` | chore | Close PHASE4-N-D — chain DB persistence with crash-equivalent recovery |
| `a3a083a` | docs | docs(phase-4): CE-N-D-1 closure evidence — 1000/1000 stress kill iterations green |
| `27960fd` | docs | docs(phase-4): lock N-A scope decisions before cluster opens |
| `a2c7ac8` | chore | chore(idd): refresh CODEMAP + TRACEABILITY + HEAD_DELTAS after N-D CI closure |
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
| `ade_network` (new workspace crate) | BLUE-majority (per-submodule scoped in `.idd-config.json` `core_paths`) | Ouroboros mini-protocol authority: 11 closed-grammar codecs, 8 pure transition state machines, Ouroboros mux frame codec, RED session/transport substrate. Wire bytes are Tier 1 — no Tier 5 latitude. Sync-only in BLUE submodules (DC-CORE-01); tokio is confined to `mux::transport`. | `codec/` (11 protocol message codecs: handshake, chain_sync, block_fetch, tx_submission, keep_alive, peer_sharing, n2c_handshake, local_chain_sync, local_state_query, local_tx_monitor, local_tx_submission + primitives + version + error); `handshake/` (version negotiation state machine, version_table.rs aligned to cardano-node 11.0.1 surface); `chain_sync/`, `block_fetch/`, `tx_submission/`, `keep_alive/`, `peer_sharing/` (BLUE pure transitions with `*State` / `*Agency` / `*Output` enums); `n2c/{local_chain_sync,local_state_query,local_tx_monitor,local_tx_submission}/` (4 N2C state machines); `mux/frame.rs` (BLUE Ouroboros mux frame encode/decode), `mux/transport.rs` (RED socket I/O — tokio first appears here), `mux/mod.rs` (GREEN glue); `session/` (RED composition — socket ↔ mux ↔ codec ↔ state); 8 RED capture binaries under `src/bin/capture_*.rs` driving live cardano-node interop | PHASE4-N-A / S-A1 (substrate) → S-A2 (codecs) → S-A3..S-A8 (state machines) → S-A8b/S-A8c (LocalTxMonitor + version-table rework) → S-A9 (real captures + wire-form fixes) → S-A10 (CE-N-A-5 evidence) |
| `ade_runtime::chaindb` | RED | Block-store abstraction and impls. Trait surface is Tier 1; backing-store choice and on-disk layout are Tier 5. | `mod.rs`, `types.rs`, `error.rs`, `in_memory.rs`, `persistent.rs` (redb-backed), `contract.rs`, `snapshot_contract.rs`, `crash_safety.rs` | PHASE4-N-D / S-33, S-34, S-35 |
| `ade_runtime::recovery` | RED | Composes ChainDb + SnapshotStore into a generic recovery primitive: load latest snapshot, replay blocks forward to chain tip. | `recovery.rs` (Recoverable trait, RecoveryReport, RecoveryError, `recover<C, S, R>`) | PHASE4-N-D / S-36 |
| `ade_runtime` bin `chaindb_kill_target` | RED | Kill-target child process driver for the 1,000-kill-9 durability stress harness. | `src/bin/chaindb_kill_target.rs`, `tests/stress_kill_harness.rs` | PHASE4-N-D / S-37 |

Workspace-level membership grew by **one crate**: `ade_network` was
added to `[workspace] members` in the root `Cargo.toml`. Its sole
runtime dependency outside the workspace is `tokio = "1"`, which
DC-CORE-01 confines to `ade_network::mux::transport`; the global CI
gate `ci/ci_check_no_async_in_blue.sh` enforces this mechanically by
scanning every BLUE submodule listed in `.idd-config.json` `core_paths`.

The `ade_runtime` crate gained `redb = "2"` (S-34, Tier 5 choice
per CE-79 addendum) plus `tempfile = "3"` (dev) plus a local-path
dependency on `ade_types`.

A real-capture corpus shipped under `corpus/network/{n2n,n2c}/` —
**11 protocol directories** (6 N2N, 5 N2C) holding ~55 CBOR frame
captures plus their TOML metadata, taken against mainnet / preprod
relays and a local cardano-node 11.0.1. The corpus is the
authoritative replay surface for CE-N-A-1 through CE-N-A-5.

CODEMAP cross-reference: all new modules listed above are entered
in `docs/ade-CODEMAP.md` at HEAD. The next `/codemap` regeneration
will need to add the `ade_network` crate's per-submodule entries
(BLUE for codec/handshake/state-machines/n2c/mux::frame; RED for
mux::transport, session, and the 8 capture binaries).

---

## 3. Modules Modified

| Module | Scope | Key changes |
|--------|-------|-------------|
| `ade_network` (new crate, large initial drop) | +~100 files, +17,861 lines | Crate created from scratch in S-A1; built out S-A2 → S-A10. Surface: 11 closed message-enum codecs (one per mini-protocol), 8 pure state machines (handshake, chain_sync, block_fetch, tx_submission, keep_alive, peer_sharing, and 4 N2C protocols), Ouroboros mux frame codec, RED transport + session substrate, 8 RED capture binaries. **Three wire-form bugs found by real interop and fixed in flight**: (a) S-A9 ChainSync codec wrapped-header — RollForward era-wrap was not preserved on round-trip (`84d3eab`); (b) S-A9 block-fetch flat-range — RequestRange was emitted as a triple instead of nested pair (`a1b47ec`, follow-up to `ef38212`); (c) S-A9 N2C 0x8000 wire-flag — version selection on N2C did not set the high bit per the cardano-node convention (`b7cd39d`). **Two additional structural reworks**: LocalTxMonitor wire-grammar (`b7fade3`, S-A8b) reworked LSQ Acquire vs AcquireNoPoint as distinct closed-grammar variants; LocalTxSubmission and N2N TxSubmission2 (`56bfa7b`, S-A10) gained the HFC envelope around inner txs. **Eight codecs hardened against DoS** by replacing unbounded `Vec::with_capacity(len)` with size-checked alternatives (`56bfa7b`). **38 integration tests** under `crates/ade_network/tests/` (19 test files: agency traces, signal traces, real-capture corpora, malformed-frame negative tests, version negotiation, frame corpora). |
| `ade_runtime` (root crate) | +13 files, +2,163 lines | Crate went from empty shell to substantive RED module set. New top-level submodules `chaindb` and `recovery` exported from `lib.rs`. New bin target `chaindb_kill_target` and integration test `tests/stress_kill_harness.rs`. New deps: `ade_types` (path), `redb = "2"`, `tempfile = "3"` (dev). All landed across S-33 / S-34 / S-35 / S-36 / S-37. Tier isolation continuously verified: `rg "redb\|rocksdb\|sled\|sqlite" crates/ade_runtime/src/` matches only `chaindb/persistent.rs`. |
| `ade_ledger` | +1 file, +73 lines | Single change: 10 new unit tests for `decode_invalid_tx_indices` in `plutus_eval.rs` covering empty/definite/indefinite CBOR arrays, duplicate-collapse via BTreeSet, malformed headers, non-uint truncation in both definite and indefinite paths. Shipped piggybacked on the CE-73 reclassification commit (`9b15378`). No production-code change in this crate. |

No other crate had non-trivial source changes since baseline.
`ade_codec`, `ade_types`, `ade_crypto`, `ade_plutus`, `ade_core`,
`ade_testkit`, and `ade_node` were untouched by code commits.
`ade_plutus`'s `evaluator.rs` is **referenced** by the BLUE-scope
CI extension (see §5) as the named chokepoint for
`PlutusScript::from_cbor` but the source file itself was not modified.

---

## 4. Feature Flags

No Cargo `[features]` tables exist at HEAD in any workspace crate
(including the new `ade_network` crate), and none existed at baseline.
The project does not use Cargo feature flags as a semantic surface —
closed semantic surfaces are encoded in the type system per the IDD
core principles, and conditional compilation is checked out of BLUE
code via `ci/ci_check_no_semantic_cfg.sh` (now scoped over the
full 6-crate BLUE set plus all `ade_network` BLUE submodules).

No `#[cfg(feature = ...)]` gates appear at either ref. **Status:
unchanged — zero feature flags at baseline, zero at HEAD.**

---

## 5. CI Checks

The CI surface is the shell-script set under `ci/` (no
`.github/workflows` in this repo). At baseline there were 15 scripts.
At HEAD there are **21**: CE-73 reclassification added one
(`ci_check_hfc_translation.sh`), Phase 4 N-D CI gap closure added
three (`ci_check_chaindb_contract.sh`, `ci_check_recovery_contract.sh`,
`ci_check_chaindb_crash_safety.sh`), and Phase 4 N-A added two
(`ci_check_no_async_in_blue.sh` from S-A1; `ci_check_ce_n_a_5_proof.sh`
from S-A10). One repo-local git hook also shipped, six scripts had
their BLUE-scope arrays extended, and one had a path-only registry
edit. Grouped by cluster.

### CE-73 reclassification (Phase 2C close-out)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_hfc_translation.sh` | **New** (`9b15378`) | CE-73-semantic gate: runs the three HFC ledger-side translation proof surfaces — `translation_summary_proof` (22/22 encoding-independent fields match oracle at Allegra→Mary), `translation_comparison_surface`, `transition_proof_surface`. Authoritative test for invariant `DC-EPOCH-02`. |

### IDD canonicalization (post-Phase-4-N-D)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_constitution_coverage.sh` | Modified (`39865f6`) | Path-only edit: `REGISTRY` and Python `REGISTRY_PATH` default now point at `docs/ade-invariant-registry.toml` (was `constitution_registry.toml`). What it enforces is unchanged: registry coverage against `PLAN_DOC` and `CLASSIFICATION_TABLE`. |
| `ci/git-hooks/commit-msg` | **New** (`2047c42`) | Local git hook (not a CI script proper): rejects commit messages lacking a `Co-Authored-By: Claude ...` trailer. Activated per clone via `git config core.hooksPath ci/git-hooks`. Repo-local exception to the global no-AI-attribution rule, scoped to commit messages only. |

### BLUE-list drift closure (`5b70bee`)

Six CI scripts hard-coded a narrower `BLUE_CRATES` array than the
6-crate set declared in `.idd-config.json` `core_paths`. All six
were extended to the full 6-crate set (`ade_codec`, `ade_types`,
`ade_crypto`, `ade_core`, `ade_ledger`, `ade_plutus`).

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_module_headers.sh` | Modified — BLUE-scope (`5b70bee`) | `// Core Contract:` first-line header on every `.rs` in BLUE crates. Enforces `T-BUILD-01`. |
| `ci/ci_check_no_semantic_cfg.sh` | Modified — BLUE-scope (`5b70bee`) | No semantic `#[cfg(...)]` in BLUE crate `src/`. Enforces `T-BUILD-01`. |
| `ci/ci_check_no_signing_in_blue.sh` | Modified — BLUE-scope (`5b70bee`) | No `SigningKey`/signing primitives in BLUE crates. Enforces `T-KEY-01`. |
| `ci/ci_check_hash_uses_wire_bytes.sh` | Modified — BLUE-scope (`5b70bee`) | All hashing in BLUE goes via wire-byte fingerprint surfaces. Enforces `T-ENC-01`, `DC-CBOR-02`. |
| `ci/ci_check_ingress_chokepoints.sh` | Modified — BLUE-scope + named-chokepoint registry growth (`5b70bee`) | No raw CBOR decoding outside named chokepoints in BLUE. Named-chokepoint registry grew 10 → 11 (added `PlutusScript::from_cbor` in `ade_plutus`). Enforces `T-INGRESS-01`, `DC-INGRESS-01`. |
| `ci/ci_check_dependency_boundary.sh` | Modified — BLUE-scope (`5b70bee`) | BLUE crates must not depend on RED crates. Enforces `T-BOUND-02`. |

Follow-up commit `c8fa37f` re-ran CODEMAP and TRACEABILITY generation
against the new scope, removing 14 `_(scope gap)_` markers across
13 rules.

### Phase 4 N-D CI gap closure (`78da6c9`)

Three new RED-scope CI scripts; nine recovery/storage rules flipped
to `enforced` in the same atomic edit.

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_chaindb_contract.sh` | **New** (`78da6c9`) | Runs `cargo test -p ade_runtime --lib chaindb::` — 8 tests across `run_contract_tests` + `run_snapshot_contract_tests`. Enforces `DC-STORE-02`, `DC-STORE-03`, `CN-STORE-04`, `CN-STORE-05`. |
| `ci/ci_check_recovery_contract.sh` | **New** (`78da6c9`) | Runs `cargo test -p ade_runtime --lib recovery::` — 6-test recovery bundle. Enforces `T-REC-01`, `T-REC-02`, `DC-STORE-05`. |
| `ci/ci_check_chaindb_crash_safety.sh` | **New** (`78da6c9`) | Smoke variant of the subprocess-SIGKILL harness (`stress_kill_smoke`, 10 iterations) plus integrity post-checks. Closure-gate variant (`stress_kill_1000`) remains `#[ignore]`. Enforces `T-REC-01`, `DC-STORE-01`, `CN-STORE-03`. |

### Phase 4 N-A wire + semantic enforcement (S-A1, S-A10)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_no_async_in_blue.sh` | **New** (`4fde3a7`, S-A1) | Mechanically enforces `DC-CORE-01` — BLUE authoritative code is sync-only. Scans every `.rs` under each BLUE path in `.idd-config.json` `core_paths` for `async fn`, `.await`, `tokio`, `async_std`, `futures::`, and `spawn` (with a `std::thread::spawn` allowlist filter). Reads BLUE paths from `.idd-config.json` `core_paths` so `ade_network`'s per-submodule BLUE scope is enforced automatically (codec/handshake/state-machines/n2c/mux::frame are scanned; mux::transport and session are not). |
| `ci/ci_check_ce_n_a_5_proof.sh` | **New** (`56bfa7b`, S-A10) | CE-N-A-5 closure-gate evidence script. Exercises all 5 conditions of the CE-N-A-5 proof obligation (PHASE4-N-A_invariants.md §6) against the captured real-cardano-node corpus: (1) handshake version negotiation succeeds/fails deterministically, (2) all 11 mini-protocols reject unsupported versions deterministically, (3) captured frames decode and re-encode byte-identically where required, (4) live peer interaction produces expected agency transitions, (5) malformed frames produce canonical structured errors. Logs results in the §7 #6 evidence schema `{ protocol_id, selected_version, canonical_bytes, output_or_error }` to `docs/active/CE-N-A-5_evidence.toml`. Authoritative gate for CE-N-A-5. |

TRACEABILITY cross-reference: `ci_check_no_async_in_blue.sh` is the
enforcement for the new `DC-CORE-01` rule (added in `492de56`);
`ci_check_ce_n_a_5_proof.sh` is the enforcement surface for the
four mini-protocol-coverage rules that gained real-capture tests
in PHASE4-N-A (T-ENC-03, CN-WIRE-07, DC-PROTO-02, DC-PROTO-05).
Their TRACEABILITY rows will move from `declared` / `partial` →
`enforced` on the next `/traceability` refresh.

---

## 6. Canonical Type Registry Delta

n/a — `.idd-config.json` `canonical_type_registry` is null. Canonical-type
rules live inline in the invariant registry under family `T`.

---

## 7. Normative Rule Delta

The project's invariant registry tracks structured rules (TOML), not
prose normative-doc rules; this section reports on it.

- Rules at baseline: **147** (in `constitution_registry.toml`)
- Rules at HEAD: **149** (in `docs/ade-invariant-registry.toml`)
- Net additions: **+2** (PHASE4-N-A scope)
  - **`DC-CORE-01`** (added in `492de56`, PHASE4-N-A invariant sketch):
    BLUE authoritative code is sync-only — no async, no tokio, no
    futures. Enforced by `ci/ci_check_no_async_in_blue.sh` (new in
    S-A1, `4fde3a7`).
  - **`DC-PROTO-06`** (added in `ae9c473`, PHASE4-N-A §7 decision
    closure): BLUE mini-protocol transitions are pure functions of
    canonical inputs with no ambient session influence (closes
    DC-PROTO-02 under DC-CORE-01).
- Removals: **0** (expected under append-only discipline; clean)
- Strengthenings (`declared` / `partial` → `enforced`):
  - **`DC-EPOCH-02`** (`9b15378`): `partial` → `enforced` via
    `ci/ci_check_hfc_translation.sh`.
  - **`T-REC-01`**, **`T-REC-02`**, **`DC-STORE-01`**, **`DC-STORE-02`**,
    **`DC-STORE-03`**, **`DC-STORE-05`**, **`CN-STORE-03`**,
    **`CN-STORE-04`**, **`CN-STORE-05`** (all `78da6c9`):
    `declared` → `enforced` via the three new N-D CI scripts.
  - **`T-ENC-03`**, **`CN-WIRE-07`**, **`DC-PROTO-02`**,
    **`DC-PROTO-05`** (PHASE4-N-A, `d977640` registry wiring plus
    S-A9 real-capture commits): each gained real-capture corpus
    tests (`crates/ade_network/tests/*_real_capture_corpus.rs`
    suites and the malformed-frame negative test), and
    `ci_check_ce_n_a_5_proof.sh` is the authoritative gate. Status
    movement will be reflected in the next TRACEABILITY refresh.
- Annotated but unchanged:
  - **`DC-STORE-04`** (`78da6c9`): 12-line Tier-5-divergence comment
    block appended above the rule entry; rule body unchanged.
  - 13 further rules had their TRACEABILITY rows annotated by
    `c8fa37f` to remove `_(scope gap)_` markers — TRACEABILITY-doc
    edits, not registry edits.

Net strengthening this delta: **14 rules** moved
`declared` / `partial` → `enforced` (1 from `9b15378`, 9 from
`78da6c9`, 4 from the PHASE4-N-A real-capture wiring). All are
permitted strengthenings, not weakenings. No rule IDs were retired
or reassigned. Family counts at HEAD: T=30, DC=39 (+2 for
DC-CORE-01 and DC-PROTO-06), CN=64, RO=6, OP=7, remaining placeholders
unchanged.

Normative-doc rule extraction (the `normative_docs` list in
`.idd-config.json`) is approximate and not regenerated here — the
structured registry is the authoritative source.

---

## Anomalies and Cross-Reference Warnings

- **PHASE4-N-A closed at HEAD.** CE-N-A-5 closure evidence is logged
  to `docs/active/CE-N-A-5_evidence.toml` by `ci_check_ce_n_a_5_proof.sh`
  (`56bfa7b`). The cluster doc was archived to
  `docs/clusters/completed/PHASE4-N-A/` by the closure commit
  (`69a2862`). CODEMAP does not yet contain `ade_network`
  per-submodule entries — flagged for the next `/codemap` run.
- **PHASE4-N-D closed.** Cluster N-D archived at
  `docs/clusters/completed/PHASE4-N-D/` (`436b1d7`); CE-N-D-1
  evidence at `CE-N-D-1_2026-05-19.log` (1000/1000 stress-kill
  iterations green).
- **CODEMAP stale on CI-script count and ade_network surface.** CODEMAP
  needs to record 21 CI scripts (was 19 after `78da6c9`, now +2 for
  S-A1 and S-A10) and the `ade_network` per-submodule TCB color map.
  Flagged for the next `/codemap` run.
- **TRACEABILITY stale on PHASE4-N-A rule status flips.** Four
  protocol-family rules (T-ENC-03, CN-WIRE-07, DC-PROTO-02, DC-PROTO-05)
  gained real-capture tests but their `status` field has not yet been
  flipped in the registry; flagged for the next `/traceability` run.
  The two new rules DC-CORE-01 and DC-PROTO-06 also need TRACEABILITY
  rows.
- **`ade_core` is BLUE by config but empty.** Acknowledged in CODEMAP
  callout and TRACEABILITY; treated as a CE-79 Tier-4 non-goal. Not
  new in this delta.
- **`ade_node` MUST NOT list is forward-looking.** Binary is a
  hello-world stub; no authority surface exercised yet. Cluster N-E
  (ledger + runtime composition) will activate it. Not new in this
  delta.
- No removed canonical types (n/a — no separate registry).
- No removed registry rules (expected: 0; actual: 0).
- No commit subjects in the delta lack a conventional-commits prefix.

---

## Generation Notes

Regenerate via `/head-deltas <baseline>` or by re-running the
`head-deltas-generator` agent with the same baseline. Baseline lives
in `.idd-config.json` `head_deltas_baseline`. Update on next phase
boundary (Phase 4 close, or when the next cluster — N-B / N-E /
N-F — closes).
