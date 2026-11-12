#!/usr/bin/env bash
# Shared headless Picasso96 RTG smoke for UAEGFX and emulated Picasso IV.
#
# Boots a clean directory HD with Workbench 3.1, the Aminet Picasso96 2.0
# runtime, and the selected monitor driver, runs the RTG demo under FS-UAE,
# and checks both the report and a 64x64 PPM capture. The emulator runs on a
# private Xvfb display; no host window is shown. Licensed media stays under
# /amiga and is never written or committed.
set -euo pipefail

card="${1:-}"
bin="${2:-}"
case "${card}" in
    uaegfx)
        description="A1200/Kickstart 3.1 Picasso96 UAEGFX smoke"
        work_name="a1200-rtg"
        artifact="docs/amiga-sdk/a1200-rtg.png"
        amiga_model="A1200"
        rom_name="amiga-os-310-a1200.rom"
        rom_sha1="e21545723fe8374e91342617604f1b3d703094f1"
        fast_memory="0"
        fsuae_card="uaegfx"
        p96_card="uaegfx.card"
        p96_chip=""
        monitor="uaegfx"
        monitor_info=""
        attach_settings=""
        startup_wait=""
        expected_accessors="rtg: accessors boards 1 mode 640x480x32 screen 640 480 32 9 bmp 32 32 32 6 128 fill 1 bmpread 1"
        expected_ok="rtg: ok boards 1 mode 640x480x32 red 1 green 1 blue 1 blit 1"
        ;;
    picasso-iv)
        description="A4000/Kickstart 3.1 emulated Picasso IV RTG smoke"
        work_name="a4000-rtg-piv"
        artifact="docs/amiga-sdk/a4000-rtg-piv.png"
        amiga_model="A4000"
        rom_name="amiga-os-310-a4000.rom"
        rom_sha1="5fe04842d04a489720f0f4bb0e46948199406f49"
        fast_memory="8192"
        fsuae_card="picasso-iv-z3"
        p96_card="PicassoIV.card"
        p96_chip="CirrusGD5446.chip"
        monitor="PicassoIV"
        monitor_info="Picasso96.info"
        attach_settings="yes"
        startup_wait="2"
        expected_accessors="rtg: accessors boards 1 mode 640x480x24 screen 640 480 24 3 bmp 32 32 24 3 96 fill 1 bmpread 1"
        expected_ok="rtg: ok boards 1 mode 640x480x24 red 1 green 1 blue 1 blit 1"
        ;;
    *)
        echo "usage: rtg-smoke.sh uaegfx|picasso-iv BINARY" >&2
        exit 2
        ;;
esac
if [ -z "${bin}" ]; then
    echo "rtg-smoke: missing demo binary" >&2
    exit 2
fi
if [ ! -f "${bin}" ]; then
    echo "rtg-smoke: missing binary: ${bin}" >&2
    exit 2
fi

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
docker_dir="$(dirname "${script_dir}")"
repo_dir="$(dirname "${docker_dir}")"
cd "${repo_dir}"

# shellcheck disable=SC1091
. "${script_dir}/workbench-media.sh"

p96_sha256="0146d4a2aaadb82ec454dde6ca4fb33a3f646ca4c0daa46e569c025f4d9728af"
picasso_iv_sha1="cafafb916f16b9f3ec9b49aa4b40eb4eeceb5b5b"
work="${repo_dir}/build/amiga-sdk/${work_name}"
hd="${work}/hd"
work_rel="${work#${repo_dir}/}"
rom="${work}/ks31.rom"
container="clorraine-${work_name}-$$"
xvfb_num=$(( ($$ % 50) + 250 ))
while [ -e "/tmp/.X11-unix/X${xvfb_num}" ]; do
    xvfb_num=$((xvfb_num + 1))
done
xvfb_display=":${xvfb_num}"
xvfb_container="clorraine-$(basename "${BASH_SOURCE[0]}" .sh)-xvfb-$$"
runner=""

if [ ! -f "${docker_dir}/.env" ]; then
    echo "rtg-smoke: docker/.env is missing" >&2
    exit 2
fi
for tool in convert sha1sum sha256sum python3; do
    command -v "${tool}" >/dev/null 2>&1 \
        || { echo "rtg-smoke: missing host tool '${tool}'" >&2; exit 2; }
done

set -a
# shellcheck disable=SC1090
. "${docker_dir}/.env"
set +a
image="${IMAGE:-clorraine-sdk:dev}"
p96_lha="${P96_LHA:-${AMIGA_DATA}/picasso96/Picasso96.lha}"
picasso_iv_rom="${PICASSO_IV_ROM:-${AMIGA_DATA}/roms/villagetronic-picasso-iv-74.rom}"

cleanup() {
    docker rm -f "${container}" "${xvfb_container}" >/dev/null 2>&1 || true
    if [ -n "${runner}" ]; then
        wait "${runner}" 2>/dev/null || true
    fi
    rm -f "/tmp/.X11-unix/X${xvfb_display#:}"
    rm -f "${rom}"
    # The Picasso IV flash ROM is user media; never leave a staged copy.
    rm -rf "${work}/Kickstarts"
}
trap cleanup EXIT

if [ ! -f "${p96_lha}" ]; then
    echo "rtg-smoke: missing ${p96_lha}" >&2
    echo "rtg-smoke: install the approved Aminet Picasso96.lha there or set P96_LHA" >&2
    exit 2
fi
sha256="$(sha256sum "${p96_lha}" | awk '{print $1}')"
if [ "${sha256}" != "${p96_sha256}" ]; then
    echo "rtg-smoke: Picasso96.lha has unexpected SHA-256 ${sha256}" >&2
    exit 2
fi
if [ "${card}" = "picasso-iv" ]; then
    if [ ! -f "${picasso_iv_rom}" ]; then
        echo "rtg-smoke: missing ${picasso_iv_rom}" >&2
        echo "rtg-smoke: install the Picasso IV 7.4 flash ROM or set PICASSO_IV_ROM" >&2
        exit 2
    fi
    sha1="$(sha1sum "${picasso_iv_rom}" | awk '{print $1}')"
    if [ "${sha1}" != "${picasso_iv_sha1}" ]; then
        echo "rtg-smoke: Picasso IV ROM has unexpected SHA-1 ${sha1}" >&2
        exit 2
    fi
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
required=(
    "${p96}/Libs/Picasso96API.library"
    "${p96}/Libs/Picasso96/rtg.library"
    "${p96}/Libs/Picasso96/emulation.library"
    "${p96}/Libs/Picasso96/${p96_card}"
    "${p96}/Devs/Monitors/Picasso96"
    "${p96}/Devs/Picasso96Settings.31"
)
if [ -n "${p96_chip}" ]; then
    required+=("${p96}/Libs/Picasso96/${p96_chip}")
fi
if [ -n "${attach_settings}" ]; then
    required+=("${p96}/Picasso96/AttachSettings")
fi
for file in "${required[@]}"; do
    if [ ! -f "${file}" ]; then
        echo "rtg-smoke: P96 archive is missing ${file#"$p96/"}" >&2
        exit 2
    fi
done

mkdir -p "${hd}/Libs/Picasso96" "${hd}/Devs/Monitors"
cp "${p96}/Libs/Picasso96API.library" "${hd}/Libs/"
cp "${p96}/Libs/Picasso96/rtg.library" "${hd}/Libs/Picasso96/"
cp "${p96}/Libs/Picasso96/emulation.library" "${hd}/Libs/Picasso96/"
cp "${p96}/Libs/Picasso96/${p96_card}" "${hd}/Libs/Picasso96/"
if [ -n "${p96_chip}" ]; then
    cp "${p96}/Libs/Picasso96/${p96_chip}" "${hd}/Libs/Picasso96/"
fi
cp "${p96}/Devs/Monitors/Picasso96" "${hd}/Devs/Monitors/${monitor}"
if [ -n "${monitor_info}" ]; then
    cp "${p96}/Devs/Monitors/${monitor_info}" "${hd}/Devs/Monitors/${monitor}.info"
fi
cp "${p96}/Devs/Picasso96Settings.31" "${hd}/Devs/Picasso96Settings"
if [ -n "${attach_settings}" ]; then
    cp "${p96}/Picasso96/AttachSettings" "${hd}/AttachSettings"
    chmod +x "${hd}/AttachSettings"
fi
cp "${bin}" "${hd}/optional-rtg"
chmod +x "${hd}/optional-rtg"

{
    wb_assigns "${hd}"
    if [ -n "${attach_settings}" ]; then
        cat <<'EOF'
CNEXT:AttachSettings PicassoIV 0 Devs:Picasso96Settings >CNEXT:attach.txt
EOF
    fi
    cat <<EOF
DEVS:Monitors/${monitor}
EOF
    if [ -n "${startup_wait}" ]; then
        cat <<EOF
CNEXT:C/Wait ${startup_wait}
EOF
    fi
    cat <<'EOF'
CNEXT:optional-rtg >CNEXT:rtg.txt
EOF
} > "${hd}/s/startup-sequence"

if [ "${card}" = "picasso-iv" ]; then
    mkdir -p "${work}/Kickstarts"
    cp "${picasso_iv_rom}" "${work}/Kickstarts/picasso_iv_flash.rom"
fi

(
    cd "${docker_dir}"
    docker compose -p clorraine --env-file .env -f compose.yaml run --rm -T dev \
        python3 -c "from amitools.rom.kickrom import Loader; data = Loader.load('/amiga/roms/${rom_name}', '/amiga/roms/rom.key'); open('/work/${work_rel}/ks31.rom', 'wb').write(data)"
)
sha1="$(sha1sum "${rom}" | awk '{print $1}')"
if [ "${sha1}" != "${rom_sha1}" ]; then
    echo "rtg-smoke: decrypted ROM has unexpected SHA-1 ${sha1}" >&2
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
    echo "$(basename "$0"): the private Xvfb did not start" >&2
    docker logs "${xvfb_container}" >&2 2>/dev/null || true
    exit 1
fi
export DISPLAY="${xvfb_display}"
unset XAUTHORITY

fsuae_args=(
    --amiga-model="${amiga_model}"
    --chip-memory=2048 --fast-memory="${fast_memory}" --slow-memory=0
    --kickstart-file="/work/${work_rel}/ks31.rom"
    --hard-drive-0="/work/${work_rel}/hd"
    --hard-drive-0-label=CNEXT
    --hard-drive-0-priority=0
    --graphics-card="${fsuae_card}"
    --governor-warning=0
    --window-width=640 --window-height=256 --window-border=0
    --scale-x=1 --scale-y=1 --crop=1
)
if [ "${card}" = "picasso-iv" ]; then
    fsuae_args+=(--kickstarts-dir="/work/${work_rel}/Kickstarts")
fi

docker run --rm --name "${container}" --network host \
    -e DISPLAY="${xvfb_display}" \
    -e SDL_VIDEO_X11_XINPUT2=0 \
    -v "${repo_dir}":/work \
    -v "${AMIGA_DATA}":/amiga:ro \
    -v /tmp/.X11-unix:/tmp/.X11-unix \
    "${image}" fs-uae "${fsuae_args[@]}" \
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
        echo "rtg-smoke: missing ${file}" >&2
        tail -30 "${work}/fs-uae.log" >&2 || true
        exit 1
    fi
done
actual_accessors="$(sed -n '1p' "${hd}/rtg.txt" | tr -d '\r')"
actual_ok="$(sed -n '2p' "${hd}/rtg.txt" | tr -d '\r')"
if [ "${actual_accessors}" != "${expected_accessors}" ] ||
   [ "${actual_ok}" != "${expected_ok}" ]; then
    echo "rtg-smoke: rtg.txt mismatch" >&2
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

convert "${hd}/rtg.ppm" -strip "${artifact}"
echo "rtg-smoke: OK ${description} (${expected_ok}; ${artifact})"
