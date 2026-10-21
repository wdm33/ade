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

Run 2026-10-21 with `crates/ade_runtime/tests/body_decode_census.rs` over chunks 06197–06200 of the
docker `cardano-node-preprod` ImmutableDB (slots 133,880,000–133,940,000), and over a copy of the stopped
store's `chain.db` (sha256 `3f76c5a0266f4014caae…`, the same as the original).

**CE-LV10-1, the block.** Slot 133,902,219, block number 5,184,855, header hash
`8ddb2e9618495b11dc5bf2820d177ec38453faaf81afbfdb02ab99a7ebc9e2f8`. Its `prev_hash` is Ade's durable tip
(slot 133,902,215, `9bb45a4c…6948`). Every scanned block before it passes every stage. It is the next
block after the tip, and the first one refused.

**CE-LV10-2, the bytes.** The block is in chunk `06199` at byte 543,486, as a 1,258-byte envelope with
sha256 `0fb060f0193cc9c6f8ad7dd8c8e071c14c58d585b142b191143b5193b1310329`. It is saved unmodified at
`~/.cardano-live1/lv10-census/block_133902219.cbor`, outside the repo. Ade's store held 1,036 of the
scanned blocks, all received over the wire, and all 1,036 are byte-identical to their chunk entries. So
the chunk bytes are what the peer serves.

**CE-LV10-3, the reproduction.** `decode_block`, the receive and header entry, accepts the block. So do
`decode_conway_block` and the header checks. The refusal is in the next stage, the body authority's
`ade_ledger::witness::decode_witness_infos` (reached from `apply_block_with_verdicts` →
`decode_validate_tx_bodies`). It returns exactly
`Decoding(DecodingError { offset: 9, reason: UnexpectedType })`, which the node logs as
`Validity(Body(…))`. §1 named `decode_block` as the likely site. The measurement corrects that: it is
the block-validity body step, which `block_delivered` also runs.

**CE-LV10-4, the object and the byte.**
- Object: tx 0 (`b1db2a411cb651a413840d3c8b112895a5bda2519a8ba6a372b8dd1ffc7746c2`), witness set 0, key 1
  (native scripts), wrapped in tag 258.
- The offset is relative to the witness-sets slice. That slice starts at envelope byte 1,074 (inner
  1,072), so the failing byte is envelope byte 1,083.
- The script is `83 03 20 82 [8200581c3118…] [8200581ce07c…]`: `script_n_of_k` with threshold
  `0x20` (CBOR major 1, the negative integer −1) over two key-hash scripts.
- The unmapped error, from the codec's own `decode_native_scripts`:
  `UnexpectedCborType { offset: 9, expected: "unsigned integer", actual: 1 }`.

**CE-LV10-5, the class: C1, a valid encoding Ade's codec does not accept.**
- The Conway CDDL (`conway.cddl:292`) says `script_n_of_k = (3, n : int64, [* native_script])`, with the
  note "Allegra switched to int64 for script_n_of_k thresholds". The reference checkout is
  cardano-ledger `cb57dc73`.
- cardano-ledger decodes the threshold as a Haskell `Int` (`Allegra/Scripts.hs:218`) and evaluates
  `isValidMOf n … = n <= 0 || …` (`:421–423`). A threshold of zero or below is always satisfied, so this
  script passes with no signatures.
- Ade decodes it with `cbor::read_uint` and stores `NativeScript::MOfN(n as u32, …)`
  (`ade_codec/src/allegra/script.rs:71–73`). That is an unsigned model of a signed field.
- Not C2: the era is Conway, and it is decoded as Conway. Not C3: every slice boundary checks out, and
  the error is at the threshold's own byte. Not C4: cardano-node accepted the block into its immutable
  chain. Not C5: the bytes compared are preserved wire bytes on both sides. C6 is close, but the
  difference here is not two encodings of one value. The value lies outside the type Ade chose.

**The repair it justifies, not implemented.** Model the threshold as a signed 64-bit integer, decode
it with the signed reader, and evaluate as cardano-ledger does: `n <= 0` passes, and a positive `n`
needs `n` passing sub-scripts. The same `n as u32` also truncates a threshold above `u32::MAX`. For
example, 2^32 would become 0 and pass, where cardano-ledger requires 2^32 signatures and fails. That is
a latent validity divergence on the same field. The repair belongs in its own slice, with this block
as the fixture. It touches a consensus decode surface, which is a store-semantics question.

**Separate finding: wire-supplied nesting depth.** On the way to the refused block, the census
overflowed a 2 MiB thread stack, in release too, on block 5,183,974 (slot 133,883,340). Tx 3 of that
block carries a native script `[1, [[1, [[1, …]]]]]`, an `all` nested about 5,385 levels deep in 16,185
bytes, close to the most one transaction can carry under the transaction-size limit. Its CBOR depth is
10,769. Ade's native-script decoder and `skip_item` recurse once per level, with no depth bound. The
live node passed this block because `main` (`#[tokio::main]`) awaits the relay loop directly. So
`pump_block` and its decoders run on the 8 MiB main thread, and the spawned admission wire pump only
moves bytes (DC-PUMP-01).
The margin is real but unmeasured. A move of that work to a 2 MiB tokio worker would turn this block
into a crash. This needs its own assurance slice: a depth bound or an iterative decoder. It is out of
scope here.

The census ran with `RUST_MIN_STACK=268435456` to get past that block.
