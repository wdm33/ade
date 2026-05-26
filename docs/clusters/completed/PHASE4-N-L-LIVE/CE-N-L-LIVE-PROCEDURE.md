# PHASE4-N-L-LIVE — Operator-action procedure (CE-N-L-LIVE-6)

> **Honest-scope statement.** This procedure closes the
> **wire-only** half of RO-LIVE-03 (now tracked as **RO-LIVE-04**).
> It evidences that the binary can dial a real cardano-node peer
> over TCP, complete the N2N handshake, read the peer's announced
> tip, and exit cleanly. It does **NOT** evidence block admission,
> agreement verdict, or any semantic ledger claim — those are
> RO-LIVE-05's deliverable, blocked on the future
> `PHASE4-N-M-LEDGER-SEED` cluster.

## Target

The repo ships a local preprod cardano-node in a docker container:

- Image: `ghcr.io/intersectmbo/cardano-node:11.0.1`
- Container name: `cardano-node-preprod`
- Listen: `127.0.0.1:3001` (N2N)
- Bind mounts: `.cardano-node-preprod/{config,db,ipc}`
- Protocol magic: `1` (preprod)

The container is the canonical live-pass target — no external
network, no AWS, no IP redaction. The ledger DB at
`.cardano-node-preprod/db/` is a pre-synced preprod ImmutableDB
so the node only needs to replay the volatile tail on startup.

## Build

```
cd /home/ts/Code/rust/ade
cargo build --release -p ade_node
```

## Start the cardano-node container

```
docker start cardano-node-preprod
# Wait for it to finish ledger replay (look for the first
# `ChainExtended` or `AddBlock` log line; the bound port at
# 127.0.0.1:3001 will not accept handshakes until then):
docker logs -f cardano-node-preprod 2>&1 | grep -E 'ChainExtended|AddBlock'
```

## Run the wire-only pass

```
LOG=docs/clusters/completed/PHASE4-N-L-LIVE/CE-N-L-LIVE-LIVE.log
./target/release/ade_node \
    --mode wire_only \
    --genesis-path /dev/null \
    --network preprod \
    --peer 127.0.0.1:3001 \
    --log "$LOG" \
    --tip-read-timeout-secs 30
```

`--genesis-path /dev/null` is parsed-but-unused on the wire-only
path (the flag remains required for the future admission cluster).

## Expected JSONL output

```
{"event":"node_started","mode":"wire_only","peer_count":1}
{"event":"peer_dial_started","peer":"127.0.0.1:3001"}
{"event":"handshake_ok","peer":"127.0.0.1:3001","negotiated_version":15}
{"event":"peer_tip_read","peer":"127.0.0.1:3001","slot":NNNNNNNN,"hash_hex":"...","block_no":NNNNN}
{"event":"wire_smoke_complete","admission_enabled":false,"peer_count_ok":1,"peer_count_failed":0}
{"event":"node_shutdown","reason":"tip_read_complete"}
```

Exit `0` on a successful tip-read. Exit `20`
(`EXIT_LIVE_PASS_PEER_FAILURE`) if the handshake fails or the tip
read times out. Exit `1` (`EXIT_GENERIC_STARTUP`) on CLI error.

## Validate the log

```
jq -c '.event' < docs/clusters/completed/PHASE4-N-L-LIVE/CE-N-L-LIVE-LIVE.log
```

Should print one event discriminator per line, in the expected
order. No `agreement_verdict` / `admitted_block` / `ledger_applied`
/ `projection_updated` (the closed enum forbids them at the type
level and CI grep forbids them at the file-tree level).

## Cluster close

Once the captured log is present at
`docs/clusters/completed/PHASE4-N-L-LIVE/CE-N-L-LIVE-LIVE.log`:

1. Flip `RO-LIVE-04` from `declared` to `enforced` in
   `docs/ade-invariant-registry.toml`. Populate `code_locus`,
   `tests`, `ci_script`, `strengthened_in = ["PHASE4-N-L-LIVE"]`,
   and `evidence_notes` with the captured `negotiated_version`
   + tip slot.
2. Refresh `RO-LIVE-03`'s `evidence_notes` (do NOT change the
   `open_obligation` pointer; it stays
   `"blocked_until_RO-LIVE-04_and_RO-LIVE-05_close"` until
   RO-LIVE-05 also closes).
3. Run `/cluster-close` (regen the four grounding docs,
   commit, push).

## What this does NOT close

- RO-LIVE-03 (the wide rule). Stays open until RO-LIVE-05 also
  closes.
- RO-LIVE-05 (admission/agreement). Blocked on
  `PHASE4-N-M-LEDGER-SEED`.
- RO-LIVE-01, RO-LIVE-02, CN-CONS-06 — unchanged.
