#!/usr/bin/env bash
# A500/Kickstart 1.3 diskfont smoke.
#
# Boots a copy of the licensed Workbench 1.3.5 HDF on a real A500/KS1.3,
# assigns FONTS: to SYS:Fonts, opens diamond.font size 12, and asserts the
# report. Runs on a private Xvfb display; no host input.
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
docker_dir="$(dirname "${script_dir}")"
repo_dir="$(dirname "${docker_dir}")"
cd "${repo_dir}"

bin="${1:-build/clorraine/diskfont/hosted-amiga-13/diskfont}"
expected="diskfont: ok h 12"
work="${repo_dir}/build/amiga-sdk/a500-diskfont"
hdf="${work}/workbench.hdf"
hdf_rel="${hdf#"${repo_dir}/"}"
rom="${work}/ks13.rom"
rom_rel="${rom#"${repo_dir}/"}"
container="clorraine-a500-diskfont-$$"

if [ ! -f "${bin}" ]; then
    echo "a500-diskfont-smoke: missing binary: ${bin}" >&2
    echo "a500-diskfont-smoke: build it with make sdk-a500-diskfont" >&2
    exit 2
fi
if [ ! -f "${docker_dir}/.env" ]; then
    echo "a500-diskfont-smoke: docker/.env is missing; run 'make -C docker init'" >&2
    exit 2
fi
set -a
# shellcheck disable=SC1090
. "${docker_dir}/.env"
set +a
image="${IMAGE:-clorraine-sdk:dev}"
if [ -z "${AMIGA_DATA:-}" ]; then
    echo "a500-diskfont-smoke: AMIGA_DATA is not set in docker/.env" >&2
    exit 2
fi
if [ ! -f "${AMIGA_DATA}/hdf/workbench-135.hdf" ]; then
    echo "a500-diskfont-smoke: missing Workbench 1.3.5 HDF under ${AMIGA_DATA}/hdf" >&2
    exit 2
fi
for tool in sha1sum; do
    command -v "${tool}" >/dev/null 2>&1 \
        || { echo "a500-diskfont-smoke: missing host tool '${tool}'" >&2; exit 2; }
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
    rm -f "${rom}"
}
trap cleanup EXIT

rm -rf "${work}"
mkdir -p "${work}"
cp "${AMIGA_DATA}/hdf/workbench-135.hdf" "${hdf}"
chmod u+w "${hdf}"
cp "${bin}" "${work}/diskfont"
cat > "${work}/startup-sequence" <<'EOF'
Assign FONTS: SYS:Fonts
SYS:diskfont >SYS:diskfont.txt
EOF

xdftool() {
    (
        cd "${docker_dir}"
        docker compose -p clorraine --env-file .env -f compose.yaml run --rm -T dev \
            xdftool "/work/${hdf_rel}" "$@"
    )
}
xdftool write "/work/${work#"${repo_dir}/"}/diskfont" diskfont >/dev/null
xdftool protect diskfont +re >/dev/null
xdftool delete S/startup-sequence >/dev/null 2>&1 || true
xdftool write "/work/${work#"${repo_dir}/"}/startup-sequence" S/startup-sequence >/dev/null

(
    cd "${docker_dir}"
    docker compose -p clorraine --env-file .env -f compose.yaml run --rm -T dev \
        python3 -c "from amitools.rom.kickrom import Loader; data = Loader.load('/amiga/roms/amiga-os-130.rom', '/amiga/roms/rom.key'); open('/work/${rom_rel}', 'wb').write(data)"
)
sha1="$(sha1sum "${rom}" | awk '{print $1}')"
if [ "${sha1}" != "891e9a547772fe0c6c19b610baf8bc4ea7fcb785" ]; then
    echo "a500-diskfont-smoke: decrypted ROM has unexpected SHA-1 ${sha1}" >&2
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
    --amiga-model=A500 \
    --kickstart-file="/work/${rom_rel}" \
    --hard-drive-0="/work/${hdf_rel}" \
    --hard-drive-0-label=DH0 \
    --hard-drive-0-priority=0 \
    --window-width=640 --window-height=256 --window-border=0 \
    --scale-x=1 --scale-y=1 --crop=1 \
    > "${work}/fs-uae.log" 2>&1 &
runner=$!

# The HDF is locked by FS-UAE, so poll only after the run. The A500 reaches
# the Shell in roughly forty seconds in this environment.
sleep 60
docker stop -t 8 "${container}" >/dev/null 2>&1 || true
wait "${runner}" 2>/dev/null || true

xdftool read diskfont.txt "/work/${work#"${repo_dir}/"}/diskfont.txt" >/dev/null 2>&1
actual="$(tr -d '\r' < "${work}/diskfont.txt")"
if [ "${actual}" != "${expected}" ]; then
    echo "a500-diskfont-smoke: report mismatch" >&2
    echo "  expected: ${expected}" >&2
    echo "  actual:   ${actual}" >&2
    exit 1
fi

echo "a500-diskfont-smoke: OK (${actual})"
