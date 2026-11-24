#!/bin/sh
# Clorraine Linux installer.
#
# The released script is assembled by `tools/setup/package.sh`, which replaces
# the five pins below. The checked-in copy is a template: its network path
# refuses to run until assembled, while `--archive/--checksums` works for a
# local candidate.
#
# Usage:
#   install.sh [--prefix DIR] [--modify-path]
#              [--archive FILE --checksums FILE] [--uninstall [--purge]]
#
# The installer never invokes Docker, a toolchain, a ROM, or FS-UAE. The
# archive is verified before extraction, and the active version is switched
# only after the extracted tree passes its version check.
set -eu
umask 022

VERSION='@VERSION@'
ARCHIVE_NAME='@ARCHIVE_NAME@'
ARCHIVE_URL='@ARCHIVE_URL@'
ARCHIVE_BYTES='@ARCHIVE_BYTES@'
ARCHIVE_SHA256='@ARCHIVE_SHA256@'

usage() {
    cat <<EOF
Usage: install.sh [OPTIONS]

Install Clorraine $VERSION under a prefix (default: \$HOME/.local).

Options:
  --prefix DIR              installation prefix (absolute path)
  --modify-path             add the bin directory to \$HOME/.profile
  --archive FILE            install an exact local archive
  --checksums FILE          SHA256SUMS for --archive
  --uninstall               remove the active version and PATH block
  --purge                   also remove versions and the CLI config
  -h, --help                show this help
EOF
}

# Removes terminal-control bytes from dynamic message text.
sanitize() {
    if command -v tr >/dev/null 2>&1; then
        printf '%s' "$1" | LC_ALL=C tr -d '\000-\011\013-\037\177'
    else
        printf '%s' "$1"
    fi
}

die() {
    printf 'clorraine: %s\n' "$(sanitize "$*")" >&2
    exit 1
}

usage_error() {
    printf 'clorraine: %s\n' "$(sanitize "$*")" >&2
    usage >&2
    exit 2
}

# True when every named command exists on PATH.
missing_commands() {
    missing=
    for tool in "$@"; do
        if ! command -v "$tool" >/dev/null 2>&1; then
            missing="$missing $tool"
        fi
    done
    printf '%s\n' "${missing# }"
}

require_commands() {
    missing=$(missing_commands "$@")
    if [ -n "$missing" ]; then
        die "missing required commands:$missing; install them and retry"
    fi
}

# Resolves the config directory exactly as the CLI does.
config_dir() {
    case "${XDG_CONFIG_HOME:-}" in
        /*) printf '%s\n' "$XDG_CONFIG_HOME/clorraine" ;;
        *) printf '%s\n' "$HOME/.config/clorraine" ;;
    esac
}

# Resolves an absolute prefix and rejects bytes that cannot be represented in a
# shell PATH or a terminal.
check_prefix() {
    case "$PREFIX" in
        /*) ;;
        *) die "--prefix must be an absolute path" ;;
    esac
    case "$PREFIX" in
        *:*|*'
'*) die "--prefix must not contain ':' or a newline" ;;
    esac
    # Control characters other than the newline rejected above.
    if printf '%s' "$PREFIX" | LC_ALL=C grep -q '[[:cntrl:]]'; then
        die "--prefix must not contain control characters"
    fi
    if [ ! -d "$PREFIX" ]; then
        mkdir -p "$PREFIX" 2>/dev/null || die "cannot create prefix '$PREFIX'"
    fi
    if [ ! -w "$PREFIX" ]; then
        die "prefix '$PREFIX' is not writable"
    fi
}

# Host gate: Linux x86_64 with glibc 2.38 or newer.
check_host() {
    system=$(uname -s)
    machine=$(uname -m)
    if [ "$system" != Linux ] || [ "$machine" != x86_64 ]; then
        die "Linux x86_64 is required"
    fi
    libc=$(getconf GNU_LIBC_VERSION 2>/dev/null || true)
    case "$libc" in
        glibc\ *) ;;
        *) die "Clorraine $VERSION needs glibc 2.38+ (Ubuntu 24.04+); found '$libc'" ;;
    esac
    major=${libc#glibc }
    major=${major%%.*}
    rest=${libc#glibc *.}
    minor=${rest%%.*}
    if [ -z "$major" ] || [ -z "$minor" ]; then
        die "Clorraine $VERSION needs glibc 2.38+ (Ubuntu 24.04+); found '$libc'"
    fi
    case "$major:$minor" in
        *[!0-9:]*|:*) die "Clorraine $VERSION needs glibc 2.38+ (Ubuntu 24.04+); found '$libc'" ;;
    esac
    if [ "$major" -lt 2 ] || { [ "$major" -eq 2 ] && [ "$minor" -lt 38 ]; }; then
        die "Clorraine $VERSION needs glibc 2.38+ (Ubuntu 24.04+); found '$libc'"
    fi
}

# Reads a lock owner file into LOCK_PID and LOCK_START.
read_lock_owner() {
    local owner=$1
    LOCK_PID=
    LOCK_START=
    [ -f "$owner" ] || return 1
    read -r LOCK_PID LOCK_START <"$owner" || true
    case "$LOCK_PID:$LOCK_START" in
        *[!0-9:]*|:*) return 1 ;;
    esac
    [ -n "$LOCK_PID" ] && [ -n "$LOCK_START" ]
}

# True when the recorded process is gone or was replaced.
lock_is_stale() {
    local owner=$1 stat start
    # An absent or partially written owner is not proof of a stale lock: a
    # concurrent creator writes it just after mkdir. Treat it as live.
    read_lock_owner "$owner" || return 1
    if ! kill -0 "$LOCK_PID" 2>/dev/null; then
        return 0
    fi
    stat="/proc/$LOCK_PID/stat"
    [ -r "$stat" ] || return 1
    start=$(awk '{print $22}' "$stat" 2>/dev/null || true)
    [ "$start" = "$LOCK_START" ] && return 1
    return 0
}

# Creates the lock directory and records this process.
write_lock_owner() {
    local lock=$1 start
    start=$(awk '{print $22}' "/proc/$$/stat" 2>/dev/null || true)
    printf '%s %s\n' "$$" "${start:-0}" >"$lock/owner"
}

# Takes the shared install/uninstall lock.
acquire_lock() {
    share=$PREFIX/share/clorraine
    mkdir -p "$PREFIX/share" || die "cannot create '$PREFIX/share'"
    lock=$PREFIX/share/clorraine.lock
    if mkdir "$lock" 2>/dev/null; then
        write_lock_owner "$lock"
        return 0
    fi
    # Wait out the short mkdir -> owner write window before deciding.
    local attempts=0
    while [ "$attempts" -lt 10 ]; do
        if read_lock_owner "$lock/owner"; then
            break
        fi
        sleep 0.05
        attempts=$((attempts + 1))
    done
    if lock_is_stale "$lock/owner"; then
        rm -rf "$lock"
        if mkdir "$lock" 2>/dev/null; then
            write_lock_owner "$lock"
            return 0
        fi
    fi
    if read_lock_owner "$lock/owner"; then
        die "another install is running (pid $LOCK_PID); retry after it exits"
    fi
    die "another install is running; retry after it exits"
}

release_lock() {
    [ -n "${lock:-}" ] && rm -rf "$lock"
}

# Removes only this invocation's private download and verification storage.
cleanup_temp() {
    if [ -n "${download_dir:-}" ]; then
        rm -rf "$download_dir"
    fi
    if [ -n "${verified_dir:-}" ]; then
        rm -rf "$verified_dir"
    fi
    return 0
}

# Canonical install tree for link ownership checks.
share_root() {
    readlink -f "$PREFIX/share/clorraine" 2>/dev/null || printf '%s' "$PREFIX/share/clorraine"
}

# True only when the link resolves inside this prefix's Clorraine share tree.
link_is_owned() {
    local link=$1 share resolved
    share=$(share_root)
    resolved=$(readlink -f "$link" 2>/dev/null || true)
    case "$resolved" in
        "$share"/*) return 0 ;;
        *) return 1 ;;
    esac
}

# POSIX single-quote escaping for a shell word.
single_quote() {
    local text=$1 output="'" character
    while [ -n "$text" ]; do
        character=${text%"${text#?}"}
        text=${text#?}
        if [ "$character" = "'" ]; then
            output="$output'\\''"
        else
            output="$output$character"
        fi
    done
    printf "%s'" "$output"
}

# Removes the marked block from a profile, if present.
strip_path_block() {
    local profile=$1 temporary=$2
    awk '
        /^# >>> clorraine >>>$/ { skip = 1; next }
        /^# <<< clorraine <<<$/ { skip = 0; next }
        skip != 1 { print }
    ' "$profile" >"$temporary"
}

# Writes or replaces the one marked PATH block.
modify_path() {
    local profile="$HOME/.profile"
    local temporary="$profile.clorraine.$$"
    local quoted
    quoted=$(single_quote "$PREFIX/bin")
    touch "$profile" || die "cannot update '$profile'"
    strip_path_block "$profile" "$temporary"
    {
        cat "$temporary"
        printf '%s\n' '# >>> clorraine >>>'
        printf "clorraine_bin=%s\n" "$quoted"
        printf '%s\n' 'case ":$PATH:" in *":$clorraine_bin:"*) ;; *) PATH="$clorraine_bin:$PATH";; esac'
        printf '%s\n' 'export PATH'
        printf '%s\n' '# <<< clorraine >>>'
    } >"$profile.clorraine.new"
    mv "$profile.clorraine.new" "$profile"
    rm -f "$temporary"
}

remove_path_block() {
    local profile="$HOME/.profile" temporary
    [ -f "$profile" ] || return 0
    temporary="$profile.clorraine.$$"
    strip_path_block "$profile" "$temporary"
    mv "$temporary" "$profile"
}

# Reads the checksum line for one archive and prints its digest.
local_checksum() {
    local archive=$1 checksums=$2 base expected
    base=$(basename "$archive")
    expected=$(awk -v name="$base" '
        $2 == name { print $1; count++ }
        END { exit count == 1 ? 0 : 1 }
    ' "$checksums") || die "checksum file '$checksums' has no unique entry for '$base'"
    printf '%s\n' "$expected"
}

# Verifies a local archive against its checksum file.
verify_local_archive() {
    local archive=$1 checksums=$2 expected actual bytes
    [ -f "$archive" ] || die "archive '$archive' is not a readable file"
    [ -f "$checksums" ] || die "checksum file '$checksums' is not a readable file"
    expected=$(local_checksum "$archive" "$checksums")
    actual=$(sha256sum "$archive" | awk '{print $1}')
    if [ "$actual" != "$expected" ]; then
        die "archive SHA-256 mismatch; expected $expected, got $actual"
    fi
    bytes=$(wc -c <"$archive")
    printf 'clorraine: using local archive (%s bytes)\n' "$bytes" >&2
}

# Downloads and verifies the assembled network archive.
download_archive() {
    case "$ARCHIVE_BYTES" in
        ''|*[!0-9]*) die "this is an unassembled installer template; use a released install.sh" ;;
    esac
    case "$ARCHIVE_SHA256" in
        ''|*[!0-9a-fA-F]*) die "this is an unassembled installer template; use a released install.sh" ;;
    esac
    if [ "$ARCHIVE_BYTES" -le 0 ] || [ "${#ARCHIVE_SHA256}" -ne 64 ]; then
        die "this is an unassembled installer template; use a released install.sh"
    fi
    require_commands curl
    download_dir=$(mktemp -d "${TMPDIR:-/tmp}/clorraine-install.XXXXXXXX") ||
        die "cannot create private download storage"
    downloaded="$download_dir/$ARCHIVE_NAME"
    if ! curl --disable --fail --location --proto '=https' --proto-redir '=https' \
        --connect-timeout 20 --max-time 600 \
        --max-filesize "$((ARCHIVE_BYTES + 1))" \
        --output "$downloaded" "$ARCHIVE_URL"; then
        die "download failed; check HTTPS access, curl, and CA certificates"
    fi
    actual_bytes=$(wc -c <"$downloaded")
    if [ "$actual_bytes" -ne "$ARCHIVE_BYTES" ]; then
        die "archive byte count mismatch; expected $ARCHIVE_BYTES, got $actual_bytes"
    fi
    actual_sha=$(sha256sum "$downloaded" | awk '{print $1}')
    if [ "$actual_sha" != "$ARCHIVE_SHA256" ]; then
        die "archive SHA-256 mismatch; expected $ARCHIVE_SHA256, got $actual_sha"
    fi
    archive=$downloaded
}

# Rejects absolute, parent-relative, and non-regular archive members.
validate_archive() {
    local archive=$1 listing types first top name type
    listing=$(tar -tzPf "$archive" 2>/dev/null) || die "archive '$archive' is not a readable tarball"
    [ -n "$listing" ] || die "archive '$archive' is empty"
    types=$(tar -tvzPf "$archive" 2>/dev/null) || die "archive '$archive' is not a readable tarball"
    while IFS= read -r line; do
        type=${line%"${line#?}"}
        case "$type" in
            l|h|b|c|p) die "archive contains an unsafe member: $line" ;;
        esac
    done <<EOF
$types
EOF
    first=
    while IFS= read -r name; do
        case "$name" in
            /*|..|../*|*/../*|*/..) die "archive contains an unsafe member: $name" ;;
        esac
        top=${name%%/*}
        [ "$top" != "$name" ] || die "archive must contain exactly one top-level directory"
        if [ -z "$first" ]; then
            first=$top
        elif [ "$top" != "$first" ]; then
            die "archive must contain exactly one top-level directory"
        fi
    done <<EOF
$listing
EOF
    [ -n "$first" ] || die "archive must contain exactly one top-level directory"
    printf '%s\n' "$first"
}

# Extracts the archive into a private staging directory.
extract_archive() {
    local archive=$1 staging=$2
    rm -rf "$staging"
    mkdir -p "$staging" || die "cannot create staging directory"
    tar -xzf "$archive" -C "$staging" --strip-components=1 \
        --no-same-owner --no-same-permissions ||
        die "cannot extract archive"
}

# Verifies the extracted layout and reports the version.
verify_tree() {
    local tree=$1 binary version
    for binary in clc clorraine clorraine-lsp; do
        [ -x "$tree/bin/$binary" ] || die "archive is missing bin/$binary"
    done
    for document in README.md LICENSE LICENSE-EXCEPTION.md NOTICE THIRD_PARTY_NOTICES.md; do
        [ -f "$tree/$document" ] || die "archive is missing $document"
    done
    version=$("$tree/bin/clorraine" --version 2>/dev/null || true)
    if [ "$version" != "clorraine $VERSION" ]; then
        die "installed binary failed its version check: '$version' (expected 'clorraine $VERSION')"
    fi
}

# Creates or replaces only Clorraine-owned bin symlinks.
link_bins() {
    local name link target
    mkdir -p "$PREFIX/bin" || die "cannot create '$PREFIX/bin'"
    # Check every destination before replacing any link, so a foreign file
    # cannot leave a half-updated bin directory. Remember which names were
    # absent so rollback removes only those links.
    absent_links=
    for name in clc clorraine clorraine-lsp; do
        link="$PREFIX/bin/$name"
        if [ -L "$link" ]; then
            link_is_owned "$link" || die "'$link' already exists and is not a Clorraine symlink"
        elif [ -e "$link" ]; then
            die "'$link' already exists and is not a Clorraine symlink"
        else
            absent_links="$absent_links $name"
        fi
    done
    for name in clc clorraine clorraine-lsp; do
        link="$PREFIX/bin/$name"
        target="../share/clorraine/current/bin/$name"
        rm -f "$link"
        ln -s "$target" "$link" || die "cannot create '$link'"
    done
}

# Switches current atomically.
activate() {
    local share=$1 version_dir=$2
    old_current=
    if [ -L "$share/current" ]; then
        old_current=$(readlink "$share/current")
    fi
    activation_started=1
    rm -f "$share/current.new"
    ln -s "$version_dir" "$share/current.new" || die "cannot create current.new"
    mv -Tf "$share/current.new" "$share/current" || die "cannot activate the new version"
}

# Restores the previous current and removes links this invocation created.
rollback() {
    local share=$1 name link
    [ "${activation_started:-0}" = 1 ] || return 0
    if [ -n "${old_current:-}" ]; then
        ln -sfn "$old_current" "$share/current"
    else
        rm -f "$share/current"
    fi
    for name in ${absent_links:-}; do
        link="$PREFIX/bin/$name"
        [ -L "$link" ] && rm -f "$link"
    done
    return 0
}

# Installs one verified archive.
install_archive() {
    local archive=$1 share version_dir staging sha12
    share="$PREFIX/share/clorraine"
    mkdir -p "$share/versions" || die "cannot create '$share/versions'"
    sha12=$(sha256sum "$archive" | awk '{print substr($1, 1, 12)}')
    version_dir="versions/$VERSION-$sha12"
    staging="$share/.tmp.$$"
    validate_archive "$archive" >/dev/null
    extract_archive "$archive" "$staging"
    verify_tree "$staging"
    if [ -d "$share/$version_dir" ]; then
        rm -rf "$staging"
    else
        mv "$staging" "$share/$version_dir" || die "cannot move the extracted tree into versions"
    fi
    verify_tree "$share/$version_dir"
    activate "$share" "$version_dir"
    link_bins
    rm -rf "$staging"
    if [ "$MODIFY_PATH" = 1 ]; then
        modify_path
        printf 'updated %s\n' "$HOME/.profile"
    else
        printf 'add this to your shell profile: export PATH=%s:$PATH\n' "$(single_quote "$PREFIX/bin")"
    fi
    printf 'installed Clorraine %s in %s\n' "$VERSION" "$PREFIX"
}

# Removes this prefix's links, current, and PATH block; keeps versions/config.
uninstall() {
    local share="$PREFIX/share/clorraine" name link
    for name in clc clorraine clorraine-lsp; do
        link="$PREFIX/bin/$name"
        if [ -L "$link" ] && link_is_owned "$link"; then
            rm -f "$link"
        fi
    done
    rm -f "$share/current"
    remove_path_block
    printf 'removed Clorraine from %s\n' "$PREFIX"
}

# Removes the install tree and the resolved config for --purge.
purge() {
    local config
    config="$(config_dir)"
    rm -rf "$PREFIX/share/clorraine"
    rm -f "$config/config"
    rmdir "$config" 2>/dev/null || true
    printf 'removed Clorraine data and %s\n' "$config/config"
}

main() {
    PREFIX="$HOME/.local"
    MODIFY_PATH=0
    UNINSTALL=0
    PURGE=0
    ARCHIVE=
    CHECKSUMS=
    download_dir=
    downloaded=
    archive=
    absent_links=
    activation_started=0
    old_current=
    verified_dir=
    downloaded=
    download_dir=
    trap 'cleanup_temp' EXIT

    while [ "$#" -gt 0 ]; do
        case "$1" in
            --prefix)
                [ "$#" -ge 2 ] || usage_error "--prefix needs a value"
                PREFIX=$2
                shift 2
                ;;
            --modify-path)
                MODIFY_PATH=1
                shift
                ;;
            --archive)
                [ "$#" -ge 2 ] || usage_error "--archive needs a value"
                ARCHIVE=$2
                shift 2
                ;;
            --checksums)
                [ "$#" -ge 2 ] || usage_error "--checksums needs a value"
                CHECKSUMS=$2
                shift 2
                ;;
            --uninstall)
                UNINSTALL=1
                shift
                ;;
            --purge)
                PURGE=1
                shift
                ;;
            -h|--help)
                usage
                exit 0
                ;;
            *)
                usage_error "unknown option '$1'"
                ;;
        esac
    done

    if [ "$PURGE" = 1 ] && [ "$UNINSTALL" = 0 ]; then
        usage_error "--purge requires --uninstall"
    fi
    if [ "$UNINSTALL" = 1 ] && { [ -n "$ARCHIVE" ] || [ -n "$CHECKSUMS" ] || [ "$MODIFY_PATH" = 1 ]; }; then
        usage_error "--uninstall cannot be combined with install options"
    fi
    if { [ -n "$ARCHIVE" ] && [ -z "$CHECKSUMS" ]; } || { [ -z "$ARCHIVE" ] && [ -n "$CHECKSUMS" ]; }; then
        usage_error "--archive and --checksums must be used together"
    fi

    require_commands uname tar gzip sha256sum getconf mktemp awk sed grep find \
        wc basename dirname sort ln mv rm mkdir cp chmod readlink sleep tr
    check_prefix
    check_host

    if [ "$UNINSTALL" = 1 ]; then
        acquire_lock
        trap 'release_lock; cleanup_temp' EXIT
        uninstall
        [ "$PURGE" = 1 ] && purge
        release_lock
        cleanup_temp
        trap - EXIT
        return 0
    fi

    if [ -n "$ARCHIVE" ]; then
        verify_local_archive "$ARCHIVE" "$CHECKSUMS"
        # Verify and use the same private copy; never re-open a user path for
        # extraction after the digest check.
        verified_dir=$(mktemp -d "${TMPDIR:-/tmp}/clorraine-verified.XXXXXXXX") ||
            die "cannot create private archive storage"
        archive="$verified_dir/$(basename "$ARCHIVE")"
        cp "$ARCHIVE" "$archive" || die "cannot copy the archive into private storage"
        verify_local_archive "$archive" "$CHECKSUMS"
    else
        download_archive
        archive=$downloaded
    fi

    acquire_lock
    trap 'rollback "$PREFIX/share/clorraine"; rm -rf "$PREFIX/share/clorraine/.tmp.$$"; release_lock; cleanup_temp' EXIT
    install_archive "$archive"
    release_lock
    cleanup_temp
    trap - EXIT
}

# Sourced implementation tests skip the command-line entry point.
case "${CLORRAINE_INSTALL_LIB:-0}" in
    1) ;;
    *) main "$@" ;;
esac
