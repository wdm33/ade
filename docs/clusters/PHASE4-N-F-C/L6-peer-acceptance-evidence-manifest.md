# Slice PHASE4-N-F-C / L6 — Peer-acceptance evidence manifest

> The evidence slice. Given an Ade-forged block (hash + forge metadata from L5) and a Haskell
> cardano-node peer's acceptance signal, produce a closed, canonical BA-02 evidence manifest —
> and produce it ONLY when the peer accepted that exact forged block. This slice ships the
> manifest schema + the parser/correlator + the synthetic dry-run that proves the mechanics;
> the real BA-02 claim stays an explicit open obligation until a real Haskell peer-accept log
> exists. Evidence, not production semantics. Authority doc: `cluster.md` (CE-L-6). Builds on
> L5 (`forge_one_from_recovered` → `ForgeSucceeded { artifact }` supplies the forged-block hash).
>
> The bounty evaluates Ade's node lifecycle; it does not define Ade's architecture.

## 2. Slice Header
- **Slice Name:** A closed, canonical BA-02 evidence manifest + its GREEN parser/correlator:
  `(Ade forged-block hash + forge metadata, Haskell peer accept log, chain-point context)
  → BA02Manifest | NoEvidence`, where `BA02Manifest` is producible ONLY on an exact
  forged-hash ↔ peer-accept match. Self-accept, `ForgeSucceeded` alone, `block_received` alone,
  and a lagging/diverged agreement verdict are each `NoEvidence`.
- **Cluster:** PHASE4-N-F-C — Build the real Ade node lifecycle.
- **Status:** Proposed.
- **Cluster Exit Criteria Addressed:** CE-L-6 — *"BA-02 manifest schema + synthetic dry-run
  mechanical. (Live peer-accept = gated obligation on the RO-LIVE family, NOT a mechanical CE.)"*
- **Slice Dependencies:** L5 (`forge_one_from_recovered` → `CoordinatorEvent::ForgeSucceeded
  { slot, artifact }` is the forged-block-hash source the manifest correlates). Reuses the
  closed-JSONL-event idiom from `admission_log` / `live_log` verbatim (pattern, not code).

## 3. Implementation Instruction (AI)
Implement §10 only — the closed `BA02Manifest`/`NoEvidence` types, the GREEN parser/correlator,
the synthetic dry-run tests, and the CI gate. **Produce no BA-02 claim**: no code path emits a
`BA02Manifest` from a synthetic log, from Ade self-accept, from `ForgeSucceeded` alone, from
`block_received` alone, or from a lagging/diverged verdict. The live peer-accept run remains
operator-gated (it is NOT a mechanical CE here). Do NOT change forge / recovery / sync semantics
(`produce_mode`, `node_sync`, `bootstrap.rs`, `replay.rs`, the forge engine) — L6 only READS their
already-emitted outputs. **Add no new BLUE authority** (the correlator is GREEN evidence, like the
agreement-verdict reducer — it compares already-authoritative outputs; per
`[[feedback-evidence-reducers-are-green-not-authority]]`). Resolve the entry obligations in §9.0
before coding. **Sequential edits only: one edit, confirm, then the next.** Commit with the
model-attribution trailer.

## 4. Intent
Make "Ade produced a block a Haskell peer accepted" (BA-02) a *closed, mechanically-checkable
correlation* rather than an assertion: the manifest is constructible iff the operator-captured
Haskell peer-accept signal names the exact hash of an Ade-forged block at the matching chain point.
The invariant impact is a **closed evidence surface**: every weaker signal (self-accept,
forge-only, receive-only, agreement verdict) is structurally `NoEvidence`, so no run can overstate
peer acceptance — directly instantiating
`[[feedback-shell-must-not-overstate-semantic-truth]]` (wire/forge success ≠ peer acceptance).

## 5. Scope
- **Modules / crates:**
  - `ade_node::ba02_evidence` (GREEN, new) — the closed `BA02Manifest` / `NoEvidence` types, the
    closed peer-accept-log event vocabulary (parser), and the pure correlator
    `correlate(ade_forge: AdeForgeRecord, peer_log: &[PeerAcceptEvent]) -> BA02Outcome`.
  - `ade_node::lib` (RED, 1 line) — `pub mod ba02_evidence`.
  - `ci/ci_check_ba02_evidence_closed.sh` (new) — the BA-02 honesty gate (see §9.3).
- **State machines affected:** none. The correlator is a pure function over already-emitted
  records.
- **Persistence impact:** none. L6 reads logs/artifacts; it persists no node state. (A produced
  manifest is an operator artifact under `docs/evidence/`, not node state.)
- **Network-visible impact:** none (no live I/O in the mechanical slice; the live capture is the
  operator-gated obligation).
- **Out of scope:** the live `--mode node` produce-and-capture run (operator-gated, RO-LIVE);
  any change to forge/recovery/sync; any new BLUE authority; emitting a real BA-02 claim;
  flipping RO-LIVE-01/RO-LIVE-05 status.

## 6. Execution Boundary (TCB color)
- **BLUE (reuse only — no change):** none touched. The forged-block hash the correlator reads is
  already minted by the BLUE forge/`self_accept` path (L5); L6 does not re-derive it.
- **GREEN:** `ade_node::ba02_evidence` — the manifest types + parser + correlator. Pure,
  deterministic, no I/O / clock / float / HashMap; it COMPARES already-authoritative outputs and
  emits evidence (exactly the agreement-verdict reducer's color per
  `[[feedback-evidence-reducers-are-green-not-authority]]`).
- **RED:** the `lib.rs` module declaration; (operator-gated, not in this slice) the live capture
  driver that would feed a real peer log to the correlator.
- **CI:** `ci_check_ba02_evidence_closed.sh`.

## 7. Invariants Preserved
- `RO-LIVE-01` (registry, `status = partial`) — BA-02's home (the live peer-accept obligation).
  L6 does NOT enforce it, does NOT flip its status, and does NOT weaken its `open_obligation`
  (the operator-witnessed live pass). L6 ships the manifest schema + correlator the eventual live
  capture will use; the live accept stays owed.
- `RO-LIVE-05` (registry, `status = enforced`) — the admission/agreement live pass. L6 must not
  conflate its `agreement_verdict { agreed }` (Ade-vs-Ade self-comparison at the peer's tip) with
  BA-02 (a Haskell peer accepting an *Ade-forged* block). The manifest treats an agreement verdict
  as `NoEvidence`.
- `CN-CINPUT-03` / `DC-CINPUT-02b` (L5) — untouched; L6 reads `ForgeSucceeded`, it does not forge.
- `CN-NODE-01`, the forge/recovery/sync semantics, and the closed admission/live JSONL vocabularies
  — all unchanged (L6 adds a new disjoint vocabulary; it does not edit the existing ones).

## 8. Invariants Strengthened or Introduced
- **Introduces a new release-family rule — candidate `RO-LIVE-06` (BA-02 evidence-manifest
  closure)** — proposed, NOT appended in-slice, and deliberately SEPARATE from `RO-LIVE-01` so
  "we have a schema" can never be confused with "we have a live accepted block":
  - `RO-LIVE-01` = the live peer-accept obligation (stays `partial`; L6 does not enforce it).
  - `RO-LIVE-06` = the closed BA-02 evidence-manifest schema + correlation, enforced mechanically
    after L6: *"A BA02Manifest is constructible only from an exact match between an Ade-forged
    block hash (from a ForgeSucceeded artifact) and a Haskell peer-accept event naming that same
    hash at the matching chain point; PeerServedBlock is the strongest signal and PeerChainTip is
    acceptable only when it names the exact forged hash at the matching slot/block context; if
    multiple peer-accept signals are present they must all agree, and any conflict yields
    NoEvidence; every weaker or mismatched signal yields NoEvidence; a synthetic log proves the
    correlation mechanics but cannot satisfy the live BA-02 obligation, which remains
    operator-gated under RO-LIVE-01."*
  The enforcing artifacts are the synthetic dry-run tests + `ci_check_ba02_evidence_closed.sh`.
  The rule is surfaced as a candidate for the user to confirm; the registry append happens at
  `/cluster-close` (consistent with L1–L5; DC-CINPUT-02b / CN-CINPUT-03 also still await their
  close-time append).
- A slice strengthens one invariant family — here the **live-evidence honesty / BA-02** family.
  No registry edit in-slice.

## 9. Design Summary

### 9.0 Entry obligations (resolve before coding)
- **(M1) What is a "Haskell peer accept" signal — both forms, ranked.** A Haskell cardano-node
  does not emit an "I accepted block X" line on the wire that Ade receives directly. The honest,
  capturable acceptance signals are two, and L6 accepts BOTH as closed `PeerAcceptEvent` variants,
  ranked:
  - **`PeerServedBlock` — the STRONGEST signal.** After Ade forges block H and submits it, the
    peer accepts it iff the peer subsequently SERVES H back on its own chain-serving path (a
    `RollForward` / `BlockFetch` carrying header/hash H — captured in the operator-pass transcript
    style). Served-back proves the peer is carrying the block in the chain it serves to others.
  - **`PeerChainTip` — acceptable only as a corroborating/standalone tip signal.** The peer's own
    `cardano-cli query tip` / log naming H at the matching slot/block context. It is valid ONLY
    when it names the EXACT forged hash at the matching slot/block context; it must never be
    allowed to paper over disagreement.
  - **Ranking + agreement rule (mandatory):** if multiple peer-accept signals are present, ALL
    signals naming the forged slot/hash context MUST agree. If `PeerServedBlock` and `PeerChainTip`
    both exist and agree, the manifest records the provenance clearly, preferring `PeerServedBlock`
    as primary (the schema may record both). **Any conflict between `PeerServedBlock` and
    `PeerChainTip` (different hash at the same forged context) yields `NoEvidence`.** Weaker signals
    remain `NoEvidence`.
  The manifest records the capture provenance in a typed `peer_accept_source` field so it is honest
  about which signal(s) backed the match.
- **(M2) The forged evidence uses ONLY forge-event-exposed fields.** `CoordinatorEvent::ForgeSucceeded
  { artifact }` carries a `ForgedBlockArtifact { slot, hash, bytes }` — it exposes the block `hash`
  and `slot` directly, and NOTHING else usable as a chain-point key (no block-number, no prev-hash).
  So the manifest's Ade side is `AdeForgeRecord { forged_block_hash, slot, network_magic }`, with
  `forged_block_hash = artifact.hash` and `slot = artifact.slot` read DIRECTLY — the hash is never
  recomputed, the block bytes are never parsed to recover a prev-hash or block-number, and no new
  BLUE derivation is added (an over-specified block_number/prev_hash key was removed for exactly this
  reason). The correlation is HASH-PRIMARY: the forged hash is the required key; the slot is the only
  additional context, and it comes straight from the forge event.
- **(M3) Closed vocabulary + allow-list + negative tests (mandatory, per
  `[[feedback-shell-must-not-overstate-semantic-truth]]`).** The peer-accept-log event enum is a
  CLOSED sum with a stable `event` discriminator (mirroring `admission_log`'s pattern), parsed by
  an allow-list (unknown event ⇒ rejected/ignored, never coerced to acceptance), with negative
  tests for each non-acceptance case. The manifest itself is a closed struct with a versioned
  schema tag (mirroring `RawMithrilManifest`).
- **(M4) NoEvidence is the default; BA02Manifest is the exception.** `correlate` returns
  `BA02Outcome::NoEvidence { reason }` for: no peer-accept event; peer-accept names a different
  hash; the forged hash matches but a PRESENT peer slot contradicts the forge slot; conflicting
  peer-accept signals at the forged context; the only signals are self-accept / `ForgeSucceeded` /
  `block_received` / an agreement verdict. `BA02Manifest` is returned ONLY on a forged-hash match
  with no contradicting context and no conflicting signal. Context rule (M1): the forged hash is the
  required key; a peer signal's slot is OPTIONAL — when present it must equal the forge slot, when
  omitted the signal still matches on hash (it must not CONTRADICT the forge record, but it may be
  silent about slot).

### 9.1 The manifest + correlator (grounded in the existing patterns)
```
struct AdeForgeRecord { forged_block_hash: Hash32, slot, block_number, prev_hash, network_magic }
enum PeerAcceptEvent {                 // closed, JSON `event` discriminator (allow-list parsed)
    PeerServedBlock { block_hash: Hash32, slot, peer },     // STRONGEST: peer served H back
    PeerChainTip   { block_hash: Hash32, slot, peer },      // corroborating: peer's tip names H
    // non-acceptance lines (block_received, agreement_verdict, …) parse to a NonAcceptance marker,
    // never to a PeerAcceptEvent.
}
enum PeerAcceptSource { ServedBlock, ChainTip, ServedBlockAndChainTip }   // typed provenance
enum BA02Outcome {
    Ba02Manifest(BA02Manifest),        // exact match only, no conflict
    NoEvidence { reason: NoEvidenceReason },   // closed reason sum (DEFAULT)
}
struct BA02Manifest {                  // closed, versioned (schema_version tag, RawMithrilManifest-style)
    schema_version, ade_forge: AdeForgeRecord,
    peer_accept_source: PeerAcceptSource,   // ServedBlock primary; records both when both agree
    matched_block_hash: Hash32,        // == ade_forge.forged_block_hash == every accepting signal
}
```
`correlate` is pure:
1. Collect every `PeerAcceptEvent` whose chain point matches `ade_forge` (slot/block context).
2. If none → `NoEvidence { NoPeerAccept }`.
3. If any matching-context signal names a hash ≠ `ade_forge.forged_block_hash` → `NoEvidence`
   (`HashMismatch` or, when a served-block and a tip disagree at the same context,
   `ConflictingPeerSignals`).
4. Otherwise (all matching-context signals agree on the forged hash) build `Ba02Manifest` with
   `peer_accept_source` = `ServedBlock` (if served-back present), `ChainTip` (tip only), or
   `ServedBlockAndChainTip` (both agree). `PeerServedBlock` is primary when present.

### 9.2 Why this is GREEN evidence, not authority
The correlator compares two already-authoritative outputs (the BLUE-minted forged hash; the
operator-captured peer signal) and emits a verdict — it admits nothing, forges nothing, persists
no node state. This is the exact color of the agreement-verdict reducer
(`[[feedback-evidence-reducers-are-green-not-authority]]`): a `Ba02Manifest` is a *claim about*
authority, not authority. It therefore cannot, by construction, make a block valid or accepted —
it can only report a match the peer independently produced.

### 9.3 CI gate: BA-02 honesty (`ci_check_ba02_evidence_closed.sh`)
Mechanical, data-flow-resistant:
- the `BA02Manifest` constructor is reachable ONLY from `correlate`'s exact-match arm (no other
  production site builds a `Ba02Manifest`);
- the `ba02_evidence` module references none of the self-evidence tokens as an acceptance source —
  `self_accept` / `ForgeSucceeded` / `block_received` / `agreement_verdict` / `agreed` must not
  appear as a `PeerAcceptEvent` variant or feed the match arm;
- no committed `docs/evidence/*ba02*` manifest exists that was built from a synthetic/test log
  (the synthetic fixtures live in test code only; a real manifest requires the operator log).
Comment- + `#[cfg(test)]`-stripped; teeth-verified.

## 10. Changes Introduced
### Types
- New GREEN closed types: `AdeForgeRecord`, `PeerAcceptEvent` (closed sum + discriminator),
  `PeerAcceptSource`, `NoEvidenceReason` (closed sum), `BA02Outcome`, `BA02Manifest` (versioned).
  No new BLUE/canonical type; reuses `Hash32`/`SlotNo`/`BlockNo`.
### State Transitions
- None. `correlate` is a pure function.
### Persistence
- None (no node state). Manifests are operator artifacts.
### Removal / Refactors
- None to forge / recovery / sync / admission / produce_mode / bootstrap.rs / replay.rs.

## 11. Replay, Crash, and Epoch Validation
- **Replay (new, this slice):** `ba02_correlate_two_runs_byte_identical` — `correlate` over a
  fixed `(forge record, peer log)` yields a byte-identical `BA02Outcome` across two runs
  (determinism; the manifest is canonically serializable).
- **Crash/restart:** n/a (pure function, no persisted state).
- **Epoch:** n/a (single forge event; the correlator checks chain-point context, not epoch
  transitions).

## 12. Mechanical Acceptance Criteria
- [ ] **Closed manifest schema exists**: `BA02Manifest` + `BA02Outcome` + `PeerAcceptEvent` +
      `PeerAcceptSource` + `NoEvidenceReason` compile as closed sums; `ba02_manifest_schema_round_trips`
      (encode→decode→encode byte-identical, versioned tag verified).
- [ ] **Exact-match positive (synthetic)**: `ba02_correlate_served_block_yields_manifest` — a
      synthetic `PeerServedBlock` naming the exact forged hash at the matching chain point →
      `Ba02Manifest` with `peer_accept_source = ServedBlock`. (Synthetic ⇒ proves mechanics, NOT a
      BA-02 claim — the fixture is labelled throwaway.)
- [ ] **Tip-only positive**: `ba02_correlate_chain_tip_only_yields_manifest` — a `PeerChainTip`
      naming the exact forged hash/context, no served-block → `Ba02Manifest` with
      `peer_accept_source = ChainTip`.
- [ ] **Both-agree**: `ba02_correlate_both_signals_agree_records_served_primary` — served-block +
      tip agree on the forged hash → `Ba02Manifest` with `peer_accept_source =
      ServedBlockAndChainTip`, primary = served-block.
- [ ] **Conflict → NoEvidence**: `ba02_correlate_conflicting_signals_is_no_evidence` — served-block
      and tip name DIFFERENT hashes at the same forged context → `NoEvidence { ConflictingPeerSignals }`.
- [ ] **Mismatch → NoEvidence**: `ba02_correlate_wrong_hash_is_no_evidence`,
      `ba02_correlate_chain_point_mismatch_is_no_evidence`, `ba02_correlate_stale_log_is_no_evidence`.
- [ ] **Weaker-signal → NoEvidence**: `ba02_self_accept_is_not_evidence`,
      `ba02_forge_succeeded_alone_is_not_evidence`, `ba02_block_received_alone_is_not_evidence`,
      `ba02_agreement_verdict_is_not_evidence` — each yields `NoEvidence`, never a manifest.
- [ ] **Missing accept → NoEvidence**: `ba02_correlate_empty_peer_log_is_no_evidence`.
- [ ] `ci/ci_check_ba02_evidence_closed.sh` passes (BA02Manifest constructor contained to the
      exact-match arm; no self-evidence token feeds the match; no synthetic-built committed
      manifest).
- [ ] `cargo build` + scoped `ade_node` tests + the named gate pass. Full `ade_testkit` corpus
      lane is NOT an L6 gate.

## 13. Failure Modes (all → NoEvidence, never a false manifest)
No peer-accept event; wrong hash; chain-point mismatch; conflicting peer signals at the forged
context; stale/wrong-network log; only weaker signals present; malformed/unknown peer-log event
(allow-list rejects → not coerced to accept). Each is a deterministic `NoEvidence { reason }`. There
is NO path from a synthetic log, self-accept, forge-only, receive-only, an agreement verdict, or a
conflicting signal to a `Ba02Manifest`.

## 14. Hard Prohibitions
**Inherited (cluster):** no BA-02 claim before a real peer accept; no treating self-accept /
`ForgeSucceeded` / `block_received` / a lagging-or-diverged agreement verdict as acceptance; no new
BLUE authority/type; no `HashMap`/clock/float in GREEN.
**Slice-specific (from the L6 brief):** **no BA-02 claim without real Haskell peer acceptance**;
Ade self-accept is not evidence; `ForgeSucceeded` alone is not evidence; `block_received` alone is
not evidence; a lagging/diverged verdict is not evidence; conflicting peer signals are not evidence;
**synthetic logs may test the parser/correlator but cannot satisfy BA-02** (no synthetic-built
committed manifest); the live run remains operator-gated; no changes to forge / recovery / sync
semantics; no registry status flip (RO-LIVE-01/RO-LIVE-05 unchanged); no grounding-doc regeneration.
**Process:** sequential edits only.

## 15. Explicit Non-Goals
No live `--mode node` produce-and-capture run (operator-gated, RO-LIVE obligation); no real BA-02
manifest emission; no forge/recovery/sync change; no RO-LIVE status flip; no registry append; no
grounding-doc refresh. The bounty evaluates Ade's node lifecycle; it does not define Ade's
architecture — L6 ships the honesty surface, not the live claim.

## 16. Completion Checklist
- [ ] §9.0 M1–M4 honored (both signal forms accepted + ranked; PeerServedBlock primary; conflict →
      NoEvidence; forged hash sourced from `ForgeSucceeded`; closed vocabulary + allow-list;
      NoEvidence default).
- [ ] Closed `BA02Manifest`/`BA02Outcome`/`PeerAcceptEvent`/`PeerAcceptSource`/`NoEvidenceReason` +
      pure `correlate`.
- [ ] Exact-match positives (served, tip-only, both-agree) + all NoEvidence negatives (conflict /
      mismatch / chain-point / stale / weaker signals / empty) green.
- [ ] `ci_check_ba02_evidence_closed.sh` green + teeth-verified.
- [ ] Forge / recovery / sync / admission / produce_mode / bootstrap.rs / replay.rs unchanged.
- [ ] Live BA-02 recorded as an explicit open obligation (RO-LIVE-01 stays `partial`); candidate
      RO-LIVE-06 surfaced for cluster-close, separate from RO-LIVE-01, not appended in-slice.
- [ ] `cargo build` + scoped tests + named gate pass (full corpus lane excluded).

## 17. Review Notes
- **Invariant risk considered:** that a synthetic dry-run or a weaker live signal (agreement
  verdict / block_received / self-accept) or a conflicting tip silently becomes a BA-02 claim.
  Fenced four ways: the `BA02Manifest` constructor is contained to `correlate`'s exact-match arm;
  weaker signals are not `PeerAcceptEvent` variants; conflicting signals at the forged context →
  NoEvidence; the gate forbids a synthetic-built committed manifest.
- **Assumption challenged (M1):** "Haskell peer accept" has no direct wire line — the honest
  signals are the peer serving the forged block back (strongest) and the peer's tip naming it; the
  manifest records the capture provenance and refuses to let a tip paper over a served-block
  disagreement.
- **Follow-up implied:** the operator-gated live capture (feed a real peer-accept transcript to
  `correlate`, commit the resulting real manifest) supplies the evidence required to consider
  RO-LIVE-01 for enforcement; the status flip happens only after registry review at the appropriate
  close. L6 does not promise an automatic flip and does not enforce RO-LIVE-01.
