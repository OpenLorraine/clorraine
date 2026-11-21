# Installer, metadata and activation

> **Parent**: [Index](00-index.md)
> **Authority**: AR-15/18/20/22/26/27/28

## Entry and trust

The distribution release provides `install.sh` and a release-specific
`install.pyz` Python zip application. The shell entry pins that helper's SHA-256
and verifies it before invoking system Python in isolated mode (`-I`). The
helper uses only the standard library. Keep the implementation in `tools/setup/` alongside,
but separate from, the Docker setup tool. Reuse small existing utility patterns
where appropriate; do not change `make setup` into an unrelated installer.

The `.pyz` contains its `__main__` entry, `install_native`, `native_archive`,
`native_state` and the shared distribution manifest validator, including all
runtime data those modules need. Preserve package imports inside the archive;
no import may require the source checkout, current directory or `PYTHONPATH`.
NDK preparation remains in the separately verified support payload.
`tools/distribution/package_release.py` assembles this helper, its hash-pinned
shell entry and the manifest from the same production inputs used by workflows.
Test the shipped shell entry with the checkout inaccessible. Packaging is a
local Phase 3 prerequisite, not work postponed until workflow automation.

Public interface:

```text
sh install.sh [--root PATH] [--release TAG]
```

Default root is `$HOME/.clorraine`; omitted release means the latest published
distribution. Exact release tags follow `v1.2.0` / `v1.2.0-rN` and later release
versions of that form. Reject unrecognised options, missing values, path-like
tags, unknown manifest schemas and incompatible platform/interpreter cohorts.
The default release lookup excludes drafts, support releases and compiler/editor
component releases. Exact selection also accepts distribution tags only. A
component release is never a distribution merely because GitHub lists it first.

Download only over HTTPS with normal certificate verification. Treat redirects
as new HTTPS destinations, never as permission to downgrade transport.
Use the release identity returned by lookup consistently for the whole run.
Verify helper bytes before executing them and all other assets before extraction
or use. Hashes establish integrity relative to the trusted release publisher;
they do not provide an independent signature.

## Metadata contract

JSON schema version 1 has these required concepts; the packager and installer
share the schema definition and fixtures in `tools/distribution/`:

| Object | Required fields |
|---|---|
| Distribution | `schema`, `release`, `core_version`, `extension_version`, `platform`, `assets`, `support`, `prerequisites` |
| Platform | Linux, x86_64, supported Ubuntu baselines and CPython ABIs |
| Asset | Unique component identity, HTTPS URL, byte length, SHA-256, kind and expected payload root |
| Support | Exact Amiga, preparation and wheel-set identities, supported profiles and prepared-output inventory identity |
| Prerequisites | Validated OS/interpreter cohort and system package requirements from support proof |

Manifest payload URLs refer to separately publishable support and
compiler/editor component releases, including the VSIX. Bootstrap/helper
integrity is owned by the shell's pinned helper hash; they are distribution
entry assets, not self-referenced payloads requiring pre-publication URLs.
The release topology and promotion order are owned by
[03-04](03-04-editor-and-release.md#payload-identities-and-local-producers).

Validate JSON duplicate keys and types before selecting any field. Reject
missing/unknown schema fields, duplicate identities, invalid hashes/lengths,
unsupported platform combinations and inconsistent core/VSIX/support metadata.
Treat every value as data: no `eval`, shell sourcing or command strings from
the manifest. Package names used in printed guidance must be validated package
identifiers, not executable metadata.

An asset identity can be reused only with exactly the same bytes. A distribution
revision can refer to previous core assets; it cannot mutate them. Record the
selected manifest bytes/hash and component identities with the installation.
Secrets are not required for public installs; authenticated draft testing is a
maintainer-only acceptance operation described in [03-04](03-04-editor-and-release.md).

### Executable manifest contract

The shared public Python boundary is
`tools.distribution.manifest.parse_manifest(data: bytes) -> dict`.
It returns the validated, otherwise unchanged JSON object and raises
`ManifestError` (a `ValueError` subclass) on invalid input. It performs structural
and cross-reference validation without fetching, extracting or executing anything.
The installer/producer additionally bind it to actual payload bytes as below.
No public native-installer validation-only flag is added.

Use `Hash`, `PackageName` and `Rel` from the support contract. An `Identity`
matches `[a-z0-9][a-z0-9-]*`; a version has three decimal components with no
leading zero except zero itself. A distribution tag is `v<version>` or
`v<version>-r<positive decimal without leading zero>`, with version at least
`1.2.0`. Core and extension versions may remain `1.1.2` in local Phase 3 proof;
neither is inferred from the distribution tag.

Parse UTF-8 JSON, reject duplicate keys at every depth, nonfinite numbers,
booleans in integer fields and documents over 16 MiB. All record fields below
are exact: unknown/missing fields fail. Arrays/maps described as sets have no
duplicates; empty sets fail unless expressly allowed.

| Object | Exact fields and values |
|---|---|
| Manifest | `schema: 1`, `release: distribution tag`, `core_version: version`, `extension_version: version`, `platform: Platform`, `assets: array<Asset>`, `support: Support`, `prerequisites: map<Ubuntu, array<PackageName>>` |
| Platform | `os: "linux"`, `architecture: "x86_64"`, `cohorts: {"22.04":"cp310","24.04":"cp312"}` |
| Asset | `identity: Identity`, `kind: Kind`, `url: HTTPS URL`, `bytes: positive integer`, `sha256: Hash`, `root: string or null`, `entries: nonnegative integer`, `expanded_bytes: nonnegative integer`, `audit: Identity or null` |
| Support | `identity: Identity`, `lock: Identity`, `amiga: Identity`, `preparation: Identity`, `wheels: {"cp310":Identity,"cp312":Identity}`, `ndk: Identity`, `profiles: ["hosted-amiga","hosted-amiga-13"]`, `prepared_inventory_sha256: Hash` |

`assets` has exactly twelve entries: one each of `core`, `vsix`, `amiga`,
`preparation`, `wheels-cp310`, `wheels-cp312`, `support-lock`, `ndk`, and four
`support-audit` entries. Every identity is unique. The four support archives
reference four distinct audit identities; every audit is referenced once.
All other assets have `audit: null`. `support` references each corresponding
kind, including both wheel ABIs, and the single `support-lock` and `ndk`.
Core and VSIX are identified by their unique kinds, not by array position.
No unreferenced or wrong-kind asset is accepted.

For core/Amiga/preparation/wheel tar assets, `root` is `"."` (no enclosing
archive wrapper). For VSIX it is `"extension"`; the ZIP may also contain its
normal `[Content_Types].xml` and `extension.vsixmanifest` packaging records.
These roots describe archive contents, not a user-controlled installation
destination. Record exact member count and sum of ordinary member lengths in
`entries`/`expanded_bytes`, including directory and link headers in the count.
Raw lock/audit/NDK assets have `root: null`, `entries: 0`, `expanded_bytes: 0`.
NDK is passed only to the verified preparation process, never treated as tar.
All integer values are at most `2**63-1`; local hard limits still bound actual
reads and extraction before using any declaration to allocate memory.

URLs are absolute HTTPS with a nonempty host, no credentials, fragment,
backslash, whitespace or control characters. Ports, when present, are valid
numeric TCP ports. Enforce the same rules on each redirect before requesting
it, with a finite redirect count; do not automatically follow a downgrade and
then inspect its final URL. No value is an executable expression.

The producer emits GitHub URLs under the fixed repository
`https://github.com/OpenLorraine/clorraine/releases/download/`:
core and VSIX under `components-<components identity>/core.tar` and
`clorraine.vsix`; lock/audits/support archives under `support-<support identity>/`
using the filenames in the release-producer contract. The NDK URL and exact
length/hash come unchanged from the locked preparation archive source.
Structural validation permits other safe HTTPS hosts for upstream inputs and
redirects; it does not pretend a hostname alone authenticates arbitrary content.

After download, require these additional relationships before use:

- Exact declared length/hash for every input. The support lock and all four
  audits use the existing support schemas; audit `lock_sha256` equals the
  hash of those exact lock bytes. Full archive file/link coverage and hashes
  match the respective audit before extraction.
- Core `core.json` and all its member records use the established core contract;
  its version equals `core_version`. VSIX `extension/package.json` identifies
  `openlorraine.clorraine`, agrees with `extension_version`, and agrees with
  the supplied current extension's package metadata during local production.
- Both wheel manifests equal the support lock hash and their respective ABI;
  preserve the complete five-package runtime/bootstrap manifest contract.
- `support.prepared_inventory_sha256` is SHA-256 of sorted compact UTF-8 JSON
  plus newline for `lock.preparation.inventory`. NDK bytes match that lock's
  preparation archive source. Profiles/ABI cohorts agree with the locked
  nix13/nix20 and cp310/cp312 support set.
- Prerequisite keys are exactly `22.04` and `24.04`. Each package set equals
  that cohort's locked runtime-package names union `gcc`, `libc6-dev`,
  `python3`, `python3-venv`, `curl`, `ca-certificates`. Package guidance does not
  pin users to old security-update versions. Check installed packages, usable
  host C compiler/headers, venv capability and required runtime loading.
  A small host smoke program is not installation-dependency compilation.

Only the matching wheel archive/audit needs downloading for an installation;
the manifest still validates both ABI references. Source/licence files already
inside audited support archives and core/VSIX payloads remain intact.

### Lookup and bootstrap handoff

The release-specific shell contains literal helper URL, positive byte length
and SHA-256 produced together with `install.pyz`. Its helper URL is
`.../releases/download/<shell distribution tag>/install.pyz`. It fetches that
helper using `curl` with normal CA verification, HTTPS-only initial/redirect
protocols, finite connect/overall timeouts and failure-on-HTTP-error. It checks
length and SHA-256 before `python3 -I <private-helper.pyz> <literal user args>`.
Do not use `eval`, source a downloaded file or interpolate args into code.
Use private mode-0700 bootstrap storage and clean only that run's storage.
The pinned helper selects `--release` or the latest distribution; selecting
another distribution does not silently replace the shell's pinned helper.
No helper version bump or public URL existence is claimed by local proof.

The helper uses the fixed GitHub API repository
`https://api.github.com/repos/OpenLorraine/clorraine`. Exact lookup requests
`/releases/tags/<validated tag>`. Default lookup follows `/releases?per_page=100`
and its validated same-repository `next` links, bounded to 100 pages; an
incomplete over-limit listing fails rather than selecting from a partial list.
Ignore support/component tags, drafts and prereleases. Select the distribution
with greatest valid `published_at` UTC timestamp (GitHub `YYYY-MM-DDTHH:MM:SSZ`);
ties use lexical tag order. Require boolean `draft`/`prerelease`, valid
`tag_name`, timestamp and an `assets` list on a selected release. GitHub may
add other response fields; these are not manifest-schema fields.
Exact lookup must return the requested published distribution identity.

Find exactly one release asset named `manifest.json`; its
`browser_download_url` must name that selected repository/tag/name over HTTPS.
Read its bytes with bounds and require `manifest.release` to equal the selected
tag. Pin this lookup result for the entire transaction. API/manifest/helper
downloads use fixed implementation caps/timeouts; payload downloads also stop
on declared-length violations. Save the exact selected manifest bytes/hash,
not a reserialized replacement. No GitHub credential is needed for public use.

## Owned filesystem

Logical layout under the user-owned root:

```text
bin/                  stable entry links through current
components/           immutable verified component versions
installations/        complete assembled versions, with interpreter identity
current -> installations/<complete-version>
```

Each assembled version contains real compiler executables under `bin/`,
`sdk/amiga/`, the compatible vamos environment under `sdk/vamos/`, the downloaded
VSIX, notices and install metadata. The driver locates `sdk/amiga` from its
canonical executable's installation root. Do not resolve the driver into an
isolated component directory that has no adjacent SDK.

The concrete native-driver descriptor, fixed SDK/venv filenames and process
interface are defined in [03-03](03-03-native-driver.md#concrete-installed-layout-contract).
Both the local core staging command and installer write that same descriptor
after tool assembly. It is not a final transaction receipt: the installer
retains the selected manifest bytes/hash, distribution/component identities
and interpreter identity, and writes its completed-installation state only
after all smoke tests pass. The driver may run against private assembled
staging before activation. No descriptor or pre-existing directory alone
makes an incomplete candidate reusable or eligible as `current`.

The assembled SDK is prepared privately from verified support inputs. Do not
modify a shared component or an active installation while adding NDK output.
Relative internal links may reuse immutable content only when relocation and
driver discovery remain valid. Never hard-link a file that preparation mutates.

“Root-owned” in AR-20 means owned by the installation root's user, not UID 0.
Refuse unrelated nonempty destinations and foreign-owned installer state.
Operate on the canonical selected root; reject traversal, special files and
unexpected symlink substitutions in installer-managed paths. A user's chosen
root may contain spaces; command guidance must quote it safely.

## Transaction and interruption

1. Validate invocation/platform/prerequisites and root ownership.
2. Hold a nonblocking process lock for the root, including reuse checks and
   activation. OS-held locking must release after process death; do not infer
   a live lock merely from a leftover filename.
3. Fetch the selected metadata and assets into private incomplete state.
   Bound network reads, size and elapsed time; stop at declared byte length
   violations and report which component failed.
4. Verify all archives; reuse only verified complete components.
5. Build a fresh assembled version, prepare NDK, install matching local wheels,
   and run required smoke tests through its own compiler/driver.
6. Mark the assembled version complete only after success. Flush complete state
   before replacing `current` with one same-filesystem atomic rename.
7. Print installed identity, quoted PATH guidance and the downloaded VSIX
   installation command. Release the lock and remove only this run's scratch.

At every pre-activation failure the old `current` remains unchanged. An
interruption at activation leaves either the old or new complete target;
partial directories never become eligible targets. A retry starts from verified
complete components and ignores incomplete state. Do not erase an existing
working version to repair an update.

Same-release reruns revalidate the selected complete installation and are
idempotent. A changed Python ABI produces a fresh matching installation;
never mutate the active venv. Preserve previous versions and user settings.
No automatic pruning or rollback-management command is introduced.

## Extraction and process controls

Validate archive members before writing any of them. Reject absolute paths,
parent traversal, duplicate/conflicting destinations, symlink-parent escapes,
outside hard/symbolic links, devices/FIFOs and unsafe modes. Permit only
directories, ordinary files and required internal relative links; extract into
fresh staging without following pre-existing links. Clear setuid/setgid bits.
Apply equivalent path controls to the verified NDK's helper extraction.

Bound entry count and expanded byte totals using the payload inventory from
the packager; do not allocate from an unchecked archive length. Download limits,
timeouts and extraction bounds are tested implementation constants derived from
the proved payload sizes, not performance promises about build time.

Install vamos with no package-index access and binary-only local wheels,
including all runtime dependencies. Invoke subprocesses by arguments and check
exit status. NDK helpers receive only validated staging paths.

## Success and failure output

Required smoke tests run a small host program and both Amiga-profile programs
through vamos. They must use the candidate's tools, not a developer PATH.
FS-UAE is checked when requested by the user through the driver; its absence
does not invalidate a compiler/host/vamos installation.

Failures return nonzero with the failed step and an actionable remedy. Missing
Ubuntu packages print the validated `apt` command; the installer never invokes
sudo or changes shell startup files. Do not report “installed” before activation.
An unsafe manifest/archive fails without writing outside staging.

Emit the two user-action commands on separate lines beginning `export PATH=`
and `code --install-extension ` respectively. Use POSIX shell quoting for the
absolute stable `bin` path and retained VSIX path. The PATH command preserves
the caller's existing PATH. Printing these commands does not execute them.
No other exact success-message wording is required.
Positive command-usability proof runs through the shipped bootstrap on the
real supported cohorts under the actual-context contract in the testing
strategy; local fault fixtures cannot stand in for a successful installation.

Specification cases: ST-11–ST-25. Review includes the before/during/after
activation failure sequence and running-driver component consistency.
