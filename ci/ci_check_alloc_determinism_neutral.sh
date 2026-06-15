#!/usr/bin/env bash
set -uo pipefail

# DC-MEM-06 — the process global allocator is determinism-neutral. Two
# mechanical facts over the source tree (MEM-OPT-OPS S1, OP-MEM-02):
#
#   1. Exactly ONE `#[global_allocator]` in crates/**, and it is the RED
#      ade_node binary entry (crates/ade_node/src/main.rs). The allocator is a
#      process-runtime byte-provider: it is fixed at build (never selected
#      per-run) and never lives in an authoritative crate.
#
#   2. ZERO allocator references in any BLUE crate (source OR manifest). Every
#      canonical encoder and every fingerprint lives in BLUE; if the allocator
#      type is invisible there, allocation addresses/sizes cannot leak into a
#      fingerprint. This is the load-bearing "allocator is determinism-neutral"
#      assertion — a representation/runtime change, never a consensus change.
#
# Always strict (a source-tree gate, not an evidence gate). Run `--self-test`
# to validate the gate against temp fixtures.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

# Allocator-type tokens forbidden in BLUE; `global_allocator` also pins check 1.
ALLOC_RE='mimalloc|MiMalloc|jemalloc|Jemalloc|tikv.?jemalloc|global_allocator|GlobalAlloc'
# BLUE scan targets (.idd-config.json core_paths), relative to crates/. The six
# pure authoritative crates, PLUS the BLUE ade_network submodules. ade_network is
# mixed-color (mux::transport / session are RED), so only its BLUE submodules are
# scanned — never the whole crate.
BLUE_SCAN=(
    ade_ledger ade_codec ade_types ade_crypto ade_plutus ade_core
    ade_network/src/mux/frame.rs
    ade_network/src/codec
    ade_network/src/handshake
    ade_network/src/chain_sync
    ade_network/src/block_fetch
    ade_network/src/tx_submission
    ade_network/src/keep_alive
    ade_network/src/peer_sharing
    ade_network/src/n2c
)
EXPECTED_ALLOC_SITE="crates/ade_node/src/main.rs"

# run_checks <root> : print FAIL lines, return 0 if all pass else 1.
run_checks() {
    local root="$1"
    local failed=0

    # 1. Exactly one #[global_allocator], at the RED binary entry.
    local locs n
    locs="$(grep -rlE '#\[global_allocator\]' "$root/crates" --include='*.rs' 2>/dev/null \
            | sed "s#^$root/##" | sort)"
    n="$(printf '%s' "$locs" | grep -c . || true)"
    if [[ "$n" -ne 1 ]]; then
        echo "FAIL: expected exactly one #[global_allocator] in crates/**, found $n:"
        [[ -n "$locs" ]] && printf '%s\n' "$locs" | sed 's/^/    /'
        failed=1
    elif [[ "$locs" != "$EXPECTED_ALLOC_SITE" ]]; then
        echo "FAIL: #[global_allocator] must live in $EXPECTED_ALLOC_SITE (the RED binary entry), found: $locs"
        failed=1
    fi

    # 2. Zero allocator references in any BLUE scan target (source or manifest).
    local target path hits
    for target in "${BLUE_SCAN[@]}"; do
        path="$root/crates/$target"
        [[ -e "$path" ]] || continue
        hits="$(grep -rnE "$ALLOC_RE" "$path" --include='*.rs' --include='Cargo.toml' 2>/dev/null \
                | sed "s#^$root/##")"
        if [[ -n "$hits" ]]; then
            echo "FAIL: allocator reference in BLUE path '$target' (DC-MEM-06: the allocator type must be invisible to authoritative code):"
            printf '%s\n' "$hits" | sed 's/^/    /'
            failed=1
        fi
    done

    return "$failed"
}

self_test() {
    local st_failed=0 tmp
    tmp="$(mktemp -d)"
    trap 'rm -rf "$tmp"' RETURN

    # GOOD: one allocator at the binary entry; BLUE crates clean.
    mkdir -p "$tmp/good/crates/ade_node/src" "$tmp/good/crates/ade_ledger/src"
    printf '#[global_allocator]\nstatic GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;\n' \
        > "$tmp/good/crates/ade_node/src/main.rs"
    printf 'pub fn f() {}\n' > "$tmp/good/crates/ade_ledger/src/lib.rs"
    if run_checks "$tmp/good" >/dev/null; then
        echo "self-test: GOOD fixture accepted [ok]"
    else
        echo "self-test: GOOD fixture should pass but FAILED"; st_failed=1
    fi

    # BAD-1: allocator ref leaked into a BLUE crate.
    mkdir -p "$tmp/bad1/crates/ade_node/src" "$tmp/bad1/crates/ade_ledger/src"
    printf '#[global_allocator]\nstatic G: mimalloc::MiMalloc = mimalloc::MiMalloc;\n' \
        > "$tmp/bad1/crates/ade_node/src/main.rs"
    printf 'use mimalloc::MiMalloc;\n' > "$tmp/bad1/crates/ade_ledger/src/lib.rs"
    if run_checks "$tmp/bad1" >/dev/null; then
        echo "self-test: BAD-1 (BLUE allocator ref) should FAIL but passed"; st_failed=1
    else
        echo "self-test: BAD-1 (BLUE allocator ref) rejected [ok]"
    fi

    # BAD-2: #[global_allocator] in the wrong (BLUE) crate.
    mkdir -p "$tmp/bad2/crates/ade_node/src" "$tmp/bad2/crates/ade_core/src"
    printf 'fn main() {}\n' > "$tmp/bad2/crates/ade_node/src/main.rs"
    printf '#[global_allocator]\nstatic G: X = X;\n' > "$tmp/bad2/crates/ade_core/src/lib.rs"
    if run_checks "$tmp/bad2" >/dev/null; then
        echo "self-test: BAD-2 (global_allocator in BLUE) should FAIL but passed"; st_failed=1
    else
        echo "self-test: BAD-2 (global_allocator in BLUE) rejected [ok]"
    fi

    # BAD-3: two #[global_allocator] sites.
    mkdir -p "$tmp/bad3/crates/ade_node/src"
    printf '#[global_allocator]\nstatic G: X = X;\n' > "$tmp/bad3/crates/ade_node/src/main.rs"
    printf '#[global_allocator]\nstatic H: Y = Y;\n' > "$tmp/bad3/crates/ade_node/src/lib.rs"
    if run_checks "$tmp/bad3" >/dev/null; then
        echo "self-test: BAD-3 (two global_allocator) should FAIL but passed"; st_failed=1
    else
        echo "self-test: BAD-3 (two global_allocator) rejected [ok]"
    fi

    if [[ "$st_failed" -eq 0 ]]; then
        echo "OK: --self-test — gate accepts the good fixture and rejects BLUE-leak / wrong-site / double-allocator."
        return 0
    fi
    return 1
}

MODE="${1:-}"
if [[ "$MODE" == "--self-test" ]]; then
    self_test
    exit $?
fi

if run_checks "$REPO_ROOT"; then
    echo "OK: allocator determinism-neutral — exactly one #[global_allocator] at $EXPECTED_ALLOC_SITE; zero allocator refs across ${#BLUE_SCAN[@]} BLUE scan targets (6 pure crates + BLUE ade_network submodules)."
    exit 0
fi
exit 1
