// RED — Imperative Shell.
//
// Tokio-based socket scaffold for the Ouroboros mux layer. The async
// surface lives here and only here within ade_network. BLUE submodules
// (codec, handshake, chain_sync, block_fetch, tx_submission, keep_alive,
// peer_sharing, n2c, mux::frame) MUST remain sync; DC-CORE-01 enforces
// this via ci/ci_check_no_async_in_blue.sh.
//
// No protocol logic here. Frame parsing is mux::frame (BLUE). Session
// composition (driving the mux + state machines together) lands in
// S-A9 under ade_network::session.

use std::io;
use std::net::SocketAddr;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

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
