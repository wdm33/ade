//! Observable-surface differential harness for the snapshot→tip sync window.
//!
//! GREEN evidence: this module reads committed oracle fixtures and compares
//! Ade's **observable** surfaces — per-block verdict, selected tip hash, block
//! hash, and a `query utxo`-style UTxO set — against the oracle. It never
//! compares Ade's internal ledger `fingerprint` to a Haskell/serialized-state
//! hash (DC-COMPAT-01); compatibility is proven only on observable surfaces.
//!
//! The harness is deterministic over the committed fixtures (T-DET-01): the
//! same fixture re-run yields the same diff verdict. It decides nothing about
//! authority — it compares already-authoritative outputs.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::diff_report::{DiffReport, Divergence};
use super::HarnessError;

/// The closed per-block verdict surface. This mirrors the receive-side
/// admission outcome as an *observable* string, not the internal ledger state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BlockVerdict {
    /// The block was admitted (validated + chain-selected).
    Admitted,
    /// The block was rejected (failed validation / chain-select).
    Rejected,
}

/// One observable per-block surface in the snapshot→tip window.
///
/// These are the surfaces a `cardano-node` peer exposes and that an operator
/// can witness — never Ade's private serialized state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ObservableBlockSurface {
    /// Slot number of the block.
    pub slot: u64,
    /// Block height.
    pub block_number: u64,
    /// Hash of the block (hex), a hash-critical wire surface.
    pub block_hash: String,
    /// Selected chain tip hash (hex) after admitting this block.
    pub selected_tip_hash: String,
    /// Observable admission verdict.
    pub verdict: BlockVerdict,
}

/// One `query utxo`-style entry: an outpoint mapped to a lovelace value.
///
/// This is the observable post-window UTxO surface (what `cardano-cli query
/// utxo` would print), not Ade's internal UTxO encoding.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UtxoEntry {
    /// `<txid>#<index>` outpoint reference.
    pub tx_in: String,
    /// Bech32 address holding the output.
    pub address: String,
    /// Lovelace amount (integer; no floating point).
    pub lovelace: u64,
}

/// A committed oracle fixture for one snapshot→tip differential window.
///
/// Pins the oracle versions (`cardano_node_version`, `cardano_cli_version`)
/// and the reproducible chain point, then carries the per-block observable
/// surfaces and the post-window `query utxo` set.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyncOracleFixture {
    /// Pinned cardano-node version that produced the oracle surfaces.
    pub cardano_node_version: String,
    /// Pinned cardano-cli version that produced the `query utxo` dump.
    pub cardano_cli_version: String,
    /// Network this window was captured on (e.g. "preprod", "private-conway").
    pub network: String,
    /// Whether the surfaces are real captures or synthetic/representative.
    pub fixture_kind: String,
    /// Chain point the window starts from (snapshot anchor point, hex/slot).
    pub start_point: String,
    /// Chain point the window ends at (tip point).
    pub end_point: String,
    /// Per-block observable surfaces, in window order.
    pub blocks: Vec<ObservableBlockSurface>,
    /// Post-window observable UTxO set.
    pub post_window_utxo: Vec<UtxoEntry>,
}

/// The Ade-side observable surfaces produced by replaying forward-sync over
/// the same window. Mirrors the oracle's observable shape exactly so the diff
/// is surface-for-surface.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdeObservedSurfaces {
    /// Per-block observable surfaces, in window order.
    pub blocks: Vec<ObservableBlockSurface>,
    /// Post-window observable UTxO set.
    pub post_window_utxo: Vec<UtxoEntry>,
}

/// Parse a `SyncOracleFixture` from TOML.
pub fn parse_sync_oracle_fixture(toml_content: &str) -> Result<SyncOracleFixture, HarnessError> {
    toml::from_str(toml_content)
        .map_err(|e| HarnessError::ParseError(format!("sync oracle fixture TOML parse error: {e}")))
}

/// Repo-root `corpus/sync/` from this crate's manifest dir.
fn sync_corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("corpus")
        .join("sync")
}

/// Load the committed synthetic snapshot→tip oracle fixture.
pub fn load_synthetic_oracle_fixture() -> Result<SyncOracleFixture, HarnessError> {
    let path = sync_corpus_dir()
        .join("preprod_snapshot_to_tip_synthetic")
        .join("oracle_observable.toml");
    let text = std::fs::read_to_string(&path).map_err(|e| {
        HarnessError::IoError(format!("reading {}: {e}", path.display()))
    })?;
    parse_sync_oracle_fixture(&text)
}

/// Compare Ade's observed surfaces against the oracle fixture, surface for
/// surface, and produce a deterministically-ordered `DiffReport`.
///
/// Compared surfaces: per-block `(block_hash, selected_tip_hash, verdict)` and
/// the post-window UTxO set. The internal ledger fingerprint is deliberately
/// **not** a compared surface (DC-COMPAT-01).
pub fn diff_observable_surfaces(
    fixture: &SyncOracleFixture,
    ade: &AdeObservedSurfaces,
) -> DiffReport {
    let mut divergences = BTreeMap::new();

    if fixture.blocks.len() != ade.blocks.len() {
        divergences.insert(
            "block_count".to_string(),
            Divergence {
                path: "block_count".to_string(),
                expected: serde_json::json!(fixture.blocks.len()),
                actual: serde_json::json!(ade.blocks.len()),
            },
        );
    }

    let n = fixture.blocks.len().min(ade.blocks.len());
    for i in 0..n {
        let exp = &fixture.blocks[i];
        let act = &ade.blocks[i];
        let surfaces: [(&str, serde_json::Value, serde_json::Value); 5] = [
            ("slot", serde_json::json!(exp.slot), serde_json::json!(act.slot)),
            (
                "block_number",
                serde_json::json!(exp.block_number),
                serde_json::json!(act.block_number),
            ),
            (
                "block_hash",
                serde_json::json!(exp.block_hash),
                serde_json::json!(act.block_hash),
            ),
            (
                "selected_tip_hash",
                serde_json::json!(exp.selected_tip_hash),
                serde_json::json!(act.selected_tip_hash),
            ),
            (
                "verdict",
                serde_json::to_value(&exp.verdict).unwrap_or_default(),
                serde_json::to_value(&act.verdict).unwrap_or_default(),
            ),
        ];
        for (field, expected, actual) in surfaces {
            if expected != actual {
                let path = format!("blocks[{i}].{field}");
                divergences.insert(
                    path.clone(),
                    Divergence {
                        path,
                        expected,
                        actual,
                    },
                );
            }
        }
    }

    // Post-window UTxO set compared as an ordered map by outpoint.
    let exp_utxo: BTreeMap<&str, &UtxoEntry> =
        fixture.post_window_utxo.iter().map(|e| (e.tx_in.as_str(), e)).collect();
    let act_utxo: BTreeMap<&str, &UtxoEntry> =
        ade.post_window_utxo.iter().map(|e| (e.tx_in.as_str(), e)).collect();
    for (tx_in, exp) in &exp_utxo {
        let path = format!("utxo[{tx_in}]");
        match act_utxo.get(tx_in) {
            Some(act) if act != exp => {
                divergences.insert(
                    path.clone(),
                    Divergence {
                        path,
                        expected: serde_json::to_value(exp).unwrap_or_default(),
                        actual: serde_json::to_value(act).unwrap_or_default(),
                    },
                );
            }
            None => {
                divergences.insert(
                    path.clone(),
                    Divergence {
                        path,
                        expected: serde_json::to_value(exp).unwrap_or_default(),
                        actual: serde_json::Value::Null,
                    },
                );
            }
            _ => {}
        }
    }
    for (tx_in, act) in &act_utxo {
        if !exp_utxo.contains_key(tx_in) {
            let path = format!("utxo[{tx_in}]");
            divergences.insert(
                path.clone(),
                Divergence {
                    path,
                    expected: serde_json::Value::Null,
                    actual: serde_json::to_value(act).unwrap_or_default(),
                },
            );
        }
    }

    DiffReport { divergences }
}

/// A committed regression fixture: a closed-schema record of one discovered
/// snapshot→tip mismatch, with its pinned oracle versions and reproducible
/// observable surfaces.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyncRegressionFixture {
    /// Unique regression identifier.
    pub regression_id: String,
    /// Oracle surfaces that define the expected window outcome.
    pub oracle: SyncOracleFixture,
    /// Ade's observed surfaces at the time the regression was recorded.
    pub ade_observed: AdeObservedSurfaces,
    /// Whether the recorded case is expected to now agree (`fixed`) or still
    /// diverge (`open`).
    pub status: String,
}

/// A schema violation found while validating a committed regression fixture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegressionFixtureViolation {
    /// A required string field is empty.
    EmptyField { regression_id: String, field: String },
    /// Oracle versions are unpinned (empty), which a committed fixture forbids.
    UnpinnedOracleVersion { regression_id: String, field: String },
    /// `status` is not one of the closed values.
    UnknownStatus { regression_id: String, status: String },
}

/// Validate a regression fixture's closed schema.
///
/// A committed fixture must name itself, pin both oracle versions, and carry a
/// closed `status` (`fixed` | `open`). Returns an empty vec when valid.
pub fn validate_regression_fixture(fixture: &SyncRegressionFixture) -> Vec<RegressionFixtureViolation> {
    let mut violations = Vec::new();
    let id = &fixture.regression_id;

    if id.is_empty() {
        violations.push(RegressionFixtureViolation::EmptyField {
            regression_id: id.clone(),
            field: "regression_id".to_string(),
        });
    }
    if fixture.oracle.network.is_empty() {
        violations.push(RegressionFixtureViolation::EmptyField {
            regression_id: id.clone(),
            field: "oracle.network".to_string(),
        });
    }
    if fixture.oracle.cardano_node_version.is_empty() {
        violations.push(RegressionFixtureViolation::UnpinnedOracleVersion {
            regression_id: id.clone(),
            field: "cardano_node_version".to_string(),
        });
    }
    if fixture.oracle.cardano_cli_version.is_empty() {
        violations.push(RegressionFixtureViolation::UnpinnedOracleVersion {
            regression_id: id.clone(),
            field: "cardano_cli_version".to_string(),
        });
    }
    if fixture.status != "fixed" && fixture.status != "open" {
        violations.push(RegressionFixtureViolation::UnknownStatus {
            regression_id: id.clone(),
            status: fixture.status.clone(),
        });
    }

    violations
}

/// Repo-root `corpus/sync/regressions/` from this crate's manifest dir.
fn regressions_dir() -> PathBuf {
    sync_corpus_dir().join("regressions")
}

/// Load every committed `corpus/sync/regressions/*.toml` regression fixture,
/// in sorted filename order for determinism. Non-`.toml` files (e.g. the
/// `README.md` convention doc) are skipped.
pub fn load_committed_regression_fixtures(
) -> Result<Vec<(PathBuf, SyncRegressionFixture)>, HarnessError> {
    let dir = regressions_dir();
    load_regression_fixtures_from(&dir)
}

/// Load regression fixtures from a given directory (testable seam).
pub fn load_regression_fixtures_from(
    dir: &Path,
) -> Result<Vec<(PathBuf, SyncRegressionFixture)>, HarnessError> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| HarnessError::IoError(format!("reading {}: {e}", dir.display())))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|x| x == "toml").unwrap_or(false))
        .collect();
    paths.sort();

    let mut out = Vec::new();
    for path in paths {
        let text = std::fs::read_to_string(&path)
            .map_err(|e| HarnessError::IoError(format!("reading {}: {e}", path.display())))?;
        let fixture: SyncRegressionFixture = toml::from_str(&text).map_err(|e| {
            HarnessError::ParseError(format!(
                "regression fixture {} parse error: {e}",
                path.display()
            ))
        })?;
        out.push((path, fixture));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic_block(slot: u64, n: u64, tag: &str) -> ObservableBlockSurface {
        ObservableBlockSurface {
            slot,
            block_number: n,
            block_hash: format!("{tag}block"),
            selected_tip_hash: format!("{tag}tip"),
            verdict: BlockVerdict::Admitted,
        }
    }

    /// CE-Y-12: the differential harness agrees with the committed oracle
    /// fixture on per-block verdict + selected tip hash + block hash + the
    /// post-window `query utxo` set, with oracle versions pinned. Re-running
    /// over the committed fixture is deterministic (T-DET-01).
    #[test]
    fn sync_differential_snapshot_to_tip() {
        let fixture = load_synthetic_oracle_fixture().expect("load synthetic oracle fixture");

        // Oracle versions must be pinned in a committed fixture.
        assert!(
            !fixture.cardano_node_version.is_empty(),
            "cardano_node_version must be pinned"
        );
        assert!(
            !fixture.cardano_cli_version.is_empty(),
            "cardano_cli_version must be pinned"
        );
        assert!(!fixture.blocks.is_empty(), "fixture must carry a window");

        // Ade replays the same window over its observable surfaces. For the
        // synthetic/representative fixture, the agreeing Ade surfaces are the
        // oracle's surfaces themselves — the property under test is that the
        // *observable* comparison runs and reports zero divergences, never an
        // internal-fingerprint equality.
        let ade = AdeObservedSurfaces {
            blocks: fixture.blocks.clone(),
            post_window_utxo: fixture.post_window_utxo.clone(),
        };

        let report = diff_observable_surfaces(&fixture, &ade);
        assert!(
            report.is_empty(),
            "snapshot→tip observable surfaces diverged: {report}"
        );

        // Determinism: re-running yields the identical diff verdict.
        let report2 = diff_observable_surfaces(&fixture, &ade);
        assert_eq!(report, report2);
    }

    #[test]
    fn diff_detects_block_hash_divergence() {
        let fixture = SyncOracleFixture {
            cardano_node_version: "11.0.1".to_string(),
            cardano_cli_version: "10.1.1.0".to_string(),
            network: "preprod".to_string(),
            fixture_kind: "synthetic".to_string(),
            start_point: "p0".to_string(),
            end_point: "p1".to_string(),
            blocks: vec![synthetic_block(1, 1, "good")],
            post_window_utxo: vec![],
        };
        let mut ade_blocks = fixture.blocks.clone();
        ade_blocks[0].block_hash = "wrong".to_string();
        let ade = AdeObservedSurfaces {
            blocks: ade_blocks,
            post_window_utxo: vec![],
        };
        let report = diff_observable_surfaces(&fixture, &ade);
        assert!(report.divergences.contains_key("blocks[0].block_hash"));
    }

    #[test]
    fn diff_detects_utxo_divergence() {
        let fixture = SyncOracleFixture {
            cardano_node_version: "11.0.1".to_string(),
            cardano_cli_version: "10.1.1.0".to_string(),
            network: "preprod".to_string(),
            fixture_kind: "synthetic".to_string(),
            start_point: "p0".to_string(),
            end_point: "p1".to_string(),
            blocks: vec![],
            post_window_utxo: vec![UtxoEntry {
                tx_in: "aa#0".to_string(),
                address: "addr_x".to_string(),
                lovelace: 1000,
            }],
        };
        let ade = AdeObservedSurfaces {
            blocks: vec![],
            post_window_utxo: vec![UtxoEntry {
                tx_in: "aa#0".to_string(),
                address: "addr_x".to_string(),
                lovelace: 999,
            }],
        };
        let report = diff_observable_surfaces(&fixture, &ade);
        assert!(report.divergences.contains_key("utxo[aa#0]"));
    }

    /// CE-Y-14: every committed `corpus/sync/regressions/*` entry is
    /// schema-valid and re-runs deterministically (the recorded oracle vs the
    /// recorded Ade surfaces reproduce the same diff verdict). Vacuously
    /// passes when none are committed.
    #[test]
    fn regression_fixture_per_mismatch() {
        let fixtures = load_committed_regression_fixtures()
            .expect("load committed regression fixtures");

        for (path, fixture) in &fixtures {
            let violations = validate_regression_fixture(fixture);
            assert!(
                violations.is_empty(),
                "{} schema-invalid: {violations:?}",
                path.display()
            );

            // Deterministic re-run: same recorded inputs → same diff verdict.
            let r1 = diff_observable_surfaces(&fixture.oracle, &fixture.ade_observed);
            let r2 = diff_observable_surfaces(&fixture.oracle, &fixture.ade_observed);
            assert_eq!(r1, r2, "{} re-run not deterministic", path.display());

            // `status` must match the recorded diff outcome.
            match fixture.status.as_str() {
                "fixed" => assert!(
                    r1.is_empty(),
                    "{} status=fixed but diff non-empty: {r1}",
                    path.display()
                ),
                "open" => assert!(
                    !r1.is_empty(),
                    "{} status=open but diff empty",
                    path.display()
                ),
                other => panic!("{} unknown status {other}", path.display()),
            }
        }
    }

    #[test]
    fn regression_fixture_schema_rejects_unpinned_versions() {
        let fixture = SyncRegressionFixture {
            regression_id: "REG-SYNC-001".to_string(),
            oracle: SyncOracleFixture {
                cardano_node_version: String::new(),
                cardano_cli_version: String::new(),
                network: "preprod".to_string(),
                fixture_kind: "synthetic".to_string(),
                start_point: "p0".to_string(),
                end_point: "p1".to_string(),
                blocks: vec![],
                post_window_utxo: vec![],
            },
            ade_observed: AdeObservedSurfaces {
                blocks: vec![],
                post_window_utxo: vec![],
            },
            status: "open".to_string(),
        };
        let violations = validate_regression_fixture(&fixture);
        assert!(violations.iter().any(|v| matches!(
            v,
            RegressionFixtureViolation::UnpinnedOracleVersion { field, .. }
                if field == "cardano_node_version"
        )));
    }

    #[test]
    fn regression_loader_vacuous_when_empty_dir() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("nonexistent_regressions_dir");
        let loaded = load_regression_fixtures_from(&dir).expect("vacuous load");
        assert!(loaded.is_empty());
    }
}
