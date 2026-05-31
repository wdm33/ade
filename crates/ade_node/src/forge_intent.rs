// Core Contract:
// - Deterministic: same inputs => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Pure: no I/O, no clock, no await, no key material

//! GREEN forge-intent classifier (PHASE4-N-F-F S1).
//!
//! The pure tri-state decision "may `--mode node` forge?" as a total function of
//! which operator-key CLI flags are *present* — never of their contents. It
//! reads only `Option<&Path>` presence; it opens no file, parses no key, and
//! touches no secret. Material loading into RED custody is S2; building the
//! `ForgeActivation` and the binary `Some`/`None` flip is S3.
//!
//! The closed two-variant [`ForgeIntent`] (`On`/`Off`) plus the structured
//! [`ForgeIntentError::PartialKeySet`] make the dangerous state — "forge with a
//! partial / missing / fabricated key set" — unrepresentable as a classifier
//! result: the complete set yields `On`, the empty set yields `Off`, and every
//! in-between combination fails closed. There is no wildcard arm in the
//! decision that could collapse an unenumerated presence combination into `On`
//! or `Off`; the two total outcomes are matched by explicit patterns and the
//! partial case binds the tuple by name (exhaustive without `_`).
//!
//! This lands the intent-classification half of `CN-NODE-03` (registry,
//! `declared`). It is GREEN — pure, deterministic, content-blind over path
//! values, secret-free — and lands tested-but-unwired (nothing consumes
//! `ForgeIntent` until S3).

use std::path::{Path, PathBuf};

/// The presence-validated complete operator-key path set.
///
/// Constructed ONLY after the classifier has proven all five flags present —
/// path ownership here is a *result* of classification, not evidence of key
/// validity. Carries paths only: no secrets, no file contents. No path is
/// materialized into a `ForgePaths` on the `Off` or `PartialKeySet` outcomes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForgePaths {
    pub cold: PathBuf,
    pub kes: PathBuf,
    pub vrf: PathBuf,
    pub opcert: PathBuf,
    pub genesis: PathBuf,
}

/// Closed forge intent. `On` carries the complete validated path set; `Off` is
/// the exact relay-only behavior. There is no third "partial" variant — a
/// partial key set is an error, never an intent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ForgeIntent {
    On(ForgePaths),
    Off,
}

/// Closed forge-intent classification error. Carries only static CLI flag-name
/// strings — never a supplied path string, never key material.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ForgeIntentError {
    /// Some — but not all — of the required operator-key flags were supplied.
    /// `--mode node` forges only with the complete set or with none of them.
    PartialKeySet {
        present: Vec<&'static str>,
        missing: Vec<&'static str>,
    },
}

impl std::fmt::Display for ForgeIntentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ForgeIntentError::PartialKeySet { present, missing } => write!(
                f,
                "incomplete operator key set: present [{}], missing [{}] -- \
                 --mode node forges only with the complete operator key set or \
                 with none of it (relay-only)",
                present.join(", "),
                missing.join(", "),
            ),
        }
    }
}

impl std::error::Error for ForgeIntentError {}

/// Classify forge intent from the *presence* of the five required operator-key
/// CLI flags. Pure and total over all 2^5 presence combinations:
///
/// - all five present  => `Ok(ForgeIntent::On(paths))`
/// - all five absent    => `Ok(ForgeIntent::Off)`
/// - any partial subset => `Err(ForgeIntentError::PartialKeySet { .. })`
///
/// Required set: `--cold-skey`, `--kes-skey`, `--vrf-skey`, `--opcert`,
/// `--genesis-file`. `pool_id` is not a CLI flag and not part of intent — it is
/// derived from the cold key in one named place in S3, never here.
pub fn classify_forge_intent(
    cold: Option<&Path>,
    kes: Option<&Path>,
    vrf: Option<&Path>,
    opcert: Option<&Path>,
    genesis: Option<&Path>,
) -> Result<ForgeIntent, ForgeIntentError> {
    match (cold, kes, vrf, opcert, genesis) {
        // Complete set — forge on. Path ownership taken only here, after the
        // complete-set condition is proven by the pattern itself.
        (Some(cold), Some(kes), Some(vrf), Some(opcert), Some(genesis)) => {
            Ok(ForgeIntent::On(ForgePaths {
                cold: cold.to_path_buf(),
                kes: kes.to_path_buf(),
                vrf: vrf.to_path_buf(),
                opcert: opcert.to_path_buf(),
                genesis: genesis.to_path_buf(),
            }))
        }
        // Empty set — exact relay-only behavior.
        (None, None, None, None, None) => Ok(ForgeIntent::Off),
        // Partial set — fail closed. The tuple is bound by name (not `_`), so
        // the decision surface enumerates every variable and no wildcard can
        // collapse an unenumerated combination into `On` or `Off`.
        (cold, kes, vrf, opcert, genesis) => {
            let flags: [(&'static str, bool); 5] = [
                ("--cold-skey", cold.is_some()),
                ("--kes-skey", kes.is_some()),
                ("--vrf-skey", vrf.is_some()),
                ("--opcert", opcert.is_some()),
                ("--genesis-file", genesis.is_some()),
            ];
            let present: Vec<&'static str> = flags
                .iter()
                .filter(|(_, p)| *p)
                .map(|(name, _)| *name)
                .collect();
            let missing: Vec<&'static str> = flags
                .iter()
                .filter(|(_, p)| !*p)
                .map(|(name, _)| *name)
                .collect();
            Err(ForgeIntentError::PartialKeySet { present, missing })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    #[test]
    fn classify_forge_intent_all_present_is_on() {
        let intent = classify_forge_intent(
            Some(&p("/k/cold.skey")),
            Some(&p("/k/kes.skey")),
            Some(&p("/k/vrf.skey")),
            Some(&p("/k/op.cert")),
            Some(&p("/k/genesis.json")),
        )
        .expect("complete set classifies");
        assert_eq!(
            intent,
            ForgeIntent::On(ForgePaths {
                cold: p("/k/cold.skey"),
                kes: p("/k/kes.skey"),
                vrf: p("/k/vrf.skey"),
                opcert: p("/k/op.cert"),
                genesis: p("/k/genesis.json"),
            })
        );
    }

    #[test]
    fn classify_forge_intent_none_present_is_off() {
        assert_eq!(
            classify_forge_intent(None, None, None, None, None).expect("empty set classifies"),
            ForgeIntent::Off,
        );
    }

    #[test]
    fn classify_forge_intent_total_over_all_32_flag_combinations() {
        // Iterate every presence combination of the five flags. Exactly the
        // all-present mask (0b11111) => On and the all-absent mask (0b00000) =>
        // Off; the other 30 must fail closed with PartialKeySet. This is the
        // load-bearing CE-F-1 property: partial never maps to On (or Off).
        let path = p("/x");
        let mut on = 0u32;
        let mut off = 0u32;
        let mut partial = 0u32;
        for mask in 0u8..32 {
            let bit = |i: u8| {
                if mask & (1 << i) != 0 {
                    Some(path.as_path())
                } else {
                    None
                }
            };
            let r = classify_forge_intent(bit(0), bit(1), bit(2), bit(3), bit(4));
            match (mask, r) {
                (0b11111, Ok(ForgeIntent::On(_))) => on += 1,
                (0b00000, Ok(ForgeIntent::Off)) => off += 1,
                (m, Err(ForgeIntentError::PartialKeySet { .. })) => {
                    assert!(
                        m != 0b11111 && m != 0b00000,
                        "mask {m:#07b} must not be partial"
                    );
                    partial += 1;
                }
                (m, other) => panic!("mask {m:#07b} classified unexpectedly as {other:?}"),
            }
        }
        assert_eq!(on, 1, "exactly one all-present combination");
        assert_eq!(off, 1, "exactly one all-absent combination");
        assert_eq!(partial, 30, "the other 30 combinations fail closed");
    }

    #[test]
    fn classify_forge_intent_partial_lists_present_and_missing_flags() {
        // KES + VRF present; cold, opcert, genesis missing.
        let err = classify_forge_intent(None, Some(&p("/k/kes")), Some(&p("/k/vrf")), None, None)
            .expect_err("partial set fails closed");
        assert_eq!(
            err,
            ForgeIntentError::PartialKeySet {
                present: vec!["--kes-skey", "--vrf-skey"],
                missing: vec!["--cold-skey", "--opcert", "--genesis-file"],
            }
        );
    }

    #[test]
    fn forge_intent_error_carries_no_path_bytes() {
        // A distinctive path string must never appear in the error's
        // Debug/Display — only static flag names.
        let secret_marker = "/super/secret/operator/key/path-MARKER";
        let err = classify_forge_intent(Some(&p(secret_marker)), None, None, None, None)
            .expect_err("partial set fails closed");
        let dbg = format!("{err:?}");
        let disp = format!("{err}");
        assert!(!dbg.contains("MARKER"), "Debug leaked a path: {dbg}");
        assert!(!disp.contains("MARKER"), "Display leaked a path: {disp}");
        // The flag name (not the path) is what surfaces.
        assert!(disp.contains("--cold-skey"));
    }

    #[test]
    fn classify_forge_intent_is_deterministic() {
        let args = || {
            classify_forge_intent(
                Some(&p("/k/cold")),
                None,
                Some(&p("/k/vrf")),
                None,
                Some(&p("/k/genesis")),
            )
        };
        assert_eq!(args(), args());
    }
}
