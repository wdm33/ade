# OP-OPS-04 — KES operator flow (Ade-native + cardano-cli expanded)

> **Audience:** challenge operators / bounty judges.
> **Status:** Normative for the current Ade release. Closes
> OP-OPS-04 under PHASE4-N-O (Ade-native flow) + PHASE4-N-P
> (cardano-cli expanded `Sum6KES` import — now supported via the
> Ade-owned BLUE algorithm).
> **Source:** §"Scope" through §"Logging and evidence rules"
> verbatim from the original user-provided operator spec; the
> "Unsupported key flow" section has been retired and the
> bounty-facing claim boundary updated per PHASE4-N-P S5 close.

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

## Alternative: cardano-cli expanded KES skey (also supported, since PHASE4-N-P)

After PHASE4-N-P S5, Ade additionally imports cardano-cli's
expanded `Sum6KES` signing-key file directly:

```sh
cardano-cli node key-gen-KES \
  --verification-key-file kes.vkey \
  --signing-key-file kes.skey
```

The resulting `kes.skey` (a 608-byte `KesSigningKey_ed25519_kes_2^6`
text-envelope) is now loadable via:

```sh
ade_node \
  --mode admission \
  --kes-key kes.skey \
  ...
```

Internally, Ade's loader runs the BLUE-owned Sum6KES
deserializer (`ade_crypto::kes_sum::Sum6Kes::raw_deserialize_signing_key_kes`)
which matches Haskell `cardano-base`'s `rawDeserialiseSignKeyKES`
byte-for-byte over the canonical 608-byte expanded layout. Any
payload of any other size, or a 608-byte payload with structural
defects (truncated sub-tree, inconsistent vk hash, leaf-all-zero,
period > 63 tree shape), fail-closes via a closed
`KeyLoadError::UnsupportedExpandedKesKeyFormat` or
`KeyLoadError::KesParse(KesParseError::*)` variant. No fallback
parser; the deserializer is the structural validator.

### Why this changed in PHASE4-N-P

PHASE4-N-O fail-closed the cardano-cli expanded import path
because `cardano-crypto` Rust 1.0.8 exposes no public constructor
for rehydrating `SumSigningKey<D, H>` from expanded bytes.
PHASE4-N-P resolved the gap by reimplementing Sum6KES from
scratch in `ade_crypto::kes_sum` (BLUE-owned), matching Haskell
`cardano-base` byte-for-byte (including the `expand_seed` prefix
bytes 0x01 / 0x02 that `cardano-crypto` Rust 1.0.8 happens to get
wrong — see [`docs/clusters/PHASE4-N-P/S4.md`](../../docs/clusters/PHASE4-N-P/S4.md)
for the discovery). Cross-impl agreement is mechanically enforced
via the cardano-cli ground-truth corpus at
`crates/ade_crypto/src/kes_sum/cardano_cli_corpus.rs`.

The N9 hard prohibition (no upstream-shim hack via `unsafe`,
`transmute`, vendored `pub(crate)` access, or fork-only
constructors) is mechanically enforced by
`ci/ci_check_kes_sum_compatibility.sh`.

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
> For the challenge build, Ade supports both KES key flows:
>
> - `ade_node --mode key_gen_kes --out-file kes.ade.skey` —
>   Ade-native flow (recommended for fresh setups).
> - `cardano-cli node key-gen-KES --signing-key-file kes.skey
>   --verification-key-file kes.vkey` — cardano-cli expanded
>   flow (useful for operators with existing keys).
>
> Either flow produces a key Ade can load via its admission
> mode. Block-production semantics are identical; the difference
> is purely the on-disk envelope format.

---

## Registry / planning language

OP-OPS-04 is **fully closed** as of PHASE4-N-P S5 (`open_obligation
= null`). Ade supports both KES key flows:

```
ade_node --mode key_gen_kes
  -> AdeKesEnvelope (ade.kes.seed.v1)
  -> Ade RED signing shell (BLUE-owned ade_crypto::kes_sum::Sum6Kes)

cardano-cli node key-gen-KES
  -> KesSigningKey_ed25519_kes_2^6 (608-byte expanded)
  -> Ade loader (BLUE-owned ade_crypto::kes_sum::Sum6Kes deserializer)
```

The expanded cardano-cli `Sum6KES` compatibility path is implemented
by `ade_crypto::kes_sum::Sum6Kes::raw_deserialize_signing_key_kes`,
backed by cross-implementation vectors from the cardano-cli
ground-truth corpus (`crates/ade_crypto/src/kes_sum/cardano_cli_corpus.rs`),
negative tests for every malformed-tree shape, and CI enforcement
via `ci/ci_check_kes_sum_compatibility.sh`.

**Claim:** Ade can generate and use Ade-native KES keys AND
import cardano-cli-generated KES keys to produce Cardano-valid
blocks. Both flows route through the same BLUE-owned signing
algorithm; cardano-node accepts blocks signed by either flow.

---

## Claim boundary

This closure of OP-OPS-04 covers both KES key flows after
PHASE4-N-P S5. Specifically:

It claims:

> Ade can generate and use Ade-native KES keys for challenge
> block production.

It also claims:

> Ade can import cardano-cli-generated `KesSigningKey_ed25519_kes_2^6`
> envelopes (608-byte expanded `Sum6KES`) and use them for
> challenge block production. The import path is mechanically
> validated against a cardano-cli ground-truth corpus.

It does **not** claim:

> Ade matches `cardano-crypto` Rust 1.0.8 byte-for-byte.

(`cardano-crypto` Rust 1.0.8 uses different `expand_seed` prefix
bytes than Haskell `cardano-base`; Ade matches Haskell — see the
PHASE4-N-P S4 closure record for the discovery.)
