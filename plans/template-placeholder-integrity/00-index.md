# Template Placeholder Integrity Implementation Plan

> **Feature**: Make every template placeholder well-defined or rejected: a
> nullable placeholder is E10010, a non-formattable generic instance is
> E10500, each placeholder evaluates once at its source position, and a
> string placeholder's bytes are copied before a later sibling can move or
> release its source.
> **Status**: Planning Complete
> **Created**: 2026-11-11 14:08
> **Implements**: R1–R8
> **CodeOps Artifact Schema**: 1

**Original goal:** deliver issue #26 — four pre-existing template-placeholder
defects: a null `String?` placeholder segfaults, a `str`-returning call is
evaluated twice, a `take` sibling crashes an earlier placeholder, and a `str`
view is used after its owner was moved.

## Overview

Issue #26 records four defects that reproduce on the base compiler before the
direct-template-formatting work and still reproduce on `8b4bff2`. The defects
come from two places: the function-body template walk does not apply the
existing nullable-use rule, and the lowering embeds placeholder values in the
builder call while a later placeholder's `take` argument is hoisted into a
statement ahead of it.

The plan also covers three same-root variants found while grounding the
issue: generic type-parameter placeholders (`show<String>(s, s)` and
`show<String?>(null)`), guarded positions (`?:`, `&&`, `||`, loop conditions),
and module initializers (`let s: str = $"v={mk()}";`). Each reaches the same
unsafe path. A generic placeholder whose instantiation is not formattable is
rejected at the instance with E10500 instead of emitting a nonexistent
converter.

The fix reuses the existing converters and temporary materialization. A
string-like placeholder is lowered to an internal text-conversion call whose
result is bound at the placeholder's source position; the template part adopts
that owned copy. Pure scalar placeholders stay inline unless a later
placeholder can reorder them. The nullable rule reuses E10010 and the check
the module-initializer path already performs.

## Minimum-Sufficient Baseline

**Original goal:** make the four #26 reproductions correct or rejected, with
the same guarantee for the generic, guarded, and module-initializer variants.
**Smallest viable design:** add the existing nullable check to the function
template walk; lower each string-like placeholder through one existing
converter call into a compiler temporary at its source position; keep scalar
placeholders inline unless ordering requires a temporary. No new IR variant,
no new shim function, no language or ABI change.
**Excluded machinery:** a new formatting framework, a new `ValueKind` variant,
a new runtime helper, a general evaluation-order pass, an instance-specialized
generic fast path, and any change to #24 or the other deferred issues.
**Approved complexity:** None — the plan extends existing lowering and guard
patterns (AR-8, AR-9, AR-10, AR-11).

## Document Index

| # | Document | Description |
|---|----------|-------------|
| AR | [Ambiguity Register](00-ambiguity-register.md) | Zero-Ambiguity Gate decisions |
| 00 | [Index](00-index.md) | This document — overview and navigation |
| 01 | [Requirements](01-requirements.md) | Requirements, scope, acceptance |
| 02 | [Current State](02-current-state.md) | Reproductions and lowering audit |
| 03-01 | [Nullable and Generic Placeholders](03-01-nullable-placeholders.md) | The E10010/E10500 rules |
| 03-02 | [Ordered Placeholder Evaluation](03-02-ordered-evaluation.md) | Technical specification |
| 07 | [Testing Strategy](07-testing-strategy.md) | Spec cases and verification |
| 99 | [Execution Plan](99-execution-plan.md) | Phases, sessions, and tasks |
| 05 | [Findings](05-findings.md) | Evidence record (created at the first review, extended at closeout) |

## Quick Reference

### Usage Examples

```clorraine
// Rejected: an unnarrowed nullable placeholder is E10010.
let s: String? = null;
println($"{s}");

// Accepted: a narrowed nullable placeholder formats the value.
if (s != null) { println($"{s}"); }

// Accepted: the call runs once, so the line prints `x calls=1`.
println($"{mk()} calls={calls}");

// Accepted: the placeholder copy precedes consume's move, printing `alpha5`.
let s: String = stringOf("alpha");
println($"{s}{consume(s)}");

// Rejected: the view is read after the move (E10022).
let v: str = s;
println($"{consume(s)}{v}");
```

### Key Decisions

| Decision | Outcome |
|----------|---------|
| Nullable placeholder | E10010, narrowed values format (AR-1, AR-2) |
| `str` call | Evaluated once at its source position (AR-3) |
| Copy before move | Copy at the placeholder position; the reverse order stays E10022 (AR-4–AR-6) |
| Generic placeholders | Same rule per instantiation; nullable instances judged by a duty (AR-7) |
| Mechanism | Internal `$text` copy call through the existing converter; no new IR or shim (AR-8) |
| Scalar order | Materialize only with effects or before a later effect (AR-9) |
| Module initializers | Once-only evaluation (AR-10) |
| Guarded positions | Fill through the existing deferred `Materialize` machinery (AR-11) |

## Specialist Agents

| Role | Kind | Use | AR Ref |
|------|------|-----|--------|
| correctness-reviewer | reviewer | Both phase reviews and fix re-review | AR-16 |
| semantics-reviewer | reviewer | Nullability and left-to-right evaluation rules | AR-1, AR-3, AR-5 |
| memory-safety-auditor | auditor | ASan probes for the copy/move and guarded cases | AR-4, AR-11 |
| emission-integrity-auditor | auditor | Base/head emission diff and strict C build | AR-8, AR-9, AR-12 |

_Detection evidence: `.opencode/agents/` contains all four role briefs; the
change touches ownership/null checks (`compiler/src/own/expr.rs`), lowering
(`compiler/src/own/lower.rs`), and emission (`compiler/src/codegen/expr.rs`,
`compiler/src/codegen/program.rs`), so AGENTS.md requires the full review
surface._

## Related Files

Created:
- `compiler/tests/template_placeholder_integrity_spec_test.rs`
- `compiler/tests/template_placeholder_integrity_impl_test.rs`
- `plans/template-placeholder-integrity/05-findings.md` (created at the first
  phase review and extended at closeout)

Modified:
- `compiler/src/ir.rs` — `ValuePiece::Placeholder` carries the owned-copy
  provenance flag; re-export the new duty
- `compiler/src/ir/mono.rs` — the `RequiresTemplatePlaceholder` duty
- `compiler/src/own/expr.rs` — nullable check and the generic duty record
- `compiler/src/own/lower.rs` — ordered placeholder lowering
- `compiler/src/own/instance_check.rs` — the per-instance E10500/E10010 check
- `compiler/src/own.rs` — the duty field, dedupe set, and transfer
- `compiler/src/own/call.rs` — expose the effect predicate to the sibling
  lowering module
- `compiler/src/sema/expr.rs` — expose the formattability predicate and reject
  an opaque placeholder
- `compiler/src/sema/mod.rs` — re-export `formattable` for the instance check
- `compiler/src/codegen/expr.rs` — internal copy-call rendering; adopt only the
  owned part
- `compiler/src/codegen/program.rs` — once-only module-initializer placeholders
- the `ValuePiece` walk sites — carry the owned flag unchanged
- `compiler/tests/template_direct_formatting_impl_test.rs` — update the
  intended emission pins
- `plans/00-roadmap.md` — PLAN-30 row

Read-only:
- `spec/` — the frozen oracle (no edit)
- `compiler/runtime/shim_host.c`, `shim_hosted_amiga.c` — existing converters
  and part builder (no edit)
