#!/usr/bin/env bash
#
# ci_check_mithril_documented_evidence.sh — RO-MITHRIL-IMPORT-01 gate
# (slice RO-MITHRIL-IMPORT-01-EVIDENCE-SCHEMA; promoted from
# ci/validate_mithril_documented_evidence.sh).
#
# A real, validator-green, operator-witnessed bundle is committed
# (docs/evidence/mithril-documented-evidence_preprod_2026-06-17.toml), so this
# gate is wired onto RO-MITHRIL-IMPORT-01.ci_script and the rule is `enforced`.
# Same no-theater pattern as ci_check_ba02_evidence_manifest_schema.sh: it was
# promoted ONLY AFTER the real fixture landed, never before — and stays honest
# if the bundle is ever removed (vacuous-PASS when none is committed).
#
# Behaviour:
#   * No committed bundle (the typical state — the documented-interface pass
#     is operator-gated): VACUOUSLY satisfied. No bundle, no claim. The
#     obligation remains open; this tool does not assert otherwise.
#   * A committed bundle `docs/evidence/mithril-documented-evidence_*.toml`:
#     STRICTLY validated. Every required field present; schema_version == 1;
#     every COMMITTED artifact (both manifests, consensus-inputs, transcript)
#     exists and its recorded sha256 matches the real file bytes (the "no
#     synthetic bundle" teeth — a hand-authored manifest with no real
#     sha-matching fixtures FAILS); the multi-GB UTxO seed is out-of-tree
#     (hash-pinned, step 3b); the positive episode recorded
#     `binding_result = "pass"` at node exit 0; and the negative control
#     fail-closed with a recognised binding error.
#
# What this proves when green over a committed bundle: the documented
# cardano-cli -> Ade seed_import -> bootstrap_from_mithril_snapshot ->
# verify_mithril_binding chain ran end-to-end on real artifacts, AND the
# binding discriminates (the negative control fail-closed). It does NOT
# pass on "Mithril cert verified" alone.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

TAG="ci_check_mithril_documented_evidence"
BUNDLE_GLOB="docs/evidence/mithril-documented-evidence_*.toml"

# Build the committed-bundle list. A glob that matches nothing must be the
# empty list (vacuous pass), never the literal pattern.
shopt -s nullglob
BUNDLES=( $BUNDLE_GLOB )
shopt -u nullglob

if (( ${#BUNDLES[@]} == 0 )); then
  echo "[$TAG] PASS (no bundle committed; vacuous — RO-MITHRIL-IMPORT-01 item (c) is operator-gated, obligation remains open)"
  exit 0
fi

# Every top-level field the closed bundle schema requires. Mirrors
# docs/evidence/schemas/mithril-documented-evidence.schema.md.
REQUIRED_FIELDS=(
  schema_version
  ade_commit
  network
  captured_by
  mithril_aggregator_endpoint
  mithril_certificate_hash
  mithril_client_version
  cardano_node_version
  cardano_cli_version
  mithril_signed_entity
  mithril_immutable_range_lo
  mithril_immutable_range_hi
  mithril_certified_slot
  mithril_certified_block_hash
  operator_seed_point_slot
  operator_seed_point_block_hash
  utxo_json_file
  utxo_json_sha256
  consensus_inputs_file
  consensus_inputs_sha256
  mithril_manifest_file
  mithril_manifest_sha256
  node_transcript_file
  node_transcript_sha256
  ade_recomputed_seed_artifact_hash
  ade_initial_ledger_fingerprint
  ade_imported_utxo_fingerprint
  binding_result
  node_exit_code
  negative_control_manifest_file
  negative_control_manifest_sha256
  negative_control_outcome
  negative_control_error
  negative_control_node_exit_code
)

# (file-field, sha256-field) pairs the bundle must sha256-bind to a real
# committed artifact. THIS is the no-synthetic-bundle enforcement. The UTxO
# seed is NOT here: it is a multi-GB out-of-tree artifact (gitignored, like the
# repo's existing docs/evidence/*-utxo-seed.json seeds), checked separately at
# step 3b — sha-verified if present locally, hash-pinned if absent.
FILE_SHA_PAIRS=(
  "consensus_inputs_file:consensus_inputs_sha256"
  "mithril_manifest_file:mithril_manifest_sha256"
  "node_transcript_file:node_transcript_sha256"
  "negative_control_manifest_file:negative_control_manifest_sha256"
)

# Closed set of binding failures the negative control may legitimately prove
# (MithrilImportError variants + the FirstRun epoch-window guard). A negative
# control that fails for any OTHER reason does not prove the binding
# discriminates, so it is not accepted.
ALLOWED_NEG_ERRORS="CertifiedPointMismatch EpochMismatch NetworkMagicMismatch GenesisHashMismatch CertificateHashMismatch UnsupportedArtifactType"

# Scalar TOML reader: first `key = <value>`, quotes stripped, trailing
# comment dropped. Bundles are flat key=value TOML (see the template), so a
# grep reader is sufficient and dependency-light (matches the BA-02 gate).
read_field() {
  local key="$1" file="$2"
  grep -E "^${key}[[:space:]]*=" "$file" | head -1 \
    | sed -E "s/^${key}[[:space:]]*=[[:space:]]*//; s/[[:space:]]*(#.*)?$//; s/^\"//; s/\"$//"
}

FAIL=0
fail() { echo "[$TAG] FAIL — $1"; FAIL=1; }

for bundle in "${BUNDLES[@]}"; do
  echo "[$TAG] validating $bundle"
  bundle_dir="$(dirname "$bundle")"

  # 1. Required fields present.
  missing=""
  for f in "${REQUIRED_FIELDS[@]}"; do
    grep -qE "^${f}[[:space:]]*=" "$bundle" || missing+=" $f"
  done
  if [[ -n "$missing" ]]; then
    fail "$bundle missing required field(s):$missing"
    continue
  fi

  # 2. schema_version == 1.
  sv="$(read_field schema_version "$bundle")"
  [[ "$sv" == "1" ]] || fail "$bundle schema_version is '$sv', expected 1"

  # 3. Every (file, sha256) pair binds a real committed artifact by hash.
  for pair in "${FILE_SHA_PAIRS[@]}"; do
    ffield="${pair%%:*}"; sfield="${pair##*:}"
    rel="$(read_field "$ffield" "$bundle")"
    want="$(read_field "$sfield" "$bundle")"
    art="$bundle_dir/$rel"
    if [[ ! -f "$art" ]]; then
      fail "$bundle references missing artifact ($ffield): $art"
      continue
    fi
    got="$(sha256sum "$art" | awk '{print $1}')"
    if [[ "$want" != "$got" ]]; then
      fail "$bundle $sfield mismatch for $art (expected $want, actual $got)"
    fi
  done

  # 3b. The UTxO seed is an out-of-tree large artifact (gitignored, like the
  #     repo's existing *-utxo-seed.json). Its recorded hashes MUST be
  #     well-formed; if the seed file IS present locally, sha-verify it (teeth
  #     when available); if absent, it is hash-pinned + reproducible.
  seed_rel="$(read_field utxo_json_file "$bundle")"
  seed_sha="$(read_field utxo_json_sha256 "$bundle")"
  seed_b2="$(read_field ade_recomputed_seed_artifact_hash "$bundle")"
  [[ "$seed_sha" =~ ^[0-9a-f]{64}$ ]] || fail "$bundle utxo_json_sha256 is not 64-hex ('$seed_sha')"
  [[ "$seed_b2" =~ ^[0-9a-f]{64}$ ]] || fail "$bundle ade_recomputed_seed_artifact_hash is not 64-hex ('$seed_b2')"
  if [[ -f "$bundle_dir/$seed_rel" ]]; then
    seed_got="$(sha256sum "$bundle_dir/$seed_rel" | awk '{print $1}')"
    [[ "$seed_sha" == "$seed_got" ]] || fail "$bundle utxo_json_sha256 mismatch for present seed (expected $seed_sha, actual $seed_got)"
  else
    echo "[$TAG] NOTE — $bundle UTxO seed not present locally ($seed_rel); out-of-tree large artifact, hash-pinned (sha=$seed_sha). Reproduce via the recorded mithril/cardano-cli commands."
  fi

  # 4. Positive episode actually bound: verify_mithril_binding passed at exit 0.
  br="$(read_field binding_result "$bundle")"
  [[ "$br" == "pass" ]] || fail "$bundle binding_result is '$br', expected 'pass'"
  nec="$(read_field node_exit_code "$bundle")"
  [[ "$nec" == "0" ]] || fail "$bundle node_exit_code is '$nec', expected 0"

  # 5. The binding actually held: the cert's certified point equals the
  #    operator's independently-extracted seed point (what verify_mithril_binding
  #    cross-checks). DC-MITHRIL-02 independence is about ORIGIN, enforced in
  #    code / ci_check_mithril_seed_point_independence.sh — the VALUES must
  #    agree for a passing binding.
  cs="$(read_field mithril_certified_slot "$bundle")"
  os="$(read_field operator_seed_point_slot "$bundle")"
  [[ "$cs" == "$os" ]] || fail "$bundle certified_slot ($cs) != operator_seed_point_slot ($os) — binding could not have passed"
  cbh="$(read_field mithril_certified_block_hash "$bundle")"
  obh="$(read_field operator_seed_point_block_hash "$bundle")"
  [[ "$cbh" == "$obh" ]] || fail "$bundle certified_block_hash != operator_seed_point_block_hash — binding could not have passed"

  # 6. Negative control fail-closed with a recognised binding error at nonzero exit.
  nco="$(read_field negative_control_outcome "$bundle")"
  [[ "$nco" == "fail_closed" ]] || fail "$bundle negative_control_outcome is '$nco', expected 'fail_closed'"
  nce="$(read_field negative_control_error "$bundle")"
  if ! grep -qw -- "$nce" <<< "$ALLOWED_NEG_ERRORS"; then
    fail "$bundle negative_control_error '$nce' is not a recognised binding failure ($ALLOWED_NEG_ERRORS)"
  fi
  ncec="$(read_field negative_control_node_exit_code "$bundle")"
  [[ "$ncec" != "0" && -n "$ncec" ]] || fail "$bundle negative_control_node_exit_code is '$ncec', expected nonzero"

  # 7. Advisory: if b2sum is available, cross-check Ade's recomputed
  #    seed_artifact_hash against an independent BLAKE2b-256 of utxo.json.
  #    Advisory (NOTE, not FAIL) only because hash-domain equivalence with
  #    Ade's blake2b_256_of_file is not asserted here; the authoritative
  #    equality is captured at capture time.
  if command -v b2sum >/dev/null 2>&1; then
    utxo_rel="$(read_field utxo_json_file "$bundle")"
    ade_sah="$(read_field ade_recomputed_seed_artifact_hash "$bundle")"
    indep="$(b2sum -l 256 "$bundle_dir/$utxo_rel" 2>/dev/null | awk '{print $1}' || true)"
    if [[ -n "$indep" && "$indep" != "$ade_sah" ]]; then
      echo "[$TAG] NOTE — $bundle ade_recomputed_seed_artifact_hash ($ade_sah) != independent b2sum -l 256 ($indep); confirm hash domain (advisory)"
    fi
  fi
done

if (( FAIL )); then
  echo "[$TAG] FAIL — one or more committed bundles are invalid"
  exit 1
fi

echo "[$TAG] PASS (${#BUNDLES[@]} committed bundle(s) schema-valid, sha256-bound, binding=pass, negative-control fail-closed)"
