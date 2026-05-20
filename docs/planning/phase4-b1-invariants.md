# PHASE4-B1 — Invariant Sketch (Full Block Validity Agreement)

> **Status**: planning artifact, non-normative. Produced via `/invariants` (IDD Part I).
> **Tier**: 1. The block-validity verdict is observable — disagreeing with the
> reference node on whether a block is valid forks the network. Bounty priority #1
> (block validity agreement); dependency root for B2 (tx validity) and B3 (block
> production).
> **Framing**: a block is valid iff its **header** is valid under consensus
> authority (PHASE4-N-B) **and** its **body** is valid under ledger authority
> (Phases 1–3). B1 composes the two existing authorities into one deterministic
> verdict and proves it equals the reference (Haskell cardano-node) verdict — on
> both valid AND invalid blocks — over real mainnet corpus.

## Public grounding (provenance)

Every invariant below is derived from public sources only:
- the Cardano **ledger specification** (Shelley → Conway formal ledger rules) and
  the **Ouroboros Praos/TPraos** consensus specification;
- the **cardano-node / cardano-ledger** reference implementation behavior;
- Ade's own **IDD doctrine** (fail-fast on invariant risk; closed semantic
  surfaces; illegal states unrepresentable) and the project **Byte Authority Model**.

No non-public material informs this sketch.

## Can it be expressed as `canonical input → canonical output`?

**Yes.**

```
block_validity
  (LedgerState, PraosChainDepState, EraSchedule, &LedgerView, block_cbor)
  -> Result<(LedgerState', PraosChainDepState', BlockValidityVerdict), BlockValidityError>
```

- **Canonical input**: the loaded `LedgerState` (UTxO + epoch snapshots + cert/pool
  state + pparams), the `PraosChainDepState` (nonces + op-cert counters), the typed
  `EraSchedule`, and the raw block bytes.
- **Canonical output**: a `BlockValidityVerdict` (Valid / Invalid{reason}) plus the
  evolved states. Pure, deterministic, replay-equivalent.
- **Agreement** = the verdict equals the reference verdict for the same block. No
  new validation logic — composition of two existing authorities.

The only nondeterminism risk is the **provenance of per-epoch consensus inputs**
(epoch nonce, set-snapshot stake, pool VRF keys). These are *canonical inputs* that
must be supplied, not computed live. Flagged as a proof obligation (§7), not an
authority-path nondeterminism.

## 1. What must always be true

| # | Invariant | Anchor |
|---|---|---|
| 1.1 | A block's validity verdict is a pure function of `(LedgerState, PraosChainDepState, EraSchedule, LedgerView, block_cbor)` — no wall-clock, no ambient state | NEW `DC-VAL-01` |
| 1.2 | `BlockValidity = HeaderValid ∧ BodyValid`. A block is Valid iff **both** the consensus header authority (N-B `validate_and_apply_header`) and the ledger body authority (`apply_block_with_verdicts`) accept it | NEW `DC-VAL-02` |
| 1.3 | The header is validated **before** the body; body validation never runs on a header-invalid block (fail-fast ordering) | NEW `DC-VAL-03` |
| 1.4 | `LedgerView` for epoch E is a **pure, total projection** of `LedgerState` (set-snapshot `pool_stakes`, `cert_state` pool VRF keys, `protocol_params` asc) + the epoch nonce from `PraosChainDepState`; it rederives nothing | **CN-EPOCH-01**, **DC-CONSENSUS-02** strengthened |
| 1.5 | Leader eligibility uses the **set snapshot** (frozen at E−2), consumed via `LedgerView`, never the live/mark snapshot | **CN-EPOCH-01** strengthened |
| 1.6 | Ade's verdict for a block equals the reference verdict (Valid/Invalid, and the reason class where the reference exposes it) | NEW `DC-VAL-04` (agreement) |
| 1.7 | **No false accept, proven on adversarial input**: a block the reference rejects is never marked Valid by Ade. Established by a *negative/adversarial corpus*, not only by agreement on valid blocks | **DC-LEDGER-02** (CE-88 lineage) strengthened; NEW `DC-VAL-04` |
| 1.8 | Header validation binds to the body actually present: the validated header's `body_hash`/`body_size` match the decoded body, and this check is **wired into the authority path** (not a defined-but-unwired validator), proven by a negative test (altered body → Invalid) | **CN-CONS-04** strengthened |
| 1.9 | State evolution is total: a Valid block yields `(LedgerState', PraosChainDepState')`; an Invalid block yields the **unchanged** input states + a structured reason (no partial mutation) | NEW `DC-VAL-05` |
| 1.10 | **Fail-closed validation**: every crypto-input / field-size / structural check rejects (→ Invalid) on wrong size or shape, and never silently skips. The anti-pattern `if X.len() == K { check } else { /* skip */ }` is forbidden in the authority path | NEW `DC-VAL-06` |

## 2. What must never be possible

| # | Forbidden | Anchor |
|---|---|---|
| 2.1 | Body validation running on a block whose header failed | DC-VAL-03 |
| 2.2 | A verdict depending on wall-clock, arrival order, `HashMap`/`HashSet` iteration, or float | DC-CORE-01, T-CORE-02 |
| 2.3 | `LedgerView` rederiving a stake snapshot instead of projecting `LedgerState` | DC-CONSENSUS-02 |
| 2.4 | Using the mark/go snapshot for leader eligibility instead of the set snapshot | CN-EPOCH-01 |
| 2.5 | Marking Valid any block the reference rejects (false accept) | DC-VAL-04, 1.7 |
| 2.6 | Partial state mutation on an Invalid block | DC-VAL-05 |
| 2.7 | A "skip header validation" / "trust the body" path in the authoritative verdict (the follow-bridge's peer-trusted shortcut is RED-only and must not leak here) | DC-VAL-02 |
| 2.8 | The epoch nonce / pool VRF key / set-stake being *guessed* when absent — absence is a structured fail-fast, never a default | DC-VAL-01, DC-VAL-06 |
| 2.9 | A fail-open length/size guard: wrong-size crypto input silently skipped instead of rejected | DC-VAL-06 |
| 2.10 | A defined-but-unwired check: a validator implemented and unit-tested but never called at the authority site | DC-VAL-02, 1.8 |
| 2.11 | A tautological/no-op guard (a value compared to itself) standing in for a real check | DC-VAL-06 |
| 2.12 | Recomputing a body/tx hash by re-encoding instead of using preserved wire bytes | T-ENC-01 / Byte Authority Model |
| 2.13 | Hardcoding one era's crypto domain (e.g. a single VRF seed/domain) across all eras — TPraos split-VRF vs Praos combined-VRF must be era-correct | DC-CRYPTO-01 |

## 3. What must remain identical across executions

- `block_validity(...)` → same `BlockValidityVerdict` and same evolved
  `(LedgerState', PraosChainDepState')` for the same canonical inputs.
- The `LedgerView` projection → same `(σ, total_active_stake, vrf_key, asc)` answers
  for the same `LedgerState` + epoch.
- The verdict's reason class for an Invalid block (so oracle-comparison is byte-stable).

## 4. What must be replay-equivalent

Two corpora, both replay-equivalent:

1. **Positive corpus** `corpus/validity/<era>_epoch<N>/` — real mainnet blocks at
   epochs where we hold the matching loaded `LedgerState`, the `PraosChainDepState`
   inputs (epoch nonce), and committed reference verdicts. Replaying `block_validity`
   over the block sequence produces an identical sequence of `BlockValidityVerdict`,
   identical evolved state fingerprints, and identical match against the committed
   reference verdicts.
2. **Negative / adversarial corpus** `corpus/validity/adversarial/` — blocks the
   reference rejects, derived by mutating real blocks: wrong witness/key/sig sizes,
   fabricated witnesses, oversized bignum (datum integer overflow), body bytes that
   do not match the header `body_hash`, future slot, era-mismatched VRF, missing
   required signer. Each replays to a deterministic `Invalid{reason}` matching the
   reference's rejection class. **This corpus is mandatory** — positive agreement
   alone does not establish 1.7/1.10.

Anchored by **T-DET-01**, **DC-LEDGER-02**.

## 5. State transitions in scope

```rust
BlockValidity
  (LedgerState, PraosChainDepState, &EraSchedule, &dyn LedgerView, block_cbor: &[u8])
  -> Result<(LedgerState, PraosChainDepState, BlockValidityVerdict), BlockValidityError>

LedgerViewProjection            ; pure, total; the production LedgerView impl
  (&LedgerState, EpochNo, epoch_nonce: &Nonce)
  -> impl LedgerView             ; reads set snapshot + cert pool params + pparams asc

// closed taxonomies
BlockValidityVerdict = Valid { new_tip, body_verdict: BlockVerdict }
                     | Invalid { reason: BlockValidityError }

BlockValidityError =
  | Header(HeaderValidationError)        // from N-B
  | Body(LedgerError)                    // from ade_ledger
  | BodyHashMismatch { header, actual }
  | MalformedField(FieldError)           // fail-closed size/shape rejection
  | MissingConsensusInput(MissingInput)  // nonce / set-stake / vrf key absent

MissingInput = EpochNonce | SetSnapshot | PoolVrfKey(Hash28) | ActiveSlotsCoeff
FieldError   = { field: &'static str, expected: usize, actual: usize }   // closed, no String
```

## 6. TCB color hypothesis

**BLUE — authoritative core**
- The `block_validity` composition transition (header authority ∧ body authority → verdict).
- The production `LedgerView` projection from `LedgerState` (pure, total, `BTreeMap`-backed — no `HashMap`).
- `BlockValidityVerdict` / `BlockValidityError` / `FieldError` closed taxonomies + canonical encoding.
- The fail-closed `expect_size`-style helper (returns `Err`, never skips).

**GREEN — deterministic glue / test infra**
- The agreement harness in `ade_testkit`: assembles `(LedgerState, chain-dep, blocks,
  reference verdicts)` from corpus and drives `block_validity`, comparing to the
  reference — for both the positive and adversarial corpora.

**RED — shell**
- Snapshot + reference-verdict + per-epoch-consensus-input loading from disk/corpus
  (extends the existing snapshot loader).
- Adversarial-corpus generation tooling (mutators) — non-authoritative.

**Unresolved (open question):** whether the `LedgerView` projection is BLUE (pure
projection of canonical state — my hypothesis) or GREEN (adapter). It feeds
authoritative decisions and is pure, so I lean BLUE; confirm at cluster-plan.

## 7. Open questions (must resolve before `/cluster-plan`)

1. **Per-epoch consensus-input provenance (proof obligation).** Full header validation
   at epoch E needs: the **epoch nonce** for E, the **set-snapshot** `pool_stakes`
   (frozen E−2), and each issuer's **registered pool VRF key**. Which of these does the
   existing snapshot loader already capture, and which must be added to the corpus from
   the reference node? Slice-entry proof obligation, not a footnote.
2. **Corpus coverage.** At which epochs do we hold *all three* of: boundary blocks, a
   loaded `LedgerState` snapshot, and the consensus inputs from (1)? B1's reach is
   bounded by that intersection. Need the concrete list before planning.
3. **Reference verdict granularity.** Does the reference expose only Valid/Invalid, or
   a reason class? Determines how strong the `DC-VAL-04` agreement assertion can be
   (verdict-only vs. reason-matched), and how precisely the adversarial corpus can pin
   rejection reasons.
4. **KES signature verification.** N-B's `validate_and_apply_header` checks the op-cert
   counter but defers KES signature verification (S-B7 §15). Is KES-sig verification in
   scope for B1's "header valid", or does it stay deferred (and tracked as a known gap)?
5. **`LedgerView` color** (see §6).
6. **Set vs. mark vs. go snapshot mapping.** Confirm `SnapshotState`'s set snapshot is
   the one used for epoch-E leader schedule (E−2 freeze), matching Ouroboros semantics.
7. **Fail-closed audit of the existing engine (proof obligation).** Before composing,
   audit `ade_ledger` phase-1 witness validation for any fail-open length guard
   (the `if len == K { check } else { skip }` pattern) on vkey / signature / bootstrap
   key inputs; confirm body/tx hashing uses preserved wire bytes; confirm the VRF domain
   is era-correct (TPraos vs Praos). Any gap found is folded into B1 as a fail-closed fix
   with a negative test.

## 8. Investigation results (resolves §7 #1, #2, #7)

Read-only investigation against the codebase and the `ade-corpus-snapshots` corpus.

**#7 fail-closed audit — RESOLVED, Ade is structurally ahead.**
- Ed25519 witness signature verification is present and fail-closed:
  `crates/ade_ledger/src/shelley.rs:204-221` constructs keys/sigs via
  `Ed25519VerificationKey::from_bytes` / `Ed25519Signature::from_bytes` (which return
  `Err` on wrong length) and then calls `verify_ed25519`. Byron bootstrap witnesses are
  verified at `crates/ade_ledger/src/byron.rs:231-238` (`verify_byron_bootstrap`).
- The `witness.rs::decode_vkey_hashes` path that "skips signature" is the
  required-signer *presence-hash* path, not the verification path — legitimate.
- `ade_crypto` length guards are all fail-closed constructors (`ed25519.rs`,
  `vrf.rs`, `kes.rs` return `Err` on wrong length). No fail-open `if len==K {} else {skip}`
  pattern exists in `ade_ledger` non-test code.
- Hashing uses preserved wire bytes: `rules.rs:415` `tx_hash = blake2b_256(wire_bytes)`;
  blocks decode via `preserved.decoded()` (PreservedCbor). The re-encode/hash-invariance
  class is not present.
- **Carry into cluster-plan (confirm, don't assume):** (a) that the `shelley.rs`
  `verify_ed25519` path is reached by `apply_block_with_verdicts` for every era (a "wired"
  check — prove with a negative test: fabricated witness → Invalid); (b) era-correct VRF
  domain in `ade_core::consensus::vrf_cert` (TPraos vs Praos input/seed).
  Net: B1's adversarial corpus will mostly *confirm* fail-closed behavior with negative
  tests rather than fix gaps.

**#1 consensus-input provenance — RESOLVED via `ade-corpus-snapshots` (S3).**
The current in-repo snapshot loader does NOT capture the consensus inputs:
`snapshot_loader.rs:406` sets `vrf_hash: Hash32([0u8;32])` and `:962` skips the pool
`vrf` field; no epoch nonce is captured anywhere. But the inputs exist in S3:
- **epoch nonce + op-cert counters** → `extracts/proto_state_babbage.json` (a CBOR
  `cardano-cli query protocol-state` dump despite the `.json` name — the Praos
  ChainDepState: lab/evolving/candidate/tick nonces + cert counters).
- **pool VRF keys + set/mark/go stake snapshots** → `extracts/ledger_state_conway576*.json`,
  `extracts/stake_snap_babbage.json`, `extracts/ledger_state_babbage.json` (full
  ledger-state dumps; the loader currently skips the VRF field — stop skipping).
- **active-slots-coeff** → `extracts/proto_params_babbage406.json`.
- **blocks** → `extracts/blocks_epoch577.tar.gz` + existing `corpus/boundary_blocks/`.
- Reference node binary for oracle verdicts → `tools/cardano-node-8.12.2-linux.tar.gz`.
- **First B1 task:** extend extraction to (a) capture pool VRF keys (un-skip the field)
  and (b) parse the protocol-state CBOR for the epoch nonce + op-cert counters.

**#2 corpus coverage — partially resolved.** Confirmed reach: **Babbage** (ledger state +
stake snap + proto params + proto state) and **Conway 576/577** (ledger-state dumps +
`blocks_epoch577`). Open: confirm/obtain a **Conway protocol-state** dump (epoch nonce for
576/577) — S3 `extracts/` shows `proto_state_babbage.json` but no conway proto-state yet;
may need extraction via the bundled reference node. This bounds B1's first cluster to the
epochs where blocks + ledger state + protocol state (nonce) all coexist.

## Proposed registry entries (DC-VAL family — NEW)

The `DC` (derived constraint) family gains a new sub-family **`DC-VAL`** for
block-validity composition + agreement. Six entries proposed; shown for confirmation
before appending to `docs/ade-invariant-registry.toml`. `introduced_in` is `TBD`
until `/cluster-plan` assigns the cluster id.

```toml
[[rules]]
id = "DC-VAL-01"
name = "Block validity verdict is a pure function of canonical inputs"
invariant = """
A block's validity verdict is a pure function of (LedgerState, PraosChainDepState,
EraSchedule, LedgerView, block_cbor). No wall-clock, arrival order, HashMap/HashSet
iteration, float, or ambient state may influence it.
"""
family = "DC"
source = "Cardano ledger spec; Ouroboros Praos spec; IDD determinism doctrine"
kind = "determinism"
introduced_in = "TBD"
status = "active"

[[rules]]
id = "DC-VAL-02"
name = "Block validity = header authority AND body authority"
invariant = """
A block is Valid iff both the consensus header authority (N-B
validate_and_apply_header) and the ledger body authority (apply_block_with_verdicts)
accept it. No path may produce a Valid verdict while skipping either authority.
"""
family = "DC"
source = "Cardano ledger + Ouroboros Praos specs"
kind = "authority"
introduced_in = "TBD"
status = "active"

[[rules]]
id = "DC-VAL-03"
name = "Header validated before body; fail-fast ordering"
invariant = """
The header is validated before the body. Body validation never runs on a
header-invalid block. The first failing authority determines the reason.
"""
family = "DC"
source = "Ouroboros Praos spec; IDD fail-fast doctrine"
kind = "fail-fast"
introduced_in = "TBD"
status = "active"

[[rules]]
id = "DC-VAL-04"
name = "Block validity verdict agrees with the reference node"
invariant = """
Ade's Valid/Invalid verdict for a block equals the reference cardano-node verdict,
including the reason class where the reference exposes it. Established over both a
positive corpus (real valid blocks) and a mandatory adversarial corpus (blocks the
reference rejects).
"""
family = "DC"
source = "cardano-node reference behavior; Cardano ledger spec"
kind = "authority"
introduced_in = "TBD"
status = "active"

[[rules]]
id = "DC-VAL-05"
name = "Total state evolution; no partial mutation on invalid blocks"
invariant = """
A Valid block yields evolved (LedgerState', PraosChainDepState'). An Invalid block
yields the unchanged input states plus a structured reason. No partial or in-place
mutation occurs on the invalid path.
"""
family = "DC"
source = "IDD explicit-total-transition doctrine; Cardano ledger spec"
kind = "authority"
introduced_in = "TBD"
status = "active"

[[rules]]
id = "DC-VAL-06"
name = "Validation fails closed on malformed input"
invariant = """
Every crypto-input, field-size, and structural check on the authority path rejects
(produces Invalid) on wrong size or shape and never silently skips. The pattern
`if X.len() == K { check } else { skip }` is forbidden in BLUE validation; size
checks go through a helper that returns an error. No defined-but-unwired check and
no tautological (value-compared-to-itself) guard may stand in for a real check.
"""
family = "DC"
source = "Cardano ledger spec (mandatory witness/field checks); IDD fail-fast doctrine"
kind = "fail-fast"
introduced_in = "TBD"
status = "active"
```

**Existing rules to strengthen** (recorded at cluster close, not now):
`DC-LEDGER-02` (no-false-accept / CE-88 lineage), `CN-EPOCH-01`, `DC-CONSENSUS-02`,
`CN-CONS-04`, `DC-CRYPTO-01`, `T-ENC-01`.

## Authority reminder

This sketch is a planning aid. Authority for invariants belongs to
`docs/ade-invariant-registry.toml`. If this sketch conflicts with the registry or the
normative specs, those win.
