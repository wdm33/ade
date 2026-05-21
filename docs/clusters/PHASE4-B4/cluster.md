# Cluster PHASE4-B4 — Conway certificate-state accumulation fail-closed path

> **Status:** Planning artifact (non-normative). Authority lives in
> `docs/ade-invariant-registry.toml` (`DC-LEDGER-08`, strengthens `DC-VAL-06`) and
> the Conway/Cardano ledger spec. Produced via `/cluster-doc PHASE4-B4` from
> `docs/planning/phase4-b4-invariants.md` and `docs/planning/phase4-b4-cluster-slice-plan.md`.
> If this doc conflicts with the registry or specs, those win.

---

## Primary invariant

> `process_block_certificates` either produces complete canonical **B4-owned**
> `CertState` (delegation + pool) **or** a deterministic structured `LedgerError`.
> Conway certs decode through the **completed** closed Conway grammar (tags 0..18)
> via explicit era dispatch — never the Shelley 6-variant decoder, never reduced
> into the 7-variant Shelley `Certificate`, never with payload fields dropped.
> Every `ConwayCert` is classified to an **owner-tagged** disposition: it mutates
> B4-owned `CertState`, or it is owner-tagged to `ConwayGovState` and routed
> out-of-mutation-scope (observed, not swallowed, not applied by B4), or it is a
> structured reject. **No cert is decode-dropped, apply-swallowed, reduced into
> Shelley, or flattened to "neutral" because B4 has nowhere to put it.** A false
> accept (wrong B4-owned cert-state silently constructed) is release-blocking.

### Slice boundary (the load-bearing decision)

B4 owns **delegation + pool** cert-state accumulation. It does **not** wire
cert→`ConwayGovState` mutation — that is a future governance-accumulation cluster
(**PHASE4-B5**, declared below). The governance-affecting certs (vote-delegation,
committee, DRep) are decoded fully and **owner-tagged to `ConwayGovState`**, then
routed as out-of-mutation-scope. They are explicitly **not** Neutral (the owner
exists — flattening them would reproduce the Shelley-decoder bug class at finer
granularity), **not** Unsupported (the owner is not absent), and **not** applied
by B4 (that would broaden the cluster into governance-state replay,
ratification/enactment interaction, and a CE-72 fingerprint migration).

---

## Grounding (verified against HEAD)

1. **The swallow rationale is false.** `process_block_certificates`
   (`crates/ade_ledger/src/rules.rs:1383-1466`) runs **only** at the two
   `track_utxo == true` call sites (`rules.rs:286`, `rules.rs:369`). The
   "non-fatal during replay without full UTxO state" comment at `rules.rs:1433`
   is self-contradictory: the swallow fires precisely when full state **is**
   present.

2. **Two cert grammars, not one per era.** `ade_codec` has exactly
   `shelley::cert::decode_certificates` (Shelley→Babbage; identical grammar) and
   `conway::cert::decode_conway_certs` (Conway). Byron uses a non-`ShelleyBlock`
   path and never reaches this function. "All eras unified" = explicit
   `CardanoEra` dispatch selecting between these two decoder+apply pairs.

3. **The shared Conway decoder is a deposit-purpose, lossy projection.**
   `decode_conway_certs` doc: *"Only the deposit/refund-relevant fields are
   retained; every other field is structurally consumed during decode and
   dropped."* Confirmed in `crates/ade_codec/src/conway/cert.rs`: tag 2
   `StakeDelegation` carries **no credential/pool_id**; tags 11/12/13 drop
   `_credential` (lines 113/119/125); tags 16/17 drop `_drep_credential`
   (lines 134/139). `apply_conway_cert` therefore **cannot** mutate
   `DelegationState.delegations` (credential→pool) from today's `ConwayCert`.
   **Finding A** below requires additively enriching the single shared decoder —
   not a second decoder.

4. **The governance owners exist, on a different accumulation path.**
   `CertState = { delegation: DelegationState, pool: PoolState }`
   (`delegation.rs:21`) — no DRep/committee/vote state. But
   `LedgerState.gov_state: Option<ConwayGovState>` (`state.rs:83`) owns
   `vote_delegations`, `committee`, `committee_hot_keys`, `drep_expiry`. Crucially
   `gov_state` is **snapshot-loaded** (`"Loaded from UMap"`), not accumulated from
   certs during block replay; it is read only by ratification/enactment
   (`rules.rs:1152+`, the CE-72 path). Wiring cert→`ConwayGovState` accumulation is
   out of B4 scope (PHASE4-B5).

---

## Per-variant cert-state effect table (resolved — load-bearing)

Closed and total over Conway CDDL certificate tags `0..18`. Each row's disposition
is exactly one owner-tagged outcome (composites carry **two**). This table is the
`DC-LEDGER-08` totality proof obligation and the source for the
`conway_cert_action_total` test (machine-checkable fixture, never a hand-coded copy).

| tag | constructor | B4-owned effect | gov-owned effect | disposition |
|---|---|---|---|---|
| 0 | `account_registration_cert` | register credential (deposit recorded) | — | `MutateDelegationState(Register)` |
| 1 | `account_unregistration_cert` | deregister credential (refund recorded deposit) | — | `MutateDelegationState(Unregister)` |
| 2 | `delegation_to_stake_pool_cert` | set credential→pool | — | `MutateDelegationState(Delegate)` |
| 3 | `pool_registration_cert` | register/update pool params | — | `MutatePoolState(Register)` |
| 4 | `pool_retirement_cert` | schedule retirement (refund at POOLREAP) | — | `MutatePoolState(ScheduleRetirement)` |
| 5/6 | removed (genesis-deleg / MIR) | — | — | `Reject(NotValidInEra)` |
| 7 | `account_registration_deposit_cert` | register credential (explicit deposit) | — | `MutateDelegationState(RegisterWithDeposit)` |
| 8 | `account_unregistration_deposit_cert` | deregister (explicit refund, must match recorded) | — | `MutateDelegationState(UnregisterWithRefund)` |
| 9 | `delegation_to_drep_cert` | — | vote delegation | `MutateGovernanceState{ConwayGovState, VoteDelegation}` |
| 10 | `delegation_to_stake_pool_and_drep_cert` | set credential→pool | vote delegation | **composite**: `MutateDelegationState(Delegate)` **+** `MutateGovernanceState{…, StakeVoteDelegation}` |
| 11 | `account_registration_delegation_to_stake_pool_cert` | register + delegate-to-pool | — | `MutateDelegationState(RegisterAndDelegate)` |
| 12 | `account_registration_delegation_to_drep_cert` | register | vote delegation | **composite**: `MutateDelegationState(Register)` **+** `MutateGovernanceState{…, VoteDelegation}` |
| 13 | `…_to_stake_pool_and_drep_cert` | register + delegate-to-pool | vote delegation | **composite**: `MutateDelegationState(RegisterAndDelegate)` **+** `MutateGovernanceState{…, StakeVoteDelegation}` |
| 14 | `committee_authorization_cert` | — | committee hot-key auth | `MutateGovernanceState{…, CommitteeHotKeyAuthorization}` |
| 15 | `committee_resignation_cert` | — | committee cold resign | `MutateGovernanceState{…, CommitteeColdKeyResignation}` |
| 16 | `drep_registration_cert` | — | DRep registration (deposit) | `MutateGovernanceState{…, DRepRegistration}` |
| 17 | `drep_unregistration_cert` | — | DRep unregistration (refund) | `MutateGovernanceState{…, DRepUnregistration}` |
| 18 | `drep_update_cert` | — | DRep update | `MutateGovernanceState{…, DRepUpdate}` |

**Honesty check — no Conway cert is `Neutral`.** Every defined tag has an owner
(B4 `CertState` or `ConwayGovState`) or is removed. The `Neutral` disposition maps
to ∅ in Conway and exists only for closed-enum totality: if a slice is ever tempted
to classify a Conway cert as `Neutral`, it has mis-owned the cert — the exact
flattening this cluster forbids.

**Composite certs (10/12/13) split across owners.** A single cert produces both a
B4-owned `CertState` mutation **and** an owner-tagged `ConwayGovState` effect. The
apply outcome must represent **both** — see B4-S2 design note; the binary
`{Applied | OwnerTaggedOutOfScope}` shape is insufficient.

---

## Action + outcome model (B4-S2)

```rust
// Classification — total over tags 0..18. A cert yields 1..2 actions.
enum ConwayCertAction {
    MutateDelegationState(DelegationMutation),
    MutatePoolState(PoolMutation),
    // Decoded and owner-identified, but NOT applied by B4.
    MutateGovernanceState { owner: GovernanceOwner, effect: GovernanceCertEffect },
    Neutral(NeutralCertReason),       // ∅ for Conway; closed-enum totality only
    Reject(CertRejectReason),
}

enum GovernanceOwner { ConwayGovState }

enum GovernanceCertEffect {
    VoteDelegation, StakeVoteDelegation,
    CommitteeHotKeyAuthorization, CommitteeColdKeyResignation,
    DRepRegistration, DRepUnregistration, DRepUpdate,
}

enum CertRejectReason { NotValidInEra, Malformed, UnsupportedUntilStateOwner }

// Apply — extended past the binary sketch to carry composite outcomes.
struct ConwayCertOutcome {
    state: CertState,                          // B4-owned state after applying the B4 part
    owner_tagged: Vec<OwnerTaggedEffect>,      // gov effects observed, routed out-of-scope, never swallowed
}
struct OwnerTaggedEffect { owner: GovernanceOwner, effect: GovernanceCertEffect }

fn apply_conway_cert(state: &CertState, cert: &ConwayCert, env: &ConwayCertEnv)
    -> Result<ConwayCertOutcome, CertApplyError>;
```

`process_block_certificates` (B4-S3) must route `owner_tagged` effects as observed
out-of-scope (e.g. counted/logged into the verdict), **not** discarded as success
and **not** applied. The positive corpus proves they do not perturb B4-owned state.

---

## Normative anchors

- **`DC-LEDGER-08`** (declared) — cert-state accumulation closure (this cluster
  introduces it).
- Strengthens **`DC-VAL-06`** — removes the `Err(_) => /* non-fatal */` silent-skip.
- Anchored by **`T-DET-01`** — B4-owned `CertState` fingerprint; intentional
  migration (see Replay obligations).
- Conway CDDL `certificate` grammar (`eras/conway/impl/cddl/data/conway.cddl`,
  IntersectMBO/cardano-ledger); B3 classification table
  `corpus/conway_certs/classification_table.md` (deposit projection — B4 enriches
  the same decoder).

---

## Entry conditions

- B3 closed: `decode_conway_certs` (closed grammar, tags 0..18, `UnknownCertTag`,
  trailing-bytes-strict); `LedgerError` carries `From<CodecError>`,
  `EraInvalidCertificate`, `UnsupportedStateDependentDeposit`.
- Snapshot loader exposes prior `CertState` + resolved state over the Conway-576
  window at `track_utxo=true` (`project_snapshot_loader_done`).
- `ConwayGovState` exists and is snapshot-loaded (`state.rs:83`) — confirms the
  governance owner exists and is **out of B4 mutation scope**.
- Current `process_block_certificates` uses the Shelley decoder + double swallow +
  ignored `_era` — the thing this cluster removes.

---

## Exit criteria (CI-verifiable)

The cluster is complete only when:

- [ ] **CE-B4-1** — `cargo test -p ade_codec --test conway_cert_decode_complete` PASS:
  `ConwayCert` + `decode_conway_certs` **additively enriched** to retain every
  owner payload (delegation credential, pool id, DRep credential, vote-delegation
  target, committee hot/cold credentials) alongside B3's deposit/refund fields;
  B3's deposit/conservation tests stay green (valid projection over the enriched
  type); decoder remains the **single** Conway cert authority (no second decoder);
  trailing-bytes/unknown-tag strictness preserved.
- [ ] **CE-B4-2** — `cargo test -p ade_ledger --lib delegation::conway_apply` PASS:
  `apply_conway_cert` + `ConwayCertAction`/`ConwayCertOutcome` total over all 18
  variants (`conway_cert_action_total`, machine-checkable fixture); composite certs
  10/12/13 yield both a `CertState` mutation and an owner-tagged effect; **no path
  reduces `ConwayCert` into Shelley `Certificate`**; **no Conway cert maps to
  `Neutral`**; `CertApplyError` taxonomy closed. Pure BLUE, unwired.
- [ ] **CE-B4-3** — `cargo test -p ade_ledger --test cert_state_era_dispatch` PASS:
  `process_block_certificates` selects decoder+apply by `CardanoEra`
  (Conway→enriched `decode_conway_certs`+`apply_conway_cert`; Shelley..Babbage→
  `decode_certificates`+`apply_cert`); the `_era` discard at `rules.rs:1385` is
  gone; decode errors propagate as structured `LedgerError`; owner-tagged gov
  effects are routed out-of-scope (observed, not swallowed, not applied).
- [ ] **CE-B4-4** — `cargo test -p ade_ledger --test cert_state_apply_fail_closed`
  PASS: the apply-error swallow at `rules.rs:1433-1436` removed for both paths; an
  apply error halts the block transition with a structured `LedgerError`; the false
  "non-fatal during replay" comment deleted; `ci_check_forbidden_patterns.sh`
  (extended) shows no `Err(_) => { /* non-fatal */ }` in
  `process_block_certificates`.
- [x] **CE-B4-5** — `cargo test -p ade_ledger --test cert_state_corpus` PASS:
  positive (synthetic, controlled state) → complete B4-owned `CertState`, **state
  stream byte-identical across two runs**; adversarial corpus (malformed CBOR,
  unknown tag ≥19, Conway-removed tag 5/6, truncated, trailing bytes,
  unregistered-delegation) → structured reject. **No false accept.**
  **Environment-blocked (reclassified, documented):** the real epoch-576
  `CertState`-vs-cardano-node oracle cannot run here — the epoch-576 UMap snapshot
  was deleted post-extraction and is not in the repo (identical to the B3-S5
  constraint; see `corpus/cert_state/README.md`). The real-corpus agreement is an
  open obligation reclassified environment-blocked per tier doctrine, exactly as
  `DC-TXV-06`/`DC-TXV-03` did. The synthetic positive + replay + adversarial gate
  is the mechanical closure B4 ships.

  **Governance exit rule (binding):** real Conway cert-bearing blocks may contain
  governance-affecting certs. In B4 they must **decode fully and be owner-tagged as
  `ConwayGovState` effects** — never treated as `Neutral`, `Unsupported`, malformed,
  or applied to B4-owned `CertState`. `UnsupportedUntilStateOwner` must **not** be
  reachable on the real corpus (it is release-blocking); it exists only for unit
  tests of genuinely-ownerless hypotheticals before an owner is introduced.

> No human review may substitute for these checks.

---

## Slices (dependency order)

> Reordered/expanded vs the original 4-slice sketch: a **decoder-enrichment slice
> (B4-S1)** was split out because Finding A makes it a prerequisite in a *different
> crate* (`ade_codec`) and it is independently mergeable; the apply model then
> consumes the enriched type. Confirmed reorder posture from `/cluster-plan`.

- **B4-S1 — Complete the shared Conway cert decoder** (`ade_codec`) — *invariant:*
  `ConwayCert` + `decode_conway_certs` additively retain all owner payloads;
  B3's deposit projection stays valid; single decoder authority preserved;
  unknown-tag/trailing-bytes strictness intact. — *addresses:* **CE-B4-1** —
  *TCB:* BLUE.
- **B4-S2 — Native Conway cert apply action model** (`ade_ledger`) — *invariant:*
  owner-tagged `ConwayCertAction` + `ConwayCertOutcome` + `apply_conway_cert`;
  total over 18 variants; composites split across owners; no Shelley reduction; no
  `Neutral` for Conway. Pure BLUE, unwired. — *addresses:* **CE-B4-2**; introduces
  `DC-LEDGER-08` — *TCB:* BLUE.
- **B4-S3 — Era-dispatched cert-state decode boundary** — *invariant:* explicit
  `CardanoEra`→decoder+apply dispatch in `process_block_certificates`; `_era`
  discard removed; decode errors fail closed; owner-tagged gov effects routed
  out-of-scope. — *addresses:* **CE-B4-3**; strengthens `DC-VAL-06` — *TCB:* BLUE.
- **B4-S4 — Remove swallowed apply errors (fail-closed propagation)** —
  *invariant:* apply errors on both paths propagate as structured `LedgerError` and
  halt the block transition; no `Err(_)` swallow remains. — *addresses:*
  **CE-B4-4**; strengthens `DC-VAL-06` — *TCB:* BLUE.
- **B4-S5 — Cert-state replay + adversarial corpus + oracle** — *invariant:*
  positive epoch-576 B4-owned cert-state replay byte-identical AND matches the
  reference oracle for the B4-owned surface; gov certs owner-tagged (governance
  exit rule); adversarial → structured reject; no false accept. — *addresses:*
  **CE-B4-5**; strengthens `DC-VAL-06`, anchored by `T-DET-01` — *TCB:* GREEN + RED.

---

## TCB color map (FC/IS partition)

- **BLUE (deterministic, authoritative):**
  - `ade_codec::conway::cert` — enriched `ConwayCert` + `decode_conway_certs` (S1).
  - `ade_ledger::delegation` — `apply_conway_cert`, `ConwayCertAction`,
    `ConwayCertOutcome`, `CertApplyError`, `ConwayCertEnv`, `CertState` transitions (S2).
  - `ade_ledger::rules::process_block_certificates` — `CardanoEra` dispatch +
    fail-closed propagation + owner-tagged routing (S3, S4).
  - reused: `ade_codec::shelley::cert`, existing `LedgerError` taxonomy.
- **GREEN (deterministic glue, non-authoritative):**
  - `ade_testkit` cert-state positive + adversarial corpus harness + cert mutators (S5).
- **RED (nondeterministic shell):**
  - corpus fixtures; snapshot-derived prior `CertState` + resolved state over the
    Conway-576 window; reference-oracle B4-owned cert-state.

Rules: no RED in BLUE; GREEN must not affect authoritative outputs; colors fixed
before any slice begins.

---

## Forbidden during this cluster (inherits B1/B2/B3 + global doctrine)

- Reducing `ConwayCert` (18 variants) into the 7-variant Shelley `Certificate`
  (locked OQ-1) — **or** dropping payload fields inside `ConwayCert` (the same bug
  class at finer granularity; Finding A).
- A **second** Conway cert decoder — B4 *completes* the single shared decoder.
- Classifying any Conway cert as `Neutral` because B4 has nowhere to put it
  (semantic flattening) — owner-tag it instead.
- Wiring cert→`ConwayGovState` mutation in B4 (that is PHASE4-B5).
- `UnsupportedUntilStateOwner` reachable on the real corpus (release-blocking).
- Any `Err(_) => { /* non-fatal */ }` swallow on decode or apply in the
  `track_utxo == true` path; era-blind decode (the `_era` discard); Shelley decoder
  on Conway bytes.
- `HashMap`/float/wall-clock in BLUE; re-encoding for any hash.
- Positive-only coverage — the adversarial corpus (CE-B4-5) is mandatory.
- Touching `.idd-config.json`'s entry-count doc string (OQ-6) unless a mechanical
  registry-count check fails.
- **A false accept is release-blocking.**

---

## Replay obligations

- **New canonical types:** enriched `ConwayCert` (S1); `ConwayCertAction`,
  `ConwayCertOutcome`, `OwnerTaggedEffect`, `CertApplyError`, `ConwayCertEnv` (S2).
  Only the B4-owned `CertState` is persisted/fingerprinted; the action/outcome
  types are intermediate.
- **Behavior/fingerprint change (load-bearing):** wiring real Conway accumulation
  makes the previously-dropped delegation/pool variants actually mutate `CertState`
  → the epoch-576 B4-owned `CertState` fingerprint **will change** vs HEAD. This is
  an *intentional* `T-DET-01` migration; the new value must be confirmed correct
  against the reference oracle (CE-B4-5), else B4 trades a silent fail-open for a
  silent wrong-state false-accept.
- **Owner-tagged effects do not enter the B4 fingerprint** — they are observed and
  routed out-of-scope; `ConwayGovState` remains snapshot-sourced. CE-B4-5 proves
  gov certs do not perturb the B4-owned surface.
- **New corpus:** `corpus/cert_state/{positive,adversarial}/`. Replay
  byte-identically; anchored by `T-DET-01`.

---

## Open items for the slice docs

- **`ConwayCertEnv` exact fields** — `apply_conway_cert` needs: deposit params,
  prior registration/deposit map (for refund matching, tags 1/8), current epoch
  (pool retirement). Resolve in B4-S2 slice doc.
- **Refund-matching semantics (tags 1/8):** Conway explicit-unreg refund (tag 8)
  must *match* the deposit recorded in `DelegationState.registrations`; mismatch →
  structured reject vs. accept-recorded. Mirrors B3 D1; confirm parity.
- **Pool re-registration (tag 3):** new-vs-update resolved against `PoolState`;
  update is not a new deposit (B3 N-5) — confirm the cert-state mutation matches.
- **Composite `ConwayCertEnv` ordering:** for tags 10/12/13, confirm the B4-owned
  mutation and the owner-tagged emission are independent (no ordering hazard).
- **Oracle for B4-owned surface (CE-B4-5):** which cardano-node query / snapshot
  field gives the delegation/pool cert-state to diff against.

---

## Future cluster marker (declared, not promised inside B4)

**PHASE4-B5 — Conway governance certificate accumulation authority** *(declared)*:
replay cert-derived updates into `ConwayGovState` (vote_delegations, committee,
committee_hot_keys, drep_expiry) and prove equivalence against the snapshot/oracle
governance state. Consumes B4's enriched decoder and owner-tagged effects. This is
where the `MutateGovernanceState` effects become *applied* rather than
owner-tagged-out-of-scope.

---

## Authority reminder

Planning aid only. Rule authority: `docs/ade-invariant-registry.toml`; mechanical
acceptance: the named tests/CI. Conflicts resolve to the registry/specs.
