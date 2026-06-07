# CE-A5 manifest — a real Haskell relay adopts an Ade-forged block

**2026-06-07. Hermetic C2-LOCAL venue `c2ae18` (cardano-testnet, `--testnet-magic 42`,
Conway, `--epoch-length 2000`).**

A real Haskell `cardano-node 11.0.1` relay (a non-producing pool node) adopted, as its
**current chain tip**, a block forged by Ade impersonating `pool1` — the bounty-relevant
C2-LOCAL #8–#9 manifest.

## What happened

1. **Recover behind the relay tip (T−k).** Ade `--mode admission` recovered from the
   frozen venue at block 8 / slot 98 (`seed_to_snapshot` + Ade-native WAL).
2. **Freeze.** The venue was stopped; relay node2 restarted from the frozen DB and
   served its tip **T = block 16 / slot 271 / hash `18384db8…`** (node2 forging = 0 —
   non-producing).
3. **Follow → gate → forge.** Ade `--mode node` (single continuous run) followed the
   relay to block 16, the forge-on-followed-tip gate (AE.A) admitted, and Ade forged
   **T+1 = block 17 / slot 421 / hash `db3b5675…`** on parent block 16 (`succeeded=1`).
4. **Serve → ADOPT.** Ade served block 17. The relay `FindIntersect`'d at block 16,
   rolled forward, and **`AddedToCurrentChain` block 17**.

## The proof (relay's own log — `phase4-n-ae-ce-a5-relay-adoption.jsonl`)

```json
{"ns":"ChainDB.AddBlockEvent.AddedToCurrentChain","data":{"kind":"AddedToCurrentChain",
 "newSuffixSelectView":{"blockNo":17,"issuerHash":"a1ed4e040f7f7f387b83b939bd95f240f540d0a7ed32eb82e8dc5238",
 "slotNo":421,...},"newtip":"db3b5675a441b7b9468a8f5b334a9b7c03164ce41ba742604711bea888e65d17@421"}}
```

## Issuer correlation (airtight)

- `pool1` cold-key hash = `blake2b-224(cold VK c14343dc…)` = **`a1ed4e040f7f7f387b83b939bd95f240f540d0a7ed32eb82e8dc5238`**.
- Relay block-17 `issuerHash` = **`a1ed4e04…`** → **MATCH**.
- Ade loaded `pool1`'s cold/kes/vrf/opcert; the relay (node2) forging count = 0; Ade
  `forge_result succeeded` = 1. The only producer of block 17 was Ade. ∎

## Root cause that closed it — chain-sync server cursor (PHASE4-N-AE.E)

The prior four CE-A5 attempts failed with `HeaderEnvelopeError (UnexpectedBlockNo (BlockNo N) (BlockNo 0))`.
Root cause: the chain-sync **server** `FindIntersect` handler resolved the intersect
(block 16) and replied `IntersectFound`, but never set its read cursor
(`last_announced`). The next `RequestNext` therefore served `next_after(None)` = block 0,
and the relay (read pointer at block 16, expecting block_no 17) rejected
`UnexpectedBlockNo(17)(0)`. Origin-sync clients were unaffected (`None` is correct for
them); only a non-Origin intersect (the relay's own tip) exposed it. Fix: set
`last_announced` to the intersect point in the `FindIntersect` handler
(`crates/ade_network/src/chain_sync/server.rs`). AE.A (forge on the right parent) +
AE.B (anchor/forge-parent intersectable) + AE.C (recover→follow WAL continuity) were each
necessary; AE.E was the final closer.

## Honest scope / residual

- After adoption, the relay echoed Ade's own block 17 (slot 421) back over the follow
  link; Ade's receive path fail-closed `SlotBeforeLastApplied { last: 421, attempted: 421 }`
  (exit 43). This is a **post-adoption** artifact — the manifest (`AddedToCurrentChain`)
  occurred first. Idempotent handling of a peer re-announcing Ade's own tip is a
  follow-up refinement, not a manifest blocker.
- Single hermetic venue; the forge half has been live-proven across all reruns. The
  cursor fix is mechanically regression-tested
  (`producer_chain_sync_serve_find_intersect_sets_cursor_then_rolls_forward_past_it`).
