# PHASE4-N-AD — Tip-successor durability proof (coverage correction for T-REC-05 / DC-WAL-04)

> **A narrow coverage-correction follow-up to the closed PHASE4-N-U** (do NOT reopen N-U; do NOT rewrite history). A diagnostic surfaced by the C2 tip-path scoping (2026-06-06): a live aged-C1 cold-start run hit a `ChainBreak` on WarmStart at the **seed → block-0** WAL seam. This cluster proves that seam is **C1 genesis-successor-only** and that the **C2-relevant tip-successor seam (block N → block N+1) is clean**, banking it as a permanent regression so the genesis ChainBreak is never again mistaken for a C2 durability blocker. Source: `project_oqr1_tipseed_durability.md`.

## §1 Primary property (no new rule)
A forged **tip-successor** block (block N+1 on a **non-Origin** parent) must durably admit, and on kill/restart its WAL entry's `prior_fp` (= the **real** durable `post_fp` of block N) must chain across WAL replay so WarmStart recovers the **byte-identical** successor tip — **no ChainBreak**. This is exactly the "previous entry's post_fp otherwise" clause already asserted by **DC-WAL-04**, and the forged+received recovery equivalence of **T-REC-05** — extended from a single block-0 forge to a real multi-block forged progression.

## §2 The diagnostic finding (proven from code + a live run)
- A live aged-C1 cold-start `--mode node` run forged 5 blocks in-process (WAL 76 B → 651 B; durable admit via `admit_forged_block_durably → pump_block` works), but WarmStart failed: `ChainBreak { entry_index: 1, expected_prior_fp: 036111…, actual_prior_fp: 0000… }`.
- Root cause (`ade_ledger/src/wal/replay.rs:112`): the **seed** WAL entry's `post_fp` = the seed-UTxO **ledger** fingerprint (`036111…`), but the **genesis-successor** block 0 (`forge_header_position → (0, PrevHash::Genesis)`) records `prior_fp = 0000…` (the Cardano genesis/null predecessor). `0000 ≠ 036111` → break at the **seed → block-0** seam only.
- This seam exists **only on the C1 genesis-successor path** (seed-at-Origin + block-0-on-`PrevHash::Genesis`). **C2 never hits it** — C2's first durable entry is a non-Origin snapshot-at-N, not block-0-on-Origin.
- Every prior recovery test (incl. T-REC-05) **masked** this by constructing `anchor_fp == the forged block's prior_fp` (`0xA0`), so the genesis seam was matched by construction and the **block-N → block-N+1** seam was never replay-tested.

## §3 The design — a controlled hermetic test, no venue
`forge_tip_successor_kill_then_warm_start_recovers_block_one` (`crates/ade_node/src/node_sync.rs`):
forge block 0 (genesis-successor) → durable admit → **forge block 1 on the durable non-Origin tip** (`PrevHash::Block(tip0)`, block-1's `prior_fp` = block-0's *real* `post_fp` from the durable apply) → durable admit → **kill** (drop chaindb+wal+state) → reopen → `warm_start_recovery` → assert recovers **block 1** slot+hash byte-identically, **no ChainBreak**.

The genesis seam (block-0 `prior_fp` vs seed) stays construction-matched (`anchor_fp == block-0 prior_fp`, exactly as T-REC-05) — that case is the documented C1-only limitation and is deliberately out of scope. The **new** seam under test is block-0 → block-1: a real tip-successor `prior_fp` chain. It is a faithful proxy for C2's N+1-on-N (structurally identical: `PrevHash::Block`, `prior_fp` = parent `post_fp`).

## §4 Scope of claim (careful wording — per user)
- **Proves:** the C2-style tip-successor WAL/recovery seam — a forged successor's `prior_fp` chains to the real previous `post_fp`, and WarmStart recovers the byte-identical successor tip with no ChainBreak.
- **Does NOT claim:** full preprod/C2 acceptance, peer adoption, or the end-to-end real-C2 integration (Mithril `seed_to_snapshot` at a real non-Origin tip → recover from the non-Origin snapshot → forge → recover live). That remains the eventual preprod/C2 pass — now unblocked on the durability front.
- **Classifies** the genesis ChainBreak as a **known C1 genesis-successor durability limitation** (seed ledger fingerprint ≠ `PrevHash::Genesis`/null prior), **NOT** a C2 tip-successor blocker.

## §5 TCB color map (FC/IS partition)
- **No production code change.** Test-only (`#[cfg(test)]` in the RED `ade_node::node_sync`) + docs (C1 runbook note) + registry evidence/strengthening.
- **Reused unchanged:** `forge_one_from_recovered`, `admit_forged_block_durably → pump_block`, `warm_start_recovery`, the WAL `prior_fp`/`post_fp` chain check (`ade_ledger::wal`). No BLUE change, no new canonical type, no schema/wire change.

## §6 Slices
| Slice | Scope | CE | Registry | TCB |
|---|---|---|---|---|
| **S1** | Add `forge_tip_successor_kill_then_warm_start_recovers_block_one` (permanent regression). Add the C1 runbook genesis-ChainBreak C1-only note. Strengthen T-REC-05 + DC-WAL-04 (append the test + `strengthened_in` + evidence note). | CE-1 | T-REC-05 + DC-WAL-04 strengthened (no new rule) | test/docs |

## §7 Cluster Exit Criteria (all mechanical)
- **CE-1:** `cargo test -p ade_node forge_tip_successor_kill_then_warm_start_recovers_block_one` green (the kill→recover recovers block 1, no ChainBreak).
- C1 runbook note committed (genesis ChainBreak = C1-only, not a C2 blocker).
- Registry evidence note updated on T-REC-05 + DC-WAL-04 (append-only; `strengthened_in += "PHASE4-N-AD"`).
- Tree green: `cargo test -p ade_node` green.

## §8 Forbidden during this cluster (hard boundaries — user-set)
- **No genesis ChainBreak fix** (it is the documented C1-only limitation; not touched here).
- **No change to `PrevHash::Genesis` / null** or block-0 wire semantics.
- **No young-C1 genesis regen / no live venue work.**
- **No preprod claim, no RO-LIVE flip** (test-only durability proof; not a live-acceptance claim).
- **No new rule, no reopening N-U, no history rewrite.**

## §9 Replay obligations
The test IS a replay-equivalence assertion (kill → WarmStart forward-replay → byte-identical tip). Recovery rides the existing snapshot + forward-replay law (`bootstrap_initial_state` warm-start branch); no new durability law, no schema/WAL change.

## §10 Invariants
- **Adds:** none.
- **Strengthens** (`strengthened_in += "PHASE4-N-AD"` at close): **T-REC-05** (forged recovery equivalence extended from block-0-only to a multi-block forged progression) and **DC-WAL-04** (the "previous entry's post_fp otherwise" tip-successor `prior_fp` clause now replay-tested).
- **Preserves:** the C1 genesis-successor durability limitation (documented, not changed), `PrevHash::Genesis`/null, all BLUE/wire rules.
