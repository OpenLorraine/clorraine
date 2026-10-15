# Preflight Report: Residual Major Closeout

> **Status**: ✅ PREFLIGHT PASSED WITH NOTES — 0 critical, 0 major; 5 minor findings resolved in
> place by preflight corrections (auto-design delegated); 1 observation accepted
> **Iteration**: 1 (first scan; single-pass full 13-dimension audit)
> **Artifact**: Implementation plan at `plans/residual-major-closeout/` — 13 documents
> (00-index, 00-ambiguity-register, 01-requirements, 02-current-state, 03-01…03-06,
> 05-findings, 07-testing-strategy, 99-execution-plan)
> **Content hashes**: the audited (pre-correction) files were untracked; the post-correction
> state recorded by this report is:
>
> ```
> f568de9c65586941e72005a932a2b24c2c739b9cd9bd8bbc920ab402e1b59729  00-ambiguity-register.md
> bfd8333a5d003eb98b12dfe8a0790e601acb91a8bce6e92a7d817310cbcee217  00-index.md
> 23933a150c1cfb0045b8bffefbf28efa420bf27ed76e54ebed95e833a0e38384  01-requirements.md
> 29dc741b74e5c300acde644a0126e663b8ce2d9c4effd8caa2b8b01a46eb71da  02-current-state.md
> 98e56aa0f9d54a5a154be28d9dcc04c415ea1bca7242c835332e20ce3cdca026  03-01-init-call-guards-view-conditionals.md
> ce2ae736bf13a641bb2ca45e349a174e951eee781bdf7139c1819861d0c89090  03-02-element-placeholder-inference.md
> 7e7eaa832d544cde79ebd8a3c567a54bca4d99a85da42d625ea6fad1484880dc  03-03-member-typing.md
> c158c42cac388a7c0ceee029fad7e843fd7548f672164b43b21771cc56e94844  03-04-call-conversions-ffi-typing.md
> 212f1b36cb841d75b753875546cf8a26ce59be3d8a175695468e7a08bc355c6a  03-05-null-destinations-capture-stores.md
> b776a71fc169adb30202f18f99294d4b6d76d39cf995566ecbcf948431c03104  03-06-standalone-emission-shapes.md
> 5f5c9163057744cdea05d38f015bf5b3995b6e16bc60c521e11ea37ca214c7c9  05-findings.md
> 5351756e63f12a31f8585b3a96e965f3de7c462ae73da22448b3581fa9475444  07-testing-strategy.md
> d199cfdc50e8f4d30d3844db6d66796e226c2a5201427234622eeb8659fe74a6  99-execution-plan.md
> ```
>
> (the audited draft differed only by the five corrections listed below).
> **Codebase Grounded**: 30+ source/test files examined; 44 cited `file:line` sites verified;
> 20/20 defect rows re-probed on the pinned container (GCC 12.2, ASan); `make check` exit 0 at
> head `80433a6`
> **Reviewer**: separate `preflight-auditor` dispatch; the plan was authored by the parent
> planning session (same agent model — the probe re-verification below is the compensating
> control; every claim is grounded in a re-run probe or a cited code site)
> **Last Updated**: 2026-10-16

### Codebase Context Summary

**Tech Stack:** Rust std-only workspace; `clc` (compiler) and `clorraine` (driver); C99 emission;
pinned Docker SDK (GCC 12.2, `m68k-amigaos-gcc`, AddressSanitizer); `make check` / `make verify`.

**Architecture:** staged pipeline `lex → parse → load → resolve → sema → own → ir → codegen`; a
monomorphization walk (`own/generics.rs::collect`); per-instance duties in
`own/instance_check.rs`; a second, diminished initializer lowering in `codegen/program.rs`;
per-profile C shims in `compiler/runtime/`.

**Key files examined:** `compiler/src/codegen/program.rs`, `codegen/{expr,calls,function,guards,
types,cleanup}.rs`, `compiler/src/sema/{expr,call,stmt,mod}.rs`, `compiler/src/own/{generics,
call,call_check,instance_check,expr,loops,lower}.rs`, `compiler/src/{codegen.rs,types/*}`,
`compiler/runtime/shim_freestanding.c`, the five origin suites, `spec/{04,06,09,11,14,17}.md`,
`plans/00-roadmap.md`, the origin ledgers.

**Key observations:**

- Every cited `file:line` exists in the head tree and matches the described code (accuracy ±3
  lines; the plan correctly says the executor re-resolves them).
- All 20 rows reproduce AR-3 exactly at head `80433a6`, including the `DEF-209` green control
  (build + run + ASan) and the `DEF-229` partial (direct-read control green, `cl_id__unknown`
  red). The `make check` baseline (66 suite results + `spec/check.sh`, 110 compile-fail tests) is
  green, so the regression-oracle claim holds.
- Every diagnostic code the plan uses exists in `spec/14-diagnostics.md` (`E10012`, `E10034`,
  `E10035`, `E10200`, `E10212`, `E10216`, `E10401`, `E11211`, `E11213`), and AR-6/AR-7/AR-8/AR-13
  are conformance readings of the frozen text (Ch 04 §8 `Type.MEMBER`, Ch 04 §11 / Ch 11 §7
  `sizeof`, Ch 06 L-3, Ch 17 §3 `T?`); no spec or registry edit is needed.
- `RDC_ASAN` is supported by the P1–P5 origin suites; the hosted-amiga and freestanding legs the
  plan relies on exist in the container and the suites.

### Probe Evidence — AR-3 re-verification (2026-10-16, head `80433a6`)

Method: `docker compose run … bash` on the pinned image; `target/debug/clc --profile host`
(`freestanding` for `DEF-189`, `hosted-amiga` + `m68k-amigaos-gcc -mcrt=nix20` for the `File?`
spellings); strict `cc -std=c99 -O0 -g -Wall -Wextra -Werror`; ASan
`-fsanitize=address -no-pie` with `ASAN_OPTIONS=detect_leaks=1`. Scratch under
`/tmp/opencode/batch-k-probes/` (removed below).

| Row | AR-3 says | Re-probe result | Match |
|---|---|---|---|
| `DEF-170` | strict build fails, bare written name | `implicit declaration of function 'g'` | ✅ |
| `DEF-189` | host no-libc segfault 139, m68k link clean | run 139 (local `str[2]` shape); m68k link rc 0; host build clean | ✅ |
| `DEF-196` | accepted, run 255, stores `h = (5);` | run 255; emitted `h = (5);` for `mk()(5)` and the conditional callee | ✅ |
| `DEF-197` | strict fails, no instance | `implicit declaration … cl_probe_def197_pick` | ✅ |
| `DEF-207` | accepted, run segfault 139 | run 139; emitted `cl_string_view((c ? stringOf("abc") : NULL))` | ✅ |
| `DEF-209` | fixed: build/run 0, ASan-clean | run 0 + ASan clean (let, return, argument, module-init spellings) | ✅ |
| `DEF-211` | accepted, run exit 252 | run 252 (`sizeof(i32)` = 4) | ✅ |
| `DEF-219` | strict fails `'__cl_itmp1' undeclared` | same; declaration inside the `if`, release outside | ✅ |
| `DEF-220` | strict fails, implied `viewOf` instance | `implicit declaration … cl_probe_def220_viewOf` + invalid initializer | ✅ |
| `DEF-229` | `cl_id__unknown`; direct read green | same; direct-read control run 0 | ✅ |
| `DEF-230` | strict fails, env field `i32 v` for `ClS` | `incompatible types when initializing type 'int' using type 'ClS'` | ✅ |
| `DEF-231` | `cl_inner__unknown` / `cl_each__unknownQ` | both implicit declarations | ✅ |
| `DEF-232` | no guard, runs to completion; concrete twin `E10216` | run 1; no `cl_trap` on `row[5]`; concrete `row[5]` → `E10216` | ✅ |
| `DEF-233` | accepted; member error; both paths | concrete and `read<E>` instance both fail `request for member 'A'` | ✅ |
| `DEF-238` | accepted; `int-conversion`; `.len` green | same; `.len` control run 0 | ✅ |
| `DEF-242` | accepted; `ClStr` where `char *` declared | same (ffi argument spelling) | ✅ |
| `DEF-243` | accepted; pointer-from-integer | `passing argument 1 of 'Uses' makes pointer from integer` | ✅ |
| `DEF-245` | accepted; host `void *` into view, hosted-amiga `BPTR` | same; **plus**: the `File?` return is red on hosted-amiga (`returning 'void *' … 'BPTR'`; see PF-003) | ✅ |
| `DEF-251` | accepted; abort 134; ASan double-free | run 134; ASan `attempting double-free` | ✅ |
| `DEF-255` | accepted; `'return' with a value` | same; block-bodied control aborts `E11213`; `return panic(...)` → `E10217` | ✅ |

Controls re-measured green: `pick<i32>(1, 2)` run 0; direct element read run 0; concrete `E10216`;
`.len` twin run 0; concrete null argument → `E10012`; `DEF-209` ASan run clean.

### Summary by Dimension

| # | Dimension | Findings | Highest |
|---|---|---|---|
| 1 | Ambiguities | 1 (PF-004) | 🟡 |
| 2 | Implicit Assumptions | 0 | — |
| 3 | Logical Contradictions | 0 | — |
| 4 | Completeness Gaps | 1 (PF-003) | 🟡 |
| 5 | Dependency Issues | 0 | — |
| 6 | Feasibility Concerns | 1 (PF-004) | 🟡 |
| 7 | Testability | 2 (PF-003, PF-005) | 🟡 |
| 8 | Security Blind Spots | 0 (N/A — compiler input validation unchanged) | — |
| 9 | Edge Cases | 0 | — |
| 10 | Scope Creep Indicators | 0 (AR-14 holds: every fix rides an existing hook; no new dependency/machinery) | — |
| 11 | Ordering & Sequencing | 0 (6 phases, 39 tasks, spec-first per phase) | — |
| 12 | Consistency | 2 (PF-001, PF-006) | 🟡 |
| 13 | Codebase Alignment | 1 (PF-002) + full reality check | 🟡 |

### Summary by Severity

| Severity | Count | Status |
|---|---|---|
| CRITICAL | 0 | — |
| MAJOR | 0 | — |
| MINOR | 5 (PF-001…PF-005) | all resolved in place (corrections below) |
| OBSERVATION | 1 (PF-006) | accepted, recorded |

---

## Findings

### PF-001: Ambiguity Register numbers collide between the decision table and the Resolution Notes 🟡 MINOR

**Dimension:** 12 Consistency
**Location:** `00-ambiguity-register.md`, decision table rows 9–13 vs Resolution Notes AR-9–AR-13
**Codebase Evidence:** not code-dependent; every cross-document citation resolves against the
Notes — e.g. `01-requirements.md` "the stop condition in AR-13 applies" (DEF-245) and
`00-index.md` "Test placement … AR-9", "Lenses … AR-10".
**The Problem:** the table's row 9 was `DEF-245`, row 12 `Naming`, row 13 `Lenses`; the Notes'
AR-9 is test placement, AR-12 is "no spec edit", AR-13 is the `DEF-245` stop. Rows 1–8 and 14–15
coincided, so the collision was a lookup trap.

**Options:**
| Option | Description | Pros | Cons |
|---|---|---|---|
| A | Align the table to the Notes: renumber, move rows 9–13 into AR order, add the two decisions that existed only in the Notes | Every citation in the plan set stays valid; one mapping | A table edit |
| B | Renumber the Notes to the table and update every citation | Table layout untouched | ~20 edits across 8 files; error-prone |

**Recommendation:** Option A — all citations use the Notes' numbers, so only the table needs the
edit.

**Resolution:** ✅ **Resolved — Option A applied (auto-design delegated).** Rows renumbered 9–13
(Naming, Lenses, `DEF-189` legs, no-spec-edit, `DEF-245`); the two table-only decisions moved to
rows 17–18; rows 14–16 unchanged. No other document changed.

### PF-002: "The 20 remaining open majors" reads as exhaustive — 26 open majors exist 🟡 MINOR

**Dimension:** 13 Codebase Alignment (Scope vs. Reality)
**Location:** `00-index.md` (Feature line and Overview), `01-requirements.md` (Feature Overview)
**Codebase Evidence:** `plans/00-roadmap.md` — of the 26 rows with `Backlog | ⬜` and 🟠
severity, 20 are in scope; the other six are the two ruling-parked majors (`DEF-167`,
`DEF-171`) and four batch-J follow-on majors (`DEF-315`, `DEF-322`, `DEF-323`, `DEF-329`,
roadmap lines 333/340/341/347).
**The Problem:** "the 20 remaining open 🟠 major defect rows that remain after the batch-I/J
work" implies these are all the remaining majors. The exclusions are stated later (AR-1/AR-2),
so the working scope was never in doubt — but the headline fact is wrong as written.

**Options:**
| Option | Description | Pros | Cons |
|---|---|---|---|
| A | Qualify the phrases: "the 20 rows named by AR-1" and name the excluded majors count | Factually exact; scope unchanged | Three small sentences |
| B | Leave as-is | No edit | A false completeness claim |

**Recommendation:** Option A — a one-word-class fix; no scope change.

**Resolution:** ✅ **Resolved — Option A applied (auto-design delegated).** The three sentences
now say the rows are "named by AR-1" and note the four batch-J follow-on majors plus the two
parked majors stay out (AR-2).

### PF-003: ST-15 and the P5 spec list omit the nullable-handle return position that R5 requires 🟡 MINOR

**Dimension:** 4 Completeness Gaps / 7 Testability
**Location:** `07-testing-strategy.md` ST-15; `03-05-null-destinations-capture-stores.md`
Testing Requirements; contrast `01-requirements.md` R5 ("store, argument, return")
**Codebase Evidence:** probe `def245retfile` (hosted-amiga): `func f(): File? { let x = null; return x; }`
fails exactly as the store does — `error: returning 'void *' from a function with return type
'BPTR' {aka 'long int'}` (host build passes; the view twin reports `E10035`, probe `def245ret`,
so only the handle return is open).
**The Problem:** R5 and the 03-05 design cover "wherever the destination type is known (the
store/argument/return positions)", but the probe table, 02-current-state's gap row, and ST-15
name only the store and the argument. An executor working from ST-15 could ship the fix without
covering the handle return, leaving the requirement's return clause unverified.

**Options:**
| Option | Description | Pros | Cons |
|---|---|---|---|
| A | Add the `File?` return to ST-15 and the P5 spec list (hosted-amiga build) | Closes the requirement; one case | One more build leg in the case |
| B | Drop "return" from R5 | Smaller test | Contradicts Ch 17's lowering rule and leaves a known red |

**Recommendation:** Option A — the position is already in R5 and the design; only the test/docs
lagged.

**Resolution:** ✅ **Resolved — Option A applied (auto-design delegated).** ST-15 now reads
"the `File?` store and return on hosted-amiga" with the red evidence, and 03-05's Testing
Requirements covers "the `File?` store and return".

### PF-004: `DEF-229`'s fix direction names only the element mapping; the member-read argument's key mechanism is unstated 🟡 MINOR

**Dimension:** 6 Feasibility Concerns / 1 Ambiguities
**Location:** `03-02-element-placeholder-inference.md` §1 (last sentence) and its Testing
Requirements impl item
**Codebase Evidence:** `sema/expr.rs:585-597` — a member read on a `Param` base records an
opaque projection and returns `Ty::unknown()`; `own/expr.rs:20-26` — the ownership stage reads
sema-checked types; `own/call.rs:386-399` — inferred keys come from the argument `Value.ty`;
`codegen.rs:743-753` — the substitution maps only the element placeholder
(`#element of T → arg.iterated_element()`). So `inner(v)` (the argument *is* the placeholder)
keys correctly through the mapping, while `id(v.y)` (a member-read argument) still infers
`unknown` — the very key the probe shows (`cl_id__unknown`). The old impl note's example key
`cl_id__arr2_S` is also inconsistent (that is the `S[2]` key; the member read's substituted
field type — `i32` here — drives the key).
**The Problem:** as written, §1's mechanism covers `DEF-231` (`inner(v)`) but not `DEF-229`
(`id(v.y)`); an executor implementing only the stated mapping leaves ST-7 red and burns a
diagnosis cycle. The phase's red → green loop still reaches the acceptance criteria, so this is
a design-text gap, not a blocked fix.

**Options:**
| Option | Description | Pros | Cons |
|---|---|---|---|
| A | State the requirement: an unresolved member-read argument must resolve (its substituted field type, not the element) before the key derives; drop the wrong example key | Executor knows the bar; acceptance unchanged | The exact mechanism stays the implementer's choice |
| B | Rewrite §1 with a prescribed implementation | Certain | Over-specifies beyond the plan's own evidence |

**Recommendation:** Option A — keep the plan's acceptance (ST-7: keyed instance emits, run
proves the values) and let the phase pin the exact key.

**Resolution:** ✅ **Resolved — Option A applied (auto-design delegated).** 03-02 §1 now adds
the member-read resolution requirement and says the phase pins the exact key; the impl
requirement says "the member read's substituted type key". The P2 red probe re-measured:
`id(v.y)` → `cl_id__unknown`, direct read green — the executor should reproduce it first.

### PF-005: Several ST expectations rely on run exit 0 without reading the value back 🟡 MINOR

**Dimension:** 7 Testability
**Location:** `07-testing-strategy.md` ST-2, ST-4, ST-6, ST-9, ST-15, ST-19
**Codebase Evidence:** the batch-J-era trap class: a wrong value can exit 0 (the probes encode
values in the exit code — `def211` red is exit 252, `def197` control exits 0 only with
`g == 1`); the memory defects are covered by `RDC_ASAN` suites, but silent-value defects are
not.
**The Problem:** "run exit 0" alone does not prove the fixed value; a case author following the
row literally could write a `main` that always exits 0, so the case would not fail on a
silently wrong result (the `DEF-196` class is exactly a silent wrong value).

**Options:**
| Option | Description | Pros | Cons |
|---|---|---|---|
| A | State the read-back in each affected ST row (`g`/`e`/the projection/the null test/the `sizeof` values) | The case fails on a wrong value | Wording edits |
| B | Leave to the author | Smaller diff | Repeats the batch-J trap |

**Recommendation:** Option A — the probes already compute the values; the case just has to
assert them.

**Resolution:** ✅ **Resolved — Option A applied (auto-design delegated).** ST-2 (`g`),
ST-4 (`e` = 2), ST-6 (projection), ST-9 (the call's result), ST-15 (`s == null` and
`len(s) == 0`), ST-19 (the 8/24 values) now state the read-back.

### PF-006: The `E10212` message for a value-form enum read is imprecise 🔵 OBSERVATION

**Dimension:** 12 Consistency
**Location:** `03-03-member-typing.md` §1 and AR-6
**Codebase Evidence:** `spec/14-diagnostics.md:98` — E10212 message `'{name}' is not a member of
'{type}'`, help "check the declaration or the spelling"; the value-form read at `e.A` names an
existing member, so the rendered message ("'A' is not a member of 'E'") is literally false even
though the rejection is right (Ch 04 §8's member table has no value form; the "zero matches is
a member error" rule is E10212).
**The Problem:** wording only; no frozen code fits better and a new one is out of scope (AR-12).
**Recommendation:** keep the pinned E10212 report and add nothing to this plan; record the
wording imprecision for a future registry-clarification batch.
**Resolution:** 🔵 **Accepted** (recorded; no plan change — the fix direction is the only one
inside the frozen codes).

---

## Plan Corrections Applied (recorded)

| # | Finding | File | Change |
|---|---------|------|--------|
| 1 | PF-001 | `00-ambiguity-register.md` | Table renumbered/reordered to match the Resolution Notes: rows 9–13 = Naming, Lenses, `DEF-189` legs (new row), no-spec-edit (new row), `DEF-245` (moved); table-only decisions moved to rows 17–18; rows 14–16 unchanged |
| 2 | PF-002 | `00-index.md` (×2), `01-requirements.md` | "the 20 remaining open majors" → "the 20 … named by AR-1", with the four batch-J follow-on majors and two parked majors named as excluded (AR-2) |
| 3 | PF-003 | `07-testing-strategy.md` (ST-15), `03-05-…md` | ST-15 input/expectation gains the `File?` return and the null read-back; P5 Testing Requirements gains the return |
| 4 | PF-004 | `03-02-…md` | §1 states the member-read resolution requirement and drops the wrong `cl_id__arr2_S` example; the impl requirement states the substituted-type key |
| 5 | PF-005 | `07-testing-strategy.md` (ST-2/4/6/9/15/19) | Expected-output cells now state the read-back values |

No other file was modified; no source, test, roadmap, or origin-plan file was touched. The
corrections are documentation-only and change no scope, requirement, or acceptance criterion.

## Notes for Execution (not findings)

- **Stale context comment:** `compiler/tests/checker_conformance_spec_test.rs:926-930` (the
  `memenum` case) documents "a declared member stays accepted" for the instance path. The
  `DEF-233` fix deliberately reverses that acceptance; the test asserts only the undeclared
  member, so it stays green, but P3 must update the comment to avoid a misleading pin. The
  plan's risk table already anticipates measuring the existing pins.
- **`DEF-229` impl key:** ST-7 should pin the key the phase settles (a `cl_id__i32`-family
  symbol for the probed shape) rather than the plan's old example.
- **`DEF-170` probe:** the AR-3 probe uses `lib` while the working probe pair needs the module
  path (`from probe.lib`) and `export` on the globals; the case must ship both files.
- **`DEF-242` probe:** a function literally named `Open` collides with the host shim's
  prototype (`conflicting types for 'Open'`); the case should use a unique name (the plan's
  `call<str>` shape with any contracted ffi function is equivalent).
- **`DEF-189` shape:** the segfault reproduces with the `str[2]` literal **inside `main`**; the
  module-level spelling (global array) builds and runs clean, so the case must use the local
  shape the plan records.
- **Roadmap sync:** per the preflight process, a passing audit advances the plan's roadmap row
  to `Plan Preflighted` (🔬) — not performed here (the dispatch forbids roadmap edits); the
  parent owns that update.

## Not Verified / Limits

- The m68k and hosted-amiga legs were **built** (link/build rc), not run under an emulator
  (FS-UAE is outside this plan's scope).
- The exact post-fix key symbol for `DEF-229` and the exact post-fix emission spellings are
  implementation-time facts; the audited expectations are phrased at the acceptance level.
- `DEF-234` (the `str[2]`/`len` freestanding sibling) stays out of scope by design (minor row);
  its root is `DEF-189`'s, noted in `03-06` — no action required here.
- The plan's `DEF-234` note in the AR-3 probe method ("plus `--profile freestanding` for
  `DEF-189`/`DEF-234`") records a probe convenience only; `DEF-234` is not an in-scope row.

## Verdict

**✅ PREFLIGHT PASSED WITH NOTES** — no critical or major findings; five minor findings resolved
by in-place documentation corrections (recorded above); one observation accepted. The 20-row
status table, all cited code sites, all frozen-spec readings, the regression baseline, and the
testing strategy's red/green claims are grounded in re-run probes at head `80433a6`. Ready for
execution after the parent reviews the corrections.
