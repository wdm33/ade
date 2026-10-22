// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use std::collections::BTreeSet;
use ade_types::allegra::script::{NativeScript, NativeScriptNode};
use ade_types::Hash28;

use crate::error::NativeScriptFailure;

/// Result of evaluating a script against its environment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptVerdict {
    /// All conditions of the native script were satisfied.
    NativeScriptPassed,
    /// At least one condition of the native script was not satisfied.
    NativeScriptFailed(NativeScriptFailure),
    /// Plutus script executed successfully within the declared
    /// ex_units budget. The verdict agrees with `isValid = True`.
    PlutusPassed {
        /// CPU steps consumed.
        cpu: i64,
        /// Memory units consumed.
        mem: i64,
    },
    /// Plutus script failed during evaluation. Triggers phase-2
    /// (collateral consumed) if `isValid = True` was declared,
    /// or phase-1 if `isValid = False` was declared and this
    /// script should have failed (tx dropped).
    PlutusFailed {
        /// CPU steps attempted before the failure.
        cpu_attempted: i64,
        /// Memory units attempted before the failure.
        mem_attempted: i64,
        /// Classification of the failure cause.
        reason: PlutusFailureReason,
    },
}

/// Classification of Plutus script evaluation failure modes.
///
/// Mirrors the Haskell `FailureDescription` from `cardano-ledger`'s
/// `AlonzoUtxosPredFailure::ValidationTagMismatch`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlutusFailureReason {
    /// CEK machine returned an error term or the evaluator
    /// exhausted its budget (over-ran declared ex_units).
    ExecutionFailed,
    /// Script context / redeemer / cost-model couldn't be
    /// constructed. Mirrors `CollectErrors`.
    ContextBuildFailed,
    /// Budget consumed exceeded the declared ex_units ceiling.
    BudgetExhausted,
}

/// Deterministic classification of a transaction's script posture.
///
/// This is a classification surface, not a verdict. It describes what
/// kind of scripts a transaction contains, determined purely from the
/// parsed transaction body structure — no execution, no state lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptPosture {
    /// Transaction contains no script-related indicators.
    NoScripts,
    /// Transaction may contain native scripts but no Plutus indicators.
    /// (No `script_data_hash` in the body.)
    NonPlutusScriptsOnly,
    /// Transaction contains Plutus script indicators (`script_data_hash`
    /// present). Execution is deferred to Phase 3.
    PlutusPresentDeferred,
}

/// Evaluate a native script against available signatures and the current slot.
///
/// Deterministic: same inputs always produce the same verdict.
/// Handles all six native script constructors:
/// - Sig: checks if the key hash is in available_sigs
/// - All: all sub-scripts must pass; the first failure in order is reported
/// - Any: at least one sub-script must pass
/// - MOfN: at least M sub-scripts must pass (M <= 0 always passes)
/// - InvalidBefore: current_slot >= required_slot
/// - InvalidHereafter: current_slot < required_slot
///
/// One forward pass over the post-order nodes: each node's sub-script verdicts are the last `arity`
/// entries of a heap stack, so stack use does not grow with nesting depth (DC-LEDGER-SCRIPT-02).
/// Every sub-script is evaluated; evaluation has no side effects, so the verdict is the one a
/// short-circuiting evaluator reaches.
pub fn evaluate_native_script(
    script: &NativeScript,
    available_sigs: &BTreeSet<Hash28>,
    current_slot: u64,
) -> ScriptVerdict {
    let mut verdicts: Vec<ScriptVerdict> = Vec::new();
    for node in script.nodes() {
        // from_post_order guarantees the stack holds at least `arity` verdicts here.
        let arity = usize::try_from(node.arity()).unwrap_or(usize::MAX).min(verdicts.len());
        let subs = verdicts.split_off(verdicts.len() - arity);
        verdicts.push(evaluate_node(node, &subs, available_sigs, current_slot));
    }
    verdicts.pop().unwrap_or(ScriptVerdict::NativeScriptPassed)
}

fn evaluate_node(
    node: &NativeScriptNode,
    subs: &[ScriptVerdict],
    available_sigs: &BTreeSet<Hash28>,
    current_slot: u64,
) -> ScriptVerdict {
    match node {
        NativeScriptNode::Sig(key_hash) => {
            if available_sigs.contains(key_hash) {
                ScriptVerdict::NativeScriptPassed
            } else {
                ScriptVerdict::NativeScriptFailed(
                    NativeScriptFailure::MissingRequiredSignature {
                        key_hash: key_hash.clone(),
                    },
                )
            }
        }

        NativeScriptNode::All { .. } => {
            for verdict in subs {
                if let ScriptVerdict::NativeScriptFailed(reason) = verdict {
                    return ScriptVerdict::NativeScriptFailed(reason.clone());
                }
            }
            ScriptVerdict::NativeScriptPassed
        }

        NativeScriptNode::Any { .. } => {
            // Empty Any: no sub-script can pass → threshold not met
            if subs.contains(&ScriptVerdict::NativeScriptPassed) {
                ScriptVerdict::NativeScriptPassed
            } else {
                ScriptVerdict::NativeScriptFailed(NativeScriptFailure::ThresholdNotMet {
                    required: 1,
                    provided: 0,
                })
            }
        }

        NativeScriptNode::MOfN { threshold, .. } => {
            // cardano-ledger isValidMOf: a threshold of zero or below is satisfied outright.
            let Ok(needed) = u64::try_from(*threshold) else {
                return ScriptVerdict::NativeScriptPassed;
            };
            if needed == 0 {
                return ScriptVerdict::NativeScriptPassed;
            }
            let passed = subs
                .iter()
                .filter(|v| **v == ScriptVerdict::NativeScriptPassed)
                .count() as u64;
            if passed >= needed {
                ScriptVerdict::NativeScriptPassed
            } else {
                ScriptVerdict::NativeScriptFailed(NativeScriptFailure::ThresholdNotMet {
                    required: *threshold,
                    provided: passed,
                })
            }
        }

        NativeScriptNode::InvalidBefore(required_slot) => {
            if current_slot >= *required_slot {
                ScriptVerdict::NativeScriptPassed
            } else {
                ScriptVerdict::NativeScriptFailed(
                    NativeScriptFailure::TimelockNotSatisfied {
                        required_slot: ade_types::SlotNo(*required_slot),
                        current_slot: ade_types::SlotNo(current_slot),
                    },
                )
            }
        }

        NativeScriptNode::InvalidHereafter(required_slot) => {
            if current_slot < *required_slot {
                ScriptVerdict::NativeScriptPassed
            } else {
                ScriptVerdict::NativeScriptFailed(
                    NativeScriptFailure::TimelockNotSatisfied {
                        required_slot: ade_types::SlotNo(*required_slot),
                        current_slot: ade_types::SlotNo(current_slot),
                    },
                )
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn sigs(hashes: &[[u8; 28]]) -> BTreeSet<Hash28> {
        hashes.iter().map(|h| Hash28(*h)).collect()
    }

    // -----------------------------------------------------------------------
    // Sig tests
    // -----------------------------------------------------------------------

    #[test]
    fn sig_passes_when_key_present() {
        let key = Hash28([0x01; 28]);
        let available = sigs(&[[0x01; 28]]);
        let verdict = evaluate_native_script(&NativeScript::sig(key), &available, 0);
        assert_eq!(verdict, ScriptVerdict::NativeScriptPassed);
    }

    #[test]
    fn sig_fails_when_key_absent() {
        let key = Hash28([0x01; 28]);
        let available = sigs(&[[0x02; 28]]);
        let verdict = evaluate_native_script(&NativeScript::sig(key.clone()), &available, 0);
        assert_eq!(
            verdict,
            ScriptVerdict::NativeScriptFailed(NativeScriptFailure::MissingRequiredSignature {
                key_hash: key,
            })
        );
    }

    // -----------------------------------------------------------------------
    // All tests
    // -----------------------------------------------------------------------

    #[test]
    fn all_passes_when_all_subs_pass() {
        let script = NativeScript::all(vec![
            NativeScript::sig(Hash28([0x01; 28])),
            NativeScript::sig(Hash28([0x02; 28])),
        ]);
        let available = sigs(&[[0x01; 28], [0x02; 28]]);
        let verdict = evaluate_native_script(&script, &available, 0);
        assert_eq!(verdict, ScriptVerdict::NativeScriptPassed);
    }

    #[test]
    fn all_fails_when_one_sub_fails() {
        let script = NativeScript::all(vec![
            NativeScript::sig(Hash28([0x01; 28])),
            NativeScript::sig(Hash28([0x03; 28])),
        ]);
        let available = sigs(&[[0x01; 28]]);
        let verdict = evaluate_native_script(&script, &available, 0);
        match verdict {
            ScriptVerdict::NativeScriptFailed(_) => {}
            _ => unreachable!(),
        }
    }

    #[test]
    fn all_empty_passes() {
        let script = NativeScript::all(vec![]);
        let available = BTreeSet::new();
        let verdict = evaluate_native_script(&script, &available, 0);
        assert_eq!(verdict, ScriptVerdict::NativeScriptPassed);
    }

    // -----------------------------------------------------------------------
    // Any tests
    // -----------------------------------------------------------------------

    #[test]
    fn any_passes_when_one_sub_passes() {
        let script = NativeScript::any(vec![
            NativeScript::sig(Hash28([0x01; 28])),
            NativeScript::sig(Hash28([0x02; 28])),
        ]);
        let available = sigs(&[[0x02; 28]]);
        let verdict = evaluate_native_script(&script, &available, 0);
        assert_eq!(verdict, ScriptVerdict::NativeScriptPassed);
    }

    #[test]
    fn any_fails_when_none_pass() {
        let script = NativeScript::any(vec![
            NativeScript::sig(Hash28([0x01; 28])),
            NativeScript::sig(Hash28([0x02; 28])),
        ]);
        let available = sigs(&[[0x03; 28]]);
        let verdict = evaluate_native_script(&script, &available, 0);
        assert_eq!(
            verdict,
            ScriptVerdict::NativeScriptFailed(NativeScriptFailure::ThresholdNotMet {
                required: 1,
                provided: 0,
            })
        );
    }

    #[test]
    fn any_empty_fails() {
        let script = NativeScript::any(vec![]);
        let available = BTreeSet::new();
        let verdict = evaluate_native_script(&script, &available, 0);
        assert_eq!(
            verdict,
            ScriptVerdict::NativeScriptFailed(NativeScriptFailure::ThresholdNotMet {
                required: 1,
                provided: 0,
            })
        );
    }

    // -----------------------------------------------------------------------
    // MOfN tests
    // -----------------------------------------------------------------------

    #[test]
    fn m_of_n_passes_when_threshold_met() {
        let script = NativeScript::m_of_n(2, vec![
            NativeScript::sig(Hash28([0x01; 28])),
            NativeScript::sig(Hash28([0x02; 28])),
            NativeScript::sig(Hash28([0x03; 28])),
        ]);
        let available = sigs(&[[0x01; 28], [0x03; 28]]);
        let verdict = evaluate_native_script(&script, &available, 0);
        assert_eq!(verdict, ScriptVerdict::NativeScriptPassed);
    }

    #[test]
    fn m_of_n_fails_when_threshold_not_met() {
        let script = NativeScript::m_of_n(2, vec![
            NativeScript::sig(Hash28([0x01; 28])),
            NativeScript::sig(Hash28([0x02; 28])),
            NativeScript::sig(Hash28([0x03; 28])),
        ]);
        let available = sigs(&[[0x01; 28]]);
        let verdict = evaluate_native_script(&script, &available, 0);
        assert_eq!(
            verdict,
            ScriptVerdict::NativeScriptFailed(NativeScriptFailure::ThresholdNotMet {
                required: 2,
                provided: 1,
            })
        );
    }

    #[test]
    fn m_of_n_zero_threshold_always_passes() {
        let script = NativeScript::m_of_n(0, vec![
            NativeScript::sig(Hash28([0x01; 28])),
        ]);
        let available = BTreeSet::new();
        let verdict = evaluate_native_script(&script, &available, 0);
        assert_eq!(verdict, ScriptVerdict::NativeScriptPassed);
    }

    // -----------------------------------------------------------------------
    // Timelock tests
    // -----------------------------------------------------------------------

    #[test]
    fn invalid_before_passes_when_at_slot() {
        let script = NativeScript::invalid_before(100);
        let verdict = evaluate_native_script(&script, &BTreeSet::new(), 100);
        assert_eq!(verdict, ScriptVerdict::NativeScriptPassed);
    }

    #[test]
    fn invalid_before_passes_when_after_slot() {
        let script = NativeScript::invalid_before(100);
        let verdict = evaluate_native_script(&script, &BTreeSet::new(), 200);
        assert_eq!(verdict, ScriptVerdict::NativeScriptPassed);
    }

    #[test]
    fn invalid_before_fails_when_before_slot() {
        let script = NativeScript::invalid_before(100);
        let verdict = evaluate_native_script(&script, &BTreeSet::new(), 50);
        match verdict {
            ScriptVerdict::NativeScriptFailed(NativeScriptFailure::TimelockNotSatisfied {
                required_slot,
                current_slot,
            }) => {
                assert_eq!(required_slot, ade_types::SlotNo(100));
                assert_eq!(current_slot, ade_types::SlotNo(50));
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn invalid_hereafter_passes_when_before_slot() {
        let script = NativeScript::invalid_hereafter(200);
        let verdict = evaluate_native_script(&script, &BTreeSet::new(), 100);
        assert_eq!(verdict, ScriptVerdict::NativeScriptPassed);
    }

    #[test]
    fn invalid_hereafter_fails_when_at_slot() {
        let script = NativeScript::invalid_hereafter(200);
        let verdict = evaluate_native_script(&script, &BTreeSet::new(), 200);
        match verdict {
            ScriptVerdict::NativeScriptFailed(NativeScriptFailure::TimelockNotSatisfied { .. }) => {}
            _ => unreachable!(),
        }
    }

    #[test]
    fn invalid_hereafter_fails_when_after_slot() {
        let script = NativeScript::invalid_hereafter(200);
        let verdict = evaluate_native_script(&script, &BTreeSet::new(), 300);
        match verdict {
            ScriptVerdict::NativeScriptFailed(NativeScriptFailure::TimelockNotSatisfied { .. }) => {}
            _ => unreachable!(),
        }
    }

    // -----------------------------------------------------------------------
    // Composed / nested tests
    // -----------------------------------------------------------------------

    #[test]
    fn time_window_script() {
        // Valid only in slot range [100, 200)
        let script = NativeScript::all(vec![
            NativeScript::invalid_before(100),
            NativeScript::invalid_hereafter(200),
        ]);

        // Too early
        let v = evaluate_native_script(&script, &BTreeSet::new(), 50);
        assert!(matches!(v, ScriptVerdict::NativeScriptFailed(_)));

        // In range
        let v = evaluate_native_script(&script, &BTreeSet::new(), 150);
        assert_eq!(v, ScriptVerdict::NativeScriptPassed);

        // Too late
        let v = evaluate_native_script(&script, &BTreeSet::new(), 250);
        assert!(matches!(v, ScriptVerdict::NativeScriptFailed(_)));
    }

    #[test]
    fn sig_plus_timelock() {
        // Requires signature AND slot >= 50
        let script = NativeScript::all(vec![
            NativeScript::sig(Hash28([0xaa; 28])),
            NativeScript::invalid_before(50),
        ]);

        let available = sigs(&[[0xaa; 28]]);

        // Has sig but too early
        let v = evaluate_native_script(&script, &available, 10);
        assert!(matches!(v, ScriptVerdict::NativeScriptFailed(_)));

        // Has sig and at correct slot
        let v = evaluate_native_script(&script, &available, 50);
        assert_eq!(v, ScriptVerdict::NativeScriptPassed);

        // Correct slot but missing sig
        let empty = BTreeSet::new();
        let v = evaluate_native_script(&script, &empty, 50);
        assert!(matches!(v, ScriptVerdict::NativeScriptFailed(_)));
    }

    #[test]
    fn determinism_same_inputs_same_output() {
        let script = NativeScript::m_of_n(1, vec![
            NativeScript::sig(Hash28([0x01; 28])),
            NativeScript::invalid_before(10),
        ]);
        let available = sigs(&[[0x01; 28]]);

        let v1 = evaluate_native_script(&script, &available, 5);
        let v2 = evaluate_native_script(&script, &available, 5);
        assert_eq!(v1, v2);
    }

}
