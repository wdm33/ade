# Follow-up: producer-corpus gate Guard 3 decision (CI semantics)

> **Status:** Decision note for a small, standalone CI change.
> **Not** PHASE4-N-F; **not** to be bundled with produce-mode wiring planning.
> Surfaced 2026-05-29 while doing the `e8bde40` docs/CI hygiene pass.

## Finding

`ci/ci_check_producer_corpus_present.sh` (the CN-CONS-06 mechanical-half gate)
is **red on `main`** for a reason the `e8bde40` archived-path fix did **not**
resolve. After that fix the gate advances to **Guard 3**, which asserts that
`crates/ade_core_interop/src/bin/live_block_production_session.rs` parses the
CLI flags `--cold-skey`, `--kes-skey`, `--vrf-skey`, `--opcert`, `--target`.

That binary parses **none** of them (`grep -oE '"--[a-z-]+"'` over it returns
empty). The operator block-production path moved to **`ade_node --mode
produce`** — the registry now even describes `live_block_production_session` as
"superseded by `ade_node --mode produce`" (corrected in `e8bde40`). So Guard 3
is verifying the dead interface of a superseded binary.

(Reproduce: `bash ci/ci_check_producer_corpus_present.sh` — multiple
`FAIL: Guard 3 (live_block_production_session does not parse …)`.)

## The decision (Guard 3 changes what the gate proves)

This is **not** a casual patch — it changes what the CI gate actually attests.
Choose explicitly:

- **(A) Repoint Guard 3 at the live surface (`ade_node --mode produce`).**
  Assert the *current* operator-production CLI in `crates/ade_node/src/cli.rs`
  parses the real required flags (per the C1 scoping doc §"How … is invoked":
  `--listen`, `--cold-skey`, `--kes-skey`, `--vrf-skey`, `--opcert`,
  `--genesis-file`, `--json-seed`, `--consensus-inputs-path`, `--peer`,
  `--snapshot-store`). This keeps a real "operator-production entry parses its
  inputs" guard, now pointed at the binary the operator pass uses.
- **(B) Retire the legacy-binary assertion as obsolete.** If
  `live_block_production_session` is fully dead, drop Guard 3 (and Guard 4's
  procedure-doc references, if they only describe the legacy binary), leaving
  the gate to prove the replay-corpus + cross-impl-adapter + registry-shape
  guards (1, 2, 5) that remain meaningful. Optionally remove the dead binary in
  a separate cleanup.

**Recommendation:** (A) if the operator pass is genuinely staged against
`ade_node --mode produce` (it is — see the C1 scoping doc), so the gate keeps
proving something real about the live surface. Fall back to (B) only if the
legacy binary is being deleted.

Either way, also re-check **Guard 4** (it greps the procedure doc for
`live_block_production_session`): if Guard 3 repoints/retires, Guard 4's
expectations likely need the same treatment.

## Scope fence

- Standalone CI change. **Separate from PHASE4-N-F.** Tied to **CN-CONS-06**
  (mechanical half) — the gate's registry-shape Guard 5 already passes
  (CN-CONS-06 `enforced` + the three `cross_impl_adapter` tests).
- The `e8bde40` archived-path fix (`docs/clusters/completed/PHASE4-N-C/…`) is
  already done; this note covers **only** Guard 3 (+ Guard 4 follow-on).
- No produce-mode behavior change implied; this is about CI assertions.

Relates to: `CN-CONS-06`, `RO-LIVE-01`, `ade_node --mode produce`,
[[feedback_produce_subordinate_to_sync_spine]].
