#!/usr/bin/env bash
# Boot the interrupt-counter corpus program in FS-UAE and diff its output.
#
# The interrupt-counter is the only corpus program that needs the real VBlank
# interrupt dispatcher and the AmigaOS scheduler, so it cannot run under
# `vamos`; it is the corpus's FS-UAE leg. FS-UAE has no automated exit, so the
# emulator runs under a bounded wait: the script polls the captured output for
# the program's final line, stops the emulator, and diffs the capture against
# the recorded output.
#
# The binary must already be built (the `corpus-amiga` script builds it); this
# host-side script only runs the emulator, which needs the X display and the
# local SDK settings in `docker/.env`.
#
# Usage (from the repository root):
#   docker/scripts/corpus-fs-uae.sh
#
# Environment overrides:
#   FS_UAE_TIMEOUT  seconds to wait for the captured output (default: 240)
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
docker_dir="$(dirname "${script_dir}")"
repo_dir="$(dirname "${docker_dir}")"
cd "${repo_dir}"

binary="build/corpus/amiga/interrupt-counter/program"
expected="spec/tests/corpus/interrupt-counter/expected-output.txt"
capture="build/run/out.txt"
log="build/corpus/amiga/interrupt-counter/fs-uae.log"
sentinel="interrupt-counter: ok"
timeout_seconds="${FS_UAE_TIMEOUT:-240}"

if [ ! -f "${binary}" ]; then
    echo "corpus-fs-uae: missing ${binary}; run 'make corpus-amiga' first" >&2
    exit 2
fi
if [ ! -f "${docker_dir}/.env" ]; then
    echo "corpus-fs-uae: docker/.env is missing; run 'make -C docker init'" >&2
    exit 2
fi

mkdir -p "$(dirname "${log}")"
rm -f "${capture}"

# Run the emulator in the background so the script can watch for the captured
# output and stop it as soon as the program has finished.
RUN_CAPTURE=1 timeout "${timeout_seconds}" "${script_dir}/run.sh" "${binary}" \
    >"${log}" 2>&1 &
runner=$!

found=0
deadline=$(( SECONDS + timeout_seconds ))
while [ "${SECONDS}" -lt "${deadline}" ]; do
    if [ -f "${capture}" ] && grep -qF "${sentinel}" "${capture}"; then
        found=1
        break
    fi
    if ! kill -0 "${runner}" 2>/dev/null; then
        break
    fi
    sleep 2
done

kill "${runner}" 2>/dev/null || true
wait "${runner}" 2>/dev/null || true

if [ "${found}" -ne 1 ]; then
    echo "corpus-fs-uae: the emulator produced no complete output" >&2
    echo "corpus-fs-uae: see ${log}" >&2
    tail -20 "${log}" >&2 || true
    exit 1
fi

tr -d '\r' < "${capture}" > "${capture}.normalized"
if diff -u "${expected}" "${capture}.normalized"; then
    echo "corpus-fs-uae: interrupt-counter output matches"
else
    echo "corpus-fs-uae: interrupt-counter output differs" >&2
    exit 1
fi
