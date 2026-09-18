#!/bin/sh
# Run each fault fixture case and assert the emitted runtime contract:
#   - the first output line is exactly "error CODE: message";
#   - the cleanup ladder did not run; and
#   - the program did not exit 0.
#
# usage: run-faults.sh <label> <binary> <outdir> [runner...]
#
# The optional runner wraps each invocation, for example `vamos` for an Amiga
# binary. Output is normalised to LF before it is compared.

set -u

label=$1
bin=$2
outdir=$3
shift 3
runner="$*"

here=$(dirname "$0")
expected="$here/expected-faults.txt"

fail=0
while read -r name code message; do
    if [ -z "$name" ]; then
        continue
    fi
    case "$name" in
        \#*) continue ;;
    esac

    out="$outdir/$label-$name.txt"
    if [ -n "$runner" ]; then
        # shellcheck disable=SC2086
        $runner "$bin" "$name" > "$out" 2>&1
    else
        "$bin" "$name" > "$out" 2>&1
    fi
    status=$?
    tr -d '\r' < "$out" > "$out.tmp" && mv "$out.tmp" "$out"

    first=$(sed -n '1p' "$out")
    want="error $code: $message"

    if [ "$first" != "$want" ]; then
        echo "FAIL $label/$name: first line '$first' != '$want'"
        fail=1
    fi
    if grep -q 'cleanup' "$out"; then
        echo "FAIL $label/$name: the cleanup ladder ran"
        fail=1
    fi
    if [ "$status" -eq 0 ]; then
        echo "FAIL $label/$name: the program exited 0"
        fail=1
    fi
    if [ "$fail" -eq 0 ]; then
        echo "ok   $label/$name: '$first' (exit $status)"
    fi
done < "$expected"

exit "$fail"
