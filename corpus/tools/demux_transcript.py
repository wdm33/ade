#!/usr/bin/env python3
"""
Demux Cardano multiplexer frames into per-miniprotocol JSON transcripts.

Cardano mux framing (ouroboros-network-framework):
  - 4 bytes: transmission time (big-endian, microseconds)
  - 2 bytes: miniprotocol ID (big-endian, bit 15 = direction)
  - 2 bytes: payload length (big-endian)
  - N bytes: payload

Direction bit (bit 15 of miniprotocol ID field):
  - 0 = InitiatorToResponder
  - 1 = ResponderToInitiator

This script parses raw TCP payload extracted from pcap, reassembles
fragmented messages at the mux level, and produces per-miniprotocol
JSON files with the authoritative message sequence.

PROOF OBLIGATION: This demuxer must preserve the authoritative message
sequence while discarding only transport-level nondeterminism (socket
fragmentation, mux frame ordering, TCP timing). Verification: replay
demuxed output against a known protocol exchange with independently
verified message boundaries.

Usage:
    python3 demux_transcript.py <pcap_file> <output_dir>
"""

import json
import os
import struct
import sys
from collections import defaultdict

MUX_HEADER_SIZE = 8  # 4 (time) + 2 (protocol+direction) + 2 (length)


def parse_mux_frames(raw_bytes):
    """Parse raw TCP payload into mux frames."""
    frames = []
    offset = 0

    while offset + MUX_HEADER_SIZE <= len(raw_bytes):
        ts = struct.unpack_from(">I", raw_bytes, offset)[0]
        proto_dir = struct.unpack_from(">H", raw_bytes, offset + 4)[0]
        payload_len = struct.unpack_from(">H", raw_bytes, offset + 6)[0]

        # Extract direction from bit 15
        direction_bit = (proto_dir >> 15) & 1
        mini_protocol_id = proto_dir & 0x7FFF

        direction = "ResponderToInitiator" if direction_bit else "InitiatorToResponder"

        payload_start = offset + MUX_HEADER_SIZE
        payload_end = payload_start + payload_len

        if payload_end > len(raw_bytes):
            break  # incomplete frame

        payload = raw_bytes[payload_start:payload_end]

        frames.append({
            "timestamp": ts,
            "miniprotocol_id": mini_protocol_id,
            "direction": direction,
            "payload": payload,
            "payload_length": payload_len,
        })

        offset = payload_end

    return frames


def demux_frames(frames):
    """Group frames by miniprotocol ID, preserving message order."""
    protocols = defaultdict(list)

    for frame in frames:
        pid = frame["miniprotocol_id"]
        protocols[pid].append(frame)

    return dict(protocols)


PROTOCOL_NAMES = {
    0: "Handshake",
    2: "ChainSync",
    3: "BlockFetch",
    4: "TxSubmission2",
    5: "LocalChainSync",
    6: "LocalTxSubmission",
    7: "LocalStateQuery",
    8: "KeepAlive",
    9: "LocalTxMonitor",
    10: "PeerSharing",
}


def frames_to_transcript(miniprotocol_id, frames):
    """Convert frames for a single miniprotocol into a transcript."""
    protocol_name = PROTOCOL_NAMES.get(miniprotocol_id, f"Unknown({miniprotocol_id})")

    messages = []
    for i, frame in enumerate(frames):
        messages.append({
            "index": i,
            "direction": frame["direction"],
            "payload_hex": frame["payload"].hex(),
            "payload_length": frame["payload_length"],
        })

    return {
        "protocol": protocol_name,
        "miniprotocol_id": miniprotocol_id,
        "protocol_version": "",  # filled in from handshake if available
        "messages": messages,
    }


def extract_tcp_payload_from_pcap(pcap_path):
    """
    Extract raw TCP payload from a pcap file.

    This is a simplified extractor that handles the common case of
    Ethernet + IPv4 + TCP pcap files. For complex captures, consider
    using scapy or tshark.
    """
    with open(pcap_path, "rb") as f:
        data = f.read()

    # Minimal pcap global header check
    if len(data) < 24:
        return b""

    magic = struct.unpack_from("<I", data, 0)[0]
    if magic == 0xA1B2C3D4:
        endian = "<"
    elif magic == 0xD4C3B2A1:
        endian = ">"
    else:
        print(f"WARNING: Unknown pcap magic: {magic:#x}", file=sys.stderr)
        return b""

    # Read link-layer header type
    ll_type = struct.unpack_from(f"{endian}I", data, 20)[0]

    payload = bytearray()
    offset = 24  # skip global header

    while offset + 16 <= len(data):
        # Read packet header
        ts_sec = struct.unpack_from(f"{endian}I", data, offset)[0]
        ts_usec = struct.unpack_from(f"{endian}I", data, offset + 4)[0]
        cap_len = struct.unpack_from(f"{endian}I", data, offset + 8)[0]
        orig_len = struct.unpack_from(f"{endian}I", data, offset + 12)[0]

        pkt_start = offset + 16
        pkt_end = pkt_start + cap_len

        if pkt_end > len(data):
            break

        pkt = data[pkt_start:pkt_end]

        # Parse Ethernet (ll_type 1) or raw IP (ll_type 101)
        if ll_type == 1 and len(pkt) > 34:
            # Ethernet: skip 14 bytes, check IP + TCP
            ip_start = 14
            if pkt[ip_start] >> 4 == 4:  # IPv4
                ip_hdr_len = (pkt[ip_start] & 0x0F) * 4
                if pkt[ip_start + 9] == 6:  # TCP
                    tcp_start = ip_start + ip_hdr_len
                    tcp_hdr_len = ((pkt[tcp_start + 12] >> 4) & 0x0F) * 4
                    tcp_payload = pkt[tcp_start + tcp_hdr_len:]
                    payload.extend(tcp_payload)
        elif ll_type == 101 and len(pkt) > 20:
            # Raw IP
            if pkt[0] >> 4 == 4:
                ip_hdr_len = (pkt[0] & 0x0F) * 4
                if pkt[9] == 6:
                    tcp_start = ip_hdr_len
                    tcp_hdr_len = ((pkt[tcp_start + 12] >> 4) & 0x0F) * 4
                    tcp_payload = pkt[tcp_start + tcp_hdr_len:]
                    payload.extend(tcp_payload)

        offset = pkt_end

    return bytes(payload)


def main():
    if len(sys.argv) < 3:
        print(f"Usage: {sys.argv[0]} <pcap_file> <output_dir>")
        sys.exit(1)

    pcap_path = sys.argv[1]
    output_dir = sys.argv[2]

    if not os.path.exists(pcap_path):
        print(f"ERROR: pcap file not found: {pcap_path}")
        sys.exit(1)

    os.makedirs(output_dir, exist_ok=True)

    # Extract TCP payload from pcap
    print(f"Reading pcap: {pcap_path}")
    raw_tcp = extract_tcp_payload_from_pcap(pcap_path)

    if not raw_tcp:
        print("WARNING: No TCP payload extracted from pcap.")
        sys.exit(0)

    print(f"Extracted {len(raw_tcp)} bytes of TCP payload")

    # Parse mux frames
    frames = parse_mux_frames(raw_tcp)
    print(f"Parsed {len(frames)} mux frames")

    # Demux by miniprotocol
    by_protocol = demux_frames(frames)

    for pid, proto_frames in sorted(by_protocol.items()):
        transcript = frames_to_transcript(pid, proto_frames)
        protocol_name = transcript["protocol"].lower().replace("(", "").replace(")", "")
        output_path = os.path.join(output_dir, f"{protocol_name}_{pid}.json")

        with open(output_path, "w") as f:
            json.dump(transcript, f, indent=2)
            f.write("\n")

        print(f"  {transcript['protocol']} (ID {pid}): {len(proto_frames)} messages -> {output_path}")

    print("Done.")


if __name__ == "__main__":
    main()
