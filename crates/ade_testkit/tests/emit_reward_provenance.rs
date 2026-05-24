//! Emit `reward_provenance/*_tick_registered_creds.txt` files from snapshots.
//!
//! Format: one hex-encoded 28-byte credential hash per line (no separators).
//! Consumer: `epoch_oracle_comparison.rs` lines ~3795-3815.
//!
//! Run with:
//!   cargo test -p ade_testkit --test emit_reward_provenance -- --ignored --nocapture
//!
//! Inputs:
//!   alonzo310: corpus/snapshots/snapshot_48557176.tar.gz (epoch 310 tick)
//!   conway576: corpus/snapshots/snapshot_163468813.tar.gz (epoch 576 tick)

use std::io::Write;
use std::path::PathBuf;

use ade_testkit::harness::snapshot_loader::{extract_state_from_tarball, parse_registered_credentials};

fn corpus_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("corpus")
}

fn emit(tarball_name: &str, out_name: &str) -> Result<(), String> {
    let tarball = corpus_root().join("snapshots").join(tarball_name);
    if !tarball.exists() {
        return Err(format!("tarball not found: {}", tarball.display()));
    }
    eprintln!("loading {}", tarball.display());
    let state = extract_state_from_tarball(&tarball)
        .map_err(|e| format!("extract_state_from_tarball: {e:?}"))?;
    eprintln!("  state size: {} bytes", state.len());

    let creds = parse_registered_credentials(&state)
        .map_err(|e| format!("parse_registered_credentials: {e:?}"))?;
    eprintln!("  parsed {} registered credentials", creds.len());

    let out_dir = corpus_root().join("snapshots").join("reward_provenance");
    std::fs::create_dir_all(&out_dir).map_err(|e| format!("mkdir: {e}"))?;
    let out_path = out_dir.join(out_name);

    let mut f = std::fs::File::create(&out_path).map_err(|e| format!("create: {e}"))?;
    for cred in &creds {
        for b in cred.0.iter() {
            write!(f, "{b:02x}").map_err(|e| format!("write: {e}"))?;
        }
        writeln!(f).map_err(|e| format!("write: {e}"))?;
    }
    let size = std::fs::metadata(&out_path).map_err(|e| format!("stat: {e}"))?.len();
    eprintln!("  wrote {} ({} bytes, {} creds)", out_path.display(), size, creds.len());
    Ok(())
}

#[test]
#[ignore]
fn emit_alonzo310_registered_creds() {
    emit("snapshot_48557176.tar.gz", "alonzo310_tick_registered_creds.txt")
        .expect("alonzo310 emit failed");
}

#[test]
#[ignore]
fn emit_conway576_registered_creds() {
    emit("snapshot_163468813.tar.gz", "conway576_tick_registered_creds.txt")
        .expect("conway576 emit failed");
}
