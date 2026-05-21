# Cluster/Slice Plan — Ade / PHASE4-B5 (Conway governance certificate accumulation authority)

> **Status**: planning artifact, non-normative. Authority lives in
> `docs/ade-invariant-registry.toml` (DC-LEDGER-09) and
> `docs/planning/phase4-b5-invariants.md`. Produced via `/cluster-plan`.
> **Distinct from the phase-level `docs/active/phase_4_cluster_plan.md`, which
> this doc does not modify** (same convention as the B4 plan).

## Inputs
- `docs/planning/phase4-b5-invariants.md` — B5 sketch (I-1..I-5; OQ-1..OQ-5).
- `docs/ade-invariant-registry.toml` — **DC-LEDGER-09** (declared); strengthens
  DC-LEDGER-08; anchored by T-DET-01.
- B4 (closed): owner-complete `ConwayCert` (tags 0..18, all gov payloads
  retained); `apply_conway_cert` + `ConwayCertOutcome.owner_tagged`;
  era-dispatched fail-closed `accumulate_tx_certs`; `LedgerError` taxonomy.
- Existing: `ConwayGovState` (state.rs:83), already canonically fingerprinted
  (`fingerprint.rs:278-313`); `conway_deposit_view()` fail-fast pattern
  (state.rs:128).
- `~/.claude/methodology/idd.md` Part I §§1–10, Part IV.

> Detailed per-variant authority comes in `docs/clusters/PHASE4-B5/cluster.md`
> (`/cluster-doc`). This plan is the high-level index; where terser than the
> cluster doc, the cluster doc and the registry win.

## Grounding (load-bearing, verified at HEAD)
- **Payload gap:** `OwnerTaggedEffect` (delegation.rs:320-324) is `Copy` and
  payload-less; payloads live on `ConwayCert` (cert.rs:24-99).
- **Drop site:** rules.rs:1474-1478 — `state = outcome.state`,
  `outcome.owner_tagged` observed-and-dropped.
- **Owner:** `ConwayGovState` (state.rs:83-103). Cert-driven fields:
  `vote_delegations` (9/10/12/13), `committee_hot_keys` hot→cold (14/15),
  `drep_expiry` (16/17/18). `committee` membership is enactment-driven
  (`UpdateCommittee`), NOT cert-driven.
- **Env gap:** `drep_activity` is absent from BLUE `ProtocolParameters`,
  `ConwayGovState`, and `ConwayOnlyDepositParams`; parsed only by the GREEN
  loader, which currently stops at `drep_deposit` (snapshot_loader.rs:2565).
  `current_epoch` reachable via `state.epoch_state.epoch`;
  `process_block_certificates` already receives `&LedgerState`.
- **Fingerprint surface:** `fingerprint.rs:278-313` already serializes
  `drep_expiry`/`vote_delegations`/`committee_hot_keys` canonically — the B5
  migration is observable here; no new canonical type.
- **Fail-fast precedent:** `ValidationEnvironmentError::MissingConwayDepositParams`
  + `conway_deposit_view()` (state.rs:128).

## OQ resolutions (load-bearing — endorsed at plan approval)

- **OQ-1 (payload gap) → (b) B5-native gov dispatch.** A new pure
  `apply_conway_gov_cert(gov_state, &ConwayCert, &GovCertEnv) ->
  Result<ConwayGovState, LedgerError>` reads payloads directly off the
  owner-complete `ConwayCert`. *Rationale:* (1) parallels B4's own OQ-1 (native
  apply, no lossy adapter); (2) **additive** — leaves B4's closed
  `OwnerTaggedEffect`/`ConwayCertOutcome` surface and its totality tests
  untouched, rather than making the effect non-`Copy` and rippling through B4;
  (3) B4's accumulate comment already declares the effect "a pure function of the
  cert, re-derivable there" — native re-dispatch *is* that re-derivation. B4's
  `owner_tagged` remains the **classification/closure proof** (every gov cert has
  a recognized owner); B5 owns **application**. A test-level cross-check pins that
  the certs B5 mutates on are exactly those B4 owner-tags.
- **OQ-2 (env gap) → explicit `GovCertEnv { current_epoch, drep_activity }`,
  fail-fast when absent.** `drep_activity` is added to BLUE
  `ConwayOnlyDepositParams` (Conway-only, absent from base
  `ProtocolParameters`); `current_epoch` from `state.epoch_state.epoch`. A
  `LedgerState::gov_cert_env()` accessor parallels `conway_deposit_view()`,
  returning `Err(ValidationEnvironmentError::MissingDRepActivityParam)` on the
  Conway path when the param is absent — never a defaulted expiry.
  Vote-delegation (9/10/12/13) and committee (14/15) need no env; only DRep 16/18
  consume it.
- **OQ-3 (committee semantics) → unconditional accumulation, no membership gate
  in B5.** Tag 14 `AuthCommitteeHot{cold,hot}` → `committee_hot_keys[hot.0] =
  cold.0` (insert/overwrite). Tag 15 `ResignCommitteeCold{cold}` → remove every
  `committee_hot_keys` entry whose value == `cold.0` (clears that member's hot
  authorization, matching Ade's hot→cold vote-resolution at governance.rs:197).
  *Rationale:* committee **membership** is enactment-driven (`UpdateCommittee`),
  not in B5's accumulation scope; gating hot-key auth on a possibly-incomplete
  `committee` map is a false-reject risk and is a separable **GOVCERT
  tx-validity** concern (declared follow-up, not a B5 CE). Exact node-rule
  fidelity is env-blocked (OQ-4) anyway.

## Cluster Index (Dependency Order)
1. **PHASE4-B5** — Conway governance certificate accumulation authority —
   primary invariant: *every governance-affecting Conway cert that B4
   owner-tagged to `ConwayGovState` resolves to exactly one explicit
   `ConwayGovState` mutation or a structured reject — applied, never
   observed-and-dropped, never with payloads lost; B5 touches only
   governance-owned fields and never double-applies the delegation/pool half of
   composites; DRep expiry is env-driven with fail-fast; `ConwayGovState` becomes
   cert-accumulated rather than snapshot-frozen (a deliberate, oracle-confirmable
   fingerprint migration).* *(detailed below)*

(Single cluster. B5 **consumes** B4's complete decoder + owner-tagged
classification; introduces **no** new canonical type — `ConwayGovState` already
exists and is already fingerprinted. The follow-up **GOVCERT
committee-membership tx-validity gate** is declared, not implemented in B5.)

---

## Cluster PHASE4-B5 — Conway governance certificate accumulation authority

**Primary invariant:** DC-LEDGER-09 (see registry for full text).

**TCB partition:**
- **BLUE** — `crates/ade_ledger`: new `apply_conway_gov_cert` + `GovCertEnv`
  (in `delegation.rs` or a new `gov_cert.rs`); `state.rs`
  (`ConwayOnlyDepositParams.drep_activity`, `gov_cert_env()`); `error.rs`
  (`MissingDRepActivityParam`); `rules.rs` (accumulation wiring); `fingerprint.rs`
  (existing gov-state serialization — surface for the migration).
- **GREEN** — `crates/ade_testkit`: `snapshot_loader.rs` (parse + populate
  `drep_activity`); synthetic gov-state corpus harness; replay/adversarial
  drivers.
- **RED** — none.

**Cluster Exit Criteria:**
- **CE-1 (apply totality/closure):** `apply_conway_gov_cert` is
  compiler-exhaustive over all 18 `ConwayCert` tags; a totality test pins each
  gov tag → its `ConwayGovState` mutation, and pins CertState-only + removed tags
  → `gov_state` unchanged.
- **CE-2 (owner exclusivity + no double-apply):** gov apply never mutates
  `CertState`; composite certs (10/12/13) apply the governance half exactly once,
  with the delegation/pool half left untouched by B5 (cross-checked against B4's
  `owner_tagged`).
- **CE-3 (env completeness + fail-fast):** DRep expiry (16/18) =
  `current_epoch + drep_activity` from an explicit `GovCertEnv`;
  `gov_cert_env()` returns `MissingDRepActivityParam` when the Conway param is
  absent — never a defaulted expiry. Env-free tags consume no env.
- **CE-4 (fail-closed wiring):** the B4 observe-and-drop at the accumulation site
  is replaced by application; a gov-apply error propagates as `LedgerError` and
  halts the block transition; the `// routed out of B4 mutation scope` comment is
  gone, no gov cert silently dropped.
- **CE-5 (replay + fingerprint migration):** replaying the synthetic block
  stream yields **byte-identical** accumulated `ConwayGovState` (via
  `fingerprint.rs`); the gov-state fingerprint migration is intentional,
  confirmed correct where a reference exists, and **environment-blocked +
  documented** for the real epoch-576 VState oracle (reclassified per tier
  doctrine, as DC-LEDGER-08/DC-TXV-06).
- **CE-6 (adversarial no-false-accept):** an adversarial gov-cert corpus
  (missing-env DRep, malformed, removed-tag, double-resign, etc.) is rejected
  fail-closed; no mutation yields a silently-accepted wrong gov-state.
- **CE-7 (CI gate):** DC-LEDGER-09 flips declared→enforced with a CI script
  (`ci/ci_check_gov_cert_accumulation_closed.sh`, paralleling DC-TXV-06's
  classification-closed gate); bidirectional cross_ref maintained.

**Slices:**
- **S1 — gov-cert environment infrastructure** — invariant: env-completeness
  scaffold (I-3) — add `GovCertEnv { current_epoch, drep_activity }`,
  `ConwayOnlyDepositParams.drep_activity` (BLUE), `LedgerState::gov_cert_env()` +
  `MissingDRepActivityParam` fail-fast (paralleling `conway_deposit_view()`), and
  snapshot-loader population of `drep_activity` (GREEN; loader currently stops at
  `drep_deposit` — extend the Conway PP read, exact field index pinned in
  slice-doc against the real snapshot). Nothing consumes the env yet — additive,
  mergeable. — addresses: CE-3 — TCB: BLUE + GREEN.
- **S2 — native gov apply model (complete, unwired)** — invariant: closure +
  owner-exclusivity + env-driven expiry (I-1, I-2) — `apply_conway_gov_cert`
  total over all 18 tags: vote-deleg (9/10/12/13)→`vote_delegations`; committee
  (14/15)→`committee_hot_keys` per OQ-3; DRep (16/17/18)→`drep_expiry` (16/18 use
  `GovCertEnv`, 17 removes); CertState-only + removed tags → `gov_state`
  unchanged. Fully unit-tested (per-tag, totality, composite no-double-apply,
  determinism); not yet wired into block application. — addresses: CE-1, CE-2,
  CE-3 — TCB: BLUE.
- **S3 — accumulation wiring (fingerprint migration)** — invariant: fail-closed
  application + provenance migration (I-4, I-5) — thread `Option<ConwayGovState>`
  through `process_block_certificates`/`accumulate_tx_certs`, build
  `gov_cert_env()` from state (fail-fast on Conway when absent), call
  `apply_conway_gov_cert` per Conway cert, wire the accumulated gov-state into the
  new `LedgerState.gov_state` at both `apply_block` sites (rules.rs:286/369), and
  **remove** the B4 observe-and-drop. `gov_state` becomes cert-accumulated. —
  addresses: CE-4, CE-5 (migration) — TCB: BLUE.
- **S4 — corpus, oracle, CI gate** — invariant: replay-equivalence + adversarial
  closure + enforcement (I-4, replay obligation) — synthetic positive
  gov-accumulation corpus + replay-byte-identical assertion (via
  `fingerprint.rs`) + adversarial corpus (missing-env DRep, malformed/removed-tag,
  double-resign) proving fail-closed no-false-accept + documented
  environment-blocked real epoch-576 VState oracle (README, reclassified); add
  `ci/ci_check_gov_cert_accumulation_closed.sh`; flip DC-LEDGER-09 → enforced and
  populate its `code_locus`/`tests`/`ci_script`. — addresses: CE-5, CE-6, CE-7 —
  TCB: GREEN (corpus/harness) + BLUE (registry/CI wiring).

**Replay obligations:** no new canonical type (reuses `ConwayGovState` + its
existing `fingerprint.rs` serialization). New authoritative **accumulation**
(gov-state now cert-derived) ⇒ new synthetic replay-corpus entries (S4) and a
**deliberate fingerprint migration** on the existing gov-state fingerprint
surface (S3, T-DET-01). Replay command: `cargo test -p ade_testkit`.

**FC/IS partition:** dependencies flow inward only — GREEN testkit
(loader/corpus) depends on BLUE ledger types; BLUE never depends on testkit. No
RED in this cluster.

## Declared follow-up (NOT a B5 CE)
- **GOVCERT committee-membership tx-validity gate** — validating that
  `AuthCommitteeHot`'s cold credential is an active/proposed (non-resigned)
  committee member is a tx-validity (GOVCERT) concern, separable from gov-state
  accumulation. Declared here for a future DC-TXV strengthening; B5 accumulates
  unconditionally (OQ-3).
