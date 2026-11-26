#!/usr/bin/env bash
# Windows candidate producer internals.
#
# Edge cases behind the Windows packaging contract: the zip is reproducible
# and holds only regular files, the assembled installer keeps LF line ends and
# refuses non-ASCII text or a leftover pin, and verification catches a changed
# installer pin and an extra zip entry even when SHA256SUMS is rewritten.

set -u

. "$(dirname "$0")/install_fixtures.sh"

zip_name=clorraine-1.2.0-x86_64-windows.zip

fixture() {
    root="$(new_root)"
    stage="$(make_stage "$root")"
    wstage="$(make_windows_stage "$root")"
    vsix="$root/clorraine-1.2.0.vsix"
    printf 'fake vsix\n' >"$vsix"
    output="$root/dist"
    mkdir -p "$output"
}

run_package() {
    "$@" >"$root/stdout" 2>"$root/stderr"
    printf '%s\n' "$?"
}

build() {
    run_package "${script:-$package_script}" --stage "$stage" --windows-stage "$wstage" \
        --output "$output" --version 1.2.0 --vsix "$vsix" "$@"
}

rewrite_sums() {
    ( cd "$output" && sha256sum clorraine-1.2.0-x86_64-linux.tar.gz "$zip_name" clorraine-1.2.0.vsix \
        install.sh install.ps1 >SHA256SUMS )
}

# Two builds of the same stage give the same zip bytes.
the_zip_is_reproducible() {
    fixture
    check_status 0 "$(build)" "first build"
    local first
    first="$(sha256sum "$output/$zip_name" | awk '{print $1}')"
    touch -d '2001-02-03 04:05' "$wstage/bin/clc.exe"
    check_status 0 "$(build)" "second build"
    if [ "$(sha256sum "$output/$zip_name" | awk '{print $1}')" = "$first" ]; then pass; else fail "the zip changed between builds"; fi
}

# Every entry is a regular file with Unix mode bits; there are no folder
# entries and executables are marked executable.
the_zip_holds_only_regular_files() {
    fixture
    check_status 0 "$(build)" "package build"
    if python3 - "$output/$zip_name" <<'PY'
import stat, sys, zipfile
for info in zipfile.ZipFile(sys.argv[1]).infolist():
    mode = info.external_attr >> 16
    if info.filename.endswith("/") or not stat.S_ISREG(mode):
        sys.exit(f"{info.filename}: not a regular file entry")
    if info.filename.endswith(".exe") != bool(mode & 0o111):
        sys.exit(f"{info.filename}: wrong executable bit")
PY
    then pass; else fail "zip entry types"; fi
}

# The assembled installer uses LF line ends only.
the_powershell_installer_keeps_lf_line_ends() {
    fixture
    check_status 0 "$(build)" "package build"
    if grep -q $'\r' "$output/install.ps1"; then fail "install.ps1 has a carriage return"; else pass; fi
}

# A non-ASCII Amiga hello cannot be embedded in the PowerShell installer.
a_non_ascii_hello_is_refused() {
    fixture
    printf 'module demo.Caf\xc3\xa9;\n' >"$root/hello.clo"
    check_status 1 "$(build --hello-amiga "$root/hello.clo")" "non-ASCII hello"
    check_contains "$root/stderr" "ASCII" "non-ASCII message"
}

# A template pin the producer does not know is reported by name.
a_leftover_pin_is_refused() {
    fixture
    local copy="$root/producer"
    mkdir -p "$copy"
    cp "$package_script" "$repo/tools/setup/install.sh" "$copy/"
    sed 's/^\$script:Version = .*/&\n$script:Extra = '"'"'@EXTRA_PIN@'"'"'/' "$repo/tools/setup/install.ps1" >"$copy/install.ps1"
    script="$copy/package.sh"
    check_status 1 "$(build --hello-amiga "$repo/examples/amiga/hello-intro.clo")" "leftover pin"
    unset script
    check_contains "$root/stderr" "@EXTRA_PIN@" "leftover-pin message"
}

# Verification compares the installer pins with the zip, not only SHA256SUMS.
a_stale_powershell_pin_fails_verification() {
    fixture
    check_status 0 "$(build)" "package build"
    sed -i "s/^\\\$script:ArchiveSha256 = '[0-9a-f]*'/\$script:ArchiveSha256 = '$(printf '0%.0s' $(seq 64))'/" "$output/install.ps1"
    rewrite_sums
    check_status 1 "$(run_package "$package_script" --verify --output "$output" --version 1.2.0)" "stale pin"
    check_contains "$root/stderr" "ArchiveSha256" "stale-pin message"
}

# An extra zip entry fails verification even with matching checksums and pins.
an_extra_zip_entry_fails_verification() {
    fixture
    check_status 0 "$(build)" "package build"
    python3 - "$output/$zip_name" <<'PY'
import sys, zipfile
with zipfile.ZipFile(sys.argv[1], "a") as archive:
    archive.writestr("clorraine-1.2.0-x86_64-windows/bin/extra.dll", b"x")
PY
    local bytes digest
    bytes="$(wc -c <"$output/$zip_name")"
    digest="$(sha256sum "$output/$zip_name" | awk '{print $1}')"
    sed -i -e "s/^\\\$script:ArchiveBytes = .*/\$script:ArchiveBytes = '$bytes'/" \
        -e "s/^\\\$script:ArchiveSha256 = .*/\$script:ArchiveSha256 = '$digest'/" "$output/install.ps1"
    rewrite_sums
    check_status 1 "$(run_package "$package_script" --verify --output "$output" --version 1.2.0)" "extra entry"
    check_contains "$root/stderr" "exactly the release files" "extra-entry message"
}

# A Windows stage holding a symbolic link is refused.
a_link_in_the_windows_stage_is_refused() {
    fixture
    ln -s README.md "$wstage/LINK.md"
    check_status 1 "$(build)" "link in the stage"
    check_contains "$root/stderr" "LINK.md" "link message"
}

the_zip_is_reproducible
the_zip_holds_only_regular_files
the_powershell_installer_keeps_lf_line_ends
a_non_ascii_hello_is_refused
a_leftover_pin_is_refused
a_stale_powershell_pin_fails_verification
an_extra_zip_entry_fails_verification
a_link_in_the_windows_stage_is_refused
finish
