// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN handshake driver (PHASE4-N-L S4).
//!
//! Drives `handshake::n2n_transition` to completion against an
//! opaque sync `Transport` trait. Production wires this trait to
//! `tokio::net::TcpStream` via `tokio::task::block_in_place`
//! (S7); tests use an in-memory loopback transport pair.
//!
//! The driver itself is pure: same `Transport` trace → same
//! `NegotiatedN2n` (DC-SESS-03 partial).

use crate::codec::handshake::{
    decode_handshake_message, encode_handshake_message, HandshakeMessage, VersionParams,
    VersionTable,
};
#[cfg(test)]
use crate::codec::version::N2NVersion;
use crate::handshake::agency::HandshakeAgency;
use crate::handshake::state::{
    HandshakeError, HandshakeState, N2nHandshakeOutput, VersionData,
};
use crate::handshake::transition::n2n_transition;
use crate::mux::frame::{
    decode_frame, encode_frame, MiniProtocolId, MuxError, MuxFrame, MuxHeader, MuxMode,
    HEADER_LEN,
};

/// Successful N2N handshake outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NegotiatedN2n {
    pub version: u16,
    pub params: VersionData,
}

/// Closed transport-error sum surfaced by `Transport` impls.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportError {
    Io,
    Eof,
    Mux(MuxError),
    Handshake(HandshakeError),
}

/// Sync transport seam used by the handshake driver. Tests use an
/// in-memory pair; production bridges to `tokio::net::TcpStream`
/// via `tokio::task::block_in_place` in S7.
pub trait Transport {
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), TransportError>;
    fn write_all(&mut self, bytes: &[u8]) -> Result<(), TransportError>;
}

/// Run the N2N handshake as the initiator (we dial). Proposes
/// `our_versions`; waits for the peer's `AcceptVersion` or
/// `Refuse`.
pub fn run_n2n_handshake_initiator(
    transport: &mut dyn Transport,
    our_versions: VersionTable,
) -> Result<NegotiatedN2n, TransportError> {
    // 1. Encode + send our ProposeVersions.
    let proposal = HandshakeMessage::ProposeVersions(our_versions);
    let payload = encode_handshake_message(&proposal);
    write_handshake_frame(transport, &payload, MuxMode::Initiator)?;

    // 2. Read one frame and decode.
    let frame = read_handshake_frame(transport)?;
    let msg = decode_handshake_message(&frame.payload).map_err(|_| {
        TransportError::Handshake(HandshakeError::MalformedMessage {
            reason: "peer reply failed handshake decode",
        })
    })?;

    // 3. Drive the state machine: we're at Proposed, peer has agency.
    let (_state, output) = n2n_transition(
        HandshakeState::Proposed,
        HandshakeAgency::ServerHasAgency,
        crate::handshake::version_table::N2N_SUPPORTED,
        msg,
    )
    .map_err(TransportError::Handshake)?;

    match output {
        N2nHandshakeOutput::Selected(version, params) => Ok(NegotiatedN2n {
            version: version.get(),
            params,
        }),
        N2nHandshakeOutput::Refused(_) | N2nHandshakeOutput::Done => Err(
            TransportError::Handshake(HandshakeError::MalformedMessage {
                reason: "handshake refused or terminated without selection",
            }),
        ),
        N2nHandshakeOutput::Reply(_) => Err(TransportError::Handshake(
            HandshakeError::MalformedMessage {
                reason: "initiator received Reply where Accept/Refuse expected",
            },
        )),
    }
}

/// Run the N2N handshake as the responder (peer dialed us).
/// Reads the peer's ProposeVersions; replies with AcceptVersion
/// (if a mutually-supported version exists) or Refuse.
pub fn run_n2n_handshake_responder(
    transport: &mut dyn Transport,
    our_supported: &[(u16, VersionData)],
) -> Result<NegotiatedN2n, TransportError> {
    use crate::codec::handshake::RefuseReason;
    use crate::codec::version::N2NVersion;

    let frame = read_handshake_frame(transport)?;
    let msg = decode_handshake_message(&frame.payload).map_err(|_| {
        TransportError::Handshake(HandshakeError::MalformedMessage {
            reason: "initiator proposal failed handshake decode",
        })
    })?;
    let transition_result = n2n_transition(
        HandshakeState::Idle,
        HandshakeAgency::ClientHasAgency,
        our_supported,
        msg,
    );
    let (_state, output) = match transition_result {
        Ok(r) => r,
        Err(HandshakeError::VersionMismatch {
            supported_set, ..
        }) => {
            // Write a Refuse on the wire before erroring so the
            // initiator can read its disposition deterministically.
            let refuse = HandshakeMessage::Refuse(RefuseReason::VersionMismatch(
                supported_set
                    .iter()
                    .map(|v| N2NVersion::new(*v))
                    .collect(),
            ));
            let bytes = encode_handshake_message(&refuse);
            write_handshake_frame(transport, &bytes, MuxMode::Responder)?;
            return Err(TransportError::Handshake(
                HandshakeError::VersionMismatch {
                    supported_set,
                    proposed_set: Vec::new(),
                },
            ));
        }
        Err(other) => return Err(TransportError::Handshake(other)),
    };
    match output {
        N2nHandshakeOutput::Selected(version, params) => {
            // Send AcceptVersion.
            let reply = HandshakeMessage::AcceptVersion(version, VersionParams(vec![0x01]));
            let bytes = encode_handshake_message(&reply);
            write_handshake_frame(transport, &bytes, MuxMode::Responder)?;
            Ok(NegotiatedN2n {
                version: version.get(),
                params,
            })
        }
        N2nHandshakeOutput::Reply(reply) => {
            let bytes = encode_handshake_message(&reply);
            write_handshake_frame(transport, &bytes, MuxMode::Responder)?;
            Err(TransportError::Handshake(HandshakeError::MalformedMessage {
                reason: "responder selection produced Reply (refuse path) instead of Selected",
            }))
        }
        N2nHandshakeOutput::Refused(_) | N2nHandshakeOutput::Done => Err(
            TransportError::Handshake(HandshakeError::MalformedMessage {
                reason: "handshake refused on responder side",
            }),
        ),
    }
}

fn write_handshake_frame(
    transport: &mut dyn Transport,
    payload: &[u8],
    mode: MuxMode,
) -> Result<(), TransportError> {
    let frame = MuxFrame {
        header: MuxHeader {
            timestamp: 0,
            mode,
            mini_protocol_id: MiniProtocolId::new(0).expect("0"),
            length: payload.len() as u16,
        },
        payload: payload.to_vec(),
    };
    let bytes = encode_frame(&frame).map_err(TransportError::Mux)?;
    transport.write_all(&bytes)
}

fn read_handshake_frame(transport: &mut dyn Transport) -> Result<MuxFrame, TransportError> {
    let mut header = [0u8; HEADER_LEN];
    transport.read_exact(&mut header)?;
    // Peek the declared payload length without consuming.
    let length = u16::from_be_bytes([header[6], header[7]]) as usize;
    let mut all = Vec::with_capacity(HEADER_LEN + length);
    all.extend_from_slice(&header);
    if length > 0 {
        let pre = all.len();
        all.resize(pre + length, 0);
        transport.read_exact(&mut all[pre..])?;
    }
    let (frame, _rest) = decode_frame(&all).map_err(TransportError::Mux)?;
    Ok(frame)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use crate::codec::handshake::VersionTable;
    use crate::handshake::state::{PeerSharingFlag, VersionData};
    use crate::handshake::version_table::{MAINNET_NETWORK_MAGIC, N2N_SUPPORTED};
    use std::sync::{Arc, Mutex};

    /// In-memory transport pair for tests.
    struct Pipe {
        inbox: Arc<Mutex<Vec<u8>>>,
        outbox: Arc<Mutex<Vec<u8>>>,
    }

    impl Transport for Pipe {
        fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), TransportError> {
            // Spin until enough bytes are available; in real tests this
            // blocks the test thread which is fine since both ends are
            // driven sequentially.
            loop {
                let mut inbox = self.inbox.lock().expect("lock");
                if inbox.len() >= buf.len() {
                    let drained: Vec<u8> = inbox.drain(..buf.len()).collect();
                    buf.copy_from_slice(&drained);
                    return Ok(());
                }
                drop(inbox);
                std::thread::yield_now();
            }
        }
        fn write_all(&mut self, bytes: &[u8]) -> Result<(), TransportError> {
            self.outbox.lock().expect("lock").extend_from_slice(bytes);
            Ok(())
        }
    }

    fn pipe_pair() -> (Pipe, Pipe) {
        let a_to_b = Arc::new(Mutex::new(Vec::new()));
        let b_to_a = Arc::new(Mutex::new(Vec::new()));
        (
            Pipe {
                inbox: b_to_a.clone(),
                outbox: a_to_b.clone(),
            },
            Pipe {
                inbox: a_to_b,
                outbox: b_to_a,
            },
        )
    }

    fn versions_14_to_16() -> VersionTable {
        VersionTable(vec![
            (
                N2NVersion::new(14),
                VersionParams(vec![0x01]),
            ),
            (
                N2NVersion::new(15),
                VersionParams(vec![0x01]),
            ),
            (
                N2NVersion::new(16),
                VersionParams(vec![0x01]),
            ),
        ])
    }

    #[test]
    fn handshake_initiator_accepts_when_responder_supports_proposed_version() {
        let (mut a, mut b) = pipe_pair();
        let join = std::thread::spawn(move || {
            // Responder side using the project's N2N_SUPPORTED table.
            run_n2n_handshake_responder(&mut b, N2N_SUPPORTED)
        });
        let outcome = run_n2n_handshake_initiator(&mut a, versions_14_to_16())
            .expect("initiator handshake");
        let resp_outcome = join.join().expect("join").expect("responder handshake");
        // Both sides agree on the highest mutually-supported version.
        assert_eq!(outcome.version, resp_outcome.version);
        assert!(outcome.version >= 14 && outcome.version <= 16);
        assert_eq!(outcome.params.network_magic, MAINNET_NETWORK_MAGIC);
        assert_eq!(outcome.params.peer_sharing, PeerSharingFlag::NoPeerSharing);
        assert!(!outcome.params.peras_support);
        // Touch VersionData to silence unused-import lint warnings.
        let _ = VersionData {
            network_magic: 0,
            initiator_only_diffusion: false,
            peer_sharing: PeerSharingFlag::NoPeerSharing,
            query: false,
            peras_support: false,
        };
    }

    #[test]
    fn handshake_initiator_rejects_on_no_overlap() {
        // Initiator proposes only v3 (not in our supported table).
        let (mut a, mut b) = pipe_pair();
        let join = std::thread::spawn(move || {
            run_n2n_handshake_responder(&mut b, N2N_SUPPORTED)
        });
        let disjoint = VersionTable(vec![(
            N2NVersion::new(3),
            VersionParams(vec![0x01]),
        )]);
        let err = run_n2n_handshake_initiator(&mut a, disjoint)
            .expect_err("must reject on no overlap");
        // The responder also errs symmetrically; join it to clean up.
        let _ = join.join().expect("join");
        assert!(
            matches!(
                err,
                TransportError::Handshake(HandshakeError::VersionMismatch { .. })
                    | TransportError::Handshake(HandshakeError::MalformedMessage { .. })
            ),
            "got {err:?}"
        );
    }
}
