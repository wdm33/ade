# PHASE4-N-AF / DC-NODE-18 — CE-AF-6a live core proof (run c2t7)

**Date:** 2026-06-07. **Venue:** C2-LOCAL — a fresh `cardano-testnet` (3 pools, network magic 42,
epoch-length 2000, slot-length 1, k=5, ASC 0.05), frozen, with node2 relaunched as a **non-producing**
Haskell relay. **Binary:** `ade_node --mode node --single-producer-venue --adoption-cert-path <file>`.

## What this proves — CE-AF-6a (the DC-NODE-18 core invariant)

Single-producer Ade, after catching up to a real peer tip and given **explicit relay-adoption evidence**
for its first successor, forged the **next** successor on its **own durable adopted spine** — and the
relay adopted it — **without** the relay re-announcing (echoing) the adopted block back to Ade.

Sequence:
1. Ade caught up to the frozen relay tip (DC-NODE-15) and forged **block 11** — slot 327, hash
   `7e67cd0d4850737d278eb3e10403a830335e8918d60f87160870fccc88838fb2`.
2. The non-producing Haskell relay **adopted block 11** (`AddedToCurrentChain`).
3. The operator/harness observed the adoption (relay `query tip` = block 11) and wrote the **RED
   venue-adoption certificate** `11 327 7e67cd0d…` (block_no, slot, hash) — explicit evidence, **never**
   inferred from Ade's own self-admit.
4. Ade, in `FirstOwnBlockServed`, read the certificate and **promoted** to
   `SingleProducerExtendOwnDurableSpine`, matched by **chain-point identity (hash + block_no)**. Confirmed
   by the run diagnostic: `own_tip(slot=327,blk=11,h4=[7e,67,cd,0d]) == cert(slot=327,blk=11,h4=[7e,67,cd,0d])`.
5. Ade forged **block 12** — slot 406, hash
   `5b6b52bf5a40e45d981091e600daa00acd7b26bba07d46fd87239f4edf7e41b6` — on its **own durable adopted
   spine**, with **no relay re-announce of block 11**.
6. The relay **adopted block 12** (`AddedToCurrentChain`). Final relay tip: block 12 @ slot 406.

Counts: Ade `forged = 2` (blocks 11, 12); relay adopted both; `cert_written = 1`; `sparse = 0`.

## What this does NOT claim — CE-AF-6b (deferred to DC-NODE-19)

- sustained production past k,
- relay ImmutableDB settlement,
- follow-link liveness,
- forge-loop continuation after a follow-link EOF,
- epoch crossing,
- rung-1 complete.

The run stopped at 2 blocks because Ade's follow link to the relay **EOF'd** (relay ~5 s idle timeout;
Ade's follow pump sends no keep-alive), and the forge loop currently treats the follow source as a
lifecycle authority (`forge_tick_skipped: unknown_disconnected`). That stop cause is a loop-lifecycle
obligation scoped to **DC-NODE-19** (single-producer extend-mode loop continuation after follow EOF;
touches DC-NODE-05) — it is **not** a DC-NODE-18 authority failure. The extend authority is proven by the
adopted block 12.

## Two live-surfaced bugs (both missed by the hermetic suite + the IDD/security reviews — the live gate's value)

1. **not_leader advanced the mode** — the post-forge mode advancement keyed off the loop's `forged` flag,
   which is set on `not_leader` ticks (the forge ran, VRF said not-elected, no block admitted). Fixed:
   advance only on an actual admit (`forge_mode_after_admit`; regression test
   `forge_mode_after_admit_only_advances_on_real_admit`).
2. **cert match too strict** — promotion required full `TipPoint` equality incl. slot; the relay-reported
   slot need not byte-equal the served-tip slot. Fixed: match on chain-point identity (hash + block_no),
   consistent with the catch-up gate's documented slot-ignoring equality.

## Reproduce

- **Hermetic (no venue):** `cargo test -p ade_node` (the 6 DC-NODE-18 node_sync tests) +
  `bash ci/ci_check_single_producer_extend_own_spine.sh`.
- **Live core proof:** re-run the operator's C2-LOCAL single-producer harness (a fresh, frozen
  `cardano-testnet` magic 42 + a non-producing relay) with `ade_node --mode node --single-producer-venue
  --adoption-cert-path <file>`; the harness writes the certificate on observing the relay's adoption of
  Ade's first successor. The machine-readable transcript of this run is
  `phase4-n-af-extend-own-spine.jsonl` (alongside this file).
