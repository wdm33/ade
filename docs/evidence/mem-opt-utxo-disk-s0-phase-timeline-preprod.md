# MEM-OPT-UTXO-DISK S0 (DIAGNOSTIC) — phase-resolved owned-footprint timeline (CE-UD-0)

**Venue:** preprod via the docker peer `cardano-node-preprod`, `ade_node --mode
admission` with `ADE_MEM_PHASE_DIAGNOSTIC=1` — the SAME A2/S1/S2/S3 scenario
(same seed, same seed point slot 125821799, fresh wal/snap). RELEASE binary with
the S0 mechanism (mimalloc global allocator + the quarantined `ade_mem_diag`
forced-collect probe).

**Transcript:** `mem-opt-utxo-disk-s0-phase-timeline-preprod.jsonl` (147 lines)
**sha256:** `060d4ce2b857d07a2b6f52d34fdc16f72d1e907dc61f4b5b0f1e05287cb6c861`
**Same scenario (replay-equivalent):** `initial_ledger_fp_hex ==
fb7cb12a2332cbbc6dc04aea470c1001d6a8dfa647a59a9f3c149a9560aed6b4` (== S1/S2/S3);
`memory_summary{replay_verdict: "agreed"}`, **0 diverged**, 34 block_admitted. The
diagnostic (phase taps + the two forced collects) did NOT perturb authority.

## The owned (`RssAnon`) trajectory — the decisive phase timeline

| phase point | owned `RssAnon` | what it shows |
|---|---|---|
| `seed_import` (t1, post-import) | 3.24 GiB | parsed UTxO + import residue |
| `t2_snapshot_serializing` (t2) | 1.81 GiB | post-`seed_to_snapshot` |
| **`t3_after_forced_allocator_collect_diagnostic_only`** | **0.05 GiB** | **bootstrap transient FULLY reclaimable → ~idle** |
| `idle_recovered_tip` | 0.05 GiB | post-collect idle baseline |
| first `mempool_admission` | 4.76 GiB | **+2.8 GiB STEP the instant admission begins** |
| `mempool_admission` ×10 (pre-t5) | 4.59 GiB (t4) | flat active-admission level |
| **`t5_active_admission_after_forced_collect`** | **1.78 GiB** | **collect CAN dip it (−2.8 GiB)** |
| first `mempool_admission` (post-t5) | 4.59 GiB | **RE-ACCUMULATES instantly** |
| `mempool_admission` ×23 + `sustained` | 4.59 GiB | flat — re-established + maintained |

## Reading (decisive)

1. **Bootstrap transient (import + `seed_to_snapshot` serialization) is fully
   reclaimable.** The t3 forced collect drops owned to ~idle (0.05 GiB this run;
   1.95 GiB on the prior no-t5 run — the difference is `mi_collect`'s `MADV_DONTNEED`
   aggressiveness). The original cluster hypothesis (the 4.6 GiB *is* the snapshot
   serialization) is **overturned** — that part is transient.

2. **The active-admission footprint (4.59 GiB) is a LIVE working set.** A forced
   collect *during* admission (t5) momentarily drops owned to 1.78 GiB, but the
   **very next block re-establishes 4.59 GiB** and it stays flat for 23 more admits.
   The collect cannot keep it down — admission re-needs ~2.8 GiB resident every
   block. It is **flat, not per-admit-growing**, so it is a stable resident working
   set, not accumulating churn.

   *Why re-accumulation, not the dip, is the signal:* `mi_collect(force)` uses
   `MADV_DONTNEED`, dropping even live pages (which fault back on access) — so the
   t3=0.05 and t5=1.78 dips are momentary. Whether owned returns to the active
   level t4 (=4.59) is what classifies it; here it returns immediately ⇒ live.

**Verdict** (see `…-classification.jsonl`): `bootstrap_transient_but_admission_live_working_set`.
**Next slice:** the on-disk / bounded in-memory UTxO backend (`DC-MEM-05` + `DC-MEM-07`).
