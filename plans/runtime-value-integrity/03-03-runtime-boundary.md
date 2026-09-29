# Runtime Boundary: Runtime Value Integrity

> **Document**: 03-03-runtime-boundary.md
> **Parent**: [Index](00-index.md)

## Overview

This component fixes the two runtime-boundary defects: a `panic` message that is a `str`
view or a `String` handle must reach the C runtime (DEF-28), and a `take` argument inside
a short-circuit operand must move its value only when that operand is evaluated (DEF-29).

## Architecture

### Current Architecture

`panic` lowers to a `Statement::Call` with callee `panic`
(`compiler/src/own/statements.rs:156-172`) and the codegen arm renders
`cl_panic(<message>)` (`compiler/src/codegen/function.rs:453-460`). `panic_message`
(`:527-532`) passes a string literal as an escaped C string and every other argument as
its rendered spelling: `str` renders `ClStr` (`codegen/types.rs:24`) and `String` renders
`ClString` (`:87`; `typedef void *ClString` at `codegen/program.rs:98`), while `cl_panic`
takes `const char *` in all three shims
(`shim_host.c:47`, `shim_hosted_amiga.c:79`, `shim_freestanding.c:218`). A `String` keeps
a trailing NUL (`spec/08-arrays-strings.md:179`) with `cl_string_ptr`/`cl_string_len`
accessors; a `str` view has no NUL guarantee.

A `take` argument lowers to a materialized value plus an eagerly emitted
`Statement::MoveOut` (`compiler/src/own/lower.rs:95-107`, `own/call.rs:308-322`).
The short-circuit constructs lower their operands as ordinary values (`&&`/`||` at
`own/lower.rs:114-133`, `?:` at `:149-157`) and the emitter defers calls through
`ValueKind::Materialize`/`guarded_value` (`codegen/guards.rs:286-307`,
`codegen/expr.rs:55-82`). A materialized take call's move-out is emitted before the guard,
so it runs even when the operand is skipped.

### Proposed Changes

**1. Panic messages carry pointer and length (DEF-28; AR-10, AR-17).**

- All three shims change `cl_panic` to take `(const u8 *message, usize len)` and write
  exactly those bytes with a length-based writer — no `%s`/`%.*s` formatting, so an
  embedded NUL byte is printed as a byte and not silently truncated:
  - host (`shim_host.c`): write the bytes to `stderr` with `fwrite` (or an equivalent
    bounded loop), then `fflush` and `abort`;
  - hosted-amiga (`shim_hosted_amiga.c`): use the shim's own length-based output (the
    same style as its console helpers) rather than a `%.*s` conversion whose libnix
    support is unverified, then halt;
  - freestanding (`shim_freestanding.c`): extend `cl_report`/`cl_write_bytes` usage to
    write the prefix and up to the available buffer, then halt; a message longer than the
    fixed shim buffer is truncated (an abort path cannot allocate), and the truncation is
    the documented behavior.
  This keeps the `spec/17-codegen.md:344` behavior (print then exit/abort) and drops the
  NUL assumption. The m68k legs verify the new writer through `make verify`.
- `panic_message` returns the two spelled arguments:
  - a string literal: `((const u8 *)"<escaped>", <byte length>)`;
  - a `str` value: `((const u8 *)(<view>).ptr, (usize)(<view>).len)`;
  - a `String` value: `((const u8 *)cl_string_ptr(<handle>), (usize)cl_string_len(<handle>))`.
- A message value that is not already a place or temporary is rendered once through a
  materialized temporary, so a side-effecting message expression evaluates exactly once.
- Other argument types keep today's behavior (AR-17). `assert` is unchanged: its codegen
  arm ignores the optional message and traps with the canonical text.

**2. Deferred take move-outs (DEF-29; AR-11).**

- The ownership walk carries a "defer move-outs" context for the operands of `&&`, `||`,
  and `?:`. Inside that context, a call with `take` arguments does not emit
  `Statement::MoveOut`; instead the move-out places travel on the materialized value
  (an extension of `ValueKind::Materialize`, e.g. a move-out list beside `temp` and
  `value`).
- The guard emitter (`codegen/guards.rs`, both the statement form and the embedded comma
  form) emits the carried move-outs immediately after the call/materialization on the
  evaluated path only.
- Outside the context, the existing eager emission is unchanged, so ordinary statements
  keep their current shape.
- Semantics: with `ok && sink(s)` and `ok == false`, the call and the move do not run and
  `s` stays owned; with `ok == true`, the call runs and `s` is moved exactly once. The
  same applies to both arms of `?:`.

## Implementation Details

### New Types/Interfaces

- `ValueKind::Materialize` gains the deferred move-out places (a `Vec<Place>` field or a
  sibling variant; the name is an implementation detail). Every walker of `ValueKind`
  (`own/generics.rs`, `own/instance_check.rs`, `codegen/function.rs`, `ir/guards.rs`) is
  updated for the new field.

### New Functions/Methods

- A helper that moves a take call's move-out places onto its materialized value when the
  deferral context is active.
- The guard emitter's move-out emission on both guarded shapes.
- `panic_message` returns a rendered argument pair; a helper renders a `str` or `String`
  view/handle once through a temporary when needed.

### Integration Points

- `own/lower.rs` sets and clears the deferral context while lowering the guarded operands.
- `own/call.rs` `take_move_out_places` supplies the places.
- `codegen/guards.rs` is the single place that decides guarded emission for both shapes.
- `codegen/function.rs` `panic` arm uses the new message rendering.
- The three runtime shims change together; no emitted program changes its include set.

## Code Examples

### Example 1: A `str` panic message

```clorraine
func label(): str { return "boom"; }

func main(): i32 {
    panic(label());        // prints `panic: boom` and aborts
}
```

### Example 2: A guarded take argument

```clorraine
import { String, stringOf } from std.text;

func sink(take s: String): bool { return len(s) > 0; }

func main(): i32 {
    let s: String = stringOf("keep");
    let ok: bool = false;
    let skipped: bool = ok && sink(s);   // the call and the move do not run
    return i32(len(s));                  // 4: s is still owned here
}
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| `str`/`String`/literal panic message | Pointer + length rendering; the runtime writes the exact bytes with a length-based writer | AR-10 |
| Embedded NUL byte in a message | Printed as a byte (no `%s`/`%.*s` conversion) | AR-10 |
| Message longer than the freestanding shim buffer | Truncated by the halting writer; documented behavior | AR-10 |
| Side-effecting message expression | Rendered once through a temporary | AR-10 |
| Other panic message types | Unchanged (no new diagnostic) | AR-17 |
| Guarded `take` call not evaluated | No call, no move; the value stays owned | AR-11 |
| Guarded `take` call evaluated | Call plus exactly one move on that path | AR-11 |
| Unguarded `take` call | Existing eager shape unchanged | AR-11 |

> **Traceability:** Every error-handling strategy and design choice references the
> Ambiguity Register entry (AR #) that resolved it. See `00-ambiguity-register.md`.

## Testing Requirements

- Specification tests ST-18 – ST-23 in `07-testing-strategy.md`.
- Implementation tests pin the emitted shape: the two-argument `cl_panic` call for each
  message form, the move-out inside the guarded path (and its absence outside), and the
  once-only message rendering.
- The panic run cases assert the process aborts with `panic: <message>` on stderr; the
  guarded-take run cases assert the value is usable when the operand is skipped and moved
  when it is evaluated.
