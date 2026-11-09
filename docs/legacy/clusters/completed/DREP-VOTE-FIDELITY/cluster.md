# Cluster DREP-VOTE-FIDELITY — DRep-voter credential discriminant

> **Status:** Planning artifact (non-normative). **Strengthens DC-LEDGER-10** (no
> new rule). The direct parallel of COMMITTEE-CRED-FIDELITY for the DRep-voter
> surface. From `docs/planning/drep-vote-fidelity-invariants.md`. Origin:
> COMMITTEE-CRED security review WARN-LOW. Registry/specs win on conflict.

## Primary invariant (strengthens DC-LEDGER-10)
> `GovActionState.drep_votes` carries the discriminated `StakeCredential`
> DRep-voter credential, and DRep vote→stake resolution maps the voter's
> key/script variant to the **exact** `DRep::{KeyHash,ScriptHash}` stake key —
> never a key/script OR-fallback. A key-hash DRep voter never resolves a
> script-hash DRep's stake of equal bytes. The `drep_votes` voter serializes the
> discriminant. `spo_votes` stays `Hash28` (pools are always key-hash).

## OQ resolutions (locked)
- **OQ-A → `spo_votes` stays `Hash28`, permanently** (pools have no script
  variant; not a follow-up).
- **OQ-B → rename the shared writers** `write_committee_vote_list` →
  `write_credential_vote_list`, `parse_committee_vote_map` →
  `parse_credential_vote_map` (now serve committee + drep; pure rename,
  output-identical).
- Strengthen DC-LEDGER-10 (no new rule).

## Grounding (HEAD a157c92)
- `drep_votes: Vec<(Hash28,Vote)>` (conway/governance.rs:44).
- `lookup_stake` OR-fallback (governance.rs:240-244).
- Reusable committee infra: `write_committee_vote_list` (fingerprint),
  `parse_committee_vote_map` (loader).

## Per-surface change
| surface | before | after |
|---|---|---|
| `GovActionState.drep_votes` | `Vec<(Hash28,Vote)>` | `Vec<(StakeCredential,Vote)>` |
| `lookup_stake` | `DRep::KeyHash(c).or_else(ScriptHash(c))` | exact variant: KeyHash→DRep::KeyHash, ScriptHash→DRep::ScriptHash |
| fingerprint drep_votes | `write_vote_list` (Hash28) | `write_credential_vote_list` (StakeCredential) |
| loader drep_votes | `parse_vote_map` (Hash28) | `parse_credential_vote_map` (tag-preserving) |
| `spo_votes` | `Vec<(Hash28,Vote)>` | unchanged (non-goal) |

## Entry Conditions
- COMMITTEE-CRED-FIDELITY closed: `StakeCredential` discriminated; committee
  surface discriminated; `write_committee_vote_list`/`parse_committee_vote_map`
  exist (to be renamed/reused); DC-LEDGER-10 enforced + its CI gate.

## Exit Criteria (CI-Verifiable)
- **CE-1 (drep votes discriminated):** `GovActionState.drep_votes` carries
  `StakeCredential`. Compile + extended CI gate.
- **CE-2 (exact stake resolution):** `drep_keyhash_scripthash_do_not_cross_resolve`
  — a key-hash DRep voter does NOT tally a script-hash DRep's stake of equal
  bytes (and the matching variant does). No OR-fallback in `lookup_stake`.
- **CE-3 (fingerprint fidelity):** `drep_vote_discriminant_changes_fingerprint`.
- **CE-4 (CI + strengthen):** the credential gate asserts `drep_votes` is
  `StakeCredential`-typed and `lookup_stake` has no key/script OR-fallback;
  DC-LEDGER-10 `strengthened_in += DREP-VOTE-FIDELITY` (code_locus/tests
  extended); coverage PASS.

## Expected Slice Types
- **S1** — discriminate `drep_votes`; exact-variant `lookup_stake`; fingerprint +
  loader reuse (rename shared writers credential-generic). *(CE-1..CE-3)*
- **S2** — cross-resolve negative + distinctness corpus + CI gate + strengthen
  DC-LEDGER-10. *(CE-2/CE-4)*

## TCB Color Map
- BLUE — `ade_types` (`conway::governance` drep_votes), `ade_ledger`
  (`governance.rs`, `fingerprint.rs`). GREEN — `ade_testkit` loader. RED — none.

## Forbidden During This Cluster
- Any `DRep::KeyHash(c).or_else(ScriptHash(c))` OR-fallback / key-script
  cross-resolution in DRep vote tallying.
- A bare-`Hash28` DRep voter on the BLUE path.
- Touching `spo_votes` (pools are key-hash; non-goal) or a new rule.

## Declared non-goals
- `spo_votes` discrimination (no script variant for pools).
- `EnactmentEffects.committee_changes` migration (carried; a committee-enactment
  prerequisite, not this cluster).
