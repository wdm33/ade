# Opcert envelope fixtures — OQ4 proof obligation capture

> **Phase:** PHASE4-N-R-A S1 / N-R-PREFLIGHT.
> **Captured:** 2026-05-27 against the local docker container
> `cardano-node-preprod` (cardano-node 11.0.1, cardano-cli
> 11.0.0.0 git rev `97036a66bcf8c89f687ae57a048eecc0389977ef`,
> ghc-9.6, linux-x86_64).
> **Capture procedure:**
>
> ```bash
> docker exec cardano-node-preprod sh -c '
>   cd /tmp &&
>   cardano-cli latest node key-gen \
>     --cold-verification-key-file cold.vkey \
>     --cold-signing-key-file cold.skey \
>     --operational-certificate-issue-counter-file cold.counter &&
>   cardano-cli latest node key-gen-KES \
>     --verification-key-file kes.vkey \
>     --signing-key-file kes.skey &&
>   cardano-cli latest node issue-op-cert \
>     --kes-verification-key-file kes.vkey \
>     --cold-signing-key-file cold.skey \
>     --operational-certificate-issue-counter-file cold.counter \
>     --kes-period 0 \
>     --out-file node.opcert'
> docker cp cardano-node-preprod:/tmp/node.opcert ./
> ```

## Locked envelope shape

The cardano-cli `NodeOperationalCertificate` envelope is a JSON
object with `type` / `description` / `cborHex` fields. The
`cborHex` decodes to a CBOR **`array(2)`** whose elements are:

- **Element 0:** `array(4)` of
  - `bytes(32)` — `hot_vkey` (the KES verification key for
    this period; matches the `KesVerificationKey_ed25519_kes_2^6`
    envelope's payload bytes byte-for-byte).
  - `uint` — `sequence_number` (issue counter).
  - `uint` — `kes_period` (the KES period this opcert is
    valid from).
  - `bytes(64)` — `sigma` (Ed25519 signature by the cold
    signing key over the canonical opcert pre-image).
- **Element 1:** `bytes(32)` — `cold_vk` (the cold-key
  verification key, used for self-verification of `sigma`).

The **N-Q `OperationalCert` struct** (in
`ade_types::shelley::block`) corresponds to element 0 of this
envelope. The C1 opcert parser must:

1. Parse the JSON envelope (closed `type` check:
   `NodeOperationalCertificate`).
2. Hex-decode `cborHex`.
3. Decode CBOR `array(2)`.
4. Extract element 0 → `OperationalCert { hot_vkey,
   sequence_number, kes_period, sigma }`.
5. Optionally extract element 1 → `cold_vk` for
   sigma-verification (sigma signs
   `hot_vkey ‖ seq_no_be8 ‖ kes_period_be8`).

## Fixtures

| File | Purpose | Expected parser outcome |
|---|---|---|
| `accepted-cardano-cli-11.0.0.opcert.json` | Real, well-formed envelope captured from docker | Accept; produce `OperationalCert` matching the documented field layout |
| `cold.vkey.json` | Companion cold VK envelope (for sigma verification testing) | Type-check passes (`StakePoolVerificationKey_ed25519`) |
| `kes.vkey.json` | Companion KES VK envelope (must match opcert element 0[0]) | Type-check passes (`KesVerificationKey_ed25519_kes_2^6`); hot_vkey bytes match opcert element 0[0] |
| `cold.counter.json` | Companion counter envelope | Type-check passes (`NodeOperationalCertificateIssueCounter`) |
| `malformed-type.opcert.json` | Envelope `type` field is wrong (`GovernanceVotingKey_ed25519`) | Reject with closed envelope-type error |
| `malformed-cborhex.opcert.json` | `cborHex` contains non-hex characters | Reject with hex-decode error |
| `wrong-arity.opcert.json` | Inner array has 5 elements instead of 4 | Reject with arity error |

## Captured field values (for cross-impl checks)

From `accepted-cardano-cli-11.0.0.opcert.json`:

| Field | Hex |
|---|---|
| `hot_vkey` | `0c6ea1d8de23bf345996c6b26e0699f81a8e3fe79021b764ba3727c0eeb62314` |
| `sequence_number` | `0` |
| `kes_period` | `0` |
| `sigma` | `ca267e71ec582af8cbfb15b6856d0098febefbdeb55b7fc813b312ba3625766dc9a32324ced476d31b26e61f3250ac8fe410cf603b39dec92b7c0ee27e480d0a` |
| `cold_vk` | `180537b7910f1dcb35bed2bcbc2d374f0f8a68f4f63cd0662afa38d3c4499d93` |

The KES VK envelope's payload (after `5820` CBOR `bytes(32)` tag)
is the 32 bytes `0c6ea1...62314` — byte-identical to opcert
element 0[0]. The cold VK envelope's payload (after `5820` tag)
is the 32 bytes `180537...499d93` — byte-identical to opcert
element 1.

## Honest-scope reminder

These keys + opcert are **synthetic test fixtures**. The
cold/KES signing keys were generated inside the docker
container, are **not** deployed to any chain, and are **not**
committed to the repo. Only verification keys + the opcert
envelope itself + the issue counter are in-tree.

## Use sites

- N-R-A A2: `verify_and_evaluate_leader` proof unit tests
  consume `cold.vkey.json` and the synthetic stake-distribution
  to validate the BLUE leader-check evaluator.
- N-R-C C1: opcert envelope parser test suite consumes ALL
  fixtures here (4 accept + 3 reject paths).
