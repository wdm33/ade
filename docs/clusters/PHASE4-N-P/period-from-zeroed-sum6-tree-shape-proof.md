# `period_from_zeroed_sum6_tree_shape` — proof obligation

> **Status:** Slice-entry artifact for PHASE4-N-P S1. **Authoritative
> for the deserializer.** S3's `raw_deserialize_signing_key_kes` must
> implement exactly this algorithm; any drift is a slice-failure, not
> a refactor opportunity.
>
> **Inputs:** [`docs/planning/phase4-n-p-invariants.md`](../../planning/phase4-n-p-invariants.md)
> §1.I5 + §2.N3 + §2.N4.
>
> **Doctrine:** [[feedback-proof-discipline]]. Unknown compatibility
> facts are slice-entry proof obligations, not footnotes.

---

## 1. Tree layout

The Sum_n KES signing key is a balanced binary tree of depth `n`.
For Sum6KES with Ed25519DSIGN and Blake2b256 hashing, the in-memory
shape is:

```text
                    Sum6 (depth 6, 64 periods)
                    seed_6 || vk6_left || vk6_right
                         /                       \
              Sum5 (depth 5, 32 periods)    Sum5 (mirror; only one active at a time)
              seed_5 || vk5_left || vk5_right
              /                          \
            ...                          ...
          Sum1 (depth 1, 2 periods)
          seed_1 || vk1_left || vk1_right
          /                          \
       Sum0 (depth 0, 1 period)    Sum0 (mirror)
       sk_ed25519 (32-byte seed)
```

At each internal level `n`, the in-memory representation is a
4-tuple:

```text
SumKES_n :=
  ( sk_child : SignKeyKES_(n-1)     -- the currently-active sub-tree's signing key
  , seed_n  : 32 bytes              -- seed for the OTHER sub-tree, or zeroed if consumed
  , vk_left  : 32 bytes             -- left  sub-tree's verification key (Blake2b256 hash)
  , vk_right : 32 bytes             -- right sub-tree's verification key (Blake2b256 hash)
  )
```

The leaf (Sum0) is a single 32-byte Ed25519 seed.

### Forward-secrecy semantics

When `update_kes` advances past the level-`n` boundary (period
crosses `2^(n-1)` within the sub-tree's range), the level-`n`
state mutates:

1. `sk_child` is replaced from the level-`(n-1)` *left* sub-tree's
   signing key to the level-`(n-1)` *right* sub-tree's signing key,
   reconstructed by `gen_key_kes_from_seed_bytes(seed_n)`.
2. `seed_n` is overwritten with 32 zero bytes (best-effort
   zeroize).
3. `vk_left` and `vk_right` remain as captured at key generation.

This is irreversible. The level-`n` seed_n can never be recovered;
the left sub-tree's signing keys (for periods < `2^(n-1)` within
this sub-tree) are gone.

## 2. On-disk byte layout (608 bytes, `rawSerialiseSignKeyKES`)

The Haskell recurrence:

```haskell
rawSerialiseSignKeyKES (SignKeySumKES sk_child seed_n vk_left vk_right) =
       rawSerialiseSignKeyKES sk_child   -- recursive: child's serialization
    <> rawSerialiseSeedOrZero seed_n     -- 32 bytes
    <> rawSerialiseVerKeyKES vk_left     -- 32 bytes
    <> rawSerialiseVerKeyKES vk_right    -- 32 bytes
rawSerialiseSignKeyKES (SignKeySingleKES sk_ed25519) =
    sk_ed25519.bytes                      -- 32 bytes
```

Concretely for Sum6KES, the 608-byte payload has the following
big-endian byte ranges (level numbers correspond to Sum_n where
n=0 is the leaf):

| Bytes | Field | Level | Notes |
|---|---|---|---|
| `[0..32)`     | leaf Ed25519 sk seed | 0 | current period's signing seed |
| `[32..64)`   | seed_1 (32 bytes)     | 1 | zero ⇒ right sub-tree active |
| `[64..96)`   | vk_1_left (32 bytes)  | 1 | |
| `[96..128)`  | vk_1_right (32 bytes) | 1 | |
| `[128..160)` | seed_2 (32 bytes)     | 2 | |
| `[160..192)` | vk_2_left (32 bytes)  | 2 | |
| `[192..224)` | vk_2_right (32 bytes) | 2 | |
| `[224..256)` | seed_3 (32 bytes)     | 3 | |
| `[256..288)` | vk_3_left (32 bytes)  | 3 | |
| `[288..320)` | vk_3_right (32 bytes) | 3 | |
| `[320..352)` | seed_4 (32 bytes)     | 4 | |
| `[352..384)` | vk_4_left (32 bytes)  | 4 | |
| `[384..416)` | vk_4_right (32 bytes) | 4 | |
| `[416..448)` | seed_5 (32 bytes)     | 5 | |
| `[448..480)` | vk_5_left (32 bytes)  | 5 | |
| `[480..512)` | vk_5_right (32 bytes) | 5 | |
| `[512..544)` | seed_6 (32 bytes)     | 6 | |
| `[544..576)` | vk_6_left (32 bytes)  | 6 | |
| `[576..608)` | vk_6_right (32 bytes) | 6 | |

Total: `32 + 6 * (32 + 32 + 32) = 32 + 576 = 608`. ✓

The level-1 seed is at offset 32 (not 0); the leaf sk seed
occupies [0..32). Each subsequent level adds 96 bytes.

## 3. `period_from_zeroed_sum6_tree_shape` — pseudocode

```text
fn period_from_zeroed_sum6_tree_shape(bytes: &[u8; 608]) -> Result<u32, KesParseError> {
    // Reject leaf-zero (exhausted or malformed key).
    if bytes[0..32] == [0; 32] {
        return Err(KesParseError::LeafSignKeyAllZero);
    }

    let mut period: u32 = 0;

    // Level 6 first (most significant bit). Seed at [512..544).
    if bytes[512..544] == [0; 32] {
        period += 32;   // 2^(6-1) = 32
    }
    // Level 5. Seed at [416..448).
    if bytes[416..448] == [0; 32] {
        period += 16;   // 2^(5-1) = 16
    }
    // Level 4. Seed at [320..352).
    if bytes[320..352] == [0; 32] {
        period += 8;
    }
    // Level 3. Seed at [224..256).
    if bytes[224..256] == [0; 32] {
        period += 4;
    }
    // Level 2. Seed at [128..160).
    if bytes[128..160] == [0; 32] {
        period += 2;
    }
    // Level 1. Seed at [32..64).
    if bytes[32..64] == [0; 32] {
        period += 1;
    }

    // Sum6 encodes periods 0..=63 by structure; period is in range
    // by construction. No overflow possible from this function.
    debug_assert!(period <= SUM6_MAX_PERIOD);
    Ok(period)
}
```

Constant `SUM6_MAX_PERIOD = 63` — defined in `ade_crypto::kes`.

## 4. Proof sketch: agreement with `update_kes`

### Claim

For every seed `s` and every `p ∈ 0..=63`:

```text
let sk_p = update_kes^p(gen_key_kes_from_seed_bytes(s));
let bytes_p = raw_serialize_signing_key_kes(sk_p);
period_from_zeroed_sum6_tree_shape(bytes_p) == Ok(p).
```

### Induction on tree depth

**Base case (Sum1, depth 1, 2 periods).**

- `p = 0`: leaf seed is non-zero (left sub-tree's seed), level-1
  seed is non-zero (right sub-tree's seed_1, still intact).
  `period = 0`. ✓
- `p = 1`: after `update_kes`, level-1's `sk_child` swaps to the
  right sub-tree's leaf; level-1 seed_1 zeroized. Leaf seed (at
  bytes [0..32)) is the right sub-tree's Ed25519 seed (non-zero
  by Blake2b256 PRF non-trivially). `period_from_…` sees
  seed_1 zero ⇒ period = 1. ✓

**Inductive step (Sum_n, depth n).**

Assume the claim holds for Sum_(n-1) over periods `0..=2^(n-1)-1`.
Show it holds for Sum_n over periods `0..=2^n - 1`.

- For `p ∈ 0..2^(n-1)` (left sub-tree active):
  - level-`n` seed is non-zero (the right sub-tree's seed,
    untouched).
  - The level-`n` sub-key is `update_kes^p(gen_key_kes_from_seed_bytes(s_left))`,
    a Sum_(n-1) key at period `p`. By IH,
    `period_from_zeroed_sum_(n-1)_tree_shape(child_bytes) == p`.
  - `period_from_zeroed_sum6_tree_shape` reads no `2^(n-1)`
    contribution (level-`n` seed non-zero) and recurses into
    the child for the lower bits.
  - Total: `period = 0 (level n) + p (child) = p`. ✓

- For `p ∈ 2^(n-1)..2^n` (right sub-tree active):
  - level-`n` seed is zeroed (consumed by the `update_kes` that
    crossed the boundary).
  - The level-`n` sub-key is now the right sub-tree's signing
    key at period `p - 2^(n-1)`. By IH applied to this
    sub-tree, the child contributes `p - 2^(n-1)` to the
    period.
  - `period_from_zeroed_sum6_tree_shape` adds `2^(n-1)` for the
    zeroed level-`n` seed and `p - 2^(n-1)` from recursion.
  - Total: `period = 2^(n-1) + (p - 2^(n-1)) = p`. ✓

By induction, the claim holds for Sum6 over `p ∈ 0..=63`. ∎

### Non-collision assumption

The proof relies on level-`n` seeds being non-zero when present
(otherwise we'd falsely conclude the right sub-tree is active).
The seeds are derived via Blake2b256 expansion of the parent
seed. The probability that any derived seed is the 32-byte zero
string is `2^-256`. We treat this as cryptographically impossible
and do not include an additional check for "non-zero seed at
left-sub-tree-active state."

The leaf check `bytes[0..32] == [0; 32]` covers the related case
where the deserialized key carries a zeroed leaf (exhausted key
or malformed payload).

## 5. Closed `KesParseError` variants

The deserializer S3 introduces exactly the following variants.
N-P S5 wraps them in `KeyLoadError::KesParse(KesParseError)` at
the loader boundary.

```rust
pub enum KesParseError {
    /// Payload size is not 608 bytes. Carries actual size for the
    /// CI gate; never carries any byte content.
    WrongPayloadSize { actual: usize },

    /// Leaf Ed25519 signing-key seed is all zeros. Either an
    /// exhausted key or a malformed payload; both fail-closed.
    LeafSignKeyAllZero,

    /// At level `level`, vk_left does not match the
    /// recursively-derived sub-tree VK (when verifiable — i.e.,
    /// when seed at that level is non-zero, meaning the left
    /// sub-tree is still constructible).
    InconsistentSubtreeVkLeft { level: u32 },

    /// At level `level`, vk_right does not match the
    /// recursively-derived sub-tree VK (when verifiable — i.e.,
    /// from current child sub-tree when right is active, or from
    /// seed when left is active).
    InconsistentSubtreeVkRight { level: u32 },

    /// `level` is outside 1..=6. Defense-in-depth; should not
    /// arise from a 608-byte payload because the offsets are
    /// fixed.
    LevelOutOfRange { level: u32 },
}
```

`level` numbering follows §2's table: 1 = innermost SumKES wrap,
6 = outermost. Level 0 (the leaf) is not parameterized; its
malformation is `LeafSignKeyAllZero`.

**Hard prohibitions on the error surface:**

- Variants **must not** carry raw key bytes, raw seeds, or any
  hex / decimal representation of secret material.
- `Debug` impls **must** redact any inner state — but since
  variants carry only `level: u32` and `actual: usize`, this is
  automatic.
- The variants are exhaustively matched in the loader; no
  wildcard `_ =>` arms in production code.

## 6. 64-period exhaustive fixture-test plan (S3 deliverable)

S3 must add a test that exercises the period inference for **every
period 0..=63** from a fixed seed. The test shape:

```rust
#[test]
fn period_from_zeroed_sum6_tree_shape_agrees_with_update_kes_chain() {
    let seed: [u8; 32] = [0x42; 32];  // arbitrary but fixed
    let mut sk = Sum6Kes::gen_key_kes_from_seed_bytes(&seed).unwrap();

    for p in 0u32..=63 {
        let bytes = raw_serialize_signing_key_kes(&sk);
        let inferred = period_from_zeroed_sum6_tree_shape(&bytes).unwrap();
        assert_eq!(inferred, p, "period mismatch at p={}", p);

        if p < 63 {
            sk = update_kes(sk, p).unwrap().expect("update must succeed for p < 63");
        }
    }
}
```

Plus the round-trip test:

```rust
#[test]
fn round_trip_preserves_period_for_every_period() {
    let seed: [u8; 32] = [0x42; 32];
    let mut sk = Sum6Kes::gen_key_kes_from_seed_bytes(&seed).unwrap();

    for p in 0u32..=63 {
        let bytes_before = raw_serialize_signing_key_kes(&sk);
        let parsed = raw_deserialize_signing_key_kes(&bytes_before).unwrap();
        let bytes_after = raw_serialize_signing_key_kes(&parsed);
        assert_eq!(bytes_before, bytes_after, "re-serialize drift at p={}", p);
        assert_eq!(
            period_from_zeroed_sum6_tree_shape(&bytes_after).unwrap(),
            p,
            "period drift at p={}",
            p
        );

        if p < 63 {
            sk = update_kes(sk, p).unwrap().expect("update");
        }
    }
}
```

Plus the negative tests:

```rust
#[test]
fn wrong_payload_size_rejected() {
    for size in [0, 32, 100, 512, 607, 609, 612, 1000] {
        let bytes = vec![0xAB; size];
        match Sum6Kes::raw_deserialize_signing_key_kes(&bytes) {
            Err(KesParseError::WrongPayloadSize { actual }) => assert_eq!(actual, size),
            other => panic!("expected WrongPayloadSize, got {:?} at size {}", other, size),
        }
    }
}

#[test]
fn leaf_all_zero_rejected() {
    let mut bytes = [0u8; 608];
    // Construct an otherwise-valid tail (e.g., from a real fixture)
    // but zero the leaf.
    let mut valid = real_throwaway_fixture_p0();
    valid[0..32].copy_from_slice(&[0; 32]);
    match Sum6Kes::raw_deserialize_signing_key_kes(&valid) {
        Err(KesParseError::LeafSignKeyAllZero) => (),
        other => panic!("expected LeafSignKeyAllZero, got {:?}", other),
    }
}

#[test]
fn inconsistent_vk_left_rejected_when_verifiable() {
    let mut bytes = real_throwaway_fixture_p0();
    // At p=0, level-1 seed is non-zero; flip one byte of vk_1_left.
    bytes[64] ^= 0xFF;
    match Sum6Kes::raw_deserialize_signing_key_kes(&bytes) {
        Err(KesParseError::InconsistentSubtreeVkLeft { level: 1 }) => (),
        other => panic!("expected InconsistentSubtreeVkLeft@1, got {:?}", other),
    }
}
```

(Per-level VK consistency tests for levels 2..6 follow the same
shape and are exhaustive in S3.)

## 7. Verification chain in the deserializer (S3 implementation note)

S3's `raw_deserialize_signing_key_kes` must, after computing the
period:

1. Walk from leaf upward to level 6.
2. At each level `n`:
   - Construct the level-`n` sub-key's verification key from the
     recursively-derived child.
   - If `seed_n` is non-zero (left sub-tree active): also
     reconstruct the right sub-tree from `seed_n` via
     `gen_key_kes_from_seed_bytes` recursively, derive its vk,
     and compare against the bytes' `vk_right` at level `n`.
     Then derive `vk_left` from the *current child* and compare
     against the bytes' `vk_left` at level `n`.
   - If `seed_n` is zero (right sub-tree active): the original
     left sub-tree is unrecoverable. Derive `vk_right` from the
     current child and compare against the bytes' `vk_right`.
     `vk_left` is accepted as given (cannot be re-derived).
3. The root vk for the resulting Sum6KES key is
   `blake2b256(vk_6_left || vk_6_right)` and is **not** stored in
   the 608-byte payload — it's computed on demand by
   `derive_verification_key`.

This walk catches any malformed sub-tree where the vks don't
agree with their derivations.

## 8. Out-of-scope concerns

- **Mlocked seed buffers** — the level-`n` seeds live in normal
  heap memory; `Drop` zeroizes them. Mlocking is future work.
- **Constant-time leaf-zero comparison** — a `==` comparison
  against `[0; 32]` is sufficient because the leaf zero check
  is fail-closed at parse time, not a hot path; an attacker
  observing timing learns only "leaf was zero," which is already
  a published error.
- **Indefinite-length CBOR byte string** — the 608-byte raw
  payload is the inner contents of the cardano-cli envelope's
  cborHex field, which already passed through
  `decode_cbor_byte_string` (PHASE4-N-O `keys.rs`). The
  CBOR-shape rejection lives at the envelope layer, not here.

## 9. References

- Haskell: `cardano-base/cardano-crypto-class/src/Cardano/Crypto/KES/Sum.hs`
  (`SignKeySumKES`, `rawSerialiseSignKeyKES`, `updateKES`).
- Haskell: `cardano-base/cardano-crypto-class/src/Cardano/Crypto/KES/Single.hs`
  (`SignKeySingleKES Ed25519DSIGN`).
- Rust: `cardano-crypto-1.0.8/src/kes/sum/basic.rs` (`SumSigningKey<D, H>`,
  the in-memory layout that mirrors the Haskell shape).
- Rust: `cardano-crypto-1.0.8/src/kes/single/mod.rs` (`SingleKes<D>`).
- Invariants: [`phase4-n-p-invariants.md`](../../planning/phase4-n-p-invariants.md)
  §1 (I3, I5), §2 (N2, N3, N4, N6).
