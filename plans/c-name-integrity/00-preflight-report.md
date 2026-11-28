## Preflight Report: C Name Integrity (implementation plan)

> **Status**: ✅ PREFLIGHT PASSED — all 21 findings resolved (iteration 2 verification below)
> **Iteration**: 2 (first scan, fixes applied, bounded re-scan)
> **Artifact**: implementation plan at `plans/c-name-integrity/` (all ten documents)
> **Audited revision**: commit `733ddeec`; blobs `00-ambiguity-register.md` b76fb0ab, `00-index.md` 8f15ce1c, `01-requirements.md` 9c32000a, `02-current-state.md` 4777b34b, `03-01-source-names.md` 1f3d6a2f, `03-02-generated-symbols.md` f96ab916, `03-03-line-mapping.md` 47001669, `03-04-driver-paths-and-docs.md` 9c8265cb, `07-testing-strategy.md` d6223851, `99-execution-plan.md` 5c1f44ab
> **Codebase Grounded**: compiler at `5c3316ea` (no `compiler/` change since the plan's baseline `7f63bb3c`); about 40 source and test files examined, every file:line reference in 02, 03-*, the register notes, and 99 re-checked; probes with `target/debug/clc`, host `cc`, MinGW, and m68k GCC in `clorraine-sdk:windows-cli`
> **Independence**: uniform-effort (five clustered general agents with the preflight-auditor brief, one blind design challenger; same model family as the planner)
> **Mode**: `--auto-design` (policy version 1, root invocation `ad-pf-7b2e91c4`), strict scope; fixes applied under the user's `--auto-commit` instruction
> **Last Updated**: 2026-11-28

⚠️ The plan was written in an earlier session by the same model family. Same-agent bias risk is
reduced by a fresh session and independent clustered auditors, not removed. The plan changes the
spelling of names in every program's C, so a human compiler reviewer is worth adding before
Phase 2.

Audit target: the ten plan documents. Context documents (evidence only): `spec/01-lexical.md`,
`spec/17-codegen.md`, `docs/known-limitations.md`, `site/guide/known-limitations.md`,
`THIRD_PARTY_NOTICES.md`, the WS3 findings and probes, `compiler/src/**`, `compiler/tests/**`,
`compiler/runtime/**`, `docker/`, `tools/vscode/src/actions.ts`, AGENTS.md. Modification set:
the plan folder and `plans/00-roadmap.md`.

Domain lenses: compiler and language (emission, naming, IR flag); data and migration does not
apply (emitted C is regenerated on every build; no persisted format). Both were selected by the
plan's coverage map.

### Codebase Context Summary

**Tech Stack:** std-only Rust workspace (`compiler/`), C99 runtime shims, emitted C built by host
GCC, MinGW-w64, m68k GCC (nix20, nix13), and vbcc.
**Architecture:** `clc` pipeline lex → parse → load/resolve → sema → own (IR) → codegen; codegen
`run()` (`codegen.rs:658-680`) writes header, prefix shim, typedefs, enum macros, structs, closures,
prototypes, Box helpers, globals, initializers, bodies, hook support, entry stub, then the
hosted-amiga suffix (`profile.rs:18-28`). The `clorraine` driver calls `clc` in process
(`clorraine/build.rs:65-79`).
**Key Files Examined:** `codegen.rs` (reservation, `user_symbol`, `unique_local`,
`unique_param_names`, `sanitize_identifier`), `codegen/{expr,program,function,drop,guards,calls,
cleanup}.rs`, `ir.rs`, `own.rs`, `own/lower.rs`, `own/call.rs`, `types/mod.rs`, `types/keys.rs`,
`sema/mod.rs`, `ast/mod.rs`, `profile.rs`, `driver.rs`, `clorraine/{build,artifacts,platform}.rs`,
`compiler/tests/{codegen_run,amiga_optional,clorraine_driver,callable_release_integrity}_spec_test.rs`,
`compiler/tests/compiler_robustness_impl_test.rs`, `docker/{Makefile,compose.yaml}`,
`docker/scripts/bench-shape.sh`, `docs/known-limitations.md`.

**Confirmed claims:** probes 340/341/342 fail today; ST-3, ST-4, ST-6, ST-12, ST-15, ST-16 fail
today; ST-11, ST-17, ST-24 pass today; every ST expected value is arithmetically right; the 37
keywords; the suffix composition (six shim files, `picasso96.h`, two separators); `-dM` and
`-aux-info` work for host `cc`, MinGW, and m68k GCC; the vbcc include tree exists; the measured
name counts are plausible (6,232 to 7,385 hosted-amiga macros); no `spec/tests` golden holds
emitted C; no spec-test pin changes because of class-3 escaping.

### Summary by Dimension

| # | Dimension | Findings | Highest Severity |
|---|-----------|----------|-----------------|
| 1 | Ambiguities | 3 (PF-014, PF-015, PF-021) | 🟡 |
| 2 | Implicit Assumptions | 2 (PF-003, PF-005) | 🟠 |
| 3 | Logical Contradictions | 1 (PF-009) | 🟡 |
| 4 | Completeness Gaps | 1 (PF-008) | 🟠 |
| 5 | Dependency Issues | 2 (PF-006, PF-017) | 🟠 |
| 6 | Feasibility Concerns | 1 (PF-019) | 🟡 |
| 7 | Testability | 3 (PF-013, PF-016, PF-018) | 🟡 |
| 8 | Security Blind Spots | 0 | — |
| 9 | Edge Cases | 2 (PF-001, PF-020) | 🟠 |
| 10 | Scope Creep Indicators | 0 | — |
| 11 | Ordering & Sequencing | 1 (PF-007) | 🟠 |
| 12 | Consistency | 2 (PF-010, PF-011) | 🟡 |
| 13 | Codebase Alignment | 3 (PF-002, PF-004, PF-012) | 🟠 |

### Summary by Severity

| Severity | Count | Status |
|----------|-------|--------|
| CRITICAL | 0 | — |
| MAJOR | 8 | all resolved (delegated, challenger concurred) and applied |
| MINOR | 13 | all resolved (delegated) and applied |
| OBSERVATION | 0 | — |

Every resolution below is `Authority: AI — delegated by --auto-design`, policy version 1, root
invocation `ad-pf-7b2e91c4`, eligibility class "compiler mechanism / testing strategy /
implementation sequencing inside the confirmed AR-1..AR-4, AR-10, AR-15 scope". None changes
product scope, a spec chapter, or a spec test; none accepts risk. Hardening for the eight majors:
one blind design-challenger received the batch without the picks and chose the same option for
all eight, adding amendments that are taken (noted per finding).

---

### PF-001: Enum constants as macros break the escape and rewrite unlisted names 🟠 MAJOR

**Dimension:** Edge Cases · **Location:** AR-16; 03-01 §Escape classes, §Source-name entry point; ST-9, ST-10
**Codebase Evidence:** `codegen/program.rs:999-1001` writes `#define {Enum}_{member} {value}`; the only use is `codegen/expr.rs:584-585`; spec/17 §3 (`spec/17-codegen.md:74`) asks only for "an emitted constant", and §7 (`:273`) says C macros are not used; the stdlib declares no enums.
**The Problem:** (a) a local `INT_MAX` (class 3) and the escaped constant of `enum INT { MAX }` both become `INT_MAX_0`, so the local declares `i32 1 = 7;`; patching with other suffixes still collides (`enum Color { Red, Red_0 }` against a local `Color_Red`). (b) The macros also rewrite names no file-scope list holds: suffix struct members and locals (`io_Command`, `font_size`), user symbols (`enum cl_Probe_Main { helper }`), and each other (`enum A_B { C }` and `enum A { X, B_C }` both define `A_B_C`; a release build prints `0 0` instead of `0 1`).

| Option | Description | Pros | Cons |
|---|---|---|---|
| A | Emit each use as its parenthesized folded value; remove the `#define` loop | removes the position; closes (a) and all of (b); matches spec/17 §7; one site | 7 impl pins change; emitted C loses constant names |
| B | Keep macros; distinct suffix; test against every token of prefix and suffix and the symbol sets | keeps names | complex; constant-vs-constant still a residual; keeps the §7 conflict |
| C | Keep macros; `#undef` all before the suffix, plus B for the prefix | protects the suffix | prefix, user-code, and constant-vs-constant collisions remain |

**Recommendation:** Option A.
**Resolution:** Authority: AI — delegated by --auto-design. Decision: A. Evidence as above. Rejected: B, C (reasons in the table). Strongest counterargument: readability of emitted C (cosmetic; macro names are not in debug info). Confidence: High. Hardening: challenger chose A independently. Reopen triggers: another site reads `<Enum>_<Member>`. Applied: AR-16 reopened (register §Preflight amendments); R1; 02; 03-01; ST-9 now `1 1 7 8` with locals `INT_MAX`, `MEMF_CHIP` and an exact-macro check; ST-10 adds `enum A_B` / `enum A` (`7 0 0 1`, default and release); task 2.2.4 with the seven pins (iteration 2 added `checker_conformance_impl_test.rs:1510`).

### PF-002: Generated owners share the locals table the plan escapes 🟠 MAJOR

**Dimension:** Codebase Alignment · **Location:** 03-01 §Source-name entry point; task 1.2.3; R4
**Codebase Evidence:** `codegen/function.rs:197-203,296-304` names every `function.locals` entry; generated owners `__cl_owned<N>` come from `declare_generated_local` (`own.rs:1689-1701`, sets `generated`), but `ir::Local` has only `name`, `ty` (`ir.rs:616-623`) and the flag is dropped at `own.rs:1250-1259` (also `own/lower.rs:1812-1815`, `ir.rs:776-780`); spec test `codegen_run_spec_test.rs:1384` pins `FreeString(__cl_owned0)`; a source local `__cl_owned0` is legal (`callable_release_integrity_spec_test.rs:425`).
**The Problem:** class 2 would rename the generated owner to `__cl_owned0_0`, breaking R4 and a spec test (an AR-25 stop).

| Option | Description | Pros | Cons |
|---|---|---|---|
| A | `generated: bool` on `ir::Local`, copied from the binding | exact; flag already exists | three construction sites |
| B | Exempt generated spellings by pattern | no IR change | lets a source `__cl_owned0` reach C unescaped |

**Recommendation:** Option A.
**Resolution:** Authority: AI — delegated. Decision: A. Rejected: B (unsound). Strongest counterargument: widens a struct in three places. Confidence: High. Hardening: challenger chose A, adding the lambda-body block and the second lowering path, both taken. Reopen triggers: a generated local created outside `declare_generated_local`. Applied: R4, 02, 03-01, task 1.2.3 (files `ir.rs`, `own.rs`, `own/lower.rs`), 07 impl row.

### PF-003: The not-in-table fallback spells `ffi` values and generated paths 🟠 MAJOR

**Dimension:** Implicit Assumptions · **Location:** 03-01 (fallbacks routed through the escape); AR-17; task 1.2.3
**Codebase Evidence:** `codegen/expr.rs:627-631` falls back to `sanitize_identifier(first)`; `resolve_function` (`codegen.rs:984-1020`) covers only user functions, so `let g: func(i32): i32 = abs;` for an `ffi` `abs` emits `g = abs;`; lowering builds `Path(["cl_hooktramp_<id>"])` (`own/call.rs:628`); every source local and parameter enters `local_names` first (`function.rs:190,202,296,304`).
**The Problem:** escaping the fallback emits `abs_0`, an undeclared name; NDK functions used as values break the same way.

| Option | Description | Pros | Cons |
|---|---|---|---|
| A | Escape only where a source name is given its C spelling; fallbacks keep `sanitize` | correct by construction | a future source local missing the table would leak raw |
| B | Escape in the fallback with special cases | — | a forgotten generated path breaks |

**Recommendation:** Option A, with an impl test that every source local and parameter is in the table.
**Resolution:** Authority: AI — delegated. Decision: A. Rejected: B. Strongest counterargument: as in the table, closed by the impl test. Confidence: High. Hardening: challenger chose A. Reopen triggers: a source name reaches a fallback. Applied: 03-01, AR-17 amendment, ST-25 (host `ffi` value, guard), 07 impl row.

### PF-004: The member-site list is incomplete and misplaces the definition 🟠 MAJOR

**Dimension:** Codebase Alignment · **Location:** 02 §Fields; 03-01; AR-18; tasks 2.2.1–2.2.2; 07 coverage goal
**Codebase Evidence:** unlisted member renderers `ValueKind::Member` (`expr.rs:66-70`, `.` and `->`) and `Projection::Field` (`expr.rs:205-210`); the struct definition is `program.rs:886-889` from names at `:954` (`:753` is the capture field); `offsetof` at `calls.rs:494,582` was in the tasks only.
**The Problem:** the executor could leave the `->` branches raw and untested; a list will drift again.

| Option | Description | Pros | Cons |
|---|---|---|---|
| A | Correct the list; add a raw-pointer read test | small | list stays the contract |
| B | Rule "every member spelling goes through one helper", a recorded grep audit, and keyword-named fields through every access form under strict builds | a leak is a C syntax error | a larger impl test |

**Recommendation:** Option B (the list kept as known sites).
**Resolution:** Authority: AI — delegated. Decision: B. Rejected: A (drifts). Strongest counterargument: keyword fields prove classes 1–2 per site; class 3 goes through the same call. Confidence: High. Hardening: challenger chose B and found further sites, all now named. Reopen triggers: a member site outside the helper. Applied: 02, 03-01, AR-18 amendment, ST-7 (`raw Gps` read, `3 21 2`), tasks 2.2.1–2.2.2, 2.3.1, 07.

### PF-005: Instance symbols can collide with each other 🟠 MAJOR

**Dimension:** Implicit Assumptions · **Location:** 03-02 Overview and step 1; AR-20
**Codebase Evidence:** instance symbols are `cl_<f>__<key>` (`program.rs:1019`); generic struct keys contain `__` (`types/keys.rs:98-105`); `get<T>` at `Holder<i32>` and `get__Holder<T>` at `i32` both emit `cl_get__Holder__i32` (probe).
**The Problem:** 03-02 claims instances never meet "each other", but the trigger covers only helper families and the shim set.

| Option | Description | Pros | Cons |
|---|---|---|---|
| A | Assign instance symbols once, in monomorphization order; a later instance meeting an earlier one takes `__user` | reuses the exact sets; one map for all four sites | discovery order decides the loser |
| B | Narrow the claim; residual limit | no code | leaves invalid C for an accepted program |

**Recommendation:** Option A.
**Resolution:** Authority: AI — delegated. Decision: A. Rejected: B. Strongest counterargument: an unrelated call can change which instance is renamed (emitted names are not ABI). Confidence: Med-High. Hardening: challenger chose A, amended to a precomputed map (taken). Reopen triggers: monomorphization order becomes nondeterministic. Applied: R5, 02, 03-02 steps 1, 3, 4, ST-26, task 3.2.1.

### PF-006: Phase 4 would change a spec-pinned profile API 🟠 MAJOR

**Dimension:** Dependency Issues · **Location:** 03-03 suffix pieces; task 4.2.1
**Codebase Evidence:** `amiga_optional_spec_test.rs:302-316` calls `Profile::HostedAmiga.shim_suffix()` as `&str`; also `stdlib.rs:1496`, `codegen.rs:581`, three impl suites.
**The Problem:** returning pieces from `shim_suffix()` breaks a spec test (an AR-25 stop).

| Option | Description | Pros | Cons |
|---|---|---|---|
| A | Keep `shim_suffix()` byte-identical; add a pieces accessor from the same constants; join-equality test | no spec impact | two accessors |

The only alternative, changing the signature and asking for a spec ruling, buys nothing and is dropped.
**Recommendation:** Option A.
**Resolution:** Authority: AI — delegated. Decision: A. Confidence: High. Hardening: challenger chose A. Reopen triggers: the join test fails. Applied: 01 Compatibility, 02, 03-03, tasks 4.2.1, 4.2.3.

### PF-007: The plan predates the known-limitations prime directive 🟠 MAJOR

**Dimension:** Ordering & Sequencing · **Location:** 99 per-task rules; task 5.3.2; 03-04 §Docs; AR-14
**Codebase Evidence:** AGENTS.md §Prime directive (commit `5c3316ea`) requires re-reading `docs/known-limitations.md` and `site/guide/known-limitations.md` after every change and editing in the same change; the plan commits per phase with all edits in Phase 5; `docs/known-limitations.md:217-218` (§11 hook and Box helper rows) become stale in Phase 3 and were not in any task.
**The Problem:** Phase 1–4 commits would leave stale limitations or claim "unchanged" falsely.

| Option | Description | Pros | Cons |
|---|---|---|---|
| A | A known-limitations step in every phase; each edit moved to the phase that causes it; §11 rows in Phase 3 | follows the directive | text revised several times |
| B | All edits in Phase 5 | fewer edits | violates the directive |

**Recommendation:** Option A.
**Resolution:** Authority: AI — delegated. Decision: A. Confidence: High. Hardening: challenger chose A. Reopen triggers: AGENTS.md changes the rule. Applied: R9, 03-02, 03-04 §Docs, AR-14 amendment, 99 rules and tasks 1.3.3, 2.3.2, 3.3.2, 4.3.2, 5.3.2 (44 tasks).

### PF-008: No method for typedefs, objects, and header enumerators 🟠 MAJOR

**Dimension:** Completeness Gaps · **Location:** 03-01 §The name list; AR-7, AR-19; tasks 1.2.1, 1.3.1
**Codebase Evidence:** `-dM` and `-aux-info` work on every GCC leg (probed); typedefs and objects only "from the preprocessed unit's file-scope declarations"; file-scope enumerators not named; ST-2 covers only `BYTE`, `APTR`, `ULONG`, `SysBase`.
**The Problem:** the completeness test is the only proof of R3 and had no defined extraction; header enumerators would be missed.

| Option | Description | Pros | Cons |
|---|---|---|---|
| A | Std-only depth-0 scan of `-E` output (typedefs, objects, `(*name)`, enumerators), shared by the one-off command and the test, with a recall check against `-aux-info` | R3 provable; safe over-collection | a hand-rolled scanner |
| B | Hand-maintained typedef/object list | no scanner | header updates go unnoticed |

**Recommendation:** Option A.
**Resolution:** Authority: AI — delegated. Decision: A; a scanner inside a test is a test, not generator tooling (no complexity escalation). Confidence: Med (the scanner's robustness on m68k `__asm` declarations; the recall check measures it). Hardening: challenger chose A with the recall check (taken). Reopen triggers: recall check fails. Applied: 03-01, AR-7/AR-19 amendment, tasks 1.2.1, 1.3.1, 07.

### PF-009: The uniqueness counter is not checked against the classes 🟡 MINOR

**Location:** 03-01 §Spelling; AR-18 · **Evidence:** `unique_local` (`codegen.rs:848-858`) appends `_<n>` without a class check; the hosted-amiga unit defines `COLORMAP_TYPE_V1_2`, `DAY_1..7`. Shadowed bindings are sanitized to `x_N` and marked first, so the counter only fires for `x_N_1`-shaped names; the hole is narrow.
**Resolution:** Delegated: the counter skips candidates in class 1–3 for source names; triple-shadow impl case. Confidence: High. Applied: 03-01, 07.

### PF-010: "Six positions" listed five 🟡 MINOR

**Location:** 01 R1, Scope Decisions; 03-01; 03-04 · **Evidence:** R1 lists locals, parameters, fields, captures, enum constants.
**Resolution:** Delegated: four name positions plus enum constants emitted as values (PF-001); the doc text no longer says "six". Applied: 01, 03-01, 03-04.

### PF-011: Zero helpers are per struct unit, not per moved type 🟡 MINOR

**Location:** 03-02 step 3; AR-20; task 3.3.1 · **Evidence:** `emit_struct_definitions` (`program.rs:883-905`) writes `cl_zero_<S>` for every struct unit.
**Resolution:** Delegated: exact set per emitted struct unit plus `cl_zero_ClStr`; the `Rec__user` case uses an unmoved struct. Applied: 02, 03-02, 99 3.2.2, 3.3.1.

### PF-012: The emitter's fixed prelude helpers are not reserved 🟡 MINOR

**Location:** 03-02 steps 2–3; R5 · **Evidence:** `program.rs:206-231` writes `cl_str`, `cl_string_view`, `cl_zero_ClStr`, outside the shim scan (`codegen.rs:576-597`); `module string; export func view` emits two `cl_string_view` (probe).
**Resolution:** Delegated: the three exact names join the reserved and exact sets; ST-27. Applied: R5, 02, 03-02, 07, 99 3.2.2.

### PF-013: ST-8, ST-13, ST-14, ST-16 as written fail for the wrong reason 🟡 MINOR

**Location:** 07 · **Evidence:** a stored closure is E10404; `import { Point }` beside `struct Point` is E11103; Box values read back need `boxWith`; lambda sites are numbered program-wide (`function.rs:314`).
**Resolution:** Delegated: ST-8 immediately invoked; ST-13 `import { Point as viaGet }` with `get<T>(v: T): T` called as `get<Point>(Point { x: 1 })` (input completed in iteration 2); ST-14 states `box.alloc__i32(1)` and `boxWith`; ST-16 split into (a) `7 8` and (b) a lambda-only fixture printing `9`, with the duplicate `cl_lambda_0__i32` recorded red. Applied: 07, 99 3.1.2.

### PF-014: "Top-level definition" undefined; the suffix has its own definitions 🟡 MINOR

**Location:** R7; 03-03; ST-18; 07 impl row · **Evidence:** the suffix files hold many function definitions with one `#line 1` each; hook support writes a struct, an object, and three functions (`program.rs:4614-4640`).
**Resolution:** Delegated: "top-level item" defined in 03-03 (function, file-scope object, or struct definition outside the suffix); group members repeat the line; the scan stops at the first `compiler/runtime/` directive. Applied: R7, 03-03, ST-18, 07.

### PF-015: ST-9's negative check matched the correct output 🟡 MINOR

**Location:** 07 ST-9 · **Evidence:** `#define MEMF_CHIP_0 1` contains `#define MEMF_CHIP`.
**Resolution:** Delegated: exact macro-name match (trailing space), and no enum macro at all after PF-001. Applied: ST-9.

### PF-016: vbcc builds inside `cargo test` had no rule 🟡 MINOR

**Location:** 07; 99 1.1.1 · **Evidence:** no Rust test runs `vc`; `bench-shape.sh:45-62` and `corpus-amiga.sh:71-87` retry vbcc's transient `No config file!`; NDK headers warn under vbcc.
**Resolution:** Delegated: skip without `vc`, one retry on that message, exit status only. Applied: 07, 99 1.1.1.

### PF-017: The rc24 completeness run had no runnable command 🟡 MINOR

**Location:** 99 1.3.3, 5.3.3, Verify lines; 03-01 · **Evidence:** the host has no `cargo`; the container mounts only the repository (`docker/compose.yaml:17-18`) and forwards only `PASSTHROUGH` names (`docker/Makefile:68-74`); the Verify lines omitted the rc24 runs that 5.3.3 and AR-12 require.
**Resolution:** Delegated: the root under `build/rc24/`, `PASSTHROUGH=CLORRAINE_RC24_ROOT`, the test target; fail (not skip) when set and unusable; stop for the user if the rc24 GCC cannot run in the image; Verify lines aligned. Unverified: whether the rc24 GCC runs in the image. Applied: 99, 03-01, 07.

### PF-018: ST-23's layout and argument order were unclear 🟡 MINOR

**Location:** 07 ST-23 · **Evidence:** the artifact folder takes the first source's stem (`artifacts.rs:149-153`).
**Resolution:** Delegated: imports, command line, and the C path spelled out. Applied: ST-21, ST-23.

### PF-019: Lambdas and hooks have no span to map 🟡 MINOR

**Location:** 03-03; AR-21; task 4.2.2 · **Evidence:** `ClosureSite` (`types/mod.rs:182-190`) and the hook call sites (`sema/mod.rs:187-209`) carry no span; `ir::Function` has none (`ir.rs:643-655`); `ModuleDecl.span` exists (`ast/mod.rs:35-40`).
**Resolution:** Delegated: new span fields recorded where sema creates the sites; files added to 4.2.2. Applied: 02, 03-03, 99 4.2.2.

### PF-020: The separator rule contradicted the backslash case on Linux 🟡 MINOR

**Location:** 03-04; 07; 99 5.3.1 · **Evidence:** `\` is a file-name byte on Linux; `platform.rs` keeps host rules as pure functions taking `HostOs`.
**Resolution:** Delegated: a pure `platform.rs` function; `\` is a separator only on Windows; both families tested on Linux. Applied: 03-04, 07, 99 5.2.1, 5.3.1.

### PF-021: The empty-name spelling was undefined after retiring `cl_` 🟡 MINOR

**Location:** 03-01; 07 unit-test row · **Evidence:** `sanitize_identifier("")` returns `cl_` (`codegen.rs:1230-1232`), pinned at `:1305-1308`; unreachable from source (LEX-5).
**Resolution:** Delegated: `""` → `_0`. Applied: 03-01, 07, 99 1.2.2.

---

## Iteration 2

> **Previous Iteration**: 21 findings — all resolved and applied
> **This Iteration**: 0 new root causes; 3 minor residuals of PF-001, PF-013, PF-003/011/014, fixed
> **Carried Forward**: none

A fresh auditor (general agent, preflight-auditor brief) verified every PF-001..PF-021 fix in
every document it touches, the counts (ST-1..ST-27; 44 tasks, 10/10/8/8/8), the red/green
ranges, every anchor, about 60 new file:line references against `5c3316ea`, and every new
expected value. Red-phase probes matched the plan: ST-25 passes today (`5`); ST-26
(`cl_get__Holder__i32`), ST-27 (`cl_string_view`), ST-10 (`A_B_C` redefined), ST-9 (`INT_MAX`
redefined), ST-16 (b) (`cl_lambda_0__i32`), and ST-7 fail today. `expr.rs:584-585` is the only
`<Enum>_<Member>` spelling site (switch labels, module and static initializers all go through
`path()`), and the parenthesized literal stays an integer constant expression.

Residuals of existing root causes (identities kept; no new root cause):

- **PF-001, iteration 2 evidence (🟡):** a seventh pin, `checker_conformance_impl_test.rs:1510`
  (`"e = E_B;"`), was missing from task 2.2.4. Fixed: 2.2.4 lists seven pins.
- **PF-013, iteration 2 evidence (🟡):** ST-13's input did not determine the expected `1`. Fixed:
  `get<T>(v: T): T` called as `get<Point>(Point { x: 1 })`, printing `x`, then `viaGet(2)`.
- **PF-003 / PF-011 / PF-014, iteration 2 evidence (🟡):** the original AR-17, AR-20, and AR-21
  notes still contradicted the amended design. Fixed: those notes are marked partly superseded,
  and §Preflight amendments states that it and the 03-* documents win.

All three are applied and checked directly; they change no behaviour, ownership, ordering, or
contract, so no iteration 3 is needed (report-format §Convergence).

## Adversarial checklist

- *Assumption confirmed rather than tested:* that emitting enum values inline changes no observable
  behaviour. The macro expands to the same token text, so the C type and constant-expression
  status are the same; the parentheses only guard a negative value. ST-10's release leg tests it.
- *External standard:* C99 §7.1.3 reserved identifiers (class 2) and §6.4.1 keywords are quoted
  from the plan's list, not re-read from the standard text; the 37-keyword list matches C99 §6.4.1
  as commonly reproduced. Flag for human review if exact conformance matters.
- *A disagreeing expert:* would object that a token scanner over preprocessed headers is fragile.
  The recall check and the over-collection rule make its failure visible instead of silent.

## Scope

Strict scope. No optional additions were reported or applied. Pre-existing defects outside the
goal stay in AR-4. With PF-001, enum constant collisions are fixed as a side effect of removing
the macros; no new scope was added.
