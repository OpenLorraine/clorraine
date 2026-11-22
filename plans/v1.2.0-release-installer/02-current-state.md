# Current state: native Linux delivery

> **Parent**: [Index](00-index.md)
> **Observed**: 2026-11-16, baseline `daa2fba0`

## Existing surfaces

| Surface | Evidence | Gap relevant to this delivery |
|---|---|---|
| Driver environment | `compiler/src/clorraine/environment.rs:20`; `mod.rs:109-126` | Fixed `/opt/amiga` marker blocks native user-root builds. |
| Tool selection | `compiler/src/clorraine/toolchain.rs:174-205,267-280` | Bare tool names rely on PATH; installed component selection is absent. |
| Run defaults | `compiler/src/clorraine/mod.rs:134-155` | Host needs `cc`; Amiga Run needs vamos unless `--emulator` is set. |
| Emulator | `toolchain.rs:295-308`; `docker/scripts/run.sh:65-148` | Docker handoff and repository/container paths; native launch absent. |
| Driver oracle | `compiler/tests/clorraine_driver_spec_test.rs:305-355` | Exact container handoff must continue; native tests must be additional. |
| Build artifacts | `compiler/src/clorraine/artifacts.rs:51-61` | Every build resets the program directory; live emulator mounts must live elsewhere. |
| SDK build | `docker/Dockerfile:21-82`; `docker/versions.env:13-31` | Debian bookworm, broad SDK targets, no clean Ubuntu 22.04 distribution proof. |
| Setup | `tools/setup/install_sdk.py:1-19` | Docker detect/build tool, not native archive installation. |
| Extension | `tools/vscode/package.json:18-49`; `src/extension.ts:324-354` | LSP support but no Build/Run actions or installed-root discovery. |
| CI | `.github/workflows/check.yml:15-30` | Ubuntu host tests; no support/core release workflows. |
| Published payload | `RELEASE.md:138-150` | v1.1.2 contains two core binaries, no LSP; runbook is historical. |
| Sibling installer | `../clorraine-installer/README.md:3-15` | QtIFW scaffold; user chose main-repository shell delivery. |

## Toolchain inputs

The build-repository commit is
`e7ed7a735c4dd9d2c47e23a50e51b2b7ddd4688c`. Its
[default-repos](https://github.com/AmigaPorts/m68k-amigaos-gcc/blob/e7ed7a735c4dd9d2c47e23a50e51b2b7ddd4688c/default-repos)
names moving branches. Its
[Makefile](https://github.com/AmigaPorts/m68k-amigaos-gcc/blob/e7ed7a735c4dd9d2c47e23a50e51b2b7ddd4688c/Makefile)
clones those branches; a root commit alone is not a complete lock.

The current SDK provenance records these abbreviated revisions. They are
research evidence, not acceptable full hashes for a release lock:

| Component | Observed revision |
|---|---|
| GCC | `26e100b31` |
| binutils | `486d13ba` |
| libnix | `0a1d18a` |
| newlib-cygwin | `d8eff91` |
| sfdc | `4e7afc0` |
| fd2sfd | `189ac23` |
| fd2pragma | `bcffac5` |
| amiga-netinclude | `7d155ca` |
| aros-stuff | `86711c7` |

The LHA helper's exact source revision is not in that installed provenance.
The first phase must resolve and record it, audit its terms and prove the
resulting archive handling. No claim of reproducible support builds is made yet.

The upstream `min` target pulls in newlib, libnix, NDK preparation, NDK-derived
1.3 headers, network headers and libgcc. NDK preparation modifies C headers,
generates wrappers using sfdc/fd2sfd and applies the pinned build-tree patches.
The archive alone does not reproduce the installed include trees.

The NDK archive SHA-256 is
`96cabd4ad683dced632e147bf86dee0f50dcb1254386216c25c362916a6409bb`.
Its retrieved bytes matched this value. Inspection of `ReadMe-NDK.txt` and
`lib/README` did not establish modified-tree redistribution permission.

## vamos inputs

Current SDK metadata identifies:

| Package | Observed identity |
|---|---|
| amitools | `0.8.1.dev274+gd0b9727da`, source `d0b9727da3069328231784d2d5fdcf44816c95b1` |
| machine68k | `0.4.1`, native CPython extension |
| greenlet | `3.5.6`, native CPython extensions |
| PySDL2 | `0.9.17` |

The installed native modules are CPython 3.11 binaries. They are not the
required CPython 3.10/3.12 release wheels. Their build/runtime dependencies and
licences need the same support-payload audit.

## Verified baseline

`make check` passed during planning: **4,232 Rust tests**, **160 Python tests**
(setup 29, performance 103, fuzz 28), plus registry/spec checks and fuzzer smoke
`cases=28 host=28 sanitizer=16 target=16 failures=0`.

No native package, Ubuntu 22.04 relocation, Linux desktop editor or native
FS-UAE acceptance run has happened. Earlier platform/build-time/size estimates
are not evidence.

## Principal risks

| Risk | Required proof or control |
|---|---|
| Host library/GLIBC mismatch | Support build and candidate tests on the approved oldest baseline. |
| Hidden original-prefix dependency | Clean machine with no source tree or `/opt/amiga`; move the installed root. |
| Missing or incompatible wheels | Offline binary-only install on both supported interpreter versions. |
| NDK preparation mismatch | Compare local preparation to the same locked reference build and compile both profiles. |
| Partial update/publication | Single activation point locally; reference assets public before distribution promotion. |
| Editor/driver drift | Editor calls the driver; packaged VSIX acceptance from the Linux desktop. |
| Unknown licensed-media evidence | Block candidate acceptance until the named local emulator runs exist. |

## Execution source research — 2026-11-17

The current SDK image is
`sha256:3835c03b273e3d71b5ae3790c6c5706a511722690c9e67a6316420746880a782`,
created `2026-10-08T01:04:09.644273162+02:00`. Its observed GCC is
`16.2.0b 20260825082934`. This is the tested Debian SDK, not the future
Ubuntu-compatible build environment.

Read-only upstream commit lookups resolved all recorded abbreviated identities;
each returned complete ID matches its observed prefix:

| Source repository under `https://github.com/AmigaPorts/` | Complete commit |
|---|---|
| `gcc` | `26e100b31d25392a21068483d4c541a4b55822c3` |
| `binutils-gdb` | `486d13bafc19f15a65be861071bdee1381314f02` |
| `libnix` | `0a1d18a0938dce3f7a234963bcb7fa6148f6a6f3` |
| `newlib-cygwin` | `d8eff91f1916ee996c30f10e90984563cddc310f` |
| `sfdc` | `4e7afc044c1ff129574fcf3b839c3351edcfe2fb` |
| `fd2sfd` | `189ac23b5b2d0d0555381404d75e0508c2e95920` |
| `fd2pragma` | `bcffac5c89295baf1db033addc984312b4b4a722` |
| `amiga-netinclude` | `7d155ca3a3090afe3466f819dd178f2af6f5c176` |
| `aros-stuff` | `86711c70569ffc6b70ee1fb99895866c1d9f30e4` |
| `amitools` | `d0b9727da3069328231784d2d5fdcf44816c95b1` |

The [pinned build Makefile](https://github.com/AmigaPorts/m68k-amigaos-gcc/blob/e7ed7a735c4dd9d2c47e23a50e51b2b7ddd4688c/Makefile#L1356-L1363)
clones LHA using a shallow `master` checkout into the build directory, not the
project directory used by `make l`. The installed SDK retains `/opt/amiga/bin/lha`
but no LHA checkout or revision record. Its exact original source ID is therefore
not established.

The latest currently reachable LHA master commit preceding the final image's
creation is `08486d2da2ca77a6adf83194fd67400c8d86e7c7`, dated 2026-08-24.
This is a candidate historical snapshot, not recovered build evidence.
Current master `979815dc8699a9c5018de067946737c83ec6f697` is dated 2026-10-17,
after the image was built. AR-31 now explicitly approves the historical
`08486d2...` commit as a new LHA candidate. It has not yet been built or proven;
no historical-identity claim is made.

The historical [README](https://github.com/AmigaPorts/lha/blob/08486d2da2ca77a6adf83194fd67400c8d86e7c7/README.md)
points to `man/lha.man` for current redistribution terms, with additional
file-specific terms. Its distribution section requires source/documentation
and includes commercial-use conditions. No permission conclusion or waiver is
recorded here; the required legal/source audit remains unperformed.

The pinned build Makefile also identifies GMP 6.2.1, MPFR 4.1.0 and MPC 1.2.1
archives and hashes, but their complete records, Ubuntu build image/tool/package
closure, wheel inputs, preparation inventory and licence records are not yet
resolved. No partial `support-lock.json` has been emitted as eligible output.

### Verified archive bytes

The following archives were downloaded over HTTPS with bounded reads. All
actual lengths and SHA-256 values match the pinned upstream Makefile values:

| Archive | Bytes | SHA-256 |
|---|---:|---|
| `gmp-6.2.1.tar.bz2` | 2493916 | `eae9326beb4158c386e39a356818031bd28f3124cf915f8c5b1dc4c7a36b4d7c` |
| `mpfr-4.1.0.tar.bz2` | 1747243 | `feced2d430dd5a97805fa289fed3fc8ff2b094c02d05287fd6133e7f1f0ec926` |
| `mpc-1.2.1.tar.gz` | 838731 | `17503d2c395dfcf106b622dc142683c1199431d095367c6aacba6eec30340459` |
| `NDK3.2.lha` | 8448365 | `96cabd4ad683dced632e147bf86dee0f50dcb1254386216c25c362916a6409bb` |

The three arithmetic-library URLs are beneath
`https://gcc.gnu.org/pub/gcc/infrastructure/`; NDK is
`https://aminet.net/dev/misc/NDK3.2.lha`.

Exact-version PyPI source archives were also downloaded and matched their
published lengths/hashes. These are source inputs, not the required built
CPython 3.10/3.12 wheels:

| Source archive | Bytes | SHA-256 |
|---|---:|---|
| `machine68k-0.4.1.tar.gz` | 490759 | `a58ae3f0f5f32639296e7eaf564f2aa96852462078e35b0c6d8e714d2510a9cb` |
| `greenlet-3.5.6.tar.gz` | 207653 | `8e67c43bdfc88d5fee6db0d3e40175b362fc95fb85f0412d233b9b203c53a575` |
| `pysdl2-0.9.17.tar.gz` | 775955 | `48c6ef01a4eb123db5f7e46e1a1b565675755b07e615f3fe20a623c94735b52b` |
| `pip-26.2.1.tar.gz` | 1848877 | `f6ad667e89a1fe78046c8f13232b247200f5258d7828f3f7883d660878e0813f` |
| `setuptools-66.1.1.tar.gz` | 2644145 | `ac4008d396bc9cd983ea483cb7139c0240a07bbc74ffb6232fceffedc6cf03a8` |

The actual SDK venv contains pip 26.2.1 and setuptools 66.1.1 in addition to the
previously recorded runtime packages. This installed setuptools version does
not identify the transient isolated backend used to build those packages.
Inspection of verified source metadata establishes additional build inputs:

- greenlet requires `setuptools >= 77.0.3`.
- machine68k and the exact amitools Git revision require setuptools,
  setuptools-scm and `cython >= 3.0.0`.
- pip requires `flit-core >= 3.11, < 4`.

The complete build-only dependency pins remain to be resolved. Source-archive
metadata and hashes do not establish wheel compatibility, loader closure,
licensing compliance or successful offline preparation.

### Build-environment candidates after AR-32

Registry inspection resolves the official Ubuntu 22.04 Linux/amd64 base to
`docker.io/library/ubuntu@sha256:45e9ddc6895604674618a409ca9f73d301da8fdb75fe330e8e9a27b3b219e4ad`.
This identifies the base only; it is not a prepared compiler/Python build image.
The local environment recipe and its measured tool/package closure remain
unproduced. AR-32 permits preparing that environment locally and executing by
its immutable image ID, without publishing a builder image.

The base has now been pulled by that exact digest. Inspection confirms
Linux/amd64 and one root filesystem layer; a non-root, network-disabled
container reports Ubuntu 22.04. Docker reports its inspected ID as the same
`sha256:45e9dd...` value on this host. Consume the actual inspected ID rather
than assuming all engine versions expose a particular underlying object kind.
This named base remains in the Docker cache for environment preparation; it
does not contain the required prepared build tools and is not a native artifact.

The following exact build-only candidates now have source bytes verified
against their published lengths and SHA-256 values. None is claimed as a
tested environment input:

| Build-only source | Version | Verified source SHA-256 |
|---|---|---|
| setuptools | 77.0.3 | `583b361c8da8de57403743e756609670de6fb2345920e36dc5c2d914c319c945` |
| setuptools-scm | 8.3.1 | `3d555e92b75dacd037d32bafdf94f97af51ea29ae8c7b234cf94b7a5bd242a63` |
| Cython | 3.0.12 | `b988bb297ce76c671e28c97d017b95411010f7c77fa6623dd0bb47eed1aee1bc` |
| wheel | 0.45.1 | `661e1abd9198507b1409a20c02106d9670b2576e916d58f520316666abca6729` |
| packaging | 24.2 | `c228a6dc5e932d346bc5739379109d49e8853dd8223571c7c5b55260edc0b97f` |
| tomli | 2.0.2 | `d46d457a85337051c36524bc5349dd91b1877838e2979ac5ced3e710ed8a60ed` |
| flit-core | 3.12.0 | `18f63100d6f94385c6ed57a72073443e1a71a4acb4339491615d0f16d6ff01b2` |

Runtime pins are unchanged. The verified setuptools-scm source imposes
`tomli<=2.0.2` on Python 3.10 during its own build, as well as `tomli>=1` at
runtime. The initially researched 2.2.1 candidate was therefore rejected;
2.0.2 satisfies both. Documentation/test extras are not selected. Flit-core
and setuptools declare in-tree backends with no external bootstrap requirements;
wheel, packaging and tomli use flit-core. Cython's source has no pyproject file
and uses the legacy setuptools build.

The official CPython 3.12.16 archive was also downloaded and verified:
20,839,652 bytes, SHA-256
`b5796915dc45ff745cfb4e990a7e7c93d6f546328a102ce31448d69ebc37575c`.
It is a maintainer-side interpreter input for building cp312 wheels on Ubuntu
22.04, not a new bundled user runtime. The source includes an ensurepip
pip 25.0.1 wheel; any bootstrap use must be recorded separately from selected
pip 26.2.1.

`tools/distribution/inputs/README.md` records source lengths/hashes, acquisition
locations and the inspected backend constraints without pretending to be a
complete lock.

### Acquired Ubuntu package inputs

The Ubuntu 22.04 base was created on 2026-11-10. A proposed 2026-11-01 snapshot
could not satisfy perl's exact dependency because the base already contains a
newer perl-base. The later fixed snapshot
`https://snapshot.ubuntu.com/ubuntu/20261117T000000Z/` resolves the direct build
and Python-development requests without downgrading that package.

APT verified the three suite signatures with the base's Ubuntu archive keyring.
All six main/universe package indexes were available without errors.
Download-only acquisition produced 126 packages, 123,238,900 bytes.
Independent SHA-256/length checks matched every actual archive to the signed
index record. Nothing was installed into the host or base image.

`tools/distribution/inputs/ubuntu-packages.sha256` preserves all acquired
package hashes/paths; its hash is
`e066847cd7c084c0151f60bd2788c63b2d1c576374701722fb73c58bf2751e1a`.
The input guide records exact direct requests and signed-index hashes.
This is a proposed build-environment package set, not an installed-version
claim or the final output runtime closure. Environment/reference recipes,
licence reviews and actual build proof remain pending. The set includes the
existing SDK's math development prerequisites: upstream newlib depends on the
binutils GDB build, even though GDB is excluded from the shipped payload. This
is build-time closure, not an expansion of distributed tools.
