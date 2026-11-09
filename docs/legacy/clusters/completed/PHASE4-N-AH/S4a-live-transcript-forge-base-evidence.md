# Invariant Slice — Live transcript forge-base evidence (PHASE4-N-AH S4a)

## 2. Slice Header
**Slice Name:** Live transcript forge-base evidence — make the DC-NODE-20 forge-base decision auditable in the `--mode node` JSONL transcript
**Cluster:** PHASE4-N-AH — local selected durable chain forge-base authority; **rung-1, single-producer only**
**Status:** Proposed
**Authority source:** `docs/clusters/PHASE4-N-AH/cluster.md` (CE-AH-6 prerequisite); the S4 run-1 partial (`docs/evidence/phase4-n-ah-live-run-1-partial.md`) surfaced that the live transcript surface is too weak to witness claims 2/3/4. Registry: `CN-NODE-04` (closed sched-event vocabulary) — strengthened; `DC-NODE-20`/`DC-NODE-21` observed, not changed.

**Cluster Exit Criteria Addressed:**
- [ ] **CE-AH-6 prerequisite (run-2 code half):** the `--mode node` live transcript directly witnesses the forge-base decision — `forge_base_source = local_chaindb_tip`, the entered forge mode, `cert_path_present = false`, and self-admit via `pump_block` — as a closed-vocabulary JSONL written to the `--log` file. (S4a does not itself close CE-AH-6; it makes run-2 able to.)

S4a is a **prerequisite slice** for S4; it does not flip DC-NODE-20/21 (close), and it is not the live run.

## 3. Implementation Instruction (AI — INLINE)
**RED evidence only.** Extend the closed `NodeSchedEvent` vocabulary (`live_log/sched_event.rs`) with a `ForgeBaseSelected` event + enrich `ForgeResult`; emit them at the existing `ForgeTick`/self-admit site (`node_lifecycle.rs ~1161–1413`); wire `NodeSchedLogWriter` to the `--log` JSONL file (`cli.log_path`) so `node-run.jsonl` is the canonical transcript (stderr may stay as a secondary operational sink). Keep `NodeSchedEvent` `Copy` (Copy field types only). Update the allow-list + the closed-vocab negative test; add a hermetic transcript test + `ci/ci_check_live_transcript_forge_base.sh`. **No BLUE change. No forge-authority / chain-selection / durable-state / WAL change** — S4a *observes and serializes the decision already made*. §12 is the completion proof. Commit carries the repo's model trailer.

## 4. Intent
The S4 run-1 proved the DC-NODE-20 path works live, but the transcript only carried `forge_result{outcome}` — so the **exact invariant under test** (forge base = `ChainDb::tip`, cert-free, self-admit via `pump_block`, direct extend mode) was only *implied* by the relay's adoption, not directly witnessed. S4a makes that decision **auditable**: it serializes the already-made forge-base choice into the closed `--mode node` evidence vocabulary, so run-2 can produce a transcript that mechanically satisfies claims 2/3/4 of the CE-AH-6 bar. This adds an evidence surface; it changes no authority.

## 5. Scope
- **Extend `NodeSchedEvent`** (`live_log/sched_event.rs`) with one explicit event + two closed sub-enums; enrich `ForgeResult` (§ Event shape).
- **Emit** `ForgeBaseSelected` at the `ForgeTick` decision site (where `selected_tip = ChainDb::tip`, `forge_mode`, `followed_peer_tip` are in scope) and the enriched `ForgeResult` at the self-admit (`admit_forged_block_durably`) outcome.
- **Wire** `NodeSchedLogWriter` to the `--log` JSONL file (`node_lifecycle.rs:452/622`), making `node-run.jsonl` the canonical transcript.
- **Encoder** (`live_log/sched_writer.rs`): exhaustive arms for the new event/fields + `push_key_{u64,bool}` + hash-as-hex helpers; update `SCHED_DISCRIMINATORS`.
- **Tests + gate** (§12).
- **Out of scope:** any BLUE/forge-authority/chain-selection/durable-state/WAL change; the harness warm-start leg (operator scratch, run-2); the live run; flipping DC-NODE-20/21 (CE-AH-7).

## 6. Execution Boundary (TCB color)
- **BLUE (UNCHANGED):** ledger / ChainDb / `pump_block` / the forge-base decision — observed, not modified.
- **GREEN:** the closed `NodeSchedEvent` vocabulary + its byte-deterministic encoder (the existing `CN-NODE-04` emit-only diagnostic surface — "operational/diagnostic tier ONLY, never a consensus/acceptance signal").
- **RED:** `node_lifecycle.rs` emit calls + the `--log` file sink.
- **Emit-only** (`ci_check_node_sched_events_emit_only.sh`): the planner never reads a `NodeSchedEvent`; best-effort `record` (emit errors swallowed) never alters control flow. No new authority of any color.

## 7. Invariants Preserved (registry IDs)
`DC-NODE-20` (forge base = local durable tip — observed verbatim, never changed) · `DC-NODE-21` (cert evidence-only — the transcript records `cert_path_present=false`, it does not read a cert) · `DC-NODE-05`/`DC-NODE-12` (`pump_block` durable admit) · `DC-NODE-15`/`18`-core/`19`-core · `DC-CONS-03` · `T-REC-03`/`T-REC-05` (the sched stream is operational-tier, never replay-equivalence-weighted — the durable/WAL/served surfaces are untouched) · **`CN-NODE-04`** (the closed emit-only sched vocabulary — extended, fail-closed-on-unknown preserved).

## 8. Invariants Strengthened or Introduced
**Strengthens `CN-NODE-04`** — the closed `--mode node` sched-event vocabulary gains a `ForgeBaseSelected` event + an enriched `ForgeResult`, making the DC-NODE-20 forge-base decision auditable in the canonical `--log` JSONL transcript, while preserving closed/emit-only/fail-closed-on-unknown. Exactly **one** family (the CN-NODE-04 evidence vocabulary). This is the run-2 code prerequisite for witnessing CE-AH-6 claims 2/3/4; it introduces no new authority.

## 9. Event shape (per the approved design)
One explicit event (no overloading), plus a minimal `ForgeResult` enrichment where the durable-admit outcome is naturally known. `NodeSchedEvent` stays `Copy` — every field is a Copy type (closed enums, `Hash32`, `u64`, `bool`).
```
// new closed sub-enums (Copy, as_str discriminators)
enum ForgeBaseSource { LocalChaindbTip }            // "local_chaindb_tip"
enum ForgeModeKind   { InitialCatchupRequired, CaughtUpToPeerTip, SingleProducerExtendOwnDurableSpine }

NodeSchedEvent::ForgeBaseSelected {
    forge_mode: ForgeModeKind,                       // = SingleProducerExtendOwnDurableSpine
    forge_base_source: ForgeBaseSource,              // = local_chaindb_tip
    forge_base_hash: Hash32,                          // emitted hex
    forge_base_block_no: u64,
    followed_peer_tip_block_no: Option<u64>,         // Copy; the divergent peer tip
    followed_peer_tip_hash: Option<Hash32>,
    cert_path_present: bool,                          // = false
}

NodeSchedEvent::ForgeResult {
    outcome: ForgeOutcome,                            // existing
    self_admit_via_pump_block: bool,                  // = true on Succeeded (admit_forged_block_durably)
    entered_forge_mode: ForgeModeKind,
}
```

## 10. Changes Introduced
- `live_log/sched_event.rs`: `ForgeBaseSource` + `ForgeModeKind` (closed, Copy, `as_str`); `ForgeBaseSelected` variant; `ForgeResult` fields; `discriminator()` arm; allow-list test updated.
- `live_log/sched_writer.rs`: `encode_event` arms for `ForgeBaseSelected` + the enriched `ForgeResult`; `push_key_u64`/`push_key_bool` + hash-hex; `SCHED_DISCRIMINATORS += "forge_base_selected"`; writer serialization test.
- `node_lifecycle.rs`: emit `ForgeBaseSelected` in the `ForgeTick` arm; enrich the `ForgeResult` emit with self-admit/entered-mode; wire `NodeSchedLogWriter` to the `--log` file.
- `ci/ci_check_node_sched_events_emit_only.sh`: allow-list += `forge_base_selected`.
- `ci/ci_check_live_transcript_forge_base.sh` (new).

## 11. Replay, Crash, and Epoch Validation
- **Replay:** unchanged — the sched stream is operational-tier, never persisted / replay-visible / replay-weighted (the WAL/durable/served bytes are untouched). The S3 `local_spine_*` byte-identity tests stay green.
- **Epoch:** not applicable.

## 12. Mechanical Acceptance Criteria
- [ ] `node_sched_event_allowlist_rejects_unknown_variants` (updated) green — the vocabulary == the allow-list incl. `forge_base_selected`; a new/unknown variant fails closed.
- [ ] New hermetic test `forge_base_selected_transcript_witnesses_local_tip` — drive the local-spine forge with a `NodeSchedLogWriter<Vec<u8>>` sink; assert the emitted JSONL contains `ForgeBaseSelected{forge_base_source=local_chaindb_tip, forge_base_hash, forge_base_block_no, cert_path_present=false}` and `ForgeResult{self_admit_via_pump_block=true, entered_forge_mode=single_producer_extend_own_durable_spine}`.
- [ ] `ci/ci_check_live_transcript_forge_base.sh` (new) green — asserts (against the committed hermetic sample transcript + the source): `ForgeBaseSelected` is in the closed vocabulary; the encoder emits `forge_base_source` / `cert_path_present`; the emit site is the `ForgeTick` arm; `ForgeResult` carries `self_admit_via_pump_block`; **no adoption-cert token** in the sched path; `forge_base_source == local_chaindb_tip` and `cert_path_present == false` in the sample.
- [ ] `cargo test -p ade_node` green (incl. the writer + emit-only tests).
- [ ] `ci_check_node_sched_events_emit_only.sh` green (allow-list updated; planner still never reads a `NodeSchedEvent`).
- [ ] `ci_check_cert_evidence_only.sh` + `ci_check_local_durable_forge_base.sh` + `ci_check_node_run_loop_containment.sh` + `ci_check_node_path_fidelity.sh` stay green.
- [ ] `DC-NODE-20` + `DC-NODE-21` still `declared`.

## 13. Failure Modes
None to authority — a sched-emit error is swallowed and never alters the loop. A test failure means the emit site/encoder don't carry the forge-base evidence (the whole point) and is caught hermetically.

## 14. Hard Prohibitions
**Inherited (cluster §8):** no cert in the forge path; no new authority of any color; no fork-choice.
**Slice-specific:**
- **RED evidence only** — no BLUE / forge-authority / chain-selection / durable-state / WAL change; S4a only observes + serializes.
- **Keep `NodeSchedEvent` `Copy`** and emit-only (the planner never reads it).
- **Do not** make the transcript a consensus/acceptance/BA-02/replay signal (operational tier only).
- **Do not** run `cargo fmt -p ade_node`; **do not** touch the pre-existing-stale `ci_check_forge_followed_tip_admission.sh`.

## 15. Explicit Non-Goals
The live run-2 (the operator pass) · the harness warm-start leg · flipping DC-NODE-20/21 (CE-AH-7 close) · the competing-block fence broadening (AH-FOLLOW-1) · any forge-base/authority behavior change.

## 16. Completion Checklist
- [ ] `ForgeBaseSelected` + enriched `ForgeResult` in the closed vocabulary; encoder + emit site + `--log` wiring done; `Copy` preserved.
- [ ] Hermetic transcript test + `ci_check_live_transcript_forge_base.sh` green; all sched/AH/path-fidelity gates green; `cargo test -p ade_node` green.
- [ ] `DC-NODE-20` + `DC-NODE-21` still `declared`.
