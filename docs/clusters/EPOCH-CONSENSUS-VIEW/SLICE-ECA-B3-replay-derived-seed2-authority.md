# ECA-B3 — replay-derived seed+2 authority (generalize the activation seam past boundary 1)

**Rule:** DC-EPOCH-17 (declared by this slice). **Cluster:** EPOCH-CONSENSUS-VIEW. **Depends on:**
DC-EPOCH-16 (enforced — the self-evolved `eta0(seed+2)` is proven live), DC-EPOCH-15 (the ECA-5
first-boundary bridge), DC-EPOCH-13/14 (automatic activation, in-place promotion).

## The gap

`eta0(seed+2)` is proven (DC-EPOCH-16, live), but Ade still cannot **validate** seed+2 blocks: header
validation needs both the epoch nonce (have it) AND the **leadership authority** (the stake
distribution + pool VRF keyhashes) for that epoch. The activation seam
(`epoch_wire.rs::prepare_authority_for_candidate_slot`) is welded to boundary 1:

- It promotes the N+1 authority from the **bridge** (the imported MARK snapshot, lines 552-613).
- It is **idempotent after one promotion** (`if active_view.is_promoted() { return Ok(false) }`,
  line 512) and gated on `tip_epoch == seed_epoch` (521) + `candidate_epoch == seed_epoch + 1` (532).

So after boundary 1 the seam is inert forever; the first seed+2 candidate fails
`OutsideForecastRange` (the forecast never extends past N+1) — the Tier-B ceiling.

## The architecture (already established)

`authority(epoch E) = replay(E-2 window)` (the leadership snapshot lag = 2; see
`epoch_wire.rs:547-548`). A node bootstrapped at N:

- **Boundary 1 (N→N+1):** no N-1 window exists, so the **bridge** supplies seed+1 (PROVEN, ECA-5).
- **Boundary k≥2 (N+k-1 → N+k):** the N+k-2 window is durable (Ade followed it), so
  `try_activate_at_boundary` over that window derives the **seed+k** authority. For boundary 2 that
  is `replay(N) → seed+2`. The reduced-window driver (`reduced_window_driver.rs`) already applies the
  per-boundary POOLREAP/re-registration so the replayed cert-state/stake is correct.

So both halves exist: the **window-replay** authority (`try_activate_at_boundary`) and the proven
**eta0** (the evolved chain-dep, DC-EPOCH-16). B3 wires them onto the live follow path past boundary 1.

## Design

Generalize `prepare_authority_for_candidate_slot` from a one-shot bridge promotion into a
**per-boundary advance**:

1. **Detect the boundary generally.** Replace the `tip_epoch == seed_epoch` weld with: the candidate
   slot is in epoch `C`, the currently-promoted authority is for epoch `P` (or the seed for the
   pre-boundary-1 state), and `C == P + 1` (exactly one boundary ahead; `C > P + 1` is the terminal
   `CandidateSlotSkipsBoundary`). The durable tip's parent must still bind the candidate (guard (c),
   unchanged).
2. **Choose the source by boundary index, deterministically:**
   - `C == seed_epoch + 1` → the **bridge** (DC-EPOCH-15, unchanged; proven).
   - `C ≥ seed_epoch + 2` → **window-replay**: `try_activate_at_boundary` over the `C-2` window (the
     durable reduced checkpoint + canonical `C-2` window blocks + the v4 sidecar geometry), yielding
     the seed+(C-N) leadership. The epoch nonce for the bound view is the **evolved chain-dep
     `epoch_nonce`** at the boundary (DC-EPOCH-16), NOT a bridge precompute.
3. **Advance the active view in place.** `ActiveEpochAuthority` gains an `advance(source, projected)`
   that replaces the promoted authority with the next epoch's (P → P+1), durable-before-visible (WAL
   activation record first, exactly as `promote`). Header validation + leadership keep resolving the
   single current holder (DC-EPOCH-14).
4. **Extend the forecast** to the new epoch (already done by `extend_schedule_to_epoch`).

The seam stays AUTOMATIC + DETERMINISTIC (no arming flag): the only gate is the candidate-slot/tip
predicate over canonical durable state.

## Proof obligations (slice-entry)

- **PO-B3-1 (lag).** Confirm `try_activate_at_boundary` over the `C-2` window yields the authority for
  epoch `C` (the MARK→SET→GO snapshot lag), so boundary 2 reads `replay(N)`, not `replay(N+1)`. Pin
  against the cardano `NewEpochState` snapshot semantics (pstate `psStakeMark/Set/Go`).
- **PO-B3-2 (eta0 source).** The window-replay-bound view's `epoch_nonce` must be the **evolved
  chain-dep** value (DC-EPOCH-16), not a recomputed/bridge nonce — the live gate proved that value;
  binding a different one would re-introduce the boundary-2 nonce divergence.
- **PO-B3-3 (replay-determinism).** The promoted seed+2 authority is a pure function of the durable N
  window (blocks + reduced checkpoint + sidecar) — same durable state ⇒ byte-identical bound view +
  WAL record (warm-start re-derives identically — that is B4's gate, but the determinism is asserted
  here).
- **PO-B3-4 (fail-closed).** A missing/short `C-2` window, a candidate skipping a boundary, or a
  parent not binding the durable tip is TERMINAL (no silent bridge fallback past seed+1, no Origin
  fallback).

## Mechanical acceptance (CE)

- **CE-B3-1:** `prepare_authority_for_candidate_slot` promotes at boundary 2 (candidate in N+2, tip in
  N+1) from the window-replay over the N window; hermetic test with a synthetic 2-epoch durable window.
- **CE-B3-2:** `advance` replaces P with P+1 in place, durable-before-visible (WAL record precedes the
  view publish); idempotent re-call at the same boundary is a no-op.
- **CE-B3-3:** the bound view's `epoch_nonce` equals the evolved chain-dep `epoch_nonce` at the
  boundary (DC-EPOCH-16 coupling).
- **CE-B3-4:** boundary-skip / missing-window / parent-mismatch are terminal (negative tests).
- **CE-B3-5 (gate, live):** Ade **validates + admits** seed+2 blocks against the replay-derived seed+2
  authority + `eta0(seed+2)` — the venue run that crosses N+1→N+2 in the relay loop without
  `OutsideForecastRange` / `VrfCert` / fail-close. (Operational detail in the untracked runbook.)

## Out of scope (later Tier B)

- **B4:** warm-start recovery — re-derive the identical seed+2 authority + eta0 from durable followed
  blocks across a restart (PO-B3-3's determinism is the hook); durable RSW in the sidecar.
- **B5:** the full unattended forge-off crossing proof (a producer crossing the boundary).
