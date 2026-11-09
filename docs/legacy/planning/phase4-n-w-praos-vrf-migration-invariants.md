# PHASE4-N-W — Producer Praos VRF Authority Migration — Invariant Sketch

> IDD Part I sketch. Scopes the cluster declared by the brief
> `phase4-n-w-praos-vrf-migration.md` (rule `CN-FORGE-04`). Confirmed scope:
> **Praos-only producer forging** (decided 2026-05-28). Not yet a cluster plan —
> that is `/cluster-plan`.

## Concept (pure-transform check ✓)

The producer's leader-eligibility VRF is a pure function selected by era:
`(era, slot, eta0, vrf_sk) → (VrfProof, leader_value)`. For Praos eras it must
equal the construction the validator already runs. The fix **mirrors the
validator's existing closed `HeaderVrf::{Tpraos, Praos}` discriminant**
(`ade_core::consensus::header_validate.rs:142`); it does not invent a new
selector and does not remove the TPraos construction.

## The bug (grounded against code, 2026-05-28)

Two-fold, both in the producer-side leader-check — not just the alpha:

1. **Alpha.** Producer builds the TPraos role-tagged
   `vrf_input(slot, eta0, LeaderEligibility) = slot‖eta0‖0x4C`
   (`produce_mode.rs:631`, `leader_check.rs:188`, `leader_schedule.rs:108`).
   Conway validator uses the Praos `praos_vrf_input(slot, eta0) =
   blake2b256(slot‖eta0)` (`vrf_cert.rs:131`).
2. **Leader value.** `verify_and_evaluate_leader` feeds the **raw** VRF output to
   the threshold (`leader_check.rs:209`). Conway validator feeds
   `praos_leader_value(output)` — the `"L"`-tag range-extension
   (`header_validate.rs:178`, `vrf_cert.rs:162`).

Validator side is **already era-correct** (the `HeaderVrf` match). The migration
makes the producer mirror it. Pinned by
`crates/ade_node/tests/forge_succeeds.rs::forge_to_self_accept_blocked_on_praos_vrf_construction`
(self_accept reaches header validation and fails *specifically* at the VRF proof
step). KES path is real (N-S-A two-pass `unsigned_header_pre_image`), not a
placeholder — the `:609` doc comment is stale; VRF is the sole immediate blocker.

## Scope decision (confirmed) — Praos-only producer

| Requirement | Tier | Decision |
|---|---|---|
| Same forge inputs → byte-identical block outputs | true | Preserve |
| Producer/validator VRF construction agree for Conway/Praos | derived | **Enforce now** |
| Produce a valid preprod/preview block for the challenge | release/bounty | Supports directly |
| Forge Shelley–Alonzo historical TPraos blocks | — | **Explicit non-goal** |

The producer implements only the Praos VRF construction (Babbage/Conway).
A TPraos-era forge request **fails closed** with a structured
`UnsupportedEra::ProducerForge`. TPraos *validation* stays fully supported.

## 1. What must always be true

- **I1 (CN-FORGE-04).** Exactly one VRF transcript authority per *supported
  producer* era. Producer and validator derive the leader VRF input from the
  **same** era→construction mapping. Praos: `alpha = praos_vrf_input(slot,eta0)`,
  `leader_value = praos_leader_value(output)`, single combined proof.
- **I2.** The producer's eligibility verdict for a Praos era is computed from
  `praos_leader_value(output)`, and **equals** the validator's verdict for the
  same `(slot, eta0, proof, stake, asc)`.
- **I3 (strengthens CN-FORGE-01).** A Praos block the producer forges carries
  `HeaderVrf::Praos { proof, output }` that the **same** receive-path
  `verify_praos_vrf` accepts → `self_accept` reaches `ForgeSucceeded`.
- **I4.** `LeaderScheduleAnswer.expected_vrf_input` is era-correct — for Praos it
  carries the Praos alpha, never the TPraos role-tagged alpha.
- **I5.** The TPraos construction (`vrf_input`/`VrfRole`, two role-tagged proofs,
  `verify_vrf_cert`) remains correct and reachable for genuine TPraos-era
  **validation** — preserved, not removed, still tested.
- **I6.** A TPraos-era *producer forge* request fails closed with a structured
  `UnsupportedEra::ProducerForge` — never a silent skip, never a TPraos producer
  proof.

## 2. What must never be possible

- **N1.** No code verifies/constructs the leader VRF by trying both alphas and
  accepting either (brief hard prohibition). Structurally forbidden.
- **N2.** The producer never emits, for a Praos era, a header whose VRF proof is
  over the TPraos alpha (today's bug) — unrepresentable / fail-closed.
- **N3.** The era→construction selection is not duplicated with divergent logic
  across producer and validator (no two dispatch tables that can drift). One
  shared BLUE authority. The RED prove-step may hold the key but **must call**
  the BLUE construction authority, not embed its own dispatch.
- **N4.** `expected_vrf_input` has no silent dual meaning — it cannot be a bare
  blob that could hold either alpha with no era discriminant.
- **N5.** No untested TPraos *producer* branch "for completeness" — no dead
  consensus-critical code. Historical TPraos producer support is a later
  invariant slice if a real consumer appears.

## 3. Deterministic surface

- **D1.** ECVRF-draft-03 proof generation is deterministic given `(vrf_sk,
  alpha)`; `praos_vrf_input` / `praos_leader_value` / `is_leader` are pure (no
  clock/rand/float/HashMap). Same inputs → byte-identical proof + identical
  verdict.
- **D2.** Era→construction selection is a pure function of era / protocol
  version, no ambient state.

## 4. Replay-equivalent

- **R1 (extends DC-PROD-03).** Same canonical forge inputs → byte-identical
  forged block, including the byte-identical `HeaderVrf::Praos` proof + output.
- **R2.** The forged block's VRF cert, replayed through the validator's
  `verify_praos_vrf`, yields the same Accept that `self_accept` saw (producer
  self-accept ≡ receive-path verification).

## 5. State transitions in scope

- **T-W1** (RED holds `vrf_sk`; calls BLUE construction authority):
  ```
  (era, slot, eta0, vrf_sk)
    Praos era  → Result<(VrfProof, VrfOutput), VrfError>
    TPraos era → Err(UnsupportedEra::ProducerForge)
  ```
- **T-W2** (BLUE): `(slot, eta0, vrf_vk, proof, answer{era-correct}, stake, asc)
  → Result<LeaderCheckVerdict, _>` — Praos path derives `praos_leader_value`
  then `is_leader`.
- **T-W3** (BLUE): `(state, query) →
  Result<LeaderScheduleAnswer{era-correct expected_vrf_input}, _>`.
- **T-W4** (BLUE composition): forge emits `HeaderVrf::Praos` for a Praos era.
- **T-W5** (validator, **unchanged** — the compatibility oracle):
  `verify_praos_vrf` / `HeaderVrf::Praos`.

## 6. TCB color hypothesis

- **BLUE:** the era→construction selector + Praos alpha/leader-value (already in
  `ade_core::consensus::vrf_cert`); `verify_and_evaluate_leader` (`leader_check`);
  `query_leader_schedule` + `LeaderScheduleAnswer` (`leader_schedule`); the
  validator path (unchanged).
- **RED:** `run_real_forge` VRF-prove step (`produce_mode`) — operator key
  custody; calls the BLUE alpha builder, proves, hands the proof to the BLUE
  evaluator.
- **GREEN:** none new.
- **Open color crux (→ cluster-plan):** where the era→construction selector lives
  so **one** BLUE authority serves both the BLUE evaluator and the RED
  prove-step. The RED step must not embed its own era dispatch (N3).

## 7. Acceptance criteria (normative — future CEs)

1. Conway/Praos producer uses `praos_vrf_input(slot, eta0)`, not TPraos
   role-tagged `vrf_input`.
2. Conway/Praos eligibility threshold uses `praos_leader_value(output)`, not the
   raw VRF output.
3. Forged Praos block carries `HeaderVrf::Praos { proof, output }`.
4. The same receive-path `verify_praos_vrf` accepts the forged header.
5. `self_accept` reaches `ForgeSucceeded`.
6. A TPraos producer-forge request returns structured
   `UnsupportedEra::ProducerForge`.
7. TPraos validation remains unchanged and tested.
8. No fallback, dual-alpha verification, hidden parser fallback, TODO-based
   correctness, or semantic feature flag is introduced.
9. `CN-FORGE-04` moves `declared → enforced` with tests and CI evidence.

## 8. Open questions (remaining)

- **Q2 (→ cluster-plan).** Exact retype/rename of `expected_vrf_input` to satisfy
  N4. Candidates: (a) era-tagged enum `{ TpraosLeader([u8;40]), Praos([u8;32]) }`;
  (b) carry era + raw `(slot, eta0)` and let the consumer build the alpha via the
  single authority; (c) Praos-only `[u8;32]` rename. The invariant only requires
  "TPraos-alpha-in-Praos-context unrepresentable" — (a) or (b) satisfy N4 most
  directly.
- *Resolved this session:* Q1 (Praos-only, confirmed); Q3 (raw-vs-`praos_leader_value`
  gap, confirmed part of the bug); Q4 (KES is real, not a placeholder); Q5 (no
  producer nonce emission — single proof suffices, validator derives the nonce).

## 9. Registry impact (no new IDs)

- **`CN-FORGE-04`** (declared) already encodes I1/I2/I4/N1 — N-W moves it
  `declared → enforced` and appends `tests` + `ci_script` (criterion 9).
  Candidate minor **strengthening** of its statement to name the I6 TPraos
  producer-forge fail-closed clause (a strengthening, not a new rule).
- **`CN-FORGE-01`** — apply the deferred "ForgeSucceeded reachable" strengthening
  (I3).
- No new family/IDs proposed (per the confirmed registry-grain decision). N3
  (one shared selector) and R1 (producer-VRF replay) are covered by CN-FORGE-04 +
  DC-PROD-03; promote to dedicated `DC-FORGE-*` rules later only if cluster-plan
  finds they need independent enforcement.
