#!/usr/bin/env bash
# Verifier A500/KS1.3 runner (host). Boots a copy of the licensed Workbench
# 1.3.5 HDF (DH0) with a replaced S:startup-sequence plus a host directory
# drive OUT: for the results, on a private Xvfb display. The decrypted ROM and
# every copy live under build/tmp/assessment/verify/a500 and the ROM is removed
# on exit. Usage: run-a500.sh <case> <startup-sequence file> <bin>...
#   T=seconds (default 240), CLICK=seconds (inject one left click at that time)
set -uo pipefail
repo="$(git rev-parse --show-toplevel)"
cd "$repo"
set -a; . docker/.env; set +a
image="${IMAGE:-clorraine-sdk:dev}"
T="${T:-240}"
case_name=$1; seq_file=$2; shift 2
base=build/tmp/assessment/verify/a500
work=$base/$case_name
rom=$base/ks13.rom
num=$(( ($$ % 50) + 230 ))
while [ -e "/tmp/.X11-unix/X$num" ]; do num=$((num + 1)); done
xvfb="clorraine-verify13-xvfb-$$"
uae="clorraine-verify13-uae-$$"
cleanup() { docker rm -f "$xvfb" "$uae" >/dev/null 2>&1; rm -f "/tmp/.X11-unix/X$num" "$rom"; }
trap cleanup EXIT
rm -rf "$work"; mkdir -p "$work/out"
cp "$AMIGA_DATA/hdf/workbench-135.hdf" "$work/wb.hdf"; chmod u+w "$work/wb.hdf"
cp "$seq_file" "$work/startup-sequence"
for f in "$@"; do cp "$f" "$work/"; done
dc() { (cd docker && docker compose -p clorraine --env-file .env -f compose.yaml run --rm -T dev "$@") >/dev/null 2>&1; }
dc python3 -c "from amitools.rom.kickrom import Loader; data = Loader.load('/amiga/roms/amiga-os-130.rom', '/amiga/roms/rom.key'); open('/work/$rom', 'wb').write(data)"
[ "$(sha1sum "$rom" | awk '{print $1}')" = 891e9a547772fe0c6c19b610baf8bc4ea7fcb785 ] || { echo "bad rom"; exit 1; }
for f in "$@"; do
    n=$(basename "$f")
    dc xdftool "/work/$work/wb.hdf" write "/work/$work/$n" "$n" + protect "$n" +re
done
dc xdftool "/work/$work/wb.hdf" delete S/startup-sequence
dc xdftool "/work/$work/wb.hdf" write "/work/$work/startup-sequence" S/startup-sequence
docker run -d --rm --name "$xvfb" -v /tmp/.X11-unix:/tmp/.X11-unix "$image" \
    Xvfb ":$num" -screen 0 1280x720x24 -ac +extension GLX -noreset >/dev/null
for _ in $(seq 1 100); do [ -S "/tmp/.X11-unix/X$num" ] && break; sleep 0.1; done
docker run -d --rm --name "$uae" -e DISPLAY=":$num" -e SDL_VIDEO_X11_XINPUT2=0 \
    -v "$repo":/work -v "$AMIGA_DATA":/amiga:ro -v /tmp/.X11-unix:/tmp/.X11-unix \
    "$image" fs-uae --amiga-model=A500 --kickstart-file="/work/$rom" \
    "--hard-drive-0=/work/$work/wb.hdf" --hard-drive-0-label=DH0 --hard-drive-0-priority=0 \
    "--hard-drive-1=/work/$work/out" --hard-drive-1-label=OUT --hard-drive-1-priority=-5 \
    --window-width=640 --window-height=512 --window-border=0 >/dev/null
found=0; clicked=0; t=0
while [ "$t" -lt "$T" ]; do
    sleep 2; t=$((t + 2))
    if [ -f "$work/out/end.txt" ]; then found=1; break; fi
    if [ -n "${CLICK:-}" ] && [ "$clicked" = 0 ] && [ "$t" -ge "$CLICK" ]; then
        DISPLAY=":$num" import -window root "$work/shot-before-click.png" >/dev/null 2>&1 || true
        DISPLAY=":$num" xdotool mousemove 320 200 click 1 >/dev/null 2>&1 || true
        sleep 0.3
        DISPLAY=":$num" xdotool click 1 >/dev/null 2>&1 || true
        clicked=1
    fi
done
DISPLAY=":$num" import -window root "$work/shot.png" >/dev/null 2>&1 || true
docker rm -f "$uae" >/dev/null 2>&1
echo "== $case_name (t=$t s)"
for f in "$work"/out/*; do [ -f "$f" ] && { echo "-- $(basename "$f")"; tr -d '\r' < "$f"; }; done
[ "$found" = 1 ] || echo "no END within $T s (hung, crashed, or blocked)"
