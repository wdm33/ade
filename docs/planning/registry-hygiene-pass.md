# Registry & doc hygiene pass (handoff)

> Deferred to a fresh session (decided 2026-05-28). Surfaced by the
> grounding-doc regen at HEAD `e681baa` (TRACEABILITY now traces all 291 rules
> and flags these inline). These are **pre-existing** gaps (predate N-T/N-V) —
> append-only registry/doc reconciliation, no BLUE/code behavior change.

## Pick-up state
- HEAD `e681baa`; working tree clean. N-T + N-V closed; N-W (Praos VRF,
  `CN-FORGE-04`) declared but NOT started. All four grounding docs current at
  `22eef90`. Registry: 291 rules.
- **Do this hygiene pass BEFORE N-W.** Read the four grounding docs first
  (lesson: [[feedback-read-grounding-docs-first]]); TRACEABILITY's inline
  `[not found on disk — drift]` markers are the authoritative gap list.
- Commit discipline: attribution trailer required → write msg to a file, `git
  commit -F <file>` (heredoc/`-m` with the trailer is blocked by the scrubber).
  Close gate must be **unmasked** `cargo test --workspace` (RO-CLOSE-01) — never
  pipe through tail/grep for pass/fail.

## Item 1 — reconcile 9 rules naming renamed/relocated tests (registry↔code drift)
For each, grep the cited file for the CURRENT fn name and update the rule's
`tests` array (replace the stale name with the renamed one; this is justified
reconciliation, not unjustified removal — the test still exists under a new
name; note the rename in the commit). Verify each against the actual code.
- **DC-CONS-04, DC-CONS-10** (`crates/ade_core/src/consensus/{op_cert,praos_state}.rs`): stale `op_cert_upsert_rejects_equal_counter`, `op_cert_upsert_accepts_strictly_increasing`, `apply_op_cert_rejects_equal_counter`. After the N-M-FOLLOW op-cert fix: equal-counter is `..._accepts_equal_counter_as_noop`; rejection is `op_cert_upsert_rejects_regression` / `apply_op_cert_rejects_lower_counter`.
- **DC-CRYPTO-01** (`crates/ade_crypto/src/{blake2b,ed25519,vrf}.rs`): stale `blake2b_256_rfc_test_vectors`, `ed25519_rfc_8032_test_1`, `vrf_ietf_draft03_section_a3`. KATs were split (e.g. `blake2b_256_empty` / `_abc` / `_single_byte`) — grep current names.
- **DC-CONSENSUS-02** (`crates/ade_core/src/consensus/leader_schedule.rs`): `is_leader_for_vrf_output_delegates_to_vrf_cert` relocated (see comment ~:292).
- **DC-EPOCH-02** (`crates/ade_testkit/tests/transition_proof_surface.rs` exists): `translation_summary_proof`, `translation_comparison_surface`, `transition_proof_surface` not found as fns — grep current.
- **DC-REF-01**: `provenance_validation`, `self_comparison_block_fields` not found — grep.
- **T-CORE-01**: `apply_block_never_returns_ok` not found — grep.
- **T-ENC-03**: `full_corpus_round_trip` not found — grep.
- **CN-SESS-01** (`crates/ade_network/src/mux/frame.rs`): `tests::round_trip_canonical_frames` not found under that name — grep.

## Item 2 — bind real-invariant CI scripts to rules (24 unbound `ci_script`)
24 `ci/ci_check_*.sh` exist but aren't in any rule's `ci_script`. Bind the ones
that enforce a rule (append to the rule's `ci_script`); leave operator tooling
unbound. Confirmed bind candidates: `ci_check_no_independent_forge_codepath.sh`
→ CN-FORGE-01/02; `ci_check_node_mode_closure.sh` → CN-NODE-01;
`ci_check_ingress_chokepoints.sh` → T-INGRESS-01/DC-INGRESS-01;
`ci_check_no_secrets.sh`, `ci_check_n2n_server_no_signing_dep.sh`,
`ci_check_orchestrator_core_purity.sh`, `ci_check_pallas_quarantine.sh`,
`ci_check_module_headers.sh`, `ci_check_receive_orchestrator_no_producer_dep.sh`,
`ci_check_ce_n_a_5_proof.sh`, `ci_check_genesis_replay_open_obligation.sh`, and
the N-M family (`ci_check_admission_*`, `ci_check_live_consensus_inputs_*`,
`ci_check_lagging_is_evidence_only.sh`, `ci_check_admit_replay_equivalence.sh`,
`ci_check_adversarial_false_accept_corpus.sh`) — match each to its rule by
reading the script header + the rule statements. **Leave unbound (operator
tooling, not gates):** `build_consensus_inputs_bundle.sh`,
`mithril_restore_preprod_peer.sh`.

## Item 3 — archive closed cluster docs
`docs/clusters/PHASE4-N-{Q,R-A,R-B,R-C,S-A,S-B,S-C}/` are closed but still under
`docs/clusters/` (N-T and N-V were archived; these were missed). `git mv` each to
`docs/clusters/completed/`. (CODEMAP/SEAMS generators sourced their colors from
banners+registry because these dirs lack the standard `## §5 TCB color map`
heading N-T/N-V use — optional: not worth back-filling.)

## Item 4 — RO-CLOSE-01 CI meta-gate (decide)
`RO-CLOSE-01` is procedurally enforced with no CI gate. Optionally add
`ci/ci_check_unmasked_close_gate.sh` (e.g. grep cluster-close transcripts /
forbid piped close-gate invocations) and bind it. Or leave procedural — user's
call.

## Close
After Items 1–2 (registry edits) + 3 (archive): re-run `/traceability` (the
registry `tests`/`ci_script` arrays changed) to confirm the drift markers clear
and the new bindings show; CODEMAP/SEAMS/HEAD_DELTAS are unaffected by these
edits (re-run only if a binding surfaces a new module). Then commit
(`docs(registry): reconcile test-name drift + bind CI gates + archive N-Q/R/S`).
Validate `cargo test --workspace` unmasked stays green (no code changed, but
RO-CLOSE-01 applies).
