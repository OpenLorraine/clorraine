# Template Placeholder Residuals Implementation Plan

> **Feature**: Close the five template-placeholder residuals from #26: a null
> literal placeholder, a mixed-nullability conditional, the zero-state String
> handle, a discarded template in a `for` step, and an owning aggregate field
> evaluated after a later placeholder copy.
> **Status**: Planning Complete
> **Created**: 2026-11-11 21:03
> **Implements**: R1–R9
> **CodeOps Artifact Schema**: 1

**Original goal:** deliver issue #27 — make each residual either well-defined
or rejected by an existing rule, with no crash, leak, or use-after-free.

## Overview

Issue #26 fixed the four original template-placeholder defects and their
generic, guarded, loop-condition, and module-initializer variants. Its phase
reviews found five pre-existing residuals that reproduce on `b6f12d8`:

1. `$"{null}"` emits `cl_text_unknown(NULL)` and fails the C build.
2. A conditional of `String` and `String?` is typed non-null and crashes when
   the null arm is selected.
3. A zero-state `String` handle from `H {}` reaches `cl_string_copy(NULL)`.
4. A discarded template statement (including a `for` step) leaks its result.
5. An owning aggregate literal field before a later placeholder evaluates after
   the copy (`seq=21`).

The fixes are small and reuse existing rules: the formattability rule rejects a
null-shaped placeholder; `conditional_result` keeps the nullable flag; the
String accessors treat the zero state as the empty String; discarded owned
templates bind for release and the `for` step drains; `Statement::Adopt` gains
a no-pre-drop structured store for a pinned owning aggregate. No language or
ABI surface changes.

## Minimum-Sufficient Baseline

**Original goal:** make the five #27 reproductions correct or rejected with the
existing rules.
**Smallest viable design:** one sema formattability check; one nullability OR
in `conditional_result`; null checks in three runtime helpers; one discard
binding plus one release-floor drain; one codegen arm for an adopted `StructLit`.
No new IR value kind, no new diagnostic code, no new runtime helper, no new
language surface.
**Excluded machinery:** a general null-flow analysis, a zero-state sentinel
allocation, a new statement variant, a new formatting framework, and any change
to `text(null)` or the other deferred issues.
**Approved complexity:** None — the plan extends existing checks and emitters
(AR-1–AR-24).

## Document Index

| # | Document | Description |
|---|----------|-------------|
| AR | [Ambiguity Register](00-ambiguity-register.md) | Zero-Ambiguity Gate decisions |
| 00 | [Index](00-index.md) | This document — overview and navigation |
| 01 | [Requirements](01-requirements.md) | Requirements, scope, acceptance |
| 02 | [Current State](02-current-state.md) | Reproductions and root-cause audit |
| 03-01 | [Type Diagnostics](03-01-type-diagnostics.md) | Null literal and conditional nullability |
| 03-02 | [Lowering and Runtime](03-02-lowering-and-runtime.md) | Zero state, discard cleanup, aggregate Adopt |
| 07 | [Testing Strategy](07-testing-strategy.md) | Spec cases and verification |
| 99 | [Execution Plan](99-execution-plan.md) | Phases, sessions, and tasks |
| 05 | [Findings](05-findings.md) | Evidence record (created at the first review) |

## Quick Reference

### Usage Examples

```clorraine
// Rejected: a null-shaped placeholder has no formattable type (E10500).
println($"{null}");

// Rejected: the nullable arm is not narrowed (E10010).
let flag: bool = false;
let a: String = stringOf("alpha");
let b: String? = null;
println($"{flag ? a : b}");

// Accepted: the zero state is the empty String, so this prints an empty line.
struct H { name: String; }
let h: H = H {};
println($"{h.name}");
println(h.name);

// Accepted: the discarded template result is released, including in a step.
for (let i: i32 = 0; i < 3; $"{i}", i = i + 1) { println($"{i}"); }

// Accepted in source order: inner's mkString runs before the later copy.
let p: P = P { inner: Inner { a: mkString() }, b: $"{mark(2)}" };
```

### Key Decisions

| Decision | Outcome |
|----------|---------|
| Null literal placeholder | E10500 (AR-1) |
| Null arm with a handle sibling | `String?`; E10010 at the null arm (AR-23) |
| Mixed conditional | Preserve nullable from non-null-shaped arms; E10010 at an unnarrowed arm (AR-2) |
| Zero-state String | Null-safe accessors; empty String (AR-3) |
| Discarded template / step | Bind the owner and drain per step (AR-4) |
| Owning aggregate order | Adopt with a zero-fill and a fieldwise store (AR-5, AR-24) |
| `text(null)` | Out of scope, follow-up (AR-10) |
| Spec/ABI | No language surface or ABI change (AR-13) |

## Specialist Agents

| Role | Kind | Use | AR Ref |
|------|------|-----|--------|
| correctness-reviewer | reviewer | Phase reviews and fix re-review | AR-15 |
| semantics-reviewer | reviewer | Nullability, zero state, source order | AR-1, AR-2, AR-5 |
| memory-safety-auditor | auditor | ASan probes for zero state, discarded templates, aggregate loops | AR-3, AR-4, AR-5 |
| emission-integrity-auditor | auditor | Adopt structured store, host/Amiga C build | AR-5, AR-6 |

_Detection evidence: `.opencode/agents/` contains all four role briefs; the
change touches sema nullability (`compiler/src/types`, `compiler/src/sema`),
ownership lowering (`compiler/src/own`), emission (`compiler/src/codegen`), and
the host/Amiga runtime shims._

## Related Files

Created:
- `compiler/tests/template_placeholder_residuals_spec_test.rs`
- `compiler/tests/template_placeholder_residuals_impl_test.rs`
- `plans/template-placeholder-residuals/05-findings.md`

Modified:
- `compiler/src/types/mod.rs` — conditional nullability
- `compiler/src/sema/expr.rs` — null-shaped placeholder rejection
- `compiler/src/own/lower.rs` — discarded template binding, aggregate pin
- `compiler/src/own/loops.rs` — per-step release drain
- `compiler/src/codegen/function.rs` — adopted `StructLit` zero-fill and fieldwise store
- `compiler/runtime/shim_host.c`, `shim_hosted_amiga.c` — null-safe String accessors
- `plans/00-roadmap.md` — PLAN-31 row

Read-only:
- `spec/` — the frozen oracle (no edit)
- `compiler/tests/template_placeholder_integrity_{spec,impl}_test.rs` — #26
  oracle that must stay green
