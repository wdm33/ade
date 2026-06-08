# PHASE4-N-AH S4 — Live Run 3: DC-NODE-22 warm-start re-entry CONFIRMED live

**Status: DC-NODE-22 confirmed; full bar one row short (run-length).** Run-3 closed the last *architectural* gap (point 8, warm-start forge resumption). It does **not** yet close CE-AH-6: point 7 (>k immutable) was run-length-short. A run-4 with a longer pre-kill chain follows.

- **Date:** 2026-06-08 · **Venue:** rung1-auto C2-LOCAL (magic 42, k=5, 2-pool) · binary with S4b.

## The result — DC-NODE-22 works on a real wire
After the kill + warm-start, the post-restart transcript shows the node **re-entering extend directly from its recovered durable tip, with no follow-link catch-up**:
```
{"event":"forge_base_selected","forge_mode":"single_producer_extend_own_durable_spine",
 "forge_base_source":"local_chaindb_tip","forge_base_block_no":18,
 "followed_peer_tip_block_no":null,"followed_peer_tip_hash":null,"cert_path_present":false}
```
- `forge_mode = single_producer_extend_own_durable_spine` (DC-NODE-22 re-entry, **not** `caught_up_to_peer_tip`).
- `followed_peer_tip = null` — **no follow-link catch-up** (the exact run-2 dependency, eliminated).
- 78 post-restart `forge_base_selected`, 318 extend-mode events, `post_forge=1` (forged block 19), relay `post_adopt=7` (adopted a post-restart block), `ws_err=0` (no ChainBreak).

In run-2 this count was **0** (the node stalled in `NoTipAvailable`). DC-NODE-22 closed it.

## Per-claim (against `S4-operator-live-acceptance.md` §12)
| # | Claim | Run-3 |
|---|---|---|
| 1 | catch up once | ✅ (`caught_up_to_peer_tip` @ block 11) |
| 2 | self-admit via `pump_block` | ✅ (`self_admit=7`) |
| 3 | direct extend mode | ✅ |
| 4 | forge from `ChainDb::tip` | ✅ (`local_chaindb_tip` ×244 pre + ×73 post) |
| 5 | relay adopts | ✅ (adopt 6→7) |
| 6 | ≥1 follow-link EOF | ✅✅ (crossed pre **and** post-restart) |
| 7 | >k immutable | ⚠️ ~2 (run-length-short; see below) |
| 8 | warm-start (recovery + resumption) | ✅✅✅ **DC-NODE-22** (above) |
| ¬1 ¬2 ¬3 | cert-free | ✅ (`cert_path_present:false` ×244) |

**The architecture is fully proven live — DC-NODE-20 + DC-NODE-21 + DC-NODE-22.**

## Point 7 — run-length, not architecture
`SUSTAINED` triggered at `adopt≥6`, then the warm-start leg ended the run at relay block 18 (anchor ~11, Ade forged 12–18) → only ~2 Ade blocks are k-deep. The C2-LOCAL testnet produces blocks slowly near the tip (~1 block/75 s), so >k immutable needs a longer chain. **Run-1 already showed >k immutable** (relay reached 27) on the same cert-free DC-NODE-20 path. **Run-4** raises `SUSTAINED` to `adopt≥12` so >k Ade blocks are immutable *before* the warm-start kill — the clean single-run full bar → CE-AH-6 close.

## Provenance
Raw logs preserved in operator scratch `~/.cardano-rung1-host/s4-run3-20260608T123413Z/`. Harness not committed (competition secrecy); no keys/hosts/addresses here.
