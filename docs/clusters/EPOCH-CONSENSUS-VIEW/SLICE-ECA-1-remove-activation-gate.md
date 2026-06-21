# SLICE ECA-1 — remove the semantic activation gate

Part of EPOCH-CONTINUITY-ACTIVATION. Built on the whole EVIEW activation machinery
(S3f-4a..4d-wire-3b-2) + ECA-0a/0b (the leadership-complete candidate). ECA-1 removes the dev
scaffold that decided WHETHER the epoch-view activation occurs and makes activation purely a
function of canonical durable state + the deterministic predicate.

## The correction (user, 2026-06-21)

`EVIEW_ACTIVATION_ARMED: bool = false` (+ the `armed` param threaded through
`EviewActivationInputs::maybe_activate` → `maybe_activate_first_boundary`, + the "controlled-build
flip at the boundary") is a **SEMANTIC GATE** — a runtime/build switch deciding whether a
consensus transition happens. That is forbidden as production architecture (closed semantic
surfaces; replay must decide identically every run). It was acceptable ONLY as temporary scaffold
to prove byte-identical current behavior while the machinery landed, and must be removed by the
activation slice, never preserved. A continuous producer cannot require a human/flag/build to arm
it at each epoch boundary. See `feedback_no_semantic_activation_gate`.

## Design

1. **Delete `EVIEW_ACTIVATION_ARMED`** (`epoch_wire.rs`). NO equivalent flag / env var / build
   feature / CLI option replaces it.

2. **Drop the `armed: bool` parameter** from `EviewActivationInputs::maybe_activate` and
   `maybe_activate_first_boundary`, and the `if !armed { return Ok(None); }` short-circuit. The
   relay loop's call site (`node_lifecycle.rs::maybe_activate_epoch_boundary`) stops passing the
   const.

3. **The gate is now ONLY the deterministic predicate over canonical durable state.**
   `maybe_activate_first_boundary` proceeds to `try_activate_at_boundary` whenever the seed
   epoch's window is COMPLETE (the durable tip located in a later epoch via `era_schedule.locate`
   — never the wall clock) AND no view is promoted yet (idempotent). `try_activate_at_boundary`
   then runs the sole authoritative pipeline (extract durable window → readiness witness →
   fresh-materialize → `activate_at_boundary`: derive → predicate → durable WAL → atomic promote),
   failing closed (terminal `ActivationError`) on any error and `NotYet` on a predicate decline.

4. **The remaining "off" is canonical-state-keyed, not a flag.** `maybe_activate_epoch_boundary`
   short-circuits only when `(eview_activation, reduced_checkpoint)` is not `(Some, Some)` — i.e.
   when EVIEW is not configured (the cert-state package + reduced checkpoint are absent = canonical
   durable state). This is the legitimate byte-identical guarantee for non-EVIEW nodes.

5. **The path stays inert after ECA-1, but only because the inputs are not yet wired.** The relay
   loop still binds `eview_activation = None` (a hardcoded placeholder). Constructing it
   deterministically from canonical state when the EVIEW package is present is **ECA-2**. So ECA-1
   is byte-identical (inert via an un-wired `None`), with the forbidden flag gone — inert by
   un-finished wiring, never by a semantic gate.

## Invariant

- **DC-EPOCH-13 (new):** no build- or runtime-level switch decides whether the epoch-view
  activation occurs. There is no `EVIEW_ACTIVATION_ARMED` (or equivalent flag / env / build
  feature / CLI option) anywhere in the source. Activation is gated ONLY by the deterministic
  activation predicate over canonical durable state (candidate exists + bindings match the
  selected chain + source window complete + readiness valid + activation WAL durable ⇒ promote;
  else ⇒ structured terminal halt). The non-EVIEW byte-identical guarantee keys on canonical state
  (the EVIEW cert-state package / reduced checkpoint absent ⇒ no EVIEW), never a flag. Every replay
  of the same durable inputs makes the same activation decision.
- DC-EPOCH-11 strengthened (the -wire arming gate it introduced is removed; the relay-loop
  activation call is now flag-free and keyed on canonical state).

## Tests + CI

- `maybe_activate_first_boundary` is a strict no-op pre-boundary (the tip still in the seed
  epoch), seed view untouched.
- **Decisive (the gate is gone):** with the boundary CROSSED (the tip located in a later epoch)
  and the inputs present, `maybe_activate_first_boundary` AUTOMATICALLY drives the authoritative
  activation — here it fails closed (terminal `ActivationError`) on an empty durable window,
  proving it proceeds rather than no-opping on a flag, and the seed view stays authoritative.
- `ci/ci_check_eview_automatic_activation.sh` (DC-EPOCH-13): negative-greps that
  `EVIEW_ACTIVATION_ARMED` / an `armed` param / an `if !armed` guard appear NOWHERE in `crates/`;
  positive-asserts the canonical-state gating `(Some(inputs), Some(live))` + boundary detection
  (`era_schedule.locate`) + the predicate (`activate_at_boundary`).
- `ci/ci_check_eview_live_checkpoint.sh` checks (18)/(19) updated: the arming-flag assertions are
  replaced with the deterministic-gating assertions (boundary detection + the idempotent promoted
  check + canonical-state gating at the call site).

## Out of scope

ECA-2 (construct `EviewActivationInputs` deterministically from canonical state — the path goes
live-capable then), ECA-3 (the atomic `ActiveEpochAuthority` swap feeding BOTH header validation
and leadership), ECA-4 (warm-start recovery), ECA-5 (the live boundary proof). ECA-1 only removes
the forbidden gate and locks it out mechanically; the live follow/forge path stays byte-identical.
