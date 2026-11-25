#!/usr/bin/env bash
# WS5 FS-UAE probes (540, 541) on an A1200 with Kickstart 3.1, booted from a
# directory hard drive on a private Xvfb display (no desktop window).
#
# Each case boots a clean drive whose startup-sequence runs the system-state
# walker (54x-walker.c), the probe program, the walker again, and writes END.
# Output lands in <out>/run-<case>/out.txt. A case whose END line never appears
# within the timeout is reported as "no END" (the machine hung or crashed).
#
# Usage (repository root):
#   54x-fs-uae.sh build <clc> <out>     # inside the SDK container
#   54x-fs-uae.sh run <out> <case>...   # on the host (needs docker)
# Cases: 540 540c 541 541c 542 542c seeded; a trailing "f" (541f, 541cf) also runs
# 54x-filler.c after the second walker, which overwrites free memory with the
# ILLEGAL opcode while VBlank interrupts run. "c" is the contrasting input; "seeded" runs
# 54x-seeded.c, which leaves a VERTB server and the ClProbe541 port installed
# and removes card.resource, so the walker must report all three.
set -uo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
mode=$1; shift

amiga_build() {  # src.c out
    m68k-amigaos-gcc -std=c99 -mcrt=nix20 -m68000 -fwrapv -O2 -ffunction-sections \
        -fdata-sections -Wl,--gc-sections -s -o "$2" "$1" -lamiga -lm
}

if [ "$mode" = build ]; then
    clc=$1; out=$2; mkdir -p "$out/src"
    amiga_build "$here/54x-walker.c" "$out/walker" || exit 1
    amiga_build "$here/54x-seeded.c" "$out/seeded" || exit 1
    amiga_build "$here/54x-filler.c" "$out/filler" || exit 1
    cp "$here/540-card-resource.clo" "$out/src/540.clo"
    sed 's/let b: bool = cardPresent();/let b: bool = true;/; s/let a: bool = cardPresent();/let a: bool = true;/' \
        "$here/540-card-resource.clo" > "$out/src/540c.clo"
    cp "$here/541-fault-handlers.clo" "$out/src/541.clo"
    sed 's/pick(5)/pick(1)/' "$here/541-fault-handlers.clo" > "$out/src/541c.clo"
    cp "$here/542-null-intuition-base.clo" "$out/src/542.clo"
    sed 's/    println("calling");/    let s: Screen? = openScreen(320, 200, 2, "probe542");\n    println("calling");/' \
        "$here/542-null-intuition-base.clo" > "$out/src/542c.clo"
    for c in 540 540c 541 541c 542 542c; do
        "$clc" --profile hosted-amiga -o "$out/src/$c.c" "$out/src/$c.clo" || exit 1
        amiga_build "$out/src/$c.c" "$out/$c" || exit 1
    done
    echo "built"
    exit 0
fi

out=$1; shift
repo="$(git rev-parse --show-toplevel)"
. "$repo/docker/.env"
image="${IMAGE:-clorraine-sdk:dev}"
num=$(( ($$ % 50) + 120 ))
while [ -e "/tmp/.X11-unix/X$num" ]; do num=$((num + 1)); done
xvfb="clorraine-ws5-xvfb-$$"
cleanup() { docker rm -f "$xvfb" "clorraine-ws5-uae-$$" >/dev/null 2>&1; rm -f "/tmp/.X11-unix/X$num"; }
trap cleanup EXIT
docker run -d --rm --name "$xvfb" -v /tmp/.X11-unix:/tmp/.X11-unix "$image" \
    Xvfb ":$num" -screen 0 1280x720x24 -ac +extension GLX -noreset >/dev/null
for _ in $(seq 1 100); do [ -S "/tmp/.X11-unix/X$num" ] && break; sleep 0.1; done

for c in "$@"; do
    d="$out/run-$c"; rm -rf "$d"; mkdir -p "$d/s"
    base="${c%f}"; fill=""
    [ "$base" != "$c" ] && fill="filler >>CNEXT:out.txt"
    cp "$out/walker" "$d/walker"; cp "$out/filler" "$d/filler"; cp "$out/$base" "$d/prog"
    cat > "$d/s/startup-sequence" <<EOF
failat 100
walker before >CNEXT:out.txt
prog >>CNEXT:out.txt
echo "rc \$RC" >>CNEXT:out.txt
walker after >>CNEXT:out.txt
$fill
echo "END" >>CNEXT:out.txt
EOF
    rel="${d#"$repo"/}"
    docker run -d --rm --name "clorraine-ws5-uae-$$" -e DISPLAY=":$num" -e SDL_VIDEO_X11_XINPUT2=0 \
        -v "$repo":/work -v "$AMIGA_DATA":/amiga:ro -v /tmp/.X11-unix:/tmp/.X11-unix \
        "$image" fs-uae --amiga-model=A1200 --kickstart-file=/amiga/roms/amiga-os-310-a1200.rom \
        "--hard-drive-0=/work/$rel" --hard-drive-0-label=CNEXT --hard-drive-0-priority=0 \
        >/dev/null
    found=0
    for _ in $(seq 1 90); do
        if [ -f "$d/out.txt" ] && grep -q '^END' "$d/out.txt"; then found=1; break; fi
        sleep 2
    done
    docker rm -f "clorraine-ws5-uae-$$" >/dev/null 2>&1
    echo "== $c"
    tr -d '\r' < "$d/out.txt" 2>/dev/null || echo "(no out.txt)"
    [ "$found" = 1 ] || echo "no END within 180 s (hung or crashed)"
done
