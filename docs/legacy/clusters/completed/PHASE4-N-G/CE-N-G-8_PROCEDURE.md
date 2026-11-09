# CE-N-G-8 Procedure — Operator-action live evidence

**Status at HEAD:** `blocked_until_operator_peer_available`. No
private cardano-node peer is wired to this clone at cluster close.
The `RO-LIVE-01` registry rule ships as `partial` with an
`open_obligation` to flip to `enforced` once the operator captures
the log this procedure produces.

This procedure mirrors PHASE4-N-C's `CE-N-C-8_PROCEDURE.md` — same
operator-action pattern, different evidence target. Where N-C asked
"does cardano-node accept an Ade-forged block as the next chain
head?", N-G asks "does cardano-node successfully fetch and validate
an Ade-served block via block-fetch?".

The bytes path under test:

```
AcceptedBlock (PHASE4-N-C)
  → BroadcastQueue (N-C-S6)
  → ServedChainSnapshot (N-G-S2, admitted via N-G-S5 GREEN adapter)
  → producer_block_fetch_serve (N-G-S4) returning [StartBatch, Block{bytes}, BatchDone]
  → encoded BlockFetchMessage frames (N-A codec)
  → mux frame (N-A mux::frame, BLUE)
  → tokio socket (operator-supplied)
  → cardano-node peer's block-fetch client
  → cardano-node header+body validators
```

## Prerequisites

1. A private cardano-node binary (1.35+ recommended; preprod / preview
   / private-devnet configuration). Public endpoints are not
   appropriate — see [[feedback-competition-secrecy]] and
   [[feedback-no-credential-leaks]].
2. A captured AcceptedBlock corpus from a PHASE4-N-C
   `live_block_production_session` run, or a fresh forge against the
   same private peer with stake provisioned (N-C's
   `blocked_until_operator_stake_available` follow-on).
3. The Ade workspace built: `cargo build --release -p ade_core_interop`.

## Run the live pass

```bash
cargo run --release -p ade_core_interop \
    --bin live_block_fetch_session -- \
    --connect \
    --network <preprod|preview|custom> \
    --target <peer-host>:<peer-port> \
    --out docs/clusters/PHASE4-N-G/CE-N-G-LIVE_$(date -u +%Y%m%d).log
```

The binary's `--connect` mode in this slice prints the wiring stub
plan; the actual tokio socket → n2n_server bridge is the operator's
out-of-band integration step (the `n2n_server` module is a pure
driver; plugging it into a real socket is one layer up and uses the
N-A `session` shell).

## Evidence file shape (CE-N-G-LIVE_<date>.log)

JSON-Lines, one record per RequestRange answered:

```jsonl
{"slot": 163900801, "block_hash": "<hex>", "served_bytes_len": 1234, "peer_verdict": "accepted"}
{"slot": 163900802, "block_hash": "<hex>", "served_bytes_len": 1567, "peer_verdict": "accepted"}
```

`peer_verdict` is captured out-of-band from the cardano-node log
(grep for `Block from <Ade peer> accepted`). The operator commits
this file to `docs/clusters/PHASE4-N-G/`.

## Flipping the registry

When the log is captured and committed:

1. Edit `docs/ade-invariant-registry.toml`:
   - `RO-LIVE-01.status = "enforced"` (was `"partial"`)
   - `RO-LIVE-01.evidence += ["docs/clusters/PHASE4-N-G/CE-N-G-LIVE_<date>.log"]`
   - Remove the `open_obligation` field.
2. Commit with the standard project trailer (see `CLAUDE.md`).

## If the peer rejects a block

Investigate before re-running:

- `block_body_hash` mismatch → CE-N-G-7 should have already caught
  this in CI. If it did not, the canonical body-hash recipe
  (DC-CONS-16) has drifted; that is a CN-CONS-07 / DC-CONS-16
  regression, not an N-G concern.
- VRF / KES verification fail → producer signing (PHASE4-N-C S1)
  regression. Capture the cardano-node debug log and follow the
  CE-N-C-8 procedure first.
- Mux / framing mismatch → ade_network::mux::frame regression
  (PHASE4-N-A).

The `live_block_fetch_session` binary is a thin shell; failures
almost always trace to a deeper invariant the underlying tests
should already cover.
