# SLICE LV-11 — native-script threshold fidelity

**Kind:** compatibility repair of a consensus decode surface, with a persisted-history census that decides
the store-version question.

**Rule:** DC-LEDGER-SCRIPT-01 (new).

## 1. The defect (from LV-10)

LV-10 found that the live node stopped at slot 133,902,219 (block 5,184,855, header `8ddb2e96…e2f8`).
Tx 0 of that block carries the native script `[3, -1, [sig, sig]]`, a `script_n_of_k` with threshold −1.
Ade's body authority refused it in `decode_witness_infos` with
`UnexpectedCborType { offset: 9, expected: "unsigned integer", actual: 1 }`.

The cause is one type mismatch, and it fails in both directions:

- **Ade rejects what Cardano accepts.** The threshold is read with `cbor::read_uint`, so any negative
  threshold refuses the whole block.
- **Ade can accept what Cardano rejects.** The value is stored as `n as u32`
  (`ade_codec/src/allegra/script.rs:73`). A threshold of 2^32 truncates to 0, and Ade's evaluator passes
  a threshold of 0. cardano-ledger requires 2^32 passing sub-scripts and fails it. A threshold above
  `i64::MAX` fails cardano-ledger's decoder outright, and Ade's decoder accepts it.

## 2. What Cardano does

- **CDDL.** Conway `conway.cddl:292` says `script_n_of_k = (3, n : int64, [* native_script])`, noting
  that Allegra switched the threshold to `int64`.
- **Decode.** cardano-ledger `Allegra/Scripts.hs:218` decodes it as `From` into a Haskell `Int`, a signed
  64-bit integer. cborg's `Int` decoder takes CBOR major 0 or 1 and fails on any value outside
  `[i64::MIN, i64::MAX]`.
- **Evaluation.** `Allegra/Scripts.hs:421–423`:

  ```haskell
  isValidMOf n SSeq.Empty = n <= 0
  isValidMOf n (ts SSeq.:<| tss) =
    n <= 0 || if go ts then isValidMOf (n - 1) tss else isValidMOf n tss
  ```

  So a threshold of zero or below always passes. A positive `n` passes once `n` sub-scripts have
  passed, and evaluation stops at that point. The reference checkout is cardano-ledger `cb57dc73`.

## 3. The invariant

**DC-LEDGER-SCRIPT-01.** `script_n_of_k.n` is decoded and represented with Cardano's signed 64-bit
semantics:

- every integer in `[i64::MIN, i64::MAX]` is accepted;
- every integer outside that range is refused;
- no narrowing or unsigned reinterpretation may change a decode outcome or an evaluation verdict;
- a threshold `n <= 0` is satisfied; a threshold `n > 0` is satisfied exactly when at least `n`
  sub-scripts pass.

## 4. The change

- `ade_types::allegra::script::NativeScript::MOfN(u32, …)` becomes `MOfN(i64, …)`. The type cannot hold
  a value outside Cardano's range, and there is no cast left to truncate.
- `ade_codec::allegra::script::decode_native_script` reads the threshold with a signed reader, built on
  `read_any_int`, that refuses a magnitude outside `i64` with a typed `CodecError`.
- `ade_ledger::scripts::evaluate_native_script` evaluates `MOfN` as `isValidMOf` does. `n <= 0` passes.
  Otherwise it counts passing sub-scripts as `u64`, stops once the count reaches `n`, and compares
  without any narrowing.
- `NativeScriptFailure::ThresholdNotMet { required: u32, provided: u32 }` becomes
  `{ required: i64, provided: u64 }`, so the error can name the real threshold.

No other decoder, no CBOR primitive and no other script constructor changes. The fail-closed behaviour
for every other malformed script stays as it is.

## 5. The store-version question

The type change alone does not decide this. The project rule (`store_semantics.rs`) is to bump
`STORE_SEMANTICS_VERSION` exactly when a store written by the previous binary is no longer
replay-equivalent under this one. Two facts decide it.

**What the old binary could have persisted differently.** A stored block is one the old binary
accepted. Replaying it under the new binary could differ in only two ways:

1. **Decode outcome.** The new decoder refuses a threshold above `i64::MAX`. A stored block holding one
   would now fail replay. Every stored block was also accepted by cardano-node, whose decoder refuses
   such a threshold, so none should exist. The census checks this rather than assuming it.
2. **Evaluation verdict.** The new evaluator differs from the old one only for a threshold below 0 or
   above `u32::MAX`. Whether a verdict difference reaches durable state depends on who consumes the
   verdict. In the tree today, `evaluate_native_script` has one production call site
   (`rules.rs`, in `decode_validate_tx_bodies`), which only increments `BlockVerdict`
   counters `native_script_passed` / `native_script_failed`. Nothing reads those counters, and
   `BlockVerdict` is not persisted. The census re-proves this with a search, recorded with the
   results.

**The census.** Walk every block in a COPY of the stopped live store's `chain.db`, which is the whole
replay interval Ade itself decoded, up to the durable tip at 133,902,215. For every witness-set native
script, record every `script_n_of_k` threshold as read raw from the CBOR, independent of the decoder
under repair. The walk uses an explicit stack, so the 5,385-level script LV-10 found does not depend on
thread stack. Then:

```
any threshold < 0 or > u32::MAX before the durable tip?
  yes -> the old store may be semantically different -> bump STORE_SEMANTICS_VERSION, rebuild
  no  -> the persisted prefix is unaffected -> apply the policy: semantics-neutral, no bump
```

The census covers the witness-set field because that is the only place Ade decodes a native script
(`decode_native_scripts` has one production caller, `witness.rs`). Native scripts inside reference-script
outputs and auxiliary data are never decoded into `NativeScript` by Ade, so they cannot have shaped
stored state through this code.

**The lock gate.** `ci/store-semantics-surface.lock` does not list `ade_codec/src/allegra/script.rs`,
`ade_types/src/allegra/script.rs`, `ade_ledger/src/scripts.rs` or `ade_ledger/src/witness.rs`. So
`ci_check_store_semantics_lock.sh` would not fire for this change either way, and the decision rests on
§5's argument and census. Whether block-body decoders belong on that surface is recorded as a question
for the owner, not changed here.

## 6. MECHANICAL ACCEPTANCE CRITERIA

| CE | Criterion |
|---|---|
| CE-LV11-1 | The real block (slot 133,902,219, sha256 `0fb060f0…0329`) is a committed fixture. `decode_witness_infos` accepts it, tx 0 decodes to `MOfN(-1, [Sig, Sig])`, and that script passes with no signatures |
| CE-LV11-2 | Constructed `[3, n, [sig]]` scripts decode to exactly `n` for −1, 0, 1, `u32::MAX`, `u32::MAX + 1`, 2^32, `i64::MAX` and `i64::MIN`, and are refused for `i64::MAX + 1` (major 0) and `i64::MIN − 1` (major 1, argument 2^63) and `u64::MAX` |
| CE-LV11-3 | Evaluation matches `isValidMOf` at the boundaries: `n <= 0` passes with no signatures (including over an empty list); `n = 1` passes with one valid sub-script and fails with none; `n = 2^32`, `u32::MAX + 1` and `i64::MAX` fail with one passing sub-script; `ThresholdNotMet` carries the untruncated threshold |
| CE-LV11-4 | Mutation accountability: reverting the decoder to `read_uint` fails CE-LV11-1 and CE-LV11-2 with the original error; restoring a `u32` truncation fails CE-LV11-2/3 at 2^32; dropping the `n <= 0` clause fails CE-LV11-3. Each mutation asserted applied, tree restored byte-exact |
| CE-LV11-5 | A CI gate runs the LV-11 tests, asserts they ran, and structurally refuses an unsigned threshold read or a `u32` threshold type returning |
| CE-LV11-6 | The persisted-history census over the stopped store's `chain.db` copy: number of blocks, scripts and thresholds walked, the min and max threshold, and the count outside `0..=u32::MAX`. A search proves the verdict consumers named in §5. The store-version decision is recorded from those results |
| CE-LV11-7 | Live: the node, resumed on its existing store if CE-LV11-6 allows it (or on a rebuilt store if not), passes block 5,184,855 and keeps following past it |

## 7. OUT OF SCOPE

- **Deep nesting (LV-12).** The recursive decoders and evaluator stay recursive here. LV-12 makes them
  independent of thread stack.
- **Native-script verdicts not enforced (new observation).** §5 found that no production path consumes
  `evaluate_native_script`'s verdict. Required signers are enforced (`check_required_signers`), but a
  transaction whose native script fails is not refused by Ade. That is a wider compatibility gap than
  the threshold, and it needs its own census before any change. It is recorded here and not fixed.
- **Timelock interval.** cardano-ledger evaluates `InvalidBefore` and `InvalidHereafter` against the
  transaction's validity interval. Ade's evaluator uses the block slot. While verdicts are not
  enforced, this changes nothing; it belongs with the item above.
- **`From<CodecError> for LedgerError` keeps losing detail.** That is a separate assurance slice.
- **Adding decoders to the store-semantics lock surface.** That is an owner decision (§5).
