# PHASE4-N-P — Sum6KES expanded compatibility (planning placeholder)

> **Status:** Queued. Drafted as a placeholder by PHASE4-N-O to
> make the cardano-cli expanded-import deferral visible in the
> planning tree. **Not yet expanded into a full cluster doc** —
> see `~/.claude/methodology/templates/cluster.md` for the
> standard fill-out when this cluster starts.

**Predecessor:** PHASE4-N-O (Ade-native KES key-gen + envelope;
cardano-cli `KesSigningKey_ed25519_kes_2^6` envelope is currently
fail-closed via `KeyLoadError::UnsupportedExpandedKesKeyFormat`).

---

## §1 Intent

Implement `ade_crypto::kes_sum` from first principles so Ade can
deserialize the cardano-cli 608-byte expanded `Sum6KES` skey
serialization and load it into a signable `KesSecret`.

The work is crypto-sensitive: must agree byte-for-byte with
Haskell's `rawSerialiseSignKeyKES (Sum6KES Ed25519DSIGN)` and
must verify cross-implementation with `cardano-crypto`'s Rust
`Sum6Kes::sign_kes` / `verify_kes` for every period 0..=63.

---

## §2 Acceptance criteria

1. `ade_crypto::kes_sum` exposes `Sum0Kes` .. `Sum6Kes` with
   recursive Blake2b-256 verification-key hashing, period-indexed
   `update_kes`, and `raw_serialize_signing_key_kes` /
   `raw_deserialize_signing_key_kes` for the 608-byte expanded
   payload.
2. 608-byte expanded `Sum6KES` payload accepted (positive corpus:
   N seeds, each with the full expanded serialization captured
   from cardano-cli ≥ 11.0.0 inside the docker preprod peer at
   `127.0.0.1:3001`).
3. 612-byte payload rejected deterministically with the
   structured error variant.
4. Malformed tree-node payloads (truncated children, wrong VK
   length, indefinite-length CBOR, etc.) rejected with closed
   error variants.
5. `update_kes` chain matches `cardano-crypto`'s reference
   behavior for every period 0..=63.
6. `sign_kes` / `verify_kes` cross-implementation vectors:
   - Sign with `ade_crypto::kes_sum::Sum6Kes`; verify with
     `cardano-crypto::kes::Sum6Kes::verify_kes`.
   - Sign with `cardano-crypto::kes::Sum6Kes::sign_kes`; verify
     with `ade_crypto::kes_sum::Sum6Kes::verify_kes`.
7. CI gate `ci/ci_check_kes_sum_expanded_compatibility.sh` enforcing
   (2)–(6) mechanically.
8. PHASE4-N-O OP-OPS-04 `open_obligation` for the cardano-cli
   import path cleared on cluster close.

---

## §3 Out of scope

- Mlocked secret memory (libsodium `sodium_mlock`). Ade keeps
  the seed/expanded tree in normal heap memory; mlocking is a
  future operational concern.
- Compact KES (`CompactSum6Kes`). The mainnet on-chain header
  format uses non-compact Sum6KES signatures; we do not need
  the compact variant.
- Replacing `cardano-crypto`'s `Sum6Kes` in the existing producer
  path. PHASE4-N-O continues to use the upstream crate for
  signing; PHASE4-N-P adds a parallel implementation strictly for
  the expanded-format deser case + cross-impl validation.

---

## §4 References

- PHASE4-N-O closure (`docs/clusters/PHASE4-N-O/`,
  `docs/active/op-ops-04-ade-native-kes-flow.md`).
- Upstream `cardano-base` Haskell:
  `Cardano.Crypto.KES.Sum` — `rawSerialiseSignKeyKES` recurrence.
- Upstream `cardano-crypto` Rust 1.0.8:
  `src/kes/sum/basic.rs` — `SumSigningKey<D, H>` (fields
  `pub(crate)`; no public constructor for the expanded bytes).

---

## §5 Why this is a separate cluster

- Crypto-sensitive: must agree byte-for-byte with Haskell across
  all periods; getting this wrong silently is a forge-validity
  defect, not a cosmetic one.
- Independent of the bounty's challenge-build acceptance test.
  The bounty proof goes through Ade-native key-gen + Ade-signed
  blocks accepted by cardano-node; cardano-cli import is an
  ergonomics improvement for existing operators, not a
  correctness gate.
- Different surface to test: the positive corpus must come from
  real cardano-cli output, the negative corpus must enumerate
  every malformed sub-tree case. Both are large enough to
  deserve their own slice plan.

See `~/.claude/methodology/templates/cluster.md` for the full
template; this placeholder is intentionally short.
