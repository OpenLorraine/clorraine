#!/usr/bin/env bash
# Run the six runtime-fault programs on the hosted-amiga profile under vamos.
#
# Each program must compile with zero diagnostics on the hosted-amiga profile,
# build warning-free with the m68k GCC toolchain, print the exact fault line as
# its first output line, and exit nonzero. The fault aborts before the cleanup
# ladder, so a clean exit is always a failure. The host C compiler cannot
# reproduce the m68k trap behavior, so this is the corpus's target leg for the
# runtime fault contract.
#
# Run inside the SDK container from the repository root:
#   docker/scripts/run-fault-amiga.sh
#
# Environment overrides:
#   CLC      compiler binary        (default: ./target/debug/clc)
#   GCC      m68k GCC command       (default: m68k-amigaos-gcc)
#   VAMOS    emulator command       (default: vamos)
#   OUTDIR   build output directory (default: build/corpus/run-fault)
#   RUN_DIR  run-suite directory    (default: spec/tests/run)
set -euo pipefail

CLC="${CLC:-./target/debug/clc}"
GCC="${GCC:-m68k-amigaos-gcc}"
VAMOS="${VAMOS:-vamos}"
OUTDIR="${OUTDIR:-build/corpus/run-fault}"
RUN_DIR="${RUN_DIR:-spec/tests/run}"

GCC_FLAGS=(-std=c99 -mcrt=nix20 -m68000 -fwrapv -O0 -g -Wall -Wextra -Werror)

# Each entry is `<program>|<expected first output line>`.
CASES=(
    "E11210_division_by_zero|error E11210: integer division by zero"
    "E11210_min_neg_one|error E11210: integer division by zero"
    "E11211_index_out_of_range|error E11211: index out of range"
    "E11212_float_to_int_range|error E11212: float-to-integer conversion out of range"
    "E11212_nan_float_to_int|error E11212: float-to-integer conversion out of range"
    "E11213_assertion_failed|error E11213: assertion failed"
)

if [ ! -x "${CLC}" ]; then
    echo "run-fault-amiga: compiler not found: ${CLC}" >&2
    exit 2
fi

status=0
for entry in "${CASES[@]}"; do
    name="${entry%%|*}"
    want="${entry#*|}"
    work="${OUTDIR}/${name}"
    rm -rf "${work}"
    mkdir -p "${work}"

    echo "run-fault-amiga: ${name}: emit and build"

    "${CLC}" --profile hosted-amiga -o "${work}/program.c" "${RUN_DIR}/${name}.clo"
    "${GCC}" "${GCC_FLAGS[@]}" -o "${work}/program" "${work}/program.c" -lamiga -lm

    abs_program="$(cd "${work}" && pwd)/program"
    abs_out="$(cd "${work}" && pwd)/out.txt"
    # The fault exits nonzero, which would abort the script under `set -e`, so
    # the exit code is captured instead of propagated.
    set +e
    ( cd "${work}" && timeout 120 "${VAMOS}" "${abs_program}" ) 2>&1 | tr -d '\r' > "${abs_out}"
    rc="${PIPESTATUS[0]}"
    set -e

    first="$(sed -n '1p' "${abs_out}")"
    if [ "${first}" != "${want}" ]; then
        echo "run-fault-amiga: ${name}: first line '${first}' != '${want}'" >&2
        status=1
    elif [ "${rc}" -eq 0 ]; then
        echo "run-fault-amiga: ${name}: the program exited 0" >&2
        status=1
    else
        echo "run-fault-amiga: ${name}: '${first}' (exit ${rc})"
    fi
done

if [ "${status}" -ne 0 ]; then
    echo "run-fault-amiga: FAILED" >&2
else
    echo "run-fault-amiga: all six faults match on hosted-amiga"
fi
exit "${status}"
