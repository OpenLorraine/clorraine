#!/usr/bin/env bash
# Run the local acceptance path from the exact candidate archive.
#
# `make acceptance CANDIDATE=... TOOLCHAIN_ARCHIVE=... KICKSTART=...` installs
# the candidate into an isolated HOME, verifies and extracts the pinned rc24
# archive, runs setup and the verified FS-UAE matrix, and writes `evidence.json`
# beside the raw output. It never tags, publishes, or uploads; the only download
# is the VS Code extension, which the candidate installer takes from the
# Marketplace into the isolated HOME.
#
# PUBLISH_RUN and PUBLISH_SHA256 name the publish-vscode run that published
# this version and the VSIX SHA-256 from its summary. EDITOR_CONFIRMED=1
# records that a person checked the language server and Build/Run in VS Code
# with this release; without it the evidence is written and the run fails.
#
# This executes the self-built candidate installer with the caller's privileges.
# Do not point it at an untrusted candidate; use a dedicated user or container.
set -euo pipefail

candidate=${1:-}
toolchain_archive=${2:-}
kickstart=${3:-}
if [ -z "$candidate" ] || [ -z "$toolchain_archive" ] || [ -z "$kickstart" ]; then
    echo 'usage: tools/setup/acceptance.sh CANDIDATE TOOLCHAIN_ARCHIVE KICKSTART' >&2
    exit 2
fi

candidate=$(cd "$candidate" && pwd)
toolchain_archive=$(cd "$(dirname "$toolchain_archive")" && pwd)/$(basename "$toolchain_archive")
kickstart=$(cd "$(dirname "$kickstart")" && pwd)/$(basename "$kickstart")
repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
version=$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$repo/compiler/Cargo.toml" | head -1)
archive_name="clorraine-$version-x86_64-linux.tar.gz"
archive="$candidate/$archive_name"
installer="$candidate/install.sh"
sums="$candidate/SHA256SUMS"
for required in "$archive" "$installer" "$sums" "$toolchain_archive" "$kickstart"; do
    [ -f "$required" ] || { echo "acceptance: missing $required" >&2; exit 2; }
done
publish_run=${PUBLISH_RUN:-}
publish_sha=${PUBLISH_SHA256:-}
case "$publish_run" in
    ''|*[!0-9]*) echo 'acceptance: PUBLISH_RUN must be the publish-vscode run ID' >&2; exit 2 ;;
esac
case "$publish_sha" in
    *[!0-9a-f]*|'') echo 'acceptance: PUBLISH_SHA256 must be the published VSIX SHA-256' >&2; exit 2 ;;
esac
[ "${#publish_sha}" -eq 64 ] || { echo 'acceptance: PUBLISH_SHA256 must be the published VSIX SHA-256' >&2; exit 2; }
editor=$(command -v code || command -v code-insiders || true)
[ -n "$editor" ] || { echo 'acceptance: VS Code (code or code-insiders) is required on PATH' >&2; exit 2; }
if ! command -v pgrep >/dev/null 2>&1; then
    echo 'acceptance: pgrep is required' >&2
    exit 2
fi
if pgrep -x fs-uae >/dev/null || pgrep -x Xvfb >/dev/null; then
    echo 'acceptance: close existing FS-UAE/Xvfb processes before this run' >&2
    exit 2
fi

stamp=$(date -u +%Y%m%dT%H%M%SZ)
out="${OUTDIR:-$repo/build/acceptance/$stamp}"
work="$out/work"
umask 077
mkdir -p "$work/home" "$work/config" "$work/tmp" "$work/prefix" "$work/run" "$work/toolchain"
chmod 700 "$out" "$work"
export HOME="$work/home" XDG_CONFIG_HOME="$work/config" TMPDIR="$work/tmp"
prefix="$work/prefix"

fs_uae=$(command -v fs-uae)
[ -n "$fs_uae" ] || { echo 'acceptance: fs-uae is not on PATH' >&2; exit 2; }
fs_uae_version=$("$fs_uae" --version | head -1)
glibc=$(getconf GNU_LIBC_VERSION | awk '{print $2}')
case "$glibc" in
    2.3[8-9]|2.[4-9][0-9]|[3-9].*) ;;
    *) echo "acceptance: glibc $glibc is below 2.38" >&2; exit 2 ;;
esac
prefix_commands=()
if [ -z "${DISPLAY:-}" ]; then
    command -v xvfb-run >/dev/null 2>&1 || { echo 'acceptance: no DISPLAY and no xvfb-run' >&2; exit 2; }
    prefix_commands=(xvfb-run -a)
fi

sha() { sha256sum "$1" | awk '{print $1}'; }
normalized_sha() { tr -d '\r' <"$1" | sha256sum | awk '{print $1}'; }
no_survivor() {
    if pgrep -x fs-uae >/dev/null || pgrep -x Xvfb >/dev/null; then
        echo "acceptance: $1 left an FS-UAE or Xvfb process" >&2
        exit 1
    fi
    echo "acceptance: $1 no fs-uae/Xvfb survivor" | tee -a "$out/fs-uae.log"
}

archive_bytes=$(stat -c %s "$archive")
archive_sha=$(sha "$archive")
installer_sha=$(sha "$installer")
toolchain_bytes=$(stat -c %s "$toolchain_archive")
toolchain_sha=$(sha "$toolchain_archive")
[ "$toolchain_bytes" = 69868548 ] || { echo 'acceptance: toolchain archive size is not the rc24 pin' >&2; exit 2; }
[ "$toolchain_sha" = b93b070ac47d2f01d126f57c03a8405f71fb470c8397b0bda51041217d25c100 ] ||
    { echo 'acceptance: toolchain archive SHA-256 is not the rc24 pin' >&2; exit 2; }
kickstart_sha=$(sha "$kickstart")

# Keep the exact candidate bytes and command record in the evidence directory;
# the installer and checker then use the same private copies.
cp -- "$archive" "$out/$archive_name"
cp -- "$installer" "$out/install.sh"
cp -- "$sums" "$out/SHA256SUMS"
chmod 600 "$out/$archive_name" "$out/SHA256SUMS"
chmod 700 "$out/install.sh"
archive="$out/$archive_name"
installer="$out/install.sh"
sums="$out/SHA256SUMS"
[ "$(sha "$installer")" = "$installer_sha" ] || { echo 'acceptance: private installer copy mismatch' >&2; exit 1; }

echo "acceptance: install candidate"
"$installer" --prefix "$prefix" --archive "$archive" --checksums "$sums" --vscode >"$out/install.log" 2>&1
# The installer installed the extension from the Marketplace into the isolated
# HOME; the editor lists it there with its version.
extension_line=$("$editor" --list-extensions --show-versions </dev/null 2>/dev/null |
    grep -i '^openlorraine\.clorraine@' | head -1 || true)
extension_version=${extension_line#*@}
[ -n "$extension_line" ] && [ "$extension_version" = "$version" ] || {
    echo "acceptance: VS Code lists '${extension_line:-no Clorraine extension}', not openlorraine.clorraine@$version; see $out/install.log" >&2
    exit 1
}
if [ "${EDITOR_CONFIRMED:-}" = 1 ]; then editor_confirmed=true; else editor_confirmed=false; fi
installed_version=$("$prefix/bin/clorraine" --version)
[ "$installed_version" = "clorraine $version" ] || {
    echo "acceptance: installed version '$installed_version' is not clorraine $version" >&2
    exit 1
}

echo "acceptance: extract the rc24 toolchain"
toolchain_copy="$work/toolchain.tar.xz"
cp -- "$toolchain_archive" "$toolchain_copy"
chmod 600 "$toolchain_copy"
[ "$(stat -c %s "$toolchain_copy")" = 69868548 ] ||
    { echo 'acceptance: private toolchain copy size mismatch' >&2; exit 1; }
[ "$(sha "$toolchain_copy")" = "$toolchain_sha" ] ||
    { echo 'acceptance: private toolchain copy hash mismatch' >&2; exit 1; }
tar -xJf "$toolchain_copy" -C "$work/toolchain"
compiler=$(find "$work/toolchain" -maxdepth 3 -path '*/bin/m68k-amigaos-gcc' -print -quit)
[ -n "$compiler" ] || { echo 'acceptance: extracted toolchain has no compiler' >&2; exit 1; }
toolchain_root=${compiler%/bin/m68k-amigaos-gcc}
"$prefix/bin/clorraine" setup --toolchain "$toolchain_root" >"$out/setup.log" 2>&1

echo "acceptance: build hello"
(cd "$work/run" && "$prefix/bin/clorraine" build --profile hosted-amiga \
    "$repo/examples/hello/hello.clo") >"$out/build.log" 2>&1

run_guest() {
    local name=$1 profile=$2 source=$3 expected=$4
    (cd "$work/run" && "${prefix_commands[@]}" "$prefix/bin/clorraine" run \
        --profile "$profile" --emulator --fs-uae "$fs_uae" --kickstart "$kickstart" "$source") \
        >"$out/$name.txt" 2>"$out/$name.err"
    diff -u <(tr -d '\r' <"$expected") <(tr -d '\r' <"$out/$name.txt")
    echo "acceptance: $name output ok"
    cat "$out/$name.err" >>"$out/fs-uae.log"
    no_survivor "$name"
}

printf 'Hello, Amiga, from Clorraine!\n' >"$out/hello.expected"
run_guest hello hosted-amiga "$repo/examples/hello/hello.clo" "$out/hello.expected"

corpus_names=(file-copier string-parser buffer-container volatile-mmio)
for name in "${corpus_names[@]}"; do
    run_guest "$name" hosted-amiga "$repo/spec/tests/corpus/$name/program.clo" \
        "$repo/spec/tests/corpus/$name/expected-output.txt"
done
run_guest file-copier-a500 hosted-amiga-13 "$repo/spec/tests/corpus/file-copier/program.clo" \
    "$repo/spec/tests/corpus/file-copier/expected-output.txt"

echo "acceptance: rc-1 negative"
cat >"$work/return-one.clo" <<'EOF'
module return_one;

func main(): i32 {
    return 1;
}
EOF
set +e
(cd "$work/run" && "${prefix_commands[@]}" "$prefix/bin/clorraine" run \
    --profile hosted-amiga --emulator --fs-uae "$fs_uae" --kickstart "$kickstart" \
    "$work/return-one.clo") >"$out/negative.out" 2>"$out/negative.err"
negative_rc=$?
set -e
cat "$out/negative.err" >>"$out/fs-uae.log"
[ "$negative_rc" -ne 0 ] || { echo 'acceptance: the rc-1 negative passed' >&2; exit 1; }
grep -q 'exited with code 1' "$out/negative.err" || {
    echo 'acceptance: the negative did not report code 1' >&2
    exit 1
}
retained=$(sed -n "s/.*retained at '\\([^']*\\)'.*/\\1/p" "$out/negative.err" | tail -1)
[ -n "$retained" ] && [ -d "$retained" ] || { echo 'acceptance: the negative did not retain its run' >&2; exit 1; }
case "$retained" in
    "$work"/*|"$out"/*) ;;
    *) echo "acceptance: retained run is outside the private work tree: $retained" >&2; exit 1 ;;
esac
[ "$(tr -d '\r\n' <"$retained/drive/clorraine-rc.txt")" = 1 ] ||
    { echo 'acceptance: the negative did not record rc 1' >&2; exit 1; }
[ ! -f "$retained/drive/clorraine-status.txt" ] ||
    { echo 'acceptance: the negative wrote a success status' >&2; exit 1; }
rm -rf -- "$retained"
no_survivor negative

hello_output=$(cat "$out/hello.txt")
hello_sha=$(printf '%s' "$hello_output" | sha256sum | awk '{print $1}')
file_copier_sha=$(normalized_sha "$out/file-copier.txt")
string_parser_sha=$(normalized_sha "$out/string-parser.txt")
buffer_container_sha=$(normalized_sha "$out/buffer-container.txt")
volatile_mmio_sha=$(normalized_sha "$out/volatile-mmio.txt")
file_copier_a500_sha=$(normalized_sha "$out/file-copier-a500.txt")

EVIDENCE_VERSION="$version" \
EVIDENCE_PLATFORM=linux EVIDENCE_ARCH=x86_64 EVIDENCE_GLIBC="$glibc" \
EVIDENCE_ARCHIVE_NAME="$archive_name" EVIDENCE_ARCHIVE_SHA="$archive_sha" EVIDENCE_ARCHIVE_BYTES="$archive_bytes" \
EVIDENCE_INSTALLER_SHA="$installer_sha" \
EVIDENCE_EXTENSION_VERSION="$extension_version" EVIDENCE_PUBLISH_RUN="$publish_run" \
EVIDENCE_PUBLISH_SHA="$publish_sha" EVIDENCE_EDITOR_CONFIRMED="$editor_confirmed" \
EVIDENCE_TOOLCHAIN_SHA="$toolchain_sha" EVIDENCE_TOOLCHAIN_BYTES="$toolchain_bytes" \
EVIDENCE_KICKSTART_NAME="$(basename "$kickstart")" EVIDENCE_KICKSTART_SHA="$kickstart_sha" \
EVIDENCE_FS_UAE="$fs_uae_version" \
EVIDENCE_HELLO="$hello_output" EVIDENCE_HELLO_SHA="$hello_sha" \
EVIDENCE_NEGATIVE_RC="$negative_rc" \
EVIDENCE_FILE_COPIER_SHA="$file_copier_sha" EVIDENCE_STRING_PARSER_SHA="$string_parser_sha" \
EVIDENCE_BUFFER_CONTAINER_SHA="$buffer_container_sha" EVIDENCE_VOLATILE_MMIO_SHA="$volatile_mmio_sha" \
EVIDENCE_FILE_COPIER_A500_SHA="$file_copier_a500_sha" \
EVIDENCE_TIMESTAMP="$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
python3 - "$out/evidence.json" <<'PY'
import json
import os
import sys

env = os.environ
record = {
    "version": env["EVIDENCE_VERSION"],
    "platform": env["EVIDENCE_PLATFORM"],
    "architecture": env["EVIDENCE_ARCH"],
    "glibc": env["EVIDENCE_GLIBC"],
    "archive": {
        "name": env["EVIDENCE_ARCHIVE_NAME"],
        "sha256": env["EVIDENCE_ARCHIVE_SHA"],
        "bytes": int(env["EVIDENCE_ARCHIVE_BYTES"]),
    },
    "installer": {"sha256": env["EVIDENCE_INSTALLER_SHA"]},
    "extension": {
        "id": "openlorraine.clorraine",
        "version": env["EVIDENCE_EXTENSION_VERSION"],
        "publish_run": env["EVIDENCE_PUBLISH_RUN"],
        "vsix_sha256": env["EVIDENCE_PUBLISH_SHA"],
    },
    "manual": {"editor_build_run": env["EVIDENCE_EDITOR_CONFIRMED"] == "true"},
    "toolchain": {
        "tag": "v16.2-rc24",
        "sha256": env["EVIDENCE_TOOLCHAIN_SHA"],
        "bytes": int(env["EVIDENCE_TOOLCHAIN_BYTES"]),
    },
    "kickstart": {
        "name": env["EVIDENCE_KICKSTART_NAME"],
        "sha256": env["EVIDENCE_KICKSTART_SHA"],
    },
    "fs_uae": env["EVIDENCE_FS_UAE"],
    "guest": {
        "status": "ok",
        "rc": 0,
        "output": env["EVIDENCE_HELLO"],
        "output_sha256": env["EVIDENCE_HELLO_SHA"],
    },
    "negative": {"status": "failed", "rc": int(env["EVIDENCE_NEGATIVE_RC"])},
    "corpus": [
        {"name": "file-copier", "profile": "hosted-amiga", "status": "ok",
         "output_sha256": env["EVIDENCE_FILE_COPIER_SHA"]},
        {"name": "string-parser", "profile": "hosted-amiga", "status": "ok",
         "output_sha256": env["EVIDENCE_STRING_PARSER_SHA"]},
        {"name": "buffer-container", "profile": "hosted-amiga", "status": "ok",
         "output_sha256": env["EVIDENCE_BUFFER_CONTAINER_SHA"]},
        {"name": "volatile-mmio", "profile": "hosted-amiga", "status": "ok",
         "output_sha256": env["EVIDENCE_VOLATILE_MMIO_SHA"]},
        {"name": "file-copier", "profile": "hosted-amiga-13", "status": "ok",
         "output_sha256": env["EVIDENCE_FILE_COPIER_A500_SHA"]},
    ],
    "timestamp": env["EVIDENCE_TIMESTAMP"],
}
with open(sys.argv[1], "w", encoding="utf-8") as target:
    json.dump(record, target, indent=2, sort_keys=False)
    target.write("\n")
PY

rm -rf "$work"
{
    printf 'candidate: %s\n' "$candidate"
    printf 'archive: %s\n' "$archive"
    printf 'installer: %s\n' "$installer"
    printf 'extension: openlorraine.clorraine@%s (publish run %s, VSIX %s)\n' "$extension_version" "$publish_run" "$publish_sha"
    printf 'toolchain archive: %s\n' "$toolchain_archive"
    printf 'kickstart: %s\n' "$kickstart"
    printf 'fs-uae: %s\n' "$fs_uae"
    printf 'profile matrix: hello, file-copier, string-parser, buffer-container, volatile-mmio, file-copier-a500, rc-1 negative\n'
} >"$out/commands.txt"
chmod 600 "$out"/* 2>/dev/null || true
echo "acceptance: evidence $out/evidence.json"
echo "acceptance: candidate $out"
if [ "$editor_confirmed" != true ]; then
    echo 'acceptance: confirm the language server and Build/Run in VS Code, then rerun with EDITOR_CONFIRMED=1' >&2
    exit 1
fi
