#!/usr/bin/env bash
# A1200/Kickstart 3.1 Picasso96 RTG smoke.
#
# Boots a clean directory HD with Workbench 3.1, the Aminet Picasso96 2.0
# runtime, and the UAEGFX monitor, runs the RTG demo under FS-UAE's UAEGFX
# card, and checks both the report and a 64x64 PPM capture. The emulator runs
# on a private Xvfb display; no host window is shown.
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
docker_dir="$(dirname "${script_dir}")"
repo_dir="$(dirname "${docker_dir}")"
cd "${repo_dir}"

# shellcheck disable=SC1091
. "${script_dir}/workbench-media.sh"

p96_sha256="0146d4a2aaadb82ec454dde6ca4fb33a3f646ca4c0daa46e569c025f4d9728af"
expected_accessors="rtg: accessors boards 1 mode 640x480x32 screen 640 480 32 9 bmp 32 32 32 6 128 fill 1 bmpread 1"
expected_ok="rtg: ok boards 1 mode 640x480x32 red 1 green 1 blue 1 blit 1"
work="${repo_dir}/build/amiga-sdk/a1200-rtg"
hd="${work}/hd"
work_rel="${work#${repo_dir}/}"
rom="${work}/ks31.rom"
container="clorraine-a1200-rtg-$$"
xvfb_num=$(( ($$ % 50) + 250 ))
while [ -e "/tmp/.X11-unix/X${xvfb_num}" ]; do
    xvfb_num=$((xvfb_num + 1))
done
xvfb_display=":${xvfb_num}"
xvfb_pid=""
runner=""

bin="${1:-build/clorraine/optional-rtg/hosted-amiga/optional-rtg}"
if [ ! -f "${bin}" ]; then
    echo "a1200-rtg-smoke: missing binary: ${bin}" >&2
    exit 2
fi
if [ ! -f "${docker_dir}/.env" ]; then
    echo "a1200-rtg-smoke: docker/.env is missing" >&2
    exit 2
fi
for tool in Xvfb convert sha1sum sha256sum python3; do
    command -v "${tool}" >/dev/null 2>&1 \
        || { echo "a1200-rtg-smoke: missing host tool '${tool}'" >&2; exit 2; }
done

set -a
# shellcheck disable=SC1090
. "${docker_dir}/.env"
set +a
image="${IMAGE:-clorraine-sdk:dev}"
p96_lha="${P96_LHA:-${AMIGA_DATA}/picasso96/Picasso96.lha}"

cleanup() {
    docker rm -f "${container}" >/dev/null 2>&1 || true
    if [ -n "${runner}" ]; then
        wait "${runner}" 2>/dev/null || true
    fi
    if [ -n "${xvfb_pid}" ]; then
        kill "${xvfb_pid}" >/dev/null 2>&1 || true
        wait "${xvfb_pid}" 2>/dev/null || true
    fi
    rm -f "${rom}"
}
trap cleanup EXIT

if [ ! -f "${p96_lha}" ]; then
    echo "a1200-rtg-smoke: missing ${p96_lha}" >&2
    echo "a1200-rtg-smoke: install the approved Aminet Picasso96.lha there or set P96_LHA" >&2
    exit 2
fi
sha256="$(sha256sum "${p96_lha}" | awk '{print $1}')"
if [ "${sha256}" != "${p96_sha256}" ]; then
    echo "a1200-rtg-smoke: Picasso96.lha has unexpected SHA-256 ${sha256}" >&2
    exit 2
fi

rm -rf "${work}"
mkdir -p "${work}/p96" "${work}/stage31" "${hd}/s"
wb_dirs "${hd}"
stage31="$(wb_stage /amiga/adf/amiga-os-310-workbench.adf "${work}/stage31")"
wb_copy "${stage31}" "${hd}" Libs Devs C Classes

cp "${p96_lha}" "${work}/Picasso96.lha"
# The Amiga LHA may report a nonzero status for an extended filename while
# extracting every required file; the file checks below are authoritative.
docker run --rm -v "${repo_dir}":/work -v "${AMIGA_DATA}":/amiga:ro "${image}" \
    bash -lc "cd /work/${work_rel}/p96 && /opt/amiga/bin/lha x ../Picasso96.lha" \
    > "${work}/lha.log" 2>&1 || true

p96="${work}/p96/Picasso96Install"
for file in \
    "${p96}/Libs/Picasso96API.library" \
    "${p96}/Libs/Picasso96/rtg.library" \
    "${p96}/Libs/Picasso96/emulation.library" \
    "${p96}/Libs/Picasso96/uaegfx.card" \
    "${p96}/Devs/Monitors/Picasso96" \
    "${p96}/Devs/Picasso96Settings.31"; do
    if [ ! -f "${file}" ]; then
        echo "a1200-rtg-smoke: P96 archive is missing ${file#"${work}/"}" >&2
        exit 2
    fi
done
mkdir -p "${hd}/Libs/Picasso96" "${hd}/Devs/Monitors"
cp "${p96}/Libs/Picasso96API.library" "${hd}/Libs/"
cp "${p96}/Libs/Picasso96/rtg.library" "${hd}/Libs/Picasso96/"
cp "${p96}/Libs/Picasso96/emulation.library" "${hd}/Libs/Picasso96/"
cp "${p96}/Libs/Picasso96/uaegfx.card" "${hd}/Libs/Picasso96/"
cp "${p96}/Devs/Monitors/Picasso96" "${hd}/Devs/Monitors/uaegfx"
cp "${p96}/Devs/Picasso96Settings.31" "${hd}/Devs/Picasso96Settings"
cp "${bin}" "${hd}/optional-rtg"
chmod +x "${hd}/optional-rtg"

{
    wb_assigns "${hd}"
    cat <<'EOF'
DEVS:Monitors/uaegfx
CNEXT:optional-rtg >CNEXT:rtg.txt
EOF
} > "${hd}/s/startup-sequence"

(
    cd "${docker_dir}"
    docker compose -p clorraine --env-file .env -f compose.yaml run --rm -T dev \
        python3 -c "from amitools.rom.kickrom import Loader; data = Loader.load('/amiga/roms/amiga-os-310-a1200.rom', '/amiga/roms/rom.key'); open('/work/${work_rel}/ks31.rom', 'wb').write(data)"
)
sha1="$(sha1sum "${rom}" | awk '{print $1}')"
if [ "${sha1}" != "e21545723fe8374e91342617604f1b3d703094f1" ]; then
    echo "a1200-rtg-smoke: decrypted ROM has unexpected SHA-1 ${sha1}" >&2
    exit 1
fi

Xvfb "${xvfb_display}" -screen 0 1280x720x24 -ac > "${work}/xvfb.log" 2>&1 &
xvfb_pid=$!
for _ in $(seq 1 50); do
    [ -S "/tmp/.X11-unix/X${xvfb_display#:}" ] && break
    sleep 0.1
done
export DISPLAY="${xvfb_display}"
unset XAUTHORITY

docker run --rm --name "${container}" --network host \
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
    --graphics-card=uaegfx \
    --window-width=640 --window-height=256 --window-border=0 \
    --scale-x=1 --scale-y=1 --crop=1 \
    > "${work}/fs-uae.log" 2>&1 &
runner=$!

for _ in $(seq 1 90); do
    sleep 1
    if [ -f "${hd}/rtg.txt" ] && [ -f "${hd}/rtg.ppm" ]; then
        break
    fi
done
sleep 1
docker stop -t 8 "${container}" >/dev/null 2>&1 || true
wait "${runner}" 2>/dev/null || true
runner=""

for file in rtg.txt rtg.ppm; do
    if [ ! -f "${hd}/${file}" ]; then
        echo "a1200-rtg-smoke: missing ${file}" >&2
        tail -30 "${work}/fs-uae.log" >&2 || true
        exit 1
    fi
done
actual_accessors="$(sed -n '1p' "${hd}/rtg.txt" | tr -d '\r')"
actual_ok="$(sed -n '2p' "${hd}/rtg.txt" | tr -d '\r')"
if [ "${actual_accessors}" != "${expected_accessors}" ] ||
   [ "${actual_ok}" != "${expected_ok}" ]; then
    echo "a1200-rtg-smoke: rtg.txt mismatch" >&2
    echo "  expected accessors: ${expected_accessors}" >&2
    echo "  actual accessors:   ${actual_accessors}" >&2
    echo "  expected ok:        ${expected_ok}" >&2
    echo "  actual ok:          ${actual_ok}" >&2
    exit 1
fi

python3 - "${hd}/rtg.ppm" <<'PY'
import sys

path = sys.argv[1]
data = open(path, "rb").read()
header = b"P6\n64 64\n255\n"
assert data.startswith(header), data[:20]
payload = data[len(header):]
assert len(payload) == 64 * 64 * 3, len(payload)

def pixel(x, y):
    offset = (y * 64 + x) * 3
    return tuple(payload[offset:offset + 3])

assert pixel(0, 0) == (255, 0, 0), pixel(0, 0)
assert pixel(1, 0) == (0, 255, 0), pixel(1, 0)
assert pixel(2, 0) == (0, 0, 255), pixel(2, 0)
assert pixel(10, 10) == (255, 255, 0), pixel(10, 10)
assert pixel(11, 11) == (0, 0, 255), pixel(11, 11)
assert pixel(20, 20) == (0, 0, 255), pixel(20, 20)
PY

convert "${hd}/rtg.ppm" "docs/amiga-sdk/a1200-rtg.png"
echo "a1200-rtg-smoke: OK (rtg: ok boards 1 mode 640x480x32 red 1 green 1 blue 1 blit 1; docs/amiga-sdk/a1200-rtg.png)"
