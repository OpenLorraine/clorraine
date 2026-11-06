#!/usr/bin/env bash
# A1200/Kickstart 3.1 datatypes, network, and AmigaGuide smoke.
#
# Boots a clean directory HD with the licensed Workbench 3.1 libraries and
# classes, starts Workbench with the test fixture, runs the ILBM datatypes
# viewer, a TCP echo round-trip, and an AmigaGuide async context. The network
# leg uses FS-UAE's emulated bsdsocket.library with a host echo peer.
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
docker_dir="$(dirname "${script_dir}")"
repo_dir="$(dirname "${docker_dir}")"
cd "${repo_dir}"

# shellcheck disable=SC1091
. "${script_dir}/workbench-media.sh"

expected_dt="dt: ok w 320 h 200 depth 4 colors 16"
expected_net="net: ok send 5 recv 5"
expected_guide="guide: ok msg 1 result 0"
work="${repo_dir}/build/amiga-sdk/a1200-addons"
hd="${work}/hd"
work_rel="${work#${repo_dir}/}"
rom="${work}/ks31.rom"
start_bin="${START_BIN:-build/amiga-sdk/workbench-start}"
container="clorraine-a1200-addons-$$"
net_port=12345
xvfb_num=$(( ($$ % 50) + 200 ))
while [ -e "/tmp/.X11-unix/X${xvfb_num}" ]; do
    xvfb_num=$((xvfb_num + 1))
done
xvfb_display=":${xvfb_num}"
xvfb_pid=""
peer_pid=""

for bin in \
    build/clorraine/optional-datatypes/hosted-amiga/optional-datatypes \
    build/clorraine/optional-network/hosted-amiga/optional-network \
    build/clorraine/optional-guide/hosted-amiga/optional-guide; do
    if [ ! -f "${bin}" ]; then
        echo "a1200-addons-smoke: missing binary: ${bin}" >&2
        exit 2
    fi
done
if [ ! -f "${start_bin}" ]; then
    echo "a1200-addons-smoke: missing fixture: ${start_bin}" >&2
    echo "a1200-addons-smoke: build it with make sdk-advanced-build" >&2
    exit 2
fi
if [ ! -f "${docker_dir}/.env" ]; then
    echo "a1200-addons-smoke: docker/.env is missing" >&2
    exit 2
fi
for tool in Xvfb convert compare identify sha1sum python3; do
    command -v "${tool}" >/dev/null 2>&1 \
        || { echo "a1200-addons-smoke: missing host tool '${tool}'" >&2; exit 2; }
done

set -a
# shellcheck disable=SC1090
. "${docker_dir}/.env"
set +a
image="${IMAGE:-clorraine-sdk:dev}"

cleanup() {
    docker rm -f "${container}" >/dev/null 2>&1 || true
    if [ -n "${peer_pid}" ]; then
        kill "${peer_pid}" >/dev/null 2>&1 || true
        wait "${peer_pid}" 2>/dev/null || true
    fi
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
stage31="$(wb_stage /amiga/adf/amiga-os-310-workbench.adf "${work}/stage31")"
wb_copy "${stage31}" "${hd}" Libs Devs C Classes
stage31fonts="$(wb_stage_any /amiga/adf/amiga-os-310-fonts.adf "${work}/stage31fonts")"
wb_copy "${stage31fonts}" "${hd}" Fonts

cp build/clorraine/optional-datatypes/hosted-amiga/optional-datatypes \
    "${hd}/optional-datatypes"
cp build/clorraine/optional-network/hosted-amiga/optional-network \
    "${hd}/optional-network"
cp build/clorraine/optional-guide/hosted-amiga/optional-guide \
    "${hd}/optional-guide"
cp "${start_bin}" "${hd}/workbench-start"
chmod +x "${hd}/workbench-start"
cp examples/amiga/assets/clorraine.ilbm "${hd}/image.ilbm"
cp examples/amiga/hello.guide "${hd}/hello.guide"

{
    wb_assigns "${hd}"
    cat <<'EOF'
CNEXT:C/AddDataTypes REFRESH QUIET
CNEXT:optional-network >CNEXT:net.txt
CNEXT:workbench-start >CNEXT:wb-start.txt
CNEXT:optional-datatypes >CNEXT:dt.txt
CNEXT:optional-guide >CNEXT:guide.txt
EOF
} > "${hd}/s/startup-sequence"

(
    cd "${docker_dir}"
    docker compose -p clorraine --env-file .env -f compose.yaml run --rm -T dev \
        python3 -c "from amitools.rom.kickrom import Loader; data = Loader.load('/amiga/roms/amiga-os-310-a1200.rom', '/amiga/roms/rom.key'); open('/work/${work_rel}/ks31.rom', 'wb').write(data)"
)
sha1="$(sha1sum "${rom}" | awk '{print $1}')"
if [ "${sha1}" != "e21545723fe8374e91342617604f1b3d703094f1" ]; then
    echo "a1200-addons-smoke: decrypted ROM has unexpected SHA-1 ${sha1}" >&2
    exit 1
fi

# The emulated bsdsocket library proxies guest connect() to the host, so the
# echo peer binds 127.0.0.1 and the FS-UAE container runs on the host network.
python3 - "${net_port}" > "${work}/echo.log" 2>&1 <<'PY' &
import socket
import sys
import time

port = int(sys.argv[1])
server = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
server.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
server.bind(("127.0.0.1", port))
server.listen(4)
server.settimeout(90)
for _ in range(8):
    try:
        conn, _ = server.accept()
        print("accepted", flush=True)
    except OSError as error:
        print("accept-error", error, flush=True)
        break
    conn.settimeout(10)
    data = b""
    try:
        while len(data) < 5:
            chunk = conn.recv(5 - len(data))
            if not chunk:
                break
            data += chunk
        print("read", len(data), flush=True)
        if data:
            conn.sendall(data)
            print("echoed", len(data), flush=True)
    except OSError as error:
        print("recv-error", error, flush=True)
    time.sleep(0.5)
    conn.close()
server.close()
PY
peer_pid=$!
sleep 1

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
    --bsdsocket_library=1 \
    --window-width=640 --window-height=256 --window-border=0 \
    --scale-x=1 --scale-y=1 --crop=1 \
    > "${work}/fs-uae.log" 2>&1 &
runner=$!

for _ in $(seq 1 90); do
    sleep 1
    if [ -f "${hd}/dt.txt" ] && [ -f "${hd}/net.txt" ] &&
       [ -f "${hd}/guide.txt" ]; then
        break
    fi
done
sleep 1
docker stop -t 8 "${container}" >/dev/null 2>&1 || true
wait "${runner}" 2>/dev/null || true
kill "${peer_pid}" >/dev/null 2>&1 || true
wait "${peer_pid}" 2>/dev/null || true
peer_pid=""

for file in wb-start.txt dt.txt net.txt guide.txt; do
    if [ ! -f "${hd}/${file}" ]; then
        echo "a1200-addons-smoke: missing ${file}" >&2
        tail -20 "${work}/fs-uae.log" >&2 || true
        exit 1
    fi
done
line() { tr -d '\r' < "$1"; }
check() {
    local file="$1" expected="$2"
    local actual
    actual="$(line "${hd}/${file}")"
    if [ "${actual}" != "${expected}" ]; then
        echo "a1200-addons-smoke: ${file} mismatch" >&2
        echo "  expected: ${expected}" >&2
        echo "  actual:   ${actual}" >&2
        exit 1
    fi
}
check dt.txt "${expected_dt}"
check net.txt "${expected_net}"
check guide.txt "${expected_guide}"
if ! grep -aq '^startwb 1 screen [1-9a-f]' "${hd}/wb-start.txt"; then
    echo "a1200-addons-smoke: Workbench did not start" >&2
    cat "${hd}/wb-start.txt" >&2 || true
    exit 1
fi

echo "a1200-addons-smoke: OK (${expected_dt}; ${expected_net}; ${expected_guide})"
