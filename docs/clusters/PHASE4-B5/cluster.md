# Cluster PHASE4-B5 — Conway governance certificate accumulation authority

> **Status:** Planning artifact (non-normative). Authority lives in
> `docs/ade-invariant-registry.toml` (`DC-LEDGER-09`, strengthens `DC-LEDGER-08`)
> and the Conway/CIP-1694 ledger spec. Produced via `/cluster-doc PHASE4-B5` from
> `docs/planning/phase4-b5-invariants.md` and
> `docs/planning/phase4-b5-cluster-slice-plan.md`. If this doc conflicts with the
> registry or specs, those win.

---

## Primary invariant

> Every governance-affecting Conway certificate that B4 owner-tagged to
> `ConwayGovState` (vote-delegation tags 9/10/12/13, committee 14/15, DRep
> 16/17/18) resolves to exactly one explicit `ConwayGovState` mutation **or** a
> deterministic structured `LedgerError` — **applied**, never observed-and-dropped,
> never flattened, never with payload fields lost. B5 mutates **only**
> governance-owned fields (`vote_delegations`, `committee_hot_keys`,
> `drep_expiry`), never B4-owned `CertState`, and never double-applies the
> delegation/pool half of composites (10/12/13). DRep expiry is computed only from
> an explicit `GovCertEnv` (current epoch + `drep_activity`); a missing required
> env input is a structured fail-fast, never a defaulted expiry. `ConwayGovState`
> becomes a deterministic function of (boundary-loaded base ∘ replayed
> block-stream cert effects) rather than a frozen snapshot. A false accept (wrong
> gov-state silently constructed) is release-blocking. See `DC-LEDGER-09`.

### Slice boundary (the load-bearing decisions)

- **OQ-1 → B5-native gov dispatch (additive).** B5 adds a pure
  `apply_conway_gov_cert(&ConwayGovState, &ConwayCert, &GovCertEnv) ->
  Result<ConwayGovState, LedgerError>` reading payloads **directly off** the
  owner-complete `ConwayCert`. It does **not** enrich B4's
  `OwnerTaggedEffect`/`GovernanceCertEffect` (which stay `Copy`, payload-less, and
  closed). B4's `owner_tagged` remains the **classification/closure proof** (every
  gov cert has a recognized owner); B5 owns **application**. This leaves B4's
  closed surface and its totality tests untouched — the IDD-faithful additive move.
- **OQ-2 → explicit `GovCertEnv`, fail-fast.** `drep_activity` (a Conway-only PP
  absent from base `ProtocolParameters`) is added to `ConwayOnlyDepositParams`;
  `current_epoch` from `state.epoch_state.epoch`. `LedgerState::gov_cert_env()`
  parallels `conway_deposit_view()` and returns `Err(MissingDRepActivityParam)` on
  the Conway path when the param is absent.
- **OQ-3 → unconditional committee accumulation.** B5 does **not** gate
  `AuthCommitteeHot` on committee membership; that membership predicate is
  enactment-dependent and a separable **GOVCERT tx-validity** concern (declared
  follow-up, not a B5 CE).

---

## Grounding (verified against HEAD)

1. **The drop site is live and explicit.** `accumulate_tx_certs`
   (`rules.rs:1460-1487`) calls `apply_conway_cert`, takes `state =
   outcome.state`, and **drops** `outcome.owner_tagged` with `// observed and
   routed out of B4 mutation scope` (`rules.rs:1476-1478`). This runs only at
   `track_utxo == true` (call sites `rules.rs:286`, `rules.rs:369`). B5 replaces
   the drop with application.
2. **Payloads are present on `ConwayCert`, absent on the effect.** `ConwayCert`
   (`cert.rs:24-99`) retains `credential`/`drep`/`cold_credential`/
   `hot_credential`/`drep_credential`. `OwnerTaggedEffect` (`delegation.rs:320-324`)
   is `Copy` and carries only the effect *kind*. Native dispatch (OQ-1) reads the
   payloads from the cert.
3. **The owner exists and is already fingerprinted.** `ConwayGovState`
   (`state.rs:83-103`): `vote_delegations: BTreeMap<Hash28, DRep>`,
   `committee_hot_keys: BTreeMap<Hash28, Hash28>` (hot→cold), `drep_expiry:
   BTreeMap<Hash28, u64>`. `fingerprint.rs:278-313` already serializes all three
   canonically — the B5 migration is observable on an existing surface; **no new
   canonical type**.
4. **`committee` membership is enactment-driven.** The `committee` map is set by
   `UpdateCommittee` enactment, not by certs; only `committee_hot_keys` is
   cert-driven (read by vote resolution at `governance.rs:197`). B5 touches
   `committee_hot_keys`, never `committee`.
5. **The env gap is real.** `drep_activity` is absent from BLUE
   `ProtocolParameters`, `ConwayGovState`, and `ConwayOnlyDepositParams`; the GREEN
   loader parses Conway PP only through `drep_deposit` (`snapshot_loader.rs:2565`)
   and must be extended. `current_epoch` is reachable via the `&LedgerState`
   already passed to `process_block_certificates`.
6. **Fail-fast precedent.** `ValidationEnvironmentError::MissingConwayDepositParams`
   + `conway_deposit_view()` (`state.rs:128-138`) is the exact pattern for the new
   `MissingDRepActivityParam` + `gov_cert_env()`.

---

## Per-variant governance effect table (resolved — load-bearing)

Closed over the governance-affecting Conway tags. Each row's mutation is read from
the named `ConwayCert` payload. CertState-only tags (0/2/3/4/7/8/11) and removed
tags (5/6) make **no** gov mutation (gov dispatch returns `gov_state` unchanged).
This table is the `DC-LEDGER-09` totality proof obligation and the source for the
`gov_apply_total_over_18_tags` fixture (machine-checkable, never hand-copied).

| tag | constructor | `ConwayCert` payload read | `ConwayGovState` mutation | env? | B4 half (do not re-apply) |
|---|---|---|---|---|---|
| 9 | `delegation_to_drep` | `{credential, drep}` | `vote_delegations[credential.0] = drep` | no | — |
| 10 | `deleg_to_pool_and_drep` | `{credential, pool_id, drep}` | `vote_delegations[credential.0] = drep` | no | stake→pool |
| 12 | `reg_deleg_to_drep` | `{credential, drep, deposit}` | `vote_delegations[credential.0] = drep` | no | registration |
| 13 | `reg_deleg_to_pool_and_drep` | `{credential, pool_id, drep, deposit}` | `vote_delegations[credential.0] = drep` | no | registration + stake→pool |
| 14 | `committee_authorization` | `{cold_credential, hot_credential}` | `committee_hot_keys[hot.0] = cold.0` (insert/overwrite) | no | — |
| 15 | `committee_resignation` | `{cold_credential}` | remove every `committee_hot_keys` entry where value == `cold.0` | no | — |
| 16 | `drep_registration` | `{drep_credential, deposit}` | `drep_expiry[drep_credential.0] = env.current_epoch + env.drep_activity` | **yes** | — |
| 17 | `drep_unregistration` | `{drep_credential, refund}` | remove `drep_expiry[drep_credential.0]` | no | — |
| 18 | `drep_update` | `{drep_credential}` | `drep_expiry[drep_credential.0] = env.current_epoch + env.drep_activity` | **yes** | — |

**No double-apply.** For composites 10/12/13 the B4 cert-state half (registration /
stake→pool) is already applied by `apply_conway_cert` in the same accumulation
pass; `apply_conway_gov_cert` touches **only** `vote_delegations`. Cross-checked by
`composite_gov_half_applied_once_certstate_untouched_by_b5`.

**Env discipline.** Only tags 16/18 read `env`. The env is built once per block on
the Conway path via `gov_cert_env()`; absence of `drep_activity` is a fail-fast
(`MissingDRepActivityParam`), since `drep_activity` is a mandatory Conway protocol
parameter.

---

## Apply + env model (B5-S1/S2)

```rust
// BLUE — ade_ledger. Conway-only governance environment for cert accumulation.
pub struct GovCertEnv {
    pub current_epoch: u64,   // from state.epoch_state.epoch
    pub drep_activity: u64,   // Conway PP, added to ConwayOnlyDepositParams
}

// BLUE — LedgerState accessor, fail-fast (parallels conway_deposit_view()).
//   Err(ValidationEnvironmentError::MissingDRepActivityParam) when absent on Conway.
pub fn gov_cert_env(&self) -> Result<GovCertEnv, ValidationEnvironmentError>;

// BLUE — native gov dispatch over the owner-complete ConwayCert.
//   Gov-affecting tags mutate ConwayGovState; CertState-only + removed tags
//   return gov_state unchanged. A precondition/env failure is a structured reject.
//   Never mutates CertState; never re-applies the B4 delegation/pool half.
pub fn apply_conway_gov_cert(
    gov_state: &ConwayGovState,
    cert: &ConwayCert,
    env: &GovCertEnv,
) -> Result<ConwayGovState, LedgerError>;
```

---

## Entry Conditions

- **B4 closed (HEAD `644eb03`):** owner-complete `ConwayCert` (tags 0..18, all gov
  payloads), `apply_conway_cert` + `ConwayCertOutcome.owner_tagged` (closed, total
  over 18 tags), era-dispatched fail-closed `accumulate_tx_certs`, `LedgerError`
  taxonomy with `From<CodecError>`. `DC-LEDGER-08` enforced.
- **`ConwayGovState` exists** and is canonically fingerprinted
  (`fingerprint.rs:278-313`).
- **`conway_deposit_view()` fail-fast pattern** exists for the env accessor to
  mirror.
- **Conway PP parser** (`snapshot_loader.rs`) parses gov params through
  `drep_deposit` and can be extended to `drep_activity`.

---

## Exit Criteria (CI-Verifiable)

The cluster is complete only when:
- [ ] **CE-1 (totality):** `apply_conway_gov_cert` is a compiler-exhaustive match
  over all 18 `ConwayCert` variants; `gov_apply_total_over_18_tags` pins each gov
  tag → its `ConwayGovState` mutation and each CertState-only/removed tag →
  `gov_state` unchanged. *(S2)*
- [ ] **CE-2 (owner exclusivity + no double-apply):**
  `gov_apply_never_mutates_certstate` and
  `composite_gov_half_applied_once_certstate_untouched_by_b5` pass. *(S2)*
- [ ] **CE-3 (env + fail-fast):** `drep_expiry_uses_epoch_plus_activity`,
  `gov_cert_env_missing_drep_activity_is_fail_fast`, and
  `env_free_gov_certs_need_no_env` pass. *(S1, S2)*
- [ ] **CE-4 (fail-closed wiring):** `gov_apply_error_halts_block_transition`
  passes; `ci/ci_check_gov_cert_accumulation_closed.sh` asserts the B4 `// routed
  out of B4 mutation scope` drop comment is gone and no gov cert is silently
  dropped. *(S3)*
- [ ] **CE-5 (replay + fingerprint migration):**
  `gov_state_accumulation_replays_byte_identical` passes over the synthetic corpus
  (via `fingerprint.rs`); the gov-state fingerprint migration is documented in
  `corpus/gov_state/README.md` and reclassified env-blocked for the real
  epoch-576 VState oracle. *(S3, S4)*
- [ ] **CE-6 (adversarial no-false-accept):** `gov_adversarial_no_false_accept`
  passes over an adversarial corpus including `drep_update_missing_env_rejected`,
  `malformed_gov_cert_rejected`, and `double_resign_is_deterministic`. *(S4)*
- [ ] **CE-7 (CI gate):** `ci/ci_check_gov_cert_accumulation_closed.sh` is green
  and wired into CI; `DC-LEDGER-09` flips `declared`→`enforced` with populated
  `code_locus`/`tests`/`ci_script`; `ci/ci_check_constitution_coverage.sh` passes
  (bidirectional `DC-LEDGER-08`↔`DC-LEDGER-09` cross_ref). *(S4)*

> No human review may substitute for these checks.

---

## Expected Slice Types

- **S1 — environment infrastructure:** new canonical env type + state-field
  addition + fail-fast accessor + GREEN loader population. *(addresses CE-3)*
- **S2 — pure state-transition definition:** native total apply function, fully
  unit-tested, unwired. *(addresses CE-1, CE-2, CE-3)*
- **S3 — authoritative-path wiring + fingerprint migration:** thread accumulated
  `ConwayGovState` through block application, remove the B4 drop. *(addresses
  CE-4, CE-5)*
- **S4 — replay/adversarial corpus + CI gate + registry enforcement.**
  *(addresses CE-5, CE-6, CE-7)*

---

## TCB Color Map (FC/IS Partition)

- **BLUE (deterministic, authoritative):**
  - `crates/ade_ledger/src/delegation.rs` (or new `gov_cert.rs`) —
    `apply_conway_gov_cert`, `GovCertEnv`
  - `crates/ade_ledger/src/state.rs` — `ConwayOnlyDepositParams.drep_activity`,
    `LedgerState::gov_cert_env()`
  - `crates/ade_ledger/src/error.rs` —
    `ValidationEnvironmentError::MissingDRepActivityParam`
  - `crates/ade_ledger/src/rules.rs` — accumulation wiring
    (`process_block_certificates`, `accumulate_tx_certs`, `apply_block` gov-state
    threading)
  - `crates/ade_ledger/src/fingerprint.rs` — existing gov-state serialization
    (migration surface; no change expected beyond confirming coverage)
- **GREEN (deterministic glue, non-authoritative):**
  - `crates/ade_testkit/src/harness/snapshot_loader.rs` — parse + populate
    `drep_activity`
  - `crates/ade_testkit/src/...` — synthetic gov-state corpus harness, replay +
    adversarial drivers
- **RED (nondeterministic shell):** none.

Color is resolved; no open color questions.

---

## Forbidden During This Cluster

- Silently dropping any governance cert effect (the B4 observe-and-drop must be
  **replaced**, not relabeled).
- Computing DRep expiry from a defaulted/guessed epoch or `drep_activity` (must
  come from `GovCertEnv`; absence is a fail-fast).
- Mutating B4-owned `CertState`, the `committee` membership map, proposals,
  reserves, or treasury from B5's gov dispatch.
- Re-applying the delegation/pool half of composite certs 10/12/13 (B4 owns it).
- Enriching/weakening B4's closed `OwnerTaggedEffect`/`GovernanceCertEffect`
  surface or its totality tests (OQ-1 chose additive native dispatch precisely to
  avoid this).
- Adding a committee-membership precondition gate (separable GOVCERT tx-validity
  follow-up; out of scope).
- Any TODO/deferred logic on the authoritative gov-accumulation path.

---

## Declared follow-up (NOT a B5 CE)

- **GOVCERT committee-membership tx-validity gate** — validating that
  `AuthCommitteeHot`'s cold credential is an active/proposed (non-resigned)
  committee member is a tx-validity (GOVCERT) concern, separable from gov-state
  accumulation; a future `DC-TXV` strengthening. B5 accumulates unconditionally
  (OQ-3).
