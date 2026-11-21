# Execution plan: native Linux delivery

> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-11-21 20:45
> **Progress**: 80/91 tasks (88%)
> **CodeOps Artifact Schema**: 1

## Execution contract

The user authorised Phase 1 execution with “proceed” on 2026-11-16 after the
preflight handoff. The user then selected “Continue; commit at end”: do not
commit or ask again until the plan completes. Public promotion retains its
separate AR-22 approval boundary.

On Phase3 acceptance the user directs continuing without trivial questions.
Make routine in-scope choices directly. Material uncertainty, scope expansion
and mandatory review/publication decisions retain their recorded gates; no
auto-design or commit authority is inferred.

Execution entry verified all ten documents against the passed target hash
`bd89221f668746a31d6235269f1eb29a81159db77372c4dbd54ed2ef7e5f540a`.
The active register is resolved, all nine preflight findings are closed, and
the approved support surfaces and specification-first order are intact.
Subsequent checklist/evidence updates are execution state, not a new preflight.

The task lines below are the sole mutable progress record. After implementing
a task mark `[~]` with a real timestamp; after its verification passes mark `[x]`
and update the Progress/Last Updated header immediately. Only `[x]` counts as
complete. Resume the first `[~]`, otherwise the first `[ ]`. Mark `[!]` and
append a short blocker when work cannot continue.

Record each phase baseline tree through the exec-plan skill. Follow its
commit mode; use the git-commit skill for checkpoints when authorised.
Do not combine spec-test authoring with implementation. Expected red failures
are recorded in the red task; compatibility cases already green need a reason.
No test may be weakened to obtain green.

**Task verify**: `make check` plus the declared specification files in
[07 §Test slices](07-testing-strategy.md#test-slices-and-declared-files).
**Task verify evidence**: 4,232 Rust + 160 Python tests, 2026-11-16,
baseline `daa2fba0`; fuzzer smoke 28 cases, zero failures.
**Verify (gate)**: `make verify`.
Editor tasks also run `make vscode-check`; site tasks run `make site-check`.
Shared manifests/test/build configuration changes select full verification.
First establish the private-display prerequisite in Phase 1 using non-launching
red tests, both wrapper fixes and real private-environment proof. No full gate,
including an infrastructure-change gate, may run before that prerequisite.
All subsequent full gates use the recorded private Xvfb/authentication pair.

## Phases

| Phase | Title | Tasks | Dependency |
|---|---|---|---|
| 1 | Private gates, support lock and clean-machine proof | 20 | None |
| 2 | Native driver and local core packaging | 16 | 1 |
| 3 | Installer, bootstrap packaging and atomic updates | 17 | 1, 2 |
| 4 | VS Code actions and saved-source selection | 14 | 2, 3 |
| 5 | Release packaging and bounded workflows | 13 | 1–4 |
| 6 | Candidate acceptance and authorised promotion | 11 | 1–5 |

**Total: 91 tasks across 6 phases and 18 ordered work sessions.**
Tasks are one reviewable concern, normally 1–3 files. Split an implementation
task if it grows past that boundary or combines unrelated concerns; preserve
spec-first order and update counts before execution. No build-duration estimate
is inferred from the planning research.

## Phase 1: Support lock and clean-machine proof

> **Phase baseline tree**: c584fb9aaf5512c6eaea0bac28741d6172553e50
> **Reasoning**: max — transitive input, licence and native ABI proof
> **Lenses**: correctness, supply-chain security, Amiga platform
> **Scope mode**: strict
> **Expected modification set**: `tools/distribution/`, `compiler/tests/native_gate_spec_test.rs`, `docker/scripts/run.sh`, `docker/scripts/emulator.sh`, `THIRD_PARTY_NOTICES.md`, `Makefile`, `.gitignore` (only the exact libamiga input exception), this plan's execution/evidence records and approved runtime amendments, and `plans/00-roadmap.md`

The baseline includes the uncommitted planning documents, roadmap and approved
driver-requirements amendment. Those pre-existing edits are not Phase 1 work.

**Execution-entry verification:** `make check` passed on 2026-11-16: 4,232 Rust
tests, Python suites 29 + 103 + 28, and fuzzer smoke
`cases=28 host=28 sanitizer=16 target=16 failures=0`.
This is the unchanged implementation baseline. AR-30 initially blocked task
1.1.1 before any tests were written. The user approved the interface amendment;
`03-01` now supplies the executable lock/source/build/payload contract.
The entry log and owned scratch directory were removed after recording evidence.

Task 1.1.1's authoring verification and red evidence are recorded in `07`;
its verification logs and temporary fixtures were cleaned. The read-only
progress helper derives the correct 1/89 count and next task 1.1.2, but exits 1
because it does not accept this approved standalone plan's `Implements: —`.
No requirement ID was invented to satisfy that helper.

Reference: [03-01](03-01-support-builds.md); AR-18/24/27/28.
This is a stop gate: unresolved feasibility prevents dependent phases.

### Session 1.1: Specification tests

- [x] 1.1.1 [spec-author] Write lock/payload cases ST-1/ST-2/ST-10 — `tools/distribution/tests/test_support_spec.py`; AR-30 executable contract. ✅ (completed: 2026-11-16 22:20; authoring verified: 24 expected-red tests, no fixture errors/skips; `make check` passed)
- [x] 1.1.2 [spec-author] Add preparation/wheel/portability cases ST-3–ST-9 to the same suite, with real-artifact acceptance entry points. ✅ (completed: 2026-11-17 00:01; 18 local tests and 5 mandatory real-artifact methods authored; expected-red/static checks and `make check` verified)
- [x] 1.1.3 [spec-author] Write private-display precedence case ST-60 with copied configs and logging process fixtures — `compiler/tests/native_gate_spec_test.rs`; no desktop launch. ✅ (completed: 2026-11-17 00:25; 2 default controls green, 6 overrides red; `make check` fails only the new suite after 3,244 passing Rust tests; remaining legs await implementation)
- [x] 1.1.4 Run support and private-display specs red; record unmet assertions and existing green controls — `07-testing-strategy.md`. ✅ (completed: 2026-11-17 00:26; consolidated red rerun and immutable oracle hashes recorded; no implementation yet)

### Session 1.2: Implementation and proof

- [x] 1.2.1 Preserve caller display/authentication through both wrapper `.env` loads — `docker/scripts/run.sh`, `emulator.sh`; ST-60. ✅ (completed: 2026-11-17 00:53; 8 unchanged tests, shell lint, direct/wrapped real private-display proof and full `make verify` passed)
- [x] 1.2.2 Run ST-60 green and prove effective direct/wrapped Docker/FS-UAE environment on private Xvfb; record setup before any full gate — `tools/distribution/README.md`. ✅ (completed: 2026-11-17 01:00; actual proof/full gate recorded, guide syntax validated, `make check` passed: 4,240 Rust + 160 Python tests and fuzz 28)
- [x] 1.2.3 Resolve immutable source/build-environment inputs and write the reference recipes — `tools/distribution/README.md`, `tools/distribution/inputs/`; AR-31 exact LHA candidate and AR-32 base/recipe contract. No eligible partial lock. ✅ (completed: 2026-11-17 16:02; input identities/bytes, recipe authoring/static/guard checks and `make check` verified; unchanged support specs expected red; actual environment/reference build and final lock remain 1.2.5)
- [x] 1.2.4 Record per-component licence/source obligations and payload origin audit — `tools/distribution/README.md`, `THIRD_PARTY_NOTICES.md`; ST-10. ✅ (completed: 2026-11-17 22:10; source/notice identities, install-origin/exclusion trace, zero-fuzz source-form reconstruction and `make check` verified; support oracle unchanged/expected red; fresh reference inventory and delivered source/payload acceptance remain 1.2.5/1.2.10/1.2.11, libamiga legal review user-owned)
- [x] 1.2.5 Complete the fixed-input reference environment/tree proof and measured inventories; author the approved standalone-object oracle amendment before affected implementation — `tools/distribution/README.md`, `tools/distribution/tests/`; AR-32/34. ✅ (completed: 2026-11-18 13:09; offline reference/receipts, measured tree, 52 ELF files, 166 archives, all 26 standalone objects and both strict-C99 profile links verified; approved 55-method oracle amendment/red gate and `make check` pass; actual packaging/acceptance and final lock remain later tasks)
- [x] 1.2.6 Implement exact source staging and verification — `tools/distribution/support.py`; ST-1/ST-2; approved package/source oracle ran red before code; AR-36 raw identity tests preceded staging changes. ✅ (completed: 2026-11-18 15:19; all owned public-command cases, 12/12 actual source checks, bounded schema/security/resource probes, Python 3.10/3.12 and `make check` pass; canonical fd2pragma helper rebuild byte-identical to preserved reference; full support suite still expected red only for absent sibling commands/required real-artifact setup; no final-lock/packaging acceptance)
- [x] 1.2.7 Implement baseline support build/provenance and its tracked payload recipe using the proved reference commands and explicit execution image ID — `tools/distribution/build-support.sh`, `tools/distribution/inputs/`; author/verify producers before final lock, no eligible partial-lock claim. ✅ (completed: 2026-11-18 15:39; nine new spec cases red/green as applicable, unchanged owned-command cases, real Docker synthetic success/blocked-fetch cleanup, actual 423-file/11-link selection, both C99 runtime profiles ordinary/LTO, Python 3.10/3.12, ShellCheck and `make check` pass; complete real locked build and packaging remain later tasks)
- [x] 1.2.8 Implement standalone NDK preparation and its required tracked inputs with prebuilt helpers — `tools/distribution/prepare_ndk.py`, `tools/distribution/inputs/`; ST-4/ST-5; verify fixtures/manual fixed-input transformations before final lock. ✅ (completed: 2026-11-18 16:28; unchanged oracle red first, 83 authenticated resources, exact 1,372-file reproduction/baseline preservation, standalone synthetic-lock marker/repeat/space-path proof, Python 3.10/3.12, bounded member probes, shell/Python checks and `make check` pass; unused fd2pragma binary omitted from refined candidate; real-lock/archive acceptance remains later)
- [x] 1.2.9 Implement the CPython 3.10/3.12 wheel producer/recipe and manually prove fixed-input offline wheel builds with complete dependency metadata — `tools/distribution/build-wheels.sh`, `tools/distribution/inputs/`; ST-6/ST-7; real locked-command execution follows 1.2.10. ✅ (completed: 2026-11-18 17:22; nine new cases red before code, both actual five-wheel ABI builds, offline fresh-venv installs/dependency checks, native imports/ELF audits, nix13/nix20 hello/exit7, Python 3.10/3.12 focused suites, shell lint and `make check` pass; full discovery expected red only for packaging/mandatory artifacts; no real release-lock/archive acceptance claim)
- [x] 1.2.10 Finalise/validate the complete real lock after all hashed producers/reviews/reference inventories exist, then execute real public source/build/preparation/wheel commands against it — `tools/distribution/support-lock.json`, `tools/distribution/README.md`; AR-32/34; no placeholders or manual/fixture inputs represented as an eligible lock. ✅ (completed: 2026-11-18 19:07; real lock `8529d78d…`, public source/C/both-wheel commands, actual inventories/provenance and standalone preparation on both Ubuntu releases pass; 1,372 overlays/markers/repeat/baseline checks, AR-37 controls/reproof, canonical Git guard controls, shell/Python checks and `make check` pass; original/superseded evidence preserved; packaging/compliance/archive acceptance remain later)
- [x] 1.2.11 Implement payload packaging/loader audit and notices/source assembly — `tools/distribution/package_support.py`; ST-3/ST-10, including the approved Amiga-object contract. ✅ (completed: 2026-11-18 20:18; packaging oracle red first and wheel-binding regression red/green; all 86 local methods on both builder Pythons, four real assembly/native/source/notice audits and public deterministic/safe/no-clobber archives, full assembled preparation on both Ubuntu releases, 15 security/resource/native probes and `make check` pass; actual clean-archive corpus/relocation acceptance remains next; no legal-clearance/publication claim)
- [x] 1.2.12 Execute clean Ubuntu and relocation proof using actual support archives; record closed prerequisites — `tools/distribution/README.md`; ST-3–ST-9. ✅ (completed: 2026-11-18 23:51; all six unchanged artifact + five frozen GCC specs methods pass on actual isolated Ubuntu 22.04/cp310 and 24.04/cp312 against `aca1a0ec…` archives, including native closure/imports, strict corpus before/after real spaced-root relocation and fresh preparation; private source patch/reconstruction/source delivery verified, original staged GCC/artifact oracle unchanged; task `make check` and static controls pass)
- [x] 1.2.13 Run all support specification cases green and retain source/archive evidence — `tools/distribution/tests/test_support_spec.py`. ✅ (completed: 2026-11-19 00:07; all 104 distinct methods pass in their required contexts: 93 local on host/cp310/cp312 and eleven actual archive/GCC methods per Ubuntu cohort; current lock/104 inputs, actual archives, source/oracle/emitter/fixture identities and 28 strict corpus pairs per release rechecked; task `make check` passes, named receipts retained and owned scratch cleaned; phase full gate/review remain)

### Session 1.3: Hardening and gate

- [x] 1.3.1 Add focused lock/inventory/preparation implementation tests — `tools/distribution/tests/test_support_impl.py`. ✅ (completed: 2026-11-19 00:16; 17 implementation tests plus 93 unchanged local specs pass on host/cp310/cp312, three scratch-only mutation controls fail as intended, task `make check` passes: 4,240 Rust + 160 Python tests and fuzz 28 zero failures; syntax/documentation checks pass, owned scratch cleaned and named receipt retained; no production/lock/specification changes)
- [x] 1.3.2 Connect distribution unit tests to existing Makefile checks without making licensed/real-machine tests silently optional — `Makefile`; 07 §Test slices. ✅ (completed: 2026-11-19 00:42; two wiring controls red/green, all 113 local tests pass via `make check`; missing real inputs remain explicit errors, never skips; full authenticated-private-display `make verify` passes all legs, including twelve UBSan suites; documentation checks and owned scratch/display/cookie cleanup complete, named gate receipt retained)
- [x] 1.3.3 Run full gate and independent correctness/security/Amiga review; resolve required findings before Phase 2 — `99-execution-plan.md`. ✅ (completed: 2026-11-19 01:05; full authenticated-private-display gate and both actual Ubuntu eleven-method proofs pass; all four independent reviews complete, zero critical/major findings; RV-001/SA-001/SA-002 remain minor deferred/report-only in findings ledger; all 130 reviewed non-plan files still match head `07350719…`; named review receipt retained and all owned review scratch cleaned; Phase 1 verified, no legal/publication claim)

**Verify (gate)**: `make verify` on private display; ST-60 and real support proof ST-3–ST-10.

**Runtime ordering amendment:** AR-32 adds one reference/lock-finalisation
task and separates immutable input research from eligible-lock production.
The six verified task IDs are unchanged. Original authoring evidence below
the passed preflight remains historical; that amendment produced 90 tasks.
AR-34 subsequently splits final-lock completion for the current total of 91.
The approved image-contract test amendment must be authored and rechecked
before the affected support implementation starts.

**Archive inclusion directive:** AR-33's later explicit user instruction lifts
the implementation hold for the exact ordinary libamiga archive. Preserve its
hash and provenance; the user owns its legal review under DEF-560. This is not
evidence of redistribution permission or matching corresponding source.
Other component reviews and all technical proof gates remain required.
Public promotion still needs separate explicit approval.

**Approved reference/producer ordering:** AR-34 explicitly approves the
standalone Amiga-object contract/test amendment and the split between reference
proof (1.2.5) and final real-lock completion (1.2.10). The intervening tasks
produce their real hashed scripts/recipes and use specification fixtures or
explicit manual fixed-input proof, not an eligible partial lock. Real public
commands run against the completed lock before packaging and acceptance.
This adds one task (91 total); the eight verified task IDs are unchanged.
Only the still-unstarted late Session 1.2 tasks were renumbered.
The approved object tests must be authored/red-verified before implementation.

**Approved package-name correction:** AR-35 permits actual unqualified Debian
names only in package maps/provider references. Source/helper/tool IDs remain
strict. Author and verify the narrow oracle amendment red before validation
implementation, then resume 1.2.6. No source pin or dependency changed.

**Approved source identity:** AR-36 selects canonical authenticated blob bytes,
not Git-ident expansion, in source staging. Keep verification read-only and
strict; add raw-positive/expanded-negative cases before changing staging.
Preserve original acquired/reference evidence and re-prove the affected
helper build. No source pin, dependency or public interface changes.

## Phase 2: Native driver and FS-UAE

> **Phase baseline tree**: cadcbce6fc84c9c88701ff16b0080018908edefb
> **Reasoning**: high — native/container compatibility and process lifetime
> **Lenses**: API, correctness, security, concurrency, Amiga platform
> **Scope mode**: strict
> **Expected modification set**: `compiler/src/clorraine/`, `compiler/tests/native_driver_spec_test.rs`, `compiler/tests/native_driver_impl_test.rs`, test-only native-driver fixture modules under `compiler/tests/`, `tools/distribution/package_core.py`, `tools/distribution/tests/test_core_package_spec.py`, test-only core-package fixture modules under `tools/distribution/tests/`, AR-42's explicitly approved `compiler/src/lsp_main.rs` and `compiler/tests/native_lsp_version_spec_test.rs`, this plan's execution/evidence records and approved runtime amendments, and `plans/00-roadmap.md`

The baseline includes all uncommitted Phase 1 work and its closure records.
It is not a commit and does not include that earlier work in the Phase 2 diff.
Execution resumes with task 2.1.1 after checking the ten required documents,
resolved register, preflight closure and the retained four-review receipt.
RV-001/SA-001/SA-002 remain minor deferred/report-only; no fix is inferred.
Fresh `make check` passes with 4,240 Rust tests, Python suites 29 + 113 + 103 +
28 and fuzzer smoke 28, zero failures. AR-40's authoring-readiness stop is
resolved by the user's explicit interface-amendment approval. The concrete
installed-layout and local-producer contracts now precede independent tests;
no implementation was written during that stop.

Reference: [03-03](03-03-native-driver.md); AR-19/29.

### Session 2.1: Specification tests

- [x] 2.1.1 [spec-author] Write resolution/compatibility cases ST-26–ST-32/ST-39 — `compiler/tests/native_driver_spec_test.rs`; AR-40 concrete installed-layout/runner amendment approved. ✅ (completed: 2026-11-19 02:55; parent authoring verification confirms 59 methods, 53 behavioral red/6 justified compatibility green, no fixture/syntax failures; unchanged historical driver 22/22 green; make check fails only new native suite as expected; all test/helper documentation reviewed, oracle hashes/commands retained in native-driver-authoring-proof.json and owned verification scratch cleaned; no native implementation yet)
- [x] 2.1.2 [spec-author] Add emulator argument/status/lifetime cases ST-33–ST-38 to the native suite; define private-display acceptance steps for ST-37. ✅ (completed: 2026-11-19 03:39; parent confirms native 94 = 88 expected behavioral red/6 justified compatibility green; make check fails only native suite, historical 22/22 green; prior five oracle modules byte-unchanged; all new tests/helpers reviewed, syntax/doc/link checks pass; actual private-display acceptance procedure recorded but not executed; named native-emulator-authoring-proof.json retained, owned verification logs cleaned)
- [x] 2.1.3 [spec-author] Write local core/archive/installed-layout case ST-57 — `tools/distribution/tests/test_core_package_spec.py`; AR-40 public local producer/fixture contract. ✅ (completed: 2026-11-19 05:24; 37 methods, parent confirms 23 local expected-red/52 absent-producer records and two explicit missing-context errors for fourteen unrun actual methods; fixture-only ETXTBSY correction independently passes 1,440 stressed host/SDK executions, preserving assertions/barriers; make check restored to native 88 expected red/6 compatibility green; source/doc checks pass, core-package-authoring-proof.json/native-fixture-publication-proof.json retained, all owned scratch cleaned; no producer implementation or actual package acceptance)
- [x] 2.1.4 Run native/core-package suites red and historical driver suite unchanged; record per-case results — `07-testing-strategy.md`. ✅ (completed: 2026-11-19 05:30; fresh native 94 = 88 expected behavioral red/6 compatibility green, historical 22/22 green, core 23 local methods/52 absent-producer records and two explicit missing-context errors for fourteen unrun actual methods; make check fails only expected native feature; all sixteen oracle hashes unchanged, docs/diff pass; phase2-spec-red-proof.json retained, all owned scratch cleaned; specification-first gate satisfied, no product acceptance claim)

### Session 2.2: Implementation

- [x] 2.2.1 Add CLI fields, parsing, validation and help — `compiler/src/clorraine/cli.rs`; 03-03 §Integration surface. ✅ (completed: 2026-11-19 09:22; complete native 96/96 and make check green; source checks and frozen oracle identities pass; native-emulator-implementation-proof.json)
- [x] 2.2.2 Implement SDK selection and canonical invocation context — `compiler/src/clorraine/environment.rs`; ST-26–ST-29. ✅ (completed: 2026-11-19 09:22; complete native 96/96, activation/relocation/receipt rejection and make check green; focused native_metadata.rs reuses public JSON model; no oracle changes)
- [x] 2.2.3 Thread resolved compiler paths through build commands — `compiler/src/clorraine/build.rs`, `toolchain.rs`; ST-30/ST-32. ✅ (completed: 2026-11-19 09:22; complete native 96/96 and make check green, both profiles/modes and freestanding host leg retain existing vectors)
- [x] 2.2.4 Thread paired vamos paths through run and environment checks — `compiler/src/clorraine/run.rs`, `mod.rs`; ST-26/ST-29/ST-39. ✅ (completed: 2026-11-19 09:22; complete native 96/96 and make check green; canonical context, isolated venv-spelled Python argv, status/capture and error controls pass)
- [x] 2.2.5 Implement private emulator configuration, drive staging and base/cache/state/log isolation — `compiler/src/clorraine/emulator.rs`; ST-34/ST-35/ST-38. ✅ (completed: 2026-11-19 09:22; additive oracle red before implementation, then native 96/96 and make check green; mode-0700 independent roots, fixed guest files, literal early path controls, isolated initialization and private trailing-space ROM copies verified with external observers; real version-specific boot/isolation still required by 2.2.8)
- [x] 2.2.6 Implement emulator wait/status/cleanup and native-versus-legacy dispatch — `compiler/src/clorraine/emulator.rs`, `mod.rs`; ST-31/ST-33/ST-36. ✅ (completed: 2026-11-19 09:22; native 96/96 and make check green; pre-build media/tool rejection, child 0/7/signal/launch failure, concurrent rebuild/media lifetime and post-stop cleanup controls pass; legacy historical 22/22 unchanged; no guest-boot acceptance claimed)
- [x] 2.2.7 Implement local core archive and installed-layout staging from current binaries/proved support, without version bump or publication — `tools/distribution/package_core.py`; ST-57. ✅ (completed: 2026-11-19 10:03; local core23/23, additive LSP3/3 and historical LSP32/32, make check green; actual archive/stage twelve methods plus checkout-free runtime/relocation two methods pass on each Ubuntu22/cp310 and Ubuntu24/cp312 cohort; same Ubuntu22-built 1.1.2 core archive ada0742cb72a98cab0b4fde50a15f2ecc56030b26c4b3983cf5569494ca9b98d, original inputs unchanged; core-package-implementation-proof.json retained, completed-task logs cleaned; staged candidates retained for real native FS-UAE/phase proof, no publication)
- [x] 2.2.8 Run native/core-package specs green, including packaged exit-7 and real A1200/KS3.1 hostile-config isolation proof on private display — native and core-package suites; ST-32/ST-37/ST-57. ✅ (completed: 2026-11-19 10:25; both actual core/runtime cohorts green; packaged FS-UAE3.2.35 both-profile guest Hello directly inspected; effective A1200/KS3.1/single CNEXT/private initialization/cache/state/logs traced, real concurrent rebuild/independent cleanup, hostile negative control and SIGTERM/SIGKILL survivor/stale safety pass; renewed make check green, source/oracle identities unchanged; phase2-native-acceptance-proof.json links exact retained evidence; all owned scratch/media/auth/processes cleaned; full phase gate/reviews still pending)

### Session 2.3: Hardening and gate

- [x] 2.3.1 Add path/argv/error implementation tests — `compiler/tests/native_driver_impl_test.rs`; 03-03 §Result matrix. ✅ (completed: 2026-11-19 10:37; six implementation-only receipt-token/relative-path/temp-scope/initialization-Xauthority controls and exact-current make check pass; final assertion had postdated the first compiled test, so complete task gate rerun before promotion; frozen helpers/oracles unchanged; native-path-hardening-proof.json retained, all owned scratch cleaned)
- [x] 2.3.2 Add activation/cleanup race controls to the implementation suite — ST-29/ST-38. ✅ (completed: 2026-11-19 10:45; all eight implementation methods and make check pass, covering both-profile environment activation and stale-directory/alias allocation/cleanup; initial new test's profile-specific helper call corrected to existing profile-aware helper, frozen oracles unchanged; native-race-hardening-proof.json retained and all owned scratch cleaned)
- [x] 2.3.3 Check every build/run caller, profile and diagnostic/capture branch; rerun historical driver oracle — `compiler/src/clorraine/`. ✅ (completed: 2026-11-19 11:09; complete call-site/profile/result matrix audited; full make verify reruns historical driver22, native96, implementation8 and LSP35 green; 4,681 Rust instances including UBSan, every SDK/emulator/differential/fault leg passes; phase2-full-verify-proof.json retained, all gate scratch/media/auth/server/logs cleaned)
- [x] 2.3.4 Run full gate and independent correctness/concurrency/Amiga review — `99-execution-plan.md`. ✅ (completed: 2026-11-19 15:12; all four initial/fix reviews recorded, approved major corrections and narrow AR-43 oracle amendment verified; bounded RV-301 final diff explicitly accepted by user with Phase 2 closure; independent red/targeted/actual14 per cohort/packaged alias+Hello+rebuild and full private make verify green, 4,695 Rust instances; source/diff identities rechecked, all owned scratch/media/auth/build/processes cleaned; no third review/commit/publication; minors remain report-only)

**Verify (gate)**: `make verify`; native FS-UAE observation ST-37.

## Phase 3: Installer and atomic updates

> **Phase baseline tree**: 1ecded4acc2c3196b670ef6edf2c1856c677dd2a
> **Reasoning**: max — untrusted archives and durable activation state
> **Lenses**: security, data/migration, concurrency
> **Scope mode**: strict
> **Expected modification set**: `tools/setup/install.sh`, `tools/setup/install_native.py`, `tools/setup/native_archive.py`, `tools/setup/native_state.py`, native installer specification/implementation tests and test-only helpers under `tools/setup/tests/`, `tools/distribution/manifest.py`, `tools/distribution/package_release.py`, bootstrap specification tests and test-only helpers under `tools/distribution/tests/`, this plan's execution/evidence records and approved runtime amendments, and `plans/00-roadmap.md`

The baseline includes all accepted uncommitted Phases 1–2 work and closure
records. Resume checks confirm the ten required documents, closed preflight
and major findings, and exact accepted Phase 2 source/full-gate identities.
Phase 3 begins with an implementation-blind authoring-readiness check. No
installer/bootstrap production code or specification test has been written.
The five existing minor findings remain report-only. Commit-at-end and
separate publication approval remain in force.

AR-44's independent authoring-readiness stop is resolved by explicit user
approval of the bounded exact manifest/producer/test-transport amendment.
The owning installation/release documents now define the callable shared
validator, complete asset/reference fixture, local assembly command and
test-only HTTP/process binding. Private state filenames/APIs are not fixed.
Fresh unchanged-implementation `make check` passed with 4,361 Rust instances.
The author confirms the original three interfaces are resolved and requests
the positive ST-12 context. The existing mandatory actual-cohort obligation is
now bound to explicit candidate/context fields and a separate fail-closed
unittest group in a test-only helper module; no success is simulated on the
unsupported development host. Earlier runtime images lack curl and must gain
only declared prerequisites in disposable acceptance environments before proof.
That readiness checkpoint preceded all installer/bootstrap tests and
implementation; independent specification authoring then resumed below.

Task3.1.1's verified oracle is now frozen. Its separate actual positive group
still requires both real cohorts after implementation. Task3.1.2 authoring
uses explicit initial/update candidate records and a real cp310 seed for
cross-cohort migration, as detailed in the testing strategy; local observers
do not substitute successful installed-state evidence. All owned task3.1.1
scratch was cleaned before the independent next-task dispatch. No installer
production code is written.

The task3.1.2 readiness check requests precise targeted-child/rename-barrier
bindings and a foreign-UID fixture. The approved test-only contract now specifies
those external boundaries and a dedicated non-root Docker tmpfs context,
confirmed by an actual no-host-mount probe. No private installer hook, host
chown or success substitute is added. Authoring resumes with the ten earlier
files still frozen; task3.1.2 remains unstarted until its author delivers tests.

Reference: [03-02](03-02-installation.md); AR-20/27/28.

### Session 3.1: Specification tests

- [x] 3.1.1 [spec-author] Write metadata/download/extraction cases ST-12–ST-17 — `tools/setup/tests/test_native_manifest_spec.py`; AR-44 concrete data/transport contract approved and applied. ✅ (completed: 2026-11-19 17:02; independent ten-file oracle parent-reviewed; 24 local methods/44 expected red records, no collection errors/skips; one explicit unavailable-actual-context error, not behavioral red/acceptance; make check fails only new native suite and historical setup29 pass; miniature lock/core/four audits/both wheels pass prior validators after fixture-only wheels.json role correction, all assertions and other nine files unchanged; native-manifest-authoring-proof.json retains identities/evidence; no production implementation)
- [x] 3.1.2 [spec-author] Write installation/prerequisite/state cases ST-11/ST-18–ST-25 — `tools/setup/tests/test_native_install_spec.py`; AR-44 coherent manifest/payload contract. ✅ (completed: 2026-11-19 18:25; independent eight-file oracle parent-reviewed; seven local methods/ten expected red records and ten explicit missing-actual-context errors, not behavioral red/acceptance; make check stops only new setup oracles, historical29 pass; fourteen parent mechanical subprocess/load/syscall-barrier probes pass, including fd-relative rename; previous ten-file oracle and five historical setup files unchanged; native-install-authoring-proof.json retained; no product implementation)
- [x] 3.1.3 [spec-author] Write packaged bootstrap/module-closure case ST-58 — `tools/distribution/tests/test_bootstrap_package_spec.py`; AR-44 local producer and test-only transport binding. ✅ (completed: 2026-11-19 19:11; independent five-file oracle parent-reviewed; sixteen expected-red local methods, no collection errors/skips, and one explicit missing-actual-context error; make check known native red only; nine parent mechanical closure/poison/import/entry probes pass without loading installer code; earlier eighteen oracle files unchanged; bootstrap-package-authoring-proof.json retained; no product implementation or installed-runtime acceptance)
- [x] 3.1.4 Run installer/bootstrap suites red with preserved-old-installation controls — `07-testing-strategy.md`. ✅ (completed: 2026-11-19 19:14; fresh consolidated 47 local methods/70 expected red records, zero local collection errors/skips; twelve mandatory actual methods report missing contexts explicitly, not behavioral red/acceptance; make check stops only known feature red with historical setup29 green; all23 oracles frozen in phase3-spec-red-proof.json, scope exactly23 tests+6 plan/roadmap paths; preservation assertions authored but unreached behind absent producer, actual preservation/activation unproved; all six production entry/modules still absent, no implementation before red)

### Session 3.2: Implementation

**Approved cohort ordering (AR-45):** Implement tasks3.2.1–3.2.9 in this order,
marking each `[~]` immediately. Keep all nine unverified until unchanged
`make check` and all three declared local suites pass; only then promote each
with concern-specific evidence. Task3.2.10's actual-context checks, full phase
verification/review and all commit/publication boundaries remain unchanged.
At the 21:50 integration checkpoint all nine tasks were `[~]`. An initial run passed
all47 local methods and unchanged `make check`; the final rerun includes an
additional bootstrap OS file ceiling for older curl's headerless response
behavior. The ceiling's repeated-write probe confirms bounded output/EFBIG,
no interpreter handoff and private bootstrap cleanup. No task is promoted
until this final source's cohort gate passes.
Both supported runtime images now include the missing declared curl package,
with non-root runtime users and exact image/package receipts retained.
Real accepted support/core plus a freshly packaged current1.1.2 VSIX pass both
local revision producers; exact-byte actual contexts are prepared but no
actual installation method had run. These are private inputs, not publication.
The final unchanged gate subsequently passed all47 local methods plus
`make check`/4361 Rust instances. Each of the nine tasks is now separately
verified for that local checkpoint, with concern-specific limitations recorded
below and source hashes in `phase3-implementation-cohort-proof.json`.
All integration scratch/logs/probes are cleaned; the private real candidates,
frozen verification files and exact context bindings remain for task3.2.10.
The loader-only four-line test import correction is independently authored,
parent-reverified and refrozen in `phase3-spec-red-refreeze-proof.json`; all
assertions and other22 oracle files remain unchanged. All owned authoring
scratch is cleaned before implementation starts.

- [x] 3.2.1 Define distribution metadata validation shared with packaging — `tools/distribution/manifest.py`; 03-02 §Metadata contract. ✅ (verified: 2026-11-19 21:53; nine structural cases green; complete AR-45 cohort gate passes all47 local methods and unchanged make check/4361 Rust instances; phase3-implementation-cohort-proof.json binds exact source/oracle hashes; actual acceptance remains task3.2.10)
- [x] 3.2.2 Implement release-specific shell bootstrap and verified helper handoff — `tools/setup/install.sh`; ST-12–ST-16. ✅ (verified: 2026-11-19 21:53; immutable handoff/hash/length/TLS/timeouts/corruption cases, sh -n and independent OS ceiling probe pass; shellcheck unavailable, no result claimed; complete unchanged AR-45 cohort gate green in phase3-implementation-cohort-proof.json; actual bootstrap acceptance remains task3.2.10)
- [x] 3.2.3 Implement native argument/platform/prerequisite checks and guidance — `tools/setup/install_native.py`; ST-11/ST-14/ST-18. ✅ (verified: 2026-11-19 21:53; malformed argument/cohort rejection and targeted compiler/header/package fault guidance green without package-manager execution; complete unchanged AR-45 cohort gate green; phase3-implementation-cohort-proof.json; actual venv/runtime cohort negatives remain task3.2.10)
- [x] 3.2.4 Implement bounded download and safe archive staging — `tools/setup/native_archive.py`; ST-15–ST-17. ✅ (verified: 2026-11-19 21:53; hash/length/truncation/timeout/manual HTTPS redirect controls, eleven hostile archive scenarios and existing VSIX inspection green; real support inputs pass producer audits; complete unchanged AR-45 cohort gate green in phase3-implementation-cohort-proof.json; live public HTTPS acceptance not claimed)
- [x] 3.2.5 Implement owned-root validation and process lock — `tools/setup/native_state.py`; ST-19/ST-24. ✅ (verified: 2026-11-19 21:53; unrelated-root/managed-alias preservation and canonical-alias real OS lock/death/retry cases green; cache reopen probe passes; complete unchanged AR-45 cohort gate green in phase3-implementation-cohort-proof.json; real foreign-root rejection remains task3.2.10)
- [x] 3.2.6 Assemble candidate with NDK preparation, matching offline venv and VSIX — `tools/setup/install_native.py`; ST-11/ST-25. ✅ (verified local implementation checkpoint: 2026-11-19 21:53; component/content binding and targeted preparation-failure reachability green; real current inputs pass both producers; complete unchanged AR-45 cohort gate green in phase3-implementation-cohort-proof.json; successful real offline environment/preparation/runtime assembly remains mandatory task3.2.10, not inferred from miniature fixtures)
- [x] 3.2.7 Implement smoke tests, completion state and atomic activation — `tools/setup/native_state.py`; ST-20–ST-23. ✅ (verified local implementation checkpoint: 2026-11-19 21:53; separate completion/inventory and fd-relative current transition wired; local negative controls and prior mechanical barrier probes pass; complete unchanged AR-45 cohort gate green in phase3-implementation-cohort-proof.json; real smoke/complete-state/interruption acceptance remains mandatory task3.2.10)
- [x] 3.2.8 Add explicit PATH/VSIX guidance and same-release/Python-upgrade handling — `tools/setup/install_native.py`; ST-12/ST-20/ST-25. ✅ (verified local implementation checkpoint: 2026-11-19 21:53; literal public handoff/argument/cohort controls green; quoted guidance and exact-manifest/interpreter reuse wired; complete unchanged AR-45 cohort gate green in phase3-implementation-cohort-proof.json; actual command usability/idempotence/ABI transition remains task3.2.10)
- [x] 3.2.9 Assemble the complete `.pyz`, hash-pinned shell and manifest using Phase 2 core and existing VSIX packaging; add exact-byte local/draft transport fixture — `tools/distribution/package_release.py`; ST-58. ✅ (verified: 2026-11-19 21:53; sixteen producer/closure/integrity methods green, including determinism/no-clobber/snapshot/isolated module and entry controls; exact real current core/support/fresh VSIX produce both private revisions; complete unchanged AR-45 cohort gate green in phase3-implementation-cohort-proof.json; no publication or real installed-bootstrap acceptance inferred)
- [x] 3.2.10 Run installer/bootstrap suites green through shipped shell with checkout inaccessible and `PYTHONPATH` cleared — setup and bootstrap-package suites; ST-11–ST-25/ST-58. ✅ (verified: 2026-11-19 23:46; all23 mandatory actual instances plus real cp310 seed pass on required Ubuntu22/cp310 and24/cp312 contexts: positive bootstrap/guidance/closure, foreign root, idempotent/support-only update, every child failure/retry, real lock/death, exact activation-boundary deaths, whole-root relocation and ABI transition; all47 locals/make check4361 Rust pass same exact source; all23 frozen oracles/accepted prior source unchanged; phase3-installer-actual-proof.json binds full evidence/hashes; owned logs/work/seeds/processes cleaned, named non-media inputs/contexts retained; live public HTTPS/WSL/publication and full phase hardening/review not claimed)

### Session 3.3: Hardening and gate

After the actual checkpoint, include adversarial coverage for the existing
download time budget across blocking/trickled HTTP reads, and terminal-control
sanitization before producer argument-parser diagnostics. These exercise the
already required bounds/input contracts, not new installer interfaces.

- [x] 3.3.1 Add malformed metadata/archive boundary implementation tests — `tools/setup/tests/test_native_manifest_impl.py`. ✅ (verified: 2026-11-20 00:01; recorded red7 boundary failure records corrected without changing23 independent oracles; all11 implementation methods,47 local specs and unchanged make check4361 Rust green; phase3-metadata-hardening-proof.json binds source/evidence; own logs/fixtures/scratch/processes cleaned; final helper bytes change, so fresh actual phase acceptance remains mandatory after state hardening)
- [x] 3.3.2 Add process-death/activation/lock implementation tests — `tools/setup/tests/test_native_install_impl.py`. ✅ (verified: 2026-11-20 00:22; recorded13-method red exposes6 cases in typed cache receipt/final completion guards; all13 state methods plus11 metadata implementation methods,47 frozen local specs and unchanged make check4361 Rust green after necessary guard corrections; phase3-state-hardening-proof.json binds source/red/green/task evidence and old actual helper source authority; all23 oracles unchanged; own logs/work/processes cleaned; final-source actual/full phase/review gate remains mandatory)
- [x] 3.3.3 Run full gate and independent security/concurrency review with failure injection and outside-path sentinels — `99-execution-plan.md`. ✅ (verified and accepted: 2026-11-21 02:23; user accepts RV-Q303 verified bounded fix and Phase3 closure; source325c76ce,31actual+seed,6separatemechanics,68spec/24impl/makecheck4361/fullprivateverify4695 evidence and exactdiff/source/oracleidentities rechecked; independent original/fixreviews and authorised finaluserdisposition complete, parent-authorshipexceptionaccurate, allmajorfindingsresolved/sixminorsreport-only; allowncompletedworkcleaned, no thirdreview/commit/publication; Phase3closed53/91)

Safety fix assessment returned at04:42: SA-301/302 addressed, no new findings;
correctness at04:43 assesses RV-C301 addressed with no new findings.
Concurrency at04:44 addresses the original workspace/completion findings but
reports new RV-Q303. Aggregate closure is blocked; no third review is started.

**Verify (gate)**: `make verify`; real archive install/update proof ST-11–ST-25.

## Phase 4: VS Code actions and discovery

> **Phase baseline tree**: d20dcf2685edc5b3b7ea2fb0ac845b5c91a3c215
> **Reasoning**: high — executable selection and workspace trust
> **Lenses**: security, API, integration
> **Scope mode**: strict
> **Expected modification set**: `tools/vscode/src/` (existing lifecycle plus focused actions/discovery), `tools/vscode/test/`, `tools/vscode/package.json`, `tools/vscode/README.md`, `compiler/src/lsp/`, `compiler/tests/editor_sources_spec_test.rs`, `compiler/tests/editor_sources_impl_test.rs` and their focused test-support modules/fixtures, this plan's owning specification/execution/evidence/findings records and `plans/00-roadmap.md`.

Phase3 is accepted with its source and all oracles intact. Phase4 reuses the
current extension, Rust parser/module inventory and CLI explicit-file contract;
no second parser, dependency, project-file format, workflow or version bump.
`phase4-entry-proof.json` records the complete dirty-worktree baseline and
existing editor-test identities; establish the unchanged editor gate before
independent test authoring.

Unchanged `make vscode-check` passes at phase entry:9local tests plus the
explicit packaged-client test. The local historical packaged skip is separate
from its explicit successful invocation. Baseline log is cleaned. Independent
author `ses_e0446eb27ffeT3KaVNU26LfLjP` delivered only task4.1.1's action
specifications before any Phase4 production changes. Parent repeated red and
the task gates, reviewed the observable expectations and froze all three files
in `phase4-action-authoring-proof.json`. The editor gate is expected red solely
for the absent commands; this verifies authoring, not implemented behavior.

Independent author `ses_e042a9463ffeU6Wktcrxpvvikf` delivered task4.1.2
discovery/settings/lifecycle specifications. Parent repeats59cases:7compatibility
controls and52expectedred, with no fixture errors/skips. Three new hashes and
per-case classifications are frozen in `phase4-discovery-authoring-proof.json`.
Real desktop/private-display FS-UAE/Remote-WSL steps are recorded but unexecuted.
Production and frozen action/original tests remain unchanged.

Independent author `ses_e04124d20ffej0IOAfooRt5ftn` delivered task4.1.3's
three real saved-disk protocol/closure/root/limit test files. Parent repeats
44cases:4 genuine explicit-file driver controls pass;39 requests and1capability
assertion are expected red. Existing LSP32/version3/driver22 pass unchanged.
`phase4-source-authoring-proof.json` freezes all three new hashes.
The formatter is unavailable; manual style/documentation checks and warning-free
compilation pass without source edits or tool installation. Production and all
prior oracles remain unchanged. Consolidated red is the next separate task.

Reference: [03-04 §VS Code actions](03-04-editor-and-release.md#vs-code-actions);
AR-21/23.

### Session 4.1: Specification tests

- [x] 4.1.1 [spec-author] Write action/trust/save/CWD cases ST-40–ST-43 — `tools/vscode/test/actions.spec.test.ts`. ✅ (verified: 2026-11-21 02:55; independent author and parent repeat32cases:1startupcontrol/31missing-commandassertions, zero fixture errors/skips; deeper assertions await implementation; historical editor10/10 including explicit packagedclient, strictTS/syntax/build/stage and makecheck4361Rust pass; vscode gate expectedred only same31commands,11pass/0skip; all1108baseline nonplan files/38acceptedPhase3 identities and oldtests unchanged; three new oracle hashes frozen in phase4-action-authoring-proof.json, documentation/self-containment reviewed, completed scratch/log/cache cleaned; no production/commit/publication)
- [x] 4.1.2 [spec-author] Write discovery/LSP/package cases ST-44–ST-46 — `tools/vscode/test/discovery.spec.test.ts`; record real-machine acceptance steps. ✅ (verified: 2026-11-21 03:24; independent author/parent59cases=7compatibilitygreen/52expectedred, zero fixtureerrors/skips;31command-blocked deeper assertions remain unproved; strictTS/syntax/build/stage, historical editor10/10 and makecheck4361Rust pass; combinedvscode expectedred83/18pass/0skip, nofreshVSIXacceptance; all1108baseline nonplan files/38Phase3identities/oldandactiontests unchanged; three hashes frozen in phase4-discovery-authoring-proof.json; realcandidate desktop/privateFSUAE/actualRemoteWSL procedure recorded but unexecuted, manualfixture export corrected from spec10, no new Windows acceptance leg; docs/reference checks pass, completed scratch/log/cache cleaned; no production/commit/publication)
- [x] 4.1.3 [spec-author] Write saved-disk selection/protocol/root/limit cases ST-59 — `compiler/tests/editor_sources_spec_test.rs`, `tools/vscode/test/actions.spec.test.ts`; actual two-file driver control. ✅ (verified: 2026-11-21 03:47; independent author/parent44cases=4realhost/bothAmigadrivercontrols/40expectedred39method+1capability, zero fixtureerrors/skips; oldLSP32/version3/driver22 pass; makecheck2810priorRust+4controls then only40newfailures, remaininglegsnotreached; editor18pass/83expectedred, nofreshVSIX; threeRustoracles frozen in phase4-source-authoring-proof.json, all1108baseline nonplan/38Phase3 identities and priororacles unchanged; warningfreecompile/manualstyle/docs/reference checks pass, rustfmtunavailable/noinstall, completed scratch/log/cache cleaned; no production/commit/publication)
- [x] 4.1.4 Run editor/source-selection suites red and preserve existing LSP tests — `07-testing-strategy.md`. ✅ (verified: 2026-11-21 03:58; fresh135newcases=12controls+123expectedassertionfailures, zero fixtureerrors/skips; separatehistoricalRust57 and editor10 green; makecheckonlynew40red after2810priorRust+4controls, laterlegsunrun; combinededitor101=18pass/83expectedred, freshpackagingunreached; all9oracles frozen/unchanged, baseline1108files unchanged and exactly9newnonplanfiles; phase4-spec-red-proof.json retains commands/cases/identities, documentation/link/whitespace checks pass, allcompleted scratch/log/cache cleaned; specfirstgate satisfied57/91, no implementation/acceptancegreen/commit/publication)

### Session 4.2: Implementation

The user approves AR-46 at07:48. Implement4.2.1–4.2.5 in order as one cohort;
immediately record each implemented member `[~]`, with source/oracle identities.
Resume an interrupted group at its first unimplemented member after integrity
checks, retaining earlier members as unverified. Promote none until unchanged
task/editor/private full gates pass; then promote each separately with its
documentation check. Actual desktop4.2.6 and hardening/reviews remain separate.

All five members were implemented/unverified in the approved order.
`phase4-cohort-progress.json` records their transition hashes, current source
identities and nine unchanged oracles. Focused editor101 and Rust driver22/
saved-selection44/LSP32/version3 pass. A function-type AST parameter access
compile error was corrected without test changes. Fresh package, task and
authenticated-private-display full verification subsequently pass. All five
members promote separately at08:33 after integrity/documentation checks and
completed-work cleanup. `phase4-cohort-verified-proof.json` binds source tree
`5385a84ea9a29149b47a6c20bb93688b1b4a616a`, task4405Rust, editor101 plus1fresh
packaged client, private full4739Rust and the retained exact development VSIX.
The cohort scheduling exception is complete; normal task ordering resumes.

Task4.2.6 preflight is in progress without advancing the task. The host is
Ubuntu26.04, so its desktop is not substituted for a supported cohort. The
verified source is rebuilt offline in the existing Ubuntu22 runtime with the
SDK's prebuilt Rust toolchain; the three real current-version binaries build
successfully. Existing public producers archive them and assemble an unpublished
private development candidate with the exact retained VSIX and proved support.
All twelve asset lengths/hashes match. Earlier frozen-core acceptance inputs
and all oracles stay unchanged. A real non-root Ubuntu22 default-root install
now passes with exact rebuilt binary/VSIX hashes, all three runnable profiles'
saved-import stdout/status and the entry-only omission failure. Completed
build/toolchain/source copies are cleaned. The disposable GUI context now runs.
A test-image-only alias fixes the unchanged Snap Code interpreter path.
Manual exact-VSIX installation succeeds. Actual desktop observations show
null/i32 diagnostics and clearing, cross-file hover/definition, installed-driver
Build/Run for host and both Amiga profiles with status0, a saved dependency
output change, invalid user driver/LSP failures without fallback, and exactly
one Build after reload. Direct PATH-only inspection through the real extension
host's built-in Node debugger subsequently confirms no installed/development
tool paths. Image-only Mesa/libGL prerequisites resolve FS-UAE3.1.66 startup.
Both profile actions display the expected real A1200/KS3.1 guest line, use
the private authenticated display/configuration and remove their private roots
after closing. Guest exit status is not inferred from emulator completion.
Named `phase4-desktop-proof/` evidence now retains the actual source/core/
bootstrap, selected screenshots, process observations and environment recipes.
The owned editor/container/debugger/private display/auth/media/runtime and
disposable image tags are cleaned. Task `make check` passes4,405 Rust instances;
the final explicit-input editor gate passes101 plus1fresh packaged-client case
with zero skips. An earlier optional old-package skip is recorded, not hidden.
All fresh VSIX members match the exact installed candidate. Task4.2.6 verifies
at09:39; source/oracles, commit and publication boundaries remain unchanged.

- [x] 4.2.1 Declare commands and machine-level path settings — `tools/vscode/package.json`; 03-04 §VS Code actions. ✅ (verified: 2026-11-21 08:33; all AR46 gates pass on5385a84e:makecheck4405Rust/editor101+freshpackaged1/privatefull4739Rust; staged and exactVSIX commands/five machine scopes/version1.1.2 audited; frozen oracles intact, documentation/metadata self-check passes; phase4-cohort-verified-proof.json, completed scratch/auth/logs cleaned, desktop/reviews still separate; no commit/publication)
- [x] 4.2.2 Implement installed/development tool resolution — `tools/vscode/src/discovery.ts`, `extension.ts`; ST-44/ST-45. ✅ (verified: 2026-11-21 08:33; same complete AR46 gates/source5385a84e pass; installed/explicit/env/trusteddev/PATH identities, workspace isolation, invalid-path and restricted-start cases green; discovery API/documentation self-check passes, frozen oracles unchanged; named cohort proof/exactVSIX retained and completed scratch cleaned; actual desktop proof remains)
- [x] 4.2.3 Implement bounded saved-disk dependency selection reusing Rust parser/module-reference logic — `compiler/src/lsp/source_files.rs`, `workspace.rs`; ST-59. ✅ (verified: 2026-11-21 08:33; same complete AR46 gates/source5385a84e pass; all44 real saved-source specs including graph/bindings/overlays/root/readability/exact2000+256/overflow/finalfile and actual driver controls green; exhaustive AST/parser/loader/symbol-table paths and documentation self-check reviewed; nine oracles unchanged, named proof retained, completed work cleaned; internal traversal hardening remains4.3)
- [x] 4.2.4 Add request dispatch and capability version, preserving current LSP behavior — `compiler/src/lsp/`; ST-59; 03-04 §Saved-source selection. ✅ (verified: 2026-11-21 08:33; same complete AR46 gates/source5385a84e pass; actual version1 capability/request/error/lifecycle and historicalLSP32/version3 green, live overlay behavior preserved; dispatch/caller/documentation self-check passes, source/oracle identities intact; named proof retained, completed scratch cleaned; phase review remains)
- [x] 4.2.5 Implement trusted saved-file selection request, dirty-dependency checks and process tasks — `tools/vscode/src/actions.ts`, `extension.ts`; ST-40–ST-43/ST-59. ✅ (verified: 2026-11-21 08:33; same complete AR46 gates/source5385a84e pass; editor101 plus1fresh real packagedclient green with no skips, trust/save/root/races/dirtydeps/literalargv/capability/config/lifecycle assertions reached; all9oracles unchanged, documentation/API/callers reviewed, exactVSIX and proof retained; completed scratch/log/auth/process cleanup complete; group closed62/91, desktop4.2.6 and hardening/reviews separate, no commit/publication)
- [x] 4.2.6 Run editor/source-selection suites green and exercise installed VSIX from desktop — editor and Rust suites; ST-40–ST-46/ST-59, WSL final evidence in Phase 6. ✅ (verified: 2026-11-21 09:39; real Ubuntu22 installed-VSIX diagnostics/navigation/all-profile Build/Run/dependency/override/reload/PATH observations and both A1200/KS3.1 native guest actions pass; current makecheck4405Rust/editor101+freshpackaged1 with zero final skips, exact member/source/oracle hashes preserved; phase4-desktop-proof/proof.json and named core/bootstrap/screens retained, all owned runtime/media/auth/images/gate scratch cleaned; Phase4 hardening/review and final Remote-WSL remain separate; no commit/publication)

### Session 4.3: Hardening and gate

The final review input is frozen at tree
`a753ada584c1812c3d2003491720f438a1cd1bfd` against Phase4 base `d20dcf26…`.
All23 non-plan changes are inside the approved modification set.
`phase4-review/packet-identity.json`, `phase4.diff` and `review-packet.md`
bind the scope, deliverable/test evidence, minimum design, required blast-radius/
test-integrity method and honest actual-versus-final-candidate limits.
The current authenticated-private-display full gate and packaged-editor gate
run before independent read-only correctness/API, local packaging/workspace
trust, concurrency and the existing Amiga action/acceptance specialist review.
Structured routing is absent, so strict defaults and complete generic fallback
packets apply where named tools are unavailable. No reviewer has been dispatched
yet at the freeze checkpoint. The current full gate subsequently passes:
4,761 Rust instances/every acceptance leg and editor122 plus1fresh packaged
client with zero skips. Owned gate display/authentication/logs/scratch are
cleaned. All four initial reviews and both bounded recommendation challenges
are complete; reports and sessions are retained in `phase4-review/`.
Three majors remain uncorrected: CA-401, RV-C401 and SA-401. The user now
approves the consolidated package, including AR-47's longer-member field-first
selection policy/bounded helper and AR-48's `${` task-input restriction.
Independent implementation-blind regression authoring is dispatched before
any production edit. Targets are new `editor_review_spec_test.rs`,
`actions.review.spec.test.ts` and focused test support; all nine original
oracle/support files remain frozen. DEF-581 stays open through correction
verification and one applicable fix-diff re-review. Original review/challenge
scratch is cleaned; current correction scratch is separately owned.
No Phase4 closure, Phase5, commit or publication is inferred.

The delivered independent red phase is recorded in `phase4-fix-spec-red-proof.json`.
After the approved corrections, focused editor160/Rust123 and shape/control10/
historical symbol4 pass. Current correction input freezes at
`e440a68abc5b545760621baa32b6bdd47cb62924` against reviewed `a753ada5…`.
All11 changed non-plan paths are within the approved correction surface; the
original nine and delivered three oracle hashes are unchanged. The full
authenticated-private-display/editor gates run before the single applicable
fix-diff re-review. `phase4-fix-identity.json` and `phase4-fix.diff` bind that
review boundary. No review has yet been launched on this fix.

Current correction full gate passes at11:56: private `make verify`4796 Rust
instances/all acceptance legs and editor170 plus1fresh packaged client,
zero skips. All11 fix identities, nine original/three new frozen oracles and
exact packaged README match. `phase4-fix-full-proof.json` retains the evidence.
Owned gate logs/scratch/display/auth are cleaned. The single applicable fix-diff
re-review then returned no findings from all four roles: correctness/API pass
(independent133 Rust/170 editor/strictTS and compiler/LSP probes); local
packaging/workspace-trust safety SA-401 closed (independent133 Rust/48 editor
plus guard-removal mutation controls); concurrency clean (receipt-only/no-shell);
Amiga action/acceptance clean (independent48/48). Task4.3.4 verifies at12:05,
Phase4 closes at67/91 and DEF-581 resolves. No third review, earlier-phase
reopening, commit or publication occurred.

- [x] 4.3.1 Add task disposal/setting-inspection implementation tests — `tools/vscode/test/actions.impl.test.ts`. ✅ (verified: 2026-11-21 09:52; initial19 controls14green/5red; final21pass and all five defects independently reproduced against compiled pre-hardening source; narrow null/disposal fixes, frozen specs unchanged; makecheck4405Rust/editor122+freshpackaged1zero skips, documentation/scope/source hashes checked; phase4-actions-hardening-proof.json retained, all owned scratch cleaned; no commit/publication)
- [x] 4.3.2 Add disk/overlay/index-limit/path implementation controls — `compiler/tests/editor_sources_impl_test.rs`. ✅ (verified: 2026-11-21 10:05; initial22=16green/6red after actual valid host builds, then22green plus frozen44/historicalLSP32/core symbol4; narrow shared metadata correction preserves declaration-token lookup and all historical inline assertion bytes; makecheck4427Rust/editor122+freshpackaged1zero skips; phase4-sources-hardening-proof.json retained, scope/documentation/oracle integrity checked and task scratch cleaned; no new resolver/API/language rule/ABI/emission change, commit or publication)
- [x] 4.3.3 Update extension installation/action/Remote-WSL and compatible-LSP guide — `tools/vscode/README.md`; 03-04 §Tool discovery / Saved-source selection. ✅ (verified: 2026-11-21 10:15; guide identifiers/limits/capability/shell syntax and manual transfer/hash/guest startup checked; makecheck4427Rust/editor122+freshpackaged1zero skips, exact README bytes match shipped extension/readme.md; frozen oracles unchanged, phase4-guide-proof.json retained and task scratch cleaned; no Windows/Remote-WSL acceptance claim, commit or publication)
- [x] 4.3.4 Run full and packaged-editor gates; independent security/API review — `99-execution-plan.md`. ✅ (verified: 2026-11-21 12:05; approved CA-401/RV-C401/SA-401 corrections implemented after independent Rust8red17green/editor39red9green; focused editor160/Rust123/shape10/symbol4; private full4796Rust/all legs and editor170+freshpackaged1zero skips; all11 fix, nine original and three new oracle hashes unchanged; single fix-diff re-review returns no findings from correctness/API, local safety, concurrency and Amiga roles; DEF-581 resolved, all owned scratch/gates/display/auth cleaned; no third review, commit or publication)

**Verify (gate)**: `make verify` and `make vscode-check`.

## Phase 5: Release packaging and bounded workflows

> **Phase baseline tree**: recorded at execution start
> **Reasoning**: high — immutable release identity and publication permissions
> **Lenses**: supply-chain security, data/migration, API

Reference: [03-04](03-04-editor-and-release.md); AR-17/22/25.

### Session 5.1: Specification tests

- [x] 5.1.1 [spec-author] Write workflow/payload/identity cases ST-47–ST-51 — `tools/distribution/tests/test_release_spec.py`. ✅ (authored: 2026-11-21 19:03; 13 independent cases cover manual-input validation, pinned least-privilege workflows, actual/fixture payload identity, independent core/VSIX reuse, support-only revision and promotion blockers; public producer only, no implementation imports)
- [x] 5.1.2 [spec-author] Add version/documentation case ST-52 to the release suite. ✅ (authored: 2026-11-21 19:03; version records, runbook/notes/changelog, README, quick-start, licence set and immutable v1.1.2 facts/hashes pinned)
- [x] 5.1.3 Run release suite red; record historical document controls — `07-testing-strategy.md`. ✅ (red: 2026-11-21 19:03; 13 cases ran, 16 subtest failures plus 1 downstream error: missing workflows, absent validate/verify-component/promotion commands, 1.1.2 records and old docs; historical controls bind v1.1.2 MIT, tag object 5beec93 and published binary hash 52f9e364…; no implementation file changed)

### Session 5.2: Implementation

- [x] 5.2.1 Update compiler version records and run the existing Phase 2 core producer with final versions — `compiler/Cargo.toml`, `Cargo.lock`; ST-48/ST-57. ✅ (verified: 2026-11-21 19:10; Ubuntu22 builder used pinned Rust1.99/glibc2.35; all three release binaries report1.2.0; core archive9960d775… 18,544,640 bytes with five notices and exact core.json; existing producer suite45 tests passed with only the two expected missing-actual-input setup errors excluded from the fast target; no clobber/publication)
- [x] 5.2.2 Update extension version records and verify VSIX payload — `tools/vscode/package.json`, `package-lock.json`; ST-48. ✅ (verified: 2026-11-21 19:10; manifest and lock root both1.2.0; makevscode-package produced clorraine-1.2.0.vsix with name/publisher/version match; final rebuilt VSIX ed0c803d… 26,556 bytes; audit_vsix ok14 entries; packaged client later passed)
- [x] 5.2.3 Extend Phase 3 assembly with immutable component/distribution identities, independent core/VSIX reuse and promotion eligibility — `tools/distribution/package_release.py`; ST-49–ST-51. ✅ (verified: 2026-11-21 19:40; validate/verify-component/promotion added; private snapshots, identity/digest reuse, exact approval binding, per-asset size/hash/inventory and ST-56 separation; frozen release cases13/13 and implementation controls10/10 pass; no publication or output mutation)
- [x] 5.2.4 Add manual support workflow calling proved local commands — `.github/workflows/support-release.yml`; ST-47/ST-49. ✅ (verified: 2026-11-21 19:40; manual only, required inputs, SHA-pinned actions, read-only default, one write upload job, self-hosted prepared sources/image, draft support release with make_latest=false; structural cases pass; not dispatched)
- [x] 5.2.5 Add manual distribution workflow managing both compiler/editor component and distribution drafts — `.github/workflows/distribution-release.yml`; ST-47/ST-50. ✅ (verified: 2026-11-21 19:40; independent core/VSIX reuse, Ubuntu22/24.04 checks, component+distribution drafts, exact approved-hash gate and promotion; one write job, SHA-pinned actions, no run-body interpolation; not dispatched)
- [x] 5.2.6 Update v1.2.0 runbook/notes/changelog while preserving historical facts — `RELEASE.md`, `RELEASE-NOTES.md`, `CHANGELOG.md`; ST-52. ✅ (verified: 2026-11-21 19:55; v1.2.0 candidate records, two-workflow/promotion commands and Apache terms; v1.1.2 MIT tag5beec93 and published hashes443377…/52f9e364… preserved historical; old unreleased compiler notes folded into1.2.0 with no loss)
- [x] 5.2.7 Update native quick-start/root README and installation links — `site/guide/quick-start.md`, `README.md`; ST-52. ✅ (verified: 2026-11-21 19:55; installer/PATH/VSIX/prerequisite/WSL boundaries and v1.2.0 candidate status documented; site status/brief/roadmap aligned; no publication or WSL acceptance claimed)
- [x] 5.2.8 Run release specifications green against locally assembled candidates — `tools/distribution/tests/test_release_spec.py`. ✅ (verified: 2026-11-21 20:03; 13/13 pass with actual core9960d775… and VSIXed0c803d… plus miniature reuse/promotion controls; actual payload checks bind three binaries, five notices and1.2.0 VSIX metadata; no real-machine claim)

### Session 5.3: Hardening and gate

- [x] 5.3.1 Add payload/reuse/input-validation implementation tests — `tools/distribution/tests/test_release_impl.py`. ✅ (verified: 2026-11-21 19:42; 11 controls cover mode/identity/commit rejection, changed VSIX metadata, alias/FIFO existing assets, independent reuse, approval schema/duplicates, unreadable payload, inventory mismatch and the locked preparation bundle; distribution-check139 tests passes; no source/test pin weakened)
- [x] 5.3.2 Run full/editor/site gates and independent release-security review — `99-execution-plan.md`. ✅ (verified: 2026-11-21 20:45; initial correctness/security reviews found one major YAML defect, nine major correctness gaps and five security minors; fix cohorts971e62b5 and8608a774 corrected them with valid YAML, preparation bundle, support-run artifact handoff, pinned tools, public URL checks, draft-only reruns, exact-candidate hashes and gate coverage; single fix-diff re-review confirmed the accepted findings and its residual preparation/rerun findings were fixed and locally verified by consumer schema/tests; final makecheck139 distribution tests/fuzz28, releasesuite27/27, sitecheck pass; earlier full4796Rust/alllegs and editor receipts remain on unchanged compiler/extension bytes; no workflow dispatch, tag, release, commit before review, or publication)

**Verify (gate)**: `make verify`, `make vscode-check`, `make site-check`.

## Phase 6: Candidate acceptance and authorised promotion

> **Phase baseline tree**: recorded at execution start
> **Reasoning**: high — real-platform evidence and public activation
> **Lenses**: integration, Amiga platform, release integrity

Reference: [03-04 §Draft acceptance](03-04-editor-and-release.md#draft-acceptance-and-public-promotion);
AR-22/24. Actual WSL2 evidence is owned by the user; missing evidence blocks.

### Session 6.1: Specification acceptance

- [ ] 6.1.1 [spec-author] Write candidate completeness checks and exact manual input/output acceptance steps ST-53–ST-56 — `tools/distribution/tests/test_acceptance_spec.py`, `RELEASE.md`.
- [ ] 6.1.2 Run acceptance checks red on absent/incomplete candidate evidence; record negative controls — `07-testing-strategy.md`.

### Session 6.2: Candidate execution and green

- [ ] 6.2.1 Dispatch the two implemented workflows for draft candidates after implementation authorisation; record resulting identities — `RELEASE.md`; ST-47–ST-51.
- [ ] 6.2.2 Run clean Ubuntu, relocated install/update and desktop candidate proof — `RELEASE.md`; ST-53.
- [ ] 6.2.3 Run private-display native/legacy Amiga gates and obtain user's real WSL2/Remote-WSL evidence — `RELEASE.md`; ST-54/ST-55.
- [ ] 6.2.4 Verify pre-publication specification cases green: ST-53–ST-55 and applicable earlier cases, with ST-56 explicitly post-publication — `RELEASE.md`.

### Session 6.3: Hardening, approval and public gate

- [ ] 6.3.1 Add evidence/hash-mismatch negative-control implementation tests — `tools/distribution/tests/test_acceptance_impl.py`.
- [ ] 6.3.2 Run final full/editor/site gates and independent acceptance audit against exact candidate bytes; resolve required findings before any publication — `99-execution-plan.md`.
- [ ] 6.3.3 Request and record explicit publication approval bound to reviewed hashes/evidence; block if absent or invalidated — `RELEASE.md`; ST-50.
- [ ] 6.3.4 Publish/verify support and compiler/editor payloads, promote distribution, then pass anonymous-install ST-56 against approved hashes — `RELEASE.md`; 03-04 §Draft acceptance and public promotion.
- [ ] 6.3.5 Close release evidence, sync roadmap and run post-completion project analysis through their skills — `RELEASE.md`, `plans/00-roadmap.md`.

**Verify (gate)**: `make verify`, `make vscode-check`, `make site-check`,
ST-53–ST-56 with real-machine evidence.

## Review and stop conditions

Every phase gets an independent correctness review plus the listed risk lenses;
the Amiga specialist joins Phases 1, 2 and 6. A separate auditor checks
test integrity and omitted parallel paths. Role unavailability uses a generic
agent with the same packet; it does not lower the gate.

Include all callers/exhaustive matches/parallel native-container paths and
negative controls in packets. Emission, ownership or language-rule changes are
not expected; if a necessary change reaches them, follow AGENTS.md specialist
dispatch and reopen scope rather than silently extending this plan.

Critical/major findings require the user's ruling before work continues.
Unsupported dependency/redistribution/portability assumptions stop at Phase 1.
Unavailable licensed media or actual WSL2 evidence blocks acceptance. Public
promotion always waits for the explicit approval in AR-22.
