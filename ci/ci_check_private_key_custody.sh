#!/usr/bin/env bash
set -euo pipefail

# PHASE4-N-C S1 — private-key custody gate.
#
# Mechanical guards (closure proof for CE-N-C-1):
#   1. No private-key types defined outside crates/ade_runtime/src/producer/.
#   2. No cardano_crypto VRF prove / KES sign_kes / KES update_kes call
#      outside crates/ade_runtime/src/producer/ — inside production code.
#      (test modules are excluded; verify paths self-consistency-test by
#      calling sign_kes / prove inside `#[cfg(test)] mod tests`.)
#   3. No re-exports of kes_sign / vrf_prove / kes_update from BLUE crates.
#   4. No `pub fn` in producer/signing.rs returns raw [u8; N] / Vec<u8>.
#   5. Debug impl for VrfSigningKey / KesSecret / ColdSigningKey is hand-rolled
#      (not derived).
#   6. No prove(/sign_kes(/update_kes( inside crates/ade_testkit/src/producer/
#      except in reference_vectors.rs (whitelisted: one-shot vector
#      materialization, not a production signing path).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

BLUE_CRATE_SRC=(
    "$REPO_ROOT/crates/ade_core/src"
    "$REPO_ROOT/crates/ade_codec/src"
    "$REPO_ROOT/crates/ade_types/src"
    "$REPO_ROOT/crates/ade_ledger/src"
    "$REPO_ROOT/crates/ade_crypto/src"
)

PRODUCER_DIR="$REPO_ROOT/crates/ade_runtime/src/producer"
SIGNING_RS="$PRODUCER_DIR/signing.rs"
TESTKIT_PRODUCER_DIR="$REPO_ROOT/crates/ade_testkit/src/producer"
TESTKIT_REF_VECTORS="$TESTKIT_PRODUCER_DIR/reference_vectors.rs"

FAILED=0

print_fail() {
    echo "FAIL: $1"
    FAILED=1
}

# Emit only non-test lines for a given .rs file: lines from start of
# file up to (but not including) the first `#[cfg(test)]`. Inline tests
# annotated with bare `#[test]` are not the concern of this gate; the
# pattern of interest is the surrounding `#[cfg(test)] mod tests` block
# used throughout the workspace.
emit_production_lines() {
    local f="$1"
    awk '/^#\[cfg\(test\)\]/ { exit } { print NR ":" $0 }' "$f"
}

# ---------------------------------------------------------------------------
# Guard 1 — No private-key types defined outside RED.
# Allowed BLUE: `*VerificationKey` types and signed-artifact wrappers.
# ---------------------------------------------------------------------------
GUARD1_PATTERNS=(
    'pub struct [A-Za-z_]*SigningKey'
    'pub struct KesSecret'
    'struct ColdSigningKey'
)

for pattern in "${GUARD1_PATTERNS[@]}"; do
    for src in "${BLUE_CRATE_SRC[@]}"; do
        if [ -d "$src" ]; then
            while IFS= read -r -d '' rs; do
                matches=$(emit_production_lines "$rs" | grep -E "$pattern" || true)
                if [ -n "$matches" ]; then
                    print_fail "Guard 1 (private-key type defined in BLUE): $pattern"
                    echo "$rs:"
                    echo "$matches"
                fi
            done < <(find "$src" -name '*.rs' -print0)
        fi
    done
done

# ---------------------------------------------------------------------------
# Guard 2 — No cardano-crypto signing API calls outside producer/.
# Test self-consistency lives in `#[cfg(test)] mod tests`; the
# production-only filter above excludes those.
# ---------------------------------------------------------------------------
GUARD2_PATTERNS=(
    'VrfDraft03::prove'
    'Sum6Kes::sign_kes'
    'Sum6Kes::update_kes'
    'KesAlgorithm::sign_kes'
    'KesAlgorithm::update_kes'
)

for pattern in "${GUARD2_PATTERNS[@]}"; do
    for src in "${BLUE_CRATE_SRC[@]}"; do
        if [ -d "$src" ]; then
            while IFS= read -r -d '' rs; do
                matches=$(emit_production_lines "$rs" | grep -E "$pattern" || true)
                if [ -n "$matches" ]; then
                    print_fail "Guard 2 (cardano-crypto signing API call in BLUE production): $pattern"
                    echo "$rs:"
                    echo "$matches"
                fi
            done < <(find "$src" -name '*.rs' -print0)
        fi
    done
done

# ---------------------------------------------------------------------------
# Guard 3 — No re-exports of producer signing primitives from BLUE crates.
# ---------------------------------------------------------------------------
GUARD3_PATTERN='pub use [^;]*(kes_sign|vrf_prove|kes_update)'

for src in "${BLUE_CRATE_SRC[@]}"; do
    if [ -d "$src" ]; then
        matches=$(grep -rEn --include='*.rs' "$GUARD3_PATTERN" "$src" 2>/dev/null || true)
        if [ -n "$matches" ]; then
            print_fail "Guard 3 (BLUE re-export of producer signing primitive):"
            echo "$matches"
        fi
    fi
done

# ---------------------------------------------------------------------------
# Guard 4 — `pub fn` in signing.rs returns wrapped types only.
# Allowed BLUE-bound returns: VrfProof / VrfOutput / KesSignature /
# Result<...>. Forbidden: raw [u8; …] / Vec<u8>.
# Whitelist: `derive_verification_key_bytes` (returns a *public* key —
# it is not a signing output and is required by the cold-key consumer).
# ---------------------------------------------------------------------------
if [ -f "$SIGNING_RS" ]; then
    pub_fn_signatures=$(awk '
        /^pub fn / { collecting=1; line="" }
        collecting {
            line = line " " $0
            if (index($0, "{") || index($0, ";")) {
                print line
                collecting=0
                line=""
            }
        }' "$SIGNING_RS")

    while IFS= read -r sig; do
        if [ -z "$sig" ]; then continue; fi
        if echo "$sig" | grep -F -q "derive_verification_key_bytes"; then
            continue
        fi
        if echo "$sig" | grep -E -q '\-> *\[u8;'; then
            print_fail "Guard 4 (raw [u8; …] return in signing.rs):"
            echo "  $sig"
        fi
        if echo "$sig" | grep -E -q '\-> *Vec<u8>'; then
            print_fail "Guard 4 (raw Vec<u8> return in signing.rs):"
            echo "  $sig"
        fi
    done <<< "$pub_fn_signatures"
else
    print_fail "Guard 4 setup: $SIGNING_RS not found"
fi

# ---------------------------------------------------------------------------
# Guard 5 — Debug for secret-bearing structs is hand-rolled.
# ---------------------------------------------------------------------------
GUARD5_TYPES=("VrfSigningKey" "KesSecret" "ColdSigningKey")

if [ -f "$SIGNING_RS" ]; then
    for ty in "${GUARD5_TYPES[@]}"; do
        # Forbidden: any `#[derive(…Debug…)]` immediately preceding the
        # struct declaration of one of these types.
        bad_derive=$(awk -v ty="$ty" '
            /^#\[derive\(.*Debug.*\)\]/ { prev_derive=NR; prev_line=$0; next }
            { if (NR == prev_derive + 1 && $0 ~ "(pub )?struct " ty "\\b") {
                  print prev_line " >> " $0
              }
            }' "$SIGNING_RS")
        if [ -n "$bad_derive" ]; then
            print_fail "Guard 5 ($ty has derived Debug — must be hand-rolled):"
            echo "$bad_derive"
        fi

        # Positive: ensure a hand-rolled impl exists.
        if ! grep -E -q "impl core::fmt::Debug for $ty\b|impl std::fmt::Debug for $ty\b|impl fmt::Debug for $ty\b" "$SIGNING_RS"; then
            print_fail "Guard 5 ($ty missing hand-rolled Debug impl)"
        fi
    done
fi

# ---------------------------------------------------------------------------
# Guard 6 — testkit/producer/ must not call prove(/sign_kes(/update_kes(
# outside reference_vectors.rs.
# ---------------------------------------------------------------------------
if [ -d "$TESTKIT_PRODUCER_DIR" ]; then
    while IFS= read -r -d '' rs; do
        if [ "$rs" = "$TESTKIT_REF_VECTORS" ]; then
            continue
        fi
        bad=$(grep -nE 'prove\(|sign_kes\(|update_kes\(' "$rs" 2>/dev/null || true)
        if [ -n "$bad" ]; then
            print_fail "Guard 6 (signing call in testkit/producer/ outside reference_vectors.rs):"
            echo "  $rs:"
            echo "$bad"
        fi
    done < <(find "$TESTKIT_PRODUCER_DIR" -name '*.rs' -print0)
fi

if [ "$FAILED" -eq 0 ]; then
    echo "PASS: private-key custody gates green (6/6)"
    exit 0
else
    exit 1
fi
