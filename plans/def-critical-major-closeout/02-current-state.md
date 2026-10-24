# Current State: Critical/Major DEF Closeout

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Existing Implementation

The compiler already has the ownership walk (`compiler/src/own/`), the
post-monomorphization instance audit (`own/instance_check.rs`), and the
conditional/guarded emission helpers (`codegen/function.rs`,
`codegen/guards.rs`). Earlier batches closed the main safety classes; this
program addresses the residual shapes those fixes left behind.

`DEF-470` is already fixed at current HEAD. The evidence probe
`take h: H; let r: String? = c ? h.name : null; let s = h.name;` reports one
`E10020` (`'h.name' was moved and cannot be used`). The roadmap row is stale and
is closed in Phase 0.

### Relevant Files

| File | Purpose | Changes Needed |
|---|---|---|
| `compiler/src/own/expr.rs` | Moves, stores, conditional source provenance, whole-slot judgment | Source-state union; conditional-store source reads |
| `compiler/src/own/place.rs` | Place moves, alias/containment helpers, provenance | Rebound source resolution; alias root resolution |
| `compiler/src/own/statements.rs` | Let/const and assignment effects | Preserve/union source state across rebinds |
| `compiler/src/own/flow.rs`, `lower.rs` | Binding facts and constructors | Source-state field(s) and initialization |
| `compiler/src/own/instance_check.rs` | Substituted duties for generic instances | Nested projections, maybe-target moves, generic parity |
| `compiler/src/own/call_check.rs` | Callee scans and caller guards | Ordered-prefix source scan and conservative viewed-position/return proofs |
| `compiler/src/codegen/function.rs` | Fixed-array conditional assignment, struct-literal stores, take captures | Fixed-array/conditional destination captures, the take-capture stack, the DEF-460 save restore |
| `compiler/src/codegen/guards.rs`, `expr.rs` | Guarded literal materialization and plain literal rendering | Read-base, nested literal, and deferred take emission |
| `compiler/src/codegen/program.rs` | Module-initializer literals and projection roots | Per-arm init stores, fresh-root binding, call-arm release |
| `compiler/src/codegen/calls.rs` | Argument materialization and aliasing | Opaque/array alias guards |
| `compiler/src/types/table.rs`, `types/mod.rs`, `types/keys.rs`, `codegen/types.rs` | Type identity, diagnostics, and C symbol keys | DEF-416 collision only; no qualified key or C naming change |
| `compiler/src/sema/call.rs`, `sema/ops.rs`, `sema/expr.rs` | E10300 call checks, conditional/null rules, intrinsic typing | DEF-167, DEF-411/415, DEF-171 owners |
| `compiler/tests/borrow_store_integrity_*`, `def_*_integrity_*` | Regression coverage | New ST cases per batch |

## Gaps Identified

### Batch A — Critical flow (DEF-543)

**Current Behavior:** A local view is assigned a static branch and a
frame-backed branch; the declaration/assignment provenance is overwritten by
the last assignment, so the same-frame store judge accepts the frame-backed
path. The cross-frame caller guard now rejects, but the same-frame form remains
open.
**Required Behavior:** A store of a rebound local whose source union contains a
non-`take` parameter root reports the existing `E10034` once, except the slot's
own parameter (the existing `owner == destination` exclusion); a local/temp
root reports `E10030`; a `take` root is left to the existing ordinary rule (a
take view that dies with the frame reports `E10030`, an owned take value is
accepted); an all-static or owned union stays accepted. A destination or value
that still mentions a type parameter is left to the existing per-instance duty.
**Fix Required:** Preserve a small source set on the flow-sensitive place state
of rebound locals and consult it at whole-out-slot stores; do not build a
general dataflow framework.

### Batch B — Raw/conditional store integrity (DEF-454…466)

| DEF | Current Behavior | Required Behavior |
|---|---|---|
| 454 | Guarded struct-literal argument/receiver copies a raw-deref owning field; ASAN double-free | Move the raw-deref field (empty the pointee) in the guarded position |
| 455 | Conditional fixed-array assignment empties the destination through a raw-deref take and frees the saved array again | Install the take capture around the destination/arms |
| 456 | Alias test is address equality, not subobject containment; nonzero offset double-frees or leaks siblings | Test containment and preserve the correct save granularity |
| 457 | Nested temporary materialization loses the outer take capture | Use a take-capture stack so every live destination is flagged |
| 458 | A named-place take in a skipped `\|\|` right zeroes the save unconditionally | Apply the runtime take-ran capture to place move-outs; a skipped right must not zero the save |
| 460 | Conditional self-move through a raw pointer copies the destination into itself and empties it | Recognize the raw self-alias, restore the save into the destination, and zero the save |
| 461 | Fixed-array take parameter aliasing the raw target is not guarded | Keep a stable alias guard for the array source |
| 462 | Raw-field intermediate is not pinned before a side-effecting RHS | Pin the intermediate lvalue before lowering the RHS |
| 464 | Post-conditional-store source read is accepted and dangles | Mark the source moved and report `E10020` on the later read |
| 465 | Duplicable fixed-array literal self-read uses the cleared destination | Evaluate the RHS before clearing the destination |
| 466 | Raw alias element store whose RHS pointee is the destination loses the value | Add the runtime target/source address test before the owning move |

### Batch C — Conditional materialization and read bases (DEF-470…481)

| DEF | Current Behavior | Required Behavior |
|---|---|---|
| 470 | Already reports the concrete `E10020` at HEAD | Close as stale in Phase 0 |
| 471 | Hidden generic deref-chain division omits the `E11210` guard; `/0` SIGFPE | Resolve the raw pointee type and emit the guard |
| 472 | Maybe-target generic conditional with a concrete lend double-frees | Judge the concrete source against the substituted target; reject once |
| 473 | Maybe-target take + concrete local conditional use-after-move is accepted | Record the definite move on the maybe-target path; report `E10020` |
| 475 | Owned mixed conditional read base is accepted and emits invalid C | Bind the hidden destination or distribute the projection per arm |
| 476 | Guarded duplicable conditional literal read base emits invalid C | Emit the per-arm store on the evaluated path |
| 477 | Nested conditional arms in a literal store/guarded take emit invalid C | Recurse into conditional arms in the store emitter |
| 478 | Duplicable struct conditional take in a guard fills eagerly | Include the duplicable struct conditional in the deferred take predicate |
| 479 | Guarded nested-literal field evaluates in declaration order | Hoist/emit nested literal fields in source order |
| 480 | Owning fresh-root projection read base emits invalid C or leaks siblings | Bind the fresh root, move the selected member, release the rest |
| 481 | Owning conditional read base over call arms leaks | Bind and release the call arm's root |

### Batch D — Generic/null/conditional parity (DEF-411…522)

| DEF | Current Behavior | Required Behavior |
|---|---|---|
| 411 | Null-arm conditional in a guard/cast/binary/let/loop head is accepted and fails C or SEGVs | Apply the existing null rule at each concrete context |
| 412 | Substituted view-array return is falsely rejected `E10036` | Match the concrete twin; accept the safe return |
| 413 | Nested element of an unknown substitution emits a placeholder and invalid C | Resolve the element through the instance |
| 415 | Same-arm nullable conditional at a non-nullable `str` destination synthesizes zero views | Report one `E10010` per nullable arm (two for the two-arm ST-26), naming the base |
| 505 | Nested projection of a type-parameter place is falsely joined; duplicate `E10034` | Resolve the nested projection; a duplicable member is accepted with no duty, and an owning member reports the concrete twin's one `E10026`/`E10034` |
| 506 | Mixed aggregate whose only viewed position is static is falsely rejected | Revalidate at HEAD; if the exact filed shape still rejects, accept only with a conservative no-escape proof |
| 507 | Helper with one lend parameter that returns static storage is falsely rejected | Revalidate at HEAD; the documented safe shapes are already accepted, so close stale unless the exact filed shape still rejects |
| 513 | Alias return over an inline iterable temporary escapes and UAFs | Resolve the inline iterable/temporary and report the escape |
| 522 | View-member projection over a type-parameter conditional is conservatively rejected or emits invalid C | Accept the safe move and read/return projections and emit valid C; pin the arm backing |

### Batch E — Strict build and type identity (DEF-167/171/416)

| DEF | Current Behavior | Required Behavior |
|---|---|---|
| 167 | `a < b > (c)` is accepted as a generic call, but the callee is not generic and the C call fails. The filed shape is the stage test's **i32 local** `a`; a free-function callee is the `DEF-418` twin | Keep the frozen generic-call parse; add a parse-only/stage-select mode (`compiler/src/driver.rs`); reject explicit type arguments on **every** non-generic callee with one `E10300`, including the non-callable local/parameter value |
| 171 | `embed` outside a constant context is accepted and emits invalid C | Enforce DI-4 / `E10213`; re-enable the E10213 golden; emit no translation unit. Add a constant-context flag so a module `const` keeps only `E10224`. A module-level `let embed` reports `E10213` under R-6 (follow `spec/04` §16; the `spec/13` §1 sentence is a recorded frozen inconsistency) |
| 416 | Two modules' same simple type name collapse into one type-table entry | Report one `E11101` at the second declaration from a cross-module check in `sema` (with a pure `TypeTable` collision query); no merged entry and no C. Keep the simple table; no qualified identity layer |

## Dependencies

- Batch A's flow-sensitive source union is independent of Batch B.
- Batch B and Batch C share the `codegen/function.rs`/`guards.rs` conditional
  and capture surface; B lands first as risk control, not as a hard dependency.
  Batch C's maybe-target source split is a soft dependency for Batch D.
- Batch D reuses the `own/call_check.rs` pattern for its bounded scans; it does
  not depend on a new summary abstraction. `DEF-507` is revalidated first.
- Batch E is source-independent but output-coupled to the C pins; it lands last
  and re-runs the A–D determinism pins. The `DEF-167` parse-only mode is a
  driver/test integration, not a language change.

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| A fix changes accepted C byte output unexpectedly | Medium | High | Determinism pins and corpus diffs; update only the affected ST case |
| Precision fixes weaken a safety rejection | Medium | High | Every accepted control is paired with the unsafe twin; ASAN/strict C verify |
| Source union becomes a general dataflow layer | Low | Medium | Bound it to rebound locals and join values; complexity stop if it grows |
| Conditional emission fixes interact across rows | High | Medium | Fix B before C; run the focused conditional suites after each codegen task |
| A DEF is stale or no longer reproducible | Medium | Low | Phase 0 revalidates all 35 rows and closes only with HEAD evidence; `DEF-506`/`DEF-507` are the current suspects. A stale close gets a green HEAD pin, not a forced red test |
| The DEF-416 collision report is mistaken for a spec change | Medium | Medium | It uses the existing E11101 code and is recorded as a conservative limitation (AR-20), not a chapter change |
| The new source set leaks across a reused temporary id | Medium | Medium | Keep it on `PlaceState`, which `drop_bindings_from`/`edge_state` already prune; add an id-reuse unit pin |
| The module-level `let embed` spec conflict blocks DEF-171 | Medium | High | R-6 resolved: follow `spec/04` §16; a module-level `let` reports `E10213`; the `spec/13` §1 sentence is recorded as a frozen inconsistency |
| Spec change needed mid-flight | Low | High | Stop under AR-11 and obtain approval before editing the spec |
