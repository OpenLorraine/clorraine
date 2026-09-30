# Owned Conditional Stores: Call Value Integrity

> **Document**: 03-01-scalar-conditional-ownership.md
> **Parent**: [Index](00-index.md)

## Overview

Fix DEF-34 (an owned conditional double-frees) and DEF-45 (a deferred take-call transfer temporary
leaks when nothing adopts it), and pin the spec-mandated `E10026` rejection for move-only element
arms. The design extends the fixed-array branch-store rule to every owned, non-array destination
(a handle such as `String`, or an owned aggregate such as a struct with owned fields) and makes the
deferred-take transfer temporary a first-class release-temp. "Owned conditional" below means a
conditional whose destination type needs a drop and is not a fixed array; fixed arrays keep their
own branch store (`03-02`).

**Decisions per:** AR #2 (mechanism), AR #3 (element arms), AR #15 (existing pins), AR #19 (call
arms and by-value adoption).

## Architecture

### Current Architecture

- A store whose value is an owned conditional lowers to a plain `Assign` with a
  `BindingEffect::ConditionalMove` (`compiler/src/own/statements.rs:272-292`, `:381-411`, `:58`),
  and emission writes `target = (cond ? a : b);` in the scalar path
  (`compiler/src/codegen/function.rs:709-748`, rendering `compiler/src/codegen/expr.rs:73-82`).
- The forwarding rule `projects_from_place` (`compiler/src/own/lower.rs:696-717`) treats a
  conditional whose arms are place-rooted (including any `Materialize`) as forwarding, so no owning
  copy is made.
- A take call deferred into a guard creates `ValueKind::Materialize { temp, value, move_outs }`
  (`compiler/src/ir.rs:268-276`, `compiler/src/own/lower.rs:99-118`) without a pending release; the
  adoption test `projects_from_place` returns true for it, and the `materialize_owned_argument`
  skip at `compiler/src/own/lower.rs:950-962` treats it as place-rooted.
- Fixed arrays already solve the same problem with a branch store per path
  (`emit_fixed_array_conditional_assign`, `compiler/src/codegen/function.rs:825`, arms at `:877`).

### Proposed Changes

1. **Owned conditional store rule.** When the destination type needs a drop and is not a fixed
   array, and the lowered value is a conditional whose arms are owned sources, emit a branch store
   instead of a value expression:

   ```c
   if (ok) { c = a; a = NULL; } else { c = b; b = NULL; }
   ```

   Each arm moves its source through the aggregate-aware zero (`emit_move_zero`,
   `compiler/src/codegen/function.rs:1101`: the null literal for a handle, `cl_zero` for an
   aggregate); after the statement both arms are maybe-moved, matching the existing join rule. The
   condition is evaluated exactly once, and only the chosen arm's source expression is evaluated.

   The store mirrors the fixed-array twin's destination rules
   (`emit_fixed_array_conditional_assign`, `compiler/src/codegen/function.rs:825`):

   - an empty destination is stored directly per arm;
   - a live owned destination is saved first; the save is released once after the branch, and an
     arm that names the destination itself restores the saved value (with the save emptied, so
     the release is a no-op);
   - a nested conditional arm recurses through the same rule.

2. **Call arms.** `conditional_move_sources` (`compiler/src/own/expr.rs:69-93`) accepts a call
   whose result type needs a drop. The branch store assigns the call result directly
   (`c = mk();`) when the arm is a plain call; a by-value fixed-array call arm continues to use the
   hidden destination through the existing arm machinery.

3. **Mixed arms.** Place arm and call arm in the same conditional are both allowed; per path the
   arm moves (place arms zero their source; call arms produce a value the destination adopts).

4. **Transfer temporary as a release temp.** A `Materialize` created under `defer_move_outs`
   (`compiler/src/own/lower.rs:99-118`, context `compiler/src/own.rs:218-223`) registers a pending
   release (`register_pending_release`, `compiler/src/own.rs:963`) so a path that discards the
   value releases it exactly once. Temporary lifecycle facts the design relies on: every temporary
   is zero-initialized at function entry (`emit_local_declarations`,
   `compiler/src/codegen/function.rs:249-262`); every release zeroes the place after freeing
   (`compiler/src/codegen/drop.rs:32-38`); so a release of a never-assigned, discarded, or adopted
   temporary is a null-safe no-op. No statement-start initialization is added.

5. **Adoption moves out.** Every adopting site takes the value and zeroes the temporary (or the
   chosen arm), so the statement-end release is a no-op:

   - binding/assignment: the branch store's call arm (`c = __cl_tmp0; __cl_tmp0 = NULL;`);
   - by-value (`take`) argument: the conditional lowers to a branch store into a **non-releasing
     argument temporary**; the chosen source arm is zeroed, and the call consumes the temporary
     (the callee's parameter cleanup owns the value). No pending release is registered for the
     argument temporary, and the arm's storage is never released by the caller. A fixed-array
     argument keeps the existing element zeroing (`take_move_out_places`,
     `compiler/src/own/call.rs:333-347`);
   - `return`: a conditional return is stored per arm into the result (mirroring
     `emit_fixed_array_return_arm`, `compiler/src/codegen/cleanup.rs:358`) and the source is zeroed
     after the copy, so the cleanup ladder never releases the returned storage;
   - struct-literal field: an owned conditional field is an owning destination; the field store
     takes the value and zeroes the temporary (the fixed-array field store is designed in
     [03-02](03-02-fixed-array-value-positions.md)).

6. **Discard path.** A discarded owned conditional keeps the transfer temporary pending; at
   statement end `drain_pending_releases` (`compiler/src/own.rs:979`) releases it, and a path that
   never assigned it releases the null handle (no-op). A place arm in a discard is only read; it
   stays owned.

7. **Element arms.** `conditional_move_sources` keeps rejecting move-only element places with
   `E10026` (`spec/03-ownership.md:126-128`); duplicable element arms continue to copy. A
   regression case pins both.

## Implementation Details

### Changed Functions

| Function | File | Change |
| -------- | ---- | ------ |
| `prepare_store` / binding effect | `compiler/src/own/statements.rs` | Owned (non-array) conditional takes the branch-store effect; struct-literal fields mark their arms |
| `conditional_move_sources` | `compiler/src/own/expr.rs` | Accept owned call arms and owning struct-literal fields; keep the element rejection |
| Conditional lowering | `compiler/src/own/lower.rs` | Transfer `Materialize` registers a pending release; adoption zeroes the temporary |
| `drain_pending_releases` | `compiler/src/own.rs` | Releases transfer temps like destination temps (release-on-discard) |
| `emit_assign` | `compiler/src/codegen/function.rs` | Owned conditional emits the branch store (`emit_conditional_move_assign`) with the fixed-array twin's old-target/save and self-naming rules |
| Arm emission | `compiler/src/codegen/expr.rs` | Call arm assigns the call result; a place arm zeroes through `emit_move_zero` |
| Argument binding | `compiler/src/own/lower.rs`, `compiler/src/codegen/calls.rs` | Owned conditional argument uses a non-releasing argument temporary |
| Return and field adoption | `compiler/src/codegen/cleanup.rs`, `compiler/src/codegen/expr.rs` | Conditional return stores per arm and zeroes the source; a struct-literal field is an owning destination (03-02) |
| Impl/unit pins | `compiler/tests/runtime_value_integrity_impl_test.rs`, `compiler/src/own/tests.rs` | Update the two pins that assert today's no-release transfer behavior (AR #15) |

### Integration Points

- The C shape is C99: the branch store is a statement, so a conditional in a value position that
  cannot host a statement of this kind (e.g., a sub-expression) still materializes first through
  the existing `materialize_read_conditional` path.
- Move-state bookkeeping keeps the conservative join: after the statement both arms are maybe-moved
  (`E10020` on later use), consistent with the fixed-array behavior
  (`compiler/tests/runtime_value_integrity_spec_test.rs`, case `rvi30`).
- Aggregate destinations reuse the existing aggregate move (`emit_move_zero` → `cl_zero`); no new
  runtime function is added.
- Registration is release-on-discard: a temporary whose value is adopted (binding, assignment,
  argument, return, or field) is zeroed by the adopter and its release is a no-op; a discarded
  temporary is released at statement end.
- `ReleaseTemp` emission is the existing statement (`compiler/src/codegen/function.rs:423`); no new
  IR statement is added.

## Code Examples

```clorraine
func take_it(take s: String): i32 { return i32(len(s)); }
func mk(): String { return stringOf("xy"); }
func sink(take s: String): String { return s; }
func pick(ok: bool, take s: String, take t: String): String {
    return ok ? sink(s) : t;                     // return adoption
}

func main(): i32 {
    let ok: bool = false;
    let a: String = stringOf("ab");
    let b: String = stringOf("cde");
    let c: String = ok ? a : b;                  // branch store; the chosen arm moves
    let d: String = ok ? mk() : stringOf("fg"); // mixed call/place arms
    let e: i32 = take_it(ok ? sink(d) : c);     // by-value conditional argument
    let f: String = pick(ok, a, b);             // conditional return inside the callee
    return e + i32(len(f));
}
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Move-only element arm | `E10026` (unchanged); regression-pinned | AR #3 |
| Use of an arm after the conditional | `E10020` moved-value error via the conservative join (unchanged) | AR #2 |
| Transfer temp on a path that never assigned it | Zero at function entry; release of the empty state is a no-op | AR #2 |
| Adoption of a call arm | Explicit move-out zeroing at every adopting site; no double release | AR #2, AR #19 |
| Adoption in a `take` argument | Non-releasing argument temporary; the callee owns the value | AR #2, AR #19 |
| Adoption in a `return` or a struct-literal field | Per-arm store into the result/field, then zero the source; the ladder releases nothing | AR #2, AR #19 |
| Non-array owned destination (handle or aggregate) | The branch store keys on `needs_drop` and not-fixed-array, so aggregates are covered | AR #2 |

> **Traceability:** Every error-handling strategy references the Ambiguity Register entry that
> resolved it. See `00-ambiguity-register.md`.

## Testing Requirements

- Specification tests: ST-1 – ST-8 and ST-32 – ST-37 in `07-testing-strategy.md` (host runs; leak
  detection in close-out per AR #16).
- Implementation tests: emitted-C branch-store shape (empty, live, and self-naming destinations),
  arm zeroing for handles and aggregates, transfer-temp registration and release, the
  non-releasing take-argument temporary, the return store, and the struct-literal field store (see
  `07-testing-strategy.md`).
- Updated pins: the two existing tests that assert the old transfer behavior (AR #15).
