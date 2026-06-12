# PHASE4-N-AO — CE-AO-6 live SELECT pass: surfaced gap (multi-block competing branch)

> **The live run is the arbiter.** The hermetic SELECT mechanism (S1–S6) is complete + committed,
> but the **live** two-producer SELECT pass surfaced a real gap, exactly the "multi-header /
> long-branch live geometry" the S2–S6 docs explicitly deferred ("not claimed by CE-AO-4").
> `CN-CONS-03` stays `declared` — it does NOT flip.

## What was run (2026-06-12)

- **Venue:** fresh 2-pool `cardano-testnet`, magic 42 (`altestnet-aos2`, attempt 2 — attempt 1 hit
  the known flaky 45s leadership check). Block 33 / slot 669 freeze point.
- **Recover:** Ade `--mode admission` `seed_to_snapshot` @ the live tip (slot 162) — **OK**.
- **Smoke test (host network, both peers same chain):** Ade `--mode node --participant-venue`
  vs both pools, 120s — **PASS**: recover→follow→admit (9 `block_admitted`), keep-alive
  **sustained** past the ~96s timeout (cookies 1–5, both peers, pongs validated), `agreement_verdict`
  reached. **The S6 binary's live path works.** (Evidence: `~/.cardano-ceai6/ao-smoke-*`.)
- **SELECT venue:** froze the venue, re-ran the two pools as `cn1`/`cn2` on **separate** docker nets
  (`aonet1`/`aonet2`, each published to the host :6001/:6002) so they diverge yet both stay reachable
  from Ade — the topology a multi-candidate SELECT needs (FOLLOW's partition→heal only makes a peer
  *reorg*). Confirmed divergence: cn1 block 36 / cn2 block 35, distinct hashes off the common parent.
- **SELECT run:** Ade vs `cn1`(:6001) + `cn2`(:6002), 150s — **FAIL-CLOSED**: `UnexpectedRollback`,
  exit 43. (Evidence: `~/.cardano-ceai6/ao-select-*`.)

## Root cause (confirmed)

The transcript shows a **slot regression** in the received stream (… 769 → 774 → **770**) — a competing
branch arrived — immediately before the fail-close. The fork had grown **deep** (common parent block 33;
cn1≈37, cn2≈42 → ~4–9 blocks each side).

`run_participant_sync`'s `NeedsForkChoice` dispatch (`dispatch_competing_fork_choice`,
`crates/ade_node/src/node_lifecycle.rs:~2712-2719`) resolves the fork anchor as
`get_block_by_hash(decoded.prev_hash)` — the competing block's **immediate parent** — and requires it to
be a durable stored block:

```
let anchor_stored = match chaindb.get_block_by_hash(&prev)? {
    Some(s) => s,
    None => return Err(NodeSyncError::UnexpectedRollback),   // <-- here
};
```

For a **single-block** competing candidate (the hermetic case), the immediate parent IS the durable
common parent — fine. But a **live** competing branch is **multi-block**: the competing block's immediate
parent is an intermediate block *on the competing branch*, which Ade (on the other branch) never admitted
→ `get_block_by_hash(prev) → None` → fail-closed. **The dispatch conflates "the competing block's
immediate parent" with "the fork anchor (last common durable ancestor)" — true only for a 1-deep fork.**

## The fix (the follow-on slice — the deferred multi-header aggregation)

The fork anchor is the **last common ancestor** (the deepest block shared by Ade's durable chain and the
competing branch), which IS durable — NOT the competing block's immediate parent. So the live SELECT must:

1. **Find the common-ancestor fork anchor** by walking the competing branch's `prev_hash` chain back until
   a durable stored block is reached (bounded by k). That stored block is the real `fork_anchor`.
2. **Fetch the full branch** anchor→competing-tip (the existing `prefetch_branch_bodies` already does
   `RequestRange(fork_anchor → winner_tip)` — it just needs the *deeper* anchor + the *intermediate*
   headers).
3. **Build a multi-header `CandidateFragment`** (S2's `build_candidate_fragment` already takes
   `&[HeaderInput]` — feed the whole branch, not one header).
4. S4 `prevalidate_branch` then proves the **complete** branch before commit (already multi-body-ready).

So S2/S4/S5/S6 are largely ready for N≥1; the gap is **S3's anchor resolution** (immediate-parent →
common-ancestor walk) + obtaining the intermediate headers (the competing branch's headers above the
anchor, which the wire pump must surface or `prefetch` must fetch headers-first).

**Secondary characterization owed:** the live wire pump (`run_admission_wire_pump`) per-peer ChainSync
re-anchors a divergent peer via `RollBackward(intersection)`; characterize when a 2nd producer's branch
reaches Ade as a competing **Block** (→ `NeedsForkChoice`, this path) vs a **RollBackward** (→ the FOLLOW
RollBack arm) — the interleaving in `spawn_live_wire_pump_source`'s merged source determines which fires.
(In this run all 33 received blocks were tagged peer `:6001` despite a cn2-branch hash appearing — the
per-peer labeling under the deep-fork interleave also needs a look.)

## Status

- **Hermetic SELECT (S1–S6): COMPLETE + committed** (`origin/main` `3e0a6ad6`). All gates + `cargo test
  -p ade_node` green. The byte-only boundary, prove-before-commit, replay/fence, and live BlockFetch
  bridge are all proven hermetically.
- **Live two-producer SELECT: BLOCKED on the above** — `CN-CONS-03` stays `declared`. This is a new
  follow-on slice (the multi-header live aggregation), the analog of the N-AK/AL/AM/AN clusters the
  FOLLOW half (CE-AI-6) needed.
- Diagnostic artifacts: `~/.cardano-ceai6/ao-{smoke,select}-*` (outside the repo).
