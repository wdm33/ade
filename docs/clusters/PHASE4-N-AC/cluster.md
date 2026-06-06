# PHASE4-N-AC — KES signing evolves key to current period (DC-CRYPTO-10)

> **Live-readiness cleanup, surfaced by the item-4 C1 re-run (2026-06-06).** The C1 genesis rehearsal stopped reproducing once the private net aged past one KES period: Ade forges only in the KES period its key was minted at, because the forge never evolves the operator KES key forward. A real C2/preprod run will not stay in the minted period, so this is a genuine live-readiness gap, not a private-net quirk. Tracked in `project_pre_rolive_hardening_queue.md` (item 4 finding → item 5 fix).

## §1 Primary invariant (DC-CRYPTO-10)
The RED signing shell must evolve the operator KES signing key to the requested KES period before signing, using the existing deterministic Sum6KES update primitive. It must fail closed if the requested period is before the key start, beyond the key lifetime, or cannot be reached by sequential evolution.

## §2 The problem (proven from code + the live run)
- The forge's only real KES sign is `produce_mode.rs:891` `shell.kes_sign_header(kes_period, &preimage)`. `kes_sign_header` → `kes_sign_at` (`producer_shell.rs:196`) which requires `period == kes.current_period()`, else `ShellSignError::KesPeriodNotCurrent`.
- The cardano-cli `kes.skey` is loaded at its minted period (0). Nothing in the forge calls the existing `kes_advance_to` before signing. So the moment the chain's KES period > 0, every leader slot fails `KesPeriodNotCurrent { requested: N, current: 0 }` → `ForgeFailed { KesPeriodMismatch }`.
- **Live evidence (item-4 C1 re-run, HEAD `6184eb0b`):** the net crossed into KES period 1 (slot ~249659 / `slotsPerKESPeriod` 129600); ~5 leader wins in 93 ticks (ASC 0.05), all `failed`, `succeeded=0`; the follower handshook + found the chain-sync intersection then KeepAlive-timed-out waiting for a block that never came. The wire/serve path (incl. N-AA/N-AB) was healthy; the failure is isolated to KES signing. The two prior C1 successes (Jun 4) were both in KES period 0.

## §3 The design — the existing pieces compose; the forge just never used them
All the machinery already exists and is unit-tested:
- `kes_advance_to(&mut self, to_period)` (`producer_shell.rs:210`) → `kes_update` (`signing.rs:267`), the deterministic Sum6KES forward evolution. `kes_update` is **idempotent at the current period** (`while current < to`), and **fail-closed**: `to < current` → `EvolutionBackwards`; `to > current + evolutions_remaining` → `EvolutionExhausted`.
- `kes_current_period()` (`:279`), `kes_period_in_window()` (`:253`), `kes_sign_header()` (`:270`).

**Fix:** add `ProducerShell::kes_sign_header_advancing(&mut self, period, preimage) = kes_advance_to(period)? ; kes_sign_header(period, preimage)` and wire `produce_mode.rs:891` to it. The forge already holds `&mut ProducerShell` (`run_real_forge_inner` `:682`), so no mutability threading. `kes_advance_to` already enforces every fail-closed case + the no-op-at-current case, so the new method is a thin, total composition.
- The forge's existing `kes_period_in_window` pre-check (`:781`) stays (it also rejects `period > opcert_last` and backwards before the placeholder pass).
- Signing stays RED; the KES verifier stays BLUE/core; no consensus rule, forge-eligibility, or wire change; no private-net / C1-only path.

## §4 Normative anchors
- Registry — adds **DC-CRYPTO-10** (tier derived; declared → enforced at S1).
- Cross-ref: **CN-KES-HEADER-01** (the real-pre-image KES sign this evolves before), **T-KEY-01** (signing confined to RED — preserved), **DC-CRYPTO-04..09** (the BLUE Sum6KES algorithm + key custody), **CN-FORGE-03** (forge/self-accept).
- Source: `docs/evidence/c1-genesis-rehearsal-reproduction-README.md` (the regression target) + `project_pre_rolive_hardening_queue.md` item 4 finding.

## §5 Entry conditions (what prior clusters guarantee)
- N-P: the BLUE Sum6KES `update_kes` algorithm (byte-for-byte cardano-base) + `kes_update` + `kes_advance_to` (all tested).
- N-S-A (CN-KES-HEADER-01): the two-pass real-pre-image KES sign in the forge (the single real sign at `:891`).
- N-Q/N-R: `ProducerShell` key custody (RED), the forge `&mut shell` plumbing.

## §6 TCB color map (FC/IS partition)
- **RED (changed):** `ade_runtime::producer::producer_shell` (new `kes_sign_header_advancing`), `ade_node::produce_mode` (the `:891` call site). Signing is RED key custody.
- **BLUE/core (reused, NOT edited):** the Sum6KES algorithm (`ade_crypto::kes_sum`), `kes_update`/`kes_advance_to` logic, the KES verifier. No consensus / eligibility / wire change.
- **No new canonical type, no schema change, no BLUE semantic change.**

## §7 Slices
| Slice | Scope | CE | Registry → enforced | TCB |
|---|---|---|---|---|
| **S1** | `ProducerShell::kes_sign_header_advancing` (evolve-to-period-then-sign; fail-closed via `kes_advance_to`/`kes_update`) + wire `produce_mode.rs:891`. New gate `ci_check_kes_evolution_before_sign.sh`. | CE-1 | DC-CRYPTO-10 | RED |

*(The C1 re-run — acceptance #3 — is live verification at the close, not a code slice.)*

## §8 Cluster Exit Criteria (all mechanical)
- **CE-1 (S1, DC-CRYPTO-10):** `cargo test -p ade_runtime` green incl. the 4 shell tests:
  1. `shell_kes_sign_header_advancing_evolves_then_signs` — period-0 key + requested period 1 → evolves to 1 + signs; the signature verifies at period 1. *(acceptance #1)*
  2. `shell_kes_sign_header_advancing_at_current_period_signs` — requested period == current (0) → no-op evolution + signs + verifies. *(acceptance #4: existing period-0 signing still works)*
  3. `shell_kes_sign_header_advancing_backwards_fails_closed` — advance to 5, then request 2 → fail-closed (`Signing(EvolutionBackwards)`); no signature. *(acceptance #2: before key start / backwards)*
  4. `shell_kes_sign_header_advancing_beyond_lifetime_fails_closed` — request `SUM6_MAX_PERIOD + 1` (64) → fail-closed (`Signing(EvolutionExhausted)`). *(acceptance #2: beyond lifetime / unreachable)*
- **CE-1 gate:** `ci/ci_check_kes_evolution_before_sign.sh` — (a) the forge's real KES sign uses the evolving `kes_sign_header_advancing`, NOT the raw `kes_sign_header`/`kes_sign_at`; (b) `kes_sign_header_advancing` calls `kes_advance_to` before `kes_sign_header` and passes the period verbatim (no `period + 1` / `period - 1` mutation); (c) `kes_advance_to`/`kes_update` retain the backwards + exhausted fail-closed checks; (d) signing stays in RED `ade_runtime` (no `kes_sign`/`SigningKey` in BLUE — the existing `ci_check_no_signing_in_blue.sh` is the standing fence).
- **Live (acceptance #3, close-time):** the C1 re-run (`docs/evidence/c1-genesis-rehearsal-reproduction-README.md`) in KES period 1 — Ade now forges (evolves 0→1) + the follower `ValidCandidate`+`AddedToCurrentChain`s + `correlate` → fresh `PrivateRehearsalManifest`. NO net reset.
- **Cluster-wide:** `cargo test -p ade_runtime -p ade_node` green; full `ci/ci_check_*.sh` sweep 137 + 1 = **138 / 0**.

## §9 Replay obligations
KES evolution is deterministic (`Sum6Kes::update_kes`, byte-for-byte cardano-base; same key + same target period → same evolved key + same signature). Signing stays a pure function of (key-at-period, message). No canonical type / WAL / schema change. The forward-secrecy property is preserved (one-way evolution; past periods unsignable).

## §10 Invariants
- **Adds:** DC-CRYPTO-10 (declared → enforced at S1).
- **Strengthens** (`strengthened_in += "PHASE4-N-AC"` at close): CN-KES-HEADER-01 (the real-pre-image sign now works across KES periods, not only the minted one).
- **Preserves (NOT changed):** T-KEY-01 (signing in RED), the BLUE Sum6KES algorithm + KES verifier, CN-FORGE-* eligibility, all wire rules.

## §11 Forbidden during this cluster (hard boundaries — user-set)
- No signing in BLUE.
- No skipping KES period checks (the `kes_period_in_window` pre-check + `kes_update` bounds stay).
- No accepting stale-period signatures (after evolution `current == period`; backwards is fail-closed, never signed at a past period).
- No manually mutating the requested period (evolve to the period the forge computed, verbatim — no `±1`).
- No C1-only key path (the same `--mode node` forge path serves C1 and C2).
- No RO-LIVE flip (C1 is a non-promotable rehearsal; this is a signing-correctness fix, not a live-acceptance claim).

## §12 Open questions
- **C1 re-run as the acceptance proof (resolved):** acceptance #3 is the live C1 re-run in KES period 1 — performed at close against the running `cardano-node-c1` (no net reset). It is non-promotable rehearsal evidence (`PrivateRehearsalManifest`), not a bounty/RO-LIVE claim.
- **Cross-period sustained production (out of scope):** this fixes signing within the opcert's KES window across periods; sustained multi-epoch production (epoch-nonce roll, opcert renewal) remains a separate larger cluster (per the C1 scoping §4a).

## §13 Close record
*(Open — filled at `/cluster-close` once CE-1 + the C1 re-run are green.)*
