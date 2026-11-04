#!/usr/bin/env bash
# A500/Kickstart 1.3 paddle game smoke.
#
# Decrypts the user's licensed Amiga Forever Kickstart 1.3 ROM locally (the
# decrypted image stays in gitignored build/ and is removed on exit), boots the
# paddle game from a clean ADF under FS-UAE configured as an A500, polls for the
# live Intuition screen and the final result line, and compares both to the
# committed goldens.
#
# The demo uses a 320x256 Intuition screen, so the live display is the 640x512
# double-height area at +160+14 in the 960x540 FS-UAE window.
#
# Usage:
#   scripts/a500-paddle-smoke.sh [binary]
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
docker_dir="$(dirname "${script_dir}")"
repo_dir="$(dirname "${docker_dir}")"
cd "${repo_dir}"

binary="${1:-build/clorraine/paddle/hosted-amiga-13/paddle}"
golden="${repo_dir}/docs/amiga-sdk/a500-paddle-line.png"
frame_golden="${repo_dir}/docs/amiga-sdk/a500-paddle.png"
work="${repo_dir}/build/amiga-sdk/paddle-smoke"
work_rel="${work#"${repo_dir}/"}"
adf="${work_rel}/a500-paddle.adf"
rom="${work_rel}/ks13.rom"
screenshot="${work}/screen.png"
crop="${work}/crop.png"
display="${work}/display.png"
live="${work}/live.png"
container="clorraine-a500-paddle-$$"

if [ ! -f "${binary}" ]; then
    echo "a500-paddle-smoke: missing binary: ${binary}" >&2
    echo "a500-paddle-smoke: build it with --profile hosted-amiga-13 first" >&2
    exit 2
fi
if [ ! -f "${golden}" ]; then
    echo "a500-paddle-smoke: missing golden line: ${golden}" >&2
    exit 2
fi
if [ ! -f "${frame_golden}" ]; then
    echo "a500-paddle-smoke: missing golden frame: ${frame_golden}" >&2
    exit 2
fi
if [ ! -f "${docker_dir}/.env" ]; then
    echo "a500-paddle-smoke: docker/.env is missing; run 'make -C docker init'" >&2
    exit 2
fi
for tool in xdotool import convert compare sha1sum; do
    command -v "${tool}" >/dev/null 2>&1 \
        || { echo "a500-paddle-smoke: missing host tool '${tool}'" >&2; exit 2; }
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
rm -f "${screenshot}" "${crop}" "${display}" "${live}" "${repo_dir}/${adf}" "${repo_dir}/${rom}"
cp "${binary}" "${work}/paddle"
printf 'DF0:paddle\n' > "${work}/startup-sequence"
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
    echo "a500-paddle-smoke: decrypted ROM has unexpected SHA-1 ${sha1}" >&2
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
xdftool write "/work/${work_rel}/paddle" paddle + protect paddle +re >/dev/null
xdftool write "/work/${work_rel}/startup-sequence" startup-sequence >/dev/null
xdftool makedir S + write "/work/${work_rel}/S/startup-sequence" S/startup-sequence >/dev/null
xdftool boot install boot1x >/dev/null
if ! xdftool list | grep -q ' startup-sequence'; then
    echo "a500-paddle-smoke: the ADF has no root startup-sequence" >&2
    exit 1
fi

# Boot the real A500/KS1.3 machine. The ADF takes roughly twenty seconds to
# reach the startup sequence; the demo then holds the final paddle frame for
# sixteen seconds before restoring the shell and printing the paddle line.
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
sleep 30

window="$(DISPLAY="${DISPLAY}" xdotool search --name 'FS-UAE' 2>/dev/null | tail -1 || true)"
if [ -z "${window}" ]; then
    echo "a500-paddle-smoke: no FS-UAE window" >&2
    tail -20 "${work}/fs-uae.log" >&2 || true
    exit 1
fi

saw_live=0
frame_matched=0
line_matched=0
step=0
while [ "${step}" -lt 15 ]; do
    step=$((step + 1))
    sleep 5
    DISPLAY="${DISPLAY}" import -window "${window}" "${screenshot}"
    # The held frame is deterministic: the paddle is parked at x=136 and the
    # ball at (160, 100). The crop match is the liveness gate.
    convert "${screenshot}" -crop 640x512+160+14 +repage "${display}"
    frame_metric="$(compare -metric AE "${display}" "${frame_golden}" null: 2>&1 || true)"
    frame_metric="${frame_metric%% *}"
    if [ "${frame_metric}" = "0" ] && [ "${frame_matched}" -eq 0 ]; then
        frame_matched=1
        saw_live=1
        cp "${screenshot}" "${live}"
    fi
    convert "${screenshot}" -crop 430x22+160+88 +repage "${crop}"
    metric="$(compare -metric AE "${crop}" "${golden}" null: 2>&1 || true)"
    metric="${metric%% *}"
    if [ "${metric}" = "0" ]; then
        line_matched=1
    fi
    if [ "${frame_matched}" -eq 1 ] && [ "${line_matched}" -eq 1 ]; then
        break
    fi
done

docker stop -t 5 "${container}" >/dev/null 2>&1 || true
wait "${runner}" 2>/dev/null || true

if [ "${saw_live}" -eq 0 ]; then
    echo "a500-paddle-smoke: never saw the paddle screen" >&2
    echo "a500-paddle-smoke: last screen kept at ${screenshot}" >&2
    exit 1
fi
if [ "${frame_matched}" -ne 1 ]; then
    echo "a500-paddle-smoke: held display never matched ${frame_golden}" >&2
    echo "a500-paddle-smoke: last display and screen kept at ${display} and ${screenshot}" >&2
    exit 1
fi
if [ "${line_matched}" -ne 1 ]; then
    echo "a500-paddle-smoke: paddle line never matched ${golden}" >&2
    echo "a500-paddle-smoke: last crop and screen kept at ${crop} and ${screenshot}" >&2
    exit 1
fi

echo "a500-paddle-smoke: OK (held frame: ${live}; paddle line: ${golden})"
