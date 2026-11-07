#!/usr/bin/env bash
# Build the compiler at one recorded commit and run the current benchmark
# suite against it.
#
# The harness, the benchmark sources, and the toolchain come from the current
# tree; only `clc` comes from the base commit. That is the base/head
# comparison: the compiler is the variable. The base binary is retained as
# build/perf/base-clc for the emission audit.
#
# Run inside the pinned SDK container from the repository root:
#   docker/scripts/bench-base.sh <40-hex-commit>
#
# Environment:
#   BASE_COMMIT  canonical 40-hex commit (passed by `make bench-base`)
set -euo pipefail

COMMIT="${BASE_COMMIT:-${COMMIT:-${1:-}}}"
if ! printf '%s' "${COMMIT}" | grep -Eq '^[0-9a-f]{40}$'; then
    echo "bench-base: expected a canonical 40-hex commit, got '${COMMIT}'" >&2
    exit 2
fi

ROOT="$(pwd)"
WORK="${ROOT}/build/perf/base-worktree"
BASE_CLC="${ROOT}/build/perf/base-clc"
mkdir -p "${ROOT}/build/perf"

if [ -L "${WORK}" ]; then
    echo "bench-base: refusing symlinked worktree: ${WORK}" >&2
    exit 2
fi
# Recover from an interrupted run: remove a registered worktree and prune any
# stale registration before adding the new one.
git worktree remove --force "${WORK}" 2>/dev/null || true
git worktree prune
rm -rf "${WORK}"

cleanup() {
    git worktree remove --force "${WORK}" 2>/dev/null || true
    git worktree prune
    rm -rf "${WORK}"
}
trap cleanup EXIT

git worktree add --detach "${WORK}" "${COMMIT}"
(
    cd "${WORK}"
    cargo build --workspace --quiet
)

cp "${WORK}/target/debug/clc" "${BASE_CLC}"
chmod +x "${BASE_CLC}"

python3 tools/perf/bench.py run \
    --clc "${WORK}/target/debug/clc" \
    --compiler-git "${COMMIT}" \
    --label base \
    --out "${ROOT}/build/perf/base.json"

echo "bench-base: wrote build/perf/base.json and retained build/perf/base-clc"
