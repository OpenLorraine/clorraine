#!/usr/bin/env bash
# Windows candidate producer contract.
#
# These cases run the packaging script on fixture Linux and Windows stage
# trees. They cover the five-asset candidate, the Windows zip layout, the
# Windows stage checks (exact file set, PE32+ x86-64 executables that carry
# the version), the assembled install.ps1, and verification, without Docker,
# Node, Windows, or the network.

set -u

. "$(dirname "$0")/install_fixtures.sh"

# Builds both stage trees, a fake VSIX, and an output directory.
fixture() {
    root="$(new_root)"
    stage="$(make_stage "$root")"
    wstage="$(make_windows_stage "$root")"
    vsix="$root/clorraine-1.2.0.vsix"
    printf 'fake vsix\n' >"$vsix"
    output="$root/dist"
    mkdir -p "$output"
}

# Runs the packaging script and prints its status.
run_package() {
    "$@" >"$root/stdout" 2>"$root/stderr"
    printf '%s\n' "$?"
}

# Builds the candidate from the fixture and prints the status.
build() {
    run_package "$package_script" --stage "$stage" --windows-stage "$wstage" --output "$output" \
        --version 1.2.0 --vsix "$vsix" "$@"
}

zip_name=clorraine-1.2.0-x86_64-windows.zip

# The build writes five assets and one SHA256SUMS line for each.
build_writes_five_assets_under_one_checksum_file() {
    fixture
    check_status 0 "$(build)" "package build"
    local name
    for name in clorraine-1.2.0-x86_64-linux.tar.gz "$zip_name" clorraine-1.2.0.vsix install.sh install.ps1; do
        check_exists "$output/$name" "$name"
        check_contains "$output/SHA256SUMS" "  $name" "checksum line for $name"
    done
    if [ "$(wc -l <"$output/SHA256SUMS")" -eq 5 ]; then pass; else fail "SHA256SUMS has $(wc -l <"$output/SHA256SUMS") lines"; fi
    if ( cd "$output" && sha256sum -c --status SHA256SUMS ); then pass; else fail "SHA256SUMS does not match the files"; fi
}

# The zip holds exactly the release files under one top folder, sorted, with
# forward slashes.
the_windows_zip_has_the_exact_layout() {
    fixture
    check_status 0 "$(build)" "package build"
    local listing expected
    listing="$(python3 -c 'import sys, zipfile; print("\n".join(i.filename for i in zipfile.ZipFile(sys.argv[1]).infolist()))' "$output/$zip_name")"
    expected="$(printf 'clorraine-1.2.0-x86_64-windows/%s\n' LICENSE LICENSE-EXCEPTION.md NOTICE README.md THIRD_PARTY_NOTICES.md bin/clc.exe bin/clorraine-lsp.exe bin/clorraine.exe | LC_ALL=C sort)"
    if [ "$listing" = "$expected" ]; then pass; else fail "zip entries differ:
$listing"; fi
    case "$listing" in
        *'\'*) fail "a zip entry uses a backslash" ;;
        *) pass ;;
    esac
}

# A missing or an extra file in the Windows stage stops the build by name.
a_windows_stage_with_a_missing_or_extra_file_fails() {
    fixture
    rm "$wstage/bin/clc.exe"
    check_status 1 "$(build)" "missing clc.exe"
    check_contains "$root/stderr" "bin/clc.exe" "missing-file message"
    fixture
    printf 'extra\n' >"$wstage/bin/extra.dll"
    check_status 1 "$(build)" "extra file"
    check_contains "$root/stderr" "bin/extra.dll" "extra-file message"
}

# An executable that is not PE32+ x86-64, or lacks the version, stops the build.
a_windows_executable_that_is_not_the_release_fails() {
    fixture
    printf '\177ELF fixture\n' >"$wstage/bin/clorraine.exe"
    check_status 1 "$(build)" "ELF named clorraine.exe"
    check_contains "$root/stderr" "clorraine.exe" "ELF message"
    fixture
    make_pe "$wstage/bin/clorraine.exe"
    check_status 1 "$(build)" "PE32+ without the version"
    check_contains "$root/stderr" "clorraine.exe" "version message"
}

# The assembled installer is ASCII, has no pin left, and carries the version;
# a hello source line that would end its here-string stops the build.
the_assembled_powershell_installer_is_complete() {
    fixture
    check_status 0 "$(build)" "package build"
    if LC_ALL=C grep -q '[^ -~[:space:]]' "$output/install.ps1"; then fail "install.ps1 has a non-ASCII byte"; else pass; fi
    if grep -Eq '@[A-Z0-9_]+@' "$output/install.ps1"; then fail "install.ps1 has a pin left"; else pass; fi
    check_contains "$output/install.ps1" "Version = '1.2.0'" "version pin"
    check_contains "$output/install.ps1" "module demo.HelloIntro;" "embedded Amiga hello"
    fixture
    printf "module demo.Bad;\n'@\n" >"$root/bad-hello.clo"
    check_status 1 "$(build --hello-amiga "$root/bad-hello.clo")" "hello that ends the here-string"
    check_contains "$root/stderr" "here-string" "here-string message"
}

# Verification passes on the candidate and names a changed zip.
verify_checks_all_five_assets() {
    fixture
    check_status 0 "$(build)" "package build"
    check_status 0 "$(run_package "$package_script" --verify --output "$output" --version 1.2.0)" "verify"
    python3 - "$output/$zip_name" <<'PY'
import sys
path = sys.argv[1]
data = bytearray(open(path, "rb").read())
data[len(data) // 2] ^= 0xFF
open(path, "wb").write(bytes(data))
PY
    check_status 1 "$(run_package "$package_script" --verify --output "$output" --version 1.2.0)" "verify after a changed zip byte"
    check_contains "$root/stderr" "$zip_name" "changed-zip message"
}

build_writes_five_assets_under_one_checksum_file
the_windows_zip_has_the_exact_layout
a_windows_stage_with_a_missing_or_extra_file_fails
a_windows_executable_that_is_not_the_release_fails
the_assembled_powershell_installer_is_complete
verify_checks_all_five_assets
finish
