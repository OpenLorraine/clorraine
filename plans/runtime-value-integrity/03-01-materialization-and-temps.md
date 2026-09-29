# Materialization and Temporary Release: Runtime Value Integrity

> **Document**: 03-01-materialization-and-temps.md
> **Parent**: [Index](00-index.md)

## Overview

This component makes compiler-materialized values real owners: a materialized temporary
that holds an owning value is released at its statement end (DEF-23), a call-rooted
element-loop iterable is materialized before the loop (DEF-24), and a fixed-array
conditional value is stored through a branch that evaluates only its chosen arm (DEF-25).
All three share one mechanism — the ownership stage tracks the temporaries it creates and
the codegen stage emits their release and their store shapes.

## Architecture

### Current Architecture

`ValueKind::Materialize` (`compiler/src/ir.rs:248-253`) wraps a value whose evaluation
must be deferred to the guarded path; `materialize_guard_operand`
(`compiler/src/own/lower.rs:523-553`) is its only constructor. Temporaries live in
`FunctionChecker::temps: Vec<Ty>` (`compiler/src/own.rs:160-161`, created by `new_temp`
at `:888-892`), are declared as empty-state C locals
(`compiler/src/codegen/function.rs:249-263`), and their assignment is emitted by the
guard emitter (`compiler/src/codegen/guards.rs:103-125`, `:286-307`). Nothing releases
them. `Statement::Drop` is a scope cleanup: `Emitter::collect_cleanup`
(`compiler/src/codegen/cleanup.rs:39-52`) collects every `Drop` into its scope's action
list and `emit_block_body`/`emit_scope_inline` render it at scope end or on a ladder
rung, so a statement-position release needs its own path. Element loops index the
iterable value inline (`compiler/src/own/loops.rs:379-408`). A fixed-array value —
including a conditional — is stored through `emit_fixed_array_assign`
(`compiler/src/codegen/function.rs:659-705`), which emits `memcpy` over the rendered value
expression.

### Proposed Changes

**1. Owning-temporary release with view lifetime (DEF-23; AR-5, AR-12, AR-18).** The
ownership walk records each *destination* materialization whose type needs a drop on the
current statement's pending list, in creation order. Only destination materializations
join the list: the guard-operand temporaries (`ValueKind::Materialize`), the new iterable
temporary, and the conditional read-position temporary. The take-call result
(`materialize_value`) and the aggregate-literal adoption temporary (`consume_place`) are
transfer temps whose destination already owns the value, so they are excluded by
construction.

At the end of the statement the walk drains the pending list in reverse creation order
through a **dedicated statement-position release**, not a scope-collected
`Statement::Drop` (which would fire at scope end). The drain runs after each statement
and, for a statement that terminates the block, before the terminator (`return`, `break`,
`continue`), so an early exit still releases exactly what the statement created. Only the
evaluated path matters: an unevaluated guard leaves the emitted temp in its empty state,
and the release machinery already treats an empty handle as a no-op (the same guarantee
that makes a moved-out binding's scope-end drop safe). `spec/17-codegen.md:80` and
`spec/decisions.md:28` (D5, O2) are the oracle; the reverse creation order is stated at
`spec/17-codegen.md:80` and `:212-213`.

**1a. A view of a released temporary cannot outlive its statement (AR-18).** A view
derived from a statement-scoped temporary is a loan whose owner is that temporary. The
statement-end release is recorded as a release event for the temporary owner, so the
existing liveness pass reports `E10030` ("borrow of '…' does not live long enough") for any
use of the view with no release-free path — the same rule that already protects a local
owner. A use inside the same statement (for example `vlen(mk()[1])`) stays accepted, and
the existing escape rules (`E10036` for a returned call-rooted view, `E10205` for a
view-typed field) mean a temporary view cannot leave the function by another route. This
extension of the existing provenance/loan records is the approved complexity surface
(AR-18; gate packet in `00-preflight-report.md`).

**2. Call-rooted iterable materialization (DEF-24; AR-6, AR-12).**
`check_for_in` (`own/loops.rs:379-408`) uses `place_target` to test whether the iterable
names a place. When it does not (a call result or another ownerless value), the walk:

- creates a temporary for the iterable type (`new_temp`),
- emits `Statement::Assign { target: temp_place, value: iterable }` before the loop, which
  the codegen fills through the existing call-destination or byte-copy shape,
- registers the temporary for the statement-end release when its type needs a drop,
- binds the loop's index base to `ValueKind::Temp(temp)`.

`spec/05-statements.md` §9 defines the loop over an indexable value, so this keeps every
accepted program compiling. `borrow` element loops alias elements of the temporary exactly
as they alias elements of a place; the temporary is released after the loop statement. A
call-rooted iterable whose type mentions a generic parameter follows the existing
`mentions_type_param` guard of `materialize_guard_operand`: it keeps today's inline
behavior because the uninstantiated body cannot spell the temporary's C type, and the
case is recorded as a limitation rather than silently declaring a temporary.

**3. Fixed-array conditional values (DEF-25; AR-7, AR-16).** The shared fixed-array value
assignment path (`codegen/function.rs`'s `emit_fixed_array_assign` and its real callers:
initializer, plain assignment, and a read position that first materializes the value)
becomes branch-aware, and the fixed-array return emitter (`emit_fixed_array_return` in
`codegen/cleanup.rs`) gets the same branch shape into `__cl_ret`. When the value is
`ValueKind::Conditional { cond, then, else }` and the destination is a fixed array, the
emitter produces

```c
if (cond) {
    /* store `then` into the destination */
} else {
    /* store `else` into the destination */
}
```

with these rules:

- The condition is evaluated once, after its guards run.
- Each arm's guards and calls run only inside its branch, preserving short-circuit
  semantics for arm calls.
- A duplicable arm is byte-copied into the destination; an owned arm is moved (byte copy
  plus zeroed source) so only the chosen source is emptied.
- With an owned destination, the existing save of the old target happens before the
  branch; an arm that names the assignment target reads the saved value, so
  `s = ok ? s : t;` keeps `s`'s old value when the condition holds and takes `t`'s value
  otherwise, releasing the old target exactly once.
- An arm that is itself a conditional recurses.

A read position that must be a place (an index base, an element source) and holds a
fixed-array conditional is materialized first: the walk emits an assignment of the
conditional into a temporary (the branch-aware store shape) and reads the temporary. The
temporary joins the statement-end release list (rule 1) when it owns. A conditional
argument whose arms are place-rooted stays inline as a forwarding value: the checker's
place-projection predicate (`projects_from_place`, `own/lower.rs:633-641`) is extended so
`materialize_owned_argument` does not bind an owning copy — the copy would double-free,
and binding it is what emits the invalid `memcpy(&binding, &(cond ? a : b))` today. A
borrow parameter then reads the caller's chosen storage, and a take parameter consumes the
chosen arm through the parameter pointer exactly as today. A conditional argument with a
call arm is out of scope (`DEF-32`): the decayed call result names no destination. An
indexed element target (`rows[i] = value;`) does not route through
`emit_fixed_array_assign`; it goes through `Statement::StorePointer`, which emits an
invalid array assignment even for a plain value, and it is out of scope here as `DEF-31`
(AR-16).

## Implementation Details

### New Types/Interfaces

- A statement-position release statement in the IR (a sibling of `Statement::Drop` that
  the emitter renders inline, so it is not collected as a scope cleanup). The name is an
  implementation detail.
- A temporary-owner form in the view/loan records (`compiler/src/own/place.rs`,
  `compiler/src/own/liveness.rs`) so a view derived from a statement temporary carries a
  loan of that temporary and its later use reports `E10030`.
- The existing `ValueKind::Materialize`, `PlaceBase::Temp`, and `Statement::Assign`
  carry the rest. A private bookkeeping field on `FunctionChecker` (pending owning
  destination temporaries for the current statement, in creation order) is expected.

### New Functions/Methods

- A checker helper that registers a destination materialization for release (creation
  order).
- A checker helper that drains the pending list at statement end and before each
  terminator, emitting the statement-position release in reverse order.
- A provenance helper that names the temporary a view derives from, so the view binding
  carries a loan of it and a later use reaches the `E10030` rule.
- A codegen helper that emits the branch-aware fixed-array store for a conditional value,
  reused by the assignment path and the return emitter.
- A checker predicate extension that treats a conditional whose arms are place-rooted as a
  forwarding value, so no owning copy is bound for an argument.
- A checker helper that materializes an iterable or a place-requiring conditional value
  into a temporary and returns its value.

### Integration Points

- `own/loops.rs` `check_for_in` uses the iterable materialization.
- `own/lower.rs` `materialize_guard_operand` and the new materialization sites register
  destination temporaries; the place-projection predicate gains the forwarding-conditional
  case; `own/statements.rs` drains the pending list after each statement and before every
  terminator.
- `own/place.rs`/`own/liveness.rs` extend view provenance and loan origins with the
  statement temporary.
- `codegen/function.rs` `emit_fixed_array_assign` gains the conditional arm and the
  inline release arm; the existing `emit_assign` routing (`function.rs:608-614`) is
  unchanged.
- `codegen/cleanup.rs` `emit_fixed_array_return` gains the branch shape for a conditional
  return; `codegen/function.rs` already renders a `PlaceBase::Temp` place and the drop
  expression, which the release reuses.
- Existing behavior to preserve: the inline `mk()[i]` projection path for a call whose
  result is *not* an iterable or a store (array-value projections stay as they are),
  `E10040` for a non-duplicable element loop without `borrow`, the transfer-temp shapes
  (take-call and literal adoption), and the existing guard/laziness tests.

## Code Examples

### Example 1: Call-rooted iterable (accepted, valid C)

```clorraine
func mk(): i32[2] { return [1, 2]; }

func main(): i32 {
    let sum: i32 = 0;
    for (x in mk()) {
        sum = sum + x;
    }
    return sum;              // 3
}
```

### Example 2: Owned conditional value

```clorraine
import { String, stringOf } from std.text;

func main(): i32 {
    let ok: bool = true;
    let a: String[2] = [stringOf("x"), stringOf("yy")];
    let b: String[2] = [stringOf("z"), stringOf("www")];
    let c: String[2] = ok ? a : b;   // a moves, b keeps owning its elements
    return i32(len(c[1]));           // 2
}
```

### Example 3: Temporary released at statement end

```clorraine
func mk(): String[2] { return [stringOf("a"), stringOf("bb")]; }

func vlen(s: str): i32 { return i32(len(s)); }

func main(): i32 {
    return vlen(mk()[1]);   // the call result's elements are released after this statement
}
```

### Example 4: A view of a temporary lives only within its statement

```clorraine
func mk(): String[2] { return [stringOf("a"), stringOf("bb")]; }

func main(): i32 {
    let n: i32 = i32(len(mk()[0]));   // accepted: the view is used in the same statement
    let v: str = mk()[0];             // accepted: the binding carries the temporary's loan
    return i32(len(v));               // E10030: the use follows the temporary's release
}
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Owning temporary left unreleased | Statement-position release in reverse creation order; empty state makes an unevaluated temp's release a no-op | AR-5, AR-18 |
| A view derived from a released temporary used after its statement | Statement-scoped temporary loan plus the existing `E10030` liveness rule | AR-18 |
| A transfer temp included in the release set | Excluded by construction: only destination materializations register; a unit test pins the sites | AR-18 |
| Call-rooted iterable | Materialized; accepted programs compile per `spec/05` §9 | AR-6 |
| Call-rooted iterable whose type mentions a generic parameter | Keeps today's behavior under the `mentions_type_param` guard; recorded as a limitation | AR-6 |
| Non-duplicable element loop without `borrow` | Unchanged `E10040` | AR-12 |
| Owned conditional arm | Move (copy plus zeroed source); unchosen arm untouched | AR-7 |
| Conditional arm that names the assignment target | Resolves to the saved old target value | AR-7 |
| Fixed-array conditional in a read position | Materialized into a temporary first; the temporary joins the release list | AR-16 |
| Fixed-array conditional in a return | The fixed-array return emitter stores the chosen arm into `__cl_ret` | AR-16 |
| Conditional argument with place arms | Stays inline as a forwarding value; the place-projection predicate is extended so no owning copy is bound | AR-16 |
| Conditional argument with a call arm | Out of scope; the decayed call result names no destination, filed as `DEF-32` | AR-16 |
| Indexed fixed-array element target | Out of scope; `Statement::StorePointer` invalid today, filed as `DEF-31` | AR-16 |
| Conditional arm calls | Arm guards and calls emitted inside the chosen branch only | AR-7 |

> **Traceability:** Every error-handling strategy and design choice references the
> Ambiguity Register entry (AR #) that resolved it. See `00-ambiguity-register.md`.

## Testing Requirements

- Specification tests ST-1 – ST-11 and ST-24 – ST-29 in `07-testing-strategy.md`.
- Implementation tests pin the emitted shape: an inline release for an owning
  `__cl_tmpN`; the iterable temporary; the branch store for a conditional in both the
  assignment and return positions; per-arm guards; the absence of a release for the
  take-call and literal-adoption transfer temps.
- Unit coverage pins the registered release set to the destination materialization sites
  and the temporary-view loan registration.
- The close-out phase runs the host probes under AddressSanitizer with **leak detection
  on** for the owning temporary cases (the prior plan ran them with leak detection off).
