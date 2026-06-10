# PHASE4-N-AI — CE-AI-6 convergence-pass operator runbook

> Operator-gated, derived-tier (cluster hard line 8). This proves the **exercised
> single-best-peer competing-producer convergence venue** — Ade follows one peer
> through a ChainSync rollback and replay-equivalently re-converges on the peer's
> branch. It does **NOT** prove full multi-peer Cardano ChainSel; multi-candidate
> live selection remains out of scope.

## What CE-AI-6 demonstrates

Ade (`--mode node`, **Participant** venue) + ≥1 Haskell `cardano-node` producer on a
competing-producer venue converge on the **same tip**, including **through a peer
reorg** (a `RollBackward`), with **no `Diverged`** verdict. The hermetic
arrival-order-independence of the selection authority is already proven by CE-AI-5
(`select_best_chain_arrival_order_independent_*`); this pass is the live counterpart.

## Prerequisites

- A competing-producer venue — see `docs/active/c2-preprod-tip-guide.md` (C2 = recover
  the real preprod tip via Mithril, then a second producer creates a short competing
  branch so the peer issues a `RollBackward`). **Never** a from-genesis Conway local net.
- Ade built at the cluster HEAD; operator KES/op-cert/VRF keys for the Participant.
- The local docker preprod peer (`reference-local-preprod-docker-cardano-node`) or a
  private testnet with a competing producer.

## Steps

1. Recover Ade's durable tip (Mithril → recover), per the C2 guide.
2. Start Ade **keyed-but-unstaked (σ=0)** so it is a pure follower (Ade never wins a slot ⇒
   never forges ⇒ no self-forged competing block ⇒ no `diverged`):
   `ade_node --mode node --participant-venue <keys/paths>
   --convergence-evidence-path docs/evidence/phase4-n-ai-convergence-pass.jsonl`.
   The convergence-evidence sink (PHASE4-N-AJ AJ-S1/S2, DC-NODE-30) writes the closed
   `block_received` / `block_admitted` / `agreement_verdict` transcript **directly** to that
   path. It is a **dedicated** file — **not** the sched/`forge_*` `--log` (that log is a
   separate, non-gate-valid artifact). Absent `--convergence-evidence-path`, no transcript is
   written and node behavior is unchanged.
3. Drive a peer reorg: have the competing Haskell producer extend a branch that wins,
   so the followed peer issues a `RollBackward` — Ade follows it (AI-S4b-ii:
   `run_participant_sync` → `apply_chain_event` → `WalEntry::RollBack`).
4. Let Ade re-converge on the peer's branch (sustained `agreement_verdict` agreed at the
   peer's tip; **0 diverged**).
5. Stop Ade. Write the manifest `docs/evidence/phase4-n-ai-convergence-pass.md` binding
   the `.jsonl` sha256:
   `echo "convergence-pass manifest; jsonl sha256: $(sha256sum docs/evidence/phase4-n-ai-convergence-pass.jsonl | cut -d' ' -f1)" >> docs/evidence/phase4-n-ai-convergence-pass.md`
   (plus run metadata: venue, peer, slots of the reorg).

## Validate + commit

- `bash ci/ci_check_convergence_evidence_schema.sh` — must pass (closed vocabulary,
  sha256-bound, 0 diverged, **≥1 slot regression** = the reorg was followed, no boring
  same-tip-only run).
- Commit the `.jsonl` + `.md`. The gate is **vacuous-until-committed**, so until then CI
  stays green without the transcript.

## Honesty (binding)

The committed transcript proves convergence for the **exercised** venue only. At
`/cluster-close`, scope or strengthen `CN-CONS-03` per its exact registry wording — do
**not** over-flip it to a full multi-peer ChainSel claim.

---

## Venue reproducibility (PHASE4-N-AL update, 2026-06-10) — the exact path that produced CE-AL-3-LIVE

> **DC-NODE-33 (PHASE4-N-AL) unblocked the participant recover→follow.** Before N-AL the
> participant path (`run_participant_sync`) failed closed on the relay's standard
> post-`IntersectFound` `RollBackward(anchor)` — a bare anchor is not a stored block, so
> `get_block_by_hash(anchor) → None → UnexpectedRollback`. N-AL adds the exact-slot+hash
> recovered-anchor no-op (the participant mirror of N-AK's `DC-NODE-32`). **CE-AL-3-LIVE proved
> it live** (fresh bare-anchor recover @ slot 741 → `RollBackward(741)` no-op'd → first admit @
> slot 777 → `agreed` @ slot 801, exact hash match, 0 diverged). So the recover→follow START is
> proven; **CE-AI-6 now = capturing a REORG** (a peer `RollBackward` Ade follows + re-converges).

Conventions: `IMG=ghcr.io/intersectmbo/cardano-node:11.0.1`; `W=$HOME/.cardano-c2-testnet`;
`ADE=…/target/debug/ade_node`; `<V>` = the venue output-dir; `<gen>`/`<inputs>`/`<store>`/`<K>` =
scratch dirs OUTSIDE the repo (e.g. `~/.cardano-ceai6/al-*`). Operator keys `<K>` are a
**keyed-but-unstaked (σ=0)** set — valid in format but NOT registered on the target venue, so
Ade can never win a slot (e.g. `~/.cardano-c2-testnet/ade-ceai6/keys/`).

### 1. Fresh venue — NEVER un-freeze a stale one
A long-idle frozen venue is unusable: wall-clock outruns the frozen tip past the forecast
horizon → `BlockchainTime.CurrentSlotUnknown` → the node never forges (deadlock). Always build
FRESH (`systemStart ≈ now`). `cardano-testnet`'s "node1 was unable to produce any blocks for
45s" is a FLAKY startup check (f=0.05 leadership luck) — **retry with fresh output-dirs** until
one bootstraps (container stays Up, tip block ≥ 2). Keep **both** pools producing (rung-2).
```
docker run -d --name altestnet --network host -v "$W:/work" -v "$HOME/.cardano-testnet-bin:/ctn" \
  -e PATH=/ctn/bin:/usr/bin:/bin -e HOME=/work -e TMPDIR=/work/tmp \
  -e CARDANO_CLI=/ctn/bin/cardano-cli -e CARDANO_NODE=/ctn/bin/cardano-node -e CARDANO_SUBMIT_API=/ctn/bin/cardano-submit-api \
  --entrypoint /ctn/bin/cardano-testnet "$IMG" \
  cardano --num-pool-nodes 2 --testnet-magic 42 --epoch-length 2000 --slot-length 1 --output-dir /work/<V>
```

### 2. Genesis + hash (in-container `cat` → ts-readable; venue files are root:600)
```
for f in shelley-genesis.json byron-genesis.json alonzo-genesis.json conway-genesis.json configuration.yaml; do
  docker exec altestnet cat /work/<V>/$f > <gen>/$f; done
VH=$(docker exec altestnet /ctn/bin/cardano-cli hash genesis-file --genesis /work/<V>/shelley-genesis.json)
```

### 3. Recover FRESH at the live tip (never reuse a stale store)
```
SOCK=/work/<V>/socket/node1/sock
docker exec altestnet cardano-cli query tip --testnet-magic 42 --socket-path $SOCK   # -> TIPSLOT, TIPHASH
ADE_LIVE_PEER_CONTAINER=altestnet ADE_LIVE_NETWORK_MAGIC=42 ADE_LIVE_PEER_SOCKET=$SOCK \
  ADE_LIVE_SHELLEY_GENESIS=<gen>/shelley-genesis.json ADE_LIVE_GENESIS_CONFIG=<gen-config.json> \
  bash ci/build_consensus_inputs_bundle.sh <inputs>/consensus-inputs.json
docker exec altestnet cardano-cli query utxo --whole-utxo --testnet-magic 42 --socket-path $SOCK --output-json > <inputs>/utxo-seed.json
$ADE --mode admission --json-seed <inputs>/utxo-seed.json --consensus-inputs-path <inputs>/consensus-inputs.json \
  --seed-point-slot $TIPSLOT --seed-block-hash $TIPHASH --wal-dir <store>/wal --snapshot-dir <store>/snap \
  --network-magic 42 --genesis-hash $VH --peer 127.0.0.1:9 --genesis-path <gen>
# bare anchor (76B WAL); the dummy --peer 127.0.0.1:9 dial-fail line is expected/non-fatal.
```

### 4. Participant follow (σ=0) — the operator key set needs FIVE flags
`--mode node --participant-venue` forge activation requires the COMPLETE set
`--cold-skey --kes-skey --vrf-skey --opcert --genesis-file` (the last is easy to miss → fail-closed exit 44).
```
$ADE --mode node --participant-venue \
  --cold-skey <K>/cold.skey --kes-skey <K>/kes.skey --vrf-skey <K>/vrf.skey --opcert <K>/opcert.cert \
  --genesis-file <gen>/shelley-genesis.json \
  --peer 127.0.0.1:<node1-N2N-port> --wal-dir <store>/wal --snapshot-dir <store>/snap \
  --network-magic 42 --genesis-hash $VH --genesis-path <gen> \
  --convergence-evidence-path docs/evidence/phase4-n-ai-convergence-pass.jsonl --log <node-run.jsonl>
```
CE-AL-3-LIVE PASS = first `block_admitted` (slot > anchor) with 0 `UnexpectedRollback` / 0
`UnsupportedRollbackPoint`; healthy follow reaches `agreement_verdict{agreed}` (our_hash == peer_hash).

## Inducing the reorg (CE-AI-6 — approach A, the remaining pass)

Natural reorgs at f=0.05 with 2 equal pools are rare (~1 per ~33-min epoch in a
fast-propagation local net), so CE-AI-6 needs an **induced** reorg via **partition → heal**:
run the two pools so they can be network-partitioned (separate containers on a docker **bridge**
network — `cardano-testnet`'s single `--network host` container cannot be cleanly partitioned),
partition them so each forges a divergent branch from a common parent, then reconnect so the
loser reorgs to the winner. The peer Ade follows must be the one that reorgs → it sends Ade
`RollBackward(forkpoint)` → Ade follows it → **strict slot regression** in the observed
peer-block sequence → **re-converges to `agreed`**, 0 diverged.

### Transcript integrity (BINDING — do not game the gate)
- The gate (`ci/ci_check_convergence_evidence_schema.sh`, DC-EVIDENCE-03) needs **ONE honest
  transcript from the participant path** showing the slot regression AND a final
  `agreement_verdict{agreed}` with **0 diverged**, sha256-bound by the `.md` manifest.
- **Do NOT filter, splice, or stitch transcripts.** A reorg captured across an Ade restart is
  acceptable ONLY if the restart-loop is a DEFINED operator harness preserving a single,
  continuous, append-only transcript — never hand-edited fragments.
- **EOF caveat (observed in CE-AL-3-LIVE):** Ade's `--mode node` follow exits cleanly
  (`admission_wire_pump: exit=Eof`) after it idles at the peer tip. To stay attached across a
  reorg the harness must re-attach on EOF (same `--convergence-evidence-path`), and that
  re-attach behavior must be documented as part of the operator pass — not reconstructed after.
