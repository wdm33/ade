# Cluster ENACTMENT-COMMITTEE-FIDELITY — committee_changes discriminant (preventive)

> **Status:** Planning artifact (non-normative). **Strengthens DC-LEDGER-10** (no
> new rule). Closes the last tracked credential-fidelity follow-up (d): the
> dormant `EnactmentEffects.committee_changes` bare-`Hash28` field. Origin:
> COMMITTEE-CRED security review (informational). Single-slice. Registry/specs win.

## Primary invariant (strengthens DC-LEDGER-10)
> `EnactmentEffects.committee_changes` (the removed + added committee members
> produced by `UpdateCommittee` enactment) carries discriminated `StakeCredential`
> committee credentials, not bare `Hash28`. The type makes a tag-erased committee
> member unrepresentable in the enactment-effect surface, so when committee
> enactment is implemented it CANNOT re-collapse the discriminated
> `ConwayGovState.committee` map on write-back.

## Why preventive (the latent trap)
`committee_changes` is currently **dormant**: `UpdateCommittee` enactment is a
no-op (`enact_proposals` arm is `let _ = raw`), `EnactmentEffects::default()`
leaves it `None`, and nothing reads it. But its type is `Option<(Vec<Hash28>,
Vec<(Hash28, u64)>)>` — bare `Hash28`. If committee enactment were implemented
against that type, it would produce tag-erased members and re-collapse the
committee map this family just discriminated (COMMITTEE-CRED-FIDELITY). Migrating
the type now (illegal-states-unrepresentable) closes the trap before the logic
exists. Zero behavior change (the field stays `None`).

## Grounding (HEAD 06f517f)
- `EnactmentEffects.committee_changes: Option<(Vec<Hash28>, Vec<(Hash28,u64)>)>`
  (governance.rs:344) — the ONLY occurrence; never populated, never read.
- `enact_proposals` `UpdateCommittee` arm: `let _ = raw` (governance.rs:395-398).
- `EnactmentEffects` derives `Default` → `None` regardless of inner type.
- `StakeCredential` already imported in governance.rs; `Hash28` stays (pool/SPO
  lookup).

## Per-surface change
| surface | before | after |
|---|---|---|
| `EnactmentEffects.committee_changes` | `Option<(Vec<Hash28>, Vec<(Hash28,u64)>)>` | `Option<(Vec<StakeCredential>, Vec<(StakeCredential,u64)>)>` |

## Exit Criteria (CI-Verifiable)
- **CE-1 (type discriminated):** `EnactmentEffects.committee_changes` carries
  `StakeCredential`. Compile + extended CI gate.
- **CE-2 (distinctness):** `enactment_committee_changes_keyhash_scripthash_distinct`
  — a key-hash and a script-hash member of equal bytes are distinct entries in the
  field (the type holds discriminated creds, no collapse).
- **CE-3 (CI + strengthen):** the credential gate asserts `committee_changes` is
  `StakeCredential`-typed (not `Hash28`); DC-LEDGER-10 `strengthened_in +=
  ENACTMENT-COMMITTEE-FIDELITY`; coverage PASS.

## Slice
- **S1** — the type migration + pinning distinctness test + CI gate extension +
  DC-LEDGER-10 strengthen. *(CE-1..CE-3)* — TCB: BLUE (`ade_ledger::governance`).

## TCB Color Map
- BLUE — `crates/ade_ledger/src/governance.rs`. GREEN/RED — none.

## Forbidden
- Reverting `committee_changes` to bare `Hash28`.
- Implementing committee-enactment write-back logic here (out of scope — this is
  the type-fidelity prerequisite only).

## Declared non-goals
- Actually implementing `UpdateCommittee` enactment (stays a no-op; a separate
  governance-enactment cluster).
