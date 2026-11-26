# Verification packet 9.2.1 — WS0, WS1, WS6

## Scope

- Whole components at the baseline `e24a8752b08827c3a5d7c77b6e2827dac708b629`; pre-existing defects are in scope.
- Finding IDs use the `ASM-WS<n>-<NNN>` scheme.
- Read-only: do not edit any tracked file; write scratch output only under `build/tmp/assessment/verify/`.
- Before each reproduction run the drift check: `bash plans/independent-assessment/probes/WS0/drift-check.sh plans/independent-assessment e24a8752b08827c3a5d7c77b6e2827dac708b629` (it must pass). Builds and runs go through the SDK container: `make -C docker exec CMD="…"` (no nested quotes in CMD). The pinned compiler is `build/tmp/assessment/baseline/clc` (release; `clc-debug` alongside).
- Judge each finding from its text, the committed probes, the code, and `spec/`. Do not rely on the workstream files' other sections; they are the author's reasoning.

## Checklist briefs

- `.opencode/agents/semantics-reviewer.md`
- `.opencode/agents/correctness-reviewer.md`

## Protocol

For every finding, the verifier:

1. Re-runs the reproduction or measurement from the committed sources with the drift check
   passing; for every `probable` Major or Critical it attempts a reproduction.
2. Tries to refute it: checks `spec/`, `docs/known-limitations.md`, earlier plans, and
   whether the observed behavior is intended.
3. Checks the severity against §Severity and the confidence against §Confidence.
4. Checks for duplicates across workstreams and merges them under the lowest ID.
5. Records one verdict: `confirmed`, `refuted`, `downgraded <from→to>`, `upgraded <from→to>`,
   or `merged into <ID>`, with a one-line reason.
6. Escalates any verdict that results in a Critical, per §Escalation.

The verifier also re-checks at least two `clean` outcomes per workstream, preferring items on
the A500 lens and memory-safety checklist items (WS2, WS5-03, WS6-05), and records a verdict
for each. Findings raised by a verifier are
added to the workstream file and receive their verdict from a different verification session.
Refuted findings stay in `05-findings.md` with their verdict; they never reach the report's
ranked list.

## Severity

| Severity | Definition | Ref |
|----------|------------|-----|
| 🔴 Critical | An accepted program miscompiles, is memory-unsafe, or emits invalid C; a crash or hang on a real Kickstart; data loss | AR #15 |
| 🟠 Major | Latent undefined behavior in emitted C; a shim leak or missing failure path; a wrongly rejected valid program; ≥10% net m68k instructions or `.text` against the equivalent C reference on a shipped 68000 build (§Measurement rules) | AR #15, PF-009 |
| 🟡 Minor | Edge-case divergence with low impact; maintainability debt that has already caused a recorded DEF | AR #15, PF-020 |
| 🟢 Opportunity | A measured improvement with no defect behind it below the Major threshold, or any gap measured only on a 68020 probe build | AR #15, PF-009 |

A `hypothesis` is never above Minor. A `clc` crash or hang on any input is Critical; a compile
time or peak RSS above 10× the Phase 0 `bench-compile` value for the same input size is Major
(PF-020). For emitted C (WS3-11), a C-compiler crash on valid emitted C, or a C compile time
above 10× the Phase 0 C compile time of the nearest-size emitted program, is Major (PB-005).

### Confidence

`verified` is only this confidence label; it never means "passed Phase 9" (PF-002).

| Label | Meaning |
|-------|---------|
| `verified` | Reproduced from committed probe sources at the baseline (drift check passing), or measured |
| `probable` | Strong code-reading evidence, no reproduction yet |
| `hypothesis` | Plausible from reading; never ranked above Minor (AR #15) |

A finding is **surviving** when its Phase 9 verdict is `confirmed`, `downgraded`, or
`upgraded`. Only surviving findings reach the report and the DEF ledger (AR #13 as clarified
by PF-002).



## Output

Write `build/tmp/assessment/verify/9.2.1.md` and return its content: one row per finding `| ID | verdict | reason (one line) | what you re-ran and observed |`, one row per sampled clean outcome with a verdict (`confirmed` or `refuted` + reason), any duplicate you would merge, and any new finding you raise in the finding template (it will be verified by a different session).

## Findings to verify

### ASM-WS0-001 — The flag audit's host binary floor fails on `lookup` at the baseline

- **Severity**: Minor
- **Confidence**: verified
- **Lens**: host
- **Location**: `tools/perf/flag_audit.py:444-446` at e24a8752
- **Checklist item**: WS0
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Measurement — `python3 tools/perf/flag_audit.py run --clc build/tmp/assessment/baseline/clc --check --out build/perf/assessment-baseline-flags.json` (container, 2026-11-25; record `records/assessment-baseline-flags.json`, `checks[0]`). Expected (documented harness invariant: `make bench-flags` "assert[s] the size/runtime floors", AGENTS.md §Commands; `check_floors` requires a scored host binary ≤ 85% of the old-flag binary): every row passes. Observed: `host/lookup: host binary 30912 > 34880 * 85%` (88.6%); exit status 1. The earlier `build/perf/flags.json` (2026-11-15, `clc 1.1.2`) records the same failure with the same byte counts, so it predates this release.
- **Impact**: `make bench-flags` exits non-zero on a clean tree, so the size/runtime floor gate is red for every run and cannot signal a real regression on the other rows; no shipped program is affected.
- **Root cause**: the 85% host-binary floor is uniform, but `lookup` (added in `a886a345`, 2026-11-09) is dominated by its 16 KB static table, which section GC and stripping cannot shrink; its `.text` falls 19.5% while its file falls only 11.4%.
- **Recommendation**: give data-dominated benchmarks a floor on `.text` instead of the file size, or exclude the table bytes from the host-binary floor; alternatively record `lookup` as exempt from the host floor with its reason.
- **Effort**: S
- **Related**: `plans/constant-array-initializer` (added `lookup`); WS8-05/WS8-06 (gate integrity)

### ASM-WS1-001 — An inferred-type decimal literal above `i32` range is accepted and silently wraps

- **Severity**: Major
- **Confidence**: verified
- **Lens**: all
- **Location**: `compiler/src/sema/expr.rs:690-707` at e24a8752
- **Checklist item**: WS1-01
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Reproduction — probe `probes/WS1/002-default-i32-literal.clo` (`let a = 2147483648;`), command `make -C docker exec CMD="python3 plans/independent-assessment/probes/WS1/probe.py plans/independent-assessment/probes/WS1 --clc build/tmp/assessment/baseline/clc --only 002"`. Expected (spec/01-lexical.md §7 LEX-14/LEX-15, spec/02-types.md §9 TY-15): reject with E10202, because a decimal literal defaults to `i32` and 2147483648 does not fit. Observed: accepted on host, hosted-amiga, hosted-amiga-13 (`rc=0`); the emitted C is `i32 a = 0; … a = 2147483648u;`. A follow-up program (`let a = 2147483648; let c = 4294967296;`, `build/tmp/assessment/WS1/diag/lit2.clo`) printed `-2147483648 0` on host, and host GCC reports `-Woverflow` for `c = 4294967296u`, so the emitted C fails the strict debug line (`-Werror`). Every annotated or operator context rejects correctly: `let a: i32 = 2147483648;`, `a = 2147483648;`, `1 + 2147483648`, and `let a: u32 = 4294967296;` are all E10202; a literal above `u64` is E20004.
- **Impact**: a program with an un-annotated large decimal constant (for example a 32-bit mask or address written in decimal) compiles and runs with a different value, or emits C that the strict build rejects; the spec requires a compile error.
- **Root cause**: `check_int` performs the fit check only when an expected integer type is present; with no context it returns `i32` without checking that the value fits, and codegen spells the original literal with a `u` suffix.
- **Recommendation**: in `check_int`, when there is no expected type, check the value against `i32` and report E10202 (spec LEX-14/LEX-15); add a compile-fail golden for `let a = 2147483648;`.
- **Effort**: S
- **Related**: WS1-04 (literal typing); WS3-01 (strict build)

### ASM-WS1-002 — Call arguments with side effects are evaluated right to left

- **Severity**: Critical
- **Confidence**: verified
- **Lens**: all
- **Location**: `compiler/src/own/call.rs:409-428`, `compiler/src/own/call.rs:565-575`, `compiler/src/own/lower.rs:7286-7316` at e24a8752
- **Checklist item**: WS3-03 (found by the WS1-02 probes 021 and 066)
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Reproduction — probes `probes/WS1/021-evaluation-order.clo` and `066-evaluation-order-shapes.clo`, command `make -C docker exec CMD="python3 plans/independent-assessment/probes/WS1/probe.py plans/independent-assessment/probes/WS1 --clc build/tmp/assessment/baseline/clc --only 021 066"`. Expected (spec/04-expressions.md §3: "Function call — callee expression, then arguments left to right"; "Evaluation is strictly left-to-right and deterministic"): `pair(next(), next())` prints `1 2`; `three(next(), one(next()), next())` prints `123`. Observed on host, hosted-amiga, and hosted-amiga-13, debug and release builds: `2 1` and `321`. The emitted C is `cl_Probe_Main_pair(cl_Probe_Main_next(), cl_Probe_Main_next());`, whose argument order C leaves unspecified (C99 6.5.2.2p10); GCC evaluates right to left on both targets. Struct-literal fields, array-literal elements, template pieces, operator chains, and assignment target-then-value printed the spec order. The binary operator `next() - next()` printed `-1` (left first), but it is emitted as `(u32)cl_Probe_Main_next() - (u32)cl_Probe_Main_next()`, whose operand order is also unspecified in C (C99 6.5p3), so its correct result rests on GCC's current choice.
- **Impact**: any call whose arguments include two or more calls with side effects (I/O reads, allocators, counters, iterators, hardware reads) runs them in the opposite order to the source on every shipped profile; the result is a silent miscompile. Example: `copy(readByte(f), readByte(f))` swaps the two bytes.
- **Root cause**: argument lowering pins an argument into a statement temporary only when an argument contains a syntactic update (`++`, `--`, assignment; `contains_update`) or when a later argument needs a statement fill; an argument that is a call with side effects but no syntactic update is passed directly as a C call argument, so C's unspecified argument order decides. Binary operands follow the same direct-emission path.
- **Recommendation**: when two or more argument (or operand) positions have effects (`expression_has_effects`, which already counts calls), bind every effectful position except the last into a statement temporary in source order, as the template and aggregate-literal paths already do; add a run golden with two effectful arguments and two effectful operands.
- **Effort**: M
- **Related**: WS3-03, WS3-08; the same argument-order class was fixed for narrower shapes in DEF-184 (update operands), DEF-278 (owning argument before a later update), DEF-438 (take-deref carrier hoist), and DEF-552 (later statement-store literal) — each fix pinned only the shape it found, so plain effectful calls were never pinned (input to WS8-07/WS8-08)

### ASM-WS1-003 — Data inclusion (`embed`) is staged out, but chapter 13 reads stable and the limitation is undocumented

- **Severity**: Minor
- **Confidence**: verified
- **Lens**: all
- **Location**: `compiler/src/sema/expr.rs:1734-1749`; `compiler/tests/compile_fail_spec_test.rs:24-52` at e24a8752
- **Checklist item**: WS1-08
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Reproduction — probes `probes/WS1/058`–`064`, command `make -C docker exec CMD="python3 plans/independent-assessment/probes/WS1/probe.py plans/independent-assessment/probes/WS1 --clc build/tmp/assessment/baseline/clc --only 058 059 060 061 062 063 064"`. Expected (spec/13-data-inclusion.md, status "stable (v1.0)", §1 DI-1..DI-5 and its own examples): a module `const DATA: u8[10] = embed("…")` and a function-local `const d = embed("…")` are accepted and yield the file bytes; an absolute, escaping, or missing path is E10214; a local `let` is E10213. Observed on host, hosted-amiga, hosted-amiga-13: every module `const` with `embed` is E10224 ("expression is not constant"), every local `const` and module `let` is E10213, so E10214 is never reported. The frozen golden `spec/tests/compile-fail/E10214_embed_missing_file.clo` produces E10224, and `compile_fail_spec_test.rs` excludes E10214 from the oracle ("data inclusion is not part of the implemented language subset"); `sema/expr.rs:1734` states "Data inclusion is staged out". `docs/known-limitations.md` does not mention it.
- **Impact**: every program that bakes an asset with `embed` (the spec's own sprite, palette, and level examples) is rejected on every profile; the spec suite hides the gap by excluding its golden.
- **Root cause**: the `embed` intrinsic was never implemented beyond the constant-context diagnostic; the constant folder treats it as a non-constant form.
- **Recommendation**: list data inclusion in `docs/known-limitations.md`, report a staged diagnostic (E9xxxx, as `comptime` does with E90001) instead of E10224, and either implement chapter 13 or ask for a spec ruling that marks it staged.
- **Effort**: M
- **Related**: `plans/clorraine-compiler-mvp/00-ambiguity-register.md` AR-10 (user ruling: data inclusion staged out of the MVP, E10213/E10214 goldens skipped by an explicit list); `plans/def-critical-major-closeout/00-ambiguity-register.md` AR-23 (user ruling 2026-10-24: a module-level `let embed` follows spec/04 §16 and is E10213; the spec/13 §1 sentence is a recorded frozen inconsistency); WS8-01/WS8-05 (excluded golden)

### ASM-WS1-004 — `clc` aborts with a stack overflow on moderately deep nesting

- **Severity**: Critical
- **Confidence**: verified
- **Lens**: host (the compiler process; every profile is affected)
- **Location**: the recursive stages after parsing (sema, ownership, lowering, codegen) run on the main thread with no depth limit; no `stack_size`/depth guard exists anywhere in `compiler/src` (grep at e24a8752)
- **Checklist item**: WS1-10
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Reproduction — `probes/WS1/065-robustness.py`, command `make -C docker exec CMD="python3 plans/independent-assessment/probes/WS1/065-robustness.py build/tmp/assessment/baseline/clc plans/independent-assessment/findings/records/assessment-baseline-compile.json build/tmp/assessment/WS1/robustness"`. Expected (03-01 §Severity, `065-robustness.expected`): exit 0 or a coded diagnostic, never a panic or signal. Observed: `thread 'main' has overflowed its stack … aborting` (signal 6) for parens-10k (20 KB source), parens-100k, blocks-10k, unary-100k, addchain-100k, ifchain-5k, calls-10k. Bisection (`build/tmp/assessment/WS1/bisect.py`): the first failing size is 758 nested calls, 1,172 terms in one `+` chain, 1,378 nested blocks, 3,819 nested parentheses, 4,310 `else if` arms. `--parse-only` accepts a 3,000-term chain and 2,000 nested calls, so the overflow is in a later stage.
- **Impact**: generated or table-like sources (long sums, deep `else if` dispatch, nested helper calls) crash the compiler with no diagnostic; the crash also takes down `clorraine-lsp`, which runs the same analysis on every keystroke.
- **Root cause**: recursive tree walks with large frames on the default 8 MB main-thread stack and no nesting limit.
- **Recommendation**: run the compiler pipeline on a thread with a larger explicit stack and add a nesting-depth limit with a coded diagnostic (an implementation limit), or convert the deepest walks (left-associated binary chains, `else if` chains) to iteration.
- **Effort**: M
- **Related**: WS7-06; the LSP shares `analysis.rs`

### ASM-WS1-005 — Polymorphic recursion with explicit type arguments hangs `clc` instead of E10305

- **Severity**: Critical
- **Confidence**: verified
- **Lens**: host (the compiler process; every profile is affected)
- **Location**: `compiler/src/sema/call.rs:1314-1330`, `compiler/src/sema/call.rs:957-971` at e24a8752
- **Checklist item**: WS1-10 (also WS1-04 generic checking)
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Reproduction — `065-robustness.py` case `generic-recursion` (288 bytes: `func g<T>(x: T, n: i32): i32 { … return g<W<T>>(w, n - 1); }`). Expected (spec/07-structs.md STR-24: instantiation recursion beyond the cap is E10305; 03-01 §Severity: a hang is Critical): a coded diagnostic within the limit. Observed: no exit within 600 s (killed); a second run under `timeout 20` also did not finish (`rc=124`).
- **Impact**: a small, plausible generic program (a recursive container walker that wraps its element type) hangs the compiler and the language server.
- **Root cause**: E10305 is judged only on the inferred-type-argument path (`if infer { … is_recursive_instantiation … }`); an explicit type-argument list skips the check, and monomorphization has no instantiation-depth cap of its own.
- **Recommendation**: judge `is_recursive_instantiation` for explicit type arguments too, and add an instantiation-depth cap in monomorphization that reports E10305 as a backstop.
- **Effort**: S
- **Related**: WS2-08 (generics); DEF-397 (`qualified-access-residue`, fixed): "an explicit type-argument list fixes the instance", which is why the explicit path skips the recursion check

### ASM-WS6-001 — `readFile` reports a read error exactly like end of file

- **Severity**: Major
- **Confidence**: verified
- **Lens**: host (reproduced); A500, A1200 shipped (the same `fread` code)
- **Location**: `stdlib/std.io.clo:46-54` (`readFile` returns `Read`'s count), `compiler/runtime/shim_host.c:645-648` and `compiler/runtime/shim_hosted_amiga.c:1259-1262` (`Read` is `fread`, which returns 0 on both end of file and error; `ferror` is never consulted); `writeBytes` / `Write` likewise report a failed write only as a short count, at e24a8752
- **Checklist item**: WS6-02 (also WS6-06)
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Reproduction — probe `probes/WS6/617-read-error.clo`, command `make -C docker exec CMD="python3 plans/independent-assessment/probes/WS1/probe.py plans/independent-assessment/probes/WS6 --clc build/tmp/assessment/baseline/clc --sanitize --only 617"`. Expected (`617-read-error.expected`: spec/appendix-stdlib.md §4 — partial IO returns the count moved plus a status; failures are not hidden; C99 7.19.8.1 — end of file and error are told apart only by `feof`/`ferror`): a failing read (a directory opened for reading, `EISDIR`) and a clean end of file report differently ("true true"). Observed on host debug, release, ASan, and UBSan: "true false" — both return 0; the contrast (two clean ends of file) prints "true false" as expected.
- **Impact**: a program cannot tell a failed read from the end of a file: a copier or loader stops early and reports success with truncated data (a bad floppy sector on the Amiga, a removed volume, a directory given as a file). The `std.io` doc comment groups the two ("zero or less means end of file or an error"), but no value is ever negative, so even that reading cannot be acted on. The `file-copier` corpus program follows exactly this loop.
- **Root cause**: the shim maps the AmigaDOS-style `Read` contract (−1 on error) onto `fread`, which has no error return.
- **Recommendation**: return −1 when `ferror` is set (and clear it), as the AmigaDOS `Read` contract does, and document the −1; or return the spec §4 count-plus-`Status` pair. Same for `Write`.
- **Effort**: S
- **Related**: SQ-WS6-2 (the §4 carrier); ASM-WS6-004 (`std.io` has no appendix table)

### ASM-WS6-002 — `slice` of a `str` is rejected, although the appendix gives it the same view type

- **Severity**: Major
- **Confidence**: verified
- **Lens**: all
- **Location**: `stdlib/std.mem.clo:40` (`intrinsic func slice<T>(v: T[], lo: usize, hi: usize): T[]` — `str` does not unify with `T[]`), the intrinsic's checking in `compiler/src/sema/call.rs:114`, `:341`, `:557`, at e24a8752
- **Checklist item**: WS6-01
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Reproduction — probe `probes/WS6/612-cstring-of-slice.clo` (`let part: str = slice(full, 0, 36);`), command `make -C docker exec CMD="python3 plans/independent-assessment/probes/WS1/probe.py plans/independent-assessment/probes/WS6 --clc build/tmp/assessment/baseline/clc --only 612"`. Expected (spec/appendix-stdlib.md §11: `slice(view, lo, hi)` — "view -> same view type", no allocation; spec/08 §3: the stdlib `slice` helper is a source of views; the expectation file pre-registers a rejection as this finding): the program compiles. Observed on all three profiles: `error E10200: cannot convert '?[]' to 'str' implicitly`.
- **Impact**: text code cannot take a substring view without copying (`stringOf` of bytes is not available either: `str` from `u8[]` has no safe conversion), so parsers either copy or work on `u8[]` and lose the `str` type. A valid program per the appendix is rejected.
- **Root cause**: `slice` is declared over `T[]` only; `str` is a distinct view type, and the intrinsic has no `str` overload.
- **Recommendation**: give `slice` a `str` form (or a `substr(s: str, lo, hi): str` helper) with the same bounds check; at the same time decide SQ-WS6-3 — `toCString` returns the view's pointer without a terminator at the view's end, which is sound today only because every `str` ends at a NUL.
- **Effort**: M
- **Related**: SQ-WS6-3 (`toCString` of a sub-view); ASM-WS1-003 (the earlier spec-vs-implementation gap class)

### ASM-WS6-003 — NaN prints as `-nan` on the host and `NaN` on the Amiga profiles

- **Severity**: Minor
- **Confidence**: verified
- **Lens**: host vs. A500/A1200 shipped
- **Location**: `cl_text_f64` / `cl_str_part_f64` through each C library's `%g` (`compiler/runtime/shim_host.c:540-548`, `compiler/runtime/shim_hosted_amiga.c:1158-1166`), at e24a8752
- **Checklist item**: WS6-02 (and WS5-11 parity)
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Reproduction — probe `probes/WS6/611-float-format.clo`, command `make -C docker exec CMD="python3 plans/independent-assessment/probes/WS1/probe.py plans/independent-assessment/probes/WS6 --clc build/tmp/assessment/baseline/clc --sanitize --only 611"`. Expected (`611-float-format.expected`: S2a six significant digits; one program prints one text on every profile): `nan inf -inf`. Observed: host (all four lines) `-nan inf -inf`; hosted-amiga and hosted-amiga-13 `NaN inf -inf`. The finite values (`0.1 2.5 1.23457e+08 1e+300 -0`) agree everywhere.
- **Impact**: a host test golden for a NaN result does not match the target; a NaN's sign leaks into text on the host only.
- **Root cause**: the template delegates to each libc's `%g`, whose NaN spelling and sign handling differ.
- **Recommendation**: format non-finite values in the shim (`nan`, `inf`, `-inf`) before calling `%g`; record the spelling in the spec (SQ-WS6-1).
- **Effort**: S
- **Related**: SQ-WS6-1

### ASM-WS6-004 — 25 public functions, including `std.io`'s writers, have no specification entry

- **Severity**: Minor
- **Confidence**: verified
- **Lens**: all
- **Location**: `stdlib/` exports listed below, at e24a8752
- **Checklist item**: WS6-01, WS6-06
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Measurement — 600's undocumented list (`records/WS6-runs.log`) filtered by `grep -c "\b<name>\b" spec/*.md` = 0: `std.io.createFile`, `std.io.writeBytes`, `std.amiga.dos.delayTicks`, `std.amiga.timer.freeTimer`, `std.amiga.input.decodeJoystick`, `std.amiga.intuition.openScreenHires`, `std.amiga.intuition.openWorkbenchWindow`, `std.amiga.audio.sampleCycles`/`samplePeriod`/`sampleVolume`, `std.amiga.sprite.freeAnimation`/`spriteChannel`/`spriteHeight`/`spriteVisible`/`spriteX`/`spriteY`, `std.amiga.video.bitmapBytesPerRow`/`bitmapDepth`/`bitmapHeight`/`chipRead16`/`chipRead32`/`chipWrite16`/`chipWrite32`/`freeCopper`/`openDisplayAt`. `std.io` as a whole has no API table (its read side appears only in chapter examples).
- **Impact**: the appendix is the frozen API oracle; functions outside it have no stated ownership, nullability, or fault behavior to conform to (the `readFile` defect, ASM-WS6-001, sits in such a gap), and changes to them are not spec changes.
- **Root cause**: modules grew after the appendix tables were written.
- **Recommendation**: add a `std.io` section and the missing rows (a spec change, so it needs a user ruling); or mark the functions internal.
- **Effort**: M
- **Related**: ASM-WS6-001

### ASM-WS6-005 — The appendix's Kickstart minimums and Box signatures drift from the code

- **Severity**: Minor
- **Confidence**: verified
- **Lens**: A1200 shipped (the 2.0 modules); all (spelling)
- **Location**: `spec/appendix-amiga.md:236-250` ("2.0" for `std.amiga.utility`, `gadtools`, `asl`, `regions`, `commodities`) against their opens at V37 (`shim_hosted_amiga_optional.c`, `_optional_ui.c:38`, `:46`, `_optional_services.c:76`); `spec/appendix-stdlib.md:88-92` (`value: take T`) against `stdlib/std.mem.clo:75-91` (`take value: T`) and grammar `param = [ param_mode ] , identifier , ":" , value_type`, at e24a8752
- **Checklist item**: WS6-04, WS6-01
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Measurement — 640 (`above-min` for the five modules: V37 requested, appendix 2.0 = V36) and 600 (the four Box rows), `records/WS6-runs.log`.
- **Impact**: on a V36 Kickstart 2.0 (early A3000 ROMs) the five modules return null while the appendix says they work; the Box rows teach a parameter spelling the grammar rejects.
- **Root cause**: "2.0" is used for 2.04 (V37) in the table; the Box rows were written in a type-position mode style.
- **Recommendation**: state "2.04 (V37)" in the table; respell the Box rows (both are spec edits needing a ruling).
- **Effort**: S
- **Related**: ASM-WS5-019 f (V38/V44 tags behind V37 opens)

## Clean outcomes to sample

From `findings/WS1-semantic-core.md` §Checklist outcomes:

| ID | Outcome | Evidence |
|----|---------|----------|
| WS1-05 | `clean` | probes 039–047: folded and run-time values agree for every width (wrap, MIN×−1, −MIN, truncating `/` and `%`), every shift-count mask including `i8`/`u8`/`u16`/`i16` and a negative count, MIN/−1 faults for `i8` and `i16`, constant division by zero and MIN/−1 (E10223), constant float-to-int overflow (E10202), run-time float-to-`u8` fault (E11212) — host, hosted-amiga, hosted-amiga-13, debug and release |
| WS1-07 | `clean` | probes 050–052 (every stdlib module on hosted-amiga, the 20 KS1.3 modules on hosted-amiga-13, the 6 core modules on host) build with `-Wall -Wextra -Werror` and run; the seeded conflicting declaration fails the build. Site list: 419 stdlib `ffi` functions, of which only `std.io` `Open`/`Close`/`Read`/`Write` and `std.mem` `FreeBuffer` are not `cl_*` shim functions; the four DOS names are shim-defined on purpose (`shim_hosted_amiga.c:63-70`), so no stdlib `ffi` declaration binds an NDK prototype directly |

From `findings/WS6-stdlib.md` §Checklist outcomes:

| ID | Outcome | Evidence |
|----|---------|----------|
| WS6-03 | `clean` | 620 on the A500 lens: every workload linear (ratio 1.98–2.03; seeded quadratic 4.00). Marginal costs: `push` 22.7 instructions/element, `padLeft`/`clone`/`stringOf` 10.1/byte, `bufferFrom`+hash 18.1/byte, `toDecimal` 318 per call, a two-placeholder template 711 per call (the formatting cost is ASM-WS5-022 / ASM-WS4-001) |
| WS6-07 | `clean` | 670: 43 files, 43 embedded modules, every embedded name equals the file's `module` declaration, every 1.3-allowlist entry exists, every `ffi` function has a C body in the shims of every profile that allows its module; the seeded missing function is reported |

