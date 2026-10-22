// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use crate::Hash28;

/// One node of a native script, in post-order (reverse Polish notation).
///
/// A composite node owns the `arity` scripts that end immediately before it. All six constructors map
/// directly to the Cardano on-chain representation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeScriptNode {
    /// Requires a signature from the given key hash.
    Sig(Hash28),
    /// Requires ALL of its `arity` sub-scripts to pass.
    All { arity: u64 },
    /// Requires ANY of its `arity` sub-scripts to pass.
    Any { arity: u64 },
    /// Requires at least `threshold` of its `arity` sub-scripts to pass.
    ///
    /// The threshold is the CDDL `int64`, held at its full signed width: a threshold of zero or below
    /// is always satisfied (cardano-ledger `isValidMOf`), and no narrowing may change a verdict.
    MOfN { threshold: i64, arity: u64 },
    /// Transaction is valid only at or after this slot.
    InvalidBefore(u64),
    /// Transaction is valid only before this slot.
    InvalidHereafter(u64),
}

impl NativeScriptNode {
    /// How many sub-scripts this node owns.
    pub fn arity(&self) -> u64 {
        match self {
            Self::All { arity } | Self::Any { arity } | Self::MOfN { arity, .. } => *arity,
            Self::Sig(_) | Self::InvalidBefore(_) | Self::InvalidHereafter(_) => 0,
        }
    }
}

/// Native script language introduced in Allegra.
///
/// Held flat: the nodes of exactly one script tree in post-order, root last. The type has no
/// recursive field, so dropping, cloning, comparing or printing a script never uses stack in
/// proportion to its nesting depth, and Cardano permits scripts thousands of levels deep
/// (DC-LEDGER-SCRIPT-02). Post-order is canonical, so two scripts are equal exactly when their node
/// sequences are.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeScript {
    nodes: Vec<NativeScriptNode>,
}

/// A node sequence that is not exactly one script tree in post-order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MalformedPostOrder;

impl NativeScript {
    /// Build a script from post-order nodes, refusing any sequence that is not exactly one tree.
    pub fn from_post_order(nodes: Vec<NativeScriptNode>) -> Result<Self, MalformedPostOrder> {
        // Scripts completed so far and not yet owned by a parent.
        let mut pending: u64 = 0;
        for node in &nodes {
            let arity = node.arity();
            if arity > pending {
                return Err(MalformedPostOrder);
            }
            pending = pending - arity + 1;
        }
        if pending == 1 {
            Ok(Self { nodes })
        } else {
            Err(MalformedPostOrder)
        }
    }

    /// The nodes in post-order, root last.
    pub fn nodes(&self) -> &[NativeScriptNode] {
        &self.nodes
    }

    /// The root node.
    pub fn root(&self) -> &NativeScriptNode {
        // Non-empty by construction: from_post_order accepts exactly one tree.
        &self.nodes[self.nodes.len() - 1]
    }

    fn leaf(node: NativeScriptNode) -> Self {
        Self { nodes: vec![node] }
    }

    fn composite(subs: Vec<NativeScript>, node: impl FnOnce(u64) -> NativeScriptNode) -> Self {
        let arity = subs.len() as u64;
        let mut nodes: Vec<NativeScriptNode> = Vec::with_capacity(subs.iter().map(|s| s.nodes.len()).sum::<usize>() + 1);
        for s in subs {
            nodes.extend(s.nodes);
        }
        nodes.push(node(arity));
        Self { nodes }
    }

    pub fn sig(key_hash: Hash28) -> Self {
        Self::leaf(NativeScriptNode::Sig(key_hash))
    }

    pub fn all(subs: Vec<NativeScript>) -> Self {
        Self::composite(subs, |arity| NativeScriptNode::All { arity })
    }

    pub fn any(subs: Vec<NativeScript>) -> Self {
        Self::composite(subs, |arity| NativeScriptNode::Any { arity })
    }

    pub fn m_of_n(threshold: i64, subs: Vec<NativeScript>) -> Self {
        Self::composite(subs, |arity| NativeScriptNode::MOfN { threshold, arity })
    }

    pub fn invalid_before(slot: u64) -> Self {
        Self::leaf(NativeScriptNode::InvalidBefore(slot))
    }

    pub fn invalid_hereafter(slot: u64) -> Self {
        Self::leaf(NativeScriptNode::InvalidHereafter(slot))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn h(b: u8) -> Hash28 {
        Hash28([b; 28])
    }

    #[test]
    fn sig_script_equality() {
        assert_eq!(NativeScript::sig(h(0xaa)), NativeScript::sig(h(0xaa)));
        assert_ne!(NativeScript::sig(h(0xaa)), NativeScript::sig(h(0xbb)));
    }

    #[test]
    fn composite_scripts_are_post_order_root_last() {
        let script = NativeScript::m_of_n(2, vec![
            NativeScript::sig(h(1)),
            NativeScript::sig(h(2)),
            NativeScript::sig(h(3)),
        ]);
        assert_eq!(script.root(), &NativeScriptNode::MOfN { threshold: 2, arity: 3 });
        assert_eq!(script.nodes().len(), 4);
        assert_eq!(script.nodes()[0], NativeScriptNode::Sig(h(1)));
    }

    #[test]
    fn nested_script_structure() {
        // All(Sig, Any(Sig, InvalidBefore))
        let nested = NativeScript::all(vec![
            NativeScript::sig(h(1)),
            NativeScript::any(vec![NativeScript::sig(h(2)), NativeScript::invalid_before(500)]),
        ]);
        assert_eq!(
            nested.nodes(),
            &[
                NativeScriptNode::Sig(h(1)),
                NativeScriptNode::Sig(h(2)),
                NativeScriptNode::InvalidBefore(500),
                NativeScriptNode::Any { arity: 2 },
                NativeScriptNode::All { arity: 2 },
            ]
        );
    }

    #[test]
    fn constructors_and_from_post_order_agree() {
        let built = NativeScript::any(vec![NativeScript::invalid_hereafter(9), NativeScript::all(vec![])]);
        let parsed = NativeScript::from_post_order(built.nodes().to_vec()).unwrap();
        assert_eq!(built, parsed);
    }

    #[test]
    fn from_post_order_accepts_exactly_one_tree() {
        assert_eq!(NativeScript::from_post_order(vec![]), Err(MalformedPostOrder));
        // two roots
        assert_eq!(
            NativeScript::from_post_order(vec![NativeScriptNode::Sig(h(1)), NativeScriptNode::Sig(h(2))]),
            Err(MalformedPostOrder)
        );
        // a parent owning more scripts than precede it
        assert_eq!(
            NativeScript::from_post_order(vec![NativeScriptNode::Sig(h(1)), NativeScriptNode::All { arity: 2 }]),
            Err(MalformedPostOrder)
        );
        assert!(NativeScript::from_post_order(vec![NativeScriptNode::All { arity: 0 }]).is_ok());
    }

    #[test]
    fn a_deep_script_drops_clones_and_compares_on_a_small_stack() {
        let r = std::thread::Builder::new()
            .stack_size(64 << 10)
            .spawn(|| {
                let mut nodes = vec![NativeScriptNode::Sig(h(7))];
                nodes.extend(std::iter::repeat(NativeScriptNode::All { arity: 1 }).take(100_000));
                let s = NativeScript::from_post_order(nodes).unwrap();
                let c = s.clone();
                let eq = c == s;
                drop(c);
                drop(s);
                eq
            })
            .unwrap()
            .join()
            .unwrap();
        assert!(r);
    }
}
