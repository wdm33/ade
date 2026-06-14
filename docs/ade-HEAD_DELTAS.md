# Ade — HEAD Deltas (Changes Since Baseline)

> **Status:** Living architectural document. Regenerated; not hand-edited.
>
> Regenerate with `/head-deltas <baseline>` after every cluster close. Baseline is recorded in `.idd-config.json` `head_deltas_baseline`.

> Baseline: `862cd2cb` (PHASE4-N-AO close — flip `CN-CONS-03` enforced on the natural CE-AO-6 transcript, 2026-06-13 17:05)
> HEAD: `0887b2ad` (flip `DC-PROTO-02` enforced — live tx-submission2 full exchange closes the last surface, 2026-06-14 10:54)
> Span: **the pre-preprod local-first enforcement-mapping window** — three threads run in the planned strict order (3 → 1 → 2): **Stream 3** (mechanical enforcement of the wire-FSM / codec / BLUE-sync surface — `DC-CORE-01`, `CN-WIRE-07`, `CN-CONS-04`, `DC-PROTO-03/04`, `DC-PROTO-01/06` all flipped `declared → enforced`), the **Stream 1 Plutus thread** (a real false-accept fix + the IOG-conformance manifest — `CN-PLUTUS-04` + `CN-PLUTUS-01` flipped) plus the Stream 1 ledger-coverage slices B/C, and the **tx-submission2 real-wire codec slice** (`TXSUB2-CODEC-REALWIRE`: new rule `DC-PROTO-11` enforced + `DC-PROTO-02` flipped `declared → enforced` on a live server-side full-exchange capture). The window opens right after the PHASE4-N-AO functional close (`862cd2cb`), so it also carries the trailing N-AO close housekeeping (grounding-doc regen + cluster-doc archive).
> **18 commits** (no merges), **51 files changed, +4182 / −6652 lines** (the large deletion count is the SEAMS regeneration at the head of the span — `−5672` in `docs/ade-SEAMS.md` alone). **This span TOUCHES BLUE and adds +2 BLUE canonical types** (`462 → 464`): `TxSubmissionTxId` (the era-tagged txid wire type — the `DC-PROTO-11` codec fix, `ade_network::codec::tx_submission`) and `RedeemerFields` (the per-script ex_units-cap fix, `ade_plutus::tx_eval`; private `struct`, counted by the mechanical grep). **+10 new CI gates, 0 modified, 0 removed** (173 → 183). **Registry 372 → 373** (+1 new rule `DC-PROTO-11`, enforced; **10 declared→enforced flips**; +1 strengthening `DC-PROTO-02 += TXSUB2-CODEC-REALWIRE`; **zero removals**). **One new RED capture bin** (`ade_tx_submission2_server_capture`) and a new **real-capture corpus** under `corpus/network/n2n/tx_submission2/`. **No new library module, no new crate** (still 11). **No `[features]` table, no `cfg(feature)`, no `compile_error!`, no new CLI flag.**

> **Baseline note (load-bearing — read before §0).** This window's baseline is **`862cd2cb`**, the PHASE4-N-AO
> cluster-close (it flipped `CN-CONS-03 → enforced`), and it is **valid**: `git rev-parse 862cd2cb` resolves and
> `git merge-base 862cd2cb HEAD == 862cd2cb` (it is a strict ancestor of HEAD; `862cd2cb` carries no tag). HEAD is
> **`0887b2ad`** (the `DC-PROTO-02` flip). The PHASE4-N-AO close's grounding-doc regen, cluster-doc archive, and the
> rung-2 c2-guide note land in the FIRST two commits of this span (`388d8073`, `5532ddf4`) — they belong to the *prior*
> cluster's close but were committed just after the `862cd2cb` functional close, so they appear here as in-range churn
> (they explain the 13 `R100` cluster-doc renames into `docs/clusters/completed/PHASE4-N-AO/` and the bulk of the
> grounding-doc line churn). The substantive NEW work of this window is the three local-first threads scoped by
> `86252176` ("pre-preprod local-first work streams + strict order 3→1→2").
>
> **Working-tree note.** At this regen the working tree is **CLEAN** for tracked files — `git status --porcelain` shows
> only untracked scratch (`.mithril-scratch/`, `wire_smoke.jsonl`), neither part of this doc. §1 narrates the committed
> span `862cd2cb..0887b2ad` verbatim from `git log`; §0/§6/§7 read rule **status** and canonical-type counts from the
> registry / BLUE trees at HEAD `0887b2ad` (`DC-PROTO-02` **enforced**, **373** rules, **464** BLUE canonical types).
> **NB:** the *committed* `.idd-config.json` baseline reads `862cd2cb` (this window's value, already advanced by the
> prior regen); the post-`0887b2ad` baseline bump to `0887b2ad` is the named follow-on this regen performs.

This window is **not a cluster** in the slice-doc sense — it is a **pre-preprod local-first enforcement-mapping pass**
scoped by `86252176`, ordered strictly **3 → 1 → 2** to flip as many `declared` rules to `enforced` as the existing code
already justifies (mechanical gates over already-shipped behavior) before any preprod operator pass. It decomposes into
four threads:

1. **Stream 3 — wire-FSM / codec / BLUE-sync enforcement (6 flips).** Six rules that were already structurally true in
   the shipped codebase but lacked a mechanical gate are now `enforced`: `CN-WIRE-07` (closed codec-message taxonomy,
   `ci_check_codec_message_closed.sh`), `CN-CONS-04` (header/body binding, `ci_check_header_body_binding.sh`),
   `DC-PROTO-03` + `DC-PROTO-04` (the mini-protocol surface, `ci_check_mini_protocol_surface.sh`), `DC-PROTO-01` +
   `DC-PROTO-06` (mini-protocol FSM transition purity, `ci_check_mini_protocol_transition_purity.sh`), and `DC-CORE-01`
   (BLUE is sync-only — `declared → enforced` against the existing `ci_check_no_async_in_blue.sh`, gate-complete). All
   gate-only / docs-only — **no production code changed in Stream 3**.
2. **Stream 1 Plutus thread — a real false-accept fix + IOG conformance (2 flips).** `ade_plutus::tx_eval` gained a
   per-script declared-`ex_units` cap that closes a Plutus false-accept (the A1 `fix`); the adversarial reject corpus was
   broadened (A3); the host-environment purity gate `ci_check_plutus_eval_purity.sh` flips `CN-PLUTUS-04 → enforced`
   (A2); and a registry-bound IOG `plutus-conformance` manifest (`docs/evidence/plutus-conformance-manifest.toml`) +
   `ci_check_plutus_conformance.sh` flips `CN-PLUTUS-01 → enforced` (A4 — 514/514 result + ex_units parity over the
   pinned runnable corpus).
3. **Stream 1 ledger coverage (slices B, C — gate + corpus, NO flip).** A required-signer closure gate
   (`ci_check_required_signer_closure.sh`, attached to the still-`partial` `DC-LEDGER-05`) locks the witness
   enumeration, and a double-spend adversarial corpus broadens `ade_ledger` coverage. **Neither flips its named rule** —
   `DC-LEDGER-05` stays `partial` and `CN-LEDGER-08` stays `declared` (see the anomaly note in §7).
4. **tx-submission2 real-wire codec (`TXSUB2-CODEC-REALWIRE`) — +1 new rule + 1 flip.** A new RED server-side capture
   harness (`ade_tx_submission2_server_capture`, option B: Ade listens, the docker preprod cardano-node 11.0.1 dials Ade
   as a `localRoots` peer and, as the tx-submission2 provider, sends its real mempool messages) surfaced a real Cardano
   incompatibility the synthetic round-trip tests had missed: the node's `MsgReplyTxIds` uses era-tagged txids
   (`[6, h'..32']` for Conway) inside CBOR **indefinite**-length arrays, which Ade's prior codec FALSE-REJECTED. The fix
   (`DC-PROTO-11`, new, enforced) makes the codec accept + byte-identically preserve that real wire form; with the live
   full exchange (Init → RequestTxIds → ReplyTxIds → RequestTxs → ReplyTxs) captured, **`DC-PROTO-02` flips
   `declared → enforced`** — the last of the 11 N2N+N2C mini-protocol surfaces now has a real-capture byte-identical
   round-trip.

### The Stream 3 enforcement band (6 flips, `enforced` — gate/docs only)

> **Stream 3 flips rules that the shipped code already satisfied; it adds the missing mechanical gate, not new
> behavior.** No production `.rs` changed in Stream 3 — each gate greps the existing wire-FSM / codec / BLUE trees for
> the property and fails closed on a violation. The classification + conservative routing that scoped which rules were
> safe to flip is `b01d1d38`.

- **`CN-WIRE-07` (enforced) — closed codec-message taxonomy.** Versioned, closed per-protocol message enums (no runtime
  meaning negotiation). New gate `ci_check_codec_message_closed.sh` (`8e8f8eb0`).
- **`CN-CONS-04` (enforced) — header/body binding.** A block's body binds to its header (Cardano header/body
  separation). New gate `ci_check_header_body_binding.sh` (`37d4c068`).
- **`DC-PROTO-03` + `DC-PROTO-04` (enforced) — mini-protocol surface.** The closed mini-protocol surface (the 11
  N2N+N2C protocols' codec + transition modules). New gate `ci_check_mini_protocol_surface.sh` (`7f13d646`).
- **`DC-PROTO-01` + `DC-PROTO-06` (enforced) — mini-protocol FSM transition purity.** Each protocol's `transition` is a
  pure total reducer (no I/O, no nondeterminism). New gate `ci_check_mini_protocol_transition_purity.sh` (`378ca2ca` —
  "Stream 3 complete").
- **`DC-CORE-01` (enforced) — BLUE is sync-only.** No `async` in BLUE. Flipped `declared → enforced` against the
  EXISTING `ci_check_no_async_in_blue.sh` (gate-complete; docs/registry-only, `c27ee281`).

### The Stream 1 Plutus band (`CN-PLUTUS-04`, `CN-PLUTUS-01` enforced; 1 BLUE production fix)

> **The A1 fix is a real Plutus false-accept closure in BLUE `ade_plutus`** — a per-script declared-`ex_units` cap. The
> A2/A4 gates then flip the two Plutus rules the now-fixed evaluator justifies. The conformance flip (A4) is
> registry-bound to a pinned IOG corpus + pinned aiken commit so the gate asserts the EXACT outcome.

- **A1 `fix` (`ed408410`) — per-script declared `ex_units` cap.** `ade_plutus::tx_eval` (`tx_eval.rs` **+282 / −28**)
  now caps each script's declared `ex_units`, closing a Plutus false-accept; a small `ade_ledger::plutus_eval` touch
  (**+9**) threads it. New private type `RedeemerFields` (see §6). End-to-end test coverage added
  (`end_to_end_plutus_eval.rs` **+183**).
- **A3 `test` (`b25d1594`) — broaden the adversarial Plutus reject corpus.** Negative coverage so the cap + purity hold
  fail-closed.
- **A2 `feat(ci)` (`dec0fd22`) — `CN-PLUTUS-04 → enforced`.** `ci_check_plutus_eval_purity.sh` mechanically forbids
  wall-clock / `rand` / env / fs / net / thread / process / `HashMap` / `HashSet` anywhere in `ade_plutus/src`
  (self-tested). (Plus `ci_check_plutus_budget_cap.sh` + `ci_check_plutus_oracle_no_false_accept.sh` ship in this
  thread.)
- **A4 `feat(ci)` (`717febaa`) — `CN-PLUTUS-01 → enforced`.** A registry-bound conformance manifest
  (`docs/evidence/plutus-conformance-manifest.toml`, **+55**) pins aiken `42babe5` + the IOG plutus-conformance corpus
  `643ddd13` (content sha256 `83e8f447`); `ci_check_plutus_conformance.sh` asserts the exact outcome (514/514 result +
  byte-exact ex_units parity over runnable cases; 19 alpha-equivalent printer divergences classified as printer-only).
  `crates/ade_testkit/tests/plutus_conformance.rs` extended (**+33 / −11**).

### The Stream 1 ledger-coverage band (slices B, C — gate + corpus, NO flip)

> **Slices B and C broaden enforcement and coverage but do NOT flip their named rules.** The required-signer gate is
> attached to the still-`partial` `DC-LEDGER-05`; the double-spend corpus broadens `ade_ledger` coverage but
> `CN-LEDGER-08` remains `declared` (empty `code_locus` / `tests` / `ci_script`). The commit subject for slice C reads
> "close the CN-LEDGER-08 gap"; the registry status did **not** change — see the §7 anomaly note.

- **Slice B `feat(ci)` (`55a8a7e1`) — required-signer closure gate.** `ci_check_required_signer_closure.sh` locks the
  witness enumeration; attached to `DC-LEDGER-05` (stays `partial`) and cross-referenced by `CN-LEDGER-08` /
  `CN-LEDGER-09` (both stay `declared`).
- **Slice C `test` (`91f63195`) — double-spend adversarial coverage.** `conway_conservation_adversarial.rs` (**+37**)
  broadens the no-double-spend negative corpus. `CN-LEDGER-08` status unchanged (`declared`).

### The tx-submission2 real-wire band (`TXSUB2-CODEC-REALWIRE` — `DC-PROTO-11` new + `DC-PROTO-02` flip)

> **A live server-side capture (option B) found a real Cardano wire-form incompatibility the synthetic round-trips had
> missed** — the canonical "real interop finds codec bugs" finding. Ade's prior tx-submission2 codec used definite
> arrays + bare 32-byte txids; the real cardano-node 11.0.1 sends era-tagged txids (`[6, h'..']`) inside indefinite
> arrays and FALSE-REJECTED. The fix is a BLUE codec change adding one new wire type; the flip rests on a committed
> real-capture corpus + a byte-identity unit test.

- **`92b855c4` `feat(ade_network)` — tx-submission2 codec on the real wire form (`DC-PROTO-11`, new, enforced).**
  `ade_network::codec::tx_submission` (`tx_submission.rs` **+257 / −52**) gains `TxSubmissionTxId` (era-tag +
  `hash32`), `decode_seq` accepting BOTH definite and indefinite sequences, and indefinite-form encode so a captured
  frame re-encodes byte-identically; the era tag is preserved (never stripped/guessed). Small `event.rs` / `transition.rs`
  touches thread it. New RED capture harness `ade_tx_submission2_server_capture`
  (`src/bin/capture_tx_submission2_server.rs` **+603**) + a hermetic loopback smoke test
  (`tx_submission2_server_capture_loopback.rs` **+258**). New real-capture corpus
  (`corpus/network/n2n/tx_submission2/preprod_server_txsub_{init,reply_txids,reply_txs}_00_recv.cbor`) + the round-trip
  test (`tx_submission2_real_capture_corpus.rs` **+135**). New gate `ci_check_tx_submission2_real_capture.sh`.
- **`0887b2ad` `feat(ade_network)` — `DC-PROTO-02 → enforced`.** With the live full exchange
  (Init → RequestTxIds → ReplyTxIds → RequestTxs → ReplyTxs) captured and round-tripping byte-identically, the last
  N2N+N2C mini-protocol surface is closed. `DC-PROTO-02` flips `declared → enforced` and gains
  `strengthened_in += TXSUB2-CODEC-REALWIRE`; its gate becomes `ci_check_tx_submission2_real_capture.sh`. A small
  `ade_core_interop` ingress-test touch (**+9 / −3**) and a `tx_submission2_mempool_trace.rs` touch (**±5**) accompany it.

### Trailing PHASE4-N-AO close housekeeping (`388d8073`, `5532ddf4`)

- **`388d8073` `docs(phase4-n-ao)` — cluster-close housekeeping.** Regenerated the four grounding docs to the N-AO HEAD,
  archived the cluster (the 13 `R100` renames `docs/clusters/PHASE4-N-AO/* → docs/clusters/completed/PHASE4-N-AO/*`), and
  groomed the registry. This is the bulk of the grounding-doc line churn in this span (`docs/ade-SEAMS.md` **−5672**,
  `docs/ade-CODEMAP.md`, `docs/ade-TRACEABILITY.md`, `docs/ade-invariant-registry.toml`).
- **`5532ddf4` `docs(c2-guide)` — rung 2 fully closed.** Records `CN-CONS-03` enforced (PHASE4-N-AO) in the C2 preprod
  tip guide.

**BLUE was TOUCHED this span (+2 canonical types).** Unlike the prior PHASE4-N-AO window (which was GREEN+RED only),
this window changed two BLUE trees: `ade_network::codec::tx_submission` (the `DC-PROTO-11` real-wire fix, +`TxSubmissionTxId`)
and `ade_plutus::tx_eval` (the A1 ex_units-cap fix, +`RedeemerFields`). BLUE canonical types **462 → 464** (§6). **No
`RO-LIVE` rule flipped** — every flip is a `CN`/`DC`-family enforcement-mapping flip over already-shipped behavior or the
new tx-submission2 fix; no preprod operator pass ran this window. The headline flips are **`DC-PROTO-02`** (the last wire
surface) and the six Stream 3 flips.

## 0. Headline

| Count | Baseline (`862cd2cb`) | HEAD (`0887b2ad`) | Δ |
|---|---|---|---|
| CI gates (`ci/ci_check_*.sh`) | 173 | **183** | **+10** new (Stream 3: `codec_message_closed`, `header_body_binding`, `mini_protocol_surface`, `mini_protocol_transition_purity`; Plutus: `plutus_eval_purity`, `plutus_conformance`, `plutus_budget_cap`, `plutus_oracle_no_false_accept`; ledger: `required_signer_closure`; txsub2: `tx_submission2_real_capture`). **0 modified in place**, **0 removed** (`--diff-filter=M`/`-D` over `ci/` both empty; `ls ci/ci_check_*.sh \| wc -l` = 173 → 183). |
| Registry rules (`docs/ade-invariant-registry.toml`) | 372 | **373** | **+1** new rule `DC-PROTO-11` (the tx-submission2 real-wire fix). **Zero removed** (`comm -23` of the sorted `id =` lists is empty — exactly one addition). |
| Registry status (enforced / enforced_scaffolding / partial / declared) | 239 / 1 / 19 / 113 | **250 / 1 / 19 / 103** | **+11 enforced**, **−10 declared** (`enforced_scaffolding=1`, `partial=19` unchanged). Reconciliation: the 1 new rule (`DC-PROTO-11`) closes `enforced` (+1 enforced, net 0 declared) **and** 10 prior-`declared` rules flipped `enforced` this span (+10 enforced, −10 declared). Net: +11 enforced, −10 declared. |
| **`DC-PROTO-02` (tx-submission2 surface)** | `declared` | **`enforced`** | **THE headline flip.** `strengthened_in` `["PHASE4-N-A"]` → `["PHASE4-N-A","TXSUB2-CODEC-REALWIRE"]`; gate `→ ci_check_tx_submission2_real_capture.sh`; enforced on the live full-exchange real-capture corpus. |
| Declared → enforced flips (count) | — | **10** | `DC-PROTO-01`, `DC-PROTO-02`, `DC-PROTO-03`, `DC-PROTO-04`, `DC-PROTO-06`, `CN-CONS-04`, `CN-WIRE-07`, `DC-CORE-01` (Stream 3 + txsub2) + `CN-PLUTUS-01`, `CN-PLUTUS-04` (Plutus). No rule weakened; no rule removed. |
| Registry strengthenings | — | **+1** | `strengthened_in += TXSUB2-CODEC-REALWIRE` on **`DC-PROTO-02`** (the only `strengthened_in` change this span). |
| BLUE canonical types | 462 | **464** | **+2** — **the BLUE tree IS touched this span** (unlike N-AO). `+TxSubmissionTxId` (`pub struct`, `ade_network::codec::tx_submission` — the `DC-PROTO-11` era-tagged txid wire type) and `+RedeemerFields` (private `struct`, `ade_plutus::tx_eval` — the A1 ex_units-cap fix; counted by the mechanical `^(pub )?(struct\|enum)` grep, the same metric CODEMAP uses). Still 11 crates (no new crate; the only `Cargo.toml` change is a new `[[bin]]` target). |
| Grounding docs | CODEMAP / SEAMS / TRACEABILITY all regenerated to **`862cd2cb`** by the in-span PHASE4-N-AO close (`388d8073`): 462 canonical types / 173 CI / 372 rules. | Now **one window stale**: none carries the new rule `DC-PROTO-11`, the +2 BLUE types, the new bin, the new corpus, or the 10 new CI gates (`grep -c DC-PROTO-11` / `ci_check_tx_submission2_real_capture` / `capture_tx_submission2_server` in each = 0); their pins read 173 CI / 372 rules / 462 types vs. HEAD 183 / 373 / 464. | **CODEMAP + SEAMS + TRACEABILITY are now ONE window STALE** — they MISS the new rule, the +2 BLUE types, the 10 declared→enforced flips, the new RED bin + corpus, and the 10 new CI gates. The registry holds all of it authoritatively at HEAD (**373 rules / 250 enforced**); the refresh to `0887b2ad` is the named follow-on. See the cross-reference warnings at the end of §2 and §5. |

> **Grounding-doc state this regen (load-bearing).** **CODEMAP, SEAMS, and TRACEABILITY were all regenerated to
> `862cd2cb`** (this window's BASELINE — by the in-span PHASE4-N-AO close commit `388d8073`), so they pin to `862cd2cb`
> / 462 types / 173 CI / 372 rules and carry everything through PHASE4-N-AO but **nothing from this window's three
> threads**. They are now **one window stale**: this window introduced **1 new rule** (`DC-PROTO-11`), **+2 BLUE
> canonical types**, **10 declared→enforced flips**, **1 new RED bin** + a **real-capture corpus**, and **10 new CI
> gates** — none of which appear in CODEMAP/SEAMS/TRACEABILITY (`grep -c` for the new rule / bin / corpus / each new gate
> = 0; CI pin 173 vs. HEAD 183; rule pin 372 vs. HEAD 373; type pin 462 vs. HEAD 464). The invariant registry holds all
> of it authoritatively at HEAD (**373 rules**). **Action:** regenerate CODEMAP + SEAMS + TRACEABILITY to `0887b2ad` as
> a follow-on so the new rule with its gate, the +2 BLUE types, the 10 flips, the new bin/corpus, and the 10 new gates
> all appear, and all three docs pin to this HEAD. Until then the registry is authoritative for the new bindings.

The thread↔rule↔gate map for this window (the full verbatim log is §1):

| Thread / slice | Rule(s) | Gate | What shipped |
|---|---|---|---|
| **N-AO close housekeeping** (`388d8073`, `5532ddf4`) | — (prior cluster) | — | Regenerate grounding docs to `862cd2cb` + archive N-AO cluster docs (13 `R100` renames) + groom registry; c2-guide rung-2 note. |
| **scope** (`86252176`, `b01d1d38`, `04a857a3`) | — (planning) | — | Pre-preprod local-first work streams + strict order (3→1→2); Stream-3 classification + conservative routing; DC-PROTO-02 assessment + option-B routing. |
| **Stream 3** (`c27ee281`) | **`DC-CORE-01`** (→ enforced) | `ci_check_no_async_in_blue.sh` (existing — gate-complete) | BLUE sync-only flip (docs/registry only; no new gate). |
| **Stream 3** (`8e8f8eb0`) | **`CN-WIRE-07`** (→ enforced) | `ci_check_codec_message_closed.sh` (NEW) | Closed codec-message taxonomy gate. |
| **Stream 3** (`37d4c068`) | **`CN-CONS-04`** (→ enforced) | `ci_check_header_body_binding.sh` (NEW) | Header/body binding gate. |
| **Stream 3** (`7f13d646`) | **`DC-PROTO-03`** + **`DC-PROTO-04`** (→ enforced) | `ci_check_mini_protocol_surface.sh` (NEW) | Mini-protocol surface gate. |
| **Stream 3** (`378ca2ca`) | **`DC-PROTO-01`** + **`DC-PROTO-06`** (→ enforced) | `ci_check_mini_protocol_transition_purity.sh` (NEW) | FSM transition-purity gate; Stream 3 complete. |
| **Stream 1 / A1** (`ed408410`) | (Plutus false-accept fix) | — | **BLUE fix** in `ade_plutus::tx_eval` — per-script declared ex_units cap (+`RedeemerFields`). |
| **Stream 1 / A3** (`b25d1594`) | (Plutus negative corpus) | — | Broaden the adversarial Plutus reject corpus. |
| **Stream 1 / A2** (`dec0fd22`) | **`CN-PLUTUS-04`** (→ enforced) | `ci_check_plutus_eval_purity.sh` (NEW) | Host-environment purity gate (+`plutus_budget_cap`, `plutus_oracle_no_false_accept`). |
| **Stream 1 / A4** (`717febaa`) | **`CN-PLUTUS-01`** (→ enforced) | `ci_check_plutus_conformance.sh` (NEW) | Registry-bound IOG conformance manifest gate. |
| **Stream 1 / B** (`55a8a7e1`) | `DC-LEDGER-05` (stays `partial`) | `ci_check_required_signer_closure.sh` (NEW) | Required-signer closure gate — locks the witness enumeration. **No flip.** |
| **Stream 1 / C** (`91f63195`) | `CN-LEDGER-08` (stays `declared`) | — | Double-spend adversarial coverage. **No flip** (see §7 anomaly). |
| **TXSUB2-CODEC-REALWIRE** (`92b855c4`) | **`DC-PROTO-11`** (NEW, → enforced) | `ci_check_tx_submission2_real_capture.sh` (NEW) | tx-submission2 codec on the real wire form (era-tagged txid + indefinite arrays); +`TxSubmissionTxId`; new RED server capture bin + corpus. |
| **TXSUB2-CODEC-REALWIRE** (`0887b2ad`) | **`DC-PROTO-02`** (→ enforced) | `ci_check_tx_submission2_real_capture.sh` | **Flip `DC-PROTO-02`** — live full exchange closes the last surface (`strengthened_in += TXSUB2-CODEC-REALWIRE`). |

## 1. Commit Log (newest first)

| Hash | Type | Summary |
|------|------|---------|
| `0887b2ad` | feat | feat(ade_network): flip DC-PROTO-02 enforced -- live tx-submission2 full exchange closes the last surface |
| `92b855c4` | feat | feat(ade_network): tx-submission2 codec on cardano-node's real wire form (era-tagged txid + indefinite arrays) |
| `04a857a3` | docs | docs(planning): DC-PROTO-02 assessment + option-B routing (Stream 1) |
| `717febaa` | feat | feat(ci): Plutus conformance manifest -> flip CN-PLUTUS-01 enforced (Stream 1 / slice A4) |
| `dec0fd22` | feat | feat(ci): host-environment purity gate -> flip CN-PLUTUS-04 enforced (Stream 1 / slice A2) |
| `b25d1594` | test | test(ade_plutus): broaden the adversarial Plutus reject corpus (Stream 1 / slice A3) |
| `ed408410` | fix | fix(ade_plutus): per-script declared ex_units cap -- close a Plutus false-accept (Stream 1 / slice A1) |
| `55a8a7e1` | feat | feat(ci): required-signer closure gate -- lock the witness enumeration (Stream 1 / slice B) |
| `91f63195` | test | test(ade_ledger): double-spend adversarial coverage -- close the CN-LEDGER-08 gap (Stream 1 / slice C) |
| `378ca2ca` | feat | feat(ci): FSM transition-purity gate -> flip DC-PROTO-01/06 enforced; Stream 3 complete |
| `7f13d646` | feat | feat(ci): mini-protocol surface gate -> flip DC-PROTO-03/04 enforced (Stream 3) |
| `37d4c068` | feat | feat(ci): header-body binding gate -> flip CN-CONS-04 enforced (Stream 3) |
| `8e8f8eb0` | feat | feat(ci): closed-codec-message gate -> flip CN-WIRE-07 enforced (Stream 3) |
| `b01d1d38` | docs | docs(planning): Stream-3 classification + conservative routing |
| `c27ee281` | docs | docs(registry): flip DC-CORE-01 enforced (Stream 3) -- BLUE sync-only, gate-complete |
| `86252176` | docs | docs(planning): pre-preprod local-first work streams + strict order (3->1->2) |
| `5532ddf4` | docs | docs(c2-guide): rung 2 fully closed -- CN-CONS-03 enforced (PHASE4-N-AO) |
| `388d8073` | docs | docs(phase4-n-ao): cluster-close housekeeping -- regenerate grounding docs + archive + groom registry |

No merge commits in the span. **18 commits, zero unclassified** — every subject carries an explicit conventional-commits
prefix: **`feat`×9**, **`docs`×6**, **`test`×2** (`b25d1594` the Plutus reject corpus, `91f63195` the double-spend
corpus), **`fix`×1** (`ed408410` the Plutus ex_units-cap false-accept fix). The substantive production code lands in the
two `ade_network` `feat`s (the txsub2 real-wire codec + flip) and the `fix` (the Plutus cap); the other seven `feat`s are
`feat(ci)` gate-only commits (the 5 Stream 3 gates + the 2 Plutus gates A2/A4 + the slice-B required-signer gate), the two
`test`s are corpus-only, and the six `docs` are planning / N-AO close housekeeping.

> **Note (commit-attribution policy).** Per this repo's `CLAUDE.md` override (vibe-coded-node bounty trailer
> requirement), commits in this repo carry a `Co-Authored-By:` model-attribution trailer; that is an Ade-local override
> of the global no-AI-attribution rule and applies to **commit messages only**. It does not affect this doc's content.

## 2. New Modules

**No new library module and no new crate this window.** `git diff --diff-filter=A --name-only 862cd2cb..HEAD --
'crates/**/*.rs'` lists three new files — but **none is a `src/` library module**: one is a RED capture **binary**
(`src/bin/`) and two are **test files** (`tests/`). `git diff --diff-filter=A '**/Cargo.toml'` is empty (still **11
crates**); there is **no new BLUE module** (the BLUE deltas this span are new types inside EXISTING modules — §6).

| Added file | Color | Purpose | Path | Added in |
|--------|-------|---------|---------------|----------|
| `ade_tx_submission2_server_capture` (bin) | **RED** | Server-side N2N TxSubmission2 (protocol 4) capture harness (option B): Ade LISTENS, the operator adds Ade to the node's `localRoots`, the node dials Ade and — as the tx-submission2 provider/CLIENT — sends `MsgInit` then its real mempool (`MsgReplyTxIds` / `MsgReplyTxs`) while Ade plays the SERVER/consumer; captures the node-originated rich frames to the corpus. | `crates/ade_network/src/bin/capture_tx_submission2_server.rs` (+603). | `TXSUB2-CODEC-REALWIRE` |
| `tx_submission2_real_capture_corpus` (test) | **test** | Decodes each captured real cardano-node 11.0.1 tx-submission2 frame, re-encodes, and asserts byte-identical round-trip (the `DC-PROTO-11` / `DC-PROTO-02` proof against the Haskell wire grammar). | `crates/ade_network/tests/tx_submission2_real_capture_corpus.rs` (+135). | `TXSUB2-CODEC-REALWIRE` |
| `tx_submission2_server_capture_loopback` (test) | **test** | Hermetic Ade-vs-Ade loopback smoke for the server capture harness (handshake responder + SERVER role + request decisions + frame capture are internally consistent; does NOT prove Haskell-wire conformance — that is the real-capture corpus test). | `crates/ade_network/tests/tx_submission2_server_capture_loopback.rs` (+258). | `TXSUB2-CODEC-REALWIRE` |

The new capture bin is registered as a `[[bin]]` target in `crates/ade_network/Cargo.toml` (the **only** manifest change
this span — name `ade_tx_submission2_server_capture`, path `src/bin/capture_tx_submission2_server.rs`; not a dependency,
not a feature flag). A new **real-capture corpus** was also added (not source):
`corpus/network/n2n/tx_submission2/preprod_server_txsub_{init,reply_txids,reply_txs}_00_recv.cbor` (+ a `NOTES.md`
update).

> **Cross-reference (CODEMAP) — the new RED bin is NOT yet registered; CODEMAP is stale.** Neither
> `capture_tx_submission2_server` nor the new corpus appears in `docs/ade-CODEMAP.md` (`grep -c` = **0**) — CODEMAP is
> pinned to `862cd2cb` (this window's baseline, the N-AO close), before the txsub2 thread. This is a real staleness flag:
> CODEMAP/SEAMS/TRACEABILITY have not been regenerated for this window. **Action:** run `/codemap` (and `/seams`,
> `/traceability`) to `0887b2ad` so the new RED capture bin + corpus land in CODEMAP §RED with the rest of the
> `ade_network` capture harnesses, and so the +2 BLUE types (`TxSubmissionTxId`, `RedeemerFields`) land in the
> canonical-type tables, before relying on CODEMAP for the tx-submission2 / Plutus surfaces.

## 3. Modules Modified

The production work modified **`ade_network::codec::tx_submission`** (the BLUE real-wire fix — the bulk),
**`ade_plutus::tx_eval`** (the BLUE Plutus ex_units-cap fix), a small **`ade_ledger::plutus_eval`** touch, and the
**`ade_network::tx_submission`** event/transition pair. The rest of the span churn is the Stream 3 / Plutus test
extensions, the new corpus + manifest, the planning docs, and the in-span N-AO grounding-doc regen.

| Module | Color / scope | Key changes |
|--------|---------------|-------------|
| `ade_network::codec::tx_submission` (`tx_submission.rs` **+257 / −52**) | **BLUE** wire codec, additive + fix | **The `DC-PROTO-11` real-wire fix.** New `pub struct TxSubmissionTxId` (era-tag + `hash32` — the HardFork `GenTxId`, e.g. `[6, h'..']` for Conway); `decode_seq` now accepts BOTH definite and CBOR indefinite (`9f..ff`) sequences; encode reproduces the indefinite form so a captured frame re-encodes byte-identically; the era tag is preserved (never stripped/guessed) so a requester echoes the exact advertised id. Closes the real Cardano false-reject the synthetic round-trips had missed. |
| `ade_plutus::tx_eval` (`tx_eval.rs` **+282 / −28**) | **BLUE** Plutus evaluator integration, additive + fix | **The A1 ex_units-cap fix.** Caps each script's declared `ex_units`, closing a Plutus false-accept. New private `struct RedeemerFields`. |
| `ade_plutus` (`tests/end_to_end_plutus_eval.rs` **+183**) | **test**, additive | End-to-end Plutus eval coverage for the cap + the conformance path. |
| `ade_ledger::plutus_eval` (`plutus_eval.rs` **+9**) | **BLUE** ledger Plutus bridge, additive | Threads the per-script declared-ex_units cap into the ledger-side Plutus evaluation. |
| `ade_ledger` (`tests/conway_conservation_adversarial.rs` **+37**) | **test**, additive | Slice C double-spend adversarial coverage (no-double-spend negative corpus). |
| `ade_network::tx_submission` (`event.rs` **±5**, `transition.rs` **±6**) | **BLUE** mini-protocol FSM, additive | Threads `TxSubmissionTxId` through the tx-submission2 event/transition surface. |
| `ade_network` (`tests/tx_submission2_mempool_trace.rs` **±5**) | **test**, additive | tx-submission2 event-trace test updated for the new txid type. |
| `ade_core_interop` (`tests/tx_submission_ingress.rs` **+9 / −3**) | **test**, additive | tx-submission ingress test updated for the real-wire codec. |
| `ade_testkit` (`tests/plutus_conformance.rs` **+33 / −11**) | **test**, additive | A4 IOG conformance suite wired to the registry-bound manifest. |
| grounding docs + planning (`docs/...`) | docs | The in-span N-AO close regen (`docs/ade-{CODEMAP,SEAMS,TRACEABILITY,invariant-registry}.{md,toml}`, the 13 `R100` cluster-doc archive renames) + the new planning doc `docs/active/pre-preprod-local-streams.md` (+262) + the conformance manifest `docs/evidence/plutus-conformance-manifest.toml` (+55) + the c2-guide note. |

> **BLUE was TOUCHED this span (load-bearing — contrast with N-AO).** `git diff 862cd2cb..HEAD` over the configured BLUE
> `core_paths` trees is **non-empty**: `ade_network/src/codec/tx_submission.rs` (+`TxSubmissionTxId`),
> `ade_plutus/src/tx_eval.rs` (+`RedeemerFields`), `ade_ledger/src/plutus_eval.rs`, and the tx-submission FSM
> `event.rs`/`transition.rs`. BLUE canonical types **462 → 464** (§6). The two BLUE fixes are genuine wire-form /
> validity corrections (a real Cardano false-reject + a Plutus false-accept), not mere refactors — which is why the
> `DC-PROTO-02` flip and the `CN-PLUTUS-*` flips are earned by changed-and-tested BLUE behavior, not only by new gates.

## 4. Feature Flags

**No project feature-flag deltas, and the only manifest change is a new `[[bin]]` target.** Ade declares no `[features]`
table in any workspace `Cargo.toml` at either ref (`git grep '^\[features\]'` is empty at both `862cd2cb` and HEAD). **No
`#[cfg(feature = …)]` gate was introduced** (`git diff 862cd2cb..HEAD -- 'crates/**/*.rs' | grep -c '^+.*cfg(feature'`
= **0**), **no `compile_error!` coupling was added** (grep = **0**), and the sole `Cargo.toml` change is the new
`[[bin]]` target `ade_tx_submission2_server_capture` in `crates/ade_network/Cargo.toml` (not a dependency, not a
feature). **No new CLI flag** — `crates/ade_node/src/cli.rs` is untouched. There is no feature-flag coupling to report.

## 5. CI Checks (173 → 183; +10 new, 0 modified, 0 removed)

Ten CI scripts were added this span; **0 modified in place**, **0 removed** (`git diff --diff-filter=M`/`-D` over `ci/`
is empty; `ls ci/ci_check_*.sh | wc -l` = **173 → 183**). Each new gate backs an enforcement-mapping flip (or, for the
two coverage gates, attaches to an existing rule without flipping it). Grouping mirrors the three threads.

### Stream 3 — wire-FSM / codec / BLUE-sync enforcement (new gates)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci_check_codec_message_closed.sh` | **New** (`CN-WIRE-07`) | The per-protocol message taxonomy is a closed, versioned enum — no runtime meaning negotiation, no open message set. |
| `ci_check_header_body_binding.sh` | **New** (`CN-CONS-04`) | A block's body binds to its header (Cardano header/body separation); the binding is checked, not assumed. |
| `ci_check_mini_protocol_surface.sh` | **New** (`DC-PROTO-03`, `DC-PROTO-04`) | The closed mini-protocol surface — the 11 N2N+N2C protocols' codec + transition modules — has no out-of-surface message or state. |
| `ci_check_mini_protocol_transition_purity.sh` | **New** (`DC-PROTO-01`, `DC-PROTO-06`, `DC-CORE-01`) | Each protocol `transition` is a pure total reducer (no I/O, no `async`, no nondeterminism); also enforces BLUE-sync-only (`DC-CORE-01` is flipped against the existing `ci_check_no_async_in_blue.sh`, but this gate covers the FSM-purity half). |

### Stream 1 — Plutus + ledger coverage (new gates)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci_check_plutus_eval_purity.sh` | **New** (`CN-PLUTUS-04`) | No wall-clock / `rand` / env / fs / net / thread / process / `HashMap` / `HashSet` anywhere in `ade_plutus/src` (self-tested: detects an injected `SystemTime::now()` and clears a pure file). |
| `ci_check_plutus_conformance.sh` | **New** (`CN-PLUTUS-01`) | The registry-bound IOG `plutus-conformance` manifest (pinned aiken `42babe5` + pinned IOG corpus `643ddd13` / sha256 `83e8f447`) asserts the EXACT result + ex_units outcome (514/514 over runnable cases; CI fails on any diverge). |
| `ci_check_plutus_budget_cap.sh` | **New** (`CN-PLUTUS-01`, `DC-LEDGER-03` cross-ref) | The per-script declared-`ex_units` cap is enforced (the A1 false-accept closure). |
| `ci_check_plutus_oracle_no_false_accept.sh` | **New** (Plutus oracle no-false-accept) | The Plutus oracle path admits no false-accept against the adversarial reject corpus. |
| `ci_check_required_signer_closure.sh` | **New** (`DC-LEDGER-05`, attached; cross-ref `CN-LEDGER-08`/`CN-LEDGER-09`) | The required-signer / witness enumeration is closed (era-specific witness binding). **Attached to `DC-LEDGER-05` (stays `partial`) — coverage, not a flip.** |

### tx-submission2 real-wire (new gate)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci_check_tx_submission2_real_capture.sh` | **New** (`DC-PROTO-11`, `DC-PROTO-02`) | The tx-submission2 codec decodes a REAL cardano-node 11.0.1 frame (era-tagged txid `[6,hash]` + indefinite arrays) and re-encodes it byte-identically; accepts both definite + indefinite sequences; rejects bare/wrong-length txids + unterminated indefinite sequences. **This is the gate the `DC-PROTO-02` flip was proven against.** |

> **Cross-reference (CODEMAP + SEAMS + TRACEABILITY) — ONE window stale; all 10 new gates absent.** The 10 new gates
> (and the new rule `DC-PROTO-11` they help bind) are recorded **in the registry at HEAD**
> (`docs/ade-invariant-registry.toml`, 373 rules) but are **NOT yet in TRACEABILITY, SEAMS, or CODEMAP**, all three
> pinned to `862cd2cb` (`grep -c` for `ci_check_tx_submission2_real_capture` / `ci_check_plutus_conformance` /
> `ci_check_mini_protocol_surface` / each of the 10 in TRACEABILITY = **0**; their CI-count pins read **173** vs.
> HEAD's **183**). **No gate is orphaned** — each of the 10 new gates binds a registry rule (8 flip a rule to
> `enforced` + the new `DC-PROTO-11`; the required-signer gate attaches to `DC-LEDGER-05`). **Action:** regenerate
> CODEMAP + SEAMS + TRACEABILITY to `0887b2ad` so the 10 new gates, the new rule, the 10 flips, and the +2 BLUE types
> all appear and all three docs pin to this HEAD; until then the registry is authoritative.

## 6. Canonical Type Registry Delta

**n/a — no separate canonical-type registry is configured** (`canonical_type_registry: null`); canonical-type rules
live inline in the invariant registry under family **T**. **This window TOUCHED BLUE and added +2 BLUE canonical types**
(contrast the prior N-AO window, which added zero). Over the configured BLUE `core_paths` trees, the mechanical
`git grep -hE '^(pub )?(struct|enum) '` count moves **`462 → 464`**:

- **`+TxSubmissionTxId`** — `pub struct` in `crates/ade_network/src/codec/tx_submission.rs` (the `DC-PROTO-11`
  era-tagged txid wire type: era-tag + `hash32`, the HardFork `GenTxId`).
- **`+RedeemerFields`** — private `struct` in `crates/ade_plutus/src/tx_eval.rs` (the A1 per-script ex_units-cap fix).
  It is non-`pub`, but the structural grep (`^(pub )?(struct|enum)`) counts it — the SAME metric CODEMAP uses for its
  BLUE-tree canonical-type number, so `462 → 464` is consistent with the CODEMAP method.

**Zero BLUE canonical types removed** (the sorted struct/enum name lists show two additions, no removal).
CODEMAP's BLUE-tree metric should move **462 → 464** when it is regenerated to this HEAD (it currently still pins 462).

## 7. Normative / Invariant Rule Delta (372 → 373; +1 rule, 10 declared→enforced flips, +1 strengthening, zero removals)

**One rule ID was added; zero removed** (`372 → 373`; `comm -23` of the sorted `id =` lists is empty — exactly one
addition `DC-PROTO-11`, no removal). The status tally moves **239 → 250 enforced** and **113 → 103 declared**
(`enforced_scaffolding = 1`, `partial = 19` unchanged). The +11-enforced / −10-declared reconciles as: the 1 new rule
closes `enforced` (+1 enforced — `DC-PROTO-11` was added directly at `enforced`, net 0 declared), **and** 10
prior-`declared` rules flipped `enforced` this span (+10 enforced, −10 declared).

*(The configured `normative_docs` — the CE-79 tier-gate statement + addendum, the three contract docs, the CE-73
reclassification, and `CLAUDE.md` — were **not** changed this span: `git diff --name-only 862cd2cb..HEAD` over those
paths is empty. The rule-count delta is entirely the invariant-registry change.)*

**New rule (`+1`, enforced at HEAD):**

| Rule | Family / Tier · Status | Statement (summary) |
|------|------------------------|---------------------|
| `DC-PROTO-11` | DC / `derived` · **enforced** · `introduced_in = "TXSUB2-CODEC-REALWIRE"` | **tx-submission2 real-wire codec.** The TxSubmission2 codec accepts + byte-identically preserves cardano-node's REAL wire form for the txid/tx messages: each txId is era-tagged `[eraIndex, hash32]` (the HardFork `GenTxId`, e.g. `[6, h'..']` for Conway), and the txid/tx sequences are CBOR indefinite-length arrays (`9f..ff`). Decode accepts definite AND indefinite sequences; encode reproduces the indefinite form so a captured frame re-encodes byte-identically; the era tag is preserved (never stripped/guessed) so a requester echoes the exact advertised id. Bound to a live server-side capture (option B) + the real-capture corpus + `ci_check_tx_submission2_real_capture.sh`. |

**Declared → enforced flips (`10`):**

- **Stream 3 (5 gate commits, 6 rules):** **`CN-WIRE-07`** (closed codec-message taxonomy, `ci_check_codec_message_closed.sh`),
  **`CN-CONS-04`** (header/body binding, `ci_check_header_body_binding.sh`), **`DC-PROTO-03`** + **`DC-PROTO-04`** (the
  mini-protocol surface, `ci_check_mini_protocol_surface.sh`), **`DC-PROTO-01`** + **`DC-PROTO-06`** (FSM transition
  purity, `ci_check_mini_protocol_transition_purity.sh`), **`DC-CORE-01`** (BLUE sync-only, flipped against the existing
  `ci_check_no_async_in_blue.sh`, gate-complete, docs/registry-only).
- **Stream 1 Plutus (2 rules):** **`CN-PLUTUS-04`** (host-environment purity, `ci_check_plutus_eval_purity.sh`),
  **`CN-PLUTUS-01`** (IOG conformance, `ci_check_plutus_conformance.sh`).
- **tx-submission2 (1 rule):** **`DC-PROTO-02`** (the headline — the last N2N+N2C mini-protocol surface, flipped on the
  live full-exchange real-capture corpus; `ci_check_tx_submission2_real_capture.sh`).

**Strengthening (`strengthened_in += "TXSUB2-CODEC-REALWIRE"`) — 1:** **`DC-PROTO-02`**
(`["PHASE4-N-A"]` → `["PHASE4-N-A","TXSUB2-CODEC-REALWIRE"]`). No other `strengthened_in` changed this span; no rule
weakened; no rule removed.

**No rule was removed (expected: 0).** The registry delta is **1 new rule (enforced), 10 declared→enforced flips, 1
strengthening, zero removals** — consistent with append-only registry discipline.

> **Anomaly to surface — slice C / slice B commit intent vs. registry status (NOT a removal).** Slice C's commit subject
> reads "double-spend adversarial coverage -- close the CN-LEDGER-08 gap" (`91f63195`), and slice B adds the
> required-signer gate that **cross-references** `CN-LEDGER-08` / `CN-LEDGER-09`. But **`CN-LEDGER-08` remains
> `declared`** at HEAD (`code_locus = ""`, `tests = []`, `ci_script = ""`) — the new coverage broadened the negative
> corpus and the gate cross-refs the rule, but neither flipped it. Likewise the required-signer gate is **attached to the
> still-`partial` `DC-LEDGER-05`**, not a flip. This is a commit-intent-vs-status mismatch worth noting, **not a
> discipline violation** (no rule was removed or weakened; the coverage is real, the flip was simply not claimed). The
> §0 flip count (10) and the registry status tally (250/1/19/103) reflect the actual `status` fields, not the commit
> subjects.

## Honest residual (window scope)

This window flipped 11 rules to `enforced` (1 new + 10 prior-`declared`) on a pre-preprod local-first pass. The honest
residual:

- **Most flips are enforcement-mapping over already-shipped behavior, not new capability.** The six Stream 3 flips and
  the two coverage gates add mechanical gates for properties the codebase already satisfied; the genuinely new BLUE
  behavior is the two fixes (the tx-submission2 real-wire codec `DC-PROTO-11` and the Plutus ex_units cap A1) plus their
  flips (`DC-PROTO-02`, `CN-PLUTUS-01`, `CN-PLUTUS-04`).
- **`DC-PROTO-02` is the real headline — the last wire surface.** With the live full exchange captured + round-tripping
  byte-identically, all 11 N2N+N2C mini-protocol surfaces now have a real-capture byte-identical round-trip. This is a
  codec transcript-equivalence claim against the Haskell node, NOT a preprod operator-pass / bounty claim.
- **No `RO-LIVE` flip; no preprod operator pass.** Every flip is a `CN`/`DC`-family enforcement-mapping flip or the
  txsub2 fix. `RO-LIVE-01` stays operator-gated / partial; no `RO-LIVE` status changed. The window's whole point is the
  local-first work that PRECEDES the preprod pass (strict order 3→1→2 then preprod).
- **`CN-LEDGER-08` / `DC-LEDGER-05` did NOT flip despite the slice-C/B commit subjects.** The double-spend coverage and
  the required-signer gate are real, but their named rules stay `declared` / `partial` (see the §7 anomaly note). The
  next ledger pass should either flip them with a binding gate or re-scope the commit-subject claim.
- **The tx-submission2 mempool follow-up is honest about its source.** The flip rests on a captured real-node frame +
  corpus; the loopback test is Ade-vs-Ade (wiring consistency only) and is explicitly NOT a Haskell-wire conformance
  proof — that comes from the real-capture corpus test. The capture harness depends on a public-preprod mempool tx being
  available when run.
- **CODEMAP + SEAMS + TRACEABILITY refresh owed this window.** All three are pinned to `862cd2cb` and miss the **1 new
  rule**, the **+2 BLUE types**, the **10 flips**, the **new RED bin + corpus**, and the **10 new CI gates** (`grep -c`
  for each new gate / the new rule / the new bin in all three = 0; CI pin 173 vs. HEAD 183; rule pin 372 vs. HEAD 373;
  type pin 462 vs. HEAD 464). The registry holds all of it authoritatively at HEAD (373 rules); regenerating CODEMAP +
  SEAMS + TRACEABILITY to `0887b2ad` is the named follow-on (surfaced in §2 and §5). No orphan gate (each new gate binds
  a registry rule).

## Working tree at HEAD `0887b2ad` (clean for tracked files)

**The working tree is CLEAN for tracked files at this regen** — `git status --porcelain` shows only untracked scratch
(`.mithril-scratch/`, `wire_smoke.jsonl`), neither part of this doc. §1 narrates the committed span
`862cd2cb..0887b2ad` verbatim; §0/§6/§7 read rule status + canonical-type counts from the registry / BLUE trees at HEAD
(`DC-PROTO-02` enforced, 373 rules, 464 BLUE canonical types). The remaining follow-on actions are: (a) bump
`.idd-config.json` `head_deltas_baseline` `862cd2cb → 0887b2ad`, and (b) the CODEMAP + SEAMS + TRACEABILITY refresh to
`0887b2ad` (surfaced in §2 and §5).

---

## Historical — PHASE4-N-AO live multi-candidate fork-choice SELECT + `CN-CONS-03` flip (`31efec44 → 862cd2cb`)

> The section below is the **previous** HEAD_DELTAS lead, preserved in condensed form. It narrated the
> `31efec44 → 862cd2cb` span — **the PHASE4-N-AO cluster** (slices S2–S14, with S1 at the baseline tip): the live
> multi-candidate fork-choice SELECT + adopt path. Ade DECIDES a fork-choice win among competing live branches, PROVES
> the replacement branch (fetch → bind → link → validate), COMMITS the adoption, and re-converges — on a NATURAL
> two-producer partition-and-reconverge venue. **This is the cluster that flipped `CN-CONS-03` `declared → enforced`**
> (Cardano post-partition convergence) on a committed, sha256-pinned natural CE-AO-6 transcript
> (`ContinuesSelectedBranch` + `AgreedAtSwitchTip{391}` + 25 descendants + 0 diverged; transcript OUTSIDE-repo per
> competition-secrecy). **42 commits, 50 files, +9797 / −98.** **GREEN+RED ONLY — ZERO new BLUE canonical type and ZERO
> BLUE-tree change** (`462 → 462`; `select_best_chain` + `validate_and_apply_header` reused byte-unchanged). **+6 new
> modules**, all GREEN/RED in `ade_node` (`candidate_aggregator`, `selector_state`, `fork_switch`, `lca_walk` GREEN;
> `fair_merge` RED; `post_switch_continuity` GREEN + bin); **NO new crate (11)**, **NO `Cargo.toml` change**, **NO new
> CLI flag**. **+11 CI gates, 6 modified, 0 removed** (162 → 173). **Registry 365 → 372** (+7 new rules `DC-NODE-38..41`,
> `DC-PUMP-04`, `DC-EVIDENCE-04/05`, all enforced; +5 declared→enforced flips `CN-CONS-03` + `DC-NODE-34..37`; +10
> strengthenings; **zero removals**; status 227/1/19/118 → 239/1/19/113). **NO `RO-LIVE` flip** (`RO-LIVE-01` stays
> operator-gated). The full §§0–7 narrative is recoverable from this doc's git history at `862cd2cb`. *(The N-AO close's
> grounding-doc regen + cluster-doc archive land in the FIRST two commits of THIS window — `388d8073`, `5532ddf4` — so
> the four grounding docs were pinned to `862cd2cb` at the start of this window.)*

---

## Historical — PHASE4-N-AM keep-alive client + PHASE4-N-AN rollback-materialize eta0 (`e87e8a43 → b8860b16`)

> Preserved as a pointer. It narrated the `e87e8a43 → b8860b16` span: the **PHASE4-N-AL close commit** (`35a851b9`) +
> the **PHASE4-N-AM cluster** (`DC-PUMP-03` — the N2N keep-alive CLIENT, mini-protocol 8) + the **PHASE4-N-AN cluster**
> (`T-REC-06` — `materialize_rolled_back_state` overlays the recovered seed-epoch eta0 before the `block_validity` fold)
> + a stale-gate triage. **12 commits, 32 files, +2288 / −516.** **Touched BLUE but added ZERO new canonical type**
> (462 → 462; a single METHOD + a field). **NO new crate (11), NO new module, NO new CLI flag** (only a `tokio`
> `test-util` dev-dependency). **+2 CI gates** (159 → 161). **Registry 359 → 361** (+2; +1 strengthening; 0 removed).
> **NO `RO-LIVE` flip; `CN-CONS-03` NOT flipped** (single-best-peer rollback-FOLLOW scope — flipped the next window,
> PHASE4-N-AO). The full §§0–7 narrative is recoverable from this doc's git history at `b8860b16`.

---

## Historical — PHASE4-N-AL participant recovered-anchor rollback no-op (`b4c0983d → e87e8a43`)

> Preserved as a pointer. The **N-AK close commit** (`efa2a44e`) + a C2-guide remediation note + the **PHASE4-N-AL
> cluster** (single slice AL-S1, `DC-NODE-33` — the participant mirror of N-AK's `DC-NODE-32` recovered-anchor rollback
> no-op). **4 commits, 14 files, +1792 / −825.** **Did NOT touch BLUE** (462 → 462); **NO new crate/module/type/gate**
> (159 → 159). Registry **358 → 359** (+1; 0 strengthenings; 0 removals). **NO `RO-LIVE` flip.** The full §§0–7 narrative
> is recoverable from this doc's git history at `e87e8a43`.

---

## Historical — PHASE4-N-AK recovered-anchor live-follow start + rollback boundary (`b1bed361 → b4c0983d`)

> Preserved as a pointer. The **N-AJ close commit** (`bbdc3585`) + the **PHASE4-N-AK cluster** (AK-S1 + AK-S2). **7
> commits, 33 files, +2647 / −544.** **Touched BLUE — +2 canonical types** (`RecoveredAnchorPoint` +
> `RecoveredAnchorPointError`, new BLUE module `crates/ade_ledger/src/recovered_anchor_point.rs`; + a new RED module).
> `DC-NODE-31` (persist the bootstrap anchor POINT + resolve the live-follow FindIntersect start) + `DC-NODE-32`
> (single-producer `RollBackward(anchor)` idempotent no-op). Registry **356 → 358** (+2; `T-REC-05` strengthened; 0
> removed); CI **159 → 159**. **NO `RO-LIVE` flip.** The full §§0–7 narrative is recoverable from this doc's git history
> at `b4c0983d`.

---

## Historical — PHASE4-N-AJ Participant-path convergence evidence emission (`e99a86c7 → b1bed361`)

> Preserved as a pointer. The **PHASE4-N-AJ cluster** — Participant-path convergence evidence emission, the CE-AI-6
> bridge. **9 commits, 19 files, +1813 / −35.** **EVIDENCE-ONLY — ZERO BLUE change.** Added a deterministic GREEN
> evidence side-output (the EXISTING closed `AgreementVerdict` vocabulary to a `--convergence-evidence-path` JSONL sink,
> the new GREEN/RED module `ade_node::convergence_evidence`). CI **157 → 159** (+2). Registry **354 → 356** (+2;
> `DC-ADMIT-04` strengthened; **`CN-CONS-03` NOT flipped**; 0 removed). **NO `RO-LIVE` flip.** The full §§0–7 narrative
> is recoverable from this doc's git history at `b1bed361`.

---

## Historical — earlier windows (`8e2c3672 → e99a86c7` and before)

> Preserved as pointers. The **PHASE4-N-AI cluster** (live fork-choice rollback-follow wiring — single-best-peer FOLLOW,
> `DC-NODE-23..29`; +2 BLUE types `RollbackPoint`/`RollbackReason`; 347 → 354 rules / 148 → 157 CI; H-1 found + closed by
> AI-S6; `CN-CONS-03` NOT flipped); the **PHASE4-N-AG/N-AH** (single-producer loop-continuation + local-tip forge-base
> authority, `DC-NODE-19..22`; 343 → 347 rules / 143 → 148 CI; cert-free single-producer block production on C2-LOCAL);
> the **PHASE4-N-AF / N-AE.F / N-AD-N-AE CE-A5 window** (single-producer durable-spine extend + receive idempotency +
> recover→serve continuity + forge-on-followed-tip admissibility — the CE-A5 manifest: a real `cardano-node 11.0.1`
> relay `AddedToCurrentChain` an Ade-forged block; `DC-NODE-14..18`, `DC-CONS-24`, `DC-PROTO-10`); the **PHASE4-N-AC/AB/AA**
> (KES key evolution `DC-CRYPTO-10`, outbound mux segmentation `CN-SESS-05`, bounded serve range `DC-SERVEMEM-01`); the
> **PHASE4-N-U** (forged-block durability); and the **G-K…G-R + C1 multi-cluster catch-up**. The full §§0–7 narrative
> for each is recoverable from this doc's git history at the respective HEADs.

---

## Generation notes

### Regen `862cd2cb → 0887b2ad` (pre-preprod local-first enforcement-mapping window — `DC-PROTO-02` flip + Stream 3 + Plutus — current lead)

- **Baseline valid; not a cluster — a local-first enforcement-mapping pass.** Run against `862cd2cb` (the PHASE4-N-AO
  cluster-close), which `git rev-parse` resolves and `git merge-base 862cd2cb HEAD == 862cd2cb` confirms is a strict
  ancestor of HEAD `0887b2ad` (`862cd2cb` carries no tag). The N-AO close's own grounding-doc regen + cluster-doc
  archive land in the FIRST two commits of this span (`388d8073`, `5532ddf4`) — prior-cluster housekeeping that appears
  here as in-range churn (it explains the 13 `R100` renames + the `−5672` SEAMS line churn). The substantive new work is
  the three threads scoped by `86252176` (strict order 3→1→2). This regen's post-step is the baseline bump
  `862cd2cb → 0887b2ad`.
- **Counts are mechanical (git/grep/ls):** commit log + `--shortstat` over `862cd2cb..HEAD` (**18** commits, no merges /
  **51** files / **+4182 / −6652**); CI gate count via `git ls-tree -r --name-only <ref> ci/ | grep -c ci_check_.*\.sh`
  = **173** at baseline, **183** at HEAD (`--diff-filter=A` = 10 new, `--diff-filter=M`/`-D` empty); registry rule count
  via `grep -c '^id = '` at each ref (**372 → 373**; `comm -23` of the sorted `id =` lists is empty — exactly one
  addition `DC-PROTO-11`, zero removals); registry status via `grep '^status = ' | sort | uniq -c` (baseline
  **239 / 1 / 19 / 113**, HEAD **250 / 1 / 19 / 103**); strengthenings = **1** (`DC-PROTO-02` gains
  `TXSUB2-CODEC-REALWIRE`); BLUE canonical types **462 → 464** (`git grep -hE '^(pub )?(struct|enum) '` over the BLUE
  `core_paths` trees: `+TxSubmissionTxId`, `+RedeemerFields`).
- **BLUE TOUCHED — +2 canonical types (contrast N-AO).** `git diff 862cd2cb..HEAD` over the BLUE `core_paths` trees is
  non-empty: `ade_network/src/codec/tx_submission.rs` (+`TxSubmissionTxId`, the `DC-PROTO-11` real-wire fix),
  `ade_plutus/src/tx_eval.rs` (+`RedeemerFields`, the A1 ex_units-cap fix), `ade_ledger/src/plutus_eval.rs`, and the
  tx-submission FSM `event.rs`/`transition.rs`. Both BLUE fixes are genuine validity/wire corrections, not refactors.
- **No new library module; one new RED bin + a corpus.** `git diff --diff-filter=A --name-only 862cd2cb..HEAD --
  'crates/**/*.rs'` lists exactly three new files: the RED capture bin `src/bin/capture_tx_submission2_server.rs` and
  two test files (`tx_submission2_real_capture_corpus.rs`, `tx_submission2_server_capture_loopback.rs`). No new `src/`
  library module; no new crate / workspace — still 11 crates. The new real-capture corpus lives under
  `corpus/network/n2n/tx_submission2/`. Colors read from the `// RED`/`// Core Contract` banners.
- **Manifest change is a `[[bin]]` target only; no feature flag, no CLI flag.** `git diff --name-only 862cd2cb..HEAD --
  '**/Cargo.toml'` = `crates/ade_network/Cargo.toml` (a single new `[[bin]]` `ade_tx_submission2_server_capture`); no
  `[features]` table at either ref; 0 `cfg(feature)` and 0 `compile_error!` added; `cli.rs` untouched.
- **Registry delta is +1 rule + 10 declared→enforced flips + 1 strengthening, NOT a removal.** The new rule
  `DC-PROTO-11` was added at `enforced`; 10 prior-`declared` rules flipped `enforced` (`DC-PROTO-01/02/03/04/06`,
  `CN-CONS-04`, `CN-WIRE-07`, `DC-CORE-01`, `CN-PLUTUS-01`, `CN-PLUTUS-04`). The sorted-id `comm -23` confirms zero
  removals. `DC-PROTO-02` gains `strengthened_in += TXSUB2-CODEC-REALWIRE` and is bound (in its `evidence_notes`) to the
  live server-side capture + the real-capture corpus.
- **+10 CI gates, 0 modified, 0 removed.** New: 4 Stream-3 gates + 4 Plutus gates + 1 required-signer gate + the txsub2
  real-capture gate. `--diff-filter=M`/`-D` over `ci/` are both empty.
- **`DC-PROTO-02` is the headline flip; no `RO-LIVE` flip.** `DC-PROTO-02` (the last N2N+N2C mini-protocol surface)
  flipped `declared → enforced` on the live full-exchange real-capture corpus (`ci_check_tx_submission2_real_capture.sh`).
  `RO-LIVE-01` stays operator-gated / partial; no `RO-LIVE` status changed — this is a local-first pass that PRECEDES
  the preprod operator pass.
- **Anomaly noted (NOT a removal): slice C/B commit intent vs. status.** Slice C's subject says "close the CN-LEDGER-08
  gap" and the required-signer gate cross-refs `CN-LEDGER-08`, but `CN-LEDGER-08` stays `declared` (empty
  code_locus/tests/ci_script) and the required-signer gate attaches to the still-`partial` `DC-LEDGER-05`. The coverage
  is real; the flip was simply not claimed. The flip count (10) and status tally reflect the actual `status` fields.
  Surfaced in §7.
- **Normative docs unchanged this span.** `git diff --name-only 862cd2cb..HEAD` over the configured `normative_docs`
  (CE-79 statement + addendum, the three contract docs, CE-73 reclassification, `CLAUDE.md`) is empty — the §7 delta is
  entirely the invariant-registry change.
- **§1 commit log verbatim from `git log` (newest first).** The per-thread synthesis is in §0/§3. All 18 subjects carry
  a conventional-commits prefix (`feat`×9 / `docs`×6 / `test`×2 / `fix`×1); zero unclassified.
- **Doc-refresh state — CODEMAP + SEAMS + TRACEABILITY now ONE window STALE.** All three were regenerated to `862cd2cb`
  (this window's BASELINE, by the in-span N-AO close `388d8073`) and carry everything through PHASE4-N-AO but **nothing
  from this window** — they miss the new rule `DC-PROTO-11`, the +2 BLUE types, the 10 flips, the new RED bin + corpus,
  and the 10 new CI gates (`grep -c` for each in all three = 0; CI pin 173 vs. HEAD 183; rule pin 372 vs. HEAD 373; type
  pin 462 vs. HEAD 464). **Cross-reference warnings surfaced in §2 and §5.** Regenerate CODEMAP + SEAMS + TRACEABILITY
  to `0887b2ad` as a follow-on; the registry holds all of it authoritatively in the interim (373 rules). No orphan gate
  (each new gate binds a registry rule).
- **Working tree CLEAN for tracked files.** This regen runs with all this-window artifacts committed
  (`git status --porcelain` = untracked scratch only). Follow-on actions: bump `.idd-config.json` `head_deltas_baseline`
  `862cd2cb → 0887b2ad`, and refresh CODEMAP + SEAMS + TRACEABILITY to `0887b2ad`.
