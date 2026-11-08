# Chapter 11 - Memory and Resources

> **Status:** stable (v1.0)
> **Depends on:** Ch 00 (axioms), Ch 03 (ownership), Ch 07 (structs), Ch 08 (arrays and strings), Ch 12 (FFI and contracts), Ch 14 (diagnostics), Ch 15 (profiles), Ch 17 (codegen) *(informational)*
> **Grammar:** [grammar.ebnf.md](grammar.ebnf.md) sections 3.4 (handles), 3.5 (module-level values), 4 (types), 7 (intrinsics)
> **Ledger:** [decisions.md](decisions.md) section 18 (standard-library scope); Ch 00 axioms A2 and A7; X1, T2, T8, F7, F9; P2-P6, P14

## 1. Memory classes

A value lives in one of four places. The first three are part of v1; the
fourth waits on the profile work ([appendix-amiga.md](appendix-amiga.md)).

| Class | Holds | Lifetime | Release |
|---|---|---|---|
| Stack | locals, parameters, temporaries | the scope or statement that created it | automatic at scope exit (O2, D5) |
| Heap | memory behind a handle | until the owner drops | the handle's `freedBy` deleter (X1, P5) |
| Static | module-level values | the whole program | none; a global may not own (O10) |
| Profile region | memory with a placement cost | profile-defined | a profile allocator (later) |

The **stack** is the default location for locals and temporaries. A local
aggregate is stack storage unless an allocator puts it behind a handle. Drop
order is fixed: locals in reverse declaration order, parameters after body
locals, and temporaries at the end of the statement that created them (O2, D5).
Every exit joins the single-exit cleanup ladder, in reverse registration order
(P2; [Ch 17](17-codegen.md) section 4). Definite assignment applies (F9): a
local read before it is assigned on every path is E10021.

The **heap** holds anything an allocator returns. Reaching it always goes
through an owned handle, so the release function is part of the type.

The **static** class holds module-level values. A module-level `let` must be
duplicable or `const`, and a module-level `const` must be a compile-time value
(V2, O10). Because there are no exit drops, a global can never own a resource
([Ch 03](03-ownership.md) section 5); a non-duplicable module-level `let` is
E11106. Runtime-initialized
globals become generated `cl_init_<module>` functions run in dependency order
before `main` (M5, P9); a cycle that involves them is E11100.

**Profile regions** are memory whose placement matters, such as Amiga chip and
fast memory. They are a later profile feature; the appendix records the plan.

## 2. Explicit allocation

Allocation is explicit. A resource is named by a `handle` declaration, and the
handle carries its own deleter (X1):

```clorraine
// Declared once, in the standard library that owns the resource (X1).
export handle Buffer  freedBy FreeBuffer;
export handle String  freedBy FreeString;
export handle File    freedBy Close;
export handle MsgPort freedBy DeleteMsgPort;

// Core handles get their own FFI deleters; C-facing types keep the C
// symbols `Memory`/`FreeVec` (T2a).
ffi stdmem {
    func FreeString(take s: String): void;
    func FreeBuffer(take b: Buffer): void;
}
```

`freedBy F` names a function that takes `take T` and returns `void`; it may be
an ordinary Clorraine function (resolved module-scoped, X1) or an `ffi`
declaration. There is exactly one deleter per type (X1). The standard library
wraps the C functions; application code never writes `ffi` blocks (X6).

| Resource | C allocator | C deleter | `freedBy` |
|---|---|---|---|
| Heap bytes (`Buffer`) | `AllocVec` | `FreeVec` | `FreeBuffer` |
| File | `Open` | `Close` | `Close` |
| Message port | `CreateMsgPort` | `DeleteMsgPort` | `DeleteMsgPort` |

| Rule | Statement | Diagnostic |
|---|---|---|
| MR-1 | There is no language-level `new`, no garbage collection, and no reference counting (A2); the built-in `Box<T>` constructor is the one compiler-owned allocation, and it traps `E1` on failure (section 2.1) | - |
| MR-2 | An allocator returns a nullable owned value, `T?` (T8), except the `Box<T>` constructor, which is non-nullable and traps | - |
| MR-3 | The caller checks `== null` / `!= null` before use | E10010 |

Ownership lowers to plain C: a move copies the value and zeroes the source
(P3), the all-zero bit pattern is the empty state (P4), and the drop of a
handle is a null-safe call to its deleter (P5). An allocation reports failure
as `null`:

```clorraine
import { Buffer, bufferNew } from std.mem;

func makeBuffer(size: usize): Buffer? {
    let buf: Buffer? = bufferNew(size);
    if (buf == null) {
        return null;              // failure: the caller sees a plain null
    }
    return buf;                   // the owned handle moves to the caller
}
```

`Buffer` is standard-library owned bytes; the library reference fixes the exact
API names. `String` is built in (T2) and follows the same rule: one allocation
holds `[len][cap][bytes][NUL]`, and `FreeString` releases it (P6, T2a). The string
types and their conversions are in [Ch 08](08-arrays-strings.md).

### 2.1 Box

`Box<T>` is the built-in owning pointer (P7a). It is one `void *` in C, move-only,
and released by generated recursive drop glue. `boxAlloc<T>()` is the reserved
compiler intrinsic; `std.mem` exposes the safe surface:

| API | Meaning |
|---|---|
| `boxNew<T>(value: take T): Box<T>` | allocate and move `value` into a new box |
| `boxSet<T>(box: borrow Box<T>, value: take T): void` | replace and drop the old pointee |
| `boxWith<T, F: func(T): void>(box: borrow Box<T>, f: F): void` | lend the pointee to a callable-bound callback for the duration of the call |
| `boxPtr<T>(box: borrow Box<T>): raw T` | internal helper: the pointee address for library code; not exported |

The allocation uses `cl_box_alloc` (which calls `cl_alloc`) and frees through
`cl_box_free` (`cl_free`). The block is zeroed. An allocation failure uses the
existing `E1` trap; `Box<T>` itself is non-nullable, while `Box<T>?` is an
optional slot. A boxed value cannot be moved out. A non-static view-typed
element is E10227 (`static str` is accepted), an overlay element is E10207, a
non-view element must be storable, and a direct Box FFI crossing is E10105. The
freestanding profile has no allocator and rejects Box with E11200. A user type
declaration named `Box` shadows the built-in; inside the embedded `std.mem` the
built-in always wins.

## 3. Stack discipline

Clorraine uses ordinary C frames. Static Frame Allocation, the Blend65 model
that placed every function's locals in fixed memory, is gone (F7, P1).

| Rule | Statement | Diagnostic |
|---|---|---|
| MR-4 | Recursion is unrestricted: direct, indirect, mutual, and generic (F7) | - |
| MR-5 | Stack depth is a runtime concern with documented guidance | - |
| MR-6 | A local frame that exceeds the profile threshold warns | W10004 |
| MR-7 | There are no VLAs and no dynamic-size stack arrays in emitted C | - |

```clorraine
func depth(n: i32): i32 {
    if (n <= 0) { return 0; }
    return 1 + depth(n - 1);      // an ordinary C frame, no static frame (F7)
}

func scratch(): void {
    let page: u8[4096] = [; 0];
    page[0] = 1;                  // W10004 above the profile threshold
}
```

The threshold is a profile fact with a v1 default (W10004a), verified across
the build matrix (P14; [Ch 17](17-codegen.md) section 13).

### Stack-depth guidance

Recursion depth is the program's responsibility. Keep recursion bounded, and
never recurse deeply in interrupt context, where the stack is small and shared
with the interrupted code. The hosted profiles publish concrete stack sizes in
the SDK documentation; a freestanding build owns its stack policy. A compiler
stack-usage report is a follow-up.

## 4. Failure handling

Recoverable allocation failure is reported as a nullable result. An allocation
returns `T?`, and the caller must narrow it with an `== null` or `!= null`
check before use; use before the check is E10010 (T8).

Invariant failure is different. `panic(msg)` and `assert(cond, msg)` abort
without running the cleanup ladder (E4, [Ch 03](03-ownership.md) section 5).
The standard library uses `panic` for out-of-memory conditions that its own
contract cannot report (A7). A runtime fault aborts the same way: an
out-of-range index is E11211, and integer division by zero is E11210
([Ch 14](14-diagnostics.md) section 11).

## 5. Pools and arenas

A pool or arena releases many allocations at once. That is useful, but a bulk
release invalidates every outstanding view into the pool, and v1 has no region
model to check those uses. Pools and arenas are therefore deferred to v2
(E90004), and a later region model must reject a use that outlives its pool.
[Ch 12](12-ffi-contracts.md) section 8 records the deferral.

## 6. Standard-library scope

The library is where memory ergonomics live. In priority order, v1 ships:

| Item | Purpose |
|---|---|
| `Buffer` (owned bytes) | Bulk IO, bitmaps, samples; safe slices into owned storage |
| `Array<T>` (growable) | Default dynamic container; most workloads need no other structure |
| One error/status carrier | Detail-carrying failures; `T?` stays detail-free; partial-IO conventions |
| Format helpers | `toHex` / `toDecimal`, padding, and float formatting used by templates |
| Per-chip MMIO wrappers | Safe accessors over the volatile intrinsics; `const raw` register bases |
| Interrupt/hook dispatcher | C-side qualified handler plus token add/remove; handlers are allocation-free, drop-free, panic-free |
| `Span` | Offset-based parsing helper (the view-field ban still applies) |

The MMIO wrappers cover the volatile intrinsics (T7), and register bases are
`const raw` values ([grammar.ebnf.md](grammar.ebnf.md) section 6.8). Library
APIs are written dot-call style (F8): `x.f(y)` resolves to the free function
`f(x, y)`. The v1 signatures for these library items are in
[appendix-stdlib.md](appendix-stdlib.md).

```clorraine
import { Array, arrayNew } from std.mem;   // names illustrative

func demo(): usize {
    let xs: Array<i32> = arrayNew<i32>();   // constructors need explicit arguments (G2)
    return xs.len();                        // resolves to len(xs)
}
```

`Array<T>` usage is in [Ch 08](08-arrays-strings.md) section 9. Diagnostics for
the dispatcher and the status carrier are registered in
[Ch 14](14-diagnostics.md). A `Span` never becomes a `str` field, because
view-typed fields are banned (T4, E10205).

## 7. Alignment and layout

Alignment and layout come from [Ch 07](07-structs.md) and the profile appendix.
Struct fields keep declaration order (P1). There are no packing attributes and
no packed-struct tricks: an overlay describes the layout it must match, and the
target ABI supplies the rest. `sizeof` and `offsetof` return `usize`
([grammar.ebnf.md](grammar.ebnf.md) section 7).

## 8. The runtime shim

The runtime is deliberately small and profile-specific: acquire and drop
helpers, allocator hooks, and `panic`. It contains no ownership logic; it only
names the release function and the failure sentinel for each handle type. The
shim is described in [Ch 17](17-codegen.md) section 12.

## 9. Diagnostics

| Code | Trigger |
|---|---|
| E10010 | a nullable allocation result is used before a null check |
| E10021 | a local is read before it is assigned on every path |
| E11100 | an initializer dependency cycle (between modules or same-module globals) |
| E11106 | a module-level `let` uses a non-duplicable initializer |
| E11210 | runtime fault: integer division by zero (shared abort path) |
| E11211 | runtime fault: index out of range |
| E90004 | pools and arenas are deferred to v2 |
| W10004 | stack frame exceeds the profile threshold |

All message text lives in [Ch 14](14-diagnostics.md).
