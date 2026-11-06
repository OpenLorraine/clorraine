#!/usr/bin/env bash
# A1200/Kickstart 3.0 pool and BOOPSI smoke.
#
# Boots a clean directory HD with the licensed Workbench 3.0 libraries and
# classes, runs the utility-pool and native colorwheel demos, and compares the
# native colorwheel capture with its committed golden. No host input.
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
docker_dir="$(dirname "${script_dir}")"
repo_dir="$(dirname "${docker_dir}")"
cd "${repo_dir}"

# shellcheck disable=SC1091
. "${script_dir}/workbench-media.sh"

expected_pool="pool: ok 1"
expected_boopsi="boopsi: ok val 42"
work="${repo_dir}/build/amiga-sdk/a1200-ks30-addons"
hd="${work}/hd"
work_rel="${work#${repo_dir}/}"
rom="${work}/ks30.rom"
golden="${repo_dir}/docs/amiga-sdk/a1200-ks30-boopsi.png"
container="clorraine-a1200-ks30-addons-$$"
capture_golden="${CAPTURE_GOLDEN:-0}"
xvfb_num=$(( ($$ % 50) + 180 ))
while [ -e "/tmp/.X11-unix/X${xvfb_num}" ]; do
    xvfb_num=$((xvfb_num + 1))
done
xvfb_display=":${xvfb_num}"
xvfb_pid=""

for bin in \
    build/clorraine/optional-pool/hosted-amiga/optional-pool \
    build/clorraine/optional-boopsi/hosted-amiga/optional-boopsi; do
    if [ ! -f "${bin}" ]; then
        echo "a1200-ks30-addons-smoke: missing binary: ${bin}" >&2
        exit 2
    fi
done
if [ "${capture_golden}" != "1" ] && [ ! -f "${golden}" ]; then
    echo "a1200-ks30-addons-smoke: missing golden: ${golden}" >&2
    echo "a1200-ks30-addons-smoke: run with CAPTURE_GOLDEN=1 once" >&2
    exit 2
fi
if [ ! -f "${docker_dir}/.env" ]; then
    echo "a1200-ks30-addons-smoke: docker/.env is missing" >&2
    exit 2
fi
for tool in Xvfb convert compare identify sha1sum python3; do
    command -v "${tool}" >/dev/null 2>&1 \
        || { echo "a1200-ks30-addons-smoke: missing host tool '${tool}'" >&2; exit 2; }
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
wb_dirs "${hd}"
stage30="$(wb_stage /amiga/adf/amiga-os-300-workbench.adf "${work}/stage30")"
wb_copy "${stage30}" "${hd}" Libs Devs C Classes
cp build/clorraine/optional-pool/hosted-amiga/optional-pool "${hd}/optional-pool"
cp build/clorraine/optional-boopsi/hosted-amiga/optional-boopsi "${hd}/optional-boopsi"

{
    wb_assigns "${hd}"
    cat <<'EOF'
CNEXT:optional-pool >CNEXT:pool.txt
CNEXT:optional-boopsi >CNEXT:boopsi.txt
EOF
} > "${hd}/s/startup-sequence"

(
    cd "${docker_dir}"
    docker compose -p clorraine --env-file .env -f compose.yaml run --rm -T dev \
        python3 -c "from amitools.rom.kickrom import Loader; data = Loader.load('/amiga/roms/amiga-os-300-a1200.rom', '/amiga/roms/rom.key'); open('/work/${work_rel}/ks30.rom', 'wb').write(data)"
)
sha1="$(sha1sum "${rom}" | awk '{print $1}')"
if [ "${sha1}" != "70033828182fffc7ed106e5373a8b89dda76faa5" ]; then
    echo "a1200-ks30-addons-smoke: decrypted ROM has unexpected SHA-1 ${sha1}" >&2
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

docker run --rm --name "${container}" \
    -e DISPLAY="${xvfb_display}" \
    -e SDL_VIDEO_X11_XINPUT2=0 \
    -v "${repo_dir}":/work \
    -v "${AMIGA_DATA}":/amiga:ro \
    -v /tmp/.X11-unix:/tmp/.X11-unix \
    "${image}" fs-uae \
    --amiga-model=A1200 \
    --chip-memory=2048 --fast-memory=0 --slow-memory=0 \
    --kickstart-file="/work/${work_rel}/ks30.rom" \
    --hard-drive-0="/work/${work_rel}/hd" \
    --hard-drive-0-label=CNEXT \
    --hard-drive-0-priority=0 \
    --window-width=640 --window-height=256 --window-border=0 \
    --scale-x=1 --scale-y=1 --crop=1 \
    > "${work}/fs-uae.log" 2>&1 &
runner=$!

for _ in $(seq 1 60); do
    sleep 1
    if [ -f "${hd}/pool.txt" ] && [ -f "${hd}/boopsi.txt" ]; then
        break
    fi
done
sleep 1
docker stop -t 8 "${container}" >/dev/null 2>&1 || true
wait "${runner}" 2>/dev/null || true

for file in pool.txt boopsi.txt; do
    if [ ! -f "${hd}/${file}" ]; then
        echo "a1200-ks30-addons-smoke: missing ${file}" >&2
        tail -20 "${work}/fs-uae.log" >&2 || true
        exit 1
    fi
done
pool="$(tr -d '\r' < "${hd}/pool.txt")"
boopsi="$(tr -d '\r' < "${hd}/boopsi.txt")"
if [ "${pool}" != "${expected_pool}" ]; then
    echo "a1200-ks30-addons-smoke: pool mismatch: ${pool}" >&2
    exit 1
fi
if [ "${boopsi}" != "${expected_boopsi}" ]; then
    echo "a1200-ks30-addons-smoke: BOOPSI mismatch: ${boopsi}" >&2
    exit 1
fi

python3 "${script_dir}/cscr2png.py" "${hd}/a1200-ks30-boopsi.cscr" "${work}/boopsi.png" --scale 2
colors="$(identify -format '%k' "${work}/boopsi.png" 2>/dev/null || echo 0)"
if [ "${colors}" -lt 2 ] 2>/dev/null; then
    echo "a1200-ks30-addons-smoke: the BOOPSI capture is blank" >&2
    exit 1
fi

if [ "${capture_golden}" = "1" ]; then
    mkdir -p "$(dirname "${golden}")"
    cp "${work}/boopsi.png" "${golden}"
    echo "a1200-ks30-addons-smoke: captured ${golden}"
    exit 0
fi

metric="$(compare -metric AE "${work}/boopsi.png" "${golden}" null: 2>&1 || true)"
metric="${metric%% *}"
if [ "${metric}" != "0" ]; then
    echo "a1200-ks30-addons-smoke: BOOPSI capture differs (AE=${metric})" >&2
    exit 1
fi

echo "a1200-ks30-addons-smoke: OK (${pool}; ${boopsi}; ${golden})"
