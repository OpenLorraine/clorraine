#!/usr/bin/env bash
# Run every suite that references RDC_UBSAN under UndefinedBehaviorSanitizer.
#
# The suites discover the sanitizer through the `RDC_UBSAN` environment
# variable. This runner finds every test file that references it, runs each
# suite in its own bounded process with one test thread, and reports the
# discovered count. It also proves the leg is active: a known-overflow C probe
# must report `runtime error:` before any suite runs. The SDK image's GCC 12
# runtime is unstable for PIE executables, so a temporary `cc` wrapper adds
# `-no-pie` to sanitizer links.
#
# Run inside the SDK container from the repository root:
#   docker/scripts/ubsan-suite.sh
#
# Environment overrides:
#   UBSAN_SUITE_TIMEOUT  per-suite timeout in seconds (default: 900)
set -euo pipefail
cd "$(dirname "$0")/../.."

# A fault or sanitizer abort must not leave a core file in the tree.
ulimit -c 0 || true

suite_timeout="${UBSAN_SUITE_TIMEOUT:-900}"

real_cc="$(command -v cc)"
wrapper="$(mktemp -d)"
probe_dir="$(mktemp -d)"
trap 'rm -rf "${wrapper}" "${probe_dir}"' EXIT
cat > "${wrapper}/cc" <<WRAPPER
#!/bin/sh
case " \$* " in
    *"-fsanitize=undefined"*|*"-fsanitize=address,undefined"*)
        exec "${real_cc}" "\$@" -no-pie
        ;;
esac
exec "${real_cc}" "\$@"
WRAPPER
chmod +x "${wrapper}/cc"

# Activation self-test. The probe exits 1 under -fno-sanitize-recover=all, so
# capture its status instead of letting `set -e` turn the expected failure into
# a runner failure.
cat > "${probe_dir}/overflow.c" <<'PROBE'
#include <limits.h>
int main(void) {
    volatile int value = INT_MAX;
    value = value + 1;
    return value == 0;
}
PROBE
if ! "${real_cc}" -std=c99 -O0 -g -fsanitize=undefined \
    -fno-sanitize-recover=all -no-pie \
    -o "${probe_dir}/overflow" "${probe_dir}/overflow.c"; then
    echo "ubsan: the activation probe did not build" >&2
    exit 2
fi
set +e
probe_output="$(ASAN_OPTIONS=detect_leaks=1 UBSAN_OPTIONS=print_stacktrace=0 \
    "${probe_dir}/overflow" 2>&1)"
set -e
if ! printf '%s' "${probe_output}" | grep -q 'runtime error:'; then
    echo "ubsan: the activation probe did not report undefined behavior" >&2
    exit 2
fi

export PATH="${wrapper}:${PATH}"

files="$(grep -l 'RDC_UBSAN' compiler/tests/*.rs | sort || true)"
if [ -z "${files}" ]; then
    echo "ubsan: no RDC_UBSAN suites discovered" >&2
    exit 2
fi

count=0
status=0
while IFS= read -r file; do
    suite="$(basename "${file}" .rs)"
    count=$((count + 1))
    echo "ubsan: ${suite}"
    if ! timeout --kill-after=30 "${suite_timeout}" env RDC_UBSAN=1 \
        cargo test --test "${suite}" -- --test-threads=1; then
        echo "ubsan: ${suite}: FAILED" >&2
        status=1
    fi
done <<< "${files}"

if [ "${count}" -lt 2 ]; then
    echo "ubsan: expected at least two suites, discovered ${count}" >&2
    exit 2
fi

echo "ubsan: ${count} suites"
exit "${status}"
