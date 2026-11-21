#!/bin/sh
# Build through the proved fixed-input reference recipe, then select the C
# toolchain candidate. The private reference contains licensed NDK data and
# must never be copied wholesale into the payload.
set -eu
if [ "$(id -u)" -eq 0 ] || [ "$(pwd -P)" != /work ]; then
    echo "support-payload: require a non-root builder in /work" >&2
    exit 2
fi
if [ "$#" -eq 2 ] && [ "$1" = --wheels ]; then
    case "$2" in
        cp310|cp312)
            exec "/opt/clorraine-build/venvs/$2/bin/python" -I \
                /sources/inputs/wheel-payload.py --abi "$2"
            ;;
    esac
fi
if [ "$#" -ne 0 ]; then
    echo "support-payload: require no arguments or --wheels cp310|cp312" >&2
    exit 2
fi
if [ -e /work/payload ] || [ -L /work/payload ]; then
    echo "support-payload: refusing an existing payload" >&2
    exit 2
fi
/bin/sh /sources/inputs/reference-support.sh

python3 -I - <<'SELECT'
"""Select fixed C-toolchain/helper outputs; leave final source/native audits separate."""

import os
from pathlib import Path
import shutil
import stat

source = Path("/work/reference/sdk")
destination = Path("/work/payload")
if source.is_symlink() or not source.is_dir():
    raise SystemExit("support-payload: missing ordinary SDK reference")
destination.mkdir()

# These executables are C compilation/linking tools or locally required NDK
# preparation helpers. C++/Objective-C front ends, debuggers, coverage tools,
# plugin-development tools and installation-time header fixers are omitted.
programs = {
    "fd2sfd", "lha", "sfdc",
    "m68k-amigaos-gcc", "m68k-amigaos-cpp",
    "m68k-amigaos-gcc-ar", "m68k-amigaos-gcc-nm", "m68k-amigaos-gcc-ranlib",
    "m68k-amigaos-ar", "m68k-amigaos-as", "m68k-amigaos-ld", "m68k-amigaos-ld.bfd",
    "m68k-amigaos-nm", "m68k-amigaos-objcopy", "m68k-amigaos-objdump",
    "m68k-amigaos-ranlib", "m68k-amigaos-readelf", "m68k-amigaos-size",
    "m68k-amigaos-strings", "m68k-amigaos-strip",
}
gcc = "lib/gcc/m68k-amigaos/16.2.0b/"
libexec = "libexec/gcc/m68k-amigaos/16.2.0b/"


def selected(relative):
    """Use reviewed install producers, never the presence of an SDK directory alone."""
    if relative.endswith("/libamiga.a") and relative != "m68k-amigaos/lib/libamiga.a":
        return False
    if relative.startswith("bin/"):
        return relative[4:] in programs
    if relative.startswith(libexec):
        return relative[len(libexec):] in {
            "cc1", "collect2", "lto-wrapper", "lto1", "liblto_plugin.so",
        }
    if relative.startswith(gcc):
        local = relative[len(gcc):]
        if local.startswith("include/"):
            return not local.startswith(("include/c++/", "include/objc/"))
        if local.startswith("include-fixed/"):
            return True
        return Path(local).name in {
            "libgcc.a", "libgcov.a", "libatomic.a", "libatomic_asneeded.a",
        }
    # The upstream install rules trace these to binutils, newlib, libnix,
    # pthread and fd2sfd sources. The prepared NDK trees and NDK link inputs
    # are deliberately not selected; later content/origin audit is still
    # required for every candidate file, including independently overlaid data.
    return relative.startswith((
        "m68k-amigaos/bin/", "m68k-amigaos/lib/",
        "m68k-amigaos/libnix/", "m68k-amigaos/include/",
        "m68k-amigaos/sys-include/", "share/fd2sfd/",
    )) and relative != "m68k-amigaos/sys-include/COPIED"


def unreadable(error):
    """An incomplete directory traversal cannot produce a valid candidate."""
    raise error


for directory, folders, files in os.walk(source, followlinks=False, onerror=unreadable):
    for name in sorted(folders + files):
        path = Path(directory) / name
        relative = path.relative_to(source).as_posix()
        mode = path.lstat().st_mode
        if stat.S_ISDIR(mode) or not selected(relative):
            continue
        output = destination / relative
        output.parent.mkdir(parents=True, exist_ok=True)
        if stat.S_ISLNK(mode):
            output.symlink_to(os.readlink(path))
        elif stat.S_ISREG(mode):
            shutil.copy2(path, output)
        else:
            raise SystemExit(f"support-payload: unsupported reference entry: {relative}")

for name in programs:
    if not (destination / "bin" / name).is_file():
        raise SystemExit(f"support-payload: required program missing: {name}")
for name in ("libnix.a", "libnix13.a", "libnix20.a", "ncrt0.o"):
    if not (destination / "m68k-amigaos/libnix/lib" / name).is_file():
        raise SystemExit(f"support-payload: required target runtime missing: {name}")
print("support-payload: C-toolchain candidate selected; source/notice/native audits required")
SELECT
