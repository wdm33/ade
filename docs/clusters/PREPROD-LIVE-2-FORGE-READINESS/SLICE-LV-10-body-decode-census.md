# SLICE LV-10 — census: which block, object and byte stopped the node at 313

**Kind:** census. No codec change, no node change, no guard change. The current fail-closed behaviour
stays.

## 1. The observation

The LV-8 live run (`~/.cardano-live1/ade-preprod-lv8b`, release build of `6b5c34b9`) crossed 304→305
through 312→313 with no restart. Around 2026-10-21T04:08Z, after the follow tip reached slot
133,897,405, it stopped:

```
ade_node --mode node: relay run-loop sync step failed (Pump("Receive(Validity(Body(Decoding(DecodingError { offset: 9, reason: UnexpectedType }))))")); failing closed (no skip-past, no fallback).
```

The path is `forward_sync::pump_block` → `block_validity::decode_block` → `decode_block_envelope`,
then `decode_conway_block` (or `decode_babbage_block`), then the header checks. Every `CodecError`
reaches the log through `From<CodecError> for LedgerError`, which keeps only an offset and a coarse
reason. Three different codec errors become `UnexpectedType`:
- `UnexpectedCborType { offset, .. }`;
- `UnknownCertTag { tag, offset }`;
- `UnknownEraTag` (offset forced to 0, so not this one).

The offset is whatever the failing decoder held. It may be relative to a nested slice, such as one
transaction body, and not to the block. So "offset 9" alone says neither which object failed nor
which byte.

## 2. What the census must establish, in order

1. The failing block: slot, block number, header hash.
2. Its original bytes, as the peer holds and serves them. They must never be re-encoded through Ade.
3. Which decode authority entered: envelope, Babbage or Conway block, or header.
4. Which nested object was being decoded (header, transaction body *i*, witness set *i*, auxiliary
   data, invalid-tx list), and that object's start offset within the block.
5. The full, unmapped `CodecError`: variant, offset, and the expected and actual type or tag.
6. The absolute byte offset of the failure in the block, and the CBOR major type and argument found
   there.

## 3. Classification (after the measurement, before any change)

| class | meaning | action it would justify |
|---|---|---|
| C1 | a valid Conway encoding Ade's codec does not accept | compatibility repair |
| C2 | a valid block decoded with the wrong era or schema | era-dispatch repair |
| C3 | a container or slice boundary computed wrong | framing repair |
| C4 | the peer's bytes are malformed | none; rejection is correct |
| C5 | preserved wire bytes confused with Ade-canonical bytes | byte-authority repair |
| C6 | the decoder assumes one representation where Cardano permits several | wire decoders must accept every permitted form |

The deciding oracle is the original bytes plus the cardano-ledger CDDL and decoder for the era.
cardano-node already accepted this block, and that proves the bytes are valid Conway but not which
of C1–C3, C5 or C6 applies. C4 needs positive evidence that the bytes are wrong.

## 4. Method

- **The bytes.** The failing block never reached Ade's ChainDb: decode runs before StoreBlockBytes.
  The oracle copy is the docker `cardano-node-preprod` ImmutableDB. block-fetch serves a block's
  stored bytes unchanged, so the chunk entry is the wire block. The census checks this on the
  neighbouring blocks. Ade's ChainDb holds wire-received bytes for the blocks before the failure, and
  a copy of it is compared byte for byte with the chunk entries at the same slots. It also checks
  that the failing entry's `prev_hash` is Ade's durable tip hash.
- **The tool.** A read-only, `#[ignore]`d evidence test in `ade_runtime/tests/`, in the same family
  as the LV-5 and LV-7 census tools. It walks the chunk entries in a slot range and runs the
  production `decode_block` on each one's original bytes, so the first failure is found the way the
  node finds it. On a failure it:
  - re-runs the codec stages directly, to get the unmapped `CodecError`;
  - decodes the block's top-level parts one by one, to name the failing part and its offsets;
  - writes the entry's bytes, unmodified, to a file outside the repo, with its sha256.
- **The fixture.** If the result leads to a repair, the raw block is committed as a fixture in that
  later slice, as LV-8 did. CI never depends on the census tool.

## 5. MECHANICAL ACCEPTANCE CRITERIA

| CE | Criterion |
|---|---|
| CE-LV10-1 | The failing block named by slot, block number and header hash, and shown to be the next block after Ade's durable tip that `decode_block` refuses |
| CE-LV10-2 | Its original bytes saved with sha256, and shown byte-identical to what the peer serves (hash match) |
| CE-LV10-3 | The production `decode_block` reproduces the live error exactly (`Decoding { offset: 9, UnexpectedType }`) on those bytes |
| CE-LV10-4 | The unmapped `CodecError`, the failing object, its start offset, and the absolute byte offset with the CBOR head found there |
| CE-LV10-5 | A classification (C1–C6) argued from the CDDL and the bytes, with the repair, if any, named but not implemented |

## 6. OUT OF SCOPE

Any codec, ledger or node change. Weakening the fail-closed stop. Making `From<CodecError> for
LedgerError` keep more detail: if the census shows the lost detail cost real diagnosis time, that is a
separate assurance slice.

## 7. RESULTS

Not yet run.
