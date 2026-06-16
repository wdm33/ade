# MEM-OPT-UTXO-DISK S2b-2c.1b-A.2.2 part 2 — owned-RSS re-measure (BA-08 evidence)

**Result: BA-08 achieved.** Dropping the in-memory static UTxO (A.2.2) brings Ade's
**active-admission owned RssAnon to 1.94 GiB — below the Haskell `cardano-node-preprod`
baseline (2.57 GiB)** and 58% below the prior Ade baseline (4.59 GiB), with the live
admission still **agreeing** (0 diverged).

## Measurement (live, preprod docker peer, epoch 295)

| Run | seed | active-admission owned RssAnon (p50) |
|---|---|---|
| **A.2 (this run, in-memory UTxO dropped)** | fresh preprod, 3.8 GB JSON, slot 125928328 | **2,037,588 kiB = 1.94 GiB** |
| S3 baseline (full in-memory UTxO retained) | preprod, 554 MB JSON | 4,813,052 kiB = 4.59 GiB |
| Haskell `cardano-node-preprod` (owned) | — | ~2.57 GiB |

- **Reduction: 2.65 GiB (58%)** vs the S3 baseline — and the A.2 run used a *larger* UTxO
  (3.8 GB current preprod vs 554 MB), so the win is conservative.
- **Below Haskell:** 1.94 GiB < 2.57 GiB → the active-admission owned posture is now at/below
  the Haskell node (the BA-08 target).

## Owned RssAnon trajectory (this run)
- `seed_import`: 3.26 GiB — the 3.8 GB seed parsed + the full UTxO built (transient).
- `wal_checkpoint_recovery` / `idle_recovered_tip`: 1.94 GiB — **after `drop(utxo)`** (the
  in-memory UTxO is freed; `seed_to_snapshot` already wrote the durable copy).
- `mempool_admission` / `sustained` (active admission): **1.94 GiB** (p50 == peak, n=33,
  min 2,037,444 / max 2,037,592 kiB — rock-steady).
- gross peak VmRSS 6.32 GiB during the import/snapshot transient (reclaimable; the owned
  steady state is the 1.94 GiB above).

## Correctness (the dropped-UTxO + static-fp path admits + agrees)
- **36 block_admitted**, 36 agreement_verdict: **1 agreed + 35 lagging, 0 diverged**, 0
  same-slot hash mismatches. `memory_summary.replay_verdict = "agreed"`.
- "lagging" = Ade behind the peer's *tip* (the peer advanced during the run) while agreeing
  on the shared slots — not a divergence.

## Setup (reproducible)
- Binary: `target/release/ade_node` (commit `c73a420f`, A.2.2).
- Fresh preprod seed: `cardano-cli query utxo --whole-utxo` (epoch 295) → 3.8 GB JSON.
- Fresh bundle: `ci/build_consensus_inputs_bundle.sh --network preprod` (carries the
  `protocol_params_json` preimage; epoch 295, tip slot 125928328).
- `ade_node --mode admission --json-seed <seed> --consensus-inputs-path <bundle>
  --seed-point-slot 125928328 --seed-block-hash 136cc4…ca63 --network-magic 1
  --genesis-hash 162d29c4… --peer 127.0.0.1:3001 --log <transcript>`.
- The seed (3.8 GB) is operator-extracted scratch (NOT committed); the owned-RSS samples are
  in `mem-opt-utxo-disk-s2b-2cA-owned-rss-remeasure.jsonl`.

## Scope (honest)
This is the `track_utxo=false` header/tip-following live path. It replaces a retained static
in-memory UTxO with an explicit bootstrap UTxO fingerprint (`StaticUtxoFp`) + the existing
snapshot durability. It is **not** full live UTxO validation — `track_utxo=true` live
validation (B = `LIVE-LEDGER-APPLY`) remains owed. With this evidence, the A acceptance
criteria are all met; **`OP-MEM-02` (owned-RSS posture) can flip** from `declared`/`ade_heavier`
to enforced (owned now at/below Haskell on the live admission path).
