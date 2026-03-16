use serde::{Deserialize, Serialize};
use std::fmt;

use super::transcript::{Direction, Transcript};
use super::HarnessError;

/// Cardano mini-protocol identifiers.
///
/// Numeric IDs sourced from ouroboros-network:
/// - N2N: Handshake (0), ChainSync (2), BlockFetch (3),
///   TxSubmission2 (4), KeepAlive (8), PeerSharing (10)
/// - N2C: Handshake (0), LocalChainSync (5), LocalTxSubmission (6),
///   LocalStateQuery (7), LocalTxMonitor (9)
///
/// Reference: ouroboros-network/ouroboros-network-protocols/src/Ouroboros/Network/Protocol/
/// Overlapping IDs (Handshake = 0 for both N2N and N2C) are disambiguated
/// by connection type prefix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum MiniProtocolId {
    // Node-to-Node (N2N)
    /// N2N Handshake — miniprotocol ID 0
    /// Ref: ouroboros-network-protocols Ouroboros.Network.Protocol.Handshake.Type
    N2NHandshake,
    /// N2N ChainSync — miniprotocol ID 2
    /// Ref: ouroboros-network-protocols Ouroboros.Network.Protocol.ChainSync.Type
    N2NChainSync,
    /// N2N BlockFetch — miniprotocol ID 3
    /// Ref: ouroboros-network-protocols Ouroboros.Network.Protocol.BlockFetch.Type
    N2NBlockFetch,
    /// N2N TxSubmission2 — miniprotocol ID 4
    /// Ref: ouroboros-network-protocols Ouroboros.Network.Protocol.TxSubmission2.Type
    N2NTxSubmission2,
    /// N2N KeepAlive — miniprotocol ID 8
    /// Ref: ouroboros-network-protocols Ouroboros.Network.Protocol.KeepAlive.Type
    N2NKeepAlive,
    /// N2N PeerSharing — miniprotocol ID 10
    /// Ref: ouroboros-network-protocols Ouroboros.Network.Protocol.PeerSharing.Type
    N2NPeerSharing,

    // Node-to-Client (N2C)
    /// N2C Handshake — miniprotocol ID 0
    /// Ref: ouroboros-network-protocols Ouroboros.Network.Protocol.Handshake.Type
    N2CHandshake,
    /// N2C LocalChainSync — miniprotocol ID 5
    /// Ref: ouroboros-network-protocols Ouroboros.Network.Protocol.LocalChainSync
    N2CLocalChainSync,
    /// N2C LocalTxSubmission — miniprotocol ID 6
    /// Ref: ouroboros-network-protocols Ouroboros.Network.Protocol.LocalTxSubmission.Type
    N2CLocalTxSubmission,
    /// N2C LocalStateQuery — miniprotocol ID 7
    /// Ref: ouroboros-network-protocols Ouroboros.Network.Protocol.LocalStateQuery.Type
    N2CLocalStateQuery,
    /// N2C LocalTxMonitor — miniprotocol ID 9
    /// Ref: ouroboros-network-protocols Ouroboros.Network.Protocol.LocalTxMonitor.Type
    N2CLocalTxMonitor,
}

impl MiniProtocolId {
    /// Returns the numeric miniprotocol ID as used in the mux framing header.
    pub fn numeric_id(&self) -> u16 {
        match self {
            MiniProtocolId::N2NHandshake | MiniProtocolId::N2CHandshake => 0,
            MiniProtocolId::N2NChainSync => 2,
            MiniProtocolId::N2NBlockFetch => 3,
            MiniProtocolId::N2NTxSubmission2 => 4,
            MiniProtocolId::N2CLocalChainSync => 5,
            MiniProtocolId::N2CLocalTxSubmission => 6,
            MiniProtocolId::N2CLocalStateQuery => 7,
            MiniProtocolId::N2NKeepAlive => 8,
            MiniProtocolId::N2CLocalTxMonitor => 9,
            MiniProtocolId::N2NPeerSharing => 10,
        }
    }

    /// Returns whether this is a Node-to-Node protocol.
    pub fn is_n2n(&self) -> bool {
        matches!(
            self,
            MiniProtocolId::N2NHandshake
                | MiniProtocolId::N2NChainSync
                | MiniProtocolId::N2NBlockFetch
                | MiniProtocolId::N2NTxSubmission2
                | MiniProtocolId::N2NKeepAlive
                | MiniProtocolId::N2NPeerSharing
        )
    }
}

impl fmt::Display for MiniProtocolId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MiniProtocolId::N2NHandshake => write!(f, "N2N-Handshake(0)"),
            MiniProtocolId::N2NChainSync => write!(f, "N2N-ChainSync(2)"),
            MiniProtocolId::N2NBlockFetch => write!(f, "N2N-BlockFetch(3)"),
            MiniProtocolId::N2NTxSubmission2 => write!(f, "N2N-TxSubmission2(4)"),
            MiniProtocolId::N2NKeepAlive => write!(f, "N2N-KeepAlive(8)"),
            MiniProtocolId::N2NPeerSharing => write!(f, "N2N-PeerSharing(10)"),
            MiniProtocolId::N2CHandshake => write!(f, "N2C-Handshake(0)"),
            MiniProtocolId::N2CLocalChainSync => write!(f, "N2C-LocalChainSync(5)"),
            MiniProtocolId::N2CLocalTxSubmission => write!(f, "N2C-LocalTxSubmission(6)"),
            MiniProtocolId::N2CLocalStateQuery => write!(f, "N2C-LocalStateQuery(7)"),
            MiniProtocolId::N2CLocalTxMonitor => write!(f, "N2C-LocalTxMonitor(9)"),
        }
    }
}

/// Trait for a protocol state machine that processes messages.
///
/// Implementations will track protocol state and validate transitions.
/// The harness uses this to replay transcripts and compare behavior.
pub trait ProtocolStateMachine {
    /// Process an inbound message and return the outbound response (if any).
    fn receive_message(
        &mut self,
        direction: Direction,
        payload: &[u8],
    ) -> Result<Option<Vec<u8>>, HarnessError>;

    /// Returns a label for the current protocol state.
    fn current_state_label(&self) -> &str;

    /// Reset the state machine to its initial state.
    fn reset(&mut self);
}

/// Stub state machine that returns `NotYetImplemented` for all operations.
pub struct StubProtocolStateMachine;

impl ProtocolStateMachine for StubProtocolStateMachine {
    fn receive_message(
        &mut self,
        _direction: Direction,
        _payload: &[u8],
    ) -> Result<Option<Vec<u8>>, HarnessError> {
        Err(HarnessError::NotYetImplemented(
            "protocol state machine not yet implemented".to_string(),
        ))
    }

    fn current_state_label(&self) -> &str {
        "stub-initial"
    }

    fn reset(&mut self) {}
}

/// A point of divergence in a protocol transcript replay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolDivergence {
    /// Message index where divergence was detected.
    pub message_index: usize,
    /// Direction of the divergent message.
    pub direction: Direction,
    /// Expected payload from the reference transcript (hex).
    pub expected_payload_hex: String,
    /// Actual payload or error from the state machine.
    pub actual: String,
}

/// Report from replaying a protocol transcript.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolDiffReport {
    /// The first divergence found, if any.
    pub first_divergence: Option<ProtocolDivergence>,
}

impl ProtocolDiffReport {
    /// Returns true if the replay matched the transcript.
    pub fn is_match(&self) -> bool {
        self.first_divergence.is_none()
    }
}

/// Replay a transcript through a state machine, comparing outbound messages
/// against the transcript's expected messages.
///
/// For each `ResponderToInitiator` message: feeds preceding `InitiatorToResponder`
/// messages, then compares the state machine's response against the expected
/// outbound payload. Reports the first divergence found.
pub fn replay_transcript(
    state_machine: &mut dyn ProtocolStateMachine,
    transcript: &Transcript,
) -> Result<ProtocolDiffReport, HarnessError> {
    for msg in &transcript.messages {
        let payload = hex_to_bytes(&msg.payload_hex)?;

        match msg.direction {
            Direction::InitiatorToResponder => {
                // Feed inbound message to state machine
                let response = state_machine.receive_message(msg.direction, &payload)?;

                // If state machine produced an unexpected outbound, that's a divergence
                // (we only expect outbound for ResponderToInitiator messages)
                if response.is_some() {
                    // State machine responded when it shouldn't have — this is
                    // valid behavior depending on protocol, so we don't flag it here
                }
            }
            Direction::ResponderToInitiator => {
                // Compare state machine's response to expected
                let response = state_machine.receive_message(msg.direction, &payload)?;
                match response {
                    Some(actual_payload) => {
                        let actual_hex = bytes_to_hex(&actual_payload);
                        if actual_hex != msg.payload_hex {
                            return Ok(ProtocolDiffReport {
                                first_divergence: Some(ProtocolDivergence {
                                    message_index: msg.index,
                                    direction: msg.direction,
                                    expected_payload_hex: msg.payload_hex.clone(),
                                    actual: actual_hex,
                                }),
                            });
                        }
                    }
                    None => {
                        // No response when one was expected — record divergence
                        return Ok(ProtocolDiffReport {
                            first_divergence: Some(ProtocolDivergence {
                                message_index: msg.index,
                                direction: msg.direction,
                                expected_payload_hex: msg.payload_hex.clone(),
                                actual: "<no response>".to_string(),
                            }),
                        });
                    }
                }
            }
        }
    }

    Ok(ProtocolDiffReport {
        first_divergence: None,
    })
}

fn hex_to_bytes(hex: &str) -> Result<Vec<u8>, HarnessError> {
    if hex.len() % 2 != 0 {
        return Err(HarnessError::ParseError(
            "hex string must have even length".to_string(),
        ));
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&hex[i..i + 2], 16)
                .map_err(|e| HarnessError::ParseError(format!("invalid hex: {e}")))
        })
        .collect()
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mini_protocol_numeric_ids() {
        assert_eq!(MiniProtocolId::N2NHandshake.numeric_id(), 0);
        assert_eq!(MiniProtocolId::N2NChainSync.numeric_id(), 2);
        assert_eq!(MiniProtocolId::N2NBlockFetch.numeric_id(), 3);
        assert_eq!(MiniProtocolId::N2NTxSubmission2.numeric_id(), 4);
        assert_eq!(MiniProtocolId::N2CLocalChainSync.numeric_id(), 5);
        assert_eq!(MiniProtocolId::N2CLocalTxSubmission.numeric_id(), 6);
        assert_eq!(MiniProtocolId::N2CLocalStateQuery.numeric_id(), 7);
        assert_eq!(MiniProtocolId::N2NKeepAlive.numeric_id(), 8);
        assert_eq!(MiniProtocolId::N2CLocalTxMonitor.numeric_id(), 9);
        assert_eq!(MiniProtocolId::N2NPeerSharing.numeric_id(), 10);
    }

    #[test]
    fn mini_protocol_handshake_overlap() {
        // Both N2N and N2C handshake use ID 0 — disambiguated by type
        assert_eq!(MiniProtocolId::N2NHandshake.numeric_id(), 0);
        assert_eq!(MiniProtocolId::N2CHandshake.numeric_id(), 0);
        assert_ne!(MiniProtocolId::N2NHandshake, MiniProtocolId::N2CHandshake);
    }

    #[test]
    fn mini_protocol_n2n_classification() {
        assert!(MiniProtocolId::N2NHandshake.is_n2n());
        assert!(MiniProtocolId::N2NChainSync.is_n2n());
        assert!(MiniProtocolId::N2NBlockFetch.is_n2n());
        assert!(!MiniProtocolId::N2CHandshake.is_n2n());
        assert!(!MiniProtocolId::N2CLocalStateQuery.is_n2n());
    }

    #[test]
    fn mini_protocol_display() {
        assert_eq!(
            format!("{}", MiniProtocolId::N2NChainSync),
            "N2N-ChainSync(2)"
        );
        assert_eq!(
            format!("{}", MiniProtocolId::N2CLocalStateQuery),
            "N2C-LocalStateQuery(7)"
        );
    }

    #[test]
    fn stub_state_machine_returns_not_yet_implemented() {
        let mut stub = StubProtocolStateMachine;
        let result = stub.receive_message(Direction::InitiatorToResponder, &[]);
        assert!(matches!(result, Err(HarnessError::NotYetImplemented(_))));
    }

    #[test]
    fn stub_state_machine_initial_state() {
        let stub = StubProtocolStateMachine;
        assert_eq!(stub.current_state_label(), "stub-initial");
    }

    #[test]
    fn hex_conversion_roundtrip() {
        let bytes = vec![0x82, 0x00, 0x01, 0xff];
        let hex = bytes_to_hex(&bytes);
        assert_eq!(hex, "820001ff");
        let decoded = hex_to_bytes(&hex).unwrap();
        assert_eq!(decoded, bytes);
    }

    #[test]
    fn hex_to_bytes_odd_length_error() {
        let result = hex_to_bytes("abc");
        assert!(matches!(result, Err(HarnessError::ParseError(_))));
    }

    #[test]
    fn protocol_diff_report_roundtrip_json() {
        let report = ProtocolDiffReport {
            first_divergence: Some(ProtocolDivergence {
                message_index: 3,
                direction: Direction::ResponderToInitiator,
                expected_payload_hex: "820001".to_string(),
                actual: "820002".to_string(),
            }),
        };
        let json = serde_json::to_string(&report).unwrap();
        let parsed: ProtocolDiffReport = serde_json::from_str(&json).unwrap();
        assert_eq!(report, parsed);
    }

    #[test]
    fn mini_protocol_id_roundtrip_json() {
        for id in [
            MiniProtocolId::N2NHandshake,
            MiniProtocolId::N2NChainSync,
            MiniProtocolId::N2NBlockFetch,
            MiniProtocolId::N2NTxSubmission2,
            MiniProtocolId::N2NKeepAlive,
            MiniProtocolId::N2NPeerSharing,
            MiniProtocolId::N2CHandshake,
            MiniProtocolId::N2CLocalChainSync,
            MiniProtocolId::N2CLocalTxSubmission,
            MiniProtocolId::N2CLocalStateQuery,
            MiniProtocolId::N2CLocalTxMonitor,
        ] {
            let json = serde_json::to_string(&id).unwrap();
            let parsed: MiniProtocolId = serde_json::from_str(&json).unwrap();
            assert_eq!(id, parsed);
        }
    }
}
