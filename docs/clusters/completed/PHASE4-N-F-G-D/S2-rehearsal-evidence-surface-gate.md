# Invariant Slice — PHASE4-N-F-G-D S2: Rehearsal-evidence surface + gate

> **Status:** Planning Artifact (Non-Normative). Normative authority is the registry + CI.

## 2. Slice Header

### Slice Name
Rehearsal-evidence surface + gate (`PrivateRehearsalManifest` = a distinct, non-promotable envelope wrapping a correlate-produced `Ba02Manifest` payload, + `ci_check_rehearsal_manifest_schema.sh`).

### Cluster
**PHASE4-N-F-G-D** — Private-testnet accepted-block bounty dry-run.

### Status
Merged (PHASE4-N-F-G-D close — impl `459cf78d`; CE-G-D-2 green). Second slice — the evidence-non-promotability half.

### Cluster Exit Criteria Addressed
- [ ] **CE-G-D-2 (rehearsal-evidence non-promotability — MECHANICAL, closeable)** — a clearly-marked rehearsal manifest that **wraps the same correlate-produced `Ba02Manifest` payload but carries a distinct rehearsal envelope** (`venue` / `is_rehearsal` / `not_bounty_evidence`) lives **only** under `docs/evidence/phase4-n-f-g-d-private-rehearsal-*.toml`, is **correlate-produced** and **sha256-bound**; a new vacuous-until-committed gate forbids it under the bounty home (and a bounty manifest under the rehearsal home) and cross-checks that `ci_check_ba02_evidence_manifest_schema.sh` does not match the rehearsal home; a hermetic fixture proves correlate-produced + fail-closed (`NoEvidence` → write nothing). Flips **no** RO-LIVE rule.

### Slice Dependencies
- PHASE4-N-F-G-D S1 (path-fidelity proof + fence) — merged (`d4d0f456`).
- PHASE4-N-F-C / G-C (`RO-LIVE-06`: `ba02_evidence::correlate` is the sole `Ba02Manifest` ctor; `ba02_pass` I/O model) — merged.

## 3. Implementation Instruction (AI)
Add a **GREEN** type `ade_node::rehearsal_evidence::PrivateRehearsalManifest` that **wraps** a correlate-produced `Ba02Manifest` payload plus a distinct rehearsal envelope (a closed `RehearsalVenue` enum → `"private-testnet-c1"`, the bound peer-log filename, and its sha256). Its **sole** constructor takes a `BA02Outcome` and yields `Some` only for the `Ba02Manifest` arm — `NoEvidence` → `None` (writes nothing). `to_canonical_toml()` ALWAYS emits `is_rehearsal = true` + `not_bounty_evidence = true` as literals (the type cannot represent a non-rehearsal). Add a **RED** module `ade_node::rehearsal_pass` (mirrors `ba02_pass`): `correlate_peer_log_file_into_rehearsal(...) -> io::Result<Option<PrivateRehearsalManifest>>` (reuses `ba02_pass::correlate_peer_log_file`; missing file = `io::Error`) + `write_private_rehearsal_manifest(&PrivateRehearsalManifest, out_path)` (accepts only the manifest — the type is the gate). Add `ci/ci_check_rehearsal_manifest_schema.sh` (vacuous-until-committed). **No alternate correlator** (reuse `correlate` verbatim); **no synthetic manifest**; **no RO-LIVE flip**; **no** write under the bounty home / `CE-G-C-LIVE_*` glob; **no** BLUE change; do not touch the containment/handoff/memory fences or the S1 path-fidelity fence. Commit carries the project attribution trailer (CLAUDE.md).

## 4. Intent
Make a private-testnet rehearsal manifest **structurally impossible to mistake for, or promote into, bounty BA-02 evidence**: it wraps the *same* correlate-produced proof payload but carries a distinct rehearsal envelope, lives only in the rehearsal home, is sha256-bound to a real Haskell peer log, and flips no RO-LIVE rule. (Begins enforcing clause (2) — *evidence non-promotability* — of `CN-REHEARSAL-FIDELITY-01`; preserves `RO-LIVE-06`'s sole-correlate-ctor + allow-list and `CN-OPERATOR-EVIDENCE-01`'s bounty-manifest schema unchanged.)

## 5. Scope
- **Modules / crates:**
  - `ade_node::rehearsal_evidence` (NEW, GREEN) — `PrivateRehearsalManifest`, closed `RehearsalVenue`, `REHEARSAL_MANIFEST_SCHEMA_VERSION: u32 = 1`, `from_correlate_outcome(&BA02Outcome, RehearsalEnvelope) -> Option<Self>` (the sole ctor; `None` on `NoEvidence`), `to_canonical_toml()` (deterministic; `is_rehearsal`/`not_bounty_evidence` literal `true`).
  - `ade_node::rehearsal_pass` (NEW, RED) — `correlate_peer_log_file_into_rehearsal` + `write_private_rehearsal_manifest`; registered `pub mod` in `lib.rs`.
  - `ci/ci_check_rehearsal_manifest_schema.sh` (NEW gate, RED).
- **State machines affected:** none (evidence surface; `correlate` reused unchanged).
- **Persistence impact:** none in the node path. The manifest is an evidence artifact written only by the operator-gated S3 run (the gate is vacuous until then).
- **Network-visible impact:** none.
- **Out of scope:** the C1 dry-run runbook + operator execution (S3); any change to `ba02_evidence` / `ba02_pass` / the bounty gate; any serve/forge/containment/memory change; any RO-LIVE flip.

## 6. Execution Boundary
- **BLUE (none — unchanged):** no BLUE crate touched. A BLUE change → reject.
- **GREEN:** `ade_node::rehearsal_evidence` (pure: serde + a wrapper over the GREEN `Ba02Manifest`; deterministic `to_canonical_toml`; no I/O/clock/rand/float); `ade_node::ba02_evidence::correlate` (reused, unchanged — the sole payload producer).
- **RED:** `ade_node::rehearsal_pass` (file I/O, mirrors `ba02_pass`); `ci/ci_check_rehearsal_manifest_schema.sh`.
- **Color resolved:** the open color from the cluster doc is resolved — `PrivateRehearsalManifest` is a **pure GREEN** type (it only wraps + serializes); the I/O is RED. No BLUE.

## 7. Invariants Preserved
- `RO-LIVE-06` — `correlate` stays the **sole** `Ba02Manifest` constructor + the allow-list parser; S2 **reuses** it verbatim (no alternate correlator), and does **not** flip it (still schema + mechanics only).
- `CN-OPERATOR-EVIDENCE-01` — the bounty operator-evidence manifest schema + `ci_check_ba02_evidence_manifest_schema.sh` are **unchanged**; the rehearsal gate is a distinct, separate gate over a distinct home.
- `RO-LIVE-01` — **not** flipped (stays `partial`).
- `CN-REHEARSAL-FIDELITY-01` clause (1) — the S1 path-fidelity fence (`ci_check_node_path_fidelity.sh`) is **byte-unchanged**.
- `DC-NODE-06` / `CN-NODE-02` / `DC-LIVEMEM-01` — containment / handoff / memory fences byte-unchanged.

## 8. Invariants Strengthened or Introduced
- **`CN-REHEARSAL-FIDELITY-01` — clause (2) evidence non-promotability (strengthened: enforcement begun).** S2 records its gate (`ci/ci_check_rehearsal_manifest_schema.sh`) + tests in the rule's `evidence_notes` now; the final `tests`/`ci_script` binding + the `declared → enforced` flip happen at G-D close (clause 2 is mechanically green in CI from S2 onward). Status stays `declared`.

> Single invariant family: "a private-testnet rehearsal manifest is non-promotable to bounty evidence." S2 covers exactly the evidence-surface half; the runbook + operator execution is S3.

## 9. Design Summary
- **`PrivateRehearsalManifest` (GREEN)** wraps `ba02: Ba02Manifest` (the correlate-produced payload, verbatim) + envelope `{ venue: RehearsalVenue, peer_log_file: String, peer_log_file_sha256: String }`. Sole ctor `from_correlate_outcome(&BA02Outcome, env) -> Option<Self>`: `Some(wrap(m))` for `Ba02Manifest(m)`, `None` for `NoEvidence`. `to_canonical_toml()` emits a flat TOML: `schema_version`, `venue = "private-testnet-c1"`, `is_rehearsal = true`, `not_bounty_evidence = true`, `peer_log_file`, `peer_log_file_sha256`, then the wrapped payload (`forged_block_hash_hex`, `slot`, `network_magic`, `peer_accept_source`, `peer`, `matched_block_hash_hex`). The markers are **literals** — the type cannot represent a non-rehearsal.
- **`rehearsal_pass` (RED)** mirrors `ba02_pass`: `correlate_peer_log_file_into_rehearsal(ade, peer_log_path, env)` = `ba02_pass::correlate_peer_log_file(ade, peer_log_path)?` → `PrivateRehearsalManifest::from_correlate_outcome(&outcome, env)` (the `None` path is "NoEvidence writes nothing"). `write_private_rehearsal_manifest(&m, out)` writes `m.to_canonical_toml()` — accepts **only** a manifest.
- **`ci_check_rehearsal_manifest_schema.sh`** globs `docs/evidence/phase4-n-f-g-d-private-rehearsal-*.toml`; vacuous if none. When present: required fields (`schema_version` == 1, `venue`, `is_rehearsal`, `not_bounty_evidence`, `peer_log_file`, `peer_log_file_sha256`, `forged_block_hash_hex`, …); `venue` contains `private-testnet`; `is_rehearsal == true`; `not_bounty_evidence == true`; `peer_log_file_sha256` matches the committed peer log's sha256. **Non-promotability cross-checks:** (i) the rehearsal manifest is under `docs/evidence/`, never the bounty home `docs/clusters/PHASE4-N-F-G-C/`; (ii) no file under the bounty home carries `is_rehearsal`/`not_bounty_evidence` markers; (iii) the bounty gate's `CE-G-C-LIVE_*` glob does not match the rehearsal home (asserted by construction — distinct dir + prefix).

## 10. Changes Introduced
### Types
- New GREEN: `PrivateRehearsalManifest`, closed `RehearsalVenue` (`PrivateTestnetC1`), `RehearsalEnvelope`, `REHEARSAL_MANIFEST_SCHEMA_VERSION`. No new canonical (BLUE) type, no new `Mode`, no CLI field, no new `Ba02Manifest`/`BA02Outcome` variant.
### State Transitions
- None (evidence wrapper over the reused `correlate`).
### Persistence
- None in the node path (manifest written only by the operator-gated S3 run).
### Removal / Refactors
- None (additive; `ba02_evidence`/`ba02_pass` untouched).

## 11. Replay, Crash, and Epoch Validation
- **Replay:** `to_canonical_toml()` is deterministic; the wrapped payload is `correlate`'s output (R1 carried — same inputs → byte-identical manifest). No new authoritative state.
- **Crash/restart:** n/a (no durable node state; the manifest is an evidence file).
- **Epoch boundary:** n/a.

## 12. Mechanical Acceptance Criteria
This slice is complete only when **all** of the following exist and pass in CI (hermetic):

- [ ] `rehearsal_envelope_wraps_correlate_produced_payload` (`ade_node`) — a matching peer log → `correlate` → `Ba02Manifest` → wrapped; the manifest's payload equals `correlate`'s `Ba02Manifest` byte-for-byte, and `to_canonical_toml()` carries `is_rehearsal = true` + `not_bounty_evidence = true` + `venue = "private-testnet-c1"` + the sha256 binding.
- [ ] `rehearsal_correlate_no_evidence_writes_nothing` (`ade_node`) — a non-matching peer log → `correlate` → `NoEvidence` → `from_correlate_outcome` returns `None`; no manifest is produced and `write_private_rehearsal_manifest` is never reached (the out path does not exist after).
- [ ] `rehearsal_envelope_is_structurally_distinct_from_ba02_manifest` (`ade_node`) — the rehearsal TOML carries the rehearsal markers a bare `Ba02Manifest` lacks, and does not satisfy the bounty schema's required-field set (including peer-log capture/filter fields required by the bounty gate) — so a rehearsal manifest cannot satisfy the bounty gate.
- [ ] `ci_check_rehearsal_manifest_schema.sh` — NEW gate, green: vacuous-until-committed; smoke-tested fail-closed on (a) a committed rehearsal manifest with a wrong `peer_log_file_sha256`, (b) one missing `not_bounty_evidence`, and (c) a rehearsal-marked file placed under the bounty home.
- [ ] `ci_check_ba02_evidence_manifest_schema.sh` — **byte-unchanged + green** (verified `git diff` vs `main`); confirmed its glob does not match the rehearsal home.
- [ ] `ci_check_node_path_fidelity.sh` (S1) + `ci_check_node_run_loop_containment.sh` + `ci_check_served_chain_handoff_fence.sh` + `ci_check_live_feed_memory_bounds.sh` — **byte-unchanged + green**.
- [ ] `cargo test -p ade_node` green (no regression).

## 13. Failure Modes
- `correlate` → `NoEvidence` → `from_correlate_outcome` → `None` → **no manifest written** (the type-level gate; "NoEvidence writes nothing").
- A hand-authored rehearsal manifest with no matching peer-log sha256, or missing a rehearsal marker, or placed under the bounty home → `ci_check_rehearsal_manifest_schema.sh` **fails closed**.
- A missing/unreadable peer-log file → `io::Error` (fail closed), never a synthesized manifest (inherited from `ba02_pass::correlate_peer_log_file`).

## 14. Hard Prohibitions
### Inherited Cluster-Level Prohibitions
All PHASE4-N-F-G-D "Forbidden During This Cluster" prohibitions apply (no private-only shortcut; no containment/handoff/memory relaxation; no synthetic manifest; no RO-LIVE flip; no new BLUE authority/canonical type/`--mode node` flag/from-genesis constructor).
### Slice-Specific Prohibitions
- **No alternate correlator** — `ba02_evidence::correlate` stays the sole acceptance-evidence producer; the rehearsal type only *wraps* its output.
- **No synthetic manifest** — a manifest exists only via `correlate` → `Ba02Manifest` → wrap; `NoEvidence` writes nothing.
- **No RO-LIVE flip** — `RO-LIVE-01`/`RO-LIVE-06` unchanged; the rehearsal manifest is `not_bounty_evidence = true`.
- **No bounty-home write / `CE-G-C-LIVE_*` glob** — the rehearsal manifest lives only under `docs/evidence/phase4-n-f-g-d-private-rehearsal-*.toml`; the gate forbids rehearsal markers under the bounty home.
- **No acceptance from Ade self-accept / served bytes / wire success** — only the Haskell peer log through `correlate` (inherited allow-list).
- **No BLUE change; no change to `ba02_evidence`/`ba02_pass`/the bounty gate/the S1 fence.**

## 15. Explicit Non-Goals
This slice MUST NOT: write the C1 dry-run runbook or wire operator execution (S3); modify `ba02_evidence`/`ba02_pass`/`ci_check_ba02_evidence_manifest_schema.sh`; add a CLI flag or a from-genesis constructor (S1 fence forbids); modify any BLUE crate; relax any fence; claim live evidence / BA-02 / bounty completion; flip RO-LIVE.

## 16. Completion Checklist
- [ ] `PrivateRehearsalManifest` wraps a correlate-produced `Ba02Manifest`; sole ctor returns `None` on `NoEvidence`; markers are literal `true`.
- [ ] `rehearsal_pass` I/O mirrors `ba02_pass` (write accepts only a manifest); missing peer log → `io::Error`.
- [ ] `ci_check_rehearsal_manifest_schema.sh` green + fail-closed-smoke-verified (wrong sha256 / missing marker / bounty-home leak).
- [ ] BA-02 bounty gate + S1 fence + 3 containment/memory fences byte-unchanged + green.
- [ ] No BLUE change; `cargo test -p ade_node` green.
- [ ] `CN-REHEARSAL-FIDELITY-01` `evidence_notes` record S2 (stays `declared`; bind + flip at close).

## 17. Review Notes
- **Non-promotability is structural, not conventional:** the bounty gate globs only `docs/clusters/PHASE4-N-F-G-C/CE-G-C-LIVE_*.toml` — it physically cannot see `docs/evidence/phase4-n-f-g-d-private-rehearsal-*.toml`; the rehearsal manifest's `is_rehearsal`/`not_bounty_evidence` literals + its non-satisfaction of the bounty schema mean it would *fail* the bounty gate even if copied there; and the rehearsal gate asserts no rehearsal marker under the bounty home. Three independent barriers.
- **Reuse, not reimplementation:** `correlate` is the sole payload producer (RO-LIVE-06 unchanged); the rehearsal layer is a pure wrapper + a distinct home + a distinct gate. The "no synthetic manifest" guarantee is inherited verbatim.
- **Why GREEN type + RED I/O:** mirrors the `ba02_evidence`(GREEN) / `ba02_pass`(RED) split exactly — the type is a pure serializable wrapper; only the file I/O is RED.
