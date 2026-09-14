#!/usr/bin/env bash
# Launch FS-UAE in the SDK container with the window shown on the host.
#
# Usage:
#   scripts/emulator.sh [fs-uae arguments...]
#
# Arguments are appended after FS_UAE_ARGS from docker/.env, so they win.
# See docker/examples/default.fs-uae for a template config.
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
docker_dir="$(dirname "${script_dir}")"
env_file="${docker_dir}/.env"

if [ ! -f "${env_file}" ]; then
    "${script_dir}/init-env.sh"
fi

set -a
# shellcheck disable=SC1090
. "${env_file}"
set +a

if [ ! -f "${XAUTHORITY}" ]; then
    echo "ERROR: X11 cookie file not found: ${XAUTHORITY}" >&2
    echo "The emulator window cannot authenticate with your X server." >&2
    echo "Fix XAUTHORITY in ${env_file}, or run 'xhost +local:' (less secure)." >&2
    exit 1
fi

cd "${docker_dir}"
# shellcheck disable=SC2086
exec docker compose -p clorraine --env-file .env -f compose.yaml run --rm emulator \
    fs-uae ${FS_UAE_ARGS:-} "$@"
