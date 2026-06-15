# MEM-MEASURE-A2 — Live preprod memory-evidence transcript (OP-MEM-01)

**Venue:** preprod public testnet via the local docker peer `cardano-node-preprod`
(N2N `127.0.0.1:3001`), epoch 295, `ade_node --mode admission` — the proven
RO-LIVE-05 / DC-EVIDENCE-01 admission path. The memory taps are the same
observe-only seams as the `--mode node` `ConvergenceEvidence` instrumentation
(MEM-MEASURE-A2 build, `fbe08b58`), added to the admission runner so the
available live venue is covered.

**Binary:** release build of the A2 build (`fbe08b58`) + the admission-runner
memory seams (committed alongside this transcript).

**Seed point:** slot `125821799`, hash
`79b8826f8a6f6762251e7fee3ed27d50fd82fee1468df8221fd15a318e6cf614`;
consensus-inputs epoch 295 (412 pools, source_tip realigned to the seed point —
same epoch, same Eta0); full preprod UTxO seed (~3.8 GB cardano-cli
`query utxo --whole-utxo` JSON at the seed tip).

**Run:** warm-start bootstrap (full-UTxO import) → follow + admit preprod blocks
from the seed point → SIGINT clean shutdown.

**Transcript:** `mem-measure-a2-preprod-memory.jsonl`
**sha256:** `cba0bcccfd59506fcea67217c497ce1e1a918bd229156f7512246f19a26e6b5b`

## Evidence (OP-MEM-01 — no starvation + replay-equivalent under memory observation)

- **24 `block_admitted`** + 24 `agreement_verdict{lagging}` — block validation and
  chain agreement progressed continuously while RSS was being sampled (no
  starvation of block validation / chain selection / persistence; **0 `diverged`**).
- **27 `memory_measure`** across **4 measurement points**
  (`wal_checkpoint_recovery`, `idle_recovered_tip`, `mempool_admission` ×24,
  `sustained`), each paired with the durable tip ledger fingerprint observed at
  that point.
- **1 `memory_summary{replay_verdict: "agreed"}`** — the run exited cleanly with no
  fatal Diverged halt, so the durable chain is replay-equivalent by the enforced
  `DC-WAL-03`. RSS p50 / p95 / peak = `6874024` / `6874028` / `6874028` kiB
  (~6.87 GB — dominated by the full preprod UTxO held as an in-memory map).

The RSS magnitude (~6.87 GB) is recorded as operational evidence; reducing it to
match/beat the Haskell node's average over 10 days is `MEM-COMPARE-D` (the BA-08
criterion), not A2. A2's claim is narrower and proven here: **memory sampling did
not perturb authoritative semantics and did not starve the authoritative work**.

Validated by `ci/ci_check_mem_measure_evidence.sh` (closed convergence+admission+
memory vocabulary, closed measurement points, replay verdict `agreed`, `≥1`
`block_admitted`, `0` diverged, sha256-binding).
