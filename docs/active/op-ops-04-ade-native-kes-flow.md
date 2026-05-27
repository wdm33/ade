# OP-OPS-04 — Ade-native KES operator flow (challenge build)

> **Audience:** challenge operators / bounty judges.
> **Status:** Normative for the current Ade release. Closes
> OP-OPS-04 for the Ade-native KES flow under PHASE4-N-O. The
> cardano-cli expanded `Sum6KES` skey import path is deferred to
> PHASE4-N-P.
> **Source:** verbatim from the user-provided operator spec.

---

## Scope

For the challenge build, Ade supports **Ade-native KES key
generation** for block production.

Ade does **not** currently import `cardano-cli node key-gen-KES`
expanded KES signing-key files. That import path is explicitly
unsupported until the follow-up Sum6KES compatibility slice.

This does not change Cardano block format, header format, KES
verification semantics, or whether Haskell / Cardano nodes
accept Ade-produced blocks. The bounty-relevant proof remains:

> Ade produces a valid block on preview / preprod
> → Haskell / Cardano nodes accept it.

The bounty requires block production and protocol / validity
agreement, **not** compatibility with every cardano-cli operator
key-file format.

---

## Required KES key flow

Use Ade's key-generation command:

```sh
ade_node --mode key_gen_kes --out-file kes.ade.skey
```

Then run Ade block production with the generated Ade-native KES
envelope:

```sh
ade_node \
  --mode admission \
  --kes-key kes.ade.skey \
  --vrf-key <vrf-key-file> \
  --operational-certificate <opcert-file> \
  <other node args>
```

(Exact VRF / opcert flags should match the current
block-production runbook.)

---

## Unsupported key flow

**Do not** use this for the current challenge build:

```sh
cardano-cli node key-gen-KES \
  --verification-key-file kes.vkey \
  --signing-key-file kes.skey
```

Ade rejects that expanded cardano-cli KES signing-key file with
a structured error:

```
KeyLoadError::UnsupportedExpandedKesKeyFormat
```

This is intentional. The observed cardano-cli expanded KES
payload is **608 bytes**, matching:

```
32 + 6 * (32 + 2 * 32) = 32 + 576 = 608
```

for `rawSerialiseSignKeyKES (Sum6KES Ed25519DSIGN)`. The upstream
`cardano-crypto` Rust crate does not expose a public constructor
for rehydrating `SumSigningKey<D, H>` from those expanded bytes.
**Ade will not use private-field hacks, unsafe transmute,
heuristic parsing, or fallback deserialization.**

---

## Security note

The Ade KES envelope contains hot signing secret material.

Treat it like a cardano-cli KES signing key:

- **do not** commit it,
- **do not** upload it,
- **do not** paste it into issues or logs,
- **do not** include it in evidence bundles,
- protect it with restrictive filesystem permissions
  (the writer enforces `0o600`),
- rotate according to normal KES-period operational policy.

Ade's signing path is RED / shell-only. Verification remains
in the deterministic core. This preserves the project rule that
private-key operations stay outside BLUE authority while
verification remains pure and replayable.

---

## Format boundary

Ade accepts only the closed Ade KES envelope format for this
challenge build:

```jsonc
{
  "format":         "ade.kes.seed.v1",
  "role":           "kes_hot_signing_key",
  "crypto":         "Sum6KES-Ed25519DSIGN",
  "seed_32":        "<64 lowercase hex chars>",
  "period_idx":     <0..=63>,
  "format_version": 1
}
```

Optional metadata may include:

- `genesis_hash`
- `network_magic`
- `created_at_slot`
- `created_by`

These metadata fields **must not** change signing semantics
unless explicitly specified and mechanically tested.

---

## Fail-closed requirements

Ade fails closed for:

- cardano-cli expanded KES skey input
- 612-byte KES payloads
- unknown KES envelope format
- missing `seed_32`
- malformed period index
- wrong key role
- unsupported crypto tag
- malformed JSON / CBOR envelope

**No fallback parser is allowed.** No unsupported format may be
guessed, partially accepted, or interpreted through compatibility
heuristics.

---

## Logging and evidence rules

Private-key material must **never** appear in:

- JSONL logs
- admission transcripts
- Shadowbox / evidence bundles
- panic messages
- debug output
- structured errors
- CLI success messages

Allowed output on `key-gen-KES` success (verbatim — exactly four
lines, no more):

```
Generated Ade KES key: kes.ade.skey
Format: ade.kes.seed.v1
Role: kes_hot_signing_key
Public verification key fingerprint: <hash>
```

Forbidden output (sample):

```
seed_32: ...
raw private key bytes: ...
expanded signing key bytes: ...
```

---

## Bounty-facing explanation

> **KES key format note**
>
> For this challenge build, Ade uses its own KES key-generation
> command:
>
> ```
> ade_node --mode key_gen_kes --out-file kes.ade.skey
> ```
>
> Ade does not currently import cardano-cli's expanded KES
> signing-key file. This is an operator file-format limitation
> only. It does not change Cardano block format, header format,
> KES verification semantics, or block acceptance by Haskell /
> Cardano nodes.
>
> Please generate the KES key with `ade_node` for the
> block-production test. cardano-cli expanded KES skey import is
> not claimed in this release.

---

## Registry / planning language

OP-OPS-04 is closed for the **Ade-native KES operator flow**.

Ade supports:

```
ade_node --mode key_gen_kes
  -> AdeKesEnvelope (ade.kes.seed.v1)
  -> Ade RED signing shell
```

Ade does not yet support:

```
cardano-cli node key-gen-KES
  -> expanded 608-byte kes.skey
  -> Ade loader
```

The expanded cardano-cli `Sum6KES` compatibility path is deferred
to **PHASE4-N-P** and must include a full `ade_crypto::kes_sum`
implementation, cross-implementation vectors, negative tests, and
CI enforcement.

**Do not claim:** Ade supports cardano-cli KES skey import.

**Do claim:** Ade can generate and use Ade-native KES keys to
produce Cardano-valid blocks.

---

## Claim boundary

This closure of OP-OPS-04 covers **only** the Ade-native KES
operator flow.

It does **not** claim:

> Ade supports cardano-cli KES skey import.

It does claim:

> Ade can generate and use Ade-native KES keys for challenge
> block production.

This avoids judge confusion while keeping the long-term
compatibility path open under PHASE4-N-P.
