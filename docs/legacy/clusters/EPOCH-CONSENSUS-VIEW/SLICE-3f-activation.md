# EPOCH-CONSENSUS-VIEW — S3f / DC-EVIEW-08 (scope): activation (the ONLY live-path change)

> **Status:** SCOPED (2026-06-20, pre-code). The activation slice — the ONLY slice that touches the live producer path. Decomposed into fail-safe sub-slices, each gated so NO live behavior changes until the final live-activation step, which is itself gated on the two LIVE cardano-node proofs. The riskiest slice in the cluster; correctness-first.

## Intent
Wire the observe-only S3a–S3e logic into the live producer so the NEXT epoch's leader election reads Ade's own derived `EpochConsensusView` instead of the seeded `PoolDistrView`. This is where a bug = wrong block production / wrong fork, so it is decomposed so each step is fail-safe and the live flip is last + proof-gated.

## Decomposition (fail-safe order)
- **S3f-1 — boundary consumption point (BLUE, hermetic; NO live change).** Thread the S3c aggregate into `apply_epoch_boundary_with_registrations` as `Option<&StakeByPool>`: `Some(agg)` ⇒ `new_mark = form_mark_snapshot(agg)` (the real per-pool stake); `None` ⇒ the existing stub. `apply_epoch_boundary_full` (the live path) passes `None`, so the live boundary is UNCHANGED. Proven hermetically (the aggregate-fed mark == the aggregate's pool_stakes). This makes the boundary ABLE to consume the aggregate without changing live behavior.
- **S3f-2 — the window driver (GREEN/RED).** The missing driver that PRODUCES a real aggregate at a boundary over a real chain: bootstrap-reduce (S3b-1) → windowed-advance one epoch (S3b-2) → aggregate (S3c) → `StakeByPool`. `track_utxo=true` lives only in this transient window. Validated by the boundary-aligned differential oracle (Ade's aggregate over the boundary state == the fresh `stakeMark`) — the fully-clean pool proof owed from DC-EVIEW-05.
- **S3f-3 — the `ledger_view` rebind seam (RED).** `run_relay_loop_with_sched` (node_lifecycle.rs:~1386) today borrows `ledger_view` IMMUTABLY for the loop's life — no rebind. Add a per-epoch rebind seam so the next epoch's leadership can read a swapped-in derived view; and the DC-EPOCH-03 wall (node_sync.rs:1544) admits epoch N+1 slots WHEN a bound+stable view for N+1 exists, instead of fail-closing.
- **S3f-4 — the live activation + replay-equivalence (RED; the live flip).** Add a DISTINCT WAL activation variant (a new `WalEntry` + tag, preserving the bootstrap `SeedEpochConsensusInputsImported` single-import; relax `DuplicateProvenance` `replay.rs:170` ONLY for the new variant), satisfying DC-WAL-03 (two-run byte-identical). Bind the emitted `EpochConsensusView` (S3e) + consult the S3d stability gate, then feed it to live leader election (`PoolDistrView` from the derived view, `node_sync.rs:1560`). **GATED (binding): NOT entered until S3f-1..3 merge AND both live proofs pass — (1) the boundary-aligned stake oracle, (2) the leadership-schedule live proof (ADE1's derived schedule across a real boundary == cardano-cli leadership-schedule + a forge on the new epoch).**

## Hard constraints (binding)
- NO live producer-path behavior change until S3f-4, and S3f-4 only after both live proofs.
- `track_utxo=true` stays inside the S3f-2 transient window; the live follow/forge path stays `track_utxo=false`.
- The activation is replay-equivalent (DC-WAL-03): a WAL-recorded activation reproduces the same bound view + the same leader decisions across two runs; NO network/wall-clock/rand in BLUE.
- CE-71 (reward accounting, correct-vs-corpus, never live) makes its first live-path appearance here — gated on deterministic replay + crash/recovery + the differential results + no change to current live consensus decisions.
- Fail-closed: an unbound / mismatched / not-yet-stable view is INERT (S3e `matches` + S3d `is_boundary_stable`); the producer never elects on a view that does not match the activation context or whose boundary is not `> k` deep.

## Where it sits
S3a–S3e (committed, reviewed) built + proved the logic, observe-only; the S3c stake oracle is live-validated (reduction byte-exact, aggregation formula confirmed). S3f wires it live in fail-safe steps, the live flip last + proof-gated. This is the slice that flips DC-EPOCH-03's single-epoch containment into bound cross-epoch production — the BA02-adjacent end state.
