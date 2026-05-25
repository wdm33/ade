# PHASE4-N-C — Block Production Closure — Invariant Sketch

**Status**: invariants phase complete; awaiting `/cluster-plan`
**HEAD pin**: `96d043c`
**Date**: 2026-05-25

## Framing

N-C is **not** "block assembly." N-C is **block production closure**:

1. **Deterministic forge authority** (BLUE): given a canonical
   `ProducerTick` carrying already-signed VRF proof + KES signature +
   OpCert, BLUE assembles the block bytes deterministically and
   guards leader-check, tx-admissibility, and body-hash parity with
   the validator path.
2. **Self-acceptance gate** (BLUE → RED bridge): a forged block is
   not eligible for RED broadcast until Ade's own header validator
   (N-B) and body validator (B1) accept it under the same
   slot / era / context.
3. **Cross-impl live evidence** (release/interop): forged blocks are
   accepted by cardano-node when delivered via N2N. Operator-action
   live evidence binary following the CE-N-B-6 / CE-N-E-6 precedent.

This sketch is constrained by the project key-boundary doctrine
(established by Phase 4 prior clusters and reaffirmed during OQ
resolution): **private-key custody and signing operations live in
RED/GREEN, never BLUE**. BLUE consumes signed artifacts (proofs,
signatures, opcerts) and proves they are consistent. Replay captures
signed artifacts, not private keys.

---

## 1. What must always be true

### VRF — leader proof (transcript-level)
- **NC-VRF-1** Signing transcript equivalence: given canonical inputs
  `(slot, epoch_nonce, vrf_signing_key, vrf_role)`, the RED signer
  produces a `VrfProof` byte-identical to cardano-node's reference
  output. *(VRF_03 is deterministic; this is interop-equivalence.)*
- **NC-VRF-2** Verification symmetry: the emitted `VrfProof` passes
  `verify_praos_vrf` with the matching verification key.
- **NC-VRF-3** Single source of leader truth: the producer's leader
  decision uses the **same** `is_leader` / `check_leader_claim`
  functions the validator uses. No producer-side fork of the
  leader-check.

### KES — signing transcript + evolution discipline
- **NC-KES-1** Signing transcript equivalence: given
  `(kes_secret, period, msg)`, RED `kes_sign` produces a signature
  byte-identical to reference and verifies under `verify_kes`.
- **NC-KES-2** Evolution discipline: `evolve(k_i) → k_{i+1}` is
  one-way; the evolved key signs period `i+1` and **cannot** sign
  period `i`. Forbidden in RED to use a key past
  `evolutions_remaining`.
- **NC-KES-3** Slot-to-period purity (BLUE check): the KES period at
  slot `s` is `(s − opcert_anchor_slot) / slots_per_kes_period`
  (integer floor) over **operator-supplied explicit inputs**. BLUE
  never infers the anchor from wall-clock or filesystem state.
- **NC-KES-4** Header-period consistency (BLUE check): a block signed
  at period `p` carries an opcert + header KES period field equal to
  `p`. Mismatch is a forge-time error.

### OpCert — operator-key lifecycle
- **NC-OC-1** OpCert wire grammar (BLUE): OpCert CBOR Ade produces
  via the closed encoder is byte-identical to
  `cardano-cli node issue-op-cert` output for identical
  `(cold_sk, kes_vk, kes_period, counter)`. *(Closed-grammar interop
  with operator tooling; Ade ships the encoder/decoder, not the
  renewal action.)*
- **NC-OC-2** OpCert counter monotonicity (BLUE check): per
  (cold-key, node) the opcert serial counter is **strictly
  monotonically increasing**. Regression is a hard error at the
  RED→BLUE boundary (RED feeds the value; BLUE rejects regression).

### Block forging — the BLUE integration
- **NC-FORGE-1** Forge is pure: given a canonical `ProducerTick`
  carrying `{slot, ledger_state, mempool_snapshot, pparams,
  vrf_proof, kes_sig, opcert}`, forge has no clock, no rand, no
  `HashMap`, no I/O, no locale, no ambient state. (BLUE rule;
  strengthens T-DET-01.)
- **NC-FORGE-2** Forge byte-equality across replays: forged block
  bytes are byte-identical across two replays of the same canonical
  `ProducerTick`. Replay uses **captured signed artifacts**, never
  re-signs.
- **NC-FORGE-3** Leader-check gate (forbidden transition): forge is
  only invoked when `is_leader(...) == true` for
  `(slot, state, vrf_output)`. Calling forge as non-leader is a
  BLUE-rejected forbidden input.
- **NC-FORGE-4** Tx-set admissibility: every tx in the forged block
  was admissible under `ade_ledger::mempool::admit` against the
  snapshot's base ledger state, in the snapshot's canonical
  accumulating order. No tx in a forged block bypasses mempool
  validation.
- **NC-FORGE-5** Body-hash parity with the validator path:
  `header.body_hash = blake2b_256(forged_body_wire_bytes)`, where
  `forged_body_wire_bytes` are produced by the **single
  Cardano-compatible canonical block-body encoder used by the
  validator hash path**. The producer and validator hash the same
  bytes through the same encoder. (Strengthens T-ENC-01.)
- **NC-FORGE-6** Block-budget conformance: forged blocks honor
  `max_block_body_size` and `max_block_ex_units` from `pparams` at
  the slot's era.

### Self-acceptance — BLUE→RED bridge
- **NC-SELF-1** Self-acceptance gate before broadcast: a forged block
  is **not eligible** for RED broadcast unless Ade's own N-B header
  validator and B1 body validator accept it under the same slot,
  era, and context. Self-acceptance failure halts the producer
  deterministically.

### Cross-impl acceptance — release evidence
- **NC-LIVE-1** Cross-impl acceptance (release/interop, not "true"):
  forged blocks are accepted by cardano-node when delivered via N2N
  block-fetch / chain-sync. Mechanical CI adapter + operator-action
  `live_block_production_session` binary producing
  `CE-N-C-LIVE_<date>.log`. Conditional on
  `blocked_until_operator_stake_available` — if testnet stake/SPO
  registration isn't yet available, the live evidence is gated, not
  the cluster's mechanical CEs.

---

## 2. What must never be possible

- KES key signs at period `p` while opcert/header stamp `p' ≠ p`.
  *(BLUE rejects at forge time.)*
- A KES secret past `evolutions_remaining` produces a signature in
  RED. *(RED signing path enforces; BLUE cannot recover.)*
- OpCert serial counter regresses or repeats per (cold-key, node).
  *(BLUE rejects at forge time.)*
- Forge runs at a slot where `is_leader` is false. *(BLUE rejects.)*
- Forge emits a block whose tx-set is not a prefix of the mempool's
  canonical accumulating order. *(BLUE rejects.)*
- BLUE forge reads wall-clock, rand, env, or filesystem.
- VRF/KES/cold private-key bytes appear in BLUE code, in any `Debug`
  / error / log path, or in any replay artifact.
- BLUE infers `opcert_anchor_slot` from anything other than the
  explicit `ProducerTick` input.
- A forged block is broadcast before passing Ade's own validator
  path. *(NC-SELF-1 gate.)*
- Block production proceeds without an opcert (Shelley+).

---

## 3. Deterministic surface (identical across executions)

Producer-side bytes are deterministic at two layers:

**RED signing transcripts** (deterministic given inputs, but executed
in RED for key-custody discipline):
- `vrf_prove(sk, input)` bytes
- `kes_sign(sk, period, msg)` bytes
- `evolve(sk)` bytes after `n` evolutions

**BLUE consumed bytes** (deterministic given canonical
`ProducerTick`):
- OpCert CBOR bytes for fixed fields
- Forged block CBOR bytes
- `header.body_hash` bytes for fixed canonical body

---

## 4. Replay-equivalent

Replaying
`(initial_ledger_state, ordered Vec<ProducerTick>)` —
where each tick carries
`{slot, mempool_snapshot, leader_check_inputs, vrf_proof, kes_sig, opcert, pparams}` —
twice produces a byte-identical `Vec<ForgedBlockBytes>`.

This extends the verdict-stream / chain-selector replay claims from
N-B / B1 / B2. **Replay never invokes RED signing**; captured signed
artifacts are part of the replay input. Private-key material does
not appear in replay corpora.

---

## 5. State transitions in scope

```
// ---- RED / GREEN (private-key custody) ----

vrf_prove(
    sk: VrfSigningKey,
    input: VrfInput,
) -> Result<(VrfProof, VrfOutput), CryptoError>
  // RED. Deterministic given (sk, input). Output is the BLUE-consumed artifact.

kes_sign(
    sk: KesSecret,
    period: KesPeriod,
    msg: &[u8],
) -> Result<KesSignature, KesError>
  // RED. Forbidden: period > sk.evolutions_remaining + sk.current_period.

kes_evolve(
    sk: KesSecret,
    from: KesPeriod,
    to: KesPeriod,
) -> Result<KesSecret, KesError>
  // RED. Forbidden: to < from || to > from + sk.evolutions_remaining.

// ---- BLUE (signed-artifact consumer) ----

opcert_validate(
    opcert: OpCert,
    cold_vk: ColdVk,
    expected_period: KesPeriod,
    prev_counter: Option<u64>,
) -> Result<(), OpCertError>
  // BLUE. Forbidden: prev_counter.is_some() && opcert.counter <= prev_counter.unwrap()
  // Forbidden: opcert.kes_period != expected_period.

forge_block(
    tick: ProducerTick,  // carries signed artifacts, no private keys
) -> Result<(ForgedBlock, ForgeEffects), ForgeError>
  // BLUE. Pure.
  // Forbidden: leader_check(...) == false (NC-FORGE-3)
  // Forbidden: any tx fails phase-1 against accumulating state (NC-FORGE-4)
  // Forbidden: opcert.kes_period != period_at_slot(tick.slot, anchor) (NC-KES-4)
  // Forbidden: body_size > pparams.max_block_body_size (NC-FORGE-6)
  // Effects include "ReadyForSelfAccept(bytes)" — RED cannot consume until NC-SELF-1.

self_accept(
    forged_bytes: &[u8],
    state: &LedgerState,
    pparams: &ProtocolParams,
) -> Result<AcceptVerdict, ValidationError>
  // BLUE. Re-uses N-B header validator + B1 body validator paths.
  // Returns Accept | Reject(reason). RED broadcast gated on Accept.

producer_step(
    state: ProducerState,
    tick: ProducerTick,
) -> Result<(ProducerState, Vec<ProducerEffect>), ProducerError>
  // BLUE iff tick carries explicit signed artifacts (current design).
  // Effects: ForgeAttempted | ForgedAndSelfAccepted(bytes) | NotLeader | ForgeFailed | SelfAcceptFailed.
```

---

## 6. TCB color hypothesis

**BLUE (authoritative core, no private-key custody):**
- `ade_core::consensus::forge` (new): `forge_block`, leader-check
  guard, tx-admissibility guard, body-hash parity.
- `ade_core::consensus::opcert_validate` (new): grammar +
  counter-monotonicity + period-consistency checks.
- `ade_core::consensus::self_accept` (new): wraps existing N-B + B1
  validator paths into a single accept/reject gate.
- `ade_core::consensus::producer_state` (new): `producer_step` total
  transition over explicit `ProducerTick`.
- `ade_crypto::vrf::verify_*`, `kes::verify_*`, `opcert` decoder —
  **already BLUE**, unchanged.

**GREEN (deterministic glue, non-authoritative):**
- `ade_runtime::producer::tick_assembler`: composes a canonical
  `ProducerTick` from RED scheduler outputs (slot tick, mempool
  snapshot, ledger tip, signed proofs from the signer). No
  nondeterminism by construction — if any sneaks in, this is a
  cluster bug.

**RED (shell — private-key custody, I/O, scheduling):**
- `ade_runtime::producer::signing` (new): `vrf_prove`, `kes_sign`,
  `kes_evolve`. **Key material is loaded from disk and held only
  here.** Outputs are signed artifacts (bytes); private keys never
  leave this module.
- `ade_runtime::producer::keys` (new): disk reads of `*.skey` /
  opcert files in cardano-cli format; decoding into in-memory
  secrets; zeroize-on-drop discipline.
- `ade_runtime::producer::scheduler` (new): slot wakeup loop,
  RED→GREEN→BLUE call sequence, post-self-accept network handoff.
- `ade_runtime::producer::broadcast` (new): outbound queue handing
  self-accepted bytes to `ade_network`'s N2N block-fetch / chain-sync
  server path. Scope: enough delivery for cardano-node to fetch the
  block. Full relay-mesh behavior is N-A successor scope.
- `ade_core_interop::bin::live_block_production_session` (new):
  operator-action evidence binary producing `CE-N-C-LIVE_<date>.log`.
  Conditional on `blocked_until_operator_stake_available`.

**No "open colors" remain** after OQ resolution. The producer-to-
network handoff (OQ10) is RED owned by `ade_runtime::producer::
broadcast`.

---

## 7. OQ resolutions (locked)

| OQ | Resolution | Tier |
|----|---|---|
| OQ1 Key origin | Operator-supplied cardano-cli-format keys/opcerts only; Ade never generates. | operational + true boundary |
| OQ2 KES algo | Sum6KES only. | derived |
| OQ3 VRF algo | Praos VRF compatible with the validator-side verify path. | derived |
| OQ4 Praos only | Conway/Praos producer; TPraos producer explicit non-goal. | derived / scope |
| OQ5 OpCert anchor | RED/operator config supplies anchor explicitly; BLUE never infers from clock/FS. | true + derived |
| OQ6 Renewal | Ade does not own opcert renewal; round-trip/validate only. cardano-cli mints. | operational + release |
| OQ7 Empty block | Empty mempool → empty block when leader. Forbidden transition is non-leader forge, not empty-body forge. | derived |
| OQ8 Nonce contribution | Explicit proof obligation; fixture extracts Ade's forged VRF nonce contribution and compares to validator/oracle path. | derived |
| OQ9 Liveness | Operational SLA (`OP-OPS-05`), not BLUE invariant. | operational |
| OQ10 Broadcast | N-C includes forge → N2N delivery → cardano-node acceptance evidence. | release / bounty |
| OQ11 Live evidence | Mechanical CI + operator-action live; live half marked `blocked_until_operator_stake_available`. | release |
| OQ12 Greenfield | Sign-side primitives are greenfield; zero validator regressions allowed. | true + derived |

---

## 8. Proposed registry entries (14 candidates)

Per the IDD invariant-registry contract (append-only, family-prefixed,
next-sequential numbering), proposing:

| Proposed ID | Subject | Kind / tier |
|---|---|---|
| `DC-CRYPTO-03` | VRF signing transcript equivalence + verify symmetry; private-key execution RED-confined. | determinism (derived) |
| `DC-CRYPTO-04` | KES signing transcript equivalence + verify symmetry; private-key execution RED-confined. | determinism (derived) |
| `DC-CRYPTO-05` | KES evolution discipline: one-way, cannot sign past `evolutions_remaining`. | fail-fast (derived) |
| `DC-CONS-11` | OpCert `kes_period` field equals KES period at the forged slot under operator-supplied anchor. | authority (derived) |
| `DC-CONS-12` | OpCert serial counter strictly monotonic per (cold-key, node); BLUE rejects regression at RED→BLUE boundary. | fail-fast (derived) |
| `DC-CONS-13` | Forge is pure given canonical `ProducerTick`; no clock / rand / I/O / non-canonical iteration (strengthens T-DET-01). | determinism (derived) |
| `DC-CONS-14` | Forge byte-equal across replays for identical canonical `ProducerTick`; replay uses captured signed artifacts. | replay (derived) |
| `DC-CONS-15` | Forge only invoked when leader-check passes; non-leader forge forbidden. | authority (derived) |
| `DC-CONS-16` | Forged `header.body_hash = blake2b_256(forged_body_wire_bytes)` via the **single Cardano-compatible canonical block-body encoder used by the validator hash path** (strengthens T-ENC-01). | closure (derived) |
| `DC-LEDGER-12` | Every tx in a forged block is admissible via `mempool::admit` against the base ledger state, in the snapshot's canonical accumulating order. | authority (derived) |
| `CN-CONS-06` | Cross-impl acceptance: forged blocks accepted by cardano-node when delivered via N2N. Operator-action live evidence; conditional on testnet stake. | release / interop |
| `CN-CONS-07` | Self-acceptance bridge: a forged block is not eligible for RED broadcast unless Ade's own header and body validators accept it under the same slot/era/context. | release / derived bridge |
| `OP-OPS-04` | Operator-supplied keys; Ade does not generate cold/KES/VRF material. | operational |
| `OP-OPS-05` | Slot-deadline forging SLA — operational, not constitutional. | operational |

CN-CONS-06 is **release/interop**, not "true." Cross-impl acceptance
is evidence that derived compatibility holds; it is not itself a
universal law.

---

## 9. Cluster-plan handoff notes

When entering `/cluster-plan PHASE4-N-C` the planner should keep:

- **5–8 slices**, organized around invariant authority clusters
  (per `feedback_invariant_slice_planning`), not feature/era
  accumulation. Suggested grouping:
  1. RED signing primitives + key loading (NC-VRF-1, NC-KES-1/2,
     OP-OPS-04).
  2. BLUE opcert validate + counter monotonicity (NC-OC-1, NC-OC-2,
     DC-CONS-11/12).
  3. BLUE forge core + leader-check + tx-admissibility (NC-FORGE-1
     through NC-FORGE-4, DC-CONS-13/14/15, DC-LEDGER-12).
  4. BLUE body-hash parity + validator-shared encoder (NC-FORGE-5,
     DC-CONS-16).
  5. BLUE self-acceptance gate (NC-SELF-1, CN-CONS-07).
  6. RED scheduler + GREEN tick-assembler + broadcast handoff
     (operational shape; covers OQ10).
  7. Live evidence binary + mechanical CI adapter (NC-LIVE-1,
     CN-CONS-06, OP-OPS-05).
- **No carry-forward**. Every CE named in the cluster doc must be
  reachable mechanically by the slices planned. Live-evidence CE is
  conditional via `blocked_until_operator_stake_available`, not
  deferred.
- **Self-acceptance is first-class**, not a footnote. CN-CONS-07
  exists because the bounty's "accepted block" claim hinges on it.
- **Adversarial negative corpus** (per `feedback_fail_closed_validation`):
  every forge-time forbidden transition needs a negative test that
  drives forge with a malformed `ProducerTick` and confirms BLUE
  rejects. No positive-only closure.

---

## 10. Related

- [[project-phase4-n-c-handoff]] — the session pickup that named this cluster
- [[project-bounty-requirements]] — why N-C is Tier 1 (validation + production both required)
- [[feedback-invariant-slice-planning]] — slices organize around invariant authority clusters
- [[feedback-fail-closed-validation]] — negative corpus required for every forbidden transition
- [[feedback-codec-closed-grammar]] — opcert encoder closure pattern
- [[feedback-tx-validity-priority]] — fail-closed discipline carried to block-validity
