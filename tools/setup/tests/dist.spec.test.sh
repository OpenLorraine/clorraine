#!/usr/bin/env bash
# Candidate-archive producer contract.
#
# These cases run the packaging script on fixture stage trees. They cover the
# exact layout, version pins, forbidden files, installer pins, checksum
# coverage, and release-document links without Docker, Node, or the network.

set -u

. "$(dirname "$0")/install_fixtures.sh"

# Builds a stage tree, a fake VSIX, and an output directory.
fixture() {
    root="$(new_root)"
    stage="$(make_stage "$root")"
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

# The build produces the four candidate assets with the exact layout.
build_creates_the_expected_assets() {
    fixture
    local status
    status="$(run_package "$package_script" --stage "$stage" --output "$output" --version 1.2.0 --vsix "$vsix")"
    check_status 0 "$status" "package build"
    local archive="$output/clorraine-1.2.0-x86_64-linux.tar.gz"
    check_exists "$archive" "archive"
    check_exists "$output/clorraine-1.2.0.vsix" "vsix"
    check_exists "$output/install.sh" "installer"
    check_exists "$output/SHA256SUMS" "checksums"

    local listing
    listing="$(tar -tzf "$archive")"
    local top="clorraine-1.2.0-x86_64-linux/"
    local expected="bin/clc bin/clorraine bin/clorraine-lsp README.md LICENSE LICENSE-EXCEPTION.md NOTICE THIRD_PARTY_NOTICES.md"
    local member name
    for member in $expected; do
        if printf '%s\n' "$listing" | grep -Fxq "$top$member"; then pass; else fail "archive missing $member"; fi
    done
    if printf '%s\n' "$listing" | grep -Fq "toolchain" || printf '%s\n' "$listing" | grep -Fq ".rom"; then
        fail "archive contains a forbidden payload"
    else
        pass
    fi
    for name in clorraine-1.2.0-x86_64-linux.tar.gz clorraine-1.2.0.vsix install.sh; do
        check_contains "$output/SHA256SUMS" "$name" "checksum coverage for $name"
    done
    check_contains "$output/install.sh" "VERSION='1.2.0'" "installer version pin"
    check_contains "$output/install.sh" "ARCHIVE_BYTES='$(wc -c <"$archive")'" "installer size pin"
    check_contains "$output/install.sh" "ARCHIVE_SHA256='$(sha256sum "$archive" | awk '{print $1}')'" "installer hash pin"

    status="$(run_package "$package_script" --verify --output "$output" --version 1.2.0)"
    check_status 0 "$status" "verify"
}

# A stage built for another version is rejected before packaging.
a_version_mismatch_fails() {
    root="$(new_root)"
    stage="$(make_stage "$root" 1.1.0)"
    mv "$stage" "$root/stage/clorraine-1.2.0-x86_64-linux"
    stage="$root/stage/clorraine-1.2.0-x86_64-linux"
    vsix="$root/clorraine-1.2.0.vsix"
    printf 'fake vsix\n' >"$vsix"
    output="$root/dist"
    mkdir -p "$output"
    local status
    status="$(run_package "$package_script" --stage "$stage" --output "$output" --version 1.2.0 --vsix "$vsix")"
    check_status 1 "$status" "version mismatch"
    check_contains "$root/stderr" "version" "version-mismatch message"
    check_absent "$output/clorraine-1.2.0-x86_64-linux.tar.gz" "no archive after a version mismatch"
}

# An extra file in the stage is rejected.
an_extra_file_fails() {
    fixture
    printf 'extra\n' >"$stage/bin/extra"
    local status
    status="$(run_package "$package_script" --stage "$stage" --output "$output" --version 1.2.0 --vsix "$vsix")"
    check_status 1 "$status" "extra file"
    check_contains "$root/stderr" "unexpected file" "extra-file message"
}

# A toolchain or ROM payload is rejected even with the right name.
a_forbidden_file_fails() {
    fixture
    printf 'not a real toolchain\n' >"$stage/toolchain.tar.xz"
    local status
    status="$(run_package "$package_script" --stage "$stage" --output "$output" --version 1.2.0 --vsix "$vsix")"
    check_status 1 "$status" "forbidden file"
    check_contains "$root/stderr" "unexpected file" "forbidden-file message"
}

# A stale installer pin fails verification after a successful build.
a_stale_installer_pin_fails() {
    fixture
    local status
    status="$(run_package "$package_script" --stage "$stage" --output "$output" --version 1.2.0 --vsix "$vsix")"
    check_status 0 "$status" "package before pin change"
    sed -i "s/ARCHIVE_SHA256='[0-9a-f]*'/ARCHIVE_SHA256='0000000000000000000000000000000000000000000000000000000000000000'/" "$output/install.sh"
    ( cd "$output" && sha256sum clorraine-1.2.0-x86_64-linux.tar.gz clorraine-1.2.0.vsix install.sh >SHA256SUMS )
    status="$(run_package "$package_script" --verify --output "$output" --version 1.2.0)"
    check_status 1 "$status" "stale pin"
    check_contains "$root/stderr" "pin" "stale-pin message"
}

# A missing VSIX is a producer error, not a silent partial candidate.
a_missing_vsix_fails() {
    fixture
    local status
    status="$(run_package "$package_script" --stage "$stage" --output "$output" --version 1.2.0 --vsix "$root/missing.vsix")"
    check_status 1 "$status" "missing vsix"
    check_contains "$root/stderr" "VSIX" "missing-vsix message"
}

# The release README uses repository URLs, not paths absent from the archive.
release_readme_has_no_local_links() {
    fixture
    local status
    status="$(run_package "$package_script" --stage "$stage" --output "$output" --version 1.2.0 --vsix "$vsix")"
    check_status 0 "$status" "package for README check"
    local archive="$output/clorraine-1.2.0-x86_64-linux.tar.gz"
    local extracted="$root/extracted"
    mkdir -p "$extracted"
    tar -xzf "$archive" -C "$extracted"
    local readme="$extracted/clorraine-1.2.0-x86_64-linux/README.md"
    if grep -Eq '\]\((\.\./|docs/|spec/|docker/|tools/)' "$readme"; then
        fail "release README has an archive-relative repository link"
    else
        pass
    fi
    check_contains "$readme" "https://github.com/OpenLorraine/clorraine" "release README repository URL"
}

build_creates_the_expected_assets
a_version_mismatch_fails
an_extra_file_fails
a_forbidden_file_fails
a_stale_installer_pin_fails
a_missing_vsix_fails
release_readme_has_no_local_links
finish
