# Native SDK and FS-UAE driver

> **Parent**: [Index](00-index.md)
> **Authority**: AR-14/19/24/29

## Integration surface

Extend `compiler/src/clorraine/{cli,environment,build,run,toolchain}.rs` and
their orchestration in `mod.rs`. Put native emulator staging/process lifecycle
in a focused sibling module rather than enlarging `mod.rs`. Keep the compiler
std-only. Reuse existing diagnostics, argument-vector and artifact helpers.

Parse `--sdk-root PATH`, `--fs-uae PATH` and `--kickstart PATH` into explicit
options. CLI flags override their environment equivalents:
`CLORRAINE_SDK`, `CLORRAINE_FS_UAE`, `CLORRAINE_KICKSTART`.
Keep existing profiles, compilation modes, command forms and diagnostic formats.
Neither driver parser currently supports a `--` terminator; retain that
behavior. Use absolute or `./`-prefixed source paths where needed.

SDK options apply to non-host build/run. FS-UAE/ROM options apply to Amiga
`run --emulator`; inapplicable CLI options are usage errors, as with existing
`--emulator`/`--capture` validation. Environment settings for unused tools do
not make ordinary host/emit/help/version commands require those tools.
After SDK-mode resolution, explicit `--fs-uae` or `--kickstart` in legacy mode
is a usage error (exit 2) before building, with guidance to select a native SDK.
Unused emulator environment values do not change mode or the old handoff.

## Resolution

Resolve one invocation context before building:

| Selection | Resolution and execution mode |
|---|---|
| Explicit `--sdk-root` | Validate exactly that SDK; native mode |
| `CLORRAINE_SDK` | Validate exactly that SDK; native mode |
| SDK adjacent to canonical installed driver | Resolve installation root once, then `sdk/amiga`; native mode |
| Legacy `/opt/amiga` marker | Preserve existing SDK tool lookup and Docker handoff |

A selected invalid SDK fails without searching a lower-priority location.
Validate the selected root, provenance/installation metadata and tools required
by the requested command. Native compiler and runner paths are absolute paths
within the resolved installation; do not repeatedly traverse `current`.
An SDK override names the Amiga root; its packaged vamos is the companion
`sdk/vamos` from the same installation. Validate that pairing for vamos Run.
A build does not require vamos or FS-UAE.

Keep host `cc` lookup and legacy tool lookup as before. Thread resolved native
program paths through both the environment check and the actual subprocess
invocation: validating one binary then running a bare name is insufficient.
Retain existing target flag arrays, link libraries and mode selection.
Freestanding still cross-links then uses its existing host execution leg.

The adjacent lookup must work after root relocation and through stable
`~/.clorraine/bin` links. Resolve actual executable location rather than CWD or
the source workspace. Pin the companion runner and compiler for the entire
invocation so a concurrent installer activation cannot mix versions.

### Concrete installed-layout contract

The native descriptor is the ordinary UTF-8 JSON file `installation.json`
at the assembled installation root `I`. Its exact fields are:

| Field | Required value |
|---|---|
| `schema` | Integer `1`, not a boolean |
| `core_version` | Three decimal version components, such as `1.1.2`; records the staged core, not a requirement that an explicit SDK override share the invoking driver's version |
| `support_lock_sha256` | Lowercase 64-character SHA-256 |
| `ndk_sha256` | Lowercase 64-character SHA-256 |
| `prepared_inventory_sha256` | Lowercase 64-character SHA-256 |
| `python_abi` | Exactly `cp310` or `cp312` |

Reject duplicate/unknown/missing fields, wrong types, invalid UTF-8 and
metadata larger than 16 MiB. Read metadata as ordinary files, not devices,
FIFOs or symlinks. This descriptor records assembled tool readiness, not
publication authority, authenticity against a hostile local user, or installer
activation/completion. Installation separately retains the exact downloaded
distribution manifest and its final transaction receipt. Writing this descriptor
in private staging allows candidate-driver smoke tests before activation.
The installer must not treat this descriptor alone as completed installation.

SDK overrides name `I/sdk/amiga`, not `I` or an arbitrary compiler prefix.
Resolve a user-selected path or stable `current` alias once to `I`; then retain
paths beneath that canonical root for the invocation. The real `sdk` and
`sdk/amiga` directories may not redirect into another installation. Discover
adjacent `I` from the canonical executable's `I/bin/clorraine`, not from argv[0].
An `installation.json` or `sdk` entry beside the executable's parent `bin/`
signals a selected adjacent candidate, even if that entry is broken or invalid;
do not silently fall back from such a candidate. Without either entry, a
development binary can still select legacy mode.

Require `I/sdk/amiga/.ndk-prepared.json`, using the existing preparation
schema: exactly integer `schema: 1` and three lowercase hash strings
`lock_sha256`, `ndk_sha256`, `inventory_sha256`. They must equal the descriptor's
support, NDK and prepared-inventory fields. This is the preparation receipt,
not an instruction to rehash the entire SDK at every invocation; archive and
preparation integrity remain producer/installer responsibilities.
Require the selected command's cross compiler at the fixed path
`I/sdk/amiga/bin/m68k-amigaos-gcc`. Internal compiler aliases are permitted only
when their resolved target stays within this SDK and is executable.

For Amiga Run without `--emulator`, additionally require:

- A real companion directory `I/sdk/vamos`, not an alias to another installation.
- `I/sdk/vamos/wheels.json`, copied unchanged from the proved matching wheel
  payload. It uses the complete wheel-manifest schema in
  [03-01](03-01-support-builds.md#wheel-process-interface). Validate schema,
  field/types/identities, package records and the required runtime/bootstrap
  names `amitools`, `machine68k`, `greenlet`, `pysdl2`, `pip`. Its `lock_sha256`
  and `abi` must match the installation descriptor.
- The executable `I/sdk/vamos/bin/python` and ordinary `pyvenv.cfg` in that
  companion root. Venv directories and metadata stay inside this companion.
  The interpreter itself may be a symlink to the supported system Python,
  as ordinary Ubuntu venvs are. Preserve the **venv spelling** in the executed
  path; canonicalising its final interpreter symlink would lose venv selection.

A missing/non-executable compiler or companion interpreter, or a missing
companion/`pyvenv.cfg`, is a missing-tool failure (exit 1, named remedy).
Missing/malformed/mismatched descriptor, NDK receipt or required wheel manifest
is an invalid selected SDK (exit 2, selection named, no build/run/fallback).
Build and freestanding commands do not require companion files or validate
unused wheel metadata. Host/emit/help/version do not resolve this native SDK
from environment settings. Native tool roots containing spaces/punctuation
remain valid; displayed errors must not replay terminal control bytes.

### Companion Python argument vector

Invoke the selected venv Python directly as these five argument elements:

```text
[I/sdk/vamos/bin/python, "-I", "-c", ENTRY, ABSOLUTE_PROGRAM]
```

`ENTRY` is this fixed text, never metadata or a user-provided command:

```python
import importlib.metadata
import sys
entries = [entry for entry in importlib.metadata.distribution("amitools").entry_points
           if entry.group == "console_scripts" and entry.name == "vamos"]
if len(entries) != 1:
    raise SystemExit("clorraine: expected exactly one installed vamos entry point")
sys.argv = ["vamos", *sys.argv[1:]]
raise SystemExit(entries[0].load()())
```

No shell, pip-generated entry-script shebang, extra Python search path or bare
`vamos` executable is used. Both the compiler and this venv-spelled runner
path use the same already-canonical `I`, including after concurrent activation.
The legacy bare `vamos PROGRAM` path remains unchanged.

### Complete miniature native fixture

An independent process-selection fixture may use arbitrary fixed lowercase
hashes; it is not a real accepted support archive:

```text
I/
  installation.json
  bin/clorraine                       real copy of tested driver
  sdk/amiga/.ndk-prepared.json
  sdk/amiga/bin/m68k-amigaos-gcc        distinct executable logging helper
  sdk/vamos/pyvenv.cfg                 ordinary file, "version = 3.10.12\n"
  sdk/vamos/wheels.json
  sdk/vamos/bin/python                 distinct executable logging helper
```

The descriptor is `{"schema":1,"core_version":"1.1.2",
"support_lock_sha256":A,"ndk_sha256":B,"prepared_inventory_sha256":C,
"python_abi":"cp310"}`, where A/B/C are respectively 64 copies of `a`/`b`/`c`.
The NDK receipt is `{"schema":1,"lock_sha256":A,"ndk_sha256":B,
"inventory_sha256":C}`. A cp312 fixture uses its matching ABI/version.
The wheel manifest is `{"schema":1,"lock_sha256":A,"abi":"cp310","packages":P}`.
P contains the five required names. Each value has `version:"1.0"`,
`source` equal to that name, `purpose:"bootstrap"` for pip and `"runtime"`
otherwise, `filename:"wheels/<name>-1.0-py3-none-any.whl"`, `bytes:1`,
`sha256:A`, `tags:["py3-none-any"]`. These synthetic wheel files need not exist
inside a venv: the copied manifest is a source receipt, not a wheel directory.
Helpers log their actual executable path and literal argv; the compiler helper
may create a chosen output after `-o`, and the runner helper may print/return
chosen values. Such helpers test dispatch only, never real Python/Amiga proof.
Stable access uses `ROOT/current -> installations/a` and
`ROOT/bin/clorraine -> ../current/bin/clorraine`, with this fixture at
`ROOT/installations/a`.

## FS-UAE process

Native `run --emulator` validates emulator and readable Kickstart paths, rejects
`--capture`, builds the requested Amiga profile and stages a private run.
Resolve FS-UAE from flag, environment, then PATH; resolve Kickstart from flag or
environment only. Do not scan the disk, fetch media or rewrite user settings.

Use the established directory-drive recipe from `docker/scripts/run.sh`:
A1200, supplied KS3.1, volume label `CNEXT`, boot priority zero. Stage the binary
as a fixed guest name (for example `program`) and `s/startup-sequence`.
Only these files are mounted, from a dedicated `drive/` child of the run
directory. Configuration and emulator state are siblings outside that mounted
child. Source names and host paths are never interpolated into guest shell statements.

Create unique run storage outside the current workspace's entire canonical
`build/clorraine/` artifact tree. Another source or profile's build may reset
its own child directory while this emulator remains alive. Checking only the
current binary's directory does not protect that lifetime.
Also exclude the current program's canonical output directory. A child
artifact-directory symlink can place that directory outside the canonical
artifact root; root containment does not replace this existing protection.
Use both component-aware exclusions, without scanning other directories.
Reject a canonical staging parent containing a comma before launching FS-UAE,
with a diagnostic naming TMPDIR and requesting a safe location outside that
artifact tree. FS-UAE embeds the drive path in comma-delimited UAE mount
syntax; literal process argv alone cannot preserve that directory boundary.
Ordinary safe temporary parents retain existing behavior; do not silently
choose another location or change user settings.
Require an explicit private configuration with `end_config=1` to suppress
additional configuration loading. Use a private working directory, and direct
FS-UAE base/cache/save-state/log output into this run's private storage. Supply
path controls early enough to govern emulator initialization. The generated
configuration names only the staged drive and the approved A1200/ROM settings.
Do not inherit CWD `Config.fs-uae`, user `Default.fs-uae` or `Host.fs-uae`, extra
drives, boot overrides or shared writable state; leave all user files untouched.

Configuration/arguments use absolute host paths and direct process arguments.
Encode configuration values according to FS-UAE syntax and reject
control-character injection. Verify the exact controls and effective mounts
and state paths on recorded emulator versions in the acceptance environments;
unsupported isolation behavior blocks the proof. Merely supplying drive 0,
an explicit config, or a clean test home is insufficient. Do not mount the source
tree or share one mutable directory between runs.

FS-UAE's INI reader treats whitespace-prefixed semicolons as comments, and its
option reader trims terminal whitespace even in direct arguments. Supply
ordinary absolute host paths through literal CLI path options early enough for
initialization, rather than inventing INI escapes. The private explicit config
still contains `end_config=1`; do not set that termination flag early enough
to suppress reading the private config itself.

For the one unrepresentable case where the selected readable ROM filename ends
in ASCII space, copy its exact bytes to a fixed safe private name such as
`media/kickstart.rom` under this invocation's run root. This is an ordinary
private local file **outside** `drive/`, not a guest-mounted file, shared cache
or release input. Pass its absolute path to FS-UAE. Preserve the original ROM
and keep the copy until the child stops; one run cannot remove another run's
copy. Ordinary spaced/punctuated ROM names retain their original literal
argument path, and control-character paths are still rejected.
This narrowly approved local media copy is never packaged, uploaded or included
in published evidence. Additive process/lifetime specifications and actual
version-specific acceptance verify the exception before it is accepted.

Wait for the emulator to exit. Clean only this invocation's staging after its
process has stopped, including launch/wait failure paths. Normal cancellation
must not delete a directory still mounted by a surviving child. A hard kill
may leave private stale files; another invocation must neither adopt nor erase
them as live state.

## Result matrix

| Condition | Observable result |
|---|---|
| Native FS-UAE exits with code N | Driver returns N; no claim about guest exit status |
| Child terminated without a code or failed to start/wait | Report emulator failure, exit 1 |
| Missing required tool/media | Name the missing input and configuration remedy, exit 1 |
| Invalid explicit SDK, malformed/inapplicable option, native emulator capture | Usage/environment diagnostic, exit 2, no emulator launch |
| Explicit FS-UAE/ROM flag with legacy SDK selection | Exit 2 before build; native SDK selection remedy; no mode switch |
| Legacy marker path, no native override | Existing handoff text and capture behavior unchanged |
| Host/vamos capture | Existing output-file and guest/program-status behavior unchanged |

Use the current text/NDJSON error machinery and terminal sanitisation. Actual
path arguments preserve spaces and punctuation; sanitised display strings must
not replace filesystem paths.

## Compatibility and verification

Keep `compiler/tests/clorraine_driver_spec_test.rs` as the historical container
oracle. Add `native_driver_spec_test.rs` for native contracts and
`native_driver_impl_test.rs` for resolution/staging internals. Test process
selection with distinct temporary executable helpers, then prove the real
installed tools in acceptance. A helper proving argv/status is not proof of
an Amiga boot.

Local core packaging and installed-layout staging must exist before the first
packaged-driver observation; see [03-04 §Payload identities](03-04-editor-and-release.md#payload-identities-and-local-producers).
Native acceptance uses the packaged driver, user-installed FS-UAE and local
licensed KS3.1 on a private Xvfb display. Verify the guest's known visible
output before closing the emulator, then verify the process result and staging
lifetime. This test-side observation does not implement native `--capture`.
Include hostile inherited configuration with an outside writable drive and
conflicting boot settings. Inspect effective mounts/model/ROM and private
state/log paths; outside sentinels must remain unchanged. Two simultaneous runs
must have distinct writable state as well as distinct guest drives.
Retain the existing A500/KS1.3 gates; an A1200 launch of a nix13 binary is not
evidence of A500 compatibility.

Specification cases: ST-26–ST-39. Review every build/run call site, profile
branch, capture path, symlink boundary and cleanup path.
