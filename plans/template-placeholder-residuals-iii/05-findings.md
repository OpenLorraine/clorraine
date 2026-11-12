# Findings and Evidence: Template Placeholder Residuals III

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)
> **Created**: 2026-11-12 09:40
> **Status**: Done — full gate green; #29 closed

## 1. Red Evidence (baseline `541ba27`)

The spec suite ran against the base worktree compiler
(`build/base29/target/debug/clc`). 24 cases failed:

- ST-1–ST-9, ST-12, ST-13: the aliased or qualified `text` spelling was
  accepted and emitted an undeclared symbol, or skipped the intrinsic
  judgment.
- ST-16, ST-17, ST-19–ST-21: the generic condition reported E10012 at the
  null arm (or was accepted for `String`) instead of the concrete E10218;
  ST-19 also missed E10218.
- ST-33, ST-34: the left-literal null comparison at `T = i32` was accepted
  and emitted pointer/integer C.
- ST-41: `panic(cstring null)` reported the UBSan null `strlen`.
- ST-43, ST-44: the fixed-array step kept the compact header and failed the
  strict C build.
- ST-46: an unrelated loaded module's function captured the alias.
- ST-47: the module initializer spellings were not canonicalized.
- ST-49: the multi-instance logical condition missed E10218.

The 25 guards passed on the base: ST-10, ST-11, ST-14, ST-15, ST-18,
ST-22–ST-32, ST-35–ST-40, ST-42, ST-45, and ST-48.

## 2. Implementation

- `compiler/src/sema/call.rs`: `check_text_call` is the one intrinsic body for
  the bare, aliased, and qualified spellings. `is_text_intrinsic_name` accepts
  the bare name and the `std.fmt` alias, excluding a real function/`ffi`
  signature/global declaration; `is_std_fmt_text_path` gates the qualified
  path on the head not being a value and on no real declaration. An imported
  `ffi` call now resolves its written signature through the import binding.
- `compiler/src/own/expr.rs`: `is_builtin_text_call` recognizes the alias and
  qualified forms from the import binding or module path (never the
  program-wide simple-name fallback), rejects a real function/`ffi`
  signature/target declaration, and mirrors the type stage's value-head gate.
- `compiler/src/own/call.rs`: every intrinsic spelling lowers to the `$text`
  sentinel; `compiler/src/codegen/expr.rs` renders the sentinel before user
  resolution, and the old name-based `text` arm is removed. A module
  initializer canonicalizes the same spellings in its own AST walk
  (`compiler/src/codegen/program.rs`).
- `compiler/src/sema/mod.rs`, `ops.rs`: `ConditionalArms`/`OpaqueConditional`
  carry `condition` and `judged_operand`; the `&&`/`||` operands use
  `check_judged_condition_value`. A new `OpaqueNullComparison` records a
  left-literal null against an open parameter.
- `compiler/src/own/instance_check.rs`: the instance conditional reports
  E10218 for a non-bool settlement; a direct condition reports it alone, a
  logical operand reports the null-arm E10012 first and E10218 after, matching
  the concrete twin, once per site. The null-comparison audit reports E10012
  at the literal for a non-null-capable substituted sibling.
- `compiler/src/types/mod.rs`: the both-null settlement decision is documented
  in `settle_null_arm`; no behavior or variant change.
- `compiler/runtime/shim_host.c`, `shim_hosted_amiga.c`,
  `shim_freestanding.c`: `cl_cstring_len(NULL)` is 0.
- `compiler/src/own/loops.rs`: a fixed-array step target forces the flat loop.

The right-literal `x == null` / `x != null` form is unchanged: it is the frozen
generic narrowing idiom, and the existing conformance oracle
(`should_keep_a_caller_parameter_resolved_nullable_instance_accepted`) pins
that the concrete `pair<i32>` case reports only at its null argument. The
issue's reproduction is the left-literal `null == x` form.

## 3. Green Evidence

- `template_placeholder_residuals_iii_spec_test`: 49 passed.
- `template_placeholder_residuals_iii_impl_test`: 13 passed.
- `checker_conformance_spec_test`: 212 passed.
- `make check`: green (fuzz-smoke 28/0).

## 4. Phase Review

### 4.1 First Round

| Review | Verdict | Findings |
|--------|---------|----------|
| Correctness | no critical; 1 major, 3 minor | RV-001 value-head split, RV-002 stale comments, RV-003 logical parity, RV-004 bookkeeping |
| Semantics | no critical; 3 major, 1 minor | SR-001 imported ffi typing, SR-002 value-head split, SR-003 alias capture, SR-006 order |
| Memory safety | clean in scope | pre-existing text-argument leak recorded |
| Emission integrity | 1 major | imported ffi invalid C |

**RV-001 / SR-002 (MAJOR) — module-level value head split.** Sema treated a
module-level `std` value as a member path while own lowered it as `$text`.
The original fix made both stages agree on the member path. **PLAN-35 (#31)
supersedes the module-level clause**: MOD-13 makes the qualified module path
always available, and MOD-16 lets only a local shadow it. The gate is now
lexical-only; IT-13 pins the module-path emission.

**SR-001 (MAJOR) — imported `ffi text` typed as unknown.** The sema `ffi`
branch read only the current module's signature. It now falls back to
`qualified_ffi_signature(binding.module, binding.original)`, so the call result
is the declared `String` and `println` widens. IT-11 object-builds the pair.

**SR-003 (MAJOR) — unrelated module function captured the alias.** Own's
`resolved_callee` consulted the program-wide simple-name map. The alias now
resolves from the import binding alone; ST-46 loads an unrelated module
exporting `t(take String)` and still prints `7`.

**RV-003 / SR-006 (MINOR) — logical-operand parity.** `&&`/`||` operands now
carry `judged_operand`; the instance reports the null-arm E10012 first and
E10218 after, matching the concrete twin, once per site. ST-19 and ST-49 pin
it.

**RV-002 / RV-104 (MINOR) — stale comments and simple-name resolution.** The
comments now describe the left-literal form and the sentinel-only codegen path.
The simple-name resolution in the deferred duties is shared with the existing
infrastructure and remains a pre-existing shape.

### 4.2 Second Round

| Review | Verdict | Findings |
|--------|---------|----------|
| Correctness | no new critical/major | RV-101 alias modes (fixed), RV-102 duplicate E10218 (fixed), RV-103 bookkeeping (this document) |
| Semantics | 2 major | SR-004 initializer spellings, SR-005 ffi block name |
| Memory safety | clean in scope | pre-existing text-argument leak recorded |
| Emission integrity | clean after the fixes | pre-existing module-level member path noted |

**SR-004 (MAJOR) — module initializer spellings.** `lower_global` built the
call name from the AST and lost the canonicalization. `init_text_intrinsic_name`
now mirrors the type-stage recognition; ST-47 runs all three spellings at
module scope.

**SR-005 (MAJOR) — `ffi` block name.** Own rejected any `DeclKind::Ffi`,
including an `ffi` block's name, which is not callable. The gate now rejects
only a function declaration or an actual `ffi` signature; ST-48 pins the
intrinsic under an `ffi text { func foo() }` block.

**RV-101 (MINOR) — alias modes.** `check_call_expr` short-circuits the
intrinsic before the named-callee mode lookup, so a program-wide `take`
function cannot supply modes to `text`. ST-46's holder now exports
`t(take String)`.

**RV-102 (MINOR) — duplicate E10218.** The judged-operand path now breaks after
its report; ST-49 instantiates two non-bool types and gets one E10218 plus one
E10012.

## 5. Residual and Pre-existing Shapes

The reviews found these adjacent shapes; none is caused by this plan and each
reproduces on the base or is a documented limitation:

1. The right-literal `x == null` / `x != null` at `T = i32` remains accepted
   and can emit a pointer/integer C comparison. The conformance oracle pins the
   generic body's acceptance; the issue's reproduction is the left-literal
   form.
2. **Superseded by PLAN-35 (#31).** A module-level or imported value head
   `std` does not shadow the `std.fmt.text` module path (MOD-13); only a local
   does (MOD-16). The module-path intrinsic now fires in bodies and
   initializers, and IT-13 pins that emission.
3. **Fixed by PLAN-35 (#30).** An explicit `text(stringOf("x"))` or
   `text($"{s}")` leaked the owned temporary argument; the intrinsic call now
   borrow-materializes and releases it.
4. The deferred duties resolve the function by simple name, so an unrelated
   loaded module's same-named function can suppress a duty; the shared
   infrastructure has this shape for several duties.
5. The right-literal narrowing form's null-at-scalar case is out of scope by
   the AR-9 decision.

## 6. Acceptance

| Gate | Result |
|------|--------|
| `make check` | green (spec check, workspace tests, bench-test, fuzz-test, fuzz-smoke 28/0) |
| `make asan` | 36 suites green |
| `make ubsan` | 10 suites green |
| `make bench-shape` | gcc, nix13, vbcc, freestanding warning-free |
| `make verify` | `verify: all legs passed` (A500/A600/A1200/A4000 FS-UAE legs, corpus, differential, fault legs) |

The FS-UAE legs ran on a private Xvfb display with `LIBGL_ALWAYS_SOFTWARE=1`;
the local `docker/.env` was restored to the user's original `DISPLAY=:0`
afterward.

`make bench-compare` passed with every scored output equal:

| Benchmark | Base instructions | Head instructions | Delta | Base `.text` | Head `.text` | Delta |
|-----------|-------------------|-------------------|-------|--------------|--------------|-------|
| lookup | 8,779 | 8,779 | +0.0% | 39,884 | 39,884 | +0.0% |
| matrix | 9,361 | 9,361 | +0.0% | 23,592 | 23,592 | +0.0% |
| moves | 44,630 | 44,630 | +0.0% | 23,684 | 23,684 | +0.0% |
| sieve | 1,040,081 | 1,040,081 | +0.0% | 23,816 | 23,816 | +0.0% |
| strings | 1,221,232 | 1,221,232 | +0.0% | 23,972 | 23,972 | +0.0% |

The generated C grows by 90 bytes (+0.0%) from the cstring-length guard; the
scored instructions and `.text` are unchanged, satisfying the 1% guard.

### Final Test Inventory

- `template_placeholder_residuals_iii_spec_test`: 49 behavior cases (plain,
  ASan, UBSan).
- `template_placeholder_residuals_iii_impl_test`: 13 emission/diagnostic
  cases.
- `checker_conformance_spec_test`: 212; `def_semantic_parity_integrity_spec_test`:
  18; `template_placeholder_residuals_ii_spec_test`: 39; `text_arity_spec_test`:
  6; `make check` green.

No critical or major review finding remains open. All minor review findings
from both rounds were fixed; the adjacent pre-existing shapes are recorded in
§5.
