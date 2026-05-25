# Cluster/Slice Plan — Ade — PHASE4-N-E (Tier 1 half)

> **Status:** Planning artifact (non-normative). Produced via `/cluster-plan`
> on 2026-05-25 against HEAD `52642e5`. Built on the invariants sketch
> at `docs/planning/phase4-n-e-tier1-invariants.md`. Authority lives in
> `docs/ade-invariant-registry.toml`; if this plan conflicts with the
> registry the registry wins.

## Cluster Index (Dependency Order)

1. **PHASE4-N-E** (Tier 1 half) — *no-false-accept on wire-level tx ingress*
   — single cluster; depends on PHASE4-N-A (tx-submission2 + local-tx-submission
   state machines + codecs, both closed) and PHASE4-B1..B5+B3F (tx_validity
   authority + admit chokepoint, all closed).

Outbound propagation and bounds/shedding policy are **out of scope** per
OQ-2 and OQ-5 calls in the invariants sketch.

---

## Cluster PHASE4-N-E — Wire-level mempool ingress (Tier 1)

### Primary invariant

Mempool acceptance equals ledger/block validity for every tx, regardless of
ingress source — `admit(state, tx)` is a function of `(state, tx_bytes)`
alone, reached through a single BLUE chokepoint
`mempool_ingress(IngressEvent) → (MempoolState, AdmitOutcome)`. The two RED
ingress sources (N2N tx-submission2, N2C local-tx-submission) funnel through
a deterministic GREEN canonicalizer into that one BLUE bridge.

### TCB partition

| Color | Modules |
|---|---|
| BLUE | `ade_ledger::mempool::ingress` (new: `IngressEvent`, `IngressSource`, `mempool_ingress`); `ade_ledger::mempool::admit` (existing, untouched). |
| GREEN | `ade_ledger::mempool::canonicalize` (new: deterministic per-peer ordering); `ade_testkit::mempool::ingress_replay` (new: replay harness). |
| RED | `ade_runtime::tx_submission::n2n_session` (new); `ade_runtime::tx_submission::n2c_session` (new). |

### Cluster Exit Criteria

- **CE-N-E-1** — `IngressEvent`/`IngressSource` is the closed sum-type entry;
  `mempool_ingress` is the only sanctioned path into `admit` from non-test
  code; CI gate forbids `MempoolState.accumulating` mutation outside `admit`
  and forbids non-test callers of `admit` outside `mempool_ingress`.
  (DC-MEM-03.)
- **CE-N-E-2** — Mempool acceptance via `mempool_ingress` ≡ direct `admit`
  ≡ `tx_validity` on the existing B-track adversarial corpus (byte-identical
  verdicts and accumulating state). (DC-MEM-01 strengthening.)
- **CE-N-E-3** — Source-invariance: same `(state, tx_bytes)` under
  `IngressSource::N2N` vs `IngressSource::N2C` produces byte-identical
  `(MempoolState, AdmitOutcome)`. Property test + per-corpus replay.
  (Invariant N-E-8.)
- **CE-N-E-4** — Ingress-trace replay: replaying the same ordered
  `[IngressEvent]` against the same `base` produces byte-identical
  `(MempoolState, [AdmitOutcome])`. Includes a multi-peer interleaving case
  proving the GREEN canonicalizer's determinism. (DC-MEM-04.)
- **CE-N-E-5** — Adversarial-reuse no-false-accept: the full B1–B5 + B3F
  adversarial corpus, wrapped in synthetic `IngressEvent`s without mutating
  tx bytes, reproduces the exact `TxValidityError` rejection class for every
  reject case. (Strengthens DC-MEM-01.)
- **CE-N-E-6** — Live N2N evidence: real `cardano-node` peer submits txs to
  Ade over real N2N tx-submission2; Ade's verdicts on the received txs are
  byte-identical to direct-replay verdicts for the same tx bytes. Closure
  via operator-captured evidence log committed under
  `docs/clusters/PHASE4-N-E/` (CE-N-B-6 pattern).
- **CE-N-E-7** — Live N2C evidence: real `cardano-cli transaction submit` to
  Ade over real N2C local-tx-submission UDS; Ade's verdict matches N2N
  submission of the same tx bytes. Closure via operator evidence log.

### Slices

| ID | Name | Invariant | Addresses | TCB |
|---|---|---|---|---|
| **N-E-S1** | IngressEvent + mempool_ingress + closure CI | Tx ingress reduces to a closed `IngressEvent` before BLUE admission; the source variant is metadata only. | CE-N-E-1, CE-N-E-3 (type-level half) | BLUE + CI |
| **N-E-S2** | Adversarial corpus reuse + ingress-replay harness | Replaying the B-track adversarial corpus through `mempool_ingress` reproduces byte-identical results to direct `admit`. | CE-N-E-2, CE-N-E-3 (corpus half), CE-N-E-5 | GREEN harness over BLUE |
| **N-E-S3** | Per-peer canonicalizer | Two distinct interleavings of the same per-peer tx streams produce the same `IngressEvent` sequence. | CE-N-E-4 (multi-peer half) | GREEN |
| **N-E-S4** | N2N InventoryEvent → mempool_ingress GREEN bridge | InventoryEvent::TxsDelivered streams reduce to deterministic IngressEvent streams + replay. | CE-N-E-6 (adapter mechanical half) | GREEN |
| **N-E-S5** | N2C LocalTxSubmissionEvent → mempool_ingress GREEN bridge + cross-bridge agreement | Same tx bytes under N2N vs N2C bridges produce byte-identical outcomes. | CE-N-E-7 adapter mechanical half (CE itself **deferred**, see below) | GREEN |
| **N-E-S6** | Live N2N tx-submission2 binary | BLUE state machine drives correctly over real wire + handshake + mux + codec; protocol grammar respected for sustained window. | CE-N-E-6 (live-wire half) | RED + `#[ignore]` test |

**Deferred from N-E:** CE-N-E-7's live N2C UDS server + operator pass.
Moved to the future node-binary cluster as cross-cluster obligation
`CE-NODE-N2C-LTX`. No temporary UDS scaffolding is built in this
cluster. Procedure spec retained at `docs/clusters/PHASE4-N-E/CE-N-E-7_PROCEDURE.md`.

### Replay obligations

- New canonical types — `IngressEvent`, `IngressSource` — must be in the
  project's closed-enum CI grep (`ci_check_consensus_closed_enums.sh` or
  the new `ci_check_mempool_ingress_closure.sh`).
- New BLUE state transition — `mempool_ingress` — is a pure function of
  `(state, IngressEvent)`; replay equivalence is mechanical in S2.
- No new adversarial-corpus tx bytes — the cluster *reuses* B1–B5 + B3F
  bytes verbatim, wrapped in `IngressEvent` envelopes. The corpus-load layer
  in `ade_testkit::mempool::ingress_replay` is the only new GREEN corpus
  surface.
- Live evidence (S4, S5) — committed log files; not replay-equivalent (RED),
  but reproducible via the documented operator procedure.

### Independent-mergeability check (per IDD discipline)

- N-E-S1 strictly *adds* a typed entry point + a CI gate; existing `admit`
  callers (currently only tests and the future bridge) remain valid. No
  invariant weakened.
- N-E-S2 adds a harness + corpus reuse + DC-MEM-04 entry; no production
  code changes.
- N-E-S3 adds a GREEN canonicalizer; no caller required yet (S4/S5 use it).
- N-E-S4 adds a RED driver that calls `mempool_ingress`; the BLUE chokepoint
  already exists.
- N-E-S5 same shape as S4 over a different transport.

Each slice independently leaves the system in a fully correct state.

### Registry moves at cluster-close (per slice)

| Slice | Registry change |
|---|---|
| N-E-S1 | Append `DC-MEM-03` with real `code_locus` and `ci_script`. |
| N-E-S2 | Append `DC-MEM-04` with real `code_locus`. Pin `DC-MEM-01.strengthened_in += "PHASE4-N-E"`, extend `DC-MEM-01.code_locus` and `DC-MEM-01.tests`. |
| N-E-S3 | Extend `DC-MEM-04.tests` with multi-peer determinism test name. |
| N-E-S4 | No registry change (RED evidence); commit live evidence log under `docs/clusters/PHASE4-N-E/`. |
| N-E-S5 | No registry change (RED evidence); commit live evidence log under `docs/clusters/PHASE4-N-E/`. |

CN-MEM-01, CN-MEM-03, CN-MEM-04, DC-MEM-02 unchanged.

### Sequencing rationale

S1 (BLUE bridge) before S2 (harness) — harness depends on the bridge.
S2 before S3 (canonicalizer) — replay invariant is mechanical without
multi-peer; S3 extends it. S3 before S4/S5 — RED drivers both consume the
canonicalizer. S4 before S5 — N2N is the load-bearing seam; N2C is a thin
variant.

## Stop conditions

- The cluster does NOT fully close in this session if CE-N-E-6 or CE-N-E-7
  evidence logs require operator action against a live cardano-node /
  cardano-cli. In that case the cluster lands at "code + harness complete,
  live evidence pending" and the operator captures the logs separately.
  This mirrors the CE-N-B-6 close pattern.
