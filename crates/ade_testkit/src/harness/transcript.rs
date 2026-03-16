use serde::{Deserialize, Serialize};

use super::HarnessError;

/// A single message in a protocol transcript.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TranscriptMessage {
    /// Zero-based message index within the transcript.
    pub index: usize,
    /// Message direction.
    pub direction: Direction,
    /// Message payload as hex-encoded bytes.
    pub payload_hex: String,
    /// Payload length in bytes.
    pub payload_length: usize,
}

/// Direction of a protocol message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Direction {
    InitiatorToResponder,
    ResponderToInitiator,
}

/// A recorded protocol transcript from a mini-protocol exchange.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Transcript {
    /// Mini-protocol identifier.
    pub protocol: String,
    /// Protocol version negotiated.
    pub protocol_version: String,
    /// Ordered sequence of messages.
    pub messages: Vec<TranscriptMessage>,
}

/// Parse a JSON string into a `Transcript`.
pub fn parse_transcript(json_content: &str) -> Result<Transcript, HarnessError> {
    serde_json::from_str(json_content)
        .map_err(|e| HarnessError::ParseError(format!("transcript JSON parse error: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_transcript_json() -> &'static str {
        r#"{
            "protocol": "ChainSync",
            "protocol_version": "13",
            "messages": [
                {
                    "index": 0,
                    "direction": "InitiatorToResponder",
                    "payload_hex": "820001",
                    "payload_length": 3
                },
                {
                    "index": 1,
                    "direction": "ResponderToInitiator",
                    "payload_hex": "8200820000",
                    "payload_length": 5
                }
            ]
        }"#
    }

    #[test]
    fn parse_valid_transcript() {
        let transcript = parse_transcript(sample_transcript_json()).unwrap();
        assert_eq!(transcript.protocol, "ChainSync");
        assert_eq!(transcript.protocol_version, "13");
        assert_eq!(transcript.messages.len(), 2);
    }

    #[test]
    fn parse_transcript_message_fields() {
        let transcript = parse_transcript(sample_transcript_json()).unwrap();
        let msg = &transcript.messages[0];
        assert_eq!(msg.index, 0);
        assert_eq!(msg.direction, Direction::InitiatorToResponder);
        assert_eq!(msg.payload_hex, "820001");
        assert_eq!(msg.payload_length, 3);
    }

    #[test]
    fn parse_transcript_directions() {
        let transcript = parse_transcript(sample_transcript_json()).unwrap();
        assert_eq!(
            transcript.messages[0].direction,
            Direction::InitiatorToResponder
        );
        assert_eq!(
            transcript.messages[1].direction,
            Direction::ResponderToInitiator
        );
    }

    #[test]
    fn parse_invalid_json_returns_error() {
        let result = parse_transcript("not json");
        assert!(result.is_err());
        match result.unwrap_err() {
            HarnessError::ParseError(msg) => assert!(msg.contains("JSON")),
            other => panic!("expected ParseError, got {other:?}"),
        }
    }

    #[test]
    fn transcript_roundtrip_json() {
        let transcript = parse_transcript(sample_transcript_json()).unwrap();
        let serialized = serde_json::to_string(&transcript).unwrap();
        let reparsed: Transcript = serde_json::from_str(&serialized).unwrap();
        assert_eq!(transcript, reparsed);
    }
}
