# Seams — Where New Work Can Attach (Ade)

> **Status:** Living architectural document. Regenerated; not hand-edited.
> Per-project instance of `~/.claude/methodology/templates/seams.md`.

> 11 crates, 31 CI checks at HEAD (`43fcc31`).
> Reads CODEMAP for the module list and TCB colors; reads the invariant
> registry (`docs/ade-invariant-registry.toml`) for rule IDs; reads the
> Phase 4 cluster plan (`docs/active/phase_4_cluster_plan.md`), the
> closed N-D / N-A / N-B / B1 / B2 / B3 / B4 / B5 cluster docs, the
> OQ5-CREDENTIAL-FIDELITY, COMMITTEE-CRED-FIDELITY, DREP-VOTE-FIDELITY,
> ENACTMENT-COMMITTEE-FIDELITY, ENACTMENT-COMMITTEE-WRITEBACK cluster
> docs, and the **in-flight PHASE4-N-E cluster doc** (`docs/clusters/PHASE4-N-E/cluster.md`
> + slices S1..S5; cluster mechanically closed at HEAD but not yet
> archived to `docs/clusters/completed/`).
>
> **This is a PHASE4-N-E close refresh (HEAD `43fcc31`).** Five
> slices land between the prior SEAMS HEAD (`168ac02`) and this HEAD;
> together they wire and close the **Tier 1 wire-level mempool ingress
> seam** — the prior revision's single most load-bearing §1 candidate.
> Inventory:
>
> - `32c1ee6` — **N-E-S1** ships the closed `IngressEvent` + the BLUE
>   chokepoint `mempool_ingress` in the new BLUE module
>   `ade_ledger::mempool::ingress` (`crates/ade_ledger/src/mempool/ingress.rs`),
>   plus the new CI gate `ci/ci_check_mempool_ingress_closure.sh`.
>   Registers `DC-MEM-03` in the invariant registry.
> - `2d0c918` — **N-E-S2** ships the GREEN ingress-replay harness in the
>   new module `ade_testkit::mempool` (`crates/ade_testkit/src/mempool/{mod.rs,ingress_replay.rs}`)
>   plus the new CI gate `ci/ci_check_mempool_ingress_replay.sh`. Wraps
>   the existing B-track adversarial corpus in synthetic `IngressEvent`s.
>   Registers `DC-MEM-04` in the invariant registry; appends
>   `PHASE4-N-E` to `DC-MEM-01.strengthened_in`.
> - `509d714` — **N-E-S3** ships the GREEN per-peer canonicalizer
>   `canonicalize_peer_streams` in the new module
>   `ade_ledger::mempool::canonicalize` (`crates/ade_ledger/src/mempool/canonicalize.rs`).
>   Extends `ci/ci_check_mempool_ingress_replay.sh` with three further
>   checks (existence + closed import shape + no async / RNG / clock
>   / `HashMap` / `HashSet` in the canonicalizer body).
> - `ca3f23a` — **N-E-S4** ships the GREEN N2N tx-submission2 bridge
>   `ade_core_interop::tx_submission` (`crates/ade_core_interop/src/tx_submission.rs`):
>   `InventoryEvent → IngressEvent` adapter + per-peer `PeerAccumulator`
>   + the orchestrator `ingest_n2n_events`, all pure functions of the
>   inputs. Documents the operator procedure in
>   `docs/clusters/PHASE4-N-E/CE-N-E-6_PROCEDURE.md`.
> - `43fcc31` (THIS HEAD) — **N-E-S5** ships the GREEN N2C
>   local-tx-submission bridge `ade_core_interop::local_tx_submission`
>   (`crates/ade_core_interop/src/local_tx_submission.rs`):
>   `LocalTxSubmissionEvent → IngressEvent` adapter + per-client
>   `ClientAccumulator` + the orchestrator `ingest_n2c_events`. Documents
>   the operator procedure in `docs/clusters/PHASE4-N-E/CE-N-E-7_PROCEDURE.md`.
>
> **THE KEY N-E DELTA:** the prior revision's single most load-bearing
> §1 candidate seam — "N2N/N2C tx-submission ingest → `mempool::admit`"
> — is now **WIRED AND CLOSED on the code half**. The closed
> 2-variant `IngressSource { N2N, N2C }` + the closed `IngressEvent
> { source, tx_bytes }` struct + the **single BLUE chokepoint**
> `mempool_ingress(&MempoolState, &IngressEvent) -> (MempoolState,
> AdmitOutcome)` are the production path; `mempool_ingress` is a
> **thin pass-through to `admit` over `event.tx_bytes()`** (the
> `source` variant is metadata only and the CI gate forbids the
> chokepoint body from reading it). The two RED ingress transports (N2N
> tx-submission2 inventory events, N2C local-tx-submission events) feed
> through deterministic GREEN bridges (`ade_core_interop::tx_submission`
> and `ade_core_interop::local_tx_submission`) into the GREEN per-peer
> canonicalizer (`ade_ledger::mempool::canonicalize::canonicalize_peer_streams`,
> round-robin by sorted `PeerId` with explicit source-byte tie-break)
> and then into `mempool_ingress`. The closure is mechanically defended
> by **two new CI gates**: `ci_check_mempool_ingress_closure.sh`
> (DC-MEM-03 — `IngressSource` closed 2-variant; `MempoolState.accumulating`
> field-write is forbidden outside `mempool/admit.rs`; `admit()` callers
> outside `mempool/admit.rs` definition + `mempool/ingress.rs` bridge in
> production source are forbidden; `mempool_ingress` body must not
> reference `source`) and `ci_check_mempool_ingress_replay.sh`
> (DC-MEM-04 — the GREEN replay harness must call `mempool_ingress` and
> never direct `admit`; the five named integration tests must exist;
> no batching/out-of-order helpers; the canonicalizer body must carry
> no async / RNG / clock / `HashMap` / `HashSet` / `RwLock` / `Mutex`).
> **CE-N-E-6 and CE-N-E-7 are operator-action evidence-log artifacts**
> (the live N2N + N2C wire-evidence logs at
> `docs/clusters/PHASE4-N-E/CE-N-E-{6,7}_<date>.log`), parallel to the
> CE-N-B-6 live tip-agreement pattern — the **code + GREEN evidence is
> CI-green at this HEAD**; the live-log artifacts are operator-action
> (see "Operator-action evidence" pattern below). **N-E ADDED TWO NEW
> CI SCRIPTS** (CI count `29 → 31`), **two new BLUE / GREEN modules**
> inside the existing BLUE `ade_ledger` crate (`mempool::ingress` BLUE
> + `mempool::canonicalize` GREEN) + the new GREEN `ade_testkit::mempool`
> module + the two new GREEN bridges in `ade_core_interop`; **three new
> closed surfaces** (`IngressSource`, `IngressEvent`, `mempool_ingress`);
> **two new derived rules** (`DC-MEM-03`, `DC-MEM-04`) and a
> `strengthened_in += PHASE4-N-E` on `DC-MEM-01`. It introduces **no
> new external wire-format frozen contract** — the tx bytes flow
> verbatim end-to-end and the verdict is `tx_validity`'s verdict
> (DC-MEM-01 unchanged, now strengthened with the source-invariance
> property).
>
> **Per-peer canonicalizer is the load-bearing GREEN fairness contract**
> (round-robin by sorted `PeerId`, with **single-byte source tie-break**
> N2N=0 / N2C=1 — stable across binary builds). Any future change to
> this ordering — fairness policy, batching, parallelization, or
> alternative tie-breaks — is a **SEAMS-level change** because the
> GREEN replay-byte-identity property (DC-MEM-04) and the multi-peer
> interleaving test (`multi_peer_round_robin_by_sorted_peer_id` +
> `unsorted_input_canonicalizes_identically_to_sorted_input` +
> `same_peer_id_same_source_stable_ordering`) pivot on this exact rule.
> The canonicalizer is currently the only GREEN module under the BLUE
> `ade_ledger` crate other than `mempool::policy` (which sits under the
> same Tier-5 doctrine), and the only one whose output sequencing is
> consumed by a BLUE chokepoint that produces `(MempoolState,
> AdmitOutcome)` traces.
>
> **No section below has its module list or rule list shrunk by this
> refresh — N-E is purely additive.** The counts the body reports
> (canonical types unchanged at 376; tests grew with the new
> ingress/canonicalize/bridge integration tests; CI scripts 29 → 31)
> match CODEMAP at this HEAD. The body's prior text on §1 (surface
> reduction), §2 (data-only / authoritative layers), §3 (closed /
> extensible registries), §4 (frozen / version-gated contracts), §5
> (module addition rules), and §6 (forbidden patterns) is preserved
> wherever N-E did not touch it; the load-bearing additions are:
>
> - **§1** — a new wired surface "Mempool ingress (Tier-1 wire-level —
>   wired in N-E)" + the candidate-table row for N2N/N2C tx-submission
>   flipped to **wired & closed**.
> - **§2** — a new authoritative-domain block "Mempool ingress —
>   the Tier-1 wire-level / per-peer canonicalizer / `mempool_ingress`
>   boundary (N-E)" alongside the existing "Mempool admission" block.
> - **§3** — three new closed surfaces in the Closed table
>   (`IngressSource`, `IngressEvent`, `mempool_ingress`); the
>   `MempoolState.accumulating` write-closure rule moved from "by
>   convention" to grep-gated.
> - **§4** — three new frozen contracts (the `mempool_ingress`
>   chokepoint rule; `IngressSource` source-invariance / N-E-N7+N-E-8;
>   verbatim tx-bytes flow through ingress without decode/re-encode);
>   no new version-gated contract.
> - **§5** — the per-peer canonicalizer / GREEN bridge / RED operator
>   live-session pattern as the load-bearing N-E module-addition shape.
> - **§6** — explicit BLUE-bridge / GREEN-bridge / RED-driver
>   prohibitions for the new ingress path.
>
> Cross-reference at this HEAD: CODEMAP is being regenerated in
> parallel; the in-flight CODEMAP may still pin pre-N-E HEAD
> (`168ac02`) at the moment of this regen (it will catch up on the
> next refresh). The narrative below names the exact N-E source-file
> paths so any follow-up regen can find them mechanically.
>
> **(Prior context — ENACTMENT-COMMITTEE-WRITEBACK close, HEAD `3180e27`.)**
> The body was fully regenerated at PHASE4-B3 close (`7784bf8`), folded in
> the B3F hardening deltas (`193d2fc`), the PHASE4-B4 deltas (`ee35493`),
> the PHASE4-B5 deltas (`644eb03`), the OQ5-CREDENTIAL-FIDELITY deltas
> (`a3ee2da`), the COMMITTEE-CRED-FIDELITY deltas (`2aeea16`), the
> DREP-VOTE-FIDELITY deltas (`62c9020`), the ENACTMENT-COMMITTEE-FIDELITY
> delta (`a6b8de7`), the ENACTMENT-COMMITTEE-WRITEBACK deltas (S1
> `f2f15f9`, S2 `3180e27`), and the snapshot-loader follow-ups (`168ac02`).
> **THE PRIOR-REFRESH KEY DELTA:** the previously-dormant `UpdateCommittee`
> enactment LOGIC — the prior revision's last remaining open
> governance-enactment seam, where the `enact_proposals` arm was literally
> `let _ = raw;` — is WIRED and CLOSED. `GovAction::UpdateCommittee` (in
> `ade_types::conway::governance`) moved from the opaque
> `{ prev_action, raw: Vec<u8> }` to the **closed structured variant**
> `{ prev_action, removed: BTreeSet<StakeCredential>, added:
> BTreeMap<StakeCredential, u64>, threshold: (u64, u64) }` (the
> `GovAction` enum **cardinality is unchanged — still a closed 7-variant
> enum**). Committee write-back is the closed pure transition
> `ade_ledger::governance::apply_committee_enactment`, called at the
> `ade_ledger::rules` epoch-boundary apply site (`rules.rs:1224`); the
> snapshot-loader decode (GREEN) carries fail-closed
> `parse_cold_credential` / `parse_cold_credential_set` /
> `parse_cold_credential_epoch_map` / `parse_unit_interval`. **The remaining
> open governance-domain seam** is the declared non-goal
> `proposal_procedures` tx-body decode (the wire codec keeps
> `proposal_procedures` opaque `Option<Vec<u8>>`).

Ade is a Cardano block-producing node. Its closure surface is dominated
by two facts:

1. The Cardano protocol fixes wire bytes and hashes for hash-critical
   paths (Tier 1 — must-conform). New work that touches those bytes
   has essentially no degrees of freedom.
2. Everything operator-facing — storage layout, query API, telemetry,
   packaging — is Tier 5: deliberate divergence "in our own image"
   (per `docs/active/CE-79_tier5_addendum.md`).

This document names where the system opens and where it stays closed.

**PHASE4-N-E (Tier 1 wire-level mempool ingress) just closed (code +
GREEN evidence).** It closed the prior revision's most load-bearing
§1 candidate — the N2N/N2C tx-submission bridge into `mempool::admit`.
It added: a **closed 2-variant `IngressSource { N2N, N2C }`** in
`ade_ledger::mempool::ingress`; the **closed `IngressEvent { source,
tx_bytes }` struct** carrying tx bytes verbatim (PreservedCbor
end-to-end through the ingress layer — no decode, no re-encode); the
**single BLUE chokepoint `mempool_ingress`** which is the only
sanctioned production path into `admit` (CI-enforced — direct `admit`
callers outside `mempool/admit.rs` definition and `mempool/ingress.rs`
bridge are forbidden in production source); the **GREEN per-peer
canonicalizer `canonicalize_peer_streams`** in `ade_ledger::mempool::canonicalize`
(round-robin by sorted `PeerId` with explicit source-byte tie-break —
the load-bearing GREEN fairness contract); the **GREEN
`ade_testkit::mempool::ingress_replay`** harness (wraps the existing
B-track adversarial corpus in synthetic `IngressEvent`s and replays
via `mempool_ingress` — single-step fold per OQ-6); the **GREEN
`ade_core_interop::tx_submission` N2N bridge** (`InventoryEvent →
IngressEvent` adapter + `PeerAccumulator` + `ingest_n2n_events`); the
**GREEN `ade_core_interop::local_tx_submission` N2C bridge**
(`LocalTxSubmissionEvent → IngressEvent` adapter + `ClientAccumulator`
+ `ingest_n2c_events`); **two new CI gates**
(`ci_check_mempool_ingress_closure.sh` for DC-MEM-03,
`ci_check_mempool_ingress_replay.sh` for DC-MEM-04); and two new
derived rules (`DC-MEM-03`, `DC-MEM-04`) plus a strengthening of
`DC-MEM-01` (`strengthened_in += PHASE4-N-E`). **N-E added no new
crate and no new external ingress wire-format frozen contract** — the
tx-submission2 / local-tx-submission wire grammars (N-A) are
unchanged; only the bridge from their delivered payloads into the
Tier-1 admission gate is new. The verdict equation is unchanged
(`mempool_ingress(s, evt) == admit(s, evt.tx_bytes())`, which is
`tx_validity`'s verdict) — N-E adds **source-invariance** as a
first-class property (N-E-N7 / N-E-8): two events differing only in
`source` produce byte-identical outcomes. **The remaining open
ingress-side seams** (declared non-goals at the N-E cluster doc) are
outbound tx propagation (Ade as a tx source), mempool bounds /
shedding policy (Tier-5: `CN-MEM-01`, `CN-MEM-03`, `DC-MEM-02`
strengthening), and the carried `proposal_procedures` tx-body decode
into `GovAction`.

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
> the new N-E wire-level mempool ingress**), plus the three **internal
> composition roots** (`block_validity` from B1, `tx_validity` from B2,
> and the new BLUE chokepoint `mempool_ingress` from N-E), the **mempool
> admission gate** (`mempool::admit`, a Tier-1 surface over
> `tx_validity`), the **consensus-input extraction surface** (snapshot
> `state` CBOR tail-scan from B1), plus the remaining surfaces named in
> the Phase 4 plan (forge, query API, outbound tx propagation).
>
> **N-E added one new external ingress seam — and CLOSED it.** The
> N2N/N2C tx-submission ingest into `mempool::admit` — the prior
> revision's single most load-bearing §1 candidate — is now WIRED end-to-end
> on the code path: RED wire transport (`ade_network::mux::transport`)
> → BLUE wire grammar (`ade_network::tx_submission` /
> `ade_network::n2c::local_tx_submission` codecs + state machines,
> closed in N-A) → GREEN bridge in `ade_core_interop` (per-peer or
> per-client accumulator over the protocol's delivered events) →
> GREEN per-peer canonicalizer
> (`ade_ledger::mempool::canonicalize::canonicalize_peer_streams`,
> round-robin by sorted `PeerId` with single-byte source tie-break) →
> **BLUE chokepoint `ade_ledger::mempool::ingress::mempool_ingress`**
> → BLUE `admit` → `tx_validity`. Tx bytes flow verbatim end-to-end
> (no decode, no re-encode anywhere on the ingress path); the
> `IngressSource` variant is metadata only and **MUST NOT** change the
> verdict (CI-enforced — N-E-N7 / N-E-8). The two new CI gates
> (`ci_check_mempool_ingress_closure.sh`,
> `ci_check_mempool_ingress_replay.sh`) defend the closure +
> single-step replay properties. **CE-N-E-6 / CE-N-E-7 LIVE EVIDENCE
> LOGS** (real cardano-node N2N peer / real `cardano-cli` over the
> N2C UDS) are operator-action artifacts pending capture; the
> code + GREEN evidence is CI-green at this HEAD (the CE-N-B-6 pattern).
>
> **B3 added no new ingress surface.** The Conway cert array and
> withdrawals map are sub-grammars *inside* the already-existing
> standalone Conway tx CBOR surface and the block-body surface.
>
> **B4/B5/OQ5/COMMITTEE/DREP/ENACTMENT-COMMITTEE-FIDELITY/WRITEBACK
> added no new external ingress surface.** Each was a type-fidelity or
> apply-step tightening over the existing block-body / Conway-tx-body
> / gov-state / fingerprint surfaces.

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
         — per-peer `PeerAccumulator::observe` collects every
           InventoryEvent::TxsDelivered { tx_bytes } into a per-peer
           FIFO; drains to PeerSubmissionQueue { peer, source: N2N, txs };
           passes through the canonicalizer; replays via replay_ingress_trace.
       N2C: ade_core_interop::local_tx_submission::ingest_n2c_events(
             base, per_client: &[(PeerId, Vec<LocalTxSubmissionEvent>)]
           ) -> (MempoolState, Vec<AdmitOutcome>)
         — per-client `ClientAccumulator::observe` collects every
           LocalTxSubmissionEvent::TxSubmitted { tx_bytes }; drains to
           PeerSubmissionQueue { peer, source: N2C, txs }; same canonicalize + replay.
  4. GREEN canonicalizer (N-E S3; deterministic, no I/O)
       ade_ledger::mempool::canonicalize::canonicalize_peer_streams(
         &[PeerSubmissionQueue]
       ) -> Vec<IngressEvent>
         — round-robin by sorted `PeerId` byte-lex; tie-break by single-byte
           source tag (N2N=0, N2C=1); round[i] emits one tx from every
           queue whose `.txs.get(i)` is `Some`. Pure function of inputs;
           output independent of input iteration order (the function
           sorts internally). The load-bearing GREEN fairness contract
           (any change is SEAMS-level).
  5. BLUE chokepoint (N-E S1)
       ade_ledger::mempool::ingress::mempool_ingress(
         &MempoolState, &IngressEvent
       ) -> (MempoolState, AdmitOutcome)
         — pure pass-through to `admit` over `event.tx_bytes()`. The
           `event.source` is RECORDED for evidence/policy/replay but
           MUST NOT affect the verdict — CI-enforced (the body of
           `mempool_ingress` may not reference `source`; N-E-N7/N-E-8).
  6. BLUE admission gate (B2 — unchanged)
       admit(mempool, tx_cbor) -> (MempoolState, AdmitOutcome)
         — verdict equals `tx_validity(accumulating, tx)`; no false
           accept (DC-MEM-01).
Cross-surface state sharing: the mempool's `accumulating` LedgerState
  is the only state carried across consecutive `mempool_ingress` calls;
  it is the same shape `admit` consumes, threaded by value.
```

**Rule.** Mempool ingress is the **Tier-1 wire-level seam**:
every production tx-bytes path into `admit` MUST go through
`mempool_ingress`. The single BLUE chokepoint, the closed
2-variant `IngressSource`, and the verbatim flow of `tx_bytes` (no
decode, no re-encode) are the load-bearing properties (DC-MEM-03).
The `IngressSource` variant is **metadata only**: it lives on the
event so evidence/policy/replay can record which transport carried
the tx, but the verdict equation `mempool_ingress(state, evt) ==
admit(state, evt.tx_bytes())` does not read it (source-invariance,
N-E-N7 / N-E-8). New ingress transports attach by **producing
`IngressEvent`s and feeding them into `mempool_ingress`** — not by
adding a parallel admission path, not by adding a verdict-side
match on `source`, not by mutating `MempoolState.accumulating`
directly (CI-forbidden outside `mempool/admit.rs`). The replay
contract (DC-MEM-04) is a **single-step fold** over
`mempool_ingress`: replaying the same ordered `IngressEvent` list
against the same base ledger state must produce a byte-identical
`(MempoolState, Vec<AdmitOutcome>)` pair; batching, parallel
folding, or out-of-order interleaving on the harness side is
CI-forbidden (the gate scans the harness body for
`chunks`/`partition`/`par_iter`/`rayon`/`tokio::spawn`). **Operator
action**: the live wire evidence (real cardano-node peer over N2N
tx-submission2; real `cardano-cli` over N2C UDS) is captured into
`docs/clusters/PHASE4-N-E/CE-N-E-{6,7}_<date>.log`, mirroring the
CE-N-B-6 pattern.

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
`local-tx-submission` (N2C) protocols' delivered tx bytes are now WIRED
into `mempool::admit` via the new `mempool_ingress` chokepoint (the
prior revision's candidate seam closed). The bridge is GREEN
(`ade_core_interop::tx_submission` + `ade_core_interop::local_tx_submission`)
+ the operator-action live session (RED bearer + RED transport).

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
- **N-E (THIS REFRESH) WIRED AND CLOSED the prior revision's most
  load-bearing §1 candidate** — N2N/N2C tx-submission ingest into
  `mempool::admit`. The row below is recorded as **wired & closed**
  with a pointer to PHASE4-N-E.

| Cluster | Surface | Expected reduction target | Expected chokepoint | Confidence |
|---------|---------|---------------------------|---------------------|------------|
| **PHASE4-N-E** *(WIRED + CLOSED — code half; live-evidence logs operator-action)* | **N2N/N2C tx-submission → mempool ingress (Tier-1 wire-level)** — the RED ingress that delivers a candidate tx from the `tx-submission2` (N2N) or `local-tx-submission` (N2C) opaque-bytes payload into the Tier-1 gate | `mempool_ingress(&MempoolState, &IngressEvent) -> (MempoolState, AdmitOutcome)`, where `IngressEvent { source: IngressSource::{N2N,N2C}, tx_bytes }` flows verbatim into `admit` | **DONE:** GREEN bridges `ade_core_interop::tx_submission::ingest_n2n_events` (S4, `InventoryEvent → IngressEvent`) and `ade_core_interop::local_tx_submission::ingest_n2c_events` (S5, `LocalTxSubmissionEvent → IngressEvent`) + GREEN canonicalizer `ade_ledger::mempool::canonicalize::canonicalize_peer_streams` (S3, round-robin by sorted `PeerId` with source-byte tie-break) + the BLUE chokepoint `ade_ledger::mempool::ingress::mempool_ingress` (S1). Gated by `ci_check_mempool_ingress_closure.sh` (DC-MEM-03) + `ci_check_mempool_ingress_replay.sh` (DC-MEM-04); `DC-MEM-01.strengthened_in += PHASE4-N-E`. **Live N2N/N2C log artifacts pending operator action** (CE-N-E-6 / CE-N-E-7 — CE-N-B-6 pattern; code + GREEN evidence CI-green at HEAD) | **wired & closed in N-E** (was the B+/N-E top candidate; code + GREEN evidence in; live wire-evidence logs operator-action) |
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

| Procedure | Evidence-log artifact | What it asserts | TCB |
|-----------|----------------------|------------------|-----|
| `docs/clusters/PHASE4-N-B/CE-N-B-6_PROCEDURE.md` (pre-existing) | `docs/clusters/PHASE4-N-B/CE-N-B-6_<date>.log` | Real cardano-node N-B follow-mode tip agreement | RED operator action |
| `docs/clusters/PHASE4-N-E/CE-N-E-6_PROCEDURE.md` (NEW in N-E) | `docs/clusters/PHASE4-N-E/CE-N-E-6_<date>.log` | Real cardano-node peer submits txs to Ade over N2N tx-submission2; verdicts byte-identical to direct corpus replay | RED operator action |
| `docs/clusters/PHASE4-N-E/CE-N-E-7_PROCEDURE.md` (NEW in N-E) | `docs/clusters/PHASE4-N-E/CE-N-E-7_<date>.log` | Real `cardano-cli transaction submit` to Ade over the N2C UDS; verdict matches N2N submission of the same bytes | RED operator action |

**These are evidence-log patterns, not BLUE seams.** They do not move
or wrap any chokepoint; they are the on-the-wire half of an existing
mechanical surface. New live-wire CE families should follow the same
pattern: a `_PROCEDURE.md` doc + a `_<date>.log` capture committed to
the cluster directory. The mechanical half of these CEs is already
CI-green (the GREEN bridges + harnesses + integration tests). N-E's
follow-up clusters (outbound tx propagation, mempool bounds, the
`proposal_procedures` decode) each carry the same expectation.

User confirmation needed for each candidate at cluster entry. **The
most load-bearing remaining candidates for the bounty** are the
**Conway block-body vkey-witness closure** (the carried B2 gap) and
the **forge / header→body bridge**.

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
| **GREEN bridge — N2N (NEW in N-E S4)** | `ade_core_interop::tx_submission::{event_to_ingress, PeerAccumulator, ingest_n2n_events}` | GREEN | Per-peer `InventoryEvent` accumulator. `event_to_ingress(&InventoryEvent, IngressSource) -> Vec<IngressEvent>` maps a single event (only `TxsDelivered` produces ingress events); `PeerAccumulator::observe` appends; `drain` produces `PeerSubmissionQueue { peer, source: N2N, txs }`; `ingest_n2n_events(base, &[(PeerId, Vec<InventoryEvent>)]) -> (MempoolState, Vec<AdmitOutcome>)` is the orchestrator. Pure; no I/O; no clocks. The live socket loop that drives a real cardano-node peer is the operator-action half (`CE-N-E-6_PROCEDURE.md`). |
| **GREEN bridge — N2C (NEW in N-E S5)** | `ade_core_interop::local_tx_submission::{local_event_to_ingress, ClientAccumulator, ingest_n2c_events}` | GREEN | Per-client `LocalTxSubmissionEvent` accumulator. `local_event_to_ingress(&LocalTxSubmissionEvent) -> Vec<IngressEvent>` maps a single event (only `TxSubmitted` produces ingress events; `TxAccepted` / `TxRejected` are server-to-client responses with no bytes to admit); `ClientAccumulator::observe` appends; `drain` produces `PeerSubmissionQueue { peer, source: N2C, txs }`; `ingest_n2c_events(base, &[(PeerId, Vec<LocalTxSubmissionEvent>)]) -> (MempoolState, Vec<AdmitOutcome>)` is the orchestrator. Pure; no I/O; no clocks. The live UDS loop that drives a real `cardano-cli` is the operator-action half (`CE-N-E-7_PROCEDURE.md`). |
| **GREEN per-peer canonicalizer (NEW in N-E S3)** | `ade_ledger::mempool::canonicalize::{canonicalize_peer_streams, PeerId, PeerSubmissionQueue}` | GREEN | Deterministic round-robin canonicalization of multi-peer queues. Peers visited in `PeerId` byte-lex order; round[i] emits one tx from every queue whose `.txs.get(i)` is `Some`. Ties (same `PeerId` across two queues) broken by single-byte source tag (N2N=0, N2C=1) — stable across binary builds (the doc-comment + the test `same_peer_id_same_source_stable_ordering`). Pure function of inputs; output independent of input iteration order (the function sorts internally). **The load-bearing GREEN fairness contract.** Any change to ordering, fairness policy, or tie-break is a SEAMS-level change because DC-MEM-04 (replay byte-identity) and the multi-peer interleaving tests pivot on it. |
| **BLUE chokepoint (NEW in N-E S1)** | `ade_ledger::mempool::ingress::{IngressSource, IngressEvent, mempool_ingress}` | BLUE | The single sanctioned production path into `admit` from non-test code. `IngressSource` is the closed 2-variant `{ N2N, N2C }` (no `#[non_exhaustive]`). `IngressEvent { source, tx_bytes }` carries the source variant as metadata only. `mempool_ingress(&MempoolState, &IngressEvent) -> (MempoolState, AdmitOutcome)` is a pure pass-through: `admit(mempool, event.tx_bytes())`. The `event.source` is **never read** inside `mempool_ingress`'s body — CI-enforced (`ci_check_mempool_ingress_closure.sh` greps the body for `\bsource\b`). Verbatim tx-bytes flow (no decode, no re-encode at this layer; PreservedCbor end-to-end into `admit`). DC-MEM-03 (`enforced`). |
| **BLUE admission gate (B2 — carried; unchanged)** | `ade_ledger::mempool::admit::admit` | BLUE | A tx is admitted iff `tx_validity(accumulating, tx)` is `Valid`. No false accept (DC-MEM-01, `strengthened_in += PHASE4-N-E`). The only sanctioned production caller is `mempool_ingress` (production callers elsewhere are CI-forbidden); `#[cfg(test)]` callers in tests/ directories are exempt. |
| **GREEN replay harness (NEW in N-E S2)** | `ade_testkit::mempool::{ingress_replay::{wrap_as_ingress, b_track_corpus_as_ingress, replay_ingress_trace, BTrackCase, ExpectedOutcome}}` | GREEN | Wraps the existing B-track adversarial corpus (the valid synthetic case + every `SyntheticMutation`) in synthetic `IngressEvent`s. `replay_ingress_trace(base, &[IngressEvent]) -> (MempoolState, Vec<AdmitOutcome>)` is a **single-step fold** over `mempool_ingress` (no batching, no parallel folding, no out-of-order interleaving — CI-enforced). Replay produces byte-identical traces (DC-MEM-04). |
| **CI gates (NEW in N-E)** | `ci/ci_check_mempool_ingress_closure.sh` + `ci/ci_check_mempool_ingress_replay.sh` | CI | The closure gate defends: (1) `mempool/ingress.rs` defines `IngressSource` / `IngressEvent` / `mempool_ingress`; (2) `IngressSource` is closed 2-variant (no `#[non_exhaustive]`); (3) `MempoolState.accumulating` is field-written only inside `mempool/admit.rs`; (4) production callers of `admit()` outside `mempool/admit.rs` definition and `mempool/ingress.rs` bridge are forbidden (`tests/` / `benches/` exempt); (5) `mempool_ingress` body must not reference `source`. The replay gate defends: (1) the four `ade_testkit::mempool::ingress_replay` symbols exist + are re-exported; (2) `replay_ingress_trace` body calls `mempool_ingress` and not direct `admit`; (3) the named integration tests exist; (4) no batching/parallel helpers in the harness; (5) the S3 canonicalizer module exists + exports the three items + is re-exported from `mempool/mod.rs`; (6) the canonicalizer body uses no async / RNG / clock / `HashMap` / `HashSet` / `RwLock` / `Mutex`. |
| **Operator-action evidence (NEW in N-E)** | `docs/clusters/PHASE4-N-E/{CE-N-E-6_PROCEDURE.md, CE-N-E-7_PROCEDURE.md}` + `CE-N-E-{6,7}_<date>.log` | RED operator action | Live cardano-node N2N peer submits txs (CE-N-E-6); real `cardano-cli` over Ade's N2C UDS submits txs (CE-N-E-7). Verdicts byte-identical to corpus / cross-bridge replay. **NOT a BLUE seam** — the on-the-wire half of an existing mechanical surface; mirror of the CE-N-B-6 pattern. |

**Rule.** This domain has **two GREEN bridge layers** (one per
transport, both pure deterministic functions in `ade_core_interop`),
**one GREEN canonicalizer** (the per-peer round-robin in
`ade_ledger::mempool::canonicalize` — the load-bearing fairness
contract), and **one BLUE chokepoint** (`mempool_ingress` in
`ade_ledger::mempool::ingress` — the single sanctioned production
path into `admit`). **THE KEY SEAMS:**

1. **The source variant is metadata only — verdict is a function of
   `(state, tx_bytes)` alone, regardless of source variant**
   (source-invariance, N-E-N7 / N-E-8). `mempool_ingress`'s body
   must not branch on `event.source` — CI-enforced. Two events
   differing only in `source` produce byte-identical
   `(MempoolState, AdmitOutcome)`.
2. **Tx bytes flow verbatim from ingress to admit — no decode, no
   re-encode at the ingress layer.** PreservedCbor end-to-end:
   `IngressEvent` holds `Vec<u8>`; the bridges build the event from
   the bytes the protocol delivered (`TxsDelivered.tx_bytes` for N2N,
   `TxSubmitted.tx_bytes` for N2C); the canonicalizer passes the
   bytes through unmodified (`tx_bytes_preserved_verbatim` covers
   the all-256-byte payload); `mempool_ingress` invokes
   `admit(mempool, event.tx_bytes())` directly.
3. **`mempool_ingress` is the only sanctioned production path into
   `admit` from non-test code** (DC-MEM-03 — CI-enforced).
4. **The per-peer canonicalizer is the load-bearing GREEN fairness
   contract.** Round-robin by sorted `PeerId` byte-lex with single-byte
   source tie-break (N2N=0, N2C=1) is the cluster's deterministic
   multi-peer interleaving rule (DC-MEM-04). Any future change is
   SEAMS-level.

**New work** that adds an ingress transport (a third
`IngressSource`, or a new way to deliver N2N/N2C txs) attaches by
**producing `IngressEvent`s and feeding them into `mempool_ingress`**
— not by adding a parallel admission path, not by adding a
verdict-side match on `source`, not by mutating `accumulating`
directly. Adding a third source variant is a closed-enum addition
(version-gated).

**Declared non-goals carried from the cluster doc:** outbound tx
propagation (Ade as a tx source), mempool bounds / shedding policy
(Tier-5 strengthening of `DC-MEM-02`), the `proposal_procedures`
tx-body decode into `GovAction`.

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
| **Tx-submission bridges** *(NEW in N-E)* | `ade_core_interop::{tx_submission, local_tx_submission}` | GREEN | The two N-E bridges that translate `InventoryEvent` / `LocalTxSubmissionEvent` payloads into `IngressEvent`s — see §2 "Mempool ingress" above. |

**Rule.** The codec layer is opaque to higher semantics. **N-E wired
the tx-submission2 / local-tx-submission tx-bytes → `mempool::admit`
bridge** (was a candidate seam in the prior revision; now closed via
the two GREEN `ade_core_interop` bridges + the BLUE `mempool_ingress`
chokepoint).

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
| **Live-interop driver (scaffold)** | `ade_core_interop::bin::live_consensus_session` | RED | |
| **Replay harness** | `ade_testkit::consensus::stream_replay::replay_stream` | GREEN | |

**Rule.** Five rules: genesis-parser sole RED→BLUE materialization;
`BootstrapAnchorHash` binds; `LedgerView` closed; authoritative
chokepoints never move; selector and chain-dep advance in lockstep.

### Where the boundary is enforced

- `ci_check_dependency_boundary.sh` — no BLUE crate may depend on RED.
  N-E added no new crate edge.
- `ci_check_no_async_in_blue.sh` — async forbidden in BLUE.
- `ci_check_mempool_ingress_closure.sh` *(NEW in N-E — DC-MEM-03,
  `status=enforced`)* — see the §2 "Mempool ingress" entry above.
- `ci_check_mempool_ingress_replay.sh` *(NEW in N-E — DC-MEM-04,
  `status=enforced`)* — defends the GREEN replay harness, the
  single-step fold property, the canonicalizer's existence + closed
  shape, and the canonicalizer body's freedom from async / RNG /
  clock / `HashMap` / `HashSet`.
- `ci_check_credential_discriminant_closed.sh` — DC-LEDGER-10.
- `ci_check_gov_cert_accumulation_closed.sh` *(B5 — DC-LEDGER-09)*.
- `ci_check_deposit_param_authority.sh` *(B3 — DC-TXV-07)*.
- `ci_check_conway_cert_classification_closed.sh` *(B3F — DC-TXV-06)*.
- `ci_check_no_chaindb_in_consensus_blue.sh` / `ci_check_no_float_in_consensus.sh`
  / `ci_check_no_density_in_fork_choice.sh` / `ci_check_consensus_closed_enums.sh`.
- `ci_check_pallas_quarantine.sh`, `ci_check_no_signing_in_blue.sh`,
  `ci_check_ingress_chokepoints.sh`, `ci_check_ce_n_a_5_proof.sh`.

**N-E note on `ci_check_ingress_chokepoints.sh`:** the new
`mempool_ingress` BLUE chokepoint does not construct a `PreservedCbor`
— it carries `Vec<u8>` and passes it to `admit`. The ingress chokepoint
is therefore outside this gate's scope (the existing
`tx_validity::decode_tx` chokepoint downstream is the one that lifts
the preserved body slice).

---

## 3. Closed vs. Extensible Registries

Ade's authority surface is **almost entirely closed.** **N-E added
three closed surfaces** — `IngressSource`, `IngressEvent`,
`mempool_ingress` — plus **two new CI gates**
(`ci_check_mempool_ingress_closure.sh`,
`ci_check_mempool_ingress_replay.sh`), bringing CI count `29 → 31`.
N-E strengthened one extensible surface's closure (the
`MempoolState.accumulating` field-write is now grep-gated, formerly
review-discipline only). N-E added **no new wholly open extensible
surface.**

### Closed (frozen — version-gated changes only)

| Registry | Location | Count | Change Rule |
|----------|----------|-------|-------------|
| `CardanoEra` | `ade_types::era` | 8 variants | New variant = new hard fork. |
| `Certificate` | `ade_types::shelley::cert` | 7 variants | Shelley-era frozen. **B4:** `PoolRegistrationCert.owners` added. |
| **`StakeCredential`** *(closed 2-variant — NEW shape in OQ5)* | `ade_types::shelley::cert` | 2 variants — `KeyHash(Hash28)`, `ScriptHash(Hash28)` | Grep-gated by `ci_check_credential_discriminant_closed.sh` (DC-LEDGER-10, strengthened across COMMITTEE / DREP / ENACTMENT-COMMITTEE-FIDELITY / ENACTMENT-COMMITTEE-WRITEBACK). |
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
| **`EnactmentEffects` struct** *(committee_changes discriminated in ENACTMENT-COMMITTEE-FIDELITY; committee_threshold added in ENACTMENT-COMMITTEE-WRITEBACK)* | `ade_ledger::governance` | closed struct | Grep-gated by `ci_check_credential_discriminant_closed.sh` check 6. |
| **`IngressSource`** *(NEW in N-E S1 — DC-MEM-03)* | `ade_ledger::mempool::ingress` | **2 variants — `N2N`, `N2C`** | Closed source discriminant. **No `#[non_exhaustive]`** — grep-defended by `ci_check_mempool_ingress_closure.sh`. The CI gate (check 2) defends both variant names + the closed-enum count (`grep -cE '^pub enum '` must be `1` in `ingress.rs`). New transport (a third `IngressSource` variant) is a versioned addition — requires coordinated updates to the CI gate's per-variant grep, the canonicalizer's source-byte tie-break (`source_byte` in `mempool/canonicalize.rs`), the two GREEN bridges (`ade_core_interop::{tx_submission, local_tx_submission}`), and any future bridge for the new transport. `Hash` + `Copy` derives are on the variant; ordering across variants in the canonicalizer uses the explicit `source_byte` tag (N2N=0, N2C=1), not declaration order. |
| **`IngressEvent`** *(NEW in N-E S1 — DC-MEM-03)* | `ade_ledger::mempool::ingress` | closed struct `{ source: IngressSource, tx_bytes: Vec<u8> }` | Closed flat-data envelope carrying tx bytes verbatim. Constructor is `pub fn new(source, tx_bytes)`; accessors are `source()` and `tx_bytes()`. `Clone` + `Debug` + `PartialEq` + `Eq`. New field = a versioned addition + downstream consumers updated; closed flat-data shape required (no `String`, no `Box<dyn>`). |
| **`mempool_ingress` chokepoint** *(NEW in N-E S1 — DC-MEM-03; closed surface, not a registry)* | `ade_ledger::mempool::ingress` | 1 function `mempool_ingress(&MempoolState, &IngressEvent) -> (MempoolState, AdmitOutcome)` | The **single BLUE chokepoint** from wire ingress into `admit`. Pure pass-through (`admit(mempool, event.tx_bytes())`); the body must not reference `source` (CI-enforced — `ci_check_mempool_ingress_closure.sh` greps the body for `\bsource\b`). The only sanctioned production caller of `admit` outside `mempool/admit.rs` definition (`#[cfg(test)]` tests/ + benches/ exempt). Removal / renaming / a second public path into `admit` from production source is CI-forbidden. |
| **`MempoolState.accumulating` field-write closure** *(strengthened in N-E S1 — DC-MEM-03)* | `crates/ade_ledger/src/mempool/admit.rs` | 1 production write site | The `accumulating: applied` field-set inside `admit()` is the only sanctioned production write site of this field. `ci_check_mempool_ingress_closure.sh` greps every `crates/ade_ledger/src/*.rs` outside `admit.rs` for `accumulating:[[:space:]]*[A-Za-z]` and fails on any match (B2 was review-discipline only; N-E grep-gates the closure). |
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
| **`MempoolState`** *(B2; field-write grep-gated in N-E)* | `ade_ledger::mempool::admit` | struct `{ accepted, accumulating }` | `accumulating` field-write is the **only** production write site, inside `admit()`. Grep-gated by `ci_check_mempool_ingress_closure.sh` (any `accumulating:` field-init outside `admit.rs` fails CI). |
| **`OrderPolicy`** *(B2)* | `ade_ledger::mempool::policy` | 2 variants — ArrivalOrder, TxIdAscending | |
| **`ConwayOnlyDepositParams`** *(B3; B5-enriched)* | `ade_ledger::pparams` | struct + `drep_activity` | |
| **`ConwayDepositParams`** *(B3)* | `ade_ledger::pparams` | struct (view) | |
| **`ValidationEnvironmentError`** *(B3)* | `ade_ledger::error` | | |
| **`UnsupportedStateDependentDepositAccounting`** *(B3)* | `ade_ledger::error` | | |
| **`EraInvalidCertificateError`** *(B3)* | `ade_ledger::error` | | |
| **`PraosNonces` / `NonceScanError`** *(B1)* | `ade_ledger::consensus_input_extract` | | |
| **`PraosChainDepState` / `ChainEvent` canonical encodings** *(N-B)* | `ade_core::consensus::encoding` | 4 chokepoints | |
| **`LedgerFingerprint` fold** *(B3-extended; B5-extended)* | `ade_ledger::fingerprint` | | |
| **CI check set** | `ci/ci_check_*.sh` | **31 scripts (29 → 31 in N-E)** | Existing checks may be tightened, never relaxed. **N-E added two:** `ci_check_mempool_ingress_closure.sh` (DC-MEM-03), `ci_check_mempool_ingress_replay.sh` (DC-MEM-04). |
| **Invariant registry families** | `docs/ade-invariant-registry.toml` | Families T / CN / DC / OP / RO; DC extended across all prior clusters; **N-E adds DC-MEM-03 + DC-MEM-04 and appends PHASE4-N-E to `DC-MEM-01.strengthened_in`** | Append-only IDs. |

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
| Mempool admitted set *(B2; ingress-fed in N-E)* | `ade_ledger::mempool::admit::MempoolState::accepted` | `Vec<Hash32>` of admitted tx ids in admission order. Shape closed; set open and grows monotonically per accepted tx. Mutated only by `admit` (Tier-1, called only from `mempool_ingress` in production source). NOT runtime-pluggable; no policy may add/remove ids (DC-MEM-02). |
| `SignerSource` provenance set *(B2)* | `ade_ledger::tx_validity::required_signers::RequiredSigners::{keys, provenance}` | Per-tx open; `SignerSource` *enum* closed. |
| `RollbackSnapshot` ring *(N-B)* | `ade_runtime::consensus::chain_selector::OrchestratorState::recent_snapshots` | Bounded ≤ `DEFAULT_SNAPSHOT_LIMIT = 2160`. |
| Oracle reference snapshots / regression corpus | `ade_testkit::harness::*` | Tooling-only. |
| Network corpus | `corpus/network/{n2n,n2c}/*` | Tooling-only. |
| Consensus corpus | `corpus/consensus/*` | Tooling-only. |
| Block-validity corpus *(B1)* | `corpus/validity/*` | Tooling-only. |
| Tx-validity corpus *(B2; B3-extended)* | `ade_testkit::tx_validity::*` + B3 conservation corpora | Tooling-only. |
| **Mempool ingress corpus** *(N-E; tooling-only)* | `ade_testkit::mempool::ingress_replay` + the B-track corpus wrapped via `b_track_corpus_as_ingress` | Tooling-only. The replay harness is a single-step fold over `mempool_ingress` (no batching; CI-enforced). New cases come from extending the existing B-track tx-validity corpus (NOT a new corpus file), wrapped as synthetic `IngressEvent`s via `wrap_as_ingress`. Append-only by convention. GREEN. |
| `KillStrategy<D>` trait impls | `ade_runtime::chaindb::crash_safety` | RED-only test infrastructure. |
| Recovery state types | callers of `Recoverable` | Open: any state with canonical encode + apply-block step. |
| Pinned external crates | `crates/*/Cargo.toml` | Tier-5 rationale doc required. |

### Candidates — extensible surfaces not yet wired

| Cluster | Candidate registry | Rationale |
|---------|-------------------|-----------|
| **N-E+ Tier-5** | **Mempool eviction / prioritization policy (bounded mempool, shedding policy)** beyond the `OrderPolicy` stub | Tier-5 — operator-tunable. Plugin trait candidate: `MempoolPolicy`. MUST stay below the Tier-1 `admit` gate (DC-MEM-02). **N-E note:** declared OUT-OF-SCOPE in the N-E cluster doc; would touch `CN-MEM-01`, `CN-MEM-03`, `DC-MEM-02`. Escalates to a Tier-1 / N-E-scope change only if a bound would change admission verdicts. |
| **N-E+ Tier-1** | **Outbound tx propagation (Ade as a tx source — `tx-submission2` server side)** | Separate authority surface from N-E's ingress half. A new BLUE/GREEN outbound bridge that reads the `MempoolState.accepted` admitted-id set and serves txs through the existing `tx_submission2_transition` server side. Declared OUT-OF-SCOPE in the N-E cluster doc. |
| N-A (deferred) | Peer address book | Operator-supplied; runtime mutable. |
| N-C | Block-production policy (forge cadence, KES rotation, slot election) | Tier 1 semantics, Tier 5 operator triggers. |
| N-F | Query API method set | Tier 5 wire / Tier 1 semantics. |
| N-F | Prometheus metric names | Tier 5; append-only registry expected. |
| GOVCERT-validity *(OQ-3, separable)* | Committee-membership precondition | Tier 1 — a tx-validity gate, NOT a registry. |
| credential-discriminant *(WIRED + CLOSED across OQ5 / COMMITTEE / DREP / ENACTMENT-COMMITTEE-FIDELITY / ENACTMENT-COMMITTEE-WRITEBACK)* | DONE — see closed surfaces above. | |
| proposal-decode *(declared non-goal — separable, NOT an open seam now)* | `proposal_procedures` tx-body decode into `GovAction` | Carried from ENACTMENT-COMMITTEE-WRITEBACK; declared non-goal in N-E. |

User confirmation needed for each at cluster entry.

### Closed-grammar audit (PHASE4-N-E specific)

This sweep was performed after PHASE4-N-E close (code half).

1. **`IngressSource`** — **closed by intent.** Closed 2-variant enum
   `{ N2N, N2C }`; no `#[non_exhaustive]`; grep-gated by
   `ci_check_mempool_ingress_closure.sh` (checks 2 + variant-name
   greps + `pub enum` count == 1). A new transport variant is
   version-gated and requires coordinated updates to the
   canonicalizer's `source_byte` tie-break and the GREEN bridges.
2. **`IngressEvent`** — **closed by intent.** Closed flat-data struct
   `{ source, tx_bytes }`; the gate (check 1) requires `pub struct
   IngressEvent` to be present. A new field is a versioned addition.
3. **`mempool_ingress` chokepoint** — **closed by intent, not an
   extension point.** Body must not reference `source` (check 5).
   Production callers of `admit` outside this chokepoint are
   CI-forbidden (check 4); `tests/` and `benches/` are exempt.
4. **`MempoolState.accumulating` field-write closure** — **grep-gated
   in N-E** (was review-discipline only at B2). Any `accumulating:`
   field-init in `crates/ade_ledger/src/**.rs` outside `admit.rs`
   fails CI (check 3).
5. **Per-peer canonicalizer** — **closed by intent on the algorithm.**
   Round-robin by sorted `PeerId` byte-lex with single-byte source
   tie-break (N2N=0, N2C=1). The function sorts internally so output
   is independent of input iteration order. The canonicalizer body
   has no async / RNG / clock / `HashMap` / `HashSet` / `RwLock` /
   `Mutex` (`ci_check_mempool_ingress_replay.sh` check 6).
   Source-byte ordering is asserted by the doc-comment and the
   `same_peer_id_same_source_stable_ordering` test, not by
   enum-declaration order — change of the ordering rule is
   SEAMS-level.
6. **GREEN replay harness — single-step fold** — **closed by intent
   on the fold shape** (OQ-6). `replay_ingress_trace` calls
   `mempool_ingress` (not direct `admit`); no batching / parallel /
   out-of-order helpers (checks 2 + 4).
7. **N2N/N2C GREEN bridges** — **closed by intent on the
   event-to-ingress mapping.** Only `TxsDelivered` (N2N) /
   `TxSubmitted` (N2C) produce `IngressEvent`s; all other variants
   of the closed N-A event enums emit empty Vec.

**Gap note — N-E (narrow).** The N-E GREEN bridges in `ade_core_interop`
(`tx_submission.rs`, `local_tx_submission.rs`) are not in any
grep-gate's `TARGETS` scope — their closed `event_to_ingress` /
`local_event_to_ingress` `match` shape rests on review-discipline +
the compiler-exhaustive `match` over the closed `InventoryEvent` /
`LocalTxSubmissionEvent` enums (closed at N-A). Extending
`ci_check_mempool_ingress_closure.sh` to scan the bridges for "did
`event_to_ingress` ever emit an event from a non-`TxsDelivered`
variant" would close this narrow gap. Not load-bearing today (N-A
closed both event enums), surfaced for confirmation.

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
`ci_check_consensus_closed_enums.sh` — their closed shape is
compiler-exhaustive-match + test-and-review-enforced rather than
grep-gated. Unchanged at this HEAD.

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
- **`mempool_ingress` chokepoint contract** *(NEW in N-E S1)*: the
  **single sanctioned production path into `admit` from wire ingress**.
  `mempool_ingress(state, evt) == admit(state, evt.tx_bytes())` — a
  pure pass-through; `event.source` is recorded but MUST NOT affect
  the verdict (CI-enforced — `ci_check_mempool_ingress_closure.sh`
  check 5 greps the body for `\bsource\b`). Production callers of
  `admit` outside `mempool/admit.rs` definition and `mempool/ingress.rs`
  bridge are CI-forbidden (check 4); `tests/` and `benches/` are
  exempt. Removal / renaming / a second public production path into
  `admit` from production source is CI-forbidden (DC-MEM-03).
- **`IngressSource` source-invariance contract** *(NEW in N-E S1 — N-E-N7 / N-E-8)*:
  `IngressSource` is metadata only — the verdict is a function of
  `(state, tx_bytes)` alone, regardless of source variant. Equivalently,
  for all `(s, b)`:
  `mempool_ingress(s, IngressEvent { source: N2N, b }) ==
   mempool_ingress(s, IngressEvent { source: N2C, b })`. Enforced by
  the inline `ingress_source_does_not_change_verdict_*` tests + the
  corpus-wide `ingress_trace_source_invariant_n2n_vs_n2c` test, and
  defended by `ci_check_mempool_ingress_closure.sh` check 5.
- **Verbatim tx-bytes flow through ingress** *(NEW in N-E)*: tx bytes
  flow verbatim from the wire grammar through the GREEN bridges and
  canonicalizer into `mempool_ingress` and then into `admit` (which
  passes them to `tx_validity::decode_tx`, the existing ingress
  chokepoint that lifts the PreservedCbor body slice). **No decode,
  no re-encode at the ingress layer** — `IngressEvent` carries `Vec<u8>`
  unchanged; `canonicalize_peer_streams` passes bytes through
  (`tx_bytes_preserved_verbatim` over an all-256-byte payload);
  `mempool_ingress` calls `admit(mempool, event.tx_bytes())` directly.
  Per the N-E cluster doc §"Forbidden During This Cluster":
  "Decoding and re-encoding tx body bytes anywhere on the ingress path —
  `PreservedCbor<Tx>` MUST flow end-to-end."
- **GREEN single-step replay fold contract** *(NEW in N-E S2 — DC-MEM-04)*:
  replaying the same ordered `[IngressEvent]` against the same `base`
  produces a byte-identical `(MempoolState, Vec<AdmitOutcome>)` pair;
  the fold is **single-step per OQ-6** (no batching, no parallel
  folding, no out-of-order interleaving). Grep-defended by
  `ci_check_mempool_ingress_replay.sh` check 4 (the harness body must
  contain none of `chunks` / `chunks_exact` / `partition` / `par_iter`
  / `rayon` / `tokio::spawn`).
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
  `ade_core_interop` (incl. the new N-E `tx_submission` /
  `local_tx_submission` bridges) is RED-crate / GREEN-pure-functions.
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
- **New CI check**: additive. (N-E added two:
  `ci_check_mempool_ingress_closure.sh`,
  `ci_check_mempool_ingress_replay.sh`.)
- **Pinned external crate bump**: Tier-5 rationale doc required.
- **New mini-protocol**.
- **Mini-protocol version-table bump**.
- **New `ChainEvent` / `ChainSelectionReject` / `StreamInput` variant** *(N-B)*.
- **New `NetworkMagic`** *(N-B)*.
- **New `LedgerView` impl / LedgerState-backed `PoolDistrView` constructor**.
- **`BootstrapAnchorHash` preimage v2** *(N-B)*: hard version-gated.
- **N2N/N2C tx-submission → `mempool_ingress` ingress** *(WIRED + CLOSED in N-E)*:
  the GREEN bridges + the BLUE chokepoint are the canonical wiring.
  **Remaining separable version-gated follow-ups** (NOT open seams
  now): a third `IngressSource` variant (would require a coordinated
  CI-gate update + the canonicalizer's `source_byte` extension + a
  new GREEN bridge); outbound tx propagation (Ade as a tx source —
  declared non-goal in the N-E cluster doc); a bounded mempool /
  shedding policy if a bound would change admission verdicts
  (escalates to Tier-1 / N-E scope; declared non-goal in N-E).
- **Phase-4 cluster surface additions** (N-C, N-F): each cluster's
  wire surface gates additions via its own cluster doc.

---

## 5. Module Addition Rules

Ade's workspace is small and color-disciplined. **N-E added two new
submodules entirely inside the existing BLUE `ade_ledger` crate**
(`ade_ledger::mempool::ingress` BLUE + `ade_ledger::mempool::canonicalize`
GREEN), **one new GREEN submodule inside the existing GREEN
`ade_testkit` crate** (`ade_testkit::mempool` with `ingress_replay.rs`),
and **two new GREEN files inside the existing RED `ade_core_interop`
crate** (`tx_submission.rs`, `local_tx_submission.rs`). It added **no
new crate, no new external ingress wire-format frozen contract, and
no new public composer.** The new BLUE chokepoint `mempool_ingress`
sits in `ade_ledger::mempool` alongside the existing `admit` /
`policy` — the same crate-internal seam B2 established for the
Tier-1 / Tier-5 mempool split, now extended with a Tier-1 wire-level
layer above `admit`.

**The module-addition rule N-E sets for future wire-ingress work:**

1. **A new wire-ingress transport attaches as a closed source
   variant on `IngressSource`** (version-gated; coordinated update
   to the CI gate's per-variant grep + the canonicalizer's
   `source_byte` tie-break + a new GREEN bridge in `ade_core_interop`).
2. **A new wire-ingress transport bridge lives in `ade_core_interop`**
   (the RED crate that already houses the live cardano-node session
   binary): a deterministic GREEN `event_to_ingress` mapping + a
   per-peer/per-client accumulator + an orchestrator that calls
   `canonicalize_peer_streams` and `replay_ingress_trace`. The
   bridge code itself is pure / deterministic; the live socket loop
   is the RED operator-action half (documented via a
   `_PROCEDURE.md` in the cluster directory + a `_<date>.log`
   capture).
3. **A new wire-ingress transport must NOT add a parallel admission
   path** — `mempool_ingress` is the single sanctioned BLUE
   chokepoint into `admit` from non-test code (CI-enforced).
4. **A new wire-ingress transport must NOT branch the verdict on
   the source variant** — `IngressSource` is metadata only
   (CI-enforced).
5. **A new wire-ingress transport must NOT decode and re-encode the
   tx bytes at the ingress layer** — verbatim flow into `admit`.

**OQ5/COMMITTEE/DREP/ENACTMENT-COMMITTEE-FIDELITY/WRITEBACK** all
followed the in-place-tightening model. **B5** added one new
crate-internal BLUE module + one new CI gate. **B4** added the
owner-tagged apply model in place. **B3** added four BLUE submodules
inside existing BLUE crates. **B2** added the `tx_validity::*` and
`mempool::{admit, policy}` submodule trees. **N-E follows the B2
pattern**: another submodule tree inside `ade_ledger::mempool`
(`ingress` BLUE + `canonicalize` GREEN), the BLUE chokepoint
sitting adjacent to `admit`.

| Color | Naming convention | Build-config flags | May depend on | MUST NOT depend on |
|-------|-------------------|--------------------|----------------|--------------------|
| **BLUE** | `ade_*` | First line of every `.rs` is the contract banner. `lib.rs` carries `#![deny(unsafe_code, clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_arithmetic)]`. No `#[cfg(feature = ...)]`. No async. No `ChainDb`/`f32`/`f64`/density inside `ade_core::consensus`. No `#[non_exhaustive]`/open-tail/`String`/`Box<dyn>` in `ade_core::consensus`, `ade_ledger::block_validity`, `ade_ledger::tx_validity`, `ade_ledger::mempool`. **N-E:** `IngressSource` closed 2-variant; `IngressEvent` closed flat-data struct; `mempool_ingress` body must not reference `source` (DC-MEM-03). | Other BLUE crates / submodules only | Any RED submodule or crate; GREEN in non-dev deps; `pallas_*` (except `ade_plutus`); async runtime; `HashMap`/`HashSet`/`IndexMap`; clock/rand/float/env/I/O. |
| **GREEN** | `ade_*` | Banner + deny attrs are project convention but not currently enforced for `ade_testkit` / `ade_network::mux::mod` / `ade_ledger::mempool::policy` / **`ade_ledger::mempool::canonicalize`** / **`ade_testkit::mempool::ingress_replay`** / **the two `ade_core_interop` N-E bridges**. **N-E:** the canonicalizer body is grep-gated NO async / RNG / clock / `HashMap` / `HashSet` / `RwLock` / `Mutex` (`ci_check_mempool_ingress_replay.sh` check 6); the replay harness is grep-gated single-step (check 4). | BLUE crates + standard library + ecosystem crates | `ade_runtime` (for `ade_testkit`); RED submodules in non-test paths. Results must never feed back into a BLUE authoritative decision (policy must never affect `admit`; the canonicalizer must never call `tx_validity`; the bridges must never bypass `mempool_ingress`). |
| **RED** | `ade_*` | No special header. Free to use clocks, I/O, async, `HashMap`, signing keys. | Any BLUE / GREEN crate or submodule (one-way) | Cannot be depended on by BLUE. |

### New module checklist

1. **Add to `Cargo.toml` workspace members** (if a new crate).
2. **Declare TCB color** by editing `.idd-config.json` `core_paths` if BLUE.
3. **CI script update obligations** — extend the relevant BLUE-scoped
   scripts. For a new mempool ingress transport, extend
   `ci_check_mempool_ingress_closure.sh`'s per-variant grep (check 2),
   the canonicalizer's `source_byte` map, and ensure any new GREEN
   bridge satisfies the `ci_check_mempool_ingress_replay.sh`
   single-step constraint. For closed-taxonomy additions, add the
   new module path to the `TARGETS` of `ci_check_consensus_closed_enums.sh`
   (which already covers `ade_core::consensus`,
   `ade_ledger::block_validity`, `ade_ledger::tx_validity`, and
   `ade_ledger::mempool` — note `mempool::ingress` is inside this
   TARGETS scope since `mempool/` was extended for B2; the new
   `IngressSource` / `IngressEvent` shapes are therefore already
   grep-gated for `String` / `Box<dyn>` / `#[non_exhaustive]`).
4. **Add contract banner** (BLUE) to every `.rs` file.
5. **Add deny attributes** to `lib.rs` (BLUE).
6. **New canonical types:** add a `[[rules]]` block under family `T`
   in the invariant registry, plus a round-trip test.
7. **Run `cargo test --workspace` and the full CI script suite.**

### Phase 4 anticipated additions

- **PHASE4-N-E (Tier 1 wire-level mempool ingress) — DONE (code half)**:
  see §1 / §2 / §3 / §4 above. Live N2N/N2C evidence logs
  (CE-N-E-6 / CE-N-E-7) operator-action.
- **Tx-validity completeness follow-ups**: full `track_utxo=true`
  corpus; pre-Conway eras; the Conway block-body vkey-witness
  closure (carried).
- **Outbound tx propagation (post-N-E)**: Ade as a tx source via
  `tx-submission2` server side. A new BLUE/GREEN outbound bridge in
  `ade_core_interop` joining `MempoolState.accepted` readers to
  `tx_submission2_transition`. Declared non-goal in N-E.
- **Mempool bounds / shedding policy (Tier-5)**: extends `OrderPolicy`
  with a bounded-eviction variant; MUST stay below the Tier-1
  `admit` gate (DC-MEM-02). Declared non-goal in N-E unless a bound
  would change admission verdicts.
- **`proposal_procedures` decode (post-ENACTMENT-COMMITTEE-WRITEBACK)**:
  a closed sub-grammar reader inside the Conway tx body lifting the
  opaque slice into `Vec<GovAction>`. Carried; declared non-goal in N-E.
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
  `IngressSource` or `IngressEvent` (the `mempool/` subtree is in
  the `ci_check_consensus_closed_enums.sh` `TARGETS` set extended
  for B2). No second public production path into `admit` outside
  this chokepoint (check 4 — `tests/` and `benches/` exempt). No
  mutation of `MempoolState.accumulating` from outside
  `mempool/admit.rs` (check 3). No decode / re-encode of tx body
  bytes at this layer — verbatim flow into `admit`.

### GREEN (`ade_testkit` incl. `validity` / `tx_validity` / **`mempool`** + B3/B4/B5/OQ5/COMMITTEE/DREP corpora; `ade_network::lib` / `mux::mod`; `ade_runtime::consensus::{candidate_fragment, chain_selector}`; **`ade_ledger::mempool::{policy, canonicalize}`**; **the two `ade_core_interop` N-E bridges**)

- No nondeterminism that leaks into stored fixtures — fixtures must
  be byte-reproducible. **N-E:** `ingress_trace_replay_byte_identical`,
  `ingress_admit_equals_direct_admit_on_b_track_corpus`,
  `b_track_adversarial_rejections_preserved_through_ingress`.
- No participation in authoritative outputs. The N-E ingress harness
  only *drives* `mempool_ingress` / `admit` / `tx_validity` and
  asserts; the canonicalizer is a pure GREEN sequencer.
- No `HashMap` even in test helpers — `BTreeMap` only.
- No import of `ade_runtime` from `ade_testkit`.
- (`ade_runtime::consensus::chain_selector`) No comparison decision.
- (`ade_ledger::mempool::policy`) No call to `tx_validity`; no read
  of the accumulating state; no add/remove of a tx id (DC-MEM-02).
- **(`ade_ledger::mempool::canonicalize`, N-E)** No async / RNG /
  clock / `HashMap` / `HashSet` / `RwLock` / `Mutex` —
  `ci_check_mempool_ingress_replay.sh` check 6. The per-peer
  ordering rule (round-robin by sorted `PeerId` byte-lex, source-byte
  tie-break N2N=0/N2C=1) is the load-bearing GREEN fairness contract
  — any change is SEAMS-level.
- **(`ade_testkit::mempool::ingress_replay`, N-E)** No call to
  direct `admit` from `replay_ingress_trace` (must go through the
  BLUE bridge `mempool_ingress` — check 2). No batching / parallel /
  out-of-order helpers — `chunks` / `chunks_exact` / `partition` /
  `par_iter` / `rayon` / `tokio::spawn` are grep-forbidden
  (check 4). The fold is strictly single-step per OQ-6.
- **(`ade_core_interop::tx_submission` / `local_tx_submission`, N-E)**
  Pure deterministic functions over their inputs; no I/O, no clocks
  inside `event_to_ingress` / `PeerAccumulator::observe` /
  `local_event_to_ingress` / `ClientAccumulator::observe` /
  `ingest_n2n_events` / `ingest_n2c_events`. The compiler-exhaustive
  `match` over the closed `InventoryEvent` / `LocalTxSubmissionEvent`
  enums (closed at N-A) keeps the event-to-ingress mapping closed:
  only `TxsDelivered` (N2N) and `TxSubmitted` (N2C) produce ingress
  events; all other variants emit empty Vec. The bridge must
  produce a `PeerSubmissionQueue` with the correct `IngressSource`
  variant (N2N for `ingest_n2n_events`, N2C for `ingest_n2c_events`).
  The live socket/UDS loop that drives a real peer / `cardano-cli`
  is the operator-action half — never linked into a GREEN test binary.

### RED (`ade_runtime`, `ade_node`, `ade_network::mux::transport`, `ade_network::session`, `ade_network::bin::capture_*`, `ade_runtime::consensus::genesis_parser`, `ade_core_interop`, and the RED-behavior `ade_ledger::consensus_input_extract` scan)

- No direct mutation of `ade_ledger` state — all transitions go
  through `ade_ledger::rules::*`, the `block_validity` /
  `tx_validity` composers, or **`mempool::ingress::mempool_ingress`**
  (the new Tier-1 wire-level chokepoint; the prior `mempool::admit`
  direct call from production is CI-forbidden in N-E).
- No bypassing `ade_codec` to construct semantic types from raw bytes.
- (`ade_runtime` specifically) No dep on `ade_ledger`. No leakage of
  `redb` types. No second public `chaindb` path.
- (`ade_network::mux::transport`) No protocol logic.
- (`ade_network::session`) Composition glue only.
- (`ade_network::bin::capture_*`) Live-interop tools only.
- (`ade_runtime::consensus::genesis_parser`) No re-derivation of the
  bootstrap anchor outside `compute_anchor_hash`.
- (`ade_ledger::consensus_input_extract`) Pure-over-bytes.
- **(N-E live N2N/N2C operator-action sessions — the RED half of
  CE-N-E-6 / CE-N-E-7)** The live socket / UDS loop that drives a
  real cardano-node peer or `cardano-cli` MUST funnel its delivered
  tx-byte events through the GREEN `ade_core_interop::tx_submission`
  / `ade_core_interop::local_tx_submission` bridge — it MUST NOT
  carry a parallel admission path, MUST NOT call `admit` directly
  from production source, MUST NOT bypass `mempool_ingress`, and
  MUST NOT branch the verdict on whether the bytes arrived over N2N
  or N2C. Evidence is captured into
  `docs/clusters/PHASE4-N-E/CE-N-E-{6,7}_<date>.log`.
- (`ade_core_interop`) Live-interop driver only; tests
  `#[ignore]`-gated. The N-E GREEN bridges in this crate are
  deterministic pure functions; the live socket loops are
  operator-action.

### Project-specific additions

- **No commits of credentials, hostnames, IPs, private keys** —
  enforced by `ci_check_no_secrets.sh`.
- **No `Phase 4 internal-mode mock network`** — Tier 1 surfaces must
  be exercised against real cardano-node peers. **N-E:** the live
  N2N/N2C wire-evidence logs (CE-N-E-6 / CE-N-E-7) follow the
  CE-N-B-6 pattern — real cardano-node + real `cardano-cli`.
- **No collapsing wire and canonical bytes** — dual-authority rule.
- **No Tier 5 surface without a stated rationale** — the new GREEN
  per-peer canonicalizer is Tier-5 within `ade_ledger::mempool`; the
  Tier-5 rationale is "deterministic per-peer fairness for replay
  byte-identity" (DC-MEM-04) and the SEAMS-level pin protects the
  ordering rule.
- **No "we'll match it later" stubs on Tier 1 surfaces** — Tier 1
  closure is hard-gated. The B1 block verdict, the B2 tx verdict,
  the B2 mempool admission gate, the B3 full value-conservation
  accounting, the B4 Conway cert-state accumulation, the B5 Conway
  governance-cert accumulation, the ENACTMENT-COMMITTEE-WRITEBACK
  committee write-back, and **the N-E Tier-1 wire-level mempool
  ingress** are all Tier-1 surfaces. (Like CE-N-B-6, CE-N-E-6 /
  CE-N-E-7 live wire-evidence are operator-action — the code +
  GREEN evidence is CI-green; the live log artifacts are not stubs
  but evidence patterns for the on-the-wire half.)

---

## Cross-references

- CODEMAP: `docs/ade-CODEMAP.md` — module-by-module authority table,
  upstream of this document. **Cross-reference check:** CODEMAP is
  being regenerated in parallel with this SEAMS at HEAD `43fcc31`;
  the in-flight CODEMAP may still pin pre-N-E HEAD (`168ac02`) at
  the moment of this regen. The narrative above names exact source
  file paths so the next CODEMAP regen can find every new module
  mechanically: the BLUE `crates/ade_ledger/src/mempool/ingress.rs`,
  the GREEN `crates/ade_ledger/src/mempool/canonicalize.rs`, the
  GREEN `crates/ade_testkit/src/mempool/{mod.rs, ingress_replay.rs}`,
  the GREEN bridges `crates/ade_core_interop/src/{tx_submission.rs,
  local_tx_submission.rs}`, and the two new CI scripts
  `ci/ci_check_mempool_ingress_{closure,replay}.sh`. **Note:** the
  prior CODEMAP at `168ac02` reports 29 CI checks; the post-N-E
  count is 31. The next CODEMAP regen should pick this up
  automatically by re-scanning `ci/ci_check_*.sh`.
- Invariant registry: `docs/ade-invariant-registry.toml` — rule
  families incl. T / CN / DC / OP / RO. **N-E added:** `DC-MEM-03`
  (`enforced`, `ci_script = ci/ci_check_mempool_ingress_closure.sh`,
  `introduced_in = PHASE4-N-E`) and `DC-MEM-04` (`enforced`,
  `ci_script = ci/ci_check_mempool_ingress_replay.sh`,
  `introduced_in = PHASE4-N-E`); and appended `PHASE4-N-E` to
  `DC-MEM-01.strengthened_in` (now `["PHASE4-B2", "PHASE4-N-E"]`).
- Phase 4 cluster plan: `docs/active/phase_4_cluster_plan.md`.
- Tier doctrine: `docs/active/CE-79_gate_statement.md` and
  `docs/active/CE-79_tier5_addendum.md`.
- Cluster N-D slices (closed): `docs/clusters/completed/PHASE4-N-D/S-{33..37}.md`.
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
- Cluster B5 (closed): `docs/clusters/PHASE4-B5/cluster.md` +
  `B5-S{2..5}.md` (declares OQ-3 / OQ-5).
- Cluster OQ5-CREDENTIAL-FIDELITY (closed).
- Cluster COMMITTEE-CRED-FIDELITY (closed).
- Cluster DREP-VOTE-FIDELITY (closed).
- Cluster ENACTMENT-COMMITTEE-FIDELITY (closed).
- Cluster ENACTMENT-COMMITTEE-WRITEBACK (closed): leaves
  `proposal_procedures` decode as the one declared non-goal
  governance-domain seam.
- **Cluster PHASE4-N-E (mechanically closed, not yet archived to
  `docs/clusters/completed/`)**: the cluster doc + slices
  `docs/clusters/PHASE4-N-E/{cluster.md, N-E-S{1..5}.md,
  CE-N-E-6_PROCEDURE.md, CE-N-E-7_PROCEDURE.md}`. WIRES AND CLOSES
  the Tier-1 wire-level mempool ingress seam — `IngressEvent` +
  closed `IngressSource` 2-variant + the BLUE chokepoint
  `mempool_ingress` (DC-MEM-03) + the GREEN replay harness
  (DC-MEM-04) + the GREEN per-peer canonicalizer + the two GREEN
  `ade_core_interop` bridges (`tx_submission`, `local_tx_submission`).
  STRENGTHENS DC-MEM-01 (`strengthened_in += PHASE4-N-E`). Adds two
  new CI scripts (`ci_check_mempool_ingress_closure.sh`,
  `ci_check_mempool_ingress_replay.sh` — count `29 → 31`); no new
  crate; no new external ingress wire-format frozen contract; the
  source-invariance + verbatim-tx-bytes + single-step-fold properties
  are the load-bearing N-E rules. **Declared non-goals carried to
  future clusters:** outbound tx propagation (Ade as a tx source),
  mempool bounds / shedding (Tier-5), and (from
  ENACTMENT-COMMITTEE-WRITEBACK) `proposal_procedures` tx-body
  decode. **CE-N-E-6 / CE-N-E-7 live N2N/N2C evidence logs** are
  operator-action artifacts pending capture (CE-N-B-6 pattern).
