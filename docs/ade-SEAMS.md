# Seams — Where New Work Can Attach (Ade)

> **Status:** Living architectural document. Regenerated; not hand-edited.
> Per-project instance of `~/.claude/methodology/templates/seams.md`.

> 11 crates, 31 CI checks at HEAD (`caa5ce8`).
> Reads CODEMAP for the module list and TCB colors; reads the invariant
> registry (`docs/ade-invariant-registry.toml`) for rule IDs; reads the
> Phase 4 cluster plan (`docs/active/phase_4_cluster_plan.md`), the
> closed N-D / N-A / N-B / B1 / B2 / B3 / B4 / B5 cluster docs, the
> OQ5-CREDENTIAL-FIDELITY, COMMITTEE-CRED-FIDELITY, DREP-VOTE-FIDELITY,
> ENACTMENT-COMMITTEE-FIDELITY, ENACTMENT-COMMITTEE-WRITEBACK cluster
> docs, and the **just-closed and archived PHASE4-N-E cluster doc**
> (`docs/clusters/completed/PHASE4-N-E/cluster.md` + slices S1..S6 +
> the two CE-N-E procedure docs + the CE-N-E-6 live evidence log).
>
> **This is the PHASE4-N-E FULL CLOSE refresh (HEAD `caa5ce8`).** The
> previous SEAMS (HEAD `350130e`) pinned the partial-close state — N-E
> S1..S5 landed; S6 + the live N2N evidence + the cluster archival
> were pending. **Two deltas land between that revision and this one**:
>
> 1. **N-E-S6 (commit `d1068b3`)** ships the operator-action probe
>    binary `live_tx_submission_session` in
>    `crates/ade_core_interop/src/bin/live_tx_submission_session.rs`.
>    Modeled on `live_consensus_session` from PHASE4-N-B: hermetic
>    default codec-loopback mode plus a `--connect` live pass that
>    drives a real cardano-node peer as the outbound tx-submission2
>    client. RED (operator-action / live-I/O), `#[ignore]`-gated by
>    closure-gate test, no new BLUE / GREEN library code; the
>    `--connect` pass is the executable mirror of the
>    `CE-N-E-6_PROCEDURE.md` script-stage. The follow-up commit
>    `caa5ce8` adds retry-on-timeout + elapsed-time logging.
> 2. **CE-N-E-6 live evidence captured** at
>    `docs/clusters/completed/PHASE4-N-E/CE-N-E-6_2026-05-25.log`. A
>    real preprod relay handshakes at v15 over N2N, the tx-submission2
>    state machine sends `Init` → `Idle`, the peer replies with
>    `RequestTxIds`, and the bridge accumulator records
>    `frames_received=1 requests_ids=1 tx_bytes=0` before the peer
>    drops the connection (relays do not push txs at outbound clients
>    in this direction — peer-initiated tx delivery is documented in
>    the procedure header as the deferral that joins CE-NODE-N2C-LTX
>    in the future node-binary cluster). The agreement assertion
>    (bridge ≡ direct `tx_validity` for `0/0` txs) is **vacuously
>    true** at this evidence depth; the BLUE / GREEN path's
>    correctness is what the GREEN integration tests + the closure-gate
>    test prove, and the live log is the on-the-wire half of an existing
>    mechanical surface. **PHASE4-N-E is now mechanically and
>    operationally CLOSED**; the cluster directory has been moved from
>    `docs/clusters/PHASE4-N-E/` to
>    `docs/clusters/completed/PHASE4-N-E/` (every path in this SEAMS
>    pointing at the cluster directory uses the archived location).
>
> **THE KEY FULL-CLOSE DELTA — cross-cluster obligation pattern
> introduced.** PHASE4-N-E deliberately splits CE-N-E-7 in two halves
> and only claims half of it as closed:
>
> - The **adapter-mechanical half** is closed in S5 via the
>   integration test
>   `n2n_and_n2c_bridges_produce_identical_outcomes`
>   (`crates/ade_core_interop/tests/local_tx_submission_ingress.rs`).
>   This test wires the same `tx_bytes` through both bridges
>   (`ingest_n2n_events` and `ingest_n2c_events`) against the same
>   base ledger state and asserts the resulting
>   `(MempoolState, Vec<AdmitOutcome>)` are byte-identical. It is
>   counted as closed under the N-E cluster.
> - The **live-N2C-UDS-server half** is **DEFERRED** to a future
>   node-binary cluster under the new identifier `CE-NODE-N2C-LTX`. The
>   PHASE4-N-E cluster doc does **NOT** claim CE-N-E-7 as closed; the
>   `CE-N-E-7_PROCEDURE.md` document is retained in the archived
>   cluster directory as the procedural anchor that the future cluster
>   will execute. The future cluster's evidence log lives at
>   `CE-NODE-N2C-LTX_<date>.log`.
>
> **This is the first use of the cross-cluster-obligation pattern in
> this project.** It is recorded as a new closure pattern in §5 below
> alongside the existing operator-action evidence pattern: a cluster
> may legitimately close on a partial CE provided that (a) the
> mechanical half is fully landed in this cluster, (b) the deferred
> half names its successor cluster explicitly, (c) the procedure doc
> is retained at a canonical path so the successor cluster has zero
> ambiguity about what evidence is required, and (d) the cluster doc
> records the split unambiguously.
>
> Counts in this refresh are stable relative to the partial-close
> pin — N-E-S6 added no new closed enum, no new BLUE chokepoint, no
> new frozen contract, no new version-gated contract, and no new CI
> script (the S6 binary is RED operator-action with no library code).
> What did change at the SEAMS level: the §1 candidates row for N2N
> tx-submission2 now reads **fully closed** (not "wired & closed,
> live log pending"), the operator-action evidence pattern table now
> carries the CE-N-E-6 live log as a captured artifact, the
> operator-action surface in `ade_core_interop` lists the new probe
> binary, and §5 records the cross-cluster obligation pattern.
>
> **N-E summary (carried from partial-close pin, unchanged on
> structure).** The prior revision's single most load-bearing §1
> candidate seam — "N2N/N2C tx-submission ingest → `mempool::admit`" —
> remains **WIRED AND CLOSED on the code half**. The closed 2-variant
> `IngressSource { N2N, N2C }`, the closed `IngressEvent
> { source, tx_bytes }` struct, and the single BLUE chokepoint
> `mempool_ingress(&MempoolState, &IngressEvent) -> (MempoolState,
> AdmitOutcome)` are the production path; `mempool_ingress` is a
> thin pass-through to `admit` over `event.tx_bytes()`. The two RED
> ingress transports feed through deterministic GREEN bridges
> (`ade_core_interop::tx_submission` and
> `ade_core_interop::local_tx_submission`) into the GREEN per-peer
> canonicalizer (`canonicalize_peer_streams`) and then into
> `mempool_ingress`. The closure is mechanically defended by the two
> N-E CI gates `ci_check_mempool_ingress_closure.sh` (DC-MEM-03) and
> `ci_check_mempool_ingress_replay.sh` (DC-MEM-04). The verdict
> equation is unchanged (`mempool_ingress(s, evt) == admit(s,
> evt.tx_bytes())`); source-invariance (N-E-N7 / N-E-8) is the
> first-class property.
>
> **Per-peer canonicalizer is the load-bearing GREEN fairness
> contract** (round-robin by sorted `PeerId`, with single-byte source
> tie-break N2N=0 / N2C=1 — stable across binary builds). Any future
> change to this ordering — fairness policy, batching, parallelization,
> or alternative tie-breaks — is a SEAMS-level change because the
> GREEN replay-byte-identity property (DC-MEM-04) and the multi-peer
> interleaving tests pivot on this exact rule.

Ade is a Cardano block-producing node. Its closure surface is dominated
by two facts:

1. The Cardano protocol fixes wire bytes and hashes for hash-critical
   paths (Tier 1 — must-conform). New work that touches those bytes
   has essentially no degrees of freedom.
2. Everything operator-facing — storage layout, query API, telemetry,
   packaging — is Tier 5: deliberate divergence "in our own image"
   (per `docs/active/CE-79_tier5_addendum.md`).

This document names where the system opens and where it stays closed.

**PHASE4-N-E (Tier 1 wire-level mempool ingress) is fully closed.** The
prior revision (HEAD `350130e`) pinned the partial-close state with
S1..S5 mechanical evidence and the live N2N/N2C logs as pending
operator-action artifacts. At this HEAD: S6 ships the
`live_tx_submission_session` operator probe binary; the CE-N-E-6 live
N2N evidence log is captured (vacuously-true agreement on `0/0` txs at
the relay-as-server boundary, with the bulk-tx half deferred to
CE-NODE-N2C-LTX); CE-N-E-7 is split into adapter-mechanical (closed in
S5) + live-N2C-UDS-server (deferred to the future node-binary cluster
as CE-NODE-N2C-LTX); the cluster is archived to
`docs/clusters/completed/PHASE4-N-E/`. N-E's closed surfaces and CI
gates carry forward unchanged.

**PHASE4-B3 (Full Conway tx value-conservation accounting) is closed.**
It closed the deposit/refund/withdrawal value-conservation follow-up B2
deliberately deferred. It added: a **closed Conway certificate CDDL
grammar** (`ade_codec::conway::cert::decode_conway_certs` over tags
`0..18`, with `CodecError::UnknownCertTag` for tags ≥19, `RemovedInConway`
for tags 5/6, and **no catch-all accept arm**); a **closed withdrawals
map grammar** (`ade_codec::conway::withdrawals` rejecting a repeated key
with `CodecError::DuplicateMapKey` — never last-wins); the **closed
`ConwayCert` / `CertDisposition` / `DepositEffect` / `CoinSource` sum
types** in `ade_types::conway::cert` plus `RewardAccount` in
`ade_types::tx`; a **canonical-only deposit-parameter surface**; the
**closed total cert classifier** `ade_ledger::cert_classify::classify`;
and the **full preservation-of-value equation** in
`ade_ledger::conway::check_conway_coin_conservation` with the **frozen
§9.1 reject precedence**.

**PHASE4-B4 (Conway certificate-state accumulation, fail-closed) is
closed.** It made the B3-introduced `ConwayCert` **owner-complete** and
added the native owner-tagged Conway apply model in
`ade_ledger::delegation`. **THE B4 SEAM:** governance-affecting Conway
certs are decoded fully and **owner-tagged to `ConwayGovState`** via
`OwnerTaggedEffect`, routed OUT of B4's mutation scope.

**PHASE4-B5 (Conway governance-certificate accumulation) is closed.** It
**applies** the owner-tagged governance effects B4 deliberately left
unapplied: `ade_ledger::gov_cert::apply_conway_gov_cert` is a **total,
compiler-exhaustive dispatch over `ConwayCert`** with no `_ =>` arm, that
folds vote-delegation / committee / DRep effects into `ConwayGovState`.

**OQ5-CREDENTIAL-FIDELITY → COMMITTEE-CRED-FIDELITY → DREP-VOTE-FIDELITY
→ ENACTMENT-COMMITTEE-FIDELITY → ENACTMENT-COMMITTEE-WRITEBACK** closed
the credential discriminant chain across the gov-state keys, the
committee/DRep votes, the enactment effects, and the live
`UpdateCommittee` write-back. **The remaining open governance-domain
seam** is the declared non-goal `proposal_procedures` tx-body decode.

---

## 1. Surface Reduction Rules

> External inputs reduce to canonical form before entering authoritative
> pipelines. At HEAD there are **seven** fully-wired *external* ingress
> surfaces (block bytes, Plutus script bytes, snapshot bytes, Ouroboros
> mux frames, genesis JSON bundles, chain-selector stream inputs, **and
> the N-E wire-level mempool ingress**), plus the three **internal
> composition roots** (`block_validity` from B1, `tx_validity` from B2,
> and the BLUE chokepoint `mempool_ingress` from N-E S1), the
> **mempool admission gate** (`mempool::admit`, a Tier-1 surface over
> `tx_validity`), the **consensus-input extraction surface** (snapshot
> `state` CBOR tail-scan from B1), plus the remaining surfaces named in
> the Phase 4 plan (forge, query API, outbound tx propagation).
>
> **At this HEAD the N-E wire-level seam is fully closed — code +
> live evidence at the depth the outbound-client probe can attest.**
> The mechanical halves close in N-E S1..S5; the operator-action live
> evidence closes via S6's probe binary and the CE-N-E-6 evidence log
> committed under the archived cluster directory. The bulk-tx half
> (peer pushes txs to Ade as an inbound listener) and the live N2C
> UDS half (cardano-cli over Unix-socket) are **deferred to the
> future node-binary cluster as CE-NODE-N2C-LTX** — neither is an
> open seam in N-E itself.

### Surface: Mempool ingress (Tier-1 wire-level — wired in N-E)

```
Surface: A candidate transaction delivered by a real cardano-node N2N
         peer (via tx-submission2) or by a real cardano-cli over the
         N2C local-tx-submission UDS, against the mempool's
         accumulating LedgerState
Reduces to: (MempoolState, AdmitOutcome)
            { Admitted { tx_id } | Rejected { class, error } }
            (AdmitOutcome from `ade_ledger::mempool::admit`)
Pipeline (fixed step ordering — no reorder, no shortcut):
  1. RED transport
       ade_network::mux::transport (tokio TCP / UDS bearer; bytes in)
       — moves bytes off the bearer; no parsing.
  2. BLUE wire grammar (N-A)
       N2N: ade_network::codec::tx_submission::decode_tx_submission2_message
            + ade_network::tx_submission::transition::tx_submission2_transition
            emits InventoryEvent { ServerOpened, IdsRequested, IdsDelivered,
            TxsRequested, TxsDelivered { tx_bytes: Vec<Vec<u8>> } }
       N2C: ade_network::n2c::local_tx_submission::codec::decode_local_tx_submission_message
            + ade_network::n2c::local_tx_submission::transition::local_tx_submission_transition
            emits LocalTxSubmissionEvent { TxSubmitted { tx_bytes },
            TxAccepted, TxRejected { .. } }
       — opaque tx bytes; the state machines do not interpret tx CBOR.
  3. GREEN bridge (N-E S4 / S5; deterministic, no I/O)
       N2N: ade_core_interop::tx_submission::ingest_n2n_events(
             base: LedgerState,
             per_peer: &[(PeerId, Vec<InventoryEvent>)]
           ) -> (MempoolState, Vec<AdmitOutcome>)
       N2C: ade_core_interop::local_tx_submission::ingest_n2c_events(
             base, per_client: &[(PeerId, Vec<LocalTxSubmissionEvent>)]
           ) -> (MempoolState, Vec<AdmitOutcome>)
  4. GREEN canonicalizer (N-E S3; deterministic, no I/O)
       ade_ledger::mempool::canonicalize::canonicalize_peer_streams(
         &[PeerSubmissionQueue]
       ) -> Vec<IngressEvent>
         — round-robin by sorted `PeerId` byte-lex; tie-break by single-byte
           source tag (N2N=0, N2C=1).
  5. BLUE chokepoint (N-E S1)
       ade_ledger::mempool::ingress::mempool_ingress(
         &MempoolState, &IngressEvent
       ) -> (MempoolState, AdmitOutcome)
         — pure pass-through to `admit` over `event.tx_bytes()`. The
           `event.source` is RECORDED for evidence/policy/replay but
           MUST NOT affect the verdict (N-E-N7/N-E-8).
  6. BLUE admission gate (B2 — unchanged)
       admit(mempool, tx_cbor) -> (MempoolState, AdmitOutcome)
         — verdict equals `tx_validity(accumulating, tx)`; no false
           accept (DC-MEM-01).
Cross-surface state sharing: the mempool's `accumulating` LedgerState
  is the only state carried across consecutive `mempool_ingress` calls.
```

**Rule.** Mempool ingress is the **Tier-1 wire-level seam**:
every production tx-bytes path into `admit` MUST go through
`mempool_ingress`. The single BLUE chokepoint, the closed
2-variant `IngressSource`, and the verbatim flow of `tx_bytes` (no
decode, no re-encode) are the load-bearing properties (DC-MEM-03).
The `IngressSource` variant is **metadata only**: source-invariance
(N-E-N7 / N-E-8) is CI-enforced. New ingress transports attach by
**producing `IngressEvent`s and feeding them into `mempool_ingress`**.
The replay contract (DC-MEM-04) is a **single-step fold** over
`mempool_ingress`. **Operator action**: the live N2N tx-submission2
wire evidence is captured into
`docs/clusters/completed/PHASE4-N-E/CE-N-E-6_2026-05-25.log`
(committed at this HEAD; outbound-client probe; the bulk-tx half
joins CE-NODE-N2C-LTX in the future node-binary cluster). The live
N2C UDS evidence (CE-N-E-7) is **DEFERRED** to the same future
cluster (CE-NODE-N2C-LTX) — see "Cross-cluster obligation pattern"
in §5.

### Surface: Single-tx validity (composition root — wired in B2)

```
Surface: A single Conway transaction (full tx CBOR
         [body, witness_set, is_valid, aux_data]) decided against a
         LedgerState (its track_utxo flag selects partial vs. full)
Reduces to: TxValidityVerdict { Valid { tx_id, applied } |
                                Invalid { class, error } }
Pipeline (fixed step ordering — no reorder, no shortcut):
  1. ade_ledger::tx_validity::phase1::decode_tx(tx_cbor) -> DecodedTx
  2. ade_ledger::tx_validity::phase1::tx_phase_one(ledger, &decoded)
  3. phase-2 (Plutus) via plutus_eval::try_evaluate_tx
  4. Valid -> evolve via rules::apply_conway_tx_to_utxo;
     Invalid -> input state returned UNCHANGED
Cross-surface state sharing: none — pure function.
```

**Rule.** `tx_validity` is the **single per-tx composition root** —
DC-TXV-02. **N-E note:** the production entry from wire transports
into `tx_validity` is now `mempool_ingress` → `admit` → `tx_validity`;
the composer itself is untouched.

### Surface: Mempool admission (Tier-1 gate — wired in B2; N-E added the BLUE bridge above it)

```
Surface: A candidate transaction offered to the mempool, against the
         mempool's accumulating LedgerState
Reduces to: AdmitOutcome + new MempoolState
Pipeline:
  1. ade_ledger::mempool::admit(mempool, tx_cbor) -> (MempoolState, AdmitOutcome)
     - calls tx_validity(&mempool.accumulating, tx_cbor). NO FALSE ACCEPT (DC-MEM-01).
  2. ade_ledger::mempool::policy::order(mempool, OrderPolicy)
     (Tier-5, GREEN — deterministic PERMUTATION over admitted ids — DC-MEM-02)
```

**Rule.** Admission is a **thin Tier-1 gate over `tx_validity`**. The
Tier-1 / Tier-5 split is the key seam. **N-E added a second seam
ABOVE `admit`** — `mempool_ingress` is now the only sanctioned
production caller of `admit` outside its own `#[cfg(test)]` block.
`admit` itself is unchanged.

### Surface: Full block validity (composition root — wired in B1)

```
Surface: A full block (era-tagged envelope CBOR) decided against
         (LedgerState, PraosChainDepState, EraSchedule, LedgerView)
Reduces to: BlockValidityVerdict
Pipeline (fixed step ordering — no reorder, no shortcut):
  1. ade_ledger::block_validity::decode_block(block_cbor) -> DecodedBlock
  2. ade_core::consensus::validate_and_apply_header (BLUE, FAIL-FAST)
  3. body-hash binding (CN-CONS-04)
  4. ade_ledger::rules::apply_block_with_verdicts (BLUE)
  5. Valid -> evolved (LedgerState', PraosChainDepState'); Invalid -> unchanged
```

**Rule.** `block_validity` is the **single block-level composition root**.
**Remaining adjacent gap:** the Conway block-body vkey-witness closure
(`project_conway_body_witness_gap`).

### Surface: Block bytes (wired today)

```
Surface: Block bytes (file/stream/network — caller-supplied)
Reduces to: BlockEnvelope { era: CardanoEra, era_block: PreservedCbor<EraBlock> }
Pipeline:
  1. decode_block_envelope(&[u8]) -> BlockEnvelope
  2. era-specific decode_<era>_block (closed set — 8 era-block decoders)
  3. ade_ledger::rules::apply_block_with_verdicts(...) (BLUE)
```

**Rule.** `ade_network` is forbidden from decoding block CBOR.

### Surface: Plutus script bytes (wired today)

```
Surface: Plutus script bytes (CBOR-wrapped Flat)
Reduces to: PlutusScript { inner: aiken_uplc::ast::Program<DeBruijn> }
Pipeline:
  1. ade_plutus::evaluator::PlutusScript::from_cbor (named ingress chokepoint)
  2. ade_plutus::tx_eval::eval_tx_phase_two(...) -> TxEvalResult (BLUE)
```

**Rule.** Distinct ingress surface from block CBOR. Allowlisted file-path
exception in `ci_check_ingress_chokepoints.sh` Check 3.

### Surface: Snapshot bytes (wired in N-D)

```
Surface: Snapshot bytes (disk — written and read by the node itself)
Reduces to: Recoverable::decode_snapshot(&[u8]) -> R  (caller-supplied)
Pipeline:
  1. SnapshotStore::latest_snapshot()
  2. Recoverable::decode_snapshot(bytes)
  3. for block in ChainDb::iter_from_slot(slot+1): R::apply_block(&block.bytes)
```

**Rule.** The recovery primitive (`ade_runtime::recovery::recover`) is
the single path from on-disk state to in-memory state.

### Surface: Consensus-input extraction (snapshot `state` CBOR tail-scan — wired in B1)

```
Surface: A UTxO-HD `utxohd-mem` ExtLedgerState snapshot `state` CBOR
Reduces to: PraosNonces { evolving, candidate, epoch, lab, last_epoch_block }
Pipeline:
  1. ade_ledger::consensus_input_extract::extract_praos_nonces(&[u8])
     — fail-CLOSED on anything other than exactly five nonces.
```

**Rule.** Classified RED behavior; pure-over-bytes. Exact-five is a
closure invariant.

### Surface: Ouroboros mux frames (wired in N-A)

```
Surface: Raw bytes off a TCP / Unix-socket bearer (cardano-node peer)
Reduces to: per-protocol message enums in `ade_network::codec::*` (11 closed enums)
Pipeline:
  1. ade_network::mux::transport::MuxTransport::read_raw (RED, async)
  2. ade_network::mux::frame::decode_frame(&[u8]) (BLUE, sync, pure)
  3. ade_network::codec::<protocol>::decode_<protocol>_message(payload) (BLUE)
  4. ade_network::<protocol>::transition::<protocol>_transition(...) (BLUE, sync, pure)
  5. Session composition / ade_core_interop bridge (RED)
```

**Rule.** The two chokepoints `mux::frame::{encode_frame, decode_frame}`
never move. **N-E note:** the `tx-submission2` (N2N) and
`local-tx-submission` (N2C) protocols' delivered tx bytes are WIRED
into `mempool::admit` via the `mempool_ingress` chokepoint (the prior
revision's candidate seam closed in N-E). The bridge is GREEN
(`ade_core_interop::tx_submission` + `ade_core_interop::local_tx_submission`)
+ the operator-action live session (RED bearer + RED transport — the
N-E S6 probe binary `live_tx_submission_session` is the executable
instance of the N2N half).

### Surface: Genesis JSON bundles (wired in N-B)

```
Surface: Four genesis JSON blobs (byron + shelley + alonzo + conway)
Reduces to: EraSchedule { anchor, system_start_unix_ms, eras: [EraSummary; ≤7] }
Pipeline:
  1. GenesisBundle assembly (RED)
  2. compute_anchor_hash (RED, pure)
  3. parse_genesis (RED — serde_json)
  4. BLUE consensus consumes EraSchedule by-reference
```

**Rule.** The v1 preimage is FROZEN. Any future schema change is hard
version-gated.

### Surface: Chain-selector stream inputs (wired in N-B)

```
Surface: Ordered stream of N-A events (header arrival, rollback request, epoch boundary)
Reduces to: StreamInput (closed 3-variant enum)
Pipeline:
  1. caller wraps each external event in StreamInput
  2. process_stream_input(...) (GREEN, sync, pure)
  3. BLUE returns ChainEvent or ChainSelectionReject
```

**Rule.** Every external trigger that can advance Ade's chain state must
reduce to one of these three variants.

### Candidates — surfaces not yet wired (Phase 4 N-C, N-F, B+ residuals)

The following surfaces are named in the Phase 4 plan / B+ planning
but have no source today. They are listed so future slice docs can
attach without reinventing the reduction step. **Each is a candidate
seam pending confirmation at cluster entry.**

- **B3 closed the prior revision's "deposit/refund preservation-of-value"
  candidate** — removed.
- **B5 WIRED AND CLOSED the prior revision's B4 confirmed extension
  point** — the owner-tagged `ConwayGovState` effect channel.
- **OQ5 / COMMITTEE / DREP / ENACTMENT-COMMITTEE-FIDELITY /
  ENACTMENT-COMMITTEE-WRITEBACK** closed the credential discriminant
  chain and the live `UpdateCommittee` write-back.
- **N-E WIRED AND CLOSED the prior revision's most load-bearing §1
  candidate** — N2N/N2C tx-submission ingest into `mempool::admit`.
  The N2N row below is recorded as **fully closed** (code + live
  outbound-client evidence at the depth a relay-as-server peer
  attests); the N2C bulk-tx half (live UDS server) and the N2N
  bulk-tx half (peer pushes txs to Ade as inbound listener) join
  `CE-NODE-N2C-LTX` in the future node-binary cluster.

| Cluster | Surface | Expected reduction target | Expected chokepoint | Confidence |
|---------|---------|---------------------------|---------------------|------------|
| **PHASE4-N-E** *(FULLY CLOSED at this HEAD — code + live evidence at outbound-client depth; bulk-tx live half joins CE-NODE-N2C-LTX)* | **N2N/N2C tx-submission → mempool ingress (Tier-1 wire-level)** — the RED ingress that delivers a candidate tx from the `tx-submission2` (N2N) or `local-tx-submission` (N2C) opaque-bytes payload into the Tier-1 gate | `mempool_ingress(&MempoolState, &IngressEvent) -> (MempoolState, AdmitOutcome)`, where `IngressEvent { source: IngressSource::{N2N,N2C}, tx_bytes }` flows verbatim into `admit` | **DONE:** GREEN bridges `ade_core_interop::tx_submission::ingest_n2n_events` (S4) and `ade_core_interop::local_tx_submission::ingest_n2c_events` (S5) + GREEN canonicalizer `ade_ledger::mempool::canonicalize::canonicalize_peer_streams` (S3) + the BLUE chokepoint `ade_ledger::mempool::ingress::mempool_ingress` (S1). Operator probe `live_tx_submission_session` (S6) drives a real preprod N2N relay. Gated by `ci_check_mempool_ingress_closure.sh` (DC-MEM-03) + `ci_check_mempool_ingress_replay.sh` (DC-MEM-04); `DC-MEM-01.strengthened_in += PHASE4-N-E`. **Live N2N evidence captured** at `docs/clusters/completed/PHASE4-N-E/CE-N-E-6_2026-05-25.log` (outbound-client depth; bulk-tx half deferred to CE-NODE-N2C-LTX). **CE-N-E-7 split — adapter-mechanical half closed in S5 (`n2n_and_n2c_bridges_produce_identical_outcomes`); live-N2C-UDS-server half deferred to CE-NODE-N2C-LTX.** | **fully closed in N-E** (mechanical + live evidence at outbound-client depth; cross-cluster obligation CE-NODE-N2C-LTX carries the deferred bulk-tx + N2C-UDS halves) |
| **PHASE4-B5** *(WIRED + CLOSED)* | Owner-tagged Conway governance-cert effects → `ConwayGovState` | An applied `ConwayGovState'` via a deterministic fold | **DONE:** `ade_ledger::gov_cert::apply_conway_gov_cert`. Gated by `ci_check_gov_cert_accumulation_closed.sh` (DC-LEDGER-09) | **wired & closed in B5** |
| **OQ-5** *(WIRED + CLOSED in OQ5-CREDENTIAL-FIDELITY)* | Credential key/script discriminant | A discriminant-preserving credential representation | **DONE.** Gated by `ci_check_credential_discriminant_closed.sh` (DC-LEDGER-10) | **wired & closed in OQ5** |
| **Committee member / committee-vote discrimination** *(WIRED + CLOSED in COMMITTEE-CRED-FIDELITY)* | `committee` + `committee_votes` were bare `Hash28` at OQ5 | discriminant-faithful committee | **DONE.** Gated by the EXTENDED `ci_check_credential_discriminant_closed.sh` | **wired & closed in COMMITTEE-CRED-FIDELITY** |
| **DRep-vote discrimination** *(WIRED + CLOSED in DREP-VOTE-FIDELITY)* | `drep_votes` key/script OR-fallback | discriminant-faithful DRep tally | **DONE.** Gated by the EXTENDED gate | **wired & closed in DREP-VOTE-FIDELITY** |
| **`EnactmentEffects.committee_changes`** *(WIRED + CLOSED in ENACTMENT-COMMITTEE-FIDELITY)* | Bare-`Hash28` committee-change set | discriminant-faithful committee-change set | **DONE.** Gated by check 6 | **wired & closed in ENACTMENT-COMMITTEE-FIDELITY** |
| **`UpdateCommittee` / `NoConfidence` enactment LOGIC** *(WIRED + CLOSED in ENACTMENT-COMMITTEE-WRITEBACK)* | Prior `enact_proposals` was `let _ = raw;` | `ConwayGovState'` with committee + quorum updated | **DONE:** structured `GovAction::UpdateCommittee`, `apply_committee_enactment`, `rules.rs:1224`. Gated by EXTENDED checks 6 + 7 | **wired & closed in ENACTMENT-COMMITTEE-WRITEBACK** |
| OQ-3 *(separable follow-up — NOT an open seam now)* | **GOVCERT committee-membership tx-validity gate** | A `TxValidityVerdict::Invalid` on a committee cert with no matching elected member | A new BLUE tx-validity precondition check | candidate (declared separable in B5 cluster doc) |
| OQ5+ *(declared non-goal — NOT an open seam now)* | **Withdrawal / required-signer / address credential discriminant** | A discriminant-faithful credential threaded through these surfaces | extend the closed `StakeCredential` discriminant | candidate |
| OQ5+ *(declared non-goal — NOT an open seam now)* | **`Hash28`-keyed stake-distribution snapshot** | A discriminant-faithful snapshot key | re-key on `StakeCredential` | candidate |
| OQ5+ *(declared non-goal — NOT an open seam now)* | **Byron credential surface** | discriminant-faithful Byron credentials (if ever required) | a SEPARABLE Byron-era follow-up | candidate |
| ENACTMENT+ *(declared non-goal — separable, NOT an open seam now)* | **`proposal_procedures` tx-body decode → `GovAction`** — the wire codec keeps `proposal_procedures` as an opaque `Option<Vec<u8>>` | A typed `Vec<GovAction>` decoded from a real Conway tx body's `proposal_procedures` | a closed `proposal_procedures` sub-grammar reader inside the existing Conway-tx-body surface | candidate (declared non-goal in ENACTMENT-COMMITTEE-WRITEBACK; carried forward in N-E) |
| **CE-NODE-N2C-LTX (NEW cross-cluster obligation introduced in N-E S5; carried to the future node-binary cluster)** | **Live N2C UDS server + N2N bulk-tx inbound listener** — Ade as a server (N2C UDS for cardano-cli) and as an inbound N2N peer that real relays push txs to | The deferred half of CE-N-E-7 + the deferred half of CE-N-E-6 — both reduce through the same BLUE `mempool_ingress` chokepoint that's already wired | The future node-binary cluster ships the live socket loops; the GREEN bridges + canonicalizer + chokepoint + CI gates are already in place at this HEAD; only the `_<date>.log` evidence file is owed | **deferred cross-cluster obligation (new closure pattern; NOT an open seam in N-E)** |
| **N-E+ (declared non-goal in the N-E cluster doc; separable future seam, NOT an open seam now)** | **Outbound tx propagation** — Ade serving txs to peers via tx-submission2 (Ade as a tx source) | An outbound `TxSubmission2Message` stream emitted from the mempool's admitted set | A separate authority surface — a new BLUE/GREEN outbound bridge in `ade_core_interop`; **explicitly declared OUT-OF-SCOPE for N-E** | candidate (declared non-goal in N-E) |
| **N-E+ (declared non-goal in the N-E cluster doc; Tier-5)** | **Mempool bounds / shedding policy** — `CN-MEM-01`, `CN-MEM-03`, `DC-MEM-02` strengthening | A bounded mempool whose shedding rule never alters `admit`'s verdict | Tier-5 cluster extending `OrderPolicy`; would only escalate to N-E scope if a bound changed admission verdicts | candidate (declared non-goal in N-E) |
| DREP-FIDELITY+ *(permanent non-goal — NOT a follow-up)* | **`spo_votes`** | n/a — SPO votes are pool key-hashes only | no change | **permanent non-goal** |
| B+ (full tx UTxO scope) | Full-scope single-tx validity over real resolved UTxO | `TxValidityVerdict` at `track_utxo=true` | `tx_validity` (existing) | candidate |
| B+ (Conway body witness depth) | **Conway block-body vkey-witness closure** — `project_conway_body_witness_gap` | `BlockValidityVerdict` whose body authority runs the same closure as `tx_phase_one` | wire `tx_phase_one` / `verify_required_witnesses` into the Conway block-body path in `rules.rs` | candidate (B2-carried) |
| B+ (pre-Conway tx) | Pre-Conway single-tx validity | `TxValidityVerdict` via per-era body decode + per-era `SignerSource` | extend `decode_tx` + add the era arm to `required_signers` | candidate |
| B1+ (header→body bridge) | Forge/fetch bridge: a fork-choice-winning header triggers a full-block decision on the fetched body | `block_validity(...)` over the fetched body | `ade_node` composition layer joining `process_stream_input` and `block_validity` | candidate |
| B1+ (pre-Babbage block) | TPraos full-block validity (Shelley..Alonzo) | `BlockValidityVerdict` via a TPraos `HeaderInput` projection | extend `block_validity::decode_block` to build `HeaderVrf::Tpraos` headers | candidate |
| N-C | Forge-block inputs (mempool + state + slot + KES + VRF) | `BlockEnvelope` bytes (forged, then re-decoded for validation) | `ade_runtime::forge::forge_block` (proposed) | candidate |
| N-C | Operator block-production trigger | `StreamInput::HeaderArrival(HeaderInput)` | `process_stream_input` (existing) | candidate |
| N-F | LSQ semantic dispatch (LocalStateQuery payloads) | Internal Query enum (closed, not yet defined) | Single dispatch fn that consumes `LocalStateQueryMessage` opaque-bytes payloads | candidate |
| N-F | LocalTxMonitor semantic dispatch | Mempool-snapshot Query/Reply enums | Single dispatch fn that consumes `LocalTxMonitorMessage` opaque-bytes payloads | candidate |
| N-B+ | Live cardano-node session driver (for `ade_core_interop::live_consensus_session`) | `StreamInput` translated from `ChainSyncMessage` and `BlockFetchMessage` events | Composition layer in `ade_core_interop` | candidate |

### Operator-action evidence (live-wire artifacts — not BLUE seams)

The Ade workspace closes Tier-1 wire-level seams in two halves: a
mechanical / GREEN half (code + harness + CI gates that the workspace
itself can certify on every push) and a **live-wire operator-action
half** (a real peer / client at the other end of a real socket
producing bytes Ade has never seen). The latter is captured into
evidence logs at canonical paths in the cluster directory.

**At this HEAD two live-evidence logs are committed**, and one
cross-cluster obligation is named.

| Procedure | Evidence-log artifact | Status at HEAD | What it asserts | TCB |
|-----------|----------------------|----------------|------------------|-----|
| `docs/clusters/completed/PHASE4-N-B/CE-N-B-6_PROCEDURE.md` | `docs/clusters/completed/PHASE4-N-B/CE-N-B-6_<date>.log` | **CAPTURED** (carried from N-B close) | Real cardano-node N-B follow-mode tip agreement | RED operator action |
| `docs/clusters/completed/PHASE4-N-E/CE-N-E-6_PROCEDURE.md` | `docs/clusters/completed/PHASE4-N-E/CE-N-E-6_2026-05-25.log` | **CAPTURED at this HEAD** (NEW — N-E full close) | Outbound-client probe against a real preprod N2N relay: handshake v15, tx-submission2 codec round-trip, BLUE `tx_submission2_transition` state-machine driven correctly under live traffic, accumulator records `frames_received=1 requests_ids=1 tx_bytes=0`. The bulk-tx half (peer pushes txs at us as inbound listener) is documented in the procedure header as deferred to CE-NODE-N2C-LTX. | RED operator action |
| `docs/clusters/completed/PHASE4-N-E/CE-N-E-7_PROCEDURE.md` | (deferred) `CE-NODE-N2C-LTX_<date>.log` in the future node-binary cluster | **DEFERRED to CE-NODE-N2C-LTX** (NEW — cross-cluster obligation pattern) | Real `cardano-cli transaction submit` to Ade over the N2C UDS; verdict matches N2N submission of the same bytes. **Adapter-mechanical half closed in N-E S5** via `n2n_and_n2c_bridges_produce_identical_outcomes`; live-N2C-UDS-server half owed by the future node-binary cluster. | RED operator action (deferred) |

**Operator-action probe binaries (RED — `ade_core_interop::bin::*`).**
The mechanical half of an operator-action evidence pattern is an
`#[ignore]`-gated closure-gate test + a binary that the operator
invokes to capture the live log. The binary is RED (uses `tokio`,
real sockets) and lives in `crates/ade_core_interop/src/bin/`. At
this HEAD there are **two** such binaries:

| Binary | Slice | Live-evidence target | Status |
|--------|-------|----------------------|--------|
| `live_consensus_session` (PHASE4-N-B) | N-B | CE-N-B-6 (live chain-sync follow-mode tip agreement) | captured |
| `live_tx_submission_session` (PHASE4-N-E S6) | N-E S6 | CE-N-E-6 (live N2N tx-submission2 outbound-client probe) | **CAPTURED at this HEAD** |

**Pattern.** Hermetic default mode (codec loopback / readiness probe
that runs in CI without network access — gated `#[ignore]`); plus a
`--connect <peer>` live pass that the operator runs against a real
cardano-node peer. The binary's evidence log is committed alongside
the `_PROCEDURE.md` in the cluster directory.

**These are evidence-log patterns, not BLUE seams.** They do not move
or wrap any chokepoint; they are the on-the-wire half of an existing
mechanical surface. New live-wire CE families should follow the same
pattern: a `_PROCEDURE.md` doc + a `_<date>.log` capture committed to
the cluster directory, plus (when a probe is needed) a
`live_<surface>_session` binary in `ade_core_interop` modeled on these
two. The mechanical half of these CEs is CI-green (the GREEN bridges +
harnesses + integration tests).

User confirmation needed for each candidate at cluster entry. **The
most load-bearing remaining candidates for the bounty** are the
**Conway block-body vkey-witness closure** (the carried B2 gap), the
**forge / header→body bridge**, and **CE-NODE-N2C-LTX** (the deferred
live N2C UDS server + N2N bulk-tx inbound listener — though this is
a node-binary cluster obligation rather than a B+/N-side seam).

---

## 2. Data-Only vs. Authoritative Layers

Ade has thirteen authoritative domains. For each, a single BLUE
chokepoint holds enforcement authority; tooling layers (when they
exist) live in GREEN (`ade_testkit`, `ade_core_interop` bridges) or
RED (`ade_runtime`, `ade_network::mux::transport`,
`ade_network::session`). **N-E added one domain — the mempool ingress
authority — a new BLUE chokepoint `mempool_ingress` sitting between
the RED tx-submission transports and the existing Tier-1 `admit` gate,
plus a GREEN per-peer canonicalizer that is the load-bearing fairness
contract for multi-peer interleavings.** (Prior cluster narratives —
B3 cert/conservation, B4 owner-tagged apply, B5 gov-cert apply,
credential-discriminant fidelity, committee-enactment write-back —
are preserved unchanged below.)

### Mempool ingress — the Tier-1 wire-level / per-peer canonicalizer / `mempool_ingress` boundary (NEW in N-E)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **RED wire transport** | `ade_network::mux::transport` (existing) | RED | Bears bytes off a TCP / UDS socket; no parsing. Untouched by N-E. |
| **BLUE wire grammar (N-A; carried)** | `ade_network::tx_submission::{codec, transition}` (N2N) + `ade_network::n2c::local_tx_submission::{codec, transition}` (N2C) | BLUE | Closed mini-protocol codecs + state machines; emit `InventoryEvent { TxsDelivered { tx_bytes: Vec<Vec<u8>> }, … }` (N2N) or `LocalTxSubmissionEvent { TxSubmitted { tx_bytes }, … }` (N2C). Tx bytes are opaque `Vec<u8>` at this layer. |
| **GREEN bridge — N2N (N-E S4)** | `ade_core_interop::tx_submission::{event_to_ingress, PeerAccumulator, ingest_n2n_events}` | GREEN | Per-peer `InventoryEvent` accumulator. Pure; no I/O; no clocks. The live socket loop that drives a real cardano-node peer is the operator-action half — at this HEAD the executable instance is the **N-E S6 probe binary `live_tx_submission_session`** (RED, `#[ignore]`-gated; documented in `CE-N-E-6_PROCEDURE.md`; live log captured at `CE-N-E-6_2026-05-25.log`). |
| **GREEN bridge — N2C (N-E S5)** | `ade_core_interop::local_tx_submission::{local_event_to_ingress, ClientAccumulator, ingest_n2c_events}` | GREEN | Per-client `LocalTxSubmissionEvent` accumulator. Pure; no I/O; no clocks. The live UDS server loop is owed by the future node-binary cluster (`CE-NODE-N2C-LTX`); the `CE-N-E-7_PROCEDURE.md` doc is retained at the archived cluster path as the cross-cluster anchor. |
| **GREEN per-peer canonicalizer (N-E S3)** | `ade_ledger::mempool::canonicalize::{canonicalize_peer_streams, PeerId, PeerSubmissionQueue}` | GREEN | Deterministic round-robin canonicalization of multi-peer queues. Peers visited in `PeerId` byte-lex order; ties broken by single-byte source tag (N2N=0, N2C=1). Pure function of inputs; output independent of input iteration order. **The load-bearing GREEN fairness contract.** |
| **BLUE chokepoint (N-E S1)** | `ade_ledger::mempool::ingress::{IngressSource, IngressEvent, mempool_ingress}` | BLUE | The single sanctioned production path into `admit` from non-test code. `IngressSource` closed 2-variant; `IngressEvent { source, tx_bytes }` carries the source variant as metadata only; `mempool_ingress` is a pure pass-through. CI-enforced. DC-MEM-03 (`enforced`). |
| **BLUE admission gate (B2 — carried; unchanged)** | `ade_ledger::mempool::admit::admit` | BLUE | A tx is admitted iff `tx_validity(accumulating, tx)` is `Valid`. No false accept (DC-MEM-01, `strengthened_in += PHASE4-N-E`). The only sanctioned production caller is `mempool_ingress`. |
| **GREEN replay harness (N-E S2)** | `ade_testkit::mempool::{ingress_replay::{wrap_as_ingress, b_track_corpus_as_ingress, replay_ingress_trace, BTrackCase, ExpectedOutcome}}` | GREEN | Wraps the B-track adversarial corpus in synthetic `IngressEvent`s. Single-step fold over `mempool_ingress` (CI-enforced). Byte-identical traces (DC-MEM-04). |
| **CI gates (N-E)** | `ci/ci_check_mempool_ingress_closure.sh` + `ci/ci_check_mempool_ingress_replay.sh` | CI | See SEAMS §3 for the full check list; DC-MEM-03 + DC-MEM-04. |
| **Operator-action evidence (N-E)** | `docs/clusters/completed/PHASE4-N-E/{CE-N-E-6_PROCEDURE.md, CE-N-E-7_PROCEDURE.md, CE-N-E-6_2026-05-25.log}` + `live_tx_submission_session` (S6 probe binary) | RED operator action | CE-N-E-6 **captured at this HEAD** (outbound-client depth; bulk-tx half deferred to CE-NODE-N2C-LTX). CE-N-E-7 **DEFERRED to CE-NODE-N2C-LTX** (adapter-mechanical half closed in S5). **Cross-cluster obligation pattern** (NEW — see §5). |

**Rule.** This domain has **two GREEN bridge layers** (one per
transport, both pure deterministic functions in `ade_core_interop`),
**one GREEN canonicalizer** (the per-peer round-robin in
`ade_ledger::mempool::canonicalize` — the load-bearing fairness
contract), and **one BLUE chokepoint** (`mempool_ingress` in
`ade_ledger::mempool::ingress` — the single sanctioned production
path into `admit`). **THE KEY SEAMS:**

1. **The source variant is metadata only — verdict is a function of
   `(state, tx_bytes)` alone, regardless of source variant**
   (source-invariance, N-E-N7 / N-E-8).
2. **Tx bytes flow verbatim from ingress to admit — no decode, no
   re-encode at the ingress layer.** PreservedCbor end-to-end.
3. **`mempool_ingress` is the only sanctioned production path into
   `admit` from non-test code** (DC-MEM-03 — CI-enforced).
4. **The per-peer canonicalizer is the load-bearing GREEN fairness
   contract.** Round-robin by sorted `PeerId` byte-lex with single-byte
   source tie-break (N2N=0, N2C=1).

**New work** that adds an ingress transport attaches by producing
`IngressEvent`s and feeding them into `mempool_ingress` — not by
adding a parallel admission path, not by adding a verdict-side match
on `source`, not by mutating `accumulating` directly. Adding a third
source variant is a closed-enum addition (version-gated).

**Declared non-goals carried from the cluster doc:** outbound tx
propagation (Ade as a tx source), mempool bounds / shedding policy
(Tier-5 strengthening of `DC-MEM-02`), the `proposal_procedures`
tx-body decode into `GovAction`.

**Deferred cross-cluster obligation:** `CE-NODE-N2C-LTX` (live N2C
UDS server + N2N bulk-tx inbound listener) — see §5 cross-cluster
obligation pattern.

### Conway value-conservation accounting — the deposit/refund/withdrawal authority (NEW in B3)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Data-only — cert grammar** | `ade_codec::conway::cert::decode_conway_certs` | BLUE | Closed CDDL grammar over tags `0..18`. No catch-all accept. |
| **Data-only — withdrawals grammar** | `ade_codec::conway::withdrawals::{decode_withdrawals, withdrawals_sum}` | BLUE | Closed map grammar. Never last-wins. |
| **Closed cert domain types** | `ade_types::conway::cert::{ConwayCert, CertDisposition, DepositEffect, CoinSource}` | BLUE | Closed sum types. |
| **Canonical deposit-param surface** | `ade_ledger::pparams::{ConwayOnlyDepositParams, ConwayDepositParams}` + `ade_ledger::state::{conway_deposit_params, conway_deposit_view}` | BLUE | Sole canonical authority (DC-TXV-07). |
| **Closed cert classifier** | `ade_ledger::cert_classify::classify` | BLUE | Total, compiler-exhaustive map. |
| **Authoritative enforcement** | `ade_ledger::conway::check_conway_coin_conservation` | BLUE | Frozen §9.1 reject precedence. |
| **Determinism fold** | `ade_ledger::fingerprint::fingerprint_pparams` | BLUE | Byte-identical for non-Conway. |
| **Allowlisted deposit-param loader** | `ade_testkit` snapshot loader | GREEN | One allowlisted non-canonical source. |
| **Adversarial harness** | `ade_testkit` conservation adversarial corpus (CE-B3-6) | GREEN | No false accept. |

**Rule.** Frozen §9.1 reject precedence: decode → era-validity →
missing-environment → state-dependent-accounting → conservation.

### Conway certificate-state accumulation — the owner-tagged apply authority (NEW in B4)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Data-only — owner-complete cert grammar** | `ade_codec::conway::cert::decode_conway_certs` (+ `decode_drep`) | BLUE | Owner-complete closed CDDL grammar. |
| **Data-only — single shared pool-params decoder** | `ade_codec::shelley::cert::read_pool_registration_cert` | BLUE | ONE pool_params decode site (DC-LEDGER-08). |
| **Closed action classifier** | `ade_ledger::delegation::conway_cert_action` | BLUE | Total, compiler-exhaustive. No `Neutral` action. |
| **Owner-tagged apply model** | `ade_ledger::delegation::apply_conway_cert` | BLUE | Governance-affecting certs owner-tagged out of B4 mutation scope. |
| **Era-dispatch + fail-closed accumulation** | `ade_ledger::rules::accumulate_tx_certs` | BLUE | Fail-closed era dispatch. |

**Rule.** The owner-tagging boundary is the key seam (consumed by B5).

### Credential discriminant fidelity — the closed credential surface (NEW in OQ5; extended in COMMITTEE / DREP / ENACTMENT-COMMITTEE-FIDELITY / ENACTMENT-COMMITTEE-WRITEBACK)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Closed credential domain type** | `ade_types::shelley::cert::StakeCredential` | BLUE | Closed 2-variant `{ KeyHash(Hash28), ScriptHash(Hash28) }`. |
| **Data-only — closed credential-decode chokepoints** | `ade_codec::{shelley,conway}::cert::decode_stake_credential` | BLUE | Each maps `0 → KeyHash`, `1 → ScriptHash`; rejects unknown tag. |
| **Authoritative enforcement — gov-state key surface** | `ade_ledger::state::ConwayGovState.{vote_delegations, committee_hot_keys, drep_expiry, committee}` + `ade_types::conway::governance::GovActionState.{committee_votes, drep_votes}` | BLUE | All discriminated `StakeCredential`. |
| **Determinism — discriminant-faithful fingerprint** | `ade_ledger::fingerprint::{write_stake_credential, write_credential_vote_list}` | BLUE | Emits discriminant before hash. |
| **Narrow read-only boundary adapter** | `StakeCredential::hash()` | BLUE | Sanctioned discriminant-discarding extraction; ONLY against declared non-goal surfaces. |

**Rule.** Discriminant preserved end-to-end on the BLUE authoritative
path (DC-LEDGER-10, strengthened across all five clusters).

### Conway governance-cert accumulation — the owner-tagged apply authority (NEW in B5)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Data-only — owner-complete cert grammar (carried)** | `ade_codec::conway::cert::decode_conway_certs` | BLUE | B4 grammar carried unchanged. |
| **Fail-fast gov-cert environment** | `ade_ledger::state::GovCertEnv` + `LedgerState::gov_cert_env()` | BLUE | Fail-fast `MissingDRepActivityParam`. |
| **Closed total gov-cert dispatch** | `ade_ledger::gov_cert::apply_conway_gov_cert` | BLUE | Total compiler-exhaustive `match` with no `_ =>` wildcard. |
| **Gov-state ingress (era-dispatch + fold)** | `ade_ledger::rules::accumulate_tx_certs` | BLUE | Threads `Option<ConwayGovState>`. |
| **Determinism fold (T-DET-01 migration)** | `ade_ledger::fingerprint` | BLUE | `drep_activity` extension. |

**Rule.** B5 closes the consuming half of B4's owner-tagging boundary
(DC-LEDGER-09, strengthens DC-LEDGER-08).

### Single-tx validity — the per-tx composition root (B2)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Decode / projection** | `ade_ledger::tx_validity::phase1::decode_tx` | BLUE | Lifts the PRESERVED body slice. |
| **Required-signer enumeration** | `ade_ledger::tx_validity::required_signers::*` | BLUE | Closed, era-versioned (DC-TXV-05). |
| **Witness closure** | `ade_ledger::tx_validity::witness::verify_required_witnesses` | BLUE | Fail-closed coverage. |
| **Shared per-tx phase-1** | `ade_ledger::tx_validity::phase1::tx_phase_one` | BLUE | The single per-tx phase-1 authority. |
| **Phase-2 dispatch** | `crate::plutus_eval::try_evaluate_tx` → `ade_plutus::tx_eval::eval_tx_phase_two` | BLUE | Plutus phase-2. |
| **Composition transition** | `ade_ledger::tx_validity::transition::tx_validity` | BLUE | Single chokepoint. |
| **Comparison surface** | `ade_ledger::tx_validity::encoding::*` | BLUE | Canonical CBOR (coarse class). |

**Rule.** Two phase authorities and one composer. The composer
`tx_validity` introduces no rules of its own and never moves
(DC-TXV-02). **N-E note:** the production path from wire transports
into `tx_validity` is now `mempool_ingress` → `admit` → `tx_validity`;
the composer itself is untouched.

### Mempool admission — the Tier-1 / Tier-5 boundary (B2; carried)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Tier-1 admission gate** | `ade_ledger::mempool::admit::admit` | BLUE | Admitted iff `tx_validity(...)` is `Valid`. No false accept (DC-MEM-01). |
| **Mempool state** | `ade_ledger::mempool::admit::MempoolState` | BLUE | `accepted: Vec<Hash32>` + `accumulating: LedgerState`. |
| **Tier-5 ordering policy** | `ade_ledger::mempool::policy::order` | GREEN behavior | Deterministic permutation (DC-MEM-02). |

**Rule.** The Tier-1 / Tier-5 split is load-bearing. **N-E added the
new BLUE bridge `mempool_ingress` ABOVE `admit`** — see the new
"Mempool ingress" domain above. `admit` itself is unchanged.

### Full block validity — the block-level composition root (B1)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Decode / projection** | `ade_ledger::block_validity::header_input::decode_block` | BLUE | Era-dispatched. |
| **Consensus header authority** | `ade_core::consensus::validate_and_apply_header` | BLUE | Decided first, fail-fast. |
| **Ledger body authority** | `ade_ledger::rules::apply_block_with_verdicts` | BLUE | Body half. |
| **Composition transition** | `ade_ledger::block_validity::transition::block_validity` | BLUE | Single chokepoint. |
| **Comparison surface** | `ade_ledger::block_validity::encoding::*` | BLUE | Canonical CBOR (coarse). |

**Rule.** Two sub-authorities and one composer. `block_validity`
never moves. **Known extension points:** the Conway block-body
vkey-witness closure; pre-Babbage TPraos full blocks.

### Ledger application

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Data-only tooling** | `ade_codec` | BLUE\* | Decodes block / tx / cert / withdrawal bytes. |
| **Authoritative enforcement** | `ade_ledger` | BLUE | `apply_block_with_verdicts` / `tx_validity` / `check_conway_coin_conservation` / `accumulate_tx_certs` + `delegation::apply_conway_cert` / `gov_cert::apply_conway_gov_cert` / `governance::apply_committee_enactment`. |
| **Loader** | `ade_runtime::chaindb` + `ade_runtime::recovery` | RED | Reads block / snapshot bytes from disk. |

\* `ade_codec` is BLUE-data-only.

### Stake-snapshot projection for consensus (B1)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Trait boundary** | `ade_core::consensus::ledger_view::LedgerView` | BLUE | Closed 4-method surface. |
| **Production projection** | `ade_ledger::consensus_view::PoolDistrView` | BLUE | The leadership-relevant projection. |
| **Test stub** | `ade_testkit::consensus::ledger_view_stub::LedgerViewStub` | GREEN | Pre-B1 stub. |

**Rule.** `LedgerView` is a closed trait, not a plugin point.

### Plutus phase-2 evaluation

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Data-only tooling** | `ade_plutus::cost_model`, `ade_plutus::script_context` | BLUE | |
| **Script ingress** | `ade_plutus::evaluator::PlutusScript::from_cbor` | BLUE | Named ingress chokepoint. |
| **Authoritative enforcement** | `ade_plutus::tx_eval::eval_tx_phase_two` | BLUE | Single entry. |
| **Quarantine** | `aiken_uplc` git dep | external | Frozen at tag `v1.1.21` `42babe5d`. |

**Rule.** No second public entry; aiken types do not leak.

### Governance ratification / enactment (Conway)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Data-only tooling** | `ade_types::conway` (governance types) | BLUE | |
| **Authoritative enforcement** | `ade_ledger::governance::{evaluate_ratification, enact_proposals, expire_proposals}` | BLUE | Chokepoints. |
| **Committee write-back** *(ENACTMENT-COMMITTEE-WRITEBACK)* | `ade_ledger::governance::apply_committee_enactment` | BLUE | Closed pure transition called at the `rules.rs` epoch boundary. |
| **Snapshot decode (data-only)** *(tightened in 168ac02; carried)* | `ade_testkit` snapshot loader | GREEN | Fail-closed decode of `update_committee`. |

**Rule.** A new governance action variant adds a variant to `GovAction`
+ arms in all three chokepoints. The remaining open seam is the
declared non-goal `proposal_procedures` tx-body decode.

### Mini-protocol wire conformance (N-A)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Data-only tooling (frame)** | `ade_network::mux::frame` | BLUE | Pure encode/decode. |
| **Data-only tooling (messages)** | `ade_network::codec::*` (11 modules) | BLUE | 11 closed wire grammars. |
| **Authoritative enforcement (state)** | `ade_network::*::transition` + `n2c::local_*::transition` | BLUE | 8 closed pure transition functions. |
| **Bearer (I/O)** | `ade_network::mux::transport` | RED | Tokio-based scaffold. |
| **Session composition (placeholder)** | `ade_network::session::mod` | RED | S-A9 placeholder. |
| **Live-interop capture tools** | `ade_network::bin::capture_*` | RED | Operator/dev tools. |
| **Tx-submission bridges** *(N-E)* | `ade_core_interop::{tx_submission, local_tx_submission}` | GREEN | The two N-E bridges that translate `InventoryEvent` / `LocalTxSubmissionEvent` payloads into `IngressEvent`s — see §2 "Mempool ingress" above. |
| **Tx-submission probe binary** *(N-E S6)* | `ade_core_interop::bin::live_tx_submission_session` | RED | The operator-action live N2N probe; `#[ignore]`-gated; produces the CE-N-E-6 evidence log. |

**Rule.** The codec layer is opaque to higher semantics. **N-E wired
the tx-submission2 / local-tx-submission tx-bytes → `mempool::admit`
bridge** (was a candidate seam in the prior revision; closed via the
two GREEN `ade_core_interop` bridges + the BLUE `mempool_ingress`
chokepoint, with the operator-action probe binary
`live_tx_submission_session` shipping at S6 + the CE-N-E-6 live
evidence log captured at this HEAD).

### Praos consensus runtime (N-B)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Data-only tooling (genesis)** | `ade_runtime::consensus::genesis_parser` | RED | Closed `GenesisParseError`. |
| **Schedule authority** | `ade_core::consensus::era_schedule` | BLUE | `EraSchedule::new` validates. |
| **Stake-snapshot boundary** | `LedgerView` ↔ `PoolDistrView` / `LedgerViewStub` | mixed | |
| **Header admission** | `ade_core::consensus::header_validate::validate_and_apply_header` | BLUE | Single chokepoint. |
| **Best-chain authority** | `ade_core::consensus::fork_choice::select_best_chain` | BLUE | Single chokepoint. |
| **Rollback authority** | `ade_core::consensus::rollback::apply_rollback` | BLUE | Single chokepoint. |
| **Candidate materialization** | `ade_runtime::consensus::candidate_fragment` | GREEN | Non-authoritative. |
| **Orchestration** | `ade_runtime::consensus::chain_selector::process_stream_input` | GREEN | Threads `StreamInput`. |
| **Live-interop driver (scaffold)** | `ade_core_interop::bin::live_consensus_session` | RED | The original operator-action probe binary; pattern reused by N-E S6 `live_tx_submission_session`. |
| **Replay harness** | `ade_testkit::consensus::stream_replay::replay_stream` | GREEN | |

**Rule.** Five rules: genesis-parser sole RED→BLUE materialization;
`BootstrapAnchorHash` binds; `LedgerView` closed; authoritative
chokepoints never move; selector and chain-dep advance in lockstep.

### Where the boundary is enforced

- `ci_check_dependency_boundary.sh` — no BLUE crate may depend on RED.
  N-E added no new crate edge beyond `ade_core_interop → ade_ledger`
  (already recorded in CODEMAP).
- `ci_check_no_async_in_blue.sh` — async forbidden in BLUE.
- `ci_check_mempool_ingress_closure.sh` *(N-E — DC-MEM-03,
  `status=enforced`)* — see the §2 "Mempool ingress" entry above.
- `ci_check_mempool_ingress_replay.sh` *(N-E — DC-MEM-04,
  `status=enforced`)*.
- `ci_check_credential_discriminant_closed.sh` — DC-LEDGER-10.
- `ci_check_gov_cert_accumulation_closed.sh` *(B5 — DC-LEDGER-09)*.
- `ci_check_deposit_param_authority.sh` *(B3 — DC-TXV-07)*.
- `ci_check_conway_cert_classification_closed.sh` *(B3F — DC-TXV-06)*.
- `ci_check_no_chaindb_in_consensus_blue.sh` / `ci_check_no_float_in_consensus.sh`
  / `ci_check_no_density_in_fork_choice.sh` / `ci_check_consensus_closed_enums.sh`.
- `ci_check_pallas_quarantine.sh`, `ci_check_no_signing_in_blue.sh`,
  `ci_check_ingress_chokepoints.sh`, `ci_check_ce_n_a_5_proof.sh`.

**N-E note on `ci_check_ingress_chokepoints.sh`:** the `mempool_ingress`
BLUE chokepoint does not construct a `PreservedCbor` — it carries
`Vec<u8>` and passes it to `admit`. The ingress chokepoint is therefore
outside this gate's scope (the existing `tx_validity::decode_tx`
chokepoint downstream is the one that lifts the preserved body slice).

---

## 3. Closed vs. Extensible Registries

Ade's authority surface is **almost entirely closed.** **N-E added
three closed surfaces** — `IngressSource`, `IngressEvent`,
`mempool_ingress` — plus **two CI gates**
(`ci_check_mempool_ingress_closure.sh`,
`ci_check_mempool_ingress_replay.sh`), bringing CI count `29 → 31`.
N-E strengthened one extensible surface's closure (the
`MempoolState.accumulating` field-write is now grep-gated, formerly
review-discipline only). N-E added **no new wholly open extensible
surface**. **N-E S6 (full close) added no new closed enum, no new
chokepoint, no new CI script** — the operator probe binary is RED
operator-action with no library code that adds an authority surface.

### Closed (frozen — version-gated changes only)

| Registry | Location | Count | Change Rule |
|----------|----------|-------|-------------|
| `CardanoEra` | `ade_types::era` | 8 variants | New variant = new hard fork. |
| `Certificate` | `ade_types::shelley::cert` | 7 variants | Shelley-era frozen. **B4:** `PoolRegistrationCert.owners` added. |
| **`StakeCredential`** *(closed 2-variant — NEW shape in OQ5)* | `ade_types::shelley::cert` | 2 variants — `KeyHash(Hash28)`, `ScriptHash(Hash28)` | Grep-gated by `ci_check_credential_discriminant_closed.sh` (DC-LEDGER-10). |
| **Credential-decode chokepoints** *(closed grammar — NEW in OQ5)* | `ade_codec::{shelley,conway}::cert::decode_stake_credential` | 2 functions | |
| **`ConwayCert`** *(closed CDDL grammar — refined in B3, owner-completed in B4)* | `ade_types::conway::cert` | 19 variants over tags `0..18` | Grep-gated by `ci_check_conway_cert_classification_closed.sh`. |
| `GovAction` *(UpdateCommittee re-shaped structured in ENACTMENT-COMMITTEE-WRITEBACK)* | `ade_types::conway::governance` | 7 variants (cardinality unchanged) | One variant re-shaped in place; closed 7-variant. |
| `MIRPot` | `ade_types::shelley::cert` | 2 variants | Frozen. |
| `DRep` | `ade_types::conway::cert` | 4 variants | CIP-1694 fixed. |
| **`CertDisposition`** *(B3)* | `ade_types::conway::cert` | 3 variants | Era-grammar reject is NOT a `DepositEffect`. |
| **`DepositEffect`** *(B3)* | `ade_types::conway::cert` | 2 variants | Closed. |
| **`CoinSource`** *(B3)* | `ade_types::conway::cert` | 3 variants | Closed deposit-provenance set. |
| **`ConwayCertAction`** *(B4)* | `ade_ledger::delegation` | closed — one variant per Conway cert kind | No `Neutral` variant. |
| **`GovernanceCertEffect`** / **`GovernanceOwner`** / **`OwnerTaggedEffect`** / **`ConwayCertOutcome`** *(B4)* | `ade_ledger::delegation` | closed | The owner-tagged effect plumbing B5 consumes. |
| **`GovCertEnv`** *(B5)* | `ade_ledger::state` | closed struct `{ current_epoch, drep_activity }` | Fail-fast `MissingDRepActivityParam`. |
| **`apply_conway_gov_cert` dispatch** *(B5 — closed surface, not a registry)* | `ade_ledger::gov_cert` | 1 function — total `match` over `ConwayCert` | No `_ =>` wildcard. Grep-gated by `ci_check_gov_cert_accumulation_closed.sh` (DC-LEDGER-09). |
| **`apply_committee_enactment` write-back** *(ENACTMENT-COMMITTEE-WRITEBACK — closed surface, not a registry)* | `ade_ledger::governance` | 1 pure transition | Operates on discriminated `BTreeMap<StakeCredential, u64>`. Called at `rules.rs:1224`. |
| **`EnactmentEffects` struct** | `ade_ledger::governance` | closed struct | Grep-gated by `ci_check_credential_discriminant_closed.sh` check 6. |
| **`IngressSource`** *(N-E S1 — DC-MEM-03)* | `ade_ledger::mempool::ingress` | **2 variants — `N2N`, `N2C`** | Closed source discriminant. **No `#[non_exhaustive]`** — grep-defended by `ci_check_mempool_ingress_closure.sh`. A new transport variant requires coordinated updates to the CI gate's per-variant grep, the canonicalizer's `source_byte` tie-break, and the two GREEN bridges. |
| **`IngressEvent`** *(N-E S1 — DC-MEM-03)* | `ade_ledger::mempool::ingress` | closed struct `{ source: IngressSource, tx_bytes: Vec<u8> }` | Closed flat-data envelope carrying tx bytes verbatim. |
| **`mempool_ingress` chokepoint** *(N-E S1 — DC-MEM-03; closed surface, not a registry)* | `ade_ledger::mempool::ingress` | 1 function | The **single BLUE chokepoint** from wire ingress into `admit`. Pure pass-through; body must not reference `source`. Production callers of `admit` outside `mempool/admit.rs` definition and `mempool/ingress.rs` bridge are CI-forbidden. |
| **`MempoolState.accumulating` field-write closure** *(strengthened in N-E S1 — DC-MEM-03)* | `crates/ade_ledger/src/mempool/admit.rs` | 1 production write site | `ci_check_mempool_ingress_closure.sh` greps for `accumulating:[[:space:]]*[A-Za-z]` outside `admit.rs` and fails on any match. |
| `PlutusLanguage` | `ade_plutus::evaluator` | 3 variants (V1, V2, V3) | New variant = new Plutus version. |
| **Named ingress chokepoints (block CBOR)** | `ade_codec::*` | 10 — `decode_block_envelope`, per-era block decoders, `decode_address` | Header comment of `ci_check_ingress_chokepoints.sh` enumerates. |
| **Conway cert/withdrawals sub-grammar decoders** *(B3; cert decoder owner-completed in B4)* | `ade_codec::conway::{cert::{decode_conway_certs, decode_drep}, withdrawals::*}` + `ade_codec::shelley::cert::read_pool_registration_cert` | 5 functions | Closed sub-grammars. |
| **Named ingress chokepoint (Plutus script CBOR)** | `ade_plutus::evaluator::PlutusScript::from_cbor` | 1 — file `crates/ade_plutus/src/evaluator.rs` | Allowlisted by exact file path. |
| **`PreservedCbor::new` constructor** | `ade_codec::preserved` | 1 chokepoint, `pub(crate)` | Construction lives inside `ade_codec`. |
| **`CodecError` variants** *(B3-extended)* | `ade_codec::error` | + `UnknownCertTag`, `DuplicateMapKey` | Flat-data, no `String`. |
| **Mini-protocol message enums** | `ade_network::codec::*` | 11 closed enums | Closed wire grammar per protocol. |
| **Mini-protocol encode/decode chokepoints** | `ade_network::codec::*::{encode_*, decode_*}` | 22 functions | Single chokepoint per direction per protocol. |
| **Mux frame chokepoints** | `ade_network::mux::frame::{encode_frame, decode_frame}` | 2 free functions | The single byte↔frame translation. |
| **Mini-protocol transition functions** | `ade_network::*::transition` + `n2c::local_*::transition` | 8 state-machine modules | Pure, sync, no ambient session influence. |
| **Mini-protocol version enums** | `ade_network::codec::version::*` | 11 closed enums | Each pins the upper version audited. |
| **`ChainDb` trait surface** | `ade_runtime::chaindb::mod` | 6 methods | |
| **`SnapshotStore` trait surface** | `ade_runtime::chaindb::mod` | 5 methods | |
| **`Recoverable` trait surface** | `ade_runtime::recovery` | 2 methods + 1 associated type | |
| **`recover` entry point** | `ade_runtime::recovery::recover` | 1 free function | |
| **Hash domain functions** | `ade_crypto::blake2b::*` | 4 named domains | Algorithm immutable per protocol version. |
| **`ChainEvent`** *(N-B)* | `ade_core::consensus::events` | 5 variants | |
| **`ChainSelectionReject`** *(N-B)* | `ade_core::consensus::events` | 4 variants | |
| **Consensus error families** *(N-B)* | `ade_core::consensus::errors` | 8 closed error enums | |
| **`StreamInput`** *(N-B)* | `ade_runtime::consensus::chain_selector` | 3 variants | |
| **`OrchestratorError`** *(N-B)* | `ade_runtime::consensus::chain_selector` | 2 variants | |
| **`DecodeError`** *(N-B)* | `ade_core::consensus::encoding` | 4 variants | |
| **`GenesisParseError`** *(N-B)* | `ade_runtime::consensus::genesis_parser` | 5 variants | |
| **`GenesisBlob`** *(N-B)* | `ade_runtime::consensus::genesis_parser` | 4 variants | |
| **`NetworkMagic`** *(N-B)* | `ade_runtime::consensus::genesis_parser` | 3 const-named values | |
| **`LedgerView` trait** *(N-B; B1-refined)* | `ade_core::consensus::ledger_view` | 4 methods | |
| **`HeaderVrf`** *(N-B; surfaced at B1)* | `ade_core::consensus::header_summary` | 2 variants | |
| **`BlockValidityVerdict`** *(B1)* | `ade_ledger::block_validity::verdict` | 2 variants | |
| **`BlockValidityError` / `BlockRejectClass` / `FieldKind` / `FieldError` / `MissingInput`** *(B1)* | `ade_ledger::block_validity::verdict` | 5 / 5 / 9 / struct / 4 | |
| **`VerdictSurface` / `SurfaceDecodeError`** *(B1)* | `ade_ledger::block_validity::encoding` | 2 / 3 variants | |
| **`block_validity` chokepoint** *(B1)* | `ade_ledger::block_validity::transition` | 1 function | |
| **`TxValidityVerdict`** *(B2)* | `ade_ledger::tx_validity::verdict` | 2 variants | |
| **`TxRejectClass`** *(B2)* | `ade_ledger::tx_validity::verdict` | 5 variants — discriminants 0..4 fixed | |
| **`TxValidityError`** *(B2)* | `ade_ledger::tx_validity::verdict` | 5 variants | |
| **`SignerSource`** *(B2 — the DC-TXV-05 surface)* | `ade_ledger::tx_validity::required_signers` | 6 variants | |
| **`RequiredSignerError` / `RequiredSignerField`** *(B2)* | 3 / 4 variants | | |
| **`WitnessClosureError` / `WitnessField`** *(B2)* | 3 / 2 variants | | |
| **`TxVerdictSurface` / `TxSurfaceDecodeError`** *(B2)* | 2 / 3 variants | | |
| **`tx_validity` chokepoint** *(B2)* | 1 function | | |
| **Tx-verdict-surface encode/decode chokepoints** *(B2)* | 2 functions | | |
| **`AdmitOutcome`** *(B2)* | `ade_ledger::mempool::admit` | 2 variants | |
| **`MempoolState`** *(B2; field-write grep-gated in N-E)* | `ade_ledger::mempool::admit` | struct `{ accepted, accumulating }` | `accumulating` field-write is the **only** production write site, inside `admit()`. Grep-gated by `ci_check_mempool_ingress_closure.sh`. |
| **`OrderPolicy`** *(B2)* | `ade_ledger::mempool::policy` | 2 variants — ArrivalOrder, TxIdAscending | |
| **`ConwayOnlyDepositParams`** *(B3; B5-enriched)* | `ade_ledger::pparams` | struct + `drep_activity` | |
| **`ConwayDepositParams`** *(B3)* | `ade_ledger::pparams` | struct (view) | |
| **`ValidationEnvironmentError`** *(B3)* | `ade_ledger::error` | | |
| **`UnsupportedStateDependentDepositAccounting`** *(B3)* | `ade_ledger::error` | | |
| **`EraInvalidCertificateError`** *(B3)* | `ade_ledger::error` | | |
| **`PraosNonces` / `NonceScanError`** *(B1)* | `ade_ledger::consensus_input_extract` | | |
| **`PraosChainDepState` / `ChainEvent` canonical encodings** *(N-B)* | `ade_core::consensus::encoding` | 4 chokepoints | |
| **`LedgerFingerprint` fold** *(B3-extended; B5-extended)* | `ade_ledger::fingerprint` | | |
| **CI check set** | `ci/ci_check_*.sh` | **31 scripts (29 → 31 in N-E; no additional in S6 / full close)** | Existing checks may be tightened, never relaxed. |
| **Invariant registry families** | `docs/ade-invariant-registry.toml` | Families T / CN / DC / OP / RO; DC extended across all prior clusters; **N-E added DC-MEM-03 + DC-MEM-04 and appended PHASE4-N-E to `DC-MEM-01.strengthened_in`** | Append-only IDs. |

### Extensible (open within constraints)

| Registry | Location | Extension Rule |
|----------|----------|---------------|
| `CostModels` map (Plutus V1/V2/V3 cost tables) | `ade_plutus::cost_model::CostModels` | Decoder-driven; constrained by closed `PlutusLanguage`. |
| `ProtocolParameters` / `ProtocolParameterUpdate` field set | `ade_ledger::pparams` | Era-versioned. |
| Pool / DRep / Stake registrations | `ade_ledger::state::{DelegationState, CertState}` | Shape closed; set open. |
| Governance proposal / committee / DRep registration set | `ade_ledger::state::ConwayGovState` | Shape closed; instance set open. **ENACTMENT-COMMITTEE-WRITEBACK**: also written back at the epoch boundary by `apply_committee_enactment`. |
| `OpCertCounterMap` *(N-B)* | `ade_core::consensus::praos_state` | BTreeMap; inserts strictly increasing per `(pool, kes_period)`. |
| `PoolDistrView` pool table *(B1)* | `ade_ledger::consensus_view::PoolDistrView::pools` | `BTreeMap<Hash28, PoolEntry>`. |
| Withdrawals map *(B3)* | decoded by `ade_codec::conway::withdrawals::decode_withdrawals` → `BTreeMap<RewardAccount, Coin>` | Shape closed; never last-wins. |
| Mempool admitted set *(B2; ingress-fed in N-E)* | `ade_ledger::mempool::admit::MempoolState::accepted` | `Vec<Hash32>` of admitted tx ids. Shape closed; set open; grows monotonically. Mutated only by `admit` (Tier-1, called only from `mempool_ingress` in production source). NOT runtime-pluggable. |
| `SignerSource` provenance set *(B2)* | `ade_ledger::tx_validity::required_signers::RequiredSigners::{keys, provenance}` | Per-tx open; `SignerSource` *enum* closed. |
| `RollbackSnapshot` ring *(N-B)* | `ade_runtime::consensus::chain_selector::OrchestratorState::recent_snapshots` | Bounded ≤ `DEFAULT_SNAPSHOT_LIMIT = 2160`. |
| Oracle reference snapshots / regression corpus | `ade_testkit::harness::*` | Tooling-only. |
| Network corpus | `corpus/network/{n2n,n2c}/*` | Tooling-only. |
| Consensus corpus | `corpus/consensus/*` | Tooling-only. |
| Block-validity corpus *(B1)* | `corpus/validity/*` | Tooling-only. |
| Tx-validity corpus *(B2; B3-extended)* | `ade_testkit::tx_validity::*` + B3 conservation corpora | Tooling-only. |
| **Mempool ingress corpus** *(N-E; tooling-only)* | `ade_testkit::mempool::ingress_replay` + the B-track corpus wrapped via `b_track_corpus_as_ingress` | Tooling-only. Single-step fold; append-only by convention; GREEN. |
| **Operator-action probe binaries** *(N-B + N-E S6)* | `ade_core_interop::bin::{live_consensus_session, live_tx_submission_session}` | RED operator-action; `#[ignore]`-gated by closure-gate tests; new probe binaries for future operator-action evidence patterns should follow the same `live_<surface>_session` naming convention and live in this directory. |
| `KillStrategy<D>` trait impls | `ade_runtime::chaindb::crash_safety` | RED-only test infrastructure. |
| Recovery state types | callers of `Recoverable` | Open: any state with canonical encode + apply-block step. |
| Pinned external crates | `crates/*/Cargo.toml` | Tier-5 rationale doc required. |

### Candidates — extensible surfaces not yet wired

| Cluster | Candidate registry | Rationale |
|---------|-------------------|-----------|
| **N-E+ Tier-5** | **Mempool eviction / prioritization policy (bounded mempool, shedding policy)** beyond the `OrderPolicy` stub | Tier-5 — operator-tunable. Plugin trait candidate: `MempoolPolicy`. MUST stay below the Tier-1 `admit` gate (DC-MEM-02). Declared OUT-OF-SCOPE in the N-E cluster doc. |
| **N-E+ Tier-1** | **Outbound tx propagation (Ade as a tx source — `tx-submission2` server side)** | Separate authority surface from N-E's ingress half. Declared OUT-OF-SCOPE in the N-E cluster doc. |
| **CE-NODE-N2C-LTX (cross-cluster obligation)** | **Live N2C UDS server + N2N bulk-tx inbound listener** | The deferred halves of CE-N-E-7 + CE-N-E-6; both reduce through `mempool_ingress` (already wired); only the live socket loops + evidence log are owed. |
| N-A (deferred) | Peer address book | Operator-supplied; runtime mutable. |
| N-C | Block-production policy (forge cadence, KES rotation, slot election) | Tier 1 semantics, Tier 5 operator triggers. |
| N-F | Query API method set | Tier 5 wire / Tier 1 semantics. |
| N-F | Prometheus metric names | Tier 5; append-only registry expected. |
| GOVCERT-validity *(OQ-3, separable)* | Committee-membership precondition | Tier 1 — a tx-validity gate, NOT a registry. |
| credential-discriminant *(WIRED + CLOSED)* | DONE — see closed surfaces above. | |
| proposal-decode *(declared non-goal — separable, NOT an open seam now)* | `proposal_procedures` tx-body decode into `GovAction` | Carried from ENACTMENT-COMMITTEE-WRITEBACK; declared non-goal in N-E. |

User confirmation needed for each at cluster entry.

### Closed-grammar audit (PHASE4-N-E full close)

This sweep was performed after PHASE4-N-E full close (S1..S6 + live
N2N evidence + archive).

1. **`IngressSource`** — **closed by intent.** Closed 2-variant enum;
   no `#[non_exhaustive]`; grep-gated.
2. **`IngressEvent`** — **closed by intent.** Closed flat-data struct.
3. **`mempool_ingress` chokepoint** — **closed by intent.** Body must
   not reference `source`; production callers of `admit` outside this
   chokepoint are CI-forbidden.
4. **`MempoolState.accumulating` field-write closure** — **grep-gated
   in N-E.**
5. **Per-peer canonicalizer** — **closed by intent on the algorithm.**
6. **GREEN replay harness — single-step fold** — **closed by intent.**
7. **N2N/N2C GREEN bridges** — **closed by intent on the
   event-to-ingress mapping.** Only `TxsDelivered` (N2N) /
   `TxSubmitted` (N2C) produce `IngressEvent`s.
8. **Operator-action probe binary `live_tx_submission_session` (S6)** —
   **closed by pattern.** RED operator-action; `#[ignore]`-gated;
   modeled on `live_consensus_session`; default hermetic / `--connect`
   live; produces the CE-N-E-6 evidence log against a real preprod N2N
   relay. No new BLUE / GREEN library code; no new authority surface.

**Gap note — N-E (narrow).** The N-E GREEN bridges in `ade_core_interop`
(`tx_submission.rs`, `local_tx_submission.rs`) are not in any
grep-gate's `TARGETS` scope — their closed `event_to_ingress` /
`local_event_to_ingress` `match` shape rests on review-discipline +
the compiler-exhaustive `match` over the closed `InventoryEvent` /
`LocalTxSubmissionEvent` enums (closed at N-A). Not load-bearing today.

### Closed-grammar audit (carried — PHASE4-B3 / B4 / B5)

Carried unchanged from the prior revision: `ConwayCert` /
`CertDisposition` / `DepositEffect` / `CoinSource` / withdrawals
grammar / deposit-param authority / §9.1 reject precedence (B3);
owner-complete `ConwayCert` / `decode_drep` / shared
`read_pool_registration_cert` / owner-tagged apply types /
owner-tagging boundary to `ConwayGovState` (B4); `apply_conway_gov_cert`
dispatch / `GovCertEnv` + `gov_cert_env()` / DRep expiry arithmetic /
`ConwayGovState` accumulation path (B5).

**Carried gap — B4 (narrow).** The `ade_ledger::delegation`
owner-tagged apply types live in `crates/ade_ledger/src/delegation.rs`,
which is NOT in the `TARGETS` array of
`ci_check_consensus_closed_enums.sh`. Unchanged at this HEAD.

---

## 4. Version-Gated vs. Frozen Contracts

### Frozen (immutable at current version — change = new major version)

- **Cardano-canonical CBOR wire format**.
- **Block envelope shape**: `[era_tag:u8, era_block:CBOR]`; era tags
  0..=7 (closed).
- **`PreservedCbor<T>` invariant**.
- **Hash algorithms**: Blake2b-224 / 256, Ed25519, Byron-bootstrap,
  KES-sum, VRF-draft-03.
- **Era-correct block body hash** *(B1)*: preserved-CBOR-segment bytes (T-ENC-01).
- **Tx id over preserved body bytes** *(B2)*.
- **Conway certificate CDDL grammar** *(B3; hardened in B3F; owner-completed in B4)*.
- **Conway `DRep` decode grammar** *(B4)*: closed (no catch-all).
- **Owner-tagged Conway cert-state apply contract** *(B4)*: DC-LEDGER-08.
- **Closed total gov-cert dispatch contract** *(B5)*: DC-LEDGER-09.
- **Fail-fast gov-cert environment** *(B5)*: `GovCertEnv` via `gov_cert_env()` only.
- **Checked DRep-expiry arithmetic** *(B5)*: `checked_add`; `DRepActivityOverflow`.
- **`ConwayGovState` deterministic-fold accumulation** *(B5)*.
- **Conway withdrawals map grammar** *(B3)*: never last-wins.
- **Closed deposit-effect sum types** *(B3)*.
- **Canonical deposit-param authority** *(B3)*: DC-TXV-07.
- **Full Conway value-conservation equation** *(B3)*: frozen §9.1 reject precedence.
- **`LedgerFingerprint` Conway deposit-param fold** *(B3)*: byte-identical for non-Conway.
- **Plutus script ingress chokepoint**: `PlutusScript::from_cbor`.
- **Plutus language set**: V1, V2, V3.
- **Aiken UPLC quarantine pin**: `aiken_uplc` at tag `v1.1.21`.
- **Ouroboros mux frame layout**: 8-byte big-endian header, payload ≤ 65535.
- **11 closed mini-protocol message enums** + **8 closed state graphs** (N-A).
- **`BootstrapAnchorHash` v1 preimage** *(N-B)*.
- **`EraSchedule` invariants** *(N-B)*.
- **`PraosChainDepState` / `ChainEvent` CBOR encodings** *(N-B)*.
- **Consensus error taxonomies** *(N-B)*.
- **`StreamInput` 3-variant taxonomy** *(N-B)*. **`HeaderVrf` era model**.
- **`block_validity` composition contract** *(B1)*.
- **`VerdictSurface` CBOR encoding** *(B1)*.
- **`LedgerView` trait shape** *(N-B; B1-refined)*.
- **`tx_validity` composition contract** *(B2)*.
- **`SignerSource` enumeration** *(B2)*.
- **Witness-closure contract** *(B2)*.
- **`TxVerdictSurface` CBOR encoding** *(B2)*.
- **Mempool admission contract** *(B2)*: `admit`'s verdict equals
  `tx_validity`'s verdict; no false accept (DC-MEM-01).
- **`mempool_ingress` chokepoint contract** *(N-E S1)*: the
  **single sanctioned production path into `admit` from wire ingress**.
  `mempool_ingress(state, evt) == admit(state, evt.tx_bytes())` — a
  pure pass-through; `event.source` is recorded but MUST NOT affect
  the verdict. Production callers of `admit` outside `mempool/admit.rs`
  definition and `mempool/ingress.rs` bridge are CI-forbidden;
  `tests/` and `benches/` are exempt. Removal / renaming / a second
  public production path into `admit` from production source is
  CI-forbidden (DC-MEM-03).
- **`IngressSource` source-invariance contract** *(N-E S1 — N-E-N7 / N-E-8)*:
  `IngressSource` is metadata only — the verdict is a function of
  `(state, tx_bytes)` alone, regardless of source variant. Defended
  by `ci_check_mempool_ingress_closure.sh` check 5.
- **Verbatim tx-bytes flow through ingress** *(N-E)*: tx bytes flow
  verbatim from the wire grammar through the GREEN bridges and
  canonicalizer into `mempool_ingress` and then into `admit`. **No
  decode, no re-encode at the ingress layer.**
- **GREEN single-step replay fold contract** *(N-E S2 — DC-MEM-04)*:
  replaying the same ordered `[IngressEvent]` against the same `base`
  produces a byte-identical `(MempoolState, Vec<AdmitOutcome>)` pair;
  the fold is single-step per OQ-6.
- **Cross-cluster obligation pattern** *(NEW — introduced in N-E
  full close)*: a cluster may close on a partial CE provided that
  (a) the mechanical half is fully landed in this cluster, (b) the
  deferred half names its successor cluster + identifier explicitly,
  (c) the procedure doc is retained at a canonical path so the
  successor cluster has zero ambiguity, and (d) the cluster doc
  records the split unambiguously. The N-E instance: CE-N-E-7 split
  into adapter-mechanical (closed in S5) + live-N2C-UDS-server
  (deferred to `CE-NODE-N2C-LTX` in the future node-binary cluster);
  the CE-N-E-6 bulk-tx half similarly joins CE-NODE-N2C-LTX. The
  retained procedure docs at
  `docs/clusters/completed/PHASE4-N-E/CE-N-E-{6,7}_PROCEDURE.md` are
  the cross-cluster anchors. **This pattern is a frozen project-level
  closure contract** — once a cross-cluster obligation is named, the
  successor cluster MUST honor the identifier (`CE-NODE-N2C-LTX`)
  and the canonical evidence path (`CE-NODE-N2C-LTX_<date>.log`);
  re-numbering or moving the procedure docs is forbidden by
  review-discipline.
- **Operator-action evidence pattern** *(strengthened in N-E full
  close)*: live-wire evidence for a Tier-1 surface lives in two
  artifacts under the cluster directory: a `_PROCEDURE.md` doc and a
  `_<date>.log` capture. Where a probe binary is needed it lives at
  `crates/ade_core_interop/src/bin/live_<surface>_session.rs` and
  follows the hermetic-default-plus-`--connect`-live shape of
  `live_consensus_session` (N-B) and `live_tx_submission_session`
  (N-E S6). At this HEAD two probe binaries + two captured logs are
  in tree.
- **Closed credential discriminant contract** *(OQ5 / COMMITTEE / DREP /
  ENACTMENT-COMMITTEE-FIDELITY / ENACTMENT-COMMITTEE-WRITEBACK)*.
- **Committee-enactment write-back contract** *(ENACTMENT-COMMITTEE-WRITEBACK)*:
  `apply_committee_enactment` at `rules.rs:1224`; `NoConfidence`
  dissolves the committee (DC-EPOCH-01 / DC-LEDGER-10).
- **All canonical types**: shapes frozen at the era / version they
  entered.
- **TCB color assignments**: per `.idd-config.json` `core_paths`.
  `ade_core::consensus`, `ade_ledger::{block_validity, tx_validity,
  mempool::admit, mempool::ingress, consensus_view, cert_classify,
  delegation, gov_cert}`, `ade_codec::conway::{cert, withdrawals}`,
  `ade_codec::shelley::cert`, and `ade_types::conway::cert` are BLUE;
  `ade_ledger::mempool::policy` and `ade_ledger::mempool::canonicalize`
  are GREEN behavior inside the BLUE crate;
  `ade_ledger::consensus_input_extract` is RED-behavior-inside-BLUE;
  `ade_runtime::consensus` is RED;
  `ade_testkit::{consensus, validity, tx_validity, mempool}` is GREEN;
  `ade_core_interop` (incl. the N-E `tx_submission` /
  `local_tx_submission` bridges and the N-E S6 probe binary
  `live_tx_submission_session`) is RED-crate / GREEN-pure-functions /
  RED-operator-action-binaries.
- **`ChainDb` / `SnapshotStore` / `Recoverable` trait shapes** (N-D
  closed): trait method sets frozen.

### Version-gated (can evolve across major versions)

- **New `CardanoEra` variant**: full coordinated change.
- **New Conway certificate tag** *(B3 / B4 / B5)*: compiler-exhaustive
  matches break the build until every arm is added.
- **New `CoinSource` deposit-provenance** *(B3)*.
- **Pre-Conway single-tx validity** *(B2 extension point)*.
- **Full-scope `track_utxo=true` tx corpus** *(B2 extension point)*.
- **Conway block-body vkey-witness closure** *(B2-carried, post-B3/B4/B5/N-E)*.
- **Conway governance certificate accumulation authority** *(B5, WIRED + CLOSED)*.
- **Credential discriminant extension** *(declared non-goal carried)*.
- **Committee-enactment write-back** *(ENACTMENT-COMMITTEE-WRITEBACK, WIRED + CLOSED)*.
  **Remaining separable follow-up:** decoding `proposal_procedures`
  from real tx bodies into `GovAction`.
- **TPraos full-block validity** *(B1 extension point)*.
- **New `GovAction` / Plutus version variant**.
- **New `SignerSource` variant** *(B2)*.
- **New `TxRejectClass` / `BlockRejectClass` / `FieldKind` /
  `MissingInput` variant**.
- **New `OrderPolicy` variant** *(B2)*.
- **New protocol parameter field**.
- **New CI check**: additive. (N-E added two; full close added none.)
- **Pinned external crate bump**: Tier-5 rationale doc required.
- **New mini-protocol**.
- **Mini-protocol version-table bump**.
- **New `ChainEvent` / `ChainSelectionReject` / `StreamInput` variant** *(N-B)*.
- **New `NetworkMagic`** *(N-B)*.
- **New `LedgerView` impl / LedgerState-backed `PoolDistrView` constructor**.
- **`BootstrapAnchorHash` preimage v2** *(N-B)*: hard version-gated.
- **N2N/N2C tx-submission → `mempool_ingress` ingress** *(WIRED + CLOSED in N-E; live N2N outbound-client probe evidence captured at this HEAD)*:
  the GREEN bridges + the BLUE chokepoint are the canonical wiring.
  **Remaining separable version-gated follow-ups** (NOT open seams
  now): a third `IngressSource` variant (would require a coordinated
  CI-gate update + the canonicalizer's `source_byte` extension + a
  new GREEN bridge); outbound tx propagation; a bounded mempool /
  shedding policy; and the **cross-cluster obligation
  `CE-NODE-N2C-LTX`** (live N2C UDS server + N2N bulk-tx inbound
  listener — owed by the future node-binary cluster).
- **Phase-4 cluster surface additions** (N-C, N-F): each cluster's
  wire surface gates additions via its own cluster doc.

---

## 5. Module Addition Rules

Ade's workspace is small and color-disciplined. **N-E added two new
submodules entirely inside the existing BLUE `ade_ledger` crate**
(`ade_ledger::mempool::ingress` BLUE + `ade_ledger::mempool::canonicalize`
GREEN), **one new GREEN submodule inside the existing GREEN
`ade_testkit` crate** (`ade_testkit::mempool` with `ingress_replay.rs`),
**two new GREEN files inside the existing RED `ade_core_interop`
crate** (`tx_submission.rs`, `local_tx_submission.rs`), and **one
new RED operator-action probe binary inside `ade_core_interop`**
(`bin/live_tx_submission_session.rs` — N-E S6). It added **no new
crate, no new external ingress wire-format frozen contract, and no
new public composer.** The new BLUE chokepoint `mempool_ingress`
sits in `ade_ledger::mempool` alongside the existing `admit` /
`policy` — the same crate-internal seam B2 established for the
Tier-1 / Tier-5 mempool split.

**The module-addition rule N-E sets for future wire-ingress work:**

1. **A new wire-ingress transport attaches as a closed source
   variant on `IngressSource`** (version-gated; coordinated update
   to the CI gate's per-variant grep + the canonicalizer's
   `source_byte` tie-break + a new GREEN bridge in `ade_core_interop`).
2. **A new wire-ingress transport bridge lives in `ade_core_interop`**:
   a deterministic GREEN `event_to_ingress` mapping + a
   per-peer/per-client accumulator + an orchestrator that calls
   `canonicalize_peer_streams` and `replay_ingress_trace`. The
   bridge code itself is pure / deterministic; the live socket loop
   is the RED operator-action half.
3. **The operator-action half ships as a `live_<surface>_session`
   probe binary** in `crates/ade_core_interop/src/bin/` modeled on
   `live_consensus_session` (N-B) and `live_tx_submission_session`
   (N-E S6). Hermetic-default mode (codec loopback / readiness probe
   that runs in CI without network access — gated `#[ignore]`) plus
   a `--connect <peer>` live pass that the operator runs against a
   real cardano-node peer / `cardano-cli` / future inbound listener.
   The evidence log is committed alongside the `_PROCEDURE.md` in
   the cluster directory.
4. **A new wire-ingress transport must NOT add a parallel admission
   path** — `mempool_ingress` is the single sanctioned BLUE
   chokepoint into `admit` from non-test code.
5. **A new wire-ingress transport must NOT branch the verdict on
   the source variant** — `IngressSource` is metadata only.
6. **A new wire-ingress transport must NOT decode and re-encode the
   tx bytes at the ingress layer** — verbatim flow into `admit`.

### Cross-cluster obligation pattern (NEW — introduced in N-E full close)

PHASE4-N-E full close introduces the **cross-cluster obligation
pattern** for the first time in this project. The pattern lets a
cluster legitimately close while explicitly deferring a half of a
CE to a successor cluster. The first instance: CE-N-E-7 is split
into:

- The **adapter-mechanical half** — closed in N-E S5 via the
  integration test `n2n_and_n2c_bridges_produce_identical_outcomes`
  (wires the same `tx_bytes` through both `ingest_n2n_events` and
  `ingest_n2c_events` against the same base ledger state and asserts
  byte-identical outcomes).
- The **live-N2C-UDS-server half** — DEFERRED to the future
  node-binary cluster under the new identifier `CE-NODE-N2C-LTX`.

The CE-N-E-6 outbound-client probe evidence (captured at this HEAD)
similarly defers the **bulk-tx half** (where a peer actively pushes
txs at Ade as an inbound listener) to the same `CE-NODE-N2C-LTX`,
because both halves require Ade to host a server side that the
current `ade_core_interop` library + the operator probe binary do
not.

**The pattern's binding rules** (a frozen project-level closure
contract — see §4):

1. **The cluster doc MUST record the split unambiguously** —
   "adapter-mechanical half closed in this cluster, live-server
   half deferred to <successor cluster identifier>".
2. **The deferred half MUST name its successor cluster + identifier
   explicitly** — N-E names `CE-NODE-N2C-LTX` and the future
   node-binary cluster.
3. **The mechanical half MUST be fully landed in this cluster** —
   no carry-forward of mechanical work; the deferral is strictly
   on live-wire evidence that requires an Ade-side server.
4. **The procedure doc MUST be retained at a canonical path** —
   `docs/clusters/completed/PHASE4-N-E/CE-N-E-7_PROCEDURE.md`
   stays in the archived cluster directory as the cross-cluster
   anchor; the successor cluster picks it up by reference.
5. **The successor cluster MUST honor the identifier and canonical
   evidence path** — `CE-NODE-N2C-LTX_<date>.log` in the future
   cluster directory; re-numbering or moving is forbidden.

**New work that adds a cross-cluster obligation** attaches by
naming the successor cluster identifier in the deferring cluster's
doc, retaining a `_PROCEDURE.md` at the deferring cluster's
archived path, and recording the obligation in this SEAMS §1
candidate table (the `CE-NODE-N2C-LTX` row at this HEAD).

### Operator-action evidence pattern (strengthened in N-E full close)

This pattern was established by PHASE4-N-B (CE-N-B-6 live tip
agreement) and is reinforced by PHASE4-N-E with two captured logs +
the new probe binary `live_tx_submission_session`.

**Captured artifacts at this HEAD:**

- `docs/clusters/completed/PHASE4-N-B/CE-N-B-6_<date>.log` — chain-sync
  follow-mode tip agreement (carried).
- `docs/clusters/completed/PHASE4-N-E/CE-N-E-6_2026-05-25.log` —
  tx-submission2 outbound-client probe (NEW at this HEAD).

**Pending closure (cross-cluster):**

- `CE-NODE-N2C-LTX` in the future node-binary cluster — live N2C UDS
  server + N2N bulk-tx inbound listener.

**Pattern shape:**

- A `_PROCEDURE.md` doc in the cluster directory documenting the
  operator-action script (preconditions, command line, expected
  observables).
- A `_<date>.log` capture committed alongside the procedure doc as
  the evidence artifact.
- (Where a probe is needed) a `live_<surface>_session` binary in
  `crates/ade_core_interop/src/bin/`, hermetic-default + `--connect`-live,
  `#[ignore]`-gated by a closure-gate test that runs in CI without
  network access.

**OQ5/COMMITTEE/DREP/ENACTMENT-COMMITTEE-FIDELITY/WRITEBACK** all
followed the in-place-tightening model. **B5** added one new
crate-internal BLUE module + one new CI gate. **B4** added the
owner-tagged apply model in place. **B3** added four BLUE submodules
inside existing BLUE crates. **B2** added the `tx_validity::*` and
`mempool::{admit, policy}` submodule trees. **N-E follows the B2
pattern** + introduces the **cross-cluster obligation pattern** for
deferred live-wire halves + reinforces the **operator-action evidence
pattern** with the second captured log + the second probe binary.

| Color | Naming convention | Build-config flags | May depend on | MUST NOT depend on |
|-------|-------------------|--------------------|----------------|--------------------|
| **BLUE** | `ade_*` | First line of every `.rs` is the contract banner. `lib.rs` carries `#![deny(unsafe_code, clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_arithmetic)]`. No `#[cfg(feature = ...)]`. No async. No `ChainDb`/`f32`/`f64`/density inside `ade_core::consensus`. No `#[non_exhaustive]`/open-tail/`String`/`Box<dyn>` in `ade_core::consensus`, `ade_ledger::block_validity`, `ade_ledger::tx_validity`, `ade_ledger::mempool`. **N-E:** `IngressSource` closed 2-variant; `IngressEvent` closed flat-data struct; `mempool_ingress` body must not reference `source` (DC-MEM-03). | Other BLUE crates / submodules only | Any RED submodule or crate; GREEN in non-dev deps; `pallas_*` (except `ade_plutus`); async runtime; `HashMap`/`HashSet`/`IndexMap`; clock/rand/float/env/I/O. |
| **GREEN** | `ade_*` | Banner + deny attrs are project convention but not currently enforced for `ade_testkit` / `ade_network::mux::mod` / `ade_ledger::mempool::policy` / `ade_ledger::mempool::canonicalize` / `ade_testkit::mempool::ingress_replay` / the two `ade_core_interop` N-E bridges. **N-E:** the canonicalizer body is grep-gated NO async / RNG / clock / `HashMap` / `HashSet` / `RwLock` / `Mutex` (`ci_check_mempool_ingress_replay.sh` check 6); the replay harness is grep-gated single-step (check 4). | BLUE crates + standard library + ecosystem crates | `ade_runtime` (for `ade_testkit`); RED submodules in non-test paths. Results must never feed back into a BLUE authoritative decision. |
| **RED** | `ade_*` | No special header. Free to use clocks, I/O, async, `HashMap`, signing keys. The operator-action probe binaries in `crates/ade_core_interop/src/bin/` (currently `live_consensus_session`, `live_tx_submission_session`) follow the hermetic-default / `--connect`-live pattern and are `#[ignore]`-gated by closure-gate tests. | Any BLUE / GREEN crate or submodule (one-way) | Cannot be depended on by BLUE. |

### New module checklist

1. **Add to `Cargo.toml` workspace members** (if a new crate).
2. **Declare TCB color** by editing `.idd-config.json` `core_paths` if BLUE.
3. **CI script update obligations** — extend the relevant BLUE-scoped
   scripts.
4. **Add contract banner** (BLUE) to every `.rs` file.
5. **Add deny attributes** to `lib.rs` (BLUE).
6. **New canonical types:** add a `[[rules]]` block under family `T`
   in the invariant registry, plus a round-trip test.
7. **New operator-action probe binary:** add to
   `crates/ade_core_interop/src/bin/<name>.rs` following the
   `live_<surface>_session` naming + hermetic-default-plus-`--connect`-live
   shape; document in `<cluster>/CE-<id>_PROCEDURE.md`; capture
   evidence to `<cluster>/CE-<id>_<date>.log`.
8. **Cross-cluster obligation:** if a CE is split, follow the 5
   binding rules above; record the obligation in §1 / §3 / §5 of
   this SEAMS at the next refresh.
9. **Run `cargo test --workspace` and the full CI script suite.**

### Phase 4 anticipated additions

- **PHASE4-N-E (Tier 1 wire-level mempool ingress) — FULLY CLOSED at
  this HEAD**: code + CE-N-E-6 live N2N evidence captured + cluster
  archived. CE-N-E-7 + bulk-tx halves deferred to CE-NODE-N2C-LTX.
- **Tx-validity completeness follow-ups**: full `track_utxo=true`
  corpus; pre-Conway eras; the Conway block-body vkey-witness
  closure (carried).
- **Future node-binary cluster (`CE-NODE-N2C-LTX`)**: live N2C UDS
  server + N2N bulk-tx inbound listener. The GREEN bridges +
  canonicalizer + chokepoint + CI gates are already in place; only
  the live server loops + the `CE-NODE-N2C-LTX_<date>.log` evidence
  file are owed.
- **Outbound tx propagation (post-N-E)**: Ade as a tx source via
  `tx-submission2` server side. Declared non-goal in N-E.
- **Mempool bounds / shedding policy (Tier-5)**: extends `OrderPolicy`
  with a bounded-eviction variant. Declared non-goal in N-E.
- **`proposal_procedures` decode (post-ENACTMENT-COMMITTEE-WRITEBACK)**:
  carried; declared non-goal in N-E.
- **B4 / sync — LedgerState-backed `PoolDistrView`**.
- **header→body bridge**: `ade_node` composition layer joining
  `process_stream_input` and `block_validity`.
- **N-C (forge)**: forge-block path in `ade_runtime` (RED).
- **N-F (operator API)**: thin RED layer mapping a closed Query enum
  to gRPC/HTTP.

**These placements are candidates** — user confirmation needed at
cluster entry.

---

## 6. Forbidden Patterns (per color)

### BLUE (universal IDD prohibitions; enforced by CI where marked)

- No `HashMap`, `HashSet`, `IndexMap`, `IndexSet`.
- No `SystemTime`, `Instant`, `std::time::*` clocks.
- No `rand::thread_rng`, `thread::spawn`.
- No `f32`, `f64`, floating-point arithmetic.
- No `std::fs`, `std::net`, `tokio`, `async fn`.
- No `anyhow`; `unwrap`/`expect`/`panic` denied at the lint level.
- No `unsafe` outside an explicit allowlist.
- No `#[cfg(feature = ...)]` semantic gating.
- No signing patterns in BLUE.
- No re-hashing of `canonical_bytes` or re-encoded bytes — wire bytes only.
- No construction of `PreservedCbor` outside `ade_codec`.
- No raw CBOR decoding in any BLUE crate except `ade_codec` and the
  single allowlisted file `crates/ade_plutus/src/evaluator.rs`.
- No `pallas_*` reference outside `ade_plutus`.
- **(N-A specific)** No `Box<dyn Codec>` / `Box<dyn Protocol>` on
  mini-protocol enums. No reading "selected protocol version" from a
  session global. No decoding block/tx/address CBOR inside `ade_network`.
- **(N-B specific)** No `ChainDb` in `ade_core::consensus`. No density
  in fork choice. No `#[non_exhaustive]`/open-tail/`String`/`Box<dyn>`
  in `ade_core::consensus`.
- **(B1 specific)** No `Valid` block verdict that skips either authority.
  No partial mutation on the invalid path. No fail-open length/size guard.
- **(B2 specific)** No `#[non_exhaustive]`, no open-tail, no owned
  `String`, no `Box<dyn>` in `ade_ledger::tx_validity` or
  `ade_ledger::mempool`. No `Valid` tx verdict that skips either
  phase. No reading `track_utxo=false` as "full validity".
- **(B2 specific — `mempool::admit`)** No false accept (DC-MEM-01).
- **(B3 specific)** No catch-all accept in `decode_conway_certs`. No
  last-wins withdrawals. No deposit/refund literal next to a deposit
  field. No guessed state-dependent deposit/refund. No accept of a
  cert/withdrawal-bearing tx without the full value check.
- **(B4 specific)** No reduction of `ConwayCert` into Shelley
  `Certificate`. No flattening to neutral. No dropping of owner
  payloads. No swallowing of decode or apply errors. No second
  pool-params decoder.
- **(B5 specific)** No `_ =>` wildcard in `apply_conway_gov_cert`.
  No reintroduction of the B4 observe-and-drop. No fabricated
  `GovCertEnv`. No unchecked DRep-expiry arithmetic.
- **(OQ5 / COMMITTEE / DREP)** No tuple-struct `StakeCredential(Hash28)`.
  No tag-erasing decode. No bare-`Hash28` `StakeCredential(<hash>)`
  coercion on BLUE. No re-key of `ConwayGovState`'s discriminated
  maps. No DRep OR-fallback. `cred.hash()` only against declared
  non-goal surfaces.
- **(ENACTMENT-COMMITTEE-WRITEBACK)** No opaque-bytes
  `UpdateCommittee`. No committee write-back outside
  `apply_committee_enactment`. No re-collapse of the discriminant.
  `rules.rs` MUST call `apply_committee_enactment` at the epoch
  boundary.
- **(N-E specific — closed BLUE chokepoint `mempool_ingress`)** No
  reference to `event.source` (`\bsource\b` token) inside the
  `mempool_ingress` function body — grep-defended by
  `ci_check_mempool_ingress_closure.sh` check 5. The body must
  remain a pure pass-through to `admit(mempool, event.tx_bytes())`.
  No `String`, no `Box<dyn>`, no `#[non_exhaustive]` on
  `IngressSource` or `IngressEvent`. No second public production
  path into `admit` outside this chokepoint (check 4 — `tests/` and
  `benches/` exempt). No mutation of `MempoolState.accumulating`
  from outside `mempool/admit.rs` (check 3). No decode / re-encode
  of tx body bytes at this layer — verbatim flow into `admit`.

### GREEN (`ade_testkit` incl. `validity` / `tx_validity` / `mempool` + B3/B4/B5/OQ5/COMMITTEE/DREP corpora; `ade_network::lib` / `mux::mod`; `ade_runtime::consensus::{candidate_fragment, chain_selector}`; `ade_ledger::mempool::{policy, canonicalize}`; the two `ade_core_interop` N-E bridges)

- No nondeterminism that leaks into stored fixtures — fixtures must
  be byte-reproducible.
- No participation in authoritative outputs.
- No `HashMap` even in test helpers — `BTreeMap` only.
- No import of `ade_runtime` from `ade_testkit`.
- (`ade_runtime::consensus::chain_selector`) No comparison decision.
- (`ade_ledger::mempool::policy`) No call to `tx_validity`; no read
  of the accumulating state; no add/remove of a tx id (DC-MEM-02).
- **(`ade_ledger::mempool::canonicalize`, N-E)** No async / RNG /
  clock / `HashMap` / `HashSet` / `RwLock` / `Mutex`. The per-peer
  ordering rule (round-robin by sorted `PeerId` byte-lex, source-byte
  tie-break N2N=0/N2C=1) is the load-bearing GREEN fairness contract
  — any change is SEAMS-level.
- **(`ade_testkit::mempool::ingress_replay`, N-E)** No call to
  direct `admit` from `replay_ingress_trace` (must go through the
  BLUE bridge `mempool_ingress`). No batching / parallel /
  out-of-order helpers. The fold is strictly single-step per OQ-6.
- **(`ade_core_interop::tx_submission` / `local_tx_submission`, N-E)**
  Pure deterministic functions over their inputs; no I/O, no clocks
  inside `event_to_ingress` / `PeerAccumulator::observe` /
  `local_event_to_ingress` / `ClientAccumulator::observe` /
  `ingest_n2n_events` / `ingest_n2c_events`. The compiler-exhaustive
  `match` over the closed `InventoryEvent` / `LocalTxSubmissionEvent`
  enums keeps the event-to-ingress mapping closed: only
  `TxsDelivered` (N2N) and `TxSubmitted` (N2C) produce ingress
  events; all other variants emit empty Vec.

### RED (`ade_runtime`, `ade_node`, `ade_network::mux::transport`, `ade_network::session`, `ade_network::bin::capture_*`, `ade_runtime::consensus::genesis_parser`, `ade_core_interop` (incl. N-E S6 probe binary `live_tx_submission_session`), and the RED-behavior `ade_ledger::consensus_input_extract` scan)

- No direct mutation of `ade_ledger` state — all transitions go
  through `ade_ledger::rules::*`, the `block_validity` /
  `tx_validity` composers, or `mempool::ingress::mempool_ingress`
  (the Tier-1 wire-level chokepoint; direct `mempool::admit` calls
  from production source are CI-forbidden in N-E).
- No bypassing `ade_codec` to construct semantic types from raw bytes.
- (`ade_runtime` specifically) No dep on `ade_ledger`. No leakage of
  `redb` types. No second public `chaindb` path.
- (`ade_network::mux::transport`) No protocol logic.
- (`ade_network::session`) Composition glue only.
- (`ade_network::bin::capture_*`) Live-interop tools only.
- (`ade_runtime::consensus::genesis_parser`) No re-derivation of the
  bootstrap anchor outside `compute_anchor_hash`.
- (`ade_ledger::consensus_input_extract`) Pure-over-bytes.
- **(N-E live N2N operator-action session — the RED probe binary
  `live_tx_submission_session` + the RED half of CE-N-E-6)** The
  live socket loop that drives a real cardano-node peer MUST funnel
  its delivered tx-byte events through the GREEN
  `ade_core_interop::tx_submission` bridge — it MUST NOT carry a
  parallel admission path, MUST NOT call `admit` directly from
  production source, MUST NOT bypass `mempool_ingress`, and MUST NOT
  branch the verdict on whether the bytes arrived over N2N or N2C.
  The hermetic default mode + `--connect` live mode split MUST
  follow the `live_consensus_session` pattern (N-B). Evidence is
  captured into
  `docs/clusters/completed/PHASE4-N-E/CE-N-E-6_<date>.log` (the
  2026-05-25 capture is the first instance).
- **(Deferred RED operator-action surfaces — CE-NODE-N2C-LTX)** The
  live N2C UDS server + N2N bulk-tx inbound listener belong to the
  future node-binary cluster; their probe binaries (when authored)
  MUST live in `crates/ade_core_interop/src/bin/` following the
  same `live_<surface>_session` naming.
- (`ade_core_interop`) Live-interop driver only; library tests
  `#[ignore]`-gated. The N-E GREEN bridges in this crate are
  deterministic pure functions; the live socket loops are
  operator-action.

### Project-specific additions

- **No commits of credentials, hostnames, IPs, private keys** —
  enforced by `ci_check_no_secrets.sh`.
- **No `Phase 4 internal-mode mock network`** — Tier 1 surfaces must
  be exercised against real cardano-node peers.
- **No collapsing wire and canonical bytes** — dual-authority rule.
- **No Tier 5 surface without a stated rationale** — the GREEN
  per-peer canonicalizer is Tier-5 within `ade_ledger::mempool`; the
  Tier-5 rationale is "deterministic per-peer fairness for replay
  byte-identity" (DC-MEM-04).
- **No "we'll match it later" stubs on Tier 1 surfaces** — Tier 1
  closure is hard-gated. The B1 block verdict, the B2 tx verdict,
  the B2 mempool admission gate, the B3 full value-conservation
  accounting, the B4 Conway cert-state accumulation, the B5 Conway
  governance-cert accumulation, the ENACTMENT-COMMITTEE-WRITEBACK
  committee write-back, and the N-E Tier-1 wire-level mempool
  ingress are all Tier-1 surfaces. **Cross-cluster obligation
  deferrals (CE-NODE-N2C-LTX) are NOT "we'll match it later" stubs**
  — they are explicit cluster-doc-recorded deferrals of a CE's
  live-wire half (the mechanical half is fully landed and CI-green
  in the deferring cluster; only the live-server evidence is owed
  by the named successor cluster).

---

## Cross-references

- CODEMAP: `docs/ade-CODEMAP.md` — module-by-module authority table,
  upstream of this document. **Cross-reference check at this HEAD:**
  CODEMAP is being regenerated in parallel; the in-flight CODEMAP at
  this regen still pins pre-N-E-S6 HEAD `43fcc31` (per its header
  line). The S6 probe binary `live_tx_submission_session` is named
  in this SEAMS at exact path
  `crates/ade_core_interop/src/bin/live_tx_submission_session.rs`
  so the next CODEMAP regen picks it up mechanically as a new RED
  entry under `ade_core_interop` (sibling of `live_consensus_session`).
  CODEMAP's CI count is consistent with this SEAMS at 31 (N-E added
  the two ingress gates; S6 added no new CI). All other N-E S1..S5
  modules + the two new CI gates carry forward from the partial-close
  regen.
- Invariant registry: `docs/ade-invariant-registry.toml` — rule
  families incl. T / CN / DC / OP / RO. **N-E added:** `DC-MEM-03`
  (`enforced`, `ci_script = ci/ci_check_mempool_ingress_closure.sh`,
  `introduced_in = PHASE4-N-E`) and `DC-MEM-04` (`enforced`,
  `ci_script = ci/ci_check_mempool_ingress_replay.sh`,
  `introduced_in = PHASE4-N-E`); and appended `PHASE4-N-E` to
  `DC-MEM-01.strengthened_in` (now `["PHASE4-B2", "PHASE4-N-E"]`).
  S6 + full close added no new registry entries.
- Phase 4 cluster plan: `docs/active/phase_4_cluster_plan.md`.
- Tier doctrine: `docs/active/CE-79_gate_statement.md` and
  `docs/active/CE-79_tier5_addendum.md`.
- Cluster N-D slices (closed): `docs/clusters/completed/PHASE4-N-D/S-{33..37}.md`.
- Cluster N-A (closed): `docs/clusters/completed/PHASE4-N-A/cluster.md`
  + `S-A{1..10}.md`.
- Cluster N-B (closed): `docs/clusters/completed/PHASE4-N-B/cluster.md` +
  `S-B{1..10}.md` + `CE-N-B-6_PROCEDURE.md` + the captured
  `CE-N-B-6_<date>.log`.
- Cluster B1 (closed): `docs/clusters/completed/PHASE4-B1/cluster.md` +
  `B1-S{1..7}.md`.
- Cluster B2 (closed): `docs/clusters/completed/PHASE4-B2/cluster.md` +
  `B2-S{1..5}.md`.
- Cluster B3 (closed): `docs/clusters/completed/PHASE4-B3/cluster.md` +
  `B3-S{1..6}.md`.
- Cluster B4 (closed): `docs/clusters/completed/PHASE4-B4/cluster.md` +
  `B4-S1.md` (declares PHASE4-B5).
- Cluster B5 (closed): `docs/clusters/completed/PHASE4-B5/cluster.md` +
  `B5-S{2..5}.md` (declares OQ-3 / OQ-5).
- Cluster OQ5-CREDENTIAL-FIDELITY (closed).
- Cluster COMMITTEE-CRED-FIDELITY (closed).
- Cluster DREP-VOTE-FIDELITY (closed).
- Cluster ENACTMENT-COMMITTEE-FIDELITY (closed).
- Cluster ENACTMENT-COMMITTEE-WRITEBACK (closed): leaves
  `proposal_procedures` decode as the one declared non-goal
  governance-domain seam.
- **Cluster PHASE4-N-E (mechanically AND operationally closed at
  this HEAD; archived to `docs/clusters/completed/PHASE4-N-E/`)**:
  the cluster doc + slices `cluster.md, N-E-S{1..6}.md,
  CE-N-E-6_PROCEDURE.md, CE-N-E-7_PROCEDURE.md,
  CE-N-E-6_2026-05-25.log`. WIRES AND CLOSES the Tier-1 wire-level
  mempool ingress seam — `IngressEvent` + closed `IngressSource`
  2-variant + the BLUE chokepoint `mempool_ingress` (DC-MEM-03) +
  the GREEN replay harness (DC-MEM-04) + the GREEN per-peer
  canonicalizer + the two GREEN `ade_core_interop` bridges
  (`tx_submission`, `local_tx_submission`) + the N-E S6 RED operator
  probe binary `live_tx_submission_session` + the CE-N-E-6 live
  evidence log at outbound-client depth. STRENGTHENS DC-MEM-01
  (`strengthened_in += PHASE4-N-E`). Added two CI scripts in S1+S2
  (`ci_check_mempool_ingress_closure.sh`,
  `ci_check_mempool_ingress_replay.sh` — count `29 → 31`); S6 + full
  close added no new CI script. **INTRODUCES the cross-cluster
  obligation pattern** (CE-N-E-7 split: adapter-mechanical closed in
  S5; live-N2C-UDS-server deferred to `CE-NODE-N2C-LTX` in the future
  node-binary cluster; CE-N-E-6 bulk-tx half similarly deferred).
  **Declared non-goals carried to future clusters:** outbound tx
  propagation, mempool bounds / shedding (Tier-5), `proposal_procedures`
  tx-body decode, and (cross-cluster) `CE-NODE-N2C-LTX`.
- **Future obligation: `CE-NODE-N2C-LTX`** — the node-binary
  cluster's live N2C UDS server + N2N bulk-tx inbound listener; the
  procedure anchors are
  `docs/clusters/completed/PHASE4-N-E/CE-N-E-{6,7}_PROCEDURE.md`;
  the evidence artifact will land at the future cluster's directory
  as `CE-NODE-N2C-LTX_<date>.log`.
