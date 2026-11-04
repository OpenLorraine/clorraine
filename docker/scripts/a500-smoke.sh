#!/usr/bin/env bash
# A500/Kickstart 1.3 build and load/run smoke.
#
# Runs a binary built with `--profile hosted-amiga-13` under `vamos`, the
# 68000 Amiga emulator used by the driver's fast target leg, and checks the
# expected line. The FS-UAE A500/KS1.3 visual run is a manual hardware check:
# the stock A500 has no bootable hard-drive filesystem, and the ADF serial
# capture harness is deferred (see the Bundle 0 findings).
#
# Usage:
#   scripts/a500-smoke.sh <binary> ["expected line"]
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
docker_dir="$(dirname "${script_dir}")"
repo_dir="$(dirname "${docker_dir}")"
cd "${repo_dir}"

binary="${1:-}"
expected="${2:-Hello, Amiga, from Clorraine!}"

if [ -z "${binary}" ]; then
    echo "usage: $0 <binary> [expected line]" >&2
    exit 2
fi
if [ ! -f "${binary}" ]; then
    echo "a500-smoke: missing binary: ${binary}" >&2
    echo "a500-smoke: build it with --profile hosted-amiga-13 first" >&2
    exit 2
fi
case "${binary}" in
    /*) echo "a500-smoke: pass a repository-relative binary path" >&2; exit 2 ;;
esac

output="$(timeout 120 make -C docker exec CMD="vamos /work/${binary}" 2>&1)" || {
    echo "a500-smoke: the 1.3 program failed to run" >&2
    printf '%s\n' "${output}" >&2
    exit 1
}
normalized="$(printf '%s\n' "${output}" | tr -d '\r')"
if ! printf '%s\n' "${normalized}" | grep -qxF "${expected}"; then
    echo "a500-smoke: output differs" >&2
    echo "--- expected ---" >&2
    printf '%s\n' "${expected}" >&2
    echo "--- actual ---" >&2
    printf '%s\n' "${normalized}" >&2
    exit 1
fi

echo "a500-smoke: OK (nix13 build + vamos load/run: ${expected})"
echo "a500-smoke: FS-UAE A500/KS1.3 visual check is manual; see the plan findings"
