# Template Placeholder Residuals II Implementation Plan

> **Feature**: Close the seven adjacent residuals from #27: explicit `text`
> formattability (including a generic argument), a bare null binary operand,
> generic and scalar null-arm parity, a for-step owning assignment, for-init
> effects, aggregate field order, and `cl_panic` zero-length messages.
> **Status**: Planning Complete
> **Created**: 2026-11-12 02:00
> **Implements**: R1–R11
> **CodeOps Artifact Schema**: 1

**Original goal:** deliver issue #28 — every residual must be rejected by an
existing rule or behave correctly, with no crash, leak, or undefined C.

## Overview

PLAN-31 closed #27's five template-placeholder residuals. Its four-lens review
found seven adjacent pre-existing shapes that reproduce on the base or are
pre-existing gaps. This plan closes each with the frozen rules and reuses the
existing duties:

1. `text(null)`, `text(nonFormattable)`, and `text(x.n)` over an opaque
   parameter projection are accepted and emit a converter that does not exist.
2. A generic `flag ? null : x` reports the wrong diagnostics: E10012+E10010 at
   `T = str`/`String`, and a scalar or struct `T` also carries the spurious
   E10010 beside the E10012 null report.
3. `let x = flag ? null : 5;` is accepted and emits `(flag ? NULL : 5)`, and
   `$"{null + 1}"` emits invalid pointer arithmetic.
4. A for-step owning assignment keeps the compact C header and leaks the old
   value.
5. A template in a for-init expression list is checked but not lowered.
6. A non-`needs_stores` aggregate's field order relies on the supported C
   compilers' initializer order.
7. `cl_panic` writes a zero-length NULL message with `fwrite`.

No language, stdlib, or ABI surface changes.

## Minimum-Sufficient Baseline

**Original goal:** make each #28 reproduction correct or rejected by an
existing rule.
**Smallest viable design:** one explicit-converter formattability check plus a
deferred generic duty and a nullable-use check; one shared null-arm settlement
helper (with a bare-null predicate) used by the concrete conditional and the
instance audit; one null-operand exclusion removal; one loop-shape predicate;
one for-init statement path; one codegen contract comment with a regression
test; two `fwrite` guards.
No new diagnostic code, no new runtime helper, no new language surface, no
emission-wide aggregate rewrite.
**Excluded machinery:** a general nullable dataflow analysis, a new text
conversion framework, a new loop representation, and any spec/stdlib edit.
**Approved complexity:** None — the plan extends existing checks, duties, and
emitters (AR-1–AR-19).

## Document Index

| # | Document | Description |
|---|----------|-------------|
| AR | [Ambiguity Register](00-ambiguity-register.md) | Zero-Ambiguity Gate decisions |
| 00 | [Index](00-index.md) | This document — overview and navigation |
| 01 | [Requirements](01-requirements.md) | Requirements, scope, acceptance |
| 02 | [Current State](02-current-state.md) | Reproductions and root-cause audit |
| 03-01 | [Static Diagnoses](03-01-static-diagnoses.md) | text, null operand, null-arm parity |
| 03-02 | [Lowering and Runtime](03-02-lowering-and-runtime.md) | for-step, for-init, aggregate order, panic |
| 07 | [Testing Strategy](07-testing-strategy.md) | Spec cases and verification |
| 99 | [Execution Plan](99-execution-plan.md) | Phases, sessions, and tasks |
| 05 | [Findings](05-findings.md) | Evidence record (created at the first review) |

## Quick Reference

```clorraine
// Rejected: the explicit converter requires a formattable argument (E10500).
let t: String = text(null);

// Rejected: a bare null has no arithmetic operand surface (E10012).
println($"{null + 1}");

// Rejected: a scalar cannot hold the null arm (E10012).
let x = flag ? null : 5;

// Accepted: a null arm at a view is the empty state.
println($"{flag ? null : "x"}");

// Accepted: the null arm at a String settles to String?; a concrete use reports E10010.
let y = flag ? stringOf("x") : null;

// Fixed: the old String is released in a for step.
for (let i: i32 = 0; i < 3; s = $"{i}", i = i + 1) { }

// Fixed: the for-init template effect runs once.
for ($"{tick()}", i = 0; i < 1; i = i + 1) { }
```

### Key Decisions

| Decision | Outcome |
|----------|---------|
| Explicit `text` argument | E10500 at a null-shaped, non-formattable, or opaque-projection argument (AR-1, AR-2, AR-21) |
| Nullable `text` argument | E10010 unless narrowed (AR-20) |
| Generic `text` argument | Deferred formattability/nullability duty (AR-3) |
| Bare null operand | E10012 outside `==`/`!=` with a null-capable sibling; E10218 for `&&`/`||` (AR-4) |
| Null-arm parity | One shared settlement helper (AR-5 to AR-9) |
| For-step owning assignment | Flat loop form; ordinary pre-drop store (AR-10) |
| For-init effects | Statement path with a release floor/drain (AR-11) |
| Aggregate initializer order | Documented supported-toolchain assumption + test (AR-12) |
| `cl_panic` | Skip the zero-length message write (AR-13) |
| #24 text arity | Separate mini-plan (AR-14) |
| Spec/ABI | No language surface or ABI change (AR-15) |

## Specialist Agents

| Role | Kind | Use | AR Ref |
|------|------|-----|--------|
| correctness-reviewer | reviewer | Phase reviews and fix re-review | AR-16 |
| semantics-reviewer | reviewer | Null-arm parity, formattability, order | AR-1–AR-9, AR-12 |
| memory-safety-auditor | auditor | For-step/for-init ASan probes, panic probes | AR-10, AR-11 |
| emission-integrity-auditor | auditor | Loop shapes, text emission, strict C build | AR-3, AR-10, AR-12 |

_Detection evidence: `.opencode/agents/` contains all four briefs; the change
touches sema formattability and binary operands (`compiler/src/sema`), the
type/null-arm settlement (`compiler/src/types`), the generic instance duty
(`compiler/src/own/instance_check.rs`), the loop lowering (`compiler/src/own/loops.rs`),
the C loop shape (`compiler/src/codegen/control.rs`), and both C shims._

## Related Files

Created:
- `compiler/tests/template_placeholder_residuals_ii_spec_test.rs`
- `compiler/tests/template_placeholder_residuals_ii_impl_test.rs`
- `plans/template-placeholder-residuals-ii/05-findings.md`

Modified:
- `compiler/src/sema/call.rs` — explicit `text` argument formattability
- `compiler/src/sema/ops.rs` — bare-null binary operand, conditional settlement, single-report guard
- `compiler/src/types/mod.rs` — shared null-arm settlement and bare-null predicate
- `compiler/src/own/expr.rs` — nullable `text` argument, deferred duty record, bare-null predicate delegate
- `compiler/src/own/instance_check.rs` — generic null-arm parity and the reused deferred duty
- `compiler/src/own/loops.rs` — for-init statement path, for-step shape
- `compiler/src/codegen/function.rs`, `compiler/src/codegen/guards.rs` — codegen contract comment
- `compiler/runtime/shim_host.c`, `shim_hosted_amiga.c` — zero-length panic guard
- `plans/00-roadmap.md` — PLAN-32 row

Read-only:
- `spec/` — the frozen oracle (no edit)
- `compiler/tests/template_placeholder_residuals_{spec,impl}_test.rs` — #27 oracle
- `compiler/tests/template_placeholder_integrity_{spec,impl}_test.rs` — #26 oracle
