# Invariant Slice — PHASE4-N-AC S1: evolve KES key to current period before signing

## §2 Slice Header
- **Slice Name:** evolve KES key before sign
- **Cluster:** PHASE4-N-AC (KES signing evolves key to current period) — primary invariant **DC-CRYPTO-10**
- **Status:** Merged
- **Cluster Exit Criteria Addressed:** **CE-1** (the whole one-slice cluster). *(Acceptance #3 = the live C1 re-run at close.)*

## §3 Dependencies
N-P (`kes_update` / `kes_advance_to` / the BLUE Sum6KES `update_kes`). N-S-A (CN-KES-HEADER-01, the two-pass real-pre-image sign at `produce_mode.rs:891`). N-Q/N-R (`ProducerShell` `&mut` plumbing in the forge).

## §4 Intent (invariant impact)
The forge's only real KES sign requires the key already be at the requested period; it never evolves the minted-at-period-0 key forward, so forging fails the moment the chain's KES period > 0. This slice makes the RED shell evolve the key to the requested period before signing — closing DC-CRYPTO-10 — so the forge works across KES periods within the opcert window.

## §5 Scope / What is built
- **NEW `ProducerShell::kes_sign_header_advancing(&mut self, period: u32, preimage: &UnsignedHeaderPreImage) -> Result<KesSignature, ShellSignError>`** (`producer_shell.rs`):
  ```
  self.kes_advance_to(period)?;        // forward evolution; idempotent at current; fail-closed
  self.kes_sign_header(period, preimage)
  ```
  `kes_advance_to → kes_update` already enforces every case: `period == current` → no-op (the `while current < to` loop); `period < current` → `EvolutionBackwards`; `period > current + evolutions_remaining` → `EvolutionExhausted`. The requested `period` is passed verbatim (no `±1`). After a successful advance, `current == period`, so `kes_sign_header → kes_sign_at` passes.
- **Wire `produce_mode.rs:891`**: `shell.kes_sign_header(kes_period, &preimage)` → `shell.kes_sign_header_advancing(kes_period, &preimage)`. The forge already holds `&mut ProducerShell`; the existing `kes_period_in_window` pre-check (`:781`) stays.
- **NEW gate** `ci/ci_check_kes_evolution_before_sign.sh`.
- **Out of scope:** any change to `kes_sign_at`/`kes_sign_header` (the `&self`, no-evolve variants stay for callers that manage periods); the BLUE Sum6KES algorithm; the KES verifier; forge eligibility; wire rules; cross-period sustained production (separate cluster).

## §6 Execution Boundary (TCB color)
- **RED (changed):** `ade_runtime::producer::producer_shell` (the new method) + `ade_node::produce_mode` (the `:891` call site). Signing = RED key custody (T-KEY-01 preserved).
- **BLUE/core (reused, NOT edited):** `ade_crypto::kes_sum` (Sum6KES `update_kes`), the KES verifier, `kes_update`/`kes_advance_to` logic.
- **No new canonical type, no schema change, no BLUE semantic change.**

## §7 Invariants Preserved
T-KEY-01 (signing in RED). Forward-secrecy (one-way evolution; past periods unsignable — backwards fail-closed). Determinism (Sum6KES evolution is deterministic; same key + period → same evolved key + signature). The opcert-window pre-check (`kes_period_in_window`) + the `kes_update` bounds are retained.

## §8 Invariants Strengthened or Introduced
**DC-CRYPTO-10 → enforced** (4 shell tests + the gate + the live C1 re-run). At close: `strengthened_in += "PHASE4-N-AC"` on CN-KES-HEADER-01 (the real-pre-image sign now works across periods).

## §11 Replay / Crash / Epoch Validation
KES evolution + signing are deterministic pure functions; no WAL/checkpoint/schema surface. The forward-secrecy property holds (evolution is monotone; the shell cannot re-sign a destroyed past period).

## §12 Mechanical Acceptance Criteria (CE-1)
`cargo test -p ade_runtime` green incl. the 4 shell tests:
1. `shell_kes_sign_header_advancing_evolves_then_signs` — period-0 shell, `kes_sign_header_advancing(1, preimage)` → Ok; the signature verifies at KES period 1; `kes_current_period() == 1` afterward.
2. `shell_kes_sign_header_advancing_at_current_period_signs` — period-0 shell, `kes_sign_header_advancing(0, preimage)` → Ok (no-op evolution); verifies at period 0 (existing period-0 signing still works).
3. `shell_kes_sign_header_advancing_backwards_fails_closed` — advance to 5, then `kes_sign_header_advancing(2, preimage)` → `Err(Signing(EvolutionBackwards{..}))`; no signature.
4. `shell_kes_sign_header_advancing_beyond_lifetime_fails_closed` — period-0 shell, `kes_sign_header_advancing(SUM6_MAX_PERIOD + 1 = 64, preimage)` → `Err(Signing(EvolutionExhausted{..}))`.
Gate `ci/ci_check_kes_evolution_before_sign.sh` green + non-vacuous (pre-S1 `:891` used the non-evolving `kes_sign_header`): (a) the forge real-sign uses `kes_sign_header_advancing`; (b) the method calls `kes_advance_to` before `kes_sign_header` with the period verbatim (no `±1`); (c) `kes_update` keeps the backwards + exhausted checks.
Cluster-wide: `cargo test -p ade_runtime -p ade_node` green; full `ci/ci_check_*.sh` sweep 137 + 1 = **138 / 0**.

## §14 Hard Prohibitions (cluster §11)
No signing in BLUE. No skipping KES period checks. No accepting stale-period signatures (after advance `current == period`; backwards fail-closed). No manually mutating the requested period (verbatim). No C1-only key path. No RO-LIVE flip.

## §15 Explicit Non-Goals
Cross-period sustained / multi-epoch production (epoch-nonce roll, opcert renewal — separate cluster); any change to the BLUE Sum6KES algorithm or the KES verifier; the non-evolving `kes_sign_at`/`kes_sign_header` (retained).
