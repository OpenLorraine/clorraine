# Chapter 12 - FFI and Ownership Contracts

> **Status:** draft
> **Depends on:** Ch 03 (ownership), Ch 17 (codegen)
> **Validated by:** Phase 0 skeleton (`docs/findings-phase0.md`)

## 1. Purpose

Clorraine programs call the C ABI, and on the hosted profile that means the
AmigaOS libraries. C headers do not describe ownership: nothing tells the
compiler whether `OpenLibrary` returns something the caller must release, or
whether `Write` keeps the buffer. **Contracts** supply that missing half.

A contract is a trusted declaration. The compiler checks Clorraine code against
contracts; it cannot verify the external implementation. This is the same trust
model as Rust's `unsafe extern` boundary, and it is why contract definitions
are small, reviewed, and versioned.

## 2. Contract vocabulary

| Kind | Meaning | Example |
|---|---|---|
| `owns Deleter` | Caller becomes owner; must be dropped with `Deleter` | `OpenLibrary` -> `CloseLibrary` |
| `consumes` | The call releases the resource; the argument must be `own` | `CloseLibrary(base)` |
| `borrows` | Read-only use during the call; callee must not keep it | `Write(f, buf, len)` |
| `borrows mut` | Exclusive use during the call | `Read(f, buf, len)` |
| `value` | No ownership transfer in either direction | `IoErr()` |
| `nullable` | The result may be the zero handle; callers must check | `Open` returns `BPTR 0` on failure |
| `stores` | Callee keeps the pointer beyond the call (deferred; unsafe-only) | `AddIntServer` |
| `transfers` | Ownership crosses a task or message boundary (deferred) | `PutMsg`, `GetMsg`, `ReplyMsg` |

`stores` and `transfers` are intentionally out of scope for v1: they cannot be
checked at the C boundary and will be confined to audited raw wrappers.

## 3. Declaration format (v0)

Contracts live beside the modules that use them as FFI declarations. The
surface syntax below is a **v0 draft**; the contract *data model* (the columns)
is what the compiler consumes today. The keyword spelling is part of the
pending naming decision.

```clorraine
ffi exec {
    function OpenLibrary(name: cstring, version: u32): handle Library
        owns CloseLibrary nullable;
    function CloseLibrary(base: handle Library): void
        consumes;
    function AllocVec(size: u32, flags: u32): handle Memory
        owns FreeVec nullable;
    function FreeVec(mem: handle Memory): void
        consumes;
}

ffi dos {
    function Open(name: cstring, mode: i32): handle File
        owns Close nullable;                 /* BPTR, 0 means failure */
    function Close(file: handle File): void
        consumes;
    function Write(file: handle File, buf: borrow u8[], len: i32): i32
        value borrows buf;
    function Read(file: handle File, buf: borrow mut u8[], len: i32): i32
        value borrows mut buf;
}
```

Rules:

* *FFI-1* An extern function without a contract may only be called from an
  escape block; passing an `own` or `borrow` value to it is E10100.
* *FFI-2* A `consumes` parameter must receive an `own` value; the caller's
  binding is invalidated.
* *FFI-3* An `owns` result is an `own` binding; dropping it calls the named
  deleter; dropping a `nullable` result is a no-op when the handle is zero.
* *FFI-4* A `borrows` buffer must remain valid for the duration of the call
  only; `stores` contracts (when added) require a longer lifetime.
* *FFI-5* Contracts are per profile: `hosted-amiga` contracts name AmigaOS
  functions; the `freestanding` profile will name LorraineOS entry points.

## 4. Validated subset (v0)

The Phase 0 skeleton exercised these contracts on `vamos`, FS-UAE directory
drive, and FS-UAE ADF with GCC and vbcc:

| Function | Contract | Nullable | Validated |
|---|---|---|---|
| `OpenLibrary(name, ver)` | owns `CloseLibrary` | yes | yes |
| `CloseLibrary(base)` | consumes | - | yes |
| `AllocVec(size, flags)` | owns `FreeVec` | yes | yes |
| `FreeVec(mem)` | consumes | - | yes |
| `Open(name, mode)` | owns `Close` (`BPTR`) | yes (`0`) | yes |
| `Close(file)` | consumes | - | yes |
| `Write(file, buf, len)` | borrows `buf` | - | yes |

Next to add, in order of expected use: `CreateMsgPort`/`DeleteMsgPort`,
`CreateIORequest`/`DeleteIORequest`, `CreatePool`/`DeletePool`/`AllocPooled`,
`Read`, `Seek`, `Lock`/`Unlock`, then the `stores`-class registrations
(interrupt servers, hooks) as explicitly raw wrappers.

## 5. Boundary rules

| Situation | Rule |
|---|---|
| Passing `borrow` to legacy C | Allowed only when the contract says `borrows`; otherwise E10100 |
| Passing `own` to legacy C | Allowed when the contract says `borrows` (temporary view) or `consumes` (transfer) |
| Receiving a handle | Mode comes from the contract (`owns` or `value`) |
| Receiving a pointer to a C-owned object | `borrow` (contract), never `own` |
| `void *`-style untyped parameters | Represented as `handle Memory`; conversions require raw casts |
| Callbacks | Deferred: function values with C signatures exist, but `stores` contracts and lifetime guarantees are v2 |
| Ownership across tasks | Deferred: audited raw wrappers with documented transfers (message passing) |

## 6. Trust and review policy

* Contract tables are reviewed like code; each entry cites the NDK prototype
  it describes.
* The SDK ships the reviewed tables; users may add local ones but the compiler
  reports them as user-provided (W10110) so audits can find them.
* A later tool can diff contracts against `.fd`/`.sfd`/NDK headers for
  signature drift; ownership columns remain human-reviewed.

## 7. Deferred items

| Item | Why deferred |
|---|---|
| `stores` contracts (callbacks, interrupt servers, hooks) | Requires lifetime guarantees tied to a matching remove call |
| `transfers` (message ownership) | Crosses task boundaries; needs an explicit trusted-transfer model |
| Pool/arena release semantics | Bulk release invalidates many handles; needs a region model |
| Varargs functions (`PrintF`, `DoMethod`) | Needs a typed wrapper layer before contracts can apply |
