# MEM-COMPARE-D — Haskell-vs-Ade RSS comparison, preprod (BA-08)

The bounty's BA-08 criterion is that Ade match or beat the Haskell node's average
memory over 10 days. This artifact establishes the **measurement methodology** and
records the **current standing** at the same preprod venue/chain — honestly.

## Result

| Node | Workload (same preprod chain, epoch 295) | RSS |
|---|---|---|
| **Haskell `cardano-node-preprod`** | full node (chain-sync, ledger, mempool, block-production candidate) | **5.50 GB** (p50/p95/peak; 12 samples over ~48 s, dead-flat) |
| **Ade `--mode admission`** | warm-start + follow + admit (a subset of full-node work) | **6.56 GB** (p50/p95/peak; MEM-MEASURE-A2 transcript) |

**Verdict: `ade_heavier` — Ade loses BA-08 by 1.05 GB (+19.1%).** And the raw delta
understates it: Ade was doing *less* work (admission-follow, not a full node), yet
used ~19% more memory. A full Ade node would be heavier still.

## Root cause (to confirm by profiling — the optimization follow-on)

Ade's `--mode admission` warm-start imports the full preprod UTxO seed (~3.8 GB
cardano-cli JSON) and holds it as a fully-parsed in-memory map. That import
footprint (~6.56 GB stable RSS) dominates. The Haskell node holds the same UTxO
far more compactly (binary in-memory, or on-disk UTxO-HD). Closing the ~1 GB gap
is a memory-representation optimization (compact CBOR-bytes UTxO with lazy decode,
or on-disk UTxO) — the bounty-winning follow-on, NOT this slice.

## Methodology

- **Haskell:** `VmRSS` from `/proc/<pid>/status` of the `cardano-node-preprod`
  container (`docker inspect -f '{{.State.Pid}}'`), sampled 12× over ~48 s;
  nearest-rank p50/p95/peak (the A1 `RssWindow` math).
- **Ade:** the committed MEM-MEASURE-A2 transcript `memory_summary`
  (`docs/evidence/mem-measure-a2-preprod-memory.jsonl`) — Ade's in-process RSS
  sampler over the same preprod follow.
- **Caveats (honest):** not a perfectly concurrent / equal-workload run (Ade
  admission-follow vs Haskell full-node), and a representative snapshot rather than
  a 10-day sustained average (the bounty's actual test; this slice fixes the
  methodology so the sustained run is a re-sample, not a redesign).

## Transcript

**`mem-compare-d-preprod.jsonl`** (12 `haskell_rss_sample` + 1 `comparison_summary`)
**sha256:** `b22a910f3a687027c929f6a9b257d9be9122c427a3f07f31dc8431f081372101`

Validated by `ci/ci_check_mem_compare_evidence.sh`.
