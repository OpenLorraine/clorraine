#!/usr/bin/env bash
# A500/Kickstart 1.3 runtime smoke.
#
# Decrypts the user's licensed Amiga Forever Kickstart 1.3 ROM locally (the
# decrypted image stays in gitignored build/ and is removed on exit), boots a
# clean ADF under FS-UAE configured as an A500, waits for the hello line, and
# captures the real AmigaDOS 1.3 screen. The captured text-line crop is compared
# to the committed golden image.
#
# Usage:
#   scripts/a500-smoke.sh [binary] ["expected line"]
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
docker_dir="$(dirname "${script_dir}")"
repo_dir="$(dirname "${docker_dir}")"
cd "${repo_dir}"

binary="${1:-build/clorraine/hello/hosted-amiga-13/hello}"
expected="${2:-Hello, Amiga, from Clorraine!}"
golden="${repo_dir}/docs/amiga-sdk/a500-hello.png"
work="${repo_dir}/build/amiga-sdk/a500-smoke"
work_rel="${work#"${repo_dir}/"}"
adf="${work_rel}/a500.adf"
rom="${work_rel}/ks13.rom"
screenshot="${work}/screen.png"
crop="${work}/crop.png"
container="clorraine-a500-smoke-$$"

if [ ! -f "${binary}" ]; then
    echo "a500-smoke: missing binary: ${binary}" >&2
    echo "a500-smoke: build it with --profile hosted-amiga-13 first" >&2
    exit 2
fi
if [ ! -f "${docker_dir}/.env" ]; then
    echo "a500-smoke: docker/.env is missing; run 'make -C docker init'" >&2
    exit 2
fi
for tool in xdotool import convert compare sha1sum; do
    command -v "${tool}" >/dev/null 2>&1 \
        || { echo "a500-smoke: missing host tool '${tool}'" >&2; exit 2; }
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
rm -f "${screenshot}" "${crop}" "${repo_dir}/${adf}" "${repo_dir}/${rom}"
cp "${binary}" "${work}/hello"
printf 'DF0:hello\n' > "${work}/startup-sequence"
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
    echo "a500-smoke: decrypted ROM has unexpected SHA-1 ${sha1}" >&2
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
xdftool write "/work/${work_rel}/hello" hello + protect hello +re >/dev/null
xdftool write "/work/${work_rel}/startup-sequence" startup-sequence >/dev/null
xdftool makedir S + write "/work/${work_rel}/S/startup-sequence" S/startup-sequence >/dev/null
xdftool boot install boot1x >/dev/null
if ! xdftool list | grep -q ' startup-sequence'; then
    echo "a500-smoke: the ADF has no root startup-sequence" >&2
    exit 1
fi

# Boot the real A500/KS1.3 machine and poll for the visible line. FS-UAE takes
# roughly forty seconds to reach the Shell in this environment; polling keeps
# the gate stable when the host is busy while still comparing the same golden.
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
    echo "a500-smoke: no FS-UAE window" >&2
    tail -20 "${work}/fs-uae.log" >&2 || true
    exit 1
fi
metric=1
step=0
while [ "${step}" -lt 15 ]; do
    step=$((step + 1))
    sleep 5
    DISPLAY="${DISPLAY}" import -window "${window}" "${screenshot}"
    convert "${screenshot}" -crop 430x22+160+88 +repage "${crop}"
    metric="$(compare -metric AE "${crop}" "${golden}" null: 2>&1 || true)"
    metric="${metric%% *}"
    if [ "${metric}" = "0" ]; then
        break
    fi
done
docker stop -t 5 "${container}" >/dev/null 2>&1 || true
wait "${runner}" 2>/dev/null || true

if [ "${metric}" != "0" ]; then
    echo "a500-smoke: captured line differs from ${golden} (AE=${metric})" >&2
    echo "a500-smoke: screen kept at ${screenshot}" >&2
    exit 1
fi

echo "a500-smoke: OK (FS-UAE A500/KS1.3 screenshot: ${screenshot})"
echo "a500-smoke: expected line: ${expected}"
