# WS1 — Semantic core

> **Baseline**: e24a8752b08827c3a5d7c77b6e2827dac708b629 (see [WS0-baseline.md](WS0-baseline.md))
> **Checklist**: 03-02-compiler-workstreams.md §WS1 (WS1-01..WS1-10)

## Scope and limits

Components: `lex`, `parse`, `ast`, `load`, `resolve`, `sema`, `types`, `consts`,
`analysis.rs`, `driver.rs`, `abi.rs`, `ffi.rs`, `profile.rs`. Lens: wrong acceptance and wrong
rejection, including a wrong diagnostic code; diagnostic wording is not assessed.

## Dispatch record

| Role | Effort | Date |
|------|--------|------|
| Executing session (inline) | high | 2026-11-25 |

## Coverage table

| Group | Files | Status |
|-------|-------|--------|
| Probes written (1.1.1) | `probes/WS1/001`–`065` | done |
| Red check (1.1.2) | all probes with a contrast | done |
| Main run (1.2.x) | 225 probe cells + probe 066 (6 cells); log `build/tmp/assessment/WS1/main.log`, results `build/tmp/assessment/WS1/probes/results.json` | done |
| `lex`, `parse`, `load`, `resolve`, `analysis.rs` (1.2.1) | probes 001–029, 066; stage order read | done |
| `sema`, `types`, `consts` (1.2.2) | probes 030–048; `check_int`, `is_recursive_instantiation` read | done |
| `ffi.rs`, `abi.rs`, `profile.rs`, `driver.rs`, data inclusion, robustness (1.2.3) | probes 050–065, gating matrix, robustness, bisection | done |
| Self-refutation (1.3.1) | each finding against `spec/`, `docs/known-limitations.md`, the roadmap DEF ledger, and earlier ambiguity registers | done |

## Checklist outcomes

| ID | Outcome | Evidence |
|----|---------|----------|
| WS1-01 | ASM-WS1-001 | otherwise `clean`: probes 001, 003–018 (ranges, minimum literals, separators, float dot, suffixes, every escape, multibyte char, comments, identifiers, byte length, keywords, reserved names, foreign syntax) matched on three profiles, debug and release |
| WS1-02 | `clean` | probes 019 (precedence and associativity, 12 expressions), 020 (assignment chaining); `--parse-only` handles 3,000-term chains and 2,000 nested calls; stage gating `analysis.rs:150-170` (type check only after error-free resolution, ownership only after an error-free type check) and `driver.rs:231-265` (no emission or output file after any error). The evaluation-order defect found by probe 021 is a lowering defect, filed as ASM-WS1-002 under WS3-03 |
| WS1-03 | `clean` | probes 022 (shadowing), 023 (use before declaration), 024 (non-exported import), 025 (unknown import), 026 (duplicate bind), 027 (qualified access without import), 028 (circular imports), 029 (initializer cycle E11100) |
| WS1-04 | `clean` | probes 030–038 (mixed sign, u8→i16, literal operand typing and wrap, literal fit, non-bool condition, nullable scalar, enum ordering, enum backing, negative literal against u8). Generic constraint checking is covered only where it affects robustness (ASM-WS1-005); the bound diagnostics E10300–E10309 were not re-probed |
| WS1-05 | `clean` | probes 039–047: folded and run-time values agree for every width (wrap, MIN×−1, −MIN, truncating `/` and `%`), every shift-count mask including `i8`/`u8`/`u16`/`i16` and a negative count, MIN/−1 faults for `i8` and `i16`, constant division by zero and MIN/−1 (E10223), constant float-to-int overflow (E10202), run-time float-to-`u8` fault (E11212) — host, hosted-amiga, hosted-amiga-13, debug and release |
| WS1-06 | `clean` | probe 048 with its C companion: 11 struct layouts (sizes, offsets) and two folded constants used as array extents match the target C compiler on host, hosted-amiga, hosted-amiga-13, debug and release |
| WS1-07 | `clean` | probes 050–052 (every stdlib module on hosted-amiga, the 20 KS1.3 modules on hosted-amiga-13, the 6 core modules on host) build with `-Wall -Wextra -Werror` and run; the seeded conflicting declaration fails the build. Site list: 419 stdlib `ffi` functions, of which only `std.io` `Open`/`Close`/`Read`/`Write` and `std.mem` `FreeBuffer` are not `cl_*` shim functions; the four DOS names are shim-defined on purpose (`shim_hosted_amiga.c:63-70`), so no stdlib `ffi` declaration binds an NDK prototype directly |
| WS1-08 | ASM-WS1-003 (Minor after 1.3.1) | the size-limit, alignment, and path-validation checks are unreachable while every `embed` is rejected (probes 059, 060, 062 could not reach E10214); spec/13 §6 sets no chip-placement requirement |
| WS1-09 | `clean` | `057-gating-matrix.py`: 172 module × profile cells match the expectation written from the appendices (0 mismatches) and an unknown profile is E11201; probes 051 (KS1.3 subset builds and runs), 053 (ASL rejected on KS1.3), 054 (Amiga module rejected on host), 055 (template rejected on freestanding); allowlist `profile.rs:93-117`. See SQ-WS1-3 |
| WS1-10 | ASM-WS1-004, ASM-WS1-005 | `065-robustness.py`: ident-1m, string-1mb, literal-10k-digits (E20004), template-10k, switch-5k within limits; array-literal-100k is E10224, which matches spec/04 §16 (array literals are not constant expressions). Compile time grows super-linearly with locals in one function (5k: 0.26 s, 10k: 1.07 s, 20k: 3.1 s, 50k: 53 s); handed to WS7-06 |

## Findings

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
- **Self-refutation (1.3.1)**: spec/01 LEX-14/LEX-15 are explicit; no DEF row or limitation covers an un-annotated out-of-range literal. The impact needs an unusual source spelling, but the result is a silent value change or strict-build failure. Stays Major.
- **Verdict**: confirmed (Major) — verifier ses_dea9a6131ffesU3qOxzrQQoINm (9.2.1): `check_int` (`sema/expr.rs:690-707`) checks fit only with a context type; a missing diagnostic with a silent wrap (implementation-defined, C99 6.3.1.3p3).

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
- **Self-refutation (1.3.1)**: spec/04 §3 is unambiguous ("There is no implementation-defined order"); `docs/known-limitations.md` has no evaluation-order entry; no DEF row records two plain effectful calls. Not a regression of a closed DEF but a new instance of a recurring class. Severity stays Critical (an accepted program miscompiles).
- **Verdict**: confirmed (Critical) — verifier ses_dea9a6131ffesU3qOxzrQQoINm (9.2.1): `2 1` and `321` on every profile and line; the argument pins in `own/call.rs` fire only for syntactic updates; no ruling accepts the order (DEF-278 fixed only the owning-argument shape).

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
- **Self-refutation (1.3.1)**: the absence of `embed` is a deliberate, user-ruled staging decision, so it is not a wrongly rejected valid program in the Major sense. What survives is the documentation gap: spec/13 is marked "stable (v1.0)", `docs/known-limitations.md` (the user-facing list) does not mention that every `embed` is rejected, and `E10224` ("expression is not constant") is a misleading diagnostic for a staged feature. Rated Minor.
- **Verdict**: confirmed (Minor) — verifier ses_dea9a6131ffesU3qOxzrQQoINm (9.2.1): staged out by ruling; the defect is the missing limitation entry and the diagnostic shape (E10224/E10213 instead of the promised E90000).

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
- **Self-refutation (1.3.1)**: no DEF row, plan, or `docs/known-limitations.md` entry sets a nesting limit; 03-01 §Severity rates any `clc` crash Critical (AR #15). The exposure is generated or table-like code rather than typical hand-written code, which the report states. Stays Critical.
- **Verdict**: confirmed (Critical) — verifier ses_dea9a6131ffesU3qOxzrQQoINm (9.2.1): every deep case aborts with a stack overflow; thresholds reproduce (700/800 nested calls, 1,100/1,250 `+` terms); no depth guard in `compiler/src`.

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
- **Self-refutation (1.3.1)**: the explicit-list skip is intended by DEF-397, but spec/07 STR-24 makes E10305 apply to any instantiation recursion beyond the cap, and a hang is never an intended outcome. Stays Critical.
- **Verdict**: confirmed (Critical) — verifier ses_dea9a6131ffesU3qOxzrQQoINm (9.2.1): `is_recursive_instantiation` (`sema/call.rs:957`) skips the explicit path; the 288-byte program times out at 120 s while the inferred twin reports E10305 at once.

## Spec questions

- **SQ-WS1-1 — withdrawn in task 1.3.1.** The module-level `let embed` contradiction between
  spec/13 §1 and spec/04 §16 was already ruled by the user on 2026-10-24
  (`plans/def-critical-major-closeout/00-ambiguity-register.md` AR-23: follow spec/04 §16)
  and recorded there as a frozen inconsistency; it is not re-raised.
- **SQ-WS1-2 — May a hex or binary literal use the full unsigned width in a signed context?**
  spec/01 §7 and spec/02 §9 say a hex or binary literal "may use the full unsigned width of
  the target" (`0xFFFFFFFF` is a valid `u32`). `let a: i32 = 0x80000000;` is rejected with
  E10202 (`build/tmp/assessment/WS1/diag/v.clo`). The text does not say whether the
  full-width allowance is a bit-pattern rule for signed contexts too.
- **SQ-WS1-3 — Is the opt-in `-m68020` / `-m68881` leg a profile requirement?**
  spec/15-profiles.md §2 lists `hosted-amiga` "Default CPU `-m68000` (opt-in `-m68020`)" and
  an "FPU build leg: opt-in `-m68881`". No compiler or driver option offers either (`grep
  -rn "68020\|68881" compiler/src` finds nothing at e24a8752; the flag tables in
  `compiler/src/clorraine/toolchain.rs:58-63` fix `-m68000`). Either the opt-in is a
  missing feature or the table describes flags a user adds by hand. Input to WS7-03.

## Falsifiability log

Red check run 2026-11-25 after the probes were committed (`4ab6b39a`; amended FFI
contrasts `c70f47c7`): `probe.py … --contrast` (log `build/tmp/assessment/WS1/contrast.log`,
`contrast-ffi.log`; results `build/tmp/assessment/WS1/probes/contrast.json`,
`probes-ffi/contrast.json`). Every contrast ran on every listed profile, debug and release
builds where applicable.

| Probe | Form | Red-check result |
|-------|------|------------------|
| 001, 002, 004–007, 009–011, 013, 014, 016–018, 020, 023–026, 029–031, 033–038, 044–046 | contrasting input: the offending token is replaced by a valid one; the reject probe's contrast must be accepted and print the stated line | all contrasts accepted and printed their expected output on host, hosted-amiga, hosted-amiga-13 (debug and release) |
| 003, 008, 012, 015, 019, 022, 027, 028, 032, 039–041 | contrasting input: a changed operand or literal whose expected output differs from the probe's | all contrasts printed their (different) expected output |
| 042, 043, 047 | contrasting input: an in-range operand; the fault probe's contrast must print a value instead of faulting | all printed the value |
| 048 | seeded defect: a field added to the Clorraine `struct C` only; the probe must then differ from its C companion | differed on host, hosted-amiga, hosted-amiga-13 (debug and release) |
| 050, 051, 052 | seeded defect: a user `ffi` declaration of libc `strlen` returning `u16`; the C build must fail | failed with `conflicting types for 'strlen'` on each profile. The original seed (a user `ffi` redeclaring a shim function) was rejected by `clc` with E11101 before C emission, so it could not reach the C compiler; amended in `c70f47c7` with the original kept |
| 053, 054, 055 | contrasting input: the same program shape with an available module or a plain string must be accepted on the same profile | all accepted |
| 057 | contrasting input inside the matrix: the same generated one-import program is expected `A` on some (module, profile) cells and `R` on others, so a gate that accepted or rejected everything would mismatch | form recorded; observed in task 1.2.3 |
| 065 | seeded defect: `--seeded` sets a 1 ms limit on the first case; the checker must report a failure | reported `FAIL parens-10k`, exit 1. The same run showed `clc` ending with signal 6 (`PANIC`) on that input; recorded for WS1-10 in task 1.2.3 |
| 021 | contrasting input: `pair(next() * 10, next())` must print `10 2` | **did not match**: printed `20 1` on every profile and build. The contrast does discriminate (its output differs from the probe's own expectation), and it is itself evidence that call arguments are not evaluated left to right; confirmed or refuted by the main run in task 1.2.1 |
| 066 (added in 1.2.1, committed before it ran) | contrasting input: the `+` chain drops its last term, so line 3 must change from `123` to `120` | line 3 changed to `120` as expected; the contrast still differs from its expectation on line 5 (`321`) for the ASM-WS1-002 reason, so the probe discriminates on both lines |
| 058–064 | contrasting input: a valid `embed` form | **could not run**: every `embed` form, including the spec's own examples, is rejected (`E10224` for a module `const`, `E10213` for a local `const` or a module `let`). The path-validation detectors (059, 060, 062) can never be reached, so their checklist parts are a `limit`; the rejection itself is evidence for WS1-08, recorded in task 1.2.3 |

## Measurement and run records

Committed copies (ST-12): [`records/WS1-results.json`](records/WS1-results.json) (main run,
225 + 6 cells), [`records/WS1-contrast.json`](records/WS1-contrast.json) (red check),
[`records/WS1-gating.log`](records/WS1-gating.log), [`records/WS1-robustness.log`](records/WS1-robustness.log).
All runs: pinned `clc` sha256 `0c2fd66e…`, baseline `e24a8752`, inside the SDK container;
builds use the probe runner's debug and shipped release lines (`probe.py` `BUILDS`), host
x86-64 and `vamos` 68000. Robustness times and RSS: host `clc --profile host`, wall clock and
`ru_maxrss` of the single child, limits from `records/assessment-baseline-compile.json`.
Supplementary diagnosis (not committed, under `build/tmp/assessment/WS1/`): `bisect.py`
(nesting thresholds), `locals.py` (locals scaling), `diag/` (literal and `embed` variants).

## Phase 1 gate (1.3.2)

| Case | Result |
|------|--------|
| Task verify | drift check PASS |
| ST-1 | five findings, every template field present, IDs `ASM-WS1-001..005`, severities and efforts valid |
| ST-2 | each `verified` finding names committed probes (`002`, `021`/`066`, `058`–`064`, `065`) and the command; reproduced at HEAD with the drift check passing |
| ST-3 | no `hypothesis` finding |
| ST-4 | no performance finding — n/a |
| ST-5 | probes committed in `4ab6b39a` before the red check; amended FFI contrasts in `c70f47c7` and probe 066 in its own commit, each before running; falsifiability log names the form per probe |
| ST-6 | code disagreements filed as findings citing spec sections; two spec questions open (SQ-WS1-2, SQ-WS1-3), one withdrawn as already ruled; no file under `spec/` changed |
| ST-7 | drift check PASS; commits touch only this plan's folder and the roadmap row |
| ST-8 | WS1-01..WS1-10 each have one outcome; coverage table complete |
| ST-11 | ASM-WS1-002, -004, -005 escalated at 2026-11-25 20:22 with the message text |
| ST-12 | records above |
| ST-14, ST-15 | no patched C, no 68020 number — n/a |
| ST-16 | dispatch record `high` |

## Escalations

| Time | ID | Message sent to the user |
|------|----|--------------------------|
| 2026-11-25 20:22 | ASM-WS1-002 | "Critical (verified): call arguments with side effects are evaluated right to left on every profile — `pair(next(), next())` prints `2 1`, spec/04 §3 requires `1 2`. Reproduce: `make -C docker exec CMD=\"python3 plans/independent-assessment/probes/WS1/probe.py plans/independent-assessment/probes/WS1 --clc build/tmp/assessment/baseline/clc --only 021 066\"`." |
| 2026-11-25 20:22 | ASM-WS1-004 | "Critical (verified): `clc` aborts with a stack overflow on 758 nested calls, a 1,172-term `+` chain, 1,378 nested blocks, or 4,310 `else if` arms. Reproduce: `make -C docker exec CMD=\"python3 plans/independent-assessment/probes/WS1/065-robustness.py build/tmp/assessment/baseline/clc plans/independent-assessment/findings/records/assessment-baseline-compile.json build/tmp/assessment/WS1/robustness\"`." |
| 2026-11-25 20:22 | ASM-WS1-005 | "Critical (verified): a 288-byte generic program with explicit polymorphic recursion (`g<W<T>>(…)`) hangs `clc` (no exit in 600 s) instead of E10305. Reproduce: the same `065-robustness.py` command (case `generic-recursion`)." |

No fix is made (AR #12).

## Probe index

Runner: `probes/WS1/probe.py` (expectation format in its docstring). Scripts:
`057-gating-matrix.py` (WS1-09), `065-robustness.py` (WS1-10).

| Item | Probes |
|------|--------|
| WS1-01 lexer | 001–018 |
| WS1-02 parser | 019–021 |
| WS1-03 resolution | 022–029 |
| WS1-04 types | 030–038 |
| WS1-05 constant evaluation | 039–047 |
| WS1-06 layout | 048 (+ `048-layout-sizeof.c` companion) |
| WS1-02/WS3-03 evaluation order | 021, 066 |
| WS1-07 FFI | 050–052 |
| WS1-08 data inclusion | 058–064 |
| WS1-09 profile gating | 051–055, 057 |
| WS1-10 robustness | 065 |
