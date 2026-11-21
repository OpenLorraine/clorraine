#!/bin/sh
# Prepare maintainer-only Python interpreters and wheel build backends offline.
#
# Run as the image's unprivileged builder after installing the verified Ubuntu
# package set. Inputs are read-only at /sources/archives/<source-id>; the image
# owns /build and /opt/clorraine-build. No source acquisition is performed here.
# This recipe does not create redistributable support archives.
set -eu

if [ "$#" -ne 0 ]; then
    echo "build-environment: this fixed image recipe takes no arguments" >&2
    exit 2
fi
if [ "$(id -u)" -eq 0 ]; then
    echo "build-environment: source builds require an unprivileged builder" >&2
    exit 2
fi

export LC_ALL=C
export PIP_CONFIG_FILE=/dev/null
export PIP_NO_INDEX=1
export PIP_DISABLE_PIP_VERSION_CHECK=1
unset PYTHONHOME PYTHONPATH
umask 022

prefix=/opt/clorraine-build
work=/build
archives=/sources/archives

# Check every source used by this image before extracting or executing any of
# them. The same source records must appear in the final support lock.
cd "$archives"
sha256sum --check <<'CHECKSUMS'
b5796915dc45ff745cfb4e990a7e7c93d6f546328a102ce31448d69ebc37575c  cpython312
18f63100d6f94385c6ed57a72073443e1a71a4acb4339491615d0f16d6ff01b2  flit-core
c228a6dc5e932d346bc5739379109d49e8853dd8223571c7c5b55260edc0b97f  packaging
d46d457a85337051c36524bc5349dd91b1877838e2979ac5ced3e710ed8a60ed  tomli
661e1abd9198507b1409a20c02106d9670b2576e916d58f520316666abca6729  wheel
583b361c8da8de57403743e756609670de6fb2345920e36dc5c2d914c319c945  setuptools
3d555e92b75dacd037d32bafdf94f97af51ea29ae8c7b234cf94b7a5bd242a63  setuptools-scm
b988bb297ce76c671e28c97d017b95411010f7c77fa6623dd0bb47eed1aee1bc  cython
f6ad667e89a1fe78046c8f13232b247200f5258d7828f3f7883d660878e0813f  pip
CHECKSUMS

# Fresh directories make a previous partial build an error rather than a cache
# that could conceal changed inputs. The caller discards failed image builds.
mkdir "$work/cpython312"
mkdir "$work/python-sources"
mkdir "$prefix/python312"
mkdir "$prefix/venvs"
tar --extract --xz --file "$archives/cpython312" --directory "$work/cpython312" \
    --strip-components=1 --no-same-owner --no-same-permissions
cd "$work/cpython312"
# The interpreter stays inside the build image. Users receive wheels, not this
# interpreter. Its source-bundled ensurepip wheel is a recorded bootstrap input.
./configure --prefix="$prefix/python312" --with-ensurepip=install
make -j2
make install
"$prefix/python312/bin/python3.12" -I -c \
    'import bz2, ctypes, lzma, sqlite3, ssl, venv, zlib; import sys; assert sys.version_info[:3] == (3, 12, 16)'

# The order below follows the source-declared backend dependencies. Flit-core
# and setuptools provide in-tree bootstrap backends. The older Ubuntu venv pip
# can build those sources without downloading an isolated backend environment.
for abi in cp310 cp312; do
    case "$abi" in
        cp310) interpreter=/usr/bin/python3.10 ;;
        cp312) interpreter="$prefix/python312/bin/python3.12" ;;
    esac
    "$interpreter" -I -m venv "$prefix/venvs/$abi"
    python="$prefix/venvs/$abi/bin/python"
    "$python" -I -m pip --isolated --disable-pip-version-check --version
    mkdir "$work/wheels-$abi"

    for source in flit-core packaging tomli wheel setuptools setuptools-scm cython pip; do
        destination="$work/python-sources/$abi-$source"
        mkdir "$destination"
        tar --extract --file "$archives/$source" --directory "$destination" \
            --strip-components=1 --no-same-owner --no-same-permissions
        wheel_output="$work/wheels-$abi/$source"
        mkdir "$wheel_output"
        "$python" -I -m pip --isolated --disable-pip-version-check --no-cache-dir wheel \
            --no-index --no-deps --no-build-isolation --use-pep517 \
            --wheel-dir "$wheel_output" "$destination"
        # Each invocation builds one project. Requiring one wheel avoids
        # silently installing an unexpected dependency or ambiguous artifact.
        set -- "$wheel_output"/*.whl
        if [ "$#" -ne 1 ] || [ ! -f "$1" ]; then
            echo "build-environment: expected one wheel for $abi/$source" >&2
            exit 1
        fi
        "$python" -I -m pip --isolated --disable-pip-version-check --no-cache-dir install \
            --no-index --no-deps --force-reinstall "$1"
    done
    "$python" -I -m pip --isolated --disable-pip-version-check check
    "$python" -I -m pip --isolated --disable-pip-version-check list --format=json
done

# Retain explicit bootstrap and final-tool measurements in the image. These
# supplement, rather than replace, the final lock and executed-image checks.
dpkg-query -W -f='${binary:Package}\t${Version}\n' > "$prefix/ubuntu-packages.txt"
/usr/bin/python3.10 -I -m venv "$work/ubuntu-bootstrap"
"$work/ubuntu-bootstrap/bin/python" -I -m pip --version > "$prefix/ubuntu-bootstrap.txt"
"$work/ubuntu-bootstrap/bin/python" -I -c \
    'from importlib.metadata import version; print("setuptools " + version("setuptools"))' \
    >> "$prefix/ubuntu-bootstrap.txt"
"$prefix/python312/bin/python3.12" -I -m pip --version > "$prefix/cpython-bootstrap.txt"
