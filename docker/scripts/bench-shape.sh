#!/usr/bin/env bash
# Build the generated-C shape fixture with both m68k toolchains and the
# freestanding profile. This is a build-only leg: no emulator or ROM is used.
#
# Run inside the pinned SDK container from the repository root:
#   docker/scripts/bench-shape.sh
#
# Environment:
#   CLC  compiler binary (default: ./target/debug/clc)
#   GCC  m68k GCC       (default: m68k-amigaos-gcc)
#   VC   vbcc front end (default: vc)
set -euo pipefail

CLC="${CLC:-./target/debug/clc}"
GCC="${GCC:-m68k-amigaos-gcc}"
VC="${VC:-vc}"
OUTDIR="${OUTDIR:-build/perf/shapes}"
case "${OUTDIR}" in
    build/perf/*) ;;
    *) echo "bench-shape: OUTDIR must stay under build/perf/" >&2; exit 2 ;;
esac
case "${OUTDIR}" in
    *..*) echo "bench-shape: OUTDIR must not contain '..'" >&2; exit 2 ;;
esac
FIXTURE="${FIXTURE:-tools/perf/fixtures/shapes.clo}"

GCC_FLAGS=(-std=c99 -mcrt=nix20 -m68000 -fwrapv -O2 -Wall -Wextra -Werror)
GCC13_FLAGS=(-std=c99 -mcrt=nix13 -m68000 -fwrapv -O2 -Wall -Wextra -Werror)
VBCC_FLAGS=(+aos68k -c99 -O2 -cpu=68000 -I/opt/amiga/m68k-amigaos/ndk-include)
FREESTANDING_FLAGS=(-std=c99 -ffreestanding -fno-builtin -nostdlib -m68000 -fwrapv -O2 -Wall -Wextra -Werror)

rm -rf "${OUTDIR}"
mkdir -p "${OUTDIR}"

"${CLC}" --profile hosted-amiga -o "${OUTDIR}/shapes.c" "${FIXTURE}"
"${GCC}" "${GCC_FLAGS[@]}" -o "${OUTDIR}/shapes-gcc" "${OUTDIR}/shapes.c" -lamiga -lm
echo "bench-shape: gcc build is warning-free"

"${CLC}" --profile hosted-amiga-13 -o "${OUTDIR}/shapes13.c" "${FIXTURE}"
# nix13 is a build-only leg: linking a floating-point program needs the 1.3
# math bases, so compile the translation unit and stop there.
"${GCC}" "${GCC13_FLAGS[@]}" -c -o "${OUTDIR}/shapes13.o" "${OUTDIR}/shapes13.c"
echo "bench-shape: gcc nix13 compile is warning-free"

"${VC}" "${VBCC_FLAGS[@]}" -o "${OUTDIR}/shapes-vbcc" "${OUTDIR}/shapes.c" \
    -lamiga -lmieee > "${OUTDIR}/vbcc.log" 2>&1
if grep -i 'warning' "${OUTDIR}/vbcc.log" | grep -q 'shapes\.c'; then
    echo "bench-shape: vbcc emitted warnings in the emitted C" >&2
    cat "${OUTDIR}/vbcc.log" >&2
    exit 1
fi
echo "bench-shape: vbcc build is warning-free"

# The freestanding leg is build-only; a floating-point program needs the
# target's soft-float math bases to link, so compile the translation unit.
"${CLC}" --profile freestanding -o "${OUTDIR}/shapes-free.c" "${FIXTURE}"
"${GCC}" "${FREESTANDING_FLAGS[@]}" -c -o "${OUTDIR}/shapes-free.o" \
    "${OUTDIR}/shapes-free.c"
echo "bench-shape: freestanding compile is warning-free"
