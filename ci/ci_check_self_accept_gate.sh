#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-C S5 — self-accept gate. Closes CN-CONS-07 mechanical half.
#
# Mechanical guards (closure proof for CE-N-C-5 / CN-CONS-07):
#
#   1. `AcceptedBlock` has no public constructor outside self_accept.rs.
#      - struct-literal `AcceptedBlock {` matches ONLY in self_accept.rs.
#      - EXACTLY one `pub fn .* -> AcceptedBlock` match across crates/
#        (the `self_accept` function in self_accept.rs returning
#        `Result<AcceptedBlock, SelfAcceptError>` — the regex matches
#        the `AcceptedBlock` portion of the return).
#      - No `impl Default for AcceptedBlock` / `impl From<.*> for
#        AcceptedBlock` / `impl TryFrom<.*> for AcceptedBlock` anywhere.
#   2. `AcceptedBlock.bytes` field is private (no `pub bytes:` in
#      self_accept.rs).
#   3. `SelfAcceptError` is a closed sum (no #[non_exhaustive]; no
#      String-bearing variant).
#   4. `self_accept` calls the canonical `block_validity` validator
#      (grep `block_validity(` in self_accept.rs returns >= 1 hit).
#   5. `self_accept` does NOT re-implement validator sub-steps. Grep
#      self_accept.rs for `validate_and_apply_header(` / `decode_block(`
#      / `block_body_hash(` — finding any is failure.
#   6. No `pub fn` in self_accept.rs returns a raw `Vec<u8>` /
#      `&[u8]` other than the `as_bytes` / `into_bytes` accessors on
#      `AcceptedBlock` (those decompose a token already gated by the
#      verdict).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

SELF_ACCEPT_RS="$REPO_ROOT/crates/ade_ledger/src/producer/self_accept.rs"
CRATES_DIR="$REPO_ROOT/crates"

FAIL=0

print_fail() {
    echo "FAIL: $1"
    FAIL=1
}

if [ ! -f "$SELF_ACCEPT_RS" ]; then
    print_fail "expected file missing: $SELF_ACCEPT_RS"
    exit 1
fi

# ---------------------------------------------------------------------------
# Guard 1 — AcceptedBlock has no public constructor outside self_accept.rs.
# ---------------------------------------------------------------------------

# (1a) Struct-literal `AcceptedBlock {` only in self_accept.rs.
G1A_HITS=$(grep -rnE 'AcceptedBlock\s*\{' "$CRATES_DIR" --include='*.rs' 2>/dev/null || true)
while IFS= read -r hit; do
    [ -z "$hit" ] && continue
    file="${hit%%:*}"
    if [ "$file" != "$SELF_ACCEPT_RS" ]; then
        print_fail "Guard 1a (AcceptedBlock {} construction outside self_accept.rs):"
        echo "  $hit"
    fi
done <<< "$G1A_HITS"

# (1b) Exactly one public function whose return type binds AcceptedBlock,
# anywhere in crates/. The canonical shape is
# `-> Result<AcceptedBlock, SelfAcceptError>`; we also flag a bare
# `-> AcceptedBlock` if it ever appears. Multi-line signatures are
# handled by greppng for the arrow-+-type token directly.
G1B_HITS=$(grep -rnE '\-> *(Result<)?AcceptedBlock\b' "$CRATES_DIR" --include='*.rs' 2>/dev/null || true)
G1B_COUNT=$(echo -n "$G1B_HITS" | grep -c . || true)
if [ "$G1B_COUNT" -ne 1 ]; then
    print_fail "Guard 1b (expected exactly 1 '-> [Result<]AcceptedBlock' return-type match across crates/, got $G1B_COUNT):"
    echo "$G1B_HITS"
fi
# And that one match must live in self_accept.rs.
G1B_FILE=$(echo "$G1B_HITS" | head -1 | cut -d: -f1)
if [ -n "$G1B_FILE" ] && [ "$G1B_FILE" != "$SELF_ACCEPT_RS" ]; then
    print_fail "Guard 1b (the single AcceptedBlock-returning fn lives outside self_accept.rs):"
    echo "  $G1B_HITS"
fi

# (1c) No `impl Default | From<*> | TryFrom<*> for AcceptedBlock` anywhere.
G1C_HITS=$(grep -rnE 'impl (Default|From<.*>|TryFrom<.*>) for AcceptedBlock' \
    "$CRATES_DIR" --include='*.rs' 2>/dev/null || true)
while IFS= read -r hit; do
    [ -z "$hit" ] && continue
    print_fail "Guard 1c (forbidden trait impl for AcceptedBlock):"
    echo "  $hit"
done <<< "$G1C_HITS"

# ---------------------------------------------------------------------------
# Guard 2 — AcceptedBlock.bytes field is private.
# ---------------------------------------------------------------------------
G2_HITS=$(grep -nE '^\s*pub\s+bytes\s*:' "$SELF_ACCEPT_RS" 2>/dev/null || true)
if [ -n "$G2_HITS" ]; then
    print_fail "Guard 2 (AcceptedBlock.bytes is pub — token field must be private):"
    echo "$G2_HITS"
fi

# ---------------------------------------------------------------------------
# Guard 3 — SelfAcceptError is a closed sum.
#   - No #[non_exhaustive] on the enum.
#   - No String-bearing variant.
# ---------------------------------------------------------------------------
if grep -B1 -E '^pub enum SelfAcceptError\b' "$SELF_ACCEPT_RS" | grep -q '#\[non_exhaustive\]'; then
    print_fail "Guard 3 (SelfAcceptError is #[non_exhaustive] — must be a closed sum)"
fi

G3_BODY=$(awk '
    /^pub enum SelfAcceptError *\{/ { open=1; depth=0 }
    open {
        depth += gsub(/\{/, "{")
        depth -= gsub(/\}/, "}")
        print
        if (depth == 0 && /\}/) { exit }
    }
' "$SELF_ACCEPT_RS")
if echo "$G3_BODY" | grep -E -q ': *String\b|: *alloc::string::String\b'; then
    print_fail "Guard 3 (SelfAcceptError has a String-bearing variant):"
    echo "$G3_BODY" | grep -E ': *String\b|: *alloc::string::String\b'
fi

# ---------------------------------------------------------------------------
# Guard 4 — self_accept calls the canonical block_validity validator.
# ---------------------------------------------------------------------------
if ! grep -qE 'block_validity\s*\(' "$SELF_ACCEPT_RS"; then
    print_fail "Guard 4 (self_accept.rs does not call block_validity() — canonical authority is unused)"
fi

# ---------------------------------------------------------------------------
# Guard 5 — self_accept does NOT re-implement validator sub-steps.
# ---------------------------------------------------------------------------
FORBIDDEN_SUBCALLS=(
    'validate_and_apply_header\s*\('
    'decode_block\s*\('
    'block_body_hash\s*\('
)
# Production view: strip line comments and stop at the first #[cfg(test)].
PROD_LINES=$(awk '
    /^#\[cfg\(test\)\]/ { exit }
    {
        line = $0
        sub(/\/\/.*$/, "", line)
        print NR ":" line
    }
' "$SELF_ACCEPT_RS")
for pat in "${FORBIDDEN_SUBCALLS[@]}"; do
    hits=$(echo "$PROD_LINES" | grep -E "$pat" || true)
    if [ -n "$hits" ]; then
        print_fail "Guard 5 (self_accept.rs re-implements validator sub-step '$pat' — must delegate to block_validity):"
        echo "$hits"
    fi
done

# ---------------------------------------------------------------------------
# Guard 6 — No `pub fn` returning raw bytes other than `as_bytes` /
# `into_bytes` accessors on AcceptedBlock.
# ---------------------------------------------------------------------------
RAW_BYTE_PUB_FNS=$(echo "$PROD_LINES" | grep -E 'pub fn .*-> *(Vec<u8>|&\[u8\])' || true)
while IFS= read -r line; do
    [ -z "$line" ] && continue
    # Drop the leading "NR:" prefix from awk to inspect the function name.
    body="${line#*:}"
    if echo "$body" | grep -qE 'pub fn (as_bytes|into_bytes)\b'; then
        continue
    fi
    print_fail "Guard 6 (pub fn returning raw bytes outside as_bytes/into_bytes):"
    echo "  $line"
done <<< "$RAW_BYTE_PUB_FNS"

if [ "$FAIL" -eq 0 ]; then
    echo "PASS: self-accept gate green (6/6)"
    exit 0
else
    exit 1
fi
