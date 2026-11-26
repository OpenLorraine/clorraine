#!/usr/bin/env bash
# Copy files to a Windows test host and run commands in its desktop session.
#
#   tools/setup/windows-remote.sh push HOST DIR
#       Copy DIR to %USERPROFILE%\clorraine-remote\<name of DIR>, replacing an
#       earlier copy. Kickstart ROMs and ROM keys are refused: they stay on the
#       machine that owns them.
#   tools/setup/windows-remote.sh run HOST [--timeout SECONDS] -- COMMAND...
#       Run COMMAND as one cmd.exe line in the signed-in desktop session of
#       HOST (working directory %USERPROFILE%\clorraine-remote), print its
#       output, and exit with its exit code (124 on timeout).
#
# HOST is an ssh destination whose login shell is Windows PowerShell, with
# key-based authentication. This helper is a convenience for development and
# acceptance runs; no Makefile gate calls it.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ssh_opts=(-o BatchMode=yes -o WarnWeakCrypto=no -o ConnectTimeout=15)

usage() {
    sed -n '4,11p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//' >&2
    exit 2
}

# Run one PowerShell command line on HOST.
remote() {
    local host="$1"
    shift
    ssh "${ssh_opts[@]}" "$host" "$@"
}

push() {
    [ $# -eq 2 ] || usage
    local host="$1" dir="${2%/}" name
    [ -d "$dir" ] || { echo "windows-remote: no directory $dir" >&2; exit 2; }
    name="$(basename "$dir")"
    # The name becomes part of a remote command line, so only plain names pass.
    [[ "$name" =~ ^[A-Za-z0-9._-]+$ ]] || {
        echo "windows-remote: the directory name '$name' must use only letters, digits, '.', '_', '-'" >&2
        exit 2
    }
    if find "$dir" -type f \( -iname '*.rom' -o -iname 'rom.key' \) | grep -q .; then
        echo "windows-remote: refusing to copy Kickstart ROMs or ROM keys from $dir" >&2
        exit 2
    fi
    # rmdir /s removes a junction without following it into its target.
    remote "$host" "New-Item -ItemType Directory -Force -Path clorraine-remote | Out-Null; cmd /c 'if exist clorraine-remote\\$name rmdir /s /q clorraine-remote\\$name'"
    scp -q -r "${ssh_opts[@]}" "$dir" "$host:clorraine-remote/$name"
    echo "windows-remote: copied $dir to $host:%USERPROFILE%\\clorraine-remote\\$name"
}

run() {
    [ $# -ge 1 ] || usage
    local host="$1" timeout=300 line b64
    shift
    while [ $# -gt 0 ]; do
        case "$1" in
            --timeout)
                [ $# -ge 2 ] && [[ "$2" =~ ^[0-9]+$ ]] || usage
                timeout="$2"
                shift 2
                ;;
            --) shift; break ;;
            *) usage ;;
        esac
    done
    [ $# -ge 1 ] || usage
    line="$*"
    b64="$(printf '%s' "$line" | base64 -w0)"
    remote "$host" "New-Item -ItemType Directory -Force -Path clorraine-remote | Out-Null"
    scp -q "${ssh_opts[@]}" "$here/windows-desktop-run.ps1" "$host:clorraine-remote/windows-desktop-run.ps1"
    # The login shell reports only success or failure, so pass the exact code on.
    remote "$host" "powershell -NoProfile -ExecutionPolicy Bypass -File clorraine-remote\\windows-desktop-run.ps1 -CommandBase64 $b64 -TimeoutSeconds $timeout; exit \$LASTEXITCODE"
}

[ $# -ge 1 ] || usage
command="$1"
shift
case "$command" in
    push) push "$@" ;;
    run) run "$@" ;;
    *) usage ;;
esac
