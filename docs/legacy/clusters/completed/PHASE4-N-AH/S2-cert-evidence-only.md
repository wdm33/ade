# Invariant Slice — Adoption certificate is evidence-only (DC-NODE-21 S2)

## 2. Slice Header
**Slice Name:** Adoption certificate is evidence-only, never forge authority (DC-NODE-21, PHASE4-N-AH S2)
**Cluster:** PHASE4-N-AH — local selected durable chain forge-base authority; **rung-1, single-producer only**
**Status:** Proposed
**Authority source:** `docs/clusters/PHASE4-N-AH/cluster.md` (§4, CE-AH-3); registry `DC-NODE-21` (declared — **not flipped** by this slice)

**Cluster Exit Criteria Addressed:**
- [ ] **CE-AH-3:** `read_adoption_cert` removed from the forge-base/`proceed_to_forge` path; cert **writing** preserved only as transcript/evidence capture; `ci/ci_check_cert_evidence_only.sh` asserts the cert is never read into the forge path and never appears in multi-producer/preprod/production forge paths.

Exit criteria not listed (CE-AH-1/2/5=S1; CE-AH-4=S3; CE-AH-6=S4; CE-AH-7=close) are out of scope.

**Slice Dependencies:** S1 (`b0fb8817`) — removed the cert from the forge *decision*; the parser is `#[allow(dead_code)]` awaiting this slice.

## 3. Implementation Instruction (AI — INLINE)
**Stricter route (user-confirmed): the adoption certificate leaves `ade_node` entirely as forge input — the operator harness owns cert/evidence parsing.** Delete `read_adoption_cert` + `parse_hex32` + `VenueAdoptionCertificate` + the now-unused `SingleProducerFenceReason::AdoptionCertificateMissingOrMalformed` + the `adoption_cert_path` field + `declare_single_producer_venue`'s cert param + the `--adoption-cert-path` CLI flag. Update `ci_check_node_path_fidelity.sh` 28 → 29. Compiler-guided: update the test call sites that pass a cert path. Add the gate. `DC-NODE-21` stays `declared`. §12 is the completion proof. Commit carries the repo's model trailer.

## 4. Intent
Make it **mechanically impossible** for the operator adoption certificate to act as forge authority in `ade_node`. S1 removed the cert from the forge *decision*; S2 removes the cert *parsing surface* from the node entirely — there is no `ade_node` code path that reads or interprets the cert. The cert remains a real operator artifact (the harness writes it into the evidence bundle), but it is **outside** the node's forge authority. This closes the authority-creep vector: dead cert-parsing inside `ade_node` is exactly what could be re-wired into the forge path later.

## 5. Scope
- **Delete (node forge surface):** `node_lifecycle::read_adoption_cert` + `parse_hex32`; `node_sync::VenueAdoptionCertificate` (struct); `node_sync::SingleProducerFenceReason::AdoptionCertificateMissingOrMalformed` (unused since S1); the `VenueAdoptionCertificate` import in `node_lifecycle`.
- **Full removal of the cert-path config:** delete the `adoption_cert_path` field (`ForgeActivation`), `declare_single_producer_venue`'s `cert_path` param, and the `--adoption-cert-path` CLI flag (`cli.rs`); update `ci_check_node_path_fidelity.sh`'s pinned flag-set **28 → 29**.
- **Tests:** update the `node_sync`/`node_lifecycle` test call sites that pass a cert path (the N-AG S2 `s2_extend_lead` / idle tests + `declare_single_producer_venue(Some(..))`), dropping the cert.
- **New gate:** `ci/ci_check_cert_evidence_only.sh`.
- **Out of scope:** the harness cert-write / evidence tooling (lives outside the repo; updated later to stop passing `--adoption-cert-path`); any forge-base change (S1); flipping DC-NODE-21 (close).

## 6. Execution Boundary (TCB color)
- **BLUE (UNCHANGED):** ledger / ChainDb / `pump_block` — no cert ever reached BLUE.
- **GREEN:** `ade_node::node_sync` — delete the `VenueAdoptionCertificate` type + the unused fence-reason variant.
- **RED:** `ade_node::node_lifecycle` (delete the cert parsers + the field + plumbing) + `ade_node::cli` (remove the `--adoption-cert-path` flag).

## 7. Invariants Preserved (registry IDs)
`DC-NODE-20` (forge base = local durable tip — the cert was already off the forge path in S1; S2 only removes its parsing) · `DC-NODE-05` / `DC-NODE-12` (`pump_block` sole durable admit authority) · `DC-NODE-15` (initial catch-up gate, untouched) · `DC-NODE-18` core / `DC-NODE-19` core (own-spine forge / continue-past-EOF — both already cert-free after S1) · **`CN-REHEARSAL-FIDELITY-01`** (the `cli.rs` flag set remains a closed allow-list — now 29; the `--mode node` path + the no-from-genesis guard are unchanged) · `DC-CONS-03` · `T-REC-03`/`T-REC-05` (the cert was never persisted/replay-visible; removing it changes no replay surface).

## 8. Invariants Strengthened or Introduced
**Introduces `DC-NODE-21`** (the adoption cert is rung-1 evidence-only, never forge authority) as mechanically enforced: no `ade_node` forge-authority code reads/parses the cert, asserted by `ci_check_cert_evidence_only.sh`. Exactly **one** invariant family (cert evidence-only). `DC-NODE-21` flips declared→enforced at `/cluster-close` (CE-AH-7), not here.

## 9. Design Summary
The cert parsing surface is **deleted** from `ade_node`, not fenced. After S2, a `grep` for the cert tokens in the node's forge-authority regions returns nothing. The cert remains: (1) a file the operator harness may write; (2) parsed/written by harness/evidence tooling outside the node; (3) included in the bounty/transcript evidence bundle. The node neither reads nor interprets it. The `--adoption-cert-path` flag is gone (it was never in the pinned set); the pinned allow-list reconciles to 29 by adding the pre-existing legitimate `--single-producer-venue` (see §10 premise correction); a stored-unread field is not left behind.

## 10. Changes Introduced
- **Types/fns deleted:** `read_adoption_cert`, `parse_hex32`, `VenueAdoptionCertificate`, `AdoptionCertificateMissingOrMalformed`.
- **Config deleted:** the `adoption_cert_path` field + `declare_single_producer_venue` cert param + the `--adoption-cert-path` CLI flag (`cli.rs`); `ci_check_node_path_fidelity.sh` pinned set **28 → 29** (premise correction below). The rung1-auto harness must stop passing `--adoption-cert-path` (a harness edit, outside the repo — done later).
- **Path-fidelity premise correction (implementation `050237e9`):** the pinned set never contained the N-AF DC-NODE-18 flags. `--adoption-cert-path` was **never** pinned (so removing it changed nothing there); the real divergence was the legitimately-missing `--single-producer-venue`. The reconciliation is therefore **28 → 29** (add the venue flag), NOT the 28 → 29 assumed elsewhere in this doc. CN-REHEARSAL-FIDELITY-01 preserved (closed allow-list; no from-genesis/devnet flag). See cluster.md §12.
- **Gate added:** `ci/ci_check_cert_evidence_only.sh`.
- **Tests:** drop the cert path from the affected call sites (compiler-guided).

## 11. Replay, Crash, and Epoch Validation
- **Replay:** unchanged — the cert was admissibility-only, never persisted / replay-visible; the existing replay gates (`ci_check_node_run_loop_containment.sh`, the T-REC tests) stay green.
- **Epoch:** not applicable.

## 12. Mechanical Acceptance Criteria
- [ ] `ci/ci_check_cert_evidence_only.sh` (new) green — FAILS if any of `read_adoption_cert | adoption_cert_path | parse_hex32 | VenueAdoptionCertificate | AdoptionCertificate` appears in `ade_node` forge-authority regions (`run_relay_loop_with_sched`, `proceed_to_forge`, `ForgeMode`, `single_producer_forge_decision`) outside explicitly whitelisted evidence/test/harness paths.
- [ ] `cargo test -p ade_node` green (all existing tests, with the cert-path call sites updated).
- [ ] `ci_check_local_durable_forge_base.sh` + `ci_check_single_producer_extend_own_spine.sh` + `ci_check_single_producer_loop_continuation.sh` + `ci_check_node_run_loop_containment.sh` stay green.
- [ ] `ci_check_node_path_fidelity.sh` green (pinned flag-set reconciled to 29 — see §10 premise correction).
- [ ] `grep -rE 'read_adoption_cert|parse_hex32|VenueAdoptionCertificate' crates/ade_node/src` outside `#[cfg(test)]`/harness is empty.
- [ ] `DC-NODE-21` still `declared`; `DC-NODE-20` untouched.

## 13. Failure Modes
None introduced — S2 only deletes dead/cert-input surface. A test failure means a cert-path call site was missed (compiler-caught).

## 14. Hard Prohibitions
**Inherited (cluster §8):** no cert read in the forge path; no new BLUE authority; no fork-choice in the fence; no weakening of DC-NODE-15 / DC-NODE-20.
**Slice-specific:**
- **No cert parsing anywhere in `ade_node` forge authority** — the harness owns it.
- **Do not** re-introduce the cert into `proceed_to_forge` / `ForgeMode` / the forge decision.
- **Do not** weaken `CN-REHEARSAL-FIDELITY-01` — the flag set stays a closed allow-list (just one smaller); no from-genesis / devnet flag added.
- **Do not** touch the pre-existing-stale `ci_check_forge_followed_tip_admission.sh` (cluster.md §12 / AH-FOLLOW).

## 15. Explicit Non-Goals
The forge-base authority (S1) · replay (S3) · the live re-homed pass (S4) · flipping DC-NODE-21 (close) · the competing-block fence broadening (AH-FOLLOW-1) · the operator harness cert-write/evidence tooling (outside the repo).

## 16. Completion Checklist
- [ ] Cert parsers + types + the field + the flag deleted from `ade_node`; `ci_check_node_path_fidelity.sh` 28→29 (+`--single-producer-venue`; see §10).
- [ ] `ci_check_cert_evidence_only.sh` green; the AH gates + `ci_check_node_path_fidelity.sh` green; `cargo test -p ade_node` green.
- [ ] `DC-NODE-21` still `declared`; `DC-NODE-20` untouched.
```
