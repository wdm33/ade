# Credential key/script discriminant fidelity corpus (OQ5, DC-LEDGER-10)

Backs `crates/ade_ledger/tests/credential_fidelity_corpus.rs` and the codec
decode tests. Credentials are built in-test (typed `StakeCredential` + minimal
CBOR), so this directory holds only this README.

## What is mechanically closed (CE-1..CE-7)

1. **Decode fidelity (CE-1):** both era `decode_stake_credential` map the 0/1
   type tag to `KeyHash`/`ScriptHash`; an unknown credential tag is a
   deterministic reject. Tests: `shelley_credential_preserves_discriminant`,
   `conway_credential_preserves_discriminant`, `unknown_credential_tag_rejects`.
2. **Closed type (CE-2):** `StakeCredential` is the closed 2-variant enum; no
   `Hash28 -> StakeCredential` coercion on the BLUE path. Compile +
   `ci/ci_check_credential_discriminant_closed.sh`.
3. **Distinct keys (CE-3/CE-6):** a key-hash and a script-hash credential sharing
   the same 28 bytes are distinct keys in CertState and ConwayGovState — no
   silent collapse or overwrite. Tests:
   `keyhash_scripthash_same_bytes_are_distinct_certstate`,
   `..._govstate`.
4. **Fingerprint fidelity (CE-4):** two states differing only in a credential's
   key/script tag fingerprint differently. Tests: `discriminant_changes_fingerprint`
   (lib), `discriminant_changes_fingerprint_corpus`.
5. **Replay (CE-5):** a mixed key/script credential sequence accumulates
   byte-identical across runs over the discriminated fingerprint (T-DET-01).
   Test: `credential_accumulation_replays_byte_identical`.

## Fingerprint migration (T-DET-01, intentional)

`write_stake_credential` now emits a discriminant byte (0 KeyHash / 1 ScriptHash)
before the hash, and the ConwayGovState gov-map writers use it. This is a
deliberate dual cert-state + gov-state encoding upgrade toward cardano-node's
`Credential` keying. Empty / credential-free states are byte-identical to the
pre-migration encoding (no golden drift observed at S1). The stake-distribution
snapshot stays `Hash28`-keyed (a separate, non-goal surface).

## Environment-blocked open obligation (NOT closed here)

Real-chain agreement of the discriminated keys vs cardano-node's
`Credential`-keyed `UMap`/`VState` is environment-blocked: the epoch-576 /
boundary ledger-state snapshots are absent locally (recoverable from the
ImmutableDB EBS snapshots). Reclassified per the project tier doctrine, the same
posture as DC-LEDGER-08 / DC-LEDGER-09 / DC-TXV-06. The synthetic
distinctness + replay + decode-fidelity gate above is the mechanical closure OQ5
ships.

## Declared non-goals (NOT in this cluster)
- Withdrawal / required-signer / address credential discriminant fidelity.
- Stake-distribution snapshot (`epoch.rs`, `Hash28`-keyed) discriminant.
- Byron credential surface (different scheme).
