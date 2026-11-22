# Preflight report: v1.2.0 native Linux delivery

> **Status**: ✅ PREFLIGHT PASSED — all 9 findings resolved
> **Iteration**: 2 — bounded re-scan complete; final minor clarification directly verified
> **Artifact**: Ten planning documents in `plans/v1.2.0-release-installer/`
> **Passed target SHA-256**: `bd89221f668746a31d6235269f1eb29a81159db77372c4dbd54ed2ef7e5f540a`
> **Independent re-scan SHA-256**: `4492791f4283f73bb03556e5e5624b867bc9dc9784a5fb4861e2ef587228c5bd`
> **Original SHA-256 (iteration 1)**: `54538d957d8a9ef801806981ff3929607444a1947baa9cc2cf3b97634fdc8746`
> **Code baseline**: `daa2fba0fb0114921f5e49638ac3a20eb2cd7050`
> **Last Updated**: 2026-11-16 14:53
> **Scope update (2026-11-22)**: AR-49 removed WSL2/Remote-WSL and Windows access
> from v1.2.0. WSL references below are historical; they no longer define a
> current acceptance gate.
> **CodeOps Artifact Schema**: 1

**SAME-SESSION REVIEW:** This plan was created in the current session. Five
fresh-context audit clusters and the Amiga specialist reviewed it independently.
One additional fresh-context challenger reviewed the combined major findings.
Consider a fresh-session or human domain review before implementation.

## Scope and authority

| Boundary | Record |
|---|---|
| Exact target | `00-ambiguity-register.md`, `00-index.md`, `01-requirements.md`, `02-current-state.md`, `03-01-support-builds.md`, `03-02-installation.md`, `03-03-native-driver.md`, `03-04-editor-and-release.md`, `07-testing-strategy.md`, `99-execution-plan.md` |
| Hash method | SHA-256 over sorted filename + NUL + file bytes + NUL records; this report and continuity notes are excluded |
| Scope baseline | Confirmed AR-12–29 and their detailed contract; historical AR-1–11 are superseded |
| Context only | Driver requirements amendment; compiler/driver/LSP source and tests; Docker SDK/build/runner files; setup/editor code and tests; release/site docs; AGENTS.md and roadmap |
| Current modification authority | User explicitly approved applying all six major and all three minor recommendations to the target, followed by bounded re-scan. Context files remain outside that authority |
| Execution/publication | Not authorised by this audit. All 89 revised implementation tasks remain unstarted |
| Roadmap | PLAN-45 advances to Plan Preflighted; 0/89 implementation tasks complete |

## Codebase context and verification

| Surface | Grounded observation and principal files |
|---|---|
| Compiler/driver | Std-only Rust; explicit user-source inputs, embedded standard-library injection, profile-specific C commands. `compiler/src/{driver,analysis}.rs`, `compiler/src/load/mod.rs`, `compiler/src/clorraine/{cli,environment,build,run,toolchain,artifacts,mod}.rs` |
| Editor/LSP | Existing dependency index is separate from CLI loading; extension currently supplies LSP features. `compiler/src/lsp/workspace.rs`, `tools/vscode/src/extension.ts`, `tools/vscode/package.json` |
| Historical oracle | Explicit multi-file inputs, exact Docker emulator handoff, host exit-status propagation. `compiler/tests/clorraine_driver_spec_test.rs` |
| Setup/packaging | Current Python installer expects a checkout; native packagers are planned work. `tools/setup/install_sdk.py`, `tools/setup/tests/support.py`, `RELEASE.md` |
| Amiga gates | Full verification reaches wrappers that reload local `.env`. `Makefile`, `docker/scripts/{corpus-fs-uae,run,emulator}.sh`, `docker/compose.yaml` |
| Support inputs | Current container does not establish clean Ubuntu or Python 3.10/3.12 portability. `docker/Dockerfile`, `docker/versions.env`, target `02-current-state.md` |

| Check | Evidence |
|---|---|
| Structural validation | Original: 56 cases / 76 tasks. Revised: ten target documents, 60 unique ST cases, 89 unique unstarted tasks across six phases/18 sessions; 14/14 requirements covered; IDs, links/anchors, tables, red/green ordering and whitespace valid |
| Staleness | Independent re-scan used the recorded hash; the final user-approved PF-001 clarification was directly verified against code constants and ST-59. Final hash includes that clarification and the index's status-only update |
| Measured planning baseline | `make check`: 4,232 Rust tests; Python 29 setup + 103 performance + 28 fuzz tests; smoke `cases=28 host=28 sanitizer=16 target=16 failures=0` |
| Evidence limits | That baseline validates the existing checkout and task command. No native archive, clean Ubuntu, desktop candidate, native FS-UAE or actual WSL2 acceptance is claimed |
| Full gate | Not rerun for this document audit. PF-007 must supply a private-display route before implementation's first full gate |

## Finding history and final disposition

Findings can belong to several dimensions; dimension counts are not additive.

| # | Dimension | Findings | Highest severity |
|---|---|---|---|
| 1 | Ambiguities | PF-002 | Minor |
| 2 | Implicit assumptions | PF-001, PF-006, PF-007 | Major |
| 3 | Logical contradictions | PF-004, PF-005 | Major |
| 4 | Completeness gaps | PF-003 | Major |
| 5 | Dependency issues | PF-003, PF-004 | Major |
| 6 | Feasibility concerns | None additional; first-phase proof stops retained | — |
| 7 | Testability | PF-001, PF-006, PF-007, PF-008 | Major |
| 8 | Security blind spots | PF-006 | Major |
| 9 | Edge cases | PF-002, PF-006 | Major |
| 10 | Scope creep indicators | None against confirmed AR-12–29 | — |
| 11 | Ordering and sequencing | PF-003, PF-005, PF-007 | Major |
| 12 | Consistency | PF-001, PF-004, PF-009 | Major |
| 13 | Codebase alignment | PF-001, PF-002, PF-003, PF-006, PF-007, PF-009 | Major |

| Severity | Count | State |
|---|---|---|
| Critical | 0 | — |
| Major | 6 | Resolved at plan level |
| Minor | 3 | Resolved at plan level |
| Observation | 0 | — |

Domain coverage included compiler/profile compatibility, installation
data/migration, concurrent activation/process state, and the Amiga platform.
Licensing, locked-source closure, native wheel portability and relocation are
already explicit Phase 1 proof stops. Their uncollected evidence is not treated
as an extra planning defect or as proof of feasibility.

## Major findings

Finding locations below retain iteration 1 line references. The correction
table at the end identifies the current owning sections; finding IDs stay stable.

### PF-001: Editor actions omit imported user-source files 🟠 MAJOR

**Dimensions:** 2, 7, 12, 13. **Related:** AR-21, R9.

**Location:** `03-04-editor-and-release.md:12–27`;
`00-ambiguity-register.md:127–130`; ST-40/ST-43.

**Codebase evidence:** `compiler/src/clorraine/build.rs:65–81` forwards the
supplied filenames; `compiler/src/driver.rs:197–212` reads only those files.
`compiler/src/analysis.rs:225–285` injects embedded standard-library modules,
not arbitrary user files. The existing multi-file oracle explicitly supplies
both paths (`compiler/tests/clorraine_driver_spec_test.rs:435–455`).
`compiler/src/lsp/workspace.rs:187–228` separately discovers transitive imports,
qualified module references and files belonging to selected modules.

**Problem:** The approved action uses one entry and its imports, but its planned
argv contains only the entry. LSP navigation can work while Build/Run fails
with a missing-module diagnostic. Changing CWD cannot supply missing inputs.

| Option | Correction | Benefit | Cost |
|---|---|---|---|
| A | Add one editor-facing LSP request selecting the saved entry's bounded dependency-file set through existing Rust parsing/closure logic; return ordered absolute paths to the unchanged CLI | Delivers approved imports without a second parser | Build requires a compatible LSP; needs explicit-root, saved-file and limit handling |
| B | Explicitly narrow editor actions to entry plus embedded standard-library imports; retain multi-file CLI guidance | Smaller editor change | Withdraws confirmed user-import behavior; requires a scope ruling |

**Recommendation — A:** The extension owns trust, saving and the containing
root; the LSP owns dependency selection from refreshed saved disk contents.
Return absolute paths, entry first, and fail explicitly on incomplete selection
or exceeded limits. Do not reuse live analysis overlays blindly:
`compiler/src/lsp/workspace.rs:191–192,204–228` permits overlays and truncation.
The request must carry its own root because current extension startup uses only
the first workspace (`tools/vscode/src/extension.ts:266–272`).

**Confidence:** High; the added compatible-LSP dependency is the strongest
trade-off. **Hardening:** Refined ownership and disk/root/limit boundaries.
**Challenger:** Converged.

**Required verification:** Real two-file imported-function Build/Run; transitive
and qualified references; same-module files; unrelated programs excluded;
deterministic entry-first argv; saved-source consistency; changed imports after
save; standalone/second-root files; exceeded limits and incompatible/unavailable
LSP. Removing a dependency must fail. Exercise all three actions and compare
LSP discovery with build input.

**User Decision:** Approved Option A; user selected “Approve and apply
(Recommended)” for all six major corrections. Plan-only edits and bounded
re-scan authorised. **Fix state:** Verified — resolved at plan level.

**Iteration 2 residual:** Grounding and risk reviewers found that the phrase
“existing file/byte limits” referenced a nonexistent disk-source byte cap.
`compiler/src/lsp/workspace.rs:20,23` defines only 2,000 indexed files and
256 selected files; its scan truncates after traversal and its reads are
unbounded (`:111–150,258,268`). `protocol.rs:12` caps messages, not disk files.
This was a Minor residual under the same finding, not a new root cause.

**Residual ruling:** The user selected “Apply clarification (Recommended)” for
the explicit 2,000/256 file-count contract, enforcement during traversal and
exact-cap/overflow tests. The lead adopted grounding's minimum correction;
risk's byte-budget suggestion was not adopted as a new source-size policy.
`03-04` §Saved-source selection and ST-59 now state the count boundaries and
complete-closure requirement. Direct verification confirmed both named
constants against source and exact-cap/overflow/traversal coverage against
ST-59. No third broad scan was required for this minor clarification.

### PF-003: Installer gate precedes its package producers 🟠 MAJOR

**Dimensions:** 4, 5, 11, 13. **Related:** AR-17/20/24.

**Location:** `99-execution-plan.md:108,135–151,199–206`;
`03-02-installation.md:8–10,36–37`; `03-03-native-driver.md:104`.

**Codebase evidence:** `tools/setup/install_sdk.py:184–186` uses repository
location; `tools/setup/tests/support.py:13–15` adds checkout modules to
`sys.path`. No existing native packager supplies the planned candidate inputs.

**Problem:** Phase 3 requires real candidate installation, but core packaging
and manifest/bootstrap assembly are first scheduled in dependent Phase 5.
Phase 2's packaged-driver emulator observation is an even earlier consumer
of installed-layout assembly. The verified downloaded helper's module closure
is also unspecified despite
imports spanning setup and distribution modules. Checkout-based tests could
hide missing modules in the shipped helper.

| Option | Correction | Benefit | Cost |
|---|---|---|---|
| A | Schedule local installed-layout/core assembly before Phase 2's packaged-driver test, and complete helper/manifest assembly before Phase 3's installer gate, with specification-first tests | Removes both dependency gaps and tests the delivered bootstrap | Requires task redistribution; automation/version updates remain later |

Moving the whole release phase earlier also moves editor/version/documentation
work before its dependencies. Hand-built test-only layouts would not verify the
production bootstrap. Neither resolves this as directly as moving its actual
local producers.

**Recommendation — A:** Move each local producer before its first real consumer.
Use one hash-pinned standard-library Python zip application (`.pyz`) containing
the installer modules and shared manifest validator. Keep NDK preparation in
the separately verified support payload. Phase 5 automates these same producers
and performs final version/release work.

**Confidence:** High; manual fixtures would be easier initially but could hide
missing shipped modules. **Hardening:** Added the earlier Phase 2 consumer and
specified the helper's packaging boundary. **Challenger:** Converged.

**Required verification:** Run the packaged shell entry with repository and
original SDK inaccessible, `PYTHONPATH` cleared, both Python cohorts, updates
and relocation. A missing helper module must fail package proof; corrupt helper
bytes must prevent execution.

**User Decision:** Approved Option A; same explicit six-major approval and
plan-only fix/re-scan instruction. **Fix state:** Verified — resolved at plan level.

### PF-004: Initial payload publication depends on its own private draft 🟠 MAJOR

**Dimensions:** 3, 5, 12. **Related:** AR-22, R12/R13.

**Location:** `03-04-editor-and-release.md:63–66,88–105`;
`99-execution-plan.md:201–203,234`.

**Codebase evidence:** `RELEASE.md:158–163` promotes the entire draft release.
The [GitHub Releases API](https://docs.github.com/en/rest/releases/releases#update-a-release)
defines `draft: false` on a release; assets have no independent public/draft
state.

**Problem:** New core assets are produced in the distribution draft, but must
be publicly verified before that distribution and its manifest are published.
The first release cannot satisfy both conditions. Later core reuse does not
solve the initial publication. Moving only core binaries is insufficient if
the required VSIX remains private (`03-02-installation.md:71–73`).

| Option | Correction | Benefit | Cost |
|---|---|---|---|
| A | Give immutable compiler/editor payloads a separate component release managed by the existing distribution workflow; publish/verify it before the distribution | Keeps manifest promotion as a complete-release operation; covers core and matching VSIX | Adds a component tag/record and installer lookup exclusion |
| B | Publish the same release's payloads with its manifest withheld; verify payloads, then publish the manifest as activation | Keeps one core/distribution release location | Needs manifest-last lookup rules for default and exact releases, latest designation, interruption and retries |

Both options retain two workflows and immutable released bytes. Neither
authorises publication now.

**Recommendation — A:** Publish the compiler/editor payload release and support
assets only after candidate acceptance and explicit approval; anonymously verify
every referenced payload before promoting the bootstrap/manifest distribution.
Include the matching VSIX and any other manifest-referenced payload otherwise
trapped in the draft. Exclude component tags from installation lookup and
GitHub's latest designation. Keep the two approved workflows.

**Confidence:** Medium-high; the strongest alternative is manifest-last
activation, but it requires more partial-publication/retry rules and depends on
assets remaining appendable after publication. **Hardening:** Expanded the
payload boundary beyond core binaries. **Challenger:** Converged.

**Required verification:** First release, reused-core revision, interrupted
promotion and retry. Private/missing core or required source/notices must block
distribution availability. Lookup must exclude every non-installable release.

**User Decision:** Approved Option A; same explicit six-major approval and
plan-only fix/re-scan instruction. **Fix state:** Verified — resolved at plan level.

### PF-005: Public promotion precedes final acceptance review 🟠 MAJOR

**Dimensions:** 3, 11. **Related:** AR-22/24, R13.

**Location:** `99-execution-plan.md:226–239`;
`07-testing-strategy.md:120–126`.

**Codebase evidence:** `Makefile:211–235` provides full existing verification;
it does not establish candidate-evidence integrity. The editor gate at
`Makefile:92` builds a checkout server, not proof of exact downloaded binaries.

**Problem:** Task 6.2.4 publishes before 6.3.1's evidence/hash controls and
6.3.2's independent acceptance audit. It also requires candidate acceptance
green before publication while ST-56 is a post-publication check. ST-54 already
runs full gates before publication; the missing ordering concerns the final
evidence controls and review.

| Option | Correction | Benefit | Cost |
|---|---|---|---|
| A | Finish evidence-integrity controls, candidate gates and independent review before requesting approval; promote afterwards; run ST-56 as the post-publication completion gate | Makes the release decision depend on complete candidate evidence | Requires splitting and reordering final tasks |

A post-publication-only review cannot prevent an immutable bad release.
Treating anonymous public-install evidence as pre-publication proof is circular.

**Recommendation — A:** Define candidate-green as ST-53 and ST-54 plus all
applicable earlier cases, evidence-integrity controls and independent review.
ST-55 is retired under AR-49 (Linux-only scope). Bind the publication approval to
those reviewed hashes. Then publish, run ST-56, and compare public bytes with
the approved candidate.

**Confidence:** High; some public transport failures are necessarily discoverable
only after publication, which is why ST-56 remains a separate completion gate.
**Hardening:** Clarified the two acceptance boundaries. **Challenger:** Converged.

**Required verification:** Missing licensed-media evidence, changed candidate
hashes, unresolved major findings or absent approval blocks promotion. Missing
ST-56 evidence blocks final completion, not the earlier approval request. Apply
to initial and support-only releases.

**User Decision:** Approved Option A; same explicit six-major approval and
plan-only fix/re-scan instruction. **Fix state:** Verified — resolved at plan level.

### PF-006: Native FS-UAE inherits configuration and shared writable state 🟠 MAJOR

**Dimensions:** 2, 7, 8, 9, 13. **Related:** AR-19, R8.

**Location:** `03-03-native-driver.md:61–78`; ST-35/ST-37/ST-38.

**Codebase evidence:** `docker/scripts/run.sh:110–129` sets model, ROM and
drive 0. `docker/compose.yaml:21–34` does not mount the normal user's FS-UAE
configuration directory. Native execution loses that container separation.
FS-UAE v3.1.66 source:

- [`main.c:669–728`](https://github.com/FrodeSolheim/fs-uae/blob/v3.1.66/src/fs-uae/main.c#L669-L728)
  loads CWD/default configuration and additional `Host.fs-uae`.
- [`config-drives.c:334–380`](https://github.com/FrodeSolheim/fs-uae/blob/v3.1.66/src/fs-uae/config-drives.c#L334-L380)
  mounts additional configured drives, with directory mounts writable by default.
- [`paths.c:287–323`](https://github.com/FrodeSolheim/fs-uae/blob/v3.1.66/src/fs-uae/paths.c#L287-L323)
  derives shared save-state locations from a configuration basename or `Default`.

**Problem:** A user's configuration can mount another drive or change boot
behavior despite the promised single-program mount. Distinct staging drives
alone also do not prevent concurrent runs sharing emulator state. Making a
generated configuration optional does not suppress the additional host config.

| Option | Correction | Benefit | Cost |
|---|---|---|---|
| A | Require explicit per-run configuration, suppress additional config loading, and direct base/state/log output into private per-run storage; verify actual emulator behavior | Completes the approved isolation/lifetime boundary without editing user settings | Needs supported-version checks and hostile-configuration acceptance |

FS-UAE's [`end_config`](https://fs-uae.net/docs/options/end_config) documentation
states that it prevents loading additional files such as `Host.fs-uae`.
The exact argument/configuration ordering and all writable path controls still
need real-version proof. An explicit drive 0 or a clean test home alone does not
establish isolation.

**Recommendation — A:** Require explicit private configuration containing
`end_config=1`; supply private base/cache/state/log settings early enough to
govern emulator initialization. Keep configuration, guest drive and writable
state within the invocation's run directory. Record real emulator versions and
prove effective behavior in the supported acceptance environments.

**Confidence:** High on the defect, medium on cross-version sufficiency until
runtime proof. Isolation also removes user preferences; retaining those
preferences cannot override the approved single-drive/private-state boundary.
**Hardening:** Added early initialization and cache/state path coverage.
**Challenger:** Converged.

**Required verification:** Conflicting CWD `Config.fs-uae`, user
`Default.fs-uae` and `Host.fs-uae`, extra writable drive and boot overrides.
Inspect effective mounts/model/ROM; outside sentinels must remain unchanged.
Concurrent runs need distinct state/log paths. Removing isolation must expose
the hostile fixture. Preserve Docker handoff behavior.

**User Decision:** Approved Option A; same explicit six-major approval and
plan-only fix/re-scan instruction. **Fix state:** Verified — resolved at plan level.

### PF-007: Full verification lacks a private-display route 🟠 MAJOR

**Dimensions:** 2, 7, 11, 13. **Related:** AR-24, ST-54.

**Location:** `07-testing-strategy.md:124`;
first full gate at `99-execution-plan.md:84`.

**Codebase evidence:** `Makefile:233` calls `corpus-fs-uae`;
`docker/scripts/corpus-fs-uae.sh:58` calls `run.sh`.
`docker/scripts/run.sh:24–42` reloads `.env` without preserving caller
`DISPLAY`/`XAUTHORITY`; `docker/scripts/emulator.sh:19–22` reloads it again.
`docker/compose.yaml:28–34` consumes the resulting display settings.

**Problem:** Setting a private outer `DISPLAY` does not keep all legacy gate
windows off the desktop. The plan requires private displays at every full gate
but schedules no working route before Phase 1's first gate.

| Option | Correction | Benefit | Cost |
|---|---|---|---|
| A | Define a disposable verification checkout/configuration with only private Xvfb values in its Docker environment | Preserves current wrapper behavior and primary local settings | Every full gate needs a faithful candidate snapshot and isolated configuration |
| B | Plan narrow caller `DISPLAY`/`XAUTHORITY` preservation in both wrappers, with conflict tests before the first full gate | Reuses the existing environment-override pattern and makes private caller settings effective | Adds two legacy wrapper changes and compatibility tests |

**Recommendation — B:** Plan caller-display preservation in both wrappers,
following `docker/scripts/a500-smoke.sh:51–60`. Schedule specification tests and
the narrow fix before any full gate, including infrastructure-change gates.
Use a private Xvfb/authentication pair without editing the primary user `.env`.

**Confidence:** High; a disposable checkout avoids legacy changes but creates
recurring candidate/configuration coordination. **Hardening:** Confirmed the
existing override idiom and the earliest gate deadline. **Challenger:** Converged.

**Required verification:** Deliberately conflicting desktop values in `.env`;
inspect actual emulator environment and authentication path, not just the outer
shell. Cover the full corpus chain and unchanged no-override defaults. Keep
emulator observations on private displays.

**User Decision:** Approved Option B; same explicit six-major approval and
plan-only fix/re-scan instruction. **Fix state:** Verified — resolved at plan level.

## Minor findings

### PF-002: Native emulator flags have no legacy-mode outcome 🟡 MINOR

**Dimensions:** 1, 9, 13. **Related:** AR-19/29, R7/R8.

**Location:** `03-03-native-driver.md:19–33,88`; ST-31/ST-34.

**Codebase evidence:** `compiler/src/clorraine/toolchain.rs:295–308` constructs
the fixed Docker handoff without emulator/ROM override inputs.
`compiler/tests/clorraine_driver_spec_test.rs:305–355` pins that handoff.

**Problem:** An Amiga `run --emulator --fs-uae ...` or `--kickstart ...` can
resolve only the legacy SDK. The plan does not explicitly state the combined
case. The general inapplicable-option rule and fixed SDK dispatch already
constrain this to a local clarification.

**Recommendation — A:** After SDK-mode resolution, reject explicit FS-UAE/ROM
flags in legacy mode with exit 2, before building. Explain that the override
requires native SDK selection. Ignore unused emulator environment values;
preserve no-override handoffs.

Silent ignore violates explicit selection. Docker override forwarding or an
implicit mode switch would need a different compatibility contract; neither is
a smaller correction within the approved behavior.

**Required verification:** Both Amiga profiles, both flags separately/together,
with/without capture and conflicting environment values. Assert exit 2 and no
build/emulator launch; retain exact no-override handoff assertions.

**Confidence:** High on correction, medium on severity. **Hardening:** The lead
accepted the challenger's downgrade from Major to Minor because this is a narrow
omission under an existing applicability rule. **Challenger:** Diverged on
severity, converged on correction. It remains an unresolved finding.

**User Decision:** Approved Option A; user selected “Approve and apply
(Recommended)” for PF-002, PF-008 and PF-009, authorising all nine plan
corrections and the bounded re-scan. **Fix state:** Verified — resolved at plan level.

### PF-008: Packaged vamos lacks a nonzero guest-status test 🟡 MINOR

**Dimension:** 7. **Related:** AR-26, R3/R7.

**Location:** `07-testing-strategy.md:56–59,92,123` (ST-6/ST-8/ST-32/ST-53);
`03-03-native-driver.md:89`.

**Codebase evidence:** `compiler/src/clorraine/run.rs:125–127` propagates runner
status, but `compiler/tests/clorraine_driver_spec_test.rs:295–303` tests exit 7
only on host. The portable corpus is zero-exit
(`docker/scripts/corpus-amiga.sh:35–36`); the fault gate invokes container
vamos directly (`docker/scripts/run-fault-amiga.sh:72–84`).

**Problem:** A new Python launcher could preserve output but discard the vamos
return code. Successful-output tests and the separate FS-UAE exit test would
not detect it.

**Recommendation — A:** Extend packaged ST-32 acceptance with a printing
program returning 7 for both Amiga profiles and Python cohorts, with/without
capture. Assert status and output; a launcher forcing zero must fail the test.
Reuse the existing fixture pattern. A second testing framework is unnecessary.

**User Decision:** Approved Option A; same explicit three-minor approval and
plan-only fix/re-scan instruction. **Fix state:** Verified — resolved at plan level.

### PF-009: The claimed existing `--` terminator does not exist 🟡 MINOR

**Dimensions:** 12, 13. **Related:** R7 compatibility.

**Location:** `03-03-native-driver.md:16–17`.

**Codebase evidence:** `compiler/src/clorraine/cli.rs:248–277` and
`compiler/src/driver.rs:103–106` reject `--`. The grounding reviewer also ran
`target/debug/clorraine run -- examples/hello/hello.clo`: exit 2,
`usage error: unknown option '--'`.

**Problem:** An implementer could insert the promised delimiter into editor
argv and break otherwise valid commands.

**Recommendation — A:** Remove the inaccurate preservation claim. Keep current
parsing and use absolute or `./`-prefixed paths where needed. Test constructed
argv against the actual driver. New delimiter semantics would require a
separate end-to-end contract and are unnecessary for this delivery.

**User Decision:** Approved Option A; same explicit three-minor approval and
plan-only fix/re-scan instruction. **Fix state:** Verified — resolved at plan level.

## Review provenance and next boundary

| Review | Session | Merged result |
|---|---|---|
| Document soundness | `ses_e1b9001b9ffe7L3q4sFIqX3b2X` | PF-001, PF-002, PF-004, PF-009 |
| Codebase grounding | `ses_e1b8fc93bffeCCfXfMYDnVejhC` | PF-001, PF-009 |
| Delivery sequencing | `ses_e1b8f94c3ffeD7S4zvahZSSFWW` | PF-003, PF-004, PF-005 |
| Risk/migration/concurrency | `ses_e1b8f629cffevyA3n337sCOLfS` | PF-006 |
| Testability/scope | `ses_e1b8f36f2ffeQqG31fQA4JCCLN` | PF-008; no scope expansion found |
| Amiga platform | `ses_e1b8efb25ffeVFhl3I6O4OAF6q` | PF-006, PF-007 |
| Independent recommendation challenge | `ses_e1b88ce83ffeweLPbO2gXr4VFR` | Six Majors confirmed; PF-002 downgraded to Minor; recommendations reconciled |

Adversarial synthesis tested three author assumptions: that LSP source discovery
also applies to CLI actions; that draft assets can be published independently;
and that a Docker emulator recipe remains isolated when launched natively.
The findings above reject those assumptions using code and external behavior.
The challenge refined PF-001's selection ownership, PF-003's earliest consumer
and helper payload, PF-004's VSIX boundary, and PF-006's writable-state boundary.
It found no need for a third workflow or a second source parser.

## Iteration 2 correction evidence

| Finding | Applied correction and current owner | Verification |
|---|---|---|
| PF-001 | `03-04` §Saved-source selection; ST-40–ST-43/ST-59; Phase 4 Rust/editor spec-first tasks | Soundness/grounding/fit; minor count clarification directly verified |
| PF-002 | `03-03` §Integration surface / Result matrix; ST-33 | Soundness |
| PF-003 | `03-02` §Entry and trust; `03-04` §Payload identities and local producers; ST-57/ST-58; Phase 2/3 producers before green | Delivery/risk/fit |
| PF-004 | `03-02` §Metadata contract; `03-04` §Payload identities / Draft acceptance; ST-13/ST-49–ST-51 | Soundness/delivery/risk |
| PF-005 | `03-04` §Draft acceptance; `07` §Oracle and verification; reordered Phase 6 controls/review/approval/public gate | Delivery/risk/fit |
| PF-006 | `03-03` §FS-UAE process; ST-35/ST-37/ST-38 | Risk/fit/Amiga |
| PF-007 | `03-01` §Private-display gate prerequisite; ST-60; first Phase 1 implementation tasks | Delivery/risk/fit/Amiga |
| PF-008 | ST-32/ST-53, both profiles/Python cohorts/capture variants and forced-zero control | Fit |
| PF-009 | `03-03` §Integration surface; ST-43 real-driver argv check | Soundness/grounding |

The user authorised all nine plan edits and bounded re-scan in two explicit
question responses. No source implementation, publication or commit is
authorised by those responses. All nine planning corrections are now verified.

### Iteration 2 independent results

The same six read-only review sessions checked the independent re-scan hash.

| Cluster | Dimensions/lens | Result |
|---|---|---|
| Soundness | 1, 3, 12 | PF-001/PF-002/PF-004/PF-009 closed; no new findings |
| Grounding | 2, 13; compiler | PF-009 closed; PF-001 major corrected, minor file-limit reference clarified under explicit user ruling |
| Delivery | 4, 5, 11 | PF-003/PF-004/PF-005 closed; PF-007 ordering verified; no new findings |
| Risk | 6, 8, 9; migration/concurrency | PF-006 closed; same PF-001 minor reference residual; no independent new findings |
| Fit | 7, 10 | PF-008 closed; testability/scope and 60-case/89-task ownership verified; no new findings |
| Amiga specialist | Platform | No findings |

**Final verdict:** Passed. No unresolved major/minor findings and no accepted-risk
waivers. The last count-bound clarification was checked directly, without
another broad review cycle. The passed hash covers the current ten-document
target. This is implementation-plan readiness, not native-package acceptance.
