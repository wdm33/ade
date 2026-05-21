# HEAD Deltas — Ade

> **Status:** Living architectural document. Regenerated; not hand-edited.
> Regenerate via `/head-deltas <baseline>`. Baseline is declared in
> `.idd-config.json` (`head_deltas_baseline`).

> Baseline: `d509f02` (Phase 3 handoff snapshot, 2026-04-15)
> HEAD: `a6b8de7` (feat(ledger): discriminate EnactmentEffects.committee_changes (ENACTMENT-COMMITTEE-FIDELITY-S1), 2026-05-22)
> 123 commits, 11,272 files changed, +172,611 / −7,233,532 lines

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
> below are verbatim from `git log d509f02..HEAD` at this HEAD. The
> six PHASE4-B5 commits (`fdb6601`, `9c8d118`, `7a48727`, `d63c700`,
> `06385d0`, `651adc9`) extend the previous (B4-level) regen's HEAD —
> the B4 close `644eb03` (which committed the B4 grounding-doc refresh
> on top of the B4-S5 implementation HEAD `ee35493`).

> **ENACTMENT-COMMITTEE-FIDELITY cluster note (newest thread).** This
> regen is cut at committed HEAD `a6b8de7`. Since the DREP-VOTE-FIDELITY
> close `06f517f` (the grounding-doc refresh that committed the
> DREP-VOTE CODEMAP/SEAMS/HEAD_DELTAS/TRACEABILITY ripple on top of the
> DREP-VOTE-FIDELITY-S2 implementation HEAD `62c9020`), the
> **ENACTMENT-COMMITTEE-FIDELITY arc** has landed as two commits:
> `5d64fee` (cluster plan + invariants, *strengthens* `DC-LEDGER-10`) and
> `a6b8de7` (S1, discriminate `EnactmentEffects.committee_changes`). It
> discharges the DREP-VOTE-FIDELITY carry-forward follow-up **(d)** — the
> only remaining bare-`Hash28` committee-credential surface — by a
> **one-line preventive type migration**: `EnactmentEffects.committee_changes`
> (`ade_ledger::governance`) re-typed `Option<(Vec<Hash28>, Vec<(Hash28,
> u64)>)>` → `Option<(Vec<StakeCredential>, Vec<(StakeCredential, u64)>)>`.
> The field is **DORMANT** — `UpdateCommittee` enactment is still a no-op
> and the field stays `None` by default — so this pins the *type*, not
> live behavior; it prevents committee enactment, once wired, from
> re-collapsing the discriminated `ConwayGovState.committee` map on
> write-back. **No new module, no new rule, no new CI script.** The
> existing `ci/ci_check_credential_discriminant_closed.sh` was
> **extended** with one clause (clause 6: `pub committee_changes:` must
> carry `StakeCredential`) — it stays the **29th** script; **the CI count
> does not increment.** **`DC-LEDGER-10` is STRENGTHENED a third time**
> (`strengthened_in += ENACTMENT-COMMITTEE-FIDELITY`; +1 test → 14;
> `code_locus` extended) — it is the same rule, not a new one; the
> registry total **stays 173**. **No golden drift** (the field is dormant
> `None`; no fingerprint surface changes). Of the DREP-VOTE-FIDELITY
> carry-forward follow-ups, **(d) is now RESOLVED**; the two remaining are
> unchanged and out of this cluster's scope: **(e)** the GREEN loader
> `mk_credential` defaults `tag != 1` to `KeyHash` (contained to
> `ade_testkit`, cannot reach the node binary), and the pre-OQ5 **(b)**
> Shelley unknown-cert zero-hash placeholder remains a WARN LOW non-goal.
> Both ENACTMENT-COMMITTEE-FIDELITY commits carry the model-attribution
> trailer.

> **DREP-VOTE-FIDELITY cluster note (prior thread).** That regen was
> cut at committed HEAD `62c9020`. Since the COMMITTEE-CRED-FIDELITY
> close `a157c92` (the grounding-doc refresh that committed the
> COMMITTEE-CRED CODEMAP/SEAMS/HEAD_DELTAS/TRACEABILITY ripple on top of
> the COMMITTEE-CRED-FIDELITY-S2 implementation HEAD `2aeea16`), the
> **DREP-VOTE-FIDELITY arc** has landed as three commits: `ecb0b92`
> (cluster plan + invariants, *strengthens* `DC-LEDGER-10`), `ba4ff37`
> (S1, discriminate `drep_votes` + exact-variant DRep stake resolution),
> and `62c9020` (S2, DRep cross-resolve negative + CI gate, `DC-LEDGER-10`
> *strengthened*). It discharges the COMMITTEE-CRED-FIDELITY per-cluster
> security-review follow-up **(c)** — the DRep-vote key/script
> OR-fallback `governance.rs` `lookup_stake` did over identical bytes —
> the natural next discriminant cluster after the committee surface.
> **No new module, no new rule, no new CI script.** It is an in-place
> type change: `GovActionState.drep_votes` re-typed `Vec<(Hash28, Vote)>`
> → `Vec<(StakeCredential, Vote)>` (`ade_types::conway::governance`) —
> `spo_votes` stays `Hash28` (pools are always key-hash, a permanent
> non-goal). `governance.rs` `lookup_stake` now resolves a DRep voter to
> **exactly one** `DRep` stake key by mapping its discriminant
> (`StakeCredential::KeyHash` → `DRep::KeyHash`, `ScriptHash` →
> `DRep::ScriptHash`) — the prior `.or_else(…ScriptHash…)` OR-fallback is
> gone, so a key-hash voter never tallies a script-hash DRep's stake of
> equal bytes. The fingerprint writer `write_committee_vote_list` is
> **renamed `write_credential_vote_list`** and now serves both
> `committee_votes` and `drep_votes` (`spo_votes` stays `write_vote_list`
> over `Hash28`); the GREEN `ade_testkit` loader
> `parse_committee_vote_map` is **renamed `parse_credential_vote_map`**
> and parses both the committee and the DRep vote maps tag-preserving.
> **No golden drift** (the gov-action-state vote surfaces are empty in
> the committed fingerprint surfaces). The existing
> `ci/ci_check_credential_discriminant_closed.sh` was **extended** with
> two DRep-surface clauses (`drep_votes` carries `StakeCredential`, and
> `governance.rs` has no `DRep::KeyHash(…).or_else` OR-fallback) — it
> stays the **29th** script; **the CI count does not increment.**
> **`DC-LEDGER-10` is STRENGTHENED AGAIN** (`strengthened_in +=
> DREP-VOTE-FIDELITY`; +2 tests → 13; `code_locus` extended) — it is the
> same rule, not a new one; the registry total **stays 173**. Of the
> three COMMITTEE-CRED carry-forward follow-ups, **(c) is now RESOLVED**;
> the two remaining are unchanged and out of this cluster's scope: **(d)
> `EnactmentEffects.committee_changes`** is still bare `Hash28` but
> DORMANT (UpdateCommittee enactment is a no-op) — it MUST be migrated to
> `StakeCredential` before committee enactment is implemented or it would
> re-collapse on write-back; **(e)** the GREEN loader `mk_credential`
> defaults `tag != 1` to `KeyHash` (contained to `ade_testkit`, cannot
> reach the node binary); and the pre-OQ5 **(b)** Shelley unknown-cert
> zero-hash placeholder remains a WARN LOW non-goal. All three
> DREP-VOTE-FIDELITY commits carry the model-attribution trailer.

> **COMMITTEE-CRED-FIDELITY cluster note (newest thread).** This regen
> is cut at committed HEAD `2aeea16`. Since the prior (OQ5-level) regen —
> cut at the OQ5 close `676af5a` (which committed the OQ5 grounding-doc
> refresh on top of the OQ5-S2 implementation HEAD `a3ee2da`) — the
> **COMMITTEE-CRED-FIDELITY arc** has landed as three commits: `32d7a2e`
> (cluster plan + invariants, *strengthens* `DC-LEDGER-10`), `2303a60`
> (S1, discriminate committee member + vote credentials end-to-end), and
> `2aeea16` (S2, committee cross-resolve negative + CI gate, `DC-LEDGER-10`
> *strengthened*). It discharges the OQ5 per-cluster security-review
> follow-up **(a)** — committee member / vote credential discrimination
> (WARN MEDIUM) — by closing the two committee surfaces OQ5 left at the
> hash level while only `committee_hot_keys` was discriminated.
> **No new module, no new rule, no new CI script.** It is an in-place
> type change: `ConwayGovState.committee` re-keyed `Hash28` →
> `StakeCredential` (`ade_ledger::state`) and `GovActionState.committee_votes`
> re-typed `Vec<(Hash28, Vote)>` → `Vec<(StakeCredential, Vote)>`
> (`ade_types::conway::governance`) — `drep_votes` / `spo_votes` stay
> `Hash28` (they are out of scope, see follow-up (c)). `governance.rs`
> committee ratification now resolves hot-voter → hot→cold mapping → cold
> member by **full-credential equality** (the prior `.hash()` comparisons
> are gone), so a key-hash hot key never cross-resolves to a script-hash
> member of equal bytes. The fingerprint gains `write_committee_vote_list`
> (canonical, sorts committee votes by the discriminated credential's
> `Ord`) and the committee-member map writer routes through
> `write_stake_credential` (`ade_ledger::fingerprint`); the GREEN
> `ade_testkit` `snapshot_loader` committee parses preserve the tag.
> **No golden drift** (the committee states are empty in the committed
> fingerprint surfaces). The existing
> `ci/ci_check_credential_discriminant_closed.sh` was **extended** with
> two committee-surface clauses (member map is `StakeCredential`-keyed,
> `committee_votes` carries `StakeCredential`) — it stays the **29th**
> script; **the CI count does not increment.** **`DC-LEDGER-10` is
> STRENGTHENED** (`strengthened_in += COMMITTEE-CRED-FIDELITY`; +3 tests;
> `code_locus` extended) — it is the same rule, not a new one; the
> registry total **stays 173**. Per-cluster security review surfaced
> three carry-forward follow-ups, all out of this cluster's scope:
> **(c) DRep-vote discrimination** — `governance.rs` `lookup_stake` still
> does a key/script OR-fallback over identical bytes for `drep_votes`
> (the recommended next discriminant cluster); **(d)
> `EnactmentEffects.committee_changes`** is still bare `Hash28` but
> DORMANT (UpdateCommittee enactment is a no-op) — it MUST be migrated to
> `StakeCredential` before committee enactment is implemented or it would
> re-collapse on write-back; **(e)** the GREEN loader `mk_credential`
> defaults `tag != 1` to `KeyHash` (contained to `ade_testkit`, cannot
> reach the node binary). All three COMMITTEE-CRED-FIDELITY commits carry
> the model-attribution trailer.

> **OQ5-CREDENTIAL-FIDELITY cluster note (newest thread).** This regen
> is cut at committed HEAD `a3ee2da`. Since the prior (B5-level) regen —
> cut at the B5 close `f81f815` (which committed the B5 grounding-doc
> refresh on top of the B5-S5 implementation HEAD `651adc9`) — the
> **OQ5-CREDENTIAL-FIDELITY arc** has landed as four commits: `959e16c`
> (OQ-5 grounding — credential-fidelity invariants + `DC-LEDGER-10`
> *declared*), `007b0e8` (cluster plan + cluster doc), `4187330` (OQ5-S1,
> discriminated `StakeCredential` end-to-end), and `a3ee2da` (OQ5-S2,
> credential-fidelity corpus + CI gate, `DC-LEDGER-10` *enforced*). OQ5
> closes the B5-named **OQ-5 credential-discriminant collapse**: the
> credential type that B5 promoted to authority — `ConwayGovState`
> keyed on a tag-erased `Hash28`, indistinguishable for a key-hash and a
> script-hash sharing 28 bytes — is now a closed sum.
> `StakeCredential` changed from the tuple struct `StakeCredential(pub
> Hash28)` to an `enum { KeyHash(Hash28), ScriptHash(Hash28) }` with a
> discriminant-erasing `hash()` accessor reserved for boundary adapters
> only. Both era decoders' `decode_stake_credential`
> (`ade_codec::{shelley,conway}::cert`) now **preserve the key/script
> tag** (tag 0 → `KeyHash`, 1 → `ScriptHash`, any other → deterministic
> reject), retiring the prior `let (_tag, _) = …` tag-discard form.
> `ConwayGovState` was **re-keyed `Hash28` → `StakeCredential`** across
> `vote_delegations` / `committee_hot_keys` / `drep_expiry`, so a
> key-hash and a script-hash credential are distinct authoritative-state
> keys (matching cardano-node's `Credential`-keyed UMap/VState). The
> fingerprint `write_stake_credential` now **emits discriminant + hash**
> (a deliberate dual cert-state + gov-state fingerprint migration,
> `T-DET-01`); **no golden drift** results because the affected
> fingerprint surfaces are empty/credential-free in the committed
> states. The GREEN `ade_testkit` `snapshot_loader` preserves the tag on
> its gov-map / DRep-registration parses. **`DC-LEDGER-10`** is
> introduced and flipped straight to **`enforced`** (strengthens
> `T-DET-01` / `T-ENC-03`), gated by the new
> `ci/ci_check_credential_discriminant_closed.sh` (the **29th script** —
> the count was 28 at the B5 close and is 29 at this HEAD). The
> **real-chain discriminated-key agreement** vs cardano-node's
> `Credential`-keyed UMap/VState is **environment-blocked** (epoch-576
> snapshot absent) and reclassified per tier doctrine, the same
> constraint as `DC-LEDGER-08`/`DC-LEDGER-09`/`DC-TXV-06`. Per-cluster
> security review surfaced two separable follow-ups — **committee
> member/vote credential discrimination** (WARN MEDIUM) and the
> **Shelley unknown-cert zero-hash placeholder** (WARN LOW) — and three
> declared non-goals: withdrawal / required-signer / address credential,
> the `Hash28`-keyed stake-distribution snapshot, and Byron. All four
> OQ5 commits carry the model-attribution trailer.

> **B5 cluster note (newest thread).** This regen is cut at committed
> HEAD `651adc9`. Since the prior (B4-level) regen — cut at the B4 close
> `644eb03` (which committed the B4 grounding-doc refresh) — the
> **PHASE4-B5 Conway governance-certificate accumulation arc** has
> landed as six commits: `fdb6601` (B5 grounding — invariants, cluster
> plan, cluster doc; introduces `DC-LEDGER-09`), `9c8d118` (B5-S1,
> gov-cert env infrastructure — `drep_activity` + `GovCertEnv`
> fail-fast), `7a48727` (B5-S2, native Conway gov-cert apply model
> `apply_conway_gov_cert`), `d63c700` (B5-S3, apply gov-cert
> accumulation in the block path + carry `gov_state` forward),
> `06385d0` (B5-S4, gov-state accumulation corpus + CI gate enforcing
> `DC-LEDGER-09`), and `651adc9` (B5-S5, checked DRep-expiry arithmetic,
> deterministic fail-closed on overflow). B5 closes the B4
> observe-and-drop: the governance-affecting Conway certs B4 owner-tagged
> to `ConwayGovState` (vote-delegation 9/10/12/13, committee 14/15, DRep
> 16/17/18) are now **applied** — the new BLUE module
> `ade_ledger::gov_cert` (`apply_conway_gov_cert`) is a native dispatch
> over the owner-complete `ConwayCert`, total over the 18 tags, mutating
> only governance-owned fields and never the B4-owned `CertState`.
> `accumulate_tx_certs` / `process_block_certificates` now thread an
> `Option<ConwayGovState>` alongside the cert-state and **carry
> `gov_state` forward through `apply_block`** (it was nulled to `None`
> at every block apply before B5). DRep expiry is computed only from the
> new fail-fast `LedgerState::gov_cert_env()` (`current_epoch +
> drep_activity` via `checked_add`); a missing `drep_activity`
> (`ValidationEnvironmentError::MissingDRepActivityParam`) or an overflow
> (`DRepActivityOverflow`) is a deterministic structured halt, never a
> defaulted or wrapped expiry. The state fingerprint's Conway-deposit
> tag is extended from a **2-field to a 3-field** array
> (`drep_activity` added) — a deliberate `T-DET-01` fingerprint
> migration; the golden was regenerated
> (`b69422ef…71d9` → `d1803cb7…8827`) and pre-Conway / param-absent
> states stay byte-identical. `DC-LEDGER-09` is introduced and flipped
> straight to **`enforced`** (strengthens `DC-LEDGER-08`, retiring its
> "routed out-of-mutation-scope" disposition for governance certs),
> gated by the new `ci/ci_check_gov_cert_accumulation_closed.sh` (the
> 28th script). The real epoch-576 governance-state (VState) oracle is
> **environment-blocked** (UMap/ledger snapshot absent) and reclassified
> per tier doctrine, the same constraint as
> `DC-LEDGER-08`/`DC-TXV-06`/`DC-TXV-03`. Declared separable follow-ups:
> **OQ-3** (the GOVCERT committee-membership tx-validity gate) and
> **OQ-5** (the pre-existing `Hash28` credential-discriminant collapse,
> promoted to authority by B5 but not introduced here). All six B5
> commits carry the model-attribution trailer.

> **B4 cluster note (newest thread).** This regen is cut at committed
> HEAD `ee35493`. Since the prior (B3F-level) regen — cut at `193d2fc`,
> with the B3F CODEMAP/SEAMS/TRACEABILITY ripple in flight — the
> **PHASE4-B4 Conway certificate-state accumulation arc** has landed as
> five commits: `ae1300a` (B4 grounding — invariants, cluster plan,
> cluster doc, B4-S1 slice; introduces `DC-LEDGER-08`), `228415b`
> (B4-S1, owner-complete Conway cert decoder), `da30706` (B4-S2, native
> owner-tagged Conway cert apply model), `302d22c` (B4-S3/S4,
> era-dispatched fail-closed cert-state accumulation), and `ee35493`
> (B4-S5, cert-state accumulation corpus). B4 closes the
> cert-state-accumulation fail-open: the cert decoder is now
> **owner-complete** (retains every owner payload, not the deposit-only
> projection B3 left), the ledger gains a **native owner-tagged apply
> model** total over the 18 Conway tags, and `process_block_certificates`
> now **propagates** decode/apply errors instead of swallowing them
> "non-fatal during replay". `DC-LEDGER-08` is introduced and flipped to
> **`enforced`** (strengthens `DC-VAL-06`); the declared follow-up is
> **PHASE4-B5** (Conway governance-certificate accumulation authority —
> wiring the owner-tagged `ConwayGovState` effects into applied state).
> The real epoch-576 cert-state-vs-cardano-node oracle is
> **environment-blocked** (UMap snapshot absent) and reclassified per
> tier doctrine, the same constraint as `DC-TXV-06`/`DC-TXV-03`. All
> five B4 commits carry the model-attribution trailer. Whether the B3F
> grounding-doc ripple is committed alongside the B4 close is tracked in
> Anomalies.

> **B3 close + B3F follow-up note (carried forward).** The prior regen
> was cut at committed HEAD `193d2fc`. Three commits preceded it: the
> **`Close PHASE4-B3`** commit `d766eb0` (which committed the B3
> grounding-doc refresh, moved the `docs/clusters/PHASE4-B3/*` slice
> docs to `docs/clusters/completed/`, and set `DC-TXV-06` =
> `enforced`/`partial` per the close registry edit), followed by the
> **PHASE4-B3F follow-up hardening** pair `d6c1993` (B3F-S1) and
> `193d2fc` (B3F-S2). B3F discharges the named `DC-TXV-06` grep-gate
> follow-up (flips it `partial` → **`enforced`** with a standing CI
> invariant) and tightens the Conway cert decoder. The
> formerly-in-flight B3 anomalies (no close commit, committed-vs-working
> `DC-TXV-06` disagreement) were resolved at that point. Both B3F
> commits carry the model-attribution trailer.

The delta covers twenty threads of work. The newest thread — the
**ENACTMENT-COMMITTEE-FIDELITY arc** — landed on top of the
DREP-VOTE-FIDELITY close (`06f517f`); below it the **DREP-VOTE-FIDELITY
arc** sat on the COMMITTEE-CRED-FIDELITY close (`a157c92`); below that the
**COMMITTEE-CRED-FIDELITY arc** sat on the OQ5 close (`676af5a`); below that the **OQ5-CREDENTIAL-FIDELITY arc** sat on the
PHASE4-B5 close (`f81f815`); below that the **PHASE4-B5 Conway
governance-certificate accumulation arc** sat on the PHASE4-B4 close
(`644eb03`), which sat on the PHASE4-B3F
follow-up hardening (`193d2fc`), which itself sat on the **PHASE4-B3
Conway value-conservation accounting arc** above the PHASE4-B2 close
(`c1cba82`). In rough proportion of the substantive change budget:

0. **ENACTMENT-COMMITTEE-FIDELITY (committee-enactment effect credential
   discriminant fidelity) — closed (`a6b8de7`).** A 1-slice arc (plus one
   grounding/plan commit) that discharges the DREP-VOTE-FIDELITY
   security-review follow-up **(d)** without a new module, rule, or CI
   script — the last bare-`Hash28` committee-credential surface is closed
   *preventively*, before `UpdateCommittee` enactment is ever wired. **The
   cluster plan + invariants** (`5d64fee`) opened
   `docs/clusters/ENACTMENT-COMMITTEE-FIDELITY/` and declared the
   strengthening of `DC-LEDGER-10`. **S1** (`a6b8de7`) re-typed the
   dormant `EnactmentEffects.committee_changes`
   `Option<(Vec<Hash28>, Vec<(Hash28, u64)>)>` →
   `Option<(Vec<StakeCredential>, Vec<(StakeCredential, u64)>)>`
   (`ade_ledger::governance`) so the (removed, added-with-expiry)
   committee-enactment effect carries the discriminated cold committee
   credential. The field stays **DORMANT** (always `None`; `UpdateCommittee`
   enactment is still a no-op), so this is a type pin, not a behavior
   change — it prevents committee enactment, once wired, from
   re-collapsing the discriminated `ConwayGovState.committee` map on
   write-back. An inline `committee_fidelity_tests` test
   (`enactment_committee_changes_keyhash_scripthash_distinct`) asserts
   key/script members of equal bytes are distinct entries and the default
   stays `None`, and the existing
   `ci/ci_check_credential_discriminant_closed.sh` was **extended** with
   one clause (`committee_changes` carries `StakeCredential`).
   **`DC-LEDGER-10` is STRENGTHENED a THIRD time** (`strengthened_in +=
   ENACTMENT-COMMITTEE-FIDELITY`; +1 test → 14; `code_locus` extended) —
   same rule, not a new one; registry total **stays 173**, CI count
   **stays 29**. **No golden drift** (the field is `None`; no fingerprint
   surface change). DREP-VOTE follow-up **(d) is RESOLVED**; the remaining
   carry-forward follow-up (out of scope) is unchanged: **(e)** the GREEN
   loader `mk_credential` `tag != 1` → `KeyHash` default (contained to
   `ade_testkit`), plus the pre-OQ5 **(b)** Shelley unknown-cert zero-hash
   placeholder (WARN LOW non-goal). Both commits carry the
   model-attribution trailer.
1. **DREP-VOTE-FIDELITY (DRep-vote credential discriminant fidelity) —
   closed (`62c9020`).** A 2-slice arc (plus one grounding commit) that
   discharges the COMMITTEE-CRED security-review follow-up **(c)**
   without a new module, rule, or CI script — credential identity is now
   closed for the *DRep-vote* surface, the last governance vote surface
   COMMITTEE-CRED left at the hash level. **The cluster plan +
   invariants** (`ecb0b92`) opened `docs/clusters/DREP-VOTE-FIDELITY/`
   (+ `docs/planning/drep-vote-fidelity-invariants.md`) and declared the
   strengthening of `DC-LEDGER-10`. **S1** (`ba4ff37`) re-typed
   `GovActionState.drep_votes` `Vec<(Hash28, Vote)>` → `Vec<(StakeCredential,
   Vote)>` (`ade_types::conway::governance`) — `spo_votes` stays
   `Hash28` (pools are always key-hash, a permanent non-goal); `governance.rs`
   `lookup_stake` now resolves a DRep voter to **exactly one** `DRep`
   stake key by mapping its discriminant (`KeyHash` → `DRep::KeyHash`,
   `ScriptHash` → `DRep::ScriptHash`) — the prior
   `.or_else(…DRep::ScriptHash…)` OR-fallback is gone, so a key-hash
   voter cannot tally a script-hash DRep's stake of equal bytes; the
   fingerprint writer `write_committee_vote_list` is **renamed
   `write_credential_vote_list`** and now serves both `committee_votes`
   and `drep_votes` (`ade_ledger::fingerprint`, **no golden drift** — the
   gov-action-state vote surfaces are empty in the committed surfaces);
   the GREEN `ade_testkit` loader `parse_committee_vote_map` is
   **renamed `parse_credential_vote_map`** and parses both the committee
   and the DRep vote maps tag-preserving. **S2** (`62c9020`) shipped the
   negative corpus (`drep_keyhash_scripthash_do_not_cross_resolve` in
   `governance.rs`, `drep_vote_discriminant_changes_fingerprint` in
   `credential_fidelity_corpus.rs`) and **extended** the existing
   `ci/ci_check_credential_discriminant_closed.sh` with two DRep-surface
   clauses (`drep_votes` carries `StakeCredential`; no
   `DRep::KeyHash(…).or_else` OR-fallback in `governance.rs`).
   **`DC-LEDGER-10` is STRENGTHENED AGAIN** (`strengthened_in +=
   DREP-VOTE-FIDELITY`; +2 tests → 13; `code_locus` extended) — same
   rule, not a new one; registry total **stays 173**, CI count **stays
   29**. COMMITTEE-CRED follow-up **(c) is RESOLVED**; the remaining
   carry-forward follow-ups (out of scope at the time) were **(d)** the
   dormant bare-`Hash28` `EnactmentEffects.committee_changes` (must
   migrate before committee enactment — **subsequently RESOLVED by
   ENACTMENT-COMMITTEE-FIDELITY, thread 0 above**) and **(e)** the GREEN
   loader `mk_credential` `tag != 1` → `KeyHash` default (contained to
   `ade_testkit`).
2. **COMMITTEE-CRED-FIDELITY (committee member + vote credential
   discriminant fidelity) — closed (`2aeea16`).** A 2-slice arc (plus
   one grounding commit) that discharges the OQ5 security-review
   follow-up **(a)** without a new module, rule, or CI script —
   credential identity is now closed for the *committee* surfaces OQ5
   left at the hash level. **The cluster plan + invariants** (`32d7a2e`)
   opened `docs/clusters/COMMITTEE-CRED-FIDELITY/` and declared the
   strengthening of `DC-LEDGER-10`. **S1** (`2303a60`) re-keyed
   `ConwayGovState.committee` `Hash28` → `StakeCredential`
   (`ade_ledger::state`) and re-typed `GovActionState.committee_votes`
   `Vec<(Hash28, Vote)>` → `Vec<(StakeCredential, Vote)>`
   (`ade_types::conway::governance`) — `drep_votes` / `spo_votes` stay
   `Hash28`; `governance.rs` committee ratification now resolves
   hot-voter / hot→cold / cold-member by **full-credential equality**
   (the `.hash()` comparisons are gone), so a key-hash hot key cannot
   cross-resolve to a script-hash member of equal bytes; the fingerprint
   gains `write_committee_vote_list` (canonical, sorts committee votes by
   the discriminated credential's `Ord`) and the committee-member writer
   routes through `write_stake_credential` (`ade_ledger::fingerprint`,
   **no golden drift** — committee states are empty in the committed
   surfaces); the GREEN `ade_testkit` `snapshot_loader` committee parses
   preserve the tag. **S2** (`2aeea16`) shipped the negative corpus
   (`committee_keyhash_scripthash_do_not_cross_resolve` in `governance.rs`,
   `committee_keyhash_scripthash_same_bytes_distinct` and
   `committee_discriminant_changes_fingerprint` in
   `credential_fidelity_corpus.rs`) and **extended** the existing
   `ci/ci_check_credential_discriminant_closed.sh` with two
   committee-surface clauses. **`DC-LEDGER-10` is STRENGTHENED**
   (`strengthened_in += COMMITTEE-CRED-FIDELITY`; +3 tests; `code_locus`
   extended) — same rule, not a new one; registry total **stays 173**,
   CI count **stays 29**. Declared carry-forward follow-ups (out of
   scope): **(c)** DRep-vote discrimination (the natural next
   discriminant cluster), **(d)** the dormant bare-`Hash28`
   `EnactmentEffects.committee_changes` (must migrate before committee
   enactment), **(e)** the GREEN loader `mk_credential` `tag != 1` →
   `KeyHash` default (contained to `ade_testkit`).
3. **OQ5-CREDENTIAL-FIDELITY (credential key/script discriminant
   fidelity) — closed (`a3ee2da`).** A 2-slice arc (plus two grounding
   commits) that closes the B5-named **OQ-5** collapse: credential
   identity is now a closed sum, not a tag-erased `Hash28`. **OQ-5
   grounding** (`959e16c`) declared the credential-fidelity invariants
   and registry rule `DC-LEDGER-10`; **the cluster plan + cluster doc**
   (`007b0e8`) opened `docs/clusters/OQ5-CREDENTIAL-FIDELITY/`. **OQ5-S1**
   (`4187330`) made `StakeCredential` discriminated **end-to-end** — no
   new module, an in-place type change: `StakeCredential(pub Hash28)` →
   `enum { KeyHash(Hash28), ScriptHash(Hash28) }` with a
   discriminant-erasing `hash()` accessor reserved for genuine bare-byte
   boundary adapters (`ade_types::shelley::cert`); both era decoders'
   `decode_stake_credential` now **preserve the tag** (0 → `KeyHash`, 1 →
   `ScriptHash`, other → deterministic reject — the prior `let (_tag, _)
   = …` tag-discard form is gone) (`ade_codec::{shelley,conway}::cert`);
   `ConwayGovState` was **re-keyed `Hash28` → `StakeCredential`** across
   `vote_delegations` / `committee_hot_keys` / `drep_expiry`
   (`ade_ledger::state`, with `gov_cert.rs` / `governance.rs` /
   `cert_classify.rs` / `rules.rs` following the key-type change and the
   `cred.hash()` boundary adapter retained only for the `Hash28`-keyed
   stake snapshot); the fingerprint `write_stake_credential` now **emits
   discriminant + hash** (a deliberate dual cert-state + gov-state
   fingerprint migration, `T-DET-01` — the gov-map writers use it, the
   stake-snapshot writer stays `write_hash28`), with **no golden drift**
   because the affected states are empty/credential-free; and the GREEN
   `ade_testkit::harness::snapshot_loader` preserves the tag on its
   gov-map / DRep-registration parses. **OQ5-S2** (`a3ee2da`) shipped the
   credential-fidelity corpus (`credential_fidelity_corpus.rs`,
   `corpus/credential_fidelity/README.md`,
   `shelley_credential_discriminant.rs`, extended
   `conway_cert_decode_complete.rs`) and the standing CI gate
   `ci/ci_check_credential_discriminant_closed.sh` enforcing
   `DC-LEDGER-10`. `DC-LEDGER-10` is **`enforced`** (strengthens
   `T-DET-01` / `T-ENC-03`); the real-chain discriminated-key agreement
   vs cardano-node's `Credential`-keyed UMap/VState is
   **environment-blocked** and reclassified per tier doctrine. Per-cluster
   security review surfaced two separable follow-ups — committee
   member/vote credential discrimination (WARN MEDIUM) and the Shelley
   unknown-cert zero-hash placeholder (WARN LOW); declared non-goals:
   withdrawal / required-signer / address credential, the
   stake-distribution snapshot, and Byron.
4. **Phase 4 cluster B5 (Conway governance-cert accumulation) — closed
   (`651adc9`).** A 5-slice arc (plus a grounding commit) that closes
   the B4 observe-and-drop: governance-affecting Conway certs are now
   **applied** to `ConwayGovState`, not just owner-tagged and dropped.
   **B5 grounding** (`fdb6601`) opened the cluster (invariants, cluster
   plan, cluster doc) and introduced registry rule `DC-LEDGER-09`.
   **B5-S1** (`9c8d118`) added the gov-cert env infrastructure: the new
   Conway-only `ConwayOnlyDepositParams.drep_activity` field
   (`pparams.rs`), the `GovCertEnv` struct + fail-fast
   `LedgerState::gov_cert_env()` (`state.rs`), and the two new
   `ValidationEnvironmentError::{MissingDRepActivityParam,
   DRepActivityOverflow}` variants (`error.rs`); the state fingerprint's
   Conway-deposit tag was extended **2→3 fields** to fold `drep_activity`
   (`fingerprint.rs`, a deliberate `T-DET-01` migration with a
   regenerated golden, byte-identical for pre-Conway / param-absent
   states). **B5-S2** (`7a48727`) added the **native gov-cert apply
   model** in the new BLUE module `ade_ledger::gov_cert`:
   `apply_conway_gov_cert(gov_state, cert, env)` is a pure dispatch over
   the owner-complete `ConwayCert`, total over the 18 tags — vote
   delegation (9/10/12/13) → `vote_delegations`, committee auth/resign
   (14/15) → `committee_hot_keys`, DRep reg/update (16/18) → env-driven
   `drep_expiry`, DRep unreg (17) → remove — mutating **only**
   governance-owned fields, never the B4-owned `CertState`, and never
   double-applying the delegation/pool half of composite certs.
   **B5-S3** (`d63c700`) wired it into the block path:
   `accumulate_tx_certs` / `process_block_certificates` now thread an
   `Option<ConwayGovState>` alongside the cert-state, apply the gov half
   when governance is tracked, **remove the B4 observe-and-drop
   comment**, and **carry `gov_state` forward through `apply_block`**
   (`rules.rs` — `gov_state` was nulled to `None` at every block apply
   before B5). **B5-S4** (`06385d0`) shipped the corpus
   (`gov_state_corpus.rs`, `corpus/gov_state/README.md`: synthetic
   positive accumulation + replay byte-identical + adversarial
   no-false-accept) and the standing CI gate
   `ci/ci_check_gov_cert_accumulation_closed.sh` enforcing
   `DC-LEDGER-09`. **B5-S5** (`651adc9`) hardened the DRep-expiry
   arithmetic to `checked_add` (deterministic fail-closed
   `DRepActivityOverflow` on the absurd-`drep_activity` edge, never a
   silent `u64` wrap). `DC-LEDGER-09` is **`enforced`** (strengthens
   `DC-LEDGER-08`); the real epoch-576 VState oracle is
   **environment-blocked** and reclassified per tier doctrine. Declared
   separable follow-ups: **OQ-3** (GOVCERT committee-membership
   tx-validity gate) and **OQ-5** (the pre-existing `Hash28`
   credential-discriminant collapse, promoted to authority by B5).
5. **Phase 4 cluster B4 (Conway cert-state accumulation, fail-closed) —
   closed (`ee35493`).** A 5-slice arc that closes the cert-state
   accumulation fail-open. **B4 grounding** (`ae1300a`) opened the
   cluster (invariants, cluster plan, cluster doc, B4-S1 slice) and
   introduced registry rule `DC-LEDGER-08`. **B4-S1** (`228415b`) made
   the Conway cert decoder **owner-complete**: `ConwayCert` (in
   `ade_types::conway::cert`) and `decode_conway_certs` (in
   `ade_codec::conway::cert`) now retain **all** owner payloads —
   stake/DRep/committee credentials, pool id, full pool parameters
   (incl. `pool_owners`), DRep delegation targets — where B3 kept only
   the deposit/refund projection; the shared
   `read_pool_registration_cert` (`ade_codec::shelley::cert`) now
   retains `pool_owners` (new `PoolRegistrationCert.owners` field), a
   new `decode_drep` reads the DRep target, and a new `DRep` enum lands
   in `ade_types::conway::cert`. Fields no owner stores (cert anchors,
   pool relays/metadata) are still structurally consumed and dropped;
   unknown tags still reject; removed tags 5/6 still decode to
   `RemovedInConway`. **B4-S2** (`da30706`) added a **native
   owner-tagged apply model** in `ade_ledger::delegation`:
   `conway_cert_action` + `apply_conway_cert` with
   `ConwayCertAction`/`ConwayCertOutcome` and the
   `GovernanceOwner`/`GovernanceCertEffect`/`OwnerTaggedEffect` owner
   tags, **total over all 18 Conway tags** — governance certs
   (vote-deleg / committee auth+resign / DRep reg/unreg/update) are
   owner-tagged to `ConwayGovState` and **routed out of B4 mutation
   scope** (observed, not applied — deferred to the declared
   **PHASE4-B5**); composite certs (tags 10/12/13) carry both a B4-owned
   cert-state mutation and an owner-tagged governance effect, both
   represented (`CertStateAndGovernance`); no Conway cert is flattened
   to `Neutral` for lack of an owner. **B4-S3/S4** (`302d22c`) made the
   block-level accumulation **era-dispatched and fail-closed**:
   `process_block_certificates` now calls a new `accumulate_tx_certs`
   (`ade_ledger::rules`) that **removes the `_era` discard** (explicit
   `CardanoEra` dispatch — Conway via `decode_conway_certs` +
   `apply_conway_cert`, Shelley..Babbage via the Shelley path) and
   **removes the two "non-fatal during replay" swallows**; decode and
   apply errors now propagate as structured `LedgerError` and halt the
   block transition. `ci/ci_check_forbidden_patterns.sh` was extended to
   fail if either the `non-fatal during replay` rationale or an
   `Err(_) =>` swallow arm in `accumulate_tx_certs` is reintroduced.
   **B4-S5** (`ee35493`) shipped the corpus: synthetic positive
   accumulation + replay byte-identical + adversarial no-false-accept
   (`cert_state_corpus.rs`, `conway_cert_decode_complete.rs`,
   `corpus/cert_state/README.md`). `DC-LEDGER-08` is **`enforced`**
   (strengthens `DC-VAL-06`); the real epoch-576 cert-state oracle is
   **environment-blocked** (UMap snapshot absent) and reclassified per
   tier doctrine.
6. **Phase 4 cluster B3F (follow-up hardening) — committed
   (`193d2fc`).** A 2-slice follow-up that closes the two named B3
   carry-overs without a new module or crate. **B3F-S1** (`d6c1993`)
   adds the CI grep-gate `ci/ci_check_conway_cert_classification_closed.sh`
   — it fails if the Conway cert-classification closure regresses: an
   open-tail `Other`/`Unknown` variant or `#[non_exhaustive]` on
   `ConwayCert` / `CertDisposition` / `DepositEffect` / `CoinSource`, a
   catch-all decoder arm constructing a `ConwayCert`, or a `_ =>`
   wildcard in `classify`. This **flips `DC-TXV-06` `partial` →
   `enforced`** (the rule now has a standing CI invariant, not only
   exhaustive-match + tests). **B3F-S2** (`193d2fc`) hardens
   `ade_codec::conway::cert::decode_conway_certs`: it now **rejects
   trailing bytes** after the cert array (`CodecError::TrailingBytes`,
   parity with `decode_withdrawals` — the cert field is an exact CBOR
   item) and **bounds preallocation by remaining input**
   (`Vec::with_capacity((n).min(data.len()))`, no behavioral change for
   valid input, defangs a crafted huge definite-array count). It also
   consumes the indefinite-array break byte. **Strengthens `DC-VAL-06`**
   (`strengthened_in += PHASE4-B3F`); +2 tests
   (`trailing_bytes_after_cert_array_rejected`,
   `huge_array_count_rejects_without_overallocating`).
7. **Phase 4 cluster B3 (Conway value-conservation accounting) —
   closed (`d766eb0`).** Three implementation/planning commits plus the
   close: the planning commit `3aebbe5` (invariants, cluster/slice plan,
   registry rules `DC-TXV-06`/`DC-TXV-07`), the implementation `978c222`
   (full Conway value-conservation accounting — **removes the
   cert/withdrawal early-out** that B2-S4 left as a conservative
   deferral), the corpora commit `7784bf8` (real epoch-576 positive +
   synthetic + adversarial conservation corpora, no false accept), and
   the close commit `d766eb0` (grounding-doc refresh + slice-doc
   archival). It **closes the deferred value-conservation gap** named at
   the B2 close: the full Conway preservation-of-value equation —
   `Σ(inputs) + Σ(withdrawals) + refunded_deposits == Σ(outputs) + fee
   + donation + new_deposits` (i128, no float) — is now enforced for
   **cert- and withdrawal-bearing txs**, and the release-blocking
   false-accept early-out is gone. New BLUE surfaces: the closed Conway
   cert decoder `ade_codec::conway::cert` (grammar tags 0..18,
   `CodecError::UnknownCertTag`), the withdrawals decoder
   `ade_codec::conway::withdrawals` (`RewardAccount`, i128 sum,
   `CodecError::DuplicateMapKey`), and the closed cert-deposit
   classifier `ade_ledger::cert_classify`. Two new registry rules:
   `DC-TXV-06` (closed cert-deposit classification — flipped to
   **`enforced`** by the B3F grep-gate, see thread 0) and `DC-TXV-07`
   (canonical deposit-parameter authority — `enforced` via the new
   `ci_check_deposit_param_authority.sh`). Strengthens
   `T-CONSERV-01`/`CN-LEDGER-07`, `DC-VAL-06`, and `DC-TXV-03`.
8. **Phase 4 cluster B2 (tx validity agreement) — closed (`c1cba82`).**
   A 5-slice arc (B2-S1 → B2-S5) shipped as `feat(tx-validity):`
   commits, opened by the planning trio `b79f632` (invariant sketch +
   `DC-TXV` family), `b32fef3` (cluster/slice plan), `7263699`
   (cluster doc), and closed by `c1cba82` (close commit +
   grounding-doc refresh). It is the **per-transaction** counterpart to
   B1's per-block verdict: it introduces the new BLUE
   `ade_ledger::tx_validity` submodule (vkey-witness + required-signer
   closure, phase-1 composition, closed verdict taxonomy, canonical
   verdict surface) and the BLUE/GREEN `ade_ledger::mempool` admission
   gate. **All 5 CEs closed** (CE-B2-1 vkey-witness/required-signer
   closure, CE-B2-2 tx_validity composition + verdict taxonomy, CE-B2-3
   positive corpus replay with 103/103 real Conway txs Valid, CE-B2-4
   adversarial corpus no-false-accept, CE-B2-5 mempool admission gate).
   The arc added 5 `DC-TXV-*` rules, flipped the two `DC-MEM-*` rules to
   `enforced`, and — critically — **found and fixed a real
   value-conservation fail-open** (`617139f`, B2-S4) whose deferred
   deposit/withdrawal residual B3 has now closed (thread 1).
9. **Conway value-conservation: the B2-S4 fail-open and its B3
   completion.** B2-S4 (`617139f`) added
   `ade_ledger::conway::check_conway_coin_conservation`, but with a
   **deliberate early-out** — it returned before checking value for any
   tx carrying certs or withdrawals (so it could never false-*reject*,
   but the deposit/withdrawal residual was uncaught). B3-S4 (`978c222`)
   **removes that early-out** and replaces the deposit-free form with
   the full equation, sourcing every deposit/refund/withdrawal term from
   canonical ledger state (`ProtocolParameters.{key_deposit,pool_deposit}`
   + `LedgerState.conway_deposit_params`), classified by the closed
   `ade_ledger::cert_classify::classify` over the closed `ConwayCert`
   grammar. The named tx-validity-completeness follow-up from the B2
   regen is therefore **discharged** for Conway cert/withdrawal txs.
10. **Phase 4 cluster B1 (full block validity agreement) — closed
   (`993f363`).** The 7-slice arc (B1-S1 → B1-S7) composes the N-A wire
   layer, the N-B consensus header authority, and the `ade_ledger` body
   authority into a single block verdict. Introduced the BLUE
   `ade_ledger::block_validity` submodule, the BLUE `consensus_view`
   `LedgerView` projection, the RED `consensus_input_extract` snapshot
   tail-scan, the GREEN `validity` testkit harness, the `kes_check`
   fail-closed header crypto guard in `ade_core::consensus`, and the
   `corpus/validity/` positive + (derived) adversarial corpora. **All 5
   CEs closed.** Opened the `DC-VAL-*` registry family (6 rules) and
   added the new crate dependency edge `ade_ledger -> ade_core`. Closed
   by `993f363`; `3552bc2` synced `Cargo.lock` for the new edges and
   `e0af99d` gitignored the multi-GB ledger-state dumps.
11. **Phase 4 cluster N-A (network mini-protocols) — closed.** 10
   slices (S-A1 → S-A10, with S-A8b/S-A8c rework). Introduced the new
   BLUE workspace crate `ade_network` with 11 mini-protocol codecs, 8
   state machines, the Ouroboros mux frame codec, a RED `session`
   substrate. Closed CE-N-A-1 → CE-N-A-5 against pinned cardano-node
   11.0.1, including a real-capture corpus at `corpus/network/{n2n,n2c}/`.
   Three wire-form codec bugs surfaced by real interop were fixed in
   flight, plus an LSQ Acquire/AcquireNoPoint split, a
   LocalTxSubmission/N2N TxSubmission2 inner-tx HFC envelope fix, and
   DoS-hardening on `Vec::with_capacity` in eight codecs.
12. **Phase 4 cluster N-B (consensus runtime) — closed (`a0c73e1`).**
   10 slices (S-B1 → S-B10), opened by `d9f0426` (invariant sketch v2 +
   8 `DC-CONS-*` rules). Built out the BLUE `ade_core::consensus` module
   (15+ source files: closed `PraosChainDepState`, `EraSchedule`,
   fork-choice, rollback, nonce/op-cert/leader-schedule/VRF/header
   validation, `CandidateFragment`, structured event/error taxonomies).
   GREEN `ade_runtime::consensus` shipped the chain-selector
   orchestrator, candidate-fragment builder, and a RED genesis parser.
   New replay corpora under `corpus/consensus/`. All 6 CEs closed.
13. **CE-N-B-6 follow-mode bridge** — `807bcb6` retargeted the N-B
   live-interop pin to cardano-node 11.0.1, then `e5f1f64` added the RED
   `ade_core_interop::follow` bridge plus live preprod tip-agreement
   evidence. Follow mode runs BLUE fork-choice + rollback only — it
   trusts the already-validated peer for header/VRF/leader/nonce/KES, so
   it carries no authoritative validation decision.
14. **Phase 4 cluster N-D (ChainDB persistence) — closed (`436b1d7`).**
   Slices S-33 → S-37. CE-N-D-1 closure evidence (1000/1000 stress-kill
   iterations).
15. **Phase 2C close-out / CE-73 reclassification** — single commit
   (`9b15378`) splitting CE-73 into a Tier-2 semantic gate (enforced via
   `ci_check_hfc_translation.sh`) and an explicit Tier-4 bytes non-goal.
16. **IDD canonicalization** — `chore(idd)` commits that make the repo
    legible to the global IDD slash commands: `.idd-config.json`,
    registry rename (`constitution_registry.toml` →
    `docs/ade-invariant-registry.toml`), cluster N-D moved into
    `docs/clusters/PHASE4-N-D/`, repo-local commit-msg trailer hook.
17. **Grounding-doc generation + ripple** — `a87c3a3` produced the
    first cuts of CODEMAP/SEAMS/HEAD_DELTAS/TRACEABILITY; `f0b0fd6`,
    `a2c7ac8`, `744ef34`, the B2-close refresh in `c1cba82`, and the
    B3-close refresh in `d766eb0` refreshed subsets after the
    BLUE-scope, N-D, N-A, B2, and B3 closures respectively. **No
    grounding-doc refresh commit landed for N-B, the follow-bridge, or
    B1**, and the CODEMAP/SEAMS/TRACEABILITY refresh for the B3F
    follow-up is the in-flight working tree (see Anomalies).
18. **BLUE-list drift closure** — `5b70bee` extended six CI scripts to
    the full 6-crate BLUE scope; `c8fa37f` refreshed CODEMAP and
    TRACEABILITY to remove 14 `_(scope gap)_` markers across 13 rules.
19. **Corpus relayout** — `corpus/snapshots/*` and the
    `reward_provenance/*_registered_creds.txt` files were removed (they
    carried credential material that does not belong in a public repo);
    12 boundary-block sets were re-extracted at exact era-boundary
    slots; the consensus corpus (`corpus/consensus/*`, N-B), the validity
    corpus (`corpus/validity/*`, B1), the tx-validity adversarial
    README (`corpus/tx_validity/adversarial/`, B2), and the B3
    conservation corpora (`corpus/conway_certs/*`,
    `corpus/validity/conway_epoch576/{resolution_set.txt,resolved_inputs.json}`,
    `corpus/tools/extract_conway_resolved_inputs.md`) were added.

---

## 1. Commit Log

| Hash | Type | Summary |
|------|------|---------|
| `a6b8de7` | feat | feat(ledger): discriminate EnactmentEffects.committee_changes (ENACTMENT-COMMITTEE-FIDELITY-S1) |
| `5d64fee` | docs | docs(ledger): ENACTMENT-COMMITTEE-FIDELITY plan (strengthens DC-LEDGER-10) |
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
| `ade_codec::conway::cert` (new file in an existing BLUE crate) | BLUE | **Conway-complete certificate decoder** with a *closed* wire grammar. `decode_conway_certs` decodes the full Conway certificate array over tags `0..18`; tags `5`/`6` (the legacy MIR/genesis-delegation certs removed in Conway) are not valid, and any unrecognized tag is a deterministic `CodecError::UnknownCertTag { tag, offset }` reject — never a silent skip. **PHASE4-B3F-S2 hardened it**: trailing bytes after the cert array are now rejected (`CodecError::TrailingBytes`, parity with `decode_withdrawals`), the indefinite-array break byte is consumed, and definite-array preallocation is bounded by `data.len()` (no over-allocation on a crafted huge count). Decode is replay-deterministic over input bytes. | `conway/cert.rs` (`decode_conway_certs`) | PHASE4-B3 / B3-S1, B3-S2; **strictness PHASE4-B3F / B3F-S2** |
| `ade_codec::conway::withdrawals` (new file in an existing BLUE crate) | BLUE | Conway withdrawals-map decoder. Decodes the `{ RewardAccount => Coin }` map into a canonical ordered form, summing to an `i128` consumed-side term for the value-conservation equation, and rejecting a repeated key with `CodecError::DuplicateMapKey { offset }` (a duplicate-key map is malformed wire, not last-write-wins). | `conway/withdrawals.rs` | PHASE4-B3 / B3-S3 |
| `ade_ledger::cert_classify` (new file in an existing BLUE crate) | BLUE | **Closed cert-deposit classification** — the bridge between the decoded `ConwayCert` grammar and the value-conservation equation. `classify(state, cert)` is a total, era-versioned map resolving every cert variant to exactly one `CertDisposition` over `DepositEffect` (`new_deposit` / `refund` / `neutral` / `explicit_reject`) with the coin sourced via a closed `CoinSource` (from the cert for the Conway explicit-deposit variants, tags 7/8; from the protocol parameter for the legacy variants, tags 0/1). State-dependent cases that cannot be accounted reject with `UnsupportedStateDependentDeposit` rather than guessing. Incomplete classification is a forbidden false-accept path. **PHASE4-B3F-S1** added the CI grep-gate guarding `classify`'s exhaustiveness (no `_ =>` wildcard). | `cert_classify.rs` (`classify`) | PHASE4-B3 / B3-S2; **closure gate PHASE4-B3F / B3F-S1** |
| `ade_ledger::gov_cert` (new file in an existing BLUE crate) | BLUE | **Native Conway governance-certificate accumulation** — the B5 application of the governance effects B4 owner-tagged but routed out of mutation scope. `apply_conway_gov_cert(gov_state, cert, env)` is a pure, total dispatch over the owner-complete `ConwayCert` (18 tags): vote-delegation (9/10/12/13) sets `vote_delegations[cred] = drep`; committee hot-key auth (14) sets `committee_hot_keys[hot] = cold`; committee cold resignation (15) clears that member's hot authorizations; DRep registration/update (16/18) sets `drep_expiry[cred] = current_epoch + drep_activity` via `checked_add` (fail-closed `DRepActivityOverflow`); DRep unregistration (17) removes the expiry; CertState-only and removed tags leave `gov_state` unchanged. It mutates **only** governance-owned fields of `ConwayGovState`, never the B4-owned `CertState`, so composite certs (10/12/13) are not double-applied. The DRep-expiry env (`GovCertEnv`) is required only by tags 16/18; an absent `drep_activity` is a structured fail-fast (`MissingDRepActivityParam`), never a defaulted expiry. | `gov_cert.rs` (`apply_conway_gov_cert`) | PHASE4-B5 / B5-S2 (apply model); B5-S1 (env), B5-S3 (block-path wiring), B5-S5 (checked arithmetic) |
| `ade_ledger::tx_validity` (new submodule of an existing BLUE crate) | BLUE | **Per-transaction verdict authority** — the per-tx counterpart to `block_validity`. Closed `TxValidityVerdict` (`Valid { tx_id, applied: LedgerState }` / `Invalid { class, error }`), closed `TxRejectClass` (Phase1Invalid / WitnessInvalid / MissingRequiredSigner / Phase2Invalid / MalformedField) with a *total* `class()` mapping, and closed `TxValidityError`. `required_signers` enumerates, over a CLOSED era-versioned `SignerSource`, every `Hash28` key hash a Conway tx must witness (grounded in `getConwayWitsVKeyNeeded` / `getVKeyWitnessConwayTxCert`); `witness::verify_required_witnesses` checks each required key has a fail-closed Ed25519 witness over the preserved body hash. `tx_phase_one` composes the witness closure with the state-backed checks; `tx_validity` is the pure `(LedgerState, tx_cbor) → verdict` transition. Canonical `TxVerdictSurface` encode/decode for the replay/comparison surface. | `mod.rs` (re-exports), `verdict.rs` (`TxValidityVerdict`, `TxRejectClass`, `TxValidityError`), `required_signers.rs` (`SignerSource`, `required_signers`, `tx_derived_required_signers`, `ResolvedInputs`/`ResolvedOutput`), `witness.rs` (`verify_required_witnesses`, `WitnessClosureError`, `WitnessField`), `phase1.rs` (`tx_phase_one`, `decode_tx`, `DecodedTx`), `transition.rs` (`tx_validity`, `TxValidityOutcome`), `encoding.rs` (`encode_tx_verdict_surface`, `decode_tx_verdict_surface`, `TxVerdictSurface`) | PHASE4-B2 / B2-S1 (witness/required-signer closure), B2-S2 (composition + taxonomy) |
| `ade_ledger::mempool` (new submodule of an existing BLUE crate) | BLUE (`admit`) / GREEN (`policy`) | Mempool admission gate, two layers strictly separated by TCB color. `admit` (BLUE, Tier-1) admits a tx iff `tx_validity` is `Valid` against the mempool's accumulating `MempoolState`; no false accept, and on Invalid the mempool is unchanged. `policy` (GREEN, Tier-5) does deterministic eviction/ordering over already-admitted tx ids; it never calls `tx_validity` and provably cannot alter an admit verdict (it reads only the admitted-id list). | `mod.rs` (re-exports), `admit.rs` (`admit`, `AdmitOutcome`, `MempoolState`), `policy.rs` (`order`, `OrderPolicy`) | PHASE4-B2 / B2-S5 |
| `ade_testkit::tx_validity` (new submodule of an existing crate) | GREEN | Test-only tx-validity harness. Extracts every on-wire Conway tx from the committed Conway-576 corpus blocks and drives BLUE `tx_validity` over each (positive corpus, 103/103 Valid); supplies synthetic valid txs at controlled UTxO (`valid_synthetic.rs`); derives adversarial txs via deterministic mutators — family A (witness mutations W1–W4 on real corpus txs at `track_utxo=false`) and family B (synthetic value/input/witness mutations S1–S4 at `track_utxo=true`) — plus a judge (`adversarial.rs`). B3 extended `valid_synthetic.rs` / `adversarial.rs` for the conservation corpora and added the resolved-input intra-corpus resolver to the harness snapshot loader. Non-authoritative. | `tx_validity/mod.rs`, `tx_validity/extract.rs`, `tx_validity/valid_synthetic.rs`, `tx_validity/adversarial.rs`; `harness/snapshot_loader.rs` (B3 resolution helper); B3 example bins `examples/{dump_b3_cert_tags,dump_b3_resolution_set,resolve_b3_intra_corpus}.rs` | PHASE4-B2 / B2-S3, B2-S4; B3 conservation-corpus extensions |
| `ade_ledger::block_validity` (new submodule of an existing BLUE crate) | BLUE | Full-block verdict authority: closed `BlockValidityVerdict`, closed `BlockValidityError` / `BlockRejectClass`, fail-closed `FieldKind` / `FieldError` taxonomy, and the `block_validity(...)` transition composing the N-B header authority with the body authority. Header validated before body (fail-fast). Canonical `VerdictSurface` for replay/comparison. | `mod.rs`, `verdict.rs`, `transition.rs`, `header_input.rs`, `encoding.rs` | PHASE4-B1 / B1-S3 (taxonomy), B1-S4 (composition) |
| `ade_ledger::consensus_view` (new file in an existing BLUE crate) | BLUE | Production `LedgerView` projection. `PoolDistrView` projects a `LedgerState`'s pool-distribution into exactly the four leadership-relevant facts BLUE consensus consumes through the `ade_core::consensus::LedgerView` boundary — total active stake, per-pool active stake, per-pool registered VRF keyhash, active-slots coefficient — and nothing else. | `consensus_view.rs` (`PoolDistrView`) | PHASE4-B1 / B1-S2 |
| `ade_ledger::consensus_input_extract` (new file in an existing BLUE crate) | RED | Tail-scan of a snapshot `state` CBOR for the five `PraosState` nonces. RED because it parses an external dump format; the scan is pure over input bytes and fail-closed (requires exactly five non-neutral nonces). | `consensus_input_extract.rs` | PHASE4-B1 / B1-S1 |
| `ade_core::consensus::kes_check` (new file in an existing BLUE crate) | BLUE | Fail-closed wiring of `ade_crypto::kes` into Praos header validation. `expect_size` rejects wrong-length crypto fields rather than skipping them (DC-VAL-06 fail-closed pattern). Adds the single-VRF + KES header verification path exercised by 14/14 real Conway headers. | `kes_check.rs` | PHASE4-B1 / B1-S5 |
| `ade_testkit::validity` (new submodule of an existing crate) | GREEN | Test-only block-validity harness: positive Conway-576 corpus replay over `block_validity`, corpus-backed `LedgerView`, deterministic adversarial mutators M1–M6. Non-authoritative. | `validity/mod.rs`, `validity/corpus.rs`, `validity/ledger_view.rs`, `validity/replay.rs`, `validity/adversarial.rs` | PHASE4-B1 / B1-S6, B1-S7 |
| `ade_core_interop::follow` (new file in an existing RED crate) | RED | Follow-mode bridge between a peer's ChainSync stream and BLUE fork-choice. Runs BLUE `select_best_chain` + `apply_rollback` ONLY; calls no header/VRF/leader/nonce/KES validation. Asserts tip-selection agreement with an already-validated peer. Carries no authoritative decision. | `follow.rs`, `tests/follow_offline_replay.rs` | CE-N-B-6 follow-bridge (`e5f1f64`) |
| `ade_network` (new workspace crate) | BLUE-majority (per-submodule scoped in `.idd-config.json` `core_paths`) | Ouroboros mini-protocol authority: 11 closed-grammar codecs, 8 pure transition state machines, Ouroboros mux frame codec, RED session/transport substrate. Wire bytes are Tier 1. Sync-only in BLUE submodules (DC-CORE-01); tokio confined to `mux::transport`. | `codec/` (11 codecs); `handshake/`; `chain_sync/`, `block_fetch/`, `tx_submission/`, `keep_alive/`, `peer_sharing/`; `n2c/`; `mux/frame.rs` (BLUE), `mux/transport.rs` (RED); `session/` (RED); RED capture binaries | PHASE4-N-A / S-A1 → S-A10 |
| `ade_core::consensus` (new submodule of an existing BLUE crate) | BLUE | Praos consensus authority: closed `PraosChainDepState`, era-aware slot/time translation, header validation, nonce evolution, op-cert monotonicity, leader schedule, fork choice, rollback. Closed `ChainEvent` / `ChainSelectionReject` taxonomies; flat-data errors. No async, no ChainDb, no floats. | `mod.rs`, `era_schedule.rs`, `header_validate.rs`, `vrf_cert.rs`, `nonce.rs`, `op_cert.rs`, `leader_schedule.rs`, `fork_choice.rs`, `rollback.rs`, `kes_check.rs` (B1), `praos_state.rs`, `candidate.rs`, `events.rs`, `errors.rs`, `encoding.rs`, `ledger_view.rs`, `header_summary.rs` | PHASE4-N-B / S-B1 → S-B9 (`kes_check` from B1) |
| `ade_runtime::consensus` (new submodule of an existing RED crate) | GREEN/RED mix | Imperative-shell composition for consensus: stream-driven orchestrator (GREEN), candidate-fragment builder, RED genesis parser (genesis JSON → BLUE `EraSchedule`). | `mod.rs`, `candidate_fragment.rs`, `chain_selector.rs`, `genesis_parser.rs` | PHASE4-N-B / S-B8, S-B10 |
| `ade_core_interop` (new workspace crate) | RED | Live cardano-node interop driver for CE-N-B-6. No authoritative decisions; readiness probe, `live_consensus_session` binary, and (B1-era) the `follow` bridge. CI does not run it by default (`#[ignore]`-gated / offline-replay only). | `src/lib.rs`, `src/follow.rs`, `src/bin/live_consensus_session.rs`, `tests/` | PHASE4-N-B / S-B10; follow-bridge `e5f1f64` |
| `ade_testkit::consensus` (new submodule of an existing crate) | GREEN | Test-only harness for consensus replay corpora (`corpus/consensus/*`): JSON fixture loader, `LedgerView` stub, `consensus_stream_replay` driver. | `consensus/mod.rs`, `consensus/corpus.rs`, `consensus/ledger_view_stub.rs`, `consensus/stream_replay.rs` | PHASE4-N-B / S-B1, S-B6, S-B8 → S-B10 |
| `ade_runtime::chaindb` | RED | Block-store abstraction and impls. Trait surface Tier 1; backing-store choice and on-disk layout Tier 5. | `mod.rs`, `types.rs`, `error.rs`, `in_memory.rs`, `persistent.rs` (redb), `contract.rs`, `snapshot_contract.rs`, `crash_safety.rs` | PHASE4-N-D / S-33 → S-35 |
| `ade_runtime::recovery` | RED | Composes ChainDb + SnapshotStore into a generic recovery primitive: load latest snapshot, replay forward to tip. | `recovery.rs` | PHASE4-N-D / S-36 |
| `ade_runtime` bin `chaindb_kill_target` | RED | Kill-target child process for the 1,000-kill-9 durability stress harness. | `src/bin/chaindb_kill_target.rs`, `tests/stress_kill_harness.rs` | PHASE4-N-D / S-37 |

Workspace-level membership grew by **two crates** across the full
delta: `ade_network` (PHASE4-N-A) and `ade_core_interop` (PHASE4-N-B).
Both are RED-or-mixed. **None of PHASE4-B3, B3F, B4, B5, OQ5, or COMMITTEE-CRED-FIDELITY added a
new crate** — B3's new surfaces are submodules/files of the existing BLUE
crates `ade_codec`, `ade_ledger`, and `ade_types`; B3F added **no new
module at all** (a CI script + a strictness change to the existing
`ade_codec::conway::cert` decoder); **PHASE4-B4 added no new module
either** (it enriched the existing B3 BLUE surfaces in place — see
below); and **PHASE4-B5 added one new BLUE module — the file
`ade_ledger::gov_cert` (`apply_conway_gov_cert`)** — plus in-place
enrichments to `ade_ledger::{state,pparams,error,fingerprint,rules}` (a
new `GovCertEnv` + `gov_cert_env()`, the `ConwayOnlyDepositParams.drep_activity`
field, two new `ValidationEnvironmentError` variants, a fingerprint
tag-2→3 migration, and the gov-state threading/carry-forward in the
block path) and the new CI gate
`ci/ci_check_gov_cert_accumulation_closed.sh`. PHASE4-B4 enriched the
existing B3 BLUE surfaces in place
(owner-complete `ConwayCert`, the `DRep` enum and
`PoolRegistrationCert.owners` field in `ade_types`, the
`apply_conway_cert` owner-tagged apply model and `GovernanceOwner` tags
in `ade_ledger::delegation`, and the `accumulate_tx_certs` era-dispatch
in `ade_ledger::rules`) plus a CI-gate extension and a new corpus. The
B4 surfaces are §3 module modifications, not new modules. **OQ5
added no new module either** — it is an in-place type change
(`StakeCredential` tuple struct → discriminated enum in
`ade_types::shelley::cert`) rippled across `ade_codec::{shelley,conway}::cert`,
`ade_ledger::{state,gov_cert,governance,cert_classify,rules,fingerprint}`,
and the GREEN `ade_testkit` snapshot loader, plus a new CI gate
(`ci/ci_check_credential_discriminant_closed.sh`) and a new corpus. The
OQ5 surfaces are §3 module modifications. **COMMITTEE-CRED-FIDELITY added
no new module, no new crate, no new CI script, and no new rule** — it is
an in-place re-keying/re-typing of the committee surfaces
(`ConwayGovState.committee` `Hash28` → `StakeCredential` in
`ade_ledger::state`; `GovActionState.committee_votes`
`Vec<(Hash28, Vote)>` → `Vec<(StakeCredential, Vote)>` in
`ade_types::conway::governance`) plus full-credential-equality committee
ratification (`ade_ledger::governance`), a new `write_committee_vote_list`
fingerprint writer (`ade_ledger::fingerprint`), tag-preserving committee
parses in the GREEN loader, an **extension** of the existing
`ci_check_credential_discriminant_closed.sh` (committee-surface clauses),
and a corpus extension. The COMMITTEE-CRED-FIDELITY surfaces are §3 module
modifications.

Crate dependency shape at HEAD (new deps in this delta):
- `ade_core` gained `ade_types`, `ade_crypto`, `minicbor`, and
  `ade_codec` (B1) as deps; dev deps `ade_testkit`, `serde_json`,
  `cardano-crypto` (`vrf-draft03`).
- **`ade_ledger` gained an `ade_core` dep edge** (PHASE4-B1) so the
  block-validity composition can call the consensus header authority,
  plus `minicbor` (dep) and `ade_testkit` (dev). PHASE4-B2, PHASE4-B3,
  and PHASE4-B3F added **no new ledger manifest deps** — `tx_validity`,
  `mempool`, and the B3 `cert_classify` / conservation accounting
  compose existing ledger surfaces (`conway`, `cert_classify`, `rules`,
  `shelley`, `delegation`, `pparams`, `state`, `witness`, `plutus_eval`)
  plus the existing `ade_codec` / `ade_types` / `ade_crypto` deps.
- `ade_runtime` gained `ade_core`, `ade_crypto`, `ade_codec`,
  `serde_json` (deps); `ade_testkit`, `cardano-crypto` (dev); the N-D
  deps `redb = "2"` (Tier 5) and `tempfile = "3"` (dev).
- `ade_testkit` gained `ade_core`, `ade_runtime` (deps);
  `cardano-crypto` (dev).
- `ade_core_interop` is new.

Corpora at HEAD: N-A capture corpus under `corpus/network/{n2n,n2c}/`;
N-B replay corpus under `corpus/consensus/`; B1 validity corpus under
`corpus/validity/` (positive Conway-576 blocks; the adversarial half is
*derived at test time*, only a README is committed). The B2
tx-adversarial corpus is likewise *derived*. **PHASE4-B3 added** the
closed cert-grammar reference (`corpus/conway_certs/{classification_table.md,tags.json}`),
the real epoch-576 resolved-input oracle
(`corpus/validity/conway_epoch576/{resolution_set.txt,resolved_inputs.json}`),
and the extraction tool note (`corpus/tools/extract_conway_resolved_inputs.md`).
**PHASE4-B3F added no corpus** (the strictness tests are inline unit
tests in `conway_cert_classification.rs`). **PHASE4-B4 added** the
cert-state accumulation corpus note (`corpus/cert_state/README.md`,
recording the environment-blocked epoch-576 oracle); the positive /
replay-byte-identical / adversarial cert-state cases are synthetic and
derived at test time in `crates/ade_ledger/tests/cert_state_corpus.rs`
and `crates/ade_codec/tests/conway_cert_decode_complete.rs`.
**PHASE4-B5 added** the governance-state accumulation corpus note
(`corpus/gov_state/README.md`, recording the environment-blocked
epoch-576 VState oracle); the positive / replay-byte-identical /
adversarial gov-state cases are synthetic and derived at test time in
`crates/ade_ledger/tests/gov_state_corpus.rs`.

Cross-reference: CODEMAP must be regenerated to add per-submodule/file
entries for the B3 BLUE surfaces (`ade_codec::conway::cert`,
`ade_codec::conway::withdrawals`, `ade_ledger::cert_classify`) in
addition to the still-unrecorded N-B + B1 + B2 surfaces, to record the
B3F strictness on `conway::cert`, and to record the **B4 enrichments**:
the owner-complete `ConwayCert` grammar, the `DRep` enum and
`PoolRegistrationCert.owners` field in `ade_types`, the
`apply_conway_cert` owner-tagged apply model
(`ConwayCertAction`/`ConwayCertOutcome`/`GovernanceOwner`) in
`ade_ledger::delegation`, and the `accumulate_tx_certs` era-dispatch in
`ade_ledger::rules`, and the **B5 surface**: the new BLUE file
`ade_ledger::gov_cert` (`apply_conway_gov_cert`), the `GovCertEnv` +
`gov_cert_env()` and `ConwayOnlyDepositParams.drep_activity` in
`ade_ledger::{state,pparams}`, the two new
`ValidationEnvironmentError::{MissingDRepActivityParam,DRepActivityOverflow}`
variants, and the gov-state carry-forward in `ade_ledger::rules`. SEAMS
must record the closed Conway cert grammar (tags 0..18, `UnknownCertTag`
reject, trailing-byte reject), the closed deposit-classification surface
(`CertDisposition` / `DepositEffect` / `CoinSource` in
`ade_types::conway::cert`), the canonical deposit-param authority
(`ConwayOnlyDepositParams` / `ConwayDepositParams` /
`LedgerState::conway_deposit_view`), the **B4 closed cert-apply
surface** — the owner-tagged `ConwayCertAction` taxonomy (total over 18
tags) and the fail-closed `accumulate_tx_certs` era-dispatch chokepoint —
and the **B5 closed gov-apply surface**: the native
`apply_conway_gov_cert` dispatch (total over 18 tags, governance-owned
fields only), the fail-fast `gov_cert_env()` env constructor, and the
`Option<ConwayGovState>` threading + carry-forward through `apply_block`
(replacing B4's observe-and-drop). TRACEABILITY must add a row for the
new **`DC-LEDGER-09`** (`enforced`, CI
`ci_check_gov_cert_accumulation_closed.sh`) on top of `DC-LEDGER-08`
(`enforced`, CI `ci_check_forbidden_patterns.sh`), `DC-TXV-06` (now
`enforced`) and `DC-TXV-07` (and still owes the 5 `DC-TXV-*`, 2
`DC-MEM-*`, 8 `DC-CONS-*`, and 6 `DC-VAL-*` rows). **The B3, B4, and B5
grounding-doc refreshes landed in `d766eb0`, `644eb03`, and `f81f815`
respectively; the OQ5 CODEMAP/SEAMS/TRACEABILITY refresh is the
in-flight working tree (this HEAD_DELTAS is current).** CODEMAP must
record the OQ5 type change — `StakeCredential` is now a closed enum
`{ KeyHash, ScriptHash }` with a `hash()` boundary accessor (`ade_types`),
both era `decode_stake_credential` preserve the tag (`ade_codec`),
`ConwayGovState` is re-keyed `Hash28` → `StakeCredential` (`ade_ledger::state`),
and `write_stake_credential` emits discriminant+hash (`ade_ledger::fingerprint`).
SEAMS must record that credential identity is now a closed sum
(discriminant-less credentials unrepresentable on the BLUE path).
TRACEABILITY must add the new `DC-LEDGER-10` row (`enforced`, CI
`ci_check_credential_discriminant_closed.sh`).

---

## 3. Modules Modified

| Module | Scope | Key changes |
|--------|-------|-------------|
| `ade_ledger` | +57 source/test files over the full delta (of which **PHASE4-B5: +8 files touched, +864 / −~14 lines** — `gov_cert.rs` +366 new, `gov_state_corpus.rs` +207 new, `rules.rs` +161/−42, `state.rs` +56, `error.rs` +16, `fingerprint.rs` +14, `pparams.rs` +8; **PHASE4-B4: +3 files touched, +780 / −~30 lines** — `delegation.rs` +385, `rules.rs` +212, `cert_classify.rs` +100, `cert_state_corpus.rs` +183 new; **PHASE4-B3: +12 files, +1,558 / −12 lines**; PHASE4-B2: +24 files, +3,817 / −5; PHASE4-B1: +13 files, +1,755; CE-73: +73). **PHASE4-B3F: no source change** | **PHASE4-B3 (primary value-conservation thread):** the crate gained the closed cert-deposit classifier `cert_classify.rs` (`classify`, `CertState` bridge) and substantially reworked the value-conservation accounting. **`conway.rs`** (+130 / −): `check_conway_coin_conservation` rewritten to the **full equation** `Σ(inputs) + Σ(withdrawals) + refunded_deposits == Σ(outputs) + fee + donation + new_deposits` (i128) — **the cert/withdrawal early-out is REMOVED**; certs and withdrawals are now decoded and accounted, never skipped. **`error.rs`** (+86): new `LedgerError::EraInvalidCertificate(EraInvalidCertificateError)` and `LedgerError::UnsupportedStateDependentDeposit(UnsupportedStateDependentDepositAccounting)`, plus `ValidationEnvironmentError::MissingConwayDepositParams`. **`pparams.rs`** (+78): new `ConwayOnlyDepositParams` (Conway-only, structurally `None` for pre-Conway) and the resolved `ConwayDepositParams` view. **`state.rs`** (+28): `LedgerState.conway_deposit_params: Option<ConwayOnlyDepositParams>` field + `conway_deposit_view()` accessor. **`fingerprint.rs`** (+116): Conway deposit-param fold added to the state fingerprint; the pre-Conway fold is **byte-identical** (the new field folds to nothing when `None`). Small wiring touches in `rules.rs`, `phase.rs`, `tx_validity/phase1.rs`, `byron.rs`, `epoch.rs`, `hfc.rs`, `shelley.rs`, `lib.rs` + 3 conservation test suites (`conway_conservation_full.rs`, `conway_conservation_adversarial.rs`, `conway_conservation_positive_synthetic.rs`). **PHASE4-B2:** the `tx_validity/` (7 files) and `mempool/` (3 files) submodules + B2 integration tests; the B2-S4 `check_conway_coin_conservation` first cut (deposit-free form). **PHASE4-B1:** the `block_validity/` submodule, `consensus_view.rs`, `consensus_input_extract.rs`, the `ade_core` dep edge. **CE-73:** 10 unit tests for `decode_invalid_tx_indices`. **PHASE4-B3F:** no `ade_ledger` source change — the B3F-S1 grep-gate references `cert_classify.rs` from CI but does not edit it. **PHASE4-B4 (primary cert-state-accumulation thread):** **`delegation.rs`** (+385) gained the **native owner-tagged apply model** — `conway_cert_action` + `apply_conway_cert` returning `ConwayCertOutcome`, the `ConwayCertAction` taxonomy (`MutateCertState` / `Governance(effect)` / `CertStateAndGovernance(effect)` / `NotValidInEra`) total over the 18 Conway tags, the owner tags `GovernanceOwner` / `GovernanceCertEffect` / `OwnerTaggedEffect`, and `ConwayCertEnv`; governance certs (vote-deleg / committee auth+resign / DRep reg/unreg/update) are owner-tagged to `ConwayGovState` and routed out of B4 mutation scope (observed, not applied — deferred to PHASE4-B5), composite tags 10/12/13 carry both a cert-state mutation and a governance effect, and no Conway cert maps to `Neutral` for lack of an owner. **`rules.rs`** (+212): `process_block_certificates` now calls the new fail-closed `accumulate_tx_certs(era, cert_bytes, &cert_state, key_deposit) -> Result<CertState, LedgerError>` — the **`_era` discard is removed** (explicit `CardanoEra` dispatch: Conway via `ade_codec::conway::cert::decode_conway_certs` + `apply_conway_cert`, Shelley..Babbage via the Shelley path) and the **two "non-fatal during replay" swallows are removed**; decode/apply errors propagate as structured `LedgerError` and halt the block transition (with an inline test module exercising era-dispatch + fail-closed decode/apply/unknown-tag/removed-tag/governance-routing). **`cert_classify.rs`** (+100): re-pointed at the owner-complete `ConwayCert` shape (`PoolRegistration(cert)`, struct variants now `{ .., deposit }` / `{ refund, .. }`); classification dispositions unchanged, exhaustiveness preserved (still no `_ =>` wildcard). New test corpus `tests/cert_state_corpus.rs` (+183, synthetic positive accumulation + replay byte-identical + adversarial no-false-accept). **PHASE4-B5 (primary governance-cert-accumulation thread):** the crate gained the new BLUE module **`gov_cert.rs`** (+366) — `apply_conway_gov_cert`, a pure total dispatch over the owner-complete `ConwayCert` mutating only `ConwayGovState` (vote-delegation → `vote_delegations`, committee auth/resign → `committee_hot_keys`, DRep reg/update → env-driven `drep_expiry` via `checked_add`, DRep unreg → remove); it never touches the B4-owned `CertState`, so composite certs are not double-applied. **`state.rs`** (+56): new `GovCertEnv { current_epoch, drep_activity }` + fail-fast `LedgerState::gov_cert_env()` (the only `GovCertEnv` constructor; absent param → `MissingDRepActivityParam`, never defaulted). **`pparams.rs`** (+8): `ConwayOnlyDepositParams.drep_activity: u64` (Conway PParams field 31). **`error.rs`** (+16): two new `ValidationEnvironmentError` variants — `MissingDRepActivityParam` and `DRepActivityOverflow`. **`fingerprint.rs`** (+14): the Conway-deposit fingerprint tag's array extended **2→3** to fold `drep_activity` — a deliberate `T-DET-01` migration, golden regenerated (`b69422ef…71d9` → `d1803cb7…8827`), byte-identical for pre-Conway / param-absent states. **`rules.rs`** (+161/−42): `process_block_certificates` / `accumulate_tx_certs` now thread an `Option<ConwayGovState>` alongside the cert-state, **apply the governance half** via `apply_conway_gov_cert` (gov apply errors propagate and halt), the B4 "routed out of B4 mutation scope" observe-and-drop comment is **removed**, and `gov_state` is **carried forward through `apply_block`** (it was nulled to `None` at both classified/verdict apply sites before B5). New test corpus `tests/gov_state_corpus.rs` (+207, synthetic positive gov-state accumulation + replay byte-identical + adversarial: missing-env reject, expiry-overflow reject, decode-layer guard, double-resign determinism). **OQ5 (credential-discriminant fidelity, OQ5-S1 `4187330` / OQ5-S2 `a3ee2da`; +6 files touched):** **`state.rs`** (+11): `ConwayGovState` **re-keyed `Hash28` -> `StakeCredential`** across `vote_delegations` / `committee_hot_keys` / `drep_expiry`, so a key-hash and a script-hash sharing 28 bytes are distinct authoritative-state keys (matching cardano-node's `Credential`-keyed UMap/VState). **`fingerprint.rs`** (+78): new `write_stake_credential` emits **discriminant + hash**; the gov-map fingerprint writers route through it (a deliberate dual cert-state + gov-state migration, `T-DET-01`) while the `Hash28`-keyed stake-snapshot writer stays `write_hash28` -- **no golden drift** (the affected gov/cert surfaces are empty/credential-free in the committed states). **`gov_cert.rs`** (+/-38), **`governance.rs`** (+32), **`cert_classify.rs`** (+2), **`rules.rs`** (+17): follow the key-type change; `cred.hash()` is used only at the genuine bare-byte boundary (the stake-distribution snapshot). New test corpus `tests/credential_fidelity_corpus.rs` (+140: same-bytes-distinct cert-state / gov-state, discriminant-changes-fingerprint, replay byte-identical). **COMMITTEE-CRED-FIDELITY (committee member + vote credential fidelity, S1 `2303a60` / S2 `2aeea16`; +4 ledger files touched):** **`state.rs`** (+2): `ConwayGovState.committee` **re-keyed `Hash28` -> `StakeCredential`** (committee member map keys on the discriminated cold credential). **`governance.rs`** (+76): `evaluate_ratification` / `check_ratification` now take a `BTreeMap<StakeCredential, u64>` committee-member set, and committee-vote resolution (hot voter -> hot->cold mapping -> cold member) is **full-credential equality** -- the prior `hot.hash() == hot_cred` and `**c == *cold.hash()` comparisons are gone, so a key-hash hot key never cross-resolves to a script-hash member of equal bytes; an inline `committee_fidelity_tests` module adds the cross-resolve negative + positive control. **`fingerprint.rs`** (+18): new `write_committee_vote_list` (canonical, sorts committee votes by the discriminated credential's `Ord`) replaces `write_vote_list` for `committee_votes`, and the committee-member map writer routes through `write_stake_credential` (instead of `write_hash28`) -- a `T-DET-01` migration with **no golden drift** (committee states are empty in the committed fingerprint surfaces); `drep_votes` / `spo_votes` still use `write_vote_list` (`Hash28`). New corpus cases in `tests/credential_fidelity_corpus.rs` (+32): `committee_keyhash_scripthash_same_bytes_distinct`, `committee_discriminant_changes_fingerprint`. **DREP-VOTE-FIDELITY (DRep-vote credential fidelity, S1 `ba4ff37` / S2 `62c9020`; +3 ledger files touched):** **`governance.rs`** (+57): `check_ratification`'s `lookup_stake` closure re-typed `|cred: &Hash28|` -> `|cred: &StakeCredential|` and now resolves the voter to **exactly one** `DRep` stake key by its discriminant (`KeyHash` -> `DRep::KeyHash`, `ScriptHash` -> `DRep::ScriptHash`) -- the prior `drep_stake.get(&DRep::KeyHash(..)).or_else(|| drep_stake.get(&DRep::ScriptHash(..)))` OR-fallback over identical bytes is **gone**; an inline `committee_fidelity_tests` addition (`drep_keyhash_scripthash_do_not_cross_resolve` + a `ratifies_drep` helper) is the cross-resolve negative + positive control. **`fingerprint.rs`** (+6): `write_committee_vote_list` **renamed `write_credential_vote_list`** and now writes both `committee_votes` and `drep_votes` (`write_gov_action_state` routes `drep_votes` through it instead of `write_vote_list`); `spo_votes` stays `write_vote_list` over `Hash28` -- a `T-DET-01` migration with **no golden drift** (the gov-action-state vote surfaces are empty in the committed states). New corpus case in `tests/credential_fidelity_corpus.rs` (+38): `drep_vote_discriminant_changes_fingerprint`. **ENACTMENT-COMMITTEE-FIDELITY (committee-enactment effect credential fidelity, S1 `a6b8de7`; +1 ledger file touched):** **`governance.rs`** (+30): `EnactmentEffects.committee_changes` re-typed `Option<(Vec<Hash28>, Vec<(Hash28, u64)>)>` -> `Option<(Vec<StakeCredential>, Vec<(StakeCredential, u64)>)>` -- the (removed, added-with-expiry) committee-enactment effect now carries the discriminated cold committee credential, never bare `Hash28`. The field is **DORMANT** (always `None`; `UpdateCommittee` enactment is still a no-op), so this pins the type only -- it prevents committee enactment, once wired, from re-collapsing the discriminated `ConwayGovState.committee` map on write-back. An inline `committee_fidelity_tests` addition (`enactment_committee_changes_keyhash_scripthash_distinct`) builds an `EnactmentEffects` with key/script members of equal bytes and asserts they are distinct entries, plus that the default stays dormant `None`. **No golden drift** (the field is `None`; no fingerprint surface changes). |
| `ade_codec` | +11 source/test files (PHASE4-B3 + B3F + B4; B4: `conway/cert.rs` +147, `shelley/cert.rs` +108, `conway_cert_decode_complete.rs` +313 new, `conway_cert_classification.rs` +14) | **PHASE4-B3:** the new BLUE `conway::cert` decoder (`cert.rs`, closed grammar tags 0..18) and `conway::withdrawals` decoder (`withdrawals.rs`, `RewardAccount` map, i128 sum), wired through `conway/mod.rs`; `error.rs` (+13) added `CodecError::UnknownCertTag { tag, offset }` and `CodecError::DuplicateMapKey { offset }`. Two new test suites: `conway_cert_classification.rs` (decode total over tags 0..18, unknown-tag reject, removed-tag-5/6 reject, malformed-CBOR reject, replay-determinism), `conway_withdrawals.rs`. **PHASE4-B3F-S2 (`193d2fc`, +18 in `cert.rs`):** `decode_conway_certs` hardened — **trailing bytes after the cert array now reject** with `CodecError::TrailingBytes { consumed, total }` (parity with `decode_withdrawals`); the indefinite-array break byte is consumed; definite-array preallocation bounded by `(n).min(data.len())` (no over-allocation on a crafted huge count, no behavioral change for valid input). +2 tests in `conway_cert_classification.rs` (`trailing_bytes_after_cert_array_rejected`, `huge_array_count_rejects_without_overallocating`). `ade_codec` was untouched before B3. **PHASE4-B4-S1 (`228415b`, +147 in `conway/cert.rs`, +108 in `shelley/cert.rs`):** `decode_conway_certs` made **owner-complete** — it now reads and retains every owner payload for all 18 tags (credentials, pool id, full pool params incl. `pool_owners`, DRep delegation targets) where it previously kept only the deposit/refund projection; a new `decode_drep` reads the DRep target; the shared `read_pool_registration_cert` (`shelley/cert.rs`) now reads up to and including `pool_owners` (the caller consumes trailing relays/metadata) and returns them via the new `PoolRegistrationCert.owners`. Fields no owner stores (cert anchors, relays, metadata) remain structurally consumed; unknown-tag reject, removed-tag-5/6 → `RemovedInConway`, trailing-byte reject, and bounded preallocation are unchanged. New test suite `conway_cert_decode_complete.rs` (+313, per-tag owner-payload retention) + `conway_cert_classification.rs` (+14, updated to the new variant shapes). **OQ5-S1 (`4187330`, +14 in `conway/cert.rs`, +14 in `shelley/cert.rs`):** both era `decode_stake_credential` now **preserve the key/script tag** -- tag `0` -> `StakeCredential::KeyHash`, `1` -> `ScriptHash`, any other tag -> deterministic `CodecError::InvalidCborStructure` reject; the prior `let (_tag, _) = ...` / `let (_cred_type, _) = ...` tag-discard form is gone. OQ5-S2 extended `conway_cert_decode_complete.rs` (+71) with discriminant-preservation cases and added `tests/shelley_credential_discriminant.rs` (+65). |
| `ade_types` | +3 files (B3) + 2 files (B4); B4: `conway/cert.rs` +92, `shelley/cert.rs` +4 | **PHASE4-B3:** `conway/cert.rs` (+84) gained the closed `ConwayCert` enum plus the classification value types `CertDisposition`, `DepositEffect`, `CoinSource` consumed by `ade_ledger::cert_classify`; `tx.rs` (+6) added `RewardAccount(pub [u8; 29])` for the withdrawals decoder; `lib.rs` re-export wiring. First delta since baseline. The B3F-S1 grep-gate references this file (closed-variant guard) but does not edit it. **PHASE4-B4-S1 (`228415b`, +92 in `conway/cert.rs`, +4 in `shelley/cert.rs`):** `ConwayCert` made **owner-complete** — formerly payload-free variants now carry their owner fields (`StakeDelegation { credential, pool_id }`, `PoolRegistration(PoolRegistrationCert)`, `PoolRetirement { pool_id, epoch }`, `VoteDelegation { credential, drep }`, the composite tags 10/12/13 with their full field sets, `AuthCommitteeHot { cold_credential, hot_credential }`, `ResignCommitteeCold`, `DRepRegistration`/`DRepUnregistration`/`DRepUpdate` with `drep_credential`); the deposit-bearing variants additionally retain `credential`/`deposit`/`refund`. New **`DRep` enum** added for the vote-delegation target. `shelley/cert.rs` (+4) added the **`PoolRegistrationCert.owners: Vec<Hash28>`** field (`pool_owners`, retained for cert-state accumulation; relays/metadata still dropped). The closed taxonomy stays closed (no `#[non_exhaustive]`, no open-tail variant) — the B3F grep-gate continues to guard it. **OQ5-S1 (`4187330`, +20 in `shelley/cert.rs`):** **`StakeCredential` changed from the tuple struct `StakeCredential(pub Hash28)` to a closed `enum { KeyHash(Hash28), ScriptHash(Hash28) }`** with a discriminant-erasing `hash()` accessor reserved (per its doc comment) for genuine bare-byte boundary adapters -- never to re-key authoritative cert/gov state. `Ord` derives lexicographically over (variant, hash) so the two same-byte credentials are distinct keys. A discriminant-less credential is now unrepresentable on the BLUE authority path. **COMMITTEE-CRED-FIDELITY-S1 (`2303a60`, +1 in `conway/governance.rs`):** `GovActionState.committee_votes` re-typed `Vec<(Hash28, Vote)>` -> `Vec<(StakeCredential, Vote)>` (the committee voter now carries its key/script discriminant); `drep_votes` / `spo_votes` stay `Vec<(Hash28, Vote)>` (out of scope -- DRep-vote discrimination is the declared next-cluster follow-up). **DREP-VOTE-FIDELITY-S1 (`ba4ff37`, +1 in `conway/governance.rs`):** `GovActionState.drep_votes` re-typed `Vec<(Hash28, Vote)>` -> `Vec<(StakeCredential, Vote)>` (the DRep voter now carries its key/script discriminant); `spo_votes` stays `Vec<(Hash28, Vote)>` (pools are always key-hash -- a permanent non-goal, the type-level expression of the spo/drep asymmetry). |
| `ade_core` | +29 source files + tests (N-B, +8,076 lines); +828 / −86 across 16 files (B1) | **PHASE4-N-B:** stub `lib.rs` → substantive BLUE consensus module under `src/consensus/`. **PHASE4-B1:** added `consensus/kes_check.rs` (fail-closed `expect_size` + KES header guard); wired single-VRF + KES header validation across `header_validate.rs`, `vrf_cert.rs`, `leader_schedule.rs`, `fork_choice.rs`, etc. (14/14 real Conway headers validate). New dep `ade_codec` (B1). **No B2, B3, or B3F source change** — tx-validity and value-conservation compose only `ade_ledger` surfaces. |
| `ade_crypto` | 1 file, +24 / −81 lines (B1) | Single change in `kes.rs` (`500589b`): **`build_opcert_signable` fixed** as part of B1-S5 KES header validation. No source change in N-A, N-B, N-D, B2, B3, or B3F. |
| `ade_core_interop` | +1,546 across 6 files (B1) | **CE-N-B-6 follow-bridge (`e5f1f64`) + pin retarget (`807bcb6`):** RED `follow.rs` (BLUE fork-choice + rollback only) + `follow_offline_replay.rs`; reworked `lib.rs`, the live-session binary and test. New deps `ade_codec`, `ade_crypto` for offline replay. |
| `ade_network` (existing crate, refined) | 100 files, +17,861 lines (whole crate is new this delta — see §2; the post-N-A delta is the DoS hardening) | **DoS hardening of 6 codecs** (`744ef34`, post-N-A close): capped untrusted `Vec::with_capacity` hints. No transition-authority change since N-A closure; no change in N-B/B1/B2/B3/B3F. |
| `ade_runtime` | +18 files, +3,440 lines (N-B `consensus/` + N-D `chaindb`/`recovery`; B1 one small touch) | **PHASE4-N-B:** new `consensus/` submodule (`candidate_fragment.rs`, `chain_selector.rs`, `genesis_parser.rs`) + corpus test. **PHASE4-B1:** one small touch. The N-D `chaindb`/`recovery` submodules + kill-target binary are §2 New Modules. No B2/B3/B3F change. |
| `ade_testkit` | +28 files, +3,251 lines: `consensus/` (N-B); `validity/` (B1); `tx_validity/` (B2); **B3 conservation extensions** | **PHASE4-N-B:** `consensus/` harness. **PHASE4-B1:** `validity/` harness (M1–M6 mutators). **PHASE4-B2:** `tx_validity/` submodule (extractor, synthetic builders, W1–W4 / S1–S4 mutators + judge). **PHASE4-B3:** extended `harness/snapshot_loader.rs` (+20, intra-corpus input resolution), `tx_validity/{adversarial,valid_synthetic}.rs` for conservation cases, added the real epoch-576 positive-corpus suite `tests/conway_conservation_positive_corpus.rs` (10 non-Plutus cert/withdrawal txs Valid at `track_utxo=true`; Plutus carved out per CE-88), and three RED example bins that materialize the B3 corpora (`examples/{dump_b3_cert_tags,dump_b3_resolution_set,resolve_b3_intra_corpus}.rs`). New deps `ade_core`, `ade_runtime` (B1). **No B3F change.** **PHASE4-OQ5-S1 (`4187330`):** GREEN `harness/snapshot_loader.rs` (+75) -- the gov-map and DRep-registration parses now **preserve the key/script tag** when constructing `StakeCredential` keys (the loader is the boundary that materializes a snapshot into discriminated gov-state); `epoch_oracle_comparison.rs` (+36) follows the key-type change. **COMMITTEE-CRED-FIDELITY-S1 (`2303a60`):** the GREEN `harness/snapshot_loader.rs` committee parses (`parse_committee_state` / `parse_committee_vote_map`) now **preserve the key/script tag** when materializing committee members and committee votes as `StakeCredential` keys; `epoch_oracle_comparison.rs` follows the committee key-type change. (The GREEN loader's `mk_credential` defaults an unknown `tag != 1` to `KeyHash` -- a declared follow-up (e), contained to `ade_testkit`, cannot reach the node binary.) **DREP-VOTE-FIDELITY-S1 (`ba4ff37`):** the GREEN `harness/snapshot_loader.rs` (+20) committee-vote parser `parse_committee_vote_map` is **renamed `parse_credential_vote_map`** and now parses **both** the committee and the DRep vote maps tag-preserving (the `drep_votes` map previously read via the bare-`Hash28` `parse_vote_map`); `spo_votes` stays `parse_vote_map`. `epoch_oracle_comparison.rs` (+40) follows the DRep vote key-type change. |

No other crate had non-trivial source changes since baseline.
`ade_plutus` and `ade_node` were untouched by code commits. **OQ5
touched `ade_types`, `ade_codec`, `ade_ledger`, and (GREEN)
`ade_testkit`** (plus the new CI gate and corpus) — all in-place ripples
of the `StakeCredential` tuple-struct -> enum change; `ade_plutus`,
`ade_core`, `ade_runtime`, and `ade_node` were untouched by OQ5.
**COMMITTEE-CRED-FIDELITY touched only `ade_ledger`
(`state`/`governance`/`fingerprint`), `ade_types` (`conway::governance`),
and (GREEN) `ade_testkit` (the committee snapshot parses)** — plus the
*extension* of the existing `ci_check_credential_discriminant_closed.sh`
and the corpus extension; no new module, no new crate, no new CI script,
no new rule. `ade_codec` was untouched by COMMITTEE-CRED-FIDELITY (both
era `decode_stake_credential` already preserve the tag from OQ5).
**DREP-VOTE-FIDELITY touched only `ade_ledger`
(`governance`/`fingerprint`), `ade_types` (`conway::governance`), and
(GREEN) `ade_testkit` (the renamed DRep/committee vote-map parser)** —
plus the *extension* of the same
`ci_check_credential_discriminant_closed.sh` and the corpus extension;
no new module, no new crate, no new CI script, no new rule. `ade_codec`
was untouched by DREP-VOTE-FIDELITY (the DRep target decoder already
discriminates from B4/OQ5).
**ENACTMENT-COMMITTEE-FIDELITY touched only `ade_ledger`
(`governance`)** -- a one-line preventive type migration of the dormant
`EnactmentEffects.committee_changes` field (`Hash28` -> `StakeCredential`)
plus an inline distinctness test -- alongside the *extension* of the same
`ci_check_credential_discriminant_closed.sh` (one new clause); no new
module, no new crate, no new CI script, no new rule, no fingerprint /
golden change (the field stays `None`). `ade_types`, `ade_codec`, and
`ade_testkit` were untouched (the migration is internal to the BLUE
enactment-effect type; no wire surface and no GREEN loader is involved
while the field is dormant).
**PHASE4-B4 touched only `ade_codec`, `ade_types`, and `ade_ledger`**
(plus the CI script and corpus). **PHASE4-B5 touched only `ade_ledger`** (the new
`gov_cert.rs` module + the `state`/`pparams`/`error`/`fingerprint`/`rules`
enrichments, plus its corpus and the new CI gate); the five `ade_testkit`
touches in B5 (`harness/snapshot_loader.rs` +7,
`tx_validity/{adversarial,valid_synthetic}.rs`,
`tests/{conway_conservation_positive_corpus,tx_validity_compose}.rs`,
each +1) are **trivial** ripples populating the new
`ConwayOnlyDepositParams.drep_activity` field in fixtures — under the
<10-line trivial-change threshold, not a §3 row. `.idd-config.json` had
prose edits during the delta; the `core_paths` array already covers the
B3, B4, and B5 surfaces (`ade_codec::conway::*`,
`ade_ledger::cert_classify`, `ade_ledger::delegation`,
`ade_ledger::rules`, the new `ade_ledger::gov_cert`,
`ade_ledger::{state,pparams,error,fingerprint}`) and the B3F-hardened
decoder via the already-listed `ade_codec` / `ade_ledger` / `ade_types`
crate prefixes.

---

## 4. Feature Flags

No Cargo `[features]` tables exist at HEAD in any workspace crate, and
none existed at baseline. The project does not use Cargo feature flags
as a semantic surface — closed semantic surfaces are encoded in the
type system per the IDD core principles, and conditional compilation is
checked out of BLUE code via `ci/ci_check_no_semantic_cfg.sh` (scoped
over the full 6-crate BLUE set; the B3 surfaces
`ade_codec::conway::cert`/`::withdrawals` and
`ade_ledger::cert_classify`, the B4 surfaces
`ade_ledger::delegation`/`::rules`, the B5 surface
`ade_ledger::gov_cert`, and the OQ5 surfaces (`ade_types::shelley::cert`
`StakeCredential`, `ade_codec::{shelley,conway}::cert`
`decode_stake_credential`), and the COMMITTEE-CRED-FIDELITY surfaces
(`ade_ledger::{state,governance,fingerprint}`,
`ade_types::conway::governance`) are covered by their crate-level scope,
as are the B2 `tx_validity`/`mempool` and B1 `block_validity` surfaces).

No `#[cfg(feature = ...)]` gates appear at either ref. `cardano-crypto`
(`vrf-draft03`) and `minicbor` (`alloc`) feature selections in the
dependency entries are upstream-crate selections, not Ade-side flags.

**Status: unchanged — zero Ade feature flags at baseline, zero at HEAD.**

---

## 5. CI Checks

The CI surface is the shell-script set under `ci/` (no
`.github/workflows` in this repo). At baseline there were 15 scripts.
At HEAD there are **29 scripts plus one git hook**: CE-73 added one
(`ci_check_hfc_translation.sh`), N-D added three, N-A added two, N-B
added four, PHASE4-B3 added one (`ci_check_deposit_param_authority.sh`),
**PHASE4-B3F added one** (`ci_check_conway_cert_classification_closed.sh`),
**PHASE4-B5 added one** (`ci_check_gov_cert_accumulation_closed.sh`),
**OQ5 added one** (`ci_check_credential_discriminant_closed.sh` — the
29th script, up from 28 at the B5 close), and one repo-local git hook
(`ci/git-hooks/commit-msg`). **COMMITTEE-CRED-FIDELITY added no new CI
script** — it **extended the OQ5 `ci_check_credential_discriminant_closed.sh`**
with two committee-surface clauses (see the COMMITTEE-CRED-FIDELITY
subsection below); the count **stays 29**. **DREP-VOTE-FIDELITY likewise
added no new CI script** — it **extended the same
`ci_check_credential_discriminant_closed.sh`** with two DRep-surface
clauses (see the DREP-VOTE-FIDELITY subsection below); the count
**stays 29**. **ENACTMENT-COMMITTEE-FIDELITY likewise added no new CI
script** — it **extended the same
`ci_check_credential_discriminant_closed.sh`** with one
committee-enactment-effect clause (clause 6; see the
ENACTMENT-COMMITTEE-FIDELITY subsection below); the count **stays 29**.
**PHASE4-B1
and PHASE4-B2 each added no new CI script** (both reused/extended the
N-B closed-enums script). **PHASE4-B4 added no new CI script** — it
**extended the baseline `ci_check_forbidden_patterns.sh`** with a
fail-closed cert-state-accumulation guard (see the B4 subsection
below). Grouped by cluster.

### CE-73 reclassification (Phase 2C close-out)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_hfc_translation.sh` | **New** (`9b15378`) | CE-73-semantic gate: runs the three HFC ledger-side translation proof surfaces. Authoritative test for invariant `DC-EPOCH-02`. |

### IDD canonicalization (post-Phase-4-N-D)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_constitution_coverage.sh` | Modified (`39865f6`) | Path-only edit: registry path now `docs/ade-invariant-registry.toml`. Coverage enforcement unchanged. |
| `ci/git-hooks/commit-msg` | **New** (`2047c42`) | Local git hook: rejects commit messages lacking a `Co-Authored-By: Claude ...` trailer. Repo-local exception to the global no-AI-attribution rule, commit-messages only. |

### BLUE-list drift closure (`5b70bee`)

Six CI scripts were extended to the full 6-crate BLUE set
(`ade_codec`, `ade_types`, `ade_crypto`, `ade_core`, `ade_ledger`,
`ade_plutus`).

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_module_headers.sh` | Modified — BLUE-scope (`5b70bee`) | `// Core Contract:` header on every `.rs` in BLUE crates. `T-BUILD-01`. |
| `ci/ci_check_no_semantic_cfg.sh` | Modified — BLUE-scope (`5b70bee`) | No semantic `#[cfg(...)]` in BLUE `src/`. `T-BUILD-01`. |
| `ci/ci_check_no_signing_in_blue.sh` | Modified — BLUE-scope (`5b70bee`) | No signing primitives in BLUE crates. `T-KEY-01`. |
| `ci/ci_check_hash_uses_wire_bytes.sh` | Modified — BLUE-scope (`5b70bee`) | All BLUE hashing via wire-byte fingerprint surfaces. `T-ENC-01`, `DC-CBOR-02`. |
| `ci/ci_check_ingress_chokepoints.sh` | Modified — BLUE-scope + registry growth (`5b70bee`) | No raw CBOR decoding outside named chokepoints in BLUE. `T-INGRESS-01`, `DC-INGRESS-01`. |
| `ci/ci_check_dependency_boundary.sh` | Modified — BLUE-scope (`5b70bee`) | BLUE crates must not depend on RED crates. `T-BOUND-02`. |

Follow-up `c8fa37f` re-ran CODEMAP and TRACEABILITY against the new
scope, removing 14 `_(scope gap)_` markers across 13 rules.

### Phase 4 N-D CI gap closure (`78da6c9`)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_chaindb_contract.sh` | **New** (`78da6c9`) | `cargo test -p ade_runtime --lib chaindb::` — 8 contract tests. `DC-STORE-02/03`, `CN-STORE-04/05`. |
| `ci/ci_check_recovery_contract.sh` | **New** (`78da6c9`) | `cargo test -p ade_runtime --lib recovery::` — 6-test recovery bundle. `T-REC-01/02`, `DC-STORE-05`. |
| `ci/ci_check_chaindb_crash_safety.sh` | **New** (`78da6c9`) | Smoke variant of the subprocess-SIGKILL harness + integrity post-checks. `T-REC-01`, `DC-STORE-01`, `CN-STORE-03`. |

### Phase 4 N-A wire + semantic enforcement (S-A1, S-A10)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_no_async_in_blue.sh` | **New** (`4fde3a7`, S-A1) | Enforces `DC-CORE-01` — BLUE code is sync-only. Scans every BLUE path in `core_paths`. Covers `ade_core::consensus`, `ade_ledger::block_validity`, `ade_ledger::tx_validity`, `ade_ledger::mempool`, and the B3 `ade_codec::conway::*` / `ade_ledger::cert_classify` surfaces via crate prefixes. |
| `ci/ci_check_ce_n_a_5_proof.sh` | **New** (`56bfa7b`, S-A10) | CE-N-A-5 closure-gate evidence over the real-cardano-node corpus. |

### Phase 4 N-B consensus authority enforcement (S-B1, S-B2, S-B8) — extended by B1 and B2

Four BLUE-scope CI scripts targeting `crates/ade_core/src/consensus/`.
The closed-enums script was **extended in PHASE4-B1** to also scan
`ade_ledger::block_validity`, then **extended again in PHASE4-B2** to
also scan `ade_ledger::tx_validity` and `ade_ledger::mempool`. The
closed-grammar / closed-enum cert surfaces (`ConwayCert`,
`CertDisposition`, `DepositEffect`, `CoinSource`,
`CodecError::UnknownCertTag`, `CodecError::DuplicateMapKey`) are now
guarded by the **dedicated PHASE4-B3F grep-gate** below (which
supersedes the prior "guarded by tests only" note).

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_consensus_closed_enums.sh` | **New** (N-B); **Modified** (B1, `7b95ccd`); **Modified** (B2) | Four-part scan over `ade_core/src/consensus/`, `ade_ledger/src/block_validity/`, `ade_ledger/src/tx_validity/`, and `ade_ledger/src/mempool/`: no `#[non_exhaustive]`; no open-tail `Other`/`Unknown`; no owned `String` in the error/encoding/verdict files; no `Box<dyn ...>`. Strengthens `DC-CONS-04/10`, `T-DET-01`, the `DC-VAL-*`, `DC-TXV-01/02/04/05`, and `DC-MEM-01/02` rules. |
| `ci/ci_check_no_chaindb_in_consensus_blue.sh` | **New** (N-B / S-B1) | No `ChainDb`/`chain_db` token in `consensus/`. Strengthens `DC-CORE-01`, `DC-CONS-07`. |
| `ci/ci_check_no_density_in_fork_choice.sh` | **New** (N-B / S-B8) | No `density` token in `fork_choice.rs` / `candidate.rs`. Strengthens `DC-CONS-03`. |
| `ci/ci_check_no_float_in_consensus.sh` | **New** (N-B / S-B1) | No `f32`/`f64` in `consensus/`. Strengthens `T-CORE-02`, `DC-CONS-07/08/09`. |

### Phase 4 B3 Conway value-conservation enforcement (`978c222`)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_deposit_param_authority.sh` | **New** (`978c222`) | **Enforces `DC-TXV-07` (canonical deposit-param authority).** Greps the BLUE crate sources and fails on any non-canonical deposit-parameter read — every `key_deposit` / `pool_deposit` / `drep_deposit` / `gov_action_deposit` term must come from `ProtocolParameters` + `LedgerState.conway_deposit_params`, never from the testkit `ConwayGovParams` RED intermediate, `ProtocolParameters::default()`, a literal constant beside a deposit field, or env/shell config. Comment-only lines are stripped so prose naming a forbidden symbol does not trip the gate. |

### Phase 4 B3F cert-classification closure enforcement (`d6c1993`)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_conway_cert_classification_closed.sh` | **New** (`d6c1993`, B3F-S1) | **Enforces `DC-TXV-06` (closed Conway cert-deposit classification) — flips the rule `partial` → `enforced`.** Three-part grep-gate: (1) the classification value types stay CLOSED — no `#[non_exhaustive]` and no open-tail `Other`/`Unknown` variant on `ConwayCert` / `CertDisposition` / `DepositEffect` / `CoinSource` (`ade_types::conway::cert`); (2) the decoder rejects unknown cert tags and has NO catch-all accept arm — `CodecError::UnknownCertTag` present and no `_ =>` arm constructs a `ConwayCert` (the reintroduced-Shelley-fallback anti-pattern; `ade_codec::conway::cert`); (3) `classify` stays exhaustive — no `_ =>` wildcard arm, so adding a new `ConwayCert` variant breaks the build instead of silently classifying it (`ade_ledger::cert_classify`). |

### Phase 4 B4 cert-state-accumulation fail-closed enforcement (`302d22c`)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_forbidden_patterns.sh` | **Modified** (`302d22c`, B4-S3/S4) | **Enforces `DC-LEDGER-08` (fail-closed cert-state accumulation).** Two new grep clauses on top of the baseline forbidden-pattern set: (1) fail if the cert-state fail-open rationale string `non-fatal during replay` reappears anywhere in `crates/**/*.rs` (the swallow that B4-S3/S4 removed — the justification was false, since the path runs only at `track_utxo` with full state present); (2) fail if `accumulate_tx_certs` (in `ade_ledger/src/rules.rs`) contains an `Err(_) =>` swallow arm. A reintroduced decode/apply swallow on the cert-state path is therefore caught by a standing CI invariant, not just tests. |

### Phase 4 B5 governance-cert-accumulation fail-closed enforcement (`06385d0`)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_gov_cert_accumulation_closed.sh` | **New** (`06385d0`, B5-S4) | **Enforces `DC-LEDGER-09` (closed, applied Conway governance-cert accumulation) — the 28th script.** A four-part grep-gate over `crates/ade_ledger/src/{gov_cert,rules,error,state}.rs`: (1) `apply_conway_gov_cert` exists and its `ConwayCert` dispatch has **no `_ =>` wildcard** arm (adding a variant breaks the build instead of silently dropping its governance effect); (2) the DRep-expiry computation uses **`checked_add`, never a bare `current_epoch + ...`** (a comment-stripped grep so prose naming `+` does not trip it) — an unchecked `+` would wrap silently in a release profile instead of halting with `DRepActivityOverflow`; (3) the **B4 observe-and-drop is gone** — the `routed out of B4 mutation scope` comment must not reappear in `rules.rs` and `accumulate_tx_certs` **must call `apply_conway_gov_cert`** (gov certs are applied, not dropped); (4) the env fail-fast is wired — `ValidationEnvironmentError::MissingDRepActivityParam` present in `error.rs` and `LedgerState::gov_cert_env()` present in `state.rs`. A regression that reintroduces observe-and-drop, drops a variant, or replaces the checked arithmetic is caught by a standing CI invariant, not just tests. |

### OQ5 credential key/script discriminant fidelity enforcement (`a3ee2da`)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_credential_discriminant_closed.sh` | **New** (`a3ee2da`, OQ5-S2) | **Enforces `DC-LEDGER-10` (credential key/script discriminant fidelity) — the 29th script.** A three-part grep-gate: (1) `StakeCredential` is the **closed 2-variant enum**, not the old tuple struct `StakeCredential(pub Hash28)` (`ade_types::shelley::cert`) — the tuple-struct shape reappearing fails the gate; (2) **both era decoders preserve the tag** — `ade_codec::{shelley,conway}::cert` map the credential type to `KeyHash`/`ScriptHash` and have no `let (_cred_type|_tag` tag-discard form; (3) **no `StakeCredential(<hash>)` tuple-construction coercion remains on the BLUE authority path** — credentials are built only via `::KeyHash` / `::ScriptHash` (or decode), so a bare-`Hash28` coercion that would re-introduce the collapse is caught. A regression to the tag-erased representation is caught by a standing CI invariant, not just the compiler + tests. |

### COMMITTEE-CRED-FIDELITY committee credential discriminant fidelity (`2aeea16`)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_credential_discriminant_closed.sh` | **Modified** (`2aeea16`, COMMITTEE-CRED-FIDELITY-S2) | **Extends the OQ5 `DC-LEDGER-10` gate to the committee surface — no new script, the count stays 29.** Two new grep clauses on top of the three OQ5 clauses: (4a) `ConwayGovState.committee` stays `StakeCredential`-keyed — `pub committee:.*BTreeMap<.*StakeCredential` must be present in `ade_ledger::state` (a bare-`Hash28`-keyed committee map fails the gate); (4b) `GovActionState.committee_votes` carries `StakeCredential` — `pub committee_votes:.*StakeCredential` must be present in `ade_types::conway::governance` (a `Vec<(Hash28, Vote)>` committee-vote list fails the gate). A regression that re-collapses either committee surface to a tag-erased `Hash28` is caught by the same standing CI invariant. |

### DREP-VOTE-FIDELITY DRep-vote credential discriminant fidelity (`62c9020`)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_credential_discriminant_closed.sh` | **Modified** (`62c9020`, DREP-VOTE-FIDELITY-S2) | **Extends the `DC-LEDGER-10` gate to the DRep-vote surface — no new script, the count stays 29.** Two new grep clauses on top of the three OQ5 + two committee clauses: (5a) `GovActionState.drep_votes` carries `StakeCredential` — `pub drep_votes:.*StakeCredential` must be present in `ade_types::conway::governance` (a `Vec<(Hash28, Vote)>` drep-vote list fails the gate); (5b) `governance.rs` has **no DRep key/script OR-fallback** — a `DRep::KeyHash(...).or_else` resolution pattern in `ade_ledger::governance` fails the gate (DRep stake must resolve to the exact variant, never a cross-resolution over identical bytes). A regression that re-collapses the DRep-vote surface to a tag-erased `Hash28` or re-introduces the OR-fallback is caught by the same standing CI invariant. `spo_votes` is deliberately left bare-`Hash28` (pools are always key-hash). |

### ENACTMENT-COMMITTEE-FIDELITY committee-enactment effect credential fidelity (`a6b8de7`)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_credential_discriminant_closed.sh` | **Modified** (`a6b8de7`, ENACTMENT-COMMITTEE-FIDELITY-S1) | **Extends the `DC-LEDGER-10` gate to the (dormant) committee-enactment effect surface — no new script, the count stays 29.** One new grep clause (clause 6) on top of the three OQ5 + two committee + two DRep-vote clauses: `EnactmentEffects.committee_changes` carries `StakeCredential` — `pub committee_changes:.*StakeCredential` must be present in `ade_ledger::governance` (a bare-`Hash28` committee-changes field fails the gate). A regression that re-collapses the committee-enactment effect to a tag-erased `Hash28` — which would silently re-collapse the discriminated committee map on write-back once `UpdateCommittee` enactment is wired — is caught by the same standing CI invariant before the field is ever made live. |

TRACEABILITY cross-reference: the four N-B scripts map to the 8
`DC-CONS-*` rules; the closed-enums script also enforces four
`DC-VAL-*`, four `DC-TXV-*` (01/02/04/05), and both `DC-MEM-*` rules;
`ci_check_deposit_param_authority.sh` is the `ci_script` for
`DC-TXV-07`; the B3F script
`ci_check_conway_cert_classification_closed.sh` is the `ci_script` for
`DC-TXV-06` (set at HEAD); **the extended
`ci_check_forbidden_patterns.sh` is the `ci_script` for `DC-LEDGER-08`**
(set in the B4 registry edit); and **the new
`ci_check_gov_cert_accumulation_closed.sh` is the `ci_script` for the
new `DC-LEDGER-09`** (set in the B5 registry edit); and **the new
`ci_check_credential_discriminant_closed.sh` is the `ci_script` for the
new `DC-LEDGER-10`** (set in the OQ5 registry edit; COMMITTEE-CRED-FIDELITY
**extended** the same script for the same rule — no new `ci_script`, the
committed registry records the committee tests under `DC-LEDGER-10` and
`strengthened_in += "COMMITTEE-CRED-FIDELITY"`).
The N-B / B1 / B2 rows, the B3 rows (`DC-TXV-06`/`07`), the B3F
DC-TXV-06/DC-VAL-06 edits, and the B4 `DC-LEDGER-08` row landed in the
*committed* TRACEABILITY at the B3 close `d766eb0` and the B4 close
`644eb03`; the B5
TRACEABILITY edit (the `DC-LEDGER-09` row) landed at the B5 close
`f81f815`; and the **OQ5 TRACEABILITY edit** (the `DC-LEDGER-10` row,
`enforced` with `ci_check_credential_discriminant_closed.sh`, plus the
`T-DET-01` / `T-ENC-03` / `DC-LEDGER-08` / `DC-LEDGER-09` / `DC-TXV-05`
`cross_ref` links) landed at the OQ5 close `676af5a`. The
**COMMITTEE-CRED-FIDELITY TRACEABILITY edit** (`DC-LEDGER-10`'s
`strengthened_in += "COMMITTEE-CRED-FIDELITY"`, the +3 committee tests,
the extended `code_locus`) landed at the COMMITTEE-CRED close `a157c92`.
The **DREP-VOTE-FIDELITY TRACEABILITY edit** (`DC-LEDGER-10`'s
`strengthened_in += "DREP-VOTE-FIDELITY"`, the +2 DRep tests, the
further-extended `code_locus`) landed at the DREP-VOTE close `06f517f`.
The **ENACTMENT-COMMITTEE-FIDELITY TRACEABILITY edit** — `DC-LEDGER-10`'s
`strengthened_in += "ENACTMENT-COMMITTEE-FIDELITY"`, the +1 enactment
test (now **14** total), and the further-extended `code_locus` (all
already in the *committed* registry at HEAD `a6b8de7`) — needs only a
TRACEABILITY-row refresh; that grounding ripple is the in-flight working
tree (this HEAD_DELTAS is current). COMMITTEE-CRED-FIDELITY,
DREP-VOTE-FIDELITY, and ENACTMENT-COMMITTEE-FIDELITY all **extended the
same `ci_check_credential_discriminant_closed.sh` for the same
`DC-LEDGER-10`** — no new `ci_script`, the count stays 29.

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
  PHASE4-B2: 5; PHASE4-B3: 2; **PHASE4-B3F: 0** — B3F added no rule; it
  flipped `DC-TXV-06` `partial` → `enforced` and strengthened
  `DC-VAL-06`, both already counted; **PHASE4-B4: 1** — `DC-LEDGER-08`;
  **PHASE4-B5: 1** — `DC-LEDGER-09`; **OQ5: 1** — `DC-LEDGER-10`;
  **COMMITTEE-CRED-FIDELITY: 0** — it added no rule; it *strengthened*
  `DC-LEDGER-10` in place, `strengthened_in += "COMMITTEE-CRED-FIDELITY"`,
  +3 committee tests, `code_locus` extended; **DREP-VOTE-FIDELITY: 0** —
  it also added no rule; it *strengthened* `DC-LEDGER-10` AGAIN,
  `strengthened_in += "DREP-VOTE-FIDELITY"`, +2 DRep tests (11 → 13),
  `code_locus` extended; **ENACTMENT-COMMITTEE-FIDELITY: 0** — it added
  no rule either; it *strengthened* `DC-LEDGER-10` a THIRD time,
  `strengthened_in += "ENACTMENT-COMMITTEE-FIDELITY"`, +1 enactment test
  (13 → 14), `code_locus` extended — the registry total **stays 173**). The two `DC-MEM-*` rules were
  *introduced earlier* (`2047c42`, `status = "declared"`) and were
  flipped to `enforced` in B2, not counted as new.
  - PHASE4-N-A: `DC-CORE-01`, `DC-PROTO-06`.
  - PHASE4-N-B (`d9f0426`): `DC-CONS-03` → `DC-CONS-10` (8 rules).
  - PHASE4-B1 (`c0acd59`, `DC-VAL` family): `DC-VAL-01` → `DC-VAL-06`.
  - PHASE4-B2 (`b79f632`, `DC-TXV` family): `DC-TXV-01` → `DC-TXV-05`.
  - PHASE4-B3 (`3aebbe5`, two new `DC-TXV` rules):
    - **`DC-TXV-06`** — for each era, the certificate-deposit
      classification `map(state, cert)` is a closed, total,
      era-versioned function: every cert variant resolves to exactly one
      of `new_deposit(coin) | refund(coin) | neutral | explicit_reject`,
      coin sourced from the cert (Conway tags 7/8) or the protocol
      parameter (legacy tags 0/1). An unrecognized cert tag, malformed
      cert CBOR, or undecodable withdrawals field is a deterministic
      reject — never a silent neutral. State-dependent cases that cannot
      be accounted reject with `UnsupportedStateDependentDeposit`.
      Incomplete classification is a forbidden false-accept path feeding
      the value-conservation equation.
      Tests: `decode_total_over_tags_0_18`,
      `unknown_cert_tag_is_codec_error`,
      `removed_tag_5_6_is_not_valid_in_conway`,
      `malformed_cert_cbor_rejected`, `decode_is_replay_deterministic`,
      `class_mapping_is_total`,
      `legacy_unregistration_unresolved_is_unsupported_state_dependent`,
      `legacy_unregistration_resolves_recorded_deposit`,
      `pool_reregistration_is_neutral`,
      `pool_new_registration_charges_pool_deposit`.
      code_locus: `ade_codec::conway::cert` (closed `ConwayCert`
      grammar), `ade_codec::error` (`UnknownCertTag`),
      `ade_types::conway::cert` (`ConwayCert`, `CertDisposition`,
      `DepositEffect`, `CoinSource`), `ade_ledger::cert_classify`
      (`classify`), `ade_ledger::error`
      (`UnsupportedStateDependentDepositAccounting`),
      `corpus/conway_certs/*`.
      **Status at HEAD: `enforced`** — flipped from `partial` by
      **PHASE4-B3F-S1** (`d6c1993`), which added the standing CI gate
      `ci/ci_check_conway_cert_classification_closed.sh`. `ci_script`
      now set; `strengthened_in = ["PHASE4-B3", "PHASE4-B3F"]`;
      `cluster = "CL-LEDGER-VERDICT"`.
    - **`DC-TXV-07`** — canonical Conway deposit-parameter authority:
      every deposit/refund amount in the value-conservation equation is
      sourced from canonical ledger state
      (`ProtocolParameters.{key_deposit,pool_deposit}` +
      `LedgerState.conway_deposit_params` / `conway_deposit_view`) and
      never from the testkit RED intermediate, genesis defaults, literal
      constants, or env config. **Status: `enforced`** via
      `ci/ci_check_deposit_param_authority.sh`. cross_ref:
      `T-DET-01`, `T-CONSERV-01`, `CN-LEDGER-07`, `DC-TXV-06`.
  - PHASE4-B4 (`ae1300a`, one new `DC-LEDGER` rule):
    - **`DC-LEDGER-08`** — Conway cert-state accumulation is a closed,
      total, era-versioned transition: at `track_utxo`, certificates
      decode through the era-correct closed grammar (Conway via the
      completed `decode_conway_certs` retaining all owner payloads, tags
      0..18) selected by explicit era dispatch — never the Shelley
      6-variant decoder on Conway bytes, never reduced into the Shelley
      `Certificate`, never with payload fields dropped. Every cert
      resolves to an owner-tagged disposition: it mutates B4-owned
      `CertState` (delegation/pool), or is owner-tagged to
      `ConwayGovState` and routed out-of-mutation-scope (observed, not
      swallowed, not applied), or is a structured reject
      (`NotValidInEra` for removed tags 5/6, `Malformed` for bad CBOR).
      Composite certs (tags 10/12/13) carry both a B4-owned mutation and
      a governance effect; both are represented. No cert is flattened to
      neutral, decode-dropped, or apply-swallowed; a decode/apply error
      propagates as a structured `LedgerError` and halts the block
      transition. Incomplete/best-effort accumulation is a forbidden
      fail-open. (Wiring the owner-tagged `ConwayGovState` effects into
      applied governance state is **PHASE4-B5**, not B4.)
      code_locus: `ade_types::conway::cert` (owner-complete `ConwayCert`,
      `DRep`), `ade_types::shelley::cert` (`PoolRegistrationCert.owners`),
      `ade_codec::conway::cert` (`decode_conway_certs` retention +
      `decode_drep`), `ade_codec::shelley::cert`
      (`read_pool_registration_cert`), `ade_ledger::delegation`
      (`apply_conway_cert` + `ConwayCertAction`/`ConwayCertOutcome`),
      `ade_ledger::rules` (`accumulate_tx_certs` era-dispatch).
      Tests: `each_tag_retains_owner_payloads`, `drep_grammar_total`,
      `conway_cert_action_total`, `apply_outcome_agrees_with_action`,
      `removed_tag_rejects_as_era_invalid`,
      `drep_registration_is_owner_tagged_not_applied`,
      `era_dispatch_conway_accumulates_via_conway_path`,
      `era_dispatch_shelley_accumulates_via_shelley_path`,
      `conway_decode_error_is_fail_closed`,
      `conway_unknown_tag_is_fail_closed`,
      `conway_removed_tag_is_fail_closed`,
      `conway_apply_error_is_fail_closed`,
      `conway_governance_cert_routed_out_of_scope`,
      `positive_synthetic_cert_state_accumulates`,
      `cert_state_replay_byte_identical`, `adversarial_no_false_accept`.
      **Status: `enforced`** via `ci/ci_check_forbidden_patterns.sh`
      (fail-closed swallow-guard); `strengthened_in = ["PHASE4-B4"]`;
      `cluster = "CL-LEDGER-VERDICT"`; `cross_ref = [DC-VAL-06,
      DC-TXV-06, DC-LEDGER-07, T-DET-01, CN-LEDGER-07]`.
      `open_obligation`: the real epoch-576 cert-state-vs-cardano-node
      oracle is environment-blocked (UMap snapshot absent) and
      reclassified per tier doctrine — same constraint as
      `DC-TXV-06`/`DC-TXV-03`.
  - PHASE4-B5 (`fdb6601`, one new `DC-LEDGER` rule):
    - **`DC-LEDGER-09`** — Conway governance-certificate accumulation is
      a closed, total, era-versioned transition into `ConwayGovState`:
      every governance-affecting Conway cert that B4 owner-tagged
      (vote-delegation 9/10/12/13 → `vote_delegations`; committee 14/15
      → `committee_hot_keys`; DRep 16/17/18 → `drep_expiry`) resolves to
      exactly one explicit `ConwayGovState` mutation or a structured
      reject — never observed-and-dropped, never flattened, never with
      payload lost. B5 mutates only governance-owned fields, never the
      B4-owned `CertState`, and does not double-apply the
      delegation/pool half of composites (10/12/13). DRep expiry is
      computed only from an explicit gov-env (`current_epoch +
      drep_activity` via `checked_add`); a missing required env input is
      a structured fail-fast, an overflow is a deterministic
      `DRepActivityOverflow`, never a defaulted or wrapped expiry. A
      cert that cannot be applied propagates a structured `LedgerError`
      and halts the block transition; incomplete/best-effort
      accumulation is a forbidden fail-open. `ConwayGovState` becomes a
      deterministic function of (boundary-loaded base then replayed
      block-stream cert effects) rather than a frozen snapshot — a
      deliberate, oracle-confirmable fingerprint migration (`T-DET-01`,
      the Conway-deposit tag 2→3 `drep_activity` extension).
      code_locus: `ade_ledger::gov_cert` (`apply_conway_gov_cert`,
      native dispatch total over 18 tags — B5-S2),
      `ade_ledger::state` (`GovCertEnv` + `gov_cert_env()` fail-fast),
      `ade_ledger::pparams` (`ConwayOnlyDepositParams.drep_activity`),
      `ade_ledger::error`
      (`ValidationEnvironmentError::{MissingDRepActivityParam,DRepActivityOverflow}`)
      — B5-S1; `ade_ledger::rules`
      (`accumulate_tx_certs`/`process_block_certificates` thread
      `Option<ConwayGovState>`, apply the gov half, carry `gov_state`
      forward through `apply_block` — replaces the B4 observe-and-drop)
      — B5-S3; `ade_ledger::fingerprint` (gov-state + `drep_activity`
      fingerprint surface).
      Tests: `gov_apply_total_over_18_tags`,
      `composite_gov_half_applied_once_certstate_untouched_by_b5`,
      `drep_expiry_uses_epoch_plus_activity`,
      `env_free_gov_certs_need_no_env`,
      `drep_register_missing_env_is_fail_fast`,
      `drep_expiry_overflow_is_fail_closed`, `gov_apply_is_deterministic`,
      `gov_cert_env_present_ok`,
      `gov_cert_env_missing_drep_activity_is_fail_fast`,
      `gov_accumulation_applies_drep_registration_into_gov_state`,
      `gov_apply_error_halts_accumulation`,
      `positive_synthetic_gov_state_accumulates`,
      `gov_state_accumulation_replays_byte_identical`,
      `adversarial_drep_register_update_missing_env_rejected`,
      `adversarial_drep_expiry_overflow_rejected`,
      `adversarial_decode_layer_rejects_guard_gov_path`,
      `adversarial_double_resign_is_deterministic`.
      **Status: `enforced`** via
      `ci/ci_check_gov_cert_accumulation_closed.sh`;
      `strengthened_in` (on `DC-LEDGER-08`) `+= "PHASE4-B5"`;
      `cluster = "CL-LEDGER-VERDICT"`; `cross_ref = [DC-LEDGER-08,
      DC-VAL-06, DC-TXV-06, T-DET-01, CN-LEDGER-07]`.
      `open_obligation`: the real epoch-576 governance-state
      (VState)-vs-cardano-node oracle is environment-blocked
      (UMap/ledger snapshot absent) and reclassified per tier doctrine —
      same constraint as `DC-LEDGER-08`/`DC-TXV-06`/`DC-TXV-03`.
      Declared separable follow-ups: **OQ-3** (the GOVCERT
      committee-membership tx-validity gate, not part of this
      accumulation closure) and **OQ-5** (a pre-existing decoder
      behavior promoted to authority by B5 — `ConwayGovState` keys on
      bare `Hash28` because the codec collapses the key/script
      credential discriminant, so a 28-byte-colliding key-hash and
      script-hash are indistinguishable in the gov maps; carrying the
      discriminant into `StakeCredential`/`DRep` is a separate
      cross-cutting follow-up).
  - OQ5 (`959e16c`, one new `DC-LEDGER` rule):
    - **`DC-LEDGER-10`** — credential key/script discriminant fidelity:
      a stake/committee/DRep credential is a closed sum over
      `{KeyHash, ScriptHash}` of a 28-byte hash, never a tag-erased
      `Hash28`. Both era certificate decoders preserve the key/script
      discriminant (unknown tag → deterministic reject); the credential
      type makes a discriminant-less credential **unrepresentable** on
      the BLUE authority path (no `Hash28` → credential coercion);
      `CertState` (registrations/delegations/rewards) and
      `ConwayGovState` (vote_delegations/committee_hot_keys/drep_expiry)
      key on the discriminated credential, so a key-hash and a
      script-hash sharing 28 bytes are distinct authoritative-state keys
      (matching cardano-node's `Credential`-keyed UMap/VState); and the
      canonical fingerprint serializes the discriminant. The
      discriminated representation is a deliberate cert-state + gov-state
      fingerprint migration (`T-DET-01`). Default scope is Shelley+;
      Byron is a declared non-goal.
      code_locus: `ade_types::shelley::cert` (`StakeCredential` enum
      `{KeyHash,ScriptHash}` + `hash()`), `ade_codec::{shelley,conway}::cert`
      (`decode_stake_credential` preserves the tag, rejects unknown),
      `ade_ledger::state` (`ConwayGovState` re-keyed to `StakeCredential`),
      `ade_ledger::{gov_cert,governance,cert_classify,rules}`,
      `ade_ledger::fingerprint` (`write_stake_credential` emits
      discriminant+hash), `ade_testkit::harness::snapshot_loader` (GREEN:
      gov-map + DRep-reg parses preserve the tag).
      Tests: `shelley_credential_preserves_discriminant`,
      `conway_credential_preserves_discriminant`,
      `unknown_credential_tag_rejects`, `discriminant_changes_fingerprint`,
      `keyhash_scripthash_same_bytes_are_distinct_certstate`,
      `keyhash_scripthash_same_bytes_are_distinct_govstate`,
      `discriminant_changes_fingerprint_corpus`,
      `credential_accumulation_replays_byte_identical`,
      `committee_keyhash_scripthash_do_not_cross_resolve`,
      `committee_keyhash_scripthash_same_bytes_distinct`,
      `committee_discriminant_changes_fingerprint`,
      `drep_keyhash_scripthash_do_not_cross_resolve`,
      `drep_vote_discriminant_changes_fingerprint`,
      `enactment_committee_changes_keyhash_scripthash_distinct` (14 total
      — 8 OQ5 + 3 COMMITTEE-CRED + 2 DREP-VOTE + 1
      ENACTMENT-COMMITTEE).
      **Status: `enforced`** via
      `ci/ci_check_credential_discriminant_closed.sh`; strengthens
      `T-DET-01` / `T-ENC-03`; `cross_ref = [T-DET-01, T-ENC-03,
      DC-LEDGER-08, DC-LEDGER-09, DC-TXV-05]`;
      `cluster = "CL-LEDGER-VERDICT"`.
      `open_obligation`: real-chain agreement of the discriminated
      credential keys vs cardano-node's `Credential`-keyed UMap/VState is
      environment-blocked (epoch-576/boundary snapshots absent locally)
      and reclassified per tier doctrine — same posture as
      `DC-LEDGER-08`/`DC-LEDGER-09`/`DC-TXV-06`. Declared non-goals
      (separate work): withdrawal / required-signer / address credential
      discriminant, the `Hash28`-keyed stake-distribution snapshot, and
      the Byron credential surface.
- Removals: **0** (expected under append-only discipline; clean).
- Strengthenings (`declared`/`partial` → `enforced`, or tightened):
  - **`DC-LEDGER-08`** (B5, `fdb6601`/`d63c700`): strengthened — B4's
    "routed out-of-mutation-scope" disposition for governance certs is
    **retired**; those certs are now applied (`DC-LEDGER-09` is the
    application rule). `strengthened_in += "PHASE4-B5"`; `cross_ref`
    extended to `DC-LEDGER-09`. (Note: in the registry the
    `strengthened_in += "PHASE4-B5"` edit is recorded on `DC-LEDGER-09`'s
    cross-link rather than mutating `DC-LEDGER-08`'s array; the
    relationship is captured via `cross_ref`. See Anomalies.)
  - **`T-DET-01` / `T-ENC-03`** (OQ5, `4187330`/`a3ee2da`):
    strengthened — the canonical fingerprint and credential encoding now
    carry the key/script discriminant (`write_stake_credential` emits
    discriminant+hash; `StakeCredential` is a closed sum), so two states
    differing only in a credential's tag fingerprint differently.
    `DC-LEDGER-10` records the relationship via `cross_ref`; no golden
    drift (affected surfaces empty/credential-free in committed states).
  - **`DC-LEDGER-10`** (COMMITTEE-CRED-FIDELITY, `2303a60`/`2aeea16`):
    strengthened — the credential-fidelity rule is **extended to the
    committee surfaces** OQ5 left at the hash level:
    `ConwayGovState.committee` re-keyed `Hash28` → `StakeCredential`,
    `GovActionState.committee_votes` re-typed to carry `StakeCredential`,
    committee ratification by full-credential equality (no `.hash()`),
    and the `write_committee_vote_list` fingerprint writer.
    `strengthened_in += "COMMITTEE-CRED-FIDELITY"`; `tests` += the three
    committee tests (`committee_keyhash_scripthash_do_not_cross_resolve`,
    `committee_keyhash_scripthash_same_bytes_distinct`,
    `committee_discriminant_changes_fingerprint`); `code_locus` extended;
    `ci_script` unchanged (the existing
    `ci_check_credential_discriminant_closed.sh` was extended with two
    committee-surface clauses). No new rule, no new CI script; **registry
    total stays 173, CI count stays 29.** No golden drift (committee
    states empty in the committed fingerprint surfaces).
  - **`DC-LEDGER-10`** (DREP-VOTE-FIDELITY, `ba4ff37`/`62c9020`):
    strengthened **again** — the credential-fidelity rule is **extended
    to the DRep-vote surface**, the last governance vote surface left at
    the hash level: `GovActionState.drep_votes` re-typed to carry
    `StakeCredential`, and `governance.rs` `lookup_stake` now resolves a
    DRep voter to the **exact** `DRep` variant (the key/script OR-fallback
    over identical bytes is removed). `spo_votes` stays bare `Hash28`
    (pools are always key-hash — a permanent non-goal).
    `strengthened_in += "DREP-VOTE-FIDELITY"`; `tests` += the two DRep
    tests (`drep_keyhash_scripthash_do_not_cross_resolve`,
    `drep_vote_discriminant_changes_fingerprint`) → 13 total;
    `code_locus` extended; `ci_script` unchanged (the existing
    `ci_check_credential_discriminant_closed.sh` was extended with two
    DRep-surface clauses). No new rule, no new CI script; **registry
    total stays 173, CI count stays 29.** No golden drift (the
    gov-action-state vote surfaces are empty in the committed states).
  - **`DC-LEDGER-10`** (ENACTMENT-COMMITTEE-FIDELITY, `a6b8de7`):
    strengthened **a third time** — the credential-fidelity rule is
    **extended to the (dormant) committee-enactment effect surface**, the
    last bare-`Hash28` committee-credential surface:
    `EnactmentEffects.committee_changes` re-typed
    `Option<(Vec<Hash28>, Vec<(Hash28, u64)>)>` →
    `Option<(Vec<StakeCredential>, Vec<(StakeCredential, u64)>)>` so
    committee enactment, once wired, cannot re-collapse the discriminated
    `ConwayGovState.committee` map on write-back. The field stays dormant
    (`None`); this pins the type, not live behavior.
    `strengthened_in += "ENACTMENT-COMMITTEE-FIDELITY"`; `tests` += the one
    enactment test (`enactment_committee_changes_keyhash_scripthash_distinct`)
    → 14 total; `code_locus` extended; `ci_script` unchanged (the existing
    `ci_check_credential_discriminant_closed.sh` was extended with one
    committee-enactment-effect clause). No new rule, no new CI script;
    **registry total stays 173, CI count stays 29.** No golden drift (the
    field is dormant `None`; no fingerprint surface change).
  - **`DC-TXV-06`** (B3F, `d6c1993`): `partial` → **`enforced`** — the
    closed cert-deposit classification now carries a standing CI
    grep-gate, not only exhaustive-match + tests.
    `strengthened_in += "PHASE4-B3F"`.
  - **`DC-VAL-06`** (B3F, `193d2fc`; B4, `302d22c`): the
    fail-closed-field rule strengthened — the Conway cert decoder
    rejects trailing bytes and bounds preallocation (B3F), and the
    cert-state accumulation path now propagates decode/apply errors
    instead of swallowing them "non-fatal during replay" (B4);
    `strengthened_in = ["PHASE4-B1", "PHASE4-B2", "PHASE4-B3",
    "PHASE4-B3F", "PHASE4-B4"]`. (Also strengthened in B3 with the
    fail-closed deposit/refund accounting on the body-validity path.)
  - **`T-CONSERV-01` / `CN-LEDGER-07`** (B3, `978c222`): the
    preservation-of-value invariant is strengthened from the
    deposit-free form to the **full** Conway equation
    (`Σ(inputs)+Σ(withdrawals)+refunded_deposits ==
    Σ(outputs)+fee+donation+new_deposits`); the cert/withdrawal
    early-out is removed. `strengthened_in = ["PHASE4-B3"]`,
    `cross_ref` extended to `DC-TXV-06`/`DC-TXV-07`.
  - **`DC-TXV-03`** (B3): the no-false-accept verdict-agreement rule's
    `tests` extended with the conservation corpora — real epoch-576
    positive (10 cert/withdrawal txs Valid), synthetic positive, and
    adversarial value-imbalance cases; `cross_ref` extended to
    `DC-TXV-06`.
  - **`DC-MEM-01`, `DC-MEM-02`** (B2, `85a50dc`): `declared` →
    `enforced`. **`DC-TXV-03`, `DC-VAL-06`, `DC-LEDGER-02`** previously
    strengthened by B2-S4 (`617139f`).
  - Earlier-delta strengthenings (unchanged): `DC-EPOCH-02`
    (`9b15378`); the N-D bundle (`78da6c9`); the N-A real-capture
    bundle; `T-CORE-02` (S-B1).
  - The six `DC-VAL-*` and five `DC-TXV-01..05` rules each record
    `strengthened_in` containing their introducing cluster — recorded
    faithfully; see Anomalies.

Family counts at HEAD: CN dominates (~64), DC grew most across the delta
(now including `DC-CONS` ×8, `DC-VAL` ×6, `DC-TXV` ×7, `DC-MEM` ×2, and
the three new `DC-LEDGER-08`/`DC-LEDGER-09`/`DC-LEDGER-10` joining the
existing `DC-LEDGER` rules), T = 30, RO/OP combined ×9.

Normative-doc rule extraction (the `normative_docs` list in
`.idd-config.json`) is approximate and not regenerated here — the
structured registry is the authoritative source.

---

## Anomalies and Cross-Reference Warnings

- **ENACTMENT-COMMITTEE-FIDELITY closed (`a6b8de7`) — DREP-VOTE
  carry-forward follow-up (d) CLOSED.** The last bare-`Hash28`
  committee-credential surface is now discriminated *preventively*:
  `EnactmentEffects.committee_changes` (`ade_ledger::governance`) is
  re-typed `Option<(Vec<Hash28>, Vec<(Hash28, u64)>)>` →
  `Option<(Vec<StakeCredential>, Vec<(StakeCredential, u64)>)>`, so
  committee enactment, once wired, cannot re-collapse the discriminated
  `ConwayGovState.committee` map on write-back. The field is **DORMANT**
  (always `None`; `UpdateCommittee` enactment is still a no-op) — this is
  a type pin, not a behavior change. **No new module, no new crate, no new
  rule, no new CI script.** `DC-LEDGER-10` is **STRENGTHENED a third time**
  in place (`strengthened_in += "ENACTMENT-COMMITTEE-FIDELITY"`; +1
  enactment test → 14; `code_locus` extended); the existing
  `ci_check_credential_discriminant_closed.sh` was **extended** with one
  committee-enactment-effect clause — **registry total stays 173, CI count
  stays 29.** **No golden drift** (the field is dormant `None`; no
  fingerprint surface change).
- **ENACTMENT-COMMITTEE-FIDELITY carry-forward follow-ups (out of scope).**
  With (d) resolved, the only remaining DREP-VOTE follow-up is **(e)** the
  GREEN loader `mk_credential` `tag != 1` → `KeyHash` default — contained
  to `ade_testkit` (cannot reach the node binary). The pre-OQ5 **(b)**
  Shelley unknown-cert zero-hash placeholder also remains a WARN LOW
  non-goal. Both are unchanged by this cluster.
- **DREP-VOTE-FIDELITY closed (`62c9020`) — COMMITTEE-CRED DRep-vote
  follow-up (c) CLOSED.** The DRep-vote surface COMMITTEE-CRED left at the
  hash level is now discriminated: `GovActionState.drep_votes` is re-typed
  `Vec<(Hash28, Vote)>` → `Vec<(StakeCredential, Vote)>`
  (`ade_types::conway::governance`), and `governance.rs` `lookup_stake`
  resolves a DRep voter to **exactly one** `DRep` stake key by its
  discriminant — the prior key/script `.or_else(…ScriptHash…)` OR-fallback
  over identical bytes is gone, so a key-hash voter never tallies a
  script-hash DRep's stake of equal bytes. `spo_votes` stays bare
  `Hash28` (pools are always key-hash — a permanent non-goal). **No new
  module, no new crate, no new rule, no new CI script.** `DC-LEDGER-10` is
  **STRENGTHENED AGAIN** in place (`strengthened_in += "DREP-VOTE-FIDELITY"`;
  +2 DRep tests → 13; `code_locus` extended); the existing
  `ci_check_credential_discriminant_closed.sh` was **extended** with two
  DRep-surface clauses — **registry total stays 173, CI count stays 29.**
- **DREP-VOTE-FIDELITY fingerprint change (T-DET-01, deliberate; no golden
  drift).** `write_committee_vote_list` is **renamed
  `write_credential_vote_list`** and now serves both `committee_votes` and
  `drep_votes` (`drep_votes` previously used `write_vote_list` over
  `Hash28`); `spo_votes` stays `write_vote_list`. As with OQ5 and
  COMMITTEE-CRED, **no golden was regenerated**: the gov-action-state vote
  surfaces are empty in the committed states, so they stay byte-identical.
  Confirm `ci_check_ledger_determinism.sh` / the fingerprint golden test
  pass on the next determinism replay.
- **DREP-VOTE-FIDELITY carry-forward follow-ups.** At the DREP-VOTE close,
  two COMMITTEE-CRED follow-ups remained: **(d) `EnactmentEffects.committee_changes`**
  (then still bare `Hash28`, DORMANT — flagged as MUST-migrate before
  committee enactment) and **(e)** the GREEN loader `mk_credential`
  `tag != 1` → `KeyHash` default (contained to `ade_testkit`). **(d) has
  since been RESOLVED by ENACTMENT-COMMITTEE-FIDELITY** (see the top of
  this section); **(e)** is unchanged. The pre-OQ5 **(b)** Shelley
  unknown-cert zero-hash placeholder also remains a WARN LOW non-goal.
- **Prior narrated HEAD reconciliation (no anomaly — expected).** The
  prior HEAD_DELTAS narrated HEAD `62c9020` (the DREP-VOTE-FIDELITY-S2
  *implementation* commit). The DREP-VOTE-FIDELITY *close* commit — the
  grounding-doc refresh `06f517f` — landed immediately after and is now in
  §1 (it was not in the prior cut because that regen was current as of
  `62c9020`). The two ENACTMENT-COMMITTEE-FIDELITY commits (`5d64fee`,
  `a6b8de7`) sit on top of `06f517f`. No history rewrite; the span
  `62c9020..a6b8de7` is 3 commits (the DREP-VOTE close + the 2 enactment
  commits). (A prior agent's note claiming the doc read `ee35493` was
  incorrect — `ee35493` appears only in older historical narration blocks
  referring to the B4-level regen baseline, never as the narrated HEAD on
  the header line.)
- **COMMITTEE-CRED-FIDELITY closed (`2aeea16`) — OQ5 committee
  credential-discrimination follow-up (a) CLOSED.** The two committee
  surfaces OQ5 left at the hash level are now discriminated:
  `ConwayGovState.committee` is re-keyed `Hash28` → `StakeCredential`
  (`ade_ledger::state`), `GovActionState.committee_votes` is re-typed
  `Vec<(Hash28, Vote)>` → `Vec<(StakeCredential, Vote)>`
  (`ade_types::conway::governance`), and committee ratification resolves
  hot-voter / hot→cold / cold-member by **full-credential equality** (the
  `.hash()` comparisons in `ade_ledger::governance` are gone), so a
  key-hash hot key never cross-resolves to a script-hash member of equal
  bytes. **No new module, no new crate, no new rule, no new CI script.**
  `DC-LEDGER-10` is **STRENGTHENED** in place
  (`strengthened_in += "COMMITTEE-CRED-FIDELITY"`; +3 committee tests;
  `code_locus` extended); the existing
  `ci_check_credential_discriminant_closed.sh` was **extended** with two
  committee-surface clauses — **registry total stays 173, CI count stays
  29.**
- **COMMITTEE-CRED-FIDELITY fingerprint change (T-DET-01, deliberate; no
  golden drift).** `write_committee_vote_list` (canonical, sorts
  committee votes by the discriminated credential's `Ord`) replaces
  `write_vote_list` for `committee_votes`, and the committee-member map
  writer routes through `write_stake_credential`. As with OQ5, **no
  golden was regenerated**: the committee fingerprint surfaces are empty
  in the committed states, so they stay byte-identical. Confirm
  `ci_check_ledger_determinism.sh` / the fingerprint golden test pass on
  the next determinism replay.
- **COMMITTEE-CRED-FIDELITY carry-forward follow-ups.** Three follow-ups
  from the per-cluster security review were NOT part of the
  COMMITTEE-CRED closure: **(c) DRep-vote discrimination** — was
  `ade_ledger::governance` `lookup_stake` doing a key/script OR-fallback
  over identical bytes for `drep_votes` (DReps can be script, pools
  cannot); **(c) is now CLOSED by DREP-VOTE-FIDELITY** (`62c9020`, see the
  top anomaly). **(d)
  `EnactmentEffects.committee_changes`** is still a bare `Hash28` but
  **DORMANT** (UpdateCommittee enactment is a no-op) — it MUST be migrated
  to `StakeCredential` **before committee enactment is implemented**, or
  it would re-collapse the committee discriminant on write-back. **(e)**
  the GREEN loader `mk_credential` defaults an unknown `tag != 1` to
  `KeyHash` — contained to `ade_testkit` (cannot reach the node binary);
  could reject unknown tags instead. `drep_votes` / `spo_votes` remaining
  `Hash28`-typed is the type-level expression of (c).
- **OQ5-CREDENTIAL-FIDELITY closed (`a3ee2da`) — OQ-5 credential
  discriminant collapse CLOSED.** The B5-named OQ-5 collapse is closed:
  `StakeCredential` is now a closed `enum { KeyHash, ScriptHash }` (was
  the tuple struct `StakeCredential(pub Hash28)`); both era
  `decode_stake_credential` preserve the tag (unknown → reject);
  `ConwayGovState` is re-keyed `Hash28` → `StakeCredential`; and the
  fingerprint `write_stake_credential` emits discriminant+hash.
  `DC-LEDGER-10` introduced and `enforced` (CI guard
  `ci_check_credential_discriminant_closed.sh`, the 29th script);
  strengthens `T-DET-01` / `T-ENC-03`.
- **OQ5 fingerprint migration (T-DET-01, deliberate; no golden drift).**
  `write_stake_credential` now emits the key/script discriminant ahead of
  the hash — a deliberate dual cert-state + gov-state fingerprint
  migration. Unlike the B5 2→3 array migration, **no golden was
  regenerated**: the gov/cert fingerprint surfaces that route through the
  new writer are empty/credential-free in the committed states, so those
  states stay byte-identical. Confirm `ci_check_ledger_determinism.sh` /
  the fingerprint golden test pass on the next determinism replay.
- **OQ5 real discriminated-key oracle environment-blocked
  (reclassified).** The real-chain agreement of the discriminated
  credential keys vs cardano-node's `Credential`-keyed UMap/VState could
  not run — the epoch-576/boundary UMap/ledger snapshot is absent
  (recoverable from the ImmutableDB EBS snapshots per
  `DC-LEDGER-10.open_obligation`). Reclassified per tier doctrine, the
  **same constraint as `DC-LEDGER-08`/`DC-LEDGER-09`/`DC-TXV-06`**.
  Mechanical closure shipped instead: closed-sum credential type +
  tag-preserving decoders + discriminated gov-state keys +
  discriminant-folding fingerprint + corpus
  (`credential_fidelity_corpus.rs`, `shelley_credential_discriminant.rs`).
- **OQ5 per-cluster security-review follow-ups.** Two separable
  follow-ups were surfaced: **committee member / vote credential
  discrimination** (WARN MEDIUM) — **now CLOSED by
  COMMITTEE-CRED-FIDELITY** (`2aeea16`, see the top anomaly) — and the
  **Shelley unknown-cert zero-hash placeholder** (WARN LOW — the Shelley
  `decode_single_certificate`
  fallback still returns `Certificate::StakeRegistration(StakeCredential::KeyHash(Hash28([0u8;28])))`
  for unrecognized tags; it is `apply_cert`-ignored but is a tag-erased
  placeholder). Declared non-goals: withdrawal / required-signer /
  address credential, the `Hash28`-keyed stake-distribution snapshot, and
  Byron.
- **PHASE4-B5 closed (`651adc9`) — B4 governance observe-and-drop CLOSED.**
  The governance-affecting Conway certs B4 owner-tagged to
  `ConwayGovState` but routed out of mutation scope are now **applied**:
  the new BLUE module `ade_ledger::gov_cert` (`apply_conway_gov_cert`,
  total over 18 tags) mutates governance-owned fields only, and
  `accumulate_tx_certs`/`process_block_certificates` thread an
  `Option<ConwayGovState>` and carry `gov_state` forward through
  `apply_block` (it was nulled to `None` at every block apply before
  B5). `DC-LEDGER-09` introduced and `enforced` (CI guard
  `ci_check_gov_cert_accumulation_closed.sh`); strengthens
  `DC-LEDGER-08` (retires its "routed out-of-mutation-scope"
  disposition for governance certs).
- **B5 fingerprint migration (T-DET-01, deliberate).** The state
  fingerprint's Conway-deposit tag array was extended **2→3 fields** to
  fold the new `drep_activity` parameter; the combined golden was
  regenerated (`b69422ef…71d9` → `d1803cb7…8827`). This is an
  intentional, oracle-confirmable migration — **not** a determinism
  regression: pre-Conway and `conway_deposit_params == None` states emit
  nothing at this tag and keep their pre-B5 fingerprint byte-identical.
  Confirm `ci_check_ledger_determinism.sh` / the fingerprint golden test
  pass on the next determinism replay (the golden assertion is in
  `fingerprint.rs`).
- **B5 real governance-state oracle environment-blocked (reclassified).**
  The real epoch-576 governance-state (VState)-vs-cardano-node oracle
  could not run — the epoch-576 UMap/ledger snapshot is absent
  (`corpus/gov_state/README.md`, `corpus/cert_state/README.md`,
  `corpus/validity/conway_epoch576/README.md`). Reclassified per tier
  doctrine, the **same environment constraint as
  `DC-LEDGER-08`/`DC-TXV-06`/`DC-TXV-03`**. Mechanical closure shipped
  instead: native total gov dispatch + env fail-fast + checked
  DRep-expiry arithmetic (`DRepActivityOverflow`) + block-path
  accumulation (gov_state carried forward, observe-and-drop removed) +
  synthetic positive / replay-byte-identical / adversarial corpus
  (`gov_state_corpus.rs`). Not a coverage gap in the transition itself.
- **B5 declared separable follow-ups: OQ-3 and OQ-5.** **OQ-3** — the
  GOVCERT committee-membership tx-validity gate is *not* part of this
  accumulation closure; it is a declared follow-up. **OQ-5** — a
  pre-existing decoder behavior **promoted to authority by B5** (NOT
  introduced here): `ConwayGovState` keys gov state on bare `Hash28`
  because the codec collapses the key/script credential discriminant
  (`ade_codec` `decode_stake_credential` drops the type tag), so a
  key-hash and a script-hash credential sharing 28 bytes are
  indistinguishable in `vote_delegations`/`committee_hot_keys`/`drep_expiry`,
  whereas cardano-node keys on the full discriminated `Credential`. A
  28-byte collision is not cheaply forgeable; carrying the discriminant
  into `StakeCredential`/`DRep` is a separate cross-cutting follow-up
  (touches the whole credential surface), tracked in `DC-LEDGER-09`'s
  `open_obligation` per the per-cluster security review.
- **`DC-LEDGER-08` strengthening recorded via `cross_ref`, not
  `strengthened_in`.** B5 strengthens `DC-LEDGER-08` (retiring its
  governance observe-and-drop disposition), but the registry records the
  relationship via the bidirectional `cross_ref` between `DC-LEDGER-08`
  and the new `DC-LEDGER-09` rather than appending `"PHASE4-B5"` to
  `DC-LEDGER-08.strengthened_in` (which stays `["PHASE4-B4"]`). Harmless
  (no weakening; the application rule is fully recorded under
  `DC-LEDGER-09`), but consider appending `strengthened_in += "PHASE4-B5"`
  on the next registry curation pass for consistency with the
  `DC-VAL-06` convention.
- **PHASE4-B4 closed (`ee35493`) — cert-state accumulation fail-open
  CLOSED.** The fail-open that B3 left on the block-level
  cert-accumulation path (the `_era` discard plus the two "non-fatal
  during replay" swallows in `process_block_certificates`) is removed:
  `accumulate_tx_certs` era-dispatches and propagates decode/apply
  errors as structured `LedgerError`, the Conway decoder is now
  owner-complete (no payload dropping), and the apply model is total
  over all 18 tags. `DC-LEDGER-08` introduced and `enforced` (CI guard
  in `ci_check_forbidden_patterns.sh`); strengthens `DC-VAL-06`.
- **B4 governance certs deferred to PHASE4-B5 (declared follow-up).**
  Vote-delegation, committee auth/resign, and DRep reg/unreg/update
  certs are decoded and **owner-tagged to `ConwayGovState`** but routed
  **out of B4 mutation scope** — observed, not applied. This is a
  deliberate, declared boundary (`DC-LEDGER-08` names it explicitly),
  not a gap: wiring the owner-tagged governance effects into applied
  governance state is **PHASE4-B5 (Conway governance-certificate
  accumulation authority)**. Composite tags 10/12/13 already represent
  both their B4-owned cert-state mutation and their governance effect.
- **B4 real cert-state oracle environment-blocked (reclassified).** The
  real epoch-576 cert-state-vs-cardano-node oracle could not run — the
  epoch-576 UMap snapshot is absent (`corpus/cert_state/README.md`,
  `corpus/validity/conway_epoch576/README.md`). Reclassified per tier
  doctrine, the **same environment constraint as `DC-TXV-06`/`DC-TXV-03`**.
  Mechanical closure shipped instead: owner-complete decode + total
  owner-tagged apply + era-dispatched fail-closed accumulation +
  synthetic positive / replay-byte-identical / adversarial corpus
  (`cert_state_corpus.rs`, `conway_cert_decode_complete.rs`). Not a
  coverage gap in the transition itself.
- **Grounding docs partially stale (COMMITTEE-CRED-FIDELITY).** CODEMAP,
  SEAMS, and TRACEABILITY were *committed*-refreshed through the OQ5 close
  `676af5a` (N-B + B1 + B2 + B3 + B3F + B4 + B5 + OQ5 rows, incl. the
  `DC-LEDGER-10` row and the `StakeCredential` enum / tag-preserving
  decoders / re-keyed `ConwayGovState` / discriminant-folding
  fingerprint). The **COMMITTEE-CRED-FIDELITY edits** are the **in-flight
  working tree**: CODEMAP must record the committee re-keying
  (`ConwayGovState.committee` `Hash28` → `StakeCredential` in
  `ade_ledger::state`; `GovActionState.committee_votes` carrying
  `StakeCredential` in `ade_types::conway::governance`), the
  full-credential-equality committee ratification (`ade_ledger::governance`),
  and the `write_committee_vote_list` fingerprint writer
  (`ade_ledger::fingerprint`); SEAMS must record that the committee
  credential surfaces are now closed sums; TRACEABILITY must refresh the
  `DC-LEDGER-10` row (`strengthened_in += "COMMITTEE-CRED-FIDELITY"`, the
  +3 committee tests, the extended `code_locus`) — **no new row, no new
  `ci_script`**; the rule and the extended CI script already exist. **This
  HEAD_DELTAS is current.** Run `/codemap`, `/seams`, `/traceability` and
  commit alongside the COMMITTEE-CRED-FIDELITY close.
- **PHASE4-B3 closed (`d766eb0`); PHASE4-B3F follow-up committed
  (`193d2fc`).** The prior regen's "B3 close in flight" anomaly is
  **resolved** — the `Close PHASE4-B3` commit landed, archived the
  `docs/clusters/PHASE4-B3/*` slice docs to `docs/clusters/completed/`,
  and committed the B3 grounding-doc refresh. The B3F follow-up then
  shipped as a 2-slice arc (`d6c1993` B3F-S1, `193d2fc` B3F-S2).
- **`DC-TXV-06` `partial` → `enforced` — RESOLVED.** The prior regen
  flagged `DC-TXV-06` as `partial` with the dedicated grep-gate CI named
  as a follow-up. **B3F-S1 (`d6c1993`) discharges that follow-up**: the
  registry at HEAD has `status = "enforced"`, `ci_script =
  "ci/ci_check_conway_cert_classification_closed.sh"`, and
  `strengthened_in = ["PHASE4-B3", "PHASE4-B3F"]`. The
  committed-vs-working-tree `DC-TXV-06` disagreement from the prior
  regen is gone. A future refactor reintroducing an open-tail or
  silent-neutral classification arm is now caught by a standing CI
  invariant, not just tests.
- **Conway value-conservation gap CLOSED for cert/withdrawal txs (B3,
  `978c222`).** The deposit/refund/withdrawal accounting that B2-S4
  deliberately deferred (the early-out in `check_conway_coin_conservation`)
  is removed: the full equation is enforced, sourcing every term from
  canonical ledger state via `cert_classify::classify` over the closed
  `ConwayCert` grammar. The named tx-validity-completeness follow-up
  from the prior regen is **discharged for Conway**. Confirm
  `ci_check_differential_divergence.sh` / `ci_check_ledger_determinism.sh`
  still cover the full equation on the next TRACEABILITY pass; the
  `fingerprint.rs` Conway deposit-param fold is byte-identical for
  pre-Conway state (verify in the determinism replay).
- **Conway cert decoder strictness (B3F-S2, `193d2fc`).** The decoder
  now rejects trailing bytes (`CodecError::TrailingBytes`) and bounds
  definite-array preallocation by `data.len()`. No behavioral change for
  valid input (the cert field is an exact CBOR item); the change closes
  a malformed-trailing-input acceptance edge and a crafted-huge-count
  over-allocation. Strengthens `DC-VAL-06`.
- **DC-VAL status mismatch vs. closure claim (B1, carried forward).**
  PHASE4-B1 is reported fully closed, but in the registry only
  `DC-VAL-01` is `enforced` — `DC-VAL-02` → `DC-VAL-05` remain
  `declared` despite named tests and the extended closed-enums
  enforcement point (`DC-VAL-06` is `enforced`). Flip on the next
  `/traceability` pass.
- **`strengthened_in` records the introducing cluster on freshly-created
  rules.** Each `DC-VAL-*` records `["PHASE4-B1"]` and each
  `DC-TXV-01..05` records `["PHASE4-B2"]` even though those clusters
  *created* the families. Harmless (no weakening), but consider
  normalizing on the next registry curation pass.
- **`ade_ledger -> ade_core` dependency edge (B1, carried forward).**
  First ledger→consensus edge. Both BLUE, so the BLUE→RED guard is
  unaffected; B3 and B3F added no new manifest edge.
- **`ade_crypto::kes::build_opcert_signable` fixed in B1-S5
  (`500589b`).** BLUE crypto-surface behavioral change; confirm
  `ci_check_crypto_vectors.sh` still covers it on the next
  TRACEABILITY pass.
- **B3 positive corpus carves out Plutus per CE-88.** The real
  epoch-576 positive conservation corpus
  (`conway_conservation_positive_corpus.rs`) drives **10 non-Plutus**
  cert/withdrawal txs to `Valid` at `track_utxo=true`; Plutus-witnessing
  txs are excluded because CE-88 (Conway `validity_range` aiken bug) is
  externally blocked. Intentional carve-out, not a coverage gap in the
  conservation equation itself.
- **Adversarial corpora are derived, not committed.** `corpus/validity/`
  (B1), `corpus/tx_validity/` (B2), and the B3 adversarial conservation
  cases are generated deterministically at test time by the mutators in
  `ade_testkit`. The B3 *positive* oracle, by contrast, is committed
  (`corpus/validity/conway_epoch576/resolved_inputs.json`,
  `resolution_set.txt`) because it is a real on-chain input-resolution
  set, not a derivable mutation.
- **PHASE4-N-A / N-B / N-D / B1 / B2 / B3 / B4 / B5 / OQ5 /
  COMMITTEE-CRED-FIDELITY closed.** N-A `69a2862` (+`744ef34`); N-B
  `a0c73e1`; N-D `436b1d7`; B1 `993f363`; B2 `c1cba82`; B3 `d766eb0`; B4
  `644eb03` (grounding refresh; B4-S5 corpus `ee35493` closes the
  implementation arc); B5 `f81f815` (grounding refresh; B5-S5 `651adc9`
  closes the implementation arc); OQ5 `676af5a` (grounding refresh; OQ5-S2
  corpus + CI gate `a3ee2da` closes the implementation arc);
  COMMITTEE-CRED-FIDELITY `2aeea16` (S2 negative corpus + CI-gate
  extension closes the arc). B3F follow-up committed (`193d2fc`); the
  COMMITTEE-CRED-FIDELITY grounding-doc ripple is in flight. **OQ-5 is
  CLOSED**; its committee-discrimination security-review WARN (a) is now
  **CLOSED by COMMITTEE-CRED-FIDELITY**. **Remaining declared follow-ups:
  OQ-3** (GOVCERT committee-membership tx-validity gate), **(c)** DRep-vote
  discrimination (recommended next cluster), **(d)** the dormant
  bare-`Hash28` `EnactmentEffects.committee_changes`, **(e)** the GREEN
  loader unknown-tag default, and the Shelley unknown-cert zero-hash
  placeholder (WARN LOW).
- **`ade_core_interop` tests `#[ignore]`-gated / offline-replay by
  design.** Live tip-agreement not run in CI; CE-N-B-6 closure evidence
  is a manual operator pass.
- **Corpus relayout: credentialed snapshots removed.** Deleted
  `corpus/snapshots/reward_provenance/*_registered_creds.txt` dominates
  the ~7M-line negative line count; replaced by 12 re-extracted
  boundary-block sets.
- No removed canonical types (n/a — no separate registry).
- No removed registry rules (expected: 0; actual: 0). OQ5 added
  `DC-LEDGER-10` (append-only; net +1 since B5, +26 since baseline);
  COMMITTEE-CRED-FIDELITY added **no rule** — it strengthened
  `DC-LEDGER-10` in place (registry total stays 173).
- **All commit subjects carry a conventional-commits prefix or are
  cluster-close housekeeping.** The four `Close PHASE4-*` commits
  (`69a2862`, `436b1d7`, `a0c73e1`, `993f363`, `d766eb0`) and the bare
  `chore:` commits (`3552bc2`, `e0af99d`) are classified `chore` on
  scope grounds. The B3 trio (`3aebbe5` docs, `978c222` feat, `7784bf8`
  test), the B3F pair (`d6c1993` feat(ci), `193d2fc` feat(codec)), the
  B4 quintet (`ae1300a` docs(planning), `228415b` feat(codec),
  `da30706`/`302d22c` feat(ledger), `ee35493` test(ledger)), the B5
  sextet (`fdb6601` docs(gov), `9c8d118`/`7a48727`/`d63c700`
  feat(ledger), `06385d0` test(ledger), `651adc9` fix(ledger)), the
  OQ5 quartet (`959e16c`/`007b0e8` docs(ledger), `4187330` feat(types),
  `a3ee2da` test(ledger)), and the COMMITTEE-CRED-FIDELITY trio (`32d7a2e`
  docs(ledger), `2303a60` feat(ledger), `2aeea16` test(ledger)) are all
  conventional. No unclassifiable subjects. All three
  COMMITTEE-CRED-FIDELITY commits (and all four OQ5 / six B5 / five B4
  commits) carry the repo-required `Co-Authored-By` model-attribution
  trailer.

---

## Generation Notes

Regenerate via `/head-deltas <baseline>` or by re-running the
`head-deltas-generator` agent with the same baseline. Baseline lives in
`.idd-config.json` `head_deltas_baseline` (still `d509f02` — **this is a
cluster-close-level follow-up refresh, not a phase boundary, so the
baseline is unchanged**). Update the baseline on the next phase boundary
(Phase 4 close). Note the commit-hash rewrite caveat at the top —
re-derive hashes from `git log` at each regen rather than carrying them
forward. This regen was cut at committed HEAD `62c9020`
(DREP-VOTE-FIDELITY-S2) with the DREP-VOTE-FIDELITY
CODEMAP/SEAMS/TRACEABILITY ripple still in the working tree. The prior
regen narrated HEAD `2aeea16` (COMMITTEE-CRED-FIDELITY-S2); its close
commit `a157c92` (the COMMITTEE-CRED grounding-doc refresh) and the three
DREP-VOTE-FIDELITY commits are the new span (`2aeea16..62c9020`, 4
commits). The OQ5 ripple landed at the OQ5 close `676af5a`.
