# Seams — Where New Work Can Attach (Ade)

> **Status:** Living architectural document. Regenerated; not hand-edited.
> Per-project instance of `~/.claude/methodology/templates/seams.md`.

> 11 crates, **40 CI checks** at HEAD (`694dd74`).
> Reads CODEMAP for the module list and TCB colors; reads the invariant
> registry (`docs/ade-invariant-registry.toml`) for rule IDs; reads the
> Phase 4 cluster plan (`docs/active/phase_4_cluster_plan.md`), the
> closed N-D / N-A / N-B / N-E / B1 / B2 / B3 / B4 / B5 cluster docs,
> the OQ5-CREDENTIAL-FIDELITY, COMMITTEE-CRED-FIDELITY, DREP-VOTE-FIDELITY,
> ENACTMENT-COMMITTEE-FIDELITY, ENACTMENT-COMMITTEE-WRITEBACK and
> PROPOSAL-PROCEDURES-DECODE cluster docs, and the **just-closed and
> staged PHASE4-N-C cluster doc + S1..S7 slice docs**
> (`docs/clusters/PHASE4-N-C/cluster.md` + `N-C-S{1..7}.md` +
> `CE-N-C-8_PROCEDURE.md`).
>
> **This is the PHASE4-N-C FULL CLOSE refresh (HEAD `694dd74`).** The
> previous SEAMS (HEAD `928c2be`) pinned the PROPOSAL-PROCEDURES-DECODE
> full-close state. Seven N-C slices have landed between that revision
> and this one and close the producer half of the bounty:
>
> 1. **N-C-S1 (commit `ea9770e`)** ships RED signing primitives
>    (`ade_runtime::producer::signing::{vrf_prove, kes_sign,
>    kes_update}`), the closed RED key types (`VrfSigningKey`,
>    `KesSecret`, `ColdSigningKey` — `zeroize`-on-drop, no `pub`
>    raw-byte accessors), the cardano-cli `*.skey` loader
>    (`ade_runtime::producer::keys::{load_vrf_signing_key_skey,
>    load_kes_signing_key_skey, load_cold_signing_key_skey}`), and the
>    CI gate `ci/ci_check_private_key_custody.sh`. Registry rules
>    `DC-CRYPTO-03/04/05` + `OP-OPS-04` introduced (status
>    `enforced`).
> 2. **N-C-S2 (commit `4cf4b65`)** ships the BLUE
>    `ade_core::consensus::opcert_validate::opcert_validate` chokepoint
>    (closed `OpCertError` sum: `CounterRegression`, `CounterRepeat`,
>    `PeriodMismatch`, `BadColdSignature`, shape rejects) AND the
>    new closed-grammar opcert encoder authority
>    `ade_codec::shelley::opcert::{encode_opcert, decode_opcert}`
>    (single sanctioned byte authority — both header CBOR
>    (`shelley::block`) and standalone opcert CBOR delegate here;
>    the prior inline header-path emit is forbidden). Registry rules
>    `DC-CONS-11/12` introduced (`enforced`); CI gate
>    `ci/ci_check_opcert_closed.sh` introduced.
> 3. **N-C-S3 (commit `8312690`)** ships the BLUE producer core:
>    the closed canonical input value `ade_ledger::producer::state::ProducerTick`
>    (14 fields; no `#[non_exhaustive]`; no private-key fields by CI),
>    the pure BLUE transition `ade_ledger::producer::forge::forge_block`
>    with closed sums `ForgeError`, `ForgeEffects`, `ForgedBlock`, the
>    leader-check delegation to the shared validator function
>    `is_leader_for_vrf_output` (NC-VRF-3 — single source of leader
>    truth), and the **tx-admissibility prefix gate** (re-`admit` over
>    `base_state` in canonical accumulating order). CI gates
>    `ci/ci_check_forge_purity.sh` + `ci/ci_check_no_private_keys_in_corpus.sh`.
>    Registry rules `DC-CONS-13/14/15`, `DC-LEDGER-12` introduced
>    (`enforced`).
> 4. **N-C-S4 (commit `4fd714c`)** unifies the body-hash recipe into a
>    **single canonical authority**:
>    `ade_ledger::block_body_hash::block_body_hash_from_buckets`
>    (4-bucket recipe over preserved CBOR — `blake2b_256` of
>    `blake2b_256` per bucket, concatenated). Both
>    `ade_ledger::block_validity::header_input` (validator
>    recomputation) and `ade_ledger::producer::forge::forge_block`
>    (producer emission) hash through this function — closing the
>    producer/validator encoder bifurcation. Registry rule
>    `DC-CONS-16` introduced (`enforced`); CI gate
>    `ci/ci_check_no_producer_body_encoder.sh` introduced.
> 5. **N-C-S5 (commit `aa7a7dd`)** ships the **type-level broadcast
>    gate** `ade_ledger::producer::self_accept::{self_accept,
>    AcceptedBlock, SelfAcceptError}`. `AcceptedBlock` is a newtype
>    whose only field (`bytes: Vec<u8>`) is private and whose only
>    constructor is the `Ok(...)` arm of `self_accept` — which calls
>    the canonical BLUE validator chokepoint
>    `ade_ledger::block_validity::transition::block_validity`. RED
>    `broadcast::BroadcastQueue::enqueue(&mut self, AcceptedBlock)`
>    consumes the token by value; the producer cannot broadcast bytes
>    its own validator would reject. Registry rule `CN-CONS-07`
>    introduced (`enforced`); CI gate `ci/ci_check_self_accept_gate.sh`
>    introduced (6 mechanical guards).
> 6. **N-C-S6 (commit `58678af`)** ships the RED scheduler core
>    `ade_runtime::producer::scheduler::{scheduler_step, SchedulerInput,
>    SchedulerEffect, SchedulerState, SchedulerHaltReason}` (closed
>    sums; pure RED state transition mirroring N-B's `process_stream_input`
>    shape — wall-clock and I/O live in the outer driver), the GREEN
>    tick-assembler `ade_runtime::producer::tick_assembler::{assemble_tick,
>    TickInputs, TickAssemblyError}` (pure function stitching signed
>    artifacts + mempool snapshot into a canonical `ProducerTick`), and
>    the RED `ade_runtime::producer::broadcast::{BroadcastQueue,
>    BroadcastError}` FIFO queue (`enqueue` takes `AcceptedBlock` by
>    value — type-level gate). Registry rule `OP-OPS-05` introduced
>    (`enforced`); CI gate `ci/ci_check_scheduler_closure.sh` introduced.
> 7. **N-C-S7 (commit `694dd74`)** ships the mechanical cross-impl
>    adapter `ade_testkit::producer::cross_impl_adapter`
>    (decode-round-trip + body-hash binding via S4's authority +
>    structural field agreement; covers the bytes-shape claim) and
>    the operator-action probe binary
>    `ade_core_interop::bin::live_block_production_session` (third
>    instance of the operator-action probe binary pattern; the
>    crypto-level cross-impl claim — real KES/VRF over N2N — is
>    captured live). Registry rule `CN-CONS-06` introduced
>    (`enforced` with `open_obligation = blocked_until_operator_stake_available`
>    — same precedent as OP-OPS-04); CI gate
>    `ci/ci_check_producer_corpus_present.sh` introduced. CE-N-C-8
>    procedure documented at
>    `docs/clusters/PHASE4-N-C/CE-N-C-8_PROCEDURE.md`.
>
> **THE KEY FULL-CLOSE DELTAS.** Three §1 candidate rows from the
> prior SEAMS flip from "candidate" to "wired & closed":
>
> - **Forge-block inputs (mempool + state + slot + KES + VRF)** →
>   wired through canonical `ProducerTick` value (BLUE).
> - **Operator block-production trigger** → wired through closed
>   `SchedulerInput::SlotTick { slot, inputs: TickInputs }` (RED).
> - **header→body bridge (forge/fetch-winning header triggers a
>   full-block decision)** → wired via the producer's own
>   `self_accept` (forge bytes -> validator decision before
>   broadcast). The forge-from-receive-side header bridge for
>   externally-arriving headers remains a candidate (see §1).
>
> Counts at this refresh: **+8 CI scripts** (32 → 40:
> `ci_check_private_key_custody.sh`, `ci_check_opcert_closed.sh`,
> `ci_check_forge_purity.sh`, `ci_check_no_private_keys_in_corpus.sh`,
> `ci_check_no_producer_body_encoder.sh`, `ci_check_self_accept_gate.sh`,
> `ci_check_scheduler_closure.sh`, `ci_check_producer_corpus_present.sh`);
> **+14 registry rules** (`DC-CRYPTO-03/04/05`, `DC-CONS-11/12/13/14/15/16`,
> `DC-LEDGER-12`, `CN-CONS-06`, `CN-CONS-07`, `OP-OPS-04`,
> `OP-OPS-05`); **+1 new BLUE crate-internal authority surface**
> (`ade_ledger::producer` — submodules `forge`, `self_accept`,
> `state`); **+1 new BLUE chokepoint** (`opcert_validate` in
> `ade_core::consensus`); **+1 new BLUE closed sub-grammar
> encoder/decoder pair** (`ade_codec::shelley::opcert::{encode_opcert,
> decode_opcert}`); **+1 unified BLUE body-hash authority**
> (`block_body_hash::block_body_hash_from_buckets` — single
> canonical recipe consumed by both producer and validator); **+1
> new RED crate-internal authority surface** (`ade_runtime::producer`
> — submodules `signing`, `keys`, `scheduler`, `broadcast`); **+1
> new GREEN crate-internal authority surface**
> (`ade_runtime::producer::tick_assembler`); **+1 new GREEN
> harness** (`ade_testkit::producer::cross_impl_adapter`); **+1
> new operator-action probe binary**
> (`live_block_production_session` — third in the family alongside
> `live_consensus_session` (N-B) and `live_tx_submission_session`
> (N-E)); **+1 new live-evidence procedure doc**
> (`CE-N-C-8_PROCEDURE.md`); **0 new operator-action live-evidence
> log artifacts at this HEAD** — CE-N-C-8 is recorded
> `blocked_until_operator_stake_available` per registry
> `CN-CONS-06.open_obligation`.

Ade is a Cardano block-producing node. Its closure surface is dominated
by two facts:

1. The Cardano protocol fixes wire bytes and hashes for hash-critical
   paths (Tier 1 — must-conform). New work that touches those bytes
   has essentially no degrees of freedom.
2. Everything operator-facing — storage layout, query API, telemetry,
   packaging — is Tier 5: deliberate divergence "in our own image"
   (per `docs/active/CE-79_tier5_addendum.md`).

This document names where the system opens and where it stays closed.

**PHASE4-N-C is fully closed at this HEAD.** Forge-block inputs,
operator block-production trigger, body-hash authority unification,
and the type-level self-accept gate are all wired and CI-defended.
The crypto-level cross-impl claim (CE-N-C-8) is
`blocked_until_operator_stake_available` per `CN-CONS-06.open_obligation`.

**PROPOSAL-PROCEDURES-DECODE remains fully closed** (carried).

**PHASE4-N-E (Tier 1 wire-level mempool ingress) remains fully closed**
(carried).

**PHASE4-B3..B5, OQ5 / COMMITTEE / DREP / ENACTMENT-COMMITTEE-WRITEBACK**
all remain closed (carried).

---

## 1. Surface Reduction Rules

> External inputs reduce to canonical form before entering authoritative
> pipelines. At HEAD there are **seven** fully-wired *external* ingress
> surfaces (block bytes, Plutus script bytes, snapshot bytes, Ouroboros
> mux frames, genesis JSON bundles, chain-selector stream inputs, and
> the N-E wire-level mempool ingress), plus the **internal composition
> roots** (`block_validity` from B1, `tx_validity` from B2, the BLUE
> chokepoint `mempool_ingress` from N-E S1, **and — newly closed at
> this HEAD — the BLUE `forge_block` producer transition and the BLUE
> `self_accept` broadcast gate**), the **mempool admission gate**
> (`mempool::admit`), the **consensus-input extraction surface**
> (snapshot `state` CBOR tail-scan from B1), the **`proposal_procedures`
> sub-grammar entry point**, **and — newly closed at this HEAD — the
> operator-action `SchedulerInput` ingress (slot ticks + chain
> advances + signed-artifact tick inputs)**.

### Surface: Forge-block transition (NEW in N-C-S3/S4 — RED→BLUE seam)

```
Surface: A canonical ProducerTick value, assembled by GREEN
         tick_assembler from RED signing-primitive outputs against
         a base LedgerState + MempoolState
Reduces to: (ForgedBlock, Vec<ForgeEffects>) | ForgeError
            (closed sums; no String payloads — replay-byte-stable)
Pipeline (fixed step ordering — no reorder, no shortcut):
  1. Width check: mempool_tx_bytes.len() == mempool.accepted().len()
  2. opcert_validate(opcert, cold_vk, kes_period, prev_opcert_counter)
       (BLUE — shared chokepoint with header validation; closed
        OpCertError sum)
  3. KES signature length defense-in-depth
  4. Leader check: is_leader_for_vrf_output(leader_answer, vrf_output)
       (the same BLUE function the validator uses — NC-VRF-3)
  5. Admit-prefix re-validation: replay mempool::admit from
     MempoolState::new(base_state.clone()) over mempool_tx_bytes in
     order; every step must Admitted; resulting accepted-id list
     must byte-equal tick.mempool.accepted()
  6. Per-tx component split (split_conway_tx_components)
  7. Build the four body buckets (preserved bytes; never re-encoded)
  8. Body hash via block_body_hash_from_buckets (single canonical
     authority — same recipe the validator recomputes through)
  9. Build ShelleyHeader + ShelleyBlock; encode via ade_encode
  10. Return ForgedBlock + ForgeEffects::ReadyForSelfAccept
Cross-surface state sharing: none — forge is pure; replay equivalence
  is byte-identical over identical ProducerTick streams.
```

**Rule.** `forge_block` is the **single producer-side composition
root**. The leader-check is delegated to the validator's
`is_leader_for_vrf_output` (no producer-side fork — NC-VRF-3). The
body-hash recipe lives in `block_body_hash::block_body_hash_from_buckets`
(single canonical authority — DC-CONS-16). Forge is pure (no clock,
no rand, no I/O, no `HashMap`, no `std::env`, no `std::fs` — defended
by `ci/ci_check_forge_purity.sh`); replay corpora carry only signed
artifacts (`VrfProof`, `KesSignature`, `OpCert`), never private keys
(DC-CONS-14, defended by `ci/ci_check_no_private_keys_in_corpus.sh`).
**New work** that wants to forge attaches by producing a canonical
`ProducerTick` — not by calling into BLUE state, not by adding a
parallel encode-block-body path. The ProducerTick struct is closed
(no `#[non_exhaustive]`; private-key fields forbidden by CI).

### Surface: Self-accept broadcast gate (NEW in N-C-S5 — BLUE→RED seam)

```
Surface: Forged block bytes (Vec<u8>) emitted by forge_block, paired
         with the same (LedgerState, PraosChainDepState, EraSchedule,
         LedgerView) context that drove the forge
Reduces to: AcceptedBlock                          (BLUE token —
            Result type: Result<AcceptedBlock,       constructor lives
                                SelfAcceptError>)    only in self_accept)
Pipeline (fixed step ordering — matches receive-side validator exactly):
  1. block_validity(ledger, chain_dep, era_schedule, ledger_view,
                    forged_bytes)
       — runs the full validator chain: decode + header validate +
         body-hash bind + body apply.
  2. On Valid -> Ok(AcceptedBlock { bytes: forged_bytes.to_vec() })
  3. On Invalid -> Err(SelfAcceptError::Rejected(error))
Cross-surface state sharing: none — pure transition.
```

**Rule.** `self_accept` is the **single sanctioned production
constructor of `AcceptedBlock`**. The newtype's only field is private,
and there is **exactly one** `pub fn ... -> Result<AcceptedBlock, ...>`
in the workspace (defended by `ci/ci_check_self_accept_gate.sh`
guard 1b). `RED broadcast::BroadcastQueue::enqueue(&mut self,
block: AcceptedBlock)` consumes the token **by value** — type-level
gate: RED cannot construct a broadcastable value without passing
through BLUE validation. `self_accept` MUST NOT re-implement any
validator sub-step (`validate_and_apply_header`, `decode_block`,
`block_body_hash` are forbidden in `self_accept.rs` production source
by guard 5). `SelfAcceptError` is a closed sum (no `#[non_exhaustive]`,
no `String`-bearing variant). **New work** that adds an authority
gate above broadcast attaches by producing or consuming `AcceptedBlock`
— not by constructing it elsewhere, not by adding a parallel
broadcast-eligible token.

### Surface: Scheduler input ingress (NEW in N-C-S6 — operator-side trigger)

```
Surface: An ordered stream of SchedulerInput events delivered by the
         outer driver (binary layer reading wall-clock + chain-sync)
Reduces to: SchedulerEffect (closed 4-variant sum)
            { EnqueueBroadcast(AcceptedBlock) | SilentNonLeader{slot}
            | HaltOnInvariant{slot, reason: SchedulerHaltReason}
            | HaltOnAssembly{slot, reason: TickAssemblyError} }
Pipeline (fixed step ordering — no reorder, no shortcut):
  1. caller wraps each external trigger in a SchedulerInput variant:
       - SlotTick { slot, inputs: TickInputs }     (RED→GREEN→BLUE→BLUE→RED)
       - ChainAdvanced { ledger, chain_dep, mempool }
                                                   (GREEN—pure mutation)
  2. scheduler_step(state, input, ledger_view) -> (state', effects)
       (RED, pure; SchedulerState carries last_seen_slot +
        prev_opcert_counter + halted)
  3. On SlotTick: assemble_tick (GREEN) -> ProducerTick;
                  forge_block (BLUE) -> ForgedBlock;
                  self_accept (BLUE) -> AcceptedBlock;
                  EnqueueBroadcast(token) effect.
  4. On forge / self-accept failure: HaltOnInvariant — scheduler
     marks `halted = Some(reason)`; subsequent SlotTicks are ignored
     and re-emit the original halt reason.
Cross-surface state sharing: SchedulerState carries the baseline
  (ledger, chain_dep, mempool, era_schedule); refreshed only by
  ChainAdvanced. Once halted, deterministic re-emission.
```

**Rule.** `SchedulerInput` is a **closed 2-variant sum**:
`SlotTick { slot, inputs }` and `ChainAdvanced { .. }`. Every
external trigger that drives forging must reduce to one of these two
variants. `SchedulerEffect` is a **closed 4-variant sum**. The
scheduler is **pure RED state transition** (mirrors N-B's
`process_stream_input` shape — wall-clock + I/O live in the outer
driver, never inside `scheduler_step`). **New work** that adds an
operator trigger attaches by producing a `SchedulerInput` — not by
calling forge or self-accept directly, not by adding a parallel
slot-loop. **GREEN seam**: `assemble_tick(slot, base_state, mempool,
&TickInputs) -> Result<ProducerTick, TickAssemblyError>` is the only
sanctioned path from RED signing-primitive outputs to the canonical
BLUE `ProducerTick` value. The pure-function property is the
load-bearing GREEN contract (identical `(slot, base_state, mempool,
inputs)` MUST produce byte-identical ticks across replays —
DC-CONS-13 / DC-CONS-14).

### Surface: Mempool ingress (Tier-1 wire-level — wired in N-E; unchanged at N-C HEAD)

```
Surface: A candidate transaction delivered by a real cardano-node N2N
         peer (via tx-submission2) or by a real cardano-cli over the
         N2C local-tx-submission UDS, against the mempool's
         accumulating LedgerState
Reduces to: (MempoolState, AdmitOutcome)
            { Admitted { tx_id } | Rejected { class, error } }
Pipeline (fixed step ordering — no reorder, no shortcut):
  1. RED transport (ade_network::mux::transport)
  2. BLUE wire grammar (N-A) — tx_submission / local_tx_submission
  3. GREEN bridge (N-E S4 / S5) — ingest_n2n_events / ingest_n2c_events
  4. GREEN canonicalizer (N-E S3) — canonicalize_peer_streams
  5. BLUE chokepoint (N-E S1) — mempool_ingress (verbatim pass-through)
  6. BLUE admission gate (B2) — admit (verdict equals tx_validity)
Cross-surface state sharing: the mempool's accumulating LedgerState
  is the only state carried across consecutive `mempool_ingress` calls.
```

**Rule.** Carried unchanged from the N-E revision. **N-C note:** the
mempool snapshot consumed by `forge_block` is precisely the
`MempoolState` produced by this pipeline — `tick.mempool.accepted()`
is the canonical accumulating order forge respects (DC-LEDGER-12).

### Surface: Conway tx-body `proposal_procedures` sub-grammar (closed entry point — carried unchanged from PROPOSAL-PROCEDURES-DECODE)

Carried unchanged from the prior revision. The
`decode_proposal_procedures` BLUE entry point + closed
`ProposalProcedure` struct + `ci_check_proposal_procedures_closed.sh`
gate + DC-LEDGER-11 rule remain. N-C did not touch this surface.

### Surface: Single-tx validity (composition root — wired in B2; unchanged at N-C HEAD)

```
Surface: A single Conway transaction (full tx CBOR
         [body, witness_set, is_valid, aux_data]) decided against
         a LedgerState
Reduces to: TxValidityVerdict { Valid { tx_id, applied } |
                                Invalid { class, error } }
```

**Rule.** Carried. **N-C note:** `tx_validity` is invoked transitively
by `forge_block` only through `admit`'s prefix re-validation (step 4
of the forge pipeline); the composer itself is untouched.

### Surface: Mempool admission (Tier-1 gate — wired in B2; unchanged)

**Rule.** Carried. **N-C note:** `admit` is **read-only** from the
producer side. `forge_block` re-runs `admit` over `tick.mempool_tx_bytes`
against a fresh `MempoolState::new(tick.base_state.clone())` (a
side-effect-free re-validation), but **does not call `admit` against
the running mempool** — the producer never mutates production mempool
state. DC-LEDGER-12 is the rule.

### Surface: Full block validity (composition root — wired in B1; consumed by N-C self_accept)

**Rule.** Carried. **N-C note:** `block_validity` is the single
chokepoint `self_accept` wraps (CN-CONS-07). The unified body-hash
recipe at `ade_ledger::block_body_hash::block_body_hash_from_buckets`
is the **single canonical authority** consumed by both
`block_validity::header_input` (validator recomputation) and
`producer::forge::forge_block` (producer emission). DC-CONS-16
defends the no-bifurcation property
(`ci_check_no_producer_body_encoder.sh`).

### Surface: Block bytes, Plutus script bytes, Snapshot bytes, Consensus-input extraction, Ouroboros mux frames, Genesis JSON bundles, Chain-selector stream inputs (carried)

All seven external ingress surfaces are unchanged at this HEAD. **N-C
note (mux frames):** the producer's broadcast queue
(`ade_runtime::producer::broadcast::BroadcastQueue`) is the upstream
side of a future N-A handoff into the N2N block-fetch server / chain-sync
extension. The handoff itself is N-A successor scope; the broadcast
queue's `dequeue() -> Option<AcceptedBlock>` is the BLUE→RED→wire seam.

### Candidates — surfaces not yet wired (Phase 4 N-F, B+ residuals; N-C+ residuals; PP open obligations)

The following surfaces are named in the Phase 4 plan / B+ planning /
the PP open-obligation set but have no source today. They are listed
so future slice docs can attach without reinventing the reduction
step. **Each is a candidate seam pending confirmation at cluster
entry.**

- **N-C-S3 WIRED AND CLOSED the prior revision's "Forge-block inputs"
  candidate** — removed (now `ProducerTick` + `forge_block`).
- **N-C-S6 WIRED AND CLOSED the prior revision's "Operator
  block-production trigger" candidate** — removed (now `SchedulerInput`
  + `scheduler_step`).
- **N-C-S5 WIRED AND CLOSED the producer half of the
  "header→body bridge" candidate** — the producer-side self-accept
  bridge consumes `block_validity` directly. The remaining half —
  routing an externally-arriving header through fork-choice to
  trigger a full-block decision on the fetched body — remains a
  candidate (B1+; see below).
- **PROPOSAL-PROCEDURES-DECODE remains closed** (carried). The four
  PP open obligations remain separable candidate seams (carried).
- **PHASE4-N-E remains closed** (carried).

| Cluster | Surface | Expected reduction target | Expected chokepoint | Confidence |
|---------|---------|---------------------------|---------------------|------------|
| **PHASE4-N-C** *(FULLY CLOSED at this HEAD — mechanical close; live half blocked_until_operator_stake_available)* | **Producer pipeline: ProducerTick → ForgedBlock → AcceptedBlock → broadcast queue** | `(SchedulerState, Vec<SchedulerEffect>)` per slot tick; `AcceptedBlock` on success | **DONE:** `ade_ledger::producer::{forge::forge_block, self_accept::self_accept, state::ProducerTick}` (BLUE); `ade_runtime::producer::{signing, keys, scheduler, broadcast, tick_assembler}` (RED+GREEN); `ade_codec::shelley::opcert::{encode_opcert, decode_opcert}` (BLUE closed-grammar); `ade_core::consensus::opcert_validate::opcert_validate` (BLUE chokepoint); `ade_ledger::block_body_hash::block_body_hash_from_buckets` (single canonical body-hash authority); CI gates `ci_check_private_key_custody.sh`, `ci_check_opcert_closed.sh`, `ci_check_forge_purity.sh`, `ci_check_no_private_keys_in_corpus.sh`, `ci_check_no_producer_body_encoder.sh`, `ci_check_self_accept_gate.sh`, `ci_check_scheduler_closure.sh`, `ci_check_producer_corpus_present.sh`. Registry rules `DC-CRYPTO-03/04/05`, `DC-CONS-11/12/13/14/15/16`, `DC-LEDGER-12`, `CN-CONS-06/07`, `OP-OPS-04/05` (`enforced`). Tests: 17 named tests across S1..S7 plus replay corpus. | **wired & closed in PHASE4-N-C (mechanical half + structural cross-impl); crypto-level cross-impl awaiting operator stake** |
| **CE-N-C-8 (cross-cluster obligation introduced in N-C S7; operator-action live evidence)** | **Live N2N block-fetch acceptance by a real cardano-node peer of an Ade-forged block** | The crypto-level cross-impl claim (real KES/VRF signatures observed over the wire) — same operator-action evidence pattern as CE-N-B-6 and CE-N-E-6 | The future evidence-capture pass via `live_block_production_session` against a real preprod / preview cardano-node; procedure at `docs/clusters/PHASE4-N-C/CE-N-C-8_PROCEDURE.md`; output `CE-N-C-LIVE_<date>.log`. | **deferred operator-action obligation — `blocked_until_operator_stake_available` per `CN-CONS-06.open_obligation`** |
| **N-C+ (declared non-goal in N-C cluster doc; OQ-4 lock — separable future seam)** | **TPraos producer (Shelley..Alonzo full-block production)** | A TPraos-flavored `ProducerTick` arm + per-era body buckets | Extend `forge_block` to a closed `era` dispatch; today Conway/Praos only. | candidate (declared non-goal — explicit OQ-4 lock) |
| **N-A+ (N-C handoff target)** | **N2N producer-side block-fetch server role + chain-sync extension** (delivery of broadcast-queued `AcceptedBlock` bytes to peers) | An outbound `BlockFetchMessage::Block(bytes)` stream emitted from the producer's `BroadcastQueue::dequeue()` output | A new BLUE / GREEN outbound bridge in `ade_network` consuming the queue; the producer side is already in place. | candidate (declared non-goal in N-C; binary layer + N-A successor) |
| **CE-NODE-N2C-LTX (cross-cluster obligation carried from N-E)** | **Live N2C UDS server + N2N bulk-tx inbound listener** | The deferred halves of CE-N-E-7 + CE-N-E-6 | The future node-binary cluster ships the live socket loops. | **deferred cross-cluster obligation (NOT an open seam in N-E)** |
| **PP OQ-1..OQ-4 (NEW separable seams — declared open obligations on DC-LEDGER-11)** | voting_procedures decode / ParameterChange.update nested / NewConstitution.raw nested / typed RewardAccount | per OQ | per OQ | candidate (carried) |
| B+ (full tx UTxO scope) | Full-scope single-tx validity over real resolved UTxO | `TxValidityVerdict` at `track_utxo=true` | `tx_validity` (existing) | candidate |
| B+ (Conway body witness depth) | **Conway block-body vkey-witness closure** — `project_conway_body_witness_gap` | `BlockValidityVerdict` whose body authority runs the same closure as `tx_phase_one` | wire `tx_phase_one` / `verify_required_witnesses` into the Conway block-body path in `rules.rs` | candidate (B2-carried) |
| B+ (pre-Conway tx) | Pre-Conway single-tx validity | `TxValidityVerdict` via per-era body decode + per-era `SignerSource` | extend `decode_tx` + add the era arm to `required_signers` | candidate |
| B1+ (header→body bridge — receive-side) | Externally-arriving header triggering a full-block decision on the fetched body | `block_validity(...)` over the fetched body | `ade_node` composition layer joining `process_stream_input` and `block_validity` | candidate (B1-carried; N-C closed the **producer-side** half via self_accept) |
| B1+ (pre-Babbage block) | TPraos full-block validity (Shelley..Alonzo) | `BlockValidityVerdict` via a TPraos `HeaderInput` projection | extend `block_validity::decode_block` to build `HeaderVrf::Tpraos` headers | candidate |
| N-F | LSQ semantic dispatch (LocalStateQuery payloads) | Internal Query enum (closed, not yet defined) | Single dispatch fn that consumes `LocalStateQueryMessage` opaque-bytes payloads | candidate |
| N-F | LocalTxMonitor semantic dispatch | Mempool-snapshot Query/Reply enums | Single dispatch fn that consumes `LocalTxMonitorMessage` opaque-bytes payloads | candidate |
| N-B+ | Live cardano-node session driver (for `ade_core_interop::live_consensus_session`) | `StreamInput` translated from `ChainSyncMessage` and `BlockFetchMessage` events | Composition layer in `ade_core_interop` | candidate |

### Operator-action evidence (live-wire artifacts — not BLUE seams)

The Ade workspace closes Tier-1 wire-level seams in two halves: a
mechanical / GREEN half (code + harness + CI gates that the workspace
itself can certify on every push) and a **live-wire operator-action
half** (a real peer / client at the other end of a real socket
producing bytes Ade has never seen).

**At this HEAD two live-evidence logs are committed**, one
cross-cluster obligation is carried from N-E, and **N-C adds one
new operator-action procedure that is `blocked_until_operator_stake_available`**.

| Procedure | Evidence-log artifact | Status at HEAD | What it asserts | TCB |
|-----------|----------------------|----------------|------------------|-----|
| `docs/clusters/completed/PHASE4-N-B/CE-N-B-6_PROCEDURE.md` | `docs/clusters/completed/PHASE4-N-B/CE-N-B-6_<date>.log` | **CAPTURED** (carried from N-B close) | Real cardano-node N-B follow-mode tip agreement | RED operator action |
| `docs/clusters/completed/PHASE4-N-E/CE-N-E-6_PROCEDURE.md` | `docs/clusters/completed/PHASE4-N-E/CE-N-E-6_2026-05-25.log` | **CAPTURED** (carried from N-E close) | Outbound-client probe against a real preprod N2N relay | RED operator action |
| `docs/clusters/completed/PHASE4-N-E/CE-N-E-7_PROCEDURE.md` | (deferred) `CE-NODE-N2C-LTX_<date>.log` in the future node-binary cluster | **DEFERRED to CE-NODE-N2C-LTX** | Real `cardano-cli transaction submit` to Ade over the N2C UDS | RED operator action (deferred) |
| **`docs/clusters/PHASE4-N-C/CE-N-C-8_PROCEDURE.md` (NEW in N-C-S7)** | **(pending)** `docs/clusters/PHASE4-N-C/CE-N-C-LIVE_<date>.log` | **`blocked_until_operator_stake_available`** (per `CN-CONS-06.open_obligation`) | An Ade-forged block carrying real cardano-cli-skey-signed KES + VRF + opcert sigma is accepted by a real cardano-node peer when delivered via N2N. Crypto-level cross-impl claim (the bytes-shape claim is mechanically closed by `cross_impl_adapter`). | RED operator action |

**Operator-action probe binaries (RED — `ade_core_interop::bin::*`).**
At this HEAD there are **three** such binaries:

| Binary | Slice | Live-evidence target | Status |
|--------|-------|----------------------|--------|
| `live_consensus_session` (PHASE4-N-B) | N-B | CE-N-B-6 (live chain-sync follow-mode tip agreement) | captured |
| `live_tx_submission_session` (PHASE4-N-E S6) | N-E S6 | CE-N-E-6 (live N2N tx-submission2 outbound-client probe) | captured |
| **`live_block_production_session` (PHASE4-N-C S7) — NEW** | N-C S7 | CE-N-C-8 (live N2N block-fetch acceptance by cardano-node) | **blocked_until_operator_stake_available** |

**Pattern.** Hermetic default mode (readiness probe that runs in CI
without network access — gated `#[ignore]`); plus a `--connect <peer>`
live pass that the operator runs against a real cardano-node peer.
The binary's evidence log is committed alongside the `_PROCEDURE.md`
in the cluster directory. **N-C strengthens the pattern with the
"blocked_until_operator_stake_available" status**: a third closure
mode (beyond captured / deferred) for live evidence whose blocker is
not Ade-internal — testnet SPO stake registration is an external
dependency the operator must provision. The pattern follows OP-OPS-04's
precedent.

**These are evidence-log patterns, not BLUE seams.**

User confirmation needed for each candidate at cluster entry. **The
most load-bearing remaining candidates for the bounty** are
**CE-N-C-8** (the live cardano-node acceptance), the **receive-side
header→body bridge**, **CE-NODE-N2C-LTX** (the deferred live N2C UDS
server + N2N bulk-tx inbound listener), and the four
**PROPOSAL-PROCEDURES-DECODE open obligations**.

---

## 2. Data-Only vs. Authoritative Layers

Ade has **fifteen** authoritative domains. **PHASE4-N-C added one new
domain — block production authority** — a new BLUE composition root
(`forge_block`) consuming a canonical input value (`ProducerTick`),
gated above by a closed self-accept bridge (`self_accept` /
`AcceptedBlock`), and assembled below by a closed RED / GREEN
scheduler / tick-assembler stack. Prior cluster narratives are
preserved unchanged below.

### Block production authority (NEW in PHASE4-N-C)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **RED signing primitives (S1)** | `ade_runtime::producer::signing::{vrf_prove, kes_sign, kes_update, VrfSigningKey, KesSecret, ColdSigningKey, SigningError}` | RED | Holds in-memory private keys; `zeroize`-on-drop; no `pub` raw-byte accessors. Forbidden in BLUE / codec / types / ledger / crypto public APIs by `ci_check_private_key_custody.sh`. |
| **RED key loader (S1)** | `ade_runtime::producer::keys::{load_vrf_signing_key_skey, load_kes_signing_key_skey, load_cold_signing_key_skey, KeyLoadError}` | RED | Disk reads of cardano-cli `*.skey` text envelopes; decoding into RED in-memory secrets. Operator-provided keys only (Ade does NOT generate — OP-OPS-04). |
| **BLUE closed-grammar opcert authority (S2)** | `ade_codec::shelley::opcert::{encode_opcert, decode_opcert, OpCertCodecError}` | BLUE | Single producer-side opcert byte authority. Standalone 4-tuple `[hot_vkey:bstr32, sequence_number:uint, kes_period:uint, sigma:bstr64]`. Header CBOR delegates here. Closed `OpCertCodecError` sum. |
| **BLUE opcert validation chokepoint (S2)** | `ade_core::consensus::opcert_validate::{opcert_validate, OpCertError}` | BLUE | Single RED→BLUE opcert acceptance chokepoint. Closed error sum: `CounterRegression`, `CounterRepeat`, `PeriodMismatch`, `BadColdSignature`, shape rejects. Defended by `ci_check_opcert_closed.sh`. |
| **BLUE canonical input value (S3)** | `ade_ledger::producer::state::ProducerTick` | BLUE | Closed 14-field struct (no `#[non_exhaustive]`). Every input `forge_block` needs as an explicit value. Private-key fields forbidden by CI. |
| **BLUE forge transition (S3)** | `ade_ledger::producer::forge::{forge_block, ForgeError, ForgeEffects, ForgedBlock}` | BLUE | Pure, total, deterministic. Closed error sum (no `String`-bearing variants; replay-byte-stable). Composes `is_leader_for_vrf_output` (validator-shared). Tx-admissibility prefix gate via `admit`. Body buckets assembled from preserved bytes via `split_conway_tx_components`. |
| **BLUE single canonical body-hash authority (S4)** | `ade_ledger::block_body_hash::{block_body_hash_from_buckets, block_body_hash}` | BLUE | The **only** function in the workspace that computes the Cardano block body-hash recipe. Both `block_validity::header_input` (validator) and `producer::forge::forge_block` (producer) hash through this function. Closes the producer/validator encoder bifurcation (DC-CONS-16). Defended by `ci_check_no_producer_body_encoder.sh`. |
| **BLUE self-accept bridge (S5)** | `ade_ledger::producer::self_accept::{self_accept, AcceptedBlock, SelfAcceptError}` | BLUE | Wraps `block_validity` (the single closed validator authority). `AcceptedBlock` newtype: private field, private constructor (only reachable via `Ok(...)` arm of `self_accept`). Closed `SelfAcceptError` sum. The type-level broadcast gate. Defended by `ci_check_self_accept_gate.sh` (6 guards). |
| **GREEN tick assembler (S6)** | `ade_runtime::producer::tick_assembler::{assemble_tick, TickInputs, TickAssemblyError}` | GREEN | Pure function stitching RED signing-primitive outputs + mempool snapshot into a canonical `ProducerTick`. Observably deterministic (identical inputs → byte-identical `ProducerTick`). Never invokes signing primitives, never reads I/O. |
| **RED scheduler core (S6)** | `ade_runtime::producer::scheduler::{scheduler_step, SchedulerInput, SchedulerEffect, SchedulerState, SchedulerHaltReason}` | RED | Pure RED state transition (mirrors N-B's `process_stream_input` shape). Wall-clock + I/O live in the outer driver. Closed `SchedulerInput` (2 variants) + `SchedulerEffect` (4 variants) + `SchedulerHaltReason` (2 variants). |
| **RED broadcast queue (S6)** | `ade_runtime::producer::broadcast::{BroadcastQueue, BroadcastError}` | RED | FIFO `VecDeque<AcceptedBlock>`. `enqueue(&mut self, AcceptedBlock)` consumes the token by value — type-level gate. Closed `BroadcastError` sum. |
| **GREEN mechanical cross-impl harness (S7)** | `ade_testkit::producer::{cross_impl_adapter, fixtures, reference_vectors, replay}` | GREEN | Decode-round-trip + body-hash binding (via S4's authority) + structural field agreement across forge ⊕ decoder. Closes the bytes-shape half of CN-CONS-06. Replay corpus carries signed artifacts only — never private keys (defended by `ci_check_no_private_keys_in_corpus.sh`). |
| **RED operator-action probe binary (S7)** | `ade_core_interop::bin::live_block_production_session` | RED | Third instance of the operator-action probe binary pattern. Hermetic default + `--connect` live pass. Loads cardano-cli `*.skey` envelopes, runs RED scheduler → GREEN tick-assembler → BLUE forge → BLUE `self_accept` for each leader slot in the window, logs JSON-Lines per slot. Status `blocked_until_operator_stake_available`. |
| **CI gates (S1..S7)** | `ci/ci_check_{private_key_custody, opcert_closed, forge_purity, no_private_keys_in_corpus, no_producer_body_encoder, self_accept_gate, scheduler_closure, producer_corpus_present}.sh` | CI | 8 mechanical gates defending the producer authority surface. Total CI count: 32 → 40. |

**Rule.** This domain has **one BLUE forge transition** (`forge_block`),
**one BLUE self-accept bridge** (`self_accept`), **one BLUE closed
sub-grammar opcert authority** (`{encode,decode}_opcert`), **one BLUE
opcert validation chokepoint** (`opcert_validate`), **one BLUE single
canonical body-hash authority** (`block_body_hash_from_buckets`),
**one BLUE canonical input value** (`ProducerTick`), **one GREEN tick
assembler** (`assemble_tick`), **one RED scheduler** (`scheduler_step`),
**one RED broadcast queue** (`BroadcastQueue`), **one GREEN
mechanical cross-impl harness** (`cross_impl_adapter`), and **one RED
operator-action probe binary** (`live_block_production_session`).
**THE KEY SEAMS:**

1. **RED→BLUE seam is `forge_block(&ProducerTick)`** — every input
   forge needs is a value on `ProducerTick`. No ambient state, no
   implicit ledger reads, no clock, no rand, no private-key bytes.
2. **BLUE→RED seam is `AcceptedBlock`** — type-level broadcast gate.
   `BroadcastQueue::enqueue(&mut self, AcceptedBlock)` consumes the
   token by value; the only constructor of `AcceptedBlock` is the
   `Ok(...)` arm of `self_accept`. CI-defended (6 guards).
3. **Body-hash recipe is single-authority** —
   `block_body_hash_from_buckets` is the only function in the
   workspace that computes the recipe. Producer and validator hash
   through the same function. CI-defended.
4. **Opcert byte authority is single-authority** —
   `ade_codec::shelley::opcert::{encode_opcert, decode_opcert}` is
   the closed-grammar pair. The prior inline header-path emit is
   forbidden. CI-defended.
5. **Leader-check is shared with the validator** —
   `is_leader_for_vrf_output` is the single source of leader truth
   (NC-VRF-3, OQ-12). No producer-side fork.
6. **Tx-admissibility prefix gate** — forge re-runs `admit` over
   `tick.mempool_tx_bytes` against `MempoolState::new(tick.base_state.clone())`
   in canonical accumulating order. Permutation / fabrication /
   skipping = `ForgeError::MempoolAcceptedMismatch`. DC-LEDGER-12.
7. **Private-key custody is RED-confined** —
   `*SigningKey` / `KesSecret` / `ColdSigningKey` types are forbidden
   in `ade_core` / `ade_codec` / `ade_types` / `ade_ledger` /
   `ade_crypto` public APIs by `ci_check_private_key_custody.sh`.
   Replay corpora carry signed artifacts only (no private keys) by
   `ci_check_no_private_keys_in_corpus.sh`.
8. **Scheduler is pure-RED state transition** — wall-clock + I/O live
   in the outer driver. Once halted (forge or self-accept failure),
   subsequent `SlotTick`s re-emit the original halt reason —
   deterministic re-emission, not silent recovery.

**New work** that adds a producer feature attaches by extending
`ProducerTick` (canonical input value), the forge pipeline (closed
steps), or the scheduler effect set (closed sum) — not by adding a
parallel forge path, not by bypassing the self-accept gate, not by
re-implementing the body-hash recipe.

**Declared non-goals carried from the cluster doc:** TPraos producer
(OQ-4 lock — Conway/Praos only), Ade-side key generation (OQ-1
lock — cardano-cli operator workflow), Ade-side opcert renewal
(OQ-6 lock — cardano-cli minting), full N2N producer-side block-fetch
server-side delivery (N-A successor scope; the broadcast queue is
the upstream side of that handoff).

### Mempool ingress (carried unchanged from N-E)

Carried. **N-C note:** the mempool snapshot consumed by `forge_block`
is the canonical `MempoolState` produced by this pipeline.

### Conway tx-body `proposal_procedures` sub-grammar authority (carried unchanged from PROPOSAL-PROCEDURES-DECODE)

Carried.

### Conway value-conservation accounting / Conway certificate-state accumulation / Credential discriminant fidelity / Conway governance-cert accumulation / Single-tx validity / Mempool admission / Full block validity / Ledger application / Stake-snapshot projection for consensus / Plutus phase-2 evaluation / Governance ratification & enactment / Mini-protocol wire conformance / Praos consensus runtime

All carried unchanged from the prior revision. **N-C-specific
strengthening:** the body-hash recipe used by `block_validity` is now
delegated to `block_body_hash::block_body_hash_from_buckets` (T-ENC-01
`strengthened_in += PHASE4-N-C`); the leader-schedule function
`is_leader_for_vrf_output` gained a second canonical caller (the
producer-side `forge_block` — DC-CONS-15); the opcert byte path now
goes through the closed-grammar pair `{encode,decode}_opcert`
(DC-CONS-11).

### Where the boundary is enforced

- `ci_check_dependency_boundary.sh` — no BLUE crate may depend on
  RED. N-C added an `ade_runtime → ade_ledger` edge (RED → BLUE
  via the `producer::AcceptedBlock` token import) — allowed.
- `ci_check_no_async_in_blue.sh` — async forbidden in BLUE.
- **`ci_check_private_key_custody.sh`** *(N-C-S1 — DC-CRYPTO-03/04/05,
  OP-OPS-04)* — forbids any `*SigningKey` / `KesSecret` / cold-key
  type from appearing in `ade_core` / `ade_codec` / `ade_types` /
  `ade_ledger` / `ade_crypto` public APIs.
- **`ci_check_opcert_closed.sh`** *(N-C-S2 — DC-CONS-11/12)* —
  forbids parallel opcert encoders / decoders outside
  `ade_codec::shelley::block` + `ade_codec::shelley::opcert` +
  `ade_core::consensus::opcert_validate`.
- **`ci_check_forge_purity.sh`** *(N-C-S3 — DC-CONS-13/14/15,
  DC-LEDGER-12)* — forbids `std::time::SystemTime`, `rand`,
  `HashMap` iteration, `std::env`, `std::fs`, `async`, `println!`
  in `forge.rs` / `state.rs` / `tx_components.rs`. Forbids
  `VrfDraft03::prove` / `Sum6Kes::sign_kes` calls inside
  `ade_ledger/src/producer/` or `ade_core/src/`. Forbids
  `#[non_exhaustive]` on the closed sums. Forbids `String`-bearing
  variants on `ForgeError` / `ForgeEffects`.
- **`ci_check_no_private_keys_in_corpus.sh`** *(N-C-S3 — DC-CONS-14)*
  — forbids private-key bytes in producer replay corpora.
- **`ci_check_no_producer_body_encoder.sh`** *(N-C-S4 — DC-CONS-16)*
  — grep gate forbidding any new `pub fn .*encode_block_body` outside
  the canonical authority.
- **`ci_check_self_accept_gate.sh`** *(N-C-S5 — CN-CONS-07)* —
  6 mechanical guards: (1) `AcceptedBlock` has no public constructor
  outside `self_accept.rs` (struct-literal + return-type grep);
  (2) `AcceptedBlock.bytes` field is private; (3) `SelfAcceptError`
  is a closed sum (no `#[non_exhaustive]`, no `String`-bearing
  variant); (4) `self_accept` calls the canonical `block_validity`;
  (5) `self_accept` does NOT re-implement validator sub-steps
  (`validate_and_apply_header`, `decode_block`, `block_body_hash`
  forbidden in `self_accept.rs` production source);
  (6) no `pub fn` returning raw `Vec<u8>` / `&[u8]` outside the
  `as_bytes` / `into_bytes` accessors on the token.
- **`ci_check_scheduler_closure.sh`** *(N-C-S6 — OP-OPS-05)* —
  scheduler closure properties.
- **`ci_check_producer_corpus_present.sh`** *(N-C-S7 — CN-CONS-06)*
  — guards producer fixture corpus presence + non-empty
  `expected_forged.cbor` outputs.
- **`ci_check_constitution_coverage.sh`** *(modified at N-C close)*
  — release/operational entries may carry `code_locus` / `ci_script`
  / `tests` when `status = "enforced"`; forbidden only on
  `declared` / `partial` / `blocked` statuses. Closes the enforcement-evidence
  pinning for `CN-CONS-06/07` and `OP-OPS-04/05`.
- `ci_check_proposal_procedures_closed.sh` *(PP — DC-LEDGER-11)* — carried.
- `ci_check_mempool_ingress_closure.sh` / `ci_check_mempool_ingress_replay.sh`
  *(N-E — DC-MEM-03/04)* — carried.
- `ci_check_credential_discriminant_closed.sh` *(OQ5 / COMMITTEE /
  DREP / ENACTMENT — DC-LEDGER-10)* — carried.
- `ci_check_gov_cert_accumulation_closed.sh` *(B5 — DC-LEDGER-09)* —
  carried.
- `ci_check_deposit_param_authority.sh` *(B3 — DC-TXV-07)* — carried.
- `ci_check_conway_cert_classification_closed.sh` *(B3F — DC-TXV-06)*
  — carried.
- `ci_check_no_chaindb_in_consensus_blue.sh` / `ci_check_no_float_in_consensus.sh`
  / `ci_check_no_density_in_fork_choice.sh` / `ci_check_consensus_closed_enums.sh`
  — carried.
- `ci_check_pallas_quarantine.sh`, `ci_check_no_signing_in_blue.sh`,
  `ci_check_ingress_chokepoints.sh`, `ci_check_ce_n_a_5_proof.sh` —
  carried.

---

## 3. Closed vs. Extensible Registries

Ade's authority surface is **almost entirely closed.** **PHASE4-N-C
added eleven closed surfaces** — `ProducerTick` (closed 14-field
struct), `ForgeError` / `ForgeEffects` / `ForgedBlock` (closed sums),
`OpCertError` (closed validation-error sum), `OpCertCodecError`
(closed codec-error sum), `SelfAcceptError` / `AcceptedBlock` (closed
sum + closed newtype token), `SchedulerInput` / `SchedulerEffect` /
`SchedulerHaltReason` (closed sums for the producer state machine),
`TickAssemblyError` (closed GREEN assembler-error sum), `BroadcastError`
(closed RED queue-error sum), the `signing::{VrfSigningKey, KesSecret,
ColdSigningKey, SigningError}` closed key/error set, and the
single-canonical body-hash chokepoint
`block_body_hash::block_body_hash_from_buckets`. Plus **eight CI gates**
(CI count 32 → 40) and **fourteen registry rules** (T total 176 → 190).

### Closed (frozen — version-gated changes only)

| Registry | Location | Count | Change Rule |
|----------|----------|-------|-------------|
| `CardanoEra` | `ade_types::era` | 8 variants | New variant = new hard fork. |
| `Certificate` | `ade_types::shelley::cert` | 7 variants | Shelley-era frozen. |
| `StakeCredential` *(OQ5)* | `ade_types::shelley::cert` | 2 variants | DC-LEDGER-10. |
| Credential-decode chokepoints *(OQ5 + PP)* | `ade_codec::{shelley,conway}::cert::decode_stake_credential` + `ade_codec::conway::governance::decode_stake_credential` | 3 functions | Closed 2-variant mapping. |
| `ConwayCert` *(B3/B4)* | `ade_types::conway::cert` | 19 variants | DC-LEDGER-08. |
| `GovAction` *(PP/ENACTMENT)* | `ade_types::conway::governance` | 7 variants | DC-LEDGER-11. |
| `ProposalProcedure` *(PP)* | `ade_types::conway::governance` | closed 4-field struct | DC-LEDGER-11. |
| `decode_proposal_procedures` / `encode_proposal_procedures` *(PP)* | `ade_codec::conway::governance` | 2 functions | DC-LEDGER-11. |
| `MIRPot` | `ade_types::shelley::cert` | 2 variants | Frozen. |
| `DRep` | `ade_types::conway::cert` | 4 variants | CIP-1694 fixed. |
| `CertDisposition` / `DepositEffect` / `CoinSource` *(B3)* | `ade_types::conway::cert` | 3 / 2 / 3 variants | Closed. |
| `ConwayCertAction` *(B4)* | `ade_ledger::delegation` | closed | No `Neutral`. |
| `GovernanceCertEffect` / `OwnerTaggedEffect` / etc. *(B4)* | `ade_ledger::delegation` | closed | B4 plumbing. |
| `GovCertEnv` *(B5)* | `ade_ledger::state` | closed struct | Fail-fast. |
| `apply_conway_gov_cert` dispatch *(B5)* | `ade_ledger::gov_cert` | 1 function | DC-LEDGER-09. |
| `apply_committee_enactment` *(ENACTMENT)* | `ade_ledger::governance` | 1 pure transition | Closed. |
| `IngressSource` *(N-E)* | `ade_ledger::mempool::ingress` | 2 variants | Closed source discriminant. |
| `IngressEvent` *(N-E)* | `ade_ledger::mempool::ingress` | closed struct | Closed flat-data envelope. |
| `mempool_ingress` chokepoint *(N-E)* | `ade_ledger::mempool::ingress` | 1 function | DC-MEM-03. |
| **`ProducerTick`** *(NEW in N-C-S3 — DC-CONS-13)* | `ade_ledger::producer::state` | **closed 14-field struct** — `{ slot, base_state, mempool, mempool_tx_bytes, pparams, leader_answer, vrf_proof, vrf_output, vrf_vkey, kes_period, kes_signature, opcert, cold_vk, prev_opcert_counter, block_number, prev_hash, protocol_version }` | The **canonical input value to `forge_block`**. No `#[non_exhaustive]`. Private-key fields forbidden by `ci_check_forge_purity.sh` guard 5 + `ci_check_no_private_keys_in_corpus.sh`. New field = strengthen DC-CONS-13. |
| **`forge_block` chokepoint** *(NEW in N-C-S3 — DC-CONS-13/14/15, DC-LEDGER-12; closed-grammar entry point, not a registry)* | `ade_ledger::producer::forge` | 1 function — `pub fn forge_block(&ProducerTick) -> Result<(ForgedBlock, Vec<ForgeEffects>), ForgeError>` | The **single producer-side composition root**. Pure, total, deterministic. Defended by `ci_check_forge_purity.sh`. |
| **`ForgeError`** *(NEW in N-C-S3 — DC-CONS-13)* | `ade_ledger::producer::forge` | 7 variants — `NotLeader`, `OpCertRejected`, `TxSetNotAdmissiblePrefix`, `MempoolWidthMismatch`, `MempoolAcceptedMismatch`, `BadKesSignatureLength`, `TxComponentSplit` | No `#[non_exhaustive]`. No `String`-bearing variants (replay-byte-stable). |
| **`ForgeEffects`** *(NEW in N-C-S3)* | `ade_ledger::producer::forge` | 1 variant — `ReadyForSelfAccept { next_prev_opcert_counter: u64 }` | Closed effect sum. |
| **`ForgedBlock`** *(NEW in N-C-S3)* | `ade_ledger::producer::forge` | closed struct `{ bytes: Vec<u8>, block: ShelleyBlock }` | Closed. |
| **`encode_opcert` / `decode_opcert` chokepoint pair** *(NEW in N-C-S2 — DC-CONS-11; closed-grammar entry points)* | `ade_codec::shelley::opcert` | 2 functions | The **single producer-side opcert byte authority**. Standalone 4-tuple. Both header CBOR (`shelley::block`) and standalone opcert delegate here. Defended by `ci_check_opcert_closed.sh`. |
| **`OpCertCodecError`** *(NEW in N-C-S2)* | `ade_codec::shelley::opcert` | 7 variants — `BadArrayHeader`, `BadFieldType`, `WrongHotVkeyLength`, `WrongSigmaLength`, `SequenceNumberOverflow`, `KesPeriodOverflow`, `TrailingBytes` | No `#[non_exhaustive]`. |
| **`opcert_validate` chokepoint** *(NEW in N-C-S2 — DC-CONS-11/12; closed-grammar entry point)* | `ade_core::consensus::opcert_validate` | 1 function | The **single RED→BLUE opcert acceptance chokepoint**. Defended by `ci_check_opcert_closed.sh`. |
| **`OpCertError`** *(NEW in N-C-S2 — DC-CONS-12)* | `ade_core::consensus::opcert_validate` | closed validation-error sum | No `#[non_exhaustive]`. |
| **`block_body_hash_from_buckets` chokepoint** *(NEW in N-C-S4 — DC-CONS-16; single canonical authority, not a registry)* | `ade_ledger::block_body_hash` | 1 function — `pub fn block_body_hash_from_buckets(tx_bodies: &[u8], witness_sets: &[u8], metadata: &[u8], invalid_txs: Option<&[u8]>) -> Hash32` | The **only function** in the workspace that computes the Cardano block body-hash recipe. Both `block_validity::header_input` (validator) and `producer::forge::forge_block` (producer) hash through this function. Defended by `ci_check_no_producer_body_encoder.sh`. |
| **`AcceptedBlock` token** *(NEW in N-C-S5 — CN-CONS-07; closed-grammar token, not a registry)* | `ade_ledger::producer::self_accept` | 1 newtype `{ bytes: Vec<u8> }` (private field) | The **type-level broadcast gate**. Only constructor is the `Ok(...)` arm of `self_accept`. `pub fn ... -> Result<AcceptedBlock, ...>` count must equal 1 across crates/ (CI guard 1b). |
| **`self_accept` chokepoint** *(NEW in N-C-S5 — CN-CONS-07)* | `ade_ledger::producer::self_accept` | 1 function | The **single sanctioned production constructor of `AcceptedBlock`**. Calls `block_validity` (the canonical validator authority). MUST NOT re-implement validator sub-steps. Defended by `ci_check_self_accept_gate.sh` (6 guards). |
| **`SelfAcceptError`** *(NEW in N-C-S5)* | `ade_ledger::producer::self_accept` | 1 variant — `Rejected(BlockValidityError)` | Closed sum. No `#[non_exhaustive]`. No `String`-bearing variant. |
| **`SchedulerInput`** *(NEW in N-C-S6 — OP-OPS-05)* | `ade_runtime::producer::scheduler` | 2 variants — `SlotTick { slot, inputs: TickInputs }`, `ChainAdvanced { ledger, chain_dep, mempool }` | Closed state-machine boundary. New trigger = strengthening. |
| **`SchedulerEffect`** *(NEW in N-C-S6 — OP-OPS-05)* | `ade_runtime::producer::scheduler` | 4 variants — `EnqueueBroadcast(AcceptedBlock)`, `SilentNonLeader{slot}`, `HaltOnInvariant{slot, reason}`, `HaltOnAssembly{slot, reason}` | Closed effect sum. |
| **`SchedulerHaltReason`** *(NEW in N-C-S6 — OP-OPS-05)* | `ade_runtime::producer::scheduler` | 2 variants — `Forge(ForgeError)`, `SelfAccept(SelfAcceptError)` | Closed halt-reason sum. |
| **`SchedulerState`** *(NEW in N-C-S6)* | `ade_runtime::producer::scheduler` | closed struct `{ ledger, chain_dep, mempool, era_schedule, last_seen_slot, prev_opcert_counter, halted }` | Closed. |
| **`TickInputs`** *(NEW in N-C-S6)* | `ade_runtime::producer::tick_assembler` | closed 13-field struct | The closed RED-supplied input set to the GREEN tick assembler. Carries signed artifacts only; no private keys. |
| **`TickAssemblyError`** *(NEW in N-C-S6)* | `ade_runtime::producer::tick_assembler` | 2 variants — `VrfProofMalformed{detail}`, `MempoolWidthMismatch{tx_bytes, accepted_ids}` | Closed sum. No `String`-bearing variants. |
| **`assemble_tick` chokepoint** *(NEW in N-C-S6 — RED→GREEN seam)* | `ade_runtime::producer::tick_assembler` | 1 function — `pub fn assemble_tick(slot, &LedgerState, &MempoolState, &TickInputs) -> Result<ProducerTick, TickAssemblyError>` | The **single GREEN seam from RED signing-primitive outputs to the canonical BLUE `ProducerTick` value**. Pure; observably deterministic. |
| **`BroadcastError`** *(NEW in N-C-S6)* | `ade_runtime::producer::broadcast` | 2 variants — `QueueFull`, `Shutdown` | Closed. No `#[non_exhaustive]`. |
| **RED signing primitives + key types** *(NEW in N-C-S1 — DC-CRYPTO-03/04/05, OP-OPS-04)* | `ade_runtime::producer::signing::{vrf_prove, kes_sign, kes_update, VrfSigningKey, KesSecret, ColdSigningKey, SigningError}` | 3 functions + 3 closed key types + 1 closed error sum | The **only sanctioned RED signing surface**. Keys are `zeroize`-on-drop; no `pub` raw-byte accessors. Forbidden in BLUE / codec / types / ledger / crypto public APIs by `ci_check_private_key_custody.sh`. |
| **RED key loader** *(NEW in N-C-S1 — OP-OPS-04)* | `ade_runtime::producer::keys::{load_vrf_signing_key_skey, load_kes_signing_key_skey, load_cold_signing_key_skey, KeyLoadError}` | 3 loader functions + 1 closed error sum | The **single sanctioned RED disk-to-secret path**. Cardano-cli `*.skey` text envelopes only. |
| `PlutusLanguage` | `ade_plutus::evaluator` | 3 variants | |
| Named ingress chokepoints (block CBOR) | `ade_codec::*` | 10 | |
| Conway cert/withdrawals sub-grammar decoders *(B3 / B4)* | `ade_codec::conway::{cert, withdrawals}` + `ade_codec::shelley::cert::read_pool_registration_cert` | 5 functions | Closed. |
| Named ingress chokepoint (Plutus script CBOR) | `ade_plutus::evaluator::PlutusScript::from_cbor` | 1 | |
| `PreservedCbor::new` constructor | `ade_codec::preserved` | 1 chokepoint, `pub(crate)` | |
| `CodecError` variants *(B3-extended)* | `ade_codec::error` | + `UnknownCertTag`, `DuplicateMapKey` | |
| Mini-protocol message enums | `ade_network::codec::*` | 11 closed enums | |
| Mini-protocol encode/decode chokepoints | `ade_network::codec::*::{encode_*, decode_*}` | 22 functions | |
| Mux frame chokepoints | `ade_network::mux::frame::{encode_frame, decode_frame}` | 2 free functions | |
| Mini-protocol transition functions | `ade_network::*::transition` + `n2c::local_*::transition` | 8 modules | |
| Mini-protocol version enums | `ade_network::codec::version::*` | 11 closed enums | |
| `ChainDb` / `SnapshotStore` / `Recoverable` trait surfaces | `ade_runtime::chaindb` + `ade_runtime::recovery` | closed | |
| Hash domain functions | `ade_crypto::blake2b::*` | 4 named domains | |
| `ChainEvent` / `ChainSelectionReject` *(N-B)* | `ade_core::consensus::events` | 5 / 4 variants | |
| Consensus error families *(N-B)* | `ade_core::consensus::errors` | 8 closed error enums | |
| `StreamInput` / `OrchestratorError` / `DecodeError` / `GenesisParseError` / `GenesisBlob` / `NetworkMagic` *(N-B)* | various | closed | |
| `LedgerView` trait *(N-B; B1-refined)* | `ade_core::consensus::ledger_view` | 4 methods | |
| `HeaderVrf` *(N-B; B1)* | `ade_core::consensus::header_summary` | 2 variants | |
| `BlockValidityVerdict` / `BlockValidityError` etc. *(B1)* | `ade_ledger::block_validity::verdict` | closed | |
| `block_validity` chokepoint *(B1)* | `ade_ledger::block_validity::transition` | 1 function | Single chokepoint `self_accept` (N-C-S5) wraps. |
| `TxValidityVerdict` / `TxRejectClass` / `TxValidityError` / `SignerSource` / `WitnessClosureError` etc. *(B2)* | `ade_ledger::tx_validity::*` | closed | |
| `AdmitOutcome` / `MempoolState` / `OrderPolicy` *(B2)* | `ade_ledger::mempool::*` | closed | |
| `LeaderScheduleAnswer` *(N-B; consumed unchanged by N-C)* | `ade_core::consensus::leader_schedule` | closed struct | Shared between validator and producer (NC-VRF-3 — single source of leader truth). |
| `is_leader_for_vrf_output` *(N-B; consumed unchanged by N-C)* | `ade_core::consensus::leader_schedule` | 1 function | The **single leader-truth function**. CI gate (`ci_check_forge_purity.sh` guard 2) asserts this is the only `fn .*is_leader.*` definition reachable from `ade_core` / `ade_ledger`. |
| `PraosNonces` / `NonceScanError` *(B1)* | `ade_ledger::consensus_input_extract` | | |
| `PraosChainDepState` / `ChainEvent` canonical encodings *(N-B)* | `ade_core::consensus::encoding` | 4 chokepoints | |
| `LedgerFingerprint` fold *(B3/B5)* | `ade_ledger::fingerprint` | | |
| **CI check set** | `ci/ci_check_*.sh` | **40 scripts (32 → 40 in PHASE4-N-C)** | Existing checks may be tightened, never relaxed. |
| **Invariant registry families** | `docs/ade-invariant-registry.toml` | Families T / CN / DC / OP / RO; **N-C added 14 rules** (`DC-CRYPTO-03/04/05`, `DC-CONS-11/12/13/14/15/16`, `DC-LEDGER-12`, `CN-CONS-06`, `CN-CONS-07`, `OP-OPS-04`, `OP-OPS-05`); `T-DET-01.strengthened_in += PHASE4-N-C`; `T-ENC-01.strengthened_in += PHASE4-N-C`. Total: **190 entries** (176 → 190). | Append-only IDs. |

### Extensible (open within constraints)

| Registry | Location | Extension Rule |
|----------|----------|---------------|
| `CostModels` map (Plutus V1/V2/V3 cost tables) | `ade_plutus::cost_model::CostModels` | Decoder-driven; constrained by closed `PlutusLanguage`. |
| `ProtocolParameters` / `ProtocolParameterUpdate` field set | `ade_ledger::pparams` | Era-versioned. |
| Pool / DRep / Stake registrations | `ade_ledger::state::{DelegationState, CertState}` | Shape closed; set open. |
| Governance proposal / committee / DRep registration set | `ade_ledger::state::ConwayGovState` | Shape closed; instance set open. |
| Tx-body `proposal_procedures` instance set *(PP)* | `ade_types::conway::tx::ConwayTxBody.proposal_procedures` | `Option<Vec<ProposalProcedure>>`. Shape closed; instance set open. |
| `OpCertCounterMap` *(N-B)* | `ade_core::consensus::praos_state` | BTreeMap; inserts strictly increasing per `(pool, kes_period)`. |
| `PoolDistrView` pool table *(B1)* | `ade_ledger::consensus_view::PoolDistrView::pools` | `BTreeMap<Hash28, PoolEntry>`. |
| Withdrawals map *(B3)* | `ade_codec::conway::withdrawals::decode_withdrawals` → `BTreeMap<RewardAccount, Coin>` | Never last-wins. |
| Mempool admitted set *(B2; ingress-fed in N-E)* | `ade_ledger::mempool::admit::MempoolState::accepted` | `Vec<Hash32>`; shape closed; set open; monotonic. |
| `SignerSource` provenance set *(B2)* | `ade_ledger::tx_validity::required_signers::RequiredSigners::{keys, provenance}` | Per-tx open; closed enum. |
| `RollbackSnapshot` ring *(N-B)* | `ade_runtime::consensus::chain_selector::OrchestratorState::recent_snapshots` | Bounded ≤ 2160. |
| Oracle reference snapshots / regression corpus | `ade_testkit::harness::*` | Tooling-only. |
| Network corpus / Consensus corpus / Block-validity corpus / Tx-validity corpus / Mempool ingress corpus / PP canonical corpus | various | Tooling-only. |
| **Producer replay corpus** *(NEW in N-C-S3/S4 — tooling-only; signed-artifact-only by DC-CONS-14)* | `crates/ade_testkit/fixtures/producer/` + `ade_testkit::producer::{fixtures, replay, reference_vectors}` | Tooling-only. GREEN. Ordered `Vec<ProducerTick>` plus expected `Vec<ForgedBlockBytes>`. **Forbidden:** private-key bytes in any corpus fixture (defended by `ci_check_no_private_keys_in_corpus.sh`). Append-only by convention. |
| **Producer mechanical cross-impl corpus** *(NEW in N-C-S7 — tooling-only)* | `ade_testkit::producer::cross_impl_adapter` | Tooling-only. GREEN. Decode-round-trip + body-hash binding + structural field agreement. Append-only by convention. |
| **Operator-action probe binaries** *(N-B + N-E S6 + N-C S7)* | `ade_core_interop::bin::{live_consensus_session, live_tx_submission_session, live_block_production_session}` | RED operator-action; `#[ignore]`-gated by closure-gate tests. **N-C added `live_block_production_session`** — status `blocked_until_operator_stake_available`. |
| `KillStrategy<D>` trait impls | `ade_runtime::chaindb::crash_safety` | RED-only test infrastructure. |
| Recovery state types | callers of `Recoverable` | Open: any state with canonical encode + apply-block step. |
| Pinned external crates | `crates/*/Cargo.toml` | Tier-5 rationale doc required. **N-C consumed:** `cardano-crypto = "1.0.8"` features `["vrf-draft03", "kes-sum", "dsign"]` (already pinned at HEAD `96d043c`). |

### Candidates — extensible surfaces not yet wired

| Cluster | Candidate registry | Rationale |
|---------|-------------------|-----------|
| **CE-N-C-8 (operator-action live evidence — `blocked_until_operator_stake_available`)** | **Live N2N block-fetch acceptance log** | The crypto-level cross-impl claim. Requires testnet SPO registration. Re-opens on operator availability. |
| **N-A+ (N-C handoff target)** | **N2N producer-side block-fetch server role + chain-sync extension** | Outbound delivery of broadcast-queued `AcceptedBlock` bytes. Declared OUT-OF-SCOPE in N-C cluster doc. |
| **N-C+ Tier-5** | **Operator-tunable producer policy** (slot-deadline budget, opcert renewal cadence, broadcast retry policy) | Tier-5 — operator-tunable. Declared OUT-OF-SCOPE in N-C cluster doc. |
| **CE-NODE-N2C-LTX (cross-cluster obligation carried from N-E)** | **Live N2C UDS server + N2N bulk-tx inbound listener** | Carried. |
| **PP OQ-1..OQ-4** | various | Carried. |
| N-A (deferred) | Peer address book | Operator-supplied; runtime mutable. |
| N-F | Query API method set | Tier 5 wire / Tier 1 semantics. |
| N-F | Prometheus metric names | Tier 5; append-only registry expected. |

### Closed-grammar audit (PHASE4-N-C full close)

This sweep was performed after PHASE4-N-C full close (S1..S7).

1. **`opcert_validate` chokepoint** — **closed by intent and
   CI-defended.** Closed `OpCertError` sum. Defended by
   `ci_check_opcert_closed.sh`.
2. **`encode_opcert` / `decode_opcert` chokepoint pair** — **closed
   by intent and CI-defended.** Single producer-side opcert byte
   authority. Header CBOR delegates here. Closed `OpCertCodecError`
   sum.
3. **`ProducerTick` canonical input value** — **closed by intent and
   CI-defended.** Closed 14-field struct. No `#[non_exhaustive]`.
   Private-key fields forbidden by CI.
4. **`forge_block` chokepoint** — **closed by intent and CI-defended.**
   Pure, total, deterministic. Closed `ForgeError` sum (7 variants).
   Closed `ForgeEffects` sum (1 variant). Closed `ForgedBlock` struct.
   Defended by `ci_check_forge_purity.sh` (7 mechanical guards).
5. **`block_body_hash_from_buckets` single canonical authority** —
   **closed by intent and CI-defended.** The only function in the
   workspace that computes the Cardano block body-hash recipe. Both
   producer and validator hash through this function. Defended by
   `ci_check_no_producer_body_encoder.sh`.
6. **`self_accept` chokepoint + `AcceptedBlock` token** — **closed
   by intent and CI-defended.** Single sanctioned production
   constructor of `AcceptedBlock`. `pub fn ... -> Result<AcceptedBlock,
   ...>` count = 1 across crates/. `AcceptedBlock.bytes` field is
   private. Closed `SelfAcceptError` sum. Defended by
   `ci_check_self_accept_gate.sh` (6 guards).
7. **`SchedulerInput` / `SchedulerEffect` closed sums** — **closed
   by intent.** 2-variant / 4-variant. Pure RED state transition;
   wall-clock + I/O live in the outer driver.
8. **`TickInputs` + `assemble_tick` GREEN seam** — **closed by intent.**
   Single GREEN seam from RED signing-primitive outputs to the
   canonical BLUE `ProducerTick` value. Pure; observably deterministic.
9. **Private-key custody** — **closed by intent and CI-defended.**
   `*SigningKey` / `KesSecret` / `ColdSigningKey` types forbidden in
   `ade_core` / `ade_codec` / `ade_types` / `ade_ledger` / `ade_crypto`
   public APIs by `ci_check_private_key_custody.sh`. Replay corpora
   carry signed artifacts only by `ci_check_no_private_keys_in_corpus.sh`.
10. **`live_block_production_session` operator-action probe binary**
    — **closed by intent on the harness pattern.** Third instance of
    the family (after `live_consensus_session` / `live_tx_submission_session`).
    Hermetic-default-plus-`--connect`-live. Status
    `blocked_until_operator_stake_available` — third closure mode
    introduced by N-C (beyond captured / deferred).

**Gap note — N-C (CE-N-C-8).** The crypto-level cross-impl claim is
the only N-C obligation that depends on an external resource
(testnet SPO stake). Per `CN-CONS-06.open_obligation` it is
`blocked_until_operator_stake_available` — not deferred to a future
cluster, not silently accepted. Reopens when stake is provisioned;
mechanical half (structural cross-impl) is already enforced via
`cross_impl_adapter` + `ci_check_producer_corpus_present.sh`.

### Closed-grammar audit (carried — PROPOSAL-PROCEDURES-DECODE / PHASE4-N-E / B3 / B4 / B5)

All carried unchanged from prior revision.

---

## 4. Version-Gated vs. Frozen Contracts

### Frozen (immutable at current version — change = new major version)

- **Cardano-canonical CBOR wire format**.
- **Block envelope shape**: `[era_tag:u8, era_block:CBOR]`; era tags 0..=7.
- **`PreservedCbor<T>` invariant**.
- **Hash algorithms**: Blake2b-224 / 256, Ed25519, Byron-bootstrap,
  KES-sum, VRF-draft-03.
- **Era-correct block body hash** *(B1; strengthened in N-C)*:
  preserved-CBOR-segment bytes (T-ENC-01,
  `strengthened_in += PHASE4-N-C`).
- **Single canonical body-hash authority** *(NEW in N-C-S4 —
  DC-CONS-16)*: `ade_ledger::block_body_hash::block_body_hash_from_buckets`
  is the **only** function in the workspace that computes the
  4-bucket Cardano body-hash recipe. Both producer (`forge_block`)
  and validator (`block_validity::header_input`) hash through this
  function. The producer/validator encoder bifurcation is closed by
  construction. Defended by `ci_check_no_producer_body_encoder.sh`.
- **Tx id over preserved body bytes** *(B2)*.
- **Conway certificate CDDL grammar** *(B3/B3F/B4)*.
- **Conway `DRep` decode grammar** *(B4)*.
- **Owner-tagged Conway cert-state apply contract** *(B4)*: DC-LEDGER-08.
- **Closed total gov-cert dispatch contract** *(B5)*: DC-LEDGER-09.
- **Fail-fast gov-cert environment** *(B5)*.
- **Checked DRep-expiry arithmetic** *(B5)*.
- **`ConwayGovState` deterministic-fold accumulation** *(B5)*.
- **Conway withdrawals map grammar** *(B3)*: never last-wins.
- **Closed deposit-effect sum types** *(B3)*.
- **Canonical deposit-param authority** *(B3)*: DC-TXV-07.
- **Full Conway value-conservation equation** *(B3)*: frozen §9.1
  reject precedence.
- **`LedgerFingerprint` Conway deposit-param fold** *(B3)*.
- **Closed `proposal_procedures` wire grammar at Conway tx-body
  key 20** *(PP — DC-LEDGER-11)*.
- **Plutus script ingress chokepoint**: `PlutusScript::from_cbor`.
- **Plutus language set**: V1, V2, V3.
- **Aiken UPLC quarantine pin**: `aiken_uplc` at tag `v1.1.21`.
- **Ouroboros mux frame layout**: 8-byte big-endian header.
- **11 closed mini-protocol message enums** + **8 closed state graphs**.
- **`BootstrapAnchorHash` v1 preimage** *(N-B)*.
- **`EraSchedule` invariants** *(N-B)*.
- **`PraosChainDepState` / `ChainEvent` CBOR encodings** *(N-B)*.
- **Consensus error taxonomies** *(N-B)*.
- **`StreamInput` 3-variant taxonomy** *(N-B)*. **`HeaderVrf` era model**.
- **`block_validity` composition contract** *(B1; consumed unchanged
  by N-C `self_accept`)*.
- **`VerdictSurface` CBOR encoding** *(B1)*.
- **`LedgerView` trait shape** *(N-B; B1-refined)*.
- **`tx_validity` composition contract** *(B2; consumed transitively
  via `admit` by N-C `forge_block`)*.
- **`SignerSource` enumeration** *(B2)*.
- **Witness-closure contract** *(B2)*.
- **`TxVerdictSurface` CBOR encoding** *(B2)*.
- **Mempool admission contract** *(B2)*: `admit`'s verdict equals
  `tx_validity`'s verdict; no false accept (DC-MEM-01).
- **`mempool_ingress` chokepoint contract** *(N-E)*.
- **`IngressSource` source-invariance contract** *(N-E)*.
- **Verbatim tx-bytes flow through ingress** *(N-E)*.
- **GREEN single-step replay fold contract** *(N-E — DC-MEM-04)*.
- **Cross-cluster obligation pattern** *(N-E)*.
- **Operator-action evidence pattern** *(N-B / N-E / N-C —
  strengthened in N-C with the third closure mode
  `blocked_until_operator_stake_available`)*.
- **Closed credential discriminant contract** *(OQ5 / COMMITTEE /
  DREP / ENACTMENT / PP)*.
- **Committee-enactment write-back contract** *(ENACTMENT)*.
- **All canonical types**: shapes frozen at the era / version they
  entered.
- **TCB color assignments**: per `.idd-config.json` `core_paths`.
  `ade_core::consensus` (incl. `opcert_validate` — N-C),
  `ade_ledger::{block_validity, tx_validity, mempool::admit,
  mempool::ingress, consensus_view, cert_classify, delegation,
  gov_cert, block_body_hash, producer::{forge, self_accept, state}}`
  *(producer new in N-C)*, `ade_codec::shelley::opcert` *(new in N-C)*,
  `ade_codec::conway::{cert, withdrawals, governance}`,
  `ade_codec::shelley::cert`, `ade_types::conway::{cert, governance}`
  are BLUE; `ade_ledger::mempool::{policy, canonicalize}` are GREEN
  behavior inside the BLUE crate;
  `ade_ledger::consensus_input_extract` is RED-behavior-inside-BLUE;
  `ade_runtime::consensus` + `ade_runtime::producer::{signing, keys,
  scheduler, broadcast}` *(producer new in N-C)* are RED;
  `ade_runtime::producer::tick_assembler` *(new in N-C)* is GREEN
  inside the RED crate;
  `ade_testkit::{consensus, validity, tx_validity, mempool, governance,
  producer}` *(producer new in N-C)* is GREEN;
  `ade_core_interop` is RED-crate / GREEN-pure-functions /
  RED-operator-action-binaries (third probe binary added in N-C-S7).
- **`ChainDb` / `SnapshotStore` / `Recoverable` trait shapes** (N-D).
- **`AcceptedBlock` type-level broadcast gate** *(NEW in N-C-S5 —
  CN-CONS-07)*: `AcceptedBlock` is a newtype `{ bytes: Vec<u8> }`
  whose only field is private and whose only constructor is the
  `Ok(...)` arm of `ade_ledger::producer::self_accept::self_accept`.
  RED `BroadcastQueue::enqueue(&mut self, AcceptedBlock)` consumes
  the token by value. **Producer/validator agreement at the type
  level**: a forged block whose body-hash, KES signature, leader
  claim, or body validity disagrees with Ade's own validator halts
  the producer deterministically before any bytes leave the host.
  Defended by `ci_check_self_accept_gate.sh` (6 mechanical guards).
- **`forge_block` pure-transition contract** *(NEW in N-C-S3 —
  DC-CONS-13)*: no clock, no rand, no I/O, no `HashMap`, no
  `std::env`, no `std::fs`. Replay byte-equivalence over identical
  `ProducerTick` streams (DC-CONS-14). T-DET-01
  `strengthened_in += PHASE4-N-C`. Defended by
  `ci_check_forge_purity.sh`.
- **Single source of leader truth** *(NEW in N-C-S3 — DC-CONS-15)*:
  `is_leader_for_vrf_output` is the **only** `fn .*is_leader.*`
  definition reachable from `ade_core` / `ade_ledger`. Producer and
  validator share it. No producer-side fork permitted.
- **Tx-admissibility prefix property** *(NEW in N-C-S3 — DC-LEDGER-12)*:
  every tx in a forged block is admissible via `mempool::admit`
  against the base ledger state, in the snapshot's canonical
  accumulating order. No permute, no fabricate, no skip.
- **Private-key custody RED-confinement** *(NEW in N-C-S1 —
  OP-OPS-04, DC-CRYPTO-03/04/05)*: `*SigningKey` / `KesSecret` /
  `ColdSigningKey` types are forbidden in BLUE / codec / types /
  ledger / crypto public APIs. Replay corpora carry signed artifacts
  only. KES evolution is one-way; the evolved key signs period
  `i+1` and MUST NOT sign for period `i`.
- **Closed-grammar opcert byte authority** *(NEW in N-C-S2 —
  DC-CONS-11)*: `ade_codec::shelley::opcert::{encode_opcert,
  decode_opcert}` is the single producer-side opcert byte authority.
  Standalone 4-tuple `[hot_vkey:bstr32, sequence_number:uint,
  kes_period:uint, sigma:bstr64]`. The prior inline header-path emit
  is forbidden.
- **OpCert serial counter strict monotonicity** *(NEW in N-C-S2 —
  DC-CONS-12)*: `opcert_validate` rejects regression or repetition
  at the RED→BLUE boundary.

### Version-gated (can evolve across major versions)

- **New `CardanoEra` variant**: full coordinated change.
- **New Conway certificate tag** *(B3 / B4 / B5)*.
- **New `CoinSource` deposit-provenance** *(B3)*.
- **Pre-Conway single-tx validity** *(B2 extension point)*.
- **Full-scope `track_utxo=true` tx corpus** *(B2 extension point)*.
- **Conway block-body vkey-witness closure** *(B2-carried)*.
- **Conway governance certificate accumulation** *(B5)*.
- **Credential discriminant extension** *(declared non-goal)*.
- **Committee-enactment write-back** *(ENACTMENT)*.
- **Conway tx-body `proposal_procedures` decode** *(PP — wired)*.
- **TPraos full-block validity** *(B1 extension point)*.
- **TPraos producer** *(N-C declared non-goal — OQ-4 lock)*: a
  future TPraos producer would add a closed `era` dispatch arm to
  `forge_block` + per-era body buckets. Today Conway/Praos only.
- **New `GovAction` / Plutus version variant**.
- **New `SignerSource` / `TxRejectClass` / `BlockRejectClass` /
  `OrderPolicy` variant**.
- **New protocol parameter field**.
- **New `ProducerTick` field** *(N-C extension point — DC-CONS-13
  strengthening)*: shape changes to the canonical input value are
  version-gated; new fields preserved by replay corpora; CI guards
  preserved (no `#[non_exhaustive]`, no private-key fields).
- **New `ForgeError` / `SchedulerInput` / `SchedulerEffect` variant**:
  closed sums; variant additions are version-gated; no
  `#[non_exhaustive]`, no `String`-bearing variants permitted.
- **New `SelfAcceptError` variant** *(N-C extension point —
  CN-CONS-07 strengthening)*: closed sum; variant additions are
  version-gated; today the sum is the single `Rejected(BlockValidityError)`
  arm — broadening would require both a normative slice and updated
  guards in `ci_check_self_accept_gate.sh`.
- **New CI check**: additive. (N-C added eight —
  `ci_check_private_key_custody.sh`, `ci_check_opcert_closed.sh`,
  `ci_check_forge_purity.sh`, `ci_check_no_private_keys_in_corpus.sh`,
  `ci_check_no_producer_body_encoder.sh`, `ci_check_self_accept_gate.sh`,
  `ci_check_scheduler_closure.sh`, `ci_check_producer_corpus_present.sh`.)
  N-C also modified `ci_check_constitution_coverage.sh` to allow
  enforcement evidence on release/operational entries when status
  is `enforced`.
- **Pinned external crate bump**: Tier-5 rationale doc required.
  **N-C consumed:** `cardano-crypto = "1.0.8"` (already pinned).
- **New mini-protocol** / **Mini-protocol version-table bump**.
- **New `ChainEvent` / `ChainSelectionReject` / `StreamInput` variant**.
- **New `NetworkMagic`** *(N-B)*.
- **New `LedgerView` impl / LedgerState-backed `PoolDistrView` constructor**.
- **`BootstrapAnchorHash` preimage v2** *(N-B)*: hard version-gated.
- **N2N/N2C tx-submission → `mempool_ingress` ingress** *(N-E)*.
- **Live cardano-node N2N block-fetch acceptance** *(N-C —
  `blocked_until_operator_stake_available` per `CN-CONS-06.open_obligation`)*:
  reopens when testnet SPO stake is provisioned; the live half is
  not a Tier-1 hash-critical invariant (the bytes-shape claim is
  mechanically closed via `cross_impl_adapter`).
- **Phase-4 cluster surface additions** (N-F): each cluster's wire
  surface gates additions via its own cluster doc.

---

## 5. Module Addition Rules

Ade's workspace is small and color-disciplined. **PHASE4-N-C added
five new BLUE submodules** (`ade_codec::shelley::opcert`,
`ade_core::consensus::opcert_validate`, `ade_ledger::block_body_hash`,
`ade_ledger::producer::{forge, self_accept, state}`,
`ade_ledger::producer` mod root), **five new RED submodules**
(`ade_runtime::producer::{signing, keys, scheduler, broadcast}` +
the producer mod root), **one new GREEN submodule**
(`ade_runtime::producer::tick_assembler`), **one new GREEN testkit
submodule** (`ade_testkit::producer` with `fixtures`, `replay`,
`reference_vectors`, `cross_impl_adapter`), **one new operator-action
probe binary** (`ade_core_interop::bin::live_block_production_session`),
**eight new CI gates**, **fourteen new registry rules**, **one
strengthening of two carried rules** (`T-DET-01.strengthened_in +=
PHASE4-N-C`, `T-ENC-01.strengthened_in += PHASE4-N-C`), and **one
modification of a carried CI gate** (`ci_check_constitution_coverage.sh`
to allow enforcement evidence on release/operational entries when
status is `enforced`). N-C added **no new crate**, **no new external
ingress wire-format frozen contract beyond the closed standalone
opcert grammar**, **no new public composer outside the producer
authority surface**.

**N-C also added one new cross-color dependency edge**:
`ade_runtime → ade_ledger` (RED → BLUE), required because the
producer's RED scheduler and broadcast queue consume the BLUE
`AcceptedBlock` token from `ade_ledger::producer::self_accept`. The
edge passes `ci_check_dependency_boundary.sh` — RED depending on
BLUE is the intended direction (BLUE → RED would close a Cargo cycle
and is forbidden).

**The host-crate decision for the producer authority surface**
(recorded in `docs/clusters/PHASE4-N-C/N-C-S3.md` §Host-crate decision):
the producer's BLUE core was originally planned at
`ade_core::consensus::forge` but relocated to `ade_ledger::producer`
because `ade_ledger` already depends on `ade_core` and the forge body
needs `ade_ledger::{state::LedgerState, mempool::admit::*,
block_body_hash::*}` — the inverse import would close a Cargo cycle.
BLUE classification is unchanged.

**The module-addition rule N-C sets for future producer-domain work:**

1. **A new producer authority sub-module attaches inside
   `ade_ledger::producer`** (sibling of `forge`, `self_accept`,
   `state`). The module MUST be BLUE: no clock, no rand, no I/O, no
   `HashMap`, no `std::env`, no `std::fs`. Defended by
   `ci_check_forge_purity.sh`.
2. **A new RED producer-runtime sub-module attaches inside
   `ade_runtime::producer`** (sibling of `signing`, `keys`,
   `scheduler`, `broadcast`). The module MAY use clocks / I/O /
   async / `tokio` — but MUST NOT export private-key types into
   non-`producer/` paths.
3. **A new GREEN producer glue module attaches inside
   `ade_runtime::producer`** (sibling of `tick_assembler`). The
   module MUST be a pure function over its inputs; MUST NOT invoke
   signing primitives; MUST NOT read I/O; MUST produce byte-identical
   outputs across replays.
4. **A new producer state-machine input variant attaches to
   `SchedulerInput`** — not as a parallel state machine, not as a
   side-channel. New trigger = closed-sum extension.
5. **A new producer effect variant attaches to `SchedulerEffect`**
   — closed-sum extension; no `#[non_exhaustive]`.
6. **A new producer authority registry rule attaches as a derived
   `DC-*` family entry** with `code_locus`, `ci_script`, `tests`,
   `cross_ref`. Bidirectional cross-refs to consumed rules
   (e.g. DC-CONS-15 cross-refs DC-CONS-03 — the validator-shared
   leader function — bidirectionally).
7. **A new operator-action probe binary attaches inside
   `crates/ade_core_interop/src/bin/`** following the
   `live_<surface>_session` naming + hermetic-default-plus-`--connect`-live
   shape. The binary MUST stub its live socket halt when stake /
   external dependency is unavailable; capture status via the
   `blocked_until_operator_stake_available` mode if the live evidence
   cannot complete.

### Cross-cluster obligation pattern (carried — strengthened in N-C close)

**N-C strengthens the cross-cluster obligation pattern with a third
closure mode**: `blocked_until_operator_stake_available`. The mode
applies when an obligation's blocker is not Ade-internal — e.g.
testnet SPO stake registration must be provisioned by the operator
before live evidence can be captured. The mode follows OP-OPS-04's
precedent (`enforced` + `open_obligation` for a follow-on artifact).
The mechanical half MUST be closed on the same HEAD (e.g. N-C's
mechanical cross-impl adapter closes CN-CONS-06's bytes-shape claim).
**Re-opens on operator availability** — the procedure doc names the
specific blocker and the re-open criteria.

### Operator-action evidence pattern (carried — strengthened in N-C close)

N-C adds the **third instance** of the operator-action probe binary
family: `live_block_production_session`. The pattern is now
established across three Tier-1 wire-level seams (chain-sync, tx
ingress, block production) — each with a hermetic default that runs
in CI without network access, a `--connect <peer>` live pass that
the operator runs against a real cardano-node peer, and a captured
evidence log committed alongside the procedure doc. **N-C also
introduces the third closure mode** (`blocked_until_operator_stake_available`)
into the pattern's frozen rules.

| Color | Naming convention | Build-config flags | May depend on | MUST NOT depend on |
|-------|-------------------|--------------------|----------------|--------------------|
| **BLUE** | `ade_*` | First line of every `.rs` is the contract banner. `lib.rs` carries `#![deny(unsafe_code, clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_arithmetic)]`. No `#[cfg(feature = ...)]`. No async. **N-C:** `ProducerTick` closed 14-field struct; `forge_block` body has no clock / rand / `HashMap` / `std::env` / `std::fs` / async / `println!`; no `String`-bearing variant on `ForgeError` / `ForgeEffects`; no `#[non_exhaustive]` on closed sums (DC-CONS-13). `AcceptedBlock.bytes` private; `pub fn ... -> Result<AcceptedBlock, ...>` count = 1 across crates/ (CN-CONS-07). `opcert_validate` is the sole RED→BLUE opcert acceptance chokepoint (DC-CONS-11/12). `block_body_hash_from_buckets` is the single canonical body-hash recipe (DC-CONS-16). | Other BLUE crates / submodules only | Any RED submodule or crate; GREEN in non-dev deps; `pallas_*` (except `ade_plutus`); async runtime; `HashMap`/`HashSet`/`IndexMap`; clock/rand/float/env/I/O. **N-C:** no `*SigningKey` / `KesSecret` / `ColdSigningKey` types (defended by `ci_check_private_key_custody.sh`). |
| **GREEN** | `ade_*` | Banner + deny attrs are project convention. **N-C:** `tick_assembler` is a pure function — no I/O, no clocks, no signing-primitive calls. Identical `(slot, base_state, mempool, inputs)` → byte-identical `ProducerTick` (DC-CONS-14). `cross_impl_adapter` is a structural-agreement harness — decode round-trip + body-hash binding via S4's authority + field agreement. | BLUE crates + standard library + ecosystem crates | `ade_runtime` (for `ade_testkit`); RED submodules in non-test paths. Results must never feed back into a BLUE authoritative decision. |
| **RED** | `ade_*` | No special header. Free to use clocks, I/O, async, `HashMap`, signing keys. The operator-action probe binaries in `crates/ade_core_interop/src/bin/` (now `live_consensus_session`, `live_tx_submission_session`, `live_block_production_session`) follow the hermetic-default / `--connect`-live pattern. **N-C:** `ade_runtime::producer::{signing, keys, scheduler, broadcast}` are RED; `signing` holds `*SigningKey` / `KesSecret` / `ColdSigningKey` with `zeroize`-on-drop and no `pub` raw-byte accessors; `scheduler_step` is a pure RED state transition (wall-clock + I/O live in the outer driver); `BroadcastQueue::enqueue` consumes `AcceptedBlock` by value. | Any BLUE / GREEN crate or submodule (one-way). **N-C added the `ade_runtime → ade_ledger` edge** (RED → BLUE via the `AcceptedBlock` token). | Cannot be depended on by BLUE. |

### New module checklist

1. **Add to `Cargo.toml` workspace members** (if a new crate).
2. **Declare TCB color** by editing `.idd-config.json` `core_paths` if BLUE.
3. **CI script update obligations** — extend the relevant BLUE-scoped
   scripts; for producer-domain sub-modules, model the new CI gate
   on `ci_check_forge_purity.sh` / `ci_check_self_accept_gate.sh`
   shape (closure proof + private-field proof + closed-sum proof +
   no-re-implementation proof).
4. **Add contract banner** (BLUE) to every `.rs` file.
5. **Add deny attributes** to `lib.rs` (BLUE).
6. **New canonical types:** add a `[[rules]]` block under family `T`
   in the invariant registry, plus a round-trip test. For new
   producer-domain authority rules, append `DC-CONS-1X` /
   `DC-CRYPTO-0X` / `OP-OPS-0X` with bidirectional cross-ref to
   consumed rules. T-DET-01 / T-ENC-01 may receive a `strengthened_in`
   entry when the new module participates in their byte-deterministic
   / byte-authoritative properties.
7. **New operator-action probe binary:** add to
   `crates/ade_core_interop/src/bin/<name>.rs` following the
   `live_<surface>_session` naming + hermetic-default-plus-`--connect`-live
   shape; document in `<cluster>/CE-<id>_PROCEDURE.md`; capture
   evidence to `<cluster>/CE-<id>_<date>.log` OR mark
   `blocked_until_operator_stake_available` if the live evidence
   depends on an external dependency the operator must provision.
8. **Cross-cluster obligation:** follow the binding rules from the
   N-E full-close narrative; N-C strengthens the rules with the
   third closure mode (`blocked_until_operator_stake_available`).
9. **Run `cargo test --workspace` and the full CI script suite.**

### Phase 4 anticipated additions

- **PHASE4-N-C — FULLY CLOSED at this HEAD** (mechanical half +
  structural cross-impl): code + CI gates + DC-CRYPTO-03/04/05 +
  DC-CONS-11/12/13/14/15/16 + DC-LEDGER-12 + CN-CONS-06/07 +
  OP-OPS-04/05 + 8 new CI scripts. CE-N-C-8 live-evidence is
  `blocked_until_operator_stake_available` per `CN-CONS-06.open_obligation`
  — re-opens on operator availability.
- **PROPOSAL-PROCEDURES-DECODE — FULLY CLOSED** (carried).
- **PHASE4-N-E — FULLY CLOSED** (carried).
- **Future node-binary cluster (`CE-NODE-N2C-LTX`)**: live N2C UDS
  server + N2N bulk-tx inbound listener (carried).
- **Future cluster: live cardano-node block-fetch acceptance of
  Ade-forged blocks (CE-N-C-8 re-open trigger)**: reopens when
  testnet SPO stake is provisioned; the procedure is documented at
  `docs/clusters/PHASE4-N-C/CE-N-C-8_PROCEDURE.md`.
- **N-A successor: N2N producer-side block-fetch server role +
  chain-sync extension**: outbound delivery of broadcast-queued
  `AcceptedBlock` bytes. Declared OUT-OF-SCOPE in N-C; the broadcast
  queue is the upstream side of the handoff.
- **Tx-validity completeness follow-ups**: full `track_utxo=true`
  corpus; pre-Conway eras; the Conway block-body vkey-witness
  closure (carried).
- **PP OQ-1..OQ-4 follow-ups**: voting_procedures decode /
  ParameterChange.update nested / NewConstitution.raw nested /
  typed RewardAccount (carried).
- **header→body bridge (receive-side)**: `ade_node` composition
  layer joining `process_stream_input` and `block_validity` for
  externally-arriving headers (carried).
- **N-F (operator API)**: thin RED layer mapping a closed Query
  enum to gRPC/HTTP.

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
- **(N-A specific)** Carried.
- **(N-B specific)** Carried.
- **(B1 specific)** Carried.
- **(B2 specific)** Carried.
- **(B3 / B4 / B5 specific)** Carried.
- **(OQ5 / COMMITTEE / DREP / ENACTMENT-COMMITTEE-WRITEBACK)** Carried.
- **(N-E specific — closed BLUE chokepoint `mempool_ingress`)** Carried.
- **(PP specific — closed BLUE sub-grammar `decode_proposal_procedures`)** Carried.
- **(N-C-S1 specific — RED-confined private-key custody)** No
  `pub struct .*SigningKey` outside `ade_runtime::producer::*`. No
  `*SigningKey` / `KesSecret` / `ColdSigningKey` type in `ade_core` /
  `ade_codec` / `ade_types` / `ade_ledger` / `ade_crypto` public
  APIs. No `pub fn` in `ade_runtime::producer::signing` returning a
  raw `[u8; N]` / `Vec<u8>`; every `pub fn` returns a typed wrapper
  or `Result<typed, _>`. No `Default` impl for `*SigningKey`,
  `KesSecret`, `ColdSigningKey`. Defended by
  `ci_check_private_key_custody.sh`.
- **(N-C-S2 specific — closed BLUE opcert byte authority)** No
  parallel opcert encoders / decoders outside
  `ade_codec::shelley::block` + `ade_codec::shelley::opcert` +
  `ade_core::consensus::opcert_validate`. No catch-all accept in
  `decode_opcert`. No bypass of `opcert_validate` for the RED→BLUE
  opcert acceptance step. No `String`-bearing variant on
  `OpCertCodecError` / `OpCertError`. Defended by
  `ci_check_opcert_closed.sh`.
- **(N-C-S3 specific — closed BLUE producer transition `forge_block`
  + canonical input value `ProducerTick`)** No clock / rand /
  `HashMap` iteration / `std::env` / `std::fs` / async / `println!`
  in `forge.rs` / `state.rs` / `tx_components.rs`. No
  `VrfDraft03::prove` / `Sum6Kes::sign_kes` / `KesAlgorithm::sign_kes`
  / `update_kes` call inside `ade_ledger/src/producer/` or
  `ade_core/src/`. No `#[non_exhaustive]` on `ProducerTick` /
  `ForgeError` / `ForgeEffects` / `ForgedBlock`. No `String`-bearing
  variant on `ForgeError` / `ForgeEffects`. No private-key field on
  `ProducerTick`. No second public producer-side leader-check
  function — `is_leader_for_vrf_output` is the only sanctioned source
  of leader truth. No tx in a forged block that bypasses the
  mempool-admit prefix replay. No private-key bytes in producer
  replay corpora. Defended by `ci_check_forge_purity.sh` +
  `ci_check_no_private_keys_in_corpus.sh`.
- **(N-C-S4 specific — single canonical body-hash authority)** No
  new `pub fn .*encode_block_body` outside the canonical authority
  in `ade_codec::shelley::block`. No parallel body-hash recipe —
  `block_body_hash::block_body_hash_from_buckets` is the only
  function in the workspace that computes the recipe. Both producer
  and validator hash through it. Defended by
  `ci_check_no_producer_body_encoder.sh`.
- **(N-C-S5 specific — closed BLUE self-accept bridge + closed
  type-level broadcast token `AcceptedBlock`)** No `pub fn ... ->
  AcceptedBlock` / `... -> Result<AcceptedBlock, _>` outside the
  `self_accept` function in `self_accept.rs` (CI guard 1b: exactly
  one such return-type match across crates/). No `pub bytes:` on
  `AcceptedBlock`. No `#[non_exhaustive]` on `SelfAcceptError`. No
  `String`-bearing variant on `SelfAcceptError`. No `impl Default
  for AcceptedBlock` / `impl From<.*> for AcceptedBlock` /
  `impl TryFrom<.*> for AcceptedBlock`. No call to
  `validate_and_apply_header(` / `decode_block(` /
  `block_body_hash(` from `self_accept.rs` production source —
  `self_accept` MUST delegate to the canonical `block_validity`
  chokepoint, never re-implement validator sub-steps. No `pub fn`
  in `self_accept.rs` returning raw `Vec<u8>` / `&[u8]` outside the
  `as_bytes` / `into_bytes` accessors on the token. Defended by
  `ci_check_self_accept_gate.sh` (6 guards).

### GREEN (`ade_testkit` incl. `producer` corpus + carried sub-trees; `ade_runtime::consensus::{candidate_fragment, chain_selector}`; `ade_ledger::mempool::{policy, canonicalize}`; the two `ade_core_interop` N-E bridges; **`ade_runtime::producer::tick_assembler` — NEW in N-C-S6**)

- No nondeterminism that leaks into stored fixtures — fixtures must
  be byte-reproducible.
- No participation in authoritative outputs.
- No `HashMap` even in test helpers — `BTreeMap` only.
- No import of `ade_runtime` from `ade_testkit`.
- (carried bullets per prior revision)
- **(`ade_runtime::producer::tick_assembler`, NEW in N-C-S6)** No I/O;
  no clocks; no nondeterminism. The function MUST be observably
  deterministic: identical `(slot, base_state, mempool, inputs)`
  MUST produce byte-identical `ProducerTick` outputs across replays
  (DC-CONS-14). MUST NOT invoke signing primitives (RED-only). MUST
  NOT branch on wall-clock or environment state. MUST NOT mutate the
  base ledger or mempool. The closed `TickInputs` struct carries
  signed artifacts only — no private-key types.
- **(`ade_testkit::producer::{fixtures, replay, reference_vectors,
  cross_impl_adapter}`, NEW in N-C-S1..S7)** No private-key bytes in
  any corpus fixture (defended by `ci_check_no_private_keys_in_corpus.sh`).
  Fixtures carry signed artifacts (`VrfProof`, `KesSignature`,
  `OpCert`) only. `cross_impl_adapter` covers the bytes-shape claim
  only (decode round-trip + body-hash binding + structural field
  agreement); the crypto-level cross-impl claim is operator-action.

### RED (`ade_runtime`, `ade_node`, `ade_network::mux::transport`, `ade_network::session`, `ade_network::bin::capture_*`, `ade_runtime::consensus::genesis_parser`, `ade_core_interop` (incl. N-C S7 probe binary `live_block_production_session`), and the RED-behavior `ade_ledger::consensus_input_extract` scan; **`ade_runtime::producer::{signing, keys, scheduler, broadcast}` — NEW in N-C-S1/S6**)

- No direct mutation of `ade_ledger` state — all transitions go
  through `ade_ledger::rules::*`, the `block_validity` / `tx_validity`
  composers, `mempool::ingress::mempool_ingress`, or **the new
  producer authority chokepoints `producer::forge::forge_block` +
  `producer::self_accept::self_accept`** (the BLUE composers that
  RED dispatches through).
- No bypassing `ade_codec` to construct semantic types from raw bytes.
  **(PP-strengthened)** Constructing `ProposalProcedure` from raw
  bytes outside `decode_proposal_procedures` is CI-forbidden.
  **(N-C-strengthened)** Constructing `AcceptedBlock` outside
  `self_accept` is CI-forbidden (`ci_check_self_accept_gate.sh`
  guard 1).
- (`ade_runtime` specifically) No dep on `ade_ledger` was permitted
  prior to N-C; **N-C added the `ade_runtime → ade_ledger` edge**
  (required to consume the BLUE `AcceptedBlock` token + the BLUE
  `forge_block` / `self_accept` chokepoints from RED scheduler +
  broadcast). The edge passes `ci_check_dependency_boundary.sh`
  (RED → BLUE is the intended direction). No leakage of `redb`
  types. No second public `chaindb` path.
- (`ade_network::mux::transport`) No protocol logic.
- (`ade_network::session`) Composition glue only.
- (`ade_network::bin::capture_*`) Live-interop tools only.
- (`ade_runtime::consensus::genesis_parser`) No re-derivation of the
  bootstrap anchor outside `compute_anchor_hash`.
- (`ade_ledger::consensus_input_extract`) Pure-over-bytes.
- (N-E live N2N operator-action session) Carried.
- (Deferred RED operator-action surfaces — CE-NODE-N2C-LTX) Carried.
- (`ade_core_interop`) Live-interop driver only; library tests
  `#[ignore]`-gated. The N-E GREEN bridges in this crate are
  deterministic pure functions; the live socket loops are
  operator-action. **N-C added `live_block_production_session`** —
  third operator-action probe binary in the family. The binary's
  default mode prints readiness and exits (hermetic, runs in CI
  without network access); `--connect` performs the live pass against
  a real cardano-node peer.
- **(N-C-S1 specific — `ade_runtime::producer::signing` + `::keys`)**
  No `pub` raw-byte accessor on `VrfSigningKey` / `KesSecret` /
  `ColdSigningKey`. `zeroize`-on-drop. No `Default` impl. No
  `Display` / `Debug` that emits key bytes. The key loader reads
  cardano-cli `*.skey` text envelopes only — no key generation, no
  network fetch (OP-OPS-04, OQ-1 lock).
- **(N-C-S6 specific — `ade_runtime::producer::scheduler`)** The
  scheduler core MUST be pure RED state transition: wall-clock + I/O
  live in the outer driver, not inside `scheduler_step`. No silent
  recovery from a halt — once `halted = Some(reason)`, subsequent
  `SlotTick` inputs are ignored and re-emit the original halt
  reason. No `#[non_exhaustive]` on `SchedulerInput` /
  `SchedulerEffect` / `SchedulerHaltReason`. No `String`-bearing
  variant.
- **(N-C-S6 specific — `ade_runtime::producer::broadcast`)** `enqueue`
  MUST take `AcceptedBlock` by value (move semantics — type-level
  gate). No `enqueue` overload taking `&[u8]` / `&AcceptedBlock` /
  `Vec<u8>`. No FIFO ordering bypass.
- **(N-C-S7 specific — `live_block_production_session`)** The live
  socket loop MUST drive the RED scheduler → GREEN tick-assembler →
  BLUE forge → BLUE `self_accept` pipeline through the canonical
  chokepoints — no parallel admission path, no direct construction
  of `AcceptedBlock`, no bypass of `self_accept`. The live evidence
  log committed alongside the procedure doc redacts hostnames per
  `feedback_no_credential_leaks`.

### Project-specific additions

- **No commits of credentials, hostnames, IPs, private keys** —
  enforced by `ci_check_no_secrets.sh`. **N-C-strengthened:** no
  private-key bytes in producer replay corpora
  (`ci_check_no_private_keys_in_corpus.sh`).
- **No `Phase 4 internal-mode mock network`** — Tier 1 surfaces must
  be exercised against real cardano-node peers. **N-C:** the
  mechanical cross-impl harness `cross_impl_adapter` is a
  structural-agreement harness over a synthetic corpus (bytes-shape
  claim only); the crypto-level cross-impl claim requires
  operator-action live evidence per CE-N-C-8.
- **No collapsing wire and canonical bytes** — dual-authority rule.
- **No Tier 5 surface without a stated rationale**.
- **No "we'll match it later" stubs on Tier 1 surfaces** — Tier 1
  closure is hard-gated. The producer authority surface (forge +
  self-accept + opcert + body-hash) is now Tier 1; the eight new CI
  gates enforce mechanical closure. **Cross-cluster obligation
  deferrals (CE-NODE-N2C-LTX) are NOT "we'll match it later"
  stubs**. **The N-C `blocked_until_operator_stake_available` status
  is NOT a "we'll match it later" stub either** — the mechanical
  half is fully enforced at this HEAD; the live half is recorded as
  an `open_obligation` on `CN-CONS-06`, tied to a specific
  operator-action procedure (`CE-N-C-8_PROCEDURE.md`), and reopens
  on a named external dependency (testnet SPO stake registration).
  Follows OP-OPS-04's precedent.

---

## Cross-references

- CODEMAP: `docs/ade-CODEMAP.md` — module-by-module authority table,
  upstream of this document. **Cross-reference check at this HEAD:**
  CODEMAP is being regenerated in parallel; pending the regen,
  CODEMAP may pin pre-N-C HEAD. The new BLUE submodules
  (`ade_codec::shelley::opcert`, `ade_core::consensus::opcert_validate`,
  `ade_ledger::block_body_hash`, `ade_ledger::producer::{forge,
  self_accept, state}`), the new RED submodules
  (`ade_runtime::producer::{signing, keys, scheduler, broadcast}`),
  the new GREEN submodules (`ade_runtime::producer::tick_assembler`,
  `ade_testkit::producer::*`), and the new operator-action probe
  binary (`ade_core_interop::bin::live_block_production_session`)
  are not yet in the prior CODEMAP. The next CODEMAP regen picks
  these up mechanically. CI count moves from 32 → 40.
- Invariant registry: `docs/ade-invariant-registry.toml` — rule
  families incl. T / CN / DC / OP / RO. **N-C added:**
  `DC-CRYPTO-03/04/05` (`enforced`, `ci_script =
  ci/ci_check_private_key_custody.sh`,
  `introduced_in = PHASE4-N-C`); `DC-CONS-11/12` (`enforced`,
  `ci_script = ci/ci_check_opcert_closed.sh`); `DC-CONS-13/14/15`
  (`enforced`, `ci_script = ci/ci_check_forge_purity.sh` +
  `ci/ci_check_no_private_keys_in_corpus.sh`); `DC-CONS-16`
  (`enforced`, `ci_script = ci/ci_check_no_producer_body_encoder.sh`);
  `DC-LEDGER-12` (`enforced`, `ci_script = ci/ci_check_forge_purity.sh`);
  `CN-CONS-06` (`enforced` + `open_obligation =
  blocked_until_operator_stake_available`, `ci_script =
  ci/ci_check_producer_corpus_present.sh`); `CN-CONS-07`
  (`enforced`, `ci_script = ci/ci_check_self_accept_gate.sh`);
  `OP-OPS-04` (`enforced` + `open_obligation =
  cardano-cli Sum6KES skey path`, `ci_script =
  ci/ci_check_private_key_custody.sh`); `OP-OPS-05` (`enforced`,
  `ci_script = ci/ci_check_scheduler_closure.sh`); appended
  `PHASE4-N-C` to `T-DET-01.strengthened_in` +
  `T-ENC-01.strengthened_in`. Total: 176 → 190 entries.
- Phase 4 cluster plan: `docs/active/phase_4_cluster_plan.md`.
- Tier doctrine: `docs/active/CE-79_gate_statement.md` and
  `docs/active/CE-79_tier5_addendum.md`.
- Cluster N-D / N-A / N-B / B1 / B2 / B3 / B4 / B5 / OQ5-CREDENTIAL-FIDELITY
  / COMMITTEE-CRED-FIDELITY / DREP-VOTE-FIDELITY /
  ENACTMENT-COMMITTEE-FIDELITY / ENACTMENT-COMMITTEE-WRITEBACK /
  PHASE4-N-E / PROPOSAL-PROCEDURES-DECODE: all closed; cluster docs
  carried.
- **Cluster PHASE4-N-C (CLOSED at this HEAD; mechanical half + structural
  cross-impl)**: the cluster doc + slices `cluster.md, N-C-S{1..7}.md`
  + `CE-N-C-8_PROCEDURE.md` at `docs/clusters/PHASE4-N-C/`. WIRES AND
  CLOSES block production end-to-end: RED signing primitives + key
  loader (S1), BLUE `opcert_validate` chokepoint + closed-grammar
  opcert encoder authority (S2), BLUE `forge_block` core +
  `ProducerTick` canonical input value + tx-admissibility prefix (S3),
  unified BLUE body-hash recipe authority (S4), BLUE `self_accept`
  bridge + `AcceptedBlock` type-level broadcast gate (S5), RED
  scheduler + GREEN tick-assembler + RED broadcast queue (S6),
  mechanical cross-impl adapter + `live_block_production_session`
  operator-action probe binary (S7). Added eight CI scripts (count
  32 → 40); added fourteen derived-Cardano / consensus / release /
  operational registry rules (total 176 → 190); strengthened two
  carried universal rules (T-DET-01, T-ENC-01); modified
  `ci_check_constitution_coverage.sh` to allow enforcement evidence
  on release/operational entries when status is `enforced`.
  **CE-N-C-8 live-evidence `blocked_until_operator_stake_available`**
  per `CN-CONS-06.open_obligation`; mechanical bytes-shape claim is
  closed by `cross_impl_adapter`. Three operator-action probe
  binaries now in the family: `live_consensus_session` (N-B),
  `live_tx_submission_session` (N-E S6), `live_block_production_session`
  (N-C S7).
- **Future obligation: `CE-N-C-8`** — operator-action live evidence
  for crypto-level cross-impl; reopens on testnet SPO stake
  registration availability.
- **Future obligation: `CE-NODE-N2C-LTX`** — the node-binary
  cluster's live N2C UDS server + N2N bulk-tx inbound listener;
  carried from N-E.
