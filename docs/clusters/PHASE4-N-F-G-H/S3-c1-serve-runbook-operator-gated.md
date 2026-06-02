# Invariant Slice — PHASE4-N-F-G-H S3: C1 dry-run runbook + operator-gated serve harness

> **Status:** Planning Artifact (Non-Normative). Normative authority is the registry + CI.

## 2. Slice Header

### Slice Name
C1 dry-run runbook + operator-gated serve harness (exercise a real Haskell follower fetching Ade's served forged block over the now-wired, magic-aware node-spine serve — evidence only via `correlate`, non-promotable C1, no RO-LIVE flip).

### Cluster
**PHASE4-N-F-G-H** — Node-spine live serve-to-peer. The **operator-gated half** (mirrors G-C/G-D): a mechanical scaffold (runbook + env-gated harness, closeable) + the actual C1 live execution (`blocked_until_operator_c1_serve_executed`).

### Status
Proposed (doc-before-implement). Depends on S2 (node serve sibling) + S2b (magic-aware serve listener — without it the C1 magic-42 follower's handshake would be refused).

### Cluster Exit Criteria Addressed
- [ ] **CE-G-H-3 (operator-gated C1 serve — SCAFFOLDS ONLY; live execution BLOCKED)** — the runbook `docs/evidence/phase4-n-f-g-h-node-serve-README.md` is committed (a strict adaptation of the G-C/G-D operator-pass runbooks); a candidate env-gated `node_c1_serve_live` (`ADE_LIVE_C1_SERVE`) is skipped/blocked without the C1 net; **no synthetic evidence committed**; live execution stays `blocked_until_operator_c1_serve_executed`.

### Slice Dependencies
- **S2** (`8c6a6a7e`): `run_node_serve_task` — `--mode node --listen` serves the self-accepted chain.
- **S2b** (`a8ca5e52`): the magic-aware serve listener — the C1 (magic 42) follower's handshake now succeeds.
- **G-D** (`6bd60c80`): `rehearsal_pass::{correlate_peer_log_file_into_rehearsal, write_private_rehearsal_manifest}` + `PrivateRehearsalManifest` + `RehearsalVenue::PrivateTestnetC1` — reused verbatim (C1 evidence is non-promotable).
- **G-C** (`351d46bc`): `ba02_evidence::correlate` — the sole acceptance-evidence correlator (allow-list includes `peer_served_block`, the serve-direction event); reused.

## 3. Implementation Instruction (AI)
Commit the serve runbook `docs/evidence/phase4-n-f-g-h-node-serve-README.md` as a **strict adaptation** of the G-C/G-D `--mode node` operator-pass runbooks — same recovered-seed forge + `--listen` + `--network-magic` + operator-key flow + peer-log capture + `correlate` + `NoEvidence` fail-closed; the **only** addition is the **downstream follower topology** (a real Haskell `cardano-node` whose `topology.json` lists Ade's `--listen` address; it ChainSync-discovers + BlockFetches Ade's forged block) and the venue/env labels. Add the env-gated harness `node_c1_serve_live` (gated by `ADE_LIVE_C1_SERVE=1`, mirroring `node_c1_dry_run_rehearsal_live`): it reads the operator-captured **Haskell-follower** log, calls `correlate_peer_log_file_into_rehearsal` (the follower's `peer_served_block` event for the exact forged hash → a **non-promotable** `PrivateRehearsalManifest`), and writes it ONLY on a real match; `NoEvidence` panics (no manifest). Skipped in CI; NOT a runtime node mode. Reuse the existing hermetic correlate test (`c1_dry_run_correlate_to_rehearsal_envelope`) — add no second hermetic correlate test. **No handshake-semantics change; no proactive `advance_tip`; no `served_chain_view.changed()` reactor; no `--mode produce` switch; no private-only path; no RO-LIVE flip; no bounty/preprod completion claim; no BLUE change.** Commit carries the project attribution trailer.

## 4. Intent
Exercise (operator-gated) the next live fact on the now-wired, magic-aware node-spine serve: a **real Haskell follower** connects to Ade's `--listen`, ChainSync-discovers Ade's served forged block, and BlockFetches its body. The follower is **expected to validate/accept it if the served block is protocol-valid**, but the **only accepted/served claim may come from the follower log through `correlate`** — the harness does not itself prove acceptance. C1 acceptance is a **non-promotable rehearsal** (private net, magic 42), never the bounty deliverable; flips no RO-LIVE rule.

## 5. Scope
- **Modules / crates:**
  - **NEW** `docs/evidence/phase4-n-f-g-h-node-serve-README.md` (RED — runbook) — the serve-direction operator procedure.
  - **NEW** `crates/ade_node/tests/node_c1_serve_live.rs` (RED — env-gated harness) — `node_c1_serve_live` (`ADE_LIVE_C1_SERVE`), reusing `rehearsal_pass`/`correlate`.
- **State machines affected:** none. The serve mechanism (S2/S2b) + correlate (G-C) + rehearsal_pass (G-D) are reused unchanged.
- **Persistence / network-visible impact:** none new (the operator runs the live nodes per the runbook; the harness only correlates the captured log).
- **Out of scope:** the C2 preprod bounty pass (BA-02 manifest, RO-LIVE-06); any proactive `advance_tip`; any handshake change; any BLUE change; any RO-LIVE flip.

## 6. Execution Boundary
- **BLUE (none — unchanged):** no BLUE crate touched. A BLUE change → reject.
- **GREEN:** `ade_node::ba02_evidence::correlate` + `ade_node::rehearsal_evidence::PrivateRehearsalManifest` (reused, the sole acceptance-evidence path).
- **RED:** the serve runbook; `node_c1_serve_live` (env-gated test reusing `rehearsal_pass`).
- **Color resolved:** no ambiguity — the harness is RED scaffolding over the reused GREEN correlate; no BLUE.

## 7. Invariants Preserved
- `DC-NODE-07` (S1/S2/S2b) — the serve mechanism + magic-aware listener + single serve-dispatch authority are reused unchanged; `ci_check_single_serve_dispatch_authority.sh` + `ci_check_serve_listener_magic_aware.sh` stay green.
- `CN-REHEARSAL-FIDELITY-01` (G-D) — the C1 serve dry-run uses the **same** `--mode node` accepted-block path as preprod (path fidelity); the manifest is non-promotable.
- `RO-LIVE-01` / `RO-LIVE-06` — **NOT flipped.** The live serve ACCEPT stays operator-gated; C1 ≠ bounty.
- `CN-NODE-02` containment, `ci_check_node_run_loop_containment.sh`, `ci_check_node_path_fidelity.sh`, `ci_check_served_chain_handoff_fence.sh` — byte-unchanged.
- The serve handshake semantics (S2b) — unchanged (no S3 handshake change).

## 8. Invariants Strengthened or Introduced
- **`DC-NODE-07` — operator-pass serve runbook + harness (strengthened; the operator-gated leg).** S3 records the serve-direction operator-pass runbook + the env-gated `node_c1_serve_live` harness in `evidence_notes`; the live serve ACCEPT stays operator-gated (no RO-LIVE flip). The final `tests`/`ci_script` binding + the `declared → enforced` flip happen at the G-H close — the mechanical scaffold (runbook committed + harness skips in CI + reused correlate green) is what closes; the C1 live execution is a separate operator-witnessed leg.

> Single invariant family: "the node-spine serve to a real peer is operator-exercisable on the same path, with acceptance proven only via correlate." S3 closes the scaffold; the live ACCEPT is gated.

## 9. Design Summary
- **Serve runbook** (strict adaptation of G-C/G-D): Ade runs `--mode node --listen <addr> --network-magic 42` + operator forge keys + the C1 recovered seed (the kept `~/.cardano-private-testnet-c1` net); a real Haskell `cardano-node` follower with `topology.json` listing Ade's `--listen` ChainSync-discovers + BlockFetches Ade's forged block; the operator captures the follower's JSONL log and runs `correlate`. Differs from the G-C/G-D runbooks **only** in the downstream-follower topology (the A7 serve direction) + the venue/env labels — the forge/seed/correlate/fail-closed steps are identical (path fidelity, `CN-REHEARSAL-FIDELITY-01`).
- **Env-gated harness** `node_c1_serve_live` (`ADE_LIVE_C1_SERVE=1`): reads the operator-captured follower log → `correlate_peer_log_file_into_rehearsal(&AdeForgeRecord, &log, RehearsalEnvelope{venue: PrivateTestnetC1, ..})` → on a real `peer_served_block` match for the exact forged hash, a **non-promotable** `PrivateRehearsalManifest`; `NoEvidence` panics (no manifest). Skipped in CI (env unset); not a node mode.
- **Reuse, no new evidence code:** `correlate` (G-C) + `rehearsal_pass`/`PrivateRehearsalManifest` (G-D) verbatim. The existing `c1_dry_run_correlate_to_rehearsal_envelope` hermetic test already proves the correlate→manifest wiring; S3 adds no second hermetic correlate test.
- **Stop-and-scope boundary:** if the real C1 follower **parks at tip before Ade forges** and requires a proactive `RollForward` (an `advance_tip` driver), **STOP and scope that as a separate cluster** — do not patch it into S3 (it is not wired in produce_mode either; request-driven serve only).

## 10. Changes Introduced
### Types
- None. No new canonical type, no `Mode`/flag/variant. Reuses `AdeForgeRecord`, `RehearsalEnvelope`, `RehearsalVenue::PrivateTestnetC1`, `PrivateRehearsalManifest`.
### State Transitions / Persistence
- None.
### Removal / Refactors
- None.

## 11. Replay, Crash, and Epoch Validation
- **Replay:** no new authoritative state. `correlate` is the existing deterministic correlator (`RO-LIVE-06` replay property, carried). The harness is env-gated (no CI replay).
- **Crash/epoch:** not applicable.

## 12. Mechanical Acceptance Criteria
This slice's **mechanical scaffold** is complete only when all of the following hold (the C1 live execution is a separate operator-gated leg, NOT a CI criterion):

- [ ] `docs/evidence/phase4-n-f-g-h-node-serve-README.md` is committed — a serve-direction operator-pass runbook, a strict adaptation of the G-C/G-D runbooks (only the downstream follower topology + venue/env labels differ).
- [ ] `node_c1_serve_live` (`crates/ade_node/tests/node_c1_serve_live.rs`, gated by `ADE_LIVE_C1_SERVE=1`) compiles and is **skipped** in CI (env unset); when enabled it correlates the operator-captured follower log → a non-promotable `PrivateRehearsalManifest`, `NoEvidence` panics; NOT a node mode.
- [ ] **No synthetic manifest committed** — no `PrivateRehearsalManifest` / BA-02 manifest is committed by this slice (the live execution is gated).
- [ ] The existing hermetic `c1_dry_run_correlate_to_rehearsal_envelope` is green (the reused correlate→manifest wiring).
- [ ] `ci_check_single_serve_dispatch_authority.sh` + `ci_check_serve_listener_magic_aware.sh` + `ci_check_node_run_loop_containment.sh` + `ci_check_node_path_fidelity.sh` + `ci_check_served_chain_handoff_fence.sh` — green / byte-unchanged.
- [ ] `cargo test -p ade_node` + `cargo test -p ade_runtime` green (no regression).

## 13. Failure Modes
- **NoEvidence** (the Haskell follower did NOT accept Ade's served block) → `node_c1_serve_live` panics, writes no manifest. Ade self-accept / served-block / wire-success is NOT acceptance — only the follower's `correlate`-matched log is.
- **Follower parks at tip before Ade forges** (needs proactive `advance_tip`) → **STOP and scope separately** (a new shared-path cluster); do not patch S3. (Deterministic; the runbook arranges the follower to start behind Ade's forged tip so request-driven serve suffices.)
- A bad/forged peer log → `correlate` yields `NoEvidence` (no match) → no manifest. Never a synthesized manifest.

## 14. Hard Prohibitions
### Inherited Cluster-Level Prohibitions
All PHASE4-N-F-G-H prohibitions apply: no `--mode produce` switch; no private-only path; no relay-loop serve mutation; no second serve authority/serializer; no new `--mode node` flag; no new BLUE authority/canonical type/variant (beyond S2b's already-landed builder); no RO-LIVE flip.
### Slice-Specific Prohibitions
- **No handshake-semantics change** — S2b's magic-aware serve is reused unchanged.
- **No proactive `advance_tip` / `served_chain_view.changed()` reactor** — request-driven serve only; a parked follower → stop-and-scope.
- **No manifest without a peer log through `correlate`** — Ade self-accept / served-block / wire-success is never acceptance.
- **No bounty/preprod completion claim; no RO-LIVE flip** — C1 is a non-promotable rehearsal; the bounty is the separate C2 preprod pass.
- **No synthetic/committed manifest** — the live execution is operator-gated; the harness writes only on a real operator-captured match, to an operator-specified path (never the committed evidence home in this slice).

## 15. Explicit Non-Goals
This slice MUST NOT: run the C2 preprod bounty pass or produce a bounty BA-02 manifest; add a proactive `advance_tip` driver; change the serve handshake / dispatch / any BLUE surface; switch to `--mode produce`; flip RO-LIVE; commit any manifest.

## 16. Completion Checklist
- [ ] Serve runbook committed (strict G-C/G-D adaptation; only the follower topology + labels differ).
- [ ] `node_c1_serve_live` env-gated + skipped in CI; reuses `correlate`/`rehearsal_pass`; `NoEvidence` panics.
- [ ] No synthetic manifest committed; the reused hermetic correlate test green.
- [ ] The 5 serve/containment/fidelity gates green / byte-unchanged; `cargo test -p {ade_node, ade_runtime}` green.
- [ ] No BLUE change; no handshake change; no `advance_tip`; no RO-LIVE flip.
- [ ] `DC-NODE-07` `evidence_notes` record the serve runbook + harness; binding + flip at the G-H close.

## 17. Review Notes
- **Now executable (fetch-reachable, not acceptance-proven):** the G-D C1 dry-run found `--mode node` couldn't serve; S2 wired the serve, S2b made it magic-aware. So S3's C1 serve dry-run is genuinely runnable — a magic-42 Haskell follower can now **reach + fetch** Ade's served block. The follower is **expected to accept it if it is protocol-valid**, but acceptance is proven ONLY by the follower log through `correlate`; the harness does not guarantee it. The operator pass is no longer scaffold-blocked, only operator-gated.
- **C1 ≠ bounty:** the C1 serve manifest is a non-promotable rehearsal (reuses G-D's `PrivateRehearsalManifest`); the bounty deliverable is the separate C2 preprod pass (BA-02, RO-LIVE-06) — not in scope.
- **The advance_tip boundary is load-bearing:** request-driven serve only; a parked-at-tip follower is a new cluster, never an S3 patch.
- **Implied follow-up:** the C2 preprod operator pass (the real RO-LIVE-01/06 flip) — a separate leg after C1 de-risks it.
