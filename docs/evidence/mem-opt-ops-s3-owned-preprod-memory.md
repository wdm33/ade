# MEM-OPT-OPS S3 (MEASURE) — Owned-footprint re-measurement + honest owned comparison (CE-OPS-3)

**Venue:** preprod via the docker peer `cardano-node-preprod`, `ade_node --mode
admission` — the SAME A2/S1/S2 scenario, same seed, fresh wal/snap. The S3 binary
samples the OWNED footprint (`RssAnon` from `/proc/self/status`, `Private_Dirty`
from `/proc/self/smaps_rollup`) alongside gross `VmRSS`.

**Transcript:** `mem-opt-ops-s3-owned-preprod-memory.jsonl`
**sha256:** `f50bf97c8c79ed388bbf0a045892cd34ba885396675fbac39abf5e5a29122138`
**Same scenario (replay-equivalent):** `initial_ledger_fp_hex` ==
`fb7cb12a2332cbbc6dc04aea470c1001d6a8dfa647a59a9f3c149a9560aed6b4` (== S1/S2);
`memory_summary{replay_verdict: "agreed"}`, 0 diverged, 9 block_admitted.

## Owned footprint (the OP-MEM-02 metric) — the honest picture

| point | gross `VmRSS` | OWNED `RssAnon` | OWNED `Private_Dirty` |
|---|---|---|---|
| seed_import (post-import) | 3,405,056 kiB | 3,399,752 (3.24 GiB) | 3,399,752 |
| **idle / recovered tip** | 2,049,612 | **2,042,968 kiB (1.95 GiB)** | 2,042,972 |
| **active admission (p50)** | 4,820,096 | **4,813,052 kiB (4.59 GiB)** | 4,812,984 |
| summary owned p50 / peak | — | **4,813,052 / 4,994,832** | 4,812,984 / 4,994,832 |

**Key reading — the owned metric overturns the gross signal.** `RssAnon ≈ VmRSS`
at every point: Ade's resident memory is almost entirely **anonymous owned heap**,
NOT a reclaimable file-backed `chain.db` mmap (the S2 hypothesis was wrong — redb's
`chain.db` cost during admission is **anonymous write buffers** + the
`seed_to_snapshot` serialization, counted in `RssAnon`, not mmap). So the owned
footprint does NOT shrink below gross.

- **Idle / recovered owned = 1.95 GiB** — matches the grounding's ~2 GB live-state
  estimate; clearly below the ≤3 GB target. The import-side wins (S1+S2) are real.
- **Active-admission owned = 4.59 GiB (p50)** — the working set jumps ~2.8 GiB over
  idle during admission and stays there (the `seed_to_snapshot`/redb serialization
  cost, anonymous + retained). This is ABOVE the ≤3 GB target.

## Honest owned comparison (regenerated) — `ade_heavier`

Same window (15 samples, nearest-rank): the Haskell `cardano-node-preprod`'s OWNED
`RssAnon` (readable from `/proc/<pid>/status`; `smaps_rollup` is ptrace-denied):

| OWNED `RssAnon` | Ade (admission p50) | Haskell (p50) | Haskell (peak) |
|---|---|---|---|
| | **4,813,052 kiB (4.59 GiB)** | 2,696,828 (2.57 GiB) | 4,139,200 (3.95 GiB) |

**Verdict: `ade_heavier` on the owned metric.** Ade's active-admission owned heap
(4.59 GiB) is ABOVE the Haskell node's windowed owned p50 (2.57 GiB; GHC's GC swings
it 2.57–3.95 GiB). This is the OPPOSITE of the gross-VmRSS signal (where Ade 4.82 <
Haskell 5.50 in MEM-COMPARE-D) — the owned measurement reveals the true standing.

**Caveats (honest):** Ade was actively *catching up* (9 blocks, working set up) while
the Haskell node was following its tip (lighter); Haskell's owned is GC-variable. Ade's
*idle* owned (1.95 GiB) IS below Haskell — but the representative active-admission
footprint is heavier.

## `OP-MEM-02` STAYS `declared`

Ade's owned footprint is NOT clearly below the target/reference during active
admission (4.59 GiB > 2.57 GiB Haskell, > 3 GB target). No flip from VmRSS/VmHWM,
and no flip from owned either — because owned is NOT clearly below. The import-side
levers (S1 allocator + S2 streaming import) banked the idle footprint (1.95 GiB),
but the **dominant owned cost is the `seed_to_snapshot`/`chain.db` serialization
during admission (the S2 finding, now confirmed on the owned metric)** — the
MEM-OPT-UTXO-DISK target. MEM-OPT-OPS alone does NOT clear the preprod owned posture.

Validated by `ci/ci_check_mem_measure_evidence.sh` (the S3 transcript schema) +
`ci/ci_check_mem_opt_s3_owned.sh` (owned-evidence schema + the honest-verdict check
over `mem-opt-ops-s3-owned-compare-preprod.jsonl`).
