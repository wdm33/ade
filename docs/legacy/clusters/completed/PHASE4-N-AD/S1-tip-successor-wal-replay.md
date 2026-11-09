# PHASE4-N-AD S1 — Tip-successor WAL-replay recovery proof

> Strengthens **T-REC-05** + **DC-WAL-04** with a permanent regression for the C2-style tip-successor durability seam. No new rule. No production code change (test + C1 runbook note + registry evidence).

## Goal
Bank a controlled, hermetic proof that a forged **tip-successor** (block N+1 on a **non-Origin** parent) recovers byte-identically across kill/restart — i.e. the block-N → block-N+1 `prior_fp` chains across WAL replay — and classify the live aged-C1 `ChainBreak` as a **C1 genesis-successor-only** limitation, not a C2 blocker.

## Change
1. **Test (permanent regression)** — `crates/ade_node/src/node_sync.rs::forge_tip_successor_kill_then_warm_start_recovers_block_one`:
   - Forge block 0 (genesis-successor) over a recovered genesis base → durable admit (`admit_forged_block_durably → pump_block`).
   - Read the durable `ChainTip` of block 0; **forge block 1 on it** via `forge_one_from_recovered(selected_tip = Some(block 0))` → `PrevHash::Block(tip0)`, block-1 `prior_fp` = block-0's **real** durable `post_fp` → durable admit.
   - Drop chaindb + wal + state (the kill boundary).
   - Reopen + `warm_start_recovery` → assert recovers **block 1** slot+hash byte-identically (forward-replay from the slot-0 snapshot over **both** WAL blocks; **no ChainBreak**).
   - The genesis seam stays construction-matched (`anchor_fp == block-0 prior_fp == 0xA0`, exactly as the sibling T-REC-05 test); the **new** asserted seam is block-0 → block-1.
2. **C1 runbook note** — `docs/evidence/c1-genesis-rehearsal-reproduction-README.md`: the WarmStart `ChainBreak` on a cold-start genesis run is a **known C1 genesis-successor durability limitation** (seed ledger fingerprint ≠ `PrevHash::Genesis`/null prior), **NOT** a C2 tip-successor blocker (proven by the new test).
3. **Registry** — append the test to `T-REC-05.tests` + `DC-WAL-04.tests`; `strengthened_in += "PHASE4-N-AD"` on both; append an evidence note recording the tip-successor coverage + the genesis-ChainBreak classification.

## CE-1 (mechanical acceptance)
- `cargo test -p ade_node forge_tip_successor_kill_then_warm_start_recovers_block_one` → green (recovers block 1, no ChainBreak).
- `cargo test -p ade_node` → green (no regression).
- C1 runbook note + registry evidence note committed.

## Boundaries (per §8 of the cluster doc)
No genesis ChainBreak fix; no `PrevHash::Genesis` change; no live venue / young-C1 regen; no preprod/RO-LIVE claim; no new rule.
