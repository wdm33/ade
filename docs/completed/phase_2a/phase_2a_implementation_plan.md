# Implementation Plan: Phase 2A — Cryptographic Verification

## Context

Phase 1 closed the byte-authority loop on the full golden corpus (42 mainnet blocks, 7 eras) with byte-identical round-trip and established `PreservedCbor<T>.wire_bytes()` for hash-critical paths. The `ade_crypto` crate was an empty shell (contract headers + deny attributes only).

Phase 2A populates `ade_crypto` with pure cryptographic verification functions whose observable behavior must be byte-identical to the Haskell cardano-node 10.6.2. This is the foundation for all subsequent ledger validation (Phase 2B) and consensus (Phase 4).

## Constitutional Framing

All verification functions must deterministically distinguish **malformed** from **well-formed-invalid** from **valid**. The three-class verdict discipline is normative across all algorithms.

Key invariants: DC-CRYPTO-01 (crypto matches Haskell), T-DET-01 (determinism), T-ERR-01 (structured errors), T-KEY-01 (no signing in BLUE), CN-CRYPTO-04 (no fallback/retry).

## Slice Structure

### Slice 2A-1: Hash Authority Closure
Close byte-authority and Blake2b obligations for all known hash-critical paths. After this slice, `ade_crypto` is the single authoritative source for Blake2b-256/224 hashing. Introduces `CryptoError` enum, `HashAlgorithm` trait, `blake2b_256()`, `blake2b_224()`, and domain wrappers. Replaces 3 duplicate hash implementations in testkit.

### Slice 2A-2: Ed25519 Witness-Verification Closure
Close standard Ed25519 and Byron extended Ed25519 witness verification with deterministic malformed/invalid separation. Two verification surfaces: standard (32-byte vk, 64-byte sig) and Byron bootstrap (64-byte extended vk, first 32 bytes used). Test vectors cross-validated against libsodium (PyNaCl).

### Slice 2A-3: VRF Boundary Closure
Prove the implementation boundary for VRF verification (ECVRF-ED25519-SHA512-Elligator2, IETF draft-irtf-cfrg-vrf-03). Extractive verification: valid proofs return `Ok(VrfOutput)`. Defers full protocol-context verification (alpha construction from epoch nonce + slot) to Phase 4 consensus.

### Slice 2A-4: KES/Opcert Closure
Close Sum6KES signature verification (depth 6, 64 periods) and operational certificate verification (Ed25519 cold key signature over CBOR-encoded opcert data). KES uses Merkle tree with Blake2b-256 internal hashing.

## Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `blake2` | 0.10 | Blake2b-256/224 (RustCrypto, pure Rust) |
| `ed25519-dalek` | 2 | Ed25519 verification |
| `cardano-crypto` | 1.0 | VRF (draft-03) and KES (Sum6) verification |

## Verification

All slices verified by: `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check`, and all CI scripts.
