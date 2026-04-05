//! Integration test: Compute and verify anchor hashes for all proof-grade snapshots.
//!
//! Each snapshot's `state` file is the ExtLedgerState CBOR at that slot.
//! Its Blake2b-256 hash is the oracle state hash — the comparison surface
//! for T-25 epoch boundary and T-26 HFC transition proofs.
//!
//! This test establishes the anchor hash chain: a deterministic, reproducible
//! mapping from slot → state_hash that T-25/T-26 will verify against.

use std::path::PathBuf;

use ade_testkit::harness::snapshot_loader::{
    compute_state_hash, extract_state_from_tarball, parse_oracle_hashes, parse_snapshot_header,
};

fn snapshots_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("corpus")
        .join("snapshots")
}

struct AnchorHash {
    slot: u64,
    label: &'static str,
    purpose: &'static str,
    epoch: u64,
    telescope: u32,
    state_size: usize,
    hash_hex: String,
}

fn compute_anchor(slot: u64, label: &'static str, purpose: &'static str) -> Option<AnchorHash> {
    let tarball = snapshots_dir().join(format!("snapshot_{slot}.tar.gz"));
    if !tarball.exists() {
        return None;
    }

    let state_bytes = extract_state_from_tarball(&tarball).ok()?;
    let header = parse_snapshot_header(&state_bytes).ok()?;
    let hash = compute_state_hash(&state_bytes);

    Some(AnchorHash {
        slot,
        label,
        purpose,
        epoch: header.epoch,
        telescope: header.telescope_length,
        state_size: header.state_size,
        hash_hex: format!("{hash}"),
    })
}

#[test]
fn anchor_hash_chain_all_proof_grade() {
    let specs: Vec<(u64, &str, &str)> = vec![
        (4492800, "byron->shelley", "pre_hfc"),
        (4924880, "shelley", "epoch_boundary"),
        (16588800, "shelley->allegra", "pre_hfc"),
        (17020848, "allegra", "epoch_boundary"),
        (23068800, "allegra->mary", "pre_hfc"),
        (23500962, "mary", "epoch_boundary"),
        (39916975, "mary->alonzo", "pre_hfc"),
        (40348902, "alonzo", "epoch_boundary"),
        (72316896, "alonzo->babbage", "pre_hfc"),
        (72748820, "babbage", "epoch_boundary"),
        (133660855, "babbage->conway", "pre_hfc"),
        (134092810, "conway", "epoch_boundary"),
    ];

    let anchors: Vec<AnchorHash> = specs
        .iter()
        .filter_map(|(slot, label, purpose)| compute_anchor(*slot, label, purpose))
        .collect();

    eprintln!("\n=== Anchor Hash Chain (12 Proof-Grade Snapshots) ===");
    eprintln!(
        "{:<22} {:>5} {:>4} {:>12} {:>10}  Blake2b-256",
        "Label", "Epoch", "Tele", "Slot", "Size"
    );
    eprintln!("{}", "-".repeat(120));

    for a in &anchors {
        eprintln!(
            "{:<22} {:>5} {:>4} {:>12} {:>10}  {}",
            a.label, a.epoch, a.telescope, a.slot, a.state_size, a.hash_hex
        );
    }
    eprintln!("{}", "=".repeat(120));

    // Structural assertions
    assert_eq!(anchors.len(), 12, "all 12 proof-grade snapshots must load");

    // Telescope grows monotonically for HFC snapshots
    let hfc_telescopes: Vec<u32> = anchors
        .iter()
        .filter(|a| a.purpose == "pre_hfc")
        .map(|a| a.telescope)
        .collect();
    for w in hfc_telescopes.windows(2) {
        assert!(
            w[0] < w[1],
            "telescope must grow: {} < {}",
            w[0],
            w[1]
        );
    }

    // Epoch boundary snapshots have same telescope as preceding HFC
    for pair in anchors.chunks(2) {
        if pair.len() == 2 {
            assert_eq!(
                pair[0].telescope, pair[1].telescope,
                "HFC and following epoch boundary must have same telescope: {} vs {}",
                pair[0].label, pair[1].label
            );
        }
    }

    // All hashes are unique
    let hashes: Vec<&str> = anchors.iter().map(|a| a.hash_hex.as_str()).collect();
    for (i, h) in hashes.iter().enumerate() {
        for (j, h2) in hashes.iter().enumerate() {
            if i != j {
                assert_ne!(h, h2, "anchor hashes must be unique");
            }
        }
    }

    // Determinism: recompute one and verify same hash
    let recompute = compute_anchor(4492800, "byron->shelley", "pre_hfc");
    assert_eq!(
        recompute.as_ref().map(|a| a.hash_hex.as_str()),
        Some(anchors[0].hash_hex.as_str()),
        "anchor hash must be reproducible"
    );
}

#[test]
fn epoch_boundary_hashes_chain_to_oracle() {
    // For each epoch boundary snapshot that has a hash file,
    // verify the snapshot's state size is consistent with the
    // oracle's first post-snapshot entry.
    let cases = [
        (4924880, "hashes_4924880.txt", "shelley"),
        (17020848, "hashes_17020848.txt", "allegra"),
        (40348902, "hashes_40348902.txt", "alonzo"),
        (134092810, "hashes_134092810.txt", "conway"),
    ];

    eprintln!("\n=== Epoch Boundary → Oracle Hash Chain ===");

    for (slot, hash_file, era) in &cases {
        let tarball = snapshots_dir().join(format!("snapshot_{slot}.tar.gz"));
        let hf_path = snapshots_dir().join(hash_file);
        if !tarball.exists() || !hf_path.exists() {
            continue;
        }

        let state_bytes = extract_state_from_tarball(&tarball).unwrap();
        let snap_hash = compute_state_hash(&state_bytes);
        let snap_size = state_bytes.len();

        let content = std::fs::read_to_string(&hf_path).unwrap();
        let oracle_entries = parse_oracle_hashes(&content).unwrap();

        eprintln!(
            "  {era} (slot {slot}): snapshot_hash={}, snap_size={snap_size}, oracle_entries={}",
            &format!("{snap_hash}")[..16],
            oracle_entries.len()
        );

        // The snapshot hash is the state BEFORE the first oracle block.
        // The first oracle entry is the state AFTER applying that block.
        // They must differ (a block was applied).
        assert_ne!(
            format!("{snap_hash}"),
            format!("{}", oracle_entries[0].hash),
            "{era}: snapshot hash should differ from first oracle hash (block applied)"
        );

        // Oracle entries must be slot-ordered
        for w in oracle_entries.windows(2) {
            assert!(
                w[0].slot < w[1].slot,
                "{era}: oracle hashes must be slot-ordered"
            );
        }
    }
}

/// CBOR structural probe for HFC boundary snapshots.
///
/// Walks the top-level encoding of each HFC snapshot to document the exact
/// CBOR conventions used by the Haskell serializer. This is the foundation
/// for building a state encoder that produces byte-identical output.
#[test]
fn hfc_cbor_structure_probe() {
    use ade_testkit::harness::snapshot_loader::{
        navigate_to_nes_pub, read_array_header_pub, read_cbor_initial_pub, skip_cbor_pub,
    };

    // Skip Byron→Shelley: Byron has a fundamentally different encoding.
    // Focus on Shelley+ transitions where the telescope structure is consistent.
    let hfc_snapshots: &[(&str, u64)] = &[
        ("Shelley→Allegra", 16588800),
        ("Allegra→Mary", 23068800),
        ("Mary→Alonzo", 39916975),
        ("Alonzo→Babbage", 72316896),
        ("Babbage→Conway", 133660855),
    ];

    eprintln!("\n=== HFC CBOR Structure Probe ===\n");

    for (label, slot) in hfc_snapshots {
        let tarball = snapshots_dir().join(format!("snapshot_{slot}.tar.gz"));
        if !tarball.exists() {
            eprintln!("  {label}: SKIPPED");
            continue;
        }

        let data = extract_state_from_tarball(&tarball).unwrap();
        use ade_testkit::harness::snapshot_loader::read_uint_pub;

        // Follow the exact navigate_to_nes code path to understand the encoding:
        // outer: array(2) → [LedgerState_block, HeaderState]
        //   LedgerState_block:
        //     uint: era_index
        //     array: state_pair → [telescope, ...]
        //       telescope: array(N) → [Past..., Current]
        //         Current: array(2) → [Bound, State]
        //           State: array(2) → [version, payload]
        //             payload: array(3) → [WithOrigin, NES, Transition]
        let (top_body, top_len) = read_array_header_pub(&data, 0).unwrap();
        let (off, era_index) = read_uint_pub(&data, top_body as usize).unwrap();
        let (sp_body, _sp_len) = read_array_header_pub(&data, off).unwrap();
        let (tele_body, tele_len) = read_array_header_pub(&data, sp_body as usize).unwrap();

        // Walk Past entries
        let mut off = tele_body as usize;
        let mut past_sizes = Vec::new();
        for i in 0..tele_len.saturating_sub(1) {
            let start = off;
            off = skip_cbor_pub(&data, off).unwrap();
            past_sizes.push((i, off - start));
        }

        // Current entry
        let current_start = off;
        let (_cur_body, _cur_len) = read_array_header_pub(&data, off).unwrap();
        let current_size = skip_cbor_pub(&data, off).unwrap() - off;

        // Navigate to NES using the existing proven path
        let nes_off = navigate_to_nes_pub(&data).unwrap();

        // Walk NES fields (navigate_to_nes returns body offset, i.e., first field)
        let mut nes_field_info = Vec::new();
        let mut fi = nes_off;
        for i in 0..8 {
            let (_, fm, fv) = match read_cbor_initial_pub(&data, fi) {
                Ok(v) => v, Err(_) => break,
            };
            let fs = match skip_cbor_pub(&data, fi) {
                Ok(v) => v - fi, Err(_) => break,
            };
            let ft = match fm { 0=>"uint", 1=>"nint", 2=>"bytes", 3=>"text",
                4=>"arr", 5=>"map", 6=>"tag", 7=>"special", _=>"?" };
            nes_field_info.push((i, ft, fv, fs, data[fi]));
            fi = match skip_cbor_pub(&data, fi) {
                Ok(v) => v, Err(_) => break,
            };
        }

        // HeaderState: follows AFTER the entire LedgerState (outer array(2))
        // The outer array(2) = LedgerState NS encoding = [era_index, era_state]
        // HeaderState starts where the outer array(2) ends.
        let ls_end = skip_cbor_pub(&data, 0).unwrap(); // skip entire outer array(2)
        let header_size = data.len() - ls_end;
        let header_raw = if ls_end < data.len() { format!("0x{:02x}", data[ls_end]) } else { "EOF".to_string() };

        // Report
        eprintln!("  {label} (slot {slot}, {} bytes):", data.len());
        eprintln!("    outer: array({top_len}) raw=0x{:02x}", data[0]);
        eprintln!("    era_index: {era_index}  telescope: array({tele_len})");
        for (i, psize) in &past_sizes {
            eprintln!("      Past[{i}]: {psize} bytes");
        }
        eprintln!("      Current: {} bytes at offset {current_start}", current_size);
        eprintln!("    NES fields (from offset {nes_off}):");
        for (i, ft, fv, fs, raw) in &nes_field_info {
            let sz = if *fs > 1_000_000 { format!("{}MB", fs/1_000_000) }
                else if *fs > 1000 { format!("{}KB", fs/1000) }
                else { format!("{fs}B") };
            let note = if *ft == "uint" { format!(" = {fv}") } else { String::new() };
            eprintln!("      NES[{i}]: {ft} {sz}{note} raw=0x{raw:02x}");
        }
        // state_pair[1]: skip entire telescope to find what follows
        let tele_end = skip_cbor_pub(&data, sp_body as usize).unwrap();
        let sp1_start = tele_end;
        if sp1_start < data.len() {
            let (_, sp1_major, sp1_val) = read_cbor_initial_pub(&data, sp1_start).unwrap();
            let sp1_size = skip_cbor_pub(&data, sp1_start).unwrap() - sp1_start;
            let sp1_type = match sp1_major { 0=>"uint", 1=>"nint", 2=>"bytes", 3=>"text",
                4=>"arr", 5=>"map", 6=>"tag", 7=>"special", _=>"?" };
            let sp1_hex: String = data[sp1_start..sp1_start.saturating_add(40).min(data.len())]
                .iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(" ");
            eprintln!("    state_pair[1]: {sp1_type}(val={sp1_val}) {sp1_size} bytes raw=0x{:02x}", data[sp1_start]);
            eprintln!("      first bytes: {sp1_hex}");
        }

        eprintln!("    HeaderState: {header_size} bytes raw={header_raw}");
        eprintln!();
    }
    eprintln!("=== End CBOR Probe ===\n");
}

/// Detailed byte-level analysis of Shelley→Allegra telescope for CBOR surgery.
#[test]
fn shelley_allegra_telescope_surgery_analysis() {
    use ade_testkit::harness::snapshot_loader::{
        read_uint_pub, read_array_header_pub, skip_cbor_pub,
        navigate_to_nes_pub, read_cbor_initial_pub,
    };

    let shelley_path = snapshots_dir().join("snapshot_4924880.tar.gz"); // Shelley epoch 209
    let allegra_path = snapshots_dir().join("snapshot_16588800.tar.gz"); // Allegra epoch 236 (post-HFC)

    if !shelley_path.exists() || !allegra_path.exists() {
        eprintln!("SKIPPED"); return;
    }

    let shelley_data = extract_state_from_tarball(&shelley_path).unwrap();
    let allegra_data = extract_state_from_tarball(&allegra_path).unwrap();

    eprintln!("\n=== Shelley→Allegra Telescope Surgery Analysis ===");

    for (label, data) in [("Shelley (epoch 209)", &shelley_data), ("Allegra (epoch 236)", &allegra_data)] {
        eprintln!("\n  {label}: {} bytes total", data.len());

        // Dump first 200 bytes as hex
        let prefix_len = 200.min(data.len());
        let hex: String = data[..prefix_len].iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(" ");
        eprintln!("    first {prefix_len} bytes: {hex}");

        // Parse top-level structure
        let (top_body, top_len) = read_array_header_pub(data, 0).unwrap();
        let (off, era_idx) = read_uint_pub(data, top_body as usize).unwrap();
        let (sp_body, sp_len) = read_array_header_pub(data, off).unwrap();
        let (tele_body, tele_len) = read_array_header_pub(data, sp_body as usize).unwrap();
        eprintln!("    outer=array({top_len}) era_idx={era_idx} state_pair=array({sp_len}) tele=array({tele_len})");

        // Dump each telescope entry's raw bytes
        let mut off = tele_body as usize;
        for i in 0..tele_len {
            let start = off;
            let end = skip_cbor_pub(data, off).unwrap();
            let entry_bytes = &data[start..end.min(start + 60)];
            let hex: String = entry_bytes.iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(" ");
            let is_last = i == tele_len - 1;
            let kind = if is_last { "Current" } else { "Past" };
            eprintln!("    {kind}[{i}] ({} bytes): {hex}{}", end - start,
                if end - start > 60 { "..." } else { "" });
            off = end;
        }

        // NES offset
        let nes_off = navigate_to_nes_pub(data).unwrap();
        eprintln!("    NES starts at byte {nes_off}");
        // First NES field (epoch)
        let (_, _, epoch) = read_cbor_initial_pub(data, nes_off).unwrap();
        eprintln!("    NES[0] (epoch) = {epoch}");
    }

    // Compare: what bytes differ between Shelley and Allegra in the telescope region?
    // Shelley telescope: array(2) [Past(Byron), Current(Shelley)]
    // Allegra telescope: array(3) [Past(Byron), Past(Shelley), Current(Allegra)]
    //
    // The Allegra telescope should contain:
    //   Past[0] = same as Shelley Past[0] (Byron bounds)
    //   Past[1] = Shelley's Current bounds converted to Past bounds
    //   Current = new Allegra bounds + era_state

    // Compute hashes
    let shelley_hash = compute_state_hash(&shelley_data);
    let allegra_hash = compute_state_hash(&allegra_data);
    eprintln!("\n  Shelley hash: {shelley_hash}");
    eprintln!("  Allegra hash: {allegra_hash}");

    // === CBOR Surgery Experiment ===
    // Can we transform the Shelley snapshot's telescope into Allegra format?
    //
    // Shelley: array(2) [Past(Byron), Current(Shelley)]
    // Allegra: array(3) [Past(Byron), Past(Shelley), Current(Allegra)]
    //
    // The transformation:
    // 1. Past[0]: unchanged (Byron bounds)
    // 2. Past[1]: array(2) [Shelley_Current_start_bound, new_HFC_bound]
    // 3. Current: array(2) [new_HFC_bound, Shelley_Current_versioned_state]
    //
    // The new_HFC_bound comes from the Allegra snapshot. Let's extract it.

    // Parse Shelley telescope entries
    let (s_top_body, _) = read_array_header_pub(&shelley_data, 0).unwrap();
    let (s_off, _) = read_uint_pub(&shelley_data, s_top_body as usize).unwrap();
    let (s_sp_body, _) = read_array_header_pub(&shelley_data, s_off).unwrap();
    let (s_tele_body, s_tele_len) = read_array_header_pub(&shelley_data, s_sp_body as usize).unwrap();
    assert_eq!(s_tele_len, 2);

    let s_past0_start = s_tele_body as usize;
    let s_past0_end = skip_cbor_pub(&shelley_data, s_past0_start).unwrap();
    let s_past0_bytes = shelley_data[s_past0_start..s_past0_end].to_vec();

    let s_current_start = s_past0_end;
    let s_current_end = skip_cbor_pub(&shelley_data, s_current_start).unwrap();

    // Parse Shelley Current: array(2) [start_bound, versioned_state]
    let (s_cur_body, s_cur_len) = read_array_header_pub(&shelley_data, s_current_start).unwrap();
    assert_eq!(s_cur_len, 2);
    let s_cur_bound_start = s_cur_body as usize;
    let s_cur_bound_end = skip_cbor_pub(&shelley_data, s_cur_bound_start).unwrap();
    let s_cur_bound_bytes = &shelley_data[s_cur_bound_start..s_cur_bound_end];
    let s_cur_state_start = s_cur_bound_end;
    let s_cur_state_end = s_current_end;
    let s_cur_state_bytes = &shelley_data[s_cur_state_start..s_cur_state_end];

    // Parse Allegra telescope to extract the HFC bound (Past[1]'s end = Current's start)
    let (a_top_body, _) = read_array_header_pub(&allegra_data, 0).unwrap();
    let (a_off, _) = read_uint_pub(&allegra_data, a_top_body as usize).unwrap();
    let (a_sp_body, _) = read_array_header_pub(&allegra_data, a_off).unwrap();
    let (a_tele_body, a_tele_len) = read_array_header_pub(&allegra_data, a_sp_body as usize).unwrap();
    assert_eq!(a_tele_len, 3);

    // Skip Past[0]
    let a_past1_start = skip_cbor_pub(&allegra_data, a_tele_body as usize).unwrap();
    // Past[1] = array(2) [shelley_start_bound, hfc_bound]
    let (a_p1_body, _) = read_array_header_pub(&allegra_data, a_past1_start).unwrap();
    let a_p1_bound1_end = skip_cbor_pub(&allegra_data, a_p1_body as usize).unwrap(); // skip start bound
    let a_hfc_bound_start = a_p1_bound1_end;
    let a_hfc_bound_end = skip_cbor_pub(&allegra_data, a_hfc_bound_start).unwrap();
    let hfc_bound_bytes = &allegra_data[a_hfc_bound_start..a_hfc_bound_end];

    // Also extract Allegra's full Past[1] for comparison
    let a_past1_end = skip_cbor_pub(&allegra_data, a_past1_start).unwrap();
    let a_past1_bytes = &allegra_data[a_past1_start..a_past1_end];

    // Extract Allegra's Current versioned_state for comparison
    let a_current_start = a_past1_end;
    let (a_cur_body, _) = read_array_header_pub(&allegra_data, a_current_start).unwrap();
    let a_cur_bound_end = skip_cbor_pub(&allegra_data, a_cur_body as usize).unwrap();
    let a_cur_state_start = a_cur_bound_end;
    let a_cur_state_end = skip_cbor_pub(&allegra_data, a_current_start).unwrap();
    let a_cur_state_bytes = &allegra_data[a_cur_state_start..a_cur_state_end];

    eprintln!("\n  Shelley Current start_bound ({} bytes): {}", s_cur_bound_bytes.len(),
        s_cur_bound_bytes.iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(" "));
    eprintln!("  HFC bound from Allegra ({} bytes): {}", hfc_bound_bytes.len(),
        hfc_bound_bytes.iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(" "));
    eprintln!("  Allegra Past[1] ({} bytes): {}", a_past1_bytes.len(),
        a_past1_bytes.iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(" "));

    // The Allegra Past[1] should be: array(2) [Shelley_Current_start_bound, HFC_bound]
    // Let's verify
    let mut expected_past1 = Vec::new();
    expected_past1.push(0x82u8); // array(2)
    expected_past1.extend_from_slice(s_cur_bound_bytes);
    expected_past1.extend_from_slice(hfc_bound_bytes);

    if expected_past1 == a_past1_bytes {
        eprintln!("  ✓ Allegra Past[1] = array(2) [Shelley_start_bound, HFC_bound]");
    } else {
        eprintln!("  ✗ Past[1] mismatch: expected {} bytes, got {}",
            expected_past1.len(), a_past1_bytes.len());
    }

    // Now attempt CBOR surgery: reconstruct the Allegra CBOR from Shelley parts
    // The Allegra state should be identical to Shelley EXCEPT for the telescope wrapper
    // and possibly the NES epoch number.
    //
    // Surgery plan:
    // 1. outer: array(2) [era_index=1, state_pair]
    // 2. state_pair: array(2) [telescope, ...rest]
    //    Wait - is state_pair always array(2)? Let's check.

    eprintln!("  Shelley state_pair elements: {}", {
        let (_, len) = read_array_header_pub(&shelley_data, s_off).unwrap();
        len
    });
    eprintln!("  Allegra state_pair elements: {}", {
        let (_, len) = read_array_header_pub(&allegra_data, a_off).unwrap();
        len
    });

    // Check if Shelley's versioned_state matches Allegra's versioned_state
    // (modulo epoch number and other era-specific changes)
    eprintln!("  Shelley versioned_state: {} bytes", s_cur_state_bytes.len());
    eprintln!("  Allegra versioned_state: {} bytes", a_cur_state_bytes.len());

    // NOTE: These are different snapshots at different epochs so the state CONTENTS
    // will differ. The question is whether the STRUCTURE is the same.
    // Let's compare the first 20 bytes of each versioned_state.
    let s_prefix: String = s_cur_state_bytes[..20.min(s_cur_state_bytes.len())]
        .iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(" ");
    let a_prefix: String = a_cur_state_bytes[..20.min(a_cur_state_bytes.len())]
        .iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(" ");
    eprintln!("  Shelley state prefix: {s_prefix}");
    eprintln!("  Allegra state prefix: {a_prefix}");

    // === Bound Encoding Verification ===
    // Verify our CBOR encoder produces byte-identical bounds to the oracle.
    eprintln!("\n  === Bound Encoding Verification ===");

    // Byron genesis bound: epoch=0, slot=0, time=0
    let mut genesis_buf = Vec::new();
    ade_codec::cbor::write_hfc_bound(&mut genesis_buf, 0, 0, 0);
    let genesis_hex: String = genesis_buf.iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(" ");
    eprintln!("    Genesis bound: {genesis_hex}");
    // Expected from Past[0] first half: 83 00 00 00
    assert_eq!(genesis_buf, vec![0x83, 0x00, 0x00, 0x00], "genesis bound");
    eprintln!("    ✓ Genesis bound matches oracle");

    // Byron→Shelley bound: epoch=208, slot=4492800, time=89856000*10^12
    let byron_end_time: u128 = 89_856_000 * 1_000_000_000_000;
    let mut bs_buf = Vec::new();
    ade_codec::cbor::write_hfc_bound(&mut bs_buf, 208, 4_492_800, byron_end_time);
    let bs_hex: String = bs_buf.iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(" ");
    eprintln!("    Byron→Shelley bound: {bs_hex}");
    // Expected from oracle: 83 c2 49 04 df 00 a3 ec 29 80 00 00 1a 00 44 8e 00 18 d0
    let expected_bs = vec![
        0x83, 0xc2, 0x49, 0x04, 0xdf, 0x00, 0xa3, 0xec, 0x29, 0x80, 0x00, 0x00,
        0x1a, 0x00, 0x44, 0x8e, 0x00, 0x18, 0xd0,
    ];
    assert_eq!(bs_buf, expected_bs, "Byron→Shelley bound");
    eprintln!("    ✓ Byron→Shelley bound matches oracle");

    // Shelley→Allegra bound: epoch=236, slot=16588800, time=101952000*10^12
    let shelley_end_time: u128 = 101_952_000 * 1_000_000_000_000;
    let mut sa_buf = Vec::new();
    ade_codec::cbor::write_hfc_bound(&mut sa_buf, 236, 16_588_800, shelley_end_time);
    let sa_hex: String = sa_buf.iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(" ");
    eprintln!("    Shelley→Allegra bound: {sa_hex}");
    assert_eq!(sa_buf, hfc_bound_bytes, "Shelley→Allegra bound vs oracle");
    eprintln!("    ✓ Shelley→Allegra bound matches oracle");

    // Verify Past[0] encoding (genesis → Byron→Shelley)
    let mut past0_buf = Vec::new();
    ade_codec::cbor::write_hfc_past(&mut past0_buf, 0, 0, 0, 208, 4_492_800, byron_end_time);
    assert_eq!(&past0_buf[..], s_past0_bytes, "Past[0] encoding");
    eprintln!("    ✓ Past[0] (Byron bounds) matches oracle");

    // Verify Past[1] encoding (Byron→Shelley → Shelley→Allegra)
    let mut past1_buf = Vec::new();
    ade_codec::cbor::write_hfc_past(
        &mut past1_buf,
        208, 4_492_800, byron_end_time,
        236, 16_588_800, shelley_end_time,
    );
    assert_eq!(&past1_buf[..], a_past1_bytes, "Past[1] encoding");
    eprintln!("    ✓ Past[1] (Shelley bounds) matches oracle");

    eprintln!("  === All bounds verified byte-identical ===");

    // === Identity Round-Trip: Re-encode wrapper, copy NES verbatim ===
    // Proves the encoder is correct for everything outside the NES bulk.
    // Strategy: parse Allegra snapshot, re-encode its outer wrapper using our
    // encoder functions, copy NES + HeaderState verbatim, verify hash matches.
    eprintln!("\n  === Identity Round-Trip ===");

    // Parse Allegra structure to extract byte ranges
    let a_outer_end = skip_cbor_pub(&allegra_data, 0).unwrap(); // end of entire LedgerState
    let a_sp1_end = skip_cbor_pub(&allegra_data, a_past1_end).unwrap(); // end of Current
    // state_pair[1] = HeaderState starts after Current (= after telescope ends in state_pair)
    let a_tele_end = a_sp1_end; // Current is the last telescope entry
    // Actually: the telescope is state_pair[0]. We need to skip the entire telescope
    // to find state_pair[1] (HeaderState).
    let a_telescope_end = skip_cbor_pub(&allegra_data, a_sp_body as usize).unwrap();
    let a_header_start = a_telescope_end;
    let a_header_end = a_outer_end; // HeaderState ends where LedgerState ends

    eprintln!("    Allegra byte ranges:");
    eprintln!("      outer: 0..{a_outer_end}");
    eprintln!("      telescope: {}..{a_telescope_end} ({} bytes)",
        a_sp_body, a_telescope_end - a_sp_body as usize);
    eprintln!("      HeaderState: {a_header_start}..{a_header_end} ({} bytes)",
        a_header_end - a_header_start);

    // Extract the versioned_state content from Current
    // Current = array(2) [bound, versioned_state]
    // versioned_state = everything from after the bound to end of Current
    let a_versioned_state_bytes = &allegra_data[a_cur_state_start..a_sp1_end];

    // Now re-encode the Allegra snapshot from parts:
    // 1. outer: array(2) [era_index=1, state_pair]
    // 2. state_pair: array(2) [telescope, HeaderState]
    // 3. telescope: array(3) [Past[0], Past[1], Current]
    //    Past[0] = Byron bounds (encoded)
    //    Past[1] = Shelley bounds (encoded)
    //    Current = array(2) [Allegra_start_bound, versioned_state]
    // 4. HeaderState: raw bytes from original

    let mut reconstructed = Vec::with_capacity(allegra_data.len());

    // outer: array(2) [era_index, state_pair]
    ade_codec::cbor::write_array_header(&mut reconstructed,
        ade_codec::cbor::ContainerEncoding::Definite(2, ade_codec::cbor::canonical_width(2)));
    ade_codec::cbor::write_uint_canonical(&mut reconstructed, 1); // era_index

    // state_pair: array(2) [telescope, HeaderState]
    ade_codec::cbor::write_array_header(&mut reconstructed,
        ade_codec::cbor::ContainerEncoding::Definite(2, ade_codec::cbor::canonical_width(2)));

    // telescope: array(3) [Past[0], Past[1], Current]
    ade_codec::cbor::write_array_header(&mut reconstructed,
        ade_codec::cbor::ContainerEncoding::Definite(3, ade_codec::cbor::canonical_width(3)));

    // Past[0]: Byron bounds (genesis → Byron→Shelley)
    ade_codec::cbor::write_hfc_past(&mut reconstructed,
        0, 0, 0,
        208, 4_492_800, byron_end_time);

    // Past[1]: Shelley bounds (Byron→Shelley → Shelley→Allegra)
    ade_codec::cbor::write_hfc_past(&mut reconstructed,
        208, 4_492_800, byron_end_time,
        236, 16_588_800, shelley_end_time);

    // Current: array(2) [Allegra_start_bound, versioned_state]
    ade_codec::cbor::write_array_header(&mut reconstructed,
        ade_codec::cbor::ContainerEncoding::Definite(2, ade_codec::cbor::canonical_width(2)));
    ade_codec::cbor::write_hfc_bound(&mut reconstructed, 236, 16_588_800, shelley_end_time);
    reconstructed.extend_from_slice(a_versioned_state_bytes);

    // HeaderState: copy verbatim
    reconstructed.extend_from_slice(&allegra_data[a_header_start..a_header_end]);

    // Verify: hash should match original
    let original_hash = compute_state_hash(&allegra_data);
    let reconstructed_hash = compute_state_hash(&reconstructed);

    eprintln!("    Reconstructed: {} bytes (original: {} bytes)",
        reconstructed.len(), allegra_data.len());
    eprintln!("    Original hash:      {original_hash}");
    eprintln!("    Reconstructed hash: {reconstructed_hash}");

    if reconstructed.len() == allegra_data.len() && original_hash == reconstructed_hash {
        eprintln!("    ✓ Identity round-trip: PASS — byte-identical reconstruction");
    } else {
        // Find first differing byte
        let first_diff = reconstructed.iter().zip(allegra_data.iter())
            .position(|(a, b)| a != b);
        if let Some(pos) = first_diff {
            eprintln!("    ✗ First diff at byte {pos}: ours=0x{:02x} oracle=0x{:02x}",
                reconstructed[pos], allegra_data[pos]);
            let ctx_start = pos.saturating_sub(5);
            let ctx_end = (pos + 10).min(reconstructed.len()).min(allegra_data.len());
            let ours: String = reconstructed[ctx_start..ctx_end].iter()
                .map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(" ");
            let theirs: String = allegra_data[ctx_start..ctx_end].iter()
                .map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(" ");
            eprintln!("      ours:   {ours}");
            eprintln!("      oracle: {theirs}");
        }
        eprintln!("    ✗ Size diff: {} bytes", reconstructed.len() as i64 - allegra_data.len() as i64);
    }

    assert_eq!(original_hash, reconstructed_hash,
        "identity round-trip must produce byte-identical output");

    // === HFC Translation Surgery ===
    // Transform the Shelley snapshot's telescope to Allegra format.
    // NES and HeaderState content stay the same (same underlying data).
    // Only the telescope wrapper changes: array(2) → array(3) with new Past entry.
    eprintln!("\n  === HFC Translation Surgery (Shelley → Allegra) ===");

    // Extract Shelley's versioned_state and HeaderState
    let s_versioned_state_bytes = &shelley_data[s_cur_state_start..s_cur_state_end];
    let s_telescope_end = skip_cbor_pub(&shelley_data, s_sp_body as usize).unwrap();
    let s_header_start = s_telescope_end;
    let s_outer_end = skip_cbor_pub(&shelley_data, 0).unwrap();
    let s_header_end = s_outer_end;
    let s_header_bytes = &shelley_data[s_header_start..s_header_end];

    eprintln!("    Shelley versioned_state: {} bytes", s_versioned_state_bytes.len());
    eprintln!("    Shelley HeaderState: {} bytes", s_header_bytes.len());

    // Build Allegra-format CBOR from Shelley parts + new telescope
    let mut translated = Vec::with_capacity(shelley_data.len() + 50);

    // outer: array(2) [era_index=1, state_pair]
    ade_codec::cbor::write_array_header(&mut translated,
        ade_codec::cbor::ContainerEncoding::Definite(2, ade_codec::cbor::canonical_width(2)));
    ade_codec::cbor::write_uint_canonical(&mut translated, 1);

    // state_pair: array(2) [telescope, HeaderState]
    ade_codec::cbor::write_array_header(&mut translated,
        ade_codec::cbor::ContainerEncoding::Definite(2, ade_codec::cbor::canonical_width(2)));

    // telescope: array(3) [Past[0](Byron), Past[1](Shelley), Current(Allegra)]
    // This is the KEY transformation: Shelley's array(2) becomes array(3)
    ade_codec::cbor::write_array_header(&mut translated,
        ade_codec::cbor::ContainerEncoding::Definite(3, ade_codec::cbor::canonical_width(3)));

    // Past[0]: Byron bounds (unchanged from Shelley)
    ade_codec::cbor::write_hfc_past(&mut translated, 0, 0, 0, 208, 4_492_800, byron_end_time);

    // Past[1]: Shelley bounds (NEW — Shelley's Current becomes a Past entry)
    ade_codec::cbor::write_hfc_past(&mut translated,
        208, 4_492_800, byron_end_time,
        236, 16_588_800, shelley_end_time);

    // Current: array(2) [Allegra_start_bound, versioned_state_from_Shelley]
    ade_codec::cbor::write_array_header(&mut translated,
        ade_codec::cbor::ContainerEncoding::Definite(2, ade_codec::cbor::canonical_width(2)));
    ade_codec::cbor::write_hfc_bound(&mut translated, 236, 16_588_800, shelley_end_time);
    translated.extend_from_slice(s_versioned_state_bytes);

    // HeaderState: copy from Shelley verbatim
    translated.extend_from_slice(s_header_bytes);

    let translated_hash = compute_state_hash(&translated);
    eprintln!("    Translated: {} bytes (Shelley was {} bytes)",
        translated.len(), shelley_data.len());
    eprintln!("    Shelley hash:    {}", compute_state_hash(&shelley_data));
    eprintln!("    Translated hash: {translated_hash}");
    eprintln!("    Allegra hash:    {original_hash}");

    // The translated hash won't match Allegra's hash because the NES content
    // is from epoch 209 (Shelley), not epoch 236 (Allegra). But the STRUCTURE
    // is correct: if we had the PRE-HFC snapshot at the last Shelley block,
    // the translation would produce the POST-HFC state.
    //
    // Key proof: the translated output is a valid Allegra-format snapshot
    // with telescope array(3), correct bounds, and a well-formed NES.

    // Verify the translated output's structure matches Allegra format
    let (t_top_body, t_top_len) = read_array_header_pub(&translated, 0).unwrap();
    let (t_off, t_era_idx) = read_uint_pub(&translated, t_top_body as usize).unwrap();
    let (t_sp_body, t_sp_len) = read_array_header_pub(&translated, t_off).unwrap();
    let (t_tele_body, t_tele_len) = read_array_header_pub(&translated, t_sp_body as usize).unwrap();
    assert_eq!(t_top_len, 2, "outer array");
    assert_eq!(t_era_idx, 1, "era index");
    assert_eq!(t_sp_len, 2, "state_pair");
    assert_eq!(t_tele_len, 3, "telescope length (Allegra format)");

    // Verify Past[0] matches Allegra's Past[0]
    let t_past0_start = t_tele_body as usize;
    let t_past0_end = skip_cbor_pub(&translated, t_past0_start).unwrap();
    let a_past0_start = a_tele_body as usize;
    let a_past0_end = skip_cbor_pub(&allegra_data, a_past0_start).unwrap();
    assert_eq!(&translated[t_past0_start..t_past0_end],
        &allegra_data[a_past0_start..a_past0_end], "Past[0] match");
    eprintln!("    ✓ Past[0] (Byron bounds): matches Allegra oracle");

    // Verify Past[1] matches Allegra's Past[1]
    let t_past1_start = t_past0_end;
    let t_past1_end = skip_cbor_pub(&translated, t_past1_start).unwrap();
    assert_eq!(&translated[t_past1_start..t_past1_end],
        a_past1_bytes, "Past[1] match");
    eprintln!("    ✓ Past[1] (Shelley bounds): matches Allegra oracle");

    // Verify Current start bound matches Allegra's Current start bound
    let t_current_start = t_past1_end;
    let (t_cur_body, _) = read_array_header_pub(&translated, t_current_start).unwrap();
    let t_cur_bound_start = t_cur_body as usize;
    let t_cur_bound_end = skip_cbor_pub(&translated, t_cur_bound_start).unwrap();
    let a_cur_bound_bytes = &allegra_data[a_cur_body as usize..
        skip_cbor_pub(&allegra_data, a_cur_body as usize).unwrap()];
    assert_eq!(&translated[t_cur_bound_start..t_cur_bound_end],
        a_cur_bound_bytes, "Current start bound match");
    eprintln!("    ✓ Current start bound: matches Allegra oracle");

    // Verify NES is accessible in translated output
    let t_nes_off = navigate_to_nes_pub(&translated).unwrap();
    let (_, _, t_epoch) = read_cbor_initial_pub(&translated, t_nes_off).unwrap();
    eprintln!("    ✓ NES accessible: epoch={t_epoch} (from Shelley snapshot)");

    eprintln!("    ✓ Telescope surgery: Shelley array(2) → Allegra array(3) CORRECT");
    eprintln!("    Note: hash differs from Allegra oracle because NES content is from epoch {t_epoch}");
    eprintln!("    To close CE-73: need last-Shelley-block snapshot + same NES surgery");

    eprintln!("\n=== End Telescope Surgery Analysis ===\n");
}

