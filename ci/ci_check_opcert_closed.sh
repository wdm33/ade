#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-C S2 — opcert closed-grammar + closed-validate authority gate.
#
# Mechanical guards (closure proof for CE-N-C-2 / DC-CONS-11 / DC-CONS-12):
#
#   1. No parallel opcert encoders. The only `pub fn .*encode.*opcert`
#      or `pub fn .*write_opcert` declarations in BLUE source live in
#      crates/ade_codec/src/shelley/opcert.rs. The Shelley header path
#      no longer contains an inline 4-field opcert emit sequence.
#   2. No parallel opcert decoders. The only `pub fn .*decode.*opcert`
#      or `pub fn .*read_opcert` declarations in BLUE source live in
#      crates/ade_codec/src/shelley/opcert.rs.
#   3. `OpCertError` is a closed sum — no `#[non_exhaustive]`.
#   4. `OpCertCodecError` is a closed sum — no `#[non_exhaustive]`.
#   5. No production call site of `opcert_validate(` outside
#      crates/ade_core/src/consensus/ and crates/ade_runtime/src/producer/.
#      `crates/*/tests/` and `#[cfg(test)] mod tests` blocks are
#      whitelisted.
#   6. `opcert_validate` is a free function, never a method on
#      OperationalCert or any opcert-named type.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

BLUE_CRATE_SRC=(
    "$REPO_ROOT/crates/ade_core/src"
    "$REPO_ROOT/crates/ade_codec/src"
    "$REPO_ROOT/crates/ade_types/src"
    "$REPO_ROOT/crates/ade_ledger/src"
    "$REPO_ROOT/crates/ade_crypto/src"
)

OPCERT_CODEC="$REPO_ROOT/crates/ade_codec/src/shelley/opcert.rs"
OPCERT_VALIDATE="$REPO_ROOT/crates/ade_core/src/consensus/opcert_validate.rs"
HEADER_CODEC="$REPO_ROOT/crates/ade_codec/src/shelley/block.rs"

FAIL=0

print_fail() {
    echo "FAIL: $1"
    FAIL=1
}

# Emit non-test production lines for a .rs file: stop at the first
# `#[cfg(test)]` block opener. Inline `#[cfg(test)] mod tests` lives at
# the bottom of source files throughout this repo.
emit_production_lines() {
    local f="$1"
    awk '/^#\[cfg\(test\)\]/ { exit } { print NR ":" $0 }' "$f"
}

# Required-file presence.
for f in "$OPCERT_CODEC" "$OPCERT_VALIDATE"; do
    [ -f "$f" ] || { print_fail "expected file missing: $f"; }
done
[ "$FAIL" -eq 0 ] || exit 1

# ---------------------------------------------------------------------------
# Guard 1 — No parallel opcert encoders outside opcert.rs.
# ---------------------------------------------------------------------------
GUARD1_PATTERNS=(
    'pub fn [A-Za-z_]*opcert[A-Za-z_]*encode'
    'pub fn encode[A-Za-z_]*opcert'
    'pub fn write_opcert'
)

for pattern in "${GUARD1_PATTERNS[@]}"; do
    for src in "${BLUE_CRATE_SRC[@]}"; do
        if [ -d "$src" ]; then
            while IFS= read -r -d '' rs; do
                if [ "$rs" = "$OPCERT_CODEC" ]; then
                    continue
                fi
                matches=$(emit_production_lines "$rs" | grep -E "$pattern" || true)
                if [ -n "$matches" ]; then
                    print_fail "Guard 1 (parallel opcert encoder outside opcert.rs): $pattern"
                    echo "$rs:"
                    echo "$matches"
                fi
            done < <(find "$src" -name '*.rs' -print0)
        fi
    done
done

# Guard 1b — header codec must not inline opcert bstr/uint emits.
# A regression that re-introduces `write_bytes_canonical(.., &..hot_vkey)`
# or any of the other three field emits outside opcert.rs would resurface
# the parallel-encoder hazard.
INLINE_PATTERNS=(
    'write_bytes_canonical\(.*\.hot_vkey'
    'write_bytes_canonical\(.*\.sigma'
    'write_uint_canonical\(.*\.sequence_number'
    'write_uint_canonical\(.*\.kes_period'
)
for pattern in "${INLINE_PATTERNS[@]}"; do
    matches=$(grep -nE "$pattern" "$HEADER_CODEC" || true)
    if [ -n "$matches" ]; then
        print_fail "Guard 1b (inline opcert field emit in $HEADER_CODEC): $pattern"
        echo "$matches"
    fi
done

# ---------------------------------------------------------------------------
# Guard 2 — No parallel opcert decoders outside opcert.rs.
# ---------------------------------------------------------------------------
GUARD2_PATTERNS=(
    'pub fn [A-Za-z_]*opcert[A-Za-z_]*decode'
    'pub fn decode[A-Za-z_]*opcert'
    'pub fn read_opcert'
)

for pattern in "${GUARD2_PATTERNS[@]}"; do
    for src in "${BLUE_CRATE_SRC[@]}"; do
        if [ -d "$src" ]; then
            while IFS= read -r -d '' rs; do
                if [ "$rs" = "$OPCERT_CODEC" ]; then
                    continue
                fi
                matches=$(emit_production_lines "$rs" | grep -E "$pattern" || true)
                if [ -n "$matches" ]; then
                    print_fail "Guard 2 (parallel opcert decoder outside opcert.rs): $pattern"
                    echo "$rs:"
                    echo "$matches"
                fi
            done < <(find "$src" -name '*.rs' -print0)
        fi
    done
done

# ---------------------------------------------------------------------------
# Guard 3 — OpCertError closed (no #[non_exhaustive]).
# ---------------------------------------------------------------------------
if grep -B1 'pub enum OpCertError' "$OPCERT_VALIDATE" | grep -q '#\[non_exhaustive\]'; then
    print_fail "Guard 3 (OpCertError is #[non_exhaustive] — must be a closed sum)"
fi

# ---------------------------------------------------------------------------
# Guard 4 — OpCertCodecError closed.
# ---------------------------------------------------------------------------
if grep -B1 'pub enum OpCertCodecError' "$OPCERT_CODEC" | grep -q '#\[non_exhaustive\]'; then
    print_fail "Guard 4 (OpCertCodecError is #[non_exhaustive] — must be a closed sum)"
fi

# ---------------------------------------------------------------------------
# Guard 5 — No production call site of opcert_validate( outside the
# sanctioned BLUE module + RED producer. Test code is whitelisted.
# ---------------------------------------------------------------------------
SANCTIONED_VALIDATE_DIRS=(
    "$REPO_ROOT/crates/ade_core/src/consensus/"
    "$REPO_ROOT/crates/ade_runtime/src/producer/"
    "$REPO_ROOT/crates/ade_ledger/src/producer/"
)

# Collect all callsites across the workspace, then filter.
ALL_CALLSITES=$(grep -rnE 'opcert_validate\s*\(' "$REPO_ROOT/crates" --include='*.rs' 2>/dev/null || true)

while IFS= read -r hit; do
    [ -z "$hit" ] && continue
    file="${hit%%:*}"
    rest="${hit#*:}"
    line_no="${rest%%:*}"

    # Whitelist: tests directories and inline #[cfg(test)] blocks.
    if echo "$file" | grep -qE '/tests/'; then
        continue
    fi
    cfg_test_line=$(grep -nE '^#\[cfg\(test\)\]' "$file" 2>/dev/null | head -1 | cut -d: -f1)
    if [ -n "$cfg_test_line" ] && [ "$line_no" -gt "$cfg_test_line" ]; then
        continue
    fi

    # Whitelist: the function definition itself.
    if echo "$rest" | grep -qE 'pub fn opcert_validate\b'; then
        continue
    fi

    # Sanctioned-directory check.
    sanctioned=0
    for dir in "${SANCTIONED_VALIDATE_DIRS[@]}"; do
        case "$file" in
            "$dir"*) sanctioned=1; break ;;
        esac
    done
    if [ "$sanctioned" -eq 0 ]; then
        print_fail "Guard 5 (opcert_validate production call outside sanctioned dirs):"
        echo "  $hit"
    fi
done <<< "$ALL_CALLSITES"

# ---------------------------------------------------------------------------
# Guard 6 — opcert_validate is a free function. Forbidden: any
# `impl OperationalCert { fn validate ... }` or `impl <Opcert-named> {
# fn validate ... }` in source.
# ---------------------------------------------------------------------------
IMPL_HITS=$(grep -rEn 'impl [A-Za-z_:]*[Oo]p[A-Za-z_]*[Cc]ert[A-Za-z_]* *\{' \
    "$REPO_ROOT/crates" --include='*.rs' 2>/dev/null || true)
while IFS= read -r hit; do
    [ -z "$hit" ] && continue
    file="${hit%%:*}"
    rest="${hit#*:}"
    line_no="${rest%%:*}"
    # If the impl block contains `fn validate`, that's a method-shaped
    # validate — forbidden.
    end_line=$(awk -v start="$line_no" 'NR >= start { depth += gsub(/\{/, "{"); depth -= gsub(/\}/, "}"); if (NR >= start && depth == 0) { print NR; exit } }' "$file")
    if [ -z "$end_line" ]; then
        continue
    fi
    body=$(awk -v s="$line_no" -v e="$end_line" 'NR >= s && NR <= e' "$file")
    if echo "$body" | grep -qE '\bfn validate\b'; then
        print_fail "Guard 6 (impl <opcert type> { fn validate ... } — validate must be a free function):"
        echo "  $hit"
    fi
done <<< "$IMPL_HITS"

if [ "$FAIL" -eq 0 ]; then
    echo "PASS: opcert closed-grammar + closed-validate gates green (6/6)"
    exit 0
else
    exit 1
fi
