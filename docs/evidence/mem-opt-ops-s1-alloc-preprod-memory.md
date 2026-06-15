# MEM-OPT-OPS S1 (ALLOC) — Live preprod memory transcript with the mimalloc allocator (CE-OPS-1)

**Venue:** preprod public testnet via the local docker peer `cardano-node-preprod`
(N2N `127.0.0.1:3001`), epoch 295, `ade_node --mode admission` — the identical
MEM-MEASURE-A2 protocol. The **only** change from the A2 baseline run is the
process global allocator (default glibc `System` → **mimalloc**, `OP-MEM-02` /
`DC-MEM-06`). Same seed, same peer, same fresh-import warm-start; the comparison
is apples-to-apples (only the allocator differs).

**Binary:** release build of the MEM-OPT-OPS S1 commit (`#[global_allocator]
mimalloc::MiMalloc` at the RED binary entry; mimalloc symbols linked — verified).

**Seed point:** slot `125821799`, hash
`79b8826f8a6f6762251e7fee3ed27d50fd82fee1468df8221fd15a318e6cf614`;
consensus-inputs epoch 295 (fingerprint `40700aca…`); full preprod UTxO seed
(~3.8 GB cardano-cli `query utxo --whole-utxo` JSON), **fresh** wal/snapshot so
the run does the same full import (same ~6.8 GB import peak as A2).

**Run:** warm-start bootstrap (full-UTxO import) → follow + admit preprod blocks
from the seed point → SIGINT clean shutdown.

**Transcript:** `mem-opt-ops-s1-alloc-preprod-memory.jsonl`
**sha256:** `1ea71f1cdf153ebb999dedaae15a8e4f6b72ca1cc835c27050ab7ca29266e71f`

## Result — the allocator returns the retained import peak to the OS

| RSS (`VmRSS`, same `rss_sampler` as A2) | A2 baseline (glibc) | **S1 (mimalloc)** | Δ |
|---|---|---|---|
| p50 | 6,874,024 kiB (6.56 GiB) | **4,824,884 kiB (4.60 GiB)** | **−2,049,140 kiB / −29.8%** |
| p95 | 6,874,028 kiB | **4,824,968 kiB** | −29.8% |
| peak | 6,874,028 kiB (6.56 GiB) | **4,824,976 kiB (4.60 GiB)** | −29.8% |

**CE-OPS-1 met:** S1 resident memory is **strictly below** the A2 baseline
(6,874,028 → 4,824,976 kiB peak), `memory_summary{replay_verdict: "agreed"}`,
**0 diverged**, 18 `block_admitted` + 18 `agreement_verdict{lagging}` interleaved
with 21 `memory_measure` across 4 measurement points — block validation /
chain selection / persistence kept progressing under the new allocator (no
starvation, replay-equivalent by the enforced `DC-WAL-03`).

**Diagnosis confirmed.** The post-import idle footprint
(`idle_recovered_tip` / `wal_checkpoint_recovery`) is **2,432,956 kiB
(2.32 GiB)** — matching the grounding's ~2 GB steady-state-structures estimate
(`mem-opt-grounding.md §A`). Under glibc the A2 run stayed pinned flat at
~6.87 GiB the entire run (the freed import pages never returned); under mimalloc
the freed import peak is returned to the OS, and RSS settles to the live working
set (the p50/peak ~4.60 GiB reflects the active block-admission catch-up; the
recovered-idle point is lower still at 2.32 GiB). The ~4 GB A2 excess was indeed
retained transient import memory — the cheapest lever, banked.

## Standing vs the Haskell reference (informational; not the OP-MEM-02 claim)

On the same chain, the Haskell `cardano-node-preprod` measured **5.50 GiB**
(MEM-COMPARE-D). S1's peak (4.60 GiB `VmRSS`) is **0.90 GiB (−16.3%) below** it —
the allocator swap alone moves Ade from `ade_heavier` (+19%) toward `ade_below`.

**Honest scope — this is NOT yet OP-MEM-02 complete:**
- The metric here is `VmRSS` (the A2 sampler), **not** the OP-MEM-02 *owned*
  footprint (`Private_Dirty`/`RssAnon` via `smaps_rollup`) — that sampler + the
  ceiling gate are MEM-OPT-OPS **S3**.
- "Below the reference" (−16%) is not yet "**clearly** below" (the cluster
  target is ≤ 3.0 GB owned, aim 2.0–2.5) — the streaming import (S2), on-disk
  UTxO (MEM-OPT-UTXO-DISK), and compact representation (MEM-OPT-COMPACT) levers
  remain. The formal MEM-COMPARE-D verdict flip is S3.

So `OP-MEM-02` stays **`declared`**; this transcript is a committed S1 data point,
not a claim that OP-MEM-02 is met.

Validated by `ci/ci_check_mem_measure_evidence.sh` (closed convergence+memory
vocabulary, closed measurement points, replay verdict `agreed`, ≥1 block_admitted,
0 diverged, sha256-binding) and `ci/ci_check_mem_opt_s1_reduction.sh` (S1 RSS
strictly below the A2 baseline).
