#!/usr/bin/env bash
# Run the eight runtime-fault programs on the hosted-amiga profile under vamos.
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
set -euo pipefail

CLC="${CLC:-./target/debug/clc}"
GCC="${GCC:-m68k-amigaos-gcc}"
VAMOS="${VAMOS:-vamos}"
OUTDIR="${OUTDIR:-build/corpus/run-fault}"

GCC_FLAGS=(-std=c99 -mcrt=nix20 -m68000 -fwrapv -O0 -g -Wall -Wextra -Werror)

# The hosted-amiga shim leaves the process with this code after a fault, so a
# genuine trap is exactly this value. A timeout (124) or any other code means
# the program did not reach its fault and must fail the leg.
TRAP_EXIT=20

# Each entry is `<program>|<source path>|<expected first output line>`.
# The six frozen faults stay under spec/tests/run; the two closeout probes live
# under examples/robustness and share the same fault contract.
CASES=(
    "E11210_division_by_zero|spec/tests/run/E11210_division_by_zero.clo|error E11210: integer division by zero"
    "E11210_min_neg_one|spec/tests/run/E11210_min_neg_one.clo|error E11210: integer division by zero"
    "E11211_index_out_of_range|spec/tests/run/E11211_index_out_of_range.clo|error E11211: index out of range"
    "E11212_float_to_int_range|spec/tests/run/E11212_float_to_int_range.clo|error E11212: float-to-integer conversion out of range"
    "E11212_nan_float_to_int|spec/tests/run/E11212_nan_float_to_int.clo|error E11212: float-to-integer conversion out of range"
    "E11213_assertion_failed|spec/tests/run/E11213_assertion_failed.clo|error E11213: assertion failed"
    "fill-bounds|examples/robustness/fill-bounds.clo|error E11211: index out of range"
    "fill-divisor|examples/robustness/fill-divisor.clo|error E11210: integer division by zero"
)

if [ ! -x "${CLC}" ]; then
    echo "run-fault-amiga: compiler not found: ${CLC}" >&2
    exit 2
fi

status=0
for entry in "${CASES[@]}"; do
    name="${entry%%|*}"
    rest="${entry#*|}"
    source="${rest%%|*}"
    want="${rest#*|}"
    work="${OUTDIR}/${name}"
    rm -rf "${work}"
    mkdir -p "${work}"

    echo "run-fault-amiga: ${name}: emit and build"

    "${CLC}" --profile hosted-amiga -o "${work}/program.c" "${source}"
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
    elif [ "${rc}" -eq 124 ]; then
        echo "run-fault-amiga: ${name}: timed out after 120s" >&2
        status=1
    elif [ "${rc}" -ne "${TRAP_EXIT}" ]; then
        echo "run-fault-amiga: ${name}: exit ${rc}, expected the trap exit ${TRAP_EXIT}" >&2
        status=1
    else
        echo "run-fault-amiga: ${name}: '${first}' (exit ${rc})"
    fi
done

if [ "${status}" -ne 0 ]; then
    echo "run-fault-amiga: FAILED" >&2
else
    echo "run-fault-amiga: all eight faults match on hosted-amiga"
fi
exit "${status}"
