#!/usr/bin/env bash
# A1200/Kickstart 3.1 Workbench and services smoke.
#
# Boots a clean directory hard drive on a real A1200/KS3.1 under FS-UAE, starts
# the ROM-resident Workbench with the test-only fixture, runs the Workbench and
# services demos, posts the three AppMessages, asserts the report lines, and
# compares the native Workbench capture and AppIcon crop with the committed
# goldens. The gate starts its own private Xvfb display and needs no host
# input.
#
# Usage:
#   scripts/a1200-workbench-smoke.sh [workbench-binary] [services-binary]
#   CAPTURE_GOLDEN=1 scripts/a1200-workbench-smoke.sh [workbench] [services]
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
docker_dir="$(dirname "${script_dir}")"
repo_dir="$(dirname "${docker_dir}")"
cd "${repo_dir}"

workbench_bin="${1:-build/clorraine/workbench/hosted-amiga/workbench}"
services_bin="${2:-build/clorraine/services/hosted-amiga/services}"
start_bin="${START_BIN:-build/amiga-sdk/workbench-start}"
post_bin="${POST_BIN:-build/amiga-sdk/workbench-post}"
golden="${repo_dir}/docs/amiga-sdk/a1200-workbench.png"
icon_golden="${repo_dir}/docs/amiga-sdk/a1200-workbench-icon.png"
expected_wb="wb: ok icon 1 win 1 menu 1 types 8 7 9 ids 7 8 9 users 42 43 44 classes 0 0 0 args 0 0 0"
expected_svc="svc: ok icon 3 exp 2 man 2011 prod 82 card 1 status 0 key 1"
work="${repo_dir}/build/amiga-sdk/a1200-workbench"
hd="${work}/hd"
work_rel="${work#"${repo_dir}/"}"
rom="${work}/ks31.rom"
container="clorraine-a1200-workbench-$$"
capture_golden="${CAPTURE_GOLDEN:-0}"
xvfb_num=$(( ($$ % 50) + 160 ))
while [ -e "/tmp/.X11-unix/X${xvfb_num}" ]; do
    xvfb_num=$((xvfb_num + 1))
done
xvfb_display=":${xvfb_num}"
xvfb_pid=""

for binary in "${workbench_bin}" "${services_bin}" "${start_bin}" "${post_bin}"; do
    if [ ! -f "${binary}" ]; then
        echo "a1200-workbench-smoke: missing binary: ${binary}" >&2
        echo "a1200-workbench-smoke: build it with make sdk-advanced-build" >&2
        exit 2
    fi
done
if [ ! -f "${docker_dir}/.env" ]; then
    echo "a1200-workbench-smoke: docker/.env is missing; run 'make -C docker init'" >&2
    exit 2
fi
if [ "${capture_golden}" != "1" ] && { [ ! -f "${golden}" ] || [ ! -f "${icon_golden}" ]; }; then
    echo "a1200-workbench-smoke: missing golden(s): ${golden}, ${icon_golden}" >&2
    echo "a1200-workbench-smoke: run with CAPTURE_GOLDEN=1 once" >&2
    exit 2
fi
for tool in Xvfb convert compare identify sha1sum python3; do
    command -v "${tool}" >/dev/null 2>&1 \
        || { echo "a1200-workbench-smoke: missing host tool '${tool}'" >&2; exit 2; }
done

set -a
# shellcheck disable=SC1090
. "${docker_dir}/.env"
set +a
image="${IMAGE:-clorraine-sdk:dev}"

cleanup() {
    docker rm -f "${container}" >/dev/null 2>&1 || true
    if [ -n "${xvfb_pid}" ]; then
        kill "${xvfb_pid}" >/dev/null 2>&1 || true
        wait "${xvfb_pid}" 2>/dev/null || true
    fi
    rm -f "${rom}"
}
trap cleanup EXIT

rm -rf "${work}"
mkdir -p "${hd}/s"
cp "${workbench_bin}" "${hd}/wb-demo"
cp "${services_bin}" "${hd}/services"
cp "${start_bin}" "${hd}/workbench-start"
cp "${post_bin}" "${hd}/workbench-post"
cat > "${hd}/s/startup-sequence" <<'EOF'
CNEXT:services >CNEXT:svc.txt
CNEXT:workbench-start >CNEXT:wb-start.txt
CNEXT:workbench-post >CNEXT:wb-post.txt
EOF

Xvfb "${xvfb_display}" -screen 0 1280x720x24 -ac > "${work}/xvfb.log" 2>&1 &
xvfb_pid=$!
for _ in $(seq 1 50); do
    [ -S "/tmp/.X11-unix/X${xvfb_display#:}" ] && break
    sleep 0.1
done
export DISPLAY="${xvfb_display}"
unset XAUTHORITY

# Decrypt the licensed Cloanto ROM locally. The official A1200 Kickstart 3.1
# SHA-1 is e21545723fe8374e91342617604f1b3d703094f1.
(
    cd "${docker_dir}"
    docker compose -p clorraine --env-file .env -f compose.yaml run --rm -T dev \
        python3 -c "from amitools.rom.kickrom import Loader; data = Loader.load('/amiga/roms/amiga-os-310-a1200.rom', '/amiga/roms/rom.key'); open('/work/${work_rel}/ks31.rom', 'wb').write(data)"
)
sha1="$(sha1sum "${rom}" | awk '{print $1}')"
if [ "${sha1}" != "e21545723fe8374e91342617604f1b3d703094f1" ]; then
    echo "a1200-workbench-smoke: decrypted ROM has unexpected SHA-1 ${sha1}" >&2
    exit 1
fi

docker run --rm --name "${container}" \
    -e DISPLAY="${xvfb_display}" \
    -e SDL_VIDEO_X11_XINPUT2=0 \
    -v "${repo_dir}":/work \
    -v "${AMIGA_DATA}":/amiga:ro \
    -v /tmp/.X11-unix:/tmp/.X11-unix \
    "${image}" fs-uae \
    --amiga-model=A1200 \
    --chip-memory=2048 --fast-memory=0 --slow-memory=0 \
    --kickstart-file="/work/${work_rel}/ks31.rom" \
    --hard-drive-0="/work/${work_rel}/hd" \
    --hard-drive-0-label=CNEXT \
    --hard-drive-0-priority=0 \
    --window-width=640 --window-height=256 \
    --window-border=0 \
    --scale-x=1 --scale-y=1 \
    --crop=1 \
    > "${work}/fs-uae.log" 2>&1 &
runner=$!

ready=0
for _ in $(seq 1 90); do
    sleep 1
    if [ -f "${hd}/svc.txt" ] &&
       grep -aq '^startwb 1 screen [1-9a-f]' "${hd}/wb-start.txt" 2>/dev/null &&
       grep -aq '^ready$' "${hd}/wb-ready.txt" 2>/dev/null &&
       grep -aq '^wb: ok ' "${hd}/wb.txt" 2>/dev/null &&
       grep -aq '^posted$' "${hd}/wb-post.txt" 2>/dev/null &&
       grep -aq '^done$' "${hd}/wb-done.txt" 2>/dev/null &&
       grep -aq '^svc: ok ' "${hd}/svc.txt" 2>/dev/null; then
        ready=1
        break
    fi
done

sleep 1
docker stop -t 8 "${container}" >/dev/null 2>&1 || true
wait "${runner}" 2>/dev/null || true

if [ "${ready}" -ne 1 ]; then
    echo "a1200-workbench-smoke: the run did not finish" >&2
    for file in wb-start.txt wb-ready.txt wb.txt wb-post.txt wb-done.txt svc.txt; do
        if [ -f "${hd}/${file}" ]; then
            echo "--- ${file}" >&2
            cat "${hd}/${file}" >&2
        fi
    done
    tail -20 "${work}/fs-uae.log" >&2 || true
    exit 1
fi

actual_wb="$(tr -d '\r' < "${hd}/wb.txt")"
actual_svc="$(tr -d '\r' < "${hd}/svc.txt")"
posted="$(tr -d '\r' < "${hd}/wb-post.txt")"
done="$(tr -d '\r' < "${hd}/wb-done.txt")"
if [ "${actual_wb}" != "${expected_wb}" ]; then
    echo "a1200-workbench-smoke: Workbench report mismatch" >&2
    echo "  expected: ${expected_wb}" >&2
    echo "  actual:   ${actual_wb}" >&2
    exit 1
fi
if [ "${actual_svc}" != "${expected_svc}" ]; then
    echo "a1200-workbench-smoke: services report mismatch" >&2
    echo "  expected: ${expected_svc}" >&2
    echo "  actual:   ${actual_svc}" >&2
    exit 1
fi
if [ "${done}" != "done" ]; then
    echo "a1200-smoke: demo did not finish cleanup" >&2
    exit 1
fi
if [ "${posted}" != "posted" ]; then
    echo "a1200-workbench-smoke: fixture did not report posted" >&2
    exit 1
fi

python3 "${script_dir}/cscr2png.py" "${hd}/workbench.cscr" "${work}/workbench.png" --scale 2
colors="$(identify -format '%k' "${work}/workbench.png" 2>/dev/null || echo 0)"
if [ "${colors}" -lt 2 ] 2>/dev/null; then
    echo "a1200-workbench-smoke: the Workbench capture is blank" >&2
    exit 1
fi
convert "${work}/workbench.png" -crop 160x120+800+280 +repage "${work}/icon.png"

if [ "${capture_golden}" = "1" ]; then
    mkdir -p "$(dirname "${golden}")"
    cp "${work}/workbench.png" "${golden}"
    cp "${work}/icon.png" "${icon_golden}"
    echo "a1200-workbench-smoke: captured ${golden} and ${icon_golden}"
    exit 0
fi

metric="$(compare -metric AE "${work}/workbench.png" "${golden}" null: 2>&1 || true)"
metric="${metric%% *}"
if [ "${metric}" != "0" ]; then
    echo "a1200-workbench-smoke: Workbench capture differs (AE=${metric})" >&2
    exit 1
fi
icon_metric="$(compare -metric AE "${work}/icon.png" "${icon_golden}" null: 2>&1 || true)"
icon_metric="${icon_metric%% *}"
if [ "${icon_metric}" != "0" ]; then
    echo "a1200-workbench-smoke: AppIcon crop differs (AE=${icon_metric})" >&2
    exit 1
fi

echo "a1200-workbench-smoke: OK (${actual_wb}; ${actual_svc}; ${golden})"
