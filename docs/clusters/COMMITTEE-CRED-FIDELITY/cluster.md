# Cluster COMMITTEE-CRED-FIDELITY — committee member + vote credential discriminant

> **Status:** Planning artifact (non-normative). **Strengthens DC-LEDGER-10** (no
> new rule) — closes the committee-surface enforcement gap its statement already
> implies. Produced from
> `docs/planning/committee-cred-fidelity-invariants.md`. Origin: OQ5 per-cluster
> security review WARN-MEDIUM. If this doc conflicts with the registry/specs,
> those win.

---

## Primary invariant (strengthens DC-LEDGER-10)

> The committee credential surface is fully discriminated:
> `ConwayGovState.committee` keys on the discriminated `StakeCredential` (cold
> member), `GovActionState.committee_votes` carries the discriminated hot voter
> credential, and committee ratification resolves hot→cold→member by
> **full-credential equality** — never `.hash()`. A key-hash hot key never
> cross-resolves to a script-hash member of equal bytes. The committee map key and
> vote voter serialize the discriminant. This closes the one remaining hash-level
> committee resolution OQ5 left on the BLUE path.

## OQ resolutions (locked)
- **OQ-A → committee-only.** `drep_votes` (DRep voters) and `pool_votes` (always
  key-hash) are NOT in scope; drep-vote discrimination is a parallel follow-up.
- **OQ-B → reuse `StakeCredential`** for committee cold/hot creds (consistent with
  the already-discriminated `committee_hot_keys`).
- **OQ-E → strengthen DC-LEDGER-10** (append `strengthened_in`, extend
  `code_locus`/`tests`); reuse/extend `ci_check_credential_discriminant_closed.sh`.

## Grounding (verified at HEAD a3ee2da)
- `committee: BTreeMap<Hash28,u64>` (state.rs:87); `committee_votes:
  Vec<(Hash28,Vote)>` (conway/governance.rs:42); `committee_hot_keys` already
  `StakeCredential` (OQ5).
- `governance.rs:209-219` resolves via `.hash()` (the WARN-MEDIUM).
- `committee_votes` populated by the GREEN loader (`parse_governance_proposals`);
  tx `voting_procedures` are raw bytes — **no tx-decoder change**.
- committee members from `parse_committee_state` (loader) / `UpdateCommittee`.

## Per-surface change
| surface | before | after |
|---|---|---|
| `ConwayGovState.committee` | `BTreeMap<Hash28,u64>` | `BTreeMap<StakeCredential,u64>` |
| `GovActionState.committee_votes` | `Vec<(Hash28,Vote)>` | `Vec<(StakeCredential,Vote)>` |
| `governance.rs` resolution | `hot.hash()==hot_cred`, `**m==*cold.hash()` | full-credential `==` |
| fingerprint | committee key + vote voter as `write_hash28` | `write_stake_credential` |
| loader (GREEN) | committee members + proposal votes bare `Hash28` | preserve key/script tag |

`drep_votes`/`pool_votes` unchanged (non-goals). `committee_hot_keys` unchanged
(already discriminated).

## Entry Conditions
- OQ5 closed: `StakeCredential` is the discriminated enum + `hash()`;
  `committee_hot_keys` discriminated; `ci_check_credential_discriminant_closed.sh`
  + DC-LEDGER-10 enforced.

## Exit Criteria (CI-Verifiable)
- **CE-1 (committee discriminated):** `ConwayGovState.committee` +
  `GovActionState.committee_votes` key/carry `StakeCredential`. Compile + the
  extended CI gate.
- **CE-2 (full-credential resolution):** `governance.rs` committee resolution uses
  full-credential equality (no `.hash()` in the committee path). Test:
  `committee_keyhash_scripthash_do_not_cross_resolve` (a KeyHash hot key and a
  ScriptHash member of equal bytes do NOT ratify).
- **CE-3 (distinct members):** `committee_keyhash_scripthash_same_bytes_distinct`
  — two committee members of equal bytes/different variant are distinct.
- **CE-4 (fingerprint fidelity):** `committee_discriminant_changes_fingerprint`.
- **CE-5 (replay):** committee ratification + fingerprint replay byte-identical.
- **CE-6 (CI + strengthen):** the credential CI gate covers the committee surfaces
  (no bare-`Hash28` committee key/vote on the BLUE path); DC-LEDGER-10
  `strengthened_in += COMMITTEE-CRED-FIDELITY` with extended code_locus/tests;
  `ci_check_constitution_coverage.sh` PASS.

## Expected Slice Types
- **S1** — discriminate `committee` + `committee_votes`; `governance.rs`
  full-credential equality; fingerprint migration; GREEN loader tag preservation.
  Atomic. *(CE-1..CE-5)*
- **S2** — negative/distinctness corpus + extend the CI gate + strengthen
  DC-LEDGER-10. *(CE-2/CE-3/CE-6)*

## TCB Color Map
- **BLUE** — `ade_types` (`conway::governance::GovActionState`,
  `state::ConwayGovState.committee`), `ade_ledger` (`governance.rs`,
  `fingerprint.rs`, `state.rs`).
- **GREEN** — `ade_testkit` (snapshot loader committee + proposal-vote parses).
- **RED** — none.

## Forbidden During This Cluster
- Any `.hash()` comparison standing in for committee-credential identity in
  `governance.rs` ratification.
- A bare-`Hash28` committee member or committee voter on the BLUE path.
- Dragging `drep_votes` / `pool_votes` / a new rule into scope (non-goals).

## Environment-blocked (reclassified)
- Real-chain committee-state agreement vs cardano-node — same env-block as
  DC-LEDGER-10.

## Declared non-goals
- `drep_votes` (DRep-voter) and `pool_votes` discrimination; the broader voting
  surface.
