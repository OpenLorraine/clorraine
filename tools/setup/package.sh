#!/usr/bin/env bash
# Build and verify a local Clorraine CLI candidate archive.
#
# `make dist` stages the release tree and calls this script; `make dist-verify`
# calls it with `--verify`. The script never downloads, tags, publishes, or
# uploads anything. It validates the stage layout, writes release documents,
# packages the archive, copies the VSIX, assembles the installer, and covers
# all three assets with SHA256SUMS.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
template="$here/install.sh"

usage() {
    cat <<'EOF'
Usage: package.sh --stage DIR --output DIR --version VERSION --vsix FILE
       package.sh --verify --output DIR --version VERSION

Build a candidate archive, VSIX copy, installer, and SHA256SUMS. --verify
checks an existing candidate instead of writing one.
EOF
}

die() {
    printf 'package: %s\n' "$*" >&2
    exit 1
}

stage=
output=
version=
vsix=
verify=0

while [ "$#" -gt 0 ]; do
    case "$1" in
        --stage) stage=$2; shift 2 ;;
        --output) output=$2; shift 2 ;;
        --version) version=$2; shift 2 ;;
        --vsix) vsix=$2; shift 2 ;;
        --verify) verify=1; shift ;;
        -h|--help) usage; exit 0 ;;
        *) die "unknown option '$1'" ;;
    esac
done

[ -n "$output" ] || die "--output is required"
[ -n "$version" ] || die "--version is required"

archive_name="clorraine-$version-x86_64-linux.tar.gz"
vsix_name="clorraine-$version.vsix"
installer_name="install.sh"

# Asserts the exact allowed stage layout and version.
check_stage() {
    local dir=$1 reported entry
    [ -d "$dir" ] || die "missing stage directory: $dir"
    [ "$(basename "$dir")" = "clorraine-$version-x86_64-linux" ] ||
        die "stage directory must be named clorraine-$version-x86_64-linux"
    for name in clc clorraine clorraine-lsp; do
        [ -f "$dir/bin/$name" ] || die "missing bin/$name"
    done
    for document in README.md LICENSE LICENSE-EXCEPTION.md NOTICE THIRD_PARTY_NOTICES.md; do
        [ -f "$dir/$document" ] || die "missing $document"
    done
    while IFS= read -r entry; do
        local relative=${entry#"$dir/"}
        case "$relative" in
            README.md|LICENSE|LICENSE-EXCEPTION.md|NOTICE|THIRD_PARTY_NOTICES.md) ;;
            bin/clc|bin/clorraine|bin/clorraine-lsp) ;;
            *) die "unexpected file: $relative" ;;
        esac
    done < <(find "$dir" -type f -o -type l)
    reported="$("$dir/bin/clorraine" --version 2>/dev/null || true)"
    [ "$reported" = "clorraine $version" ] ||
        die "version mismatch: stage reports '$reported', expected 'clorraine $version'"
}

# Writes release-specific documents that use absolute repository URLs.
write_release_documents() {
    local dir=$1
    cat >"$dir/README.md" <<EOF
# Clorraine $version (Linux x86_64)

Clorraine is a compiler and build driver for AmigaOS-compatible systems.
This archive contains the \`clc\`, \`clorraine\`, and \`clorraine-lsp\` binaries.

Install with the release installer:

\`\`\`sh
./install.sh
\`\`\`

The installer checks Linux x86_64 and glibc 2.38 or newer, verifies the
archive, and installs side by side with an atomic \`current\` switch. The
Amiga toolchain and Kickstart ROM are user-supplied and are not included.

Source, documentation, and issues:
https://github.com/OpenLorraine/clorraine
EOF
    cat >"$dir/NOTICE" <<EOF
Clorraine $version

Copyright the OpenLorraine contributors.
Licensed under the Apache License 2.0 with the Clorraine Output Exception;
see LICENSE and LICENSE-EXCEPTION.md. Third-party notices are in
THIRD_PARTY_NOTICES.md.

Project: https://github.com/OpenLorraine/clorraine
EOF
}

# Replaces the installer template pins.
assemble_installer() {
    local archive=$1 target=$2 bytes digest
    bytes="$(wc -c <"$archive")"
    digest="$(sha256sum "$archive" | awk '{print $1}')"
    python3 - "$template" "$target" "$version" "$(basename "$archive")" \
        "https://github.com/OpenLorraine/clorraine/releases/download/v$version/$(basename "$archive")" \
        "$bytes" "$digest" <<'PY'
import sys
template, target, version, name, url, size, digest = sys.argv[1:]
text = open(template, encoding="utf-8").read()
for token, value in {
    "@VERSION@": version,
    "@ARCHIVE_NAME@": name,
    "@ARCHIVE_URL@": url,
    "@ARCHIVE_BYTES@": size,
    "@ARCHIVE_SHA256@": digest,
}.items():
    text = text.replace(token, value)
open(target, "w", encoding="utf-8").write(text)
PY
    chmod +x "$target"
}

# Builds the four candidate assets.
build_candidate() {
    [ -n "$stage" ] || die "--stage is required"
    [ -n "$vsix" ] || die "--vsix is required"
    [ -f "$vsix" ] || die "missing VSIX: $vsix"
    check_stage "$stage"
    write_release_documents "$stage"
    mkdir -p "$output" || die "cannot create $output"
    local archive="$output/$archive_name"
    rm -f "$archive"
    tar -czf "$archive" -C "$(dirname "$stage")" "$(basename "$stage")"
    cp "$vsix" "$output/$vsix_name"
    assemble_installer "$archive" "$output/$installer_name"
    (
        cd "$output"
        sha256sum "$archive_name" "$vsix_name" "$installer_name" >SHA256SUMS
    )
    printf 'package: wrote %s\n' "$output/$archive_name"
    printf 'package: wrote %s\n' "$output/$vsix_name"
    printf 'package: wrote %s\n' "$output/$installer_name"
    printf 'package: wrote %s\n' "$output/SHA256SUMS"
}

# Checks the asset set, checksums, archive layout, installer pins, and links.
verify_candidate() {
    local archive="$output/$archive_name"
    local target="$output/$vsix_name"
    local installer="$output/$installer_name"
    local sums="$output/SHA256SUMS"
    local bytes digest lines top entry

    [ -f "$archive" ] || die "missing archive: $archive"
    [ -f "$target" ] || die "missing VSIX: $target"
    [ -f "$installer" ] || die "missing installer: $installer"
    [ -f "$sums" ] || die "missing checksums: $sums"
    [ "$(awk 'END { print NR }' "$sums")" = 3 ] || die "SHA256SUMS must cover exactly three assets"
    for name in "$archive_name" "$vsix_name" "$installer_name"; do
        grep -Fq "  $name" "$sums" || die "SHA256SUMS does not cover $name"
    done
    ( cd "$output" && sha256sum -c --status SHA256SUMS ) || die "checksum verification failed"

    bytes="$(wc -c <"$archive")"
    digest="$(sha256sum "$archive" | awk '{print $1}')"
    grep -Fq "VERSION='$version'" "$installer" || die "installer pin mismatch: VERSION"
    grep -Fq "ARCHIVE_BYTES='$bytes'" "$installer" || die "installer pin mismatch: ARCHIVE_BYTES"
    grep -Fq "ARCHIVE_SHA256='$digest'" "$installer" || die "installer pin mismatch: ARCHIVE_SHA256"

    top="$(tar -tzf "$archive" | awk -F/ 'NR == 1 { print $1 }')"
    [ "$top" = "clorraine-$version-x86_64-linux" ] || die "archive has the wrong top-level directory"
    local listing allowed
    listing="$(tar -tzf "$archive")"
    allowed="bin/clc bin/clorraine bin/clorraine-lsp README.md LICENSE LICENSE-EXCEPTION.md NOTICE THIRD_PARTY_NOTICES.md"
    while IFS= read -r entry; do
        [ -n "$entry" ] || continue
        case "$entry" in
            "$top/"|"$top/bin/") continue ;;
        esac
        case "$entry" in
            "$top"/*) ;;
            *) die "archive contains an unexpected path: $entry" ;;
        esac
        local relative=${entry#"$top/"}
        case " $allowed " in
            *" $relative "*) ;;
            *) die "archive contains an unexpected file: $relative" ;;
        esac
    done <<<"$listing"

    local extracted
    extracted="$(mktemp -d "${TMPDIR:-/tmp}/clorraine-verify.XXXXXXXX")"
    tar -xzf "$archive" -C "$extracted"
    if grep -Eq '\]\((\.\./|docs/|spec/|docker/|tools/)' "$extracted/$top/README.md"; then
        rm -rf "$extracted"
        die "release README has an archive-relative repository link"
    fi
    rm -rf "$extracted"
}

if [ "$verify" = 1 ]; then
    verify_candidate
    printf 'package: verified %s\n' "$output"
else
    build_candidate
fi
