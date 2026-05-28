# PHASE4-N-M-SCHED — Closure record

**Closed:** 2026-05-27
**Closure HEAD:** (set on commit)
**Predecessor HEAD:** `4d3dc98` (PHASE4-N-M-FRAG).

## Goal

Close the gate `blocked_until_consensus_inputs_eta0_extraction`
that held `DC-EVIDENCE-01` + `RO-LIVE-05` at
`enforced_scaffolding` after FRAG. With the FRAG-era diagnostic
chain in place, the live operator pass against the fully-synced
peer surfaced `Header(VrfCert(VerificationFailed))` at the
`pool_active_stake_missing` stage — pointing at the
era-schedule's epoch-number plumbing, not the
consensus_inputs Eta0 extraction the gate name assumed.

## Outcome

**DC-EVIDENCE-01 + RO-LIVE-05 fully `enforced`.** The C5 live
operator pass against the local docker `cardano-node-preprod`
peer (epoch 291, fully synced) now produces the complete
load-bearing transcript shape:

```
admission_started     consensus_inputs_fingerprint_hex=8166ba41…
bootstrap_complete    initial_ledger_fp_hex=65461b64…
                      chain_tip_slot=124136968
block_received        peer=127.0.0.1:3001
                      slot=124140368
                      block_hash=d111e613…
block_admitted        slot=124140368
                      block_hash_hex=d111e613…
                      post_fp_hex=9528023e…
                      consensus_inputs_fingerprint_hex=8166ba41…
agreement_verdict     kind=agreed
                      slot=124140368
                      our_hash_hex=peer_hash_hex=d111e613…
                      consensus_inputs_fingerprint_hex=8166ba41…
admission_shutdown    reason=signal_received
```

Meeting the DC-EVIDENCE-01 statement exactly:
- AT LEAST: 1 AdmissionStarted ✓, 1 BootstrapComplete ✓, ≥ 1
  BlockAdmitted ✓, ≥ 1 AgreementVerdict { kind: "agreed" } ✓
- AT MOST: 0 AgreementVerdict { kind: "diverged" } ✓, 0
  mismatched-hash BlockAdmitted ✓.

## Root cause

The gate's expected location (consensus_inputs Eta0 extraction)
was **WRONG**; cardano-cli's `query protocol-state`
`epochNonce` IS Eta0 — same shape the corpus harness uses.
The actual bug was upstream:

```rust
// Before SCHED:
fn make_schedule_for_imported_window(epoch_start_slot: &SlotNo) -> EraSchedule {
    EraSchedule::new(
        ...,
        vec![EraSummary {
            ...
            start_epoch: EpochNo(0),  // ← BUG: hardcoded to 0
            ...
        }],
    )
}
```

With `start_epoch = 0`, the era schedule reports
`locate(slot).epoch == 0` for every slot in the imported
window. `LiveLedgerView`'s correct epoch-window guard (gates
all per-pool lookups by `epoch == inputs.epoch_no`) refused
every lookup → `header_validate` short-circuited at the
`pool_active_stake.ok_or(VerificationFailed)` line. The error
surface (`Header(VrfCert(VerificationFailed))`) is identical
to a true VRF math failure, so without a per-stage diagnostic
the bug was easy to misdiagnose as a nonce issue.

Diagnosis chain (reverted from BLUE after the bug was
identified; the BLUE doctrine forbids I/O):

```
admission_admit_rejected: slot=124137791 ...
                          error=Header(VrfCert(VerificationFailed))
praos_vrf_verify_failed: stage=pool_active_stake_missing
                         epoch=0
                         issuer_pool=9772d8814b1fdf3d…
```

`epoch=0` is the smoking gun — `LiveLedgerView` would only ever
serve data for epoch 291.

## What shipped

### BLUE / GREEN code

- `crates/ade_node/src/admission/bootstrap.rs`:
  - `make_schedule_for_imported_window` now takes a second
    parameter `epoch_no: EpochNo` and uses it as
    `EraSummary::start_epoch`.
  - Calling site passes `canonical.epoch_no`.
  - `EpochNo` added to the `ade_types` import.
  - New unit test
    `imported_window_schedule_uses_bundle_epoch` proves
    `schedule.locate(slot_in_window).epoch == bundle_epoch`
    for a non-zero `epoch_no` (regression bound).

### Tests

- `crates/ade_node/src/admission/bootstrap.rs::tests::imported_window_schedule_uses_bundle_epoch`.
- Existing 36 session-reducer tests still green (FRAG).
- Existing 26 seed-import tests still green (A1.1).
- `cargo test --workspace` clean.

### Diagnostics

The BLUE-side `praos_vrf_verify_failed: stage=...` eprintln
chain that surfaced the bug was REVERTED after the fix. The
BLUE doctrine (`ade_core`, `ade_ledger`, `ade_codec`,
`ade_crypto`) must not do I/O / side effects.

The RED-side diagnostics added during FRAG investigation
remain:
- `wire_pump::finalize` prints
  `admission_wire_pump: peer=<addr> exit=<result>`
- `runner::process_block` prints
  `admission_decode_block_failed: prefix=… error=…` on
  Undecodable
- `runner::process_block` prints
  `admission_admit_rejected: slot=… error=…` on Invalid

These three RED diagnostics carry the typed error and the
relevant fields without leaking BLUE state.

### Evidence

- `docs/evidence/phase4-n-m-c-operator-pass-transcript.jsonl`
  (committed) — the load-bearing transcript above.
- `docs/evidence/phase4-n-m-c-operator-pass-README.md` §10
  (added) — closure runbook + the diagnosed-then-reverted
  diagnostic chain.

## Registry effects

- **`DC-EVIDENCE-01`**: `status` flipped from
  `enforced_scaffolding` to `enforced`. `open_obligation`
  removed. `strengthened_in` += `PHASE4-N-M-SCHED`.
  `evidence_notes` extended with the committed transcript
  binding + root-cause narrative.
- **`RO-LIVE-05`**: same shape (closure).
- **`CN-CONS-IN-01`**: `strengthened_in` +=
  `PHASE4-N-M-SCHED`. `evidence` extended.
- **`DC-VIEW-01`**: `strengthened_in` += `PHASE4-N-M-SCHED`.
  `evidence` extended.

## Open obligations (post-SCHED)

- **RO-LIVE-03** (wide live admission-agreement against an
  arbitrary peer over a 30-minute window): still open.
  DC-EVIDENCE-01 + RO-LIVE-05 closed the BOUNDED version
  against the local docker preprod.
- **RO-LIVE-04** (live wire smoke against a private peer):
  closed by PHASE4-N-L-LIVE.
- **RO-GENESIS-REPLAY-01**: still open.
- **RO-MITHRIL-IMPORT-01**: still open.
- **Multi-epoch admission**: future cluster.
- **ChainDb persistence of admitted blocks**: future
  strengthening.
- **Block production live pass**: future cluster.

## What's NOT in this cluster

- Multi-era schedules (the bundle covers one epoch by
  design).
- Outbound-side schedule changes.
- Consensus-inputs Eta0 extraction (cardano-cli's
  `epochNonce` IS Eta0; no bundle-builder change needed).
- BLUE VRF impl changes (the impl works on both corpus and
  live blocks once fed correct epoch routing).

## References

- Cluster doc: `docs/clusters/PHASE4-N-M-SCHED/cluster.md`.
- Slice doc: `docs/clusters/PHASE4-N-M-SCHED/S1.md`.
- Predecessor closures: `4d3dc98` (FRAG), `0016722` (A1.1
  follow-ups), `03d1d24` (A1.1), `8843e20` (N-M-C).
- Doctrine: [[feedback-shell-must-not-overstate-semantic-truth]],
  [[feedback-fail-closed-validation]],
  [[feedback-evidence-reducers-are-green-not-authority]].
