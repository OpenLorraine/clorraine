#!/bin/sh
# Clorraine Linux installer.
#
# The released script is assembled by `tools/setup/package.sh`, which replaces
# the nine pins below. The checked-in copy is a template: its network paths
# refuse to run until assembled, while `--archive/--checksums` works for a
# local candidate.
#
# Usage:
#   install.sh [--prefix DIR] [--modify-path]
#              [--archive FILE --checksums FILE] [--uninstall [--purge]]
#
# The installer never invokes Docker. It verifies the CLI archive before
# extraction, optionally installs the pinned rc24 toolchain from AmigaPorts,
# remembers FS-UAE and a licensed ROM when they are found, runs the host hello
# and the demo-scene Amiga hello, and offers a starter project that opens in
# VS Code. The active version is switched only after the extracted tree passes
# its version check.
set -eu
umask 022

VERSION='@VERSION@'
ARCHIVE_NAME='@ARCHIVE_NAME@'
ARCHIVE_URL='@ARCHIVE_URL@'
ARCHIVE_BYTES='@ARCHIVE_BYTES@'
ARCHIVE_SHA256='@ARCHIVE_SHA256@'
# The VS Code extension, installed from the Visual Studio Marketplace by ID.
EXTENSION_ID=openlorraine.clorraine

# Plain-text style until `init_style` sees an interactive terminal.
BOLD= DIM= RESET= GREEN= YELLOW= CYAN= C1= C2= C3= C4= C5=
CHECK='[ok]' CROSS='[!!]' ARROW='>' PLAY='>' UNICODE=0

TOOLCHAIN_URL='https://github.com/AmigaPorts/m68k-amigaos-gcc/releases/download/v16.2-rc24/m68k-amigaos-gcc-16.2-rc24-x86_64-linux.tar.xz'
TOOLCHAIN_BYTES='69868548'
TOOLCHAIN_SHA256='b93b070ac47d2f01d126f57c03a8405f71fb470c8397b0bda51041217d25c100'

usage() {
    cat <<EOF
Usage: install.sh [OPTIONS]

Install Clorraine $VERSION under a prefix (default: \$HOME/.local).

Options:
  --prefix DIR              installation prefix (absolute path)
  --modify-path             add the bin directory to \$HOME/.profile
  --archive FILE            install an exact local archive
  --checksums FILE          SHA256SUMS for --archive
  --toolchain PATH          use an existing rc24 toolchain root
  --no-toolchain            skip toolchain setup and the hello checks
  --fs-uae PATH             FS-UAE executable for the guest hello
  --kickstart PATH          licensed KS3.1 A1200 ROM for the guest hello
  --no-emulator             skip FS-UAE detection and the Amiga hello
  --no-hello                do not run the hello programs
  --starter DIR             create a starter Amiga project in DIR
  --no-starter              do not offer a starter project
  --vscode                  install or update the VS Code extension from the
                            Marketplace without asking
  --no-vscode               leave VS Code and its extensions unchanged
  --uninstall               remove the active version and PATH block
  --purge                   also remove versions and the CLI config
  -h, --help                show this help

When stdin and stdout are terminals, an install asks for the prefix, the PATH
choice, confirmation, the toolchain, the ROM, whether to run the hello, the VS
Code extension, and a starter project in the current folder. Non-terminal runs
keep the flag/default behavior. The hello sources stay under
$PREFIX/share/clorraine/examples for later use.
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

# Verifies a local archive against its checksum file; a third argument
# `quiet` suppresses the report for the private re-check.
verify_local_archive() {
    local archive=$1 checksums=$2 quiet=${3:-} expected actual bytes
    [ -f "$archive" ] || die "archive '$archive' is not a readable file"
    [ -f "$checksums" ] || die "checksum file '$checksums' is not a readable file"
    expected=$(local_checksum "$archive" "$checksums")
    actual=$(sha256sum "$archive" | awk '{print $1}')
    if [ "$actual" != "$expected" ]; then
        die "archive SHA-256 mismatch; expected $expected, got $actual"
    fi
    bytes=$(wc -c <"$archive")
    [ -n "$quiet" ] || printf 'clorraine: using local archive (%s bytes)\n' "$bytes" >&2
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
        ok "updated $HOME/.profile"
    else
        printf 'add this to your shell profile: export PATH=%s:$PATH\n' "$(single_quote "$PREFIX/bin")"
    fi
    ok "installed Clorraine $VERSION in $PREFIX"
}

# Asks a yes/no question on the terminal; an empty answer uses the default.
confirm() {
    local prompt=$1 default=$2 answer
    while :; do
        printf '%s' "$prompt"
        read -r answer || return 0
        case "$answer" in
            '') [ "$default" = yes ] && return 0 || return 1 ;;
            y|Y|yes|YES) return 0 ;;
            n|N|no|NO) return 1 ;;
            *) printf 'Please answer y or n.\n' ;;
        esac
    done
}

# Where downloaded toolchains are stored, following the XDG data rule.
toolchain_data_dir() {
    case "${XDG_DATA_HOME:-}" in
        /*) printf '%s\n' "$XDG_DATA_HOME/clorraine/toolchains" ;;
        *) printf '%s\n' "$HOME/.local/share/clorraine/toolchains" ;;
    esac
}

# Downloads, verifies, and extracts the pinned rc24 toolchain.
download_toolchain() {
    local work archive root
    [ -n "${TOOLCHAIN_DIR:-}" ] || TOOLCHAIN_DIR="$(toolchain_data_dir)/rc24"
    if ! command -v xz >/dev/null 2>&1; then
        printf 'clorraine: xz is required to extract the rc24 toolchain\n' >&2
        return 1
    fi
    work=$(mktemp -d "${TMPDIR:-/tmp}/clorraine-toolchain.XXXXXXXX") || {
        printf 'clorraine: cannot create toolchain storage\n' >&2
        return 1
    }
    archive="$work/toolchain.tar.xz"
    printf 'Downloading the rc24 toolchain from AmigaPorts...\n' >&2
    if ! curl --disable --fail --location --proto '=https' --proto-redir '=https' \
        --connect-timeout 20 --max-time 1200 \
        --max-filesize "$((TOOLCHAIN_BYTES + 1))" \
        --output "$archive" "$TOOLCHAIN_URL"; then
        printf 'clorraine: toolchain download failed; check HTTPS access and retry\n' >&2
        rm -rf "$work"
        return 1
    fi
    if [ "$(wc -c <"$archive")" != "$TOOLCHAIN_BYTES" ]; then
        printf 'clorraine: toolchain byte count mismatch\n' >&2
        rm -rf "$work"
        return 1
    fi
    if [ "$(sha256sum "$archive" | awk '{print $1}')" != "$TOOLCHAIN_SHA256" ]; then
        printf 'clorraine: toolchain SHA-256 mismatch\n' >&2
        rm -rf "$work"
        return 1
    fi
    if ! tar -xJf "$archive" -C "$work"; then
        printf 'clorraine: cannot extract the rc24 toolchain\n' >&2
        rm -rf "$work"
        return 1
    fi
    root=$(find "$work" -maxdepth 3 -path '*/bin/m68k-amigaos-gcc' -print -quit)
    if [ -z "$root" ]; then
        printf 'clorraine: the extracted toolchain has no m68k-amigaos-gcc\n' >&2
        rm -rf "$work"
        return 1
    fi
    root=${root%/bin/m68k-amigaos-gcc}
    mkdir -p "$(dirname "$TOOLCHAIN_DIR")" || {
        printf 'clorraine: cannot create the toolchain directory\n' >&2
        rm -rf "$work"
        return 1
    }
    rm -rf "$TOOLCHAIN_DIR"
    if ! mv "$root" "$TOOLCHAIN_DIR"; then
        printf 'clorraine: cannot move the toolchain into place\n' >&2
        rm -rf "$work"
        return 1
    fi
    rm -rf "$work"
    printf '%s\n' "$TOOLCHAIN_DIR"
}

# Writes the portable host hello source to one path.
write_hello_source() {
    cat >"$1" <<'EOF'
// The smallest Clorraine program: print one templated line.
module demo.Hello;

import { println } from std.io;

func main(): i32 {
    let who: str = "Amiga";
    println($"Hello, {who}, from Clorraine!");
    return 0;
}
EOF
}

# Seconds an editor command may run before it is stopped. Installing the
# extension talks to the Marketplace, and a stuck proxy must not hang the
# installer; CLORRAINE_EDITOR_TIMEOUT overrides the bound with a whole number
# of seconds from 1 to 86400 (one day), as on Windows.
#
# The editor runs in timeout's own process group, so Ctrl-C ends the installer
# but not a Marketplace install already under way; it stops by itself at the
# bound. Running it in the foreground group would lose the bound's kill of
# the editor's child processes.
editor_timeout() {
    local seconds=${CLORRAINE_EDITOR_TIMEOUT:-}
    case "$seconds" in
        ''|*[!0-9]*) seconds=300 ;;
    esac
    { [ "$seconds" -gt 0 ] && [ "$seconds" -le 86400 ]; } 2>/dev/null || seconds=300
    printf '%s\n' "$seconds"
}

# Runs an editor command within the bound, keeping its standard output.
bounded_editor() {
    if command -v timeout >/dev/null 2>&1; then
        timeout --kill-after=5 "$(editor_timeout)" "$@" </dev/null 2>/dev/null
    else
        "$@" </dev/null 2>/dev/null
    fi
}

# Runs an editor command within the bound and discards its output.
run_editor() {
    bounded_editor "$@" >/dev/null
}

# Finds a working VS Code-family launcher.
detect_editor() {
    local name path
    for name in code code-insiders; do
        path=$(command -v "$name" 2>/dev/null) || continue
        if run_editor "$path" --version; then
            printf '%s\n' "$path"
            return 0
        fi
    done
    return 1
}

# True when the editor already lists the Clorraine extension.
vscode_extension_installed() {
    bounded_editor "$1" --list-extensions | grep -Fxiq "$EXTENSION_ID"
}

# Installs the VS Code extension from the Marketplace, or reinstalls it when
# it is already there. A failure is reported and never stops the install.
install_vscode() {
    local editor
    VSCODE_STATE=${VSCODE_STATE:-unavailable}
    if [ "$NO_VSCODE" = 1 ]; then
        VSCODE_STATE=skipped
        return 0
    fi
    if ! editor=$(detect_editor); then
        if [ "$VSCODE" = 1 ]; then
            printf 'VS Code was not found; install VS Code, then run: code --install-extension %s\n' "$EXTENSION_ID" >&2
        fi
        VSCODE_STATE=unavailable
        return 0
    fi
    if vscode_extension_installed "$editor"; then
        # Uninstalling first replaces an install from a release file, which VS
        # Code pins and never updates, with a Marketplace install it keeps
        # current; --force alone would keep the pin.
        if ! run_editor "$editor" --uninstall-extension "$EXTENSION_ID"; then
            printf 'could not update the VS Code extension; run: code --install-extension %s --force\n' "$EXTENSION_ID" >&2
            VSCODE_STATE=failed
            return 0
        fi
        if run_editor "$editor" --install-extension "$EXTENSION_ID"; then
            ok "updated the Clorraine extension from the Marketplace"
            VSCODE_STATE=updated
        else
            printf 'the Clorraine extension was removed but could not be reinstalled from the Marketplace; run: code --install-extension %s\n' "$EXTENSION_ID" >&2
            VSCODE_STATE=failed
        fi
        return 0
    fi
    if [ "$VSCODE_SET" = 0 ]; then
        if [ -t 0 ] && [ -t 1 ]; then
            if ! confirm "  Install the Clorraine extension in VS Code? [Y/n] " yes; then
                VSCODE_STATE=skipped
                return 0
            fi
        else
            printf 'VS Code detected; run with --vscode to install the extension\n'
            VSCODE_STATE=available
            return 0
        fi
    fi
    if run_editor "$editor" --install-extension "$EXTENSION_ID"; then
        ok "installed the Clorraine extension in VS Code"
        VSCODE_STATE=installed
    else
        printf 'could not install the VS Code extension from the Marketplace; run: code --install-extension %s\n' "$EXTENSION_ID" >&2
        VSCODE_STATE=failed
    fi
    return 0
}

# Finds a working FS-UAE executable from flags, environment, or PATH.
detect_fs_uae() {
    local candidate
    if [ -n "${FS_UAE:-}" ]; then
        candidate=$FS_UAE
        if [ -x "$candidate" ] && "$candidate" --version >/dev/null 2>&1; then
            printf '%s\n' "$candidate"
            return 0
        fi
        printf 'clorraine: FS-UAE is not executable: %s\n' "$candidate" >&2
        return 1
    fi
    if [ -n "${CLORRAINE_FS_UAE:-}" ]; then
        candidate=$CLORRAINE_FS_UAE
        if [ -x "$candidate" ] && "$candidate" --version >/dev/null 2>&1; then
            printf '%s\n' "$candidate"
            return 0
        fi
        printf 'clorraine: FS-UAE is not executable: %s\n' "$candidate" >&2
        return 1
    fi
    candidate=$(command -v fs-uae 2>/dev/null) || return 1
    if [ -x "$candidate" ] && "$candidate" --version >/dev/null 2>&1; then
        printf '%s\n' "$candidate"
        return 0
    fi
    return 1
}

# Finds a 512 KB KS3.1 A1200 ROM from flags, environment, or common paths.
detect_kickstart() {
    local path size dir
    for path in "${KICKSTART:-}" "${CLORRAINE_KICKSTART:-}"; do
        [ -n "$path" ] || continue
        if [ -f "$path" ]; then
            printf '%s\n' "$path"
            return 0
        fi
        printf 'clorraine: Kickstart ROM is not a file: %s\n' "$path" >&2
        return 1
    done
    for path in \
        "$HOME/amiga/roms/amiga-os-310-a1200.rom" \
        "$HOME/amiga/roms/kick31a1200.rom" \
        "$HOME/Documents/FS-UAE/Kickstarts/kick31a1200.rom" \
        "$HOME/Documents/FS-UAE/Kickstarts/kick31.rom" \
        "$HOME/.local/share/fs-uae/Kickstarts/kick31a1200.rom" \
        "$HOME/.config/fs-uae/Kickstarts/kick31a1200.rom" \
        "$HOME/Desktop/kick31a1200.rom"
    do
        [ -f "$path" ] || continue
        size=$(wc -c <"$path")
        case "$size" in
            524288|524299) printf '%s\n' "$path"; return 0 ;;
        esac
    done
    for dir in \
        "$HOME/amiga/roms" \
        "$HOME/Documents/FS-UAE/Kickstarts" \
        "$HOME/.local/share/fs-uae/Kickstarts" \
        "$HOME/.config/fs-uae/Kickstarts"
    do
        [ -d "$dir" ] || continue
        for path in "$dir"/*a1200*.rom "$dir"/*A1200*.rom "$dir"/kick31*.rom "$dir"/Kick31*.rom; do
            [ -f "$path" ] || continue
            size=$(wc -c <"$path")
            case "$size" in
                524288|524299) printf '%s\n' "$path"; return 0 ;;
            esac
        done
    done
    return 1
}

# Sets the colours and symbols for an interactive terminal. Plain text is
# used for pipes, NO_COLOR, a dumb terminal, or a non-UTF-8 locale.
init_style() {
    BOLD= DIM= RESET= GREEN= YELLOW= CYAN= C1= C2= C3= C4= C5=
    CHECK='[ok]' CROSS='[!!]' ARROW='>' PLAY='>'
    UNICODE=0
    case "${LC_ALL:-${LC_CTYPE:-${LANG:-}}}" in
        *UTF-8*|*utf-8*|*UTF8*|*utf8*) UNICODE=1 ;;
    esac
    if [ "$UNICODE" = 1 ]; then
        CHECK='✔' CROSS='✘' ARROW='▸' PLAY='▶'
    fi
    if [ -t 1 ] && [ -z "${NO_COLOR:-}" ] && [ "${TERM:-dumb}" != dumb ]; then
        esc=$(printf '\033')
        BOLD="${esc}[1m" DIM="${esc}[2m" RESET="${esc}[0m"
        GREEN="${esc}[38;5;114m" YELLOW="${esc}[38;5;221m" CYAN="${esc}[38;5;81m"
        C1="${esc}[38;5;51m" C2="${esc}[38;5;75m" C3="${esc}[38;5;141m"
        C4="${esc}[38;5;205m" C5="${esc}[38;5;215m"
    fi
}

# Prints the opening banner on a terminal.
banner() {
    printf '\n'
    if [ "$UNICODE" = 1 ]; then
        printf '  %s%s%s\n' "$C1" '▄▀▀▀▄ █     ▄▀▀▀▄ █▀▀▀▄ █▀▀▀▄ ▄▀▀▀▄ ▀▀█▀▀ █▄  █ █▀▀▀▀' "$RESET"
        printf '  %s%s%s\n' "$C2" '█     █     █   █ █▄▄▄▀ █▄▄▄▀ █▄▄▄█   █   █ ▀▄█ █▄▄▄' "$RESET"
        printf '  %s%s%s\n' "$C3" '█   ▄ █     █   █ █ ▀▄  █ ▀▄  █   █   █   █   █ █' "$RESET"
        printf '  %s%s%s\n' "$C4" ' ▀▀▀  ▀▀▀▀▀  ▀▀▀  ▀   ▀ ▀   ▀ ▀   ▀ ▀▀▀▀▀ ▀   ▀ ▀▀▀▀▀' "$RESET"
    else
        printf '  %sC L O R R A I N E%s\n' "$BOLD" "$RESET"
    fi
    printf '\n  %sSafe, modern programming for the Amiga.%s\n' "$C5" "$RESET"
    printf '  %sClorraine %s developer preview%s\n' "$DIM" "$VERSION" "$RESET"
}

# True when both stdin and stdout are terminals, so prompts are possible.
interactive() {
    [ -t 0 ] && [ -t 1 ]
}

# Prints a numbered step heading.
step() {
    printf '\n%s%s Step %s of 5%s  %s%s%s\n' "$CYAN" "$ARROW" "$1" "$RESET" "$BOLD" "$2" "$RESET"
}

# Prints a success line.
ok() {
    printf '  %s%s%s %s\n' "$GREEN" "$CHECK" "$RESET" "$*"
}

# Prints a warning line.
warn() {
    printf '  %s%s%s %s\n' "$YELLOW" "$CROSS" "$RESET" "$*"
}

# Shows a path with ~ for the home directory when that stays a plain word.
pretty_path() {
    case "$1" in
        "$HOME"/*)
            rest=${1#"$HOME"/}
            case "$rest" in
                *[!A-Za-z0-9._/+-]*) single_quote "$1" ;;
                *) printf '~/%s\n' "$rest" ;;
            esac
            ;;
        *[!A-Za-z0-9._/+-]*) single_quote "$1" ;;
        *) printf '%s\n' "$1" ;;
    esac
}

# The command name to show: plain `clorraine` when it is (or will be) on PATH.
cli_name() {
    case ":$PATH:" in
        *":$PREFIX/bin:"*) printf 'clorraine\n'; return ;;
    esac
    if [ "${MODIFY_PATH:-0}" = 1 ]; then
        printf 'clorraine\n'
    else
        pretty_path "$PREFIX/bin/clorraine"
    fi
}

# Runs one `clorraine setup` and shows its report as success lines, or its
# error as a warning.
run_setup() {
    local output status=0 line
    output=$("$PREFIX/bin/clorraine" "$@" 2>&1) || status=$?
    SETUP_OUTPUT=$output
    printf '%s\n' "$output" | while IFS= read -r line; do
        [ -n "$line" ] || continue
        if [ "$status" -eq 0 ]; then
            ok "$(sanitize "$line")"
        else
            warn "$(sanitize "$line")"
        fi
    done
    return "$status"
}

# Runs the installed CLI setup with the toolchain and any selected emulator
# paths. A rejected emulator path is reported and setup is retried without
# the emulator, so the toolchain is still configured.
configure_toolchain() {
    local root=$1
    if [ -n "${FS_UAE_SELECTED:-}" ] || [ -n "${KICKSTART_SELECTED:-}" ]; then
        set -- setup --toolchain "$root"
        [ -n "${FS_UAE_SELECTED:-}" ] && set -- "$@" --fs-uae "$FS_UAE_SELECTED"
        [ -n "${KICKSTART_SELECTED:-}" ] && set -- "$@" --kickstart "$KICKSTART_SELECTED"
        if run_setup "$@"; then
            return 0
        fi
        # Only a rejected emulator path is worth a second try; a toolchain
        # problem would fail the same way again.
        case "$SETUP_OUTPUT" in
            *'clorraine: --fs-uae:'*|*'clorraine: --kickstart:'*) ;;
            *) return 1 ;;
        esac
        warn "the emulator paths were not accepted; continuing without the Amiga run"
        FS_UAE_SELECTED=
        KICKSTART_SELECTED=
    fi
    run_setup setup --toolchain "$root"
}

# Writes the Amiga hello: the demo-scene intro shown in FS-UAE.
write_guest_hello_source() {
    cat >"$1" <<'CLORRAINE_HELLO_AMIGA'
@HELLO_AMIGA_SOURCE@
CLORRAINE_HELLO_AMIGA
}

# Leaves the hello sources under the prefix without overwriting a user edit.
install_examples() {
    local examples="$PREFIX/share/clorraine/examples"
    HELLO_FILE="$examples/hello.clo"
    HELLO_GUEST_FILE="$examples/hello-amiga.clo"
    mkdir -p "$examples" || {
        printf 'clorraine: cannot create %s\n' "$examples" >&2
        return 1
    }
    if [ ! -e "$HELLO_FILE" ] && [ ! -L "$HELLO_FILE" ]; then
        write_hello_source "$HELLO_FILE" || return 1
    fi
    if [ ! -e "$HELLO_GUEST_FILE" ] && [ ! -L "$HELLO_GUEST_FILE" ]; then
        write_guest_hello_source "$HELLO_GUEST_FILE" || return 1
    fi
    chmod 644 "$HELLO_FILE" "$HELLO_GUEST_FILE" 2>/dev/null || true
}

# Builds and runs the persistent host hello; requires the exact output.
run_hello() {
    local dir output status
    [ -f "${HELLO_FILE:-}" ] || return 1
    dir=$(mktemp -d "${TMPDIR:-/tmp}/clorraine-hello.XXXXXXXX") || return 1
    status=0
    output=$(cd "$dir" && "$PREFIX/bin/clorraine" run "$HELLO_FILE" 2>&1) || status=$?
    rm -rf "$dir"
    printf '    %s%s%s\n' "$BOLD" "$(sanitize "$output")" "$RESET"
    [ "$status" -eq 0 ] && [ "$output" = "Hello, Amiga, from Clorraine!" ]
}

# Makes a typed path absolute, expanding a leading ~/.
absolute_path() {
    case "$1" in
        '~/'*) printf '%s/%s\n' "$HOME" "${1#'~/'}" ;;
        /*) printf '%s\n' "$1" ;;
        *) printf '%s/%s\n' "$PWD" "$1" ;;
    esac
}

# Detects FS-UAE and a Kickstart ROM for the Amiga run and asks on a terminal.
select_emulator() {
    local emulator rom answer
    if [ "$NO_EMULATOR" = 1 ]; then
        return 0
    fi
    if ! emulator=$(detect_fs_uae); then
        warn "FS-UAE was not found; the Amiga run is skipped (install FS-UAE 3.2 or newer to see it)"
        return 0
    fi
    rom=$(detect_kickstart || true)
    if [ -t 0 ] && [ -t 1 ]; then
        ok "FS-UAE detected: $(pretty_path "$emulator")"
        if [ -n "$rom" ]; then
            printf '  Kickstart 3.1 A1200 ROM [%s] (Enter to use, - to skip): ' "$(pretty_path "$rom")"
            read -r answer || answer=
            case "$answer" in
                '') ;;
                -) rom= ;;
                *) rom=$answer ;;
            esac
        else
            printf '  Kickstart 3.1 A1200 ROM path (leave empty to skip the Amiga run): '
            read -r rom || rom=
        fi
    elif [ -z "${FS_UAE:-}${CLORRAINE_FS_UAE:-}" ] || [ -z "${KICKSTART:-}${CLORRAINE_KICKSTART:-}" ]; then
        printf 'FS-UAE detected; pass --fs-uae and --kickstart to run the Amiga hello\n'
        return 0
    fi
    FS_UAE_SELECTED=$(absolute_path "$emulator")
    if [ -z "$rom" ]; then
        warn "no Kickstart ROM selected; the Amiga run is skipped"
        return 0
    fi
    KICKSTART_SELECTED=$(absolute_path "$rom")
}

# Runs the demo-scene hello in FS-UAE when an emulator and ROM are ready.
run_guest_hello() {
    local output status work
    work=$(mktemp -d "${TMPDIR:-/tmp}/clorraine-guest.XXXXXXXX") || return 0
    printf '  Booting an Amiga 1200 in FS-UAE. The intro ends by itself, or click in its window.\n'
    status=0
    output=$(cd "$work" && CLORRAINE_EMULATOR_TIMEOUT_SECONDS="${CLORRAINE_EMULATOR_TIMEOUT_SECONDS:-90}" \
        "$PREFIX/bin/clorraine" run -p hosted-amiga --release --emulator \
        --fs-uae "$FS_UAE_SELECTED" --kickstart "$KICKSTART_SELECTED" "$HELLO_GUEST_FILE" \
        </dev/null 2>"$work/stderr") || status=$?
    if [ "$status" -eq 0 ] && [ "$output" = "Hello, Amiga, from Clorraine!" ]; then
        printf '    %s%s%s\n' "$BOLD" "$output" "$RESET"
        ok "your Amiga said hello"
        GUEST_STATE=passed
    else
        [ -n "$output" ] && printf '%s\n' "$(sanitize "$output")"
        warn "the Amiga run did not complete (status $status)"
        if [ -s "$work/stderr" ]; then
            sed 's/^/    /' "$work/stderr" >&2
        fi
        GUEST_STATE=failed
    fi
    rm -rf "$work"
}

# Picks ./amiga-starter, or the first free amiga-starter-N.
default_starter_dir() {
    local base="$PWD/amiga-starter" candidate n=2
    candidate=$base
    while [ -e "$candidate" ] && [ -n "$(ls -A "$candidate" 2>/dev/null)" ]; do
        candidate="$base-$n"
        n=$((n + 1))
    done
    printf '%s\n' "$candidate"
}

# Offers a starter project in the current folder and opens it in VS Code.
offer_starter() {
    local dir answer editor
    dir=$STARTER
    if [ -z "$dir" ]; then
        if [ "$NO_STARTER" = 1 ] || [ ! -t 0 ] || [ ! -t 1 ]; then
            return 0
        fi
        dir=$(default_starter_dir)
        if ! confirm "  Create a starter Amiga project in $(pretty_path "$dir")? [Y/n] " yes; then
            return 0
        fi
    fi
    if ! "$PREFIX/bin/clorraine" new "$dir" >/dev/null; then
        warn "could not create the starter project in $(pretty_path "$dir")"
        return 0
    fi
    STARTER_DIR=$dir
    ok "created $(pretty_path "$dir") with main.clo and a VS Code setup"
    if [ -t 0 ] && [ -t 1 ] && [ "$NO_VSCODE" = 0 ] && editor=$(detect_editor); then
        if confirm "  Open it in VS Code now? [Y/n] " yes; then
            PATH="$PREFIX/bin:$PATH" "$editor" "$dir" "$dir/main.clo" </dev/null >/dev/null 2>&1 &
            ok "opening VS Code: press $PLAY Run in FS-UAE in the editor title bar"
            STARTER_OPENED=1
        fi
    fi
}

# Asks before running the hello programs on a terminal, then runs the host
# hello and, with FS-UAE and a ROM, the Amiga intro. Fails only when the host
# hello itself is wrong.
run_first_hello() {
    if [ "$NO_HELLO" = 1 ]; then
        return 0
    fi
    if [ -t 0 ] && [ -t 1 ]; then
        confirm "  Run the hello program now? [Y/n] " yes || return 0
    fi
    if command -v cc >/dev/null 2>&1; then
        if ! run_hello; then
            HOST_STATE=failed
            return 1
        fi
        HOST_STATE=passed
        ok "the compiler, the toolchain, and your C compiler work"
    else
        warn "cc not found; skipping the host hello check"
    fi
    if [ -n "${KICKSTART_SELECTED:-}" ] && [ -n "${FS_UAE_SELECTED:-}" ] && [ -f "${HELLO_GUEST_FILE:-}" ]; then
        if [ -t 0 ] && [ -t 1 ]; then
            confirm "  Now watch it on an Amiga 1200 in FS-UAE? [Y/n] " yes || return 0
        fi
        run_guest_hello
    fi
    return 0
}

# Prints the closing summary and the exact next commands.
print_summary() {
    local cli
    cli=$(cli_name)
    printf '\n%s%s First run complete. You are all set!%s\n\n' "$GREEN$BOLD" "$CHECK" "$RESET"
    printf '  CLI        %s\n' "$(pretty_path "$PREFIX/bin/clorraine")"
    if [ -n "${TOOLCHAIN_ROOT:-}" ]; then
        printf '  Toolchain  %s\n' "$(pretty_path "$TOOLCHAIN_ROOT")"
    else
        printf '  Toolchain  not configured\n'
    fi
    if [ -n "${KICKSTART_SELECTED:-}" ]; then
        printf '  Emulator   FS-UAE with %s\n' "$(pretty_path "$KICKSTART_SELECTED")"
    else
        printf '  Emulator   not set up\n'
    fi
    case "${HOST_STATE:-skipped}/${GUEST_STATE:-skipped}" in
        passed/passed) printf '  Hello      ran on this PC and on the Amiga\n' ;;
        passed/failed) printf '  Hello      ran on this PC; the Amiga run failed\n' ;;
        passed/*) printf '  Hello      ran on this PC\n' ;;
        failed/*) printf '  Hello      failed\n' ;;
        *) printf '  Hello      not run\n' ;;
    esac
    case "${VSCODE_STATE:-unavailable}" in
        installed) printf '  VS Code    extension installed\n' ;;
        updated) printf '  VS Code    extension updated from the Marketplace\n' ;;
        failed) printf '  VS Code    extension not installed; run: code --install-extension %s\n' "$EXTENSION_ID" ;;
        available) printf '  VS Code    detected; rerun with --vscode to install the extension\n' ;;
        skipped) printf '  VS Code    skipped\n' ;;
        *) printf '  VS Code    not found\n' ;;
    esac
    printf '  Examples   %s\n' "$(pretty_path "${HELLO_FILE:-$PREFIX/share/clorraine/examples/hello.clo}")"
    if [ -n "${STARTER_DIR:-}" ]; then
        printf '  Project    %s\n' "$(pretty_path "$STARTER_DIR")"
    fi

    printf '\n%sWhat next%s\n' "$BOLD" "$RESET"
    if [ -n "${STARTER_DIR:-}" ]; then
        printf '  cd %s\n' "$(pretty_path "$STARTER_DIR")"
        printf '  code .                      %s# open main.clo, change MESSAGE, press %s Run in FS-UAE%s\n' "$DIM" "$PLAY" "$RESET"
        printf '  %s run -p hosted-amiga --emulator main.clo\n' "$cli"
        printf '  %s new my-game         %s# start another project%s\n' "$cli" "$DIM" "$RESET"
    elif [ -n "${TOOLCHAIN_ROOT:-}" ]; then
        printf '  %s new my-game         %s# create a starter Amiga project with VS Code tasks%s\n' "$cli" "$DIM" "$RESET"
    else
        printf '  %s setup --toolchain <rc24 root>\n' "$cli"
    fi
    if [ -n "${TOOLCHAIN_ROOT:-}" ]; then
        if [ "${GUEST_STATE:-}" = passed ]; then
            printf '\n%sWatch the Amiga hello again%s\n' "$BOLD" "$RESET"
        else
            printf '\n%sWatch the Amiga hello%s\n' "$BOLD" "$RESET"
        fi
        if [ -n "${KICKSTART_SELECTED:-}" ]; then
            printf '  %s run -p hosted-amiga --release --emulator %s\n' "$cli" \
                "$(pretty_path "${HELLO_GUEST_FILE:-$PREFIX/share/clorraine/examples/hello-amiga.clo}")"
        else
            printf '  %s run -p hosted-amiga --release --emulator --kickstart <ks31-a1200.rom> %s\n' "$cli" \
                "$(pretty_path "${HELLO_GUEST_FILE:-$PREFIX/share/clorraine/examples/hello-amiga.clo}")"
            printf '  %s(or remember the ROM: %s setup --toolchain %s --kickstart <ks31-a1200.rom>)%s\n' \
                "$DIM" "$cli" "$(pretty_path "$TOOLCHAIN_ROOT")" "$RESET"
        fi
    fi
    if [ "${VSCODE_STATE:-}" = available ]; then
        printf '\nInstall the VS Code extension with: %s --vscode (rerun the installer)\n' "$0"
    fi
    case ":$PATH:" in
        *":$PREFIX/bin:"*) ;;
        *)
            if [ "${MODIFY_PATH:-0}" = 1 ]; then
                printf '\n%sOpen a new terminal (or run: . ~/.profile) so clorraine is on your PATH.%s\n' "$YELLOW" "$RESET"
            fi
            ;;
    esac
    printf '\n  %sDocs and examples: https://github.com/OpenLorraine/clorraine%s\n\n' "$DIM" "$RESET"
}

# Selects an existing rc24 root or downloads the pinned one. Sets
# TOOLCHAIN_ROOT; configuration happens once the emulator is known.
toolchain_first_run() {
    local answer root
    if [ -n "$TOOLCHAIN" ]; then
        TOOLCHAIN_ROOT=$TOOLCHAIN
        return 0
    fi
    if [ -n "${CLORRAINE_TOOLCHAIN:-}" ]; then
        TOOLCHAIN_ROOT=$CLORRAINE_TOOLCHAIN
        return 0
    fi
    if [ -d "$TOOLCHAIN_DIR" ]; then
        if [ -t 0 ] && [ -t 1 ]; then
            confirm "  Use the rc24 toolchain at $(pretty_path "$TOOLCHAIN_DIR")? [Y/n] " yes || return 0
        fi
        TOOLCHAIN_ROOT=$TOOLCHAIN_DIR
        return 0
    fi
    if [ ! -t 0 ] || [ ! -t 1 ]; then
        printf 'no toolchain selected; run: %s/bin/clorraine setup --toolchain <rc24 root>\n' "$PREFIX"
        return 0
    fi
    printf '  Toolchain: path to an existing rc24 root (leave empty to download): '
    read -r answer || answer=
    if [ -n "$answer" ]; then
        TOOLCHAIN_ROOT=$answer
        return 0
    fi
    if confirm "  Download rc24 from AmigaPorts (70 MB)? [Y/n] " yes; then
        root=$(download_toolchain) || return 1
        TOOLCHAIN_ROOT=$root
        return 0
    fi
    printf 'skipped toolchain; run: %s/bin/clorraine setup --toolchain <rc24 root>\n' "$PREFIX"
    return 0
}

# Prints the remaining manual steps after a failed first-run step.
print_next_steps() {
    printf 'Next steps:\n'
    if [ -z "${TOOLCHAIN_CONFIGURED:-}" ]; then
        printf '  %s/bin/clorraine setup --toolchain <rc24 root>\n' "$PREFIX"
    fi
    printf '  Host hello: %s/bin/clorraine run %s\n' "$PREFIX" \
        "${HELLO_FILE:-$PREFIX/share/clorraine/examples/hello.clo}"
    printf '  Amiga hello: %s/bin/clorraine run -p hosted-amiga --release --emulator --fs-uae fs-uae --kickstart <ks31-a1200.rom> %s\n' \
        "$PREFIX" "${HELLO_GUEST_FILE:-$PREFIX/share/clorraine/examples/hello-amiga.clo}"
    printf '  VS Code extension: code --install-extension %s\n' "$EXTENSION_ID"
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
    PREFIX_SET=0
    MODIFY_PATH=0
    MODIFY_PATH_SET=0
    TOOLCHAIN=
    NO_TOOLCHAIN=0
    TOOLCHAIN_CONFIGURED=
    TOOLCHAIN_ROOT=
    TOOLCHAIN_DIR="$(toolchain_data_dir)/rc24"
    FS_UAE=
    KICKSTART=
    NO_EMULATOR=0
    NO_HELLO=0
    STARTER=
    NO_STARTER=0
    STARTER_DIR=
    STARTER_OPENED=
    SETUP_OUTPUT=
    VSCODE=0
    VSCODE_SET=0
    NO_VSCODE=0
    VSCODE_STATE=
    HOST_STATE=skipped
    GUEST_STATE=skipped
    HELLO_FILE=
    HELLO_GUEST_FILE=
    FS_UAE_SELECTED=
    KICKSTART_SELECTED=
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
                PREFIX_SET=1
                shift 2
                ;;
            --modify-path)
                MODIFY_PATH=1
                MODIFY_PATH_SET=1
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
            --toolchain)
                [ "$#" -ge 2 ] || usage_error "--toolchain needs a value"
                TOOLCHAIN=$2
                shift 2
                ;;
            --no-toolchain)
                NO_TOOLCHAIN=1
                shift
                ;;
            --fs-uae)
                [ "$#" -ge 2 ] || usage_error "--fs-uae needs a value"
                FS_UAE=$2
                shift 2
                ;;
            --kickstart)
                [ "$#" -ge 2 ] || usage_error "--kickstart needs a value"
                KICKSTART=$2
                shift 2
                ;;
            --no-emulator)
                NO_EMULATOR=1
                shift
                ;;
            --no-hello)
                NO_HELLO=1
                shift
                ;;
            --starter)
                [ "$#" -ge 2 ] || usage_error "--starter needs a value"
                STARTER=$2
                shift 2
                ;;
            --no-starter)
                NO_STARTER=1
                shift
                ;;
            --vscode)
                VSCODE=1
                VSCODE_SET=1
                shift
                ;;
            --no-vscode)
                NO_VSCODE=1
                VSCODE_SET=1
                shift
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
    if [ "$VSCODE" = 1 ] && [ "$NO_VSCODE" = 1 ]; then
        usage_error "--vscode and --no-vscode cannot be combined"
    fi
    if [ "$NO_TOOLCHAIN" = 1 ] && { [ -n "$FS_UAE" ] || [ -n "$KICKSTART" ]; }; then
        usage_error "--fs-uae and --kickstart require the toolchain"
    fi
    if [ -n "$STARTER" ] && { [ "$NO_STARTER" = 1 ] || [ "$NO_TOOLCHAIN" = 1 ]; }; then
        usage_error "--starter cannot be combined with --no-starter or --no-toolchain"
    fi
    if [ "$UNINSTALL" = 1 ] && { [ -n "$ARCHIVE" ] || [ -n "$CHECKSUMS" ] || [ "$MODIFY_PATH" = 1 ] || [ -n "$TOOLCHAIN" ] || [ "$NO_TOOLCHAIN" = 1 ] || [ -n "$FS_UAE" ] || [ -n "$KICKSTART" ] || [ "$NO_EMULATOR" = 1 ] || [ "$VSCODE_SET" = 1 ] || [ "$NO_HELLO" = 1 ] || [ -n "$STARTER" ] || [ "$NO_STARTER" = 1 ]; }; then
        usage_error "--uninstall cannot be combined with install options"
    fi
    if { [ -n "$ARCHIVE" ] && [ -z "$CHECKSUMS" ]; } || { [ -z "$ARCHIVE" ] && [ -n "$CHECKSUMS" ]; }; then
        usage_error "--archive and --checksums must be used together"
    fi

    require_commands uname tar gzip sha256sum getconf mktemp awk sed grep find cat \
        wc basename dirname sort ln mv rm mkdir cp chmod readlink sleep tr
    check_host

    if [ "$UNINSTALL" = 1 ]; then
        check_prefix
        acquire_lock
        trap 'release_lock; cleanup_temp' EXIT
        uninstall
        [ "$PURGE" = 1 ] && purge
        release_lock
        cleanup_temp
        trap - EXIT
        return 0
    fi

    init_style
    if [ -t 0 ] && [ -t 1 ]; then
        banner
        step 1 "Install the Clorraine CLI"
        printf '  Detected %s %s, %s.\n' "$(uname -s)" "$(uname -m)" "$libc"
        if [ "$PREFIX_SET" = 0 ]; then
            printf '  Install prefix [%s]: ' "$PREFIX"
            read -r answer || answer=
            [ -n "$answer" ] && PREFIX=$answer
        fi
        if [ "$MODIFY_PATH_SET" = 0 ]; then
            if confirm "  Add $PREFIX/bin to PATH in $HOME/.profile? [Y/n] " yes; then
                MODIFY_PATH=1
            else
                MODIFY_PATH=0
            fi
        fi
        if ! confirm "  Install Clorraine $VERSION into $PREFIX? [Y/n] " yes; then
            printf 'install cancelled\n'
            exit 0
        fi
    fi

    check_prefix

    if [ -n "$ARCHIVE" ]; then
        verify_local_archive "$ARCHIVE" "$CHECKSUMS"
        # Verify and use the same private copy; never re-open a user path for
        # extraction after the digest check.
        verified_dir=$(mktemp -d "${TMPDIR:-/tmp}/clorraine-verified.XXXXXXXX") ||
            die "cannot create private archive storage"
        archive="$verified_dir/$(basename "$ARCHIVE")"
        cp "$ARCHIVE" "$archive" || die "cannot copy the archive into private storage"
        verify_local_archive "$archive" "$CHECKSUMS" quiet
    else
        download_archive
        archive=$downloaded
    fi

    acquire_lock
    trap 'rollback "$PREFIX/share/clorraine"; rm -rf "$PREFIX/share/clorraine/.tmp.$$"; release_lock; cleanup_temp' EXIT
    install_archive "$archive"
    install_examples || die "cannot write the hello examples"
    if [ "$NO_TOOLCHAIN" = 0 ]; then
        interactive && step 2 "Amiga toolchain"
        if ! toolchain_first_run; then
            print_next_steps
            exit 1
        fi
        if [ -n "${TOOLCHAIN_ROOT:-}" ]; then
            interactive && step 3 "Amiga emulator"
            select_emulator
            if ! configure_toolchain "$TOOLCHAIN_ROOT"; then
                print_next_steps
                exit 1
            fi
            TOOLCHAIN_CONFIGURED=1
            [ "$NO_HELLO" = 0 ] && interactive && step 4 "Say hello"
            run_first_hello || {
                print_next_steps
                exit 1
            }
        fi
    fi
    interactive && step 5 "Your first Amiga project"
    install_vscode
    # A starter named with --starter is created even without a toolchain;
    # `new` does not need one. The interactive offer needs a working setup.
    if [ -n "${TOOLCHAIN_CONFIGURED:-}" ] || [ -n "$STARTER" ]; then
        offer_starter
    fi
    print_summary
    release_lock
    cleanup_temp
    trap - EXIT
}

# Sourced implementation tests skip the command-line entry point.
case "${CLORRAINE_INSTALL_LIB:-0}" in
    1) ;;
    *) main "$@" ;;
esac
