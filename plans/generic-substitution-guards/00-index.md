# Generic Substitution Guards Implementation Plan

> **Feature**: Audit a program's concrete instantiations so a `str` or slice substituted into a struct field reports `E10205`, and a move-only type substituted into an array fill reports `E10215`, instead of compiling.
> **Status**: Planning Complete
> **Created**: 2026-09-26
> **Implements**: DEF-12
> **CodeOps Artifact Schema**: 1

## Overview

Both declaration-site guards are blind to a type substituted for a type
parameter. `struct Wrap<T> { v: T; }` used as `Wrap<str>` or `Wrap<u8[]>`
compiles although the frozen rule bans a view-typed field
(`spec/02-types.md:184`, TY-10, `E10205`). A generic fill
`func fillIt<T>(a: T): T[2] { return [; a]; }` called as `fillIt<String>(s)`
compiles, emits two aliases of one handle, and double-frees
(AddressSanitizer-confirmed); `spec/08-arrays-strings.md:50` (AL-1) requires a
duplicable fill value, so it is `E10215`. Both guards read the *declared* type,
where the type parameter hides the concrete kind
(`compiler/src/sema/items.rs:39`, `compiler/src/sema/expr.rs:706-708`).

The ownership stage already collects every concrete instantiation of the
program (`Monomorphization`, `compiler/src/ir/mono.rs:106`). The fix adds one
focused audit over that set: a struct instance whose substituted field is `str`
or a slice reports `E10205`, and a function instance whose body repeats a
substituted non-duplicable fill value reports `E10215`. Only a declared type
that mentions a type parameter is audited, so a declaration-site case is never
re-reported (and the type checker already stops the stage before the audit).

The frozen specification already forbids both programs, so this plan changes no
`spec/` chapter, no standard-library source, and no emitted-C contract for a
valid program. It is a correctness fix that closes a double free and a
view-field escape before code generation.

## Minimum-Sufficient Baseline

**Original goal:** reject the concrete instantiation that violates a frozen
rule, while keeping the legal instantiations accepted.
**Smallest viable design:** one new module `compiler/src/own/instance_check.rs`
that reads the already-collected `Monomorphization` and reports through the
existing diagnostics; it reuses `mentions_type_param`
(`compiler/src/own/lower.rs:698`), `substitute_instance`, and
`TypeTable::duplicable` (`compiler/src/types/table.rs:362`). No new pass, layer,
dependency, harness, or generalized rule engine (AR-10).
**Excluded machinery:** a generalized per-instance replay of every checking
rule; transitive function-instance collection; intrinsic-only instance
discovery; the unrecorded-instance gaps (discarded-call result types, IR-only
nested instances, lifted lambda bodies) (AR-5, AR-10, AR-19).
**Approved complexity:** none needed — AR-10 records the no-escalation
classification.

## Document Index

| #    | Document                                                    | Description                                 |
| ---- | ----------------------------------------------------------- | ------------------------------------------- |
| AR   | [Ambiguity Register](00-ambiguity-register.md)              | Zero-Ambiguity Gate decisions (audit trail) |
| 00   | [Index](00-index.md)                                        | This document — overview and navigation     |
| 01   | [Requirements](01-requirements.md)                          | Requirements and scope                      |
| 02   | [Current State](02-current-state.md)                        | Analysis of the current implementation      |
| 03-01 | [Instantiation Checks](03-01-instantiation-checks.md)      | The audit module and its integration        |
| 07   | [Testing Strategy](07-testing-strategy.md)                  | Test cases and verification                 |
| 99   | [Execution Plan](99-execution-plan.md)                      | Phases, sessions, and task checklist        |

## Quick Reference

### Programs that change from accepted to rejected

```clorraine
struct Wrap<T> { v: T; }
let w: Wrap<str> = Wrap<str> { v: "hi" };        // E10205
let b: Wrap<u8[]> = Wrap<u8[]> { v: bytes(s) };  // E10205

func fillIt<T>(a: T): T[2] { return [; a]; }
let copies: String[2] = fillIt<String>(s);       // E10215 (was: double free)
```

### Programs that stay accepted

```clorraine
let a: Wrap<String> = Wrap<String> { v: stringOf("x") };  // owned field
let b: Wrap<static str> = Wrap<static str> { v: "hi" };   // literal view
let c: i32[2] = fillIt<i32>(7);                            // duplicable fill
```

### Key Decisions

| Decision | Outcome |
| -------- | ------- |
| Fill semantics | Audit the concrete instance: reject `fillIt<String>`, keep `fillIt<i32>` (AR-2) |
| View predicate | Mirror the declaration-site check exactly; no deep `str[2]` ban (AR-4) |
| Check placement | New `compiler/src/own/instance_check.rs`, called from `own::check` (AR-6) |
| Duplicate guard | Audit only types that mention a type parameter (AR-7, AR-17) |
| Diagnostics | Reuse the registry message/help; field-declaration and statement spans (AR-3) |
| Out of scope | Transitive generic calls; `sizeof`-only discovery; unrecorded IR-only/lambda instances (AR-5, AR-19) |
| Verify | `make check` per task; final `make verify` (AR-16) |

## Related Files

- `compiler/src/own/instance_check.rs` — new audit module (both checks).
- `compiler/src/own/generics.rs` — shared `struct_declarations` helper.
- `compiler/src/own.rs` — module wiring and the audit call.
- `compiler/src/ir/mono.rs` — `Monomorphization` (the audit input, unchanged).
- `compiler/src/types/table.rs`, `compiler/src/own/lower.rs` — reused helpers.
- `compiler/tests/ownership_drop_spec_test.rs` — specification diagnostics.
- `compiler/tests/ownership_drop_impl_test.rs` — build-and-run controls.
