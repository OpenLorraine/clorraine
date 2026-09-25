# Generic Field Integrity Implementation Plan

> **Feature**: Substitute a struct instance's type arguments into its field types consistently across the checker, ownership, and emitter.
> **Status**: Planning Complete
> **Created**: 2026-09-25
> **Implements**: DEF-06, DEF-07, DEF-08, DEF-09, DEF-10
> **CodeOps Artifact Schema**: 1

## Overview

Reading a field of an instantiated generic struct currently returns the declared
field type without substituting the instance's type arguments. `Pair<i32>.first`
is therefore typed as the bare parameter `T` in every stage that inspects it.
This one gap produces five recorded defects: an overlay (volatile) type slips
through a generic wrapper by value (DEF-06), a generic struct holding a handle is
treated as duplicable and copied (DEF-07), its handle field is never dropped
(DEF-08), a generic zero value emits invalid C (DEF-09), and generic
`sizeof`/`offsetof`, the ownership field read, and raw-pointer member access are
mis-typed (DEF-10).

The fix is the same everywhere: substitute the instance's type arguments into the
declared field type. A single small helper owns that substitution and every
stage routes its field read through it, so the stages cannot drift apart again —
the drift is exactly what caused the five defects. The plan also closes the
related defect discovered earlier and already committed as DEF-05's own follow-up.

## Minimum-Sufficient Baseline

**Original goal:** Make an instantiated generic struct's field types correct in
every compiler stage, so the five recorded defects stop reproducing.
**Smallest viable design:** One pure helper (`types::substitute_instance`) plus
one table convenience (`TypeTable::instance_field`) that substitute an instance's
arguments into a field type, called from the ~6 existing field-read sites. No new
module, dependency, traits, or harness.
**Excluded machinery:** None. No new support surface was considered; a shared
helper is the "small helper" exempt from the complexity-escalation gate (AR-22).
**Approved complexity:** None (AR-22).

## Document Index

| #   | Document                                                              | Description                                 |
| --- | --------------------------------------------------------------------- | ------------------------------------------- |
| AR  | [Ambiguity Register](00-ambiguity-register.md)                        | Zero-Ambiguity Gate decisions (audit trail) |
| 00  | [Index](00-index.md)                                                  | This document — overview and navigation     |
| 01  | [Requirements](01-requirements.md)                                    | Requirements and scope                      |
| 02  | [Current State](02-current-state.md)                                  | Analysis of the current implementation      |
| 03-01 | [Types Substitution Foundation](03-01-types-substitution.md)        | The shared helper and the type-table reads  |
| 03-02 | [Checker Field Reads](03-02-checker.md)                             | Overlay walk and checker field reads        |
| 03-03 | [Ownership and Emission](03-03-ownership-codegen.md)                | Ownership field read and emitter paths      |
| 07  | [Testing Strategy](07-testing-strategy.md)                            | Regression cases and verification           |
| 99  | [Execution Plan](99-execution-plan.md)                                | Phases, sessions, and task checklist        |

## Quick Reference

### Usage Examples

These programs change from mis-handled to correct under the plan:

```clorraine
// DEF-06: an overlay reached through a generic wrapper is rejected by value.
struct Regs { r: volatile u16; }
struct Outer<T> { inner: T; }
func takes(o: Outer<Regs>): void { }   // E10207, like non-generic Outer

// DEF-07 / DEF-08: a generic struct holding a handle is move-only and drops.
struct Box<T> { value: T; }
func demo(): void {
    let a = Box<String> { value: stringOf("x") };
    let b = a;              // okay: a is moved
    // use(a)               // E10020
}                           // emits FreeString(b.value)

// DEF-09: a nested generic zero value emits valid C.
let z = Pair<Pair<i32>> {};        // (ClPair__Pair__i32){ {0}, {0} }
```

### Key Decisions

| Decision | Outcome |
| -------- | ------- |
| Shared substitution | `types::substitute_instance` + `TypeTable::instance_field` (AR-3, AR-6) |
| DEF-09 | Codegen-only; `Point {}` stays valid (AR-2) |
| `offset_of` | Signature takes `&Ty` so it sees instance arguments (AR-7) |
| Tests | Regression tests in existing suites, written red-first; no `spec/` edits (AR-4) |
| Verify | `make check` per task; final `make verify` (AR-5) |

## Related Files

- `compiler/src/types/mod.rs` — new `substitute_instance` free function.
- `compiler/src/types/table.rs` — `instance_field`, `duplicable_at`, `size_of_at`, `offset_of`.
- `compiler/src/sema/mod.rs` — `overlay_reached`.
- `compiler/src/sema/expr.rs`, `compiler/src/sema/call.rs` — field reads already routed through `field_type`.
- `compiler/src/own/expr.rs` — ownership `member_type`.
- `compiler/src/codegen/drop.rs`, `compiler/src/codegen/expr.rs` — drop and struct-literal/place field reads.
- `compiler/src/consts/mod.rs` — `offsetof` caller.
