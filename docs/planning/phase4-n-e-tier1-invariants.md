# PHASE4-N-E (Tier 1 half) — invariant sketch

> **Status:** Planning artifact (non-normative). Produced via `/invariants`
> on 2026-05-25 against HEAD `52642e5`. Authority lives in
> `docs/ade-invariant-registry.toml` (`DC-MEM-01`, `DC-MEM-02`, `CN-MEM-01..04`
> and the new `DC-MEM-03` / `DC-MEM-04` to be appended on first-slice landing).
> If this doc conflicts with the registry, the registry wins.

## Scope

The **Tier-1 no-false-accept half** of PHASE4-N-E:

- Two RED ingress sources — N2N tx-submission2 (peer-to-peer) and N2C
  local-tx-submission (cardano-cli over UDS) — funnel through a deterministic
  GREEN canonicalizer into a *single* BLUE bridge:

    `mempool_ingress(mempool, IngressEvent) -> (MempoolState, AdmitOutcome)`

  which in turn calls the *existing* BLUE `mempool::admit` chokepoint
  (closed in CE-B2-5, `crates/ade_ledger/src/mempool/admit.rs`).

**Explicitly out of scope:**

- Outbound propagation (Ade serving txs to peers via tx-submission2) — a
  separate authority surface ("what we serve" ≠ "what we accept");
  belongs to a later N-E Tier-5 cluster or a fresh cluster.
- Mempool bounds / shedding policy beyond a simple deterministic hard cap if
  one is needed to prevent unbounded authoritative resource consumption.
  Full deterministic-shedding work stays Tier 5 unless the cap changes
  admission verdicts.

The framing per IDD Part I: **canonical ingress → preserved tx bytes → single
BLUE admission chokepoint → oracle-aligned no-false-accept evidence.**

## OQ resolutions (per 2026-05-25 review)

| OQ | Decision |
|---|---|
| OQ-1 | Both N2N TxSubmission2 and N2C LocalTxSubmission. N2C is bounty-critical (cardano-cli submit). Two RED ingress sources, one BLUE bridge. |
| OQ-2 | Close `DC-MEM-01` (no false accepts) here; hold `DC-MEM-02` / `CN-MEM-01` / `CN-MEM-03` unless a bound changes admission verdicts. Tier 1 stays narrow. |
| OQ-3 | Wrap the existing B-track adversarial corpus in synthetic `IngressEvent`s. Preserve tx bytes exactly; add only the canonical ingress envelope. |
| OQ-4 | One closed `IngressEvent { source: IngressSource, tx_bytes: PreservedCbor<Tx> }`. `source` is evidence/policy/replay metadata only — MUST NOT change the validity verdict. |
| OQ-5 | End at admission. Outbound propagation is a different authority surface; mixing weakens slice closure. |
| OQ-6 | BLUE bridge is single-step: `mempool_ingress(mempool, event) -> (mempool', outcome)`. GREEN canonicalizer may batch; BLUE state transition is one event at a time. Batch replay = fold. |

## 1. What must always be true (closed invariants)

- **N-E-1** — Mempool ≡ ledger/block validity. (Strengthens existing
  `DC-MEM-01`; N-E adds wire ingress to its `code_locus` and the ingress-replay
  tests to its `tests` array.)
- **N-E-2** — `mempool::admit` is the only mutator of `MempoolState`.
- **N-E-3** — Ingress trace replay is byte-identical. (New `DC-MEM-04`.)
- **N-E-4** — Tx ingress reduces to a closed `IngressEvent` before BLUE
  admission. (New `DC-MEM-03`.)
- **N-E-5** — `PreservedCbor` end-to-end; no decode-and-re-encode of body bytes.
- **N-E-6** — Intra-mempool dependency: B spending A admits iff A is admitted
  (re-validation against current `accumulating`).
- **N-E-7** — Admission outcome is closed: `AdmitOutcome::{Admitted, Rejected}`
  with `Rejected` carrying `TxValidityError`.
- **N-E-8** — `IngressEvent.source` is evidence-only.
  `admit(state, tx_bytes)` is a function of `(state, tx_bytes)` alone,
  regardless of source variant. Equivalent verdicts across N2N and N2C
  for the same `(state, tx)`.

## 2. What must never be possible

- **N-E-N1** — Ade-accepts / oracle-rejects on the same `(accumulating, tx)`
  (false-accept; bounty-killing).
- **N-E-N2** — Mutating `MempoolState.accumulating` without going through
  `admit` (the chokepoint property).
- **N-E-N3** — Admitting a tx whose `PreservedCbor` round-trip fails.
- **N-E-N4** — Wall-clock, randomness, `HashMap`/`HashSet` ordering, or floats
  in the BLUE bridge or admit.
- **N-E-N5** — Concurrent admit ordering that diverges from single-threaded
  replay of the canonical event stream.
- **N-E-N6** — Rejected tx mutating state. Already enforced by admit's
  `mempool.clone()`; the new bridge must preserve this.
- **N-E-N7** — Same `(state, tx)` producing different verdicts depending on
  `IngressEvent.source` variant.

## 3. What must remain identical across executions

- The function `(base_ledger_state, [IngressEvent]) → [AdmitOutcome]` is
  total, pure, deterministic.
- The accumulating-state prefix at any point is a pure function of the same.
- Closed discriminants frozen per cluster cut:
  - `IngressEvent`, `IngressSource` (new)
  - `AdmitOutcome` (exists)
  - `TxRejectClass`, `TxValidityError` (exist)
- Wire format for the tx-submission2 frames Ade negotiates: byte-identical
  to cardano-node for negotiated protocol versions.

## 4. What must be replay-equivalent

- **Ingress trace** = ordered `[IngressEvent { source: N2N|N2C, payload: PreservedCbor<Tx> }]`.
  Replaying against the same `base` produces byte-identical
  `(MempoolState, [AdmitOutcome])`.
- **Adversarial corpus reuse.** The existing B1–B5 + B3F adversarial corpus,
  wrapped in synthetic `IngressEvent { source: N2N, tx_bytes }`, replays through
  `mempool_ingress` and reproduces the exact `TxValidityError` rejection class.
  Corpus bytes are NOT altered; only the canonical envelope is added.
- **Multi-peer canonicalization.** Two distinct interleavings of the same
  per-peer tx streams yield the same admitted set iff they canonicalize to the
  same `IngressEvent` sequence. The GREEN canonicalizer is itself deterministic
  and replay-equivalent.

## 5. State transitions in scope

| Layer | Transition | Status |
|---|---|---|
| BLUE | `tx_validity(state, tx_cbor) → TxValidityVerdict` | exists, closed (B1–B5) |
| BLUE | `admit(mempool, tx_cbor) → (MempoolState, AdmitOutcome)` | exists, closed (CE-B2-5) |
| BLUE | `tx_submission2_transition(state, msg) → (next, [InventoryEvent])` | exists, closed (S-A6) |
| **BLUE — new** | **`mempool_ingress(mempool, IngressEvent) → (MempoolState, AdmitOutcome)`** | **load-bearing; single-step per OQ-6** |
| BLUE — new | `IngressEvent { source: IngressSource, tx_bytes: PreservedCbor<Tx> }`; `IngressSource::{N2N, N2C}` | closed enum |
| GREEN — new | `canonicalize_peer_streams([(PeerId, Stream<TxBytes>)]) → Stream<IngressEvent>` | deterministic ordering; may batch |
| GREEN — new | `ade_testkit::mempool::ingress_replay` harness | mirrors B-track replay shape |
| RED — new | N2N tx-submission2 session loop | sockets, peer churn, retries |
| RED — new | N2C local-tx-submission UDS session loop | cardano-cli ingress |

## 6. TCB color hypothesis

- **BLUE:** `mempool::admit`, `mempool::ingress` (new), `IngressEvent` /
  `IngressSource` (new), `tx_submission2_transition`. Dependencies flow
  inward only.
- **GREEN:** Cardano-canonical CBOR codec wrappers for tx-submission2; the
  per-peer canonicalizer; the `ingress_replay` testkit harness.
- **RED:** Both ingress session loops (N2N TCP, N2C UDS). Wall-clock /
  socket / retry semantics live only here.
- **Open color:** mempool bound / shedding policy. If a bound is needed and
  it does not change admission verdicts, it can sit GREEN/RED as
  back-pressure. If a bound *would* change verdicts (which is itself a
  consensus problem), it must be BLUE — but that's a different cluster.

## 7. Registry moves (DEFERRED to first-slice landing)

The append is deliberately deferred so each new registry entry's
`code_locus` is real, not aspirational. The first slice that creates
`mempool::ingress.rs` should land the registry append in the same commit.

### Strengthen on close
- **`DC-MEM-01`** (status already `enforced`):
  - `strengthened_in += "PHASE4-N-E"`
  - `code_locus += "; crates/ade_ledger/src/mempool/ingress.rs"`
  - `tests += [ingress_replay test names — TBD per slice]`

### Hold
- **`DC-MEM-02`** — unchanged unless `policy.rs` gains a
  verdict-affecting bound.
- **`CN-MEM-01`** / **`CN-MEM-03`** / **`CN-MEM-04`** — leave as
  declared constitutional rules; enforcement lives in `DC-MEM-01/02`
  and the new `DC-MEM-03/04`.

### Append on first-slice landing

```toml
[[rules]]
id = "DC-MEM-03"
tier = "derived"
statement = "Tx ingress reduces to a closed IngressEvent before BLUE mempool admission; the source variant is evidence/policy/replay metadata only and MUST NOT change the validity verdict."
source = "Project constitution §3, T-CORE-01 (closed semantic surfaces), DC-MEM-01"
cross_ref = ["DC-MEM-01", "CN-MEM-04", "T-CORE-01"]
code_locus = "crates/ade_ledger/src/mempool/ingress.rs (IngressEvent, IngressSource, mempool_ingress)"
tests = []           # TBD per slice
ci_script = ""       # candidate ci/ci_check_mempool_ingress_closure.sh
status = "declared"
introduced_in = "PHASE4-N-E"
strengthened_in = []

[[rules]]
id = "DC-MEM-04"
tier = "derived"
statement = "Replaying the same ordered ingress trace against the same base ledger state produces a byte-identical sequence of (MempoolState, AdmitOutcome) pairs."
source = "Project constitution §3, T-DET-01, DC-MEM-01"
cross_ref = ["DC-MEM-01", "T-DET-01"]
code_locus = "crates/ade_ledger/src/mempool/ingress.rs; crates/ade_testkit/src/mempool/ingress_replay.rs"
tests = []           # TBD per slice
ci_script = ""
status = "declared"
introduced_in = "PHASE4-N-E"
strengthened_in = []
```

## 8. Open items remaining (to resolve in `/cluster-plan`)

- **Cluster ID.** `PHASE4-N-E` per the existing plan, or
  `PHASE4-N-E-T1` to make the Tier 1 / Tier 5 split explicit in the
  cluster directory. Default: `PHASE4-N-E` (other split clusters used
  a single name).
- **CI script for DC-MEM-03.** A candidate
  `ci/ci_check_mempool_ingress_closure.sh` (forbid
  `MempoolState.accumulating` mutation outside `admit` /
  `mempool_ingress`) is the natural shape. Decide whether to ship it
  with the first slice or as a closure slice.
- **CI script for DC-MEM-04.** Likely discharged by the replay harness
  alone, no CI script needed.

## Related

- Existing closed clusters this builds on: PHASE4-B1..B5, PHASE4-B3F
  (tx_validity authority); PHASE4-N-A (Ouroboros mini-protocols incl.
  `tx_submission2`); PHASE4-N-B (consensus runtime); PHASE4-N-D
  (chain DB).
- `docs/active/phase_4_cluster_plan.md` §N-E (original plan stub).
- SEAMS open candidate: "N2N/N2C tx-submission → mempool::admit RED
  bridge (B+ / N-E) — most load-bearing for the bounty."
