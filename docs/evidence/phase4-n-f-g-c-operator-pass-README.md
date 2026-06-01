# PHASE4-N-F-G-C — `--mode node` operator-pass runbook (BA-02 live evidence)

> **Status:** operator runbook (release/evidence scope, NOT runtime authority).
> Supersedes the stale `docs/clusters/completed/PHASE4-N-S-C/S1.md` for the
> `--mode node` live leg. Closes the C1-scoping-doc bugs
> (`docs/planning/operator-pass-live-leg-c1-scoping.md`).
>
> **Hard line — read first.** Ade self-accept **≠** peer acceptance. A served
> block **≠** peer acceptance. Wire success **≠** peer acceptance. The ONLY
> thing that produces BA-02 evidence is a real operator-captured **peer
> validation log** naming the EXACT Ade-forged block hash, run through
> `ba02_evidence::correlate` (the sole `Ba02Manifest` constructor). This
> runbook produces **no** acceptance claim by itself; the live ACCEPT is
> `blocked_until_operator_stake_available` (RO-LIVE-01 / RO-LIVE-06).

## 0. Venue (OQ4)

- **C1 — private testnet (cheapest real ACCEPT):** Ade holds ~all stake, so it
  is a legitimate slot leader and a follower peer can validate+accept its
  block. The peer must be a **follower, NOT a co-producer** (see §3).
- **C2 — preprod (graded surface):** needs ~2 epochs of provisioned, active
  stake before Ade is ever a leader. The public preprod docker peer
  (`cardano-node-preprod`) **cannot** grant acceptance (Ade has no stake there)
  — a run against it validates **wiring only**, never acceptance.

Either way the live ACCEPT is `blocked_until_operator_stake_available`.

## 1. Pre-flight — genesis-consistency pin (OQ5, BEFORE any live KES signature)

The `--mode node` forge leadership source is the recovered
`SeedEpochConsensusInputs`. Before any live KES signature, pin Ade's recovered
values + leader-eligibility inputs against the genesis-derived reference for
the recovered seed epoch:

```
cargo test -p ade_testkit --test <genesis_pinning harness>   # ade_testkit::consensus::genesis_pinning
```

**Necessary but NOT sufficient for acceptance.** A genesis-consistent recovered
state is required for the peer to even consider the block, but acceptance is
still proven ONLY by the operator-captured peer log through `correlate` (§4).
For a from-genesis/tip private net the operator **pre-seeds the store** (reusing
the N-M-C `seed_to_snapshot` extraction) so the node takes the **WarmStart** arm
— node `FirstRun` is Mithril-only; there is no second bootstrap path
(CN-NODE-01 intact).

## 2. Launch the forge-capable `--mode node` (corrected flags)

```
ade_node --mode node \
  --peer <UPSTREAM_PEER_ADDR> \              # PHASE4-N-F-G-C S1: the live WirePump feed (REQUIRED for a live run)
  --network-magic <MAGIC> \                  # REQUIRED on the live (--peer) path
  --json-seed <utxo.json> \                  # MANDATORY (was missing from S1.md — C1 doc G5)
  --consensus-inputs-path <bundle> \         # MANDATORY (was missing from S1.md — C1 doc G5)
  --cold-skey <node.skey> \                  # forge intent: the COMPLETE operator set or none
  --kes-skey <kes.skey> \
  --vrf-skey <vrf.skey> \
  --opcert <node.opcert> \
  --genesis-file <shelley-genesis.json> \
  --wal-dir <wal/> --snapshot-dir <snap/> \  # durable state (REQUIRED)
  --genesis-hash <64-hex> --genesis <config>
```

- **Live feed (S1):** with `--peer` supplied, the On arm dials the upstream and
  feeds a live `NodeBlockSource::WirePump`, so `LoopStep::ForgeTick` is
  reachable. **If `--peer` is absent or unreachable, the run is NOT live-feed
  evidence** (the empty source halts before any `ForgeTick`); the run is only a
  valid live-feed pass when `--peer` is supplied AND the feed reaches
  `Continuing`. Record this condition in the manifest (§5).
- **Forge intent** is the COMPLETE key set or none — a partial set fails closed
  (`EXIT_NODE_FORGE_KEY_INGRESS_FAILED = 44`), never a silent relay fallback.

## 3. The peer is a follower, NOT a co-producer (S1.md fix)

The validating peer (the `cardano-node` whose log you capture) must run with **no
forging credentials** — no KES/VRF/opcert. Its job is purely to
validate+accept+serve the chain. A co-producing peer would fork/compete and
muddy the acceptance signal. (The stale S1.md recipe ran the peer as a producer
— this is the corrected instruction.)

## 4. Capture the peer's validation log → JSONL → `correlate`

When the peer validates and adopts/serves the Ade-forged block, capture the
peer's evidence and normalize it to the closed JSONL the allow-list parser
accepts (`ba02_evidence::parse_peer_accept_events`):

- `{"event":"peer_served_block","block_hash_hex":"<64-hex>","slot":<N>,"peer":"<addr>"}` — STRONGEST (peer serves the forged block on its own chain-serving path).
- `{"event":"peer_chain_tip","block_hash_hex":"<64-hex>","slot":<N>,"peer":"<addr>"}` — corroborating (peer's tip names the forged hash).

Every other line (a weaker/unknown signal — including any Ade-internal
`self_accept` / `forge_succeeded` / `block_received` line) is **dropped** by the
allow-list; it can never be coerced into acceptance.

Run the wiring (env-gated; writes the manifest ONLY on a real
`correlate`-produced `Ba02Manifest`):

```
ADE_LIVE_OPERATOR_TEST=1 \
ADE_LIVE_FORGED_BLOCK_HASH=<64-hex of the Ade-forged block> \
ADE_LIVE_FORGED_SLOT=<N> \
ADE_LIVE_NETWORK_MAGIC=<MAGIC> \
ADE_LIVE_PEER_LOG=<captured-peer.log> \
ADE_LIVE_BA02_MANIFEST_OUT=<ade-evidence.json> \
  cargo test -p ade_node --test node_operator_pass_ba02 node_operator_pass_ba02_live -- --nocapture
```

A `NoEvidence` outcome **panics and writes nothing** — the peer did not accept;
re-check stake / leadership / genesis-consistency. Ade self-accept is NOT
acceptance.

## 5. Assemble + commit the evidence manifest (gated by `ci_check_ba02_evidence_manifest_schema.sh`)

Commit the operator-captured peer log + the `correlate`-produced
`ade-evidence.json` under `docs/clusters/PHASE4-N-F-G-C/`, plus a manifest
`CE-G-C-LIVE_<run>.toml` binding them by sha256:

```toml
schema_version = 1                              # == BA02_MANIFEST_SCHEMA_VERSION
block_hash = "<64-hex>"                          # the Ade-forged block hash
slot = <N>
accept_event_kind = "served_block"               # served_block | chain_tip | served_block_and_chain_tip
peer_log_file = "CE-G-C-LIVE_<run>-peer.log"     # relative to this manifest
peer_log_file_sha256 = "<sha256 of the peer log>"
peer_log_capture_command = "<the exact capture command>"
peer_log_filter = "<the exact normalization filter>"
# Plus, for the record: ade_evidence_file = "CE-G-C-LIVE_<run>-ade-evidence.json",
# ade_commit, cardano_node_version, network, and live_feed_exercised = true
# (--peer supplied + feed reached Continuing — see §2).
```

The gate is **vacuous until a manifest is committed**; once committed it
verifies the schema + that `peer_log_file_sha256` matches the committed peer
log (the "no synthetic manifest" line — a hand-authored manifest with no real,
matching fixture FAILS). The `Ba02Manifest` itself is correlate-produced by
construction (`ba02_pass::write_ba02_manifest` accepts only a `Ba02Manifest`,
which only `correlate`'s exact-match arm builds).

## 6. What this does NOT do

- It does **not** flip `RO-LIVE-01` or `RO-LIVE-06` — both stay
  partial/operator-gated until a real operator-witnessed pass produces
  peer-log evidence and a registry review records it at a future close.
- It commits **no synthetic manifest**; a hermetic correlate fixture is
  reducer/mechanics only and never lives under this manifest home.
- It makes **no** acceptance claim from any Ade-internal signal.
