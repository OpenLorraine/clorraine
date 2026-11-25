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

# Runs the installer under a pseudo-terminal and feeds the interactive answers.
run_interactive() {
    local root=$1 input=$2 no_toolchain=${3:-0} status extra=
    if [ "$no_toolchain" = 1 ]; then extra=--no-toolchain; fi
    cat >"$root/interactive.sh" <<EOF
#!/bin/sh
exec "$installer" $extra --archive "$archive" --checksums "$checksums"
EOF
    chmod +x "$root/interactive.sh"
    ( export HOME="$root/home" XDG_CONFIG_HOME="$root/config" PATH="$root/fake:$PATH"
      printf '%b' "$input" | script -qec "$root/interactive.sh" /dev/null ) \
        >"$root/stdout" 2>"$root/stderr"
    status=$?
    printf '%s\n' "$status"
}

# Runs the installer with a reduced PATH that contains every required tool
# except curl; local mode must not need the downloader.
run_without_curl() {
    local minimal="$root/minimal"
    mkdir -p "$minimal"
    for tool in uname getconf sha256sum tar gzip mktemp awk sed grep find wc \
        basename dirname sort ln mv rm mkdir cp chmod readlink head tr sleep; do
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
        basename dirname sort ln mv rm mkdir cp chmod readlink head tr sleep; do
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
    status="$(FAKE_CURL_SOURCE="$archive" FAKE_CURL_LOG="$root/curl-args" run_installer "$root" "$installer" --prefix "$prefix")"
    check_status 0 "$status" "network install"
    local curl_args
    curl_args="$(cat "$root/curl-args" 2>/dev/null || true)"
    for expected in "--disable" "--fail" "--location" "--proto =https" \
        "--proto-redir =https" "--connect-timeout 20" "--max-time 600" \
        "--max-filesize $(($(wc -c <"$archive") + 1))"; do
        case "$curl_args" in
            *"$expected"*) pass ;;
            *) fail "curl arguments lack $expected" ;;
        esac
    done
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

    rm -rf "$prefix/share/clorraine.lock"
    mkdir -p "$prefix/share/clorraine.lock"
    status="$(run_installer "$root" "$installer" --prefix "$prefix" --archive "$archive" --checksums "$checksums")"
    check_status 1 "$status" "uninitialized lock"
    check_contains "$root/stderr" "another install" "uninitialized-lock message"
    check_exists "$prefix/share/clorraine.lock" "uninitialized lock is not deleted"
    rm -rf "$prefix/share/clorraine.lock"
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

# Hardlink and absolute members are rejected before extraction.
hostile_member_types_are_rejected() {
    fixture
    ln "$stage/bin/clc" "$stage/bin/clc-hard"
    archive="$(make_archive "$root" "$stage")"
    write_checksums "$archive" "$checksums"
    assemble_installer "$installer_template" "$installer" 1.2.0 "$archive"
    local status
    status="$(run_installer "$root" "$installer" --prefix "$prefix" --archive "$archive" --checksums "$checksums")"
    check_status 1 "$status" "hardlink member"
    check_contains "$root/stderr" "unsafe member" "hardlink message"

    local absolute="$root/absolute.tar.gz"
    python3 - "$absolute" <<'TARPY'
import io, sys, tarfile
with tarfile.open(sys.argv[1], "w:gz") as archive:
    info = tarfile.TarInfo("/absolute-member")
    info.size = 1
    archive.addfile(info, io.BytesIO(b"x"))
TARPY
    write_checksums "$absolute" "$root/absolute-sums"
    assemble_installer "$installer_template" "$installer" 1.2.0 "$absolute"
    status="$(run_installer "$root" "$installer" --prefix "$prefix" --archive "$absolute" --checksums "$root/absolute-sums")"
    check_status 1 "$status" "absolute member"
    check_contains "$root/stderr" "unsafe member" "absolute-member message"
}

# Uninstall keeps data; purge removes only this prefix and config file.
purge_scope_is_exact() {
    fixture
    local status
    status="$(run_installer "$root" "$installer" --prefix "$prefix" --archive "$archive" --checksums "$checksums")"
    check_status 0 "$status" "install before uninstall"
    mkdir -p "$root/config/clorraine"
    printf 'toolchain=/tmp/x\n' >"$root/config/clorraine/config"
    printf 'sibling\n' >"$root/config/clorraine/keep"
    status="$(run_installer "$root" "$installer" --prefix "$prefix" --uninstall)"
    check_status 0 "$status" "uninstall"
    check_exists "$prefix/share/clorraine/versions" "uninstall keeps versions"
    check_exists "$root/config/clorraine/config" "uninstall keeps config"

    status="$(run_installer "$root" "$installer" --prefix "$prefix" --uninstall --purge)"
    check_status 0 "$status" "purge with a sibling file"
    check_absent "$prefix/share/clorraine" "purge removes share"
    check_absent "$root/config/clorraine/config" "purge removes config file"
    check_exists "$root/config/clorraine/keep" "purge keeps a sibling"
    check_exists "$root/config/clorraine" "purge keeps a non-empty config dir"
    rm -f "$root/config/clorraine/keep"
    status="$(run_installer "$root" "$installer" --prefix "$prefix" --uninstall --purge)"
    check_status 0 "$status" "purge after removing the sibling"
    check_absent "$root/config/clorraine" "empty config dir is gone"

    status="$(run_installer "$root" "$installer" --prefix "$prefix" --archive "$archive" --checksums "$checksums")"
    check_status 0 "$status" "reinstall for home purge"
    mkdir -p "$root/home/.config/clorraine"
    printf 'x\n' >"$root/home/.config/clorraine/config"
    status="$(HOME="$root/home" XDG_CONFIG_HOME= PATH="$root/fake:$PATH" "$installer" --prefix "$prefix" --uninstall --purge >"$root/stdout" 2>"$root/stderr"; printf '%s' "$?")"
    check_status 0 "$status" "purge with HOME config"
    check_absent "$root/home/.config/clorraine/config" "home purge removes config"
    check_absent "$root/home/.config/clorraine" "home purge removes the empty config dir"
    check_absent "$prefix/share/clorraine" "home purge removes share"

    status="$(run_installer "$root" "$installer" --prefix "$prefix" --purge)"
    check_status 2 "$status" "purge without uninstall"
}

# A foreign symlink is never adopted and never removed by uninstall.
foreign_symlinks_are_not_owned() {
    fixture
    mkdir -p "$prefix/bin"
    ln -s "/srv/other/share/clorraine/current/bin/clorraine" "$prefix/bin/clorraine"
    local status
    status="$(run_installer "$root" "$installer" --prefix "$prefix" --archive "$archive" --checksums "$checksums")"
    check_status 1 "$status" "foreign symlink install"
    check_contains "$root/stderr" "not a Clorraine symlink" "foreign-symlink message"
    if [ -z "$(find "$root/run_tmp" -maxdepth 1 -name 'clorraine-verified.*' -print -quit 2>/dev/null)" ]; then
        pass
    else
        fail "foreign-symlink failure left a private verified copy"
    fi
    rm -f "$prefix/bin/clorraine"

    status="$(run_installer "$root" "$installer" --prefix "$prefix" --archive "$archive" --checksums "$checksums")"
    check_status 0 "$status" "valid install before foreign uninstall"
    rm -f "$prefix/bin/clorraine"
    ln -s "/srv/other/share/clorraine/current/bin/clorraine" "$prefix/bin/clorraine"
    status="$(run_installer "$root" "$installer" --prefix "$prefix" --uninstall)"
    check_status 0 "$status" "uninstall with a foreign link"
    check_exists "$prefix/bin/clorraine" "foreign link is left in place"
    rm -f "$prefix/bin/clorraine"
}

# Interactive prompts install into the default prefix and update PATH.
interactive_install_uses_the_prompts() {
    fixture
    local status
    status="$(run_interactive "$root" '\ny\ny\n' 1)"
    check_status 0 "$status" "interactive install"
    check_contains "$root/stdout" "Install prefix" "prefix prompt"
    check_contains "$root/stdout" "Add " "path prompt"
    check_contains "$root/stdout" "Install Clorraine" "confirm prompt"
    check_contains "$root/stdout" "installed Clorraine 1.2.0" "interactive success"
    check_contains "$root/stdout" "Next steps" "next-step output"
    check_exists "$root/home/.local/bin/clorraine" "default prefix"
    check_contains "$root/home/.profile" "# >>> clorraine >>>" "interactive PATH block"
}

# A typed prefix is used, and declining PATH leaves the profile untouched.
interactive_custom_prefix_is_used() {
    fixture
    local status
    status="$(run_interactive "$root" "$root/custom prefix\nn\ny\n" 1)"
    check_status 0 "$status" "interactive custom prefix"
    check_exists "$root/custom prefix/bin/clorraine" "custom prefix"
    check_absent "$root/home/.profile" "no PATH block after decline"
}

# Declining the final confirmation installs nothing.
interactive_decline_cancels() {
    fixture
    local status
    status="$(run_interactive "$root" '\ny\nn\n' 1)"
    check_status 0 "$status" "interactive decline"
    check_contains "$root/stdout" "cancelled" "cancellation message"
    check_absent "$root/home/.local/bin/clorraine" "no install after decline"
}

# A supplied rc24 root is configured and the host hello runs.
toolchain_root_is_configured_and_hello_runs() {
    fixture
    mkdir -p "$root/fake-toolchain"
    local status
    status="$(run_installer "$root" "$installer" --prefix "$prefix" \
        --toolchain "$root/fake-toolchain" --archive "$archive" --checksums "$checksums")"
    check_status 0 "$status" "toolchain root install"
    check_contains "$root/stdout" "configured toolchain" "toolchain setup"
    check_contains "$root/stdout" "Hello, Amiga, from Clorraine!" "host hello"
    check_contains "$root/stdout" "installed Clorraine 1.2.0" "CLI install"
}

# The interactive path also accepts an existing toolchain root.
interactive_toolchain_path_is_used() {
    fixture
    mkdir -p "$root/fake-toolchain"
    local status
    status="$(run_interactive "$root" "\ny\ny\n$root/fake-toolchain\n" 0)"
    check_status 0 "$status" "interactive toolchain root"
    check_contains "$root/stdout" "Toolchain" "toolchain prompt"
    check_contains "$root/stdout" "configured toolchain" "interactive setup"
    check_contains "$root/stdout" "Hello, Amiga, from Clorraine!" "interactive hello"
}

# --no-toolchain keeps the CLI-only behavior for scripts.
no_toolchain_skips_the_first_run() {
    fixture
    local status
    status="$(run_installer "$root" "$installer" --prefix "$prefix" --no-toolchain \
        --archive "$archive" --checksums "$checksums")"
    check_status 0 "$status" "no-toolchain install"
    check_contains "$root/stdout" "installed Clorraine 1.2.0" "CLI install"
    check_not_contains "$root/stdout" "configured toolchain" "no toolchain"
    check_not_contains "$root/stdout" "Hello, Amiga, from Clorraine!" "no hello"
}

# A non-terminal run keeps the old defaults and never blocks on a prompt.
non_interactive_defaults_do_not_prompt() {
    fixture
    local status=0
    HOME="$root/home" XDG_CONFIG_HOME="$root/config" PATH="$root/fake:$PATH" \
        "$installer" --archive "$archive" --checksums "$checksums" </dev/null \
        >"$root/stdout" 2>"$root/stderr" || status=$?
    check_status 0 "$status" "non-interactive defaults"
    check_exists "$root/home/.local/bin/clorraine" "non-interactive default prefix"
    check_not_contains "$root/stdout" "Install prefix" "no prefix prompt"
    check_absent "$root/home/.profile" "no implicit PATH change"
}

host_checks_are_enforced
command_checks_are_enforced
interactive_install_uses_the_prompts
interactive_custom_prefix_is_used
interactive_decline_cancels
non_interactive_defaults_do_not_prompt
foreign_symlinks_are_not_owned
hostile_member_types_are_rejected
purge_scope_is_exact
local_checksums_are_exact
network_download_is_verified
unsafe_archive_members_are_rejected
a_wrong_installed_version_is_rejected
locks_are_live_and_stale_safe
path_block_is_quoted_replaced_and_removed
toolchain_root_is_configured_and_hello_runs
interactive_toolchain_path_is_used
no_toolchain_skips_the_first_run
finish
