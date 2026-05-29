# `corpus/sync/preprod_snapshot_to_tip_synthetic`

Forward-sync replay corpus for PHASE4-N-Y S2 (CE-Y-7,
`forward_sync_replay_two_runs_byte_identical`).

## What this is

A **synthetic / representative** in-tree fixture for the S2
replay-equivalence property:

> same verified anchor + same ordered block sequence
> → byte-identical post-state ledger fingerprint + byte-identical WAL.

S2 does not require a real preprod snapshot→tip capture. The block
sequence is **reused from the committed validity corpus**
(`corpus/validity/conway_epoch576/`, loaded via
`ade_testkit::validity::ConwayValidityCorpus::load`). The S2 replay
test drives the forward-sync pump
(`ade_runtime::forward_sync::pump_block`) over that ordered sequence
twice from the same anchor and asserts the two runs are byte-identical
on both surfaces.

## Anchor

The replay origin is a fixed synthetic anchor fingerprint
(`Hash32([0xA0; 32])`) seeding the WAL fingerprint chain, plus the
Conway epoch-576 `eta0` from the validity corpus. The anchor here is
the S1 `BootstrapAnchor` stand-in for the durable-write ordering
proof; binding verification is S1's concern.

## Out of scope (deferred)

- **Real preprod snapshot→tip capture** — S5 (compatibility evidence
  bundle) + the operator-witnessed pass (CE-Y-16). The live transcript
  is captured against a fully-synced Haskell peer, not in CI.
- Crash/restart recovery over this sequence — S3.

## Regeneration

The block bytes are not duplicated here; they live in
`corpus/validity/conway_epoch576/`. To extend the sequence, add blocks
to that corpus and they flow into this replay test automatically.
