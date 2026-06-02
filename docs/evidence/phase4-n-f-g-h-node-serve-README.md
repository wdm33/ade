# PHASE4-N-F-G-H — C1 node-spine **serve-to-peer** dry-run runbook

> **Status:** operator runbook (release/evidence scope, NOT runtime authority).
> A **strict adaptation** of the preprod/private operator-pass runbooks
> `docs/evidence/phase4-n-f-g-c-operator-pass-README.md` +
> `docs/evidence/phase4-n-f-g-d-private-rehearsal-README.md` — **same
> accepted-block path; the only addition is the downstream-follower topology**
> (a real Haskell `cardano-node` dials Ade's `--listen` and fetches Ade's served
> block). This is a bounty **dry-run** of the serve direction, **not** the bounty
> deliverable.
>
> **Hard line — read first.** Ade self-accept **≠** peer acceptance. A served
> block **≠** peer acceptance. Wire success **≠** peer acceptance. The follower is
> **expected to validate/accept** the block **if it is protocol-valid**, but the
> **only accepted/served claim may come from the follower's log through
> `ba02_evidence::correlate`** — running this harness does not itself prove
> acceptance. **Private C1 acceptance ≠ bounty completion** (the bounty
> deliverable is the separate operator-witnessed **C2 preprod** pass). This run
> produces only a **clearly-labeled, non-promotable** `PrivateRehearsalManifest`
> (`is_rehearsal = true`, `not_bounty_evidence = true`) under the rehearsal home,
> produced ONLY by a real operator-captured Haskell-follower log through
> `correlate`. This runbook flips **no** RO-LIVE rule.

## What this proves (and what it does not)

It answers **one** serve-direction question on the now-wired, magic-aware
node-spine serve (S2 + S2b): *can a real Haskell follower **connect to Ade's
`--listen`**, ChainSync-discover Ade's served forged block, and **BlockFetch its
body**?* — and, if the block is protocol-valid, the follower is expected to adopt
it. It surfaces the serve-direction failure classes (the follower's handshake
against Ade's magic-aware serve table, ChainSync `RollForward` discovery,
BlockFetch tag-24 payload, the follower's header+body validation, peer-log
parsing) on a private net (magic 42) where the operator controls stake and Ade
wins slots in minutes. It does **not** satisfy the bounty (which grades
preview/preprod). After a passing C1 serve dry-run the next step is **register on
preprod and run the same path** (the C2 pass), not more private-net work.

## Why the path cannot drift (the bounty-alignment guarantee)

The C1 serve dry-run uses the **identical** `--mode node` accepted-block + serve
path as the preprod pass — `import_live_consensus_inputs` → forge → self-accept →
sibling serve task (S2) → the single serve-dispatch authority (S1) → the BLUE
chain-sync/block-fetch servers (`DC-CONS-17/18`) → the single CN-WIRE-08 tag-24
envelope → the follower's log → `correlate`. Mechanically fenced:
`ci/ci_check_node_path_fidelity.sh` (no private-only flag, no from-genesis
constructor), `ci/ci_check_single_serve_dispatch_authority.sh` (one serve
authority), `ci/ci_check_serve_listener_magic_aware.sh` (the serve advertises the
**configured** magic, S2b — so the magic-42 follower's handshake succeeds), and
the containment / handoff fences (byte-unchanged). The serve is **request-driven
only** (no proactive `advance_tip`). **Fast slots come from the operator-authored
private genesis stake (operator input), never from Ade code.**

## 0. Venue (C1 only)

- **C1 — private testnet** (the kept `~/.cardano-private-testnet-c1`, network
  magic **42**): the operator authors a Shelley+Conway genesis where Ade's pool
  holds ~all active stake, so Ade is a near-every-slot leader and a **follower**
  Haskell peer validates+accepts its served block. The peer is a **follower, NOT
  a co-producer** (§3). There is no C2 arm here — preprod is the bounty surface
  (separate G-C runbook). The single-epoch limit applies (`DC-EPOCH-03` fails
  closed at the boundary).

## 1. Pre-flight — genesis-consistency pin (BEFORE any live KES signature)

Identical to the G-C/G-D pre-flight: pin Ade's recovered consensus-inputs +
leader-eligibility inputs (extracted via the same `import_live_consensus_inputs`
path preprod uses) against the genesis-derived reference for the recovered seed
epoch, before any live KES signature:

```
cargo test -p ade_testkit --test <genesis_pinning harness>   # ade_testkit::consensus::genesis_pinning
```

**Necessary but NOT sufficient for acceptance** — acceptance is proven ONLY by the
follower's log through `correlate` (§4). Pre-seed the store (reusing the N-M-C
`seed_to_snapshot` extraction) so the node takes the **WarmStart** arm (`FirstRun`
is Mithril-only; `CN-NODE-01` intact).

## 2. Launch the forge-capable `--mode node` **with `--listen`** (same flags as preprod + serve)

```
ade_node --mode node \
  --listen <ADE_LISTEN_ADDR> \               # S2: the node-spine serve listener (e.g. 0.0.0.0:3001)
  --network-magic 42 \                       # C1 magic — drives the S2b magic-aware serve version table
  --peer <UPSTREAM_FEED_ADDR> \              # the live WirePump feed (REQUIRED to reach ForgeTick)
  --json-seed <utxo.json> \                  # MANDATORY (operator-seeded ledger)
  --consensus-inputs-path <bundle> \         # MANDATORY (import_live_consensus_inputs — SAME path as preprod)
  --cold-skey <node.skey> --kes-skey <kes.skey> --vrf-skey <vrf.skey> \  # forge intent: COMPLETE set or none
  --opcert <node.opcert> --genesis-file <shelley-genesis.json> \
  --wal-dir <wal/> --snapshot-dir <snap/> \  # durable state (REQUIRED)
  --genesis-hash <64-hex> --genesis <config>
```

`--listen` (S2) spawns the node-spine serve sibling outside `run_relay_loop`; the
serve advertises the **configured** magic (42) via `n2n_supported_for_magic`
(S2b), so the magic-42 follower's N2N handshake succeeds. `--listen` is in the
closed `--mode node` flag set (no new flag; `ci_check_node_path_fidelity.sh`
green). Forge intent is the COMPLETE key set or none
(`EXIT_NODE_FORGE_KEY_INGRESS_FAILED = 44`). A serve-listener bind failure is
surfaced fail-fast (`NodeLifecycleError::ServeStart`) — the node never proceeds
claiming live-serve capability while serving is disabled.

## 3. Topology — Ade gets a Continuing feed **and** the follower fetches from Ade (A7)

Two **distinct** traffic directions must both hold (the A7 obligation):

- **(a) upstream feed** → keeps Ade's `NodeBlockSource::WirePump` `Continuing` so
  the forge loop reaches `ForgeTick` (Ade dials `--peer`).
- **(b) downstream serve** → a real Haskell `cardano-node` follower whose
  `topology.json` lists **Ade's `--listen` address** dials Ade, ChainSync-discovers
  Ade's served tip, and BlockFetches the body.

Whether (a) and (b) are the **same reciprocal Haskell peer** (Ade `--peer` it; its
topology lists Ade) or **two Haskell nodes** is the operator's topology choice —
**confirm it at run time** (do not assume one connection satisfies both). The
follower (whose log you capture) runs with **no forging credentials** (no
KES/VRF/opcert) — a validator, **NOT a co-producer**. Exactly one producer per
pool key (Ade); a co-producing peer would fork/double-forge and muddy the signal.

**Request-driven only:** arrange the follower to ChainSync from a point **behind**
Ade's forged tip so its `RequestNext` finds the block already served (a
`RollForward` then a `BlockFetch`). **If the follower parks at tip before Ade
forges and never receives a proactive `RollForward`, STOP** — that needs a
proactive `advance_tip` driver, which is a **separate shared-path cluster**, not
part of this run.

## 4. Capture the follower's log → `correlate` → rehearsal manifest

When the follower fetches + validates + adopts/serves the Ade-forged block,
capture its evidence and normalize to the closed JSONL the allow-list parser
accepts (`ba02_evidence::parse_peer_accept_events`):

- `{"event":"peer_served_block","block_hash_hex":"<64-hex>","slot":<N>,"peer":"<ade-listen-addr>"}` — STRONGEST (the follower fetched this block from Ade).
- `{"event":"peer_chain_tip","block_hash_hex":"<64-hex>","slot":<N>,"peer":"<addr>"}` — corroborating.

Every other line (any Ade-internal `self_accept` / `forge_succeeded` /
`block_received` / served-side signal) is **dropped** by the allow-list. Compute
the captured log's sha256, then run the env-gated operator execution harness (a
RED test, skipped in CI; **not** a runtime node mode) — it writes the manifest
ONLY on a real `correlate`-produced match:

```
ADE_LIVE_C1_SERVE=1 \
ADE_LIVE_FORGED_BLOCK_HASH=<64-hex of the Ade-forged block> \
ADE_LIVE_FORGED_SLOT=<N> \
ADE_LIVE_NETWORK_MAGIC=42 \
ADE_LIVE_PEER_LOG=<captured-follower.log> \
ADE_LIVE_REHEARSAL_PEER_LOG_SHA256=$(sha256sum <captured-follower.log> | awk '{print $1}') \
ADE_LIVE_REHEARSAL_MANIFEST_OUT=docs/evidence/phase4-n-f-g-h-node-serve-<run>.toml \
  cargo test -p ade_node --test node_c1_serve_live node_c1_serve_live -- --nocapture
```

A `NoEvidence` outcome **panics and writes nothing** — the follower did not accept;
re-check stake / leadership / genesis-consistency / topology. Ade self-accept is
NOT acceptance.

## 5. The evidence is a non-promotable `PrivateRehearsalManifest`

The harness reuses `rehearsal_pass::{correlate_peer_log_file_into_rehearsal,
write_private_rehearsal_manifest}` (G-D) — the C1 serve evidence is the SAME
non-promotable rehearsal envelope (`venue = "private-testnet-c1"`,
`is_rehearsal = true`, `not_bounty_evidence = true`), gated by
`ci_check_rehearsal_manifest_schema.sh`, produced ONLY by a real correlate match
(`NoEvidence` writes nothing). It is **not** a `Ba02Manifest` and lives **never**
under the bounty home — C1 serve acceptance is a rehearsal, not the bounty.

## 6. Delta from the G-C/G-D operator-pass runbooks (the ONLY differences)

| Aspect | G-C/G-D pass | G-H C1 serve dry-run |
|---|---|---|
| `--mode node` path + flags | same | **same + `--listen`** (S2; in the closed flag set) |
| forge / self-accept / consensus-inputs | same | **same** |
| serve handshake magic | (serve unexercised) | **configured magic 42 via `n2n_supported_for_magic`** (S2b) |
| traffic direction | Ade dials/forges | **+ downstream follower DIALS Ade's `--listen` and fetches** (A7) |
| peer-log capture → `correlate` | same | **same** (follower's `peer_served_block` log) |
| `NoEvidence` fail-closed | same | **same** |
| evidence envelope | G-D: `PrivateRehearsalManifest` | **`PrivateRehearsalManifest`** (`docs/evidence/phase4-n-f-g-h-node-serve-*.toml`) |
| RO-LIVE flip | none (G-D) | **none** |

## 7. What this does NOT do

- It does **not** flip `RO-LIVE-01` or `RO-LIVE-06` — both stay
  partial/operator-gated; the bounty deliverable is the separate C2 preprod pass.
- It commits **no synthetic manifest**; the hermetic `correlate` fixture
  (`c1_dry_run_correlate_to_rehearsal_envelope`) is mechanics-only and never lives
  under the evidence home.
- It makes **no** acceptance claim from any Ade-internal signal (self-accept /
  served-block / wire success).
- It adds **no** proactive `advance_tip` — request-driven serve only; a
  parked-at-tip follower is a separate cluster.
- It is **not** the bounty deliverable; a passing C1 serve dry-run only
  **increases confidence** in the serve path it is a strict adaptation of.
