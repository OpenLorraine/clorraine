# Chapter 03 - Ownership, Moves, and Destructors

> **Status:** draft
> **Depends on:** Ch 00 (axioms), Ch 12 (FFI contracts), Ch 16 (borrow rules), Ch 17 (codegen)
> **Note:** example keywords (`own`, `borrow`, `borrow mut`, `move`, `raw`) are placeholders; the semantic modes are normative.

## 1. Values and resources

A **value** is a scalar, an aggregate (struct, array, tagged union), or a
**handle**. A handle names a resource with a release function: heap memory,
a library base, a file, a message port.

| Kind | Examples | Release function |
|---|---|---|
| Heap memory | `AllocVec` buffer | `FreeVec` |
| Library | `OpenLibrary` base | `CloseLibrary` |
| File | `Open` handle | `Close` |
| Port / IO request | `CreateMsgPort`, `CreateIORequest` | `DeleteMsgPort`, `DeleteIORequest` |

The release function comes from the FFI contract of the acquiring function
(Ch 12). The language itself knows nothing about AmigaOS specifics.

## 2. Ownership modes

| Mode | Meaning | May be copied | May be dropped |
|---|---|---|---|
| `copy` | Plain data; duplication is safe | yes | no (nothing to release) |
| `own` | Exclusive responsibility for release | no (moved) | yes, exactly once |
| `borrow` | Shared read-only access | yes (reborrow) | no |
| `borrow mut` | Exclusive access, no other live borrow | no (reborrow only) | no |
| `raw` | Unchecked pointer; no ownership | yes | no |

*OW-1* Every binding, parameter, and field has exactly one mode.
*OW-2* `copy` is a property of the type: scalars and aggregates whose fields
are all `copy` (marker: `copy struct`). Everything else is move-only.
*OW-3* `borrow` and `borrow mut` are temporary views; they never release.

## 3. Moves

A **move** transfers ownership. The source becomes unusable; any later read,
borrow, or drop of it is a compile-time error.

Moves happen at:

| Operation | Moved? |
|---|---|
| `let b = a;` where `a` is `own` or move-only aggregate | yes |
| Passing an `own` argument | yes |
| `return` of an `own` value | yes |
| Assignment to a field of a move-only type | yes |
| Copy of a `copy` value | no |

*OW-4* Moving a value while it is borrowed is an error (E10022).
*OW-5* Using a moved value is an error (E10020).
*OW-6* Aggregate moves are lowered as "copy then zero the source" so that
inserted drops are null-safe and cannot double-release (Ch 17 §4).

```clorraine
let a: own Buffer = Buffer.alloc(64);
let b: own Buffer = move a;      /* a is now invalid */
b.write(...);
/* b released at scope end; a has nothing left to release */
```

## 4. Borrows

Borrows grant access without ownership. Their lifetime is, in v1, the
**lexical scope** in which they are created; a later version adds non-lexical
regions (Ch 16).

| Rule | Statement | Diagnostic |
|---|---|---|
| OW-7 | A borrow may not outlive the value it borrows | E10030 |
| OW-8 | While a shared borrow is live, the owner may not be moved or exclusively borrowed | E10031 |
| OW-9 | While an exclusive borrow is live, no other borrow may be live and the owner may not be moved | E10032 |
| OW-10 | A borrow may not escape: not returned unless derived from a parameter, not stored in a struct field, not stored in a global | E10033 |

The `borrow mut` view is unique: creating it invalidates every other borrow of
the same owner until its last use in the scope.

```clorraine
let buf: own Buffer = Buffer.alloc(64);
let view: borrow Buffer = buf;       /* shared */
inspect(view);
/* view's last use is above; scope rules keep it live until here in v1 */
let edit: borrow mut Buffer = buf;   /* allowed in v2 non-lexical model;
                                        E10032 in the v1 lexical model */
```

The v1 lexical rule is intentionally conservative: it rejects some correct
programs rather than accepting wrong ones. Phase 3's non-lexical checker (Ch
16) replaces it without changing the source-level contract.

## 5. Destructors and drops

*OW-11* Every `own` binding that is still live at scope exit is dropped, in
reverse declaration order.
*OW-12* Drops are null-safe and exactly once: generated drop calls guard
against null, and moves zero the source.
*OW-13* Temporaries are dropped at the end of the statement that created them,
in reverse creation order.
*OW-14* There are no drops at process exit. Globals may therefore not be
`own` (Ch 03 §8); resources held for the whole program are released by the
program before returning.

```clorraine
function copyFile(): void {
    let src: own File = File.open("RAM:a");   /* drop at scope end */
    let dst: own File = File.open("RAM:b");   /* dropped first (reverse order) */
    let buf: own Buffer = Buffer.alloc(4096);
    memset(buf.borrow_mut());
    loop {
        let n: u32 = src.read(buf.borrow_mut());
        if n == 0 { break; }
        dst.write(buf.borrow());
    }
    /* drop buf, then dst, then src */
}
```

Failures use ordinary C-like control flow: an acquisition that can fail
returns a nullable owned handle, and the compiler's control-flow check
rejects use of a possibly-null value (E10010). There are no exceptions and no
unwind: drops run on every `return`, `break`, and `continue` that leaves a
scope, but not on `exit` (documented; the runtime or OS reclaims).

## 6. Functions and parameters

| Position | Default mode | Explicit alternatives |
|---|---|---|
| Scalar parameter | `copy` (by value) | - |
| Aggregate parameter | `borrow` (read-only) | `borrow mut`, `own` |
| Return value | `own`, `copy`, or a parameter-derived `borrow` | - |

*OW-15* An `own` parameter means ownership is transferred into the callee; the
caller's binding is invalidated.
*OW-16* A `borrow`/`borrow mut` parameter must not be released by the callee
and must not be stored beyond the call (E10034).
*OW-17* Returning a borrow is allowed only when it is derived from a
parameter; the result borrows that parameter's owner (single elided lifetime,
shortest of all parameters). Returning a borrow of a local is E10035.

```clorraine
function firstByte(data: borrow Buffer): borrow u8 {
    return data.at(0);          /* borrow of a parameter: allowed */
}

function bad(): borrow u8 {
    let local: Buffer = Buffer.alloc(4);
    return local.at(0);         /* E10035: borrows a local */
}
```

## 7. Aggregates

* Structs are move-only unless marked `copy`.
* `copy struct` requires every field to be `copy`.
* v1 structs may not contain `borrow` fields (E10036; OW-18); borrow-typed
  fields and struct lifetime parameters are a later extension.
* Arrays of move-only elements are move-only and are dropped element by
  element in index order.

```clorraine
copy struct Point { x: i32; y: i32; }        /* freely copyable */
struct Owning { name: own string; n: u32; }  /* move-only; drop releases name */
```

## 8. Globals

* Globals must be `copy` or `const`; `own` globals are rejected (E10037;
  OW-19) because there are no exit-time drops.
* A global borrow is rejected (E10038; OW-19).
* Mutable global state is allowed but any resource it references must be
  released by explicit code before the program returns.

## 9. The raw escape hatch

`raw` pointers exist for hardware registers, FFI boundaries, and code the
checker must not reason about.

* Raw operations (dereference, offset, cast to `raw`) require an explicit
  escape block (`unsafe` is the placeholder keyword) so their use is
  greppable and reviewable.
* Ownership never flows through `raw` pointers automatically; converting
  between `own` and `raw` is explicit and audited (Ch 12).
* `raw` may not be used to launder a borrow out of its scope.

## 10. Rules summary (normative list)

| Rule | Statement | Diagnostic |
|---|---|---|
| OW-1 | One mode per binding, parameter, field | - |
| OW-2 | `copy` is a type property | - |
| OW-3 | Borrows never release | - |
| OW-4 | No move while borrowed | E10022 |
| OW-5 | No use after move | E10020 |
| OW-6 | Aggregate move = copy + zero source | - (codegen) |
| OW-7 | Borrow may not outlive owner | E10030 |
| OW-8 | No move/exclusive borrow while shared borrow live | E10031 |
| OW-9 | Exclusive borrow is unique | E10032 |
| OW-10 | No escaping borrows | E10033 |
| OW-11 | Drop at scope exit, reverse order | - (codegen) |
| OW-12 | Drops null-safe, exactly once | - (codegen) |
| OW-13 | Temporaries dropped at statement end | - (codegen) |
| OW-14 | No drops at process exit | - |
| OW-15 | `own` parameter transfers ownership | - |
| OW-16 | Borrow parameters are call-scoped | E10034 |
| OW-17 | Returned borrows derive from parameters | E10035 |
| OW-18 | No borrow fields in v1 aggregates | E10036 |
| OW-19 | No `own` globals, no global borrows | E10037/E10038 |
