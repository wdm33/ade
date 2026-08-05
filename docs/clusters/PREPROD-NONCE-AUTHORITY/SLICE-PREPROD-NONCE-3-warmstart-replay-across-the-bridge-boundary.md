# SLICE PREPROD-NONCE-3 — warm-start replay cannot re-validate the seed→seed+1 BRIDGE boundary block

> **SEALED — OBSERVED, NOT DIAGNOSED.** Found 2026-08-05 immediately after CE-N2-4 went green: the
> first restart of a store that has actually crossed its bootstrap-bridge boundary. Blocks **CE-N2-5**
> (restart recovers the same eta0) and therefore preprod LIVE-2. Deliberately not investigated further
> here — it is a different surface from the nonce-authority defect CE-N2-4 fixed, and conflating them
> is how a correct fix gets reverted.

## What happened

CE-N2-4's live run crossed preprod 304→305 correctly and left a durable store whose tip is in epoch
305 and whose WAL holds the (correct) activation record. Restarting that store fails before the relay
loop:

```
nonce1-freeze-window: source=durable-sidecar k=2160 f=1/20 store_rsw=Some(172800)
                      cli_rsw=Some(172800) cross_check=agreed effective_rsw=Some(172800)
ade_node --mode node: warm-start recovery failed in the bootstrap authority
  (Materialize(ReplayFailedAt { slot: SlotNo(130118424),
                                error: Header(VrfCert(VerificationFailed)) }));
  failing closed. The recovered sidecar did not verify; no bundle fallback.
rc=42
```

`130118424` is the **first block of epoch 305** — the bridge-boundary block itself.

## This is NOT the CE-N2-4 defect, and not a regression of it

Stated explicitly because the two are one slot apart and a reader will be tempted to merge them:

| | CE-N2-4 (fixed) | this slice (open) |
|---|---|---|
| surface | which eta0 the bridge BINDS into the durable activation record | whether the warm-start REPLAY can re-validate the boundary block |
| path | `epoch_wire::bind_bridge_view` (live promotion + recovery) | `node_lifecycle::warm_start_recovery` → materialize replay → header validate |
| status | **GREEN, live-proven** — record commits `74f10bea…` == cardano-node | fails closed, rc=42 |

The CE-N2-4 diff does not touch `warm_start_recovery`, the materialize replay, or header validation.
It became *reachable* only because the boundary can now be crossed at all: before CE-N2-4 the node
halted AT the boundary (`rc=43`), so no post-bridge-boundary store had ever existed to restart from.
This is the next blocker revealed, not a new one introduced.

## What is already ruled out by measurement

- **Not the R4c RSW-inert cause.** `5e83aaaa` (CE-4A.3-R4c) fixed a warm-start replay whose
  `RSW=None` left `CANDIDATE_FREEZE_INERT`, over-tracking the candidate into a wrong `eta0(N+1)` and
  exactly this `VrfCert` symptom. Here the RSW **is** present and cross-checked: the run's own
  `nonce1-freeze-window` line reports `store_rsw=Some(172800) cli_rsw=Some(172800) cross_check=agreed`.
  So the known cause of this symptom is excluded before any new theory starts.
- **Not a wrong committed nonce.** The record the replay is reconciling against holds
  `74f10bea…`, verified against `cardano-node query protocol-state` — see
  `docs/evidence/run-stores/preprod-nonce-1/ce-n2-4-boundary-green.log`.

## The untested-path hypothesis (NOT selected — this is the open work)

The bridge boundary is promoted LIVE by `prepare_authority_for_candidate_slot`, using the bridge's
**imported MARK leadership** — the seed+1 stake/pool/VRF set that exists only in the bridge record. The
warm-start materialize replay re-validates blocks without that promotion step. If it validates the
epoch-305 header against the SEED epoch's leadership (or a chain-dep that never took the bridge
boundary), the header's VRF cannot verify.

That would make this the **restart half of ECA-5**, never exercised: ECA-5 (`26565bec`) proved a
native-Mithril node SURVIVES its first boundary on preview, but a restart ACROSS that bridge boundary
is not in the proof record, and preprod had never crossed one at all until today.

Not selected, because `VrfCert(VerificationFailed)` is consistent with both a wrong nonce and wrong
leadership, and the run does not emit the operands needed to separate them. Naming a cause here would
be the mistake PREPROD-NONCE-1 spent a full re-bootstrap cycle correcting.

## First step when this is picked up

Emit the replay's per-block chain-dep operands at the boundary block the way `nonce1-boundary-operands`
does for the live tick — candidate / evolving / last-epoch-block / computed eta0, plus which leadership
set the header check resolved against. One instrumented restart then separates "wrong nonce" from
"wrong leadership" without a re-bootstrap, because the store is already in the failing state.

## Reproducer

| | |
|---|---|
| store | `~/.cardano-live1/ade-preprod-n2` (tip in epoch 305, activation record present, semantics v2) |
| seed | `~/.cardano-live1/preprod-snapshot-6009`, certified slot 129813427 |
| restart log | `docs/evidence/run-stores/preprod-nonce-1/ce-n2-5-warmstart-replay-blocked.log` |
| green run log | `docs/evidence/run-stores/preprod-nonce-1/ce-n2-4-boundary-green.log` |

The store reproduces the failure directly — no re-bootstrap needed. **Keep it** until this closes.

## Not claimed

No fix, no root cause, no invariant, no CE. This records a reproducible failure with its evidence, and
records what it is NOT: it is not the nonce-authority defect, and CE-N2-4 is not reopened by it.
