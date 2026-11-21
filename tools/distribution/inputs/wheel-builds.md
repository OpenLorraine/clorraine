# Fixed-source wheel production

`wheel-payload.py` runs only inside the selected prepared interpreter.
`support-payload.sh --wheels cp310|cp312` selects that interpreter. The public
builder authenticates source inputs and the prepared image before execution.
It supplies the exact lock digest; the recipe never invents one.

| Output | Version | Role |
|---|---|---|
| amitools | `0.8.1.dev274+gd0b9727da` | vamos and supporting Python code |
| machine68k | `0.4.1` | Native m68k emulation binding |
| greenlet | `3.5.6` | Native coroutine support |
| PySDL2 | `0.9.17` | Dependency declared by amitools' `vamos` extra |
| pip | `26.2.1` | Offline bootstrap |

The native wheels have ABI-specific `cp310-cp310-linux_x86_64` or
`cp312-cp312-linux_x86_64` tags. Other outputs are `py3-none-any`.
PySDL2 is retained because the actual built metadata declares it for vamos,
not merely because an older SDK installed it. Console execution does not
exercise SDL graphics; it must not be reported as graphics acceptance.

## Inputs and execution

The recipe checks the exact twelve original Python source archive hashes and
eight prepared backend versions. It builds with no index, no dependencies and
no build-isolation downloads. The complete runtime dependency closure is then
checked against actual built wheel metadata, including the vamos extra.
Build backends remain build-only; only bootstrap pip is added to the runtime
wheel set.

The selected amitools checkout has no release tags. Its packaging version is
fixed to the observed SDK version above, independently of local history depth.
`SOURCE_DATE_EPOCH=1790576253` is that commit's timestamp. The clean compiler
and Python build environment does not inherit a caller's flags or package
index configuration.

Corresponding-source data includes a raw amitools snapshot, the twelve
original Python archives and the wheel recipe. Notice collection retains
named licence files plus explicitly identified inline terms in amitools and
machine68k, the Musashi source/header notices, and PySDL2's detailed copying
document. These are original source bytes, not a licence-clearance claim.
The complete wheel-content/compliance audit remains mandatory.

## Manual fixed-input evidence

Actual builds for both ABIs ran as the caller's UID/GID, with no network and
read-only source staging, in immutable image
`sha256:0bea57344e1ed0024b0968192f40c44b716fd27e504307a0d817f81d29bcc9a3`.
The recipe's `produce()` entry returned package records without creating a
release-bound `wheels.json`. Receipts explicitly have `release_lock: null`.

Retained private output:
`build/distribution/reference-20261117/wheel-authoring/<abi>/`.
Each `manual-wheel-receipt.json` records the actual wheel hashes, source IDs,
versions and tags. `manual-runtime-receipt.json` records:

- A new venv without preinstalled pip or build backends, bootstrapped using
  the built pip wheel.
- Wheel-only, no-index installation of the actual `amitools[vamos]` closure,
  followed by `pip check`.
- Actual machine68k/greenlet imports, a greenlet execution check and loading
  the installed vamos entry point.
- Strict C99 target programs compiled from the selected relocated SDK, then
  hello stdout and exit status 7 under vamos for both `nix13` and `nix20`.
- Four native ELF files per ABI, all x86_64, with resolved library dependencies,
  no RPATH/RUNPATH or SONAME, and GLIBC requirements no newer than 2.14.

The native dependencies are `libc.so.6`, `libstdc++.so.6`, `libgcc_s.so.1` and
`ld-linux-x86-64.so.2`; no build backend is part of the installed Python set.
All imports execute inside the fresh venv, not the producer environment.

These are authoring probes in the prepared 22.04 image. They do not replace
the final real-lock producer runs, clean Ubuntu 22.04/24.04 archive tests,
relocation acceptance, complete source/notice assembly or publication approval.

## Runtime-only Greenlet selection

The first real archive run exposed upstream's deliberately included native
test extensions. Importing them executes `greenlet.tests`, which requires the
optional test extra (psutil/objgraph/setuptools), not the vamos runtime closure.
The approved runtime wheel excludes exactly `greenlet/tests/`; it retains all
other member bytes/ZIP metadata and regenerates RECORD hashes/sizes. It does
not bypass any surviving-native import or loader check.

Seven independent selection/preservation/RECORD/determinism/negative controls
ran red before implementation and now pass. Both actual original ABI wheel
copies lose exactly 31 test members with all other member bytes except RECORD
unchanged. Original source archives and both Greenlet licences remain conveyed.
This changes no source pin, version or runtime dependency.

Previous lock-bound producers, archives and failed real-archive receipts are
preserved under `pre-greenlet-selection/` in the private reference directory.
All affected exact-lock producers and actual Ubuntu proofs must be repeated;
the old manual and first-candidate receipts are not evidence for the new
runtime-only wheel revision.
