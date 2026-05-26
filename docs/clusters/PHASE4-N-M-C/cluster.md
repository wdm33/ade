# PHASE4-N-M-C — Live operator pass (cluster doc)

**Status:** CLOSED 2026-05-26. 11/12 rules `enforced`; DC-EVIDENCE-01 + RO-LIVE-05 `enforced_scaffolding` (full `BlockAdmitted` transcript gated on A1.1 reference-script seed-import — explicit C non-goal).
**Predecessors:** PHASE4-N-M-A (BootstrapAnchor + WAL), PHASE4-N-M-B (admission orchestrator).
**Successor:** none planned. C is the closure cluster for RO-LIVE-05.
**Sketch:** `docs/planning/phase4-n-m-c-operator-pass-invariants.md`.
**Evidence:** `docs/evidence/phase4-n-m-c-*` (consensus-inputs bundle + wire-only transcript + runbook).

## Primary invariant

**The `ade_node` binary admits real Conway blocks from a real
`cardano-node-preprod` peer, validates them through BLUE
`admit_via_block_validity` using imported, fingerprinted
`LiveConsensusInputs`, and emits a JSONL transcript that (a)
binds every block-event to the consensus-inputs fingerprint
and (b) contains zero false-accepts under an adversarial
mutation corpus.**

This invariant is the bounty's "tx/block-validity agreement"
acceptance test in concrete form. It does NOT claim full sync
or full live consensus (see §4 of the sketch).

## Scope

### In scope

- Single-peer admission against local docker
  `cardano-node-preprod` at `127.0.0.1:3001`.
- Conway-only admission within a single epoch window.
- Operator-extracted `LiveConsensusInputs` (cardano-cli JSON
  bundle).
- Adversarial false-accept corpus across 4 mandatory mutation
  classes (body / header-body / KES / VRF).
- JSONL transcript binding `(block_hash, slot,
  consensus_inputs_fingerprint)`.

### Explicit non-goals (¬P-C5, ¬P-C6)

- Block production.
- Multi-epoch admission.
- Chain selection across forks.
- Multi-day live runs.
- ChainDb persistence of admitted blocks.
- Mithril / Mithril-snapshot import.
- Reference-script seed-import support.
- Genesis → P self-replay.

## Slice index

| Slice | Purpose | New rules |
|---|---|---|
| **C1a** | Import schema + closed decode | CN-CONS-IN-01, DC-CONS-IN-01 |
| **C1b** | Canonicalization + Blake2b-256 fingerprint | DC-CONS-IN-02 |
| **C2** | `LiveLedgerView` + epoch-window guard + event-vocabulary extension | DC-VIEW-01, DC-ADMIT-10, DC-ADMIT-11 |
| **C3** | RED wire pump + Undecodable-tightening | CN-PUMP-01, DC-PUMP-01, DC-PUMP-02, DC-ADMIT-12 |
| **C4** | Adversarial false-accept corpus (4 mandatory mutations) | DC-EVIDENCE-02 |
| **C5** | Live operator pass against docker preprod | DC-EVIDENCE-01 |
| **C6** | Cluster close + RO-LIVE-05 closure | (closure) |

## Exit criteria (Mechanical Acceptance Criteria — cluster-level)

1. All 12 N-M-C rules `enforced` in `docs/ade-invariant-registry.toml`.
2. `RO-LIVE-05` flipped to `enforced` with the bounded
   statement from sketch §4.
3. 6 N-M-C CI gates pass:
   - `ci/ci_check_live_consensus_inputs_closure.sh`
   - `ci/ci_check_live_consensus_inputs_fingerprint.sh`
   - `ci/ci_check_live_ledger_view_epoch_window.sh`
   - `ci/ci_check_admission_wire_pump_closure.sh`
   - `ci/ci_check_admission_no_red_verdicts.sh`
   - `ci/ci_check_adversarial_false_accept_corpus.sh`
4. The C5 live transcript is committed under
   `docs/evidence/phase4-n-m-c-operator-pass-transcript.jsonl`
   (one canonical capture, redacted of any operator-secret
   data; the bounty-public layer).
5. Adversarial corpus integration test (C4) green: 4
   mutation classes each produce `Diverged` or
   `PeerSentUndecodableBytes`; zero `BlockAdmitted`; zero
   `Agreed`; zero `InputNotFound`.
6. C5 live test (gated by environment variable
   `ADE_LIVE_OPERATOR_TEST=1` so CI doesn't dial a real peer
   on every run) green: at least 1 `BlockAdmitted`, at least
   1 `Agreed`, zero `Diverged`, zero `BlockAdmitted` for any
   hash differing from a peer-announced hash at the same slot.
7. All N-M-B CI gates still pass (no regression).
8. `cargo build` + `cargo test` workspace-clean.
9. Commit + push to `main` with the
   `Co-Authored-By: Claude Opus 4.7 (1M context)` trailer
   (project override).

## TCB color + tier classification

| Component | Color | Tier |
|---|---|---|
| `ade_runtime::consensus_inputs::{json,importer,canonical}` | GREEN | release / derived |
| `ade_runtime::consensus_inputs::view::LiveLedgerView` | GREEN | derived |
| `ade_runtime::admission::wire_pump::run_admission_wire_pump` | RED | release |
| Block-event vocabulary extension (`consensus_inputs_fingerprint`) | GREEN | derived |
| Adversarial corpus integration test | GREEN | derived |
| Live operator-pass binary integration test (env-gated) | RED | release / derived |
| `admit_via_block_validity` exercised against real bytes | BLUE | (existing; unchanged) |

## Forbidden patterns (carried from sketch §2)

- No ambient consensus context.
- No cross-epoch silent use.
- No RED-derived verdicts.
- No partial importer fallback.
- No claim inflation.
- No wide-obligation closure.
- No `InputNotFound` for adversarial input.
- No reference-script seed-import fallback (DC-ADMIT-09 preserved).
- No silent clean-exit on adversarial bytes.
- No mid-epoch CLI swap.

## Replay obligations preserved

- DC-WAL-03 (anchor + WAL replay-equivalence) — unchanged. C5
  must show that re-running the captured peer-event stream
  produces a byte-identical WAL.
- DC-ADMIT-07 (admit-replay-equivalence, true-tier) —
  strengthened by C: same anchor + same
  `consensus_inputs_fingerprint` + same WAL produces same
  post-admit state.
- T-DET-01 (determinism supreme law) — strengthened by C:
  live admission preserves replay-equivalence with the
  fingerprint added as a canonical input.

## Open obligations after C closes

- RO-LIVE-03 (wide): still open.
- RO-LIVE-04 (wide): still open.
- RO-GENESIS-REPLAY-01: still open, `blocked_until_genesis_replay_cluster`.
- RO-MITHRIL-IMPORT-01: still open.
- A1.1 (reference-script TxOut decode): still open.
- Multi-epoch admission: future cluster.
- ChainDb persistence of admitted blocks: future strengthening.
- Block production live pass: future cluster.

## References

- Memory: [[feedback-evidence-reducers-are-green-not-authority]],
  [[feedback-shell-must-not-overstate-semantic-truth]],
  [[feedback-tx-validity-priority]],
  [[feedback-fail-closed-validation]],
  [[feedback-real-interop-finds-codec-bugs]],
  [[reference-local-preprod-docker-cardano-node]].
- Sketch: `docs/planning/phase4-n-m-c-operator-pass-invariants.md`.
- Predecessor closure records: `docs/clusters/PHASE4-N-M-B/`,
  N-M-A closure commit `c7e2a23`.
