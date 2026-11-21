#!/bin/sh
# Release-specific bootstrap template. Packaging substitutes the three pinned
# helper values; the Python application owns public argument/release selection.
set -eu
umask 077

HELPER_URL='@CLORRAINE_HELPER_URL@'
HELPER_SHA256='@CLORRAINE_HELPER_SHA256@'
HELPER_BYTES='@CLORRAINE_HELPER_BYTES@'

# Keep helper authentication outside Python: unverified application bytes must
# never reach an interpreter, including for malformed public arguments.
for tool in curl head sha256sum wc mktemp rm rmdir python3; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        printf '%s\n' 'clorraine: missing bootstrap tool; install curl, ca-certificates, coreutils and python3, then retry' >&2
        exit 1
    fi
done

case "$HELPER_BYTES" in
    ''|*[!0-9]*)
        printf '%s\n' 'clorraine: this is an unassembled installer template; use a release install.sh' >&2
        exit 1
        ;;
esac
if [ "$HELPER_BYTES" -le 0 ]; then
    printf '%s\n' 'clorraine: invalid pinned helper size' >&2
    exit 1
fi

if ! work=$(mktemp -d "${TMPDIR:-/tmp}/clorraine-bootstrap.XXXXXXXX" 2>/dev/null); then
    printf '%s\n' 'clorraine: cannot create private bootstrap storage; check TMPDIR permissions and free space' >&2
    exit 1
fi

# Remove only this invocation's known files. Never recursively remove a
# caller-selected root or another installer invocation's temporary directory.
cleanup() {
    rm -f -- "$work/install.pyz" "$work/curl.stderr" "$work/curl.ok" 2>/dev/null || :
    rmdir -- "$work" 2>/dev/null || :
}
trap cleanup 0
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM

# --disable is first so a user's curlrc cannot disable certificate checks.
# Both initial and redirected requests remain HTTPS, with finite deadlines.
# Older curl versions check --max-filesize against response headers only.
# Limit only helper bytes, not every file curl might use for diagnostics.
# One extra byte makes an oversized response fail the subsequent length check.
# POSIX pipelines do not preserve the first command's status, so curl alone
# creates a success marker after a successful transfer; an EOF is not success.
helper_limit=$(( HELPER_BYTES + 1 ))
if ! (
    if curl --disable --silent --show-error --fail --location \
        --proto '=https' --proto-redir '=https' \
        --connect-timeout 15 --max-time 300 --max-filesize "$HELPER_BYTES" \
        --output - "$HELPER_URL" 2>"$work/curl.stderr"; then
        printf '%s\n' ok >"$work/curl.ok"
    fi
) | head -c "$helper_limit" >"$work/install.pyz"; then
    printf '%s\n' 'clorraine: cannot store the bounded helper download; check temporary storage' >&2
    exit 1
fi
if [ ! -f "$work/curl.ok" ]; then
    printf '%s\n' 'clorraine: helper download failed; check HTTPS access, curl and CA certificates, then retry' >&2
    exit 1
fi

actual_bytes=$(wc -c <"$work/install.pyz")
if [ "$actual_bytes" -ne "$HELPER_BYTES" ]; then
    printf '%s\n' 'clorraine: helper byte length mismatch; retry the release download' >&2
    exit 1
fi
# Hash stdin rather than a filename: sha256sum's filename-escaping convention
# must not make otherwise valid temporary paths affect the digest comparison.
actual_hash=$(sha256sum <"$work/install.pyz")
actual_hash=${actual_hash%% *}
if [ "$actual_hash" != "$HELPER_SHA256" ]; then
    printf '%s\n' 'clorraine: helper SHA-256 mismatch; do not run the downloaded helper' >&2
    exit 1
fi

# The normal exit status, including usage failures, propagates through the exit
# trap. Do not exec: the shell must retain ownership of its private helper copy.
python3 -I "$work/install.pyz" "$@"
