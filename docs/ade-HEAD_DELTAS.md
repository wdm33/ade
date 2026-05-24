# HEAD Deltas — Ade

> **Status:** Living architectural document. Regenerated; not hand-edited.
> Regenerate via `/head-deltas <baseline>`. Baseline is declared in
> `.idd-config.json` (`head_deltas_baseline`).

> Baseline: `d509f02` (Phase 3 handoff snapshot, 2026-04-15)
> HEAD: `168ac02` (fix(testkit): snapshot-loader follow-ups (tip slot + Conway UMElem), 2026-05-25)
> 133 commits, 11,281 files changed, +174,817 / −7,233,590 lines

Headline numbers note: the massive negative line count is dominated by
the **corpus relayout** under `corpus/snapshots/` and the deletion of
the multi-MB credentialed-snapshot text files
(`*_registered_creds.txt`, ~7M lines combined). Source-tree deltas are
far smaller — the per-crate breakdown in §3 is the representative view.

> **Commit-hash note.** This regen runs against the current (rebased)
> history. The prior HEAD_DELTAS regen — cut at the then-HEAD
> `0d4457e` (B1-S7) — references commit hashes from a history that has
> since been rewritten; e.g. B1-S7 is now `2630267`, the N-A close is
> `69a2862` (unchanged), and the B1 close is `993f363`. All hashes
> below are verbatim from `git log d509f02..HEAD` at this HEAD.

> **Testkit follow-up note (newest thread).** This regen is cut at
> committed HEAD `168ac02`. Since the previous grounding-doc refresh
> commit `3d94c22` (which committed the ENACTMENT-COMMITTEE-WRITEBACK
> CODEMAP/SEAMS/HEAD_DELTAS/TRACEABILITY ripple + the `DC-EPOCH-01` /
> `DC-LEDGER-10` strengthenings on top of the `69e2d4b` close-hardening
> commit), **four new commits have landed** — all GREEN-scope,
> non-authoritative, and bounded to `ade_testkit` / corpus tooling:
> `b9cfaf9` (real-chain committee oracle at mainnet 575→576), `396664a`
> (align previously-blocked `ade_testkit` tests + `ade_plutus` compile
> with the regenerated corpus), `c78ec76` (re-runnable `reward_provenance`
> generator, ignored by default), and `168ac02` (snapshot-loader
> follow-ups: tip-slot surface + Conway UMElem layout). **No new module,
> no new crate, no new rule, no new CI script, no BLUE source change.**
> Two existing rules — `DC-EPOCH-01` and `DC-LEDGER-10` — gain a single
> real-chain oracle test each (the same `committee_oracle_mainnet_575_576_noop_agreement`
> entry on both); the committee-side `open_obligation` is reclassified
> **from environment-blocked to reality-blocked** for the positive
> committee-CHANGE case (mainnet enacted no `UpdateCommittee` /
> `NoConfidence` across the 575→576 boundary, so no snapshot pair
> exhibits a committee delta to diff). The other two test commits make
> the previously snapshot-gated `ade_testkit` suites pass against the
> regenerated boundary corpus and add the canonical Conway-layout
> credential-emitter for `reward_provenance/*_tick_registered_creds.txt`.
> The registry total **stays 173**, the CI count **stays 29**. All four
> commits carry the model-attribution trailer.

> **ENACTMENT-COMMITTEE-WRITEBACK close-hardening note (immediately prior
> thread).** Between the WRITEBACK-S2 implementation HEAD `3180e27` and
> the WRITEBACK grounding-doc refresh `3d94c22`, the per-cluster IDD
> review surfaced two cluster-close hardening items, both landed in
> `69e2d4b`: the GREEN snapshot loader's
> `parse_cold_credential_set` / `parse_cold_credential_epoch_map` now
> reject a declared-but-under-length (truncated) set/map on their own
> terms (a `terminated` flag distinguishes a proper end from a run-off,
> fail-closed at the container level), and the existing
> `ci/ci_check_credential_discriminant_closed.sh` was extended with a
> **section 7** asserting the structured `GovAction::UpdateCommittee`
> surface (discriminated `removed: BTreeSet<StakeCredential>` / `added:
> BTreeMap<StakeCredential, …>`) and the **`apply_committee_enactment`
> presence + call-site** in `rules.rs` — so the enactment cannot silently
> revert to a clone. The per-cluster security review found no HIGH+
> findings. No new module, no new crate, no new rule, no new CI script;
> the gate stays the 29th.

> **ENACTMENT-COMMITTEE-WRITEBACK cluster note (prior thread).** Cut at
> committed HEAD `3180e27`. Since the ENACTMENT-COMMITTEE-FIDELITY close
> `3706534`, the **ENACTMENT-COMMITTEE-WRITEBACK arc** landed as three
> implementation commits: `ea25dd9` (cluster plan + invariants, *wires*
> committee enactment), `f2f15f9` (S1, structured `UpdateCommittee` gov
> action), and `3180e27` (S2, wire committee enactment write-back), plus
> the close-hardening `69e2d4b` and the grounding-doc refresh `3d94c22`.
> It turns the *dormant* type pin that ENACTMENT-COMMITTEE-FIDELITY
> landed (the discriminated `EnactmentEffects.committee_changes`) into a
> **live committee write-back** — `UpdateCommittee` enactment is no
> longer a no-op and the `ConwayGovState.committee` map is no longer
> cloned unchanged at the epoch boundary. **No new module, no new rule,
> no new CI script** — the existing gate is extended (section 7) and
> `DC-EPOCH-01` / `DC-LEDGER-10` are both strengthened. **S1**
> (`f2f15f9`) replaced the opaque `GovAction::UpdateCommittee {
> prev_action, raw: Vec<u8> }` with a **structured** `{ prev_action,
> removed: BTreeSet<StakeCredential>, added: BTreeMap<StakeCredential,
> u64>, threshold: (u64, u64) }` (`ade_types::conway::governance`); the
> fingerprint `write_gov_action` emits the structured 5-field shape `[5,
> prev, set<cred>, {cred=>epoch}, num, den]` replacing the prior opaque
> `[4, prev, bytes]` (`ade_ledger::fingerprint`, a deliberate `T-DET-01`
> migration); the GREEN `ade_testkit` `snapshot_loader`
> `parse_gov_action` tag-4 path decodes the set/map/`unit_interval`
> shape with **fail-closed** cold-credential parsing. **S2** (`3180e27`)
> made committee enactment **live**: `enact_proposals` now populates
> `committee_changes` and the new `committee_threshold`; the new pure
> transition `apply_committee_enactment(committee, quorum, effects) ->
> (committee, quorum)` (`ade_ledger::governance`) dissolves the
> committee on `NoConfidence`, removes/adds the discriminated cold
> members and sets the new quorum on `UpdateCommittee`; and the
> epoch-boundary apply site in `rules.rs` **calls it**.
> **Cluster-close** (`69e2d4b`) added a **fail-closed container-level
> reject** in the loader and **extended**
> `ci/ci_check_credential_discriminant_closed.sh` with section 7
> (structured-`UpdateCommittee` surface + `apply_committee_enactment`
> presence/call-site). **`DC-EPOCH-01` and `DC-LEDGER-10` are both
> STRENGTHENED** (+9 new tests across the two; `DC-LEDGER-10` reaching
> 19 at the grounding refresh, **20 after the post-3d94c22 real-chain
> oracle**). CI count **stays 29**, registry **stays 173**.

> **ENACTMENT-COMMITTEE-FIDELITY cluster note (prior thread).** Cut at
> committed HEAD `a6b8de7`. The **ENACTMENT-COMMITTEE-FIDELITY arc**
> landed as two commits: `5d64fee` (plan, *strengthens* `DC-LEDGER-10`)
> and `a6b8de7` (S1, discriminate `EnactmentEffects.committee_changes`).
> Discharges the DREP-VOTE-FIDELITY carry-forward follow-up **(d)** via
> a one-line preventive type migration of the
> `EnactmentEffects.committee_changes` field (`Hash28` →
> `StakeCredential`). At the FIDELITY close the field was **DORMANT**
> (always `None`); after the WRITEBACK close above it is **LIVE**.

> **DREP-VOTE-FIDELITY cluster note (prior thread).** Cut at committed
> HEAD `62c9020`. A 2-slice arc (plus one grounding commit:
> `ecb0b92`/`ba4ff37`/`62c9020`) discharging the COMMITTEE-CRED
> security-review follow-up **(c)**. `GovActionState.drep_votes`
> re-typed `Vec<(Hash28, Vote)>` → `Vec<(StakeCredential, Vote)>`;
> `governance.rs` `lookup_stake` resolves a DRep voter to exactly one
> `DRep` stake key by discriminant (no OR-fallback).

> **COMMITTEE-CRED-FIDELITY cluster note (prior thread).** Cut at
> committed HEAD `2aeea16`. A 2-slice arc (`32d7a2e`/`2303a60`/`2aeea16`)
> discharging the OQ5 security-review follow-up **(a)**.
> `ConwayGovState.committee` re-keyed `Hash28` → `StakeCredential`;
> `GovActionState.committee_votes` re-typed similarly.

> **OQ5-CREDENTIAL-FIDELITY cluster note (prior thread).** Cut at
> committed HEAD `a3ee2da`. A 2-slice arc (plus two grounding commits:
> `959e16c`/`007b0e8`/`4187330`/`a3ee2da`) that closes the B5-named
> **OQ-5** collapse. `StakeCredential` changed from the tuple struct
> `StakeCredential(pub Hash28)` to an `enum { KeyHash(Hash28),
> ScriptHash(Hash28) }`. Both era decoders preserve the tag.
> `ConwayGovState` re-keyed `Hash28` → `StakeCredential` across
> `vote_delegations` / `committee_hot_keys` / `drep_expiry`.
> `DC-LEDGER-10` introduced and flipped to **`enforced`** via the new
> `ci/ci_check_credential_discriminant_closed.sh` (the 29th script).

> **B5 cluster note (prior thread).** Cut at committed HEAD `651adc9`.
> The **PHASE4-B5 Conway governance-certificate accumulation arc**
> landed as six commits (`fdb6601` grounding + `9c8d118`/`7a48727`/
> `d63c700`/`06385d0`/`651adc9` implementation). New BLUE module
> `ade_ledger::gov_cert`; `DC-LEDGER-09` introduced and `enforced` via
> `ci/ci_check_gov_cert_accumulation_closed.sh` (the 28th script).

> **B4 cluster note (prior thread).** Cut at committed HEAD `ee35493`.
> The **PHASE4-B4 Conway certificate-state accumulation arc** landed
> as five commits (`ae1300a`/`228415b`/`da30706`/`302d22c`/`ee35493`).
> `DC-LEDGER-08` introduced and `enforced` via the extended
> `ci/ci_check_forbidden_patterns.sh`.

> **B3 close + B3F follow-up note (carried forward).** B3 closed by
> `d766eb0`; B3F follow-up landed as `d6c1993` (B3F-S1) + `193d2fc`
> (B3F-S2). B3F flips `DC-TXV-06` `partial` → **`enforced`** and
> hardens the Conway cert decoder.

The delta covers twenty-two threads of work. The newest thread — the
**testkit follow-ups** (`b9cfaf9` real-chain committee oracle,
`396664a` corpus-alignment, `c78ec76` reward-provenance generator,
`168ac02` snapshot-loader follow-ups) — landed on top of the WRITEBACK
grounding-doc refresh `3d94c22`. Below it the
**ENACTMENT-COMMITTEE-WRITEBACK arc** sat on the
ENACTMENT-COMMITTEE-FIDELITY close (`3706534`); below it the
**ENACTMENT-COMMITTEE-FIDELITY arc** sat on the DREP-VOTE-FIDELITY
close (`06f517f`); below it the **DREP-VOTE-FIDELITY arc** sat on the
COMMITTEE-CRED-FIDELITY close (`a157c92`); below that the
**COMMITTEE-CRED-FIDELITY arc** sat on the OQ5 close (`676af5a`);
below that the **OQ5-CREDENTIAL-FIDELITY arc** sat on the PHASE4-B5
close (`f81f815`); below that the **PHASE4-B5 arc** sat on the
PHASE4-B4 close (`644eb03`), which sat on the PHASE4-B3F follow-up
hardening (`193d2fc`), which itself sat on the **PHASE4-B3 Conway
value-conservation accounting arc** above the PHASE4-B2 close
(`c1cba82`). In rough proportion of the substantive change budget:

0. **Testkit follow-ups (post-WRITEBACK, four commits).** All
   GREEN-scope, non-authoritative. `b9cfaf9` wires the **real-chain
   committee oracle** at mainnet Conway 575→576: the new test
   `committee_oracle_mainnet_575_576_noop_agreement` (in
   `crates/ade_testkit/tests/epoch_oracle_comparison.rs`) parses
   cardano-node's epoch-576 interim Constitutional Committee (exactly
   7 `ScriptHash` members, term-expiry epoch 580, quorum 2/3) and
   asserts (a) **PARSE fidelity** — our discriminated loader sees the
   committee as all `ScriptHash`, never a defaulted `KeyHash`
   (`DC-LEDGER-10`); and (b) **no-op write-back agreement** — mainnet
   enacted no committee change across the boundary, so our
   ratification + enactment over the epoch-575 gov state reproduces
   the epoch-576 committee + quorum exactly via
   `apply_committee_enactment` (`DC-EPOCH-01`). The test is
   snapshot-gated, and `corpus/snapshots/` is now `.gitignore`-d
   (multi-GB tarballs belong in S3, never git). The committee-side
   **`open_obligation` on both `DC-EPOCH-01` and `DC-LEDGER-10` is
   reclassified** — environment-blocked → reality-blocked for the
   positive committee-CHANGE case (mainnet enacted no such change in
   this range); the non-committee discriminated keys
   (`vote_delegations` / `drep_expiry` vs the `Credential`-keyed
   UMap/VState) remain environment-blocked. `396664a` **aligns 11
   previously-blocked `ade_testkit` tests + the `ade_plutus` compile
   with the regenerated corpus**: 11 pinned fingerprint hashes
   regenerated in `boundary_fingerprint_agreement.rs`; the
   skip-until-first-success pattern replaces fragile
   break-on-first-error loops in `boundary_stateful_replay.rs` /
   `transition_proof_surface.rs` /
   `contiguous_plutus_verdict_harness.rs`; pin-mismatch detection
   stays strict, only reference values changed; **no ledger-rule
   check weakened**. `c78ec76` adds the **re-runnable,
   `#[ignore]`-gated `reward_provenance` generator**
   (`crates/ade_testkit/tests/emit_reward_provenance.rs`) — canonical
   entry point for regenerating
   `reward_provenance/*_tick_registered_creds.txt` from snapshot
   tarballs (alonzo310: 1,049,128 creds; conway576 was empty at the
   c78ec76 cut). `168ac02` **closes three coherent snapshot-loader
   follow-ups**: **FU #1** — `parse_snapshot_header` now extracts the
   tip slot from the `HeaderState` `WithOrigin (AnnTip …)` envelope
   and `to_ledger_state` propagates it into `state.epoch_state.slot`;
   **FU #2** — `boundary_stateful_replay`,
   `late_era_composer_integration`, and `transition_proof_surface`
   pre-filter blocks by `slot > state.epoch_state.slot.0`; **FU #4**
   — `parse_registered_credentials` discriminates UMElem layouts by
   `UMElem[0]` major type (`array(major=4)` → pre-Conway; `uint(major=0)`
   → Conway compact `array(4) [reward, deposit, nullable Pool,
   nullable DRep]`). Pre-Conway emission stays byte-identical
   (alonzo310 sha256 unchanged); conway576 emission now produces
   **1,445,758 creds**. 34/34 snapshot_loader unit tests green.
   **GREEN-scope only — no BLUE-crate change, no determinism surface
   moves, no new module / rule / CI script.** Net registry effect:
   `DC-EPOCH-01.tests` and `DC-LEDGER-10.tests` each `+=
   committee_oracle_mainnet_575_576_noop_agreement` (one entry on
   each rule; `DC-LEDGER-10` reaches **20**); both rules'
   `authority_surface` / `open_obligation` rewritten. All four
   commits carry the model-attribution trailer.
1. **ENACTMENT-COMMITTEE-WRITEBACK (wire committee enactment write-back)
   — closed.** Three implementation commits (`ea25dd9`, `f2f15f9`,
   `3180e27`) + close-hardening (`69e2d4b`) + grounding refresh
   (`3d94c22`). Turns the dormant type pin ENACTMENT-COMMITTEE-FIDELITY
   landed into a **live committee write-back**, without a new module,
   rule, or CI script. **S1** (`f2f15f9`) made
   `GovAction::UpdateCommittee` **structured**: the opaque `{
   prev_action, raw: Vec<u8> }` is replaced by `{ prev_action,
   removed: BTreeSet<StakeCredential>, added:
   BTreeMap<StakeCredential, u64>, threshold: (u64, u64) }`. The
   fingerprint `write_gov_action` emits the structured 5-field shape
   replacing the prior opaque `[4, prev, bytes]` (`T-DET-01`); the
   GREEN loader `parse_gov_action` tag-4 path decodes the
   set/map/`unit_interval` shape with **fail-closed** cold-credential
   parsing. **S2** (`3180e27`) made committee enactment **live**: the
   new pure transition `apply_committee_enactment(committee, quorum,
   effects) -> (committee, quorum)` (`ade_ledger::governance`) is
   called at the epoch-boundary apply site. **Cluster-close**
   (`69e2d4b`) added a fail-closed container-level reject in the
   loader and extended
   `ci/ci_check_credential_discriminant_closed.sh` with section 7.
   **`DC-EPOCH-01` and `DC-LEDGER-10` are both STRENGTHENED** (+9
   new tests across the two; `DC-LEDGER-10` reaching 19 at the
   grounding refresh, **20 after the post-3d94c22 real-chain
   oracle**). CI count **stays 29**, registry **stays 173**.
   Fingerprint surface change (`T-DET-01`, the structured
   `UpdateCommittee` encoding). All cluster docs at
   `docs/clusters/completed/ENACTMENT-COMMITTEE-WRITEBACK/`.
2. **ENACTMENT-COMMITTEE-FIDELITY (committee-enactment effect
   credential discriminant fidelity) — closed (`a6b8de7`).** A
   1-slice arc (plus one grounding/plan commit) that discharges the
   DREP-VOTE-FIDELITY security-review follow-up **(d)**.
   `EnactmentEffects.committee_changes` re-typed
   `Option<(Vec<Hash28>, Vec<(Hash28, u64)>)>` →
   `Option<(Vec<StakeCredential>, Vec<(StakeCredential, u64)>)>`. At
   the FIDELITY close the field was **DORMANT**; after the WRITEBACK
   close above it is **LIVE**. Cluster docs at
   `docs/clusters/completed/ENACTMENT-COMMITTEE-FIDELITY/`.
3. **DREP-VOTE-FIDELITY (DRep-vote credential discriminant fidelity)
   — closed (`62c9020`).** A 2-slice arc discharging the
   COMMITTEE-CRED security-review follow-up **(c)**.
   `GovActionState.drep_votes` re-typed `Vec<(Hash28, Vote)>` →
   `Vec<(StakeCredential, Vote)>`; `governance.rs` `lookup_stake`
   resolves a DRep voter to exactly one `DRep` stake key by
   discriminant — no OR-fallback. Cluster docs at
   `docs/clusters/completed/DREP-VOTE-FIDELITY/`.
4. **COMMITTEE-CRED-FIDELITY — closed (`2aeea16`).** A 2-slice arc
   discharging the OQ5 security-review follow-up **(a)**.
   `ConwayGovState.committee` re-keyed `Hash28` → `StakeCredential`;
   `GovActionState.committee_votes` re-typed similarly. Cluster docs
   at `docs/clusters/completed/COMMITTEE-CRED-FIDELITY/`.
5. **OQ5-CREDENTIAL-FIDELITY — closed (`a3ee2da`).** A 2-slice arc
   that closes the B5-named **OQ-5** collapse. `StakeCredential`
   tuple struct → closed `enum { KeyHash, ScriptHash }`; both era
   decoders preserve the tag; `ConwayGovState` re-keyed `Hash28` →
   `StakeCredential`. `DC-LEDGER-10` introduced and `enforced` via
   the new CI gate. Cluster docs at
   `docs/clusters/completed/OQ5-CREDENTIAL-FIDELITY/`.
6. **Phase 4 cluster B5 (Conway gov-cert accumulation) — closed
   (`651adc9`).** Native total gov dispatch + env fail-fast + checked
   DRep-expiry arithmetic + block-path accumulation (gov_state
   carried forward, B4 observe-and-drop removed). New BLUE module
   `ade_ledger::gov_cert`. `DC-LEDGER-09` introduced and `enforced`.
   Cluster docs at `docs/clusters/completed/PHASE4-B5/`.
7. **Phase 4 cluster B4 (Conway cert-state accumulation,
   fail-closed) — closed (`ee35493`).** Owner-complete Conway cert
   decoder; native owner-tagged apply model; era-dispatched
   fail-closed `accumulate_tx_certs`. `DC-LEDGER-08` introduced and
   `enforced`. Cluster docs at `docs/clusters/completed/PHASE4-B4/`.
8. **Phase 4 cluster B3F (follow-up hardening) — committed
   (`193d2fc`).** Two-slice follow-up: B3F-S1 adds the CI grep-gate
   (flips `DC-TXV-06` `partial` → `enforced`); B3F-S2 hardens
   `decode_conway_certs` (trailing-byte reject + bounded
   preallocation). Strengthens `DC-VAL-06`. Cluster docs at
   `docs/clusters/completed/PHASE4-B3F/`.
9. **Phase 4 cluster B3 (Conway value-conservation accounting) —
   closed (`d766eb0`).** Full Conway preservation-of-value equation
   now enforced for cert/withdrawal txs; B2-S4 early-out removed.
   New BLUE surfaces `ade_codec::conway::cert`,
   `ade_codec::conway::withdrawals`, `ade_ledger::cert_classify`.
   Two new registry rules: `DC-TXV-06` (flipped to `enforced` by
   B3F) and `DC-TXV-07`.
10. **Phase 4 cluster B2 (tx validity agreement) — closed
    (`c1cba82`).** New BLUE `ade_ledger::tx_validity` submodule and
    BLUE/GREEN `ade_ledger::mempool` admission gate. Added 5
    `DC-TXV-*` rules; flipped the two `DC-MEM-*` to `enforced`.
11. **Conway value-conservation: the B2-S4 fail-open and its B3
    completion.** B2-S4 (`617139f`) added
    `check_conway_coin_conservation` with a deliberate early-out;
    B3-S4 (`978c222`) removes it and replaces with the full equation.
12. **Phase 4 cluster B1 (full block validity agreement) — closed
    (`993f363`).** Composes N-A wire + N-B consensus header authority
    + `ade_ledger` body authority into a single block verdict. New
    BLUE `ade_ledger::block_validity` submodule, BLUE
    `consensus_view`, RED `consensus_input_extract`, GREEN `validity`
    testkit harness, `kes_check` fail-closed guard. Opened the
    `DC-VAL-*` registry family.
13. **Phase 4 cluster N-A (network mini-protocols) — closed.** 10
    slices. New BLUE crate `ade_network` with 11 mini-protocol codecs,
    8 state machines, mux frame codec, RED session substrate.
14. **Phase 4 cluster N-B (consensus runtime) — closed (`a0c73e1`).**
    10 slices. New BLUE `ade_core::consensus` module. All 6 CEs
    closed.
15. **CE-N-B-6 follow-mode bridge** — RED `ade_core_interop::follow`
    + live preprod tip-agreement evidence.
16. **Phase 4 cluster N-D (ChainDB persistence) — closed (`436b1d7`).**
    Slices S-33 → S-37; CE-N-D-1 closure evidence (1000/1000
    stress-kill iterations).
17. **Phase 2C close-out / CE-73 reclassification** — single commit
    splitting CE-73 into a Tier-2 semantic gate (enforced via
    `ci_check_hfc_translation.sh`) and an explicit Tier-4 bytes
    non-goal.
18. **IDD canonicalization** — `chore(idd)` commits that make the
    repo legible to the global IDD slash commands.
19. **Grounding-doc generation + ripple** — successive refreshes at
    each cluster close from `a87c3a3` through `3d94c22`.
20. **BLUE-list drift closure** — `5b70bee` extended six CI scripts
    to the full 6-crate BLUE scope.
21. **Corpus relayout** — credentialed `*_registered_creds.txt`
    removed (~7M-line negative line count); 12 boundary-block sets
    re-extracted; consensus/validity/tx_validity/B3 corpora added.
    The **post-3d94c22 testkit thread** regenerated the multi-GB
    snapshot tarballs via the db-truncater + cardano-node v1-in-mem
    chained recipe (canonical home `s3://ade-corpus-snapshots`,
    `.gitignore`-d locally) and committed the
    `emit_reward_provenance` generator.

---

## 1. Commit Log

| Hash | Type | Summary |
|------|------|---------|
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
| `ade_codec::conway::cert` (new file in an existing BLUE crate) | BLUE | **Conway-complete certificate decoder** with a *closed* wire grammar. `decode_conway_certs` decodes the full Conway certificate array over tags `0..18`; tags `5`/`6` are not valid; unrecognized tag → deterministic `CodecError::UnknownCertTag { tag, offset }` reject. **B3F-S2 hardened it**: trailing-byte reject (`CodecError::TrailingBytes`), bounded preallocation. | `conway/cert.rs` | PHASE4-B3 / B3-S1, B3-S2; strictness PHASE4-B3F / B3F-S2 |
| `ade_codec::conway::withdrawals` (new file in an existing BLUE crate) | BLUE | Conway withdrawals-map decoder. Decodes `{ RewardAccount => Coin }` into a canonical ordered form, summing to an `i128` consumed-side term; duplicate key → `CodecError::DuplicateMapKey { offset }`. | `conway/withdrawals.rs` | PHASE4-B3 / B3-S3 |
| `ade_ledger::cert_classify` (new file in an existing BLUE crate) | BLUE | **Closed cert-deposit classification** — `classify(state, cert)` is total, era-versioned, resolves every cert variant to exactly one `CertDisposition` over `DepositEffect` with coin sourced via a closed `CoinSource`. **B3F-S1** added the CI grep-gate guarding `classify`'s exhaustiveness. | `cert_classify.rs` | PHASE4-B3 / B3-S2; closure gate B3F / B3F-S1 |
| `ade_ledger::gov_cert` (new file in an existing BLUE crate) | BLUE | **Native Conway governance-certificate accumulation**. `apply_conway_gov_cert(gov_state, cert, env)` is a pure, total dispatch over the owner-complete `ConwayCert`; mutates **only** governance-owned fields of `ConwayGovState`. `GovCertEnv` is required only by tags 16/18; absent `drep_activity` is structured fail-fast. | `gov_cert.rs` | PHASE4-B5 / B5-S2; B5-S1 env; B5-S3 block-path; B5-S5 checked arithmetic |
| `ade_ledger::tx_validity` (new submodule of an existing BLUE crate) | BLUE | **Per-transaction verdict authority**. Closed `TxValidityVerdict` / `TxRejectClass` / `TxValidityError`. `required_signers` enumerates over a closed `SignerSource`. `tx_phase_one` composes witness closure + state-backed checks; `tx_validity` is the pure transition. | `mod.rs`, `verdict.rs`, `required_signers.rs`, `witness.rs`, `phase1.rs`, `transition.rs`, `encoding.rs` | PHASE4-B2 / B2-S1, B2-S2 |
| `ade_ledger::mempool` (new submodule of an existing BLUE crate) | BLUE (`admit`) / GREEN (`policy`) | Two-layer mempool: BLUE `admit` requires `tx_validity` Valid; GREEN `policy` does eviction/ordering, never calls `tx_validity`. | `mod.rs`, `admit.rs`, `policy.rs` | PHASE4-B2 / B2-S5 |
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
| `ade_core_interop` (new workspace crate) | RED | Live cardano-node interop driver for CE-N-B-6; no authoritative decisions. | `src/lib.rs`, `src/follow.rs`, `src/bin/live_consensus_session.rs`, `tests/` | PHASE4-N-B / S-B10; follow-bridge `e5f1f64` |
| `ade_testkit::consensus` (new submodule of an existing crate) | GREEN | Test-only harness for consensus replay corpora. | `consensus/mod.rs`, `consensus/corpus.rs`, `consensus/ledger_view_stub.rs`, `consensus/stream_replay.rs` | PHASE4-N-B / S-B1, S-B6, S-B8 → S-B10 |
| `ade_runtime::chaindb` | RED | Block-store abstraction and impls. Trait surface Tier 1; backing-store choice Tier 5. | `mod.rs`, `types.rs`, `error.rs`, `in_memory.rs`, `persistent.rs` (redb), `contract.rs`, `snapshot_contract.rs`, `crash_safety.rs` | PHASE4-N-D / S-33 → S-35 |
| `ade_runtime::recovery` | RED | Composes ChainDb + SnapshotStore into a generic recovery primitive. | `recovery.rs` | PHASE4-N-D / S-36 |
| `ade_runtime` bin `chaindb_kill_target` | RED | Kill-target child process for the 1,000-kill-9 durability stress harness. | `src/bin/chaindb_kill_target.rs`, `tests/stress_kill_harness.rs` | PHASE4-N-D / S-37 |

Workspace-level membership grew by **two crates** across the full
delta: `ade_network` (PHASE4-N-A) and `ade_core_interop` (PHASE4-N-B).
Both are RED-or-mixed. **The post-3d94c22 testkit thread added no new
module, no new crate, no new CI script, no new rule** — its changes
are entirely in `ade_testkit` (GREEN) and `.gitignore`. **None of B3,
B3F, B4, B5, OQ5, COMMITTEE-CRED-FIDELITY, DREP-VOTE-FIDELITY,
ENACTMENT-COMMITTEE-FIDELITY, or ENACTMENT-COMMITTEE-WRITEBACK added a
new crate**; their surfaces are §3 module modifications. B5 added one
new BLUE module (`ade_ledger::gov_cert`).

Crate dependency shape at HEAD is unchanged since the WRITEBACK regen
(no manifest dep added by the post-3d94c22 testkit thread).

Corpora at HEAD: N-A capture corpus, N-B replay corpus, B1 validity
corpus, B3 conservation corpora, B4/B5 README-only synthetic notes,
plus the credential-fidelity corpus added in OQ5-S2. **The
post-3d94c22 testkit thread** regenerated the multi-GB snapshot
tarballs (now `.gitignore`-d locally; canonical home
`s3://ade-corpus-snapshots`) and added the canonical Conway-layout
`reward_provenance` generator (`emit_reward_provenance.rs`,
`#[ignore]`-gated). No new corpus files are committed by this thread.

Cross-reference: CODEMAP at the 3d94c22 refresh records
`apply_committee_enactment` + the structured `UpdateCommittee` + the
fail-closed snapshot-loader cold-credential parsers; SEAMS records
the WRITEBACK seam as WIRED+CLOSED. The post-3d94c22 testkit thread
does not affect CODEMAP / SEAMS row content (GREEN-scope only); the
`docs/ade-TRACEABILITY.md` working-tree modification is the in-flight
grounding-doc refresh for `DC-EPOCH-01` / `DC-LEDGER-10` adding the
new `committee_oracle_mainnet_575_576_noop_agreement` test entry on
both rules.

---

## 3. Modules Modified

| Module | Scope | Key changes |
|--------|-------|-------------|
| `ade_ledger` | +57 source/test files over the full delta. **Post-3d94c22: no `ade_ledger` source change** — the new testkit thread is bounded to `ade_testkit` + `.gitignore` + `docs/ade-invariant-registry.toml` (two `tests`-array extensions + `authority_surface`/`open_obligation` rewrites in `b9cfaf9`). | **B3:** the closed cert-deposit classifier `cert_classify.rs` and the full Conway value-conservation accounting in `conway.rs` (cert/withdrawal early-out **REMOVED**); new error variants; `ConwayOnlyDepositParams` + `conway_deposit_view()` in `pparams.rs`/`state.rs`; Conway deposit-param fold added to the state fingerprint (byte-identical pre-Conway). **B2:** `tx_validity/` + `mempool/` submodules + B2 integration tests; B2-S4 first cut of `check_conway_coin_conservation`. **B1:** `block_validity/`, `consensus_view.rs`, `consensus_input_extract.rs`, the `ade_core` dep edge. **B3F:** no source change (CI grep-gate added). **B4:** `delegation.rs` (+385) native owner-tagged apply model — `ConwayCertAction` total over 18 Conway tags; `rules.rs` (+212) fail-closed `accumulate_tx_certs` (`_era` discard + swallows removed); `cert_classify.rs` (+100) re-pointed at owner-complete `ConwayCert`. New corpus `tests/cert_state_corpus.rs`. **B5:** new BLUE module `gov_cert.rs` (+366); `state.rs` (+56) `GovCertEnv` + `gov_cert_env()`; `pparams.rs` (+8) `drep_activity`; `error.rs` (+16) two new variants; `fingerprint.rs` (+14) tag 2→3 array (golden regenerated); `rules.rs` (+161/−42) thread `Option<ConwayGovState>` + carry-forward. New corpus `tests/gov_state_corpus.rs`. **OQ5:** `state.rs` (+11) re-key `Hash28` → `StakeCredential`; `fingerprint.rs` (+78) `write_stake_credential` emits discriminant+hash; ripples across `gov_cert.rs`/`governance.rs`/`cert_classify.rs`/`rules.rs`. New `tests/credential_fidelity_corpus.rs`. **COMMITTEE-CRED-FIDELITY:** `state.rs` (+2) re-key committee; `governance.rs` (+76) full-credential-equality ratification; `fingerprint.rs` (+18) `write_committee_vote_list`. **DREP-VOTE-FIDELITY:** `governance.rs` (+57) exact-variant DRep resolution; `fingerprint.rs` (+6) writer rename. **ENACTMENT-COMMITTEE-FIDELITY:** `governance.rs` (+30) `EnactmentEffects.committee_changes` re-typed (dormant pin at the FIDELITY close). **ENACTMENT-COMMITTEE-WRITEBACK:** `governance.rs` (+~189) `enact_proposals` populates `committee_changes` + new `committee_threshold`; new `apply_committee_enactment`; `rules.rs` (+~53) epoch-boundary call site; `fingerprint.rs` (+~88) structured `write_gov_action`. |
| `ade_codec` | +11 source/test files (B3 + B3F + B4 + OQ5). **No post-3d94c22 change.** | **B3:** new `conway::cert` decoder + `conway::withdrawals` decoder; `error.rs` (+13) `UnknownCertTag` / `DuplicateMapKey`. **B3F-S2:** `decode_conway_certs` hardened (trailing-byte reject, bounded preallocation). **B4-S1:** `decode_conway_certs` owner-complete; new `decode_drep`; `read_pool_registration_cert` returns `pool_owners`. **OQ5-S1:** both era `decode_stake_credential` preserve the key/script tag. |
| `ade_types` | +3 files (B3) + 2 files (B4) + governance ripples through the FIDELITY clusters. **No post-3d94c22 change.** | **B3:** closed `ConwayCert` enum + classification types; `RewardAccount`. **B4-S1:** `ConwayCert` owner-complete; new `DRep` enum; `PoolRegistrationCert.owners`. **OQ5-S1:** `StakeCredential` tuple struct → closed enum + `hash()` accessor. **COMMITTEE-CRED-FIDELITY-S1:** `GovActionState.committee_votes` re-typed to carry `StakeCredential`. **DREP-VOTE-FIDELITY-S1:** `GovActionState.drep_votes` re-typed same way. **ENACTMENT-COMMITTEE-WRITEBACK-S1:** `GovAction::UpdateCommittee` re-shaped from `{ prev_action, raw: Vec<u8> }` to `{ prev_action, removed: BTreeSet<StakeCredential>, added: BTreeMap<StakeCredential, u64>, threshold: (u64, u64) }`. |
| `ade_core` | +29 source files + tests (N-B); +828 / −86 across 16 files (B1). **No post-B1 change.** | **N-B:** substantive BLUE consensus module under `src/consensus/`. **B1:** `consensus/kes_check.rs` + single-VRF + KES wiring. |
| `ade_crypto` | 1 file, +24 / −81 lines (B1). | `kes.rs` (`500589b`): `build_opcert_signable` fixed as part of B1-S5. |
| `ade_core_interop` | +1,546 across 6 files (B1). | CE-N-B-6 follow-bridge (`e5f1f64`) + pin retarget (`807bcb6`). |
| `ade_network` | 100 files, +17,861 lines. | DoS hardening of 6 codecs (`744ef34`, post-N-A close). |
| `ade_runtime` | +18 files, +3,440 lines (N-B `consensus/` + N-D `chaindb`/`recovery`; B1 one small touch). **No post-B1 ripple.** | **N-B:** new `consensus/` submodule. **B1:** one small touch. N-D `chaindb`/`recovery` are §2 New Modules. |
| `ade_testkit` | +28 files across the full delta to 3d94c22; **post-3d94c22: 5 test files modified + 1 new test file (`emit_reward_provenance.rs`) + `harness/snapshot_loader.rs` (+78 lines, FU #1 tip slot + FU #4 Conway UMElem)** | **N-B:** `consensus/` harness. **B1:** `validity/` harness. **B2:** `tx_validity/` submodule. **B3:** extended `harness/snapshot_loader.rs` (+20, intra-corpus resolution), `tx_validity/{adversarial,valid_synthetic}.rs` extensions. **OQ5 → WRITEBACK:** progressive extensions of the snapshot loader to preserve key/script tags on gov-map / committee / DRep-vote / structured-`UpdateCommittee` decodes (WRITEBACK-S1 added fail-closed `parse_cold_credential` / `_set` / `_epoch_map` + `parse_unit_interval`). **WRITEBACK close-hardening (`69e2d4b`):** `parse_cold_credential_set` / `parse_cold_credential_epoch_map` reject truncated headers via a `terminated` flag (fail-closed at the container level). **Post-3d94c22 thread (`b9cfaf9`/`396664a`/`c78ec76`/`168ac02`):** (i) **new test** `tests/epoch_oracle_comparison.rs::committee_oracle_mainnet_575_576_noop_agreement` (+~116 lines in the file) wires the real-chain committee oracle at mainnet 575→576 — snapshot-gated, asserts 7-`ScriptHash` interim committee parse fidelity + no-op write-back agreement; (ii) **corpus-alignment ripples** in `boundary_fingerprint_agreement.rs` (11 pinned fingerprint hashes regenerated, pin-mismatch detection still strict), `boundary_stateful_replay.rs` / `transition_proof_surface.rs` / `contiguous_plutus_verdict_harness.rs` / `late_era_composer_integration.rs` / `epoch_oracle_comparison.rs` / `emit_divergent_fixture.rs` (skip-until-first-success replaces fragile break-on-first-error; no ledger-rule check weakened); (iii) **new `tests/emit_reward_provenance.rs`** (+67, `#[ignore]`-gated); (iv) **`harness/snapshot_loader.rs` follow-ups in `168ac02`** — FU #1 surfaces the tip slot in `SnapshotHeader` (extracts from `HeaderState` `WithOrigin (AnnTip …)`; propagates into `state.epoch_state.slot`); FU #2 apply tests pre-filter blocks by `slot > state.epoch_state.slot.0`; FU #4 `parse_registered_credentials` discriminates UMElem layouts by `UMElem[0]` major type (pre-Conway `array(major=4)` vs Conway compact `uint(major=0)` over `array(4) [reward, deposit, nullable Pool, nullable DRep]`). 34/34 snapshot_loader unit tests green; `boundary_stateful_replay` 4 passed, `transition_proof_surface` 2 passed, `late_era_composer_integration` 10 passed. **GREEN-scope only — no BLUE-crate change, no determinism surface moves.** |

No other crate had non-trivial source changes since baseline.
`ade_plutus` and `ade_node` were untouched by code commits. **The
post-3d94c22 testkit thread touched only `ade_testkit` and
`.gitignore`** (the registry edit in `b9cfaf9` is two `tests`-array
extensions + two `authority_surface`/`open_obligation` text rewrites
on `DC-EPOCH-01` and `DC-LEDGER-10`; no new rule, no schema change).
`.idd-config.json` is untouched in this thread.

---

## 4. Feature Flags

No Cargo `[features]` tables exist at HEAD in any workspace crate, and
none existed at baseline. The project does not use Cargo feature flags
as a semantic surface — closed semantic surfaces are encoded in the
type system per the IDD core principles, and conditional compilation
is checked out of BLUE code via `ci/ci_check_no_semantic_cfg.sh`
(scoped over the full 6-crate BLUE set, covering all surfaces
introduced through the WRITEBACK and post-WRITEBACK threads).

No `#[cfg(feature = ...)]` gates appear at either ref. `cardano-crypto`
(`vrf-draft03`) and `minicbor` (`alloc`) feature selections in the
dependency entries are upstream-crate selections, not Ade-side flags.

**Status: unchanged — zero Ade feature flags at baseline, zero at HEAD.**

---

## 5. CI Checks

The CI surface is the shell-script set under `ci/` (no
`.github/workflows` in this repo). At baseline there were 15 scripts.
At HEAD there are **29 scripts plus one git hook** (`ci/git-hooks/commit-msg`):
CE-73 added one, N-D added three, N-A added two, N-B added four, B3
added one, B3F added one, B5 added one, OQ5 added one (the 29th — the
`ci_check_credential_discriminant_closed.sh` gate). **B1, B2, B4,
COMMITTEE-CRED-FIDELITY, DREP-VOTE-FIDELITY,
ENACTMENT-COMMITTEE-FIDELITY, ENACTMENT-COMMITTEE-WRITEBACK each
added no new CI script** — they either reused an existing closed-enums
script or (the FIDELITY clusters and WRITEBACK) **extended the same
OQ5 credential gate**. **The post-3d94c22 testkit thread added no
new CI script** — the new
`committee_oracle_mainnet_575_576_noop_agreement` test is a runtime
oracle assertion, the corpus-alignment ripples are GREEN tests, and
the loader FUs are GREEN harness changes; none requires a new standing
CI invariant. Grouped by cluster.

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
| same | **Modified** (`2aeea16`, COMMITTEE-CRED-FIDELITY-S2) | +2 committee clauses: `ConwayGovState.committee` stays `StakeCredential`-keyed; `GovActionState.committee_votes` carries `StakeCredential`. |
| same | **Modified** (`62c9020`, DREP-VOTE-FIDELITY-S2) | +2 DRep clauses: `GovActionState.drep_votes` carries `StakeCredential`; no `DRep::KeyHash(...).or_else` OR-fallback in `ade_ledger::governance`. |
| same | **Modified** (`a6b8de7`, ENACTMENT-COMMITTEE-FIDELITY-S1) | +1 enactment-effect clause (clause 6): `EnactmentEffects.committee_changes` carries `StakeCredential`. |
| same | **Modified** (`69e2d4b`, ENACTMENT-COMMITTEE-WRITEBACK close) | +section 7: `GovAction::UpdateCommittee.removed` is `BTreeSet<StakeCredential>` and `.added` is `BTreeMap<StakeCredential, _>` (never opaque raw bytes); `apply_committee_enactment` is present in `governance.rs` and called from `rules.rs`. |
| same | **Unmodified post-3d94c22** | The post-3d94c22 testkit thread does not extend this gate — the real-chain oracle is a runtime test, not a standing CI invariant. The gate stays the **29th** script. |

TRACEABILITY cross-reference: every script listed above appears as a
`ci_script` for at least one rule in `docs/ade-invariant-registry.toml`,
re-traced via `ci/ci_check_constitution_coverage.sh`. **The
post-3d94c22 testkit thread** extended `DC-EPOCH-01.tests` and
`DC-LEDGER-10.tests` each by one entry
(`committee_oracle_mainnet_575_576_noop_agreement`) and rewrote both
rules' `authority_surface` / `open_obligation` (reality-blocked vs
environment-blocked for the committee CHANGE oracle); no `ci_script`
change, registry total **stays 173**.

---

## 6. Canonical Type Registry Delta

n/a — `.idd-config.json` `canonical_type_registry` is null. Canonical-type
rules live inline in the invariant registry under family `T`.

---

## 7. Normative Rule Delta

The project's invariant registry tracks structured rules (TOML), not
prose normative-doc rules; this section reports on it.

- Rules at baseline: **147** (in `constitution_registry.toml`)
- Rules at HEAD: **173** (in `docs/ade-invariant-registry.toml`)
- Net additions: **+26** (PHASE4-N-A: 2; PHASE4-N-B: 8; PHASE4-B1: 6;
  PHASE4-B2: 5; PHASE4-B3: 2; PHASE4-B3F: 0; PHASE4-B4: 1
  (`DC-LEDGER-08`); PHASE4-B5: 1 (`DC-LEDGER-09`); OQ5: 1
  (`DC-LEDGER-10`); COMMITTEE-CRED-FIDELITY / DREP-VOTE-FIDELITY /
  ENACTMENT-COMMITTEE-FIDELITY / ENACTMENT-COMMITTEE-WRITEBACK / the
  post-3d94c22 testkit thread: 0 each — all strengthenings of
  existing rules in place). The two `DC-MEM-*` rules were *introduced
  earlier* and flipped to `enforced` in B2, not counted as new.

- Removals: **0** (expected under append-only discipline; clean).

- Strengthenings (`declared`/`partial` → `enforced`, or tightened) at
  HEAD:
  - **`DC-EPOCH-01`** (ENACTMENT-COMMITTEE-WRITEBACK, `3180e27` /
    `3d94c22`; **post-3d94c22 `b9cfaf9`**): strengthened —
    `apply_committee_enactment` is now wired at the epoch boundary,
    so a ratified `NoConfidence` / `UpdateCommittee` is no longer
    observed-and-dropped. `strengthened_in +=
    "ENACTMENT-COMMITTEE-WRITEBACK"`. `tests`: WRITEBACK added 4
    (`enact_noconfidence_dissolves_committee`,
    `enact_update_committee_applies_changes`,
    `committee_enactment_replays_byte_identical`,
    `epoch_boundary_ratified_noconfidence_dissolves_committee`); the
    post-3d94c22 thread added a 7th
    (`committee_oracle_mainnet_575_576_noop_agreement`).
    `authority_surface` rewritten to record the confirmed real-chain
    committee parse + no-op write-back agreement at mainnet 575→576.
    Stays `partial` (governance-Plutus oracle still deferred to
    CE-88, the positive committee-CHANGE case reality-blocked).
  - **`DC-LEDGER-10`** (OQ5 → COMMITTEE-CRED-FIDELITY →
    DREP-VOTE-FIDELITY → ENACTMENT-COMMITTEE-FIDELITY →
    ENACTMENT-COMMITTEE-WRITEBACK → post-3d94c22 testkit thread):
    strengthened five times since introduction (`enforced` from
    OQ5). `strengthened_in = [OQ5-CREDENTIAL-FIDELITY,
    COMMITTEE-CRED-FIDELITY, DREP-VOTE-FIDELITY,
    ENACTMENT-COMMITTEE-FIDELITY, ENACTMENT-COMMITTEE-WRITEBACK]`.
    `tests` total: **20** at HEAD (8 OQ5 + 3 COMMITTEE-CRED + 2
    DREP-VOTE + 1 ENACTMENT-COMMITTEE-FIDELITY + 5
    ENACTMENT-COMMITTEE-WRITEBACK + 1
    `committee_oracle_mainnet_575_576_noop_agreement` from
    `b9cfaf9`). `open_obligation` rewritten by `b9cfaf9` — the
    **committee-credential discriminated-key agreement is now
    CONFIRMED** at mainnet 575→576 (loader sees the interim committee
    as 7 `ScriptHash`, never defaulted `KeyHash`; ratification +
    enactment reproduce epoch 576 with no spurious mutation); the
    positive committee-CHANGE case is **reality-blocked** (mainnet
    enacted no such change in this range), not environment-blocked;
    the non-committee discriminated keys (`vote_delegations` /
    `drep_expiry`) remain environment-blocked pending those
    extractions. `ci_script` unchanged
    (`ci_check_credential_discriminant_closed.sh`); registry **stays
    173**, CI **stays 29**.
  - **`DC-LEDGER-08`** (B5, `fdb6601` / `d63c700`): strengthened —
    B4's "routed out-of-mutation-scope" disposition for governance
    certs is retired; those certs are now applied (`DC-LEDGER-09`).
    Recorded via `cross_ref` (see Anomalies).
  - **`T-DET-01` / `T-ENC-03`** (OQ5, `4187330` / `a3ee2da`):
    strengthened — the canonical fingerprint and credential encoding
    now carry the key/script discriminant.
  - **`DC-TXV-06`** (B3F, `d6c1993`): `partial` → **`enforced`**.
  - **`DC-VAL-06`** (B3F, `193d2fc`; B4, `302d22c`): strengthened
    (trailing-byte reject, bounded preallocation; fail-closed
    cert-state accumulation).
  - **`T-CONSERV-01` / `CN-LEDGER-07`** (B3, `978c222`): the
    preservation-of-value invariant strengthened to the full Conway
    equation.
  - **`DC-TXV-03`** (B3): tests extended with the conservation
    corpora.
  - **`DC-MEM-01`, `DC-MEM-02`** (B2, `85a50dc`): `declared` →
    `enforced`. `DC-TXV-03`, `DC-VAL-06`, `DC-LEDGER-02` previously
    strengthened by B2-S4 (`617139f`).
  - Earlier-delta strengthenings (unchanged): `DC-EPOCH-02`
    (`9b15378`); the N-D bundle (`78da6c9`); the N-A real-capture
    bundle; `T-CORE-02` (S-B1).
  - The six `DC-VAL-*` and five `DC-TXV-01..05` rules each record
    `strengthened_in` containing their introducing cluster — recorded
    faithfully; see Anomalies.

Family counts at HEAD: CN dominates (~64), DC grew most across the
delta (now including `DC-CONS` ×8, `DC-VAL` ×6, `DC-TXV` ×7, `DC-MEM`
×2, and `DC-LEDGER-08`/`-09`/`-10` joining the existing `DC-LEDGER`
rules), T = 30, RO/OP combined ×9.

Normative-doc rule extraction (the `normative_docs` list in
`.idd-config.json`) is approximate and not regenerated here — the
structured registry is the authoritative source.

---

## Anomalies and Cross-Reference Warnings

- **Post-3d94c22 testkit thread (`b9cfaf9` / `396664a` / `c78ec76` /
  `168ac02`) — four GREEN-scope commits with one registry-level
  effect.** No new module, no new crate, no new rule, no new CI
  script, no BLUE source change. The only registry change is the
  in-place strengthening of `DC-EPOCH-01.tests` and
  `DC-LEDGER-10.tests` (each `+=
  committee_oracle_mainnet_575_576_noop_agreement`) + the
  `authority_surface` / `open_obligation` text rewrites on both rules
  to record the confirmed mainnet 575→576 committee parse + no-op
  write-back agreement (`b9cfaf9`). `DC-LEDGER-10` reaches **20**
  tests at HEAD (was 19 at the WRITEBACK refresh). `396664a` aligns
  11 previously-blocked `ade_testkit` tests with the regenerated
  boundary corpus; `c78ec76` adds the `#[ignore]`-gated
  `reward_provenance` generator; `168ac02` closes three coherent
  snapshot-loader follow-ups. All four commits carry the
  model-attribution trailer.
- **`open_obligation` reclassification on `DC-EPOCH-01` /
  `DC-LEDGER-10`: environment-blocked → reality-blocked (committee
  CHANGE case).** Until `b9cfaf9`, the real-chain
  committee-transition oracle was reported as environment-blocked.
  `b9cfaf9` clarifies the distinction: the **committee parse + no-op
  write-back oracle is now confirmed** at mainnet 575→576 (snapshots
  in `s3://ade-corpus-snapshots`, gitignored locally per
  `feedback_no_credential_leaks` discipline); the **positive
  committee-CHANGE case is reality-blocked** — mainnet enacted no
  `UpdateCommittee` / `NoConfidence` across the 575→576 boundary, so
  no snapshot pair exhibits a committee delta to diff. Not a
  regression, not a fail-open; the transition is exercised by
  synthetic positive + replay-byte-identical + key/script-distinct
  tests + the real-chain no-op oracle. The non-committee
  discriminated keys (`vote_delegations` / `drep_expiry` vs the
  `Credential`-keyed UMap/VState) remain environment-blocked pending
  those extractions.
- **Cluster docs archived (uncommitted at this HEAD — staged
  renames).** Seven cluster directories were moved via `git mv`
  (staged in the index, not yet committed) from
  `docs/clusters/<NAME>/` to `docs/clusters/completed/<NAME>/`:
  COMMITTEE-CRED-FIDELITY, DREP-VOTE-FIDELITY,
  ENACTMENT-COMMITTEE-FIDELITY, ENACTMENT-COMMITTEE-WRITEBACK,
  OQ5-CREDENTIAL-FIDELITY, PHASE4-B3F, PHASE4-B4, PHASE4-B5. This
  HEAD_DELTAS references their **archive paths**
  (`docs/clusters/completed/<NAME>/`). At HEAD, `docs/clusters/`
  contains only `PHASE4-N-B/` (which is itself a non-IDD log
  directory: `CE-N-B-6_2026-05-20.log`; the substantive N-B cluster
  docs were archived at the N-B close). All archived cluster
  references are consistent with the staged tree; the
  `docs/ade-TRACEABILITY.md` working-tree modification is the
  in-flight grounding-doc refresh that will land alongside the
  archive-moves commit.
- **In-flight working-tree state at this HEAD.** `git status` shows
  one modified file (`docs/ade-TRACEABILITY.md`) and 22 staged
  renames (the 8 cluster-dir archival moves above span 22 file
  renames). No source code or BLUE surface is in the working tree;
  the regeneration scope is doc / archive only. **The branch is 5
  commits ahead of `origin/main`** (`b9cfaf9`, `c78ec76`, `396664a`,
  `3d94c22`, `168ac02`) — expected for an unpushed close-and-archive
  flow. (`3d94c22` is the WRITEBACK grounding-doc refresh; the
  remaining four are the post-3d94c22 testkit thread.)
- **ENACTMENT-COMMITTEE-WRITEBACK fingerprint change (T-DET-01,
  deliberate; carried forward).** `write_gov_action` emits the
  structured `UpdateCommittee` shape `[5, prev, set<cred>,
  {cred=>epoch}, num, den]` in place of the opaque `[4, prev,
  bytes]`. Committee enactment is live, so the dormant-`None`
  no-golden-drift invariant no longer holds for the
  `UpdateCommittee` path. Confirm `ci_check_ledger_determinism.sh` /
  the fingerprint golden test reflect the structured encoding on the
  next determinism replay (exercised by the structured-fingerprint
  and `committee_enactment_replays_byte_identical` tests + the new
  `committee_oracle_mainnet_575_576_noop_agreement` real-chain
  oracle).
- **WRITEBACK carry-forward follow-ups (narrowed, unchanged).**
  FIDELITY follow-up **(d)** is RESOLVED + WIRED. The remaining
  **(e)** GREEN loader `mk_credential` `tag != 1` → `KeyHash` default
  is **narrowed** — the new `parse_cold_credential` IS fail-closed on
  unknown tags, so (e) now applies only to the older `mk_credential`
  helper (contained to `ade_testkit`, cannot reach the node binary).
  The pre-OQ5 **(b)** Shelley unknown-cert zero-hash placeholder
  remains a WARN LOW non-goal.
- **ENACTMENT-COMMITTEE-FIDELITY / DREP-VOTE-FIDELITY /
  COMMITTEE-CRED-FIDELITY / OQ5-CREDENTIAL-FIDELITY closures —
  carried forward as written in the prior regen.** Each cluster's
  recorded follow-ups, fingerprint surfaces, golden-drift posture,
  and real-chain oracle status are unchanged at this HEAD, **except**
  that the committee-credential parse + no-op write-back agreement vs
  cardano-node is now real-chain-confirmed (per the top anomaly). All
  cluster docs at `docs/clusters/completed/<NAME>/`.
- **B5 / B4 / B3F / B3 / B2 / B1 / N-D / N-B / N-A closures —
  carried forward unchanged.** All cluster docs at
  `docs/clusters/completed/<NAME>/` (B3 already archived; the others
  are part of the staged-but-uncommitted moves above; PHASE4-N-A,
  PHASE4-B1, PHASE4-B2, PHASE4-B3, PHASE4-N-D had already been
  archived at their respective closes). The B4 governance
  observe-and-drop, the B5 observe-and-drop closure, and the
  WRITEBACK live committee write-back form a complete chain.
- **`DC-LEDGER-08` strengthening recorded via `cross_ref`, not
  `strengthened_in` (carried forward).** B5 strengthens
  `DC-LEDGER-08`, recorded via bidirectional `cross_ref` to
  `DC-LEDGER-09` rather than appending `"PHASE4-B5"` to
  `DC-LEDGER-08.strengthened_in`. Harmless; consider normalizing on
  the next registry curation pass.
- **DC-VAL status mismatch vs. closure claim (B1, carried forward).**
  PHASE4-B1 is reported fully closed, but in the registry only
  `DC-VAL-01` is `enforced` — `DC-VAL-02` → `DC-VAL-05` remain
  `declared` despite named tests and the extended closed-enums
  enforcement point. Flip on the next `/traceability` pass.
- **`strengthened_in` records the introducing cluster on
  freshly-created rules (carried forward).** Each `DC-VAL-*` records
  `["PHASE4-B1"]` and each `DC-TXV-01..05` records `["PHASE4-B2"]`
  even though those clusters *created* the families. Harmless.
- **`ade_ledger -> ade_core` dependency edge (B1, carried forward).**
  First ledger→consensus edge. Both BLUE.
- **`ade_crypto::kes::build_opcert_signable` fixed in B1-S5
  (`500589b`, carried forward).** BLUE crypto-surface behavioral
  change.
- **B3 positive corpus carves out Plutus per CE-88 (carried
  forward).** The real epoch-576 positive conservation corpus drives
  10 non-Plutus cert/withdrawal txs to `Valid`; Plutus-witnessing
  txs excluded because CE-88 is externally blocked.
- **Adversarial corpora are derived, not committed (carried
  forward).** `corpus/validity/` (B1), `corpus/tx_validity/` (B2),
  and the B3 adversarial conservation cases are generated
  deterministically at test time. The B3 positive oracle is
  committed.
- **Corpus relayout: credentialed snapshots removed, then regenerated
  off-repo.** Deleted `corpus/snapshots/reward_provenance/*_registered_creds.txt`
  dominates the ~7M-line negative line count; replaced by 12
  re-extracted boundary-block sets at exact era-boundary slots. The
  **post-3d94c22 testkit thread** added `corpus/snapshots/` to
  `.gitignore` (canonical home is `s3://ade-corpus-snapshots`) and
  committed the `emit_reward_provenance` generator (`c78ec76`) so
  that `*_tick_registered_creds.txt` is reproducible from snapshot
  tarballs without committing credential bytes to git (per
  `feedback_no_credential_leaks`).
- **`ade_core_interop` tests `#[ignore]`-gated / offline-replay by
  design (carried forward).** Live tip-agreement not run in CI.
- No removed canonical types (n/a — no separate registry).
- No removed registry rules (expected: 0; actual: 0). OQ5 added
  `DC-LEDGER-10`; net +1 since B5, +26 since baseline. All FIDELITY
  / WRITEBACK / post-3d94c22 commits added **no rule** — they
  strengthened `DC-LEDGER-10` / `DC-EPOCH-01` in place. Registry
  total stays **173** at HEAD.
- **All commit subjects carry a conventional-commits prefix or are
  cluster-close housekeeping.** The four `Close PHASE4-*` commits
  and the bare `chore:` commits are classified `chore` on scope
  grounds. The post-3d94c22 thread (`b9cfaf9` test(ledger),
  `c78ec76`/`396664a` test(corpus), `168ac02` fix(testkit)) is
  conventional. All five carry the repo-required `Co-Authored-By`
  model-attribution trailer.

---

## Generation Notes

Regenerate via `/head-deltas <baseline>` or by re-running the
`head-deltas-generator` agent with the same baseline. Baseline lives
in `.idd-config.json` `head_deltas_baseline` (still `d509f02` —
**this is a cluster-close-level follow-up refresh, not a phase
boundary, so the baseline is unchanged**). Update the baseline on the
next phase boundary (Phase 4 close). Note the commit-hash rewrite
caveat at the top — re-derive hashes from `git log` at each regen
rather than carrying them forward. This regen is cut at committed
HEAD `168ac02` (snapshot-loader follow-ups), with the
`docs/ade-TRACEABILITY.md` working-tree refresh + the 22 staged
cluster-dir archive renames pending a single grounding-doc commit.
The prior regen narrated HEAD `3180e27`
(ENACTMENT-COMMITTEE-WRITEBACK-S2); the new span is
`3180e27..168ac02` — 6 commits (`69e2d4b` close hardening, `3d94c22`
grounding refresh, `b9cfaf9` real-chain committee oracle, `396664a`
corpus alignment, `c78ec76` reward_provenance generator, `168ac02`
snapshot-loader follow-ups). The branch is 5 commits ahead of
`origin/main`.
