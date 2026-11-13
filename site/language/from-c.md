---
title: From C to Clorraine
description: "A Rosetta guide for Amiga C programmers: pointers, allocation, strings, MMIO, interrupts, and files."
---

# From C to Clorraine

This guide translates the C patterns an Amiga programmer already knows. It is a
set of excerpts, not a second specification. The full guide is
[docs/c-to-clorraine.md](https://github.com/OpenLorraine/clorraine/blob/main/docs/c-to-clorraine.md).

Clorraine keeps C's machine model: fixed-width integers, explicit layout,
pointers at the hardware boundary, and one C translation unit. What changes is
**who owns a value and when it is released**.

## Mental model

| C habit | Clorraine rule |
|---|---|
| A pointer may be null, borrowed, or owned; the type does not say | `T`, `T?`, `borrow T`, `take T`, and views make the mode explicit |
| `free` is called by convention | The owner releases a value once at scope end; `take` transfers the duty |
| A struct copy is a byte copy | A value copies only when its type is duplicable; owning types move |
| A cast reinterprets | `raw T` is explicit and `unsafe` at the hardware boundary |
| An error is a sentinel return | A nullable return or a status struct says what failed |

A parameter with no mode or with `const` is a read-only lend. `borrow` is the
writable lend, and `take` transfers ownership.

## Allocation

C:

```c
struct Buffer *b = AllocMem(sizeof *b, MEMF_CLEAR);
if (b == NULL) {
    return ENOMEM;
}
b->len = 0;
FreeMem(b, sizeof *b);
```

Clorraine:

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

A recursive owner uses `Box<T>`:

```clorraine
import { boxNew } from std.mem;

struct Node {
    next: Box<Node>?;
    value: i32;
}

func makeNode(): Box<Node> {
    return boxNew(Node { next: null, value: 1 });
}
```

## Strings

C has `char *` and `strlen`. Clorraine separates three things:

| Spelling | Meaning |
|---|---|
| `str` | A borrowed byte view, not NUL-terminated |
| `String` | An owned byte string |
| `cstring` | A borrowed NUL-terminated view for an `ffi` call |

```clorraine
import { String, stringOf, toCString } from std.text;

func nameLength(name: str): usize {
    return len(name);
}

func conversions(): void {
    let owned: String = stringOf("hello");
    let view: str = owned;
    let c: cstring = toCString(view);
}
```

A `String` does not implicitly become a `cstring`; the NUL-terminated form is a
distinct promise to the C side.

## MMIO

C:

```c
volatile UWORD *color = (UWORD *)0xDFF180;
*color = 0x0F00;
```

Clorraine keeps the address but makes the access auditable:

```clorraine
import { setColor } from std.amiga.custom;

func setBackground(): void {
    setColor(0, 0x0F00);
}
```

`setColor` writes the volatile register; the `unsafe` and raw address stay
inside the standard library.

## Interrupts

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

The handler must not allocate or drop an owning value. The restriction is part
of the interrupt contract, not a compiler limitation.

## Files

C:

```c
BPTR f = Open(path, MODE_OLDFILE);
if (f == 0) {
    return ERROR;
}
LONG n = Read(f, buffer, size);
Close(f);
```

Clorraine:

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

A count of zero or less is end of file or an error. Loop when the whole request
matters; wrap the count in a status struct when the caller must distinguish the
two.

## Where to go next

- [Ownership](/language/ownership)
- [Types and modules](/language/types)
- [Volatile and MMIO](/language/mmio)
- [Chapter 12 — FFI and Contracts](https://github.com/OpenLorraine/clorraine/blob/main/spec/12-ffi-contracts.md)
- [The full Rosetta guide](https://github.com/OpenLorraine/clorraine/blob/main/docs/c-to-clorraine.md)
