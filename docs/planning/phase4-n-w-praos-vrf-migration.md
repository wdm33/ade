# PHASE4-N-W — Producer Praos VRF Authority Migration (follow-on note)

> Follow-on cluster surfaced by PHASE4-N-V's honest fallback. NOT yet scoped via
> `/invariants` — this note captures the user-specified shape so N-W starts from
> a fixed brief. Declares `CN-FORGE-04`.

## Why N-W exists

PHASE4-N-V fixed the **block-envelope** producer/validator codec asymmetry
(forge output now decodes through the validator path; `CN-FORGE-03` enforced).
With the envelope fixed, a consistent eligible-leader tick now reaches header
validation and exposes the **next** producer/validator semantic mismatch:

- **Producer** evaluates/signs leader eligibility VRF using the **TPraos**
  role-tagged alpha: `vrf_input(slot, eta0, LeaderEligibility)` = `slot‖eta0‖0x4C`.
- **Conway validator (Praos)** verifies via `verify_praos_vrf` over the **Praos**
  alpha `praos_vrf_input(slot, eta0)` = `blake2b256(slot‖eta0)` plus the
  `vrfLeaderValue` range-extension.
- The alphas differ → `Header(VrfCert(VerificationFailed))`. The keyhash binding
  passes, so this is a producer-side authority bug, not a test-setup issue.

Pinned by `crates/ade_node/tests/forge_succeeds.rs::forge_to_self_accept_blocked_on_praos_vrf_construction`.

## Purpose
Make the producer's leader-eligibility VRF construction match the validator's
Conway/Praos verification semantics, so a Conway/Praos block the producer forges
carries a VRF certificate accepted by the **same** `verify_praos_vrf` path used
for received blocks.

## Authority surface / touched authorities
- `ade_node::produce_mode::run_real_forge` (VRF prove step).
- `ade_core::consensus::leader_check::verify_and_evaluate_leader`.
- `ade_core::consensus::leader_schedule::query_leader_schedule`.
- `ade_core::consensus::leader_schedule::LeaderScheduleAnswer.expected_vrf_input`
  (the alpha contract).
- `verify_praos_vrf` / `praos_vrf_input` (validator side — the compatibility target).
- Any test fixtures asserting TPraos-style leader alpha.

## Proof obligations
1. For Conway/Praos, **producer alpha == validator alpha** (`praos_vrf_input`
   + `vrfLeaderValue` range-extension).
2. Existing TPraos-era behavior is either unreachable in this slice or preserved
   behind an **explicit era discriminant** (no silent dual meaning).
3. `LeaderScheduleAnswer.expected_vrf_input` is **renamed or retyped** so it
   cannot silently mean TPraos alpha in a Praos context.
4. The honest-fallback test flips from "expected blocked (`ForgeFailed`)" to
   "expected `ForgeSucceeded`" **only after** the Praos migration is complete.
5. **No fallback accepts both TPraos and Praos VRF inputs.**

## Hard prohibition
Do **not** "just try both alphas." A both-alphas verification/construction
fallback is a hidden dual-semantic surface and violates single semantic
interpretation. For a given era/protocol version there must be **exactly one**
VRF transcript authority.

## Registry
- **`CN-FORGE-04` (declared)** — producer-side Praos VRF construction must match
  the Conway/Praos validator authority (alpha, schedule evidence,
  `expected_vrf_input` contract, self-accept verification — one era-correct
  Praos construction). Enforced when N-W lands.
- **`CN-FORGE-01`** — its "ForgeSucceeded reachable" strengthening is the
  deliverable of N-W (deferred from N-V).

## Note: may reveal further mismatches
Per the N-V CLOSURE strategic note, the producer path was never validated
end-to-end. Behind the VRF mismatch there may be further producer↔validator
asymmetries (KES payload, body application). N-W scopes the VRF authority; if a
deeper mismatch appears, apply the same honest fallback (pin it, re-scope).
