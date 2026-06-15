# MEM-OPT-OPS S2 (IMPORT) — Live preprod streaming-import transcript (CE-OPS-2)

**Venue:** preprod via the local docker peer `cardano-node-preprod` (N2N
`127.0.0.1:3001`), epoch 295, `ade_node --mode admission` — the identical
A2/S1 protocol, same seed, same peer, fresh wal/snapshot. The ONLY change from
S1 is the seed-import path: **whole-buffer → streaming** (`Deserializer::from_reader`,
never materializing the 3.8 GB file buffer or the intermediate `RawUtxoMap`).

**Binary:** release build of the MEM-OPT-OPS S2 commit (streaming
`import_cardano_cli_json_utxo` + the `seed_import` memory tap).

**Seed point:** slot `125821799`, hash `79b8826f…`; full preprod UTxO seed
(~3.8 GB), fresh wal/snap (clean import).

**Transcript:** `mem-opt-ops-s2-import-preprod-memory.jsonl`
**sha256:** `70151e45a8c63a6438e032e723bd4d369d6ba31eb4e66d62d5cedd5bf2e3176d`

## Result — the import peak is halved, byte-identical import

The `seed_import` measurement point captures `VmHWM` **right after `import()`
returns, before the chain.db snapshot write** — the import-specific peak:

| import peak (`VmHWM`) | whole-buffer (A2 resident footprint) | **streaming (S2 `seed_import`)** | Δ |
|---|---|---|---|
| | 6,874,028 kiB (6.56 GiB) | **3,405,288 kiB (3.25 GiB)** | **−3,468,740 kiB / −50.5%** |

- **CE-OPS-2 import-peak:** the streaming import peak (3.25 GiB) is **strictly
  below** the whole-buffer import footprint (6.56 GiB; glibc retained the
  whole-buffer import in A2, so its resident peak == that footprint). S1 *returned*
  the whole-buffer peak after the fact; **S2 prevents the spike** — the streaming
  build never holds the file buffer + intermediate map.
- **Byte-identical import (`DC-MEM-06` live):** `bootstrap_complete.initial_ledger_fp_hex`
  = `fb7cb12a2332cbbc6dc04aea470c1001d6a8dfa647a59a9f3c149a9560aed6b4` —
  **exactly S1's**. The streamed import produced the identical canonical ledger
  state (the hermetic 10-fixture equivalence test holds live too). A per-slice IDD
  review (M1) hardened this: both import paths now reject duplicate/colliding TxIns
  fail-closed (`DuplicateTxIn`) — byte-identical-or-rejected, never an order-dependent
  survivor. That is a no-op on this unique-key seed (no colliding outrefs), so this
  transcript faithfully represents the hardened binary's real-seed behavior.
- **Replay-equivalent:** `memory_summary{replay_verdict: "agreed"}`, **0 diverged**,
  9 `block_admitted` + 9 `agreement_verdict` interleaved with the memory samples.

## Separate finding — the chain.db snapshot transient (the next target, NOT the import)

The run-end `memory_summary.rss_hwm_kib` is **8,395,440 kiB (8.0 GiB)** — but this
is NOT the import. It is the node serializing the recovered 1.9M-entry UTxO into a
~4 GB `chain.db` snapshot (`seed_to_snapshot`), which happens *after* import. The
admission-phase gross `VmRSS` (p50 4.82 GiB, peak 5.0 GiB sampled; ~6.9 GiB observed
live) is dominated by the **mmap'd `chain.db`** (clean, file-backed, reclaimable
pages) plus that serialization transient. This is now the largest single memory
peak — a **separate optimization target**, logged for the next slice (likely folding
into MEM-OPT-UTXO-DISK, where the on-disk UTxO redesigns how state is snapshotted),
and a reason the *owned* footprint (`Private_Dirty`, MEM-OPT-OPS S3) — which excludes
the reclaimable mmap pages — is the metric that matters, not gross `VmRSS`.

**`OP-MEM-02` stays `declared`:** S2 measured `VmHWM`/`VmRSS`, not the owned
`Private_Dirty`/`RssAnon` metric (S3). S2's scoped claim — streaming import peak <
whole-buffer import peak, byte-identical — is met; the owned-footprint standing +
the comparison-verdict flip remain S3 / cluster-close.

Validated by `ci/ci_check_mem_measure_evidence.sh` (closed vocab incl. the
`seed_import` point, replay `agreed`, ≥1 admitted, 0 diverged, sha256-bound) and
`ci/ci_check_mem_opt_s2_import_peak.sh` (seed_import VmHWM < whole-buffer import +
`initial_ledger_fp_hex` == S1's).
