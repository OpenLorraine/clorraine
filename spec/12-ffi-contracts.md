# Chapter 12 - FFI and Ownership Contracts

> **Status:** draft (Phase C - rewritten against the ledger and the grammar)
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
| `x: T` or `const x: T` | read-only lend for the duration of the call |
| `borrow x: T` | writable lend for the duration of the call |
| `T?` | nullability; `null` is the failure sentinel |

The draft's clause keywords are retired. Nullability lives in the type
(`File?`), never in a clause (X3).

| Rule | Statement | Diagnostic |
|---|---|---|
| CN-4 | FFI declarations use ordinary signatures and modes; nullability lives in the type | - |
| CN-5 | `handle T freedBy F;` declares an opaque resource once, in the library that owns it | - |
| CN-6 | `F` must be an `ffi`-declared function taking `take T` and returning `void` | E10102 |
| CN-7 | Exactly one deleter per handle type; a second `handle` declaration for the same type is a duplicate declaration | E11101 |
| CN-8 | A native struct never declares `freedBy`; it drops structurally | E10104 |

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
    func Write(borrow file: File, buf: u8[], len: i32): i32;
    func Read(borrow file: File, borrow buf: u8[], len: i32): i32;
}
```

`OpenLibrary`, `AllocVec`, and `Open` return `T?`, so a failure is an empty
value. `CloseLibrary`, `FreeVec`, and `Close` take ownership with `take`.
`Write` reads the buffer; `Read` writes to it. The file handle is lent
mutably, because both calls advance the position stored in the handle.

## 3. Calling contracted functions

A call to an `ffi`-declared function is checked against the declared signature:

* an argument for a `take` parameter moves the value into the call; later use
  of the binding is E10020;
* an argument for an unmarked or `const` parameter is a read-only lend;
* an argument for a `borrow` parameter is a writable lend;
* a nullable result is narrowed before it is used, otherwise E10010.

An **uncontracted extern call** is a call to something C provides that no `ffi`
declaration describes. It may appear only inside `unsafe` (E10100), and it may
not receive an owned or borrowed value (E10101). Application code never needs
this form, because the library wraps the call (X6).

| Rule | Statement | Diagnostic |
|---|---|---|
| CN-9 | A contracted call is checked against the declared modes and types | - |
| CN-10 | An uncontracted extern call may appear only inside `unsafe` | E10100 |
| CN-11 | An uncontracted extern call may not receive an owned or borrowed value | E10101 |
| CN-12 | A nullable result is narrowed before use | E10010 |
| CN-13 | Application code does not write `ffi` blocks | E10103 |

## 4. Ownership in and out of a call

A returned handle belongs to the caller: whoever holds the handle releases it,
through its `freedBy` deleter, null-safely and exactly once (P5). A `take`
parameter moves ownership into the call, and the caller's binding is invalid
afterward. A lend (unmarked, `const`, or `borrow`) carries access only; the
callee must not keep the pointer or release the value.

| Rule | Statement | Diagnostic |
|---|---|---|
| CN-14 | A returned handle is owned by the caller and released by its `freedBy` deleter | - |
| CN-15 | A `take` argument transfers ownership; the caller's binding is invalid after the call | E10020 |
| CN-16 | A lend never transfers ownership; the callee must not store or release it | E10034 |

## 5. Nullable acquisition and out-parameters

An acquisition returns a nullable handle. The caller has two idioms:

* return the nullable handle and narrow it at the call site; or
* write it into a `borrow T?` out-parameter, which is exempt from definite
  assignment (F9) and lets the wrapper report failure with detail in a status
  struct.

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
| `Write` | `func Write(borrow file: File, buf: u8[], len: i32): i32;` | - | yes |
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
| Passing a value to legacy C | use a contracted `ffi` declaration; modes are checked. Without a contract, only inside `unsafe`, and never an owned or borrowed value | E10100, E10101 |
| Receiving a handle | acquisition returns `T` or `T?`; the caller owns it; `null` is the failure case | - |
| Pointer to a C-owned object | model it as `raw T` (`raw T?` when it can be empty); access requires `unsafe`; never silently convert it into an owned handle | E10201, E10600-E10604 |
| `BPTR` file handles | declared as `handle File freedBy Close;`; the zero `BPTR` is `null` | - |
| `void *` parameters | use `raw u8` (or a typed `raw T`); converting to or from a typed pointer needs an explicit cast inside `unsafe` | E10201, E10603 |
| Callbacks | deferred: function values exist, but `stores` contracts and their lifetime guarantees are v2 | E90003 |
| Ownership across tasks | deferred: message passing crosses tasks; the transfer model is v2, and shared state stays `raw`/`unsafe` | E90003 |
| Caching a handle in a global | a handle is not duplicable, so a global may not own it | E10037 |
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

## 10. Emitted ABI and runtime shim

An `ffi` declaration lowers to a C function prototype with the same signature.
The drop of a handle emits its `freedBy` deleter, null-safe, as one entry in the
cleanup ladder (P5). The runtime shim names the acquire and drop helpers per
handle type. The emitted C ABI, the symbol naming, and the shim are specified
in [Ch 17](17-codegen.md).

## 11. Diagnostics

| Code | Trigger |
|---|---|
| E10037 | A handle cached in a global (non-duplicable global) |
| E10100 | Uncontracted extern call outside `unsafe` |
| E10101 | Owned or borrowed value passed to an uncontracted extern |
| E10102 | Invalid `freedBy` deleter |
| E10103 | Exported `ffi` block or FFI leaking into application code |
| E10104 | Native struct declared with `freedBy` |
| E10201 | Cast not allowed at the boundary |
| E10600-E10604 | Raw pointer operations outside `unsafe` |
| E90003 | `stores` and `transfers` contracts (deferred) |
| W10005 | User-provided contract table in use |

All message text lives in [Ch 14](14-diagnostics.md).
