#!/usr/bin/env bash
#
# Capture a hosted-amiga program running in FS-UAE (a real Kickstart) to a
# normalized 1100x620 MP4. demo/story.sh calls this once per emulator chapter;
# it also works on its own.
#
# Requirements:
#   - Docker with the SDK image built: make -C docker image
#   - ffmpeg and xwininfo on PATH
#   - An X display reachable from the host (docker/.env DISPLAY and XAUTHORITY)
#   - A real Kickstart in docker/.env (otherwise AROS is shown)
#
# Usage: ./demo/record-emulator.sh SOURCE_FILE OUTPUT.mp4

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if [ "$#" -lt 2 ]; then
    echo "usage: $0 SOURCE_FILE OUTPUT.mp4" >&2
    exit 2
fi

source_file="$1"
output="$2"
mkdir -p "$(dirname "$output")"

stem="$(basename "$source_file" .clo)"
program="build/clorraine/${stem}/hosted-amiga/${stem}"

env_file="docker/.env"
if [ ! -f "$env_file" ]; then
    echo "error: $env_file not found; run 'make -C docker init'" >&2
    exit 1
fi
set -a
# shellcheck disable=SC1090
. "$env_file"
set +a

for tool in ffmpeg xwininfo docker; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        echo "error: $tool is not on PATH" >&2
        exit 1
    fi
done
if [ -z "${DISPLAY:-}" ] || [ ! -f "${XAUTHORITY:-/nonexistent}" ]; then
    echo "error: DISPLAY or XAUTHORITY is not usable in $env_file" >&2
    exit 1
fi

log="${TMPDIR:-/tmp}/clorraine-emulator.log"

echo "building $program..."
make -s build FILE="$source_file" PROFILE=hosted-amiga

# Launch FS-UAE in the background and always tear it down on exit. The extra
# options keep the on-screen messages out of the recording: the CPU governor
# warning is a persistent warning, and the startup notifications are shortened
# so they have faded before the capture starts.
docker/scripts/run.sh "$program" \
    --governor-warning=0 --notification-duration=500 >"$log" 2>&1 &
emulator_pid=$!
cleanup() {
    kill "$emulator_pid" 2>/dev/null || true
    for name in $(docker ps --format '{{.Names}}' | grep '^clorraine-emulator-run-' || true); do
        docker rm -f "$name" >/dev/null 2>&1 || true
    done
}
trap cleanup EXIT

# Wait for the FS-UAE client window (class "fs-uae") and remember its id. The
# window manager may move it after it first appears, so the geometry is read
# again just before the capture, from the id, not from the first sighting.
window_id=""
for _ in $(seq 1 60); do
    # Read the whole tree (no early exit) so xwininfo is not killed by SIGPIPE
    # under `set -o pipefail`; keep the first matching window id.
    window_id="$(xwininfo -root -tree 2>/dev/null | awk '/\("fs-uae"/ { if (id == "") id = $1 } END { print id }')"
    [ -n "$window_id" ] && break
    sleep 0.5
done
if [ -z "$window_id" ]; then
    echo "error: the FS-UAE window did not appear; see $log" >&2
    exit 1
fi

# Let Kickstart finish booting and the startup notifications fade before
# capturing, so the recording shows the booted program and nothing else.
sleep 12

# Read the client window's current absolute position and size.
read_geometry() {
    xwininfo -id "$window_id" 2>/dev/null | awk '
        /Absolute upper-left X:/ { x = $4 }
        /Absolute upper-left Y:/ { y = $4 }
        /Width:/                 { w = $2 }
        /Height:/                { h = $2 }
        /Map State:/             { state = $3 }
        END { print x, y, w "x" h, state }'
}
geometry=""
for _ in $(seq 1 20); do
    geometry="$(read_geometry)"
    set -- $geometry
    if [ -n "${1:-}" ] && [ -n "${2:-}" ] && [ "${3:-x}" != "x" ] && [ "${4:-}" = "IsViewable" ]; then
        x="$1"
        y="$2"
        size="$3"
        break
    fi
    geometry=""
    sleep 0.5
done
if [ -z "$geometry" ]; then
    echo "error: the FS-UAE window geometry was not readable" >&2
    exit 1
fi

# x11grab wants an explicit screen; add .0 when DISPLAY omits it.
display_screen="$DISPLAY"
case "$display_screen" in
    *.*) ;;
    *) display_screen="$display_screen.0" ;;
esac

echo "capturing ${size} at +${x}+${y} for 9s..."
ffmpeg -y -f x11grab -framerate 25 -video_size "$size" \
    -i "${display_screen}+${x},${y}" -t 9 \
    -vf "scale=1100:620:force_original_aspect_ratio=decrease,pad=1100:620:(ow-iw)/2:(oh-ih)/2:color=black" \
    -pix_fmt yuv420p -an "$output" >/dev/null 2>&1

cleanup
trap - EXIT
echo "wrote $output"
