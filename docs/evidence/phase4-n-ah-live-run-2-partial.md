# PHASE4-N-AH S4 — Live Run 2 (PARTIAL): forge-base transcript direct; warm-start re-entry gap found

**Status: S4 PARTIAL.** A meaningfully stronger live architectural validation than run-1. It **does NOT close CE-AH-6**: warm-start *recovery* is confirmed, but warm-start *forge-resumption* failed — a real node-lifecycle gap the live gate surfaced (the next invariant, DC-NODE-22). Recorded honestly, not softened; the S4 bar is **not** weakened.

- **Date:** 2026-06-08
- **Venue:** rung1-auto C2-LOCAL — `cardano-testnet`, magic 42, k=5, 2 pools (operator scratch, outside the repo).
- **Key difference vs run-1:** S4a (`7049d813`) made the `--mode node` `--log` JSONL the canonical transcript, so the forge-base decision is now **directly witnessed**.

## What run-2 confirmed
- **Forge-base transcript is now direct** (S4a). The pre-kill `node-run.jsonl` carries **249 `forge_base_selected`** events, all `forge_base_source=local_chaindb_tip` and `cert_path_present=false`, plus **10 `forge_result{self_admit_via_pump_block=true, entered_forge_mode=single_producer_extend_own_durable_spine}`**:
  ```
  {"event":"forge_base_selected","forge_mode":"caught_up_to_peer_tip","forge_base_source":"local_chaindb_tip",
   "forge_base_hash":"62c308d3…","forge_base_block_no":10,"followed_peer_tip_block_no":10,…,"cert_path_present":false}
  {"event":"forge_result","outcome":"succeeded","self_admit_via_pump_block":true,
   "entered_forge_mode":"single_producer_extend_own_durable_spine"}
  ```
- **Cert-free local-tip path confirmed** (249 × `cert_path_present:false`; no `--adoption-cert-path`; only cert in any log is the KES opcert).
- **Relay adoption confirmed** (a real non-producing Haskell relay, `AddedToCurrentChain`).
- **EOF continuation confirmed** (`admission_wire_pump … exit=Eof`; the extend state sustained past it pre-kill).
- **Warm-start recovery confirmed** (`ws_err=0`, no ChainBreak; the relay continued block 18→20 with no rollback).

## What failed — warm-start forge resumption (point 8b)
After kill+restart, the post-restart `node-run.jsonl` shows **96 `forge_tick_considered`, 0 `forge_attempted`, 0 `forge_base_selected`** → all `NoTipAvailable`.
**Root cause:** warm-start re-initializes `forge_mode = InitialCatchupRequired`, so Ade must *re-catch-up* via the follow link before it can forge — but the follow link **EOF'd before re-catch-up**, so it stalled. DC-NODE-20 covers the *extend state*; it does **not** cover **warm-start re-entry into** the extend state. Ade recovered its durable tip (block 20) fine; it just won't resume *forging* on it. This reintroduces the old follow-link dependency through restart.

## Per-claim table (against `S4-operator-live-acceptance.md` §12)
| # | Claim | Run-2 |
|---|---|---|
| 1 | catch up once | ✅ witnessed (`forge_mode=caught_up_to_peer_tip`, followed==base=10) |
| 2 | self-admit via `pump_block` | ✅ **now witnessed** (`self_admit_via_pump_block=true` ×10) |
| 3 | direct extend mode | ✅ **now witnessed** (`entered_forge_mode=single_producer_extend_own_durable_spine`) |
| 4 | forge from `ChainDb::tip` | ✅ **now witnessed** (`forge_base_source=local_chaindb_tip` ×249) |
| 5 | relay adopts | ✅ (`AddedToCurrentChain` 8→10) |
| 6 | ≥1 follow-link EOF | ✅ (`exit=Eof`; extend sustained past) |
| 7 | >k immutable | ⚠️ borderline (~k) — chain growth cut short by 8b |
| 8 | warm-start | ⚠️ **recovery ✓, forge-resumption ✗** |
| ¬1 ¬2 ¬3 | cert-free | ✅ (`cert_path_present:false` ×249) |

## Next (per the live gate's purpose — it found the next invariant)
1. **S4b / DC-NODE-22 — single-producer warm-start re-entry derives forge mode from the recovered local durable spine.** On warm-start in a declared rung-1 single-producer venue, when the recovered durable `ChainDb::tip` is above the bootstrap anchor (Ade has forged its own spine), `forge_mode` must re-enter `SingleProducerExtendOwnDurableSpine{current_tip = ChainDb::tip}` under the DC-NODE-20 fence, **without** a fresh followed-peer catch-up. The warm-start analog of DC-NODE-20.
2. **Fix the harness counter** to read `node-run.jsonl` (S4a moved the forge events off stderr).
3. **Run-3** after S4b.

## Provenance
Raw logs preserved in operator scratch `~/.cardano-rung1-host/s4-run2-20260608T103233Z/` (`ws-pre-node.jsonl`, `node-run-postrestart.{jsonl,log}`, `c2-relay.log`, `ws-pre/post-relay-tip.json`). Harness not committed (competition secrecy); no keys/hosts/addresses here. Hermetic S3 (`local_spine_kill_warm_start_byte_identical`) remains the byte-identity proof.
