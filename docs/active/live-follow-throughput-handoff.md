# LIVE-FOLLOW-THROUGHPUT — handoff (next session)

> **RESOLVED 2026-06-18** — DC-MEM-11, commit `88e64df2` (pushed to origin/main;
> IDD reviewer "SAFE TO PUSH" + security review clean). The
> forward-sync reducer now serves the per-block WAL `post_fp` from a cached
> UTxO-component fingerprint (`ForwardSyncState.utxo_fp_cache` →
> `fingerprint_v2_with_utxo`) and `emit_participant_admit` reuses `state.prior_fp`,
> eliminating the 2× per-block full Ristretto255 UTxO fingerprint (~20s/block). The
> bottleneck WAS the UTxO fingerprint (a CPU-bound hash-to-curve over ~1.9M entries),
> not "something else" — the low-RSS hedge below was wrong. LIVE-CONFIRMED on
> C2-PREVIEW: recovered ~1000 blocks behind, closed the backlog in a ~5s burst
> (~190 blocks/s) and HELD the live tip (115065873 vs peer 115066258); replay agreed;
> rss_anon 1.36 GiB. Next: push, then Phase B (`ba02-phase-b-runbook.md`). The
> original investigation below is retained for history.

**Date:** 2026-06-17. **Why:** the forge is *correct* (recovers, KES-valid, races
slots) but cannot **stay at the live tip**, so it can't forge at a live leader slot.
This is the next blocker after the three correctness slices below.

## Where we are (all on origin/main)
Three slices restored a correct, forge-capable producer (Phase A — all 6 forge-up
checkpoints green):
- DURABLE-ADMISSION-BYTES (`3c6c30ea`, DC-WAL-05): WAL admit bytes survive restart.
- WARMSTART-ERA-SCHEDULE-VENUE (`1be6e855`, DC-CINPUT-05): venue epoch geometry
  persisted in the sidecar (v2→v3); no hardcoded 432000.
- OP-OPS-04 (`c51d7d81`): KES shell-init anchors evolution-0 at opcert_start,
  evolves to the current period.

Leadership was VERIFIED GREEN (see `ba02-phase-b-runbook.md`): Ade's OWN leader-check
code path AND cardano-cli both found Ade leads epoch-1331 slot **115077884**; ADE1
stake 1.0M ADA, sigma 0.000325. (NOTE: 115077884 is epoch-1331-specific — epoch 1331
ends slot 115084799 ≈ 22:30 UTC today; re-run the leadership probe for the
then-current epoch next session.)

## The blocker (THIS slice): live-follow throughput
With the OP-OPS-04 binary, a fresh forge on store6 was clean (WarmStart OK, KES OK,
feed stable — 0 wire-pump Eof). BUT measured catch-up rate:
- **~0.05 blocks/s (≈ 20s of CPU per admitted block), 99.8% CPU**, measured twice
  (115 B/25s and 345 B/60s of WAL growth; ~115 B per AdmitBlock entry).
- That equals the chain's OWN rate (1 block/20s on preview) → the forge **keeps pace
  but cannot close a backlog**. store6's recovered tip (~115033000) is ~1h / ~9000
  blocks behind the live tip → it stays ~9000 behind → cannot reach a live leader slot.
- RSS only **1.30 GiB** (NOT 4.59 GiB) → it is **NOT** the full 3M-entry UTxO re-scan;
  open files were `store6/chain.db` + `wal6/wal-0000.bin` (not the 2.9GB seed).

## Diagnosis / hypothesis
~20s CPU per block is pathological (a block should validate in ms). The bottleneck is
in the forge's per-block path (`pump_block` → validate → put_block → fingerprint),
NOT the admission path — the admission admits fast via the StaticUtxoFp optimization
(MEM-OPT-UTXO-DISK: compute the constant UTxO-component fingerprint ONCE, drop the
UTxO). The forge's `pump_block` very likely does NOT use that optimization and re-does
a per-block scan/read (candidate: per-block `fingerprint_v2` over a large recovered
ledger component, or an inefficient redb chain.db read per block). Because RSS is low,
it's probably NOT the UTxO map — look at the *other* per-block costs first.

## Investigation plan (next session)
1. Profile the forge live-follow loop: `perf record`/flamegraph on `ade_node --mode
   node` during catch-up, OR temporary instrumentation (timestamps) around
   `pump_block` sub-steps (header validate / body / ledger apply / fingerprint /
   put_block) — find which sub-step costs ~20s.
2. Compare to the admission path (`run_admission`, fast): what does the admission do
   that pump_block doesn't (StaticUtxoFp? a cached fingerprint? a different ledger
   apply)? Apply the same class of fix to the forge path.
3. Confirm: re-run the forge, measure catch-up >> chain rate (closes a backlog), then
   the forge reaches + holds the live tip.
4. Keep it consensus-critical-inline; hermetic test + the same memory guardrail
   (no heap-resident map regression; OP-MEM-02 / BA-08 owned-RSS).

## Then Phase B / BA02 (after throughput)
Re-seed close to the current tip → forge catches up + holds the live tip → forges at a
live leader slot H → dedicated localRoot topology (172.17.0.1:3033) → cardano-node
`AddedToCurrentChain H` → correlate. Manifest = leadership_reachability +
block_production + peer_adoption, from RAW logs. See `ba02-phase-b-runbook.md`.

## Venue state (preserved, hermetic)
- `$C2 = /home/ts/.cardano-c2-preview`. store6 (3.1G, v3 sidecar), wal6,
  ade-inputs2.json (epoch-1331 bundle), preview-utxo2.json (2.9GB seed).
- Forge evidence logs preserved: `$C2/ba02-forge.{jsonl,out}`.
- cardano-node-preview UP; topology RESTORED (Ade localRoot removed; backup at
  `config/topology.json.ba02-bak`). N2N `127.0.0.1:3001`→container, N2C socket via
  `docker exec`. Keys: `/home/ts/Code/rust/ade-ops/preview/ade-pool/keys`.
- Leader-schedule probe (scratch, deleted): rebuild as a cargo example using
  `import_live_consensus_inputs` → `merge_seed_epoch_consensus_inputs` →
  `PoolDistrView::from_seed_epoch_consensus_inputs` → `query_leader_schedule` +
  `is_leader_for_vrf_output` + `vrf_prove`. ADE1 pool `431549bf…b2b8f3`.
