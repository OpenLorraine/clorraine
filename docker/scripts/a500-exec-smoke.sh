#!/usr/bin/env bash
# A500/Kickstart 1.3 Exec smoke.
#
# Decrypts the user's licensed Amiga Forever Kickstart 1.3 ROM locally (the
# decrypted image stays in gitignored build/ and is removed on exit), boots the
# Exec demo from a clean ADF under FS-UAE configured as an A500, waits for the
# report file, and compares the visible report line with the committed golden.
#
# The demo is a single console command with one output line, so the line crop
# uses the same origin as the hello gate. The crop must be non-blank and
# byte-exact; the report file must match the pinned line. No user input is
# needed. The gate uses the caller's private Xvfb display.
#
# Usage:
#   scripts/a500-exec-smoke.sh [binary] [expected line]
#   CAPTURE_GOLDEN=1 scripts/a500-exec-smoke.sh [binary]   # one-time golden
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
docker_dir="$(dirname "${script_dir}")"
repo_dir="$(dirname "${docker_dir}")"
cd "${repo_dir}"

binary="${1:-build/clorraine/exec-demo/hosted-amiga-13/exec-demo}"
expected="${2:-exec: ok task 464 lib 34 dev 512 io 13 key -2 in 1}"
golden="${repo_dir}/docs/amiga-sdk/a500-exec.png"
work="${repo_dir}/build/amiga-sdk/exec-smoke"
work_rel="${work#"${repo_dir}/"}"
adf="${work_rel}/exec.adf"
rom="${work_rel}/ks13.rom"
screenshot="${work}/screen.png"
crop="${work}/crop.png"
container="clorraine-a500-exec-$$"
capture_golden="${CAPTURE_GOLDEN:-0}"

if [ ! -f "${binary}" ]; then
    echo "a500-exec-smoke: missing binary: ${binary}" >&2
    echo "a500-exec-smoke: build it with --profile hosted-amiga-13 first" >&2
    exit 2
fi
if [ ! -f "${docker_dir}/.env" ]; then
    echo "a500-exec-smoke: docker/.env is missing; run 'make -C docker init'" >&2
    exit 2
fi
if [ "${capture_golden}" != "1" ] && [ ! -f "${golden}" ]; then
    echo "a500-exec-smoke: missing golden: ${golden}" >&2
    echo "a500-exec-smoke: run with CAPTURE_GOLDEN=1 once to capture it" >&2
    exit 2
fi
for tool in xdotool import convert compare identify sha1sum; do
    command -v "${tool}" >/dev/null 2>&1 \
        || { echo "a500-exec-smoke: missing host tool '${tool}'" >&2; exit 2; }
done

set -a
# shellcheck disable=SC1090
. "${docker_dir}/.env"
set +a
image="${IMAGE:-clorraine-sdk:dev}"

cleanup() {
    docker rm -f "${container}" >/dev/null 2>&1 || true
    rm -f "${repo_dir}/${rom}"
}
trap cleanup EXIT

mkdir -p "${work}/S"
rm -f "${screenshot}" "${crop}" "${repo_dir}/${adf}" "${repo_dir}/${rom}"
cp "${binary}" "${work}/exec"
printf 'DF0:exec\n' > "${work}/startup-sequence"
cp "${work}/startup-sequence" "${work}/S/startup-sequence"

# Decrypt the licensed Cloanto ROM locally. The official A500 Kickstart 1.3
# SHA-1 is 891e9a547772fe0c6c19b610baf8bc4ea7fcb785.
(
    cd "${docker_dir}"
    docker compose -p clorraine --env-file .env -f compose.yaml run --rm -T dev \
        python3 -c "from amitools.rom.kickrom import Loader; data = Loader.load('/amiga/roms/amiga-os-130.rom', '/amiga/roms/rom.key'); open('/work/${rom}', 'wb').write(data)"
)
sha1="$(sha1sum "${repo_dir}/${rom}" | awk '{print $1}')"
if [ "${sha1}" != "891e9a547772fe0c6c19b610baf8bc4ea7fcb785" ]; then
    echo "a500-exec-smoke: decrypted ROM has unexpected SHA-1 ${sha1}" >&2
    exit 1
fi

# Build a clean ADF. The bootblock executes the root startup-sequence on 1.3.
xdftool() {
    (
        cd "${docker_dir}"
        docker compose -p clorraine --env-file .env -f compose.yaml run --rm -T dev \
            xdftool "/work/${adf}" "$@"
    )
}
xdftool create + format CNEXT >/dev/null
xdftool write "/work/${work_rel}/exec" exec + protect exec +re >/dev/null
xdftool write "/work/${work_rel}/startup-sequence" startup-sequence >/dev/null
xdftool makedir S + write "/work/${work_rel}/S/startup-sequence" S/startup-sequence >/dev/null
xdftool boot install boot1x >/dev/null
if ! xdftool list | grep -q ' startup-sequence'; then
    echo "a500-exec-smoke: the ADF has no root startup-sequence" >&2
    exit 1
fi

# Boot the real A500/KS1.3 machine. Poll from 10 s, re-find the window on every
# iteration, and only crop once the report file is visible. The 60-cycle bound
# is generous for a loaded host.
XAUTHORITY="${XAUTHORITY}" DISPLAY="${DISPLAY}"
docker run --rm --name "${container}" \
    -e DISPLAY="${DISPLAY}" -e XAUTHORITY=/tmp/.Xauthority \
    -v "${repo_dir}":/work \
    -v "${AMIGA_DATA}":/amiga:ro \
    -v /tmp/.X11-unix:/tmp/.X11-unix \
    -v "${XAUTHORITY}":/tmp/.Xauthority:ro \
    "${image}" fs-uae \
    --amiga-model=A500 \
    --kickstart-file="/work/${rom}" \
    --floppy-drive-0="/work/${adf}" \
    --writable-floppy-images=1 \
    > "${work}/fs-uae.log" 2>&1 &
runner=$!
sleep 10

window=""
window_seen=0
reported=0
line_matched=0
step=0
while [ "${step}" -lt 60 ]; do
    step=$((step + 1))
    sleep 2
    found="$(DISPLAY="${DISPLAY}" xdotool search --name 'FS-UAE' 2>/dev/null | tail -1 || true)"
    if [ -n "${found}" ]; then
        window="${found}"
        window_seen=1
    fi
    if [ -z "${window}" ]; then
        continue
    fi
    if ! DISPLAY="${DISPLAY}" import -window "${window}" "${screenshot}" 2>/dev/null; then
        continue
    fi
    if [ "${reported}" -eq 0 ] &&
       grep -aq "exec: ok task 464" "${repo_dir}/${adf}" 2>/dev/null; then
        reported=1
        # Let the Shell draw the line after the file is committed.
        sleep 1
        DISPLAY="${DISPLAY}" import -window "${window}" "${screenshot}" 2>/dev/null || true
    fi
    if [ "${reported}" -eq 0 ]; then
        continue
    fi
    convert "${screenshot}" -crop 620x22+160+88 +repage "${crop}"
    if [ "${capture_golden}" = "1" ]; then
        line_matched=1
        break
    fi
    metric="$(compare -metric AE "${crop}" "${golden}" null: 2>&1 || true)"
    metric="${metric%% *}"
    if [ "${metric}" = "0" ]; then
        line_matched=1
        break
    fi
done

if [ "${window_seen}" -eq 0 ]; then
    echo "a500-exec-smoke: no FS-UAE window" >&2
    tail -20 "${work}/fs-uae.log" >&2 || true
    exit 1
fi
if [ "${reported}" -eq 0 ]; then
    echo "a500-exec-smoke: the demo never wrote the report line" >&2
    echo "a500-exec-smoke: last screen kept at ${screenshot}" >&2
    xdftool list >&2 || true
    exit 1
fi

docker stop -t 5 "${container}" >/dev/null 2>&1 || true
wait "${runner}" 2>/dev/null || true

if ! xdftool read exec.txt "/work/${work_rel}/exec.txt" >/dev/null 2>&1; then
    echo "a500-exec-smoke: exec.txt is not in the ADF" >&2
    xdftool list >&2 || true
    exit 1
fi
actual="$(cat "${work}/exec.txt")"
if [ "${actual}" != "${expected}" ]; then
    echo "a500-exec-smoke: report line mismatch" >&2
    echo "  expected: ${expected}" >&2
    echo "  actual:   ${actual}" >&2
    exit 1
fi

colors="$(identify -format '%k' "${crop}" 2>/dev/null || echo 0)"
if [ "${colors}" -lt 2 ] 2>/dev/null; then
    echo "a500-exec-smoke: the line crop is blank" >&2
    echo "a500-exec-smoke: last screen kept at ${screenshot}" >&2
    exit 1
fi

if [ "${capture_golden}" = "1" ]; then
    mkdir -p "$(dirname "${golden}")"
    cp "${crop}" "${golden}"
    echo "a500-exec-smoke: captured golden ${golden}"
    exit 0
fi

if [ "${line_matched}" -ne 1 ]; then
    metric="$(compare -metric AE "${crop}" "${golden}" null: 2>&1 || true)"
    metric="${metric%% *}"
    echo "a500-exec-smoke: captured line differs from ${golden} (AE=${metric})" >&2
    echo "a500-exec-smoke: last screen kept at ${screenshot}" >&2
    exit 1
fi

echo "a500-exec-smoke: OK (report line: ${actual}; line crop: ${golden})"
