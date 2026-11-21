#!/bin/sh
# Only the declared ABI is accepted; the recipe suffix is constructed internally.
set -eu
script_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
exec python3 -I -c '
import sys
sys.path.insert(0, sys.argv.pop(1))
import support_build
raise SystemExit(support_build.main("wheels"))
' "$script_dir" "$@"
