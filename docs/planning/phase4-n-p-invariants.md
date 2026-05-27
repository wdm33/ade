# PHASE4-N-P — Invariants sketch

> **Status:** Planning artifact (non-normative). Frames the
> Sum6KES-expanded-compatibility concept in IDD terms before any
> cluster-doc expansion or slice work.
>
> **Predecessor:** PHASE4-N-O (HEAD `6eb4fbd`). N-O fail-closes the
> cardano-cli expanded envelope; N-P opens the 608-byte well-formed
> case while keeping every malformed case fail-closed.
>
> **User decisions on file (2026-05-27 session):**
> - OQ1(a): wholesale migrate `KesSecret.inner` to the Ade-owned
>   `ade_crypto::kes_sum::Sum6Kes::SigningKey`. cardano-crypto is
>   demoted to a `#[cfg(test)]` oracle.
> - OQ4(a): commit real cardano-cli expanded outputs as hex-literal
>   `&[u8; 608]` constants with mandatory throwaway-fixture comments.
>   No `.skey` files in the test corpus.
> - OQ5: yes — write the `period_from_zeroed_sum6_tree_shape` proof
>   obligation **before** any deserializer code lands.
> - Registry entries DC-CRYPTO-08 + DC-CRYPTO-09 approved as renamed,
>   appended as `status = "declared"` until tests + CI attach.
> - New invariant **N9** (no upstream-shim compatibility hack)
>   accepted.

---

## 1. What must always be true

- **I1. Byte-identical to Haskell `cardano-base`.** For every triple
  `(seed, period, msg)` in 0..=63 × any message:
  - `derive_verification_key(seed)` returns the same 32 bytes as
    Haskell's `deriveVerKeyKES`.
  - `gen_key_kes_from_seed_bytes(seed)` followed by
    `raw_serialize_signing_key_kes` returns the same 608 bytes as
    Haskell's `rawSerialiseSignKeyKES (genKeyKES seed)`.
  - `sign_kes(skey_at_p, p, msg)` followed by
    `raw_serialize_signature_kes` returns the same 448 bytes as
    Haskell's `rawSerialiseSigKES (signKES p msg sk)`.
- **I2. Cross-impl equivalence with `cardano-crypto` 1.0.8.** For the
  same triple `(seed, period, msg)`:
  - Our VK == upstream's VK.
  - Our 448-byte signature, decoded via
    `cardano_crypto::kes::Sum6Kes::raw_deserialize_signature_kes`,
    verifies under upstream's `verify_kes`.
  - Upstream's signature verifies under our `verify_kes`.
- **I3. Expanded-skey round-trip is identity.**
  `parse(serialize(skey)) == skey` for skeys at every period 0..=63,
  including post-update states with consumed sub-seeds zeroed.
- **I4. `update_kes` chain matches the reference.** Starting from any
  seed `s`, the sequence `(s, k_0, k_1, ..., k_63)` produced by
  repeated `update_kes` yields skeys whose serializations match
  Haskell's at every step.
- **I5. `current_period` inferable from tree shape.** Given a 608-byte
  expanded payload at unknown period, the period is **uniquely**
  determined by which sub-seeds are zeroed. The deserializer computes
  it deterministically; round-trip preserves it. **No heuristic
  inference; no "best matching" tree. Exactly one valid period or a
  closed parse error.**
- **I6. Forward-secrecy carried.** DC-CRYPTO-04 + DC-CRYPTO-05 hold
  for the new BLUE algorithm: once at period `p`, the skey cannot
  sign for period `< p`; `update_kes` is monotonic; period exhaustion
  is a closed error variant.
- **I7. Loader determinism.** `load_kes_signing_key_skey(path)` is a
  pure function of the file bytes. Same bytes → byte-identical
  `KesSecret`.

## 2. What must never be possible

- **N1. Cross-impl divergence** on any `(seed, period, msg)` triple
  in scope. An ade-derived VK differing from upstream's VK by even
  one bit must halt the cross-impl test corpus.
- **N2. 612-byte payload accepted** as a `Sum6KES` skey. Any payload
  size ≠ 608 inside the cardano-cli envelope → `KesParseError::WrongPayloadSize`.
- **N3. Malformed sub-tree silently accepted.** Truncated child
  skey, wrong VK length at any recursion level, indefinite-length
  CBOR, structurally inconsistent (vk0/vk1 hashes that don't match
  recursively-derived sub-tree VKs) → closed error variant. **No
  best-effort guesswork.**
- **N4. `current_period` overflow.** Tree shape consistent with
  period > 63 → fail-closed. Sum6 capacity is exactly 64 periods.
- **N5. Period mismatch on serialize→deserialize→serialize→compare.**
  If a fresh `genKey` at period 0 is serialized, deserialized, then
  re-serialized, the bytes must be identical (no normalization
  drift).
- **N6. Private-key byte leakage.** Seed bytes, expanded sub-tree
  bytes, or any private material in: `Debug` output, error messages,
  JSONL logs, panic messages, structured errors, or test fixtures
  committed to the repo as `.skey` envelopes. **Test corpora carry
  hex-literal `&[u8; 608]` constants only**, never operator-grade
  `.skey` files; each fixture is preceded by the mandatory throwaway
  comment.
- **N7. Heap retention after `Drop`.** An `ade_crypto::kes_sum::Sum6Kes::SigningKey`
  going out of scope without best-effort zeroing of its sub-seed
  buffers.
- **N8. `Ok(_)` from `load_kes_signing_key_skey` on a payload that
  wasn't a structurally-valid 608-byte expanded `Sum6KES`.** The
  fail-closed surface narrows but never opens to malformed inputs.
- **N9. No upstream-shim compatibility hack.** No compatibility shim
  may construct an upstream `cardano_crypto::kes::sum::SumSigningKey`
  through unsafe layout assumptions, `transmute`, vendored
  `pub(crate)` field access, or fork-only constructors.
  `cardano-crypto` is a **test oracle only** after PHASE4-N-P. This
  prohibits future "temporary" shortcuts from reintroducing
  architecture drift.

## 3. What must remain identical across executions

- **D1.** `gen_key_kes_from_seed_bytes(seed)` — same seed → same 608
  bytes.
- **D2.** `derive_verification_key(skey)` — same skey → same 32-byte
  VK.
- **D3.** `update_kes(skey, current_period)` — same input → same
  output (including the deterministic-zeroing behavior of consumed
  sub-seeds).
- **D4.** `sign_kes(skey, period, msg)` — same inputs → same 448-byte
  signature.
- **D5.** `raw_serialize_signing_key_kes` / `raw_deserialize_signing_key_kes`
  — deterministic codecs.
- **D6.** All of the above match the Haskell `cardano-base` reference
  byte-for-byte. *Deterministic* in this cluster means not just
  stable across our own runs, but stable against the canonical
  reference.

## 4. What must be replay-equivalent

- **R1.** The cross-impl test corpus: replaying
  `cargo test -p ade_crypto kes_sum` produces byte-identical outputs.
- **R2. Signing-implementation choice is non-load-bearing for
  replay.** A WAL replay containing a block whose KES sig was
  produced by `ade_crypto::kes_sum::Sum6Kes::sign_kes` must
  hash-equal the same replay if the sig were produced by
  `cardano_crypto::kes::Sum6Kes::sign_kes`. This is the strongest
  cross-impl claim — it justifies the OQ1(a) migration: replacing
  the production signer with the BLUE-owned impl does not fork the
  chain.
- **R3.** The producer pipeline (`crates/ade_runtime/src/producer/`)
  treats a real cardano-cli `KesSigningKey_ed25519_kes_2^6` envelope
  identically to the Ade-native `ade.kes.seed.v1` envelope at the
  `KesSecret`-consumption boundary. After load, the two paths are
  observationally indistinguishable for `kes_sign` / `kes_update`
  over the same `(seed, period)`.

## 5. State transitions in scope

Inside `ade_crypto::kes_sum` (pure, BLUE):

| Function | Transition |
|---|---|
| `gen_key_kes_from_seed_bytes` | `Seed[u8;32] → Result<SignKey, KesError>` |
| `derive_verification_key` | `&SignKey → VerKey[u8;32]` |
| `update_kes` | `(SignKey, u32) → Result<Option<SignKey>, KesError>` |
| `sign_kes` | `(&SignKey, u32, &[u8]) → Result<Signature[u8;448], KesError>` |
| `verify_kes` | `(&VerKey, u32, &[u8], &Signature) → Result<(), KesError>` |
| `raw_serialize_signing_key_kes` | `&SignKey → [u8;608]` |
| `raw_deserialize_signing_key_kes` | `&[u8] → Result<SignKey, KesParseError>` |
| `current_period_of_signing_key` | `&SignKey → u32` |

At the RED loader boundary (`ade_runtime::producer::keys`):

```
load_kes_signing_key_skey(path) ::
  (filesystem bytes, path) →
    Result<KesSecret { inner, current_period, evolutions_remaining },
           KeyLoadError>
```

**N-P strengthens (not replaces):** the 608-byte structurally-valid
path transitions from `Err(UnsupportedExpandedKesKeyFormat)` (N-O)
to `Ok(KesSecret)` (N-P). Every other payload shape remains
fail-closed via the existing or new closed error variants.

The `KesSecret` produced is observationally equivalent to one built
via `from_seed_at_period` for the same `(seed, period)` — same
`current_period`, same sign/verify behavior.

## 6. TCB color hypothesis

| Component | Color | Rationale |
|---|---|---|
| `crates/ade_crypto/src/kes_sum/` | **BLUE** | Pure, deterministic, no I/O, no RNG, no `HashMap`. Authoritative for the byte layout. Consumed by RED at the type level only. |
| `crates/ade_crypto/src/kes.rs` | **BLUE** (existing) | Continues to host `KesSignature`, `KesVerificationKey`, `verify_kes_signature`. Re-exports new types from `kes_sum::Sum6Kes`. |
| `crates/ade_runtime/src/producer/signing.rs::KesSecret` | **RED** (existing) | Wraps the BLUE signing-key with zeroize / redacted-`Debug` / custody discipline. `KesSecret.inner` migrates from `cardano_crypto::kes::Sum6Kes::SigningKey` to `ade_crypto::kes_sum::Sum6Kes::SigningKey` per OQ1(a). |
| `crates/ade_runtime/src/producer/keys.rs::load_kes_signing_key_skey` | **RED** (existing) | File I/O + envelope parsing; delegates payload decoding to BLUE. |
| Cross-impl test harness | `#[cfg(test)]` in `ade_crypto` | Not a separate crate. Imports `cardano_crypto` only under `#[cfg(test)]`. Production code MUST NOT import `cardano_crypto` after N-P (carries N9). |
| Positive corpus (608-byte expanded payloads) | Test fixtures | Hex-literal `&[u8; 608]` constants with mandatory throwaway comments per OQ4(a). |

No GREEN module introduced by N-P.

## 7. Open questions — resolved

All seven open questions from the initial sketch have user-decided
answers (see preamble). Recorded here for traceability:

| OQ | Decision |
|---|---|
| OQ1 | **(a)** wholesale migrate `KesSecret.inner` to Ade-owned BLUE type |
| OQ2 | Out of scope — mlocked memory is a future operational cluster |
| OQ3 | Out of scope — `CompactSum6Kes` not used on mainnet headers |
| OQ4 | **(a)** hex-literal `&[u8; 608]` from real cardano-cli with mandatory throwaway-fixture comment |
| OQ5 | Yes — write `period_from_zeroed_sum6_tree_shape` proof obligation **before** any deserializer code |
| OQ6 | `ade_crypto::kes_sum` defines its own `KesAlgorithm` trait — completes BLUE ownership |
| OQ7 | Cross-period verify mismatch test included — sign at period N, attempt verify at period M ≠ N, expect `VerificationFailed` |

## Slice-entry proof obligation (per OQ5)

> Before implementing `raw_deserialize_signing_key_kes`, document
> ```
> period_from_zeroed_sum6_tree_shape(bytes: &[u8; 608]) -> Result<u32, KesParseError>
> ```
> and prove it agrees with `update_kes` serialization for all
> periods 0..=63. Add an exhaustive 64-period fixture test:
> for each `p` in 0..=63, generate a key from a fixed seed,
> `update_kes` to period `p`, serialize, then assert
> `period_from_zeroed_sum6_tree_shape(serialized) == Ok(p)`.
> 
> No heuristic inference. No "best matching" tree. Exactly one valid
> period or a closed parse error.

## Cannot-yet-be-expressed concerns

The concept **can** be expressed as a pure transformation
`(seed, period, msg) → (vk, signed_bytes, expanded_bytes)`.
Nondeterminism enters only at the seed-source boundary (already
handled by N-O's `/dev/urandom` + `--seed-file` test seam). No new
nondeterminism introduced by N-P.

---

## Approved slice shape

The cluster doc (next artifact: `/cluster-plan` then `/cluster-doc`)
should expand into a slice ordering equivalent to:

1. Document Sum6KES tree layout + `period_from_zeroed_sum6_tree_shape`
   (the proof obligation lands first, before any code).
2. Implement BLUE types + deterministic `KesError` / `KesParseError`
   surfaces.
3. Implement seed derivation, VK derivation, `update_kes`, `sign_kes`,
   `verify_kes`.
4. Implement raw 608-byte skey serde + 448-byte signature serde.
5. Add cross-impl tests against `cardano-crypto` 1.0.8 (under
   `#[cfg(test)]` only).
6. Add cardano-cli 608-byte expanded-key fixtures as hex literals
   (throwaway-comment-prefixed).
7. Migrate `KesSecret.inner` to the Ade-owned signing key (OQ1(a)).
8. Update loader: structurally-valid 608-byte expanded `Sum6KES` →
   `Ok(KesSecret)`; every other shape stays fail-closed.
9. Append registry entries DC-CRYPTO-08 + DC-CRYPTO-09 as
   `status = "declared"`; mark `enforced` only when tests + CI land
   on the closing slice.

## Registry entries proposed

Approved as renamed by the user, with `status = "declared"` until
mechanical enforcement attaches. Written in the project's actual
schema (the registry uses `tier`/`statement`/`source`/`cross_ref`/…,
not the IDD-template `family`/`kind`/`invariant` fields).

```toml
[[rules]]
id = "DC-CRYPTO-08"
tier = "derived"
statement = "Ade-owned Sum6KES algorithm is Haskell-equivalent. ade_crypto::kes_sum::Sum6Kes is byte-identical to Haskell cardano-base's Sum6KES Ed25519DSIGN: derive_verification_key, gen_key_kes_from_seed_bytes, update_kes (chain across all 64 periods), and sign_kes produce the same bytes as the Haskell reference for every (seed, period, msg) triple. Cross-impl agreement with cardano-crypto 1.0.8 is mechanically enforced (under #[cfg(test)] only): ade-signed signatures verify under cardano-crypto's verify_kes and vice versa. After PHASE4-N-P, KesSecret.inner is the Ade-owned signing key; cardano-crypto becomes a test oracle, never a production signer."
source = "docs/clusters/PHASE4-N-P/cluster.md; docs/planning/phase4-n-p-invariants.md"
cross_ref = ["OP-OPS-04", "T-DET-01", "DC-CRYPTO-03", "DC-CRYPTO-04", "DC-CRYPTO-05", "DC-CRYPTO-06", "DC-CRYPTO-07"]
code_locus = "TBD — populated by N-P S2/S3 (algorithm implementation slice)"
tests = []
ci_script = ""
status = "declared"
introduced_in = "PHASE4-N-P"
strengthened_in = []

[[rules]]
id = "DC-CRYPTO-09"
tier = "derived"
statement = "Sum6KES expanded signing-key serde and period inference. raw_serialize_signing_key_kes / raw_deserialize_signing_key_kes are byte-identical to Haskell's rawSerialiseSignKeyKES / rawDeserialiseSignKeyKES for Sum6KES Ed25519DSIGN. The on-disk format is exactly 608 bytes; any other payload size fails closed via KesParseError::WrongPayloadSize. current_period is uniquely inferable from which sub-seeds are zeroed in the tree (no heuristic; exactly one valid period or a closed parse error). Round-trip preserves period; serialize -> deserialize -> serialize -> compare yields byte-identical output for every period 0..=63."
source = "docs/clusters/PHASE4-N-P/cluster.md; docs/planning/phase4-n-p-invariants.md"
cross_ref = ["OP-OPS-04", "T-ENC-01", "DC-CRYPTO-07", "DC-CRYPTO-08"]
code_locus = "TBD — populated by N-P S4 (serde slice) and S1 (period_from_zeroed_sum6_tree_shape proof obligation slice)"
tests = []
ci_script = ""
status = "declared"
introduced_in = "PHASE4-N-P"
strengthened_in = []
```

## Strengthenings to record on N-P close (not appended now)

- `OP-OPS-04.strengthened_in += "PHASE4-N-P"`; `open_obligation`
  **fully cleared** (cardano-cli expanded import path supported).
- `DC-CRYPTO-07.strengthened_in += "PHASE4-N-P"`; `open_obligation`
  cleared (fail-closed surface narrows from "always" to "wrong
  payload size / malformed").
- `DC-CRYPTO-03 / 04 / 05.strengthened_in += "PHASE4-N-P"` (the
  DC-CRYPTO family now has a BLUE-owned algorithm backing each
  statement; previously the algorithm lived in upstream).
- DC-CRYPTO-08 + DC-CRYPTO-09 `status` flips from `"declared"` to
  `"enforced"` once tests + CI script are wired.

---

## Related

- [[project-phase4-n-c-handoff]] — N-C raised OP-OPS-04 with the
  cardano-cli expanded-import `open_obligation`.
- [[project-phase4-n-o-closed]] — N-O closed OP-OPS-04 for the
  Ade-native flow; N-P closes the cardano-cli half.
- [[feedback-codec-closed-grammar]] — N9 codifies the
  closed-grammar discipline at the codec layer (no shim escape
  hatch).
- [[feedback-proof-discipline]] — OQ5's
  `period_from_zeroed_sum6_tree_shape` is a textbook slice-entry
  proof obligation, not a footnote.
- [[feedback-no-credential-leaks]] — OQ4(a) hex-literal-only
  fixtures keep the public-repo discipline intact even when the
  test corpus carries Sum6KES private material derived from
  throwaway seeds.
- [[feedback-real-interop-finds-codec-bugs]] — using real
  cardano-cli output as the positive corpus (vs Haskell-stack
  reference vectors) is the stronger ground truth for this
  cluster.
