# PHASE4-N-F-G-J — C1 private-testnet **genesis-successor** rehearsal runbook

> **Status:** operator runbook (release/evidence scope, NOT runtime authority).
> A **strict adaptation** of the G-D dry-run runbook
> `docs/evidence/phase4-n-f-g-d-private-rehearsal-README.md` — **same accepted-block
> path + same `correlate` evidence flow, one scenario delta: cold start.**
>
> **Hard line — read first.** Ade self-accept **≠** peer acceptance. A served
> block **≠** peer acceptance. Wire success **≠** peer acceptance. **Private C1
> acceptance ≠ bounty completion.** The ONLY thing this run produces is a
> **clearly-labeled, non-promotable** `PrivateRehearsalManifest`
> (`is_rehearsal = true`, `not_bounty_evidence = true`) under the genesis-rehearsal
> home `docs/evidence/phase4-n-f-g-j-genesis-rehearsal-*.toml`, produced ONLY by a
> real operator-captured Haskell follower log through `ba02_evidence::correlate`.
> This runbook flips **no** RO-LIVE rule; the live run is
> `blocked_until_operator_c1_genesis_successor_rehearsal`.

## What this proves (and what it does not)

It answers **one** question fast: *will a real Haskell follower validate/fetch an
Ade-forged **genesis-successor** block — block 0 whose `prev_hash` is CBOR `null`
(`PrevHash::Genesis`) — when Ade has legitimate leader rights at cold start?* This
is the headline G-J compatibility question (S2 wire grammar, S3 position rule +
forge, S4 node-spine reachability). It surfaces the genesis-specific failure
classes (null-prev header encoding, KES pre-image over the null-prev body, the
follower's `$hash32 / null` decode) on a private net where the operator controls
stake. It does **not** satisfy the bounty (which grades preview/preprod), and it
makes **no** acceptance claim from any Ade-internal signal.

## Why the path cannot drift (the fidelity guarantee)

The C1 genesis rehearsal uses the **identical** `--mode node` cold-start path:
recovered-lineage WarmStart → `LoopStep::ForgeTick` with both tips `None` →
`forge_one_from_recovered(None)` → block 0 + `PrevHash::Genesis` → `self_accept`
→ sibling serve → follower log → `correlate`. This is **mechanically fenced** by
`ci/ci_check_node_path_fidelity.sh` (**no private-only flag, no from-genesis
consensus-inputs constructor**) and `ci/ci_check_genesis_successor_reachability.sh`
(the cold-start convention + gating). Fast slots come from the operator-authored
private genesis stake allocation (operator input), never from Ade code.

## 0. Venue (C1 only)

- **C1 — private testnet:** the operator authors a Shelley+Conway genesis where
  Ade's pool holds ~all active stake, so Ade is a near-every-slot leader and a
  **follower** Haskell peer validates+accepts its block. The peer is a
  **follower, NOT a co-producer** (no KES/VRF/opcert) — exactly one producer per
  pool key (Ade). The single-epoch limit applies (`DC-EPOCH-03` fails closed at
  the boundary). There is no C2 arm here — preprod is the bounty surface.

## 1. Cold-start pre-seed (the genesis-successor delta from G-D)

The G-J scenario is **cold start**: pre-seed the C1 store with the recovered
seed-epoch lineage but **no persisted block tip** (a recovered-lineage WarmStart
with no tip), so the node's first `ForgeTick` finds `ChainDb::tip()` **and**
`recovered.tip` both `None` and forges the **genesis-successor** (block 0 +
`Genesis`) per S4.

- Pre-seed the lineage via the same `seed_to_snapshot` extraction the node's
  WarmStart consumes (reusing the N-M-C path) — **no second bootstrap**
  (`CN-NODE-01`; node `FirstRun` stays Mithril-only). The recovered
  `SeedEpochConsensusInputs` must be present (the cold-start gate
  `may_cold_start_forge` requires it), with **no persisted tip**.
- Pin the recovered values against the genesis-derived reference for the seed
  epoch (necessary, not sufficient — acceptance is proven only by §4):

```
cargo test -p ade_testkit --test <genesis_pinning harness>
```

## 2. Launch the forge-capable `--mode node` (the SAME flags as G-D/preprod)

```
ade_node --mode node \
  --peer <FOLLOWER_PEER_ADDR> \              # the live WirePump feed (REQUIRED for a live run)
  --network-magic <C1_MAGIC> \
  --json-seed <utxo.json> \                  # operator-seeded ledger
  --consensus-inputs-path <bundle> \         # import_live_consensus_inputs — SAME path as preprod
  --cold-skey <node.skey> --kes-skey <kes.skey> --vrf-skey <vrf.skey> --opcert <node.opcert> \
  --genesis-file <shelley-genesis.json> \
  --wal-dir <wal/> --snapshot-dir <snap/> \  # the cold-start store: lineage present, NO tip
  --genesis-hash <64-hex> --genesis <config>
```

**No genesis-only flag exists** — this is byte-for-byte the G-D/preprod invocation
modulo operator inputs (the private genesis/stake/keys) + the no-tip store.
Forge intent is the COMPLETE key set or none (a partial set fails closed,
`EXIT_NODE_FORGE_KEY_INGRESS_FAILED = 44`). With `--peer` supplied, the On arm
feeds a live `NodeBlockSource::WirePump`, the feed is forge-eligible
(`no_block_available | clean_empty`), and `LoopStep::ForgeTick` reaches the
cold-start genesis forge.

## 3. The peer is a follower, NOT a co-producer

The validating Haskell `cardano-node` (whose log you capture) runs with **no
forging credentials**. Its job is purely to validate+accept+serve. A co-producing
peer would fork and muddy the acceptance signal.

## 4. Capture the follower log → `correlate` → rehearsal manifest

When the follower validates and adopts/serves the Ade-forged **genesis-successor**
block, capture its evidence as the closed JSONL the allow-list parser accepts
(`ba02_evidence::parse_peer_accept_events`):

- `{"event":"peer_served_block","block_hash_hex":"<64-hex>","slot":<N>,"peer":"<addr>"}` — STRONGEST.
- `{"event":"peer_chain_tip","block_hash_hex":"<64-hex>","slot":<N>,"peer":"<addr>"}` — corroborating.

Every other line (any Ade-internal `self_accept` / `forge_succeeded` /
`block_received` signal) is **dropped** by the allow-list. Compute the committed
log's sha256, then run the env-gated operator harness (skipped in CI; NOT a node
mode). It writes the manifest ONLY on a real `correlate`-produced match:

```
ADE_LIVE_C1_GENESIS_REHEARSAL=1 \
ADE_LIVE_FORGED_BLOCK_HASH=<64-hex of the Ade-forged genesis block> \
ADE_LIVE_FORGED_SLOT=<N> \
ADE_LIVE_NETWORK_MAGIC=<C1_MAGIC> \
ADE_LIVE_PEER_LOG=<captured-peer.log> \
ADE_LIVE_REHEARSAL_PEER_LOG_SHA256=$(sha256sum <captured-peer.log> | awk '{print $1}') \
ADE_LIVE_REHEARSAL_MANIFEST_OUT=docs/evidence/phase4-n-f-g-j-genesis-rehearsal-<run>.toml \
  cargo test -p ade_node --test node_c1_genesis_rehearsal node_c1_genesis_rehearsal_live -- --nocapture
```

A `NoEvidence` outcome **panics and writes nothing** — the follower did not accept
the null-prev block; re-check stake / leadership / genesis-consistency / the
null-prev wire encoding. Ade self-accept is NOT acceptance.

## 5. Commit the evidence (gated by `ci_check_rehearsal_manifest_schema.sh`)

Commit the captured follower log + the `correlate`-produced manifest under the
genesis-rehearsal home `docs/evidence/phase4-n-f-g-j-genesis-rehearsal-*`. The
gate verifies the closed schema + the rehearsal markers + the peer-log sha256
binding, and forbids any rehearsal marker under the bounty home.

## 6. Delta from the G-D dry-run runbook (the ONLY difference)

| Aspect | G-D dry-run | G-J genesis rehearsal |
|---|---|---|
| `--mode node` path + flags + `--peer` + `import_live_consensus_inputs` | same | **same** (fidelity-fenced) |
| peer-log capture → `correlate` → non-promotable manifest | same | **same** |
| `NoEvidence` fail-closed / no RO-LIVE flip | same | **same** |
| **scenario** | forge atop an existing tip (block N + `Block`) | **cold start: no tip → block 0 + `Genesis` (null prev)** |
| evidence home | `phase4-n-f-g-d-private-rehearsal-*.toml` | **`phase4-n-f-g-j-genesis-rehearsal-*.toml`** |
| live env gate | `ADE_LIVE_C1_DRY_RUN` | **`ADE_LIVE_C1_GENESIS_REHEARSAL`** |

## 7. What this does NOT do

- It does **not** flip `RO-LIVE-01`/`RO-LIVE-06` — both stay partial/operator-gated.
- It commits **no synthetic manifest**; a hermetic `correlate` fixture is
  mechanics-only (`forge_succeeds.rs::genesis_rehearsal_manifest_binds_block_zero_genesis`)
  and never lives under the rehearsal home.
- It makes **no** acceptance claim from any Ade-internal signal; only the
  allow-listed follower-log events through `correlate`.
- It is **not** the bounty deliverable, **not** durable progression to block 1+,
  **not** broad live sync; a passing C1 genesis rehearsal only **increases
  confidence** in the null-prev genesis-successor path it exercises.
