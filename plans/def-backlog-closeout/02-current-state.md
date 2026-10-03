# Current State: Def Backlog Closeout

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Existing Implementation

### What Exists

The ownership stage, typed IR, and emitter already carry every mechanism these fixes need:
a deferred materialization carrier for guarded operands, an owned-argument binding for
borrow-taking callees, a place derivation with index pinning, prepare/emit store arms with a
move-out, a return-release append for loop conditions, a single pointer-parameter predicate, and a
lambda checker with explicit capture handling. The defects below are gaps in how those mechanisms
are reached, not missing machinery.

### Relevant Files

| File | Purpose | Changes Needed |
| ---- | ------- | -------------- |
| `compiler/src/own/lower.rs` | Value lowering, place derivation, guard operands, owned arguments | DEF-63/64/66/67 fix surface |
| `compiler/src/own/call.rs` | Call and intrinsic lowering, take move-out places | DEF-63/67 fix surface |
| `compiler/src/own/loops.rs` | Loop lowering and the do-while condition-release walk | DEF-68 fix surface |
| `compiler/src/own/statements.rs` | Stores, returns, defer registration | DEF-64/68 support |
| `compiler/src/ir.rs`, `compiler/src/ir/place.rs` | Materialize carrier, deferred releases | DEF-63/64/68 support |
| `compiler/src/codegen/types.rs` | Parameter lowering predicate | DEF-65 fix surface |
| `compiler/src/codegen/calls.rs` | Call arguments | DEF-65 call-site fix |
| `compiler/src/codegen/expr.rs` | Place rendering | DEF-64 support |
| `compiler/src/sema/expr.rs`, `compiler/src/sema/mod.rs` | Lambda checking, name resolution | DEF-69 fix surface |
| `spec/14-diagnostics.md`, `spec/tests/compile-fail/` | Diagnostic registry and goldens | Approved E10405 addition |
| `compiler/tests/def_backlog_closeout_{spec,impl}_test.rs` | New suites | Created by this plan |

### Code Analysis

Shared machinery the fixes reuse:

| Function / type | Location | Role |
| --------------- | -------- | ---- |
| `materialize_guard_operand` | `compiler/src/own/lower.rs:646-679` | Deferred `Materialize` temp for a side-effecting operand; registers the release |
| `materialize_owned_argument` | `compiler/src/own/lower.rs:1336-1432` | Binds a caller-owned value the callee borrows; take block `:1336-1369` skips the root bind when guarded |
| `take_move_out_places` | `compiler/src/own/call.rs:346-380` | Places emptied after a take; `Place`/`Temp`/`Materialize` arms, `_ => value_place` |
| `value_place` | `compiler/src/own/lower.rs:892-931` | Value to place; `Element(None)` at `:923`, `_ => None` at `:929` |
| `bind_ownerless_projection_root` | `compiler/src/own/lower.rs:1281-1316` | Re-roots `Member`/`Index` chains whose root is `Call`/`ClosureCall`; no `Intrinsic` arm |
| `settle_projection_base` / `materialize_index_chain` | `compiler/src/own/lower.rs:690-730` / `:739-764` | Emit the deferred base fill; pin a run-time index |
| `consume_place` | `compiler/src/own/lower.rs:863-884` | Aggregate-literal consume; no settle, no index chain |
| `prepare_store` / `PreparedStore::AssignThenMoveOut` | `compiler/src/own/statements.rs:287-348`, `:16`, `:351-359` | The complete store preparation the consume path lacks |
| `append_condition_releases` | `compiler/src/own/loops.rs:174-221` | Walks CFG successors from the do-while body; misses defer blocks |
| `Builder::append_return_releases` | `compiler/src/ir.rs:719-747` | Prepends releases to a return terminator; correct as-is |
| `check_defer` | `compiler/src/own/statements.rs:763-798` | Registers and lowers the deferred block at the marker, before the condition exists |
| `is_pointer_param` | `compiler/src/codegen/types.rs:30-32` | `Borrow && Struct && !duplicable`; ignores `ty.nullable` |
| `Checker::check_lambda` | `compiler/src/sema/expr.rs:105-219` | Binds captures and params; outer scopes stay visible |
| `Checker::name_type` | `compiler/src/sema/mod.rs:421-459` | Walks every scope outward with no capture diagnostic |
| `collect_lowered_temp_owners` | `compiler/src/own/lower.rs:1064-1119` | The only slice-aware root walk (`:1112-1116`), used for view-loan provenance |

## Gaps Identified

### Gap 1: DEF-63 — guarded member-root take never binds its call root

**Current Behavior:** `flag && sink(mkv().a)` lowers the guarded operand with
`defer_move_outs > 0`; `materialize_owned_argument` skips the root bind
(`compiler/src/own/lower.rs:1342-1346`) and gates the second bind (`:1356`).
`value_place(Member{base: Call})` then hits `_ => None` (`:929`), so
`take_move_out_places` finds no place, the deferred branch at `:99-121` is never entered, and the
call result's sibling fields are never released (ASan direct leak, no double free).
**Required Behavior:** the call result is bound to a deferred temporary; the evaluated path
zeroes the moved field and releases the result exactly once.
**Fix Required:** bind the call root through the deferred `Materialize` temp under a guard (03-01 §1, AR-8).

### Gap 2: DEF-64 — aggregate-literal consume discards the root call and index

**Current Behavior:** `Box<T> { v: mk()[i] }` lowers the field through `consume_place`
(`compiler/src/own/lower.rs:863-884`), which calls `value_place` directly: the `Materialize` base
flattens to `Temp` (`:902-906`), the run-time index becomes `Element(None)` (`:923`), and a bare
`Statement::Move` — which cannot carry the discarded fill — is emitted. Codegen renders `[0]`, and
the root call is never emitted (`compiler/src/codegen/expr.rs:134-141`).
**Required Behavior:** the root call is emitted before the move, the run-time index is pinned and
evaluated once, and the `i`-th slot is emptied.
**Fix Required:** prepare the consume path like the store path (settle + index chain + root bind)
and emit the base fill explicitly (03-01 §2, AR-9).

### Gap 3: DEF-66 — element read through a slice view over a statement temporary

**Current Behavior:** filed before the base-settlement fix landed; code reading suggests the
generic store/take paths now settle and move out, but no test or probe covers the slice root:
`value_place` has no `Intrinsic` arm (`compiler/src/own/lower.rs:929`) and
`bind_ownerless_projection_root` recognizes only `Call`/`ClosureCall` (`:1281-1316`).
**Required Behavior:** each probed form is either proven clean and pinned, or fixed so the element
moves out before the temporary release; existing `E10026`/`E10030` rejections stay.
**Fix Required:** reproduction probe first; extend the slice-intrinsic root handling only if a form
reproduces (03-01 §3, AR-2).

### Gap 4: DEF-65 — nullable borrow out-slot passed by value

**Current Behavior:** `is_pointer_param` (`compiler/src/codegen/types.rs:30-32`) returns false for
`String?`/`File?`/`str?`/`T[]?` (kinds `Handle`, `Str`, `Array`), so a `borrow` out-slot is passed
by value; the callee's assignment drops the old target (`compiler/src/codegen/function.rs:965-968`)
while the caller still owns it, and the caller's scope cleanup frees it again (double free). For
`str?`, callee writes are lost.
**Required Behavior:** a pointer per `spec/17-codegen.md:65`; the callee's write reaches the
caller; exactly one free on every path.
**Fix Required:** add `ty.nullable` to the predicate and materialize view conversions at the call
site (03-03, AR-3, AR-10).

### Gap 5: DEF-67 — ownerless call result as a borrow argument leaks

**Current Behavior:** the intrinsic arm lowers its argument with plain `lower_value`
(`compiler/src/own/lower.rs:244-258`) and the dot-`len` receiver likewise
(`compiler/src/own/call.rs:273-285`); no owned binding or release is registered, so
`i32(len(stringOf("abc")))` emits the call inline and the `ClString` handle leaks (ASan 12 bytes).
**Required Behavior:** the result is bound and released exactly once at statement end or return;
guarded operands defer; no double release where a destination adopts.
**Fix Required:** route intrinsic arguments and dot-`len` receivers through
`materialize_owned_argument`, guard-aware (03-02, AR-6).

### Gap 6: DEF-68 — do-while deferred return misses the condition release

**Current Behavior:** `check_defer` registers the block before the condition is lowered
(`compiler/src/own/statements.rs:763-798`); `append_condition_releases` walks CFG successors from
`body_block` (`compiler/src/own/loops.rs:183-204`) and finds no terminator targeting the defer
block, so the condition temporary's release is never appended to that return (ASan 20 bytes in
2 allocations).
**Required Behavior:** the deferred return releases the condition temporaries; other paths are
unchanged.
**Fix Required:** scan the body's reachable defer markers and union their return successors into
the existing append (03-04, AR-7).

### Gap 7: DEF-69 — unlisted enclosing read emits undeclared C

**Current Behavior:** `resolve` and `sema` leave outer scopes visible
(`compiler/src/resolve/mod.rs:317-338`, `compiler/src/sema/mod.rs:421-459`), so `[x] () => n`
types `n` from the enclosing function; `lift_lambda` replaces the binding tables with captures and
params only (`compiler/src/own/lower.rs:378-496`), the unknown name becomes a bare path, and the
lifted C function references undeclared `n` (C build failure). No E10401/E10402 fires either.
**Required Behavior:** an unlisted enclosing read reports the approved `E10405`; listed captures,
globals, and existing capture diagnostics are unchanged.
**Fix Required:** a scope-floor check in the lambda checker plus the approved registry row and
golden (03-05, AR-4, AR-13, AR-14).

## Dependencies

### Internal Dependencies

- The def-backlog-completion machinery: `Builder::append_return_releases`
  (`compiler/src/ir.rs:719-747`) and the element-temp/index-pinning paths
  (`settle_projection_base`, `materialize_index_chain`).
- The runtime-shim-host probe toolchain: `target/debug/clc --profile host` plus
  `cc -fsanitize=address`, as used by the previous cycle's probe runner.
- The quality-profile reviewer dispatch per `AGENTS.md` (correctness, memory-safety,
  emission-integrity, semantics-reviewer).

### External Dependencies

- None. The workspace stays std-only.

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| DEF-66 no longer reproduces (already fixed by the base-settlement change) | High | Low | AR-2 reproduction gate; clean forms close with regression pins and a roadmap correction |
| `Statement::Move` cannot carry the DEF-64 fill; the fix needs a new emission shape | Medium | Medium | Reuse the existing `PreparedStore::AssignThenMoveOut`/explicit-fill pattern (AR-9); emission-integrity review |
| `&(rvalue)` for a pointer-lowered `borrow str?` receiving a `String` | Medium | Medium | Materialize the view conversion into a temporary (AR-10) |
| The DEF-68 marker scan re-appends releases on a path that already has them | Low | Medium | Dedupe by `Release.temp` already exists (`compiler/src/ir.rs:719-747`); spec case ST-19 |
| The DEF-69 scope floor flags module-level names or shadowed names | Medium | Medium | Floor is set at lambda entry; module bindings are above it; cases ST-21 – ST-23 |
| In-container `RDC_ASAN=1` is timing-flaky, hiding memory regressions | Medium | High | Task-time host-ASan probes are mandatory (AR-15); `RDC_ASAN` is not used as a gate |
