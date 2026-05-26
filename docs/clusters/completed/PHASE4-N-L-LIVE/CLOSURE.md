# PHASE4-N-L-LIVE — Closure record

> Companion to `cluster.md`. Records what shipped, the registry
> deltas, the captured live evidence, and the carry-forward
> obligations.

## Registry deltas applied

| Rule | Change | Notes |
|------|--------|-------|
| `RO-LIVE-04` | `declared → enforced` | Wire-only smoke. Operator pass captured at `docs/clusters/completed/PHASE4-N-L-LIVE/CE-N-L-LIVE-LIVE.log`. `strengthened_in = ["PHASE4-N-L-LIVE"]`. Full `code_locus`, `tests`, `ci_script` populated. |
| `RO-LIVE-05` | unchanged (`declared`, `open_obligation = "blocked_until_ledger_seed_cluster"`) | Carried forward. Closes via the future `PHASE4-N-M-LEDGER-SEED` cluster. |
| `RO-LIVE-03` | `evidence_notes` refreshed + `open_obligation` narrowed from `"blocked_until_RO-LIVE-04_and_RO-LIVE-05_close"` to `"blocked_until_RO-LIVE-05_close"`. Statement untouched. | The original wide rule still bundles two claims; RO-LIVE-04 is now closed, RO-LIVE-05 remains the only blocker. |

## Captured live evidence

`docs/clusters/completed/PHASE4-N-L-LIVE/CE-N-L-LIVE-LIVE.log`:

```
{"event":"node_started","mode":"wire_only","peer_count":1}
{"event":"peer_dial_started","peer":"127.0.0.1:3001"}
{"event":"handshake_ok","peer":"127.0.0.1:3001","negotiated_version":15}
{"event":"peer_tip_read","peer":"127.0.0.1:3001","slot":23013663,"hash_hex":"2a2fd42f8f1a3f9fc4faf3d448c5b3d84a8c5d0b1039aa90b5c022e11cb4704f","block_no":720328}
{"event":"wire_smoke_complete","admission_enabled":false,"peer_count_ok":1,"peer_count_failed":0}
{"event":"node_shutdown","reason":"tip_read_complete"}
```

- **Peer**: local docker `cardano-node-preprod`
  (`ghcr.io/intersectmbo/cardano-node:11.0.1`, preprod magic = 1,
  127.0.0.1:3001). See `.cardano-node-preprod/README.md`.
- **Negotiated N2N version**: 15 (max common from our V11..V16
  proposal + the peer's V11..V16 advertised set).
- **Tip read**: preprod slot 23013663, block 720328,
  header hash
  `2a2fd42f8f1a3f9fc4faf3d448c5b3d84a8c5d0b1039aa90b5c022e11cb4704f`.
- **Exit code**: 0.
- **Forbidden literals NOT emitted**: `agreement_verdict`,
  `admitted_block`, `ledger_applied`, `projection_updated`
  (verified by `jq -c '.event' < log` + by
  `ci/ci_check_wire_only_event_vocabulary_closed.sh`).

## Mechanical artifacts shipped

### New GREEN files (3)
- `crates/ade_node/src/live_log/mod.rs` — barrel.
- `crates/ade_node/src/live_log/event.rs` — closed
  `LiveLogEvent` sum (7 variants) + `ModeTag` + closed
  `WireOnlyShutdownReason` + closed `PeerDialFailureKind` +
  `discriminator()` stable-string mapping.
- `crates/ade_node/src/live_log/writer.rs` — hand-rolled JSON
  writer over `LiveLogEvent`. No serde-derive dep.

### New RED files (1) + modified files (3)
- `crates/ade_node/src/wire_only.rs` (new RED) —
  `run_wire_only` + `run_admission_unavailable` +
  `wire_only_peer_session` + structured `our_n2n_versions`
  (per-version `NodeToNodeVersionData` CBOR record) + tip
  reader.
- `crates/ade_node/src/main.rs` — replaced print-and-exit
  stub with `#[tokio::main]` routing on `cli.mode`.
- `crates/ade_node/src/cli.rs` — added `Mode` enum
  (`WireOnly | Admission`), `--mode`, `--log`,
  `--tip-read-timeout-secs` flags, two new `CliError`
  variants.
- `crates/ade_node/src/lib.rs` — re-exports.

### New integration tests (7)
- `crates/ade_node/tests/wire_only_loopback.rs` — full
  hermetic test of the wire-only mode via an in-process
  loopback responder. Covers exit code, event sequence, tip
  agreement, negative-event guards (forbidden literals),
  admission fail-closed, peer-dial-failure exit path, JSONL
  one-object-per-line validity.

### New CI scripts (2)
- `ci/ci_check_wire_only_event_vocabulary_closed.sh` —
  RO-LIVE-04 ¬P-1.
- `ci/ci_check_wire_only_no_bootstrap.sh` —
  RO-LIVE-04 ¬P-2.

### Test summary
- `cargo test -p ade_node --lib` → **14 passed, 0 failed**.
- `cargo test -p ade_node --test wire_only_loopback` →
  **7 passed, 0 failed**.
- All 13 PHASE4-N-K + N-L + N-L-LIVE CI gates pass.

## Cluster docs

- Sketch: `docs/planning/phase4-n-l-live-wire-smoke-invariants.md`
- Cluster doc:
  `docs/clusters/completed/PHASE4-N-L-LIVE/cluster.md`
- Slice docs:
  `docs/clusters/completed/PHASE4-N-L-LIVE/N-L-LIVE-S{1,2,3}.md`
- Procedure:
  `docs/clusters/completed/PHASE4-N-L-LIVE/CE-N-L-LIVE-PROCEDURE.md`
- Captured log:
  `docs/clusters/completed/PHASE4-N-L-LIVE/CE-N-L-LIVE-LIVE.log`

## Honest-scope carry-forwards

- **RO-LIVE-05 (live admission/agreement)** — still
  `blocked_until_ledger_seed_cluster`. Closes via a future
  PHASE4-N-M-LEDGER-SEED cluster that adds either a
  genesis-JSON → initial-LedgerState builder or a
  cardano-node ledger-snapshot importer.
- **RO-LIVE-03 (wide rule)** — `evidence_notes` refreshed;
  `open_obligation` narrowed to `"blocked_until_RO-LIVE-05_close"`.
- **RO-LIVE-01, RO-LIVE-02, CN-CONS-06** — unchanged.

## Doctrine reference

This cluster was the inaugural application of
[[feedback-shell-must-not-overstate-semantic-truth]] in the
registry split: RO-LIVE-03 was found to bundle two layer
claims at planning time; rather than over-evidence the wide
rule we appended two narrower ones (RO-LIVE-04 wire smoke +
RO-LIVE-05 admission) and shipped only the wire-side claim
mechanically + with live evidence. The closed `LiveLogEvent`
enum + CI grep enforce the discipline at the file-tree level.
The discipline was also locked into the repo's CLAUDE.md +
`.cardano-node-preprod/README.md` so the next operator does
not have to rediscover the AWS-vs-local scope split.
