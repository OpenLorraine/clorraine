#!/usr/bin/env bash
# A500/Kickstart 1.3 text/layers smoke.
#
# Decrypts the user's licensed Amiga Forever Kickstart 1.3 ROM locally (the
# decrypted image stays in gitignored build/ and is removed on exit), boots the
# scrolling text demo from a clean ADF under FS-UAE configured as an A500, polls for the
# live native display, and then polls for the final text line. The live
# frame proves the display rendered; the line crop is compared byte-exact to
# the committed golden image.
#
# This harness mirrors scripts/a500-smoke.sh: the boot path, ROM check, ADF
# layout, and X11 capture are deliberately the same so both A500 gates exercise
# the same machine. The polling difference is needed because text opens its
# own custom display before it prints.
#
# Usage:
#   scripts/a500-text-smoke.sh [binary]
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
docker_dir="$(dirname "${script_dir}")"
repo_dir="$(dirname "${docker_dir}")"
cd "${repo_dir}"

binary="${1:-build/clorraine/scroller/hosted-amiga-13/scroller}"
golden="${repo_dir}/docs/amiga-sdk/a500-text-line.png"
frame_golden="${repo_dir}/docs/amiga-sdk/a500-text.png"
work="${repo_dir}/build/amiga-sdk/text-smoke"
work_rel="${work#"${repo_dir}/"}"
adf="${work_rel}/a500-text.adf"
rom="${work_rel}/ks13.rom"
screenshot="${work}/screen.png"
crop="${work}/crop.png"
display="${work}/display.png"
live="${work}/live.png"
container="clorraine-a500-text-$$"

if [ ! -f "${binary}" ]; then
    echo "a500-text-smoke: missing binary: ${binary}" >&2
    echo "a500-text-smoke: build it with --profile hosted-amiga-13 first" >&2
    exit 2
fi
if [ ! -f "${golden}" ]; then
    echo "a500-text-smoke: missing golden line: ${golden}" >&2
    exit 2
fi
if [ ! -f "${frame_golden}" ]; then
    echo "a500-text-smoke: missing golden frame: ${frame_golden}" >&2
    exit 2
fi
if [ ! -f "${docker_dir}/.env" ]; then
    echo "a500-text-smoke: docker/.env is missing; run 'make -C docker init'" >&2
    exit 2
fi
for tool in xdotool import convert compare sha1sum; do
    command -v "${tool}" >/dev/null 2>&1 \
        || { echo "a500-text-smoke: missing host tool '${tool}'" >&2; exit 2; }
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
cp "${binary}" "${work}/scroller"
printf 'DF0:scroller\n' > "${work}/startup-sequence"
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
    echo "a500-text-smoke: decrypted ROM has unexpected SHA-1 ${sha1}" >&2
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
xdftool write "/work/${work_rel}/scroller" scroller + protect scroller +re >/dev/null
xdftool write "/work/${work_rel}/startup-sequence" startup-sequence >/dev/null
xdftool makedir S + write "/work/${work_rel}/S/startup-sequence" S/startup-sequence >/dev/null
xdftool boot install boot1x >/dev/null
if ! xdftool list | grep -q ' startup-sequence'; then
    echo "a500-text-smoke: the ADF has no root startup-sequence" >&2
    exit 1
fi

# Boot the real A500/KS1.3 machine. The ADF takes roughly twenty seconds to
# reach the startup sequence; the scrolling text demo then opens its custom
# display and holds the final frame before restoring the shell and printing the
# measured text line.
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
    echo "a500-text-smoke: no FS-UAE window" >&2
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
    # The held acceptance frame is deterministic; comparing the display crop
    # proves the layer clipping, the ROM font, and the parked scroll text. The
    # animation frames differ in text position, so only the hold matches.
    convert "${screenshot}" -crop 408x438+418+102 +repage "${display}"
    frame_metric="$(compare -metric AE "${display}" "${frame_golden}" null: 2>&1 || true)"
    frame_metric="${frame_metric%% *}"
    if [ "${frame_metric}" = "0" ] && [ "${frame_matched}" -eq 0 ]; then
        frame_matched=1
        saw_live=1
        cp "${screenshot}" "${live}"
    fi
    convert "${screenshot}" -crop 540x22+160+88 +repage "${crop}"
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
    echo "a500-text-smoke: never saw the native text display" >&2
    echo "a500-text-smoke: last screen kept at ${screenshot}" >&2
    exit 1
fi
if [ "${frame_matched}" -ne 1 ]; then
    echo "a500-text-smoke: held display never matched ${frame_golden}" >&2
    echo "a500-text-smoke: last display and screen kept at ${display} and ${screenshot}" >&2
    exit 1
fi
if [ "${line_matched}" -ne 1 ]; then
    echo "a500-text-smoke: text line never matched ${golden}" >&2
    echo "a500-text-smoke: last crop and screen kept at ${crop} and ${screenshot}" >&2
    exit 1
fi

echo "a500-text-smoke: OK (held frame: ${live}; text line: ${golden})"
