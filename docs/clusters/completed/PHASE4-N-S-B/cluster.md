# PHASE4-N-S-B — MuxPump outbound relay (cluster doc)

> **Status:** Planning. 4-slice sub-cluster shipping the
> closed `OutboundCommand` enum + `MuxPump::outbound_relay`
> field + `PerPeerOutbound` shared map + `dispatch_server_frame_event`
> wiring + loopback integration test. Closes the
> "MuxPump outbound-relay extension" bridge listed in
> N-R-B's `CN-PROD-01.open_obligation`.
>
> **Predecessor:** PHASE4-N-R close (HEAD `c02aefc`) +
> N-S planning (HEAD `72bcda3`).
>
> **Closure type:** MECHANICAL (loopback test, no real
> peer required).
> **Parallel to N-S-A:** module trees are disjoint
> (B touches `ade_runtime::network::mux_pump` +
> `ade_node::produce_mode`; A touches `ade_ledger` +
> `ade_runtime::producer`).

## §1 Primary invariant

> Reducer outputs from `dispatch_server_frame_event`
> traverse the existing `MuxPump` session-aware encoder
> before reaching the peer's TCP socket. `produce_mode`
> never holds an `mpsc::Sender<Vec<u8>>` writing directly
> into `MuxTransportHandle::outbound`. The only outbound
> API is `peer_outbound.get(&peer_id)?.send(OutboundCommand
> { ... })`. The branded `OutboundCommand` closed enum
> carries typed `ChainSyncServerMsg` / `BlockFetchServerMsg`
> — no `Vec<u8>` byte tunnel.

## §2 Doctrine: typed-commands-only outbound

```
dispatch_server_frame_event (GREEN reducer composition)
  ↓ produces typed ServerReply variants
OutboundCommand::{ChainSync,BlockFetch,ClosePeer}
  ↓ Arc<RwLock<BTreeMap<PeerId, Sender<OutboundCommand>>>>
  ↓ per-peer mpsc::Sender (looked up by PeerId)
MuxPump::outbound_relay (mpsc::Receiver<OutboundCommand>)
  ↓ tokio::select! arm
  ↓ MuxPump's session-aware encoder (existing path)
  ↓ MuxTransportHandle::outbound (raw bytes channel)
  ↓ duplex writer task
  ↓ TCP socket
```

The session-aware encoder is the **only** producer of
wire-byte streams. `produce_mode` decides *what* to send
(BLUE/GREEN authority); MuxPump serializes *how*
(RED transport authority).

## §3 Slice index

| Slice | Purpose | Closes (invariant IDs) |
|---|---|---|
| **B1** | Planning + 3 candidate registry entries (`CN-OUTBOUND-RELAY-01`, `CN-PEER-OUTBOUND-MAP-01`, `DC-OUTBOUND-FIFO-01`) declared. OQ-S-B audit of `MuxPump::run` shape after adding outbound receiver. | — (declarative) |
| **B2** | Closed `OutboundCommand` enum + `MuxPump::outbound_relay: Option<mpsc::Receiver<OutboundCommand>>` field + `tokio::select!` integration. New `MuxPump` is backwards-compatible — `None` outbound_relay = dialer mode (existing behavior). | `CN-OUTBOUND-RELAY-01`, N4, N8, D3 |
| **B3** | `PerPeerOutbound = Arc<RwLock<BTreeMap<PeerId, mpsc::Sender<OutboundCommand>>>>` type alias + insertion/removal in `run_per_peer_session` (PeerConnected emit) and `MuxPump::emit_peer_disconnected`. `dispatch_server_frame_event` extended with `&PerPeerOutbound` parameter; reducer outputs converted to `OutboundCommand::{ChainSync,BlockFetch}` and enqueued through the per-peer sender. Closed `DispatchError::{UnknownPeer, PeerOutboundMissing, SendFailure}`. | `CN-PEER-OUTBOUND-MAP-01`, I5, N5, D4 |
| **B4** | Loopback integration test (synthetic dialer ↔ Ade listener; reply bytes byte-identical) + CI grep gate `ci/ci_check_no_produce_mode_direct_transport_writes.sh`. Sub-cluster close. Flip 3 N-S-B rules + clear `CN-PROD-01.open_obligation` final remainder. | `DC-OUTBOUND-FIFO-01`; sub-cluster close |

## §4 OQ-S-B audit (recorded at plan time)

`MuxPump::run` is currently a `loop { let chunk = match
self.transport.inbound.recv().await { ... } }`. Adding
`outbound_relay: Option<mpsc::Receiver<OutboundCommand>>`
converts the loop to `tokio::select!`:

```rust
loop {
    tokio::select! {
        chunk = self.transport.inbound.recv() => {
            // ... existing inbound handling ...
        }
        cmd = async {
            match self.outbound_relay.as_mut() {
                Some(rx) => rx.recv().await,
                None => std::future::pending().await,
            }
        } => {
            // Encode OutboundCommand via existing
            // session-aware encoder, write to
            // self.transport.outbound.
        }
    }
}
```

`std::future::pending()` for the `None` case means the
outbound arm never fires when MuxPump is in dialer mode —
existing dialer behavior preserved without conditional
select! arms.

**No double-mut-borrow of `transport.outbound`:** the
existing inbound handler uses `route_effect` which mutates
`self.transport.outbound` via `send`. The new outbound arm
also calls `send`. Both are async sends through the same
sender — no aliasing, since `mpsc::Sender::send(&self)`
takes `&self`, not `&mut self`.

## §5 Exit criteria (CI-verifiable)

- [ ] CE-B-1: `OutboundCommand` closed enum lands at
  `ade_runtime::network::mux_pump::OutboundCommand`
  (or sibling module).
- [ ] CE-B-2: `MuxPump::outbound_relay` field added;
  existing dialer-mode tests still pass.
- [ ] CE-B-3: `PerPeerOutbound` type alias + per-peer
  map populated by `run_per_peer_session`.
- [ ] CE-B-4: `dispatch_server_frame_event` extended;
  closed `DispatchError` variants.
- [ ] CE-B-5: Loopback test `outbound_relay_byte_identity_against_synthetic_dialer`
  passes — synthetic dialer issues `RequestRange` against
  Ade listener; received `Block(bytes)` is byte-identical
  to the snapshot's admitted bytes.
- [ ] CE-B-6: CI grep gate `ci/ci_check_no_produce_mode_direct_transport_writes.sh`
  passes — no module in `ade_node::produce_mode` writes
  directly into `MuxTransportHandle::outbound`.
- [ ] CE-B-7: 3 N-S-B rules flipped to `enforced`;
  `CN-PROD-01.open_obligation` final remainder cleared
  (the listener-side per-peer dispatch + transmit closure
  is complete).
- [ ] CE-B-8: `cargo test --workspace --lib` clean.

## §6 References

- Predecessor cluster: PHASE4-N-R-B
  ([[project-phase4-n-r-b-closed]]).
- Cluster plan: [`../../planning/phase4-n-s-cluster-slice-plan.md`](../../planning/phase4-n-s-cluster-slice-plan.md).
- Doctrine: [[feedback-fail-closed-validation]] (closed
  enum + structured DispatchError), [[feedback-shell-must-not-overstate-semantic-truth]]
  (MuxPump's authority is byte serialization, not
  protocol-message decision).
