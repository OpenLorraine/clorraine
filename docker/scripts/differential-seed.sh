#!/usr/bin/env bash
# Compare host and hosted-amiga stdout for the portable differential programs.
#
# For each program the script emits the host profile, builds it with the host
# `cc`, and runs it in a private working directory. It then emits the
# hosted-amiga profile, builds it with the m68k GCC toolchain, runs it under
# `vamos`, and diffs the two captured outputs. This is a convenience gate over
# the same recorded outputs the corpus pins, not an independent signal.
#
# Run inside the SDK container from the repository root:
#   docker/scripts/differential-seed.sh
#
# Environment overrides:
#   CLC       compiler binary        (default: ./target/debug/clc)
#   GCC       m68k GCC command       (default: m68k-amigaos-gcc)
#   VAMOS     emulator command       (default: vamos)
#   OUTDIR    build output directory (default: build/differential)
#   DIFF_ONLY 1 to re-diff an existing OUTDIR without rebuilding
set -euo pipefail
cd "$(dirname "$0")/../.."

CLC="${CLC:-./target/debug/clc}"
GCC="${GCC:-m68k-amigaos-gcc}"
VAMOS="${VAMOS:-vamos}"
OUTDIR="${OUTDIR:-build/differential}"
DIFF_ONLY="${DIFF_ONLY:-0}"

# The program list is fixed by the verification design; a future portable
# program joins only with a new decision.
PROGRAMS=(buffer-container file-copier string-parser)

GCC_FLAGS=(-std=c99 -mcrt=nix20 -m68000 -fwrapv -O0 -g -Wall -Wextra -Werror)

if [ "${DIFF_ONLY}" != "1" ]; then
    cargo build --workspace --quiet
    if [ ! -x "${CLC}" ]; then
        echo "differential: compiler not found: ${CLC}" >&2
        exit 2
    fi
fi

for name in "${PROGRAMS[@]}"; do
    workdir="${OUTDIR}/${name}"
    echo "differential: ${name}"

    if [ "${DIFF_ONLY}" = "1" ]; then
        diff -u "${workdir}/host/out.txt" "${workdir}/amiga/out.txt"
        continue
    fi

    src="spec/tests/corpus/${name}/program.clo"
    rm -rf "${workdir}"
    mkdir -p "${workdir}/host" "${workdir}/amiga"

    "${CLC}" --profile host -o "${workdir}/host/program.c" "${src}"
    cc -std=c99 -O0 -Wall -Wextra -Werror \
        -o "${workdir}/host/program" "${workdir}/host/program.c"
    host_dir="$(cd "${workdir}/host" && pwd)"
    ( cd "${host_dir}" && timeout 120 ./program | tr -d '\r' > out.txt )

    "${CLC}" --profile hosted-amiga -o "${workdir}/amiga/program.c" "${src}"
    "${GCC}" "${GCC_FLAGS[@]}" \
        -o "${workdir}/amiga/program" "${workdir}/amiga/program.c" -lamiga -lm
    amiga_dir="$(cd "${workdir}/amiga" && pwd)"
    ( cd "${amiga_dir}" && timeout 120 "${VAMOS}" "${amiga_dir}/program" \
        | tr -d '\r' > out.txt )

    if ! diff -u "${workdir}/host/out.txt" "${workdir}/amiga/out.txt"; then
        echo "differential: ${name}: output differs" >&2
        exit 1
    fi
    echo "differential: ${name}: output matches"
done

echo "differential: all portable programs match"
