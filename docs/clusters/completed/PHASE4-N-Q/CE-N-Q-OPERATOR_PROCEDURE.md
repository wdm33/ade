# CE-N-Q — Operator procedure (live ade_node produce-mode pass)

> Companion to [`../../active/cn-cons-06-operator-runbook.md`](../../active/cn-cons-06-operator-runbook.md).
> This file is the step-list pinned to the cluster so the
> sequence is reproducible from cluster-local context.

## A. Hermetic stack-up

```bash
cd ~/Code/rust/ade
docker start cardano-node-preprod
cargo build -p ade_node
```

Wait for the docker container to reach "ready" — see
[[reference-local-preprod-docker-cardano-node]] §3 for the
ready-signal recipe. Until ready, the peer will reject
inbound handshakes.

## B. Key + opcert provisioning (one-time)

```bash
mkdir -p .runbook-tmp && cd .runbook-tmp

cardano-cli node key-gen \
  --cold-verification-key-file cold.vkey \
  --cold-signing-key-file cold.skey \
  --operational-certificate-issue-counter-file cold.counter

cardano-cli node key-gen-VRF \
  --verification-key-file vrf.vkey \
  --signing-key-file vrf.skey

../target/debug/ade_node \
  --mode key_gen_kes \
  --out-file kes.ade.skey \
  --period-idx 0
```

> The KES VK fingerprint is printed on stderr. Pin it; you
> will need to recompute it after every period rotation.

Issue the opcert using the KES VK extracted from
`kes.ade.skey`. The current runbook (S6 honest scope)
requires a small helper to emit the raw VK bytes; the
follow-on cluster ships `--mode key-show-vk`.

After issuing `node.opcert`, convert to the simple JSON
shape `produce_mode::parse_simple_opcert_json` expects:

```json
{
  "hot_vkey_hex": "<64 hex chars>",
  "sequence_number": 0,
  "kes_period": 0,
  "sigma_hex": "<128 hex chars>"
}
```

Save as `node.opcert.simple.json`.

## C. Genesis subset

```json
{
  "network_magic": 1,
  "slot_zero_time_unix_ms": 1666656000000,
  "slot_length_ms": 1000,
  "slots_per_kes_period": 129600,
  "kes_anchor_slot": 0,
  "kes_max_period": 63
}
```

Save as `conway-genesis-min.json`. The values above are the
preprod defaults; mainnet differs.

## D. Smoke pass (S5-era; no real forge yet)

```bash
../target/debug/ade_node \
  --mode produce \
  --listen 127.0.0.1:3002 \
  --cold-skey cold.skey \
  --kes-skey kes.ade.skey \
  --vrf-skey vrf.skey \
  --opcert node.opcert.simple.json \
  --genesis-file conway-genesis-min.json \
  --evidence-log evidence.jsonl \
  --max-slots 10
```

Expected: exit 0; `evidence.jsonl` has 1
`CoordinatorStarted`, 10 `SlotTick`, 10 `LeaderCheckOutcome`
(all `is_leader: false` against the synthetic VRF), 1
`CoordinatorShutdown { reason: "ScheduleEnded" }`.

If the listener was contacted by a peer during the 10 slots,
you'll additionally see `PeerConnected { chain_sync_version,
block_fetch_version }` and `PeerDisconnect` lines.

## E. Pair-check against peer (when the real forge handler
lands)

```bash
# Tail the cardano-node container for accepted blocks.
docker logs --follow cardano-node-preprod 2>&1 \
  | tee peer.log \
  | grep -E "BlockAccepted|AddedToCurrentChain"
```

Then in another shell run the same `ade_node --mode produce`
without `--max-slots` until the slot loop has issued enough
`BlockForged` events that the peer's log emits at least one
`BlockAccepted` whose hash matches a `BlockForged.block_hash`
from `evidence.jsonl`.

Commit the artifacts:

```bash
cp evidence.jsonl ../docs/clusters/PHASE4-N-Q/CE-N-Q-LIVE_$(date +%Y%m%d).jsonl
cp peer.log       ../docs/clusters/PHASE4-N-Q/CE-N-Q-LIVE-PEER_$(date +%Y%m%d).log
```

Append the file paths to the `evidence` field of the
`RO-LIVE-01` registry entry; flip its status to `enforced`.

## F. Replay equivalence (S2 anchor)

The S2 coordinator replay test
(`crates/ade_runtime/tests/producer_coordinator_replay.rs`)
already covers the mechanical claim of `DC-PROD-02` against a
fixed canonical event corpus. The operator-pass equivalent —
two `ade_node --mode produce` runs against the same canonical
inputs (same keys, same opcert, same genesis, same slot
range, same peer state) producing byte-identical
`ProducerLogEvent` sequences when filtered to the replayable
vocabulary — is the *additional* live-system anchor. Capture
this only after the forge handler is real; until then S2's
mechanical anchor is the load-bearing proof.

## G. Honest deferrals

See [`../../active/cn-cons-06-operator-runbook.md`](../../active/cn-cons-06-operator-runbook.md) §5.
The bounty artifact requires the follow-on forge cluster.
