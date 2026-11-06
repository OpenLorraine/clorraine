#!/usr/bin/env bash
# A4000/Kickstart 3.1 emulated Picasso IV RTG smoke.
#
# Thin entry point for the shared RTG gate; see rtg-smoke.sh.
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec "${script_dir}/rtg-smoke.sh" picasso-iv "$@"
