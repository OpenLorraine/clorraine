# Ambiguity Register: Template Placeholder Residuals III

> **Status**: ✅ GATE PASSED — all 15 items resolved
> **Last Updated**: 2026-11-12 09:05
> **Authority mode**: auto-design (root invocation
> `codeops-20261112-0845-template-placeholder-residuals-iii`, policy version 1)

## Register

| # | Category | Ambiguity / Gap | Options Presented | Decision | Status |
|---|----------|-----------------|-------------------|----------|--------|
| 1 | Behavioral | An imported alias of `text` (`import { text as t }`) bypasses the compiler-known branch and emits an undeclared C symbol | support the alias / reject the spelling / leave | Support it: an import binding whose module is `std.fmt` and original name is `text` names the intrinsic; the alias keeps every intrinsic judgment | ✅ Resolved |
| 2 | Behavioral | `std.fmt.text(7)` bypasses the compiler-known branch and emits `std.fmt.text(...)` | support the qualified spelling / reject / leave | Support it when no real declaration is reached through the path; a user `std.fmt.text` function, `ffi` declaration, or module global wins, for both the qualified and aliased spellings | ✅ Resolved |
| 3 | Technical | How the three spellings reach the `cl_text_*` lowering | canonicalize to `text` / use the internal `$text` sentinel / duplicate codegen cases | Canonicalize every intrinsic form to the internal `$text` sentinel and remove the name-based `text` arm: `render_call` handles the sentinel before any user-symbol resolution, so an unrelated function or `ffi` named `text` keeps its own symbol | ✅ Resolved |
| 4 | Behavioral | A generic conditional used directly as a condition reports E10012 at its null arm where the concrete twin reports E10218 only | carry the condition context / special-case null arms / leave | Carry a `condition` flag on the deferred conditional; a non-bool substituted result reports E10218 at the conditional span and suppresses the null-arm E10012 | ✅ Resolved |
| 5 | Behavioral | `let z = c ? null : null;` is accepted with a `void *` C type, and `settle_null_arm` calls it `Deferred` | reject / accept as the null value / add a settlement variant | Accept it deliberately as the null value. The `Deferred` return has no consumer for this shape (a no-destination pair is not recorded as an opaque conditional), so no instance duty exists and no settlement variant is added; document the decision and pin it with guard tests | ✅ Resolved |
| 6 | Behavioral | A both-null conditional at a nullable destination | accept / reject | Accept; the destination is nullable, matching `slot = c ? null : null` and `let z = null` | ✅ Resolved |
| 7 | Behavioral | A both-null conditional at a concrete destination | E10012 once / E10012 per arm / E10500 | Keep the existing E10012 at each written null arm, exactly as the concrete destination rule already reports | ✅ Resolved |
| 8 | Behavioral | A both-null conditional in a placeholder | accept / E10500 | Keep the existing E10500 `type 'null' is not formattable`; the placeholder rule owns it | ✅ Resolved |
| 9 | Behavioral | `null == x` with `x: T` is accepted and emits a pointer/integer comparison at `T = i32` | defer to the instance / reject the open form / leave | Defer the **left-literal** form: record the comparison and report E10012 at the null when the substituted sibling is not null-capable. The right-literal `x == null` / `x != null` form stays the frozen generic narrowing idiom and is unchanged | ✅ Resolved |
| 10 | Behavioral | Diagnostic for the deferred null comparison | E10012 / E10200 / E10218 | E10012 at the literal null, exactly the concrete twin's rule; `x != null` on a non-null-capable instance is the same report | ✅ Resolved |
| 11 | Behavioral | `panic(cstring null)` calls `strlen(NULL)` before the guarded panic | guard the panic length / make `cl_cstring_len` null-safe / leave | Make `cl_cstring_len` return 0 for NULL in all three shims, honoring O7 (zero = empty). A language-level `len(cstring)` rejects a statically possibly-null operand with E10010; a runtime-null cstring (for example an FFI result) now gets the empty length | ✅ Resolved |
| 12 | Behavioral | A for-step fixed-array assignment from a call or member keeps the compact header and emits invalid C | reject every fixed-array step target / special-case calls / leave | Force the flat loop for any `Assign` whose target type is a fixed array; the ordinary store path supplies the hidden out-parameter and the element copy | ✅ Resolved |
| 13 | Scope | Whether any spec, stdlib, or ABI artifact changes | edit spec / no edit | No `spec/`, `stdlib/`, or ABI edit. The fixes apply the frozen formattability, condition, null, O3/O7, and arity rules; the both-null shape is a delegated decision the spec leaves open | ✅ Resolved |
| 14 | Non-functional | Verification and regression budget | full gate + 1% guard / correctness only | Full gate (`check`, `asan`, `ubsan`, `bench-shape`, `bench` + `bench-compare`, `verify`) against `541ba27`; every scored output equal and no scored metric above 1% | ✅ Resolved |
| 15 | Testing | The alias/qualified and generic-condition cases need an oracle that distinguishes a true intrinsic judgment from an opaque call | single-file / two-file pair / compile-only | Multi-file pairs where a declaration is involved (`import { helper as t }`, a user `std.fmt.text`), single-file cases otherwise; every case asserts output or a diagnostic, never just acceptance | ✅ Resolved |

## Delegated Resolution Records

All decisions are eligible technical consequences of the frozen specification
and issue #29's six fix directions. Root invocation
`codeops-20261112-0845-template-placeholder-residuals-iii`.

**AR-1 to AR-3 — alias and qualified `text`.** `text` is compiler-known but
imported by name: the resolver requires an import, and the embedded `std.fmt`
does not declare it. An aliased import keeps `binding.original == "text"` and
`binding.module == "std.fmt"`, so the type stage can recognize it without a
second intrinsic table. `std.fmt.text` is the same helper reached through its
module path. A user module may declare `std.fmt.text`; the ordinary
`table.function`/`qualified_decl_kind` route must win, so the intrinsic path is
gated on no real declaration. Rejected: rejecting the alias/qualified spelling
(valid imports would lose the converter) and duplicating the codegen selector
per spelling (three copies of the same type dispatch).

The lowered spelling of every intrinsic form — including a module
initializer's own AST walk — is the internal `$text` sentinel. An `ffi` block
name is not a callable signature and does not shadow the intrinsic; a declared
`ffi` signature does. The alias/qualified recognition uses the import binding
or module path, never the program-wide simple-name fallback.
`render_call` checks `$text` before it resolves func-typed bindings and named
functions, so an unrelated user function named `text` cannot capture an alias
call; the old name-based `text` arm is removed so a user `ffi text` keeps its
own symbol. The sentinel renders the same `cl_text_*` selector as before.

**AR-4 — generic condition context.** `check_conditional` already receives the
one-shot `condition_root` flag and uses it to suppress a non-bool conditional's
null-arm report in favor of E10218. The deferred `OpaqueConditional` does not
carry it, so the instance reports the null arm before `require_bool` can own
the site. The flag is recorded on the deferred site; the instance reports
E10218 at the conditional span for a settled non-bool type. A direct condition
then lets the condition rule own the site; a `&&`/`||` operand also keeps the
operand judge's null-arm E10012 (reported before E10218), matching the concrete
twin. The site reports once across instances. A bool settlement
keeps the ordinary null-arm rule, and an arm-agreement failure keeps E10200. The concrete E10218 span is the conditional
expression itself (its start is the inner condition), which is exactly the
recorded `span`.

**AR-5 to AR-8 — both-null conditional.** A conditional with two null arms has
no concrete value type; it is the null value itself. The bare and aliased
`text` gates also reject a user `std.fmt` `ffi` declaration, so a real
`ffi text` symbol is never replaced by the intrinsic. The existing acceptance
matches the bare `let z = null;` rule, and every concrete use already reports
through the ordinary null rules. A no-destination conditional of that pair is not recorded as an opaque
conditional (the opaque flag needs an open destination or a parameter), so the
`Deferred` classification has no consumer and creates no phantom instance duty. The plan
documents that decision in `settle_null_arm` and pins the behavior with guards:
accepted in an inferred binding or nullable destination, E10012 at each written
arm at a concrete destination, E10500 in a placeholder, E10218 in a condition.
Rejected: rejecting the shape (it is semantically a bare null) and adding an
inert settlement variant (dead code with no consumer).

**AR-9, AR-10 — null compared with an open parameter.** A literal `null` is
legal against a null-capable sibling; a still-open parameter hides whether the
instance is null-capable. The issue's reproduction is the **left-literal**
`null == x` form. The right-literal `x == null` / `x != null` form is the
frozen generic narrowing idiom: the existing conformance oracle pins that a
generic body keeps it and that the concrete `pair<i32>` case reports only at
the null argument, not at the body test. So only the left-literal comparison
records a deferred duty; the instance substitutes the sibling and reports
E10012 at the null when it is not null-capable. Rejected: rejecting the open
form (it would break generic narrowing and the conformance oracle) and
accepting the substituted non-nullable left-literal form (it emits invalid C).

**AR-11 — cstring panic.** `cstring` is a nullable-capable view type, so a
null cstring is its empty state. `cl_cstring_len(NULL)` must therefore be 0.
The panic then passes a zero length and the shipped zero-length guard skips the
message write. A language-level `len(cstring)` already rejects a possibly-null
operand with E10010, so the shim change is only observable through the panic
path. Rejected: a codegen-only guard (three shims would disagree about the
empty length) and a panic-only null substitution (more work for the same
output).

**AR-12 — for-step fixed array.** A fixed array has no C assignment or
value-return form: the store needs the hidden destination argument or an
element copy. The compact `for` header cannot carry that statement form, so any
fixed-array step target takes the flat label/`goto` loop. Rejected:
special-casing only calls (a member source has the same defect) and rejecting
the shape (the flat store already supports it).

**AR-13 to AR-15 — scope, verification, and test shape.** No `spec/`,
`stdlib/`, or ABI edit; the frozen rules already decide every other shape. The
full gate runs on the pinned SDK image; FS-UAE legs use the same private-Xvfb
procedure as PLAN-32. The performance base is the current integration head
`541ba27`. Multi-file cases pass both the declaring module and the program on
the compiler command line, as a build does.

## Reopen Triggers

- An alias or qualified `text` call emits a symbol other than `cl_text_*`, or
  canonicalization captures an unrelated user function named `text`
  (AR-1 to AR-3).
- A user `std.fmt.text` declaration no longer wins over the intrinsic
  (AR-2).
- A generic conditional used as a condition reports E10012 where the concrete
  twin reports E10218, or vice versa (AR-4).
- A both-null guard changes an existing typed-destination report (AR-5 to
  AR-8).
- The deferred null comparison rejects `x != null` for a nullable-capable
  instance or accepts it for a scalar (AR-9, AR-10).
- `panic(cstring null)` still reaches `strlen(NULL)` on any shim (AR-11).
- A fixed-array for-step assignment still emits the compact header (AR-12).
- `make bench-compare` shows a scored regression above 1% (AR-14).
