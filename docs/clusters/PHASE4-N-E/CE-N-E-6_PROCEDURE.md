# CE-N-E-6 — operator procedure for live N2N evidence capture

> **Status:** Operator procedure (non-normative). Closure of CE-N-E-6
> requires running this procedure against a real cardano-node N2N
> peer and committing the resulting log to
> `docs/clusters/PHASE4-N-E/CE-N-E-6_<YYYY-MM-DD>.log`. The procedure
> mirrors the PHASE4-N-B CE-N-B-6 pattern.

## What CE-N-E-6 asserts

Real `cardano-node` peer submits txs to Ade over N2N tx-submission2;
Ade's verdicts on the received tx_bytes are byte-identical to direct
corpus replay against the same tx bytes. The mechanical half (the
deterministic GREEN bridge `ade_core_interop::tx_submission`) is
already CI-green via the S4 integration tests. This procedure adds
the live-wire half: real bytes coming off a real socket.

## Pre-conditions

- Reachable cardano-node N2N relay (preprod recommended; mainnet
  acceptable). Hostname / network-magic redacted from this document
  per `feedback_no_credential_leaks`.
- Ade workspace built (`cargo build -p ade_core_interop`).
- A fresh `LedgerState` snapshot to use as `base`. The snapshot loader
  (`ade_testkit::harness::snapshot_loader`) supplies one for the same
  epoch as the relay's current tip.

## Procedure

1. **Handshake + tx-submission2 session.** Connect to the relay via the
   PHASE4-N-A handshake codec, negotiate tx-submission2 v15+, and run a
   client session that collects every `InventoryEvent::TxsDelivered`
   for a sustained window:
   - duration ≥ 10 minutes,
   - ≥ 50 distinct `tx_bytes` observed (collected via
     `event_to_ingress` and accumulated by `PeerAccumulator`).

   The session loop mirrors the `live_consensus_session` binary
   pattern in `crates/ade_core_interop/src/bin/`. A dedicated
   `live_tx_submission_session` binary is the natural future home;
   in this slice the procedure references it as the planned executable.

2. **Bridge to mempool.** Feed the captured per-peer InventoryEvent
   stream through `ade_core_interop::tx_submission::ingest_n2n_events`:

   ```rust
   let (mempool, outcomes) = ingest_n2n_events(base.clone(), &per_peer);
   ```

   For each tx in the captured stream, log:
   - tx_id (computed from tx_bytes by the BLUE path),
   - AdmitOutcome variant (Admitted / Rejected),
   - on Rejected: the `TxRejectClass` and `TxValidityError`,
   - on Admitted: the accumulating-state delta size (utxo entries
     added/removed).

3. **Cross-check against direct `tx_validity`.** For every captured tx
   bytes, also evaluate `ade_ledger::tx_validity::tx_validity(&base, &tx_bytes)`
   directly (no ingress wrapping). Assert that each verdict matches
   the bridge's `AdmitOutcome`:
   - Bridge `Admitted` ↔ direct `Valid`.
   - Bridge `Rejected { class }` ↔ direct `Invalid { class, .. }`
     with the same class.

   A divergence is a release-blocking false-accept / false-reject; the
   procedure halts and the log is committed as an evidence-of-bug
   artifact rather than evidence-of-pass.

4. **Commit the log.** Write the captured session to
   `docs/clusters/PHASE4-N-E/CE-N-E-6_<YYYY-MM-DD>.log` with the
   following minimum content:

   ```
   [live] peer=<redacted> network=<magic> mode=tx-submission2 v15+
   [live] handshake accepted at v<negotiated>
   [live] session window: <ISO-8601 start> .. <ISO-8601 end>
   [live] observed N=<count> distinct tx_bytes from M=<count> peers
   [bridge] admitted=<count> rejected=<count>
   [agreement] bridge ≡ direct tx_validity for <count>/<count> txs
   [agreement] divergences: <0 expected; if >0, list each one>
   ```

5. **Cluster gate.** CE-N-E-6 is closed when:
   - the log entry is committed under `docs/clusters/PHASE4-N-E/`,
   - the `[agreement] divergences:` line reports 0, AND
   - the cluster's TRACEABILITY entry for CE-N-E-6 references the
     specific log filename.

## Why this is operator-action, not CI

A CI runner cannot reliably reach a healthy `cardano-node` relay,
maintain the connection over 10+ minutes, and observe ≥ 50 distinct
txs without flaking. The PHASE4-N-B CE-N-B-6 close used the same
operator-action pattern for the same reason. The mechanical half
(the GREEN bridge) is CI-green; the live-log half is captured by an
operator and committed as an artifact.

## Non-goals

- Hardening the binary against arbitrary cardano-node misbehavior
  (separate Tier-5 cluster on session resilience).
- Multi-peer load testing (CE-N-E-6 needs only a single healthy peer).
- Closing CE-N-E-7 (N2C; that's S5's procedure).

## Reference

- `crates/ade_core_interop/src/tx_submission.rs` — the GREEN bridge.
- `crates/ade_core_interop/tests/tx_submission_ingress.rs` — the
  mechanical agreement evidence over synthetic InventoryEvents.
- `docs/clusters/PHASE4-N-E/cluster.md` — CE-N-E-6 statement.
- `docs/clusters/completed/PHASE4-N-B/` and the PHASE4-N-B archived
  `CE-N-B-6_2026-05-20.log` for the precedent.
