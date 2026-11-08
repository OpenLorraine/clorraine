# Current State: v1.1 Language Surfaces

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)
> **Baseline tree**: `a7dbdd5` (clean, equal to `origin/main`)
> **Baseline gate**: `make check` exit 0 on 2026-11-08; fuzzer smoke 28 cases,
> 0 failures

This document records what the compiler does at the starting tree, with a
reproduction for every gap. Each reproduction is small enough to run with the
built `target/debug/clc` and the host C compiler.

## 1. X1: only `ffi` functions are valid deleters

`compiler/src/ffi.rs:65-77` resolves the `freedBy` name through
`program.ffi_function` only. An ordinary Clorraine function with the exact
`take T -> void` signature is rejected.

**Reproduction** (`/tmp` scratch, not committed):

```clorraine
module t;

handle File freedBy CloseFile;

func CloseFile(take f: File): void {
}

func main(): i32 {
    return 0;
}
```

```text
$ target/debug/clc --profile host -o ordinary_deleter.c ordinary_deleter.clo
error E10102: 'CloseFile' is not a valid deleter for 'File'
  --> ordinary_deleter.clo:3:8
  help: declare the deleter in an `ffi` block with the right signature
```

**Root cause:** the validator is deliberately restricted to
`program.ffi_function`; it does not consult a user function declaration. The
emitter's deleter lookup (`compiler/src/codegen/drop.rs:273-298`) already
tries `resolve_function` and falls back to the raw name, so an ordinary
deleter would mangle correctly once the check accepts it. A qualified name is
flattened with `segments.join("_")` before resolution, which loses the module
boundary for a cross-module deleter.

**Consequence:** a user type cannot own a recursive resource without writing
an `ffi` block, which `X6` forbids in application code. The ergonomics study
records this as the `Box<T>` blocker (`docs/findings-ergonomics.md:81-98`).

## 2. No `Box<T>`

There is no `Box` type, constructor, accessor, or drop glue. A search of
`stdlib/` and `compiler/src/` finds only Rust `Box` uses in the compiler and
user-defined example structs named `Box` in tests. The specification stages
the type (`spec/appendix-stdlib.md:382-388`) and ties it to X1
(`spec/decisions.md:161`, `spec/lineage.md:94`).

**Consequence:** recursive data uses `raw` pointers and explicit release, as
the specification says (`spec/07-structs.md:252-267`). A safe owned pointer
does not exist.

The `Array<T>` container is the closest precedent: a compiler-known generic
struct in `stdlib/std.mem.clo` with a `Buffer` field, a special drop walk
(`compiler/src/codegen/drop.rs:47-58`), and a dot-call surface
(`compiler/src/sema/call.rs:1308-1317`). `Box<T>` can follow the same shape
with one extra rule: the pointee is dropped before the storage handle.

## 3. L5: a capturing lambda at an `ffi` callback is accepted and emits invalid C

The ordinary call path rejects a capturing lambda at a concrete `func(...)`
parameter with `E10400` (`compiler/src/sema/expr.rs:603-614`). The `ffi` call
path does **not** apply that conversion: `check_ffi_arguments`
(`compiler/src/sema/call.rs:1546-1603`) checks the argument count and the
`cstring` rule only, then walks each argument for its own errors. A capturing
lambda therefore reaches the emitter as a two-word `ClClosure_N` value where
the ABI expects a C function pointer.

**Reproduction:**

```clorraine
module t;

ffi lib {
    func each(cb: func(i32): i32): i32;
}

func main(): i32 {
    let n: i32 = 1;
    return each([n] (x: i32) => x + n);
}
```

The compiler exits 0 and emits:

```c
LONG each(LONG (*)(LONG));
...
return each(((ClClosure_0){ cl_lambda_0, &(((struct ClEnv_0){ n })) }));
```

The strict host build rejects it:

```text
error: incompatible type for argument 1 of 'each'
note: expected 'LONG (*)(LONG)' but argument is of type 'ClClosure_0'
```

This is a latent accepted-source/emits-invalid-C defect in the exact surface
L5 owns. A user `func` parameter is rejected correctly:

```text
error E10400: capturing lambda cannot be used as 'func(...)'
```

**Root cause:** the FFI argument check has no general conversion judgment and
no record of which callback parameters are synchronous.

## 4. Callback parameters have no synchronous contract

Existing `ffi` declarations that store a callback use unmarked `func(...)`
parameters:

- `stdlib/std.amiga.utility.clo:32` — `cl_new_hook(entry: func(): void, ...)`
  stores the entry in a `Hook`.
- `stdlib/std.amiga.exec.clo:96` — `cl_create_task(entry: func(): void, ...)`
  starts a task with the entry.
- `stdlib/std.amiga.interrupt.clo:31-32` — `cl_add_handler(...)` registers a
  handler.

A capturing lambda's environment is a caller-stack object. Passing one to a
storing API would leave a dangling environment pointer after the call. The
current vocabulary has no marker that distinguishes "called during this call"
from "stored". The `borrow` parameter mode is the natural marker: it already
means a call-scoped loan, and it does not change the C ABI. No existing
declaration uses `borrow` on a `func` parameter, so the new rule changes no
accepted program except the invalid-C capture case above.

## 5. Generic aggregates in `ffi` signatures emit invalid C

`compiler/src/abi.rs:55` maps every `TyKind::Struct` to `Cl<name>` without
the instance arguments. A generic instance is emitted as `Cl<type_key>`
(`compiler/src/codegen/types.rs:159-165`), so an `ffi` signature naming
`Wrap<i32>` declares `ClWrap` while no such typedef exists.

**Reproduction:**

```clorraine
module t;

export struct Wrap<T> { v: T; }

ffi lib {
    func takeWrap(take w: Wrap<i32>): void;
}

func main(): i32 {
    return 0;
}
```

Emitted C contains `void takeWrap(ClWrap);` and the strict build rejects the
prototype. This matters for `Box<T>` because a `Box<T>` in an `ffi` signature
would take the same path. The v1.1 design rejects `Box<T>` at the boundary
rather than generalising the generic-aggregate ABI.

## 6. Existing primitives the design reuses

| Primitive | Where | What it gives the new surfaces |
|---|---|---|
| Recursive drop walk | `compiler/src/codegen/drop.rs:21-131` | Handles, structs, fixed arrays, and `Array<T>` already drop recursively; `Box<T>` adds one arm |
| Deleter symbol lookup | `compiler/src/codegen/drop.rs:273-298` | User and `ffi` deleters are resolved by `resolve_function` then by raw name |
| Capturing environment | `compiler/src/codegen/program.rs:697-780`, `compiler/src/own/lower.rs:844-890` | One environment struct per capturing site, one lifted function with an `__env` first parameter |
| Callable bounds | `compiler/src/sema/call.rs:771-810` | A capturing closure already satisfies `F: func(T): R` and is called synchronously |
| Generic struct instances | `compiler/src/types/table.rs:755-823` | `Box<T>` parses and substitutes like `Array<T>` |
| Allocator | `compiler/runtime/shim_host.c:68-79`, `shim_hosted_amiga.c:689-700` | `cl_alloc`/`cl_free` already exist on host and both Amiga profiles |
| Profile module gate | `compiler/src/profile.rs:95-128` | `std.mem` is available on host and both Amiga profiles, absent on freestanding |
| FFI argument walk | `compiler/src/sema/call.rs:1546-1603` | The place to add the `func` conversion judgment and hook recording |
| Assertion/test helper | `compiler/tests/closure_integrity_spec_test.rs`, `compiler/tests/amiga_assets_impl_test.rs:82-123` | Strict host build/run and ASan patterns to copy |

## 7. Baseline evidence

| Check | Result |
|---|---|
| `git status --short` | empty |
| `git log --oneline -1` | `a7dbdd5 docs(tests): describe compiler robustness cases in plain language` |
| `git rev-list --count origin/main..HEAD` | `0` |
| `make check` | exit 0; fuzzer smoke `cases=28 host=28 sanitizer=16 target=16 failures=0` |
| Only release tag | `v1.0.0` |
| PLAN-06 / PLAN-07 | Parked; not reopened |
