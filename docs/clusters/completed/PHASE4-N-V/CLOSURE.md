# PHASE4-N-V — CLOSURE

> **Cluster:** PHASE4-N-V — producer/validator codec symmetry (forge envelope fix).
> **Closed on the envelope fix (S1+S2).** CE-V-6 (first in-process
> `ForgeSucceeded`) is **deferred to a follow-on cluster (N-W)** because driving
> to it uncovered a second, deeper producer-side bug (TPraos/Praos VRF). This is
> the honest re-scope the cluster plan + S3 spec pre-authorized; no faked success.
> **Predecessor HEAD:** `be748ff` (PHASE4-N-T close).

## What shipped

| Slice | Commit | Summary |
|---|---|---|
| docs | `be2f8da` | cluster doc + invariants/plan + 1 declared rule |
| S1 | `e29d655` | new BLUE `ade_codec::encode_block_envelope` (symmetric to `decode_block_envelope`, Conway era 7); round-trip + corpus re-encode-identity tests |
| S2 | `1096733` | `forge_block` wraps output in `[era, block]` via the encoder; forge↔decode round-trip regression test + `ci_check_forge_decode_round_trip.sh`; OQ5 confirmed |
| S3 | (this) | honest-fallback test pinning the Praos-VRF blocker; CE-V-6 deferred to N-W |
| close | (this) | registry flip + CLOSURE + archive |

## The core fix (delivered)

The producer now emits **decodable** blocks. Root cause (from N-T): `forge_block`
emitted a bare Conway block (`array(5)`, no era envelope), so `decode_block` →
`decode_block_envelope` (requires `[era, block]`, `array(2)`) rejected **every**
forged block at offset 0 before any validation. N-V adds the single canonical
`encode_block_envelope` and routes `forge_block` through it, so forge output
round-trips through the same `decode_block` authority that validates received
blocks — pinned by `forge_block_output_decodes_via_decode_block` (fails on the
pre-N-V tree) and the corpus re-encode-identity test, gated by
`ci_check_forge_decode_round_trip.sh`.

## Registry delta

- **Enforced (new):** `CN-FORGE-03` — producer/validator codec symmetry
  (forge↔decode round-trip; bare-block forge CI-gated impossible). 4 tests +
  `ci_check_forge_decode_round_trip.sh`.
- **Strengthened (`strengthened_in += "PHASE4-N-V"`):** `DC-CONS-18` (forge
  transcript equivalence now includes the envelope + round-trip).
- **Explicitly NOT strengthened:** `CN-FORGE-01` — its planned "ForgeSucceeded
  reachable end-to-end" strengthening is **deferred to N-W** (ForgeSucceeded is
  blocked by the VRF bug below; N-V made the `self_accept` gate *reachable* —
  forge output now decodes — but a successful pass is not yet achievable).
- Registry total: 288 → **289** (+1). Append-only; no removals.

## Exit-criteria status

- **Met:** CE-V-1 (docs), CE-V-2 (encoder round-trip), CE-V-3 (corpus head pin),
  CE-V-4 (forge↔decode round-trip), CE-V-5 (CI gate), CE-V-7 (CN-FORGE-03
  enforced; DC-CONS-18 strengthened — CN-FORGE-01 deferred), CE-V-8 (workspace
  test).
- **Deferred to N-W:** CE-V-6 (first in-process `ForgeSucceeded`).

## New blocker → follow-on cluster N-W (Praos producer VRF)

Driving to `ForgeSucceeded` (S3) reached `self_accept` (envelope fixed) and was
rejected with `BlockValidityError::Header(VrfCert(VerificationFailed))`. Root
cause: the **producer** builds the leader VRF proof over the **TPraos**
role-tagged input `vrf_input(slot, eta0, LeaderEligibility)` = `slot‖eta0‖0x4C`,
but **Conway is Praos** — `validate_and_apply_header` → `verify_praos_vrf`
verifies over `praos_vrf_input(slot, eta0)` = `blake2b256(slot‖eta0)` with the
`vrfLeaderValue` range-extension. The keyhash binding (step 5) passes, proving
all test-controlled bindings are correct — this is a genuine BLUE producer-side
bug. **N-W scope:** migrate the producer-side leader-eligibility VRF
construction TPraos→Praos across `run_real_forge`,
`ade_core::consensus::leader_check::verify_and_evaluate_leader`,
`query_leader_schedule`, and the `LeaderScheduleAnswer.expected_vrf_input`
contract; then promote `forge_to_self_accept_blocked_on_praos_vrf_construction`
to assert `ForgeSucceeded`. May reveal further producer↔validator mismatches
behind it (KES payload, body) — scope with an honest fallback.

## Strategic note (load-bearing)

The producer/forge path was **never validated end-to-end** before N-T's loopback
attempt. Each attempt to produce a self-acceptable block reveals the next latent
producer↔validator asymmetry: **(1) block envelope (N-V, fixed) → (2) TPraos vs
Praos VRF construction (N-W) → possibly more.** The bounty block-production leg
(`CN-CONS-06` / `RO-LIVE-01`) is therefore **multi-cluster**, further than the
registry's prior "structurally guaranteed" notes implied. The envelope and VRF
findings are the in-house analogue of [[feedback-real-interop-finds-codec-bugs]]:
synthetic round-trips (corpus blocks decode; forge_block returns Ok) masked the
fact that forge output had never been fed to the validator.

## Open obligations carried after closure

- `CN-CONS-06` / `RO-LIVE-01` — bounty block-production leg, now gated by the
  N-W Praos-VRF migration (and the deferred serve-side wire-wrap), not just the
  operator pass.
- Forged-block durability → N-U (carried from N-T).
- Serve-side tag-24 wire-wrap → follow-on (carried from N-V scope lock).
- Grounding-doc regen (CODEMAP/SEAMS/TRACEABILITY/HEAD_DELTAS; now N-P-narrow
  through N-T + N-V) → tracked separate item.
