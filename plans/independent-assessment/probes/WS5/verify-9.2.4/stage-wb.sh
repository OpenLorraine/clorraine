#!/usr/bin/env bash
set -euo pipefail
v=build/tmp/assessment/verify
rm -rf $v/media/wb31; mkdir -p $v/media/wb31
xdftool -r /amiga/adf/amiga-os-310-workbench.adf unpack $v/media/wb31 >/dev/null
ls $v/media/wb31
