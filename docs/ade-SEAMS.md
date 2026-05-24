# Seams — Where New Work Can Attach (Ade)

> **Status:** Living architectural document. Regenerated; not hand-edited.
> Per-project instance of `~/.claude/methodology/templates/seams.md`.

> 11 crates, 29 CI checks at HEAD (`168ac02`).
> Reads CODEMAP for the module list and TCB colors; reads the invariant
> registry (`docs/ade-invariant-registry.toml`) for rule IDs; reads the
> Phase 4 cluster plan (`docs/active/phase_4_cluster_plan.md`), the
> closed N-D / N-A / N-B / B1 / B2 / B3 / B4 / B5 cluster docs, and the
> OQ5-CREDENTIAL-FIDELITY, COMMITTEE-CRED-FIDELITY, DREP-VOTE-FIDELITY,
> ENACTMENT-COMMITTEE-FIDELITY, and ENACTMENT-COMMITTEE-WRITEBACK cluster
> docs plus their slices.
>
> **This is a post-ENACTMENT-COMMITTEE-WRITEBACK GREEN-only refresh
> (HEAD `168ac02`).** Six commits land between the prior SEAMS HEAD
> (`3180e27`) and this HEAD; ALL six are GREEN-scope only and SHIP NO
> NEW BLUE / RED MODULE, NO NEW CLOSED ENUM, NO NEW INGRESS SURFACE, NO
> NEW COMPOSER, NO NEW CRATE EDGE, NO NEW CI SCRIPT, NO NET NEW
> CANONICAL TYPE, AND NO MUST-NOT CHANGE. Inventory:
> - `3d94c22` — prior grounding refresh (SEAMS, CODEMAP, HEAD_DELTAS,
>   TRACEABILITY) for the ENACTMENT-COMMITTEE-WRITEBACK close. Docs
>   only.
> - `69e2d4b` — testkit harness alignment for the ENACTMENT-COMMITTEE-WRITEBACK
>   close (already covered by the existing §2 governance / credential
>   narrative; no SEAMS shape change).
> - `b9cfaf9` — **new GREEN test** `committee_oracle_mainnet_575_576_noop_agreement`
>   (in `crates/ade_testkit/tests/epoch_oracle_comparison.rs`). Replays the
>   mainnet 575→576 epoch boundary against the real cardano-node snapshot
>   and asserts noop agreement on the committee surface. **Strengthens
>   DC-EPOCH-01 + DC-LEDGER-10** by extending real-chain oracle coverage
>   to the live committee write-back path — a positive-oracle test, not
>   a new seam.
> - `396664a` — `ade_testkit` + `ade_plutus` test compile re-aligned
>   against the regenerated corpus. No source-shape change in any
>   library surface.
> - `c78ec76` — new GREEN, `#[ignore]`-gated `emit_reward_provenance.rs`
>   corpus generator (two tests `emit_alonzo310_registered_creds` /
>   `emit_conway576_registered_creds`); only runs under `cargo test --
>   --ignored`. A re-runnable corpus producer, not a new ingress.
> - `168ac02` (THIS HEAD) — `harness/snapshot_loader.rs` (GREEN)
>   snapshot-loader follow-ups. Three orthogonal hardenings; all sit
>   in the GREEN snapshot-loader and align it with the BLUE schema
>   migrations CODEMAP already records:
>   (1) the live tip slot now flows from the `HeaderState`'s `WithOrigin
>   (AnnTip …)` prefix via the new helper `parse_with_origin_slot`
>   (`array(1) [array(N≥3) [slot, blockNo, hash, …]]`, falling back to
>   `0` for the `Origin` case); `SnapshotHeader` gained a `slot: u64`
>   field in place (GREEN; not in the BLUE canonical-type count);
>   (2) `parse_registered_credentials` is now a **GREEN decoder closure
>   aligned to the BLUE Conway schema migration** — it decodes BOTH the
>   pre-Conway UMElem layout `array(4) [StrictMaybe RDPair, Set Ptr,
>   StrictMaybe Pool, StrictMaybe DRep]` (registered iff RDPair is
>   `SJust`) AND the Conway compact layout `array(4) [uint reward,
>   uint deposit, nullable Pool, nullable DRep]` (registered iff
>   `deposit > 0`); discrimination is by major type of UMElem[0]
>   (`array(major=4)` = pre-Conway, `uint(major=0)` = Conway compact),
>   so anything else flips `is_registered = false` without claiming the
>   credential. This is a GREEN snapshot-loader closure that tracks the
>   BLUE `ConwayGovState` shape change already documented in CODEMAP —
>   it is **not a new SEAM** (no new ingress, no new BLUE chokepoint,
>   no new canonical type), but it is a textbook example of "GREEN
>   decoder closures aligned to BLUE schema migrations" worth naming
>   so future drift refreshes know where to look;
>   (3) `parse_cold_credential_set` and `parse_cold_credential_epoch_map`
>   are now **fail-closed on truncation**: each tracks a `terminated`
>   flag (flipped only by the indefinite `0xff` break or the definite
>   `count >= declared_len` exit) and rejects with `HarnessError::ParseError`
>   if a declared-but-under-length set/map exits the loop without
>   consuming all declared members — replacing the prior silent advance
>   that depended on a downstream field to hit EOF. The new regression
>   `under-length set must reject` in the existing `parse_gov_action`
>   test covers the structured reject. This tightens the same GREEN
>   `update_committee` decode chokepoint the prior revision already
>   surfaced under §2 "Governance ratification / enactment"; it does
>   not move that chokepoint, add a new one, or change a BLUE rule.
>
> **No section below (§1 surfaces, §2 authoritative domains, §3 closed
> + extensible registries, §4 frozen vs. version-gated contracts, §5
> module-addition rules, §6 forbidden patterns) has its module list,
> count, or rule list changed by this refresh.** The counts the body
> reports (376 canonical types, 1325 tests, 29 CI checks) match
> CODEMAP at this HEAD; the SEAMS narrative records 29 CI checks
> because no CI script was added or removed since `3180e27`. The
> ENACTMENT-COMMITTEE-WRITEBACK narrative block immediately below
> remains the load-bearing context for §1 ENACTMENT-COMMITTEE-WRITEBACK
> rows and §2 / §3 / §4 / §6 ENACTMENT-COMMITTEE-WRITEBACK references.
>
> **This is an ENACTMENT-COMMITTEE-WRITEBACK close refresh (HEAD `3180e27`).**
> The body was fully regenerated at PHASE4-B3 close (`7784bf8`), folded in the
> B3F hardening deltas (`193d2fc`), the PHASE4-B4 deltas (`ee35493`), the
> PHASE4-B5 deltas (`644eb03`), the OQ5-CREDENTIAL-FIDELITY deltas (`a3ee2da`),
> the COMMITTEE-CRED-FIDELITY deltas (`2aeea16`), the DREP-VOTE-FIDELITY
> deltas (`62c9020`), and the ENACTMENT-COMMITTEE-FIDELITY delta (`a6b8de7`);
> this revision folds in the ENACTMENT-COMMITTEE-WRITEBACK deltas (S1 `f2f15f9`,
> S2 `3180e27`). **THE KEY ENACTMENT-COMMITTEE-WRITEBACK DELTA:** the
> previously-dormant `UpdateCommittee` enactment LOGIC — the prior revision's last
> remaining open governance-enactment seam, where the `enact_proposals` arm was
> literally `let _ = raw;` — is now **WIRED and CLOSED**. Three things landed:
> (1) `GovAction::UpdateCommittee` (in `ade_types::conway::governance`) moved from
> the opaque `{ prev_action, raw: Vec<u8> }` to the **closed structured variant**
> `{ prev_action, removed: BTreeSet<StakeCredential>, added:
> BTreeMap<StakeCredential, u64>, threshold: (u64, u64) }` (cold committee
> credentials discriminated, never bare `Hash28`); the `GovAction` enum
> **cardinality is unchanged — still a closed 7-variant enum** (one variant
> re-shaped in place). (2) Committee write-back is now the **closed pure
> transition** `ade_ledger::governance::apply_committee_enactment`, called at the
> `ade_ledger::rules` **epoch-boundary apply site** (`rules.rs:1224`); it removes
> the `removed` cold credentials, inserts the `added` ones with their term-expiry
> epochs, and applies the new quorum from the new `EnactmentEffects.committee_threshold:
> Option<(u64, u64)>` field; a ratified `NoConfidence` dissolves the committee.
> (3) The snapshot-loader decode (GREEN) gained fail-closed
> `parse_cold_credential` / `parse_cold_credential_set` /
> `parse_cold_credential_epoch_map` / `parse_unit_interval`, which decode the
> `update_committee` gov-action structure (rejecting malformed input).
> **ENACTMENT-COMMITTEE-WRITEBACK STRENGTHENS DC-EPOCH-01 and DC-LEDGER-10 (no new
> rule, `strengthened_in += ENACTMENT-COMMITTEE-WRITEBACK`)** and **EXTENDS the
> existing CI gate** `ci/ci_check_credential_discriminant_closed.sh` (no new gate,
> no new file — CI count stays 29) with two more checks (6 + 7) that fail if
> `EnactmentEffects.committee_changes` is not `StakeCredential`-typed, if
> `GovAction::UpdateCommittee.removed`/`.added` are not discriminated
> `StakeCredential`, if `governance.rs` does not define
> `apply_committee_enactment`, or if `rules.rs` does not call it at the epoch
> boundary. **It added no new crate, no new module, no new ingress surface, no new
> public composer, and no net new canonical type** — `UpdateCommittee` was
> re-shaped in place (the closed 7-variant `GovAction` is the same already-counted
> type), `apply_committee_enactment` is a function, and `committee_threshold` is a
> field on the existing `EnactmentEffects` struct. **This WIRES AND CLOSES the
> prior revision's "dormant `UpdateCommittee` enactment LOGIC" candidate seam.**
> **The remaining open seam in this area is now ONLY the declared non-goal:**
> decoding `proposal_procedures` from real tx bodies into a typed `GovAction` —
> the wire codec (`ade_codec::conway::tx`) keeps `proposal_procedures` as an
> opaque `Option<Vec<u8>>` (and `ade_types::conway::tx::ConwayTxBody.proposal_procedures`
> mirrors it), so a ratified `UpdateCommittee` enacted from a tx-submitted proposal
> is not yet reachable end-to-end. That is a **candidate future seam, NOT yet
> wired.** The carried non-goal credential surfaces are unchanged: the
> withdrawal/required-signer/address credential discriminant; the `Hash28`-keyed
> stake-distribution snapshot; the Byron credential surface; and `spo_votes`
> (`Hash28`-keyed by design — pools are key-hash only, a **permanent non-goal, NOT
> a follow-up**).
>
> **(Prior context — ENACTMENT-COMMITTEE-FIDELITY close, HEAD `a6b8de7`.) THE KEY
> ENACTMENT-COMMITTEE-FIDELITY DELTA:** the last bare-`Hash28` credential surface
> in the governance domain — `ade_ledger::governance::EnactmentEffects.committee_changes`
> — was **discriminated** (re-typed `Option<(Vec<Hash28>, Vec<(Hash28, u64)>)>` →
> `Option<(Vec<StakeCredential>, Vec<(StakeCredential, u64)>)>`). At that HEAD it
> was a PREVENTIVE type-fidelity guard-rail on a still-**dormant** field
> (`UpdateCommittee` enactment was still a no-op, `let _ = raw;`).
> **ENACTMENT-COMMITTEE-WRITEBACK then activated it** — `committee_changes` is now
> populated by `enact_proposals` and consumed by `apply_committee_enactment` — so
> the guard-rail it installed is now load-bearing. ENACTMENT-COMMITTEE-FIDELITY
> STRENGTHENED DC-LEDGER-10 (no new rule, `strengthened_in +=
> ENACTMENT-COMMITTEE-FIDELITY`) and EXTENDED
> `ci/ci_check_credential_discriminant_closed.sh` with a sixth check
> (`EnactmentEffects.committee_changes` `StakeCredential`-typed); it added no new
> crate, module, ingress surface, composer, or net new canonical type.
>
> **(Prior context — DREP-VOTE-FIDELITY close, HEAD `62c9020`.) THE KEY
> DREP-VOTE-FIDELITY DELTA:** the DRep-vote credential surface is now
> CLOSED/discriminated. `GovActionState.drep_votes` (in
> `ade_types::conway::governance`) is re-typed `Vec<(Hash28, Vote)>` →
> `Vec<(StakeCredential, Vote)>`, and the DRep tally in
> `ade_ledger::governance::{evaluate_ratification, check_ratification}` now
> resolves a DRep vote by **exact credential variant** — its `lookup_stake`
> closure maps `StakeCredential::KeyHash → DRep::KeyHash` and
> `StakeCredential::ScriptHash → DRep::ScriptHash` and reads that single
> DRep-stake key, with **NO key/script OR-fallback** — so a key-hash DRep
> voter can never tally a script-hash DRep's stake of equal 28 bytes
> (matching cardano-node's `Credential`-keyed DRep distribution). The
> committee + DRep vote writer was **renamed credential-generic**:
> `ade_ledger::fingerprint::write_committee_vote_list → write_credential_vote_list`
> (it now serves BOTH the committee-vote list AND the DRep-vote list — both
> emit the credential discriminant before the hash via `write_stake_credential`),
> and the snapshot-loader parser `parse_committee_vote_map →
> parse_credential_vote_map`; both are output-identical to the prior
> committee-only forms. `spo_votes` **stays `Hash28`-keyed** (pools are
> key-hash only — a permanent non-goal, not a follow-up) via the unchanged
> `write_vote_list`. **DREP-VOTE-FIDELITY STRENGTHENS DC-LEDGER-10 (no new
> rule, `strengthened_in += DREP-VOTE-FIDELITY`)** and **EXTENDS the existing
> CI gate** `ci/ci_check_credential_discriminant_closed.sh` (no new gate, no
> new file — CI count stays 29) to also defend the DRep surface:
> `GovActionState.drep_votes` must stay `Vec<(StakeCredential, Vote)>` (no
> reversion to `Hash28`), the DRep-vote serializer must route through
> `write_credential_vote_list`, and `governance.rs` must carry no DRep
> key/script OR-fallback (`DRep::KeyHash(...).or_else(...)`). **It added no
> new crate, no new module, no new ingress surface, no new public composer,
> and no net new canonical type** — the `drep_votes` re-type is a field-type
> change on the existing closed `GovActionState`, the `lookup_stake`
> exact-variant resolution is a resolution change, and
> `write_credential_vote_list` / `parse_credential_vote_map` are renamed
> functions. **This WIRES AND CLOSES the COMMITTEE-CRED-FIDELITY-declared
> DRep-vote discrimination candidate seam** (the recommended next discriminant
> cluster); the remaining declared non-goal credential surfaces are carried
> below — at the DREP-VOTE-FIDELITY HEAD this notably included
> **`EnactmentEffects.committee_changes`** (then a dormant bare-`Hash28` that had
> to migrate before committee enactment; **ENACTMENT-COMMITTEE-FIDELITY later
> discriminated it — now WIRED + CLOSED**) and the carried
> OQ5 non-goals (stake-distribution snapshot / withdrawal / required-signer /
> address / Byron). `spo_votes` is **NOT** a follow-up. **(Prior context —
> COMMITTEE-CRED-FIDELITY close, HEAD `2aeea16`.) THE KEY COMMITTEE-CRED-FIDELITY
> DELTA:** the committee
> credential surface is now CLOSED/discriminated. `ConwayGovState.committee`
> (the elected-member set) is re-keyed from bare `Hash28` to the discriminated
> `StakeCredential`, and `GovActionState.committee_votes` (in
> `ade_types::conway::governance`) is re-typed `Vec<(Hash28, Vote)>` →
> `Vec<(StakeCredential, Vote)>`. Committee ratification in
> `ade_ledger::governance` resolves hot→cold→member by **full-credential
> equality** (`*hot == hot_cred`, `**c == *cold`) — no `.hash()` collapse — so a
> key-hash hot key never cross-resolves to a script-hash member of equal bytes.
> The new closed fingerprint writer `ade_ledger::fingerprint::write_committee_vote_list`
> emits the credential discriminant before each vote (via `write_stake_credential`)
> — DREP-VOTE-FIDELITY later renamed it `write_credential_vote_list` and routed
> `drep_votes` through it too. At the COMMITTEE-CRED-FIDELITY HEAD the DRep / SPO
> vote lists (`drep_votes` / `spo_votes`) were still `Hash28`-keyed via the
> unchanged `write_vote_list`; DREP-VOTE-FIDELITY then discriminated `drep_votes`
> (`spo_votes` stays `Hash28`). **COMMITTEE-CRED-FIDELITY STRENGTHENS
> DC-LEDGER-10 (no new rule)** and **EXTENDS the existing CI gate**
> `ci/ci_check_credential_discriminant_closed.sh` (no new gate, no new file —
> CI count stays 29) to also defend the committee surface: `ConwayGovState.committee`
> must stay `StakeCredential`-keyed and `GovActionState.committee_votes` must
> stay `Vec<(StakeCredential, Vote)>` (no reversion to `Hash28`). **It added no
> new crate, no new module, no new ingress surface, no new public composer, and
> no net new canonical type** — the `committee` re-key and the `committee_votes`
> re-type are field-type changes on existing types; `write_committee_vote_list`
> is a serializer function, not a type. **This closes the committee half of the
> OQ5-declared `committee` / `committee_votes` non-goal candidate seam** (a
> security-review follow-up); the remaining declared non-goal credential surfaces
> are carried below. **(Prior context — OQ5-CREDENTIAL-FIDELITY close, HEAD
> `a3ee2da`.) THE KEY OQ5 DELTA:** the credential
> key/script discriminant the codec previously collapsed onto a bare `Hash28`
> is now **preserved end-to-end** — `ade_types::shelley::cert::StakeCredential`
> changed from the tuple-struct `StakeCredential(pub Hash28)` to the **closed
> 2-variant enum `{ KeyHash(Hash28), ScriptHash(Hash28) }`** (plus a read-only
> `hash()` accessor). Both era credential decoders
> (`ade_codec::shelley::cert::decode_stake_credential` /
> `ade_codec::conway::cert::decode_stake_credential`) are now **closed
> credential-decode chokepoints**: they read the credential type tag and map
> `0 → KeyHash`, `1 → ScriptHash`, rejecting any other tag with
> `CodecError::InvalidCborStructure { detail: "unknown stake credential type" }`
> — the prior tag-discarding `let (_cred_type|_tag, _)` form is gone, and the
> bare-hash `StakeCredential(<hash>)` tuple-construction is no longer
> constructible on the BLUE path. `state::ConwayGovState` re-keyed
> `vote_delegations` / `committee_hot_keys` / `drep_expiry` from bare `Hash28`
> to the discriminated `StakeCredential`, so a key-hash and a script-hash of the
> same 28 bytes are now **distinct authoritative-state keys** (matching
> cardano-node's `Credential`-keyed UMap/VState); `fingerprint::write_stake_credential`
> emits the discriminant (`0`/`1`) before the 28-byte hash, so two states
> differing only in a credential's key/script tag fingerprint differently (a
> deliberate cert-state + gov-state fingerprint migration, T-DET-01 /
> strengthens T-ENC-03). **OQ5 added one new CI script** —
> `ci/ci_check_credential_discriminant_closed.sh` (DC-LEDGER-10, `enforced`):
> it fails if the tuple-struct shape reappears, if either decoder reverts to a
> tag-discard form, or if any `StakeCredential(<hash>)` coercion reappears on
> the BLUE path. **OQ5 added no new crate, no new module, no new ingress
> surface, no new public composer, and no net new canonical type** —
> `StakeCredential` is the same already-counted `ade_types` type, re-shaped.
> **The one deliberate, narrow boundary seam OQ5 introduces is `cred.hash()`:**
> a read-only discriminant-discarding extraction used ONLY against declared
> non-goal surfaces — the `Hash28`-keyed stake-distribution snapshot (`epoch.rs`,
> `governance::evaluate_ratification` / `check_ratification`,
> `rules::apply_epoch_boundary_with_registrations`) and the bare-`Hash28`
> committee member / committee-vote sets — never to re-key authoritative state.
> **COMMITTEE-CRED-FIDELITY then CLOSED the committee half of OQ5's `committee` /
> `committee_votes` non-goal seam** — both are now `StakeCredential`-discriminated
> (the `committee_hot_keys` map was already discriminated at OQ5). The prior B5
> follow-up OQ-5 (credential key/script discriminant collapse) was WIRED AND
> CLOSED at OQ5; the remaining declared separable follow-up OQ-3 (GOVCERT
> committee-membership tx-validity gate) stays a **candidate future seam, NOT an
> open extension point now**. **DREP-VOTE-FIDELITY then WIRED AND CLOSED the
> DRep-vote discrimination non-goal** — `drep_votes` is now
> `StakeCredential`-discriminated and `lookup_stake` resolves to the exact DRep
> variant (no OR-fallback). The remaining declared non-goal candidate seams are
> recorded below: the withdrawal/required-signer/address credential discriminant;
> the `Hash28`-keyed stake-distribution snapshot;
> **`EnactmentEffects.committee_changes`** (at the DREP-VOTE-FIDELITY HEAD still
> a dormant bare-`Hash28` that had to migrate before committee enactment;
> **ENACTMENT-COMMITTEE-FIDELITY later discriminated it — now WIRED + CLOSED**);
> the Byron credential surface; and
> `spo_votes` (`Hash28`-keyed by design — pools are key-hash only, a **permanent
> non-goal, NOT a follow-up**). The carried B4 narrow gap stands: the
> `ade_ledger::delegation` owner-tagged apply *types* are still outside the
> `ci_check_consensus_closed_enums.sh` `TARGETS` array.
>
> **(Prior context — PHASE4-B5 close, HEAD `644eb03`.)** B5 shipped the closed
> gov-cert dispatch `ade_ledger::gov_cert::apply_conway_gov_cert` (a total,
> compiler-exhaustive `match` over `ConwayCert` — all 18 tags + removed 5/6, no
> `_ =>` wildcard arm; a closed surface, not an extension point), the closed
> fail-fast `GovCertEnv` (constructed only via `LedgerState::gov_cert_env()`;
> absent param → `MissingDRepActivityParam`), the checked DRep-expiry arithmetic
> (`current_epoch.checked_add(drep_activity)`, fail-closed via
> `DRepActivityOverflow`), and the migration of `ConwayGovState` from a frozen
> snapshot value to a deterministic fold over replayed governance-cert effects.
> The owner-tagged governance effects B4 routed OUT of mutation scope are now
> APPLIED — `accumulate_tx_certs` calls `apply_conway_gov_cert`. B5 added
> `ci/ci_check_gov_cert_accumulation_closed.sh` (DC-LEDGER-09, `enforced`,
> strengthens DC-LEDGER-08).

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

**PHASE4-B5 (Conway governance-certificate accumulation) just closed.** It
**applies** the owner-tagged governance effects B4 deliberately left
unapplied: the new closed `ade_ledger::gov_cert::apply_conway_gov_cert` is a
**total, compiler-exhaustive dispatch over `ConwayCert`** (all 18 tags + the
removed 5/6 marker, **no `_ =>` wildcard arm**) that folds vote-delegation /
committee auth-cold-resign / DRep register-update-unregister effects into
`ConwayGovState`; non-governance tags are a no-op (the empty arm is explicit,
not a wildcard). It is a **closed surface, not an extension point.** DRep
register/update certs consult the new closed `GovCertEnv` (carrying
`current_epoch` + `drep_activity`), constructed **only** via
`LedgerState::gov_cert_env()` — fail-fast: a non-Conway / missing-param state
yields `ValidationEnvironmentError::MissingDRepActivityParam`, never a
fabricated env. DRep expiry is `env.current_epoch.checked_add(env.drep_activity)`
— **deterministic fail-closed** via `ValidationEnvironmentError::DRepActivityOverflow`,
never an unchecked `+` that would wrap silently in release. `accumulate_tx_certs`
/ `process_block_certificates` now thread an `Option<ConwayGovState>` and call
`apply_conway_gov_cert`; the B4 "routed out of B4 mutation scope"
observe-and-drop is **removed**. `ConwayGovState` migrates from a frozen
snapshot value to a **deterministic fold over replayed governance-cert
effects** — an authoritative ingress that consumes the closed
`decode_conway_certs` grammar (the `drep_activity` extension to the
Conway-deposit fingerprint tag carries the T-DET-01 fingerprint migration; the
fold is byte-identical on replay). **B5 added one new crate-internal BLUE
module** (`ade_ledger::gov_cert`) and **one new CI script**
(`ci/ci_check_gov_cert_accumulation_closed.sh`, DC-LEDGER-09) — no new crate,
no new ingress surface, no new public composer; `gov_cert` consumes the
existing `ade_types::conway::cert::ConwayCert` and `ade_ledger::state::ConwayGovState`.
Registry rule `DC-LEDGER-09` is `enforced` and **strengthens DC-LEDGER-08**,
leaving B4's closed `OwnerTaggedEffect` / `GovernanceCertEffect` surface intact.
**Two declared separable follow-ups — OQ-3 (committee-membership tx-validity
gate) and OQ-5 (credential key/script discriminant collapse) — are candidate
future seams, NOT open extension points at this HEAD.**

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
>
> **B5 added no new external ingress surface either.** Its gov-cert
> accumulation is reached only through the existing block-body chokepoint
> `apply_block_with_verdicts` (at `track_utxo`), via `accumulate_tx_certs`
> → the new `gov_cert::apply_conway_gov_cert`. **The genuinely new seam B5
> introduces is INTERNAL and now CLOSED, not an extension point:** the
> `ConwayGovState` accumulation ingress is a deterministic fold over the
> owner-tagged effects B4 produced — it consumes the closed
> `decode_conway_certs` grammar and the closed `apply_conway_gov_cert`
> dispatch, both compiler-exhaustive. The B4 "confirmed extension point"
> row below is therefore now **wired and closed** (B5 consumed it).
>
> **OQ5 added no new external ingress surface either.** It is a type-fidelity
> refactor across the existing credential-decode + gov-state-key + fingerprint
> surfaces, not a new entry point. **The genuinely changed seams are two
> existing chokepoints made CLOSED on the discriminant:** the per-era
> `decode_stake_credential` (Shelley + Conway) now read the credential type tag
> and reject an unknown tag (`CodecError::InvalidCborStructure`,
> `"unknown stake credential type"`) — no tag-erasing, no bare-`Hash28`
> coercion (`ci_check_credential_discriminant_closed.sh`). These are
> sub-grammar readers inside the already-existing block-body and Conway-tx-body
> surfaces; they construct no `PreservedCbor` and add no new ingress. **The one
> genuinely new internal seam OQ5 introduces is a NARROW, read-only boundary
> adapter, not an extension point:** `StakeCredential::hash()` — a deliberate
> discriminant-discarding extraction used ONLY against declared non-goal
> surfaces (the `Hash28`-keyed stake-distribution snapshot in `epoch.rs` /
> `governance` / `apply_epoch_boundary_with_registrations`, and the bare-`Hash28`
> committee member / committee-vote sets). It is a one-way down-projection, never
> a re-key of authoritative state — see the candidate-seam table for the
> declared non-goal surfaces it touches.
>
> **COMMITTEE-CRED-FIDELITY added no new external ingress surface either.** It is
> the committee half of the same credential-discriminant fidelity refactor across
> the existing gov-state-key + gov-cert-decode + fingerprint surfaces, not a new
> entry point. **The changed seams are internal type-fidelity migrations:**
> `ConwayGovState.committee` re-keyed `Hash28` → `StakeCredential`,
> `GovActionState.committee_votes` re-typed `Vec<(Hash28, Vote)>` →
> `Vec<(StakeCredential, Vote)>`, the committee ratification hot→cold→member
> resolution in `ade_ledger::governance` now compares full discriminated
> credentials (no `.hash()` collapse), and the new
> `fingerprint::write_committee_vote_list` emits the discriminant. **Two of the
> declared non-goal surfaces the OQ5 `cred.hash()` adapter touched are now
> discriminated and no longer use the adapter** (the `committee` member set and
> the `committee_votes` set); `cred.hash()` is still the sanctioned one-way
> down-projection against the remaining `Hash28`-keyed stake-distribution
> snapshot. No new CI gate — `ci_check_credential_discriminant_closed.sh` was
> extended to also defend the committee key/vote shapes (DC-LEDGER-10
> strengthened, no new rule).
>
> **DREP-VOTE-FIDELITY added no new external ingress surface either.** It is the
> DRep-vote half of the same credential-discriminant fidelity refactor across the
> existing gov-state vote-list + DRep-tally + fingerprint surfaces, not a new
> entry point. **The changed seams are internal type-fidelity migrations:**
> `GovActionState.drep_votes` re-typed `Vec<(Hash28, Vote)>` →
> `Vec<(StakeCredential, Vote)>`, and the DRep tally in
> `ade_ledger::governance::{evaluate_ratification, check_ratification}` now
> resolves a DRep vote by exact credential variant via the `lookup_stake` closure
> (`StakeCredential::KeyHash → DRep::KeyHash`,
> `StakeCredential::ScriptHash → DRep::ScriptHash`, reading that single DRep-stake
> key) — **no key/script OR-fallback**, so a key-hash DRep voter never tallies a
> script-hash DRep's stake of equal bytes. The committee + DRep vote fingerprint
> writer was renamed credential-generic (`write_committee_vote_list →
> write_credential_vote_list`) and now emits the discriminant for both vote
> lists; the snapshot-loader parser was renamed `parse_committee_vote_map →
> parse_credential_vote_map` (output-identical). **The last gov-state vote
> surface OQ5/COMMITTEE still resolved through a bare `Hash28` is now
> discriminated**; `spo_votes` stays `Hash28`-keyed by design (pools are
> key-hash only — a permanent non-goal, never a follow-up). No new CI gate —
> `ci_check_credential_discriminant_closed.sh` was extended again to defend the
> DRep vote shape + the no-OR-fallback resolution (DC-LEDGER-10 strengthened,
> `strengthened_in += DREP-VOTE-FIDELITY`, no new rule).

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
and is removed from this table. **B5 WIRED AND CLOSED the prior revision's
single confirmed extension point** — the owner-tagged `ConwayGovState` effect
channel B4 produced is now consumed by `gov_cert::apply_conway_gov_cert` and
folded into `ConwayGovState`; it is recorded below as a **wired-and-closed**
row (no longer an *open* seam — it is a closed dispatch). **B5 introduced no new
candidate ingress seam.** **OQ5-CREDENTIAL-FIDELITY then WIRED AND CLOSED one of
B5's two declared separable follow-ups** — OQ-5 (credential key/script
discriminant collapse): `StakeCredential` is now the closed 2-variant enum and
both era credential decoders preserve the discriminant; `ConwayGovState`
(`vote_delegations` / `committee_hot_keys` / `drep_expiry`) is re-keyed on it;
the OQ-5 row below is recorded **wired-and-closed**. **OQ5 introduced no new
ingress seam.** The remaining B5 follow-up OQ-3 (GOVCERT committee-membership
tx-validity gate) stays a **candidate future seam**, and OQ5 added a new set of
**declared non-goal candidate seams** (recorded in the rows below): the
withdrawal/required-signer/address credential discriminant; the `Hash28`-keyed
stake-distribution snapshot; committee member/vote discrimination
(security-review follow-up — at OQ5 `committee_hot_keys` was discriminated, but the
`committee` and `committee_votes` sets were not); and the Byron credential
surface. These were framed exactly as the OQ5 cluster declared them: separable
non-goals, not wired, not open extension points then. **COMMITTEE-CRED-FIDELITY
(HEAD `2aeea16`) then WIRED AND CLOSED the committee member/vote discrimination
non-goal** — `ConwayGovState.committee` and `GovActionState.committee_votes` are
now `StakeCredential`-discriminated and committee ratification resolves by
full-credential equality (DC-LEDGER-10 strengthened, no new rule, no new gate).
**DREP-VOTE-FIDELITY (HEAD `62c9020`) then WIRED AND CLOSED the DRep-vote
discrimination non-goal** — `GovActionState.drep_votes` is now
`StakeCredential`-discriminated and the DRep tally resolves to the exact DRep
variant via `lookup_stake` with no key/script OR-fallback (DC-LEDGER-10
strengthened again, `strengthened_in += DREP-VOTE-FIDELITY`, no new rule, no new
gate). At the DREP-VOTE-FIDELITY HEAD the remaining non-goals were
(withdrawal/required-signer/address; stake-distribution snapshot;
**`EnactmentEffects.committee_changes`** — then still a dormant bare-`Hash28`
that had to migrate before committee enactment (**ENACTMENT-COMMITTEE-FIDELITY
later discriminated it — now WIRED + CLOSED**); Byron credentials; and
`spo_votes`, which is
`Hash28`-keyed by design — pools are key-hash only, a **permanent non-goal, NOT a
follow-up**) stay separable non-goals, not open extension points now.
**ENACTMENT-COMMITTEE-FIDELITY (HEAD `a6b8de7`) then WIRED AND CLOSED the
`EnactmentEffects.committee_changes` type-fidelity seam** — the field was
discriminated `Hash28` → `StakeCredential` as a preventive guard-rail, leaving
the dormant `UpdateCommittee` enactment LOGIC as the one remaining open
governance-enactment seam. **ENACTMENT-COMMITTEE-WRITEBACK (HEAD `3180e27`) then
WIRED AND CLOSED that dormant LOGIC** — `GovAction::UpdateCommittee` re-shaped to
the closed structured `{ removed: BTreeSet<StakeCredential>, added:
BTreeMap<StakeCredential, u64>, threshold }` (GovAction stays a closed 7-variant
enum), and committee write-back is now the closed pure transition
`apply_committee_enactment` called at the `rules.rs` epoch boundary
(DC-EPOCH-01 / DC-LEDGER-10 strengthened, no new rule, no new gate). **The one
NEW candidate seam ENACTMENT-COMMITTEE-WRITEBACK introduces** is the declared
non-goal `proposal_procedures` tx-body decode (the wire codec keeps
`proposal_procedures` opaque `Option<Vec<u8>>`) — recorded in the row below,
NOT yet wired.

| Cluster | Surface | Expected reduction target | Expected chokepoint | Confidence |
|---------|---------|---------------------------|---------------------|------------|
| **PHASE4-B5** *(WIRED + CLOSED)* | Owner-tagged Conway governance-cert effects → `ConwayGovState` — the B4 channel `ConwayCertOutcome.owner_tagged` | An applied `ConwayGovState'` via a deterministic fold | **DONE:** `ade_ledger::gov_cert::apply_conway_gov_cert` (closed total dispatch over `ConwayCert`, no `_ =>` arm) called from `accumulate_tx_certs`; folds into `ConwayGovState`. No new composer, no new ingress. Gated by `ci_check_gov_cert_accumulation_closed.sh` (DC-LEDGER-09) | **wired & closed in B5** (was the B4 confirmed extension point; now a closed dispatch, no longer an open seam) |
| OQ-3 *(separable follow-up — NOT an open seam now)* | **GOVCERT committee-membership tx-validity gate** — validating that a committee hot-key auth / cold-key resign cert references an *actually-elected* committee member before it accumulates (B5 accumulates committee certs **unconditionally**; the precondition gate is deliberately deferred) | A `TxValidityVerdict::Invalid` on a committee cert with no matching elected member | A new BLUE tx-validity precondition check (likely in `tx_validity` / `cert_classify`) consulting `ConwayGovState` committee membership — a SEPARABLE GOVCERT validity gate, NOT a change to `apply_conway_gov_cert` | candidate (declared separable in the B5 cluster doc §"Out of scope" / B5-S2/S4 — confirm at the GOVCERT-validity cluster entry) |
| **OQ-5** *(WIRED + CLOSED in OQ5-CREDENTIAL-FIDELITY)* | Credential key/script discriminant — gov-state was keyed on a bare `Hash28`, collapsing key-hash vs. script-hash credentials of the same 28 bytes | A discriminant-preserving credential representation threaded codec → gov-state | **DONE:** `ade_types::shelley::cert::StakeCredential` is now the closed 2-variant enum `{ KeyHash, ScriptHash }`; both `decode_stake_credential` (Shelley + Conway) preserve the discriminant and reject an unknown tag; `ConwayGovState.{vote_delegations, committee_hot_keys, drep_expiry}` re-keyed on it; `fingerprint::write_stake_credential` emits the discriminant. Gated by `ci_check_credential_discriminant_closed.sh` (DC-LEDGER-10) | **wired & closed in OQ5** (was the B5 separable follow-up; now a closed credential surface) |
| OQ5+ *(declared non-goal — NOT an open seam now)* | **Withdrawal / required-signer / address credential discriminant** — `RewardAccount` / `SignerSource::WithdrawalKey,CertificateKey,GovernanceVoter` / address payment-and-stake credentials still arrive without (or project away) the key/script discriminant on the BLUE path | A discriminant-faithful credential threaded through these surfaces | extend the closed `StakeCredential` discriminant through `decode_withdrawals` keys, `required_signers`, and `decode_address` — a SEPARABLE per-surface fidelity follow-up, not a change to `decode_stake_credential` | candidate (declared non-goal in OQ5 — confirm before any of these surfaces must distinguish key vs. script credentials) |
| OQ5+ *(declared non-goal — NOT an open seam now)* | **`Hash28`-keyed stake-distribution snapshot** — `epoch.rs` keys the stake-distribution snapshot on bare `Hash28`; `governance::evaluate_ratification` / `apply_epoch_boundary_with_registrations` reach it via the read-only `cred.hash()` adapter | A discriminant-faithful stake-distribution snapshot key | re-key the snapshot on `StakeCredential` (touches the snapshot loader + every `cred.hash()` call site) — a SEPARABLE snapshot-fidelity follow-up; the `cred.hash()` adapter is a deliberate one-way down-projection until then | candidate (declared non-goal in OQ5 — `cred.hash()` is a sanctioned boundary adapter, never a re-key) |
| **Committee member / committee-vote discrimination** *(WIRED + CLOSED in COMMITTEE-CRED-FIDELITY)* | `committee` member set + `committee_votes` set were bare `Hash28` at OQ5 (only `committee_hot_keys` was discriminated) | discriminant-faithful committee member / vote sets | **DONE:** `ConwayGovState.committee` re-keyed `Hash28` → `StakeCredential`; `GovActionState.committee_votes` re-typed `Vec<(Hash28, Vote)>` → `Vec<(StakeCredential, Vote)>`; committee ratification resolves hot→cold→member by full-credential equality (no `.hash()` collapse); `fingerprint::write_committee_vote_list` emits the discriminant. Gated by the EXTENDED `ci_check_credential_discriminant_closed.sh` (DC-LEDGER-10 strengthened, no new gate) | **wired & closed in COMMITTEE-CRED-FIDELITY** (was the OQ5 security-review follow-up; now a closed committee credential surface) |
| **DRep-vote discrimination** *(WIRED + CLOSED in DREP-VOTE-FIDELITY)* | `ade_ledger::governance` `drep_votes` formerly did a key/script OR-fallback (`drep_stake.get(KeyHash).or_else(ScriptHash)`) over a bare `Hash28` rather than reading a discriminated DRep credential | discriminant-faithful DRep-vote lookup (no OR-fallback) | **DONE:** `GovActionState.drep_votes` re-typed `Vec<(Hash28, Vote)>` → `Vec<(StakeCredential, Vote)>`; the DRep tally's `lookup_stake` closure maps `StakeCredential::KeyHash → DRep::KeyHash` / `StakeCredential::ScriptHash → DRep::ScriptHash` and reads that single DRep-stake key (no OR-fallback); `write_committee_vote_list` renamed `write_credential_vote_list` (now serves committee + DRep), `parse_committee_vote_map` renamed `parse_credential_vote_map`. Gated by the EXTENDED `ci_check_credential_discriminant_closed.sh` (DC-LEDGER-10 strengthened, no new gate) | **wired & closed in DREP-VOTE-FIDELITY** (was the COMMITTEE-CRED-FIDELITY recommended-next discriminant follow-up; now a closed DRep-vote credential surface) |
| **`EnactmentEffects.committee_changes`** *(WIRED + CLOSED in ENACTMENT-COMMITTEE-FIDELITY)* | The committee-enactment effect formerly carried a dormant bare-`Hash28` committee-change set (removed + added-with-expiry), a migrate-before-enactment risk | discriminant-faithful committee-change set | **DONE:** `EnactmentEffects.committee_changes` re-typed `Option<(Vec<Hash28>, Vec<(Hash28, u64)>)>` → `Option<(Vec<StakeCredential>, Vec<(StakeCredential, u64)>)>` — a preventive guard-rail (the field is dormant; `UpdateCommittee` enactment is still a no-op) so a future write-back cannot re-collapse the discriminated `ConwayGovState.committee` map. Gated by the EXTENDED `ci_check_credential_discriminant_closed.sh` (DC-LEDGER-10 strengthened, no new gate) | **wired & closed in ENACTMENT-COMMITTEE-FIDELITY** (was the DREP-VOTE-FIDELITY migrate-before-enactment candidate; the effect TYPE is now discriminated) |
| **`UpdateCommittee` / `NoConfidence` enactment LOGIC** *(WIRED + CLOSED in ENACTMENT-COMMITTEE-WRITEBACK)* | At the prior HEAD `enact_proposals` did not apply a ratified committee change to `ConwayGovState.committee`; the arm was `let _ = raw;` (effects stayed `None`) | A `ConwayGovState'` with committee membership + quorum updated per the enacted `UpdateCommittee` / dissolved per `NoConfidence` | **DONE:** `GovAction::UpdateCommittee` re-shaped to the closed structured `{ prev_action, removed: BTreeSet<StakeCredential>, added: BTreeMap<StakeCredential, u64>, threshold }` (GovAction stays a closed 7-variant enum); `enact_proposals` now populates `EnactmentEffects.committee_changes` + the new `committee_threshold` field; the closed pure transition `ade_ledger::governance::apply_committee_enactment` writes the change into `ConwayGovState.committee` + `committee_quorum`, called at the `rules.rs` epoch-boundary apply site (`rules.rs:1224`); `NoConfidence` dissolves the committee. No new composer, no new ingress, no new crate, no net new type. Gated by the EXTENDED `ci_check_credential_discriminant_closed.sh` (checks 6 + 7 — DC-EPOCH-01 / DC-LEDGER-10 strengthened) | **wired & closed in ENACTMENT-COMMITTEE-WRITEBACK** (was the ENACTMENT-COMMITTEE-FIDELITY dormant-LOGIC candidate; the enactment write-back is now a closed pure transition) |
| ENACTMENT+ *(declared non-goal — separable future seam, NOT an open seam now)* | **`proposal_procedures` tx-body decode → `GovAction`** — the wire codec `ade_codec::conway::tx` keeps `proposal_procedures` as an opaque `Option<Vec<u8>>` (mirrored by `ade_types::conway::tx::ConwayTxBody.proposal_procedures`); a `GovAction` enacted from a tx-submitted proposal is not yet reachable end-to-end (gov-state is fed from the snapshot loader, not from decoded tx proposals) | A typed `GovAction` (incl. the now-structured `UpdateCommittee`) decoded from a real Conway tx body's `proposal_procedures` | a closed `proposal_procedures` sub-grammar reader inside the existing Conway-tx-body surface (parallel to the existing `decode_conway_certs` / `decode_withdrawals` sub-grammars) that lifts the opaque slice into `Vec<GovAction>` — a SEPARABLE codec-fidelity follow-up, not a change to `apply_committee_enactment` | candidate (declared non-goal in ENACTMENT-COMMITTEE-WRITEBACK — confirm at the proposal-decode cluster entry) |
| DREP-FIDELITY+ *(permanent non-goal — NOT a follow-up)* | **`spo_votes`** — the SPO-vote tally is keyed on a bare `Hash28` | n/a — SPO votes are pool key-hashes only | no change — Cardano pools are identified by a key-hash only; there is no key/script discriminant to preserve, so `spo_votes` keeps `Hash28` (and `write_vote_list`) by design | **permanent non-goal** (NOT a follow-up; do not migrate) |
| OQ5+ *(declared non-goal — NOT an open seam now)* | **Byron credential surface** — Byron-era credential structures are outside the OQ5 Shelley..Conway `StakeCredential` discriminant migration | discriminant-faithful Byron credentials (if ever required) | a SEPARABLE Byron-era follow-up — only if a Byron credential surface needs the key/script distinction | candidate (declared non-goal in OQ5) |
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
mempool/tx cluster entry. **OQ5-CREDENTIAL-FIDELITY WIRED AND CLOSED the B5
follow-up OQ-5** (credential key/script discriminant — answered "at the codec
layer AND the gov-state key shape": the closed `StakeCredential` enum is built
in the per-era decoders and threaded into the `ConwayGovState` keys).
**The remaining B5 follow-up OQ-3 (committee-membership GOVCERT validity gate) is
still a separable future seam, not an open extension point** — it needs
confirmation before a GOVCERT-validity or committee-authority cluster opens,
framed as: "Is the committee-membership precondition a `tx_validity` gate or a
`cert_classify` disposition?" **COMMITTEE-CRED-FIDELITY then CLOSED the committee
member/vote half of OQ5's `committee` / `committee_votes` non-goal seam**
(`ConwayGovState.committee` + `GovActionState.committee_votes` discriminated),
and **DREP-VOTE-FIDELITY then CLOSED the DRep-vote discrimination non-goal** —
`GovActionState.drep_votes` is now `StakeCredential`-discriminated and the DRep
tally resolves to the exact DRep variant via `lookup_stake` (no key/script
OR-fallback). **ENACTMENT-COMMITTEE-FIDELITY then CLOSED the
`EnactmentEffects.committee_changes` type-fidelity seam, and
ENACTMENT-COMMITTEE-WRITEBACK then WIRED AND CLOSED the dormant `UpdateCommittee`
enactment LOGIC** — `GovAction::UpdateCommittee` is now the closed structured
variant and `apply_committee_enactment` is the closed pure write-back called at
the `rules.rs` epoch boundary. **The remaining declared non-goal candidate seams**
(withdrawal/required-signer/address credential discriminant; `Hash28`-keyed
stake-distribution snapshot; the Byron credential surface; and the NEW
`proposal_procedures` tx-body decode into `GovAction`) each need confirmation
before the relevant cluster opens. For the credential surfaces the framing is:
"Does this surface need the discriminant, or is the bare-`Hash28` projection via
`cred.hash()` still the right declared non-goal?"; for `proposal_procedures` the
framing is: "Is the proposal-decode a closed sub-grammar reader inside the Conway
tx body (parallel to `decode_conway_certs`), and is `Vec<GovAction>` the right
reduction target?" **`spo_votes` is a PERMANENT non-goal, not a follow-up** —
Cardano pools are identified by a key-hash only, so there is no key/script
discriminant to preserve; it keeps `Hash28` + `write_vote_list` by design.

---

## 2. Data-Only vs. Authoritative Layers

Ade has twelve authoritative domains. For each, a single BLUE chokepoint
holds enforcement authority; tooling layers (when they exist) live in
GREEN (`ade_testkit`) or RED (`ade_runtime`, `ade_network::mux::transport`,
`ade_network::session`, `ade_core_interop`). **B3 added one domain — the
Conway value-conservation accounting — and added a closed cert/withdrawal
data-only layer (`ade_codec::conway::{cert, withdrawals}`) under the
existing codec authority. B4 added one more domain — the Conway
certificate-state accumulation — built on the SAME data-only cert grammar
(now owner-complete) plus a new owner-tagged apply layer in
`ade_ledger::delegation` and an era-dispatch layer in `ade_ledger::rules`.
B5 added one more — the Conway governance-cert accumulation authority — a new
closed total dispatch (`ade_ledger::gov_cert::apply_conway_gov_cert`) that
APPLIES the owner-tagged effects B4 routed out of scope, folding them into
`ConwayGovState`; it is the consuming half of B4's owner-tagging boundary, now
wired and closed. OQ5-CREDENTIAL-FIDELITY added one more — the credential
discriminant-fidelity authority — closing the credential key/script discriminant
across the per-era credential decoders, the `ConwayGovState` keys, and the
canonical fingerprint (a type-fidelity refactor of the existing credential
surface, not a new chokepoint). COMMITTEE-CRED-FIDELITY then EXTENDED that same
credential discriminant-fidelity authority to the committee surface — the
`ConwayGovState.committee` member set, the `GovActionState.committee_votes` set,
the committee ratification hot→cold→member resolution, and the committee-vote
fingerprint — strengthening DC-LEDGER-10 (no new domain, no new rule, no new
chokepoint). DREP-VOTE-FIDELITY then EXTENDED it once more to the DRep-vote
surface — the `GovActionState.drep_votes` list, the DRep tally's exact-variant
`lookup_stake` resolution (no key/script OR-fallback), and the credential-generic
rename of the committee-vote fingerprint writer
(`write_committee_vote_list → write_credential_vote_list`, now serving committee +
DRep) and the snapshot-loader parser (`parse_committee_vote_map →
parse_credential_vote_map`) — again strengthening DC-LEDGER-10
(`strengthened_in += DREP-VOTE-FIDELITY`; no new domain, no new rule, no new
chokepoint, no new gate). `spo_votes` stays `Hash28`-keyed by design (pools are
key-hash only — a permanent non-goal). ENACTMENT-COMMITTEE-FIDELITY then EXTENDED
the same credential discriminant-fidelity authority to
`EnactmentEffects.committee_changes` (the committee-enactment effect), and
ENACTMENT-COMMITTEE-WRITEBACK then WIRED the dormant committee-enactment LOGIC
itself within the existing governance ratification/enactment domain — the
structured `GovAction::UpdateCommittee` (closed 7-variant `GovAction`,
re-shaped in place) + the new closed pure transition `apply_committee_enactment`
+ the new `EnactmentEffects.committee_threshold` field + the epoch-boundary
apply site in `rules.rs`. No new domain, no new chokepoint, no new rule, no new
gate (strengthens DC-EPOCH-01 + DC-LEDGER-10).**

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

### Credential discriminant fidelity — the closed credential surface (NEW in OQ5-CREDENTIAL-FIDELITY)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Closed credential domain type** | `ade_types::shelley::cert::StakeCredential` | BLUE | The closed 2-variant enum `{ KeyHash(Hash28), ScriptHash(Hash28) }` (was the tuple-struct `StakeCredential(pub Hash28)`). A key-hash and a script-hash of the same 28 bytes are now distinct values. Carries a read-only `hash()` accessor (the narrow boundary seam). No `#[non_exhaustive]`, no open tail. |
| **Data-only — closed credential-decode chokepoints** | `ade_codec::shelley::cert::decode_stake_credential` / `ade_codec::conway::cert::decode_stake_credential` | BLUE | Each reads the credential type tag and maps `0 → StakeCredential::KeyHash`, `1 → StakeCredential::ScriptHash`, rejecting any other tag with `CodecError::InvalidCborStructure { detail: "unknown stake credential type" }`. **No tag-erasing** (`let (_cred_type\|_tag, _)` is gone), **no bare-`Hash28` coercion** (`StakeCredential(<hash>)` is no longer constructible on the BLUE path). A closed credential grammar — an unknown discriminant is a deterministic reject. |
| **Authoritative enforcement — gov-state key surface** | `ade_ledger::state::ConwayGovState.{vote_delegations, committee_hot_keys, drep_expiry, committee}` + `ade_types::conway::governance::GovActionState.{committee_votes, drep_votes}` | BLUE | Re-keyed/re-typed from bare `Hash28` to the discriminated `StakeCredential` (matching cardano-node's `Credential`-keyed UMap/VState/DRep distribution). OQ5 did `vote_delegations` / `committee_hot_keys` / `drep_expiry`; COMMITTEE-CRED-FIDELITY added `committee` (the elected-member set, `BTreeMap<StakeCredential, u64>`) and `committee_votes` (`Vec<(StakeCredential, Vote)>`); **DREP-VOTE-FIDELITY added `drep_votes` (`Vec<(StakeCredential, Vote)>` on `GovActionState`)** — `spo_votes` stays `Vec<(Hash28, Vote)>` (pools key-hash only). `gov_cert::apply_conway_gov_cert` consumes the discriminated credential directly, not a `.0` projection. **Committee ratification resolves hot→cold→member by full-credential equality** (no `.hash()` collapse), and **the DRep tally resolves a vote by exact credential variant** — `lookup_stake` maps `StakeCredential::KeyHash → DRep::KeyHash` / `StakeCredential::ScriptHash → DRep::ScriptHash` and reads that single DRep-stake key, with **no key/script OR-fallback** — so neither a committee hot key nor a DRep voter cross-resolves a key-hash and a script-hash sharing 28 bytes. |
| **Determinism — discriminant-faithful fingerprint** | `ade_ledger::fingerprint::{write_stake_credential, write_credential_vote_list}` | BLUE | `write_stake_credential` emits the discriminant (`0`/`1`) before the 28-byte hash; the gov-map fingerprint writers (`drep_expiry`, `vote_delegations`, `committee_hot_keys`, `committee`) call it instead of `write_hash28`. **`write_credential_vote_list`** (DREP-VOTE-FIDELITY's rename of COMMITTEE-CRED-FIDELITY's `write_committee_vote_list`) is the closed discriminant-emitting vote-list writer — it now serves **both** `committee_votes` AND `drep_votes` (sorts by the discriminated credential's `Ord`, then emits `write_stake_credential` + vote tag). The SPO vote list keeps the unchanged `Hash28`-keyed `write_vote_list`. Two states differing only in a committee or DRep credential's key/script tag fingerprint differently (T-DET-01 / strengthens T-ENC-03). |
| **Narrow read-only boundary adapter** | `StakeCredential::hash()` at `epoch.rs` / `governance::{evaluate_ratification, check_ratification}` / `rules::apply_epoch_boundary_with_registrations` | BLUE | A deliberate discriminant-discarding extraction used ONLY against the remaining declared non-goal surface: the `Hash28`-keyed stake-distribution snapshot. **COMMITTEE-CRED-FIDELITY removed the committee member / committee-vote sets from this adapter's scope** — they are now `StakeCredential`-discriminated and committee ratification compares full credentials directly. The `DRep::{KeyHash,ScriptHash}` discriminant is rebuilt into the matching `StakeCredential` variant to read `drep_expiry`; reward-account / pool-owner hashes that arrive without a discriminant are still projected through `KeyHash` to match how the snapshot is keyed. **A one-way down-projection, never a re-key of authoritative state.** |
| **Adversarial / fidelity corpus** | `ade_ledger` credential-fidelity corpus + codec discriminant tests | GREEN/BLUE-test | `keyhash_scripthash_same_bytes_are_distinct_certstate`, `keyhash_scripthash_same_bytes_are_distinct_govstate`, `discriminant_changes_fingerprint(_corpus)`, `credential_accumulation_replays_byte_identical`; codec `shelley_credential_preserves_discriminant` / `conway_credential_preserves_discriminant` / `unknown_credential_tag_rejects`. |

**Rule.** The credential key/script discriminant is **preserved end-to-end** on
the BLUE authoritative path: it is read once at the closed per-era
`decode_stake_credential` chokepoints, threaded through `ConwayGovState`'s
discriminated keys, and emitted into the canonical fingerprint (DC-LEDGER-10).
**The discriminant is never erased on the BLUE path** — the tuple-struct shape,
the tag-discard decode form, and any `StakeCredential(<hash>)` bare-hash
coercion are grep-forbidden by `ci_check_credential_discriminant_closed.sh`.
**The ONE sanctioned discriminant-discarding move is `cred.hash()`**, a
read-only adapter used ONLY against the remaining declared non-goal surface (the
`Hash28`-keyed stake-distribution snapshot). **COMMITTEE-CRED-FIDELITY removed
the committee member / committee-vote sets from that adapter's scope** — they are
now `StakeCredential`-discriminated (DC-LEDGER-10 strengthened; the EXTENDED
`ci_check_credential_discriminant_closed.sh` defends the `committee` key shape and
the `committee_votes` element type). **DREP-VOTE-FIDELITY then closed the DRep-vote surface** — `drep_votes` is
`StakeCredential`-discriminated and the DRep tally resolves to the exact DRep
variant via `lookup_stake` (no key/script OR-fallback); the committee-vote
fingerprint writer was renamed credential-generic
(`write_committee_vote_list → write_credential_vote_list`) and now serves DRep
votes too, and the snapshot-loader parser was renamed
(`parse_committee_vote_map → parse_credential_vote_map`). New work that needs the
discriminant on a surface that does not yet carry it
(withdrawals/required-signer/address credentials; the stake-distribution
snapshot; the Byron credential surface)
**extends the closed `StakeCredential` discriminant into that surface** — a
separable per-surface fidelity follow-up, not a change to
`decode_stake_credential`. These are declared non-goal candidate seams (§1, §3),
not open extension points at this HEAD. **`spo_votes` is a permanent non-goal**
(pools key-hash only — no discriminant to preserve).

### Conway governance-cert accumulation — the owner-tagged apply authority (NEW in B5)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Data-only — owner-complete cert grammar (carried)** | `ade_codec::conway::cert::decode_conway_certs` | BLUE | The SAME B4 owner-complete closed grammar over tags `0..18`; B5 added no codec change. The owner payloads (vote-delegation `drep` targets, committee cold/hot credentials, DRep credentials) it retains are exactly what `apply_conway_gov_cert` consumes. |
| **Fail-fast gov-cert environment** | `ade_ledger::state::GovCertEnv` + `LedgerState::gov_cert_env()` | BLUE | A closed struct `{ current_epoch, drep_activity }`, the explicit environment DRep register/update certs need to compute expiry. Constructed **only** via `gov_cert_env()` — `Some` iff the state carries `drep_activity`, else `ValidationEnvironmentError::MissingDRepActivityParam` (no fabricated env). The new `drep_activity` field lives on `ConwayOnlyDepositParams`. |
| **Closed total gov-cert dispatch** | `ade_ledger::gov_cert::apply_conway_gov_cert(&mut ConwayGovState, &ConwayCert, Option<&GovCertEnv>) -> Result<(), LedgerError>` | BLUE | A **total, compiler-exhaustive `match` over `ConwayCert`** (all 18 tags + the removed 5/6 marker) with **NO `_ =>` wildcard arm**. Vote-delegation (tags 9/10/11/12), committee auth-cold (14) / cold-resign (15), and DRep register (16) / update (18) / unregister (17) fold into `ConwayGovState`; every non-governance tag (account/stake/pool/`RemovedInConway`) is an **explicit no-op arm**, not a wildcard. DRep register/update consult `GovCertEnv` and set expiry = `current_epoch.checked_add(drep_activity)` — deterministic fail-closed via `ValidationEnvironmentError::DRepActivityOverflow`. A **closed surface, not an extension point.** |
| **Gov-state ingress (era-dispatch + fold)** | `ade_ledger::rules::accumulate_tx_certs` / `process_block_certificates` (now threading `Option<ConwayGovState>`) | BLUE | For each Conway cert, calls `apply_conway_gov_cert` and folds the result into the threaded `ConwayGovState`. The B4 "routed out of B4 mutation scope" observe-and-drop is **removed** — the owner-tagged effects are now applied, not dropped. `ConwayGovState` migrates from a frozen snapshot value to a **deterministic fold over replayed governance-cert effects** (an authoritative ingress over the closed `decode_conway_certs` grammar). |
| **Determinism fold (T-DET-01 migration)** | `ade_ledger::fingerprint` | BLUE | The Conway-deposit fingerprint tag is extended with `drep_activity`; the `ConwayGovState` fold replays byte-identically (`gov_state_accumulation_replays_byte_identical`). |
| **Positive corpus harness** | `ade_testkit` B5-S4 gov-state corpus (`positive_synthetic_gov_state_accumulates`) | GREEN | Synthetic positive gov-state accumulation; replay byte-identical. |
| **Adversarial harness** | `ade_testkit` B5 adversarial corpus | GREEN | `adversarial_drep_register_update_missing_env_rejected`, `adversarial_drep_expiry_overflow_rejected`, `adversarial_decode_layer_rejects_guard_gov_path`, `adversarial_double_resign_is_deterministic` — each maps to its expected fail-closed / deterministic outcome. |

**Rule.** This domain has **one closed total dispatch**
(`apply_conway_gov_cert`), **one fail-fast environment**
(`GovCertEnv` via `gov_cert_env()`), and **one fold ingress** (inside
`accumulate_tx_certs`). **THE KEY SEAM B5 closes is the consuming half of B4's
owner-tagging boundary:** the governance effects B4 owner-tagged and routed out
of scope are now APPLIED here — `apply_conway_gov_cert` is the single chokepoint
that folds vote-delegation / committee / DRep effects into `ConwayGovState`.
**This is a closed surface, not an extension point:** a new Conway governance
cert tag adds an explicit `ConwayCert` variant + a `decode_conway_certs` arm + a
`conway_cert_action` arm (B4) + an `apply_conway_gov_cert` arm — and because the
dispatch is a compiler-exhaustive `match` with no `_ =>` arm, a new variant
**breaks the build** rather than being silently dropped (DC-LEDGER-09, which
**strengthens DC-LEDGER-08** — B4's owner-tagging surface is left intact;
B5 adds the native dispatch additively). The closure is mechanically defended
by the new `ci_check_gov_cert_accumulation_closed.sh`, which fails CI if (1)
`apply_conway_gov_cert` grows a `_ =>` wildcard, (2) the B4 observe-and-drop
comment reappears or `accumulate_tx_certs` stops calling
`apply_conway_gov_cert`, (3) DRep expiry uses an unchecked `+` instead of
`checked_add`, or (4) the `MissingDRepActivityParam` env fail-fast /
`gov_cert_env()` constructor are absent — plus the compiler-exhaustive `match`
and the 17 named B5 tests. **Two declared separable follow-ups attach ABOVE
this domain, NOT inside it:** OQ-3 (a committee-membership precondition gate —
B5 accumulates committee certs unconditionally; the gate is a separate
tx-validity check, not a change to `apply_conway_gov_cert`) and OQ-5 (preserving
the credential key/script discriminant that the codec currently collapses).
Both are candidate future seams (§1, §3), not open extension points at this HEAD.

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
| **Authoritative enforcement** | `ade_ledger` | BLUE | `rules::apply_block_with_verdicts` is the single chokepoint that produces `BlockVerdict` + new `LedgerState`; `tx_validity` is the single-tx chokepoint (B2); `check_conway_coin_conservation` is the value-conservation authority (B3); `accumulate_tx_certs` + `delegation::apply_conway_cert` is the era-dispatched cert-state accumulation authority (B4), and `gov_cert::apply_conway_gov_cert` (called from `accumulate_tx_certs`) is the governance-cert accumulation authority that folds owner-tagged effects into `ConwayGovState` (B5) — both reached inside `apply_block_with_verdicts` at `track_utxo`. |
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
| **Authoritative enforcement** | `ade_ledger::governance::{evaluate_ratification, enact_proposals, expire_proposals}` | BLUE | The chokepoints that compute Conway ratification outcomes. **ENACTMENT-COMMITTEE-WRITEBACK:** `enact_proposals` now populates `EnactmentEffects.committee_changes` + `committee_threshold` for a ratified `UpdateCommittee` / `NoConfidence`. |
| **Committee write-back** *(NEW in ENACTMENT-COMMITTEE-WRITEBACK)* | `ade_ledger::governance::apply_committee_enactment` | BLUE | The closed pure transition `fn(&committee, quorum, &EnactmentEffects) -> (new_committee, new_quorum)`. Removes the `removed` cold credentials, inserts the `added` ones with term-expiry epochs, applies `committee_threshold` to the quorum; `NoConfidence` dissolves the committee. Called at the `ade_ledger::rules` epoch-boundary apply site (`rules.rs:1224`); replays byte-identically. Never re-collapses the discriminated `ConwayGovState.committee` (`BTreeMap<StakeCredential, u64>`). |
| **Snapshot decode (data-only)** *(NEW in ENACTMENT-COMMITTEE-WRITEBACK; tightened in 168ac02)* | `ade_testkit` snapshot loader: `parse_cold_credential` / `parse_cold_credential_set` / `parse_cold_credential_epoch_map` / `parse_unit_interval` (+ `parse_with_origin_slot` and dual-layout `parse_registered_credentials`) | GREEN | Fail-closed decode of the `update_committee` gov-action structure (discriminated cold credentials + epoch map + unit-interval threshold) from snapshot bytes; malformed input rejects. **168ac02 hardenings (GREEN decoder closure aligned to BLUE schema migrations):** `parse_cold_credential_set` / `parse_cold_credential_epoch_map` are now fail-closed on truncation (a `terminated` flag is flipped by the indefinite `0xff` break or the definite `count >= declared_len` exit; a declared-but-under-length set/map rejects deterministically — replacing the prior silent advance); `parse_registered_credentials` now decodes BOTH UMElem layouts the BLUE `ConwayGovState` migration spans (pre-Conway `array(4) [StrictMaybe RDPair, Set Ptr, StrictMaybe Pool, StrictMaybe DRep]` and Conway compact `array(4) [uint reward, uint deposit, …]`, discriminated by the major type of UMElem[0]), so the same loader covers both the discriminated `StakeCredential`-keyed Conway shape and the historical Shelley..Babbage shape without misclaiming a credential; and `parse_with_origin_slot` lifts the live tip slot from the `HeaderState`'s `WithOrigin (AnnTip …)` prefix (a new `slot: u64` field on the existing GREEN `SnapshotHeader`). **Still GREEN**, still a fixture-decode helper, still asserts nothing about ratification or about the BLUE `apply_committee_enactment` transition. **Not a new SEAM** — no new BLUE chokepoint, no new canonical type, no new ingress; surfaced here as the canonical example of GREEN decoder closures aligned to BLUE schema migrations. |

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
the confirmed-extension-point row in §1). **B5 note:** PHASE4-B5 CLOSED that
consumption — `ade_ledger::gov_cert::apply_conway_gov_cert` is the closed total
dispatch that folds vote-delegation / committee / DRep effects into
`ConwayGovState`, called from `accumulate_tx_certs`. The gov-cert apply is the
**accumulation** half (which credentials/committee/DRep entries exist); the
ratification/enactment chokepoints here remain the **lifecycle** half
(`evaluate_ratification` / `enact_proposals` / `expire_proposals`). DRep
register/update set expiry via `GovCertEnv` (`checked_add`, fail-closed on
overflow). **OQ5-CREDENTIAL-FIDELITY then CLOSED the credential key/script
discriminant (OQ-5):** `ConwayGovState`'s `vote_delegations` / `committee_hot_keys`
/ `drep_expiry` are now keyed on the discriminated `StakeCredential`, and the
ratification reads use the read-only `cred.hash()` adapter only against the
`Hash28`-keyed stake-distribution snapshot (a declared non-goal surface — see
§2 "Credential discriminant fidelity"). The committee-membership precondition
gate (OQ-3) remains a separable follow-up (DC-LEDGER-09). **COMMITTEE-CRED-FIDELITY
then CLOSED the committee member/vote discrimination half:** `ConwayGovState.committee`
and `GovActionState.committee_votes` are now `StakeCredential`-discriminated, and
`evaluate_ratification` / `check_ratification` resolve committee hot→cold→member by
full-credential equality (no `.hash()` collapse), so a key-hash hot key never
cross-resolves to a script-hash member of equal bytes (DC-LEDGER-10 strengthened).
**DREP-VOTE-FIDELITY then CLOSED the DRep-vote discrimination half:**
`GovActionState.drep_votes` is now `StakeCredential`-discriminated, and
`evaluate_ratification` / `check_ratification`'s DRep tally resolves a vote by
exact credential variant — `lookup_stake` maps `StakeCredential::KeyHash →
DRep::KeyHash` / `StakeCredential::ScriptHash → DRep::ScriptHash` and reads that
single DRep-stake key, with **no key/script OR-fallback** — so a key-hash DRep
voter never tallies a script-hash DRep's stake of equal bytes (DC-LEDGER-10
strengthened). The committee + DRep vote fingerprint writer is the renamed
`write_credential_vote_list`. **ENACTMENT-COMMITTEE-FIDELITY then CLOSED the
last bare-`Hash28` credential surface in this domain** —
`EnactmentEffects.committee_changes` was discriminated `Hash28` →
`StakeCredential` (DC-LEDGER-10 strengthened). **ENACTMENT-COMMITTEE-WRITEBACK
then WIRED the dormant `UpdateCommittee` enactment LOGIC itself:**
`GovAction::UpdateCommittee` is now the closed structured `{ removed:
BTreeSet<StakeCredential>, added: BTreeMap<StakeCredential, u64>, threshold }`
(GovAction stays a closed 7-variant enum, re-shaped in place); `enact_proposals`
populates `EnactmentEffects.committee_changes` + the new `committee_threshold`
field; and the new closed pure transition `apply_committee_enactment` writes the
change into `ConwayGovState.committee` + `committee_quorum` at the `rules.rs`
epoch boundary (`rules.rs:1224`), with `NoConfidence` dissolving the committee
(DC-EPOCH-01 + DC-LEDGER-10 strengthened, no new rule, no new gate). **Remaining
open seam in this domain:** the declared non-goal `proposal_procedures` tx-body
decode into `GovAction` — the wire codec keeps `proposal_procedures` opaque
`Option<Vec<u8>>`, so a tx-submitted `UpdateCommittee` proposal is not yet
reachable end-to-end; a candidate future seam (§1, §3), NOT an open extension
point now. `spo_votes` is a **permanent non-goal** (pools key-hash only). **168ac02 added one new positive GREEN real-chain oracle test** — `committee_oracle_mainnet_575_576_noop_agreement` (in `crates/ade_testkit/tests/epoch_oracle_comparison.rs`) — that replays the mainnet 575→576 epoch boundary against the real cardano-node snapshot and asserts noop agreement on the committee surface, strengthening DC-EPOCH-01 + DC-LEDGER-10 by extending real-chain oracle coverage to the live `apply_committee_enactment` write-back. It is a positive-oracle test, not a new ingress and not a new seam — the BLUE write-back chokepoint already named above does not move.

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
  `ade_ledger::{tx_validity, mempool, cert_classify, conway, delegation, gov_cert}`).
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
  or catch-all accept arm, so that gate continues to pass.
- `ci_check_gov_cert_accumulation_closed.sh` *(NEW in B5 — DC-LEDGER-09,
  `status=enforced`)* — defends the gov-cert accumulation closure: (1)
  `apply_conway_gov_cert` exists and its `ConwayCert` match has **no `_ =>`
  catch-all** (a new variant breaks the build instead of silently dropping its
  governance effect); (2) the B4 observe-and-drop is gone — the "routed out of
  B4 mutation scope" comment must not reappear and `accumulate_tx_certs` must
  call `apply_conway_gov_cert`; (3) DRep expiry uses `checked_add` (no unchecked
  `current_epoch + drep_activity`); (4) the `MissingDRepActivityParam` env
  fail-fast and the `gov_cert_env()` constructor are present. DC-LEDGER-09
  strengthens DC-LEDGER-08. The new
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
field). **B4 added no extensible surface.** **B5 added two closed surfaces** — the
fail-fast `GovCertEnv` struct (`ade_ledger::state`) and the closed total
dispatch `apply_conway_gov_cert` (`ade_ledger::gov_cert`) — plus the new CI
gate `ci_check_gov_cert_accumulation_closed.sh`, and **enriched one existing
closed surface in place** (`ConwayOnlyDepositParams` gained `drep_activity`).
The `ConwayGovState` proposal set stays the same extensible surface, but its
**accumulation path** is now a closed deterministic fold rather than a frozen
snapshot value. **B5 added no new open extension point.** **OQ5-CREDENTIAL-FIDELITY
re-shaped one existing closed surface and made it discriminant-faithful** —
`ade_types::shelley::cert::StakeCredential` is now the closed 2-variant enum
`{ KeyHash(Hash28), ScriptHash(Hash28) }` (was the tuple-struct
`StakeCredential(pub Hash28)`); it added the new CI gate
`ci_check_credential_discriminant_closed.sh` (DC-LEDGER-10) and re-keyed three
`ConwayGovState` maps on the discriminated credential, with no net new type and
no new open extension point. **COMMITTEE-CRED-FIDELITY extended that same closed
`StakeCredential` discriminant to the committee surface** — `ConwayGovState.committee`
re-keyed `Hash28` → `StakeCredential`, `GovActionState.committee_votes` re-typed
`Vec<(Hash28, Vote)>` → `Vec<(StakeCredential, Vote)>`, and the new closed
fingerprint writer `write_committee_vote_list` — strengthening DC-LEDGER-10 and
**extending the existing CI gate** (no new gate, no new file, CI count stays 29),
with no net new type and no new open extension point. **DREP-VOTE-FIDELITY extended
the same closed `StakeCredential` discriminant once more to the DRep-vote surface** —
`GovActionState.drep_votes` re-typed `Vec<(Hash28, Vote)>` → `Vec<(StakeCredential,
Vote)>`, the DRep tally's exact-variant `lookup_stake` resolution (no OR-fallback),
and the credential-generic rename of the vote-list fingerprint writer
(`write_committee_vote_list → write_credential_vote_list`, now serving committee +
DRep) — strengthening DC-LEDGER-10 again
(`strengthened_in += DREP-VOTE-FIDELITY`) and **extending the existing CI gate
again** (no new gate, no new file, CI count stays 29), with no net new type and no
new open extension point. `spo_votes` stays `Hash28`-keyed by design (a permanent
non-goal). **ENACTMENT-COMMITTEE-FIDELITY then re-typed one more existing closed
surface in place** — `EnactmentEffects.committee_changes` `Hash28` →
`StakeCredential` (DC-LEDGER-10 strengthened, EXTENDED gate, no new gate, CI
count stays 29), with no net new type and no new open extension point.
**ENACTMENT-COMMITTEE-WRITEBACK then re-shaped one existing closed surface in
place and added one closed surface** — `GovAction::UpdateCommittee` moved from
the opaque `{ raw: Vec<u8> }` to the closed structured `{ removed:
BTreeSet<StakeCredential>, added: BTreeMap<StakeCredential, u64>, threshold }`
(the `GovAction` enum stays a **closed 7-variant** enum — one variant re-shaped,
cardinality unchanged), and the new closed pure transition
`apply_committee_enactment` (`ade_ledger::governance`) plus the new
`EnactmentEffects.committee_threshold: Option<(u64, u64)>` field. It extended the
existing CI gate (checks 6 + 7 — DC-EPOCH-01 / DC-LEDGER-10 strengthened, no new
gate, no new file, CI count stays 29), with **no net new type** (`UpdateCommittee`
re-shaped in place, `apply_committee_enactment` is a function,
`committee_threshold` is a field) and **no new open extension point**.

### Closed (frozen — version-gated changes only)

| Registry | Location | Count | Change Rule |
|----------|----------|-------|-------------|
| `CardanoEra` | `ade_types::era` | 8 variants (ByronEbb, ByronRegular, Shelley, Allegra, Mary, Alonzo, Babbage, Conway) | New variant = new hard fork. Coordinated change across `ade_codec`, `ade_ledger`, the canonical type list, and the genesis parser's `later_eras` table. Unknown era tags produce a `CodecError`, never a fallback. |
| `Certificate` | `ade_types::shelley::cert` | 7 variants | Frozen Shelley-era certificate set. New cert types live in `ConwayCert`. **B4:** `PoolRegistrationCert` (in `ade_types::shelley::cert`) gained an `owners: Vec<Hash28>` field (`pool_owners`, additive within the closed surface) and is decoded by the single shared `ade_codec::shelley::cert::read_pool_registration_cert`, the ONE pool-params decode site for both the Shelley and Conway cert decoders. |
| **`StakeCredential`** *(closed 2-variant enum — NEW shape in OQ5; committee surface added in COMMITTEE-CRED-FIDELITY)* | `ade_types::shelley::cert` | 2 variants — `KeyHash(Hash28)`, `ScriptHash(Hash28)` | The closed credential discriminant (was the tuple-struct `StakeCredential(pub Hash28)`). A key-hash and a script-hash of the same 28 bytes are distinct. No `#[non_exhaustive]`, no open tail; carries a read-only `hash()` accessor (the narrow boundary seam). New credential kind = a versioned variant + arm in both `decode_stake_credential` decoders + every match site. **Grep-gated by `ci_check_credential_discriminant_closed.sh`** (the tuple-struct shape may not reappear; neither decoder may revert to a tag-discard form; no bare-`Hash28` `StakeCredential(<hash>)` coercion may reappear on the BLUE path — DC-LEDGER-10). **COMMITTEE-CRED-FIDELITY extended the gate's scope:** `ConwayGovState.committee` must stay `StakeCredential`-keyed and `GovActionState.committee_votes` must stay `Vec<(StakeCredential, Vote)>` (no reversion to `Hash28`). **DREP-VOTE-FIDELITY extended it again:** `GovActionState.drep_votes` must stay `Vec<(StakeCredential, Vote)>`, the DRep-vote serializer must route through `write_credential_vote_list`, and `governance.rs` must carry no DRep key/script OR-fallback (`DRep::KeyHash(...).or_else(...)`). **ENACTMENT-COMMITTEE-FIDELITY extended it (check 6):** `EnactmentEffects.committee_changes` must stay `StakeCredential`-typed (no reversion to bare `Hash28`). **ENACTMENT-COMMITTEE-WRITEBACK extended it once more (check 7):** `GovAction::UpdateCommittee.removed` must stay `BTreeSet<StakeCredential>` and `.added` `BTreeMap<StakeCredential, _>`, `governance.rs` must define `apply_committee_enactment`, and `rules.rs` must call it at the epoch boundary — guarding the live committee write-back against discriminant re-collapse and against silently dropping the change. `spo_votes` stays `Hash28`-keyed by design. |
| **Credential-decode chokepoints** *(closed grammar — NEW in OQ5)* | `ade_codec::{shelley,conway}::cert::decode_stake_credential` | 2 functions | Each reads the credential type tag → `0 → KeyHash`, `1 → ScriptHash`; an unknown tag rejects with `CodecError::InvalidCborStructure { detail: "unknown stake credential type" }`. No catch-all accept, no tag-erasing. Removal/renaming or a tag-discard regression is forbidden (DC-LEDGER-10, `ci_check_credential_discriminant_closed.sh`). |
| **`ConwayCert`** *(closed CDDL grammar — refined in B3, owner-completed in B4)* | `ade_types::conway::cert` | **19 variants** over CDDL tags `0..18` (incl. the explicit `RemovedInConway { tag }` marker for tags 5/6) | The closed Conway-complete certificate domain type. **B4 — owner-complete:** each variant now retains its owner payloads (credentials, pool id, `PoolRegistration(PoolRegistrationCert)` at tag 3, DRep target), enriched additively without an open tail or `#[non_exhaustive]`. **No `#[non_exhaustive]`, no open-tail `Other`/`Unknown`** — `RemovedInConway` is an explicit closed marker. New cert tag = a new explicit variant + a `decode_conway_certs` decoder arm + a `conway_cert_action` arm + an `apply_conway_cert` arm + a `cert_classify::classify` arm, version-gated. Decoder rejects tags ≥19 with `CodecError::UnknownCertTag`; B3F also rejects trailing bytes (`TrailingBytes`) and bounds preallocation. **Closure grep-gated by `ci_check_conway_cert_classification_closed.sh`** (no `#[non_exhaustive]`/open-tail on the type; no catch-all `_ =>` accept arm in the decoder; exhaustive `classify`); the B4 enrichment kept all three gate properties. |
| `GovAction` *(UpdateCommittee re-shaped structured in ENACTMENT-COMMITTEE-WRITEBACK)* | `ade_types::conway::governance` | **7 variants (cardinality unchanged)** | CIP-1694 fixed; new variant = CIP amendment + ratification chokepoint update. **ENACTMENT-COMMITTEE-WRITEBACK** re-shaped the `UpdateCommittee` variant from the opaque `{ prev_action, raw: Vec<u8> }` to the closed structured `{ prev_action, removed: BTreeSet<StakeCredential>, added: BTreeMap<StakeCredential, u64>, threshold: (u64, u64) }` (discriminated cold committee credentials — never bare `Hash28` — gated by `ci_check_credential_discriminant_closed.sh` checks 6 + 7, DC-LEDGER-10). One variant re-shaped in place; the enum stays closed 7-variant. |
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
| **`GovCertEnv`** *(NEW in B5)* | `ade_ledger::state` | closed struct — `{ current_epoch, drep_activity }` | The fail-fast gov-cert environment DRep register/update certs consult. Constructed **only** via `LedgerState::gov_cert_env()`: `Some` iff `drep_activity` is present, else `ValidationEnvironmentError::MissingDRepActivityParam`. No `#[non_exhaustive]`, no `String`. New env field = versioned addition + thread through `gov_cert_env()` + `apply_conway_gov_cert`. |
| **`apply_conway_gov_cert` dispatch** *(NEW in B5 — closed surface, not a registry)* | `ade_ledger::gov_cert` | 1 function — a total `match` over `ConwayCert` (18 tags + removed 5/6) | The closed total governance-cert dispatch. **No `_ =>` wildcard arm** — non-governance tags are explicit no-op arms; a new `ConwayCert` variant breaks the build. New governance cert tag = explicit `ConwayCert` variant + `decode_conway_certs` arm + `conway_cert_action` arm (B4) + `apply_conway_gov_cert` arm, version-gated. DRep expiry is `checked_add` (fail-closed via `DRepActivityOverflow`). Grep-gated by `ci_check_gov_cert_accumulation_closed.sh` (DC-LEDGER-09). |
| **`apply_committee_enactment` write-back** *(NEW in ENACTMENT-COMMITTEE-WRITEBACK — closed surface, not a registry)* | `ade_ledger::governance` | 1 pure transition `fn(&committee, quorum, &EnactmentEffects) -> (committee', quorum')` | The closed committee-enactment write-back. Removes the `removed` cold credentials, inserts the `added` ones with term-expiry epochs, applies `committee_threshold` to the quorum; `NoConfidence` dissolves the committee. Operates on the discriminated `ConwayGovState.committee` (`BTreeMap<StakeCredential, u64>`) — never re-collapses the discriminant. Called at the `rules.rs` epoch boundary (`rules.rs:1224`); replays byte-identically. Grep-gated by `ci_check_credential_discriminant_closed.sh` checks 6 + 7 (DC-EPOCH-01 / DC-LEDGER-10). |
| **`EnactmentEffects` struct** *(committee_changes discriminated in ENACTMENT-COMMITTEE-FIDELITY; committee_threshold added in ENACTMENT-COMMITTEE-WRITEBACK)* | `ade_ledger::governance` | closed struct — incl. `committee_changes: Option<(Vec<StakeCredential>, Vec<(StakeCredential, u64)>)>`, **`committee_threshold: Option<(u64, u64)>`** | The closed enactment-effects carrier. `committee_changes` carries the discriminated cold committee credential removed/added-with-expiry sets; `committee_threshold` carries the new quorum. Populated by `enact_proposals`, consumed by `apply_committee_enactment`. Re-collapse to bare `Hash28` is grep-forbidden (DC-LEDGER-10, `ci_check_credential_discriminant_closed.sh` check 6). |
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
| **`ConwayOnlyDepositParams`** *(NEW in B3; B5-enriched)* | `ade_ledger::pparams` | struct { drep_deposit: Coin, gov_action_deposit: Coin, **drep_activity** } | The Conway-only deposit/governance params. **B5 added `drep_activity`** (the DRep activity window the gov-cert apply uses for expiry, sourced canonically; `GovCertEnv` carries it). Closed shape; the value-set is per-state. |
| **`ConwayDepositParams`** *(NEW in B3)* | `ade_ledger::pparams` | struct (view) { key_deposit, pool_deposit, drep_deposit, gov_action_deposit } | The single canonical view combining `ProtocolParameters.{key_deposit, pool_deposit}` with the Conway-only pair — every deposit/refund amount in BLUE flows from here (DC-TXV-07). Built only via `LedgerState::conway_deposit_view()`. |
| **`ValidationEnvironmentError`** *(NEW in B3)* | `ade_ledger::error` | incl. `MissingConwayDepositParams` | The fail-fast environment-error taxonomy returned when the deposit-param view is consulted on a non-Conway state. Closed, no `String`. |
| **`UnsupportedStateDependentDepositAccounting`** *(NEW in B3)* | `ade_ledger::error` | structured (e.g. `LegacyUnregistrationRefundUnresolved`) | The `cert_classify` reject for a deposit/refund that cannot be resolved from registration state — never a guessed amount (DC-TXV-06). Closed. |
| **`EraInvalidCertificateError`** *(NEW in B3)* | `ade_ledger::error` | struct { cert_index: u16, removed_tag } | The §9.1 step-2 reject for a known-but-removed cert tag (5/6). Closed, flat-data. |
| **`PraosNonces` / `NonceScanError`** *(B1)* | `ade_ledger::consensus_input_extract` | 1 struct (5 nonces) + 1 error | The consensus-input extraction shape. Exact-five-nonce requirement is a closure invariant. |
| **`PraosChainDepState` / `ChainEvent` canonical encodings** *(N-B)* | `ade_core::consensus::encoding` | 4 chokepoints | Frozen CBOR; round-trip required (T-DET-01); field additions are version-gated. |
| **`LedgerFingerprint` fold** *(B3-extended)* | `ade_ledger::fingerprint` | + `CONWAY_DEPOSIT_PARAMS_TAG` fold | The canonical `LedgerState` fingerprint; B3 added a deposit-param fold that is byte-identical for any non-Conway state (DC-LEDGER-01, enforced by `ci_check_ledger_determinism.sh`). |
| **CI check set** | `ci/ci_check_*.sh` | 29 scripts | Existing checks may be tightened, never relaxed. New CI check is additive. Deleting a script requires recording the deprecation in the registry's `ci_scripts` arrays. (B3 added `ci_check_deposit_param_authority.sh`; B3F added `ci_check_conway_cert_classification_closed.sh`; B4 added none — DC-LEDGER-08 reuses `ci_check_forbidden_patterns.sh`; **B5 added `ci_check_gov_cert_accumulation_closed.sh`** for DC-LEDGER-09; **OQ5 added `ci_check_credential_discriminant_closed.sh`** for DC-LEDGER-10; **COMMITTEE-CRED-FIDELITY added no new file** — it EXTENDED that same gate to the committee surface; **DREP-VOTE-FIDELITY added no new file** — it EXTENDED the same gate again to the DRep-vote surface (drep_votes `StakeCredential` + no DRep OR-fallback); **ENACTMENT-COMMITTEE-FIDELITY added no new file** — it EXTENDED the same gate once more to `EnactmentEffects.committee_changes` (`StakeCredential`-typed); **ENACTMENT-COMMITTEE-WRITEBACK added no new file** — it EXTENDED the same gate again with checks 6 + 7 to defend the live committee-enactment surface (`GovAction::UpdateCommittee` structured + discriminated, `apply_committee_enactment` defined, `rules.rs` calls it), so the count stays 29.) |
| **Invariant registry families** | `docs/ade-invariant-registry.toml` | Families T / CN / DC / OP / RO; DC extended in N-A (`DC-PROTO-*`, `DC-CORE-01`), N-B (`DC-CONS-03..10`), B1 (`DC-VAL-01..06`), B2 (`DC-TXV-01..05`, `DC-MEM-01/02`), B3 (`DC-TXV-06`, `DC-TXV-07`), B4 (`DC-LEDGER-08`), and **B5 (`DC-LEDGER-09`, which strengthens `DC-LEDGER-08`)**, and **OQ5-CREDENTIAL-FIDELITY (`DC-LEDGER-10`, which strengthens `T-DET-01` / `T-ENC-03`); COMMITTEE-CRED-FIDELITY, DREP-VOTE-FIDELITY, ENACTMENT-COMMITTEE-FIDELITY, then ENACTMENT-COMMITTEE-WRITEBACK each further strengthen `DC-LEDGER-10` (committee, then DRep-vote, then committee-enactment-effect, then the live committee-enactment write-back — no new rule ID; `strengthened_in += COMMITTEE-CRED-FIDELITY, DREP-VOTE-FIDELITY, ENACTMENT-COMMITTEE-FIDELITY, ENACTMENT-COMMITTEE-WRITEBACK`); ENACTMENT-COMMITTEE-WRITEBACK also strengthens `DC-EPOCH-01` (the epoch-boundary apply now performs the committee write-back)** | Append-only IDs; rules may be strengthened, never weakened; deprecation needs an explicit `deprecated_in`. |

### Extensible (open within constraints)

| Registry | Location | Extension Rule |
|----------|----------|---------------|
| `CostModels` map (Plutus V1/V2/V3 cost tables) | `ade_plutus::cost_model::CostModels` | New entries enter via the cost-model CBOR decoder when a protocol parameter update lands. Not runtime-pluggable; constrained by the closed `PlutusLanguage` set. |
| `ProtocolParameters` / `ProtocolParameterUpdate` field set | `ade_ledger::pparams` | Fields are appended per era. Versioned-gated by era. **B3 note:** the Conway-only `ConwayOnlyDepositParams` (`drep_deposit`, `gov_action_deposit`) are a closed-shape addition; the deposit *view* combining them is `ConwayDepositParams`. **B5 note:** `ConwayOnlyDepositParams` gained `drep_activity`, sourced canonically and carried into `GovCertEnv`. |
| Pool / DRep / Stake registrations | `ade_ledger::state::{DelegationState, CertState}` | Mutated at runtime by `ade_ledger::delegation::apply_cert` (Shelley..Babbage) and, **as of B4**, by `ade_ledger::delegation::apply_conway_cert` (Conway, owner-tagged). The **shape** of what can be registered is closed; the **set** of registrations is open and grows monotonically. **B3 note:** registration state is now the authoritative source for `CoinSource::RegistrationState` refunds (`cert_classify`). **B4 note:** `apply_pool_registration` now populates `PoolParams.owners` from the enriched cert; Conway delegation/pool certs mutate `CertState` here, while governance certs are owner-tagged out of scope (PHASE4-B5). |
| Governance proposal / committee / DRep registration set | `ade_ledger::state::ConwayGovState` | Shape closed, instance set open, lifecycle managed by `evaluate_ratification` / `enact_proposals` / `expire_proposals`. **B5 note:** the owner-tagged governance-cert effects B4 produced are now APPLIED — `gov_cert::apply_conway_gov_cert` (called from `accumulate_tx_certs`) folds vote-delegation / committee / DRep entries into this state. The **shape** of what can be registered is closed and the **accumulation path** is a closed deterministic fold (DC-LEDGER-09); the **set** of registrations is open and grows as certs accumulate. `ConwayGovState` migrated from a frozen snapshot value to this fold (T-DET-01 fingerprint migration). **OQ5 note:** `vote_delegations` / `committee_hot_keys` / `drep_expiry` are now keyed on the discriminated `StakeCredential` (was bare `Hash28`); the **key shape** is the closed `StakeCredential` enum and the fingerprint emits the discriminant (DC-LEDGER-10). **COMMITTEE-CRED-FIDELITY note:** the `committee` member set (`BTreeMap<StakeCredential, u64>`) and the `GovActionState.committee_votes` set (`Vec<(StakeCredential, Vote)>`) are now `StakeCredential`-discriminated too, fingerprinted by `write_committee_vote_list` (DC-LEDGER-10 strengthened). **DREP-VOTE-FIDELITY note:** `GovActionState.drep_votes` is now `StakeCredential`-discriminated too (`Vec<(StakeCredential, Vote)>`), fingerprinted by the renamed `write_credential_vote_list`, and the DRep tally resolves to the exact DRep variant via `lookup_stake` (no key/script OR-fallback); `spo_votes` stays `Hash28`-keyed by design (pools key-hash only). The **set** of members/votes stays open; the **key/element shape** is the closed `StakeCredential` enum. The last bare-`Hash28` credential surface in this state, `EnactmentEffects.committee_changes`, was discriminated in ENACTMENT-COMMITTEE-FIDELITY (`Option<(Vec<StakeCredential>, Vec<(StakeCredential, u64)>)>`, DC-LEDGER-10 strengthened). **ENACTMENT-COMMITTEE-WRITEBACK note:** the committee member set is now also **written back** at the epoch boundary — `apply_committee_enactment` (consuming the discriminated `committee_changes` + `committee_threshold`) removes/inserts cold committee credentials and applies the new quorum into `ConwayGovState.committee` / `committee_quorum`; `NoConfidence` dissolves the committee. The **shape** stays closed (`BTreeMap<StakeCredential, u64>`); the **set** of members evolves through the closed write-back, not a frozen snapshot. `GovAction::UpdateCommittee` is now the closed structured variant (still a closed 7-variant `GovAction`). |
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
| GOVCERT-validity *(OQ-3, separable)* | Committee-membership precondition (whether a hot-key-auth / cold-resign cert references an elected member) | Tier 1 — a tx-validity gate, NOT a registry; declared separable in the B5 cluster doc. B5 accumulates committee certs unconditionally. Confirm shape (a `tx_validity` precondition vs. a `cert_classify` disposition) at cluster entry. |
| credential-discriminant *(OQ-5 + committee — WIRED + CLOSED in OQ5 / COMMITTEE-CRED-FIDELITY)* | Credential key/script discriminant in `ConwayGovState` keys + the committee member/vote sets | **DONE:** OQ5 closed `vote_delegations` / `committee_hot_keys` / `drep_expiry`; COMMITTEE-CRED-FIDELITY closed `committee` + `GovActionState.committee_votes` and the full-credential-equality committee ratification. A closed `StakeCredential` key-type threaded codec → gov-state key → fingerprint. Gated by the extended `ci_check_credential_discriminant_closed.sh` (DC-LEDGER-10). Not an open registry. |
| DRep-vote discriminant *(WIRED + CLOSED in DREP-VOTE-FIDELITY)* | DRep-vote key/script discriminant in `ade_ledger::governance` `drep_votes` | **DONE:** `GovActionState.drep_votes` re-typed `Vec<(Hash28, Vote)>` → `Vec<(StakeCredential, Vote)>`; the DRep tally's `lookup_stake` resolves to the exact DRep variant (no OR-fallback); `write_credential_vote_list` (renamed) emits the discriminant. Gated by the extended `ci_check_credential_discriminant_closed.sh` (DC-LEDGER-10). Not an open registry. |
| committee-enactment write-back *(WIRED + CLOSED in ENACTMENT-COMMITTEE-WRITEBACK)* | `ade_ledger::governance::enact_proposals` `UpdateCommittee` arm — formerly `let _ = raw;`, did not write a ratified committee change into `ConwayGovState.committee` | **DONE:** `GovAction::UpdateCommittee` re-shaped to the closed structured `{ removed, added, threshold }` (closed 7-variant `GovAction`); `enact_proposals` populates `EnactmentEffects.committee_changes` + `committee_threshold`; the closed pure transition `apply_committee_enactment` writes the change into `ConwayGovState.committee` + `committee_quorum` at the `rules.rs` epoch boundary; `NoConfidence` dissolves the committee. Not an open registry — a closed pure transition. Gated by the extended `ci_check_credential_discriminant_closed.sh` checks 6 + 7 (DC-EPOCH-01 / DC-LEDGER-10). |
| proposal-decode *(declared non-goal — separable, NOT an open seam now)* | `proposal_procedures` tx-body decode into `GovAction` — `ade_codec::conway::tx` keeps `proposal_procedures` opaque `Option<Vec<u8>>`; a tx-submitted `UpdateCommittee` proposal is not yet decoded into a typed `GovAction` | Not an open registry — a closed sub-grammar reader inside the Conway tx body (parallel to `decode_conway_certs`) lifting the opaque slice into `Vec<GovAction>`. A separable codec-fidelity follow-up; confirm shape (`Vec<GovAction>` reduction target) at the proposal-decode cluster entry. |

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

### Closed-grammar audit (PHASE4-B5 specific)

This sweep was performed after PHASE4-B5 close.

1. **`apply_conway_gov_cert` dispatch** — **closed by intent, NOT an
   extension point.** A total, compiler-exhaustive `match` over `ConwayCert`
   (all 18 tags + the removed 5/6 marker) with **no `_ =>` wildcard arm** —
   non-governance tags are explicit no-op arms, governance tags fold into
   `ConwayGovState`. A new variant breaks the build. Grep-gated by
   `ci_check_gov_cert_accumulation_closed.sh` (DC-LEDGER-09): no `_ =>` arm, the
   B4 observe-and-drop comment must stay removed, `accumulate_tx_certs` must
   call it, DRep expiry must use `checked_add`, and `MissingDRepActivityParam` /
   `gov_cert_env()` must be present.
2. **`GovCertEnv` + `gov_cert_env()`** — **closed by intent, fail-fast.**
   The sole constructor is `LedgerState::gov_cert_env()`; a non-Conway /
   missing-`drep_activity` state yields `MissingDRepActivityParam`, never a
   fabricated env. No `#[non_exhaustive]`, no `String`.
3. **DRep expiry arithmetic** — **fail-closed by intent.** Expiry is
   `current_epoch.checked_add(drep_activity)`; overflow is the deterministic
   `DRepActivityOverflow` reject, never a silent wrap. Grep-forbidden from
   regressing to an unchecked `+`.
4. **`ConwayGovState` accumulation path** — **closed deterministic fold.**
   The migration from a frozen snapshot value to a fold over replayed
   governance-cert effects is byte-identical on replay
   (`gov_state_accumulation_replays_byte_identical`; T-DET-01 fingerprint
   migration). The owner-tagged `OwnerTaggedEffect` / `GovernanceCertEffect`
   surface B4 produced is **left intact** — B5 is additive native dispatch, not
   a replacement (DC-LEDGER-09 strengthens DC-LEDGER-08).
5. **OQ-3 (committee-membership gate) and OQ-5 (credential key/script
   discriminant)** — **declared SEPARABLE follow-ups, NOT closed-by-accident
   surfaces and NOT open extension points now.** B5 accumulates committee certs
   unconditionally (OQ-3) and keys gov-state on bare `Hash28` (OQ-5, the codec
   collapses the discriminant). Both are candidate future seams (§1, §3); confirm
   before a GOVCERT-validity or committee/DRep-authority cluster opens.

**Gap note — B5 (none new).** B5 added a dedicated grep gate
(`ci_check_gov_cert_accumulation_closed.sh`) for its closed dispatch, env
fail-fast, and checked arithmetic, so the gov-cert accumulation closure is
mechanically enforced rather than test-only. The carried B4 narrow gap stands
unchanged: the `ade_ledger::delegation` owner-tagged apply types are still
outside the `ci_check_consensus_closed_enums.sh` `TARGETS` array (the
`gov_cert.rs` dispatch is covered by its own gate, but the
`delegation.rs` *types* are not). No new B5 gap.

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
- **Closed total gov-cert dispatch contract** *(NEW in B5)*:
  `ade_ledger::gov_cert::apply_conway_gov_cert` is a total, compiler-exhaustive
  `match` over `ConwayCert` (18 tags + removed 5/6) with **no `_ =>` wildcard
  arm** — vote-delegation (9/10/11/12), committee auth-cold (14) /
  cold-resign (15), and DRep register (16) / update (18) / unregister (17) fold
  into `ConwayGovState`; every non-governance tag is an explicit no-op arm.
  Called from `accumulate_tx_certs`; the B4 observe-and-drop is removed
  (DC-LEDGER-09, which strengthens DC-LEDGER-08).
- **Fail-fast gov-cert environment** *(NEW in B5)*: `GovCertEnv`
  `{ current_epoch, drep_activity }` is constructed **only** via
  `LedgerState::gov_cert_env()`; a non-Conway / missing-`drep_activity` state
  yields `ValidationEnvironmentError::MissingDRepActivityParam` — never a
  fabricated env.
- **Checked DRep-expiry arithmetic** *(NEW in B5)*: DRep register/update set
  expiry = `current_epoch.checked_add(drep_activity)`; overflow is the
  deterministic `ValidationEnvironmentError::DRepActivityOverflow` reject, never
  a silent wrap. Grep-forbidden from regressing to an unchecked `+`.
- **`ConwayGovState` deterministic-fold accumulation** *(NEW in B5)*:
  `ConwayGovState` migrated from a frozen snapshot value to a deterministic fold
  over replayed governance-cert effects — byte-identical on replay; the
  `drep_activity` extension to the Conway-deposit fingerprint tag carries the
  T-DET-01 migration (DC-LEDGER-01).
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
- **Closed credential discriminant contract** *(NEW in OQ5; committee surface in COMMITTEE-CRED-FIDELITY; DRep-vote surface in DREP-VOTE-FIDELITY; committee-enactment effect in ENACTMENT-COMMITTEE-FIDELITY; committee-enactment write-back in ENACTMENT-COMMITTEE-WRITEBACK)*:
  `StakeCredential` is the closed 2-variant enum `{ KeyHash(Hash28),
  ScriptHash(Hash28) }`; both per-era `decode_stake_credential` chokepoints
  preserve the discriminant (unknown tag → `CodecError::InvalidCborStructure`),
  and the gov-state keys + fingerprint are discriminant-faithful. OQ5 froze
  `vote_delegations` / `committee_hot_keys` / `drep_expiry`;
  **COMMITTEE-CRED-FIDELITY froze the committee surface** — `ConwayGovState.committee`
  (`StakeCredential`-keyed), `GovActionState.committee_votes`
  (`Vec<(StakeCredential, Vote)>`), the full-credential-equality committee
  ratification (no `.hash()` collapse), and the discriminant-emitting committee
  vote-list fingerprint writer; **DREP-VOTE-FIDELITY froze the DRep-vote surface** —
  `GovActionState.drep_votes` (`Vec<(StakeCredential, Vote)>`), the exact-variant
  DRep tally (`lookup_stake` maps `StakeCredential::KeyHash → DRep::KeyHash` /
  `StakeCredential::ScriptHash → DRep::ScriptHash`, no key/script OR-fallback),
  and the credential-generic rename of the vote-list fingerprint writer
  (`write_committee_vote_list → write_credential_vote_list`, now serving committee
  + DRep); **ENACTMENT-COMMITTEE-FIDELITY froze the committee-enactment effect** —
  `EnactmentEffects.committee_changes` (`Option<(Vec<StakeCredential>,
  Vec<(StakeCredential, u64)>)>`, a guard-rail on the committee write-back);
  **ENACTMENT-COMMITTEE-WRITEBACK froze the structured `UpdateCommittee` + the
  committee write-back** — `GovAction::UpdateCommittee` is the closed structured
  `{ removed: BTreeSet<StakeCredential>, added: BTreeMap<StakeCredential, u64>,
  threshold }` (closed 7-variant `GovAction`), and `apply_committee_enactment` is
  the closed pure write-back at the `rules.rs` epoch boundary. Grep-defended by
  the extended `ci_check_credential_discriminant_closed.sh` (DC-LEDGER-10,
  strengthened in COMMITTEE-CRED-FIDELITY, DREP-VOTE-FIDELITY,
  ENACTMENT-COMMITTEE-FIDELITY, and ENACTMENT-COMMITTEE-WRITEBACK).
  The SPO vote list stays `Hash28`-keyed (`write_vote_list`) — a permanent
  non-goal (pools key-hash only).
- **Committee-enactment write-back contract** *(NEW in ENACTMENT-COMMITTEE-WRITEBACK)*:
  a ratified `UpdateCommittee` enacted at the epoch boundary removes the `removed`
  cold credentials, inserts the `added` ones with term-expiry epochs, and applies
  `committee_threshold` to the quorum via the closed pure transition
  `ade_ledger::governance::apply_committee_enactment`, called once at the
  `ade_ledger::rules` epoch-boundary apply site (`rules.rs:1224`); a ratified
  `NoConfidence` dissolves the committee. The write-back operates on the
  discriminated `ConwayGovState.committee` (`BTreeMap<StakeCredential, u64>`) and
  never re-collapses the discriminant; it replays byte-identically (DC-EPOCH-01 /
  DC-LEDGER-10 strengthened; grep-defended by checks 6 + 7 of
  `ci_check_credential_discriminant_closed.sh`).
- **All canonical types**: shapes frozen at the era / version they
  entered. Adding fields requires a versioned gate; renaming forbidden.
- **TCB color assignments**: per `.idd-config.json` `core_paths`.
  `ade_core::consensus`, `ade_ledger::{block_validity, tx_validity,
  mempool::admit, consensus_view, cert_classify, delegation, gov_cert}`,
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
  `ConwayGovState` effect, or composite — never `Neutral`). For a **governance** cert tag, also a
  `conway_cert_action` arm (B4) and an `apply_conway_gov_cert` arm (B5, no
  `_ =>` wildcard). Version-gated per protocol; the compiler-exhaustive matches
  break the build until every arm is added (DC-LEDGER-08 / DC-LEDGER-09).
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
  WIRED + CLOSED)*: DONE — `ade_ledger::gov_cert::apply_conway_gov_cert` is the
  closed total dispatch (no `_ =>` arm) that consumes the owner-tagged effects
  B4 produced and folds them into `ConwayGovState`, called from
  `accumulate_tx_certs`; no new composer, no new ingress (DC-LEDGER-09,
  strengthens DC-LEDGER-08). **Remaining SEPARABLE version-gated follow-ups**
  (NOT open seams now): a new governance cert tag adds an `apply_conway_gov_cert`
  arm; OQ-3 (a committee-membership precondition tx-validity gate) and OQ-5
  (preserving the credential key/script discriminant) attach above this domain.
- **Credential discriminant extension** *(OQ5 / COMMITTEE-CRED-FIDELITY /
  DREP-VOTE-FIDELITY follow-ups)*: extending the closed `StakeCredential`
  discriminant into a surface that still carries a bare `Hash28` — the
  withdrawal/required-signer/address credentials; the `Hash28`-keyed
  stake-distribution snapshot; the Byron credential surface. (`EnactmentEffects.committee_changes`
  was **DONE in ENACTMENT-COMMITTEE-FIDELITY** — discriminated; and the dormant
  `UpdateCommittee` enactment LOGIC it guarded is **DONE in
  ENACTMENT-COMMITTEE-WRITEBACK** — the structured `UpdateCommittee` +
  `apply_committee_enactment` write-back. What remains in the governance domain is
  the declared non-goal `proposal_procedures` tx-body decode into `GovAction`, a
  separate codec-fidelity seam.) Each remaining credential surface is a separable per-surface fidelity follow-up that extends
  `ci_check_credential_discriminant_closed.sh`'s scope, not a change to
  `decode_stake_credential`. Declared non-goal candidate seams (§1, §3).
  **DONE in DREP-VOTE-FIDELITY:** the DRep-vote `lookup_stake` key/script
  OR-fallback in `ade_ledger::governance` is closed — `drep_votes` is
  `StakeCredential`-discriminated and the tally resolves to the exact DRep variant.
  **`spo_votes` is a permanent non-goal** (pools key-hash only — no discriminant to
  preserve).
- **Committee-enactment write-back** *(ENACTMENT-COMMITTEE-WRITEBACK, WIRED +
  CLOSED)*: DONE — `GovAction::UpdateCommittee` is the closed structured variant
  and `ade_ledger::governance::apply_committee_enactment` is the closed pure
  write-back at the `rules.rs` epoch boundary; `NoConfidence` dissolves the
  committee. No new composer, no new ingress (DC-EPOCH-01 / DC-LEDGER-10
  strengthened). **Remaining SEPARABLE follow-up** (NOT an open seam now):
  decoding `proposal_procedures` from real tx bodies into `GovAction` — the wire
  codec keeps `proposal_procedures` opaque `Option<Vec<u8>>`. A closed
  sub-grammar reader inside the Conway tx body lifting the opaque slice into
  `Vec<GovAction>`; version-gated per protocol; confirm at the proposal-decode
  cluster entry.
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
mutating `ConwayGovState` from inside B4's cert path. **B5 followed exactly that rule** — it added
**one new crate-internal BLUE submodule** (`ade_ledger::gov_cert`, the closed
total dispatch `apply_conway_gov_cert`), the closed `GovCertEnv` +
`gov_cert_env()` accessor on `ade_ledger::state`, the `drep_activity` field on
`ConwayOnlyDepositParams`, and the `Option<ConwayGovState>` threading in
`ade_ledger::rules` — **no new crate, no new ingress, no new composer**, and it
added **one new CI gate** (`ci_check_gov_cert_accumulation_closed.sh`,
DC-LEDGER-09). The gov-cert apply consumes the existing `ConwayCert` /
`ConwayGovState` types; it mutates `ConwayGovState` from a single closed
chokepoint reached via `accumulate_tx_certs`, not from a new composer.
**OQ5-CREDENTIAL-FIDELITY followed the same model — no new crate, no new module,
no new ingress, no new composer, and no net new canonical type.** It re-shaped
an existing closed type in place (`ade_types::shelley::cert::StakeCredential`
tuple-struct → closed 2-variant enum), tightened the two existing per-era
`decode_stake_credential` chokepoints to preserve the discriminant, re-keyed
three existing `ConwayGovState` maps on the discriminated credential, and made
the existing canonical fingerprint discriminant-faithful. It added **one new CI
gate** (`ci_check_credential_discriminant_closed.sh`, DC-LEDGER-10). **The
module-addition rule OQ5 sets for future credential-fidelity work:** a surface
that needs the key/script distinction (withdrawals/required-signer/address
credentials; the stake-distribution snapshot; the Byron credential surface — but
NOT `spo_votes`, a permanent non-goal, and NOT `EnactmentEffects.committee_changes`,
discriminated and closed in ENACTMENT-COMMITTEE-FIDELITY) extends the closed
`StakeCredential` discriminant into that
surface in place — it does NOT add a new credential type
and does NOT change `decode_stake_credential`; and the read-only `cred.hash()`
adapter is the only sanctioned discriminant-discarding move, used solely against
declared non-goal surfaces. **COMMITTEE-CRED-FIDELITY followed exactly that
rule** — no new crate, no new module, no new ingress, no new composer, no net new
canonical type, and **no new CI gate** (it EXTENDED
`ci_check_credential_discriminant_closed.sh` to the committee surface). It
re-keyed `ConwayGovState.committee` in place (`Hash28` → `StakeCredential`),
re-typed `GovActionState.committee_votes` in place
(`Vec<(Hash28, Vote)>` → `Vec<(StakeCredential, Vote)>`), tightened the committee
ratification resolution to full-credential equality, and added the closed
serializer `write_committee_vote_list`. DC-LEDGER-10 was strengthened, not
re-issued. **DREP-VOTE-FIDELITY followed exactly the same rule** — no new crate,
no new module, no new ingress, no new composer, no net new canonical type, and
**no new CI gate** (it EXTENDED `ci_check_credential_discriminant_closed.sh` to the
DRep-vote surface). It re-typed `GovActionState.drep_votes` in place
(`Vec<(Hash28, Vote)>` → `Vec<(StakeCredential, Vote)>`), tightened the DRep
tally's `lookup_stake` resolution to the exact DRep variant (no key/script
OR-fallback), and **renamed the committee-vote writer credential-generic**
(`write_committee_vote_list → write_credential_vote_list`, now serving committee +
DRep — output-identical) plus the snapshot-loader parser
(`parse_committee_vote_map → parse_credential_vote_map`). DC-LEDGER-10 was
strengthened again (`strengthened_in += DREP-VOTE-FIDELITY`), not re-issued.
`spo_votes` stayed `Hash28`-keyed (a permanent non-goal — pools key-hash only).
**ENACTMENT-COMMITTEE-FIDELITY followed exactly the same rule** — no new crate, no
new module, no new ingress, no new composer, no net new canonical type, and **no
new CI gate** (it EXTENDED `ci_check_credential_discriminant_closed.sh` to the
committee-enactment effect). It re-typed `EnactmentEffects.committee_changes` in
place (`Option<(Vec<Hash28>, Vec<(Hash28, u64)>)>` →
`Option<(Vec<StakeCredential>, Vec<(StakeCredential, u64)>)>`) — a preventive
guard-rail; at that HEAD the field stayed dormant and `UpdateCommittee` enactment
was still a no-op. DC-LEDGER-10 was strengthened once more
(`strengthened_in += ENACTMENT-COMMITTEE-FIDELITY`), not re-issued.
**ENACTMENT-COMMITTEE-WRITEBACK followed the same model — no new crate, no new
module, no new ingress surface, no new public composer, and no net new canonical
type, and no new CI gate** (it EXTENDED `ci_check_credential_discriminant_closed.sh`
with checks 6 + 7). It re-shaped the `GovAction::UpdateCommittee` variant in place
(opaque `{ raw: Vec<u8> }` → structured `{ removed: BTreeSet<StakeCredential>,
added: BTreeMap<StakeCredential, u64>, threshold }`; the closed 7-variant
`GovAction` is the same already-counted type), populated the already-discriminated
`EnactmentEffects.committee_changes` from `enact_proposals` and added the new
`EnactmentEffects.committee_threshold` field, added the **closed pure transition
`ade_ledger::governance::apply_committee_enactment`** (a function, not a type) and
called it at the existing `ade_ledger::rules` epoch-boundary apply site, and added
fail-closed GREEN snapshot-decode helpers (`parse_cold_credential` /
`parse_cold_credential_set` / `parse_cold_credential_epoch_map` /
`parse_unit_interval`). **The module-addition rule it confirms:** a governance
enactment write-back attaches as a new BLUE pure transition in
`ade_ledger::governance`, consumed at the existing epoch-boundary apply site in
`ade_ledger::rules` — never as a new composer and never by mutating
`ConwayGovState` from a new ingress. DC-EPOCH-01 + DC-LEDGER-10 were strengthened
(`strengthened_in += ENACTMENT-COMMITTEE-WRITEBACK`), not re-issued.

| Color | Naming convention | Build-config flags | May depend on | MUST NOT depend on |
|-------|-------------------|--------------------|----------------|--------------------|
| **BLUE** | `ade_*` (no color in name) | First line of every `.rs` is the contract banner `// Core Contract:`. `lib.rs` carries `#![deny(unsafe_code)]`, `#![deny(clippy::unwrap_used/expect_used/panic/float_arithmetic)]`. No `#[cfg(feature = ...)]`. No async (DC-CORE-01). No `ChainDb`/`f32`/`f64`/density inside `ade_core::consensus`. No `#[non_exhaustive]`/open-tail/`String`/`Box<dyn>` in `ade_core::consensus`, `ade_ledger::block_validity`, `ade_ledger::tx_validity`, and `ade_ledger::mempool` (B3's closed cert/disposition enums in `ade_types::conway::cert` hold the same shape and are grep-gated as of B3F by `ci_check_conway_cert_classification_closed.sh`). No deposit/refund literal next to a deposit field; no testkit `ConwayGovParams` read (DC-TXV-07). No tuple-struct `StakeCredential(Hash28)` shape, no tag-discard credential decode, no bare-`Hash28` `StakeCredential(<hash>)` coercion on the BLUE path (DC-LEDGER-10, `ci_check_credential_discriminant_closed.sh`). | Other BLUE crates / submodules only (incl. the `ade_ledger → ade_core` edge) | Any RED submodule or crate; GREEN in non-dev deps; `pallas_*` (except `ade_plutus`); async runtime; `HashMap`/`HashSet`/`IndexMap`; clock/rand/float/env/I/O. |
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
   closed shape). For any new credential surface that must preserve the
   key/script discriminant, extend the closed `StakeCredential` enum and add
   the surface's BLUE path to `ci_check_credential_discriminant_closed.sh` (it
   forbids the tuple-struct shape, the tag-discard decode form, and any
   bare-`Hash28` `StakeCredential(<hash>)` coercion on the scanned BLUE path —
   DC-LEDGER-10). For any new deposit/refund amount source, add the path to the
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
- **PHASE4-B5 (Conway governance certificate accumulation authority) — DONE**:
  the new BLUE module `ade_ledger::gov_cert` (`apply_conway_gov_cert`, a closed
  total dispatch with no `_ =>` arm) consumes the owner-tagged effects B4
  produced and folds them into `ConwayGovState`, called from
  `accumulate_tx_certs`. No new composer, no new ingress, no new crate; one new
  CI gate (`ci_check_gov_cert_accumulation_closed.sh`, DC-LEDGER-09 — strengthens
  DC-LEDGER-08). **Separable follow-ups (NOT this cluster):** OQ-3
  (committee-membership tx-validity gate) and OQ-5 (credential key/script
  discriminant) — candidate future seams, confirm at their own cluster entry.
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
- **(B5 specific — closed gov-cert dispatch)** No `_ =>` wildcard arm in
  `apply_conway_gov_cert` — the `match` over `ConwayCert` is total
  (18 tags + removed 5/6); non-governance tags are explicit no-op arms, so a new
  `ConwayCert` variant breaks the build instead of silently dropping its
  governance effect. No reintroduction of the B4 "routed out of B4 mutation
  scope" observe-and-drop — `accumulate_tx_certs` must call
  `apply_conway_gov_cert` and fold the result into `ConwayGovState`
  (DC-LEDGER-09, `ci_check_gov_cert_accumulation_closed.sh`).
- **(B5 specific — fail-fast env / checked arithmetic)** No fabricated
  `GovCertEnv` — it is constructed only via `gov_cert_env()`, which fails fast
  with `MissingDRepActivityParam` on a non-Conway / missing-param state. No
  unchecked `current_epoch + drep_activity` for DRep expiry — `checked_add` only,
  with the deterministic `DRepActivityOverflow` reject on overflow
  (DC-LEDGER-09).
- **(B5 specific — gov-state fold)** No non-deterministic `ConwayGovState`
  accumulation — the migration from a frozen snapshot value to a fold over
  replayed governance-cert effects must replay byte-identically (T-DET-01;
  `gov_state_accumulation_replays_byte_identical`).
- **(OQ5 specific — closed credential discriminant)** No tuple-struct
  `StakeCredential(Hash28)` shape — `StakeCredential` is the closed 2-variant
  enum `{ KeyHash(Hash28), ScriptHash(Hash28) }`. No tag-erasing decode — both
  `decode_stake_credential` chokepoints (Shelley + Conway) read the credential
  type tag and map `0 → KeyHash` / `1 → ScriptHash`, rejecting an unknown tag
  with `CodecError::InvalidCborStructure { detail: "unknown stake credential type" }`;
  the prior `let (_cred_type|_tag, _)` discard form may not reappear. No
  bare-`Hash28` `StakeCredential(<hash>)` coercion on the BLUE path (`ade_codec` /
  `ade_ledger` / `ade_types`) — that would re-collapse the discriminant
  (DC-LEDGER-10, `ci_check_credential_discriminant_closed.sh`).
- **(OQ5 / COMMITTEE-CRED-FIDELITY / DREP-VOTE-FIDELITY — discriminant-faithful gov-state + fingerprint)** No re-key
  of `ConwayGovState.{vote_delegations, committee_hot_keys, drep_expiry, committee}`
  or re-type of `GovActionState.{committee_votes, drep_votes}` back to bare `Hash28`
  — they key/carry the discriminated `StakeCredential`. (`spo_votes` stays
  `Hash28`-keyed by design — pools key-hash only.) `fingerprint::write_stake_credential`
  must emit the discriminant (`0`/`1`) before the 28-byte hash; the gov-map fingerprint
  writers must call it, not `write_hash28`. **`write_credential_vote_list`
  (DREP-VOTE-FIDELITY's rename of `write_committee_vote_list`)** is the closed
  discriminant-emitting vote-list writer for BOTH `committee_votes` and `drep_votes`
  — it must not collapse to `write_hash28` / `write_vote_list`. Committee ratification
  must resolve hot→cold→member by full-credential equality, never via `.hash()`; the
  DRep tally must resolve a vote by exact credential variant
  (`StakeCredential::KeyHash → DRep::KeyHash` / `StakeCredential::ScriptHash →
  DRep::ScriptHash`, single-key read) with **no key/script OR-fallback**
  (`DRep::KeyHash(...).or_else(...)` is grep-forbidden in `governance.rs`). Two states
  differing only in a committee or DRep credential's key/script tag fingerprint
  differently (T-DET-01 / strengthens T-ENC-03; DC-LEDGER-10 strengthened,
  EXTENDED gate, no new gate).
- **(OQ5 / COMMITTEE-CRED-FIDELITY / DREP-VOTE-FIDELITY — narrow boundary adapter)** `StakeCredential::hash()` is the
  ONLY sanctioned discriminant-discarding move and is used ONLY against the
  remaining declared non-goal surface (the `Hash28`-keyed stake-distribution
  snapshot in `epoch.rs` / `governance` / `apply_epoch_boundary_with_registrations`).
  **COMMITTEE-CRED-FIDELITY removed the committee member / committee-vote sets from
  this adapter's scope** — they are now discriminated and resolved by full-credential
  equality. `cred.hash()` must never be used to re-key authoritative `ConwayGovState`
  state.
- **(ENACTMENT-COMMITTEE-WRITEBACK specific — committee enactment write-back)** No
  opaque-bytes `UpdateCommittee` — `GovAction::UpdateCommittee` is the closed
  structured `{ removed: BTreeSet<StakeCredential>, added:
  BTreeMap<StakeCredential, u64>, threshold }`, never an opaque `raw: Vec<u8>` or
  bare-`Hash28` committee credential (grep-defended by
  `ci_check_credential_discriminant_closed.sh` check 7). No committee write-back
  outside `ade_ledger::governance::apply_committee_enactment`, and `ade_ledger::rules`
  MUST call it at the epoch boundary — silently dropping a ratified committee
  change is grep-forbidden. The write-back operates on the discriminated
  `ConwayGovState.committee` (`BTreeMap<StakeCredential, u64>`) and must never
  re-collapse the key/script discriminant; `NoConfidence` dissolves the committee;
  the transition replays byte-identically (DC-EPOCH-01 / DC-LEDGER-10
  strengthened).

### GREEN (`ade_testkit` incl. `validity` / `tx_validity` + the B3 conservation corpora + the B4 cert-state corpus + the B5 gov-state corpus + the OQ5 credential-fidelity corpus + the COMMITTEE-CRED-FIDELITY committee cross-resolve corpus + the DREP-VOTE-FIDELITY DRep cross-resolve corpus, `ade_network::lib` / `mux::mod`, `ade_runtime::consensus::{candidate_fragment, chain_selector}`, `ade_ledger::mempool::policy`)

- No nondeterminism that leaks into stored fixtures — fixtures must be
  byte-reproducible (the block-validity, tx-validity, B3 conservation,
  B4 cert-state, B5 gov-state, and OQ5 credential-fidelity corpora replay
  identically — `cert_state_replay_byte_identical`,
  `gov_state_accumulation_replays_byte_identical`,
  `credential_accumulation_replays_byte_identical`).
- No participation in authoritative outputs. The B1/B2/B3/B4 validity
  harnesses only *drive* `block_validity` / `tx_validity` /
  `check_conway_coin_conservation` / `apply_conway_cert` /
  `apply_conway_gov_cert` (via `accumulate_tx_certs`) and assert; the mutators
  are deterministic transforms over real or synthetic corpus blocks/txs/certs.
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
  mempool admission gate, the B3 full value-conservation accounting, the
  B4 Conway cert-state accumulation, and the B5 Conway governance-cert
  accumulation are all Tier-1 surfaces. (Like B4, B5's real epoch-oracle
  obligation is environment-blocked by the absent UMap/gov-state snapshot, not a
  stub; B5 closes mechanically with the closed total dispatch + checked
  fail-closed arithmetic + the synthetic positive/replay/adversarial gov-state
  corpus. OQ-3 and OQ-5 are deliberately-deferred separable follow-ups, not
  Tier-1 stubs.)

---

## Cross-references

- CODEMAP: `docs/ade-CODEMAP.md` — module-by-module authority table,
  upstream of this document. **Cross-reference check:** CODEMAP was
  regenerated at HEAD (`168ac02`) — same SHA this SEAMS reports. CODEMAP's
  header explicitly records the six post-`3180e27` commits as
  GREEN-scope only (testkit harness, snapshot-loader follow-ups, corpus
  generators, real-chain committee oracle), with **376 canonical types,
  1325 tests (+3 vs `3180e27`, all in `ade_testkit`), 29 CI checks** —
  matching the counts this SEAMS reports. CODEMAP also folds in
  ENACTMENT-COMMITTEE-WRITEBACK — the `GovAction::UpdateCommittee` re-shaping
  (opaque `{ raw: Vec<u8> }` → structured `{ removed: BTreeSet<StakeCredential>,
  added: BTreeMap<StakeCredential, u64>, threshold }`, GovAction stays closed
  7-variant), the new `EnactmentEffects.committee_threshold` field, the new
  `ade_ledger::governance::apply_committee_enactment` function, the `rules.rs`
  epoch-boundary apply-site call, the GREEN snapshot-decode helpers
  (`parse_cold_credential` / `_set` / `_epoch_map` / `parse_unit_interval`), and
  the EXTENDED `ci_check_credential_discriminant_closed.sh` (checks 6 + 7) all
  appear in its narrative; it records **376 canonical types** (unchanged —
  `UpdateCommittee` re-shaped in place, `apply_committee_enactment` is a function,
  `committee_threshold` is a field; no net new type) and **29 CI checks**
  (unchanged — the gate was EXTENDED, no new file). No new module, no new crate,
  no new ingress surface — so no CODEMAP/SEAMS module-list drift. Prior
  ENACTMENT-COMMITTEE-FIDELITY close: HEAD `a6b8de7` — the
  `EnactmentEffects.committee_changes` discriminant re-type and the EXTENDED gate
  (check 6) appear in its entries; 376 types, 29 CI checks (unchanged). Prior
  DREP-VOTE-FIDELITY close:
  HEAD `62c9020` — the `GovActionState.drep_votes` re-type
  (`Vec<(Hash28, Vote)>` → `Vec<(StakeCredential, Vote)>`; `spo_votes` stays
  `Hash28`-keyed), the exact-variant DRep `lookup_stake` resolution (no
  OR-fallback), and the `fingerprint::write_committee_vote_list →
  write_credential_vote_list` rename all appear in its entries; it records
  **376 canonical types** (unchanged — DREP-VOTE-FIDELITY made a field-type
  change on the existing `GovActionState`, no net new type), **1313 tests**
  (the DREP-VOTE-FIDELITY +2 are the `drep_keyhash_scripthash_do_not_cross_resolve`
  negative in `governance.rs` and the `drep_vote_discriminant_changes_fingerprint`
  test in `credential_fidelity_corpus.rs`, both appended to DC-LEDGER-10's `tests`
  array), and **29 CI checks** (unchanged — the existing
  `ci_check_credential_discriminant_closed.sh` was EXTENDED again, no new file).
  (Prior COMMITTEE-CRED-FIDELITY close: HEAD `2aeea16`, 376 types, 29 CI checks —
  the committee re-key + `committee_votes` re-type + `write_committee_vote_list`.
  Prior OQ5 close: HEAD `a3ee2da` — the closed 2-variant `StakeCredential`, the
  discriminant-preserving `decode_stake_credential` chokepoints, the OQ5
  `ConwayGovState` re-key, the `cred.hash()` adapter, and the new
  `ci_check_credential_discriminant_closed.sh`.)
  Both docs agree that ENACTMENT-COMMITTEE-WRITEBACK STRENGTHENS DC-EPOCH-01 and
  DC-LEDGER-10 (no new rule, `strengthened_in += ENACTMENT-COMMITTEE-WRITEBACK`)
  and EXTENDS the existing `ci_check_credential_discriminant_closed.sh` (checks
  6 + 7 — `EnactmentEffects.committee_changes` `StakeCredential`-typed,
  `GovAction::UpdateCommittee` structured + discriminated,
  `apply_committee_enactment` defined, `rules.rs` calls it) rather than adding a
  gate, that the `GovAction` enum stays a closed 7-variant enum (one variant
  re-shaped in place — no net new type), that the dormant `UpdateCommittee`
  enactment LOGIC is now WIRED + CLOSED (the only remaining open governance-domain
  seam is the declared non-goal `proposal_procedures` tx-body decode, the wire
  codec keeping `proposal_procedures` opaque `Option<Vec<u8>>`), that the prior
  DC-LEDGER-09 gate (`ci_check_gov_cert_accumulation_closed.sh`) stands, and that
  the carried B4 narrow gap stands — the `ade_ledger::delegation` owner-tagged
  *types* are still NOT in the `ci_check_consensus_closed_enums.sh` `TARGETS`
  array. The two docs are consistent at this SHA.
- Invariant registry: `docs/ade-invariant-registry.toml` — rule families
  incl. `T`, `CN`, `DC` (with `DC-PROTO-*` + `DC-CORE-01` under N-A,
  `DC-CONS-03..10` under N-B, `DC-VAL-01..06` under B1,
  `DC-TXV-01..05` + `DC-MEM-01/02` under B2, **`DC-TXV-06` +
  `DC-TXV-07` under B3** (`DC-TXV-06` moved partial→enforced in B3F via
  `ci_check_conway_cert_classification_closed.sh`), `DC-LEDGER-08`
  under B4 (`status=enforced`, `ci_script` = `ci_check_forbidden_patterns.sh`),
  **`DC-LEDGER-09` under B5** (`status=enforced`, `ci_script` =
  `ci_check_gov_cert_accumulation_closed.sh`, strengthens `DC-LEDGER-08`), and
  **`DC-LEDGER-10` under OQ5-CREDENTIAL-FIDELITY** (`status=enforced`, `ci_script` =
  `ci_check_credential_discriminant_closed.sh`, strengthens `T-DET-01` /
  `T-ENC-03`; **further strengthened in COMMITTEE-CRED-FIDELITY,
  DREP-VOTE-FIDELITY, ENACTMENT-COMMITTEE-FIDELITY, and
  ENACTMENT-COMMITTEE-WRITEBACK** — `strengthened_in += COMMITTEE-CRED-FIDELITY,
  DREP-VOTE-FIDELITY, ENACTMENT-COMMITTEE-FIDELITY, ENACTMENT-COMMITTEE-WRITEBACK`;
  the gate now also defends the committee + DRep-vote + committee-enactment-effect
  surfaces and the live committee write-back surface, checks 6 + 7), and
  **`DC-EPOCH-01` strengthened in ENACTMENT-COMMITTEE-WRITEBACK** (the
  epoch-boundary apply now performs the committee write-back via
  `apply_committee_enactment`);
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
- Cluster B5 (closed): `docs/clusters/PHASE4-B5/cluster.md` +
  `B5-S{2..5}.md` (declares OQ-3 / OQ-5 as separable follow-ups).
- Cluster OQ5-CREDENTIAL-FIDELITY (closed): the cluster doc + slices (WIRES
  AND CLOSES OQ-5; declares the withdrawal/required-signer/address credential
  discriminant, the `Hash28`-keyed stake-distribution snapshot, committee
  member/vote discrimination, and the Byron credential surface as separable
  non-goal follow-ups; OQ-3 stays separable).
- Cluster COMMITTEE-CRED-FIDELITY (closed): the cluster doc + slices S1/S2
  (WIRES AND CLOSES the committee member/vote discrimination — STRENGTHENS
  DC-LEDGER-10, no new rule, no new CI gate; declares DRep-vote discrimination
  and `EnactmentEffects.committee_changes` as separable non-goal follow-ups).
- Cluster DREP-VOTE-FIDELITY (closed): the cluster doc + slices S1/S2
  (`ba4ff37` discriminate `drep_votes` + exact-variant DRep `lookup_stake`
  resolution; `62c9020` DRep cross-resolve negative + CI gate) — WIRES AND CLOSES
  the DRep-vote discrimination, STRENGTHENS DC-LEDGER-10
  (`strengthened_in += DREP-VOTE-FIDELITY`), no new rule, no new CI gate; declares
  `EnactmentEffects.committee_changes` as a separable non-goal follow-up and
  `spo_votes` as a permanent non-goal.
- Cluster ENACTMENT-COMMITTEE-FIDELITY (closed): the cluster doc + slice S1
  (`a6b8de7` re-type `EnactmentEffects.committee_changes`
  `Hash28` → `StakeCredential`, preventive — the field was dormant and
  `UpdateCommittee` enactment was still a no-op) — WIRES AND CLOSES the
  `EnactmentEffects.committee_changes` type-fidelity seam, STRENGTHENS DC-LEDGER-10
  (`strengthened_in += ENACTMENT-COMMITTEE-FIDELITY`), no new rule, no new CI gate
  (extends `ci_check_credential_discriminant_closed.sh` — count stays 29); left
  the dormant `UpdateCommittee` enactment LOGIC as a separable governance-enactment
  seam.
- Cluster ENACTMENT-COMMITTEE-WRITEBACK (closed): the cluster doc + slices S1/S2
  (`f2f15f9` structured `GovAction::UpdateCommittee`
  `{ removed, added, threshold }` + GREEN snapshot-decode helpers; `3180e27` wire
  the committee write-back — `enact_proposals` populates `committee_changes` +
  `committee_threshold`, the closed pure transition `apply_committee_enactment`
  writes into `ConwayGovState.committee` + `committee_quorum` at the `rules.rs`
  epoch boundary, `NoConfidence` dissolves the committee) — WIRES AND CLOSES the
  dormant `UpdateCommittee` enactment LOGIC, STRENGTHENS DC-EPOCH-01 + DC-LEDGER-10
  (`strengthened_in += ENACTMENT-COMMITTEE-WRITEBACK`), no new rule, no new CI gate
  (extends `ci_check_credential_discriminant_closed.sh` with checks 6 + 7 — count
  stays 29), no new crate / module / ingress / composer / net new type; leaves the
  `proposal_procedures` tx-body decode into `GovAction` (wire codec keeps
  `proposal_procedures` opaque `Option<Vec<u8>>`) as the one declared non-goal
  governance-domain seam.
- N-A live-interop evidence: `docs/active/CE-N-A-5_evidence.toml`.
