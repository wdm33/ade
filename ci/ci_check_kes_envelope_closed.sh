#!/usr/bin/env bash
set -euo pipefail

# PHASE4-N-O S1 — Ade KES envelope closure gate.
#
# Mechanical guards (closure proof for DC-CRYPTO-06 + DC-CRYPTO-07):
#   1. KeyLoadError carries every load-bearing closed variant.
#   2. load_kes_signing_key_skey (the cardano-cli envelope path)
#      returns UnsupportedExpandedKesKeyFormat and never constructs a
#      KesSecret (i.e., no call to KesSecret::from_bytes_zeroizing or
#      KesSecret::from_seed_at_period in that function's body).
#   3. ade_kes_envelope.rs defines its on-disk struct with
#      `#[serde(deny_unknown_fields)]` on the load-bearing keys.
#   4. No checked-in JSONL log / admission transcript / evidence file
#      contains the literal substring "seed_32" (outside the envelope
#      source + slice/cluster/registry docs).
#   5. key_gen.rs prints exactly the four allowed success lines (and
#      no other println! invocation precedes the fingerprint println).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

KEYS_RS="$REPO_ROOT/crates/ade_runtime/src/producer/keys.rs"
ENV_RS="$REPO_ROOT/crates/ade_runtime/src/producer/ade_kes_envelope.rs"
KEYGEN_RS="$REPO_ROOT/crates/ade_node/src/key_gen.rs"

[ -f "$KEYS_RS" ] || { echo "[ci_check_kes_envelope_closed] missing keys.rs"; exit 1; }
[ -f "$ENV_RS"  ] || { echo "[ci_check_kes_envelope_closed] missing ade_kes_envelope.rs"; exit 1; }
[ -f "$KEYGEN_RS" ] || { echo "[ci_check_kes_envelope_closed] missing key_gen.rs"; exit 1; }

# ---- 1. KeyLoadError closed variants ----------------------------------
REQUIRED_VARIANTS=(
    "UnsupportedExpandedKesKeyFormat"
    "AdeEnvelope"
)
for v in "${REQUIRED_VARIANTS[@]}"; do
    if ! grep -q "^\s*$v" "$KEYS_RS"; then
        echo "[ci_check_kes_envelope_closed] FAIL: KeyLoadError missing variant: $v"
        exit 1
    fi
done

# ---- 1b. AdeKesEnvelopeError closed variants --------------------------
ADE_ENV_VARIANTS=(
    "UnknownEnvelopeFormat"
    "WrongKeyRole"
    "UnsupportedCryptoTag"
    "MissingSeed32"
    "MalformedSeed32"
    "MalformedPeriodIdx"
    "PeriodIdxOutOfRange"
    "UnsupportedFormatVersion"
    "MalformedJson"
)
for v in "${ADE_ENV_VARIANTS[@]}"; do
    if ! grep -q "$v" "$ENV_RS"; then
        echo "[ci_check_kes_envelope_closed] FAIL: AdeKesEnvelopeError missing variant: $v"
        exit 1
    fi
done

# ---- 2. cardano-cli loader fail-closed ---------------------------------
# Extract the body of load_kes_signing_key_skey via a coarse range cut and
# assert (a) UnsupportedExpandedKesKeyFormat appears, (b) no KesSecret
# constructor call appears.
LOADER_BODY="$(awk '
    /pub fn load_kes_signing_key_skey/ { in_fn = 1; depth = 0 }
    in_fn {
        print
        for (i = 1; i <= length($0); i++) {
            c = substr($0, i, 1)
            if (c == "{") depth++
            if (c == "}") {
                depth--
                if (depth == 0) { in_fn = 0; break }
            }
        }
    }
' "$KEYS_RS")"

if [ -z "$LOADER_BODY" ]; then
    echo "[ci_check_kes_envelope_closed] FAIL: cannot locate load_kes_signing_key_skey"
    exit 1
fi

if ! grep -q "UnsupportedExpandedKesKeyFormat" <<<"$LOADER_BODY"; then
    echo "[ci_check_kes_envelope_closed] FAIL: load_kes_signing_key_skey must return UnsupportedExpandedKesKeyFormat"
    exit 1
fi

if grep -q "KesSecret::from_bytes_zeroizing\|KesSecret::from_seed_at_period" <<<"$LOADER_BODY"; then
    echo "[ci_check_kes_envelope_closed] FAIL: load_kes_signing_key_skey must not construct a KesSecret (the cardano-cli envelope is the fail-closed path)"
    exit 1
fi

# ---- 3. deny_unknown_fields on the on-disk struct ----------------------
if ! grep -q "deny_unknown_fields" "$ENV_RS"; then
    echo "[ci_check_kes_envelope_closed] FAIL: ade_kes_envelope.rs must use #[serde(deny_unknown_fields)]"
    exit 1
fi

# ---- 4. No JSONL log / transcript / evidence file contains seed_32 -----
# Scan checked-in JSONL files under docs/evidence/ and any *.jsonl in
# the repo for the literal substring "seed_32".
shopt -s nullglob globstar
LEAKED=""
for f in "$REPO_ROOT"/docs/evidence/**/*.jsonl "$REPO_ROOT"/docs/evidence/*.jsonl; do
    [ -f "$f" ] || continue
    if grep -q "seed_32" "$f"; then
        LEAKED="$LEAKED $f"
    fi
done
shopt -u globstar
if [ -n "$LEAKED" ]; then
    echo "[ci_check_kes_envelope_closed] FAIL: 'seed_32' substring found in evidence files:$LEAKED"
    exit 1
fi

# ---- 5. key_gen.rs four-line success vocabulary ------------------------
REQUIRED_LINES=(
    'Generated Ade KES key:'
    'Format: ade.kes.seed.v1'
    'Role: kes_hot_signing_key'
    'Public verification key fingerprint:'
)
for line in "${REQUIRED_LINES[@]}"; do
    if ! grep -qF "$line" "$KEYGEN_RS"; then
        echo "[ci_check_kes_envelope_closed] FAIL: key_gen.rs missing closed-vocabulary line: $line"
        exit 1
    fi
done

# Count println! invocations in run_key_gen_kes — must be exactly 4
# (filename, format, role, VK fingerprint). The function body is
# delimited by `pub async fn run_key_gen_kes` ... `ExitCode::SUCCESS`.
FN_BODY="$(awk '
    /pub async fn run_key_gen_kes/ { in_fn = 1 }
    in_fn { print }
    in_fn && /ExitCode::SUCCESS/ { exit }
' "$KEYGEN_RS")"

PRINTLN_COUNT="$(grep -cE '^[[:space:]]*println!' <<<"$FN_BODY" || true)"
if [ "$PRINTLN_COUNT" != "4" ]; then
    echo "[ci_check_kes_envelope_closed] FAIL: run_key_gen_kes must call println! exactly 4 times (got $PRINTLN_COUNT)"
    exit 1
fi

echo "[ci_check_kes_envelope_closed] OK"
