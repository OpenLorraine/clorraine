#!/usr/bin/env bash
# Run the verified FS-UAE path with a real toolchain and licensed ROM.
#
# This is the local guest check behind `make fs-uae-check`. It builds hello,
# the four portable corpus programs, and one A500-profile corpus program, runs
# each as an A1200/KS3.1 guest, compares the output, and proves a returning-1
# program fails. It also checks that no FS-UAE or Xvfb process survives a run.
#
# Usage: fs_uae_check.sh KICKSTART [TOOLCHAIN]
#   KICKSTART  licensed 512 KB Kickstart 3.1 A1200 ROM
#   TOOLCHAIN  extracted rc24 root; when omitted, the stored CLI selection is
#              used and the caller's real home/config is read.
#
# The check requires a clean process start: close any FS-UAE or Xvfb process
# before running it. Every invocation is followed by the no-survivor assertion,
# including a failing guest or the negative. On failure the temporary evidence
# directory is kept and its path is printed; a successful run removes it.
set -euo pipefail

rom="${1:-}"
toolchain="${2:-}"
if [ -z "$rom" ]; then
    echo 'usage: tools/setup/fs_uae_check.sh KICKSTART [TOOLCHAIN]' >&2
    exit 2
fi
if [ ! -f "$rom" ]; then
    echo "fs-uae-check: missing Kickstart ROM $rom" >&2
    exit 2
fi
if [ -n "$toolchain" ] && [ ! -x "$toolchain/bin/m68k-amigaos-gcc" ]; then
    echo "fs-uae-check: missing $toolchain/bin/m68k-amigaos-gcc" >&2
    exit 2
fi

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
driver="$repo/target/debug/clorraine"
if [ ! -x "$driver" ]; then
    echo "fs-uae-check: missing $driver; run make check or cargo build --workspace" >&2
    exit 2
fi
fs_uae="$(command -v fs-uae || true)"
if [ -z "$fs_uae" ]; then
    echo 'fs-uae-check: fs-uae was not found on PATH; install FS-UAE 3.2.0 or newer' >&2
    exit 2
fi
if ! command -v pgrep >/dev/null 2>&1; then
    echo 'fs-uae-check: pgrep is required to verify that no emulator process survives' >&2
    exit 2
fi

tmp_root="${CODEOPS_TMPDIR:-${TMPDIR:-/tmp}}"
mkdir -p "$tmp_root"
tmp="$(mktemp -d "$tmp_root/fs-uae-check.XXXXXX")"
keep=1
cleanup() {
    if [ "$keep" = 1 ]; then
        echo "fs-uae-check: evidence kept at $tmp" >&2
    else
        rm -rf "$tmp"
    fi
}
trap cleanup EXIT

mkdir -p "$tmp/run"
export TMPDIR="$tmp/run"

selection=()
if [ -n "$toolchain" ]; then
    mkdir -p "$tmp/home" "$tmp/config"
    export HOME="$tmp/home"
    export XDG_CONFIG_HOME="$tmp/config"
    echo 'fs-uae-check: setup (full validation)'
    "$driver" setup --toolchain "$toolchain"
    selection=(--toolchain "$toolchain")
fi

prefix=()
if [ -z "${DISPLAY:-}" ]; then
    if ! command -v xvfb-run >/dev/null 2>&1; then
        echo 'fs-uae-check: no DISPLAY and xvfb-run is not installed' >&2
        exit 2
    fi
    prefix=(xvfb-run -a)
fi

echo "fs-uae-check: fs-uae $("$fs_uae" --version | head -1)"
echo "fs-uae-check: rom $(stat -c %s "$rom") bytes $(sha256sum "$rom" | cut -d' ' -f1)"
if [ -n "$toolchain" ]; then
    echo "fs-uae-check: toolchain $("$toolchain/bin/m68k-amigaos-gcc" --version | head -1)"
fi

# PIDs of emulator or virtual-display processes currently alive.
processes() {
    { pgrep -x fs-uae || true; pgrep -x Xvfb || true; } | sort -u
}

# Fails when any FS-UAE or Xvfb process is alive.
#
# The check starts from a clean process set and asserts it again after every
# invocation, including failures and the negative. That makes a survivor from
# this run, or an earlier one, visible instead of hiding behind a baseline.
assert_no_processes() {
    local alive
    alive="$(processes)"
    if [ -n "$alive" ]; then
        echo "fs-uae-check: FS-UAE or Xvfb process still alive:" >&2
        printf '%s\n' "$alive" >&2
        return 1
    fi
}

# Runs one source and compares stdout with its expected output.
run_guest() {
    local name="$1" profile="$2" source="$3" expected="$4"
    local out err status
    out="$tmp/out-$name.txt"
    err="$tmp/err-$name.txt"
    set +e
    "${prefix[@]}" "$driver" run --profile "$profile" "${selection[@]}" \
        --emulator --fs-uae "$fs_uae" --kickstart "$rom" "$source" \
        >"$out" 2>"$err"
    status=$?
    set -e
    if [ "$status" -ne 0 ]; then
        echo "fs-uae-check: $name exited $status" >&2
        sed -n '1,20p' "$err" >&2
    fi
    assert_no_processes || return 1
    if [ "$status" -ne 0 ]; then
        return 1
    fi
    if ! diff -u <(tr -d '\r' <"$expected") <(tr -d '\r' <"$out"); then
        echo "fs-uae-check: $name output differs" >&2
        return 1
    fi
    echo "fs-uae-check: $name ok"
}

if [ -n "$(processes)" ]; then
    echo 'fs-uae-check: close existing FS-UAE/Xvfb processes before this check' >&2
    processes >&2
    keep=0
    exit 2
fi

printf 'Hello, Amiga, from Clorraine!\n' >"$tmp/hello.expected"
run_guest hello hosted-amiga "$repo/examples/hello/hello.clo" "$tmp/hello.expected"

for name in file-copier string-parser buffer-container volatile-mmio; do
    run_guest "$name" hosted-amiga \
        "$repo/spec/tests/corpus/$name/program.clo" \
        "$repo/spec/tests/corpus/$name/expected-output.txt"
done

run_guest file-copier-a500 hosted-amiga-13 \
    "$repo/spec/tests/corpus/file-copier/program.clo" \
    "$repo/spec/tests/corpus/file-copier/expected-output.txt"

# A returning-1 program must fail, leave no status marker, and retain its run.
negative="$tmp/return-one.clo"
cat >"$negative" <<'EOF'
module return_one;

func main(): i32 {
    return 1;
}
EOF
set +e
"${prefix[@]}" "$driver" run --profile hosted-amiga "${selection[@]}" \
    --emulator --fs-uae "$fs_uae" --kickstart "$rom" "$negative" \
    >"$tmp/negative.out" 2>"$tmp/negative.err"
status=$?
set -e
if [ "$status" -ne 1 ]; then
    echo "fs-uae-check: the rc-1 negative exited $status, expected 1" >&2
    sed -n '1,20p' "$tmp/negative.err" >&2
fi
assert_no_processes || exit 1
if [ "$status" -ne 1 ]; then
    exit 1
fi
if ! grep -q 'exited with code 1' "$tmp/negative.err"; then
    echo 'fs-uae-check: the rc-1 negative did not report code 1' >&2
    sed -n '1,20p' "$tmp/negative.err" >&2
    exit 1
fi
retained="$(sed -n "s/.*retained at '\\([^']*\\)'.*/\\1/p" "$tmp/negative.err" | tail -1)"
if [ -z "$retained" ] || [ ! -d "$retained" ]; then
    echo 'fs-uae-check: the rc-1 negative did not retain its run' >&2
    exit 1
fi
if [ "$(tr -d '\r\n' <"$retained/drive/clorraine-rc.txt")" != 1 ]; then
    echo 'fs-uae-check: the rc-1 negative did not record rc 1' >&2
    exit 1
fi
if [ -f "$retained/drive/clorraine-status.txt" ]; then
    echo 'fs-uae-check: the rc-1 negative wrote a success status' >&2
    exit 1
fi
rm -rf "$retained"
echo 'fs-uae-check: rc-1 negative ok'

keep=0
echo 'fs-uae-check: all guests passed'
