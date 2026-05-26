// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Integration smoke test for the operator-supplied
//! consensus-inputs bundle captured by
//! `ci/build_consensus_inputs_bundle.sh`. If the bundle file
//! exists at the expected path, import it through
//! `import_live_consensus_inputs` and assert basic shape +
//! deterministic fingerprint across two import runs.
//!
//! Skipped (not failed) when the bundle file is absent — the
//! generator script is operator-run, so CI environments without
//! the docker preprod peer cannot produce one.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use ade_runtime::consensus_inputs::import_live_consensus_inputs;
use ade_types::CardanoEra;
use std::path::Path;

const BUNDLE_PATH: &str = "../../docs/evidence/phase4-n-m-c-consensus-inputs.json";

#[test]
fn live_bundle_imports_with_conway_era_and_deterministic_fingerprint() {
    let p = Path::new(BUNDLE_PATH);
    if !p.exists() {
        eprintln!(
            "skipping live_bundle smoke: {} not present (operator hasn't generated yet)",
            BUNDLE_PATH
        );
        return;
    }
    let a = import_live_consensus_inputs(p).expect("import A");
    let b = import_live_consensus_inputs(p).expect("import B");
    assert_eq!(a.era, CardanoEra::Conway);
    assert!(!a.pool_distribution.is_empty());
    assert!(!a.pool_vrf_keyhashes.is_empty());
    assert_eq!(a.fingerprint, b.fingerprint, "fingerprint must be deterministic");
}
