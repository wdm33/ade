# Invariant Slice S14 — Missing-bridge range re-fetch for winner-descendant recovery

## 2. Slice Header

- **Cluster:** PHASE4-N-AO (live multi-candidate fork-choice SELECT + adopt).
- **Type:** liveness/convergence fix slice (turns the DC-NODE-39 safe halt into an
  active, validated catch-up path).
- **Depends on:** S6 (live BlockFetch `prefetch_branch_bodies`, CE-AO-6), S11 floor
  (DC-NODE-39, the MissingBridge hold), S13 (DC-NODE-40, rolled-back retention).
- **Declares:** `DC-NODE-41` (missing-bridge range re-fetch).
- **Cluster Exit Criteria addressed:** CE-AO-6 — closes Fault 2 (a winner-descendant
  whose bridge Ade NEVER received: ChainSync streams each block once and will not
  re-send a passed block, so the passive floor stalls forever).

## 4. Intent

The DC-NODE-39 floor is **safe** (no mis-admit, no silent stall, observable hold) but
**not live**: passive waiting cannot recover a bridge ChainSync already streamed past.
A stable Cardano-compatible node cannot rely on the peer to re-send a missed block; when
Ade misses the bridge to a *winning-peer* descendant, it must **actively re-fetch** the
missing range and admit it in parent-link order. The floor remains the safety fallback
when the range cannot be fetched.

The S12 harness proved the passive floor does not recover (`late_bridge_recovers_on_progress`:
the late bridge admits, but the un-bridgeable descendant `tip != z_hash` is never admitted;
the dispatch issues no re-fetch). That deterministic evidence is sufficient to define the
invariant — a live run is where the fix is PROVEN, not where it is justified.

## 5. Scope

On a `MissingBridge` for a **post-ForkChoiceWin winning-peer descendant** (the peer Ade
just adopted from, a descendant of the adopted tip X):
1. **Hold the fence** (DC-NODE-39, unchanged).
2. **`BlockFetch RequestRange(X+1 .. descendant)`** to that winning peer — reusing S6's
   byte-only `prefetch_branch_bodies` (`crates/ade_node/src/node_lifecycle.rs:3539`).
3. **Receive the bridge + intermediates**; **validate parent links + bodies**; **admit in
   parent-link order through `pump_block`** (the sole admit) — X+1, then X+2, …, descendant.
4. **Clear the hold ONLY on real admitted progress** (the same DC-NODE-39 clear rule).
5. If the range cannot be served / is short / lies → **remain fail-closed** with a closed
   `MissingBridge` failure code (the floor fallback); no admit, no mutation.

**NOT a selector.** S3 (`select_best_chain`) already decided the winner. S14 is ONLY
recovery for the *selected/winning* peer's missing post-switch descendants. A losing
peer's gap is NOT S14's job unless/until fork-choice selects it — this avoids turning
every loser orphan into network fetch spam.

## 6. Execution Boundary (TCB color)

- **RED:** `BlockFetch RequestRange` + peer I/O + the bounded retry/backoff shell (reuses
  S6 `prefetch_branch_bodies`). BlockFetch provides BYTES only — it does not grant truth.
- **GREEN:** the bridge-gap recovery sequencing (detect winning-peer descendant gap →
  compute the range X+1..descendant → order the admits) + the closed structured recovery
  evidence (`range_refetch_started` / `range_refetch_completed` / a closed failure code).
- **BLUE — UNCHANGED:** header/body validation, parent-link proof, `pump_block` (the sole
  roll-forward admit), `select_best_chain`, `apply_fork_switch` adoption authority.

## 7. Invariants Preserved (registry IDs)

- `DC-NODE-39` (the MissingBridge structured fail-closed hold — S14 ADDS active recovery
  but the hold + the no-silent-stall behavior remain; the floor is the fallback).
- `DC-NODE-40` (rolled-back retention), `DC-NODE-37` (S4 prevalidate-before-commit; the
  re-fetched range is validated before admit), `DC-NODE-24/25` (receive routing, admit),
  `CN-CONS-01` (fail-closed), `DC-CONS-03` (no duplicate selector — S14 is not a selector).

## 8. Invariants Strengthened / Introduced

- **`DC-NODE-41` (introduced, declared).** *When a post-ForkChoiceWin winning peer
  presents a descendant whose parent chain is missing, Ade must EITHER re-fetch and admit
  the missing range from the adopted tip to that descendant IN PARENT-LINK ORDER (through
  `pump_block`, each body parent-link + body-hash validated), OR remain fail-closed with a
  structured MissingBridge (closed failure code); it must NOT passively stall forever and
  NOT admit out of order. The re-fetch targets ONLY the winning peer (the adopted-tip
  peer), bounded retry, byte-only fetch (BlockFetch is not authority).*

## 9. Design Summary (provisional)

After a ForkChoiceWin adoption, Ade records the adopted tip X + the winning peer (from
`PendingForkSwitch.winning_peer`) as the post-switch follow target (cross-iteration state,
e.g. in `ForgeActivation`). When `dispatch_competing_fork_choice` would set a
`MissingBridge` for a competing block FROM that winning peer that descends from X (parent
chain points toward X but is incomplete), it instead (a) emits `range_refetch_started`,
(b) requests `RequestRange(X+1..descendant)` from the winning peer via `prefetch_branch_bodies`,
(c) feeds the bytes through the existing validate→`pump_block` admit (LinearExtend order),
(d) emits `range_refetch_completed` and clears the hold on real admitted progress. A short
/ lying / unservable range leaves the structured MissingBridge hold (the DC-NODE-39 floor).
A gap from a non-winning peer takes the unchanged floor path (no re-fetch).

## 11. Replay / Crash / Epoch Validation

- Re-fetched bytes are byte-only input; the admit path (`pump_block` + WAL) is the existing
  replay-equivalent authority. Same served range → same admitted post-state.

## 12. Mechanical Acceptance Criteria

- [ ] `missing_bridge_triggers_range_refetch` — X adopted; Z (winning-peer descendant)
  arrives with a missing parent chain; Ade requests `RequestRange(X→Z)` from the winning
  peer (a `range_refetch_started` is emitted / the request is issued).
- [ ] `refetched_bridge_admits_in_order` — the fetch returns Y, Z; Ade admits Y THEN Z
  through `pump_block`; `pending_missing_bridge` clears on that admitted progress.
- [ ] `short_refetch_keeps_hold` — the fetch returns only Z (or misses Y); NO admit of Z;
  the hold remains.
- [ ] `lying_refetch_body_rejected` — a fetched Y whose body-hash / prev mismatches; NO
  mutation; the hold remains.
- [ ] `refetch_failure_structured` — the peer cannot serve the range; MissingBridge
  remains with a closed failure code (no silent stall).
- [ ] `bounded_retry` — the retry limit / backoff is a RED policy, deterministic in tests,
  no spin loop.
- [ ] `ci/ci_check_missing_bridge_refetch.sh` (DC-NODE-41): re-fetch triggers ONLY for the
  winning peer, byte-only (no admit bypass of `pump_block`), bounded retry, fail-closed
  fallback intact.
- [ ] `cargo test -p ade_node` green; the S11/S12/S13 tests + gates unregressed (the floor
  + harness + retention still hold).
- [ ] **Live (CE-AO-6):** a fresh two-producer run shows `fork_switch_applied X` →, if a
  bridge gap occurs, `range_refetch_started` / `range_refetch_completed` → the bridge +
  descendant admitted → no diverged → all wins terminal → agreement or validated-prefix
  continuation. Contributes to the `CN-CONS-03` flip.

## 13. Failure Modes

- Range unservable / short / lying → structured MissingBridge hold (fail-closed; no admit).
- Retry exhausted → MissingBridge persists with a closed failure code (no spin).
- Gap from a non-winning peer → unchanged floor (no re-fetch; no fetch spam).

## 14. Hard Prohibitions

- No admitting the descendant Z before the missing bridge Y (parent-link order only).
- No trusting a fetched range without parent-link + body validation.
- No clearing `pending_missing_bridge` because a fetch was *attempted* (only real admitted
  progress clears it).
- No clearing the fence except on real admitted progress or a resolved caught-up state.
- No using the peer-supplied endpoint as authority beyond the fetch address.
- No bypassing `pump_block` (the sole admit).
- No unbounded retry loop (bounded, deterministic).
- No deciding the branch wins — S3 already selected; S14 is recovery only, winning-peer-only.

## 15. Explicit Non-Goals

- Not a selector / fork-choice change (S3 owns selection).
- Not loser-orphan recovery (only the winning peer's post-switch descendants).
- Not removing the DC-NODE-39 floor — S14 layers active recovery on top; the floor stays
  the fail-closed fallback.

## 16. Completion Checklist

- [ ] Record the post-switch winning peer + adopted tip; gate the re-fetch on it.
- [ ] Re-fetch via S6 `prefetch_branch_bodies` (RequestRange X+1..descendant), byte-only.
- [ ] Validate + admit in parent-link order via `pump_block`; clear hold on real progress.
- [ ] Closed `range_refetch_started/completed` + failure-code evidence (GREEN, observe-only).
- [ ] 6 hermetic tests + `ci/ci_check_missing_bridge_refetch.sh`.
- [ ] `cargo test -p ade_node` + all AO gates green.
- [ ] Live CE-AO-6: bridge gap recovered → convergence; contributes to `CN-CONS-03` flip.
- [ ] `DC-NODE-41` declared → enforced at `/cluster-close`.
