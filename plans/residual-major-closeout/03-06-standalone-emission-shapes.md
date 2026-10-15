# Standalone Emission Shapes: Residual Major Closeout

> **Document**: 03-06-standalone-emission-shapes.md
> **Parent**: [Index](00-index.md)

## Overview

Four independent single-site rows: the imported/qualified func-global callee symbol (`DEF-170`,
batch E), the freestanding `_start` stack alignment (`DEF-189`, batch E's emission re-audit),
the fixed-array `sizeof` value (`DEF-211`, batch F's Phase 4 review), and the void
expression-bodied lambda over an abort intrinsic (`DEF-255`, the def-backlog-closeout
close-out). Each is a bounded emission/judgment fix at an existing site; none shares machinery
with the others or with P1–P5 (the phase keeps them together as the standalone residue, AR-4).

## Design

### 1. The imported or qualified func-global callee (`DEF-170`)

`import { g } from lib; g(5)` emits `g(...)` with the global's C symbol undeclared; the
qualified `lib.g(5)` spelling and the imported struct global's field call (`h.call(6)` emits
`h`) fail identically. The func-global resolution (`codegen/expr.rs:773-782`) and the callable
type lookup (`codegen/function.rs:950-960`) consult the globals only when
`id.module == current_module`, so a foreign identity never resolves; `callee_name` keeps the
written simple name and the emitter spells it bare (AR-3 probe).

Thread the global's identity into the emitted callee spelling for every spelling of the name:
the import binding and the qualified path carry the target module, so the resolution matches
the global by (module, name) — not by the current module — and renders through the existing
`global_symbol(module, name)` (the module-mangled C symbol). The same identity resolution
covers the struct global's field call. The current-module spellings and the pinned
func-global cases are byte-identical controls.

### 2. The freestanding `_start` alignment (`DEF-189`)

The freestanding host no-libc run of a 16-byte-aligned `str[2]` shape segfaults at a `movaps`
in `main` (run 139) while the m68k link is clean; the shim's `_start` calls `main()` directly
(`compiler/runtime/shim_freestanding.c:304-307`), leaving the host's call-sequence alignment
state wrong for `main`'s 16-byte stores (AR-3 probe). `DEF-234`'s `str[2]`/`len` unit fails
identically on the same root (out of scope, recorded).

Align the stack immediately before the `main` call at the `_start` entry, in the
target-conditional form the single shim file allows (an x86-64 alignment step to the 16-byte
boundary the ABI's `main` expects; a no-op where the target's alignment is already satisfied,
for example m68k's 4-byte placement). The host no-libc run of the probed shape exits with the
program's code, the m68k link stays clean, and the other freestanding host cases
(`println`, `exit7`, the aggregate and float programs) stay green.

### 3. The fixed-array `sizeof` (`DEF-211`, AR-7)

`let n: usize = sizeof(i32[2]);` emits `n = sizeof(i32);` — 4 where the operand type is 8 —
because `c_type` folds a fixed array to its element (`codegen/types.rs:98-103`) and the
intrinsic renders that type (`codegen/calls.rs:343-344`).

Render the whole type's size: for a fixed-array operand, the element's size times the extents
(or an equivalent whole-type spelling), so `sizeof(i32[2])` is 8, `sizeof(i32[2][3])` is 24,
and a first-extent-only array type uses its named extent. A fixed array with an omitted extent
(a view) keeps the existing `ClSlice` type path; scalar, struct, and `offsetof` spellings are
byte-identical controls.

### 4. The void expression-bodied lambda over an abort intrinsic (`DEF-255`)

`let f: func(): void = () => assert(false, "nope");` is accepted and the lifted body emits
`return 0;` in a void function (the strict build fails `'return' with a value`; with the
warning suppressed the run exits 0 instead of aborting). `() => panic("boom")` is identical;
the block-bodied twin aborts correctly. `lift_lambda`'s expression arm (`own/lower.rs:954`)
lacks a void judgment, the intrinsic renderer has no `assert`/`panic` arm
(`codegen/calls.rs:275`), and `emit_return` (`codegen/cleanup.rs:373`) emits the value return.

Judge a void intrinsic statement in the expression body: when the lambda's declared return type
is void and the body expression is an intrinsic statement of the `assert`/`panic` family (the
frozen void/never results), lower it as the statement form — the intrinsic emits its trap
(`E11213` for a false assert; the panic abort) with no cleanup ladder — and the lifted body
ends without a value return (`return;` or the abort's terminator). A void expression body of
any other void intrinsic/statement follows the same statement lowering; a non-void expression
body keeps the value return. The block-bodied twin and the `return panic(...)` rejection
(`E10217`) stay pinned controls.

## Integration Points

- `codegen/expr.rs:773-782`, `codegen/function.rs:950-960` — the func-global resolution;
  `codegen/program.rs::global_symbol` — the mangled spelling.
- `compiler/runtime/shim_freestanding.c:292-307` — the entry section.
- `codegen/calls.rs:343-344` (the `sizeof` arm), `codegen/types.rs:79-105` (`c_type`).
- `own/lower.rs:954` (`lift_lambda`), `codegen/calls.rs:275-350` (`intrinsic`),
  `codegen/cleanup.rs:373` (`emit_return`).

## Error Handling

| Error Case | Handling Strategy | AR Ref |
|------------|-------------------|--------|
| A func-typed global name that resolves to no identity | The existing name-resolution error path | — |
| A shim `_start` on a target where the alignment step is unnecessary | The conditional is a no-op; the m68k link and runs stay green | — |
| A view-typed `sizeof` operand | The existing slice/view type path (unchanged) | — |
| A void expression body of a non-void intrinsic | The existing expression judgment (unchanged) | — |
| A non-void lambda body over an abort intrinsic | `E10217` (the pinned rejection) | — |

## Testing Requirements

- Spec: the import and qualified func-global calls and the struct global's field call (strict
  build, run, the summed value; the current-module func-global control), the freestanding
  `str[2]` host run and m68k link (plus the `println`/`exit7` controls), the fixed-array
  `sizeof` values (2-D and omitted-extent variants; the scalar/struct controls), and the void
  lambda over `assert`/`panic` (strict build; the run aborts with the frozen fault message;
  the block-bodied and `return panic(...)` controls).
- Impl: the mangled callee spellings, the `_start` alignment shape, the emitted `sizeof`
  expression, the void return shape (`return;` with the trap), and a re-emission determinism
  pin; the non-array/non-void controls byte-identical.
