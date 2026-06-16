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

## A.2 — static UTxO off-heap (NEXT)
Move the imported static UTxO into the on-disk anchor (read-only during `track_utxo=false` admission); the in-memory `UTxOState` becomes thin (the cached fingerprint is all the live path needs). Route the snapshot/checkpoint/recovery readers through the anchor. Then re-measure owned RSS (S0/S3 scenario) — the owned-RSS drop = the BA-08 win.

## Acceptance criteria (the A merge gate)
- [x] live behavior remains `track_utxo=false`
- [x] block hash agreement unchanged (`admission_replay_equivalence` green)
- [x] replay verdict remains agreed
- [x] UTxO component fingerprint computed once and reused only while unchanged (`UtxoFpCache`)
- [x] reuse-when-changed invalidates the cache (generation bump → recompute; proven)
- [ ] static UTxO anchor is off heap (A.2)
- [ ] owned-RSS improves in the same S0/S3 scenario (A.2 re-measure)
- [x] docs explicitly say B remains owed for full live validation (this doc)
