# Execution findings

| Finding | Phase | Severity | Area | Ruling |
|---|---|---|---|---|
| RV-001 | 1 | MINOR | distribution-status-docs | deferred |
| SA-001 | 1 | MINOR | producer-provenance-validation | deferred |
| SA-002 | 1 | MINOR | display-cookie-documentation | deferred |
| SA-201 | 2 | MINOR | native-wheel-receipt-validation | report-only; unresolved, no fix applied |
| SA-202 | 2 | MINOR | core-producer-success-output | report-only; unresolved, no fix applied |
| SR-201 | 2 | MAJOR | native-emulator-mount-delimiter | resolved; all fix reviews agree comma protection is addressed |
| SR-202 | 2 | MAJOR | native-emulator-cross-build-storage | resolved; bounded conjunction verified and accepted by user 2026-11-19 15:12 |
| RV-C201 | 2 | MAJOR | core-version-snapshot-integrity | resolved; all fix reviews agree private snapshot binding is addressed |
| RV-301 | 2 | MAJOR | native-emulator-artifact-directory-alias | resolved; verified bounded fix and Phase 2 accepted by user 2026-11-19 15:12, no third review |
| RV-C301 | 3 | MAJOR | candidate-smoke-sdk-selection | resolved; verified fix/review and Phase3 accepted by user2026-11-21 |
| RV-Q301 | 3 | MAJOR | interrupted-completion-publication | resolved; verified fix/review and Phase3 accepted by user2026-11-21 |
| RV-Q302 | 3 | MAJOR | substituted-workspace-cleanup | resolved; verified workspace fix/review plus bounded receipt correction accepted |
| SA-301 | 3 | MAJOR | component-aware-archive-link-resolution | resolved; verified fix/review and Phase3 accepted by user2026-11-21 |
| SA-302 | 3 | MAJOR | installer-audit-source-notice-obligations | resolved; verified fix/review and Phase3 accepted by user2026-11-21 |
| RV-Q303 | 3 | MAJOR | pending-receipt-cleanup-root-substitution | resolved; user accepts fully verified bounded correction and Phase3 closure2026-11-21 |
| SA-303 | 3 | MINOR | producer-derived-success-path-controls | report-only; unresolved, no fix applied |
| CA-401 | 4 | MAJOR | saved-dependency-selection-race | resolved; approved guard independently re-reviewed clean2026-11-21 |
| RV-C401 | 4 | MAJOR | qualified-module-head-classification | resolved; bounded helper/field-first policy independently re-reviewed clean2026-11-21 |
| SA-401 | 4 | MAJOR | vscode-task-variable-instructions | resolved; `${` restriction independently re-reviewed clean2026-11-21 |

## Consolidated correction approval — 2026-11-21 11:10

The user replies **“i approve”** to the single consolidated request, explicitly
including AR-47's bounded helper and field-first policy and AR-48's `${`
restriction. All three necessary corrections may proceed within that packet.
Independent regression authoring precedes fixes; all frozen oracles remain
unchanged. Broader type-checking machinery triggers another stop.
DEF-581 stayed open until the authorised corrections passed gates and the one
applicable fix-diff re-review. Phase5 stayed unstarted; no commit/publication.
Older proposal/hold entries below are retained as chronological evidence.

### Correction verification and fix-diff re-review — 2026-11-21 12:05

The authorised corrections are complete on fix tree
`e440a68abc5b545760621baa32b6bdd47cb62924` against reviewed `a753ada5…`.
Independent implementation-blind regressions ran red first: Rust8 behavioral
red/17 compatibility controls, editor39 behavioral red/9 controls. After the
bounded fixes, focused editor160 and Rust123 pass; three additional shape
controls plus seven shared limits pass10; historical symbol4 pass.

Current full gates: authenticated-private-display `make verify` passes4,796
Rust instances and every acceptance leg; explicit-input `make vscode-check`
passes170 plus1fresh packaged client, zero skips. All11 fix identities, nine
original oracles and three new regression files remain byte-identical;
packaged README equals source. Evidence is bound in
`phase4-fix-identity.json`, `phase4-fix-spec-red-proof.json` and
`phase4-fix-full-proof.json`.

The single applicable fix-diff re-review then returned **no findings** from all
four roles:

| Role | Result |
|---|---|
|Correctness/API|Pass; all three majors addressed; independent133 Rust/170 editor/strictTS and compiler/LSP probes|
|Local packaging/workspace-trust safety|SA-401 closed; no findings; independent133 Rust/48 editor and guard-removal mutation controls|
|Concurrency|No findings; approved event/save/session/disposal interleavings covered; receipt-only/no-shell|
|Amiga action/acceptance|No findings; independently ran48/48 correction specs; argv/CWD/profiles/guide unchanged|

CA-401, RV-C401 and SA-401 are resolved. DEF-581 is resolved; task4.3.4 is
verified and Phase4 closes at67/91. The probe-limited results, no-atomic-snapshot
boundary, stdlib-fallback policy and earlier-desktop byte scope are recorded.
No third review, earlier-phase reopening, commit or publication occurred.

## Phase 4 initial reviews complete: three major findings — 2026-11-21

| Role | Independent result |
|---|---|
|Correctness/API|RV-C401 major; CA-401 independently reproduced/confirmed; Rust98, makecheck4427, editor122zero skips, historical symbol4 and strictTS/whitespace pass|
|Local packaging/workspace safety|SA-401 major; editor122, Rust98,20invalid-response probes, null/budget mutation controls and frozen scope/package checks pass|
|Concurrency|CA-401 major; no-shell static interleaving review, no files changed|
|Amiga action/acceptance|No findings; focused editor112 and actual evidence/source identity checks pass|

All four reviewed only the Phase4 diff. No additional critical/minor findings
or separate pre-existing finding is reported. Prior six minors remain
report-only. All initial reviews and both bounded independent recommendation
challenges are complete. Their reports are retained in `phase4-review/`.
These are not post-fix re-reviews and no review-fix budget is consumed.
All owned review/challenge scratch is cleaned. No implementation/oracle
changes are authorised.

### RV-C401 — qualified source is omitted

`compiler/src/lsp/source_references.rs:49–55` treats every same-headed top-level
declaration/import as a value-field shadow before trying inventory modules.
The independent reproduction uses:

```text
main.clo: module app; let graph: i32 = 7;
          func main(): i32 { return graph.math.answer(); }
math.clo: module graph.math; export func answer(): i32 { return 0; }
```

The real explicit two-file host build exits0. Selection returns only main;
building returned files exits1/E10212 (`math` is not a member of i32).
Function/struct-headed variants also reproduce. Resolve/typecheck source is
unchanged from the phase base; this is a new selector defect, not a language
rule correction. Frozen genuine local/global/import field controls must remain.
The bounded remedy must distinguish genuine projections from qualified paths
using existing parsed metadata/precedence, not a second parser or new semantics.
The independent challenge confirms the defect and rejects blanket guard removal.
Existing Symbol metadata lacks member/type structure. AR-47 asks whether the
existing field-first minimal selection policy extends to longer genuine member
chains. Recommend bounded request-local parsed declaration/type/import/merged-
module metadata, with at most one focused Rust helper plus existing metadata
plumbing. Retain lexical-local shadowing and longest module matching otherwise.
Stop if a general type checker or broader machinery becomes necessary.
The alternative, compiler module-first inventory selection, changes feature
behavior and frozen expectations and may pull otherwise-unneeded modules.
Confidence is high on diagnosis, medium on the bounded correction scope.

### CA-401 — independently reproduced

The correctness reviewer challenges and confirms the concurrency finding.
A scratch-only external editor observer changes/saves D before releasing the
selection reply. D is clean/version2 at completion; unchanged actions submit
one task with the old closure and no error. That argv fails an actual build;
adding the new leaf exits0. A request-local root `.clo` change/save guard,
subscribed before selection and checked before submission with finally cleanup,
closes this editor-save interleaving. It may conservatively request retry after
unrelated root-source edits; it is not an atomic filesystem snapshot or a
promise against arbitrary later/external writes. Independent confidence: high.

### SA-401 — task variables reinterpret validated data

`tools/vscode/src/actions.ts:191–195` passes validated strings into
ProcessExecution, but VS Code still resolves `${...}` task variables.
The independent probe accepts `main-${command:safety.marker}.clo` through the
compiled action/real LSP. Unmodified Code1.147.0 collector/resolver methods
invoke an observation-only command stub and rewrite the argument to
`main-resolved.clo`. No GUI, real editor command or task process is run.
Existing fixtures observe raw arguments, not task-variable resolution.

Executable, arguments and CWD need a verified literal-preserving mechanism or
explicit rejection of task-variable syntax with actionable guidance. A rejection
policy must be presented explicitly rather than silently weakening filename
support. The completed independent challenge confirms executable/argv/CWD
rewriting and disproves dollar/backslash/single/double-quote escapes against
unchanged Code1.147.0 methods. No public literal-disable option is verified.
Recommend rejecting the `${` introducer across every final task input, subject
to explicit AR-48 approval. Preserve other punctuation and provide rename/path
guidance. Confidence is high; no actual editor command or task process ran.

DEF-581 covered all three majors. At this checkpoint Phase4 closure and Phase5
were held; later entries record the authorised fixes and clean re-review.

### Consolidated recommendation — approved 2026-11-21 (historical)

The user's “i approve” authorised the corrections. The earlier wording below
records the proposal as presented and is not the current status; the result is
recorded above.

Approve CA-401's request-local root `.clo` change/save guard; choose AR-47's
field-first selection policy and specifically approve its bounded member/type
helper for RV-C401; approve AR-48's `${` task-input restriction for SA-401.
The user approved this package on 2026-11-21; the result is recorded above.

The proposed modification set is the affected action/selection/metadata code,
at most one request-local Rust helper, independently authored new regressions
and necessary test support, plus the owning guide/contract/findings records.
Keep all nine frozen oracle/support files unchanged. No second parser, general
type checker, launcher, protocol, dependency, workflow or version change is
included. Stop for another ruling if the bounded member predicate cannot suffice.

After explicit approval: independent regressions first and red, authorised
fixes, task/editor/private full gates, then exactly one fix-diff re-review per
applicable initial role. Earlier phases remain closed, minors report-only,
commits deferred to plan completion and publication separately gated.

## Phase 4 review hold: CA-401 — 2026-11-21

Independent concurrency review `ses_e0282f001ffelh6Hlzx1ljaYT3` reports one
major finding, no critical/minor findings. Its project brief forbids shell
execution: the finding is a concrete static interleaving, not a claimed probe.
No files were created or changed by that reviewer.

| Field | Evidence |
|---|---|
| Locations | `tools/vscode/src/actions.ts:160–174`; `compiler/src/lsp/source_files.rs:146–147` |
| Failure | Rust reads dependency D's old imports; the user saves D with a new import while selection is pending; old selection returns, D is clean and entry/session unchanged; task compiles new D using the old source list |
| Violated behavior | Complete saved-disk dependency selection must not become a stale partial program merely because a pending dependency edit was saved |
| Existing control gap | `tools/vscode/test/fixtures/action-host.cjs:290–299` leaves the selected dependency dirty; it does not complete its save before releasing the response |
| Proposed bounded remedy | Request-local editor source change/save invalidation under the selected root, with retry guidance; add an implementation regression that saves D before releasing selection and requires zero tasks |
| Status | Historical hold; the bounded guard was approved, implemented and re-reviewed clean on 2026-11-21. Task4.3.4 is verified and DEF-581 resolved. |

Parent inspection confirms the missing check: the action validates entry/
session and current dirty state, not intervening saved dependency changes.
At this initial checkpoint the correctness challenge and safety review were
pending. Both have since completed, as recorded above. The Amiga specialist
returns **no findings**, with independent private TypeScript/
focused-editor112pass/zero skips and actual-evidence identity checks; no GUI/
emulator launched, own scratch cleaned. Its report is retained separately.
Do not reopen previous phases, apply a fix, alter frozen tests, or start
Phase5 without the required ruling.
Current full private4761Rust/editor122+1 gates remain valid evidence for the
reviewed source, not evidence that this interleaving is covered.

## Phase 3 accepted — 2026-11-21 02:23

The user replies **“I accept”** to accepting the verified RV-Q303 fix and
Phase3 closure. Parent rechecks exact source/diff/oracle identities and the
verified gate evidence before promotion. RV-Q303 and the five previously
addressed Phase3 findings are resolved; DEF-579 is complete.

Task3.3.3 verifies and Phase3 closes at53/91. The final source remains
`325c76ce261a3e87b780713c7aca9a334c21491e`; the bounded receipt test is
accurately recorded as parent-authored under the explicit exception. No third
automated review occurs. All six minors stay unresolved/report-only.
All own completed gate/runtime/auth/work/process cleanup is verified; named
candidate artifacts and curated evidence remain intentionally retained.

The user also directs continuing without trivial questions. Make routine
in-scope choices without asking; material uncertainty, scope expansion and
mandatory quality/publication approvals retain their existing gates.
No auto-design, commit or publication permission is inferred. Continue Phase4.

## Verified receipt fix returned for final disposition — 2026-11-21 01:36

The approved one-method safe-retention correction is fully verified on source
`325c76ce261a3e87b780713c7aca9a334c21491e`. All sixteen final state/recovery
instances pass, preserving every original method/subtest on both cohorts.
Aggregate verification:

| Gate | Result |
|---|---|
| Actual supported-runtime acceptance |31instances+one real Ubuntu22/cp310 seed, including both-cohort SDK isolation and real cp310→cp312 transition/relocation |
| Separate packaged receipt mechanics |3methods on each Python3.10/3.12 interpreter;6instances, not counted as runtime acceptance |
| Local specification/implementation |68specification+24implementation methods |
| Unchanged make check |4,361Rust instances plus other required legs |
| Unchanged full private make verify |4,695Rust instances and allSDK/sanitizer legs |
| Test integrity/scope |Parent-authored receipt oracle red1/controls2→green3 unchanged; AST identical outside completion method, all previous production/test/input identities preserved |

The new regression is parent-authored under the explicit user exception, not
independently authored. Successful atomic publication still consumes the
temporary name; failure retains unselected incomplete state instead of deleting
through a changed root. Corrupt genuinely completed-state rejection stays intact.

Handoff:
`build/distribution/reference-20261117/phase3-receipt-retention/verified-fix.diff`
and adjacent `verified-fix-proof.json`. The diff includes only the completion
method correction and the one new regression file against reviewed fix head
`e5d2cef1…`. Source/oracle/candidate/prior-phase hashes match throughout.

All own red/local/producer/actual/full logs, work, seeds, private display/auth,
scratch and children are cleaned. No jobs/agents remain active. Named private
candidates, original inputs, curated receipts/contextJSON and verification
copies are retained for final disposition. No third automated review, minor
fix, commit or publication occurred.

**Recommended final disposition:** accept this verified RV-Q303 correction and
Phase3 closure. Confidence: high; concrete red→green preservation/retry proof,
all real/full gates and exact one-method scope checked. This is not a new
independent review or a claim of live public TLS/WSL/publication acceptance.
Task3.3.3 remains blocked on this final user decision (DEF-579), progress52/91.
RV-Q303 is not marked resolved until acceptance; six minors remain report-only.

## Receipt-regression authorship exception approved — 2026-11-21 00:39

The user replies **“I approve”** to the explicit limited exception: the parent
authors the receipt regression from the already approved behavioral contract
rather than an independent author. This is not independent authorship and must
not be reported as such. The tool failure is not retried through altered wording.
DEF-578's workflow decision is resolved.

Only authorship changes. Red-before-production, immutable test expectations,
the one-method safe-retention scope, affected local/actual/ABI/full private
verification and final user disposition remain mandatory. No third automated
review, other finding/minor fix, new production surface, commit or publication
is authorised. RV-Q303 and task3.3.3 remain unverified at52/91.

### Receipt safe-retention initial green — 2026-11-21 00:46

The parent-authored behavioral suite has three methods. Before production
edits, the replacement-root canary assertion fails exactly as expected; normal
completion and interrupted-write controls pass, no errors/skips. The fsync
observer forwards the real call, finds the newly flushed file by its inode
rather than a private filename convention, and replaces only this test's root.
After the one-method correction all three pass with the same test hash.

AST comparison confirms the entire state module is unchanged outside
OwnedRoot.complete's body. Remove only failure-path unlink: successful atomic
no-replace publication is unchanged and retained pending files remain in
unselected incomplete state. All prior production/test/accepted-input bytes
are unchanged. `phase3-receipt-retention-red-proof.json` records parent
authorship, explicit exception, red/green and scope; completed red logs are
cleaned. No independent authorship or third review is claimed.

Regenerated real-input candidates retain exact manifests/payloads and freeze
source325c76ce261a3e87b780713c7aca9a334c21491e. Local, original31actual+seed/
ABI/SDK isolation and full private gates are running. The new three passive
receipt methods also run on both supported interpreters against the packaged
module; their child import location is bound to the exact pyz, never the checkout.
Those mechanical results will remain distinct from installed-runtime acceptance.
Final verified diff must return to the user; findings/phase are not closed.

### Receipt regression author unavailable — 2026-11-20 11:42

Independent author `ses_e07744de5ffecBuoLuuU5yIef1` terminates with a tool
safety flag and returns no tests. The proposed test file is absent and the
author directory is empty. Production and all existing test identities remain
unchanged. This is a tool/workflow blocker, not a red test result, new product
finding or reason to weaken verification. No alternate-wording retry or third
review is started.

The user-approved one-method correction remains authorised but unimplemented.
DEF-578 blocks independent authoring. A parent-authored regression from the
already approved behavioral contract is a possible limited workflow exception,
but is not independently authored and therefore requires explicit user approval
before substituting it. Red-before-code, immutable expectations, real/full
verification and final user disposition remain mandatory under either route.
Alternatively leave execution paused until independent authoring is available.
All empty owned author scratch is cleaned; no production edit/commit/publication.

## RV-Q303 bounded correction approved — 2026-11-20 11:39

The user replies **“ok, I approve”** after the plain-language explanation and
the proposed safe-retention remedy. This approves only the existing completion
method correction: retain pending receipts on failure rather than delete through
an unverified absolute pathname. Successful atomic publication still consumes
the temporary name. Add independent red regression/control coverage, regenerate
candidates and repeat affected local/actual/ABI/full private verification.

Return the verified bounded diff for the user's final disposition and Phase 3
closure. No third automated review, new production surface, pruning, minor fix,
commit or publication is authorised. DEF-577's ruling dependency is resolved;
RV-Q303 itself remains open pending verification and final disposition.
Task3.3.3 resumes unverified at52/91; tests precede production edits.

## Single fix-review gate stop: RV-Q303 — 2026-11-20 04:44

All three authorised fix assessments are complete. The five original findings
are addressed, but concurrency reviewer `ses_e09b02f1affei8m6HCXJc4FcQR`
finds a new major cleanup regression. Aggregate Phase 3 closure is blocked
(DEF-577), task3.3.3 stays unverified at52/91. No fix is applied and no third
automated review is authorised or started.

| Finding | Concrete evidence and impact | Existing contract |
|---|---|---|
| RV-Q303 | `native_state.py:450–461` creates a private pending receipt but unconditionally unlinks its absolute name in finally at461. After forwarding real receipt fsync, the review observer renames the owned root, recreates its pathname and a different file at the same pending relative name. Validation rejects the root, yet cleanup deletes that replacement canary; original flushed receipt remains in the moved root. Exact baseline preserves the canary; corrected head deletes it. Same-user root substitution and matching pending filename are required, not a cross-user exploit. | `03-02:257` permits deleting only this invocation's scratch. |

Independent concurrency verification: six new state methods,13state implementation
and7state specification methods pass; exact before/head comparison reproduces
four red state cases→zero with both controls retained. Nineteen methods pass
under each real Python3.10.12 and3.12.3 non-root container. Nested/link/FIFO
cleanup, substituted-workspace retention and completion no-clobber controls pass.
All37source/test hashes, original/new oracles and packaged candidates match.
Full/actual receipts are inspected, not independently rerun.
All reviewer-owned probes, scratch, children and containers are cleaned.

**Recommendation pending user ruling:** make the smallest bounded correction
in the existing completion method: do not unlink pending receipt paths after
failure. Successful no-replace rename already consumes the pending filename
(`native_state.py:459`). Failure can safely retain the unselected private
receipt in incomplete state, which retries already ignore when no final
completion name exists (`native_state.py:421–423`, `03-02:259–263`).
This avoids any deletion through a root pathname whose identity has changed.
The cost is a small retained file in incomplete state, not a selected target.

Descriptor-bound cleanup is also viable but adds tracking that is unnecessary
when safe retention satisfies the existing contract. Do not weaken corrupt
completed-state rejection, remove other old checks or add automatic pruning.
Add an independent red regression for receipt/root substitution plus normal/
interrupted completion controls. Regenerate candidates and repeat affected
local, actual/ABI and full private verification, then return the verified
bounded diff to the user for final disposition rather than automatically
starting a third review. Any newly required material surface stops separately.

Confidence: high. Hardening: independent real-fsync root-substitution probe
with exact baseline/head comparison, plus parent inspection of unconditional
unlink, successful rename and incomplete-state eligibility. No optional feature,
new module/framework, minor fix, commit or publication is proposed.
Keeping execution paused is the alternative; accepting the phase with RV-Q303
unresolved is not viable. User ruling remains pending.

### Correctness fix assessment returned — 2026-11-20 04:43

Reviewer `ses_e09b02f23ffe8xRjHjx8kPs8CE` assesses RV-C301 as addressed
with **no new correctness findings**. Both non-host smokes explicitly select
candidate SDK; host receives no SDK option. Fresh/reuse callers share the
boundary and normal user-driver priority remains unchanged.

Independent runs pass65specification+24implementation methods. Exact baseline
with six frozen regressions reproduces13failure instances across11of18methods,
seven green controls, no errors/skips. Eight real-tool comparisons using exact
before/corrected helpers prove both SDK-override regressions red→green and
preserve controls. These are boundary comparisons, not repeated full bootstrap
acceptance. Source/new and original oracle/implementation/candidate identities
match. Full/actual receipts are inspected, not claimed as independently rerun.

All own reviewer scratch/logs/children/containers are cleaned, no edits made.
Six minors remain report-only. Safety also reports addressed/no new findings;
the concurrency fix assessment still gates aggregate closure.

### Packaging-safety fix assessment returned — 2026-11-20 04:42

Reviewer `ses_e09b02f1cffeXwcoFctKBhRHQg` assesses SA-301 and SA-302 as
addressed with **no new findings**. Component-aware escape rejection precedes
writes, contained link controls pass and all eleven existing SDK links remain
accepted. Locked obligation enforcement rejects missing/wrong notice/source/
review roles and preserves all four exact real support audits.

Independent runs:18new local methods,24implementation methods and16bootstrap
methods pass. Exact-before archive/audit modules with unchanged regressions
reproduce nine failure instances across twelve methods with five passing
controls and no errors/skips; four additional malformed audit shapes reject.
Corrected production, six new regression,23old oracle, two old implementation
files and packaged modules match recorded identities. Manifests/payload
declarations stay identical. Full/actual receipts are inspected, not rerun.

All own reviewer scratch/children are cleaned; no edits or further review.
All six minors and user-owned legal review remain unchanged. Correctness and
concurrency fix assessments are still active, so this is not aggregate closure.

## Single Phase 3 fix-diff review active — 2026-11-20 04:38

Corrected final gates pass on frozen `e5d2cef1c0a951e9331d523928f12626291fd720`:
31 actual test instances plus the real cp310 seed;65 local specification
methods;24 implementation methods; unchanged makecheck4361Rust and full
authenticated private makeverify4695Rust. Original/new oracle, production,
candidate and prior-phase source hashes match. All completed own gate/actual/
local/red/author work, logs, seeds, display/auth and processes are cleaned.
`phase3-correction-acceptance-proof.json` retains the source-bound evidence.

The same three independent reviewers now assess only the authorised fix diff
from reviewed `266f7218…` to corrected `e5d2cef1…`, including direct callers,
parallel paths, red-test integrity and limits. Current owning documentation
merely records later evidence; source/test bytes are frozen.

| Review | Session | Required disposition |
|---|---|---|
| Correctness | `ses_e09b02f23ffe8xRjHjx8kPs8CE` | RV-C301 plus direct impacts |
| Ordinary packaging safety | `ses_e09b02f1cffeXwcoFctKBhRHQg` | SA-301/302 plus direct impacts |
| Concurrency/recovery | `ses_e09b02f1affei8m6HCXJc4FcQR` | RV-Q301/302 plus direct impacts |

This is the single authorised fix-review round; no third automated cycle is
authorised. Review scratch is retained only while the jobs run. The five
findings remain open until assessments return; all six minors stay report-only.
Task3.3.3 remains unverified at52/91 with no commit/publication.

## Phase 3 correction ruling — 2026-11-20 03:23

The user replies **“i apprive”** to the explicit request to approve the five
bounded major fixes. This authorises the proposed three-module correction,
independent additive red regressions before production edits, regenerated
candidates, repeated real-cohort/ABI and full private gates, and one independent
fix-diff review. Preserve all existing assertions, accepted input/payload bytes
and earlier phase source. The six minors remain report-only.

DEF-576's ruling dependency is resolved; the findings themselves are not yet
resolved. Task 3.3.3 resumes unverified at 52/91. No third automated review
cycle, new dependency/module/framework/public command, version bump, commit or
publication is authorised. A material support-surface escalation still stops
separately. Author tests independently before touching the three production
files; the reviewed head remains the correction baseline.

### Approved correction initial green — 2026-11-20 03:51

Independent author `ses_e093a7017ffegtoKh0qbEDHjgA` adds six test files without
reading the forbidden production bodies or altering old tests. Local red is
eighteen methods with thirteen failure instances across eleven methods and
seven passing controls; no errors/skips. Real Ubuntu22/cp310 red has four
methods, two SDK-override failures and two controls. Missing actual contexts
produce explicit errors, excluded from behavioral red evidence. Parent repeats
both local red and exact pre-fix packaged-helper actual red.

Only the three approved modules change: explicit candidate SDK for non-host
smokes, flushed private completion file plus atomic no-replace publication,
descriptor-relative owned-workspace cleanup with safe retention on identity
changes, component-aware bounded link resolution and locked audit obligations.
Python3.10 requires the descriptor traversal because its rmtree lacks dir_fd.
Traversal stays inside the verified workspace; no broad scan/framework is added.

All eighteen new local methods pass with every new expectation unchanged.
Four existing real support audits also pass the tightened obligations.
Corrected source tree `e5d2cef1c0a951e9331d523928f12626291fd720` differs from
reviewed head only in three production modules, six new test files and four
owning records. Manifests, accepted payloads and all old assertions are unchanged.
`phase3-correction-red-proof.json` retains author/parent red and initial green;
completed author/red logs/work/children are cleaned.

Regenerated candidate real-cohort/ABI acceptance, eight new actual SDK-isolation
methods, existing/local and full private gates are active. Findings stay open
until corrected verification and the one authorised fix-diff review pass.
All six minors remain report-only, including unchanged producer success output.

## Phase 3 consolidated review gate — 2026-11-20 01:25

All three independent read-only reviews are complete. Aggregate findings:
five major, one minor, no critical. Parent confirms the cited source and
contract boundaries. No remediation is applied. DEF-576 blocks task 3.3.3
and later phases at 52/91 despite the honestly passing final-source gates.

| Finding | Independent observed reproduction and impact | Existing contract / minimum correction proposed |
|---|---|---|
| RV-Q301 | `native_state.py:400–404` writes the final completion filename directly. A real 128-byte RLIMIT_FSIZE causes EFBIG and a partial receipt; a separate child dies from SIGXFSZ during that write. `completed()` at371–374 parses the partial final receipt and retry fails after the lock releases. | `03-02:259–263` requires retry to ignore incomplete state. Flush a complete receipt under private staging and atomically publish its final filename; do not weaken rejection of corrupt completed state. |
| RV-Q302 | `native_state.py:300–302` cleans TemporaryDirectory through its absolute name. Rename the owned root, recreate that name and matching workspace entry with a canary; validation rejects replacement but cleanup deletes the replacement entry/canary and leaves original scratch. Same-user root substitution is required; no cross-user claim. | `03-02:257` permits deleting only this run's scratch. Retain workspace identity and avoid cleanup if root/workspace identity cannot be proved, or anchor cleanup to held directory identities. Prefer the smaller safe fail-closed route; no scan or general cleanup framework. |
| SA-301 | `native_archive.py:419–423` normalises `..` before intervening symlinks. With shallow/data, outside, dir/link→../shallow and escape→dir/link/../../outside, extraction accepts a link resolving outside staging; direct read reaches the reviewer's private outside canary. The shipped helper reaches preparation with that link. | `03-02:270–277` requires containment rejection before writes. Resolve components and encountered links before later `..`, following the existing bounded approach at `prepare_ndk.py:103–132`, while retaining legitimate internal links. |
| SA-302 | `install_native.py:261–280` permits emptied notice/source audit arrays without consulting the lock's source_required. Removing miniature LHA notices/source and updating coherent audit/preparation hashes reaches the real failing preparation child; producer validation rejects the same missing obligations. No activation or successful runtime is claimed. | `03-02:145–148,166–168` requires existing audit obligations. Bind installer validation to the selected lock; enforce notice, required-source, role/origin and locked review-hash requirements before extraction/use, consistent with `support_audit.py:181–218`. No legal clearance is inferred. |
| SA-303 | `package_release.py:228` prints the absolute success path unsanitised. Printable public args plus a private CWD containing ESC[31m prints the control. No command execution is claimed. | Minor/report-only. Record but do not fix under this ruling request. |

RV-C301 and its evidence are detailed below. Both fresh/reuse smoke paths,
all state-method callers, shared/parallel producer consumers and driver
selection precedence were traced. Focused independent verification totals
47 local specification plus24 implementation methods; red controls reproduce
the baseline oracle and pre-hardening guard failures. Full/actual receipts
are inspected, not falsely reported as independently rerun.
All reviewers confirm31 production/test paths,23 oracles and candidate bytes
match the frozen tree. All reviewer-owned probe files, directories, containers
and children are cleaned; no licensed media/emulator/publication work occurred.

**Recommended ruling:** author additive independent regressions first and prove
each major red, then correct only the five necessary boundaries in
`tools/setup/install_native.py`, `native_archive.py` and `native_state.py`.
New independent regression files and only necessary test-fixture/registration
additions may exercise inherited SDK selection, interrupted receipt publication,
workspace substitution, component-aware links and locked audit obligations.
Do not rewrite existing assertions or weaken corrupt-complete rejection.
Preserve the closed stdlib helper, public options, accepted lock/payload inputs,
normal driver priority and all earlier phase bytes.

Regenerate helper/shell candidates; rerun affected local, both real-cohort
acceptance/ABI and full authenticated private-display gates. Then perform
one independent fix-diff review, not a third automated cycle. Keep the six
minors (five old plus SA-303) report-only. No new dependency, module, service,
framework, command, broad filesystem scan, version bump, commit or publication
is proposed. If a material support surface proves necessary, stop separately.
The alternative is to keep execution paused with the working diff untouched.
Accepting the phase while these majors remain unresolved is not viable.

Confidence: high. Hardening: three independent role reviews with concrete
bounded real-tool/filesystem/public-helper reproductions; parent reads the
specific code, existing reference validators and authorised contracts.
User ruling remains pending; no eligible delegated authority is active.

## RV-C301 correctness review stop — 2026-11-20 01:22

Correctness reviewer `ses_e09b02f23ffe8xRjHjx8kPs8CE` reports one major finding.
Parent confirms the inherited environment and SDK-selection precedence directly.
Task 3.3.3 is blocked (DEF-576); no remediation is authorised or applied.
Packaging-safety and concurrency reviews remain active for consolidated findings.

| Evidence | Finding |
|---|---|
| `tools/setup/install_native.py:112–115,431–433` | `_run` preserves CLORRAINE_SDK and both profile smokes omit an explicit candidate SDK. |
| `compiler/src/clorraine/environment.rs:45–65` | Explicit sdk-root wins over environment, which wins over adjacent installed discovery. |
| `03-02-installation.md:290–291` | Required smokes must use the candidate's tools. |
| Independent actual reproduction | Exact shipped helper plus accepted compiler binaries in a private candidate with no SDK: without override hosted-amiga rejects, but with CLORRAINE_SDK pointing to separately mounted accepted SDK/venv, all three real smokes pass in Ubuntu22/cp310. No successful process result is mocked. |
| Blast radius | Both initial candidate and completed-version reuse call `_smoke` (`install_native.py:461,464`). Existing acceptance fixtures remove CLORRAINE variables, leaving this inherited-selection case uncovered. |

The reviewer independently passes all 47 local specification and 24
implementation methods. Baseline plus unchanged oracle reproduces 47 methods
with 70 expected failures and no collection errors/skips. All 31 production/test
paths, 23 oracle hashes and retained candidate hashes match the frozen head.
Full/actual gate receipts are inspected, not claimed as independently rerun.
All reviewer-owned scratch/containers are cleaned; no product/test/docs changes.

**Proposed minimum correction, pending user ruling:** explicitly pass the
candidate's `sdk/amiga` with existing `--sdk-root` for each non-host smoke.
The host smoke must remain free of that option, which the driver rejects for
host (`cli.rs:359`). This reuses existing driver priority (`environment.rs:45`)
without changing normal user SDK selection or adding a new interface.
Removing CLORRAINE_SDK only from smoke children is also viable, but explicit
selection states the tested target directly. Add independent red regression
coverage for a broken candidate plus valid external override and a valid
candidate plus stale override; retain all existing expectations and rerun
affected actual/full gates followed by one fix-diff review.

Confidence: high. Hardening: independent real-tool reproduction and parent
inspection of both smoke callers and driver precedence. No optional support
surface is proposed. Phase acceptance with this unresolved major is not viable;
the user may approve the bounded correction or keep execution paused.
The final ruling request awaits the two other review reports.

## Phase 3 independent reviews dispatched — 2026-11-20 01:16

Final hardened source `266f7218206afac1a1c88248ee1c098cba6f4544` is reviewed
against Phase 3 baseline `1ecded4acc2c3196b670ef6edf2c1856c677dd2a`.
All final-source gates pass: twenty-three actual instances plus a real cp310
seed and full authenticated private-display `make verify` with 4,695 Rust
instances. Source/oracle/candidate/prior-source hashes match; all owned
completed actual/full-gate work, logs, seeds, authentication and processes
are cleaned. `phase3-hardened-acceptance-proof.json` retains the evidence.

| Review | Session | State |
|---|---|---|
| Correctness/contracts/integration | `ses_e09b02f23ffe8xRjHjx8kPs8CE` | Complete; RV-C301 major |
| Ordinary local software-packaging safety | `ses_e09b02f1cffeXwcoFctKBhRHQg` | Complete; SA-301/302 major, SA-303 minor |
| Concurrency/filesystem/recovery | `ses_e09b02f1affei8m6HCXJc4FcQR` | Complete; RV-Q301/302 major |

Named roles are not callable in this session; each uses an independent generic
agent with the complete read-only role packet and no reduced coverage.
Every packet includes the original goal/minimum design, approved complexity,
exact scope/contracts/tree diff, caller/parallel-path blast radius,
assertion-red integrity, final gate results and owned-scratch cleanup.
No Amiga shim/module/gate/registry or compiler emission changes are in this
phase diff; the previously accepted specialist surfaces remain unchanged.

Reviews cannot edit source/tests/plans, fix minors, use licensed media or launch
emulators. Review scratch remains only while these jobs are active. Any
major/critical finding requires the user's ruling; no finding is waived by
green gates. Existing five report-only minors and user-owned legal review remain
unresolved. Task 3.3.3 stays unverified at 52/91; no commit or publication.

## Phase 2 final acceptance — 2026-11-19 15:12

The user replies **“ok”** to the explicit final-disposition question:
**“Accept the RV-301 fix and Phase 2 closure?”** This accepts the retained
verified bounded diff and aggregate phase closure after independent red,
targeted, actual cohort/runtime and full private-display verification.
The parent rechecks exact source/diff identities, successful gate receipt and
completed cleanup before promotion. SR-202 and RV-301 are now resolved;
all Phase 2 major findings are closed by their recorded fixes/rulings.

No third automated review, minor-finding fix, commit or publication occurred.
RV-001/SA-001/SA-002 and SA-201/SA-202 remain unresolved/report-only.
Task2.3.4 verifies and Phase 2 closes at36/91. Named candidate artifacts and
non-media evidence remain intentionally retained; all owned scratch/media/
authority/processes are cleaned. Phase 3 specification work follows.

## RV-301 verified handoff for final user disposition — 2026-11-19 15:01

The authorised bounded correction is fully verified: independent red oracle,
native110/implementation8/historical22/LSP35/core26, both actual14-method
Ubuntu/Python cohorts and packaged alias/rebuild/guest-runtime controls pass.
The full authenticated private-display `make verify` also passes every leg,
with 4,695 passing Rust instances including sanitizer reruns. Split stdout/
stderr capture avoids the earlier summary-interleaving limit.

The retained diff against reviewed head `32b5461d...` changes only the storage
call/signature and canonical current-program exclusion in `emulator.rs`, adds
the independent artifact-alias module and its registration import. Whole-root
and comma protections remain intact. Source/oracle hashes match frozen
`a88c7351d7b78049b122bb0140717c28a9c37085` throughout verification.
No other old test or producer source changes.

Handoff artifacts:
`build/distribution/reference-20261117/phase2-artifact-alias/verified-fix.diff`
and adjacent `verified-fix-proof.json`. All owned gate/runtime/media/auth/
build/author scratch and children are cleaned. Named candidate artifacts and
non-media evidence remain intentionally retained.

Per the user-approved cap, no third automated review is started. Return this
verified diff to the user for final RV-301 disposition and Phase 2 acceptance.
Task2.3.4 stays blocked on that decision (DEF-573), progress35/91, with no
phase promotion, commit or publication. All report-only minors remain open.

## RV-301 actual revalidation green — 2026-11-19 14:46

Both actual isolated Ubuntu/Python cohorts pass twelve archive/stage plus two
checkout-free installed/runtime methods on new identical archive
`eed3a17570ebd6fd312d808f28f724d9f3f4a10f1a2d1e42aa35a4e8f8738fb0`.
Actual packaged-driver traces reject four direct/aliased temporary parents
within the symlinked current-program output before any FS-UAE exec. Both
safe real profile runs show Hello on direct screenshot inspection; rebuilding
their aliased output changes actual source-artifact bytes while live mounted
copies remain unchanged. Both process exits/waits/cleanup pass.

`phase2-artifact-alias/actual-acceptance-proof.json` links new cohort and runtime
evidence; previous artifacts remain separate. One initial test-only alias
used a host absolute target inaccessible inside the isolated namespace. Its
unaccepted evidence is preserved; correcting to a relative alias makes the
intended test pass without product/oracle changes.
All owned media/auth/runtime/build scratch is cleaned. The full private gate
is still running; final user disposition remains required, no third review.

## RV-301 bounded correction targeted green — 2026-11-19 14:37

Independent implementation-blind author adds four cases: two alias-output
rejections fail before the fix and two safe-parent controls already pass.
Parent reproduces that exact red/control result and verifies all 106 previous
expectations unchanged, then restores the canonical current-program check in
conjunction with the whole-root and comma guards. No other production file or
old assertion changes. `artifact-alias-authoring-proof.json` retains the new
oracle identities/red result; `artifact-alias-implementation-proof.json`
records native110/implementation8/historical22/LSP35 and core26 green.

Actual Ubuntu22 rebuild and full private `make verify` are running against
frozen correction code. Actual cohort/runtime verification and final user
disposition remain required. No automated third review is dispatched;
progress stays 35/91 and the finding is not yet closed.

## RV-301 user ruling — 2026-11-19 14:30

The user replies **“ok”** to the explicit recommendation to retain the old
canonical current-program exclusion alongside the whole-artifact-root and
comma checks, add independent failing regressions for both profiles, repeat
affected actual/runtime and full private verification, and return the verified
diff to the user for final disposition. No third automated review cycle,
new abstraction, scanning, unrelated fix, commit or publication is authorised.
DEF-572's correction-ruling dependency is resolved, but RV-301 and aggregate
phase acceptance remain open pending that verification and final disposition.
Progress stays 35/91; specification authoring precedes production code.

## Fix-review gate stop — 2026-11-19 13:49

All four fix reviews are complete. Correctness reviewer
`ses_e0cb29fe9ffePM1ATp9vHihN27` finds a new major regression, RV-301.
The earlier specialist resolution of SR-202 covers the new whole-root
cases but does not cover the program artifact-directory alias. Aggregate
disposition is therefore partial, not resolved. SR-201 and RV-C201 remain
addressed; SA-201/SA-202 and prior minors remain unresolved/report-only.

| Finding | Reproduction and observed impact | Minimum remedy, not authorised/applied |
|---|---|---|
| RV-301 | `compiler/src/clorraine/emulator.rs:187–202` removes the prior current-program canonical-directory exclusion. Make `workspace/build/clorraine/hello` a symlink to an external artifact directory; use its profile child as TMPDIR; run hello with a waiting observer and rebuild the same source/profile. On both profiles, the prior reviewed driver rejects before launch, but corrected head launches and the rebuild deletes live drive/program files; the waiting driver later reports cleanup failure. Canonical artifact root alone cannot contain a target reached through a child alias. | Retain the prior canonical current-program-directory check alongside the whole-artifact-root and comma checks. Add independent artifact-directory-symlink regressions on both profiles, without scans or new abstractions. |

| Correctness fix-review verification | Observed result |
|---|---|
| Existing gates | `make check` passes with 4,357 Rust instances, Python29/113/103/28 and fuzz28. Focused native106/implementation8/historical22/LSP35 and core26 pass despite missing this alias case. |
| Regression integrity | Exact prior/head synthetic-process comparison reproduces both-profile rejection versus live-file deletion. Authoring hashes and narrow AR-43 method/helper amendment checked; unrelated assertions unchanged. |
| Direct scope | One storage allocator caller and both native profiles affected. Other dispatch/capture/LSP/producer paths unchanged; snapshot fix and comma protection addressed. |
| Retained evidence and cleanup | New archives/screenshots and cohort/full-gate receipts inspected, not rerun. Count-extraction limit correctly accounted for. Read-only, no media/emulator access; all own scratch/children cleaned. |

**Recommendation pending user ruling:** apply only the conjunction of old and
new canonical exclusions, author the new cases red before code, and repeat
affected actual/runtime plus full private verification. Return the verified
diff to the user for final disposition rather than starting a third automated
correction/review cycle. Current code shows the lost check at
`emulator.rs:187–202`; the original reviewed tree retains it.
Confidence: high. Hardening: independent exact before/head process reproduction
on both profiles and parent inspection of the removed check. No third review
is automatically authorised, no new machinery is proposed, and no fix is made.
The user may keep execution paused instead; phase acceptance with this
unresolved major finding is not a viable option.

## Single fix-diff review underway — 2026-11-19 13:38

Corrected full private-display `make verify` passes and source/oracle hashes
match frozen corrected head `32b5461d1eb139a92dee8bc5c9e58ca99603726e`.
Gate credentials/server/log scratch is cleaned. The same four reviewers
receive one read-only fix-diff review from `e7843384...` to `32b5461d...`,
including the separately approved AR-43 identity-oracle amendment.
The corrected full-gate receipt and honest summary-extraction limit are
recorded in `phase2-correction-full-verify-proof.json` and
`phase2-correction-summary-integrity-proof.json`. No third review round is
authorised. All three major findings remain open until returned assessments;
minor findings stay report-only. No source edits while reviews run.

### Packaging-safety fix review — 2026-11-19 13:46

Reviewer `ses_e0cb29fc0ffevlSRPjo17eUOa5` returns **no new findings** on the
exact frozen fix diff. It assesses SR-201/SR-202/RV-C201 as corrected:
canonical comma rejection precedes storage/launch, complete artifact-tree
exclusion covers sibling sources/profiles/aliases while safe siblings remain
supported, and the same private snapshots bind inventory/ELF/version/archive
without repairing non-executable originals.

| Independent verification | Observed result |
|---|---|
| Focused Rust / local core | 171/171 SDK Rust and 26/26 local core methods pass. |
| Actual archive recreation | Independently recreates corrected archive `364376da...580eb2a` exactly; no-clobber rejection preserves existing output. |
| Retained actual evidence | Both cohort archives/staged bytes and all 37 native-runtime evidence hashes match. Six actual rejection traces show status1/TMPDIR guidance/no FS-UAE exec. Actual contexts/full-gate receipt inspected, not substituted or rerun. |
| Oracle integrity | AR-43 identity method/helper only; all other original core test methods structurally unchanged. Queried/supplied/archived bytes, literal argv and cleanup retained; recorded red evidence checked. |
| Direct effects and hygiene | No new safety impact; SA-201/SA-202 still minor/unresolved. Read-only; no emulator/media reads, all own scratch removed and children joined. |

This is one role's assessment, not aggregate closure. Correctness, concurrency
and Amiga fix reviews remain pending; the three major findings stay open.

### Amiga fix review — 2026-11-19 13:47

Specialist `ses_e0cb29fb6ffe3UuOHJh0K1a7dG` explicitly resolves SR-201
and SR-202 with **no new findings**. Canonical comma rejection and complete
component-aware artifact-root exclusion pass the original boundary probes,
aliases, cross-source/profile directories and supported safe sibling cases.

| Independent verification | Observed result |
|---|---|
| Focused suites | Native106/implementation8/core26 pass. |
| Actual packaged-driver probes | Twelve unsafe rejections and six safe launch/cleanup controls pass, including a symlinked artifact root. Original retained driver still launches at both unsafe boundaries on both profiles, proving regression integrity. |
| Oracle and actual evidence | Narrow AR-43 amendment checked; all 37 runtime hashes, six no-exec traces, both guest Hello images, effective A1200/KS3.1/single CNEXT/private state and corrected cohort identities checked. Full-gate extraction limit/control accounted for. |
| Scope and cleanup | All relevant storage/profile/parallel paths reviewed; no SDK/runtime/flag/library changes or A500 claim. Read-only, no real emulator/media access, own scratch removed and children reaped. |

### Concurrency fix review — 2026-11-19 13:47

Reviewer `ses_e0cb29fb8ffeIdc5N6yvWzdWiW` explicitly resolves RV-C201
with **no new findings**. The old public-command race reproduces against the
exact pre-fix producer; the corrected producer rejects the wrong-version
private snapshot without output or private leftovers.

| Independent verification | Observed result |
|---|---|
| Additional data-integrity probes | Mutating an existing private ELF snapshot is rejected by final hashes; rotating an input-directory alias during query cannot change archived snapshot bytes; a raced destination is preserved on publication refusal. Caller inputs stay intact. |
| Focused suites and oracle | Core26/native106/implementation8 pass. AR-43 identity/helper-only amendment, literal argv/tool/PATH controls and unrelated expectations remain intact. Permission, ordinary-file, deterministic-output and cleanup boundaries checked. |
| Actual retained evidence | Both new cohort archives/eight staged core members and all 37 runtime hashes match; old archive remains separately identified. Both Hello screenshots, six prelaunch rejections and distinct CNEXT mounts checked. Full-gate receipt inspected without inventing a full instance count. |
| Scope and cleanup | Canonical storage cases reviewed; selection/host fallback/capture/legacy/stage unchanged, no installer activation. Read-only, all own children joined and scratch removed; reviewed non-plan files still match corrected head. |

Packaging-safety, Amiga and concurrency fix reviews are clear. The correctness
fix review is the only pending role; task 2.3.4 remains unverified at 35/91.
Minor findings remain unresolved/report-only, and no third review is started.

## Correction verification in progress — 2026-11-19 13:22

All three approved corrections are implemented after parent-verified independent
red tests. Native106/implementation8/historical22 and local core26 pass.
AR-43's explicitly approved independent identity assertion and C observation
helper bind actual queried bytes to supplied/archived bytes; all unrelated
expectations stay unchanged. Source/oracle hashes are frozen in
`phase2-correction-implementation-proof.json`.

Both corrected Ubuntu/Python cohorts pass the actual twelve archive/stage and
two installed/runtime/relocation methods with identical core archive
`364376dabebaf582cfd0b80eb9644e6d1354e882f8118c6a22a5d74cf580eb2a`.
Actual packaged-driver tracing proves six unsafe-parent rejections before
FS-UAE launch, and both real profile runs display Hello with private
A1200/KS3.1/CNEXT mounts/state/media and normal exit/cleanup.
`phase2-corrections/actual-acceptance-proof.json` links exact new evidence;
old artifacts remain preserved. Runtime/media/auth/build scratch is cleaned.
Full private `make verify` is still running. No major finding is closed until
that gate and the one permitted fix-diff review complete.

## Approved correction execution — 2026-11-19 12:12

The user replies **“I approve”** to the complete three-fix recommendation:
reject comma-containing canonical staging parents, exclude the current
workspace's entire resettable artifact tree, and validate/archive the same
private executable snapshot. Approval includes independent red regressions,
affected actual artifact/runtime reproof, full private-display `make verify`
and one fix-diff re-review. No broader redesign, dependency, public operation
or minor-finding remediation is approved.

The ruling dependency DEF-570 is resolved, but all three findings remain
unresolved pending implementation and verification. Task 2.3.4 resumes as
implemented/unverified; progress stays 35/91. Existing oracle assertions,
candidate/evidence identities and no-commit/publication boundaries remain.

## Phase 2 consolidated ruling packet — 2026-11-19 11:28

All four independent reviews are complete. Aggregate: three major findings,
two minor report-only findings, no critical findings. Task 2.3.4 remains blocked
at 35/91; no implementation change, waiver, commit or publication is authorised.
The accepted artifacts still match their retained identities; none of these
reproductions establishes corruption of those stable-input artifacts.

**Recommendation:** fix SR-201, SR-202 and RV-C201 using only the existing
emulator storage checks and producer's private work directory. Add independent
red regressions before code, retain existing assertions, rerun affected actual
artifact/runtime proof and full private-display `make verify`, then perform
one fix-diff re-review. Leave SA-201/SA-202 report-only unless separately
authorised. No new dependency, framework or public operation is needed.
The user may instead keep execution paused; accepting the current gate as
complete while these major findings remain unresolved is not a viable path.

| Minimum correction | Grounding and bounded change |
|---|---|
| Reject comma-containing canonical staging parents with TMPDIR guidance | Existing `Storage::create` and `safe_path`, `compiler/src/clorraine/emulator.rs:188–190,238–242`; check before child launch, preserving ordinary supported paths and no settings rewrite. |
| Exclude the entire current workspace's resettable artifact tree | Existing artifact root `compiler/src/clorraine/artifacts.rs:16–24` and reset `:55–60`; replace the current-binary-only storage check, without changing build layout or deleting stale files. |
| Query versions and archive the same private executable snapshot | Existing private sibling work at `tools/distribution/package_core.py:189–190`; inventory/query/archive the same copied input bytes instead of querying mutable original paths at `:191–194` and reopening them at `:209–210`. Preserve exact member bytes, deterministic archive, no-clobber output and caller inputs. |

Confidence: high. Hardening: independent version-specific parser analysis,
live external-observer cross-build reproduction, and deterministic public
producer race with extracted-executable verification. Parent checked the
relevant code paths. No optional broader redesign is proposed.

## Phase 2 concurrency/data-integrity review

Received 2026-11-19 11:28 from independent read-only reviewer
`ses_e0cb29fb8ffeIdc5N6yvWzdWiW`. One major new finding, RV-C201.

| Finding | Evidence and reproduction | Minimum remedy, pending ruling |
|---|---|---|
| RV-C201 | `tools/distribution/package_core.py:187–194,209–213` inventories originals, queries mutable original executable paths, then reopens originals for archiving. Supply inventoried ordinary ELF clorraine reporting 9.9.9; pause actual clc version child after inventory, substitute clorraine reporting 1.1.2 for its query, then restore inventoried bytes before archiving. Public archive command returns 0; core.json claims 1.1.2 but extracted executable reports 9.9.9. No product functions mocked. Final archive hash binds inventory to archive, not queried executable bytes. | Validate versions and archive the same private executable snapshot. Add a deterministic regression with actual process barriers and extracted-byte/version inspection. |

| Independent verification | Observed result |
|---|---|
| Focused suites | Native 96, implementation eight and local core 23 pass. |
| Cancellation/wait/stale controls | Surviving children retain bytes after SIGTERM/SIGKILL; injected wait EIO cleans only after confirmed reap and otherwise retains storage; directory/alias collisions preserve protected canaries. |
| Publication and mutation | Concurrent file writers yield one winner; twelve directory publishers and existing destinations do not clobber. Changed archive bytes before inventory or during extraction reject. |
| Actual retained evidence | Identical cohort archives, eight core members, paired receipts and all 76 native FS-UAE evidence hashes match; screenshots show Hello and logs support measured isolation/lifetime claims. |
| Blast radius and integrity | All build/run/host-fallback/profile/capture/diagnostic branches inspected; meaningful selection/lifetime assertions fail under old dispatch/shared storage. No additional lifecycle finding from this review; specialist findings remain separate. |
| Limits and cleanup | Full-gate receipt inspected, not rerun; installer activation out of scope. Source/tests still match reviewed tree; all own probes/children/scratch removed. |

## Phase 2 Amiga platform review — blocking findings

Independent read-only specialist `ses_e0cb29fb6ffe3UuOHJh0K1a7dG` reviewed the
same frozen base/head trees named below. No fixes are authorised or applied.
Task 2.3.4 is blocked pending the user's ruling; all reviews completed
against the unchanged implementation. Earlier passing gates remain historical
evidence, not proof that these uncovered cases are safe.

| Finding | Evidence and reproduction | Proposed minimum remedy, pending ruling |
|---|---|---|
| SR-201 | `compiler/src/clorraine/emulator.rs:188–190,238–242`: a comma-containing canonical temporary parent passes validation. FS-UAE 3.2.35 embeds the drive path in comma-delimited `filesystem2`; upstream `config-drives.c:204–207` and `cfgfile.cpp:3887–3891` show truncation at the first comma. With sibling `outside` and `outside,0` directories and the latter selected as TMPDIR, both profiles generate a drive path parsed as writable `outside`. Packaged-driver/fake-child probes establish generated arguments; the mount consequence is grounded in the version-specific parser, not an actual boot. | Reject comma-containing canonical staging parents before child launch, identify TMPDIR and give a safe-path remedy. Add an independent regression that rejects without child launch or outside access. |
| SR-202 | `compiler/src/clorraine/emulator.rs:191–195` excludes only the current binary's parent. `compiler/src/clorraine/build.rs:127` resets another selected program directory independently. Build `other.clo`, choose `build/clorraine/other/hosted-amiga` as TMPDIR, launch `hello.clo` with a waiting observer, then rebuild `other.clo`: the live observer loses its drive/program and the driver later reports cleanup failure. Private state and conditional media copies share the deleted root. | Exclude the current workspace's complete resettable artifact tree, with a cross-source/profile rebuild regression that fails before the fix. |

| Specialist verification | Observed result |
|---|---|
| Frozen scope and focused reruns | Diff hash and reviewed bytes match; native 96/96, implementation 8/8 and core local 23/23 pass despite these missing cases. |
| Blast radius | Every build/run caller and profile/mode/capture/diagnostic/legacy parallel path reviewed; both native Amiga profiles affected. |
| Actual retained evidence | Both candidates' 1,372 prepared files and receipts checked; both guest Hello images directly inspected; effective A1200/KS3.1/single CNEXT/private initialization evidence reviewed. |
| Test integrity | Original oracle hashes and recorded red/intentional compatibility-green evidence checked; no assertions changed. |
| Limits and cleanup | No real emulator/full gate relaunched, no licensed-media reads, no A500 claim or Ubuntu-cohort substitution. Reviewer reports all owned scratch removed and children reaped. |

## Phase 2 ordinary local software-packaging safety review

Received 2026-11-19 11:23 from independent read-only reviewer
`ses_e0cb29fc0ffevlSRPjo17eUOa5`. Exact reviewed trees:
`cadcbce6fc84c9c88701ff16b0080018908edefb` to
`e7843384caf7ca2d4c4b40bc825fb018c856e05a`.
No critical or major findings. Both findings below remain minor/report-only;
neither is fixed, waived or represented as resolved. All reviews are now complete.

| Finding | Evidence and observed impact | Minimum remedy, not implemented |
|---|---|---|
| SA-201 | `compiler/src/clorraine/native_metadata.rs:140–172`: malformed wheel tags, version `invalid:version`, duplicate wheel filenames, wrong filename root and package `extra--` pass native receipt validation. Independent mutations of complete miniature receipts returned 0 and executed compiler/runner rather than rejecting before execution. Native Amiga Run, including capture, is affected. No extraction escape or hostile-installation authentication bypass demonstrated. | Enforce the existing wheel schema's normalization, version, filename uniqueness/layout and tag rules; add rejection controls without requiring a retained wheel cache. |
| SA-202 | `tools/distribution/package_core.py:417`: successful archive output prints a literal path containing `ESC [31m` to stdout. Repeating against the existing destination correctly rejects with sanitized stderr. Shared success reporting affects archive and stage; terminal manipulation demonstrated, not command execution. | Sanitize human-facing success output while retaining literal filesystem arguments; add a control-byte success-output test. |

| Independent verification | Observed result |
|---|---|
| Focused pinned-SDK Rust / local core tests | 161/161 and 23/23 pass. |
| Archive controls | Thirteen unsafe-member cases reject before output creation; valid internal link succeeds. |
| Receipt/publication controls | FIFO, alias, oversized, escaped-duplicate and leading-zero inputs reject before tools; existing files, empty directories and dangling links preserved. |
| Actual retained evidence | Both core hashes, all eight payload hashes, complete support audits/lock and source bindings checked. Actual fourteen-method cohort proofs and full gate inspected, not rerun or substituted. |
| Test integrity and lifecycle | Prior native/media/LSP red controls inspected; current assertions unchanged. All profile/capture/process paths and retained emulator isolation evidence reviewed; no real emulator launched. |
| Cleanup | Reviewer reports all owned probes/scratch removed and commands joined; read-only, no media/key reads. |

SA-001 and SA-002 remain pre-existing deferred findings; this review establishes
no new caller expansion. The exact approved ordinary libamiga bytes match.
Legal review remains user-owned, with no clearance or publication claim.

## Phase 2 correctness review — no new findings

Received 2026-11-19 11:26 from independent read-only reviewer
`ses_e0cb29fe9ffePM1ATp9vHihN27`, covering the exact frozen base/head trees.
No critical, major or minor findings through correctness, maintainability,
standards and API-surface lenses. This verdict does not dismiss the Amiga
specialist's SR-201/SR-202; aggregate phase closure remains blocked.

| Independent verification | Observed result |
|---|---|
| Diff and current implementation identity | Packet SHA-256 matches; all 31 implementation/test files still match reviewed head. |
| `make check` | Passes twice; reported 4,347 Rust tests, Python groups 29/113/103/28 and fuzz 28 with zero failures. |
| Focused SDK Rust suites | Native 96, implementation eight, historical driver 22, historical LSP 32 and additive LSP three pass. |
| Core local and extra boundary controls | Local 23/23 and ten additional rejection/dispatch probes pass, including aliases, unused companions, malformed/deep/invalid-UTF-8 descriptors, escaped duplicates and FIFOs. |
| Actual retained evidence | Both core archives independently hash to the accepted identical identity; actual cohort 12+2 receipts and both guest Hello images checked. Full-gate receipt inspected, not rerun. |
| Blast radius | All selection paths, three build callers, freestanding host compile, two run callers, both profiles, capture/error/lifetime branches, both producer commands and exact LSP fast path reviewed. |
| Test integrity | Original 94 expectations and accompanying frozen files match red-proof identities; six intentional compatibility passes justified. Media/LSP additive assertions distinguish missing behavior; missing actual contexts remain errors, not skips. |
| Cleanup | Read-only; all own scratch removed and children joined. No emulator launched or licensed-media reads. |

RV-001/SA-001/SA-002 remain pre-existing deferred findings, with no new impact
established by this review. Concurrency results are recorded above.
