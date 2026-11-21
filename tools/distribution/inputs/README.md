# Reference build inputs

This directory records inputs for the native support reference build.
The source identities below have been resolved or checked against downloaded
bytes. They are **not** a complete support lock, redistribution approval or
evidence of a successful build. Environment recipes, package closure and the
prepared reference inventory must exist before the final lock is produced.

## Source checkouts

Stage clean checkouts at `checkouts/<id>` below the source-stage root. Preserve
the canonical HTTPS origin and complete commit. Do not replace these commits
with branch tips. The build copies sources into its private work directory;
the staged originals stay read-only.

| ID | Repository under `https://github.com/AmigaPorts/` | Commit |
|---|---|---|
| `amiga-gcc` | `m68k-amigaos-gcc` | `e7ed7a735c4dd9d2c47e23a50e51b2b7ddd4688c` |
| `gcc` | `gcc` | `26e100b31d25392a21068483d4c541a4b55822c3` |
| `binutils` | `binutils-gdb` | `486d13bafc19f15a65be861071bdee1381314f02` |
| `libnix` | `libnix` | `0a1d18a0938dce3f7a234963bcb7fa6148f6a6f3` |
| `newlib-cygwin` | `newlib-cygwin` | `d8eff91f1916ee996c30f10e90984563cddc310f` |
| `sfdc` | `sfdc` | `4e7afc044c1ff129574fcf3b839c3351edcfe2fb` |
| `fd2sfd` | `fd2sfd` | `189ac23b5b2d0d0555381404d75e0508c2e95920` |
| `fd2pragma` | `fd2pragma` | `bcffac5c89295baf1db033addc984312b4b4a722` |
| `amiga-netinclude` | `amiga-netinclude` | `7d155ca3a3090afe3466f819dd178f2af6f5c176` |
| `aros-stuff` | `aros-stuff` | `86711c70569ffc6b70ee1fb99895866c1d9f30e4` |
| `amitools` | `amitools` | `d0b9727da3069328231784d2d5fdcf44816c95b1` |
| `lha` | `lha` | `08486d2da2ca77a6adf83194fd67400c8d86e7c7` |

LHA is a newly selected candidate. Its identity was not recovered from the old
SDK. The other checkouts retain the SDK's recorded source cohort. The selected
LHA still needs build, extraction, preparation and redistribution review.

Source notice findings are recorded in [licence-review.md](licence-review.md),
[python-licences.md](python-licences.md) and
[build-licences.md](build-licences.md). The
[payload origin review](payload-origins.md) traces install producers and
exclusions. These reviews are not a completed fresh-build inventory or
publication clearance.

## Verified source archives

Stage archive bytes at `archives/<id>`. Verify exact length and SHA-256 before
extracting or using any contents. These records describe original sources, not
prebuilt wheels.

| ID | Source archive | Bytes | SHA-256 |
|---|---|---:|---|
| `gmp` | `gmp-6.2.1.tar.bz2` | 2493916 | `eae9326beb4158c386e39a356818031bd28f3124cf915f8c5b1dc4c7a36b4d7c` |
| `mpfr` | `mpfr-4.1.0.tar.bz2` | 1747243 | `feced2d430dd5a97805fa289fed3fc8ff2b094c02d05287fd6133e7f1f0ec926` |
| `mpc` | `mpc-1.2.1.tar.gz` | 838731 | `17503d2c395dfcf106b622dc142683c1199431d095367c6aacba6eec30340459` |
| `ndk` | `NDK3.2.lha` | 8448365 | `96cabd4ad683dced632e147bf86dee0f50dcb1254386216c25c362916a6409bb` |
| `machine68k` | `machine68k-0.4.1.tar.gz` | 490759 | `a58ae3f0f5f32639296e7eaf564f2aa96852462078e35b0c6d8e714d2510a9cb` |
| `greenlet` | `greenlet-3.5.6.tar.gz` | 207653 | `8e67c43bdfc88d5fee6db0d3e40175b362fc95fb85f0412d233b9b203c53a575` |
| `pysdl2` | `pysdl2-0.9.17.tar.gz` | 775955 | `48c6ef01a4eb123db5f7e46e1a1b565675755b07e615f3fe20a623c94735b52b` |
| `pip` | `pip-26.2.1.tar.gz` | 1848877 | `f6ad667e89a1fe78046c8f13232b247200f5258d7828f3f7883d660878e0813f` |
| `setuptools` | `setuptools-77.0.3.tar.gz` | 1367236 | `583b361c8da8de57403743e756609670de6fb2345920e36dc5c2d914c319c945` |
| `setuptools-scm` | `setuptools_scm-8.3.1.tar.gz` | 78088 | `3d555e92b75dacd037d32bafdf94f97af51ea29ae8c7b234cf94b7a5bd242a63` |
| `cython` | `cython-3.0.12.tar.gz` | 2757617 | `b988bb297ce76c671e28c97d017b95411010f7c77fa6623dd0bb47eed1aee1bc` |
| `wheel` | `wheel-0.45.1.tar.gz` | 107545 | `661e1abd9198507b1409a20c02106d9670b2576e916d58f520316666abca6729` |
| `packaging` | `packaging-24.2.tar.gz` | 163950 | `c228a6dc5e932d346bc5739379109d49e8853dd8223571c7c5b55260edc0b97f` |
| `tomli` | `tomli-2.0.2.tar.gz` | 16096 | `d46d457a85337051c36524bc5349dd91b1877838e2979ac5ced3e710ed8a60ed` |
| `flit-core` | `flit_core-3.12.0.tar.gz` | 53690 | `18f63100d6f94385c6ed57a72073443e1a71a4acb4339491615d0f16d6ff01b2` |
| `cpython312` | `Python-3.12.16.tar.xz` | 20839652 | `b5796915dc45ff745cfb4e990a7e7c93d6f546328a102ce31448d69ebc37575c` |

### Acquisition locations

- GMP, MPFR and MPC: append the filename to
  `https://gcc.gnu.org/pub/gcc/infrastructure/`.
- NDK: `https://aminet.net/dev/misc/NDK3.2.lha`.
- CPython: `https://www.python.org/ftp/python/3.12.16/Python-3.12.16.tar.xz`.
  Its hash matches the official release page.
- Python packages: obtain the exact-version source record from
  `https://pypi.org/pypi/<name>/<version>/json`, then require its source hash
  and length to match this table. Record the returned HTTPS archive URL in the
  final lock. The API is acquisition metadata, not executable input or permission
  to choose a newer version.

NDK acquisition does not authorise distributing the archive or derived headers.
Keep it out of ordinary payloads and corresponding-source bundles.

## Included ordinary target library

`libamiga.a` is the exact ordinary archive from the pinned build repository's
`lib/libamiga.a`, copied by explicit maintainer direction. It is 297,694 bytes,
SHA-256 `10d78b6d303a99677c0bf29a8e1653d62669f69f395691646393020325084021`.
The separate base-relative archive is not included by this direction.

Stage this file at `inputs/libamiga.a`. The reference recipe verifies its
length/hash before building, installs it at
`sdk/m68k-amigaos/lib/libamiga.a` and rechecks the installed bytes.
Its hash must be recorded among the final lock's input hashes and payload
inventory. Do not substitute a different archive.

Its original source/licence chain remains unresolved. The maintainer has
retained responsibility for its legal review; see `licence-review.md`.
Inclusion is not a licence-clearance statement or public-release approval.

## Python build dependencies

The build-only backend versions differ from the old SDK's installed
setuptools 66.1.1. That installed version cannot identify a transient isolated
backend. Do not include it merely to imitate the old venv.

| Source | Inspected build requirement |
|---|---|
| greenlet | setuptools at least 77.0.3 |
| machine68k and amitools | setuptools, setuptools-scm and Cython at least 3.0 |
| pip | flit-core at least 3.11 and below 4 |
| setuptools | In-tree `setuptools.build_meta`, no external bootstrap requirements |
| setuptools-scm | setuptools at least 61; **tomli at most 2.0.2 on Python 3.10** |
| wheel | flit-core at least 3.8 and below 4 |
| packaging | flit-core at least 3.3 |
| tomli | flit-core at least 3.2 and below 4 |
| flit-core | In-tree `flit_core.buildapi`, no external bootstrap requirements |
| Cython | Legacy setuptools build; the inspected source has no `pyproject.toml` |

Setuptools-scm also uses packaging and setuptools at runtime, plus tomli on
Python 3.10. Its typing-extensions/importlib-metadata requirements apply only
below Python 3.10. Documentation and test extras are not selected.

The initially researched tomli 2.2.1 does not satisfy setuptools-scm's
Python 3.10 build constraint and is **not** a selected input. Tomli 2.0.2
satisfies both the build bound and runtime lower bound.

CPython 3.12.16 is a maintainer build tool for producing `cp312` wheels on the
Ubuntu 22.04 baseline. It is not an interpreter bundled for users. Its verified
source includes the `pip-25.0.1-py3-none-any.whl` ensurepip bootstrap; any use
of that bootstrap must be recorded separately from the selected pip 26.2.1.
Users still use Ubuntu's CPython 3.10 or 3.12 and its venv package.

## Environment and reference recipe boundary

The Ubuntu base is
`docker.io/library/ubuntu@sha256:45e9ddc6895604674618a409ca9f73d301da8fdb75fe330e8e9a27b3b219e4ad`.
It is Linux/amd64, reports Ubuntu 22.04, and was created on 2026-11-10.
It is not a prepared support-build image.

### Acquired Ubuntu package set

[`ubuntu-packages.sha256`](ubuntu-packages.sha256) pins the proposed
build-environment package archives beneath
`https://snapshot.ubuntu.com/ubuntu/20261117T000000Z/`. It contains 126
SHA-256/path pairs. The actual downloads total 123,238,900 bytes; every length
and hash matched its signed package-index record. This is an acquisition proof,
not a successful installation or the shipped runtime-package list.

The checksum file itself has SHA-256
`e066847cd7c084c0151f60bd2788c63b2d1c576374701722fb73c58bf2751e1a`.
Preserve its relative `pool/` paths when staging downloaded files, then verify
the complete set with `sha256sum --check` before environment preparation.
Do not fetch from a live mirror during preparation or silently resolve a
replacement package when a pinned archive is unavailable.

The dependency resolver used the pinned base's dpkg status, no recommends,
and these exact direct requests:

```text
build-essential=12.9ubuntu3
autoconf=2.71-2
automake=1:1.16.5-1.3
bison=2:3.8.2+dfsg-1ubuntu0.22.04.1
flex=2.6.4-8build2
gettext=0.21-4ubuntu4
git=1:2.34.1-1ubuntu1.18
libtool=2.4.6-15build2
patch=2.7.6-7build2
rsync=3.2.7-0ubuntu0.22.04.7
texinfo=6.8-4build1
pkg-config=0.29.2-1ubuntu3
xz-utils=5.2.5-2ubuntu1.2
ca-certificates=20260816ubuntu0.22.04.2
python3.10=3.10.12-1~22.04.18
python3.10-dev=3.10.12-1~22.04.18
python3.10-venv=3.10.12-1~22.04.18
libreadline-dev=8.1.2-1
libncurses-dev=6.3-2ubuntu0.3
zlib1g-dev=1:1.2.11.dfsg-2ubuntu9.2
libssl-dev=3.0.2-0ubuntu1.30
libffi-dev=3.4.2-4
libbz2-dev=1.0.8-5ubuntu0.1
liblzma-dev=5.2.5-2ubuntu1.2
libsqlite3-dev=3.37.2-2ubuntu0.8
libgdbm-dev=1.23-1
libgdbm-compat-dev=1.23-1
uuid-dev=2.37.2-4ubuntu3.6
libgmp-dev=2:6.2.1+dfsg-3ubuntu1
libmpfr-dev=4.1.0-3build3
libmpc-dev=1.2.1-2build1
```

APT verified repository signatures with the pinned base's Ubuntu archive
keyring. Sources were `jammy`, `jammy-updates` and `jammy-security`, each
using `main universe`. Historical snapshot validity-date checking was
disabled; signature and package hash checks were not disabled.

| Signed InRelease file | SHA-256 |
|---|---|
| `jammy` | `c14060cd8c6d625874dfcb9523a35a395bf4865c28b6b6a82569ef326fe92dc6` |
| `jammy-updates` | `d818f7b9d2ea3e78e7311457bda7418d89202e7ca911fac36cbf45342720ca83` |
| `jammy-security` | `0e3778c61e3151e0b0c449759f56edd65e0cc30a3faa3f19d01c3c24e3184d34` |

Resolution against the earlier 2026-11-01 snapshot failed because it predates
the base image's newer perl-base package. The fixed 2026-11-17 snapshot resolves
without downgrading that base package. No host or container packages were
installed during this research; the base image remains unchanged.

The math development packages preserve the existing SDK's build prerequisites.
The pinned upstream newlib target depends on the binutils GDB build even though
GDB is excluded from shipped support. This requires GMP/MPFR development
headers during the build. The separate pinned GMP/MPFR/MPC source archives
remain inputs to the upstream compiler recipe.

The acquired set contains Ubuntu venv bootstrap wheels: pip
`22.0.2+dfsg-1ubuntu0.7` and setuptools `59.6.0-1.2ubuntu0.22.04.3`.
These are distinct from the selected build backends and output pip wheel.
Record any bootstrap use and actual tool versions when the environment is
prepared. Package acquisition alone does not prove that all later builds have
their required tools or that all installed files are suitable for redistribution.

Prepare the build environment only from acquired, verified inputs. Record the
actual package/tool versions and the hashed environment Dockerfile. The actual
Docker-inspected prepared image ID selects offline execution. Verify its base
layers and input labels before support or wheel builds.

### Prepared environment: local build verified

`environment.Dockerfile` and `build-environment.sh` express the fixed offline
preparation steps. The corrected recipe has now built the measured image below.
That proves environment preparation, not that the later support build succeeds.

The explicitly assembled Docker context has only:

```text
inputs/environment.Dockerfile
inputs/build-environment.sh
inputs/ubuntu-packages.sha256
ubuntu/pool/...                     # The exact 126 acquired .deb files
archives/cpython312
archives/flit-core
archives/packaging
archives/tomli
archives/wheel
archives/setuptools
archives/setuptools-scm
archives/cython
archives/pip
```

Recheck archive lengths/hashes and recipe inputs before constructing this
context. Copy only those declared inputs, not the repository or a user's
home directory. The Dockerfile checks the bound checksum/script inputs, exact
package-file set and package hashes before local package installation.
It never uses a package index to repair missing downloads.

The first actual environment attempt verified all package hashes but failed
in APT's `--no-download` local-archive installation path. A disposable
network-disabled base-container probe showed that seeding APT's canonical
cache filenames fixes the transaction. Those names include package epochs
as `%3a`, even when the upstream pool filename omits them.
The recipe now copies only already verified packages into that cache using
validated package/version/architecture fields. It retains `--no-download`,
does not refresh package indexes and adds no package input. The complete
corrected environment build subsequently passed.

The fixed base and Dockerfile bytes determine the two preparation labels.
For a verified context at `$context`, the later reference build command is:

```sh
recipe_hash=$(sha256sum "$context/inputs/environment.Dockerfile")
recipe_hash=${recipe_hash%% *}
docker build --network=none --platform=linux/amd64 \
    --build-arg "ENVIRONMENT_RECIPE_SHA256=$recipe_hash" \
    --iidfile "$context/prepared-image.id" \
    --file "$context/inputs/environment.Dockerfile" "$context"
```

This performs environment preparation, not source acquisition or publication.
The image installs only local Ubuntu packages, then builds CPython and backend
wheels as an unprivileged builder. Both ABI-specific backend environments are
under `/opt/clorraine-build/venvs/`. Ubuntu's default interpreter and build
utilities remain available.

The image retains `ubuntu-packages.txt`, `ubuntu-bootstrap.txt` and
`cpython-bootstrap.txt` beneath `/opt/clorraine-build/` for measured-version
evidence. The local build produced:

| Measurement | Verified value |
|---|---|
| Prepared image ID | `sha256:0bea57344e1ed0024b0968192f40c44b716fd27e504307a0d817f81d29bcc9a3` |
| Environment Dockerfile SHA-256 | `3463b57421aecd01a1c5f8d6219af34e3313d0611cb83f53162474d75b2154ee` |
| Platform / ancestry | Linux/amd64; the exact pinned Ubuntu base's layers are an unchanged prefix |
| Preparation labels | Match the pinned base reference and Dockerfile hash |
| Python environments | `cp310`: 3.10.12; `cp312`: 3.12.16 |
| Backend packages | All eight selected backend package versions match the catalog in both environments |
| Installed packages | All 126 acquired package versions match the image receipt; total installed cohort 226 |
| Bootstrap tools | Ubuntu venv pip 22.0.2 and setuptools 59.6.0; CPython ensurepip pip 25.0.1 |
| Native compiler | Ubuntu GCC/G++ 11.4.0 |
| Package consistency | `dpkg --audit` empty; non-locking `apt-get check` passes |

The generated receipts and exact build argv are retained at
`build/distribution/reference-20261117`. Source builds ran as the unprivileged
builder with networking disabled. Subsequent compiler/wheel/reference builds,
fresh inventories, source/payload assembly and native acceptance remain required.

### Private SDK reference recipe and measured proof

`reference-support.sh` records the fixed C-toolchain/helper build. Run it only
after the required source/licence reviews and prepared-image checks. It expects
read-only source staging at `/sources`, a fresh owned `/work` directory and
network-disabled execution as the caller's UID/GID.

The corrected private reference build passed using the inspected image
ID above, the checked source repositories and verified archives.
Its source mount is read-only and work mount is newly owned.
Separate probes on that same image reject a correct-length/wrong-hash archive
and an exact-HEAD/wrong-origin checkout before creating reference output.
These rejection probes did not establish a successful SDK build. The later
canonical-source correction below preserves the originally acquired checkout
separately and does not change any pin or original reference artifact.

The first SDK attempt built the compiler/binutils/newlib stages but failed
when libnix could not find `libinit.h`. Command-line host `CFLAGS` propagated
through recursive Make and suppressed libnix's target flags and appended
include path. A non-compiling red/green probe against the exact libnix
Makefile confirmed this cause. The reference recipe now keeps fixed host
`CFLAGS`/`CXXFLAGS` exported, rather than passing them as Make overrides;
the pinned target flags remain explicit. Syntax and ShellCheck pass.
The failed reference tree was discarded, and the corrected full build passed
from the same sources and prepared image. No source pin changed.

The measured private SDK has 3,793 regular files, 11 safe internal relative
links and six empty directories. The 1,372 files under the two NDK include
trees and `ndk/` are recorded separately; that location-based measurement
includes independent networking/helper files and is not an origin banlist.
All 52 host ELF files are x86_64, require GLIBC no newer than 2.35 and have
no RPATH/RUNPATH. That full-tree list includes programs excluded from shipping.

The packaged cross-objdump identified all 166 target archives as m68k.
Of those, 165 use Amiga HUNK format rather than Unix archive magic.
The SDK also contains 26 standalone target objects. Strict C99 hello programs
linked for both `nix13` and `nix20` with the SDK mounted at `/sdk` and the
original builder prefix unavailable. This is not a vamos run, packaged-archive
relocation proof or complete native acceptance.

The recipe checks the fixed checkout identities/origins/clean state and archive
hashes, copies sources privately, builds the selected LHA first, then invokes
the pinned upstream targets. It preserves native defaults explicitly:
`HOST=`, `TARGET=m68k-amigaos`, `THREADS=no`, `NDK=3.2` and the original
host/target optimisation flags. It does not run `make update`.

Successful output is `/work/reference`, including the full private SDK,
source-revision receipt and version measurements. Failure discards only that
newly created reference directory. This script does **not** emit the public
support command's payload/provenance contract. The subsequent support recipe
must reuse these commands and apply the reviewed origin/exclusion rules before
producing an ordinary payload.

The private reference includes licensed NDK-derived material and build-time
tools. Do not package it directly. It supplies the header/inlines inventory and
origin comparison for the final lock; the native archive audit and relocation
proof are still mandatory.

Reference work must use the same fixed inputs and explicit commands later
used by support automation. The pinned upstream build rules require these
details to be handled explicitly:

- Copy clean staged checkouts and archives into private working directories.
  Never use `make update` or a branch-based clone to select source inputs.
- Build LHA from the selected checkout before invoking upstream targets. Its
  default Makefile rule otherwise clones a moving branch into the build tree.
- Stage GMP/MPFR/MPC and NDK under the exact download names expected by the
  pinned build rules. No fallback acquisition is permitted.
- Preserve the upstream NDK patches, both header variants, generated inlines,
  netinclude overlay and their notices for reference comparison. Exclude the
  derived tree from ordinary support archives after origin auditing.
- Retain compiler/runtime functionality while excluding unneeded gdb, vbcc,
  vasm and vlink payloads. Build-time use is distinct from shipped content.
- Build wheels separately for each interpreter ABI with the selected backends
  already present. Disable indexes, dependency resolution and build isolation
  that could acquire additional build dependencies.

This document records actual environment/reference build success and bounded
inspection. The user-approved Amiga-object audit format and producer-first
lock order are now applied, with the narrow specification amendment first.
The separate package-only grammar correction is approved and its specification
tests preceded the validator implementation. Generic source/helper/tool IDs
remain unchanged. The user also selected canonical stored Git blob bytes for
source staging; no Git-ident exception was added to verification.
No contract, test or task-order correction is applied without approval. Wheel builds,
delivered-source assembly, payload acceptance and publication remain unproved.

### Canonical Git bytes and affected-helper reproof

Source staging must contain authenticated stored blob bytes, not text
converted by Git checkout attributes. Verification is read-only; acquisition
must canonicalise any proven conversion before this boundary. Never replace
an unexplained worktree edit or normalise arbitrary marker contents.

The acquired fd2pragma checkout had `*.c ident` and expanded exactly one
leading `$Id$` comment in `fd2pragma.c`. The user approved replacing only that
proven expansion with the stored blob bytes in maintainer-owned staging.
The canonical candidate passed full object/index/worktree verification before
replacing staging. The original checkout is preserved at
`build/distribution/reference-20261117/source-originals/fd2pragma`; the original
private SDK/reference tree is unchanged.

The new `source-canonicalisation.json` records both file hashes and unchanged
commit `bcffac5c89295baf1db033addc984312b4b4a722`.
`canonical-helper-reproof.json` records a fresh build via the same pinned
upstream Makefile's `fd2pragma` target, compiler flags and immutable image ID,
as the caller's UID/GID with networking disabled. Its installed 181,224-byte
binary has SHA-256
`687935426345a83c7c71ac06916ee7dc28164ea6847232a43f7fdf454885d832`,
identical to the original reference. File mode and observed invocation output
also match. Consequently, SDK binary inventory/native records and the helper
used in the previous header-preparation proof are unchanged; this is affected
reference reproof, not final-lock or packaged support acceptance.

### Support-payload recipe

`support-payload.sh` invokes the same fixed reference recipe and selects a
C-only candidate. It retains the required C/compiler/linker executables,
libgcc/atomic C runtime objects, libnix variants and traced target
include/library producers, plus local preparation helpers. It omits the NDK
trees/link inputs, C++/Objective-C front ends and headers, GDB, compiler-plugin
development files, install-time header fixers and unapproved libamiga variants.
File-origin/native and complete source/notice audits are still mandatory;
selection is not permission to convey the candidate.

Current recipe SHA-256:
`b1256b6f7ee7581a368d00f429fbc176b9574f24d273cb0201da906756640dcf`.
The exact selector code was exercised against the preserved measured SDK in
the pinned offline image. All 422 files and 11 internal relative links match
their reference bytes/targets. No empty directories are copied.
The generated candidate is retained at
`build/distribution/reference-20261117/support-selection/payload`.
`support-selection-proof.json` records the bounded selection result.

Strict C99 compile/link probes against that selected tree, mounted read-only
at `/sdk` without the original prefix, pass `nix13` and `nix20`, both normally
and with `-flto`. `support-selection-link-proof.json` retains the result.
This is fixed-input producer authoring evidence, not a complete real locked
build, notice/source package, archive audit or clean-machine acceptance.

The first candidate had 423 files. Establishing the actual preparation helper
closure showed that fd2pragma contributes header data but its executable is
unused during installation. The current selector omits that binary. All other
selected bytes/links are unchanged; the original selection receipt is retained
as `support-selection-proof-initial.json`, and the current receipt records the
exact omission. No source pin, compiler/runtime feature or public command changed.

### Standalone transformation inputs and proof

[`ndk-inputs.md`](ndk-inputs.md) traces the 83 fixed data resources, including
seven exact unified patches and the independently identified helper/networking
headers. Exact patches reproduce the existing measured outputs without
install-time fuzzy patch matching or a new patch-program dependency.

The implemented standalone preparer validates member paths/types/aliases,
metadata, the full SDK baseline and all local resources before execution.
It then performs the byte-level copies/declaration edits and invokes only
LHA, `sfdc` and `fd2sfd`. The fixed fd2sfd comment banner preserves reference
hashes while allowing arbitrary actual SDK locations.

The actual transformation pipeline reproduced all 1,372 measured prepared
files and retained every baseline file/link. Compiler, build, package,
patch/rsync and network-tool guards stayed untouched in the network-disabled
prepared image. `ndk-transform-proof.json` records that manual fixed-input
proof; the private generated result is `ndk-transform-sdk/` beside the
original reference, without a marker claiming a completed real support lock.

`ndk-cli-fixture-proof.json` separately records the copied public command,
exact marker fields, repeat refusal and bundle/SDK paths containing spaces.
It deliberately uses a synthetic integration lock. It does not replace the
later real-lock/public-command or actual-archive acceptance gates.

### CPython wheel producers

[`wheel-builds.md`](wheel-builds.md) records the exact output set, build/runtime
boundary and manual proof. The two ABI builds each produce five actual wheels
from the pinned sources. Fresh offline wheel-only installations pass dependency
checks, native imports and vamos hello/exit-7 execution for both runtime
profiles. The eight total native wheel files are x86_64, have no RPATH/RUNPATH,
and require GLIBC no newer than 2.14.

The manual authoring outputs and explicitly lock-free receipts are retained at
`build/distribution/reference-20261117/wheel-authoring/`. No release-bound
`wheels.json` or synthetic release-lock hash was created by those builds.
The final lock/public producer runs and actual archive acceptance remain later
tasks. Inline machine68k/Musashi and amitools notices are retained explicitly;
notice-name discovery alone is not a complete compliance audit.

### Real support lock

`../support-lock.json` now records 28 actual source identities, 102 hashed
producer/review/data inputs, measured builder tools/packages and actual
22.04/24.04 prerequisite cohorts. It includes the previously reviewed inherited
libnix source representation under `source-forms/`; no build source was changed.
Package ownership is checked against actual resolved files, not dpkg diversion
commentary.

The original measured reference recipe remains historical evidence. Its
current source preflight explicitly checks index/content differences and extra
inputs because Git 2.34 can misreport canonical ident source bytes using stale
checkout stat data. Actual positive and five rejection controls pass; the
strict public raw-blob checker is unchanged. Current recipe SHA-256 is
`c2a1c94cb8304b9c53933df4f42e61e2c9bf03daa383c595c1386755d5f613a6`.

Initial completed lock SHA-256:
`8529d78db6b2eea225dd5575c8d3f458c597e8d7c2cf5402b3474d9f6977a72e`.
Public producer execution now passes. Neither this lock nor these
prerequisite measurements are complete packaging, compliance or clean-archive
acceptance. Publication remains separately authorised.

The actual locked C payload has 422 files and 11 links; each wheel payload has
99 files and five actual wheels. Exact inventories match provenance. The
standalone preparer then passes on both Ubuntu releases using actual locked
helper/baseline bytes, with all 1,372 output hashes, exact real-lock markers,
space paths and repeat refusal verified. Named `locked-*-proof.json` receipts
and actual outputs are retained for packaging/acceptance. These results do not
complete conveyed-file compliance or archive acceptance.

### GCC space-path build adjustment

The complete original source pins remain unchanged. Current lock
`aca1a0ec2dc4c3fa8e98a3b5bdb17ba0ec6d750fd11e082e4ca2b88e63588566`
has 104 inputs, including the exact one-function GCC patch and its notice/
rebuild instructions under `compiler-patches/`. The recipe checks the entire
original and modified driver-source hashes around private-copy patching.
The original staged checkout remains canonical and read-only.

The preceding candidate's native closure and original corpus pass on both
Ubuntu releases, but moved-root Amiga compilation fails because raw find-file
results are reparsed as specs tokens. The new patch, corresponding-source
assembly and exact-lock proof address that boundary only. All public producers,
archives and both real acceptance suites must be repeated before promotion.
Previous original/corrected-wheel candidates and failed actual evidence are
retained separately under the private reference directory.
