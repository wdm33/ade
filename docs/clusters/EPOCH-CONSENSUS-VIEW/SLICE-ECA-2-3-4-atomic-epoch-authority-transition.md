# SLICE ECA-2/3/4 — Atomic Epoch Authority Transition

Part of EPOCH-CONTINUITY-ACTIVATION. The CORE slice: it makes the (already-built, hermetic)
activation machinery LIVE as ONE authoritative state transition. Built on ECA-0a/0b (the
leadership-complete candidate), ECA-1 (no semantic gate, `a17c7aab`), ECA-2-pre (the v4 sidecar
persists the consensus-profile hashes, `124c87da` — so every activation input is recoverable from
canonical durable state). ECA-5 (the live boundary proof) stays separate.

## Why ONE slice (user, 2026-06-22 — constitutional, not convenience)

> construct deterministic inputs → durably record activation → atomically publish the authority →
> recover that same authority after crash

is ONE authoritative state transition. **ECA-4 (recovery) is NOT cleanup**: without it the
activation WAL is only HALF an authority contract — the node can write a promoted epoch identity but
cannot prove that restart reconstructs the same validation + leadership view. ECA-2 alone
(activate-or-halt, result unconsumed) and even ECA-2+3 (restart-after-activation under-specified) are
**forbidden production states**. The activation-WAL-without-reader state is an inert hermetic STAGING
point ONLY; it MUST DISAPPEAR in this slice. Substeps 2/3/4 may be reviewed separately, but none is a
complete or deployable activation capability alone.

## Scope

**Inputs:** canonical durable state ONLY — the v4 seed sidecar (`SeedEpochConsensusInputs`, carrying
the consensus-profile hashes per DC-CINPUT-06), the live reduced checkpoint, and the durable ChainDB
selected chain. No CLI/config/genesis re-supply.

**Transition (at the deterministic epoch boundary):**
1. derive the bound candidate — durable window replay → `EpochConsensusView` bound to N+1.
2. verify the deterministic predicate — candidate exists + bindings match the selected chain + source
   window complete + readiness valid + activation WAL durable.
3. append the activation WAL (`EpochConsensusViewActivated`) — durable BEFORE publication.
4. atomically publish ONE owned `ActiveEpochAuthority` (Seed → Promoted).

**Consumers (both resolve the SAME epoch-versioned holder):**
- header validation (`run_node_sync`),
- leadership / forge decision (the DC-EPOCH-03 wall).
Both resolve `authority.ledger_view()` at each authoritative decision — never a retained stale borrow.

**Recovery (warm-start):** rebuild EXACTLY the same promoted authority from the WAL
`EpochConsensusViewActivated` record + the bound durable inputs (`recover_active_view`); reject
mismatch or ambiguity TERMINALLY — never a seed fallback past a recorded promotion.

## Design

- `ActiveEpochAuthority` — owned by the relay loop, epoch-versioned:
  `SeedView { epoch, view: PoolDistrView }` | `PromotedView { epoch, view: PoolDistrView,
  activation_binding }`. `authority.ledger_view() -> &dyn LedgerView` resolves the current view.
- The relay loop OWNS it, replacing the borrowed `ledger_view: &dyn LedgerView` parameter of
  `run_relay_loop_with_sched`. Callers construct `SeedView` from the seed `PoolDistrView`.
- At the wall (`maybe_activate_epoch_boundary`): `activate_at_boundary` → derive → predicate → WAL →
  on Promote, build the `PoolDistrView` from the promoted `EpochConsensusView` via
  `to_pool_distr_view(genesis_hash, protocol_params_hash, asc)` (the recovered v4 sidecar's hashes +
  asc — DC-EPOCH-12) → ATOMICALLY replace `SeedView` → `PromotedView`. All subsequent reads resolve
  Promoted.
- Both `run_node_sync` (header validation) AND the forge wall resolve `authority.ledger_view()` per
  decision — the borrowed `&dyn LedgerView` no longer outlives the swap.
- Warm-start: `recover_active_view` reconstructs the promoted authority from the WAL + bound inputs;
  a restart mid-N+1 resumes on the self-derived promoted view (no re-import, no re-arm).

(The exact wiring — the `run_relay_loop_with_sched` signature change, the per-call resolution at the
header-validation + forge sites, and the 5 callers — is pinned by the ledger_view-flow investigation
and refined here as implemented.)

## Invariant

- **DC-EPOCH-14 (new):** ONE owned, epoch-versioned `ActiveEpochAuthority` is the SOLE view source for
  BOTH header-validation and leadership; the seed→promoted swap is atomic + durable-before-visible;
  recovery reconstructs the EXACT promoted authority from the WAL + bound durable inputs (mismatch /
  ambiguity terminal, never a seed fallback past a recorded promotion); NO stale borrowed `ledger_view`
  outlives the swap; the non-EVIEW + same-epoch paths are byte-identical.
- Strengthens DC-EPOCH-04/05/06/10 (the activation machinery is now LIVE-WIRED + consumed + recovered,
  not observe-only), DC-EPOCH-12 (`to_pool_distr_view` consumed live), DC-EPOCH-13 (automatic +
  consumed, not fire-and-drop).

## Mechanical acceptance (hermetic — ALL required)

1. simulated boundary: N view → N+1 promoted view.
2. both header validation AND leader election read the SAME epoch-versioned holder.
3. crash BEFORE WAL → the old view remains active.
4. crash AFTER WAL, before publication → recovery publishes the recorded promoted view.
5. crash AFTER publication → recovery produces a byte-identical active authority.
6. restart CANNOT consult CLI/config/genesis files to reconstruct the profile hashes (DC-CINPUT-06).
7. NO stale borrowed `ledger_view` can outlive the swap.
8. the same-epoch path remains byte-identical.
- `ci/ci_check_eview_atomic_authority.sh`.

## Merge claim (exact)

"Automatic epoch-authority transition and recovery are hermetically implemented. Real continuous
Preview operation remains UNPROVEN until ECA-5."

## Out of scope

ECA-5 (the live 1335→1336 boundary proof — the unchanged production binary crosses a real boundary
with no manual intervention). The 3 pre-existing stale gates (ECA-0a / -mat / -wire) — cluster-close.
