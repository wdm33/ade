# C1 genesis-successor rehearsal — reproduction runbook (post-G-R)

> **Status:** operator runbook (release/evidence tier, NOT runtime authority). This
> is the **consolidated, working** procedure that produced the first formally-bound
> C1 success: a real `cardano-node` follower validated + adopted an Ade-forged
> genesis-successor block 0, bound by `ba02_evidence::correlate` →
> `PrivateRehearsalManifest` (`docs/evidence/c1-genesis-rehearsal-manifest.toml`,
> hash `56a29ac4…c868f26f`, slot 122520).
>
> It supersedes the pre-fix scope README `phase4-n-f-g-j-genesis-rehearsal-README.md`
> for **reproduction** (that doc remains the original G-J *scope* statement). The
> binding fidelity rule is `docs/active/live-pass-path-fidelity-guide.md` — **read it
> first**. Nothing here is a private-only code path.

## The claim this produces (and its hard boundary)

A **C1 private-testnet rehearsal success ONLY**. A real follower validated + adopted
an **Ade-forged** genesis-successor block 0. It is **NOT** preprod, **NOT** bounty
BA-02, and flips **NO** RO-LIVE rule. Ade self-accept ≠ peer acceptance; a served
block ≠ peer acceptance; wire success ≠ peer acceptance. The only artifact is a
clearly-labeled, non-promotable `PrivateRehearsalManifest`
(`is_rehearsal = true`, `not_bounty_evidence = true`, `venue = private-testnet-c1`),
written ONLY by a real operator-captured follower log through `correlate`. Bounty
BA-02 stays gated on a **C2/preprod** pass with a staked Ade pool.

## Confirmed runs (the procedure is repeatable)

| Run | When (UTC) | Ade-forged + follower-adopted block 0 | Slot | Manifest |
|---|---|---|---|---|
| 1 | 2026-06-04 15:33 | `56a29ac4…c868f26f` | 122520 | `c1-genesis-rehearsal-manifest.toml` |
| 2 | 2026-06-04 16:25 | `850b56dc…fcfa0a` | 125634 | `c1-genesis-rehearsal-manifest-run2.toml` |

Two independent cold-start runs (run 2 executed straight from this runbook) each
produced a **distinct** Ade-forged genesis-successor block 0 that the reset follower
validated + adopted (`ValidCandidate` + `AddedToCurrentChain` → `correlate` →
`Agreed` → manifest, `peer_accept_source = chain_tip`). Distinct hashes (a fresh
winning slot → fresh VRF) under an identical procedure = **the procedure, not the
hash, is the repeatable artifact**. Both manifests are schema/sha/marker-checked by
`ci/ci_check_rehearsal_manifest_schema.sh`.

## Fidelity rule (do not drift)

C1 uses the **same `--mode node` accepted-block path** as C2. Ade's consensus inputs
come from **extraction** (`ci/build_consensus_inputs_bundle.sh` against the venue's
Haskell node) → `import_live_consensus_inputs` — **never** built from genesis. The
private genesis only (a) configures the network (magic/start/slot/KES/hash) and
(b) stakes Ade so it wins slots. Enforced by `ci/ci_check_node_path_fidelity.sh`. See
the fidelity guide for the full red-flag list.

## Why this is two-phase (a mechanics note that the G-J scope README omits)

`--mode node` does **not** import seed inputs; it **WarmStarts** from the staged
store (`--snapshot-dir` + `--wal-dir`) and then forges + serves. The seed import is a
**separate** `--mode admission` run that consumes `--json-seed` +
`--consensus-inputs-path` and writes the store. So reproduction is:

```
extract (build_consensus_inputs_bundle.sh)         # consensus inputs from the C1 node
  → --mode admission  (import → stage a clean NO-TIP store: snap/ + wal/)
  → --mode node       (WarmStart store → cold-start forge block 0 → self-accept → serve)
  → follower fetches + adopts → capture log → correlate → PrivateRehearsalManifest
```

(The G-J README §2 lists `--json-seed`/`--consensus-inputs-path` on `--mode node`;
those flags are parsed-but-unused there — the store is what `--mode node` reads.)

---

## Preconditions

- `cargo build -p ade_node` at or after `main` `0d7624d4` (the G-R serve gate is
  required — without it the forge churns block 0 and the follower never stabilizes).
- The C1 follower docker node (`cardano-node-c1`, host-net N2N `127.0.0.1:3010`,
  magic 42) is up, runs with **no** forging credentials (validator/follower only),
  and its chain.db can be reset by the operator.
- Operator inputs present under `~/.cardano-private-testnet-c1-conway/`:
  `shelley-genesis.json` (+ byron/alonzo/conway), `config.json`, the Ade pool keys
  `pools-keys/pool1/{cold.skey,kes.skey,vrf.skey,opcert}` (paths only — never
  committed), and the extracted `ade-inputs/{consensus-inputs.json,utxo-seed.json}`.

The conventions below use `C1=~/.cardano-private-testnet-c1-conway` and
`ADE=/home/ts/Code/rust/ade`.

---

## Phase A — reset the follower to genesis

The rehearsal is a **cold start**: the follower must be at genesis (epoch 0, slot 0,
no chain) so it intersects at Origin and adopts Ade's block 0. Archive first, then
clear.

1. Archive the current follower db + any prior success log (so nothing is lost):
   ```
   STAMP=$(date -u +%Y%m%dT%H%M%SZ)
   mkdir -p "$C1/c1-follower-archive-$STAMP"
   cp -a "$C1/db" "$C1/c1-follower-archive-$STAMP/db-before-reset"
   ```
2. Stop the node, clear only the chain stores (keep config/genesis/keys), restart:
   ```
   docker stop cardano-node-c1
   rm -rf "$C1"/db/db/{immutable,volatile,ledger}     # host bind-mount of the node's ChainDB
   docker start cardano-node-c1
   ```
3. Wait until the follower is up at genesis and confirm:
   ```
   docker exec cardano-node-c1 cardano-cli query tip \
     --testnet-magic 42 --socket-path /cfg/db/node.socket
   # expect: epoch 0, block 0 / slotInEpoch 0 (a fresh genesis follower)
   ```

---

## Phase B — extract consensus inputs (fidelity-correct; reuse if epoch unchanged)

Ade's consensus inputs are **extracted from the running C1 node**, never built from
genesis:

```
ADE_LIVE_PEER_CONTAINER=cardano-node-c1 \
ADE_LIVE_NETWORK_MAGIC=42 \
ADE_LIVE_PEER_SOCKET=/cfg/db/node.socket \
  bash "$ADE/ci/build_consensus_inputs_bundle.sh" "$C1/ade-inputs/consensus-inputs.json"
```

Then dump the seed UTxO (operator-funded addresses) via cardano-cli into
`$C1/ade-inputs/utxo-seed.json` in the `--json-seed` shape.

**Reuse note:** the C1 **epoch-0** nonce (`eta0 = 953a4c34…`) is genesis-derived and
constant while the net stays in epoch 0, so an existing epoch-0
`consensus-inputs.json` is reusable across resets. Re-extract only if the C1 epoch has
advanced (then it is no longer a *genesis* rehearsal). `DC-EPOCH-03` fails closed at
the epoch boundary by design.

---

## Phase C — stage a clean NO-TIP store (`--mode admission`)

Import the extracted inputs into a fresh store with **no persisted tip** (the
cold-start precondition). Dial a **dead peer** (`127.0.0.1:9`, nothing listening) so
admission imports the seed only and admits no blocks — deterministic regardless of
follower state:

```
rm -rf "$C1"/ade-inputs/{snap,wal}                  # fresh store
ZERO=0000000000000000000000000000000000000000000000000000000000000000   # genesis = Origin
GH=953a4c343cfec1e89a6f3f012afd18b134fe254820ae7db5f21d9eb23ce4cd8b      # C1 ShelleyGenesisHash (config.json)
"$ADE"/target/debug/ade_node --mode admission \
  --json-seed            "$C1/ade-inputs/utxo-seed.json" \
  --consensus-inputs-path "$C1/ade-inputs/consensus-inputs.json" \
  --seed-point-slot      0 \                  # genesis seed point: slot 0 …
  --seed-block-hash      "$ZERO" \            # … / all-zeros hash (Origin)
  --peer                 127.0.0.1:9 \
  --snapshot-dir         "$C1/ade-inputs/snap" \
  --wal-dir              "$C1/ade-inputs/wal" \
  --genesis-path         "$C1" \              # the genesis DIR (required; NOT --genesis-file here)
  --genesis-hash         "$GH" \
  --network-magic        42
# expect exit 0 (the dead peer logs ConnectionRefused — that's fine; the seed import
# completes anyway), a tip-0 store: snap/chain.db + wal/wal-0000.bin ≈ 76B (seed only).
```

Admission's required flag set is `--json-seed --seed-point-slot --seed-block-hash
--wal-dir --snapshot-dir --network-magic --genesis-hash --consensus-inputs-path` +
≥1 `--peer` + the general `--genesis-path` (`extract_admission_cli` / `cli.rs`).

A clean store every run avoids the `ChainBreak` that a previously-ingested block 0
leaves in the WAL (that re-recovery failure is the separate N-U durability concern,
not part of the rehearsal).

---

## Phase D — run the forge+serve node (`--mode node`)

Same flags as the C2/preprod node modulo operator inputs:

```
"$ADE"/target/debug/ade_node --mode node \
  --snapshot-dir   "$C1/ade-inputs/snap" \
  --wal-dir        "$C1/ade-inputs/wal" \
  --peer           127.0.0.1:3010 \              # live WirePump feed
  --listen         127.0.0.1:3002 \              # Ade's serve port (the follower fetches here)
  --cold-skey      "$C1/pools-keys/pool1/cold.skey" \
  --kes-skey       "$C1/pools-keys/pool1/kes.skey" \
  --vrf-skey       "$C1/pools-keys/pool1/vrf.skey" \
  --opcert         "$C1/pools-keys/pool1/opcert.cert" \
  --genesis-path   "$C1" \
  --genesis-file   "$C1/shelley-genesis.json" \
  --genesis-hash   "$GH" \
  --network-magic  42 \
  2> "$C1/ade-inputs/repro-node.stderr"
```

Forge intent is the **complete** key set or none (a partial set fails closed,
`EXIT_NODE_FORGE_KEY_INGRESS_FAILED = 44`). Expect `forge_result:not_leader` on most
slots and a `succeeded` on Ade's winning slots; the G-R serve gate keeps the **first**
block 0 served stably (re-forges are skipped, not re-served). Let it run until the
follower adopts (the success run adopted well within a ~240s window).

---

## Phase E — capture the follower's adoption

1. Confirm adoption in the follower's log (the decisive events):
   ```
   docker logs cardano-node-c1 --since "$STAMP" 2>&1 \
     | grep -E "ValidCandidate|AddedToCurrentChain" | tail
   ```
   Look for `AddBlockValidation.ValidCandidate` + `ChainDB...AddedToCurrentChain` for
   the Ade-forged hash, and confirm the new tip:
   ```
   docker exec cardano-node-c1 cardano-cli query tip --testnet-magic 42 --socket-path /cfg/db/node.socket
   # the "hash" field is the Ade-forged block-0 hash; block = 0
   ```
2. Build the closed peer-accept JSONL the allow-list parser accepts. The follower
   **adopts** Ade's block but does **not serve it back**, so the faithful event is its
   **chain tip** (not an overstated served-block). Write ONE line — use the actual
   adopted hash/slot from step 1:
   ```
   {"event":"peer_chain_tip","block_hash_hex":"<adopted 64-hex>","slot":<N>,"peer":"127.0.0.1:3010"}
   ```
   Allow-list = `peer_served_block` (strongest, `is_served=true`) and
   `peer_chain_tip` (corroborating, `is_served=false`); every other line is dropped.
   Save it to `$ADE/docs/evidence/c1-genesis-rehearsal-peer-accept.jsonl`. Also keep a
   trimmed raw follower-log proof (`…-follower.log`, the ValidCandidate/AddedToCurrentChain
   lines, ANSI stripped).

---

## Phase F — correlate → manifest (env-gated harness; ABSOLUTE paths)

The harness is `cargo test`, so its cwd is the **crate dir** — **use absolute paths**
or the file read fails `NotFound`. `correlate` writes the manifest ONLY on a real
match (a `NoEvidence` outcome panics and writes nothing):

```
JSONL=$ADE/docs/evidence/c1-genesis-rehearsal-peer-accept.jsonl
OUT=$ADE/docs/evidence/c1-genesis-rehearsal-manifest.toml
ADE_LIVE_C1_GENESIS_REHEARSAL=1 \
ADE_LIVE_FORGED_BLOCK_HASH=<adopted 64-hex> \
ADE_LIVE_FORGED_SLOT=<N> \
ADE_LIVE_NETWORK_MAGIC=42 \
ADE_LIVE_PEER_LOG="$JSONL" \
ADE_LIVE_REHEARSAL_PEER_LOG_SHA256=$(sha256sum "$JSONL" | awk '{print $1}') \
ADE_LIVE_REHEARSAL_MANIFEST_OUT="$OUT" \
  cargo test -p ade_node --test node_c1_genesis_rehearsal node_c1_genesis_rehearsal_live -- --nocapture
```

Expect `peer_accept_source = chain_tip`, `matched_block_hash_hex == ADE_LIVE_FORGED_BLOCK_HASH`.

---

## Phase G — verify + commit

- Verify the manifest markers: `is_rehearsal = true`, `not_bounty_evidence = true`,
  `venue = "private-testnet-c1"`, `matched_block_hash_hex` == forged hash.
- Confirm no RO-LIVE flip and the rehearsal gate is green:
  ```
  bash "$ADE"/ci/ci_check_rehearsal_manifest_schema.sh        # closed schema + markers + sha binding
  bash "$ADE"/ci/ci_check_served_chain_stability.sh           # DC-NODE-11 (the serve gate)
  ```
- Commit the manifest + peer-accept JSONL + trimmed follower-log proof under
  `docs/evidence/`. Each fresh run forges a **different** hash/slot (new VRF over a new
  winning slot) — that is expected; the procedure (not the hash) is the repeatable
  artifact.

---

## Troubleshooting (the failure classes the live loop already closed)

| Symptom (follower or Ade) | Cause | Fix (already on main) / action |
|---|---|---|
| Ade exits `Body(Decoding(UnexpectedType @ 0))` | feed didn't strip the block-fetch tag-24 wrapper | G-O / CN-WIRE-12 (fixed) |
| Ade exits `VrfCert(VerificationFailed)` at header validate | receive-path leader view had empty PoolDistr | G-P / DC-CINPUT-04 (fixed) |
| Ade exits `RecoveredTipMissingBlockNo` on tick 2 | forge read the stale WarmStart baseline, not the evolved spine | G-Q / DC-NODE-10 (fixed) |
| follower `UnexpectedBlockNo (BlockNo 1)(BlockNo 0)` churn, 0 adoptions | forge re-minted block 0 each slot; served view churned | G-R / DC-NODE-11 serve gate (fixed) |
| Ade `ChainBreak` at WarmStart | a prior run's ingested block 0 doesn't chain to the seed WAL | re-stage a clean store (Phase C); separate N-U durability item |
| `correlate` test panics `NotFound` at the file read | relative path from the crate-dir cwd | use ABSOLUTE paths (Phase F) |
| `correlate` `NoEvidence` (panics, no manifest) | the follower did NOT adopt — Ade self-accept is not acceptance | re-check stake/leadership/genesis-consistency + that the follower actually logged ValidCandidate/AddedToCurrentChain |
| follower `Connection refused` on `:3002` | Ade's serve window missed the follower's ~160s retry, or Ade exited early | keep the node running longer; confirm Ade reached `forge_attempted` + serve is up |

---

## What this does NOT do / what's next

- No RO-LIVE flip; no bounty claim; not durable progression to block 1+; not broad
  live sync. A passing C1 genesis rehearsal only **increases confidence** in the
  null-prev genesis-successor accepted-block path.
- **Next (separate cluster, OQ-R1):** full producer own-tip advance — forge
  self-accepts → node adopts its own block as the **durable tip** → next forge builds
  block 1, 2, … This runbook is the **regression target** for that work: if OQ-R1
  breaks block-0 acceptance, re-running this procedure catches it immediately.
