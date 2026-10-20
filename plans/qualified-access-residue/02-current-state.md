# Current State

Reproduced at `5830521` with the host profile and a strict C build.

| Row | Repro | Observed | Site |
|-----|-------|----------|------|
| `DEF-397` | `1.fail()` with `func fail<T>(x: i32): T`; `1.apply(5)` with `apply<T, F: func(T): T>(x: T, f: F)` | accepted; strict C reports an implicit declaration of `cl_..._fail` and calls a non-function `f` | `sema/call.rs:705-860` (`check_generic_dot_call` never runs `E10301`, `E10305`, `check_callable_bounds`, or `check_ephemeral_bindings`) |
| `DEF-398` | `let g: func(): i32 = p.funcvalue.helper;` (self-qualified, accepted by the type stage) | accepted; emits `g = p.funcvalue.helper;`; strict C reports `'p' undeclared` | `codegen/expr.rs:545-590` (`path` resolves a single-segment function and a global path, but not a module-qualified function) |
| `DEF-399` | `std.mem.len(a)`, `std.mem.rawSlice<u8>(p, n)`, `std.mem.rawStr(p, n)`; a cross-module `ffi` call | rejected `E10219` although `std.mem.slice` works | `resolve/mod.rs:470-530` (non-own module visibility requires `module_exports`; intrinsics and `ffi` declarations are not exported); `sema/call.rs:40-80` (the qualified route only looks in `table.function`); `own/call.rs:665` (intrinsic lowering matches the full dotted name) |
| `DEF-400` | holder exports `f` and `let spec`; user imports `{ spec }` and calls `spec.cc.qcoll.lib.f()` | rejected `E10038: cannot borrow global 'spec'` in the ownership stage | `own/call_check.rs:20-120` (the field/binding checks run before the named-call branch even when the qualified name resolves a module function) |

## Test placement

| Rows | Suite |
|------|-------|
| `DEF-397` | `compiler/tests/borrow_generic_integrity_spec_test.rs` |
| `DEF-398`, `DEF-399`, `DEF-400` | `compiler/tests/checker_conformance_spec_test.rs` |

## Controls

- The accepted callable-bound dot call: `5.apply<i32, func(i32): i32>(addOne)`.
- A lexical local named like a module head still shadows the module path.
- `std.mem.slice` (already accepted) stays accepted.
- A same-module `ffi` call (the existing `should_reach_a_self_qualified_ffi_declaration`)
  stays accepted.
