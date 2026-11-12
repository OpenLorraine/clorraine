#!/usr/bin/env bash
# A600/Kickstart 3.1 lowlevel and nonvolatile smoke.
#
# Boots a clean directory HD with the licensed Workbench 3.1 libraries, runs
# the ReadJoyPort and nonvolatile demos, and asserts the report lines.
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
docker_dir="$(dirname "${script_dir}")"
repo_dir="$(dirname "${docker_dir}")"
cd "${repo_dir}"

# shellcheck disable=SC1091
. "${script_dir}/workbench-media.sh"

work="${repo_dir}/build/amiga-sdk/a600-ks31-addons"
hd="${work}/hd"
work_rel="${work#${repo_dir}/}"
rom="${work}/ks31.rom"
container="clorraine-a600-ks31-addons-$$"
xvfb_num=$(( ($$ % 50) + 150 ))
while [ -e "/tmp/.X11-unix/X${xvfb_num}" ]; do
    xvfb_num=$((xvfb_num + 1))
done
xvfb_display=":${xvfb_num}"
xvfb_container="clorraine-$(basename "${BASH_SOURCE[0]}" .sh)-xvfb-$$"

for bin in \
    build/clorraine/optional-lowlevel/hosted-amiga/optional-lowlevel \
    build/clorraine/optional-nonvolatile/hosted-amiga/optional-nonvolatile; do
    if [ ! -f "${bin}" ]; then
        echo "a600-ks31-addons-smoke: missing binary: ${bin}" >&2
        exit 2
    fi
done
for tool in sha1sum; do
    command -v "${tool}" >/dev/null 2>&1 \
        || { echo "a600-ks31-addons-smoke: missing host tool '${tool}'" >&2; exit 2; }
done
if [ ! -f "${docker_dir}/.env" ]; then
    echo "a600-ks31-addons-smoke: docker/.env is missing" >&2
    exit 2
fi

set -a
# shellcheck disable=SC1090
. "${docker_dir}/.env"
set +a
image="${IMAGE:-clorraine-sdk:dev}"

cleanup() {
    docker rm -f "${container}" "${xvfb_container}" >/dev/null 2>&1 || true
    rm -f "/tmp/.X11-unix/X${xvfb_display#:}"
    rm -f "${rom}"
}
trap cleanup EXIT

rm -rf "${work}"
mkdir -p "${hd}/s"
wb_dirs "${hd}"
# nonvolatile.library scans every drive for prefs/env-archive/sys/nv_location
# and locks the directory named inside. Give it a private writable store.
mkdir -p "${hd}/prefs/env-archive/sys" "${hd}/nonvolatile"
printf 'CNEXT:nonvolatile\n' > "${hd}/prefs/env-archive/sys/nv_location"
stage31="$(wb_stage /amiga/adf/amiga-os-310-workbench.adf "${work}/stage31")"
wb_copy "${stage31}" "${hd}" Libs Devs C Classes
cp build/clorraine/optional-lowlevel/hosted-amiga/optional-lowlevel "${hd}/optional-lowlevel"
cp build/clorraine/optional-nonvolatile/hosted-amiga/optional-nonvolatile "${hd}/optional-nonvolatile"

{
    wb_assigns "${hd}"
    cat <<'EOF'
CNEXT:C/AddDataTypes REFRESH QUIET
CNEXT:optional-lowlevel >CNEXT:ll.txt
CNEXT:optional-nonvolatile >CNEXT:nv.txt
EOF
} > "${hd}/s/startup-sequence"

(
    cd "${docker_dir}"
    docker compose -p clorraine --env-file .env -f compose.yaml run --rm -T dev \
        python3 -c "from amitools.rom.kickrom import Loader; data = Loader.load('/amiga/roms/amiga-os-310-a600.rom', '/amiga/roms/rom.key'); open('/work/${rom#${repo_dir}/}', 'wb').write(data)"
)
sha1="$(sha1sum "${rom}" | awk '{print $1}')"
if [ "${sha1}" != "3b7f1493b27e212830f989f26ca76c02049f09ca" ]; then
    echo "a600-ks31-addons-smoke: decrypted ROM has unexpected SHA-1 ${sha1}" >&2
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

docker run --rm --name "${container}" \
    -e DISPLAY="${xvfb_display}" \
    -e SDL_VIDEO_X11_XINPUT2=0 \
    -v "${repo_dir}":/work \
    -v "${AMIGA_DATA}":/amiga:ro \
    -v /tmp/.X11-unix:/tmp/.X11-unix \
    "${image}" fs-uae \
    --amiga-model=A600 \
    --chip-memory=1024 --fast-memory=0 --slow-memory=0 \
    --kickstart-file="/work/${rom#${repo_dir}/}" \
    --hard-drive-0="/work/${work_rel}/hd" \
    --hard-drive-0-label=CNEXT \
    --hard-drive-0-priority=0 \
    --window-width=640 --window-height=256 --window-border=0 \
    --scale-x=1 --scale-y=1 --crop=1 \
    > "${work}/fs-uae.log" 2>&1 &
runner=$!

for _ in $(seq 1 60); do
    sleep 1
    if [ -f "${hd}/ll.txt" ] && [ -f "${hd}/nv.txt" ]; then
        break
    fi
done
sleep 1
docker stop -t 8 "${container}" >/dev/null 2>&1 || true
wait "${runner}" 2>/dev/null || true

for file in ll.txt nv.txt; do
    if [ ! -f "${hd}/${file}" ]; then
        echo "a600-ks31-addons-smoke: missing ${file}" >&2
        tail -20 "${work}/fs-uae.log" >&2 || true
        exit 1
    fi
done
ll="$(tr -d '\r' < "${hd}/ll.txt")"
nv="$(tr -d '\r' < "${hd}/nv.txt")"
if [ "${ll}" != "ll: ok v 40" ]; then
    echo "a600-ks31-addons-smoke: lowlevel mismatch: ${ll}" >&2
    exit 1
fi
if [ "${nv}" != "nv: ok n 23 size 30" ]; then
    echo "a600-ks31-addons-smoke: nonvolatile mismatch: ${nv}" >&2
    exit 1
fi
echo "a600-ks31-addons-smoke: OK (${ll}; ${nv})"
