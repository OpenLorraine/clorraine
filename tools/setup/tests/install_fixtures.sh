#!/usr/bin/env bash
# Shared fixtures for the Linux installer and candidate-archive tests.
#
# The scripts source this file, build a fixture stage tree and archive, assemble
# a copy of the installer with fixture pins, and run the installer against fake
# host tools. Nothing here needs Docker, a toolchain, a ROM, FS-UAE, or the
# network.

set -u

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$tests_dir/../../.." && pwd)"
installer_template="$repo/tools/setup/install.sh"
package_script="$repo/tools/setup/package.sh"

failures=0
checks=0
roots_base="${CODEOPS_TMPDIR:-${TMPDIR:-/tmp}}"
mkdir -p "$roots_base"
test_roots_file="$(mktemp "$roots_base/clorraine-test-roots.XXXXXX")"

# Removes every root this suite created, including roots that failed early.
# `new_root` runs in a command substitution, so a shared file is used instead
# of a shell array that would only change in the subshell.
cleanup_test_roots() {
    local root
    if [ -f "$test_roots_file" ]; then
        while IFS= read -r root; do
            [ -n "$root" ] && rm -rf "$root"
        done <"$test_roots_file"
        rm -f "$test_roots_file"
    fi
    return 0
}
trap cleanup_test_roots EXIT

# Records a failed check and keeps the suite running.
fail() {
    printf 'FAIL: %s\n' "$*" >&2
    failures=$((failures + 1))
}

# Records one passing check.
pass() {
    checks=$((checks + 1))
}

# Asserts an exact status and labels the check.
check_status() {
    local expected="$1" actual="$2" label="$3"
    if [ "$actual" -eq "$expected" ]; then
        pass
    else
        fail "$label: expected status $expected, got $actual"
    fi
}

# Asserts that a file exists and is a regular file or a symlink.
check_exists() {
    local path="$1" label="$2"
    if [ -e "$path" ] || [ -L "$path" ]; then
        pass
    else
        fail "$label: missing $path"
    fi
}

# Asserts that a path does not exist.
check_absent() {
    local path="$1" label="$2"
    if [ ! -e "$path" ] && [ ! -L "$path" ]; then
        pass
    else
        fail "$label: unexpected $path"
    fi
}

# Asserts that a file contains a literal string.
check_contains() {
    local path="$1" text="$2" label="$3"
    if grep -Fq -- "$text" "$path" 2>/dev/null; then
        pass
    else
        fail "$label: '$text' not in $(basename "$path")"
    fi
}

# Asserts that a file does not contain a literal string.
check_not_contains() {
    local path="$1" text="$2" label="$3"
    if grep -Fq -- "$text" "$path" 2>/dev/null; then
        fail "$label: '$text' unexpectedly in $(basename "$path")"
    else
        pass
    fi
}

# Asserts a symlink target.
check_symlink() {
    local path="$1" target="$2" label="$3"
    if [ -L "$path" ] && [ "$(readlink "$path")" = "$target" ]; then
        pass
    else
        fail "$label: $path is not -> $target"
    fi
}

# Prints a short summary and exits nonzero on failure.
finish() {
    if [ "$failures" -ne 0 ]; then
        printf '%s: %s failure(s), %s check(s)\n' "$(basename "$0")" "$failures" "$checks" >&2
        exit 1
    fi
    printf '%s: %s checks passed\n' "$(basename "$0")" "$checks"
}

# Creates a private test root under the CodeOps temporary directory.
new_root() {
    local base="${CODEOPS_TMPDIR:-${TMPDIR:-/tmp}}"
    mkdir -p "$base"
    local root
    root="$(mktemp -d "$base/clorraine-installer.XXXXXX")"
    printf '%s\n' "$root" >>"$test_roots_file"
    printf '%s\n' "$root"
}

# Creates a fixture release stage tree and prints its path.
make_stage() {
    local root="$1" version="${2:-1.2.0}"
    local stage="$root/stage/clorraine-$version-x86_64-linux"
    mkdir -p "$stage/bin"
    cat >"$stage/bin/clorraine" <<EOF
#!/bin/sh
if [ -n "\${FAKE_CLI_LOG:-}" ]; then
    printf '%s\\n' "\$*" >>"\$FAKE_CLI_LOG"
fi
case "\${1:-}" in
    --version) printf '%s\\n' "clorraine $version" ;;
    setup)
        if [ "\${FAKE_SETUP_FAIL:-0}" = 1 ]; then
            printf 'clorraine: --toolchain: not a toolchain\\n' >&2
            exit 2
        fi
        case " \$* " in
            *' --kickstart '*|*' --fs-uae '*)
                if [ "\${FAKE_SETUP_REJECT_EMULATOR:-0}" = 1 ]; then
                    printf 'clorraine: --kickstart: a 256 KB Kickstart 1.3 ROM cannot boot the A1200 recipe\\n' >&2
                    exit 2
                fi
                ;;
        esac
        printf 'configured toolchain\\n' ;;
    run) printf 'Hello, Amiga, from Clorraine!\\n' ;;
    new) [ "\${FAKE_NEW_FAIL:-0}" = 1 ] && exit 1; printf 'Created your Amiga project\\n' ;;
    *) exit 0 ;;
esac
EOF
    printf '#!/bin/sh\nexit 0\n' >"$stage/bin/clc"
    printf '#!/bin/sh\nexit 0\n' >"$stage/bin/clorraine-lsp"
    for document in README.md LICENSE LICENSE-EXCEPTION.md NOTICE THIRD_PARTY_NOTICES.md; do
        printf 'fixture %s\n' "$document" >"$stage/$document"
    done
    chmod +x "$stage/bin/clc" "$stage/bin/clorraine" "$stage/bin/clorraine-lsp"
    printf '%s\n' "$stage"
}

# Packages a stage tree into the versioned fixture archive and prints its path.
make_archive() {
    local root="$1" stage="$2"
    local base
    base="$(basename "$stage")"
    tar -czf "$root/$base.tar.gz" -C "$(dirname "$stage")" "$base"
    printf '%s\n' "$root/$base.tar.gz"
}

# Writes a one-entry SHA256SUMS for an archive.
write_checksums() {
    local archive="$1" output="$2"
    (
        cd "$(dirname "$archive")"
        sha256sum "$(basename "$archive")"
    ) >"$output"
}

# Replaces the installer template pins and writes an executable fixture.
assemble_installer() {
    local template="$1" output="$2" version="$3" archive="$4" vsix="${5:-}"
    local base bytes digest vsix_base vsix_bytes vsix_digest
    base="$(basename "$archive")"
    bytes="$(wc -c <"$archive")"
    digest="$(sha256sum "$archive" | awk '{print $1}')"
    if [ -z "$vsix" ]; then
        vsix="$(dirname "$output")/clorraine-$version.vsix"
        printf 'fixture vsix for %s\n' "$version" >"$vsix"
    fi
    vsix_base="$(basename "$vsix")"
    vsix_bytes="$(wc -c <"$vsix")"
    vsix_digest="$(sha256sum "$vsix" | awk '{print $1}')"
    python3 - "$template" "$output" "$version" "$base" "https://example.invalid/$base" "$bytes" "$digest" \
        "$vsix_base" "https://example.invalid/$vsix_base" "$vsix_bytes" "$vsix_digest" \
        "$repo/examples/amiga/hello-intro.clo" <<'PY'
import sys
(template, output, version, name, url, size, digest,
 vsix_name, vsix_url, vsix_size, vsix_digest, hello_amiga) = sys.argv[1:]
text = open(template, encoding="utf-8").read()
for token, value in {
    "@VERSION@": version,
    "@ARCHIVE_NAME@": name,
    "@ARCHIVE_URL@": url,
    "@ARCHIVE_BYTES@": size,
    "@ARCHIVE_SHA256@": digest,
    "@VSIX_NAME@": vsix_name,
    "@VSIX_URL@": vsix_url,
    "@VSIX_BYTES@": vsix_size,
    "@VSIX_SHA256@": vsix_digest,
}.items():
    text = text.replace(token, value)
intro = open(hello_amiga, encoding="utf-8").read().rstrip("\n")
text = text.replace("@HELLO_AMIGA_SOURCE@", intro)
open(output, "w", encoding="utf-8").write(text)
PY
    chmod +x "$output"
}

# Creates fake host and download tools in `$root/fake`.
fake_tools() {
    local root="$1"
    local fake="$root/fake"
    mkdir -p "$fake"
    cat >"$fake/uname" <<'EOF'
#!/bin/sh
case "${1:-}" in
    -s) printf '%s\n' "${FAKE_UNAME_S:-Linux}" ;;
    -m) printf '%s\n' "${FAKE_UNAME_M:-x86_64}" ;;
    *) printf '%s\n' "${FAKE_UNAME_S:-Linux}" ;;
esac
EOF
    cat >"$fake/getconf" <<'EOF'
#!/bin/sh
printf '%s\n' "${FAKE_GLIBC:-glibc 2.39}"
EOF
    cat >"$fake/curl" <<'EOF'
#!/bin/sh
if [ -n "${FAKE_CURL_LOG:-}" ]; then
    printf '%s\n' "$*" >>"$FAKE_CURL_LOG"
fi
out=
url=
while [ "$#" -gt 0 ]; do
    case "$1" in
        --output) out="${2:-}"; shift 2 ;;
        --output=*) out="${1#--output=}"; shift ;;
        --*) shift ;;
        *) url="$1"; shift ;;
    esac
done
[ -n "$out" ] || exit 2
if [ "${FAKE_CURL_STATUS:-0}" -ne 0 ]; then
    exit "${FAKE_CURL_STATUS}"
fi
source=${FAKE_CURL_SOURCE:-}
case "$url" in
    *.vsix) [ -n "${FAKE_CURL_SOURCE_VSIX:-}" ] && source=$FAKE_CURL_SOURCE_VSIX ;;
esac
[ -n "$source" ] || exit 0
cp "$source" "$out"
if [ "${FAKE_CURL_TRUNCATE:-0}" = 1 ]; then
    head -c 10 "$source" >"$out"
fi
EOF
    cat >"$fake/sha256sum" <<'EOF'
#!/bin/sh
if [ "${FAKE_SHA256_MODE:-real}" = zeros ]; then
    printf '%064d  %s\n' 0 "${1:-archive}"
    exit 0
fi
exec /usr/bin/sha256sum "$@"
EOF
    cat >"$fake/code" <<'EOF'
#!/bin/sh
[ "${FAKE_EDITOR_READY:-0}" = 1 ] || exit 1
case "${1:-}" in
    --version) printf '1.90.0\n' ;;
    --list-extensions) [ "${FAKE_EXTENSION_INSTALLED:-0}" = 1 ] && printf 'openlorraine.clorraine\n' ;;
    --install-extension)
        [ -n "${FAKE_EDITOR_LOG:-}" ] && printf '%s\n' "$*" >>"$FAKE_EDITOR_LOG"
        [ "${FAKE_EDITOR_FAIL:-0}" = 1 ] && exit 1
        ;;
    *) [ -n "${FAKE_EDITOR_LOG:-}" ] && printf 'open %s\n' "$*" >>"$FAKE_EDITOR_LOG" ;;
esac
exit 0
EOF
    cat >"$fake/fs-uae" <<'EOF'
#!/bin/sh
[ "${FAKE_FS_UAE_READY:-0}" = 1 ] || exit 1
case "${1:-}" in
    --version) printf 'fs-uae 3.2.35\n' ;;
esac
exit 0
EOF
    chmod +x "$fake"/*
}

# Creates a 512 KB Kickstart-sized fixture file and prints its path.
make_fake_rom() {
    local path="$1"
    dd if=/dev/zero of="$path" bs=1024 count=512 status=none
    printf '%s\n' "$path"
}

# Runs one installer invocation with the fixture HOME/config and fake PATH.
run_installer() {
    local root="$1"
    shift
    HOME="$root/home" XDG_CONFIG_HOME="$root/config" PATH="$root/fake:$PATH" \
        "$@" >"$root/stdout" 2>"$root/stderr"
    printf '%s\n' "$?"
}

# Creates the fixture HOME, config, and a fake tool directory.
prepare_installer_fixture() {
    local root="$1"
    mkdir -p "$root/home" "$root/config"
    fake_tools "$root"
}
