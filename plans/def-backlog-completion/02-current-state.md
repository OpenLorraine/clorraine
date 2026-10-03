# Current State: Def Backlog Completion

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Existing Implementation

### What Exists

The compiler is at the state committed after the residual-defect-completion plan. The ownership
stage (`compiler/src/own/`) classifies moves, records post-substitution duties
(`RequiresDuplicable`), lowers values and places, and materializes ownerless call results. The
codegen stage (`compiler/src/codegen/`) renders type keys, instance-specific symbols, calls, stores,
returns, loops, and guard machinery.

Two mechanisms from the previous cycle matter here:

- the **maybe-owning branch**: while a type is still a `Ty::Param`, checks that must judge ownership
  take a conservative path (`maybe_owns` → `mark_moved_if_owning`) instead of the concrete
  `move_target`, so the `E10034` diagnostic is never reached;
- the **ownerless call-result binding**: a call result read for a projection or element is bound to
  an owned temporary and the projection is moved out, with the take path emitting an after-call
  move-out.

The seven open defects are the places where those two mechanisms stop short.

### Relevant Files

| File | Purpose | Changes Needed |
| ---- | ------- | -------------- |
| `compiler/src/own/place.rs` | `mark_moved_if_owning`, `move_place`, E10034 site | Record the post-substitution lend duty (DEF-56, DEF-61) |
| `compiler/src/own/statements.rs` | Return classification, `prepare_store`, temp releases | Store move for destination temps (DEF-59); record duties from the return branch |
| `compiler/src/own/call_check.rs` | Take-argument decision | Record duties from the take branch (DEF-61) |
| `compiler/src/ir/mono.rs` | Recorded instance requirements | New duty record beside `RequiresDuplicable` |
| `compiler/src/own.rs` | Duty collection, `declare_local`, temp release | Collect the duty; keep no other change |
| `compiler/src/own/instance_check.rs` | Post-substitution audit | Report `E10034` when the substituted type owns |
| `compiler/src/own/lower.rs` | Element places, `Materialize` temps, `defer_move_outs` | Carry the runtime index into the move-out place (DEF-60) |
| `compiler/src/own/call.rs` | `take_move_out_places` | Emit the index-correct move-out (DEF-60) |
| `compiler/src/codegen/expr.rs` | Element rendering; closure call renderer | Render the carried index (DEF-60); omit the empty separator (DEF-58) |
| `compiler/src/codegen/cleanup.rs` | Return emission and arm source moves | Move the source before releases in the no-ladder return branch (DEF-59) |
| `compiler/src/own/loops.rs` | `check_while`, `check_do_while` | Append the condition's new releases to body returns (DEF-62) |
| `compiler/src/own/generics.rs` | Instance collection | Stamp the enclosing key on recorded closure arguments (DEF-57) |
| `compiler/src/types/` | Stage-neutral type model | Host the relocated key renderer (DEF-57, AR-3) |
| `compiler/src/codegen/types.rs` | `type_key`, closure C name | Include the closure instance (DEF-57) |
| `compiler/src/ir/mono.rs` (dedupe) | Instance argument dedupe | Keep two enclosing closure specializations distinct (DEF-57) |
| `compiler/tests/residual_defect_completion_impl_test.rs` | Stale pin for the move-out index | Update the pin that asserts the buggy `__cl_tmp0[0] = NULL` for `__cl_tmp0[i]` (AR-6) |

### Reproductions

All reproductions were re-verified at `d06d451` with the host profile and AddressSanitizer leak
detection. The recipe is the previous cycle's probe loop: `target/debug/clc --profile host -o X.c
X.clo`, then `cc -std=c99 -O0 -g -fsanitize=address -o X.asan X.c`, then
`ASAN_OPTIONS=detect_leaks=1:symbolize=0 ./X.asan`.

| Defect | Minimal form | Failure on the committed compiler |
| ------ | ------------ | --------------------------------- |
| DEF-56 | `apply<T>(x: T): T { return x; }`; `outer<T>(x: T): T { return apply<T>(x); }`; `let s: String = outer<String>(stringOf("abc"));` | Accepted; ASan `attempting double-free` |
| DEF-57 | `apply<T, F: func(T): T>(x: T, f: F): T { return f(x); }`; `outer<T>(x: T): T { return apply(x, [x] (y: T) => y); }`; `main` calls `outer<i32>` | C build fails: `unknown type name 'ClClosure_0'` (call site passes `ClClosure_0__i32`) |
| DEF-58 | `consume<F: func(): i32>(f: F): i32 { return f(); }`; `consume([x] () => x)` | C build fails: `((f).fn((f).env, ))` has a trailing comma |
| DEF-59 | `first<T>(mk: func(): T[2]): T { return mk()[0]; }` and `let s: T = mk()[0];`, instantiated `String` | Accepted; ASan heap-use-after-free (the statement temporary released the slot first) |
| DEF-60 | `flag && sink<T>(mk()[i])` with `i = 1` (also unguarded `sink<T>(mk()[i])`) | Accepted; ASan double-free (the move-out zeroed slot 0 while the callee took slot `i`) |
| DEF-61 | `f<T>(take h: Holder<T>, v: T) { h.items.push(v); }`, instantiated `String` | Accepted; ASan double-free; the concrete spelling reports `E10034` |
| DEF-62 | `do { i = i + 1; if (i == 2) { return i; } } while (grow()[0].count > 0);` | Accepted; ASan `70 byte(s) leaked in 6 allocation(s)` |

## Gaps Identified

### Gap 1: The maybe-owning branch never reaches `E10034` (DEF-56, DEF-61)

**Current Behavior:** When a moved place's type is a type parameter, `mark_moved_if_owning`
(`compiler/src/own/place.rs:295-301`) records nothing when the parameter is a lend; all checks run
pre-substitution (`compiler/src/own.rs:82-128`), and the post-substitution audit
(`compiler/src/own/instance_check.rs:35-98`) only covers view fields, fills, and duplicability
duties. `E10034` is emitted only from `move_place` (`compiler/src/own/place.rs:204-215`), reached
via `move_target`. Concrete spellings reject; generic spellings accept and double-free.
**Required Behavior:** The generic spelling reports `E10034` at the move site when the instantiated
type owns (R1, R2).
**Fix Required:** Record a post-substitution lend duty at the central maybe-owning site and audit it
after substitution. See [03-01](03-01-generic-lend-duties.md).

### Gap 2: The move-out loses the element index (DEF-60)

**Current Behavior:** `value_place` stores `Projection::Element(None)` for a runtime index
(`compiler/src/own/lower.rs:792-803`), and codegen renders `Element(None)` as `[0]`
(`compiler/src/codegen/expr.rs:130-137`). The callee moves slot `i`; the after-call move-out zeroes
slot 0, so the temporary's release frees the moved slot again. The existing implementation pin even
asserts the buggy shape (`compiler/tests/residual_defect_completion_impl_test.rs:802-803`).
**Required Behavior:** The move-out zeroes the slot the callee moved, for constant and runtime
indices, guarded and unguarded (R3).
**Fix Required:** Carry the index into the move-out; update the stale pin. See
[03-02](03-02-move-out-integrity.md).

### Gap 3: The store/return projection binding skips statement-owned roots (DEF-59)

**Current Behavior:** `projects_from_place` (`compiler/src/own/lower.rs:863-884`) treats a
`Materialize` root and a destination `Temp` as storage, so `prepare_store`
(`compiler/src/own/statements.rs:307-319`) and the return path (`compiler/src/own/statements.rs:901-912`)
skip `bind_ownerless_projection_root` and produce a plain copy. No move-out empties the slot before
the statement temporary releases it, so the stored or returned value aliases freed memory.
**Required Behavior:** The read moves the element out of the statement-owned temporary before its
release (R4).
**Fix Required:** Store through `PreparedStore::Move`; return through the emitter's no-ladder source
move. See [03-02](03-02-move-out-integrity.md).

### Gap 4: The do-while return never sees the condition releases (DEF-62)

**Current Behavior:** `check_do_while` lowers the body before the condition
(`compiler/src/own/loops.rs:123-136`), so a `return` snapshots its release list
(`compiler/src/own/statements.rs:927-936`) before the condition materializes its owned call and
registers the temporary's release. The temporary's only release drains into the loop exit
(`compiler/src/own/loops.rs:149-154`), which a return skips.
**Required Behavior:** An early return releases the condition's materialized temporaries (R5).
**Fix Required:** Append the condition's newly registered releases to the body's return terminators.
See [03-03](03-03-do-while-condition-release.md).

### Gap 5: The recorded closure argument has no enclosing instance (DEF-57)

**Current Behavior:** Sema creates closure types with `instance: None`
(`compiler/src/types/mod.rs:121-135`, `compiler/src/sema/expr.rs:209-217`); the inferred callable
argument is recorded unchanged (`compiler/src/own/call.rs:226-248`,
`compiler/src/own/generics.rs:734-758`); only the emitter tags closures from the active instance key
(`compiler/src/codegen.rs:331-367`). The call-site value spells `ClClosure_0__i32` while the callee
signature, built from the recorded argument, spells `ClClosure_0`.
**Required Behavior:** The callee parameter spelling matches the call site; two enclosing
specializations stay distinct (R6).
**Fix Required:** Stamp the key at recording and include it in `type_key`. See
[03-04](03-04-closure-c-spelling.md).

### Gap 6: The closure call renderer always writes the separator (DEF-58)

**Current Behavior:** `render_call`'s closure-typed-parameter branch writes `.env, {arguments}`
unconditionally (`compiler/src/codegen/expr.rs:458-461`), so an empty argument list emits
`((f).fn((f).env, ))`. The sibling renderer `closure_call_with_destination` already handles the
empty list (`compiler/src/codegen/expr.rs:636-640`).
**Required Behavior:** A zero-argument closure call emits valid C (R7).
**Fix Required:** Branch on the empty argument list at the same line. See
[03-04](03-04-closure-c-spelling.md).

## Dependencies

### Internal Dependencies

- The `own` stage's duty machinery (`ir/mono.rs`, `own.rs`, `own/instance_check.rs`) for R1/R2.
- The `Materialize`/destination-temp machinery and the move-out statement for R3/R4.
- The return release pipeline (`emit_return`, `emit_arm_source_moves`) for R4 and R5.
- The instance collection walk (`own/generics.rs`) and the instance key (`codegen/types.rs`) for R6.
- The existing test suites and the pinned re-emission determinism tests.

### External Dependencies

None. The workspace is std-only; no crate, build script, or new tool is needed.

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| The centralized duty changes more call sites than the two filed ones | Med | Med | Audit `mark_moved_if_owning` callers; the duty only fires for lend roots and owning substitutions; concrete spellings regression-tested |
| Carrying the index changes the `Place`/move-out IR shape broadly | Med | High | Keep the constant-index rendering unchanged; pin emitted shapes; the updated stale pin is the only expected implementation-test change |
| The do-while release append double-releases outer pending releases | Low | High | Append only releases registered during condition lowering (slice from the pre-condition length); dedupe by place |
| Relocating the key renderer breaks stage boundaries or symbol determinism | Low | Med | Move only the renderer into `types`; re-emission determinism tests pin byte-identical output |
| Threading the key into `own` creates an `own` → `codegen` import | Low | Med | The renderer lives in `types`; `own` and `codegen` both consume it (AR-3) |
| The generic rejection breaks an existing accepted corpus case | Low | High | Run `make check` and `make verify`; the spec corpus contains no such case (concrete spellings already reject) |
