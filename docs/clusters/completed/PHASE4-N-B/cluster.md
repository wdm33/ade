# Cluster PHASE4-N-B — Consensus Runtime (Praos)

> **Tier**: 1 semantic. Decisions affect which blocks Ade accepts.
> **Status**: opening — slice work begins with S-B1
> **Origin**: Phase 4 cluster N-B from `docs/active/phase_4_cluster_plan.md`
> **Planning trio**:
> - `docs/planning/phase4-n-b-invariants.md` (invariants v2; closure table closed)
> - `docs/planning/phase4-n-b-cluster-slice-plan.md` (ratified 10-slice plan)
> - this document — cluster spec

## Primary invariant

> Best-chain selection is a deterministic, pure function of
> `(candidate_fragments, EraSchedule, ledger_view, protocol_params)` →
> `ChainHash | ChainSelectionReject`. Two honest Ade nodes given the
> same inputs must produce the same best-chain answer, the same
> evolving `PraosChainDepState`, and the same leader-schedule answers
> — across every execution, on every host, forever.

Anchored by `DC-CONSENSUS-01` (strengthened), `DC-CONS-03..10` (NEW),
and a strengthening pass over the existing consensus-family rules
listed in `docs/planning/phase4-n-b-invariants.md` §"Registry entries".

## Normative anchors

- `docs/ade-invariant-registry.toml` — existing consensus-family rules
  to be strengthened: `DC-CONSENSUS-{01,02}`, `DC-CRYPTO-01`,
  `CN-CONS-{01..05}`, `CN-CRYPTO-02`, `CN-EPOCH-{01..03}`,
  `DC-EPOCH-02`. 8 NEW: `DC-CONS-03..10`.
- `docs/active/CE-79_gate_statement.md` — Tier 1 (must conform).
- `docs/active/comparison_surface_contract.md` — header→body binding
  inherits the dual-authority surface rule.
- `docs/active/phase_4_cluster_plan.md` § N-B — headline CEs.
- `docs/planning/phase4-n-b-invariants.md` — full sketch with closure
  table.
- External: cardano-node 11.0.1 + ouroboros-consensus reference for
  Praos `TiebreakerView` ordering (matches N-A's CE-N-A-5 interop
  target). Exact revision pinned at S-B8 entry; `PraosTiebreaker`
  shape is forward-compatible from 10.6.2.

---

## Entry Conditions

What previous work guarantees N-B can rely on:

- **PHASE4-N-A closed** (`69a2862`): chain-sync, block-fetch, and the
  rest of the N2N/N2C mini-protocols carry validated bytes. BLUE N-B
  receives `ValidatedHeaderSummary` from GREEN N-A glue; never reads
  `&Mux`, `&ChainDb`, or raw frames.
- **PHASE4-N-D in-flight**: durable storage exists. BLUE rollback
  receives `immutable_tip: Point` as a canonical typed input; it does
  not call ChainDb directly. Where N-D is not yet feature-complete,
  S-B9 declares the minimal contract it requires.
- **`ade_types` is stable**: `SlotNo`, `BlockNo`, `EpochNo`, `Hash32`,
  `CardanoEra` exist and are reused, never redefined.
- **`ade_crypto::verify_vrf`** is BLUE and stable (extractive VRF
  verification returning `VrfOutput`).
- **`ade_ledger`** exposes per-epoch state and stake snapshots
  reachable through a typed forecast boundary; S-B6 declares the
  `LedgerView` trait surface it consumes.
- **Registry has ≥149 entries**; N-B appends 8 new rules and
  strengthens ≥10 existing ones via cluster close.

---

## Exit Criteria (CI-Verifiable)

Each CE names the concrete test that closes it. Tests are
forward-defined — they land with the slice that closes the CE. No
human review may substitute for these checks.

| CE | Check | Closed by |
|---|---|---|
| **CE-N-B-1** | `cargo test -p ade_core --test fork_choice_corpus` PASS over curated multi-tip divergence corpus at `corpus/consensus/fork_choice/` (block-number first, then `TiebreakerView`); rejection-reason byte-identity validated for `ForkBeforeImmutableTip`, `ExceededRollback`, `HeaderInvalid`, `TiebreakerLossKeepCurrent` | S-B8 |
| **CE-N-B-2** | `cargo test -p ade_core --test rollback_corpus` PASS over curated rollback corpus at `corpus/consensus/rollback/`; truncated-replay equivalence holds; `ExceededRollback` (k = 2160 mainnet) and `ForkBeforeImmutableTip` are byte-identical with oracle | S-B9 |
| **CE-N-B-3** | `cargo test -p ade_core --test hfc_schedule_corpus` PASS over `corpus/consensus/hfc_schedule/`; `EraSchedule` translation matches oracle's known hard-fork slots exactly; bound to `BootstrapAnchorHash`; slot→time is pure | S-B1 |
| **CE-N-B-4** | `cargo test -p ade_core --test leader_schedule_corpus` PASS over `corpus/consensus/leader_schedule/`; `is_leader` and `expected_vrf_proof` are byte-identical with oracle; `OutsideForecastRange` fires deterministically | S-B6 |
| **CE-N-B-5** | `cargo test -p ade_testkit --test consensus_stream_replay` PASS — ordered canonical stream `(HeaderArrival | EpochBoundary | RollBackRequest)` produces identical `ChainEvent` sequence and `PraosChainDepState` at every checkpoint across two consecutive runs | S-B10 |
| **CE-N-B-6** | `cargo test -p ade_core_interop --test live_consensus_session --release -- --ignored` PASS against pinned cardano-node 11.0.1; transcripts captured at `docs/clusters/PHASE4-N-B/CE-N-B-6_<date>.log` | S-B10 |

---

## Expected Slice Types

- **S-B1** — Canonical-type slice (BLUE + RED genesis parser): typed
  `EraSchedule` + slot/era/time translation.
- **S-B2** — Canonical-type slice (BLUE): `PraosChainDepState` shape
  + closed event/error taxonomies.
- **S-B3** — State-transition slice (BLUE): VRF wiring + leader
  threshold.
- **S-B4** — State-transition slice (BLUE): nonce evolution.
- **S-B5** — State-transition slice (BLUE): op-cert counter.
- **S-B6** — State-transition slice (BLUE): leader-schedule query.
- **S-B7** — State-transition slice (BLUE): Praos header validation.
- **S-B8** — State-transition slice (BLUE + GREEN materializer): fork
  choice + `CandidateFragment`.
- **S-B9** — State-transition slice (BLUE): rollback + k-bound +
  immutable-tip refusal.
- **S-B10** — Replay + composition slice (GREEN + RED interop): stream
  replay harness, chain-selector orchestrator, live cardano-node
  interop.

---

## TCB Color Map (FC/IS Partition)

| Module | Color | Constraint |
|---|---|---|
| `ade_core::consensus::era_schedule` | **BLUE** | Pure translation: slot↔era↔epoch, slot→time. No I/O. |
| `ade_core::consensus::praos_state` | **BLUE** | `PraosChainDepState`, closed event/error enums, canonical encoding. |
| `ade_core::consensus::vrf_cert` | **BLUE** | `VRFCertVerify` wiring + integer-arithmetic threshold check. |
| `ade_core::consensus::nonce` | **BLUE** | `NonceEvolution` transition. |
| `ade_core::consensus::op_cert` | **BLUE** | Op-cert counter monotonicity. |
| `ade_core::consensus::leader_schedule` | **BLUE** | Pure leader-schedule query; `OutsideForecastRange`. |
| `ade_core::consensus::header_validate` | **BLUE** | `PraosHeaderValidate`; composes VRF/nonce/op-cert/EraSchedule/LedgerView. |
| `ade_core::consensus::fork_choice` | **BLUE** | `ForkChoice` over `CandidateFragment`; block-number → `TiebreakerView`. |
| `ade_core::consensus::rollback` | **BLUE** | `RollBack` transition; k-bound; immutable tip refusal. |
| `ade_runtime::consensus::genesis_parser` | **RED** | Genesis text → `EraSchedule` with `BootstrapAnchorHash` binding. |
| `ade_runtime::consensus::chain_selector` | **GREEN** | Orchestrator: subscribes to N-A events, materializes `CandidateFragment`, feeds BLUE. Non-authoritative. |
| `ade_runtime::consensus::candidate_fragment` | **GREEN** | Materializes `CandidateFragment { anchor, headers, select_view, rollback_depth }` from N-D + N-A. |
| `ade_testkit::consensus` | **GREEN** | Replay corpus harness; `consensus_stream_replay` driver. |
| `crates/ade_core_interop/` | **RED** | Live cardano-node interop driver binary (S-B10). |

Rules (inherit per global IDD doctrine):
- No RED behavior may appear in BLUE code.
- GREEN code must not affect authoritative outputs.

---

## Forbidden During This Cluster

Slice-level hard prohibitions inherit:

- BLUE receiving `&ChainDb`, `&Mux`, or parsing genesis text —
  `DC-CORE-01`.
- Density-based ordering in caught-up Praos fork-choice — `DC-CONS-03`.
- Wall-clock reads in BLUE (`std::time`, `Instant`, `SystemTime`) —
  `DC-CONS-08`, `CN-CONS-05`.
- `HashMap` / `HashSet` iteration order on any authority path —
  `DC-CORE-01`.
- Floating-point arithmetic in fork-choice scoring, leader-schedule
  math, or VRF threshold check — `T-CORE-02`.
- Stake-snapshot rederivation in N-B (consume `LedgerView` only) —
  `DC-CONSENSUS-02`.
- Body inspection for fork-choice tip comparison — sketch decision (i).
- Plugin-style runtime registration of consensus protocols (closed
  enums only).
- Async/await or `tokio::` in BLUE (`DC-CORE-01`; same check as N-A's
  `ci_check_no_async_in_blue.sh` once `ade_core::consensus` is in scope).
- "We'll match it later" stubs on Tier 1 authority surfaces.
- TODO/placeholder error variants — every reject reason is structured.

---

## Slices

| ID | Name | TCB | Closes |
|---|---|---|---|
| **S-B1** | `EraSchedule` canonical authority + slot/era/time translation | BLUE + RED genesis parser | **CE-N-B-3** |
| **S-B2** | `PraosChainDepState` canonical type + closed event/error taxonomies | BLUE | — (substrate) |
| **S-B3** | VRF cert verification wiring + Praos VRF input + leader threshold | BLUE | — (substrate for CE-N-B-4) |
| **S-B4** | Nonce evolution authority | BLUE | — (substrate for CE-N-B-4) |
| **S-B5** | Op-cert counter monotonicity | BLUE | — (substrate for header-validate) |
| **S-B6** | Leader schedule | BLUE | **CE-N-B-4** |
| **S-B7** | Praos header validation | BLUE | — (substrate for fork-choice) |
| **S-B8** | Fork choice + `CandidateFragment` | BLUE + GREEN | **CE-N-B-1** |
| **S-B9** | Rollback authority + k-bound + immutable tip refusal | BLUE | **CE-N-B-2** |
| **S-B10** | Stream replay + chain-selector orchestrator + live interop | GREEN + RED | **CE-N-B-5**, **CE-N-B-6** |

Slice docs (`S-B1.md` ... `S-B10.md`) live in this directory; one is
written by `/slice-doc S-BN` before implementation.

---

## Engineering Surface (Forward-Looking)

```
crates/ade_core/
  src/
    lib.rs
    consensus/
      mod.rs
      era_schedule.rs          # S-B1 BLUE
      praos_state.rs           # S-B2 BLUE
      vrf_cert.rs              # S-B3 BLUE
      nonce.rs                 # S-B4 BLUE
      op_cert.rs               # S-B5 BLUE
      leader_schedule.rs       # S-B6 BLUE
      header_validate.rs       # S-B7 BLUE
      fork_choice.rs           # S-B8 BLUE
      rollback.rs              # S-B9 BLUE
      events.rs                # S-B2 closed event/error enums

crates/ade_runtime/
  src/
    consensus/
      mod.rs                   # S-B10
      genesis_parser.rs        # S-B1 RED (with hash binding)
      chain_selector.rs        # S-B10 GREEN
      candidate_fragment.rs    # S-B8 GREEN

crates/ade_testkit/
  src/
    consensus/
      mod.rs                   # S-B10
      corpus.rs                # GREEN harness for the 5 corpus dirs

crates/ade_core_interop/       # NEW crate S-B10
  Cargo.toml
  src/
    bin/
      live_consensus_session.rs

corpus/consensus/
  hfc_schedule/                # S-B1
  nonce_evolution/             # S-B4
  leader_schedule/             # S-B6
  fork_choice/                 # S-B8
  rollback/                    # S-B9
```

CI scripts (new):

```
ci/ci_check_no_chaindb_in_consensus_blue.sh   # enforces DC-CORE-01 for ade_core::consensus
ci/ci_check_no_float_in_consensus.sh          # T-CORE-02 narrow to consensus
ci/ci_check_consensus_closed_enums.sh         # DC-CONS-03..10 enforce closed reject enums
```

`ade_core::consensus::*` is added to `.idd-config.json` `core_paths`
in S-B1 (BLUE crate already in scope; submodule paths join the
existing BLUE list).

---

## Replay Obligations (Cluster-Level)

- **New canonical types**: `EraSchedule`, `BootstrapAnchorHash`,
  `EraSummary`, `PraosChainDepState`, `OpCertCounterMap`,
  `CandidateFragment`, `ValidatedHeaderSummary`, `ChainEvent`,
  `ChainSelectionReject`, `HeaderValidationError`, `VrfInput`,
  `VrfCertError`, `OpCertCounterError`, `NonceEvolutionError`,
  `LeaderScheduleQuery`, `LeaderScheduleAnswer`, `LeaderScheduleError`,
  `OutsideForecastRange`, `RollBackRequest`, `RollBackEvent`,
  `SlotTimeError`, `HFCError`. Each round-trip-tested in the slice
  that introduces it.
- **New replay corpus**: `corpus/consensus/{hfc_schedule,
  nonce_evolution, leader_schedule, fork_choice, rollback}/`. Per-corpus
  metadata schema (oracle source — cardano-node version, network magic,
  inputs, expected outputs, expected reject reason if any).
- **Per-slice replay equivalence MAC**: every BLUE transition slice's
  tests replay byte-identically.
- **Cluster-level replay MAC**: replayed end-to-end, the corpus
  produces the same `ChainEvent` sequence and `PraosChainDepState` as
  a parallel cardano-node 11.0.1 oracle session (`CE-N-B-5` +
  `CE-N-B-6`).

---

## Invariants Strengthened By This Cluster

| Slice | Strengthens (registry status flip pending until enforcement lands) |
|---|---|
| S-B1 | `DC-CONS-07/08/09` (NEW); `DC-EPOCH-02` strengthened |
| S-B2 | `DC-CONS-04` (NEW, type shape); `T-DET-01` (closed canonical encoding) |
| S-B3 | `DC-CRYPTO-01`, `CN-CRYPTO-02`, `T-CORE-02` |
| S-B4 | `DC-CONS-04` (NEW, behavior) |
| S-B5 | `DC-CONS-10` (NEW) |
| S-B6 | `DC-CONSENSUS-02`, `CN-EPOCH-01` |
| S-B7 | `CN-CONS-04`, `DC-CONS-04` |
| S-B8 | `DC-CONS-03` (NEW), `DC-CONSENSUS-01`, `CN-CONS-{01..05}` |
| S-B9 | `DC-CONS-05/06` (NEW) |
| S-B10 | `T-DET-01`, `DC-CONSENSUS-01`, `DC-CORE-01` (live interop closes the trans-runtime equivalence claim) |

---

## Authority Reminder

This cluster doc is a planning aid. Authority for invariants belongs
to `docs/ade-invariant-registry.toml`. Authority for mechanical
acceptance belongs to the named tests/CI checks above. If guidance
here conflicts with normative documents, normative documents win.
