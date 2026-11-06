#!/usr/bin/env bash
# A1200/Kickstart 3.1 Picasso96 UAEGFX smoke.
#
# Thin entry point for the shared RTG gate; see rtg-smoke.sh.
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec "${script_dir}/rtg-smoke.sh" uaegfx "$@"
