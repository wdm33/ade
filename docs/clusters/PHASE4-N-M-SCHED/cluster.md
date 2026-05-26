# PHASE4-N-M-SCHED — Era-schedule epoch-number wiring (cluster doc)

> **Status:** Planning. Single-slice strengthening of PHASE4-N-M-C
> bootstrap. Closes the `blocked_until_consensus_inputs_eta0_extraction`
> gate that held `DC-EVIDENCE-01` + `RO-LIVE-05` at
> `enforced_scaffolding` after FRAG closed.

**Provisional name at FRAG-closure was `PHASE4-N-M-NONCE`** —
the registry's `open_obligation` was tagged with that assumption.
The actual root cause turned out to NOT be the consensus-inputs
Eta0 extraction (cardano-cli's `epochNonce` IS Eta0 — same value
the corpus harness uses); it was an upstream bug:
`make_schedule_for_imported_window` hardcoded
`start_epoch: EpochNo(0)` instead of using
`canonical.epoch_no`. Renamed to `PHASE4-N-M-SCHED` for accuracy.

**Predecessors:** PHASE4-N-M-C (bootstrap + LiveLedgerView gating),
PHASE4-N-M-FRAG (wire-pump reassembly + tag-24 unwrap +
chain_dep epoch_nonce wiring).

**Successor:** none planned. After SCHED closes,
`DC-EVIDENCE-01` + `RO-LIVE-05` flip from `enforced_scaffolding`
to `enforced` with the committed BlockAdmitted + Agreed
transcript.

## §1 Primary invariant

> The admission-bootstrap era schedule MUST be constructed with
> the imported bundle's `epoch_no` as the schedule's
> `start_epoch`, NOT a hardcoded `EpochNo(0)`. After
> construction, `era_schedule.locate(slot).epoch` for any slot
> in the bundle's `[epoch_start_slot, epoch_end_slot)` window
> MUST equal `bundle.epoch_no` exactly — the same epoch the
> `LiveLedgerView` gates all per-pool lookups by.

### Why this matters

The admission runner's BLUE block-validity check
(`ade_ledger::block_validity::block_validity`) routes per-pool
data (stake, VRF keyhash, active-slots-coeff) through
`LedgerView::pool_active_stake(epoch, pool)`,
`LedgerView::pool_vrf_keyhash(epoch, pool)`, etc. The
`LiveLedgerView` impl in
`ade_runtime::consensus_inputs::view` gates EVERY lookup by
`epoch == self.inputs.epoch_no` — returning `None` for any
other epoch.

If the era schedule reports the wrong epoch for the block's
slot, every BLUE lookup returns `None`, and
`header_validate` short-circuits at the
`pool_active_stake.ok_or(VerificationFailed)?` line — surfacing
as `Header(VrfCert(VerificationFailed))` even though VRF +
nonce + pool data are all correct.

Diagnosis path (recorded for posterity): a stage-by-stage
`praos_vrf_verify_failed: stage=...` eprintln chain added to
header_validate revealed `stage=pool_active_stake_missing
epoch=0 issuer_pool=...` — pointing at the era schedule's
epoch arithmetic, NOT the nonce / VRF math. The diagnostic
was reverted from BLUE after the bug was identified (BLUE
must not do I/O); the RED-side `admission_admit_rejected`
diagnostic in `process_block` stays as the operator-facing
surface.

## §2 Scope

### In scope

- `crates/ade_node/src/admission/bootstrap.rs::make_schedule_for_imported_window`:
  add `epoch_no: EpochNo` parameter; pass `canonical.epoch_no`
  from the calling site.
- New unit/regression test asserting
  `era_schedule.locate(epoch_start_slot + N).epoch == epoch_no`
  for a non-zero epoch_no.

### Out of scope (explicit)

- Multi-era schedules (a future cluster — the bundle covers
  one epoch).
- Outbound-side schedule changes.
- consensus-inputs Eta0 extraction (cardano-cli's `epochNonce`
  is correct; no change needed).

## §3 Slice index

| Slice | Purpose | New rules / strengthenings |
|---|---|---|
| **S1** | Wire `canonical.epoch_no` into `make_schedule_for_imported_window` | strengthens existing CN-CONS-IN-01 + DC-VIEW-01; closes the `blocked_until_consensus_inputs_eta0_extraction` open_obligation on DC-EVIDENCE-01 + RO-LIVE-05 |

## §4 Exit criteria (cluster-level MACs)

1. `make_schedule_for_imported_window(epoch_start_slot, epoch_no)`
   exists; the calling site at the admission bootstrap passes
   `canonical.epoch_no`.
2. A unit test asserts that for a non-zero `epoch_no`,
   `era_schedule.locate(slot_in_window).epoch == epoch_no`.
3. C5 live operator pass against fully-synced docker preprod
   with `ADE_LIVE_REQUIRE_BLOCK_ADMITTED=1` produces:
   - ≥ 1 `admission_started` with `consensus_inputs_fingerprint`,
   - ≥ 1 `bootstrap_complete`,
   - ≥ 1 `block_admitted` with the fingerprint,
   - ≥ 1 `agreement_verdict { kind: "agreed" }`,
   - 0 `agreement_verdict { kind: "diverged" }`,
   - 0 `block_admitted` for any block whose hash differs from
     a peer-announced hash at the same slot.
4. Captured transcript committed at
   `docs/evidence/phase4-n-m-c-operator-pass-transcript.jsonl`.
5. `DC-EVIDENCE-01` + `RO-LIVE-05` flipped from
   `enforced_scaffolding` to `enforced`. `open_obligation`
   removed. `strengthened_in` += `PHASE4-N-M-SCHED`.
6. `cargo test --workspace` green — no regression in existing
   admission / consensus / replay tests.
7. Commit + push with the project-override
   `Co-Authored-By: Claude` trailer.

## §5 Hard prohibitions

- No silent drop of `epoch_no` from the bundle.
- No second `make_schedule_for_imported_window` authority.
- No I/O / eprintln in BLUE (`ade_core`, `ade_ledger`,
  `ade_codec`, `ade_crypto`). The header_validate diagnostics
  added during slice investigation MUST be reverted before
  close.
- No expansion of the schedule's era list beyond one entry —
  multi-era schedules are a separate slice.

## §6 Replay obligations preserved

- T-DET-01 — unchanged in shape; SCHED only fixes the
  epoch-number plumbing, not any state-transition function.
- DC-EVIDENCE-01 — flips to `enforced` per §4.
- RO-LIVE-05 — flips to `enforced` per §4.

## §7 References

- Diagnosis chain documented in
  `docs/evidence/phase4-n-m-c-operator-pass-README.md` §9
  + §10 (post-FRAG findings).
- Predecessor closures: `4d3dc98` (FRAG), `0016722` (A1.1
  follow-ups), `03d1d24` (A1.1), `8843e20` (N-M-C).
- Doctrine: [[feedback-shell-must-not-overstate-semantic-truth]],
  [[feedback-fail-closed-validation]],
  [[feedback-real-interop-finds-codec-bugs]].
