#!/usr/bin/env bash
# Run the runtime-fault programs on both hosted Amiga profiles under vamos:
# fifteen on both profiles, plus one for the 2.0+ surface on hosted-amiga only.
#
# Each program must compile with zero diagnostics on hosted-amiga (libnix 2.0)
# and hosted-amiga-13 (libnix 1.3), build warning-free with the m68k GCC
# toolchain, print the exact fault line as its first output line, and exit
# with the trap code. The fault aborts before the cleanup ladder, so a clean
# exit is always a failure. The host C compiler cannot reproduce the m68k trap
# behavior, so this is the corpus's target leg for the runtime fault contract.
#
# The fault still skips every drop, but the runtime must hand back the OS
# resources the program held. vamos reports anything left behind at shutdown,
# as a library or device it cannot expunge or as orphaned memory, so any such
# line fails the case. The fault-cleanup probes fault with locks, ports,
# messages, and a library open to exercise that release.
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

GCC_FLAGS=(-std=c99 -m68000 -fwrapv -O0 -g -Wall -Wextra -Werror)

# Each entry is `<profile>|<libnix runtime>`.
PROFILES=(
    "hosted-amiga|nix20"
    "hosted-amiga-13|nix13"
)

# A vamos line naming a resource the program left behind.
LEAK_PATTERN="can't expunge|mem_alloc:WARNING:[[:space:]]+orphan|shutdown: can't expunge"

# The hosted-amiga shim leaves the process with this code after a fault, so a
# genuine trap is exactly this value. A timeout (124) or any other code means
# the program did not reach its fault and must fail the leg.
TRAP_EXIT=20

# Each entry is `<program>|<source path>|<expected first output line>`.
# The six frozen faults stay under spec/tests/run; the closeout and
# fault-cleanup probes live under examples/robustness and share the same fault
# contract.
CASES=(
    "E11210_division_by_zero|spec/tests/run/E11210_division_by_zero.clo|error E11210: integer division by zero"
    "E11210_min_neg_one|spec/tests/run/E11210_min_neg_one.clo|error E11210: integer division by zero"
    "E11211_index_out_of_range|spec/tests/run/E11211_index_out_of_range.clo|error E11211: index out of range"
    "E11212_float_to_int_range|spec/tests/run/E11212_float_to_int_range.clo|error E11212: float-to-integer conversion out of range"
    "E11212_nan_float_to_int|spec/tests/run/E11212_nan_float_to_int.clo|error E11212: float-to-integer conversion out of range"
    "E11213_assertion_failed|spec/tests/run/E11213_assertion_failed.clo|error E11213: assertion failed"
    "fill-bounds|examples/robustness/fill-bounds.clo|error E11211: index out of range"
    "fill-divisor|examples/robustness/fill-divisor.clo|error E11210: integer division by zero"
    "lock-fault|examples/robustness/fault-cleanup/lock-fault.clo|error E11211: index out of range"
    "port-fault|examples/robustness/fault-cleanup/port-fault.clo|error E11210: integer division by zero"
    "message-fault|examples/robustness/fault-cleanup/message-fault.clo|error E11211: index out of range"
    "reply-fault|examples/robustness/fault-cleanup/reply-fault.clo|error E11211: index out of range"
    "library-fault|examples/robustness/fault-cleanup/library-fault.clo|error E11211: index out of range"
    "panic-after-acquire|examples/robustness/fault-cleanup/panic-after-acquire.clo|panic: stop"
    "assert-after-acquire|examples/robustness/fault-cleanup/assert-after-acquire.clo|error E11213: assertion failed"
)

# Cases for the 2.0+ surface, which only hosted-amiga carries.
CASES_20=(
    "utility-fault|examples/robustness/fault-cleanup/utility-fault.clo|error E11211: index out of range"
)

if [ ! -x "${CLC}" ]; then
    echo "run-fault-amiga: compiler not found: ${CLC}" >&2
    exit 2
fi

status=0
for profile_entry in "${PROFILES[@]}"; do
    profile="${profile_entry%%|*}"
    crt="${profile_entry#*|}"
    cases=("${CASES[@]}")
    if [ "${profile}" = "hosted-amiga" ]; then
        cases+=("${CASES_20[@]}")
    fi
    for entry in "${cases[@]}"; do
        name="${entry%%|*}"
        rest="${entry#*|}"
        source="${rest%%|*}"
        want="${rest#*|}"
        label="${profile}: ${name}"
        work="${OUTDIR}/${profile}/${name}"
        rm -rf "${work}"
        mkdir -p "${work}"

        echo "run-fault-amiga: ${label}: emit and build"

        "${CLC}" --profile "${profile}" -o "${work}/program.c" "${source}"
        "${GCC}" "${GCC_FLAGS[@]}" "-mcrt=${crt}" -o "${work}/program" "${work}/program.c" -lamiga -lm

        abs_program="$(cd "${work}" && pwd)/program"
        abs_out="$(cd "${work}" && pwd)/out.txt"
        # The fault exits nonzero, which would abort the script under `set -e`,
        # so the exit code is captured instead of propagated.
        set +e
        ( cd "${work}" && timeout 120 "${VAMOS}" "${abs_program}" ) 2>&1 | tr -d '\r' > "${abs_out}"
        rc="${PIPESTATUS[0]}"
        set -e

        first="$(sed -n '1p' "${abs_out}")"
        leaks="$(grep -E "${LEAK_PATTERN}" "${abs_out}" || true)"
        if [ "${first}" != "${want}" ]; then
            echo "run-fault-amiga: ${label}: first line '${first}' != '${want}'" >&2
            status=1
        elif [ "${rc}" -eq 124 ]; then
            echo "run-fault-amiga: ${label}: timed out after 120s" >&2
            status=1
        elif [ "${rc}" -ne "${TRAP_EXIT}" ]; then
            echo "run-fault-amiga: ${label}: exit ${rc}, expected the trap exit ${TRAP_EXIT}" >&2
            status=1
        elif [ -n "${leaks}" ]; then
            echo "run-fault-amiga: ${label}: resources left behind:" >&2
            echo "${leaks}" >&2
            status=1
        else
            echo "run-fault-amiga: ${label}: '${first}' (exit ${rc})"
        fi
    done
done

if [ "${status}" -ne 0 ]; then
    echo "run-fault-amiga: FAILED" >&2
else
    echo "run-fault-amiga: all faults match (16 on hosted-amiga, 15 on hosted-amiga-13)"
fi
exit "${status}"
