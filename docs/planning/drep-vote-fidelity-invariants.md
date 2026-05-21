# DREP-VOTE-FIDELITY — DRep-voter credential discriminant
## Invariant Sketch (IDD Part I) — STRENGTHENS DC-LEDGER-10

> Strengthens DC-LEDGER-10 (no new rule). The direct parallel of
> COMMITTEE-CRED-FIDELITY, for the DRep-voter surface. Origin: COMMITTEE-CRED
> security review WARN-LOW (recommended next cluster).

## Concept framing
`GovActionState.drep_votes: Vec<(Hash28, Vote)>` carries bare DRep-voter
credentials, and `governance.rs::lookup_stake` resolves DRep stake with a
key/script **OR-fallback** over identical bytes
(`drep_stake.get(DRep::KeyHash(c)).or_else(|| ...ScriptHash(c))`) — the same
cross-resolution this family eliminated for committee. DReps can be
script-credentialed, so a key-hash and a script-hash DRep of equal bytes are
conflated in vote tallying. This cluster discriminates `drep_votes` and resolves
DRep stake by **exact variant match**.

`spo_votes` stays `Hash28` — pools are **always** key-hash (no script), so there
is no discriminant to preserve. Genuine non-goal.

### Grounding (HEAD a157c92)
- `drep_votes: Vec<(Hash28, Vote)>` (conway/governance.rs:44); `spo_votes` same
  (45).
- `lookup_stake` OR-fallback (governance.rs:240-244).
- Reusable committee infra (added by COMMITTEE-CRED-FIDELITY):
  `write_committee_vote_list` (fingerprint, StakeCredential votes),
  `parse_committee_vote_map` (loader, tag-preserving). Rename both to
  credential-generic and reuse for drep.
- DRep voter is wire-tagged (DRepKeyHash=2 / DRepScriptHash=3); a DRep voter is
  always key-or-script (never AlwaysAbstain/NoConfidence — those are delegation
  targets, not voters) → `StakeCredential` is the right type.

## 1. What must always be true
- **I-1 (drep votes discriminated).** `GovActionState.drep_votes` carries the
  discriminated `StakeCredential` DRep-voter credential.
- **I-2 (exact stake resolution).** DRep vote→stake resolution maps the voter's
  `StakeCredential::{KeyHash,ScriptHash}(h)` to the **exact** `DRep::{KeyHash,
  ScriptHash}(h)` key — no key/script OR-fallback; a key-hash voter never
  resolves a script-hash DRep's stake of equal bytes.
- **I-3 (fingerprint fidelity).** `drep_votes` serializes the discriminant.

## 2. What must never be possible
- A DRep voter stored/resolved as a tag-erased `Hash28`.
- The `DRep::KeyHash(c).or_else(ScriptHash(c))` OR-fallback (or any key/script
  cross-resolution) in DRep vote tallying.

## 3. Deterministic surface
- `drep_votes` voter is `StakeCredential` (`Ord`); fingerprint sorts by it.

## 4. Replay-equivalent
- Deliberate fingerprint migration (T-DET-01) on the `drep_votes` vote-list
  surface (continuation of OQ5/committee).

## 5. State transitions in scope
- DRep vote tally (`governance.rs`): `(drep_votes: Vec<(StakeCredential, Vote)>,
  drep_stake keyed by DRep) → yes_stake` by exact variant match.

## 6. TCB color
- BLUE — `ade_types` (drep_votes type), `ade_ledger` (governance.rs lookup,
  fingerprint). GREEN — `ade_testkit` loader. RED — none.

## 7. Open questions
- **OQ-A (spo_votes).** Stays `Hash28` (pools key-hash only). Confirm — pool keys
  carry no script variant. *(Recommend: out of scope, permanently — not a
  follow-up.)*
- **OQ-B (rename shared writers).** Rename `write_committee_vote_list` /
  `parse_committee_vote_map` to credential-generic (`write_credential_vote_list`
  / `parse_credential_vote_map`) since they now serve committee + drep. Pure
  rename, output-identical (no fingerprint change). *(Recommend: yes.)*

## Registry disposition
Strengthen DC-LEDGER-10 (append `strengthened_in`, extend `code_locus`/`tests`,
extend the CI gate). No new rule.
