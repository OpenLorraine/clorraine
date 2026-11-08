# v1.1 Language Surfaces (PLAN-19)

> **Feature**: Add the three v1.1 language surfaces: built-in `Box<T>` with
> generated drop glue, ordinary Clorraine functions as `freedBy` deleters
> (the X1 amendment), and C-hook trampolines for capturing lambdas (L5).
> **Status**: RFC — requirements and plan in progress
> **Created**: 2026-11-08
> **Last Updated**: 2026-11-08
> **Implements**: R1-R13
> **CodeOps Artifact Schema**: 1

**Original goal:** close the three v1.1 language surfaces that the frozen
specification stages in `spec/appendix-stdlib.md` section 12. The work must
settle every semantic question the goal names, prove each surface with
specification and implementation tests, keep emitted C valid C99 on host,
m68k GCC, vbcc, and freestanding, and record each approved specification
amendment in the decision ledger.

## Overview

The three surfaces are independent deliverables with one shared theme: a
value that owns storage must be able to release it recursively and safely
across the C boundary.

- `Box<T>` gives the language a safe owned pointer with generated drop glue.
  The boxed storage is allocated from the profile allocator, moved into, and
  released recursively; a box of a box, a box of a struct with owned fields,
  and a box of a growable array all drop exactly once.
- The X1 amendment lets a `freedBy` deleter be an ordinary Clorraine function
  with the existing `take T -> void` signature, not only an `ffi`-declared
  function. This is what lets the standard library declare `Box`'s storage
  deleter in Clorraine and what lets application types own a recursive
  resource without writing an `ffi` block.
- L5 lets a capturing lambda cross a synchronous C hook. The compiler emits a
  per-argument trampoline and a caller-stack environment; the C callback sees
  a plain function pointer, so the C ABI does not change. The environment is
  found through a compiler-emitted linked stack, which makes nesting,
  reentrancy, and multiple callback arguments work without a heap.

The v1.0 release is the baseline. The frozen specification keeps its existing
rules except for the three approved amendments; no golden input changes.

## Minimum-Sufficient Baseline

**Smallest viable design:**

- `Box<T>` is a compiler-known generic struct in `std.mem` with one private
  `BoxStorage` handle field. `BoxStorage` is an ordinary handle whose deleter
  `boxFree` is an ordinary Clorraine function, which exercises the X1
  amendment. The emitter special-cases the `Box` struct: it drops the pointee
  through the existing recursive drop walk, then releases the storage through
  the ordinary deleter. `boxNew`, `boxSet`, and `boxWith` are the whole v1.1
  API. `Box<T>?` is rejected by the existing nullable rule; allocation failure
  is the existing allocator trap.
- The `freedBy` check accepts an ordinary module or exported function with
  the existing `take T -> void` signature. The emitter resolves both user and
  `ffi` deleters, including qualified names. Generic instances are covered by
  `Box<T>`'s generated per-instantiation drop, which calls the ordinary
  deleter.
- A `borrow`-mode `func(...)` parameter on an `ffi` declaration is the
  synchronous-callback contract. A capturing lambda passed to it is lowered
  to a static trampoline plus a caller-stack environment; the call is wrapped
  in a generated C function that pushes and pops the environment around the
  call. Capture-less lambdas stay plain function pointers and take no
  trampoline, no environment, and no push/pop.

**Excluded machinery:** a new `TyKind` variant for boxes, generic handles,
generic user `freedBy` declarations, heap closure environments, a thread-local
or global singleton environment, `stores`/`transfers` contracts, a new
allocator, a freestanding allocator, and any change to the v2 scope. The
generic-aggregate FFI ABI is not generalised; a `Box<T>` in an `ffi` signature
is rejected with a new registered diagnostic instead of emitting the known
invalid generic-struct prototype.

**Approved complexity:** one standard-library module edit, one X1 check
change plus a module-scoped deleter identity, one emitter drop special case
and `emits_drop` arm, one FFI-call contract marker and signature restriction,
one trampoline/wrapper emission path, one runtime fault code (`E11215`), one
new FFI diagnostic (`E10105`), one view diagnostic (`E10227`), a small
type-only resolver predeclaration plus reserved-type-name check, three new
specification golden files for the approved diagnostics, two new Rust test
suites plus focused additions to existing suites, one test-only C fixture for
host and m68k, and specification and documentation amendments.

## Document Index

| # | Document | Description |
|---|---|---|
| AR | [Ambiguity Register](00-ambiguity-register.md) | Every material decision and its resolution |
| 00 | [Index](00-index.md) | This document |
| 01 | [Requirements](01-requirements.md) | Functional, verification, and acceptance requirements |
| 02 | [Current State](02-current-state.md) | Starting behavior, red evidence, and root causes |
| 03-01 | [Box Design](03-01-box.md) | Representation, allocation, drop glue, API, and limits |
| 03-02 | [FreedBy Design](03-02-freedby.md) | Ordinary deleters, resolution, emission, and diagnostics |
| 03-03 | [Hook Trampolines](03-03-hook-trampolines.md) | Contract marker, ABI, environment stack, and lowering |
| 07 | [Testing Strategy](07-testing-strategy.md) | Spec, impl, sanitizer, host, and m68k evidence |
| 00-PF-REQ | [Requirements Preflight](00-preflight-report-requirements.md) | Independent requirements audit and resolution |
| 00-PF-PLAN | [Plan Preflight](00-preflight-report.md) | Independent plan audit and resolution |
| 99 | [Execution Plan](99-execution-plan.md) | Task-by-task delivery and evidence |

## Specialist Agents

**None new.** The repository's existing correctness, memory-safety,
emission-integrity, and semantics reviewers cover the change surface. The
work is compiler ownership/codegen plus a C callback ABI; it is not a new
external system or regulated domain.

_Detection evidence:_ the change touches ownership, drop, FFI ABI, and
codegen, which the four existing project reviewer roles already own. The
negative check is recorded in `05-findings.md` when the plan completes.

## Related Files

| Path | Role |
|---|---|
| `stdlib/std.mem.clo` | `Box<T>`, `BoxStorage`, `boxFree`, and the v1.1 API |
| `compiler/src/ffi.rs` | `freedBy` signature validation and FFI contract checks |
| `compiler/src/own/program.rs` | Deleter and function lookup facts |
| `compiler/src/codegen/drop.rs` | Recursive drop emission and deleter symbols |
| `compiler/src/codegen/program.rs` | Box instance emission, trampolines, and hook wrappers |
| `compiler/src/sema/call.rs` | FFI argument contract and hook-site recording |
| `compiler/src/sema/expr.rs` | Capturing-lambda conversion rule |
| `compiler/src/own/lower.rs` | Hook-call lowering and capture materialization |
| `compiler/src/abi.rs` | FFI type mapping |
| `compiler/runtime/shim_host.c`, `shim_hosted_amiga.c` | `cl_box_alloc`/`cl_box_free` |
| `compiler/tests/v11_language_surfaces_spec_test.rs` | Behavior oracle |
| `compiler/tests/v11_language_surfaces_impl_test.rs` | Emission and ABI pins |
| `compiler/tests/fixtures/` | Host and m68k C hook fixtures |
| `spec/decisions.md`, `spec/06-functions.md`, `spec/07-structs.md`, `spec/12-ffi-contracts.md`, `spec/14-diagnostics.md`, `spec/17-codegen.md`, `spec/appendix-stdlib.md` | Approved amendments |
| `plans/00-roadmap.md` | PLAN-19 row |

## Scope Boundary

This feature adds the three v1.1 surfaces and the diagnostics and tests they
need. It does not change:

- the v2 surface: pools/arenas, `stores`/`transfers`, `comptime`, nested
  templates, varargs wrappers;
- DX, installer, LSP, ARM64, LorraineOS, or performance tuning;
- unrelated ownership, borrow, evaluation-order, or emission rules;
- the C ABI of a capture-less function value, an ordinary handle, or an
  existing `ffi` declaration;
- any `spec/tests` golden or frozen corpus input.

The only accepted-source behavior changes are:

- an ordinary Clorraine function is accepted as a `freedBy` deleter;
- `Box<T>` and its API exist;
- a capturing lambda is accepted at a `borrow func` `ffi` parameter and
  rejected everywhere else, including the previously accepted (and invalid-C)
  unmarked `ffi` parameter;
- a `Box<T>` or another generic aggregate in an `ffi` signature is rejected
  instead of emitting invalid C.
