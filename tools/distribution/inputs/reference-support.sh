#!/bin/sh
# Build a private full SDK reference from fixed staged sources, without fetching.
#
# Invoke with no arguments, as a non-root user, in an owned /work directory.
# Mount verified source staging read-only at /sources and disable networking.
# Successful output is /work/reference, including NDK-derived content: it is
# reference evidence, NOT a redistributable support payload or a final lock.
set -eu

if [ "$#" -ne 0 ]; then
    echo "reference-support: this fixed recipe takes no arguments" >&2
    exit 2
fi
if [ "$(id -u)" -eq 0 ] || [ "$(pwd -P)" != /work ]; then
    echo "reference-support: require a non-root builder in the owned /work directory" >&2
    exit 2
fi
if [ -e /work/reference ] || [ -L /work/reference ]; then
    echo "reference-support: refusing existing /work/reference" >&2
    exit 2
fi

export LC_ALL=C
export PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
export GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null GIT_TERMINAL_PROMPT=0
export GIT_OPTIONAL_LOCKS=0 GIT_ATTR_NOSYSTEM=1
unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_OBJECT_DIRECTORY GIT_ALTERNATE_OBJECT_DIRECTORIES
unset GIT_CONFIG_COUNT GIT_CONFIG_PARAMETERS MAKEFLAGS MFLAGS CONFIG_SITE
unset CPPFLAGS LDFLAGS GCC_EXEC_PREFIX COMPILER_PATH LIBRARY_PATH CPATH
unset C_INCLUDE_PATH CPLUS_INCLUDE_PATH LD_LIBRARY_PATH LD_PRELOAD
export CC=gcc CXX=g++ CFLAGS=-Os CXXFLAGS=-Os
export CFLAGS_FOR_TARGET='-O2 -fomit-frame-pointer'
export CXXFLAGS_FOR_TARGET='-O2 -fomit-frame-pointer -fno-exceptions -fno-rtti'
umask 022

# The fixed roster is shared by preflight checks and private source copying.
# It is not a branch selection or a dependency-resolution mechanism.
source_records() {
    cat <<'SOURCES'
amiga-gcc m68k-amigaos-gcc e7ed7a735c4dd9d2c47e23a50e51b2b7ddd4688c
gcc gcc 26e100b31d25392a21068483d4c541a4b55822c3
binutils binutils-gdb 486d13bafc19f15a65be861071bdee1381314f02
libnix libnix 0a1d18a0938dce3f7a234963bcb7fa6148f6a6f3
newlib-cygwin newlib-cygwin d8eff91f1916ee996c30f10e90984563cddc310f
sfdc sfdc 4e7afc044c1ff129574fcf3b839c3351edcfe2fb
fd2sfd fd2sfd 189ac23b5b2d0d0555381404d75e0508c2e95920
fd2pragma fd2pragma bcffac5c89295baf1db033addc984312b4b4a722
amiga-netinclude amiga-netinclude 7d155ca3a3090afe3466f819dd178f2af6f5c176
aros-stuff aros-stuff 86711c70569ffc6b70ee1fb99895866c1d9f30e4
lha lha 08486d2da2ca77a6adf83194fd67400c8d86e7c7
SOURCES
}

source_records | while read -r name repository commit; do
    checkout="/sources/checkouts/$name"
    if [ -L "$checkout" ] || [ -L "$checkout/.git" ] || [ ! -d "$checkout/.git" ]; then
        echo "reference-support: missing ordinary Git checkout: $name" >&2
        exit 1
    fi
    if [ "$(git -C "$checkout" rev-parse --show-object-format)" != sha1 ] \
        || [ "$(git -C "$checkout" rev-parse HEAD)" != "$commit" ] \
        || [ "$(git -C "$checkout" config --local --get remote.origin.url)" != \
            "https://github.com/AmigaPorts/$repository" ]; then
        echo "reference-support: source identity/origin mismatch: $name" >&2
        exit 1
    fi
    # A clean Git entry can still be a link to an undeclared external input.
    # Check the source tree, excluding Git's own administrative directory.
    python3 -I - "$checkout" <<'LINKS'
import os
from pathlib import Path
import sys

root = Path(sys.argv[1]).resolve(strict=True)
for directory, directories, files in os.walk(root, followlinks=False):
    if Path(directory) == root and ".git" in directories:
        directories.remove(".git")
    for name in directories + files:
        path = Path(directory) / name
        if not path.is_symlink():
            continue
        try:
            target = path.resolve(strict=True)
        except (OSError, RuntimeError) as error:
            raise SystemExit(f"reference-support: invalid source link: {path}") from error
        if target != root and root not in target.parents:
            raise SystemExit(f"reference-support: escaping source link: {path}")
LINKS
    # Older Git can report a false modification from cached checkout sizes
    # after ident-expanded text is restored to its raw blob bytes. Check the
    # index and actual content explicitly instead of trusting that stat cache.
    if ! git -C "$checkout" -c core.fsmonitor=false -c core.hooksPath=/dev/null \
        diff --cached --quiet --no-ext-diff --no-textconv HEAD -- \
        || ! git -C "$checkout" -c core.fsmonitor=false -c core.hooksPath=/dev/null \
        diff --quiet --no-ext-diff --no-textconv HEAD --; then
        echo "reference-support: changed source inputs: $name" >&2
        exit 1
    fi
    # Without exclude rules, this includes ignored inputs and empty extra
    # directories as well as ordinary untracked files.
    extras=$(git -C "$checkout" -c core.fsmonitor=false -c core.hooksPath=/dev/null \
        ls-files --others --directory)
    if [ -n "$extras" ]; then
        echo "reference-support: changed or extra source inputs: $name" >&2
        exit 1
    fi
done

# Keep the selected target archive byte-identical to its upstream input. It is
# supplied separately so the SDK payload does not depend on an implicit copy
# hidden inside an upstream Makefile rule.
if [ -L /sources/inputs/libamiga.a ] || [ ! -f /sources/inputs/libamiga.a ]; then
    echo "reference-support: missing ordinary libamiga.a input" >&2
    exit 1
fi
if [ "$(wc -c < /sources/inputs/libamiga.a)" -ne 297694 ]; then
    echo "reference-support: wrong libamiga.a input length" >&2
    exit 1
fi
printf '%s\n' \
    '10d78b6d303a99677c0bf29a8e1653d62669f69f395691646393020325084021  /sources/inputs/libamiga.a' \
    | sha256sum --check -

# These are the four archives consumed by the native upstream targets. Hashes
# are checked before any source build or archive extraction can take place.
cd /sources/archives
sha256sum --check <<'ARCHIVES'
eae9326beb4158c386e39a356818031bd28f3124cf915f8c5b1dc4c7a36b4d7c  gmp
feced2d430dd5a97805fa289fed3fc8ff2b094c02d05287fd6133e7f1f0ec926  mpfr
17503d2c395dfcf106b622dc142683c1199431d095367c6aacba6eec30340459  mpc
96cabd4ad683dced632e147bf86dee0f50dcb1254386216c25c362916a6409bb  ndk
ARCHIVES
cd /work

reference=/work/reference
mkdir "$reference"

# Failure discards only the newly owned reference tree. The caller captures
# bounded logs outside this directory and never treats partial work as success.
cleanup() {
    status=$?
    trap - 0
    if [ "$status" -ne 0 ]; then
        rm -rf -- "$reference"
    fi
    exit "$status"
}
trap cleanup 0
trap 'exit 130' INT
trap 'exit 143' TERM

root="$reference/amiga-gcc"
prefix="$reference/sdk"
cp -a /sources/checkouts/amiga-gcc "$root"
mkdir "$root/projects" "$root/download" "$prefix" "$prefix/bin"
mkdir -p "$prefix/m68k-amigaos/lib"
install -m 644 /sources/inputs/libamiga.a "$prefix/m68k-amigaos/lib/libamiga.a"
mkdir "$reference/tmp"
export TMPDIR="$reference/tmp"
source_records | while read -r name repository commit; do
    printf '%s %s\n' "$name" "$commit" >> "$reference/source-revisions.txt"
    case "$name" in
        amiga-gcc) ;;
        lha) cp -a "/sources/checkouts/$name" "$reference/lha" ;;
        *) cp -a "/sources/checkouts/$name" "$root/projects/$name" ;;
    esac
done

# Patch only the private compiler copy. Both complete-file hashes make a
# mismatched, repeated or offset-adjusted transformation a build failure.
(
    cd "$root/projects/gcc"
    printf '%s\n' \
        "5e926ce80a05eef4d2094ac6f64e9e76a590a42b27b6e266fd029be065c59213  gcc/gcc.cc" \
        | sha256sum --check -
    patch --batch --forward --fuzz=0 -p1 \
        --input=/sources/inputs/compiler-patches/gcc-find-file-specs.diff
    printf '%s\n' \
        "be312720a5feae41361762dec7d453c0914ee0bd93118baa48fb08d21aa5fc48  gcc/gcc.cc" \
        | sha256sum --check -
)
cp /sources/archives/gmp "$root/download/gmp-6.2.1.tar.bz2"
cp /sources/archives/mpfr "$root/download/mpfr-4.1.0.tar.bz2"
cp /sources/archives/mpc "$root/download/mpc-1.2.1.tar.gz"
cp /sources/archives/ndk "$root/download/NDK3.2.lha"

# Build the explicitly selected LHA first. Its upstream Makefile rule would
# otherwise clone a moving branch into the build directory. All writes here
# affect the private copy, never the staged source checkout.
cd "$reference/lha"
autoreconf -fi
./configure --prefix="$prefix"
make -j2 all
install -m 755 src/lha "$prefix/bin/lha"

cd "$root"
# Command-line assignments prevent inherited host/profile options from changing
# this native build. NDK 1.3 and netinclude targets retain the original header
# preparation paths used by the compiler's two Amiga runtime profiles.
# Host CFLAGS/CXXFLAGS stay exported above, not command-line overrides. GNU
# Make propagates overrides to recursive target builds, preventing libnix from
# appending its required header paths and replacing its target compiler flags.
make -j2 PREFIX="$prefix" HOST= TARGET=m68k-amigaos THREADS=no NDK=3.2 \
    CFLAGS_FOR_TARGET='-O2 -fomit-frame-pointer' \
    CXXFLAGS_FOR_TARGET='-O2 -fomit-frame-pointer -fno-exceptions -fno-rtti' \
    min fd2sfd fd2pragma sfdc ndk13 netinclude

for helper in lha sfdc fd2sfd fd2pragma m68k-amigaos-gcc m68k-amigaos-objdump; do
    test -x "$prefix/bin/$helper"
done
test -d "$prefix/m68k-amigaos/ndk-include"
test -d "$prefix/m68k-amigaos/ndk13-include"
printf '%s\n' \
    "10d78b6d303a99677c0bf29a8e1653d62669f69f395691646393020325084021  $prefix/m68k-amigaos/lib/libamiga.a" \
    | sha256sum --check -
"$prefix/bin/m68k-amigaos-gcc" -v > "$reference/compiler-version.txt" 2>&1
dpkg-query -W -f='${binary:Package}\t${Version}\n' > "$reference/ubuntu-packages.txt"
echo "reference-support: private SDK built; origin, licence, inventory and relocation audits still required"
