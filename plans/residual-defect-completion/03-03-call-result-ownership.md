# Call-Result Ownership: Residual Defect Completion

> **Document**: 03-03-call-result-ownership.md
> **Parent**: [Index](00-index.md)

## Overview

This component fixes DEF-55 (an owning read of a projection from a fresh call result leaks the
result's siblings) and DEF-53 (an ownerless element take under a maybe-owning substitution frees
the same storage twice). Both arise because a call result has no C place that outlives the
statement, yet an owning read takes a part of it without ever binding or releasing the whole.

The approved mechanism is one: bind the ownerless call result to an owned temporary, move the
projection out of it, and release the temporary for the rest (AR-4).

## Architecture

### Current Architecture

- `return mkv().a` is emitted inline: `return cl_mkv().a;`; nothing releases `mkv()`'s other
  owning fields. The `let` and take-argument forms leak the same way (02-current-state §Reproductions).
- For a `take` argument, `check_argument_place` treats a maybe-owning type as "record the duty
  marker" rather than a move (`compiler/src/own/call_check.rs:281-305`); with no place to mark
  (`is_ownerless_element_move`, `compiler/src/own/expr.rs:201-217`) nothing is recorded, and
  under an owning substitution the callee releases the element while the call temporary releases
  the same storage.
- The ownership stage already materializes ownerless values for loop iterables and guarded
  operands; the previous plan added the ownerless maybe-owning non-take binding
  (residual-defect-hardening Phase 4). This plan extends the same materialization to owning
  projection reads and to the element-take case.

### Proposed Changes

1. **Bind before an owning projection read.** When an owning read's root is a call result and the
   read selects a field or element (a projection), lower the call result into a fresh owned
   temporary local first, rewrite the projection to read from the temporary, and move the
   projection out. The temporary's scope-end release drops the remaining fields/elements; the
   moved part is emptied from the temporary by the ordinary move rules.
2. **Apply the same binding to the maybe-owning element take.** For a `take` argument that is an
   element of an ownerless call result under a maybe-owning substitution, lowering binds the call
   result to an owned temporary, re-roots the element read through it, and emits the element
   move-out after the call. The callee then owns the element it receives and the temporary's
   scope release frees only the remaining elements. The ownership checker is unchanged: checking
   runs before lowering, so it cannot see the bound temporary, and its maybe-owning suppression is
   what keeps the generic body accepted. The concrete non-generic element take keeps reporting
   `E10026` (R7).

The binding is a normal owned local of the call result's type with an initializer; no new IR
statement is needed if the existing materialization machinery can produce it, otherwise the
smallest existing statement form is reused.

## Implementation Details

### Changed functions

- `compiler/src/own/lower.rs` — the projection-read path and the maybe-owning element-take
  argument path materialize the call-result root into an owned temporary before the read/move,
  and the take path emits the element move-out after the call so the temporary's scope release
  drops only the remaining elements. The take binding must not be gated behind the speculative
  pass, or a deferred path can lose the move-out. The materialized binding participates in the
  scope's release set like any owned local.
- `compiler/src/own/call_check.rs` — no change: the maybe-owning suppression stays, because
  checking runs before lowering and cannot see the bound temporary. The concrete-case `E10026`
  reporting is untouched.
- `compiler/src/own/expr.rs` — no change for the take path: `is_ownerless_element_move` stays as
  the diagnostic predicate for the concrete case; the maybe-owning binding is a lowering
  responsibility.

### Integration Points

- The scope release machinery drops the bound temporary at scope end; the moved projection is
  zeroed on the temporary by the existing move-out emission, so the release cannot double-drop.
- The return path, the `let` binding path, and the call-argument path all reach the projection
  read through the same lowering helper; one binding site covers all three forms.
- Checking runs before lowering in this stage (`compiler/src/own/statements.rs:220` versus
  `:235-302`), so the binding is a lowering responsibility and the checker's maybe-owning
  suppression is retained. The take path must emit the element move-out itself; the binding alone
  would leave the element in the temporary and double-free it.
- The `E10026` diagnostic stays owned by the ownership stage and its message and code are
  unchanged for the concrete case.

## Code Examples

```clorraine
struct Pair { a: String; b: String; }
func mkv(): Pair { return Pair { a: stringOf("x"), b: stringOf("yz") }; }

func takea(): String { return mkv().a; }   // binds mkv(), moves .a, releases .b
```

```clorraine
func sink<T>(take s: T): i32 { return 0; }
func at<T>(mk: func(): T[2], i: i32): i32 { return sink<T>(mk()[i]); }
func main(): i32 { return at<String>(mkString, 0); }   // element taken once; other element released
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Projection of a projection (`mkv().a.b`) | The root call result is bound once; the read walks the same projection chain from the temporary | AR-4 |
| The call result is a view or duplicable | No binding (no owning duty); behavior unchanged | AR-4 |
| The projection is not moved (a read view) | The binding still releases the whole temporary at scope end; the read derives its view from the temporary | AR-4 |
| Concrete element take (`sink(mkv()[0])`, non-generic) | `E10026` unchanged; compatibility case pins it | AR-4, R7 |
| The bound temporary is never moved from | Its scope release drops it exactly once | AR-4 |

> **Traceability:** Every error-handling strategy references the Ambiguity Register entry. See
> `00-ambiguity-register.md`.

## Testing Requirements

- Specification cases ST-10 – ST-14 (07-testing-strategy.md): `return`, `let`, and take-argument
  field projections; the maybe-owning element take; the concrete `E10026` compatibility case.
- Implementation tests: sanitizer runs for the leak and double-free cases, emitted binding shape,
  single release counts, and `E10026` unchanged.
