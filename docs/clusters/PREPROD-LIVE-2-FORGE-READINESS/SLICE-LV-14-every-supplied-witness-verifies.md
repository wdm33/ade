# SLICE LV-14 — every supplied witness signature verifies

**Kind:** ledger-validity correctness. Changes which transactions, and so which blocks, are accepted.

**Rule:** DC-TXV-08 (new).

## 1. The finding (LV-13)

Ade checks witness signatures in one place, `tx_validity::verify_required_witnesses`, and only for the key
hashes a transaction is required to be signed by. A vkey witness for any other key is parsed and size-checked
and its signature is never verified. Bootstrap witnesses (witness-set key 2) are never verified in Conway at
all. So Ade accepts a transaction carrying a supplied witness whose signature does not verify.

That matters beyond the witness itself. A native script's `sig` is satisfied by key-hash membership in the
supplied witnesses, as in Cardano (DC-LEDGER-SCRIPT-03), so a script can pass on a key whose witness carries a
bad signature. In Cardano that transaction is rejected by a different rule; in Ade it is accepted.

## 2. What Cardano requires (cardano-ledger cb57dc73)

`Shelley/Rules/Utxow.hs:400`, run by Babbage's UTXOW (`Babbage/Rules/Utxow.hs:377`) and so by Conway's:

```
validateVerifiedWits tx = failureOnNonEmpty (failed <> failedBootstrap) InvalidWitnessesUTXOW
  failed          = [ k | WitVKey k _ ∈ addrTxWits,     not (verifyWitVKey txBodyHash w) ]
  failedBootstrap = [ k | w ∈ bootAddrTxWits,           not (verifyBootstrapWit txBodyHash w) ]
```

`verifyBootstrapWit txbodyHash w = verifySignedDSIGN (bwKey w) txbodyHash (bwSignature w)`
(`Keys/Bootstrap.hs:154`): a plain Ed25519 check with the witness's 32-byte key over the body hash. The chain
code and attributes take no part in it; they feed only the key hash.

It is a separate rule from `validateNeededWitnesses` (`:421`), which proves only that each required key hash
is among the supplied ones. Ade keeps the two separate.

## 3. The invariant

**DC-TXV-08.** Every vkey witness and every bootstrap witness a Conway transaction supplies, required or not,
has an Ed25519 signature that verifies under its own key over the transaction's preserved body hash. A
witness whose key or signature is malformed, or whose signature does not verify, rejects the transaction
deterministically, and in the block path the block. The check is UTxO-independent, so it runs in both modes,
in the block path and in `tx_phase_one`. It is mechanically distinct from required-signer coverage
(`verify_required_witnesses`), which is unchanged.

## 4. The change

1. `WitnessInfo` keeps each bootstrap witness (`BootstrapWitnessRef`: key, signature, chain code,
   attributes), from which the LV-13 key hashes are still derived.
2. `tx_validity::witness::verify_supplied_witnesses(body_hash, vkey_witnesses, bootstrap_witnesses)`, with
   two new `WitnessClosureError` variants, `InvalidSuppliedWitnessSignature` and
   `InvalidBootstrapWitnessSignature`, both of class `WitnessInvalid`. Wrong sizes reuse
   `MalformedWitnessField`.
3. Both paths call it after the required-signer check. So a required key with a bad signature is still
   reported as `InvalidWitnessSignature`, as today; any other bad signature is the new error.

## 5. The census (before enforcement)

Over every block of the halted LV-12 store copy (326,471 blocks), every supplied vkey and bootstrap witness
of every transaction is verified over its preserved body hash. Expected: zero failures. Any failure is an Ade
bug until shown otherwise.

## 6. MECHANICAL ACCEPTANCE CRITERIA

- **CE-LV14-1.** A transaction whose only bad signature is on a non-required vkey witness is rejected with
  `InvalidSuppliedWitnessSignature`, in both modes and both paths. The same transaction with a good signature
  is accepted.
- **CE-LV14-2.** A bad bootstrap-witness signature is rejected with `InvalidBootstrapWitnessSignature`. A good
  one, on a real preprod bootstrap transaction, is accepted.
- **CE-LV14-3.** A required key with a bad signature is still `InvalidWitnessSignature` from the
  required-signer check, and a missing required key is still `MissingRequiredSigner`. The two checks stay
  distinct.
- **CE-LV14-4.** Wrong-size vkey or signature, on a vkey or a bootstrap witness, is `MalformedWitnessField`.
- **CE-LV14-5.** A native script satisfied only by a key whose witness has a bad signature: the transaction
  is rejected.
- **CE-LV14-6.** Replay gives the same structured error twice. The block path and the standalone path give
  the same error on the same real transaction with one signature byte flipped.
- **CE-LV14-7 (census).** §5, zero failures.
- **CE-LV14-8 (mutations).** Each fails the intended tests and a new gate: verifying only required keys
  again; skipping bootstrap witnesses; ignoring the verification result.
- **CE-LV14-9.** Sweep equals the baseline. LV-13's gate still passes.
- **CE-LV14-10 (live).** The LV-13 store restarts on the LV-14 binary with no reset and follows. The
  store-semantics decision follows the census.

## 7. OUT OF SCOPE

- Plutus and state-backed verdicts the block path counts but does not enforce, the sequential intra-block
  UTxO, and the UTxO-dependent native-script obligations of LV-13. Those are the next slice.
- Bootstrap key hashes in required-signer coverage (`check_required_signers`).
- Pre-Conway eras.

## 8. RESULTS

Code: `df057072`. Every CE below passed mechanically.

- **CE-LV14-7 (census, before enforcement).** `persisted_witness_signature_census` over the halted LV-12
  store copy (326,471 blocks, slots 129,813,444..=137,096,367, 991,602 transactions): 1,028,673 vkey witnesses
  and 314 bootstrap witnesses verified over their preserved body hashes, zero failures. The 314 also confirm
  on real data that a bootstrap witness verifies under its 32-byte key alone.
- **CE-LV14-1..6.** `crates/ade_ledger/tests/lv14_every_supplied_witness_verifies.rs`, 6 tests. A bad signature
  on a non-required witness is `InvalidSuppliedWitnessSignature` in full and reduced mode, and the good
  signature is accepted. A required key's bad signature stays `InvalidWitnessSignature` and a missing one
  `MissingRequiredSigner`; in reduced mode, where the payer is not derivable, the same bad signature is still
  caught by the new check. Wrong sizes on either witness kind are `MalformedWitnessField`. A `sig` native
  script passing on a badly signed key does not carry the transaction. On the real LV-13 fixtures, one
  flipped signature byte gives the same structured error from `apply_block_with_verdicts` and from
  `tx_validity`, twice identically: `InvalidBootstrapWitnessSignature` on the bootstrap transaction, and a
  witness rejection on the timelock one.
- **CE-LV14-8 (mutations).** Skipping the vkey walk, skipping bootstrap witnesses, and ignoring
  `verify_ed25519`'s result each fail the intended test and the gate, and the gate's structural checks name
  each. (The first gate draft accepted a loop over `vkey_witnesses.iter().take(0)`; it now requires the full
  walk.)
- **CE-LV14-9.** Sweep: 283 scripts, the failing set identical to the baseline's 46. LV-13's gate passes.
- **Store semantics.** Not bumped: `rules.rs` is on the lock surface and the lock records a neutral entry at
  11. The check writes no state, and the census shows no persisted block is refused.
- **CE-LV14-10 (live).** `docs/evidence/run-stores/preprod-live2c/lv14-ce10-restart-on-supplied-witnesses.md`.
