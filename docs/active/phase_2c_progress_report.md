# Phase 2 Complete Progress Report — Detailed Technical State

**Date**: 2026-03-28
**Commit**: `7a5e8e7` (main)
**Branch**: main

---

## 0. Phase 2 Overview

Phase 2 covers all ledger rules from cryptographic verification through epoch boundary transitions:

| Sub-phase | Scope | Status |
|---|---|---|
| Phase 2A | Cryptographic verification (Blake2b, Ed25519, VRF, KES) | **Complete** (closed 2026-03-16) |
| Phase 2B | Ledger rules Phase 1 — Byron through Mary validation | **Complete** (verdict replay across 6,000 blocks) |
| Phase 2C | Ledger rules Phase 2 — Alonzo–Conway structural + epoch + HFC | **In progress** |

### Phase 2A — Cryptographic Verification (COMPLETE)

**Delivered**: Populated `ade_crypto` BLUE crate with pure cryptographic verification:
- Blake2b-256/224 hashing (5 golden vectors, cross-validated with Python hashlib)
- Ed25519 signature verification (standard + Byron extended, 5 libsodium vectors)
- ECVRF-ED25519-SHA512-Elligator2 VRF proof verification (pure Rust via cardano-crypto)
- Sum6KES signature verification (depth 6, 64 periods)
- Operational certificate verification

**Key files**: `crates/ade_crypto/src/{blake2b,ed25519,vrf,kes,error,traits}.rs`
**Tests**: 52 unit tests in ade_crypto
**Dependencies added**: blake2 0.10, ed25519-dalek 2, cardano-crypto 1.0
**Closing report**: `docs/completed/phase_2a/phase_2a_closing_report.md`

### Phase 2B — Byron through Mary Ledger Rules (COMPLETE)

**Delivered**: Full verdict replay across Byron, Shelley, Allegra, and Mary eras:

**Byron validation** (`crates/ade_ledger/src/byron.rs`):
- UTxO model: insert/delete/lookup with BTreeMap
- Conservation law: consumed == produced (with fee exception)
- Double-spend rejection
- Bootstrap witness verification (Ed25519 extended keys)
- Fee validation (min_fee_a × size + min_fee_b)
- 1,500/1,500 Byron blocks accepted with verdict agreement
- UTxO equality: 14,505/14,505 entries match oracle at genesis and block 1

**Shelley/Allegra/Mary validation** (`crates/ade_ledger/src/{shelley,mary}.rs`):
- Shelley bootstrap: 84,609 UTxO entries loaded from oracle ExtLedgerState dump
- Key witness verification (Ed25519)
- Validity interval enforcement (TTL, validity_interval_start)
- Native script evaluation (Sig, All, Any, MOfN, timelocks)
- Multi-asset value support (Mary)
- Certificate parsing and delegation tracking
- 4,500/4,500 blocks accepted across Shelley/Allegra/Mary

**Infrastructure**:
- 7-crate workspace: ade_codec, ade_types, ade_crypto, ade_core, ade_ledger, ade_testkit, ade_runtime
- BLUE/RED separation enforced by CI (`ci_check_dependency_boundary.sh`)
- Forbidden patterns in BLUE crates (`ci_check_forbidden_patterns.sh`)
- Contiguous corpus: 6,000 blocks with oracle state hashes
- Differential replay harness with verdict agreement testing
- Oracle manifest and provenance tracking

**Tests at Phase 2B completion**: 482 passing
**Closing report**: `docs/completed/phase_2c/phase_2c_closing_report.md` (covers the contiguous corpus extension)

---

## 1. What Phase 2C Is

Phase 2C completes the remaining Phase 2 ledger rules scope:
- T-24: Alonzo/Babbage/Conway structural transaction validation
- T-25: Epoch boundary transitions (Shelley through Conway)
- T-26: HFC ledger-side era translation functions

The cluster plan is at `docs/active/cluster_plan.md` with slice specs in `docs/active/T-24_*.md`, `T-25_*.md`, `T-26_*.md`.

---

## 2. What Has Been Completed

### T-24A — Late-Era Structural Classification (CLOSED)

**What it does**: Replaces opaque `raw: Vec<u8>` tx body types with parsed field-level structures for Alonzo (14 fields), Babbage (17 fields), and Conway (21 fields).

**Key files**:
- `crates/ade_types/src/{alonzo,babbage,conway}/tx.rs` — parsed tx body types
- `crates/ade_codec/src/{alonzo,babbage,conway}/tx.rs` — CBOR decoders
- `crates/ade_ledger/src/{alonzo,babbage,conway}.rs` — structural validation
- `crates/ade_ledger/src/scripts.rs` — `ScriptPosture` enum (NoScripts / NonPlutusScriptsOnly / PlutusPresentDeferred)
- `crates/ade_ledger/src/rules.rs` — `BlockVerdict` with per-tx classification

**Evidence**: 10,500 blocks / 80,935 transactions classified. 71,499 non-Plutus, 9,436 Plutus-deferred. Zero corpus rejects.

### T-24B — Witness Binding + Native Script Evaluation (CLOSED)

**What it does**: Parses witness sets to detect Plutus V1/V2/V3 scripts. Evaluates native scripts via the existing `evaluate_native_script` function. Structural validation at correct authority boundary (only `EmptyInputs` rejected — other checks like `EmptyOutputs`, `ZeroFee`, `ZeroCoinOutput` are state-backed ledger rules, not structural).

**Key files**:
- `crates/ade_codec/src/allegra/script.rs` — NativeScript CBOR decoder
- `crates/ade_ledger/src/witness.rs` — `WitnessInfo` classifier (detects VKeys, native scripts, Plutus V1/V2/V3)
- `crates/ade_ledger/src/error.rs` — `StructuralError` with 13 failure reason variants

**Evidence**: Conway `tag(258)` set encoding handled. Witness-confirmed Plutus detection matches body-only heuristic across full corpus.

### T-26A — HFC Translation Functions (IMPLEMENTATION CLOSED, CE-73 OPEN)

**What it does**: All 6 HFC translation functions implemented as pure deterministic functions. Unified `translate_era` dispatch. Full Byron→Conway chain tested.

**Key files**:
- `crates/ade_ledger/src/hfc.rs` — 6 translation functions + dispatch + 26 unit tests

**Evidence**:
- 22/22 encoding-independent fields match oracle at Shelley→Allegra boundary (era, epoch, treasury, reserves, UTxO count, all 17 protocol parameter values including rational numerators/denominators)
- Translation logic is proven correct at all compared surfaces
- CE-73 remains open: requires full oracle state-hash equality, which is blocked on CBOR encoding surface mismatch (not translation logic)

**CE-73 gap diagnosis**: The oracle uses Haskell's `serialise` library with non-canonical CBOR choices (mixed integer widths, mixed definite/indefinite lengths). Our `Rational` normalization is already correct (GCD reduction). The remaining gap is serializer behavior, not ledger semantics. Decision: do not implement full Haskell disk re-encoding unless proven necessary. This is a policy decision for the planners.

### T-21B — State-Load Bridge (CLOSED)

**What it does**: Parses the oracle's compact on-disk ExtLedgerState CBOR to load delegation, pool, and snapshot state into the Ade `LedgerState`.

**Key files**:
- `crates/ade_testkit/src/harness/snapshot_loader.rs` — tarball extraction, CBOR header parsing, delegation/pool/snapshot loading, block production parsing, fee parsing

**Data loaded from oracle (Allegra epoch 237 snapshot)**:
- 98,331 delegations (credential → pool hash)
- 1,445 pools with params (pledge, cost, margin, reward account)
- 96,260 stake distribution entries (credential → lovelace)
- 20.4 billion ADA total delegated stake
- 611 block-producing pools from `nesBprev`
- Epoch fees from SnapShots[3]

**Count chain verified**: raw CBOR → stored in LedgerState → visible at epoch boundary. All counts preserved exactly.

**Known limitation**: On-disk UTxO map uses position-based compact keys `(slot, tx_in_block, output_index)` instead of TxId hashes. Cannot populate `BTreeMap<TxIn, TxOut>` without full chain history. UTxO tracking uses output production during replay instead.

### T-25A — Epoch Boundary Transitions (CE-71 REWARD FORMULA CLOSED)

**What it does**: Full epoch boundary pipeline: detect epoch transition → compute rewards from go snapshot → rotate snapshots → retire pools → update treasury/reserves. Four-flow accounting separates reward distribution from MIR.

**Key files**:
- `crates/ade_ledger/src/state.rs` — `EpochState` with snapshots, reserves, treasury, block_production, epoch_fees. Epoch detection via `slot_to_epoch` / `detect_epoch_transition`.
- `crates/ade_ledger/src/rules.rs` — `apply_epoch_boundary_full` (rewards before rotation, correct ordering). `EpochBoundaryAccounting` with four-flow decomposition (reward + MIR buckets).
- `crates/ade_ledger/src/rational.rs` — BigInt-backed arbitrary-precision Rational (num-bigint). Matches Haskell's Integer precision.
- `crates/ade_ledger/src/epoch.rs` — `rotate_snapshots`, `compute_total_reward`, `compute_pool_reward`, `apply_epoch_boundary` orchestration
- `crates/ade_ledger/src/delegation.rs` — `CertState`, `DelegationState`, `PoolState`, `apply_cert`
- `crates/ade_codec/src/shelley/cert.rs` — Certificate CBOR decoder (6,354 certs decoded across 7 eras)

**Shelley reward formula (matches Haskell cardano-ledger exactly)**:
```
eta = min(1, blocksMade / expectedBlocks)  [when d < 0.8]
deltaR1 = floor(eta × rho × reserves)
total_reward = deltaR1 + epoch_fees
deltaT1 = floor(total_reward × tau)
pool_reward_pot = total_reward - deltaT1
sigma' = min(pool_stake/total_stake, 1/k)
s' = min(pledge/total_stake, 1/k)
bracket = sigma' + s' × a0 × (sigma' - s'×(z-sigma')/z)
maxPool = floor(R/(1+a0) × bracket)
f = floor(maxPool × apparentPerformance)
leaderRew = c + floor((f-c) × (m + (1-m) × s_op/σ))   [operator excluded from member loop]
memberRew(t) = floor((f-c) × (1-m) × t/σ)              [non-operator members only]
deltaR2 = pool_pot - sum_rewards                         [undistributed → reserves]
deltaT2 = sum(rewards to unregistered credentials)       [→ treasury]
```

**Per-pool formula comparison (Allegra epoch 236→237)**:

| Metric | Value |
|---|---|
| Haskell-formula total | 12,816,444,600,670 lovelace |
| Ade-formula total | 12,816,444,600,670 lovelace |
| **Per-pool delta** | **0 lovelace across 617 pools** |
| Pools compared | 617 producing (1,400 in go snapshot) |
| Delegators processed | 94,409 |
| Total stake | 20,218,109,006,931,644 lovelace (20.2B ADA) |

**Four-flow epoch boundary decomposition (Allegra 236→237)**:

| Flow | Reserves effect | Treasury effect |
|---|---|---|
| Reward distribution | −20,357,958,214,532 | +7,563,628,272,023 |
| MIR reserves→treasury | −170,076,120,225 | +170,076,120,225 |
| MIR reserves→accounts | −15,198,411,257 | 0 |
| **Total (predicted)** | **−20,543,232,746,014** | **+7,733,704,392,248** |
| **Oracle (actual)** | **−20,543,232,746,014** | **+7,733,704,392,248** |
| **Prediction error** | **0** | **0** |

The previous 921 ADA "gap" was a false divergence created by an accounting identity that conflated reward distribution with MIR-to-accounts. The reward formula itself has zero divergence.

**Protocol parameters from oracle (not genesis defaults)**:
- n_opt = 500 (genesis default was 150)
- a0 (pool_influence) = 3/10
- rho (monetary_expansion) = 3/1000
- tau (treasury_growth) = 1/5
- decentralization = 8/25 at epoch 236

---

## 3. Infrastructure

### Corpus Data Available

| Data | All 7 Eras |
|---|---|
| Contiguous blocks | 1,500 per era (10,500 total) |
| State hashes | 1,500 per era |
| Golden blocks | 3-15 per era |
| ExtLedgerState dumps | 1-2 per era |
| Genesis files | Byron, Shelley, Alonzo, Conway |
| Boundary blocks | 12 sets (6 HFC + 6 epoch), re-extracted at correct slots |
| Snapshots | 23 tarballs (12 proof-grade + 11 coverage fills), 23GB |
| Oracle hash files | 5 hash files at epoch boundary points |

### Anchor Hash Chain (12 proof-grade snapshots)

All 12 snapshots verified: tarball extraction → CBOR header parsing → anchor hash computation (Blake2b-256 of raw state bytes). Telescope progression 2→7 confirmed.

### Sub-State Summaries

Oracle sub-state values extracted at each HFC boundary: epoch, treasury, reserves, UTxO count, pool count, delegation count, block producer count, mark/set/go snapshot sizes. Stored in `corpus/snapshots/sub_state_summaries.toml` and `corpus/snapshots/protocol_params_oracle.toml`.

### Test Infrastructure

- `structural_classification_report.rs` — 80,935 tx classification across 7 eras
- `boundary_anchor_hashes.rs` — 12 anchor hashes verified
- `boundary_replay.rs` — 240 blocks across 12 boundaries
- `boundary_stateful_replay.rs` — UTxO + cert tracking through boundaries
- `epoch_boundary_logic.rs` — epoch transition detection + reward verification
- `epoch_oracle_comparison.rs` — four-flow decomposition, per-pool formula comparison, MIR root cause proof
- `translation_summary_proof.rs` — 22/22 field match for Shelley→Allegra
- `translation_comparison_surface.rs` — oracle sub-state preservation
- `transition_proof_surface.rs` — end-to-end HFC transition diagnostic
- `certificate_decode.rs` — 6,354 certs across 7 eras

---

## 4. What Is Still Open

### CE-71 (Epoch Boundary Oracle Equivalence)
- Per-pool reward formula: **0 lovelace delta** across 617 pools (BigInt Rational + Haskell-exact leader/member split)
- Four-flow decomposition: **0 prediction error**
- MIR modeled as separate typed fields in `EpochBoundaryAccounting`
- **Status: CLOSED** for reward distribution.

### CE-72 (Conway Epoch Boundary)

**Reward formula slice — PROVEN:**

Reward-formula correctness proven on the authoritative PREALL comparison surface (correct epoch data matching the oracle's actual inputs):

| Boundary | PV | Ratio | Residual |
|----------|-----|-------|----------|
| Allegra 236→237 | 3 | 100.000% | 0 (activeStake + MIR, separate formula) |
| Alonzo 310→311 | 6 | 100.0008% | 164 ADA (see residual analysis below) |
| Babbage 406→407 | 8 | 100.0000% | 0 (exact) |
| Conway 528→529 | 9 | 100.0000% | 0 (exact) |

Formula (confirmed from Haskell source — PulsingReward.hs, Rewards.hs):
- PV < 4: `totalStake = activeStake`
- PV 4+: `sigma = poolStake / circulation` (bracket), `sigmaA = poolStake / totalActiveStake` (performance)
- `apparentPerformance = beta / sigmaA` — unbounded, NOT capped at 1.0
- `poolReward = floor(appPerf * maxPool)` — can exceed maxPool
- PV ≤ 6: `hardforkBabbageForgoRewardPrefilter` is inactive — leader/member rewards only distributed to registered accounts (DState UMap entries with non-null RDPair)
- PV > 6: pre-filter skipped, all rewards distributed regardless of registration

Root cause of earlier 2-4% gaps: (1) used circulation for both sigma and sigmaA, (2) incorrectly capped performance at 1.0. Both fixed.

**Alonzo 164 ADA residual — root cause analysis:**

The Alonzo residual (100.0008%, 164 ADA on a 20.4B ADA boundary) comes from the leader/member reward pre-filter at PV ≤ 6. The exact mechanism:

1. At PV ≤ 6, the Haskell only distributes rewards to **registered** accounts. The registration set = DState UMap entries with non-null RDPair (reward+deposit pair). Unregistered credentials' rewards go back to reserves (dr2).

2. The registration set the oracle uses is the DState at **startStep time** — after all epoch N-1 blocks have been processed but before the boundary tick. This state doesn't exist in our PRE/POST snapshot pair:
   - PRE snapshot = start of epoch N (after previous boundary processing)
   - POST snapshot = start of epoch N+1 (after current boundary processing)
   - Oracle's set = end of epoch N-1, after all N-1 blocks (between PRE N-1 and PRE N)

3. We approximate with POST registered credentials (1,055,538). The oracle's actual set is slightly smaller — accounts that registered during epoch 310's first block (POST capture point) or were added by applyRUpd are in our set but not in the oracle's. The ~6,411 extra registrations cause us to distribute ~164 ADA to accounts the oracle didn't.

4. **Where to look if this becomes an issue:**
   - `parse_registered_credentials()` in `snapshot_loader.rs` — parses UMap from DState
   - The UMap for pre-Conway is at `LS[0] → DPState[1] → DState[0] → array(2) [umElems, umPtrs]`
   - The UMap for Conway is at `LS[0] → CertState[2] → DState[0] → map(indefinite)`
   - The RDPair check: UMElem[0] major=4 (array) with val > 0 = registered (SJust)
   - To close the 164 ADA exactly: need the DState at the exact epoch boundary tick, OR implement the full pulser with per-credential registration checking against the epoch-end DState

5. **This does NOT affect Babbage/Conway** — at PV > 6, `hardforkBabbageForgoRewardPrefilter` returns True, skipping the registration check entirely. All rewards are distributed regardless of registration status.

**Broader CE-72 — OPEN:**

CE-72 covers the full Conway epoch boundary, not just the reward formula. Remaining:
- Governance-state correctness at epoch boundary (DRep stake, ratification/enactment ordering)
- Pulser equivalence beyond reward computation
- Conway-specific epoch boundary state effects (treasury withdrawals, governance proposals)

**Status: PARTIAL** — reward formula proven, broader Conway governance semantics open.

### CE-73 (HFC Translation Oracle State-Hash Equality)
- Translation logic semantically correct (22/22 fields match)
- State hash requires full LedgerState→CBOR encoder
- **Status: OPEN** — requires go/no-go decision on encoder work

### CE-74 (Ledger Determinism CI)
- `ci_check_ledger_determinism.sh` exists, runs `ledger_determinism.rs`
- Current: structural decode determinism on empty state. 7 eras, <1s.
- Missing: stateful replay with loaded snapshots, UTxO tracking, epoch boundary crossings, reward computation.
- **Status: PARTIAL** — structural smoke only. Needs stateful depth.

### CE-75 (Differential Divergence CI)
- `ci_check_differential_divergence.sh` exists, runs `differential_replay_all_eras.rs`
- Current: verdict agreement (accept/reject) across 10,500 blocks (1,500 per era, 7 eras). <2s.
- Missing: per-block state comparison against oracle state hashes. Epoch boundary portion required by CE definition.
- **Status: PARTIAL** — verdict agreement only. Needs state-level comparison.

### CE-77 (ScriptVerdict)
- ScriptVerdict wired through pipeline with native_script_passed/failed counts
- Plutus txs → NotYetEvaluated
- **Status: shape satisfied**

### CE-79 (Four-Tier Gate Statement)
- Not yet documented
- **Status: NOT STARTED**

### CE-68/69/70 (Alonzo/Babbage/Conway Structural Validation)
- Partial: parsed tx bodies + structural classification done
- Missing: state-backed checks (collateral existence, datum-hash presence)
- **Status: PARTIAL** — deferred to Phase 3 with Plutus

---

## 5. Key Design Decisions

### State Hash Comparison Surface
The oracle uses `Blake2b-256(encodeDiskExtLedgerState)` which includes Haskell-specific CBOR encoding choices. Our internal state uses different representations. Decision: prove semantic equivalence first (done), defer full encoding match.

### UTxO Tracking
`track_utxo: bool` flag on `LedgerState` controls whether `apply_block` produces/consumes UTxO entries. Disabled by default for performance (contiguous replay). Enabled when state is loaded from snapshots.

### Certificate Processing
Certificates parsed from tx body key 4, applied via `apply_cert` to accumulate `CertState` during replay. Types 0-4 (Shelley) handled; Conway types (7+) stored as opaque fallback.

### Reward Computation
Dual-denominator formula with unbounded apparent performance. Rewards computed using go snapshot data at the epoch boundary. PREALL comparison surface (using the correct epoch's data) is the authoritative test.

### Epoch Detection
Slot-to-epoch mapping uses mainnet Shelley parameters: start slot 4,492,800, start epoch 208, epoch length 432,000. Not configurable (hardcoded for mainnet).

---

## 6. Next Steps

Open items by status:

| Item | Status | What's needed |
|------|--------|---------------|
| CE-72 governance | OPEN | DRep stake, ratification/enactment, Conway governance state at boundary |
| CE-73 | OPEN | Full LedgerState→CBOR encoder or go/no-go decision |
| CE-74 | PARTIAL | Stateful replay depth in determinism CI |
| CE-75 | PARTIAL | Per-block state comparison in divergence CI |
| CE-79 | NOT STARTED | Four-tier gate statement document |
| CE-68/69/70 | PARTIAL | Deferred to Phase 3 (Plutus) |

---

## 7. File Map

### New Crate Modules (this phase)
```
crates/ade_codec/src/allegra/script.rs     — NativeScript CBOR decoder
crates/ade_codec/src/shelley/cert.rs       — Certificate CBOR decoder
crates/ade_ledger/src/alonzo.rs            — Alonzo structural validation
crates/ade_ledger/src/babbage.rs           — Babbage structural validation
crates/ade_ledger/src/conway.rs            — Conway structural validation
crates/ade_ledger/src/witness.rs           — WitnessInfo classifier
crates/ade_testkit/src/harness/snapshot_loader.rs — Snapshot loading + state bridge
```

### Modified Significantly
```
crates/ade_types/src/{alonzo,babbage,conway}/tx.rs — parsed tx body types
crates/ade_codec/src/{alonzo,babbage,conway}/tx.rs — full CBOR decoders
crates/ade_ledger/src/rules.rs     — apply_block pipeline + epoch boundary + rewards + four-flow accounting
crates/ade_ledger/src/rational.rs  — BigInt-backed arbitrary-precision Rational (num-bigint)
crates/ade_ledger/src/state.rs     — LedgerState + EpochState expanded
crates/ade_ledger/src/hfc.rs       — 6 translation functions + dispatch
crates/ade_ledger/src/error.rs     — StructuralError variants
crates/ade_ledger/src/scripts.rs   — ScriptPosture enum
```

### Corpus Data
```
corpus/boundary_blocks/     — 12 boundary block sets (re-extracted at correct slots)
corpus/genesis/             — 4 genesis files (Byron, Shelley, Alonzo, Conway)
corpus/snapshots/           — 23 tarballs + registry + hash files + sub-state summaries
```

---

## 8. Phase 2 Cumulative Totals

| Metric | Value |
|---|---|
| Total tests | 583 passing |
| Workspace crates | 7 (ade_codec, ade_types, ade_crypto, ade_core, ade_ledger, ade_testkit, ade_runtime) |
| BLUE crates | 6 (all except ade_runtime) |
| Corpus blocks | 10,500 contiguous + 252 boundary + 45 golden |
| Corpus snapshots | 23 ExtLedgerState tarballs (23GB) |
| Oracle state hashes | 10,502 per-block hashes + 12 anchor hashes |
| Eras covered | All 7 (Byron through Conway) |
| Transactions classified | 80,935 (71,499 non-Plutus + 9,436 deferred) |
| Certificates decoded | 6,354 across 7 eras |
| Delegations loaded | 98,331 from oracle |
| Pools loaded | 1,445 with full parameters |
| HFC translations | 6 functions, all tested end-to-end |
| Epoch boundary | CE-71 reward formula exact (0 lovelace delta), MIR four-flow decomposition proven |
| CI scripts | 12 (dependency boundary, forbidden patterns, crypto vectors, etc.) |

### Phase 2 Architecture

```
ade_types (BLUE)     — domain types, era-specific types, primitives
    ↑
ade_codec (BLUE)     — CBOR encoding/decoding, wire-byte preservation
    ↑
ade_crypto (BLUE)    — Blake2b, Ed25519, VRF, KES verification
    ↑
ade_ledger (BLUE)    — ledger rules, epoch boundary, HFC translations
    ↑                    UTxO, delegation, rewards, certificates
ade_testkit (GREEN)  — differential harness, snapshot loader, oracle comparison
    ↑
ade_runtime (RED)    — I/O, networking, storage (imperative shell)
    ↑
ade_node (RED)       — binary entry point
```

### Key Invariants Enforced

- **T-DET-01**: Same canonical inputs → same authoritative bytes
- **T-CORE-02**: No HashMap/HashSet, SystemTime, floats, fs, net, tokio, async, rand in BLUE
- **T-BOUND-02**: No BLUE crate depends on RED crate
- **DC-LEDGER-01**: `apply_block` is pure and deterministic
- **T-CONSERV-01**: Conservation law: consumed == produced (with protocol exceptions)
- **T-NOSPEND-01**: Double-spend rejection
- **DC-CRYPTO-01**: Crypto verification matches oracle (Blake2b, Ed25519, VRF, KES)
