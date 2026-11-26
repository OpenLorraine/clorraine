#!/usr/bin/env bash
# A500/Kickstart 1.3 fault-cleanup gate.
#
# Boots a copy of the licensed Workbench 1.3.5 HDF on a real A500/KS1.3 and
# runs the fault-cleanup probes from its startup-sequence. Each probe acquires
# OS resources (an interrupt server, a public port, a custom display, a screen
# with a window and a console, a disk font, or a layer, copper list, sprites,
# an audio channel, and a sample) and then faults, or returns normally without
# releasing them. Two probes fault inside a worker task, which the parent
# reports. A test-only walker records the system state before and after
# every probe; the runtime must leave it exactly as it found it, and each
# probe must print its expected output and leave with its expected rc.
#
# FS-UAE keeps the HDF locked while it runs, so every walker line, every
# probe's output, and every rc marker is redirected to a blank capture ADF in
# DF1. DF0 stays empty so Kickstart 1.3 boots the HDF. The host polls the raw
# ADF for the end marker while the emulator runs and unpacks the files after
# it stops. Runs on a private Xvfb display; no window opens on the desktop.
#
# Usage:
#   docker/scripts/a500-fault-cleanup-smoke.sh
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
docker_dir="$(dirname "${script_dir}")"
repo_dir="$(dirname "${docker_dir}")"
cd "${repo_dir}"

work="${repo_dir}/build/amiga-sdk/a500-fault-cleanup"
work_rel="${work#"${repo_dir}/"}"
hdf_rel="${work_rel}/workbench.hdf"
adf_rel="${work_rel}/capture.adf"
rom_rel="${work_rel}/ks13.rom"
container="clorraine-a500-fault-cleanup-$$"
done_marker="fault-cleanup-done"

# Each entry is `<probe>|<expected output>|<rc>|<chip>`. The expected output
# is the probe's whole output, with `\n` between lines. The rc is `fail` when
# the probe must leave with a failing return code (20, after a fault) and `ok`
# when it must return normally. The memory column is `chip` for the one probe
# whose free chip and total memory must also come back exactly (it loads only
# ROM-resident code); every other probe uses `-`.
CASES=(
    "display-fault|error E11211: index out of range|fail|-"
    "window-fault|error E11211: index out of range|fail|-"
    "font-fault|error E11211: index out of range|fail|-"
    "device-fault|error E11211: index out of range|fail|chip"
    "handler-fault|error E11211: index out of range|fail|-"
    "clean-exit|clean|ok|-"
    "worker-fault|error E11210: integer division by zero|fail|-"
    "worker-fault-unjoined|error E11211: index out of range\nerror E11210: integer division by zero|fail|-"
)

if [ ! -x "${repo_dir}/target/debug/clc" ]; then
    echo "a500-fault-cleanup-smoke: compiler not found; run make sdk-a500-fault-cleanup" >&2
    exit 2
fi
if [ ! -f "${docker_dir}/.env" ]; then
    echo "a500-fault-cleanup-smoke: docker/.env is missing; run 'make -C docker init'" >&2
    exit 2
fi
set -a
# shellcheck disable=SC1090
. "${docker_dir}/.env"
set +a
image="${IMAGE:-clorraine-sdk:dev}"
if [ -z "${AMIGA_DATA:-}" ]; then
    echo "a500-fault-cleanup-smoke: AMIGA_DATA is not set in docker/.env" >&2
    exit 2
fi
if [ ! -f "${AMIGA_DATA}/hdf/workbench-135.hdf" ]; then
    echo "a500-fault-cleanup-smoke: missing Workbench 1.3.5 HDF under ${AMIGA_DATA}/hdf" >&2
    exit 2
fi
for tool in sha1sum; do
    command -v "${tool}" >/dev/null 2>&1 \
        || { echo "a500-fault-cleanup-smoke: missing host tool '${tool}'" >&2; exit 2; }
done

xvfb_num=$(( ($$ % 50) + 60 ))
while [ -e "/tmp/.X11-unix/X${xvfb_num}" ]; do
    xvfb_num=$((xvfb_num + 1))
done
xvfb_display=":${xvfb_num}"
xvfb_container="clorraine-$(basename "${BASH_SOURCE[0]}" .sh)-xvfb-$$"

cleanup() {
    docker rm -f "${container}" "${xvfb_container}" >/dev/null 2>&1 || true
    rm -f "/tmp/.X11-unix/X${xvfb_display#:}"
    rm -f "${repo_dir}/${rom_rel}"
}
trap cleanup EXIT

# Run a command in the SDK container with the repository at /work.
sdk() {
    (
        cd "${docker_dir}"
        docker compose -p clorraine --env-file .env -f compose.yaml run --rm -T dev "$@"
    )
}

rm -rf "${work}"
mkdir -p "${work}/bin" "${work}/capture"

# Build the walker and every probe for Kickstart 1.3 with the release line of
# the compile matrix, as the other A500 gates do.
{
    echo "set -euo pipefail"
    echo "m68k-amigaos-gcc -std=c99 -mcrt=nix13 -m68000 -O2 -Wall -Wextra -Werror -s \\"
    echo "    -o ${work_rel}/bin/walker docker/scripts/fault-cleanup-walker.c"
    for entry in "${CASES[@]}"; do
        name="${entry%%|*}"
        echo "./target/debug/clc --profile hosted-amiga-13 -o ${work_rel}/bin/${name}.c \\"
        echo "    examples/robustness/fault-cleanup/${name}.clo"
        echo "m68k-amigaos-gcc -std=c99 -mcrt=nix13 -m68000 -fwrapv -O2 -ffunction-sections \\"
        echo "    -fdata-sections -Wl,--gc-sections -s -o ${work_rel}/bin/${name} \\"
        echo "    ${work_rel}/bin/${name}.c -lamiga -lm"
    done
} > "${work}/build.sh"
sdk bash "/work/${work_rel}/build.sh"

# The startup-sequence brackets every probe with the walker. FailAt 100 keeps
# the script running after a probe's failing rc; IF FAIL records it.
{
    echo "FailAt 100"
    echo "Assign FONTS: SYS:Fonts"
    index=0
    for entry in "${CASES[@]}"; do
        index=$((index + 1))
        id="$(printf '%02d' "${index}")"
        name="${entry%%|*}"
        echo "SYS:fc/walker >DF1:${id}.wb before"
        echo "SYS:fc/${name} >DF1:${id}.out"
        echo "IF FAIL"
        echo "  Echo >DF1:${id}.rc \"rc fail\""
        echo "EndIF"
        echo "SYS:fc/walker >DF1:${id}.wa after"
    done
    echo "Echo >DF1:done \"${done_marker}\""
} > "${work}/startup-sequence"

cp "${AMIGA_DATA}/hdf/workbench-135.hdf" "${repo_dir}/${hdf_rel}"
chmod u+w "${repo_dir}/${hdf_rel}"
hdf_commands=(makedir fc + write "/work/${work_rel}/bin/walker" fc/walker + protect fc/walker +re)
for entry in "${CASES[@]}"; do
    name="${entry%%|*}"
    hdf_commands+=(+ write "/work/${work_rel}/bin/${name}" "fc/${name}" + protect "fc/${name}" +re)
done
hdf_commands+=(+ delete S/startup-sequence + write "/work/${work_rel}/startup-sequence" S/startup-sequence)
sdk xdftool "/work/${hdf_rel}" "${hdf_commands[@]}" >/dev/null

# A blank, unbootable capture disk for DF1.
sdk xdftool "/work/${adf_rel}" create + format Capture >/dev/null

sdk python3 -c "from amitools.rom.kickrom import Loader; data = Loader.load('/amiga/roms/amiga-os-130.rom', '/amiga/roms/rom.key'); open('/work/${rom_rel}', 'wb').write(data)"
sha1="$(sha1sum "${repo_dir}/${rom_rel}" | awk '{print $1}')"
if [ "${sha1}" != "891e9a547772fe0c6c19b610baf8bc4ea7fcb785" ]; then
    echo "a500-fault-cleanup-smoke: decrypted ROM has unexpected SHA-1 ${sha1}" >&2
    exit 1
fi

# Run Xvfb inside the SDK image so FS-UAE gets a compatible GLX context.
docker run -d --rm --name "${xvfb_container}" \
    -v /tmp/.X11-unix:/tmp/.X11-unix \
    "${image}" Xvfb "${xvfb_display}" \
    -screen 0 1280x720x24 -ac +extension GLX -noreset >/dev/null
for _ in $(seq 1 100); do
    [ -S "/tmp/.X11-unix/X${xvfb_display#:}" ] && break
    sleep 0.1
done
if [ ! -S "/tmp/.X11-unix/X${xvfb_display#:}" ]; then
    echo "a500-fault-cleanup-smoke: the private Xvfb did not start" >&2
    docker logs "${xvfb_container}" >&2 2>/dev/null || true
    exit 1
fi
export DISPLAY="${xvfb_display}"
unset XAUTHORITY

docker run --rm --name "${container}" \
    -e DISPLAY="${xvfb_display}" \
    -e SDL_VIDEO_X11_XINPUT2=0 \
    -v "${repo_dir}":/work \
    -v "${AMIGA_DATA}":/amiga:ro \
    -v /tmp/.X11-unix:/tmp/.X11-unix \
    "${image}" fs-uae \
    --amiga-model=A500 \
    --kickstart-file="/work/${rom_rel}" \
    --hard-drive-0="/work/${hdf_rel}" \
    --hard-drive-0-label=DH0 \
    --hard-drive-0-priority=0 \
    --floppy-drive-count=2 \
    --floppy-drive-1="/work/${adf_rel}" \
    --writable-floppy-images=1 \
    --floppy-drive-speed=0 \
    --window-width=640 --window-height=256 --window-border=0 \
    --scale-x=1 --scale-y=1 --crop=1 \
    > "${work}/fs-uae.log" 2>&1 &
runner=$!

# The A500 reaches the Shell in roughly forty seconds; each probe and its two
# walker runs take a few seconds more. The marker is the last line of the
# startup-sequence, so its data on the capture disk means every probe ran.
finished=0
for _ in $(seq 1 300); do
    if grep -aq "${done_marker}" "${repo_dir}/${adf_rel}" 2>/dev/null; then
        finished=1
        break
    fi
    sleep 1
done
# The marker's data block reaches the ADF before AmigaDOS writes the last
# header blocks, which wait in the floppy track buffer until the drive goes
# idle, so stop only once the image has been unchanged for three seconds.
last_sum=""
stable=0
for _ in $(seq 1 30); do
    sum="$(sha1sum "${repo_dir}/${adf_rel}" | awk '{print $1}')"
    if [ "${sum}" = "${last_sum}" ]; then
        stable=$((stable + 1))
        [ "${stable}" -ge 3 ] && break
    else
        stable=0
        last_sum="${sum}"
    fi
    sleep 1
done
docker stop -t 8 "${container}" >/dev/null 2>&1 || true
wait "${runner}" 2>/dev/null || true

# A machine that crashed mid-write leaves a damaged capture disk. Report it
# and go on: every case whose files are missing then fails by name.
if ! sdk xdftool "/work/${adf_rel}" unpack "/work/${work_rel}/capture" \
        > "${work}/unpack.log" 2>&1; then
    echo "a500-fault-cleanup-smoke: the capture disk is damaged: $(tail -1 "${work}/unpack.log")" >&2
fi
capture="${work}/capture/Capture"
[ -d "${capture}" ] || capture="${work}/capture"

# The text of one capture file without Amiga line ends, or empty.
captured() {
    if [ -f "${capture}/$1" ]; then
        tr -d '\r' < "${capture}/$1"
    fi
}

status=0
index=0
for entry in "${CASES[@]}"; do
    index=$((index + 1))
    id="$(printf '%02d' "${index}")"
    name="${entry%%|*}"
    rest="${entry#*|}"
    want_output="$(printf '%b' "${rest%%|*}")"
    rest="${rest#*|}"
    want_rc="${rest%%|*}"
    want_chip="${rest#*|}"
    before="$(captured "${id}.wb")"
    output="$(captured "${id}.out")"
    rc_line="$(captured "${id}.rc")"
    after="$(captured "${id}.wa")"
    # The memory fields come last; keep them only where they are asserted.
    if [ "${want_chip}" != "chip" ]; then
        before="${before% chip *}"
        after="${after% chip *}"
    fi

    case_ok=1
    if [ -z "${before}" ] || [ -z "${after}" ]; then
        echo "a500-fault-cleanup-smoke: ${name}: missing walker line (before '${before}', after '${after}')" >&2
        case_ok=0
    elif [ "${before#before }" != "${after#after }" ]; then
        echo "a500-fault-cleanup-smoke: ${name}: system state changed" >&2
        echo "  ${before}" >&2
        echo "  ${after}" >&2
        case_ok=0
    fi
    if [ "${output}" != "${want_output}" ]; then
        echo "a500-fault-cleanup-smoke: ${name}: output mismatch" >&2
        echo "  expected: ${want_output}" >&2
        echo "  actual:   ${output}" >&2
        case_ok=0
    fi
    if [ "${want_rc}" = "fail" ] && [ "${rc_line}" != "rc fail" ]; then
        echo "a500-fault-cleanup-smoke: ${name}: expected a failing rc" >&2
        case_ok=0
    elif [ "${want_rc}" = "ok" ] && [ -n "${rc_line}" ]; then
        echo "a500-fault-cleanup-smoke: ${name}: unexpected failing rc" >&2
        case_ok=0
    fi
    if [ "${case_ok}" -eq 1 ]; then
        echo "a500-fault-cleanup-smoke: ${name}: ok (${after#after })"
    else
        status=1
    fi
done

if [ "${finished}" -ne 1 ]; then
    echo "a500-fault-cleanup-smoke: the startup-sequence never finished (timeout or crash)" >&2
    tail -20 "${work}/fs-uae.log" >&2 || true
    status=1
fi
if [ "${status}" -ne 0 ]; then
    echo "a500-fault-cleanup-smoke: FAILED" >&2
    exit 1
fi
echo "a500-fault-cleanup-smoke: OK"
