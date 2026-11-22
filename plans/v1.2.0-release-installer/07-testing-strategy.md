# Testing strategy: native Linux delivery

> **Parent**: [Index](00-index.md)
> **Authority**: AR-18/19/20/21/22/23/24/25

## Oracle and verification

The cases below derive from the approved requirements and component contracts.
Write specification tests before implementation. A failing assertion requires
an implementation fix or an explicit specification ruling; never weaken it to
match the code. Existing frozen tests remain intact.

**Task verify**: `make check`, plus each task's declared new specification file.
**Task verify evidence**: 4,232 Rust tests and 160 Python tests observed,
2026-11-16; fuzzer smoke 28 cases, zero failures; baseline `daa2fba0`.
**Verify (gate)**: `make verify`.

Run `make vscode-check` for editor changes and `make site-check` for site changes.
Run full verification for shared build/test infrastructure changes, phase exit
and candidate acceptance. Scope selection does not turn host tests into proof
of Ubuntu package compatibility or an Amiga boot. Baseline evidence is in
[02](02-current-state.md); native evidence is still uncollected.

Full gates use the private-display prerequisite in
[03-01](03-01-support-builds.md#private-display-gate-prerequisite), including
infrastructure-change gates. Run its red tests without an emulator first, fix
both wrappers, then prove the actual private emulator environment before any
full gate. A caller's `DISPLAY` alone is not proof.

Candidate acceptance has two explicit stages. Pre-publication checks require
ST-53 and ST-54, applicable earlier cases, evidence-integrity controls and
independent review; they do not require public-install evidence. ST-55 is
retired (AR-49). Post-publication checks require ST-56 and matching approved
hashes. A stage-aware acceptance invocation
must state which evidence is required; missing required evidence fails rather
than skipping. Publication approval follows the pre-publication gate.

## Test slices and declared files

No test map exists. The measured task command is the approved scoped opt-in;
new Python suites must run explicitly until connected to `make check`.

| Slice | Specification files | Task invocation beyond `make check` | Release/acceptance |
|---|---|---|---|
| Support | `tools/distribution/tests/test_support_spec.py` | Python unittest discovery in that directory, pattern `test_support_spec.py` | ST-1–ST-10 on real support archives |
| GCC specs paths | `tools/distribution/tests/support_gcc_relocation_spec.py` | Explicit five-method unittest class in the same isolated real acceptance context | Both Ubuntu cohorts; original six archive methods still run unchanged |
| Support internals | `tools/distribution/tests/test_support_impl.py` | Existing unittest discovery, pattern `test_support_impl.py`; alongside the 93 local specification methods | Does not substitute for any real-archive method |
| Support check wiring | `tools/distribution/tests/test_distribution_check_impl.py` | `make distribution-check`, included unconditionally in `make check` | Missing real-context inputs still fail both actual acceptance groups |
| Private gate | `compiler/tests/native_gate_spec_test.rs` | Rust suite in the pinned container; host-side real-environment observation after green | ST-60 before any full gate |
| Driver | `compiler/tests/native_driver_spec_test.rs` and existing `clorraine_driver_spec_test.rs` | Both Rust suites in the pinned container | ST-26–ST-39 against installed candidate; real emulator for ST-37 |
| Local core packaging | `tools/distribution/tests/test_core_package_spec.py` | Explicit `CoreArchiveTests` and `CoreStageArgumentTests`; whole-file discovery includes mandatory actual groups and errors without their inputs | ST-57: `CorePackageArtifactAcceptance` and `CoreInstalledAcceptance` on both supported cohorts before Phase 2's installed-driver proof |
| Installer | `tools/setup/tests/test_native_manifest_spec.py`, `test_native_install_spec.py` | Both Python files through existing setup unittest runner | ST-11–ST-25 with candidate assets on both Ubuntu versions |
| Installer positive context | `tools/setup/tests/native_manifest_acceptance_spec.py` | Explicit `NativeManifestActualAcceptance` class; no implicit skip or replacement with local mocks | Positive ST-12 on both actual cohorts; same actual-input contract reused for later installer/bootstrap proof |
| Installer state contexts | `tools/setup/tests/native_install_acceptance_spec.py` | Explicit `NativeInstallActualAcceptance`, `NativeInstallAbiTransitionAcceptance`, `NativeInstallForeignRootAcceptance`; no implicit skip | Initial/update/negative cases on both cohorts; ABI transition on actual Ubuntu24 with real Ubuntu22 seed; dedicated tmpfs ownership rejection |
| Bootstrap packaging | `tools/distribution/tests/test_bootstrap_package_spec.py` | Python unittest discovery, pattern `test_bootstrap_package_spec.py` | ST-58 before Phase 3's installed-package gate |
| Bootstrap actual context | `tools/distribution/tests/bootstrap_package_acceptance_spec.py` | Explicit `BootstrapPackageActualAcceptance` class with the required actual installer context | Same exact shell/helper/manifest/payload bytes and closed imports on both real cohorts; missing context errors |
| Editor | `tools/vscode/test/actions.spec.test.ts`, `discovery.spec.test.ts` | Existing Node test command and `make vscode-check` | ST-40–ST-46; Linux desktop observations |
| Source selection | `compiler/tests/editor_sources_spec_test.rs`, `tools/vscode/test/actions.spec.test.ts` | Rust integration suite plus editor gate | ST-59 real LSP and multi-file driver execution |
| Approved Phase4 corrections | `compiler/tests/editor_review_spec_test.rs`, `tools/vscode/test/actions.review.spec.test.ts` | New independent Rust/Node regressions, existing helpers and runners, plus all frozen suites and private full gate | Clean-after-save invalidation; full member-chain policy; final task-variable input rejection |
| Release | `tools/distribution/tests/test_release_spec.py` | Python unittest discovery, pattern `test_release_spec.py` | ST-47–ST-52 against drafts and eventual public bytes |
| Candidate | `tools/distribution/tests/test_acceptance_spec.py` | Explicit candidate test invocation with required asset paths; no implicit skip | ST-53, ST-54 and ST-56 plus real-machine records; ST-55 retired |

Use existing Python unittest, Rust integration and Node runners. Python filenames
follow existing `test_*` discovery and distinguish `_spec` from `_impl`. A release
or acceptance invocation fails if a required tool, ROM, platform or candidate is
missing; it does not report a skipped test as passing. Local unit suites use
fixtures for external process/transport failures. Keep actual platform evidence
separate and mandatory.

### Real editor acceptance procedure

These are required future observations for ST-46, not evidence from the Node API
fixtures. Bind each run to the exact packaged candidate and its VSIX. Task4.2.6
owns the Linux desktop observation; Phase6 owns final candidate repetition.
WSL2 and Remote-WSL are outside v1.2.0 (AR-49) and are not acceptance steps.

**Inputs and isolation:** Record date, Ubuntu22.04/24.04 x86_64 and Python ABI,
VS Code version, distribution/core/support identities, compiler commit and
SHA-256/lengths of assets, installed binaries, VSIX and saved sources.
Use an owned disposable login/HOME and editor user/extensions profile, not a
personal installation. Keep checkout/development binaries inaccessible.
Establish the private authenticated display prerequisite before emulator work.
Use a real isolated desktop session on that display and launch VS Code through
its desktop launcher, without Clorraine tools in inherited PATH. Record actual
extension-host and emulator process identities and effective display/auth
pair; never launch verification emulators on the primary desktop.

Use this saved two-file input in an owned directory containing spaces:

```text
main.clo:
module app.main;
import { println } from std.io;
import { message } from app.message;
func main(): i32 { println(message()); return 0; }

message.clo:
module app.message;
export func message(): str { return "editor-installed-proof"; }
```

The required output is `editor-installed-proof` plus newline. Host/vamos status
is zero. Validate these input bytes with the real candidate before crediting
acceptance; the authoring task has not compiled or executed this example.

| Actual context | Required actions and observations |
|---|---|
| Linux desktop discovery | Install the exact VSIX using the installer's printed command into the isolated editor profile. Restart via the real desktop launcher. Privately inspect only the extension-host PATH field to prove neither installed bin nor development tools are inherited. With path settings empty, observe the installed `~/.clorraine/bin` LSP/driver identities and matching hashes, not just equal version banners. |
| Editing and tasks | Grant workspace trust. Introduce a deliberate `i32`/`null` error, observe real Problems diagnostics, restore valid saved code and observe them clear. Hover and Go to Definition resolve `message` across files. Build/Run host and both Amiga profiles; retain visible process-task output, selected installed driver, containing-root CWD, ordered explicit source argv, expected stdout and status. |
| Saved sources and negative controls | Save a distinct dependency output marker and rerun. The installed CLI with entry only must fail for the missing dependency; the editor's complete selection then succeeds. With valid default tools still present, set an invalid user LSP/driver override and observe actionable failure without fallback/task; clear it and re-prove success. Reload/reactivate and invoke one Build, observing one task and working language features. |
| Native Linux FS-UAE | Set user/machine SDK, FS-UAE and licensed KS3.1 paths. Invoke Run in FS-UAE for both Amiga profiles on the private authenticated display. Observe the real A1200/KS3.1 guest line, effective process/configuration/isolation and closing behavior. Emulator process completion is not guest exit status. Keep ROM/media and display authentication private. |
| WSL2 / Remote-WSL | Out of scope for v1.2.0 (AR-49); no Windows editor, WSL tool-execution or manual Windows-emulator guidance is required or claimed. |

Retain dated candidate-bound, redacted logs/screenshots and their hashes.
Never publish user paths, secrets, authentication files or licensed media.
An API facade, fake LSP/tool signature, headless process-task observation or
Linux desktop pass is sufficient only for the Linux desktop acceptance it
actually exercises. No WSL/Windows support or automatic shell/extension change
is introduced.

### Actual installer context and positive root/guidance proof

The supported-cohort obligation already applies to installer ST-11–ST-25.
Task-level local suites may use miniature metadata and true external-boundary
failure fixtures. They do **not** claim successful installation on the
development host (currently Ubuntu 26.04/Python 3.14).
Positive ST-12 stays mandatory in an explicit real-context unittest group,
not a skipped or weakened local test.

The task 3.1.1 author also creates
`tools/setup/tests/native_manifest_acceptance_spec.py`, containing
`NativeManifestActualAcceptance`. This focused specification module does not
match the generic local `test_*.py` discovery pattern. Invoke it explicitly:

```text
cd tools/setup/tests
python3 -m unittest native_manifest_acceptance_spec.NativeManifestActualAcceptance -v
```

The invocation requires `CLORRAINE_INSTALL_ACCEPTANCE` to name an ordinary
UTF-8 JSON file with exactly these fields:

| Field | Value |
|---|---|
| `schema` | Integer `1` |
| `ubuntu` | `"22.04"` or `"24.04"` |
| `abi` | Respectively `"cp310"` or `"cp312"` |
| `bootstrap` | Absolute path to the ordinary directory containing the exact producer's three output files |
| `bootstrap_sha256` | Exactly `install.sh`, `install.pyz`, `manifest.json` mapped to their expected SHA-256 values |
| `payloads` | Map of all twelve manifest asset identities to absolute ordinary input-file paths with matching byte lengths/hashes |
| `work_root` | Absolute path to an empty user-owned disposable directory, disjoint from all inputs and on the installation filesystem |

Reject missing context, unknown/duplicate keys, mismatched hashes, unsupported
or simulated cohorts and unavailable prerequisite packages/tools as explicit
test errors, never skips. The actual process's Ubuntu release and CPython ABI
must match the record; do not set platform/interpreter probe overrides.
The test harness itself is copied into an isolated verification directory;
the checkout, original SDK and development binaries are not mounted.
Clear `PYTHONPATH`, use only declared system prerequisites on PATH, and bind
candidate inputs read-only. Tests create and remove only children of their
owned work root; never alter the source candidate or prior accepted artifacts.

Existing proved runtime image inputs are:

| Cohort | Existing image configuration ID |
|---|---|
| Ubuntu 22.04 / Python 3.10.12 | `sha256:07e5d109f0438311482ea2b780e6084614f571250f08a248290b854ec36f8f85` |
| Ubuntu 24.04 / Python 3.12.3 | `sha256:46b397e434a7871e1dbb8a043c927d1f1004f4de4714d736a74de56136d0b2a1` |

A fresh read-only probe confirms both cohorts and host C tools, but these
earlier images do **not** have curl installed. Before positive installer proof,
prepare each acceptance environment with only its declared Ubuntu prerequisites,
including curl/CA packages, and record the resulting immutable image ID and
actual package versions. No host package change, dependency compilation,
simulated successful prerequisite or new supported OS/Python is authorised.
These existing images are starting inputs, not a claim that Phase 3's positive
proof has run or that unchanged image IDs include the missing packages.

Actual candidate inputs come from the local `assemble` command, using the
accepted current core archive
`eed3a17570ebd6fd312d808f28f724d9f3f4a10f1a2d1e42aa35a4e8f8738fb0`,
unchanged accepted Phase 1
archives/audits/lock, and a freshly built current VSIX from the existing package
command. The upstream NDK is the exact accepted lock source. Candidate final
URLs need not be public for local proof: use the approved copied test runner
and exact-byte transport fixture, with no platform/prerequisite/preparation/
wheel/smoke overrides. Fixture API lookup responses name the producer's release
and exact manifest URL; route payload responses from the context's actual files.
Keep normal installer validation/assembly/smoke/activation code unchanged.
Transport simulation is not proof of live TLS or public anonymous downloads.

The mandatory positive method installs using `--release v1.2.0-r1` into an
owned path containing spaces and punctuation, checks selected manifest identity,
stable links/current, all three real executable versions, retained exact VSIX,
and runs host plus both Amiga-profile Hello programs through that new driver.
The expected line is `Hello, Clorraine!\n`, status zero.
Execute the printed PATH command in a fresh POSIX shell with an unrelated
existing PATH entry and prove the selected driver comes from the new stable
`bin` while that existing entry remains. Parse and run the printed VSIX command
against a test-only logging `code` executable: it must receive exactly
`--install-extension` and the exact retained VSIX path. The installer must not
have invoked that executable itself or modified shell startup/extension state.
This checks command usability without automatically installing an extension.

Before implementation, the local specs are expected red for absent production
modules/commands; an actual-group invocation without its required candidate
context must fail explicitly. Record that as **unavailable actual context**,
not a behavioral red result, test pass or acceptance. After implementation,
positive ST-12 must pass in both cohorts before Phase 3 can close, regardless
of local-suite results. Later installation/update/bootstrap actual groups reuse
these context/input ownership rules; they do not inherit a pass from ST-12.

### Actual installer state/update inputs

Task3.1.2's positive state cases use the same actual-context schema, not
handwritten completed receipts or miniature successful compilers/venvs.
`CLORRAINE_INSTALL_ACCEPTANCE` names the initial `v1.2.0-r1` context.
`CLORRAINE_INSTALL_UPDATE_ACCEPTANCE` names a second record of the same shape
for a separately assembled `v1.2.0-r2` candidate. Each candidate has its own
exact shell/helper/manifest hashes and complete payload-path map. The update
retains identical core asset identity/bytes and changes the support identity;
the already accepted support archive bytes may be reused under the new
immutable support identity. No old archive, lock, audit or manifest is edited.
Use the local producer to assemble both candidates before isolating the tests.
Both helpers retain normal metadata/download/assembly/smoke/activation checks.

Put task3.1.2's actual groups in the test-only module
`native_install_acceptance_spec.py`, outside generic local discovery:

| Explicit class | Required real context | Obligation |
|---|---|---|
| `NativeInstallActualAcceptance` | Initial and update records for the process's actual supported Ubuntu/ABI | Successful install, same-release rerun/settings preservation, support-only update, prior-version retention and real host/both-vamos output before/after whole-root relocation |
| `NativeInstallAbiTransitionAcceptance` | Ubuntu24/cp312 update record and `CLORRAINE_INSTALL_PREVIOUS_ROOT` | Fresh cp312 environment after a real earlier cp310 installation; old version/settings preserved, new host/both-vamos output and relocation work |
| `NativeInstallForeignRootAcceptance` | A real supported-cohort initial record and `CLORRAINE_INSTALL_FOREIGN_ROOT` | Refuse a dedicated, writable but foreign-owned disposable tmpfs root without writing installer state |

Invoke either class explicitly through the existing unittest command, as for
the manifest actual group. The first class must pass on both cohorts. The
second runs on real Ubuntu24 after a separate real Ubuntu22 bootstrap seed;
it is not a class silently skipped on Ubuntu22. Missing candidate/seed context
is an explicit error in the corresponding invocation.

For the ABI-transition seed, the parent first installs the initial exact
candidate in Ubuntu22 with its real CPython3.10 and records successful
host/both-vamos output, exact bootstrap/payload hashes and selected installation.
Keep that disposable root on a shared owned volume, stop all its processes,
then mount it read-write into the Ubuntu24 proof at
`CLORRAINE_INSTALL_PREVIOUS_ROOT` (an absolute path).
The seed is outside the new record's empty `work_root`, ordinary/user-owned,
and explicitly disposable; never select a live user installation.
Validate its retained manifest, selected cp310 descriptor and complete runnable
host installation before the update. The old cp310 environment need not run
under the new incompatible interpreter before repair, and must not be mutated
to make that happen. Retain its exact file bytes and old installation after
the new cp312 activation. Tests must not delete the seed; parent owns its
eventual cleanup.

Local interruption/lock/path/failure tests may use real filesystem/process
operations, sentinels and bounded external fault helpers. A local old-program
observer can establish byte/path preservation, not successful installed
compiler/venv behavior. Successful ownership recognition, smoke-tested
completion, idempotent reuse and ABI migration still require the actual groups.
No private transaction filename, receipt schema or production test hook is
introduced by these test-data contracts.

Targeted preparation/wheel/smoke faults and exact before/after-current death
tests use the copied runner's external `Popen`/rename bindings specified in
the release document. Reachability must be observed; do not credit an arbitrary
nonzero result or approximate the rename boundary by polling.

For real foreign ownership, the parent supplies the explicit
`CLORRAINE_INSTALL_FOREIGN_ROOT` only to the test harness. It is an absolute
path to a fresh empty dedicated tmpfs mount owned by UID/GID65534, mode1777;
the test process is an ordinary user with a different UID. Use the existing
Docker execution surface, for example these added run options:

```text
--user <ordinary uid>:<ordinary gid> --read-only --network none --tmpfs /foreign:uid=65534,gid=65534,mode=1777
```

Set the test-harness variable to `/foreign`, and mount only its actual input/
verification directories read-only and its ordinary owned work root read-write.
No root container process, host chown, host sudo, privileged flag, Docker socket,
unrelated system directory or persistent foreign-owned host file is needed.
Validate the real mount/owner/mode, initial emptiness and different effective
UID before using it. It is intentionally writable by the test UID: rejection
cannot pass merely because filesystem permissions prevented an unsafe write.
The root must stay empty after rejection; preserve a canary in the separate
owned work root. Remove no user files. Docker destroys the tmpfs when its
disposable container exits.

This foreign-root class is explicitly invoked in the supported environment,
not imported into generic local discovery and not skipped when its context is
missing. Missing/unsuitable tmpfs or actual candidate inputs are test errors.
A parent read-only-container probe already verifies this mount mechanism with
process UID1000 and mount UID65534, including creation/removal of a test-owned
child, without mounting host paths. That probe establishes fixture feasibility,
not an installer rejection or positive acceptance result.

## Specification Test Cases

### Support inputs and payloads

| ID | Input / scenario | Expected output / behavior | Source |
|---|---|---|---|
| ST-1 | Support lock with a branch ref, abbreviated revision, missing LHA source, or missing archive hash | Lock/build validation fails naming the incomplete input; no asset is eligible | R2/R12; 03-01 §Locked inputs; AR-18 |
| ST-2 | Staged source HEAD differs from the full lock, or build attempts an undeclared download | Build stops; no successful provenance/payload output | R2/R12; 03-01 §Locked inputs |
| ST-3 | Real support archive built on the oldest baseline; install only declared packages on fresh Ubuntu 22.04 and 24.04 | All shipped host executables/native extensions load; no undeclared library or newer GLIBC requirement | R1/R2; 03-01 §Payloads |
| ST-4 | Exact NDK archive plus locked helpers/preparation inputs | Required ordinary and 1.3 header-tree paths/content match the locked reference inventory; notices retained | R4; 03-01 §NDK preparation |
| ST-5 | One changed byte in NDK, missing helper, or injected archive path | Preparation fails before unverified content executes or escapes staging; no completed SDK marker | R4/R6; AR-20/28 |
| ST-6 | CPython 3.10 and 3.12, network disabled, respective local wheel sets | vamos imports and runs the hello fixture; no index/build-backend request or local compilation | R3; 03-01 §vamos |
| ST-7 | Wrong-ABI wheel set, missing machine68k/greenlet wheel, or source archive offered instead of wheel | Offline installation fails clearly; no compilation fallback or success marker | R3; AR-18/27 |
| ST-8 | Emit portable corpus C, build with packaged GCC for nix13/nix20; compile shipped shim families | Portable outputs match existing goldens in packaged vamos; all applicable shim builds pass existing strict flags | R2/R3/R7; 03-01 §Manual proof |
| ST-9 | Move support/prepared SDK/venv root to a path with spaces; original prefix, repository and builder inaccessible | ST-6/ST-8 still pass; all executed tools come from relocated root or declared system prerequisites | R1/R2/R3; AR-18 |
| ST-10 | Payload containing NDK-derived headers, unaccounted library, absent licence/source, or an absolute builder path | Payload audit rejects it; valid audited payload includes its required notices/source and excludes media | R2/R4/R13; 03-01 §Payloads |

### Installer

| ID | Input / scenario | Expected output / behavior | Source |
|---|---|---|---|
| ST-11 | Clean supported Ubuntu user, prerequisites present, valid release manifest/assets; FS-UAE absent | Activate complete installation; host hello and both Amiga-profile vamos hellos print fixture output; retain VSIX and print setup commands | R3–R6/R10; 03-02 §Success and failure output |
| ST-12 | `--root` path containing spaces and `--release v1.2.0-r1` | Select exactly that distribution/root; quoted guidance works; no shell startup or extension state changes | R5/R10; AR-20/22 |
| ST-13 | Lookup includes newer support/component releases, draft and published distribution; exact selection names a component tag | Default selects published distribution only; exact component selection fails; neither support nor component is designated latest | R5/R12/R13; 03-02 §Entry and trust |
| ST-14 | Unknown option, missing argument, traversal-like tag, unsupported architecture or Python 3.11 | Nonzero failure with applicable guidance; no activation or fallback to other ABI/source build | R1/R3/R5; AR-12/18/20 |
| ST-15 | JSON duplicate keys, wrong schema/type, missing field, duplicate identity, inconsistent core/VSIX/support metadata | Reject metadata before payload use; old installation remains selected | R6; 03-02 §Metadata contract |
| ST-16 | Changed helper/asset byte, truncated download, wrong declared length, stalled connection or HTTPS downgrade redirect | Bounded failure; unverified helper never executes; no activation | R6; 03-02 §Entry and trust / Transaction |
| ST-17 | Archive member `../escape`, absolute path, external link, symlink-parent chain, duplicate destination, FIFO/device or oversized expansion | Reject archive; outside sentinel unchanged; no special files or writes outside staging | R6; 03-02 §Extraction and process controls |
| ST-18 | Missing GCC/C headers, Python venv capability or declared runtime library | Nonzero prerequisite result naming remedy; no sudo/apt process launched and no installation-success message | R3; AR-27 |
| ST-19 | Install into unrelated nonempty/foreign-owned root or substitute managed path with escaping symlink | Reject without changing unrelated sentinel files or active installation | R5/R6; 03-02 §Owned filesystem |
| ST-20 | Repeat successful install with same release/ABI/bytes | Same working selection remains valid; no duplicate active state, lost settings or automatic pruning | R5/R6; 03-02 §Transaction and interruption |
| ST-21 | Update to support-only revision with identical core asset hash and changed support identity | New complete installation active; core version/hash unchanged; new support used and prior version retained | R6/R12; AR-20/22 |
| ST-22 | Fail download, NDK preparation, wheel installation or any required smoke test during update | `current` still names old complete installation; it can still run; incomplete candidate not reusable as complete | R3/R6; 03-02 §Transaction and interruption |
| ST-23 | Kill installer before activation and at the activation boundary; rerun afterwards | Pointer selects old or new complete state only; lock released by process death; rerun succeeds without deleting prior working state | R6; AR-20 |
| ST-24 | Hold installer A's root lock; start installer B for same canonical root through another path spelling | B fails clearly before mutation; A retains exclusive activation authority | R6; AR-20 |
| ST-25 | Upgrade system Python cohort from 3.10 to 3.12 then rerun; move installed root afterwards | Fresh matching venv selected; previous installation/settings retained; host and vamos smoke pass after move without stale entry-script shebangs | R3/R6; AR-18/20 |

### Driver

| ID | Input / scenario | Expected output / behavior | Source |
|---|---|---|---|
| ST-26 | Distinct valid SDKs selected by CLI, env, adjacent install and legacy marker in descending combinations | Highest supplied priority wins; subprocess log proves selected compiler/runner paths, not merely preflight checks | R7; 03-03 §Resolution |
| ST-27 | Invalid CLI/env/selected adjacent SDK with valid lower-priority SDK available | Exit 2; no fallback compilation/launch; diagnostic identifies selection | R7; AR-19 |
| ST-28 | Invoke installed driver through stable `bin` symlink from unrelated CWD, then relocate whole root | Adjacent SDK found from actual executable installation root; build/run works from both locations | R7; 03-02 §Owned filesystem; 03-03 §Resolution |
| ST-29 | Pause build after resolving tools; activate a different installation before subprocesses run | Compiler and vamos still use original coherent resolved installation | R6/R7; AR-20 |
| ST-30 | Host run/capture, emit, help/version, freestanding build/run with native tools; irrelevant FS-UAE environment set | Existing command/output/status semantics preserved; unused tool settings do not require emulator/media | R7; 03-03 §Integration surface |
| ST-31 | Legacy SDK path with no native override, Amiga `--emulator` with and without capture for both profiles | Existing handoff specification assertions pass unchanged, including A500/ADF selection for 1.3 | R7; AR-19/29 |
| ST-32 | Native hosted-amiga build/run in debug/release/size and hosted-amiga-13 equivalents; printing exit-7 program on both packaged Python cohorts, with/without capture | Existing flag arrays and hello output preserved; driver returns exactly 7 and expected output/capture for failure fixture; launcher forcing zero fails the test | R3/R7; 03-03 §Resolution / Result matrix |
| ST-33 | Native `run --emulator --capture`; flags on host/build/emit; missing value; explicit FS-UAE/ROM flags with legacy SDK, both profiles and capture settings | Exit 2 before build/launch; diagnostic identifies unsupported/inapplicable input or native SDK remedy; irrelevant environment values do not alter legacy handoff | R7/R8; AR-19 |
| ST-34 | Distinct emulator paths in CLI/env/PATH; distinct ROM paths in CLI/env | Correct precedence; exactly selected program/media used; missing executable/unreadable ROM yields exit 1 with remedy | R8; 03-03 §FS-UAE process |
| ST-35 | Source/root/ROM paths with metacharacters/control bytes; hostile CWD Config and user Default/Host configs naming outside drive and boot overrides | Literal arguments and fixed guest name; unsafe config bytes rejected; explicit isolation suppresses inherited drives/settings and keeps outside sentinels unchanged | R7/R8; 03-03 §FS-UAE process |
| ST-36 | Emulator helper waits then exits 0/7; separate helper terminates by signal or cannot launch | Driver waits, returns 0/7 respectively or 1 on signal/launch failure; no claim about guest status | R8; 03-03 §Result matrix |
| ST-37 | Real packaged native driver launches hello on A1200/KS3.1/private Xvfb for both profiles, with ST-35 hostile configs present | Expected guest line observed; effective model/ROM and only staged drive verified; private base/cache/state/log paths verified on recorded emulator versions; process status/cleanup correct | R8; AR-14/24 |
| ST-38 | Two emulator invocations with separate completed builds, followed by same-source rebuild while first waits | Mounted files stay unchanged; distinct drives/configuration/writable state/logs; configuration/state are outside mounted drive; one exit removes only its own files | R8; AR-19 |
| ST-39 | Native missing compiler/vamos, compiler error and JSON diagnostics mode | Existing failure status class retained; required tool/error named; JSON stays valid and human display sanitised | R7; 03-03 §Result matrix |

### Editor

| ID | Input / scenario | Expected output / behavior | Source |
|---|---|---|---|
| ST-40 | Saved `.clo` entry importing another user source, each Build/Run/Run in FS-UAE action and applicable profile | Process task passes ordered explicit dependency paths; real two-file program builds/runs; dependency omission control fails; output visible; emulator action rejects host/freestanding | R9; 03-04 §VS Code actions / Saved-source selection |
| ST-41 | Dirty entry save success/cancellation/failure; dirty selected dependency; entry changes during selection; no file-backed `.clo` | Only valid saved selection launches; dirty dependency prompts save/retry without auto-saving unrelated files; no stale-buffer build | R9; AR-21 |
| ST-42 | Untrusted workspace with executable path overrides and candidate development binaries | No Build/Run process launch or workspace executable discovery; actionable trust message | R9/R10; AR-21 |
| ST-43 | Second-root or standalone entry and imports; spaces/metacharacters; source basename beginning with hyphen | Request root and CWD match containing root/parent; every selected absolute path is a literal argv element; no shell or unsupported `--` delimiter; real driver accepts argv | R9; AR-21 |
| ST-44 | Desktop-launched editor with installed tools absent from PATH; machine override, invalid override and workspace-only override variants | Default install discovered; valid machine override honoured; invalid explicit setting errors; workspace override cannot silently select executable | R10; 03-04 §Tool discovery |
| ST-45 | Existing LSP actions plus new commands, profile change and extension deactivate/reactivate | LSP behavior remains covered by existing tests; action profile follows setting; command registrations disposed without duplicate tasks | R9/R10; 03-04 §Tool discovery |
| ST-46 | Packaged VSIX on the Linux desktop using the installed candidate | Editing diagnostics, Build and Run work; native Linux FS-UAE action works with configured media; no WSL/Windows requirement | R9–R11; AR-21/49 |

### Release and candidate acceptance

| ID | Input / scenario | Expected output / behavior | Source |
|---|---|---|---|
| ST-47 | Workflow definitions and manual inputs including path/shell syntax | Only manual dispatch, validated identities, pinned actions/build inputs, scoped credentials and bounded concurrency/timeouts; invalid input rejected | R12/R13; 03-04 §Two bounded workflows |
| ST-48 | Distribution candidate core archive and VSIX at v1.2.0 | Three executable binaries with correct core version; required first-party and third-party licence files present; VSIX metadata matches | R2/R10/R14; 03-04 §Two bounded workflows |
| ST-49 | Existing support/core/VSIX identity with different bytes; identical rerun; interrupted component/distribution promotion | Changed bytes rejected; identical assets verified before reuse; retry never mutates released identity or exposes unavailable references | R12/R13; AR-22 |
| ST-50 | Initial or support-only candidate with missing/private support/core/VSIX/source payload; changed evidence/hash, unresolved major or absent approval | Distribution promotion blocked; component/support tags not latest; all pre-publication controls/review precede approval; changed approved bytes invalidate approval; missing ST-56 blocks final completion only | R13; 03-04 §Draft acceptance and public promotion |
| ST-51 | Support-only v1.2.0-r1 candidate references exact prior core bytes | Core is reused; distribution identity advances; candidate tests use revised support and record both identities | R12; AR-22 |
| ST-52 | Current release docs/version records and preserved v1.1.2 historical record | New records agree on 1.2.0 and current licence set; published v1.1.2 facts/terms remain historical and unchanged | R14; AR-25 |
| ST-53 | Shipped bootstrap candidate on fresh Ubuntu 22.04/24.04, prerequisites only, repository/original SDK inaccessible, `PYTHONPATH` cleared | CLI/LSP/VSIX present; host/vamos success and exit-7, both profiles, updates/failures and relocation pass; helper/manifest/payload hashes recorded | R1–R10; AR-24 |
| ST-54 | Candidate native FS-UAE and full legacy gates, all displays private | ST-37 observed; `make verify`, `make vscode-check`, `make site-check` pass; no desktop interference or media in published evidence | R7–R10/R13; AR-24 |
| ST-55 | Retired: WSL2/Remote-WSL Linux host acceptance | Not part of v1.2.0 under AR-49; no Windows/WSL support is claimed or required | — |
| ST-56 | Later approved promotion followed by public quick-start; fresh anonymous download | All links/hashes resolve and documented installation succeeds; public evidence identifies exact released assets without credentials/media | R5/R13/R14; AR-22/25 |

### Early producers and saved-source selection

| ID | Input / scenario | Expected output / behavior | Source |
|---|---|---|---|
| ST-57 | Local current-version core archive and proved support inputs; stage installed layout before native-driver observation | Three binaries/notices, canonical adjacent SDK and companion vamos available without checkout; versions match current inputs; no public tag/version bump; relocated native tests use actual archive | R2/R7; 03-04 §Payload identities and local producers |
| ST-58 | Packaged shell/helper/manifest/current VSIX, checkout inaccessible and `PYTHONPATH` cleared; remove required helper module or corrupt helper bytes | Production bootstrap installs through same validators; complete `.pyz` closure works; missing module fails proof and bad hash prevents execution; no metadata rewriting or source imports | R5/R6/R10; 03-02 §Entry and trust; 03-04 §Payload identities and local producers |
| ST-59 | Real `clorraine/sourceFiles` requests: transitive/qualified/same-module imports, value fields/shadowing, unrelated main, saved changes/overlays, second root/standalone, invalid URI/path/symlink, missing import, old/unavailable LSP; complete index of 2,000 files/closure of 256, then one extra | Fresh saved-disk entry-first stable absolute paths; value fields do not invent modules; unrelated programs/overlays excluded; exact caps succeed only after full closure processing; overflow fails during discovery with no partial result/task; removing either budget fails controls; real CLI builds and existing LSP behavior retained | R9/R10; 03-04 §Saved-source selection |
| ST-60 | Copied wrapper/.env fixtures with conflicting display/auth values; direct emulator and run-wrapper chains; unset callers; actual private Docker/FS-UAE observation after fixes | Both wrappers preserve caller private display/auth, retain no-override defaults, and keep handoff text; actual emulator uses private pair; no primary `.env` edit; removing either preservation fails | AR-24; 03-01 §Private-display gate prerequisite |

## Test data and negative controls

- Reuse existing hello and portable corpus outputs; do not rewrite spec goldens.
- Use temporary fake HTTPS responses only for transport failure injection.
  Real candidate tests use actual downloaded bytes and verified metadata.
- Tiny executable helpers can log argv, wait on a pipe and return a chosen
  status for driver tests. Use real subprocesses and filesystem operations.
- Installer tests keep sentinel files outside staging and a known-good old
  installation to prove failed updates preserve usable state.
- Use distinct tool signatures in precedence tests. A version-only assertion
  cannot detect invoking the wrong compiler.
- Prove assertions fail without the feature: native resolution tests fail on
  the current marker/handoff driver; action tests fail on absent commands;
  support tests fail on incomplete locks/payloads; unsafe archive tests include
  controls that would escape without validation.
- Imported-source proof must fail if dependency selection is replaced by
  entry-only argv. Packaged-vamos proof must fail if its launcher forces zero.
  Native isolation proof must detect inherited extra drives when suppression is
  removed. Public promotion proof must reject missing core or VSIX even when
  support is already public. Evidence controls run before any publication.
- Record red failures per case. Existing compatibility cases may already pass;
  record why rather than artificially breaking an immutable test.

## Implementation tests and external evidence

After each green specification slice, add focused `_impl` / `.impl.test`
tests for parser boundaries, archive member ordering, symlink resolution,
lock-release/activation races, cleanup errors, task disposal and packaging
internals. Use small modules and the existing runners.

External steps are structured manual acceptance, not new automation platforms.
Record OS/Python/FS-UAE/VS Code versions, candidate hashes, commands and observed
outputs in the v1.2.0 release evidence. Keep private paths/media out of published
logs. ST-37/ST-46 are not considered passed by an argv mock.

Spec authors receive these cases and the owning contracts without reading new
implementation logic. Reviewers check every affected call site/parallel path
and whether a negative control fails when the fix is absent.

## Execution evidence

Entries appear newest first. Each records the state at its timestamp; the top
entry is the current Phase5 result. Older entries retain their historical
task counts and holds.

### Linux-only scope decision — 2026-11-22 11:42

The user ruled **“linux only. no WSL”** (AR-49). v1.2.0 is Linux x86_64 only:
WSL2/Remote-WSL and Windows access are removed from requirements, acceptance,
Phase 6 and release claims. ST-55 is retired; candidate-green is ST-53 and ST-54,
with ST-56 post-publication.

The extension README no longer documents Remote-WSL or manual Windows transfer.
The VSIX was rebuilt after that documentation change: 25,749 bytes, SHA-256
`c0b4e4ae93e652f5cb065326417b52c10aac09889054132a6e2aac3c985811eb`.
`make vscode-check` passed with a fresh packaged client, and the release suite
passed 27/27 against the new VSIX and the unchanged core. The candidate scope is
now explicitly Linux-only; no WSL/Windows acceptance or support is claimed.

### Phase 5 review correction and re-review — 2026-11-21 20:35

The initial independent reviews found one major workflow-definition defect
(column-zero heredoc bodies broke YAML) and nine major correctness findings plus
five security minors. All necessary corrections are in pushed fix tree
`971e62b5`:

| Correction | Evidence |
|---|---|
|Workflow YAML and shell validity|Both workflows load with PyYAML; every `run` body passes `bash -n`; inline Python compiles|
|Validation output|Both validate jobs write to `$RUNNER_TEMP`|
|Support producer handoff|SDK/wheel assembles pass `payload` directories; `bundle-preparation` builds the locked descriptor/input/helper bundle|
|Draft handoff|Distribution uses `support_run_id` and the support workflow artifact with `actions: read`|
|Build pins|Rust 1.99.0 and Node 22.23.3|
|Publication gate|Read-only promotion job; support/component published first, every manifest URL anonymously hash-checked, then distribution|
|Partial reruns|Existing support/component/distribution assets compare and only missing assets upload|
|Candidate discriminating test|Actual test requires reviewed core/VSIX SHA-256 values and size floors|
|Coverage|`distribution-check` 139 tests; release suite 27/27; workflow scan covers all YAML, artifact handoff, public verification, rerun drafts and checkout-before-repository-command|

The single fix-diff re-review confirmed the accepted findings and found two
residual majors: the preparation descriptor lacked the required `base` inventory,
and rerun branches could add assets to a published release. Final fix tree
`8608a774` adds `base` (accepted by the consumer schema), requires
`isDraft=true` for every rerun, uses no-follow ordinary reads, creates the bundle
output exclusively, and pins the artifact/public/rerun paths in tests.

Final Phase 5 gates: `make check` exit 0 with 139 distribution tests and fuzz
smoke 28/28; `make site-check` production build exit 0; release suite 27/27
(16 specification, 11 implementation) against core `9960d775…` and VSIX
`c0b4e4ae…`. The earlier full `make verify` (4,796 Rust instances, all
acceptance legs) and `make vscode-check` receipts remain applicable because the
fix cohorts change release producers, workflows, tests and docs, not compiler or
extension runtime bytes. No third review, workflow dispatch, tag, release or
publication occurred. Phase 5 closes at 80/91.

### Phase 5 implementation and gates green — 2026-11-21 20:05

The release suite was authored first and ran red as intended: 13 cases, 16
subtest failures and one downstream JSON error for missing workflows, absent
`validate`/`verify-component`/`promotion`, 1.1.2 records and old docs. After the
bounded implementation the same suite passes unchanged.

The final core was built on Ubuntu 22.04.5 with glibc 2.35 and the pinned Rust
1.99 toolchain: three executables report 1.2.0, the archive is
`9960d77503bf5f392aa8d2631270a9b4af7672ce213838b6364e3418d350b2ab`
(18,544,640 bytes) and contains the five notices. The final VSIX is
`c0b4e4ae93e652f5cb065326417b52c10aac09889054132a6e2aac3c985811eb`
(25,749 bytes) with matching name, publisher and version.

Gates on the Phase 5 tree:

| Gate | Result |
|---|---|
|`make check`|exit 0; distribution-check 139 tests; fuzz smoke 28/28|
|`make verify`|`verify: all legs passed`; 4,796 Rust instances; all FS-UAE/legacy legs|
|`make vscode-check`|169 pass, 1 documented generic-suite skip, then fresh packaged client 1/1|
|`make site-check`|production build exit 0|
|Release suite|13 specification + 10 implementation cases pass against the actual candidates|

The first two A500 smoke attempts used an invalid host Xvfb (`AE=1`, then a
missing GLX context/authorization). The gate passed once the SDK image's proven
`Xvfb -ac +extension GLX` display replaced it; the successful `make verify` log
records `a500-smoke: OK`. No product change was made for the display correction.
The local core/VSIX pair is a candidate, not a published artifact; no workflow
was dispatched and no release, tag or publication was created.

### Phase 5 release suite authored and red — 2026-11-21 19:03

`tools/distribution/tests/test_release_spec.py` adds 13 independent cases for
ST-47–ST-52. The first run is red for the intended absent behavior: two missing
workflows, no `validate`, `verify-component` or `promotion` producer operation,
1.1.2 version records, and pre-release documentation. Sixteen subtest failures
and one downstream JSON error resolve to those absent surfaces; no production
file was changed to obtain the red state.

Historical document controls bind the published v1.1.2 record rather than
current targets: tag object `5beec93`, published binary SHA-256
`52f9e36461e27d85f06bdd7242f4ab0a4d3fe9ff07aa5eccabde9d9d33d38fbe`, the
2026-11-11 changelog heading, and the published MIT terms. The red log is
`build/tmp/native-phase5-20261121/spec-red/red.log` during execution and is
removed at session close; the case file is the durable evidence.

### Phase 4 closes after clean fix-diff re-review — 2026-11-21 12:05

The single applicable fix-diff re-review on `a753ada5… → e440a68a…` returned
no critical/major/minor findings from all four roles:

| Role | Independent result |
|---|---|
|Correctness/API|Pass; all three majors addressed;133 Rust/170 editor/strictTS and compiler/LSP probes|
|Local packaging/workspace-trust safety|SA-401 closed; no findings;133 Rust/48 editor and guard-removal mutation controls|
|Concurrency|No findings; approved event/save/session/disposal interleavings covered; receipt-only/no-shell|
|Amiga action/acceptance|No findings;48/48 correction specs; argv/CWD/profiles/guide unchanged|

CA-401, RV-C401/AR-47 and SA-401/AR-48 are resolved; DEF-581 is resolved.
Task4.3.4 verifies, Phase4 closes at67/91. Probe limits are explicit: the guard
is event-driven and not an atomic snapshot; disk-only/external writes and
post-submission changes are outside the approved guarantee; the helper falls
back to module selection for unreadable stdlib declarations; earlier desktop
evidence remains exact historical development bytes, not final1.2.0/Remote-WSL/
Windows acceptance. All review/gate scratch is cleaned; named evidence retained.
No third review, commit or publication occurred.

### Correction full gates green — 2026-11-21 11:56

Frozen fixe440a68a passes authenticated-private-display `make verify`4796 Rust
instances and all acceptance legs; explicit-input `make vscode-check` passes170
plus1fresh packaged client, zero skips. Empty authority is rejected, TCP is
disabled and wrapper/source/oracle identities match before/after. The fresh
VSIX's README matches source byte-for-byte. `phase4-fix-full-proof.json`
binds these results and the focused/shape controls. Owned gate scratch/logs/
display/auth are cleaned. The single applicable fix-diff re-review follows;
task4.3.4 remains `[~]` at66/91. No Phase4 closure, Phase5, commit or publication.

### Correction focused verification green — 2026-11-21

The independent delivered expectations pass unchanged: editor-focused160 cases
(including48 new correction specs), Rust123 across correction25/source44/
implementation22/historicalLSP32. Three additional implementation controls
exercise constant static-view length, const-sized array member length and a
finite recursive raw-struct path, with real explicit-file host runs before
selection and again through returned argv; all10 instances including7 shared
limits pass. Existing symbol4 assertions pass unchanged.

One combined library/integration command filtered integration cases; it is not
credited for those cases. The corrected unfiltered integration command ran10.
All original nine oracle and three newly delivered regression hashes remain
unchanged. Production adds only the approved request-local member helper,
existing selection plumbing and action guards. The nonfrozen ownership fixture
exposes no-op public editor events; no frozen fixture is edited.
Full current private-display/task/editor gates and one fix-diff re-review remain.

### Independent correction regressions red — 2026-11-21

Independent author `ses_e02687f87ffedyjT17taxhe9f8` delivers three new files,
bound in `phase4-fix-spec-red-proof.json`. Rust has8behavioral red/17green
controls (18 authored valid real-host examples plus7 inherited limits).
Editor has39behavioral red/9green controls, including the35-combination final-
input restriction matrix. Both extension build/stage commands pass.
The author reports no forbidden implementation/generated-JavaScript reads,
only opaque public API imports; all nine old oracles/support remain unchanged.
An initial imported-type-alias emitted-C fixture was replaced with an accepted
generic-record fixture before final red, not treated as product acceptance.
All delivered expectations are now frozen. All23 prior reviewed identities
still match before parent implementation. Spec-author scratch is cleaned.

### Three-correction approval and independent regressions — 2026-11-21 11:10

The user explicitly approves the consolidated CA-401/RV-C401/SA-401 corrections,
AR-47's bounded member/type helper and field-first policy, and AR-48's `${`
restriction. The existing contracts are amended before authoring. A fresh
implementation-blind specification author receives only the approved behavior,
public protocol/API boundaries and existing tests/support, with production
implementation reads forbidden. New Rust/Node files and focused support are
separate from all nine frozen original oracle/support files.

All23 reviewed non-plan identities are checked before authoring; no production
fix has begun. Behavioral red precedes fixes. Current correction scratch is
under `build/tmp/native-phase4-fixes-20261121/`, separately owned; original
review scratch remains cleaned. Gates/re-review remain pending at66/91.

### Phase 4 review/challenge reconciliation — 2026-11-21

All four initial reviews and both bounded independent recommendation challenges
are complete. Correctness independently passes Rust98/makecheck4427/editor122/
symbol4/strictTS and reproduces RV-C401 plus CA-401. Local packaging/workspace
safety passes editor122/Rust98,20 malformed-selection probes and null/budget
controls, and reproduces SA-401. Concurrency's no-shell review reports CA-401;
the Amiga specialist reports no findings after focused editor112 and actual-
evidence checks. The retained reports describe each verification limitation.

Challenges confirm the clean-after-save race with actual builds, identify the
longer-member selection policy/helper boundary, and confirm task-variable
rewriting across executable/argv/CWD despite dollar/backslash/quoting escapes.
No actual editor command or task process ran in the task-variable observers.
All owned review/challenge scratch is cleaned; named evidence remains.

Task4.3.4 stays `[!]` at66/91 on DEF-581. Three majors and AR-47/48 need an
explicit consolidated user ruling. No fix, frozen-oracle change, Phase5 start,
earlier-phase re-review, commit or publication is authorised. Earlier entries
below are chronological checkpoints, not the current review status.

### Phase 4 full gate green; independent reviews dispatched — 2026-11-21 10:41

Frozen treea753ada5 passes authenticated-private-display `make verify`:
4,761 Rust instances and every acceptance leg. The explicit-input editor
gate passes122 plus1fresh packaged-client case, zero skips. Empty-authority
access is rejected; TCP is disabled and wrappers match the accepted baseline.
All23 non-plan hashes match before/after. `phase4-review/full-gate-proof.json`
binds commands/log hashes, source identities and isolation. Owned gate display/
server/authentication/logs/scratch are cleaned.

The initial independent correctness/API, local packaging/workspace-trust safety,
concurrency and existing Amiga action/acceptance reviews are dispatched with
distinct owned child scratch roots. Exact sessions/fallbacks are retained in
`phase4-review/dispatch.json`. Complete generic project briefs preserve required
independence where named tools are unavailable; the Amiga tool is available.
No previous phase is reopened. No verdict or closure is claimed; task4.3.4
stays `[~]` at66/91 pending all results and any critical/major user rulings.

### Phase 4 full gate started — 2026-11-21

The final source/test/guide diff freezes at
`a753ada584c1812c3d2003491720f438a1cd1bfd` against `d20dcf26…`.
All23 changed non-plan paths match the approved Phase4 surface.
`phase4-review/packet-identity.json` binds their exact hashes; the adjacent
diff and review packet preserve the actual task evidence, approved minimum
surface and review obligations. Full `make verify` followed by the explicit-
input packaged-editor gate is running on an owned authenticated private display.
The runner checks source identities before/after commands and removes its
display/authentication in `finally`. No result is credited yet.

Independent read-only correctness/API, local packaging/workspace-trust,
concurrency and scoped native-action/platform reviews follow the full result.
Missing structured routing uses strict defaults; unavailable named generic
roles use their complete project briefs without weakening gates. The existing
Amiga specialist is limited to this phase's new actions/acceptance evidence;
accepted earlier phases are not reopened. No reviewer has been launched yet.
Task4.3.4 remains `[~]` at66/91; critical/major findings still require the user.

### Extension guide verified — 2026-11-21 10:15

Task4.3.3 passes fresh `make check`4,427 Rust instances and
`make vscode-check`122 plus1fresh packaged client, zero skips.
The VSIX's normalized `extension/readme.md` member is byte-identical to the
authored guide. Command/settings identifiers, current limits/capability and
shell-snippet syntax validate; all frozen oracles stay unchanged.
`phase4-guide-proof.json` retains command/log identities and scope limitations.
All task scratch/logs are cleaned. Progress66/91; full phase gate and
independent review remain before Phase5. No commit/publication occurs.

### Extension guide in verification — 2026-11-21

Task4.3.3 documents explicit VSIX installation and trust, installed/development/
user tool resolution, all five machine paths, saved-disk selection and limits,
the compatible version1 LSP capability, shared profile actions and native
A1200/KS3.1 FS-UAE behavior. Remote-WSL instructions distinguish the Linux
extension host from local Windows and require remote User Settings.

The manual Windows workflow names the generated Amiga executable, transfer
through the WSL share into a user-selected Windows directory, Linux SHA-256 /
PowerShell Get-FileHash comparison, mounting only that directory, licensed
media and manual guest shell startup. No automatic Windows launch or actual
Windows/Remote-WSL acceptance is claimed. Manifest identifiers, source limits/
capability and shell-snippet syntax validate. Current task/package gates run
before checking exact shipped README bytes and promotion.

### Saved-source implementation hardening verified — 2026-11-21 10:05

Task4.3.2 adds15 focused cases and reuses7 unchanged limit controls through
the existing helper module. Initial22 produce16green/6behavioral red. Every
failing binding example first passes a real host compiler build with explicit
expected sources. The defects concern initializer visibility, element-loop
scope, and missing global-lambda parameter metadata.

The existing shared symbol table is corrected, not replaced by a second
resolver: local/count-loop names become visible after their declarations,
element-loop names belong only to the body, and global initializers use the
existing expression visitor. `local_at` still recognizes a declaration's own
name token, preserving all historical inline unit assertions unchanged.
No compiler language rule, ownership, ABI or emitted-C behavior changes.

Final focused22, frozen saved-source44, historicalLSP32 and existing symbol4
pass. Fresh `make check` passes4,427 Rust instances; `make vscode-check`
passes122 plus1fresh packaged client with zero skips. Scope/comment/API/
call-site and frozen-oracle integrity checks pass. The shared symbols file
already exceeded700 lines including inline tests; its existing test layout
is preserved, with production below650 lines, rather than adding unrelated
structural work. `phase4-sources-hardening-proof.json` retains the evidence;
all owned task scratch/logs/fixtures are cleaned. Progress65/91. Phase-end
full gate/review and final hardened-candidate acceptance remain separate.

### Task ownership/configuration hardening verified — 2026-11-21 09:52

Task4.3.1 passes fresh `make check`4,405 Rust instances, final focused21/21,
and `make vscode-check`122/122 plus1fresh packaged client, zero skips. The
initial focused green run contained91spec+19impl; final editor verification
includes the later two disposed-rejection cases. All five original defects
also reproduce through the current unchanged observer against independently
compiled pre-hardening source5385a84e. Immutable specification/peer identities
are unchanged. Source/comment/API/scope self-checks pass.

`phase4-actions-hardening-proof.json` binds red/green, old-source negative
controls and exact source/oracle/gate identities. All task scratch/logs/
baseline copies/fixtures are cleaned. Progress64/91; saved-source hardening
is next. No phase-review or final-candidate acceptance, commit or publication
is inferred.

### Disposal/configuration hardening in verification — 2026-11-21

Task4.3.1 adds implementation-only controls with a focused external editor
boundary and the unchanged real client/LSP peer. Initial19 cases produce
14green/5behavioral red, zero skips or fixture errors. Explicit null user/default
paths incorrectly fall through; pending save/selection may submit after
disposal, and a task handle returned after disposal escapes termination.
The narrow corrections distinguish undefined from invalid null, gate asynchronous
continuations by activation lifetime, terminate late task handles and suppress
late deactivation error messages. No API, dependency, frozen oracle or compiler
surface changes. This is necessary hardening of the existing ownership contract,
not a new support surface.

Two additional disposed-rejection controls and import-error separation bring
the final focused suite to21/21 green. Running/completed/unrelated tasks,
reactivation, pending save/selection/submission and their rejected paths are
covered. Full task/editor gates are in progress; task4.3.1 remains `[~]`.
Desktop evidence remains bound to its exact earlier bytes; final candidate
acceptance and phase-end review still apply to the eventual hardened version.

### Installed desktop task verified — 2026-11-21 09:39

Task4.2.6 verifies after fresh `make check`4,405 Rust instances and the final
explicit-input `make vscode-check`101 cases plus1fresh packaged-client case,
zero skips. The initial editor invocation had100pass/1optional old-package skip
and fresh1pass because the explicit inputs were absent; it is retained as
history, not represented as the zero-skip final gate. All fourteen fresh VSIX
member bytes equal the exact desktop-installed candidate. Production/oracle
hashes, earlier Phase3's38 accepted identities and support lock stay unchanged.

`phase4-desktop-proof/proof.json` binds the actual observations, candidate/core/
VSIX/saved-source identities, selected screenshots and task commands/log hashes.
All remaining verification scratch is removed. Named current core/bootstrap/
evidence remains for phase review and reproduction. Progress is63/91; next is
implementation hardening4.3.1. Final actual Remote-WSL/candidate proof and the
Phase4 independent review remain mandatory. No commit/publication occurs.

### Actual desktop and native guest observations completed — 2026-11-21

The missing disposable-context graphics requirements were Mesa's software
driver and dynamically loaded `libGL.so.1`. Adding them only to owned images
lets real FS-UAE3.1.66 boot. Both actual editor profile actions display
`editor-installed-proof` in the A1200/KS3.1 guest. External process/configuration
observations confirm private HOME/cache/state/logs, `end_config=1`, one CNEXT
drive containing only program/startup script, DISPLAY`:120` and its private
authentication path. Closing through FS-UAE's F12+q removes each private root.
No emulator code is represented as guest exit status.

A PATH-only built-in Node debugger query in the actual extension host directly
returns `/tools/code/bin:/usr/bin:/bin`. It is not an API facade or replacement
extension; no candidate behavior is injected. The installed LSP's actual
`/proc/.../exe` bytes match65e66bb6. Core/VSIX/saved-source identities and every
frozen oracle remain unchanged. A later visible hosted-amiga Build/Run repetition
and language hover confirm output and features after relaunch.

`phase4-desktop-proof/` retains the current Ubuntu22 core archive/bootstrap,
example source, candidate-bound selected screenshots, redacted process and
emulator observations, exact image recipes/package versions and PATH-only
debugger result. Early captures before compilation completed, accidental
terminal-row action-icon clicks, a pending-save refusal and a palette-MRU
selection of FS-UAE instead of plain Run are not counted as missing acceptance
proof. The required actions were repeated explicitly and observed.

The owned container/editor/debugger, private display/authentication, licensed
ROM, runtime HOME/profile/cache, duplicate assets/fixtures and disposable GUI
image tags are removed. Original SDK/base images and the user's desktop remain
untouched. Current task/editor gates are running; task4.2.6 remains `[~]` until
they pass and final integrity/cleanup completes. No phase-review, final-version,
Remote-WSL, commit or publication acceptance is inferred.

### Actual installed editor observations; emulator environment pending — 2026-11-21

The unchanged Code1.147.0 Snap bundle runs in the owned Ubuntu22 context
after a test-image-only loader-path alias. Exact manual VSIX installation
succeeds. Desktop screenshots directly show null/i32 Problems diagnostics,
clearing after correction, the `message` hover signature and Go to Definition
opening `message.clo`. Host and both Amiga Build/Run commands execute the real
installed driver with entry-first literal source argv and containing-root CWD;
the external process trace records status0 for all six initial invocations.
Visible Run output matches the expected line. A saved dependency marker change
then appears in actual task output. Invalid user overrides show actionable
errors, and restoring defaults/reloading produces exactly one driver for one
Build. These observations do not replace final candidate or Remote-WSL proof.

The actual LSP child PATH is `/tools/code/bin:/usr/bin:/bin`, inherited unchanged
by the shipped client. Electron overwrites its own initial environment storage;
direct extension-host PATH inspection remains outstanding rather than inferred
as completed. An initial wrong SDK setting selects `sdk` instead of `sdk/amiga`
and is corrected only in owned User Settings. Real FS-UAE3.1.66 then starts
but fails its OpenGL context; no guest pass is credited. Missing Mesa libraries
are being supplied only in the disposable desktop image. Candidate, compiler,
VSIX and all frozen test bytes remain unchanged. Active scratch, authenticated
display, container and private licensed ROM remain owned and require cleanup
after the observation. Task4.2.6 stays incomplete at62/91.

### Real desktop inputs in preparation — 2026-11-21

Task4.2.6 is not complete. Personal desktop-control tools are unavailable and
the active user editor is untouched. The proposed actual editor run is an
owned non-root Ubuntu22 desktop context on an authenticated private display,
with a separate HOME, editor profile and extension directory. GUI dependencies
are supplied only in that disposable test context, not installed on the host
or added to installer prerequisites. The existing installed Code bundle is a
read-only test input, not a new shipped dependency or fabricated editor.

Source tree5385a84e is exported into an owned build directory and rebuilt
offline on the existing supported Ubuntu22 image using the SDK's already
prebuilt Rust toolchain. All three real development1.1.2 binaries build.
Unchanged public core/archive and release assembly commands produce private
candidate `v1.2.0-r3` with component `phase4-editor`, existing proved support
and the exact verified VSIX; all twelve measured asset identities match.
This private label is not final1.2.0 or publication.

The old frozen actual-context loader intentionally pins earlier accepted core
bytes. It is not edited or misrepresented as covering the new core. The new
manual editor proof uses exact fresh public producer bytes and the existing
download-boundary fixture, with no compatibility probes or compiler-success
substitute. Actual platform/ABI, installation/preparation/wheels, binaries and
three-profile user-program compilation remain real. The real default-root
installation now passes on Ubuntu22/uid1000: installed binaries match the new
public core inputs, VSIX matches4ea75b54, all three saved-import runs print the
expected line with status0, and omitted-dependency Build fails. Completed
build/toolchain/source copies are cleaned; candidate inputs and installed HOME
remain for the active proof. The isolated GUI context is still preparing;
no desktop observation, acceptance or task promotion is credited yet.

### Ordered editor group verified — 2026-11-21 08:33

All five AR-46 members verify separately after the unchanged complete gates
pass: `make check`4,405 Rust instances, `make vscode-check`101 Node cases plus
one fresh packaged-client/real-LSP case, and authenticated-private-display
`make verify`4,739 Rust instances with every SDK/sanitizer/differential/fault
leg green. There are no new skips. The empty-authority negative control fails
as required; TCP is disabled and wrapper bytes match accepted baseline.
The owned X server/authentication are removed after verification.

`phase4-cohort-verified-proof.json` binds source tree
`5385a84ea9a29149b47a6c20bb93688b1b4a616a`, all nine production hashes,
all nine immutable new oracles, exact commands/log hashes and private-display
evidence. Earlier Phase3's38 identities and historical tests remain unchanged;
1,103 baseline non-plan files are unchanged and only the five expected existing
production paths differ. The four focused new production files remain within
the declared scope. Changed-code documentation, invariants, exhaustive AST/
callers, file sizes, metadata and self-contained comments are reviewed.

The fresh VSIX contains byte-identical staged extension/client/discovery/action
modules, all three commands and five machine-only path settings. Exact bytes
are retained at `phase4-editor-proof/clorraine-1.1.2.vsix` under the reference
directory: SHA-256
`4ea75b54411262a9f3b99c9d704ea5273e30e4c37070a49b244faff23368aab7`.
This is an unpublished development candidate, not final1.2.0 or actual desktop
proof. All completed implementation/gate logs, temporary indexes and caches
are cleaned; named proof and VSIX remain for reproduction.

Tasks4.2.1–4.2.5 promote individually to62/91. The special scheduling group is
complete; normal ordering resumes at actual installed-editor desktop4.2.6.
Hardening4.3, phase independent review and final actual Remote-WSL/candidate
acceptance are still required. No commit/publication occurs.

### Approved editor group implemented; focused green — 2026-11-21 08:03

The user approves AR-46 at07:48. The five implementation members are written
in order and immediately marked `[~]`; none is verified. Machine-only command/
path metadata, tool discovery, a streamed saved-disk index/closure, request/
capability dispatch and trusted shared-client process actions are integrated.
The saved request remains separate from live editor overlays and reuses the
parser, loader's embedded references and existing symbol/binding metadata.
A focused AST walk classifies user references against the complete inventory.

The initial compiler run exposes one function-type parameter field access
error, corrected in implementation only. All44 saved-source cases and the
historical driver22/LSP32/version3 then pass. All101 editor cases pass without
skips, including the unchanged packaged-client check using the existing VSIX.
That is not fresh-package or real-desktop proof. All nine independent oracles
remain byte-identical; `phase4-cohort-progress.json` binds transitions and the
current source snapshot. Full task/editor/package/private-display gates are
next, followed separately by actual desktop proof and hardening/review.
No commit/publication or automatic task promotion occurs.

### Consolidated editor red inventory — 2026-11-21 03:47

Task4.1.4 records the complete independently authored Phase4 oracle before
implementation. The fresh consolidated run completed at03:58; prior authoring
receipts are not relabelled as this new run.

| Suite | Cases | Expected passing controls | Expected red |
|---|---:|---:|---:|
| Editor actions | 32 | 1 real-client startup | 31 missing commands |
| Discovery/settings/lifecycle | 59 | 7 compatibility controls | 52 absent approved behaviors |
| Real saved-source protocol | 44 | 4 actual explicit-file driver controls | 39 unknown requests + 1 missing capability |

All nine new test/support file hashes are frozen in the three named
`phase4-*-authoring-proof.json` receipts. Preserve the four historical Rust
test/support identities and three original editor tests, plus accepted earlier
source/oracle identities. Run the real LSP/version/driver suites separately so
the intentional new failures cannot hide compatibility regressions.
No new skip or fixture failure is eligible as expected red.

**Fresh consolidated result — 2026-11-21 03:58:** All135 new cases produce
12 passing controls and123 expected assertion failures. The editor run,
including its ten historical cases, is101=18pass/83expectedred/zero skips.
Source selection is44=4realcontrols/40expectedred. Separate historical LSP32,
version3 and driver22 pass. No collection/import/compile/fixture errors occur.
`make check` stops only at the40 new source assertions after2,810 earlier
Rust passes and4 new controls; later legs are not reached. The editor gate
stops only at83 expected assertions before fresh VSIX packaging.

`phase4-spec-red-proof.json` retains exact fresh commands, names/classifications,
all nine oracle hashes and integrity checks. All1,108 baseline non-plan files
are byte-unchanged; exactly nine test/support files are added outside planning
records. All previous source/oracles, documentation/local links and whitespace
checks pass. Completed logs/work/cache are removed. Task4.1.4 promotes at57/91.
This satisfies specification-first/red ordering, not implemented functionality
or Phase4 acceptance. No production, commit or publication change occurs.

After that verified checkpoint, AR-46/DEF-580 records the implementation-order
dependency: manifest4.2.1 requires full verification, while the frozen suites
need later4.2.2–4.2.5. A read-only independent schedule check concurs that a
fresh explicitly approved five-member cohort is the smallest correction.
All members would stay unverified until unchanged task/editor/private full gates
pass; explicit interrupted-cohort resumption and per-member documentation checks
are proposed. No approval is inferred; implementation is paused at57/91.
Actual desktop4.2.6, hardening/reviews and final/Remote-WSL proof remain separate.

### Saved-source specifications frozen — 2026-11-21 03:47

Independent task4.1.3 author and parent repeat44cases:4 real explicit-file
driver controls pass,39 requests fail as unknown method and1capability
assertion fails. No collection/compile/fixture errors or skips occur.
Host and both Amiga controls prove actual Build/Run, literal paths, expected
stdout/exit7 and dependency-omission failure; changed-import fixtures have an
independent real host control. Eight graph/type/shadow fixtures compile/run
before their absent-request assertion. All five permission preconditions prove
true uid1000 unreadability, with restoration and cleanup.

The new negatives reject unknown-method replies as selection diagnostics;
expected failures therefore cannot mask absent feature support as success.
Exact2000/256 and overflow2001/257 cases, last-selected-file imports, merged
modules and excluded-directory budgets are authored. Black-box tests do not
prove the internal streaming traversal; its later implementation controls stay
required. Deeper saved-selection behavior remains unproved until implementation.

Historical LSP32, exact version3 and driver22 pass unchanged. `make check`
passes2,810 earlier Rust instances plus4 new driver controls, then fails only
the40 new source-selection expectations; later Rust/bench/fuzz legs are not
reached. `make vscode-check` remains expected red83, with18 passes/zero skips;
fresh VSIX packaging is not reached.

Parent reviews frozen module/binding and approved request/root/overlay/budget
contracts, documentation and self-contained rationale. Compilation is warning
free. `rustfmt` is absent on host and SDK and no repository formatting gate
exists; manual style/size/whitespace checks pass, without source edits, downloads
or an added tool dependency. All1,108 baseline non-plan files,38 accepted
Phase3 identities and earlier Rust/Node oracles remain unchanged.

`phase4-source-authoring-proof.json` retains exact author/parent commands,
per-failure classifications, controls and three frozen hashes. Completed
author/parent logs/work/cache are cleaned. Only task4.1.3 promotes, at56/91.
No production/desktop/emulator/Remote-WSL acceptance, commit or publication.

### Discovery specifications frozen — 2026-11-21 03:24

Independent task4.1.2 author and parent repeat59cases:7 compatibility controls
pass and52 assertions fail for missing approved behavior. Categories are
installed LSP absent2, wrong LSP identity10, missing action command31,
configuration remedy3, workspace lookup before trust1 and machine metadata5.
There are no collection/import/fixture errors or skips. The31 command-blocked
cases do not yet prove their deeper behavior. All four live-profile diagnostic
observations pass before the corresponding missing-command assertion.

Strict test type-check, fixture syntax, build/stage and all ten historical
editor tests pass, including the explicit existing packaged client.
`make check` passes4,361 Rust instances. The combined `make vscode-check`
remains expected red:83 assertions,18 passes, zero skips. Its early test
failure prevents fresh VSIX packaging; no fresh package or desktop acceptance
is claimed.

Parent reviews approved tool precedence, machine-only setting inspection and
public lifecycle/profile expectations. Distinct actual executable identities
and real framed protocol observations are used, not compiler-success mocks.
All1,108 baseline non-plan files,38 accepted Phase3 identities and original/
frozen action tests remain unchanged. Documentation, self-contained rationale,
sizes, local links and whitespace pass.

`phase4-discovery-authoring-proof.json` retains author/parent command receipts,
per-case assertions and three frozen oracle hashes. The real editor procedure
above records unexecuted candidate-specific desktop/private FS-UAE/actual
Remote-WSL steps. Parent corrects the proposed manual dependency function to
`export` per the frozen module specification; its inputs remain uncompiled.
Windows transfer/emulator guide usability is retained without adding a native
Windows emulator acceptance leg. Completed author/parent work/logs/caches are
cleaned. Only task4.1.2 promotes, at55/91; no production/commit/publication.

Independent task4.1.3 author `ses_e04124d20ffej0IOAfooRt5ftn` receives the
approved saved-disk protocol/closure/root/limit contracts, frozen module/binding
rules and existing test-only framed client. Real explicit-file host/both-Amiga
driver controls must pass independently of missing selection behavior.
The SDK dev process is confirmed uid1000, enabling actual unreadable-file
controls without root/chown changes. Its `/work` maps to the repository;
author scratch is explicitly bound beneath the owned Phase4 root.
All earlier tests stay frozen, no production body may be read/edited, and no
desktop/emulator/Remote-WSL acceptance is credited by these specifications.

### Action specifications frozen — 2026-11-21 02:55

Independent task4.1.1 author delivered the action suite and two external
VS Code/LSP fixtures. Parent reviewed the approved observable contracts and
repeated all32 cases: one real-client startup control passes;31 assertions
fail solely at missing registered commands (Build16, Run8, FS-UAE7).
There are no collection/import/fixture errors or skips. Deeper assertions are
not yet reached and are not claimed as proven behavior.

Build/stage, strict test type-check and fixture syntax pass. All ten existing
editor tests pass, including the explicit packaged-client test with actual
VSIX/LSP inputs. `make check` passes4,361 Rust instances. `make vscode-check`
is expected red solely for the same31 command assertions, with11 passes and
zero skips; this is not a green editor acceptance gate.

All1,108 non-plan baseline files and38 accepted Phase3 source/oracle identities
remain unchanged, as do the three original editor tests. Parent checks
documentation, self-contained rationale, file sizes and local plan links.
`phase4-action-authoring-proof.json` retains exact commands, classifications,
case names and frozen hashes for all three new files. Completed author/parent
logs, scratch and Node caches are removed. Only task4.1.1 promotes, at54/91.
Production, desktop/emulator/Remote-WSL acceptance, commits and publication
remain untouched; continue independent discovery specifications.

Independent task4.1.2 author `ses_e042a9463ffeU6Wktcrxpvvikf` is dispatched
with the approved installed/development/PATH, machine-setting, trust and public
lifecycle contracts. Existing production bodies are forbidden to read or edit;
the three frozen action files and all historical tests remain immutable.
Only the discovery suite and minimal external fixtures may be authored.
Manual acceptance steps are proposed for parent recording, not executed or
credited. Parent will repeat red and task gates after the result returns.

### Phase 4 baseline and independent action author started — 2026-11-21 02:30

Phase4 baseline is `d20dcf2685edc5b3b7ea2fb0ac845b5c91a3c215`, including
all accepted uncommitted earlier work. Unchanged `make vscode-check` passes:
nine local tests and one explicit packaged-client test. The existing local
packaged skip is not counted as acceptance; its required explicit invocation
passes. All three existing editor-test hashes match, and the baseline log is
cleaned. `phase4-entry-proof.json` retains exact command/result/identities.

Independent author `ses_e0446eb27ffeT3KaVNU26LfLjP` writes only action/trust/
save/CWD specifications for task4.1.1. All production VSCode/Rust source bodies
are forbidden to read/edit. Existing tests stay immutable; only bounded
concern-specific external-API fixtures may accompany the new action tests.
Headless facade/task observations are not real desktop/compiler/emulator
acceptance. No Phase4 implementation or sibling test-task advancement occurs.
Progress53/91, no commit/publication.

### Phase 3 final disposition accepted — 2026-11-21 02:23

The user accepts the verified receipt fix and Phase3 closure. Parent confirms
the exact diff, final source/original/new test hashes and full acceptance
receipts, plus documentation/self-containment/reference checks. Task3.3.3
promotes at53/91. All Phase3 major findings resolve by their recorded fixes,
independent original/fix reviews and authorised final user disposition.
Six minors remain report-only; no third review or commit/publication occurred.

All own gate/runtime/media/auth/work/log/process cleanup is complete; named
private candidates and curated evidence remain for reproduction. Techdocs
auto-update is not opted in (`docs/index.md` absent); do not add an unrequested
documentation scaffold. Planned user/editor documentation stays in its owning
later tasks. Continue Phase4 independent editor specifications.

### Receipt-retention final verification handoff — 2026-11-21 01:36

All sixteen final state/recovery instances pass. Aggregate:31real actual
instances plus one real cp310 seed;6separate packaged mechanical instances;
68local specification+24implementation methods; unchanged makecheck4361Rust
and full private makeverify4695Rust. Every original method/subtest is retained.
Exact source/original/new test/candidate/prior-phase hashes match, and AST
comparison restricts the production difference to the completion method body.

`phase3-receipt-retention/verified-fix.diff` and adjacent proof retain the exact
one-method/new-test handoff. Parent authorship is recorded under the explicit
exception, not relabelled independent. All completed own logs/work/seeds/
private-display/auth/scratch/children are cleaned; no jobs/agents remain active.
Named receipts/candidates/original inputs/contextJSON remain for reproduction.
No third review is run. Final user disposition gates RV-Q303 resolution and
Phase3 closure (DEF-579); task3.3.3 stays blocked at52/91. Six minors remain
report-only, no commit/publication or live public TLS/WSL acceptance claim.

### Receipt-retention full private gate passes — 2026-11-21 01:27

Unchanged full `make verify` passes the frozen receipt-retention source with
4,695Rust instances and allSDK/sanitizer legs. Private host/container access
is authenticated, unauthenticated access rejected, desktop unused. Source,
old/new test and split-log hashes match before receipt acceptance.
`phase3-receipt-retention-full-verify-proof.json` retains exact evidence and
the runner recipe without authentication material. Server/authority/full-gate
work/logs are cleaned. Separate state acceptance remains active. Final user
disposition still gates RV-Q303 and Phase3 closure; no third review.

### Receipt-retention positive/ABI and packaged mechanics pass — 2026-11-21 01:26

Rebuilt entries pass15actual positive/foreign/ABI/SDK-isolation methods plus
the separate real Ubuntu22/cp310 seed. The parent-authored three receipt
methods additionally pass on Python3.10 and3.12 against the exact packaged
module. Only the test child import root is bound to the pyz; expectations and
production code are unchanged. These six passive mechanical instances are
reported separately, not counted as installed-runtime acceptance.

`phase3-installer-proof/receipt-retention-positive-result.json` retains exact
commands, images, source/test/candidate hashes, seed identities and separate
counts. All identities match. Completed positive/mechanical work/logs and the
owned ABI seed are cleaned; named receipts/inputs/contextJSON remain.
Sixteen state/recovery instances and full private verification remain active.
No third review or final finding/phase disposition occurs at this checkpoint.

### Receipt-retention local gate passes — 2026-11-21 01:00

The frozen receipt-retention source passes68local specification methods
(47original+18independent review regressions+3parent-authored receipt methods),
24implementation methods and unchanged makecheck4361Rust. Source,23original/
six review-regression/new receipt hashes match; no expectation changed after red.
`phase3-receipt-retention-local-proof.json` retains commands and identities.
Completed local-suite/makecheck logs and transient results are cleaned.

Real31actual+seed/ABI/SDK isolation, separate new receipt mechanics on both
supported interpreters and full private verification remain active. Parent
authorship is explicitly recorded, not independent authorship. Final verified
diff returns to the user; no third review or finding/phase closure yet.

### Parent-authored receipt regression red→green — 2026-11-21 00:46

Under the explicit limited authorship exception, the new three-method suite
reproduces one expected canary-deletion failure before code, with two passing
controls and no errors/skips. It forwards real fsync, observes the new file by
inode and substitutes only owned test directories. All three methods then pass
unchanged after removing failure-path pending unlink in the completion method.
AST and hash checks prove no other production or old/new test changes.

`phase3-receipt-retention-red-proof.json` records scope and non-independent
parent authorship accurately. Completed red/initial-green logs are cleaned.
New entries on frozen325c76ce261a3e87b780713c7aca9a334c21491e preserve manifests/
payloads. Local suites,31actual+realcp310seed/ABI/SDK-isolation, separate new
mechanical checks on both packaged interpreters and full private verification
are running. No third review is dispatched. Final user disposition remains
required before RV-Q303 or Phase3 closure. Progress52/91.

### Receipt regression author tool failure — 2026-11-20 11:42

The independent author tool returns a safety-flag error without tests or a
summary. The intended test file is absent and the author's scratch is empty.
No red result is claimed; production and all prior source/test identities
remain unchanged. Empty owned scratch is cleaned. No alternate-wording retry
or third review is started. DEF-578 blocks the approved correction's authoring
step; parent-inline authorship would need explicit user approval as a limited
independence exception. All other red/verification/final-disposition gates
remain unchanged. Task3.3.3 stays blocked at52/91.

### Receipt safe-retention correction authorised — 2026-11-20 11:39

The user approves the bounded RV-Q303 remedy after the plain explanation.
Independent specification author `ses_e07744de5ffecBuoLuuU5yIef1` adds one
regression file before production edits, with all six implementation boundaries
forbidden to read/edit. The exact prior source and original/new oracle identities
are reconfirmed before authoring. Tests observe real fsync/root substitution
and completion controls, not simulated successful installed-runtime behavior.

Only the existing completion method may change to retain failed pending
receipts; no new production surface, pruning or minor fix. Regenerate entries,
repeat affected local/actual/ABI/full private gates and return the verified
diff for final user disposition. This is specification authoring, not a third
review. No third automated review, commit or publication is authorised.
Task3.3.3 remains unverified at52/91; RV-Q303 is not yet resolved.

### Single fix review reveals receipt-cleanup regression — 2026-11-20 04:44

All three fix reviews finish. The original five findings are addressed, but a
new real-fsync/root-substitution comparison shows pending receipt cleanup
deletes replacement-root data after validation rejects the root (RV-Q303 major).
Existing passing local/actual/full gates did not cover that new failure path.
The reviewer confirms the old workspace defect is fixed on Python3.10/3.12;
this receipt finally-block regression is distinct.

`05-findings.md` records reproduction, exact source, contract and the proposed
one-method safe-retention correction. No fix is applied. Task3.3.3 is blocked
on the user ruling (DEF-577) at52/91. The single authorised review round is
complete; do not start a third automatically. Source/oracle/candidate bytes
are unchanged throughout review. All reviewer work/children are cleaned;
the parent retains the common packet only until curating the final receipt.
Six minors stay report-only, no phase closure/commit/publication.

### Corrected acceptance complete; single fix review active — 2026-11-20 04:38

All sixteen corrected state/recovery instances pass, preserving every original
method and subtest on both actual cohorts. With the fifteen positive/ABI/SDK
methods, there are31 actual instances and one separate real cp310 seed.
All65local specification/24implementation methods, unchanged makecheck4361Rust
and full private makeverify4695Rust pass the same frozen source. Exact source,
six new tests, twenty-three old oracles, candidates and prior-phase bytes match.

`phase3-correction-acceptance-proof.json` binds complete results and context
identities. Completed actual/full/local/red/author scratch, logs, seeds, private
display/auth and processes are cleaned. Named receipts/candidates/context JSON
and verification copies remain for review reproduction. The same three
independent reviewers now assess the single authorised266f7218→e5d2cef1 fix
diff. No findings close until they return, no third review is authorised,
and all six minors stay report-only. Progress52/91, no commit/publication.

### Corrected full private-display gate passes — 2026-11-20 04:29

Unchanged `make verify` passes the corrected frozen source with 4,695 Rust
instances and all SDK/sanitizer legs. Host/container private-display access
is authenticated, unauthenticated access is rejected and no desktop is used.
Source, original/new oracle and split-log hashes match before receipt curation.
`phase3-correction-full-verify-proof.json` retains the recipe and exact results
without authentication material. Private server, authority, logs and full-gate
work are cleaned. Separate state acceptance is untouched and still active.
The five findings remain open pending that evidence and one fix-diff review.

### Corrected actual SDK isolation and ABI proof passes — 2026-11-20 04:28

Corrected packages pass fifteen actual methods: seven previous manifest/
bootstrap/foreign-root/ABI methods plus eight new SDK-isolation methods on both
supported cohorts. Each new actual method bootstraps the real candidate and
checks real host/both-vamos execution; no successful process is substituted.
An SDK-less candidate cannot be rescued by a valid external SDK, and a valid
candidate is not displaced by a stale override. Both ordinary controls pass.
The separate seed is real Ubuntu22/cp310, confirmed by image and observed ABI;
the shared transition directory's24.04 suffix does not identify the seed OS.

`phase3-installer-proof/correction-positive-result.json` retains exact commands,
images, fifteen-method counts, seed identities and source/candidate/test hashes.
All six new test and twenty-three original oracle hashes still match. Completed
positive work/logs and the owned disposable ABI seed are cleaned; named inputs,
curated receipts and context JSON remain. State/recovery/full-private gates
are still active, followed by the one fix-diff review. No finding closes yet.

### Approved correction local gate passes — 2026-11-20 04:04

The frozen corrected source passes eighteen new local specification methods,
forty-seven previous local specification methods, twenty-four implementation
methods and unchanged `make check` with 4,361 Rust instances. All six new
test hashes, twenty-three old oracle hashes and production identities match
the corrected producer receipt. No test expectation changes after red.

`phase3-correction-local-proof.json` retains commands, counts and exact
source/test identities. Completed local-suite/make-check logs and transient
results are cleaned. Real-cohort/ABI/new SDK-smoke acceptance and full private
verification remain active, followed by one fix-diff review. No findings close
on this local checkpoint; task 3.3.3 stays unverified at 52/91.

### Correction oracle red and initial local green — 2026-11-20 03:51

The additive independent oracle has eighteen local methods and four mandatory
actual smoke methods. Local red: thirteen failure instances across eleven
methods, seven green controls, no errors/skips. Actual Ubuntu22/cp310 red:
two SDK-override failures and two controls, no errors/skips. Parent reproduces
both; the actual repeat uses the unchanged reviewed helper, not current source.
Missing-context errors remain separate and never count as red acceptance.

All eighteen local methods pass after the approved three-module correction.
`phase3-correction-red-proof.json` retains complete author/parent evidence and
six immutable new test hashes. All twenty-three prior oracle files and earlier
phase bytes stay unchanged. Completed author/red/initial-green logs and work
are cleaned. Corrected real-input producers retain identical manifests/payloads
and regenerate helper/shell pins on tree `e5d2cef1c0a951e9331d523928f12626291fd720`.
Both-cohort original acceptance/ABI, eight new real SDK-isolation methods,
existing suites and authenticated private full verification are active.
One fix-diff review remains mandatory. Progress52/91, no findings closed yet.

### Five bounded corrections authorised; independent tests first — 2026-11-20 03:23

The user approves the consolidated five-major correction packet. The ruling
is recorded in `05-findings.md`; DEF-576's decision dependency is resolved
without resolving the findings themselves. Task 3.3.3 resumes unverified.
Independent author `ses_e093a7017ffegtoKh0qbEDHjgA` writes additive regression
specifications against the existing contracts before any production correction.
The author is forbidden to read or edit all six implementation entry/modules,
and may not rewrite existing tests or count miniature fixtures as runtime proof.
The reviewed production and twenty-three original oracle hashes are reconfirmed.

Scope remains three existing installer modules, new regression files and only
necessary test bindings/registration. Real support audits already satisfy the
required source/notice obligations, so no accepted lock or payload bytes need
changing. Red proof, bounded corrections, regenerated candidates, repeated
real-cohort/ABI/full private verification and one fix-diff review remain pending.
All six minors stay report-only. No commit or publication is authorised.

### Consolidated Phase 3 review stop — 2026-11-20 01:25

All three independent reviews finish with five majors (RV-C301, RV-Q301/302,
SA-301/302) and one report-only minor (SA-303). Concrete probes establish
wrong-SDK smokes, partial completion blocking retry, substituted-workspace
cleanup, component-aware symlink escape and missing locked source/notice
obligations. Independent local suites still pass; the new probes reveal
boundaries absent from existing coverage. Exact frozen source/test/oracle/
candidate bytes remain unchanged. No green receipt waives these findings.

`05-findings.md` records role sessions, reproduction, contracts, the proposed
bounded correction and pending user ruling. Task 3.3.3 remains blocked at
52/91 (DEF-576). All reviewer-owned scratch/children are cleaned; the parent's
common packet is retained only until its source-bound review receipt is written.
No fixes, phase promotion, commits or publication are authorised or applied.

### Correctness review finds candidate SDK override gap — 2026-11-20 01:22

Independent real-tool testing proves `_smoke` can use an inherited external
CLORRAINE_SDK instead of the candidate SDK. Existing acceptance clears that
variable; its green result does not cover this boundary. RV-C301 is major,
requires a user ruling and blocks task 3.3.3. No fix or assertion changes are
applied. The exact hardened actual/full receipts remain honest source-bound
results, not a waiver. Parent confirms both smoke callers and explicit/env/
adjacent driver precedence. Full reproduction, scoped independent verification
and the proposed minimum remedy are recorded in `05-findings.md`.
The other two read-only reviews remain active; progress stays 52/91.

### Hardened actual acceptance complete; reviews active — 2026-11-20 01:16

All sixteen state/recovery instances pass on the exact hardened source,
including every original method and subtest on both actual Ubuntu/Python
cohorts. Together with the seven positive/foreign/ABI methods and separate
real cp310 seed, all final actual obligations pass. Unchanged full private
verification also passes with 4,695 Rust instances. Exact source, candidate,
twenty-three oracle and previously accepted compiler/driver hashes match.

`phase3-hardened-acceptance-proof.json` binds complete evidence and context
identities. All completed actual/full-gate logs, work, disposable seeds,
private display/authentication and owned processes are cleaned. Named private
candidates, original inputs, curated receipts and context JSON remain for
review reproduction. Three independent read-only fallback reviewers now assess
correctness, ordinary packaging safety and concurrency; their sessions and
packet requirements are recorded in `05-findings.md`. Review scratch is kept
only while those jobs are active. Task 3.3.3 remains unverified at 52/91 until
reviews and required user rulings permit closure. No publication is implied.

### Hardened full private-display gate passes — 2026-11-20 01:05

Unchanged `make verify` passes all repository acceptance legs on the frozen
hardened source, including 4,695 Rust instances and all SDK/sanitizer legs.
The private authenticated Xvfb recipe verifies host and container access and
rejects unauthenticated access; no desktop display is used. Source, oracle
and split-log hashes match before receipt acceptance.

`phase3-hardened-full-verify-proof.json` retains commands, summaries, exact
source/candidate/oracle hashes and the runner recipe without authentication
material. The private server is stopped, authority removed and all owned full
gate logs/runner/scratch cleaned. Separate active state acceptance remains
untouched. Final actual state/recovery results and independent reviews still
gate task 3.3.3; progress remains 52/91 with no commit or publication.

### Hardened positive and ABI acceptance verified — 2026-11-20 00:48

Seven actual methods pass on the exact hardened candidates: manifest/guidance,
bootstrap/closure and foreign-root rejection on both supported cohorts, plus
the real cp310-to-cp312 update/relocation. The separate seed uses the recorded
Ubuntu22/cp310 image, confirmed against its receipt and observed interpreter.
The console's seed suffix names the shared Ubuntu24 transition work directory,
not the seed runtime. No successful subprocess or platform probe is replaced.
All frozen source, candidate and twenty-three oracle hashes still match.

`phase3-installer-proof/hardened-positive-result.json` retains commands,
images, source/candidate hashes and the seed identity. Completed positive
work/logs and the explicitly parent-owned disposable seed are cleaned.
Context JSON and named exact inputs remain for reproduction; recreate empty
work/tmp directories before reuse. Separate state/full-gate jobs are untouched.
Sixteen state method instances, full private-display verification and independent
reviews remain pending; task 3.3.3 stays unverified at 52/91.

### Final hardened source frozen for phase acceptance

`phase3-installer-proof/hardened-producer-proof.json` records source tree
`266f7218206afac1a1c88248ee1c098cba6f4544` against the Phase 3 baseline.
The scope remains six production files, twenty-five test files and six owning
plan/roadmap files. Both real-input producers pass. Manifests and payloads
remain unchanged; hardened helper bytes and shell pins are regenerated.
All twenty-three independent oracle hashes and copied verification files match.
Final actual acceptance and authenticated private-display `make verify` must
run on these candidates before independent review. Earlier actual results do
not count as hardened-source acceptance. Task 3.3.3 remains unverified at 52/91.

### State/cache/activation hardening verified — 2026-11-20 00:22

Thirteen implementation-only state methods expose six red records in two
methods: boolean integer cache fields and malformed/aliased completion at
final activation. Eleven existing mechanical methods pass, including real
inode-lock alias/death/CLOEXEC, owned scratch cleanup, readiness rejection,
private cache copies/reuse, immutable/corrupt/incomplete cache behavior,
RENAME_NOREPLACE protection and special-file/alias inventory rejection.
These mechanical objects never count as a successful installed runtime.

Necessary source corrections enforce integer cache fields and share bounded
ordinary manifest/whole-inventory completion revalidation between reuse and
final activation, preserving literal selected-document comparison and adding
the candidate-ownership guard. All13 state methods,11 metadata methods,
47 frozen local specifications and unchanged make check4361 Rust pass.
All23 independent oracle hashes remain unchanged. The old state module
inside the earlier actual helper matches its recorded source hash, retaining
an exact pre-fix authority for the red controls.

`phase3-state-hardening-proof.json` retains source/red/green/task evidence.
Static Python3.10 syntax, documentation, size/reference/whitespace checks pass.
All owned logs/fixtures/scratch/processes are cleaned. Task3.3.2 verifies
at52/91. Fresh exact candidates, all23 actual instances, full authenticated
private-display make verify and independent review must now pass on this
final hardened source; prior actual checkpoint is not final-source proof.

### Metadata/archive hardening verified — 2026-11-20 00:01

The implementation-only boundary file initially runs10 methods and exposes
seven red records in four methods: blocking open/body deadline coverage,
preservation of another active process alarm and unknown-option terminal
controls before argparse. Six existing JSON/file/tar/ZIP methods pass.
Necessary source corrections add a main-thread total-transfer timer with
safe idle-handler restoration/active-timer refusal, and validate literal
producer argv before parser diagnostics. No new package, public option,
source recipe or old report-only minor fix is introduced.

All11 final boundary methods pass, including success/error timer cleanup,
with all47 frozen local specification methods and unchanged make check4361
Rust. All23 independent oracle hashes remain unchanged. Static Python3.10
syntax, entity documentation, imports, size/whitespace and reference-ban
checks pass. `phase3-metadata-hardening-proof.json` binds red/green/task/source
evidence. All owned task logs/fixtures/scratch/children are cleaned.
Task3.3.1 verifies at51/91. The helper source changed, so final phase acceptance
must regenerate exact candidates and rerun all actual instances after state
hardening; earlier actual checkpoint is not silently reused as final proof.

### Complete installer actual checkpoint verified — 2026-11-19 23:46

All16 current-source state method instances pass: every original method runs
on each real Ubuntu cohort without filtering subtests. This closes the
repeated smoke-failure retry regression and proves same-release reuse,
support-only updates, old bytes/settings preservation, all required child
failures/retries, managed-path rejection, prerequisite negatives, canonical
lock-holder death/retry, both exact activation-boundary deaths/retries and
whole-root relocation. Together with the seven positive/foreign/ABI methods,
there are23 mandatory actual unittest instances and one separate real cp310
seed. No unavailable-context errors are counted as passes.

`phase3-installer-actual-proof.json` binds all six implementation hashes,
all23 unchanged oracle hashes, exact candidate hashes, complete per-method
commands/images/results, current local gate and reproducible context records.
The same source passes all47 local methods and unchanged make check4361 Rust.
Accepted prior compiler/driver source and every input archive/media byte remain
unchanged. This is exact-byte-transport supported-runtime proof, not live
public HTTP/TLS, WSL/Remote-WSL, publication or final phase acceptance.

All owned completed actual logs, work directories and temporary ABI seeds
are removed after receipt validation. Every owned job/child is joined; no
author/reviewer/container job remains. Retain only named non-media receipts,
private candidates/current VSIX, local immutable runtime images, the23-file
verification copy and context JSON for hardening/review reproducibility.
Recreate empty tmp/initial-work/update-work directories before reusing those
contexts. Task3.2.10 promotes at50/91; Phase3 hardening/full gate/review follows.

### Final pipe source positive/ABI acceptance passes — 2026-11-19 23:26

The exact helper-only bounded shell passes seven mandatory actual methods:
manifest/guidance, bootstrap/closure and foreign-root rejection on each of
the two supported cohorts, plus the real cp310-to-cp312 update/relocation.
The separate real Ubuntu22 bootstrap seed also passes all three execution
profiles. No positive probe/process result is replaced, and the copied
verification tree excludes checkout/SDK source. Exact-byte transport remains
distinct from live public HTTPS/publication acceptance.

`phase3-installer-proof/pipe-bounded-positive-result.json` binds all six
implementation hashes and current producer-entry hashes to those commands,
images/results and the seed's exact payload identities. Parent verifies the
restored disposable seed is the expected cp312/r2 selected root, then removes
only that explicitly parent-owned seed and the completed positive logs.
Original payload/media files and prior accepted versions outside test storage
are untouched. Named candidates, curated receipts and reproducible context
bindings remain intentionally; separate active pipe-state-* work is untouched.
All47 locals and make check4361 Rust were reverified on the same source.
Sixteen actual state method instances still run; task3.2.10 stays unverified
at49/91 and no phase closure/commit/publication occurs.

### Helper-only pipe local checkpoint passes — 2026-11-19 23:15

The final helper-only bounded shell passes all47 frozen local specification
methods and unchanged `make check`/4361 Rust instances. Every one of the23
oracle hashes remains unchanged. `phase3-pipe-correction-local-proof.json`
binds all six implementation hashes to commands/results and exact source.
All owned local gate logs/scratch/children are cleaned. Separate real
acceptance contexts/jobs remain active; task3.2.10 is not promoted and no
final-source positive/update/ABI result is inferred from the local gate.

### Helper-only byte pipe mechanically verified — 2026-11-19 23:09

The corrected bootstrap successfully authenticates/launches its unchanged
helper four times with an existing128KiB+ external diagnostic log; the helper
then deliberately rejects the unsupported development OS. This establishes
handoff/retry plumbing, not simulated successful installation. An independent
oversize child attempts1MiB of stdout. Real system head writes exactly1025
bytes for a1024-byte pin; bootstrap fails before Python and cleans its owned
temporary files. `sh -n` passes and all23 frozen oracle hashes remain unchanged.
`phase3-bootstrap-pipe-correction-proof.json` retains the causal before/after
controls and exact shell hash. All mechanical scratch/children are cleaned.

Fresh actual r1/r2 producer entries under `phase3-installer-proof/pipe-bounded/`
preserve identical helper, manifest and payload bytes; only the bounded shell
changes. Complete local/make-check and all actual obligations rerun against
that exact shell. State verification executes every original method on both
cohorts in its own isolated work/container context, with four-worker scheduling
and the long independent case first. All eight original methods per cohort
remain included; no subtest/method expectation is filtered or changed.
Positive bootstrap/guidance, dedicated foreign-root and a fresh real
cp310-seed/cp312 transition also rerun. Task3.2.10 remains unverified at49/91.

### State retry catches process-wide download limit — 2026-11-19 23:00

The real state suites each pass seven of eight methods, including support-only
update/relocation, same-release idempotence, old-state preservation, canonical
lock-holder death, managed aliases, prerequisite negatives and both exact
activation-boundary deaths/retries. The required-child method's successful
retry fails for all three smoke operations on both cohorts (six subtest
failures), before Python starts, with the shell's helper-download diagnostic.

A separate mechanical reproduction establishes the cause without changing an
oracle: the helper-sized RLIMIT_FSIZE also limits every other file the curl
child opens, including an existing128KiB diagnostic log. A direct append raises
EFBIG, and the unchanged shell fails before helper handoff. This is not a
transport corruption, unsupported cohort or successful-runtime substitution.

Replace the process-wide limit with a pipe that writes at most the pinned
helper length plus one byte. The existing coreutils head command bounds only
the helper body, including headerless responses on older curl. A private
curl-success marker carries the first pipeline command's status; EOF alone
does not authenticate a failed transfer. Existing size/hash checks still
precede Python. No new package/framework/public option or test expectation
is introduced. Fresh exact producer outputs, repeated-log/oversize controls
and all affected local/actual gates must pass before task3.2.10 promotes.
Earlier positive/ABI/foreign receipts remain historical, not silently reused
as final-source acceptance. Progress stays49/91.

### Real ABI transition and corrected-source recheck pass — 2026-11-19 22:13

A separately bootstrapped real Ubuntu22/cp310 seed passes all three execution
profiles, then a real Ubuntu24/cp312 process updates the same disposable owned
volume. The mandatory ABI method passes fresh cp312 creation, exact old cp310
byte/settings preservation, host/both-vamos output and whole-root relocation.
No old venv is edited to fake interpreter compatibility; the test restores its
borrowed seed and leaves eventual seed removal to the parent.
`phase3-installer-proof/wheel-fixed-abi-result.json` records both images,
the seed control recipe, exact bootstrap/payload identities and results.

The corrected source again passes all47 local methods and unchanged
`make check`/4361 Rust instances with every frozen oracle hash unchanged.
`phase3-wheel-correction-local-proof.json` retains that source-bound evidence;
all local recheck logs/scratch/children are cleaned. Sixteen actual state/
update/death methods remain active; task3.2.10 is not promoted and the parent
retains actual proof/seed work only until verification and owned cleanup.

### Corrected real positive bootstrap accepted — 2026-11-19 22:09

Fresh current-source candidates pass four real positive methods: manifest/
quoted-guidance and bootstrap/module-closure on both Ubuntu22/cp310 and
Ubuntu24/cp312. They prepare actual NDK output, install the actual offline
wheels, pass candidate smokes and independent host/both-vamos execution, and
retain exact current VSIX/manifest bytes. No source checkout/original SDK is
mounted and no successful process/platform/prerequisite replacement is used.
The allowed exact-byte HTTP boundary does not establish live public TLS or
anonymous publication. Current-source foreign-root negative proof also passes
both dedicated UID65534 tmpfs cases under ordinary UID1000.

`phase3-installer-proof/wheel-fixed-positive-result.json` and
`wheel-fixed-foreign-result.json` retain commands/images/results. The candidate
manifests and all payload bytes are identical to the earlier local revisions;
only the wheel-allowlist correction changes helper bytes and corresponding
shell pins. Earlier failed positive attempts remain historical, not accepted
runtime evidence. Sixteen real state/update/interruption methods, the real
cp310-seed/cp312 transition and the unchanged full local recheck are active.
Task3.2.10 stays unverified at49/91; no phase closure/commit/publication.

### Actual verification catches wheel allowlist typo — 2026-11-19 22:01

Both real manifest and bootstrap groups reach NDK preparation under their
actual Ubuntu/Python cohorts, then reject the authentic wheel manifest.
The implementation mistakenly names pydispatcher, while both locked payloads
and frozen miniature fixtures specify pysdl2. Correct only that implementation
literal. Direct validation of both actual ABI wheelsets now passes. Every
oracle hash and accepted archive stays unchanged. The separately invoked real
foreign-owned writable-tmpfs rejection passes on both cohorts.
Task3.2.10 remains unverified; fresh exact producer entries/context hashes and
all affected actual/local checks must rerun. Earlier local implementation
checkpoint remains historical evidence, not a claim that this positive
runtime path already passed. No phase completion, commit or publication.

### Ordered implementation local checkpoint — 2026-11-19 21:53

The final source, including the independent shell file ceiling, passes all
three frozen local suites (24+7+16 methods) and unchanged `make check`
(4361 passing Rust instances, historical/new setup, support/benchmark/fuzzer
legs). All23 oracle hashes still match the current red-refreeze authority.
Each task3.2.1–3.2.9 is separately promoted only after this complete cohort
gate, with its concern-specific evidence and actual-proof limitations.
Progress49/91; no actual candidate installation method has yet passed.

`phase3-implementation-cohort-proof.json` retains exact six implementation
hashes, commands/results, resource-ceiling observation, current VSIX input and
links to the real-input producers/environment records. `sh -n` passes;
shellcheck is unavailable on the host and no result is claimed. All owned
integration logs, miniature diagnostics, ceiling fixtures and child processes
are cleaned. Private real candidate entries, VSIX, immutable runtime images,
the23-file copied verification tree and exact actual input contexts remain
intentionally for task3.2.10. Full acceptance/reviews and publication stay gated.

### Installer integration pending final gate — 2026-11-19 21:50

The nine ordered implementation tasks are authored, all still unverified.
Initial integration exposes producer asset-order, runtime-record shape and
empty-root permission-handling mistakes; production corrections preserve every
frozen oracle byte. VSIX identity is now compared before preparation can fail.
All47 local specification methods and unchanged `make check` pass on that tree.
An additional shell child-only file ceiling bounds headerless downloads even
with older supported curl. A real repeated-write probe attempts1MiB, receives
EFBIG at1024 bytes for a1024-byte helper pin, executes no helper and confirms
bootstrap storage cleanup. The final three-suite/unchanged-check rerun is active;
no integration task is promoted before it passes.

Disposable acceptance images derived from the exact two prior runtime images
now add only the missing declared curl/CA prerequisites. Runtime UID remains1000;
all installed package versions and immutable image IDs are recorded in
`phase3-runtime-prerequisites-proof.json`. Build provisioning uses the package
manager; runtime proofs remain non-root, and no host package/base image changes.
Provisioning scratch is cleaned; local image aliases stay for actual acceptance.

The unchanged existing VSIX package command builds fresh current1.1.2 input.
Accepted exact core/support inputs pass real local release production for both
`v1.2.0-r1` and `v1.2.0-r2`, including all four audits/wheel cohorts. The updated
shell producer preserves identical helper/manifest bytes and adds only the
independent download file ceiling. Private candidates and context/binding
records live under `phase3-installer-proof/`. Only the23 frozen test files are
copied into the verification tree; the forthcoming actual containers will not
mount the checkout/original SDK. No actual installation group has yet run, and
no simulated local result is substituted for it.

### Loader-only correction refrozen; implementation cohort approved — 2026-11-19 20:55

The user explicitly selects “Approve cohort (Recommended)” for AR-45:
implement tasks3.2.1–3.2.9 in order and keep all nine `[~]` until unchanged
`make check` and the three local suites pass. Actual context proof and full
phase/review/publication gates remain unchanged. DEF-575 is resolved.

Independently, the author repairs exactly four test-process import-path lines
in `native_manifest_producer_fixture.py`, with no assertions/signature/return
changes. Parent proves removing those four lines reconstructs the exact old
file hash; all other22 oracle files are unchanged. In Makefile's real setup
CWD, the missing leaf is now correctly `tools.distribution.manifest`, while
test `sys.path` identity/order and environment are restored after the call.
No child-runtime path or shipped helper byte is changed.

Parent reruns all three local suites, all12 unavailable-actual methods and
`make check`: the same47 local red methods/70 records,12 context errors and
29 historical setup passes remain. No installer production file exists at
this renewed red checkpoint.
`phase3-spec-red-refreeze-proof.json` is the **current**23-file hash/tree
authority; the original `phase3-spec-red-proof.json` and prior authoring
receipts remain separate historical evidence. All owned loader/refreeze
scratch/logs/children are cleaned. Progress40/91; production implementation
may now begin, but no implementation task is yet verified.

### Implementation gate ordering stop — 2026-11-19 19:17

Before production work, the parent records AR-45/DEF-575: automatic setup
discovery executes the public-shell oracles, whose valid-assembly prerequisite
cannot pass until the later producer and native modules exist together.
No early implementation task may be promoted while the unchanged task command
is red. The proposed ordered implementation cohort keeps tasks3.2.1–3.2.9
unverified until `make check` and all three declared local suites pass.
Actual context proof, full phase verification, reviews and immutable expected
behavior are unchanged. The user ruling is pending; no implementation starts.

A separate mechanical fixture check also finds that Makefile's
`cd tools/setup` removes the repository root from the test interpreter's import
path. A read-only probe reports no `tools` package in that CWD. The independent
author is asked to repair only the documented parser API's test-process import
binding, not any assertion, module signature, child environment or shipped
helper import path. Its corrected source must be reverified/refrozen before
implementation; original receipts remain historical evidence.

### Consolidated installer specification-first gate — 2026-11-19 19:14

Parent freshly reruns all three independent local suites: manifest24 with44
failure records, state7 with10, bootstrap16 with16. Total47 methods/70 expected
red records, zero local collection errors or skips. All process/producer cases
stop at their required valid-assembly control; structural cases stop at the
missing public parser. No test is weakened or declared green for an earlier
unrelated failure.

Explicit actual groups report eleven native plus one bootstrap missing-context
errors. These twelve methods are mandatory after implementation on their
declared real cohorts/seed/tmpfs contexts. Unavailable contexts are neither
behavioral red nor acceptance. Preservation/sentinel/old-program/update/death
assertions are authored but still unreached behind the absent producer; no
successful old installation or activation preservation is inferred.

`make check` stops on the known new setup suites: all29 historical methods
pass;31 feature methods produce54 red records. The bootstrap16 run separately.
Later compiler/distribution/fuzzer legs are not inferred green from this stop;
the unchanged pre-authoring baseline remains4361 Rust instances.

`build/distribution/reference-20261117/phase3-spec-red-proof.json` freezes all
23 source hashes and the full source tree ID in its `frozen_red_tree` field.
The complete change-scope assertion is exactly23 test files plus six owning
plan/roadmap documents against Phase3 baseline `1ecded4...`.
All six planned installer/producer production entry/modules are still absent.
Task3.1.4 verifies at40/91. No installation, green product acceptance, phase
closure, commit or publication occurred.

### Bootstrap oracle authored and verified — 2026-11-19 19:11

Independent author `ses_e0bb271d4ffeTLIjBoBhoO7L5O` delivers five new
test-only files for task3.1.3 with no forbidden implementation reads.
Parent reads all five and reproduces sixteen local expected-red methods
(eleven producer, five closure/integrity), each stopped by the required valid
assembly control because the producer is absent. There are no local collection
errors, skips or specification greens. The explicit actual method raises its
required missing-context error, not a behavioral red/acceptance result.

The parent also executes nine bounded mechanical checks on a harmless synthetic
ZIP: isolated imports, source-decoy resistance, validator-probe plumbing, four
missing runtime modules, status forwarding and missing entry. The original ZIP
stays unchanged. This validates test-probe mechanics only, not a working helper,
manifest implementation or installation. Synthetic code stays in owned scratch
and is never copied to production.

`make check` still stops only on the previously authored native setup suites:
29 historical passes, 31 expected-red methods/54 records. The bootstrap suite
is additionally run explicitly as declared; no later gate is inferred green.
All eighteen earlier oracle hashes remain unchanged. Source/documentation/
Python3.10 syntax/size/whitespace checks pass.
`build/distribution/reference-20261117/bootstrap-package-authoring-proof.json`
retains exact five-file identities, commands/results and mechanical evidence.
Task3.1.3 verifies at39/91; consolidated red proof follows before implementation.

### Installation/state oracle authored and verified — 2026-11-19 18:25

Independent author `ses_e0bb271d4ffeTLIjBoBhoO7L5O` delivers eight new
test-only files for task3.1.2 without forbidden implementation reads.
Parent reads every file and reproduces seven local expected-red methods
(ten failure records from the absent producer), with no skips/collection
errors. The separate mandatory groups report unavailable contexts explicitly:
eight ordinary actual install/update/negative methods, one ABI transition and
one foreign-root method. Those ten errors are not behavioral red or acceptance.

`make check` stops only on the two new native setup oracles: 60 methods total,
29 historical setup passes and 31 expected-red feature methods with54 failure
records. Later compiler/distribution/fuzzer legs do not execute beyond that
known feature red; the prior unchanged baseline remains 4,361 Rust instances.
All ten frozen manifest oracle files and five historical setup files remain
byte-identical to their verified checkpoints.

Parent separately executes fourteen bounded mechanical observer probes:
ten semantic subprocess/load/launch-failure cases and four real before/after
atomic-current death barriers, including directory-fd-relative `os.replace`.
Unmatched real processes forward unchanged, selected failures execute the real
nonzero child or matching external error, and all owned processes are joined.
These probes load no installer and cannot establish installed-state correctness.

`build/distribution/reference-20261117/native-install-authoring-proof.json`
retains exact source hashes, commands/results, baseline and probe recipe.
Documentation, Python3.10 syntax, size/whitespace and reference-ban checks pass.
Task3.1.2 verifies at38/91. No installer production code, actual installation,
phase closure, commit or publication occurs; real candidate groups still gate
Phase 3 closure.

### State negative bindings completed — 2026-11-19 17:20

The task3.1.2 author stops before writes and requests exact process/death and
foreign-owner fixture bindings, not private state APIs or actual candidates.
All ten frozen files still match. The approved test-only binding now includes
targeted external Popen failures and guarded before/after-current rename
barriers; observed readiness is mandatory before death/failure credit.
Successful actual runs remain entirely free of these overrides.

A real non-root, read-only-rootfs, network-disabled Docker probe mounts no host
paths and creates a fresh `/foreign` tmpfs at UID/GID65534/mode1777.
The process UID1000 creates/removes its own child, confirming foreign ownership
does not merely make the test unwritable. The mount disappears with container
exit. This proves fixture feasibility only; a separate actual-context class
must still reject installation without mutation. No host privilege/ownership
operation or production test mode is introduced.
Task3.1.2 independent authoring resumes; progress remains37/91.

### Installation/state authoring entry — 2026-11-19 17:08

After task3.1.1 verification and cleanup, the same independent author receives
task3.1.2 only, with all ten previous oracle files frozen. The approved
actual-fixture contract now identifies separately assembled initial/update
contexts and the parent's disposable real cp310 seed for Ubuntu24 migration.
No public option, simulated successful installation or private receipt API is
added. Existing accepted support/core bytes remain unchanged.
The author's new work is limited to `test_native_install_spec.py` and focused
new test-only `native_install_*` modules. Parent retains progress authority;
no sibling task or actual proof is advanced by dispatch.

### Installer manifest oracle authored and verified — 2026-11-19 17:02

Independent author `ses_e0bb271d4ffeTLIjBoBhoO7L5O` writes the ten-file
task3.1.1 oracle from the approved contracts without reading forbidden
implementation. Parent reads all files and confirms 24 local methods:
nine structural, fourteen bootstrap/transport and one eleven-scenario archive
method. All fail as expected for absent parser/producer, with 44 failure
records including subtests, zero collection errors or skips. The explicit
one-method positive group raises its required missing-context error. That is
unavailable actual proof, not a behavioral red result or acceptance pass.

Fresh `make check` stops only at the new native setup suite: all 29 historical
setup methods pass; 24 new methods fail with the same 44 expected records.
The earlier unchanged-implementation baseline is green with 4,361 Rust
instances; later compiler/distribution/fuzzer legs are not rerun past this
expected setup failure. All five historical setup files are byte-identical
to the Phase 3 baseline.

A parent viability probe exposes one fixture-only classification error masked
by the absent producer: `wheels.json` had role `wheel` and was consequently
treated as a ZIP. The independent author changes only that metadata record to
`preparation`, leaving `.whl` records, every assertion and the other nine files
unchanged. Parent reruns all red/task gates and verifies the resulting actual
miniature lock/core/four support audits and both wheel sets with the unchanged
Phase 1–2 validators. This is fixture viability, not real runtime evidence.

`build/distribution/reference-20261117/native-manifest-authoring-proof.json`
retains exact ten-file hashes, commands, baseline and per-method results.
Python3.10 syntax, documentation self-check, no planning-reference leaks and
whitespace pass. Task3.1.1 verifies at37/91. No production code, real installer
activation, phase closure, commit or publication occurs; the separate positive
method must still run on both actual supported cohorts after implementation.

### Positive fixture execution context clarified — 2026-11-19 16:27

The independent author confirms the three original AR-44 gaps are resolved
and stops once more before writing because positive ST-12 cannot run on this
Ubuntu26/Python3.14 host. The existing mandatory supported-cohort installer
proof is now bound to exact actual-context fields, accepted input sources,
real-image preparation requirements and a separate explicitly invoked,
fail-closed specification group. Local negatives do not replace that positive
method or count missing-context errors as passes.

Read-only probes reconfirm actual Ubuntu22/Python3.10.12 and
Ubuntu24/Python3.12.3 image identities and GCC/C tools. Both lack curl;
disposable acceptance environments must install only the already declared
prerequisites and record new immutable image IDs before positive proof.
No host packages or previous images/artifacts are changed.
Command line prefixes/quoting behavior are explicit for black-box guidance
checks without fixing private transaction state. This completes the approved
fixture/test-binding clarification; independent task3.1.1 authoring resumes
with the same forbidden-production list. No tests, installer code, actual
installation, commit or publication is yet claimed.

### AR-44 approved contracts and entry gate — 2026-11-19 16:17

The user selects “Approve amendment (Recommended)” for the three bounded
observable interfaces. The installation/release documents now define exact
nested manifest fields/asset relationships, the shared validator boundary,
coherent miniature metadata, the local `assemble` command/outputs and the
test-only curl/Python/HTTP response binding to unchanged shipped bytes.
The adapter never becomes a shipped option/environment/import mechanism.
Private state filenames, lock naming and function internals remain unfixed.

The unchanged implementation's `make check` passes: 4,361 Rust test instances;
Python and fuzzer summaries are retained in the Phase 3 entry receipt.
This is a baseline, not installer red/green or packaged-install acceptance.
The independent author must recheck readiness, then write tests against this
contract before any production change. Progress remains36/91 and no installer
specification or implementation is yet written. DEF-574's ruling is resolved;
the earlier stop below is historical.

### Phase 3 authoring-readiness stop — 2026-11-19 15:57

Independent implementation-blind author `ses_e0bb271d4ffeTLIjBoBhoO7L5O`
checks tasks 3.1.1–3.1.3 before authoring. All three require missing observable
interfaces: exact nested manifest data and a coherent valid payload fixture,
the local release producer's command/outputs, and the exact-byte test adapter's
binding to the shipped shell/helper and lookup/download responses.
The author writes nothing, reads no forbidden implementation and makes no
verification claim. Private lock/state/function names are not blockers.

AR-44 records the minimum bounded amendment and pending user ruling; DEF-574
blocks the specification/red tasks visibly. No tests or installer/bootstrap
code exist yet. Phase baseline `1ecded4acc2c3196b670ef6edf2c1856c677dd2a`
includes all accepted prior work. Resume checks confirm exact accepted Phase 2
source identities and successful full-gate/closure receipts.
Progress remains36/91, five minors report-only, no commit or publication.
An unchanged-implementation `make check` entry run is in progress; it cannot
substitute for unstarted installer specification red proof.

### Phase 2 accepted and closed — 2026-11-19 15:12

The user explicitly accepts the fully verified bounded RV-301 fix and Phase 2
closure. Parent rechecks exact frozen source/diff hashes, full-gate success and
cleaned owned scratch before task2.3.4 promotion. All recorded major findings
are resolved through the authorised fixes and final ruling; five minor
report-only findings stay unresolved. No third automated review, commit or
publication occurs. Progress36/91; all sixteen Phase 2 tasks are verified.
The current accepted candidate is the separately retained artifact-alias
candidate `eed3a175...738fb0`, not the earlier reviewed candidates.

### Bounded RV-301 full gate and final user handoff — 2026-11-19

Full `make verify` passes all SDK/emulator/differential/fault/sanitizer legs on
the authenticated private display after the bounded conjunction correction.
Both host and SDK-container authentication and unauthenticated rejection
pass. Split stdout/stderr logs preserve complete summaries: 4,695 passing Rust
instances including sanitizer reruns. Native110, implementation8, historical
driver22 and LSP35 pass; core26 and both actual14-method cohort/runtime proofs
remain separately bound to exact current source/binary/archive identities.
The packaged artifact-directory-alias rejection and safe real guest/rebuild
proofs were repeated on this new candidate, not inferred from earlier ones.

`phase2-artifact-alias/verified-fix-proof.json` retains exact runner/commands,
results, source/oracle identities and maintenance recipes. Adjacent
`verified-fix.diff` contains only the bounded production correction and
independent additive oracle against reviewed `32b5461d...`.
Frozen source head is `a88c7351d7b78049b122bb0140717c28a9c37085`.
All owned logs/scratch/media/auth/build/author processes are cleaned; only
named candidate artifacts and non-media evidence remain intentionally retained.

As explicitly approved, return the verified diff for final user disposition
without a third automated review. Task2.3.4/PLAN-45 are visibly blocked on
DEF-573; progress stays35/91 until that ruling. RV-301 and aggregate Phase 2
acceptance are not silently closed. No commit, publication or minor fixes.

### Bounded RV-301 actual cohort/runtime proof — 2026-11-19

Both actual isolated Ubuntu22/cp310 and Ubuntu24/cp312 pass twelve
archive/stage and two separate checkout-free installed/runtime/relocation
methods using the new identical current1.1.2 core archive
`eed3a17570ebd6fd312d808f28f724d9f3f4a10f1a2d1e42aa35a4e8f8738fb0`.
The actual packaged driver builds through a child artifact-directory symlink
outside the canonical artifact root, then rejects four unsafe direct/aliased
TMPDIR selections on both profiles before real FS-UAE exec.

Both safe real FS-UAE3.2.35 profile runs visibly show Hello (parent direct
private-root image inspection), distinct private CNEXT/A1200/KS3.1 drives,
exact private ROM copies and private initialization/cache/state. Actual
same-source/profile rebuilds through the artifact alias change original
program bytes without changing the live mounted copies. Traces bind emulator
and driver exit0/wait before owned storage cleanup. This host-system emulator
observation remains distinct from the actual Ubuntu cohorts.

An initial test-only temporary alias used a host absolute target inaccessible
inside the namespace; that unaccepted attempt is preserved separately.
Changing only the recipe alias to relative reaches the intended boundary;
no product/oracle changes. New proof is under
`phase2-artifact-alias/actual-acceptance-proof.json` and `native-runtime-proof/`.
Owned media/auth/runtime/build scratch is cleaned; full private gate scratch
remains active. Final user disposition follows successful full verification;
progress35/91, no third review or phase promotion.

### Bounded RV-301 regression and implementation — 2026-11-19

The user approves the bounded correction/verification and return for final
disposition, not a third review cycle. Independent author
`ses_e0c7c8bf6ffeCuC0T6FnfGsCrc` adds
`emulator_artifact_alias.rs` and a registration-only import. Two rejection
cases are red on the reviewed head; two safe alias-output controls pass.
Both rejection methods publicly build through the child artifact symlink,
prove canonical output lies outside the artifact root, rebuild changed bytes
and preserve outside canaries before the no-launch failure.

Parent reads/reruns the red cases and proves all previous106 expectations
unchanged. Only then the canonical current-program directory exclusion is
restored alongside—not instead of—the whole-root/comma guards. The new oracle
stays frozen. Native110, implementation8, historical driver22, LSP35 and
core26 pass. `artifact-alias-authoring-proof.json` and
`artifact-alias-implementation-proof.json` bind exact red/green/source identities.
No core producer/oracle, SDK, flag, runtime or other source change.

Actual ordinary-user offline Ubuntu22 rebuild and full authenticated private
`make verify` start against frozen code. New actual candidates/proofs go under
`phase2-artifact-alias/`; prior ones remain unchanged. Both actual cohorts and
packaged runtime alias/rebuild controls will reverify before final user
disposition. Progress stays35/91, no third automated review or phase closure.

### Single fix review complete; new regression blocks closure — 2026-11-19

Packaging-safety, concurrency and Amiga return clear assessments, but the
correctness reviewer reproduces RV-301 on both native profiles: symlinking the
program's artifact directory outside the canonical artifact root bypasses
the replacement root-only check. Using that target as TMPDIR launches an
observer and rebuilding deletes its live drive; the prior reviewed driver
rejects. Existing native106/implementation8/core26/full gates miss this distinct
alias direction (the new regressions alias TMPDIR instead).

The findings ledger reconciles specialist SR-202 closure to partial; SR-201
and RV-C201 remain addressed. Task2.3.4 and roadmap block on DEF-572 pending
the user's ruling on retaining both canonical exclusions with new independent
red regressions and repeated actual/full verification. No automatic third
correction/review cycle starts. Source/tests stay frozen, progress35/91.
All four reviewers report read-only cleanup. Earlier proofs are preserved
without treating their passing cases as coverage of this new failure.

`phase2-correction-review-proof.json` retains frozen diff/source identities,
the complete fix packet, role results, current finding dispositions and owned
maintainer recipes. Parent confirms reviewed source/tests unchanged, no
reviewer child scratch remains, and removes only its nine exact files and
empty correction root. All review/gate/runtime/media/auth/build scratch is
cleaned. Named candidate artifacts and non-media evidence remain intentionally
retained for the ruling; no code change, commit or publication occurs.

### Corrected full gate verified; single fix review next — 2026-11-19

Full `make verify` passes again on authenticated private Xvfb, including every
SDK/emulator/differential/fault and sanitizer leg. Correction source/oracle
identities still match the frozen head `32b5461d1eb139a92dee8bc5c9e58ca99603726e`.
`phase2-correction-full-verify-proof.json` retains the exact runner and results.
Its 4,658 instance count is only the complete-line parsed subtotal: merged
stdout/stderr splits one 33-test suite's result before the next suite header.
`phase2-correction-summary-integrity-proof.json` explicitly records that
extraction limit and an independent 33/33 green rerun. Do not treat the subtotal
as an authoritative total or infer a skipped suite. The full gate exited0;
no product/oracle change was needed.

Private gate server/authority/log scratch is cleaned. One fix-diff re-review
now follows on original reviewed head `e7843384...` to corrected `32b5461d...`,
including the explicitly approved AR-43 oracle amendment and its red controls.
No major finding is closed and progress remains 35/91 until review results.

### Corrected actual cohorts and native runtime reverified — 2026-11-19

The corrected producer packages the corrected Ubuntu22-built current 1.1.2
executables. Both isolated actual Ubuntu22/cp310 and Ubuntu24/cp312 cohorts
pass twelve artifact/stage and two separate checkout-free installed/runtime/
relocation methods. Their identical archive SHA-256 is
`364376dabebaf582cfd0b80eb9644e6d1354e882f8118c6a22a5d74cf580eb2a`.
This is a new candidate, not a relabelled old artifact.

Using that actual candidate, traced real FS-UAE3.2.35 runs reject six unsafe
TMPDIR choices (comma, another source and another profile, on both profiles)
with status1/TMPDIR remedy and no FS-UAE exec. Outside canaries are unchanged.
Two simultaneous safe real runs retain distinct private roots and exact local
trailing-space ROM copies, effective A1200/KS3.1/CNEXT priority0 mounts and
private initialization/cache/state. Parent directly reads both raised private
display-root screenshots: Hello is visible. Traces bind FS-UAE/driver exit0
and wait before storage cleanup. The host-system emulator observation remains
separate from the actual Ubuntu runtime cohorts.

`phase2-corrections/actual-acceptance-proof.json` links new cohort receipts and
`native-runtime-proof/` screenshots/traces/recipe. Licensed-media copies,
authority, owned processes, runtime trees and copied build tools are cleaned.
Full-gate scratch remains active. No major finding or task2.3.4 is promoted;
the full private gate and one fix-diff review remain pending at 35/91.

### Major corrections targeted green; acceptance rerun underway — 2026-11-19

Parent independently reproduces eight expected native rejection failures with
two justified safe-parent passes. The approved AR-43 assertion/helper amendment
is independently authored without production reads; parent then reproduces
two local core failures (private identity and actual 1.1.2/9.9.9 mutation race)
with 24 other methods green. The helper preserves executed `/proc/self/exe`
bytes separately from unchanged literal argv records. Only the approved
identity method and necessary C observation helper change; all unrelated
expectations remain unchanged. `phase2-correction-authoring-proof.json`
freezes exact new/approved oracle identities and parent red results.

After those red checks, storage rejects comma-containing canonical parents
and any parent within the current workspace's canonical artifact root. The
producer copies inputs privately, preserving executable eligibility, then
inventories, queries versions and archives the same copies. No new framework,
dependency or product operation is added. Native106, implementation8 and
historical driver22 pass; all 26 core local methods pass. Source/documentation/
syntax/size/diff checks and frozen oracle hashes pass.
`phase2-correction-implementation-proof.json` retains these scoped results.

Corrected current-1.1.2 binaries build offline as an ordinary user in the
pinned Ubuntu22 image; all three answer version queries with GLIBC requirements
no higher than 2.35. New artifacts live separately under `phase2-corrections/`;
earlier candidate artifacts/evidence are preserved without relabelling.
Both actual Ubuntu/Python cohorts and the full authenticated-private-display
`make verify` are running. Packaged native runtime reproof and one fix-diff
re-review still gate closure. The three major findings are not yet resolved;
SA-201/SA-202 remain report-only. Progress stays 35/91.

### Approved major-correction regression authoring — 2026-11-19

Independent author `ses_e0c7c8bf6ffeCuC0T6FnfGsCrc` adds ten native storage
methods: eight expected red rejections cover both profiles, comma parents,
canonical aliases and other-source/profile artifact directories. Two safe
temporary-parent controls pass before the fix. A separate core snapshot suite
has one expected red race and two passing stable-input/barrier controls.
The race reaches both real executable-query barriers, changes actual ELF bytes,
and twice reproduces success with metadata 1.1.2 but extracted tool 9.9.9.
No production reads or changes; no real emulator/media. Twenty-nine original
test/helper identities are unchanged after removing registration-only imports.
Author's original-oracle manifest hash:
`1a09ca18642d1c53f3736659cc071587e06c0afc02039f7ec82b623bcf3eb7e7`.
All author-owned scratch and children are cleaned.

Parent identifies the old original-executable-path assertion conflicts with
the approved snapshot guarantee. AR-43 explicitly requests a narrow independent
amendment to that assertion and necessary exact-byte observation helper. The
user selects “Approve amendment (Recommended)”; it must run red before any
production correction. All other expectations remain immutable. Progress is
35/91; no major finding is yet resolved.

### Phase 2 reviews complete; correction ruling pending — 2026-11-19

All four reviews finish against the frozen implementation. Correctness finds
no new issue and independently reruns `make check` twice plus focused probes.
Packaging safety records two minor report-only findings. Amiga reports two
major storage cases; concurrency adds RV-C201, a deterministic public-command
race where queried executable bytes differ from archived bytes and the
extracted tool's version contradicts core.json. Stable accepted artifacts
still match their identities; the race does not establish their corruption.
The consolidated findings ledger proposes three bounded corrections with
independent red regressions, actual artifact/runtime reproof, full private
gate and one fix-diff re-review. No code or oracle is changed before user
ruling. Task 2.3.4/DEF-570 remains blocked, progress 35/91.

Parent confirms all 31 implementation/test files still match the frozen head.
`phase2-review-proof.json` retains packet/tree/diff identities, reviewer
sessions, source hashes and consolidated findings. All reviewers report
read-only cleanup; parent confirms no reviewer child scratch remains and
removes only its three owned packet files and exact empty review root.
Review scratch cleanup is complete; named candidate artifacts and non-media
evidence remain intentionally retained. No commit or publication occurs.

### Phase 2 independent review stop — 2026-11-19

The ordinary local packaging-safety review reports no critical/major findings
and two minor, report-only items (SA-201/SA-202). The Amiga specialist reports
two major uncovered emulator cases (SR-201/SR-202): a canonical TMPDIR comma
can redirect FS-UAE's comma-delimited mount, and another program's resettable
artifact directory can contain storage deleted by that program's rebuild.
The findings ledger records reproductions, evidence limits and proposed
minimum remedies. Passing native96/implementation8/core23/full-gate evidence
does not cover these cases. No frozen oracle, product code or candidate bytes
are changed. Task 2.3.4 and roadmap PLAN-45 are blocked on user ruling
(DEF-570), at unchanged 35/91. Correctness/concurrency reviews continue;
consolidation and a direct user decision precede any remediation.

### Phase 2 full gate verified; reviews pending — 2026-11-19

The authoritative `make verify` passes every leg on the authenticated private
display: workspace/native/implementation/LSP tests, benchmark shape, twelve
UBSan suites, the complete SDK/emulator matrix, differential corpus and runtime
faults. The log records 4,681 passing Rust instances including UBSan reruns.
Historical driver 22, native 96, implementation eight and LSP 35 are green.
No production or frozen oracle identity changes during verification.
`phase2-full-verify-proof.json` retains the exact runner, gate summaries and
full-log identity. Private host/container authentication and rejection of
unauthenticated access are confirmed. The server is stopped, authority removed
and all owned gate scratch/logs cleaned. Task 2.3.3 verifies at
2026-11-19 11:09, progress 35/91. Task 2.3.4 stays pending the independent
correctness, ordinary local packaging-safety, concurrency and Amiga reviews.
No phase completion, commit or publication is inferred.

### Phase 2 full gate in progress — 2026-11-19

The authoritative `make verify` starts on an owned authenticated private Xvfb.
Host readiness, rejected unauthenticated access and actual SDK-container
authentication are required before the gate runs. Cookie material uses stdin,
not argv or logs; no desktop display, `.env` or user X access policy is changed.
Gate output is bounded/captured under owned scratch, and the server/authority
are stopped/removed on exit. Caller-audit task 2.3.3 and full-gate/review task
2.3.4 remain pending until observed results; independent phase reviews have
not yet run. Progress remains 34/91.

### Native caller/parallel-path audit — 2026-11-19

| Surface | Audited paths and evidence |
|---|---|
| Entry/context | `invoke` resolves SDK once before any build; native emulator preflight also runs before compilation. Host/emit bypass unused SDK environment; legacy explicit-emulator flags and native capture reject before build. |
| Builds | All three `build::build` orchestration calls receive the selected compiler or no override. `build::compile` changes only executable selection; existing profile/mode flag and library vectors remain authoritative. |
| Host/freestanding | Host keeps PATH cc and native execution. Freestanding cross-links with selected native compiler, then its separate host compiler/run call explicitly receives no native override/Python. |
| Amiga/vamos | Both Amiga profiles use the canonical same-installation compiler and optional paired venv-spelled Python, fixed isolated entry text and absolute program argument. Neither build nor native FS-UAE requires unused companion files. |
| Emulator/legacy | Native branch consumes preflight inputs and private copied artifacts; legacy branch retains exact host-handoff/capture text. Native storage/drop/wait paths retain live or unconfirmed-child files and clean only owned stopped storage. |
| Result/diagnostics | Existing compile/source status, human/NDJSON machinery and output capture flags remain in all parallel branches. Native emulator propagates process codes and reports launch/wait/code-less termination as emulator failures, never guest status. |
| LSP scope | Only exact single-argument `--version` bypasses the unchanged stdio library. Seven non-exact argv forms and historical lifecycle cases retain protocol behavior. |

Search finds three build orchestration calls, the freestanding host compile
call, two run-program calls, one SDK resolution and one emulator preflight.
Every caller is covered by native 96, implementation eight, historical driver
22 and LSP 35 cases plus actual cohort/guest evidence. No IR, ownership,
language rule, emitted shim, ABI flag, source pin or frozen spec change is
introduced. The upcoming full `make verify` reruns these historical/native
oracles; this audit task stays pending until that gate confirms them.

### Native activation/cleanup race hardening — 2026-11-19

Two implementation controls add both-profile environment-selected activation
during an external compiler pause and deterministic allocator collisions with
existing directories/aliases. A compiler wrapper publishes only its parent PID
before delegating unchanged; no production pause hook is added. Existing stale
sentinels remain unchanged during launch and after the new run's cleanup.
All barriers release and workers join before fixture deletion.
The first run catches a new test's use of the hosted-amiga-only helper for
the nix13 iteration. Selecting the existing profile-aware helper corrects only
that caller; no frozen helper/assertion changes. All eight implementation
methods and exact-hash `make check` then pass.
`native-race-hardening-proof.json` retains results/identities; source/doc/diff
checks and frozen oracle identities pass; all owned scratch is cleaned.
Task 2.3.2 verifies at 2026-11-19 10:45; progress 34/91.

### Native path/error implementation hardening — 2026-11-19

Six implementation-only cases cover decoded-key duplicates and integer-token
boundaries, valid escaped keys, relative SDK/tool/media paths, rejection of
TMPDIR beneath resettable build output, and private initial XDG/HOME paths
while preserving explicit-relative or default Xauthority meaning.
They reuse unchanged frozen helpers and do not replace the independent oracle.
The targeted six methods pass. A final diagnostic-presence assertion postdates
the first compiled test binary; timestamp inspection catches that verification
gap, so the full task `make check` is rerun against the exact current source.
It passes, including native 96/96 and all six implementation cases.
Source/documentation/size/whitespace checks and frozen oracle hashes pass.
`native-path-hardening-proof.json` retains results and identities; all owned
scratch is cleaned. Task 2.3.1 verifies at 2026-11-19 10:37; progress 33/91.

### Actual native FS-UAE observation — 2026-11-19

The actual packaged Ubuntu-22-built candidate boots the known hello program
on user-installed FS-UAE `3.2.35-2` (runtime reports 3.2.35, GLib 2.88.0).
Both `hosted-amiga` and `hosted-amiga-13` screenshots are directly inspected:
`Hello, Clorraine!` is visibly present in the guest AmigaDOS screen.
This is the prescribed A1200/KS3.1 observation, not an A500 compatibility claim.
The host system libraries/emulator run in bubblewrap filesystem/PID/network
namespaces with no checkout, builder prefix, original SDK or Docker socket.
This host-emulator observation is separate from the two genuine Ubuntu/Python
runtime cohorts, not a relabelling of the host as either approved Ubuntu image.

An authenticated private Xvfb is used; an unauthenticated connection is
rejected. Cookie bytes travel through stdin only, never argv/logs.
No desktop display or user settings are changed. Tracing confirms the actual
emulator's private display/authority environment, its initial XDG-data log
before configuration, later private logs, cache writes and effective save-state
path. Real emulator logs/debug configuration show A1200/68020/AGA, the supplied
KS3.1 identity and exactly one successful filesystem mapping:
CNEXT/DH0, boot priority zero, the dedicated run drive. Only fixed program and
startup files are initially mounted.

Two live real instances have distinct roots, media copies and writable scopes.
Both use the trailing-space local ROM filename and receive exact private
ordinary unmounted copies. A real rebuild changes source-artifact bytes while
both mounted/config/media snapshots remain unchanged. Closing one normally
removes only its own storage while the other remains live. Real process traces
prove FS-UAE exit 0, its parent's successful wait and driver exit 0; none is
called guest status. An additional ordinary punctuated-ROM pilot boots with
its original literal path. Caller config/canary bytes remain unchanged.

The direct negative control omits runner isolation. The same observation
detects inherited A500 configuration, outside drive zero at priority 127 and
an extra outside drive, proving it can detect the prohibited settings.
Separate actual SIGTERM and SIGKILL controls target only the owned driver:
its surviving emulator retains unchanged mounted/media/config files.
Replacement invocations choose different roots and neither adopt nor erase
the stale run. The gate removes its own stale directory only after all its
children stop.

Preliminary unaccepted observation attempts expose test-control issues:
keyboard-grab quit targeting, a transient startup window title and black
direct OpenGL-window capture. Final observation uses stable visible-window
selection, WM_DELETE_WINDOW and the raised private display root. No product
or frozen assertion changes. One early quit of deliberately conflicting
negative hardware returns external FS-UAE status 139; it is not accepted as
a valid boot. Standard host crash directories show no same-user FS-UAE
artifacts. The final negative control is stopped after effective-state
measurement instead of entering its invalid-hardware quit path.

`native-fsuae-proof/` retains private screenshots, logs/traces, exact commands,
source recipes and identities, including clearly separated failed controls.
It contains no ROM/key/cache/save-state bytes. All owned emulator/Xvfb
processes are stopped; gate media/auth files, stale storage, build tools and
gate scratch trees are removed. Only renewed task-check scratch remains active.
The full phase gate and independent reviews are still required.

Renewed `make check` passes after these observations. All native/core source
and frozen oracle identities match their passed task receipts.
`phase2-native-acceptance-proof.json` links the exact cohort/emulator evidence
and task gate. All owned scratch, including the final check root, is removed.
Only named binaries/archives/staged candidates and non-media evidence remain
for phase review. Task 2.2.8 verifies at 2026-11-19 10:25; progress 32/91.
Phase hardening, full `make verify` and independent reviews remain incomplete.

### Actual core archive/stage/runtime task gate — 2026-11-19

The approved LSP entry change passes all three new process specifications and
the unchanged historical LSP suite 32/32. Local core archive/argument tests
pass 23/23; `make check` passes. The actual current three release binaries are
rebuilt offline as an ordinary user on the pinned Ubuntu 22.04 image. Each
reports its own `1.1.2` version and requires no GLIBC version above 2.35.
No version bump, commit, tag, workflow dispatch or publication occurs.

Both ordinary-user, network-isolated Ubuntu 22.04/CPython 3.10 and
Ubuntu 24.04/CPython 3.12 contexts pass twelve actual archive/staging methods,
followed by two installed-runtime methods in separate containers with no
checkout, builder prefixes, original SDK or alternate PATH cross tools.
The same core archive has SHA-256
`ada0742cb72a98cab0b4fde50a15f2ecc56030b26c4b3983cf5569494ca9b98d`
on both cohorts. Real NDK preparation, offline venv installation, matching
receipts, actual compiler/vamos output/status/capture and stable-link relocation
all pass, as do every real negative-input/no-clobber control. Inputs and user
activation remain unchanged. This is neither a real FS-UAE boot nor the phase
acceptance gate.

`core-package-implementation-proof.json` retains exact commands, cohort
identities/results, local gate and source identities. `phase2-core/` retains
the actual binaries, deterministic archives and staged candidates needed by
the next native-emulator and phase-review gates. Completed-task verification
logs are removed; owned builder/gate scratch remains active for those gates.
Task 2.2.7 verifies at 2026-11-19 10:03; progress 31/91.

### Actual baseline-core version readiness stop — 2026-11-19

Current three release binaries build offline, as an ordinary user, on the
pinned actual Ubuntu 22.04/GLIBC 2.35 image using the existing SDK Rust
toolchain. No compiler version is changed. The first direct real version
validation passes for `clc`/`clorraine` but `clorraine-lsp --version` returns 1
with empty output: its thin entry unconditionally starts stdio LSP.
The local 23 archive/argument methods remain green but do not establish
actual binary interfaces. `phase2-core/missing-lsp-version-proof.json` retains
the failed real readiness result and binary identities; initial build logs
and owned tools remain scratch pending the scope ruling.

AR-42 records the bounded entry-point fix versus explicit protocol-probe
contract revision. No LSP implementation or oracle is changed. Tasks
2.2.7/2.2.8 are visibly blocked and the existing producer diff is preserved.
No actual archive, support staging, Python cohort, packaged guest boot or
release acceptance is claimed. Six native tasks remain legitimately verified
against their already passed task gate; progress stays 30/91.

The user subsequently selects “Add version flag (Recommended)” for AR-42,
explicitly approving `compiler/src/lsp_main.rs` and the new
`native_lsp_version_spec_test.rs` file. The owning core contract records the
exact single-argument fast path and unchanged behavior for other argv vectors.
The independent specification author receives the approved interface and a
forbidden LSP/driver implementation-file list. Its additive red tests precede
any entry-point code. Existing native, core and LSP protocol oracles stay
unchanged. The producer task resumes pending real green evidence, without
changing the progress count or relabelling the failed actual baseline.

The additive process oracle has three methods. The independent author and
parent both observe two expected version-query failures and one justified
compatibility pass covering seven other argv vectors. The author also passes
the existing LSP 32/32 and two wait-guard probes, confirms nineteen prior
oracle/helper files unchanged and cleans its scratch. Parent freezes the new
oracle in `native-lsp-version-authoring-proof.json` before adding the exact
single-argument version path to the approved thin entry. No LSP library or
protocol file changes. The old failed Ubuntu core binaries/build evidence are
retained under `phase2-core/pre-version-binaries` and
`pre-version-baseline-build-proof.json`, not relabelled as the fixed core.

### Native emulator implementation task gate — 2026-11-19

After the additive oracle's recorded red run, native staging/process code passes
all 96 native methods. `make check` passes, including the unchanged historical
22-method driver suite and the complete existing local gate. No frozen
expectation changes. Source/documentation/size/whitespace checks pass.
`native-emulator-implementation-proof.json` retains commands, method outcomes,
all current native source hashes and frozen oracle identities. Native-owned
verification logs are removed; ongoing core-package scratch is separate.
Tasks 2.2.1–2.2.6 verify together against their now-green complete native/task
gate, bringing progress to 30/91. These helper observations establish dispatch,
staging and process ownership only. Actual packaged tools, both supported
cohorts, real FS-UAE effective-state/guest output and the phase gate remain
mandatory and incomplete.

The local core producer is implemented but remains pending. Its first local
run catches a diagnostic that names a raw descriptor instead of the invalid
directory input; checking descriptor type before wrapping it in a stream
corrects this without touching tests. Local archive/argument methods then pass
23/23. Real support/NDK/venv/staged-runtime proof is not inferred.

### Additive private-ROM oracle verified — 2026-11-19

Parent reads the new `emulator_media.rs` module and confirms that all ten old
support files match their prior hashes. Removing only its two-line import
restores the integration entry exactly. The fresh native run confirms
96 methods: 79 green and 17 expected emulator failures, including both new
copy/lifetime methods. No real emulator or licensed media is used.
`native-emulator-media-authoring-proof.json` freezes the additive identities
and result without relabelling earlier receipts. The author reports two passing
observation probes and four rejected bad controls. All owned scratch, including
the parent media root, is removed. Emulator implementation may now proceed;
the four prior implementation tasks remain pending their complete green gate.

### Native SDK/compiler/vamos partial implementation — 2026-11-19

Tasks 2.2.2–2.2.4 add canonical SDK selection, bounded strict native receipt
validation and coherent compiler/paired-Python paths through all build/run
branches. The native receipt module reuses the public JSON model and adds
duplicate-key/integer-token checks without changing the language server's
parser. Native Python uses the fixed isolated entry-point text and an absolute
program argument; legacy flags, libraries and runner forms are unchanged.
One intermediate compile reports a Cow/string ownership mismatch in two new
diagnostic constructors; converting the sanitized values to owned strings
corrects it. Compile/source/doc checks pass and those logs are cleaned.

The integrated scoped run passes all 43 driver units, all 59 resolution/
compatibility/diagnostic specifications, twenty emulator option-validation
cases and the historical driver 22/22. Full native and `make check` remain
red at 79 green/15 emulator-only failures. No unexpected suite or fixture
failure appears. All sixteen oracles stay byte-identical. Source/doc/diff
checks pass; `native-paths-implementation-proof.json` retains commands,
outcomes and source identities, and all owned scratch is removed.
Tasks 2.2.1–2.2.4 remain `[~]` because the required complete green task gate
has not passed. No real core/venv/guest acceptance is claimed.

Before native emulator staging, upstream source investigation identifies
AR-41's trailing-space ROM filename boundary. Ordinary punctuation can use
literal early CLI path settings instead of lossy INI quoting. FS-UAE still
strips terminal spaces from those values. The installed package/source/GLib
primitive evidence is retained without a launch or media read. A bounded
private-copy exception versus an explicit input limitation awaits user ruling.
No native emulator module or speculative trailing-space oracle is written.
The private configuration, early path controls and actual version-specific
isolation/boot proof remain required under either choice.

The user selects “Private ROM copy (Recommended)” at 2026-11-19 08:58.
The owning native contract now permits exactly that filename exception:
selected ROM bytes copied to a fixed private media name outside the guest
drive, retained for the child's lifetime and never shared/packaged/uploaded.
Original ROM and existing ordinary-path expectations remain unchanged.
Independent additive media/cleanup tests run red before any emulator code;
the prior 94 expectations remain frozen. No actual launch or installation
acceptance is inferred from the ruling.

### Native CLI partial implementation — 2026-11-19

Task 2.2.1 adds only three path fields/flags, literal nonempty argument parsing,
command applicability and native/legacy help to `cli.rs`. Existing CLI unit
tests pass 10/10; the sixteen owned option specification cases pass; unchanged
historical driver tests pass 22/22. Full native and `make check` both remain
red at 24 green/70 unimplemented-resolution/dispatch cases, with no unexpected
failing suite or fixture launch race. The additional source-error and
forced-zero assertion controls become green when their arguments are accepted;
other tests still require actual selected native subprocess paths.

The required global task gate has **not** passed. Task 2.2.1 remains `[~]`,
not counted as verified, while its already planned SDK/dispatch dependencies
are implemented to reach the green gate. No ignored test, weakened assertion
or new verification exemption is introduced. All sixteen oracle hashes stay
unchanged. Source/documentation/whitespace checks pass;
`native-cli-implementation-proof.json` retains exact partial outcomes and source
identity. All owned verification scratch is removed. No native installation
or emulator acceptance is claimed.

### Phase 2 consolidated pre-implementation red gate — 2026-11-19

Task 2.1.4 consolidates the native 94-test oracle, local core 23-test slice,
the fourteen mandatory actual core methods and unchanged historical driver
oracle. Exact outcomes and source identities are retained in the authoring
receipts above/below; a fresh bounded combined run must confirm the same
expected-red behavior, missing-context errors and historical green results.
The existing `make check` remains intentionally red only for the new native
feature. No implementation or actual package/emulator acceptance is inferred
from an authoring gate. All Phase 2 implementation tasks remain unstarted
until this consolidated gate verifies.

The fresh combined run verifies at 2026-11-19 05:30: native 94 methods are
88 expected behavioral red/six justified compatibility green; historical
driver 22/22 green; local core 23 methods produce 52 absent-producer records;
whole-file core discovery additionally raises two explicit missing-context
setup errors and leaves fourteen actual methods unrun. No ignored/skipped
actual acceptance is claimed. `make check` fails only the expected new native
suite, without a fixture-launch failure. All sixteen oracle files match their
reviewed hashes, documentation/diff checks pass and all owned scratch is
removed. `phase2-spec-red-proof.json` retains exact commands, method outcomes
and identities. Task 2.1.4 verifies as the pre-implementation red gate;
progress 24/91. Native implementation may now start. No native installation,
guest boot or phase/release acceptance is inferred.

### Phase 2 entry and installed-layout readiness — 2026-11-19

Recorded fresh phase baseline tree
`cadcbce6fc84c9c88701ff16b0080018908edefb` before changes. It includes all
pre-existing uncommitted Phase 1 implementation and closure records; no commit
or publication occurred. The ten required documents, resolved prior register,
passed preflight and retained four-review receipt were checked. The three
Phase 1 minor findings remain deferred/report-only.

Independent specification author `ses_e0e9ae0d7ffejNgOFVgi48Ct4I` receives
the approved native contracts/cases, existing test conventions and a forbidden
implementation-file list. The unavailable named tool role uses a generic agent
with the full specification-author brief, implementation blindness, strict
scope and cleanup rules. It identifies missing installed-layout metadata and
paired-vamos command interfaces before authoring. It reads no forbidden
implementation, writes no tests or other files, runs no verification and creates
no scratch. Missing public core-producer arguments are also identified by the
parent from the owning local-producer section.

AR-40 records the precise gap, existing reusable marker/Python evidence,
bounded amendment options and the proposed document modification set.
DEF-567 tracks the pending user decision. Directly affected tasks show `[!]`;
no specification-red or product implementation claim is made. Progress remains
20/91. The fresh unchanged-product `make check` is launched with captured
output; its result is recorded separately, not inferred from the earlier gate.

The fresh `make check` passes: 4,240 Rust tests, Python suites 29 + 113 + 103 +
28, and fuzzer smoke `cases=28 host=28 sanitizer=16 target=16 failures=0`.
The user selects “Approve amendment (Recommended)” for AR-40. The owning
installation/native-driver/local-producer documents now define exact metadata
paths/schema, pairing checks, missing-tool versus invalid-SDK outcomes, the
venv-preserving isolated-Python argv and a complete miniature valid fixture.
They also define local core `archive`/`stage` signatures and actual audited
support inputs. These reuse existing markers/receipts; native readiness is not
installer transaction completion. No Phase 1 artifact or implementation changes.
The author is resumed against the approved concrete contracts, with the same
forbidden implementation list and specification-first ordering.

Authoring must include descriptor/marker/wheel mismatch and missing-tool
controls, build-without-runner, actual helper paths/argv and activation
coherence. Local producer tests use the public commands and check byte
preservation, current versions/notices, audit/NDK/ABI rejection and no-clobber
cleanup. Real archives, both Python cohorts and real emulator acceptance remain
mandatory later; synthetic process helpers do not pass those cases.

### Native resolution/compatibility oracle authoring — 2026-11-19

The resumed independent author delivers task 2.1.1 as the
`native_driver_spec_test.rs` integration entry plus five focused
`native_driver_support/` test-only modules: fixtures, resolution, validation,
compatibility and diagnostics. The author confirms no forbidden implementation
read and no production/historical-oracle/plan changes. Its own scratch is
removed.

| Contract group | Author's observed result |
|---|---|
| CLI/environment/adjacent precedence, literal selected paths and paired Python argv | Native cases red; legacy PATH lookup green |
| Invalid selected candidates, exact descriptor/NDK/wheel schemas and pairing, missing-tool distinction | Red at missing native dispatch/options; no fixture/syntax failure |
| Stable-link relocation and activation-coherent paths | Red |
| Host run/capture, all-profile emit and help/version with irrelevant settings | Four green controls, justified because unchanged behavior already exists |
| Both legacy profile handoffs/capture with irrelevant emulator environment | One green control, exact historical behavior retained |
| Freestanding native cross-build/host-run leg, both Amiga profiles/modes and both synthetic ABI status/capture vectors | Red; helper dispatch is not real wheel-cohort acceptance |
| Missing tools, compiler/source errors and human/NDJSON diagnostics | Red |

Total: 59 tests, 53 expected behavioral red and six compatibility green.
The unchanged historical driver suite passes 22/22. Two independent helper
argv/status/barrier probes pass; the fixed Python entry compiles. Files stay
under 700 lines and pass whitespace/planning-reference checks. The parent
reads every delivered test/helper and launches captured historical-suite and
`make check` authoring verification before promoting the task. This is a
specification-authoring result, not a green native-product gate.

Parent verification confirms the same 59-test result through `make check`:
53 expected failures solely in the new native suite and six justified green
compatibility controls. The unchanged historical suite separately passes
22/22. All delivered tests/helpers are read for oracle grounding and
documentation; file-size, whitespace and planning-reference checks pass.
`native-driver-authoring-proof.json` retains exact test outcomes, commands,
baseline and six oracle hashes. Parent verification logs/scratch are removed.
Task 2.1.1 verifies at 2026-11-19 02:55; progress 21/91. No product-green or
actual installed/Amiga acceptance is inferred. Emulator and core-package
specifications/red proof still precede all Phase 2 implementation.

### Native emulator oracle authoring — 2026-11-19

The same implementation-blind author adds 35 tests without changing the
existing 59 expectations: 20 option/applicability/pre-build cases, six
executable/ROM precedence and missing-input cases, four declarative staging/
literal-path cases, four wait/status/signal/launch-failure cases and one
concurrent-run/rebuild ownership case. The local observer is an external
process double; it does not simulate FS-UAE's configuration loader or boot a
guest. All 35 cases are behavioral red. Total native suite: 94 tests,
88 expected red and six justified compatibility green, with no ignored or
filtered tests. Historical driver: 22/22 green, unchanged.

Six independent instrumentation probes pass: argument/config observations,
barriers/signals/early-parent-exit detection, rejection of unsafe records/
malformed NDJSON, actual changed rebuild bytes and executable corruption
after successful compilation. The author verifies syntax/documentation,
whitespace, sizes and planning-reference hygiene, reads no forbidden source,
changes no production/plan/historical-oracle files and cleans its own scratch.
Parent review reads every new helper/test before captured task verification.

#### Required actual native FS-UAE acceptance

Missing prerequisites, unsupported isolation or unobservable required evidence
fail this acceptance; helper results cannot pass ST-37.

| Step | Required action and retained evidence |
|---|---|
| Candidate | Use actual local core `archive`/`stage` outputs and proved support inputs. Record core/binary hashes, support lock, OS, Python cohort and FS-UAE version. Run as ordinary user. Phase 2 does not require a VSIX or final installer transaction receipt. |
| Isolation | Make checkout, builder trees and original SDK unavailable. Establish an owned authenticated private Xvfb pair, verify the actual FS-UAE display/authority environment and use no desktop display. |
| Hostile inputs | In owned test HOME/workspace create conflicting CWD `Config.fs-uae`, user `Default.fs-uae`/`Host.fs-uae` and base-directory settings. Name an owned outside writable drive with canaries and conflicting boot/model settings. Retain complete before-run inventories. |
| Guest boot | Run the actual staged driver for both `hosted-amiga` and `hosted-amiga-13`, with explicit native SDK, FS-UAE and local KS3.1. Observe `Hello, Clorraine!` on the guest display before closing the emulator; retain private image/text evidence. Do not use native `--capture`. |
| Effective settings | Observe initialization through real emulator logs and process/file tracing as necessary. Prove A1200, the supplied ROM identity, CNEXT/priority zero and only the staged drive, fixed guest name and executable/startup inventory. Prove effective private base/cache/save-state/log paths outside that drive. Generated INI and a clean HOME alone are insufficient. |
| Lifetime | Hold two real instances, rebuild the first source while mounted and compare mounted/config bytes and writable-state locations. Close one without changing the other. Check cancellation never erases a surviving child's files; later runs neither adopt nor erase stale live storage. |
| Negative observation control | In separate owned test storage remove isolation controls from a copied config/direct emulator command. The observer must detect inherited hostile mounts/settings. An ineffective control makes the proof insufficient. Never change candidate bytes or frozen tests to obtain this result. |
| Result and cleanup | Record actual emulator and driver process results, never guest exit status. Prove cleanup after child termination and unchanged caller configs/canaries. Bound verification time, stop all owned processes before cleanup and keep media/private paths out of published evidence. |

For each profile, invoke:

```sh
"$I/bin/clorraine" run --profile "$PROFILE" --sdk-root "$I/sdk/amiga" \
  --emulator --fs-uae "$FS_UAE" --kickstart "$KS31" "$HELLO"
```

This A1200/KS3.1 observation does not replace the existing A500/KS1.3 gates.
No actual native emulator acceptance has run during test authoring.

Parent verification at 2026-11-19 03:39 confirms all 94 cases through
`make check`: 88 expected behavioral red and six justified compatibility green,
with no other failing suite. Historical driver separately passes 22/22.
The five pre-existing resolution/validation/compatibility/diagnostic/fixture
oracle modules remain byte-identical to task 2.1.1. New files and the additive
integration entry pass parent documentation, source-size, Python syntax,
whitespace, plan-link and planning-reference checks. The named
`native-emulator-authoring-proof.json` retains all test outcomes, commands,
baseline and eleven file hashes; parent logs/scratch are cleaned. Task 2.1.2
verifies as authoring, not emulator acceptance. Progress: 22/91.

### Local core oracle authoring and fixture-launch investigation — 2026-11-19

The independent author delivers five focused Python test/helper files with
35 methods. Parent coverage review requests two additive actual-stage
controls already required by the approved contract: missing/non-executable
selected Python and a different declared wheel ABI with a correctly updated
metadata audit hash. All 35 existing method bodies remain unchanged.
The complete suite has 37 methods: 23 local archive/argument cases and
14 explicit real-artifact/runtime acceptance methods.

Parent review reads all five files and independently runs local and whole-file
groups. The 23 local methods produce 52 expected subtest failure records,
each naming absent `package_core.py`. Whole-file discovery additionally gives
two explicit setup errors for missing `CLORRAINE_SUPPORT_ACCEPTANCE`;
the 14 actual methods do not run or become skips. Python 3.10 syntax,
documentation, size, whitespace and planning-reference checks pass.
Four initial ELF/oracle probes and two ABI-edit construction probes pass in
the author's independent run; these do not establish actual core/tool proof.
The parent initially double-counts a missing-command string appearing both in
traceback source and the assertion; it corrects only the log counter and reuses
the retained local log, without changing or rerunning tests to hide results.

The parallel parent `make check` run reveals an unexpected native-fixture
failure: help/version cannot start its newly copied driver and `timeout`
reports `Text file busy` (exit 126). Native total is 5 green/89 red rather than
the established 6/88. This is not accepted as expected red, not a product pass
and not grounds to weaken an assertion. All other failures remain in the
new native suite. Task 2.1.3 stays `[~]`; the same independent author receives
the retained exact log for bounded read-only fixture-race investigation after
finishing the core controls. No fixture or production correction has yet been
applied. Parent core/check logs and partial summaries remain owned scratch
pending diagnosis; no commit/publication or actual package/emulator claim.

The independent reproduction uses six fixture workers and four unrelated
process-spawning threads. The unchanged fixture fails with the same exit 126/
`Text file busy` in 25/480 SDK and 2/480 host executions, over 240 distinct
exclusively created roots per run. A controlled descriptor probe proves that
a forked child can retain a writable `O_CLOEXEC` descriptor after the parent
closes it: both ELF and script execution fail while that descriptor survives,
then succeed after child exec. This establishes the mechanism; the original
historical failure itself was not descriptor-traced.

The scratch external-copy variant has 0/480 failures in SDK and host.
Hard links also avoid the observed race but can cross-filesystem-fail and share
the source inode; renaming does not sever inherited inode descriptors. Parent
applies the narrow mechanical correction only to native test fixtures:
`copy_driver` waits for `/bin/cp --preserve=mode --`; executable helper text
is written to a private never-executed input, closed, externally copied and
removed before launch. A small private helper shares that copy command.
This reuses existing Ubuntu coreutils, not a new product dependency/framework.
Every oracle assertion, barrier and overlapping-process case stays unchanged.
No retries, sleeps, launch serialization or product changes are introduced.
The independent author now stress-tests the exact applied helper read-only;
parent `make check` verification is rerun separately. Task 2.1.3 stays pending
until both results are available.

The exact corrected helper passes independent host ELF 480/480, host script
240/240, SDK ELF 480/480 and SDK script 240/240 executions, with six fixture
workers/four unrelated spawners/240 distinct roots in each run. Unrelated-spawn
counts are respectively 12,524, 15,463, 14,909 and 17,991. Umask-077 byte/mode,
separate-inode, replacement, read-only rejection and cleanup checks pass.
No descriptor race is observed; bounded stress is not an absolute guarantee.
The author edits no files and confirms all 17 supplied baseline files unchanged.

Parent `make check` after the correction returns to exactly 88 expected
behavioral red and six compatibility green in the 94-test native suite, with
no ETXTBSY or other failing suite. Parent core local/whole-file logs still
confirm 23 expected-red methods/52 absent-producer subtest records and two
missing-context setup errors for fourteen unrun actual methods. No product
changes occur between these runs and final hash verification. Only the
fixture module differs from the prior native oracle snapshot; every test
assertion/barrier module and all five core files match their reviewed hashes.
All source/documentation/syntax/whitespace/link checks pass.
`native-fixture-publication-proof.json` retains before/after identities,
the original failure, independent reproduction/stress and parent rerun;
`core-package-authoring-proof.json` retains the complete authoring results.
All owned parent/author scratch is removed. Task 2.1.3 verifies at
2026-11-19 05:24; progress 23/91. Real package/emulator proof and native
implementation remain incomplete.

#### Explicit core-package acceptance contexts

Local-only authoring/inner-loop command, from `tools/distribution/tests/`:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest \
  test_core_package_spec.CoreArchiveTests \
  test_core_package_spec.CoreStageArgumentTests -v
```

On each ordinary-user Ubuntu 22.04/CPython 3.10 and Ubuntu 24.04/CPython 3.12
producer context, with genuine matching support inputs and current binaries:

```sh
CLORRAINE_SUPPORT_ACCEPTANCE=/absolute/support-acceptance.json \
CLORRAINE_CORE_BINARIES=/absolute/current-binaries \
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest \
  test_core_package_spec.CorePackageArtifactAcceptance -v
```

The producer checkout is available for its version/notices. Run the runtime
group separately from the copied test directory with checkout, builder
prefixes, original SDK, network and alternate PATH cross-tools unavailable:

```sh
CLORRAINE_SUPPORT_ACCEPTANCE=/absolute/support-acceptance.json \
CLORRAINE_CORE_ARCHIVE=/absolute/core.tar \
CLORRAINE_CORE_ARCHIVE_SHA256=ACTUAL_PRODUCER_HASH \
CLORRAINE_CORE_INSTALLATION=/absolute/disposable-candidate \
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest \
  test_core_package_spec.CoreInstalledAcceptance -v
```

The existing support acceptance record supplies exact lock/archive hashes,
audits, matching Ubuntu/ABI and empty owned work root. The actual candidate
must be unactivated, movable, owned, disjoint from proof work and on the same
filesystem. It is moved/restored for stable-link relocation; failed restoration
retains and names the input rather than deleting it. Missing required inputs
are setup errors, never skips. Fourteen actual methods remain unrun at
authoring, not silently satisfied by local ELF/process fixtures.

### Support contract authoring — 2026-11-16

Task 1.1.1 follows the approved AR-30 interfaces in `03-01`. The independent
author (`ses_e1a08b45fffexKdF1RfLSQlImP`) wrote
`tools/distribution/tests/test_support_spec.py` and its test-only
`support_fixtures.py`, without reading the implementation targets.

Command: `python3 -m unittest discover -s tools/distribution/tests -p test_support_spec.py -v`.

| Cases | Tests | Observed red result |
|---|---|---|
| ST-1 | 5 | Valid lock plus branch/short-ID/missing-LHA/missing-hash cases all fail on absent `support.py` |
| ST-2 | 4 | Valid source, mismatched HEAD, failed recipe and preserved-output cases fail on absent `support.py` or `build-support.sh` |
| ST-10 | 15 | Valid and rejected payload, notice/source, origin/content, builder-path and link/output cases fail on absent `package_support.py` |

All 24 tests fail; no fixture errors or skips. Real Git and filesystem setup
completed. Entry-point existence is asserted before negative-result checks so
a missing implementation cannot count as a successful rejection. The failed
Docker control must reach the locked recipe, not merely fail an earlier probe.
Syntax, entity documentation, planning-reference and whitespace checks passed.
A direct fixture-boundary probe also confirmed the synthetic image-inspection
response and failed-fetch/partial-output control without using a Docker daemon.
This is authoring evidence, not a green implementation or the complete Phase 1
red gate; ST-3–ST-9/ST-60 and task 1.1.4 remain pending.

Task verification: `make check` passed after authoring, with 4,232 Rust tests,
Python suites 29 + 103 + 28 and fuzz smoke
`cases=28 host=28 sanitizer=16 target=16 failures=0`. The new support suite is
run separately until task 1.3.2 connects it. Full `make verify` remains at the
phase gate after the private-display prerequisite; no platform acceptance is
claimed from these synthetic fixtures.

### Preparation and artifact authoring — verified 2026-11-17

Task 1.1.2 adds 18 local preparation/wheel cases and 5 real-artifact methods,
exposed by the same entry file. Whole-suite red discovery fails explicitly on
missing implementation and `CLORRAINE_SUPPORT_ACCEPTANCE`; no missing candidate
is reported as a skip. The original 24 expectations remain intact.

Authoring review checked the actual SDK archive format: `libamiga.a` is an ar
archive whose members report `a.out-amiga`, architecture `m68k:68010`, through
`m68k-amigaos-objdump -f`. Target archive checks therefore use the packaged
cross-tool, not host ELF parsing. This preserves the required architecture
oracle. Host dependency checks must preserve paths containing spaces, and
offline installation must reinstall the supplied pinned wheels rather than
silently accepting Ubuntu's existing pip. These are test-instrumentation
corrections before any support implementation exists.

The task's regression baseline `make check` passed: 4,232 Rust tests, Python
suites 29 + 103 + 28, and fuzz smoke
`cases=28 host=28 sanitizer=16 target=16 failures=0`. Final authoring verification
included rechecking the corrected artifact instrumentation and red suite.
Discovery runs 42 local methods (52 failing assertion records including
subtests), all blocked by absent planned public entries. Real acceptance fails
its class setup for the missing explicit input record; its 5 methods are
unrun, not passing or skipped. All 47 methods and embedded interpreter probes
pass syntax/documentation/reference checks. No actual Ubuntu/support archive
acceptance has run. This verifies authoring only.

### Private-display authoring — verified 2026-11-17

Task 1.1.3 adds eight direct/wrapped caller-precedence cases using copied
wrappers and a logging Docker fixture. Review confirmed the public handoff is
Docker Compose: image and bind settings are resolved from configuration, not
literal Docker-run options. Fixture observations must therefore verify the
actual Compose invocation and its effective `DISPLAY`/`XAUTHORITY`.
Exact container authentication and bind resolution remain mandatory real
Docker/FS-UAE evidence in task 1.2.2; a logging fixture cannot establish them.
This corrects test instrumentation, not the precedence or real-proof oracle.

The pinned-container run of `cargo test --workspace --test native_gate_spec_test`
now produces the expected red result: six nonempty caller overrides fail at
effective `DISPLAY`/`XAUTHORITY` assertions, while both unset-caller controls
pass. There are no compilation or fixture failures. Configurations remain
unchanged; no real Docker/emulator invocation occurs inside these tests.

Formatting evidence comes from the author's isolated Rust 1.85.1 formatter
(`--check --edition 2021`), whose temporary binary/libraries were cleaned.
The host has no `rustfmt`, and the SDK's installed stable toolchain lacks that
component; parent reproduction attempts therefore reported unavailable tools.
Parent independently type-checked the file through the pinned-container red
test run. No formatting tool was added to project dependencies.

The task-level `make check` also reaches exactly the six expected new failures,
after 3,244 passing Rust tests; two new default cases remain green. Cargo
fail-fast prevents later suites and Makefile legs from running in this red
state. They must pass after implementation. This is successful specification
authoring, not a passing regression or acceptance gate.

### Phase 1 red gate — 2026-11-17

Task 1.1.4 reran both declared specification entries before implementation.
Support discovery has the same 42 local expected-red methods / 52 assertion
records and explicit missing-real-input setup failure. The display suite has
six override failures and two green default controls. Those controls are green
because preserving configured defaults is existing behavior. No red assertion
was weakened, no absent artifact was skipped, and no emulator launched.

The task-level `make check` result immediately before this consolidation is the
same expected six-new-failure signature; no implementation changed between the
runs. Full regression and real private-display verification follow the fix.
The immutable authored oracle snapshot is:

| File under `tools/distribution/tests/` unless noted | SHA-256 |
|---|---|
| `test_support_spec.py` | `e950a7753274902465f5517df4c5e2b6ccf19631924bf4d492ac65ad80211470` |
| `support_fixtures.py` | `daba1c7dced9e75a577737860d92c00a401e425a2814ac725c3b6c44e943f10f` |
| `support_preparation_spec.py` | `c227538127a1fb7c916e84fb408802602a2c2f89c3404b82fe9f7142d6c62bb4` |
| `support_artifact_spec.py` | `1e889854de0a9acd72f8b5f5f879b3ac69503ad4de5c720844ea117a04536a18` |
| `support_artifact_fixtures.py` | `162ea716503172a6a51216e1ce69889311c9cbe9a6836a330785a08df0c1cdb9` |
| `compiler/tests/native_gate_spec_test.rs` (repository-relative) | `123059cb9eff13c33d4b6424a18b7225c497e8993fa4811d9891bfda68d83e66` |

### Caller-display implementation — verified 2026-11-17

Both wrappers now capture and restore nonempty caller `DISPLAY`/`XAUTHORITY`
around configuration loading, using the existing A500 gate pattern. All eight
unchanged behavioral tests pass. `bash -n` and ShellCheck 0.11.0 pass for both
files. ShellCheck was extracted temporarily from a distro package verified
against local apt metadata; no system or product dependency was installed.

The first real private-display probe exposed a verification-environment issue:
Xvfb's hostname-specific cookie was not usable by the container. No full gate
started. The probe was corrected to retain authentication while adding an
Xauthority wildcard-family entry for the same private secret. It does not edit
the user's `.env` or touch the desktop. Real process, bind and guest-output
checks must pass before `make verify` may start.

The corrected real probe passed for direct `emulator.sh` and the
`run.sh` → `emulator.sh` chain with FS-UAE 3.1.66. It checked the actual
FS-UAE process environment, exact read-only Xauthority bind and mounted bytes,
an X window on the private server, and the expected hello guest output.
Both owned emulator containers and the private X server were stopped; the
temporary decrypted ROM and cookie were removed.

Only after those proofs did `make verify` run on the private display. All legs
passed, including licensed legacy FS-UAE gates. Its initial `make check` passed
4,240 Rust tests (the previous 4,232 plus eight display cases), Python suites
29 + 103 + 28 and fuzz smoke
`cases=28 host=28 sanitizer=16 target=16 failures=0`. The display oracle hash
is unchanged. No user `.env` edit, tag, workflow dispatch, commit or publication
occurred. This full gate proves the wrapper change, not the still-unbuilt
native support archives or the remaining Phase 1 feasibility obligations.

Task 1.2.2 records the reusable setup in `tools/distribution/README.md`.
Its Bash examples, tables, whitespace and evidence boundaries validate;
`make check` passed again after the guide was written with 4,240 Rust tests,
160 Python tests and fuzz smoke 28 cases / zero failures.

### Approved image-contract amendment — authoring verified 2026-11-17

AR-32 explicitly permits changing the affected frozen support expectations:
the registry digest identifies the Ubuntu base; the hashed environment recipe
produces a local execution image selected by immutable ID. Support and wheel
commands now receive that ID explicitly. Source verification, non-root/offline
execution, failed-build cleanup and all real acceptance obligations remain.

The narrow amendment updates the valid synthetic environment input, external
Docker image-inspection fixture and assertions identifying the execution image.
Additional cases must reject invalid image IDs or mismatching base/recipe
evidence before recipe execution. The earlier red-oracle hashes remain historical
evidence; record the amended hashes after authoring verification. The eight
private-display tests and real-artifact acceptance expectations are unaffected.

The approved amendment is authored in exactly `support_fixtures.py`,
`test_support_spec.py` and `support_preparation_spec.py`. It adds six cases,
giving 28 contract, 20 preparation and five mandatory real-artifact methods.
Discovery executes 48 local methods with 61 failing assertion records,
including subtests. Every local failure identifies a missing planned public
command. Real acceptance fails class setup for missing
`CLORRAINE_SUPPORT_ACCEPTANCE`; its five methods are unrun, not skipped.

Parent checks passed for Python syntax, embedded Docker-fixture syntax,
entity documentation and absence of planning references. Direct fixture
exercises confirmed distinct inspected base/prepared IDs, recipe/base labels,
base-layer ancestry, all four corrupted-metadata controls, and the failed
external recipe's partial-work injection. These check test instrumentation;
they do not prove a real prepared image or build isolation.

`make check` passed after the amendment: 4,240 Rust tests, Python suites
29 + 103 + 28 and fuzz
`cases=28 host=28 sanitizer=16 target=16 failures=0`. Support tests are still
outside that Makefile gate until the planned integration task.

The explicitly approved amended oracle snapshot is:

| File under `tools/distribution/tests/` | SHA-256 |
|---|---|
| `test_support_spec.py` | `a11c53137fe50aa20ad40c90ffb8c5fd3f0040a6282f3e8c31dc2f73f12d2f67` |
| `support_fixtures.py` | `5bb5236f5a19a833c6390941af26e951b6a5f92ba7a01d896cfaf975bfd223e4` |
| `support_preparation_spec.py` | `8cf264e99bae44092fb58f724855e424ff7537b96845b5904c54d3eae1d406f9` |

The two real-artifact files and the display file remain byte-identical to the
earlier snapshot. The unchanged display cases passed within `make check`.
The earlier full private-display gate remains evidence for the wrapper change,
not evidence for native support artifacts.

### Input and environment recipe work — partial

Task 1.2.3 now records the resolved source/backend inventory under
`tools/distribution/inputs/README.md`. Seven selected backend source archives
and CPython 3.12.16 were downloaded and checked against exact published
lengths/hashes. Source inspection rejected the earlier tomli 2.2.1 candidate:
setuptools-scm's Python 3.10 build requires tomli at most 2.0.2. The selected
2.0.2 bytes satisfy that build bound and its runtime lower bound.

Ubuntu acquisition used the pinned base's dpkg status and archive keyring,
the fixed 2026-11-17 snapshot's three suites / six main-universe indexes,
31 direct requests and no recommends. All 126 downloaded package archives
(123,238,900 bytes) matched signed-index lengths/SHA-256. The durable
`ubuntu-packages.sha256` was checked against both the records and actual
archives, including a real `sha256sum --check` over a mirrored tree.
The earlier 2026-11-01 proposal could not resolve against the newer base;
the recorded later snapshot resolved without a downgrade. No packages were
installed during acquisition.

The environment Dockerfile and Python-backend preparation shell recipe are
authored. Checks pass for shell syntax, ShellCheck 0.11.0, Docker `build --check`,
bound script/checksum hashes, invalid-argument rejection, root-build rejection,
guide syntax, links and tables. Dockerfile SHA-256 at this point is
`28610b8f2d5401b7cfa2aed422653fa6ed65c08f2c513ec461857ccff8a456fa`.
Docker's check mode did not execute build steps or create a prepared image.

These are input/static checks only. The remaining support reference recipe,
licence reviews, actual environment/reference build, final lock production and
Ubuntu artifact/relocation acceptance remain pending. No public implementation
was created for the still-red support command entry points.

The final `make check` after the input/recipe changes passed: 4,240 Rust tests,
Python suites 29 + 103 + 28 and fuzz
`cases=28 host=28 sanitizer=16 target=16 failures=0`. No prepared environment,
reference toolchain or support archive was built by this gate. At that point,
task 1.2.3 remained partial because the support reference recipe was not yet
finished.

### Private SDK reference recipe authoring — verified 2026-11-17

`tools/distribution/inputs/reference-support.sh` is now authored as the
private full-SDK reference producer. It is not the public support command or
an ordinary distributable payload: it deliberately retains the NDK-derived
reference tree for later inventory/origin auditing.

Shell syntax, ShellCheck 0.11.0 and embedded Python syntax pass. Direct
link-check probes accept an internal alias and reject escaping, dangling and
cyclic links without changing an outside sentinel. Argument and root guards
reject before work is created. A real Git fixture with a wrong identity is
rejected before output; the old SDK supplied only that negative probe's tools,
not Ubuntu compatibility or successful-build evidence.

Parent checks also match the recipe's fixed source roster against the input
catalog. Read-only inspection of the pinned upstream root confirms the
expected build data and no pre-populated tracked `projects`/`download`
directories. The recipe uses the selected LHA, exact archive inputs and native
upstream build settings. It checks sources before private copying and discards
only its owned reference tree on failure.

Current recipe SHA-256 is
`e2dcce18a964665e737c74d6ada5db6005f1e817c0a69d77209f1e257b041f1c`.
The declared support suite retains its expected-red signature: 48 local
methods, 61 assertion records identifying absent public commands, and explicit
missing-artifact-input class setup failure. The final task-level `make check`
passed with 4,240 Rust tests, Python suites 29 + 103 + 28 and fuzz
`cases=28 host=28 sanitizer=16 target=16 failures=0`.

Task 1.2.3 verifies fixed-input research and recipe authoring only. Docker
check mode is not an executed environment build. Licence reviews precede the
actual environment/reference build and full Dockerfile validation in task
1.2.5; final lock and real artifact acceptance remain later tasks. No source
licence permission or successful binary build is inferred from this authoring
gate.

### Licence/origin review — evidence hold

Task 1.2.4 records a preliminary review in
`tools/distribution/inputs/licence-review.md`. The ordinary upstream
`libamiga.a` download matches the existing SDK archive byte-for-byte. Both
ordinary and base-relative reachable histories were checked through their sole
introducing commit. Complete root/libnix tree inspection and selected build/helper
files did not establish the required archive-specific origin/terms/source chain.
The independent challenger confirmed the evidence boundary and requested the
bounded history pass, now completed.

AR-33 / DEF-560 therefore holds the affected asset and dependent reference
build under existing AR-18 policy. This is not a conclusion that the archive is
unlicensed; source/permission evidence is unresolved. No test was changed or
skipped and no lock/build proof was fabricated to bypass the hold.
The latest `make check` remains the passing 4,240 Rust / 160 Python / zero-fuzz-
failure authoring baseline; no native support acceptance is claimed.

The hold's non-code validation passed: archive byte lengths/hashes, both
one-commit histories, changed-path evidence, complete-tree counts, selected
notice hashes, Markdown tables/links and the 7/90 task state. The legacy
roadmap has 590/594 rows done after adding the external evidence dependency;
the existing sync engine preserves this hand-maintained PLAN/DEF format.
`THIRD_PARTY_NOTICES.md` was left unchanged rather than asserting unverified
native-asset clearance. All downloaded binaries, NDK bytes, notices and
research metadata were removed after recording the evidence; no upstream
message or public release operation occurred.

### User-directed libamiga inclusion — verified 2026-11-17

The user explicitly directed copying and including the ordinary archive and
retained responsibility for its legalities. AR-33 now records that bounded
inclusion instruction; DEF-560 remains user-owned legal follow-up, not a local
implementation blocker. The unresolved historical source/terms evidence is
retained without a licence-clearance claim.

The exact upstream file is now `tools/distribution/inputs/libamiga.a`:
297,694 bytes, SHA-256
`10d78b6d303a99677c0bf29a8e1653d62669f69f395691646393020325084021`.
Its absence was checked before copying; the downloaded bytes were verified
before an atomic non-replacing installation. The file is mode 0644 and contains
1,149 m68k archive members, inspected with the packaged cross-objdump.

The reference recipe verifies and installs this explicit input and rechecks its
final SDK hash. A narrowly anchored `.gitignore` exception makes this one
archive visible without unignoring unrelated `.a` files. Provenance and
user-owned legal-review status are documented in the input guide, review and
`THIRD_PARTY_NOTICES.md`. The base-relative archive was not added.

Archive/hash/mode/member checks, Git visibility and unrelated-archive exclusion,
shell syntax, ShellCheck 0.11.0 and documentation checks pass. Because the change
touches shared ignore configuration, full `make verify` ran on an owned,
authenticated private Xvfb display. All legs passed. The initial check passed
4,240 Rust tests, Python suites 29 + 103 + 28 and fuzz
`cases=28 host=28 sanitizer=16 target=16 failures=0`. The copied archive's
hash remained unchanged and the private server/cookie were cleaned.

This verifies archive inclusion and the existing project gate, not a fresh
Ubuntu support build. No commit or publication was performed. Other component
reviews and native artifact proof remain pending; task 1.2.4 is still partial.

### Component notice and origin recording — 2026-11-17

The recorded review now covers all four additional exact archives (GMP,
MPFR, MPC and maintainer CPython), seven exact-revision LHA notice/exception
files, and inherited target-code notices. Source/member hashes were rechecked
against the reviewed bytes; every selected hash in the Python, build-component
and origin documents matches source evidence. Markdown table/whitespace/link
checks pass.

The full LHA distribution section was read, including its network/offline
distribution note and commercial-use conditions. No unrestricted-use label,
email, publication or extension of the libamiga inclusion direction follows.
MPC's library declaration was checked in `src/mpc.h` and `src/add.c`, rather
than inferred from its README-file notice.

Six small exact-revision component trees were inspected: libnix, fd2sfd,
sfdc, fd2pragma, netinclude and AROS. Only the pthread subtree of AROS is a
selected runtime origin. These checks exposed file-specific notices that root
labels miss, sfdc's Perl-script producer, and libnix's gnuplot-derived source
distribution form. A scratch original-plus-patch reconstruction from the
inspected released gnuplot 3.7.1 `amiga.c` reproduced the pinned `pipeoc.c`
hash with zero patch fuzz. Source-package identification/contact preservation
and delivered contents still require packaging proof.

The install-rule trace identifies NDK copies, generated wrappers, 1.3
derivatives, copied link libraries, helper headers, networking overwrites and
newlib/libnix/pthread installs. A bounded regular-file hash comparison in the
immutable existing SDK found one identical header outside NDK-named trees:
`sys-include/resolv.h` matches `ndk-include/resolv.h` and the pinned netinclude
source. This does not prove NDK authorship or rule out transformed content.
No emulator or desktop display was launched.

Support specification discovery remains the expected red: 48 local methods,
61 absent-public-command assertion records and one mandatory missing-real-input
class setup error, no skips. All three approved support-oracle hashes remain
unchanged. An initial evidence-check snippet expected the wrong acceptance
environment-variable name; the checker was corrected to
`CLORRAINE_SUPPORT_ACCEPTANCE`. No specification test was changed.

The task-level `make check` passed: 4,240 Rust tests, Python suites
29 + 103 + 28 and fuzz
`cases=28 host=28 sanitizer=16 target=16 failures=0`. Documentation
self-review and `git diff --check` pass. This task records source obligations
and origin/exclusion evidence; it does not accept a new native support build.
The fresh reference inventory, final lock, actual source/notice assembly and
complete payload acceptance remain their later declared tasks.

The owned notice-review scratch was removed after recording these results.
Verified archive bytes were copied into the generated reference-build inputs
at `build/distribution/reference-20261117`; these are retained for the following
reference/acceptance work, not committed or published support assets.

### Reference environment preparation started

Task 1.2.5 reuses the verified source archives in the generated build area.
Its environment context contains only the declared recipes, nine selected
source archives and the pinned Ubuntu package tree. Package acquisition and
the offline Docker build run separately from exact-commit Git source staging.
Neither job is a successful reference-build or final-lock claim.

The initial manual acquisition launcher rejected valid Ubuntu filenames
containing `~` before any download/build step. Its local path allowlist was
corrected; source identities, checksum records, recipes and specification
tests were unchanged. The corrected package acquisition/preparation and the
independent exact-source staging jobs are running. Current owned logs are in
`build/tmp/v120-reference.vyc6x_5_`; clean them after recording their results.

Exact source staging has now passed: 12 clean complete-commit checkouts and
all 16 archive lengths/hashes match the catalog. All 12 source symlinks resolve
within their respective checkouts. Reacquisition also verified the same 126
Ubuntu packages, totalling 123,238,900 bytes.

The first real Docker preparation reached APT only after all input checks
passed, then failed with “Pathname to install is not absolute”. A disposable
base-container probe first seeded unchanged pool filenames; that progressed
further but failed on an epoch-bearing package. A second probe used canonical
APT cache names, preserving the package epoch as `%3a`, and installed the
complete set successfully with networking disabled and `--no-download`.

The environment Dockerfile now seeds those canonical names from the already
verified archives and validates the package/version/architecture fields before
constructing paths. No package/source pin, public contract or specification
test changed. The corrected full image build is running; source builds remain
unprivileged after the declared package-installation step. No successful image,
reference SDK, final lock or native support archive is asserted yet.

### Prepared image measured; private reference running

The amended Dockerfile passes check mode and its embedded installation shell
passes syntax validation. The complete network-disabled build succeeded as
`sha256:0bea57344e1ed0024b0968192f40c44b716fd27e504307a0d817f81d29bcc9a3`.
Its Linux/amd64 platform, exact base-layer prefix and both preparation labels
were rechecked before execution. Dockerfile SHA-256:
`3463b57421aecd01a1c5f8d6219af34e3313d0611cb83f53162474d75b2154ee`.

Measured environments have Python 3.10.12 / 3.12.16 and exactly the eight
selected backend packages at the recorded versions in each ABI environment.
All 126 acquired package versions match the image's 226-package installed
cohort. Ubuntu pip/setuptools and CPython ensurepip bootstrap receipts match
their separately recorded versions. Native GCC/G++ is Ubuntu 11.4.0.
`dpkg --audit` is empty and `apt-get check` passes.

Initial measurement snippets needed two corrections: non-root APT consistency
checking uses `Debug::NoLocking=1`, and Python distribution names use standard
hyphen/underscore/dot normalization (`flit_core` becomes `flit-core`).
Neither changed an image, input pin, public contract or test oracle.

Using that exact image, real negative probes reject a correct-length but
wrong-hash libamiga input and an exact-HEAD but wrong-origin checkout before
creating reference output. Nested read-only test mounts leave the original
source/config bytes unchanged; their owned altered fixtures were removed.

The positive private SDK build is running as UID/GID 1000 with networking
disabled, read-only sources and a fresh `/work` mount. Its exact argv and
environment/source receipts are under `build/distribution/reference-20261117`.
The actual SDK result, inventory, final lock and task verification remain
pending. No desktop/emulator, commit or publication operation was used.

### First SDK failure and flag-forwarding correction

The first actual reference attempt built and installed the host compiler,
binutils/GDB/gprof and newlib, then failed in libnix startup compilation:
`libinit.c:7` could not find `libinit.h`. The outer recipe had supplied
`CFLAGS=-Os` as a Make command-line override. GNU Make propagated it into
recursive invocations, defeating both the upstream target `CFLAGS` environment
assignment and libnix's appended `-I.../sources/headers` flags. The recipe's
failure trap removed the entire partial reference; no successful output was
retained.

A fresh, network-disabled prepared-image probe mirrored only that upstream
recursive call and used the actual libnix Makefile with `make -n`.
The old command-line flags reproduced `-c -Os` without the required include.
Keeping the fixed host flags exported instead produced the intended
`-O2 -fomit-frame-pointer` plus the required libnix include path.
The probe did not compile or change source checkouts.

The reference recipe removes only the two host command-line flag assignments;
their exported fixed values and explicit target flag assignments remain.
Source pins and upstream files are unchanged. Shell syntax and ShellCheck
0.11.0 pass. Corrected reference-recipe SHA-256:
`0649467161c77f4b78b0df562be322bc527f6f66fdb7519d1256e3ba450ca35a`.
The complete offline build has restarted in the now-empty owned work mount
using the same inspected image ID. Its success, inventories and final lock
remain unverified.

Task-level `make check` after this correction passed: 4,240 Rust tests,
Python suites 29 + 103 + 28 and fuzz
`cases=28 host=28 sanitizer=16 target=16 failures=0`.
The support oracle retains its unchanged expected-red signature: 48 local
methods, 61 absent-command assertion records and one required missing-real-input
setup error, with no skips. Completed failure/probe/lint/check scratch was
cleaned after recording results. The active SDK-build log is retained while
that separate build runs; task 1.2.5 remains partial.

### Private reference passed; measured contract/order stop — 2026-11-18

The corrected offline reference build passed and retained source/compiler/
package receipts. All 11 C-toolchain source identities match acquisition
records, and its installed-package receipt matches the measured prepared
image. The ordinary libamiga hash is unchanged.

Measured SDK: 3,793 regular files, 11 safe internal relative links, six empty
directories and 1,372 hashed files under the NDK/prepared-tree locations.
That last set includes independent overlays/helper files and is not a complete
origin assignment or forbidden-hash list.

All 52 native ELF files are x86_64, require GLIBC at most 2.35 and have no
RPATH/RUNPATH. All 166 target archives were inspected using the built cross-
objdump; all member architectures are m68k, and none are empty. Of these,
165 use Amiga HUNK archive format rather than Unix `!<arch>` magic.
There are also 26 standalone target objects; `ncrt0.o` and `libinit.o` are
confirmed `file format amiga`, m68k, without archive wrappers.

Strict C99 hello links pass for `nix13` and `nix20` with the private SDK
mounted read-only at `/sdk` in the prepared image, original build prefix and
old `/opt/amiga` unavailable. Outputs were inspected as m68k. No target
execution, actual-archive relocation acceptance or full corpus claim follows.

AR-34 now stops eligible-lock completion and dependent implementation pending
an explicit ruling: accurately represent standalone Amiga objects in the
native-audit enum and put all hashed producers before final lock completion.
The current contract, oracle hashes and 90-task ordering remain unchanged.
Proposed amendments are recorded, not applied. No placeholder lock was written.
The generated private reference/receipts are retained for resumed verification;
owned build/compile-probe scratch is cleaned after recording these results.

### Approved object oracle amendment and reference-proof completion

The user selected “Approve both (Recommended)” for AR-34. The contract now
represents standalone target HUNK objects as `amiga-object`/`m68k`, separately
from `ar`. Final eligible-lock completion is task 1.2.10, after its producers;
current plan size is 91 tasks. Source pins, dependencies, public commands and
acceptance gates are unchanged.

The narrow oracle amendment adds a real host-ELF/misdeclared-target-object
rejection case and a mandatory real-artifact startup-object case. The existing
native-closure helper retains its archive checks and separately verifies the
actual standalone Amiga format, non-archive status and architecture. No host
ELF, notice, source, dependency or relocation assertion was removed.

Current support suite: 55 methods — 29 contract, 20 preparation and six
mandatory real-artifact cases. Discovery runs 49 local methods and produces
the expected 62 absent-command assertion records plus one missing-real-input
class setup error. No skips or compiler-fixture errors occurred; the new
negative fixture compiled a genuine x86_64 ELF object. All six acceptance
methods remain unrun, not green, until actual candidate inputs exist.

| Approved current oracle | SHA-256 |
|---|---|
| `test_support_spec.py` | `59e76a35b0e3b1af42f91c9da321a3acaee6f67e8b33bb60862172bcee541deb` |
| `support_preparation_spec.py` (unchanged) | `8cf264e99bae44092fb58f724855e424ff7537b96845b5904c54d3eae1d406f9` |
| `support_artifact_spec.py` | `b9374b934e0882284adc5c103fe1b3b03012c2f34e06d04f627738ad4b569c45` |

All 26 private reference objects independently match the approved format and
architecture constraints. Task-level `make check` passes: 4,240 Rust tests,
Python suites 29 + 103 + 28, fuzz
`cases=28 host=28 sanitizer=16 target=16 failures=0`.
Task 1.2.5 now verifies reference proof/oracle authoring only; the actual lock,
public producers, packaging and clean-machine acceptance remain later work.

Before validator implementation, AR-35 separately records that actual Ubuntu
package names (`g++`, `libstdc++6`) cannot use the generic ID grammar. That
package-only correction has not been applied to contract/tests. Reference
verification is complete; affected implementation waits for its ruling.

### Package-name amendment and source-verifier verification in progress

The user approved AR-35's package-only grammar correction. Its positive and
unsafe-name cases were authored before validation code. Further source
integrity cases cover staged edits, ignored extras, internal/escaping links,
non-execution of configured Git filters and forged Git object identities.

The current oracle has 64 methods: 38 contract, 20 preparation and six
mandatory real-artifact cases. Before implementation, discovery ran 58 local
methods with 86 expected absent-command assertion records and one required
missing-artifact setup error, without skips or fixture errors.

| Current oracle | SHA-256 |
|---|---|
| `test_support_spec.py` | `25ee3017f1dd80464218cadfcda9eadf2a9b394ea620ed0a124016bf8bdf7044` |
| `support_preparation_spec.py` (unchanged) | `8cf264e99bae44092fb58f724855e424ff7537b96845b5904c54d3eae1d406f9` |
| `support_artifact_spec.py` (unchanged since AR-34) | `b9374b934e0882284adc5c103fe1b3b03012c2f34e06d04f627738ad4b569c45` |

After implementation, all invocations of the two implemented public support
commands pass. Full discovery still fails on 60 assertions for absent sibling
build/package/preparation/wheel commands, plus the mandatory real-artifact
setup error. All six actual-artifact methods remain unrun, not skipped or
accepted. Eleven focused source/lock methods pass, including 16 unsafe
package-name controls.

Additional bounded probes pass 82 schema/source negative controls and
exact-byte lock-digest checks. Eight resource/metadata/source probes reject
oversized/deep/duplicate/invalid-UTF-8 JSON, a huge JSON integer, a FIFO lock,
partial-clone metadata and a tracked link into unlocked Git metadata.
The integer/FIFO/link/partial-clone probes first demonstrated the defects;
the implementation was corrected without changing the specification oracle.
File traversal now propagates unreadable-directory failures instead of
silently treating them as empty.

Real acquired-source probing exposed a different issue: fd2pragma's committed
attributes expand its leading `$Id$` into the actual blob identity on checkout.
The new exact-byte checker rejects those transformed bytes even though Git
reports a clean checkout. No acquired checkout or pin was changed, and no
exception was added to the checker. Task 1.2.6 is not verified pending a
bounded source-identity ruling. A real reference-source probe is not a final
lock or approval of the probe's notice-path selection.

The completed actual-source probe passes 11/12 repositories; only fd2pragma
fails on its ident-expanded C comment. The independently checked raw/checkout
files differ only in that marker. All pins and acquired/reference bytes remain
unchanged. AR-36 / DEF-563 records the explicit normal-mode ruling needed;
the independent challenger recommends canonical raw staging with affected
reference reproof rather than a verifier transformation exception.

Task-level `make check` passes after implementation/edge corrections:
4,240 Rust tests, Python suites 29 + 103 + 28, fuzz
`cases=28 host=28 sanitizer=16 target=16 failures=0`. This does not override
the real-source blocker or turn unbuilt sibling commands green.
The owned source-verification scratch/logs are cleaned after recording this
evidence. The existing generated private SDK/source/reference receipts under
`build/distribution/reference-20261117` remain retained for resumed proof.

### Canonical source identity approved and affected helper re-proved

The user chose “Canonical bytes (Recommended)” for AR-36. Two additional
specifications were authored and run before staging changes: raw stored bytes
pass despite committed ident attributes; an actually Git-expanded clean
checkout fails without being rewritten by verification. Both were already
green because the existing strict checker implements the selected identity
rule. The failing actual acquired checkout was the red staging input; no
verifier acceptance was relaxed to make it pass.

Current oracle: 66 methods — 40 contract, 20 preparation, six mandatory
real-artifact cases. `test_support_spec.py` SHA-256 is
`70509b1de18bfc85da59b27f9aed550ff169cf6c5ca556cc31ae2f5ba072ece7`.
The preparation/artifact oracle hashes remain unchanged.

Only the authenticated one-marker fd2pragma conversion was canonicalised in
a fresh owned candidate, fully verified, then placed in source staging.
The original acquired checkout is preserved at
`build/distribution/reference-20261117/source-originals/fd2pragma`.
The original SDK/reference and source pins are unchanged.

A fresh build of the affected helper through the pinned upstream Makefile,
same offline prepared image and fixed compiler flags produces exactly the
same 181,224-byte ELF binary, file mode and observed output as the original
reference. Binary SHA-256:
`687935426345a83c7c71ac06916ee7dc28164ea6847232a43f7fdf454885d832`.
Generated `source-canonicalisation.json` and `canonical-helper-reproof.json`
retain that bounded proof. No rebuilding of unrelated components or changes
to the original SDK inventory/native records were needed.

Task 1.2.6 verification completes at 2026-11-18 15:19. All 12 actual acquired
checkouts now pass the implemented object/index/worktree checker. The retained
`canonical-source-verifier-probe.json` records that bounded result; probe
notice paths establish file existence only, not final licence/notice selection.
Both public support commands also pass on read-only synthetic fixtures under
the prepared Python 3.10 and 3.12 interpreters, networking disabled.

Full support discovery runs 60 local methods and remains expected red solely
for 60 absent-sibling-command assertion records plus mandatory real-artifact
setup. All implemented support invocations pass; the six real-artifact
methods remain unrun, not skipped. The task-level `make check` passes with
4,240 Rust tests, Python suites 29 + 103 + 28 and fuzz
`cases=28 host=28 sanitizer=16 target=16 failures=0`.
Owned canonicalisation/helper-build/test logs are cleaned; original sources,
private SDK and the named generated canonicalisation/reproof receipts remain
under `build/distribution/reference-20261117` for subsequent producer proof.

### Support-build producer authoring and bounded proof in progress

Nine independent build specifications now cover successful exact provenance,
empty destinations, safe links, disjoint outputs, output-symlink preservation,
escaping links, empty payload directories, special files and Docker's
normalised base-repository display name. The initial eight methods ran red
on the absent public script before implementation; the additional Docker-name
case reproduced the real inspection mismatch before that correction.
The external Docker fixture keeps its default failing-fetch behaviour.
Only explicitly enabled success cases simulate successful recipe execution.

Current oracle: 75 methods — 40 contract, nine build, 20 preparation and six
mandatory real-artifact cases. Full discovery runs 69 local methods and
remains expected red only for 46 absent packaging/preparation/wheel command
assertions plus required real-artifact setup. No test is skipped or weakened.
All invocations of implemented source/lock/build commands pass.

| Current oracle | SHA-256 |
|---|---|
| `test_support_spec.py` (adds build-suite discovery only) | `48cd26ad3819386bedf74b429230279a78eadb18c8e6f62ec4903b8b81b3928f` |
| `support_build_spec.py` | `44ccd143a0a0f5bd152ee2b680da288e3db7792a4929f051a53ea8f972db6587` |
| `support_preparation_spec.py` (unchanged) | `8cf264e99bae44092fb58f724855e424ff7537b96845b5904c54d3eae1d406f9` |
| `support_artifact_spec.py` (unchanged) | `b9374b934e0882284adc5c103fe1b3b03012c2f34e06d04f627738ad4b569c45` |

Real public-builder probes use synthetic source/lock fixtures and the actual
inspected prepared image. Successful execution publishes exact provenance.
A second literal recipe creates partial private work then attempts to fetch
an undeclared URL; the actual network-disabled container reports unreachable
network, failure propagates and no successful/partial output survives.
This establishes the actual execution boundary, not final dependency closure.

The tracked payload selector was also exercised on the actual private
reference in the pinned image, without rebuilding unrelated components.
Its 423 files and 11 links all match the measured reference. Prepared NDK
trees and unneeded SDK programs are excluded. Strict C99 compile/link probes
for `nix13`/`nix20`, ordinary and LTO, pass with the candidate mounted at
`/sdk` and the original builder prefix unavailable. Generated selection and
link receipts plus the private candidate remain under the reference directory
for later preparation/audit proof. No final lock/archive/notice acceptance.

Shell syntax and embedded Python syntax checks pass. ShellCheck was absent
on both host and SDK; the pinned-index Ubuntu package `shellcheck` 0.11.0-2,
SHA-256 `06990eaf21b5dfa3d855e492f7ca2d6fcacf63b490a73a545850c50cf7af2bce`,
was downloaded, hash-verified and extracted only in owned scratch. Lint
passes without adding a system or product dependency. The package and its
extracted files are removed with the invocation's verification scratch.

Task 1.2.7 verifies at 2026-11-18 15:39. All nine build methods also pass under
the prepared Python 3.10 and 3.12 interpreters, with code mounted read-only
and no Docker socket exposed to the test container. An additional standard
stdio hello link probe passes both runtime profiles, ordinary and LTO, using
only the selected candidate. `support-selection-stdio-link-proof.json` and
`support-producer-proof.json` retain these bounded authoring results.

Task-level `make check` passes: 4,240 Rust tests, Python suites 29 + 103 + 28,
fuzz `cases=28 host=28 sanitizer=16 target=16 failures=0`.
No final lock, complete real locked producer run, delivered notices/source,
archive audit or clean-machine acceptance is claimed. Owned task logs,
compiler probes and temporary lint-package files are cleaned; the named
private candidate/reference receipts remain for subsequent preparation proof.

### Standalone NDK preparation authoring and proof in progress

The unchanged 20-method preparation/wheel oracle was rerun red before
implementation; all failures named the absent public preparation/wheel entry
points, with no fixture errors. Fourteen preparation/member methods now pass.
The six wheel methods still fail only because that command is unimplemented.
Full support discovery remains expected red only for 22 absent packaging/wheel
command assertions plus the mandatory real-artifact setup error. The same
75-method oracle and all recorded oracle hashes remain unchanged.

`prepare_ndk.py` is self-contained when copied beside its data. It validates
complete archive listings and alias resolution without mutating the listing,
rejects duplicate/unsafe metadata, authenticates helpers/inputs and exact
baseline contents before any helper, and verifies the complete overlay before
writing the four-field prepared marker last. Resource traversal rejects
undeclared data. Growing alias cycles are bounded; invalid UTF-8 errors name
their input. The real pinned LHA listing exposed the empty-file `******`
compression-ratio spelling; the parser accepts it only with zero lengths.

The 83 tracked transformation resources contain four 1.3 path lists, three
independent helper headers, 69 networking headers and seven exact unified
patches. The latter are derived from authenticated raw archive members and the
already measured patched reference. They reproduce all seven source outputs
without fuzzy matching at installation. The reference's original SFD backup
and canonical three-line fd2sfd banner are preserved, not omitted from hashes.
Generated `ndk-inputs-proof.json` records exact source/patch/origin hashes.

Manual calls to the actual extraction/transformation functions in the pinned
offline image reproduce all 1,372 prepared-file hashes while preserving the
unprepared baseline. Guards for `gcc`, `g++`, `cc`, `make`, `patch`, `rsync`,
`apt-get`, `curl`, `wget` and `git` remain unused. Only the three prebuilt
helpers and the required data receipts are mounted for this bounded proof.
The private generated `ndk-transform-sdk` has no prepared marker claiming a
final real lock.

The complete copied public command was also exercised from a read-only bundle
using the real archive/helpers and an explicitly synthetic integration lock.
Its four marker fields match that exact fixture lock and inventory; repeat
preparation is rejected without changing the marker. A second run with spaces
in bundle and SDK paths passes too. No repository, original builder tree or
source checkout is mounted into those standalone command probes. This is
command/marker/relocation proof, not completed real support-archive acceptance.

Actual helper closure is LHA, `sfdc` and `fd2sfd`. fd2pragma's two independent
headers are supplied as data; its unused executable was removed from the
candidate when establishing the preparation baseline. Re-running the exact
selector produces 422 files and 11 links, identical to the original candidate
except for that omission. Current recipe SHA-256:
`b1256b6f7ee7581a368d00f429fbc176b9574f24d273cb0201da906756640dcf`.
The initial 423-file receipt is preserved separately. No pin, new runtime
dependency, product capability or public command changed.

Task 1.2.8 verifies at 2026-11-18 16:28. All 14 preparation/member methods pass
under the prepared Python 3.10 and 3.12 interpreters. The complete copied
standalone command also passes under both versions with bundle/SDK paths
containing spaces, exact marker fields, unchanged repeat refusal and unused
forbidden-tool guards. The current script hash is recorded in both generated
transformation and synthetic-command receipts.

Task-level `make check` passes: 4,240 Rust tests, Python suites 29 + 103 + 28,
fuzz `cases=28 host=28 sanitizer=16 target=16 failures=0`.
All 75 support oracle methods remain present; future-command/mandatory-input
failures are explicitly retained. No final lock, real release marker, complete
source/notice assembly or actual-archive acceptance is claimed.
Owned extraction/compiler-guard/test/lint scratch is cleaned. Original
reference/source evidence, current private candidate, the marker-free
`ndk-transform-sdk` and the named receipts remain under
`build/distribution/reference-20261117` for subsequent producer proof.

### CPython wheel producers and native runtime proof in progress

Eight new wheel protocol methods and one source-integrity method ran red
before implementation. The wheel cases failed solely on the absent command;
the source case exposed accepted `.git/info/attributes`, which can alter
packaged source selection despite a clean worktree. That unlocked override is
now rejected without broadening source identity or executing filters.

The shared `support_build.py` keeps the existing source/image/isolation and
provenance boundary for both fixed-mode shell entries. Wheel mode appends only
the declared `--wheels <abi>` suffix. Actual manifest/ZIP/filename tags, bytes,
versions and source references are inspected before successful publication.
No protocol fixture is claimed to establish native runtime compatibility.

Current oracle: 84 methods — 41 contract, nine build, 20 preparation, eight
wheel and six mandatory real-artifact methods. Full discovery runs 78 local
methods and fails only on 16 still-unimplemented packaging assertions plus the
mandatory real-artifact setup error. No skips. All invocations of implemented
commands pass. All 38 focused wheel/build/preparation/new source-integrity
methods also pass under the prepared Python 3.10 and 3.12 interpreters.

| Current oracle | SHA-256 |
|---|---|
| `test_support_spec.py` | `13d3dd1ddeb5dcb3f6c8376694fb6c4ab36030e71ec2f9bd1afe17e0ad53ae16` |
| `support_wheels_spec.py` | `37d3adad9ac282e9bd7802fd4613a1a3f5990b9b1917f1a1529de9c62a722f96` |
| `support_build_spec.py` (unchanged) | `44ccd143a0a0f5bd152ee2b680da288e3db7792a4929f051a53ea8f972db6587` |
| `support_preparation_spec.py` (unchanged) | `8cf264e99bae44092fb58f724855e424ff7537b96845b5904c54d3eae1d406f9` |
| `support_artifact_spec.py` (unchanged) | `b9374b934e0882284adc5c103fe1b3b03012c2f34e06d04f627738ad4b569c45` |

Actual fixed-input builds ran for both ABIs in the inspected prepared image,
network disabled, non-root and source staging read-only. Each produced
amitools `0.8.1.dev274+gd0b9727da`, machine68k `0.4.1`, greenlet `3.5.6`,
PySDL2 `0.9.17` and pip `26.2.1`. The pinned original sources and eight
backend versions were checked. Actual built requirement markers, including
amitools' vamos extra, resolve within that complete set. PySDL2 is therefore
included by measured metadata rather than the old SDK package list.

Manual authoring calls use `produce()` without supplying an invented lock
digest. Outputs have no release `wheels.json`; their
`wheel-authoring/<abi>/manual-wheel-receipt.json` explicitly records a null
release lock and actual recipe/image/wheel hashes. Twelve original Python
archives, a raw amitools snapshot and original notices accompany the wheels.
Inline machine68k/Musashi and amitools notices and PySDL2's detailed terms
were explicitly retained and compared with the original source bytes.
This does not complete the conveyed-file compliance audit.

Both actual wheel sets were installed offline into fresh venvs created
without pip. The built pip wheel bootstrapped installation, followed by
no-index/wheel-only dependency resolution and `pip check`. The installed
Python set is exactly those five distributions, without producer backends.
Machine68k and greenlet import in that fresh venv; a greenlet call returns
42 and the installed vamos entry loads. Compiled strict-C99 hello and exit-7
programs execute with exact stdout/status under both nix13 and nix20.

Four actual native ELF files per ABI pass x86_64/readelf/loader inspection.
All GLIBC requirements are at most 2.14; no RPATH/RUNPATH or SONAME is present.
The actual NEEDED closure is libc, libstdc++, libgcc_s and the x86_64 loader,
all resolved in the prepared 22.04 image. Manual runtime receipts retain the
exact hashes/dependency observations and program results. Console probes do
not establish SDL graphics acceptance.

The named private wheel outputs and receipts are intentionally retained for
the real-lock/packaging tasks. These prepared-image authoring probes do not
replace clean 22.04/24.04 archive acceptance, complete notice/source assembly
or the final real locked producer execution.

Task 1.2.9 verifies at 2026-11-18 17:22. Task-level `make check` passes:
4,240 Rust tests, Python suites 29 + 103 + 28, fuzz
`cases=28 host=28 sanitizer=16 target=16 failures=0`. Python/shell syntax and
temporary hash-verified ShellCheck pass. Full 84-method support discovery
retains the 16 expected packaging assertion failures and required real-input
setup error; six actual-artifact methods are still unrun, never skipped.
Owned test/build/runtime-venv/lint scratch is cleaned. The named private
wheel outputs and manual receipts remain for final lock/producer/audit proof.

### Final-lock preparation and ownership-oracle hold

The final-lock task rehashed all sixteen original archives, resolved exact
Python source URLs by matching their actual hash/length, and measured the
immutable builder's actual tool/package versions. No placeholder or eligible
partial lock was written.

Fresh disposable Ubuntu 22.04 and 24.04 prerequisite images use pinned base
digests and the fixed `20261117T000000Z` Ubuntu snapshot. Only binary packages
are installed; runtime probes run as UID/GID 1000 with networking disabled.
Both images resolve all 38 actual selected SDK/wheel native files and launch
the required C compiler/preparation helpers. Interpreter/Perl module and
actual dpkg provider paths are recorded. These are prerequisite/loader
measurements, not the mandatory later support-archive acceptance.

The previously reviewed released-source baseline and original-plus-patch
representation for inherited libnix pipeoc code are now retained as tracked
data inputs under `inputs/source-forms/`. Exact baseline/result hashes and
zero-fuzz reconstruction match the earlier review. No compiler build-source
pin or byte changed. Source-package assembly remains unimplemented.

Real Ubuntu 24.04 measurement exposed AR-37 / DEF-564: `dpkg-query -S` exits
zero on the loader's `/lib64` alias with diversion commentary only. The actual
strictly resolved file is genuinely owned by libc6. The existing immutable
artifact oracle only resolves after a nonzero exit and parses commentary as
owners, so its unchanged assertion rejects the legitimate owner. The original
oracle hash remains
`b9374b934e0882284adc5c103fe1b3b03012c2f34e06d04f627738ad4b569c45`.
The independent challenger agrees that this is an oracle defect. No test was
changed; the bounded correction and controls require explicit user approval.

Generated `runtime-owner-compatibility-probe.json`, both runtime-image and
runtime-measurement receipts, source-form proof and acquisition/environment
measurements remain under the private reference directory. The runtime
environment recipe and original oracle bytes are retained there for resumed
proof. Affected tasks are blocked; no task was promoted beyond 13/91.

### Approved ownership correction and real-lock execution in progress

The user approved AR-37's narrow correction. Six independently authored
controls ran red on the absent actual-file ownership resolver before the
protected oracle changed. They then passed unchanged: canonical symlink owner,
diversion-only output, missing owner, wrong canonical owner without alias
search, owner records for a different file, and genuine multiple/architecture
qualified Debian owner records.

Only the approved owner-resolution/parsing block changed in artifact
acceptance. The existing expected-package assertion remains, as do all hash,
containment, version, SONAME, GLIBC and actual loader checks. The old oracle
bytes are retained at
`oracle-baselines/support_artifact_spec-before-ownership.py` beneath the private
reference directory, matching the original recorded hash.

The actual amended resolver and all six controls pass in both supported
Ubuntu environments. For each release, all 63 actual loaded-library dependency
owners across 38 SDK/wheel native files match the independently measured
provider cohort. The normal Ubuntu 24.04 loader alias is verified through its
genuine canonical libc6 file; no system layout or package identity was changed.
`runtime-<release>-ownership-reproof.json` retains each observed owner/path.

Current oracle: 90 methods — 41 contract, nine build, 20 preparation, eight
wheel, six ownership and six mandatory real-artifact methods. Full discovery
runs 84 local methods and remains expected red only for sixteen absent
packaging assertions plus mandatory real-artifact setup. No skips; all
implemented commands and ownership controls pass.

| Changed oracle | SHA-256 |
|---|---|
| `test_support_spec.py` (ownership-suite discovery only) | `3b1f7193ba93e6c5059e0f5136e2443a12408104e23de52141eb02fa354bcfbc` |
| `support_ownership_spec.py` | `61164967a5a3a42f3d0079a2dd7f9195281de3098e190bd34bfda10c1286ab5d` |
| `support_artifact_spec.py` (approved owner block only) | `8eed45bdb421f35f1c6ceee175c09d225b3c0ce667c45cf5c1e2ebbe9ab85929` |

The complete real support lock is now assembled from actual source/archive,
recipe/review, preparation and runtime measurements. It has 28 source records,
102 hashed inputs, both actual Ubuntu package cohorts, all measured builder
packages/backend versions and the 1,372-file prepared inventory. Original
authoring source inputs were preserved separately before staging this exact
input set. Component licence IDs summarise their recorded terms, not legal
clearance. The ordinary libamiga remains under the explicit inclusion directive.

Exact lock SHA-256:
`00e84d3617bbe4b0b9402282c2354cdc96160432524dbb4bb4a4f6304270cea9`.
Both public `validate-lock` and `verify-sources` pass against this real lock
and the complete actual staging tree. Actual public support/wheel producers
are now executing in the inspected prepared image; real standalone
preparation and task verification remain pending. No build success,
packaging/archive acceptance or publication is inferred from lock validation.

The first real locked wheel commands both passed with exact `00e84d…`
provenance. The C recipe stopped before compilation on its older Git-status
guard: Git 2.34.1 reports ` M fd2pragma.c` from cached checkout sizes, while
`git diff` is empty and the strict raw-blob verifier passes. This is the
parallel reference-recipe path of the already approved canonical-byte rule,
not a reason to accept transformed source bytes.

The reference preflight now checks staged/index and actual content differences
explicitly, with external diff/text conversion disabled, and separately lists
all extra files/directories without ignore exclusions. Its original link
checks and the public exact-blob verifier remain unchanged. Adversarial real
Git probes and revised-recipe lock/producer execution are required before
promoting the task. The initial real-lock bytes and successful wheel receipts
are preserved as superseded evidence rather than relabelled to a new digest.

The revised preflight passes on actual Git 2.34.1 with canonical source bytes.
Five additional real filesystem/index probes reject tracked edits, staged
changes, ignored extras, empty extra directories and changed executable mode.
`reference-source-guard-proof.json` records all six cases and the new recipe
SHA-256 `c2a1c94cb8304b9c53933df4f42e61e2c9bf03daa383c595c1386755d5f613a6`.
Shell syntax and temporary hash-verified ShellCheck pass.

The sole revised lock input is that recipe's hash. Current complete real lock:
`8529d78db6b2eea225dd5575c8d3f458c597e8d7c2cf5402b3474d9f6977a72e`.
Source pins, source blobs, image/environment inputs and package cohorts are
unchanged. Initial lock/assembly bytes are retained under `lock-baselines/`;
initial successful wheels and proof receipts remain explicitly bound to the
old digest. Revised public source/build/wheel and real preparation commands
are rerunning; no earlier receipt was rewritten to claim a new build.

The revised task-level `make check` passes at 2026-11-18 18:07:
4,240 Rust tests, Python suites 29 + 103 + 28, and fuzz
`cases=28 host=28 sanitizer=16 target=16 failures=0`.
All 90 support oracle methods remain present. Full discovery still fails only
on sixteen unbuilt packaging assertions and mandatory real-artifact setup.
This does not promote task 1.2.10: revised actual public producers and
standalone preparation are still running. Their owned scratch/logs remain
until the active commands finish; no successful real-archive acceptance is
claimed.

Task 1.2.10 verifies at 2026-11-18 19:07. Revised public lock/source commands,
the actual support build and both actual wheel producers pass against lock
`8529d78db6b2eea225dd5575c8d3f458c597e8d7c2cf5402b3474d9f6977a72e`.
Actual output trees match their exact provenance inventories: support has
422 regular files and 11 links; each wheel payload has 99 regular files.
Each wheel set has five actual, lock-bound wheel records. Named
`locked-<producer>-proof.json` receipts retain exact commands and identities.

The copied standalone preparation bundle uses that real lock and actual
newly built helper/baseline bytes. Its public command passes under the actual
Ubuntu 22.04 and 24.04 interpreters, with network off, paths containing spaces
and compiler/build/package/network guards untouched. Both 1,372-file overlays
and baseline trees match exactly; markers bind the real lock/archive/inventory
and repeats are refused without mutation. `locked-preparation-<release>-proof.json`
records those results. This supersedes the earlier synthetic-command proof,
not its preserved original evidence.

Task-level `make check` and all owned command/ownership cases pass as recorded
above. The sixteen future-packaging assertions and mandatory real-artifact
setup remain red; six actual-archive methods remain unrun, not skipped.
Complete conveyed-file/native audit, notices/source assembly and actual archive
acceptance have not passed. Owned invocation scratch/logs are cleaned; actual
locked outputs/bundles/prepared trees, both prerequisite images and named
reference/lock/oracle proof receipts remain for packaging and acceptance.

### Payload assembly and packaging verification in progress

Task 1.2.11 reran the sixteen unchanged packaging cases red before writing the
command: all failures identified only the absent public command; all previously
implemented cases passed. `package_support.py`, `support_audit.py` and
`support_native.py` now perform closed-schema/file/component-obligation checks,
actual native inspection and deterministic no-clobber tar publication.
The fixed cross-objdump is authenticated as a recorded host executable before
it is invoked to inspect HUNK archives and standalone target objects.

Two wheel-packaging cases preceded a necessary correction. The valid-lock
positive case passed, while a consistent file audit incorrectly accepted a
wheel manifest for another lock. Packaging now reuses actual wheel validation
and binds each package source to its audited origin. Both controls pass
unchanged. The original artifact oracle remains unchanged after the approved
ownership amendment.

Current oracle: 92 methods — 41 contract, nine build, 20 preparation, ten
wheel, six ownership and six mandatory real-artifact methods. All 86 local
methods pass. Whole-file discovery intentionally fails mandatory setup without
`CLORRAINE_SUPPORT_ACCEPTANCE`; six actual-archive methods are unrun, not skipped.
The wheel oracle now has SHA-256
`2804db5c81de37ffd7ad5e98ef7eca548371b78df0ce8f1986ddd83a12bf4468`;
the other five oracle hashes remain as recorded above.

All 86 local methods also pass under both prepared builder interpreters
(Python 3.10/3.12) with source code mounted read-only and networking disabled.
An earlier attempt used the intentionally Git-free runtime images and failed
fixture setup, not assertions; no compatibility success was claimed. Git was
not added to the user-runtime prerequisite list to satisfy test-only fixtures.

| Actual assembled asset | File/link records | Native records | Components |
|---|---:|---:|---:|
| SDK | 702 | 172 | 12 |
| cp310 wheels | 128 | 4 | 13 |
| cp312 wheels | 128 | 4 | 13 |
| Preparation | 128 | 2 | 6 |

The maintainer assembly API verifies complete actual source staging and
producer provenance before copying. Generated `reviews/file-origins.json`
retains original producer records, published hashes, install-producer origins
and copied-header source matches. All 141 ordinary newlib headers, 32 libnix
headers and three pthread headers match actual source bytes; `newlib.h` is the
explicit configure-generated exception. This technical tracing does not grant
new rights. The exact ordinary libamiga directive and its legal-review gap
remain unchanged.

The SDK snapshots exclude private Git metadata, amiga-gcc's prebuilt libraries
and the modified libnix pipe implementation; the latter's original-plus-patch
representation and support identification accompany the source. AROS conveys
only the selected pthread subtree. Wheel original archives and inline
amitools/machine68k/Musashi/PySDL2 notice-bearing files remain present.
Source-byte and original-notice delivery rechecks are still in progress.

The public packaging command produced all four real tar files. Two independent
public invocations per asset have identical bytes. Every member's hash/target
matches its audit; names are sorted, IDs/timestamps/names are normalised, modes
are restricted, extraction is link-safe and exact, and an overwrite attempt
fails without changing the existing archive.

| Asset | Bytes | SHA-256 |
|---|---:|---|
| SDK | 363694080 | `5166b8c3f5c071fc178c1455b9967c6444420ca4e5c931c486f00f09366857eb` |
| cp310 | 51568640 | `3e406accd63a6e3d65053751c677a0702dafd7e2f00cda6eb4883951dba067bf` |
| cp312 | 51558400 | `60bbd8457ac0948b577484afd1fcdad380f39c0390dc91854fb12112397f01eb` |
| Preparation | 2570240 | `f8c31ca12b4d2ff09c66b06849b1c961bf310a42f9020616313d0e50e8522217` |

The preparation metadata now binds the entire assembled 702-entry SDK
baseline, including source/notices/reviews, rather than the earlier raw
producer baseline. Actual extracted preparation passes under both clean
Ubuntu interpreters: complete baseline/1,372-file overlay and markers match,
repeat is refused unchanged, and real corrupt-archive/missing-helper controls
fail with unchanged baselines and no completion marker. Space paths work;
compiler/build/package/network guards remain unused.

Fifteen bounded additional probes reject malformed/duplicate/oversized/FIFO
audit input, path traversal, special files, missing native coverage and
wrong actual wheel-native architecture/dependencies. They produce no output
archive and no traceback. Real assemblies, archives, audits and named receipts
remain under `build/distribution/reference-20261117/packaging/` for acceptance.
Task `make check` is still running; task 1.2.11 is not yet promoted.

Task 1.2.11 verifies at 2026-11-18 20:18. Source-delivery rechecks now pass
for all four actual assets: each selected Git snapshot contains exactly the
intended actual source files/links, original source archives have the locked
hash, original canonical notices match, and source-form exclusions/retention
match the recorded reviews. The probe initially omitted legitimate directory
entries in the wheel producer's amitools snapshot; after fixing that probe's
directory handling, exact file/link comparisons pass. No product or protected
specification was changed to satisfy it.

Task `make check` passes: 4,240 Rust tests, Python suites 29 + 103 + 28 and
fuzz `cases=28 host=28 sanitizer=16 target=16 failures=0`. Python syntax,
Markdown links/table structure, JSON and locked-input hashes are checked.
Six actual-archive corpus/relocation methods remain mandatory work in the
next tasks; they are not represented by these narrower packaging/preparation
receipts. Original source/provenance/reference evidence remains intact.
Owned packaging scratch, repeated tar outputs, extracted test trees, negative
fixtures and logs are cleaned. The four named assembled payloads/audits,
candidate archives and their proof receipts are retained for task 1.2.12.

### Real clean-Ubuntu archive acceptance in progress

Task 1.2.12 rebuilt current `clc` in the pinned SDK and emitted the fixed five
cases for the three declared profiles (14 units; volatile-mmio has no host
unit). All five existing stdout goldens are retained as passive fixtures.
`acceptance-fixture-emission.json` records the emitter binary hash, revision,
source hashes, exact commands and all 19 fixture hashes. The ten Amiga units
contain exact full embedded runtime source bytes: base plus every suffix/P96
definition for hosted-amiga, with the existing suffix exclusions for 1.3.
No runtime shim or emitted C was edited.

Both six-method mandatory artifact suites are running through ordinary
unittest class selection on the actual previously prepared Ubuntu images.
Only individually bound candidate archives/audits, the lock, NDK input,
copied public auditor/tests and passive emitted fixtures are mounted.
The repository, original SDK, builder prefixes, compiler source, Docker socket
and non-loopback network are unavailable. Each suite checks actual host identity
and isolation before extraction and records its environment and commands.
Task `make check` runs separately. No acceptance result is inferred yet.

### Real archive failure and runtime wheel-content hold

Both actual suites completed six methods: four passed and two failed on each
release, without skips. Complete real preparation, corrupt-input/missing-helper
handling, standalone target-object auditing and incomplete/wrong-ABI/source-only
wheel rejection passed. Native/import and relocation scenarios failed before
their corpus execution because the shipped Greenlet test extensions import
`greenlet.tests`, whose initializer imports absent `psutil`.

The pinned original Greenlet `setup.py:171–190` explicitly labels these as
test extensions; `setup.py:198` includes packages discovered under `src`.
`src/greenlet/tests/__init__.py:17` imports psutil, and
`pyproject.toml:55–64` declares psutil/objgraph/setuptools under the optional
test extra. None is part of the selected runtime dependency closure.
The current recipe builds the unmodified upstream wheel; checking its runtime
metadata does not remove or validate shipped test-only modules. No dependency
or protected test has been changed.

This is AR-38 / DEF-565, pending an explicit wheel-content ruling and independent
challenge. The candidate narrow correction is to exclude only Greenlet's
test package/native test extensions from runtime wheels while retaining
runtime files, headers, metadata, original licences and full source. It must
not weaken the requirement that every shipped native extension is tested.
Any approved correction needs controls first, a new exact recipe/lock identity
and repeated affected producer/packaging/real-archive evidence. Existing bytes
and receipts remain historical evidence, not relabelled successes.

`acceptance/ubuntu-<release>-failed-proof.json` retains all 48 actual recorded
environment/command/output events from each run, the exact candidate records,
unchanged oracle hash and four-pass/two-failure result. The original oracle is
also retained at `oracle-baselines/support_artifact_spec-before-greenlet-decision.py`.
No corpus, relocated native-loader or complete archive success is claimed.
Task `make check` nevertheless passes: 4,240 Rust + 160 Python tests and fuzz
`cases=28 host=28 sanitizer=16 target=16 failures=0`; it does not override the
failed mandatory acceptance. Progress remains 15/91 verified.

Independent read-only challenger `ses_e0fdef5d2ffeV7IKRcALLI7m3C` confirms the
payload-selection mismatch. Upstream intentionally ships its optional tests;
the installer did not promise their dependency closure. Both actual wheels
have 31 `greenlet/tests/` members, including two test-native extensions.
The recommended correction is exact deterministic subtree exclusion with
regenerated RECORD, not an oracle amendment or new runtime dependency.
The runtime extension, headers, licences, remaining member bytes and complete
original source must be preserved. User approval and test-first controls are
still required. Owned failed-run scratch/logs are cleaned after retaining the
actual event evidence. Named candidate archives, copied auditor/tests, emitted
fixtures and failed proof receipts remain for reproduction and the ruling.

### Approved Greenlet runtime-only selection and renewed proof

The user selected “Exclude test-only files” for AR-38. Seven new independent
selection controls ran red on the absent operation before producer changes.
They pass unchanged after implementation: exact subtree exclusion, preservation
of every other member's bytes/ZIP metadata, complete RECORD hash/size coverage,
byte-deterministic output, and fail-unchanged controls for another distribution,
signed RECORD and absent runtime extension.

The fixed wheel recipe now selects only the pinned Greenlet distribution
before recording final wheel hashes. It drops exactly `greenlet/tests/`,
retains other member bytes/ZIP metadata and regenerates RECORD. A private
sibling is substituted only after successful writing; identity/runtime/signature
failures leave the original untouched. No generic filter or runtime installation
step was added. Original source archives, both licences and runtime dependency
versions are unchanged.

Actual selection of both pre-correction ABI wheel copies confirms removal of
exactly 31 test members per wheel. All surviving member bytes except RECORD
match, every RECORD hash/size is valid, repeated output bytes match, and the
only remaining Greenlet ELF is its real runtime extension.
`greenlet-selection-authoring-proof.json` records this lock-free authoring
probe; it is not relabelled as a release-bound producer result.

| Added/changed selection oracle | SHA-256 |
|---|---|
| `support_wheel_selection_spec.py` | `652a121ec6f28216fa27676fecccab6ce808e2ca1c1fd32f50d9807e4184a89c` |
| `test_support_spec.py` (selection-class discovery only) | `3698c874a97a0753304c449ed661c7738a92577fd7076b52076f3f1128341015` |

The artifact oracle remains exactly
`8eed45bdb421f35f1c6ceee175c09d225b3c0ce667c45cf5c1e2ebbe9ab85929`.
The total is now 99 methods: 93 local controls and six mandatory real-artifact
methods. Local/Python compatibility and the task gate are rerunning separately
from the real producer pipeline.

Current exact lock:
`0aa91466ec282293dcd4fab5befd89d4a1b3d862499fe1c95e460e3586749af1`.
It retains the same 28 source records and 102 inputs; only the wheel recipe
and its source/notice-review hashes changed. Recipe SHA-256:
`da7cdbaf47affe789af08fdf1c8201ae7b7e76ccc1784eb4734baa2ca3df3dc0`.
The old lock, changed input bytes, all successful old producer/preparation/
packaging artifacts and failed acceptance evidence are preserved under
`build/distribution/reference-20261117/pre-greenlet-selection/`.
No receipt was relabelled. All three renewed public producers are running;
new assembly, packaging, preparation and both real Ubuntu suites remain required.

All 93 local methods now pass on the host and both prepared Python versions,
including the seven unchanged selection controls. Full 99-method discovery
still fails only mandatory real-artifact setup without an acceptance input;
there are no skips or local assertion failures. Existing build-only packaging
tools supply the wheel recipe's metadata parser; no package was installed or
added to user-runtime prerequisites for these tests.
`greenlet-selection-protocol-proof.json` records both builder-cohort results.
While renewed exact-lock producers run, a separately labelled authoring-only
corpus probe uses selected wheel copies and the preserved prepared SDK; it
cannot establish the new archives' acceptance or satisfy the pending gate.

The separately labelled authoring corpus probe passes on both actual Ubuntu
releases: all 14 fixed strict-C/debug/warning-as-error cases compile and execute
with exact existing goldens using the selected Greenlet wheel copies and
preserved prepared SDK. Fresh offline venv installation and actual runtime
imports also pass. `greenlet-selection-corpus-authoring-<release>.json` records
commands/stdout with `release_lock: null`. This diagnostic establishes that the
remaining corpus is reachable after test-subtree selection; it is not the
new exact-lock archive/native/relocation acceptance.

The corrected recipe's task `make check` passes at 2026-11-18 20:51:
4,240 Rust tests, Python suites 29 + 103 + 28 and fuzz
`cases=28 host=28 sanitizer=16 target=16 failures=0`.
This is separate from the 93 local distribution methods and both diagnostic
corpus probes above. It does not promote task 1.2.12 while the renewed exact-lock
producer/packaging and mandatory real-archive acceptance chain remains pending.
Owned scratch remains necessary for those active builds and subsequent proof.

At 2026-11-18 21:52 all three renewed public producers pass against exact lock
`0aa91466ec282293dcd4fab5befd89d4a1b3d862499fe1c95e460e3586749af1`.
Public validation/source verification passes, and each actual resulting tree
matches its successful provenance inventory (433 support file/link records,
99 regular files in each wheel payload). No old provenance was relabelled.
Source-preserving assembly, accurate native audits and deterministic public
tar creation are now running. The next actual Ubuntu suites must consume those
new archives and retain their own complete command/output evidence.

At 2026-11-18 21:56 the renewed assembly/native audits and all four public
archives pass. SDK/preparation file counts remain 702/128 with 172/2 native
records. Each ABI wheel payload still has 128 file records but now exactly two
native records (machine68k and Greenlet runtime), not the two removed optional
Greenlet test extensions. Every surviving Greenlet member except RECORD matches
the previously verified wheel bytes. Original source archive/snapshot hashes
match the earlier independent source-delivery proof.

| New exact-lock archive | Bytes | SHA-256 |
|---|---:|---|
| SDK | 363704320 | `2d57b8f64d18e0c2e8631509aff38bf5f1e575265973f1fa7c901e2e8e3070d0` |
| cp310 | 51507200 | `5be467ddae166958636818a71f87c687a95429fcda7c9a370ce515ede9243601` |
| cp312 | 51496960 | `533d0f1b4da2efd6f1ef7f5aec356ea10b5f79999473d5fff0bd2611b389c6e8` |
| Preparation | 2580480 | `c0ae6bdcc2a4f5fdd7b1c182bc6ac5c0efbd619f375f91c899bafc79b7ad5384` |

Each new public output has byte-identical repeat output, normalised metadata,
exact file hashes/link targets and fail-unchanged overwrite refusal. No old
archive was replaced or its receipt relabelled. Both unchanged six-method
real Ubuntu suites are now running against these exact new bytes. The compiler
still matches the retained fixture-emission hash; no emitted C or frozen golden
was edited. Actual archive/native/corpus/relocation success remains pending.

### Actual moved-root GCC failure

Both renewed real suites finish all six methods. Five methods pass on each
release, including actual imports of all surviving wheel-native modules,
native dependency ownership/loading, complete preparation, standalone objects
and negative offline-wheel controls. All 14 original-location strict-C corpus
cases compile/run with exact goldens. After moving the assembled SDK/venv into
a new path containing spaces, the four host cases pass but all ten Amiga
compile subtests fail before cc1 with
`switch 'relocated' does not start with '-'`.

This is a compiler implementation failure, not permission to narrow the
relocation oracle. Current GCC's `find_file_spec_function` returns a raw
filesystem path (`gcc/gcc.cc:10809–10817`); Amiga SELF_SPEC inserts that result
as `-B %:find-file(...)` (`gcc/config/m68k/m68kamigaos.h:407–412`).
The specs parser sees whitespace-separated extra options and rejects them
(`gcc/gcc.cc:6004–6010`).

A separate read-only diagnostic mounts identical SDK bytes under plain and
space-containing prefixes. GCC can print the actual runtime-directory path in
both locations, but its `-###` self-spec processing succeeds only under the
plain prefix for both nix13/nix20. No compilation or source modification occurs.
`gcc-spaces-diagnostic-proof.json` retains exact queries/stdout/stderr and the
actual compiler/image hashes.

AR-39 / DEF-566 pauses affected tasks for a source-backed path-handling ruling.
The candidate narrow fix is a locked GCC specs-result quoting patch applied
only in the private build copy; original staged source pins/bytes, full source
delivery and the protected relocation assertion remain unchanged. No such
patch, recipe or lock change has been made. Independent challenge is pending.

The new `acceptance/ubuntu-<release>-relocation-failed-proof.json` receipts retain
all actual environment/command/output events (1,344 on 22.04; 1,240 on 24.04),
candidate identities and the unchanged artifact-oracle hash. Neither earlier
authoring corpus results nor five passing methods constitute relocation
acceptance. Progress remains 15/91; no commit or publication occurred.

The independent GCC challenge confirms the parser boundary and recommends
`quote_spec(xstrdup(file))` only in the find-file spec function. Existing helper
semantics cover active specs bytes without changing borrowed storage or empty
argument policy. Amiga include/`-B`/`-L` consumers and the nested Picolibc
parallel path were reviewed; plugin-only handling is not silently included.
A new exact patch, private-build application, source-delivery instructions and
renewed proof require explicit AR-39 approval. The original GCC checkout,
recipe, lock and protected tests remain unchanged for this discovery.
Current candidates and all original/new failed proof receipts are retained;
owned temporary builds, repeated tar files, venvs and logs are cleaned after
recording their actual evidence. No active job requires that scratch now.

### Approved GCC specs-path correction: test authoring

The user approved AR-39's bounded private-build patch and corresponding-source
updates. Five additional real controls are authored in
`tools/distribution/tests/support_gcc_relocation_spec.py`, selected through the
same existing acceptance manifest and ordinary unittest. They require genuine
audited SDK bytes; no compiler fixture or optional skip substitutes for them.
The original six-method artifact oracle and its discovery file are unchanged.

These controls cover both runtime `-B` branches under plain/spaced roots,
found `-isystem` paths, missing-path fallback containing spaces, attached `-L`
paths and literal active specs characters in the find-file result, and preservation of existing
empty/multiple-argument rejection. Core dumps are disabled for the deliberate
invalid-argument controls. A read-only specs-syntax prototype confirmed their
documented extension syntax and existing invalid-argument behaviour before
authoring the final assertions.

Both real baseline runs are executing before any GCC patch/recipe/lock change.
The support surface is now 104 methods across separate declared groups:
93 local methods, six unchanged real-archive methods and these five additional
real GCC controls. All real groups remain mandatory in the phase proof;
missing acceptance inputs fail rather than skip.

Independent authoring review found that the new link control initially placed
active characters in the entire SDK root, exercising unrelated static LTO
plugin/wrapper interpolation as well as the approved find-file boundary.
Before implementation or freezing these new assertions, the control was
corrected to retain a spaced SDK root, assert the actual found `-L` directory
and require an exact unique attached `-L` argument for a missing filename
containing spaces/percent/pipe/backslash. This preserves every approved
find-file control without adding a universal special-character SDK-prefix
requirement. Original failed authoring bytes/events remain preserved.
The independent static LTO-prefix behaviour is not claimed fixed.
Both corrected red runs precede any GCC patch, recipe or lock change.

The final focused oracle is
`40f1bce5a4dcd1fb1ed48ec4f01c7de556b1de8dc9b2a0fe02491d37fc1e9a2d`.
Each corrected real baseline ran five methods: four methods fail as intended
(five failure records because both spaced-root runtime subtests fail), while
the malformed/empty-argument compatibility method passes. There are no setup
errors or skips. Corrected bytes/events and the initial broader authoring
version are preserved separately under the old candidate evidence.

The approved GCC patch and private-build integration are now authored.
Original `gcc/gcc.cc` remains
`5e926ce80a05eef4d2094ac6f64e9e76a590a42b27b6e266fd029be065c59213`.
The exact patched result is
`be312720a5feae41361762dec7d453c0914ee0bd93118baa48fb08d21aa5fc48`;
patch SHA-256 is
`1a5c4044b0518f42f44e18854122bdb0d02da44e91ece9fd304acef7c4ddad6b`.
Only the find-file spec return and its durable explanatory comment change.
An actual private copy applies with zero offset/fuzz; repeat and context
mismatch are rejected. Original staged bytes remain untouched.
`gcc-quoting-patch-proof.json` records this source-transform proof, not a
successful rebuilt compiler.

The reference recipe applies the patch only after private source copying,
checking the entire original file hash before and result hash after GNU patch.
No user installation command applies compiler patches. Source assembly now
conveys every locked `compiler-patches/` input alongside the full original GCC
snapshot, modification/date notice, original licences and rebuild instructions.
The existing audit schema/roles and original artifact oracle remain unchanged.
Shell/Python syntax and temporary hash-verified ShellCheck pass.

The new exact lock is
`aca1a0ec2dc4c3fa8e98a3b5bdb17ba0ec6d750fd11e082e4ca2b88e63588566`.
It retains 28 original source identities and now has 104 inputs: the patch and
its notice/instructions are new; reference-recipe and source-review hashes are
updated. Preceding producers, preparation bundle, archives, native/original
corpus successes and moved-root/focused-control failures are preserved under
`build/distribution/reference-20261117/pre-gcc-quoting/`.
All renewed public producers are running. Patched-driver controls, source
reconstruction from the delivered patch, real archive preparation, native
closure and complete relocation must still pass before task promotion.

At 2026-11-18 22:45 the 93 existing local support methods pass after the private
patch/source-delivery changes. Whole-file discovery still reports only the
required real-artifact setup error without an acceptance manifest, never a
skip. The five newly declared GCC controls remain separate mandatory real
context tests and are not counted as local passes.
Task `make check` also passes: 4,240 Rust + 160 Python tests and fuzz
`cases=28 host=28 sanitizer=16 target=16 failures=0`. Current locked inputs
and the original artifact-oracle hash recheck unchanged. Actual public
producers are still running; source reconstruction from the delivered patch
and combined eleven-method real Ubuntu proof are prepared to follow, not
assumed successful.

At 2026-11-18 23:42 public validation/source checking and all three rebuilt
producers pass against exact lock
`aca1a0ec2dc4c3fa8e98a3b5bdb17ba0ec6d750fd11e082e4ca2b88e63588566`.
Fresh assembly/native/source audits and deterministic archives are running.
After those succeed, the actual conveyed original GCC source and patch must
reconstruct the documented modified hash, then both Ubuntu suites run all
six original archive methods plus five unchanged final focused GCC methods.
Producer success does not establish moved-root behaviour by itself.

At 2026-11-18 23:47 all four renewed assemblies/native audits and deterministic
public archives pass. SDK has 704 file/link records and 172 native records;
each ABI wheel payload and preparation bundle has 130 file records, with two
native records each. Original source archive/snapshot hashes are preserved;
new compiler-patch inputs are explicitly conveyed.

| Patched-driver exact-lock archive | Bytes | SHA-256 |
|---|---:|---|
| SDK | 363714560 | `00fe9bdf4f78e46a04ac27e2bf2eeb130357b81c1be1f1a94307ef028070649e` |
| cp310 | 51517440 | `99a9aa1eb8e3926646aef240005e223902d196a2b797c5409e9744599615ea47` |
| cp312 | 51507200 | `1778a32e76800edacbad2c416588f0806733d7af6eb57d9e86209d312112d226` |
| Preparation | 2580480 | `17df63ab79d383b3333cb0b858a6ba14496a51e0072b657d84fa75158607b733` |

The actual shipped original GCC snapshot plus shipped exact patch reconstructs
`be312720a5feae41361762dec7d453c0914ee0bd93118baa48fb08d21aa5fc48`.
All four assets contain the exact locked patch and modification/rebuild notice.
`packaging/gcc-source-reconstruction-proof.json` records this actual delivered
source proof. Neither staged GCC bytes nor the protected six-method oracle
changed.

Both real Ubuntu suites are now executing eleven methods: the original six
artifact methods and all five frozen focused GCC specs controls. Fresh
runtime work roots preserve the original isolation requirements. No combined
success, relocation success or task promotion is inferred yet.

Task 1.2.12 verifies at 2026-11-18 23:51. All eleven real methods pass on each
actual Ubuntu cohort: six unchanged artifact methods and five final focused
GCC controls, with no skips. Both new candidate suites use exact lock
`aca1a0ec2dc4c3fa8e98a3b5bdb17ba0ec6d750fd11e082e4ca2b88e63588566`.
Actual environment/isolation, native loading/ownership, wheel-only installs,
complete preparation/notices, negative archive/wheel cases and target objects
pass. The full fixed strict-C corpus runs before and after real spaced-root
relocation, with the original assembled root absent and fresh preparation
performed after moving.

`support-acceptance-summary.json` independently checks the retained events:
28 successful strict compile/run pairs per release (14 original, 14 relocated)
with exact frozen goldens, and ten focused GCC queries (seven successful,
three deliberately invalid). The two environment records per release match
actual non-root Ubuntu identity and loopback-only networking. Copied test/auditor
hashes match current source; original staged GCC and the six-method artifact
oracle remain unchanged. The emitter binary and all 19 fixture hashes still
match their recorded identities.

Task 1.2.12's `make check` remains 4,240 Rust + 160 Python tests, fuzz 28 with
zero failures. Owned GCC/assembly/real-test scratch, repeated tar files,
source-transform copies, temporary lint tools and logs are now cleaned.
Named current/prior producer outputs, archives, audits, source reconstruction,
all actual red/green event receipts and invocation evidence remain for review.
Task 1.2.13 uses a separate owned scratch root to re-run the 93 current local
methods on both prepared interpreters and its own task gate before consolidation.

Task 1.2.13 verifies at 2026-11-19 00:07. All 93 current local methods pass on
the host and both prepared Python interpreters. Together with the six original
artifact and five focused GCC methods on each actual Ubuntu cohort, all 104
distinct support specification methods pass in their required contexts.
No licensed/real-context method was substituted by a local fixture or skipped.

Its task `make check` passes: 4,240 Rust tests, Python suites 29 + 103 + 28,
and fuzz `cases=28 host=28 sanitizer=16 target=16 failures=0`.
Current lock/104 input hashes, all four actual archive hashes, staged original
GCC, protected oracle, emitter and passive fixture identities recheck cleanly.
The evidence consolidation again verifies 28 exact strict corpus compile/run
pairs and ten focused GCC queries per Ubuntu release. Python syntax and
documentation links pass.

`support-evidence-task-check-proof.json` retains the exact task metrics,
verification-log identities and consolidation invocation. The separate
`support-local-methods-proof.json` and `support-acceptance-summary.json` retain
local-cohort commands and actual-artifact event checks. Owned task scratch and
logs are cleaned. Named current/prior producer artifacts, archives and proof
receipts remain under `build/distribution/reference-20261117/` for phase review.
Progress is 17/91; hardening, test integration and the full private-display
phase gate/independent review remain before Phase 2.

### Focused support implementation hardening

Task 1.3.1 adds only `test_support_impl.py`, with 17 implementation tests.
These exercise exact lock-byte hashes versus semantic JSON equality, nested
duplicate/encoding rejection and non-mutating validation; real filesystem
file/link inventories, equal-length replacements, cyclic/dangling/escaping
aliases and unsafe names/permissions; exact multi-hunk preparation patches,
failure without partial writes, offset/count/repeat rejection and archive
link-graph resolution. Fixtures contain no licensed NDK data.

The focused tests pass initially. Distinguishing controls remove one check
at a time only in owned scratch copies (lock hash, inventory containment and
patch count validation); each selected test must fail without fixture errors.
The host and both prepared Python interpreters then run 110 local methods:
93 unchanged specification tests plus 17 implementation tests. Their task
`make check` is running. No production module, locked input, source pin or
protected specification assertion changes in this task.

Task 1.3.1 verifies at 2026-11-19 00:16. All 110 local methods pass on the
host, CPython 3.10 and CPython 3.12. All three scratch-only mutation controls
fail through the intended assertion, with no setup errors. Task `make check`
passes: 4,240 Rust tests, Python suites 29 + 103 + 28 and fuzz
`cases=28 host=28 sanitizer=16 target=16 failures=0`.
Syntax and documentation self-checks pass.
`support-hardening-proof.json` retains cohort commands, mutation identities,
test/log hashes, gate metrics and the invocation recipe. Owned scratch and
logs are cleaned; current actual-archive proof remains unchanged. Progress:
18/91; Makefile integration and full private-display phase gate/review remain.

### Local check integration

Task 1.3.2 writes three Makefile wiring controls before integration. Two fail
red on the absent local target/check invocation; the third confirms both real
acceptance groups already fail setup without their required manifest. No
specification test, skip condition or acceptance manifest changes.

`make check` now invokes `distribution-check`, which explicitly selects all
93 local specification methods, 17 implementation methods and the three
wiring controls through existing unittest. The target prints its narrower
local-only boundary. All 113 methods pass with the original assertions.
Its prerequisites are host Python/Git and the existing wheel-builder
`packaging` dependency; no package is installed or added to the native-user
runtime closure. The pinned SDK does not supply that development dependency,
so the target follows setup tests' existing host execution pattern.

The six real artifact and five focused GCC methods remain mandatory separate
isolated-context acceptance on each Ubuntu baseline. Missing inputs remain
errors in both groups, never successful skips. Local success does not replace
their real proof or its source/network/platform/old-prefix assertions.
The README records both commands and this boundary.

Because this task changes shared Makefile infrastructure, its verification is
the full `make verify`, now running with the established authenticated private
Xvfb procedure. No desktop `.env` or X access policy is changed. Task 1.3.2 is
not promoted on its green local tests alone; the full gate and subsequent
independent phase review remain pending.

The first private-display setup attempt stopped before `make verify`:
concurrent `xauth nlist | xauth nmerge` processes raced on the same authority
file lock. The owned cookie/scratch were removed. The documented procedure
now completes `nlist` before starting `nmerge`, retaining the same secret,
wildcard-family entry, owner-only cookie permissions and disabled TCP.
No authentication was weakened and no user display configuration changed.
The full gate has been restarted using that sequential procedure.

Task 1.3.2 verifies at 2026-11-19 00:42. Full `make verify` passes every leg
on authenticated private Xvfb, including the twelve UBSan suites, SDK/emulator
matrix, differential corpus and runtime faults. Workspace tests remain 4,240;
including the UBSan reruns, the log records 4,574 passing Rust test instances.
Python suites are 29 + 113 + 103 + 28; fuzz remains
`cases=28 host=28 sanitizer=16 target=16 failures=0`.

All 113 local support methods run unconditionally through `make check`.
Two wiring controls ran red before the Makefile change and now pass; both
real acceptance classes still error without inputs. The prior eleven-method
isolated proof on each supported Ubuntu release is retained independently.
`support-check-integration-proof.json` records exact gate metrics, source/log
identities and the private-display invocation without its secret. The private
server stopped, cookie removed and owned scratch/logs cleaned. Progress:
19/91; independent correctness/security/Amiga and test-integrity review is the
remaining Phase 1 gate.

### Independent Phase 1 review dispatched

At 2026-11-19 00:45 four read-only reviews start against exact baseline
`c584fb9aaf5512c6eaea0bac28741d6172553e50` and complete worktree snapshot
`07350719c87c88b031631d01889864d6a4dfbacc` (137 files). The snapshot includes
untracked phase files without touching the real Git index. The helper's diff
operation could not decode an original external-source input as UTF-8; its
snapshot operation plus raw-byte `git diff --binary` preserves the exact same
tree comparison, without rewriting source encodings.

| Review | Session | Authority and boundary |
|---|---|---|
| Correctness/maintainability/standards | `ses_e0ef2a8cfffex8l70EVjh30Lbe` | Generic fallback with complete project correctness brief |
| Supply-chain security | `ses_e0ef2a8a9ffemsReJr48P739Du` | Generic fallback with complete security packet and source/archive/isolation checklist |
| Amiga platform | `ses_e0ef2a8a0ffeek1YEZxqun7UUK` | Available project specialist; both runtime/profile and wrapper paths |
| Test integrity/omitted parallel paths | `ses_e0ef2a89effewkmBaWbcJDHrVU` | Separate generic auditor; red/green, immutable oracle and actual-proof integrity |

Every packet includes strict scope, original goal/minimum design, approved
runtime boundaries, exact diff, deliverable/checklist trace and current full/
real verification. Reviewers may run scoped probes in their owned scratch,
but may not change project files, install dependencies, rerun emulators or
publish media. Critical/major findings require the user's ruling. Progress
remains 19/91; Phase 1 and Phase 2 are not promoted while reviews are pending.

The Amiga review completes at 2026-11-19 00:50 with **no findings** and no
open question. Independent wrapper probes pass 20/20 on head; all fifteen
override cases fail on baseline. The reviewer runs 37 scoped preparation/
implementation methods and shell syntax checks, verifies 27 standalone
HUNK/m68k objects, both runtime archives and the exact ordinary libamiga
selection, and inspects both eleven-method Ubuntu receipts plus full-gate
evidence. No producers or emulators are rerun. Registry/profile/ABI/emission
paths remain unchanged; reviewed production files match the snapshot.
The specialist's owned scratch is removed. Three other reviews remain pending;
the task stays unverified.

The correctness reviewer completes with no critical/major finding. Independent
`make distribution-check` passes all 113 methods; the private-display Rust suite
passes all eight methods in the supported SDK container. Public real-lock
validation, snapshot/production identities, four archive/audit hashes and both
actual Ubuntu receipts match. Every completed task deliverable is covered;
the review task remains partial until the parallel reviews and disposition.

RV-001 is a minor documentation finding: `tools/distribution/README.md:9–15`
labels an older GCC relocation failure as the current proof state, while later
paragraphs and current receipts establish success. The minimum correction is
to label that paragraph historical or state the current passing result.
It is recorded as deferred/report-only under the minor-finding policy; no
production or documentation fix is silently applied during parallel review.
The reviewer's initial direct-host Rust launch could not resolve container
`/work` paths; the correct container invocation passes. Owned reviewer scratch
is cleaned. Supply-chain security and independent integrity reviews remain
pending; progress stays 19/91.

The separate test-integrity review completes with **no findings**. It
independently passes all 113 local methods and verifies missing-input errors,
all four archive/member/audit identities, producer inventories, the current
lock/104 inputs, protected oracles, emitter and fixtures. Both actual receipts
contain the six artifact/five GCC methods and 28 successful strict corpus
pairs per cohort, including moved-root execution and fresh preparation.
It also independently re-emits all fourteen C units (accounting only for the
expected checkout-path spelling) and reconstructs the exact patched GCC source
from delivered original plus patch. Recorded red events distinguish both
runtime corrections; compatibility and parallel paths remain accounted for.
Early chronology and original real-wrapper observations rely on preserved
evidence, explicitly not rerun. No missing in-scope deliverable is identified.
Owned reviewer scratch is removed; the security review is the sole pending
independent result.

The initial security session ends in a service-level safety rejection without
a completed review. This is neither a clean result nor a waived gate.
At 2026-11-19 00:53, replacement session `ses_e0eeb3406ffe33Xepz6qwDRH5u`
receives the same exact diff and required defensive source/file/archive/
subprocess/isolation/provenance/cleanup checklist, with static inspection and
existing local tests/evidence only. External systems, offensive simulations,
downloaded-artifact execution and project modifications are explicitly excluded.
The scope and acceptance gate are unchanged. Task 1.3.3 remains unverified
until an independent security result is available.

The replacement reviewer first reports a readiness gap: its generic brief
requires an explicit active-profile list. The executor supplies the missing
field as an empty configured built-in list (no web/auth/financial/tenant/MCP
profile applies), with the approved Phase 1 supply-chain checklist explicitly
designated as the complete phase-specific review policy. This follows the
plan's existing supply-chain lens and strict missing-configuration defaults;
it neither invents configuration nor removes a required review.

The user asks for wording that clearly describes ordinary software work.
The same review is clarified as local software-packaging safety and reliability:
checksum/metadata validation, paths/archive links, isolated builds, existing-file
preservation, cleanup, provenance consistency and private test-display data.
Only tone changes; exact diff, full checklist, independence, read-only scope and
critical/major finding gate are preserved. The same session resumes, not a
duplicate or a waived review.

The completed defensive supply-chain review reports two **minor** findings,
no critical/major findings and no inconsistency in the retained real receipts.
All 130 non-plan files match reviewed head `07350719…`; all 104 staged input
hashes, four archive/member/audit identities and both actual Ubuntu receipts
match. Independent `make distribution-check` passes all 113 methods. Source,
metadata, filesystem links, both producers/ABIs/cohorts, isolation, preparation,
native audits, publication and wrapper authentication paths are reviewed.

| Finding | Evidence and consequence | Disposition |
|---|---|---|
| SA-001 (minor) | `package_support.py:175–181` validates only part of producer provenance; schema/image/image-ID/environment-recipe/argv are not compared, and boolean false compares equal to exit zero. Contradictory or incomplete receipts can pass assembly. Actual retained receipts are consistent. | Deferred/report-only; minimum correction is complete shape/type and locked build/mode-argument validation |
| SA-002 (minor) | `README.md:376` supplies the generated cookie as an xauth command argument, briefly visible under ordinary process-list access despite 0600 file permissions. | Deferred/report-only; minimum correction is cookie input through standard input; historical evidence must remain unchanged |

Together with RV-001, these are recorded in `05-findings.md`. Under the
minor-finding policy they do not require a blocking ruling and are not silently
fixed or described as resolved. All four independent reviews are complete;
no blocking finding remains. The final reviewer changes no files and cleans
its scratch. The interrupted original reviewer is asked to remove only its
abandoned owned scratch before the parent removes the shared review packet.
No legal clearance or publication permission is inferred.

Task 1.3.3 and Phase 1 verify at 2026-11-19 01:05. All four independent
reviews complete with zero critical/major findings; the three minor findings
remain explicitly deferred/report-only. Before closure, all 130 non-plan files
are compared byte-for-byte with the exact reviewed tree and still match.
No product fix or scope change occurred during review, so the passing full
private-display gate and current actual Ubuntu receipts remain applicable.
`phase1-review-proof.json` retains reviewer sessions/verdicts, reviewed tree/
diff identities, observed commands, minor dispositions and the complete packet.
All reviewer and parent scratch is removed, including the interrupted first
reviewer's owned files. Named current/prior support artifacts and receipts
remain for later phases. Progress: 20/91 (Phase 1: 20/20).
Native driver/local core packaging, installer, editor, release and WSL2/
Remote-WSL acceptance remain incomplete. No commit, tag, push, workflow dispatch
or publication occurred.
