# PHASE4-N-AE.F — receive idempotency (post-adoption echo)

Scope source: `docs/planning/phase4-n-ae-f-echo-idempotency-invariants.md` (the `/invariants`
sketch). This slice doc grounds the design in the actual chokepoint and refines the sketch's
TCB placement.

## §2 Cluster Exit Criteria addressed
- Post-CE-A5 durability: a continuous `--mode node` run survives the relay re-announcing
  Ade's own adopted tip (the `SlotBeforeLastApplied` exit-43 observed after the CE-A5
  manifest). Prerequisite for long-running C2-LOCAL / preprod-style relay operation.

## §3 Implementation instruction (AI)
In `crates/ade_runtime/src/forward_sync/pump.rs`, `pump_block`: immediately after
`decode_block`, query `db.get_block_by_hash(&decoded.block_hash)`. If it returns
`Some(stored)` AND `stored.slot == decoded.header_input.slot`, return `Ok(None)` (idempotent
no-op) — do NOT run the RollForward/BlockDelivered reducer steps. Otherwise proceed exactly as
today. Nothing else changes; the BLUE chokepoint reducer and `validate_and_apply_header` are
untouched.

## §4 Intent (invariant impact)
A peer re-announcing a block Ade has **already durably applied byte-identically** must be an
idempotent no-op, not a fail-close. The CE-A5 run produced exactly this: after the relay
adopted Ade's forged block 17, it served block 17 back over Ade's follow link, and the BLUE
header authority correctly rejected `SlotBeforeLastApplied { last: 421, attempted: 421 }` —
terminating the run.

## §5 Design (refines the sketch §5)
The discriminator is membership in the **durable ChainDb**, keyed by the block hash. The
sketch proposed a BLUE `ReceiveOutcome::AlreadyHave` with membership threaded in as a
canonical input. The chokepoint already holds `db` and already reads/writes it, and
`get_block_by_hash` is a **deterministic** query over the accumulated durable chain (no
wall-clock / rand / OS-ordering) — so the gate lives at the RED chokepoint with NO BLUE
change and NO new reducer input:

- `get_block_by_hash(hash)` is **hash-exact** by construction — `Some` iff Ade holds exactly
  this block. A DIFFERENT block (different hash) at/before the last-applied slot returns
  `None` here, falls through to the BLUE reducer, and fails closed as today
  (`SlotBeforeLastApplied` / `BlockNoOutOfOrder`). The `stored.slot == decoded.slot` check is
  a cheap consistency belt-and-braces.
- The no-op path runs **no** reducer step, so it appends **nothing** to the WAL and writes
  **nothing** to ChainDb — the post-state `(ledger, chain_dep, ChainDb, WAL)` is identical.
  Replay-equivalent: warm-start replays the WAL, which never recorded the skipped re-announce.
- `Ok(None)` is the existing "no tip advanced" outcome; every genuinely-applied block advances
  the tip (`Some`), so callers (`run_node_sync`, `admit_forged_block_durably`) already treat
  `Ok(None)` as "continue, tip unchanged" — no caller ripple.

## §6 Execution boundary (TCB color)
- `crates/ade_runtime/src/forward_sync/pump.rs` — **RED** (the durable-admit shell chokepoint;
  already does ChainDb/WAL I/O). The added step is a deterministic ChainDb read + an early
  `Ok(None)`. No BLUE module changes; `ade_ledger::receive` + `ade_core::consensus::header_validate`
  are untouched.

## §7 Invariants preserved
- The BLUE header/body authority (`validate_and_apply_header`, `block_validity`) is unchanged —
  it still fail-closes every block that reaches it. DC-SYNC-01 (durable-before-tip),
  DC-NODE-12 (durable admit via pump_block), DC-WAL-02 / T-REC-05 (WAL chaining / replay) all
  hold — the no-op touches none of them.
- AE-F-INV-2 (the fail-closed boundary) is the load-bearing preserve: a different block at/before
  the last-applied slot is NOT short-circuited.

## §8 Invariants strengthened
- **DC-NODE-16** (introduced, enforced) — receive idempotency: a peer-re-announced block already
  durably present byte-identically (same slot, same hash) is an idempotent no-op at the
  durable-admit chokepoint; a different block at/before the last-applied slot stays fail-closed.

## §10 Changes introduced
- `crates/ade_runtime/src/forward_sync/pump.rs` — the hash-exact already-have gate + tests.
- `docs/ade-invariant-registry.toml` — DC-NODE-16 (new).
- `ci/ci_check_receive_idempotency.sh` — fences the hash-gated no-op (no slot-only skip).

## §11 Replay / crash / epoch validation
The no-op appends nothing to the WAL, so warm-start replay is unaffected — proven by the
existing recover→follow→warm-start replay tests staying green plus the new no-op test asserting
zero WAL growth.

## §12 Mechanical acceptance criteria
- **CE-F1** — `pump_block_reannounced_block_is_idempotent_noop`: pump a block, then pump the
  SAME bytes again → the second call returns `Ok(None)`; ChainDb tip, ledger fingerprint, and
  WAL length are unchanged by the second call.
- **CE-F2** — `pump_block_different_block_at_or_before_tip_still_fails_closed`: after applying a
  block at slot S, pumping a DIFFERENT block (different hash) at slot ≤ S returns
  `Err(PumpError::Receive(Validity(Header(SlotBeforeLastApplied|BlockNoOutOfOrder))))`.
- **CE-F3** — `ci/ci_check_receive_idempotency.sh`: the gate is hash-keyed
  (`get_block_by_hash`), not slot-only, and the already-have arm runs no reducer step / appends
  no WAL.
- **CE-F4** (live-shape, hermetic) — `recover_follow_forge_then_reannounce_own_tip_is_noop`:
  recover → forge a successor (durably admit it) → pump the forged block's own bytes back →
  `Ok(None)`, tip stable, no error (the CE-A5 echo, reproduced and survived).

## §14 Hard prohibitions
- No slot-only skip (must be hash-keyed). No skip-past a gap. No fork-choice (DC-CONS-03
  untouched). No weakening of `SlotBeforeLastApplied` for a different block. No BLUE change.

## §15 Explicit non-goals
- Multi-producer fork-choice / chain selection.
- Accepting a *better* competing chain (a genuine rollback is the existing receive-rollback
  path, DC-PROTO-09; unchanged).
