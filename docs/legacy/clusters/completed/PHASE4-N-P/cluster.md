# PHASE4-N-P — Sum6KES expanded compatibility (cluster doc)

> **Status:** Planning. 5-slice cluster expanding the BLUE
> `ade_crypto::kes_sum` algorithm, the 608-byte expanded skey serde,
> the cardano-cli cross-impl corpus, and the loader migration.
> Closes the cardano-cli-import half of OP-OPS-04 and the
> `open_obligation` on DC-CRYPTO-07 from PHASE4-N-O.
>
> **Predecessor:** PHASE4-N-O (HEAD `6eb4fbd`) — Ade-native KES
> key-gen flow shipped; cardano-cli expanded envelope fail-closed
> with `KeyLoadError::UnsupportedExpandedKesKeyFormat`.
>
> **Successor:** none planned. Mlocked memory + `CompactSum6Kes` are
> separate future clusters; both deliberately out of N-P scope.
>
> **Inputs:** [`docs/planning/phase4-n-p-invariants.md`](../../planning/phase4-n-p-invariants.md) +
> [`docs/planning/phase4-n-p-cluster-slice-plan.md`](../../planning/phase4-n-p-cluster-slice-plan.md).

---

## §1 Primary invariant

> Ade owns the Sum6KES algorithm in BLUE.
> `ade_crypto::kes_sum::Sum6Kes` is byte-identical to Haskell
> `cardano-base`'s `Sum6KES Ed25519DSIGN` for every operation in
> the closed surface — VK derivation, key generation, period
> evolution, signing, verification, expanded-skey serialization,
> expanded-skey deserialization — and is observationally equivalent
> to `cardano-crypto` 1.0.8 at the byte level for every `(seed,
> period, msg)` triple in the test corpus.
>
> The 608-byte expanded `Sum6KES` payload emitted by `cardano-cli
> node key-gen-KES` is loadable via `load_kes_signing_key_skey` —
> structurally-valid 608-byte payloads transition from N-O's
> fail-closed verdict to `Ok(KesSecret)`. Every other payload shape
> (32, 612, malformed sub-tree, period > 63 tree shape, wrong
> envelope type) remains fail-closed via closed `KesParseError` /
> `KeyLoadError` variants. No fallback parser. No heuristic
> guesswork.
>
> After this cluster, `KesSecret.inner` is the Ade-owned signing
> key; `cardano-crypto` becomes a `#[cfg(test)]` oracle, never a
> production signer. No compatibility shim may rehydrate an
> upstream `SumSigningKey` via `unsafe`, `transmute`, vendored
> `pub(crate)` access, or fork-only constructors (N9 — load-bearing
> hard prohibition).

### Why this matters

PHASE4-N-O shipped the Ade-native flow as the challenge-build
operator surface. The bounty proof goes through Ade-generated
keys → Ade-signed blocks → cardano-node acceptance. N-P does not
gate that proof; it removes the operator-friction case where
existing pools have a cardano-cli-generated `kes.skey` and want to
hand it to Ade unchanged.

The work is crypto-sensitive: a wrong byte in any algorithm path
silently forks the chain. The 5-slice shape (with S1 as a
docs-only proof-obligation slice) reflects this: write the
period-from-tree-shape function down before any code lands, then
build the algorithm + serde + corpus + migration in independently
reviewable units.

The doctrine boundary established in N-O is preserved: BLUE
(`ade_crypto::kes_sum`) owns the pure algorithm; RED
(`ade_runtime::producer::{keys, signing}`) owns custody +
zeroizing + file I/O. The `KesSecret` type-level discipline
(redacted-Debug, no public byte accessors, RED-only) carries
forward unchanged.

## §2 Scope

### In scope

- New module tree `crates/ade_crypto/src/kes_sum/`:
  - `mod.rs` — public surface, `KesAlgorithm` trait, `Sum6Kes`
    type alias.
  - `single.rs` — `Sum0Kes = SingleKes<Ed25519>` (leaf), 32-byte
    Ed25519 seed → ed25519-dalek `SigningKey` consumed for signing.
  - `sum.rs` — generic `SumKes<D, H>` recursive Sum_n.
  - `errors.rs` — closed `KesError` (runtime / algorithm errors)
    and `KesParseError` (deserialization-only).
  - `period.rs` — `period_from_zeroed_sum6_tree_shape` and
    helpers; agrees with S1's proof obligation doc.
  - `cardano_cli_corpus.rs` (`#[cfg(test)]`) — hex-literal
    throwaway-comment-prefixed 608-byte expanded payloads + their
    expected VKs + cross-impl sign/verify oracles.

- `crates/ade_crypto/src/kes.rs`:
  - Re-export `kes_sum::Sum6Kes`, `KesAlgorithm`, `KesError`,
    `KesParseError`.
  - `verify_kes_signature` continues to call the same byte-shape
    verification — but its inner call switches from
    `cardano_crypto::kes::Sum6Kes::verify_kes` to
    `ade_crypto::kes_sum::Sum6Kes::verify_kes`.
  - Existing public types (`KesPeriod`, `KesSignature`,
    `KesVerificationKey`, `SUM6_MAX_PERIOD`, `SUM6_KES_SIG_LEN`)
    unchanged.

- `crates/ade_crypto/Cargo.toml`:
  - `cardano-crypto` moves to `dev-dependencies` with the same
    feature set. Production code MUST NOT import it.
  - Add `blake2 = "0.10"` (or whichever Blake2b-256 crate the
    project already uses — `ade_crypto::blake2b_256` already
    exists; S2 reuses it).

- `crates/ade_runtime/src/producer/signing.rs`:
  - `KesSecret.inner: ade_crypto::kes_sum::Sum6Kes::SigningKey`
    (was `cardano_crypto::kes::Sum6Kes::SigningKey`).
  - All `Sum6Kes::*` calls migrate to the new BLUE API. Public
    API of `KesSecret` (`from_bytes_zeroizing`,
    `from_seed_at_period`, `verification_key_fingerprint`,
    `current_period`, `evolutions_remaining`) unchanged.
  - `kes_sign`, `kes_update` migrate.
  - The existing custom `Drop` deferral comment retired:
    `ade_crypto::kes_sum::Sum6Kes::SigningKey` MUST hand-roll
    `Drop` zeroizing its sub-seeds; we no longer rely on the
    upstream crate.

- `crates/ade_runtime/Cargo.toml`:
  - `cardano-crypto` drops `kes-sum` feature (keeps
    `vrf-draft03` + `dsign`).

- `crates/ade_runtime/src/producer/keys.rs`:
  - `load_kes_signing_key_skey` — body changes from "fail-closed
    unconditional" to "608-byte well-formed → Ok; everything else
    → fail-closed via `KeyLoadError::UnsupportedExpandedKesKeyFormat`
    or `KeyLoadError::KesParse(KesParseError::*)`".
  - New `KeyLoadError::KesParse(KesParseError)` variant.

- `ci/ci_check_kes_sum_compatibility.sh` (new) — see §4.
- `ci/ci_check_kes_envelope_closed.sh` — updated (the cardano-cli
  loader-body assertion narrows; see S5 MACs).
- `docs/active/op-ops-04-ade-native-kes-flow.md` — updated "Format
  boundary" section: cardano-cli expanded becomes an alternative
  supported flow.

- Registry updates (applied in S5):
  - `OP-OPS-04.open_obligation = null`;
    `strengthened_in += "PHASE4-N-P"`.
  - `DC-CRYPTO-07.open_obligation = null`;
    `strengthened_in += "PHASE4-N-P"`; statement narrows.
  - `DC-CRYPTO-08/09.status = "declared"` → `"enforced"`;
    `tests`/`ci_script` populated.
  - `DC-CRYPTO-03/04/05.strengthened_in += "PHASE4-N-P"`.

### Out of scope (explicit)

- Mlocked secret memory (`sodium_mlock`). Sub-seeds + ed25519
  expanded keys remain in normal heap, zeroized at `Drop`.
  Hardening with mlocked pages is a future cluster (TBD-OPS).
- `CompactSum6Kes`. Mainnet on-chain headers use the non-compact
  signature form (448 bytes). Compact (192 bytes) saves bandwidth
  on internal node-to-node but is not on the bounty path.
- VRF (`VrfDraft03`) and cold Ed25519 algorithm changes. N-P is
  KES-only; VRF and cold continue to use `cardano-crypto` for
  the algorithm (with the same RED-confined custody).
- ChainDb persistence of partially-evolved KES keys across
  process restarts. The Ade-native envelope persists `(seed,
  period_idx)`; the cardano-cli envelope persists the expanded
  tree directly. Either path is loadable post-N-P; explicit
  rotation tooling is operator-side.
- Generating cardano-cli compatible `KesSigningKey_ed25519_kes_2^6`
  envelopes from Ade (i.e., reverse direction). N-P only opens
  the *read* path. Write-path is unnecessary because operators
  generate via cardano-cli or Ade-native, not via Ade emitting
  cardano-cli envelopes.

## §3 Slice index

| Slice | Purpose | Strengthens | Introduces |
|---|---|---|---|
| **S1** | `period_from_zeroed_sum6_tree_shape` proof obligation doc (no code) | I5, N3, N4 | — |
| **S2** | Ade-owned BLUE algorithm (`gen`, `derive_vk`, `update`, `sign`, `verify`) + types + `KesError` | I1, I2, I4, I6, D1–D4, N1, N7, N9 | (DC-CRYPTO-08 still `declared`) |
| **S3** | 608-byte expanded skey + 448-byte signature serde + period inference + `KesParseError` | I3, I5, D5, N2, N3, N4, N5, N6 | (DC-CRYPTO-09 still `declared`) |
| **S4** | Real cardano-cli hex-literal corpus + cross-impl `cargo test` gate + `ci_check_kes_sum_compatibility.sh` | I1, I2 (vs ground truth) | new CI gate |
| **S5** | `KesSecret.inner` migration + loader acceptance + registry strengthenings + cluster close | I7, N8, N9, R3; clears OP-OPS-04 + DC-CRYPTO-07 open_obligations; flips DC-CRYPTO-08/09 to `enforced` | — |

## §4 Exit criteria (cluster-level MACs)

1. `docs/clusters/PHASE4-N-P/period-from-zeroed-sum6-tree-shape-proof.md`
   exists, defines the pseudocode + 64-period round-trip test
   plan, and lists every `KesParseError` variant the deserializer
   returns.
2. `ade_crypto::kes_sum::Sum6Kes::derive_verification_key(seed)`
   produces VK bytes byte-identical to
   `cardano_crypto::kes::Sum6Kes::derive_verification_key(seed)`
   for at least 8 distinct seeds in the test corpus.
3. For each `p ∈ 0..=63` and a fixed seed:
   - `update_kes` chain advances ade's signing key to period `p`
     in `p` steps.
   - `sign_kes(sk_at_p, p, fixed_msg)` produces a 448-byte
     signature that verifies under both ade's and
     cardano-crypto's `verify_kes`.
   - `raw_serialize_signing_key_kes(sk_at_p)` produces a
     608-byte buffer; deserializing returns a structurally-equal
     key; re-serializing yields byte-identical output.
   - `current_period_of_signing_key(parsed) == p`.
4. Negative tests pass: 32, 100, 607, 609, 612, 1000-byte payloads
   → `KesParseError::WrongPayloadSize`. Malformed sub-tree
   (truncated child / inconsistent vk hashes / period > 63 tree
   shape) → respective closed `KesParseError` variant.
5. cardano-cli corpus (≥ 3 throwaway-seed expanded 608-byte
   payloads captured from the docker preprod peer):
   - Each fixture is preceded by the mandatory throwaway-fixture
     comment.
   - Each fixture round-trips through ade's serde.
   - Each fixture's VK matches the cardano-cli-captured VK
     byte-for-byte.
   - Cross-impl sign/verify on each fixture: ade signs with the
     parsed key → cardano-crypto verifies; cardano-crypto signs
     → ade verifies.
6. `KesSecret.inner` migrates to
   `ade_crypto::kes_sum::Sum6Kes::SigningKey` with no observable
   behavior change for the existing N-C / N-O test corpus
   (reference vectors, signing.rs unit tests, keys.rs round-trip
   tests, key_gen.rs smoke tests all green).
7. `ade_runtime/Cargo.toml` drops the `kes-sum` feature from the
   `cardano-crypto` declaration.
8. `load_kes_signing_key_skey(path)` on a real cardano-cli
   608-byte envelope returns `Ok(KesSecret)` with
   `current_period` matching the embedded period; on every other
   payload shape returns a closed `KeyLoadError` variant.
9. `ci/ci_check_kes_sum_compatibility.sh` (new) passes:
   - Every cardano-cli corpus constant has the throwaway-fixture
     comment.
   - `cardano_crypto` is not imported in production code under
     `crates/ade_crypto/src/**` (outside `#[cfg(test)]`).
   - `crates/ade_runtime/src/producer/signing.rs` references
     `ade_crypto::kes_sum::Sum6Kes`, not
     `cardano_crypto::kes::Sum6Kes`.
   - `crates/ade_runtime/Cargo.toml` cardano-crypto features
     list does not contain `"kes-sum"`.
10. `ci/ci_check_kes_envelope_closed.sh` updated assertion: the
    cardano-cli loader body now contains `Ok(KesSecret)` *only*
    via the `raw_deserialize_signing_key_kes` path; it still
    contains `UnsupportedExpandedKesKeyFormat` for the
    payload-size-mismatch branch.
11. `ci/ci_check_private_key_custody.sh` still passes (`Drop` for
    the new BLUE types is hand-rolled with zeroize semantics).
12. `cargo test --workspace` clean (excluding the
    `epoch_oracle_comparison` wallclock budget caveat documented
    in [[project-phase4-n-o-closed]]).
13. Registry updates from S5 applied; DC-CRYPTO-08 + DC-CRYPTO-09
    flipped to `enforced`.
14. `docs/active/op-ops-04-ade-native-kes-flow.md` updated; the
    "unsupported flow" section retired; bounty-facing README
    text retains the Ade-native flow as the recommended path but
    acknowledges cardano-cli flow as supported.
15. `/cluster-close PHASE4-N-P` runs the four grounding-doc
    regenerators + IDD review + security review against the full
    cluster diff. No BLOCK findings.
16. Commit + push all 5 slices with the project-override trailer.

## §5 Hard prohibitions

- **N9.** No compatibility shim via `unsafe`, `transmute`, vendored
  `pub(crate)` access, or fork-only constructors.
  `cardano-crypto` is `#[cfg(test)]` + `dev-dependencies` only
  after S5.
- **N6.** No `.skey` envelope files committed under
  `crates/ade_crypto/` or anywhere in the repo. Hex-literal
  `&[u8; 608]` corpus only, with the mandatory throwaway-fixture
  comment.
- **N7.** `Drop` for every new BLUE signing-key type (Sum0..Sum6)
  best-effort zeroizes its sub-seed buffers. The existing
  `ci_check_private_key_custody.sh` discipline carries forward
  (no public byte accessors, custom `Debug` redaction, RED-only
  custody wrappers).
- No `cardano_crypto` import in production code under
  `crates/ade_crypto/src/**` after S2 lands. Mechanically enforced
  by `ci_check_kes_sum_compatibility.sh`.
- No heuristic period inference. `period_from_zeroed_sum6_tree_shape`
  returns exactly one `u32` or a closed `KesParseError`.
- No silent acceptance of period > 63 tree shapes.
- No private-key bytes in any error message, `Debug` output,
  JSONL log, panic, or test fixture committed as `.skey`.

## §6 Replay obligations

- **T-DET-01** — strengthened. The BLUE algorithm is deterministic;
  signing/verification/serialization is replay-byte-identical.
- **T-ENC-01** — strengthened. The 608-byte expanded skey
  serialization and 448-byte signature serialization are
  canonical (no alternate encodings tolerated).
- **DC-CRYPTO-03/04/05** — strengthened by the BLUE algorithm
  ownership (the rules' enforcement now backs onto Ade-owned
  code rather than upstream).
- **R2 (cross-impl replay equivalence)** — load-bearing for the
  migration: a WAL replay containing a block whose KES sig was
  produced by `ade_crypto::kes_sum::Sum6Kes::sign_kes` hash-equals
  the same replay if the sig were produced by
  `cardano_crypto::kes::Sum6Kes::sign_kes`. This is the proof
  that the migration in S5 does not fork the chain.

## §7 References

- Predecessor: `6eb4fbd` (PHASE4-N-O close).
- Invariants: [`docs/planning/phase4-n-p-invariants.md`](../../planning/phase4-n-p-invariants.md).
- Cluster plan: [`docs/planning/phase4-n-p-cluster-slice-plan.md`](../../planning/phase4-n-p-cluster-slice-plan.md).
- Haskell reference: `cardano-base/cardano-crypto-class/src/Cardano/Crypto/KES/Sum.hs`,
  `cardano-base/cardano-crypto-class/src/Cardano/Crypto/KES/Single.hs`,
  `cardano-base/cardano-crypto-class/src/Cardano/Crypto/Hash/Blake2b.hs`.
- Upstream Rust reference: `cardano-crypto-1.0.8/src/kes/sum/{mod,basic}.rs`,
  `cardano-crypto-1.0.8/src/kes/single/mod.rs`.
- Doctrine: [[feedback-codec-closed-grammar]] (N9 codifies closed
  grammar at the codec layer); [[feedback-proof-discipline]] (S1
  proof obligation slice); [[feedback-no-credential-leaks]] (OQ4
  hex-literal corpus only); [[feedback-real-interop-finds-codec-bugs]]
  (S4 cardano-cli ground-truth).
