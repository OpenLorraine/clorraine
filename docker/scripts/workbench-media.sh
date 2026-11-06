#!/usr/bin/env bash
# Read-only Workbench media staging for the Bundle 9 gates.
#
# The licensed ADFs stay under /amiga. Each helper copies only the named
# trees into a clean build/ directory; no media is written or committed.
set -euo pipefail

wb_stage() {
    local adf="$1"
    local dest="$2"
    local repo_dir
    repo_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
    local dest_rel="${dest#"${repo_dir}/"}"
    local docker_dir="${repo_dir}/docker"

    rm -rf "${dest}"
    mkdir -p "${dest}"
    (
        cd "${docker_dir}"
        # shellcheck disable=SC1090
        set -a
        . ./.env
        set +a
        docker compose -p clorraine --env-file .env -f compose.yaml run --rm -T dev \
            xdftool -r "${adf}" unpack "/work/${dest_rel}" >/dev/null
    )
    if [ -d "${dest}/Libs" ] || [ -d "${dest}/libs" ] || \
       [ -d "${dest}/C" ] || [ -d "${dest}/c" ]; then
        printf '%s\n' "${dest}"
        return 0
    fi
    local volume
    volume="$(find "${dest}" -mindepth 1 -maxdepth 1 -type d | head -1)"
    if [ -z "${volume}" ] || { [ ! -d "${volume}/Libs" ] && \
       [ ! -d "${volume}/libs" ] && [ ! -d "${volume}/C" ] && \
       [ ! -d "${volume}/c" ]; }; then
        echo "wb_stage: no Workbench volume in ${dest}" >&2
        return 1
    fi
    printf '%s\n' "${volume}"
}


# Unpack any AmigaOS volume (Workbench, Locale, Fonts) and return its root.
wb_stage_any() {
    local adf="$1"
    local dest="$2"
    local repo_dir
    repo_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
    local dest_rel="${dest#"${repo_dir}/"}"
    local docker_dir="${repo_dir}/docker"
    rm -rf "${dest}"
    mkdir -p "${dest}"
    (
        cd "${docker_dir}"
        # shellcheck disable=SC1090
        set -a
        . ./.env
        set +a
        docker compose -p clorraine --env-file .env -f compose.yaml run --rm -T dev \
            xdftool -r "${adf}" unpack "/work/${dest_rel}" >/dev/null
    )
    if find "${dest}" -mindepth 1 -maxdepth 1 -type d | grep -qv '^$'; then
        # A volume such as Locale unpacks directly into dest; a Workbench
        # volume may be nested one level under its volume name.
        local nested
        nested="$(find "${dest}" -mindepth 1 -maxdepth 1 -type d | head -1)"
        if [ -d "${nested}/Libs" ] || [ -d "${nested}/libs" ] ||
           [ -d "${nested}/C" ] || [ -d "${nested}/c" ]; then
            printf '%s\n' "${nested}"
        else
            printf '%s\n' "${dest}"
        fi
    else
        printf '%s\n' "${dest}"
    fi
}

wb_copy() {
    local stage="$1"
    local hd="$2"
    shift 2
    local tree
    for tree in "$@"; do
        if [ -d "${stage}/${tree}" ]; then
            mkdir -p "${hd}/${tree}"
            cp -a "${stage}/${tree}/." "${hd}/${tree}/"
        fi
    done
}

wb_dirs() {
    local hd="$1"
    mkdir -p "${hd}/Libs" "${hd}/Devs" "${hd}/Classes" "${hd}/Fonts" \
        "${hd}/Locale" "${hd}/C" "${hd}/env" "${hd}/envarc" "${hd}/rexx" \
        "${hd}/clips"
}

wb_assigns() {
    local hd="$1"
    cat <<EOF
CNEXT:C/Assign LIBS: CNEXT:Libs ADD CNEXT:Classes
CNEXT:C/Assign DEVS: CNEXT:Devs
CNEXT:C/Assign FONTS: CNEXT:Fonts
CNEXT:C/Assign LOCALE: CNEXT:Locale
CNEXT:C/Assign ENV: CNEXT:env
CNEXT:C/Assign ENVARC: CNEXT:envarc
CNEXT:C/Assign REXX: CNEXT:rexx
CNEXT:C/Assign CLIPS: CNEXT:clips
EOF
}

wb_datatypes() {
    local hd="$1"
    cat <<EOF
CNEXT:C/AddDataTypes REFRESH QUIET
EOF
}

wb_cleanup() {
    local work="$1"
    rm -rf "${work}"
}
