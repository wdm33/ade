# CENSUS LV-15b — which protocol parameters admission may trust, and when

**Cluster:** PREPROD-LIVE-2-FORGE-READINESS. **Follows:** CENSUS-LV-15 §7 (decision (a)), SLICE-LV-15a §1.
**Kind:** findings only. No code. The owner set four questions and one prohibition before any implementation:

1. What exactly is the authoritative parameter state?
2. How is parameter kind preserved?
3. When does admission observe enactments?
4. What happens to v11 stores?

Prohibition: admission must not maintain a second independently evolving parameter copy.

Reference: cardano-ledger cb57dc73. Ade: `e51da2f4`.

## 1. Where Ade's parameters live today

There are four copies. Only one of them evolves.

| copy | evolves? | persisted by | min-UTxO kind after restart |
|---|---|---|---|
| `EpochAccumulator.protocol_params` | yes: Conway enactment at each crossed boundary (`governance::plan_conway_governance_epoch` → `build_enacted_pparams`) | `encode_epoch_accumulator` → `snapshot::gov_state::encode_pparams` | lost (`decode_pparams` always rebuilds `LegacyAbsoluteMin`) |
| admission's `LedgerState.protocol_params` | no: set at bootstrap, carried unchanged by `apply_reduced_epoch_boundary` | ledger snapshot (`snapshot/ledger.rs` → the same `encode_pparams`), framing schema 4 | lost |
| forge constants (`node_lifecycle::forge_constants_from_pparams`) | no: a clone of admission's copy | — | — |
| `consensus_inputs::parse_protocol_parameters_json` (the cardano-cli seed path) | no | — | a placeholder `LegacyAbsoluteMin(0)`, by its own comment not authoritative |

The fingerprint (`fingerprint::fingerprint_pparams`) binds 17 fields plus the Conway deposits. It binds the min-UTxO coin but not its kind. It does not bind the transaction or block execution-unit limits, collateral percent or cost models.

So the forge also reads the stale copy. Today that is harmless only for one reason. Ade's enactment supports just three `protocol_param_update` keys (the update map numbers fields differently from the `curPParams` indices used below): 16 `minPoolCost`, 20 `maxTxExUnits` and 21 `maxBlockExUnits` (memory only). Any ratified change to another key, or any non-`ParameterChange` action, halts the boundary with no state change (`GovernanceTerminal::UnsupportedRatifiedAction`). So every other field is still its bootstrap value whenever Ade is running at all. The stale copy disagrees with the chain only on those three keys, and preprod has enacted exactly that (`e641ec80…#0`, 304→305).

## 2. Question 1: the authoritative parameter state

### What the parameter-dependent admission rules read

| Cardano check | Conway `curPParams` index | in `ProtocolParameters`? |
|---|---|---|
| `validateMaxTxSizeUTxO` | 3 `maxTxSize` | yes |
| `validateOutputTooSmallUTxO` | 14 `coinsPerUTxOByte` (per byte) | coin yes; kind lost on restart |
| `validateOutputTooBigUTxO` | 19 `maxValSize` | **no** (skipped at bootstrap) |
| `ExUnitsTooBigUTxO` | 17 `maxTxExUnits` | yes |
| `TooManyCollateralInputs` | 21 `maxCollateralInputs` | **no** (skipped at bootstrap) |
| BBODY total ex-units | 18 `maxBlockExUnits` | yes (`MaxBlockExUnits::Bound`) |
| `chainChecks` header size, body size | 4 `maxBHSize`, 2 `maxBBSize` | yes |
| `chainChecks` PV ≤ node max; BBODY `HeaderProtVerTooHigh`; `disjointRefInputs` (8 < PV < 11) | 12 `protocolVersion` | yes |

### What the later slices will read

These rules need the UTxO, so they belong to the full-UTxO slice and the cutover, not LV-15b. They are listed because persisting only LV-15b's fields would force a second store bump later:

| later rule | index | held? |
|---|---|---|
| min fee | 0, 1 | yes |
| min fee, reference-script part | 30 `minFeeRefScriptCostPerByte` | **no** |
| script fee and collateral | 16 `prices` | **no** |
| collateral percent | 20 | yes |
| Plutus evaluation | 15 `costModels` | yes, as raw bytes |

### Finding

The authoritative state should be the complete Conway `curPParams`: all 31 fields, typed, and encoded canonically in one versioned record. That means no subset chosen by the current slice. Fields the governance state already owns keep their single home and are not copied:
- voting thresholds (22, 23);
- `govActionLifetime` (26);
- the three Conway deposits (27–29), which live in `ConwayOnlyDepositParams`.

The record must bind every field into the pparams fingerprint/commitment as well, so two states that differ only in a parameter cannot share a fingerprint.

## 3. Question 2: the min-UTxO kind

The kind is semantic. Per byte means `(160 + output size) × c`. An absolute floor means `c`. Read as a floor, Conway's 4,310 would accept almost every output.

Today the kind survives only in memory, from bootstrap until the first restart:
- `encode_pparams` writes the coin;
- `decode_pparams` rebuilds `LegacyAbsoluteMin`;
- the fingerprint writes the coin, and the re-encode check passes because the bytes are identical.

So a node that never restarted and a node that did hold different in-memory parameters behind identical bytes and an identical fingerprint. No rule reads the field today, which is the only reason this has not shown up.

### Finding

- The record encodes the kind explicitly, as a tag before the coin, and the fingerprint binds the tag too.
- Decode accepts only the tags it knows. A Conway accumulator whose min-UTxO rule is not `PerByte` is refused rather than read.
- Nothing infers the kind from the era, and nothing falls back after a restart.
- The same applies to the admission ledger snapshot, unless the change below removes admission's own copy.

## 4. Question 3: when admission observes enactments

### Cardano

In Conway, `curPParams` changes only in EPOCH (`Conway/Rules/Epoch.hs`: `cgsCurPParamsL .~ nextEpochPParams govState0`). Conway has no within-epoch parameter path. TICK runs before BBODY. So the parameters for block B are those in force after the tick into B's epoch. They are constant for the whole epoch, and that includes B when it is the first block of the epoch. `chainChecks` and the header protocol-version check read the same ticked view.

### Ade today

The live order, in `node_lifecycle` around the relay loop:
1. `run_node_sync` admits blocks durably. At a boundary promotion it yields after the first block of the new epoch has been admitted.
2. Only then does `advance_ledger_state_to_durable_tip_memo` walk the accumulator to the durable tip. That walk crosses the boundary inside `apply_selected_block` on that first block.

So when the first block of epoch e+1 is admitted, the accumulator is still in epoch e. Its parameters are the pre-enactment ones. **Copying "the accumulator's latest parameters" into admission would be one transition behind for exactly the first block of every epoch.** It could also be further behind during catch-up, because the accumulator walks after each pass, not after each block.

The accumulator is also observe-only for following today. A stall or a governance terminal leaves the follow running. If admission depends on it, a stalled boundary must stop admission of the new epoch rather than let admission guess. That is the honest consequence of a single authority, but it changes what an accumulator stall means operationally.

### Finding: the binding

Admission's parameter view for block B is defined as **the accumulator's parameters after crossing into `epoch(B)`**:
- **Same epoch.** If the accumulator is already in `epoch(B)` (its last applied slot is before B), its parameters are exact, because nothing changes them within an epoch.
- **Boundary not yet crossed.** If the accumulator is in an earlier epoch, admission must not proceed on a guess. Of the options considered, (i) is the one that keeps one authority:
  - **(i) Tick before body.** Split the accumulator transition into its boundary half and its block half. The boundary half (`cross_epoch_boundary`) already reads only the prior state and the context (epoch, the mark from the reduced checkpoint at the prior tip). It never reads B's body. The shell runs the boundary half to `epoch(B)` against the durable prior tip, admission reads the crossed parameters, and then the block half applies B. This is Cardano's TICK-then-BBODY order, with one transition function and one result.
  - (ii) Admission recomputes the enactment itself from the prior accumulator state. That runs the boundary twice and can drift, so it is rejected.
  - (iii) Fail closed on the first block of each epoch until the accumulator crosses. That cannot work, because the accumulator crosses only on that block.
- **Accumulator ahead of `epoch(B)`.** Possible only after a rollback across a boundary that the settled rewind has not yet undone. The view is refused, never read backwards.

The view is a value the accumulator produces. It is not a copy admission keeps. Admission holds no parameter field of its own after this change. The admission ledger's `protocol_params` and the forge constants are both replaced by reads of the bound view. That is what satisfies the prohibition.

### Open: Ade's ratification timing

Cardano enacts at e→e+1 what its DRep pulser ratified during epoch e, using the proposals, votes and distributions captured when that pulser started (`setFreshDRepPulsingState` at the previous boundary; `extractDRepPulsingState` here). Ade's `plan_conway_governance_epoch` ratifies at the boundary itself, over the state current at the boundary.

Two real cases agree with the chain:
- preview `69c948cd…#0`, ratified at 1095 and not at 1094, against the oracle (CRE S4.4 anchor);
- preprod `e641ec80…#0` at 304→305.

Neither proves the two timings agree when a vote that tips a threshold arrives during the ending epoch. This is an accumulator question rather than an admission one. But LV-15b makes admission inherit the answer, so the slice must settle it against the reference first: prove the timings equal on the supported subset, or find the case where they differ. It is not recorded as a defect.

## 5. Question 4: v11 stores

A v11 store cannot reconstruct the new state:
- `maxValSize`, `maxCollateralInputs`, `prices` and `minFeeRefScriptCostPerByte` were never decoded or stored;
- the min-UTxO kind was erased on its first restart.

The values could be guessed from today's preprod parameters, but that is a fabricated default for consensus state, and the project has refused that each time (v3→v4, the DormantEpochs and MaxBlockExUnits versioning).

### Finding

Store semantics 11 → 12, with the accumulator schema bumped as well. If admission's ledger snapshot still carries parameters, the framing schema is bumped too. Otherwise it drops them.

A v11 store is refused with a typed error naming the missing parameter authority, and is never started reduced. Recovery is a fresh Mithril bootstrap, which decodes the full `curPParams` from the certified snapshot. The bootstrap decoder (`ledgerdb_state::read_conway_pparams`) already walks every index. It stops skipping the ones it now drops:
- 16 `prices`;
- 19 `maxValSize`;
- 21 `maxCollateralInputs`;
- 24 `committeeMinSize` and 25 `committeeMaxTermLength`;
- 30 `minFeeRefScriptCostPerByte`.

Bootstrap, WAL/checkpoint identity, recovery and the compatibility check all bind the new record through the fingerprint change in §2.

## 6. What LV-15b would then be

This section is a proposal for the owner to decide, not a plan of record. LV-15b becomes two slices, because the first changes durable state and the second only rejects.

- **LV-15b-1, parameter authority (state, store 12):**
  - the complete typed Conway parameter record, with explicit min-UTxO kind and full fingerprint binding;
  - bootstrap decodes every field;
  - the accumulator splits into tick and block halves, and the shell ticks before admitting the first block of an epoch;
  - admission and the forge read the bound view and keep no copy;
  - v11 stores are refused with a typed error;
  - the ratification-timing question in §4 is settled first.
  - Behaviour is otherwise unchanged. Proof: a byte-identical accumulator over the live store's blocks with and without the split, and a restart that recovers the identical record.
- **LV-15b-2, the parameter-dependent rejection rules:**
  - max tx size; output too small and too big; collateral count; tx and block ex-units;
  - header and body size limits; header protocol version; reference-input disjointness;
  - census before enforcement, with each rule reading only the bound view.

## 7. Facts this census relied on, and where

- Enactment subset and terminals: `governance.rs`, `UnsupportedActionKind`, `decode_enactable_param_update`, `build_enacted_pparams`.
- Kind erasure: `snapshot/gov_state.rs` `encode_pparams` / `decode_pparams`; `fingerprint.rs` `fingerprint_pparams`.
- Skipped indices: `ledgerdb_state.rs` `read_conway_pparams` (16 prices, 19 maxValSize, 21..30 except thresholds, lifetime and deposits).
- Live order: `node_lifecycle.rs` relay loop: `run_node_sync`, then `advance_ledger_state_to_durable_tip_memo`. Accumulator order: `epoch_accumulator.rs` `apply_selected_block_core`, where boundaries run before `apply_within_epoch`.
- Cardano timing: `Conway/Rules/Epoch.hs` (curPParams set only in EPOCH; ratification extracted from the pulser).

## 8. Ratification timing, settled (owner decision: before any LV-15b-1 production code)

The owner upgraded §4's open question to a derived compatibility proof obligation, using the late-vote case.

### Oracle

cardano-ledger cb57dc73 `eras/conway/impl/testlib/Test/Cardano/Ledger/Conway/Imp/EpochSpec.hs`, "proposal is accepted after two epochs". This is the Haskell transition run on exactly that case:
1. A ParameterChange is submitted, with no votes, and an epoch passes.
2. The committee, DRep and SPO vote Yes during epoch E. `isDRepAccepted` is now true over the current state.
3. `passEpoch` (E→E+1): `getParamValue shouldReturn initialParamValue`, so the parameter is **unchanged**.
4. `passEpoch` (E+1→E+2): the parameter **is** the proposed value.

The test was not executed here, because there is no Haskell toolchain on this machine. Its assertions are the reference's own result at the pinned commit. The source explains why:
- the E→E+1 boundary applies `extractDRepPulsingState`, the result of the pulser seeded at the start of E;
- it applies that result to the current proposals, which keep the newer votes (`proposalsApplyEnactment`, with the comment in `Conway/Rules/Epoch.hs`);
- it then seeds a fresh pulser (`setFreshDRepPulsingState`) that sees those votes.

### Ade

`epoch_accumulator::tests::lv15b_a_vote_cast_during_epoch_e_enacts_at_e_plus_1_in_ade_but_e_plus_2_in_cardano` runs the same case through `apply_epoch_boundary_with_registrations`, the boundary both the accumulator and replay use:
1. **1340→1341:** nothing enacts and the proposal is carried.
2. **During 1341:** two committee Yes votes are added.
3. **1341→1342:** Ade enacts (`maxTxExUnits` mem becomes 16,500,000) and removes the proposal.

It passes today, which means **Ade diverges: it enacts one epoch early**. The test records the current behaviour. LV-15b-1 must invert its 1342 assertions and add the E+1→E+2 enactment.

### What the divergence covers

The pulser freezes every input to ratification at the start of E (`Conway/Governance.hs` `setFreshDRepPulsingState`). Ade reads the version current at E→E+1 for each of them (`rules.rs`, the `ConwayGovernanceEpochInput` build):

| pulser snapshot field (taken at E start) | Ade reads at E→E+1 |
|---|---|
| `dpProposals` (proposals and votes) | current `gov.proposals` |
| `dpAccounts` (DRep delegations), `dpInstantStake` | current `vote_delegations` over the pre-boundary mark |
| `dpStakePoolDistr` = the mark pool distribution built at E (`ssStakeMarkPoolDistr snapshots1`) | the pre-boundary **go** pool stakes, which come from the snapshot built at E−2 |
| `dpDRepState` (expiry) | current `drep_expiry` |
| `dpCommitteeState` (hot keys) | current `committee_hot_keys` |
| `dpEnactState` (pparams, committee, constitution, roots, treasury) | current values |
| `dpCurrentEpoch` = E | `new_epoch − 1` = E (this one agrees) |

The SPO-basis row was already recorded as deferred in CRE S3: "DRep-mark-vs-SPO-go basis → S6". The vote-timing row is new.

The divergence is live-reachable today, independent of LV-15b. Any supported ParameterChange whose deciding vote arrives during the ending epoch enacts a boundary early. With it go one epoch early:
- the deposit refund;
- the proposal removal, and with it the reward-account balance, and so the stake.

The preprod (`e641ec80…#0`) and preview (`69c948cd…#0`) matches are consistent with their votes having landed before the ratifying epoch began. That was not checked against the chain.

### Consequence for LV-15b-1

The invariant "before admitting the first block of E, Ade has executed exactly the Cardano-compatible boundary transition" cannot hold while the boundary ratifies over boundary-current state. Making it hold needs a new piece of durable accumulator state, the ratification snapshot taken at each boundary, plus a bootstrap import of the snapshot's DRep pulsing state (`ConwayGovState` index 6). The bootstrap decoder skips that index today.

This exceeds what §6 scoped for LV-15b-1. It is put to the owner as a scope decision, not started.
