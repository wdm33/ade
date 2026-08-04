#!/usr/bin/env bash
set -uo pipefail

# PREPROD-ENTRY-AUTHORITY P6 (DC-STORE-12) -- the semantics-bearing surface lock.
#
# STORE_SEMANTICS_VERSION records which PRODUCTION rules wrote a store's derived bytes. A constant
# that must be REMEMBERED gets forgotten exactly once, which is all it takes (P4, e1de7a2e). This gate
# content-hashes the declared surface in ci/store-semantics-surface.lock and fails on any drift until
# the author makes one of two EXPLICIT choices: bump the version, or record semantics_neutral with a
# rationale. There is deliberately no silent third option.
#
# Checks:
#   (A) every declared surface file exists (a renamed/deleted file cannot silently leave the surface).
#   (B) the computed surface hash equals the LAST lock entry's surface_hash.
#   (C) the LAST entry's store_semantics_version equals the STORE_SEMANTICS_VERSION constant in code.
#   (D) versions are non-decreasing across entries (append-only, never rewritten downward).
#   (E) every entry carries a non-empty rationale.
#   (F) a non-neutral entry strictly INCREASED the version relative to the previous entry.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LOCK="$REPO_ROOT/ci/store-semantics-surface.lock"
CONST_FILE="$REPO_ROOT/crates/ade_ledger/src/store_semantics.rs"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

[[ -e "$LOCK" ]] || { echo "FAIL: missing $LOCK"; exit 1; }
[[ -e "$CONST_FILE" ]] || { echo "FAIL: missing $CONST_FILE"; exit 1; }

# --- the declared surface -------------------------------------------------------------------
mapfile -t FILES < <(awk '/^\[surface\]/{s=1;next} /^\[\[entry\]\]/{s=0} s' "$LOCK" \
    | grep -oE '"[^"]+\.rs"' | tr -d '"')

if [[ "${#FILES[@]}" -eq 0 ]]; then
    print_fail "(A) the lock declares no surface files -- the gate would pass vacuously"
fi

for f in "${FILES[@]}"; do
    [[ -e "$REPO_ROOT/$f" ]] || print_fail "(A) declared surface file is missing: $f (renamed or deleted without updating the lock)"
done

# --- (B) computed hash vs the last entry ------------------------------------------------------
# Hash file CONTENT plus the path, in the declared order, so a rename is a change too.
COMPUTED=$(
    for f in "${FILES[@]}"; do
        printf '%s\n' "$f"
        [[ -e "$REPO_ROOT/$f" ]] && cat "$REPO_ROOT/$f"
    done | sha256sum | cut -d' ' -f1
)

RECORDED=$(grep -oE '^surface_hash = "[^"]*"' "$LOCK" | tail -1 | sed 's/^surface_hash = "//;s/"$//')

if [[ "$RECORDED" == "PLACEHOLDER_FILLED_BY_FIRST_RUN" ]]; then
    echo "note: lock placeholder present; computed surface hash is:"
    echo "      $COMPUTED"
    print_fail "(B) the lock still holds the placeholder -- record the computed hash above in the last entry"
elif [[ "$COMPUTED" != "$RECORDED" ]]; then
    cat <<EOF
FAIL: (B) the semantics-bearing surface CHANGED but ci/store-semantics-surface.lock was not reconciled.

  computed: $COMPUTED
  recorded: $RECORDED

Append a new [[entry]] to ci/store-semantics-surface.lock with surface_hash = "$COMPUTED" and make ONE
explicit choice:

  * the change alters what authoritative rules PRODUCE  -> increase store_semantics_version here AND in
    crates/ade_ledger/src/store_semantics.rs (stores written by the previous binary become invalid);
  * the change provably does NOT                        -> semantics_neutral = true, same version, and a
    rationale naming why.
EOF
    FAILED=1
fi

# --- (C) last entry version == the code constant ----------------------------------------------
CODE_VERSION=$(grep -oE 'pub const STORE_SEMANTICS_VERSION: u32 = [0-9]+' "$CONST_FILE" | grep -oE '[0-9]+$')
LOCK_VERSION=$(grep -oE '^store_semantics_version = [0-9]+' "$LOCK" | tail -1 | grep -oE '[0-9]+$')
if [[ -z "$CODE_VERSION" ]]; then
    print_fail "(C) could not read STORE_SEMANTICS_VERSION from $CONST_FILE"
elif [[ "$CODE_VERSION" != "$LOCK_VERSION" ]]; then
    print_fail "(C) STORE_SEMANTICS_VERSION in code is $CODE_VERSION but the last lock entry says $LOCK_VERSION"
fi

# --- (D)(E)(F) entry-log discipline ------------------------------------------------------------
mapfile -t VERSIONS < <(grep -oE '^store_semantics_version = [0-9]+' "$LOCK" | grep -oE '[0-9]+$')
mapfile -t NEUTRALS < <(grep -oE '^semantics_neutral = (true|false)' "$LOCK" | sed 's/^semantics_neutral = //')
RATIONALES=$(grep -cE '^rationale = ".+"' "$LOCK" || true)

if [[ "${#VERSIONS[@]}" -ne "${#NEUTRALS[@]}" || "${#VERSIONS[@]}" -ne "$RATIONALES" ]]; then
    print_fail "(E) every [[entry]] needs store_semantics_version, semantics_neutral and a non-empty rationale (found ${#VERSIONS[@]}/${#NEUTRALS[@]}/$RATIONALES)"
fi

for ((i = 1; i < ${#VERSIONS[@]}; i++)); do
    prev="${VERSIONS[i-1]}"; cur="${VERSIONS[i]}"
    if (( cur < prev )); then
        print_fail "(D) store_semantics_version decreased ($prev -> $cur) -- the log is append-only"
    fi
    if [[ "${NEUTRALS[i]}" == "false" ]] && (( cur <= prev )); then
        print_fail "(F) entry $i is not semantics_neutral but did not increase the version ($prev -> $cur)"
    fi
done

if [[ "$FAILED" -ne 0 ]]; then
    echo "ci_check_store_semantics_lock: FAILED"
    exit 1
fi
echo "ci_check_store_semantics_lock: OK (surface hash matches; version $CODE_VERSION reconciled)"
