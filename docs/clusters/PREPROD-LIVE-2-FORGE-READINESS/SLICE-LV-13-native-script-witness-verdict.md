# SLICE LV-13 — native-script witness verdicts decide transaction validity

**Kind:** ledger-validity correctness. Changes which transactions, and so which blocks, are accepted.

**Rule:** DC-LEDGER-SCRIPT-03 (new).

## 1. The finding

Ade decodes and evaluates native scripts, but the verdict decides nothing. In the block path
(`rules.rs`, `decode_validate_tx_bodies`) every witness-set native script is evaluated only to bump
two `BlockVerdict` counters that nothing reads. The standalone path (`tx_validity::tx_phase_one`) does
not evaluate native scripts at all. Ade has no needed-scripts derivation and no native-script hash (the
`script_hash` field of `ScriptError` is never filled), and the witness decoder does not keep each
script's bytes. So Ade accepts a transaction whose required native script fails, a transaction missing
a required script, and a transaction carrying an extraneous one.

Three more defects sit on the same surface, and enforcement would expose each of them:

- **Wrong timelock clock.** Ade evaluates `invalid_before` / `invalid_hereafter` against the containing
  block's slot. Cardano evaluates them against the transaction's validity interval (§2.3). The two
  disagree in both directions: a transaction with no upper bound never satisfies an expiry timelock in
  Cardano, but passes in Ade whenever the block is early enough.
- **Bootstrap witnesses ignored.** Cardano's signer set for native scripts includes the key hashes of
  bootstrap witnesses (§2.4). Ade skips witness-set key 2. A native script satisfied only by a bootstrap
  witness would be falsely rejected once verdicts are enforced.
- **No hash from preserved bytes.** A native-script hash is `Blake2b-224(0x00 ‖ script bytes)` over the
  bytes as they appear on the wire. Ade would have to hash a re-encoding, which T-ENC-01 forbids.

## 2. What Cardano requires (cardano-ledger cb57dc73)

### 2.1 The rule

Conway's UTXOW is Babbage's (`Conway/Rules/Utxow.hs:221`, `transitionRules = [babbageUtxowTransition]`).
`Babbage/Rules/Utxow.hs:327`:

```
scriptsNeeded       = getScriptsNeeded utxo txBody
scriptsProvided     = getScriptsProvided utxo tx          -- reference scripts ∪ witness scripts
scriptHashesNeeded  = getScriptsHashesNeeded scriptsNeeded
runTest $ validateFailedBabbageScripts tx scriptsProvided scriptHashesNeeded
sReceived = keys (witness scripts);  sRefs = keys (getReferenceScripts utxo (refInputs ∪ inputs))
runTest $ babbageMissingScripts pp scriptHashesNeeded sRefs sReceived
```

- `validateFailedBabbageScripts` (`:238`): every provided native script whose hash is needed and which
  does not validate → `ScriptWitnessNotValidatingUTXOW (set of hashes)`. Provided-but-not-needed
  scripts are **not** evaluated.
- `babbageMissingScripts` (`:219`): `neededNonRefs = needed − sRefs`;
  `extra = sReceived − neededNonRefs` → `ExtraneousScriptWitnessesUTXOW`;
  `missing = neededNonRefs − sReceived` → `MissingScriptWitnessesUTXOW`.

`runTest` accumulates, so Cardano reports every failing set, not the first. UTXOW runs for every
transaction in a block whatever its `is_valid` flag, and any failure invalidates the block.

`sReceived` is every witness script: native (key 1) and Plutus V1/V2/V3 (keys 3/6/7). The missing and
extraneous checks therefore need Plutus witness hashes too, although only native scripts are evaluated
here.

### 2.2 The needed set (`Conway/UTxO.hs:64`, `getConwayScriptsNeeded`)

| purpose | source | script hash when |
|---|---|---|
| spending | each `input`, resolved in the UTxO | its address's payment credential is a script (`getScriptHash addr`) |
| rewarding | each withdrawal's reward account | its credential is a script |
| certifying | each certificate | `getScriptWitnessConwayTxCert` (below) |
| minting | each policy id in `mint` | always |
| voting | each voter | committee-hot or DRep voter with a script credential; never a stake-pool voter |
| proposing | each proposal | `ParameterChange _ _ (SJust h)` or `TreasuryWithdrawals _ (SJust h)` → the guardrail hash `h` |

`getScriptWitnessConwayTxCert` (`Conway/TxCert.hs:739`): `RegCert _ SNothing` (tag 0) → none;
`RegCert cred (SJust _)` (7), `UnRegCert` (1, 8), `DelegCert` (2, 9, 10), `RegDelegCert` (11, 12, 13)
→ the stake credential's script hash; pool certificates (3, 4) → none; committee authorization /
resignation (14, 15) → the cold credential's; DRep registration / unregistration / update (16, 17, 18)
→ the DRep credential's.

Every one of these is derivable from what Ade already decodes. Spending needs the resolved input
addresses. The other five are UTxO-free and use the same certificate, withdrawal and voter walkers as
`tx_validity::required_signers`, plus the typed `mint` and `proposal_procedures` fields.

Reference scripts (`Babbage/UTxO.hs:134`): the `script_ref` of every output resolved by
`reference_inputs ∪ inputs`. Ade's UTxO keeps each Alonzo+ output's raw bytes (`TxOut::AlonzoPlus`), so
`script_ref` is recoverable. Collateral inputs contribute neither needs nor reference scripts.

### 2.3 Timelocks (`Allegra/Scripts.hs:402-430`)

```
RequireTimeStart  lockStart -> lockStart `lteNegInfty` txStart   -- lteNegInfty _ SNothing = False; i <= j
RequireTimeExpire lockExp   -> txExp `ltePosInfty` lockExp       -- ltePosInfty SNothing _ = False; i <= j
RequireSignature  h         -> h ∈ vhks
RequireMOf m xs             -> isValidMOf m xs                   -- n <= 0 passes (LV-11)
```

`txStart` is the body's `validity_interval_start` (key 8), `txExp` its `ttl` (key 3). A missing start
is −∞ and a missing expiry is +∞, so with no lower bound `invalid_before` never holds, and with no upper
bound `invalid_hereafter` never holds. The block slot plays no part.

### 2.4 The signer set (`Core.hs:510`, `keyHashWitnessesTxWits`)

`vhks` = `{Blake2b-224(vkey)}` over vkey witnesses (key 0) ∪ `{bootstrapWitKeyHash w}` over bootstrap
witnesses (key 2). `bootstrapWitKeyHash` (`Keys/Bootstrap.hs:110`) =
`Blake2b-224(SHA3-256(0x83 0x00 0x82 0x00 0x58 0x40 ‖ vkey(32) ‖ chain_code(32) ‖ attributes))`.
Evaluation uses membership only: a witness's signature is checked by a different rule (§7).

### 2.5 Hashes

Native: `Blake2b-224(0x00 ‖ native_script bytes)`. Plutus V1/V2/V3: `Blake2b-224(0x01/0x02/0x03 ‖
script bytes)`, the bytes being the content of the witness-set (or `script_ref`) bytestring. In a
`script_ref` (`#6.24(bytes .cbor script)`, `script = [0, native_script] / [1..3, bytes]`), the native
script's bytes are its element of the inner `script` array.

## 3. The invariant

**DC-LEDGER-SCRIPT-03.** A Conway transaction's witness-script closure is decided by one authority, shared
by the block path and the standalone path, with two evidence levels:

- **Without full UTxO context** (reduced mode, or any spend or reference input unresolved), the authority
  returns `DefiniteReject` exactly when some witness-set native script fails evaluation, and otherwise
  `Undetermined`. It never returns "valid".
- **With every spend and reference input resolved**, it returns Cardano's exact UTXOW script-closure
  result: the not-validating, extraneous and missing sets of §2.1, each empty for a valid transaction.

Either kind of rejection rejects the transaction, and in the block path the block. Evaluation uses the
transaction's validity interval and the full §2.4 signer set. Every hash is computed from preserved
wire bytes, never from a re-encoding.

### 3.1 The proof obligation for `DefiniteReject`

Claim: if a witness-set native script `s` with hash `h` fails evaluation, Cardano rejects the
transaction, whatever the UTxO holds.

- If `h ∈ neededNonRefs`, then `h` is needed and `s ∈ scriptsProvided`, so `validateFailedBabbageScripts`
  reports `h`.
- Otherwise `h ∈ sReceived − neededNonRefs = extra`, so `babbageMissingScripts` reports it as
  extraneous.

The cases are exhaustive. Evaluation depends only on `s`, the signer set and the validity interval, all
of which are in the transaction. So the rejection is sound, though without the UTxO Ade cannot say which
of the two failures Cardano reports. The verdict type carries that: `DefiniteReject` names the failing
script and the reason, and makes no claim about the Cardano failure class.

A passing witness script proves nothing without the needed set: the transaction may still be missing a
script, or carry this one extraneously. Hence `Undetermined`, never "valid".

## 4. The change

1. **Preserved script bytes and hashes.** The witness decoder records each native script's byte range and
   hashes it, hashes each Plutus witness script, and computes bootstrap-witness key hashes. `ade_crypto`
   gains `sha3_256` (the `sha3` crate, already in `Cargo.lock`).
2. **Timelock evaluation against the validity interval.** `evaluate_native_script` takes a
   `ValidityInterval { invalid_before, invalid_hereafter }` in place of the block slot. The timelock
   failure reasons say which bound failed and carry the transaction's bound. Signature and threshold
   semantics are unchanged (DC-LEDGER-SCRIPT-01, DC-LEDGER-SCRIPT-02 hold).
3. **One authority** (`tx_validity::script_witnesses`). The verdict types make reduced mode unable to
   claim exactness: an exact verdict can be built only from a resolved-context value, and that value can
   be built only when every spend and reference input resolves.
   ```
   NativeScriptVerdict = Reduced(DefiniteReject(ScriptError) | Undetermined)
                       | Exact(ScriptClosureFailures { not_validating, extraneous, missing })
   ```
   The needed set covers all six purposes of §2.2. If any purpose cannot be derived faithfully, the
   slice stops and is re-scoped. No exact path ships over a partial needed set.
4. **Wiring.** The block path (`verify_conway_witness_closure`, which already runs in both modes) and
   `tx_phase_one` call the authority and reject on either rejection. The counter-only evaluation loop in
   `decode_validate_tx_bodies` is removed, so no second verdict path remains.
5. **Order of work.** The authority lands and is censused (§5) before the reject path is wired. Any
   corrected-evaluation failure, or any exact missing or extraneous finding, on the accepted historical
   blocks stops the slice for investigation first.

## 5. The census (before enforcement)

Over every stored block of a copy of the `ade-preprod-lv11` store, in order, the census builds a forward
UTxO from the raw outputs created inside the interval. For a phase-2-invalid transaction it consumes the
collateral and produces the collateral return, not the regular outputs. It records:

- transactions, witness-set native scripts, Plutus witness scripts;
- native scripts failing corrected evaluation (expected 0);
- timelock scripts whose verdict differs between the block-slot and the interval evaluation;
- transactions with an unresolved spend or reference input (reduced verdict only);
- for fully resolved transactions: exact not-validating, extraneous and missing findings (expected 0),
  and reference native scripts evaluated;
- needed-script counts by purpose.

The census is the differential against Cardano. cardano-node accepted every one of these blocks, so any
non-zero finding is an Ade bug until shown otherwise.

## 6. MECHANICAL ACCEPTANCE CRITERIA

- **CE-LV13-1 (accept and reject).** A valid transaction with a needed native script is accepted. The
  same transaction with a failing native script is rejected: `DefiniteReject` without context, an exact
  not-validating finding with context, and a block rejection in the block path. Failing `sig`, `all`,
  `any` and `n-of-k` each reject. `n <= 0` passes exactly as in Cardano.
- **CE-LV13-2 (timelock boundaries).** `invalid_before` at `lock−1 / lock / lock+1` and with no lower
  bound; `invalid_hereafter` at `lock−1 / lock / lock+1` and with no upper bound. Each case is
  cross-checked against §2.3, including one where the block-slot rule and the interval rule disagree.
- **CE-LV13-3 (closure).** With full context: a missing required native script is reported missing. A
  needed script supplied as a reference script is satisfied, and the same script also in the witness set
  is extraneous. A failing reference native script that is needed is reported not-validating.
- **CE-LV13-4 (extra script, two levels).** A passing extra native script with no UTxO context is
  `Undetermined`: neither accepted as valid nor rejected as extraneous. The same transaction with full
  context is exactly `extraneous = {h}`.
- **CE-LV13-5 (needed set).** One case per purpose of §2.2, including certificate tag 0 (no need) versus
  tag 7, a stake-pool voter (no need), and a proposal with and without a guardrail hash.
- **CE-LV13-6 (hashes and signers).** Native and Plutus hashes are over preserved bytes: a
  non-canonically encoded native script hashes its own bytes, not a re-encoding. A real-chain bootstrap
  witness's key hash satisfies a `sig` script.
- **CE-LV13-7 (determinism).** The same transaction gives the same structured verdict and error twice.
  Block-path and standalone-path verdicts are equal on the same transaction.
- **CE-LV13-8 (census).** §5 over the whole stored interval: zero failing native scripts, and zero exact
  extraneous or missing findings.
- **CE-LV13-9 (real fixtures).** A committed real preprod block whose transactions carry native scripts
  (timelock if any exist in the census), with the resolved outputs they spend, gives an exact valid
  verdict. Mutating one byte of the script (via its witness) yields the right rejection.
- **CE-LV13-10 (mutations).** Each of these fails the intended tests and a new gate:
  ignoring the evaluator result; evaluating against the block slot; dropping one purpose from the needed
  set; returning "valid" for a reduced pass; hashing a re-encoding.
- **CE-LV13-11 (no regression).** The CI sweep matches the LV-12 baseline (281 scripts, the same 46
  failing). The LV-11 and LV-12 gates still pass, with their tests adapted to the interval argument.
- **CE-LV13-12 (live).** The LV-12 store restarts on the LV-13 binary with no reset and follows the local
  `cardano-node-preprod` with no failure line. The store-semantics decision follows the census: no bump
  if the stored history is replay-equivalent under the new reject path.

## 7. OUT OF SCOPE

- **Verifying every supplied witness signature** (`validateVerifiedWits`, `InvalidWitnessesUTXOW`). Ade
  verifies only required signers' signatures. Cardano rejects any bad vkey or bootstrap signature. That is
  a separate UTXOW rule, recorded for its own slice. Native-script evaluation uses key-hash membership, as
  Cardano's does.
- **Plutus and state-backed verdicts counted but not enforced in the block path.** Its own slice, next.
- Redeemer, datum and script-integrity checks of UTXOW (`hasExactSetOfRedeemers`, `missingRequiredDatums`,
  `checkScriptIntegrityHash`), and `validateScriptsWellFormed`.
- Pre-Conway eras (Ade is Conway-active).
- Unchecked wire-count preallocation; Plutus-data stack safety; the store-semantics lock list.
- Adding bootstrap key hashes to the required-signer coverage set (`check_required_signers`). That is
  Cardano-correct too, but it is a different check.

## 8. RESULTS

Code: `e6910d5a`. Every CE below passed mechanically.

### Census before enforcement (CE-LV13-8)

`persisted_script_witness_census` (`crates/ade_runtime/tests/body_decode_census.rs`) ran the authority, not
yet wired to reject, over every block of two store copies. It built a forward UTxO from the outputs created
inside the interval; a phase-2-invalid transaction consumes its collateral and produces its collateral return.

| | stopped LV-8 store | halted LV-12 live store |
|---|---|---|
| slots | 129,813,444..=133,902,215 | 129,813,444..=137,096,367 |
| blocks / transactions | 184,490 / 421,827 | 326,471 / 991,602 |
| witness native scripts (all passing, corrected evaluator) | 36,069 | 46,866 |
| of which with a timelock / verdict changed vs block slot | 28,758 / 0 | 33,567 / 0 |
| witness Plutus scripts | 149,425 | 1,241,984 |
| txs with bootstrap witnesses | 142 | 272 |
| reduced definite rejects | 0 | 0 |
| fully resolved txs / exactly valid | 324,200 / 324,200 | 866,432 / 866,432 |
| exact not-validating / extraneous / missing | 0 / 0 / 0 | 0 / 0 / 0 |
| needed reference native / Plutus scripts | 2 / 16,021 | 2 / 108,079 |
| needed: spending, rewarding, certifying, minting, voting, proposing | 65,859; 3,011; 719; 71,282; 127; 1 | 153,028; 31,059; 1,939; 1,161,025; 221; 3 |

Zero findings. The exact level's zero missing and zero extraneous findings, over 1.24M Plutus witness
scripts and every needed category, are also the check that Ade's hashes and needed set agree with what
cardano-node accepted. No timelock in the interval sits where the block-slot rule and the interval rule
disagree, so the correction changes no historical verdict; the boundaries are pinned by constructed cases
(CE-LV13-2).

### CE-LV13-1..7, 9

`crates/ade_ledger/tests/lv13_native_script_witness_verdict.rs`, 22 tests, plus the evaluator's interval
tests in `scripts.rs` and `sha3_256`'s FIPS 202 vectors. Three committed real preprod fixtures, each with the
outputs its inputs resolve to, as the census dumped them:

| fixture | what it carries | exact verdict |
|---|---|---|
| `lv13_timelock_block.cbor`, slot 129,919,360 tx 0 | a needed timelocked minting script in the witness set | valid |
| `lv13_refnative_block.cbor`, slot 133,860,142 tx 0 | a script-address spend satisfied by a reference native multisig (3-of-160) | valid |
| `lv13_bootstrap_block.cbor`, slot 129,817,130 tx 1 | a bootstrap witness spending a Byron input | its key hash equals the address root |

The timelock fixture, with its lock moved out of reach (same byte length), is rejected by the block path
(`apply_block_with_verdicts`, reduced) and by `tx_validity` with the same `NativeScriptFailed`, twice
identically; with the UTxO, the changed bytes are exactly one missing and one extraneous hash. Dropping the
reference multisig's signers is exactly not-validating.

### CE-LV13-10 (mutations)

Each applied, compiled, failed its intended test and the gate, and was restored byte-exact:

| mutation | intended test | gate |
|---|---|---|
| ignore the evaluator's result | `..._failing_native_script_is_rejected_at_both_levels` | suite fails |
| an open interval end satisfies a timelock (block-slot behaviour) | `..._invalid_before_holds_from_the_lock_and_never_without_a_start` | suite fails |
| drop the Proposing purpose | `..._needed_set_covers_every_conway_purpose` | also names the purpose |
| report a reduced pass as exactly valid | `..._a_needed_passing_native_script_is_accepted` | also names the exact-verdict rule |
| hash a re-serialization of the parsed script | `..._a_real_timelock_mint_is_exactly_valid` | also names the wire-bytes rule |

### CE-LV13-11 (no regression)

Sweep: 282 scripts. Against the LV-12 baseline (281, the same 46 failing) the first run added two, both the
store-semantics lock: `rules.rs` and `phase.rs` are on its surface. Reconciled as below, both pass, and the
failing set is the baseline's 46. The LV-11 and LV-12 gates pass with their tests on the interval argument
(the LV-12 recursive reference now uses Cardano's interval semantics, over interval grids instead of slots).

### Store semantics

Not bumped; the lock records a `semantics_neutral = true` entry at version 11. The closure only accepts or
rejects: on an accepted block every authoritative output is computed as before, and the one changed value,
`BlockVerdict.native_script_passed`, is not persisted or read by any rule. A store from the previous binary
is replay-equivalent unless it holds a block the new reject path refuses, and the census shows neither store
does.

### CE-LV13-12 (live)

The LV-12 node on `ade-preprod-lv11` halted cleanly at durable tip 137,096,367; that store is the second
census column above. The `e6910d5a` binary restarted on it (`forward_fold_no_reset`) and was at the peer tip
(137,099,113) three minutes later, with no failure line:
`docs/evidence/run-stores/preprod-live2c/lv13-ce12-restart-on-witness-verdicts.md`.

### What LV-13 does not close

LV-13 closes the UTxO-free part of native-script enforcement, not every UTXOW script obligation. Both paths
continue on `Undetermined`. So a reduced node, which is what the live preprod follower is, still accepts a
block whose transaction is missing a required script or carries a passing but extraneous one, or whose
needed reference native script fails: each needs the needed set or the reference scripts, so each needs the
UTxO. The function's verdict is not wrong there; the node's acceptance is incomplete. Exactness on the
authoritative live block path belongs to the block-path state-backed validity work, which also needs the
sequential intra-block UTxO (a transaction spending an output created earlier in the same block).

### Found on the way, not fixed here

- Witness signatures are verified only for required signers (`verify_required_witnesses`); Cardano's
  `validateVerifiedWits` verifies every vkey and bootstrap witness. A native script's `sig` is satisfied by
  key-hash membership, as in Cardano, so a bad signature on a non-required witness is a gap of that rule.
- A transaction spending an output created earlier in the same block gets the reduced level in the block
  path, which resolves against the pre-block UTxO. Weaker, never wrong.
