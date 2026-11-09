# Invariant Slice — PHASE4-N-F-G-D S3: C1 dry-run runbook + operator-gated execution scaffold

> **Status:** Planning Artifact (Non-Normative). Normative authority is the registry + CI.

## 2. Slice Header

### Slice Name
C1 dry-run runbook + operator-gated execution scaffold (a strict subset of the G-C preprod operator-pass runbook + the env-gated `ADE_LIVE_C1_DRY_RUN` execution harness, blocked without the C1 net).

### Cluster
**PHASE4-N-F-G-D** — Private-testnet accepted-block bounty dry-run.

### Status
Merged (PHASE4-N-F-G-D close — impl `076a5af5`; CE-G-D-3 green). Third slice — the operator-gated half.

### Cluster Exit Criteria Addressed
- [ ] **CE-G-D-3 (operator-gated C1 dry-run — SCAFFOLDS ONLY; live execution BLOCKED)** — the C1 dry-run runbook is committed as a **provable strict subset** of the preprod operator-pass runbook (`docs/evidence/phase4-n-f-g-c-operator-pass-README.md`), differing only in venue (operator-authored private genesis stake → fast slots; follower peer, one-producer-per-key) and the rehearsal label; the rehearsal-evidence I/O is wired end-to-end and exercised on a hermetic fixture; **no synthetic manifest committed**; the actual C1 live execution is `blocked_until_operator_c1_net_executed` (named, not deferred).

### Slice Dependencies
- PHASE4-N-F-G-D S1 (path-fidelity proof + fence) — merged (`d4d0f456`). The fence is what makes the runbook a *provable* strict subset (no private-only flag/branch/constructor can exist).
- PHASE4-N-F-G-D S2 (rehearsal-evidence surface + gate) — merged (`459cf78d`). Provides `rehearsal_pass::{correlate_peer_log_file_into_rehearsal, write_private_rehearsal_manifest}` + `ci_check_rehearsal_manifest_schema.sh`.
- PHASE4-N-F-G-C (`docs/evidence/phase4-n-f-g-c-operator-pass-README.md`; the `ADE_LIVE_OPERATOR_TEST` env-gated pattern in `node_operator_pass_ba02.rs`) — merged.

## 3. Implementation Instruction (AI)
Commit `docs/evidence/phase4-n-f-g-d-private-rehearsal-README.md` — a strict subset/adaptation of the G-C preprod operator-pass runbook. It MUST reuse, verbatim, the same `--mode node` path, the same `--peer` live feed, the same `--json-seed` / `--consensus-inputs-path`, the same operator key/opcert flow, the same Haskell peer-log capture, the same `correlate` path, and the same `NoEvidence` fail-closed behavior. ONLY these differ: `venue = private-testnet-c1`; operator-authored genesis/stake for fast slots; the manifest envelope is `PrivateRehearsalManifest` (`is_rehearsal`/`not_bounty_evidence = true`) under the rehearsal home. Add a test file `crates/ade_node/tests/node_c1_dry_run_rehearsal.rs` with: a **hermetic** test `c1_dry_run_correlate_to_rehearsal_envelope` (matching peer-log fixture in a `TempDir` → `correlate_peer_log_file_into_rehearsal` → `Some(manifest)` → `write_private_rehearsal_manifest` → read back, assert the rehearsal markers + the correlate-produced payload; plus the `NoEvidence` → `None` → nothing-written path), and an **env-gated** `node_c1_dry_run_rehearsal_live` (gated by `ADE_LIVE_C1_DRY_RUN=1`, mirroring `node_operator_pass_ba02_live`; `NoEvidence` PANICS, never writes). **No** binary wiring, **no** new `--mode node` flag, **no** new bootstrap/constructor, **no** alternate correlator, **no** new CI gate, **no** synthetic manifest committed, **no** RO-LIVE flip, **no** BLUE change. Commit carries the project attribution trailer (CLAUDE.md).

## 4. Intent
Give the operator a runbook + a blocked execution harness to run the C1 dry-run **on the exact preprod accepted-block path** (S1) producing **non-promotable** evidence (S2): same `--mode node` → `--peer` feed → forge → self-accept → sibling-serve → block-fetch → Haskell peer log → `correlate` → a `PrivateRehearsalManifest`. The live run is `blocked_until_operator_c1_net_executed` and flips no RO-LIVE rule. (Completes the mechanical coverage of `CN-REHEARSAL-FIDELITY-01`; preserves the S1 fence, the S2 gate, the bounty BA-02 gate, the containment/handoff/memory fences, and RO-LIVE-01/06 unchanged.)

## 5. Scope
- **Modules / crates / docs:**
  - `docs/evidence/phase4-n-f-g-d-private-rehearsal-README.md` (NEW runbook) — the C1 dry-run procedure; a strict subset/adaptation of the G-C preprod runbook.
  - `crates/ade_node/tests/node_c1_dry_run_rehearsal.rs` (NEW test file, RED) — `c1_dry_run_correlate_to_rehearsal_envelope` (hermetic) + `node_c1_dry_run_rehearsal_live` (env-gated `ADE_LIVE_C1_DRY_RUN`).
- **State machines affected:** none (runbook + tests over the reused S1/S2/G-C path).
- **Persistence impact:** none. No rehearsal manifest is committed by this slice — the rehearsal gate stays vacuous until a real operator run.
- **Network-visible impact:** none in CI (the live test is skipped without the env).
- **Out of scope:** the actual C1 live execution (operator-gated); the bounty C2 preprod pass; any binary `--mode node` wiring or flag; any new CI gate; any change to `rehearsal_evidence`/`rehearsal_pass`/`ba02_*`/the gates.

## 6. Execution Boundary
- **BLUE (none — unchanged):** no BLUE crate touched. A BLUE change → reject.
- **GREEN (reused, unchanged):** `ade_node::rehearsal_evidence` (S2), `ade_node::ba02_evidence::correlate`.
- **RED:** `crates/ade_node/tests/node_c1_dry_run_rehearsal.rs` (the env-gated operator execution harness, mirroring `node_operator_pass_ba02.rs` — a RED test, skipped in CI, not a runtime node mode); the runbook is operational doc (release/evidence scope, not runtime authority).
- **Color resolved:** no ambiguity — S3 is a RED test + a doc over the reused S1/S2 surfaces; no BLUE/GREEN change.

## 7. Invariants Preserved
- `RO-LIVE-01` / `RO-LIVE-06` — **not** flipped; S3 wires the operator-gated dry-run harness only (the live run is `blocked_until_operator_c1_net_executed`).
- `CN-REHEARSAL-FIDELITY-01` clause (1) + (2) — the S1 fence (`ci_check_node_path_fidelity.sh`) + the S2 gate (`ci_check_rehearsal_manifest_schema.sh`) are **byte-unchanged**; the runbook uses the same flags (S1 guarantees no path divergence) and the same `PrivateRehearsalManifest`/correlate path (S2).
- `CN-OPERATOR-EVIDENCE-01` / bounty BA-02 gate (`ci_check_ba02_evidence_manifest_schema.sh`) — **byte-unchanged**; the rehearsal gate stays vacuous (no synthetic manifest committed).
- `DC-NODE-06` / `CN-NODE-02` / `DC-LIVEMEM-01` — containment / handoff / memory fences byte-unchanged.

## 8. Invariants Strengthened or Introduced
- **`CN-REHEARSAL-FIDELITY-01` — completes mechanical coverage (strengthened).** S3 records its runbook + tests in the rule's `evidence_notes`. With S1 (path fidelity) + S2 (evidence non-promotability) + S3 (operator-gated scaffold) all green, the rule is mechanically complete; the `tests`/`ci_script` array binding + the `declared → enforced` flip happen at **`/cluster-close PHASE4-N-F-G-D`**. Status stays `declared` until then.

> Single invariant family: the operator-gated execution scaffold for the non-promotable, path-faithful C1 dry-run. S3 adds no new invariant — it completes the runbook + harness half of `CN-REHEARSAL-FIDELITY-01`.

## 9. Design Summary
- **Runbook** (`phase4-n-f-g-d-private-rehearsal-README.md`) mirrors the G-C runbook section-for-section: a hard-line header (Ade self-accept ≠ peer acceptance; C1 acceptance ≠ bounty completion; no RO-LIVE flip), §0 venue (C1 only — operator genesis stake → fast slots), §1 pre-flight genesis-consistency pin (`genesis_pinning`), §2 launch (the **same** `--mode node --peer --json-seed --consensus-inputs-path --cold-skey --kes-skey --vrf-skey --opcert --genesis-file --wal-dir --snapshot-dir --network-magic --genesis-hash` flags — no private-only flag), §3 peer-as-follower (no forging creds; one-producer-per-key), §4 capture peer log → `correlate_peer_log_file_into_rehearsal` (env-gated `ADE_LIVE_C1_DRY_RUN`; `sha256sum` the committed peer log for the envelope) → `write_private_rehearsal_manifest` under the rehearsal home, §5 the rehearsal gate (`ci_check_rehearsal_manifest_schema.sh`) + the explicit delta-from-G-C list, §6 what this does NOT do (no RO-LIVE flip; not bounty completion; the bounty deliverable is the separate C2 preprod pass).
- **Hermetic test** `c1_dry_run_correlate_to_rehearsal_envelope`: end-to-end file → manifest wiring in a `TempDir` (matching log → `Some(manifest)` → write → read back → assert markers + payload; non-matching → `None` → nothing written).
- **Env-gated execution harness** `node_c1_dry_run_rehearsal_live` (`ADE_LIVE_C1_DRY_RUN=1`): the env-gated test is the operator execution harness for this rehearsal evidence path; it remains skipped in CI and is not a runtime node mode. When enabled it reads `ADE_LIVE_FORGED_BLOCK_HASH/SLOT`, `ADE_LIVE_NETWORK_MAGIC`, `ADE_LIVE_PEER_LOG`, `ADE_LIVE_REHEARSAL_PEER_LOG_SHA256`, `ADE_LIVE_REHEARSAL_MANIFEST_OUT` → `correlate_peer_log_file_into_rehearsal` → `write_private_rehearsal_manifest`; `NoEvidence` PANICS (the peer did not accept — no manifest).

## 10. Changes Introduced
### Types
- None (reuses S2's `PrivateRehearsalManifest`/`RehearsalEnvelope`). No new canonical type, no `Mode`, no CLI field.
### State Transitions
- None.
### Persistence
- None (no manifest committed; the gate stays vacuous).
### Removal / Refactors
- None (additive: one doc + one test file).

## 11. Replay, Crash, and Epoch Validation
- **Replay:** the hermetic test rides `correlate`'s determinism (R1 carried — same fixture → byte-identical manifest). No new authoritative state.
- **Crash/restart / Epoch:** n/a (operator harness + doc).

## 12. Mechanical Acceptance Criteria
This slice is complete only when **all** of the following exist and pass in CI (hermetic):

- [ ] `c1_dry_run_correlate_to_rehearsal_envelope` (`ade_node`, `node_c1_dry_run_rehearsal.rs`) — a matching peer-log fixture flows file → `correlate_peer_log_file_into_rehearsal` → `Some(PrivateRehearsalManifest)` → `write_private_rehearsal_manifest` → the written TOML carries `is_rehearsal = true` / `not_bounty_evidence = true` / `venue = "private-testnet-c1"` + the correlate-produced payload; a non-matching fixture → `None` → nothing written.
- [ ] `node_c1_dry_run_rehearsal_live` (`ade_node`) — compiles and **skips** when `ADE_LIVE_C1_DRY_RUN` is unset (the CI default), printing the required-env hint; never writes a manifest in CI.
- [ ] `docs/evidence/phase4-n-f-g-d-private-rehearsal-README.md` is committed (a strict subset of the G-C runbook; explicit delta list; no private-only flag instructed).
- [ ] `ci_check_rehearsal_manifest_schema.sh` — **green + vacuous** (no synthetic manifest committed by this slice).
- [ ] `ci_check_node_path_fidelity.sh` (S1) + `ci_check_ba02_evidence_manifest_schema.sh` (bounty) + `ci_check_node_run_loop_containment.sh` + `ci_check_served_chain_handoff_fence.sh` + `ci_check_live_feed_memory_bounds.sh` — **byte-unchanged + green**.
- [ ] `cargo test -p ade_node` green (no regression).

## 13. Failure Modes
- The live harness's `correlate` → `NoEvidence` → **panic** (the peer did not accept; no manifest written) — mirrors `node_operator_pass_ba02_live`.
- A missing/unreadable peer log in the live harness → `io::Error` (fail closed), never a synthesized manifest.
- A runbook step that would instruct a private-only path → impossible to execute: the binary rejects an unknown flag, and the S1 fence guarantees no from-genesis constructor exists.

## 14. Hard Prohibitions
### Inherited Cluster-Level Prohibitions
All PHASE4-N-F-G-D "Forbidden During This Cluster" prohibitions apply (no private-only shortcut; no containment/handoff/memory relaxation; no synthetic manifest; no RO-LIVE flip; no new BLUE authority/canonical type/`--mode node` flag/from-genesis constructor).
### Slice-Specific Prohibitions
- **No committed live result** — no `phase4-n-f-g-d-private-rehearsal-*.toml` manifest is committed by S3; the gate stays vacuous until a real operator run.
- **No RO-LIVE-01 / RO-LIVE-06 flip.**
- **The runbook must not instruct a private-only path** — same flags as preprod; only venue + stake + label + manifest envelope differ.
- **No new `--mode node` flag / bootstrap / constructor / correlator / CI gate / binary wiring.**
- **No BLUE change; no change to `rehearsal_*`/`ba02_*`/the gates.**

## 15. Explicit Non-Goals
This slice MUST NOT: execute the C1 live nodes (operator-gated, `blocked_until_operator_c1_net_executed`); perform/claim the bounty C2 preprod pass; wire the rehearsal manifest into any binary `--mode node` arm; add a CLI flag, a new gate, or a from-genesis constructor; modify any BLUE crate; flip RO-LIVE; commit any manifest/peer-log fixture.

## 16. Completion Checklist
- [ ] Runbook committed — a strict subset of the G-C preprod runbook; only venue/stake/label/envelope differ; no private-only flag.
- [ ] Hermetic `c1_dry_run_correlate_to_rehearsal_envelope` green (file → manifest wiring + `NoEvidence` → nothing).
- [ ] Env-gated `node_c1_dry_run_rehearsal_live` compiles + skips without `ADE_LIVE_C1_DRY_RUN`.
- [ ] Rehearsal gate vacuous + green; S1 fence + bounty gate + 3 containment/memory fences byte-unchanged.
- [ ] No BLUE change; `cargo test -p ade_node` green.
- [ ] `CN-REHEARSAL-FIDELITY-01` `evidence_notes` record S3 (stays `declared`; bind + flip at `/cluster-close`).

## 17. Review Notes
- **"Strict subset" is anchored, not asserted:** the S1 path-fidelity fence mechanically guarantees there is no private-only flag/branch/from-genesis constructor, so the runbook *cannot* instruct a path the preprod pass doesn't share — the binary would reject it. The runbook documents the same flags + an explicit delta-from-G-C list; S1 is the proof.
- **The env-gated test is the operator execution harness, not a node mode:** the env-gated test is the operator execution harness for this rehearsal evidence path; it remains skipped in CI and is not a runtime node mode (no binary arm produces a manifest), mirroring `node_operator_pass_ba02_live`. The live run is `blocked_until_operator_c1_net_executed`.
- **After S3:** `/cluster-close PHASE4-N-F-G-D` runs the IDD + per-cluster security review on the full G-D diff, flips `CN-REHEARSAL-FIDELITY-01` `declared → enforced` (binds `tests`/`ci_script`), and regenerates the four grounding docs. **No RO-LIVE flip** — the bounty deliverable remains the separate operator-witnessed C2 preprod pass.
