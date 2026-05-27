# CN-CONS-06 / RO-LIVE-01 — Operator runbook

> **Cluster:** PHASE4-N-Q.
> **Purpose:** how to run `ade_node --mode produce` against a
> private cardano-node peer so the bounty-facing "cardano-node
> accepts an Ade-forged block" evidence can be captured.
>
> **Honest scope as of S6 close (HEAD TBD).** This runbook
> documents the end-to-end procedure, but the `ade_node
> --mode produce` binary shipped in PHASE4-N-Q S5 wires up the
> *composition surfaces* (CLI flags, slot loop, listener,
> evidence log) with a **stub forge handler** that always
> emits `ForgeNotLeader`. To capture the bounty artifact the
> operator additionally needs the real-forge wiring (composes
> `producer_shell.vrf_prove` + `producer_shell.kes_sign_at` +
> `scheduler_step` + `self_accept` + `served_snapshot.push`),
> which is tracked as the
> `blocked_until_real_forge_handler_lands` open_obligation on
> `CN-PROD-02` and as the surviving operator-action obligation
> on `CN-CONS-06` and `RO-LIVE-01`.
>
> Until that lands, this runbook is the **infrastructure
> recipe** — bring up a private testnet + pool + opcert +
> ade_node, prove the handshake completes, prove the slot loop
> ticks, capture an empty-block (no transactions) evidence
> trail. The bounty proof itself is gated on the follow-on
> forge cluster.

## §1 Topology

```
+--------------------+         +-------------------+
|  cardano-node      |   N2N   |  ade_node         |
|  (Conway preprod   |<------->|  --mode produce   |
|   docker, BP role) |  3001   |  --listen :3002   |
+--------------------+         +-------------------+
          |
          | N2C (socket)
          v
   cardano-cli
   (issue-op-cert,
    query tip,
    query stake-pool)
```

Two roles:

- **Haskell cardano-node** — the *peer* that Ade serves blocks
  to. Pre-existing local docker container
  `cardano-node-preprod` per [[reference-local-preprod-docker-cardano-node]];
  its bind mounts already carry a synced preprod chain.
- **ade_node** — the Ade block producer. Inbound listener on a
  free port (`:3002` in this runbook).

The bounty artifact: cardano-node (after running an Ade-forged
block through its own validation pipeline) emits a
`BlockAccepted` log line for an Ade-forged block hash that
matches `BlockForged.block_hash` from Ade's evidence log.

## §2 Prerequisites

### 2.1 Hermetic local stack

- Local docker `cardano-node-preprod` per the project's
  reference memory. `docker start cardano-node-preprod` brings
  it up; bind mounts under `.cardano-node-preprod/`.
- Ade source tree at `~/Code/rust/ade`.
- `cargo build -p ade_node` succeeds. Binary at
  `target/debug/ade_node`.
- `cardano-cli` available on `$PATH` (the docker image has it
  embedded; on host, the
  `cardano-node-preprod/scripts/cardano-cli.sh` wrapper
  proxies via N2C).

### 2.2 Cryptographic material

Generate a fresh **cold key**, **VRF key**, and **KES key**
locally — do not reuse mainnet keys.

```bash
# Cold key (Ed25519, cardano-cli text envelope)
cardano-cli node key-gen \
  --cold-verification-key-file cold.vkey \
  --cold-signing-key-file cold.skey \
  --operational-certificate-issue-counter-file cold.counter

# VRF key
cardano-cli node key-gen-VRF \
  --verification-key-file vrf.vkey \
  --signing-key-file vrf.skey

# KES key: use Ade-native key-gen (matches Haskell expand_seed)
ade_node --mode key_gen_kes --out-file kes.ade.skey --period-idx 0
```

> **Critical:** the Ade-native KES key uses the
> Haskell `cardano-base` `expand_seed` convention (PHASE4-N-P
> S4). KES seeds generated with cardano-crypto Rust ≤ 1.0.8 or
> with `ade_node --mode key_gen_kes` prior to HEAD `6973318`
> will derive a different VK and will be rejected by cardano-
> node's opcert validation. Confirm the VK fingerprint printed
> on stderr matches what your opcert claims.

### 2.3 Operational certificate

cardano-cli's `node issue-op-cert` requires the KES VK that
`ade_node` will use. Extract it:

```bash
# Print the KES VK hex (32 bytes) from the Ade-native KES seed.
ade_node --mode produce \
  --kes-skey kes.ade.skey \
  --cold-skey cold.skey \
  --vrf-skey vrf.skey \
  --opcert /dev/null \
  --genesis-file /dev/null \
  --evidence-log /dev/null \
  --max-slots 0 \
  --listen 127.0.0.1:0 2>&1 \
  | grep "ade.kes.seed.v1 loaded; KES VK fingerprint"
```

The fingerprint printed is the Blake2b-256 hash of the KES VK,
**not** the raw VK. To get the raw VK the operator currently
needs to call `ade_crypto::kes_sum::Sum6Kes::derive_verification_key`
in a small Rust scratch program — a `key-show-vk` CLI mode is
a follow-on (tracked under `OP-OPS-04`).

```bash
# Once the VK is in vk.cbor (cardano-cli text envelope shape):
cardano-cli node issue-op-cert \
  --kes-verification-key-file vk.cbor \
  --cold-signing-key-file cold.skey \
  --operational-certificate-issue-counter-file cold.counter \
  --kes-period 0 \
  --out-file node.opcert
```

### 2.4 Convert opcert to ade_node's simple JSON shape

S5 ships a minimal JSON parser (see `produce_mode::parse_simple_opcert_json`):

```json
{
  "hot_vkey_hex": "<32-byte KES VK as 64 hex chars>",
  "sequence_number": 0,
  "kes_period": 0,
  "sigma_hex": "<64-byte Ed25519 signature as 128 hex chars>"
}
```

Conversion from the cardano-cli `node.opcert` cborHex envelope
to this shape currently needs a small helper script (decode
the CBOR array `[hot_vkey, sequence_number, kes_period, sigma]`).
A first-class cardano-cli envelope parser is the follow-on
tracked under `CN-PROD-02.open_obligation`. The CBOR shape is
documented in `cardano-ledger-conway`'s
`shelleyma/ShelleyOperationalCert.hs`.

### 2.5 Minimal Conway genesis fixture

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

The full Conway genesis JSON (`conway-genesis.json` from
the cardano-node preprod artifacts) is **much** larger. S5's
`produce_mode::parse_simple_genesis_json` accepts only this
subset; a real-genesis parser is the follow-on tracked under
`CN-PROD-02.open_obligation`.

## §3 Operator procedure (the slot-loop smoke pass)

This is what works today against an S5-built ade_node.

```bash
# 1. Start the Haskell peer.
docker start cardano-node-preprod

# 2. Run ade_node in produce mode.
target/debug/ade_node \
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

Expected JSONL output (closed-vocabulary `ProducerLogEvent`):

```
{"kind":"CoordinatorStarted",...}
{"kind":"SlotTick","slot":0}
{"kind":"LeaderCheckOutcome","slot":0,"is_leader":false,...}
{"kind":"SlotTick","slot":1}
{"kind":"LeaderCheckOutcome","slot":1,"is_leader":false,...}
...
{"kind":"CoordinatorShutdown","reason":"ScheduleEnded",...}
```

What's proven by this run:

- Slot loop ticks deterministically (S5 smoke test).
- The GREEN coordinator never sees secret material
  (`ci/ci_check_producer_coordinator_no_secrets.sh` + the
  smoke test's "no seed bytes in log" assertion).
- The closed evidence vocabulary (`DC-PROD-01`) holds —
  smoke asserts no socket addresses in the log.
- The listener accepts and handshakes inbound peers in the
  loopback test (`crates/ade_runtime/src/network/n2n_listener.rs`
  unit tests).

What's **not** proven by this run (because the forge handler
is a stub):

- That an Ade-forged block is accepted by cardano-node.
- That `producer_shell.vrf_prove` + `producer_shell.kes_sign_at`
  + `scheduler_step` + `self_accept` compose correctly under
  real Conway-era parameters.
- That a real cardano-node `RequestRange` returns Ade-forged
  bytes the peer can validate.

These are the bounty artifacts. They are gated on:

1. Real forge handler implementation (replaces the
   `ForgeNotLeader`-only stub in
   `produce_mode::apply_effects_with_forge_handler`).
2. `ServedChainSnapshot::push` wiring so the listener's
   per-peer block-fetch reducer can serve the forged bytes.
3. A peer connection from the local docker
   cardano-node-preprod toward `127.0.0.1:3002`.

## §4 Evidence capture (when the forge handler lands)

After the real forge handler is in place, the operator pass
captures:

```bash
# Evidence transcript:
target/debug/ade_node --mode produce ... \
  --evidence-log docs/clusters/PHASE4-N-Q/CE-N-Q-LIVE_$(date +%Y%m%d).jsonl

# Cardano-node container log (filter for accepted blocks):
docker logs cardano-node-preprod 2>&1 \
  | grep -E "BlockAccepted|AddedToCurrentChain" \
  | grep "<ade-forged-block-hash>" \
  > docs/clusters/PHASE4-N-Q/CE-N-Q-LIVE-PEER_$(date +%Y%m%d).log
```

Pairing rule: a row in the Ade evidence file with
`kind: "BlockForged"` + `block_hash: <h>` must be matched by
a peer-log line containing `<h>` and one of
`BlockAccepted` / `AddedToCurrentChain`.

## §5 Honest deferrals

The following items are required for the bounty proof and are
**not** in this runbook's S5/S6 scope:

| Deferral | Tracked under | Cluster |
|---|---|---|
| Real forge handler composition | `CN-PROD-02.open_obligation` (`blocked_until_real_forge_handler_lands`) | follow-on |
| `ServedChainSnapshot::push` on forge | `RO-LIVE-01.open_obligation` (`blocked_until_real_forge_handler_lands` extended) | follow-on |
| Per-peer chain-sync/block-fetch frame dispatch from listener into `n2n_server::dispatch_*` | `CN-PROD-01.open_obligation` (`blocked_until_per_peer_dispatch_lands`) | follow-on |
| cardano-cli text-envelope opcert parser | `CN-PROD-02.open_obligation` (separate item) | follow-on |
| Full Conway genesis JSON parser | `CN-PROD-02.open_obligation` (separate item) | follow-on |
| Retirement / rewrite of `live_block_production_session.rs` | `CN-PROD-02.open_obligation` (separate item) | follow-on |

Each item is mechanically detectable (the stub `ForgeNotLeader`
emit, the no-op `BroadcastBlock` arm, the `_ => {}` listener
event swallow) and is named in source comments inside
`crates/ade_node/src/produce_mode.rs`. The runbook is honest:
S5/S6 closes the *infrastructure*, the follow-on cluster
closes the *bounty artifact*.

## §6 Replay-equivalence test (when forge handler lands)

The S2 coordinator replay test
(`replay_byte_identity_across_two_runs`) already proves that
for a fixed canonical slot-tick + forge-result stream, the
coordinator emits byte-identical evidence. The operator pass
SHOULD additionally capture two transcripts from two runs of
`ade_node --mode produce` with the same canonical inputs
(same keys, same opcert, same genesis, same slot range)
against a quiescent cardano-node peer; both transcripts
should differ ONLY in RED operational metadata (timestamps,
socket order) — `ProducerLogEvent` lines should be byte-
identical when filtered to the replayable vocabulary.

This is the `DC-PROD-02` exit criterion. Until the forge
handler is real, the smoke-test bound (`--max-slots N`) is
the only replay anchor — and that anchor is already covered
by the unit test in S2.

## §7 References

- Cluster: [`docs/clusters/PHASE4-N-Q/cluster.md`](../clusters/PHASE4-N-Q/cluster.md).
- Slice docs: [`S1.md`](../clusters/PHASE4-N-Q/S1.md),
  [`S2.md`](../clusters/PHASE4-N-Q/S2.md),
  [`S3.md`](../clusters/PHASE4-N-Q/S3.md),
  [`S4.md`](../clusters/PHASE4-N-Q/S4.md),
  [`S5.md`](../clusters/PHASE4-N-Q/S5.md),
  [`S6.md`](../clusters/PHASE4-N-Q/S6.md).
- Operator procedure: [`CE-N-Q-OPERATOR_PROCEDURE.md`](../clusters/PHASE4-N-Q/CE-N-Q-OPERATOR_PROCEDURE.md).
- Predecessor runbook: `docs/clusters/PHASE4-N-C/CE-N-C-8_PROCEDURE.md`
  (the structural cross-impl half of CN-CONS-06).
- Predecessor runbook: `docs/clusters/completed/PHASE4-N-G/CE-N-G-8_PROCEDURE.md`
  (the server-side block-fetch half of RO-LIVE-01).
- Doctrine: [[feedback-bounded-smoke-slices]],
  [[feedback-shell-must-not-overstate-semantic-truth]],
  [[project-bounty-requirements]].
