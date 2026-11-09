# Module Initializer Integrity Implementation Plan

> **Feature**: Close 9 defects in the module-initializer pipeline: callable resolution (lambdas,
> func-typed globals, dot-call receivers, generic instances), aggregate emission (struct stores,
> `str` view wraps, `len` folds), the missing release of owning init temporaries, the missing
> write-mutability checks over initializer expressions, and one body-side slice-typedef gap.
> **Status**: Done — plan and roadmap rows complete (2026-11-09 closeout).
> **Created**: 2026-10-11 02:10
> **Implements**: DEF-130, DEF-131, DEF-132, DEF-133, DEF-134, DEF-135, DEF-137, DEF-143, DEF-152
> **CodeOps Artifact Schema**: 1

## Overview

This is batch F of the user-mandated defect program: the 9 module-initializer defects (7 major,
2 minor) that the evaluation/emission-integrity reviews filed with reproductions. They share one
root: **module initializers are lowered by a second, diminished path**. The frozen model
(`spec/10-modules.md` MOD-20) collects initializers into generated `cl_init_<module>` functions,
but the compiler carries the initializer *AST* straight into codegen
(`codegen/init.rs:69-120`), where `lower_global` (`codegen/program.rs:1560-1691`) re-implements a
smaller copy of the body lowerer. It resolves no lifted lambdas, records no generic instances,
passes no dot-call receiver, renders struct literals as compound literals that C rejects for
array fields, misses the `str` view wrap, folds a qualified `.len` on the head type only — and
the surrounding emitter has no cleanup section and the own stage never visits initializer
expressions, so owned temporaries leak and const writes go unchecked. One defect (DEF-137) is the
body-side twin of the initializer's slice-typedef collection: a written `i32[]` type argument in
a function body misses its typedef.

Every fix is conformance: no new language surface, no new diagnostic code, no specification text
change. The designs mirror the verified body-path contracts into the init path, extend the
existing AST-level generics walk to initializer expressions, and add two focused own-stage
pieces (a capture-less lambda lift for init callees; a check-only write walk reusing the
extracted mutability predicates).

Execution runs under `--auto-commit --auto-design` (AR-13-era standing mandate; this plan's
AR-16). After this batch closes, batch G (checker conformance) continues the same loop until no
defect row remains open.

## Minimum-Sufficient Baseline

**Original goal:** eliminate the 9 defects' observable failures — invalid C, unresolved symbols,
silently-wrong values, leaks, and unchecked writes — with accepted programs staying accepted and
unaffected initializers re-emitting byte-identically.
**Smallest viable design:** extend the existing codegen init path with the body path's verified
contracts (shape-gated), extend `generics::collect` to initializer expressions, lift capture-less
init lambdas through a synthetic minimal checker context, emit null-guarded releases for owning
init locals at the init function's end, and run a check-only mutability walk over initializers.
**Excluded machinery:** Design A — synthesizing per-module init functions in the own stage so one
pipeline lowers bodies and initializers alike (`codegen/program.rs:756` replaced by IR emission).
It would fix the class by construction but changes the emitted C of every program with a
runtime-initialized global, a cross-cutting refactor outside the mandate's authority (AR-2); it
remains a candidate for a future batch if drift recurs.
**Approved complexity:** None — no new layer, dependency, or harness (AR-16).

## Document Index

| #   | Document                                                                   | Description                                 |
| --- | -------------------------------------------------------------------------- | ------------------------------------------- |
| AR  | [Ambiguity Register](00-ambiguity-register.md)                             | Zero-Ambiguity Gate decisions (audit trail) |
| 00  | [Index](00-index.md)                                                       | This document — overview and navigation     |
| 01  | [Requirements](01-requirements.md)                                         | Requirements and scope                      |
| 02  | [Current State](02-current-state.md)                                       | Analysis of the current pipeline            |
| 03-01 | [Initializer Call Resolution](03-01-initializer-call-resolution.md)      | DEF-131/132/133 technical specification     |
| 03-02 | [Initializer Aggregate Emission](03-02-initializer-aggregate-emission.md)| DEF-134/135/143 technical specification     |
| 03-03 | [Initializer Release and Checks](03-03-initializer-release-and-checks.md)| DEF-130/152 technical specification         |
| 03-04 | [Body Slice Typedefs](03-04-body-slice-typedefs.md)                      | DEF-137 technical specification             |
| 07  | [Testing Strategy](07-testing-strategy.md)                                 | Test cases and verification                 |
| 99  | [Execution Plan](99-execution-plan.md)                                     | Phases, sessions, and task checklist        |

## Quick Reference

### Usage Examples

```clorraine
// DEF-131: a capture-less lambda callee lifts like a body lambda
let g: i32 = (() => mk())().count;

// DEF-132: an initializer generic call records its instance
let base: i32 = pick<i32>(1, 2);

// DEF-133: a callable-field call passes its receiver
struct Box { get: func(): i32; }
let b: Box = Box { get: ready };
let v: i32 = b.get();

// DEF-134: a fixed-array field from a place stores with memcpy
let garr: i32[2] = [1, 2];
let boxed: BoxA = BoxA { arr: garr };

// DEF-135: a str-valued template wraps its concat
let label: str = "n=${n}";

// DEF-143: a qualified .len folds to the extent
let n: usize = b.arr.len();

// DEF-152: const writes are rejected like the body spelling
const G: i32 = 0;
let h: i32 = i32(G++);            // E10023
```

### Key Decisions

| Decision                                                    | Outcome                                              |
| ----------------------------------------------------------- | ---------------------------------------------------- |
| Pipeline strategy (AR-2)                                    | Extend the init path; no unification                |
| Init lambda callees (AR-3)                                  | Lift capture-less lambdas                            |
| Initializer write checks (AR-4)                             | Run the existing mutability rules                    |
| Unaffected initializer emission (AR-13)                     | Byte-identical; shape-gated changes only             |
| New diagnostics (AR-12)                                     | None                                                 |

## Specialist Agents

| Role | Kind | Use | AR Ref |
| ---- | ---- | --- | ------ |
| correctness-reviewer | reviewer | every phase review | AR-15 |
| emission-integrity-auditor | reviewer | phases 1–4 (init emission, symbol resolution) | AR-15 |
| memory-safety-auditor | reviewer | phase 3 (releases) and any ownership-touching diff | AR-15 |
| semantics-reviewer | reviewer | phase 3 (write checks) and language-visible changes | AR-15 |

_Detection evidence: the diff surface is compiler codegen (`codegen/program.rs`,
`codegen/function.rs`), the own-stage generics walk (`own/generics.rs`), the write predicates
(`own/expr.rs`, `own/place.rs`), and the slice-typedef collector (`codegen/types.rs`); the
project already staffs all four roles (`.opencode/agents/` commit set). No new capability gap
found beyond them._

## Related Files

- `compiler/src/codegen/program.rs` — init emission, `lower_global`, init intrinsics, slice typedefs
- `compiler/src/codegen/init.rs` — init order and the `(name, expr)` list
- `compiler/src/codegen/function.rs` — the body contracts to mirror (stores, view wrap, drops)
- `compiler/src/codegen/expr.rs` — the value renderer (`struct_literal`, projections)
- `compiler/src/own/generics.rs` — the AST walk for instances
- `compiler/src/own/lower.rs` — `lower_lambda` / `lift_lambda`
- `compiler/src/own/expr.rs`, `compiler/src/own/place.rs` — mutability predicates
- `compiler/src/codegen/types.rs` — `collect_slice_types`
- `compiler/tests/module_initializer_integrity_{spec,impl}_test.rs` — new suites
- `AGENTS.md` — test naming map (close-out)
