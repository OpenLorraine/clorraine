# Ambiguity Register: v1.2.0 native Linux delivery

> **Status**: ✅ RUNTIME RULINGS — Phase4 bounded corrections and AR-47/48 approved
> **Last Updated**: 2026-11-21
> **CodeOps Artifact Schema**: 1

## Planning scope contract

| Boundary | Current scope |
|---|---|
| Planning target | Clorraine v1.2.0 Linux-first delivery: prebuilt compiler and Amiga toolchain, shell installation and explicit updates, VS Code editing/building, and execution in the user's FS-UAE. |
| Context artifacts | Compiler driver and its specification tests; `docker/versions.env` and toolchain build; `tools/setup/`; `tools/vscode/`; release and site documentation; the sibling QtIFW installer scaffold; upstream toolchain build rules. Reading these does not authorize changing them. |
| Modification set during discovery | This register and the explicitly approved native-mode amendment to `plans/clorraine-driver/01-requirements.md` (AR-29). The requested full plan documents and preflight follow after the material decisions close. Implementation, other upstream requirement/specification changes, tags, and publication need their applicable authorization. |

## Current decisions

The user selected Linux first, then replied **“confirmed”** to the three
recommendations: Ubuntu 22.04-compatible Linux x86_64 binaries tested on
22.04/24.04; A1200/Kickstart 3.1 with a directory drive first; and a shell
installer using GitHub Releases and `~/.clorraine`, with explicit updates.
This settles those choices, not the open interface and distribution details
below. The release version and prebuilt/editor/media requirements carry forward
from the earlier discussion.

| # | Category | Decision / remaining gap | Options or confirmed boundary | User decision | Status |
|---|---|---|---|---|---|
| 12 | Scope / compatibility | First delivery version and Linux baseline | v1.2.0; Linux x86_64; Ubuntu 22.04-compatible binaries tested on 22.04 and 24.04. Linux and Windows ARM are excluded from this delivery; native Windows and macOS are not the first delivery. | “Ok! Let go with linux first” and “confirmed”; version 1.2.0 confirmed earlier. | ✅ Resolved |
| 13 | Scope / dependencies | Prebuilt delivery model | Prebuild `clc`, `clorraine`, `clorraine-lsp`, and the supporting Amiga toolchain. Use GitHub Actions and durable GitHub Releases downloads. Reuse toolchain artifacts between compiler releases; users do not compile installation dependencies. | Earlier prebuild/download requirement, followed by confirmation of the Linux delivery approach. | ✅ Resolved |
| 14 | Integration | Initial native emulator setup | A1200 / Kickstart 3.1 with a staged directory drive. The compiler's existing target profiles do not become different CPU profiles through this choice. | “confirmed” to the A1200/KS3.1 recommendation. | ✅ Resolved |
| 15 | Behavioral / data & state | Installer type, root, and update trigger | Shell installer; `~/.clorraine`; downloads from GitHub Releases; updates explicitly requested by rerunning the installer. Detailed arguments, activation and failure rules remain AR-20. | “confirmed” to the shell/GitHub Releases/root/explicit-update recommendation. | ✅ Resolved |
| 16 | Scope / media | External applications and licensed media | FS-UAE is user-installed. Kickstart and any needed Workbench files are user-provided. No application-store distribution. NDK delivery is a separate unresolved dependency question, AR-18. | Confirmed earlier product and media boundaries; retained through Linux-first selection. | ✅ Resolved |
| 17 | Technical / ownership | Which repository owns the shell installer and native release jobs? | Compiler repository: native installer, packaging, one reusable toolchain-prebuild workflow and one compiler/package workflow. Existing QtIFW repository is not part of this implementation. | “Compiler repo (Recommended)” in the repository-ownership question. | ✅ Resolved |
| 18 | Dependencies / security & compliance | Exact toolchain inputs, payload and runtime dependencies | Use the current tested SDK's source cohort as the starting lock, resolving complete revisions and archive hashes before builds; build against Ubuntu 22.04. Publish reusable Amiga/tool helpers and vamos wheels, with required notices/corresponding source. CPython 3.10/3.12 match Ubuntu 22.04/24.04. AR-27/28 govern system prerequisites and NDK acquisition. Native library closure, redistribution audit and clean relocation are mandatory first-phase proofs, not asserted capabilities. | Accepted host-prerequisite and NDK choices; “Approve rules (Recommended)” for exact pins, immutable assets and source/notices. Detailed proof boundary submitted for final register confirmation. | ✅ Resolved |
| 19 | Behavioral / compatibility | Native driver and emulator contract | Preserve host/vamos and existing Docker behavior. Add native installed SDK discovery and `--sdk-root`, `--fs-uae`, `--kickstart` with matching environment variables. Native `--emulator` launches A1200/KS3.1, waits, and returns FS-UAE's exit status, not guest status. Native FS-UAE `--capture` is an explicit unsupported usage error; existing host/vamos/Docker capture continues. Detailed precedence below. | “Approve contract (Recommended)” in the native-Run question; requirement amendment separately approved in AR-29. | ✅ Resolved |
| 20 | Data & state / edge cases | Installer contract and updates | `install.sh` plus Python standard-library manifest/archive handling; `--root`, `--release`, latest published distribution by default. Verify downloads, use prebuilt wheels only, prepare NDK, smoke-test, then atomically activate. Failed/interrupted updates preserve the working installation; concurrent installs fail clearly. Keep previous versions and settings. Print PATH/VSIX setup commands; do not edit shell startup files or install the extension automatically. Initial trust: HTTPS plus published hashes. | “Approve contract (Recommended)” in the installer-behaviour question. | ✅ Resolved |
| 21 | Integration / UX & presentation | VS Code Build/Run contract | Commands: Build, Run (host/vamos), Run in FS-UAE. Use the saved active `.clo` entry file and selected profile; require workspace trust. Find the installed tools without requiring a terminal-launched editor; configurable driver/SDK/emulator/ROM paths. Download the VSIX during installation and print its install command. WSL is AR-23. | Accepted native-Run and installer contracts. Detailed settings/selection rules below submitted for final register confirmation. | ✅ Resolved |
| 22 | Technical / publication | Release automation and publication contract | The two workflows are manually dispatched and create draft releases. Immutable core/toolchain/vamos assets carry exact pins, hashes and required notices/source. A new distribution revision may reuse compiler binaries with newer support assets. Public promotion requires a later explicit user go after all evidence passes; activate the distribution manifest only when all referenced assets are public and verified. | “Approve rules (Recommended)” in the release-and-verification question. | ✅ Resolved |
| 23 | Scope / integration | What is supported under WSL2 in this delivery? | Linux installation, compilation, and VS Code Remote-WSL. Document manual use of output in Windows FS-UAE; automatic Windows-emulator launch is not a first-release requirement. | “Build/editor (Recommended)” in the WSL2-scope question. | ✅ Resolved |
| 24 | Non-functional / acceptance | Verification and first-release success criteria | `make check` for task verification after measuring its baseline; `make verify` at phase/release gates; `make vscode-check` and `make site-check` for their surfaces. Clean Ubuntu 22.04/24.04 package tests; native emulator proof and existing licensed gates locally on private displays; actual WSL2 build/editor evidence on the user's Windows machine. Missing evidence blocks the release. | “Approve rules (Recommended)” including the real WSL2 acceptance requirement. | ✅ Resolved |
| 25 | Naming / delivery documentation | Plan/roadmap identity and release documentation scope | Keep `plans/v1.2.0-release-installer/`; add standalone PLAN-45 when the plan is ready; update compiler/extension version records and v1.2.0 release/installation/site documentation, preserving historical release records. No RD selected or invented. | “Approve rules (Recommended)” in the release-and-verification question. | ✅ Resolved |
| 26 | Scope / dependencies | Are native host builds and vamos runs ready after installation? | Include host-compiler provisioning and a packaged vamos dependency set, in addition to the Amiga toolchain and user-installed FS-UAE. Exact provisioning and runtime packaging remain AR-18/20. | “Include host and vamos” in the installed-run-modes question; selected instead of the narrower Amiga/FS-UAE option. | ✅ Resolved |
| 27 | Dependencies / UX & presentation | How are host GCC and the Python runtime provisioned? | Use Ubuntu's prebuilt GCC and Python/venv packages. Show the apt prerequisite command for missing packages; the user runs it and reruns the installer. Install pinned, prebuilt vamos wheels under `~/.clorraine`; never silently invoke sudo or compile wheels locally. | “Ubuntu packages (Recommended)” in the host-prerequisites question. | ✅ Resolved |
| 28 | Dependencies / security & compliance | NDK acquisition and preparation | Download the exact upstream NDK archive, verify its hash, then prepare headers locally with shipped prebuilt helper tools. Preserve notices and verify the prepared result. No user-side compilation of installation dependencies. | “Fetch upstream (Recommended)” in the NDK-preparation question. | ✅ Resolved |
| 29 | Compatibility / modification authority | May the old container-only driver requirement gain a native mode? | Authorise a narrow amendment to `plans/clorraine-driver/01-requirements.md` for native SDK discovery and FS-UAE execution. Preserve the existing Docker path and its specification tests; exact new options remain AR-19. | “Extend native mode (Recommended)” in the driver-compatibility question. | ✅ Resolved |

## Detailed contract for complete-register confirmation

These details make the approved contracts executable. The user selected
**“Confirm complete register”** after reviewing AR-12–29 and this section.
This is the final normal-mode gate confirmation; no delegated design authority
was used.

### SDK and run selection (AR-19, AR-29)

- SDK selection: `--sdk-root PATH`, then `CLORRAINE_SDK`, then the installed
  executable's adjacent `sdk/amiga`, then the existing `/opt/amiga` SDK path.
  An explicit invalid SDK never silently falls back. The `/opt/amiga` fallback
  retains the existing container handoff; an explicitly selected or adjacent
  installed SDK uses native execution.
- `--fs-uae` overrides `CLORRAINE_FS_UAE`, otherwise search `fs-uae` on PATH.
  `--kickstart` overrides `CLORRAINE_KICKSTART`; there is no implicit ROM scan,
  download, or user-configuration rewrite.
- The native emulator recipe is A1200/KS3.1 for either existing Amiga profile.
  This does not change their emitted CPU/runtime flags or claim an A500 boot.
  Existing host, emit, freestanding and vamos paths keep their current semantics.
- Missing executables/media are clear failures before launch; malformed or
  inapplicable options and unsupported native emulator capture exit 2.
  Existing diagnostic, compilation, and missing-tool errors keep exit 1.
  Native emulator process termination without an exit code is reported as
  failure; successful launch alone is not successful program execution.
- Stage a private, per-run directory separate from the build directory that
  the driver resets. Copy the binary to a fixed guest filename and create the
  startup sequence/config without interpolating source names into guest shell
  code. Wait for FS-UAE before cleaning that run's files. A second build must
  not delete a running emulator's mounted files.
- This native runner stages the single executable and startup files, matching
  the existing runner's payload boundary. It does not mount arbitrary source
  directories or install a Workbench environment.

### Installation and artifact state (AR-18, AR-20, AR-22)

- Keep immutable component versions and a versioned assembled installation;
  activate them through one root-owned `current` symlink. Stable `bin` access
  follows `current`. Resolve the selected SDK/runner paths once per invocation
  so an update cannot mix component versions in a running build.
- Use JSON metadata with an explicit schema version, distribution identity,
  core/extension versions, exact support identities, Linux architecture and
  compatibility data, URLs, lengths, and SHA-256 values. Reject unknown schema,
  missing/duplicate fields, invalid identities, incompatible assets, unsafe
  archive entries, and failed checks. Never evaluate manifest content as shell.
- Distribution tags start at `v1.2.0`; packaging-only revisions use
  `v1.2.0-rN` without rebuilding unchanged compiler binaries. Support release
  tags are independently versioned and never designated the repository's
  latest distribution. `--release` selects an exact distribution tag.
- The shell bootstrap is release-specific and pins its Python helper by hash.
  The helper uses system Python's standard library. No separate signature
  verifier, update service, mirrored hosting, or rollback command is added.
- User-owned installation only. Reject an unrelated nonempty target; only
  replace installer-owned state. Do not automatically delete prior component
  versions. Use a process lock and same-filesystem activation; a stale temporary
  directory must not count as a completed component.
- Generate PATH setup guidance and retain the downloaded VSIX under the
  installation. Preserve existing user settings. Failure to install optional
  user-provided FS-UAE does not block compiler/host/vamos installation; an
  attempted FS-UAE run without it does fail.
- Prepare NDK content from the exact upstream archive and audited preparation
  inputs with prebuilt helpers. Do not ship a modified NDK header tree merely
  because the local SDK contains one. Retain appropriate upstream notices;
  source/licence audit failures stop publication rather than silently widening
  redistribution or dropping required compiler features.
- For vamos, select the supplied CPython 3.10 or 3.12 wheel set matching the
  interpreter, install only from that verified local wheel set, and execute
  through the environment's Python without depending on a build-machine
  shebang. Unsupported interpreter versions fail with actionable guidance.
  Ubuntu upgrades between interpreter versions require installer rerun; the
  installer builds a fresh matching environment rather than mutating a live one.
- Host prerequisites include GCC, C development headers, Python/venv, HTTPS
  download/CA tools and the runtime libraries declared by the support package.
  The clean-machine proof validates this closed dependency list and rejects
  undeclared dynamic dependencies. Never fall back to compiling a missing wheel.

### Editor and verification (AR-21, AR-24, AR-25)

- Reuse `clorraine.profile` (existing default `hosted-amiga`) and the existing
  LSP path override. Add user/machine-level driver, SDK, FS-UAE and Kickstart path
  settings; source workspaces cannot silently supply executable overrides.
- Save the active `.clo` document before launching; cancellation/save failure
  stops the command. Use that document's containing workspace folder as working
  directory, or its parent directory when no workspace is open. Multi-file CLI
  use remains available; the editor action uses one entry file and its imports.
- Use VS Code process tasks with argument arrays and visible terminal output.
  Reject untrusted-workspace builds/runs and wrong-profile FS-UAE commands.
  Resolve the default installation at `~/.clorraine/bin` even when desktop
  VS Code did not inherit shell PATH; custom roots use the path settings.
- Gate packaged installations on real host output and both Amiga profiles via
  vamos, plus native A1200/KS3.1 launch evidence. Check fresh install, same-release
  rerun, support-only revision, corrupt/partial downloads, concurrent updates,
  unsafe paths/archives, relocation and missing prerequisites.
- First-phase feasibility failures are explicit stops: do not substitute
  toolchains, change the approved Linux baseline, or remove supported modes
  without a new user ruling. No duration, memory, or artifact-size claim is
  assumed from the earlier discussion.
- Use existing Rust integration, Python unittest and Node test runners. No
  new generic test framework is needed. Full verification remains mandatory for
  shared build/test infrastructure changes, phase exit and release.
- Create the requested plan and preflight only; task checkboxes start
  incomplete. Future implementation and public publication are separate actions.

## Evidence and planning corrections

### Runtime execution decisions

| # | Category | Decision / remaining gap | Options or confirmed boundary | User decision | Status |
|---|---|---|---|---|---|
| 30 | Integration / data contract (runtime) | Support specifications need concrete lock/evidence formats and executable script interfaces | A: complete the existing support document with versioned JSON fixtures and explicit validation, source-verification, offline-build and payload-audit command contracts. B: review the detailed contract before authorising tests. Both retain the three planned scripts and existing unittest runner. | “Approve amendment (Recommended)” — define exact JSON fields, arguments, staged paths, offline boundary and success/failure outputs, then resume tests. | ✅ Resolved |
| 31 | Source identity / compatibility (runtime) | LHA's original tested revision is absent from SDK provenance; date-based reconstruction is not proof | A: recover original builder checkout/object evidence and keep the source lock blocked. B: explicitly select historical commit `08486d2da2ca77a6adf83194fd67400c8d86e7c7` as a new candidate LHA pin, not a recovered identity; preserve all known pins and retain source/licence/NDK proof gates. Independent recommendation: B with this bounded exception. | “Approve new pin (Recommended)” — exact LHA-only candidate; preserve other pins and mandatory licence/source/NDK proofs. | ✅ Resolved |
| 32 | Build environment / executable contract (runtime) | AR-30 requires an already prepared registry-digest image; no suitable Ubuntu image is identified, while a normal local Docker build produces an image configuration ID | A: provide a real compliant prepared-image registry digest and retain the contract. B: pin the Ubuntu base digest and hashed environment recipe/inputs, explicitly prepare locally, execute by immutable local image ID and record it in provenance; amend affected specifications narrowly, without registry/service/workflow additions. | “Approve correction (Recommended)” — base/recipe pinning, local preparation, actual image ID, narrow test updates and producer ordering; no registry or extra workflow. | ✅ Resolved |
| 33 | Source / redistribution evidence (runtime) | The required prebuilt `libamiga.a` has an exact identity but no established archive-specific origin/terms/corresponding-source chain | Include only the exact pinned ordinary archive. Retain factual provenance, hashes and notices without asserting a licence grant or matching source that has not been established. The user owns this archive's legal review; other component checks and the separate publication approval remain unchanged. | “copy the file, include it. I will deal with the legalities myself.” | ✅ Resolved (inclusion directive; not licence clearance) |
| 34 | Native audit contract / producer ordering (runtime) | The real reference has required standalone Amiga HUNK objects, but the audit enum permits only ELF/archive records; final lock is scheduled before its hashed preparation/payload producers exist | Add `amiga-object` for standalone m68k HUNK objects, with null SONAME and empty loader/GLIBC fields, verified against actual object contents; preserve `ar` for target archives. Author the narrow positive/mismatch specifications first. Split reference proof from final-lock completion and place the latter after the required recipe/preparation producers; keep all real locked-command and acceptance gates after lock validation. Same source cohort, product scope and public commands; no placeholder hashes, new dependencies or publication. | “Approve both (Recommended)” — accurate object-format tests/contract and producer-first lock completion, 91 tasks; no scope/pin/dependency/publication change. | ✅ Resolved |
| 35 | Package-name grammar (runtime) | Generic `Id` excludes `+`, but real build/runtime packages include `g++` and mandatory `libstdc++6`; acceptance invokes dpkg using the recorded package names | Introduce unqualified Debian `PackageName` (`[a-z0-9][a-z0-9+.-]+`) only for build/runtime package keys and `package:` provider suffixes. Preserve generic source/helper/tool IDs, reject unsafe or qualified names, and author positive/negative grammar tests before validation code. No new dependency, alias layer, pin, command or publication. | “Approve package names (Recommended)” — actual unqualified Debian names only in package fields; source IDs remain strict and unsafe names are rejected. | ✅ Resolved |
| 36 | Source identity / Git checkout bytes (runtime) | Pinned fd2pragma enables Git `ident`: a clean checkout expands `$Id$` to the blob ID, so actual bytes differ from the stored blob | A: explicitly cover only exact deterministic ident expansion, with locked-attribute proof, no external filters or unlocked attributes, and positive/negative tests first. B: canonicalise staged bytes to the existing blob and repeat affected reference proof without changing the pin. Neither permits generic transformed-byte acceptance, source fetching, new dependencies or publication. | “Canonical bytes (Recommended)” — retain strict raw-blob verification; preserve original evidence; canonicalise only the proven ident difference in owned staging, add tests first and repeat affected build proof. | ✅ Resolved |
| 37 | Test integrity / Ubuntu package ownership (runtime) | On standard Ubuntu 24.04, `dpkg-query -S` for the loaded `/lib64` loader alias returns success with diversion commentary only. The unchanged artifact oracle treats commentary as package names and never queries the actual resolved file, which is genuinely owned by `libc6`. | Narrow oracle correction: query the strictly resolved loaded file, accept only genuine Debian owner records, retain any merged-/usr fallback only for the same resolved file and no genuine canonical owner. Add positive/negative controls before changing logic. Keep hashes, containment, package/version/SONAME/GLIBC and actual loader checks unchanged. Rewriting system files, inventing providers or skipping ownership are not viable. | “Approve narrow correction” — preserve the original oracle, add positive/negative controls first, correct actual-file ownership resolution and recheck both Ubuntu releases. No pin, dependency, commit or publication change. | ✅ Resolved |
| 38 | Runtime wheel content / dependency boundary (runtime) | Both real archive suites fail while importing Greenlet's shipped test extensions: its test package imports absent `psutil`. The original source declares this under the optional test extra, not the vamos runtime closure. | Deterministically exclude exactly `greenlet/tests/` from produced runtime wheels, regenerate RECORD and preserve all other member bytes, metadata/licences and full original source. Keep every shipped-native assertion unchanged; add exclusion/preservation/RECORD/determinism controls first. Preserve old lock/artifacts and repeat all affected real lock/producer/packaging/Ubuntu proof. Independent challenge concurs. | “Exclude test-only files” — approve the narrow wheel selection, test-first preservation and RECORD proof, then regenerated affected lock-bound artifacts and both Ubuntu acceptance runs. No new dependencies or protected-test changes. | ✅ Resolved |
| 39 | Upstream compiler path handling (runtime) | The corrected wheels pass actual native imports/closure and all original-location corpus runs on both Ubuntu releases. After moving the SDK into a path with spaces, GCC splits its internally generated runtime search path and rejects stray tokens; all ten moved-root Amiga compilations fail. | A locked patch makes only `find_file_spec_function` return `quote_spec(xstrdup(file))`, applied only to the private copied build tree, preserving the source pin/staged bytes and protected relocation test. Add focused red/green path/specs controls, ship original source plus exact patch/notices/build instructions, and repeat affected exact-lock producer/archive/Ubuntu proof. Independent challenge concurs. No wrapper, space-path ban or new dependency. | “Approve GCC fix (Recommended)” — authorise the bounded quoting patch, test-first controls, corresponding-source packaging and renewed proof. No commits or publication. | ✅ Resolved |
| 40 | Installed-layout / executable interface (runtime) | Native specification tests cannot construct independently valid SDK/install fixtures: required installation metadata paths/schema, SDK/vamos pairing checks and exact companion Python invocation are unspecified. Local core packaging likewise has no callable command contract. | A: complete the existing installation/native-driver/local-producer documents with exact data and command interfaces plus one miniature valid fixture, then author tests before implementation. B: present the complete proposed contract for review before authorising its application. Neither adds a dependency, framework, workflow or product mode. | “Approve amendment (Recommended)” — define the concrete interfaces in the existing plan documents and resume independent specification tests. No new dependency, scope, commit or publication. | ✅ Resolved |
| 41 | Native ROM path representation (runtime) | FS-UAE 3.2.35 strips trailing spaces from option values; raw INI also truncates whitespace-prefixed semicolons. Literal CLI options handle the required ordinary punctuation, but cannot directly represent a valid ROM filename ending in a space. | A: for this otherwise-unrepresentable ROM path only, copy its selected bytes into a fixed private per-run media filename outside the guest drive; keep it until the child stops, then clean it. B: explicitly reject this filename case before launch with rename/path guidance and document the bounded limitation. Both keep ordinary spaced/metacharacter paths and control-byte rejection. | “Private ROM copy (Recommended)” — bounded temporary local copy only for this filename case, original preserved, cleanup after child exit, no packaging or upload. Add independent tests before implementation; existing expectations unchanged. | ✅ Resolved |
| 42 | Core package / upstream entry-point scope (runtime) | Real current binaries build on Ubuntu 22.04, but `clorraine-lsp --version` enters stdio LSP and returns 1 with closed input. The approved core contract requires all three version queries; the LSP entry is outside Phase 2's modification set. | A: permit only an exact single-argument `--version` entry returning `clorraine-lsp <current core version>` plus newline/status 0; preserve all other protocol startup. Add independent red/green process tests and retain existing core/LSP oracles. B: revise the core producer/acceptance contract to query LSP initialize/serverInfo instead, with independently amended protocol-fixture controls; no new public flag, but larger packaging/test machinery. | “Add version flag (Recommended)” — exact single-argument fast path plus independent red/green tests, normal protocol unchanged. Expand only to `compiler/src/lsp_main.rs` and new `compiler/tests/native_lsp_version_spec_test.rs`. | ✅ Resolved |

| 43 | Frozen core oracle / snapshot correction (runtime) | RV-C201's approved private executable snapshot conflicts with `test_core_package_spec.py:72`, which pins `/proc/self/exe` to the original mutable input path. Silently editing that independent oracle or adding a redundant original-path query to satisfy it is not allowed. | Permit an independent amendment only to the queried-executable identity assertion and its test-only observation helper: require a private snapshot distinct from the original, exact supplied-byte identity and the same archived bytes; preserve tool name, exact --version argv, no PATH fallback, rejection and all other expectations. Otherwise keep the snapshot correction blocked. | User selects “Approve amendment (Recommended)”: independent author changes only the conflicting identity assertion and necessary test-only byte observation, then verifies red before implementation. | ✅ Resolved |

| 44 | Integration / data contract (runtime) | Independent Phase 3 tests lack exact manifest nested fields, a callable release-producer interface and the binding for permitted test-only transport | A: complete these three bounded contracts in the existing installation/release/testing documents, including one coherent miniature manifest/payload fixture, then resume independent authoring before implementation. B: present the complete proposed contract for user review before applying it. Neither fixes private state filenames/function signatures, adds a public installer option or changes the approved product design. | “Approve amendment (Recommended)” — bounded existing-plan data/command/test-transport amendment, then independent tests before implementation. | ✅ Resolved |
| 45 | Verification / execution ordering (runtime) | Automatic setup discovery runs the public-shell oracles before their assembly producer exists; early implementation tasks cannot pass the mandated task gate independently of later task3.2.9 | Implement tasks3.2.1–3.2.9 in their listed order as one integration cohort, immediately recording each implemented task as unverified `[~]`. Keep all nine unverified until unchanged `make check` and all three declared local suites pass; then promote each separately. Preserve actual-context task3.2.10, full phase gate/reviews, immutable oracles and commit/publication boundaries. Otherwise leave implementation paused for a different approved schedule. | “Approve cohort (Recommended)” — implement in order, keep all nine unverified until the unchanged complete task gate passes. | ✅ Resolved |
| 46 | Verification / execution ordering (runtime) | Phase4's first manifest implementation triggers full verification, but frozen editor/source suites cannot pass until discovery, source selection, dispatch and actions in tasks4.2.2–4.2.5 exist; the normal first-unverified-task loop cannot advance | Only tasks4.2.1–4.2.5 form one ordered implementation cohort; immediately mark each implemented member `[~]`, resume an interrupted cohort at its first unimplemented member after integrity checks, and promote none until unchanged task/editor/private full gates pass. Actual desktop4.2.6, hardening/reviews, immutable oracles and commit/publication gates stay separate. | “I do approve” — explicit approval of the presented five-member ordered group and its retained gates/restart rules; continue ordinary in-scope choices without trivial questions. | ✅ Resolved — DEF-580 |
| 47 | Selection policy / Technical (complexity escalation, runtime) | RV-C401: a blanket nonlocal head veto omits compiler-accepted qualified module paths; blindly removing it breaks frozen genuine-field controls, and longer ambiguous chains need a deliberate selection policy | A: preserve field-first minimal selection for established genuine projections, including longer chains; approve only bounded request-local parsed declaration/type/member metadata, following imports/merged modules, while retaining longest module matching otherwise. No second parser/type checker/compiler rule change; stop if that boundary cannot suffice. B: explicitly amend feature selection/oracles to compiler module-first inventory semantics; larger behavioral/oracle change, not an automatic bug fix. | “i approve” — direct approval of the single presented bounded-helper packet and field-first policy, including at most one helper plus existing metadata plumbing; independent regressions before fixes, stop on broader machinery. | ✅ Resolved — verified and re-reviewed clean |
| 48 | Editor task data / filename policy (runtime) | SA-401: VS Code interprets `${...}` inside ProcessExecution data after validation, allowing editor-command invocation or path rewriting | Recommended bounded correction: explicitly reject the `${` introducer across executable, every argument and CWD before submission, with rename/path guidance and new regressions; preserve other literal punctuation. Dollar/backslash/quoting escapes are independently disproved; no public literal-disable option is verified. Otherwise keep held. No shell, launcher wrapper, protocol or dependency addition. | “i approve” — explicit approval of the presented `${` restriction across all final task inputs; other punctuation remains supported and rename/path guidance is required. | ✅ Resolved — verified and re-reviewed clean |

### AR-47 bounded helper approval packet — approved 2026-11-21 11:10

| Required evidence | Approved boundary |
|---|---|
| Original goal | Complete saved-source selection for native editor Build/Run without changing compiler language rules |
| Extra system or support code | At most one request-local Rust member/type metadata helper plus existing metadata plumbing |
| Why it may be needed | A declaration's existence alone cannot distinguish a genuine full member chain from a qualified module path |
| Evidence | `source_references.rs:49–55` blanket veto; `symbols.rs:75–94` lacks member/type structure; frozen genuine-field controls in `editor_sources_spec_test.rs:145–168` |
| Smallest solution that still works | Member-aware classification from parsed declarations/types/imports/merged modules; keep lexical locals and longest module matching otherwise |
| Alternative policy | Compiler module-first inventory selection; requires explicit feature/oracle amendment and can select extra modules |
| Extra cost | One focused helper at most, existing metadata plumbing, independently authored regressions and navigation/selection verification; no dependency/service/parser/general type checker |
| Independent verdict | Simplify — challenge recommends bounded request-local member-aware classification and stopping before general type-checking machinery |
| Direct user decision | “i approve” — specifically approves the presented three-correction package, bounded helper/field-first policy and `${` restriction |

The verdict wording above summarises the retained independent challenge in
`phase4-review/qualified-policy-challenge.md`; direct user approval is recorded
separately in the decision row.
If that bounded predicate cannot cover the approved policy, stop again.
CA-401's request-local root change/save guard is also approved. The guard may
require conservative retries; it does not promise atomic filesystem snapshots.
Independent regressions, task/editor/private full gates and one applicable
fix-diff re-review were completed: focused editor160/Rust123/shape10/symbol4,
private full4,796 Rust/all legs, editor170+freshpackaged1zero skips, then all
four re-review roles returned no findings. The helper stayed within this packet.
No oracle change, earlier-phase review, commit or publication authority was added.

### AR-46 editor implementation ordering stop — 2026-11-21 03:58

**User ruling recorded 2026-11-21 07:48:** “I do approve.” The user approves
the presented five-member group and repeats the direction to avoid trivial
questions. The exact order, immediate unverified marks, interrupted-group
resume rule and unchanged task/editor/private full gates below are authorised.
Actual desktop proof, hardening/reviews, immutable oracles and commit/publication
boundaries remain separate. DEF-580 is resolved. The proposal/stop evidence
below is retained as history; no automatic promotion or auto-design is inferred.

The complete specification-first checkpoint is verified at57/91. All nine
independent test/support files are frozen. Fresh135 cases give12 passing
controls and123 expected new assertions; old Rust57/editor10 pass unchanged.
Production is still exactly the Phase4 baseline.

| Evidence | Consequence |
|---|---|
| `99-execution-plan.md:26–44` | Resume the first `[~]`; task verification includes `make check`, editor checks and full verification for shared manifest/configuration changes. |
| `Makefile:60–68` | `check` runs all Rust suites, including the new request expectations. It currently stops only on40 expected source-selection assertions. |
| `Makefile:94–104` | Editor verification runs all Node tests before packaging. It currently stops only on83 expected action/discovery assertions. |
| `Makefile:223–246` | Full verification starts with `check`; a manifest-only change cannot pass it while the later request implementation is absent. |
| Session4.2 | The five listed concerns are manifest, discovery, source closure, protocol dispatch and actions; no individual early concern satisfies all frozen expectations. |

The smallest recommended correction is a **proposed ordered implementation
cohort4.2.1–4.2.5**, not a gate waiver or permission already inferred from
“continue”. AR-45 is expressly limited to Phase3's nine tasks.

If the user approves, retain all task IDs, order, target paths and concerns.
Record each implemented member immediately as `[~]` with its timestamp and
source/oracle identities. No member becomes `[x]` until unchanged `make check`,
all declared editor/source suites, `make vscode-check` and full `make verify`
on the authenticated private display pass over the complete cohort. Promote
each member separately with concern-specific evidence and documentation checks.

**Proposed interrupted-cohort rule:** Before resuming, recheck the approved
cohort membership, recorded implemented source identities, all frozen oracles
and earlier accepted bytes. Preserve implemented members as `[~]`; resume the
first not-yet-implemented member in the listed order rather than restarting
the normal first-`[~]` loop. Unexpected failures or source drift require
investigation; do not overwrite unrelated work. Once all five are implemented,
repair required failures and repeat gates before promotion. This exception
ends when the five members verify and applies to no other task.

Real installed-VSIX Linux desktop proof in4.2.6 remains separate and unverified
until observed. All4.3 hardening, full phase gate/independent reviews and actual
Remote-WSL/final acceptance obligations remain mandatory. No new support layer,
public test API, selector, framework, dependency, version bump, commit or
publication is proposed.

Trade-off: a larger unverified working diff before the first green checkpoint.
Immediate truthful marks, bounded membership and explicit restart rules reduce
visibility risk, not correctness risk. Reordering alone does not remove
mandatory sibling failures. Narrowing tests, treating expected implementation
red as verified or hiding all work under4.2.1 is rejected.

Read-only scheduling check `ses_e04035959ffeI3Lbeq8EpoC2gJ` independently checks
the same artifacts and concurs that this is the smallest supported correction
and needs fresh normal-mode approval. It adds the explicit restart and
documentation-check boundaries above. This is not a Phase3 re-review and grants
no authority. All completed test/check scratch is clean; only named receipts
remain. Tasks4.2.1–4.2.5 and PLAN-45 wait on DEF-580.

### AR-45 implementation verification ordering — 2026-11-19 19:17

The specification-first gate has verified40/91 tasks and frozen all23 new
oracle files. All six production entry/modules remain absent. Before starting
implementation, the parent identifies a task-order/gate dependency:

| Evidence | Consequence |
|---|---|
| `Makefile:77–78` | `setup-check` discovers every local `test_*.py`, including both new native suites |
| `tools/setup/tests/native_manifest_producer_fixture.py:54–58,92–104` | Process cases deliberately require successful public assembly, preventing an absent producer from satisfying negative assertions |
| `tools/distribution/tests/bootstrap_package_fixtures.py:93–99` | All bootstrap cases require the same valid assembly control |
| `99-execution-plan.md:33–39`, Session3.2 | Every task requires `make check` plus its declared local specs, while the assembler is task3.2.9 |

Implementing only the shared parser could make its nine methods green, but
the other shell/producer cases would still keep the required task command red.
Implementing the assembler first does not solve this: valid assembly requires
the installer modules and shell. Calling the early task verified despite the
known sibling failures would contradict the task gate.

Recommend a bounded **ordered implementation cohort** for tasks3.2.1–3.2.9.
Keep the same task IDs/count, concerns and target paths. Implement in order
and immediately mark each `[~]` with its timestamp; no task becomes `[x]`
until the unchanged task command and all three local suites are green against
the complete cohort. Promote tasks separately only with evidence for each
concern. Task3.2.10's actual bootstrap/Ubuntu/ABI/negative proof and the full
phase acceptance/review gates remain separate and mandatory.

The trade-off is a larger unverified working diff before the first green
checkpoint; explicit `[~]` states preserve crash-safe visibility. No test is
disabled, reclassified, weakened or omitted, and no new selector, public
test API, framework or dependency is introduced. Adding lower-level public
interfaces solely to obtain independent early green tasks is not the smallest
approved design. Skipping discovery or treating expected product red as a
verified implementation task is rejected.

This changes execution checkpoint ordering, not product scope or test
authority, so obtain an explicit normal-mode ruling before implementation.
DEF-575 tracks the dependency; the Session3.2 tasks are visibly blocked.
All prior author/parent scratch, logs and owned children have been cleaned;
named non-media receipts remain intentionally retained. No commit,
publication or phase promotion has occurred.

**User ruling recorded 2026-11-19 20:50:** The user selects
**“Approve cohort (Recommended)”** to the explicit ordered-integration-group
question. Implement tasks3.2.1–3.2.9 in order, immediately recording each `[~]`,
and promote none until unchanged `make check` plus all three local suites pass.
Task3.2.10's real-cohort/bootstrap/negative checks and full phase/review gates
are not removed or merged into local verification. No commit/publication or
new public test surface is authorised. DEF-575 is resolved.

### AR-44 installer/bootstrap authoring-readiness stop — 2026-11-19 15:57

Independent implementation-blind author `ses_e0bb271d4ffeTLIjBoBhoO7L5O`
finds tasks 3.1.1–3.1.3 cannot yet produce independently valid fixtures. The
author read no forbidden implementation, wrote no tests/code/docs/scratch
and claimed no verification. Phase 3's baseline is
`1ecded4acc2c3196b670ef6edf2c1856c677dd2a`; only startup planning records
have changed. Accepted Phases 1–2 remain intact.

| Missing observable interface | Grounding | Minimum amendment |
|---|---|---|
| Strict valid distribution metadata | `03-02-installation.md:48–68` defines required concepts but requires unknown-field rejection | Exact nested keys/types/enums, asset kinds/roots and identity/inventory relationships; one complete valid manifest with matching miniature payload metadata |
| Local release producer | `03-04-editor-and-release.md:158–161` names the producer's purpose but not its invocation | Exact command arguments for approved core/support/VSIX/metadata inputs, output filenames/layout and result conventions |
| Exact-byte test transport | `03-04-editor-and-release.md:253–257` permits a maintainer-only adapter; `07-testing-strategy.md:159–165` permits transport faults | Exact binding to the shipped shell/helper, release-lookup responses and asset bytes, including faults; no metadata rewrite, hash bypass, source import or new public option |

Recommend completing only those already approved interfaces in the owning
documents, then returning to independent specification authoring and red proof.
This is an executable-contract clarification, not approval for a new layer,
framework, service, workflow, dependency or mode. Private function signatures,
lock filenames, candidate naming and the final receipt schema are **not**
required: black-box tests can observe complete/current state, runnable
preserved versions, sentinels and process results. Preserve assembly readiness
versus smoke-tested transaction completion.

Alternative B lets the user inspect the complete concrete contract before it
is applied. Its benefit is direct control over data and command details; its
cost is an extra review handoff. Guessing these interfaces in independent
tests, deriving the oracle from production code or replacing packaged-bootstrap
proof with imported unit tests are not viable.

Proposed amendment set: this register, `03-02-installation.md`,
`03-04-editor-and-release.md`, `07-testing-strategy.md`, execution checklist
and roadmap. No source lock, accepted artifact, protected test, version,
compiler/driver implementation, commit or publication changes. The strongest
counterargument is that a concrete contract fixes details that could otherwise
remain flexible; limit it to observable data/command/transport boundaries.
Confidence: high; independent author and parent document check agree on the
three missing interfaces. DEF-574 tracks the pending ruling.
Tasks 3.1.1–3.1.4 are visibly blocked; dependent implementation remains unstarted.

**User ruling recorded 2026-11-19 16:14:** The user selects
**“Approve amendment (Recommended)”** to completing exactly the three missing
contracts and resuming independent tests. Apply the bounded owning-document
amendment, then independently author and run red before production work.
DEF-574's ruling dependency is resolved; authoring readiness still requires
the amended contracts. No framework/dependency/public-option, commit or
publication is authorised.

**Positive-fixture clarification — 2026-11-19 16:25:** The returning author
confirms the original three interfaces are resolved, but asks where positive
ST-12 runs because the host is Ubuntu 26.04/Python 3.14 and local failure
fixtures cannot claim success. The existing testing strategy already requires
candidate installer proof on both supported Ubuntu cohorts. Within the
approved coherent-fixture/test-binding amendment, its exact actual-context
fields, mandatory unittest invocation and accepted candidate input sources
are now specified. Missing context errors rather than skips; supported
cohorts run without probe/prerequisite/preparation/wheel/smoke overrides.

Read-only probes reconfirm the existing Ubuntu22/Python3.10 and
Ubuntu24/Python3.12 image identities and host C tools. Neither earlier image
includes curl: install only the already declared prerequisites in disposable
acceptance environments and record their new immutable image IDs before
actual proof. No existing image is relabelled as having passed new installer
prerequisites. Exact setup-command line prefixes permit black-box quote/
usability assertions without fixing private transaction state.
This identifies existing mandatory proof, not a new platform, successful
fixture substitute, weakened test, production test mode or additional approval.
No tests or production code have yet been written.

**State-negative fixture completion — 2026-11-19 17:20:** Before task3.1.2
authoring, the independent author confirms actual success/update contexts are
specified and requests two remaining external bindings: targeted child failure/
atomic-rename barriers and a disposable foreign-UID root. Complete the already
approved negative-fixture contracts, without changing product behavior:
the copied test runner may observe/fail one matched external subprocess and
barrier the actual `current` rename; no installer callbacks/private state names
are introduced. Successful acceptance runs have no such fault overrides.

Use a dedicated Docker tmpfs owned by UID/GID65534 with an ordinary non-root
test process and no host ownership changes for the foreign-root case.
An actual disposable-container probe confirms UID1000 can create/remove its
own child under that foreign-owned writable mount; no host paths, privileged
flags, root container process or media are involved. The separately invoked
negative actual-context group must still prove the installer rejects it.
This supplies the permitted test-environment/process bindings for the existing
mandatory cases; it does not waive a case, add a production test hook or enlarge
the public interface. The ten earlier oracle files remain frozen.

### AR-43 snapshot oracle correction stop — 2026-11-19

The user approved all three bounded major corrections, including private
snapshots for version validation and archiving. Reading the existing core
oracle then identifies a more specific conflict: its C fixture records the
actual `/proc/self/exe`, and one assertion requires the original supplied
path. A private copy cannot honestly satisfy that assertion.
The proposed amendment changes only that path-identity expectation and the
test-only observation needed to verify exact executed bytes. It keeps the
same supplied-byte, tool-name, literal single-argument, no-PATH and archive
requirements. An independent author must update it from the approved contract,
not production code, and show the old producer fails the new snapshot check.
No production correction has started. Additive regression authoring may
continue; task 2.3.4 cannot verify until this ruling and all fixes pass.

**User ruling recorded 2026-11-19 13:08:** The user explicitly selects
“Approve amendment (Recommended)” for the complete narrow assertion/helper
change. This is not permission to alter any other expected behavior or bypass
the private-snapshot byte identity. The independent author must retain the
exact version argv, no-PATH, no-input-mutation and archive controls.
DEF-571's ruling dependency is resolved; the three major fixes remain pending.

### AR-42 actual-core version entry stop — 2026-11-19 09:27

The real three executables are built offline as an ordinary user inside pinned
Ubuntu 22.04 image
`sha256:07e5d109f0438311482ea2b780e6084614f571250f08a248290b854ec36f8f85`,
using the current SDK's Rust toolchain, without changing `1.1.2`.
The local core fixture methods pass 23/23, but they cannot prove the real
language server exposes the required entry. Direct closed-input probes of the
actual binaries produce successful `clc`/`clorraine` version lines and
`clorraine-lsp` status 1 with empty output.
`phase2-core/missing-lsp-version-proof.json` retains the exact result and hashes;
no actual archive/stage/runtime acceptance is claimed.

| Grounding | Consequence |
|---|---|
| `compiler/src/lsp_main.rs:3–4` | Every invocation unconditionally runs stdio LSP; no argument handling exists. |
| `compiler/src/lsp/server.rs:511–521` | LSP initialize already reports the same current package version. The version identity exists; no protocol/library change is needed. |
| `03-04-editor-and-release.md`, local core producer contract | Every actual supplied executable must answer its own `--version`; silently bypassing the LSP assertion would violate the approved contract. |
| `99-execution-plan.md:163` | Current expected implementation paths do not include the LSP binary entry. Explicit expansion approval precedes edits. |

Recommend only the exact single-argument version fast path in the thin binary
entry, with a new independent process-specification file and the existing LSP
lifecycle suite as the compatibility control. All other argv vectors continue
to the unchanged stdio server, preserving editor startup. No help/parser
framework, new dependency or other CLI behavior is proposed.
The alternative uses existing initialize/serverInfo but adds protocol probing
to packaging and requires an explicit core-oracle amendment. Omitting LSP
version verification, accepting exit 1 or treating a banner fixture as real
evidence are not viable.
Confidence: high; hardening: actual built process result checked against the
entry point, existing protocol version response and independently authored
core oracle. This is a bounded interface/scope decision, not a licence or
publication ruling. DEF-569 tracks the pending user decision.

**AR-42 user ruling — 2026-11-19 09:43:** The user selects
“Add version flag (Recommended)” to the explicit scope-expansion question.
Approve only the exact `--version` fast path, current version/newline/status 0,
with normal stdio startup unchanged for all other argument vectors.
Independent process specifications run red before the thin-entry implementation.
The two named files join Phase 2's modification set. Existing core/LSP oracles,
version records, dependencies and no-commit/publication boundaries stay intact.
DEF-569's ruling dependency is resolved; actual core proof still gates progress.

### AR-41 ROM representation stop — 2026-11-19 06:01

Preparing task 2.2.5 against the host's installed `fs-uae` package `3.2.35-2`
reveals a syntax boundary before any native emulator code is written.
The upstream `v3.2.35` source shows:

| Source | Consequence |
|---|---|
| `libinih/ini.c:52–62,138–151` | A semicolon preceded by whitespace starts an INI comment; quotes/backslashes are not a general path-escaping solution. |
| `libfsemu/src/conf.c:104–119,256–294` | Literal `--option=value` arguments avoid INI comment parsing, but every value passes through `g_strstrip`, removing trailing ASCII spaces. |
| `src/fs-uae/config.c:80–92`, `paths.c` and `src/od-fs/libamiga.cpp:584–602` | The configured ROM value is expanded/resolved and handed to the UAE option parser; the original whitespace has already been lost. |
| `src/cfgfile.cpp:2143–2149` | The string setter copies the supplied value without a general unquoting/unescaping step. |

Sources are under
`https://raw.githubusercontent.com/FrodeSolheim/fs-uae/v3.2.35/`.
The existing local GLib `g_strchomp` primitive independently changes
`/owned/path/kickstart.rom ` into `/owned/path/kickstart.rom`.
`native-emulator-path-encoding-investigation.json` retains exact source hashes,
excerpts, installed package identity and primitive result. This is
source/primitive evidence, **not** a real FS-UAE launch, isolation or boot proof;
no emulator was launched or licensed media accessed during this investigation.

Recommend a bounded exception: ordinary host paths remain literal command
arguments; only a selected ROM filename ending in ASCII space gets a private
ordinary copy at a fixed safe media name outside the mounted `drive/`.
Preserve the original bytes/file and never expose that media in an archive,
upload or published evidence. The copy lives exactly as long as the child/run
storage; no shared ROM cache, new public option or generic path alias layer.
Existing ordinary-ROM argv expectations remain unchanged. After approval,
author a distinct trailing-space media/lifetime test before implementing
the exception. Actual version-specific boot/isolation evidence still gates
task 2.2.8.

The viable smaller alternative explicitly rejects this narrow filename case
with actionable rename guidance. It avoids copying media but narrows allowed
space-path behavior. Silently trimming a path, shell-quoting it, or inventing
unsupported INI escapes are rejected because they can select the wrong file.
The strongest cost against the recommendation is one extra private media file
to validate and clean; it adds no dependency or distribution surface.
Confidence: high on parser behavior; actual emulator acceptance remains
unverified. Hardening: checked the command-line/INI/ROM setter chain and the
real GLib whitespace primitive, rather than inferring syntax from an argv mock.

Proposed amendment set: this register, the native-driver owning section in
`03-03-native-driver.md`, additive test/acceptance evidence in
`07-testing-strategy.md`, checklist and roadmap. Implementation stays in the
already planned native emulator module. No support lock, source pin, Phase 1
artifact, existing oracle expectation, commit or publication change.
Directly affected tasks 2.2.5/2.2.6/2.2.8 await the user ruling through DEF-568.

**AR-41 user ruling — 2026-11-19 08:58:** The user selects
“Private ROM copy (Recommended)”. Only a selected ROM filename ending in
ASCII space uses the fixed private per-run media copy outside `drive/`;
ordinary ROM paths still pass literally. Validate the selected original
before use, preserve its file/bytes and keep each copy until its own child
stops. Copies are never shared, packaged, uploaded or published.
Amend the owning native contract and add independent trailing-space/copy-
lifetime tests before emulator implementation. No existing expectation is
weakened. DEF-568's ruling dependency is resolved; implementation and actual
version-specific acceptance remain required.

### AR-40 authoring readiness stop — 2026-11-19 02:22

The independent Phase 2 specification author
(`ses_e0e9ae0d7ffejNgOFVgi48Ct4I`) stops before writing task 2.1.1.
No forbidden implementation file was read, no test was written and no
verification was claimed by that author.

| Missing observable interface | Grounding | Necessary clarification |
|---|---|---|
| Independently valid installed SDK | `03-03-native-driver.md:39–44` requires provenance/installation validation and coherent pairing; `03-02-installation.md:46–57,90–94` specifies concepts and layout only | Exact metadata filenames, required fields/types, completion criteria, pairing/containment checks, and invalid-SDK versus missing-tool distinction |
| Companion vamos process | `03-01-support-builds.md:449–453,510–516` requires the matching private Python, isolated execution and the unique installed entry point | Exact installation-relative Python path and literal argument-vector contract; no relocation-sensitive pip shebang |
| Local core producer | `03-04-editor-and-release.md:152–157` identifies `package_core.py` and its output purpose, not a callable interface | Exact archive/staging arguments, current-version inputs, outputs and failure/no-clobber behavior so task 2.1.3 can test a public command |

The current preparation code already emits `.ndk-prepared.json` with schema,
lock, NDK and inventory digests (`tools/distribution/prepare_ndk.py:482,532–538`).
The accepted artifact oracle uses the venv's Python with `-I` and the unique
`amitools` console entry (`tools/distribution/tests/support_artifact_spec.py:23–30`).
These are reusable evidence, not a complete installed-layout contract.
The old driver still gates on the legacy marker and PATH
(`compiler/src/clorraine/environment.rs:19–27`,
`compiler/src/clorraine/mod.rs:109–114`); it cannot define the new oracle.

Recommend completing only the already planned interfaces and one valid
miniature fixture. Do not create a new metadata framework, generalised launcher,
dependency or production staging path outside this plan. An SDK build must
still not require vamos; native Run must use its paired runner; invalid selected
SDKs must not fall back. Preserve all established status, relocation and
activation-coherence requirements.

Proposed modification set after approval: the owning sections in
`03-02-installation.md`, `03-03-native-driver.md`,
`03-04-editor-and-release.md`, their cases/fixture contract in
`07-testing-strategy.md`, and this register/checklist/roadmap. The support lock,
Phase 1 producers/artifacts, frozen `spec/` and historical driver tests remain
unchanged. No implementation, version bump, commit or publication is proposed.
Confidence: high on the missing contract. Hardening: independent
implementation-blind author confirmed the gap; parent checked the existing
marker and accepted Python invocation before recommending reuse.
DEF-567 tracks the pending user ruling.

**AR-40 user ruling — 2026-11-19 02:33:** The user selects
“Approve amendment (Recommended)” to the explicit metadata/pairing/Python/local
producer question. Complete the bounded existing-plan amendment and resume
independent tests. The amended contracts define the six-field native descriptor,
reuse the prepared-SDK marker and full copied wheel manifest, fix the paired
venv Python argument vector, provide a complete miniature native fixture and
define the local `archive`/`stage` producer arguments/outputs. Native readiness
does not replace the installer's final smoke-tested transaction receipt.
No support-lock/producer/artifact, frozen spec or historical-oracle changes
are included. The directly affected tasks return to unstarted; DEF-567's ruling
dependency is resolved. Authoring/red proof still precedes implementation.

Task 1.1.1's independent specification author found this gap before writing
tests (`ses_e1a08b45fffexKdF1RfLSQlImP`). The conceptual records in `03-01`
§Locked inputs and the script paths in the execution plan do not define
callable signatures, JSON keys/types or provenance/output paths. The
distribution manifest is a different contract. Authoring guessed interfaces
would violate the independent-oracle rule. No implementation or tests were
written. The user authorised Phase 1 execution with “proceed”, then explicitly
approved completing this contract amendment and resuming specification tests.

**Approved contract boundary:**

| Surface | Proposed observable interface |
|---|---|
| Lock validation | `python3 tools/distribution/support.py validate-lock --lock PATH` |
| Staged-source validation | `python3 tools/distribution/support.py verify-sources --lock PATH --sources DIR` |
| Offline support build | `tools/distribution/build-support.sh --lock PATH --sources DIR --output DIR` |
| Payload audit/packaging | `python3 tools/distribution/package_support.py --lock PATH --payload DIR --audit PATH --output FILE` |
| Data formats | Schema-versioned JSON for the approved source/build/preparation/output records and per-file origin/licence/dependency evidence; one complete valid fixture for each |
| Failure contract | Nonzero exit naming the offending component/file, with no successful provenance or final payload; success is exit 0 with explicitly named output artifacts |

The contract amendment specifies exact keys/types, relative-path rules,
staged layout and network isolation before tests are written. It uses the
already approved scripts and standard-library unittest, rather than adding a
generic runner or validator framework. Public-command tests exercise the same
boundary later used by local builds and workflows; the counterargument is that
Python function tests are simpler, but they would leave the shell build and
packaging boundary separately unspecified.

The lock/source/build/payload interfaces and complete miniature fixture are now
specified in `03-01` §Executable support contract. The amendment stays within
the planned scripts, Docker build isolation and ordinary unittest boundaries.
It does not add the proposed generic oracle/flag manifest or another acceptance
runner. Preparation/wheel artifact cases use their existing planned script
surfaces and fixed corpus evidence when task 1.1.2 is authored.

The same approved amendment now specifies the two-argument preparation process,
its fixed metadata/marker, wheel builds through the existing locked recipe, and
ordinary unittest real-artifact inputs. It adds no mandatory root lock fields,
new runner or generic oracle manifest. Acceptance uses fixed emitted-C paths
and the existing compiler flag matrix; `volatile-mmio` remains Amiga-only.

**AR-31 evidence:** The pinned upstream Makefile clones LHA under `BUILD/lha`,
outside `projects/`, while the SDK provenance uses `make l` to list project
checkouts. The final image retains `bin/lha`, but no corresponding Git metadata.
All recorded short source identities resolved to full commits. The latest
reachable LHA commit before the final SDK image's creation is the proposed
historical snapshot; that date relationship cannot establish which revision
the original cached builder used. Current LHA master is a different, newer
commit and has not been substituted. See `02` §Execution source research.

The user approved this limited source-policy exception before the real lock
is written. The selected pin still requires the existing source/licence and
actual preparation proofs. Approval cannot grant redistribution rights or turn
a candidate into a tested helper.

Independent challenger `ses_e192c8bb7ffedxU3DJuOnzhTKj` recommends B: a new,
explicitly verified helper baseline can establish future reproducibility without
inventing historical provenance. A alone preserves the current exact-history
policy unchanged. The lead concurs. Confidence is high in the policy route and
moderate in the particular candidate until its source/rights/technical proofs
pass. Date alignment does not establish technical superiority or historical use.
The approved exception applies only to LHA; no other source or scope change
and no publication is authorised.

**AR-32 evidence and recommendation:** `03-01` §Support lock schema and §Source
staging and offline builds require a prepared registry-digest image. The
available SDK is a local Debian bookworm image, not the required Ubuntu
environment. Registry manifest digests and local image configuration IDs are
not interchangeable. No prepared Ubuntu image has been identified or published.

Independent challenger `ses_e18fea3c6ffeVNvktpDqLkOnqT` confirms the mismatch
and recommends B through one bounded ruling. The existing `build.image` can
identify the pinned Ubuntu base while the actual prepared execution image ID
is recorded separately in provenance. The exact preparation input/recipe and
package versions must remain locked; offline builds retain `--pull=never`,
`--network=none`, non-root execution and failure-safe publication.

A suitable existing prepared image would make A smaller, but availability
cannot be assumed and publication is not authorised. Confidence is high in the
contract issue, moderate that B is necessary until an actual image is provided.
The lead concurs with B. Required producer ordering also needs to make hashed
recipes/reviews/reference inventory available before final lock validation;
do not emit placeholder hashes or claim a partial lock is eligible.

This is a correction to the approved image semantics and frozen test boundary,
not permission to add a registry, third workflow or build-management framework.
Affected test expectations require the user's explicit amendment ruling before
they change. Dependency research itself remains authorised; it is not a new
finding or an excuse to change known runtime pins.

The user selected B. Implementation proceeds only after `03-01` states the
base/recipe versus execution-image distinction and the affected tests reflect
that explicit ruling. The existing eight display tests are unaffected.

**AR-33 evidence hold:** The required ordinary `libamiga.a` in the pinned build
repository is prebuilt. Its 297,694 bytes / SHA-256
`10d78b6d303a99677c0bf29a8e1653d62669f69f395691646393020325084021`
match the existing SDK exactly. Its archive-specific origin/terms/source chain
has not been established. The root GPL text and libnix's public-domain README
do not, by themselves, establish that chain.

Independent challenger `ses_e15eece42ffeTfvcz1WkuX8JVM` recommended finishing
one bounded primary-evidence pass before seeking a decision. That pass is now
complete for ordinary and base-relative archives: both reachable histories
contain only introducing commit `9ec400774c517fb67d65ad4ad8bdbdd94e715d8c`,
which adds the binary files and Makefile changes without a source/generator
reference. The reviewed complete root/libnix trees and helper files did not
locate a matching source/build explanation. This is not proof that no source
or licence exists.

The same challenger confirms that existing AR-18 blocks the affected asset and
dependent reference build until the evidence is supplied. No user policy
choice is needed to keep that gate; user approval cannot grant third-party
rights. Confidence is high in the evidence requirement, limited in the
archive's actual origin/legal status. The smallest remaining route within
approved scope is primary upstream/rightsholder provenance/source evidence,
not an unproven replacement library or converter.

The review and precise upstream question are retained in
`tools/distribution/inputs/licence-review.md`; roadmap dependency DEF-560 tracks
the hold. No upstream message, publication, replacement-source exception or
feature removal has been authorised. Other component reviews remain preliminary.

**AR-33 subsequent user directive — 2026-11-17 21:08:** After reviewing the
archive's immediate toolchain origin and unresolved historical provenance,
the user explicitly directed copying and including it and took responsibility
for its legalities. This supersedes the implementation stop for the exact
ordinary archive only. Resume local build/packaging work and retain DEF-560 as
user-owned legal follow-up, not an engineering dependency.

Do not change its bytes, replace the source cohort, extend the directive to
other unknown-origin files, or label the archive legally cleared. The source/
terms evidence remains unresolved. No public release, push, workflow dispatch
or commit is authorised by this directive; AR-22's separate promotion boundary
and the user's commit-at-end preference remain in effect.

**AR-34 measured evidence — 2026-11-18:** The corrected fixed-input reference
build passed. Its SDK has 3,793 regular files, 11 internal relative links,
52 native x86_64 ELF files, 166 m68k archives and 26 standalone target `.o`
files. Cross-objdump identifies required `libnix/lib/ncrt0.o` and `libinit.o`
as `file format amiga`, architecture m68k, without an archive wrapper.
The 165 HUNK-format archives still identify as archives; only one target
archive uses the Unix `!<arch>` magic.

The closed enum at `03-01:211` cannot accurately represent the standalone
startup objects. Omitting their native records or calling them ELF/archives
would weaken the actual-content audit at `03-01:230–236`. Proposed bounded
amendment: `format: "amiga-object"`, `machine: "m68k"`, `soname: null`,
`needed: {}`, `glibc: []`, with actual-content validation. Keep existing
ELF/archive rules and all bad-architecture/misdeclared-content rejections.
Add a real-artifact positive startup-object case and a content/record-mismatch
case before the affected implementation. No language specification changes.

The second issue is producer order: `03-01:101–120` requires real hashed
preparation/build script inputs, while `99:103–107` asks task 1.2.5 to finish
the eligible lock before the support/preparation producers are implemented.
`reference-support.sh` deliberately writes `/work/reference`, not the public
`/work/payload` contract (`03-01:181–186`); it also invokes compilers and is
not a valid standalone preparation script. Reusing it as either missing
producer merely to satisfy a hash/path check is not acceptable.

Recommended correction: split current task 1.2.5 into reference proof now and
final-lock completion after the required producers exist. Separate recipe
authoring/manual fixed-input wheel proof from subsequent real locked-command
execution. This adds one task (91 total, 20 in Phase 1), changes no verified
task ID, and preserves source pins, tests-first order, full verification and
publication boundaries. Renumber only the still-unstarted late Session 1.2
tasks when applying the approved correction. The alternative of authoring
all remaining producer implementations under task 1.2.5 hides their ownership;
placeholder inputs or mislabelled objects are rejected.

Proposed amendment set: this register, `03-01`, `07`, `99`, the roadmap and
the support contract/real-artifact specification files and their test-only
fixtures as needed. All are within the existing phase modification set.
Normal-mode user approval is required before changing those contracts,
oracles or task ordering. Actual reference measurements and existing green
checks remain evidence, not approval. DEF-561 tracks this temporary hold;
libamiga legal follow-up remains separately user-owned.

**AR-34 user ruling — 2026-11-18 12:52:** The user selected
“Approve both (Recommended)” on the explicit two-part question: add the
standalone Amiga object contract/tests and split final-lock completion into a
later task after its producers, for 91 total tasks. Apply that bounded
amendment, author and verify the tests red before affected implementation,
then resume. No source pin, dependency, product scope, acceptance requirement,
commit preference or public-promotion boundary changes.

**AR-35 concrete evidence:** The measured C compiler itself requires
`libstdc++.so.6`, not only an optional C++ front end. The prepared image's
package receipt records `libstdc++6:amd64` version
`12.3.0-1ubuntu1~22.04.3`, and its build package `g++` has version
`4:11.2.0-1ubuntu1`. Both unqualified package names fail the generic `Id`
grammar at `03-01:89`; build/runtime package maps currently require that
type. Real acceptance executes `dpkg-query ... name` and checks actual owner
names in `support_artifact_spec.py`, so invented safe-looking aliases would
not satisfy the existing ownership oracle.

Recommend a package-specific grammar, not a global relaxation of source IDs.
Use actual unqualified Debian names for package map keys/provider references;
keep the architecture fixed by the existing x86_64/Ubuntu cohort contract.
Add positive `g++`/`libstdc++6` structural cases and unsafe-name rejection
controls before implementing the validator. Corresponding source/helper IDs
must remain under the old generic grammar. This changes no package version or
actual dependency. Bundling another C++ runtime just to evade the key grammar,
or translating arbitrary aliases in a new layer, is rejected as unnecessary.

This is a separate bounded amendment from the two approved AR-34 corrections.
Those remain approved/applied. Current reference/oracle verification may
finish, but affected consumer/producer implementation waits for AR-35.
DEF-562 tracks the ruling. No package-name contract/test amendment is applied
before approval.

**AR-35 user ruling — 2026-11-18 14:26:** The user selected
“Approve package names (Recommended)” on the explicit package-only grammar
question. Apply that narrow contract amendment and author/verify its positive
and unsafe-name specifications before implementing validation. Source/helper/
tool IDs and actual dependencies, versions, commands and publication boundaries
remain unchanged. DEF-562's ruling dependency is resolved.

**AR-36 measured evidence and independent challenge — 2026-11-18:** The
implemented checker passes the actual stored-object/index/worktree checks for
11 of the 12 acquired repositories. fd2pragma alone fails on `fd2pragma.c`.
Its verified HEAD blob `e00f6a61fd49bf273bb09524e171b0ddebc7cb81`
starts with `/* $Id$ */`; actual checkout bytes start with
`/* $Id: e00f6a61fd49bf273bb09524e171b0ddebc7cb81 $ */`.
The complete files differ only by that built-in Git expansion. Locked
`.gitattributes` contains `*.c ident`; mode and index still match.
No pin, staged file or retained reference was modified.

Independent read-only challenger `ses_e11060287ffeWUCYdnPxI3q8iY`
recommends B: canonicalise only proven checkout transformations in fresh
maintainer-owned staging, keep the verifier's raw-byte equality and repeat
affected reference proof. The existing “exact checkout / clean state” wording
does not explicitly choose raw blobs versus built-in Git expansions
(`03-01:137–145`), so obtain a ruling rather than silently deciding.
The recipe copies actual worktree bytes (`reference-support.sh:148–154`);
old Git-clean checks alone do not establish raw-byte identity.

Option A remains viable but needs more logic: derive ident from authenticated
locked attributes, read/hash the stored blob and compare actual bytes with
exactly computed allowed expansion. Do not normalise arbitrary worktree
markers and hash the result. Unlocked `.git/info/attributes` must have no
influence; `git check-attr --cached` alone is insufficient. External filters
remain forbidden and the existing execution-sentinel test stays intact.

The lead concurs with B because it adds no accepted-byte representation or
attribute-resolution machinery. Confidence: high. Hardening: independent
challenge completed. This is a bounded source-identity clarification, not
permission for a new source cohort, public command, dependency, transformed-byte
framework, commit or publication. Add the selected narrow oracle coverage
before affected implementation; preserve original reference evidence and
repeat the affected build/inventory proof before claiming completion.
DEF-563 tracks the ruling; task 1.2.6 remains implemented but unverified.

**AR-36 user ruling — 2026-11-18 15:11:** The user selected
“Canonical bytes (Recommended)”. Git source worktree bytes must match the
stored authenticated blobs, even where Git would apply built-in checkout
conversions. Verification remains read-only and never repairs inputs.
Acquire/canonicalise only maintainer-owned staging after proving the exact
known transformation; preserve the original acquired checkout and private
reference evidence. Author the raw-positive/expanded-negative specification
cases before staging changes, then repeat affected reference build/inventory
proof. Existing raw-byte validation need not be relaxed. All pins, commands,
dependencies, commit preferences and publication boundaries remain unchanged.

**AR-37 measured evidence and independent challenge — 2026-11-18 17:35:**
The actual standard Ubuntu 24.04 image is
`sha256:46b397e434a7871e1dbb8a043c927d1f1004f4de4714d736a74de56136d0b2a1`.
The primary greenlet extension requires `ld-linux-x86-64.so.2`, and `ldd`
reports `/lib64/ld-linux-x86-64.so.2`. `dpkg-query -S` on that alias exits zero
but returns only `diversion by libc6 from:` and `diversion by libc6 to:` rows.
The strict resolved path `/usr/lib/x86_64-linux-gnu/ld-linux-x86-64.so.2`
has the genuine owner `libc6:amd64`.

The protected oracle (`support_artifact_spec.py:246–255`) canonicalises only
after a nonzero query exit and then treats all lines as owner records. Its
existing assertion therefore fails for this real standard installation.
`runtime-owner-compatibility-probe.json` retains the exact queries, original
parser result and unchanged oracle SHA-256
`b9374b934e0882284adc5c103fe1b3b03012c2f34e06d04f627738ad4b569c45`.
Actual provider/loader measurements pass in both clean prerequisite images
when resolving the real loaded file and distinguishing commentary from owners.
This is not full archive acceptance or permission to alter the test.

Independent read-only challenger `ses_e107e22c7ffe4iax27w4NtcQoJ` confirms
an oracle defect, not evidence of a binary/ownership defect. The approved goal
is actual package ownership (`03-01:116–120,515–531`), not ownership of a
lexical symlink spelling. The lead recommends the narrow correction:
strictly resolve first, parse only genuine owner records, and allow a bounded
merged-/usr alias fallback only if it resolves to the same file and no genuine
canonical owner was found. A genuine wrong owner must fail; do not search
alternate spellings until the expected name appears.

Before modifying that oracle, obtain explicit approval and add positive
canonical-symlink/diversion and negative wrong-owner/missing-owner/diversion-only
controls. Preserve the original file/hash, and rerun the real 22.04/24.04
probes. Keep all hash, containment, version, SONAME, GLIBC and real loader
assertions unchanged. Changing lock path spelling cannot fix a name taken
from `ldd`; relabelling providers, modifying standard system layouts or
skipping ownership checks are rejected as evasions. No new runtime dependency,
framework, product scope, commit or publication is proposed.
Confidence: high. Hardening: independent challenge completed. DEF-564 tracks
the ruling; task 1.2.10 has preparatory evidence but no eligible lock yet.

**AR-37 user ruling — 2026-11-18 17:42:** The user selected
“Approve narrow correction”. Preserve the original oracle and evidence,
author/run ownership controls before the correction, then require genuine
ownership of the actual resolved loaded file. Retain every existing package,
version, SONAME, GLIBC, hash, containment and actual loader check. No wrong or
missing owner may pass; diversion commentary alone is not an owner. Recheck
both actual Ubuntu environments before resuming the real lock/acceptance work.
DEF-564's ruling dependency is resolved; implementation proof remains in the
owning plan task.

**AR-38 measured evidence and independent challenge — 2026-11-18 20:28:**
Each actual Ubuntu archive suite ran six methods: four passed, while native
imports and relocation stopped before corpus execution. The shipped
`greenlet.tests` extensions import the test package, whose initializer requires
psutil. Its optional test closure also includes objgraph and setuptools; this
is not a missing declared vamos-runtime dependency.

The pinned original archive's `setup.py:171–190` deliberately includes native
test extensions, `setup.py:198` discovers the test package, and
`pyproject.toml:55–64` declares the separate test extra. Both actual wheels have
31 members under `greenlet/tests/`, including two test-native extensions.
The producer currently builds that unmodified upstream wheel
(`wheel-payload.py:268–288`) and checks only runtime requirement markers
(`:289–300`). The immutable artifact oracle correctly imports every identifiable
shipped native extension (`support_artifact_spec.py:58–65`).

Independent read-only challenger `ses_e0fdef5d2ffeV7IKRcALLI7m3C` confirms a
runtime-payload selection mismatch, not an established oracle defect.
The minimum-sufficient recommendation is deterministic removal of exactly
`greenlet/tests/` before final wheel hashes are recorded, with a regenerated
wheel RECORD and unchanged surviving member bytes/metadata. Preserve the
runtime extension, public headers, both licences and the original full source.
Add red/green exclusion, surviving-content, RECORD and determinism controls
before the producer correction. No filtering framework or new dependency is
proposed.

Pre-build exclusion is also technically viable, but must cover both extension
definitions and package data; `MANIFEST.in` recursively includes test source.
No dedicated upstream exclusion switch was found. Exact post-build subtree
selection offers a smaller, directly checkable preservation boundary.
Changing acceptance imports/skipping shipped test extensions would hide the
shipped optional code and is rejected. Adding only psutil is incomplete and
adding the test dependency closure is unnecessary scope expansion.

Explicit approval must cover this narrow runtime selection and renewed
lock-bound proof: preserve existing failed/successful historical artifacts,
update exact recipe/lock hashes, and regenerate affected producer provenance,
metadata, audits, archives and both actual Ubuntu acceptance runs. Do not
relabel old receipts to a new lock. No source pin, protected assertion,
installation-build boundary, commit preference or publication authority changes.
Confidence: high on diagnosis/boundary. Hardening: independent raw-source,
actual-wheel and log challenge completed. DEF-565 tracks the pending ruling.

**AR-38 user ruling — 2026-11-18 20:37:** The user selected
“Exclude test-only files”. Apply only the exact Greenlet test subtree boundary,
preserve all surviving member bytes except regenerated RECORD, and retain the
full original source/licences. Author exclusion/preservation/RECORD/determinism
controls before implementation. Keep the artifact oracle unchanged. Preserve
old artifacts/evidence, refresh exact lock bindings and repeat affected public
producers, preparation, assembly/packaging and real Ubuntu acceptance.
No new dependencies, pin changes, commits or publication are authorised.
DEF-565's ruling dependency is resolved; successful implementation/reproof
remains in task 1.2.12.

**AR-39 measured evidence and independent challenge — 2026-11-18 22:03:**
After the approved Greenlet correction, both actual Ubuntu suites pass five
methods and the complete original-location corpus. Native imports, loader
ownership and moved-root host cases pass. The ten relocated Amiga compile
subtests per release fail before cc1. A separate dry-run mounts identical
compiler bytes at plain/spaced prefixes: printing the resolved runtime directory
works at both, but nix13/nix20 specs processing fails only at the spaced prefix.
`gcc-spaces-diagnostic-proof.json` and both
`acceptance/ubuntu-<release>-relocation-failed-proof.json` receipts retain the
actual queries/commands/outputs; the artifact oracle is unchanged.

Independent read-only challenger `ses_e0f877e54ffeLtsL1k8B39ZvTR` confirms
the source-backed defect. The GCC specs engine reparses function results
(`gcc.cc:7194–7195`) and splits unescaped spaces (`:6231–6233`). The raw
find-file return (`:10816–10817`) is inserted by Amiga runtime `-B` rules
(`m68kamigaos.h:407–412`) and becomes stray input rejected at
`gcc.cc:6004–6010`.

The minimum-sufficient recommendation changes only that spec-function return
to `quote_spec(xstrdup(file))`, plus durable rationale. The existing helper
quotes active specs characters (`gcc.cc:11343–11368`); other functions already
use that exact allocation/helper pattern (`:10931,:11272`). Copying avoids
mutating/freeing find_file's possibly borrowed input. Do not change find_file
itself, malformed-argument handling or missing/empty-result semantics;
quote_spec_arg would add an unnecessary empty-argument behaviour change.

Blast radius covers Amiga include paths, all five runtime `-B` branches and
attached linker `-L` paths. Picolibc's nested uses were reviewed, but no general
Picolibc whitespace guarantee is proposed: its outer spec function adds another
parsing boundary. The separate plugin-only find-plugindir function is outside
this correction unless an in-scope failure is established. No unsupported
specs-file quote syntax is presented as a verified alternative. Wrappers,
whitespace-free aliases or banning spaced roots would add machinery or evade
the approved requirement.

After explicit approval, add distinguishing red/green controls for found/
missing paths, separated `-B`/`-isystem`, attached `-L`, literal specs
characters and preserved empty/malformed behaviour. Authenticate the original
checkout unchanged; apply a locked exact-context patch only after private
source copying and before compilation. Reject mismatch/repeated application.
Keep GCC pin `26e100b31d25392a21068483d4c541a4b55822c3`. Record patch/recipe/
source-delivery input hashes under a new lock and preserve current evidence.

Corresponding source must include the full original GCC snapshot, exact patch,
modification/date notice, retained licences and sufficient patch/configure/
build/install instructions. The packager's current fixed build-input list
does not automatically convey new patch inputs, so explicitly update source
assembly and verify its contents. Re-run affected public producers,
preparation/assembly/archives and unchanged real acceptance on both Ubuntu
releases. No source-pin change, new dependency/framework, protected-test
amendment, commit or publication is proposed.
Confidence: high on diagnosis/helper choice; end-to-end sufficiency unverified.
Hardening: independent parser/call-site/source-obligation challenge completed.
DEF-566 tracks the required ruling.

**AR-39 user ruling — 2026-11-18 22:20:** The user selected
“Approve GCC fix (Recommended)”. Author the focused controls first, then change
only the quoted return boundary in the private GCC build copy using an exact
locked patch. Keep original staged/source-pin bytes and the protected artifact
oracle unchanged. Convey the full original source, exact patch, retained
licences/modification notice and sufficient rebuild instructions. Refresh
affected lock/provenance/source-delivery identities and repeat real producers,
archives, preparation and both Ubuntu relocation suites. No wrapper,
dependency, source-pin change, commit or publication is authorised.
DEF-566's ruling dependency is resolved; implementation/reproof remains in
task 1.2.12.

The user subsequently approved all nine corrections in the
[preflight report](00-preflight-report.md), including plan-only application and
bounded re-scan. Those post-creation rulings supplement this register; their
decisions and fix evidence remain owned by the report. They do not authorise
implementation or publication.

| Area | Verified evidence | Consequence for the plan |
|---|---|---|
| Native SDK discovery | `compiler/src/clorraine/environment.rs:20` fixes the marker at `/opt/amiga/BUILD-PROVENANCE.txt`; `compiler/src/clorraine/mod.rs:109-126` blocks non-host build/run without it. | A portable archive alone does not provide native `clorraine build`. |
| Existing emulator contract | `compiler/src/clorraine/toolchain.rs:295-308` prints a Docker handoff; `compiler/tests/clorraine_driver_spec_test.rs:305-355` asserts it exactly. | Specify the native path and existing container behavior before altering an oracle. |
| Profile/run separation | `compiler/src/clorraine/mod.rs:134-155` requires `vamos` for Amiga `run` without `--emulator`; `toolchain.rs:60-63` selects `nix20`/`nix13`. | A native FS-UAE installation does not automatically provide every existing run mode. |
| VS Code | `tools/vscode/package.json:18-49` declares language/LSP settings but no Build/Run commands. | Build/Run integration is implementation work, not packaging already-complete functionality. |
| Current installer | `tools/setup/install_sdk.py:1-19` prepares Docker; `../clorraine-installer/README.md:3-15` describes the QtIFW online scaffold. | Neither is the approved native shell installation flow. Repository ownership remains AR-17. |
| Linux baseline | `docker/Dockerfile:21-26` uses Debian bookworm. | Ubuntu 22.04 compatibility must be built and tested against that baseline; it is not proven by the current image. |
| Source pin completeness | `docker/versions.env:13-14` pins the upstream build repository, but that commit's `default-repos` names branches (`amiga16.2`, `amiga-2.46`, `master`, etc.). Its Makefile clones those branches. | The earlier assertion that the top-level pin fixes all inner source revisions was too strong. Record exact inner revisions for the release build. |
| NDK preparation | The pinned upstream Makefile's NDK rules patch headers, add `__stdargs`, generate `proto`/`inline` files with `sfdc`, and derive `ndk13-include` from the newer NDK. | Fetching and extracting the upstream archive alone has not been proven sufficient; the preparation tools and derivative distribution terms matter. |
| Relocation | Earlier probing copied the installed tree while its original `/opt/amiga` remained present. | Require a clean-environment test with the original prefix and build/source trees unavailable. |
| vamos dependencies | Installed SDK metadata reports amitools `0.8.1.dev274+gd0b9727da` from commit `d0b9727da3069328231784d2d5fdcf44816c95b1`, machine68k `0.4.1`, greenlet `3.5.6`, and PySDL2 `0.9.17`. machine68k and greenlet contain CPython 3.11 native extension binaries. | Do not treat vamos as pure Python. Prebuild compatible wheels for Ubuntu 22.04/24.04's supported Python interpreters and validate runtime library closure. |
| NDK archive evidence | Downloaded NDK3.2 archive matches SHA-256 `96cabd4ad683dced632e147bf86dee0f50dcb1254386216c25c362916a6409bb`. Read its `ReadMe-NDK.txt` and `lib/README`; these do not establish permission to redistribute a modified header tree. | AR-28 uses upstream acquisition and local preparation; no inferred redistribution grant. The inspection ran in an ephemeral container and removed its scratch archive. |

Upstream sources inspected:
- [Pinned build Makefile](https://github.com/AmigaPorts/m68k-amigaos-gcc/blob/e7ed7a735c4dd9d2c47e23a50e51b2b7ddd4688c/Makefile)
- [Pinned default-repos](https://github.com/AmigaPorts/m68k-amigaos-gcc/blob/e7ed7a735c4dd9d2c47e23a50e51b2b7ddd4688c/default-repos)

### Current resolution notes

- **AR-12–16:** The source of authority is the user's statements above, not
  earlier assistant estimates of platform support, build duration, binary size,
  signing behavior, or licence permissions.
- **AR-17 / AR-22:** Read-only challenger `ses_e1bd0f4c3ffeVMabOc0bwJWwN4`
  recommended compiler-repository ownership. Verdict: **Simplify** release
  automation to a reusable toolchain prebuild workflow and a compiler/package
  verification workflow; **Justified** shell install/update with checksums and
  failure-safe staging. The user selected that bounded compiler-repository
  option. The review found no need for a channel service, new signature
  verifier, rollback manager, mirrors, or GUI. No such machinery is authorized.
  The strongest counterargument was independent distribution ownership/cadence
  in the installer repository; the user chose the single-repository arrangement.
  Confidence: high on ownership/simplification, medium on packaging feasibility.
- **AR-17 approval record:** Original goal: reusable GitHub prebuilds downloaded
  by a native installer. Support code: the two named workflows and small
  install/update script. Evidence: existing pins, driver, editor and tests in
  this repository; sibling is a QtIFW scaffold. Smallest solution: prove
  archives manually, then automate the same commands here. Extra cost:
  workflow, packaging and installer tests plus release upkeep; no second
  repository coordination. Independent verdict: **Simplify**. Direct user
  decision: **Compiler repo (Recommended)** on the explicitly described scope.
- **AR-18:** Upstream licensing and native compatibility are research/proof
  obligations. User approval cannot turn an unverified capability or
  redistribution right into an established fact.
- **AR-19:** Read access to driver specification tests does not authorize
  changing their required behavior. The native/container compatibility boundary
  must be stated before tests or implementation change.
- **AR-23:** WSL can invoke Windows executables through interoperability; the
  earlier claim that manual copying is necessarily required was too strong.
  The user explicitly chose build/editor support for this release. Automatic
  Windows FS-UAE launch remains unimplemented and is not a release requirement.
- **AR-26:** This is a user-requested scope choice. Do not silently replace it
  with a bundle that merely reports missing `cc` or `vamos` as optional. Resolve
  how successful installation makes both run modes available without building
  dependencies on the user's machine.
- **AR-27/28:** Package installation and local NDK header preparation do not
  authorize compiling GCC, Python, vamos native extensions, or helper tools on
  user machines. Prebuilt helpers and wheels are part of the support-artifact
  design.
- **AR-29:** The current container-only driver requirements are stale with
  respect to the newly approved native distribution. Amend only that boundary;
  retain the existing container handoff oracle. Native-specific behavior still
  needs its approved contract and specification-first tests.
- **Complete-register confirmation:** The user selected **“Confirm complete
  register”** on the question explicitly covering AR-12–29, the detailed
  precedence/activation/editor contracts, support-only distribution revisions,
  Python 3.10/3.12 wheels, and the distinction between planning, implementation
  and publication.
- **Verification baseline:** `make check` completed successfully on 2026-11-16:
  4,232 passing Rust tests across 148 result blocks; Python suites 29 + 103 + 28
  tests; fuzzer smoke `cases=28 host=28 sanitizer=16 target=16 failures=0`.
  This validates the task command, not the proposed native distribution.

## Historical register — superseded QtIFW proposal

The following 2026-11-15 entries and challenger note are retained as an audit
trail. They were never approved. They are not active decisions or authorization
for QtIFW, GitHub Pages, or cross-repository implementation. Current decisions
above replace their assumptions; their identifiers are not reused.

| # | Category | Ambiguity / Gap | Options Presented | User Decision | Status |
|---|----------|-----------------|-------------------|---------------|--------|
| 1 | Scope | How far does this delivery go? | A: v1.2.0 release + first Linux x86_64 core installer + online repository + release attachment + site quick-start; SDK/editor/macOS/Windows deferred. B: release only. C: full installer surface now. | — | ❌ Open |
| 2 | Scope | Which platforms and components are in this cycle? | A: Linux x86_64 only; core component only (`clc`, `clorraine`, `clorraine-lsp`). B: add macOS/Windows builds. C: add the SDK and editor components. | — | ❌ Open |
| 3 | Technical | The work spans two repositories. Where does the plan live and how is it shaped? | A: one umbrella plan in `clorraine` (`plans/v1.2.0-release-installer/`) with explicit installer-repo phases. B: two plans, one per repository. C: plan only in the installer repo. | — | ❌ Open |
| 4 | Integration | What must the v1.2.0 payload contain for the installer contract? | A: add `clorraine-lsp`, `LICENSE-EXCEPTION.md`, `NOTICE`, `TRADEMARKS.md`; keep `clc`, `clorraine`, `LICENSE`, `THIRD_PARTY_NOTICES.md`, README, CHANGELOG, release notes, `docs/known-limitations.md`. B: keep the v1.1.2 file set. C: add only `clorraine-lsp`. | — | ❌ Open |
| 5 | Data & state | Which version sources move to 1.2.0, and how are the runbook and notes handled? | A: bump `compiler/Cargo.toml`, `Cargo.lock`, `tools/vscode/package.json`, `tools/vscode/package-lock.json`; installer component `1.2.0`, installer build `config.xml` stays `1.0.0`; rewrite `RELEASE.md` as the v1.2.0 runbook with v1.1.2 moved to the historical section; replace `RELEASE-NOTES.md`; manual packaging commands in the runbook, no new script. B: append a v1.2.0 section; keep `RELEASE-NOTES.md` as v1.1.2. C: add a release packaging script. | — | ❌ Open |
| 6 | Technical (complexity escalation) | How is the QIF online repository published to the Pages URL baked into the installer? | A: add `publish.yml` to `clorraine-installer`: `workflow_dispatch` with version + tarball URL + SHA-256, download and verify, build repository + installer, deploy Pages, upload the installer artifact. B: build locally and publish manually to a `gh-pages` branch, switching Pages to branch source. C: build an offline installer with the payload embedded and skip hosting. | — | ❌ Open (independent verdict pending) |
| 7 | Integration | How is the installer itself distributed? | A: attach the Linux installer to the v1.2.0 GitHub release and link it from the site quick-start. B: publish it only as a workflow artifact. C: host it only on the Pages site. | — | ❌ Open |
| 8 | Security & compliance | Qt Installer Framework is GPL-3.0-only with the Qt GPL exception. May generated Linux installers be distributed publicly? | A: confirmed for Linux x86_64; review the linked Qt runtime before macOS/Windows. B: defer public publishing until a legal review; keep the repository and installer private. | — | ❌ Open |
| 9 | Non-functional | Which commands verify each surface? | A: main repo `make check` per task and `make verify` per phase/release; installer repo `make test` per task and `make smoke` per phase; site `make site-check`. B: `make check` everywhere. C: full `make verify` per task. | — | ❌ Open |
| 10 | Behavioral | When may the public tag, release, Pages deployment, and installer attachment happen? | A: only after every gate is green and the user gives an explicit publication go for that step. B: automatically after the gates. | — | ❌ Open |
| 11 | Naming | Plan folder, roadmap row, and installer-repo change scope? | A: `plans/v1.2.0-release-installer/`; new roadmap row PLAN-45 at `Plan Created`; installer repo changes limited to the publish workflow and its docs/tests. B: no roadmap row; installer repo may also change QtIFW pin or dependencies. | — | ❌ Open |

### Resolution Notes

**AR-1:** The scope boundary is the umbrella decision; AR-2 fixes the platform/component cut, AR-6 the hosting mechanism, and AR-7 the distribution channel.
**AR-6:** This row is also a Complexity Escalation Gate item. The independent blind challenger returned **Justified**, with three conditions: (1) do not upload the installer as a workflow artifact — installer distribution belongs to AR-7; (2) state that the v1.1.2 payload (two binaries) cannot satisfy `verify_payload`, so AR-4 must land first; (3) pin `runs-on: ubuntu-latest` and emit `repository/<platform>/` so the Pages artifact shape matches the baked URL. The user has not yet approved the larger option.

<!-- Decisions are recorded here as the user answers; the header changes to GATE PASSED only when every row is Resolved or a fully named Deferred entry. -->
