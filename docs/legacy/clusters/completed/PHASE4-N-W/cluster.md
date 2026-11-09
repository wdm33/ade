# Invariant Cluster — PHASE4-N-W (Producer Praos VRF Authority Migration)

> **Status:** Planning Artifact (Non-Normative). Organizes work; introduces no
> new requirements. Normative documents + CI enforcement win on any conflict.

## Cluster PHASE4-N-W — Producer Praos VRF Authority Migration

**Primary invariant:** `CN-FORGE-04` (declared → enforced) — for a supported
producer era there is exactly one VRF transcript authority; a Praos block the
producer forges carries a VRF certificate the **same** receive-path
`verify_praos_vrf` accepts. Strengthens `CN-FORGE-01` (the deferred
"ForgeSucceeded reachable").

**Normative anchors:**
- `docs/planning/phase4-n-w-praos-vrf-migration.md` (brief),
  `docs/planning/phase4-n-w-praos-vrf-migration-invariants.md` (sketch — I1–I6 /
  N1–N5 / R1–R2), `docs/planning/phase4-n-w-praos-vrf-migration-plan.md` (slice
  plan).
- Registry: `CN-FORGE-04` (primary), `CN-FORGE-01` (strengthened), `DC-PROD-03`
  (replay), `DC-CONS-18` (single forge/encode authority — cross-ref).
- Cardano `Ouroboros.Consensus.Protocol.Praos.VRF` (`mkInputVRF`,
  `vrfLeaderValue`) — the compatibility target already pinned in
  `ade_core::consensus::vrf_cert` against the Conway-576 corpus.

---

### Entry Conditions (guaranteed by prior clusters)

- **N-V / `CN-FORGE-03`:** forge output is the enveloped `[era, block]` form that
  round-trips through `decode_block`, so `self_accept` reaches header validation
  (the VRF step is the failure point now, not the envelope).
- **N-S-A / `CN-KES-HEADER-01`:** the producer KES-signs the real
  `unsigned_header_pre_image` (two-pass forge) — KES is not the blocker.
- **N-R-A / `CN-FORGE-02`:** the BLUE `leader_check` evaluator
  (`verify_and_evaluate_leader`, closed `LeaderCheckVerdict`) and the RED/BLUE
  leader split exist.
- **N-T / `CN-PROD-03`, `CN-PROD-04`:** `produce_mode` cold-starts a real forge
  base via `bootstrap_initial_state`; `self_accept`-gated serve.
- **Validator Praos path** (`header_validate.rs` `HeaderVrf::Praos` match →
  `verify_praos_vrf` + `praos_leader_value`/`praos_nonce_value`, pinned against
  Conway-576): correct and tested — the compatibility oracle. **N-W does not
  modify it.**

---

### Exit Criteria (CI-Verifiable)

- [ ] **CE-W-1:** `ci/ci_check_producer_praos_vrf.sh` (new, S3) asserts the
  producer leader-VRF Praos path builds its alpha via `praos_vrf_input`, and that
  **no** role-tagged `vrf_input(…LeaderEligibility)` survives on the Praos
  producer path; backed by a `leader_check`/`produce_mode` unit test.
- [ ] **CE-W-2:** a `leader_check` unit test asserts the Praos eligibility
  threshold consumes `praos_leader_value(output)` (not the raw output); the S3
  gate confirms no raw-output threshold on the Praos producer path.
- [ ] **CE-W-3:** a test asserts `decode_block(forge_block(<praos tick>).bytes)`
  yields a header whose VRF carrier is `HeaderVrf::Praos { proof, output }`
  (extends N-V `ci_check_forge_decode_round_trip.sh`).
- [ ] **CE-W-4:** a test asserts the forged header passes the same
  `validate_and_apply_header` → `verify_praos_vrf` the receive path runs (R2).
- [ ] **CE-W-5 (contingent):** `crates/ade_node/tests/forge_succeeds.rs::forge_to_self_accept_succeeds`
  (renamed from `…_blocked_on_praos_vrf_construction`) asserts
  `CoordinatorEvent::ForgeSucceeded`. **If a deeper producer/validator mismatch
  surfaces after the VRF fix:** S2 lands the VRF fix only with CE-W-1..4
  mechanically proven, and pins the next blocker as follow-on **N-X** rather than
  expanding N-W (honest-fallback; see plan §CE-W-5 contingency).
- [ ] **CE-W-6:** a test asserts a non-Praos producer-forge request returns
  structured `UnsupportedEra::ProducerForge` (surfaced via the producer-forge
  result / `ForgeFailureReason`).
- [ ] **CE-W-7:** the existing TPraos-validation tests stay green —
  `crates/ade_core/tests/header_validate_compose.rs` (`HeaderVrf::Tpraos` branch)
  + `vrf_cert.rs::tests::vrf_role_tags_match_convention`; the S3 gate confirms
  `vrf_input`/`VrfRole` are **preserved** (not removed).
- [ ] **CE-W-8:** `ci/ci_check_producer_praos_vrf.sh` confirms no both-alphas /
  dual-construction fallback and no producer-side era-dispatch duplication;
  `ci_check_no_semantic_cfg.sh` confirms no semantic feature flag was added.
- [ ] **CE-W-9:** `CN-FORGE-04` in the registry has non-empty `tests` +
  `ci_script` (incl. `ci_check_producer_praos_vrf.sh`) and `status = "enforced"`;
  `CN-FORGE-01` carries the N-W strengthening. Verified at `/cluster-close`.

> No human review may substitute for these checks.

---

### Expected Slice Types

- **S1** — RED policy guard (structured `UnsupportedEra::ProducerForge`; surface
  reduction).
- **S2** — BLUE canonical-type introduction (`ExpectedVrfInput` era-tag) + BLUE
  authority migration (alpha + leader-value) + RED prove-sourcing rewire +
  replay-equivalence test + the pinning-test flip.
- **S3** — CI gate authoring + registry binding/strengthening.

---

### TCB Color Map (FC/IS Partition)

- **BLUE:** `ade_core::consensus::vrf_cert` (existing `praos_vrf_input` /
  `praos_leader_value` authority — extended as the single era→construction
  selector), `ade_core::consensus::leader_schedule` (`query_leader_schedule` +
  the new `ExpectedVrfInput` type), `ade_core::consensus::leader_check`
  (`verify_and_evaluate_leader`). `ade_core::consensus::header_summary`
  (`HeaderVrf` — already BLUE, unchanged).
- **GREEN:** none.
- **RED:** `ade_node::produce_mode` (`run_real_forge` prove-alpha sourcing + the
  S1 era guard). Holds the operator VRF key; calls the BLUE authority — no
  RED-side era dispatch.

All colors resolved; no open color questions.

---

### Forbidden During This Cluster

- A both-alphas (TPraos + Praos) verification/construction fallback (N1).
- Duplicated era→VRF-construction dispatch across producer and validator (N3) —
  one shared BLUE authority; the RED prove-step must call it, not embed its own.
- Removing or weakening TPraos *validation* (`vrf_input` / `VrfRole` /
  `verify_vrf_cert` stay) (I5).
- TPraos *producer* forging — out of scope; structured
  `UnsupportedEra::ProducerForge` only (I6 / N5). No untested TPraos producer
  branch / dead consensus-critical code.
- A silent dual meaning of `expected_vrf_input` — it must be era-discriminated so
  a TPraos alpha in a Praos context is unrepresentable (N4).
- TODO-based correctness or a semantic feature flag in the authoritative path
  (CE-W-8).

---

> Planning aid only. All correctness rules live in the project's normative
> specifications + CI enforcement.
