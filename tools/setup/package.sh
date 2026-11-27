#!/usr/bin/env bash
# Build and verify a local Clorraine CLI candidate.
#
# `make dist` stages the Linux and Windows release trees and calls this script;
# `make dist-verify` calls it with `--verify`. The script never downloads, tags,
# publishes, or uploads anything. It validates both stage layouts, writes
# release documents, packages the Linux archive and the Windows zip, assembles
# both installers, and covers the four assets with SHA256SUMS. The VS Code
# extension is published to the Marketplace, not shipped with the release.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
template="$here/install.sh"
powershell_template="$here/install.ps1"
hello_amiga="$here/../../examples/amiga/hello-intro.clo"

usage() {
    cat <<'EOF'
Usage: package.sh --stage DIR --windows-stage DIR --output DIR --version VERSION
                  [--hello-amiga FILE]
       package.sh --verify --output DIR --version VERSION

Build the Linux archive, the Windows zip, both installers, and SHA256SUMS. --hello-amiga replaces the Amiga hello both installers embed.
--verify checks an existing candidate instead of writing one.
EOF
}

die() {
    printf 'package: %s\n' "$*" >&2
    exit 1
}

stage=
windows_stage=
output=
version=
verify=0

while [ "$#" -gt 0 ]; do
    case "$1" in
        --stage) stage=$2; shift 2 ;;
        --windows-stage) windows_stage=$2; shift 2 ;;
        --hello-amiga) hello_amiga=$2; shift 2 ;;
        --output) output=$2; shift 2 ;;
        --version) version=$2; shift 2 ;;
        --verify) verify=1; shift ;;
        -h|--help) usage; exit 0 ;;
        *) die "unknown option '$1'" ;;
    esac
done

[ -n "$output" ] || die "--output is required"
[ -n "$version" ] || die "--version is required"

archive_name="clorraine-$version-x86_64-linux.tar.gz"
installer_name="install.sh"
zip_name="clorraine-$version-x86_64-windows.zip"
powershell_name="install.ps1"
windows_top="clorraine-$version-x86_64-windows"

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

# Asserts the exact Windows stage layout; each executable must be PE32+ for
# x86-64 and carry `clorraine <version>`, the text `--version` prints. The
# executables cannot run here, so the version is read from their bytes.
check_windows_stage() {
    local dir=$1 entry
    [ -d "$dir" ] || die "missing Windows stage directory: $dir"
    [ "$(basename "$dir")" = "$windows_top" ] ||
        die "Windows stage directory must be named $windows_top"
    for name in clc clorraine clorraine-lsp; do
        [ -f "$dir/bin/$name.exe" ] || die "missing bin/$name.exe in the Windows stage"
    done
    for document in README.md LICENSE LICENSE-EXCEPTION.md NOTICE THIRD_PARTY_NOTICES.md; do
        [ -f "$dir/$document" ] || die "missing $document in the Windows stage"
    done
    while IFS= read -r entry; do
        local relative=${entry#"$dir/"}
        [ ! -L "$entry" ] || die "link in the Windows stage: $relative"
        case "$relative" in
            README.md|LICENSE|LICENSE-EXCEPTION.md|NOTICE|THIRD_PARTY_NOTICES.md) ;;
            bin/clc.exe|bin/clorraine.exe|bin/clorraine-lsp.exe) ;;
            *) die "unexpected file in the Windows stage: $relative" ;;
        esac
    done < <(find "$dir" -type f -o -type l)
    for name in clc clorraine clorraine-lsp; do
        python3 "$here/pe_check.py" "bin/$name.exe" "$version" <"$dir/bin/$name.exe" || exit 1
    done
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
archive, and installs side by side with an atomic \`current\` switch. It can
download the pinned AmigaPorts rc24 toolchain and, when VS Code is detected,
install the extension. A licensed Kickstart ROM is user-supplied and is not
included.

Source, documentation, and issues:
https://github.com/OpenLorraine/clorraine
EOF
    write_release_documents_notice "$dir"
}

# Writes the NOTICE shared by both archives.
write_release_documents_notice() {
    local dir=$1
    cat >"$dir/NOTICE" <<EOF
Clorraine $version

Copyright the OpenLorraine contributors.
Licensed under the Apache License 2.0 with the Clorraine Output Exception;
see LICENSE and LICENSE-EXCEPTION.md. Third-party notices are in
THIRD_PARTY_NOTICES.md.

Project: https://github.com/OpenLorraine/clorraine
EOF
}

# Writes the Windows release README and NOTICE.
write_windows_documents() {
    local dir=$1
    cat >"$dir/README.md" <<EOF
# Clorraine $version (Windows x64)

Clorraine is a compiler and build driver for AmigaOS-compatible systems.
This archive contains \`clc.exe\`, \`clorraine.exe\`, and \`clorraine-lsp.exe\`.

Install with the release installer in Windows PowerShell:

\`\`\`powershell
powershell -ExecutionPolicy Bypass -File .\\install.ps1
\`\`\`

The installer checks Windows 11 x64, verifies the archive, and installs for
the current user side by side behind a \`current\` link. It can install the
pinned AmigaPorts rc24 toolchain and FS-UAE and, when VS Code is detected,
install the extension. A licensed Kickstart ROM is user-supplied and is not
included.

Source, documentation, and issues:
https://github.com/OpenLorraine/clorraine
EOF
    write_release_documents_notice "$dir"
}

# Replaces the installer template pins.
assemble_installer() {
    local archive=$1 target=$2 bytes digest
    bytes="$(wc -c <"$archive")"
    digest="$(sha256sum "$archive" | awk '{print $1}')"
    python3 - "$template" "$target" "$version" "$(basename "$archive")" \
        "https://github.com/OpenLorraine/clorraine/releases/download/v$version/$(basename "$archive")" \
        "$bytes" "$digest" "$hello_amiga" <<'PY'
import sys
(template, target, version, name, url, size, digest, hello_amiga) = sys.argv[1:]
text = open(template, encoding="utf-8").read()
# The Amiga hello is embedded in a quoted here-document after the pins, so
# nothing in it is substituted; only its terminator line must not occur.
intro = open(hello_amiga, encoding="utf-8").read().rstrip("\n")
if "CLORRAINE_HELLO_AMIGA" in intro.splitlines():
    sys.exit("package: the Amiga hello source contains the here-document terminator")
for token, value in {
    "@VERSION@": version,
    "@ARCHIVE_NAME@": name,
    "@ARCHIVE_URL@": url,
    "@ARCHIVE_BYTES@": size,
    "@ARCHIVE_SHA256@": digest,
}.items():
    text = text.replace(token, value)
text = text.replace("@HELLO_AMIGA_SOURCE@", intro)
open(target, "w", encoding="utf-8").write(text)
PY
    chmod +x "$target"
}

# Builds the Windows zip: files only, sorted, forward slashes, fixed times.
build_windows_zip() {
    local dir=$1 target=$2
    python3 - "$dir" "$target" <<'PY'
import os, sys, zipfile
root, target = sys.argv[1], sys.argv[2]
top = os.path.basename(root.rstrip("/"))
names = []
for folder, _, files in os.walk(root):
    for file in files:
        names.append(os.path.relpath(os.path.join(folder, file), root).replace(os.sep, "/"))
with zipfile.ZipFile(target, "w", zipfile.ZIP_DEFLATED) as archive:
    for name in sorted(f"{top}/{relative}" for relative in names):
        info = zipfile.ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
        info.compress_type = zipfile.ZIP_DEFLATED
        info.create_system = 3
        mode = 0o755 if name.endswith(".exe") else 0o644
        info.external_attr = (0o100000 | mode) << 16
        with open(os.path.join(root, name[len(top) + 1:]), "rb") as source:
            archive.writestr(info, source.read())
PY
}

# Replaces the PowerShell installer pins. The Amiga hello sits in a
# single-quoted here-string, so nothing in it expands; a line that starts with
# the closing '@ would end it early. The script must stay ASCII.
assemble_powershell_installer() {
    local archive=$1 target=$2
    python3 - "$powershell_template" "$target" "$version" "$(basename "$archive")" \
        "https://github.com/OpenLorraine/clorraine/releases/download/v$version/$(basename "$archive")" \
        "$(wc -c <"$archive")" "$(sha256sum "$archive" | awk '{print $1}')" "$hello_amiga" <<'PY'
import re, sys
(template, target, version, name, url, size, digest, hello_amiga) = sys.argv[1:]
text = open(template, encoding="utf-8").read()
intro = open(hello_amiga, encoding="utf-8").read().rstrip("\n")
if any(line.startswith("'@") for line in intro.splitlines()):
    sys.exit("package: the Amiga hello has a line starting with '@, which would end the here-string")
for token, value in {
    "@VERSION@": version,
    "@ARCHIVE_NAME@": name,
    "@ARCHIVE_URL@": url,
    "@ARCHIVE_BYTES@": size,
    "@ARCHIVE_SHA256@": digest,
}.items():
    text = text.replace(token, value)
text = text.replace("@HELLO_AMIGA_SOURCE@", intro)
left = re.search(r"@[A-Z0-9_]+@", text)
if left:
    sys.exit(f"package: install.ps1 still has the pin {left.group(0)}")
if any(byte > 0x7E or (byte < 0x20 and byte not in (0x09, 0x0A)) for byte in text.encode("utf-8")):
    sys.exit("package: install.ps1 must be printable ASCII")
open(target, "w", encoding="ascii", newline="\n").write(text)
PY
}

# Builds the four candidate assets.
build_candidate() {
    [ -n "$stage" ] || die "--stage is required"
    [ -n "$windows_stage" ] || die "--windows-stage is required"
    check_stage "$stage"
    check_windows_stage "$windows_stage"
    write_release_documents "$stage"
    write_windows_documents "$windows_stage"
    mkdir -p "$output" || die "cannot create $output"
    local archive="$output/$archive_name"
    rm -f "$archive"
    tar -czf "$archive" -C "$(dirname "$stage")" "$(basename "$stage")"
    assemble_installer "$archive" "$output/$installer_name"
    rm -f "$output/$zip_name"
    build_windows_zip "$windows_stage" "$output/$zip_name"
    assemble_powershell_installer "$output/$zip_name" "$output/$powershell_name"
    (
        cd "$output"
        sha256sum "$archive_name" "$zip_name" "$installer_name" "$powershell_name" >SHA256SUMS
    )
    printf 'package: wrote %s\n' "$output/$archive_name"
    printf 'package: wrote %s\n' "$output/$zip_name"
    printf 'package: wrote %s\n' "$output/$installer_name"
    printf 'package: wrote %s\n' "$output/$powershell_name"
    printf 'package: wrote %s\n' "$output/SHA256SUMS"
}

# Checks the asset set, checksums, archive layout, installer pins, and links.
verify_candidate() {
    local archive="$output/$archive_name"
    local installer="$output/$installer_name"
    local sums="$output/SHA256SUMS"
    local bytes digest lines top entry

    [ -f "$archive" ] || die "missing archive: $archive"
    [ -f "$installer" ] || die "missing installer: $installer"
    [ -f "$output/$zip_name" ] || die "missing Windows archive: $output/$zip_name"
    [ -f "$output/$powershell_name" ] || die "missing installer: $output/$powershell_name"
    [ -f "$sums" ] || die "missing checksums: $sums"
    [ "$(awk 'END { print NR }' "$sums")" = 4 ] || die "SHA256SUMS must cover exactly four assets"
    for name in "$archive_name" "$zip_name" "$installer_name" "$powershell_name"; do
        grep -Fq "  $name" "$sums" || die "SHA256SUMS does not cover $name"
    done
    # A VSIX, archive, or installer left from an older candidate must not ride
    # along with this one; the stage folders `make dist` keeps there are fine.
    local path name
    for path in "$output"/*; do
        [ -f "$path" ] || continue
        name=$(basename "$path")
        case "$name" in
            "$archive_name"|"$zip_name"|"$installer_name"|"$powershell_name"|SHA256SUMS) ;;
            *.vsix|*.tar.gz|*.zip|install*) die "unexpected file in the candidate: $name" ;;
        esac
    done
    local failed
    failed="$(cd "$output" && sha256sum -c --quiet SHA256SUMS 2>/dev/null | sed -n 's/: FAILED.*$//p' | head -1)" || true
    [ -z "$failed" ] || die "checksum verification failed: $failed"
    ( cd "$output" && sha256sum -c --status SHA256SUMS ) || die "checksum verification failed"

    bytes="$(wc -c <"$archive")"
    digest="$(sha256sum "$archive" | awk '{print $1}')"
    grep -Fq "VERSION='$version'" "$installer" || die "installer pin mismatch: VERSION"
    grep -Fq "ARCHIVE_BYTES='$bytes'" "$installer" || die "installer pin mismatch: ARCHIVE_BYTES"
    grep -Fq "ARCHIVE_SHA256='$digest'" "$installer" || die "installer pin mismatch: ARCHIVE_SHA256"
    if grep -Fq "@HELLO_AMIGA_SOURCE@" "$installer" ||
        ! grep -Fxq "module demo.HelloIntro;" "$installer"; then
        die "installer does not embed the Amiga hello"
    fi

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

    verify_windows_assets
}

# Checks the Windows zip layout and the PowerShell installer pins.
verify_windows_assets() {
    local zip="$output/$zip_name" ps1="$output/$powershell_name"
    # The release README and NOTICE are written by this script, so the zip's
    # copies must match a fresh rendering for this version.
    local documents
    documents="$(mktemp -d "${TMPDIR:-/tmp}/clorraine-documents.XXXXXXXX")"
    write_windows_documents "$documents"
    python3 - "$zip" "$windows_top" "$version" "$here/pe_check.py" "$documents" <<'PY' || { rm -rf "$documents"; exit 1; }
import importlib.util, os, sys, zipfile
path, top, version, checker, documents = sys.argv[1:]
spec = importlib.util.spec_from_file_location("pe_check", checker)
pe_check = importlib.util.module_from_spec(spec)
spec.loader.exec_module(pe_check)
zip_name = path.rsplit("/", 1)[-1]
expected = sorted(f"{top}/{name}" for name in (
    "LICENSE", "LICENSE-EXCEPTION.md", "NOTICE", "README.md", "THIRD_PARTY_NOTICES.md",
    "bin/clc.exe", "bin/clorraine-lsp.exe", "bin/clorraine.exe"))
archive = zipfile.ZipFile(path)
infos = archive.infolist()
if [info.filename for info in infos] != expected:
    sys.exit(f"package: {zip_name} does not hold exactly the release files")
for info in infos:
    executable = info.filename.endswith(".exe")
    if info.create_system != 3 or \
            info.external_attr != (0o100755 if executable else 0o100644) << 16 or \
            info.date_time != (1980, 1, 1, 0, 0, 0):
        sys.exit(f"package: {zip_name} entry {info.filename} is not a release file entry")
    leaf = info.filename[len(top) + 1:]
    if leaf in ("README.md", "NOTICE"):
        with open(os.path.join(documents, leaf), "rb") as rendered:
            if archive.read(info) != rendered.read():
                sys.exit(f"package: {zip_name} entry {info.filename} is not the release {leaf}")
    if executable:
        problem = pe_check.problem(archive.read(info), version)
        if problem:
            sys.exit(f"package: {zip_name} entry {info.filename} {problem}")
PY
    rm -rf "$documents"
    grep -Fxq "\$script:Version = '$version'" "$ps1" || die "install.ps1 pin mismatch: Version"
    grep -Fxq "\$script:ArchiveName = '$zip_name'" "$ps1" || die "install.ps1 pin mismatch: ArchiveName"
    grep -Fxq "\$script:ArchiveBytes = '$(wc -c <"$zip")'" "$ps1" || die "install.ps1 pin mismatch: ArchiveBytes"
    grep -Fxq "\$script:ArchiveSha256 = '$(sha256sum "$zip" | awk '{print $1}')'" "$ps1" ||
        die "install.ps1 pin mismatch: ArchiveSha256"
    if grep -Eq '@[A-Z0-9_]+@' "$ps1" || ! grep -Fxq "module demo.HelloIntro;" "$ps1"; then
        die "install.ps1 does not embed the Amiga hello"
    fi
    if LC_ALL=C grep -q '[^ -~[:space:]]' "$ps1"; then
        die "install.ps1 must be printable ASCII"
    fi
    # Every other byte, pin, and line end must be what the template gives.
    local rebuilt
    rebuilt="$(mktemp "${TMPDIR:-/tmp}/clorraine-install-ps1.XXXXXXXX")"
    if ! (assemble_powershell_installer "$zip" "$rebuilt") ||
        ! cmp -s "$rebuilt" "$ps1"; then
        rm -f "$rebuilt"
        die "install.ps1 differs from the template assembled with this candidate"
    fi
    rm -f "$rebuilt"
}

if [ "$verify" = 1 ]; then
    verify_candidate
    printf 'package: verified %s\n' "$output"
else
    build_candidate
fi
