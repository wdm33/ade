# PHASE4-N-P — Cluster slice plan

> **Status:** Planning artifact. Output of `/cluster-plan` from the
> confirmed invariants sketch at
> [`phase4-n-p-invariants.md`](phase4-n-p-invariants.md).
>
> **Predecessor:** PHASE4-N-O (HEAD `6eb4fbd`).
> **User decisions:** OQ1(a) BLUE migration, OQ4(a) hex-literal
> corpus, OQ5 yes (proof obligation before code), N9 no-shim
> prohibition.

---

## Cluster shape: **5 slices**

Each slice strengthens at least one specific invariant from the
sketch and is independently mergeable. The order is dictated by
dependency: S1 (proof) gates S3 (serde) and S4 (corpus); S2
(algorithm) gates S3, S4, S5. S5 closes the loader and the
registry obligations.

### S1 — `period_from_zeroed_sum6_tree_shape` proof obligation (docs-only)

**Strengthens:** I5 (uniquely inferable period), N3 (no
best-effort guesswork), N4 (no period overflow).

**Slice deliverable:** `docs/clusters/PHASE4-N-P/period-from-zeroed-sum6-tree-shape-proof.md` —
- The recursive Sum_n tree layout (Sum6→Sum5→…→Sum0) with byte
  offsets for the 608-byte payload at every depth.
- Pseudocode for
  `period_from_zeroed_sum6_tree_shape(&[u8; 608]) -> Result<u32, KesParseError>`
  including the per-level seed-zero check and the bit-accumulator
  that emits `p ∈ 0..=63`.
- Proof sketch that the function agrees with the Haskell
  `update_kes` recurrence for every `p ∈ 0..=63` (induction on
  tree depth).
- Closed `KesParseError` variants the deserializer will return for
  every malformed-tree shape.
- The 64-period exhaustive fixture-test plan that S3 must
  implement (one round-trip assertion per period).

**No code.** Purely a slice-entry obligation document per
[[feedback-proof-discipline]]. S3 cannot start until this doc is
committed.

**Mechanical acceptance criteria:**
- The doc exists at the specified path.
- The doc declares the exact `KesParseError` enum variants.
- The doc states the 64-period round-trip test plan with the
  exact assertion shape.

---

### S2 — Ade-owned BLUE algorithm (`ade_crypto::kes_sum`)

**Strengthens:** I1 (Haskell-equivalent algorithm), I2 (cross-impl
equivalence), I4 (`update_kes` chain), I6 (forward secrecy
carried), D1–D4 (deterministic surface), N1 (no cross-impl
divergence), N7 (Drop zeroizing), N9 (no upstream-shim).

**Slice deliverable:** new module `crates/ade_crypto/src/kes_sum/`
with submodules `single.rs` (Sum0 = `SingleKES Ed25519DSIGN`),
`sum.rs` (generic `SumKes<D, H>`), `sum6.rs` (the `Sum6Kes` type
alias chain). Public surface:

```rust
pub trait KesAlgorithm {
    type SigningKey;
    type VerificationKey;
    type Signature;
    const SEED_SIZE: usize;
    const SIGNING_KEY_SIZE: usize;
    const VERIFICATION_KEY_SIZE: usize;
    const SIGNATURE_SIZE: usize;
    fn total_periods() -> u32;
    fn gen_key_kes_from_seed_bytes(seed: &[u8; 32]) -> Result<Self::SigningKey, KesError>;
    fn derive_verification_key(sk: &Self::SigningKey) -> Self::VerificationKey;
    fn update_kes(sk: Self::SigningKey, current_period: u32)
        -> Result<Option<Self::SigningKey>, KesError>;
    fn sign_kes(sk: &Self::SigningKey, period: u32, msg: &[u8])
        -> Result<Self::Signature, KesError>;
    fn verify_kes(vk: &Self::VerificationKey, period: u32, msg: &[u8], sig: &Self::Signature)
        -> Result<(), KesError>;
}

pub type Sum0Kes = SingleKes<Ed25519>;
pub type Sum1Kes = SumKes<Sum0Kes, Blake2b256>;
// ... up to Sum6Kes
pub type Sum6Kes = SumKes<Sum5Kes, Blake2b256>;
```

Plus closed `KesError` enum. `KesParseError` deferred to S3.

**S2 stops before serde.** No `raw_serialize_signing_key_kes` /
`raw_deserialize_signing_key_kes` yet. The serde is S3.

**Mechanical acceptance criteria:**
- 64 unit tests over the period chain: for each `p ∈ 0..=63`,
  generate from a fixed seed, `update_kes` to period `p`, sign a
  fixed message, `verify_kes` returns Ok.
- 64 cross-impl unit tests: for each `p`, ade-derived VK ==
  cardano-crypto-derived VK byte-for-byte.
- Cross-impl signing equivalence (`#[cfg(test)]` only): for a
  small message corpus × periods, ade-signed sig verifies under
  cardano-crypto, and vice versa.
- `Drop` for the new signing-key types best-effort zeroizes the
  inner seed buffers (carries existing
  `ci_check_private_key_custody.sh` discipline).
- ade_crypto's production code adds **no `cardano_crypto` import**
  (it's `dev-dependencies` only). CI gate added in S5.

---

### S3 — Expanded-skey serde + 448-byte signature serde

**Strengthens:** I3 (round-trip identity), I5 (period uniquely
inferable), D5 (deterministic codec), N2 (wrong size → fail-closed),
N3 (malformed sub-tree → closed error), N4 (period overflow →
fail-closed), N5 (re-serialize identity), N6 (no leakage in
errors).

**Slice deliverable:** add to `ade_crypto::kes_sum`:

```rust
fn raw_serialize_signing_key_kes(sk: &<Sum6Kes as KesAlgorithm>::SigningKey)
    -> [u8; 608];
fn raw_deserialize_signing_key_kes(bytes: &[u8])
    -> Result<<Sum6Kes as KesAlgorithm>::SigningKey, KesParseError>;
fn raw_serialize_signature_kes(sig: &<Sum6Kes as KesAlgorithm>::Signature)
    -> [u8; 448];
fn raw_deserialize_signature_kes(bytes: &[u8])
    -> Result<<Sum6Kes as KesAlgorithm>::Signature, KesParseError>;
fn current_period_of_signing_key(sk: &<Sum6Kes as KesAlgorithm>::SigningKey) -> u32;
```

Plus the implementation of `period_from_zeroed_sum6_tree_shape`
matching S1's proof obligation. Closed `KesParseError` enum
introduced.

**Mechanical acceptance criteria:**
- 64-period round-trip: for each `p ∈ 0..=63`,
  `parse(serialize(skey_at_p)) == skey_at_p` AND
  `current_period_of_signing_key(parsed) == p`.
- 64-period re-serialize identity:
  `serialize(parse(serialize(skey_at_p))) == serialize(skey_at_p)`.
- Negative tests: 32, 100, 607, 609, 612, 1000 bytes →
  `KesParseError::WrongPayloadSize`.
- Negative tests for malformed sub-trees: truncated child,
  inconsistent vk0/vk1 hashes, period-overflow tree shape — each
  with a distinct closed-error variant.
- `current_period_of_signing_key` agrees with `update_kes` chain
  output for every `p`.

---

### S4 — Real cardano-cli corpus + cross-impl `cargo test` gate

**Strengthens:** I1 / I2 against ground-truth (cardano-cli is the
authoritative reference, not just `cardano-crypto` Rust),
[[feedback-real-interop-finds-codec-bugs]].

**Slice deliverable:**
- Hex-literal `&[u8; 608]` constants under
  `crates/ade_crypto/src/kes_sum/cardano_cli_corpus.rs` (or
  `#[cfg(test)]` mod), generated by running
  `cardano-cli node key-gen-KES` in the docker preprod peer for
  a small corpus of throwaway seeds. Each fixture prefixed with
  the mandatory throwaway-fixture comment.
- Companion hex-literal constants for the (seed, expected VK,
  expected Sum6KES sig at known periods) tuples — captured under
  the same docker run with `cardano-cli` and committed alongside.
- Cross-impl test:
  `ade_crypto::kes_sum::Sum6Kes::raw_deserialize_signing_key_kes(corpus)`
  succeeds; the deserialized skey's VK matches the captured VK
  byte-for-byte; signing a fixed message at the captured period
  yields the captured signature byte-for-byte (or, if the
  captured signature was produced at a different message, verifies
  under our impl + under cardano-crypto's).
- New CI gate `ci/ci_check_kes_sum_compatibility.sh` asserting:
  - Every fixture constant is preceded by the throwaway comment.
  - The cardano-cli corpus tests exist and are non-empty.
  - No `.skey` file (envelope-shaped) appears under
    `crates/ade_crypto/`.
  - `cardano_crypto` is not imported anywhere in
    `crates/ade_crypto/src/**` outside `#[cfg(test)]` blocks.

**Mechanical acceptance criteria:**
- At least 3 throwaway-seed fixtures committed.
- Each fixture round-trips through our serde.
- Each fixture's VK matches the captured cardano-cli VK.
- Cross-impl sign/verify on each fixture succeeds in both
  directions.
- The new CI gate passes; added to OP-OPS-04 and DC-CRYPTO-08/09
  registry `ci_script` fields.

---

### S5 — `KesSecret.inner` migration + loader acceptance + cluster close

**Strengthens:** I7 (loader determinism), N8 (no `Ok(_)` on
malformed), N9 (no upstream-shim in production), R3 (cardano-cli
envelope and Ade-native envelope are observationally equivalent
post-load).

**Slice deliverable:**
- `KesSecret.inner` field type migrates from
  `cardano_crypto::kes::Sum6Kes::SigningKey` to
  `ade_crypto::kes_sum::Sum6Kes::SigningKey`.
- All call sites in `ade_runtime::producer::signing` updated to
  use the new BLUE algorithm: `kes_sign`, `kes_update`,
  `from_bytes_zeroizing`, `from_seed_at_period`,
  `verification_key_fingerprint`. Behavior is preserved
  byte-for-byte (by S2's cross-impl proof).
- `crates/ade_runtime/Cargo.toml`: `cardano-crypto` drops the
  `kes-sum` feature flag (kept only for `vrf-draft03` + `dsign`).
- `ade_runtime::producer::keys::load_kes_signing_key_skey` flips
  from "fail-closed always" to "608-byte structurally-valid →
  Ok; everything else → fail-closed" per DC-CRYPTO-07's
  open_obligation.
- Registry:
  - `OP-OPS-04.open_obligation = null`;
    `strengthened_in += "PHASE4-N-P"`.
  - `DC-CRYPTO-07.open_obligation = null`;
    `strengthened_in += "PHASE4-N-P"`; statement updated to reflect
    the new accept-608-byte-valid surface.
  - `DC-CRYPTO-08/09.status = "enforced"`;
    `tests`/`ci_script` populated from S2/S3/S4 artifacts.
  - `DC-CRYPTO-03/04/05.strengthened_in += "PHASE4-N-P"` (the
    DC-CRYPTO family now has a BLUE-owned algorithm).
- CI gate `ci/ci_check_kes_envelope_closed.sh` updated: the
  cardano-cli loader-body assertion changes from
  "must return `UnsupportedExpandedKesKeyFormat`" to
  "must return `UnsupportedExpandedKesKeyFormat` for every payload
  size ≠ 608 AND `Ok(KesSecret)` only for structurally-valid
  608-byte payloads".
- `docs/active/op-ops-04-ade-native-kes-flow.md` updated: the
  "unsupported flow" section becomes "alternative supported flow".
  Bounty-facing claim boundary unchanged for the Ade-native path,
  but the cardano-cli path is no longer fail-closed.

**Mechanical acceptance criteria:**
- All existing PHASE4-N-O tests still pass (Ade-native envelope
  loader untouched).
- `load_kes_signing_key_skey` on a real cardano-cli 608-byte
  envelope returns `Ok(KesSecret)` whose `current_period` matches
  the embedded period.
- Negative tests for 32-, 612-, malformed 608-byte payloads stay
  green (with their new `KesParseError`-wrapped variants).
- `cargo test --workspace` clean.
- New CI gate `ci_check_kes_sum_compatibility.sh` passes; updated
  `ci_check_kes_envelope_closed.sh` passes;
  `ci_check_private_key_custody.sh` passes.
- Cluster close per `/cluster-close PHASE4-N-P`: grounding docs
  regenerate; per-cluster security review returns no BLOCK
  findings.

---

## Dependency graph

```
S1 (proof) ─────────────────────────────────────┐
                                                 │
S2 (algorithm) ──┬─────► S3 (serde) ──┬──► S4 ──┴──► S5 (close)
                 │                     │
                 └───────────────────────────► S5 (migration consumes algorithm)
```

S1 blocks S3 (the serde implements the proof). S2 blocks S3, S4,
S5. S3 blocks S4 (cross-impl needs deserialization). S4 blocks
S5 (loader integration needs verified positive corpus).

S1 + S2 can ship in parallel (S1 is docs-only, S2 is code-only).
Recommended order: **S1 → S2 → S3 → S4 → S5** to keep each PR
linear.

## Out-of-scope (explicit, restated from invariants doc)

- Mlocked secret memory (`sodium_mlock`) — operational concern,
  future cluster.
- `CompactSum6Kes` — mainnet on-chain headers do not use the
  compact variant.
- VRF or cold-key changes — N-P is KES-only.
- ChainDb persistence of partially-evolved KES keys — operator
  rotation policy concern; orthogonal to N-P scope.

## Hard prohibitions (cluster-wide, restated from invariants doc)

- N9: No compatibility shim via `unsafe`, `transmute`, vendored
  `pub(crate)` access, or fork-only constructors.
- N6: No `.skey` envelope files committed. Hex-literal corpus
  only with mandatory throwaway comments.
- N7: `Drop` for new BLUE signing-key types best-effort zeroizes.
- Test imports of `cardano_crypto` only under `#[cfg(test)]` and
  in `dev-dependencies`.

## Closure protocol

Per `/cluster-close PHASE4-N-P`:
1. All 5 slices merged with their MACs green.
2. Per-cluster security review (cross-slice diff) — block on HIGH+.
3. Registry updates from S5 applied.
4. Grounding-doc regenerators run (`/codemap`, `/seams`,
   `/head-deltas`, `/traceability`).
5. `head_deltas_baseline` bumped to the closing commit.
6. Closure record committed with trailer per project override.
