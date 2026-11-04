#!/usr/bin/env bash
# Tests for the Amiga SDK registry tooling.
#
# The registry is the long-lived source of truth, so its validator must reject
# malformed rows instead of silently accepting them. This script uses only
# POSIX shell tools and a temporary copy of the registry; it never edits the
# tracked files.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
tool="${repo_root}/tools/amiga-sdk/registry.sh"
registry="${repo_root}/docs/amiga-sdk/registry.tsv"
bundles="${repo_root}/docs/amiga-sdk/bundles.tsv"
coverage="${repo_root}/docs/amiga-sdk/coverage.md"

if [ ! -x "${tool}" ]; then
    echo "registry_test: missing executable ${tool}" >&2
    exit 1
fi

tmp_root="${CODEOPS_TMPDIR:-${TMPDIR:-/tmp}}"
scratch="$(mktemp -d "${tmp_root}/clorraine-registry-test.XXXXXX")"
trap 'rm -rf "${scratch}"' EXIT

fail() {
    echo "registry_test: $*" >&2
    exit 1
}

# The committed registry and generated coverage must agree.
"${tool}" check
"${tool}" coverage > "${scratch}/coverage.md"
diff -u "${coverage}" "${scratch}/coverage.md" >/dev/null \
    || fail "coverage.md is stale; run 'make sdk-registry'"

# A duplicate id is rejected.
cp "${registry}" "${scratch}/duplicate.tsv"
awk -F '\t' 'NR == 1 { print; next } NR == 2 { print; print; next } { print }' \
    "${registry}" > "${scratch}/duplicate.tsv"
if REGISTRY_FILE="${scratch}/duplicate.tsv" BUNDLES_FILE="${bundles}" \
    "${tool}" check >/dev/null 2>&1; then
    fail "a duplicate id was accepted"
fi

# An unknown status is rejected.
awk -F '\t' 'NR == 1 { print; next } NR == 2 { $9 = "made-up"; print; next } { print }' \
    "${registry}" > "${scratch}/status.tsv"
if REGISTRY_FILE="${scratch}/status.tsv" BUNDLES_FILE="${bundles}" \
    "${tool}" check >/dev/null 2>&1; then
    fail "an unknown status was accepted"
fi

# A missing dependency is rejected.
awk -F '\t' 'NR == 1 { print; next } NR == 2 { $10 = "SDK-NOPE-999"; print; next } { print }' \
    "${registry}" > "${scratch}/dependency.tsv"
if REGISTRY_FILE="${scratch}/dependency.tsv" BUNDLES_FILE="${bundles}" \
    "${tool}" check >/dev/null 2>&1; then
    fail "a missing dependency was accepted"
fi

# An unknown bundle is rejected.
awk -F '\t' 'NR == 1 { print; next } NR == 2 { $2 = "not-a-bundle"; print; next } { print }' \
    "${registry}" > "${scratch}/bundle.tsv"
if REGISTRY_FILE="${scratch}/bundle.tsv" BUNDLES_FILE="${bundles}" \
    "${tool}" check >/dev/null 2>&1; then
    fail "an unknown bundle was accepted"
fi

# A must row cannot be out-of-scope.
awk -F '\t' 'NR == 1 { print; next } NR == 2 { $5 = "must"; $9 = "out-of-scope"; print; next } { print }' \
    "${registry}" > "${scratch}/must-scope.tsv"
if REGISTRY_FILE="${scratch}/must-scope.tsv" BUNDLES_FILE="${bundles}" \
    "${tool}" check >/dev/null 2>&1; then
    fail "a must/out-of-scope row was accepted"
fi

# A done row without acceptance or evidence is rejected (both missing, and one missing).
for variant in both one; do
    if [ "${variant}" = "both" ]; then
        awk -F '\t' 'NR == 1 { print; next } NR == 2 { $9 = "done"; $11 = "-"; $12 = "-"; print; next } { print }' \
            "${registry}" > "${scratch}/done-${variant}.tsv"
    else
        awk -F '\t' 'NR == 1 { print; next } NR == 2 { $9 = "done"; $11 = "-"; $12 = "evidence"; print; next } { print }' \
            "${registry}" > "${scratch}/done-${variant}.tsv"
    fi
    if REGISTRY_FILE="${scratch}/done-${variant}.tsv" BUNDLES_FILE="${bundles}" \
        "${tool}" check >/dev/null 2>&1; then
        fail "a done row without ${variant} acceptance/evidence was accepted"
    fi
done

# Every bundle gets an issue body with at least one checklist row.
"${tool}" issues "${scratch}/issues"
bundle_count="$(awk -F '\t' 'NR > 1 && NF > 0 && $1 !~ /^#/ { n++ } END { print n + 0 }' "${bundles}")"
body_count="$(find "${scratch}/issues" -type f -name '*.md' | wc -l | tr -d ' ')"
[ "${body_count}" -eq "${bundle_count}" ] \
    || fail "expected ${bundle_count} issue bodies, got ${body_count}"
for body in "${scratch}/issues"/*.md; do
    grep -q '^- \[[ x]\] `SDK-' "${body}" \
        || fail "issue body ${body} has no capability checklist"
done

echo "registry_test: OK (${bundle_count} bundles)"
