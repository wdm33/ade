# CE-N-E-7 — operator procedure for live N2C evidence capture

> **Status: DEFERRED to the future node-binary cluster as cross-cluster
> obligation `CE-NODE-N2C-LTX`.** This procedure spec is retained in
> place because the operator workflow it describes is the canonical
> spec for the future closure. PHASE4-N-E deliberately does NOT
> build an N2C UDS server — that scaffolding belongs to `ade_node`
> when it becomes a real Cardano node. See the "Deferred /
> cross-cluster obligation" section of `cluster.md` for the framing
> and the requirement on the future node-binary cluster.
>
> Until that cluster lands, CE-N-E-7 is open. The procedure below
> describes what closure will look like; do NOT attempt to satisfy
> it by building temporary UDS scaffolding inside PHASE4-N-E.
> When the future node-binary cluster discharges this obligation,
> the evidence log lands at
> `docs/clusters/<future-cluster>/CE-NODE-N2C-LTX_<YYYY-MM-DD>.log`
> (not under PHASE4-N-E).

## What CE-N-E-7 asserts

Real `cardano-cli transaction submit` to Ade over N2C
local-tx-submission UDS produces verdicts byte-identical to N2N
submission of the same tx bytes. The mechanical half (the GREEN
bridge `ade_core_interop::local_tx_submission` and the
cross-bridge test `n2n_and_n2c_bridges_produce_identical_outcomes`)
is already CI-green via S5's integration tests. This procedure
adds the live-wire half: bytes coming off a real UDS socket from a
real cardano-cli invocation.

## Pre-conditions

- A running Ade node binary exposing a Unix-domain N2C socket for
  the local-tx-submission mini-protocol.
- `cardano-cli` installed (any 8.x or 9.x series; the
  local-tx-submission wire format is era-stable).
- A small set of pre-signed test transactions to submit (or a
  scratch wallet for online signing).
- A fresh `LedgerState` snapshot to use as `base`.

## Procedure

1. **Bring up Ade's N2C UDS endpoint.** Serve the local-tx-submission
   mini-protocol on a documented UDS path. The N2C handshake
   negotiates a v15+ local-tx-submission version against
   cardano-cli's protocol-magic.

2. **Submit txs via cardano-cli.** Run

   ```
   cardano-cli transaction submit \
       --tx-file <path> \
       --testnet-magic <magic>     \
       --socket-path /var/run/ade.socket
   ```

   at least 10 times across at least 2 distinct cardano-cli invocations
   (different shells / processes). Each invocation MUST exercise the
   N2C handshake from scratch.

3. **Capture via the bridge.** The N2C session loop collects every
   `LocalTxSubmissionEvent::TxSubmitted` per client, builds per-client
   `ClientAccumulator`s, drains them into `PeerSubmissionQueue`s, and
   calls `ingest_n2c_events(base.clone(), &per_client)`. For each tx:

   - log tx_id (computed from tx_bytes by the BLUE path),
   - log AdmitOutcome variant,
   - on Rejected: log `TxRejectClass` and `TxValidityError`.

4. **Cross-bridge agreement assertion.** For every captured tx bytes,
   route the same bytes ALSO through the S4 N2N bridge
   (`ingest_n2n_events` with a single synthetic peer carrying one
   `InventoryEvent::TxsDelivered`). Assert the N2N and N2C bridges
   produce byte-identical `(MempoolState, AdmitOutcome)`. A divergence
   is a release-blocking source-leak (the N-E-N7 invariant must hold:
   `IngressEvent.source` is metadata only).

5. **Commit the log.**
   `docs/clusters/PHASE4-N-E/CE-N-E-7_<YYYY-MM-DD>.log` with at least:

   ```
   [live] socket=<path> magic=<protocol-magic> mode=local-tx-submission v15+
   [live] handshake accepted at v<negotiated>
   [live] session window: <ISO-8601 start> .. <ISO-8601 end>
   [live] observed N=<count> tx submissions across M=<count> cli invocations
   [bridge n2c] admitted=<count> rejected=<count>
   [agreement] n2n_bridge ≡ n2c_bridge for <count>/<count> txs
   [agreement] divergences: <0 expected; if >0, list each one>
   ```

6. **Cluster gate.** CE-N-E-7 is closed when:
   - the log entry is committed under `docs/clusters/PHASE4-N-E/`,
   - the `[agreement] divergences:` line reports 0,
   - the cluster's TRACEABILITY entry for CE-N-E-7 references the
     specific log filename.

## Why this is operator-action

A CI runner has no `cardano-cli` and no Ade node binary running as a
service. The cross-bridge mechanical evidence
(`n2n_and_n2c_bridges_produce_identical_outcomes`) IS in CI; the
live-wire half is captured by an operator and committed.

## Non-goals

- Multi-tenant cardano-cli auth / ACL (Tier-5).
- Local-state-query / local-tx-monitor mini-protocols (separate
  cluster).
- Stress testing the UDS server (Tier-5 resilience cluster).

## Reference

- `crates/ade_core_interop/src/local_tx_submission.rs` — the GREEN
  bridge.
- `crates/ade_core_interop/tests/local_tx_submission_ingress.rs` —
  mechanical evidence; especially
  `n2n_and_n2c_bridges_produce_identical_outcomes`.
- `docs/clusters/PHASE4-N-E/cluster.md` — CE-N-E-7 statement.
- `docs/clusters/PHASE4-N-E/CE-N-E-6_PROCEDURE.md` — sibling N2N
  procedure.
