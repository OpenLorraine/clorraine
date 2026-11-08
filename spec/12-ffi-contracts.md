# Chapter 12 - FFI and Ownership Contracts

> **Status:** stable (v1.0)
> **Depends on:** Ch 03 (ownership), Ch 06 (functions and parameters), Ch 08 (arrays and strings), Ch 14 (diagnostics), Ch 17 (codegen)
> **Grammar:** [grammar.ebnf.md](grammar.ebnf.md) sections 3.4 (handles), 3.6 (FFI declarations), 4 (types), 5 (statements)
> **Ledger:** [decisions.md](decisions.md) section 11 (X1-X6); plus 5 (O10), 6 (T2, T8), 7 (F9), 15 (P5)
> **Validated by:** Phase 0 hosted skeleton ([Phase 0 findings](../docs/findings-phase0.md))

Clorraine calls the C ABI. On the hosted profile that means the AmigaOS
libraries. A C header gives a name and C types, but it does not describe
ownership:

* Does `Open` return something the caller must release?
* Does `Write` keep the buffer after it returns?
* Is a zero handle a valid handle or a failure?

Clorraine supplies that missing half with two declarations: an `ffi` block for
the signature and its parameter modes, and `handle T freedBy F;` for the
lifecycle. Together they are a **contract**.

## 1. Purpose and trust model

A contract is a trusted declaration. The compiler checks Clorraine code against
it; the compiler cannot verify the external implementation. This is the same
trust model as an unsafe extern boundary in another systems language, and it is
why contract tables are small, reviewed, and versioned.

| Rule | Statement | Diagnostic |
|---|---|---|
| CN-1 | A contract is a trusted declaration; the compiler does not verify the C implementation | - |
| CN-2 | The compiler checks Clorraine code against the declared modes and types | - |
| CN-3 | A user-provided contract table is reported so audits can find it | W10005 |

## 2. Declaring a contract

An `ffi` block declares external functions with ordinary Clorraine signatures,
including the ordinary parameter modes. An owned resource is declared once,
where it lives, with `handle T freedBy F;`.

```ebnf
ffi_block   = [ "export" ] , "ffi" , identifier , "{" , { ffi_decl } , "}" ;
ffi_decl    = "func" , identifier , "(" , [ param_list ] , ")" , ":" , return_type , ";" ;
handle_decl = [ "export" ] , "handle" , identifier , "freedBy" , qualified_name , ";" ;
```

Ownership is carried by the type and the mode, not by clause words:

| Form | Meaning |
|---|---|
| returns `T` or `T?` | the caller receives an owned handle; `T?` may be empty on failure |
| `take x: T` | ownership transfers into the call |
| `const x: T` | read-only lend for the duration of the call |
| `borrow x: T` | writable lend for the duration of the call |
| `x: T` on a handle or aggregate | no ownership contract (uncontracted); calling requires `unsafe` |
| `x: T` on a scalar, `raw`, `cstring`, or view | an ordinary value or pointer argument; an unmarked view is a read-only lend |
| `T?` | nullability; `null` is the failure sentinel |

The mode is semantic only: it records who owns and releases the value, and it
never shapes the C prototype. The C parameter type comes from the declared type
through the FFI ABI mapping in [Ch 17](17-codegen.md) section 3. An unmarked
handle or aggregate parameter is not a read-only lend here; it is uncontracted
(section 3). An unmarked view parameter (`T[]`, `str`) is a read-only lend, so
it is contracted and needs no `unsafe`. The draft's clause keywords are retired.
Nullability lives in the type (`File?`), never in a clause (X3).

| Rule | Statement | Diagnostic |
|---|---|---|
| CN-4 | FFI declarations use ordinary signatures and modes; nullability lives in the type | - |
| CN-5 | `handle T freedBy F;` declares an opaque resource once, in the library that owns it | - |
| CN-6 | `F` is a function taking `take T` and returning `void`: an `ffi` declaration or an ordinary Clorraine function resolved module-scoped (X1). Its own `take T` parameter is excluded from the implicit scope drop, and the body must pass it to a call | E10102 |
| CN-7 | Exactly one deleter per handle type; a second `handle` declaration for the same type is a duplicate declaration | E11101 |
| CN-8 | A native struct never declares `freedBy`; it drops structurally | E10104 |
| CN-22 | A direct `Box<T>` FFI crossing is rejected; use `boxWith` or a raw pointer | E10105 |

The handle type is public, so it is exported. The `ffi` blocks stay
module-private (M9). Declarations resolve globally within a module, so the
order of the handles and the blocks is not significant.

```clorraine
// Declared once, in the library that owns each resource (X1).
export handle Library freedBy CloseLibrary;
export handle Memory  freedBy FreeVec;
export handle File    freedBy Close;

// Module-scoped, never exported (M9).
ffi exec {
    func OpenLibrary(name: cstring, version: u32): Library?;
    func CloseLibrary(take base: Library): void;
    func AllocVec(size: u32, flags: u32): Memory?;
    func FreeVec(take mem: Memory): void;
}

ffi dos {
    func Open(name: cstring, mode: i32): File?;        // BPTR; null means failure
    func Close(take file: File): void;
    func Write(borrow file: File, const buf: u8[], len: i32): i32;
    func Read(borrow file: File, borrow buf: u8[], len: i32): i32;
}
```

`OpenLibrary`, `AllocVec`, and `Open` return `T?`, so a failure is an empty
value. `CloseLibrary`, `FreeVec`, and `Close` take ownership with `take`.
`Write` reads the buffer through a `const` lend; `Read` writes to it through a
`borrow` lend. The file handle is lent mutably, because both calls advance the
position stored in the handle. Every handle or aggregate parameter here carries
an explicit mode, so no declaration is uncontracted (section 3).

## 3. Calling contracted functions

A call to an `ffi`-declared function is checked against the declared signature:

* an argument for a `take` parameter moves the value into the call; later use
  of the binding is E10020;
* an argument for a `const` parameter is a read-only lend;
* an argument for a `borrow` parameter is a writable lend;
* a nullable result is narrowed before it is used, otherwise E10010.

An **uncontracted parameter** is an `ffi` parameter whose ownership mode is
omitted on a handle or aggregate type (section 2). An `ffi` declaration is an
**uncontracted extern** when at least one parameter is uncontracted. Calling
an uncontracted extern may appear only inside `unsafe` (E10100); its
uncontracted parameters accept only scalars, `raw`, `cstring`, views, and
`null` handles, so passing a live owned or borrowed value is E10101. Scalar-only
declarations (for example `IoErr`) own nothing and are contracted without modes.
A fully contracted declaration needs no `unsafe`. Application code never writes
an uncontracted declaration, because the library wraps the call (X6).

| Rule | Statement | Diagnostic |
|---|---|---|
| CN-9 | A contracted call is checked against the declared modes and types | - |
| CN-10 | Calling an uncontracted extern (a declaration with at least one uncontracted parameter) may appear only inside `unsafe` | E10100 |
| CN-11 | An uncontracted parameter accepts only scalars, `raw`, `cstring`, views, and `null` handles; passing a live owned or borrowed value is an error | E10101 |
| CN-12 | A nullable result is narrowed before use | E10010 |
| CN-13 | Application code does not write `ffi` blocks | E10103 |

## 4. Ownership in and out of a call

A returned handle belongs to the caller: whoever holds the handle releases it,
through its `freedBy` deleter, null-safely and exactly once (P5). A `take`
parameter moves ownership into the call, and the caller's binding is invalid
afterward. A lend (`const` or `borrow`) carries access only; the callee must not
keep the pointer or release the value.

| Rule | Statement | Diagnostic |
|---|---|---|
| CN-14 | A returned handle is owned by the caller and released by its `freedBy` deleter | - |
| CN-15 | A `take` argument transfers ownership; the caller's binding is invalid after the call | E10020 |
| CN-16 | A lend never transfers ownership; the callee must not store or release it, except the `borrow T?` slot's own incoming value, which the callee owns on entry and may release; a different lend's value or view is E10034 | E10034 |

### 4.1 Synchronous C hooks

A `borrow`-mode `func(...)` parameter on an `ffi` declaration is a synchronous
callback (L5). The C prototype is still a plain function pointer; the mode is
semantic. The callee may call the callback zero or more times during the call
and must not store it, return it, or call it after the call returns.

A capture-less lambda is passed as a plain function pointer. A capturing lambda
is accepted at this position: the compiler materializes its environment at the
callback's own argument position and passes a generated trampoline. The
environment is a caller-frame node keyed by the lambda site, never a heap
allocation. One wrapper per call site pushes the nodes, forwards the original
call, and pops them in reverse order; nested and reentrant hooks each find
their own node. An invocation after the hook returns faults E11215 when no
environment for its site is live.

The callback signature must be ABI-identical to the lifted lambda: scalar,
`bool`, enum, `raw`, `cstring`, handle, non-generic by-value struct, and
function-pointer **parameters**. A function-pointer **return** is E10210
because the `ffi` prototype cannot spell `ret (*)(...) name(...)`; views,
slices, fixed arrays, borrowed move-only structs, and fixed-array returns are
also E10210. A signature mismatch is E10210; a non-callable argument is E10200.
A capturing hook inside a generic function body is E10400. A capturing hook
that borrows a binding the same call moves is E10030. A capturing callback's
invocation after the hook returns faults E11215 when no environment for its
site is live; a capture-less callback stays a plain pointer and has no runtime
check.

| Rule | Statement | Diagnostic |
|---|---|---|
| CN-17 | A `borrow`-mode `func(...)` `ffi` parameter is a synchronous callback; the C prototype is unchanged | - |
| CN-18 | A capturing callback crosses through a generated trampoline and a caller-stack environment; no heap environment | - |
| CN-19 | The callback signature must be ABI-identical to the lifted lambda | E10210 |
| CN-20 | A capturing hook inside a generic body needs a per-instance trampoline, which v1.1 does not add | E10400 |
| CN-21 | A capturing callback invoked after its C hook returned faults when its environment node is not live | E11215 |

## 5. Nullable acquisition and out-parameters

An acquisition returns a nullable handle. The caller has two idioms:

* return the nullable handle and narrow it at the call site; or
* write it into a `borrow T?` out-parameter. The caller initializes the slot
  (`let f: File? = null;`); passing an uninitialized slot is E10021. The callee
  owns the incoming value on entry, may release it, is not required to write on
  every path, and the slot owns the result afterwards; the write drops the
  callee-owned previous value (the O3 carve-out in [Ch 03](03-ownership.md)).
  This lets the wrapper report failure with detail in a status struct.

```clorraine
func openInto(path: cstring, mode: i32, borrow out: File?): bool {
    let f = Open(path, mode);
    if (f == null) {
        return false;       // out is left null
    }
    out = f;                // File widens into File?
    return true;
}
```

Application code then uses the ordinary nullable rules from
[Ch 03](03-ownership.md):

```clorraine
let f: File? = null;
if (openInto("RAM:data", MODE_OLDFILE, borrow f)) {
    if (f != null) {
        // f is narrowed to File here
    }
}
```

## 6. Validated subset (Phase 0)

The Phase 0 skeleton exercised this boundary on `vamos`, the FS-UAE directory
drive, and a bootable ADF, with GCC and vbcc. The table restates that subset in
the current contract vocabulary; the handle and deleter declarations appear
once, in the library.

| Function | Clorraine declaration | Nullable | Validated |
|---|---|---|---|
| `OpenLibrary` | `func OpenLibrary(name: cstring, version: u32): Library?;` | yes | yes |
| `CloseLibrary` | `func CloseLibrary(take base: Library): void;` | - | yes |
| `AllocVec` | `func AllocVec(size: u32, flags: u32): Memory?;` | yes | yes |
| `FreeVec` | `func FreeVec(take mem: Memory): void;` | - | yes |
| `Open` | `func Open(name: cstring, mode: i32): File?;` | yes | yes |
| `Close` | `func Close(take file: File): void;` | - | yes |
| `Write` | `func Write(borrow file: File, const buf: u8[], len: i32): i32;` | - | yes |
| `Read` | `func Read(borrow file: File, borrow buf: u8[], len: i32): i32;` | - | yes |

```clorraine
export handle Library freedBy CloseLibrary;
export handle Memory  freedBy FreeVec;
export handle File    freedBy Close;
```

Phase 0 released every acquisition on every exit path and confirmed the
null-safe drops: a failed `Open` still released the owned library, and a failed
`AllocVec` still released the file and the library. Next to add, in order of
expected use: `CreateMsgPort`/`DeleteMsgPort`, `CreateIORequest`/
`DeleteIORequest`, `CreatePool`/`DeletePool`/`AllocPooled`, `Seek`, and
`Lock`/`Unlock`.

## 7. Boundary rules

| Situation | Rule | Diagnostics |
|---|---|---|
| Passing a value to legacy C | use a contracted `ffi` declaration; modes are checked. An uncontracted declaration may be called only inside `unsafe`, and its uncontracted parameters never receive an owned or borrowed value | E10100, E10101 |
| Receiving a handle | acquisition returns `T` or `T?`; the caller owns it; `null` is the failure case | - |
| Pointer to a C-owned object | model it as `raw T` (`raw T?` when it can be empty); access requires `unsafe`; never silently convert it into an owned handle | E10201, E10600-E10604 |
| `BPTR` file handles | declared as `handle File freedBy Close;`; the zero `BPTR` is `null` | - |
| `void *` parameters | use `raw u8` (or a typed `raw T`); converting to or from a typed pointer needs an explicit cast inside `unsafe` | E10201, E10603 |
| Callbacks | deferred: function values exist, but `stores` contracts and their lifetime guarantees are v2 | E90003 |
| Ownership across tasks | deferred: message passing crosses tasks; the transfer model is v2, and shared state stays `raw`/`unsafe` | E90003 |
| Caching a handle in a global | a handle is not duplicable, so a global may not own it | E11106 |
| Storing a C pointer past the call | deferred; `stores` is v2 and lives in audited `raw` wrappers | E90003 |

## 8. Deferred to v2

| Item | Why deferred | Diagnostic |
|---|---|---|
| `stores` (the callee keeps a pointer past the call: callbacks, interrupt servers, hooks) | needs a matching remove call to bound the lifetime | E90003 |
| `transfers` (ownership crosses a task or message boundary) | needs an explicit trusted-transfer model (C2) | E90003 |
| Varargs functions such as `PrintF` and `DoMethod` | need a typed wrapper layer before a contract can describe them | - |
| Pools and arenas | bulk release invalidates many handles; needs a region model | - |

## 9. Trust and review policy

* Contract tables are reviewed like code. Each entry cites the NDK prototype it
  describes.
* The SDK ships the reviewed tables. A user-provided table warns as W10005 so
  audits can find it.
* A later tool may diff signatures against `.fd`/`.sfd`/NDK headers for drift.
  Signature drift is mechanical; the ownership columns stay human-reviewed,
  because no header states them.

A contract declaration uses the NDK prototype's result type where the result is
used. Where a deleter is declared `void` although the NDK returns `BOOL` or
`LONG` (for example `Close`), the compiler emits the call and discards the
value. Discarding it is ABI-safe on m68k and avoids an unused-result diagnostic.

## 10. Emitted ABI and runtime shim

An `ffi` declaration is **ABI-typed**: its C prototype comes from the declared
types through the FFI ABI mapping in [Ch 17](17-codegen.md) section 3, never
from the parameter modes. The modes (`take`, `borrow`, `const`, unmarked) are
semantic only; they record ownership and do not change the C parameter type. A
view such as `u8[]` in an `ffi` signature is therefore the ABI's pointer type
(`APTR`) with the length passed as a separate parameter, not a view value
struct; the ergonomic view interface that bundles the length lives in the
Clorraine wrapper. The Phase 0 declarations keep their meaning under this
mapping: a handle lowers to the handle's C typedef (for example `File` to
`BPTR`) and a scalar to its fixed-width C type.

The drop of a handle emits its `freedBy` deleter, null-safe, as one entry in the
cleanup ladder (P5). The runtime shim names the acquire and drop helpers per
handle type. The emitted C ABI, the symbol naming, the ABI table, and the shim
are specified in [Ch 17](17-codegen.md).

## 11. Diagnostics

| Code | Trigger |
|---|---|
| E10100 | Call to an uncontracted extern outside `unsafe` |
| E10101 | Owned or borrowed value passed to an uncontracted parameter |
| E10102 | Invalid `freedBy` deleter |
| E10103 | Exported `ffi` block or FFI leaking into application code |
| E10104 | Native struct declared with `freedBy` |
| E10201 | Cast not allowed at the boundary |
| E10600-E10604 | Raw pointer operations outside `unsafe` |
| E11106 | A handle cached in a global (non-duplicable global) |
| E90003 | `stores` and `transfers` contracts (deferred) |
| W10005 | User-provided contract table in use |

All message text lives in [Ch 14](14-diagnostics.md).
