# Slice MEM-OPT-UTXO-DISK S2b-2c-A — static UTxO off-heap + cached fingerprint

> **Status:** A.1 (cached fingerprint) DONE; A.2 (off-heap anchor) next. The bounty-critical (BA-08) memory win for the CURRENT live admission path, which runs `track_utxo=false` (it does NOT mutate the UTxO per block).
> **Prior:** the 2c pre-resolve + 2c.1a anchor machinery — proven infrastructure for the LATER `track_utxo=true` live-validation slice (B).

## Honest scope (the guardrail)
This slice **does NOT enable full live UTxO application.** It optimizes the current `track_utxo=false` live admission path by:
- **(A.1)** caching/reusing the UTxO-component fingerprint while the UTxO is unchanged — preventing repeated recomputation of an unchanged fingerprint (the S0 per-block churn), and
- **(A.2)** moving the static UTxO storage off the anonymous heap into the on-disk anchor (the owned-RSS win).

The pre-resolve / WorkingSet / redb-commit / position-reconcile path (2c, 2c.1a) remains **proven infrastructure for the later `track_utxo=true` live-validation slice (B = LIVE-LEDGER-APPLY), which remains OWED.**

## Why A (not B) for this slice
The measured BA-08 problem on the live path is: a static 1.9M-entry UTxO retained in heap + repeated full-UTxO fingerprint scans. The live admission does not mutate the UTxO (`track_utxo=false`), so the correct memory slice is to move the static UTxO off heap + reuse its unchanged fingerprint — directly targeting the measured problem **without changing live consensus behavior.** B (enable `track_utxo=true` + full live UTxO application/validation) touches ledger verdict authority, admission semantics, recovery, WAL, rollback, and peer divergence — too much blast radius for memory work; it is a deliberate later slice (`LIVE-LEDGER-APPLY`).

## A.1 — cached UTxO fingerprint (DONE)
- `OverlayUtxo` carries a `generation` counter — bumped on every insert/remove, COPIED on clone (so the live clone-per-block path keeps the same generation), NOT bumped by compaction.
- `fingerprint_v2_with_utxo(state, utxo_fp)` computes the combined fingerprint from a PRECOMPUTED utxo component (byte-identical to `fingerprint_v2` when the component is the real one).
- `UtxoFpCache` keys the utxo-component fingerprint on the generation: reuse iff unchanged; any mutation bumps the generation → recompute. It can NEVER serve a stale fingerprint.
- The live admission's `post_fp` uses the cache (`runner.rs`), so an unchanged UTxO skips the full per-block scan. post_fp is byte-identical → block-hash agreement + replay verdict unchanged (`admission_replay_equivalence` + adversarial + cross-epoch all green).

## A.2 — static UTxO off-heap (via the EXISTING snapshot — user-confirmed)
NOT a redundant redb anchor: `seed_to_snapshot` already durably stores the UTxO off-heap, and the live loop reads the in-memory UTxO ONLY for the post_fp — so it is DROPPED after bootstrap. The redb `UtxoAnchor` stays as proven infrastructure for B.

**A.2.1 (DONE):** `StaticUtxoFp { fingerprint_version, bootstrap_anchor, utxo_component_fp, valid_only_when_track_utxo_false }` — explicit (not generation-magic); `from_bootstrap_utxo` computes the component once; `utxo_component(track_utxo)` FAILS CLOSED under `track_utxo=true` / version mismatch.

**A.2.2 (DONE — wiring):** bootstrap computes `StaticUtxoFp::from_bootstrap_utxo(&utxo, initial_fp)` BEFORE `drop(utxo)` (the 1.9M-entry in-memory UTxO is freed; `ledger.utxo_state` stays empty; the durable copy is the snapshot already written). The admission `post_fp` uses `static_utxo_fp.utxo_component(next_ledger.track_utxo)?` (fail-closed exit `StaticUtxoFpInvalid` under `track_utxo=true`). Proven: `fingerprint_v2_with_utxo` ignores `state.utxo_state`, so the empty-UTxO live ledger yields the SAME post_fp as the full-UTxO ledger given the same component (byte-identical). `admission_replay_equivalence` + adversarial + cross-epoch green (wiring sound); ade_ledger 572 + ade_node 310 green.

**A.2.2 part 2 (DONE — the live owned-RSS re-measure = BA-08 evidence):** live preprod docker peer (epoch 295, fresh 3.8 GB seed): active-admission owned RssAnon **1.94 GiB** (down 2.65 GiB / 58% from the 4.59 GiB baseline; **below Haskell 2.57 GiB** → BA-08 achieved); 36 blocks admitted, `replay_verdict` agreed, 0 diverged. Evidence: `docs/evidence/mem-opt-utxo-disk-s2b-2cA-owned-rss-remeasure.{jsonl,md}`. **A is now fully met; OP-MEM-02 can flip.**

## Acceptance criteria (the A merge gate)
- [x] live behavior remains `track_utxo=false`
- [x] block hash agreement unchanged (`admission_replay_equivalence` green)
- [x] replay verdict remains agreed
- [x] UTxO component fingerprint computed once and reused only while unchanged (`UtxoFpCache`)
- [x] reuse-when-changed invalidates the cache (generation bump → recompute; proven)
- [x] static UTxO off heap (A.2.2): the in-memory UTxO is dropped after bootstrap; the existing snapshot is the durable off-heap copy
- [x] track_utxo=true still requires a real UTxO / the redb-WorkingSet (StaticUtxoFp fails closed)
- [x] post_fp equals the old post_fp for the same admitted blocks (empty-UTxO + static fp == full-UTxO full-scan; proven)
- [x] owned-RSS improves (A.2.2 part 2 DONE): active-admission owned RssAnon **1.94 GiB** (−58% from the 4.59 GiB baseline; **below Haskell 2.57 GiB**); replay agreed, 0 diverged, 36 admits
- [x] docs explicitly say B remains owed for full live validation (this doc)
