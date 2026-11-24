#!/usr/bin/env bash
# Validate a user-supplied rc24 toolchain and build the Phase 1 programs.
#
# The setup step runs the full validator (canonical root, version, glibc, and
# both libnix probes) against a private config. The build steps then use the
# explicit toolchain without touching the user's config. This is a local product
# check, not part of the fast `make check` gate.
set -euo pipefail

root="${1:-}"
if [ -z "$root" ]; then
    echo 'usage: tools/setup/toolchain_check.sh PATH' >&2
    exit 2
fi
if [ ! -x "$root/bin/m68k-amigaos-gcc" ]; then
    echo "toolchain-check: missing $root/bin/m68k-amigaos-gcc" >&2
    exit 2
fi

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
driver="$repo/target/debug/clorraine"
if [ ! -x "$driver" ]; then
    echo "toolchain-check: missing $driver; run make check or cargo build --workspace" >&2
    exit 2
fi

tmp_root="${CODEOPS_TMPDIR:-${TMPDIR:-/tmp}}"
mkdir -p "$tmp_root"
tmp="$(mktemp -d "$tmp_root/clorraine-toolchain-check.XXXXXX")"
cleanup() {
    rm -rf "$tmp"
}
trap cleanup EXIT

mkdir -p "$tmp/home" "$tmp/config"
export HOME="$tmp/home"
export XDG_CONFIG_HOME="$tmp/config"

echo 'toolchain-check: setup (full validation)'
"$driver" setup --toolchain "$root"

echo 'toolchain-check: hello'
"$driver" build --profile hosted-amiga --toolchain "$root" \
    "$repo/examples/hello/hello.clo"
stat -c 'toolchain-check: %n %s bytes' "$repo/build/clorraine/hello/hosted-amiga/hello"

for name in file-copier string-parser buffer-container volatile-mmio; do
    echo "toolchain-check: $name"
    "$driver" build --profile hosted-amiga --toolchain "$root" \
        "$repo/spec/tests/corpus/$name/program.clo"
    stat -c 'toolchain-check: %n %s bytes' \
        "$repo/build/clorraine/program/hosted-amiga/program"
done

echo 'toolchain-check: freestanding'
"$driver" build --profile freestanding --toolchain "$root" \
    "$repo/spec/tests/corpus/freestanding/program.clo"
stat -c 'toolchain-check: %n %s bytes' \
    "$repo/build/clorraine/program/freestanding/program"
