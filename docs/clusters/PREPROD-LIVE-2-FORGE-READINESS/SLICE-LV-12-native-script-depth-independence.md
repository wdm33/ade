# SLICE LV-12 — native scripts and CBOR skipping do not depend on thread stack

**Kind:** release and liveness assurance on a consensus decode surface. No change to which blocks are
accepted, and no depth limit.

**Rule:** DC-LEDGER-SCRIPT-02 (new).

## 1. The finding (LV-10, measured here)

Preprod block 5,183,974 (slot 133,883,340, 44,432 bytes, sha256 `ca0feb0e…6710`) is a real block that
cardano-node accepted. Its tx 3 carries a native script nested about 5,385 levels deep, an `all`
inside an `all` and so on, in 16,185 bytes. That is close to the most a single transaction can hold,
since each level costs about 3 bytes. The block's CBOR depth is 10,769.

Stack needed to process that block, measured on a thread of exactly the given size, by binary search:

| stage | release | debug |
|---|---|---|
| `decode_block_envelope` / `decode_block` / `decode_conway_block` (all bound by `cbor::skip_item`) | ~1.2 MiB | ~28 MiB |
| + `decode_witness_infos` (native-script decode) | ~2.5 MiB | ~28 MiB |
| `evaluate_native_script` alone | ~0.6 MiB | ~3.1 MiB |
| derived `Drop` of the decoded script alone | ~0.37 MiB | ~0.69 MiB |
| derived `Clone` / `PartialEq` alone | ~0.94 MiB | ~3.4 MiB |

So the block crashes a 2 MiB thread, which is a tokio worker's default stack. The live node survived it
only because `pump_block` runs on the 8 MiB main thread, in a release build. That leaves about 3×
headroom over this one block, and none for a debug build.

## 2. Why not a depth limit

A depth limit would trade a crash for a refusal. Cardano bounds nesting only through the transaction
and block size limits. Any limit below what those allow rejects a block cardano-node accepts, and this
block shows such scripts really appear. The requirement is the opposite one: **whether a block decodes
and evaluates must not depend on stack size at all.**

## 3. The invariant

**DC-LEDGER-SCRIPT-02.** No code path that decodes, skips, evaluates, clones, compares or drops a
wire-supplied native script, or skips any wire-supplied CBOR item, uses call-stack depth proportional
to the input's nesting depth. Every such walk keeps its pending work on the heap. Decode outcomes, error
variants and offsets, and evaluation verdicts are unchanged.

## 4. Where the recursion is

A search of `ade_codec` and `ade_ledger` production code for functions that recurse on wire data finds
exactly these:

1. `cbor::skip_item`, the generic skipper with 248 call sites, which recurses once per nesting level of
   arrays, maps and tags;
2. `allegra::script::decode_native_script` ↔ `decode_script_array`, a mutual recursion;
3. `scripts::evaluate_native_script`;
4. the derived `Drop`, `Clone`, `PartialEq` and `Debug` of `NativeScript`, which recurse because the type
   does (`All(Vec<NativeScript>)`).

Plutus data is decoded inside the evaluator dependency (aiken/uplc), not by Ade's codec. It is out of
scope here (§7).

## 5. The change

- **`cbor::skip_item` becomes iterative.** It keeps an explicit `Vec` of pending item counts:
  definite, indefinite, map pairs, tag content. It returns the same `(start, end)` range and the same
  error variant at the same offset for every input. Byte strings and text strings keep their current
  handling.
- **`NativeScript` becomes a flat tree.** The type holds a `Vec` of nodes in post-order, children before
  parents, with each composite node naming its children by index range. It has no recursive field, so
  the derived `Drop`, `Clone`, `PartialEq`, `Eq` and `Debug` are flat. Layout is canonical: the decoder
  and the constructors both produce post-order, so structural equality is equality of the node vectors.
  Constructors (`sig`, `all`, `any`, `m_of_n`, `invalid_before`, `invalid_hereafter`) keep tests
  readable, and a read-only view (`root()`, `kind()`, `children()`) replaces pattern matching on the
  enum.
- **The decoder** walks the CBOR with an explicit stack of open composite nodes and emits nodes in
  post-order. Error variants and offsets are those of the current decoder.
- **The evaluator** is one forward pass over the post-order nodes. Each node's verdict is computed from
  its children's, which are already known. The verdict, including which failure is reported, is the
  current evaluator's. Short-circuiting changes only how much work is done, never the result, because
  evaluation has no side effects. LV-11's `isValidMOf` semantics are kept exactly.

The alternative of keeping the recursive enum and hand-writing iterative `Drop`, `Clone`, `PartialEq`
and `Debug` was rejected. A `Drop` impl forbids moving out of the enum, and every future derive would
quietly reintroduce recursion. In the flat form there is no recursive type for a derive to follow, and
the gate can check that structurally.

## 6. MECHANICAL ACCEPTANCE CRITERIA

| CE | Criterion |
|---|---|
| CE-LV12-1 | The real block (slot 133,883,340, sha256 `ca0feb0e…6710`) is a committed fixture. `decode_block`, `decode_conway_block`, `decode_witness_infos`, evaluation of every script, clone, equality and drop all complete on a **256 KiB** thread, in the debug test profile CI runs |
| CE-LV12-2 | Constructed worst cases on the same 256 KiB thread: a native script nested to the depth a 16,384-byte transaction allows, through decode, evaluation, clone, equality and drop; and CBOR nested arrays 90,000 levels deep (one byte per level, the block-body size bound) through `skip_item` |
| CE-LV12-3 | `skip_item` differential against the recursive original (kept in test code only): identical result, range or error variant and offset, on every committed block fixture, on every truncated prefix of several of them, and on constructed indefinite-length, map and tag forms |
| CE-LV12-4 | Native-script differential against the recursive decoder and evaluator (kept in test code only): identical decode result or error on constructed valid and malformed scripts and on every fixture's witness sets; identical verdict, including the failure reported, across signature sets and slots. The LV-11 tests pass unchanged in meaning |
| CE-LV12-5 | Mutation accountability: restoring recursion in `skip_item`, and restoring a recursive decoder or evaluator, each makes a CE-LV12-1/2 test overflow its stack (asserted from the test output), and the gate fails. Tree restored byte-exact |
| CE-LV12-6 | A CI gate runs the LV-12 tests, asserts they ran, and structurally refuses a recursive `NativeScript` type and a self-call in `skip_item`, the native-script decoder or the evaluator |
| CE-LV12-7 | Store semantics: decode outcomes are unchanged (CE-LV12-3/4), and a re-run of LV-11's persisted census over the stopped store's `chain.db` copy decodes all 184,490 blocks with identical thresholds. Recorded as semantics-neutral, with no bump |
| CE-LV12-8 | Live: the running LV-11 node is restarted on the LV-12 binary on the same store and keeps following |

## 7. OUT OF SCOPE

- **Plutus data depth.** Datums and redeemers can nest about one byte per level, and Ade's codec skips
  them with `skip_item`, which this slice makes iterative. Decoding them happens inside aiken/uplc, a
  dependency. Whether that recursion is stack-safe needs its own census.
- **Which thread runs transaction ingress.** `tx_validity::phase1` decodes witness sets through
  `decode_witness_info_single`. After this slice that decode no longer depends on stack, so which
  thread runs it no longer matters for native scripts.
- **Native-script verdict enforcement** (LV-11 §7). This slice keeps the verdict exactly as it is and
  does not start enforcing it.
