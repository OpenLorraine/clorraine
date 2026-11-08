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

A parameter with no mode or with `const` is a read-only lend. `borrow` is the
writable lend, and `take` transfers ownership. The ownership chapter is
[Chapter 03](../spec/03-ownership.md); the memory rules are
[Chapter 11](../spec/11-memory.md).

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
import { Buffer, bufferLen, bufferNew } from std.mem;

func useBuffer(): void {
    let maybe: Buffer? = bufferNew(64);
    if (maybe == null) {
        return;
    }
    let b: Buffer = maybe;
    let n: usize = bufferLen(b);
}
```

`Buffer` is an opaque handle with a `freedBy` deleter. `bufferNew` returns
`Buffer?`, so the check is explicit and the narrowed value moves into the
non-null binding. The buffer is released at the end of the scope.

A recursive owner uses `Box<T>`. Given
`struct Node { next: Box<Node>?; value: i32; }`:

```clorraine
import { boxNew } from std.mem;

func makeNode(): Box<Node> {
    return boxNew(Node { next: null, value: 1 });
}
```

`Box<T>` is move-only and non-nullable; `Box<T>?` is optional. A box releases
the pointee recursively and then frees the block. See
[Chapter 11](../spec/11-memory.md) and the `std.mem` appendix entries.

For a caller-owned resource, declare a handle and a deleter that releases it
through an `ffi` `take` call:

```clorraine
export handle Token freedBy releaseToken;

ffi tokens {
    func FreeToken(take token: Token): void;
}

export func releaseToken(take token: Token): void {
    FreeToken(token);
}
```

The deleter is a library-module pattern: its body must release the `take`
parameter with a top-level `ffi` call.

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
import { Buffer, bufferBytes, bufferNew } from std.mem;

func readFirst(path: cstring): i32 {
    let file: File? = openFile(path, MODE_OLDFILE);
    if (file == null) {
        return -1;
    }
    let storage: Buffer? = bufferNew(4096);
    if (storage == null) {
        return -1;
    }
    let buffer: Buffer = storage;
    let bytes: u8[] = bufferBytes(buffer);
    return readFile(file, bytes);
}
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
import { stringOf, toCString } from std.text;

func nameLength(name: str): usize {
    return len(name);
}

func conversions(): void {
    let owned: String = stringOf("hello");
    let view: str = owned;
    let c: cstring = toCString(view);
}
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

Clorraine keeps the same address but makes the access auditable. The per-chip
wrapper names the register and keeps the address arithmetic in one place:

```clorraine
import { setColor } from std.amiga.custom;

func setBackground(): void {
    setColor(0, 0x0F00);
}
```

`setColor` writes the volatile register; the `unsafe` and raw address stay
inside the standard library. The overlay and volatile rules are in
[Chapter 07](../spec/07-structs.md) section 7; profile availability is in
[Chapter 15](../spec/15-profiles.md).

## 6. Interrupts

C installs a server and a client on an interrupt chain. Clorraine exposes the
same model through `std.amiga.interrupt`:

```clorraine
import { HandlerToken, addHandler, removeHandler } from std.amiga.interrupt;

func onTick(): void {
    // allocation-free and drop-free
}

func install(): void {
    let token: HandlerToken = addHandler(onTick);
    removeHandler(token);
}
```

The handler is a plain function, not a closure: a storing registration API
accepts only a capture-less callback. The registered argument is owned by the
chain until `removeHandler`. See the `std.amiga.interrupt` module.

## 7. Hooks

C builds a `struct Hook` and passes its address to `CallHookA`. The standard
library owns that layout; an application uses the `Hook` surface:

```clorraine
import { Hook, callHook, newHook } from std.amiga.utility;

func onHook(): void {
}

func useHook(): i32 {
    let maybe: Hook? = newHook(onHook, 0);
    if (maybe == null) {
        return 0;
    }
    let hook: Hook = maybe;
    return callHook(hook);
}
```

A synchronous callback at a plain C function-pointer parameter is declared
with a `borrow` mode on the parameter name:

```clorraine
ffi dispatch {
    func ForEach(item: raw u8, borrow visit: func(raw u8): void): u32;
}

func dispatchOne(item: raw u8): u32 {
    let base: i32 = 10;
    return ForEach(item, [base] (value: raw u8) => {
        let n: i32 = base;
    });
}
```

The callback can capture because the compiler emits a trampoline and a
caller-stack environment. A callback stored by the C side (a registration, a
task entry, or an interrupt handler) must be capture-less. A capturing
callback stored despite the contract and invoked after the synchronous call
returns faults `E11215`; a capture-less callback has no runtime guard. See
[Chapter 12](../spec/12-ffi-contracts.md) and the L5 rule.

## 8. Ownership modes

| Mode | Use when | Caller after the call |
| ---- | -------- | --------------------- |
| `T` or `const T` parameter | Read-only lend for the call | Still owns; a duplicable value may also be copied by value elsewhere |
| `borrow T` | Writable lend for the call | Still owns |
| `take T` | The callee becomes the owner | Must not use the value |
| `T` by value in a local or return | Copy when duplicable, move when owning | The source is invalid after a move |
| `handle` | An opaque resource with a `freedBy` deleter | Lends without a mode; moves only through `take` |
| `raw T` | Hardware or an uncontracted C boundary | Unchanged; `unsafe` |

The compiler rejects a use after a move and a release of a borrowed parameter.
Those two diagnostics catch most C habits:

```clorraine
import { stringOf } from std.text;

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
import { File, MODE_OLDFILE, openFile, readFile } from std.io;

func readInto(path: cstring, borrow buffer: u8[]): i32 {
    let maybe: File? = openFile(path, MODE_OLDFILE);
    if (maybe == null) {
        return -1;
    }
    let f: File = maybe;
    return readFile(f, buffer);
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
| `E10207` | An overlay (volatile) type used as a value | Access it through a `raw` pointer inside `unsafe`; never store, copy, or pass it by value |
| `E10306` | Storing an ephemeral closure | Use a plain function or a synchronous hook |
| `E10400` | A capturing lambda at a storing C API | Make it capture-less or use a synchronous hook |
| `E11200` | An Amiga module on the wrong profile | Select `hosted-amiga` |
| `E11215` | A capturing hook callback invoked after its call frame is gone | Keep the call synchronous; a capture-less callback has no guard |

The live registry and the fix hint for every code are in
[Chapter 14](../spec/14-diagnostics.md). `clorraine --explain E10020` prints
the same entry from the binary.

## 11. Wrapper cookbook

These are the shapes to copy when wrapping one more C facility.

### Opaque handle

```clorraine
export handle Device freedBy closeDevice;

ffi devices {
    func CloseDevice(take device: Device): void;
}

export func closeDevice(take device: Device): void {
    CloseDevice(device);
}
```

The handle owns the C resource. A caller that receives `Device` may pass it to
another `take` function or let it drop; it may not copy it.

### Borrowed view accessor

```clorraine
func payload(data: u8[], at: usize): u8[] {
    return slice(data, at, len(data));
}
```

The result borrows the same storage for the rest of the view's origin. Do not
return a view of a temporary or of a value the caller may move while the view
lives.

### Status plus payload

```clorraine
import { File, readFile } from std.io;

export struct ReadResult {
    ok: bool;
    count: i32;
}

export func readAll(borrow f: File, borrow buf: u8[]): ReadResult {
    let n: i32 = readFile(f, buf);
    return ReadResult { ok: n >= 0, count: n };
}
```

A short count is not a failure; the `ok` flag distinguishes an error from a
partial transfer.

### Synchronous callback

```clorraine
ffi dispatch {
    func ForEach(item: raw u8, borrow visit: func(raw u8): void): u32;
}

func dispatchOne(item: raw u8): u32 {
    let seen: i32 = 0;
    return ForEach(item, [seen] (value: raw u8) => {
        let n: i32 = seen;
    });
}
```

The C side must not store `visit`. A callback registered for later must be a
plain function and cannot capture.

## Where next

- [The language specification](../spec/README.md)
- [Known limitations](known-limitations.md)
- [The standard library appendix](../spec/appendix-stdlib.md)
