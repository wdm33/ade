# HEAD Deltas — Ade

> **Status:** Living architectural document. Regenerated; not hand-edited.
> Regenerate via `/head-deltas <baseline>`. Baseline is declared in
> `.idd-config.json` (`head_deltas_baseline`).

> Baseline: `d509f02` (Phase 3 handoff snapshot, 2026-04-15)
> HEAD: `43fcc31` (feat(interop): N2C local-tx-submission -> mempool_ingress bridge (PHASE4-N-E S5), 2026-05-25)
> 139 commits, 11,304 files changed, +177,095 / −7,233,590 lines

Headline numbers note: the massive negative line count is dominated by
the **corpus relayout** under `corpus/snapshots/` and the deletion of
the multi-MB credentialed-snapshot text files
(`*_registered_creds.txt`, ~7M lines combined). Source-tree deltas are
far smaller — the per-crate breakdown in §3 is the representative view.

> **Commit-hash note.** This regen runs against the current (rebased)
> history. Earlier HEAD_DELTAS regens referenced commit hashes from a
> history that has since been rewritten; all hashes below are verbatim
> from `git log d509f02..HEAD` at this HEAD.

> **PHASE4-N-E cluster note (newest thread).** This regen is cut at
> committed HEAD `43fcc31`. Since the prior grounding-doc refresh
> commit `52642e5` (which committed the post-WRITEBACK
> CODEMAP/SEAMS/HEAD_DELTAS/TRACEABILITY ripple at HEAD `168ac02`,
> archived 7 closed cluster dirs under `docs/clusters/completed/`, and
> reclassified two §B-prose paragraphs in TRACEABILITY as CLOSED for
> the deposit/refund and Conway body-witness gaps), **five new commits
> have landed** — the full **PHASE4-N-E Tier-1 wire-level mempool
> ingress arc**: `32c1ee6` (S1, `IngressEvent` + `mempool_ingress`
> closed BLUE chokepoint), `2d0c918` (S2, GREEN ingress-replay harness
> + B-track corpus reuse + `DC-MEM-04`), `509d714` (S3, per-peer
> GREEN canonicalizer), `ca3f23a` (S4, N2N tx-submission2 →
> `mempool_ingress` GREEN bridge under `IngressSource::N2N`), and
> `43fcc31` (S5, N2C local-tx-submission → `mempool_ingress` GREEN
> bridge under `IngressSource::N2C`). **2 new BLUE/GREEN modules under
> `ade_ledger::mempool` (`ingress`, `canonicalize`), 1 new GREEN
> module under `ade_testkit::mempool` (`ingress_replay`), and 2 new
> GREEN modules under `ade_core_interop` (`tx_submission`,
> `local_tx_submission`).** Two new CI scripts
> (`ci_check_mempool_ingress_closure.sh` for `DC-MEM-03`,
> `ci_check_mempool_ingress_replay.sh` for `DC-MEM-04`). Two new
> registry rules (`DC-MEM-03` closure, `DC-MEM-04` replay); registry
> total 175 (was 173). `DC-MEM-01` strengthened by `PHASE4-N-E`.
> One new Cargo dep edge: `ade_core_interop -> ade_ledger` (was
> transitive only). 39 new tests across the 5 slices. **Cluster status:
> code + harness complete, live evidence pending.** `CE-N-E-6` and
> `CE-N-E-7` close in two halves — the mechanical adapter half is
> green in S4/S5, the live-log half is operator-action per the
> documented procedures (`docs/clusters/PHASE4-N-E/CE-N-E-6_PROCEDURE.md`
> and `CE-N-E-7_PROCEDURE.md`). Cluster dir
> `docs/clusters/PHASE4-N-E/` is in-flight — **NOT yet archived to
> `docs/clusters/completed/`** (closure pending the two live-evidence
> log artifacts). The parallel `docs(grounding)` refresh of CODEMAP /
> SEAMS / TRACEABILITY is in progress in the same regeneration round
> as this HEAD_DELTAS rewrite.

> **Testkit follow-up note (prior thread, carried forward).** Between
> the prior grounding-doc commit `3d94c22` and the post-WRITEBACK
> refresh `52642e5`, four GREEN-scope commits landed
> (`b9cfaf9`/`396664a`/`c78ec76`/`168ac02`) — bounded to `ade_testkit`
> / corpus tooling, no BLUE source change, no new rule, no new CI
> script. They wired the real-chain committee oracle at mainnet
> 575→576, aligned 11 previously-blocked tests with the regenerated
> corpus, added the `#[ignore]`-gated `reward_provenance` generator,
> and closed three snapshot-loader follow-ups (tip slot + Conway
> UMElem layout). `DC-EPOCH-01` and `DC-LEDGER-10` each gained one
> oracle test (`committee_oracle_mainnet_575_576_noop_agreement`);
> the committee-CHANGE `open_obligation` was reclassified
> environment-blocked → reality-blocked.

> **ENACTMENT-COMMITTEE-WRITEBACK cluster note (prior thread, carried
> forward).** Three implementation commits (`ea25dd9`, `f2f15f9`,
> `3180e27`) + close-hardening (`69e2d4b`) + grounding refresh
> (`3d94c22`). Turned the dormant type pin into a **live committee
> write-back**, without a new module/rule/CI script. Cluster docs at
> `docs/clusters/completed/ENACTMENT-COMMITTEE-WRITEBACK/`.

> **ENACTMENT-COMMITTEE-FIDELITY / DREP-VOTE-FIDELITY /
> COMMITTEE-CRED-FIDELITY / OQ5-CREDENTIAL-FIDELITY cluster notes
> (prior threads, carried forward).** Each cluster's recorded
> structural changes, fingerprint surfaces, and credential-discriminant
> ripples are unchanged at this HEAD. All cluster docs archived at
> `docs/clusters/completed/<NAME>/`.

> **B5 / B4 / B3F / B3 / B2 / B1 / N-D / N-B / N-A cluster notes
> (carried forward).** All closed and archived at
> `docs/clusters/completed/<NAME>/`.

The delta now covers twenty-three threads of work. The newest thread
— the **PHASE4-N-E wire-level mempool ingress arc** (`32c1ee6`,
`2d0c918`, `509d714`, `ca3f23a`, `43fcc31`) — sits on the post-WRITEBACK
grounding refresh `52642e5`, which itself sat on the testkit
follow-ups `168ac02`, which sat on the WRITEBACK grounding refresh
`3d94c22`, and so on down the established stack. In rough proportion
of the substantive change budget:

0. **PHASE4-N-E (wire-level mempool ingress, Tier 1) — code +
   harness complete, live evidence pending.** Five implementation
   commits closing the wire-level ingress closure for the bounty's
   no-false-accept tx-submission slice. The cluster's load-bearing
   move is a **single closed BLUE chokepoint**: every wire-arriving
   transaction (N2N tx-submission2 deliveries; N2C local-tx-submission
   submissions) reduces to a closed `IngressEvent { source:
   IngressSource, tx_bytes }` before reaching `mempool::admit`, and
   the `source` variant is metadata only — it cannot leak into the
   verdict. **S1** (`32c1ee6`, BLUE) added
   `crates/ade_ledger/src/mempool/ingress.rs` with the closed
   2-variant `IngressSource::{N2N, N2C}` enum, the `IngressEvent`
   record, and the `mempool_ingress(state, event) -> (state',
   AdmitOutcome)` pure pass-through to `admit`, plus a 5-clause CI
   gate (`ci_check_mempool_ingress_closure.sh`) that enforces the
   closure (no `#[non_exhaustive]`, no `event.source` reference in
   the verdict path, `admit()` callable only from the chokepoint and
   its co-located tests, `MempoolState.accumulating` field-write
   localized to `admit.rs`). New rule `DC-MEM-03` (enforced) and
   bidirectional `cross_ref` between `DC-MEM-01` and `DC-MEM-03`.
   8 new tests (2 inline + 6 integration). **S2** (`2d0c918`, GREEN)
   added the `ade_testkit::mempool::ingress_replay` harness — a
   single-step replay fold over `mempool_ingress` (per OQ-6, no
   batching, no out-of-order interleaving) plus reuse of the
   PHASE4-B2 B-track adversarial corpus via `wrap_as_ingress` /
   `b_track_corpus_as_ingress`; the harness never calls `admit`
   directly. New rule `DC-MEM-04` (enforced) with its own CI gate
   `ci_check_mempool_ingress_replay.sh` (harness-shape + no-batching
   guards). `DC-MEM-01` strengthened in-place (`strengthened_in +=
   PHASE4-N-E`; +3 ingress-replay tests; bidirectional
   `cross_ref += DC-MEM-04`). 5 new integration tests (CE-N-E-2,
   CE-N-E-5, single-peer CE-N-E-4, dependent-pair N-E-6,
   source-invariance N-E-8). **S3** (`509d714`, GREEN) added
   `crates/ade_ledger/src/mempool/canonicalize.rs` — a deterministic
   per-peer round-robin canonicalizer (peers visited in byte-lex
   `PeerId` order, source-byte tie-break; pure, sync, no I/O). Two
   distinct concurrent interleavings of the same per-peer queues
   produce byte-identical `IngressEvent` sequences, closing the
   multi-peer half of CE-N-E-4. The S2 CI gate was extended with two
   new clauses (canonicalize.rs presence + no-`HashMap`/-`tokio`/-RNG
   body scan). `DC-MEM-04` extended in-place (`code_locus`/`tests`
   appended). 9 new unit tests + 2 new integration tests. **S4**
   (`ca3f23a`, GREEN) added
   `crates/ade_core_interop/src/tx_submission.rs` — a GREEN bridge
   from N-A `InventoryEvent` (tx-submission2 state-machine output)
   into N-E `mempool_ingress` under `IngressSource::N2N`, with
   per-peer accumulation (`PeerAccumulator`) and the
   `ingest_n2n_events(base, per_peer)` orchestrator. The bridge
   calls `mempool_ingress` (never `admit` directly), preserving the
   chokepoint contract. **Cargo edge added**: `ade_core_interop` now
   depends directly on `ade_ledger` (was transitive). 7 new
   integration tests (CE-N-E-6 adapter-layer agreement +
   multi-peer canonicalization + source-carry-through). The
   load-bearing CE-N-E-6 mechanical half is closed; the live-log
   half is operator-action per
   `docs/clusters/PHASE4-N-E/CE-N-E-6_PROCEDURE.md`. **S5**
   (`43fcc31`, GREEN) added
   `crates/ade_core_interop/src/local_tx_submission.rs` — the N2C
   mirror of S4's N2N shape, over the cardano-cli IPC transport
   under `IngressSource::N2C`. The load-bearing CE-N-E-7 mechanical
   evidence is the cross-bridge agreement test
   `n2n_and_n2c_bridges_produce_identical_outcomes`: the same tx
   bytes routed via `ingest_n2n_events` vs `ingest_n2c_events`
   produce byte-identical `(MempoolState, Vec<AdmitOutcome>)`,
   closing the source-invariance property (N-E-N7) at the
   wire-event layer. 8 new integration tests. CE-N-E-7 mechanical
   half closed; live-log half operator-action per
   `docs/clusters/PHASE4-N-E/CE-N-E-7_PROCEDURE.md`. **No new BLUE
   crate, no new RED crate.** **Registry: 175 entries at HEAD (was
   173); +2 new rules (`DC-MEM-03`, `DC-MEM-04`); 0 removed.**
   **CI: 31 scripts at HEAD (was 29); +2 new scripts.** Cluster
   status: code + harness complete, live evidence pending; cluster
   dir at `docs/clusters/PHASE4-N-E/` (NOT yet archived).
1. **Post-WRITEBACK testkit follow-ups (four commits, GREEN-scope) —
   carried forward.** `b9cfaf9` real-chain committee oracle at
   mainnet 575→576; `396664a` corpus-alignment; `c78ec76`
   `reward_provenance` generator; `168ac02` snapshot-loader
   follow-ups (tip slot + Conway UMElem). `DC-EPOCH-01` /
   `DC-LEDGER-10` each gained one oracle test; committee-CHANGE
   reclassified reality-blocked. No new module/rule/CI script.
2. **ENACTMENT-COMMITTEE-WRITEBACK — closed.** Wires committee
   enactment write-back; structured `UpdateCommittee` replaces the
   opaque `{ prev_action, raw: Vec<u8> }`. `DC-EPOCH-01` and
   `DC-LEDGER-10` both STRENGTHENED. No new module/rule/CI script;
   the existing OQ5 gate was extended (section 7).
3. **ENACTMENT-COMMITTEE-FIDELITY — closed.** `EnactmentEffects.
   committee_changes` re-typed `Hash28` → `StakeCredential`. Dormant
   at the FIDELITY close; LIVE after WRITEBACK.
4. **DREP-VOTE-FIDELITY — closed.** `GovActionState.drep_votes`
   re-typed; exact-variant DRep resolution (no OR-fallback).
5. **COMMITTEE-CRED-FIDELITY — closed.** `ConwayGovState.committee`
   re-keyed `Hash28` → `StakeCredential`; `GovActionState.
   committee_votes` re-typed.
6. **OQ5-CREDENTIAL-FIDELITY — closed.** `StakeCredential` tuple
   struct → closed enum `{ KeyHash, ScriptHash }`; both era decoders
   preserve the tag. `DC-LEDGER-10` introduced + `enforced` via
   the new CI gate.
7. **Phase 4 cluster B5 (Conway gov-cert accumulation) — closed.**
   New BLUE module `ade_ledger::gov_cert`; `DC-LEDGER-09` introduced
   + `enforced`.
8. **Phase 4 cluster B4 (Conway cert-state accumulation,
   fail-closed) — closed.** Owner-complete Conway cert decoder;
   `DC-LEDGER-08` introduced + `enforced`.
9. **Phase 4 cluster B3F (follow-up hardening) — committed.** Flips
   `DC-TXV-06` `partial` → `enforced`; hardens `decode_conway_certs`.
10. **Phase 4 cluster B3 (Conway value-conservation accounting) —
    closed.** Full Conway preservation-of-value equation enforced;
    B2-S4 early-out removed. New BLUE surfaces
    `ade_codec::conway::cert`, `ade_codec::conway::withdrawals`,
    `ade_ledger::cert_classify`. Rules `DC-TXV-06` (flipped) and
    `DC-TXV-07`.
11. **Phase 4 cluster B2 (tx validity agreement) — closed.** New
    BLUE `ade_ledger::tx_validity` submodule + BLUE/GREEN
    `ade_ledger::mempool` admission gate. Added 5 `DC-TXV-*` rules;
    flipped the two `DC-MEM-*` to `enforced`.
12. **Phase 4 cluster B1 (full block validity agreement) — closed.**
    Composes N-A wire + N-B consensus header authority + ledger
    body authority into a single block verdict. New BLUE
    `ade_ledger::block_validity` submodule.
13. **Phase 4 cluster N-A (network mini-protocols) — closed.** 10
    slices. New BLUE crate `ade_network`.
14. **Phase 4 cluster N-B (consensus runtime) — closed.** 10
    slices. New BLUE `ade_core::consensus` module.
15. **CE-N-B-6 follow-mode bridge.** RED `ade_core_interop::follow`
    + live preprod tip-agreement evidence.
16. **Phase 4 cluster N-D (ChainDB persistence) — closed.**
    Slices S-33 → S-37.
17. **Phase 2C close-out / CE-73 reclassification.** CE-73 split
    Tier-2 / Tier-4.
18. **IDD canonicalization.** `chore(idd)` commits.
19. **Grounding-doc generation + ripple.** Successive refreshes,
    including `52642e5` (which archived 7 closed cluster dirs).
20. **BLUE-list drift closure.** Six CI scripts extended to full
    BLUE scope.
21. **Corpus relayout.** Credentialed `*_registered_creds.txt`
    removed (~7M-line negative); `corpus/snapshots/` now
    `.gitignore`-d (canonical home `s3://ade-corpus-snapshots`);
    `emit_reward_provenance` generator committed.

---

## 1. Commit Log

| Hash | Type | Summary |
|------|------|---------|
| `43fcc31` | feat | feat(interop): N2C local-tx-submission -> mempool_ingress bridge (PHASE4-N-E S5) |
| `ca3f23a` | feat | feat(interop): N2N tx-submission2 -> mempool_ingress bridge (PHASE4-N-E S4) |
| `509d714` | feat | feat(ledger): per-peer ingress canonicalizer (PHASE4-N-E S3) |
| `2d0c918` | test | test(testkit): mempool ingress-replay harness + B-track corpus reuse (PHASE4-N-E S2) |
| `32c1ee6` | feat | feat(ledger): IngressEvent + mempool_ingress closed chokepoint (PHASE4-N-E S1) |
| `52642e5` | docs | docs(grounding): refresh CODEMAP/SEAMS/HEAD_DELTAS/TRACEABILITY + archive 7 closed cluster dirs |
| `168ac02` | fix | fix(testkit): snapshot-loader follow-ups (tip slot + Conway UMElem) |
| `c78ec76` | test | test(corpus): add reward_provenance generator (re-runnable, ignored) |
| `396664a` | test | test(corpus): align previously-blocked ade_testkit tests + ade_plutus compile with regenerated corpus |
| `b9cfaf9` | test | test(ledger): real-chain committee oracle, mainnet 575->576 (strengthens DC-EPOCH-01 + DC-LEDGER-10) |
| `3d94c22` | docs | docs(grounding): refresh CODEMAP/SEAMS/HEAD_DELTAS/TRACEABILITY + strengthen DC-EPOCH-01/DC-LEDGER-10 for ENACTMENT-COMMITTEE-WRITEBACK close |
| `69e2d4b` | test | test(ledger): harden update_committee decode + extend credential gate (ENACTMENT-COMMITTEE-WRITEBACK close) |
| `3180e27` | feat | feat(ledger): wire committee enactment write-back (ENACTMENT-COMMITTEE-WRITEBACK-S2) |
| `f2f15f9` | feat | feat(ledger): structured UpdateCommittee gov action (ENACTMENT-COMMITTEE-WRITEBACK-S1) |
| `ea25dd9` | docs | docs(ledger): ENACTMENT-COMMITTEE-WRITEBACK plan (wire committee enactment) |
| `3706534` | docs | docs(grounding): refresh CODEMAP/SEAMS/HEAD_DELTAS/TRACEABILITY for ENACTMENT-COMMITTEE-FIDELITY close |
| `a6b8de7` | feat | feat(ledger): discriminate EnactmentEffects.committee_changes (ENACTMENT-COMMITTEE-FIDELITY-S1) |
| `5d64fee` | docs | docs(ledger): ENACTMENT-COMMITTEE-FIDELITY plan (strengthens DC-LEDGER-10) |
| `06f517f` | docs | docs(grounding): refresh CODEMAP/SEAMS/HEAD_DELTAS/TRACEABILITY for DREP-VOTE-FIDELITY close |
| `62c9020` | test | test(ledger): DRep cross-resolve negative + CI gate, strengthen DC-LEDGER-10 (DREP-VOTE-FIDELITY-S2) |
| `ba4ff37` | feat | feat(ledger): discriminate drep_votes; exact-variant DRep stake resolution (DREP-VOTE-FIDELITY-S1) |
| `ecb0b92` | docs | docs(ledger): DREP-VOTE-FIDELITY plan (strengthens DC-LEDGER-10) |
| `a157c92` | docs | docs(grounding): refresh CODEMAP/SEAMS/HEAD_DELTAS/TRACEABILITY for COMMITTEE-CRED-FIDELITY close |
| `2aeea16` | test | test(ledger): committee cross-resolve negative + CI gate, strengthen DC-LEDGER-10 (COMMITTEE-CRED-FIDELITY-S2) |
| `2303a60` | feat | feat(ledger): discriminate committee member + vote credentials (COMMITTEE-CRED-FIDELITY-S1) |
| `32d7a2e` | docs | docs(ledger): COMMITTEE-CRED-FIDELITY plan (strengthens DC-LEDGER-10) |
| `676af5a` | docs | docs(grounding): refresh CODEMAP/SEAMS/HEAD_DELTAS/TRACEABILITY for OQ5 close |
| `a3ee2da` | test | test(ledger): credential-fidelity corpus + CI gate, enforce DC-LEDGER-10 (OQ5-S2) |
| `4187330` | feat | feat(types): discriminated StakeCredential end-to-end — preserve key/script tag (OQ5-S1) |
| `007b0e8` | docs | docs(ledger): OQ5-CREDENTIAL-FIDELITY cluster plan + cluster doc |
| `959e16c` | docs | docs(ledger): OQ-5 credential-fidelity invariants + DC-LEDGER-10 (declared) |
| `f81f815` | docs | docs(grounding): refresh CODEMAP/SEAMS/HEAD_DELTAS/TRACEABILITY for PHASE4-B5 close |
| `651adc9` | fix | fix(ledger): checked DRep-expiry arithmetic, deterministic fail-closed on overflow (PHASE4-B5-S5) |
| `06385d0` | test | test(ledger): gov-state accumulation corpus + CI gate, enforce DC-LEDGER-09 (PHASE4-B5-S4) |
| `d63c700` | feat | feat(ledger): apply gov-cert accumulation in block path, carry gov_state forward (PHASE4-B5-S3) |
| `7a48727` | feat | feat(ledger): native Conway gov-cert apply model — apply_conway_gov_cert (PHASE4-B5-S2) |
| `9c8d118` | feat | feat(ledger): gov-cert env infrastructure — drep_activity + GovCertEnv fail-fast (PHASE4-B5-S1) |
| `fdb6601` | docs | docs(gov): PHASE4-B5 invariants + cluster plan + DC-LEDGER-09 (Conway gov-cert accumulation) |
| `644eb03` | docs | docs(grounding): refresh CODEMAP/SEAMS/HEAD_DELTAS/TRACEABILITY for PHASE4-B4 close |
| `ee35493` | test | test(ledger): Conway cert-state accumulation corpus (PHASE4-B4-S5) |
| `302d22c` | feat | feat(ledger): era-dispatched fail-closed cert-state accumulation (PHASE4-B4-S3/S4) |
| `da30706` | feat | feat(ledger): native owner-tagged Conway cert apply model (PHASE4-B4-S2) |
| `228415b` | feat | feat(codec): owner-complete Conway certificate decoder (PHASE4-B4-S1) |
| `ae1300a` | docs | docs(planning): PHASE4-B4 grounding — invariants, cluster plan, cluster doc, B4-S1 slice (DC-LEDGER-08) |
| `1d989de` | docs | docs(grounding): refresh CODEMAP/TRACEABILITY/SEAMS/HEAD_DELTAS for PHASE4-B3F |
| `193d2fc` | feat | feat(codec): Conway cert decoder strictness — reject trailing bytes, bound preallocation (PHASE4-B3F) |
| `d6c1993` | feat | feat(ci): DC-TXV-06 cert-classification closure gate — flip partial to enforced (PHASE4-B3F) |
| `d766eb0` | chore | Close PHASE4-B3 — full Conway tx value-conservation accounting |
| `7784bf8` | test | test(tx-validity): PHASE4-B3 conservation corpora — real epoch-576 positive + adversarial no-false-accept |
| `978c222` | feat | feat(tx-validity): PHASE4-B3 full Conway value-conservation accounting — remove the cert/withdrawal early-out |
| `3aebbe5` | docs | docs(phase4-b3): invariants, cluster/slice plan, and registry rules for Conway value-conservation accounting |
| `c1cba82` | chore | chore(phase-4): close PHASE4-B2 — tx-validity agreement + mempool admission, grounding-doc refresh |
| `85a50dc` | feat | feat(tx-validity): B2-S5 mempool admission gate (Tier-1) — closes CE-B2-5 |
| `617139f` | feat | feat(tx-validity): B2-S4 adversarial tx corpus — closes CE-B2-4 (no false accept) + fixes a value-conservation fail-open |
| `4cffc2c` | feat | feat(tx-validity): B2-S3 positive tx corpus + replay — closes CE-B2-3 |
| `b24b22c` | feat | feat(tx-validity): B2-S2 tx_validity composition + verdict taxonomy — closes CE-B2-2 |
| `3e24d0b` | feat | feat(tx-validity): B2-S1 Conway vkey-witness + required-signer closure — closes CE-B2-1 |
| `7263699` | docs | docs(phase-4): PHASE4-B2 cluster doc — tx validity agreement |
| `b32fef3` | docs | docs(phase-4): PHASE4-B2 cluster/slice plan — 5-slice tx-validity-agreement arc |
| `b79f632` | docs | docs(phase-4): open PHASE4-B2 — tx validity agreement invariant sketch + DC-TXV family |
| `e0af99d` | chore | chore: gitignore multi-GB ledger-state dumps (belong in S3, not git) |
| `3552bc2` | chore | chore: sync Cargo.lock for PHASE4-B1 dependency edges |
| `993f363` | chore | Close PHASE4-B1 — full block validity agreement (validation core of workstream B) |
| `2630267` | feat | feat(validity): B1-S7 adversarial corpus — closes CE-B1-4 (no false accept) |
| `e394a82` | feat | feat(validity): B1-S6 positive agreement corpus + replay — closes CE-B1-3 |
| `7b95ccd` | feat | feat(validity): B1-S4 block_validity composition — closes CE-B1-2 + CE-B1-5 |
| `500589b` | feat | feat(validity): B1-S5 Praos single-VRF + KES header validation — 14/14 real Conway headers validate |
| `440ac72` | feat | feat(validity): B1-S3 BlockValidity verdict/error taxonomies + canonical surface encoding |
| `97a27cc` | feat | feat(validity): B1-S2 production LedgerView projection — closes CE-B1-1 |
| `a134379` | feat | feat(validity): B1-S1 consensus-input extractor + Conway-576 corpus |
| `b63f554` | docs | docs(phase-4): PHASE4-B1 cluster doc — full block validity agreement |
| `cb8165a` | docs | docs(phase-4): PHASE4-B1 cluster/slice plan — 7-slice full-block-validity arc |
| `c0acd59` | docs | docs(phase-4): open PHASE4-B1 — full block validity agreement invariant sketch + DC-VAL registry family |
| `e5f1f64` | feat | feat(interop): CE-N-B-6 follow-mode bridge + live preprod tip-agreement evidence |
| `807bcb6` | docs | docs(consensus): retarget N-B live-interop pin to cardano-node 11.0.1 |
| `a0c73e1` | chore | Close PHASE4-N-B — consensus runtime (Praos) authority + replay equivalence |
| `ad4d6f6` | feat | feat(consensus): S-B10 stream replay + orchestrator + live interop — closes CE-N-B-5 + CE-N-B-6 |
| `4f5cd7f` | feat | feat(consensus): S-B9 rollback authority — closes CE-N-B-2 |
| `8e991b5` | feat | feat(consensus): S-B8 fork choice + CandidateFragment — closes CE-N-B-1 |
| `e059652` | feat | feat(consensus): S-B7 Praos header validation |
| `f4c8369` | feat | feat(consensus): S-B6 leader schedule — closes CE-N-B-4 |
| `39cc143` | feat | feat(consensus): S-B5 op-cert counter monotonicity |
| `116eb57` | feat | feat(consensus): S-B4 nonce evolution authority |
| `70f60d9` | feat | feat(consensus): S-B3 VRF cert verification wiring + Praos VRF input + leader threshold |
| `ff01fe3` | feat | feat(consensus): S-B2 PraosChainDepState canonical type + closed event/error taxonomies |
| `fe68bb7` | feat | feat(consensus): S-B1 EraSchedule canonical authority + slot/era/time translation |
| `744ef34` | chore | chore(phase-4): complete PHASE4-N-A close — DoS hardening + grounding doc refreshes |
| `d9f0426` | docs | docs(phase-4): PHASE4-N-B invariant sketch v2 + 8 new DC-CONS-* registry rules |
| `69a2862` | chore | Close PHASE4-N-A — Ouroboros mini-protocols (11) wire-grammar conformance + state-machine determinism + real-interop validation |
| `56bfa7b` | feat | feat(phase-4): close CE-N-A-5 — 4 N2C real captures + LSQ/LTS/TxSubmission2 wire-form fixes + condition 4 + 5 + S-A10 evidence script |
| `d977640` | docs | docs(registry): wire S-A9 real-capture tests into PHASE4-N-A invariants |
| `b7cd39d` | feat | feat(phase-4): S-A9 N2C handshake + N2N keep-alive + peer-sharing real captures (3 more protocols + N2C 0x8000 wire-flag fix) |
| `a1b47ec` | feat | feat(phase-4): S-A9 block-fetch real interop + flat-range wire-form fix |
| `ef38212` | feat | feat(phase-4): S-A9 block-fetch codec wrapping fix + capture binary |
| `84d3eab` | feat | feat(phase-4): S-A9 chain-sync real capture + ChainSync codec wrapped-header fix |
| `98d0abe` | feat | feat(phase-4): S-A9 partial — real-capture corpus + handshake against mainnet relays |
| `1ba2d95` | feat | feat(phase-4): S-A8c — version table alignment with cardano-node 11.0.1 |
| `679491f` | docs | docs(phase-4): S-A8c entry obligation discharge — version table alignment with cardano-node 11.0.1 |
| `b7fade3` | feat | feat(phase-4): S-A8b — LocalTxMonitor wire-grammar rework (corrects S-A2/S-A8 misimpl) |
| `affa624` | docs | docs(phase-4): S-A8b entry obligation discharge — LocalTxMonitor wire-grammar rework |
| `9b7b96d` | docs | docs(phase-4): S-A9 + S-A10 entry obligation discharge — corpus replay harness + live interop closure gate |
| `77a02dd` | feat | feat(phase-4): S-A8 — N2C transition authority (4 state machines; structural completion) |
| `20b3554` | docs | docs(phase-4): S-A8 entry obligation discharge — N2C transition authority (4 state machines) |
| `b16329b` | feat | feat(phase-4): S-A7 — keep-alive + peer-sharing transition authority (structural completion) |
| `2cb0e86` | docs | docs(phase-4): S-A7 entry obligation discharge — keep-alive + peer-sharing transition authority |
| `844ae95` | feat | feat(phase-4): S-A6 — tx-submission2 transition authority (closes CE-N-A-4 state-machine portion) |
| `10659d5` | docs | docs(phase-4): S-A6 entry obligation discharge — tx-submission2 transition authority |
| `d702772` | feat | feat(phase-4): S-A5 — block-fetch transition authority (closes CE-N-A-3 state-machine portion) |
| `7078b9b` | docs | docs(phase-4): S-A5 entry obligation discharge — block-fetch transition authority |
| `787da55` | feat | feat(phase-4): S-A4 — chain-sync transition authority (closes CE-N-A-2 state-machine portion) |
| `7fef3a4` | docs | docs(phase-4): S-A4 entry obligation discharge — chain-sync transition authority |
| `ba02f71` | feat | feat(phase-4): S-A3 — handshake version negotiation authority (closes CE-N-A-1 state-machine portion) |
| `6faacd0` | docs | docs(phase-4): S-A3 entry obligation discharge — handshake version negotiation authority |
| `d1d47e9` | feat | feat(phase-4): S-A2 — protocol message codec authority for all 11 mini-protocols |
| `a4aabb9` | docs | docs(phase-4): S-A2 entry obligation discharge — protocol codec authority for all 11 mini-protocols |
| `4fde3a7` | feat | feat(phase-4): S-A1 — ade_network substrate + DC-CORE-01 mechanical gate |
| `22023be` | docs | docs(phase-4): S-A1 entry obligation discharge — mux/framing + sync-only CI gate |
| `6942674` | docs | docs(phase-4): open PHASE4-N-A cluster doc — wire+semantic Tier 1, 10 slices |
| `6ca2ba8` | docs | docs(phase-4): ratify PHASE4-N-A cluster plan (10 slices, authority-aligned) |
| `ae9c473` | docs | docs(phase-4): close N-A invariants §7 decisions + add DC-PROTO-06 |
| `492de56` | docs | docs(phase-4): open PHASE4-N-A — invariant sketch + DC-CORE-01 sync-only rule |
| `436b1d7` | chore | Close PHASE4-N-D — chain DB persistence with crash-equivalent recovery |
| `a3a083a` | docs | docs(phase-4): CE-N-D-1 closure evidence — 1000/1000 stress kill iterations green |
| `27960fd` | docs | docs(phase-4): lock N-A scope decisions before cluster opens |
| `a2c7ac8` | chore | chore(idd): refresh CODEMAP + TRACEABILITY + HEAD_DELTAS after N-D CI closure |
| `78da6c9` | chore | chore(ci): close Phase 4 N-D CI gap — 3 new scripts, 9 rules enforced |
| `f0b0fd6` | chore | chore(idd): refresh HEAD_DELTAS + SEAMS to align with BLUE-scope closure |
| `c8fa37f` | chore | chore(idd): refresh CODEMAP + TRACEABILITY after BLUE-list drift closure |
| `5b70bee` | chore | chore(ci): close BLUE-list drift — extend 6 CI scripts to full BLUE scope |
| `a87c3a3` | chore | chore(idd): generate four grounding docs (CODEMAP, SEAMS, HEAD_DELTAS, TRACEABILITY) |
| `3eddcbb` | chore | chore(idd): add .idd-config.json — opt the repo into IDD enforcement |
| `76c1f64` | chore | chore(idd): move in-flight cluster N-D into canonical clusters layout |
| `39865f6` | chore | chore(idd): update active-doc + CI refs to canonical registry path |
| `2047c42` | chore | chore(idd): commit-msg hook + CLAUDE.md trailer-override note |
| `5eecc8a` | feat | feat(phase-4): snapshot + forward-replay recovery (S-36) |
| `e52fe9f` | feat | feat(phase-4): SnapshotStore trait + impls (S-35) |
| `fb4a5d4` | feat | feat(phase-4): persistent ChainDb backed by redb (S-34) |
| `994203b` | feat | feat(phase-4): begin cluster N-D — ChainDb trait + InMemoryChainDb (S-33) |
| `9b15378` | feat | feat(phase-2c): reclassify CE-73 — semantic enforced, bytes Tier 4 non-goal |

Verbatim from `git log d509f02..HEAD` (`--no-merges`; history is
linear, no merge commits in range). Aggregation is in §3 and §5.

---

## 2. New Modules

| Module | Color | Purpose | Key sub-paths | Added in (cluster/slice) |
|--------|-------|---------|---------------|--------------------------|
| `ade_ledger::mempool::ingress` (new file in an existing BLUE crate) | BLUE | **Single closed wire-level ingress chokepoint.** `IngressEvent { source: IngressSource, tx_bytes: Vec<u8> }` is the canonical entry; `IngressSource::{N2N, N2C}` is a closed 2-variant sum (no `#[non_exhaustive]`). `mempool_ingress(state, event) -> (state', AdmitOutcome)` is a pure, sync pass-through to `admit` over `event.tx_bytes` — the `source` variant is metadata only and MUST NOT change the verdict. Enforced by `ci_check_mempool_ingress_closure.sh` (5 mechanical guards) and `DC-MEM-03`. | `mempool/ingress.rs` | PHASE4-N-E / S1 (`32c1ee6`) |
| `ade_ledger::mempool::canonicalize` (new file in an existing BLUE crate) | GREEN | **Deterministic per-peer ingress canonicalizer.** Takes per-peer FIFO submission queues and produces a single canonical `Vec<IngressEvent>` stream; round-robin by sorted `PeerId` (byte-lex), source-byte tie-break; pure, sync, no I/O. Two distinct concurrent interleavings of the same per-peer queues canonicalize to byte-identical sequences (CE-N-E-4 multi-peer half). | `mempool/canonicalize.rs` (`PeerId`, `PeerSubmissionQueue`, `canonicalize_peer_streams`, `source_byte`) | PHASE4-N-E / S3 (`509d714`) |
| `ade_testkit::mempool::ingress_replay` (new submodule of an existing crate) | GREEN | **Single-step ingress-replay harness** over the existing B-track adversarial corpus. `replay_ingress_trace(base, &[IngressEvent]) -> (MempoolState, Vec<AdmitOutcome>)` folds `mempool_ingress` step-by-step (per OQ-6 — no batching, no out-of-order interleaving) and never calls `admit` directly. Exports `ExpectedOutcome`, `BTrackCase`, `wrap_as_ingress`, `b_track_corpus_as_ingress`. Enforced by `ci_check_mempool_ingress_replay.sh` (4 mechanical guards) and `DC-MEM-04`. | `mempool/mod.rs`, `mempool/ingress_replay.rs` | PHASE4-N-E / S2 (`2d0c918`) |
| `ade_core_interop::tx_submission` (new file in an existing RED crate) | GREEN | **N2N tx-submission2 → `mempool_ingress` bridge.** `event_to_ingress(&InventoryEvent, IngressSource)` maps `TxsDelivered.tx_bytes → Vec<IngressEvent>` (all other inventory events yield empty); `PeerAccumulator` accumulates per-peer; `ingest_n2n_events(base, per_peer)` orchestrates over `replay_ingress_trace` (calls the chokepoint, never `admit` directly). Pure, no I/O. | `src/tx_submission.rs`; `tests/tx_submission_ingress.rs` (7 integration tests); operator procedure at `docs/clusters/PHASE4-N-E/CE-N-E-6_PROCEDURE.md` | PHASE4-N-E / S4 (`ca3f23a`) |
| `ade_core_interop::local_tx_submission` (new file in an existing RED crate) | GREEN | **N2C local-tx-submission → `mempool_ingress` bridge.** N2C mirror of S4 over the cardano-cli IPC transport. `local_event_to_ingress(&LocalTxSubmissionEvent)` maps `TxSubmitted → N2C IngressEvent` (other events empty); `ClientAccumulator` accumulates per-client; `ingest_n2c_events(base, per_client)` orchestrates. Cross-bridge agreement at the wire-event layer is the load-bearing CE-N-E-7 evidence (`n2n_and_n2c_bridges_produce_identical_outcomes`). | `src/local_tx_submission.rs`; `tests/local_tx_submission_ingress.rs` (8 integration tests); operator procedure at `docs/clusters/PHASE4-N-E/CE-N-E-7_PROCEDURE.md` | PHASE4-N-E / S5 (`43fcc31`) |
| `ade_codec::conway::cert` (new file in an existing BLUE crate) | BLUE | **Conway-complete certificate decoder** with a *closed* wire grammar. `decode_conway_certs` decodes the full Conway certificate array over tags `0..18`; tags `5`/`6` are not valid; unrecognized tag → deterministic `CodecError::UnknownCertTag { tag, offset }` reject. **B3F-S2 hardened it**: trailing-byte reject (`CodecError::TrailingBytes`), bounded preallocation. | `conway/cert.rs` | PHASE4-B3 / B3-S1, B3-S2; strictness PHASE4-B3F / B3F-S2 |
| `ade_codec::conway::withdrawals` (new file in an existing BLUE crate) | BLUE | Conway withdrawals-map decoder. Decodes `{ RewardAccount => Coin }` into a canonical ordered form, summing to an `i128` consumed-side term; duplicate key → `CodecError::DuplicateMapKey { offset }`. | `conway/withdrawals.rs` | PHASE4-B3 / B3-S3 |
| `ade_ledger::cert_classify` (new file in an existing BLUE crate) | BLUE | **Closed cert-deposit classification** — `classify(state, cert)` is total, era-versioned, resolves every cert variant to exactly one `CertDisposition` over `DepositEffect` with coin sourced via a closed `CoinSource`. **B3F-S1** added the CI grep-gate guarding `classify`'s exhaustiveness. | `cert_classify.rs` | PHASE4-B3 / B3-S2; closure gate B3F / B3F-S1 |
| `ade_ledger::gov_cert` (new file in an existing BLUE crate) | BLUE | **Native Conway governance-certificate accumulation**. `apply_conway_gov_cert(gov_state, cert, env)` is a pure, total dispatch over the owner-complete `ConwayCert`; mutates **only** governance-owned fields of `ConwayGovState`. `GovCertEnv` is required only by tags 16/18; absent `drep_activity` is structured fail-fast. | `gov_cert.rs` | PHASE4-B5 / B5-S2; B5-S1 env; B5-S3 block-path; B5-S5 checked arithmetic |
| `ade_ledger::tx_validity` (new submodule of an existing BLUE crate) | BLUE | **Per-transaction verdict authority**. Closed `TxValidityVerdict` / `TxRejectClass` / `TxValidityError`. `required_signers` enumerates over a closed `SignerSource`. `tx_phase_one` composes witness closure + state-backed checks; `tx_validity` is the pure transition. | `mod.rs`, `verdict.rs`, `required_signers.rs`, `witness.rs`, `phase1.rs`, `transition.rs`, `encoding.rs` | PHASE4-B2 / B2-S1, B2-S2 |
| `ade_ledger::mempool` (new submodule of an existing BLUE crate) | BLUE (`admit` + `ingress`) / GREEN (`policy` + `canonicalize`) | Two-layer mempool: BLUE `admit` requires `tx_validity` Valid; GREEN `policy` does eviction/ordering, never calls `tx_validity`. **PHASE4-N-E added the BLUE `ingress` chokepoint and the GREEN `canonicalize` ordering function** (rows above). | `mod.rs`, `admit.rs`, `policy.rs`, `ingress.rs` (N-E S1), `canonicalize.rs` (N-E S3) | PHASE4-B2 / B2-S5; PHASE4-N-E / S1, S3 |
| `ade_testkit::tx_validity` (new submodule of an existing crate) | GREEN | Test-only tx-validity harness — extractor, synthetic builders, W1–W4 / S1–S4 mutators + judge. Non-authoritative. | `tx_validity/mod.rs`, `tx_validity/extract.rs`, `tx_validity/valid_synthetic.rs`, `tx_validity/adversarial.rs`; B3 example bins | PHASE4-B2 / B2-S3, B2-S4; B3 extensions |
| `ade_ledger::block_validity` (new submodule of an existing BLUE crate) | BLUE | Full-block verdict authority: closed `BlockValidityVerdict`, closed `BlockValidityError` / `BlockRejectClass`, fail-closed taxonomy, the `block_validity(...)` transition. Canonical `VerdictSurface`. | `mod.rs`, `verdict.rs`, `transition.rs`, `header_input.rs`, `encoding.rs` | PHASE4-B1 / B1-S3, B1-S4 |
| `ade_ledger::consensus_view` (new file in an existing BLUE crate) | BLUE | Production `LedgerView` projection — projects pool-distribution into the four leadership-relevant facts BLUE consensus consumes. | `consensus_view.rs` | PHASE4-B1 / B1-S2 |
| `ade_ledger::consensus_input_extract` (new file in an existing BLUE crate) | RED | Tail-scan of a snapshot `state` CBOR for the five `PraosState` nonces. RED because it parses an external dump format. | `consensus_input_extract.rs` | PHASE4-B1 / B1-S1 |
| `ade_core::consensus::kes_check` (new file in an existing BLUE crate) | BLUE | Fail-closed wiring of `ade_crypto::kes` into Praos header validation. | `kes_check.rs` | PHASE4-B1 / B1-S5 |
| `ade_testkit::validity` (new submodule of an existing crate) | GREEN | Test-only block-validity harness: positive Conway-576 replay, corpus-backed `LedgerView`, M1–M6 mutators. | `validity/mod.rs`, `validity/corpus.rs`, `validity/ledger_view.rs`, `validity/replay.rs`, `validity/adversarial.rs` | PHASE4-B1 / B1-S6, B1-S7 |
| `ade_core_interop::follow` (new file in an existing RED crate) | RED | Follow-mode bridge — BLUE `select_best_chain` + `apply_rollback` only; no authoritative decision. | `follow.rs`, `tests/follow_offline_replay.rs` | CE-N-B-6 (`e5f1f64`) |
| `ade_network` (new workspace crate) | BLUE-majority (per-submodule scoped) | Ouroboros mini-protocol authority: 11 closed-grammar codecs, 8 transition state machines, mux frame codec, RED session/transport substrate. | `codec/`, `handshake/`, `chain_sync/`, `block_fetch/`, `tx_submission/`, `keep_alive/`, `peer_sharing/`, `n2c/`, `mux/frame.rs` (BLUE), `mux/transport.rs` (RED), `session/` (RED) | PHASE4-N-A / S-A1 → S-A10 |
| `ade_core::consensus` (new submodule of an existing BLUE crate) | BLUE | Praos consensus authority: closed `PraosChainDepState`, era-aware translation, header validation, nonce evolution, op-cert monotonicity, leader schedule, fork choice, rollback. | `mod.rs`, `era_schedule.rs`, `header_validate.rs`, `vrf_cert.rs`, `nonce.rs`, `op_cert.rs`, `leader_schedule.rs`, `fork_choice.rs`, `rollback.rs`, `kes_check.rs` (B1), `praos_state.rs`, `candidate.rs`, `events.rs`, `errors.rs`, `encoding.rs`, `ledger_view.rs`, `header_summary.rs` | PHASE4-N-B / S-B1 → S-B9 |
| `ade_runtime::consensus` (new submodule of an existing RED crate) | GREEN/RED mix | Imperative-shell composition: stream-driven orchestrator (GREEN), candidate-fragment builder, RED genesis parser. | `mod.rs`, `candidate_fragment.rs`, `chain_selector.rs`, `genesis_parser.rs` | PHASE4-N-B / S-B8, S-B10 |
| `ade_core_interop` (new workspace crate) | RED | Live cardano-node interop driver for CE-N-B-6 + N-E S4/S5 wire-event bridges; no authoritative decisions. | `src/lib.rs`, `src/follow.rs`, `src/tx_submission.rs` (N-E S4), `src/local_tx_submission.rs` (N-E S5), `src/bin/live_consensus_session.rs`, `tests/` | PHASE4-N-B / S-B10; follow-bridge `e5f1f64`; PHASE4-N-E / S4, S5 |
| `ade_testkit::consensus` (new submodule of an existing crate) | GREEN | Test-only harness for consensus replay corpora. | `consensus/mod.rs`, `consensus/corpus.rs`, `consensus/ledger_view_stub.rs`, `consensus/stream_replay.rs` | PHASE4-N-B / S-B1, S-B6, S-B8 → S-B10 |
| `ade_runtime::chaindb` | RED | Block-store abstraction and impls. Trait surface Tier 1; backing-store choice Tier 5. | `mod.rs`, `types.rs`, `error.rs`, `in_memory.rs`, `persistent.rs` (redb), `contract.rs`, `snapshot_contract.rs`, `crash_safety.rs` | PHASE4-N-D / S-33 → S-35 |
| `ade_runtime::recovery` | RED | Composes ChainDb + SnapshotStore into a generic recovery primitive. | `recovery.rs` | PHASE4-N-D / S-36 |
| `ade_runtime` bin `chaindb_kill_target` | RED | Kill-target child process for the 1,000-kill-9 durability stress harness. | `src/bin/chaindb_kill_target.rs`, `tests/stress_kill_harness.rs` | PHASE4-N-D / S-37 |

Workspace-level membership grew by **two crates** across the full
delta: `ade_network` (PHASE4-N-A) and `ade_core_interop` (PHASE4-N-B).
Both are RED-or-mixed. **PHASE4-N-E added no new crate** — it landed
two new BLUE/GREEN modules under `ade_ledger::mempool`
(`ingress` BLUE, `canonicalize` GREEN), one new GREEN submodule under
`ade_testkit` (`mempool::ingress_replay`), and two new GREEN modules
under the existing `ade_core_interop` crate (`tx_submission`,
`local_tx_submission`). **None of B3, B3F, B4, B5, OQ5, FIDELITY,
WRITEBACK, or the testkit follow-up thread added a new crate either.**

Crate dependency shape at HEAD: **PHASE4-N-E added one new dep edge**
— `ade_core_interop` now depends directly on `ade_ledger` (was
transitive only). `ade_network`, `ade_runtime`, `ade_testkit` edges
are unchanged. No edge from a BLUE crate to a RED crate was introduced
(the new bridges live in the RED-crate `ade_core_interop`, and they
*call into* BLUE — that direction is allowed by `ci_check_dependency_boundary.sh`).

Corpora at HEAD: N-A capture corpus, N-B replay corpus, B1 validity
corpus, B3 conservation corpora, B4/B5 README-only synthetic notes,
the credential-fidelity corpus from OQ5-S2, and `corpus/snapshots/`
under `.gitignore` (canonical home `s3://ade-corpus-snapshots`).
**PHASE4-N-E added no new corpus files** — it reuses the existing
PHASE4-B2 B-track adversarial corpus verbatim (per OQ-3), only the
`IngressEvent` envelope is new.

Cross-reference: **The `ade-CODEMAP.md` regenerated in parallel with
this HEAD_DELTAS will record the new mempool ingress chokepoint,
canonicalizer, ingress-replay harness, and the two `ade_core_interop`
bridges; the prior CODEMAP at `52642e5` does NOT yet contain them.**
SEAMS will similarly grow a mempool-ingress-surface row. TRACEABILITY
will record `DC-MEM-03` and `DC-MEM-04` and the
`DC-MEM-01.strengthened_in += "PHASE4-N-E"` strengthening.

---

## 3. Modules Modified

| Module | Scope | Key changes |
|--------|-------|-------------|
| `ade_ledger` | +57 source/test files over the full delta to `168ac02`; **PHASE4-N-E adds 4 files** (`mempool/ingress.rs` +94 / `mempool/canonicalize.rs` +210 / `mempool/mod.rs` +4 re-exports / `tests/mempool_ingress.rs` +178). | **PHASE4-N-E:** new BLUE `mempool::ingress` chokepoint + closed `IngressSource::{N2N, N2C}` + `IngressEvent` + `mempool_ingress` pass-through; new GREEN `mempool::canonicalize` (`PeerId`, `PeerSubmissionQueue`, `canonicalize_peer_streams`, `source_byte`); re-exports from `mempool/mod.rs`. **B3:** the closed cert-deposit classifier `cert_classify.rs` and the full Conway value-conservation accounting in `conway.rs` (cert/withdrawal early-out **REMOVED**); new error variants; `ConwayOnlyDepositParams` + `conway_deposit_view()` in `pparams.rs`/`state.rs`. **B2:** `tx_validity/` + `mempool/` submodules + B2 integration tests; B2-S4 first cut of `check_conway_coin_conservation`. **B1:** `block_validity/`, `consensus_view.rs`, `consensus_input_extract.rs`, the `ade_core` dep edge. **B3F:** no source change (CI grep-gate added). **B4:** `delegation.rs` (+385) native owner-tagged apply model; `rules.rs` (+212) fail-closed `accumulate_tx_certs`; `cert_classify.rs` (+100) re-pointed at owner-complete `ConwayCert`. **B5:** new BLUE module `gov_cert.rs` (+366); `state.rs` (+56) `GovCertEnv` + `gov_cert_env()`; `pparams.rs` (+8) `drep_activity`; `error.rs` (+16) two new variants; `fingerprint.rs` (+14) tag 2→3 array; `rules.rs` (+161/−42). **OQ5:** `state.rs` re-key `Hash28` → `StakeCredential`; `fingerprint.rs` (+78) `write_stake_credential`; ripples across `gov_cert.rs`/`governance.rs`/`cert_classify.rs`/`rules.rs`. **COMMITTEE-CRED-FIDELITY:** re-key committee; `governance.rs` (+76) full-credential-equality ratification. **DREP-VOTE-FIDELITY:** `governance.rs` (+57) exact-variant DRep resolution. **ENACTMENT-COMMITTEE-FIDELITY:** `governance.rs` (+30) `EnactmentEffects.committee_changes` re-typed. **ENACTMENT-COMMITTEE-WRITEBACK:** `governance.rs` (+~189) live `enact_proposals` + `apply_committee_enactment`; `rules.rs` (+~53) epoch-boundary call site; `fingerprint.rs` (+~88) structured `write_gov_action`. |
| `ade_codec` | +11 source/test files (B3 + B3F + B4 + OQ5). **No PHASE4-N-E change.** | **B3:** new `conway::cert` decoder + `conway::withdrawals` decoder; `error.rs` `UnknownCertTag` / `DuplicateMapKey`. **B3F-S2:** trailing-byte reject + bounded preallocation. **B4-S1:** owner-complete `decode_conway_certs`; new `decode_drep`. **OQ5-S1:** both era `decode_stake_credential` preserve the tag. |
| `ade_types` | +3 files (B3) + 2 files (B4) + governance ripples through the FIDELITY clusters. **No PHASE4-N-E change.** | **B3:** closed `ConwayCert` enum + classification types; `RewardAccount`. **B4-S1:** owner-complete `ConwayCert`; new `DRep` enum; `PoolRegistrationCert.owners`. **OQ5-S1:** `StakeCredential` tuple struct → closed enum + `hash()`. **COMMITTEE-CRED / DREP-VOTE / WRITEBACK:** governance-type ripples (committee_votes, drep_votes, structured `UpdateCommittee`). |
| `ade_core` | +29 source files + tests (N-B); +828 / −86 across 16 files (B1). **No post-B1 change.** | **N-B:** substantive BLUE consensus module under `src/consensus/`. **B1:** `consensus/kes_check.rs` + single-VRF + KES wiring. |
| `ade_crypto` | 1 file, +24 / −81 lines (B1). | `kes.rs` (`500589b`): `build_opcert_signable` fixed as part of B1-S5. |
| `ade_core_interop` | +1,546 across 6 files (B1/CE-N-B-6); **PHASE4-N-E adds 4 files** (`src/lib.rs` +2 module registrations; `src/tx_submission.rs` +107; `src/local_tx_submission.rs` +97; `tests/tx_submission_ingress.rs` +192; `tests/local_tx_submission_ingress.rs` +204) plus `Cargo.toml` +1 line (direct `ade_ledger` dep). | **CE-N-B-6:** follow-bridge (`e5f1f64`) + pin retarget (`807bcb6`). **PHASE4-N-E S4 (`ca3f23a`):** N2N `tx_submission` bridge module — `event_to_ingress`, `PeerAccumulator { new/observe/drain/len/is_empty }`, `ingest_n2n_events(base, per_peer)`; 7 integration tests (CE-N-E-6 adapter-layer agreement + multi-peer canonicalization + outcome carry-through). **PHASE4-N-E S5 (`43fcc31`):** N2C `local_tx_submission` bridge module mirroring S4 over cardano-cli IPC — `local_event_to_ingress`, `ClientAccumulator`, `ingest_n2c_events(base, per_client)`; 8 integration tests including the load-bearing CE-N-E-7 `n2n_and_n2c_bridges_produce_identical_outcomes` cross-bridge agreement. Both bridges call `mempool_ingress` (via `replay_ingress_trace`), never `admit` directly. Cargo edge added (`ade_core_interop -> ade_ledger`, was transitive only). |
| `ade_network` | 100 files, +17,861 lines (full N-A). **No PHASE4-N-E change.** | DoS hardening of 6 codecs (`744ef34`, post-N-A close). The N-E bridges live in `ade_core_interop`, not in `ade_network`. |
| `ade_runtime` | +18 files, +3,440 lines (N-B `consensus/` + N-D `chaindb`/`recovery`; B1 one small touch). **No PHASE4-N-E change.** The cluster doc's initial placement of the N2N session loop under `ade_runtime` was inaccurate and is corrected by the S4 commit message: the GREEN bridge lives in `ade_core_interop`; the live socket loop is operator-action per the CE-N-E-6 / CE-N-E-7 procedures. | **N-B:** new `consensus/` submodule. **B1:** one small touch. N-D `chaindb`/`recovery` are §2 New Modules. |
| `ade_testkit` | +28 files across the full delta to `52642e5`; **PHASE4-N-E adds 4 files** (`src/lib.rs` +1 `pub mod mempool;`; `src/mempool/mod.rs` +13; `src/mempool/ingress_replay.rs` +88; `tests/mempool_ingress_replay.rs` +171; `tests/mempool_ingress_canonicalize.rs` +72). | **N-B:** `consensus/` harness. **B1:** `validity/` harness. **B2:** `tx_validity/` submodule. **B3:** extended `harness/snapshot_loader.rs` (intra-corpus resolution), `tx_validity` extensions. **OQ5 → WRITEBACK:** progressive snapshot-loader extensions (key/script tag preservation, fail-closed cold-credential parsing, structured `UpdateCommittee` decode). **Post-3d94c22 thread:** real-chain committee oracle, corpus alignment, `reward_provenance` generator, snapshot-loader follow-ups (tip slot + Conway UMElem). **PHASE4-N-E S2 (`2d0c918`):** new GREEN `mempool::ingress_replay` harness — `ExpectedOutcome`, `BTrackCase`, `wrap_as_ingress`, `b_track_corpus_as_ingress`, `replay_ingress_trace`; 5 integration tests (CE-N-E-2 ingress=direct equivalence over the B-track corpus; CE-N-E-5 adversarial-rejection preservation through the chokepoint; CE-N-E-4 single-peer byte-identical replay; N-E-6 dependent-pair through the chokepoint; N-E-8 N2N vs N2C source-invariance). **PHASE4-N-E S3 (`509d714`):** 2 integration tests in `tests/mempool_ingress_canonicalize.rs` cross-checking canonicalization + replay (`two_interleavings_replay_byte_identical` CE-N-E-4 multi-peer; `empty_pool_canonicalizes_and_replays_to_initial_state`). |

No other crate had non-trivial source changes since baseline.
`ade_plutus` and `ade_node` were untouched by code commits.
**PHASE4-N-E touched `ade_ledger` (4 files), `ade_testkit` (4
files), `ade_core_interop` (4 files + `Cargo.toml`), `Cargo.lock`,
the registry, the two new CI scripts, and the new
`docs/clusters/PHASE4-N-E/` cluster + planning docs.** No
`.idd-config.json` change. No `ade_codec` / `ade_types` /
`ade_crypto` / `ade_core` / `ade_network` / `ade_runtime` /
`ade_node` / `ade_plutus` change.

---

## 4. Feature Flags

No Cargo `[features]` tables exist at HEAD in any workspace crate, and
none existed at baseline. The project does not use Cargo feature flags
as a semantic surface — closed semantic surfaces are encoded in the
type system per the IDD core principles, and conditional compilation
is checked out of BLUE code via `ci/ci_check_no_semantic_cfg.sh`
(scoped over the full 6-crate BLUE set, covering all surfaces
introduced through the PHASE4-N-E chokepoint and canonicalizer).

No `#[cfg(feature = ...)]` gates appear at either ref. `cardano-crypto`
(`vrf-draft03`) and `minicbor` (`alloc`) feature selections in the
dependency entries are upstream-crate selections, not Ade-side flags.

**Status: unchanged — zero Ade feature flags at baseline, zero at HEAD.**

---

## 5. CI Checks

The CI surface is the shell-script set under `ci/` (no
`.github/workflows` in this repo). At baseline there were 15 scripts.
At HEAD there are **31 scripts plus one git hook**
(`ci/git-hooks/commit-msg`): CE-73 added one, N-D added three, N-A
added two, N-B added four, B3 added one, B3F added one, B5 added one,
OQ5 added one (the 29th), and **PHASE4-N-E added two — the 30th and
31st**: `ci_check_mempool_ingress_closure.sh` (S1, `DC-MEM-03`) and
`ci_check_mempool_ingress_replay.sh` (S2, extended in S3,
`DC-MEM-04`). **B1, B2, B4, COMMITTEE-CRED-FIDELITY,
DREP-VOTE-FIDELITY, ENACTMENT-COMMITTEE-FIDELITY,
ENACTMENT-COMMITTEE-WRITEBACK, and the post-3d94c22 testkit thread
added no new CI script.** Grouped by cluster.

### CE-73 reclassification (Phase 2C close-out)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_hfc_translation.sh` | **New** (`9b15378`) | CE-73-semantic gate: runs the three HFC ledger-side translation proof surfaces. Authoritative test for invariant `DC-EPOCH-02`. |

### IDD canonicalization (post-Phase-4-N-D)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_constitution_coverage.sh` | Modified (`39865f6`) | Path-only edit: registry path now `docs/ade-invariant-registry.toml`. |
| `ci/git-hooks/commit-msg` | **New** (`2047c42`) | Local git hook: rejects commit messages lacking a `Co-Authored-By: Claude ...` trailer. |

### BLUE-list drift closure (`5b70bee`)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_module_headers.sh` | Modified — BLUE-scope (`5b70bee`) | `// Core Contract:` header on every `.rs` in BLUE crates. |
| `ci/ci_check_no_semantic_cfg.sh` | Modified — BLUE-scope (`5b70bee`) | No semantic `#[cfg(...)]` in BLUE `src/`. |
| `ci/ci_check_no_signing_in_blue.sh` | Modified — BLUE-scope (`5b70bee`) | No signing primitives in BLUE crates. |
| `ci/ci_check_hash_uses_wire_bytes.sh` | Modified — BLUE-scope (`5b70bee`) | All BLUE hashing via wire-byte fingerprint surfaces. |
| `ci/ci_check_ingress_chokepoints.sh` | Modified — BLUE-scope + registry growth (`5b70bee`) | No raw CBOR decoding outside named chokepoints in BLUE. |
| `ci/ci_check_dependency_boundary.sh` | Modified — BLUE-scope (`5b70bee`) | BLUE crates must not depend on RED crates. |

### Phase 4 N-D CI gap closure (`78da6c9`)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_chaindb_contract.sh` | **New** (`78da6c9`) | `cargo test -p ade_runtime --lib chaindb::` — 8 contract tests. |
| `ci/ci_check_recovery_contract.sh` | **New** (`78da6c9`) | `cargo test -p ade_runtime --lib recovery::` — 6-test recovery bundle. |
| `ci/ci_check_chaindb_crash_safety.sh` | **New** (`78da6c9`) | Smoke variant of the subprocess-SIGKILL harness + integrity post-checks. |

### Phase 4 N-A wire + semantic enforcement (S-A1, S-A10)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_no_async_in_blue.sh` | **New** (`4fde3a7`, S-A1) | Enforces `DC-CORE-01` — BLUE code is sync-only. |
| `ci/ci_check_ce_n_a_5_proof.sh` | **New** (`56bfa7b`, S-A10) | CE-N-A-5 closure-gate evidence over the real-cardano-node corpus. |

### Phase 4 N-B consensus authority enforcement (S-B1, S-B2, S-B8) — extended by B1 and B2

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_consensus_closed_enums.sh` | **New** (N-B); **Modified** (B1, `7b95ccd`); **Modified** (B2) | Four-part scan over `ade_core/src/consensus/`, `ade_ledger/src/block_validity/`, `ade_ledger/src/tx_validity/`, and `ade_ledger/src/mempool/`. |
| `ci/ci_check_no_chaindb_in_consensus_blue.sh` | **New** (N-B / S-B1) | No `ChainDb`/`chain_db` token in `consensus/`. |
| `ci/ci_check_no_density_in_fork_choice.sh` | **New** (N-B / S-B8) | No `density` token in `fork_choice.rs` / `candidate.rs`. |
| `ci/ci_check_no_float_in_consensus.sh` | **New** (N-B / S-B1) | No `f32`/`f64` in `consensus/`. |

### Phase 4 B3 Conway value-conservation enforcement (`978c222`)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_deposit_param_authority.sh` | **New** (`978c222`) | Enforces `DC-TXV-07` (canonical deposit-param authority). |

### Phase 4 B3F cert-classification closure enforcement (`d6c1993`)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_conway_cert_classification_closed.sh` | **New** (`d6c1993`, B3F-S1) | Enforces `DC-TXV-06` — flips `partial` → `enforced`. |

### Phase 4 B4 cert-state-accumulation fail-closed enforcement (`302d22c`)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_forbidden_patterns.sh` | **Modified** (`302d22c`, B4-S3/S4) | Enforces `DC-LEDGER-08` — no `non-fatal during replay` rationale; no `Err(_) =>` swallow arm in `accumulate_tx_certs`. |

### Phase 4 B5 governance-cert-accumulation fail-closed enforcement (`06385d0`)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_gov_cert_accumulation_closed.sh` | **New** (`06385d0`, B5-S4) | Enforces `DC-LEDGER-09` — four-part grep-gate over `apply_conway_gov_cert` totality, `checked_add` arithmetic, observe-and-drop removal, env fail-fast wiring. |

### OQ5 / FIDELITY / WRITEBACK credential discriminant gate (single script, extended six times)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_credential_discriminant_closed.sh` | **New** (`a3ee2da`, OQ5-S2) | Enforces `DC-LEDGER-10`. Three OQ5 clauses: `StakeCredential` is the closed 2-variant enum; both era decoders preserve the tag; no bare-`Hash28` tuple coercion on the BLUE authority path. |
| same | **Modified** (`2aeea16`, COMMITTEE-CRED-FIDELITY-S2) | +2 committee clauses. |
| same | **Modified** (`62c9020`, DREP-VOTE-FIDELITY-S2) | +2 DRep clauses. |
| same | **Modified** (`a6b8de7`, ENACTMENT-COMMITTEE-FIDELITY-S1) | +1 enactment-effect clause (clause 6). |
| same | **Modified** (`69e2d4b`, ENACTMENT-COMMITTEE-WRITEBACK close) | +section 7: structured `UpdateCommittee` surface + `apply_committee_enactment` presence/call-site. |
| same | **Unmodified post-3d94c22 and unmodified by PHASE4-N-E** | The credential-discriminant gate stays the **29th** script. |

### PHASE4-N-E wire-level mempool ingress closure (`32c1ee6`, `2d0c918`, `509d714`)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_mempool_ingress_closure.sh` | **New** (`32c1ee6`, S1) — the **30th** script | Enforces `DC-MEM-03` via 5 mechanical guards: (1) `mempool/ingress.rs` defines `IngressEvent`/`IngressSource`/`mempool_ingress` and is re-exported from `mempool/mod.rs`; (2) `IngressSource` is a closed 2-variant enum with no `#[non_exhaustive]` and exactly one `pub enum` in the file; (3) `MempoolState.accumulating` is field-written only inside `mempool/admit.rs`; (4) `admit()` is called only from `mempool/admit.rs` (definition + co-located tests) and `mempool/ingress.rs` (the new bridge) — all other production `src/` callers are forbidden, `crates/*/tests/` exempt; (5) `mempool_ingress` body must not reference `source` — the verdict is a function of `(state, tx_bytes)` alone. |
| `ci/ci_check_mempool_ingress_replay.sh` | **New** (`2d0c918`, S2); **Modified** (`509d714`, S3, +2 clauses) — the **31st** script | Enforces `DC-MEM-04` via 6 mechanical guards: (1) `mempool/ingress_replay.rs` exists, is registered in `testkit/src/lib.rs`, and exports `ExpectedOutcome`/`BTrackCase`/`wrap_as_ingress`/`b_track_corpus_as_ingress`/`replay_ingress_trace`; (2) `replay_ingress_trace` body calls `mempool_ingress`, not `admit`; (3) the 5 registry-pinned test functions exist in the test file; (4) no batching helpers (`chunks`/`partition`/`rayon`/`tokio::spawn`) — single-step per OQ-6; (5, S3) `canonicalize.rs` exists, defines the three items, re-exported; (6, S3) `canonicalize.rs` body contains no `HashMap`/`HashSet`/`tokio`/`async fn`/`.await`/`SystemTime`/`Instant`/`rand`/`thread_rng`/`RwLock`/`Mutex` — strictly sync + deterministic. |

TRACEABILITY cross-reference: every script listed above appears as a
`ci_script` for at least one rule in `docs/ade-invariant-registry.toml`,
re-traced via `ci/ci_check_constitution_coverage.sh`. **PHASE4-N-E**
added two new `ci_script` ↔ rule edges: `ci_check_mempool_ingress_closure.sh
→ DC-MEM-03` and `ci_check_mempool_ingress_replay.sh → DC-MEM-04`.
The parallel TRACEABILITY regeneration will add the two new rules
and the `DC-MEM-01.strengthened_in += "PHASE4-N-E"` strengthening.

---

## 6. Canonical Type Registry Delta

n/a — `.idd-config.json` `canonical_type_registry` is null. Canonical-type
rules live inline in the invariant registry under family `T`.

The new PHASE4-N-E closed enums (`IngressSource`) and closed structs
(`IngressEvent`) are canonical-type additions if CODEMAP's
canonical-type count is to stay in sync; the current TRACEABILITY
regeneration round is the right place to reflect that.

---

## 7. Normative Rule Delta

The project's invariant registry tracks structured rules (TOML), not
prose normative-doc rules; this section reports on it.

- Rules at baseline (`d509f02:constitution_registry.toml`): **147**
- Rules at prior refresh (`168ac02:docs/ade-invariant-registry.toml`): **173**
- Rules at HEAD (`43fcc31:docs/ade-invariant-registry.toml`): **175**
- Net additions vs baseline: **+28** (PHASE4-N-A: 2; PHASE4-N-B: 8;
  PHASE4-B1: 6; PHASE4-B2: 5; PHASE4-B3: 2; PHASE4-B3F: 0; PHASE4-B4: 1
  (`DC-LEDGER-08`); PHASE4-B5: 1 (`DC-LEDGER-09`); OQ5: 1
  (`DC-LEDGER-10`); COMMITTEE-CRED-FIDELITY / DREP-VOTE-FIDELITY /
  ENACTMENT-COMMITTEE-FIDELITY / ENACTMENT-COMMITTEE-WRITEBACK /
  post-3d94c22 testkit thread: 0 each — all in-place strengthenings;
  **PHASE4-N-E: 2** (`DC-MEM-03`, `DC-MEM-04`). The two `DC-MEM-*`
  rules introduced earlier (`DC-MEM-01`, `DC-MEM-02`) were flipped to
  `enforced` in B2 and are not counted as new.
- Net additions vs prior refresh: **+2** (`DC-MEM-03`, `DC-MEM-04`).
- Removals: **0** (expected under append-only discipline; clean).

- New rules at HEAD (since the prior refresh):
  - **`DC-MEM-03`** (derived, `enforced`, `introduced_in =
    PHASE4-N-E`): "Tx ingress reduces to a closed `IngressEvent`
    before BLUE mempool admission; the source variant is
    evidence/policy/replay metadata only and MUST NOT change the
    validity verdict." `code_locus` =
    `crates/ade_ledger/src/mempool/ingress.rs` (`IngressEvent`,
    `IngressSource`, `mempool_ingress`). `tests` = 8 (2 inline + 6
    integration covering both `IngressSource` variants, the verdict
    invariance under N2N vs N2C, and ingress=direct equivalence on
    the synthetic corpus). `ci_script` =
    `ci/ci_check_mempool_ingress_closure.sh`. `cross_ref` =
    `[DC-MEM-01]` (bidirectional).
  - **`DC-MEM-04`** (derived, `enforced`, `introduced_in =
    PHASE4-N-E`): "Replaying the same ordered ingress trace against
    the same base ledger state produces a byte-identical sequence of
    `(MempoolState, AdmitOutcome)` pairs." `code_locus` =
    `crates/ade_testkit/src/mempool/ingress_replay.rs;
    crates/ade_ledger/src/mempool/ingress.rs;
    crates/ade_ledger/src/mempool/canonicalize.rs`. `tests` = 8 (5
    from S2 + 3 added in-place by S3 covering the canonicalizer).
    `ci_script` = `ci/ci_check_mempool_ingress_replay.sh`. `cross_ref`
    = `[DC-MEM-01]` (bidirectional).

- Strengthenings at HEAD:
  - **`DC-MEM-01`** (PHASE4-N-E, `2d0c918`/`509d714`): strengthened —
    `strengthened_in += "PHASE4-N-E"`; `code_locus +=
    "; mempool/ingress.rs; mempool/ingress_replay.rs"`; `tests += 3`
    (the new ingress-replay test names from S2 + the
    canonicalize-replay names from S3); `cross_ref += "DC-MEM-04"`
    (bidirectional pairing with the new replay rule). The mempool
    admission chokepoint contract is now mechanically enforced from
    the wire-event boundary inward through the chokepoint to admit,
    not just at admit.
  - **`DC-MEM-02`** (carried forward from B2): `enforced`.
  - **All earlier strengthenings carried forward unchanged**:
    `DC-EPOCH-01` (WRITEBACK + post-3d94c22 oracle); `DC-LEDGER-10`
    (OQ5 → COMMITTEE-CRED → DREP-VOTE → ENACTMENT-COMMITTEE-FIDELITY
    → WRITEBACK → post-3d94c22 oracle; 20 tests at HEAD);
    `DC-LEDGER-08` (B5, via `cross_ref`); `T-DET-01` / `T-ENC-03`
    (OQ5); `DC-TXV-06` (B3F: `partial` → `enforced`); `DC-VAL-06`
    (B3F + B4); `T-CONSERV-01` / `CN-LEDGER-07` (B3);
    `DC-MEM-01,02` (B2, `declared` → `enforced`); `DC-EPOCH-02`
    (CE-73 reclassification); the N-D bundle; the N-A real-capture
    bundle; `T-CORE-02` (S-B1).

Family counts at HEAD: CN: 69, DC: 64 (added `DC-MEM-03`, `DC-MEM-04`
this regen), OP: 7, RO: 6, T: 29 — total 175. Per the constitution
coverage gate verification in `43fcc31`'s commit message,
`ci_check_constitution_coverage.sh` PASS.

Normative-doc rule extraction (the `normative_docs` list in
`.idd-config.json`) is approximate and not regenerated here — the
structured registry is the authoritative source.

---

## Anomalies and Cross-Reference Warnings

- **PHASE4-N-E cluster status: code + harness complete, live evidence
  pending — cluster dir NOT yet archived.** `docs/clusters/PHASE4-N-E/`
  contains the cluster doc + 5 slice docs + the two operator-procedure
  docs (`CE-N-E-6_PROCEDURE.md`, `CE-N-E-7_PROCEDURE.md`); planning
  artifacts at `docs/planning/phase4-n-e-tier1-invariants.md` and
  `docs/planning/phase4-n-e-tier1-cluster-slice-plan.md`. CE-N-E-1
  through CE-N-E-5 are mechanically green; CE-N-E-6 and CE-N-E-7
  close in two halves — the mechanical adapter half is green in
  S4/S5, the live-log half is operator-action per the documented
  procedures. **Cluster closure (`/cluster-close`) lands once the
  two `CE-N-E-{6,7}_<YYYY-MM-DD>.log` artifacts are committed under
  `docs/clusters/PHASE4-N-E/`.**
- **CODEMAP / SEAMS / TRACEABILITY are being regenerated in
  parallel with this HEAD_DELTAS rewrite — expected drift at the
  exact moment of this regen.** Prior CODEMAP (`52642e5`) does NOT
  yet contain rows for `ade_ledger::mempool::ingress`,
  `ade_ledger::mempool::canonicalize`,
  `ade_testkit::mempool::ingress_replay`,
  `ade_core_interop::tx_submission`, or
  `ade_core_interop::local_tx_submission`. Prior SEAMS does NOT yet
  contain a mempool-ingress-surface row. Prior TRACEABILITY does
  NOT yet contain `DC-MEM-03` / `DC-MEM-04` / the new CI scripts /
  the `DC-MEM-01.strengthened_in += "PHASE4-N-E"` strengthening.
  All three rewrites are in flight in the same regen round; the
  three docs will be self-consistent at the next grounding-doc
  commit (the parallel `docs(grounding)` ripple from the cluster
  close).
- **PHASE4-N-E source-invariance is the load-bearing wire-level
  no-false-accept property.** `IngressSource` is metadata only —
  the verdict path is a function of `(state, tx_bytes)` alone.
  Mechanically enforced by CI guard #5 of
  `ci_check_mempool_ingress_closure.sh` (no `source` reference in
  the `mempool_ingress` body), by the S1 inline test
  `ingress_source_does_not_change_verdict_*`, by the S2 integration
  test `ingress_trace_source_invariant_n2n_vs_n2c`, and by the S5
  cross-bridge agreement test
  `n2n_and_n2c_bridges_produce_identical_outcomes`. Any divergence
  under cross-bridge replay is **release-blocking source-leak**.
- **CE-N-E-6 / CE-N-E-7 live-log half is operator-action — not
  CI.** Mirrors the established CE-N-B-6 pattern. The committed
  procedures (`CE-N-E-6_PROCEDURE.md`, `CE-N-E-7_PROCEDURE.md`)
  describe handshake, capture window, cross-check against direct
  `tx_validity`, and the `CE-N-E-{6,7}_<YYYY-MM-DD>.log` artifact
  format. Until those logs are committed, the cluster status reads
  "code + harness complete, live evidence pending" — **not** "fully
  closed".
- **`ade_core_interop -> ade_ledger` new dependency edge
  (PHASE4-N-E S4, `ca3f23a`).** `ade_core_interop` (RED) now
  depends directly on `ade_ledger` (BLUE). The edge direction
  (RED → BLUE) is allowed by `ci_check_dependency_boundary.sh`
  (BLUE crates must not depend on RED crates; the converse is the
  Functional-Core/Imperative-Shell shape). Was a transitive dep
  via the `ade_core` edge prior to S4; now direct so the new
  bridges can `use ade_ledger::mempool::{mempool_ingress,
  IngressEvent, IngressSource}`.
- **The cluster doc's initial S4 placement under `ade_runtime` was
  inaccurate and is corrected by `ca3f23a`'s TCB Color Map
  update.** S4's home is `ade_core_interop`, not
  `ade_runtime::tx_submission::n2n_session` — `ade_core_interop` is
  the project's established RED live-interop crate (already houses
  the PHASE4-N-B follow-mode bridge). The cluster doc footnote
  records the move.
- **B-track corpus reuse is verbatim per OQ-3 (PHASE4-N-E S2).**
  The B-track adversarial corpus is the existing PHASE4-B2 corpus;
  only the `IngressEvent` envelope is new (`wrap_as_ingress` /
  `b_track_corpus_as_ingress`). No new adversarial corpus content
  for N-E. The replay fold is a literal pass over `mempool_ingress`
  (no batching, no out-of-order interleaving — per OQ-6, enforced
  by CI guard #4 of `ci_check_mempool_ingress_replay.sh`).
- **Post-3d94c22 testkit thread (`b9cfaf9` / `396664a` / `c78ec76`
  / `168ac02`) — carried forward unchanged.** Four GREEN-scope
  commits with one registry-level effect: in-place strengthening
  of `DC-EPOCH-01.tests` and `DC-LEDGER-10.tests` (each `+=
  committee_oracle_mainnet_575_576_noop_agreement`) plus the
  `authority_surface` / `open_obligation` text rewrites on both
  rules. `DC-LEDGER-10` at 20 tests at HEAD.
- **`open_obligation` reclassification on `DC-EPOCH-01` /
  `DC-LEDGER-10`: environment-blocked → reality-blocked (committee
  CHANGE case) — carried forward.** Mainnet enacted no
  `UpdateCommittee` / `NoConfidence` across the 575→576 boundary;
  the non-committee discriminated keys (`vote_delegations` /
  `drep_expiry`) remain environment-blocked pending those
  extractions. Not a regression, not a fail-open.
- **Cluster docs archived in `52642e5` (committed at this HEAD).**
  Seven cluster directories moved via `git mv` from
  `docs/clusters/<NAME>/` to `docs/clusters/completed/<NAME>/`:
  COMMITTEE-CRED-FIDELITY, DREP-VOTE-FIDELITY,
  ENACTMENT-COMMITTEE-FIDELITY, OQ5-CREDENTIAL-FIDELITY, PHASE4-B3F,
  PHASE4-B4, PHASE4-B5. Plus the previously-committed archives
  (B1, B2, B3, N-A, N-B, N-D, ENACTMENT-COMMITTEE-WRITEBACK).
  `docs/clusters/` at HEAD contains only `PHASE4-N-B/` (a non-IDD
  log directory) and the in-flight **`PHASE4-N-E/`** (NOT yet
  archived per the cluster-status anomaly above).
- **ENACTMENT-COMMITTEE-WRITEBACK fingerprint change (T-DET-01,
  deliberate; carried forward).** `write_gov_action` emits the
  structured `UpdateCommittee` shape `[5, prev, set<cred>,
  {cred=>epoch}, num, den]` in place of the opaque `[4, prev,
  bytes]`. Confirmed by the
  `committee_oracle_mainnet_575_576_noop_agreement` real-chain
  oracle.
- **WRITEBACK carry-forward follow-ups (narrowed, unchanged).**
  FIDELITY follow-up **(d)** RESOLVED + WIRED. **(e)** narrowed to
  the older `mk_credential` helper (contained to `ade_testkit`,
  cannot reach the node binary). The pre-OQ5 **(b)** Shelley
  unknown-cert zero-hash placeholder remains a WARN LOW non-goal.
- **B5 / B4 / B3F / B3 / B2 / B1 / N-D / N-B / N-A closures —
  carried forward unchanged.** All cluster docs at
  `docs/clusters/completed/<NAME>/`.
- **`DC-LEDGER-08` strengthening recorded via `cross_ref`, not
  `strengthened_in` (carried forward).** Harmless; consider
  normalizing on the next registry curation pass.
- **DC-VAL status mismatch vs. closure claim (B1, carried forward).**
  PHASE4-B1 reports fully closed, but in the registry only
  `DC-VAL-01` is `enforced` — `DC-VAL-02` → `DC-VAL-05` remain
  `declared` despite named tests and the extended closed-enums
  enforcement point. Flip on the next `/traceability` pass.
- **`strengthened_in` records the introducing cluster on
  freshly-created rules (carried forward).** Each `DC-VAL-*`
  records `["PHASE4-B1"]`, each `DC-TXV-01..05` records
  `["PHASE4-B2"]`, the two new `DC-MEM-03`/`DC-MEM-04` records
  `strengthened_in = []` (no strengthenings yet) but
  `introduced_in = "PHASE4-N-E"`. Harmless.
- **`ade_ledger -> ade_core` dependency edge (B1, carried forward)
  + new `ade_core_interop -> ade_ledger` edge (PHASE4-N-E S4).**
  All in compliance with `ci_check_dependency_boundary.sh`.
- **B3 positive corpus carves out Plutus per CE-88 (carried
  forward).**
- **Adversarial corpora are derived, not committed (carried
  forward).** N-E reuses the B2 B-track corpus verbatim — no new
  adversarial bytes committed for N-E.
- **Corpus relayout: credentialed snapshots removed, then
  regenerated off-repo (carried forward).** `corpus/snapshots/`
  `.gitignore`-d; canonical home `s3://ade-corpus-snapshots`.
- **`ade_core_interop` tests `#[ignore]`-gated / offline-replay
  by design (carried forward).** Live tip-agreement is not run in
  CI; the new PHASE4-N-E `tx_submission_ingress` and
  `local_tx_submission_ingress` integration tests run *non-ignored*
  in CI because they operate on synthetic / B-track corpus bytes,
  not against a live cardano-node socket.
- No removed canonical types (n/a — no separate registry).
- No removed registry rules (expected: 0; actual: 0). PHASE4-N-E
  added `DC-MEM-03` and `DC-MEM-04`; registry total stays **175**
  at HEAD.
- **All commit subjects in this regen carry a conventional-commits
  prefix.** The 5 PHASE4-N-E commits are `feat(ledger)` /
  `test(testkit)` / `feat(ledger)` / `feat(interop)` /
  `feat(interop)`; `52642e5` is `docs(grounding)`. **All 5 N-E
  commits + `52642e5` carry the repo-required `Co-Authored-By`
  model-attribution trailer** (per the CLAUDE.md project override
  for the bounty trailer ratio).

---

## Generation Notes

Regenerate via `/head-deltas <baseline>` or by re-running the
`head-deltas-generator` agent with the same baseline. Baseline lives
in `.idd-config.json` `head_deltas_baseline` (still `d509f02` —
**this is a cluster-level refresh after a code-complete cluster, not
a phase boundary, so the baseline is unchanged**). Update the
baseline on the next phase boundary (Phase 4 close). Note the
commit-hash rewrite caveat at the top — re-derive hashes from
`git log` at each regen rather than carrying them forward. This
regen is cut at committed HEAD `43fcc31` (PHASE4-N-E S5). The prior
regen narrated HEAD `168ac02` (snapshot-loader follow-ups); the new
span is `168ac02..43fcc31` — 6 commits (`52642e5` post-WRITEBACK
grounding refresh + archive moves, `32c1ee6` N-E S1, `2d0c918` N-E
S2, `509d714` N-E S3, `ca3f23a` N-E S4, `43fcc31` N-E S5).
