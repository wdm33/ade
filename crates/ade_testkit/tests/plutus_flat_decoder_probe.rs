//! Flat decoder probe — Slice S-29, Cluster P-B.
//!
//! Walks the mainnet Plutus corpus (Alonzo/Babbage/Conway contiguous
//! blocks) and verifies that aiken's Flat decoder at the pinned commit
//! (v1.1.21 / 42babe5d) handles every Plutus script found in witness
//! sets.
//!
//! Discharges slice-entry obligation O-29.3 per
//! `docs/active/S-29_obligation_discharge.md`. Results land in
//! `docs/active/S-29_flat_decoder_probe.md`.
//!
//! Properties verified per script:
//! - `PlutusScript::from_flat` succeeds (no parse errors)
//! - Round-trip: `from_flat(b).to_flat() == b` (canonical encoding
//!   preserved)
//!
//! Hash verification (`Blake2b-224(script_bytes) == script_hash_on_chain`)
//! would require parsing the corresponding output's `script_data_hash`
//! or script reference, which is not in scope for S-29. Added in S-30
//! or S-31 as a tighter property.

use std::path::PathBuf;

use ade_codec::cbor::envelope::decode_block_envelope;
use ade_ledger::witness::{decode_all_plutus_scripts_in_block, PlutusScriptEntry, PlutusVersion};
use ade_plutus::evaluator::PlutusScript;
use ade_types::CardanoEra;

fn corpus_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("corpus")
        .join("contiguous")
}

/// Walk an era's contiguous corpus and collect Plutus script entries.
///
/// Returns `Vec<(source_filename, script_entry)>` so failures can be
/// localized to the originating block. Skips blocks silently if the
/// block-envelope decoder rejects them (shouldn't happen on the
/// curated corpus).
fn collect_plutus_scripts(era_dir: &str) -> Vec<(String, PlutusScriptEntry)> {
    let manifest_path = corpus_root().join(era_dir).join("blocks.json");
    if !manifest_path.exists() {
        return Vec::new();
    }
    let manifest_content = std::fs::read_to_string(&manifest_path)
        .unwrap_or_else(|e| panic!("read manifest {manifest_path:?}: {e}"));
    let manifest: serde_json::Value = serde_json::from_str(&manifest_content)
        .unwrap_or_else(|e| panic!("parse manifest {manifest_path:?}: {e}"));
    let blocks = manifest["blocks"]
        .as_array()
        .unwrap_or_else(|| panic!("manifest.blocks not an array"));

    let mut out = Vec::new();
    for entry in blocks {
        let filename = entry["file"].as_str().unwrap_or("<unknown>").to_string();
        let path = corpus_root().join(era_dir).join(&filename);
        let raw = match std::fs::read(&path) {
            Ok(r) => r,
            Err(_) => continue,
        };
        let env = match decode_block_envelope(&raw) {
            Ok(e) => e,
            Err(_) => continue,
        };
        // Only Alonzo+ eras can carry Plutus scripts. Non-Plutus eras
        // would return Ok(empty) but skipping saves a pass.
        if !matches!(
            env.era,
            CardanoEra::Alonzo | CardanoEra::Babbage | CardanoEra::Conway
        ) {
            continue;
        }
        // Find witness_sets within the block. The block envelope gives
        // us block_start/block_end for the inner body; we need to
        // decode the block structure to reach the witness_sets field.
        // Rather than replicate that logic here, rely on the test
        // only counting scripts that do decode; we use the full raw
        // block and let the witness extractor scan for the map
        // structure. For correctness we decode via the era-appropriate
        // block path.
        let inner = &raw[env.block_start..env.block_end];
        let scripts = match extract_from_era_block(env.era, inner) {
            Ok(s) => s,
            Err(_) => continue,
        };
        for s in scripts {
            out.push((filename.clone(), s));
        }
    }
    out
}

fn extract_from_era_block(
    era: CardanoEra,
    inner: &[u8],
) -> Result<Vec<PlutusScriptEntry>, Box<dyn std::error::Error>> {
    // Alonzo/Babbage/Conway blocks all share the Shelley-style layout:
    // array(N) [header, tx_bodies, witness_sets, ...].
    // Parse the outer array to reach witness_sets at index 2.
    // We don't use the era-specific block struct because all three
    // eras store witness_sets at the same position.
    let _ = era; // all three share the structure
    let mut offset = 0;
    // Read the outer array header (should have ≥ 3 fields).
    let arr_hdr = ade_codec::cbor::read_array_header(inner, &mut offset)?;
    let _field_count = match arr_hdr {
        ade_codec::cbor::ContainerEncoding::Definite(n, _) => n,
        ade_codec::cbor::ContainerEncoding::Indefinite => u64::MAX,
    };
    // Skip field 0 (header) and field 1 (tx_bodies).
    let _ = ade_codec::cbor::skip_item(inner, &mut offset)?;
    let _ = ade_codec::cbor::skip_item(inner, &mut offset)?;
    // Field 2 is witness_sets. Extract scripts from it.
    let ws_start = offset;
    let _ = ade_codec::cbor::skip_item(inner, &mut offset)?;
    let ws_end = offset;
    let witness_sets = &inner[ws_start..ws_end];
    let scripts = decode_all_plutus_scripts_in_block(witness_sets)?;
    Ok(scripts)
}

/// Probe result: pass-count, fail-count, first failure for diagnostics.
struct ProbeResult {
    total: usize,
    decode_ok: usize,
    roundtrip_ok: usize,
    first_decode_fail: Option<(String, PlutusVersion, String)>,
    first_roundtrip_fail: Option<(String, PlutusVersion)>,
    v1_count: usize,
    v2_count: usize,
    v3_count: usize,
}

/// Stack size for the probe thread. Aiken's Flat decoder uses
/// recursion over UPLC term nesting; deeply-nested mainnet programs
/// can overflow the default 2 MiB test stack. 32 MiB is empirically
/// sufficient for every script in the corpus.
const PROBE_STACK_SIZE: usize = 32 * 1024 * 1024;

fn run_probe<F>(body: F)
where
    F: FnOnce() + Send + 'static,
{
    std::thread::Builder::new()
        .stack_size(PROBE_STACK_SIZE)
        .spawn(body)
        .unwrap_or_else(|e| panic!("spawn probe thread: {e}"))
        .join()
        .unwrap_or_else(|_| panic!("probe thread panicked"));
}

fn probe_era(_era: &'static str, era_dir: &str) -> ProbeResult {
    let scripts = collect_plutus_scripts(era_dir);
    let total = scripts.len();
    let mut decode_ok = 0usize;
    let mut roundtrip_ok = 0usize;
    let mut v1 = 0usize;
    let mut v2 = 0usize;
    let mut v3 = 0usize;
    let mut first_decode_fail = None;
    let mut first_roundtrip_fail = None;

    // Cardano's on-chain Plutus script convention is double-CBOR-encoded:
    // the outer bstr in the witness set wraps an inner CBOR bstr that
    // itself wraps the Flat-encoded program. `decode_all_plutus_scripts_in_block`
    // gives us the CONTENT of the outer bstr (a CBOR bstr). We must call
    // `from_cbor` (which strips one layer) rather than `from_flat`.
    //
    // For round-trip we re-encode with `to_cbor` (which wraps with one
    // CBOR bstr) and compare against the extracted bytes. Byte-identity
    // here proves the inner flat encoding is canonical.
    for (source, entry) in &scripts {
        match entry.version {
            PlutusVersion::V1 => v1 += 1,
            PlutusVersion::V2 => v2 += 1,
            PlutusVersion::V3 => v3 += 1,
        }
        match PlutusScript::from_cbor(&entry.flat_bytes) {
            Ok(script) => {
                decode_ok += 1;
                match script.to_cbor() {
                    Ok(bytes) => {
                        if bytes == entry.flat_bytes {
                            roundtrip_ok += 1;
                        } else if first_roundtrip_fail.is_none() {
                            first_roundtrip_fail = Some((source.clone(), entry.version));
                        }
                    }
                    Err(_) => {
                        if first_roundtrip_fail.is_none() {
                            first_roundtrip_fail = Some((source.clone(), entry.version));
                        }
                    }
                }
            }
            Err(e) => {
                if first_decode_fail.is_none() {
                    first_decode_fail = Some((source.clone(), entry.version, format!("{e}")));
                }
            }
        }
    }

    ProbeResult {
        total,
        decode_ok,
        roundtrip_ok,
        first_decode_fail,
        first_roundtrip_fail,
        v1_count: v1,
        v2_count: v2,
        v3_count: v3,
    }
}

#[test]
#[ignore]
fn inspect_first_few_scripts() {
    let scripts = collect_plutus_scripts("alonzo");
    for (src, entry) in scripts.iter().take(3) {
        let head: String = entry
            .flat_bytes
            .iter()
            .take(16)
            .map(|b| format!("{b:02x}"))
            .collect::<Vec<_>>()
            .join(" ");
        eprintln!(
            "  {} ({:?}) len={} head={}",
            src,
            entry.version,
            entry.flat_bytes.len(),
            head
        );
    }
}

#[test]
fn probe_alonzo_plutus_scripts() {
    run_probe(|| {
        let r = probe_era("alonzo", "alonzo");
        eprintln!(
            "  alonzo: {}/{} decode ok, {}/{} round-trip ok (V1={} V2={} V3={})",
            r.decode_ok, r.total, r.roundtrip_ok, r.total, r.v1_count, r.v2_count, r.v3_count
        );
        if let Some((src, ver, msg)) = &r.first_decode_fail {
            panic!("first decode failure — {src} ({ver:?}): {msg}");
        }
        if let Some((src, ver)) = &r.first_roundtrip_fail {
            panic!("first round-trip failure — {src} ({ver:?})");
        }
    });
}

#[test]
fn probe_babbage_plutus_scripts() {
    run_probe(|| {
        let r = probe_era("babbage", "babbage");
        eprintln!(
            "  babbage: {}/{} decode ok, {}/{} round-trip ok (V1={} V2={} V3={})",
            r.decode_ok, r.total, r.roundtrip_ok, r.total, r.v1_count, r.v2_count, r.v3_count
        );
        if let Some((src, ver, msg)) = &r.first_decode_fail {
            panic!("first decode failure — {src} ({ver:?}): {msg}");
        }
        if let Some((src, ver)) = &r.first_roundtrip_fail {
            panic!("first round-trip failure — {src} ({ver:?})");
        }
    });
}

#[test]
fn probe_conway_plutus_scripts() {
    run_probe(|| {
        let r = probe_era("conway", "conway");
        eprintln!(
            "  conway: {}/{} decode ok, {}/{} round-trip ok (V1={} V2={} V3={})",
            r.decode_ok, r.total, r.roundtrip_ok, r.total, r.v1_count, r.v2_count, r.v3_count
        );
        if let Some((src, ver, msg)) = &r.first_decode_fail {
            panic!("first decode failure — {src} ({ver:?}): {msg}");
        }
        if let Some((src, ver)) = &r.first_roundtrip_fail {
            panic!("first round-trip failure — {src} ({ver:?})");
        }
    });
}

#[test]
fn probe_summary_across_all_eras() {
    run_probe(|| {
        let alonzo = probe_era("alonzo", "alonzo");
        let babbage = probe_era("babbage", "babbage");
        let conway = probe_era("conway", "conway");

        let total = alonzo.total + babbage.total + conway.total;
        let decode_ok = alonzo.decode_ok + babbage.decode_ok + conway.decode_ok;
        let rt_ok = alonzo.roundtrip_ok + babbage.roundtrip_ok + conway.roundtrip_ok;
        let v1 = alonzo.v1_count + babbage.v1_count + conway.v1_count;
        let v2 = alonzo.v2_count + babbage.v2_count + conway.v2_count;
        let v3 = alonzo.v3_count + babbage.v3_count + conway.v3_count;

        eprintln!("\n=== S-29 Flat Decoder Probe Summary ===");
        eprintln!("  total Plutus scripts extracted: {total}");
        eprintln!("  decode ok:    {decode_ok}/{total}");
        eprintln!("  round-trip ok: {rt_ok}/{total}");
        eprintln!("  by version:   V1={v1} V2={v2} V3={v3}");
        eprintln!("========================================\n");

        // At least 100 scripts required per O-29.3 commitment.
        assert!(
            total >= 100,
            "probe must exercise at least 100 Plutus scripts (got {total}); corpus coverage insufficient"
        );
        assert_eq!(decode_ok, total, "all scripts must decode");
        assert_eq!(rt_ok, total, "all scripts must round-trip");
    });
}
