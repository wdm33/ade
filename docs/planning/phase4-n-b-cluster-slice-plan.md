# Cluster Slice Plan — Ade Phase 4 / Cluster PHASE4-N-B

> **Status**: planning artifact, non-normative. Authority lives in
> `docs/ade-invariant-registry.toml` and `docs/active/phase_4_cluster_plan.md`.
> This document orders the slices that close PHASE4-N-B; it does not
> introduce new requirements.

## Inputs

- `docs/planning/phase4-n-b-invariants.md` — invariant sketch v2 (closed).
- `docs/active/phase_4_cluster_plan.md` § N-B — headline CEs and tier rationale.
- `docs/ade-invariant-registry.toml` — rules to strengthen + 8 NEW (`DC-CONS-03..10`).
- `~/.claude/methodology/idd.md` Part I §§1–10 and Part IV.

## Cluster Index (Dependency Order — within Phase 4)

Phase 4 cluster order is fixed by `docs/active/phase_4_cluster_plan.md`
(N-D → N-A → **N-B** → N-E → N-F → N-C). N-B is the cluster being
planned here; it consumes:

- **N-A** (Tier 1, closed) — canonical chain-sync / block-fetch ingress;
  GREEN `ValidatedHeaderSummary` materialization for BLUE consumption.
- **N-D** (Tier 5, in-flight) — durable storage and the immutable-tip
  notion that N-B's rollback bound depends on.

N-B in turn gates **N-E** (mempool needs current chain state), **N-F**
(queries return consensus answers), and **N-C** (block production reads
leader schedule + chain-dep state).

---

## Cluster PHASE4-N-B — Consensus Runtime (Praos)

**Primary invariant:**
> Best-chain selection is a deterministic, pure function of
> `(candidate_fragments, EraSchedule, ledger_view, protocol_params)` →
> `ChainHash | ChainSelectionReject`. Two honest Ade nodes given the
> same inputs must produce the same best-chain answer, the same
> evolving `PraosChainDepState`, and the same leader-schedule answers
> — across every execution, on every host, forever.

**Normative anchors:**
- `docs/planning/phase4-n-b-invariants.md` (closed sketch, v2)
- `docs/ade-invariant-registry.toml` — strengthen `DC-CONSENSUS-{01,02}`,
  `DC-CRYPTO-01`, `CN-CONS-{01..05}`, `CN-CRYPTO-02`, `CN-EPOCH-{01..03}`,
  `CN-CONS-04`, `DC-EPOCH-02`
- 8 NEW rules: `DC-CONS-03..10`
- `docs/active/phase_4_cluster_plan.md` § N-B
- `docs/active/CE-79_gate_statement.md` (Tier 1 — must conform)

**Entry conditions:**
- N-A header arrival surface stable: `ValidatedHeaderSummary` materialized
  by GREEN N-A glue; BLUE never reads `&ChainDb` or `&Mux`.
- N-D immutable-tip notion exists: BLUE rollback receives
  `immutable_tip: Point` as a canonical input.
- `ade_crypto::verify_vrf` is BLUE and stable.
- `LedgerView` from Phase 3 ledger surfaces forecasted stake snapshots
  through a typed boundary.

**TCB partition** (per §6 of the sketch — closed):
- **BLUE**:
  `ade_core::consensus::{praos, fork_choice, rollback, era_schedule,
  leader_schedule, nonce, op_cert, header_validate}`; closed event /
  error taxonomies; `EraSchedule` translation.
- **GREEN**: `ade_runtime` chain-selector orchestrator (subscribes to
  N-A events, materializes `CandidateFragment`, feeds BLUE); GREEN
  `EraSchedule` builder (immutable post-startup); forecast-window bound
  derivation.
- **RED**: genesis-text → `EraSchedule` parser (with hash binding);
  stake-snapshot disk loader (BLUE consumes typed result); "wall-clock
  → current slot" mapping for operator queries only.

**Cluster Exit Criteria** (complete-work-only — every CE closes on a
specific slice; no carry-forward):

- **CE-N-B-1** — Fork choice produces identical best-chain selection
  on every multi-tip case in a curated divergence corpus, and
  byte-identical `ChainSelectionReject` reasons on every rejection
  case. *(closes in S-B8.)*
- **CE-N-B-2** — Rollback to k-deep produces identical post-rollback
  state as oracle for a curated rollback corpus; `ForkBeforeImmutableTip`
  and `ExceededRollback` fire byte-identically. *(closes in S-B9.)*
- **CE-N-B-3** — `EraSchedule` matches oracle's known hard-fork slots
  exactly and is anchored to `BootstrapAnchorHash`; slot↔era↔epoch
  translation and slot→time are pure and replay-equivalent. *(closes
  in S-B1.)*
- **CE-N-B-4** — Leader schedule produces identical `is_leader` answers
  and `expected_vrf_proof` values as oracle for a curated epoch-replay
  corpus. *(closes in S-B6, with VRF/nonce dependencies from S-B3/S-B4.)*
- **CE-N-B-5** — Replay equivalence: ordered canonical stream
  `(HeaderArrival | EpochBoundary | RollBackRequest)` produces
  identical `ChainEvent` sequence and `PraosChainDepState` at every
  checkpoint; corpus
  `corpus/consensus/{fork_choice,rollback,leader_schedule,hfc_schedule,nonce_evolution}/`
  exists and is replay-stable in CI. *(closes in S-B10.)*
- **CE-N-B-6** — Live interop: Ade's consensus runtime, fed real
  headers/rollbacks from a real cardano-node peer over N-A, produces a
  best-chain identical to that peer's for a sustained window. Patterned
  on the S-A10 live-interop precedent. *(closes in S-B10.)*

**Forbidden during this cluster** (slice-level prohibitions inherit):
- BLUE receiving `&ChainDb`, `&Mux`, or parsing genesis text —
  `DC-CORE-01`.
- Density-based ordering in caught-up Praos fork-choice — `DC-CONS-03`.
- Wall-clock reads in BLUE — `DC-CONS-08`, `CN-CONS-05`.
- `HashMap` / `HashSet` iteration order anywhere on the authority path
  — `DC-CORE-01`.
- Floating-point in fork-choice scoring or leader-schedule math —
  `T-CORE-02`.
- Stake-snapshot rederivation in N-B (consume ledger view only) —
  `DC-CONSENSUS-02`.
- Body inspection for tip comparison — sketch decision (i).
- "We'll match it later" stubs on Tier 1 surfaces.

**Replay obligations introduced** (each owned by the slice that
introduces it):
- `corpus/consensus/hfc_schedule/` — S-B1
- `corpus/consensus/nonce_evolution/` — S-B4
- `corpus/consensus/leader_schedule/` — S-B6
- `corpus/consensus/fork_choice/` — S-B8
- `corpus/consensus/rollback/` — S-B9
- End-to-end consensus stream replay — S-B10

**New canonical types introduced**: `EraSchedule`, `BootstrapAnchorHash`,
`PraosChainDepState`, `CandidateFragment`, `ValidatedHeaderSummary`,
`ChainEvent`, `ChainSelectionReject`, `HeaderValidationError`,
`LeaderScheduleQuery` / `Answer`, `VrfInput` / `VrfCert` consensus-side
wrapper, `OpCertCounterMap`, `OutsideForecastRange`. All BLUE, all
canonical-encoded, all replay-anchored.

---

### Slices

> Slices are ordered by dependency. Each is a mergeable unit that
> leaves the system in a fully correct state; no slice weakens an
> existing invariant.

- **S-B1 — `EraSchedule` canonical authority + slot/era/time translation**
  Introduces typed `EraSchedule` (BLUE-consumed), `BootstrapAnchorHash`,
  pure `(EraSchedule, slot) → (era, epoch, relative_slot)` and
  `(EraSchedule, SystemStart, slot) → UtcInstant`, structured
  `OutsideForecastRange`, RED genesis-text parser with hash binding.
  Invariants: 1.9, 1.10, 1.11. Addresses **CE-N-B-3**. Anchors:
  `DC-CONS-07/08/09` (NEW), `DC-EPOCH-02` strengthened.
  TCB: BLUE (types + translation) + RED (genesis parser only).

- **S-B2 — `PraosChainDepState` canonical type + closed event/error taxonomies**
  Defines `PraosChainDepState` (evolving/candidate/epoch/previous_epoch/
  lab/last_epoch_block nonces + `OpCertCounterMap` + `last_slot`),
  canonical encoding, and the closed `ChainEvent` /
  `ChainSelectionReject` / `HeaderValidationError` / `OpCertCounterError`
  / `VrfCertError` / `LeaderScheduleError` enums.
  Invariants: 1.6 (shape only — evolution lands in S-B4/5/7). Anchors:
  `DC-CONS-04` (NEW, type shape), `T-DET-01`.
  TCB: BLUE.

- **S-B3 — VRF cert verification wiring + Praos VRF input + leader threshold**
  Wires `ade_crypto::verify_vrf` into a BLUE `VRFCertVerify` transition.
  Pure derivation of `VrfInput` from `(slot, epoch_nonce)`. Leader-value-
  below-threshold check (stake fraction × ASC) implemented in
  deterministic integer arithmetic — no floats.
  Invariants: 1.4. Anchors: `DC-CRYPTO-01` strengthened, `CN-CRYPTO-02`
  strengthened, `T-CORE-02` (no float).
  TCB: BLUE.

- **S-B4 — Nonce evolution authority**
  Implements `NonceEvolution` transition: header VRF output contribution
  via VRF hashing / range-extension; epoch boundary transition
  (evolving → candidate → epoch). Introduces
  `corpus/consensus/nonce_evolution/`.
  Invariants: 1.6. Anchors: `DC-CONS-04` (NEW, behavior).
  TCB: BLUE.

- **S-B5 — Op-cert counter monotonicity**
  `OpCertCounterCheck` transition; rejects regression per
  `(pool, kes_period)` with typed `OpCertCounterError`. Counter map
  persisted as part of `PraosChainDepState`.
  Invariants: 1.14. Anchors: `DC-CONS-10` (NEW).
  TCB: BLUE.

- **S-B6 — Leader schedule (CE-N-B-4 close)**
  `LeaderSchedule` query as pure function of
  `(LeaderScheduleQuery, &LedgerView, &EraSchedule, &PraosChainDepState)`.
  Consumes E−2 stake snapshot from the ledger view — never rederives.
  Returns `OutsideForecastRange` for queries beyond safe zone.
  Introduces `corpus/consensus/leader_schedule/`.
  Invariants: 1.3, 1.5, 1.11. Addresses **CE-N-B-4**. Anchors:
  `DC-CONSENSUS-02` strengthened, `CN-EPOCH-01` strengthened.
  TCB: BLUE.

- **S-B7 — Praos header validation**
  `PraosHeaderValidate` transition: composes S-B3 (VRF), S-B4 (nonce
  input), S-B5 (op-cert), S-B1 (`EraSchedule`), `LedgerView` to produce
  `ValidatedHeaderSummary` consumed by S-B8 fork-choice. Binds
  header ↔ accepted body ↔ consensus context.
  Invariants: 1.6, 1.12, 1.13. Anchors: `CN-CONS-04` strengthened,
  `DC-CONS-04`.
  TCB: BLUE.

- **S-B8 — Fork choice (CE-N-B-1 close)**
  `ForkChoice` transition over typed
  `CandidateFragment { anchor, headers, select_view, rollback_depth }`.
  Block-number-first ordering; tiebreaker = Praos `TiebreakerView`
  (slot, issuer, op-cert issue number, VRF output). Density forbidden
  in caught-up path. Slice-entry obligation: pin to exact
  `ouroboros-consensus` revision shipped with the target cardano-node
  version (sketch residual `a-residual`). Introduces
  `corpus/consensus/fork_choice/`.
  Invariants: 1.1, 1.2. Addresses **CE-N-B-1**. Anchors:
  `DC-CONS-03` (NEW), `DC-CONSENSUS-01` strengthened,
  `CN-CONS-{01..05}` strengthened.
  TCB: BLUE (fork-choice) + GREEN (`CandidateFragment` materializer
  reads N-D / N-A).

- **S-B9 — Rollback authority (CE-N-B-2 close)**
  `RollBack` transition. k-block bound enforcement (`ExceededRollback`,
  mainnet k = 2160). Immutable-tip refusal (`ForkBeforeImmutableTip`).
  State byte-identical to truncated replay from checkpoint. Introduces
  `corpus/consensus/rollback/`.
  Invariants: 1.7, 1.8. Addresses **CE-N-B-2**. Anchors:
  `DC-CONS-05/06` (NEW).
  TCB: BLUE.

- **S-B10 — Consensus replay corpus + chain-selector orchestrator + live interop (CE-N-B-5 + CE-N-B-6 close)**
  GREEN chain-selector orchestrator (subscribes to N-A
  `ValidatedHeaderSummary` events, materializes `CandidateFragment`,
  drives BLUE fork-choice + rollback). End-to-end consensus stream
  replay test over `corpus/consensus/`. Live cardano-node interop check
  on a curated divergence/rollback window — patterned on S-A10. Closes
  any residual coverage across CE-N-B-1..4.
  Invariants: §4 (replay-equivalence) end-to-end. Addresses
  **CE-N-B-5**, **CE-N-B-6**. Anchors: `T-DET-01`, `DC-CONSENSUS-01`,
  `DC-CORE-01`.
  TCB: GREEN (orchestrator) + BLUE (consumption paths already in
  S-B1..9).

---

### Cluster-wide replay obligations summary

| Slice | New corpus path | Replay anchor |
|---|---|---|
| S-B1 | `corpus/consensus/hfc_schedule/` | `DC-CONS-07` |
| S-B4 | `corpus/consensus/nonce_evolution/` | `DC-CONS-04` |
| S-B6 | `corpus/consensus/leader_schedule/` | `DC-CONSENSUS-02` |
| S-B8 | `corpus/consensus/fork_choice/` | `DC-CONS-03` / `DC-CONSENSUS-01` |
| S-B9 | `corpus/consensus/rollback/` | `DC-CONS-05` / `DC-CONS-06` |
| S-B10 | end-to-end stream replay + live interop | `T-DET-01` |

### FC/IS partition summary

| Slice | BLUE | GREEN | RED |
|---|---|---|---|
| S-B1 | `EraSchedule` types + translation, slot→time | — | genesis-text parser, hash binder |
| S-B2 | canonical state + closed enums | — | — |
| S-B3 | VRF wiring + threshold check | — | — |
| S-B4 | nonce evolution | — | — |
| S-B5 | op-cert counter | — | — |
| S-B6 | leader-schedule query | — | — |
| S-B7 | header validation | — | — |
| S-B8 | fork-choice transition | `CandidateFragment` materializer | — |
| S-B9 | rollback transition | — | — |
| S-B10 | (consumes existing BLUE only) | chain-selector orchestrator | live cardano-node peer driver |

---

## Authority reminder

This document is a planning aid. Authority for the rules referenced
above belongs to `docs/ade-invariant-registry.toml`. If this plan ever
conflicts with the registry, the registry wins.
