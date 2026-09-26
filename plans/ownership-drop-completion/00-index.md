# Ownership and Drop Model Completion Implementation Plan

> **Feature**: Make a fixed array an owned aggregate rather than a view, give fixed and growable arrays owning-element drop glue, and close the view-typed-field check.
> **Status**: Planning Complete
> **Created**: 2026-09-26 14:03
> **Implements**: PLAN-11
> **CodeOps Artifact Schema**: 1

## Overview

Version 1 of the compiler classifies every array type as a *view*. That is
correct for a slice `T[]`, which is a pointer and a length, but wrong for a
fixed array `T[N]`, which is an owned aggregate that lives where its binding
lives. The mistake has three consequences. A fixed array of move-only elements
copies its elements into the array and then releases the originals, leaving two
aliases of one handle; the array itself is never dropped. A fixed-array return
is rejected as a view return because the elision rule sees a view with no
candidate. And a slice view field is accepted in a struct even though the
frozen chapters ban view-typed fields.

The plan fixes this in three parts. First, one predicate decides whether an
array type is a view, and every stage uses it, so the fixed array and the slice
can no longer be confused. Second, the emitter gains owning-element drop glue
for fixed arrays and for the growable `Array<T>`, which un-stages the
`Array<move-only>` error. Third, the view-field check rejects `str` and slice
fields while keeping the fields the chapters allow. Each part is a small change
in code the compiler already has; no new layer, dependency, or harness is
introduced.

## Minimum-Sufficient Baseline

**Original goal:** Make the version-1 implementation match the frozen ownership
and drop rules for arrays and for view-typed fields.
**Smallest viable design:** One type predicate (`Ty::is_slice`) plus routed
consumers, the existing "copy then empty" move path applied to array-literal
elements, an array arm in the existing drop ladder, and one extra branch in the
existing struct-field check. No new module, trait, dependency, or harness.
**Excluded machinery:** None considered. The growable-array drop loop reuses the
existing `cl_buffer_ptr` FFI and the existing inline drop-ladder pattern, so it
is ordinary feature code, not a support surface (AR-21).
**Approved complexity:** None (AR-21).

## Document Index

| #   | Document                                                              | Description                                 |
| --- | --------------------------------------------------------------------- | ------------------------------------------- |
| AR  | [Ambiguity Register](00-ambiguity-register.md)                        | Zero-Ambiguity Gate decisions (audit trail) |
| 00  | [Index](00-index.md)                                                  | This document — overview and navigation     |
| 01  | [Requirements](01-requirements.md)                                    | Feature requirements and scope              |
| 02  | [Current State](02-current-state.md)                                  | Analysis of the current implementation      |
| 03-01 | [Array Classification](03-01-array-classification.md)              | The view/owned predicate and its consumers  |
| 03-02 | [Array Element Drop Glue](03-02-array-drop-glue.md)                | Fixed and growable array cleanup            |
| 03-03 | [View-Field Enforcement](03-03-view-field-enforcement.md)          | The `E10205` and `E10215` checks            |
| 07  | [Testing Strategy](07-testing-strategy.md)                            | Test cases and verification                 |
| 99  | [Execution Plan](99-execution-plan.md)                                | Phases, sessions, and task checklist        |

## Quick Reference

### Usage Examples

Programs that change from rejected or unsound to correct under the plan:

```clorraine
import { String, stringOf } from std.text;

// F1(a): a fixed-array return is a by-value return, not a view return.
func pair(): String[2] {
    let a: String = stringOf("a");
    let b: String = stringOf("b");
    return [a, b];              // both handles move into the array
}                               // elements drop first-to-last, then the array

// F1(b): a fixed array is owned, so its elements move.
func demo(): void {
    let a: String = stringOf("a");
    let b: String = stringOf("b");
    let arr: String[2] = [a, b];
    // consume(take a);         // E10020: a was moved into the array
}

// F3: a growable Array of a move-only element is accepted and drops.
func grow(): void {
    let xs: Array<String> = arrayNew<String>();
    xs.push(stringOf("x"));     // no E90000
}                               // elements drop first-to-last, then the buffer

// F2: a slice view field is rejected.
// struct Bad { xs: u8[]; }   // E10205
struct Good { name: String; label: static str; path: cstring; row: u8[8]; }
```

### Key Decisions

| Decision | Outcome |
| -------- | ------- |
| Array shape predicate | `Ty::is_slice`, used by every consumer (AR-5) |
| Element moves | Array-literal elements consume like struct-literal fields (AR-6) |
| Drop order | Elements first-to-last; growable also frees the buffer (AR-7) |
| Growable drop lowering | Inline loop over `cl_buffer_ptr` in the drop ladder (AR-8) |
| `E90000` | Un-staged; the code stays a dormant fallback (AR-2, AR-3) |
| View fields | Reject `str` and slices; keep `cstring`/`static str` (AR-4) |
| Fill of move-only | Rejected under `E10215` (AR-10) |
| Tests / verify | Rust suites + build-and-run oracle; `make check`, `make verify` (AR-12, AR-13) |

## Related Files

- `compiler/src/types/mod.rs` — `Ty::is_slice` and `Ty::is_fixed_array`.
- `compiler/src/types/table.rs` — `duplicable_at` slice handling.
- `compiler/src/borrow.rs` — `is_view`, `is_elided_view`, `is_lend_candidate`.
- `compiler/src/own/expr.rs`, `compiler/src/own/lower.rs` — array-literal element consumption.
- `compiler/src/codegen/drop.rs` — array drop glue.
- `compiler/src/sema/mod.rs` — remove the `E90000` trigger.
- `compiler/src/sema/items.rs` — the `E10205` view-field check.
- `compiler/src/sema/expr.rs` — the `E10215` fill check.
- `spec/08-arrays-strings.md`, `spec/14-diagnostics.md` — `AL` rule and `E10215` text.
- `spec/tests/staged/E90000_generic_staged.clo`, `spec/tests/staged/README.md` — un-staging.
