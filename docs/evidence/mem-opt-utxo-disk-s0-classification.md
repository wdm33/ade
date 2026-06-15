# MEM-OPT-UTXO-DISK S0 — classification (CE-UD-0 verdict)

**Record:** `mem-opt-utxo-disk-s0-classification.jsonl`
**sha256:** `76cba475ff06ec6e8bd6e0b3c9eb50d47664ebab66176a93501859c9714b93ba`
**Derived from:** the committed phase timeline `mem-opt-utxo-disk-s0-phase-timeline-preprod.jsonl`.

## Verdict: `bootstrap_transient_but_admission_live_working_set`

| signal | owned `RssAnon` |
|---|---|
| idle baseline | 0.05 GiB (50,480 kiB) |
| t4 — active-admission steady (pre-t5) | 4.59 GiB (4,812,020 kiB) |
| t5 — forced collect DURING admission (dip) | 1.78 GiB (1,869,484 kiB) |
| post-t5 — next/steady admission | 4.59 GiB (4,812,912 kiB) |

**Rule (re-accumulation is the decisive signal — `mi_collect` `MADV_DONTNEED` makes
the t5 dip momentary):** `post_t5 (4,812,912) ≥ 0.85·t4 (4,090,217)` ⇒ the admission
footprint **re-accumulates to the active level** ⇒ live working set.

- **Bootstrap transient: reclaimable.** The t3 collect returns the import +
  `seed_to_snapshot` serialization to ~idle. This is NOT the active-admission cost
  (overturns the cluster's original snapshot hypothesis).
- **Admission footprint: live working set.** ~2.8 GiB re-establishes every block
  after a forced collect and holds flat. The admission path (UTxO resolution +
  redb caches + ledger view) genuinely needs it resident.

## Next slice

**The on-disk / bounded in-memory UTxO backend** (`DC-MEM-05` backend-independent
replay + `DC-MEM-07` bounded in-memory portion) — move the admission working set
off the anonymous heap (redb `TxIn→TxOut` + a bounded read cache + a k-deep
changelog), like Haskell's LMDB UTxO. **Not** the contained snapshot-streaming fix
(the bootstrap serialization is already fully reclaimable). The forced-collect
probe stays a diagnostic measurement only — it is never a production
memory-management trick.
