#!/usr/bin/env bash
set -euo pipefail

# Capture protocol transcripts between two cardano-node instances on loopback.
# Designed to run on the node host where cardano-node 10.6.2 is available.
#
# Workflow:
#   1. Start two cardano-node instances on loopback with known config
#   2. Capture traffic with tcpdump
#   3. Demux captured pcap into per-miniprotocol JSON transcripts
#   4. Write to corpus/reference/protocol_transcripts/
#
# Required environment variables (set on node host, never committed):
#   ADE_CARDANO_NODE_BIN       — Path to cardano-node binary
#   ADE_CARDANO_NODE_VERSION   — e.g., "10.6.2"
#   ADE_CARDANO_NODE_GIT_REV   — e.g., "0d697f14"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
OUTPUT_DIR="$REPO_ROOT/corpus/reference/protocol_transcripts"
DEMUX_SCRIPT="$SCRIPT_DIR/demux_transcript.py"

# --- Fail-fast on missing env vars ---
for var in ADE_CARDANO_NODE_BIN ADE_CARDANO_NODE_VERSION ADE_CARDANO_NODE_GIT_REV; do
    if [ -z "${!var:-}" ]; then
        echo "ERROR: Required environment variable $var is not set."
        echo "See corpus/tools/.env.example for required variables."
        exit 1
    fi
done

NODE_BIN="$ADE_CARDANO_NODE_BIN"
NODE_VERSION="$ADE_CARDANO_NODE_VERSION"
NODE_GIT_REV="$ADE_CARDANO_NODE_GIT_REV"

if ! command -v tcpdump &>/dev/null; then
    echo "ERROR: tcpdump not found. Install tcpdump for packet capture."
    exit 1
fi

if [ ! -f "$DEMUX_SCRIPT" ]; then
    echo "ERROR: demux_transcript.py not found at $DEMUX_SCRIPT"
    exit 1
fi

LOOPBACK_PORT=13717
CAPTURE_DURATION=30  # seconds
EXTRACTION_DATE=$(date +%Y-%m-%d)
PCAP_FILE=$(mktemp /tmp/ade_capture_XXXXXX.pcap)

echo "Protocol transcript capture"
echo "  Node: $NODE_BIN ($NODE_VERSION, git rev $NODE_GIT_REV)"
echo "  Loopback port: $LOOPBACK_PORT"
echo "  Capture duration: ${CAPTURE_DURATION}s"
echo ""

# Step 1: Start tcpdump capture on loopback
echo "Starting packet capture..."
tcpdump -i lo -w "$PCAP_FILE" "port $LOOPBACK_PORT" &
TCPDUMP_PID=$!

# Give tcpdump time to start
sleep 1

# Step 2: Start two cardano-node instances
# NOTE: This requires node configuration files and genesis data.
# The actual node startup is environment-specific.
echo "NOTE: Node startup requires environment-specific configuration."
echo "  Configure two nodes to connect on loopback port $LOOPBACK_PORT"
echo "  and allow them to complete handshake + initial ChainSync."
echo ""
echo "  After capture, run:"
echo "    python3 $DEMUX_SCRIPT $PCAP_FILE $OUTPUT_DIR"
echo ""

# Step 3: Wait for capture duration
echo "Waiting ${CAPTURE_DURATION}s for protocol exchange..."
sleep "$CAPTURE_DURATION"

# Step 4: Stop capture
kill "$TCPDUMP_PID" 2>/dev/null || true
wait "$TCPDUMP_PID" 2>/dev/null || true

echo "Capture saved to: $PCAP_FILE"

# Step 5: Demux into per-miniprotocol transcripts
if [ -s "$PCAP_FILE" ]; then
    echo "Running demux..."
    python3 "$DEMUX_SCRIPT" "$PCAP_FILE" "$OUTPUT_DIR"
    echo "Transcripts written to: $OUTPUT_DIR"
else
    echo "WARN: Empty capture file. Ensure nodes were running and communicating."
fi

# Cleanup
rm -f "$PCAP_FILE"
echo "Done."
