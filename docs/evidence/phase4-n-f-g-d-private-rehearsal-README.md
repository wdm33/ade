# PHASE4-N-F-G-D — C1 private-testnet accepted-block **dry-run** runbook

> **Status:** operator runbook (release/evidence scope, NOT runtime authority).
> A **strict subset/adaptation** of the preprod operator-pass runbook
> `docs/evidence/phase4-n-f-g-c-operator-pass-README.md` — **same accepted-block
> path, different venue + evidence label.** This is a bounty **dry-run** (a fast
> failure detector for the exact preprod path), **not** the bounty deliverable.
>
> **Hard line — read first.** Ade self-accept **≠** peer acceptance. A served
> block **≠** peer acceptance. Wire success **≠** peer acceptance. **Private C1
> acceptance ≠ bounty completion** (the bounty deliverable is the separate
> operator-witnessed **C2 preprod** pass). The ONLY thing this run produces is a
> **clearly-labeled, non-promotable** `PrivateRehearsalManifest`
> (`is_rehearsal = true`, `not_bounty_evidence = true`) under the rehearsal home
> `docs/evidence/phase4-n-f-g-d-private-rehearsal-*.toml`, produced ONLY by a real
> operator-captured Haskell peer log through `ba02_evidence::correlate`. This
> runbook flips **no** RO-LIVE rule.

## What this proves (and what it does not)

It answers **one** question fast: *will a real Haskell node accept an Ade-forged
block when Ade has legitimate leader rights?* — surfacing the bounty-blocking
failure classes (opcert/KES, Praos leader proof, header fields, block body/wire
encoding, tag-24 BlockFetch payload, serve path, peer-log parsing) on a private
net where the operator controls stake and Ade wins slots in minutes. It does
**not** satisfy the bounty (which grades preview/preprod). After a passing C1
dry-run the next step is **register/stake on preprod and run the same path**, not
more private-net work.

## Why the path cannot drift (the bounty-alignment guarantee)

The C1 dry-run uses the **identical** `--mode node` accepted-block path as the
preprod pass — `import_live_consensus_inputs` → forge → self-accept →
sibling-serve → block-fetch → peer log → `correlate`. This is **mechanically
fenced** by `ci/ci_check_node_path_fidelity.sh` (PHASE4-N-F-G-D S1): **no
private-only flag, no from-genesis consensus-inputs constructor.** A step below
that tried to diverge would be rejected by the binary (unknown flag) or is simply
unrepresentable (no such constructor exists). **Fast slots come from the
operator-authored private genesis stake allocation (operator input), never from
Ade code.**

## 0. Venue (C1 only)

- **C1 — private testnet:** the operator authors a Shelley+Conway genesis where
  Ade's pool holds ~all active stake, so Ade is a near-every-slot leader and a
  **follower** Haskell peer validates+accepts its block. The peer must be a
  **follower, NOT a co-producer** (see §3).

There is no C2 arm here — preprod is the bounty surface, captured by the separate
G-C runbook. (The single-epoch limit applies: stay inside the recovered seed
epoch; `DC-EPOCH-03` fails closed at the boundary.)

## 1. Pre-flight — genesis-consistency pin (BEFORE any live KES signature)

The `--mode node` forge leadership source is the recovered consensus-inputs
bundle, **extracted via the same `import_live_consensus_inputs` path preprod
uses** (`cardano-cli query protocol-state` for eta0, `query stake-snapshot` for
stake, `query tip`) — **not** a from-genesis constructor. Before any live KES
signature, pin Ade's recovered values + leader-eligibility inputs against the
genesis-derived reference for the recovered seed epoch:

```
cargo test -p ade_testkit --test <genesis_pinning harness>   # ade_testkit::consensus::genesis_pinning
```

**Necessary but NOT sufficient for acceptance.** A genesis-consistent recovered
state is required for the peer to even consider the block, but acceptance is
still proven ONLY by the operator-captured peer log through `correlate` (§4). For
a from-genesis/early private net the operator **pre-seeds the store** (reusing the
N-M-C `seed_to_snapshot` extraction) so the node takes the **WarmStart** arm —
node `FirstRun` is Mithril-only; there is no second bootstrap path
(`CN-NODE-01` intact).

## 2. Launch the forge-capable `--mode node` (the SAME flags as preprod)

```
ade_node --mode node \
  --peer <FOLLOWER_PEER_ADDR> \              # the live WirePump feed (REQUIRED for a live run)
  --network-magic <C1_MAGIC> \               # REQUIRED on the live (--peer) path
  --json-seed <utxo.json> \                  # MANDATORY (operator-seeded ledger)
  --consensus-inputs-path <bundle> \         # MANDATORY (import_live_consensus_inputs — SAME path as preprod)
  --cold-skey <node.skey> \                  # forge intent: the COMPLETE operator set or none
  --kes-skey <kes.skey> \
  --vrf-skey <vrf.skey> \
  --opcert <node.opcert> \
  --genesis-file <shelley-genesis.json> \
  --wal-dir <wal/> --snapshot-dir <snap/> \  # durable state (REQUIRED)
  --genesis-hash <64-hex> --genesis <config>
```

**No private-only flag exists** — this is byte-for-byte the preprod invocation
modulo operator inputs (the private genesis/stake/keys). Forge intent is the
COMPLETE key set or none — a partial set fails closed
(`EXIT_NODE_FORGE_KEY_INGRESS_FAILED = 44`), never a silent relay fallback. With
`--peer` supplied, the On arm dials the upstream and feeds a live
`NodeBlockSource::WirePump`, so `LoopStep::ForgeTick` is reachable; if `--peer`
is absent/unreachable the run is NOT live-feed evidence.

## 3. The peer is a follower, NOT a co-producer

The validating Haskell `cardano-node` (whose log you capture) must run with **no
forging credentials** — no KES/VRF/opcert. Its job is purely to
validate+accept+serve the chain. Exactly **one producer per pool key** (Ade); a
co-producing peer would fork/double-forge and muddy the acceptance signal.

## 4. Capture the peer's validation log → `correlate` → rehearsal manifest

When the peer validates and adopts/serves the Ade-forged block, capture the
peer's evidence and normalize it to the closed JSONL the allow-list parser
accepts (`ba02_evidence::parse_peer_accept_events`):

- `{"event":"peer_served_block","block_hash_hex":"<64-hex>","slot":<N>,"peer":"<addr>"}` — STRONGEST.
- `{"event":"peer_chain_tip","block_hash_hex":"<64-hex>","slot":<N>,"peer":"<addr>"}` — corroborating.

Every other line (any Ade-internal `self_accept` / `forge_succeeded` /
`block_received` signal) is **dropped** by the allow-list; it can never be coerced
into acceptance.

Compute the committed peer log's sha256 (the envelope binding the rehearsal gate
re-verifies), then run the env-gated operator execution harness (a RED test,
skipped in CI; **not** a runtime node mode). It writes the rehearsal manifest
ONLY on a real `correlate`-produced match:

```
ADE_LIVE_C1_DRY_RUN=1 \
ADE_LIVE_FORGED_BLOCK_HASH=<64-hex of the Ade-forged block> \
ADE_LIVE_FORGED_SLOT=<N> \
ADE_LIVE_NETWORK_MAGIC=<C1_MAGIC> \
ADE_LIVE_PEER_LOG=<captured-peer.log> \
ADE_LIVE_REHEARSAL_PEER_LOG_SHA256=$(sha256sum <captured-peer.log> | awk '{print $1}') \
ADE_LIVE_REHEARSAL_MANIFEST_OUT=docs/evidence/phase4-n-f-g-d-private-rehearsal-<run>.toml \
  cargo test -p ade_node --test node_c1_dry_run_rehearsal node_c1_dry_run_rehearsal_live -- --nocapture
```

A `NoEvidence` outcome **panics and writes nothing** — the peer did not accept;
re-check stake / leadership / genesis-consistency. Ade self-accept is NOT
acceptance.

## 5. Assemble + commit the rehearsal evidence (gated by `ci_check_rehearsal_manifest_schema.sh`)

Commit the operator-captured peer log + the `correlate`-produced rehearsal
manifest under the **rehearsal home** `docs/evidence/`:

```toml
schema_version = 1                              # == REHEARSAL_MANIFEST_SCHEMA_VERSION
venue = "private-testnet-c1"
is_rehearsal = true
not_bounty_evidence = true
peer_log_file = "phase4-n-f-g-d-private-rehearsal-<run>-peer.log"   # relative to this manifest
peer_log_file_sha256 = "<sha256 of the peer log>"
forged_block_hash_hex = "<64-hex>"
slot = <N>
network_magic = <C1_MAGIC>
peer_accept_source = "served_block"             # served_block | chain_tip | served_block_and_chain_tip
peer = "<addr>"
matched_block_hash_hex = "<64-hex>"
```

The gate is **vacuous until a manifest is committed**; once committed it verifies
the closed schema + the rehearsal markers + that `peer_log_file_sha256` matches
the committed peer log (the "no synthetic manifest" line), and that no rehearsal
marker appears under the bounty home. The manifest is correlate-produced by
construction (`rehearsal_pass::write_private_rehearsal_manifest` accepts only a
`PrivateRehearsalManifest`, which only `from_correlate_outcome`'s `Ba02Manifest`
arm builds — `NoEvidence` writes nothing).

## 6. Delta from the G-C preprod operator-pass runbook (the ONLY differences)

| Aspect | G-C preprod pass | G-D C1 dry-run |
|---|---|---|
| `--mode node` path + flags | same | **same** (S1-fenced) |
| `--peer` live feed | same | **same** |
| `--json-seed` / `--consensus-inputs-path` (`import_live_consensus_inputs`) | same | **same** |
| operator key/opcert flow | same | **same** |
| peer-log capture → `correlate` | same | **same** |
| `NoEvidence` fail-closed | same | **same** |
| venue | preprod (provisioned stake) | **private-testnet-c1** (operator genesis stake → fast slots) |
| evidence envelope | `Ba02Manifest` (bounty home `docs/clusters/PHASE4-N-F-G-C/CE-G-C-LIVE_*.toml`) | **`PrivateRehearsalManifest`** (`docs/evidence/phase4-n-f-g-d-private-rehearsal-*.toml`, `not_bounty_evidence = true`) |
| RO-LIVE flip | the bounty deliverable | **none** |

## 7. What this does NOT do

- It does **not** flip `RO-LIVE-01` or `RO-LIVE-06` — both stay
  partial/operator-gated; the bounty deliverable is the separate C2 preprod pass.
- It commits **no synthetic manifest**; a hermetic `correlate` fixture is
  reducer/mechanics only and never lives under the rehearsal home.
- It makes **no** acceptance claim from any Ade-internal signal.
- It is **not** the bounty deliverable; a passing C1 dry-run only **increases
  confidence** in the bounty path it is a strict subset of.
