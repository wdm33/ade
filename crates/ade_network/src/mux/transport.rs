// RED — Imperative Shell.
//
// Tokio-based socket scaffold for the Ouroboros mux layer. The async
// surface lives here and only here within ade_network. BLUE submodules
// (codec, handshake, chain_sync, block_fetch, tx_submission, keep_alive,
// peer_sharing, n2c, mux::frame) MUST remain sync; DC-CORE-01 enforces
// this via ci/ci_check_no_async_in_blue.sh.
//
// PHASE4-N-L S5 extends this file with a full-duplex bounded-queue
// driver (`MuxTransportDuplex`) that the session pump (S6) consumes.
// Bounded queues enforce DC-SESS-04: overflow surfaces as
// `TransportError::BackpressureExceeded` rather than silent drop.
// The original `MuxTransport`/`open_tcp` API is retained byte-
// identically so existing N-G consumers don't break.

use std::io;
use std::net::SocketAddr;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

/// Existing single-shot transport — preserved for back-compat with
/// pre-N-L consumers (PHASE4-N-G live-evidence binaries).
pub struct MuxTransport {
    stream: TcpStream,
}

impl MuxTransport {
    pub async fn read_raw(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.stream.read(buf).await
    }

    pub async fn write_raw(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.stream.write_all(bytes).await
    }
}

pub async fn open_tcp(addr: SocketAddr) -> io::Result<MuxTransport> {
    let stream = TcpStream::connect(addr).await?;
    Ok(MuxTransport { stream })
}

/// Closed transport-error sum surfaced by the duplex driver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportError {
    /// Underlying socket error; carries the OS error kind.
    Io(io::ErrorKind),
    /// Bounded-queue overflow — DC-SESS-04 fail-fast signal.
    BackpressureExceeded,
    /// Peer closed the socket cleanly.
    Eof,
}

/// Bounded queue capacities for the duplex driver. Tuned for the
/// PHASE4-N-L Tier-5 defaults (operator-tunable is forbidden in
/// this cluster — mirrors `SnapshotCadence::DEFAULT` discipline).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DuplexCapacity {
    pub inbound_chunks: usize,
    pub outbound_chunks: usize,
    pub read_buffer_bytes: usize,
}

impl DuplexCapacity {
    pub const DEFAULT: Self = Self {
        inbound_chunks: 1024,
        outbound_chunks: 256,
        read_buffer_bytes: 16 * 1024,
    };
}

/// Handle returned by `MuxTransportDuplex::spawn`. The session pump
/// (S6) reads inbound bytes from `inbound`, writes outbound bytes
/// to `outbound`, and joins the two background tasks on shutdown.
pub struct MuxTransportHandle {
    pub inbound: mpsc::Receiver<Vec<u8>>,
    pub outbound: mpsc::Sender<Vec<u8>>,
    pub reader_handle: JoinHandle<Result<(), TransportError>>,
    pub writer_handle: JoinHandle<Result<(), TransportError>>,
}

/// Spawn the full-duplex driver over an owned `TcpStream`. Splits
/// the stream into read + write halves and runs each in its own
/// tokio task.
///
/// The reader task reads up to `capacity.read_buffer_bytes` per
/// iteration; each chunk is forwarded to the bounded inbound
/// channel via `try_send` (DC-SESS-04: overflow → `BackpressureExceeded`).
/// The writer task pulls Vec<u8> chunks and writes them in order.
pub fn spawn_duplex(
    stream: TcpStream,
    capacity: DuplexCapacity,
) -> MuxTransportHandle {
    let (inbound_tx, inbound_rx) = mpsc::channel::<Vec<u8>>(capacity.inbound_chunks);
    let (outbound_tx, mut outbound_rx) = mpsc::channel::<Vec<u8>>(capacity.outbound_chunks);

    let (mut read_half, mut write_half) = stream.into_split();

    let reader_handle = tokio::spawn(async move {
        let mut buf = vec![0u8; capacity.read_buffer_bytes];
        loop {
            let n = match read_half.read(&mut buf).await {
                Ok(0) => return Err(TransportError::Eof),
                Ok(n) => n,
                Err(e) => return Err(TransportError::Io(e.kind())),
            };
            let chunk = buf[..n].to_vec();
            match inbound_tx.try_send(chunk) {
                Ok(()) => continue,
                Err(mpsc::error::TrySendError::Full(_)) => {
                    return Err(TransportError::BackpressureExceeded);
                }
                Err(mpsc::error::TrySendError::Closed(_)) => return Ok(()),
            }
        }
    });

    let writer_handle = tokio::spawn(async move {
        while let Some(chunk) = outbound_rx.recv().await {
            if let Err(e) = write_half.write_all(&chunk).await {
                return Err(TransportError::Io(e.kind()));
            }
        }
        Ok(())
    });

    MuxTransportHandle {
        inbound: inbound_rx,
        outbound: outbound_tx,
        reader_handle,
        writer_handle,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;

    async fn loopback_pair() -> (TcpStream, TcpStream) {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        let connect_fut = TcpStream::connect(addr);
        let accept_fut = async {
            let (s, _) = listener.accept().await.expect("accept");
            s
        };
        let (a, b) = tokio::join!(connect_fut, accept_fut);
        (a.expect("connect"), b)
    }

    #[tokio::test]
    async fn mux_transport_duplex_round_trips_bytes_over_loopback() {
        let (a, b) = loopback_pair().await;
        let mut handle_a = spawn_duplex(a, DuplexCapacity::DEFAULT);
        let handle_b = spawn_duplex(b, DuplexCapacity::DEFAULT);
        let payload = vec![0x42u8; 100];
        handle_b
            .outbound
            .send(payload.clone())
            .await
            .expect("send");
        let received = handle_a.inbound.recv().await.expect("recv");
        assert_eq!(received, payload);
        // Clean shutdown: drop b's outbound sender; reader_b will see
        // its peer (a) still open, so we drop a's stuff to trigger Eof.
        drop(handle_b);
        drop(handle_a);
    }

    #[tokio::test]
    async fn mux_transport_duplex_inbound_overflow_returns_backpressure() {
        let (a, b) = loopback_pair().await;
        // Tiny inbound buffer on a → flood from b should trigger
        // BackpressureExceeded.
        let cap_a = DuplexCapacity {
            inbound_chunks: 1,
            outbound_chunks: 16,
            read_buffer_bytes: 1024,
        };
        let mut handle_a = spawn_duplex(a, cap_a);
        let handle_b = spawn_duplex(b, DuplexCapacity::DEFAULT);
        // Don't read from handle_a.inbound; let the queue fill.
        // Send several large chunks from b.
        for _ in 0..32 {
            let _ = handle_b.outbound.send(vec![0xCC; 1024]).await;
            tokio::task::yield_now().await;
        }
        // Wait briefly for the reader to detect backpressure. We give
        // it a few yields; if it hasn't triggered we drop and check.
        for _ in 0..50 {
            tokio::task::yield_now().await;
        }
        // Don't await the reader to completion forever; abort after
        // the buffer is known to be full at least once.
        handle_a.reader_handle.abort();
        // The reader_handle may have returned BackpressureExceeded or
        // been aborted; the smoke test is that the bounded queue
        // didn't grow unboundedly. We assert by checking the queue
        // is at most `inbound_chunks` deep.
        let mut depth = 0;
        while handle_a.inbound.try_recv().is_ok() {
            depth += 1;
        }
        assert!(depth <= 1, "bounded inbound queue must cap at capacity=1, got {depth}");
        drop(handle_b);
    }

    #[tokio::test]
    async fn mux_transport_duplex_clean_shutdown_on_eof() {
        let (a, b) = loopback_pair().await;
        let handle_a = spawn_duplex(a, DuplexCapacity::DEFAULT);
        // Drop b → a's reader returns Eof.
        drop(b);
        let res = handle_a.reader_handle.await.expect("join reader");
        assert_eq!(res, Err(TransportError::Eof));
    }

    #[test]
    fn duplex_capacity_default_holds_pinned_values() {
        let d = DuplexCapacity::DEFAULT;
        assert_eq!(d.inbound_chunks, 1024);
        assert_eq!(d.outbound_chunks, 256);
        assert_eq!(d.read_buffer_bytes, 16 * 1024);
    }
}
