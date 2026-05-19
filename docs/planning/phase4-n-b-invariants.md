# PHASE4-N-B — Invariant Sketch (Consensus Runtime) — v2

> **Status**: planning artifact, not normative. Open questions closed per user closure table 2026-05-19.
> **Tier**: 1. Decisions affect which blocks Ade accepts → observable on the network.

## Final framing (for use in `/cluster-plan`)

> N-B owns Cardano-compatible chain selection and Praos header authority. It consumes canonical candidate header summaries, a typed `EraSchedule`, a forecasted/ticked ledger view, and protocol parameters. It does not read ChainDB directly, does not parse genesis text, does not rederive ledger stake snapshots, and does not inspect block bodies for fork-choice comparison.
>
> Praos selection is block-number first, then protocol-specific Praos `TiebreakerView`. Equal-block-number candidates are resolved by (slot, issuer, op-cert issue number, VRF output) per the configured Cardano-compatible tiebreaker flavour. Density belongs to Genesis/catch-up logic, not normal caught-up Praos fork-choice.
>
> Rollback is bounded by security parameter k, measured in blocks. Mainnet k = 2160. Candidate chains that fork before the immutable tip or require rollback deeper than k are deterministically rejected with structured errors.
>
> Epoch nonce evolution, VRF leader verification, op-cert counter evolution, and header validation are N-B responsibilities. Stake snapshots and ledger views are ledger responsibilities consumed by N-B through a forecast boundary. Forecast horizon violations are structured errors, not guessed values.

## Closure table (decisions captured)

| Q | Decision | Tier | Scope |
|---|---|---|---|
| a | Cardano chain order = block-number first, then Praos `TiebreakerView`. Density is Genesis/catch-up only. **Citation still needed**: pin to exact ouroboros-consensus revision of target cardano-node version before code lock. | derived | N-B |
| b | k is a **block** rollback bound. Mainnet k = 2160 blocks. Forecast/stability windows may be slot-based but must accommodate ≥k+1 blocks. | derived | N-B / N-D |
| c | HFC schedule = typed `EraSchedule` (BLUE-consumed), RED genesis ingestion produces it. Anchored to `BootstrapAnchorHash`. | true + derived + release | N-B / bootstrap |
| d | Epoch nonce evolution is **N-B consensus state**, not ledger. `PraosChainDepState` owns evolving / candidate / epoch / previous_epoch / lab / last_epoch_block nonces + op-cert counters. | derived | N-B |
| e | Stake snapshots are ledger-owned; N-B **consumes** forecasted/ticked ledger view, never rederives. | derived | N-B / ledger boundary |
| f | VRF cert / leadership verification is its **own authority slice** inside N-B (not folded into leader schedule). Depends on `ade_crypto::verify_vrf`. | true + derived | N-B / crypto |
| g | Forecast horizon enforced at the ledger-view forecast / time-conversion boundary. Out-of-range = structured `OutsideForecastRange`. Bound derived from era history + safe zone + HFC, not a magic constant. | derived | N-B / LSQ |
| h | BLUE N-B fork-choice does **not** receive `&ChainDb`. It receives a typed `CandidateFragment { anchor, headers, select_view, rollback_depth }` materialized by GREEN glue. | true | N-B / N-D |
| i | Fork-choice is header-only after header validation. Bodies are for ledger validation and adoption, not tip comparison. | derived | N-A / N-B |
| j | Deep rollback / common-prefix violation = deterministic refusal. Structured `ChainSelectionReject` enum (`ForkBeforeImmutableTip`, `ExceededRollback`). N-A surfaces, N-B owns reason, N-D enforces immutable. | derived | N-A / N-B / N-D |

## 1. What must always be true (revised)

| # | Invariant | Registry rule |
|---|---|---|
| 1.1 | Best-chain selection is a deterministic pure function of `(candidate_fragments, EraSchedule, ledger_view, protocol_params)` → `ChainHash` (or `ChainSelectionReject`) | **DC-CONSENSUS-01** strengthened |
| 1.2 | Selection ordering: **block-number first**, then `TiebreakerView` (slot, issuer, op-cert issue number, VRF output). Density is NOT part of caught-up Praos ordering. | NEW — `DC-CONS-03` (chain selection ordering) |
| 1.3 | Slot leadership = pure function `is_leader(slot, vrf_key, stake_dist_from_ledger_view, asc, epoch_nonce)` | **DC-CONSENSUS-02** strengthened |
| 1.4 | Praos VRF input = function of `(slot, epoch_nonce)`. Cert verifies under registered pool VRF key. Leader value below threshold for stake fraction × ASC. Nonce contribution derived through VRF hashing / range-extension. | **DC-CRYPTO-01** + **CN-CRYPTO-02** strengthened |
| 1.5 | Leader schedule for epoch E consumes stake snapshot frozen at E−2 **from the ledger view** — N-B does not rederive. | **CN-EPOCH-01** strengthened |
| 1.6 | `PraosChainDepState` (evolving/candidate/epoch/previous_epoch/lab/last_epoch_block nonces + op-cert counters + last_slot) is owned by N-B and evolves deterministically per header. | NEW — `DC-CONS-04` (nonce evolution authority) |
| 1.7 | Rollback bound: `k = 2160 blocks` on mainnet (parameterized for testnets). Rollback `>k` is rejected with `ExceededRollback`. | NEW — `DC-CONS-05` (k-block rollback bound) |
| 1.8 | Rollback semantics: `rollback(state, depth)` produces state byte-identical to truncated-replay from a checkpoint. Rollback before the immutable tip is rejected with `ForkBeforeImmutableTip`. | NEW — `DC-CONS-06` (rollback = truncated replay) |
| 1.9 | `EraSchedule` is a typed BLUE-consumed value, anchored to `BootstrapAnchorHash`. Era↔slot translation is total and pure. | NEW — `DC-CONS-07` (EraSchedule canonical authority) |
| 1.10 | Slot→time = pure function `(EraSchedule, SystemStart, SlotNo) → UtcInstant`. No wall clock in BLUE. | NEW — `DC-CONS-08` (slot→time pure) |
| 1.11 | Forecast horizon: queries for slots beyond the safe zone return `OutsideForecastRange`, never guessed values. Bound derived from era history + safe zone + HFC, not a magic constant. | NEW — `DC-CONS-09` (forecast horizon fail-fast) |
| 1.12 | Header validation binds exactly to the accepted body and consensus context | **CN-CONS-04** strengthened |
| 1.13 | Consensus decisions do not depend on wall-clock, arrival-order, scheduler, or OS behaviour | **CN-CONS-05** strengthened |
| 1.14 | Op-cert counter is monotonic per `(pool, kes_period)` window; out-of-order op-certs are rejected | NEW — `DC-CONS-10` (op-cert counter monotonicity) |

## 2. What must never be possible

| # | Forbidden | Anchor |
|---|---|---|
| 2.1 | Best-chain selection that depends on wall-clock arrival time of blocks | CN-CONS-05 |
| 2.2 | Best-chain selection that depends on `HashMap`/`HashSet` iteration order | DC-CORE-01 |
| 2.3 | Two honest nodes producing different best-chains on the same candidate set + ledger view + schedule | CN-CONS-01 |
| 2.4 | Slot-leader inferred from anything other than `(slot, vrf_key, stake_dist, asc, epoch_nonce)` | DC-CONSENSUS-02 |
| 2.5 | Rollback >k blocks accepted silently | DC-CONS-05 |
| 2.6 | Fork before the immutable tip incorporated into fork-choice | DC-CONS-06 |
| 2.7 | HFC era transition at a slot other than the schedule-defined boundary | DC-EPOCH-02 |
| 2.8 | Slot→time using `wall_clock()` | DC-CONS-08 |
| 2.9 | Floating-point arithmetic in fork-choice scoring or leader-schedule computation | T-CORE-02 |
| 2.10 | BLUE N-B receiving `&ChainDb` or reading filesystem | DC-CORE-01 |
| 2.11 | BLUE N-B parsing genesis JSON/text | DC-CORE-01 |
| 2.12 | N-B rederiving stake snapshots independently of ledger | DC-CONSENSUS-02 |
| 2.13 | Inspecting block bodies for fork-choice tip comparison | i (cluster boundary) |
| 2.14 | Forecast-horizon query returning a guessed answer for far-future slots | DC-CONS-09 |
| 2.15 | Op-cert counter regression accepted | DC-CONS-10 |
| 2.16 | Density-based ordering applied to caught-up Praos fork-choice | DC-CONS-03 |

## 3. What must remain identical across executions

- **Best-chain selection**: `(candidate_fragments, EraSchedule, ledger_view, params)` → same `ChainHash` or same `ChainSelectionReject`
- **Praos `TiebreakerView`**: same `(slot, issuer, op_cert_no, vrf_output)` quadruples → same ordering
- **`PraosChainDepState` evolution**: same `(state, header)` → same next state
- **Leader schedule**: same `(epoch_nonce, stake_snapshot_from_ledger_view, asc, vrf_key)` → byte-identical `is_leader` answer and `expected_vrf_proof`
- **Rollback**: same `(state, depth)` → same rolled-back state hash or same structured rejection
- **EraSchedule translation**: same `(EraSchedule, slot)` → same `(era_idx, epoch_no, relative_slot)`
- **Slot→time**: same `(EraSchedule, SystemStart, slot)` → same `UtcInstant`

## 4. What must be replay-equivalent

Given an ordered canonical stream of `(HeaderArrival | EpochBoundary | RollBackRequest)`, the BLUE consensus runtime produces:

- Identical sequence of `ChainEvent` (`ChainExtended`, `RolledBack`, `RolledForward`, `ChainSelected`, `Rejected{reason: ChainSelectionReject}`)
- Identical `PraosChainDepState` at every checkpoint (chain-tip, epoch boundary, k-deep immutable boundary)
- Identical leader-schedule answers for every queried `(slot, pool_id)`

Canonical corpus to be created by N-B's slices: `corpus/consensus/{fork_choice,rollback,leader_schedule,hfc_schedule,nonce_evolution}/`. Mechanically enforced by replay tests. Anchored by **T-DET-01**, **DC-CONSENSUS-01**.

## 5. State transitions in scope

All transitions pure. Per-protocol agency types — no generic `transition` function spans these state machines.

```rust
ForkChoice
  (ChainSelectorState, CandidateFragment, &LedgerView, &EraSchedule, &ProtocolParams)
  -> Result<(ChainSelectorState, ChainEvent), ForkChoiceError>

RollBack
  (ChainSelectorState, RollBackRequest{to_point, depth_blocks})
  -> Result<(ChainSelectorState, RollBackEvent), ChainSelectionReject>

PraosHeaderValidate
  (PraosChainDepState, ValidatedHeaderSummary, &LedgerView, &EraSchedule)
  -> Result<(PraosChainDepState, HeaderValid|HeaderInvalid{reason}), HeaderValidationError>

LeaderSchedule
  (LeaderScheduleQuery{epoch, slot, pool}, &LedgerView, &EraSchedule, &PraosChainDepState)
  -> Result<LeaderScheduleAnswer, LeaderScheduleError | OutsideForecastRange>

VRFCertVerify
  (VrfCert, VrfInput{slot, epoch_nonce}, VrfPublicKey)
  -> Result<VrfOutput, VrfCertError>      ; delegates to ade_crypto::verify_vrf

NonceEvolution
  (PraosChainDepState, NonceInput{header_vrf_output | EpochBoundary})
  -> Result<PraosChainDepState, NonceEvolutionError>

OpCertCounterCheck
  (OpCertCounterMap, pool_id, kes_period, observed_counter)
  -> Result<OpCertCounterMap, OpCertCounterError{regression}>

HFCSchedule
  (EraSchedule, SlotNo) -> Result<(EraIndex, EpochNo, RelativeSlot), HFCError>

SlotTime
  (EraSchedule, SystemStart, SlotNo) -> Result<UtcInstant, SlotTimeError | OutsideForecastRange>
```

Closed event taxonomies:
```rust
ChainEvent = ChainExtended | RolledBack | RolledForward | ChainSelected | Rejected(ChainSelectionReject)

ChainSelectionReject =
  | ForkBeforeImmutableTip { immutable_tip: Point, candidate_intersection: Point, rollback_depth: BlockDistance, security_param: SecurityParam }
  | ExceededRollback       { requested: BlockDistance, max: SecurityParam }
  | HeaderInvalid          { reason: HeaderValidationError }
  | TiebreakerLossKeepCurrent { current_tip: Point, candidate_tip: Point }
```

## 6. TCB color hypothesis (revised)

**BLUE — deterministic authoritative core**
- Praos header validation (against `LedgerView` + `EraSchedule`)
- Fork choice on `CandidateFragment` inputs (block-number → TiebreakerView)
- Rollback transition (bounded by k blocks + immutable tip)
- `PraosChainDepState` (nonce evolution + op-cert counters)
- Leader schedule query (pure function of canonical inputs)
- VRF cert verification wiring (delegates to ade_crypto BLUE)
- `EraSchedule` translation (era↔slot↔time)
- All `ChainSelectionReject` reason construction

**GREEN — deterministic glue, non-authoritative**
- Candidate-fragment materialization: reads N-D ChainDB, validates headers via BLUE, packages `CandidateFragment` for BLUE fork-choice
- `EraSchedule` typed value (materialized once at startup; immutable thereafter)
- Chain-selector orchestrator that subscribes to N-A chain-sync events and feeds BLUE
- Forecast-window bound derivation (from era history + safe zone)

**RED — nondeterministic shell**
- Genesis text → `EraSchedule` parsing (with hash binding)
- Stake-snapshot disk loader (BLUE consumes the loaded snapshot via typed `LedgerView`)
- "Current wall clock → current slot" mapping for operator queries (never inside BLUE)
- N-A chain-sync driver (already RED)
- N-D ChainDB adapter (already RED)

**Open**: none. Closure pass resolved all GREEN-vs-RED placement questions.

## 7. Open questions

None blocking `/cluster-plan`. One residual item for **code-lock time** only:

- **a-residual**: pin the Praos `TiebreakerView` ordering rule to the exact `ouroboros-consensus` revision shipped with the target cardano-node version (currently 11.0.1 → ouroboros-consensus 0.21+ approx, must verify) before merging the fork-choice slice. Recorded as a slice-entry proof obligation for the fork-choice slice.

## Registry entries (8 NEW; flagged for confirmation)

```toml
[[rules]]
id = "DC-CONS-03"
name = "Praos chain selection ordering: block-number first, TiebreakerView second"
invariant = """
Chain selection compares block number first; equal-block-number candidates are
ordered by the protocol-specific Praos TiebreakerView (slot, issuer, op-cert
issue number, VRF output). Density-based ordering is reserved for Genesis /
catch-up logic and must not be used for caught-up Praos fork-choice.
"""
family = "DC"
source = "Project constitution §3, ouroboros-consensus (pin at code-lock); CN-CONS-01"
kind = "authority"
introduced_in = "PHASE4-N-B"
status = "active"

[[rules]]
id = "DC-CONS-04"
name = "Praos chain-dep state (nonce + op-cert counters) is consensus authority"
invariant = """
Epoch nonce evolution (evolving/candidate/epoch/previous_epoch/lab/
last_epoch_block nonces) and op-cert counter evolution are owned by N-B
consensus, not by the ledger. They evolve deterministically as a function of
validated headers and epoch boundaries.
"""
family = "DC"
source = "Project constitution §3, ouroboros-consensus PraosChainDepState"
kind = "authority"
introduced_in = "PHASE4-N-B"
status = "active"

[[rules]]
id = "DC-CONS-05"
name = "Rollback is bounded by k blocks"
invariant = """
Authoritative rollback must never exceed the protocol security parameter k
(mainnet k = 2160 blocks). Rollback requests deeper than k return
ExceededRollback. Forecast/stability windows may be slot-based, but they must
accommodate at least k+1 blocks where header/block adoption is required.
"""
family = "DC"
source = "Project constitution §3, ouroboros-consensus consensus report"
kind = "authority"
introduced_in = "PHASE4-N-B"
status = "active"

[[rules]]
id = "DC-CONS-06"
name = "Rollback semantics: state-equivalent to truncated replay; immutable tip is final"
invariant = """
rollback(state, depth) must produce state byte-identical to truncated replay
from the nearest checkpoint. Rollback that would cross the immutable tip
(blocks ≥ k deep) returns ForkBeforeImmutableTip and never alters state.
"""
family = "DC"
source = "Project constitution §3, T-DET-01"
kind = "replay"
introduced_in = "PHASE4-N-B"
status = "active"

[[rules]]
id = "DC-CONS-07"
name = "EraSchedule is a typed BLUE-consumed value, anchored to bootstrap hash"
invariant = """
BLUE consensus must consume the HFC schedule only as a typed EraSchedule value
anchored to BootstrapAnchorHash. Genesis text parsing happens in RED; BLUE
never reads files, JSON, or operator config directly. The schedule is part of
replay evidence.
"""
family = "DC"
source = "Project constitution §3, DC-CORE-01, DC-EPOCH-02"
kind = "authority"
introduced_in = "PHASE4-N-B"
status = "active"

[[rules]]
id = "DC-CONS-08"
name = "Slot→time is pure of wall clock"
invariant = """
slot_to_time(EraSchedule, SystemStart, SlotNo) is a pure function of its
arguments; no BLUE consensus path may consult the wall clock to derive a slot
or a UTC instant for an authoritative decision.
"""
family = "DC"
source = "Project constitution §3, CN-CONS-05, DC-CORE-01"
kind = "determinism"
introduced_in = "PHASE4-N-B"
status = "active"

[[rules]]
id = "DC-CONS-09"
name = "Forecast horizon stops at the safe zone; out-of-range is structured error"
invariant = """
Consensus-derived queries for slots beyond the ledger-view safe zone return
OutsideForecastRange, never guessed values. The bound is derived from era
history + safe zone + HFC schedule, not encoded as a magic constant in
caller code.
"""
family = "DC"
source = "Project constitution §3, DC-EPOCH-02"
kind = "fail-fast"
introduced_in = "PHASE4-N-B"
status = "active"

[[rules]]
id = "DC-CONS-10"
name = "Op-cert counter is monotonic per (pool, kes_period)"
invariant = """
A header's op-cert issue counter must be ≥ the highest observed counter for
the same (pool, kes_period). Regression results in HeaderInvalid with a typed
OpCertCounterError reason; ChainDepState never accepts a regression.
"""
family = "DC"
source = "Project constitution §3, ouroboros-consensus OperationalCertificate"
kind = "authority"
introduced_in = "PHASE4-N-B"
status = "active"
```

**Existing rules strengthened**: DC-CONSENSUS-01, DC-CONSENSUS-02, DC-EPOCH-02, DC-CRYPTO-01, CN-CONS-01..05, CN-CRYPTO-02, CN-EPOCH-01..03, CN-CONS-04.
