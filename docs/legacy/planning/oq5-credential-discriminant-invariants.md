# OQ-5 — Credential Key/Script Discriminant Fidelity
## Invariant Sketch (IDD Part I)

> Status: invariants phase complete. Next: `/cluster-plan`.
> Registry anchor: **DC-LEDGER-10** (`status="declared"`, `cluster="OQ5-CREDENTIAL-FIDELITY"`),
> appended to `docs/ade-invariant-registry.toml`; strengthens T-DET-01 / T-ENC-03,
> relates DC-LEDGER-08 / DC-LEDGER-09 / DC-TXV-05 (bidirectional back-refs added).
> Origin: B5 per-cluster security review (WARN/MEDIUM) + DC-LEDGER-09 open_obligation.

## Concept framing

Cardano credentials are `KeyHash | ScriptHash` (a tagged `Credential`). Ade
collapses both into a bare `Hash28`: both era decoders read the 0/1 type tag and
**discard** it (`shelley/cert.rs:143`, `conway/cert.rs:236`),
`StakeCredential(pub Hash28)` carries no discriminant, `CertState` and
`ConwayGovState` key their maps on the tag-erased credential, and
`write_stake_credential` serializes only the hash. The concept: **make credential
identity faithful end-to-end** so a key-hash and a script-hash credential are
never the same authoritative-state key, matching cardano-node.

This **is** a pure transformation (`decode preserves tag → state keys on
discriminated credential → fingerprint includes discriminant`). The concept is
understood. It is a *type-fidelity change rippling through existing transitions*,
not a new feature — and it is a deliberate fingerprint migration on two surfaces.

### Default scope: Shelley+ only
Byron uses a different credential scheme and a non-`ShelleyBlock` path. **Default
scope is Shelley+**; Byron is dragged in only if `/cluster-plan` proves Byron has
an affected credential surface (see OQ-F). The default must not accidentally
migrate Byron.

### Grounding (verified at HEAD)
- Collapse points: `decode_stake_credential` in **both** `ade_codec` decoders
  drop the tag (`shelley/cert.rs:143` `_tag`, `conway/cert.rs:236` `_cred_type`);
  `DRep` (conway/cert.rs:258-259) already keeps `KeyHash`/`ScriptHash` — an
  existing asymmetry.
- `StakeCredential(pub Hash28)` (`shelley/cert.rs:46`); `BTreeMap<StakeCredential,
  …>` keys in `CertState` (`delegation.rs:48-52`) + shelley rewards.
- `ConwayGovState` keys `vote_delegations`/`committee_hot_keys`/`drep_expiry` on
  bare `Hash28` (`state.rs:87-102`).
- `write_stake_credential` (`fingerprint.rs:394`) = `write_hash28(cred.0)` — no
  discriminant; gov maps written as bare `Hash28` too.
- `address::Credential` exists but is `Vec<u8>`-shaped — a reference pattern, not
  a drop-in for a `Hash28` credential.

---

## 1. What must always be true
- **I-1 (decode fidelity).** Both era credential decoders preserve the key/script
  discriminant: a decoded credential is `KeyHash(h)` or `ScriptHash(h)`, never a
  tag-erased hash. An unknown credential tag is a deterministic decode reject.
  *(Strengthens T-ENC-03 round-trip; aligns the credential decode with the
  already-discriminated `DRep`.)*
- **I-2 (illegal states unrepresentable).** The credential type is a closed sum
  over `{KeyHash, ScriptHash}` (over `Hash28`); there is **no**
  bare-`Hash28`→credential constructor on the authority path. *(IDD §2.)*
- **I-3 (authoritative state keys on the discriminated credential).** `CertState`
  (registrations/delegations/rewards) and `ConwayGovState`
  (vote_delegations/committee_hot_keys/drep_expiry) key on the discriminated
  credential, so a key-hash and a script-hash credential sharing 28 bytes are
  **distinct** keys — matching cardano-node's `Credential`-keyed `UMap`/`VState`.
- **I-4 (fingerprint fidelity).** The canonical fingerprint serializes the
  discriminant; two states differing only in a credential's key/script tag
  fingerprint differently. *(Strengthens T-DET-01; relates DC-LEDGER-08/09 —
  cert/gov accumulation now keyed on faithful credentials.)*

## 2. What must never be possible
- A credential decode that erases the key/script tag (the `_tag`/`_cred_type`
  discard must be gone, structurally).
- A key-cred and a script-cred with identical 28 bytes silently collapsing into
  one `CertState`/`ConwayGovState` map entry (silent overwrite / wrong
  accumulation).
- A fingerprint byte-identical for two states differing only in a credential
  discriminant.
- A `Hash28 → credential` coercion anywhere on the BLUE authority path.

## 3. What must remain identical across executions
- The discriminated credential is `Ord` (BTreeMap key) with deterministic
  ordering over `(discriminant, hash)`. No `HashMap`. Decode is pure and
  deterministic.

## 4. What must be replay-equivalent
- Replaying the same block stream yields byte-identical `CertState` +
  `ConwayGovState` fingerprints.
- **Deliberate dual fingerprint migration (T-DET-01):** the discriminated
  representation changes both the cert-state and gov-state fingerprints (added
  discriminant byte **and** changed BTreeMap key ordering) vs the collapsed
  encoding. Intentional, confirmed-correct-not-merely-stable — the same migration
  class as B4/B5, on two surfaces at once.

## 5. State transitions in scope
- `decode_stake_credential` (both eras): `(cbor) → Ok(KeyHash(h) | ScriptHash(h))
  | Err(reject)`.
- Existing apply transitions re-keyed on the discriminated credential:
  `apply_cert`, `apply_conway_cert`, `apply_conway_gov_cert` (the `.0`
  projections in B5 change), `cert_classify`.
- `write_stake_credential` + the gov-map writers: emit `discriminant ‖ hash`.
- (Not new transitions — a type-fidelity change threaded through the above.)

## 6. TCB color hypothesis
- **BLUE** — `ade_types` (the credential type), `ade_codec` (both decoders),
  `ade_ledger` (`delegation`/`CertState`, `state`/`ConwayGovState`, `gov_cert`,
  `cert_classify`, `rules`, `fingerprint`).
- **GREEN** — `ade_testkit` (snapshot loader constructs credentials; corpus).
- **RED** — none. No unresolved colors.

## 7. Open questions (resolve before cluster-plan)
- **OQ-A (type design) — RECOMMENDATION RECORDED, confirm at `/cluster-plan`.**
  Make `StakeCredential` **itself** the closed enum, in place:
  ```rust
  pub enum StakeCredential {
      KeyHash(Hash28),
      ScriptHash(Hash28),
  }
  ```
  *Rationale (user-directed):* this is a **fidelity correction, not a new
  semantic feature**. The existing type name already represents the authority
  concept; only its *shape* is wrong — `StakeCredential(pub Hash28)` erases the
  discriminant the decoders already observe and then discard. Replacing
  `StakeCredential` in place makes the illegal state unrepresentable exactly
  where the bug lives.
  - **Do NOT** introduce a tuple key `(tag, Hash28)` in BLUE — mechanically
    workable but weaker: it preserves the discriminant as *data*, not as *domain
    meaning*.
  - **Do NOT** reuse `address::Credential` (`Vec<u8>`-shaped) as the authority
    type — reference pattern only.
- **OQ-B (gov-state key type).** `ConwayGovState` keys on bare `Hash28` today.
  Per OQ-A, migrate those keys to the discriminated `StakeCredential` (full
  fidelity). Confirm B5's gov maps migrate (not a `(tag, Hash28)` tuple).
- **OQ-C (DRep alignment).** `DRep` already discriminates. Decide whether `DRep`'s
  `KeyHash`/`ScriptHash` should share / wrap the discriminated `StakeCredential`,
  and keep the two consistent.
- **OQ-D (oracle validation).** The fidelity payoff is real-oracle agreement,
  currently env-blocked (recoverable from the ImmutableDB EBS snapshots). Closure
  gate, or env-blocked-reclassified like DC-LEDGER-08/09?
- **OQ-E (migration acceptance).** This migrates **both** cert-state and gov-state
  fingerprints + BTreeMap ordering, and regenerates the Conway-with-deposits
  golden and any boundary pins. Confirm acceptable and that it doesn't further
  destabilize the already-env-blocked boundary tests.
- **OQ-F (Byron/pre-Shelley) — STAYS EXPLICIT.** Default scope is **Shelley+
  only**. Byron is included only if `/cluster-plan` proves Byron has an affected
  credential surface; otherwise it is explicitly out of scope.

---

## Registry anchor (appended)

```toml
[[rules]]
id = "DC-LEDGER-10"
tier = "derived"
statement = "Credential identity is faithful end-to-end … (see registry for full text)"
cross_ref = ["T-DET-01", "T-ENC-03", "DC-LEDGER-08", "DC-LEDGER-09", "DC-TXV-05"]
status = "declared"
authority_surface = "BLUE credential representation fidelity"
cluster = "CL-LEDGER-VERDICT"   # CI-enforced architectural-domain slot
```

> **Note on the `cluster` field.** The registry `cluster` field is a CI-enforced
> closed vocabulary (`CL-LEDGER-VERDICT | CL-CRYPTO-VERIFY`) — an *architectural
> domain*, not the IDD planning-cluster name. `ci_check_constitution_coverage.sh`
> rejects any other value. The planning identity **OQ5-CREDENTIAL-FIDELITY** is
> carried by this doc + the registry comment header, and becomes
> `strengthened_in`/`introduced_in` once `/cluster-plan` runs. (Attempted
> `cluster = "OQ5-CREDENTIAL-FIDELITY"` first; CI rejected it.)

Back-refs to `DC-LEDGER-10` added into T-DET-01 / T-ENC-03 / DC-LEDGER-08 /
DC-LEDGER-09 / DC-TXV-05 (bidirectional-cross_ref CI requirement).
