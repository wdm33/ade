#!/usr/bin/env bash
set -uo pipefail

# MEM-OPT-UTXO-DISK -- ade_mem_diag quarantine enforcement.
#
# The S0 diagnostic needs one unsafe FFI call (mimalloc mi_collect) for its
# forced-reclaim probe. That unsafe is QUARANTINED in the tiny RED `ade_mem_diag`
# crate so that `ade_node` (the node authority/binary crate) keeps
# `#![deny(unsafe_code)]` with ZERO local exceptions. This gate mechanically
# enforces the quarantine:
#   (1) ade_node keeps #![deny(unsafe_code)] ...
#   (2) ... with ZERO local allow(unsafe_code) (the compiler then rejects any
#       unsafe block in ade_node, so unsafe cannot exist there).
#   (3) ade_mem_diag is depended on ONLY by ade_node -- never by a BLUE crate.
#   (4) the forced collect is reachable only behind the S0 env toggle.
#
# No --self-test branch: this gate IS a static assertion over the live tree.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"
FAILED=0
fail() { echo "FAIL: $1"; FAILED=1; }

# (1) ade_node keeps #![deny(unsafe_code)].
grep -qE '#!\[deny\(unsafe_code\)\]' crates/ade_node/src/lib.rs \
    || fail "ade_node/src/lib.rs no longer carries #![deny(unsafe_code)]"

# (2) ZERO local allow(unsafe_code) anywhere in ade_node (lib + bin + tests).
if grep -rnE 'allow\(\s*unsafe_code\s*\)' crates/ade_node/ 2>/dev/null; then
    fail "ade_node has a local allow(unsafe_code) -- the zero-exceptions invariant is broken; quarantine the unsafe in ade_mem_diag instead"
fi

# (3) ade_mem_diag is a dependency of ONLY ade_node (no BLUE crate, no leak).
#     Search every crate manifest EXCEPT ade_mem_diag's own for the dep token.
mapfile -t DEPENDERS < <(grep -rlE '^\s*ade_mem_diag\s*=' crates/*/Cargo.toml 2>/dev/null | grep -v 'crates/ade_mem_diag/Cargo.toml' || true)
for m in "${DEPENDERS[@]}"; do
    [[ "$m" == "crates/ade_node/Cargo.toml" ]] \
        || fail "ade_mem_diag is depended on by $m -- only ade_node (RED binary) may depend on the diagnostic crate"
done
if [[ ${#DEPENDERS[@]} -eq 0 ]]; then
    fail "no crate depends on ade_mem_diag -- expected exactly ade_node"
fi

# (4) the forced collect is reachable only behind the S0 env toggle. The single
#     caller of force_allocator_collect_for_diagnostic_only in ade_node must be
#     co-located with the ADE_MEM_PHASE_DIAGNOSTIC guard (both in bootstrap.rs).
# Match the CALL form (open paren), not doc-comment mentions of the fn name.
CALLERS=$(grep -rlE 'force_allocator_collect_for_diagnostic_only\(' crates/ade_node/src/ 2>/dev/null || true)
if [[ "$CALLERS" != "crates/ade_node/src/admission/bootstrap.rs" ]]; then
    fail "the diagnostic collect is called from '$CALLERS' -- expected exactly crates/ade_node/src/admission/bootstrap.rs"
fi
grep -qE 'ADE_MEM_PHASE_DIAGNOSTIC' crates/ade_node/src/admission/bootstrap.rs \
    || fail "bootstrap.rs calls the diagnostic collect but has no ADE_MEM_PHASE_DIAGNOSTIC env guard"

# (5) ade_mem_diag itself does NOT deny unsafe (it is the quarantine) and is the
#     sole home of the FFI. Sanity: it references mi_collect.
grep -qE 'mi_collect' crates/ade_mem_diag/src/lib.rs \
    || fail "ade_mem_diag no longer contains the mi_collect FFI -- the quarantine is empty"

if (( FAILED == 0 )); then
    echo "OK: ade_mem_diag quarantine (ade_node #![deny(unsafe_code)] + zero allows; ade_mem_diag dep'd only by ade_node; collect gated by ADE_MEM_PHASE_DIAGNOSTIC)"
fi
exit $FAILED
