# PHASE4-N-AH CE-AH-6 — Operator-gated live acceptance: FULL BAR MET (run-4)

**Status: CE-AH-6 MET.** The full 8+3 acceptance bar of `S4-operator-live-acceptance.md` §12 holds in a **single** cert-free C2-LOCAL run against a real non-producing Haskell relay. DC-NODE-20 (local-tip forge base) + DC-NODE-21 (cert evidence-only) + DC-NODE-22 (warm-start re-entry) are all proven live.

- **Date:** 2026-06-08 · **Venue:** rung1-auto C2-LOCAL (magic 42, k=5, 2 pools, σ≈0.5) · binary with S1–S4b · **verbatim `--mode node`, cert-free.**
- It took four runs to get here, and that is the point of the live gate: run-1 proved the cert-free architecture but had no transcript; run-2 (with S4a's transcript) found the warm-start re-entry gap → DC-NODE-22 (S4b); run-3 confirmed DC-NODE-22 but was run-length-short on immutable depth; run-4 (longer chain) closes the full bar.

## The bar (against `S4-operator-live-acceptance.md` §12)
| # | Claim | Run-4 |
|---|---|---|
| 1 | catch up once | ✅ `caught_up_to_peer_tip` @ block 10 (followed==base) |
| 2 | self-admit via `pump_block` | ✅ `self_admit_via_pump_block=true` ×18 |
| 3 | direct extend mode | ✅ `entered_forge_mode=single_producer_extend_own_durable_spine` |
| 4 | forge from `ChainDb::tip`, no cert | ✅ `forge_base_source=local_chaindb_tip` ×461 |
| 5 | real non-producing relay adopts | ✅ `AddedToCurrentChain=17`, relay `TraceForge=0` |
| 6 | ≥1 follow-link EOF, forging continued | ✅✅ crossed pre **and** post-restart |
| 7 | >k blocks settle immutable | ✅ anchor 10, Ade forged 11–28, relay tip 29 → immutable ~24 → **~14 Ade blocks k-deep > k=5** |
| 8 | warm-start (recovery + resumption) | ✅✅✅ post-restart `forge_mode=extend`, `forge_base_block_no=28`, `followed_peer_tip=null`; `post_forge=2`; relay adopted post-restart; `ws_err=0` |
| ¬1 | no `--adoption-cert-path` flag | ✅ harness cert-free; binary rejects the flag |
| ¬2 | no adoption cert read by Ade | ✅ `cert_path_present:true` count = 0; only cert = the KES opcert |
| ¬3 | no cert file in forge authority | ✅ `cert_path_present:false` ×461; `ci_check_cert_evidence_only.sh` green |

## The two decisive transcript lines
Pre-kill forge base (cert-free, local tip):
```
{"event":"forge_base_selected","forge_mode":"caught_up_to_peer_tip","forge_base_source":"local_chaindb_tip",
 "forge_base_block_no":10,"followed_peer_tip_block_no":10,…,"cert_path_present":false}
```
Post-restart warm-start re-entry (DC-NODE-22 — extend directly, no catch-up):
```
{"event":"forge_base_selected","forge_mode":"single_producer_extend_own_durable_spine","forge_base_source":"local_chaindb_tip",
 "forge_base_block_no":28,"followed_peer_tip_block_no":null,…,"cert_path_present":false}
```

## Provenance
Raw logs preserved in operator scratch `~/.cardano-rung1-host/s4-run4-20260608T130541Z/`. The earlier partials are at `s4-run1-…`, `s4-run2-…`, `s4-run3-…` and documented in `phase4-n-ah-live-run-{1,2,3}*`. Harness not committed (competition secrecy); no keys/hosts/addresses here. Hermetic backstop: S3 `local_spine_*` (replay) + S4b `warm_start_*` (re-entry) tests.
