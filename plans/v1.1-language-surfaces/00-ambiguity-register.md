# Ambiguity Register: v1.1 Language Surfaces

> **Document**: 00-ambiguity-register.md
> **Parent**: [Index](00-index.md)
> **Mode**: auto-design active — eligible technical decisions delegated under
> policy version 1; the three surface amendments are ruled by the goal file
> **Root invocation ID**: `codeops-v11-surfaces-20261108`
> **Policy version**: 1

## Register

| ID | Category | Question | Decision | Status / evidence |
|---|---|---|---|---|
| AR-1 | Scope | What does this plan deliver? | `Box<T>` with generated drop glue, ordinary Clorraine `freedBy` deleters, and C-hook trampolines for capturing lambdas. No v2, DX, ARM64, or LorraineOS work. | ✅ Resolved — user goal lines 46-77; R1-R13 |
| AR-2 | Authority | Which decisions need a user ruling? | The three surface amendments and their diagnostics. The goal file rules them; every implementation mechanism is delegated under auto-design. | ✅ Resolved — user goal lines 26-33, 57-77; AR-3 |
| AR-3 | Auto-design | Is auto-design active for requirements and plan? | Yes. The goal names `--auto-design` for execution; the same delegated authority is applied to the requirements and plan chain so the single-session workflow can close. Eligible technical mechanisms are recorded here; reserved product surface stays exactly the goal's three surfaces. | ✅ Resolved — user goal line 17; policy version 1 |
| AR-4 | Dependencies | May a crate, build script, or npm package be added? | No. Python is not needed. The compiler stays std-only Rust. | ✅ Resolved — user goal lines 85-86; R13 non-functional |
| AR-5 | Frozen inputs | May a `spec/tests` golden change? | No existing golden changes. The approved diagnostic amendment adds three new tests: `spec/tests/compile-fail/E10105_*.clo`, `E10227_*.clo`, and `spec/tests/run/E11215_*.clo`, and adds `E11215` to the runtime-fault exclusion list in `spec/check.sh`. `spec/check.sh` requires a compile-fail case for every live registry code and excludes only registered runtime faults; without the additions `make check` fails. | ✅ Resolved — user goal lines 74-76 + AI mechanism; R13.1-R13.2 |
| AR-6 | Box representation | Which type model does `Box<T>` use? | A new `TyKind::Box(Box<Ty>)`. A generic struct with an accessible storage field would let safe code move a `BoxStorage` between boxes of different `T` and type-confuse the drop; a synthetic handle loses the element type for display and helper generation. The dedicated kind is the smallest sound model. | ✅ Resolved — AI delegated; `types/mod.rs`, `types/keys.rs`, `types/layout.rs`; R1.1-R1.2 |
| AR-7 | Box C form | What is the C representation? | `void *`; size is the target pointer size; zero is `NULL`. | ✅ Resolved — AI delegated; R1.2 |
| AR-8 | Box allocation | Where does the storage come from? | The existing `cl_alloc`/`cl_free` pair in the host and hosted-amiga shims, wrapped as `cl_box_alloc`/`cl_box_free`. `cl_box_alloc` zeroes the block. | ✅ Resolved — AI delegated; `shim_host.c:68-79`, `shim_hosted_amiga.c:689-700`; R2.1 |
| AR-9 | Allocation failure | What happens when the allocator fails? | The existing runtime contract: `cl_trap("E1", "out of memory")` and abort. `boxNew` is non-nullable. | ✅ Resolved — AI delegated; R2.1; matches `Buffer` |
| AR-10 | Box nullability | Is `Box<T>?` supported? | Yes. A box is pointer-like, so `?` adds `null`; the ordinary narrowing rules apply. `Box<T>` itself is non-nullable. | ✅ Resolved — AI delegated; R3.3 |
| AR-11 | Box API | Which operations are public? | `boxNew`, `boxSet`, `boxWith`. `boxPtr` is a private raw helper; `boxAlloc` is a reserved intrinsic. | ✅ Resolved — AI delegated; R1.3 |
| AR-12 | Box access | How does a caller read a boxed value? | `boxWith<T, F: func(T): void>` lends the pointee read-only for the callback. The callback returns `void`, so a derived view cannot escape. | ✅ Resolved — AI delegated; R5.1 |
| AR-13 | Box move-out | Can a value be moved out of a box? | Not in v1.1. `boxSet` replaces it and the old value drops. The limitation is recorded. | ✅ Resolved — AI delegated; R3.4, R13.5 |
| AR-14 | Box self-reference | May a struct hold `Box<Self>?`? | Yes; Box is pointer-sized and breaks the by-value recursion. The spec's recursive-struct section is amended. | ✅ Resolved — user goal lines 48-56 (recursive drop) + AI mechanism; R3.5 |
| AR-15 | Box views | May a box hold a view? | No. `Box<str>`, `Box<cstring>`, `Box<u8[]>`, and a fixed array of views are `E10227`. | ✅ Resolved — AI delegated; R5.2 |
| AR-16 | Box overlays | May a box hold an overlay type? | No; `E10207`, because an overlay describes memory the program does not own. | ✅ Resolved — AI delegated; R1.4 |
| AR-17 | Box ephemeral | May a box hold a closure? | No; `E10306`, because a box is a storable aggregate and a closure is ephemeral. | ✅ Resolved — AI delegated; R1.4 |
| AR-18 | Box FFI | Does a box cross an `ffi` boundary? | No. A `Box<T>` in an `ffi` signature is `E10105`. Interop goes through `boxWith`. | ✅ Resolved — AI delegated; R6 |
| AR-19 | Generic FFI | Is the generic-aggregate FFI prototype gap fixed generally? | The new `E10105` check rejects every generic struct instance in an `ffi` signature, not only `Box<T>`, because the current emitter writes `ClWrap` for `ClWrap__i32`. This is the smallest change that keeps accepted source from emitting invalid C. | ✅ Resolved — AI delegated; `02-current-state.md` §5; R6.1 |
| AR-20 | Box name | What if a user declares a type named `Box`? | `Box` is a reserved built-in type name: a user declaration of a struct, enum, handle, or function named `Box` is `E20006`. The resolver predeclares `Box` as a type-only name (not a value), and the reserved check is a separate list from `RESERVED_BUILTINS` so intrinsic-call parsing is unaffected. Allowing shadowing would retype `std.mem`'s own signatures program-wide. | ✅ Resolved — AI delegated after the requirements preflight; `resolve/mod.rs:431-470`, `parse_declared_name`; R1.1 |
| AR-21 | Box availability | Must `Box` be imported? | The type spelling is built-in; `import { Box } from std.mem;` also works because the table lists `Box` as a compiler-known export. The operations are imported from `std.mem`. A freestanding program that writes `Box<T>` reports `E11200` through a new `mentions_box` type-side check. | ✅ Resolved — AI delegated; R2.2, R1.3 |
| AR-22 | Box drop helper | How is the recursive glue emitted? | One `static inline` helper per used element type: drop the pointee through the existing recursive walk, then `cl_box_free`, then null the caller's box. The expression form is `(b ? (helper(b), b = NULL) : 0)`. | ✅ Resolved — AI delegated; R4.2 |
| AR-23 | Box helper collection | How does the emitter know which helpers to emit? | The existing type-collection walk also records every `TyKind::Box` element type. Helpers are `static inline`, so an unused one does not trip `-Wunused-function`. | ✅ Resolved — AI delegated; `codegen/program.rs` type walk; R4.2 |
| AR-24 | Box intrinsic | How is `boxAlloc<T>()` lowered? | It is a reserved intrinsic declared in `std.mem`, added to `COMPILER_INTRINSICS` and the free/qualified sema branches; its call lowers to the generated `cl_box_alloc__<key>()` helper with an explicit emitter arm (the current fallback silently emits `0`). It is a low-level allocator, not in the import table; a qualified call creates a zeroed box. | ✅ Resolved — AI delegated after the requirements preflight; `stdlib.rs:215`, `sema/call.rs`, `own/lower.rs`, `codegen/calls.rs`; R1.3 |
| AR-24a | Box construction | Can a Box be fabricated? | No. `Box<T> {}` is `E10203`; `Box<T>(x)` is `E10201`; a `raw T(b)` cast of a Box outside `std.mem` is `E10201`. `boxPtr` is the only sanctioned raw access, and it is module-private. | ✅ Resolved — AI delegated after the requirements preflight; `sema/expr.rs:1150-1181`; R1.5, R3.4 |
| AR-24b | Box reassignment | Does `b = boxNew(...)` release the old box? | Yes. `emits_drop` gains a `TyKind::Box` arm; the assignment path releases the old target before storing. An ASan case covers the reassignment. | ✅ Resolved — AI delegated after the requirements preflight; `codegen/drop.rs:237-262`; R3.2 |
| AR-24c | Box helper order | How is mutual recursion emitted? | Every Box drop helper has a `static inline` prototype before its definition; helpers are collected transitively through struct fields, arrays, Array elements, and nested boxes after substitution. An `A`/`B` mutual-recursion case is added. | ✅ Resolved — AI delegated after the requirements preflight; R4.2 |
| AR-24d | Box layout | Does a struct containing a Box fold `sizeof`/`offsetof`? | Yes. `AbiLayout::size_at`/`align_at` and `TypeTable::size_of_at` gain Box arms; tests cover `sizeof(StructWithBox)`, `offsetof`, and an array extent. `sizeof(Box<i32>)` alone does not test the table. | ✅ Resolved — AI delegated after the requirements preflight; `types/layout.rs`, `types/table.rs`; R5.4 |
| AR-25 | Box pointer | How does the library reach the pointee? | `boxPtr<T>(borrow b: Box<T>): raw T` is ordinary Clorraine: `unsafe { return raw T(b); }`. The raw cast is the existing `T6a`-style pointer read, not a new rule. | ✅ Resolved — AI delegated; R5.1 |
| AR-26 | Box replacement | Does a raw store drop the old pointee? | Yes. The existing raw-store rule releases the replaced owning pointee exactly once (pinned by `conditional_projection_residue_spec_test::should_release_the_replaced_owning_pointee_of_a_raw_store`). `boxSet` relies on it. | ✅ Resolved — AI delegated; R1.3, R4.3 |
| AR-27 | Box initial state | What does `boxAlloc` put in the block? | Zeroed bytes. The language's zero state is the empty state (O7), so the first store's drop is a no-op for every owning `T`. | ✅ Resolved — AI delegated; R2.1 |
| AR-28 | Box constants | Is `Box<T>` a constant value? | No. `sizeof(Box<T>)` folds to pointer size; a box value cannot appear in an enum member, extent, or `case`. | ✅ Resolved — AI delegated; R5.4 |
| AR-29 | X1 resolution | Which declaration kinds are deleters? | An ordinary module function, an exported function, or an `ffi` function. An `intrinsic` or an unresolved name is rejected. | ✅ Resolved — user goal lines 57-63 + AI mechanism; R7.1-R7.2 |
| AR-30 | X1 signature | What exact signature is required? | One `take` parameter of exactly `T`, return `void`, non-nullable. Generic functions are not accepted in v1.1; generic-instance coverage comes from `Box<T>`. | ✅ Resolved — user goal lines 57-63; R7.1, R9.4 |
| AR-31 | X1 qualified names | Does a qualified deleter keep its module? | Yes. `HandleInfo` stores `(module, simple name, kind)` resolved once at collect time; the validator and emitter both use it. A dotted name is never passed to `function_symbol` (which would produce `cl_A_B_A_B_freeX`) or emitted as a C symbol. | ✅ Resolved — AI delegated after the requirements preflight; R7.1-R7.4 |
| AR-32 | X1 visibility | May a private function be a deleter? | Yes, in its own module; the ordinary scope and import rules decide whether the name resolves. An exported cross-module function is also valid. | ✅ Resolved — AI delegated; R7.2 |
| AR-33 | X1 duplicate | How is a second deleter rejected? | The existing duplicate `handle` declaration rule (`E11101`), extended to cross-module declarations by recording handle origins. The current per-module check and last-wins `ProgramInfo` map are insufficient and are fixed. | ✅ Resolved — AI delegated after the requirements preflight; R7.3 |
| AR-34 | X1 diagnostic | Does `E10102` get a new code? | No. The existing code's trigger and help text are widened to cover both function kinds. | ✅ Resolved — AI delegated; R8 |
| AR-35 | X1 body | May a deleter drop owned fields? | Yes. The body is checked by the ordinary ownership rules; the handle drop calls it once. A `String`-field case is ASan-tested. | ✅ Resolved — user goal line 60 + AI mechanism; R7.5 |
| AR-36 | X1 move-only | Does X1 change duplicability? | No. A handle remains move-only. | ✅ Resolved — user goal line 63; R7.6 |
| AR-37 | L5 marker | How is a synchronous C callback written? | A `borrow`-mode `func(...)` parameter on an `ffi` declaration. It is a call-scoped loan; the C prototype is unchanged. Unmarked/`const`/`take` func parameters remain storing or unknown and accept capture-less lambdas only. The callback signature must be ABI-identical to the lifted lambda; views, `borrow` move-only structs, fixed arrays, and fixed-array returns are `E10210`. | ✅ Resolved — user goal lines 64-71 + spec L5 "contract borrows"; R10.1-R10.2 |
| AR-37a | L5 order | Is the closure evaluated at its own argument position? | Yes. The closure is materialized into a statement temporary in the argument loop at its source position, before later arguments' effects; the extra env argument reads that temporary. Appending the closure expression at the end would let a later sibling change the captured value (reproduced: `hook(bump(n), [n] ...)` read `n=1` instead of 11). | ✅ Resolved — AI delegated after the requirements preflight; R11.6 |
| AR-38 | L5 base defect | What does the starting compiler do with a capturing lambda at an `ffi` callback? | It accepts it and emits a `ClClosure_N` value where a function pointer is expected; the strict C build fails. This is the red evidence for R10.2/R10.3. | ✅ Resolved — `02-current-state.md` §3 |
| AR-39 | L5 argument check | Where is the missing conversion check added? | `check_ffi_arguments` gains a function-type branch: match the signature, reject a non-ABI-identical or capturing closure unless the parameter is `borrow`, and record the hook site. The call-node address is threaded from `check_expr`; both the simple and qualified `ffi` paths pass `(Ty, Mode)`. | ✅ Resolved — AI delegated after the requirements preflight; `sema/expr.rs:57-70`, `sema/call.rs:100-119,218-231,1546-1603`; R10 |
| AR-40 | L5 record | How does lowering know a call is a hook? | The semantic stage records the call expression address and the callback argument's closure site in a set consumed by the lowerer, matching the existing `type_casts` pattern. | ✅ Resolved — AI delegated; R10-R11 |
| AR-41 | L5 trampoline | What is the trampoline ABI? | One `static` C function per hook argument with exactly the declared `func(...)` signature; it finds its environment by a unique id and calls the lifted lambda with the environment first. | ✅ Resolved — AI delegated; R11.2 |
| AR-42 | L5 environment | Where does the environment live? | The existing per-lambda environment struct, materialized in the caller's frame. It owns no droppable resource (copies are duplicable; borrows are pointers), so no heap environment and no env drop glue. | ✅ Resolved — AI delegated; R11.3 |
| AR-43 | L5 lookup | How do nested/reentrant hook calls find the right environment? | A compiler-emitted linked stack of `{ id, env, prev }` nodes; each trampoline searches by id. A single global would fail nesting; a per-call global would fail two callbacks. | ✅ Resolved — AI delegated; R11.5 |
| AR-44 | L5 wrapper | How are push/pop sequenced? | A generated `static` wrapper per hook call site pushes the node(s), calls the `ffi` function with the trampoline pointer(s), and pops in reverse. This keeps the language IR a normal call and preserves argument order and `take` moves. | ✅ Resolved — AI delegated; R11.6 |
| AR-45 | L5 multiple callbacks | Can one C call take two capturing callbacks? | Yes. Each argument gets its own id, trampoline, node, and wrapper parameter. | ✅ Resolved — AI delegated; R11.7 |
| AR-46 | L5 capture-less | Does a capture-less lambda use a trampoline? | No. It stays a plain lifted function pointer; no env, node, push/pop, or wrapper. | ✅ Resolved — user goal lines 71; R10.2, R11.1 |
| AR-47 | L5 after-return | What if C calls the callback after returning? | The trampoline finds no node and reports the new runtime fault `E11215`; it does not dereference a dangling pointer. | ✅ Resolved — AI delegated; R11.4, R12 |
| AR-48 | L5 stored APIs | Do `newHook`, `createTask`, and `addHandler` accept capturing lambdas? | No. Their `func` parameters are unmarked because the callee stores them; a capturing lambda is `E10400`. | ✅ Resolved — AI delegated; `stdlib/std.amiga.utility.clo:32`, `exec.clo:96`, `interrupt.clo:31-32`; R10.3 |
| AR-49 | L5 guards | Does a hook call inside a short-circuit or conditional work? | Yes. The call is effectful and materializes through the existing guarded-operand lowering; the wrapper is called only on the evaluated path. A spec case covers `ok && hookCall(...)`. | ✅ Resolved — AI delegated; R11.6 |
| AR-50 | L5 initializers | Can a module initializer use a capturing hook? | No; a capturing lambda needs a local capture, and a module-level capture is already `E10406`. A capture-less hook is a plain pointer. | ✅ Resolved — AI delegated; R10 |
| AR-51 | L5 generics | Does a hook call inside a generic body specialize? | A capturing hook call inside a generic function is `E10400`; move it to a non-generic wrapper. A capture-less hook call stays a plain pointer and is accepted. This bounds v1.1 to one environment/trampoline per non-generic site. | ✅ Resolved — AI delegated; R11.5, R13.5 |
| AR-52 | L5 ABI | Does the C callback ABI change? | No. The C side still receives a plain function pointer with the declared signature. | ✅ Resolved — user goal lines 64-71; R10.1 |
| AR-53 | L5 m68k gate | How is the Amiga/m68k hook proven? | A test-only C fixture linked with the emitted C for `hosted-amiga`; built with `m68k-amigaos-gcc` and run under `vamos` when the toolchain is present. The host fixture proves behavior; the m68k run proves the ABI. | ✅ Resolved — user goal line 69; R13.3 |
| AR-54 | L5 interrupts | Is the environment stack interrupt-safe? | The language is single-threaded; an interrupt handler is allocation-free/drop-free and capture-less by the existing handler convention. A capturing lambda is not accepted at an interrupt registration API (unmarked `func`). | ✅ Resolved — AI delegated; R11.5 |
| AR-55 | Diagnostics | Which new codes are allocated? | `E10105` generic aggregate/Box at the FFI boundary; `E10227` view-typed box element; `E11215` callback invoked after its hook returned. Codes append; no existing code is retired. | ✅ Resolved — AI delegated; R6, R11.4, R12 |
| AR-56 | Test naming | Which suites hold the cases? | `v11_language_surfaces_spec_test.rs` and `v11_language_surfaces_impl_test.rs`, one pair for the feature. Focused cases may be added to existing suites only where they pin an existing behavior the change touches. | ✅ Resolved — user goal line 72-73; R13.3 |
| AR-57 | Sanitizers | How are the new drop and environment paths proven? | The spec suite's memory helper runs under `RDC_ASAN` with leak detection; `make asan` covers the suite. The trampoline env is exercised by the host and m68k hook cases. | ✅ Resolved — user goal lines 100-101; R13.3 |
| AR-58 | Gates | Which gates define acceptance? | `make check`, `make asan`, `make ubsan`, `make corpus-amiga`, `make differential`, `make bench-shape`, `make verify`. | ✅ Resolved — user goal lines 129-131; R13.4 |
| AR-59 | Commits | How is the work landed? | Docs commits separate from implementation; one verified task or coherent feature per implementation commit; stage explicit paths; never amend or force-push; push only when the full gate is green. | ✅ Resolved — user goal lines 104-120; R13 |
| AR-60 | Specialist check | Is a new reviewer role needed? | No. Correctness, memory-safety, emission-integrity, and semantics reviewers cover the surface. Record the negative check in `05-findings.md`. | ✅ Resolved — repository quality policy; index "Specialist Agents" |
| AR-61 | Roadmap | Which row tracks the plan? | Add `PLAN-19` at RD Drafted; progress becomes 562/565 done, 1 open, 2 parked. Keep the stage synced. | ✅ Resolved — user goal line 77; R13.6 |
| AR-62 | Hook ABI set | Which callback signatures can cross? | Only ABI-identical ones: scalar, `bool`, enum, `raw`, `cstring`, handle, non-generic by-value struct, and function pointer. Views, `borrow` move-only structs, fixed arrays, and fixed-array returns are `E10210` because the lifted lambda's C spelling differs from `abi::ffi_type`. | ✅ Resolved — AI delegated after the requirements preflight; R10.2 |
| AR-63 | Stdlib surface | How do the generic Box functions become importable? | No `export` on a generic function (E10302). Add `boxNew`, `boxSet`, `boxWith`, and `Box` to the `std.mem` export table; add `Box` to `COMPILER_KNOWN_EXPORTS`; add `boxAlloc` to `COMPILER_INTRINSICS`. | ✅ Resolved — AI delegated after the requirements preflight; `stdlib.rs:215,246-263,975`; R1.3 |
| AR-64 | Box inference | How does `boxWith(b, lambda)` infer `T`? | The checker's `match_param` and `mentions_unbound` recurse into `TyKind::Box`, so `T` binds from `b: Box<T>` and `F` binds from the capturing lambda. Without the arm the call is E10301. | ✅ Resolved — AI delegated after the requirements preflight; `sema/call.rs:1731-1752,1794-1835`; R1.3, R5.1 |
| AR-65 | Box type name | How does `Box<T>` pass the resolve stage? | The resolver predeclares `Box` as a type-only name in `resolve_type_name` (the `String` precedent), and a separate reserved-type-name check rejects a user declaration with E20006. The name is not a value: `Box<T> {}` and `Box<T>(x)` are rejected. | ✅ Resolved — AI delegated after the requirements preflight; `resolve/mod.rs:431-470`; R1.1, R1.5 |
| AR-66 | E10105 grounding | What does the FFI aggregate check need? | The `TypeTable` and each block's module, `resolve_ty_folded` for aliases, and a recursion through arrays, raw pointers, and function-pointer types. `ffi::check_items` currently has none of these. | ✅ Resolved — AI delegated after the requirements preflight; `ffi.rs:21,106-108`, `own.rs:286-287`; R6.1 |
| AR-67 | X1 resolution | How are simple names and aliases resolved? | In the handle's declaring module: own functions, then imports, then a unique program-wide declaration. The parameter type is compared as a resolved `Ty`, not the last AST segment. | ✅ Resolved — AI delegated after the requirements preflight; R7.1-R7.2 |
| AR-68 | Box layout | Does `sizeof(Box<T>)` alone prove the layout? | No. `sizeof(Box<i32>)` is spelled by the emitter as `sizeof(void *)` without the table. Add Box arms to `AbiLayout` and `size_of_at`, and test `sizeof(StructWithBox)`, `offsetof`, and an extent. | ✅ Resolved — AI delegated after the requirements preflight; `types/layout.rs:148,212`, `types/table.rs:559`; R5.4 |

## Delegated Decision Records

### AR-6 — `Box<T>` type model

```text
Authority: AI — delegated by --auto-design
Eligibility: internal compiler type model; no product surface beyond the goal's Box<T>
Objective: a box that cannot be type-confused or leak through safe code
Decision: a dedicated TyKind::Box(Box<Ty>) with an opaque pointer C form
Evidence: a generic struct would expose its storage field (the language has no field privacy), and a user could move a BoxStorage between Box<U> and Box<T>; a synthetic handle name cannot recover T for display or helper generation (compiler/src/types/keys.rs:39-49, compiler/src/types/mod.rs:391-430)
Rejected alternatives:
- generic struct with a private-by-convention storage field: unsafe under field access;
- synthetic handle name: loses the element type and forces a side registry;
- raw-pointer field: duplicable and not droppable.
Strongest counterargument: a new TyKind touches every exhaustive match, so the change is broad; the breadth is compiler-checked and the plan allocates one plumbing task with a blast-radius checklist.
Confidence: High
Hardening: Rust exhaustive matches, focused type tests, and a review that walks every TyKind match.
Policy version: 1
Root invocation ID: codeops-v11-surfaces-20261108
Reopen triggers: a field-privacy rule lands, or the TyKind plumbing proves disproportionate in the plan preflight.
```

### AR-22 — Box drop glue

```text
Authority: AI — delegated by --auto-design
Eligibility: compiler emission mechanism
Objective: recursive, null-safe, exactly-once release with no new IR
Decision: one static inline helper per element type; the helper drops the pointee through the existing walk, frees the block, then the caller nulls the box
Evidence: the existing drop emitter already recurses through handles, structs, fixed arrays, and Array<T> (compiler/src/codegen/drop.rs:21-131); cl_alloc/cl_free exist (shim_host.c:68-79); a raw store already releases a replaced owning pointee (conditional_projection_residue_spec_test.rs:9917-9935)
Rejected alternatives:
- a new IR drop node: duplicates the existing walk;
- a heap-allocated drop function pointer: adds a heap environment and an ABI surface;
- inline glue at every site: code bloat and repeated recursive emission.
Strongest counterargument: a helper per element type adds C symbols; static inline keeps them local and warning-free.
Confidence: High
Hardening: ASan cases for scalar, string, struct, nested box, container, and recursive list; impl pins for the helper shape.
Policy version: 1
Root invocation ID: codeops-v11-surfaces-20261108
Reopen triggers: a future drop rule needs per-site specialization the helper cannot express.
```

### AR-37 — Synchronous callback contract

```text
Authority: AI — delegated by --auto-design for the mechanism; the L5 surface is user-ruled
Eligibility: FFI contract mechanism inside the approved L5 surface
Objective: let a capturing lambda cross exactly the synchronous C hooks without a heap environment or an ABI change
Decision: a borrow-mode func(...) parameter is the synchronous contract; the compiler emits a per-argument trampoline plus a caller-stack environment found through a linked stack
Evidence: the existing two-word closure already carries an environment (compiler/src/codegen/program.rs:697-780); the existing parameter modes already express call-scoped loans (spec/06-functions.md:39-45); the current unmarked path accepts the invalid C (02-current-state.md §3); the storing APIs use unmarked func parameters (stdlib/std.amiga.utility.clo:32, exec.clo:96, interrupt.clo:31-32)
Rejected alternatives:
- a new FFI clause keyword: X3 removed the draft clause words and the mode already carries the semantics;
- a hidden global environment: breaks nesting and two callbacks;
- a heap environment: the goal forbids it;
- changing the C callback ABI to carry an env: breaks every existing C hook.
Strongest counterargument: a C library can still store a borrow-marked callback; that is a trust violation, detected at run time by E11215 and documented in the contract.
Confidence: High
Hardening: host and m68k fixtures, nested/reentrant/two-callback cases, after-return E11215, and an impl pin that a capture-less lambda stays a plain pointer.
Policy version: 1
Root invocation ID: codeops-v11-surfaces-20261108
Reopen triggers: a real C hook needs an environment parameter the ABI cannot supply, or a stores contract is approved in v2.
```

### AR-55 — New diagnostics

```text
Authority: AI — delegated by --auto-design
Eligibility: diagnostic allocation inside the approved surfaces
Objective: a precise first line for each new rejection and runtime fault
Decision: E10105 (generic aggregate/Box at the FFI boundary), E10227 (view-typed box element), E11215 (callback after its hook returned)
Evidence: E101xx is the FFI block, E102xx the type block, and E112xx the runtime-fault block (spec/14-diagnostics.md:40-42, 176-180); the next free numbers append after E10104, E10226, and E11214
Rejected alternatives: reusing E10200/E11200 would give a misleading first line; E90000 would wrongly mark Box as staged.
Strongest counterargument: every new code is a permanent surface; the three are unavoidable because the existing messages do not describe the triggers.
Confidence: High
Hardening: registry entries with trigger, first line, help, and a compile-fail/run case per code.
Policy version: 1
Root invocation ID: codeops-v11-surfaces-20261108
Reopen triggers: the plan preflight finds an existing code with an exact trigger/message match.
```

## Reopen Conditions

A decision reopens when:

- the plan preflight finds a contradiction with a frozen chapter or an existing
  test;
- a `Box<T>` case cannot be made sound without field privacy or a new rule;
- the hook contract cannot be expressed without changing the C ABI;
- a gate exposes a regression outside the three surfaces.
