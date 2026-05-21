# PHASE4-B5 — Conway Governance Certificate Accumulation Authority
## Invariant Sketch (IDD Part I)

> Status: invariants phase complete. Next: `/cluster-plan`.
> Registry anchor: **DC-LEDGER-09** (`status="declared"`), appended to
> `docs/ade-invariant-registry.toml`; strengthens DC-LEDGER-08 (bidirectional
> back-ref added).

## Concept framing

B4 closed cert-state accumulation as a closed, total, owner-tagged transition:
every Conway cert either mutates B4-owned `CertState` or is **owner-tagged to
`ConwayGovState` and routed out-of-mutation-scope** (observed, not applied, not
swallowed). B5 makes those owner-tagged effects **applied**: the
governance-affecting certs (vote-delegation 9/10/12/13, committee 14/15, DRep
16/17/18) become per-tx mutations of `ConwayGovState`, replacing today's
snapshot-loaded-and-frozen gov state with cert-accumulated gov state.

This **is** expressible as a pure transformation:
`(prior ConwayGovState, ConwayCert, gov-env) -> Result<(new ConwayGovState), GovCertError>`.
The concept is understood. Two load-bearing inputs are currently missing from
the seam (the payload gap and the env gap — see Open Questions); both are
*resolve-before-slicing*, not *concept-not-understood*.

### Grounding (verified against HEAD)

- `crates/ade_ledger/src/delegation.rs:320-324` — `OwnerTaggedEffect` is
  `#[derive(Copy)]`, `{ owner, effect }`; `effect: GovernanceCertEffect` is a
  payload-less *kind* enum (`:308-317`). The cert payloads are dropped here.
- `crates/ade_types/src/conway/cert.rs:24-99` — `ConwayCert` retains every gov
  payload owner-completely (cred / drep / cold+hot committee creds / deposit).
- `crates/ade_ledger/src/rules.rs:1474-1478` — the drop site: `state =
  outcome.state`, `outcome.owner_tagged` observed-and-dropped with a comment.
- `crates/ade_ledger/src/state.rs:83-103` — `ConwayGovState`. Cert-driven
  fields: `vote_delegations: BTreeMap<Hash28, DRep>` ("Loaded from UMap"),
  `committee_hot_keys: BTreeMap<Hash28, Hash28>` (hot->cold), `drep_expiry:
  BTreeMap<Hash28, u64>`. `committee` membership is set by gov *enactment*
  (UpdateCommittee), not by certs.
- `crates/ade_ledger/src/rules.rs:1152-1226` — epoch-boundary
  ratification/enactment (CE-72 path) reads `gov.vote_delegations` /
  `gov.committee` / `gov.drep_expiry` / `gov.committee_hot_keys` and currently
  clones them forward unchanged.
- `crates/ade_types/src/shelley/cert.rs:46` — `StakeCredential(pub Hash28)` is a
  single-field newtype; it does **not** carry the key-hash/script-hash
  discriminant (relevant to OQ-5).

---

## 1. What must always be true

- **I-1 (closure).** For each era, governance-cert accumulation
  `apply_gov_cert(gov_state, cert)` is a closed, total, era-versioned function
  over the governance-affecting Conway tags. Every governance effect declared by
  B4's `ConwayCertAction` (`Governance(_)` and the governance half of
  `CertStateAndGovernance(_)`) resolves to exactly one explicit `ConwayGovState`
  mutation or a structured reject — never observed-and-dropped. *(Strengthens
  DC-LEDGER-08: B4's "routed out-of-mutation-scope" disposition is retired for
  governance certs; the effect is now applied.)*
- **I-2 (owner exclusivity preserved).** B5 mutates **only** the
  governance-owned fields of `ConwayGovState` (`vote_delegations`,
  `committee_hot_keys`, and the cert-driven part of `drep_expiry`). It does not
  mutate B4-owned `CertState`, and it does not double-apply the delegation/pool
  half of composite certs (10/12/13) that B4 already applied.
- **I-3 (gov-env completeness).** Any environment input a governance mutation
  needs (current epoch + the DRep-activity protocol parameter for DRep expiry)
  is supplied through a closed, explicit env value. A missing required env input
  is a structured fail-fast (a `MissingGovEnv`-class error), never a
  defaulted/guessed expiry.
- **I-4 (fail-closed accumulation).** A governance cert that cannot be applied
  (precondition violated, env absent) propagates a structured `LedgerError` and
  halts the block transition. Incomplete or best-effort governance accumulation
  is a forbidden fail-open.
- **I-5 (gov-state provenance migration).** Conway `gov_state` becomes a
  deterministic function of (boundary-loaded base ∘ replayed block-stream cert
  effects), not a frozen snapshot. The ratification/enactment reader path
  (`rules.rs:1152-1226`) reads the cert-accumulated maps.

## 2. What must never be possible

- A governance cert silently dropped (the B4 `// observed and routed out of B4
  mutation scope` comment must be gone, not relabeled).
- A composite cert (10/12/13) applying its delegation/pool effect twice, or its
  governance effect zero times.
- A DRep expiry computed from a defaulted epoch or a defaulted `drepActivity`
  (guessed environment).
- B5 mutating `CertState`, `committee` *membership* (set by governance
  **enactment** — `UpdateCommittee` — not by certs; only `committee_hot_keys` is
  cert-driven), proposals, reserves, or treasury.
- A `Governance(_)` effect kind without enough payload to perform the mutation
  reaching an apply path (the payload gap must be closed structurally, not
  papered over).

## 3. What must remain identical across executions (deterministic surface)

- `apply_gov_cert` is a pure function of `(ConwayGovState, ConwayCert,
  GovCertEnv)` — no wall-clock, no rand, no float, no `HashMap`. All
  `ConwayGovState` collections are already `BTreeMap` (ordered).
- Iteration order over certs is positional (tx cert-array order), threaded
  exactly as B4 threads `CertState`.
- DRep expiry arithmetic is integer (`epoch + drepActivity`), no saturating
  ambiguity beyond what is canonicalized.

## 4. What must be replay-equivalent

- Replaying the same ordered block stream over the same boundary-loaded base
  `ConwayGovState` must produce a **byte-identical** accumulated `ConwayGovState`
  (vote_delegations, committee_hot_keys, drep_expiry). This is the B5
  fingerprint.
- **Deliberate fingerprint migration (T-DET-01):** the accumulated gov-state
  fingerprint differs from the snapshot-frozen one. Intentional — same class of
  migration as B4's `CertState` fingerprint change — must be confirmed *correct*
  against the reference, not merely *stable*. (Real-oracle confirmation is
  expected to be **environment-blocked** — see OQ-4.)

## 5. State transitions in scope

All on the Conway path, at `track_utxo == true`, threaded per-tx through
block-body application:

- **Vote delegation** — tags 9/10/12/13:
  `(gov, VoteDelegation{cred, drep} | …{cred, pool, drep, …}) ->
  Ok(gov{ vote_delegations[cred.0] = drep })`
  (tags 10/13: stake->pool half already applied by B4; tags 12/13: registration
  half already applied by B4.)
- **Committee hot-key auth** — tag 14:
  `(gov, AuthCommitteeHot{cold, hot}) ->
  Ok(gov{ committee_hot_keys[hot.0] = cold.0 })` *(membership precondition is
  OQ-3).*
- **Committee cold resignation** — tag 15:
  `(gov, ResignCommitteeCold{cold}) ->
  Ok(gov{ committee_hot_keys ⊖ entries authorizing cold })` *(exact resignation
  semantics OQ-3).*
- **DRep registration** — tag 16:
  `(gov, DRepRegistration{drep_cred, deposit}, env) ->
  Ok(gov{ drep_expiry[drep_cred.0] = env.epoch + env.drep_activity })`
- **DRep unregistration** — tag 17:
  `(gov, DRepUnregistration{drep_cred, refund}) ->
  Ok(gov{ drep_expiry ⊖ drep_cred.0 })`
- **DRep update** — tag 18:
  `(gov, DRepUpdate{drep_cred}, env) ->
  Ok(gov{ drep_expiry[drep_cred.0] = env.epoch + env.drep_activity })`
- **Removed tags 5/6** and **bad CBOR**: already rejected upstream by B4's
  decode/era dispatch — B5 inherits, does not re-handle.

## 6. TCB color hypothesis

- **BLUE** — the entire B5 surface: `apply_gov_cert` (pure transition), the
  `GovCertEnv` type, any enrichment of the owner-tagged effect / `ConwayGovState`
  mutators, and the wiring in `rules.rs` accumulation. This is authoritative
  ledger state.
- **GREEN** — the synthetic gov-state corpus harness and replay drivers in
  `ade_testkit`.
- **RED** — none.
- No unresolved colors.

## 7. Open questions (resolve before cluster-plan / slicing)

- **OQ-1 (payload gap — load-bearing, B5's OQ-1-equivalent). HELD FULLY OPEN for
  `/cluster-plan`.** `OwnerTaggedEffect`/`GovernanceCertEffect` is `Copy` and
  payload-less (effect *kind* only); the payloads live on `ConwayCert`. Two
  candidates, both honoring "don't flatten/guess":
  - **(a) Enrich the seam** — make `GovernanceCertEffect` carry its payload
    (cred / drep / committee creds / deposit); B5 consumes the enriched
    owner-tagged effect at the `rules.rs:1474` drop site. Keeps a single
    owner-tagged dispatch; makes the effect non-`Copy`. More faithful to B4's
    declared "owner-tagged effect is the B5 carrier" seam.
  - **(b) B5-native gov dispatch** — a parallel
    `apply_conway_gov_cert(gov_state, &ConwayCert, env)` running alongside
    `apply_conway_cert`, reading payloads directly off `ConwayCert`. Bypasses the
    payload-less seam; two dispatches over one cert.

  Decide in `/cluster-plan` exactly as B4 decided its OQ-1 (which chose native
  apply). No leaning recorded here, per user direction.
- **OQ-2 (env gap).** DRep expiry needs `current_epoch` + `drepActivity` param.
  Neither is in today's `ConwayCertEnv` (only `key_deposit`, `cert_index`), and
  `drepActivity` is not obviously in `ConwayGovState` or threaded params.
  Resolve the canonical source + the fail-fast when absent (I-3).
  Vote-delegation and committee certs need **no** env beyond the payload — a
  possible slice-ordering seam (env-free certs first).
- **OQ-3 (committee semantics).** Exact `ConwayGovState` mutation for tags
  14/15: (i) does B5 validate that `cold` is an active/proposed committee member
  before authorizing a hot key (fail-closed precondition, mirroring B4's
  StakeNotRegistered checks), or accumulate unconditionally? (ii) resignation
  (tag 15) — remove the hot-key entry, or mark resigned? Ground against
  cardano-node `ConwayCERTS`/`COMMITTEE` rules before slicing.
- **OQ-4 (oracle — expect env-block).** The real epoch-576
  gov-state-vs-cardano-node (VState) oracle is the same absent snapshot as
  B4-S5. Plan: synthetic positive + replay + adversarial + **documented
  environment-blocked real oracle**, reclassified per the tier doctrine, exactly
  as DC-LEDGER-08 / DC-TXV-06 did.
- **OQ-5 (credential identity fidelity — known divergence, not a B5 gap).**
  `ConwayGovState` keys by raw `Hash28`; cardano-node VState keys
  DReps/committee by key-or-script-tagged `Credential`. A key-hash and
  script-hash DRep sharing 28 bytes would collide in Ade. Pre-existing
  (inherited from `StakeCredential(Hash28)` + the codec's credential decode),
  env-blocked from proof anyway. Surface as a known limitation/divergence;
  confirm it is out of B5 scope rather than silently inheriting.

---

## Registry anchor (appended)

```toml
[[rules]]
id = "DC-LEDGER-09"
tier = "derived"
statement = "Conway governance-certificate accumulation is a closed, total, era-versioned transition into ConwayGovState ... (see registry for full text)"
source = "Cardano ledger spec (Conway CDDL gov cert tags 9..18; CONWAY CERTS/GOVCERT/COMMITTEE transitions; CIP-1694); IDD fail-fast + closed-surface doctrine"
cross_ref = ["DC-LEDGER-08", "DC-VAL-06", "DC-TXV-06", "T-DET-01", "CN-LEDGER-07"]
status = "declared"
authority_surface = "BLUE Conway governance-state accumulation"
cluster = "CL-LEDGER-VERDICT"
```

`DC-LEDGER-08.cross_ref` gained a back-ref to `DC-LEDGER-09` (bidirectional,
per the B4 `ci_check_constitution_coverage.sh` requirement).
