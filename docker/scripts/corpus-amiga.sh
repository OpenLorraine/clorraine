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
PROGRAMS=(file-copier string-parser buffer-container)

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
# for warnings, because the leg must be warning-free, not merely successful.
echo "corpus-amiga: file-copier: vbcc build-only"
vbcc_log="${OUTDIR}/file-copier/vbcc.log"
"${VC}" "${VBCC_FLAGS[@]}" -o "${OUTDIR}/file-copier/program-vbcc" \
    "${OUTDIR}/file-copier/program.c" -lamiga -lmieee 2>&1 | tee "${vbcc_log}"
if grep -qi 'warning' "${vbcc_log}"; then
    echo "corpus-amiga: file-copier: vbcc emitted warnings" >&2
    status=1
else
    echo "corpus-amiga: file-copier: vbcc build is warning-free"
fi

if [ "${status}" -ne 0 ]; then
    echo "corpus-amiga: FAILED" >&2
else
    echo "corpus-amiga: all portable programs match on hosted-amiga"
fi
exit "${status}"
