# Requirements: v1.1 Language Surfaces

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)
> **Implements**: R1-R13
> **Decision authority**: the goal text rules the three surface amendments;
> eligible technical mechanisms are delegated and recorded in the
> [Ambiguity Register](00-ambiguity-register.md)

## Feature Overview

Clorraine v1.0 has three staged language surfaces. Each one is about an owned
value whose release must happen exactly once:

1. `Box<T>` is a safe owned pointer. It allocates storage from the profile
   allocator, moves a value into that storage, releases the value recursively,
   and frees the storage. A box of a box, a box of a struct with owned fields,
   and a box of a growable container must all drop exactly once and leave no
   leak or double free.
2. X1 currently requires a `freedBy` deleter to be an `ffi`-declared function.
   The amendment accepts an ordinary Clorraine function with the same
   `take T -> void` signature. This lets the standard library and application
   code own a recursive resource without writing an `ffi` block.
3. L5 lets a capturing lambda cross a synchronous C hook. The C boundary keeps
   a plain function pointer; the compiler emits a trampoline and a
   caller-stack environment, so no heap environment and no C ABI change are
   introduced. Capture-less lambdas remain plain function pointers.

The specification is frozen. The three amendments are approved by the goal
text; every other chapter change is a restatement of those amendments. No
`spec/tests` golden changes.

## Functional Requirements

### R1 - `Box<T>` type and API

**R1.1 Type.** `Box<T>` is a built-in generic owning type. It is spellable in
a type position without a declaration, accepts exactly one type argument, and
is distinct from every declared struct, enum, and handle. The name is
predeclared in the resolve stage as a type-only name (the resolver currently
predeclares only `String`), so `Box<T>` reaches the type stage without
`E10219`. `Box` is a reserved built-in type name: a user declaration of a
struct, enum, handle, or function named `Box` is `E20006`. The name is not a
value: `Box<T>(x)` and `Box<T> { ... }` are rejected.

**R1.2 Representation.** A `Box<T>` value is one opaque pointer to a heap
block that holds exactly one `T`. Its C spelling is `void *`; its size is the
target pointer size; its zero value is `null`. It has no accessible fields:
member access, indexing, and call are the ordinary "not a member / not
indexable / not callable" diagnostics for its displayed type.

**R1.3 API.** `std.mem` provides exactly these operations:

| Signature | Behavior |
|---|---|
| `boxNew<T>(take value: T): Box<T>` | Allocates one `T` block, moves `value` into it, returns the box |
| `boxSet<T>(borrow b: Box<T>, take value: T): void` | Drops the current pointee, then moves `value` into the same block |
| `boxWith<T, F: func(T): void>(borrow b: Box<T>, f: F): void` | Lends the pointee to `f` read-only for the duration of the call |

The three declarations carry no `export` marker, because a generic function
cannot be exported (`E10302`); the compiler's standard-library export table
lists `boxNew`, `boxSet`, and `boxWith` exactly as it lists `arrayNew` and
`push`. The table also lists `Box` as a compiler-known export, so
`import { Box } from std.mem;` is accepted even though the type has no
declaration.

`boxPtr<T>(borrow b: Box<T>): raw T` is a module-private helper used by the
three operations. It is not part of the public surface. `boxAlloc<T>(): Box<T>`
is a compiler intrinsic declared in `std.mem`; it allocates a zeroed block and
is recognized in both the free and module-qualified call paths. Its name joins
`COMPILER_INTRINSICS`, so a user declaration is `E20006`; it is not added to
`RESERVED_BUILTINS`, because that list drives the parser's intrinsic-call
spelling and `boxAlloc<T>()` is an ordinary generic call node. The lowering and
emitter handle it explicitly (an unhandled intrinsic currently emits `0`). The
intrinsic is a low-level allocator: a qualified `std.mem.boxAlloc<T>()` creates
a zeroed box and is safe to drop or fill with `boxSet`; it is not part of the
documented API and is not listed in the import table.

`boxWith` needs type inference from `Box<T>`: the checker's parameter matcher
must recurse into `Box` so `T` binds from `b` and `F` binds from the lambda.
The explicit form `boxWith<String, func(String): void>(b, lambda)` is not the
supported spelling, because a capturing lambda does not convert to a concrete
`func(...)`; the callable-bound inference path is the supported one.

**R1.4 Validation.** `Box` with zero or two or more type arguments is
`E10300`. `Box<T>` with a non-static view-typed `T` (`str`, `cstring`, a
slice, a fixed array of such views, or a container whose element is such a
view) is `E10227`. `Box<static str>` is accepted, matching the struct-field
rule that allows a static literal field. `Box<T>` with an overlay `T` is
`E10207`. `Box<T>` with an ephemeral closure `T` is `E10306`. A `Box<T>` where
`T` is a generic parameter is accepted in a generic body and judged at each
instantiation. The view check recurses through `Box`, `Array`, fixed arrays,
and struct type arguments after substitution.

**R1.5 Construction.** A `Box<T>` value is produced only by `boxNew` (or by
moving another box). A struct-literal spelling for `Box` is rejected with the
ordinary struct-literal diagnostic; `Box<T>(x)` is rejected as a type
conversion, and a `Box<T>` cannot be fabricated from a raw pointer in safe
code. A raw cast of a `Box<T>` operand (`raw T(b)`) outside the compiler-known
`std.mem` module is rejected with `E10201`; the library's private `boxPtr`
helper is the only sanctioned spelling. This keeps the pointee pointer and the
move-out path out of application code.

### R2 - Box allocation and profiles

**R2.1 Allocator.** Box storage comes from the profile's existing general
allocator: `cl_alloc`/`cl_free` on `host`, `hosted-amiga`, and
`hosted-amiga-13`. The allocation is zeroed so the initial pointee is the
language's empty state and the first store drops a no-op. An allocation
failure keeps the existing allocator contract: the runtime prints
`error E1: out of memory` and aborts. `boxNew` never returns a null box.

**R2.2 Profiles.** `Box<T>` is available on `host`, `hosted-amiga`, and
`hosted-amiga-13`. `std.mem` is not available on `freestanding`; the existing
profile rule rejects the module with `E11200` (`'std.mem' is not available on
profile 'freestanding'`). A written `Box<T>` in a freestanding program reports
the same profile rule rather than emitting a call to a missing allocator: the
type-side profile check gains a `mentions_box` arm next to
`mentions_heap_string`, so `Box<T>` anywhere in a declared type is `E11200`.

**R2.3 No new allocator.** No pool, arena, chip-memory, or freestanding
allocator is added. A future freestanding allocator is a separate approved
feature.

### R3 - Box ownership, moves, and nullability

**R3.1 Move-only.** `Box<T>` is not duplicable. Assigning, passing by value,
returning, or moving it transfers ownership; a second use of the moved binding
is `E10020`. A lend (`T`, `const T`, `borrow T`) keeps the caller's ownership
and creates the ordinary shared or exclusive loan.

**R3.2 Drop is exactly once.** Every owned `Box<T>` drops on its normal exit
path, including early returns, `defer` actions, loop iterations, and partial
moves of an enclosing struct. The drop nulls the box so a second drop is a
no-op. Reassignment (`b = boxNew(...)`) releases the old box before storing the
new one; `emits_drop` must include `Box<T>` so the assignment path does not
silently overwrite the old pointer. The same applies to a Box in a struct
field, a fixed array element, and a conditional arm.

**R3.3 Nullability.** `Box<T>?` is valid. `null` is the only empty state, and
using a possibly-null box without an `== null` or `!= null` narrowing check is
`E10010`. `Box<T>` itself is non-nullable; `null` in a non-nullable box
position is `E10012`. `boxNew` returns a non-nullable `Box<T>`; a
`Box<T>?` slot accepts it through the ordinary nullable widening.

**R3.4 Partial moves.** `Box<T>` has no fields, so it has no partial move. A
boxed value cannot be moved out in v1.1; `boxSet` replaces it and its drop
releases it. A raw cast of a `Box<T>` operand is reserved to `std.mem`; an
application `raw T(b)` is `E10201`, so the move-out path is not reachable from
application code without an audited unsafe escape. This limitation is
documented in `docs/known-limitations.md`.

**R3.5 Self-reference.** A struct may hold `Box<StructName>?` (for example
`struct Node { next: Box<Node>?; }`); the box is pointer-sized and breaks the
by-value recursion, and the drop recurses at run time. This is the supported
recursive-data pattern. A non-nullable `Box<StructName>` field is also
accepted; its initializer must supply a box. There is no `Self` keyword; the
requirement spells the concrete struct name.

### R4 - Box recursive drop glue

**R4.1 Recursive drop.** Dropping `Box<T>`:
1. returns immediately when the box is null;
2. drops the pointee `T` through the existing recursive drop walk;
3. releases the heap block with `cl_box_free`;
4. sets the box to `null`.

The pointee's fields drop in declaration order (O2); nested boxes recurse;
fixed arrays drop element by element; a growable `Array<T>` drops its elements
before its buffer. The storage is released only after the pointee drop.

**R4.2 Generated helper.** The emitter generates one `static inline` drop
helper per used `Box<T>` element type and calls it from every drop site,
including the expression form used by a guarded operand. The helpers are
collected transitively through struct fields, fixed arrays, `Array` elements,
and nested boxes after instance substitution. Every helper has a prototype
before any definition, so mutual recursion (`A` holds `Box<B>`, `B` holds
`Box<A>`) is valid C99. The helper is idempotent for a null pointer and for an
already-nulled box.

**R4.3 Exactly-once proof.** The implementation test pins the emitted helper
shape and the drop-site call. The behavior suite runs the following under
AddressSanitizer, with leak detection, and requires no report:
- `Box<i32>` created and dropped;
- `Box<String>` with a heap string;
- `Box<StructWithString>`;
- `Box<Box<String>>`;
- `Box<Array<String>>`;
- a three-node `Node { next: Box<Node>? }` list;
- a mutually recursive `A`/`B` pair, both directions boxed;
- a `boxSet` replacement of a `String`;
- a reassignment `b = boxNew(...)` over a live box;
- a box in a loop body and a box dropped by a `defer`.

**R4.4 No double free.** A box moved into a struct, a box returned from a
function, a box in a conditional arm, and a box dropped after a failed early
return each release exactly once.

### R5 - Box access, views, and generic arguments

**R5.1 Safe access.** `boxWith` is the only safe access. Its callback
receives a read-only lend of the pointee for the call. A view derived from the
pointee (for example `bytes(s)` of a boxed `String`) may be used inside the
callback and cannot escape: the callback returns `void`, and storing a lend in
a field or global is the existing `E10033`. A capturing lambda may be passed
as `F`; it is called synchronously inside `boxWith`, so its environment is
live for the call. `T` binds from `Box<T>` in the checker's parameter matcher;
the call is `boxWith(b, [..] => ...)` with no explicit type arguments.

**R5.2 No escaping view.** `Box<str>`, `Box<cstring>`, `Box<u8[]>`, a fixed
array of such views, and a container whose element is such a view (for
example `Box<Array<str>>`) are `E10227`. `Box<static str>` is accepted because
static storage cannot dangle. A box cannot carry a borrow out of its origin.

**R5.3 Generic arguments.** `Box<Box<String>>`, `Box<Struct>`,
`Box<Array<i32>>`, `Box<raw T>`, and `Box<func(i32): i32>` are accepted.
`Box<closure>` is `E10306`. A generic body may use `Box<T>`; each instance is
monomorphized with the usual key. A `Box<T>` type argument that is still an
unsubstituted parameter is accepted inside the generic body and judged at the
instantiation.

**R5.4 `sizeof` and constants.** `sizeof(Box<i32>)` folds to the target
pointer size. A struct that contains a `Box<T>` field has a known size and
offset: `sizeof(StructWithBox)`, `offsetof(StructWithBox, field)`, and an array
extent written with either fold correctly. `Box<T>` is not a constant-expression
value; it cannot appear in an enum member, extent, or `case`.

### R6 - Box FFI crossing and diagnostics

**R6.1 Direct crossing is rejected.** A `Box<T>` parameter or return type in
an `ffi` declaration reports the new `E10105` (`'{type}' cannot cross the FFI
boundary`) instead of emitting the known invalid generic-aggregate prototype.
The check runs in `ffi::check_items` with the `TypeTable` and the declaring
module, resolves each signature type with `resolve_ty_folded` (so import
aliases work), and recurses through arrays, raw pointers, function-pointer
parameters, and returns. It rejects any `Box<T>` and any generic struct
instance anywhere in an `ffi` signature, so accepted source never emits
`ClWrap` for `ClWrap__i32` or a `void *` box the C side cannot interpret.

**R6.2 Interop path.** A caller that needs C interop uses `boxWith` to pass
the pointee's fields or copies to the C function during the callback. A raw
pointer to the boxed storage is not part of the public surface.

**R6.3 Diagnostics.** The Box surface adds:
- `E10105` for a generic aggregate or box in an `ffi` signature;
- `E10227` for a view-typed box element;
- the existing `E10010`, `E10012`, `E10020`, `E10207`, `E10300`, and `E10306`
  at their registered triggers.

No Box-specific runtime fault is added. Allocation failure uses the existing
allocator trap.

### R7 - Ordinary `freedBy` deleters (X1)

**R7.1 Acceptance.** A `handle T freedBy F;` declaration accepts `F` when `F`
resolves to an ordinary Clorraine function, an exported function, or an
`ffi`-declared function with exactly:
- one parameter, mode `take`;
- the parameter type is exactly the handle type `T`, compared as a resolved
  `Ty` in the handle's declaring module (import aliases included), not by
  spelling the last AST segment;
- return type `void`, non-nullable;
- no type parameters (a plain non-generic function in v1.1).

The handle's declaring module and the resolved deleter declaration
(`module`, simple name, ordinary/`ffi` kind) are recorded once in `HandleInfo`
and used by both the validator and the emitter, so the check and the emitted
call cannot disagree.

**R7.2 Resolution.** The deleter name is resolved in the handle's declaring
module, not program-wide:
- a qualified name resolves in the named module;
- a simple name first looks at the declaring module's own functions, then its
  `import` bindings, then a unique program-wide declaration; an ambiguous
  simple name is invalid.

A same-module private function is valid; an exported cross-module function is
valid. A name that resolves to no declaration, to an `intrinsic`, or to more
than one declaration is invalid.

**R7.3 One per type.** A handle type has exactly one deleter. A second
`handle` declaration for the same type is the existing duplicate-declaration
`E11101`, including a declaration in a different module. Handle origins are
recorded like struct origins (`StructInfo`/`struct_origins` precedent) so the
cross-module duplicate is reported; the current per-module duplicate check
alone is insufficient. The first declaration's deleter identity is the one
both stages use.

**R7.4 Drop call.** The emitted drop calls the ordinary function through its
mangled symbol (`cl_<module>_<simple name>`) and the `ffi` function through its
simple C symbol. The call is null-safe and exactly once, as for an `ffi`
deleter. The emitter must not pass a dotted name to `function_symbol` (which
would produce `cl_A_B_A_B_freeX`) and must not emit a dotted name as a C
symbol. Qualified names preserve the module boundary because the module and
simple name are stored separately.

**R7.5 Owned fields.** A deleter body may itself drop owned fields. The
ordinary ownership rules inside the body release those fields exactly once;
the handle drop calls the deleter once and then empties the handle. A deleter
that releases a `String` field is ASan-clean.

**R7.6 Move-only.** An ordinary deleter does not make its handle duplicable.
The handle remains move-only; copying it is `E10020`.

### R8 - `freedBy` rejection and diagnostics

The registered `E10102` first line and help are updated to cover both kinds
of deleter:

| Rejected form | Diagnostic |
|---|---|
| name resolves to no function | `E10102` |
| name resolves to an `intrinsic` | `E10102` |
| zero or two or more parameters | `E10102` |
| parameter mode not `take` | `E10102` |
| parameter type is not exactly `T` | `E10102` |
| return type not `void` | `E10102` |
| nullable return type | `E10102` |
| a second deleter for the same type | `E11101` |
| `freedBy` on a native struct | `E10104` |

The diagnostic's first line stays `'{name}' is not a valid deleter for
'{type}'`; the help becomes `declare a function taking 'take T' and returning
'void'`. No new code is allocated for X1.

### R9 - `freedBy` emission and ownership

**R9.1 Symbol.** The emitted drop of a handle calls the deleter symbol once,
null-safely, in the cleanup ladder. For an ordinary deleter the symbol is the
module-mangled function name; for an `ffi` deleter it is the C name.

**R9.2 Qualified names.** A deleter declared in another module resolves to
that module's function symbol. A simple name keeps the existing scope order.

**R9.3 Existing ABI.** A handle remains an opaque pointer; the deleter's
parameter is that pointer by value. No FFI prototype changes.

**R9.4 Generic instance coverage.** `Box<T>`'s generated drop helper is a
generic-instance-shaped drop path, and its release is exercised for scalar,
struct, string, nested-box, and container instances. A user handle whose
ordinary deleter drops an owned struct field is also covered.

### R10 - Synchronous C-hook contract (L5)

**R10.1 Contract marker.** An `ffi` parameter of function type with mode
`borrow` is a **synchronous callback**: the callee may call it zero or more
times during the call and must not store it, return it, or call it after the
call returns. The mode does not change the C prototype; the parameter stays a
plain function pointer.

**R10.2 Acceptance.** A capturing lambda argument is accepted when the callee
is an `ffi` function and the corresponding parameter is a `borrow` function
type whose parameter and return types match the lambda and whose signature is
ABI-identical to the lifted function. The accepted hook signature set is:
- scalar, `bool`, enum, `raw`, `cstring`, and handle parameters and returns;
- non-generic by-value struct parameters and returns;
- function-pointer parameters and returns.

The following are rejected with `E10210` (a view, borrow, or fixed-array
callback cannot satisfy the plain C function pointer ABI the language emits):
- `str`, `static str`, slice, and fixed-array *parameters*;
- a `borrow` parameter of a non-duplicable struct (the language ABI is
  `T *`, the FFI ABI is by value);
- a fixed-array return (the language ABI is a hidden out-parameter);
- a generic by-value struct instance (the `ClWrap`/`ClWrap__i32` mismatch).

A capture-less lambda is accepted anywhere a matching `func(...)` is expected
and remains a plain function pointer. The existing generic callable-bound
acceptance (`F: func(T): R` receiving a capturing lambda for a synchronous
Clorraine call) is unchanged; `boxWith` depends on it.

**R10.3 Rejection.** A capturing lambda at any other concrete `func(...)`
position is `E10400`:
- an unmarked, `const`, or `take` `ffi` callback parameter (including
  `newHook`, `createTask`, and `addHandler`);
- a user function's concrete `func(...)` parameter;
- a local, field, global, or return position (the existing `E10404` applies
  when the closure is stored, returned, or captured).

A generic callable-bound parameter (`F: func(T): R`) keeps its existing
acceptance; it is not a concrete `func(...)` position.

**R10.4 Capture rules.** The existing capture rules are unchanged: a
duplicable capture copies, a non-duplicable capture borrows read-only, a
written capture needs `[borrow x]`, and a `take` capture is `E10403`.

### R11 - Trampoline ABI and environment

**R11.1 Plain pointer stays.** A capture-less lambda passed to a hook compiles
to the lifted function name; no trampoline, environment, stack node, or
push/pop is emitted.

**R11.2 Trampoline.** For each capturing callback argument the compiler emits
one `static` C function with exactly the declared `func(...)` signature, and
the accepted signature is restricted to the ABI-identical set in R10.2. It
finds its active environment by a unique hook-argument id, then calls the
lifted lambda with the environment as the first argument. The function pointer
passed to C is the trampoline. The C ABI is unchanged.

**R11.3 Stack environment.** The environment is the existing per-lambda
environment struct, materialized in the caller's frame. It holds copies of
duplicable captures and pointers to borrowed captures; it owns no droppable
resource, so no heap environment and no environment drop glue are introduced.
The borrowed captures' loans end when the call returns.

**R11.4 Lifetime.** The environment and its stack node are live from before
the C call until the call returns. The C callback must be synchronous. A
callback invoked after the call returns is a contract violation; the
trampoline detects the missing node and reports the new runtime fault
`E11215` instead of dereferencing a dangling pointer.

**R11.5 Reentrancy and nesting.** A compiler-emitted linked stack holds the
active nodes. Each node carries its hook-argument id; a trampoline searches the
stack for its own id. Nested calls, recursive calls, and two callbacks in one
C call all find their own environment. The stack is single-threaded, matching
the language; an interrupt handler that uses a hook follows the existing
allocation-free/drop-free handler rule and is out of the capturing path. A
capturing hook call inside a generic function body is `E10400` with help to
move the call into a non-generic wrapper; the environment and trampoline names
would otherwise need a per-instance record that v1.1 does not add. A
capture-less hook call inside a generic body stays a plain pointer and is
accepted.

**R11.6 Call wrapper.** A capturing hook call lowers through a generated
`static` C wrapper that pushes the environment node(s), calls the `ffi`
function with the trampoline pointer(s), and pops the node(s) in reverse order.
The wrapper preserves the `ffi` parameter modes and the frozen left-to-right
argument evaluation order. The capturing closure (and therefore its
environment) is materialized into a statement temporary at the callback
argument's own source position, before any later argument's effects run; the
wrapper's extra environment argument reads that temporary. Appending the
closure expression at the end of the call without materializing it would let a
later sibling argument change the captured value, which the closure
argument's own position forbids. A `take` argument still moves into the call.

**R11.7 Multiple callbacks.** One `ffi` call may carry two or more capturing
callbacks. Each argument gets its own trampoline, id, and node; no ordering
assumption is made.

### R12 - Hook errors and runtime fault

| Case | Behavior |
|---|---|
| capturing lambda at a non-`borrow` `ffi` callback | `E10400` |
| capturing lambda at a user `func` parameter | `E10400` |
| capturing closure stored, returned, or captured | `E10404` |
| `take` capture | `E10403` |
| lambda signature mismatch at a `borrow` hook | `E10210` |
| callback invoked after the hook returns | runtime `E11215` |
| a stored callback API used with a capturing lambda | `E10400` |
| callback that is itself a `func` value | accepted as a plain pointer only |

`E11215` is registered as a runtime fault: first line
`error E11215: callback invoked after its C hook returned`, help `keep the
callback synchronous; it may run only during the call`. It aborts without
running the cleanup ladder, like the other runtime faults.

### R13 - Specification amendments, tests, and gates

**R13.1 Approved amendments.** The goal text approves these amendments; each
is recorded in `spec/decisions.md` and the affected chapter or appendix:
- X1: ordinary Clorraine functions as `freedBy` deleters.
- L5: synchronous C-hook trampolines for capturing lambdas.
- P7: capture-less lambdas stay plain function pointers; capturing lambdas
  cross synchronous C hooks through generated trampolines.
- The `Box<T>` surface in the standard-library appendix, its drop rule in the
  structs chapter, its recursive-self-reference rule, and its profile rule.
- The new diagnostics `E10105`, `E10227`, and `E11215`, and the updated
  `E10102` and `E10400` triggers/help. This amendment authorizes the matching
  new specification tests: `spec/tests/compile-fail/E10105_*.clo`,
  `spec/tests/compile-fail/E10227_*.clo`, and
  `spec/tests/run/E11215_*.clo`, plus adding `E11215` to the runtime-fault
  exclusion list in `spec/check.sh`. These are new tests for approved
  diagnostics, not edits to an existing golden.

**R13.2 No existing golden change.** No existing file under `spec/tests/`
changes. The frozen corpus and every existing compile-fail/run golden stay
byte-identical. The three new files authorized by R13.1 are additions; the
`spec/check.sh` exclusion list add is the runtime-fault registration the
checker requires for every new `E112xx` code.

**R13.3 Tests.** Behavior tests live in
`compiler/tests/v11_language_surfaces_spec_test.rs`; emission and ABI pins
live in `compiler/tests/v11_language_surfaces_impl_test.rs`. A test-only C
fixture provides the host and m68k synchronous hook. Every new drop path and
the trampoline environment run under AddressSanitizer.

**R13.4 Gates.** The full acceptance is `make check`, `make asan`,
`make ubsan`, `make corpus-amiga`, `make differential`, `make bench-shape`,
and `make verify`, ending with `verify: all legs passed`. The focused suites
must be green before any commit.

**R13.5 Documentation.** `docs/known-limitations.md` records the v1.1 limits:
no `Box<T>?` allocation failure value (the allocator traps), no boxed-value
move-out (a raw cast of a Box outside `std.mem` is rejected), no direct Box FFI
crossing, no freestanding Box, no heap closure environment, the synchronous-only
callback contract, capturing hooks only in non-generic bodies, and the
ABI-identical callback signature restriction (no view, borrow, or fixed-array
callback parameters or returns).

**R13.6 Roadmap.** One roadmap row, `PLAN-19`, is added at RD Drafted and kept
in sync through Plan Preflighted, Executing, and Done.

## Non-Functional Requirements

| Area | Requirement |
|---|---|
| Determinism | The same source, profile, and flags emit byte-identical C, including trampoline, helper, and wrapper names |
| C99 validity | Every emitted shape builds under host GCC, `m68k-amigaos-gcc`, vbcc, and the freestanding profile where the feature is available |
| Memory safety | Every new drop path and the trampoline environment are ASan-clean; no leak, double free, or use-after-free |
| No UB | The new arithmetic and pointer paths are UBSan-clean |
| Determinism of test names | Test names and fixture names are stable and do not depend on wall-clock time or hash order |
| No new dependency | No new crate, build script, npm package, or external tool |

## Out of Scope

- Pools, arenas, `stores`/`transfers`, `comptime` bodies, nested templates,
  varargs wrappers.
- Generic user handles, generic `freedBy` declarations, and a general
  generic-aggregate FFI ABI.
- A heap closure environment, thread-local environments, and asynchronous or
  stored callbacks.
- DX, installer, LSP, ARM64, LorraineOS, and performance tuning.
- Changes to unrelated ownership, borrow, evaluation-order, or emission
  rules.

## Traceability

| Requirement | Surface | Primary evidence |
|---|---|---|
| R1 | Box type/API | spec: Box cases; impl: helper/type pins |
| R2 | Box allocation/profiles | spec: host/Amiga run, freestanding E11200; impl: shim call pins |
| R3 | Box ownership/moves/nullability | spec: move/drop/null cases; ASan |
| R4 | Box recursive drop | spec: nested/struct/array/list cases; ASan |
| R5 | Box access/views/generics | spec: boxWith, E10227, E10306; impl: key pins |
| R6 | Box FFI/diagnostics | spec: E10105; impl: no invalid prototype |
| R7 | X1 acceptance | spec: ordinary/exported deleter cases |
| R8 | X1 rejection | spec: E10102 table cases |
| R9 | X1 emission/ownership | impl: symbol pins; ASan owned-field case |
| R10 | L5 contract/acceptance | spec: hook accept/reject cases; impl: plain-pointer pin |
| R11 | Trampoline ABI/environment | spec: host/m68k hook, nesting, two callbacks; impl: emitted shapes |
| R12 | Hook errors/runtime fault | spec: E10400/E10404/E10210/E11215 cases |
| R13 | Spec/tests/gates/docs | spec/impl files, gate logs, roadmap row |
