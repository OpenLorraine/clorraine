# Support builds and portability proof

> **Parent**: [Index](00-index.md)
> **Authority**: AR-13/17/18/24/26/27/28/30/31/32/33/34/35/36

## Boundary

Produce reusable Linux x86_64 support archives independently of the compiler.
Keep the existing Docker SDK and its acceptance role. Native packaging uses the
tested source cohort described in [02](02-current-state.md), but must establish
complete input pins and Ubuntu compatibility before any dependent phase.

Use a small `tools/distribution/` package for source-lock data, support build,
payload audit and packaging commands. Use existing Python unittest discovery;
do not add a general build system or test framework. Keep installer transport
and activation in `tools/setup/`, and driver execution in Rust (AR-17/20).

## Private-display gate prerequisite

Before the first full gate, preserve caller-supplied `DISPLAY` and `XAUTHORITY`
across `.env` loading in both `docker/scripts/run.sh` and `emulator.sh`, following
the existing override pattern in `a500-smoke.sh`. Preserve current defaults
when callers supply no value and leave the historical driver handoff unchanged.
Use a private Xvfb display and its matching authentication file for every full
gate; do not edit the user's primary `.env`.

Write `compiler/tests/native_gate_spec_test.rs` first. Use copied wrapper/config
fixtures and a logging Docker executable to prove red behavior without opening
a desktop window. After both wrappers are fixed, verify direct and run-wrapper
invocations through actual Docker/FS-UAE on the private display, including the
effective authentication path. Conflicting desktop fixture values must lose
to the caller's private values. Only then run full verification, including any
gate required for shared infrastructure changes. Record the private display
setup in the distribution guide for subsequent phases.

## Locked inputs

`tools/distribution/support-lock.json` records the exact input closure:

| Record | Required data |
|---|---|
| Source checkout | Component name, canonical upstream HTTPS URL, complete commit object ID, applicable licence and source-notice location |
| Source archive | URL, exact byte length, SHA-256, version, licence record |
| Build environment | Ubuntu 22.04 base identity, compiler/interpreter/package versions and build commands |
| Patches and preparation | Tracked patch/script paths with hashes, helper identities, expected prepared-tree inventory |
| Output | Host architecture, supported interpreter ABI, target runtimes, runtime package closure, hashes and provenance |

Resolve the abbreviated cohort from [02](02-current-state.md); do not replace
it with current branch tips. Include build-only inputs (GMP/MPFR/MPC, Python
wheel build backends, native-extension build inputs) as well as runtime inputs.
The LHA helper is a mandatory source-resolution/audit task. No short hash,
branch, unchecked download, or best-effort provenance entry qualifies.
AR-31 explicitly selects LHA commit
`08486d2da2ca77a6adf83194fd67400c8d86e7c7` as a new candidate baseline because
the old helper's revision is unrecorded. Do not describe it as recovered SDK
provenance. All other known cohort pins remain unchanged; LHA source/licence,
archive handling and reference-preparation proofs still have to pass.

Build scripts consume the lock, stage exact sources, and prevent the upstream
Makefile from silently fetching different revisions. Verify actual source HEADs
against the lock before producing assets. Network acquisition ends before the
build/preparation steps; a later attempt to fetch an undeclared input fails.
Record resolved evidence, not a claim that the upstream root pin is sufficient.

## Executable support contract

AR-30 authorises this concrete interface for independent specification tests.
These are maintainer commands, using the already planned scripts:

| Command | Observable result |
|---|---|
| `python3 tools/distribution/support.py validate-lock --lock PATH` | Read-only validation of the lock's structure and references |
| `python3 tools/distribution/support.py verify-sources --lock PATH --sources DIR` | Read-only validation of the lock and actual staged inputs |
| `tools/distribution/build-support.sh --lock PATH --sources DIR --image-id ID --output DIR` | Verified, isolated build in an already prepared local image, then publication of successful local outputs |
| `python3 tools/distribution/package_support.py --lock PATH --payload DIR --audit PATH --output FILE` | Audit the actual payload tree, then produce a tar archive |

Success exits 0. Failure exits nonzero, identifies the affected
component/input/file on stderr and leaves no successful final output. Tests
assert that identity, not an exact sentence or nonzero status number. Malformed
arguments fail before acquisition, recipe execution or output creation.

### Support lock schema

All fields below are mandatory. Reject duplicate JSON keys, unknown fields,
wrong types and unresolved references. Boolean values are not integers.

| Type | Definition |
|---|---|
| `Id` | `[a-z0-9][a-z0-9._-]*` |
| `PackageName` | Unqualified Debian package name: `[a-z0-9][a-z0-9+.-]+`; no architecture suffix, slash, underscore, whitespace or shell syntax |
| `Rel` | Nonempty canonical POSIX relative path; no empty, `.` or `..` component, backslash, control character or `!` |
| `Hash` | SHA-256: 64 lowercase hexadecimal characters |
| `Commit` | Complete Git object ID: 40 or 64 lowercase hexadecimal characters, matching the repository's object format |
| `URL` | Absolute HTTPS URL without credentials or fragment |
| `Version` | Nonempty exact version string, not a branch, wildcard or version constraint |

| Object | Exact fields |
|---|---|
| Root | `schema: 1`, `sources: map<Id, Source>`, `inputs: map<Rel, Hash>`, `build: Build`, `preparation: Preparation`, `output: Output` |
| Git source | `kind: "git"`, `url: URL`, `commit: Commit`, `licence: Licence` |
| Archive source | `kind: "archive"`, `url: URL`, `version: Version`, `bytes: positive integer`, `sha256: Hash`, `licence: Licence` |
| Licence | `id: nonempty string`, `notice: Rel` within the source, `source_required: boolean`, `review: Rel` referencing `inputs` |
| Build | `image: digest-pinned Ubuntu base registry image`, `environment_recipe: Rel`, `ubuntu: "22.04"`, `packages: map<PackageName, Version>`, `tools: map<Id, Version>`, `argv: nonempty array<string>` |
| Preparation | `archive: Id`, `script: Rel`, `patches: array<Rel>`, `helpers: map<Id, Helper>`, `inventory: map<Rel, Hash>` |
| Helper | `source: Id`, `path: Rel` within the preparation payload |
| Output | `architecture: "x86_64"`, `python_abis: ["cp310", "cp312"]`, `runtimes: ["nix13", "nix20"]`, `runtime_packages: map<Ubuntu, map<PackageName, Package>>`, `builder_prefixes: array<absolute path>`, `forbidden_sha256: array<Hash>` |
| Package | `version: Version`, `sonames: array<string>`, `paths: array<absolute path>` |

Additional constraints:

- `sources.lha` and `preparation.helpers.lha` are required; the helper references
  source `lha`. The preparation archive references the NDK archive source.
- Script, patch and review paths reference hashed `inputs`. Inventory paths
  are SDK-relative. Sources, tools, preparation helpers and inventory are
  nonempty; arrays do not contain duplicate entries.
- Runtime-package cohorts are exactly `22.04` and `24.04`.
- Use actual unqualified package names, including valid `+` characters in
  `g++` or `libstdc++6`, only in package maps/provider references. Source,
  helper and tool identifiers remain `Id`; do not broaden that grammar or
  introduce aliases that cannot be checked against actual dpkg ownership.
- Ownership means the actual strictly resolved loaded file, not a symlink's
  lexical spelling. Only genuine `dpkg-query` owner records for that file
  qualify; successful diversion commentary alone does not. A merged-/usr
  fallback must resolve to the same file and is allowed only when no genuine
  canonical owner exists. Wrong or missing ownership remains a failure.
- Images end in `@sha256:<Hash>`; no mutable image tag alone qualifies.
- `build.image` identifies the Ubuntu base, not the prepared execution image.
  `build.environment_recipe` references a hashed Dockerfile in `inputs`.
- Build argv starts with `/bin/sh`, then `/sources/inputs/<tracked-script>`.
  That script is a hashed input. Remaining arguments are literal strings,
  not shell expressions. NUL/control characters in arguments are rejected.
- Forbidden hashes identify reviewed excluded content, including NDK-derived
  headers; the preparation archive's hash is automatically forbidden too.
- Structural validity does not establish a complete real dependency closure,
  redistribution permission or platform compatibility. Those remain measured
  proof obligations. A miniature fixture is never a release candidate.

### Source staging and offline builds

| Staged path beneath `--sources` | Contents |
|---|---|
| `checkouts/<id>/` | Exact Git checkout for a Git source |
| `archives/<id>` | Exact archive bytes, in a regular file |
| `inputs/<tracked-relative-path>` | Regular file with the locked input hash |

Verify actual checkout HEAD, object format, origin URL and clean state;
tracked changes and extra untracked/ignored build inputs fail. Verify all
archive lengths/hashes and input hashes. Reject escaping links and missing
inputs before executing a recipe. Staging acquisition occurs before this
offline build boundary; the build command never fetches sources.

Git worktree files must contain the exact authenticated stored blob bytes,
not text converted by checkout attributes (including built-in `ident` or line
ending conversion). Verification never rewrites inputs or executes filters.
Acquisition may canonicalise proven checkout conversions in maintainer-owned
staging before this boundary. Preserve original evidence and repeat affected
reference proof when canonicalising previously measured source bytes.

The caller must be non-root. AR-32 permits a locally prepared execution image
without publishing it to a registry. Pin the Ubuntu base digest and all
environment recipe/package/source inputs. Acquire and verify those inputs
first, then explicitly prepare the environment using the existing Docker CLI
and the hashed Dockerfile. Preparation can install staged packages and build
declared build tools; it occurs before the offline support build. It must not
silently acquire undeclared input or change the selected source cohort.

Supply the resulting immutable image ID reported by Docker as `--image-id sha256:<Hash>`
to the support and wheel commands. Mutable tags are not execution identities.
Before the recipe, inspect that ID, require Linux/amd64 and verify:

- inspected `Id` equals the supplied ID;
- the base image's root filesystem layer list is a prefix of the prepared
  image's layer list;
- label `org.clorraine.base-image` equals `build.image`;
- label `org.clorraine.environment-recipe-sha256` equals the locked Dockerfile
  hash.

The preparation procedure sets those two labels from verified inputs and
records the actual installed tool/package versions, which must match the lock.
Labels identify build inputs; they are not independent authentication or a
substitute for actual environment/source proof. No registry, third workflow,
image-management framework or new preparation service is introduced.

The offline support build uses the inspected immutable execution ID, not the
base reference, with `--pull=never`, `--network=none`, the caller's UID/GID,
read-only staged sources at `/sources`, and owned private output staging at
`/work`. Use `/work` as CWD and the literal locked argv. Mount no Docker socket
or unrelated host directories. Dependencies/backends are already present;
no package installation occurs in this support-build recipe.

Recipes are reviewed, hashed build inputs. They must propagate command errors;
acquisition commands are prohibited after staging. An undeclared fetch cannot
succeed through the isolated network, and a failing recipe cannot publish
success. Unit tests may replace the external Docker process to inject a failed
fetch and inspect isolation arguments; that is not evidence of real isolation.
The real build proof must exercise network isolation and failure propagation.

The output directory must be absent or empty, disjoint from source staging,
and not a symlink. Refuse existing nonempty outputs without mutation.
The recipe writes `/work/payload/`. Only after successful execution and valid
tree inspection publish `DIR/payload/` and `DIR/provenance.json`. On failure,
remove only this invocation's staging and any partially published output;
neither final artifact may remain. Do not reuse prior success markers.

Provenance has exactly these fields:

| Field | Value |
|---|---|
| `schema` | `1` |
| `lock_sha256` | SHA-256 of the exact lock file bytes |
| `image`, `image_id` | Locked Ubuntu base reference and actual prepared execution image ID |
| `environment_recipe_sha256` | Hash of the locked environment Dockerfile |
| `argv`, `exit_code` | Executed argument array and `0` |
| `sources` | Map of source ID to actual commit or archive SHA-256 |
| `inputs` | Map of tracked input path to actual SHA-256 |
| `files` | Payload-relative map to `{type: "file", sha256: Hash}` or `{type: "symlink", target: string}` |

### File-based payload audit

The audit JSON has exactly these fields and record shapes:

| Object | Exact fields |
|---|---|
| Root | `schema: 1`, `lock_sha256: Hash`, `components: map<Id, Component>`, `files: map<Rel, File>`, `native: map<string, Native>` |
| Component | `review: Rel`, `notices: nonempty array<Rel>`, `source: array<Rel>` |
| Regular file | `type: "file"`, `sha256: Hash`, `origins: nonempty array<Id>`, `role: Role` |
| Link | `type: "symlink"`, `target: string`, `origins: nonempty array<Id>`, `role: "link"` |
| Native | `format: "elf", "ar" or "amiga-object"`, `machine: "x86_64" or "m68k"`, `soname: string or null`, `needed: map<SONAME, Provider>`, `glibc: array<version string>` |

Roles are `program`, `header`, `library`, `wheel`, `preparation`, `notice`,
`review`, `source`, and `link`. Native keys identify payload files or
`wheel-path!member-path`; each side must be a `Rel`. Providers are
`package:<PackageName>` or `file:<payload-or-native-member-path>`.

The audit must inspect the actual tree, without following directory links:

1. Require exact file/link coverage, hashes and targets. Directories are implicit
   parents; reject special files, unexpected empty directories and absolute,
   escaping, dangling or cyclic links.
2. Resolve every origin to a locked source and component record. Review bytes
   must match the source licence's locked review-input hash. Notice/source
   paths name present hashed regular files with their matching roles and origin.
   Required corresponding source cannot be empty.
3. Reject NDK-origin headers wherever installed, the NDK archive and reviewed
   forbidden hashes. Added unaccounted libraries/media are failures even when
   placed outside a directory normally used for them.
4. Inspect actual ELF/ar content, including native wheel members. Require
   complete, accurate native records; check architecture, SONAME and dependency
   accounting. Package providers exist in both Ubuntu cohorts; bundled providers
   name actual audited files/members. Host GLIBC requirements may not exceed 2.35.
   Amiga static archives contain target objects, not host ELF: inspect their
    members with the packaged `m68k-amigaos-objdump`, retaining the `ar`/`m68k`
    record. Do not treat a host `readelf` failure on `a.out-amiga` as a bad SDK.
    HUNK-format archives also use `ar`; Unix archive magic is not a prerequisite.
    Standalone Amiga HUNK objects, including required startup `.o` files, use
    `amiga-object`/`m68k`, null SONAME, empty `needed` and empty `glibc`.
    Inspect actual object format/architecture with the packaged cross-objdump;
    do not omit their native records, infer type solely from a filename, label
    them ELF/archives, or accept a record that disagrees with the actual bytes.
5. Reject executable builder-prefix dependencies, including absolute script
   shebangs and builder RPATH/RUNPATH. Loader-relative `$ORIGIN` paths must resolve
   internally. A prefix quoted in corresponding source or a notice is not by
   itself an executable dependency.
6. Exclude unneeded SDK programs, FS-UAE and licensed media through reviewed
   source/content evidence, not a directory-name assumption.

Origin labels and a review file do not themselves establish permission.
Real origin tracing and redistribution review remain mandatory. Packaging
refuses an existing output file or symlink, writes a sibling temporary tar,
and renames only after a successful audit. Sort relative members, normalize
ownership/timestamps, and preserve only safe read/write/execute modes and
internal relative links.

### Valid miniature lock and audit fixture

This deterministic fixture is executable test data, not toolchain evidence.
Define `H(data)` as SHA-256 and `J(value)` as sorted, compact UTF-8 JSON plus one
newline. Use these exact byte strings:

```python
B = b"#!/bin/sh\nexit 0\n"
L = b"Synthetic fixture licence.\n"
R = b"Synthetic review; not publication approval.\n"
N = b"Synthetic excluded NDK-derived header.\n"
S = b"#!/bin/sh\nset -eu\ncp -R /sources/checkouts/lha/payload /work/payload\n"
E = b"ARG BASE_IMAGE\nFROM ${BASE_IMAGE}\n"
```

Create a real Git repository containing `LICENSE=L` and the four payload files
below. Set `bin/lha` executable. Commit with fixed author/committer identity and
dates; `C` is its actual complete HEAD. Its origin is
`https://example.invalid/lha.git`. `A` is deterministic tar bytes containing
`LICENSE=L`, with zero ownership/timestamps. Stage the repository, `A` at
`archives/seed`, and `recipe.sh=S`, `prepare.sh=B`, `review.txt=R`,
`environment.Dockerfile=E` under `inputs`.

```python
lic = {"id": "LicenseRef-Fixture", "notice": "LICENSE",
       "source_required": True, "review": "review.txt"}
pkg = {"version": "1", "sonames": [], "paths": ["/bin/sh"]}
lock = {
    "schema": 1,
    "sources": {
        "lha": {"kind": "git", "url": "https://example.invalid/lha.git",
                "commit": C, "licence": lic},
        "seed": {"kind": "archive", "url": "https://example.invalid/seed.tar",
                 "version": "1", "bytes": len(A), "sha256": H(A), "licence": lic}},
    "inputs": {"recipe.sh": H(S), "prepare.sh": H(B), "review.txt": H(R),
               "environment.Dockerfile": H(E)},
    "build": {
        "image": "docker.io/library/ubuntu@sha256:" + "0" * 64,
        "environment_recipe": "environment.Dockerfile",
        "ubuntu": "22.04", "packages": {"dash": "1"},
        "tools": {"python3": "3.10.0"},
        "argv": ["/bin/sh", "/sources/inputs/recipe.sh"]},
    "preparation": {
        "archive": "seed", "script": "prepare.sh", "patches": [],
        "helpers": {"lha": {"source": "lha", "path": "bin/lha"}},
        "inventory": {"include/fixture-ndk.h": H(N)}},
    "output": {
        "architecture": "x86_64", "python_abis": ["cp310", "cp312"],
        "runtimes": ["nix13", "nix20"],
        "runtime_packages": {"22.04": {"dash": pkg}, "24.04": {"dash": pkg}},
        "builder_prefixes": ["/sources", "/work"], "forbidden_sha256": [H(N)]}}
audit = {
    "schema": 1, "lock_sha256": H(J(lock)),
    "components": {"lha": {
        "review": "reviews/lha.txt", "notices": ["notices/lha.txt"],
        "source": ["sources/lha.sh"]}},
    "files": {
        p: {"type": "file", "sha256": H(b), "origins": ["lha"], "role": r}
        for p, b, r in [
            ("bin/lha", B, "program"), ("notices/lha.txt", L, "notice"),
            ("reviews/lha.txt", R, "review"), ("sources/lha.sh", B, "source")]},
    "native": {}}
```

Serialize both with `J`. The reserved URLs, synthetic versions and dummy image
digest cannot supply real build evidence. Positive lock/source/audit tests use
these bytes and real filesystem/Git operations; build-boundary unit tests
replace only external Docker responses. The real archive tests remain separate.
For external-boundary fixtures, the base's inspected image ID is
`sha256:` plus 64 `2` characters; the prepared ID is `sha256:` plus 64 `1`
characters. Its root filesystem layers extend the base's layer list, and its
two preparation labels match the fixture base reference and Dockerfile hash.
Neither fake image is claimed to be available or usable for real builds.

## Payloads

| Asset | Contents | Exclusions |
|---|---|---|
| Amiga support | GCC C compiler, binutils, compiler helper executables, libgcc, both libnix runtime variants and their required include/library closure | Unneeded SDK programs such as gdb/vbcc/vasm/vlink; ROM/Workbench/FS-UAE |
| NDK preparation inputs | Prebuilt LHA, sfdc/fd2sfd and any actually required helper; audited patches and preparation logic; inventory of expected generated output | Upstream NDK archive and modified NDK header trees |
| vamos wheel sets | Locked amitools and full runtime dependency closure for CPython 3.10 and 3.12, plus required notices/source | Source-build fallback or dependency-index access during installation |
| Compliance/provenance | Complete source versions, build recipes, licences/notices and corresponding source where required | Private credentials and licensed media |

The payload audit is file-based: determine which installed headers/libraries
originate in the NDK before exclusion. Do not assume all files under
`m68k-amigaos/include` are redistributable because `ndk-include` was removed.
Any permitted redistributed target library remains subject to its own terms.
Missing permission or unavailable corresponding source blocks that asset.

AR-33's later explicit user direction permits including only the ordinary
`libamiga.a` from the pinned build repository (297,694 bytes; SHA-256
`10d78b6d303a99677c0bf29a8e1653d62669f69f395691646393020325084021`)
while the user handles its legalities. Preserve its unresolved source/terms
status and known provenance; do not invent a licence or claim corresponding
source has been established. Record the included file as a hashed input and
in the SDK inventory. The base-relative archive and other unknown-origin files
are not covered by this direction. Local implementation/candidate assembly can
continue; separate public promotion approval and all other proof gates remain.

Use ordinary tar archives for directory payloads and retain `.whl` files as
wheel archives. Emit stable relative members and only internal relative links;
the installer validation rules are in [03-02](03-02-installation.md).
No absolute build prefix, RPATH or installed script shebang may point into the
builder. Runtime loader dependencies must resolve from the declared Ubuntu
packages or audited bundled libraries; record SONAME and required GLIBC versions.

## NDK preparation

The installer obtains the archive using the hash recorded in [02](02-current-state.md).
An audited preparation command takes only an archive path and a fresh destination
SDK. It verifies the archive before invoking prebuilt helpers, safely extracts
the allowed files, applies the locked patches, generates required headers and
inlines, prepares both ordinary and 1.3 include trees, and preserves notices.

Preparation is file transformation, not compilation of installation dependencies.
It must not invoke a host compiler, build system, package manager or network
client. Ship helpers for every necessary transformation; omit unused helpers.
The helper invocation uses argument arrays and private staging directories.

Derive the expected file inventory and content hashes from the same fully
locked reference build. Compare relative paths/content, excluding only metadata
proved irrelevant to compilation (timestamps and owner IDs). An unexplained
content mismatch blocks the proof. Include 1.3 headers, inline wrappers, netinclude
interactions and headers needed by every shipped hosted-amiga shim.

### Preparation process interface

```text
python3 tools/distribution/prepare_ndk.py --archive PATH --destination DIR
```

These are its only two required arguments. Resolve `preparation.json`, helpers
and patch/input files relative to the executing script, not CWD or a checkout.
The destination is an inactive, fresh unprepared SDK tree, not a live install.
Reject a prepared marker or mismatching baseline. Check archive length/hash
before invoking any helper and check helper/input hashes before executing them.
No compiler, build system, package manager or network client is permitted.

`preparation.json` has the following mandatory fields; reject unknown fields,
duplicates, wrong types and unsafe paths using the support contract's types.

| Field | Value |
|---|---|
| `schema`, `lock_sha256` | `1`, hash of the exact support lock |
| `archive` | `{source: Id, bytes: positive integer, sha256: Hash}` |
| `base` | Complete unprepared SDK tree inventory using provenance file/link records |
| `helpers` | Map of helper ID to `{path: Rel, sha256: Hash}` within this preparation payload |
| `inputs` | Map of preparation-payload-relative patch/input paths to hashes |
| `inventory` | SDK-relative regular-file paths and hashes, identical to the lock's preparation inventory |
| `notices` | Nonempty list of notice paths present in `inventory` |

The final SDK is `base` overlaid with `inventory`, plus `.ndk-prepared.json`.
Unchanged baseline paths must retain their bytes/links; unexplained generated
paths fail verification. Write the marker last, with exactly `schema: 1`,
`lock_sha256`, `ndk_sha256`, and `inventory_sha256`. The last hash covers sorted,
compact UTF-8 inventory JSON plus one newline. Consumers check actual files;
marker presence alone is not proof. Failure leaves no marker or outside writes;
the caller discards the incomplete inactive destination.

One narrow public pure function in the existing preparation module permits
unsafe-member tests without depending on LHA's textual listing syntax:
`validate_archive_members(members) -> None`. Each member has exactly `path`,
`type` (`file`, `directory`, `symlink`) and `target` (null except for a link).
Raise `ValueError` naming the offending member for unsafe paths, duplicate
destinations, unsupported types, dangling/escaping/cyclic links or extraction
through symlink parents. Validate the complete listing before extraction into
private staging. This utility does not replace the real helper/extraction proof.

## vamos

Build the native wheels on compatible build hosts for each supported Python
ABI. A newer Python build host does not establish GLIBC 2.35 compatibility:
inspect and test every native extension's dependency/version closure.
Record wheel tags and exact bytes in the support metadata.

Installation creates a private venv using the matching Ubuntu Python. Invoke
that environment's Python with the installed amitools module/entry point;
do not rely on pip's absolute entry-script shebang after moving the root.
The command must use its own site-packages and ignore unrelated user
site-packages. The lock includes pip bootstrap/runtime inputs actually needed
by the offline installation; do not download a build backend during install.

### Wheel process interface

```text
tools/distribution/build-wheels.sh --lock PATH --sources DIR --image-id ID --abi cp310|cp312 --output DIR
```

The accepted ABI values are exactly `cp310` and `cp312`. Reuse the existing
Build record, source verification, image isolation and output-publication
contract. Its hashed build script receives the fixed literal suffix
`--wheels <abi>` after `build.argv`; the ordinary support build has no suffix.
The prepared image must already contain the selected interpreter and build
backends, with actual versions recorded under `build.tools`/`build.packages`.
There is no new task graph, alternate source acquisition or image pull.

### Lock finalisation order

Resolve immutable source records first and retain them as clearly labelled
research evidence, not a supposedly eligible partial lock. Produce the hashed
environment/build recipes, source/licence reviews and fixed-input reference
tree before writing and validating the complete real lock. Reference work uses
the resolved source/environment inputs and explicit recorded commands; it
does not claim to have consumed a final lock whose inventory does not yet exist.
The subsequent support scripts reproduce those same commands using the final
lock. No placeholder hashes, dummy inventories or temporary schema bypass is
allowed. This is the manual-proof-before-automation boundary, not another build
or metadata framework.

AR-34 further separates the already measured private reference from final
eligible-lock completion: implement and verify the required tracked recipe/
preparation producers first, using specification fixtures and explicit manual
fixed-input proof where a real lock does not yet exist. Then finalise and
validate the complete lock and run the real public commands against it before
packaging/acceptance. Do not invent hashes, point preparation at a compiling
reference script, or describe fixture/manual inputs as a final lock.

Successful output is `DIR/payload/` and `DIR/provenance.json`, with the exact
executed argv. The payload holds `wheels/`, required notices/source and
`wheels.json`. Failure leaves neither final output. The latter JSON has exactly:

| Field | Value |
|---|---|
| `schema`, `lock_sha256`, `abi` | `1`, exact support-lock hash, `cp310` or `cp312` |
| `packages` | Map of normalized lowercase Python package name to the record below |
| Package record | `version: Version`, `source: Id`, `purpose: "runtime" or "bootstrap"`, `filename: Rel`, `bytes: positive integer`, `sha256: Hash`, `tags: nonempty array<string>` |

Source identities resolve to the lock's exact inputs. Recorded versions/tags
must match actual wheel metadata and bytes; archive-source versions must agree
with the lock. Native wheel members are subject to the existing payload audit.
The measured complete runtime set includes `amitools`, `machine68k`, `greenlet`
and their resolved dependencies; bootstrap includes pinned `pip`. Reject
sdists, wrong platform/ABI tags, duplicates and missing required packages.
No new mandatory root lock field is added: the valid lock fixture and already
authored lock tests remain unchanged.

For offline proof, create a venv using the matching Ubuntu Python/venv, then
install the verified local wheel set with pip's `--isolated`, `--no-index`,
`--only-binary=:all:`, `--no-deps` and `--require-hashes` controls. Run `pip check`
afterwards. Record Ubuntu-provided venv bootstrap versions too. Resolve the
unique `vamos` console entry from the installed `amitools` distribution via
standard-library `importlib.metadata`, invoke it using the venv's Python `-I`,
and propagate status; do not rely on generated absolute-shebang launchers.

## Manual proof before automation

Build candidate archives from the lock, then use fresh Ubuntu 22.04 and 24.04
environments with only the documented system prerequisites. Builds may use
containers; installation and execution run as an ordinary user. During proof,
the repository, builder trees and original `/opt/amiga` must be unavailable.
Use real filesystem isolation, not a changed PATH alone.

Before the native driver exists, use emitted C produced by the current `clc`
and the existing driver flag sets to prove cross compilation/linking. Test host
output and both Amiga runtimes through the packaged vamos. Compile all shipped
hosted-amiga shim families against the prepared headers, using their existing
minimum-profile guards. Run portable fixtures through vamos; GUI/device examples
are build-only here and run under the later FS-UAE gate.

Move the prepared root to a second location, including spaces, and repeat.
Record exact archives, commands, exit codes, outputs, installed system packages
and loader audit. Use the existing corpus and representative Amiga examples;
version output alone is not proof.

The closed prerequisite list is a measured deliverable of this phase. Starting
categories are GCC/C headers, CPython/venv, download/CA tools and support runtime
libraries (AR-27). An unexpected library is either added with evidence within
those categories or treated as a scope-changing blocker. Do not silently bundle
another host C compiler or Python runtime.

### Explicit artifact test inputs

Use ordinary unittest class selection, not another test runner. The
`test_support_spec.py` entry exposes contract/preparation test classes for local
fixtures and `SupportArtifactAcceptance` for the mandatory real proof.
Keep added spec/helper files focused rather than growing the entry beyond
the test-file size convention. Existing tests remain unchanged.

```text
python3 -m unittest discover -s tools/distribution/tests -p test_support_spec.py -v
CLORRAINE_SUPPORT_ACCEPTANCE=/absolute/acceptance.json python3 -m unittest discover -s tools/distribution/tests -p test_support_spec.py -v
```

Whole-file discovery includes real acceptance; absent inputs must fail rather
than skip. For a local-only run, select the contract/preparation classes
explicitly from the test directory. Unit green never substitutes for acceptance.

`acceptance.json` uses absolute paths and has exactly these fields:

| Field | Required value |
|---|---|
| `schema`, `ubuntu`, `python_abi` | `1`; `22.04`/`cp310` or `24.04`/`cp312`, checked against the actual host |
| `lock` | `{path, bytes, sha256}` for the actual support lock |
| `amiga`, `preparation`, `wheels` | Each `{path, bytes, sha256, audit}`: archive and its audit JSON path |
| `ndk_archive` | Path to the NDK archive identified by `lock.preparation.archive` |
| `fixtures` | Directory holding fixed emitted-C units and existing stdout goldens below |
| `work_root` | Writable, initially empty directory owned by this invocation |

Verify archives before extraction; audit their actual trees through the public
packager. No unverified archive path or metadata is executable input. Acceptance
runs non-root with network disabled, source/build trees and `/opt/amiga`
inaccessible. Only candidate inputs, copied tests/fixtures and declared system
prerequisites are available. Record actual environment/isolation and commands.
The Amiga archive's payload root is the unprepared SDK (`bin/` and target
include/library directories). The preparation archive's root contains
`prepare_ndk.py`, `preparation.json` and its referenced helpers/inputs; the
wheel archive's root contains `wheels.json` and `wheels/`.

The fixture directory is passive data, not a command/flag manifest. Before
proof, the maintainer emits `units/<profile>/<case>.c` using current `clc` for
`host`, `hosted-amiga`, `hosted-amiga-13`, and cases `hello`, `file-copier`,
`string-parser`, `buffer-container`, `volatile-mmio`. `goldens/<case>.txt` copies
the existing corresponding expected output; hello is exactly
`Hello, Amiga, from Clorraine!\n` from `examples/hello/hello.clo`.
Record emitter revision and fixture hashes in the proof evidence. The compiler
and its sources need not be available in the isolated acceptance environment.

Compile with fixed baseline debug flags: host `-std=c99 -O0 -g -Wall -Wextra
-Werror`; Amiga adds `-mcrt=nix20` or `-mcrt=nix13`, `-m68000 -fwrapv` before
the mode/warning flags, and `-lamiga -lm` after the C input. These mirror
`spec/17-codegen.md` §13 and the driver's host table. Execute host binaries
directly and Amiga binaries through the packaged vamos, each in an isolated
case directory; compare stdout after CR removal and require exit 0.
`volatile-mmio` is Amiga-only because it imports `std.amiga.custom`; omit its
host unit/run rather than broadening the host profile's module surface.
The emitted hosted-amiga units include the base, 2.0, optional, optional UI,
services, network and RTG suffix families (including embedded P96 definitions);
the 1.3 units retain their minimum-profile exclusions. No shim is hand-edited
or compiled separately out of its normal emitted-unit context.

Real preparation checks its complete inventory/notices. Corrupt NDK bytes,
remove a helper, and exercise hostile members without permitting a success
marker or outside writes. Offline wheel proof must reject missing machine68k
or greenlet, wrong ABI and source-distribution substitutions without attempting
compilation. Load all shipped host programs/native extensions, inspect actual
dependencies against declarations, then repeat preparation/build/run after
moving the assembled SDK/venv root to a new path containing spaces. The old
prefix must be unavailable. Version-only output is never sufficient.

## Failure and review

| Failure | Required response | Authority |
|---|---|---|
| Cannot identify the tested input revision | Stop support proof; report component and evidence gap | AR-18 |
| Source/licence obligation cannot be met | Block asset publication and dependent work | AR-18/22/28 |
| Ubuntu, Python, NDK or relocation proof fails | Fix within the approved contract; otherwise seek a ruling | AR-12/18/24 |
| Runtime dependency missing | Fail and identify the package; never compile it during installation | AR-26/27 |

Specification cases: ST-1–ST-10 and ST-60. Independent review must include the Amiga
platform lens and a payload audit tracing every shipped helper/library to its
source and licence.
