# CE-N-H-6 Procedure — Operator-action live evidence

**Status at HEAD:** `blocked_until_operator_peer_available`. No
private cardano-node peer is wired to this clone at cluster close.
The `RO-LIVE-02` registry rule ships as `partial` with an
`open_obligation` to flip to `enforced` once the operator captures
the log this procedure produces.

Mirrors PHASE4-N-C's `CE-N-C-8_PROCEDURE.md` and PHASE4-N-G's
`CE-N-G-8_PROCEDURE.md`. Where N-C asked "does cardano-node accept
an Ade-forged block?" and N-G asked "does cardano-node fetch an
Ade-served block?", N-H asks "does an Ade follower, fed
RollForward + BlockDelivered from a real cardano-node peer over a
captured follow window, produce a ChainDb tip equal to the peer's
announced tip at every step?"

The bytes path under test:

```
cardano-node peer's chain-sync server-side
  → N2N socket
  → ade_network::chain_sync codec decode
  → ade_runtime::receive::dispatch_chain_sync_inbound (N-H S4)
  → lift to ReceiveEvent::RollForward (N-H S3)
  → receive_apply: caches header (N-H S2)

cardano-node peer's block-fetch server-side
  → N2N socket
  → ade_network::block_fetch codec decode
  → ade_runtime::receive::dispatch_block_fetch_inbound (N-H S4)
  → lift to ReceiveEvent::BlockDelivered (N-H S3)
  → receive_apply: cross-check header, run block_validity, admit (N-H S2)
  → ChainDbWriter::write_admitted (N-H S3)
  → ChainDb tip advances
```

## Prerequisites

1. A private cardano-node binary (preprod / preview / private-devnet).
   Public endpoints are not appropriate
   (see [[feedback-no-credential-leaks]],
   [[feedback-competition-secrecy]]).
2. The Ade workspace built:
   `cargo build --release -p ade_core_interop`.

## Run the live pass

```bash
cargo run --release -p ade_core_interop \
    --bin live_block_follow_session -- \
    --connect \
    --network <preprod|preview|custom> \
    --target <peer-host>:<peer-port> \
    --out docs/clusters/PHASE4-N-H/CE-N-H-LIVE_$(date -u +%Y%m%d).log
```

In this slice the `--connect` mode prints the wiring stub plan; the
actual tokio socket → orchestrator bridge is the operator's out-of-
band integration step. The receive orchestrator (S4) is a pure
driver; plugging it into a real socket uses the N-A `session` shell
infrastructure.

## Evidence file shape (CE-N-H-LIVE_<date>.log)

JSON-Lines, one record per admitted block:

```jsonl
{"peer_tip_slot": 163900801, "peer_tip_hash": "<hex>", "ade_tip_slot": 163900801, "ade_tip_hash": "<hex>", "match": true}
{"peer_tip_slot": 163900802, "peer_tip_hash": "<hex>", "ade_tip_slot": 163900802, "ade_tip_hash": "<hex>", "match": true}
```

`match: true` requires the operator's wire layer to compare peer
tip (from chain-sync `RollForward.tip`) with the Ade ChainDb tip
after admission. The operator commits this file to
`docs/clusters/completed/PHASE4-N-H/` (after cluster close moves the
cluster doc to `completed/`).

## Flipping the registry

When the log is captured and committed:

1. Edit `docs/ade-invariant-registry.toml`:
   - `RO-LIVE-02.status = "enforced"` (was `"partial"`).
   - `RO-LIVE-02.evidence += ["docs/clusters/completed/PHASE4-N-H/CE-N-H-LIVE_<date>.log"]`.
   - Remove the `open_obligation` field.
2. Commit with the standard project trailer.

## If admission disagrees with peer tip

Investigate before re-running:

- `HeaderBodyMismatch` → DC-CONS-19 surfaces it. The wire stream
  must always deliver the body matching the most recent RollForward
  at the same `(slot, hash)`. If a real peer triggers this, capture
  the trace and dig — likely a fork or rollback we don't yet
  support (Path A: RollBackward returns RollbackOutOfScope; the
  orchestrator halts and logs).
- `Validity(_)` → block_validity rejected the body. CE-N-H-5
  (`receive_pipeline_corpus_drive`) should have caught this. If
  not, the disagreement is real — the block decodes via Ade but
  rejects, and we have a B1 / N-B regression to dig.
- `RollbackOutOfScope` → expected in Path A. The orchestrator
  halts the peer; operator restarts from a fresh state. The
  follow-on rollback cluster will close this gap.
- ChainDb-write failure → N-D regression.
