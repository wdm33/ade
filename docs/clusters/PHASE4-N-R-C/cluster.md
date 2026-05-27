# PHASE4-N-R-C — Bounty artifact run path (cluster doc)

> **Status:** Planning. 4-slice sub-cluster shipping the
> remaining N-R deliverables: real opcert envelope parser
> (consuming A1 OQ4 fixtures), real Shelley+Conway genesis
> closed-contract parser (consuming A1 OQ7 fixtures), legacy
> binary shim (`live_block_production_session.rs`), and
> bounty-facing operator-pass evidence capture.
>
> **Predecessor:** PHASE4-N-R-B (HEAD `f18fc9c`).
> **Successor:** none planned for the PHASE4-N-R cluster.
>
> **Inputs:** [`docs/planning/phase4-n-r-invariants.md`](../../planning/phase4-n-r-invariants.md)
> + [`docs/planning/phase4-n-r-cluster-slice-plan.md`](../../planning/phase4-n-r-cluster-slice-plan.md).

## §1 Primary invariant

> The opcert envelope parser accepts a real cardano-cli
> `node.opcert` text envelope (closed type check +
> documented CBOR shape per OQ4 fixtures) and produces a
> canonical `OperationalCert`. The genesis parser accepts
> real Shelley + Conway genesis JSON (closed required-field
> contract per OQ7 fixtures) and produces a canonical
> `GenesisAnchor`. The legacy
> `live_block_production_session.rs` binary is a thin
> deprecation shim. The bounty-facing operator-pass evidence
> (Ade-forged block accepted by cardano-node) is captured
> per docs/active/cn-cons-06-operator-runbook.md, gated on
> the deferred bridges (KES-signs-real-unsigned-header +
> MuxPump outbound-relay).

## §2 Honest-scope framing

The bounty artifact (`cardano-node accepts an Ade-forged
block` over N2N) requires **two open bridges** that are
deferred to future clusters:

1. **KES-signs-real-unsigned-header bridge** — currently
   `run_real_forge`'s step 3 KES-signs a placeholder
   payload (`expected_vrf_input` bytes). The real Praos
   protocol requires KES to sign the CBOR-encoded unsigned
   header body. Until this bridge lands, `self_accept`
   structurally rejects the forged block (correctly — the
   KES signature won't verify against the real header
   bytes).
2. **MuxPump outbound-relay extension** — currently
   `produce_mode::dispatch_server_frame_event` computes
   response bytes but cannot transmit them back to the
   peer. Until the MuxPump refactor lands, peers cannot
   fetch forged blocks via block-fetch.

**N-R-C ships the parsers + shim** so the operator-facing
binary surface is in place. C4's evidence-capture step is
**explicitly gated** on the two bridges landing; if either
is still open at C4 close, `CN-CONS-06` and `RO-LIVE-01`
retain their `partial` / `enforced` + open_obligation
status, and N-R-C closes with documented blockers (not a
falsified bounty win).

## §3 Slice index

| Slice | Purpose | Closes (invariant IDs) |
|---|---|---|
| **C1** | Real opcert envelope parser (consumes A1 OQ4 fixtures). Closed type check + CBOR `array(2)` decode + extract `OperationalCert` from element 0 + optionally cold VK from element 1. | `CN-OPCERT-01`, `DC-OPCERT-01` |
| **C2** | Real Shelley+Conway genesis closed-contract parser (consumes A1 OQ7 fixtures). Required-field exact; no implicit defaults; no stringly fallback; extra-key tolerance. | `CN-GENESIS-01`, `DC-GENESIS-01` |
| **C3** | `live_block_production_session.rs` shim. Prints `"DEPRECATED: ..."` banner, delegates to `produce_mode::run_produce_mode`. CI gate verifies no independent forge codepath remains. | `N10` (no independent legacy path) |
| **C4** | Cluster close. Operator-pass evidence capture if both bridges have landed; otherwise documented gated close (CN-CONS-06 / RO-LIVE-01 retain open_obligation; CN-PROD-02's "opcert + genesis parsers" sub-item closes). | sub-cluster close + cluster close |

## §4 Exit criteria

- [ ] CE-1: Opcert envelope parser passes the 4 OQ4
  golden-fixture cases (accepted, malformed-type,
  malformed-cborHex, wrong-arity).
- [ ] CE-2: Genesis parser passes the 6 OQ7 golden-fixture
  cases (accepted shelley + accepted conway + missing-required
  + stringly-int + extra-inert-key + malformed-numeric).
- [ ] CE-3: `live_block_production_session.rs` is a thin
  shim; first line printed is the documented deprecation
  banner; delegates to `produce_mode::run_produce_mode`.
- [ ] CE-4: Registry: `CN-OPCERT-01`, `CN-GENESIS-01`,
  `DC-OPCERT-01`, `DC-GENESIS-01` flipped to `enforced`.
- [ ] CE-5: `CN-PROD-02.open_obligation` updated — opcert
  parser + genesis parser sub-items closed.
- [ ] CE-6: `cargo test --workspace --lib` clean.
- [ ] CE-7: PHASE4-N-R cluster-level close documented —
  3 sub-clusters shipped (N-R-A + N-R-B + N-R-C); 13 new
  rules enforced; 5 carry-forward strengthenings; 2 named
  bridges deferred to future clusters.

## §5 References

- N-R-A close: [[project-phase4-n-r-a-closed]].
- N-R-B close: [[project-phase4-n-r-b-closed]].
- A1 OQ4 fixtures: `crates/ade_runtime/tests/fixtures/opcert/`.
- A1 OQ7 fixtures: `crates/ade_runtime/tests/fixtures/conway_genesis/`.
- Operator runbook: `docs/active/cn-cons-06-operator-runbook.md`.
