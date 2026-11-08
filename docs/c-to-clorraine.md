# From C to Clorraine: A Rosetta Guide

This guide translates the C patterns an Amiga programmer already knows into
Clorraine. It is a set of excerpts, not a second specification. Each section
links the chapter that owns the rule.

Clorraine keeps C's machine model: fixed-width integers, explicit layout,
pointers at the hardware boundary, and one C translation unit. What changes is
who owns a value and when it is released.

## 1. Mental model

| C habit | Clorraine rule |
| ------- | -------------- |
| A pointer may be null, borrowed, or owned; the type does not say | `T`, `T?`, `borrow T`, `take T`, and views make the mode explicit |
| `free` is called by convention | The owner releases a value once at scope end; `take` transfers the duty |
| A struct copy is a byte copy | A value copies only when its type is duplicable; owning types move |
| A cast reinterprets | `raw T` is explicit and `unsafe` at the hardware boundary |
| An error is a sentinel return | A nullable return or a status struct says what failed |

The ownership chapter is [Chapter 03](../spec/03-ownership.md); the memory rules
are [Chapter 11](../spec/11-memory.md).

## 2. Allocation

C:

```c
struct Buffer *b = AllocMem(sizeof *b, MEMF_CLEAR);
if (b == NULL) {
    return ENOMEM;
}
b->len = 0;
FreeMem(b, sizeof *b);
```

Clorraine, with the safe owner:

```clorraine
import { bufferNew, bufferLen } from std.mem;

let maybe: Buffer? = bufferNew(64);
if (maybe == null) {
    return;
}
let b: Buffer = maybe;
let n: usize = bufferLen(b);
// `b` is released at the end of the scope
```

`Buffer` is an opaque handle with a `freedBy` deleter. `bufferNew` returns
`Buffer?`, so the check is explicit and the narrowed value moves into the
non-null binding.

A recursive owner uses `Box<T>`. Given
`struct Node { next: Box<Node>?; value: i32; }`:

```clorraine
let node: Box<Node> = boxNew(Node { next: null, value: 1 });
```

`Box<T>` is move-only and non-nullable; `Box<T>?` is optional. A box releases
the pointee recursively and then frees the block. See
[Chapter 11](../spec/11-memory.md) and the `std.mem` appendix entries.

For a caller-owned resource, declare a handle and a deleter:

```clorraine
export handle Token freedBy releaseToken;

export func releaseToken(take token: Token): void {
    // release the resource exactly once
}
```

## 3. Files

C:

```c
BPTR f = Open(path, MODE_OLDFILE);
if (f == 0) {
    return ERROR;
}
LONG n = Read(f, buffer, size);
Close(f);
```

Clorraine uses the `std.io` wrappers, which keep the `ffi` and `unsafe` work
inside the module:

```clorraine
import { File, MODE_OLDFILE, openFile, readFile } from std.io;

let maybe: File? = openFile("data.bin", MODE_OLDFILE);
if (maybe == null) {
    return;
}
let f: File = maybe;
let n: i32 = readFile(f, buffer);
// `f` closes at the end of the scope
```

A short count is not an error. Loop when the whole request matters, and keep
the count and the error as separate values. The DOS surface also provides
`std.amiga.dos.lockPath`, `pathInfo`, `volumeInfo`, and the delay helpers.

## 4. Strings

C has `char *` and `strlen`. Clorraine separates three things:

| Spelling | Meaning |
| -------- | ------- |
| `str` | A borrowed byte view, not NUL-terminated |
| `String` | An owned byte string |
| `cstring` | A borrowed NUL-terminated view for an `ffi` call |

```clorraine
func nameLength(borrow name: str): usize {
    return len(name);
}

import { stringOf, toCString } from std.text;

let owned: String = stringOf("hello");
let view: str = owned;
let c: cstring = toCString(view);
```

A `String` does not implicitly become a `cstring`; the NUL-terminated form is
a distinct promise to the C side. Use the standard-library conversion when the
boundary needs it. See [Chapter 08](../spec/08-arrays-strings.md).

## 5. MMIO

C:

```c
volatile UWORD *color = (UWORD *)0xDFF180;
*color = 0x0F00;
```

Clorraine keeps the same address but makes the access auditable:

```clorraine
unsafe {
    writeVolatile16(raw u16(0xDFF180), 0x0F00);
}
```

The per-chip wrappers in `std.amiga.video` and the other `std.amiga.*` modules
are preferred: they name the register, keep the `unsafe` block small, and hide
the address arithmetic. A `raw` base may be `const`; the pointed-to value is
still volatile where the wrapper says so. See
[Chapter 15](../spec/15-profiles.md).

## 6. Interrupts

C installs a server and a client on an interrupt chain. Clorraine exposes the
same model through `std.amiga.interrupt`:

```clorraine
import { HandlerToken, addHandler, removeHandler } from std.amiga.interrupt;

func onTick(): void {
    // allocation-free and drop-free
}

let token: HandlerToken = addHandler(onTick);
removeHandler(token);
```

The handler is a plain function, not a closure: a storing registration API
accepts only a capture-less callback. The registered argument is owned by the
chain until `removeHandler`. See the `std.amiga.interrupt` module.

## 7. Hooks

C:

```c
struct Hook hook = { 0, 0, entry, data };
CallHookA(&hook, obj, message);
```

Clorraine passes a synchronous C hook through an `ffi` declaration. A
capturing lambda can cross that call because the compiler emits a trampoline
and a caller-stack environment:

```clorraine
ffi utility {
    func CallHookA(hook: borrow func(): void, obj: raw Object, msg: raw Message): u32;
}

let base: i32 = 10;
let result: u32 = CallHookA(() => {
    // `base` is captured in a caller-stack environment
}, obj, msg);
```

A callback stored by the C side (a `newHook` registration, a task entry, or an
interrupt handler) must be capture-less. A capturing lambda there is
diagnosed; a stored callback invoked after the synchronous hook returns faults
`E11215`. See [Chapter 12](../spec/12-ffi-contracts.md) and the L5 rule.

## 8. Ownership modes

| Mode | Use when | Caller after the call |
| ---- | -------- | --------------------- |
| `T` (by value) | The callee needs its own copy; `T` is duplicable | Still owns its copy |
| `borrow T` | Read-only use for the call | Still owns |
| `take T` | The callee becomes the owner | Must not use the value |
| `borrow T` on a view | The callee may write through a view | Still owns the storage |
| `handle` | An opaque resource with a `freedBy` deleter | Moves when passed by value |
| `raw T` | Hardware or an uncontracted C boundary | Unchanged; `unsafe` |

The compiler rejects a use after a move and a release of a borrowed parameter.
Those two diagnostics catch most C habits:

```clorraine
func consume(take s: String): void {
}

func demo(): void {
    let s: String = stringOf("x");
    consume(s);
    consume(s);   // E10020: s was moved
}
```

## 9. Error handling

C returns `LONG` sentinels, sets `errno`, or writes through an out-parameter.
Clorraine has no exceptions and no hidden error global. Pick the shape that
matches the operation:

| Shape | Example |
| ----- | ------- |
| Nullable return | `openFile` returns `File?`; narrow with `if (f != null)` |
| Status struct | A `bool`/enum code plus the payload |
| Count plus status | `readFile` returns the bytes moved; a short count is expected |
| Trap | `panic("invariant")` for a broken program, not for user input |

```clorraine
let maybe: File? = openFile(path, MODE_OLDFILE);
if (maybe != null) {
    let f: File = maybe;
    let n: i32 = readFile(f, buffer);
}
```

A nullable value is narrowed only by a check in the same scope. A non-null
type cannot be initialized from `null`; the diagnostic is `E10012`.

## 10. Diagnostics a C programmer will meet

| Code | C habit that triggers it | Fix |
| ---- | ------------------------ | --- |
| `E10010` | Using a nullable pointer without a check | Narrow with `if (p != null)` |
| `E10012` | Initializing a non-null type from `null` | Use `T?` or fix the initializer |
| `E10020` | Copying a non-duplicable value | Move it or pass `borrow` |
| `E10026` | Moving out of a borrowed view | Borrow, copy the element, or take ownership |
| `E10034` | Releasing or storing a borrowed parameter | Declare `take` or by value |
| `E10105` | Passing a generic or `Box` value across FFI | Use the box/handle API at the boundary |
| `E10201` | A C-style cast between unrelated types | Use the declared conversion or `raw` |
| `E10207` | Storing an overlay outside its scope | Keep the overlay local or copy |
| `E10306` | Storing an ephemeral closure | Use a plain function or a synchronous hook |
| `E10400` | A capturing lambda at a storing C API | Make it capture-less or use a synchronous hook |
| `E11200` | An Amiga module on the wrong profile | Select `hosted-amiga` |
| `E11215` | A stored hook callback invoked after return | Keep the call synchronous; do not store it |

The live registry and the fix hint for every code are in
[Chapter 14](../spec/14-diagnostics.md). `clorraine --explain E10020` prints
the same entry from the binary.

## Where next

- [The language specification](../spec/README.md)
- [Known limitations](known-limitations.md)
- [The standard library appendix](../spec/appendix-stdlib.md)
