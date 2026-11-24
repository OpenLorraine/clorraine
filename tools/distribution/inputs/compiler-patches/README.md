# GCC specs filename adjustment

Clorraine distribution modification, 2026-11-18.

The original GCC revision remains
`26e100b31d25392a21068483d4c541a4b55822c3`. The complete original source and its
licences are retained. `gcc-find-file-specs.diff` changes only the return of
`find_file_spec_function` in `gcc/gcc.cc`, with an explanatory comment.
It uses GCC's existing specs quoting helper so the returned filename remains
literal when the specs engine parses it again. This is a Clorraine build
adjustment, not an upstream GCC release or a new licence grant.

The original `gcc/gcc.cc` SHA-256 is
`5e926ce80a05eef4d2094ac6f64e9e76a590a42b27b6e266fd029be065c59213`.
The fixed build recipe checks the original file hash before patching and the
result hash afterwards. A mismatched or repeated application must fail.
The staged original Git checkout is never modified.

The patched `gcc/gcc.cc` SHA-256 is
`be312720a5feae41361762dec7d453c0914ee0bd93118baa48fb08d21aa5fc48`.

## Reconstruct the modified GCC source

From a payload's `sources/` directory, when its GCC source archive is present:

```sh
tar -xf gcc.tar.gz
patch --batch --forward --fuzz=0 -p1 -d gcc \
    < build-inputs/compiler-patches/gcc-find-file-specs.diff
```

The patch and these instructions are build-control data in other support
assets too; their presence does not claim those assets contain a GCC binary
or its complete source snapshot.

## Rebuild context and commands

> **Historical context.** The `support.py`/`support_build.py` commands and the
> `build-inputs/` recipe were part of the removed native pipeline. This patch
> is retained as provenance; the current candidate path is `make dist` and
> `docs/linux-cli.md`.

The supplied original `amiga-gcc` source archive contains the build orchestration.
Use the locked Ubuntu 22.04 environment recipe, original dependency sources,
component versions and fixed build settings in `build-inputs/`.
The user-supplied NDK archive remains required for the SDK build and is not
redistributed in these sources.

For the exact automated build, authenticate the original staged checkouts with
the supplied `support.py`/`support_build.py` commands, then run the recorded
`reference-support.sh` recipe. It copies sources privately, applies this patch,
builds the pinned LHA, and invokes the original Makefile with:

```sh
make -j2 PREFIX=/work/reference/sdk HOST= TARGET=m68k-amigaos THREADS=no NDK=3.2 \
    CFLAGS_FOR_TARGET='-O2 -fomit-frame-pointer' \
    CXXFLAGS_FOR_TARGET='-O2 -fomit-frame-pointer -fno-exceptions -fno-rtti' \
    min fd2sfd fd2pragma sfdc ndk13 netinclude
```

The native C/C++ compiler flags are `-Os`. The environment recipe records the
host compiler, binary package closure and build dependencies; the support lock
records exact source identities and all recipe/patch hashes.

The snapshots do not include private Git metadata. To rebuild directly from
the conveyed source archives, extract the build-orchestration archive as
`/work/reference/amiga-gcc`, place its component snapshots under `projects/`
using the source-directory mapping in `reference-support.sh`, and place the
original GMP/MPFR/MPC/NDK archives under the `download/` names in that recipe.
Restore the inherited libnix source from its supplied original-plus-patch form
and apply this GCC patch to the extracted GCC project. Install the directed
ordinary libamiga archive in the target library directory and build/install
LHA as shown by the recipe before the Makefile invocation above. No Makefile
update/fetch target is required when those exact source inputs are present.
The automated Git-integrity preflight itself still requires the matching Git
metadata; that distinction does not remove any source from these snapshots.

Retain GCC's original copyright notices, `COPYING3`, `COPYING.RUNTIME` and the
other applicable source notices with this modification and its source.
Distribution-specific support belongs at
<https://github.com/OpenLorraine/clorraine/issues>.
