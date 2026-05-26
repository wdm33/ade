# Invariant Slice — PHASE4-N-L S7

**Slice Name:** RED `ade_runtime::network::n2n_dialer` — outbound TCP + handshake → `PeerConnected`.
**Cluster:** PHASE4-N-L
**Status:** In Progress
**CEs addressed:** end-to-end CE-N-L-2 + CE-N-L-3 over real socket.
**Dependencies:** S4, S5, S6.

## Intent

Connect to a peer address; run the handshake driver to completion; spawn a `MuxPump` for the negotiated session; emit `PeerConnected { peer_id, ..., role: UpstreamClient }` to the orchestrator.

## Scope

- `crates/ade_runtime/src/network/n2n_dialer.rs` — new RED file.

```rust
pub struct N2nDialer {
    pub peer_addr: SocketAddr,
    pub our_versions: VersionTable,
    pub peer_id_generator: Arc<PeerIdGenerator>,
    pub events_out: mpsc::Sender<OrchestratorEvent>,
}

impl N2nDialer {
    pub async fn dial(self) -> Result<(), DialError>;
}

pub enum DialError {
    Io(io::ErrorKind),
    Handshake(HandshakeError),
    OrchestratorDropped,
}
```

`dial()` opens `TcpStream::connect(peer_addr).await`, spawns the duplex transport (S5), runs the handshake driver inside `tokio::task::spawn_blocking` (so the sync trait works under async), then constructs the `MuxPump` over the now-`Connected` SessionState and spawns it.

## §12 Mechanical Acceptance Criteria

- [ ] `n2n_dialer_loopback_handshake_succeeds` — bind a `TcpListener`; spawn a tiny responder task on the listener that mirrors the dialer's proposed versions; `dial()` returns `Ok(())` and the orchestrator inbox sees `PeerConnected`.
- [ ] `n2n_dialer_returns_io_error_on_unreachable_target` — connect to a closed port → `DialError::Io`.
- [ ] `n2n_dialer_returns_handshake_error_on_disjoint_versions` — responder offers only versions the dialer doesn't propose → `DialError::Handshake`.

## §14 Hard Prohibitions

- No `mpsc::unbounded_channel`.
- No reimplementation of `n2n_transition`.
- No `unwrap()` / `panic!()` in production code.

## §15 Non-Goals

- No TLS.
- No reconnect / retry loop.
- No discovery / peer-sharing.
