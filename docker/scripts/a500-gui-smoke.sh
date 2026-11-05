#!/usr/bin/env bash
# A500/Kickstart 1.3 Intuition GUI smoke.
#
# Decrypts the user's licensed Amiga Forever Kickstart 1.3 ROM locally (the
# decrypted image stays in gitignored build/ and is removed on exit), boots the
# GUI demo from a clean ADF under FS-UAE configured as an A500, drives it with
# the host keyboard on a private Xvfb display, and compares the app's native
# screen capture and report line with committed goldens. No emulator window is
# shown on the user's desktop; the artifact comes from the Amiga bitmap, not
# from a host-window screenshot.
#
# The app writes `DF0:gui.cscr` before the event loop, one `DF0:gui-*`
# handshake file per transition, and `DF0:gui.txt` after a clean Quit. The raw
# ADF is polled for those names because the emulated floppy is shared with the
# host as a normal file. FS-UAE turbo floppy mode keeps the 61 KB capture from
# stalling the event loop.
#
# Usage:
#   scripts/a500-gui-smoke.sh [binary]
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
docker_dir="$(dirname "${script_dir}")"
repo_dir="$(dirname "${docker_dir}")"
cd "${repo_dir}"

binary="${1:-build/clorraine/gui/hosted-amiga-13/gui}"
golden="${repo_dir}/docs/amiga-sdk/a500-gui.png"
work="${repo_dir}/build/amiga-sdk/gui-smoke"
work_rel="${work#"${repo_dir}/"}"
adf="${work_rel}/a500-gui.adf"
rom="${work_rel}/ks13.rom"
container="clorraine-a500-gui-$$"
expected="gui: ok win 1 gad 2 menu 2 str 3 req 2"

if [ ! -f "${binary}" ]; then
    echo "a500-gui-smoke: missing binary: ${binary}" >&2
    echo "a500-gui-smoke: build it with --profile hosted-amiga-13 first" >&2
    exit 2
fi
if [ ! -f "${golden}" ]; then
    echo "a500-gui-smoke: missing golden frame: ${golden}" >&2
    exit 2
fi
if [ ! -f "${docker_dir}/.env" ]; then
    echo "a500-gui-smoke: docker/.env is missing; run 'make -C docker init'" >&2
    exit 2
fi
for tool in xdotool Xvfb import compare convert sha1sum python3; do
    command -v "${tool}" >/dev/null 2>&1 \
        || { echo "a500-gui-smoke: missing host tool '${tool}'" >&2; exit 2; }
done

set -a
# shellcheck disable=SC1090
. "${docker_dir}/.env"
set +a
image="${IMAGE:-clorraine-sdk:dev}"

xvfb_num=$(( ($$ % 50) + 60 ))
while [ -e "/tmp/.X11-unix/X${xvfb_num}" ]; do
    xvfb_num=$((xvfb_num + 1))
done
xvfb_display=":${xvfb_num}"
xvfb_pid=""

cleanup() {
    docker rm -f "${container}" >/dev/null 2>&1 || true
    if [ -n "${xvfb_pid}" ]; then
        kill "${xvfb_pid}" >/dev/null 2>&1 || true
        wait "${xvfb_pid}" 2>/dev/null || true
    fi
    rm -f "${repo_dir}/${rom}"
}
trap cleanup EXIT

rm -rf "${work:?}"/*
mkdir -p "${work}/S"
Xvfb "${xvfb_display}" -screen 0 1280x720x24 -ac > "${work}/xvfb.log" 2>&1 &
xvfb_pid=$!
for _ in $(seq 1 50); do
    [ -S "/tmp/.X11-unix/X${xvfb_display#:}" ] && break
    sleep 0.1
done
export DISPLAY="${xvfb_display}"
unset XAUTHORITY

cp "${binary}" "${work}/gui"
printf 'DF0:gui\n' > "${work}/startup-sequence"
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
    echo "a500-gui-smoke: decrypted ROM has unexpected SHA-1 ${sha1}" >&2
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
xdftool write "/work/${work_rel}/gui" gui + protect gui +re >/dev/null
xdftool write "/work/${work_rel}/startup-sequence" startup-sequence >/dev/null
xdftool makedir S + write "/work/${work_rel}/S/startup-sequence" S/startup-sequence >/dev/null
xdftool boot install boot1x >/dev/null
if ! xdftool list | grep -q ' startup-sequence'; then
    echo "a500-gui-smoke: the ADF has no root startup-sequence" >&2
    exit 1
fi

# The private Xvfb display shows no window on the user's desktop. Scale 1 and
# crop 1 keep the host window at the native 640x256 size; the app capture is the
# artifact, so host scaling never affects the golden.
docker run --rm --name "${container}" \
    -e DISPLAY="${xvfb_display}" \
    -e SDL_VIDEO_X11_XINPUT2=0 \
    -v "${repo_dir}":/work \
    -v "${AMIGA_DATA}":/amiga:ro \
    -v /tmp/.X11-unix:/tmp/.X11-unix \
    "${image}" fs-uae \
    --amiga-model=A500 \
    --kickstart-file="/work/${rom}" \
    --floppy-drive-0="/work/${adf}" \
    --writable-floppy-images=1 \
    --window-width=640 --window-height=256 \
    --window-border=0 \
    --scale-x=1 --scale-y=1 \
    --crop=1 \
    --floppy-drive-speed=0 \
    > "${work}/fs-uae.log" 2>&1 &
runner=$!

window=""
for _ in $(seq 1 90); do
    window="$(xdotool search --name 'FS-UAE' 2>/dev/null | tail -1 || true)"
    [ -n "${window}" ] && break
    sleep 1
done
if [ -z "${window}" ]; then
    echo "a500-gui-smoke: no FS-UAE window" >&2
    tail -20 "${work}/fs-uae.log" >&2 || true
    exit 1
fi

focus() {
    window="$(xdotool search --name 'FS-UAE' 2>/dev/null | tail -1 || true)"
    if [ -n "${window}" ]; then
        xdotool windowfocus --sync "${window}" || true
        xdotool mousemove --window "${window}" 10 10 || true
    fi
    sleep 0.3
}

press() {
    local modifier="$1"
    local key="$2"
    focus
    if [ -n "${modifier}" ]; then
        xdotool keydown "${modifier}"
    fi
    xdotool key "${key}"
    if [ -n "${modifier}" ]; then
        xdotool keyup "${modifier}"
    fi
}

# The demo writes one handshake file per transition. Polling the raw ADF is
# the only reliable clock here: the emulated floppy and the requester
# transitions can take several seconds, so fixed sleeps race the UI.
wait_state() {
    local name="$1"
    for _ in $(seq 1 90); do
        if grep -aq "gui-${name}" "${repo_dir}/${adf}" 2>/dev/null; then
            return 0
        fi
        sleep 1
    done
    return 1
}

# Wait for the UI, the native capture, and the event loop.
if ! wait_state "ready"; then
    echo "a500-gui-smoke: the demo never became ready" >&2
    tail -20 "${work}/fs-uae.log" >&2 || true
    exit 1
fi

# Type into the active string gadget, end the edit with Return, then use the
# Amiga-key shortcuts. Left Amiga is Super_L; Right Amiga is Menu on this host
# mapping. Each shortcut is retried until the demo writes the next handshake.
focus
xdotool key --delay 150 a b c
sleep 1
press "" "Return"

for _ in $(seq 1 20); do
    press "Super_L" "v"
    sleep 1
    grep -aq "gui-answered" "${repo_dir}/${adf}" 2>/dev/null && break
done
if ! grep -aq "gui-answered" "${repo_dir}/${adf}" 2>/dev/null; then
    echo "a500-gui-smoke: the Continue requester was not answered" >&2
    xdftool list >&2 || true
    exit 1
fi

for _ in $(seq 1 8); do
    press "Menu" "h"
    sleep 1
    grep -aq "gui-hello" "${repo_dir}/${adf}" 2>/dev/null && break
done
if ! grep -aq "gui-hello" "${repo_dir}/${adf}" 2>/dev/null; then
    echo "a500-gui-smoke: the Hello menu item was not selected" >&2
    xdftool list >&2 || true
    exit 1
fi

press "Menu" "a"
sleep 1
press "Menu" "a"
for _ in $(seq 1 20); do
    press "Super_L" "v"
    sleep 1
    grep -aq "gui-about" "${repo_dir}/${adf}" 2>/dev/null && break
done
if ! grep -aq "gui-about" "${repo_dir}/${adf}" 2>/dev/null; then
    echo "a500-gui-smoke: the About requester was not answered" >&2
    xdftool list >&2 || true
    exit 1
fi

for _ in $(seq 1 8); do
    press "Menu" "q"
    sleep 1
    grep -aq "gui-quit" "${repo_dir}/${adf}" 2>/dev/null && break
done
if ! grep -aq "gui-quit" "${repo_dir}/${adf}" 2>/dev/null; then
    echo "a500-gui-smoke: the Quit menu item was not selected" >&2
    xdftool list >&2 || true
    exit 1
fi

# The report file is created before its data is committed, so poll the data
# itself. The literal in the program is `gui: ok win ` with a NUL after `win`,
# which does not match the `win 1` that only the report line contains.
reported=0
for _ in $(seq 1 60); do
    if grep -aq "gui: ok win 1" "${repo_dir}/${adf}" 2>/dev/null; then
        reported=1
        break
    fi
    sleep 1
done
if [ "${reported}" -ne 1 ]; then
    echo "a500-gui-smoke: the demo never wrote the report line" >&2
    echo "a500-gui-smoke: last ADF listing:" >&2
    xdftool list >&2 || true
    exit 1
fi

docker stop -t 5 "${container}" >/dev/null 2>&1 || true
wait "${runner}" 2>/dev/null || true

xdftool read gui.cscr "/work/${work_rel}/gui.cscr" >/dev/null
xdftool read gui.txt "/work/${work_rel}/gui.txt" >/dev/null
python3 "${script_dir}/cscr2png.py" "${work}/gui.cscr" "${work}/gui.png" --scale 2

actual="$(cat "${work}/gui.txt")"
if [ "${actual}" != "${expected}" ]; then
    echo "a500-gui-smoke: report line mismatch" >&2
    echo "  expected: ${expected}" >&2
    echo "  actual:   ${actual}" >&2
    exit 1
fi

metric="$(compare -metric AE "${work}/gui.png" "${golden}" null: 2>&1 || true)"
metric="${metric%% *}"
if [ "${metric}" != "0" ]; then
    echo "a500-gui-smoke: native frame mismatch (${metric} pixels differ)" >&2
    echo "a500-gui-smoke: captured frame kept at ${work}/gui.png" >&2
    exit 1
fi

echo "a500-gui-smoke: OK (frame: ${golden}; line: ${actual})"
