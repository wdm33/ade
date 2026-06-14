# Pre-preprod local-first work streams (drain before the stake gate)

> **Purpose.** Maximise what we de-risk on the **local testnet / hermetic corpus** before
> transitioning to preprod. The C2 guide §7b principle: *exercise each failure class in the
> CHEAPEST venue that can produce it* — a bug found locally costs hours; the same bug in
> preprod costs the **~2-epoch (~10-day) stake-snapshot gate** to retest. Iterate locally first.

## Context (where we are, 2026-06-13)

- **PHASE4-N-AO CLOSED** — `CN-CONS-03` enforced (multi-candidate fork-choice SELECT, NATURAL
  CE-AO-6 pass). §7b **rung 2 fully done** (FOLLOW + SELECT). Producer-accept (`CN-CONS-06`) +
  recover (`RO-LIVE-05`) already enforced.
- **Rung 3 = preprod** is the only RO-LIVE-flipping rung, and it is **stake/time-gated**
  (ADE1 active ~epoch 295 + faucet delegation), **not code-gated**. So: drain everything
  local-exercisable now; touch preprod only when the stake matures.
- Registry at HEAD: **132 open rules** (113 declared + 19 partial). Most CN-\*/T-\* are
  *constitutional* (enforced by lower-level DC rules + existing code, not yet status-flipped);
  several "open" rules already carry substantial tests but lack a CI gate. The genuinely-open,
  locally-exercisable, bounty-relevant work is the three streams below.

## Execution order (STRICT): **Stream 3 → Stream 1 → Stream 2**

Rationale: Stream 3 (hermetic enforcement sweep) sizes the real backlog and isolates
real-gaps from already-enforced bookkeeping **before** we commit to the behavioral work;
it also clears the `CN-META-01` / `RO-REL` "every invariant has mechanical enforcement"
release-eligibility bar cheaply (no venue). Then Stream 1 (the #1 release-blocker), then
Stream 2 (the gap-prone epoch transition).

---

## Stream 3 — Enforcement-mapping sweep  *(FIRST; hermetic, no venue)*

**Goal:** for every declared/partial rule, determine its true state — (a) already enforced
(tests + a real gate) but un-flipped → write/map the gate + flip; (b) a real behavioral gap →
document + route to Stream 1/2 or a new cluster; (c) constitutional (enforced by sub-rules) →
flip with cross-ref; (d) preprod/external-gated → mark out-of-scope. Surface gaps, never hide.

**Why it exists:** rules like `DC-PROTO-02` (45 tests), `DC-CONSENSUS-02` (16), `DC-LEDGER-05`
(9) are tested but `declared`/`partial` with **no `ci_scripts`** — that is bookkeeping, not new
work. Backs `CN-META-01` ("every invariant has a mechanical enforcement point"), `RO-REL-01/03`,
`T-CI-01` ("every true invariant has CI enforcement; no waivers").

**Done =** every open rule is classified; every already-enforced rule has a CI gate + is flipped;
every real gap is documented with its target stream/cluster; the registry status tally reflects
reality (no tested-but-declared rules left without an explicit "real gap" reason).

**Step 1 (this stream's first deliverable):** the enforcement classification of all 132 open
rules — `{flip-now, needs-gate-then-flip, real-gap, constitutional-flip, preprod/external}`.

### Step-1 classification + conservative routing (2026-06-13)

Bar (user-confirmed): **flip a rule ONLY when a gate (or gate+tests) COMPLETELY +
mechanically enforces its statement; never flip an umbrella to look greener.** Finding:
even the well-tested rules mostly need a *structural* gate written first (behavioral test
suites alone don't structurally prevent a future open decoder / non-deterministic transition).

- **FLIP-NOW (10)** — gate+tests, declared. On inspection only **DC-CORE-01** had a complete,
  self-testing gate → **FLIPPED (`c27ee281`)** (`ci_check_no_async_in_blue.sh` scans every BLUE
  core_path + self-tests its detection). The rest are NOT clean: `CN-CONS-02/05` are
  constitutional restatements (gates touch, not full-coverage); `DC-LEDGER-02/03` → Stream 1;
  `DC-EPOCH-01` → Stream 2; `RO-LIVE-02` operator-blocked; `DC-REF-01`/`T-ENC-01` genuinely partial.
- **NEEDS-GATE (18)** — behaviorally well-tested, **no structural gate**. Realistic work = WRITE
  the missing structural gate per rule, then flip. Candidates (declared, rich tests):
  `CN-WIRE-07` (45 — needs a closed-codec-decoder gate over ade_network::codec, mirroring
  `consensus_closed_enums`), `DC-PROTO-01/06` (90/90 — transition determinism/purity gate),
  `DC-PROTO-02` (45 — partly differential "with Haskell"; likely partial), `DC-PROTO-03/04`
  (6/42 — "FULL surface": must verify EVERY listed mini-protocol is implemented+tested before
  flipping), `CN-CONS-04` (13 — header↔body binding gate), `CN-SNAPSHOT-01` (3). Partials routed:
  `CN-LEDGER-09`/`DC-LEDGER-05` → Stream 1 (witness binding); `CN-EPOCH-01`/`DC-LEDGER-04` →
  Stream 2; `DC-DIFF-01` → Stream 1; `DC-CONSENSUS-02`,`CN-CRYPTO-02`,`CN-STORE-02`,`T-ERR-01`,
  `CN-LEDGER-07`(1 test, no locus) stay partial/declared.
- **GAP-or-CONSTITUTIONAL (99)** — conservative: **do NOT flip umbrellas.** Route real behavioral
  gaps to their stream (`DC-NODE-19` → Stream 2; the `CN-PLUTUS-*`/`CN-LEDGER-*` validation rules →
  Stream 1); leave constitutional umbrellas (`CN-LEDGER-03` "matches reference", `CN-REL-*`,
  `T-*` core, `OP-*`, …) `declared` until their sub-work lands. Expected net flips from Stream 3:
  ~10-15, not ~100.
- **PREPROD/EXTERNAL (5)** — defer (`RO-LIVE-01`, `CN-EPOCH-03`, `RO-MITHRIL-IMPORT-01`,
  `RO-SYNC-EVIDENCE-01`, `RO-LIVE-03`).

**Remaining Stream-3 work:** write the missing *structural* gates for the behaviorally-complete
NEEDS-GATE rules (CN-WIRE-07 closed-codec; DC-PROTO determinism/purity/surface; CN-CONS-04
header-binding), verifying each gate self-tests + completely enforces the statement, then flip —
one well-scoped gate at a time. Umbrellas/gaps stay documented + routed, not flipped.

### Stream-3 OUTCOME (2026-06-13) — DONE under the conservative bar

**7 rules flipped declared→enforced (enforced 239→246, declared 113→106)** with **4 new
self-testing structural gates** + 1 existing gate, each verified green on real code AND
catching a synthetic violation:
- `DC-CORE-01` (BLUE sync-only) — existing `ci_check_no_async_in_blue.sh` (already complete).
- `CN-WIRE-07` (closed versioned message type) — NEW `ci_check_codec_message_closed.sh`.
- `CN-CONS-04` (header↔body+context binding) — NEW `ci_check_header_body_binding.sh`.
- `DC-PROTO-03/04` (full N2N/N2C surface) — NEW `ci_check_mini_protocol_surface.sh`.
- `DC-PROTO-01/06` (deterministic + pure transitions) — NEW `ci_check_mini_protocol_transition_purity.sh`.

**Honest residual (NOT flipped — conservative bar):**
- `DC-PROTO-02` (transcript-equivalent *with Haskell*) → differential claim, routed to **Stream 1**.
- All **`partial`** rules stay partial — "partial" means known remaining scope (witness binding
  `CN-LEDGER-09`/`DC-LEDGER-05`, `DC-DIFF-01` → Stream 1; `CN-EPOCH-01`/`DC-LEDGER-04` → Stream 2;
  `DC-CONSENSUS-02`,`CN-CRYPTO-02`,`CN-STORE-02`,`T-ERR-01`); flipping needs the scope closed, not a gate.
- `CN-LEDGER-07` (1 test) + `CN-SNAPSHOT-01` (code_locus "TBD") — not flip-ready (thin / unrecorded locus).
- The ~99 constitutional umbrellas (`CN-LEDGER-03`, `CN-REL-*`, `T-*` core, `OP-*`, …) stay
  `declared` — flipping them would overstate; their enforcement is the SUM of sub-rules + the
  stream work. Real behavioral gaps routed: `DC-NODE-19` → Stream 2; `CN-PLUTUS-*`/`CN-LEDGER-*`
  validation → Stream 1.

**Stream 3 is complete:** every genuinely-complete-but-ungated rule is now gated + flipped; every
other open rule is honestly classified + routed (no umbrella flipped to look greener). Proceed to
Stream 1.

---

## Stream 1 — Differential validation agreement + false-accept hunt  *(SECOND; bounty #1, local)*

**Goal:** Ade's BLUE validation verdict agrees with cardano-node on every tested block/tx, and
**no false-accept** survives an adversarial negative corpus. This is the half of the bounty
`CN-CONS-03` does not cover; recorded priority: *tx-validity agreement > live-following; a
false-accept is release-blocking.*

**Rules:** `DC-LEDGER-03` (validity agrees, all eras — partial), `CN-LEDGER-09` + `DC-LEDGER-05`
(witness binding completeness, incl. required-signer enumeration — partial, no CI), `CN-LEDGER-07/08`
(conservation, no double-spend), `CN-LEDGER-03` (matches reference), `CN-PLUTUS-*` (script
determinism), harden `DC-DIFF-01` (differential harness localizes first divergence).

**Local how:** push a corpus of real preprod blocks/txs **+ adversarial mutations** through Ade's
validation and diff verdicts against a local cardano-node; the adversarial negative corpus IS the
false-accept hunt. Cheapest venue; catches the release-blocker before preprod.

**Done =** validity agreement proven across all supported eras on a named corpus + an adversarial
negative corpus with zero false-accepts; witness-binding completeness (incl. required-signers)
enforced per era with gates.

### Stream-1 progress (2026-06-13)

Foundation confirmed pre-existing: 10,500-block × 7-era differential verdict agreement + a 4-class
admission adversarial false-accept corpus + per-rule tx/conservation/witness adversarial corpora.

- **Slice C — double-spend gap closed** (`91f63195`). `CN-LEDGER-08` ("no input consumed more than
  once") had ZERO tests. Added `adversarial_double_spend_consumed_input_rejects_bad_inputs` (contrast:
  first spend accepted; re-spend of a consumed/absent input → `BadInputs` via `check_inputs_present`)
  + F7 in the shared `adversarial_corpus()` (corpus-wide no-false-accept + replay sweeps). NOT flipped
  — `CN-LEDGER-07/08` are universal ledger invariants; a finite corpus ≠ complete (conservative bar).
- **Slice B — required-signer closure gate** (this commit). New `ci_check_required_signer_closure.sh`
  locks the closed-per-era enumeration: `SignerSource` closed + complete (all 6 sources) + era-fail-
  closed (`UnsupportedEra`), witness/closure-error sums closed; self-testing. Recorded on
  `CN-LEDGER-09`/`DC-LEDGER-05` ci_scripts. NOT flipped — the per-era binding completeness (Byron
  TxWitness / Shelley bootstrap / Alonzo redeemers vs the comprehensively-tested Conway path) is the
  remaining scope; both stay `partial`.
- **Slice A1 — Plutus per-script budget-cap false-accept FIXED** (this commit). **Finding:** Ade's
  phase-2 passed `initial_budget = protocol-max` to aiken's `eval_phase_two_raw`, which caps only the
  tx-wide budget and never the per-script DECLARED ex_units. cardano-ledger caps each script at the
  ex_units its redeemer declares → an UNDER-declared tx (script consumes > declared but < protocol-max)
  was accepted by Ade yet rejected by cardano-node = **false-accept** (release-blocking direction; the
  real-block oracle structurally can't surface it — on-chain valid txs never under-declare). Confirmed
  hermetically (`under_declared_ex_units_must_reject`: declared (1,1), consumed mem=747528). **Fix
  (BLUE):** `ade_plutus::eval_tx_phase_two` derives each redeemer's declared cap from the tx
  (`declared_ex_units_by_pointer`, array + Conway-map forms; walk mirrors the proven
  `ade_ledger::witness` redeemer parse) and binds `PerScriptResult.success` to `actual<=declared`;
  `try_evaluate_tx` rejects (`Failed`) on any `!success`. Both ledger consumers (rules.rs +
  tx_validity) reject. Self-testing gate `ci_check_plutus_budget_cap.sh`. Recorded on CN-PLUTUS-02 +
  DC-LEDGER-03 (kept declared/partial — closed failure-shape classification + full reference diff
  remain; conservative bar).
- **CE-88 boundary (scope AROUND):** the aiken Conway `validity_range` ScriptContext bug is the
  OPPOSITE direction (a false-REJECT / `diverge_fail`, inherited from aiken upstream, Conway `Spend`
  only); it blocks CN-PLUTUS-03 / DC-LEDGER-06 (canonical ScriptContext derivation — Ade delegates it
  to aiken), which stay `declared`. A1 (budget) is independent of CE-88.
- **Slice A3 — adversarial Plutus reject corpus broadened** (this commit). Two new must-reject classes
  ported from aiken's own vetted vectors (v1.1.21): `failing_validator_must_reject` (a Plutus V1 policy
  `func main() Bool { false }`, aiken `test_eval_4`) and `extraneous_redeemer_must_reject` (a redeemer
  with no matching script, aiken `eval_extraneous_redeemer`) — both assert `eval_tx_phase_two` surfaces
  an error → ledger `Failed` → reject. Added to `ci_check_plutus_budget_cap.sh` (the gate now guards the
  whole reject corpus: budget / failing-validator / extraneous-redeemer present + not #[ignore]'d).
  Recorded on CN-PLUTUS-02 + DC-LEDGER-03 (kept declared/partial). Was zero adversarial Plutus reject
  coverage before A1/A3 (the prior corpora are non-Plutus).
- **Slice A2 DONE** (this commit): locked the proven surface with 3 self-testing gates + 1 determinism
  test, all green. **CN-PLUTUS-04 FLIPPED declared→enforced** (`ci_check_plutus_eval_purity.sh`:
  ade_plutus/src forbids wall-clock/rand/env/fs/net/thread/process/HashMap/HashSet; self-tested). New
  `plutus_eval_is_deterministic` (repeat-eval byte-identical) + `ci_check_plutus_oracle_no_false_accept.sh`
  (locks the harness `diverge_pass==0` assertion). **CN-PLUTUS-01 NOT flipped (USER DECISION): stays
  declared** with strengthened evidence — determinism (A) is structurally argued from the purity gate, but
  budget-accounting-matches-Cardano (B) is compatibility evidence needing a corpus-bound manifest.
  CN-PLUTUS-03 / DC-LEDGER-06 stay declared (CE-88); DC-LEDGER-03 stays partial.
- **Slice A4 DONE** (this commit): Plutus result/budget conformance manifest → **CN-PLUTUS-01 FLIPPED
  declared→enforced.** `docs/evidence/plutus-conformance-manifest.toml` binds pinned aiken 42babe5 (vs
  Cargo.lock) + IOG corpus 643ddd13 (content sha256 83e8f447) + repeatable/tamper-tested hash a23bfabc;
  `plutus_conformance_evaluation_suite` now asserts the EXACT outcome (was a floor); gate
  `ci_check_plutus_conformance.sh` (corpus-immutable + aiken-pinned + hash-intact + suite-not-weakened,
  self-tested). Result parity 514/514 runnable = 495 byte-exact + 19 alpha-equivalent (DeBruijn-identical,
  byte-exact ex_units → printer divergence, not semantic mismatch); ex_units 514/514 byte-exact; 214
  classified skips (PV11-inactive / aiken-parser, NOT false-rejects, do NOT discharge CN-PLUTUS-03 /
  DC-LEDGER-06). SCOPE: pinned runnable corpus, not full future-era coverage. **Stream 1 Plutus thread
  (A1+A2+A3+A4) COMPLETE.**
- **Stream 1 remaining = DC-PROTO-02** (transcript-equivalent miniprotocol behavior w/ Haskell, routed
  from S3). **ASSESSED 2026-06-13 — NOT flippable hermetically.** Real-capture byte-identity round-trips
  (decode a real cardano-node capture → re-encode → byte-identical = Ade's codec is on the Haskell wire
  grammar) cover **10/11 surfaces**: N2N chain_sync/block_fetch/keep_alive/peer_sharing/handshake + all 5
  N2C (`crates/ade_network/tests/*_real_capture_corpus.rs`, corpus `corpus/network/{n2n,n2c}/`). **GAP =
  tx_submission2 N2N rich messages**: only `MsgInit` (2 bytes `81 06`) was real-captured — cardano-node
  runs its tx-sub2 RESPONDER only against peers IT dials; Ade dialed IN so the node never opens its tx-sub2
  server (see `corpus/network/n2n/tx_submission2/NOTES.md`). RequestTxIds/ReplyTxIds/RequestTxs/ReplyTxs are
  synthetic-only (`tx_submission2_mempool_trace.rs`). **USER CHOSE OPTION B (next session): build the
  SERVER-SIDE CAPTURE HARNESS** — Ade exposes an inbound listener, the docker preprod cardano-node is
  configured to dial Ade as a `localRoots` peer with real tx traffic so it opens tx-sub2 + sends the rich
  messages, Ade passively records → add the tx_submission2 rich-message real-capture round-trip → write a
  gate enforcing the real-capture transcript corpus across the surface → record on DC-PROTO-02 → flip
  declared→enforced. Then Stream 1 closes → **Stream 2** (epoch transition).
  - **OPTION B DONE (2026-06-14) + it found a REAL codec false-reject → slice TXSUB2-CODEC-REALWIRE
    (DC-PROTO-11 ENFORCED).** New RED server-side harness `ade_tx_submission2_server_capture` (multi-shot;
    handshake responder + chain-sync responder [IntersectFound@node-tip + AwaitReply, coalesced-msg
    handling] + keep-alive + tx-sub CONSUMER role) + hermetic loopback test. Live against the docker
    **public-preprod** node 11.0.1: node dials Ade (172.17.0.1:3101 via topology.json localRoots), V15
    handshake, promotes Ade to a stable HOT peer, opens tx-sub2 and — as the CLIENT/provider — sent
    `MsgInit` + `MsgReplyTxIds`. **Finding:** the node's `MsgReplyTxIds` real wire form is
    `[1, 9f [[6,h'..32'],size] ff]` — an **indefinite-length entries array** + **era-tagged txids
    `[eraIdx,hash]`** — BOTH of which Ade's codec FALSE-REJECTED ("indefinite-length array not allowed").
    A real Cardano incompat the synthetic tests (definite arrays + bare txids) missed. **Fixed (BLUE,
    byte-authority): `TxSubmissionTxId{era,id}`; decode accepts definite+indefinite; encode reproduces the
    indefinite form → captured frame re-encodes BYTE-IDENTICAL; era tag preserved (no strip/guess).**
    Enforced by codec unit tests (byte-identity on the captured frame + bare/wrong-len/unterminated
    negatives) + `tx_submission2_real_capture_corpus` round-trip + `ci/ci_check_tx_submission2_real_capture.sh`
    (self-testing). Captured `ReplyTxIds`/`Init` frames committed as the regression corpus.
  - **DC-PROTO-02 stays `declared`** (strengthened_in += TXSUB2-CODEC-REALWIRE): the FLIP awaits the
    **live full exchange** `ReplyTxIds → RequestTxs → ReplyTxs` (AC #6) — blocked on a public-preprod
    mempool tx (off-peak lull). Harness left running to land `MsgReplyTxs` + the transcript-equivalence
    gate; flip then. **Harness note:** the contiguous Plutus verdict harness currently stops early on a
  cert-state divergence (`StakeAlreadyRegistered`/`StakeNotRegistered` at blocks 1/1/40), so it reaches
  ~0 passing Plutus txs — a pre-existing limitation that weakens the Plutus oracle's reach; worth a
  dedicated look in the broader Stream-1 false-accept hunt.

---

## Stream 2 — Epoch transition  *(THIRD; rung-1 remainder, local single-producer)*

**Goal:** a single-producer C2-LOCAL run crosses an epoch boundary (slot 2000) with adoption
continuing — exercising nonce roll, stake re-snapshot, leadership recompute, reward calc live.
The C2 guide flags this as *"the path most likely to surface a gap — find it here, not in preprod."*

**Rules:** `DC-NODE-19` (forge-loop continuation past EOF — declared, 0 tests), `DC-LEDGER-04`
(epoch-boundary stake/rewards match Haskell — partial, no CI), `CN-EPOCH-01`/`DC-EPOCH-01`
(activation timing, governance enactment atomic at boundary).

**Local how:** the reusable single-producer harness (`~/.cardano-rung1-host/rung1-auto.sh`); run
past slot 2000; verify Ade rolls the nonce / re-snapshots stake / recomputes leadership and keeps
forging + being adopted across the boundary. Non-promotable (flips no RO-LIVE rule), but high
gap-risk.

**Done =** one epoch crossed with adoption continuing; epoch-boundary computations differ-checked
against cardano-node; `DC-NODE-19` exercised + enforced.

---

## Out of scope here (preprod / external-gated — cannot drain locally)

`RO-LIVE-01` (preprod block accepted → `Ba02Manifest`), `CN-EPOCH-03` (canonical stake-snapshot
derivation at scale), `RO-SYNC-EVIDENCE-01`, `RO-MITHRIL-IMPORT-01` / `RO-GENESIS-REPLAY-01`
(external/import), `DC-NODE-17` (real observed-peer advancement), `RO-LIVE-02`, and the
`CN-NET`/`OP-NET` topology-diversity rules (need a real multi-region network). These wait for
the preprod pass or dedicated external work.
