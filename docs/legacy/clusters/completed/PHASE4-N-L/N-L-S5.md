# Invariant Slice — PHASE4-N-L S5

**Slice Name:** RED `ade_network::mux::transport` extended — full-duplex bounded-queue async loop.
**Cluster:** PHASE4-N-L
**Status:** In Progress
**CEs addressed:** CE-N-L-6 (DC-SESS-04).
**Dependencies:** S1, S3.

## Intent

Turn `MuxTransport` into a real bidirectional duplex driver:
- `tokio::spawn`s a reader task that drains the TCP socket into a bounded `mpsc::Receiver<Vec<u8>>` for inbound bytes.
- `tokio::spawn`s a writer task that pulls from a bounded `mpsc::Receiver<Vec<u8>>` and writes to TCP.
- Surface a synchronous `MuxTransportHandle` with `next_inbound()`/`send_outbound()` that bridges to the session core.

Bounded queues enforce DC-SESS-04: overflow triggers session-level `PeerHaltReason::BackpressureExceeded` (the writer/reader tasks return Err; the orchestrator-side runner halts the peer).

## Scope

- `crates/ade_network/src/mux/transport.rs` — extend existing 40-LOC stub.
- Maintain `open_tcp(addr)`, `read_raw`, `write_raw` for back-compat (existing N-G consumers don't break).
- Add `MuxTransportDuplex::spawn(stream, inbound_capacity, outbound_capacity) -> MuxTransportHandle`.

```rust
pub struct MuxTransportHandle {
    pub inbound: mpsc::Receiver<Vec<u8>>,
    pub outbound: mpsc::Sender<Vec<u8>>,
    pub reader_handle: tokio::task::JoinHandle<Result<(), TransportError>>,
    pub writer_handle: tokio::task::JoinHandle<Result<(), TransportError>>,
}

pub enum TransportError {
    Io(io::ErrorKind),
    BackpressureExceeded,
}
```

## §12 Mechanical Acceptance Criteria

- [ ] `mux_transport_duplex_round_trips_bytes_over_loopback` — bind two ends, write 100 bytes on outbound, observe on inbound. (Uses `TcpListener` + `TcpStream` over `127.0.0.1`.)
- [ ] `mux_transport_duplex_inbound_overflow_returns_backpressure` — inbound queue capacity 1; writer floods → reader task surfaces `BackpressureExceeded`.
- [ ] `mux_transport_duplex_clean_shutdown_on_eof` — peer closes socket → reader returns Ok, writer exits.
- [ ] `ci/ci_check_session_no_unbounded.sh` — extended to assert no `unbounded_channel` in `mux/transport.rs`.

## §14 Hard Prohibitions

- No `mpsc::unbounded_channel` / `mpsc::unbounded`.
- No fork of `encode_frame`/`decode_frame` — this slice is byte-level only.
- No clock reads.

## §15 Non-Goals

- No TLS. No authentication. No reconnect logic.
- The duplex layer is byte-level only; framing is the session core's responsibility (S2).
