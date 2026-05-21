# Seams — Where New Work Can Attach (Ade)

> **Status:** Living architectural document. Regenerated; not hand-edited.
> Per-project instance of `~/.claude/methodology/templates/seams.md`.

> 11 crates, 27 CI checks at HEAD (`ee35493`).
> Reads CODEMAP for the module list and TCB colors; reads the invariant
> registry (`docs/ade-invariant-registry.toml`) for rule IDs; reads the
> Phase 4 cluster plan (`docs/active/phase_4_cluster_plan.md`), the
> closed N-D / N-A / N-B / B1 / B2 / B3 cluster docs, and the
> PHASE4-B4 cluster doc plus its slice
> (`docs/clusters/PHASE4-B4/cluster.md`,
> `docs/clusters/PHASE4-B4/B4-S1.md`).
>
> **This is a PHASE4-B4 close refresh (HEAD `ee35493`).** The body was
> fully regenerated at PHASE4-B3 close (`7784bf8`) and folded in the B3F
> hardening deltas (`193d2fc`); this revision folds in the PHASE4-B4
> deltas — the **owner-complete `ConwayCert`** (every variant now retains
> all owner payloads, not just deposit/refund fields), the new closed
> Conway apply model in `ade_ledger::delegation`
> (`ConwayCertAction` / `GovernanceCertEffect` / `GovernanceOwner` /
> `OwnerTaggedEffect` / `ConwayCertOutcome` / `ConwayCertEnv`), the closed
> `DRep` decode grammar in `decode_drep`, the **single shared pool-params
> decoder** `read_pool_registration_cert` (no second Conway decoder), the
> **era-dispatched fail-closed cert-state accumulator**
> `ade_ledger::rules::accumulate_tx_certs`, and **THE KEY NEW SEAM** — the
> owner-tagging of governance-affecting Conway certs (vote-delegation,
> committee, DRep) to `ConwayGovState` via `OwnerTaggedEffect` /
> `ConwayCertOutcome.owner_tagged`, routed OUT of B4's mutation scope as
> the confirmed extension point where the declared PHASE4-B5 cluster
> attaches. DC-LEDGER-08 is `enforced`. **B4 added no new CI script**
> (DC-LEDGER-08 reuses the full-BLUE `ci_check_forbidden_patterns.sh` plus
> the compiler-exhaustive `match` + the named B4 test set). The prior B3F
> "closed enums grep-gated" resolution stands; the new B4
> `ade_ledger::delegation` owner-tagged apply types are
> compiler-exhaustive-match + test-and-review-enforced (a narrow gap,
> surfaced below).

Ade is a Cardano block-producing node. Its closure surface is dominated
by two facts:

1. The Cardano protocol fixes wire bytes and hashes for hash-critical
   paths (Tier 1 — must-conform). New work that touches those bytes
   has essentially no degrees of freedom.
2. Everything operator-facing — storage layout, query API, telemetry,
   packaging — is Tier 5: deliberate divergence "in our own image"
   (per `docs/active/CE-79_tier5_addendum.md`).

This document names where the system opens and where it stays closed.

**PHASE4-B3 (Full Conway tx value-conservation accounting) just closed.**
It closed the deposit/refund/withdrawal value-conservation follow-up B2
deliberately deferred. It added: a **closed Conway certificate CDDL
grammar** (`ade_codec::conway::cert::decode_conway_certs` over tags
`0..18`, with `CodecError::UnknownCertTag` for tags ≥19, `RemovedInConway`
for tags 5/6, and **no catch-all accept arm**); a **closed withdrawals
map grammar** (`ade_codec::conway::withdrawals` rejecting a repeated key
with `CodecError::DuplicateMapKey` — never last-wins); the **closed
`ConwayCert` / `CertDisposition` / `DepositEffect` / `CoinSource` sum
types** in `ade_types::conway::cert` plus `RewardAccount` in
`ade_types::tx`; a **canonical-only deposit-parameter surface**
(`ConwayOnlyDepositParams` / `ConwayDepositParams` in `ade_ledger::pparams`,
`LedgerState.conway_deposit_params` + `conway_deposit_view()` in
`ade_ledger::state`, DC-TXV-07, enforced by the new
`ci_check_deposit_param_authority.sh`); the **closed total cert
classifier** `ade_ledger::cert_classify::classify` (DC-TXV-06); and the
**full preservation-of-value equation** in
`ade_ledger::conway::check_conway_coin_conservation` with the **frozen
§9.1 reject precedence** (decode → era-validity → missing-environment →
state-dependent-accounting → conservation). The B2 cert/withdrawal
early-out (the known false-accept path) is **removed**. Registry rules
`DC-TXV-06` / `DC-TXV-07` flipped to `enforced`; `T-CONSERV-01` /
`CN-LEDGER-07` and `DC-VAL-06` were strengthened. **B3 added no new
crate, no new ingress surface, and no new public composer** — every new
BLUE module lives under the already-BLUE `ade_codec` / `ade_types` /
`ade_ledger` crate prefixes. The B2 single-tx composition root
(`tx_validity`) and the B1 block composition root (`block_validity`)
remain the upstream context for everything B3 added; B3 tightened the
phase-1 state-backed authority they share (`validate_conway_state_backed`),
not the composers.

**PHASE4-B4 (Conway certificate-state accumulation, fail-closed) just
closed.** It made the B3-introduced `ConwayCert` **owner-complete** — every
variant over tags `0..18` now retains all owner payloads (stake / DRep /
committee credentials, pool id, the full `PoolRegistrationCert` incl.
`pool_owners`, DRep delegation targets), not just the deposit/refund fields
B3's conservation projection needed; it added the closed `decode_drep`
grammar (no catch-all) and relocated the single shared pool-params decoder
to `ade_codec::shelley::cert::read_pool_registration_cert`, called by
**both** the Shelley and Conway cert decoders — a **no-new-parallel-decoder**
rule. It added the native owner-tagged Conway apply model in
`ade_ledger::delegation` (`apply_conway_cert` + the closed action classifier
`conway_cert_action`, plus the closed sum types `ConwayCertAction`,
`GovernanceCertEffect`, `GovernanceOwner`, `OwnerTaggedEffect`,
`ConwayCertOutcome`, `ConwayCertEnv`) and the **era-dispatched, fail-closed**
accumulator `ade_ledger::rules::accumulate_tx_certs` (Conway →
`decode_conway_certs` + `apply_conway_cert`; Shelley..Babbage →
`decode_certificates` + `apply_cert`), removing the prior `_era` discard and
**both fail-open swallows** in `process_block_certificates` — a decode/apply
error now propagates as a structured `LedgerError` and halts the block
transition. **THE KEY NEW SEAM:** governance-affecting Conway certs
(vote-delegation, committee auth/resign, DRep register/unregister/update) are
decoded fully and **owner-tagged to `ConwayGovState`** via `OwnerTaggedEffect`
/ `ConwayCertOutcome.owner_tagged`, then routed OUT of B4's mutation scope —
observed and returned, never silently neutralized and never applied here. B4
owns delegation/pool `CertState` only; gov-state accumulation is the
**PHASE4-B5** seam (declared in the B4 cluster doc and the registry
DC-LEDGER-08 statement). Registry rule `DC-LEDGER-08` is `enforced`; its
`ci_script` is the existing full-BLUE `ci_check_forbidden_patterns.sh` (no new
gate). **B4 added no new crate, no new ingress surface, and no new public
composer** — every new BLUE module lives under the already-BLUE `ade_codec` /
`ade_types` / `ade_ledger` crate prefixes; the change is a tightening of the
block-body cert path that `apply_block_with_verdicts` runs at `track_utxo`,
not a new composition root.

---

## 1. Surface Reduction Rules

> External inputs reduce to canonical form before entering authoritative
> pipelines. At HEAD there are six fully-wired *external* ingress
> surfaces (block bytes, Plutus script bytes, snapshot bytes, Ouroboros
> mux frames, genesis JSON bundles, and chain-selector stream inputs),
> plus the two **internal composition roots** (`block_validity` from B1,
> `tx_validity` from B2), the **mempool admission gate** (`mempool::admit`,
> a Tier-1 surface over `tx_validity`), and the **consensus-input
> extraction surface** (snapshot `state` CBOR tail-scan from B1), plus the
> remaining surfaces named in the Phase 4 plan (forge, query API, and the
> not-yet-wired N2N/N2C tx-submission ingress that will eventually feed
> `mempool::admit`).
>
> **B3 added no new ingress surface.** The new Conway cert array and
> withdrawals map are sub-grammars *inside* the already-existing standalone
> Conway tx CBOR surface and the block-body surface — they enter through
> the existing `tx_validity::decode_tx` / block-body decode paths and the
> `ade_codec` primitive set, never through a parallel decoder. The B2
> deferred "deposit/refund value conservation" candidate seam (named in the
> §1 candidate table of the prior revision) is now **wired**: it landed as
> a tightening of the phase-1 state-backed authority, exactly where the
> candidate flag predicted (see §2 "Conway value-conservation accounting").
>
> **B4 added no new ingress surface either.** Its owner-complete Conway
> cert decode is the SAME `decode_conway_certs` sub-grammar inside the
> already-existing Conway tx body and block-body surfaces — enriched (it now
> retains owner payloads + uses the new closed `decode_drep` and the shared
> `read_pool_registration_cert`) but not a new entry point. The new
> cert-state accumulation is reached only through the existing block-body
> chokepoint `apply_block_with_verdicts` (at `track_utxo`), via the new
> internal `accumulate_tx_certs` era-dispatcher; it is not a new public
> surface. **The one genuinely new seam B4 introduces is internal and
> declared:** the owner-tagged `ConwayGovState` effect channel
> (`ConwayCertOutcome.owner_tagged`) — see the confirmed-extension-point row
> in the §1 candidate table and §5 "Module Addition Rules".

### Surface: Single-tx validity (composition root — wired in B2)

```
Surface: A single Conway transaction (full tx CBOR
         [body, witness_set, is_valid, aux_data]) decided against a
         LedgerState (its track_utxo flag selects partial vs. full)
Reduces to: TxValidityVerdict { Valid { tx_id, applied } |
                                Invalid { class, error } }
            (defined in `ade_ledger::tx_validity::verdict`)
Pipeline (fixed step ordering — no reorder, no shortcut):
  1. ade_ledger::tx_validity::phase1::decode_tx(tx_cbor) -> DecodedTx
     (lifts the PRESERVED body slice → tx_id = blake2b_256(body_slice),
      the witness-set slice, the typed body, the raw vkey witnesses, and
      the script-presence WitnessInfo; Conway-only today — T-ENC-01)
  2. ade_ledger::tx_validity::phase1::tx_phase_one(ledger, &decoded)
     (the SHARED per-tx phase-1 authority — see §2; runs the witness
      closure UNCONDITIONALLY, then the UTxO-dependent state-backed
      checks ONLY at track_utxo=true; FAIL-FAST — DC-TXV-02. B3: the
      state-backed authority now runs the FULL value-conservation
      equation incl. cert deposits/refunds + withdrawals.)
  3. phase-2 (Plutus) via plutus_eval::try_evaluate_tx — ONLY when the
     tx carries Plutus scripts (decoded.witness_info.has_plutus()); a
     phase-2 failure maps into the closed TxValidityError::Phase2
     (DC-TXV-02; phase-2 never runs on a phase-1-failed tx)
  4. Valid -> evolve the UTxO via rules::apply_conway_tx_to_utxo;
     Invalid -> the input state is returned UNCHANGED (no partial
     mutation — DC-TXV-04)
Cross-surface state sharing: none — `tx_validity` is a pure total
  function fn(&LedgerState, &[u8]) -> TxValidityOutcome. The applied
  state is threaded by value through the outcome; nothing ambient
  (no arrival order, no clock, no HashMap iteration — DC-TXV-01).
```

**Rule.** `tx_validity` is the **single per-tx composition root**, the
exact parallel of B1's `block_validity`: a transaction is `Valid` **iff**
phase-1 accepts it **and** (when it carries Plutus scripts) phase-2
accepts it (DC-TXV-02). The ordering is normative — phase-1 is decided
first, phase-2 never runs on a phase-1-failed tx (DC-TXV-02). On any
Invalid outcome the input state is returned unchanged (DC-TXV-04).
`tx_validity` introduces **no new validation rules**: it is composition
only, joining the B2-S1 witness closure, the shared `tx_phase_one`
state-backed authority, and the existing Plutus phase-2 dispatch. The
function does not move and does not gain a second public entry; new work
tightens the authorities it composes (and the body authority `block_validity`
shares), not the composer. **B3 tightened the shared phase-1 authority:**
`validate_conway_state_backed` now enforces the full value-conservation
equation (cert deposits/refunds + withdrawals + donation) with the §9.1
reject precedence — the composer was untouched.

### Surface: Mempool admission (Tier-1 gate — wired in B2)

```
Surface: A candidate transaction offered to the mempool, against the
         mempool's accumulating LedgerState
Reduces to: AdmitOutcome { Admitted { tx_id } |
                           Rejected { class, error } }
            + a new MempoolState
            (defined in `ade_ledger::mempool::admit`)
Pipeline (fixed step ordering — no reorder, no shortcut):
  1. ade_ledger::mempool::admit(mempool, tx_cbor)
       -> (MempoolState, AdmitOutcome)
     - calls tx_validity(&mempool.accumulating, tx_cbor) — the Tier-1
       verdict. Re-validation is ALWAYS against the CURRENT accumulating
       state, never a stale snapshot, so a dependent tx (B spending A's
       output) validates once A is admitted.
     - Valid -> append tx_id to `accepted`, replace `accumulating` with
       the applied state; Admitted.
     - Invalid -> mempool returned UNCHANGED; Rejected with the same
       coarse class + structured reason tx_validity produced. NO FALSE
       ACCEPT (DC-MEM-01).
  2. ade_ledger::mempool::policy::order(mempool, OrderPolicy) -> Vec<Hash32>
     (Tier-5, GREEN behavior — a deterministic PERMUTATION over the
      already-admitted tx ids. Reads ONLY the accepted-id list; never
      calls tx_validity, never touches accumulating state, cannot change
      any admit verdict — DC-MEM-02.)
Cross-surface state sharing: the mempool's `accumulating` LedgerState is
  the only state carried across consecutive `admit` calls; it is the
  same shape `tx_validity` consumes, threaded by value.
```

**Rule.** Admission is a **thin Tier-1 gate over `tx_validity`** — its
verdict equals `tx_validity`'s verdict exactly (DC-MEM-01). The
Tier-1 / Tier-5 split is the key seam: `admit` (Tier-1, BLUE) owns the
validity decision; `policy` (Tier-5, GREEN behavior) may only reorder or
trim what Tier-1 already admitted, and is provably below it because
`order` consumes only the admitted-id list (DC-MEM-02). **No mempool
policy — eviction, prioritization, fee sorting, congestion shedding —
may move into the validity decision.** Every future mempool feature
attaches as Tier-5 below `admit`; anything that would change which txs
are valid is a Tier-1 change to `tx_validity` (and therefore to the
ledger authority it composes), not a policy knob. **B3 note:** because
admission inherits its verdict from `tx_validity`, B3's full
value-conservation tightening flows through `admit` automatically — a tx
that fails deposit/refund/withdrawal conservation is now correctly
rejected at the gate, with no change to `admit` itself.

### Surface: Full block validity (composition root — wired in B1)

```
Surface: A full block (era-tagged envelope CBOR) decided against
         (LedgerState, PraosChainDepState, EraSchedule, LedgerView)
Reduces to: BlockValidityVerdict { Valid { tip, block_no, body } |
                                    Invalid { class, error } }
            (defined in `ade_ledger::block_validity::verdict`)
Pipeline (fixed step ordering — no reorder, no shortcut):
  1. ade_ledger::block_validity::decode_block(block_cbor) -> DecodedBlock
     (header_input projection + block hash + recomputed body hash +
      inner-block byte range; era-dispatched; Babbage/Conway only today)
  2. ade_core::consensus::validate_and_apply_header(
         chain_dep, &header_input, ledger_view, era_schedule)
     (BLUE header authority — FAIL-FAST; the body authority is NOT
      reached if this fails — DC-VAL-03)
  3. body-hash binding: computed_body_hash == applied.summary.body_hash
     (cheap pre-flight before body application — CN-CONS-04; an altered
      body is rejected here as BodyHashMismatch)
  4. ade_ledger::rules::apply_block_with_verdicts(ledger, era, inner)
     (BLUE body authority — consumes the INNER block, env tag stripped;
      B3: the Conway per-tx state-backed path now runs the full
      value-conservation equation, the SAME authority tx_validity shares)
  5. Valid -> evolved (LedgerState', PraosChainDepState'); Invalid ->
     input states returned UNCHANGED (no partial mutation — DC-VAL-05)
Cross-surface state sharing: none — `block_validity` is a pure total
  function fn(&LedgerState, &PraosChainDepState, &EraSchedule,
  &dyn LedgerView, &[u8]) -> BlockValidityOutcome. Both states are
  threaded by value through the outcome; nothing ambient.
```

**Rule.** `block_validity` is the **single block-level composition root**
that joins the consensus header authority and the ledger body authority.
A block is `Valid` **iff** both `validate_and_apply_header` **and**
`apply_block_with_verdicts` accept it (DC-VAL-02). The ordering is
normative: header is decided first, body never runs on a header-invalid
block (DC-VAL-03). The body-hash binding sits **between** the two
authorities (DC-VAL-02/CN-CONS-04). **No path may produce a `Valid`
verdict while skipping either authority** — the follow-bridge's RED
peer-trusted "trust the body / skip header" shortcut must not leak into
this BLUE verdict. `block_validity` introduces **no new validation
rules** (DC-VAL-02). **Relationship to `tx_validity` (B2/B3):** the block
body authority `apply_block_with_verdicts` validates *all* of a block's
txs in their per-block context; `tx_validity` validates a *single* tx
against a standalone `LedgerState`. They **converge on the same per-tx
authorities** (the witness closure and `validate_conway_state_backed`,
now incl. the B3 full value-conservation equation) — see §2 — but neither
composer subsumes the other: `block_validity` composes header ∧ body,
`tx_validity` composes phase-1 ∧ phase-2. **Remaining adjacent gap:** the
Conway block-body loop in `rules.rs` still reuses the Shelley-era
applicator and does not re-run the per-tx vkey-witness closure that
`tx_validity` provides (`project_conway_body_witness_gap`); wiring
`tx_phase_one` / `verify_required_witnesses` into the Conway block-body
path is the natural remaining closure and a post-B3 item.

### Surface: Block bytes (wired today)

```
Surface: Block bytes (file/stream/network — caller-supplied)
Reduces to: BlockEnvelope { era: CardanoEra, era_block: PreservedCbor<EraBlock> }
            (BlockEnvelope is defined in `ade_codec::cbor::envelope`;
             EraBlock is one of the seven era-tagged decoded blocks)
Pipeline (fixed step ordering — no reorder, no shortcut):
  1. decode_block_envelope(&[u8]) -> BlockEnvelope
     (era tag dispatch; the only constructor of PreservedCbor for blocks)
  2. era-specific decode_{byron_ebb,byron_regular,shelley,allegra,
     mary,alonzo,babbage,conway}_block
     (closed set — 8 era-block decoders, named in
     `ci_check_ingress_chokepoints.sh`)
  3. ade_ledger::rules::apply_block_with_verdicts(state, &PreservedCbor<EraBlock>, ctx)
     (BLUE — single canonical chokepoint that produces BlockVerdict + new state)
Cross-surface state sharing: none today (Phase 3 was an offline oracle).
  Phase 4 introduces shared state between this surface and the network
  ingress surface (mux frames, below) via `ade_runtime::chaindb`
  (persistence) and a forthcoming `ade_node`-level composition layer.
```

**Rule.** New ingress that produces block bytes (e.g., the N-A `block-fetch`
mini-protocol delivering block bodies, N-D recovery replay, N-F
`local-tx-monitor`) **MUST** enter through `decode_block_envelope` and
flow through one of the era-specific block decoders before reaching any
ledger code. The pipeline cannot be reordered: hash-bearing bytes must
be preserved via `PreservedCbor` before they reach ledger rules
(enforced by `ci_check_hash_uses_wire_bytes.sh`,
`ci_check_ingress_chokepoints.sh`). **`ade_network` is forbidden from
decoding block CBOR** — its codec layer treats block / header / tx
bodies as opaque `Vec<u8>`, and dispatch into `ade_codec` happens at
the session / `ade_node` boundary. The B1 composition root reuses this
same chokepoint: `decode_block` calls `decode_block_envelope` plus the
per-era block decoder; it does not invent a parallel decode path.
**Note (B2/B3):** `tx_validity::decode_tx` decodes a *standalone* Conway
tx CBOR via the `ade_codec` primitive set + `decode_conway_tx_body` — it
does **not** go through `decode_block_envelope` (a bare tx is not a block
envelope), and it never constructs `PreservedCbor` itself. B3's
`decode_conway_certs` / `decode_withdrawals` are sub-grammar readers
*inside* the Conway tx body (keys 4 and 5); they consume already-lifted
body byte slices via the `ade_codec` primitive set and likewise never
construct `PreservedCbor`. **B3F hardened both readers to the same
exact-CBOR-item posture:** `decode_conway_certs` now rejects trailing
bytes with `CodecError::TrailingBytes` (parity with `decode_withdrawals`)
and bounds its preallocation (`with_capacity(n.min(remaining_len))`) so a
hostile length prefix cannot force a large allocation (DC-VAL-06).
**B4 made `decode_conway_certs` owner-complete** — it retains every owner
payload (credentials, pool id, full `PoolRegistrationCert` incl.
`pool_owners`, DRep targets) via the closed `decode_drep` and the single
shared `read_pool_registration_cert` (the ONE pool-params decode site for
both eras) — but it remains a sub-grammar reader inside the Conway tx body:
it still consumes already-lifted body byte slices via the `ade_codec`
primitive set, still constructs no `PreservedCbor`, still has no catch-all
accept arm (tags ≥19 → `UnknownCertTag`, tags 5/6 → `RemovedInConway`), and
is still not a new ingress surface.

### Surface: Plutus script bytes (wired today)

```
Surface: Plutus script bytes (CBOR-wrapped Flat, extracted from witness sets)
Reduces to: PlutusScript { inner: aiken_uplc::ast::Program<DeBruijn> }
            (defined in `ade_plutus::evaluator`; aiken types do not
             leak past this boundary)
Pipeline:
  1. ade_plutus::evaluator::PlutusScript::from_cbor(&[u8]) -> Result<PlutusScript, PlutusError>
     (named ingress chokepoint — the only public path that turns Plutus
     script CBOR into a runnable program; uses the aiken/pallas decoder,
     not the ade_codec primitives)
  2. ade_plutus::tx_eval::eval_tx_phase_two(...) -> TxEvalResult
     (BLUE — single canonical phase-2 evaluation entry; aiken `uplc`
     machine is invoked internally and aiken types do not escape)
Cross-surface state sharing: none — phase-2 evaluation is pure
  fn(script, ScriptContext, CostModels, ExUnits) -> EvalOutput.
```

**Rule.** Plutus script CBOR is a **distinct ingress surface** from
block CBOR. It does not go through `decode_block_envelope` because its
wire format is CBOR-wrapped Flat decoded by `aiken_uplc`, not by the
project's own `ade_codec` primitives. The chokepoint is
`PlutusScript::from_cbor` in `ade_plutus/src/evaluator.rs`, named
explicitly in the header comment of `ci_check_ingress_chokepoints.sh`
and allowlisted from Check 3 of that script (Check 3 forbids
`from_cbor`/`minicbor::decode`/`cbor_decode` everywhere in BLUE except
in `ade_plutus/src/evaluator.rs`). All other BLUE crates remain
forbidden from decoding raw CBOR. **B2 note:** `tx_validity`'s phase-2
step reaches phase-2 via `plutus_eval::try_evaluate_tx`, which feeds
`eval_tx_phase_two` — it does not bypass the chokepoint.

### Surface: Snapshot bytes (wired in N-D)

```
Surface: Snapshot bytes (disk — written and read by the node itself)
Reduces to: Recoverable::decode_snapshot(&[u8]) -> R  (caller-supplied)
Pipeline:
  1. SnapshotStore::latest_snapshot() -> Option<(SlotNo, Vec<u8>)>
  2. Recoverable::decode_snapshot(bytes) -> R       (caller's impl)
  3. for block in ChainDb::iter_from_slot(slot+1):
       R::apply_block(&block.bytes) -> R            (caller's impl)
Cross-surface state sharing: `ade_runtime` is intentionally bytes-in /
  bytes-out — it never touches the ledger state type directly. The
  shared state lives at the caller (eventually `ade_node`).
```

**Rule.** The recovery primitive (`ade_runtime::recovery::recover`) is
the **single** path from on-disk state to in-memory state. It does not
import `ade_ledger`. Any callsite that wants to recover a ledger state
must provide a `Recoverable` impl; there is no second public path
through `ade_runtime`. **B3 note:** the RED snapshot loader in
`ade_testkit` is the **one allowlisted non-canonical source** that
materializes `ConwayOnlyDepositParams` (parsing `drep_deposit` /
`gov_action_deposit` from snapshot bytes into
`LedgerState.conway_deposit_params`); `ci_check_deposit_param_authority.sh`
allowlists exactly this loader and forbids every BLUE crate from sourcing
a deposit amount any other way (DC-TXV-07).

### Surface: Consensus-input extraction (snapshot `state` CBOR tail-scan — wired in B1)

```
Surface: A UTxO-HD `utxohd-mem` ExtLedgerState snapshot `state` CBOR
         (external dump format — NOT an authoritative canonical type)
Reduces to: PraosNonces { evolving, candidate, epoch, lab,
                          last_epoch_block }   (5 Nonce([u8;32]) in
            record order — the third, `epoch`, is eta0)
            (defined in `ade_ledger::consensus_input_extract`)
Pipeline (fixed step ordering — no reorder, no shortcut):
  1. ade_ledger::consensus_input_extract::extract_praos_nonces(&[u8])
       -> Result<PraosNonces, NonceScanError>
     — a pure tail-scan for the 4-byte non-neutral nonce prefix
       (`82 01 5820`) followed by a 32-byte body. Fail-CLOSED: the
       captured snapshots always carry EXACTLY five contiguous nonce
       wrappers; anything other than five is a hard `NotFiveNonces`
       error, never a best-effort pick.
Cross-surface state sharing: none — the scan is pure over the input
  bytes; the extracted nonces seed a `PraosChainDepState` at the caller.
```

**Rule.** This is the provenance surface for the consensus nonces that
seed `PraosChainDepState`. It is **classified RED behavior** (it parses
an external dump format rather than an authoritative canonical type) but
the function is pure over its bytes, lives in `ade_ledger`, and respects
every BLUE forbidden pattern (no I/O, no clock, no HashMap, fail-closed).
It is the **only** sanctioned way to lift Praos nonces out of a captured
snapshot; it never re-derives them and never picks heuristically. The
exact-five requirement is a closure invariant: a future capture format
that carries a different nonce count is a version-gated change, not a
silent relaxation. **Candidate flag:** the module's own doc-comment
calls itself "RED" while it physically lives inside a BLUE crate
(`ade_ledger`); the cluster doc's TCB Color Map lists it as "RED (in
`ade_ledger` or testkit tool)." This dual placement is intentional
(pure-over-bytes, no ambient nondeterminism) but should be confirmed —
if a future capture introduces real I/O, the loader half must move to
`ade_runtime`/testkit and only the pure scan stays here.

### Surface: Ouroboros mux frames (wired in N-A)

```
Surface: Raw bytes off a TCP / Unix-socket bearer (cardano-node peer)
Reduces to: per-protocol message enums in `ade_network::codec::*`
            (BlockFetchMessage, ChainSyncMessage, HandshakeMessage,
             KeepAliveMessage, PeerSharingMessage, TxSubmission2Message,
             LocalChainSyncMessage, LocalStateQueryMessage,
             LocalTxMonitorMessage, LocalTxSubmissionMessage,
             N2cHandshakeMessage — 11 closed enums, one per protocol)
Pipeline (fixed step ordering — no reorder, no shortcut):
  1. ade_network::mux::transport::MuxTransport::read_raw (RED, async)
     — moves bytes off the bearer; no parsing.
  2. ade_network::mux::frame::decode_frame(&[u8])
       -> Result<(MuxFrame, &[u8]), MuxError>     (BLUE, sync, pure)
     — the **single** chokepoint that turns bytes into a typed
       (timestamp, mode, mini_protocol_id, payload) frame. Mirror
       symbol `encode_frame` is the **single** outbound chokepoint.
  3. ade_network::codec::<protocol>::decode_<protocol>_message(payload)
       -> Result<<Protocol>Message, CodecError>   (BLUE, sync, pure)
     — closed wire grammar per protocol; one decoder per closed enum
       above. Mirror symbol `encode_<protocol>_message` for outbound.
  4. ade_network::<protocol>::transition::<protocol>_transition(
         state, agency, version, msg)
       -> Result<(new_state, output), error>     (BLUE, sync, pure)
     — 8 closed transition functions (chain-sync, block-fetch,
       handshake [n2n + n2c arms share the module], keep-alive,
       peer-sharing, tx-submission2, plus 4 N2C state machines under
       `ade_network::n2c::local_*`). Selected protocol version is an
       explicit input (DC-PROTO-06); never read from a session global.
  5. Session composition (RED, S-A9 placeholder at HEAD;
     `ade_network::session::mod`) routes outputs into shell I/O and
     fans block / tx / query bytes to the appropriate authoritative
     pipeline above. **No N-A code calls `ade_ledger` or `ade_codec`
     block decoders directly** — that bridge lands in the future
     `ade_node` composition layer.
Cross-surface state sharing: protocol version table
  (`ade_network::codec::version`) is shared across handshake +
  transition + codec call sites. No other shared state.
```

**Rule.** Mux frames are a distinct ingress surface, layered above the
byte bearer and below all higher protocol decoding. The two chokepoints
`mux::frame::{encode_frame, decode_frame}` are the only byte↔frame
translation in the project; `ade_network::mux::transport` (RED) calls
them and nothing else does. **Each mini-protocol's codec and transition
function form a self-contained, structurally independent closed
semantic surface (IDD §6).** Adding a new mini-protocol is *not* an
extension of an existing one — it is a new closed `*Message` enum + a
new `encode_*_message` / `decode_*_message` pair + a new `*_transition`
function + a new `*Version` enum in `ade_network::codec::version`.
There is no `Codec<P>` trait, no `Box<dyn Protocol>`, no
`#[non_exhaustive]`, no runtime negotiation of message meaning.
Versioning happens through closed `*Version` enums that gate which
variants are legal at protocol-step time; mismatches surface as
`InvalidForVersion` at the protocol boundary rather than as a silent
fallback. **B2 note:** the `tx-submission2` (N2N) and
`local-tx-submission` (N2C) protocols carry tx bytes as opaque
`Vec<u8>`; their delivered payloads are the **future ingress to
`mempool::admit`**. That bridge is a candidate seam (see the candidate
table) — at HEAD it is unwired: B2 explicitly scoped out tx-submission
wiring (cluster doc §15), so the mempool gate is reachable only by direct
caller / test invocation, not yet from the network.

### Surface: Genesis JSON bundles (wired in N-B)

```
Surface: Four genesis JSON blobs (byron + shelley + alonzo + conway)
Reduces to: EraSchedule { anchor: BootstrapAnchorHash, system_start_unix_ms, eras: [EraSummary; ≤7] }
            (defined in `ade_core::consensus::era_schedule`)
Pipeline (fixed step ordering — no reorder, no shortcut):
  1. caller assembles four byte slices into a `GenesisBundle`
     (`ade_runtime::consensus::genesis_parser::GenesisBundle` — closed
     struct with four named `&[u8]` fields; **not** an open bag of
     JSON files).
  2. ade_runtime::consensus::genesis_parser::compute_anchor_hash(&GenesisBundle)
       -> BootstrapAnchorHash                       (RED, pure)
     — Blake2b-256 over `b"ade_bootstrap_v1" || canonical_cbor([byron,
       shelley, alonzo, conway])`. **Domain-separation tag is frozen.**
  3. ade_runtime::consensus::genesis_parser::parse_genesis(&GenesisBundle, NetworkMagic)
       -> Result<EraSchedule, GenesisParseError>    (RED — uses serde_json)
     — the **single** RED → BLUE materialization chokepoint for the
       schedule. Returns a structured (no-`String`) error taxonomy:
       MalformedJson / MissingField / InvalidValue / UnknownNetwork /
       Hfc(HFCError). Internally validates `EraSchedule::new` (which
       in turn enforces monotonicity, non-empty era list, non-zero
       slot/epoch lengths).
  4. EraSchedule is then consumed BLUE **by-reference**; never
     mutated; never re-parsed. The `BootstrapAnchorHash` it carries
     binds the schedule to the parsed genesis bytes — any downstream
     consumer (header validate, leader schedule, rollback,
     block_validity) that needs to assert "same genesis" compares
     anchor hashes.
Cross-surface state sharing: none. The schedule is constructed once at
  startup and threaded into every BLUE consensus surface as an
  argument. No global registry.
```

**Rule.** Genesis JSON is a **distinct ingress surface**. Like
block CBOR, its decoder lives in a single named chokepoint and its
canonical reduction target (`EraSchedule`) is a BLUE type. Unlike block
CBOR, the decoder is RED (`genesis_parser` uses `serde_json` and
returns structured `GenesisParseError`) — but BLUE consensus never
re-parses, never reaches into JSON, and never re-derives the anchor
hash. The four-element domain-separated preimage layout is frozen at
v1; any future schema change to the anchor preimage is a hard
version-gated event because every downstream schedule check pivots on
`BootstrapAnchorHash`. `NetworkMagic` is a closed `enum`-shaped
newtype (MAINNET / PREPROD / PREVIEW); unknown magics produce a typed
`UnknownNetwork` reject, never a silent fallback.

### Surface: Chain-selector stream inputs (wired in N-B)

```
Surface: Ordered stream of N-A events (header arrival, rollback request, epoch boundary)
Reduces to: ade_runtime::consensus::chain_selector::StreamInput
            (closed 3-variant enum — `HeaderArrival(HeaderInput)`,
             `RollBack(RollBackRequest)`, `EpochBoundary { new_epoch,
             last_block_of_prev_epoch }`)
Pipeline (fixed step ordering — no reorder, no shortcut):
  1. caller wraps each external event in `StreamInput`.
  2. ade_runtime::consensus::chain_selector::process_stream_input(
         &mut OrchestratorState, &StreamInput,
         &dyn LedgerView, &EraSchedule)
       -> Result<Option<ChainEvent>, OrchestratorError>   (GREEN, sync, pure)
     — the **single** orchestrator chokepoint. Dispatches by variant:
       - HeaderArrival   -> validate_and_apply_header  (BLUE)
                         -> build_candidate_fragment   (GREEN materializer)
                         -> select_best_chain          (BLUE)
                         -> push_snapshot              (bounded ring; ≤ k)
       - RollBack        -> find snapshot by block_no
                         -> apply_rollback             (BLUE)
                         -> trim newer snapshots
       - EpochBoundary   -> apply_nonce_input          (BLUE)
  3. BLUE returns `ChainEvent` (closed 5-variant enum: ChainExtended,
     RolledBack, RolledForward, ChainSelected, Rejected) or a
     `ChainSelectionReject` carried inside `ChainEvent::Rejected`.
Cross-surface state sharing: `OrchestratorState` holds the
  authoritative `PraosChainDepState`, `ChainSelectorState`, and a
  bounded ring of `RollbackSnapshot { block_no, chain_dep, tiebreaker }`
  (default cap = DEFAULT_SNAPSHOT_LIMIT = 2160, the mainnet k). The
  ring is the only state shared across consecutive `StreamInput`s.
```

**Rule.** Stream inputs are the **header-only** ingress surface that
drives Praos chain selection. The reduction shape is deliberately small
(3 variants) so the orchestrator's responsibility is sequencing, not
policy. **Every external trigger that can advance Ade's chain state must
reduce to one of these three variants** — there is no "fast path" into
BLUE consensus. The orchestrator never reads a chain store, never calls
into `ade_codec`, and never invents its own state-shape decisions; BLUE
owns each transition's success/reject shape. `OrchestratorError` is
closed (HeaderInvalid / NonceEvolution) and only fires when the BLUE
pipeline returns an `Err`; structured rejects (TiebreakerLossKeepCurrent,
ExceededRollback, ForkBeforeImmutableTip, HeaderInvalid) surface inside
`ChainEvent::Rejected` so a single shape carries both new state and
the rejection record. **Relationship to `block_validity`:** the
orchestrator validates *headers* (cheap, fork-choice-relevant); the
composition root validates *full blocks* (header ∧ body). At HEAD these
are two distinct surfaces; the future `ade_node` layer wires them so a
header that wins fork-choice triggers a full `block_validity` decision
on the fetched body. That bridge is a candidate seam, not yet wired.

### Candidates — surfaces not yet wired (Phase 4 N-C, N-E, N-F, B+ residuals)

The following surfaces are named in the Phase 4 plan / B2 / B3 planning
but have no source today. They are listed so future slice docs can attach
without reinventing the reduction step. **Each is a candidate seam
pending confirmation at cluster entry.** **B3 closed the prior revision's
"deposit/refund preservation-of-value" candidate** — it landed exactly as
predicted (tightening of `validate_conway_state_backed`, no new composer)
and is removed from this table. **B4 added one CONFIRMED extension point**
(not a candidate): the owner-tagged `ConwayGovState` effect channel that the
declared PHASE4-B5 cluster consumes — it is recorded as a confirmed row
(first row) because the producing seam (`ConwayCertOutcome.owner_tagged`) is
already wired and the consuming cluster is already named in the registry
(DC-LEDGER-08) and the B4 cluster doc.

| Cluster | Surface | Expected reduction target | Expected chokepoint | Confidence |
|---------|---------|---------------------------|---------------------|------------|
| **PHASE4-B5** *(declared)* | **Owner-tagged Conway governance-cert effects → `ConwayGovState`** — the channel B4 produces (`ConwayCertOutcome.owner_tagged`, an `OwnerTaggedEffect` carrying a `GovernanceCertEffect` + `GovernanceOwner`) for vote-delegation / committee / DRep certs, routed OUT of B4's mutation scope and not yet applied | An applied `ConwayGovState'` (the gov-state mutation B4 deliberately does not perform) | A new BLUE governance-cert apply step in `ade_ledger` that consumes each `OwnerTaggedEffect` from `accumulate_tx_certs` and folds it into `ConwayGovState` — joining the existing `ade_ledger::governance::*` ratification/enactment authority; no new composer, no new ingress | **confirmed extension point** (producing seam wired in B4; consuming cluster named in DC-LEDGER-08 + the B4 cluster doc) |
| B+ / N-E | **N2N/N2C tx-submission ingest → mempool** — the RED ingress that delivers a candidate tx from the `tx-submission2` (N2N) or `local-tx-submission` (N2C) opaque-bytes payload into the Tier-1 gate | `mempool::admit(mempool, tx_cbor)` | A RED bridge (likely `ade_node` / `ade_runtime`) translating `TxSubmission2Message` / `LocalTxSubmissionMessage` delivered tx bytes into an `admit` call | candidate (B2 explicitly scoped this OUT — cluster doc §15) |
| B+ (full tx UTxO scope) | Full-scope single-tx validity over real resolved UTxO (today the positive corpus runs at `track_utxo=false`; value/fee/input-resolution + the B3 deposit/refund/withdrawal accounting run at `track_utxo=true`) | `TxValidityVerdict` at `track_utxo=true` over a real or synthetic UTxO | `tx_validity` (existing) — the gating already exists in `tx_phase_one`; this is corpus + state wiring, not a new chokepoint | candidate |
| B+ (Conway body witness depth) | **Conway block-body vkey-witness closure** — the `rules.rs` Conway block-body loop re-running the per-tx witness closure `tx_validity` provides (`project_conway_body_witness_gap`) | `BlockValidityVerdict` whose body authority runs the same closure as `tx_phase_one` | wire `tx_phase_one` / `verify_required_witnesses` into the Conway block-body path in `rules.rs` (no new composer) | candidate (B2-carried, still open after B3) |
| B+ (pre-Conway tx) | Pre-Conway single-tx validity (`tx_validity` is Conway-only today; `decode_tx` and `required_signers` return `UnsupportedEra` otherwise) | `TxValidityVerdict` via per-era body decode + per-era `SignerSource` enumeration | extend `decode_tx` + add the era arm to `required_signers` | candidate |
| B1+ (header→body bridge) | Forge/fetch bridge: a fork-choice-winning header triggers a full-block decision on the fetched body | `block_validity(...)` over the fetched body | `ade_node` composition layer joining `process_stream_input` and `block_validity` | candidate |
| B1+ (pre-Babbage block) | TPraos full-block validity (Shelley..Alonzo) | `BlockValidityVerdict` via a TPraos `HeaderInput` projection | extend `block_validity::decode_block` to build `HeaderVrf::Tpraos` headers (today it returns a typed reject for non-Babbage/Conway) | candidate |
| N-C | Forge-block inputs (mempool + state + slot + KES + VRF) | `BlockEnvelope` bytes (forged, then re-decoded for validation) | `ade_runtime::forge::forge_block` (proposed) | candidate |
| N-C | Operator block-production trigger | `StreamInput::HeaderArrival(HeaderInput)` (forged header is fed back into the same chain-selector entrypoint) | `process_stream_input` (existing) | candidate |
| N-F | LSQ semantic dispatch (LocalStateQuery payloads) | Internal Query enum (closed, not yet defined) | Single dispatch fn that consumes `LocalStateQueryMessage::Acquire/Query/Result` opaque-bytes payloads — Tier 5 wire on operator-facing gRPC/HTTP, Tier 1 semantics shared with LSQ | candidate |
| N-F | LocalTxMonitor semantic dispatch | Mempool-snapshot Query/Reply enums (over the `mempool::admit` accepted set) | Single dispatch fn that consumes `LocalTxMonitorMessage` opaque-bytes payloads | candidate |
| N-B+ | Live cardano-node session driver (for `ade_core_interop::live_consensus_session`) | `StreamInput` translated from `ade_network::chain_sync::ChainSyncMessage` and `block_fetch::BlockFetchMessage` events | Composition layer in `ade_core_interop` (currently a `ready` stub binary; the full driver is operator-side work per S-B10) | candidate |

These candidates need user confirmation when each cluster is opened:
"Is the canonical reduction target named above the right one? Does the
chokepoint name fit the project's emerging naming convention?" In
particular, the **N2N/N2C tx-submission → `mempool::admit` ingress** and
the **Conway block-body vkey-witness closure** are the two seams most
load-bearing for the bounty and should be confirmed first at the next
mempool/tx cluster entry.

---

## 2. Data-Only vs. Authoritative Layers

Ade has ten authoritative domains. For each, a single BLUE chokepoint
holds enforcement authority; tooling layers (when they exist) live in
GREEN (`ade_testkit`) or RED (`ade_runtime`, `ade_network::mux::transport`,
`ade_network::session`, `ade_core_interop`). **B3 added one domain — the
Conway value-conservation accounting — and added a closed cert/withdrawal
data-only layer (`ade_codec::conway::{cert, withdrawals}`) under the
existing codec authority. B4 added one more domain — the Conway
certificate-state accumulation — built on the SAME data-only cert grammar
(now owner-complete) plus a new owner-tagged apply layer in
`ade_ledger::delegation` and an era-dispatch layer in `ade_ledger::rules`.**

### Conway value-conservation accounting — the deposit/refund/withdrawal authority (NEW in B3)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Data-only — cert grammar** | `ade_codec::conway::cert::decode_conway_certs` | BLUE | Closed CDDL grammar over tags `0..18` → `Vec<ConwayCert>`. **No catch-all accept arm:** tags ≥19 reject with `CodecError::UnknownCertTag { tag, offset }`; tags 5/6 decode to `ConwayCert::RemovedInConway { tag }` (an explicit marker, never an accept). Only deposit/refund-relevant fields are retained; every other field is structurally consumed and dropped. **B3F:** trailing bytes after the cert array reject with `CodecError::TrailingBytes` and preallocation is bounded (DC-VAL-06). The closure (no `_ =>` accept arm, `UnknownCertTag` for ≥19) is now grep-gated by `ci_check_conway_cert_classification_closed.sh`. Asserts nothing about ledger semantics. |
| **Data-only — withdrawals grammar** | `ade_codec::conway::withdrawals::{decode_withdrawals, withdrawals_sum}` | BLUE | Closed map grammar (tx-body key 5) → `BTreeMap<RewardAccount, Coin>`. A repeated `RewardAccount` key rejects with `CodecError::DuplicateMapKey { offset }` — **never last-wins**; trailing bytes after the map reject. `withdrawals_sum` is exact `i128` over the deduplicated map. |
| **Closed cert domain types** | `ade_types::conway::cert::{ConwayCert, CertDisposition, DepositEffect, CoinSource}` | BLUE | The closed sum types the codec produces and the classifier consumes (see §3). `CertDisposition` = `Accountable(DepositEffect)` / `Neutral` / `NotValidInConway`; `DepositEffect` = `NewDeposit(CoinSource)` / `Refund(CoinSource)`; `CoinSource` = `ExplicitInCert(Coin)` / `DepositParam(Coin)` / `RegistrationState(Coin)`. Era-grammar reject (`NotValidInConway`) is deliberately NOT a `DepositEffect`. |
| **Canonical deposit-param surface** | `ade_ledger::pparams::{ConwayOnlyDepositParams, ConwayDepositParams}` + `ade_ledger::state::{conway_deposit_params, conway_deposit_view}` | BLUE | `ConwayDepositParams` is the single view combining `ProtocolParameters.{key_deposit, pool_deposit}` with the Conway-only `{drep_deposit, gov_action_deposit}`. `conway_deposit_view()` is `Some` iff Conway and fails fast with `ValidationEnvironmentError::MissingConwayDepositParams` otherwise. The **sole canonical authority** for every deposit/refund amount (DC-TXV-07). |
| **Closed cert classifier** | `ade_ledger::cert_classify::classify(&ConwayCert, &ConwayDepositParams, &CertState) -> Result<CertDisposition, UnsupportedStateDependentDepositAccounting>` | BLUE | A total, compiler-exhaustive map over `ConwayCert`. Explicit-deposit variants source `CoinSource::ExplicitInCert`; legacy-implicit deposits source `CoinSource::DepositParam` from the canonical view; refunds source `CoinSource::RegistrationState` from `CertState`. A refund/deposit that cannot be resolved from registration state returns the structured `UnsupportedStateDependentDepositAccounting` reject — **never a fabricated amount, never the `key_deposit` param** (which can drift from the amount recorded at registration), and never an accept (DC-TXV-06). |
| **Authoritative enforcement** | `ade_ledger::conway::check_conway_coin_conservation` (inside `validate_conway_state_backed`) | BLUE | Enforces the FULL preservation-of-value equation `consumed = Σ inputs + Σ withdrawals + refunded_deposits == produced = Σ outputs + fee + donation + new_deposits`, with the §9.1 reject precedence below. The B2 cert/withdrawal early-out (the known false-accept) is REMOVED (T-CONSERV-01 / CN-LEDGER-07 strengthened; DC-VAL-06 strengthened). |
| **Determinism fold** | `ade_ledger::fingerprint::fingerprint_pparams` | BLUE | Folds the Conway deposit params under `CONWAY_DEPOSIT_PARAMS_TAG` when present; byte-identical to the prior fingerprint for any non-Conway state (`conway_deposit_params == None` ⇒ unchanged bytes — DC-LEDGER-01). |
| **Allowlisted deposit-param loader** | `ade_testkit` snapshot loader | GREEN | The one allowlisted non-canonical-state source: parses `drep_deposit` / `gov_action_deposit` from snapshot bytes and writes `LedgerState.conway_deposit_params`. `ci_check_deposit_param_authority.sh` allowlists exactly this loader. |
| **Positive corpus harness** | `ade_testkit::tx_validity` (extended) | GREEN | The Conway-576 corpus conservation tests; replay byte-identical. |
| **Adversarial harness** | `ade_testkit` conservation adversarial corpus (CE-B3-6) | GREEN | Each value-conservation / cert / withdrawal mutation maps to its expected reject class — no false accept. |

**Rule.** This domain has **one data-only grammar layer** (the closed
cert + withdrawals decoders in `ade_codec`), **one closed classifier**
(`cert_classify`), **one canonical deposit-param authority**
(`pparams`/`state`), and **one enforcement chokepoint**
(`check_conway_coin_conservation` inside `validate_conway_state_backed` —
which is the SAME phase-1 state-backed authority `tx_validity` and the
block body path share). The **§9.1 reject precedence is frozen** and runs
in this exact order, lowest-numbered failure winning, no later check
masking an earlier one:

```
  1. decode failure (certs / withdrawals)  → CodecError → LedgerError::Decoding
  2. era-invalid cert (CertDisposition::NotValidInConway, tags 5/6)
                                            → LedgerError::EraInvalidCertificate
  3. missing validation environment         → ValidationEnvironmentError
     (handled upstream at view assembly: conway_deposit_view fails fast)
  4. unsupported state-dependent accounting  → UnsupportedStateDependentDeposit
     (classify reject — refund/deposit not resolvable from CertState)
  5. value not conserved (consumed != produced) → ConservationError
```

The era-validity sweep runs across **all** certs before any accounting
fold, so a removed tag is reported ahead of a state-dependent or value
reject regardless of cert ordering. **New work that tightens cert
accounting lands in `cert_classify` (a new `CoinSource` resolution arm)
or in the canonical deposit-param view; new work that tightens the
balance lands in `check_conway_coin_conservation`. The closed cert
grammar `decode_conway_certs` is the data-only chokepoint and never gains
a catch-all accept arm — a new Conway certificate tag is a new explicit
`ConwayCert` variant + decoder arm + classifier arm, version-gated.**
Every deposit/refund amount MUST flow from `conway_deposit_view()` (or
from `CoinSource::ExplicitInCert` / `RegistrationState`); a literal next
to a deposit field or a testkit `ConwayGovParams` read is a CI failure
(`ci_check_deposit_param_authority.sh`, DC-TXV-07).

### Conway certificate-state accumulation — the owner-tagged apply authority (NEW in B4)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Data-only — owner-complete cert grammar** | `ade_codec::conway::cert::decode_conway_certs` (+ `decode_drep`) | BLUE | The SAME closed CDDL grammar over tags `0..18`, now **owner-complete**: every variant retains its owner payloads (credentials, pool id, full `PoolRegistrationCert`, DRep target). `decode_drep` reads the closed `drep = [0,addr_keyhash // 1,script_hash // 2 // 3]` grammar with no catch-all. Still no catch-all accept arm; tags ≥19 → `UnknownCertTag`, tags 5/6 → `RemovedInConway`; trailing bytes / over-allocation rejected (DC-VAL-06). Asserts nothing about ledger semantics. |
| **Data-only — single shared pool-params decoder** | `ade_codec::shelley::cert::read_pool_registration_cert` | BLUE | The ONE pool_params decode site (era-stable Shelley..Conway, retaining `pool_owners`), called by **both** the Shelley and the Conway cert decoders. **No second Conway decoder** — a new parallel pool-params decoder is a forbidden anti-pattern (DC-LEDGER-08). |
| **Closed action classifier** | `ade_ledger::delegation::conway_cert_action(&ConwayCert) -> ConwayCertAction` | BLUE | A total, compiler-exhaustive map over `ConwayCert` (all 18 tags + both removed tags 5/6). There is **no `Neutral` action** — every defined Conway tag has an owner: a cert either mutates B4-owned `CertState`, is owner-tagged to `ConwayGovState`, or is a structured era-invalid reject. |
| **Owner-tagged apply model** | `ade_ledger::delegation::apply_conway_cert(state, cert, env) -> Result<ConwayCertOutcome, LedgerError>` + the closed types `ConwayCertAction`, `GovernanceOwner`, `GovernanceCertEffect`, `OwnerTaggedEffect`, `ConwayCertOutcome`, `ConwayCertEnv` | BLUE | Delegation/pool certs mutate B4-owned `CertState` (`apply_pool_registration` now populates `PoolParams.owners` from the enriched cert). Governance-affecting certs are **owner-tagged to `ConwayGovState`** — observed and returned in `ConwayCertOutcome.owner_tagged`, never neutralized and never applied here. Composite tags 10/12/13 carry BOTH a `CertState` mutation and an owner-tagged effect. Removed tags 5/6 reject with `LedgerError::EraInvalidCertificate`. Never reduces a `ConwayCert` into the 7-variant Shelley `Certificate`. |
| **Era-dispatch + fail-closed accumulation** | `ade_ledger::rules::accumulate_tx_certs` (inside `process_block_certificates`, reached from `apply_block_with_verdicts` at `track_utxo`) | BLUE | Version-gates cert-state accumulation by `CardanoEra`: Conway → `decode_conway_certs` + `apply_conway_cert`; Shelley..Babbage → `decode_certificates` + `apply_cert`. **Fail-closed:** the prior `_era` discard and BOTH "non-fatal during replay" swallows are removed — a decode or apply error propagates as a structured `LedgerError` and halts the block transition. Conway bytes must dispatch to the Conway decoder, never the Shelley 6-variant decoder. |
| **B3 deposit projection (carried)** | `ade_ledger::cert_classify::classify` | BLUE | Updated to consume the enriched `ConwayCert`; dispositions are byte-identical to B3 (no accounting change). |
| **Positive corpus harness** | `ade_testkit` B4-S5 cert-state corpus (`positive_synthetic_cert_state_accumulates`) | GREEN | Synthetic positive cert-state accumulation; replay byte-identical (`cert_state_replay_byte_identical`). |
| **Adversarial harness** | `ade_testkit` B4 adversarial corpus (`adversarial_no_false_accept`) | GREEN | Each cert-state mutation maps to its expected reject / fail-closed dispatch outcome — no false accept; the four `conway_*_is_fail_closed` dispatch tests + `conway_governance_cert_routed_out_of_scope`. |

**Rule.** This domain has **one data-only grammar layer** (the
owner-complete `decode_conway_certs` + the single shared
`read_pool_registration_cert` + `decode_drep`, all in `ade_codec`), **one
closed action classifier** (`conway_cert_action`), **one owner-tagged apply
model** (`apply_conway_cert`), and **one era-dispatched enforcement
chokepoint** (`accumulate_tx_certs` inside `process_block_certificates` —
reached from `apply_block_with_verdicts` at `track_utxo`). **THE KEY SEAM is
the owner-tagging boundary:** B4 owns the delegation/pool `CertState`
mutation; governance-affecting certs (vote-delegation, committee, DRep) are
decoded fully, classified by `conway_cert_action`, and **routed out of B4's
mutation scope as an `OwnerTaggedEffect`** in `ConwayCertOutcome.owner_tagged`
— never neutralized, never applied here, never swallowed. **This is the
explicit, confirmed extension point where PHASE4-B5 (Conway governance
certificate accumulation authority) attaches:** B5 consumes these
owner-tagged effects and folds them into `ConwayGovState` (joining the
existing `ade_ledger::governance::*` ratification/enactment authority). New
work that adds a Conway cert tag adds an explicit `ConwayCert` variant + a
`decode_conway_certs` arm + a `conway_cert_action` arm + an `apply_conway_cert`
arm, version-gated — and, because both the classifier and the apply are
compiler-exhaustive `match`es over `ConwayCert`, a new variant breaks the
build rather than being silently neutralized or dropped (DC-LEDGER-08). **The
"no new parallel decoder" rule is load-bearing:** `read_pool_registration_cert`
is the one pool-params decode site for both eras; a second era-specific copy
is forbidden. Closure is mechanical via the compiler-exhaustive `match` plus
the named B4 tests; the `ci_script` is the existing full-BLUE
`ci_check_forbidden_patterns.sh` (no new gate). **Remaining open obligation
(environment-blocked, not a code gap):** the real epoch-576
cert-state-vs-cardano-node oracle is blocked by an absent epoch-576 UMap
snapshot; B4 closes mechanically with owner-complete decode + total
owner-tagged apply + era-dispatched fail-closed accumulation + the synthetic
positive/replay/adversarial corpus.

### Single-tx validity — the per-tx composition root (B2)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Decode / projection** | `ade_ledger::tx_validity::phase1::decode_tx` | BLUE | Lifts the PRESERVED body slice (`tx_id = blake2b_256(body_slice)`), the witness-set slice, the typed `ConwayTxBody`, the raw vkey witnesses, and the script-presence `WitnessInfo`. Conway-only. Builds inputs; asserts nothing. |
| **Required-signer enumeration** | `ade_ledger::tx_validity::required_signers::{required_signers, tx_derived_required_signers}` | BLUE | The closed, era-versioned `SignerSource` enumeration (DC-TXV-05). Derives every `Hash28` a tx must have a vkey witness for, partitioned by source. `tx_derived_*` is the UTxO-free strict subset (explicit/withdrawal/cert/voter); the full function adds input/collateral payment-key sources when the UTxO is available. |
| **Witness closure** | `ade_ledger::tx_validity::witness::verify_required_witnesses` | BLUE | Fail-closed coverage: every required key hash must be covered by a witness whose Ed25519 signature over the PRESERVED body hash verifies (DC-VAL-06 / CN-LEDGER-09). Wrong-size key/sig → `MalformedWitnessField`; an extra irrelevant witness never substitutes. |
| **Shared per-tx phase-1** | `ade_ledger::tx_validity::phase1::tx_phase_one` | BLUE | The single per-tx phase-1 authority. Composes the witness closure (run UNCONDITIONALLY) + `crate::conway::validate_conway_state_backed` (the SAME state-backed authority the block loop runs, gated on `track_utxo`; B3 generalized it to the full value-conservation equation). Introduces no new composer. |
| **Phase-2 dispatch** | `crate::plutus_eval::try_evaluate_tx` → `ade_plutus::tx_eval::eval_tx_phase_two` | BLUE | Plutus phase-2, reached only when the tx carries Plutus scripts. The aiken `String`-bearing failure is mapped into the closed `TxValidityError::Phase2`. |
| **Composition transition** | `ade_ledger::tx_validity::transition::tx_validity` | BLUE | The single chokepoint joining phase-1 ∧ phase-2 + the UTxO evolution. `fn(&LedgerState, &[u8]) -> TxValidityOutcome`. |
| **Comparison surface** | `ade_ledger::tx_validity::encoding::{encode_tx_verdict_surface, decode_tx_verdict_surface}` | BLUE | Canonical CBOR for the **coarse** replay/oracle surface only (`TxVerdictSurface`: `Valid -> [0, tx_id]`, `Invalid -> [1, class]`). The full `TxValidityError` detail is debug-only and NOT encoded. |
| **Positive replay harness** | `ade_testkit::tx_validity::{extract, …}` | GREEN | Extracts every on-wire Conway tx from the committed Conway-576 corpus blocks and drives BLUE `tx_validity` over each; asserts byte-identical verdict streams. |
| **Adversarial harness** | `ade_testkit::tx_validity::{adversarial, valid_synthetic}` | GREEN | Family A: witness mutations on real corpus txs at `track_utxo=false`. Family B: synthetic value/input/witness mutations at `track_utxo=true`. Each mutation must map to its expected reject class — no false accept. |

**Rule.** This domain has **two phase authorities and one composer**.
New work that tightens phase-1 lands in `tx_phase_one` (and the
authorities it composes — the witness closure and
`validate_conway_state_backed`); new work that tightens phase-2 lands in
the Plutus evaluator. **The composer `tx_validity` introduces no rules of
its own and never moves** (DC-TXV-02). The verdict comparison surface is
deliberately *coarse* (`TxRejectClass`: Phase1Invalid / WitnessInvalid /
MissingRequiredSigner / Phase2Invalid / MalformedField) so corpus
comparisons against the reference node are byte-stable; the rich
structured `TxValidityError` rides alongside for debugging but is **not**
part of the canonical bytes (the same "wire vs. semantic" rib B1 applied
to `block_validity`). **The `track_utxo` boundary is a first-class seam:**
the witness closure runs unconditionally; the UTxO-dependent state-backed
checks (now incl. the B3 full value-conservation equation) run only at
`track_utxo=true`. `track_utxo=false` is the strict PARTIAL mode
(structural + witness closure) — it must NOT be read as "full validity."
This mirrors the B1 block path exactly. **B3 closed the prior deferred
deposit/refund seam:** the deposit/refund/withdrawal value conservation
landed inside `validate_conway_state_backed` (see the new domain above),
exactly where the prior revision's candidate flag predicted — the
composer `tx_validity` was untouched. **Remaining extension points
(candidates):** full-scope `track_utxo=true` corpus over real resolved
UTxO, pre-Conway eras (attach at `decode_tx` + `required_signers`), and
the Conway block-body witness closure.

### Mempool admission — the Tier-1 / Tier-5 boundary (B2)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Tier-1 admission gate** | `ade_ledger::mempool::admit::admit` | BLUE | A tx is admitted iff `tx_validity(accumulating, tx)` is `Valid`. Threads the accumulating `LedgerState` (base + every admitted tx); re-validates against the CURRENT state. No false accept (DC-MEM-01). |
| **Mempool state** | `ade_ledger::mempool::admit::MempoolState` | BLUE | Closed: `accepted: Vec<Hash32>` (admission order) + `accumulating: LedgerState`. The only state carried across `admit` calls. |
| **Tier-5 ordering policy** | `ade_ledger::mempool::policy::order` | GREEN behavior | A deterministic PERMUTATION over the admitted-id list (`ArrivalOrder` / `TxIdAscending`). Reads only `accepted()`; never `tx_validity`, never `accumulating`. Cannot change a verdict (DC-MEM-02). |

**Rule.** The Tier-1 / Tier-5 split is the load-bearing seam. **`admit`
owns the validity decision and is provably equal to `tx_validity`'s
verdict** (DC-MEM-01). **`policy` is provably below it** — `order` reads
only the admitted-id list, so no choice of policy can alter which txs
`admit` accepts (DC-MEM-02). Every future mempool feature (eviction,
fee prioritization, congestion shedding, size caps) attaches as Tier-5
*below* `admit`; anything that would change validity is a Tier-1 change
to `tx_validity`, not a policy knob. **No mempool policy may call
`tx_validity` or touch the accumulating state.** Both rules are
mechanically enforced by `ci_check_consensus_closed_enums.sh` (target set
extended to `ade_ledger::mempool`), which keeps `AdmitOutcome`,
`OrderPolicy`, and the verdict family closed (no `String`, no `Box<dyn>`,
no `#[non_exhaustive]`). **B3 note:** the B3 full value-conservation
tightening flows through `admit` automatically — `admit` inherits its
verdict from `tx_validity`, so no `admit`/`policy` change was needed.

### Full block validity — the block-level composition root (B1)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Decode / projection** | `ade_ledger::block_validity::header_input::decode_block` | BLUE | Era-dispatched: reuses `decode_block_envelope` + the per-era block decoder, projects a `HeaderInput` (Praos for Babbage/Conway), recomputes the era-correct (segwit) body hash over preserved wire bytes, and records the inner-block byte range. Builds inputs; asserts nothing. |
| **Consensus header authority** | `ade_core::consensus::validate_and_apply_header` | BLUE | The header half. Decided first, fail-fast. |
| **Ledger body authority** | `ade_ledger::rules::apply_block_with_verdicts` | BLUE | The body half. Consumes the inner block, never reached on header failure. Runs `verify_conway_witness_closure` (unconditional) + `run_phase_one_composers` (track_utxo-gated, now incl. the B3 full value-conservation equation) — the SAME per-tx authorities `tx_validity` converges on. |
| **Composition transition** | `ade_ledger::block_validity::transition::block_validity` | BLUE | The single chokepoint joining the two authorities + the body-hash binding. `fn(&LedgerState, &PraosChainDepState, &EraSchedule, &dyn LedgerView, &[u8]) -> BlockValidityOutcome`. |
| **Comparison surface** | `ade_ledger::block_validity::encoding::{encode_verdict_surface, decode_verdict_surface}` | BLUE | Canonical CBOR for the **coarse** replay/oracle surface only (`VerdictSurface`). The full `LedgerError`/`HeaderValidationError` detail is debug-only and NOT encoded. |
| **Positive replay harness** | `ade_testkit::validity::replay` | GREEN | Drives `block_validity` over the Conway-576 positive corpus; asserts byte-identical verdict streams. |
| **Adversarial harness** | `ade_testkit::validity::adversarial` | GREEN | Deterministic block mutators (M1–M6) derive adversarial blocks from the real corpus; asserts each maps to its expected reject class. |

**Rule.** This domain has **two sub-authorities and one composer**. New
work that tightens the header half lands in `ade_core::consensus`; new
work that tightens the body half lands in `ade_ledger::rules` and the
per-era composers. **The composer `block_validity` introduces no rules
of its own and never moves** (DC-VAL-02). The verdict comparison surface
is deliberately *coarse* (`BlockRejectClass`: HeaderInvalid / BodyInvalid
/ BodyHashMismatch / MalformedField / MissingConsensusInput) so corpus
comparisons against the reference node are byte-stable. **B2 sharpened
the body authority** (it shares `validate_conway_state_backed` with
`tx_validity`); **B3 sharpened it again** — the shared state-backed
authority now runs the full deposit/refund/withdrawal value-conservation
equation. **Known extension points:** the Conway block-body vkey-witness
closure (`project_conway_body_witness_gap` — the body loop still reuses
the Shelley applicator and does not re-run the per-tx witness closure;
candidate seam in §1), and pre-Babbage TPraos full blocks (extend
`decode_block`).

### Ledger application

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Data-only tooling** | `ade_codec` (incl. B3 `conway::{cert, withdrawals}`) | BLUE\* | Decodes block / tx / cert / withdrawal bytes into typed values, preserves wire bytes via `PreservedCbor`. **Never interprets ledger semantics.** B3's closed cert/withdrawal grammars are data-only — they reject malformed/unknown/duplicate input but assert nothing about value conservation. |
| **Authoritative enforcement** | `ade_ledger` | BLUE | `rules::apply_block_with_verdicts` is the single chokepoint that produces `BlockVerdict` + new `LedgerState`; `tx_validity` is the single-tx chokepoint (B2); `check_conway_coin_conservation` is the value-conservation authority (B3); `accumulate_tx_certs` + `delegation::apply_conway_cert` is the era-dispatched cert-state accumulation authority (B4), reached inside `apply_block_with_verdicts` at `track_utxo`. |
| **Loader** | `ade_runtime::chaindb` + `ade_runtime::recovery` | RED | Reads block / snapshot bytes from disk; feeds them through caller-supplied `Recoverable` impl into ledger. |

\* `ade_codec` is BLUE-data-only: it builds typed shapes but never
asserts a transition is valid. The semantic split between "this is
what the bytes say" (codec) and "this is whether the bytes are
allowed" (ledger) is the project's central design rib. B3 is a textbook
instance: `decode_conway_certs` says "these are the certs and they are
structurally well-formed (no unknown/removed tag silently accepted)";
`cert_classify` + `check_conway_coin_conservation` say "this is whether
the deposits/refunds balance."

**Rule.** New work that touches ledger transitions adds enforcement
inside `ade_ledger` (typically a new composer step, or a tightening of
`apply_block_with_verdicts` / `apply_epoch_boundary_full` / the per-tx
`tx_phase_one` / `validate_conway_state_backed`). New work that touches
block / tx / cert / withdrawal CBOR adds parse / pack support inside
`ade_codec` only. **The compilation chokepoints
(`apply_block_with_verdicts` for blocks, `tx_validity` for single txs)
never move.**

### Stake-snapshot projection for consensus (B1)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Trait boundary** | `ade_core::consensus::ledger_view::LedgerView` | BLUE | The closed 4-method surface BLUE consensus consults for stake snapshots. `pool_vrf_keyhash(epoch, pool) -> Option<Hash32>` (the ledger holds the keyhash; the vkey arrives in the header; header validation binds the two). |
| **Production projection** | `ade_ledger::consensus_view::PoolDistrView` | BLUE | The leadership-relevant projection of a `LedgerState`'s pool-distribution. Single-epoch; `BTreeMap` only; no I/O; no rederivation. The first **production** `LedgerView` impl. |
| **Test stub** | `ade_testkit::consensus::ledger_view_stub::LedgerViewStub` | GREEN | The pre-B1 stub; still used by N-B integration tests. |

**Rule.** `LedgerView` remains a **closed trait, not a plugin point**.
The trait is expected to have a small, fixed set of impls (production +
test), never an open registry. **This is the surface where a future
LedgerState-backed `PoolDistrView` constructor attaches** — at HEAD
`PoolDistrView::new` is fed already-frozen B1 corpus data; a B4-style
sync slice will build it directly from a parsed `LedgerState` while
keeping the exact same trait shape. RED shells must not call BLUE
consensus with a hand-rolled `LedgerView` that bypasses ledger semantics.

### Plutus phase-2 evaluation

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Data-only tooling** | `ade_plutus::cost_model`, `ade_plutus::script_context` | BLUE | Decodes cost-model CBOR; builds the V1/V2/V3 `ScriptContext`. Does not run programs. |
| **Script ingress** | `ade_plutus::evaluator::PlutusScript::from_cbor` | BLUE | Named ingress chokepoint for Plutus script CBOR. Allowlisted in `ci_check_ingress_chokepoints.sh` Check 3 because the decoder is `aiken_uplc`/`pallas`, not `ade_codec`. |
| **Authoritative enforcement** | `ade_plutus::tx_eval::eval_tx_phase_two` | BLUE | Single entry to phase-two evaluation. Internally wraps the aiken `uplc` machine; aiken types do not leak (enforced by `ci_check_pallas_quarantine.sh`). Reached from `tx_validity` via `plutus_eval::try_evaluate_tx` (B2). |
| **Quarantine** | (the `aiken_uplc` git dep, pinned tag `v1.1.21` commit `42babe5d`) | external | Frozen at tag — never re-exported. PV11 builtins gated off (S-29). |

**Rule.** Adding a new Plutus version, builtin, or cost-model entry
requires a registry diff (see §3) plus a pinned-version bump of
`aiken_uplc`; the chokepoint `eval_tx_phase_two` does not move. No
second public entry into the evaluator is allowed; tests and the new
`tx_validity` phase-2 step use the same entry as production callers.
**No new BLUE callsite of `PlutusScript::from_cbor` may be added outside
`ade_plutus` itself** — the chokepoint exists to keep aiken-decoded bytes
inside the quarantine.

### Governance ratification / enactment (Conway)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Data-only tooling** | `ade_types::conway` (governance types) | BLUE | Holds `GovAction`, `GovActionState`, `DRep`, `Anchor`, `VotingProcedures` shapes. |
| **Authoritative enforcement** | `ade_ledger::governance::{evaluate_ratification, enact_proposals, expire_proposals}` | BLUE | The three chokepoints that compute Conway ratification outcomes. |

**Rule.** A new governance action variant (CIP-1694 extension) adds a
variant to `GovAction` (§3 closed registry — version-gated) **and**
arms in all three chokepoints. The CI check
`ci_check_constitution_coverage.sh` enforces the invariant-registry ↔
code coverage for governance rules. **B2 note:** governance voters are
also a `SignerSource` (`GovernanceVoter`) in the required-signer
enumeration — adding a voter credential kind touches both this domain and
`required_signers`. **B3 note:** governance deposits (`gov_action_deposit`,
`drep_deposit`) now flow through the canonical `ConwayDepositParams` view
— a DRep registration/unregistration cert's deposit/refund is classified
by `cert_classify` and sourced from `conway_deposit_view()`, not from a
governance literal (DC-TXV-07). **B4 note:** the governance-affecting certs
themselves (vote-delegation, committee auth/resign, DRep
register/unregister/update) are now decoded owner-complete and **owner-tagged
to `ConwayGovState`** by `apply_conway_cert` (`ConwayCertOutcome.owner_tagged`)
— but B4 does NOT apply them into governance state. **PHASE4-B5 is the
declared cluster that consumes these owner-tagged effects and folds them into
this domain's `evaluate_ratification` / `enact_proposals` / `expire_proposals`
lifecycle** (DC-LEDGER-08; see §2 "Conway certificate-state accumulation" and
the confirmed-extension-point row in §1).

### Mini-protocol wire conformance (N-A)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Data-only tooling (frame)** | `ade_network::mux::frame` | BLUE | Pure encode/decode over the fixed 8-byte Ouroboros mux header + opaque payload. No I/O, no async, no time. `encode_frame` / `decode_frame` are the only byte↔frame chokepoints. |
| **Data-only tooling (messages)** | `ade_network::codec::{block_fetch, chain_sync, handshake, keep_alive, local_chain_sync, local_state_query, local_tx_monitor, local_tx_submission, n2c_handshake, peer_sharing, tx_submission}` | BLUE | 11 closed wire grammars, one per mini-protocol. Each exposes `encode_<protocol>_message` + `decode_<protocol>_message`. Payloads of higher-layer surfaces (block CBOR, tx CBOR, LSQ queries, mempool queries) remain `Vec<u8>` here — interpretation lives elsewhere. |
| **Authoritative enforcement (state)** | `ade_network::{block_fetch, chain_sync, handshake, keep_alive, peer_sharing, tx_submission}::transition` and `ade_network::n2c::local_*::transition` | BLUE | 8 closed pure transition functions. Shape: `fn (state, agency, version, msg) -> Result<(new_state, output), error>`. Closed state graphs; illegal tuples produce `IllegalTransition`. |
| **Bearer (I/O)** | `ade_network::mux::transport` | RED | Tokio-based TCP / Unix-socket scaffold. Async lives **here and only here** within `ade_network`; sync-only discipline in BLUE submodules is enforced by `ci_check_no_async_in_blue.sh` (DC-CORE-01). |
| **Session composition (placeholder)** | `ade_network::session::mod` | RED | S-A9 placeholder. Will drive the mux + state machines together; no protocol logic. |
| **Live-interop capture tools** | `ade_network::bin::capture_*` (7 RED binaries) | RED | Operator/dev tools for live cardano-node 11.0.1 capture. Never linked into the node binary. |

**Rule.** Three rules carry the cluster:

1. **The codec layer is opaque to higher semantics.** `ade_network`
   never decodes block CBOR or tx CBOR — those payloads are `Vec<u8>`
   carried through `*Message` variants. The bridge into `ade_codec` /
   `ade_ledger` lives at the session/`ade_node` composition layer
   (currently a placeholder). The `tx-submission2` / `local-tx-submission`
   tx-bytes → `mempool::admit` bridge is a candidate seam (§1).
2. **The two chokepoints `mux::frame::{encode_frame, decode_frame}`
   never move.** Any future wire-framing change is a coordinated
   rewrite of both, not a duplicate path.
3. **The selected protocol version is an explicit transition input
   (DC-PROTO-06).** No state machine reads ambient session state.
   Mismatches surface as `InvalidForVersion`.

### Praos consensus runtime (N-B)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Data-only tooling (genesis)** | `ade_runtime::consensus::genesis_parser` | RED | `parse_genesis` + `compute_anchor_hash`. Reads JSON via `serde_json`, computes the v1 domain-separated anchor hash, produces a typed `EraSchedule`. Returns a closed `GenesisParseError` taxonomy (no `String`). |
| **Schedule authority** | `ade_core::consensus::era_schedule` | BLUE | `EraSchedule::new` validates monotonicity, non-empty era list, non-zero slot/epoch lengths; `locate`, `slot_to_time_ms`, `check_forecast_horizon` are pure integer arithmetic. `BootstrapAnchorHash` is carried verbatim and never recomputed in BLUE. |
| **Stake-snapshot boundary** | `ade_core::consensus::ledger_view::LedgerView` (trait, BLUE) ↔ `ade_ledger::consensus_view::PoolDistrView` (production BLUE impl) / `ade_testkit::consensus::ledger_view_stub::LedgerViewStub` (test GREEN impl) | mixed | BLUE consumes ledger-owned stake snapshots **by-reference only**; never owns, mutates, or re-derives them. See §2 "Stake-snapshot projection" above. |
| **Header admission** | `ade_core::consensus::header_validate::validate_and_apply_header` | BLUE | Single chokepoint. 10-step pipeline + B1 KES verification + era-correct VRF domain. Sequential and fail-fast; no partial state. |
| **Best-chain authority** | `ade_core::consensus::fork_choice::select_best_chain` | BLUE | Single chokepoint. Total ordering is `(BlockNo, TiebreakerView{slot, issuer_hash, op_cert_counter, leader_vrf_output_first_8})`. Chain-length-density ordering forbidden (enforced by `ci_check_no_density_in_fork_choice.sh`). |
| **Rollback authority** | `ade_core::consensus::rollback::apply_rollback` | BLUE | Single chokepoint. k-bound + immutable-tip refusal; rejects surface as `ChainEvent::Rejected`. |
| **Candidate materialization** | `ade_runtime::consensus::candidate_fragment::build_candidate_fragment` | GREEN | Builds the `CandidateFragment` consumed by `select_best_chain`. Non-authoritative. |
| **Orchestration** | `ade_runtime::consensus::chain_selector::process_stream_input` | GREEN | Threads `StreamInput` through the BLUE pipeline; owns the bounded rollback-snapshot ring; never makes a comparison decision itself. |
| **Live-interop driver (scaffold)** | `ade_core_interop::bin::live_consensus_session` | RED | Operator-driven binary; current HEAD is a "ready" stub. Never linked into the node binary. |
| **Replay harness** | `ade_testkit::consensus::stream_replay::replay_stream` | GREEN | Test-only driver for CE-N-B-5. |

**Rule.** Five rules carry the cluster:

1. **The genesis parser is the sole RED → BLUE materialization point
   for `EraSchedule`.** No other crate may construct an `EraSchedule`
   from anything but a previously-validated one.
2. **`BootstrapAnchorHash` binds the schedule.** The v1 preimage layout
   (`b"ade_bootstrap_v1" || canonical_cbor([byron, shelley, alonzo, conway])`)
   is frozen; bumping it is a version-gated event.
3. **`LedgerView` is a closed trait, not a plugin point.**
4. **The N-B/B1 authoritative chokepoints never move.**
   `validate_and_apply_header`, `select_best_chain`, `apply_rollback`,
   `block_validity`, and (B2) `tx_validity` are the only BLUE entry
   points the orchestrator / composition roots use; new clusters add new
   variants to closed inputs, never new chokepoints.
5. **Selector and chain-dep advance in lockstep through the
   orchestrator.** Header validation always precedes fork-choice.

### Where the boundary is enforced

- `ci_check_dependency_boundary.sh` — no BLUE crate may depend on
  `ade_runtime` or `ade_node`; `ade_network` BLUE submodules may not
  depend on RED submodules within the same crate. The acyclic edge
  `ade_ledger → ade_core` (both BLUE, B1) is verified cycle-free. (B3
  added no new crate edge.)
- `ci_check_no_async_in_blue.sh` — async / tokio / futures forbidden in
  BLUE (incl. `ade_core::consensus`, `ade_ledger::block_validity`, and
  `ade_ledger::{tx_validity, mempool, cert_classify, conway, delegation}`).
- **B4 added no new CI script.** DC-LEDGER-08 (closed, total, era-versioned
  Conway cert-state accumulation) cites the existing full-BLUE
  `ci_check_forbidden_patterns.sh` as its `ci_script`; its closure properties
  are discharged by the compiler-exhaustive `match` in
  `delegation::{conway_cert_action, apply_conway_cert}` (total over all 18
  Conway tags + both removed tags 5/6) plus the named B4 test set
  (`conway_cert_action_total`, `apply_outcome_agrees_with_action`,
  `each_tag_retains_owner_payloads`, `drep_grammar_total`, the four
  `conway_*_is_fail_closed` dispatch tests, `conway_governance_cert_routed_out_of_scope`,
  and the B4-S5 corpus `positive_synthetic_cert_state_accumulates` /
  `cert_state_replay_byte_identical` / `adversarial_no_false_accept`). The
  closed `ConwayCert` shape is additionally grep-gated by the B3F
  `ci_check_conway_cert_classification_closed.sh` — the owner-completion
  enriched the variants' fields but added no open tail, `#[non_exhaustive]`,
  or catch-all accept arm, so that gate continues to pass. The new
  `ade_ledger::delegation` owner-tagged apply types
  (`ConwayCertAction` / `GovernanceOwner` / `GovernanceCertEffect` /
  `OwnerTaggedEffect` / `ConwayCertOutcome` / `ConwayCertEnv`) live in
  `crates/ade_ledger/src/delegation.rs`, which is NOT in the `TARGETS` array
  of `ci_check_consensus_closed_enums.sh` — their closed shape is
  compiler-exhaustive-match + test-and-review-enforced (a narrow gap;
  extending that `TARGETS` array to `crates/ade_ledger/src/delegation.rs`
  would fold it into a grep gate).
- `ci_check_deposit_param_authority.sh` *(NEW in B3)* — across the 6
  BLUE crates, every deposit/refund amount must be sourced from canonical
  ledger state (`ProtocolParameters.{key_deposit, pool_deposit}` +
  `LedgerState.conway_deposit_params`) and NEVER from a literal next to a
  deposit field nor from a testkit `ConwayGovParams`. The sole allowlisted
  non-canonical source is the RED snapshot loader in `ade_testkit`
  (DC-TXV-07).
- `ci_check_conway_cert_classification_closed.sh` *(NEW in B3F —
  DC-TXV-06 partial→enforced)* — three closure gates the compiler-match +
  tests previously carried alone: (1) the classification value types
  `ConwayCert` / `CertDisposition` / `DepositEffect` / `CoinSource` in
  `crates/ade_types/src/conway/cert.rs` stay closed — no
  `#[non_exhaustive]`, no open-tail `Other` / `Unknown` variant; (2)
  `decode_conway_certs` in `crates/ade_codec/src/conway/cert.rs` keeps
  `CodecError::UnknownCertTag` and has no catch-all `_ =>` arm that
  constructs a `ConwayCert` (the reintroduced-Shelley-fallback
  anti-pattern); (3) `cert_classify::classify` in
  `crates/ade_ledger/src/cert_classify.rs` stays exhaustive — no `_ =>`
  wildcard, so a new `ConwayCert` variant breaks the build instead of
  being silently classified. A closure regression now fails CI.
- `ci_check_no_chaindb_in_consensus_blue.sh` *(N-B)* — forbids any
  `ChainDb` / `chain_db` token in `crates/ade_core/src/consensus`.
- `ci_check_no_float_in_consensus.sh` *(N-B)* — forbids `f32` / `f64`
  in `crates/ade_core/src/consensus`.
- `ci_check_no_density_in_fork_choice.sh` *(N-B)* — forbids any
  `density` reference in `fork_choice.rs` / `candidate.rs`.
- `ci_check_consensus_closed_enums.sh` *(N-B; B1- and B2-extended; NOT
  extended in B3)* — four checks (no `#[non_exhaustive]`; no open-tail
  `Other` / `Unknown`; no owned `String` in the named
  error/event/encoding/verdict files; no `Box<dyn>`). Its `TARGETS` set
  covers `crates/ade_core/src/consensus`, `crates/ade_ledger/src/block_validity`,
  `crates/ade_ledger/src/tx_validity`, and `crates/ade_ledger/src/mempool`.
  It is the **sole CI script** carrying `DC-TXV-01..05` and `DC-MEM-01/02`.
  **B3's closed cert/disposition sum types live in
  `crates/ade_types/src/conway/cert.rs`, which is OUTSIDE this `TARGETS`
  set** — but B3F added the dedicated
  `ci_check_conway_cert_classification_closed.sh` to grep-gate exactly
  those surfaces, so the cert/disposition closure is now mechanically
  enforced by its own check (see §3, gap note RESOLVED).
- `ci_check_pallas_quarantine.sh` — only `ade_plutus` may name
  `pallas_*`.
- `ci_check_no_signing_in_blue.sh` — signing patterns forbidden in BLUE;
  only `ade_runtime` may sign.
- `ci_check_ingress_chokepoints.sh` — three checks on `PreservedCbor`
  construction, named block-decoder presence, and raw-CBOR prohibition
  (with the `ade_plutus/src/evaluator.rs` allowlist).
- `ci_check_ce_n_a_5_proof.sh` — N-A live-interop evidence harness.

---

## 3. Closed vs. Extensible Registries

Ade's authority surface is **almost entirely closed.** This is a
consequence of being a chain-compatibility implementation: the
protocol fixes most variants. The few extensible surfaces are
operator-config or testkit-only. **B3 added five closed surfaces** — the
`ConwayCert` cert grammar, the `CertDisposition` / `DepositEffect` /
`CoinSource` deposit-effect sum types, the `RewardAccount` withdrawals-map
key, the `ConwayOnlyDepositParams` / `ConwayDepositParams` canonical
deposit-param surface, and the `UnsupportedStateDependentDepositAccounting`
/ `ValidationEnvironmentError` / `EraInvalidCertificateError` reject
taxonomies — and **no extensible one**. **B4 added five more closed
surfaces** — the owner-tagged Conway apply sum types `ConwayCertAction` /
`GovernanceCertEffect` / `GovernanceOwner` / `OwnerTaggedEffect` /
`ConwayCertOutcome` (all in `ade_ledger::delegation`) — plus the closed
`decode_drep` grammar and the single shared `read_pool_registration_cert`
decode chokepoint, and **enriched two existing closed surfaces in place**
(`ConwayCert` to owner-complete, `PoolRegistrationCert` with an `owners`
field). **B4 added no extensible surface.**

### Closed (frozen — version-gated changes only)

| Registry | Location | Count | Change Rule |
|----------|----------|-------|-------------|
| `CardanoEra` | `ade_types::era` | 8 variants (ByronEbb, ByronRegular, Shelley, Allegra, Mary, Alonzo, Babbage, Conway) | New variant = new hard fork. Coordinated change across `ade_codec`, `ade_ledger`, the canonical type list, and the genesis parser's `later_eras` table. Unknown era tags produce a `CodecError`, never a fallback. |
| `Certificate` | `ade_types::shelley::cert` | 7 variants | Frozen Shelley-era certificate set. New cert types live in `ConwayCert`. **B4:** `PoolRegistrationCert` (in `ade_types::shelley::cert`) gained an `owners: Vec<Hash28>` field (`pool_owners`, additive within the closed surface) and is decoded by the single shared `ade_codec::shelley::cert::read_pool_registration_cert`, the ONE pool-params decode site for both the Shelley and Conway cert decoders. |
| **`ConwayCert`** *(closed CDDL grammar — refined in B3, owner-completed in B4)* | `ade_types::conway::cert` | **19 variants** over CDDL tags `0..18` (incl. the explicit `RemovedInConway { tag }` marker for tags 5/6) | The closed Conway-complete certificate domain type. **B4 — owner-complete:** each variant now retains its owner payloads (credentials, pool id, `PoolRegistration(PoolRegistrationCert)` at tag 3, DRep target), enriched additively without an open tail or `#[non_exhaustive]`. **No `#[non_exhaustive]`, no open-tail `Other`/`Unknown`** — `RemovedInConway` is an explicit closed marker. New cert tag = a new explicit variant + a `decode_conway_certs` decoder arm + a `conway_cert_action` arm + an `apply_conway_cert` arm + a `cert_classify::classify` arm, version-gated. Decoder rejects tags ≥19 with `CodecError::UnknownCertTag`; B3F also rejects trailing bytes (`TrailingBytes`) and bounds preallocation. **Closure grep-gated by `ci_check_conway_cert_classification_closed.sh`** (no `#[non_exhaustive]`/open-tail on the type; no catch-all `_ =>` accept arm in the decoder; exhaustive `classify`); the B4 enrichment kept all three gate properties. |
| `GovAction` | `ade_types::conway::governance` | 7 variants | CIP-1694 fixed; new variant = CIP amendment + ratification chokepoint update. |
| `MIRPot` | `ade_types::shelley::cert` | 2 variants (Reserves, Treasury) | Frozen. |
| `DRep` | `ade_types::conway::cert` | 4 variants | CIP-1694 fixed. **B4:** the closed `ade_codec::conway::cert::decode_drep` reads the `drep = [0,addr_keyhash // 1,script_hash // 2 // 3]` grammar with **no catch-all** — an unknown DRep variant tag rejects deterministically, never an accept. |
| **`CertDisposition`** *(NEW in B3)* | `ade_types::conway::cert` | 3 variants — `Accountable(DepositEffect)`, `Neutral`, `NotValidInConway` | The closed disposition taxonomy `cert_classify::classify` returns. Era-grammar reject (`NotValidInConway`) is deliberately NOT a `DepositEffect`. No `#[non_exhaustive]`, no `String`, no `Box<dyn>`. New disposition = explicit versioned variant + arm in `classify` + arm in the conservation fold. |
| **`DepositEffect`** *(NEW in B3)* | `ade_types::conway::cert` | 2 variants — `NewDeposit(CoinSource)`, `Refund(CoinSource)` | The closed deposit-side/refund-side conservation effect. Closed. |
| **`CoinSource`** *(NEW in B3)* | `ade_types::conway::cert` | 3 variants — `ExplicitInCert(Coin)`, `DepositParam(Coin)`, `RegistrationState(Coin)` | Where a deposit/refund coin comes from. The three sources are the closed provenance set: explicit-in-cert, canonical deposit-param, and registration-state. A fourth source is a versioned change. |
| **`ConwayCertAction`** *(NEW in B4)* | `ade_ledger::delegation` | closed — one variant per Conway cert kind (delegation/pool mutation, owner-tagged governance effect, composite, era-invalid) | The result of the total, compiler-exhaustive `conway_cert_action(&ConwayCert)` classifier. **No `Neutral` variant** — every defined Conway tag has an owner. New cert tag = new explicit variant + arm in `conway_cert_action` + arm in `apply_conway_cert`, version-gated. No `#[non_exhaustive]`, no `String`, no `Box<dyn>`. |
| **`GovernanceCertEffect`** *(NEW in B4)* | `ade_ledger::delegation` | closed — the governance-cert effect kinds (vote-delegation, committee auth/resign, DRep register/unregister/update) | The payload of an owner-tagged effect destined for `ConwayGovState`. Closed; the consuming cluster is PHASE4-B5. New governance-cert kind = new versioned variant + arm in both the producer (`apply_conway_cert`) and the future B5 consumer. |
| **`GovernanceOwner`** *(NEW in B4)* | `ade_ledger::delegation` | closed — names the `ConwayGovState` sub-component an effect is tagged to | The owner tag carried alongside a `GovernanceCertEffect`, identifying which part of `ConwayGovState` the future B5 apply mutates. Closed provenance set. |
| **`OwnerTaggedEffect`** *(NEW in B4)* | `ade_ledger::delegation` | closed struct — `{ owner: GovernanceOwner, effect: GovernanceCertEffect }` | The unit B4 produces and routes out of mutation scope; B5 consumes it. Closed shape, flat-data. |
| **`ConwayCertOutcome`** *(NEW in B4)* | `ade_ledger::delegation` | closed struct — the new `CertState` + `owner_tagged: Vec<OwnerTaggedEffect>` | The total result of `apply_conway_cert`: the B4-owned `CertState` mutation plus the owner-tagged governance effects routed to B5. Composite tags 10/12/13 populate both. Closed. |
| `PlutusLanguage` | `ade_plutus::evaluator` | 3 variants (V1, V2, V3) | New variant = new Plutus version. Requires cost-model table extension + aiken bump. PV11 builtins gated off (S-29). |
| **Named ingress chokepoints (block CBOR)** | `ade_codec::{cbor::envelope, byron, shelley, allegra, mary, alonzo, babbage, conway, address}` | 10 — `decode_block_envelope`, the per-era block decoders, `decode_address` | Header comment of `ci_check_ingress_chokepoints.sh` enumerates this set. New era = new chokepoint in lockstep with a `CardanoEra` variant. Removal forbidden. |
| **Conway cert/withdrawals sub-grammar decoders** *(NEW in B3; cert decoder owner-completed in B4)* | `ade_codec::conway::{cert::{decode_conway_certs, decode_drep}, withdrawals::{decode_withdrawals, withdrawals_sum}}` + the shared `ade_codec::shelley::cert::read_pool_registration_cert` | 5 functions | Closed sub-grammars inside the Conway tx body (keys 4 and 5). NOT block-envelope chokepoints; they read already-lifted body slices via the `ade_codec` primitive set. **No catch-all accept arm in `decode_conway_certs`** (tags ≥19 → `UnknownCertTag`; B3F: trailing bytes → `TrailingBytes`, bounded preallocation — DC-VAL-06); **B4** made `decode_conway_certs` owner-complete (retains all owner payloads), added the closed `decode_drep` (no catch-all), and relocated the pool-params decode to the single shared `read_pool_registration_cert` called by **both** era decoders (no second Conway decoder — DC-LEDGER-08). `decode_withdrawals` rejects a repeated key with `DuplicateMapKey` (never last-wins). The cert-decoder closure is grep-gated by `ci_check_conway_cert_classification_closed.sh` (B3F; still passes after the B4 enrichment). Removal/renaming forbidden. |
| **Named ingress chokepoint (Plutus script CBOR)** | `ade_plutus::evaluator::PlutusScript::from_cbor` | 1 — file `crates/ade_plutus/src/evaluator.rs` | Distinct from the block-CBOR chokepoints. Allowlisted by exact file path in Check 3 of `ci_check_ingress_chokepoints.sh`. |
| **`PreservedCbor::new` constructor** | `ade_codec::preserved` | 1 chokepoint, `pub(crate)` | Construction lives inside `ade_codec`. |
| **`CodecError` variants** *(extended in B3)* | `ade_codec::error` | + `UnknownCertTag { tag, offset }`, `DuplicateMapKey { offset }` | The closed codec-error taxonomy; B3 added the two cert/withdrawal-grammar rejects. Flat-data, no `String`. |
| **Mini-protocol message enums** | `ade_network::codec::*` | 11 closed enums | Closed wire grammar per protocol. No `#[non_exhaustive]`, no `dyn` dispatch, no generic `Codec<P>` trait. New mini-protocol = new module + new closed enum + new chokepoint pair + new `*Version` enum + new transition. |
| **Mini-protocol encode/decode chokepoints** | `ade_network::codec::*::{encode_<protocol>_message, decode_<protocol>_message}` | 22 functions | Single chokepoint per direction per protocol. Removal/renaming forbidden (DC-PROTO-01..05). |
| **Mux frame chokepoints** | `ade_network::mux::frame::{encode_frame, decode_frame}` | 2 free functions | The **single** byte↔frame translation in the project. |
| **Mini-protocol transition functions** | `ade_network::*::transition` + `n2c::local_*::transition` | 8 state-machine modules | Each `fn (state, agency, version, msg) -> Result<...>` — pure, sync, no ambient session influence (DC-PROTO-06). |
| **Mini-protocol version enums** | `ade_network::codec::version::*` | 11 closed enums | Each pins the upper version this codec/state-machine pair has been audited against. Bumping = registry diff + new corpus + cluster doc. |
| **`ChainDb` trait surface** | `ade_runtime::chaindb::mod` | 6 methods | Object-safe; intended for multiple impls. |
| **`SnapshotStore` trait surface** | `ade_runtime::chaindb::mod` | 5 methods | Bytes opaque at this layer (S-35). |
| **`Recoverable` trait surface** | `ade_runtime::recovery` | 2 methods + 1 associated type | Caller-supplied; single error type per impl. |
| **`recover` entry point** | `ade_runtime::recovery::recover` | 1 free function | The sole composition of `ChainDb` + `SnapshotStore` + `Recoverable`. |
| **Hash domain functions** | `ade_crypto::blake2b::{block_header_hash, transaction_id, script_hash, credential_hash}` | 4 named domains | Algorithm immutable per protocol version. |
| **`ChainEvent`** *(N-B)* | `ade_core::consensus::events` | 5 variants | Complete output taxonomy of the fork-choice + rollback transitions. No `#[non_exhaustive]`, no `Other`, no `String`. |
| **`ChainSelectionReject`** *(N-B)* | `ade_core::consensus::events` | 4 variants | Complete reject taxonomy. Flat-data so corpus comparisons are byte-stable. |
| **Consensus error families** *(N-B)* | `ade_core::consensus::errors` | 8 closed error enums | Each flat-data, no `String`, no `Box<dyn>`. |
| **`StreamInput`** *(N-B)* | `ade_runtime::consensus::chain_selector` | 3 variants | The single ingress taxonomy for the chain-selector orchestrator. No plugin-style extension. |
| **`OrchestratorError`** *(N-B)* | `ade_runtime::consensus::chain_selector` | 2 variants | Fail-fast `Err`. Structured rejects ride inside `Ok(Some(ChainEvent::Rejected))`. |
| **`DecodeError`** *(N-B)* | `ade_core::consensus::encoding` | 4 variants | Closed CBOR-decode error taxonomy. `Cbor` payload is `&'static str`. |
| **`GenesisParseError`** *(N-B)* | `ade_runtime::consensus::genesis_parser` | 5 variants | Closed RED-side parse-error taxonomy. `field` is `&'static str`. |
| **`GenesisBlob`** *(N-B)* | `ade_runtime::consensus::genesis_parser` | 4 variants | Closed because the genesis bundle is structurally a four-tuple at v1. |
| **`NetworkMagic`** *(N-B)* | `ade_runtime::consensus::genesis_parser` | 3 const-named values | Unknown magic → `UnknownNetwork`, never a default. |
| **`LedgerView` trait** *(N-B; B1-refined)* | `ade_core::consensus::ledger_view` | 4 methods (`pool_vrf_keyhash -> Hash32`) | Closed-shape boundary. Not a plugin point — production adds `PoolDistrView`, tests add `LedgerViewStub`. |
| **`HeaderVrf`** *(N-B; surfaced at B1)* | `ade_core::consensus::header_summary` | 2 variants — Tpraos / Praos | Era-dispatched. B1's `decode_block` builds only `Praos` (Babbage/Conway); `Tpraos` is the documented pre-Babbage extension point. |
| **`BlockValidityVerdict`** *(B1)* | `ade_ledger::block_validity::verdict` | 2 variants | The block-validity composition verdict. Closed; enforced by `ci_check_consensus_closed_enums.sh`. |
| **`BlockValidityError` / `BlockRejectClass` / `FieldKind` / `FieldError` / `MissingInput`** *(B1)* | `ade_ledger::block_validity::verdict` | 5 / 5 / 9 / struct / 4 | Full structured reject + coarse class + closed fixed-size-field set. New class = new variant + arm in `class()` + corpus regeneration. |
| **`VerdictSurface` / `SurfaceDecodeError`** *(B1)* | `ade_ledger::block_validity::encoding` | 2 / 3 variants | CBOR-round-trippable coarse comparison surface; full error NOT encoded (T-DET-01). |
| **`block_validity` chokepoint** *(B1)* | `ade_ledger::block_validity::transition` | 1 function | The single block-level composition root. Does not move; introduces no rules (DC-VAL-02). |
| **`TxValidityVerdict`** *(B2)* | `ade_ledger::tx_validity::verdict` | 2 variants — Valid { tx_id, applied }, Invalid { class, error } | The single-tx composition verdict, paralleling `BlockValidityVerdict`. Closed; enforced by `ci_check_consensus_closed_enums.sh` (target extended to `tx_validity`). |
| **`TxRejectClass`** *(B2)* | `ade_ledger::tx_validity::verdict` | 5 variants — Phase1Invalid, WitnessInvalid, MissingRequiredSigner, Phase2Invalid, MalformedField | The **canonical/replay comparison surface**. CBOR-round-trippable (discriminants 0..4 fixed). New class = new variant + arm in `class_discriminant`/`class_from_discriminant` + corpus regeneration. |
| **`TxValidityError`** *(B2)* | `ade_ledger::tx_validity::verdict` | 5 variants — Decode(LedgerError), Witness(WitnessClosureError), Phase1(LedgerError), Phase2(LedgerError), MalformedField(FieldError) | The full structured reject reason. Closed. A total `class()` projects it onto `TxRejectClass`. |
| **`SignerSource`** *(B2 — the DC-TXV-05 surface)* | `ade_ledger::tx_validity::required_signers` | 6 variants — InputPaymentKey, ExplicitRequiredSigner, WithdrawalKey, CertificateKey, GovernanceVoter, CollateralPaymentKey | The **closed, era-versioned required-signer enumeration**. A signer source not in the enum is impossible to silently omit. New source = explicit, versioned addition + arm everywhere it is derived. |
| **`RequiredSignerError` / `RequiredSignerField`** *(B2)* | `ade_ledger::tx_validity::required_signers` | 3 / 4 variants | Closed fail-closed derivation-error taxonomy (UnresolvableInput / MalformedField / UnsupportedEra). No `String`. |
| **`WitnessClosureError` / `WitnessField`** *(B2)* | `ade_ledger::tx_validity::witness` | 3 / 2 variants | The fail-closed witness-coverage error shape. Reports WHICH `SignerSource` obligation went uncovered. No `String`. |
| **`TxVerdictSurface` / `TxSurfaceDecodeError`** *(B2)* | `ade_ledger::tx_validity::encoding` | 2 / 3 variants | The CBOR-round-trippable per-tx comparison surface (`Valid -> [0, tx_id]`, `Invalid -> [1, class]`); the full `TxValidityError` detail is NOT encoded (T-DET-01). |
| **`tx_validity` chokepoint** *(B2)* | `ade_ledger::tx_validity::transition` | 1 function | The single per-tx composition root. Does not move; gains no second public entry; introduces no validation rules (DC-TXV-02). |
| **Tx-verdict-surface encode/decode chokepoints** *(B2)* | `ade_ledger::tx_validity::encoding::{encode_tx_verdict_surface, decode_tx_verdict_surface}` | 2 functions | Frozen CBOR for the per-tx comparison surface. Round-trip required; field/discriminant additions are version-gated. |
| **`AdmitOutcome`** *(B2)* | `ade_ledger::mempool::admit` | 2 variants — Admitted { tx_id }, Rejected { class, error } | The closed Tier-1 admission outcome. Closed — enforced by `ci_check_consensus_closed_enums.sh` (target extended to `mempool`). |
| **`MempoolState`** *(B2)* | `ade_ledger::mempool::admit` | struct { accepted: Vec<Hash32>, accumulating: LedgerState } | The closed mempool state. The only state carried across `admit` calls. |
| **`OrderPolicy`** *(B2)* | `ade_ledger::mempool::policy` | 2 variants — ArrivalOrder, TxIdAscending | The closed Tier-5 ordering-policy set. A policy is a pure projection over the admitted-id list (DC-MEM-02). New policy = new variant; may never read validity. |
| **`ConwayOnlyDepositParams`** *(NEW in B3)* | `ade_ledger::pparams` | struct { drep_deposit: Coin, gov_action_deposit: Coin } | The Conway-only deposit params (the two CIP-1694 governance deposits). Closed shape; the value-set is per-state. |
| **`ConwayDepositParams`** *(NEW in B3)* | `ade_ledger::pparams` | struct (view) { key_deposit, pool_deposit, drep_deposit, gov_action_deposit } | The single canonical view combining `ProtocolParameters.{key_deposit, pool_deposit}` with the Conway-only pair — every deposit/refund amount in BLUE flows from here (DC-TXV-07). Built only via `LedgerState::conway_deposit_view()`. |
| **`ValidationEnvironmentError`** *(NEW in B3)* | `ade_ledger::error` | incl. `MissingConwayDepositParams` | The fail-fast environment-error taxonomy returned when the deposit-param view is consulted on a non-Conway state. Closed, no `String`. |
| **`UnsupportedStateDependentDepositAccounting`** *(NEW in B3)* | `ade_ledger::error` | structured (e.g. `LegacyUnregistrationRefundUnresolved`) | The `cert_classify` reject for a deposit/refund that cannot be resolved from registration state — never a guessed amount (DC-TXV-06). Closed. |
| **`EraInvalidCertificateError`** *(NEW in B3)* | `ade_ledger::error` | struct { cert_index: u16, removed_tag } | The §9.1 step-2 reject for a known-but-removed cert tag (5/6). Closed, flat-data. |
| **`PraosNonces` / `NonceScanError`** *(B1)* | `ade_ledger::consensus_input_extract` | 1 struct (5 nonces) + 1 error | The consensus-input extraction shape. Exact-five-nonce requirement is a closure invariant. |
| **`PraosChainDepState` / `ChainEvent` canonical encodings** *(N-B)* | `ade_core::consensus::encoding` | 4 chokepoints | Frozen CBOR; round-trip required (T-DET-01); field additions are version-gated. |
| **`LedgerFingerprint` fold** *(B3-extended)* | `ade_ledger::fingerprint` | + `CONWAY_DEPOSIT_PARAMS_TAG` fold | The canonical `LedgerState` fingerprint; B3 added a deposit-param fold that is byte-identical for any non-Conway state (DC-LEDGER-01, enforced by `ci_check_ledger_determinism.sh`). |
| **CI check set** | `ci/ci_check_*.sh` | 27 scripts | Existing checks may be tightened, never relaxed. New CI check is additive. Deleting a script requires recording the deprecation in the registry's `ci_scripts` arrays. (B3 added `ci_check_deposit_param_authority.sh`; B3F added `ci_check_conway_cert_classification_closed.sh`; **B4 added none** — DC-LEDGER-08 reuses `ci_check_forbidden_patterns.sh`.) |
| **Invariant registry families** | `docs/ade-invariant-registry.toml` | Families T / CN / DC / OP / RO; DC extended in N-A (`DC-PROTO-*`, `DC-CORE-01`), N-B (`DC-CONS-03..10`), B1 (`DC-VAL-01..06`), B2 (`DC-TXV-01..05`, `DC-MEM-01/02`), B3 (`DC-TXV-06`, `DC-TXV-07`), and **B4 (`DC-LEDGER-08`)** | Append-only IDs; rules may be strengthened, never weakened; deprecation needs an explicit `deprecated_in`. |

### Extensible (open within constraints)

| Registry | Location | Extension Rule |
|----------|----------|---------------|
| `CostModels` map (Plutus V1/V2/V3 cost tables) | `ade_plutus::cost_model::CostModels` | New entries enter via the cost-model CBOR decoder when a protocol parameter update lands. Not runtime-pluggable; constrained by the closed `PlutusLanguage` set. |
| `ProtocolParameters` / `ProtocolParameterUpdate` field set | `ade_ledger::pparams` | Fields are appended per era. Versioned-gated by era. **B3 note:** the Conway-only `ConwayOnlyDepositParams` (`drep_deposit`, `gov_action_deposit`) are a closed-shape addition; the deposit *view* combining them is `ConwayDepositParams`. |
| Pool / DRep / Stake registrations | `ade_ledger::state::{DelegationState, CertState}` | Mutated at runtime by `ade_ledger::delegation::apply_cert` (Shelley..Babbage) and, **as of B4**, by `ade_ledger::delegation::apply_conway_cert` (Conway, owner-tagged). The **shape** of what can be registered is closed; the **set** of registrations is open and grows monotonically. **B3 note:** registration state is now the authoritative source for `CoinSource::RegistrationState` refunds (`cert_classify`). **B4 note:** `apply_pool_registration` now populates `PoolParams.owners` from the enriched cert; Conway delegation/pool certs mutate `CertState` here, while governance certs are owner-tagged out of scope (PHASE4-B5). |
| Governance proposal set | `ade_ledger::state::ConwayGovState::proposals` | Shape closed, instance set open, lifecycle managed by `evaluate_ratification` / `enact_proposals` / `expire_proposals`. **B4 note:** B4 produces owner-tagged governance-cert effects (`ConwayCertOutcome.owner_tagged`) destined for this state but does NOT apply them; **PHASE4-B5 is the declared cluster that folds the owner-tagged `GovernanceCertEffect`s into this lifecycle** (DC-LEDGER-08). |
| `OpCertCounterMap` *(N-B)* | `ade_core::consensus::praos_state` | BTreeMap keyed by `(Hash28, u64)`. Inserts strictly increasing per `(pool, kes_period)`. Shape closed; set open. |
| `PoolDistrView` pool table *(B1)* | `ade_ledger::consensus_view::PoolDistrView::pools` | `BTreeMap<Hash28, PoolEntry>`. Shape closed; set of pools open (whatever the operating-epoch snapshot contains). Built once per epoch; not runtime-pluggable. |
| Withdrawals map *(NEW in B3)* | decoded by `ade_codec::conway::withdrawals::decode_withdrawals` → `BTreeMap<RewardAccount, Coin>` | The **shape** is closed (deduplicated map; `DuplicateMapKey` rejects a repeat); the **set** of withdrawals is open and is whatever the tx body demands. Built deterministically per tx; not a registry — never last-wins. |
| Mempool admitted set *(B2)* | `ade_ledger::mempool::admit::MempoolState::accepted` | `Vec<Hash32>` of admitted tx ids in admission order. Shape closed; set open and grows monotonically per accepted tx. Mutated only by `admit` (Tier-1). NOT runtime-pluggable; no policy may add/remove ids (DC-MEM-02). |
| `SignerSource` provenance set *(B2)* | `ade_ledger::tx_validity::required_signers::RequiredSigners::{keys, provenance}` | `BTreeSet<Hash28>` + `BTreeSet<(SignerSource, Hash28)>`. The `SignerSource` *enum* is closed; the per-tx **set** of required keys is open. Built deterministically per tx; not a registry. |
| `RollbackSnapshot` ring *(N-B)* | `ade_runtime::consensus::chain_selector::OrchestratorState::recent_snapshots` | Bounded `Vec<RollbackSnapshot>` capped at `DEFAULT_SNAPSHOT_LIMIT = 2160`. No plugin extension. |
| Oracle reference snapshots / regression corpus | `ade_testkit::harness::{snapshot_loader, regression_corpus}` | Tooling-only. New oracle data via `corpus/` + manifest update. `ci_check_ref_provenance.sh` enforces checksum integrity. GREEN. **B3 note:** the snapshot loader is the one allowlisted source of `conway_deposit_params`. |
| Network corpus (mini-protocol transcripts) | `corpus/network/{n2n,n2c}/*` | Tooling-only. Captured via `ade_network::bin::capture_*`. Append-only by convention. |
| Consensus corpus | `corpus/consensus/*` | Tooling-only. Append-only by convention. |
| Block-validity corpus *(B1)* | `corpus/validity/{conway_epoch576, adversarial}/` | Tooling-only. Positive + adversarial; both replay byte-identically (T-DET-01, DC-VAL-04). GREEN harness in `ade_testkit::validity`. |
| Tx-validity corpus *(B2; B3-extended)* | the Conway-576 corpus txs extracted by `ade_testkit::tx_validity::extract` + the family A/B adversarial mutators + the **B3 conservation positive + adversarial corpora** (CE-B3-5/6) | Tooling-only. Positive = on-wire Conway txs; adversarial = witness/value/input mutations + B3 deposit/refund/withdrawal-conservation mutators. Append-only; GREEN harness in `ade_testkit::tx_validity`. |
| `KillStrategy<D>` trait impls | `ade_runtime::chaindb::crash_safety` | RED-only test infrastructure. Object-safe by intent. |
| Recovery state types | callers of `Recoverable` | Open: any state type with a canonical encode + apply-block step. The trait is the only way in. |
| Pinned external crates | `crates/*/Cargo.toml` | New external crate requires a Tier-5 rationale doc (per `docs/active/CE-79_tier5_addendum.md`). |

### Candidates — extensible surfaces not yet wired

| Cluster | Candidate registry | Rationale |
|---------|-------------------|-----------|
| B+ / N-E | Mempool eviction / prioritization policy (beyond the `OrderPolicy` stub) | Tier-5 — operator-tunable. Plugin trait candidate: `MempoolPolicy`. MUST stay below the Tier-1 `admit` gate (DC-MEM-02) — never reads `tx_validity`. |
| N-A (deferred) | Peer address book | Operator-supplied; runtime mutable. Should live in `ade_runtime`. |
| N-C | Block-production policy (forge cadence, KES rotation, slot election) | Tier 1 semantics, Tier 5 operator triggers. Forge inputs must reduce to the existing `BlockEnvelope` chokepoint. |
| N-F | Query API method set | Tier 5 wire / Tier 1 semantics. Closed enum internally, mapped to gRPC/HTTP at the edge; shared with LSQ/LocalTxMonitor semantic dispatch. The LocalTxMonitor query set reads the `mempool::admit` accepted set. |
| N-F | Prometheus metric names | Tier 5; append-only registry expected. |

User confirmation needed for each at cluster entry: closed enum vs.
trait-based registry; runtime-extensible vs. compile-time-fixed; CI
enforcement shape.

### Closed-grammar audit (PHASE4-B3 specific)

This sweep was performed after PHASE4-B3 close. The author should
confirm each is intended-closed (no future plugin point) before any
extension is proposed:

1. `ConwayCert` (19 variants, CDDL tags `0..18`) — **closed by intent.**
   The decoder `decode_conway_certs` has **no catch-all accept arm**:
   tags ≥19 → `CodecError::UnknownCertTag`, tags 5/6 → the explicit
   `RemovedInConway` marker. A new Conway cert tag is an explicit
   versioned variant + decoder arm + classifier arm — never an open tail.
   B3F also rejects trailing bytes after the cert array (`TrailingBytes`)
   and bounds preallocation (DC-VAL-06). **Grep-gated as of B3F** by the
   dedicated `ci_check_conway_cert_classification_closed.sh` (no catch-all
   `_ =>` accept arm; `UnknownCertTag` present); the gap note below is now
   RESOLVED.
2. `CertDisposition` / `DepositEffect` / `CoinSource` — **closed by
   intent.** The classifier `cert_classify::classify` is a total,
   compiler-exhaustive map; an unresolvable state-dependent deposit/refund
   is the structured `UnsupportedStateDependentDepositAccounting` reject,
   never a fabricated amount and never the `key_deposit` param. Era-grammar
   reject (`NotValidInConway`) is deliberately NOT a `DepositEffect`. The
   three `CoinSource` variants are the closed deposit-provenance set.
   **Grep-gated as of B3F** — `ci_check_conway_cert_classification_closed.sh`
   asserts the value types stay closed (no `#[non_exhaustive]`/open-tail)
   and that `classify` keeps no `_ =>` wildcard, so a new `ConwayCert`
   variant breaks the build instead of being silently classified.
3. Withdrawals map grammar (`decode_withdrawals`) — **closed by intent.**
   A repeated `RewardAccount` key is a hard `DuplicateMapKey` reject —
   **never last-wins** — so `withdrawals_sum` only ever runs over a fully
   deduplicated map. Trailing bytes after the map reject.
4. `ConwayOnlyDepositParams` / `ConwayDepositParams` deposit-param surface
   — **closed by intent, canonical-only.** Every BLUE deposit/refund
   amount flows from `conway_deposit_view()` (DC-TXV-07); a literal next
   to a deposit field or a testkit `ConwayGovParams` read is a CI failure
   (`ci_check_deposit_param_authority.sh`). The sole allowlisted
   non-canonical source is the RED snapshot loader in `ade_testkit`.
5. The §9.1 reject precedence (decode → era-validity → missing-environment
   → state-dependent-accounting → conservation) — **frozen.** The
   era-validity sweep runs across all certs before any accounting fold,
   so the rejected reason is deterministic and independent of cert
   ordering; no later check may mask an earlier failure (T-CONSERV-01 /
   CN-LEDGER-07 strengthened).

**Gap note — RESOLVED in B3F.** The prior revision flagged that the
closed `ConwayCert` / `CertDisposition` / `DepositEffect` / `CoinSource`
enums and the cert-decoder closure (no catch-all, `UnknownCertTag` for
≥19, `RemovedInConway` for 5/6) rested only on the compiler-exhaustive
`match` in `cert_classify::classify` plus named `cargo test` targets
(CE-B3-2), because `crates/ade_types/src/conway/cert.rs` was not in the
`TARGETS` array of `ci_check_consensus_closed_enums.sh`. B3F closed this:
the new `ci_check_conway_cert_classification_closed.sh` grep-gates exactly
those three files — the closed value types
(`crates/ade_types/src/conway/cert.rs`), the no-catch-all decoder
(`crates/ade_codec/src/conway/cert.rs`), and the exhaustive `classify`
(`crates/ade_ledger/src/cert_classify.rs`). A closure regression
(open-tail variant, `#[non_exhaustive]`, a catch-all decoder arm
constructing a `ConwayCert`, or a `_ =>` wildcard in `classify`) now fails
CI. DC-TXV-06 moved partial→enforced. **No remaining candidate here.**

### Closed-grammar audit (PHASE4-B4 specific)

This sweep was performed after PHASE4-B4 close.

1. **Owner-complete `ConwayCert`** — **closed by intent.** The
   owner-completion enriched each variant's fields but added no open tail,
   `#[non_exhaustive]`, or catch-all accept arm; `decode_conway_certs` keeps
   `UnknownCertTag` for ≥19 and `RemovedInConway` for 5/6. The B3F
   `ci_check_conway_cert_classification_closed.sh` grep-gate still passes.
2. **`decode_drep` grammar** — **closed by intent.** The `drep` variant set
   is read with no catch-all; an unknown DRep variant tag rejects
   deterministically, never an accept.
3. **Single shared `read_pool_registration_cert`** — **the no-new-parallel-decoder
   rule.** There is ONE pool-params decode site (`ade_codec::shelley::cert`),
   called by both era cert decoders; a second era-specific copy is forbidden
   (DC-LEDGER-08). Confirm before any future era adds a pool-params decode.
4. **Owner-tagged apply sum types** (`ConwayCertAction` / `GovernanceCertEffect`
   / `GovernanceOwner` / `OwnerTaggedEffect` / `ConwayCertOutcome`) — **closed
   by intent.** `conway_cert_action` and `apply_conway_cert` are total,
   compiler-exhaustive maps over `ConwayCert` (all 18 tags + 5/6); there is
   **no `Neutral` action** (every defined tag has an owner). A new variant
   breaks the build rather than being silently neutralized.
5. **Owner-tagging boundary → `ConwayGovState`** — **a confirmed extension
   point, not a closed-by-accident surface.** Governance certs are decoded
   fully and owner-tagged (`ConwayCertOutcome.owner_tagged`), routed out of
   B4's mutation scope; the consuming cluster is **PHASE4-B5** (declared in
   DC-LEDGER-08 and the B4 cluster doc). This is the intended seam, not a gap.

**Gap note — B4 (narrow, carried).** The `ade_ledger::delegation`
owner-tagged apply types live in `crates/ade_ledger/src/delegation.rs`, which
is NOT in the `TARGETS` array of `ci_check_consensus_closed_enums.sh`, so
their closed shape (no `#[non_exhaustive]` / open-tail / `String` /
`Box<dyn>`) is compiler-exhaustive-match + test-and-review-enforced rather
than grep-gated. Extending that `TARGETS` array to
`crates/ade_ledger/src/delegation.rs` would fold them into a grep gate.
Surfaced for confirmation.

No surfaces in this cluster look closed by accident.

---

## 4. Version-Gated vs. Frozen Contracts

### Frozen (immutable at current version — change = new major version)

- **Cardano-canonical CBOR wire format**: Each `decode_*_block` in
  `ade_codec` produces values whose wire bytes are preserved
  byte-identically. Hash inputs are wire bytes, not re-encoded bytes
  (enforced by `ci_check_hash_uses_wire_bytes.sh`).
- **Block envelope shape**: `[era_tag:u8, era_block:CBOR]`; era tags
  0..=7 (closed).
- **`PreservedCbor<T>` invariant**: `wire_bytes()` is exactly what the
  decoder consumed, byte-identical.
- **Hash algorithms**: Blake2b-224 for credential / script hashes,
  Blake2b-256 for block / transaction / Merkle hashes. Ed25519,
  Byron-bootstrap, KES-sum, VRF-draft-03 — all protocol-frozen.
- **Era-correct block body hash** *(wired at B1)*: for Alonzo+ the body
  hash is computed over the **preserved CBOR segment bytes** (never
  re-encoded — T-ENC-01). The body-hash binding in `block_validity`
  pivots on this.
- **Tx id over preserved body bytes** *(wired at B2)*: `tx_id =
  blake2b_256(preserved_body_slice)` — the body slice is lifted
  byte-for-byte out of the full tx CBOR; never a re-encode (T-ENC-01).
  Both `tx_validity` and the witness closure pivot on this hash.
- **Conway certificate CDDL grammar** *(NEW in B3; hardened + grep-gated
  in B3F)*: `decode_conway_certs` is a closed grammar over tags `0..18`
  with **no catch-all accept arm** — tags ≥19 → `CodecError::UnknownCertTag`,
  tags 5/6 → `ConwayCert::RemovedInConway`. B3F made the reader exact:
  trailing bytes after the cert array → `CodecError::TrailingBytes`
  (parity with `decode_withdrawals`), and preallocation is bounded
  (DC-VAL-06). The 19-variant `ConwayCert` shape is frozen for the Conway
  protocol version; a new tag is a version-gated variant + decoder arm +
  classifier arm. The closure is mechanically defended by
  `ci_check_conway_cert_classification_closed.sh` (B3F). **B4 made the
  decode owner-complete** — every variant retains its owner payloads — and
  **froze the single shared pool-params decoder**: `read_pool_registration_cert`
  (in `ade_codec::shelley::cert`) is the ONE pool-params decode site, called by
  both era decoders; a second parallel Conway decoder is forbidden
  (DC-LEDGER-08).
- **Conway `DRep` decode grammar** *(NEW in B4)*: `decode_drep` reads the
  closed `drep = [0,addr_keyhash // 1,script_hash // 2 // 3]` grammar with no
  catch-all; an unknown variant tag rejects.
- **Owner-tagged Conway cert-state apply contract** *(NEW in B4)*: for each
  block at `track_utxo`, `accumulate_tx_certs` era-dispatches cert decode +
  apply (Conway → owner-complete `decode_conway_certs` + `apply_conway_cert`;
  Shelley..Babbage → `decode_certificates` + `apply_cert`); every Conway cert
  resolves to an owner-tagged disposition (mutates B4-owned `CertState`, or is
  owner-tagged to `ConwayGovState` and routed out of scope, or is a structured
  reject); composite tags 10/12/13 do both; removed tags 5/6 reject as
  `EraInvalidCertificate`; a decode/apply error propagates as a structured
  `LedgerError` and halts the block transition (no fail-open swallow). The
  classifier `conway_cert_action` and `apply_conway_cert` are total over all
  18 tags + 5/6, with no `Neutral` action (DC-LEDGER-08).
- **Conway withdrawals map grammar** *(NEW in B3)*: `decode_withdrawals`
  produces a deduplicated `BTreeMap<RewardAccount, Coin>` — a repeated key
  is a hard `CodecError::DuplicateMapKey` reject (never last-wins);
  trailing bytes reject. `withdrawals_sum` is exact `i128`.
- **Closed deposit-effect sum types** *(NEW in B3)*: `CertDisposition`
  (3) / `DepositEffect` (2) / `CoinSource` (3) — frozen shapes; era-grammar
  reject (`NotValidInConway`) is deliberately not an accounting effect.
- **Canonical deposit-param authority** *(NEW in B3)*: every
  deposit/refund amount in BLUE is sourced from
  `ProtocolParameters.{key_deposit, pool_deposit}` +
  `LedgerState.conway_deposit_params` via `conway_deposit_view()`
  (DC-TXV-07). The classifier never fabricates an amount and never uses
  the `key_deposit` param as a stand-in for a recorded refund.
- **Full Conway value-conservation equation** *(NEW in B3)*: `consumed =
  Σ inputs + Σ withdrawals + refunded_deposits == produced = Σ outputs +
  fee + donation + new_deposits` with the **frozen §9.1 reject
  precedence** (decode → era-validity → missing-environment →
  state-dependent-accounting → conservation; lowest-numbered failure
  wins). `i128` throughout; no float, no rounding (T-CONSERV-01 /
  CN-LEDGER-07 strengthened; DC-VAL-06 strengthened — the B2 cert/withdrawal
  early-out is removed).
- **`LedgerFingerprint` Conway deposit-param fold** *(NEW in B3)*: folded
  under `CONWAY_DEPOSIT_PARAMS_TAG`; byte-identical to the prior
  fingerprint for any non-Conway state (DC-LEDGER-01).
- **Plutus script ingress chokepoint**: `PlutusScript::from_cbor` in
  `crates/ade_plutus/src/evaluator.rs`. Moving it invalidates the
  path-exact allowlist in `ci_check_ingress_chokepoints.sh` Check 3.
- **Plutus language set**: V1, V2, V3. PV11 builtins gated off (S-29).
- **Aiken UPLC quarantine pin**: `aiken_uplc` at tag `v1.1.21`, commit
  `42babe5d`.
- **Ouroboros mux frame layout**: 8-byte big-endian header, payload
  `≤ 65535` bytes.
- **11 closed mini-protocol message enums** + **8 closed state graphs**
  (N-A): wire grammar and legal `(state, agency, version, msg)` tuple
  set per protocol are protocol-fixed.
- **`BootstrapAnchorHash` v1 preimage** *(N-B)*: Blake2b-256 over
  `b"ade_bootstrap_v1" || canonical_cbor([byron, shelley, alonzo,
  conway])`. Domain tag, ordering, encoding, and algorithm frozen for v1.
- **`EraSchedule` invariants** *(N-B)*: monotonic `start_slot`, non-empty
  era list, non-zero `slot_length_ms` and `epoch_length_slots`.
- **`PraosChainDepState` / `ChainEvent` CBOR encodings** *(N-B)*: frozen
  for the protocol version; round-trip byte-identical (T-DET-01).
- **Consensus error taxonomies** *(N-B)*: flat-data, `String`-free,
  `Box<dyn>`-free, replay-stable.
- **`StreamInput` 3-variant taxonomy** *(N-B)*. **`HeaderVrf` era model**
  *(N-B)*: two arms (Tpraos / Praos), era selects the arm.
- **`block_validity` composition contract** *(B1)*: `Valid` iff header
  authority ∧ body-hash binding ∧ body authority all accept (DC-VAL-02);
  header-before-body fail-fast (DC-VAL-03); no partial mutation on the
  invalid path (DC-VAL-05). Pure, total, deterministic (DC-VAL-01).
- **`VerdictSurface` CBOR encoding** *(B1)*: only the coarse class is
  encoded; round-trip byte-identical (T-DET-01).
- **`LedgerView` trait shape** *(N-B; B1-refined)*: 4 `Option`-returning
  methods; `pool_vrf_keyhash -> Hash32` is the registered-VRF surface.
- **`tx_validity` composition contract** *(B2)*: `Valid` iff
  phase-1 ∧ (phase-2 when Plutus scripts present) accept (DC-TXV-02);
  phase-1-before-phase-2 fail-fast; no partial mutation on the invalid
  path (DC-TXV-04). Pure, total, deterministic over `(LedgerState,
  tx_cbor)` (DC-TXV-01). The composer adds no rules of its own. (B3
  tightened the phase-1 authority it composes, not the composer.)
- **`SignerSource` enumeration** *(B2)*: the 6-variant closed,
  era-versioned required-signer surface (DC-TXV-05), grounded in Conway
  `getConwayWitsVKeyNeeded` + `getVKeyWitnessConwayTxCert`, frozen for the
  Conway protocol version.
- **Witness-closure contract** *(B2)*: coverage is by key hash =
  `Blake2b-224(vkey)`, signature verified over the preserved body hash,
  fail-closed; wrong-size fields and uncovered keys are hard rejects, and
  an extra irrelevant witness never substitutes (DC-VAL-06 /
  CN-LEDGER-09).
- **`TxVerdictSurface` CBOR encoding** *(B2)*: `Valid -> [0, tx_id]`,
  `Invalid -> [1, reject_class_discriminant]`; `TxRejectClass`
  discriminants 0..4 fixed; only the coarse class is encoded; round-trip
  byte-identical (T-DET-01).
- **Mempool admission contract** *(B2)*: `admit`'s verdict equals
  `tx_validity`'s verdict; no false accept; on Invalid the mempool is
  returned unchanged (DC-MEM-01). The Tier-5 `OrderPolicy` projection is a
  deterministic permutation of the admitted set that cannot change a
  verdict (DC-MEM-02).
- **All canonical types**: shapes frozen at the era / version they
  entered. Adding fields requires a versioned gate; renaming forbidden.
- **TCB color assignments**: per `.idd-config.json` `core_paths`.
  `ade_core::consensus`, `ade_ledger::{block_validity, tx_validity,
  mempool::admit, consensus_view, cert_classify, delegation}`,
  `ade_codec::conway::{cert, withdrawals}`, `ade_codec::shelley::cert`, and
  `ade_types::conway::cert` are BLUE;
  `ade_ledger::mempool::policy` is GREEN behavior inside the BLUE crate;
  `ade_ledger::consensus_input_extract` is pure-over-bytes "RED behavior"
  inside the BLUE crate; `ade_runtime::consensus` is RED;
  `ade_testkit::{consensus, validity, tx_validity}` is GREEN;
  `ade_core_interop` is RED.
- **`ChainDb` / `SnapshotStore` / `Recoverable` trait shapes** (N-D
  closed): trait method sets frozen.

### Version-gated (can evolve across major versions)

- **New `CardanoEra` variant**: requires new `decode_*_block` chokepoint,
  new per-era composer, new hfc translation arm, addition to
  `CardanoEra::ALL`, extension of the named-chokepoint header in
  `ci_check_ingress_chokepoints.sh`, and the `later_eras` table.
- **New Conway certificate tag** *(B3; B4-extended)*: a new explicit
  `ConwayCert` variant + a `decode_conway_certs` decoder arm (tags ≥19
  currently reject with `UnknownCertTag`) + a `cert_classify::classify` arm
  (incl. its `CoinSource` resolution if accountable) + a conservation-fold arm
  + **(B4)** a `conway_cert_action` arm and an `apply_conway_cert` arm (which
  must declare the cert's owner — B4-owned `CertState` mutation, owner-tagged
  `ConwayGovState` effect, or composite — never `Neutral`). Version-gated per
  protocol; the compiler-exhaustive matches break the build until every arm is
  added (DC-LEDGER-08).
- **New `CoinSource` deposit-provenance** *(B3)*: a fourth source beyond
  explicit-in-cert / deposit-param / registration-state — an explicit
  versioned variant + classifier arm; must remain canonical (DC-TXV-07).
- **Pre-Conway single-tx validity** *(B2 extension point)*: extending
  `decode_tx` to per-era body decode + adding the era arm to
  `required_signers` / `tx_derived_required_signers` (both return
  `UnsupportedEra` for non-Conway today). Requires a per-era
  `SignerSource` grounding + a per-era positive/adversarial corpus.
- **Full-scope `track_utxo=true` tx corpus** *(B2 extension point)*: the
  gating already exists in `tx_phase_one`; completion is corpus + state
  wiring over real/synthetic resolved UTxO, not a new chokepoint.
- **Conway block-body vkey-witness closure** *(B2-carried, post-B3)*:
  wiring `tx_phase_one` / `verify_required_witnesses` into the `rules.rs`
  Conway block-body loop (`project_conway_body_witness_gap`); no new
  composer.
- **Conway governance certificate accumulation authority** *(PHASE4-B5,
  declared)*: a new BLUE governance-cert apply step in `ade_ledger` that
  consumes the owner-tagged effects B4 produces (`ConwayCertOutcome.owner_tagged`,
  each an `OwnerTaggedEffect` of `{ GovernanceOwner, GovernanceCertEffect }`)
  and folds them into `ConwayGovState`, joining the existing
  `ade_ledger::governance::*` ratification/enactment lifecycle. No new
  composer, no new ingress — it attaches at the confirmed B4 owner-tagging
  seam (DC-LEDGER-08).
- **TPraos full-block validity** *(B1 extension point)*: extending
  `block_validity::decode_block` to build `HeaderVrf::Tpraos` for
  pre-Babbage eras.
- **New `GovAction` / Plutus version variant**: registry diff (§3) +
  arms in every chokepoint.
- **New `SignerSource` variant** *(B2)*: an explicit versioned addition —
  requires arms in `required_signers` (+ `tx_derived_*` if UTxO-free),
  the witness-closure source reporting, and a corpus showing coverage.
- **New `TxRejectClass` / `BlockRejectClass` / `FieldKind` /
  `MissingInput` variant**: arms in the relevant `class()` mapping, arms
  in the verdict-surface discriminant maps, and a regenerated
  positive + adversarial corpus.
- **New `OrderPolicy` variant** *(B2)*: a new deterministic permutation
  over the admitted set; must read only the admitted-id list (DC-MEM-02).
- **New protocol parameter field**: append to `ProtocolParameters`; CBOR
  field-order discipline preserved by `ade_codec`. (The Conway-only
  deposit params are the B3 instance — closed shape, per-state value.)
- **New CI check**: additive. Removing a check requires a registry
  deprecation note. (B3 added `ci_check_deposit_param_authority.sh`.)
- **Pinned external crate bump**: Tier-5 rationale doc required.
- **New mini-protocol**: new module with a closed enum, new chokepoint
  pair, new transition, new `*Version` enum. Never an arm on an existing
  enum.
- **Mini-protocol version-table bump**: each `*Version` enum may grow by
  appending a higher variant.
- **New `ChainEvent` / `ChainSelectionReject` / `StreamInput` variant**
  *(N-B)*: bump the envelope version, add encode/decode + dispatch arms,
  regenerate the corpus.
- **New `NetworkMagic`** *(N-B)*: the `parse_genesis` match arm + a new
  boundary table + a normative note.
- **New `LedgerView` impl / LedgerState-backed `PoolDistrView`
  constructor** *(N-B / B1; B4 sync path)*: a slice wiring the impl while
  keeping the trait shape, plus a corpus showing equivalent behavior.
- **`BootstrapAnchorHash` preimage v2** *(N-B)*: hard version-gated.
- **N2N/N2C tx-submission → `mempool::admit` ingress** *(B2 deferred)*:
  the RED bridge from tx-submission opaque-bytes payloads into the
  existing `admit` call; gated by its own cluster doc.
- **Phase-4 cluster surface additions** (N-C, N-E, N-F): each cluster's
  wire surface gates additions via its own cluster doc.

---

## 5. Module Addition Rules

Ade's workspace is small and color-disciplined. New modules enter as new
crates under `crates/`, or as new BLUE submodules within an existing BLUE
crate. `ade_network` is the first BLUE crate with **per-submodule** color
assignment; `ade_runtime` is mixed. B2 added the `tx_validity::*` (all
BLUE) and `mempool::{admit (BLUE), policy (GREEN)}` submodule trees inside
the BLUE `ade_ledger` crate. **B3 added four BLUE submodules entirely
inside existing BLUE crates and added no new crate, no new ingress, and
no new composer**: `ade_codec::conway::{cert, withdrawals}`,
`ade_ledger::cert_classify`, and the `ConwayCert` / `CertDisposition` /
`DepositEffect` / `CoinSource` / `RewardAccount` types in
`ade_types::conway::cert` + `ade_types::tx`. This is the model for
deposit/accounting completeness work: tighten the phase-1 state-backed
authority and its data-only feeders, never add a composer. **B4 followed
the same model — no new crate, no new ingress, no new composer** — adding
the owner-tagged Conway apply types to the existing BLUE `ade_ledger::delegation`
submodule (`ConwayCertAction` / `GovernanceCertEffect` / `GovernanceOwner` /
`OwnerTaggedEffect` / `ConwayCertOutcome` / `ConwayCertEnv`), the
`decode_drep` + shared `read_pool_registration_cert` decoders in `ade_codec`,
and the era-dispatcher `accumulate_tx_certs` in `ade_ledger::rules`; it
enriched `ConwayCert` / `PoolRegistrationCert` in place. **The owner-tagging
boundary is the module-addition rule B4 sets for PHASE4-B5:** the future
governance-cert apply step attaches as a new BLUE step in `ade_ledger` that
consumes `ConwayCertOutcome.owner_tagged`, not as a new composer and not by
mutating `ConwayGovState` from inside B4's cert path.

| Color | Naming convention | Build-config flags | May depend on | MUST NOT depend on |
|-------|-------------------|--------------------|----------------|--------------------|
| **BLUE** | `ade_*` (no color in name) | First line of every `.rs` is the contract banner `// Core Contract:`. `lib.rs` carries `#![deny(unsafe_code)]`, `#![deny(clippy::unwrap_used/expect_used/panic/float_arithmetic)]`. No `#[cfg(feature = ...)]`. No async (DC-CORE-01). No `ChainDb`/`f32`/`f64`/density inside `ade_core::consensus`. No `#[non_exhaustive]`/open-tail/`String`/`Box<dyn>` in `ade_core::consensus`, `ade_ledger::block_validity`, `ade_ledger::tx_validity`, and `ade_ledger::mempool` (B3's closed cert/disposition enums in `ade_types::conway::cert` hold the same shape and are grep-gated as of B3F by `ci_check_conway_cert_classification_closed.sh`). No deposit/refund literal next to a deposit field; no testkit `ConwayGovParams` read (DC-TXV-07). | Other BLUE crates / submodules only (incl. the `ade_ledger → ade_core` edge) | Any RED submodule or crate; GREEN in non-dev deps; `pallas_*` (except `ade_plutus`); async runtime; `HashMap`/`HashSet`/`IndexMap`; clock/rand/float/env/I/O. |
| **GREEN** | `ade_*` | Banner + deny attrs are project convention but not currently enforced for `ade_testkit` / `ade_network::mux::mod` / `ade_ledger::mempool::policy`. May use `HashMap`/`serde_json`/`flate2`/`tar` for fixture I/O (testkit). `ade_runtime::consensus::chain_selector` and `ade_ledger::mempool::policy` are GREEN-behavior but live in BLUE crates for dep convenience. The `ade_testkit` snapshot loader is the one allowlisted source of `conway_deposit_params` (DC-TXV-07). | BLUE crates + standard library + ecosystem crates | `ade_runtime` (for `ade_testkit`); RED submodules in non-test paths. Results must never feed back into a BLUE authoritative decision (policy must never affect `admit`). |
| **RED** | `ade_*` | No special header. Free to use clocks, I/O, async, `HashMap`, signing keys (`ade_runtime` is the only crate that may sign). | Any BLUE / GREEN crate or submodule (one-way) | Cannot be depended on by BLUE (`ci_check_dependency_boundary.sh`, `ci_check_no_async_in_blue.sh`). |

### New module checklist

1. **Add to `Cargo.toml` workspace members.** `version = "0.1.0"`,
   `edition = "2021"`.
2. **Declare TCB color** by editing `.idd-config.json` `core_paths` if
   BLUE; if the crate is mixed-color, name each BLUE submodule path and
   ensure the BLUE CI scripts scan the submodule subset.
3. **CI script update obligations** — extend the relevant BLUE-scoped
   scripts. For closed-taxonomy additions (a new verdict / reject /
   error / outcome family), add the new module path to the `TARGETS` /
   no-`String` file list in `ci_check_consensus_closed_enums.sh` (whose
   set now covers `ade_core::consensus`, `ade_ledger::block_validity`,
   `ade_ledger::tx_validity`, and `ade_ledger::mempool` — **note B3's
   `ade_types::conway::cert` closed enums are NOT in this set** — B3F gave
   them their own grep-gate, `ci_check_conway_cert_classification_closed.sh`,
   so a new cert/disposition surface must extend that check rather than
   this one; **B4's `ade_ledger::delegation` owner-tagged apply types are
   likewise outside that `TARGETS` array** — a new owner-tagged apply surface
   should extend it to `crates/ade_ledger/src/delegation.rs` to grep-gate the
   closed shape). For any new deposit/refund amount source, add the path to the
   `ci_check_deposit_param_authority.sh` scan and route the amount through
   `conway_deposit_view()`. For consensus-shaped additions also extend
   `ci_check_no_chaindb_in_consensus_blue.sh`,
   `ci_check_no_float_in_consensus.sh`, and (if a fork-choice surface is
   touched) `ci_check_no_density_in_fork_choice.sh`.
4. **Add contract banner** (BLUE) to every `.rs` file.
5. **Add deny attributes** to `lib.rs` (BLUE).
6. **New canonical types:** at HEAD the canonical-type registry is inline
   in the invariant registry (`canonical_type_registry: null`) — add a
   `[[rules]]` block under family `T`, plus a round-trip test.
7. **Run `cargo test --workspace` and the full CI script suite.** Both
   must be green before the cluster can close.

### Phase 4 anticipated additions

- **Tx-validity completeness follow-ups**: full `track_utxo=true` corpus;
  pre-Conway eras (extend `decode_tx` + `required_signers`); the Conway
  block-body vkey-witness closure (wire `tx_phase_one` into the `rules.rs`
  block-body loop). The `tx_validity` composer does not change. (B3
  closed the deposit/refund/withdrawal value-conservation follow-up; B4
  closed the Conway cert-state accumulation follow-up.)
- **PHASE4-B5 (Conway governance certificate accumulation authority)**: a
  new BLUE governance-cert apply step in `ade_ledger` that consumes the
  owner-tagged effects B4 produces (`ConwayCertOutcome.owner_tagged`) and
  folds them into `ConwayGovState`. Attaches at the confirmed B4 owner-tagging
  seam; joins the existing `ade_ledger::governance::*` lifecycle. No new
  composer, no new ingress, no new crate (DC-LEDGER-08).
- **N-E (mempool propagation / eviction)**: a Tier-5 `MempoolPolicy`
  trait below the existing `admit` gate, plus the RED N2N/N2C
  tx-submission ingress that calls `admit`. Likely a RED operator shim in
  `ade_runtime` joined to the BLUE `ade_ledger::mempool`.
- **B4 / sync — LedgerState-backed `PoolDistrView`**: a constructor that
  builds `PoolDistrView` from a parsed `LedgerState`. Lives in
  `ade_ledger` (BLUE); keeps the `LedgerView` trait shape.
- **header→body bridge**: the `ade_node` composition layer joining
  `process_stream_input` (header fork-choice) and `block_validity`
  (full-block decision on the fetched body). Likely RED glue.
- **N-C (forge)**: forge-block path likely in `ade_runtime` (RED) for
  KES / VRF signing; must call into `ade_ledger` for canonical
  validation. Reduction target is the existing `BlockEnvelope` chokepoint.
- **N-F (operator API)**: thin RED layer mapping a closed Query enum to
  gRPC/HTTP; shares semantic dispatch with N-A's LSQ / LocalTxMonitor
  opaque-bytes payloads. LocalTxMonitor reads the mempool admitted set.

**These placements are candidates** — user confirmation needed at
cluster entry.

---

## 6. Forbidden Patterns (per color)

### BLUE (universal IDD prohibitions; enforced by CI where marked)

- No `HashMap`, `HashSet`, `IndexMap`, `IndexSet`, `indexmap::*` —
  `ci_check_forbidden_patterns.sh`.
- No `SystemTime`, `Instant`, `std::time::*` clocks —
  `ci_check_forbidden_patterns.sh`.
- No `rand::thread_rng`, `thread::spawn` —
  `ci_check_forbidden_patterns.sh`.
- No `f32`, `f64`, floating-point arithmetic — `#![deny(clippy::float_arithmetic)]`
  plus the pattern script; `ci_check_no_float_in_consensus.sh` narrows
  this to `ade_core::consensus`. (B3's value-conservation arithmetic is
  `i128`-only — no float, no rounding.)
- No `std::fs`, `std::net`, `tokio`, `async fn` —
  `ci_check_forbidden_patterns.sh` + `ci_check_no_async_in_blue.sh`.
- No `anyhow`; `unwrap`/`expect`/`panic` denied at the lint level.
- No `unsafe` outside an explicit allowlist (currently only
  `ade_crypto::vrf`'s FFI binding).
- No `#[cfg(feature = ...)]` semantic gating —
  `ci_check_no_semantic_cfg.sh`.
- No signing patterns in BLUE — `ci_check_no_signing_in_blue.sh`.
- No re-hashing of `canonical_bytes` or re-encoded bytes — wire bytes
  only. `ci_check_hash_uses_wire_bytes.sh`. (B2: `tx_id` is over the
  preserved body slice, never a re-encode.)
- No construction of `PreservedCbor` outside `ade_codec` —
  `ci_check_ingress_chokepoints.sh` Checks 1 & 2.
- No raw CBOR decoding in any BLUE crate except `ade_codec` and the
  single allowlisted file `crates/ade_plutus/src/evaluator.rs` —
  `ci_check_ingress_chokepoints.sh` Check 3. (`tx_validity::decode_tx`,
  `required_signers`, and B3's `decode_conway_certs` / `decode_withdrawals`
  read CBOR via the `ade_codec` primitive set — they never construct
  `PreservedCbor`.)
- No `pallas_*` reference outside `ade_plutus` —
  `ci_check_pallas_quarantine.sh`.
- **(N-A specific)** No `Box<dyn Codec>` / `Box<dyn Protocol>` /
  `#[non_exhaustive]` on mini-protocol message enums; no generic
  `Codec<P>` trait. No reading "selected protocol version" from a session
  global inside a transition (DC-PROTO-06). No decoding block/tx/address
  CBOR inside `ade_network`.
- **(N-B specific)** No `ChainDb` / `chain_db` token inside
  `ade_core::consensus`. No density-based ordering in caught-up Praos
  fork-choice. No `#[non_exhaustive]` / open-tail / `String` / `Box<dyn>`
  in `ade_core::consensus`. No body inspection for fork-choice. No
  stake-snapshot rederivation in BLUE consensus. No plugin-style runtime
  registration of consensus protocols.
- **(B1 specific)** No `#[non_exhaustive]` / open-tail / `String` /
  `Box<dyn>` in `ade_ledger::block_validity`. No `Valid` block verdict
  that skips either authority (DC-VAL-02). No body validation on a
  header-invalid block (DC-VAL-03). No partial mutation on the invalid
  path (DC-VAL-05). No fail-open length/size guard (DC-VAL-06). No
  re-encoding for the block body hash (T-ENC-01). No encoding of the full
  error into the comparison surface — coarse class only. No silent
  fallback on a missing consensus input.
- **(B2 specific)** No `#[non_exhaustive]`, no open-tail `Other` /
  `Unknown`, no owned `String`, no `Box<dyn>` anywhere in
  `ade_ledger::tx_validity` **or `ade_ledger::mempool`** —
  `ci_check_consensus_closed_enums.sh`. Every reject is a structured
  `TxValidityError`; the canonical surface is the coarse `TxRejectClass`
  only.
- **(B2 specific)** No `Valid` tx verdict that skips either phase
  (DC-TXV-02); no phase-2 on a phase-1-failed tx; no partial mutation on
  the invalid path (DC-TXV-04); no nondeterminism (DC-TXV-01); no
  incomplete / silently-omitted required-signer source (DC-TXV-05); no
  fail-open witness check (DC-VAL-06 / CN-LEDGER-09); no re-encoding for
  the tx id (T-ENC-01); no reading `track_utxo=false` as "full validity."
- **(B2 specific — `mempool::admit`)** No false accept — a tx is admitted
  iff `tx_validity(accumulating, tx)` is `Valid` (DC-MEM-01); on Invalid
  the mempool is returned unchanged.
- **(B3 specific — cert grammar; grep-gated in B3F)** No catch-all accept
  arm in `decode_conway_certs` — an unknown tag (≥19) is a hard
  `CodecError::UnknownCertTag`, and tags 5/6 decode to the explicit
  `RemovedInConway` marker, never an accept. **B3F:** trailing bytes after
  the cert array are a hard `CodecError::TrailingBytes` and preallocation
  is bounded (DC-VAL-06). No `#[non_exhaustive]` / open-tail / owned
  `String` / `Box<dyn>` on `ConwayCert` / `CertDisposition` /
  `DepositEffect` / `CoinSource`. These were test-and-review-enforced at
  B3; **B3F added `ci_check_conway_cert_classification_closed.sh`** which
  grep-gates the closed value types, the no-catch-all decoder, and the
  exhaustive `classify` (DC-TXV-06 partial→enforced; §3 gap note RESOLVED).
- **(B3 specific — withdrawals grammar)** No last-wins on a repeated
  withdrawals-map key — a duplicate `RewardAccount` is a hard
  `CodecError::DuplicateMapKey`. `withdrawals_sum` only ever runs over a
  fully-deduplicated map.
- **(B3 specific — deposit-param authority)** No deposit/refund literal
  next to a deposit field; no read of a testkit `ConwayGovParams`. Every
  `key_deposit` / `pool_deposit` / `drep_deposit` / `gov_action_deposit`
  flows from `conway_deposit_view()` (DC-TXV-07,
  `ci_check_deposit_param_authority.sh`). `conway_deposit_view()` is
  `Some` iff Conway and fails fast with `MissingConwayDepositParams`.
- **(B3 specific — cert classification)** No guessed state-dependent
  deposit/refund — `cert_classify::classify` is total and closed over
  `ConwayCert`; an unresolvable refund/deposit is the structured
  `UnsupportedStateDependentDepositAccounting` reject, never a fabricated
  amount and never the `key_deposit` param (DC-TXV-06).
- **(B3 specific — conservation)** No accept of a cert/withdrawal-bearing
  tx without the full value check — the B2 early-out is removed; the full
  `consumed == produced` equation runs for every Conway tx, with the
  frozen §9.1 reject precedence (decode → era-validity →
  missing-environment → state-dependent-accounting → conservation), no
  later check masking an earlier one (T-CONSERV-01 / CN-LEDGER-07
  strengthened; DC-VAL-06 strengthened).
- **(B3 specific — fingerprint)** No fingerprint drift for non-Conway
  states — the `CONWAY_DEPOSIT_PARAMS_TAG` fold is byte-identical when
  `conway_deposit_params == None` (DC-LEDGER-01).
- **(B4 specific — owner-tagged cert-state apply)** No reduction of a
  `ConwayCert` into the 7-variant Shelley `Certificate`; no flattening of any
  cert to neutral (there is no `Neutral` action — every defined Conway tag has
  an owner); no dropping of owner payloads; no swallowing of a decode or apply
  error. Governance-affecting certs are owner-tagged to `ConwayGovState`
  (`ConwayCertOutcome.owner_tagged`) and routed out of B4's mutation scope —
  observed, not applied, not neutralized, not swallowed; removed tags 5/6
  reject with `LedgerError::EraInvalidCertificate` (DC-LEDGER-08).
- **(B4 specific — era dispatch / fail-closed accumulation)** No `_era`
  discard and no fail-open swallow in `accumulate_tx_certs` /
  `process_block_certificates` — the prior "non-fatal during replay" swallows
  are removed; a decode or apply error propagates as a structured
  `LedgerError` and halts the block transition. Conway bytes must dispatch to
  the Conway decoder (`decode_conway_certs`), never the Shelley 6-variant
  decoder (DC-LEDGER-08).
- **(B4 specific — no parallel decoder)** No second pool-params decoder —
  `read_pool_registration_cert` (in `ade_codec::shelley::cert`) is the ONE
  pool-params decode site for both era cert decoders; `decode_drep` is closed
  (no catch-all). A new era-specific copy of either is forbidden (DC-LEDGER-08).

### GREEN (`ade_testkit` incl. `validity` / `tx_validity` + the B3 conservation corpora + the B4 cert-state corpus, `ade_network::lib` / `mux::mod`, `ade_runtime::consensus::{candidate_fragment, chain_selector}`, `ade_ledger::mempool::policy`)

- No nondeterminism that leaks into stored fixtures — fixtures must be
  byte-reproducible (the block-validity, tx-validity, B3 conservation, and
  B4 cert-state corpora replay identically — `cert_state_replay_byte_identical`).
- No participation in authoritative outputs. The B1/B2/B3/B4 validity
  harnesses only *drive* `block_validity` / `tx_validity` /
  `check_conway_coin_conservation` / `apply_conway_cert` (via
  `accumulate_tx_certs`) and assert; the mutators are deterministic
  transforms over real or synthetic corpus blocks/txs/certs.
- No `HashMap` even in test helpers — `BTreeMap` only.
- No import of `ade_runtime` from `ade_testkit`.
- No inbound dep from any RED crate (for `ade_testkit` /
  `ade_network::lib` / `mux::mod`).
- (`ade_runtime::consensus::chain_selector` specifically) No comparison
  decision; defer to BLUE.
- **(`ade_ledger::mempool::policy` specifically — B2)** No call to
  `tx_validity`; no read of the accumulating state; no add/remove of a tx
  id. `order` is a pure deterministic PERMUTATION over the admitted-id
  list and cannot change which txs `admit` accepted (DC-MEM-02). Tier-5
  is provably below Tier-1.
- **(`ade_testkit` snapshot loader specifically — B3)** The deposit-param
  construction (`conway_deposit_params` from parsed snapshot bytes) is the
  ONE allowlisted non-canonical-state source; it must never be reached by
  the BLUE deposit-conservation path at runtime — it is a
  fixture-materialization helper only (`ci_check_deposit_param_authority.sh`
  allowlists it precisely so the BLUE crates carry no deposit literals).

### RED (`ade_runtime`, `ade_node`, `ade_network::mux::transport`, `ade_network::session`, `ade_network::bin::capture_*`, `ade_runtime::consensus::genesis_parser`, `ade_core_interop`, and the RED-behavior `ade_ledger::consensus_input_extract` scan)

- No direct mutation of `ade_ledger` state — all transitions go through
  `ade_ledger::rules::*`, the `block_validity` / `tx_validity` composers,
  or `mempool::admit`.
- No bypassing `ade_codec` to construct semantic types from raw bytes.
- (`ade_runtime` specifically) No dep on `ade_ledger` — bytes-in /
  bytes-out only (S-36). No leakage of `redb` types through `chaindb::*`
  (S-34). No second public `chaindb` path. No automatic snapshot pruning.
  No partial-recovery success. No async recovery surface.
- (`ade_network::mux::transport`) No protocol logic; bearer I/O only.
- (`ade_network::session`) Composition glue only.
- (`ade_network::bin::capture_*`) Live-interop tools only; never linked
  into the node binary.
- (`ade_runtime::consensus::genesis_parser`) No re-derivation of the
  bootstrap anchor outside `compute_anchor_hash`; no BLUE re-consumption
  of the JSON bytes.
- (`ade_ledger::consensus_input_extract`) The nonce tail-scan parses an
  external dump format (RED behavior) but stays pure-over-bytes and
  fail-closed; never gains I/O, a clock, or a heuristic "best-effort"
  nonce pick.
- (future N2N/N2C tx-submission ingress — candidate) When wired, the RED
  bridge must call `mempool::admit(mempool, tx_cbor)` — it must NOT carry
  a parallel admission path or any validity decision of its own.
- (`ade_core_interop`) Live-interop driver only; tests `#[ignore]`-gated.

### Project-specific additions

- **No commits of credentials, hostnames, IPs, private keys** —
  public-repo discipline; enforced by `ci_check_no_secrets.sh`.
- **No `Phase 4 internal-mode mock network`** — Tier 1 surfaces must be
  exercised against real cardano-node peers. B1's positive corpus is real
  on-chain Conway-576 blocks; B2's positive tx corpus is the real on-wire
  Conway txs extracted from those same blocks; **B3's positive
  value-conservation corpus is the same real Conway-576 txs run through
  the full equation**, and the adversarial corpus is mutator-derived.
- **No collapsing wire and canonical bytes** — dual-authority rule. B3 is
  a textbook instance: the codec says what the cert/withdrawal bytes are;
  `cert_classify` + `check_conway_coin_conservation` say whether they
  balance.
- **No Tier 5 surface without a stated rationale** — divergence from
  cardano-node requires naming "what's better" per
  `docs/active/CE-79_tier5_addendum.md`. The mempool `policy` layer is the
  newest Tier-5 surface and must stay below the Tier-1 `admit` gate.
- **No "we'll match it later" stubs on Tier 1 surfaces** — Tier 1
  closure is hard-gated. The B1 block verdict, the B2 tx verdict, the B2
  mempool admission gate, the B3 full value-conservation accounting, and the
  B4 Conway cert-state accumulation are all Tier-1 surfaces. (B4's one
  remaining obligation — the real epoch-576 cert-state oracle — is
  environment-blocked by an absent UMap snapshot, not a stub; B4 closes
  mechanically with the synthetic positive/replay/adversarial corpus.)

---

## Cross-references

- CODEMAP: `docs/ade-CODEMAP.md` — module-by-module authority table,
  upstream of this document. **Cross-reference check:** CODEMAP was
  regenerated at HEAD (`ee35493`) and its narrative folds in PHASE4-B4 — the
  owner-complete `ConwayCert`, the enriched `PoolRegistrationCert.owners`, the
  `ade_codec::conway::cert::decode_drep` + shared
  `ade_codec::shelley::cert::read_pool_registration_cert` decoders, the
  `ade_ledger::delegation` owner-tagged apply model (`apply_conway_cert` /
  `conway_cert_action` + the six new types), and the era-dispatched
  `ade_ledger::rules::accumulate_tx_certs` all appear in its entries; it
  records 375 canonical types (+6 from B4, all in `ade_ledger::delegation`),
  1285 tests (+17 from B4), and 27 CI checks (unchanged — B4 added no gate).
  Both docs agree that DC-LEDGER-08 reuses `ci_check_forbidden_patterns.sh`
  and that the `ade_ledger::delegation` owner-tagged types are NOT yet in the
  `ci_check_consensus_closed_enums.sh` `TARGETS` array (the narrow B4 gap
  surfaced in §3 here and in CODEMAP's PHASE4-B4 note). The two docs are
  consistent at this SHA.
- Invariant registry: `docs/ade-invariant-registry.toml` — rule families
  incl. `T`, `CN`, `DC` (with `DC-PROTO-*` + `DC-CORE-01` under N-A,
  `DC-CONS-03..10` under N-B, `DC-VAL-01..06` under B1,
  `DC-TXV-01..05` + `DC-MEM-01/02` under B2, **`DC-TXV-06` +
  `DC-TXV-07` under B3** (`DC-TXV-06` moved partial→enforced in B3F via
  `ci_check_conway_cert_classification_closed.sh`), and **`DC-LEDGER-08`
  under B4** (`status=enforced`, `ci_script` = `ci_check_forbidden_patterns.sh`);
  `T-CONSERV-01` / `CN-LEDGER-07` / `DC-VAL-06` strengthened in B3 —
  `DC-VAL-06` further reinforced in B3F by the cert decoder's trailing-byte
  rejection + bounded preallocation), `OP`, `RO`.
- Phase 4 cluster plan: `docs/active/phase_4_cluster_plan.md`.
- Tier doctrine: `docs/active/CE-79_gate_statement.md` and
  `docs/active/CE-79_tier5_addendum.md`.
- Cluster N-D slices (closed):
  `docs/clusters/completed/PHASE4-N-D/S-{33..37}.md`.
- Cluster N-A (closed): `docs/clusters/completed/PHASE4-N-A/cluster.md`
  + `S-A{1..10}.md`.
- Cluster N-B (closed): `docs/clusters/PHASE4-N-B/cluster.md` +
  `S-B{1..10}.md`.
- Cluster B1 (closed): `docs/clusters/PHASE4-B1/cluster.md` +
  `B1-S{1..7}.md`.
- Cluster B2 (closed): `docs/clusters/PHASE4-B2/cluster.md` +
  `B2-S{1..5}.md`.
- Cluster B3 (closed): `docs/clusters/PHASE4-B3/cluster.md` +
  `B3-S{1..6}.md`.
- Cluster B4 (closed): `docs/clusters/PHASE4-B4/cluster.md` +
  `B4-S1.md` (declares PHASE4-B5).
- N-A live-interop evidence: `docs/active/CE-N-A-5_evidence.toml`.
