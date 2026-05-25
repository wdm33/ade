# Invariant Slice — PHASE4-N-C S1

## Slice Header
**Slice Name:** RED signing primitives (`vrf_prove`, `kes_sign`, `kes_update`) + cardano-cli `*.skey` loader + private-key custody CI gate
**Cluster:** PHASE4-N-C
**Status:** Proposed
**CEs addressed:** CE-N-C-1 (signing transcript equivalence + verify symmetry + evolution discipline + key-custody)
**Registry flips on merge:** `DC-CRYPTO-03`, `DC-CRYPTO-04`, `DC-CRYPTO-05`, `OP-OPS-04` → `enforced`
**Dependencies:** none (N-B / N-E validator stack is the only consumer of the artifacts this slice produces; consumption lands in S3/S5)

---

## Intent

Make private-key custody and signing operations live exclusively in a new
RED module `ade_runtime::producer::signing`, prove signing transcript
equivalence with cardano-node reference vectors, prove sign/verify symmetry
against the existing `ade_crypto` verify path, prove KES evolution
discipline (one-way, period bounds), and mechanically forbid any
KES/VRF/cold private-key type from appearing in BLUE crates
(`ade_core`, `ade_codec`, `ade_types`, `ade_ledger`, `ade_crypto`).

The signed artifacts (`VrfProof`, `KesSignature`, `OpCert` bytes) become
the only producer outputs that cross the RED→BLUE boundary. Replay
corpora capture artifacts; private-key bytes never leave RED.

---

## The change (atomic; compile green as one unit)

### 1. New module `crates/ade_runtime/src/producer/signing.rs` (RED)

Wraps `cardano-crypto`'s `VrfDraft03` + `Sum6Kes` algorithms with
ergonomics suitable for producer use. Forbids private-key bytes from
crossing the module boundary.

```rust
// VRF signing — RED-confined private-key custody
pub struct VrfSigningKey(/* private */ [u8; 64]);  // libsodium expanded-secret form
impl VrfSigningKey {
    pub fn from_bytes_zeroizing(b: &[u8]) -> Result<Self, SigningError>;
}
impl Drop for VrfSigningKey { fn drop(&mut self) { /* zeroize */ } }
impl core::fmt::Debug for VrfSigningKey { /* redact */ }

pub fn vrf_prove(
    sk: &VrfSigningKey,
    alpha: &[u8],
) -> Result<(VrfProof, VrfOutput), SigningError>;

// KES signing — RED-confined private-key custody
pub struct KesSecret {
    inner: cardano_crypto::kes::Sum6Kes,  // private
    current_period: KesPeriod,
    evolutions_remaining: u32,
}
impl KesSecret {
    pub fn from_bytes_zeroizing(b: &[u8]) -> Result<Self, SigningError>;
    pub fn current_period(&self) -> KesPeriod;
    pub fn evolutions_remaining(&self) -> u32;
}
impl Drop for KesSecret { fn drop(&mut self) { /* zeroize */ } }
impl core::fmt::Debug for KesSecret { /* redact, never print bytes */ }

pub fn kes_sign(
    sk: &KesSecret,
    period: KesPeriod,
    msg: &[u8],
) -> Result<KesSignature, SigningError>;
//   Forbidden: period < sk.current_period   -> SigningError::PeriodBackwards
//   Forbidden: period > sk.current_period + sk.evolutions_remaining
//              -> SigningError::PeriodExhausted

pub fn kes_update(
    sk: KesSecret,
    to: KesPeriod,
) -> Result<KesSecret, SigningError>;
//   Forbidden: to < sk.current_period                       -> EvolutionBackwards
//   Forbidden: to > sk.current_period + sk.evolutions_remaining -> EvolutionExhausted

#[derive(Debug)]  // SigningError is BLUE-safe (no key bytes in payload)
pub enum SigningError {
    PeriodBackwards { requested: KesPeriod, current: KesPeriod },
    PeriodExhausted { requested: KesPeriod, max: KesPeriod },
    EvolutionBackwards { from: KesPeriod, to: KesPeriod },
    EvolutionExhausted { from: KesPeriod, to: KesPeriod, evolutions_remaining: u32 },
    MalformedKey { algorithm: &'static str, detail: &'static str },
    CardanoCrypto(cardano_crypto::common::Error),
}
```

Outputs reuse existing BLUE types:
- `VrfProof`, `VrfOutput` from `ade_crypto::vrf` (existing closed types).
- `KesSignature` will be added to `ade_crypto::kes` as a closed type
  (currently the crate uses raw byte slices for the signature input to
  `verify_kes`; S1 adds the closed `KesSignature(pub [u8; SUM6_KES_SIG_LEN])`
  alongside the existing `verify_kes` entry, byte-equal wrapper).
- `KesPeriod` from `ade_crypto::kes` (existing).

### 2. New module `crates/ade_runtime/src/producer/keys.rs` (RED)

Reads cardano-cli `*.skey` files (JSON text-envelope with `cborHex`)
and constructs RED in-memory secrets. The text-envelope `type` field
must match the expected algorithm string from
`cardano_crypto::key::text_envelope`.

```rust
pub fn load_vrf_signing_key_skey(path: &Path) -> Result<VrfSigningKey, KeyLoadError>;
pub fn load_kes_signing_key_skey(path: &Path) -> Result<KesSecret, KeyLoadError>;
pub fn load_cold_signing_key_skey(path: &Path) -> Result<ColdSigningKey, KeyLoadError>;

#[derive(Debug)]  // no key bytes ever in the error payload
pub enum KeyLoadError {
    Io(std::io::ErrorKind),  // ErrorKind only, never the path
    MalformedEnvelope { detail: &'static str },
    UnexpectedType { expected: &'static str, found: String },
    CborHexDecode { detail: &'static str },
    Crypto(SigningError),
}
```

Expected envelope types (from `cardano_crypto::key::text_envelope`):
- VRF skey: `VrfSigningKey_PraosVRF`
- KES skey: `KesSigningKey_ed25519_kes_2^6`
- Cold skey: `StakePoolSigningKey_ed25519`

The loader rejects any other envelope `type` string with
`KeyLoadError::UnexpectedType`.

### 3. New module `crates/ade_runtime/src/producer/mod.rs` (RED)

```rust
pub mod keys;
pub mod signing;
```

Plus the `pub mod producer;` line in `crates/ade_runtime/src/lib.rs`.

### 4. New closed type `KesSignature` in `crates/ade_crypto/src/kes.rs`

```rust
pub const SUM6_KES_SIG_LEN: usize = 448;  // verify via cardano-crypto trait

#[derive(Clone, PartialEq, Eq)]
pub struct KesSignature(pub [u8; SUM6_KES_SIG_LEN]);

impl KesSignature {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CryptoError>;
}

// existing verify_kes gains a thin overload that takes &KesSignature:
pub fn verify_kes_signature(
    vk: &KesVerificationKey,
    period: KesPeriod,
    msg: &[u8],
    sig: &KesSignature,
) -> Result<(), CryptoError> {
    verify_kes(vk, period, msg, &sig.0)  // delegates to existing function
}
```

`KesSignature` is BLUE: contains the signature *bytes only* — never any
private-key material. RED `kes_sign` returns this type so BLUE forge (S3)
can consume it.

### 5. New module `crates/ade_testkit/src/producer/reference_vectors.rs` (GREEN test harness)

Reference vectors derived from `cardano_crypto::kes::test_vectors`
and `cardano_crypto::vrf::test_vectors`. Provides the *expected* signed
bytes for a fixed `(seed, period, message)` set. Used by S1's tests; not
a production path.

```rust
pub struct VrfReferenceVector {
    pub seed: [u8; 32],
    pub alpha: Vec<u8>,
    pub expected_proof: [u8; 80],
    pub expected_output: [u8; 64],
}

pub struct KesReferenceVector {
    pub seed: [u8; 32],
    pub period: u32,
    pub message: Vec<u8>,
    pub expected_signature: [u8; 448],
}

pub fn vrf_reference_set() -> Vec<VrfReferenceVector>;
pub fn kes_reference_set() -> Vec<KesReferenceVector>;
pub fn kes_update_reference_chain() -> Vec<([u8; 32], u32, [u8; 32])>;
  // (seed, period_after_n_updates, expected_signing_key_fingerprint)
```

### 6. Unit tests (co-located in `signing.rs` + `keys.rs` + `ade_testkit`)

In `crates/ade_runtime/src/producer/signing.rs` `#[cfg(test)] mod tests`:

- `vrf_prove_matches_reference_vectors` — for each vector in
  `vrf_reference_set()`, `vrf_prove(sk, alpha)` produces a `VrfProof`
  with bytes equal to `expected_proof` and a `VrfOutput` with bytes
  equal to `expected_output`.
- `kes_sign_matches_reference_vectors` — for each vector in
  `kes_reference_set()`, `kes_sign(sk, period, message)` produces a
  `KesSignature` with bytes equal to `expected_signature`.
- `kes_update_chain_matches_reference` — for each entry in
  `kes_update_reference_chain()`, repeatedly calling `kes_update` walks
  the key to the expected period and the post-evolution key fingerprint
  matches the expected fingerprint.
- `vrf_prove_then_verify_round_trip` — for each VRF reference vector,
  `verify_vrf(vk, &vrf_prove(sk, alpha)?.0, alpha)` returns
  `Ok(VrfOutput(expected_output))`.
- `kes_sign_then_verify_round_trip` — for each KES reference vector,
  `verify_kes_signature(vk, period, msg, &kes_sign(sk, period, msg)?)`
  returns `Ok(())`.
- `kes_sign_rejects_period_past_evolutions_remaining` — `kes_sign` called
  with `period > current + evolutions_remaining` returns
  `SigningError::PeriodExhausted` and does not produce a signature.
- `kes_update_rejects_backwards_evolution` — `kes_update(sk, to)` with
  `to < sk.current_period` returns `SigningError::EvolutionBackwards`.
- `kes_secret_debug_is_redacted` — `format!("{:?}", kes_secret)` does
  not contain any byte of the secret material; output is a constant
  redaction string.
- `vrf_signing_key_debug_is_redacted` — same for `VrfSigningKey`.
- `signing_error_contains_no_key_bytes` — every variant of
  `SigningError` formatted with `{:?}` produces output that is byte-disjoint
  from any seed bytes used in the failing call.

In `crates/ade_runtime/src/producer/keys.rs` `#[cfg(test)] mod tests`:

- `cardano_cli_skey_envelope_round_trips_through_keys_loader` — for each
  of (VRF, KES, cold) a small fixture file under
  `crates/ade_runtime/tests/fixtures/producer_keys/` loads through the
  loader; the resulting secret then signs a fixed message and the
  resulting signature verifies under the corresponding verification key
  derived from the envelope's companion `*.vkey` fixture.
- `keys_loader_rejects_wrong_envelope_type` — a VRF loader pointed at a
  KES skey returns `KeyLoadError::UnexpectedType { expected, found }`
  with the algorithm-name strings.
- `keys_loader_rejects_malformed_cbor_hex` — fixture with garbled
  `cborHex` returns `KeyLoadError::CborHexDecode`.
- `key_load_error_io_carries_no_path_bytes` — the `Io` variant only
  carries `std::io::ErrorKind`, never the path string.

In `crates/ade_crypto/src/kes.rs` `#[cfg(test)] mod tests` (BLUE):

- `kes_signature_from_bytes_round_trips` — `KesSignature::from_bytes`
  accepts exactly 448 bytes, rejects other lengths with
  `CryptoError::MalformedSignature`.
- `verify_kes_signature_agrees_with_existing_verify_kes` — for the same
  inputs, both call paths return identical verdicts (extractive proof
  the new BLUE wrapper does not change the verify path).

### 7. New CI gate `ci/ci_check_private_key_custody.sh` (closure proof)

Mechanical guards:

1. **No private-key types defined outside RED.** Grep `crates/ade_core/src/`,
   `crates/ade_codec/src/`, `crates/ade_types/src/`, `crates/ade_ledger/src/`,
   `crates/ade_crypto/src/` for:
   - `pub struct .*SigningKey` (only the *verification* keys and *signed
     artifacts* like `KesSignature` are allowed BLUE; `SigningKey` is
     RED-only)
   - `KesSecret`
   - `struct ColdSigningKey`
   Any match outside `crates/ade_runtime/src/producer/` is a failure.
2. **No `cardano_crypto::vrf::VrfDraft03::prove` or
   `cardano_crypto::kes::KesAlgorithm::sign_kes` call outside
   `crates/ade_runtime/src/producer/`.** Grep for `prove(` and
   `sign_kes(` in BLUE crates; flag matches.
3. **No `kes_sign` / `vrf_prove` / `kes_update` re-exports from BLUE
   crates.** Grep `pub use .*signing::{?.*(kes_sign|vrf_prove|kes_update)`
   in `crates/ade_core/`, `crates/ade_codec/`, `crates/ade_types/`,
   `crates/ade_ledger/`, `crates/ade_crypto/`.
4. **`ade_runtime::producer::signing` has no public function returning
   raw key bytes.** Grep for `pub fn .*-> .*\[u8;` /
   `pub fn .*-> Vec<u8>` in `signing.rs`; allowed return types are
   `VrfProof`, `VrfOutput`, `KesSignature`, `SigningError`,
   `Result<(VrfProof, VrfOutput), SigningError>`,
   `Result<KesSignature, SigningError>`,
   `Result<KesSecret, SigningError>`. Anything else fails the gate.
5. **`Debug` impls for `VrfSigningKey` / `KesSecret` / `ColdSigningKey`
   exist and are explicit (custom).** Grep for
   `#[derive(.*Debug.*)]` on those types — finding a derived `Debug` is
   a failure (must be hand-rolled redaction).
6. **No `prove(` / `sign_kes(` / `update_kes(` inside `*.rs` files
   under `crates/ade_testkit/src/producer/`** except in
   `reference_vectors.rs` (which calls them at test-vector materialization
   time — a one-shot, not a production signing path). The gate whitelists
   `reference_vectors.rs` explicitly.

### 8. Registry updates (same commit)

Flip these to `enforced` and populate `tests` + `ci_script`:

- `DC-CRYPTO-03` — `tests = ["vrf_prove_matches_reference_vectors",
  "vrf_prove_then_verify_round_trip"]`,
  `ci_script = "ci/ci_check_private_key_custody.sh"`,
  `code_locus = "crates/ade_runtime/src/producer/signing.rs (vrf_prove)"`,
  `status = "enforced"`.
- `DC-CRYPTO-04` — `tests = ["kes_sign_matches_reference_vectors",
  "kes_sign_then_verify_round_trip", "kes_signature_from_bytes_round_trips",
  "verify_kes_signature_agrees_with_existing_verify_kes"]`,
  `ci_script = "ci/ci_check_private_key_custody.sh"`,
  `code_locus = "crates/ade_runtime/src/producer/signing.rs (kes_sign); crates/ade_crypto/src/kes.rs (KesSignature, verify_kes_signature)"`,
  `status = "enforced"`.
- `DC-CRYPTO-05` — `tests = ["kes_update_chain_matches_reference",
  "kes_sign_rejects_period_past_evolutions_remaining",
  "kes_update_rejects_backwards_evolution"]`,
  `ci_script = "ci/ci_check_private_key_custody.sh"`,
  `code_locus = "crates/ade_runtime/src/producer/signing.rs (kes_update, kes_sign)"`,
  `status = "enforced"`.
- `OP-OPS-04` — `code_locus =
  "crates/ade_runtime/src/producer/keys.rs (load_vrf_signing_key_skey, load_kes_signing_key_skey, load_cold_signing_key_skey)"`,
  `tests = ["cardano_cli_skey_envelope_round_trips_through_keys_loader",
  "keys_loader_rejects_wrong_envelope_type",
  "keys_loader_rejects_malformed_cbor_hex"]`,
  `ci_script = "ci/ci_check_private_key_custody.sh"`,
  `status = "enforced"`.

`T-DET-01.strengthened_in` and `T-ENC-01.strengthened_in` are NOT yet
updated in this slice — those land in the cluster-close after S3/S4
flip the forge-byte-equality + body-hash-parity invariants.

---

## Mechanical Acceptance Criteria

- **AC-1** — `cargo build --workspace` green.
- **AC-2** — `cargo test -p ade_runtime producer::signing` green
  (10 unit tests pass).
- **AC-3** — `cargo test -p ade_runtime producer::keys` green
  (4 unit tests pass).
- **AC-4** — `cargo test -p ade_crypto kes::tests::kes_signature` green
  (2 new tests pass; existing KES verify tests do not regress).
- **AC-5** — `cargo test --workspace` green (no pre-existing tests
  regress; OQ-12 zero-regression rule).
- **AC-6** — `bash ci/ci_check_private_key_custody.sh` returns `PASS`
  (all 6 guards).
- **AC-7** — `bash ci/ci_check_constitution_coverage.sh` returns `PASS`
  (registry edits round-trip; DC-CRYPTO-03/04/05 + OP-OPS-04 status
  fields parse as `"enforced"`).
- **AC-8** — `grep -rE 'pub struct .*SigningKey' crates/ade_core/src crates/ade_codec/src crates/ade_types/src crates/ade_ledger/src crates/ade_crypto/src` returns no matches (sanity-mirrors guard 1 of the CI gate).

---

## Hard Prohibitions

Cluster-level prohibitions are inherited (cluster.md §Forbidden during
this cluster). Slice-specific additions:

- No `pub struct .*SigningKey` / `KesSecret` / `ColdSigningKey` outside
  `crates/ade_runtime/src/producer/`.
- No derived `#[derive(Debug)]` on any private-key type — `Debug` must
  be hand-rolled and redact bytes.
- No `prove(` / `sign_kes(` / `update_kes(` from `cardano_crypto::*`
  in BLUE crates (`ade_core`, `ade_codec`, `ade_types`, `ade_ledger`,
  `ade_crypto`).
- No `pub fn` in `signing.rs` that returns raw `[u8; N]` or `Vec<u8>`
  for a signature / proof — must wrap in the closed `KesSignature` /
  `VrfProof` / `VrfOutput` types.
- No filesystem reads or env reads in `signing.rs` (RED key-load lives
  in `keys.rs`; `signing.rs` is pure-given-secrets).
- No `serde` / JSON re-encoding of the text envelope outside `keys.rs`.
- No path-string-bearing `KeyLoadError` variants (path strings leak
  filesystem layout into logs).
- No mutation of `cardano_crypto`'s `Sum6Kes` state through any
  function not named `kes_update` (preserves NC-KES-2 one-way
  evolution at the wrapper boundary).
- No re-introduction of `cardano-crypto` features beyond the existing
  `["vrf-draft03", "kes-sum", "dsign"]` set (avoids inadvertently
  pulling in a different VRF or KES algorithm).

---

## Explicit Non-Goals

- BLUE `forge_block` — that's S3 (CE-N-C-3).
- BLUE `opcert_validate` — that's S2 (CE-N-C-2).
- Body-hash parity / validator-shared encoder — that's S4 (CE-N-C-4).
- Self-acceptance gate — that's S5 (CE-N-C-5).
- Scheduler / tick-assembler / broadcast — that's S6 (CE-N-C-6).
- Cross-impl adapter + live-evidence binary — that's S7 (CE-N-C-7/8).
- KES / VRF / cold key *generation* — out of scope per OQ-1 / OP-OPS-04.
  Operator supplies cardano-cli-format skeys.
- Hot-reload of keys without restart — out of scope per OQ-11 (restart
  is the sanctioned update path).
- Persistent opcert counter store — that's part of S2's RED-side
  surface for `prev_counter`, defined when S2 lands. S1 only ships the
  signing primitives.
- TPraos or Shelley→Mary producer scaffolding — non-goal per OQ-4.

---

## Failure Modes

All `SigningError` and `KeyLoadError` variants are deterministic and
contain no private-key bytes. The slice introduces no consensus-affecting
failure modes (no BLUE consumer until S3); all errors fail-fast at the
RED→GREEN boundary that S6 will introduce.

---

## Notes on `cardano-crypto` API surface

Confirmed at registry path
`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cardano-crypto-1.0.8/src/`:

- `vrf::VrfAlgorithm::prove(sk, message) -> Result<Proof>` (line 122,
  `src/vrf/mod.rs`).
- `kes::KesAlgorithm::sign_kes(context, period, message, signing_key) ->
  Result<Signature>` (line 161, `src/kes/mod.rs`).
- `kes::KesAlgorithm::update_kes(context, signing_key) -> Result<Option<SigningKey>>`
  (line 178, `src/kes/mod.rs`).
- `kes::test_vectors` and `vrf::test_vectors` modules provide canonical
  reference data that S1's `reference_vectors.rs` imports verbatim.
- `key::text_envelope` provides type-string constants
  (`VRF_SIGNING_KEY_TYPE`, `KES_SIGNING_KEY_TYPE`,
  `POOL_SIGNING_KEY_TYPE`) used by `keys.rs` to match the envelope's
  `type` field.

There is no JSON parser in `cardano-crypto::key::text_envelope` — S1
owns a minimal `serde_json`-free parser (`{ "type":..., "description":...,
"cborHex":... }`) that uses only `serde_json` already in the
`ade_runtime` dep tree if present, otherwise a hand-rolled parser. The
parser is RED, not BLUE.
