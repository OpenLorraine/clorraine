# Ergonomics corpus - pre-Phase-B friction sketches

Status: review input. Findings and recommendations live in
[`findings-ergonomics.md`](findings-ergonomics.md). Rulings are accepted; the
E4 sketch keeps its pre-ruling form as evidence (the ruled loop form is
`for (borrow e in xs)` for write-through, F1).

## Purpose and method

These sketches test the locked decisions from
[`../spec/decisions.md`](../spec/decisions.md) against real workloads before
the grammar is harvested in Phase B. They are written by hand, not compiled -
no compiler exists yet.

Rules for reading the sketches:

* The code is best-effort against the decision ledger. Forms the ledger does
  not fix yet are marked with `// gap:` in the sketch and collected in the
  grammar freeze checklist, now resolved in
  [`../spec/grammar-deltas.md`](../spec/grammar-deltas.md) section 8.
* Every friction entry cites the ledger section it comes from, so the review
  can trace a pain point back to a decision.
* Severity: **Blocker** (the workload cannot be written reasonably),
  **High** (forces workarounds every time), **Medium** (regular annoyance),
  **Low** (rare or cosmetic).

Personas used to judge each friction:

| Persona | Background | Habits they bring |
|---|---|---|
| P1 | C#/TypeScript developer | methods, properties, nullable references, pattern matching, exceptions |
| P2 | Pascal/Delphi developer | records, `T(expr)` casts, string helper functions |
| P3 | Go developer | explicit error returns, slices, methods, labeled break |
| P4 | Amiga C veteran | MMIO register writes, hooks, `RawDoFmt`, manual memory |
| P5 | Rust developer | lifetimes (absent here), traits, `match`, `Drop` |
| P6 | Python developer | f-string ease (provided), exceptions |

## E1 - Custom chip access (MMIO, copper, sprite setup)

Workload: write a hardware layer, then keep application code above it safe.

```clorraine
module Demo.Chips;

// Overlay over part of the custom chip register block. The volatile
// qualifier means the compiler never caches or reorders these accesses.
struct CustomRegs {
    dmacon:  volatile u16;
    cop1lc:  volatile u32;
    cop2lc:  volatile u32;
    color00: volatile u16;
}

const CUSTOM_BASE: u32 = 0x00DFF000;
const DMACON_COPPER: u16 = 0x0080;

export func enableCopper() {
    let regs = raw CustomRegs(CUSTOM_BASE);   // gap: member access through raw
    unsafe {
        regs.dmacon = regs.dmacon | DMACON_COPPER;
    }
}

export func setColor00(c: u16) {
    let regs = raw CustomRegs(CUSTOM_BASE);   // derived again: overlays cannot
    unsafe {                                  // be locals, copied, or borrowed
        regs.color00 = c;
    }
}
```

### Friction log

| ID | Observation | Severity | Frequency | Ledger |
|---|---|---|---|---|
| E1-F1 | Every hardware touch sits inside `unsafe`. On the Amiga target the hardware layer is a large share of low-level code, so `unsafe` becomes the normal way to write it | High | daily | T7 |
| E1-F2 | Member access through a `raw` pointer has no decided syntax (`regs.dmacon` assumed). The grammar must freeze this | High (spec gap) | daily | T6 |
| E1-F3 | The register base must be re-derived in every function because overlays cannot be locals, copied, passed, or borrowed. A module-level `const raw` pointer would help, but constant evaluation of raw pointer casts is undecided | Medium (spec gap) | daily | T7, T6 |
| E1-F4 | Whether the u32-address volatile intrinsics (`writeVolatile16(base + off, v)`) require `unsafe` is undecided. This one answer decides whether MMIO can ever look safe | High (doctrine) | daily | T7, T6 |
| E1-F5 | Real chipset overlays need manual padding fields; a layout mistake is silent. `packed`/`align` attributes are deferred | Medium | per overlay | not in ledger |
| E1-F6 | Integer address arithmetic outside `unsafe` works, so address constants compose cleanly | Low (positive) | - | T6 |

Verdict: writable, but only with a disciplined wrapper-module pattern (small
`unsafe` functions, safe API above). The plan for the standard library matters
more than any grammar change here.

## E2 - File load with error paths

```clorraine
module Demo.Load;

import { File, openFile, readFile, fileSize, lastError } from std.io;
import { Buffer, bufferNew, bufferBytes } from std.mem;

export func loadInto(path: str) : Buffer? {
    let f: File? = openFile(path);
    if (f == null) {
        return null;                        // gap: no detail travels with null
    }
    let size = fileSize(f);
    let buf = bufferNew(size);
    if (buf == null) {
        return null;                        // f drops automatically here
    }
    readFile(f, borrow bufferBytes(buf));   // gap: writable lend of a view of a borrow
    return buf;                             // moves out; f drops at exit
}
```

### Friction log

| ID | Observation | Severity | Frequency | Ledger |
|---|---|---|---|---|
| E2-F1 | Owned handles drop automatically on every exit path. No `defer`, no `close`, no double-close bookkeeping | Low (positive) | - | D1, O1, O2 |
| E2-F2 | `T?` carries no error detail, so the caller cannot tell "not found" from "out of memory" from "read error". Every useful IO API needs a status struct or an error out-parameter, which is C-style ceremony | High | daily | E1, E3, T8 |
| E2-F3 | No `?.`/`??`; nullable paths are explicit `if` plus early return. Readable, but Go-level verbosity, and the detail-less `T?` pushes users into the more verbose status-struct pattern | Medium | daily | E5, T8 |
| E2-F4 | Passing a writable view of a borrow (`borrow bufferBytes(buf)`) is not addressed by the decisions. It must work, or bulk writes are impossible without `unsafe` | High (spec gap) | daily | V11, T9 |
| E2-F5 | No owning byte-buffer type exists. The decisions define `String`/`str`/`cstring`, but bitmaps, samples, and file buffers need `Buffer`. Without it, bulk IO needs `raw` and `unsafe` | High | daily | T2 |
| E2-F6 | What a partial read or write returns (`i32`, `usize`, `bool`) is undecided; each API invents its own convention | Medium (spec gap) | weekly | E3 |

Verdict: the ownership model makes cleanup excellent. The gap is the error
and buffer surface around it, and that lives mostly in the standard library.

## E3 - Text formatting and debug output

```clorraine
module Demo.Dump;

import { println } from std.io;
import { toHex, padLeft } from std.fmt;

export func dump(name: str, size: u32, addr: u32) {
    println($"name={name} size={size} addr={toHex(addr)}");
    let text = padLeft(toHex(addr), 8, '0');   // gap: char literal type
    println($"addr={text}");
}
```

### Friction log

| ID | Observation | Severity | Frequency | Ledger |
|---|---|---|---|---|
| E3-F1 | No format specs means hex and padded output need helper calls. `toHex` alone does not fix the width, so users must remember `padLeft`; address printing is daily work in systems code | Medium | daily | S3 |
| E3-F2 | Character literals have no decided type (`'0'` used as `u8` above). The type set has no `char` | Low (spec gap) | daily | T1, S2 |
| E3-F3 | Every template allocates a `String`. Fine for logs, wrong for hot paths and forbidden in interrupt context. Users must learn when to avoid it | Medium | weekly | S5 |
| E3-F4 | Templates, implicit `String` to `str`, and statement-end temporary drops give C#-like print ergonomics with no lifetime fight | Low (positive) | - | S1, S5, D5 |
| E3-F5 | No methods (`len(text)` instead of `text.len()`). Consistent with the C-like identity, but the largest "feel" gap against the easy languages | Medium | daily | D8, T2 |

Verdict: genuinely pleasant for the common case. Hex-heavy debug output is the
one place C-style helpers show through.

## E4 - Records, slices, and mutation

```clorraine
module Demo.Entities;

struct Entity {
    kind: u8;
    x: i32;
    y: i32;
}

func moveBad(e: Entity, dx: i32) {
    e.x = e.x + dx;               // error: e is a read-only lend
}

func moveGood(borrow e: Entity, dx: i32) {
    e.x = e.x + dx;               // mutates the caller's record
}

func moved(e: Entity, dx: i32) : Entity {
    let c = e;                    // copy, because Entity is duplicable
    c.x = c.x + dx;
    return c;
}

export func moveAll(borrow xs: Entity[], dx: i32) {
    for (e in xs) {
        e.x = e.x + dx;           // needs writable element views
    }
}

func totalWidth(xs: Entity[]) : i32 {
    let sum: i32 = 0;
    for (e in xs) {
        sum = sum + e.x;
    }
    return sum;
}

export func half(buf: Entity[]) : Entity[] {
    return slice(buf, 0, len(buf) / 2);   // view return from a parameter
}
```

### Friction log

| ID | Observation | Severity | Frequency | Ledger |
|---|---|---|---|---|
| E4-F1 | `moveBad` is the most common first error: C copies, Rust moves, Clorraine lends read-only. The error message is the whole recovery experience | High | very high | V5 |
| E4-F2 | What `for (e in xs)` binds is undecided: a copy (C# `foreach`, no write-through) or a writable element view (`moveAll` depends on this). Must freeze | Blocker (spec gap) | daily | B2, T9 |
| E4-F3 | Bounds checking on `xs[i]` is undecided: safe and slow on a 68000, or fast and C-like | High (spec gap) | daily | T9 |
| E4-F4 | No range syntax: `slice(buf, 0, n)` plus `len(buf)` instead of `buf[0..n]`. Clear, slightly noisy | Low | daily | B4, O9 |
| E4-F5 | Struct field punctuation and enum member access are undecided | Low (spec gap) | per file | not frozen (section 12) |
| E4-F6 | View returns derived from parameters make slices returnable with no lifetime annotations | Low (positive) | - | O9, T9 |

Verdict: the model is teachable once the element-loop rule is frozen. The
diagnostic for the read-only lend mistake will decide how the language feels.

## E5 - Timer interrupt and callbacks

```clorraine
module Demo.Timer;

import { addTimer, removeTimer } from std.amiga.interrupt;

let ticks: u32 = 0;

func onTick() {
    ticks = ticks + 1;            // gap: shared with an interrupt, but not volatile
}

export func start() : u32? {
    return addTimer(onTick);      // gap: function reference form and ABI qualifier
}

export func stop(id: u32?) {
    if (id != null) {
        removeTimer(id);          // gap: nullable parameter is banned by T8
    }
}
```

### Friction log

| ID | Observation | Severity | Frequency | Ledger |
|---|---|---|---|---|
| E5-F1 | Globals shared with an interrupt have no volatile story. A polling loop can read a stale value: the classic Amiga bug. `volatile` exists only for fields and intrinsics | High | daily | T7 |
| E5-F2 | A Clorraine function cannot carry the m68k calling-convention qualifiers an interrupt handler or shared-library entry needs (`__interrupt`, `__saveds`). Without a story, the language cannot serve interrupts directly | Blocker | per handler | not frozen (section 11) |
| E5-F3 | Function-as-value syntax is undecided (bare `onTick` versus `&onTick`) | Medium (spec gap) | daily | F3 |
| E5-F4 | Capture-less lambdas are plain C function pointers, so callbacks cost nothing and need no allocation or boxing | Low (positive) | - | L3 |
| E5-F5 | Interrupt context forbids allocation and drops, but nothing in the language marks a function as interrupt-safe. It must be a documented dispatcher contract | Medium | per handler | C1 |
| E5-F6 | Nullable parameters are banned, so optional-handle APIs (`stop(null)` as "stop all") are not expressible; callers must check first or take a status struct | Medium | weekly | T8 |
| E5-F7 | Narrowing with `!= null` works and is local and readable | Low (positive) | - | T8 |

Verdict: the blocker is the ABI boundary, not ownership. A standard-library
dispatcher (C-side `__interrupt` handler plus a table of Clorraine functions)
removes it for application code.

## E6 - Linked structure (list or tree)

```clorraine
module Demo.List;

import { allocMem, freeMem, sizeof } from std.mem;

struct Node {
    value: i32;
    next: raw Node?;
}

export func pushFront(borrow head: raw Node?, v: i32) : bool {
    let p = raw Node(allocMem(sizeof(Node)));   // gap: sizeof
    if (p == null) {
        return false;
    }
    unsafe {
        p.value = v;
        p.next = head;
    }
    head = p;
    return true;
}

export func freeList(head: raw Node?) {
    let p = head;
    while (p != null) {
        let next = p.next;      // deref, so this lives inside unsafe too
        freeMem(p);
        p = next;
    }
}
```

### Friction log

| ID | Observation | Severity | Frequency | Ledger |
|---|---|---|---|---|
| E6-F1 | Allocation returns a raw pointer; casts, field access, and linking all need `unsafe`. Every allocation site is unsafe | High | weekly | T6, X4 |
| E6-F2 | Custom recursive types cannot have automatic cleanup: `freedBy` needs an FFI deleter, and application code cannot declare `ffi` blocks. No RAII for user structures | High | weekly | X1, X6 |
| E6-F3 | `sizeof` is not decided | Medium (spec gap) | per structure | - |
| E6-F4 | No `Box<T>` or generic owning pointer, so there is no safe way to model recursive data | High | weekly | G8, C2 |
| E6-F5 | `borrow head: raw Node?` gives a C-style out-parameter update pattern; zero means empty, so raw values never need drops | Low (positive) | - | O7, T6 |
| E6-F6 | Forgetting `freeList` leaks with no help from the type system | High | weekly | X1 |

Verdict: linked structures are C with a checker watching the shallow parts.
For most Amiga game code, a standard-library dynamic array removes the need.
The credibility cost stays: every C#/Go/Pascal reviewer will try this example
first.

## Cross-sketch summary

| Friction family | Sketches | Severity | Where it can be fixed |
|---|---|---|---|
| MMIO and interrupt access is entirely `unsafe` | E1, E5 | High | doctrine plus standard-library wrappers |
| No safe heap or recursive structures | E6, E2 | High | standard library (buffer, dynamic array), later `Box<T>` |
| Element-loop and bounds semantics undecided | E4 | Blocker | grammar and semantics freeze |
| Error detail and nullable API verbosity | E2, E5 | High | standard library result types, grammar reservation |
| Missing convenience layer (methods, format specs) | E3, E4 | Medium | additive later, standard-library naming now |
| Grammar gaps found by writing real code | all | High | Phase B freeze checklist |
