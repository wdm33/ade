# B3c.0 — base-UTxO stake localization (GREEN / evidence; NO BLUE change)

> **OUTCOME (see `B3c.0-adjudication-result.md`):** the base-UTxO pipeline is proven BYTE-EXACT correct
> (`b3c0_clean_checkpoint_vs_reduction`: durable checkpoint == fresh `reduce_txout`, all 254,385 credentials).
> The −343,260,172,883 go-stake residual is REAL (reproduces cleanly under the sealed doubled adjudication) but is
> NOT a base-UTxO undercount. The problem is RENAMED to a go-stake **derivation** discrepancy; **B3c.1 stays
> closed** (no base-UTxO defect). The framing below is the original pre-diagnosis plan.

## The slice

At the exact POST-1341 anchor, produce a canonical CREDENTIAL-SORTED differential between cardano's
UTxO-derived stake and Ade's reduced-checkpoint result, and classify EVERY discrepancy into a closed cause set.
This is diagnosis only — it changes no authoritative behaviour. The corrective change is B3c.1 (separate, gated
on the cause this slice names). Governed by `INVARIANTS-B3c.0.md`.

**Acceptance is NOT "the total is close."** It is: **every component of the −343,260,172,883 lovelace residual
is assigned to a deterministic, reproducible cause.** The sum of the classified components equals −343,260,172,883
exactly; nothing is left unassigned.

## The two sides (like-for-like)

- **Ade** — `ReducedUtxoCheckpoint::sum_base_credential_stake()`: the per-`StakeCredential` sum of the reduced
  UTxO's `Base(cred)` entries (`NonContributing` skipped) at POST-1341. Read as-is (no new stake computation).
- **cardano** — the decoded POST-1341 reference (`decode_native_nonutxo_state`): the go-snapshot's per-credential
  stake (`snapshots.go.0.delegations` Coin) is the differential target Ade's self-derived go must reproduce.

The differential runs at the credential level of the go snapshot (Ade's self-derived go vs cardano's go),
because that is where the −343B lives and where the per-credential attribution is exact. The reduced-checkpoint
`sum_base_credential_stake` is the UTxO-component input to that go; when the classification points at a UTxO
cause, the drill-down is into the reduced-checkpoint entries for the affected credentials.

**Basis seam (pinned first).** Ade's `sum_base_credential_stake` is base UTxO only; the go per-credential Coin is
UTxO + reward. The differential compares LIKE for LIKE (the prior decomposition put reward drift at only
−35,490,238, so the residual is ~99% UTxO). The report separates the reward component from the UTxO component
explicitly and proves the split, so a mis-aligned basis cannot masquerade as a UTxO cause.

## The closed cause set (never a free-form string)

Every non-zero per-credential component is exactly one of:

| cause | meaning |
|---|---|
| `Omitted` | a UTxO entry cardano counts for the credential that Ade's reduced checkpoint does not hold |
| `Duplicated` | an entry Ade counts more than once |
| `WrongCredentialExtraction` | Ade attributed the output's stake to the wrong credential (or none) |
| `WrongCoinValue` | Ade holds the entry but with the wrong lovelace (e.g. a value-accounting miss) |
| `UnsupportedAddressForm` | Ade classified an address form as `NonContributing` (or the reverse) incorrectly |
| `StaleCheckpointPoint` | the checkpoint reflects the wrong point (not the POST-1341 anchor) |
| `Other` | only with a named, reproducible sub-reason recorded inline — never a catch-all |

## Method

1. Advance the reduced checkpoint (a COPY of the re-bootstrapped seed — never mutate the proof) over the local
   ChainDB corpus to the POST-1341 anchor; compute Ade's self-derived go (per-credential).
2. Decode the cardano POST-1341 reference; take its go per-credential stake.
3. Canonical credential-sorted differential: `matched / value-mismatch / only-Ade / only-cardano`, lovelace-exact.
4. Separate the reward component from the UTxO component (prove the split).
5. Classify every value-mismatch / only-* component into the closed set. A uniform ~0.02% undercount points at a
   SYSTEMATIC category (address-form / extraction / value-accounting); drill into the reduced-checkpoint entries
   for the top credentials to name it exactly.
6. Assert sum-conservation: Σ(classified components) == −343,260,172,883.

## Acceptance (CE)

- The differential + classification is emitted by an `#[ignore]` local-artifact test, credential-sorted, exact.
- Every component of −343,260,172,883 is assigned to a closed cause; the assigned sum equals it exactly.
- Replay-identical: same reduced-checkpoint fingerprint + same reference commitment ⇒ byte-identical report + a
  pinned canonical B3c.0 report hash (provenance-bound, like the S5 report).
- NO BLUE/authoritative change; NO rounding; NO compensating adjustment; NO aggregate-only match.

## Tiering

- **True**: deterministic replay, one authoritative stake result (`sum_base_credential_stake`, unchanged).
- **Derived**: cardano address / UTxO / stake interpretation parity (the differential target).
- **Release**: the permanent differential corpus + the B3c.0 regression fixture (the classified residual).
- **Operational**: local ChainDB extraction tooling only (the corpus + the re-bootstrapped seed copy).

## Hard prohibitions (binding)

No rounding tolerance; no compensating adjustment; no reimporting external stake data as live authority; no change
that merely makes the aggregate total match while credential-level state still differs; no broad governance
expansion; diagnosis and fix NEVER in the same change (the fix is B3c.1).

## What B3c.0 does NOT do

No correction (that is B3c.1, gated on the cause named here); no `MissingDRepActivityParam` work (separate
continuity slice); no snapshot/reward-model change. Diagnosis only.
