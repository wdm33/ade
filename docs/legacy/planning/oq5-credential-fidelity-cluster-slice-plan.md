# Cluster/Slice Plan — Ade / OQ5-CREDENTIAL-FIDELITY

> **Status**: planning artifact, non-normative. Authority lives in
> `docs/ade-invariant-registry.toml` (DC-LEDGER-10) and
> `docs/planning/oq5-credential-discriminant-invariants.md`. Produced via
> `/cluster-plan`. Distinct from the phase-level
> `docs/active/phase_4_cluster_plan.md`, which this doc does not modify.

## Inputs
- `docs/planning/oq5-credential-discriminant-invariants.md` — OQ-5 sketch
  (I-1..I-4; OQ-A..OQ-F).
- `docs/ade-invariant-registry.toml` — **DC-LEDGER-10** (declared); strengthens
  T-DET-01 / T-ENC-03; relates DC-LEDGER-08 / DC-LEDGER-09 / DC-TXV-05.

## Grounding (verified at HEAD)
- Collapse points: `decode_stake_credential` in BOTH `ade_codec` decoders drop
  the tag (`shelley/cert.rs:143`, `conway/cert.rs:236`). `DRep` already
  discriminates (4 variants: KeyHash/ScriptHash/AlwaysAbstain/AlwaysNoConfidence).
- `StakeCredential(pub Hash28)` (`shelley/cert.rs:46`); 13 construction sites
  across 2 codec, 4 ledger-src, 5 test, 1 types, 1 testkit file.
- `CertState` keys on `StakeCredential` (`delegation.rs:48-52`); `ConwayGovState`
  keys on bare `Hash28` (`state.rs:87-102`); `fingerprint.rs:394` writes hash
  only.
- Credential decode surface = the two cert decoders only (withdrawals /
  required-signers do NOT construct `StakeCredential` — out of scope). Byron
  carries no `StakeCredential` (Shelley+ scope confirmed).

## OQ resolutions (locked)
- **OQ-A** → `StakeCredential` becomes `enum { KeyHash(Hash28), ScriptHash(Hash28) }`
  in place (user directive). NOT a `(tag, Hash28)` tuple key; NOT a reuse of the
  `Vec<u8>`-shaped `address::Credential`.
- **OQ-B** → `ConwayGovState` maps re-key from bare `Hash28` to the discriminated
  `StakeCredential` (`vote_delegations: BTreeMap<StakeCredential, DRep>`;
  `committee_hot_keys: BTreeMap<StakeCredential, StakeCredential>`;
  `drep_expiry: BTreeMap<StakeCredential, u64>`).
- **OQ-C** → DRep stays its own 4-variant type (superset of key/script); not
  merged with StakeCredential; already correct.
- **OQ-D** → real-oracle agreement env-blocked, reclassified per tier doctrine
  (DC-LEDGER-08/09 precedent).
- **OQ-E** → dual cert-state + gov-state fingerprint migration accepted;
  regenerate goldens + any present boundary pins.
- **OQ-F** → Shelley+ only; Byron out of scope. Withdrawal/required-signer
  credential fidelity is an explicit non-goal of this cluster.

## Cluster Index (Dependency Order)
1. **OQ5-CREDENTIAL-FIDELITY** — primary invariant: *credential identity
   (key-hash vs script-hash) is faithful end-to-end — both era decoders preserve
   the discriminant, the credential type makes a tag-erased credential
   unrepresentable, CertState + ConwayGovState key on the discriminated
   credential, and the canonical fingerprint serializes the discriminant
   (DC-LEDGER-10).*

(Single cluster. Consumes B4/B5's closed cert/gov surfaces; an atomic
type-fidelity refactor + dual fingerprint migration, not a new feature.)

---

## Cluster OQ5-CREDENTIAL-FIDELITY

**Primary invariant:** DC-LEDGER-10 (see registry for full text).

**TCB partition:**
- **BLUE** — `crates/ade_types` (`StakeCredential`); `crates/ade_codec`
  (shelley + conway cert decoders); `crates/ade_ledger`
  (delegation/state/gov_cert/cert_classify/rules/fingerprint).
- **GREEN** — `crates/ade_testkit` (snapshot_loader credential construction;
  corpus harness).
- **RED** — none.

**Cluster Exit Criteria (CI-verifiable):**
- **CE-1 (decode fidelity):** both era decoders return `KeyHash`/`ScriptHash`
  (tag preserved); an unknown credential tag is a deterministic reject. Tests:
  `shelley_credential_preserves_discriminant`,
  `conway_credential_preserves_discriminant`, `unknown_credential_tag_rejects`.
- **CE-2 (illegal-states-unrepresentable):** `StakeCredential` is a closed
  2-variant enum; no `Hash28→StakeCredential` coercion on the BLUE path.
  Compile + `ci_check_credential_discriminant_closed.sh`.
- **CE-3 (state keys discriminated):** CertState + ConwayGovState key on
  `StakeCredential`; a key-hash and a script-hash credential sharing 28 bytes are
  distinct keys. Tests: `keyhash_scripthash_same_bytes_are_distinct_certstate`,
  `..._govstate`.
- **CE-4 (fingerprint fidelity):** two states differing only in a credential
  discriminant fingerprint differently; affected goldens regenerated. Test:
  `discriminant_changes_fingerprint`.
- **CE-5 (replay + dual migration):** accumulation replays byte-identical over
  the discriminated fingerprint; cert-state + gov-state migration documented.
  Test: `credential_accumulation_replays_byte_identical`.
- **CE-6 (adversarial / no-false-accept):** unknown-tag reject + collision
  distinctness corpus — no silent overwrite.
- **CE-7 (CI + enforce):** `ci/ci_check_credential_discriminant_closed.sh` green;
  DC-LEDGER-10 flipped declared→enforced; `ci_check_constitution_coverage.sh`
  PASS (bidirectional cross_refs).

**Slices:**
- **S1 — discriminated `StakeCredential` end-to-end + dual fingerprint migration**
  — invariant: I-1/I-2/I-3/I-4 — the *atomic* type change (struct→enum) + both
  decoders preserve the tag + all ~13 consumer sites updated (CertState keys and
  ConwayGovState maps re-typed to `StakeCredential`) + fingerprint writers emit
  the discriminant + GREEN loader preserves the tag + regenerate affected goldens
  (Conway-with-deposits, cert_state, any present boundary pins). Compiles; all
  existing tests green. — addresses CE-1, CE-2, CE-3, CE-4 — TCB: BLUE (+GREEN
  loader).
- **S2 — fidelity corpus + CI gate + enforce DC-LEDGER-10** — invariant: replay +
  adversarial + enforcement — distinctness corpus (key vs script same-bytes →
  distinct CertState & ConwayGovState entries; no collision), decode round-trip
  tag preservation, unknown-tag reject, replay byte-identical;
  `ci/ci_check_credential_discriminant_closed.sh`; flip DC-LEDGER-10 enforced
  (+ code_locus/tests/ci_script); documented env-blocked oracle. — addresses
  CE-5, CE-6, CE-7 — TCB: GREEN (corpus) + BLUE (CI/registry).

> **Note:** S1 is *wide but atomic* — a newtype→enum change cannot be partial
> (every consumer must update to compile), so it is the smallest mergeable unit
> that leaves the build green. S2 is the proof/enforcement layer.

**Replay obligations:** `StakeCredential` changes shape (canonical-type change)
→ **dual fingerprint migration** (cert-state + gov-state, incl. BTreeMap
key-ordering), deliberate T-DET-01; regenerate goldens + any present boundary
pins. New distinctness corpus entries (S2). Replay cmd: `cargo test -p
ade_testkit` + `-p ade_ledger`.

**FC/IS partition:** dependencies inward only — GREEN loader/corpus depend on
BLUE types; BLUE never depends on testkit. No RED.

## Declared non-goals
- Withdrawal / required-signer / address credential discriminant fidelity (a
  possible future extension; not OQ-5's claimed scope).
- Byron credential surface (different scheme; out of scope unless proven
  affected).
