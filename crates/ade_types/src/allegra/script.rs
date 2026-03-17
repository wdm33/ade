// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use crate::Hash28;

/// Native script language introduced in Allegra.
///
/// All six constructors map directly to the Cardano on-chain representation.
/// Recursive structure allows composing complex time-lock and multi-sig policies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeScript {
    /// Requires a signature from the given key hash.
    Sig(Hash28),
    /// Requires ALL sub-scripts to pass.
    All(Vec<NativeScript>),
    /// Requires ANY sub-script to pass.
    Any(Vec<NativeScript>),
    /// Requires at least M of N sub-scripts to pass.
    MOfN(u32, Vec<NativeScript>),
    /// Transaction is valid only at or after this slot.
    InvalidBefore(u64),
    /// Transaction is valid only before this slot.
    InvalidHereafter(u64),
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn sig_script_equality() {
        let a = NativeScript::Sig(Hash28([0xaa; 28]));
        let b = NativeScript::Sig(Hash28([0xaa; 28]));
        let c = NativeScript::Sig(Hash28([0xbb; 28]));
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn all_script_construction() {
        let script = NativeScript::All(vec![
            NativeScript::Sig(Hash28([0x01; 28])),
            NativeScript::Sig(Hash28([0x02; 28])),
        ]);
        match &script {
            NativeScript::All(subs) => assert_eq!(subs.len(), 2),
            _ => unreachable!(),
        }
    }

    #[test]
    fn any_script_construction() {
        let script = NativeScript::Any(vec![
            NativeScript::Sig(Hash28([0x01; 28])),
        ]);
        match &script {
            NativeScript::Any(subs) => assert_eq!(subs.len(), 1),
            _ => unreachable!(),
        }
    }

    #[test]
    fn m_of_n_script_construction() {
        let script = NativeScript::MOfN(2, vec![
            NativeScript::Sig(Hash28([0x01; 28])),
            NativeScript::Sig(Hash28([0x02; 28])),
            NativeScript::Sig(Hash28([0x03; 28])),
        ]);
        match &script {
            NativeScript::MOfN(m, subs) => {
                assert_eq!(*m, 2);
                assert_eq!(subs.len(), 3);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn timelock_scripts() {
        let before = NativeScript::InvalidBefore(100);
        let hereafter = NativeScript::InvalidHereafter(200);
        assert_ne!(before, hereafter);

        match before {
            NativeScript::InvalidBefore(slot) => assert_eq!(slot, 100),
            _ => unreachable!(),
        }
        match hereafter {
            NativeScript::InvalidHereafter(slot) => assert_eq!(slot, 200),
            _ => unreachable!(),
        }
    }

    #[test]
    fn nested_script_structure() {
        // All(Sig, Any(Sig, InvalidBefore))
        let nested = NativeScript::All(vec![
            NativeScript::Sig(Hash28([0x01; 28])),
            NativeScript::Any(vec![
                NativeScript::Sig(Hash28([0x02; 28])),
                NativeScript::InvalidBefore(500),
            ]),
        ]);
        match &nested {
            NativeScript::All(subs) => {
                assert_eq!(subs.len(), 2);
                match &subs[1] {
                    NativeScript::Any(inner) => assert_eq!(inner.len(), 2),
                    _ => unreachable!(),
                }
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn clone_preserves_equality() {
        let script = NativeScript::MOfN(1, vec![
            NativeScript::Sig(Hash28([0xff; 28])),
            NativeScript::InvalidHereafter(999),
        ]);
        let cloned = script.clone();
        assert_eq!(script, cloned);
    }
}
