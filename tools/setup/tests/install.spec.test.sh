#!/usr/bin/env bash
# Public installer contract for the Linux CLI candidate.
#
# These cases run the installed script, not its internals, against fixture
# archives and fake host/download tools. They cover host checks, local and
# network verification, activation, locking, PATH handling, and uninstall.

set -u

. "$(dirname "$0")/install_fixtures.sh"

# Builds one complete fixture and sets the shared globals.
fixture() {
    root="$(new_root)"
    prepare_installer_fixture "$root"
    stage="$(make_stage "$root")"
    archive="$(make_archive "$root" "$stage")"
    checksums="$root/SHA256SUMS"
    write_checksums "$archive" "$checksums"
    installer="$root/install.sh"
    assemble_installer "$installer_template" "$installer" 1.2.0 "$archive"
    prefix="$root/prefix"
    mkdir -p "$prefix/share"
}

# Runs the installer with a reduced PATH that contains every required tool
# except curl; local mode must not need the downloader.
run_without_curl() {
    local minimal="$root/minimal"
    mkdir -p "$minimal"
    for tool in uname getconf sha256sum tar gzip mktemp awk sed grep find wc \
        basename dirname sort ln mv rm mkdir cp chmod readlink head tr; do
        if [ -x "$root/fake/$tool" ]; then
            cp "$root/fake/$tool" "$minimal/$tool"
        else
            ln -s "$(command -v "$tool")" "$minimal/$tool"
        fi
    done
    HOME="$root/home" XDG_CONFIG_HOME="$root/config" PATH="$minimal" \
        "$installer" --prefix "$prefix" --archive "$archive" --checksums "$checksums" \
        >"$root/stdout" 2>"$root/stderr"
    printf '%s\n' "$?"
}

# The host gate rejects a non-Linux or non-x86_64 host and an old glibc.
host_checks_are_enforced() {
    fixture
    local status
    status="$(FAKE_UNAME_S=Darwin run_installer "$root" "$installer" --prefix "$prefix" --archive "$archive" --checksums "$checksums")"
    check_status 1 "$status" "Darwin host"
    check_contains "$root/stderr" "Linux x86_64 is required" "Darwin message"

    status="$(FAKE_UNAME_M=aarch64 run_installer "$root" "$installer" --prefix "$prefix" --archive "$archive" --checksums "$checksums")"
    check_status 1 "$status" "aarch64 host"
    check_contains "$root/stderr" "Linux x86_64 is required" "aarch64 message"

    status="$(FAKE_GLIBC='glibc 2.35' run_installer "$root" "$installer" --prefix "$prefix" --archive "$archive" --checksums "$checksums")"
    check_status 1 "$status" "old glibc"
    check_contains "$root/stderr" "2.38" "glibc floor"
    check_absent "$prefix/share/clorraine/current" "no activation on glibc failure"
}

# A PATH missing required commands names the gap; local mode works without curl.
command_checks_are_enforced() {
    fixture
    local status
    status="$(HOME="$root/home" XDG_CONFIG_HOME="$root/config" PATH="$root/fake" \
        "$installer" --prefix "$prefix" --archive "$archive" --checksums "$checksums" \
        >"$root/stdout" 2>"$root/stderr"; printf '%s' "$?")"
    check_status 1 "$status" "missing required tools"
    check_contains "$root/stderr" "missing required" "missing-tool message"

    local bare="$root/bare"
    mkdir -p "$bare"
    for tool in uname getconf sha256sum tar gzip mktemp awk sed grep find wc \
        basename dirname sort ln mv rm mkdir cp chmod readlink head tr; do
        if [ -x "$root/fake/$tool" ]; then
            cp "$root/fake/$tool" "$bare/$tool"
        else
            ln -s "$(command -v "$tool")" "$bare/$tool"
        fi
    done
    status="$(HOME="$root/home" XDG_CONFIG_HOME="$root/config" PATH="$bare" \
        "$installer" --prefix "$prefix" --archive "$archive" --checksums "$checksums" \
        >"$root/stdout" 2>"$root/stderr"; printf '%s' "$?")"
    check_status 0 "$status" "local mode without curl"
    check_symlink "$prefix/bin/clorraine" "../share/clorraine/current/bin/clorraine" "installed driver link"
    check_contains "$root/stdout" "installed Clorraine 1.2.0" "install message"
    status="$("$prefix/bin/clorraine" --version)"
    if [ "$status" = "clorraine 1.2.0" ]; then pass; else fail "installed version: $status"; fi
}

# Exact local bytes install; a wrong, missing, or duplicated checksum is rejected.
local_checksums_are_exact() {
    fixture
    local status
    status="$(FAKE_SHA256_MODE=zeros run_installer "$root" "$installer" --prefix "$prefix" --archive "$archive" --checksums "$checksums")"
    check_status 1 "$status" "wrong local hash"
    check_contains "$root/stderr" "SHA-256 mismatch" "wrong-hash message"
    check_absent "$prefix/share/clorraine/current" "no activation after a wrong hash"

    printf '%s  %s\n' "0000000000000000000000000000000000000000000000000000000000000000" "not-the-archive.tar.gz" >"$root/missing-sums"
    status="$(run_installer "$root" "$installer" --prefix "$prefix" --archive "$archive" --checksums "$root/missing-sums")"
    check_status 1 "$status" "missing checksum line"

    write_checksums "$archive" "$root/one-sum"
    cat "$root/one-sum" "$root/one-sum" >"$root/duplicate-sums"
    status="$(run_installer "$root" "$installer" --prefix "$prefix" --archive "$archive" --checksums "$root/duplicate-sums")"
    check_status 1 "$status" "duplicate checksum line"
}

# A network download is verified for exact bytes and hash before extraction.
network_download_is_verified() {
    fixture
    local status
    status="$(FAKE_CURL_SOURCE="$archive" run_installer "$root" "$installer" --prefix "$prefix")"
    check_status 0 "$status" "network install"
    check_symlink "$prefix/bin/clorraine" "../share/clorraine/current/bin/clorraine" "network driver link"

    local truncated="$root/truncated.tar.gz"
    cp "$archive" "$truncated"
    printf 'X' >>"$truncated"
    status="$(FAKE_CURL_SOURCE="$truncated" run_installer "$root" "$installer" --prefix "$root/prefix-truncated")"
    check_status 1 "$status" "byte-count mismatch"
    check_contains "$root/stderr" "byte count" "byte-count message"

    local corrupt="$root/corrupt.tar.gz"
    cp "$archive" "$corrupt"
    python3 - "$corrupt" <<'PY'
import sys
path = sys.argv[1]
data = bytearray(open(path, "rb").read())
data[10] ^= 0xFF
open(path, "wb").write(data)
PY
    status="$(FAKE_CURL_SOURCE="$corrupt" run_installer "$root" "$installer" --prefix "$root/prefix-corrupt")"
    check_status 1 "$status" "hash mismatch"
    check_contains "$root/stderr" "SHA-256 mismatch" "network hash message"
}

# Unsafe archive members are rejected before any extraction.
unsafe_archive_members_are_rejected() {
    fixture
    ln -s /etc/passwd "$stage/bin/evil"
    archive="$(make_archive "$root" "$stage")"
    write_checksums "$archive" "$checksums"
    assemble_installer "$installer_template" "$installer" 1.2.0 "$archive"
    local status
    status="$(run_installer "$root" "$installer" --prefix "$prefix" --archive "$archive" --checksums "$checksums")"
    check_status 1 "$status" "unsafe member"
    check_contains "$root/stderr" "unsafe member" "unsafe-member message"
    check_absent "$prefix/share/clorraine/current" "no activation after an unsafe archive"
}

# The installed binary must report the pinned version.
a_wrong_installed_version_is_rejected() {
    local other_root
    other_root="$(new_root)"
    prepare_installer_fixture "$other_root"
    local other_stage other_archive other_sums other_installer other_prefix
    other_stage="$(make_stage "$other_root" 1.1.0)"
    other_archive="$(make_archive "$other_root" "$other_stage")"
    other_sums="$other_root/SHA256SUMS"
    write_checksums "$other_archive" "$other_sums"
    other_installer="$other_root/install.sh"
    assemble_installer "$installer_template" "$other_installer" 1.2.0 "$other_archive"
    other_prefix="$other_root/prefix"
    mkdir -p "$other_prefix/share"
    local status
    status="$(run_installer "$other_root" "$other_installer" --prefix "$other_prefix" --archive "$other_archive" --checksums "$other_sums")"
    check_status 1 "$status" "wrong installed version"
    check_contains "$other_root/stderr" "version" "version-check message"
    check_absent "$other_prefix/share/clorraine/current" "no activation after a wrong version"
    rm -rf "$other_root"
}

# A live lock blocks; a stale owner is removed and the install proceeds.
locks_are_live_and_stale_safe() {
    fixture
    mkdir -p "$prefix/share/clorraine.lock"
    printf '%s %s\n' "$$" "$(awk '{print $22}' "/proc/$$/stat")" >"$prefix/share/clorraine.lock/owner"
    local status
    status="$(run_installer "$root" "$installer" --prefix "$prefix" --archive "$archive" --checksums "$checksums")"
    check_status 1 "$status" "live lock"
    check_contains "$root/stderr" "another install" "live-lock message"
    check_absent "$prefix/share/clorraine/current" "no activation under a live lock"

    printf '%s %s\n' 999999 0 >"$prefix/share/clorraine.lock/owner"
    status="$(run_installer "$root" "$installer" --prefix "$prefix" --archive "$archive" --checksums "$checksums")"
    check_status 0 "$status" "stale lock"
    check_symlink "$prefix/bin/clorraine" "../share/clorraine/current/bin/clorraine" "stale-lock install"
}

# --modify-path writes one quoted block; a second install replaces it and
# uninstall removes it without touching versions or config.
path_block_is_quoted_replaced_and_removed() {
    root="$(new_root)"
    prepare_installer_fixture "$root"
    prefix="$root/prefix with 'quote'"
    stage="$(make_stage "$root")"
    # Rebuild the stage under the prefix spelling only for the archive; the
    # installer owns the prefix name.
    archive="$(make_archive "$root" "$stage")"
    checksums="$root/SHA256SUMS"
    write_checksums "$archive" "$checksums"
    installer="$root/install.sh"
    assemble_installer "$installer_template" "$installer" 1.2.0 "$archive"

    local status
    status="$(run_installer "$root" "$installer" --prefix "$prefix" --modify-path --archive "$archive" --checksums "$checksums")"
    check_status 0 "$status" "quoted path install"
    check_contains "$root/home/.profile" "# >>> clorraine >>>" "block start"
    check_contains "$root/home/.profile" "clorraine_bin='$root/prefix with '\\''quote'\\''/bin'" "quoted prefix"
    status="$(run_installer "$root" "$installer" --prefix "$prefix" --modify-path --archive "$archive" --checksums "$checksums")"
    check_status 0 "$status" "second quoted install"
    local blocks
    blocks="$(grep -Fc "# >>> clorraine >>>" "$root/home/.profile")"
    if [ "$blocks" = 1 ]; then pass; else fail "profile has $blocks clorraine blocks"; fi

    status="$(run_installer "$root" "$installer" --prefix "$prefix" --uninstall)"
    check_status 0 "$status" "uninstall"
    check_absent "$prefix/bin/clorraine" "driver link removed"
    check_absent "$prefix/share/clorraine/current" "current removed"
    check_exists "$prefix/share/clorraine/versions" "versions kept"
    check_not_contains "$root/home/.profile" "# >>> clorraine >>>" "block removed"
}

host_checks_are_enforced
command_checks_are_enforced
local_checksums_are_exact
network_download_is_verified
unsafe_archive_members_are_rejected
a_wrong_installed_version_is_rejected
locks_are_live_and_stale_safe
path_block_is_quoted_replaced_and_removed
finish
