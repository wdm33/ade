#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-F-C L6 — BA-02 evidence honesty gate (candidate RO-LIVE-06).
#
# A BA02Manifest must be constructible ONLY from correlate's exact
# forged-hash <-> peer-accept match. No weaker signal (Ade self-accept,
# ForgeSucceeded, block_received, a lagging/diverged agreement verdict)
# may be treated as acceptance, and no synthetic-built manifest may be
# committed as evidence. This gate makes that mechanical.
#
# Scope: the evidence module `crates/ade_node/src/ba02_evidence.rs`,
# production code only (the `#[cfg(test)]` module + line comments — incl.
# the `//!`/`///` doc comments that legitimately NAME the forbidden
# signals while explaining why they are excluded — are stripped first).
#
# Guards:
#   (pos)  the module defines both peer-accept signal forms
#          (PeerServedBlock + PeerChainTip) and a `correlate` fn;
#   (g1)   the `Ba02Manifest { ... }` struct-literal constructor appears
#          EXACTLY ONCE in production (excluding the `pub struct`
#          definition), and that one site is inside `correlate` — so the
#          manifest has a single constructor on the exact-match path;
#   (g2)   no self-evidence token (ForgeSucceeded / self_accept /
#          block_received / agreement_verdict / "agreed") appears in
#          production code as an acceptance source;
#   (g3)   no committed `docs/evidence/*ba02*` manifest exists — a real
#          manifest requires a real operator-captured peer log; L6 commits
#          none (synthetic fixtures live in test code only).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MOD="$REPO_ROOT/crates/ade_node/src/ba02_evidence.rs"
EVIDENCE_DIR="$REPO_ROOT/docs/evidence"

FAILED=0
print_fail() { echo "FAIL (ba02 evidence): $1"; FAILED=1; }

if [[ ! -f "$MOD" ]]; then
    echo "FAIL (ba02 evidence): module not found at $MOD"
    echo "FAIL: ci_check_ba02_evidence_closed"
    exit 1
fi

# Strip the `#[cfg(test)]` module (attribute to EOF) + line comments
# (incl. //! and /// doc comments).
strip_for_grep() {
    awk '
        /^#\[cfg\(test\)\]/ { in_test=1 }
        in_test { next }
        { line=$0; sub(/\/\/.*$/, "", line); print line }
    ' "$1"
}

# Isolate the `correlate` fn body: from `pub fn correlate` to the next
# top-level `^}`.
isolate_correlate() {
    strip_for_grep "$1" | awk '
        /pub fn correlate/ { capture=1 }
        capture { print }
        capture && /^}/ { exit }
    '
}

PROD="$(strip_for_grep "$MOD")"
CORR="$(isolate_correlate "$MOD")"

# --- guard (pos): both signal forms + correlate present ----------------------
for tok in 'PeerServedBlock' 'PeerChainTip' 'pub fn correlate'; do
    if ! echo "$PROD" | grep -qE "$tok"; then
        print_fail "ba02_evidence.rs production must define/contain $tok"
    fi
done

# --- guard (g1): single Ba02Manifest constructor, inside correlate -----------
# Count struct-literal constructions (`Ba02Manifest {`) excluding the
# `pub struct Ba02Manifest {` definition AND the `impl Ba02Manifest {`
# block header (neither is a construction). A real construction is the
# `Ba02Manifest {` field-init form, which is neither `struct` nor `impl`.
ctor_count="$(echo "$PROD" | grep -E 'Ba02Manifest[[:space:]]*\{' | grep -vcE 'struct Ba02Manifest|impl Ba02Manifest')"
if [[ "$ctor_count" != "1" ]]; then
    print_fail "expected exactly 1 Ba02Manifest{...} constructor in production (excluding the struct def), found $ctor_count — the manifest must have a single constructor on the exact-match path"
fi
if [[ -z "$CORR" ]]; then
    print_fail "could not isolate the correlate fn body (signature moved/renamed?)"
elif ! echo "$CORR" | grep -qE 'Ba02Manifest[[:space:]]*\{'; then
    print_fail "the Ba02Manifest constructor is not inside correlate — the exact-match arm must be its sole construction site"
fi

# --- guard (g2): no self-evidence token as an acceptance source -------------
# These may appear ONLY in doc/line comments (stripped above) or test code
# (stripped). Their presence in production code would mean a weaker signal
# is being treated as acceptance.
for tok in 'ForgeSucceeded' 'self_accept' 'block_received' 'agreement_verdict' '"agreed"'; do
    if echo "$PROD" | grep -qE "$tok"; then
        print_fail "ba02_evidence.rs production references a self-evidence token: $tok — a weaker signal must never be a peer-accept source (BA-02 honesty)"
    fi
done

# --- guard (g3): no committed synthetic ba02 manifest -----------------------
# A real BA-02 manifest requires a real operator-captured peer log. L6
# commits none; synthetic fixtures live in test code only.
if [[ -d "$EVIDENCE_DIR" ]]; then
    if find "$EVIDENCE_DIR" -type f -iname '*ba02*' 2>/dev/null | grep -q .; then
        print_fail "a docs/evidence ba02 manifest exists — L6 commits no manifest (a real BA-02 manifest requires a real operator-captured peer log; synthetic logs cannot satisfy BA-02)"
    fi
fi

if (( FAILED == 0 )); then
    echo "OK (ba02 evidence): single Ba02Manifest constructor inside correlate; no self-evidence token as an acceptance source; no synthetic committed manifest (BA-02 stays operator-gated)"
fi
exit $FAILED
