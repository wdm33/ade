# Cluster OQ5-CREDENTIAL-FIDELITY — credential key/script discriminant fidelity

> **Status:** Planning artifact (non-normative). Authority lives in
> `docs/ade-invariant-registry.toml` (`DC-LEDGER-10`, strengthens
> `T-DET-01`/`T-ENC-03`) and the Cardano ledger spec. Produced via `/cluster-doc`
> from `docs/planning/oq5-credential-discriminant-invariants.md` and
> `docs/planning/oq5-credential-fidelity-cluster-slice-plan.md`. If this doc
> conflicts with the registry or specs, those win.

---

## Primary invariant

> Credential identity (key-hash vs script-hash) is faithful end-to-end. Both era
> certificate decoders preserve the 0/1 key/script discriminant (an unknown tag
> is a deterministic reject); `StakeCredential` is a closed sum
> `{KeyHash(Hash28), ScriptHash(Hash28)}` so a tag-erased credential is
> unrepresentable on the BLUE authority path; `CertState`
> (registrations/delegations/rewards) and `ConwayGovState`
> (vote_delegations/committee_hot_keys/drep_expiry) key on the discriminated
> credential, so a key-hash and a script-hash credential sharing 28 bytes are
> distinct authoritative-state keys (matching cardano-node's `Credential`-keyed
> `UMap`/`VState`); and the canonical fingerprint serializes the discriminant.
> The discriminated representation is a deliberate cert-state + gov-state
> fingerprint migration (T-DET-01). A silent key/script collapse is a forbidden
> fidelity defect. See `DC-LEDGER-10`.

### Slice boundary (the load-bearing decisions)

- **OQ-A → `StakeCredential` becomes the enum, in place.** A fidelity
  correction, not a new feature: the type name already represents the authority
  concept; only its shape was wrong. NOT a `(tag, Hash28)` tuple key (preserves
  the discriminant as data, not domain meaning); NOT a reuse of the `Vec<u8>`
  `address::Credential`.
- **OQ-B → `ConwayGovState` maps re-key to the discriminated `StakeCredential`**
  (from bare `Hash28`).
- **OQ-C → `DRep` stays its own 4-variant type** (`KeyHash`/`ScriptHash`/
  `AlwaysAbstain`/`AlwaysNoConfidence`); already discriminated; not merged.
- **OQ-F → Shelley+ only.** Byron carries no `StakeCredential`. Withdrawal /
  required-signer / address credential fidelity is a non-goal.

---

## Grounding (verified against HEAD)

1. **Both decoders drop the tag.** `decode_stake_credential` in
   `crates/ade_codec/src/shelley/cert.rs:143` (`_tag`) and
   `crates/ade_codec/src/conway/cert.rs:236` (`_cred_type`) read the 0/1 type tag
   and discard it, returning `StakeCredential(hash)`.
2. **`DRep` already discriminates** (`conway/cert.rs:258-259`) — an existing
   asymmetry the credential decode must match.
3. **Shape + keys.** `StakeCredential(pub Hash28)` (`shelley/cert.rs:46`);
   `CertState` keys on `StakeCredential` (`delegation.rs:48-52`);
   `ConwayGovState` keys on bare `Hash28` (`state.rs:87-102`);
   `fingerprint.rs:394 write_stake_credential` = `write_hash28(cred.0)` (no
   discriminant); gov maps written as bare `Hash28`.
4. **Blast radius:** 13 `StakeCredential(` construction sites across
   `ade_codec` (2), `ade_ledger` (delegation, gov_cert, cert_classify, rules), 5
   test files, `ade_types`, `ade_testkit` loader. The credential decode surface
   is the two cert decoders only — withdrawals/required-signers do not construct
   `StakeCredential`. Byron path has none.
5. **`address::Credential`** (`address/mod.rs:45`) is `Vec<u8>`-shaped — a
   reference pattern, not the authority type.

---

## Per-surface change (resolved — load-bearing)

| surface | before | after |
|---|---|---|
| type | `struct StakeCredential(pub Hash28)` | `enum StakeCredential { KeyHash(Hash28), ScriptHash(Hash28) }` |
| shelley decode | `_tag` dropped → `StakeCredential(hash)` | tag → `KeyHash`/`ScriptHash`; unknown tag → reject |
| conway decode | `_cred_type` dropped → `StakeCredential(hash)` | tag → `KeyHash`/`ScriptHash`; unknown tag → reject |
| CertState keys | `BTreeMap<StakeCredential,_>` (collapsed) | same key type, now discriminated |
| ConwayGovState | `BTreeMap<Hash28,_>` ×3 | `BTreeMap<StakeCredential,_>` ×3 (vote_delegations/committee_hot_keys/drep_expiry) |
| gov_cert apply | `credential.0.clone()` (hash) | insert the discriminated `StakeCredential` |
| fingerprint | `write_hash28(cred.0)` | `discriminant ‖ hash`; gov-map writers likewise |
| loader (GREEN) | builds collapsed credential | preserves key/script tag from snapshot |

`DRep` unchanged. Withdrawal/required-signer credentials unchanged (non-goal).

---

## Type model (S1)

```rust
// BLUE — ade_types::shelley::cert. Closed, discriminated. Ord over
// (discriminant, hash) for deterministic BTreeMap keying.
pub enum StakeCredential {
    KeyHash(Hash28),
    ScriptHash(Hash28),
}
```

---

## Entry Conditions
- B4 closed: owner-complete `ConwayCert`, `apply_conway_cert`, era-dispatched
  accumulation.
- B5 closed: `apply_conway_gov_cert`, `ConwayGovState` cert-accumulation,
  fingerprint gov-state serialization.
- Fingerprint migration machinery exists (Conway-deposit tag, golden tests,
  `FINGERPRINT_VERSION`).

---

## Exit Criteria (CI-Verifiable)

- [ ] **CE-1 (decode fidelity):** `shelley_credential_preserves_discriminant`,
  `conway_credential_preserves_discriminant`, `unknown_credential_tag_rejects`.
  *(S1)*
- [ ] **CE-2 (illegal-states-unrepresentable):** `StakeCredential` is a closed
  2-variant enum; `ci/ci_check_credential_discriminant_closed.sh` rejects a
  `pub struct StakeCredential(... Hash28)` shape and any `_ =>`-erasing decode.
  *(S1 type; S2 gate)*
- [ ] **CE-3 (state keys discriminated):**
  `keyhash_scripthash_same_bytes_are_distinct_certstate`,
  `keyhash_scripthash_same_bytes_are_distinct_govstate`. *(S1 type; S2 corpus)*
- [ ] **CE-4 (fingerprint fidelity):** `discriminant_changes_fingerprint`;
  affected goldens regenerated. *(S1)*
- [ ] **CE-5 (replay + dual migration):**
  `credential_accumulation_replays_byte_identical`; migration documented in
  `corpus/credential_fidelity/README.md`. *(S2)*
- [ ] **CE-6 (adversarial / no-false-accept):** unknown-tag reject + collision
  distinctness corpus — no silent overwrite. *(S2)*
- [ ] **CE-7 (CI + enforce):** `ci/ci_check_credential_discriminant_closed.sh`
  green; DC-LEDGER-10 declared→enforced; `ci_check_constitution_coverage.sh`
  PASS. *(S2)*

> No human review may substitute for these checks.

---

## Expected Slice Types
- **S1** — canonical-type change (struct→enum) + decoder fidelity + forced
  consumer updates + dual fingerprint migration + GREEN loader. *(CE-1..CE-4)*
- **S2** — distinctness/replay/adversarial corpus + CI gate + registry
  enforcement. *(CE-5..CE-7)*

---

## TCB Color Map
- **BLUE** — `ade_types` (`StakeCredential`); `ade_codec`
  (`shelley/cert.rs`, `conway/cert.rs` decoders); `ade_ledger`
  (`delegation.rs`, `state.rs`, `gov_cert.rs`, `cert_classify.rs`, `rules.rs`,
  `fingerprint.rs`).
- **GREEN** — `ade_testkit` (`snapshot_loader.rs`; corpus harness).
- **RED** — none. Color resolved; no open color questions.

---

## Forbidden During This Cluster
- Any `Hash28 → StakeCredential` coercion or a credential constructor that takes
  a bare hash without a discriminant.
- Dropping the key/script tag at decode (the `_tag`/`_cred_type` discard must be
  gone).
- A `(tag, Hash28)` tuple key standing in for the discriminated enum on the BLUE
  path.
- Collapsing a key-cred and script-cred with equal bytes into one map entry.
- Dragging Byron / withdrawal / required-signer credentials into the migration
  (non-goals).
- A fingerprint that omits the discriminant.

---

## Environment-blocked (reclassified per tier doctrine)
- Real epoch-576 (and boundary) gov-state/cert-state agreement vs cardano-node's
  discriminated `Credential` keying — env-blocked (snapshots absent locally,
  recoverable from the ImmutableDB EBS snapshots). Same posture as
  DC-LEDGER-08/09 / DC-TXV-06.

## Declared non-goals
- Withdrawal / required-signer / address credential discriminant fidelity.
- Byron credential surface.
