#!/usr/bin/env bash
# A600/Kickstart 2.05 Workbench services and intuition smoke.
#
# Boots a clean directory HD with the licensed Workbench 2.04/2.1 libraries,
# runs the Exec/args/services demos with serial/parallel host peers, then runs
# the gadtools/ASL demo under a private Xvfb. No host window is shown.
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
docker_dir="$(dirname "${script_dir}")"
repo_dir="$(dirname "${docker_dir}")"
cd "${repo_dir}"

# shellcheck disable=SC1091
. "${script_dir}/workbench-media.sh"

expected_exec="exec: ok sem 1 hook 1 tag 1 date 1"
expected_args="args: ok n 3 file 1 num 7 sw 1"
expected_regions="regions: ok bounds 10 10 20 20"
expected_eclock="eclock: ok 709379"
expected_cmd="cmd: ok broker 1 dup 1 reopen 1"
expected_locale="locale: ok name 1 cc 1 gmt 1 fmt 1"
expected_rexx="rexx: ok action 1"
expected_clip="clip: ok n 4"
expected_ser="ser: ok open 1"
expected_par="par: ok open 1"
expected_prt="prt: ok open 1"
expected_gt="gt: ok gad 4"
expected_asl="asl: ok path 1"

work="${repo_dir}/build/amiga-sdk/a600-addons"
hd="${work}/hd"
work_rel="${work#${repo_dir}/}"
rom="${work}/ks205.rom"
container="clorraine-a600-addons-$$"
xvfb_num=$(( ($$ % 50) + 110 ))
while [ -e "/tmp/.X11-unix/X${xvfb_num}" ]; do
    xvfb_num=$((xvfb_num + 1))
done
xvfb_display=":${xvfb_num}"
xvfb_pid=""

for bin in \
    build/clorraine/optional-exec20/hosted-amiga/optional-exec20 \
    build/clorraine/optional-args/hosted-amiga/optional-args \
    build/clorraine/optional-services/hosted-amiga/optional-services \
    build/clorraine/optional-intuition/hosted-amiga/optional-intuition; do
    if [ ! -f "${bin}" ]; then
        echo "a600-addons-smoke: missing binary: ${bin}" >&2
        echo "a600-addons-smoke: build it with make sdk-advanced-build" >&2
        exit 2
    fi
done
for tool in Xvfb xdotool convert compare sha1sum python3; do
    command -v "${tool}" >/dev/null 2>&1 \
        || { echo "a600-addons-smoke: missing host tool '${tool}'" >&2; exit 2; }
done
if [ ! -f "${docker_dir}/.env" ]; then
    echo "a600-addons-smoke: docker/.env is missing" >&2
    exit 2
fi

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
    rm -f "${rom}" "${work}/ser.pid" "${work}/par.pid"
}
trap cleanup EXIT

rm -rf "${work}"
mkdir -p "${hd}/s"
wb_dirs "${hd}"
stage204="$(wb_stage /amiga/adf/amiga-os-204-workbench.adf "${work}/stage204")"
wb_copy "${stage204}" "${hd}" Libs Devs C
if [ -f "${stage204}/System/RexxMast" ]; then
    cp "${stage204}/System/RexxMast" "${hd}/RexxMast"
    chmod +x "${hd}/RexxMast"
fi
stage210="$(wb_stage /amiga/adf/amiga-os-210-workbench.adf "${work}/stage210")"
wb_copy "${stage210}" "${hd}" Libs
stage210loc="$(wb_stage_any /amiga/adf/amiga-os-210-locale.adf "${work}/stage210loc")"
for tree in Catalogs Languages Countries; do
    if [ -d "${stage210loc}/${tree}" ]; then
        cp -a "${stage210loc}/${tree}" "${hd}/Locale/"
    fi
done

cp build/clorraine/optional-exec20/hosted-amiga/optional-exec20 "${hd}/optional-exec20"
cp build/clorraine/optional-args/hosted-amiga/optional-args "${hd}/optional-args"
cp build/clorraine/optional-services/hosted-amiga/optional-services "${hd}/optional-services"
cp build/clorraine/optional-intuition/hosted-amiga/optional-intuition "${hd}/optional-intuition"

{
    wb_assigns "${hd}"
    cat <<'EOF'
Run >NIL: CNEXT:RexxMast
CNEXT:optional-exec20 >CNEXT:exec.txt
CNEXT:optional-args >CNEXT:args.txt
CNEXT:optional-services >CNEXT:services.txt
CNEXT:optional-intuition >CNEXT:intuition.txt
EOF
} > "${hd}/s/startup-sequence"

(
    cd "${docker_dir}"
    docker compose -p clorraine --env-file .env -f compose.yaml run --rm -T dev \
        python3 -c "from amitools.rom.kickrom import Loader; data = Loader.load('/amiga/roms/amiga-os-205-a600.rom', '/amiga/roms/rom.key'); open('/work/${rom#${repo_dir}/}', 'wb').write(data)"
)
sha1="$(sha1sum "${rom}" | awk '{print $1}')"
if [ "${sha1}" != "02843c4253bbd29aba535b0aa3bd9a85034ecde4" ]; then
    echo "a600-addons-smoke: decrypted ROM has unexpected SHA-1 ${sha1}" >&2
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

for _ in $(seq 1 90); do
    sleep 1
    if [ -f "${hd}/exec.txt" ] && [ -f "${hd}/args.txt" ] &&
       [ -f "${hd}/svc-cmd.txt" ] && [ -f "${hd}/svc-prt.txt" ] &&
       [ -f "${hd}/asl-ready.txt" ]; then
        break
    fi
done

# ASL is modal; drive it with the same focus/key pattern as the A500 GUI gate.
window=""
if [ -f "${hd}/asl-ready.txt" ]; then
    for _ in $(seq 1 30); do
        window="$(DISPLAY="${xvfb_display}" xdotool search --name 'FS-UAE' 2>/dev/null | tail -1 || true)"
        if [ -n "${window}" ]; then
            DISPLAY="${xvfb_display}" xdotool windowactivate --sync "${window}" 2>/dev/null || true
            DISPLAY="${xvfb_display}" xdotool key --clearmodifiers Return 2>/dev/null || true
            sleep 1
            if [ -f "${hd}/asl-done.txt" ]; then
                break
            fi
        fi
        sleep 1
    done
fi

sleep 2
docker stop -t 8 "${container}" >/dev/null 2>&1 || true
wait "${runner}" 2>/dev/null || true

for file in exec.txt args.txt svc-cmd.txt svc-locale.txt svc-rexx.txt \
    svc-clip.txt svc-ser.txt svc-par.txt svc-prt.txt intuition.txt asl-done.txt; do
    if [ ! -f "${hd}/${file}" ]; then
        echo "a600-addons-smoke: missing ${file}" >&2
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
        echo "a600-addons-smoke: ${file} mismatch" >&2
        echo "  expected: ${expected}" >&2
        echo "  actual:   ${actual}" >&2
        exit 1
    fi
}
check exec.txt "${expected_exec}"
check svc-cmd.txt "${expected_cmd}"
check svc-locale.txt "${expected_locale}"
check svc-rexx.txt "${expected_rexx}"
check svc-clip.txt "${expected_clip}"
check svc-ser.txt "${expected_ser}"
check svc-par.txt "${expected_par}"
check svc-prt.txt "${expected_prt}"
check asl-done.txt "ready"
if ! grep -q "${expected_args}" "${hd}/args.txt" ||
   ! grep -q "${expected_regions}" "${hd}/args.txt" ||
   ! grep -q "${expected_eclock}" "${hd}/args.txt"; then
    echo "a600-addons-smoke: args/regions/eclock lines missing" >&2
    cat "${hd}/args.txt" >&2
    exit 1
fi
if ! grep -q "${expected_gt}" "${hd}/intuition.txt" ||
   ! grep -q "${expected_asl}" "${hd}/intuition.txt"; then
    echo "a600-addons-smoke: intuition lines missing" >&2
    cat "${hd}/intuition.txt" >&2
    exit 1
fi
if ! grep -aq 'asl.txt' "${hd}/asl-path.txt" 2>/dev/null; then
    echo "a600-addons-smoke: ASL path missing" >&2
    exit 1
fi

python3 "${script_dir}/cscr2png.py" "${hd}/a600-intuition.cscr" "${work}/intuition.png" --scale 2 >/dev/null
colors="$(identify -format '%k' "${work}/intuition.png" 2>/dev/null || echo 0)"
if [ "${colors}" -lt 2 ] 2>/dev/null; then
    echo "a600-addons-smoke: the intuition capture is blank" >&2
    exit 1
fi

echo "a600-addons-smoke: OK (${expected_exec}; ${expected_cmd}; ${expected_locale}; ${expected_rexx}; ${expected_ser}; ${expected_par}; ${expected_gt}; ${expected_asl})"
