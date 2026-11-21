# Editor integration and release delivery

> **Parent**: [Index](00-index.md)
> **Authority**: AR-13/17/20/21/22/23/24/25/47/48

## VS Code actions

Extend the existing package with three commands:

| Command | Driver action |
|---|---|
| `clorraine.build` — Clorraine: Build | `build --profile <profile> <entry> <selected-dependencies...>` |
| `clorraine.run` — Clorraine: Run | `run --profile <profile> <entry> <selected-dependencies...>` |
| `clorraine.runInFsUae` — Clorraine: Run in FS-UAE | `run --profile <Amiga-profile> --emulator <entry> <selected-dependencies...>` |

Use `clorraine.profile` for both the LSP and actions. Add user/machine-level
`clorraine.driver.path`, `clorraine.sdk.path`, `clorraine.fsUae.path` and
`clorraine.kickstart.path`. Retain `clorraine.lsp.path`; executable path
overrides must not silently come from workspace configuration.
Pass nonempty SDK/emulator/ROM settings through their corresponding CLI flags
only when applicable to the action.

Check workspace trust before executable discovery that could use workspace
files, saving or task launch. Require an active file-backed `.clo` document;
save dirty content and stop on cancellation/failure. Use its containing
workspace folder as CWD, otherwise its parent directory. The single entry file
can import other source files; no new project-file format is introduced.
An FS-UAE action with host/freestanding selected reports a profile error.

Use `vscode.ProcessExecution` with an argument array and visible task terminal,
not shell command strings. Titles and failure messages identify the operation,
profile and remedy. Driver exit status is visible in task output; FS-UAE success
is never described as the guest program's status.

Per AR-48, reject the literal `${` introducer in the final executable, every
argument and CWD before task submission. VS Code interprets task variables even
in ProcessExecution, so shell quoting is not an escape. Report actionable
rename/path guidance; preserve other punctuation. No launcher wrapper is added.

## Saved-source selection

The CLI keeps its explicit-file contract. Add one read-only LSP request,
`clorraine/sourceFiles`, with `{entryUri, rootUri}` file-URI parameters and
response `{files: string[]}` containing canonical absolute paths, entry first,
then remaining paths in stable lexical order with no duplicates. Advertise
version 1 through `capabilities.experimental.clorraineSourceFilesVersion`.
An unavailable/incompatible server produces an actionable error and no task;
do not silently build only the entry.

The extension owns trust, active-file saving and root selection. Supply the
active file's containing root explicitly, or its parent for a standalone file;
do not rely on the LSP session's initial first workspace folder. After saving,
request selection and check that none of the selected open documents is dirty
before launch. If a selected dependency is dirty, stop and ask the user to save
it, then retry; never silently compile its old disk contents or save unrelated
documents. Active-document changes during selection also stop the launch.

CA-401's approved request-local guard observes editor `.clo` change/save events
under the containing canonical root, from before selection until submission.
If a relevant event occurs, stop with retry guidance even if the document is
clean when selection returns. Remove subscriptions in `finally` on every
success/error/disposal path. A conservative root-wide retry is permitted;
this is not an atomic snapshot or a promise against external/later writes.

Rust owns dependency discovery. Reuse the existing parser and module-reference
closure in `compiler/src/lsp/workspace.rs`, preserving live diagnostics'
overlay behavior separately. Refresh a bounded index from saved disk contents
under the requested canonical root for this request. Include transitive
imports, qualified module references and all files in selected modules; exclude
unrelated programs. Embedded standard-library imports remain compiler-owned.
Classify qualified references against the indexed module inventory, retaining
the compiler's binding/shadowing distinction from value field access; the
single-file reference collector alone does not know all unopened user modules.
Per AR-47, lexical locals shadow module paths. For nonlocal declarations/imports,
prefer the smaller field-based closure only when the complete member chain is
established from parsed declaration/type/import/merged-module metadata. Apply
that policy to longer genuine chains as well as existing short field controls.
Otherwise retain longest inventory-module matching. At most one request-local
Rust helper plus existing metadata plumbing is approved. No second parser,
general type checker or compiler-rule change; stop if broader machinery is needed.
Reject invalid/non-file URIs, entries or selected paths outside the root,
unreadable files, escaping symlinks and incomplete indexing/closure. Reuse
`MAX_INDEX_FILES = 2000` and `MAX_ANALYSIS_FILES = 256` as file-count limits.
Enforce the index budget while traversing, before retaining an over-limit source
path, rather than collecting the whole tree before truncating it. Stream
directory entries and retain the existing generated-directory/symlink exclusions.
A complete index or selection exactly at its limit succeeds; discovering the
2,001st indexable source or 257th selected file fails explicitly without a partial list.
Process every selected file before declaring the closure complete, even when
its count already equals 256. These limits bound file counts; the existing
protocol-message byte limit is a separate transport control.
Missing explicitly imported user modules fail with
an actionable selection diagnostic; unresolved expressions and syntax/semantic
diagnostics remain compiler-owned.

Keep selection logic in a focused Rust LSP module and reuse its parser helpers,
not a second TypeScript parser. Existing LSP diagnostic/navigation contracts and
the CLI multi-file oracle remain intact. Test this through real LSP requests and
driver execution, including changed imports after save and second-root files.

## Tool discovery

Preserve explicit LSP setting and environment overrides. Extend existing
discovery with the default user installation at `~/.clorraine/bin`, so desktop
VS Code works without terminal PATH. Retain existing development `target/debug`
and `target/release` lookup in trusted workspaces and PATH fallback.

The driver uses the explicit machine setting when present; otherwise find the
installed driver, trusted development build or PATH executable. An explicit
invalid setting produces a configuration error rather than silently selecting
another executable. Custom roots use machine path settings.

Remote-WSL resolves all tool paths inside the Linux extension host. Document
installing the VSIX in that remote host. Automatic Windows FS-UAE launch is
outside the approved action contract; document transfer/use of the generated
Amiga executable in the user's Windows emulator separately.

Keep action construction and discovery in focused modules under
`tools/vscode/src/`; reuse the current LSP lifecycle. Register/dispose commands
with the extension context. Do not add an editor-specific compiler or runner.

## Two bounded workflows

Use `.github/workflows/support-release.yml` and
`.github/workflows/distribution-release.yml`. Both are manual dispatch only.
Each calls the same repository commands proved locally; neither duplicates the
build/preparation algorithm in YAML.

| Workflow | Validated inputs | Work and outputs |
|---|---|---|
| Support release | Exact source commit and new support identity | Locked support build, source/licence audit, Ubuntu/ABI tests, immutable support assets and metadata in a draft |
| Distribution release | Exact compiler commit, distribution tag, exact support identity; previous payload identities for reuse | Build/reuse core and VSIX, prepare compiler/editor component draft plus bootstrap/helper/manifest distribution draft, package checks on both Ubuntu baselines |

Pin checkout/actions and build environments. Validate dispatch values before
using them in paths/commands; pass values as arguments/environment, not
interpolated shell source. Default permissions are read-only; only the upload
job has `contents: write`. Disable persisted checkout credentials and scope
release credentials to upload. Do not upload secret-bearing logs or media.

Set finite job timeouts and concurrency per release identity. A failed rerun
must not overwrite an existing asset with different bytes. If an identical
asset is already present, verify its bytes before reuse. Caches accelerate
builds; they never replace input/provenance validation.

Build core executables on the Ubuntu 22.04-compatible baseline and test the
same binaries on both approved distributions. Include all three executables,
the current first-party licence/exception/notice/trademark files and relevant
third-party notices. VSIX contents are audited by the existing packaging checks.
`v1.2.0-rN` can reuse unchanged compiler binaries; compiler `--version` remains
the core version while install metadata records the distribution revision.

## Payload identities and local producers

Use separate immutable release namespaces: `support-<identity>` for support,
`components-<identity>` for compiler/editor payloads, and `v1.2.0` /
`v1.2.0-rN` for installable distributions. Component identity is a nonempty
lowercase ASCII alphanumeric/hyphen token beginning with an alphanumeric;
validate it as data, never a path or shell fragment. Core archives and matching
VSIX assets, plus their required source/notices, live in component releases.
Every manifest-referenced payload must be independently publishable before the
distribution. The distribution holds the shell entry, hash-pinned helper and
manifest. Support/component releases are never marked latest and are excluded
from both default and exact installer selection.

The existing distribution workflow manages both its component and distribution
drafts; no third workflow is introduced. It can reuse exact earlier core and
VSIX asset identities independently. A changed VSIX/helper or support set never
requires rebuilding unchanged core bytes. New asset bytes require a new
identity; retries verify identical existing bytes without replacement.

Prove local producers before automation:

1. In Phase 2, `tools/distribution/package_core.py` packages all three current
   compiler binaries and notices and stages an installed-layout candidate with
   already proved support/NDK/venv inputs. This precedes native packaged-driver
   observation. It creates no public tags and does not bump version records.
2. In Phase 3, `tools/distribution/package_release.py` assembles the real helper,
   shell and manifest using that core and the existing VSIX packaging command.
   Current development versions are valid local proof inputs; they must agree
   in metadata. Run the packaged bootstrap before the installer phase gate.
3. Phase 5 reuses those producers, updates final version records, adds immutable
   identity/reuse and promotion checks, and automates draft uploads.

### Local core producer command contract

The Phase 2 producer has two explicit maintainer commands:

```text
python3 tools/distribution/package_core.py archive --binaries DIR --output PATH
python3 tools/distribution/package_core.py stage --core PATH --lock PATH --sdk-archive PATH --sdk-audit PATH --preparation-archive PATH --preparation-audit PATH --wheels-archive PATH --wheels-audit PATH --ndk PATH --python PATH --output DIR
```

Unknown arguments/missing values are usage failures (exit 2). Invalid content,
missing prerequisites, failed subprocesses or an existing output are named
failures (exit 1). Success exits zero and prints the completed output path.
These commands never build a compiler/dependency, fetch inputs, publish, alter
versions, change `current`, or require a VSIX. The eventual installer/VSIX
assembler remains Phase 3. Run as an ordinary user.

`archive` reads the three ordinary executable files `clc`, `clorraine`,
`clorraine-lsp` directly under `--binaries`. Run each with `--version` using a
finite timeout, and require its own tool name and the same current three-part
version as `compiler/Cargo.toml`. Do not substitute executables from PATH.
The files must be Linux x86_64 ELF executables. This local command does not
establish Ubuntu compatibility merely from a file format; the same actual
archive is subsequently exercised in the prescribed clean environments.
Copy the supplied executable bytes into this command's existing private work
directory, and use those same private snapshots for inventory, ELF/version
validation and archiving. Do not query mutable original paths and later
reopen them for archiving: replacements can make a successful version query
describe different bytes. Preserve caller inputs, exact packaged bytes,
deterministic output and no-clobber publication; no compiler build is added.
The thin `clorraine-lsp` binary entry recognises exactly the single argument
`--version` and prints `clorraine-lsp <current core version>\n` with exit 0,
without a protocol handshake. Every other argument vector retains unchanged
stdio server startup. This bounded entry change is independently process-tested;
the existing LSP library, initialize response and protocol oracle are unchanged.
Read the five current notice files from the checkout owning the producer:
`LICENSE`, `LICENSE-EXCEPTION.md`, `NOTICE`, `TRADEMARKS.md`,
`THIRD_PARTY_NOTICES.md`.

Output is an uncompressed PAX tar with exactly these eight file members:
`bin/clc`, `bin/clorraine`, `bin/clorraine-lsp` and the five root notice names,
plus `core.json`. There is no wrapper directory. `core.json` has exactly
`schema: 1`, `core_version` and `files`. The latter maps each of the eight
paths to exactly `bytes` (positive integer), `sha256` (lowercase hash) and
`executable` (true only for the binaries). Reject duplicate JSON keys and
unknown fields. Do not include generated C, build caches, media or checkout
paths. File modes are 0755/0644; uid/gid/mtime are zero and user/group names
empty, as in support packaging. Identical input bytes produce identical
archive bytes. Publish complete output without overwriting an existing path;
on failure remove only this command's private work.

`stage` consumes that actual core archive and the three existing audited
support archives (SDK, preparation and one ABI's wheel set). Each audit uses
the existing support schema and matches the exact supplied lock and complete
archive member bytes/types/links. Validate before extraction; reuse existing
safe-member/inventory/audit contracts, rather than trusting an archive suffix
or extracting before validation. The supplied NDK must match the lock's exact
archive bytes/hash. The supplied `--python` is an executable path naming the
system CPython 3.10 or 3.12 used for this candidate; its ABI must match the wheel
manifest. No Python or C compiler is provisioned by staging.

Build privately in a fresh output sibling, without following pre-existing
managed aliases. Extract the core into the candidate root. Extract the SDK
into `sdk/amiga`; extract preparation/wheels into this invocation's private
scratch only. Invoke the authenticated preparation script/helpers on the SDK.
Create the matching private venv at `sdk/vamos`, and install the verified local
wheels using the already proved isolated/offline/hash-required/binary-only
process in [03-01](03-01-support-builds.md#vamos), including pip and dependency
checks. Copy the exact wheel manifest to `sdk/vamos/wheels.json`.
Write the six-field `installation.json` defined in
[03-03](03-03-native-driver.md#concrete-installed-layout-contract), with version
from the verified core, support/NDK/inventory hashes from the lock/preparation
and ABI from the matched wheel/Python cohort. Do not copy a caller's handwritten
installed layout or use a checkout import in the staged driver.

Verify all staged bytes against their inputs/receipts, and publish the whole
candidate directory without clobber only after successful assembly, preparation
and offline venv checks. A candidate is a **local staged proof input**, not a
completed installer transaction or accepted release. Task 2.2.8 must still run
the actual packaged driver/host/vamos/native-emulator acceptance before the
phase gate. Leave the original inputs and any pre-existing output untouched;
clean this invocation's scratch and failed private candidate.

The specification suite may use small valid core/support fixtures and logging
process helpers for local failure controls. Such a fixture cannot replace
actual current binaries/archives, isolated wheel/NDK preparation, relocation
or real emulator observations. The accepted Phase 1 support archives and their
audits are reused without changing their bytes or relabelling receipts.

Local and authenticated-draft proofs use exact candidate bytes with the same
validation/activation path as public installs. A maintainer-only transport
adapter may supply those bytes by component identity; it cannot rewrite the
manifest, bypass hashes or add repository imports. It is not a public installer
option. Final public URLs are emitted into the candidate manifest before proof.

### Local bootstrap producer command contract

The Phase 3 maintainer command is:

```text
python3 tools/distribution/package_release.py assemble --release TAG --components ID --support ID --core PATH --vsix PATH --support-inputs DIR --output DIR
```

All arguments are required. TAG is a distribution tag and both IDs use the
component identity grammar. Unknown options/missing values exit 2. Invalid
content, missing inputs or existing output of any type exit 1 with the failed
input/step named. Success exits 0 and prints the completed output directory.
Reject terminal control characters in arguments and sanitize failure output.
The command never builds binaries, invokes npm, fetches, installs, changes
versions, activates, creates tags or publishes. Run the existing
`make vscode-package` separately to obtain the current audited VSIX input.
Current 1.1.2 core/extension versions are valid local proof inputs even when
the distribution tag is `v1.2.0`.

`--support-inputs` is an ordinary directory with these nine ordinary files:

| Filename | Contents |
|---|---|
| `support-lock.json` | Exact accepted support lock |
| `amiga.tar`, `amiga-audit.json` | Audited SDK without NDK-derived headers |
| `preparation.tar`, `preparation-audit.json` | Audited preparation/helpers/notices/source |
| `cp310.tar`, `cp310-audit.json` | Audited cp310 wheels/notices/source |
| `cp312.tar`, `cp312-audit.json` | Audited cp312 wheels/notices/source |

These are the lock and four archive/audit pairs. Extra files in the caller's
directory are not packaged or read as inputs.
Do not require relabelling or changing existing Phase 1 archive/audit bytes.
The upstream NDK is not copied into this directory/output: obtain its exact
URL/length/hash from the lock. The core input is the Phase 2 archive; the VSIX
is the existing package command's output. Validate their actual contents and
the cross-component relationships defined in the manifest contract.
Inventory/query/output must describe the same private input snapshots, not
mutable originals reopened at different stages.

Publish a fresh output directory containing exactly `install.sh`,
`install.pyz`, `manifest.json`. The first two are 0755, the manifest 0644.
The `.pyz` is deterministic ZIP (sorted paths, fixed 1980 timestamp, fixed
safe file modes, no absolute/source-checkout paths). It contains `__main__.py`,
empty package initializers for `tools`, `tools/setup`, `tools/distribution`,
the three planned native installer modules and the shared manifest module.
Include needed runtime data explicitly; no import requires `package_core`,
the Docker SDK installer, source recipes, the checkout or third-party Python
packages. `__main__` calls `tools.setup.install_native.main()` and propagates
its integer status. `main(argv=None)` accepts the public shell arguments;
it is not a separate richer public CLI.

The shell template is the planned `tools/setup/install.sh`; substitute only
the validated literal helper URL/hash/length slots. No arbitrary template
evaluation. The producer must reject absent required helper modules/runtime
data before publishing any final output. Output metadata uses sorted compact
UTF-8 JSON plus newline. Identical input/source bytes and identities yield
identical three output files. Build in owned private sibling staging and
publish without clobber only after all validation; leave original inputs and
pre-existing output untouched. On failure remove only this run's scratch.

Construct manifest identities deterministically:
`<components ID>-core`, `<components ID>-vsix`,
`<support ID>-lock`, `<support ID>-amiga`, `<support ID>-preparation`,
`<support ID>-cp310`, `<support ID>-cp312`, the four respective
`<support ID>-<archive name>-audit` identities, and `<support ID>-ndk`.
Archive names in audit identities are `amiga`, `preparation`, `cp310`, `cp312`.
Measure hashes, lengths and archive expansion records from the supplied bytes.
Use the final URL namespaces/filenames in the installation contract.
The manifest references twelve inputs; helper integrity remains pinned in
the shell, not recursively self-referenced in that manifest.
Later workflow/source/promotion eligibility checks remain Phase 5 work.

### Miniature manifest and input fixture

This recipe defines independently constructible metadata, not a real support
or install acceptance result. Use `H`/`J` and the complete miniature support
lock/audit from `03-01`, and the existing core/wheel schemas. Construct actual
small tar/ZIP bytes with matching records; do not substitute arbitrary hash
strings for the bytes being checked.

1. Let `L` be the miniature lock, with cp310/cp312 and nix13/nix20 as specified
   there. `K=J(L)`. `I=H(J(L["preparation"]["inventory"]))`. Its NDK input
   is the exact miniature `sources[preparation.archive]` archive.
2. Construct four small audited support payloads following the existing
   file/origin/notice/source rules. Preparation metadata/helpers use the
   existing preparation contract. Each ABI payload has a complete matching
   five-package wheel manifest; its tiny wheel ZIP records/content are
   independently authored fixtures, not evidence that vamos works.
   Audit `lock_sha256` is `H(K)` throughout.
3. The core is the established nine-member fixture tar: three distinct
   Linux x86_64 executable observation helpers, five notice files and
   `core.json` with exact lengths/hashes and version `1.1.2`. Test-time
   compilation of observation helpers is allowed; installers never compile
   installation dependencies.
4. A minimal VSIX ZIP has `[Content_Types].xml`, `extension.vsixmanifest`,
   `extension/package.json` with `name:"clorraine"`, `publisher:"openlorraine"`,
   `version:"1.1.2"`, and `extension/out/extension.js`. For real producer proof
   use the actual current `make vscode-package` output, never this substitute.
5. Choose `--components fixture-core --support fixture-support
   --release v1.2.0-r1`. Each asset's identity follows the formula above,
   URL follows the final namespace/filename contract, length/hash come from
   its exact bytes, and tar/ZIP counts/totals are independently measured.
   Lock/audit/NDK expansion records are zero/null as specified.

The asset rows are exactly the following tuples. The first field is the
asset identity, followed by kind, filename, root, audit identity and release
namespace. Here `c="fixture-core"`, `s="fixture-support"`. `raw[filename]`
contains the exact bytes from steps 1–4; `measure(raw[filename], kind)` means
the independently counted archive members and expanded ordinary byte total,
or `(0,0)` for raw inputs. These definitions leave no field or reference for
the test author to guess.

```python
rows = [
    (c+"-core", "core", "core.tar", ".", None, "components-"+c),
    (c+"-vsix", "vsix", "clorraine.vsix", "extension", None, "components-"+c),
    (s+"-lock", "support-lock", "support-lock.json", None, None, "support-"+s),
    (s+"-amiga", "amiga", "amiga.tar", ".", s+"-amiga-audit", "support-"+s),
    (s+"-preparation", "preparation", "preparation.tar", ".",
     s+"-preparation-audit", "support-"+s),
    (s+"-cp310", "wheels-cp310", "cp310.tar", ".", s+"-cp310-audit", "support-"+s),
    (s+"-cp312", "wheels-cp312", "cp312.tar", ".", s+"-cp312-audit", "support-"+s),
    (s+"-amiga-audit", "support-audit", "amiga-audit.json", None, None, "support-"+s),
    (s+"-preparation-audit", "support-audit", "preparation-audit.json",
     None, None, "support-"+s),
    (s+"-cp310-audit", "support-audit", "cp310-audit.json", None, None, "support-"+s),
    (s+"-cp312-audit", "support-audit", "cp312-audit.json", None, None, "support-"+s),
    (s+"-ndk", "ndk", "ndk", None, None, None)]
assets = []
for identity, kind, filename, root, audit, namespace in rows:
    count, expanded = measure(raw[filename], kind)
    url = (L["sources"][L["preparation"]["archive"]]["url"] if kind == "ndk"
           else "https://github.com/OpenLorraine/clorraine/releases/download/"
                + namespace + "/" + filename)
    assets.append({"identity": identity, "kind": kind, "url": url,
                   "bytes": len(raw[filename]), "sha256": H(raw[filename]),
                   "root": root, "entries": count, "expanded_bytes": expanded,
                   "audit": audit})
```

The complete manifest object is this expression, using that `assets` array:

```python
{
    "schema": 1,
    "release": "v1.2.0-r1",
    "core_version": "1.1.2",
    "extension_version": "1.1.2",
    "platform": {"os": "linux", "architecture": "x86_64",
                 "cohorts": {"22.04": "cp310", "24.04": "cp312"}},
    "assets": assets,
    "support": {
        "identity": "fixture-support",
        "lock": "fixture-support-lock",
        "amiga": "fixture-support-amiga",
        "preparation": "fixture-support-preparation",
        "wheels": {"cp310": "fixture-support-cp310",
                   "cp312": "fixture-support-cp312"},
        "ndk": "fixture-support-ndk",
        "profiles": ["hosted-amiga", "hosted-amiga-13"],
        "prepared_inventory_sha256": I},
    "prerequisites": {
        cohort: sorted(set(L["output"]["runtime_packages"][cohort]) |
                       {"gcc", "libc6-dev", "python3", "python3-venv",
                        "curl", "ca-certificates"})
        for cohort in ("22.04", "24.04")}
}
```

For a structural-only manifest test, payload byte strings may be simple
deterministic test data and measurements rather than functional tools:
structural validation does not authenticate or execute them. Producer and
extraction tests instead supply the matching actual tar/ZIP/JSON fixtures
from steps 1–4. Neither substitutes for required actual archive/bootstrap
installation, Ubuntu/ABI, smoke, update or relocation acceptance.

### Exact-byte maintainer transport fixture

Keep this fixture under the existing test directories, never in shipped
installer modules. It is ordinary unittest/process-fixture code, not a
transport service, plugin loader or public installer test mode.

Copy the exact producer outputs and a standard-library-only test runner into
private test storage. The test invokes the unchanged shipped shell with its
ordinary public arguments. A controlled test PATH has two external-boundary
shims:

- `curl` records the literal download arguments and provides the exact pinned
  helper bytes, or a selected failure/corruption. It must not skip the shell's
  normal size/hash checks.
- `python3` records and requires the exact `-I <downloaded pyz> <user args>`
  handoff, then invokes the real absolute system Python with `-I` and the
  copied test runner. For ordinary transport tests it changes only the external
  HTTP-open boundary (`urllib.request.OpenerDirector.open`) and calls
  `runpy.run_path(pyz, run_name="__main__")` with the original installer argv.
  All installed modules still load from that exact pyz; no repository path
  or helper-source directory is added to `sys.path`.

The installer uses an opener with automatic redirects disabled; its normal
download loop validates status, each HTTPS Location, final identity, size,
hash and limits. A fixture response supplies status/headers/body stream with
`getcode()`, `geturl()`, `info()`, `read(n)`, context-manager and close behavior.
It is not a parsed manifest or a prevalidated archive. Route tables map exact
requested URLs to response sequences; unexpected requests fail. Release API
responses use `tag_name`, `draft`, `prerelease`, `published_at`, `assets` with
`name`/`browser_download_url`, and normal `Link` pagination headers.
Redirect responses use a `Location` header. No test route can itself tell the
installer to accept a hash, ignore a schema field or activate a candidate.

Fixture route keys/response sequences are test-owned data in private storage,
read only by the copied runner; they are not accepted installer arguments or
environment variables. It may log requests and block/release a response with
ordinary synchronization files for truncation, timeout and death/lock tests.
Use finite parent deadlines. Do not replace archive/state/validation/smoke
logic in the shipped modules. Process helpers may model failing prerequisites,
preparation and wheel subprocesses in failure-injection scenarios only;
successful supported Ubuntu acceptance must run the real tools and full smoke path.

Run the shell proof with checkout inaccessible and `PYTHONPATH` absent.
Record original and delivered shell/helper/manifest/payload hashes. For a
direct isolated missing-module control, remove a required member from a private
copy of the pyz and require that copy's execution to fail; this does not grant
permission to re-pin the shipped shell to unauthenticated helper bytes.
Separately corrupt bytes delivered to the unchanged shell and prove no helper
execution. Real-network HTTPS certificate handling is not established by the
simulated HTTP-open boundary; test configured verification/redirect rejection
and retain actual HTTPS/download acceptance as a separate requirement.
No mock can count as successful clean-machine installed-runtime evidence.

For local platform/prerequisite failure tests only, that copied runner may
also supply external probe results through `platform.freedesktop_os_release`,
`platform.machine` and `sys.version_info` before entering the pyz. Record every
override. This models OS/architecture/interpreter identity, not a bypass of the
installer's own compatibility comparisons. It cannot supply actual venv/wheel,
smoke, relocation or supported-cohort evidence; those run on the real matching
Python and Ubuntu without probe overrides.

### Negative process and activation bindings

The planned process-death/failure cases need additional **external-boundary**
observations, not installer callbacks. A separate copied state-test runner may
wrap `subprocess.Popen`, `os.rename` and `os.replace` before entering the exact
pyz through the same `runpy` handoff. It may compose the copied transport
runner without editing its shipped inputs or the earlier frozen oracle files.
Test controls live only in the private fixture's data; no production argument,
environment variable, import or test-mode branch reads them.

For subprocess faults, inspect and record the literal attempted argv and CWD.
Reject `shell=True` in this observer. Match a **semantic external operation**,
not a private installer function, temporary-directory name or guessed order:

| Operation | Identifying external boundary |
|---|---|
| Package availability | `dpkg-query` for the validated manifest's prerequisite package set |
| Host compiler/header probe | System `cc`/`gcc` command before candidate smoke |
| Venv capability/creation | The system interpreter's `-m venv` invocation or an explicit interpreter probe importing `venv` |
| Preparation | Verified `prepare_ndk.py` with its documented `--archive`/`--destination` arguments |
| Wheel installation | The candidate venv interpreter's isolated `-m pip ... install` invocation |
| Required smoke | Candidate `clorraine run` command, distinguishing host/hosted-amiga/hosted-amiga-13 by its public `--profile`/`-p` value or documented default |

At one selected occurrence, either raise the corresponding external launch
error (`FileNotFoundError` for an absent executable) or run a fixture-owned
real failure child that prints a unique diagnostic and exits nonzero. Every
other subprocess runs unchanged. The failure child never pretends to compile,
install wheels, prepare an SDK or pass a smoke test. Require a recorded match
and observed failed child before crediting the fault scenario; an earlier
unrelated error cannot pass it.

Tool-lookup negatives may wrap `shutil.which` for named prerequisite tools
only. Runtime-library negatives may wrap `ctypes.CDLL` for a declared system
SONAME only, raising `OSError`. They may not fake a successful lookup/load or
replace installer comparisons. If a miniature fixture cannot reach a chosen
failure stage legitimately, run that negative in the actual-context group,
starting from a real successful installation and using actual candidate bytes.
Faults affect only the attempted update; old-version usability checks run
with all fault observers disabled.

The activation observer wraps **both** standard-library rename functions and
otherwise forwards their original arguments, directory fds, results and
exceptions. Its only barrier target is the `current` destination directly
under the test's canonical selected root. Resolve relative names against the
supplied directory fd (on Linux, `/proc/self/fd/<fd>`), or CWD when absent;
do not follow the final `current` link when identifying that directory entry.
Observe the real atomic replacement, not a guessed temporary symlink name.
Immediately before calling the real rename, append a `before-current` event
and signal an invocation-specific ready file. Immediately after it returns,
append `after-current` and signal a separate ready file. A selected barrier
waits for its own release file with a finite deadline. All other renames run
unchanged; no wrapper writes, repairs or chooses an installation target.

The parent waits for that explicit syscall-boundary signal, then terminates
only the owned installer process/session, inspects old-or-new runnable complete
selection, and retries with barriers disabled. Seeing a new directory or
polling the pointer alone is not evidence of reaching the activation boundary.
No observed event means the scenario has not reached its target and must fail.
Use real OS-held locks; do not replace `flock` or fabricate lock ownership.

Fault/barrier runs are **negative acceptance**, never successful runtime proof.
Successful initial/update/relocation runs on both cohorts use the actual tools
without these overrides. Preserve all helper/manifest/payload bytes and hashes.
The earlier statement limiting ordinary transport instrumentation does not
exclude these explicitly planned, narrowly scoped negative process/rename cases.

## Draft acceptance and public promotion

Draft assets are private candidate artifacts. Maintainer tests may authenticate
their download or use exact downloaded candidate files; credentials stay outside
installed metadata and public logs. Verify candidate hashes and the production
manifest/installation path, rather than replacing metadata with an untested
handwritten package layout.

The workflows can prepare drafts after implementation is authorised. They do
not automatically publish. Promotion requires the later explicit user go
specified by AR-22:

1. Finish all candidate evidence, including real desktop VS Code, WSL2 and
   licensed local emulator gates. Candidate-green means ST-53–ST-55 and all
   applicable earlier cases; ST-56 is post-publication evidence.
2. Complete evidence/hash-mismatch controls, full/editor/site gates and
   independent acceptance review. Resolve all required findings. Verify exact
   asset identities and required corresponding-source/notices.
3. Request explicit approval bound to the reviewed candidate hashes and
   evidence. Any affected byte/evidence change invalidates that approval and
   requires the applicable rerun, review and renewed approval.
4. Publish referenced support and compiler/editor component releases first,
   without marking them latest. Verify all referenced public URLs and hashes.
   A missing/private/mismatched payload blocks distribution promotion.
5. Promote the complete distribution containing the tested bootstrap/helper
   and manifest, then perform ST-56 using fresh anonymous downloads. Compare
   public bytes to the approved hashes; missing ST-56 blocks final completion.

A missing referenced asset blocks distribution promotion. A partially uploaded
draft is not a valid install target. Existing tags/assets are immutable; fixes
require a new identity/revision. The current planning task does not dispatch
either workflow, create tags or publish assets.

## Version records and documentation

During implementation, update `compiler/Cargo.toml`, `Cargo.lock`,
`tools/vscode/package.json` and `package-lock.json` to 1.2.0. Keep unrelated npm
package versions independent. Update `CHANGELOG.md`, `RELEASE-NOTES.md`,
`README.md`, `RELEASE.md`, `THIRD_PARTY_NOTICES.md`, extension README and the
site quick-start as applicable.

The v1.2.0 runbook owns candidate evidence and publication authority. Preserve
the v1.1.2 record, published MIT terms and immutable asset identities as history;
new first-party artifacts use the current repository licence set.
Document system prerequisites, explicit updates, root/release selection,
PATH/VSIX commands, SDK discovery, both Run choices, native capture limitation,
media ownership, Python-ABI changes and WSL boundaries by referring to the
owning contracts, not claiming future acceptance already passed.

## Acceptance ownership

| Evidence | Executor/environment |
|---|---|
| Clean Ubuntu package and offline wheel tests | Repository commands in CI and clean local environments |
| Existing full compiler/Amiga gates | Local pinned SDK with licensed media |
| Native FS-UAE | Local Linux; private X display and local KS3.1 |
| Desktop VS Code | Packaged VSIX, installed candidate tools, no terminal PATH |
| Actual WSL2 and Remote-WSL | User's Windows test machine; record candidate identities and commands |

All evidence binds to candidate bytes/source revisions. A change affecting a
tested artifact invalidates the relevant evidence and requires rerun.
Specification cases: ST-40–ST-59. No external evidence is replaced by a green
host-only CI job.
