# Func Pointers and Callable Resolution: Call Value Integrity

> **Document**: 03-03-func-pointers-and-callable-resolution.md
> **Parent**: [Index](00-index.md)

## Overview

Fix DEF-30 and DEF-43 (func-pointer declarators built inline or in return position), DEF-40 and
DEF-41 (calls by name through captures and module globals), DEF-42 (no `String` → `str` widening for
func-value arguments), and DEF-44 (the `borrow` mode is dropped from func-type parameters). The
design extends the shared declarator (the DEF-27 fix), the callable spelling/resolution path, and
the type table.

**Decisions per:** AR #4 (implement `borrow`), AR #12 (widening source), AR #14 (`FuncParam`).

## Architecture

### Current Architecture

- `func_pointer_declarator` (`compiler/src/codegen/types.rs:66`) and `func_pointer_signature`
  (`:78`) build named and hidden-destination function-pointer spellings; `declarator` (`:170`)
  routes `TyKind::Func` to them. The DEF-27 fix uses it for callable struct fields.
- A function declaration still spells its return type with `c_type` in its own declarator
  (`compiler/src/codegen.rs:401-428`), so `func(): func(i32): i32` emits an unnamed pointer prefix.
- A lifted lambda's signature is built inline (`compiler/src/codegen/function.rs:198-219`), and a
  closure struct's `fn` field is built inline with `c_type`
  (`compiler/src/codegen/program.rs:267-281`), so both bypass the shared declarator.
- A call by name lowers to the source name (`compiler/src/own/call.rs:148-158`) and the emitter
  scans locals and parameters only (`func_binding_spelling`,
  `compiler/src/codegen/expr.rs:494-517`); captures and module globals are not consulted.
- Argument widening reads declared function signatures (`compiler/src/codegen/calls.rs:129-189`
  via `callee_param_types`, `compiler/src/codegen/function.rs:606-627`).
- The parsed parameter mode is dropped at resolution (`compiler/src/types/table.rs:558-564`);
  `FuncTy` has no mode field (`compiler/src/types/mod.rs:78-85`).

### Proposed Changes

1. **Return-position declarators (DEF-43).** The function declaration builder uses the declarator
   machinery for its return type; a func-typed return nests the pointer declarator
   (`i32 (*cl_make(void))(i32)`), and a fixed-array return uses the hidden-destination signature.
2. **Closure and lifted-lambda signatures (DEF-30).** One signature builder takes an optional
   environment prefix plus the parameter types and return type, and returns the full declarator
   with the hidden destination appended for a fixed-array return (the environment parameter comes
   first, the destination last). The closure struct's `fn` field and the lifted lambda's own
   signature both call that builder, so the spellings cannot drift. A fixed-array-returning lifted
   function sets `result_var = __cl_ret` (the destination parameter), and its call sites
   (`ClosureCall`, `compiler/src/codegen/expr.rs:539`) pass the destination after the environment,
   mirroring the named-call `call_with_destination` path. `func_pointer_signature` stays the plain
   function-pointer form (no prefix) and gains the mode-aware parameter spelling from change 5.
3. **Capture and global resolution (DEF-40, DEF-41).** Lowering resolves a call by name to an
   environment field when the name is a lambda capture (`sema.closures`, the existing environment
   field spelling at `compiler/src/codegen/expr.rs:167-178`) and to the global symbol when the name
   is a module-level func-typed global (`global_type`/`global_symbol`). The emitter's
   `func_binding_spelling` grows the same two lookups. The call-site argument renderer receives the
   resolved callee `TyKind::Func` (local, capture, global, or receiver value) so result types,
   expected parameter types, and modes come from that value rather than from a name lookup.
4. **Func-value widening (DEF-42).** `callee_param_types` returns the func type's parameter types
   and modes when the callee expression has function type; the existing widening path
   (`compiler/src/codegen/calls.rs:168-189`) then emits the same conversions sema accepted
   (`compiler/src/sema/expr.rs:265-304`, `implicit_conversion`,
   `compiler/src/types/mod.rs:529-548`). The current signature `call_arguments(callee: &str, ...)`
   fetches by name and cannot see a func-typed local, capture, or field, so the resolved function
   type is passed in from the call site. Declared-function and FFI callees keep today's path.
5. **Borrow modes (DEF-44).** `FuncTy.params` becomes `FuncParam { ty, mode }` (AR #14). Resolution
   carries the parsed mode from the inline form, the callable-bound form
   (`compiler/src/types/table.rs:497-510`), the declared-signature form (`FuncSig::ty`, `:47-55` —
   it zips `modes` with `params`, or a declared `nudge` cannot satisfy ST-24), and
   `resolve_ty_with`'s `TypeKind::Function` arm (`:558-564`). The named-parameter C rule (a `borrow`
   parameter is the pointer to the value) is applied in `func_pointer_signature`; call sites pass
   the address for a `borrow` parameter, with the modes read from the resolved callee function type
   at the call site (`compiler/src/codegen/calls.rs:129-156`, `compiler/src/own/call.rs:316-325`).
   Function-type equality is exact, including the mode: `func(Entity)` and `func(borrow Entity)` are
   different types and a mismatch reports the existing `E10210` (`compiler/src/sema/expr.rs:278-291`).
   The `FuncParam` change touches every `TyKind::Func` consumer — `types/mod.rs` substitution,
   display, `contains_*`, and `implicit_conversion`; `sema/{expr,call,mod}.rs`;
   `own/{lower,generics}.rs`; `abi.rs`; and the declarator builders — all compiler-enforced and
   updated in task 3.2.4. The E10210 display renders the mode so a mismatch is intelligible.

## Implementation Details

### Changed Functions

| Function | File | Change |
| -------- | ---- | ------ |
| Function declaration builder | `compiler/src/codegen.rs` | Return type through the declarator machinery |
| Shared signature builder | `compiler/src/codegen/types.rs` | Optional environment prefix + hidden destination; returns the declarator text |
| Lifted lambda signature / `result_var` | `compiler/src/codegen/function.rs` | Environment-prefixed declarator; hidden destination; `result_var = __cl_ret` |
| Closure struct builder | `compiler/src/codegen/program.rs` | `fn` field through the shared builder |
| Closure call sites | `compiler/src/codegen/expr.rs` | Pass the destination after the environment for a fixed-array result |
| Named-call lowering | `compiler/src/own/call.rs` | Captures and globals resolved to fields/symbols |
| `func_binding_spelling` | `compiler/src/codegen/expr.rs` | Captures and globals in the lookup order |
| `callee_param_types` / `callee_modes` | `compiler/src/codegen/function.rs` | Read modes/types from the callee's `TyKind::Func` |
| Resolution (`FuncSig::ty`, `resolve_ty_with`, callable bounds) | `compiler/src/types/table.rs` | Zip modes with params into `FuncTy` |
| `FuncTy` + consumers | `compiler/src/types/mod.rs`, `compiler/src/sema/{expr,call,mod}.rs`, `compiler/src/own/{lower,generics}.rs`, `compiler/src/abi.rs` | `FuncParam { ty, mode }`; exact-signature equality; mode in display |
| `func_pointer_signature` | `compiler/src/codegen/types.rs` | Mode-aware parameter spelling |
| Call argument types/modes | `compiler/src/codegen/calls.rs`, `compiler/src/own/call.rs` | Resolved callee type in; `&` for func-value borrow parameters |

### Integration Points

- ABI stability: the hidden-destination rule and the m68k pointer spellings stay exactly the DEF-27
  forms; the new code reuses them.
- Diagnostics unchanged: a signature mismatch stays `E10210`; no new diagnostic is added. The
  mismatch comparison now includes the parameter mode.
- The `FuncParam` change is internal but cross-cutting: `c_type` value-position spellings keep
  their current text, while type equality distinguishes modes and E10210's display renders them.
  Task 3.2.4 lists every `TyKind::Func` consumer.

## Code Examples

```clorraine
func make(): func(i32): i32 { return double; }        // DEF-43
let g: func(i32): i32[2] = (x: i32) => { return [x, x]; }; // DEF-30
let direct: i32 = ((x: i32) => { return [x, x + 1]; })(5)[1]; // DEF-30 direct invocation

struct Ops { run: func(borrow Entity): void; }        // DEF-44
let ops: Ops = Ops { run: nudge };
ops.run(entity);                                       // passes the pointer

func use(s: String): void { }
let show: func(str): void = print_str;
show(stringOf("hi"));                                  // DEF-42: widened to str
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Signature mismatch (including a mode difference) | `E10210` unchanged; display renders the mode | AR #4, AR #14 |
| Widening not accepted by sema | Not emitted; no new conversion | AR #12 |
| Capture/global unresolved | Existing undeclared-name diagnostic | AR #12 |
| `borrow` func-type parameter | Pointer spelling and call-site address from the resolved callee type | AR #4, AR #14 |
| Fixed-array closure return | The shared builder's hidden destination; call sites pass it after the environment | AR #9 (03-02 context) |
| `take` mode in a func type | Representable after the `FuncParam` change but has no ownership semantics; recorded in Won't Have | AR #14 |

> **Traceability:** Every error-handling strategy references the Ambiguity Register entry that
> resolved it. See `00-ambiguity-register.md`.

## Testing Requirements

- Specification tests: ST-18 – ST-25, ST-39, and ST-40 in `07-testing-strategy.md` (host runs;
  emitted C must build with the pinned C compiler).
- Implementation tests: declarator shapes for the return, closure-field, lifted-lambda, and direct
  invocation positions; capture/global spelling; widening in the emitted C; the `borrow` pointer
  spelling; and the exact-signature mode-mismatch rejection.
