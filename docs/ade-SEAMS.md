# Seams — Where New Work Can Attach (Ade)

> **Status:** Living architectural document. Regenerated; not hand-edited.
> Per-project instance of `~/.claude/methodology/templates/seams.md`.

> 11 crates, **32 CI checks** at HEAD (`928c2be`).
> Reads CODEMAP for the module list and TCB colors; reads the invariant
> registry (`docs/ade-invariant-registry.toml`) for rule IDs; reads the
> Phase 4 cluster plan (`docs/active/phase_4_cluster_plan.md`), the
> closed N-D / N-A / N-B / N-E / B1 / B2 / B3 / B4 / B5 cluster docs,
> the OQ5-CREDENTIAL-FIDELITY, COMMITTEE-CRED-FIDELITY, DREP-VOTE-FIDELITY,
> ENACTMENT-COMMITTEE-FIDELITY, ENACTMENT-COMMITTEE-WRITEBACK cluster
> docs, and the **just-closed and archived PROPOSAL-PROCEDURES-DECODE
> cluster doc** (`docs/clusters/completed/PROPOSAL-PROCEDURES-DECODE/cluster.md`
> + `PP-S1.md` + `PP-S2.md`).
>
> **This is the PROPOSAL-PROCEDURES-DECODE FULL CLOSE refresh
> (HEAD `928c2be`).** The previous SEAMS (HEAD `caa5ce8`) pinned the
> PHASE4-N-E full-close state and recorded `proposal_procedures`
> tx-body decode as the one declared-non-goal governance-domain seam
> carried forward. **Two commits land between that revision and this
> one** and close that seam:
>
> 1. **PP-S1 (commit `70bc85b`)** ships the BLUE closed-grammar
>    decoder `ade_codec::conway::governance::decode_proposal_procedures`,
>    the closed `ade_types::conway::governance::ProposalProcedure`
>    struct (4 fields — `deposit`, `return_addr`, `gov_action`,
>    `anchor`), the typed field at `ConwayTxBody.proposal_procedures:
>    Option<Vec<ProposalProcedure>>` (was `Option<Vec<u8>>`), the
>    typed encode/decode at body key 20, the new CI gate
>    `ci/ci_check_proposal_procedures_closed.sh` (5 mechanical
>    guards), and the new derived-Cardano registry rule `DC-LEDGER-11`
>    (`enforced`, `introduced_in = PROPOSAL-PROCEDURES-DECODE`). CI
>    script count `31 → 32`. Closes CE-PP-1, CE-PP-2, CE-PP-3, CE-PP-4,
>    CE-PP-5.
> 2. **PP-S2 (commit `928c2be`)** ships the GREEN canonical synthetic
>    corpus + replay harness at
>    `crates/ade_testkit/src/governance/proposal_procedures_replay.rs`,
>    decoding + re-encoding every fixture byte-identically and covering
>    all 7 `GovAction` variants including the discriminated
>    `UpdateCommittee` case. Extends `DC-LEDGER-11.tests` with 4 harness
>    test names. Closes CE-PP-6.
>
> **THE KEY FULL-CLOSE DELTA — the §1 candidates row for
> `proposal_procedures` tx-body decode FLIPS from "candidate /
> declared non-goal" to "wired & closed".** The decoder reuses the
> existing closed 7-variant `GovAction` enum (preserving the
> `UpdateCommittee` structured form added in
> ENACTMENT-COMMITTEE-WRITEBACK) and the existing opaque `Anchor`
> struct. The new rule `DC-LEDGER-11` cross-refs `DC-LEDGER-10`
> bidirectionally — DC-LEDGER-10 (UpdateCommittee discriminant) is
> CONSUMED here unchanged; DC-LEDGER-11 (proposal_procedures closure)
> closes the authoritative entry path that surfaces UpdateCommittee
> on the wire, and `DC-LEDGER-10.cross_ref` now includes `DC-LEDGER-11`.
>
> **OQ scope locks (carried into SEAMS as new §1 candidates).** The
> cluster's locked OQ resolutions name four deliberate future
> strengthenings that this cluster does NOT close; they are recorded
> in `DC-LEDGER-11.open_obligation` and surface in §1 as new
> candidate seams:
>
> - **`voting_procedures` (tx body key 19)** — same opaque-bytes
>   shape, same pressure, natural sibling cluster (OQ-1).
> - **`ParameterChange.update`** — full pparams update sub-grammar;
>   separate large cluster (OQ-2).
> - **`NewConstitution.raw`** — small but separable; bundleable with
>   the voting-procedures cluster or shipped alone (OQ-3).
> - **Typed `RewardAccount` for `proposal_procedure.return_addr`** —
>   typed reward-account fidelity decision; would also retype
>   `TreasuryWithdrawals.withdrawals` in one move (OQ-4).
>
> Counts at this refresh: **+1 closed BLUE struct** (`ProposalProcedure`),
> **+1 closed BLUE entry point** (`decode_proposal_procedures` —
> closed grammar, not a registry, but the single sanctioned production
> decoder), **+1 frozen wire contract** (the closed
> `proposal_procedures` grammar at Conway tx-body key 20), **+1 CI
> script** (count `31 → 32`), **+1 derived-Cardano registry rule**
> (`DC-LEDGER-11`), **+1 closed-grammar audit item** (PROPOSAL-PROCEDURES-DECODE
> entry), **+4 candidate seams** (the OQ-1/2/3/4 deferrals as
> separable future seams), **+1 archived cluster directory**
> (`docs/clusters/completed/PROPOSAL-PROCEDURES-DECODE/`). **Zero
> new operator-action probe binaries / live-evidence logs** — this
> cluster's evidence is fully mechanical (the 17 PP-S1 tests + the
> PP-S2 harness tests + the CI gate); there is no live-wire half.
>
> **N-E summary (carried unchanged).** The PHASE4-N-E close shipped
> the single BLUE chokepoint `mempool_ingress`, the closed 2-variant
> `IngressSource { N2N, N2C }`, the closed `IngressEvent
> { source, tx_bytes }` struct, the two GREEN bridges
> (`ade_core_interop::tx_submission` / `local_tx_submission`), the
> GREEN per-peer canonicalizer
> (`ade_ledger::mempool::canonicalize::canonicalize_peer_streams`),
> the GREEN replay harness, the operator-action probe binary
> `live_tx_submission_session`, the captured CE-N-E-6 log, the two
> N-E CI gates (`ci_check_mempool_ingress_closure.sh`,
> `ci_check_mempool_ingress_replay.sh`), the cross-cluster obligation
> pattern (`CE-NODE-N2C-LTX`), and `DC-MEM-03` / `DC-MEM-04` /
> `DC-MEM-01.strengthened_in += PHASE4-N-E`. **None of this changes
> at this HEAD.**
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

**PROPOSAL-PROCEDURES-DECODE is fully closed at this HEAD.** The prior
revision (HEAD `caa5ce8`) carried `proposal_procedures` tx-body decode
as the one declared-non-goal governance-domain seam. PP-S1 + PP-S2
close it: typed `ConwayTxBody.proposal_procedures:
Option<Vec<ProposalProcedure>>`, single sanctioned closed-grammar
decoder, mechanical CI gate, byte-identical round-trip across all 7
`GovAction` variants including discriminated `UpdateCommittee`. The
four OQ-locked nested opacities (voting_procedures, ParameterChange.update,
NewConstitution.raw, typed RewardAccount) are recorded as separable
future seams.

**PHASE4-N-E (Tier 1 wire-level mempool ingress) remains fully closed.**
Code + CE-N-E-6 live N2N evidence captured + cluster archived. CE-N-E-7
+ bulk-tx halves deferred to CE-NODE-N2C-LTX.

**PHASE4-B3 (Full Conway tx value-conservation accounting) is closed.**
It added the closed Conway certificate CDDL grammar, the closed
withdrawals map grammar, the closed `ConwayCert` / `CertDisposition` /
`DepositEffect` / `CoinSource` sum types, the canonical-only
deposit-parameter surface, the closed total cert classifier, and the
full preservation-of-value equation with the frozen §9.1 reject
precedence.

**PHASE4-B4 (Conway certificate-state accumulation, fail-closed) is
closed.** Governance-affecting Conway certs are decoded fully and
owner-tagged to `ConwayGovState` via `OwnerTaggedEffect`, routed OUT
of B4's mutation scope.

**PHASE4-B5 (Conway governance-certificate accumulation) is closed.**
`ade_ledger::gov_cert::apply_conway_gov_cert` is a total,
compiler-exhaustive dispatch over `ConwayCert` with no `_ =>` arm,
that folds vote-delegation / committee / DRep effects into
`ConwayGovState`.

**OQ5-CREDENTIAL-FIDELITY → COMMITTEE-CRED-FIDELITY → DREP-VOTE-FIDELITY
→ ENACTMENT-COMMITTEE-FIDELITY → ENACTMENT-COMMITTEE-WRITEBACK** closed
the credential discriminant chain across the gov-state keys, the
committee/DRep votes, the enactment effects, and the live
`UpdateCommittee` write-back. **The PP cluster CONSUMES the
DC-LEDGER-10 closed `StakeCredential` form** through the
`UpdateCommittee` arm of the new `decode_gov_action` decoder — the
bidirectional cross-ref `DC-LEDGER-10 ↔ DC-LEDGER-11` is now recorded
in the registry.

---

## 1. Surface Reduction Rules

> External inputs reduce to canonical form before entering authoritative
> pipelines. At HEAD there are **seven** fully-wired *external* ingress
> surfaces (block bytes, Plutus script bytes, snapshot bytes, Ouroboros
> mux frames, genesis JSON bundles, chain-selector stream inputs, and
> the N-E wire-level mempool ingress), plus the three **internal
> composition roots** (`block_validity` from B1, `tx_validity` from B2,
> and the BLUE chokepoint `mempool_ingress` from N-E S1), the
> **mempool admission gate** (`mempool::admit`, a Tier-1 surface over
> `tx_validity`), the **consensus-input extraction surface** (snapshot
> `state` CBOR tail-scan from B1), **and — newly closed at this HEAD —
> the `proposal_procedures` sub-grammar entry point within the Conway
> tx-body decode**.
>
> **At this HEAD the `proposal_procedures` sub-grammar reader is
> wired into the existing Conway-tx-body codec surface as a closed
> entry point.** The §1 candidate row for `proposal_procedures`
> tx-body decode flips from "candidate / declared non-goal" to
> "wired & closed". The decoder is purely mechanical (no operator
> action, no live-wire half) — the evidence is the 17 PP-S1 tests
> + the PP-S2 canonical synthetic corpus harness + the
> `ci_check_proposal_procedures_closed.sh` gate.

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
(outbound-client probe; the bulk-tx half joins CE-NODE-N2C-LTX in
the future node-binary cluster). The live N2C UDS evidence
(CE-N-E-7) is **DEFERRED** to the same future cluster
(CE-NODE-N2C-LTX) — see "Cross-cluster obligation pattern" in §5.

### Surface: Conway tx-body `proposal_procedures` sub-grammar (closed entry point — NEW in PROPOSAL-PROCEDURES-DECODE)

```
Surface: The CBOR item bytes at Conway tx-body key 20 (proposal_procedures)
         captured by the body decoder via skip_item, decoded into a typed
         vector of ProposalProcedure
Reduces to: Vec<ProposalProcedure>
              where ProposalProcedure = {
                deposit: Coin,
                return_addr: Vec<u8>,        // raw reward-account bytes (OQ-4)
                gov_action: GovAction,       // closed 7-variant enum (reused)
                anchor: Anchor,              // opaque { raw: Vec<u8> } (reused)
              }
Pipeline (fixed step ordering — no reorder, no shortcut):
  1. BLUE Conway tx-body decoder
       ade_codec::conway::tx::decode_conway_tx_body
       — captures the CBOR item bytes at key 20 via skip_item.
  2. BLUE closed-grammar sub-decoder (single sanctioned entry point)
       ade_codec::conway::governance::decode_proposal_procedures(
         &[u8]
       ) -> Result<Vec<ProposalProcedure>, CodecError>
       — rejects unknown gov_action tag, empty set (CIP-1694 requires
         non-empty), trailing garbage, truncated procedure, invalid
         stake credential in UpdateCommittee. No silent-skip arm.
  3. BLUE per-procedure decoder
       decode_proposal_procedure → reads [deposit, return_addr, gov_action, anchor]
       — gov_action via decode_gov_action (closed 7-variant dispatch
         over GovAction; UpdateCommittee preserves DC-LEDGER-10
         discriminated StakeCredential).
  4. BLUE re-encoder (round-trip authority)
       ade_codec::conway::governance::encode_proposal_procedures
       — byte-identical PreservedCbor round-trip for every well-formed
         Conway tx body (DC-LEDGER-11, CE-PP-2, CE-PP-6).
Cross-surface state sharing: none — pure function over the captured
  key-20 bytes; era-gate at pre-Conway still enforced by
  ade_ledger::error::ProposalProceduresInPreConway (CE-PP-4).
```

**Rule.** `decode_proposal_procedures` is the **single sanctioned
production decoder** for the Conway tx-body `proposal_procedures`
sub-grammar. The body codec at key 20 calls the typed decoder +
encoder; the prior `Option<Vec<u8>>` opaque pass-through is
forbidden by `ci_check_proposal_procedures_closed.sh` guard 2.
Construction of `ProposalProcedure` outside the decoder, the
testkit fixture builders, and `crates/*/tests/` / `#[cfg(test)]`
blocks is forbidden by guard 5. The closed grammar rejects unknown
`gov_action` tags, empty sets, trailing garbage, truncated
procedures, and invalid stake credentials in `UpdateCommittee`.
The DC-LEDGER-10 discriminant is **preserved unchanged** through
the `UpdateCommittee` arm (the test
`update_committee_keeps_stake_credential_discriminant` certifies).
**Frozen sub-shape** (see §4): `Anchor` stays opaque, `return_addr`
stays `Vec<u8>` (typed `RewardAccount` is OQ-4 future fidelity),
`gov_action` reuses the existing closed 7-variant `GovAction` enum
unchanged. **New work** that extends governance-domain decoding
attaches as a parallel closed entry point (e.g. a future
`decode_voting_procedures` for tx-body key 19) — not by reopening
this decoder, not by adding catch-all arms, not by adding nested
opacity to `ProposalProcedure`.

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
into `tx_validity` is `mempool_ingress` → `admit` → `tx_validity`;
the composer itself is untouched. **PP note:** the typed
`ConwayTxBody.proposal_procedures` field is now in scope for any
future tx-validity precondition that needs to gate on proposal
content (no such precondition is added in this cluster; the field
is decoded but not consulted by `tx_validity` at this HEAD).

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
into `mempool::admit` via the `mempool_ingress` chokepoint. The
bridge is GREEN (`ade_core_interop::tx_submission` +
`ade_core_interop::local_tx_submission`) + the operator-action live
session (RED bearer + RED transport — the N-E S6 probe binary
`live_tx_submission_session` is the executable instance of the N2N
half).

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

### Candidates — surfaces not yet wired (Phase 4 N-C, N-F, B+ residuals; PP open obligations)

The following surfaces are named in the Phase 4 plan / B+ planning
/ the PP cluster's open-obligation set but have no source today.
They are listed so future slice docs can attach without reinventing
the reduction step. **Each is a candidate seam pending confirmation
at cluster entry.**

- **B3 closed the prior revision's "deposit/refund preservation-of-value"
  candidate** — removed.
- **B5 WIRED AND CLOSED the prior revision's B4 confirmed extension
  point** — the owner-tagged `ConwayGovState` effect channel.
- **OQ5 / COMMITTEE / DREP / ENACTMENT-COMMITTEE-FIDELITY /
  ENACTMENT-COMMITTEE-WRITEBACK** closed the credential discriminant
  chain and the live `UpdateCommittee` write-back.
- **N-E WIRED AND CLOSED the N2N/N2C tx-submission ingest** into
  `mempool::admit`.
- **PROPOSAL-PROCEDURES-DECODE WIRED AND CLOSED the prior revision's
  one remaining governance-domain seam** — the `proposal_procedures`
  tx-body decode. The four OQ-locked nested opacities (voting_procedures,
  ParameterChange.update, NewConstitution.raw, typed RewardAccount)
  are recorded as new separable candidate seams below.

| Cluster | Surface | Expected reduction target | Expected chokepoint | Confidence |
|---------|---------|---------------------------|---------------------|------------|
| **PROPOSAL-PROCEDURES-DECODE** *(FULLY CLOSED at this HEAD — mechanical close, no live-wire half)* | **Conway tx-body `proposal_procedures` (key 20) sub-grammar decode** — the RED-prior opaque-bytes field that carried governance proposal payloads as `Option<Vec<u8>>` | `Option<Vec<ProposalProcedure>>` on `ConwayTxBody`, populated by `decode_proposal_procedures` over the key-20 CBOR item bytes; re-encoded byte-identically by `encode_proposal_procedures` | **DONE:** `ade_codec::conway::governance::{decode_proposal_procedures, encode_proposal_procedures, decode_proposal_procedure, decode_gov_action, decode_anchor, decode_gov_action_id_opt, decode_stake_credential, decode_unit_interval, decode_cold_credential_set, decode_cold_credential_epoch_map, decode_hash28_opt}` + closed type `ade_types::conway::governance::ProposalProcedure` + body codec key-20 typed path in `ade_codec::conway::tx`. Gated by `ci_check_proposal_procedures_closed.sh` (DC-LEDGER-11, 5 mechanical guards). Tests: 17 in PP-S1 + 4 in PP-S2 (canonical synthetic corpus harness at `ade_testkit::governance::proposal_procedures_replay`). DC-LEDGER-10 discriminant preserved through the `UpdateCommittee` arm. | **wired & closed in PROPOSAL-PROCEDURES-DECODE** |
| **PHASE4-N-E** *(FULLY CLOSED — code + live evidence at outbound-client depth; bulk-tx live half joins CE-NODE-N2C-LTX)* | **N2N/N2C tx-submission → mempool ingress (Tier-1 wire-level)** | `mempool_ingress(&MempoolState, &IngressEvent) -> (MempoolState, AdmitOutcome)`, where `IngressEvent { source: IngressSource::{N2N,N2C}, tx_bytes }` flows verbatim into `admit` | **DONE:** see N-E entry below; carried unchanged from the prior revision. | **fully closed in N-E** |
| **PHASE4-B5** *(WIRED + CLOSED)* | Owner-tagged Conway governance-cert effects → `ConwayGovState` | An applied `ConwayGovState'` via a deterministic fold | **DONE:** `ade_ledger::gov_cert::apply_conway_gov_cert`. Gated by `ci_check_gov_cert_accumulation_closed.sh` (DC-LEDGER-09) | **wired & closed in B5** |
| **OQ-5** *(WIRED + CLOSED in OQ5-CREDENTIAL-FIDELITY)* | Credential key/script discriminant | A discriminant-preserving credential representation | **DONE.** Gated by `ci_check_credential_discriminant_closed.sh` (DC-LEDGER-10) | **wired & closed in OQ5** |
| **Committee member / committee-vote discrimination** *(WIRED + CLOSED in COMMITTEE-CRED-FIDELITY)* | `committee` + `committee_votes` were bare `Hash28` at OQ5 | discriminant-faithful committee | **DONE.** Gated by the EXTENDED `ci_check_credential_discriminant_closed.sh` | **wired & closed in COMMITTEE-CRED-FIDELITY** |
| **DRep-vote discrimination** *(WIRED + CLOSED in DREP-VOTE-FIDELITY)* | `drep_votes` key/script OR-fallback | discriminant-faithful DRep tally | **DONE.** Gated by the EXTENDED gate | **wired & closed in DREP-VOTE-FIDELITY** |
| **`EnactmentEffects.committee_changes`** *(WIRED + CLOSED in ENACTMENT-COMMITTEE-FIDELITY)* | Bare-`Hash28` committee-change set | discriminant-faithful committee-change set | **DONE.** Gated by check 6 | **wired & closed in ENACTMENT-COMMITTEE-FIDELITY** |
| **`UpdateCommittee` / `NoConfidence` enactment LOGIC** *(WIRED + CLOSED in ENACTMENT-COMMITTEE-WRITEBACK)* | Prior `enact_proposals` was `let _ = raw;` | `ConwayGovState'` with committee + quorum updated | **DONE:** structured `GovAction::UpdateCommittee`, `apply_committee_enactment`, `rules.rs:1224`. Gated by EXTENDED checks 6 + 7 | **wired & closed in ENACTMENT-COMMITTEE-WRITEBACK** |
| OQ-3 *(separable follow-up — NOT an open seam now)* | **GOVCERT committee-membership tx-validity gate** | A `TxValidityVerdict::Invalid` on a committee cert with no matching elected member | A new BLUE tx-validity precondition check | candidate (declared separable in B5 cluster doc) |
| **PP OQ-1 (NEW separable seam — declared open obligation in `DC-LEDGER-11.open_obligation`)** | **`voting_procedures` (Conway tx-body key 19) closed decode** — same opaque-bytes shape as the now-closed key-20 field, deliberately deferred per OQ-1 | A typed `Option<BTreeMap<Voter, BTreeMap<GovActionId, VotingProcedure>>>` (or equivalent CIP-1694 shape) on `ConwayTxBody.voting_procedures` | A parallel BLUE closed-grammar entry point in `ade_codec::conway::governance::decode_voting_procedures` (sibling of `decode_proposal_procedures`); a new closed `VotingProcedure` struct in `ade_types::conway::governance`; a sibling CI gate or an extension of `ci_check_proposal_procedures_closed.sh`; a future `strengthened_in += <cluster>` on DC-LEDGER-11 OR a new DC-LEDGER-12 (registry's choice when planned) | candidate (natural sibling cluster to PROPOSAL-PROCEDURES-DECODE; declared `open_obligation` in DC-LEDGER-11) |
| **PP OQ-2 (NEW separable seam — declared open obligation)** | **`ParameterChange.update` nested decode** — currently the inner `update` payload stays opaque `Vec<u8>` inside the now-typed `GovAction::ParameterChange` variant | A typed `ProtocolParameterUpdate` (full Conway pparams update sub-grammar) | A new BLUE closed-grammar decoder for the pparams update sub-grammar inside `ade_codec::conway::governance` (or a new module); a future `strengthened_in += <cluster>` on DC-LEDGER-11 if/when closed | candidate (separate large cluster; declared `open_obligation` in DC-LEDGER-11) |
| **PP OQ-3 (NEW separable seam — declared open obligation)** | **`NewConstitution.raw` nested decode** — currently the inner constitution body stays opaque inside `GovAction::NewConstitution` | A typed `Constitution { anchor, script: Option<Hash28> }` (CIP-1694 shape) | A new BLUE closed-grammar decoder inside `ade_codec::conway::governance`; small but separable; bundleable with the voting-procedures cluster or shipped alone | candidate (declared `open_obligation` in DC-LEDGER-11) |
| **PP OQ-4 (NEW separable seam — declared open obligation)** | **Typed `RewardAccount` for `proposal_procedure.return_addr`** — currently the `return_addr` field stays raw `Vec<u8>` to keep this cluster's proof obligation narrow | A typed `RewardAccount` (header byte + `StakeCredential` discriminant) | Lift `return_addr: Vec<u8>` → `return_addr: RewardAccount` on `ProposalProcedure`; would also strengthen `TreasuryWithdrawals.withdrawals` element type in one move; closes a shared fidelity decision across both surfaces | candidate (typed reward-account fidelity; declared `open_obligation` in DC-LEDGER-11; would also touch B3 withdrawals shape) |
| OQ5+ *(declared non-goal — NOT an open seam now)* | **Withdrawal / required-signer / address credential discriminant** | A discriminant-faithful credential threaded through these surfaces | extend the closed `StakeCredential` discriminant | candidate |
| OQ5+ *(declared non-goal — NOT an open seam now)* | **`Hash28`-keyed stake-distribution snapshot** | A discriminant-faithful snapshot key | re-key on `StakeCredential` | candidate |
| OQ5+ *(declared non-goal — NOT an open seam now)* | **Byron credential surface** | discriminant-faithful Byron credentials (if ever required) | a SEPARABLE Byron-era follow-up | candidate |
| **CE-NODE-N2C-LTX (cross-cluster obligation introduced in N-E S5; carried to the future node-binary cluster)** | **Live N2C UDS server + N2N bulk-tx inbound listener** | The deferred half of CE-N-E-7 + the deferred half of CE-N-E-6 — both reduce through the same BLUE `mempool_ingress` chokepoint that's already wired | The future node-binary cluster ships the live socket loops; the GREEN bridges + canonicalizer + chokepoint + CI gates are already in place at this HEAD; only the `_<date>.log` evidence file is owed | **deferred cross-cluster obligation (NOT an open seam in N-E)** |
| **N-E+ (declared non-goal in the N-E cluster doc; separable future seam, NOT an open seam now)** | **Outbound tx propagation** — Ade serving txs to peers via tx-submission2 | An outbound `TxSubmission2Message` stream emitted from the mempool's admitted set | A separate authority surface — a new BLUE/GREEN outbound bridge in `ade_core_interop`; explicitly declared OUT-OF-SCOPE for N-E | candidate (declared non-goal in N-E) |
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
cross-cluster obligation is named. **PROPOSAL-PROCEDURES-DECODE adds
NO new operator-action entries** — its evidence is fully mechanical
(decoder unit tests + canonical synthetic corpus harness + CI gate).
A future cluster that strengthens this surface against real-chain
corpora (declared as a possible PP-S2 extension per OQ-5) would
fold its evidence into the existing cluster archive or its own
successor cluster directory.

| Procedure | Evidence-log artifact | Status at HEAD | What it asserts | TCB |
|-----------|----------------------|----------------|------------------|-----|
| `docs/clusters/completed/PHASE4-N-B/CE-N-B-6_PROCEDURE.md` | `docs/clusters/completed/PHASE4-N-B/CE-N-B-6_<date>.log` | **CAPTURED** (carried from N-B close) | Real cardano-node N-B follow-mode tip agreement | RED operator action |
| `docs/clusters/completed/PHASE4-N-E/CE-N-E-6_PROCEDURE.md` | `docs/clusters/completed/PHASE4-N-E/CE-N-E-6_2026-05-25.log` | **CAPTURED** (carried from N-E close) | Outbound-client probe against a real preprod N2N relay: handshake v15, tx-submission2 codec round-trip, BLUE `tx_submission2_transition` state-machine driven correctly under live traffic, accumulator records `frames_received=1 requests_ids=1 tx_bytes=0`. Bulk-tx half deferred to CE-NODE-N2C-LTX. | RED operator action |
| `docs/clusters/completed/PHASE4-N-E/CE-N-E-7_PROCEDURE.md` | (deferred) `CE-NODE-N2C-LTX_<date>.log` in the future node-binary cluster | **DEFERRED to CE-NODE-N2C-LTX** | Real `cardano-cli transaction submit` to Ade over the N2C UDS; verdict matches N2N submission of the same bytes. **Adapter-mechanical half closed in N-E S5.** | RED operator action (deferred) |

**Operator-action probe binaries (RED — `ade_core_interop::bin::*`).**
The mechanical half of an operator-action evidence pattern is an
`#[ignore]`-gated closure-gate test + a binary that the operator
invokes to capture the live log. The binary is RED (uses `tokio`,
real sockets) and lives in `crates/ade_core_interop/src/bin/`. At
this HEAD there are **two** such binaries:

| Binary | Slice | Live-evidence target | Status |
|--------|-------|----------------------|--------|
| `live_consensus_session` (PHASE4-N-B) | N-B | CE-N-B-6 (live chain-sync follow-mode tip agreement) | captured |
| `live_tx_submission_session` (PHASE4-N-E S6) | N-E S6 | CE-N-E-6 (live N2N tx-submission2 outbound-client probe) | captured |

**Pattern.** Hermetic default mode (codec loopback / readiness probe
that runs in CI without network access — gated `#[ignore]`); plus a
`--connect <peer>` live pass that the operator runs against a real
cardano-node peer. The binary's evidence log is committed alongside
the `_PROCEDURE.md` in the cluster directory.

**These are evidence-log patterns, not BLUE seams.** They do not move
or wrap any chokepoint; they are the on-the-wire half of an existing
mechanical surface. **PROPOSAL-PROCEDURES-DECODE does not add a new
probe binary** — the cluster's evidence is fully mechanical.

User confirmation needed for each candidate at cluster entry. **The
most load-bearing remaining candidates for the bounty** are the
**Conway block-body vkey-witness closure** (the carried B2 gap), the
**forge / header→body bridge**, **CE-NODE-N2C-LTX** (the deferred
live N2C UDS server + N2N bulk-tx inbound listener), and the four
**PROPOSAL-PROCEDURES-DECODE open obligations** (voting_procedures
closure being the natural sibling).

---

## 2. Data-Only vs. Authoritative Layers

Ade has **fourteen** authoritative domains. For each, a single BLUE
chokepoint holds enforcement authority; tooling layers (when they
exist) live in GREEN (`ade_testkit`, `ade_core_interop` bridges) or
RED (`ade_runtime`, `ade_network::mux::transport`,
`ade_network::session`). **PROPOSAL-PROCEDURES-DECODE added one
domain — the closed `proposal_procedures` sub-grammar authority — a
new BLUE entry point `decode_proposal_procedures` sitting inside
the existing Conway tx-body codec surface, with a closed
`ProposalProcedure` struct as the typed reduction target and a
GREEN canonical synthetic corpus + replay harness as the
PreservedCbor round-trip witness.** N-E added the prior new domain
(mempool ingress). Prior cluster narratives are preserved unchanged
below.

### Conway tx-body `proposal_procedures` sub-grammar authority (NEW in PROPOSAL-PROCEDURES-DECODE)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Data-only — closed sub-grammar decoder** | `ade_codec::conway::governance::{decode_proposal_procedures, decode_proposal_procedure, decode_gov_action, decode_anchor, decode_gov_action_id_opt, decode_stake_credential, decode_unit_interval, decode_cold_credential_set, decode_cold_credential_epoch_map, decode_hash28_opt}` | BLUE | Closed CDDL sub-grammar over `proposal_procedure = [coin, reward_account, gov_action, anchor]`. No catch-all accept. Rejects unknown `gov_action` tag, empty set, trailing garbage, truncated procedure, invalid stake credential. |
| **Data-only — closed sub-grammar re-encoder** | `ade_codec::conway::governance::{encode_proposal_procedures, encode_proposal_procedure, encode_gov_action, encode_anchor, ...}` | BLUE | Byte-identical round-trip authority. PreservedCbor for every well-formed Conway tx body (CE-PP-2, CE-PP-6). |
| **Closed type** | `ade_types::conway::governance::ProposalProcedure` | BLUE | Closed 4-field struct `{ deposit: Coin, return_addr: Vec<u8>, gov_action: GovAction, anchor: Anchor }`. Construction outside `decode_proposal_procedures` and the testkit fixture builders is forbidden on production paths by CI guard 5. |
| **Authoritative field shape** | `ade_types::conway::tx::ConwayTxBody.proposal_procedures` | BLUE | `Option<Vec<ProposalProcedure>>` (was `Option<Vec<u8>>` prior to PP-S1). The body codec at key 20 calls the typed decoder + encoder; the opaque-bytes form is CI-forbidden by guard 2. |
| **Era-gate (carried — pre-Conway reject path)** | `ade_ledger::error::ProposalProceduresInPreConway` | BLUE | Pre-Conway era rejects key 20. Unchanged by PP-S1 (CE-PP-4). |
| **GREEN canonical-synthetic corpus + replay harness (PP-S2)** | `ade_testkit::governance::proposal_procedures_replay` | GREEN | Synthesizes well-formed Conway tx-body fixtures covering all 7 `GovAction` variants + the DC-LEDGER-10 `UpdateCommittee` discriminant case; asserts decode + encode is byte-identical (CE-PP-6). Deterministic; no I/O; no clocks. |
| **CI gate (PP-S1)** | `ci/ci_check_proposal_procedures_closed.sh` | CI | 5 mechanical guards: (1) `ProposalProcedure` struct defined with 4 fields; (2) `ConwayTxBody.proposal_procedures` is `Option<Vec<ProposalProcedure>>` (not opaque bytes); (3) `ade_codec::conway::governance` exports `decode_proposal_procedures` + `encode_proposal_procedures`; (4) body codec at key 20 calls the typed decoder + encoder; (5) no `ProposalProcedure { ... }` struct-literal construction outside sanctioned sites (the decoder file, `ade_testkit`, `crates/*/tests/`, inline `#[cfg(test)]` blocks). Wired into `DC-LEDGER-11` (`enforced`). |

**Rule.** This domain has **one BLUE closed sub-grammar decoder**
(`decode_proposal_procedures`), **one BLUE re-encoder**
(`encode_proposal_procedures` — the round-trip authority for
PreservedCbor), **one closed 4-field domain type**
(`ProposalProcedure`), and **one GREEN canonical synthetic corpus +
replay harness** (`proposal_procedures_replay`). **THE KEY SEAMS:**

1. **`decode_proposal_procedures` is the single sanctioned
   production decoder** for the Conway tx-body `proposal_procedures`
   sub-grammar (CI-enforced — guards 3 + 4).
2. **`ConwayTxBody.proposal_procedures` is typed end-to-end —
   `Option<Vec<ProposalProcedure>>`** — reverting to the opaque
   `Option<Vec<u8>>` form is CI-forbidden by guard 2.
3. **The closed sub-grammar rejects** unknown `gov_action` tag,
   empty set (CIP-1694 requires non-empty), trailing garbage,
   truncated procedure, and invalid stake credential in
   `UpdateCommittee`. No silent-skip arm.
4. **DC-LEDGER-10 discriminant is preserved unchanged** through
   the `UpdateCommittee` arm of `decode_gov_action` — the test
   `update_committee_keeps_stake_credential_discriminant` is the
   in-vivo certifier; `KeyHash(h)` and `ScriptHash(h)` of equal
   28 bytes remain distinct through the round-trip.
5. **PreservedCbor round-trip is byte-identical** for every
   well-formed Conway tx body (CE-PP-2 — synthetic, CE-PP-6 —
   canonical corpus harness).

**New work** that adds a governance-domain decoder attaches as a
**parallel closed entry point** (e.g. a future
`decode_voting_procedures` for tx-body key 19) — not by reopening
this decoder, not by adding catch-all arms, not by adding nested
opacity to `ProposalProcedure`. Strengthening the four
declared-non-goal nested opacities (voting_procedures,
ParameterChange.update, NewConstitution.raw, typed RewardAccount)
attaches by either: (a) appending the new cluster to
`DC-LEDGER-11.strengthened_in` if the strengthening is in-place
(typed RewardAccount qualifies), or (b) introducing a new
DC-LEDGER-1X rule with bidirectional cross-ref to DC-LEDGER-11 if
the strengthening is a parallel surface (voting_procedures
qualifies).

**Declared non-goals carried from the cluster doc (open obligations
in DC-LEDGER-11):** `voting_procedures` decode (OQ-1),
`ParameterChange.update` nested decode (OQ-2), `NewConstitution.raw`
nested decode (OQ-3), typed `RewardAccount` for `return_addr` (OQ-4),
GREEN snapshot-loader changes (different path), Plutus phase-2
validation of proposal procedures, mempool / propagation effects of
proposals, Tier-5 governance surfaces.

### Mempool ingress — the Tier-1 wire-level / per-peer canonicalizer / `mempool_ingress` boundary (NEW in N-E)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **RED wire transport** | `ade_network::mux::transport` (existing) | RED | Bears bytes off a TCP / UDS socket; no parsing. Untouched by N-E. |
| **BLUE wire grammar (N-A; carried)** | `ade_network::tx_submission::{codec, transition}` (N2N) + `ade_network::n2c::local_tx_submission::{codec, transition}` (N2C) | BLUE | Closed mini-protocol codecs + state machines; emit `InventoryEvent { TxsDelivered { tx_bytes: Vec<Vec<u8>> }, … }` (N2N) or `LocalTxSubmissionEvent { TxSubmitted { tx_bytes }, … }` (N2C). |
| **GREEN bridge — N2N (N-E S4)** | `ade_core_interop::tx_submission::{event_to_ingress, PeerAccumulator, ingest_n2n_events}` | GREEN | Per-peer `InventoryEvent` accumulator. Pure; no I/O; no clocks. |
| **GREEN bridge — N2C (N-E S5)** | `ade_core_interop::local_tx_submission::{local_event_to_ingress, ClientAccumulator, ingest_n2c_events}` | GREEN | Per-client `LocalTxSubmissionEvent` accumulator. Pure; no I/O; no clocks. |
| **GREEN per-peer canonicalizer (N-E S3)** | `ade_ledger::mempool::canonicalize::{canonicalize_peer_streams, PeerId, PeerSubmissionQueue}` | GREEN | Deterministic round-robin canonicalization of multi-peer queues. The load-bearing GREEN fairness contract. |
| **BLUE chokepoint (N-E S1)** | `ade_ledger::mempool::ingress::{IngressSource, IngressEvent, mempool_ingress}` | BLUE | The single sanctioned production path into `admit` from non-test code. DC-MEM-03 (`enforced`). |
| **BLUE admission gate (B2 — carried; unchanged)** | `ade_ledger::mempool::admit::admit` | BLUE | A tx is admitted iff `tx_validity(accumulating, tx)` is `Valid`. No false accept (DC-MEM-01, `strengthened_in += PHASE4-N-E`). |
| **GREEN replay harness (N-E S2)** | `ade_testkit::mempool::{ingress_replay::{wrap_as_ingress, b_track_corpus_as_ingress, replay_ingress_trace, BTrackCase, ExpectedOutcome}}` | GREEN | Single-step fold over `mempool_ingress` (CI-enforced). Byte-identical traces (DC-MEM-04). |
| **CI gates (N-E)** | `ci/ci_check_mempool_ingress_closure.sh` + `ci/ci_check_mempool_ingress_replay.sh` | CI | DC-MEM-03 + DC-MEM-04. |
| **Operator-action evidence (N-E)** | `docs/clusters/completed/PHASE4-N-E/{CE-N-E-6_PROCEDURE.md, CE-N-E-7_PROCEDURE.md, CE-N-E-6_2026-05-25.log}` + `live_tx_submission_session` (S6 probe binary) | RED operator action | CE-N-E-6 **captured**; CE-N-E-7 deferred to CE-NODE-N2C-LTX. |

**Rule.** This domain has two GREEN bridge layers, one GREEN
canonicalizer, and one BLUE chokepoint. Source-invariance
(`IngressSource` is metadata only), verbatim tx-bytes flow, single
sanctioned production caller of `admit`, and per-peer round-robin
canonicalization are the load-bearing properties (see N-E close
narrative for full detail; carried unchanged from the prior
revision).

**Declared non-goals carried from the cluster doc:** outbound tx
propagation, mempool bounds / shedding policy, the
`proposal_procedures` tx-body decode (**now closed in
PROPOSAL-PROCEDURES-DECODE**).

**Deferred cross-cluster obligation:** `CE-NODE-N2C-LTX` (live N2C
UDS server + N2N bulk-tx inbound listener).

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

### Credential discriminant fidelity — the closed credential surface (NEW in OQ5; extended in COMMITTEE / DREP / ENACTMENT-COMMITTEE-FIDELITY / ENACTMENT-COMMITTEE-WRITEBACK; CONSUMED unchanged in PROPOSAL-PROCEDURES-DECODE)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Closed credential domain type** | `ade_types::shelley::cert::StakeCredential` | BLUE | Closed 2-variant `{ KeyHash(Hash28), ScriptHash(Hash28) }`. |
| **Data-only — closed credential-decode chokepoints** | `ade_codec::{shelley,conway}::cert::decode_stake_credential` | BLUE | Each maps `0 → KeyHash`, `1 → ScriptHash`; rejects unknown tag. **PP cluster adds a third use site** at `ade_codec::conway::governance::decode_stake_credential` (UpdateCommittee inner reads); preserves the same closed 2-variant mapping. |
| **Authoritative enforcement — gov-state key surface** | `ade_ledger::state::ConwayGovState.{vote_delegations, committee_hot_keys, drep_expiry, committee}` + `ade_types::conway::governance::GovActionState.{committee_votes, drep_votes}` | BLUE | All discriminated `StakeCredential`. |
| **Determinism — discriminant-faithful fingerprint** | `ade_ledger::fingerprint::{write_stake_credential, write_credential_vote_list}` | BLUE | Emits discriminant before hash. |
| **Narrow read-only boundary adapter** | `StakeCredential::hash()` | BLUE | Sanctioned discriminant-discarding extraction; ONLY against declared non-goal surfaces. |

**Rule.** Discriminant preserved end-to-end on the BLUE authoritative
path (DC-LEDGER-10, strengthened across all five OQ5/COMMITTEE/DREP/
ENACTMENT clusters). **DC-LEDGER-11 (PROPOSAL-PROCEDURES-DECODE)
CONSUMES this rule unchanged** via the `UpdateCommittee` arm of
`decode_gov_action`; bidirectional cross-ref recorded in the
registry. `DC-LEDGER-10.cross_ref` now includes `DC-LEDGER-11`;
`DC-LEDGER-11.cross_ref = [DC-LEDGER-10]`.

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
into `tx_validity` is `mempool_ingress` → `admit` → `tx_validity`;
the composer itself is untouched. **PP note:** the typed
`ConwayTxBody.proposal_procedures` field is in scope for any future
tx-validity precondition that needs to gate on proposal content;
no such precondition is added in this cluster.

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
| **Data-only tooling** | `ade_codec` | BLUE\* | Decodes block / tx / cert / withdrawal bytes; **PP: now also typed `proposal_procedures` sub-grammar via `ade_codec::conway::governance`**. |
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
| **Data-only tooling — governance types** | `ade_types::conway::governance` (incl. **NEW `ProposalProcedure` from PP-S1**; existing `GovAction`, `Anchor`, `GovActionId`, etc.) | BLUE | Closed governance domain types. |
| **Data-only tooling — closed sub-grammar decoder (NEW in PP)** | `ade_codec::conway::governance::{decode_proposal_procedures, encode_proposal_procedures, decode_gov_action, …}` | BLUE | Closed sub-grammar at Conway tx-body key 20; preserves DC-LEDGER-10 discriminant through `UpdateCommittee`. |
| **Authoritative enforcement** | `ade_ledger::governance::{evaluate_ratification, enact_proposals, expire_proposals}` | BLUE | Chokepoints. |
| **Committee write-back** *(ENACTMENT-COMMITTEE-WRITEBACK)* | `ade_ledger::governance::apply_committee_enactment` | BLUE | Closed pure transition called at the `rules.rs` epoch boundary. |
| **Snapshot decode (data-only)** *(tightened in 168ac02; carried)* | `ade_testkit` snapshot loader | GREEN | Fail-closed decode of `update_committee`. |
| **GREEN canonical synthetic corpus + replay harness (NEW in PP-S2)** | `ade_testkit::governance::proposal_procedures_replay` | GREEN | Round-trip harness for the `proposal_procedures` sub-grammar across all 7 `GovAction` variants. |

**Rule.** A new governance action variant adds a variant to `GovAction`
+ arms in all three enactment chokepoints **plus an arm in the new
PP `decode_gov_action` / `encode_gov_action`**. **The
`proposal_procedures` tx-body decode seam is CLOSED at this HEAD
(PROPOSAL-PROCEDURES-DECODE).** The remaining open governance-domain
seams are the four PP open obligations recorded in
`DC-LEDGER-11.open_obligation`: `voting_procedures` decode (OQ-1),
`ParameterChange.update` nested decode (OQ-2), `NewConstitution.raw`
nested decode (OQ-3), typed `RewardAccount` for `return_addr` (OQ-4).

### Mini-protocol wire conformance (N-A)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Data-only tooling (frame)** | `ade_network::mux::frame` | BLUE | Pure encode/decode. |
| **Data-only tooling (messages)** | `ade_network::codec::*` (11 modules) | BLUE | 11 closed wire grammars. |
| **Authoritative enforcement (state)** | `ade_network::*::transition` + `n2c::local_*::transition` | BLUE | 8 closed pure transition functions. |
| **Bearer (I/O)** | `ade_network::mux::transport` | RED | Tokio-based scaffold. |
| **Session composition (placeholder)** | `ade_network::session::mod` | RED | S-A9 placeholder. |
| **Live-interop capture tools** | `ade_network::bin::capture_*` | RED | Operator/dev tools. |
| **Tx-submission bridges** *(N-E)* | `ade_core_interop::{tx_submission, local_tx_submission}` | GREEN | |
| **Tx-submission probe binary** *(N-E S6)* | `ade_core_interop::bin::live_tx_submission_session` | RED | |

**Rule.** The codec layer is opaque to higher semantics.

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
| **Live-interop driver (scaffold)** | `ade_core_interop::bin::live_consensus_session` | RED | The original operator-action probe binary; pattern reused by N-E S6. |
| **Replay harness** | `ade_testkit::consensus::stream_replay::replay_stream` | GREEN | |

**Rule.** Five rules: genesis-parser sole RED→BLUE materialization;
`BootstrapAnchorHash` binds; `LedgerView` closed; authoritative
chokepoints never move; selector and chain-dep advance in lockstep.

### Where the boundary is enforced

- `ci_check_dependency_boundary.sh` — no BLUE crate may depend on RED.
  PP added no new crate edge.
- `ci_check_no_async_in_blue.sh` — async forbidden in BLUE.
- **`ci_check_proposal_procedures_closed.sh`** *(PP-S1 — DC-LEDGER-11,
  `status=enforced`)* — see the §2 "Conway tx-body `proposal_procedures`
  sub-grammar" entry above for the 5 guards.
- `ci_check_mempool_ingress_closure.sh` *(N-E — DC-MEM-03,
  `status=enforced`)*.
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

**PP note on `ci_check_ingress_chokepoints.sh`:** the
`decode_proposal_procedures` BLUE entry point is a **sub-grammar
decoder** invoked from within an existing ingress chokepoint
(`decode_conway_tx_body` at key 20); it operates on already-captured
sub-bytes and is therefore outside this gate's scope (the existing
body-decode chokepoint is the one that fits the chokepoints model).
Its closure is instead defended mechanically by
`ci_check_proposal_procedures_closed.sh`.

---

## 3. Closed vs. Extensible Registries

Ade's authority surface is **almost entirely closed.** **PP-S1 added
two closed surfaces** — `ProposalProcedure` (closed 4-field struct)
and `decode_proposal_procedures` (closed-grammar entry point) — plus
**one CI gate** (`ci_check_proposal_procedures_closed.sh`), bringing
CI count `31 → 32`. PP-S1 also typed the `ConwayTxBody.proposal_procedures`
field from `Option<Vec<u8>>` to `Option<Vec<ProposalProcedure>>` (a
shape change on an existing closed struct). PP added **no new wholly
open extensible surface**; PP-S2 added the GREEN canonical-synthetic
corpus harness as a tooling-only extensible surface
(`ade_testkit::governance::proposal_procedures_replay`).

### Closed (frozen — version-gated changes only)

| Registry | Location | Count | Change Rule |
|----------|----------|-------|-------------|
| `CardanoEra` | `ade_types::era` | 8 variants | New variant = new hard fork. |
| `Certificate` | `ade_types::shelley::cert` | 7 variants | Shelley-era frozen. **B4:** `PoolRegistrationCert.owners` added. |
| **`StakeCredential`** *(closed 2-variant — NEW shape in OQ5)* | `ade_types::shelley::cert` | 2 variants — `KeyHash(Hash28)`, `ScriptHash(Hash28)` | Grep-gated by `ci_check_credential_discriminant_closed.sh` (DC-LEDGER-10). |
| **Credential-decode chokepoints** *(closed grammar — NEW in OQ5; THIRD use site added in PP)* | `ade_codec::{shelley,conway}::cert::decode_stake_credential` + **`ade_codec::conway::governance::decode_stake_credential` (NEW in PP-S1 — used by `UpdateCommittee` arm of `decode_gov_action`)** | 3 functions | All three preserve the closed 2-variant mapping. |
| **`ConwayCert`** *(closed CDDL grammar — refined in B3, owner-completed in B4)* | `ade_types::conway::cert` | 19 variants over tags `0..18` | Grep-gated by `ci_check_conway_cert_classification_closed.sh`. |
| `GovAction` *(UpdateCommittee re-shaped structured in ENACTMENT-COMMITTEE-WRITEBACK; **reused unchanged by PP-S1 `decode_gov_action`**)* | `ade_types::conway::governance` | 7 variants (cardinality unchanged) | Closed 7-variant. PP-S1 wires `decode_gov_action` / `encode_gov_action` over these 7 variants; new variant = simultaneous addition to all three enactment chokepoints + the PP decoder + the PP encoder. |
| **`ProposalProcedure`** *(NEW in PP-S1 — DC-LEDGER-11)* | `ade_types::conway::governance` | closed struct `{ deposit: Coin, return_addr: Vec<u8>, gov_action: GovAction, anchor: Anchor }` | Closed 4-field domain type. **Construction outside `decode_proposal_procedures` + the testkit fixture builders + `crates/*/tests/` + inline `#[cfg(test)]` blocks is CI-forbidden** (`ci_check_proposal_procedures_closed.sh` guard 5). |
| **`decode_proposal_procedures` chokepoint** *(NEW in PP-S1 — DC-LEDGER-11; closed-grammar entry point, not a registry)* | `ade_codec::conway::governance` | 1 function — `pub fn decode_proposal_procedures(&[u8]) -> Result<Vec<ProposalProcedure>, CodecError>` | The **single sanctioned production decoder** for the Conway tx-body `proposal_procedures` sub-grammar. Closed grammar: no silent-skip arm; rejects unknown `gov_action` tag, empty set, trailing garbage, truncated procedure, invalid stake credential. The body codec at key 20 is the only production caller (CI guard 4); the opaque `Option<Vec<u8>>` form on `ConwayTxBody.proposal_procedures` is CI-forbidden (guard 2). |
| **`encode_proposal_procedures` chokepoint** *(NEW in PP-S1 — DC-LEDGER-11)* | `ade_codec::conway::governance` | 1 function | The **single re-encoder** — byte-identical round-trip authority for PreservedCbor (CE-PP-2, CE-PP-6). |
| `MIRPot` | `ade_types::shelley::cert` | 2 variants | Frozen. |
| `DRep` | `ade_types::conway::cert` | 4 variants | CIP-1694 fixed. |
| **`CertDisposition`** *(B3)* | `ade_types::conway::cert` | 3 variants | Era-grammar reject is NOT a `DepositEffect`. |
| **`DepositEffect`** *(B3)* | `ade_types::conway::cert` | 2 variants | Closed. |
| **`CoinSource`** *(B3)* | `ade_types::conway::cert` | 3 variants | Closed deposit-provenance set. |
| **`ConwayCertAction`** *(B4)* | `ade_ledger::delegation` | closed — one variant per Conway cert kind | No `Neutral` variant. |
| **`GovernanceCertEffect`** / **`GovernanceOwner`** / **`OwnerTaggedEffect`** / **`ConwayCertOutcome`** *(B4)* | `ade_ledger::delegation` | closed | The owner-tagged effect plumbing B5 consumes. |
| **`GovCertEnv`** *(B5)* | `ade_ledger::state` | closed struct `{ current_epoch, drep_activity }` | Fail-fast `MissingDRepActivityParam`. |
| **`apply_conway_gov_cert` dispatch** *(B5)* | `ade_ledger::gov_cert` | 1 function — total `match` over `ConwayCert` | No `_ =>` wildcard. Grep-gated by `ci_check_gov_cert_accumulation_closed.sh` (DC-LEDGER-09). |
| **`apply_committee_enactment` write-back** *(ENACTMENT-COMMITTEE-WRITEBACK)* | `ade_ledger::governance` | 1 pure transition | Operates on discriminated `BTreeMap<StakeCredential, u64>`. Called at `rules.rs:1224`. |
| **`EnactmentEffects` struct** | `ade_ledger::governance` | closed struct | Grep-gated by `ci_check_credential_discriminant_closed.sh` check 6. |
| **`IngressSource`** *(N-E S1 — DC-MEM-03)* | `ade_ledger::mempool::ingress` | 2 variants — `N2N`, `N2C` | Closed source discriminant. Grep-defended. |
| **`IngressEvent`** *(N-E S1 — DC-MEM-03)* | `ade_ledger::mempool::ingress` | closed struct `{ source: IngressSource, tx_bytes: Vec<u8> }` | Closed flat-data envelope. |
| **`mempool_ingress` chokepoint** *(N-E S1 — DC-MEM-03)* | `ade_ledger::mempool::ingress` | 1 function | The single BLUE chokepoint from wire ingress into `admit`. |
| **`MempoolState.accumulating` field-write closure** *(strengthened in N-E S1 — DC-MEM-03)* | `crates/ade_ledger/src/mempool/admit.rs` | 1 production write site | Grep-gated. |
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
| **`MempoolState`** *(B2; field-write grep-gated in N-E)* | `ade_ledger::mempool::admit` | struct `{ accepted, accumulating }` | Grep-gated. |
| **`OrderPolicy`** *(B2)* | `ade_ledger::mempool::policy` | 2 variants — ArrivalOrder, TxIdAscending | |
| **`ConwayOnlyDepositParams`** *(B3; B5-enriched)* | `ade_ledger::pparams` | struct + `drep_activity` | |
| **`ConwayDepositParams`** *(B3)* | `ade_ledger::pparams` | struct (view) | |
| **`ValidationEnvironmentError`** *(B3)* | `ade_ledger::error` | | |
| **`UnsupportedStateDependentDepositAccounting`** *(B3)* | `ade_ledger::error` | | |
| **`EraInvalidCertificateError`** *(B3)* | `ade_ledger::error` | | |
| **`PraosNonces` / `NonceScanError`** *(B1)* | `ade_ledger::consensus_input_extract` | | |
| **`PraosChainDepState` / `ChainEvent` canonical encodings** *(N-B)* | `ade_core::consensus::encoding` | 4 chokepoints | |
| **`LedgerFingerprint` fold** *(B3-extended; B5-extended)* | `ade_ledger::fingerprint` | | |
| **CI check set** | `ci/ci_check_*.sh` | **32 scripts (31 → 32 in PP-S1; PP-S2 added no new CI)** | Existing checks may be tightened, never relaxed. |
| **Invariant registry families** | `docs/ade-invariant-registry.toml` | Families T / CN / DC / OP / RO; DC extended across all prior clusters; **PP-S1 added `DC-LEDGER-11`** (`enforced`, `introduced_in = PROPOSAL-PROCEDURES-DECODE`, bidirectional `cross_ref = [DC-LEDGER-10]`); `DC-LEDGER-10.cross_ref += "DC-LEDGER-11"` (bidirectional pairing); PP-S2 extended `DC-LEDGER-11.tests` with 4 harness test names. Total: **176 entries** (175 → 176). | Append-only IDs. |

### Extensible (open within constraints)

| Registry | Location | Extension Rule |
|----------|----------|---------------|
| `CostModels` map (Plutus V1/V2/V3 cost tables) | `ade_plutus::cost_model::CostModels` | Decoder-driven; constrained by closed `PlutusLanguage`. |
| `ProtocolParameters` / `ProtocolParameterUpdate` field set | `ade_ledger::pparams` | Era-versioned. |
| Pool / DRep / Stake registrations | `ade_ledger::state::{DelegationState, CertState}` | Shape closed; set open. |
| Governance proposal / committee / DRep registration set | `ade_ledger::state::ConwayGovState` | Shape closed; instance set open. **ENACTMENT-COMMITTEE-WRITEBACK**: also written back at the epoch boundary by `apply_committee_enactment`. |
| **Tx-body `proposal_procedures` instance set** *(PP-S1; instance-open within closed shape)* | `ade_types::conway::tx::ConwayTxBody.proposal_procedures` | `Option<Vec<ProposalProcedure>>`. **Shape closed** (`ProposalProcedure` struct closed; populated only by `decode_proposal_procedures`); **set instance-open** (number / contents of `ProposalProcedure` entries per Conway tx body is wire-driven). Non-empty by CIP-1694 (the closed decoder rejects empty sets). |
| `OpCertCounterMap` *(N-B)* | `ade_core::consensus::praos_state` | BTreeMap; inserts strictly increasing per `(pool, kes_period)`. |
| `PoolDistrView` pool table *(B1)* | `ade_ledger::consensus_view::PoolDistrView::pools` | `BTreeMap<Hash28, PoolEntry>`. |
| Withdrawals map *(B3)* | decoded by `ade_codec::conway::withdrawals::decode_withdrawals` → `BTreeMap<RewardAccount, Coin>` | Shape closed; never last-wins. |
| Mempool admitted set *(B2; ingress-fed in N-E)* | `ade_ledger::mempool::admit::MempoolState::accepted` | `Vec<Hash32>` of admitted tx ids. Shape closed; set open; grows monotonically. |
| `SignerSource` provenance set *(B2)* | `ade_ledger::tx_validity::required_signers::RequiredSigners::{keys, provenance}` | Per-tx open; `SignerSource` *enum* closed. |
| `RollbackSnapshot` ring *(N-B)* | `ade_runtime::consensus::chain_selector::OrchestratorState::recent_snapshots` | Bounded ≤ `DEFAULT_SNAPSHOT_LIMIT = 2160`. |
| Oracle reference snapshots / regression corpus | `ade_testkit::harness::*` | Tooling-only. |
| Network corpus | `corpus/network/{n2n,n2c}/*` | Tooling-only. |
| Consensus corpus | `corpus/consensus/*` | Tooling-only. |
| Block-validity corpus *(B1)* | `corpus/validity/*` | Tooling-only. |
| Tx-validity corpus *(B2; B3-extended)* | `ade_testkit::tx_validity::*` + B3 conservation corpora | Tooling-only. |
| **Mempool ingress corpus** *(N-E; tooling-only)* | `ade_testkit::mempool::ingress_replay` + the B-track corpus wrapped via `b_track_corpus_as_ingress` | Tooling-only. Single-step fold; append-only by convention; GREEN. |
| **`proposal_procedures` canonical synthetic corpus + replay harness** *(NEW in PP-S2; tooling-only)* | `ade_testkit::governance::proposal_procedures_replay` | Tooling-only. GREEN. Synthesizes well-formed Conway tx-body fixtures covering all 7 `GovAction` variants + the DC-LEDGER-10 `UpdateCommittee` discriminant case; asserts byte-identical decode + encode round-trip. Append-only by convention. Future strengthening: real-chain corpus extraction (per OQ-5 — declared as a possible future PP-S3 or successor cluster). |
| **Operator-action probe binaries** *(N-B + N-E S6)* | `ade_core_interop::bin::{live_consensus_session, live_tx_submission_session}` | RED operator-action; `#[ignore]`-gated by closure-gate tests. |
| `KillStrategy<D>` trait impls | `ade_runtime::chaindb::crash_safety` | RED-only test infrastructure. |
| Recovery state types | callers of `Recoverable` | Open: any state with canonical encode + apply-block step. |
| Pinned external crates | `crates/*/Cargo.toml` | Tier-5 rationale doc required. |

### Candidates — extensible surfaces not yet wired

| Cluster | Candidate registry | Rationale |
|---------|-------------------|-----------|
| **PP OQ-1 (separable future seam — declared open obligation)** | **`voting_procedures` (Conway tx-body key 19) typed instance set** | Mirror of the now-closed `proposal_procedures` shape. Same opaque-bytes pressure; natural sibling cluster. Would extend `ade_codec::conway::governance` with `decode_voting_procedures` + a new closed `VotingProcedure` struct + a future `strengthened_in += <cluster>` on DC-LEDGER-11 OR a new DC-LEDGER-12. |
| **PP OQ-2 (separable future seam — declared open obligation)** | **`ParameterChange.update` typed nested instance** | Currently opaque `Vec<u8>` inside the typed `GovAction::ParameterChange` variant. Full Conway pparams update sub-grammar. Separate large cluster. |
| **PP OQ-3 (separable future seam — declared open obligation)** | **`NewConstitution.raw` typed nested instance** | Currently opaque `Vec<u8>` inside the typed `GovAction::NewConstitution` variant. Bundleable with the voting-procedures cluster. |
| **PP OQ-4 (separable future seam — declared open obligation)** | **Typed `RewardAccount` for `proposal_procedure.return_addr`** | Currently raw `Vec<u8>`. Would also strengthen `TreasuryWithdrawals.withdrawals` element type in one move. |
| **N-E+ Tier-5** | **Mempool eviction / prioritization policy (bounded mempool, shedding policy)** beyond the `OrderPolicy` stub | Tier-5 — operator-tunable. Declared OUT-OF-SCOPE in the N-E cluster doc. |
| **N-E+ Tier-1** | **Outbound tx propagation (Ade as a tx source — `tx-submission2` server side)** | Separate authority surface from N-E's ingress half. Declared OUT-OF-SCOPE in the N-E cluster doc. |
| **CE-NODE-N2C-LTX (cross-cluster obligation)** | **Live N2C UDS server + N2N bulk-tx inbound listener** | The deferred halves of CE-N-E-7 + CE-N-E-6. |
| N-A (deferred) | Peer address book | Operator-supplied; runtime mutable. |
| N-C | Block-production policy (forge cadence, KES rotation, slot election) | Tier 1 semantics, Tier 5 operator triggers. |
| N-F | Query API method set | Tier 5 wire / Tier 1 semantics. |
| N-F | Prometheus metric names | Tier 5; append-only registry expected. |
| GOVCERT-validity *(OQ-3, separable)* | Committee-membership precondition | Tier 1 — a tx-validity gate, NOT a registry. |
| credential-discriminant *(WIRED + CLOSED)* | DONE — see closed surfaces above. | |
| proposal-decode *(WIRED + CLOSED in PROPOSAL-PROCEDURES-DECODE)* | DONE — see closed surfaces above. | |

User confirmation needed for each at cluster entry.

### Closed-grammar audit (PROPOSAL-PROCEDURES-DECODE full close)

This sweep was performed after PROPOSAL-PROCEDURES-DECODE full close
(PP-S1 + PP-S2).

1. **`decode_proposal_procedures` entry point** — **closed by intent
   and CI-defended.** Closed-grammar BLUE sub-decoder. No silent-skip
   arm. Rejects unknown `gov_action` tag (5 grammar guards in unit
   tests), empty set (CIP-1694 requires non-empty), trailing garbage,
   truncated procedure, invalid stake credential in `UpdateCommittee`
   (DC-LEDGER-10 preserved via structured `UpdateCommittee` arm). The
   body codec at key 20 is the only production caller (CI guard 4);
   `ProposalProcedure` construction outside the decoder + the testkit
   fixture builders + `crates/*/tests/` + `#[cfg(test)]` blocks is
   CI-forbidden (guard 5). The opaque `Option<Vec<u8>>` form on
   `ConwayTxBody.proposal_procedures` is CI-forbidden (guard 2).
2. **`encode_proposal_procedures` re-encoder** — **closed by intent.**
   Byte-identical round-trip authority for PreservedCbor (CE-PP-2,
   CE-PP-6 — canonical synthetic corpus harness covering all 7
   `GovAction` variants + the DC-LEDGER-10 `UpdateCommittee`
   discriminant case).
3. **`ProposalProcedure` struct** — **closed by intent and CI-defended.**
   Closed 4-field struct; construction sites CI-allowlisted (guard 5).
4. **`ConwayTxBody.proposal_procedures` field** — **typed and
   CI-defended.** `Option<Vec<ProposalProcedure>>`; reverting to
   opaque bytes is CI-forbidden (guard 2).
5. **`decode_gov_action` arm closure** — **closed by intent.** Closed
   7-variant dispatch over the existing `GovAction` enum; preserves
   DC-LEDGER-10 discriminant through the `UpdateCommittee` arm
   (in-vivo certified by
   `update_committee_keeps_stake_credential_discriminant`); inner
   `ParameterChange.update` + `NewConstitution.raw` stay opaque
   `Vec<u8>` by OQ-2 / OQ-3 (deliberate scope locks).
6. **PP-S2 GREEN canonical synthetic corpus harness** — **closed by
   intent on the round-trip property.** Deterministic; no I/O; no
   clocks. Single-step `decode → encode` fold per fixture; asserts
   byte-identical. Covers all 7 `GovAction` variants + the
   DC-LEDGER-10 `UpdateCommittee` case.

**Gap note — PP (narrow).** The PP cluster's nested opacities
(voting_procedures key 19, `ParameterChange.update` inner payload,
`NewConstitution.raw` inner payload, raw `return_addr` bytes) are
deliberate OQ-locked scope decisions — they are declared
`open_obligation` in `DC-LEDGER-11` and recorded as separable
future seams in §1 above. They are NOT load-bearing on the
PROPOSAL-PROCEDURES-DECODE cluster's invariant (which is the closed
shape of `proposal_procedures` itself at the tx-body boundary).

### Closed-grammar audit (carried — PHASE4-N-E full close)

Carried unchanged from the prior revision: `IngressSource` /
`IngressEvent` / `mempool_ingress` chokepoint /
`MempoolState.accumulating` field-write closure / per-peer
canonicalizer / GREEN single-step replay harness / N2N+N2C GREEN
bridges / `live_tx_submission_session` probe binary.

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
- **Closed `proposal_procedures` wire grammar at the Conway tx-body
  key 20 boundary** *(NEW in PROPOSAL-PROCEDURES-DECODE PP-S1 —
  DC-LEDGER-11)*: the closed grammar
  `proposal_procedure = [coin, reward_account, gov_action, anchor]`
  (CIP-1694) is decoded via `decode_proposal_procedures` and
  re-encoded via `encode_proposal_procedures` byte-identically
  (PreservedCbor). **`ProposalProcedure` shape is frozen**:
  `Anchor` stays opaque (`{ raw: Vec<u8> }`), `return_addr` stays
  raw `Vec<u8>` (typed `RewardAccount` is OQ-4 future fidelity),
  `gov_action` reuses the existing closed 7-variant `GovAction`
  enum unchanged. Closed grammar rejects unknown `gov_action` tag,
  empty set, trailing garbage, truncated procedure, invalid stake
  credential. **DC-LEDGER-10 discriminant preserved** through the
  `UpdateCommittee` arm. Defended by
  `ci_check_proposal_procedures_closed.sh` (5 guards).
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
- **`IngressSource` source-invariance contract** *(N-E S1 — N-E-N7 / N-E-8)*.
- **Verbatim tx-bytes flow through ingress** *(N-E)*.
- **GREEN single-step replay fold contract** *(N-E S2 — DC-MEM-04)*.
- **Cross-cluster obligation pattern** *(introduced in N-E
  full close)*: frozen project-level closure contract.
- **Operator-action evidence pattern** *(strengthened in N-E full close)*.
- **Closed credential discriminant contract** *(OQ5 / COMMITTEE / DREP /
  ENACTMENT-COMMITTEE-FIDELITY / ENACTMENT-COMMITTEE-WRITEBACK; CONSUMED
  unchanged in PROPOSAL-PROCEDURES-DECODE)*. The bidirectional
  cross-ref `DC-LEDGER-10 ↔ DC-LEDGER-11` is now recorded in the
  registry — PP's `UpdateCommittee` decode arm CONSUMES the closed
  `StakeCredential` form unchanged.
- **Committee-enactment write-back contract** *(ENACTMENT-COMMITTEE-WRITEBACK)*:
  `apply_committee_enactment` at `rules.rs:1224`; `NoConfidence`
  dissolves the committee (DC-EPOCH-01 / DC-LEDGER-10).
- **All canonical types**: shapes frozen at the era / version they
  entered.
- **TCB color assignments**: per `.idd-config.json` `core_paths`.
  `ade_core::consensus`, `ade_ledger::{block_validity, tx_validity,
  mempool::admit, mempool::ingress, consensus_view, cert_classify,
  delegation, gov_cert}`, `ade_codec::conway::{cert, withdrawals,
  governance}` *(governance new in PP-S1)*, `ade_codec::shelley::cert`,
  and `ade_types::conway::{cert, governance}` are BLUE;
  `ade_ledger::mempool::policy` and `ade_ledger::mempool::canonicalize`
  are GREEN behavior inside the BLUE crate;
  `ade_ledger::consensus_input_extract` is RED-behavior-inside-BLUE;
  `ade_runtime::consensus` is RED;
  `ade_testkit::{consensus, validity, tx_validity, mempool, governance}`
  *(governance new in PP-S2)* is GREEN;
  `ade_core_interop` is RED-crate / GREEN-pure-functions /
  RED-operator-action-binaries.
- **`ChainDb` / `SnapshotStore` / `Recoverable` trait shapes** (N-D
  closed): trait method sets frozen.

### Version-gated (can evolve across major versions)

- **New `CardanoEra` variant**: full coordinated change.
- **New Conway certificate tag** *(B3 / B4 / B5)*.
- **New `CoinSource` deposit-provenance** *(B3)*.
- **Pre-Conway single-tx validity** *(B2 extension point)*.
- **Full-scope `track_utxo=true` tx corpus** *(B2 extension point)*.
- **Conway block-body vkey-witness closure** *(B2-carried, post-B3/B4/B5/N-E/PP)*.
- **Conway governance certificate accumulation authority** *(B5, WIRED + CLOSED)*.
- **Credential discriminant extension** *(declared non-goal carried)*.
- **Committee-enactment write-back** *(ENACTMENT-COMMITTEE-WRITEBACK, WIRED + CLOSED)*.
- **Conway tx-body `proposal_procedures` decode** *(WIRED + CLOSED in
  PROPOSAL-PROCEDURES-DECODE)*: the closed sub-grammar +
  `ProposalProcedure` struct + `decode_proposal_procedures` /
  `encode_proposal_procedures` are the canonical wiring.
  **Remaining separable version-gated follow-ups** (NOT open seams
  now; declared `open_obligation` in `DC-LEDGER-11`):
  - **`voting_procedures` (key 19) opaque-bytes** remains
    version-gated for future closure (OQ-1) — natural sibling
    cluster, same shape pressure.
  - **`ParameterChange.update` nested opacity** remains opaque
    `Vec<u8>` inside the typed `GovAction::ParameterChange` variant
    (OQ-2) — future `strengthened_in` candidate on DC-LEDGER-11 if
    closed in-place, or a new DC-LEDGER-1X if closed as a parallel
    surface.
  - **`NewConstitution.raw` nested opacity** remains opaque
    `Vec<u8>` inside the typed `GovAction::NewConstitution` variant
    (OQ-3) — bundleable with the voting-procedures cluster.
  - **Typed `RewardAccount` for `proposal_procedure.return_addr`**
    (OQ-4) — future strengthening that would also retype
    `TreasuryWithdrawals.withdrawals` element type in one move.
- **TPraos full-block validity** *(B1 extension point)*.
- **New `GovAction` / Plutus version variant**: a new `GovAction`
  variant now requires simultaneous arm additions to all three
  enactment chokepoints **PLUS** the PP `decode_gov_action` /
  `encode_gov_action` (compiler-exhaustive `match`).
- **New `SignerSource` variant** *(B2)*.
- **New `TxRejectClass` / `BlockRejectClass` / `FieldKind` /
  `MissingInput` variant**.
- **New `OrderPolicy` variant** *(B2)*.
- **New protocol parameter field**.
- **New CI check**: additive. (PP added one — `ci_check_proposal_procedures_closed.sh`.)
- **Pinned external crate bump**: Tier-5 rationale doc required.
- **New mini-protocol**.
- **Mini-protocol version-table bump**.
- **New `ChainEvent` / `ChainSelectionReject` / `StreamInput` variant** *(N-B)*.
- **New `NetworkMagic`** *(N-B)*.
- **New `LedgerView` impl / LedgerState-backed `PoolDistrView` constructor**.
- **`BootstrapAnchorHash` preimage v2** *(N-B)*: hard version-gated.
- **N2N/N2C tx-submission → `mempool_ingress` ingress** *(WIRED + CLOSED in N-E)*.
- **Phase-4 cluster surface additions** (N-C, N-F): each cluster's
  wire surface gates additions via its own cluster doc.

---

## 5. Module Addition Rules

Ade's workspace is small and color-disciplined. **PP-S1 added one
new BLUE submodule** (`ade_codec::conway::governance`, ~580 LOC,
exporting `decode_proposal_procedures` + `encode_proposal_procedures`
+ per-component decoders/encoders + helpers) inside the existing
BLUE `ade_codec` crate; **PP-S1 added one new closed type**
(`ade_types::conway::governance::ProposalProcedure`) inside the
existing BLUE `ade_types` crate; **PP-S1 typed one existing field**
(`ConwayTxBody.proposal_procedures` from `Option<Vec<u8>>` to
`Option<Vec<ProposalProcedure>>`); **PP-S1 added one new CI gate**
(`ci/ci_check_proposal_procedures_closed.sh`); **PP-S1 added one
new BLUE entry point** at the body codec key-20 path. **PP-S2 added
one new GREEN submodule** (`ade_testkit::governance::proposal_procedures_replay`,
canonical synthetic corpus + replay harness) inside the existing
GREEN `ade_testkit` crate. PP added **no new crate, no new external
ingress wire-format frozen contract beyond the closed
`proposal_procedures` sub-grammar, no new public composer, no new
operator-action probe binary, no new live-evidence log artifact**.

**The module-addition rule PP sets for future governance-domain
sub-grammar decoders:**

1. **A new governance-domain sub-grammar decoder attaches as a
   parallel closed entry point** in `ade_codec::conway::governance`
   (sibling of `decode_proposal_procedures`) — e.g. a future
   `decode_voting_procedures`. The decoder MUST be a closed BLUE
   function returning `Result<T, CodecError>` with no silent-skip
   arm, deterministic rejects on unknown discriminants, empty sets,
   trailing garbage, and structural failures.
2. **A new typed sub-grammar field on `ConwayTxBody` attaches as a
   typed shape replacement** (e.g. `voting_procedures:
   Option<Vec<VotingProcedure>>`) — not as a parallel opaque-bytes
   sibling field. The body codec at the relevant key calls the
   typed decoder + encoder; the opaque pass-through form is
   CI-forbidden.
3. **A new closed governance-domain struct attaches as a closed
   N-field struct** in `ade_types::conway::governance` (sibling of
   `ProposalProcedure`). Construction outside the closed decoder +
   the testkit fixture builders MUST be CI-forbidden by a guard
   modeled on `ci_check_proposal_procedures_closed.sh` guard 5.
4. **A new closed sub-grammar requires its own CI gate** (or an
   extension of an existing one) covering the 5-guard shape:
   (a) struct defined with the expected fields; (b) typed field
   on the parent struct (not opaque bytes); (c) decoder + encoder
   exported by the codec module; (d) body codec at the relevant
   key calls the typed decoder + encoder; (e) struct-literal
   construction outside sanctioned sites is forbidden.
5. **A new closed sub-grammar requires a new derived-Cardano
   registry rule** OR extends `DC-LEDGER-11.strengthened_in` (the
   registry's choice when planned). Bidirectional `cross_ref`
   recording is mandatory when the new rule consumes an existing
   discriminant rule (e.g. DC-LEDGER-10 ↔ DC-LEDGER-11).
6. **A new closed sub-grammar requires both unit tests
   (rejection-grammar + per-variant round-trip) and a GREEN
   canonical synthetic corpus harness** modeled on PP-S2
   (`proposal_procedures_replay`) — byte-identical decode → encode
   round-trip across all closed-enum variants + any discriminant
   preservation cases.

### Cross-cluster obligation pattern (carried — introduced in N-E full close)

PHASE4-N-E full close introduced the cross-cluster obligation
pattern. **PROPOSAL-PROCEDURES-DECODE does NOT invoke this pattern**
— the cluster's evidence is fully mechanical (no deferred live-wire
half). The four PP open obligations are recorded as separable
**candidate seams** (per §1), not as cross-cluster obligations
under the pattern's binding rules. See N-E full-close narrative for
the pattern's frozen rules and the CE-NODE-N2C-LTX instance.

### Operator-action evidence pattern (carried — strengthened in N-E full close)

This pattern was established by PHASE4-N-B (CE-N-B-6 live tip
agreement) and reinforced by PHASE4-N-E (CE-N-E-6 live N2N
tx-submission2 outbound-client probe + the
`live_tx_submission_session` probe binary).
**PROPOSAL-PROCEDURES-DECODE does NOT add a new operator-action
entry** — the cluster's evidence is fully mechanical (decoder unit
tests + canonical synthetic corpus harness + CI gate). The pattern
remains exactly as documented in the N-E full-close revision.

**OQ5/COMMITTEE/DREP/ENACTMENT-COMMITTEE-FIDELITY/WRITEBACK** all
followed the in-place-tightening model. **N-E** added two
crate-internal submodules + the cross-cluster obligation pattern +
the second probe binary. **B5** added one new crate-internal BLUE
module + one new CI gate. **B4** added the owner-tagged apply
model in place. **B3** added four BLUE submodules inside existing
BLUE crates. **B2** added the `tx_validity::*` and
`mempool::{admit, policy}` submodule trees. **PP-S1 follows the
B3 / B5 pattern** (one new BLUE submodule inside `ade_codec` + one
new closed type inside `ade_types` + one new CI gate); **PP-S2
adds one new GREEN submodule inside `ade_testkit`** following the
B5 GREEN-harness pattern. PP adds no new closure pattern of its own
beyond what N-E established.

| Color | Naming convention | Build-config flags | May depend on | MUST NOT depend on |
|-------|-------------------|--------------------|----------------|--------------------|
| **BLUE** | `ade_*` | First line of every `.rs` is the contract banner. `lib.rs` carries `#![deny(unsafe_code, clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_arithmetic)]`. No `#[cfg(feature = ...)]`. No async. No `ChainDb`/`f32`/`f64`/density inside `ade_core::consensus`. No `#[non_exhaustive]`/open-tail/`String`/`Box<dyn>` in `ade_core::consensus`, `ade_ledger::block_validity`, `ade_ledger::tx_validity`, `ade_ledger::mempool`. **N-E:** `IngressSource` closed 2-variant; `IngressEvent` closed flat-data struct; `mempool_ingress` body must not reference `source` (DC-MEM-03). **PP:** `ProposalProcedure` closed 4-field struct; `ConwayTxBody.proposal_procedures` typed `Option<Vec<ProposalProcedure>>` (not opaque bytes); `decode_proposal_procedures` body has no silent-skip arm; no struct-literal construction of `ProposalProcedure` outside sanctioned sites (DC-LEDGER-11). | Other BLUE crates / submodules only | Any RED submodule or crate; GREEN in non-dev deps; `pallas_*` (except `ade_plutus`); async runtime; `HashMap`/`HashSet`/`IndexMap`; clock/rand/float/env/I/O. |
| **GREEN** | `ade_*` | Banner + deny attrs are project convention but not currently enforced for `ade_testkit` / `ade_network::mux::mod` / `ade_ledger::mempool::policy` / `ade_ledger::mempool::canonicalize` / `ade_testkit::mempool::ingress_replay` / **`ade_testkit::governance::proposal_procedures_replay`** / the two `ade_core_interop` N-E bridges. **N-E:** canonicalizer body is grep-gated NO async / RNG / clock / `HashMap` / `HashSet` / `RwLock` / `Mutex`. **PP-S2:** the harness is a pure deterministic single-step `decode → encode` fold per fixture; no I/O, no clocks. | BLUE crates + standard library + ecosystem crates | `ade_runtime` (for `ade_testkit`); RED submodules in non-test paths. Results must never feed back into a BLUE authoritative decision. |
| **RED** | `ade_*` | No special header. Free to use clocks, I/O, async, `HashMap`, signing keys. The operator-action probe binaries in `crates/ade_core_interop/src/bin/` (currently `live_consensus_session`, `live_tx_submission_session`) follow the hermetic-default / `--connect`-live pattern and are `#[ignore]`-gated by closure-gate tests. **PP added no new probe binary.** | Any BLUE / GREEN crate or submodule (one-way) | Cannot be depended on by BLUE. |

### New module checklist

1. **Add to `Cargo.toml` workspace members** (if a new crate).
2. **Declare TCB color** by editing `.idd-config.json` `core_paths` if BLUE.
3. **CI script update obligations** — extend the relevant BLUE-scoped
   scripts; for governance-domain sub-grammar decoders, model the new
   CI gate on `ci_check_proposal_procedures_closed.sh` (5-guard shape).
4. **Add contract banner** (BLUE) to every `.rs` file.
5. **Add deny attributes** to `lib.rs` (BLUE).
6. **New canonical types:** add a `[[rules]]` block under family `T`
   in the invariant registry, plus a round-trip test. For new
   derived-Cardano sub-grammar closures, append `DC-LEDGER-1X` with
   bidirectional cross-ref to consumed discriminant rules.
7. **New operator-action probe binary:** add to
   `crates/ade_core_interop/src/bin/<name>.rs` following the
   `live_<surface>_session` naming + hermetic-default-plus-`--connect`-live
   shape; document in `<cluster>/CE-<id>_PROCEDURE.md`; capture
   evidence to `<cluster>/CE-<id>_<date>.log`.
8. **Cross-cluster obligation:** if a CE is split, follow the 5
   binding rules from the N-E full-close narrative.
9. **Run `cargo test --workspace` and the full CI script suite.**

### Phase 4 anticipated additions

- **PROPOSAL-PROCEDURES-DECODE — FULLY CLOSED at this HEAD**: code
  + CI gate + DC-LEDGER-11 registry rule + 17 PP-S1 tests + 4 PP-S2
  harness tests + cluster archived to
  `docs/clusters/completed/PROPOSAL-PROCEDURES-DECODE/`. Four
  declared `open_obligation` strengthenings carried as separable
  future seams: voting_procedures decode (OQ-1), ParameterChange.update
  nested decode (OQ-2), NewConstitution.raw nested decode (OQ-3),
  typed RewardAccount (OQ-4).
- **PHASE4-N-E — FULLY CLOSED**: code + CE-N-E-6 live N2N evidence
  + cluster archived. CE-N-E-7 + bulk-tx halves deferred to
  CE-NODE-N2C-LTX.
- **Tx-validity completeness follow-ups**: full `track_utxo=true`
  corpus; pre-Conway eras; the Conway block-body vkey-witness
  closure (carried).
- **Future node-binary cluster (`CE-NODE-N2C-LTX`)**: live N2C UDS
  server + N2N bulk-tx inbound listener.
- **Outbound tx propagation (post-N-E)**: declared non-goal in N-E.
- **Mempool bounds / shedding policy (Tier-5)**: declared non-goal in N-E.
- **`voting_procedures` (key 19) closed decode (PP OQ-1 follow-up)**:
  natural sibling cluster to PROPOSAL-PROCEDURES-DECODE.
- **`ParameterChange.update` nested decode (PP OQ-2 follow-up)**.
- **`NewConstitution.raw` nested decode (PP OQ-3 follow-up)**.
- **Typed `RewardAccount` (PP OQ-4 follow-up)**.
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
  `mempool_ingress` function body. The body must remain a pure
  pass-through to `admit(mempool, event.tx_bytes())`. No `String`,
  no `Box<dyn>`, no `#[non_exhaustive]` on `IngressSource` or
  `IngressEvent`. No second public production path into `admit`
  outside this chokepoint. No mutation of `MempoolState.accumulating`
  from outside `mempool/admit.rs`. No decode / re-encode of tx body
  bytes at this layer — verbatim flow into `admit`.
- **(PP specific — closed BLUE sub-grammar
  `decode_proposal_procedures` + closed type `ProposalProcedure`)**
  No reversion of `ConwayTxBody.proposal_procedures` back to
  `Option<Vec<u8>>` (CI guard 2). No silent-skip arm or catch-all
  accept in `decode_proposal_procedures` / `decode_proposal_procedure`
  / `decode_gov_action` / `decode_anchor`. No accept of an empty
  `proposal_procedures` set (CIP-1694 requires non-empty). No accept
  of trailing garbage / truncated procedure / invalid stake
  credential in `UpdateCommittee`. No struct-literal construction of
  `ProposalProcedure` outside `decode_proposal_procedure` (and its
  inline `#[cfg(test)]` builders), the testkit fixture builders
  (`crates/ade_testkit/...`), and integration tests
  (`crates/*/tests/`). No re-collapse of the DC-LEDGER-10
  `StakeCredential` discriminant inside the `UpdateCommittee` arm.
  No second public production decoder for the `proposal_procedures`
  sub-grammar (CI guard 3). No opaque pass-through at body codec
  key 20 (CI guard 4). No nested decoding of `voting_procedures`
  (key 19), `ParameterChange.update`, or `NewConstitution.raw` in
  this cluster's decoder — OQ-1/2/3 are scope locks. No typing of
  `proposal_procedure.return_addr` beyond `Vec<u8>` (OQ-4).

### GREEN (`ade_testkit` incl. `validity` / `tx_validity` / `mempool` + B3/B4/B5/OQ5/COMMITTEE/DREP corpora + **PP-S2 `governance` corpus**; `ade_network::lib` / `mux::mod`; `ade_runtime::consensus::{candidate_fragment, chain_selector}`; `ade_ledger::mempool::{policy, canonicalize}`; the two `ade_core_interop` N-E bridges)

- No nondeterminism that leaks into stored fixtures — fixtures must
  be byte-reproducible.
- No participation in authoritative outputs.
- No `HashMap` even in test helpers — `BTreeMap` only.
- No import of `ade_runtime` from `ade_testkit`.
- (`ade_runtime::consensus::chain_selector`) No comparison decision.
- (`ade_ledger::mempool::policy`) No call to `tx_validity`; no read
  of the accumulating state; no add/remove of a tx id (DC-MEM-02).
- (`ade_ledger::mempool::canonicalize`, N-E) No async / RNG / clock /
  `HashMap` / `HashSet` / `RwLock` / `Mutex`. The per-peer ordering
  rule is the load-bearing GREEN fairness contract.
- (`ade_testkit::mempool::ingress_replay`, N-E) Single-step fold
  only; no batching / parallel / out-of-order helpers.
- (`ade_core_interop::tx_submission` / `local_tx_submission`, N-E)
  Pure deterministic functions over their inputs; no I/O, no clocks.
- **(`ade_testkit::governance::proposal_procedures_replay`, PP-S2)**
  No I/O; no clocks; no nondeterminism in the synthetic corpus
  generator (fixtures must be byte-reproducible across builds). The
  round-trip property is single-step `decode → encode` per fixture;
  no batching or parallel asserts. Coverage MUST include all 7
  `GovAction` variants AND the DC-LEDGER-10 `UpdateCommittee`
  discriminant case (separate `KeyHash(h)` vs `ScriptHash(h)` fixtures
  with equal 28 bytes — distinct round-trip outputs are the
  certifier). Future strengthening: real-chain corpus extraction
  (per OQ-5 — declared as a possible future PP-S3 or successor
  cluster); when added, the harness MUST remain GREEN (no live
  network calls inside the harness — the corpus extractor runs
  offline against pre-staged chain data).

### RED (`ade_runtime`, `ade_node`, `ade_network::mux::transport`, `ade_network::session`, `ade_network::bin::capture_*`, `ade_runtime::consensus::genesis_parser`, `ade_core_interop` (incl. N-E S6 probe binary `live_tx_submission_session`), and the RED-behavior `ade_ledger::consensus_input_extract` scan)

- No direct mutation of `ade_ledger` state — all transitions go
  through `ade_ledger::rules::*`, the `block_validity` /
  `tx_validity` composers, or `mempool::ingress::mempool_ingress`
  (the Tier-1 wire-level chokepoint; direct `mempool::admit` calls
  from production source are CI-forbidden in N-E).
- No bypassing `ade_codec` to construct semantic types from raw bytes.
  **(PP-strengthened)** Constructing `ProposalProcedure` from raw
  bytes outside `decode_proposal_procedures` on the production path
  is CI-forbidden (guard 5).
- (`ade_runtime` specifically) No dep on `ade_ledger`. No leakage of
  `redb` types. No second public `chaindb` path.
- (`ade_network::mux::transport`) No protocol logic.
- (`ade_network::session`) Composition glue only.
- (`ade_network::bin::capture_*`) Live-interop tools only.
- (`ade_runtime::consensus::genesis_parser`) No re-derivation of the
  bootstrap anchor outside `compute_anchor_hash`.
- (`ade_ledger::consensus_input_extract`) Pure-over-bytes.
- (N-E live N2N operator-action session — the RED probe binary
  `live_tx_submission_session` + the RED half of CE-N-E-6) The live
  socket loop MUST funnel its delivered tx-byte events through the
  GREEN `ade_core_interop::tx_submission` bridge — MUST NOT carry a
  parallel admission path, MUST NOT call `admit` directly from
  production source, MUST NOT bypass `mempool_ingress`, MUST NOT
  branch the verdict on whether the bytes arrived over N2N or N2C.
- (Deferred RED operator-action surfaces — CE-NODE-N2C-LTX) The
  live N2C UDS server + N2N bulk-tx inbound listener belong to the
  future node-binary cluster.
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
  committee write-back, the N-E Tier-1 wire-level mempool ingress,
  and the **PROPOSAL-PROCEDURES-DECODE closed `proposal_procedures`
  sub-grammar at the Conway tx-body key 20 boundary (DC-LEDGER-11)**
  are all Tier-1 surfaces. **Cross-cluster obligation deferrals
  (CE-NODE-N2C-LTX) are NOT "we'll match it later" stubs**. **The
  four PROPOSAL-PROCEDURES-DECODE OQ-locked open obligations
  (voting_procedures, ParameterChange.update, NewConstitution.raw,
  typed RewardAccount) are NOT "we'll match it later" stubs either**
  — they are explicitly OQ-locked scope decisions recorded in
  `DC-LEDGER-11.open_obligation`, declared as separable future seams
  in this SEAMS §1, and tied to specific successor-cluster
  expectations in §5.

---

## Cross-references

- CODEMAP: `docs/ade-CODEMAP.md` — module-by-module authority table,
  upstream of this document. **Cross-reference check at this HEAD:**
  CODEMAP is being regenerated in parallel; pending the regen,
  CODEMAP may pin pre-PP HEAD. The new BLUE module
  `ade_codec::conway::governance` at exact path
  `crates/ade_codec/src/conway/governance.rs` exports the new
  closed entry points; the new closed type
  `ade_types::conway::governance::ProposalProcedure` at exact path
  `crates/ade_types/src/conway/governance.rs:93` is the new BLUE
  domain type; the new GREEN harness at
  `crates/ade_testkit/src/governance/proposal_procedures_replay.rs`
  is the new GREEN sibling. The next CODEMAP regen picks these up
  mechanically. CI count moves from 31 → 32 (`ci_check_proposal_procedures_closed.sh`).
- Invariant registry: `docs/ade-invariant-registry.toml` — rule
  families incl. T / CN / DC / OP / RO. **PP added:** `DC-LEDGER-11`
  (`enforced`, `ci_script = ci/ci_check_proposal_procedures_closed.sh`,
  `introduced_in = PROPOSAL-PROCEDURES-DECODE`, bidirectional
  `cross_ref = [DC-LEDGER-10]`); appended `DC-LEDGER-11` to
  `DC-LEDGER-10.cross_ref`; PP-S2 extended `DC-LEDGER-11.tests`
  with 4 harness test names. Total: 175 → 176 entries.
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
- Cluster ENACTMENT-COMMITTEE-WRITEBACK (closed).
- Cluster PHASE4-N-E (closed; archived at
  `docs/clusters/completed/PHASE4-N-E/`).
- **Cluster PROPOSAL-PROCEDURES-DECODE (CLOSED at this HEAD;
  archived to `docs/clusters/completed/PROPOSAL-PROCEDURES-DECODE/`)**:
  the cluster doc + slices `cluster.md, PP-S1.md, PP-S2.md`. WIRES
  AND CLOSES the Conway tx-body `proposal_procedures` (key 20)
  sub-grammar — typed `ConwayTxBody.proposal_procedures:
  Option<Vec<ProposalProcedure>>` + closed `ProposalProcedure`
  4-field struct + the single sanctioned BLUE decoder
  `decode_proposal_procedures` (closed grammar over CIP-1694
  `proposal_procedure = [coin, reward_account, gov_action, anchor]`)
  + the matching re-encoder `encode_proposal_procedures` (PreservedCbor
  round-trip authority) + the PP-S2 canonical synthetic corpus +
  replay harness covering all 7 `GovAction` variants and the
  DC-LEDGER-10 `UpdateCommittee` discriminant case. CONSUMES
  DC-LEDGER-10 unchanged through the `UpdateCommittee` arm;
  bidirectional `cross_ref` recorded in registry. Added one CI
  script (`ci_check_proposal_procedures_closed.sh` — 5 mechanical
  guards; count `31 → 32`). Added one derived-Cardano registry
  rule (`DC-LEDGER-11`; registry total `175 → 176`). **Declared
  non-goals carried to future clusters as `open_obligation` on
  DC-LEDGER-11**: `voting_procedures` (key 19) decode (OQ-1 —
  natural sibling cluster), `ParameterChange.update` nested decode
  (OQ-2), `NewConstitution.raw` nested decode (OQ-3), typed
  `RewardAccount` for `proposal_procedure.return_addr` (OQ-4 —
  would also retype `TreasuryWithdrawals.withdrawals` element type).
  No new operator-action probe binary, no new live-evidence log,
  no new cross-cluster obligation.
- **Future obligation: `CE-NODE-N2C-LTX`** — the node-binary
  cluster's live N2C UDS server + N2N bulk-tx inbound listener;
  carried from N-E.
