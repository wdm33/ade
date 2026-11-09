# COMMITTEE-CRED-FIDELITY — committee member + vote credential discriminant
## Invariant Sketch (IDD Part I) — STRENGTHENS DC-LEDGER-10

> Status: invariants phase. Next: cluster plan → cluster doc → slices.
> **Strengthens DC-LEDGER-10** (no new rule): closes the enforcement gap its
> statement already implies — "a stake/committee/DRep credential is a closed sum
> … never a tag-erased Hash28" — but which OQ5 left open for the committee
> *member* and committee *vote* surfaces. Origin: OQ5 per-cluster security review
> WARN-MEDIUM.

## Concept framing

OQ5 discriminated `committee_hot_keys` (and the cert/gov credential surfaces) but
left two committee surfaces bare `Hash28`:
- `ConwayGovState.committee: BTreeMap<Hash28, u64>` (cold members → expiry),
- `GovActionState.committee_votes: Vec<(Hash28, Vote)>` (hot voter creds).

Consequently `governance.rs` committee ratification resolves at the hash level
(`hot.hash() == hot_cred`, `**member == *cold.hash()`), the one remaining place a
key/script collapse could hide on the BLUE path. This cluster discriminates both,
so committee resolution is **full-credential equality** with no `.hash()`.

Pure transformation: `loader/decode preserves voter tag → committee state keys on
discriminated credential → governance resolution compares whole credentials →
fingerprint serializes the discriminant`. Understood.

### Grounding (verified at HEAD a3ee2da)
- `committee: BTreeMap<Hash28, u64>` (state.rs:87); `committee_votes:
  Vec<(Hash28, Vote)>` (conway/governance.rs:42).
- `governance.rs:209-219` resolves hot→cold and member-active via `.hash()` (the
  OQ5 WARN-MEDIUM).
- `committee_votes` is populated by the GREEN snapshot loader
  (`parse_governance_proposals`); tx `voting_procedures` (conway/tx.rs key 19) are
  captured as raw bytes — **no tx-decoder change needed**.
- committee members from `parse_committee_state` (loader) / `UpdateCommittee`
  enactment. `committee_hot_keys` is ALREADY `StakeCredential` (OQ5).
- Conway `Voter` is wire-tagged (committee-key / committee-script / drep-* /
  pool); the committee voter discriminant is recoverable where parsed.

---

## 1. What must always be true
- **I-1 (committee state discriminated).** `ConwayGovState.committee` keys on the
  discriminated `StakeCredential` (cold committee credential); a key-hash and a
  script-hash committee member sharing 28 bytes are distinct members.
- **I-2 (committee votes discriminated).** `GovActionState.committee_votes` carries
  the discriminated hot voter credential; the voter's key/script tag is preserved
  where parsed, never erased.
- **I-3 (full-credential resolution).** Committee ratification resolves
  hot→cold→member by **full-credential equality** (no `.hash()`); a key-hash hot
  key never cross-resolves to a script-hash member of equal bytes, and vice versa.
- **I-4 (fingerprint fidelity).** The `committee` map key and the `committee_votes`
  voter credential serialize the discriminant. *(Strengthens T-DET-01;
  consolidates DC-LEDGER-10's committee coverage.)*

## 2. What must never be possible
- A committee member or committee voter stored/compared as a tag-erased `Hash28`
  on the BLUE path.
- A `.hash()` comparison standing in for committee-credential identity in
  `governance.rs` ratification.
- A key-hash hot key cross-resolving to a script-hash committee member of equal
  bytes (the negative the OQ5 review asked for).
- A fingerprint byte-identical for two states differing only in a committee
  credential's discriminant.

## 3. Deterministic surface
- `committee` / `committee_votes` keyed/ordered on the discriminated
  `StakeCredential` (`Ord` over (variant, hash)); `BTreeMap`/ordered `Vec`, no
  `HashMap`. Resolution is a pure function of the discriminated state.

## 4. Replay-equivalent
- Replaying the same block stream / loading the same snapshot yields byte-identical
  `committee` + proposal fingerprints. **Deliberate fingerprint migration
  (T-DET-01)** on the governance committee + vote-list surfaces — a continuation
  of OQ5's, confirmed-correct-not-merely-stable.

## 5. State transitions in scope
- Committee ratification resolution (`governance.rs`): `(committee:
  BTreeMap<StakeCredential,u64>, committee_hot_keys: BTreeMap<StakeCredential,
  StakeCredential>, committee_votes: Vec<(StakeCredential, Vote)>) → ratify
  bool` — full-credential equality.
- Fingerprint writers: committee key + vote-list voter emit `discriminant ‖ hash`.
- GREEN loader: `parse_committee_state` members + `parse_governance_proposals`
  votes preserve the voter tag.

## 6. TCB color hypothesis
- **BLUE** — `ade_types` (`ConwayGovState.committee`, `GovActionState.committee_votes`
  types), `ade_ledger` (`governance.rs` resolution, `fingerprint.rs`, `state.rs`).
- **GREEN** — `ade_testkit` (snapshot loader committee + proposal-vote parses).
- **RED** — none.

## 7. Open questions
- **OQ-A (scope: committee only).** `drep_votes` (DRep voters — DRep can be
  script) and `pool_votes` (always key-hash) are NOT in scope. Confirm
  committee-only; drep-vote discrimination is a parallel follow-up (pool needs
  none). *(Recommend committee-only — matches the review's exact finding.)*
- **OQ-B (member type).** Reuse `StakeCredential` for committee cold/hot
  credentials (consistent with `committee_hot_keys`), vs a committee-role-specific
  type. *(Recommend reuse `StakeCredential` — `committee_hot_keys` already does.)*
- **OQ-C (loader tag availability).** Confirm the snapshot's committee-member and
  proposal-vote CBOR carries the key/script tag at the parse sites; where genuinely
  absent, GREEN default `KeyHash` with a note (as OQ5).
- **OQ-D (oracle).** Real-chain committee-state agreement vs cardano-node is the
  same env-blocked obligation as DC-LEDGER-10 (reclassified).
- **OQ-E (registry).** Strengthen DC-LEDGER-10 (append `strengthened_in`, extend
  `code_locus`/`tests`) — NOT a new rule.

## Registry disposition
**Strengthen DC-LEDGER-10.** No new rule. Append the cluster id to
`strengthened_in`; extend `code_locus` + `tests`; the existing CI gate
`ci_check_credential_discriminant_closed.sh` is extended (committee surfaces) or a
sibling assertion added.
