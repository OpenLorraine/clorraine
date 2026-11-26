#!/usr/bin/env bash
# Verifier A1200/KS3.1 runner (host). Each argument is a prepared directory
# drive (binaries + s/startup-sequence) under the repository; waits for END in
# out.txt (timeout seconds in $T, default 180) and takes a private-Xvfb
# screenshot at the end. Same method as probes/WS5/54x-fs-uae.sh.
set -uo pipefail
repo="$(git rev-parse --show-toplevel)"
. "$repo/docker/.env"
image="${IMAGE:-clorraine-sdk:dev}"
T="${T:-180}"
num=$(( ($$ % 50) + 170 ))
while [ -e "/tmp/.X11-unix/X$num" ]; do num=$((num + 1)); done
xvfb="clorraine-verify-xvfb-$$"
uae="clorraine-verify-uae-$$"
cleanup() { docker rm -f "$xvfb" "$uae" >/dev/null 2>&1; rm -f "/tmp/.X11-unix/X$num"; }
trap cleanup EXIT
docker run -d --rm --name "$xvfb" -v /tmp/.X11-unix:/tmp/.X11-unix "$image" \
    Xvfb ":$num" -screen 0 1280x720x24 -ac +extension GLX -noreset >/dev/null
for _ in $(seq 1 100); do [ -S "/tmp/.X11-unix/X$num" ] && break; sleep 0.1; done
for d in "$@"; do
    d="$(cd "$d" && pwd)"; rm -f "$d/out.txt" "$d/shot.png"
    rel="${d#"$repo"/}"
    docker run -d --rm --name "$uae" -e DISPLAY=":$num" -e SDL_VIDEO_X11_XINPUT2=0 \
        -v "$repo":/work -v "$AMIGA_DATA":/amiga:ro -v /tmp/.X11-unix:/tmp/.X11-unix \
        "$image" fs-uae --amiga-model=A1200 --kickstart-file=/amiga/roms/amiga-os-310-a1200.rom \
        "--hard-drive-0=/work/$rel" --hard-drive-0-label=CNEXT --hard-drive-0-priority=0 \
        >/dev/null
    found=0
    for _ in $(seq 1 $((T / 2))); do
        if [ -f "$d/out.txt" ] && grep -q '^END' "$d/out.txt"; then found=1; break; fi
        sleep 2
    done
    docker run --rm -v /tmp/.X11-unix:/tmp/.X11-unix -v "$repo":/work "$image" \
        bash -c "DISPLAY=:$num import -window root /work/$rel/shot.png" >/dev/null 2>&1 \
        || DISPLAY=":$num" import -window root "$d/shot.png" >/dev/null 2>&1 || true
    docker rm -f "$uae" >/dev/null 2>&1
    echo "== $rel"
    tr -d '\r' < "$d/out.txt" 2>/dev/null || echo "(no out.txt)"
    [ "$found" = 1 ] || echo "no END within $T s (hung or crashed)"
done
