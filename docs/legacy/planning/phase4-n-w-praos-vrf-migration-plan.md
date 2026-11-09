# Cluster/Slice Plan — PHASE4-N-W (Producer Praos VRF Authority Migration)

> IDD Part IV cluster plan. Derived from the confirmed invariant sketch
> `phase4-n-w-praos-vrf-migration-invariants.md` and the brief
> `phase4-n-w-praos-vrf-migration.md`. Scope: **Praos-only producer forging**;
> TPraos producer-forge fail-closes (`UnsupportedEra::ProducerForge`); TPraos
> validation preserved. Single follow-on cluster; three ordered slices.

## Cluster Index (Dependency Order)

1. **PHASE4-N-W** — Producer Praos VRF authority migration — primary invariant:
   for a supported producer era there is exactly one VRF transcript authority,
   and a Praos block the producer forges carries a VRF certificate the same
   receive-path `verify_praos_vrf` accepts (**CN-FORGE-04**).

## PHASE4-N-W — Producer Praos VRF Authority Migration

- **Primary invariant:** CN-FORGE-04 (declared → enforced at close) plus the
  deferred CN-FORGE-01 "ForgeSucceeded reachable" strengthening. No new rule IDs.
- **TCB partition:**
  - **BLUE** — `ade_core::consensus::vrf_cert` (existing `praos_vrf_input` /
    `praos_leader_value` authority), `ade_core::consensus::leader_schedule`
    (`query_leader_schedule` + the `ExpectedVrfInput` retype),
    `ade_core::consensus::leader_check` (`verify_and_evaluate_leader`).
  - **RED** — `ade_node::produce_mode` (`run_real_forge` VRF-prove sourcing + the
    TPraos fail-closed guard).
  - **GREEN** — none new.

- **Cluster Exit Criteria** (the 9 confirmed acceptance criteria):
  - **CE-W-1:** Conway/Praos producer uses `praos_vrf_input(slot, eta0)`, not the
    TPraos role-tagged `vrf_input`.
  - **CE-W-2:** Conway/Praos eligibility threshold uses `praos_leader_value(output)`,
    not the raw VRF output.
  - **CE-W-3:** the forged Praos block carries `HeaderVrf::Praos { proof, output }`.
  - **CE-W-4:** the same receive-path `verify_praos_vrf` accepts the forged header.
  - **CE-W-5:** `self_accept` reaches `ForgeSucceeded`. **(Contingent — see the
    CE-W-5 contingency below.)**
  - **CE-W-6:** a TPraos-era producer-forge request returns structured
    `UnsupportedEra::ProducerForge`.
  - **CE-W-7:** TPraos validation remains unchanged and tested.
  - **CE-W-8:** no fallback / dual-alpha verification / hidden parser fallback /
    TODO-based correctness / semantic feature flag is introduced.
  - **CE-W-9:** CN-FORGE-04 moves declared → enforced with tests and CI evidence.

- **Slices:**
  - **S1 — TPraos producer-forge fail-closed** — invariant: a non-Praos
    producer-forge request fail-closes with structured
    `UnsupportedEra::ProducerForge` (I6 / N5); the Praos path is untouched —
    addresses: **CE-W-6** — TCB: **RED** (`produce_mode`). Surface-reduction
    first: narrows the producer to Praos-only before any Praos mechanics land.
    Independently mergeable — Praos behaviour is unchanged; TPraos producer
    forging becomes explicit and structured instead of silently wrong.
  - **S2 — Praos producer leader-VRF migration** — invariant: an era-tagged
    `ExpectedVrfInput` (Praos `[u8;32]` / TPraos `[u8;40]`) makes a
    TPraos-alpha-in-a-Praos-context unrepresentable (N4); a single BLUE
    era→construction authority feeds `query_leader_schedule`,
    `verify_and_evaluate_leader` (Praos uses `praos_leader_value`), and
    `run_real_forge` (which sources its prove-alpha from the answer — no
    independent re-derivation, N3); `forge_succeeds.rs` flips to assert
    `ForgeSucceeded` — addresses: **CE-W-1, CE-W-2, CE-W-3, CE-W-4, CE-W-5,
    CE-W-7, CE-W-8** — TCB: **BLUE** (`vrf_cert` / `leader_schedule` /
    `leader_check`) + **RED** (`produce_mode`). The atomic semantic change: the
    leader-VRF alpha must agree end-to-end, so the type, the authority, and the
    three producer sites move together.
  - **S3 — Mechanical enforcement + registry** — invariant: CN-FORGE-04 is
    CI-protected — a gate asserts the producer Praos leader-VRF uses
    `praos_vrf_input` + `praos_leader_value`, rejects any both-alphas fallback,
    and confirms the TPraos producer fail-closed path — addresses: **CE-W-8,
    CE-W-9** — TCB: CI + registry. Binds `tests` / `ci_script` to CN-FORGE-04 and
    applies the CN-FORGE-01 strengthening; `/cluster-close` flips status
    declared → enforced.

- **Replay obligations:** S2 extends **DC-PROD-03** (producer
  replay-equivalence) to the now-Praos VRF bytes — same canonical forge inputs →
  byte-identical `HeaderVrf::Praos { proof, output }` (**R1**) — and adds **R2**
  (the forged VRF certificate, replayed through the validator's
  `verify_praos_vrf`, yields the same Accept that `self_accept` saw:
  producer self-accept ≡ receive-path verification). No new canonical-type
  *registry* entries; the era-tagged `ExpectedVrfInput` is a new BLUE type with
  round-trip + era-correctness tests. No new replay-corpus files required (the
  synthetic eligible-leader fixture in `forge_succeeds.rs` is sufficient).

- **FC/IS partition:** dependencies flow inward only — RED `produce_mode` →
  BLUE `ade_core::consensus`. The RED prove-step holds the operator VRF key but
  calls the BLUE construction authority (no RED-side era dispatch — N3). The
  validator path is unchanged (it is the compatibility oracle).

## CE-W-5 contingency (honest-fallback discipline)

`forge_succeeds.rs` currently fails *specifically* at the VRF proof step
(`self_accept` reaches header-validation step 6). After S2 fixes the VRF, header
validation continues into KES (real since N-S-A), op-cert, the leader threshold,
and body validation; the N-V CLOSURE warned that further producer/validator
asymmetries may lie behind the VRF one.

Therefore CE-W-5 is governed by the same envelope → VRF discovery pattern:

- The **target** is `ForgeSucceeded`.
- S2 **must** close the known VRF mismatch — CE-W-1..CE-W-4 are mechanically
  proven regardless.
- If, with the VRF fix in place, a deeper producer/validator mismatch appears,
  S2 lands the VRF fix **only if CE-W-1..CE-W-4 are mechanically proven**, then
  **pins the next blocker as a follow-on invariant slice (N-X)** rather than
  expanding N-W. The honest-fallback test pins the new blocker instead of faking
  success or loosening the assertion.

## Granularity decision

Keep three slices. **S1 narrows scope** (RED unsupported-era policy), **S2
performs the semantic migration** (BLUE VRF authority), **S3 binds enforcement**
(release / CI evidence). Collapsing them would blur three distinct authority
surfaces — RED unsupported-era policy, BLUE VRF authority migration, and the
release/CI evidence that converts "works now" into a guarded invariant.
CN-FORGE-04 is **not** considered enforced until S3 binds its tests and CI.

## Scope guardrails (carried from the brief + invariant sketch)

- Do **not** broaden scope to TPraos *producer* forging. TPraos validation
  remains supported; TPraos producer forging remains structured
  `UnsupportedEra::ProducerForge`.
- Do **not** collapse CI/registry closure (S3) into the semantic migration (S2).
- Do **not** introduce a both-alphas fallback or duplicate era→VRF-construction
  dispatch across producer and validator (N1 / N3).
