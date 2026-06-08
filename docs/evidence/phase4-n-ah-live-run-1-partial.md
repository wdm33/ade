# PHASE4-N-AH S4 — Live Run 1 (PARTIAL): cert-free DC-NODE-20 architectural validation

**Status: S4 PARTIAL.** A strong **live architectural validation** of the cert-free DC-NODE-20 local-tip path. It **does NOT close CE-AH-6**: the direct forge-base/mode/self-admit transcript events and the warm-start byte-identical leg are missing (see Gaps). Recorded honestly per the acceptance contract, not softened.

- **Date:** 2026-06-08
- **Venue:** rung1-auto C2-LOCAL — `cardano-testnet`, network magic 42, k=5, 2 pool nodes, σ≈0.5 (operator scratch, outside the repo).
- **Flow:** Ade `--mode admission` recovers the **frozen testnet tip** (never from genesis) → Ade `--mode node` forges from the recovered tip on `ChainDb::tip`, cert-free → a real non-producing Haskell relay (`cardano-node 11.0.1`) follows.

## Result — DC-NODE-20 passed live, decisively
- Ade **forged 20** cert-free blocks on its own local durable spine.
- A real **non-producing** Haskell relay (`TraceForge=0`) **adopted 12** of them (`AddedToCurrentChain`, **> k=5**).
- The relay tip reached **block 27**; ~11 Ade blocks settled k-deep (immutable, **> k**).
- Ade **crossed 1 follow-link EOF** (`admission_wire_pump … exit=Eof`) and **kept forging** — the run-4 `no_tip_available` stall is **gone**.
- **Cert-free, confirmed live:** the running `--mode node` command carried `--single-producer-venue` and **no** `--adoption-cert-path`; the binary rejects `--adoption-cert-path` (`unknown flag`); the only certificate in any log is the **KES opcert**.

This is the major result: the cert-free local-tip architecture (the PHASE4-N-AH pivot) was the right direction, and it works on a real wire.

## Per-claim table (against `S4-operator-live-acceptance.md` §12)

| # | Claim | Verdict | Method |
|---|---|---|---|
| 1 | catch up once | ✅ confirmed | `recover.jsonl`: `admission_started`→`bootstrap_complete`; snapshot+WAL |
| 2 | self-admit 1st own block via `pump_block` | ⚠️ implied only | inferred from 12 sequential adoptions; **no direct event** |
| 3 | direct local-tip extend mode | ⚠️ implied only | sustained forging, no stall; **`forge_mode` not logged** |
| 4 | forge from `ChainDb::tip`, no cert | ⚠️ partial | 20 cert-free forges ✓; **forge base not logged** |
| 5 | real non-producing relay adopts | ✅ confirmed | relay `AddedToCurrentChain=12`, `TraceForge=0` |
| 6 | ≥1 follow-link EOF crossed, forging continued | ✅ confirmed | `admission_wire_pump … exit=Eof` ×1, sustained past |
| 7 | > k blocks settle immutable | ✅ confirmed | relay tip 27, immutable ~22 ⇒ ~11 Ade blocks k-deep |
| 8 | warm-start byte-identical | ❌ not run | the harness has no kill+warm-start step |
| ¬1 | no `--adoption-cert-path` flag | ✅ confirmed | live cmd + binary rejects the flag |
| ¬2 | no adoption cert read by Ade | ✅ confirmed | only "cert" = KES opcert; no parser (S2) |
| ¬3 | no cert file in forge authority | ✅ confirmed | `ci_check_cert_evidence_only.sh` green; cert-write neutralized |

## Gaps (why this is partial, not close)
1. **No JSONL transcript from `--mode node`.** The `--log` JSONL file is never created; the only events are minimal stdout `forge_result{outcome}` lines — no forge-base/mode/parent/caught-up/warm-start fields. So claims 2/3/4 — *the exact invariant under test* — are only **implied** by the relay's adoption, not directly witnessed.
2. **Warm-start (claim 8) not exercised** — the harness ran no kill+warm-start leg.

## Run-2 plan (to close S4 honestly)
1. **S4a (code, RED evidence only — no authority change):** emit a closed-vocabulary `--mode node` live transcript carrying `forge_self_admit_via_pump_block`, `forge_mode_entered=SingleProducerExtendOwnDurableSpine`, `forge_base_source=local_chaindb_tip`, `forge_base_hash`, `forge_base_block_no`, `followed_peer_tip`, `cert_path_present=false`.
2. **Harness (operator scratch):** add a kill + warm-start + immutable-depth leg — forge past EOF → settle > k → kill Ade → warm-start from the same store → confirm recovered durable tip / served chain / ledger fingerprint.
3. **Re-run** → a transcript that `ci_check_phase4_n_ah_live_evidence.sh` validates against the full 8+3 bar → **CE-AH-6 close**.

The hermetic S3 (`local_spine_kill_warm_start_byte_identical`) already proves warm-start byte-identity, so run-2 is instrumentation + one live re-run, not new core risk.

## Provenance
Raw logs preserved in operator scratch `~/.cardano-rung1-host/s4-run1-20260608T092955Z/` (`node-run.log`, `c2-relay-final.log`, `recover.jsonl`, `relay-tip-final.json`). The rung1-auto harness is operator scratch and is **not** committed (competition secrecy). No keys, hostnames, or external addresses appear here.
