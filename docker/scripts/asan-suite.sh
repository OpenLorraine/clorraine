#!/usr/bin/env bash
# Run every memory-safety suite under AddressSanitizer.
#
# The suites discover the sanitizer through the `RDC_ASAN` environment
# variable. This runner finds every test file that references it, runs each
# suite in its own bounded process with one test thread, and reports the
# discovered count.
#
# The SDK image's GCC 12 ASan runtime intermittently fails at startup with a
# `DEADLYSIGNAL` loop when it links a PIE executable under the container's
# ASLR. A non-PIE link is stable. The suites build their ASan children by
# calling `cc`, so a temporary PATH wrapper adds `-no-pie` only to commands
# that request AddressSanitizer. Non-ASan builds pass through unchanged.
#
# Run inside the SDK container from the repository root:
#   docker/scripts/asan-suite.sh
#
# Environment overrides:
#   ASAN_SUITE_TIMEOUT  per-suite timeout in seconds (default: 900)
set -euo pipefail
cd "$(dirname "$0")/../.."

# A runaway probe must not fill the host temp filesystem or leave core files:
# cap any single file at 1 GiB and suppress core dumps.
ulimit -c 0 || true
ulimit -f 1048576 || true

suite_timeout="${ASAN_SUITE_TIMEOUT:-900}"

real_cc="$(command -v cc)"
wrapper="$(mktemp -d)"
trap 'rm -rf "${wrapper}"' EXIT
cat > "${wrapper}/cc" <<WRAPPER
#!/bin/sh
case " \$* " in
    *"-fsanitize=address"*)
        exec "${real_cc}" "\$@" -no-pie
        ;;
esac
exec "${real_cc}" "\$@"
WRAPPER
chmod +x "${wrapper}/cc"
export PATH="${wrapper}:${PATH}"

files="$(grep -l 'RDC_ASAN' compiler/tests/*.rs | sort || true)"
if [ -z "${files}" ]; then
    echo "asan: no RDC_ASAN suites discovered" >&2
    exit 2
fi

count=0
status=0
while IFS= read -r file; do
    suite="$(basename "${file}" .rs)"
    count=$((count + 1))
    echo "asan: ${suite}"
    if ! timeout --kill-after=30 "${suite_timeout}" env RDC_ASAN=1 \
        cargo test --test "${suite}" -- --test-threads=1; then
        echo "asan: ${suite}: FAILED" >&2
        status=1
    fi
done <<< "${files}"

echo "asan: ${count} suites"
exit "${status}"
