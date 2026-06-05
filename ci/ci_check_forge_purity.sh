#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-C S3 — forge purity + closed-grammar gate.
#
# Mechanical guards (closure proof for CE-N-C-3 / DC-CONS-13/14/15 /
# DC-LEDGER-12):
#
#   1. No I/O / clock / rand / HashMap iteration / floating-point /
#      println / async in forge.rs, state.rs, or tx_components.rs.
#   2. forge.rs calls `is_leader_for_vrf_output(` (validator's
#      shared function). The only `fn .*is_leader.*` definitions
#      reachable from ade_core / ade_ledger are the canonical ones at
#      crates/ade_core/src/consensus/leader_schedule.rs and
#      crates/ade_core/src/consensus/leader_check.rs (the latter hosts
#      the relocated is_leader_for_vrf_output as of N-R-A; its own
#      authority is enforced by ci_check_leader_check_authority.sh).
#   3. `pub fn forge_block` returns the closed sum
#      `Result<(ForgedBlock, Vec<ForgeEffects>), ForgeError>`.
#   4. `ForgeError`, `ForgeEffects`, `ProducerTick`, `ForgedBlock`,
#      `TxComponents` are closed sums (no `#[non_exhaustive]`).
#   5. `ProducerTick` carries no private-key types as fields.
#   6. No `VrfDraft03::prove` / `Sum6Kes::sign_kes` /
#      `KesAlgorithm::sign_kes` / `update_kes` call inside
#      ade_ledger/src/producer/ or ade_core/src/.
#   7. No `String`-bearing variant on `ForgeError` or `ForgeEffects`.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

FORGE_RS="$REPO_ROOT/crates/ade_ledger/src/producer/forge.rs"
STATE_RS="$REPO_ROOT/crates/ade_ledger/src/producer/state.rs"
TX_COMPONENTS_RS="$REPO_ROOT/crates/ade_codec/src/shelley/tx_components.rs"

LEADER_SCHEDULE_RS="$REPO_ROOT/crates/ade_core/src/consensus/leader_schedule.rs"
VRF_CERT_RS="$REPO_ROOT/crates/ade_core/src/consensus/vrf_cert.rs"
LEADER_CHECK_RS="$REPO_ROOT/crates/ade_core/src/consensus/leader_check.rs"

TARGET_FILES=("$FORGE_RS" "$STATE_RS" "$TX_COMPONENTS_RS")

FAILED=0

print_fail() {
    echo "FAIL: $1"
    FAILED=1
}

emit_production_lines() {
    local f="$1"
    # Strip line comments (`//...`) and stop at the first `#[cfg(test)]`
    # block opener. Doc comments are skipped so the standard
    # "no HashMap/HashSet/floats" Core Contract header (present on
    # every BLUE module) does not trip the impurity grep.
    awk '
        /^#\[cfg\(test\)\]/ { exit }
        {
            line=$0
            sub(/\/\/.*$/, "", line)
            print NR ":" line
        }
    ' "$f"
}

for f in "${TARGET_FILES[@]}"; do
    [ -f "$f" ] || { print_fail "expected file missing: $f"; }
done
[ "$FAILED" -eq 0 ] || exit 1

# ---------------------------------------------------------------------------
# Guard 1 — purity (no I/O / clock / rand / HashMap iter / floats / async).
# ---------------------------------------------------------------------------
GUARD1_PATTERNS=(
    'std::time'
    'tokio::time'
    'rand::'
    'getrandom'
    'std::fs'
    'std::env'
    'std::net'
    'HashMap'
    'HashSet'
    '\bf32\b'
    '\bf64\b'
    'println!'
    'eprintln!'
    'dbg!'
    'async fn'
    '\.await\b'
)

for f in "${TARGET_FILES[@]}"; do
    lines=$(emit_production_lines "$f")
    for pattern in "${GUARD1_PATTERNS[@]}"; do
        matches=$(echo "$lines" | grep -E "$pattern" || true)
        if [ -n "$matches" ]; then
            print_fail "Guard 1 (impure pattern '$pattern' in $f):"
            echo "$matches"
        fi
    done
done

# ---------------------------------------------------------------------------
# Guard 2 — forge.rs uses the validator's leader-check function and no
# parallel `is_leader` definition exists in ade_core / ade_ledger.
# ---------------------------------------------------------------------------
if ! grep -q 'is_leader_for_vrf_output(' "$FORGE_RS"; then
    print_fail "Guard 2 (forge.rs does not call is_leader_for_vrf_output)"
fi

# Collect every `fn .*is_leader` definition in ade_core/src and
# ade_ledger/src. The canonical one lives in leader_schedule.rs.
IS_LEADER_DEFS=$(grep -rEn '^\s*pub fn [a-z_]*is_leader[a-z_]*\b|^\s*fn [a-z_]*is_leader[a-z_]*\b' \
    "$REPO_ROOT/crates/ade_core/src" \
    "$REPO_ROOT/crates/ade_ledger/src" \
    --include='*.rs' 2>/dev/null || true)
while IFS= read -r hit; do
    [ -z "$hit" ] && continue
    file="${hit%%:*}"
    if [ "$file" = "$LEADER_SCHEDULE_RS" ] || [ "$file" = "$VRF_CERT_RS" ] || [ "$file" = "$LEADER_CHECK_RS" ]; then
        continue
    fi
    # Whitelist test helper definitions inside #[cfg(test)] blocks.
    line_no="${hit#*:}"
    line_no="${line_no%%:*}"
    cfg_test_line=$(grep -nE '^#\[cfg\(test\)\]' "$file" 2>/dev/null | head -1 | cut -d: -f1 || true)
    if [ -n "$cfg_test_line" ] && [ "$line_no" -gt "$cfg_test_line" ]; then
        continue
    fi
    print_fail "Guard 2 (parallel is_leader definition outside leader_schedule.rs):"
    echo "  $hit"
done <<< "$IS_LEADER_DEFS"

# ---------------------------------------------------------------------------
# Guard 3 — forge_block returns the closed Result sum exactly.
# ---------------------------------------------------------------------------
SIGNATURE_LINES=$(awk '
    /^pub fn forge_block/ { collecting=1; line="" }
    collecting {
        line = line " " $0
        if (index($0, "{") || index($0, ";")) {
            print line
            collecting=0
            line=""
        }
    }' "$FORGE_RS")
if ! echo "$SIGNATURE_LINES" | grep -F -q 'Result<(ForgedBlock, Vec<ForgeEffects>), ForgeError>'; then
    print_fail "Guard 3 (forge_block return signature does not match closed sum):"
    echo "  $SIGNATURE_LINES"
fi

# ---------------------------------------------------------------------------
# Guard 4 — closure of ForgeError / ForgeEffects / ProducerTick /
# ForgedBlock / TxComponents (no #[non_exhaustive]).
# ---------------------------------------------------------------------------
GUARD4_TYPES=(
    "ForgeError:$FORGE_RS"
    "ForgeEffects:$FORGE_RS"
    "ForgedBlock:$FORGE_RS"
    "ProducerTick:$STATE_RS"
    "TxComponents:$TX_COMPONENTS_RS"
)
for entry in "${GUARD4_TYPES[@]}"; do
    ty="${entry%%:*}"
    file="${entry#*:}"
    if grep -B1 -E "pub (enum|struct) $ty\\b" "$file" | grep -q '#\[non_exhaustive\]'; then
        print_fail "Guard 4 ($ty is #[non_exhaustive] — must be a closed sum)"
    fi
done

# ---------------------------------------------------------------------------
# Guard 5 — no private-key types on ProducerTick.
# ---------------------------------------------------------------------------
GUARD5_PATTERNS=(
    'VrfSigningKey'
    'KesSecret'
    'ColdSigningKey'
    'KesSigningKey'
)
for pattern in "${GUARD5_PATTERNS[@]}"; do
    matches=$(emit_production_lines "$STATE_RS" | grep -E "$pattern" || true)
    if [ -n "$matches" ]; then
        print_fail "Guard 5 (private-key field on ProducerTick — pattern $pattern):"
        echo "$matches"
    fi
done

# ---------------------------------------------------------------------------
# Guard 6 — no signing API call in ade_ledger/src/producer/ or
# ade_core/src/.
# ---------------------------------------------------------------------------
GUARD6_PATTERNS=(
    'VrfDraft03::prove'
    'Sum6Kes::sign_kes'
    'Sum6Kes::update_kes'
    'KesAlgorithm::sign_kes'
    'KesAlgorithm::update_kes'
)
GUARD6_DIRS=(
    "$REPO_ROOT/crates/ade_ledger/src/producer"
    "$REPO_ROOT/crates/ade_core/src"
)
for pattern in "${GUARD6_PATTERNS[@]}"; do
    for dir in "${GUARD6_DIRS[@]}"; do
        if [ -d "$dir" ]; then
            while IFS= read -r -d '' rs; do
                matches=$(emit_production_lines "$rs" | grep -E "$pattern" || true)
                if [ -n "$matches" ]; then
                    print_fail "Guard 6 (signing API call in $rs):"
                    echo "$matches"
                fi
            done < <(find "$dir" -name '*.rs' -print0)
        fi
    done
done

# ---------------------------------------------------------------------------
# Guard 7 — no String-bearing variant on ForgeError or ForgeEffects.
# Closure proof: errors must be PartialEq-stable byte-for-byte for replay.
# ---------------------------------------------------------------------------
# Extract the body of each enum and grep for `String`.
for ty in ForgeError ForgeEffects; do
    body=$(awk -v ty="$ty" '
        $0 ~ "pub enum " ty " *\\{" { open=1; depth=0 }
        open {
            depth += gsub(/\{/, "{")
            depth -= gsub(/\}/, "}")
            print
            if (depth == 0 && /\}/) { exit }
        }
    ' "$FORGE_RS")
    if echo "$body" | grep -E -q ': *String\b|: *alloc::string::String\b'; then
        print_fail "Guard 7 ($ty has a String-bearing variant):"
        echo "$body" | grep -E ': *String\b|: *alloc::string::String\b'
    fi
done

if [ "$FAILED" -eq 0 ]; then
    echo "PASS: forge purity gates green (7/7)"
    exit 0
else
    exit 1
fi
