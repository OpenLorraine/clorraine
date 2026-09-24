#!/usr/bin/env bash
# Boot the FS-UAE-only corpus programs in FS-UAE and diff their output.
#
# Two corpus programs need hardware the `vamos` emulator does not provide: the
# interrupt-counter needs the real VBlank interrupt dispatcher and the AmigaOS
# scheduler, and the graphics program needs a real display to open a screen.
# They are the corpus's FS-UAE legs, and both use the same pattern: FS-UAE has
# no automated exit, so the emulator runs under a bounded wait while the script
# polls the captured output for the program's final line, stops the emulator,
# and diffs the capture against the recorded output.
#
# The binaries must already be built (the `corpus-amiga` script builds them);
# this host-side script only runs the emulator, which needs the X display and
# the local SDK settings in `docker/.env`.
#
# Usage (from the repository root):
#   docker/scripts/corpus-fs-uae.sh
#
# Environment overrides:
#   FS_UAE_TIMEOUT  seconds to wait for each captured output (default: 240)
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
docker_dir="$(dirname "${script_dir}")"
repo_dir="$(dirname "${docker_dir}")"
cd "${repo_dir}"

timeout_seconds="${FS_UAE_TIMEOUT:-240}"

if [ ! -f "${docker_dir}/.env" ]; then
    echo "corpus-fs-uae: docker/.env is missing; run 'make -C docker init'" >&2
    exit 2
fi

# run_leg NAME BINARY EXPECTED SENTINEL
#
# Boot BINARY in FS-UAE with output capture, wait until the captured output
# contains SENTINEL (or the emulator exits or the timeout elapses), stop the
# emulator, and diff the normalized capture against EXPECTED.
run_leg() {
    local name="$1"
    local binary="$2"
    local expected="$3"
    local sentinel="$4"
    local capture="build/run/out.txt"
    local log="build/corpus/amiga/${name}/fs-uae.log"

    if [ ! -f "${binary}" ]; then
        echo "corpus-fs-uae: missing ${binary}; run 'make corpus-amiga' first" >&2
        exit 2
    fi

    mkdir -p "$(dirname "${log}")"
    rm -f "${capture}"

    # Run the emulator in the background so the script can watch for the
    # captured output and stop it as soon as the program has finished.
    RUN_CAPTURE=1 timeout "${timeout_seconds}" "${script_dir}/run.sh" "${binary}" \
        >"${log}" 2>&1 &
    local runner=$!

    local found=0
    local deadline=$(( SECONDS + timeout_seconds ))
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
        echo "corpus-fs-uae: ${name}: the emulator produced no complete output" >&2
        echo "corpus-fs-uae: see ${log}" >&2
        tail -20 "${log}" >&2 || true
        return 1
    fi

    tr -d '\r' < "${capture}" > "${capture}.normalized"
    if diff -u "${expected}" "${capture}.normalized"; then
        echo "corpus-fs-uae: ${name} output matches"
    else
        echo "corpus-fs-uae: ${name} output differs" >&2
        return 1
    fi
}

run_leg interrupt-counter \
    "build/corpus/amiga/interrupt-counter/program" \
    "spec/tests/corpus/interrupt-counter/expected-output.txt" \
    "interrupt-counter: ok"

run_leg graphics \
    "build/corpus/amiga/graphics/program" \
    "spec/tests/corpus/graphics/expected-output.txt" \
    "graphics: ok"
