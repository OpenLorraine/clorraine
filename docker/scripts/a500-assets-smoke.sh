#!/usr/bin/env bash
# A500/Kickstart 1.3 DOS/asset smoke.
#
# Decrypts the user's licensed Amiga Forever Kickstart 1.3 ROM locally (the
# decrypted image stays in gitignored build/ and is removed on exit), boots the
# ILBM image viewer from a clean ADF under FS-UAE configured as an A500, polls
# for the live custom display and its held frame, then polls for the final
# report line. The held frame proves the decoded image, palette, and caption;
# the line crop proves the DOS self-test mask, the pixel hash, and the volume
# bit.
#
# This harness mirrors scripts/a500-text-smoke.sh: the boot path, ROM check, ADF
# layout, and X11 capture are deliberately the same so every A500 gate exercises
# the same machine.
#
# Usage:
#   scripts/a500-assets-smoke.sh [binary]
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
docker_dir="$(dirname "${script_dir}")"
repo_dir="$(dirname "${docker_dir}")"
cd "${repo_dir}"

binary="${1:-build/clorraine/imageviewer/hosted-amiga-13/imageviewer}"
fixture="${repo_dir}/examples/amiga/assets/clorraine.ilbm"
golden="${repo_dir}/docs/amiga-sdk/a500-assets-line.png"
frame_golden="${repo_dir}/docs/amiga-sdk/a500-assets.png"
work="${repo_dir}/build/amiga-sdk/assets-smoke"
work_rel="${work#"${repo_dir}/"}"
adf="${work_rel}/a500-assets.adf"
rom="${work_rel}/ks13.rom"
screenshot="${work}/screen.png"
crop="${work}/crop.png"
display="${work}/display.png"
live="${work}/live.png"
container="clorraine-a500-assets-$$"

if [ ! -f "${binary}" ]; then
    echo "a500-assets-smoke: missing binary: ${binary}" >&2
    echo "a500-assets-smoke: build it with --release --profile hosted-amiga-13 first" >&2
    exit 2
fi
if [ ! -f "${fixture}" ]; then
    echo "a500-assets-smoke: missing fixture: ${fixture}" >&2
    exit 2
fi
if [ ! -f "${golden}" ]; then
    echo "a500-assets-smoke: missing golden line: ${golden}" >&2
    exit 2
fi
if [ ! -f "${frame_golden}" ]; then
    echo "a500-assets-smoke: missing golden frame: ${frame_golden}" >&2
    exit 2
fi
if [ ! -f "${docker_dir}/.env" ]; then
    echo "a500-assets-smoke: docker/.env is missing; run 'make -C docker init'" >&2
    exit 2
fi
for tool in xdotool import convert compare sha1sum; do
    command -v "${tool}" >/dev/null 2>&1 \
        || { echo "a500-assets-smoke: missing host tool '${tool}'" >&2; exit 2; }
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
cp "${binary}" "${work}/imageviewer"
cp "${fixture}" "${work}/image.ilbm"
printf 'DF0:imageviewer\n' > "${work}/startup-sequence"
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
    echo "a500-assets-smoke: decrypted ROM has unexpected SHA-1 ${sha1}" >&2
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
xdftool write "/work/${work_rel}/imageviewer" imageviewer + protect imageviewer +re >/dev/null
xdftool write "/work/${work_rel}/image.ilbm" image.ilbm >/dev/null
xdftool write "/work/${work_rel}/startup-sequence" startup-sequence >/dev/null
xdftool makedir S + write "/work/${work_rel}/S/startup-sequence" S/startup-sequence >/dev/null
xdftool boot install boot1x >/dev/null
if ! xdftool list | grep -q ' startup-sequence'; then
    echo "a500-assets-smoke: the ADF has no root startup-sequence" >&2
    exit 1
fi
if ! xdftool list | grep -q ' image.ilbm'; then
    echo "a500-assets-smoke: the ADF has no image.ilbm" >&2
    exit 1
fi

# Boot the real A500/KS1.3 machine. The ADF takes roughly forty seconds to
# reach the startup sequence; the viewer then runs the DOS self-test, loads and
# displays the image, and holds the final frame for about twenty-four seconds
# before restoring the shell and printing the report line.
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
    echo "a500-assets-smoke: no FS-UAE window" >&2
    tail -20 "${work}/fs-uae.log" >&2 || true
    exit 1
fi

frame_matched=0
frame_stable=0
line_matched=0
step=0
while [ "${step}" -lt 22 ]; do
    step=$((step + 1))
    sleep 5
    DISPLAY="${DISPLAY}" import -window "${window}" "${screenshot}"
    # The held acceptance frame is deterministic; comparing the display crop
    # proves the decoded image, the palette, and the caption. A second
    # consecutive match proves the hold is stable rather than a transient.
    convert "${screenshot}" -crop 414x410+418+98 +repage "${display}"
    frame_metric="$(compare -metric AE "${display}" "${frame_golden}" null: 2>&1 || true)"
    frame_metric="${frame_metric%% *}"
    if [ "${frame_metric}" = "0" ]; then
        if [ "${frame_matched}" -eq 0 ]; then
            frame_matched=1
            cp "${screenshot}" "${live}"
        else
            frame_stable=1
        fi
    elif [ "${frame_stable}" -eq 0 ]; then
        frame_matched=0
    fi
    convert "${screenshot}" -crop 620x22+160+88 +repage "${crop}"
    metric="$(compare -metric AE "${crop}" "${golden}" null: 2>&1 || true)"
    metric="${metric%% *}"
    if [ "${metric}" = "0" ]; then
        line_matched=1
    fi
    if [ "${frame_matched}" -eq 1 ] && [ "${frame_stable}" -eq 1 ] && [ "${line_matched}" -eq 1 ]; then
        break
    fi
done

docker stop -t 5 "${container}" >/dev/null 2>&1 || true
wait "${runner}" 2>/dev/null || true

if [ "${frame_matched}" -ne 1 ]; then
    echo "a500-assets-smoke: held display never matched ${frame_golden}" >&2
    echo "a500-assets-smoke: last display and screen kept at ${display} and ${screenshot}" >&2
    exit 1
fi
if [ "${frame_stable}" -ne 1 ]; then
    echo "a500-assets-smoke: the held display was never seen twice" >&2
    exit 1
fi
if [ "${line_matched}" -ne 1 ]; then
    echo "a500-assets-smoke: report line never matched ${golden}" >&2
    echo "a500-assets-smoke: last crop and screen kept at ${crop} and ${screenshot}" >&2
    exit 1
fi

echo "a500-assets-smoke: OK (held frame: ${live}; report line: ${golden})"
