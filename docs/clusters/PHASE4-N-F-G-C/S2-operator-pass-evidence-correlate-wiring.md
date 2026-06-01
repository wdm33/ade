# Invariant Slice — PHASE4-N-F-G-C S2: Operator-pass runbook + BA-02 evidence manifest + correlate wiring

> **Status:** Planning Artifact (Non-Normative). Normative authority is the registry + CI.

## 2. Slice Header

### Slice Name
Operator-pass runbook + BA-02 evidence manifest + `correlate` wiring (operator-gated scaffolding).

### Cluster
**PHASE4-N-F-G-C** — Live feed + operator-gated evidence (the operator-gated half).

### Status
Ready for Review (all §12 mechanical scaffolding green; `ade_node` suite + both BA-02 gates pass; the
live ACCEPT remains operator-gated by design).

### Cluster Exit Criteria Addressed
- [x] **CE-G-C-2 (operator-gated evidence — SCAFFOLDS ONLY; live ACCEPT BLOCKED)** — the corrected C1/C2
  runbook is committed; the evidence-manifest home + sha256 cross-check is defined; `ba02_evidence::correlate`
  is wired to the operator-captured peer log; **no synthetic manifest is committed**; the OQ5
  genesis-consistency pin (`genesis_pinning`) passes for the recovered seed epoch. Live peer ACCEPT stays
  `blocked_until_operator_stake_available` (`RO-LIVE-01` live half / `CN-CONS-06`) — proven ONLY by the
  peer's validation log.

CE-G-C-1 (live feed wiring) is **out of scope** for this slice (it is S1, merged `71036d10`).

### Slice Dependencies
- PHASE4-N-F-G-C S1 (live WirePump feed wiring) — **merged** on this branch (`71036d10`).
- PHASE4-N-F-C L6 (`ba02_evidence`: `correlate`, `parse_peer_accept_events`, `Ba02Manifest`,
  `BA02_MANIFEST_SCHEMA_VERSION = 1`) — merged.
- PHASE4-N-F-G-A S1 (`ade_testkit::consensus::genesis_pinning`) — merged (the OQ5 harness).

## 3. Implementation Instruction (AI)
Commit the corrected `--mode node` operator-pass runbook; add the BA-02 evidence-manifest schema gate
(mirroring `ci_check_operator_evidence_manifest_schema.sh`, **vacuous until a manifest is committed**);
wire the operator-captured peer-log file → `parse_peer_accept_events` → `correlate` →
`Ba02Manifest::to_canonical_json()` via a RED evidence-I/O path (env-gated operator-pass test, mirroring
`admission_live_operator_pass.rs` `ADE_LIVE_OPERATOR_TEST`) + a hermetic correlate-mechanics test. Do
**not** add a synthetic manifest, a new BA-02 constructor (only `correlate`), a new `Mode` variant, a new
registry rule, or flip `RO-LIVE-01`/`RO-LIVE-06`. Do **not** infer acceptance from any Ade-internal
signal. Commit messages carry the project attribution trailer (CLAUDE.md) and no other AI references.

## 4. Intent
Make it **impossible** to claim BA-02 peer acceptance from anything other than a real operator-captured
peer log run through the single `correlate` constructor: the evidence manifest is schema-bound,
sha256-cross-checked against the committed peer-log fixture, and `correlate`-produced — a synthetic or
hand-authored manifest, or any Ade-internal signal (self-accept / `ForgeSucceeded` / served-block / wire
success), cannot satisfy it. (Strengthens `RO-LIVE-06`; preserves `RO-LIVE-01` as operator-gated/partial.)

## 5. Scope
- **Modules / crates:**
  - `ade_node::ba02_evidence` (GREEN) — **reused unchanged**: `correlate` (sole `Ba02Manifest` ctor),
    `parse_peer_accept_events` (allow-list), `Ba02Manifest::to_canonical_json`. No new constructor.
  - `ade_node` operator-pass evidence-I/O (RED, NEW) — a thin path that reads the operator-captured
    peer-log file + the Ade forge record, runs `parse_peer_accept_events` + `correlate`, and (when
    live-gated) writes the manifest to the evidence home. Env-gated operator-pass integration test
    (`ADE_LIVE_OPERATOR_TEST`, mirroring `admission_live_operator_pass.rs`) + a hermetic
    correlate-mechanics test over a synthetic fixture.
  - `docs/evidence/` (NEW files) — the corrected runbook `phase4-n-f-g-c-operator-pass-README.md`; the
    BA-02 manifest home (committed only after a real pass) + its peer-log fixture (by sha256).
  - `ci/ci_check_ba02_evidence_manifest_schema.sh` (NEW gate) — vacuous-until-committed; on a committed
    BA-02 manifest, enforces the closed schema (`schema_version = BA02_MANIFEST_SCHEMA_VERSION`,
    `block_hash`, `slot`, `peer_log_file`, `peer_log_file_sha256`, `peer_log_capture_command`,
    `peer_log_filter`, `accept_event_kind`) + the `peer_log_file_sha256` cross-check + correlate-produced
    provenance.
- **State machines affected:** none. No node state, no ledger, no WAL, no served chain.
- **Persistence impact:** none on node state. The evidence manifest is a release artifact under
  `docs/evidence/`, not node-authoritative state.
- **Network-visible impact:** none in this slice (the live capture is the operator pass; this slice ships
  the runbook + the schema gate + the correlate wiring exercised hermetically).
- **Out of scope:** S1's live feed wiring (merged); any live ACCEPT / BA-02 satisfaction; flipping
  `RO-LIVE-01`/`RO-LIVE-06`; cross-epoch production; a new `Mode` variant; grounding-doc regeneration
  (that's `/cluster-close`).

## 6. Execution Boundary
- **BLUE (none):** no BLUE crate is touched.
- **GREEN (reused, unchanged):** `ade_node::ba02_evidence::{correlate, parse_peer_accept_events,
  Ba02Manifest, AdeForgeRecord}` — the comparison-only evidence reducer (a `Ba02Manifest` is a *claim
  about* authority, not authority).
- **RED (the slice's work):** the operator-pass evidence-I/O path (file read of the peer log + forge
  record; manifest write) + the env-gated operator-pass test; the runbook + the manifest-schema gate are
  docs/CI.
- **Color resolved:** no ambiguity — `correlate` is GREEN evidence (never authority); the file I/O is RED;
  no BLUE.

## 7. Invariants Preserved
- `RO-LIVE-01` — **stays `partial` / `blocked_until_operator_stake_available`.** This slice does NOT flip
  it (no live ACCEPT claim). Cluster-close may record `strengthened_in`; it remains operator-gated.
- `CN-CONS-06` — cross-impl acceptance live half stays `blocked_until_operator_stake_available`.
- `DC-EVIDENCE-01` — the operator-pass live-evidence scaffold discipline (env-gated test + committed
  runbook + manifest schema, vacuous-until-committed) is preserved + extended, not weakened.
- The `ba02_evidence` closed vocabulary (`PeerAcceptEvent`, `PeerAcceptSource`, `NoEvidenceReason`,
  `BA02Outcome`, `BA02_MANIFEST_SCHEMA_VERSION`) — `correlate` stays the **sole** `Ba02Manifest`
  constructor; the allow-list parser never coerces a non-acceptance line into acceptance
  (`ci_check_ba02_evidence_closed.sh` stays green).
- `DC-NODE-06`, `CN-NODE-02`, `DC-SYNC-01/02` — untouched (no node-spine / serve / tip code in S2).
- The S1 live-feed wiring + the broadened handoff fence + the byte-unchanged containment gate — untouched.

## 8. Invariants Strengthened or Introduced
- **`RO-LIVE-06` (strengthened — BA-02 evidence wiring + no-synthetic gate).** The operator-captured
  peer-log → `parse_peer_accept_events` → `correlate` → manifest path is wired and exercised, and a NEW
  `ci_check_ba02_evidence_manifest_schema.sh` makes a committed BA-02 manifest **schema-bound +
  sha256-cross-checked against its committed peer-log fixture + correlate-provenance-shaped** — a
  synthetic/hand-authored manifest fails the gate. **Stays operator-gated**: with no committed manifest
  (this HEAD) the gate is vacuously satisfied; a real BA-02 needs a real operator pass. Recorded as
  `strengthened_in += "PHASE4-N-F-G-C"` at cluster close.
- **`CN-OPERATOR-EVIDENCE-01` (strengthened — same family).** The N-S-C operator-pass manifest+sha256
  schema pattern is extended to the `--mode node` BA-02 evidence path (the C1-doc item-5 correlation
  fix). Recorded at cluster close.

> Single family: "operator-gated BA-02 peer-acceptance evidence is real-peer-log-only, schema-bound,
> sha256-cross-checked, and `correlate`-produced." `RO-LIVE-01` is **preserved, not strengthened** here —
> the live ACCEPT is a separate operator-witnessed event.

## 9. Design Summary
- **Runbook** (`docs/evidence/phase4-n-f-g-c-operator-pass-README.md`) supersedes the stale
  `docs/clusters/completed/PHASE4-N-S-C/S1.md`, fixing the C1-doc-identified bugs: the mandatory
  `--json-seed <utxo.json>` + `--consensus-inputs-path <bundle>` flags (G5); the G-C S1 `--peer
  <upstream>` live feed; **the peer is a follower, NOT a co-producer** (no forging credentials on the
  peer); the corrected `correlate`/jq evidence filter; the OQ5 genesis-consistency pre-check
  (`genesis_pinning`) **before** any live KES signature; and the G7 slot-alignment caveat.
- **Evidence home + sha256 cross-check** under `docs/evidence/`: a committed BA-02 manifest references
  its peer-log fixture by `peer_log_file_sha256` (the operator captures the peer's validation log, commits
  it, and the manifest binds it by sha256 — the `ci_check_operator_evidence_manifest_schema.sh` pattern).
- **`correlate` wiring**: a RED path reads the operator-captured peer-log file + the Ade forge record
  (`AdeForgeRecord::from_forge_artifact`), runs `parse_peer_accept_events` + `correlate`, and on a
  `Ba02Manifest` outcome writes `to_canonical_json()` to the evidence home. Exercised hermetically over a
  synthetic fixture (mechanics only — synthetic CANNOT satisfy BA-02); the live capture is the env-gated
  operator pass.
- **No synthetic manifest**: the schema gate fails a committed manifest whose `peer_log_file_sha256` does
  not match a committed peer-log fixture (a hand-authored manifest has no real fixture to bind).

## 10. Changes Introduced
### Types
- No new canonical type, no new `BA02Outcome`/`PeerAcceptEvent` variant, no new `Ba02Manifest`
  constructor (only `correlate`). No new `Mode` variant.
### State Transitions
- None (evidence is comparison-only; no authoritative transition).
### Persistence
- None on node state. New release artifacts under `docs/evidence/` (runbook now; manifest + fixture only
  after a real operator pass).
### Removal / Refactors
- Supersede the stale `completed/PHASE4-N-S-C/S1.md` runbook (the new runbook is the canonical
  `--mode node` operator-pass recipe). No code removal.

## 11. Replay, Crash, and Epoch Validation
- **Replay (`correlate` determinism, R2 / existing `RO-LIVE-06` property):** `correlate(ade, peer_log)`
  is a pure function of its inputs → byte-identical `BA02Outcome` on replay. Covered by the existing
  `ba02_evidence` `#[cfg(test)]` tests + a NEW hermetic test
  `correlate_from_operator_log_file_is_deterministic` (same fixture file ⇒ same outcome twice).
- **Crash/restart:** n/a — no node state; the evidence path is a one-shot read→compare→write.
- **Epoch boundary:** n/a — evidence comparison only.

## 12. Mechanical Acceptance Criteria
This slice is complete only when **all** of the following exist and pass in CI (hermetic — no operator
pass, no live peer):

- [x] `node_operator_pass_ba02.rs::correlate_wired_to_operator_peer_log` — the RED evidence-I/O path
  reads a peer-log **file** + forge record, runs `parse_peer_accept_events` + `correlate`, and yields
  `BA02Outcome::Ba02Manifest` on a matching `peer_served_block` fixture, `NoEvidence` on an
  Ade-internal-only fixture (`self_accept` / `forge_succeeded` / `block_received` lines), and a fail-closed
  `io::Error` on a missing file — synthetic fixture for reducer/mechanics only (lives in a `TempDir`,
  never under the BA-02 manifest home, never satisfies the manifest gate).
- [x] `node_operator_pass_ba02.rs::correlate_from_operator_log_file_is_deterministic` — same fixture file
  ⇒ byte-identical `BA02Outcome` (+ `to_canonical_json`) across two reads.
- [x] `ci_check_ba02_evidence_manifest_schema.sh` — NEW gate (117 → 118), exists + executable + green
  (vacuous — **no BA-02 manifest committed at this HEAD**); fails closed on a schema-incomplete or
  sha256-mismatched manifest (verified). This is the "no synthetic manifest" enforcer.
- [x] `ci_check_ba02_evidence_closed.sh` — existing closed-vocabulary gate stays **green** (no new
  constructor / variant; allow-list parser unchanged).
- [x] operator-pass runbook `docs/evidence/phase4-n-f-g-c-operator-pass-README.md` committed +
  contains the mandatory flags (`--peer`, `--json-seed`, `--consensus-inputs-path`), the
  follower-not-co-producer instruction, and the `correlate` evidence-capture procedure.
- [x] `cargo test -p ade_node` green (166 lib + all integration, incl. the 3 new BA-02 tests).

### Operator pass (NOT a slice-completion criterion — operator-gated)
The real BA-02 (`RO-LIVE-06` live half) and the live ACCEPT (`RO-LIVE-01`) are executed by a real
operator pass against a peer that can grant leadership (C1 private testnet, or C2 preprod with provisioned
stake) — `blocked_until_operator_stake_available`. This slice ships the runbook + wiring + schema gate;
it commits **no manifest**. **Evidence caveat (carried):** a run with `--peer` absent or unreachable
canNOT serve as live-feed evidence — the manifest/runbook must record that the live feed was exercised
(`--peer` supplied + feed Continuing); this is operational/evidence scope, not a runtime invariant.

## 13. Failure Modes
- `correlate` over a non-matching / Ade-internal-only log → `BA02Outcome::NoEvidence { reason }`
  (deterministic, structured). **Not** an error — "no evidence" is the honest outcome; it never coerces
  to acceptance.
- A committed BA-02 manifest whose `peer_log_file_sha256` does not match its fixture → gate **FAIL**
  (fail-closed; a hand-authored/synthetic manifest is rejected).
- Missing peer-log file at the runbook path during a live pass → operator-side error (documented in the
  runbook); no manifest is produced. **Not** a node runtime path.

## 14. Hard Prohibitions
### Inherited Cluster-Level Prohibitions
All PHASE4-N-F-G-C "Forbidden During This Cluster" prohibitions apply (no containment/handoff-fence
relaxation; no synthetic BA-02 manifest; no "peer accepted" rule; etc.).
### Slice-Specific Prohibitions
- **Ade self-accept ≠ peer acceptance; served block ≠ peer acceptance; wire success ≠ peer acceptance.**
  Only an operator-captured peer log through `correlate` may produce BA-02 evidence.
- No new `Ba02Manifest` constructor — `correlate` is the sole one.
- No new `BA02Outcome` / `PeerAcceptEvent` / `PeerAcceptSource` variant; no weakening of the allow-list
  parser (it must keep dropping non-acceptance lines).
- **No synthetic / hand-authored BA-02 manifest committed.** A committed manifest MUST bind a real
  peer-log fixture by matching sha256.
- **No `RO-LIVE-01` or `RO-LIVE-06` status flip in this slice; both remain partial / operator-gated
  until a real operator-witnessed pass produces peer-log evidence.**
- No new `Mode` variant; no new registry rule.
- No node-spine / serve / tip / forge code change (S2 is evidence-only).

## 15. Explicit Non-Goals
This slice MUST NOT: run or simulate a live operator pass; claim or infer peer acceptance / BA-02
satisfaction; commit a BA-02 manifest; flip `RO-LIVE-01`/`RO-LIVE-06`; add a `Mode` variant or a registry
rule; touch the live-feed wiring (S1), the served chain, the forge, or any BLUE crate; optimize
performance.

## 16. Completion Checklist
- [ ] All new artifacts are release-scope (runbook), not node-authoritative state.
- [ ] All failure modes are deterministic (`NoEvidence` / gate-fail; §13).
- [ ] No TODOs/placeholders in authoritative (BLUE) paths (no BLUE change).
- [ ] CI enforces the strengthened invariant (BA-02 manifest schema + sha256 + no-synthetic gate).
- [ ] `correlate` determinism test passes; `ci_check_ba02_evidence_closed.sh` stays green.

## 17. Review Notes
- **Invariant risk considered:** the central risk is overclaiming acceptance. Mitigated by: `correlate`
  as the sole constructor, the allow-list parser (Ade-internal signals not representable), the
  sha256-bound manifest (no hand-authored manifest), and the vacuous-until-committed gate (no manifest =
  no claim).
- **Carried evidence caveat:** `--peer` absent/unreachable ⇒ the run is NOT live-feed evidence; the
  runbook + manifest must record the live-feed-exercised condition. Operational/evidence scope, NOT a
  runtime invariant (no fatal condition added to the lifecycle — consistent with S1's conditional wiring).
- **OQ5 (slice-entry proof obligation):** the runbook requires pinning `genesis_pinning` for the recovered
  seed epoch **before** any live KES signature — necessary but NOT sufficient for acceptance (acceptance
  still requires the operator-captured peer log through `correlate`).
- **Follow-up implied:** the operator-witnessed live pass (C1/C2) that produces a real manifest + flips
  `RO-LIVE-01`/`RO-LIVE-06` at a future registry review — outside this branch.
