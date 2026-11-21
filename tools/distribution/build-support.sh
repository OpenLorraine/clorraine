#!/bin/sh
# A fixed mode and isolated interpreter share the same verified build boundary.
set -eu
script_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
exec python3 -I -c '
import sys
sys.path.insert(0, sys.argv.pop(1))
import support_build
raise SystemExit(support_build.main("support"))
' "$script_dir" "$@"
