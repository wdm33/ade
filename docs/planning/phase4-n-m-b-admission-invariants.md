# PHASE4-N-M-B — Admission orchestrator + AgreementVerdict + admission JSONL — invariants sketch

## Framing

Sub-cluster B of the PHASE4-N-M family. Sits on top of N-M-A
(seed importer + BootstrapAnchor + WAL) and ships the admission
mode of the `ade_node` binary: bootstrap from a seed, follow a
peer's chain via N-L's wire stack, admit each block via
CN-CONS-08, append every step to the WAL, and emit a closed
`AgreementVerdict` per admit.

This cluster is mostly **wiring** — composing N-L (wire) +
N-M-A (storage) + CN-CONS-08 (admit authority) into one tokio
runner with closed events. No new BLUE authorities; one new
GREEN reducer (`verdict::derive`); one new RED runner
(`admission::run_admission`).

Honest scope sub-cluster doctrine (load-bearing):

- Per [[feedback-shell-must-not-overstate-semantic-truth]] the
  admission JSONL vocabulary is a SEPARATE closed enum from
  wire-only's `LiveLogEvent`. The two are physically isolated
  files; CI grep keeps wire-only files free of admission
  literals and vice versa **and the reverse direction** —
  admission-mode files free of wire-only-only literals.
- Per [[feedback-oracle-seed-then-ade-owns]] the admission path
  uses A1's seed importer + A2's BootstrapAnchor + A3's WAL
  exclusively. Cardano-node is consulted ONLY for live chain
  data after the seed is imported.
- Per [[feedback-evidence-reducers-are-green-not-authority]] —
  `AgreementVerdict` is **GREEN evidence**, not consensus /
  storage / ledger authority. `verdict::derive` compares
  already-authoritative outputs (CN-CONS-08's admit verdict +
  peer's announced tip) and emits a closed evidence sum. It
  MUST NOT decide validity, chain selection, or canonical
  state. See §6 classification table for tier mapping.

This cluster **DOES NOT** close RO-LIVE-05 — it ships the
mechanical half. RO-LIVE-05 + the wide RO-LIVE-03 close at C
(operator pass).

## 0. Honest prerequisites + non-goals

### A1.1 hard prerequisite for sub-cluster C operator pass

N-M-A's seed importer fail-fast-rejects reference-script TxOuts
(`JsonSeedError::UnsupportedTxOutFeature { feature: "referenceScript" }`).
Real preprod has thousands of script outputs. **Operator-pass
admission against a live preprod tip needs A1.1 ref-script
support before C can close.**

Where this leaves B: B's mechanical evidence works against
hand-curated UTxO seeds (just the entries a specific test block
consumes). B's hermetic test uses real Conway corpus block
bytes + a hand-curated UTxO seed that ONLY contains the inputs
that block consumes. That's enough to prove admission + verdict
derivation + WAL recording end-to-end without needing A1.1.

A1.1 ships either as a slice extension of A or as the first
slice of C. Either way: NOT this cluster's deliverable.

### Other non-goals (carried from N-M family)
- Genesis → P replay (`RO-GENESIS-REPLAY-01` open).
- Mithril import (`RO-MITHRIL-IMPORT-01` open).
- utxohd-mem binary decode (Tier-5 non-goal).
- Block production (`CN-CONS-06` separate).
- Mempool / tx-submission (`PHASE4-N-E` live half).
- TLS / auth (`¬P-8` carried from N-L).

## 1. What must always be true

- **I-B1 Closed `AgreementVerdict` sum** — GREEN evidence type
  per [[feedback-evidence-reducers-are-green-not-authority]].
  Exactly four variants, each with a NARROW comparison
  semantics:
  - `Agreed { our_hash, peer_hash }` — admit succeeded and
    `our_hash == peer_hash` byte-identically at the same slot.
    The reducer asserts byte-equality before constructing this
    variant; conflating it with "validity ruling" is forbidden.
  - `Lagging { our_slot, peer_slot }` — **NARROW DEFINITION**:
    the local admitted chain is a *prefix* of the comparison
    target (peer's announced chain) up to the peer's announced
    slot. Lagging is **evidence-state only** — it MUST NOT be
    treated as success / healthy / live-ready /
    consensus-equivalent by any caller. Hard prohibition
    (¬P-B8 below); CI grep enforces.
  - `Diverged { our_hash, peer_hash, slot }` — our admit
    succeeded BUT produced a block whose hash differs from the
    peer's announced hash at that slot, OR our admit returned
    `BlockValidityVerdict::Invalid` for a block the peer
    accepted at the same slot. Authority-fatal at the binary
    boundary, evidence-typed at the reducer boundary.
  - `InputNotFound { tx_in }` — **NARROW DEFINITION**: the
    comparison input (a `TxIn` the block consumes) was
    unavailable from the configured evidence source (the
    imported UTxO seed). This variant does NOT mean "the block
    was malformed" or "storage rejected it" or "ledger
    rejected it" — those remain `CodecError` / `ChainWriteError`
    / `BlockValidityError` on their existing authority surfaces.
    `InputNotFound` is "stale-seed / incomplete-seed evidence."
- **I-B2 Diverged + InputNotFound are authority-fatal at the
  binary boundary.** Distinct exit codes:
  - `EXIT_LIVE_AGREEMENT_DIVERGED = 30`
  - `EXIT_LIVE_INPUT_NOT_FOUND = 31`
  - Mirrors PHASE4-N-K DC-NODE-04 fail-fast discipline. The
    *fatality* is the binary's response to the evidence — the
    verdict reducer itself stays pure / no exit.
- **I-B3 Per-admit emission.** Exactly one
  `agreement_verdict` JSONL event per admit-attempt. Never
  twice for the same block_hash; never zero for a successful
  admit.
- **I-B4 Closed `AdmissionLogEvent` sum** — 8 variants:
  `admission_started`, `snapshot_imported`, `bootstrap_complete`,
  `block_received`, `block_admitted`, `agreement_verdict`,
  `admission_halted`, `admission_shutdown`. Separate from
  wire-only's `LiveLogEvent`; both vocabularies are closed
  enums in separate files; CI grep keeps each mode's literals
  out of the other's files.
- **I-B5 Single admission entry authority.** Exactly one
  `pub fn` in `ade_node::admission::run_admission` enters the
  admission tokio runner. CN-ADMIT-01.
- **I-B6 Single seed-snapshot bridge authority.** Exactly one
  `pub fn` in `ade_node::admission::seed_to_snapshot` converts
  the imported `(UTxOState, ledger_fingerprint, seed_point)`
  into a persisted snapshot via the existing
  `PersistentSnapshotCache::capture` (which is CN-STORE-08's
  sole authority). The bridge does not bypass
  `bootstrap_initial_state` — it persists a snapshot at
  `seed_point.slot` so `bootstrap_initial_state`'s warm-start
  branch picks it up. CN-NODE-01 preserved.
- **I-B7 Per-admit WAL append + admit-replay-equivalence
  (true-tier).** Every successful admit appends one
  `WalEntry::AdmitBlock` to the configured `WalStore`. The
  entry's `prior_fp` chains to the previous entry's `post_fp`
  (or the anchor's `initial_ledger_fingerprint` for the first
  entry). Failure to append is authority-fatal.
  
  **The stronger replay-equivalence proof obligation that
  follows** (and that B's integration test pins): for every
  successful `AdmitBlock` transition, replay from the prior
  checkpoint + WAL produces:
  
  1. the same post-admit `LedgerState` fingerprint, AND
  2. the same emitted `AgreementVerdict` from a re-run of
     `verdict::derive` over the replayed
     `(admit_outcome, peer_tip)` pair.
  
  This is a true-tier property strengthening CN-STORE-03
  (replay-equivalent recovery), not merely a logging property.
  The integration test asserts both halves.
- **I-B8 Verdict reducer is pure.** `verdict::derive(admit_outcome,
  peer_tip)` is a pure function over closed input enums →
  closed output enum. No I/O, no clock, no state. Per
  [[feedback-evidence-reducers-are-green-not-authority]] this
  is the GREEN-evidence boundary: the reducer compares
  authoritative outputs; it never decides authority.

## 2. What must never be possible

- **¬P-B1** `AgreementVerdict { Agreed }` when our admitted block
  hash differs from the peer's announced hash at that slot. Type-
  level: the `Agreed` variant carries both hashes; the verdict
  reducer asserts equality before emitting.
- **¬P-B2** Emitting `agreement_verdict` without a preceding
  `admit_via_block_validity` call.
- **¬P-B3** Silently retrying after `Diverged` or `InputNotFound`.
- **¬P-B4** Mixing admission and wire-only event literals in one
  file. CI grep enforces.
- **¬P-B5** Bypassing `bootstrap_initial_state`. CN-NODE-01
  preserved.
- **¬P-B6** Skipping the WAL append on a successful admit. Every
  admit produces exactly one append.
- **¬P-B7** Calling `verdict::derive` with non-closed-sum inputs
  (forces every caller to construct the typed `AdmitOutcome`).
- **¬P-B8 Treating `Lagging` as success.** No code path in B
  may map `Lagging` to a success / healthy / live-ready /
  consensus-equivalent state. `Lagging` is evidence-state only:
  it MAY be logged + counted; it MUST NOT advance any
  "we passed live evidence" claim. CI grep
  (`ci/ci_check_lagging_is_evidence_only.sh`) forbids:
  - `Lagging` matched as part of a success-result pattern
    (`Ok(Lagging)` / `Lagging => true` / etc. outside the
    verdict reducer and its tests).
  - Any caller passing a `Lagging` verdict into a "ready" /
    "healthy" / "live" predicate.
- **¬P-B9 Reference-script TxOut decode.** B MUST NOT add
  partial reference-script support, permissive ref-script
  skipping, or any seed-import fallback. A1's fail-fast on
  `referenceScript` stays exactly as-is. Real preprod seed
  import remains blocked until A1.1 closes — this prevents B
  from inheriting a known-invalid importer and rests sub-cluster
  C operator-pass evidence on a clean A1.1 closure. CI grep
  (`ci/ci_check_admission_no_refscript_skip.sh`) verifies the
  admission code paths do not match `JsonSeedError::UnsupportedTxOutFeature`
  with a permissive arm.
- **¬P-B10 Mixing the wire-only and admission JSONL
  vocabularies in either direction.** CI grep enforces:
  - wire-only-mode files do not emit `AdmissionLogEvent`
    literals (`admission_started`, `snapshot_imported`,
    `bootstrap_complete`, `block_received`, `block_admitted`,
    `agreement_verdict`, `admission_halted`,
    `admission_shutdown`).
  - admission-mode files do not emit wire-only-only
    `LiveLogEvent` literals (`peer_dial_started`,
    `handshake_ok`, `peer_tip_read`, `peer_dial_failed`,
    `wire_smoke_complete`). The shared literals
    (`node_started`, `node_shutdown`) are allowed in both
    modes — they're operational, not mode-claiming.

## 3. What must remain identical across executions

- The verdict reducer is pure → `(same admit_outcome, same
  peer_tip) → same verdict` across runs.
- Given the same seed-JSON + same recorded block-byte sequence
  + DeterministicClock, the admission run produces a
  byte-identical JSONL log + byte-identical WAL.
- The seed-to-snapshot bridge is deterministic — same imported
  state → same persisted snapshot bytes (CN-STORE-08 carries
  this).

## 4. What must be replay-equivalent

- The hermetic admission test runs twice over the same seed +
  block bytes + clock and asserts byte-identical JSONL output +
  byte-identical WAL.

## 5. State transitions in scope

```text
verdict::derive(admit_outcome, peer_tip) -> AgreementVerdict
  // pure reducer; closed -> closed

admission::seed_to_snapshot(
  utxo: UTxOState,
  ledger_fp: Hash32,
  seed_point: SeedPoint,
  store: &mut dyn SnapshotStore,
) -> Result<(), SeedToSnapshotError>
  // sole authority bridging A1's seed import to N-D snapshot store
  // composes existing PersistentSnapshotCache::capture

admission::run_admission(cli, writer, wal_store, shutdown) -> ExitCode
  // 1. Emit admission_started.
  // 2. Import JSON seed via N-M-A's import_cardano_cli_json_utxo.
  // 3. Mint BootstrapAnchor.
  // 4. seed_to_snapshot — write to persistent snapshot store.
  // 5. Emit snapshot_imported.
  // 6. bootstrap_initial_state warm-starts from the snapshot.
  // 7. Emit bootstrap_complete.
  // 8. Spawn N2nDialer for each --peer.
  // 9. Loop: for each AdmittedBlock effect from orchestrator:
  //    - WalStore::append(WalEntry::AdmitBlock {...})
  //    - verdict = verdict::derive(...)
  //    - Emit agreement_verdict.
  //    - Match verdict {
  //        Diverged | InputNotFound => emit admission_halted, exit fatal,
  //        Agreed | Lagging => continue,
  //      }
  // 10. On signal: drain pipeline, emit admission_shutdown, exit 0.
```

## 6. Tier + boundary classification (per
[[feedback-evidence-reducers-are-green-not-authority]])

Pinned table — every item in this cluster MUST map cleanly to a
row. If a future commit introduces an item that doesn't fit,
either the doctrine generalizes (update the memory) or the
introduction is wrong (revert).

| Item                                                 | Tier                      | Boundary / authority                                                                |
|------------------------------------------------------|---------------------------|-------------------------------------------------------------------------------------|
| `AgreementVerdict` closed sum                        | release / GREEN evidence  | compares Ade admit output vs peer's announced tip; never decides validity           |
| `AdmitOutcome` closed input enum                     | release / GREEN evidence  | typed view of CN-CONS-08's `admit_via_block_validity` output; reducer consumes only |
| `verdict::derive` reducer                            | release / GREEN evidence  | pure; no I/O / clock / state; closed-in → closed-out                                |
| `Diverged` / `InputNotFound` → fatal exit            | release gate              | binary-boundary response to evidence; prevents continuing after failed evidence     |
| WAL append per successful admit                      | true / derived storage    | replay surface; CN-WAL-01 single authority; chains to anchor                        |
| Admit-replay-equivalence proof obligation (I-B7)     | true / mechanically enforced | recovery-test asserts replay from prior checkpoint + WAL → same post-state AND same verdict |
| Bootstrap warm-start via `bootstrap_initial_state`   | true / authority          | unchanged; CN-NODE-01 preserved                                                     |
| `admit_via_block_validity` per block                 | release / authority       | unchanged; CN-CONS-08 preserved                                                     |
| `PersistentSnapshotCache::capture` for seed snapshot | true / authority          | unchanged; CN-STORE-08 sole canonical encode path                                   |
| `AdmissionLogEvent` closed sum                       | release / GREEN observability | per-event JSONL; closed enum; vocabulary physically isolated from wire-only       |
| `run_admission` tokio runner                         | RED shell                 | composes the above; no authority shortcuts; no evidence overloading                 |
| `Lagging` verdict variant                            | release / GREEN evidence (narrow) | evidence-state only; ¬P-B8 forbids treating as success                       |

The replay-equivalence supreme law (same anchor + same inputs +
same WAL + same checkpoints → byte-identical outputs) is the
through-line: every row above either *carries* state into that
chain (storage / authority rows) or *observes* the chain
(evidence rows). Nothing in this cluster mixes the two.

## 7. TCB color hypothesis

- **GREEN (new):**
  - `ade_node::admission::verdict` — pure `derive` reducer +
    closed `AgreementVerdict` + closed `AdmitOutcome` input.
  - `ade_node::admission_log::{event, writer}` — closed
    `AdmissionLogEvent` enum + writer.
  - `ade_node::admission::seed_to_snapshot` — pure adapter
    composing N-M-A seed import → `PersistentSnapshotCache::capture`.
- **RED (new):**
  - `ade_node::admission::runner` — tokio runner; `run_admission`
    entry point.
  - `ade_node::cli` — extended (`--mode admission`, `--json-seed PATH`,
    `--wal-dir PATH`).
  - `ade_node::main` — extended dispatch on `cli.mode == Admission`.
- **BLUE:** unchanged.

## 8. Decisions on framing questions

| # | Question | Decision |
|---|----------|----------|
| 1 | Where does seed import happen? | Inside `run_admission` at startup. One-shot per node startup; not re-imported on restart (warm-start picks up the persisted snapshot). |
| 2 | Two-step vs single-step CLI | Single-step: `ade_node --mode admission --json-seed PATH --wal-dir PATH --peer ADDR ...`. Operator runs cardano-cli to produce the JSON; ade_node consumes + processes in one go. |
| 3 | What if a seed snapshot already exists in the store? | Warm-start uses it; the `--json-seed` flag is honored only if no snapshot ≤ `seed_point.slot` exists. If a snapshot exists at a higher slot, the seed is rejected with `SeedTooStale` (fail-fast). |
| 4 | Verdict reducer scope | Pure function; tested with hand-crafted `(AdmitOutcome, Tip)` inputs. No I/O, no per-call mutation. |
| 5 | What if peer's tip is empty (Origin)? | `peer_tip = Origin` → `Lagging` (our admit can't be agreed-with-Origin; we're always at-or-ahead-of Origin). |
| 6 | When does an admission run cleanly exit? | SIGINT/SIGTERM → drain + `admission_shutdown` + exit 0. Authority-fatal verdict → `admission_halted` + exit 30/31. Peer drop → continue (next dial). |
| 7 | Per-admit log line content | `agreement_verdict { kind, peer_addr, slot, our_hash?, peer_hash?, tx_in? }` — kind is the discriminator; hashes/tx_in are per-variant. |
| 8 | WAL dir default | `--wal-dir` default `./wal/`. Operator-overridable. |
| 9 | Hermetic test strategy | Pick a Conway corpus block; extract its inputs; build a minimal seed JSON containing only those entries; loopback responder sends the block; assert `Agreed`. |
| 10 | Genesis hash for the anchor | Operator-supplied via `--genesis-hash HEX` flag OR derived from `--genesis-path` bundle (reuses N-K's plumbing). |

## 9. Registry deltas (planned at /cluster-plan)

### New rules (declared at append; flipped per slice)
- `CN-ADMIT-01` — single admission-mode entry authority.
- `CN-ADMIT-02` — single seed-to-snapshot bridge authority.
- `DC-ADMIT-01` — closed `AgreementVerdict` sum (GREEN evidence,
  not authority).
- `DC-ADMIT-02` — verdict emitted exactly once per admit.
- `DC-ADMIT-03` — Diverged + InputNotFound are authority-fatal
  at the binary boundary (evidence-typed at the reducer
  boundary).
- `DC-ADMIT-04` — closed `AdmissionLogEvent` vocabulary; CI
  grep enforces **physical isolation in BOTH DIRECTIONS** —
  wire-only files emit no admission literals AND admission
  files emit no wire-only-only literals.
- `DC-ADMIT-05` — per-admit WAL append (every successful admit
  produces exactly one `WalEntry::AdmitBlock`).
- `DC-ADMIT-06` — verdict reducer is pure (no I/O, no clock,
  no state); reducer is GREEN evidence boundary.
- `DC-ADMIT-07` — admit-replay-equivalence (true-tier): for
  every successful `AdmitBlock`, replay from prior checkpoint
  + WAL produces (a) the same post-admit `LedgerState`
  fingerprint AND (b) the same emitted `AgreementVerdict`
  from a re-run of `verdict::derive`. Strengthens CN-STORE-03.
- `DC-ADMIT-08` — `Lagging` is evidence-state only; no code
  path treats it as success / healthy / live-ready /
  consensus-equivalent. CI grep
  (`ci_check_lagging_is_evidence_only.sh`).
- `DC-ADMIT-09` — admission code paths do not add partial
  reference-script support, permissive ref-script skipping,
  or any seed-import fallback. Real preprod seed import stays
  blocked until A1.1. CI grep
  (`ci_check_admission_no_refscript_skip.sh`).

### Strengthenings recorded at cluster close
- `T-DET-01.strengthened_in += "PHASE4-N-M-B"` — verdict reducer
  + admission JSONL determinism under DeterministicClock.
- `CN-CONS-08.strengthened_in += "PHASE4-N-M-B"` — admit path
  now driven by real peer chain-sync + block-fetch through
  admission mode (mechanical half).
- `CN-NODE-01.strengthened_in += "PHASE4-N-M-B"` — admission
  routes through `bootstrap_initial_state` (warm-start);
  preserves single bootstrap authority.
- `CN-WAL-01.strengthened_in += "PHASE4-N-M-B"` — every admit
  append goes through `WalStore::append`.
- `CN-STORE-03.strengthened_in += "PHASE4-N-M-B"` (if rule
  exists; if absent under that ID, the property is recorded
  as a new derived rule and cross-referenced) — admit-replay-
  equivalence is true-tier, mechanically enforced (DC-ADMIT-07).

## 10. Slice shape (proposed; refine at /cluster-plan)

| Slice | Scope | TCB | Effort |
|-------|-------|-----|--------|
| B1 | GREEN `ade_node::admission::verdict` — closed `AgreementVerdict` (4 variants, narrow per I-B1) + closed `AdmitOutcome` + pure `derive` reducer. CI gate `ci_check_lagging_is_evidence_only.sh`. DC-ADMIT-01 + DC-ADMIT-06 + DC-ADMIT-08. | GREEN + CI | small |
| B2 | GREEN `ade_node::admission_log::{event, writer}` — closed `AdmissionLogEvent` (8 variants) + hand-rolled JSON writer. CI gate `ci_check_admission_log_vocabulary_closed.sh` enforces **both-direction** vocabulary isolation against wire-only. DC-ADMIT-04. | GREEN + CI | small |
| B3 | GREEN `ade_node::admission::seed_to_snapshot` — adapter composing N-M-A seed import → `PersistentSnapshotCache::capture`. CI gate `ci_check_admission_no_refscript_skip.sh` (DC-ADMIT-09). CN-ADMIT-02. | GREEN + CI | small |
| B4 | RED `ade_node::admission::runner` — tokio entry + per-AdmittedBlock loop + WAL append + verdict emit + fatal-on-Diverged/InputNotFound. CN-ADMIT-01 + DC-ADMIT-02 + DC-ADMIT-03 + DC-ADMIT-05. | RED | medium |
| B5 | RED `ade_node::main` `--mode admission` dispatch + new CLI flags (`--json-seed`, `--wal-dir`, `--genesis-hash`). | RED | small |
| B6 | Hermetic loopback admission test — Conway corpus block + minimal seed UTxO + in-process responder; assert `Agreed`. **Admit-replay-equivalence test (DC-ADMIT-07)**: replay from prior checkpoint + WAL produces same post-admit `LedgerState` fingerprint AND same emitted `AgreementVerdict`. | RED + test | medium |
| B7 | Cluster close — flip rules (incl. DC-ADMIT-01..09, CN-ADMIT-01/02), strengthenings (T-DET-01, CN-CONS-08, CN-NODE-01, CN-WAL-01, CN-STORE-03), commit + push. | — | small |

Dependencies: B2 ↔ B1 (writer references verdict); B3 standalone;
B4 ↔ B1+B2+B3 + N-M-A + N-L stack; B5 ↔ B4; B6 ↔ B5; B7 ↔ B6.

Total estimate: ~3-5 days, mirroring PHASE4-N-L's shape.

## 11. Honest-scope carry-forward

- **Sub-cluster C** (operator pass) is the next cluster after B.
  Requires: B closed + A1.1 reference-script support landed.
- **A1.1 reference-script TxOut decode** — hard prereq for C.
  Likely a small slice extension on A1's importer (the JSON
  shape carries `referenceScript` as a structured object; the
  Babbage `script_ref` CBOR wrapping is well-defined).
- **`RO-LIVE-05` + `RO-LIVE-03`** — still open until C closes.
- **`RO-GENESIS-REPLAY-01`** + **`RO-MITHRIL-IMPORT-01`** —
  open obligations carried unchanged.
- **utxohd-mem on-disk binary decoder** — explicit non-goal.

## 12. Why this is the right shape

- Mostly wiring + closed sums; no new BLUE authorities. Risk
  surface is well-bounded.
- Mirror PHASE4-N-L-LIVE's pattern (closed JSONL vocabulary +
  per-mode-isolation CI gate) for proven discipline.
- A1.1 split-out keeps B's scope honest — operator pass
  prerequisites are explicit, not buried.
- Replay-equivalence test under DeterministicClock proves the
  full pipeline (seed → bootstrap → admit → verdict → WAL →
  JSONL) is deterministic, which is the bounty's headline
  property.

## 13. Slice ordering — decision recorded

User approval (2026-05-26) confirmed **B-first → A1.1 → C**.
Rationale: B is a valid invariant slice in its own right —
mergeable, fully correct, replay-verifiable, and does not
depend on future patches to restore safety. The cluster
closes a real integration seam (verdict reducer + WAL
admit-replay-equivalence + bootstrap discipline) without
pretending to close live readiness.

User refinements applied (above): tier classification table at §6;
narrow `Lagging` + `InputNotFound` semantics with hard
prohibitions; bidirectional CI grep for JSONL vocabulary
isolation; admit-replay-equivalence elevated to true-tier
(DC-ADMIT-07 strengthening CN-STORE-03); explicit no-A1.1-creep
prohibition (DC-ADMIT-09).
