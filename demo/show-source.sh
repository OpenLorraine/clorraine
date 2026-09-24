#!/usr/bin/env bash
#
# Open a Clorraine source file read-only in Vim with Clorraine syntax
# highlighting, line numbers, and a bright color scheme. It is what the demo
# recording uses to show the source, and it works on its own too.
#
# Usage: ./demo/show-source.sh FILE.clo

set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

exec vim -R \
    -c 'syntax on' \
    -c "source ${here}/clorraine.vim" \
    -c 'set number' \
    -c 'colorscheme desert' \
    "$@"
