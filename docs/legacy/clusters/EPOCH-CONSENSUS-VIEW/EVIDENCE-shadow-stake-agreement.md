# EVIEW shadow stake-agreement evidence (DC-EPOCH-11 / DC-EVIEW-08 — milestone A)

**What this proves.** Ade's live reduced-UTxO checkpoint, run through
`ReducedUtxoCheckpoint::derive_stake_by_pool`, derives the same next-epoch per-pool stake
distribution as `cardano-cli` on the real Preview chain — the *non-activating* shadow agreement
that precedes the live boundary flip. This is the first live proof that Ade derives its **own**
next-epoch view faithfully (not merely imports one).

## Result — real Preview epoch 1334, cardano-node 11.0.1

Re-confirmed at HEAD `cdcd9397` on 2026-06-24 (byte-identical to the 2026-06-21 run):

- **Reduction (apples-to-apples, same point).** Ade's per-credential UTxO reduction over the whole
  real UTxO (3,067,766 entries; 1,642,739 base + 1,425,027 non-contributing) **==** cardano-node's
  current incremental stake: **254,261 / 254,261 credentials EXACT (100.00%), sum|diff| = 0.**
  → `classify_output_stake_ref` + `reduce_txout` are byte-identical to cardano-node on the real
  Preview UTxO.
- **Aggregation (`derive_stake_by_pool` vs `cardano-cli stake-snapshot` MARK).** 613 / 624 pools
  EXACT (98.2%). **ADE1 EXACT: `ade = oracle = 1,001,512,398,903` (diff 0).**
- The 11 non-exact pools and the −0.0145% total are a **documented point-in-time artifact** —
  Ade's mid-epoch *current* stake vs the boundary-*frozen* mark (~15.7 h apart) — **not** a formula
  error (the reduction is proven 100 % exact at the same point). The perfectly boundary-aligned
  pool match is owed at a real boundary.

## What this does and does not gate

- This is the **shadow agreement** for `DC-EPOCH-11` (the live checkpoint derives Ade's own
  next-epoch view) and the stake half of `DC-EVIEW-08`. Per the activation-flip design it is kept
  as **GREEN evidence, not a BLUE pre-promotion gate** — promotion remains gated only by the
  deterministic activation predicate over canonical durable state.
- **Committed regression guard:** `crates/ade_runtime/tests/eview_shadow_ade1_regression.rs` pins
  the ADE1 derivation hermetically — ADE1's two real base-credential delegators
  (`185fff1f…` = 1,000,014,603,080 and `49b7177b…` = 1,497,795,823) sum to exactly the captured
  oracle value, so a reduce/aggregate/derive regression fails in CI.
- **Still owed for the flip** (both rules stay `declared`): the boundary-aligned pool match, the
  leadership-schedule agreement (Ade's derived schedule == `cardano-cli leadership-schedule`), and
  an **accepted ADE1 forge** on epoch N+1 — all at a real Preview Conway boundary (ECA-5).

## Full evidence (off-repo, run-then-revert harness)

`~/.cardano-c2-preview/eview-oracle-evidence/`:
- `eview_checkpoint_shadow.rs` — the harness (routes the real preview UTxO through the `-mat`
  checkpoint: `build_from` → `sum_base_credential_stake` → `derive_stake_by_pool`).
- `checkpoint-shadow-RESULT.txt` — the transcript.
- `ade-inputs-ep1335-fresh.json` — the fresh oracle bundle (epoch 1335).

Venue: docker `cardano-node-preview` (testnet magic 2); ADE1 = preview pool `pool1gv25…` /
hex `431549bf1414e0d4a95b9fdeccbe60f66109ff8b81f502b628b2b8f3`. The harness is off-repo by the
project's run-then-revert evidence practice; this file is the committed compact summary.
