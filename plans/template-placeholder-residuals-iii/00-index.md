# Template Placeholder Residuals III Implementation Plan

> **Feature**: Close the six adjacent residuals from #28/#24: aliased and
> qualified `text`, generic condition context, the both-null conditional,
> literal null compared with an open parameter, `panic(cstring null)`, and a
> for-step fixed-array assignment.
> **Status**: Done — 35/35; full gate green
> **Created**: 2026-11-12 08:45
> **Implements**: R1–R10
> **CodeOps Artifact Schema**: 1

**Original goal:** deliver issue #29 — every residual must be rejected by an
existing rule or behave correctly, with no crash, leak, or invalid C.

## Overview

PLAN-32 closed #28's seven residuals; PLAN-33 closed #24's arity gap. Their
phase reviews found six adjacent pre-existing shapes. This plan closes each
with the frozen rules and one delegated decision for a shape the spec leaves
open:

1. `import { text as t }` and `std.fmt.text(7)` bypass the intrinsic branch and
   emit an undeclared C symbol.
2. A generic conditional used directly as a condition reports E10012 at its
   null arm where the concrete twin reports E10218 only.
3. `let z = c ? null : null;` is accepted as the null value but its settlement
   is named `Deferred`; the spec does not decide the shape.
4. `null == x` with `x: T` is accepted and emits a pointer/integer comparison
   at `T = i32`.
5. `panic(cstring null)` calls `strlen(NULL)` and segfaults.
6. A for-step fixed-array assignment from a call or member keeps the compact
   header and emits invalid C.

No language, stdlib, or ABI surface changes.

## Minimum-Sufficient Baseline

**Original goal:** make each #29 reproduction correct or rejected by an
existing rule.
**Smallest viable design:** matching alias/qualified predicates in sema and
lowering (the stages own separate scopes) plus a `$text` sentinel
canonicalization; one `condition` flag on the
deferred conditional; one documented and pinned both-null decision; one
deferred null-comparison duty; one null check in three small shim helpers; one
fixed-array predicate in the for-step shape test.
No new diagnostic code, no new runtime helper, no new language surface, no
emission-wide rewrite, no inert settlement variant.
**Excluded machinery:** a general nullable dataflow analysis, a new text
conversion framework, a new loop representation, and any spec/stdlib edit.
**Approved complexity:** None — the plan extends existing checks, duties, and
emitters (AR-1–AR-15).

## Document Index

| # | Document | Description |
|---|----------|-------------|
| AR | [Ambiguity Register](00-ambiguity-register.md) | Zero-Ambiguity Gate decisions |
| 00 | [Index](00-index.md) | This document — overview and navigation |
| 01 | [Requirements](01-requirements.md) | Requirements, scope, acceptance |
| 02 | [Current State](02-current-state.md) | Reproductions and root-cause audit |
| 03 | [Implementation Design](03-implementation.md) | Static, lowering, and runtime changes |
| 07 | [Testing Strategy](07-testing-strategy.md) | Spec cases and verification |
| 99 | [Execution Plan](99-execution-plan.md) | Phases, sessions, and tasks |
| 05 | [Findings](05-findings.md) | Evidence record (created at the first review) |

## Quick Reference

```clorraine
// Accepted: both spellings reach the same converter.
import { text as t } from std.fmt;
println(t(7));
println(std.fmt.text(7));

// Rejected: the intrinsic judgment follows the alias.
t(null);                               // E10500

// Rejected: the generic condition reports the concrete E10218, not E10012.
func show<T>(c: bool, x: T): i32 { if (c ? null : x) { return 1; } return 0; }
show<i32>(false, 1);                   // E10218 only

// Accepted: a both-null conditional is the null value.
let z = c ? null : null;
if (z == null) { }

// Rejected: a null against an open parameter is judged at the instance.
func f<T>(x: T): bool { return null == x; }
f<i32>(1);                             // E10012 at the null

// Fixed: a null cstring panics cleanly.
let p: cstring = null;
panic(p);                              // panic: <newline>, no UB

// Fixed: the array step uses the flat store.
for (let i: i32 = 0; i < 1; a = mk3(), i = i + 1) { }
```

### Key Decisions

| Decision | Outcome |
|----------|---------|
| Aliased `text` | Resolve the import binding to the intrinsic (AR-1) |
| Qualified `std.fmt.text` | Resolve the path only when no real declaration wins (AR-2) |
| Lowered spelling | Every intrinsic form uses the `$text` sentinel; the name-based arm is removed (AR-3) |
| Generic condition | Deferred `condition` flag; E10218 for a non-bool settlement (AR-4) |
| Both-null conditional | Accepted as the null value; documented and pinned, no dead variant (AR-5 to AR-8) |
| Null vs open parameter | Deferred comparison duty; E10012 for a non-null-capable instance (AR-9, AR-10) |
| Null cstring | `cl_cstring_len(NULL) == 0` in all shims; runtime-null gets O7 (AR-11) |
| For-step fixed array | Force the flat loop (AR-12) |
| Spec/ABI | No language surface or ABI change (AR-13) |

## Specialist Agents

| Role | Kind | Use | AR Ref |
|------|------|-----|--------|
| correctness-reviewer | reviewer | Phase reviews and fix re-review | AR-14 |
| semantics-reviewer | reviewer | Condition parity, null-comparison, both-null decision | AR-4–AR-10 |
| memory-safety-auditor | auditor | Alias text ownership, cstring panic, array-step ASan/UBSan probes | AR-1, AR-11, AR-12 |
| emission-integrity-auditor | auditor | Alias/qualified text emission, step shape, strict C build | AR-1–AR-3, AR-12 |

_Detection evidence: `.opencode/agents/` contains all four briefs; the change
touches sema name resolution and condition records (`compiler/src/sema`), the
deferred duties (`compiler/src/own`), the C panic helper in all three shims,
and the for-step shape predicate._

## Related Files

Created:
- `compiler/tests/template_placeholder_residuals_iii_spec_test.rs`
- `compiler/tests/template_placeholder_residuals_iii_impl_test.rs`
- `plans/template-placeholder-residuals-iii/05-findings.md`

Modified:
- `compiler/src/sema/call.rs` — alias/qualified `text`, shared text-call helper, real-declaration gate
- `compiler/src/sema/ops.rs` — deferred literal-null comparison, condition flag
- `compiler/src/sema/mod.rs` — `OpaqueNullComparison`, `condition` fields, per-module drain
- `compiler/src/types/mod.rs` — both-null settlement documentation
- `compiler/src/own/expr.rs` — alias/qualified intrinsic recognition
- `compiler/src/own/call.rs` — `$text` canonical lowered spelling
- `compiler/src/own/instance_check.rs` — condition and null-comparison duties
- `compiler/src/own/loops.rs` — fixed-array step target
- `compiler/runtime/shim_host.c`, `shim_hosted_amiga.c`, `shim_freestanding.c` — null-safe cstring length
- `plans/00-roadmap.md` — PLAN-34 row

Read-only:
- `spec/` — the frozen oracle (no edit)
- `compiler/tests/template_placeholder_residuals_ii_{spec,impl}_test.rs` — #28 oracle
- `compiler/tests/text_arity_{spec,impl}_test.rs` — #24 oracle
