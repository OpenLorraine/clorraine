# Callable Release Integrity Implementation Plan

> **Feature**: Close 13 defects in the callable, identity, and release layers: the `__cl_owned0`
> name-collision double-free, the missing effect modes for func-typed fields/globals/params/lambda
> parameters, five owned-release holes, the func-signature type key, the duplicate `ClEnv_0`
> definition, and the null-`String` widening segfault.
> **Status**: Planning Complete
> **Created**: 2026-10-10 19:53
> **Implements**: DEF-138, DEF-139, DEF-140, DEF-141, DEF-155, DEF-156, DEF-157, DEF-158,
> DEF-159, DEF-163, DEF-164, DEF-165, DEF-166
> **CodeOps Artifact Schema**: 1

## Overview

This is batch E of the user-mandated defect program: the 13 memory-safety and callable-identity
defects (1 critical, 11 major, 1 minor) that the evaluation/emission-integrity reviews filed with
base-identical reproductions. The defects share two roots. First, **generated identities are keyed
by names and collapsed by keys**: the ownership layer's `__cl_owned{N}` temporaries collide with
source locals that spell the same name (a double-free), the `func` type key ignores its signature
(one C struct for two instances), and an inline capturing lambda emits its environment struct twice.
Second, **a callable's own signature does not reach the call it governs**: func-typed fields,
module-level func-typed globals, `Param`-typed callees, and lambda parameters all lose their
parameter modes, so owned arguments leak or double-free, and one shape reports a false `E10020`
against a shadowing binding. Alongside them, five owned-release holes and one possibly-null
widening segfault close.

Every fix is conformance: no new language surface, no new diagnostic code, no specification text
change (AR-11). The designs extend existing machinery — the owner binder, the mode propagation the
value-call fix already built, the type-key renderer, the cleanup edges, and the E10036 view-return
postlude. The one new rejection (a fixed array of view elements returned from a temporary) reuses
the diagnostic family the single-view spelling already reports (AR-8); the one user-decided
behavior (a possibly-null `String` widening reports `E10010`) follows the frozen narrowing rule
(AR-9).

Execution runs under `--auto-commit --auto-design` (AR-13); eligible mechanism decisions are
delegated with provenance in the register (PD-1 – PD-4). After this batch closes, batch F
(module-initializer integrity) and batch G (checker conformance) continue the same loop until no
defect row remains open.

## Minimum-Sufficient Baseline

**Original goal:** eliminate the 13 filed defects' observable failures — no double-free, no leak,
no segfault, no invalid C, no silently-wrong value — with accepted programs staying accepted where
the release model says release.
**Smallest viable design:** extend the machinery that already exists: identity-keyed generated
bindings through the existing `declare_local`/`lookup` contract, the existing signature→modes
propagation (built for func-typed bindings by DEF-125), the existing scoped-owner and guarded
materialize shapes, the existing type-key renderer, and the existing E10036 postlude. No new
module, no new pass, no new dependency.
**Excluded machinery:** None considered — no layer, harness, or framework is introduced
(AR-15). Rejected alternatives per decision are recorded in the register's provenance blocks.
**Approved complexity:** None.

## Document Index

| # | Document | Description |
| - | -------- | ----------- |
| AR | [Ambiguity Register](00-ambiguity-register.md) | Zero-Ambiguity Gate decisions (audit trail) |
| 00 | [Index](00-index.md) | This document — overview and navigation |
| 01 | [Requirements](01-requirements.md) | Feature requirements and scope |
| 02 | [Current State](02-current-state.md) | Per-defect analysis with `file:line` loci |
| 03-01 | [Generated Identities](03-01-generated-identities.md) | DEF-138, DEF-140, DEF-165 |
| 03-02 | [Callable Effect Modes](03-02-callable-effect-modes.md) | DEF-155, DEF-156, DEF-157, DEF-166 |
| 03-03 | [Owner Release Completeness](03-03-owner-release-completeness.md) | DEF-139, DEF-158, DEF-159, DEF-163, DEF-164 |
| 03-04 | [Possibly-Null Widening](03-04-possibly-null-widening.md) | DEF-141 |
| 07 | [Testing Strategy](07-testing-strategy.md) | Test cases and verification |
| 99 | [Execution Plan](99-execution-plan.md) | Phases, sessions, and task checklist |

## Quick Reference

### Reproduction families (from the review ledger)

```clorraine
let __cl_owned0: String = stringOf("q");   // DEF-138: collides with the generated owner
let h: Holder<func(i32): i32> = ...;       // DEF-140: ClHolder__fn redefined
([s] () => i32(len(s)))()                  // DEF-165: struct ClEnv_0 defined twice
h.take(s);                                 // DEF-155: take mode lost, double-free
g(s);                                      // DEF-156: module func-global, no modes
f(stringOf("x"));                          // DEF-157: Param callee, temporary leaks
((p: T) => Holder<T> { v: p })(x)          // DEF-166: lambda param judged nowhere
H { call: addOne }.call(5);                // DEF-139: receiver siblings leak
defer { let v: str = (Box { v: mk() }).v; }// DEF-158: no materialization in defer
(c ? b : makeBox()).v                      // DEF-159: taken call arm leaks
func make(): str[1] { return [...]; }      // DEF-163: array of view elements leaks
make();                                    // DEF-164: discarded owned call leaks
len(s);                                    // DEF-141: null String segfaults (s: String?)
```

### Key Decisions

| Decision | Outcome | AR |
| -------- | ------- | -- |
| Generated owners | Identity-keyed; never resolved by source name; C collisions rename | AR-2, AR-3 |
| Func type key | Carries the parameter/return type keys | AR-4 |
| Env structs | One definition per unique (site, instance) | AR-5 |
| Callee modes | Every callee kind applies its own signature's modes; per-instance for generics | AR-6 |
| Release holes | Extended owner/release machinery at all five holes | AR-7 |
| View-array return | Same diagnostics as the single-view spelling | AR-8 |
| Null widening | `E10010` before the widening use | AR-9 |
| Spec text | Unchanged; any needed edit pauses to the user | AR-11 |

## Specialist Agents

**None** — the batch's surfaces (ownership/drop, codegen cleanup, and symbol keys) are covered by
the four installed project reviewers; the memory-safety and emission-integrity auditors are
routinely engaged by the project's dispatch matrix (they authored the base evidence for these
rows). No candidate clears the disqualifier "would only restate AGENTS.md or an existing lens".

_Detection evidence: `codeops/specialist-check.json` (refreshed 2026-10-10, 28 plans); the row
evidence lives in `plans/eval-emission-integrity/05-findings.md` (MS/EIA/SR/RV findings)._

## Related Files

| Path | Change |
| ---- | ------ |
| `compiler/src/own.rs` | `declare_local`/`lookup` contract for generated bindings (138) |
| `compiler/src/own/lower.rs` | Owner generation/reference, conditional arm owners, defer materialization (138, 158, 159) |
| `compiler/src/own/call.rs`, `call_check.rs` | Callee resolution and mode propagation (155 – 157, 166, 139) |
| `compiler/src/own/place.rs`, `compiler/src/borrow.rs` | View-return elision/escape coverage (163) |
| `compiler/src/own/expr.rs` | Possibly-null widening use (141) |
| `compiler/src/types/keys.rs` | Func type key (140) |
| `compiler/src/codegen/{program,function,expr}.rs` | Env struct dedupe (165); collision rename (138) |
| `compiler/tests/callable_release_integrity_spec_test.rs` | New — specification tests |
| `compiler/tests/callable_release_integrity_impl_test.rs` | New — implementation tests |
