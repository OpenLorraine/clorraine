#!/usr/bin/env bash
# Build and run the portable validation corpus on the hosted-amiga profile.
#
# The compiler emits one C translation unit per program. This script builds it
# with the m68k GCC toolchain, runs it under the `vamos` emulator, and diffs
# the captured stdout against the program's recorded output. A vbcc build-only
# leg proves the same translation unit also compiles warning-free with the
# second toolchain from the compile matrix. A program that creates files (the
# file-copier round-trip) runs inside a private working directory, so nothing
# outside the build output is touched.
#
# Run inside the SDK container from the repository root:
#   docker/scripts/corpus-amiga.sh
#
# Environment overrides:
#   CLC     compiler binary          (default: ./target/debug/clc)
#   GCC     m68k GCC command         (default: m68k-amigaos-gcc)
#   VC      vbcc front end           (default: vc)
#   OUTDIR  build output directory   (default: build/corpus/amiga)
#   CORPUS  corpus directory         (default: spec/tests/corpus)
#   VAMOS   emulator command         (default: vamos)
set -euo pipefail

CLC="${CLC:-./target/debug/clc}"
GCC="${GCC:-m68k-amigaos-gcc}"
VC="${VC:-vc}"
OUTDIR="${OUTDIR:-build/corpus/amiga}"
CORPUS="${CORPUS:-spec/tests/corpus}"
VAMOS="${VAMOS:-vamos}"

# The exact lines from the compile matrix for the two m68k toolchains.
GCC_FLAGS=(-std=c99 -mcrt=nix20 -m68000 -fwrapv -O0 -g -Wall -Wextra -Werror)
VBCC_FLAGS=(+aos68k -c99 -O2 -cpu=68000 -I/opt/amiga/m68k-amigaos/ndk-include)

# The portable programs: each prints its recorded output and exits zero.
PROGRAMS=(file-copier string-parser buffer-container volatile-mmio)

if [ ! -x "${CLC}" ]; then
    echo "corpus-amiga: compiler not found: ${CLC}" >&2
    exit 2
fi

status=0
for name in "${PROGRAMS[@]}"; do
    src="${CORPUS}/${name}/program.clo"
    expected="${CORPUS}/${name}/expected-output.txt"
    workdir="${OUTDIR}/${name}"
    rundir="${workdir}/run"
    rm -rf "${workdir}"
    mkdir -p "${rundir}"

    echo "corpus-amiga: ${name}: emit and build"

    "${CLC}" --profile hosted-amiga -o "${workdir}/program.c" "${src}"
    "${GCC}" "${GCC_FLAGS[@]}" -o "${workdir}/program" "${workdir}/program.c" -lamiga -lm

    # The program path and output path are absolute so the run can use the
    # private working directory without losing the build output.
    abs_program="$(cd "${workdir}" && pwd)/program"
    abs_out="$(cd "${workdir}" && pwd)/out.txt"
    ( cd "${rundir}" && timeout 120 "${VAMOS}" "${abs_program}" | tr -d '\r' > "${abs_out}" )

    if diff -u "${expected}" "${abs_out}"; then
        echo "corpus-amiga: ${name}: output matches"
    else
        echo "corpus-amiga: ${name}: output differs" >&2
        status=1
    fi
done

# The vbcc leg is build-only: it proves the same emitted C compiles with the
# second toolchain from the compile matrix. Its output is captured and scanned
# for warnings in the generated translation unit, because the leg must be
# warning-free, not merely successful. The scan is scoped to the emitted C: the
# NDK system headers it includes carry warnings of their own under vbcc, which
# are outside this leg's contract.
echo "corpus-amiga: file-copier: vbcc build-only"
vbcc_log="${OUTDIR}/file-copier/vbcc.log"
"${VC}" "${VBCC_FLAGS[@]}" -o "${OUTDIR}/file-copier/program-vbcc" \
    "${OUTDIR}/file-copier/program.c" -lamiga -lmieee 2>&1 | tee "${vbcc_log}"
if grep -i 'warning' "${vbcc_log}" | grep -q 'program\.c'; then
    echo "corpus-amiga: file-copier: vbcc emitted warnings in the emitted C" >&2
    status=1
else
    echo "corpus-amiga: file-copier: vbcc build is warning-free"
fi

# The interrupt-counter is hosted-amiga only and needs the real interrupt
# dispatcher, so it cannot run under `vamos`. It is built here because the
# FS-UAE leg consumes the binary; that leg boots the emulator separately.
echo "corpus-amiga: interrupt-counter: build-only"
interrupt_dir="${OUTDIR}/interrupt-counter"
mkdir -p "${interrupt_dir}"
"${CLC}" --profile hosted-amiga -o "${interrupt_dir}/program.c" \
    "${CORPUS}/interrupt-counter/program.clo"
"${GCC}" "${GCC_FLAGS[@]}" -o "${interrupt_dir}/program" \
    "${interrupt_dir}/program.c" -lamiga -lm
echo "corpus-amiga: interrupt-counter: built"

# The graphics program is hosted-amiga only and opens a real screen, which
# `vamos` cannot render; it is built here because the FS-UAE leg consumes the
# binary.
echo "corpus-amiga: graphics: build-only"
graphics_dir="${OUTDIR}/graphics"
mkdir -p "${graphics_dir}"
"${CLC}" --profile hosted-amiga -o "${graphics_dir}/program.c" \
    "${CORPUS}/graphics/program.clo"
"${GCC}" "${GCC_FLAGS[@]}" -o "${graphics_dir}/program" \
    "${graphics_dir}/program.c" -lamiga -lm
echo "corpus-amiga: graphics: built"

if [ "${status}" -ne 0 ]; then
    echo "corpus-amiga: FAILED" >&2
else
    echo "corpus-amiga: all portable programs match on hosted-amiga"
fi
exit "${status}"
