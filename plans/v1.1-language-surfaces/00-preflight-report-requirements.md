# Requirements Preflight: v1.1 Language Surfaces

> **Status**: ✅ PREFLIGHT PASSED WITH NOTES — 23 findings (7 critical, 14
> major, 2 minor); all critical/major resolved in the revised requirements and
> design documents; 2 minor accepted as documented limits
> **Iteration**: 1 (first scan; the requirements and designs were revised in
> the same pass)
> **Artifact**: requirements set at `plans/v1.1-language-surfaces/`
> (`00-index.md`, `00-ambiguity-register.md`, `01-requirements.md`,
> `02-current-state.md`, `03-01-box.md`, `03-02-freedby.md`,
> `03-03-hook-trampolines.md`, `07-testing-strategy.md`)
> **Codebase Grounded**: 30+ source files examined; every finding cites a
> `compiler/src` or `stdlib/` location
> **Last Updated**: 2026-11-08
> **Same-session review**: ⚠️ the artifact was authored in this session. An
> independent challenger (general subagent, `ses_e4721693affe2U2tCCZwI4yV9g`)
> reviewed the critical/major batch against the code; its findings are
> incorporated below. A fresh-session review is recommended before execution
> if the schedule permits.

## Codebase Context Summary

**Tech Stack:** Rust 2021 workspace (std only, no external crates), C99 runtime
shims, embedded Clorraine stdlib, Python-stdlib fuzz tooling, Docker SDK with
`m68k-amigaos-gcc`, vbcc, and `vamos`.

**Architecture:** `clc` loads user and embedded stdlib sources, runs
resolve → type collection → sema → ownership/borrow lowering → single-TU C
emission. The ownership stage owns the IR, monomorphization, and drop/cleanup
decisions; the emitter owns C spelling and symbol naming.

**Key Files Examined:**
`compiler/src/resolve/mod.rs`, `types/{mod,table,keys,layout}.rs`,
`borrow.rs`, `sema/{mod,expr,call,items}.rs`, `own/{program,lower,call,expr,place,generics,instance_check}.rs`,
`codegen/{types,drop,program,function,calls,expr}.rs`, `ffi.rs`, `abi.rs`,
`profile.rs`, `stdlib.rs`, `token.rs`, `load/{mod,scan}.rs`,
`compiler/runtime/shim_host.c`, `shim_hosted_amiga.c`,
`stdlib/std.mem.clo`, `stdlib/std.text.clo`, `spec/check.sh`,
`compiler/tests/*` patterns.

**Reference Verification:** 41 references mapped; 39 verified against code;
2 corrected (the resolve-stage `Box` predeclaration and the `spec/check.sh`
diagnostic coverage), both resolved below.

## Summary by Dimension

| # | Dimension | Findings | Highest Severity |
|---|---|---|---|
| 1 | Ambiguities | 2 | 🟡 minor |
| 2 | Implicit Assumptions | 5 | 🔴 critical |
| 3 | Logical Contradictions | 2 | 🟠 major |
| 4 | Completeness Gaps | 5 | 🔴 critical |
| 5 | Dependency Issues | 1 | 🟠 major |
| 6 | Feasibility Concerns | 4 | 🔴 critical |
| 7 | Testability | 2 | 🟠 major |
| 8 | Security Blind Spots | 1 | 🟠 major |
| 9 | Edge Cases | 2 | 🟡 minor |
| 10 | Scope Creep Indicators | 0 | — |
| 11 | Ordering & Sequencing | 0 | — |
| 12 | Consistency | 1 | 🟡 minor |
| 13 | Codebase Alignment | 4 | 🔴 critical |

## Summary by Severity

| Severity | Count | Status |
|---|---|---|
| CRITICAL | 7 | all resolved |
| MAJOR | 14 | all resolved |
| MINOR | 2 | accepted as documented limits |
| OBSERVATION | 0 | — |

---

## Critical Findings

### PF-001: `Box<T>` does not resolve in the name pass 🔴 CRITICAL

**Dimension:** 13 — Codebase Alignment / Phantom Reference
**Location:** R1.1; `03-01-box.md` type model.
**Codebase Evidence:** `compiler/src/resolve/mod.rs:431-470` predeclares only
`String`; `resolve_type_name`/`resolve_dotted_name` call `resolve_name`, and
the resolve stage runs before the type table. `std.mem`'s own `Box<T>`
signatures would report `E10219`.
**The Problem:** The requirement "spellable without a declaration" is false;
the standard library the plan proposes cannot compile.
**Resolution:** R1.1 now requires a type-only `Box` predeclaration in
`resolve_type_name` (not the value path), with a bare-`Box` compile test.
**User Decision:** Resolved — AI delegated (technical resolver mechanism).

### PF-002: New diagnostics break `spec/check.sh` 🔴 CRITICAL

**Dimension:** 4 — Completeness / 11 — Ordering
**Location:** R13.1-R13.3.
**Codebase Evidence:** `spec/check.sh:90-186` requires a compile-fail case for
every live registry code; runtime faults are excluded only through
`excluded_codes` (`E11210-E11214`). `make check` runs the spec check first.
**The Problem:** `E10105`, `E10227`, and `E11215` would fail `make check`, and
R13.2 forbade the required files.
**Resolution:** R13.1 now authorizes the two new compile-fail spec tests and
the `E11215` runtime-fault exclusion; R13.2 is reworded to "no existing golden
changes". The goal's "spec only through explicit rulings" is satisfied by the
approved diagnostic amendment.
**User Decision:** Resolved — AI delegated mechanism under the approved
diagnostic amendment.

### PF-003: The hook wrapper does not preserve left-to-right argument order 🔴 CRITICAL

**Dimension:** 3 — Contradiction / 9 — Edge Case
**Location:** R11.6; `03-03-hook-trampolines.md` lowering.
**Codebase Evidence:** `own/lower.rs:6697-6708` keeps a closure argument
inline; the challenger's reproduction `hook(bump(n), [n] (x) => x + n)` read
`n=1`, not 11 (exit 246). C argument order is unspecified, so appending the
environment at the end is not enough.
**The Problem:** The frozen evaluation order is violated for any call with an
effectful sibling argument.
**Resolution:** R11.6 now requires the closure to be materialized into a
statement temporary at its own source position inside the argument loop; the
extra env argument reads that temporary. A spec case pins it.
**User Decision:** Resolved — AI delegated (lowering mechanism).

### PF-004: The trampoline ABI is incompatible with accepted callback signatures 🔴 CRITICAL

**Dimension:** 2 — Implicit Assumptions / 6 — Feasibility
**Location:** R10.2; R11.2; `03-03-hook-trampolines.md` §4.2.
**Codebase Evidence:** `abi.rs:44-65` maps views to `APTR` and ignores modes;
`codegen/types.rs:100-103,217-230` spells a `borrow` move-only struct as
`T *` and a view as `ClStr`; the lifted lambda uses the language declarator.
A view or borrow callback emits incompatible pointer types; a fixed-array
return uses a hidden out-parameter.
**The Problem:** "Exactly the declared signature" is false for those types.
**Resolution:** R10.2 now defines the ABI-identical accepted set (scalar,
`bool`, enum, `raw`, `cstring`, handle, non-generic by-value struct, function
pointer) and rejects views, borrow move-only structs, fixed arrays, and
fixed-array returns with `E10210`. A strict C test covers the accepted set.
**User Decision:** Resolved — AI delegated (ABI restriction).

### PF-005: `boxWith(b, capturingLambda)` cannot infer `T` 🔴 CRITICAL

**Dimension:** 6 — Feasibility
**Location:** R1.3, R5.1.
**Codebase Evidence:** `sema/call.rs:1794-1835` (`match_param`) has no Box arm;
the fallback leaves `T` unbound and `check_generic_call` reports `E10301`.
Explicit `F = func(T): void` rejects the closure (`sema/expr.rs:603-614`).
**The Problem:** The headline access API has no working spelling.
**Resolution:** R1.3/R5.1 require `match_param` and `mentions_unbound` to
recurse into `Box`; `boxWith(b, lambda)` is the supported spelling. A test
covers it.
**User Decision:** Resolved — AI delegated (inference mechanism).

### PF-006: A non-nullable Box can be fabricated as null 🔴 CRITICAL

**Dimension:** 8 — Security / 13 — Codebase Alignment
**Location:** R1.5, R3.3.
**Codebase Evidence:** `sema/expr.rs:1164-1181`: a struct literal whose name is
not a struct returns `Ty::unknown()` with no diagnostic; the emitter renders it
as `0`. `Box<i32> {}` would be a non-nullable null. `Box<i32>(p)` falls through
to an undefined call.
**The Problem:** `boxWith`/`boxSet` can dereference NULL.
**Resolution:** R1.5 requires `Box<T> {}` → `E10203`, `Box<T>(x)` → `E10201`,
and a `raw T(b)` cast of a Box outside `std.mem` → `E10201`. Tests cover all
three.
**User Decision:** Resolved — AI delegated (soundness fix).

### PF-007: Cross-module duplicate handles are not rejected; validator and emitter disagree 🔴 CRITICAL

**Dimension:** 2 — Implicit Assumptions / 13 — Codebase Alignment
**Location:** R7.3.
**Codebase Evidence:** `load/mod.rs:386-407` checks duplicates per module only;
`sema/mod.rs:344-355` checks cross-module collisions for structs but not
handles; `types/table.rs:115,172-174` stores handles in a set;
`ProgramInfo::collect` inserts last-wins (`own/program.rs:124-133`) while
`codegen/drop.rs:287-298` scans first-wins. The challenger reproduced two
modules with the same handle and different deleters: no `E11101`, and the
validator checked one while the emitter called the other.
**The Problem:** "One deleter per type" is not enforceable as designed.
**Resolution:** R7.3 requires handle-origin recording and cross-module
`E11101`; `HandleInfo` stores the resolved deleter identity once and both
stages use it.
**User Decision:** Resolved — AI delegated (correctness fix).

## Major Findings

### PF-008: X1 emits the wrong C symbol for dotted/cross-module deleters 🟠 MAJOR

**Dimension:** 13 — Codebase Alignment
**Location:** R7.4, R9.1-R9.2.
**Codebase Evidence:** `deleter_symbol` passes the dotted name to
`function_symbol` (`codegen/drop.rs:273-284`), producing
`cl_A_B_A_B_freeX`; the fallback can emit an invalid dotted C symbol. A simple
name resolves in the emitting module, not the handle's module.
**Resolution:** R7.4/R9.1 require the resolved `(module, simple name, kind)`
identity stored once; no second lookup, no dotted symbol. Cross-module simple,
qualified, and ffi-dotted tests are added.
**User Decision:** Resolved — AI delegated.

### PF-009: X1 simple-name resolution is program-wide and type comparison is by spelling 🟠 MAJOR

**Dimension:** 2 — Implicit Assumptions
**Location:** R7.1-R7.2.
**Codebase Evidence:** `ProgramInfo::user_function` for a simple name uses
`callee`, whose `functions` map is keyed by simple name only
(`own/program.rs:78-79,113-114,196-208`); `type_name_matches` compares the last
AST segment and ignores aliases/arguments (`own/program.rs:280-287`).
**Resolution:** R7.1-R7.2 require module-scoped resolution (own module, then
imports, then a unique program-wide declaration) and a resolved `Ty`
comparison with `resolve_ty_folded`. Tests cover an aliased valid parameter and
a same-named foreign type.
**User Decision:** Resolved — AI delegated.

### PF-010: `emits_drop` has no Box arm, so reassignment leaks 🟠 MAJOR

**Dimension:** 4 — Completeness
**Location:** R3.2.
**Codebase Evidence:** `codegen/drop.rs:237-262` returns false for any kind
other than Handle/Struct; `codegen/function.rs:1689-1694` uses it to decide
whether to release the old target.
**Resolution:** R3.2 now requires `TyKind::Box => true` in `emits_drop` and an
ASan reassignment case.
**User Decision:** Resolved — AI delegated.

### PF-011: The blast-radius claim misses silent `_` fallbacks 🟠 MAJOR

**Dimension:** 4 — Completeness / 13 — Codebase Alignment
**Location:** `03-01-box.md` §9.
**Codebase Evidence:** `types/mod.rs:836-851` `contains_closure`;
`own/generics.rs:150-195` `is_concrete`/`record_struct_type`;
`sema/call.rs:1731-1752` `mentions_unbound`; `sema/mod.rs:1203-1222`
`mentions_heap_string`; `types/keys.rs:24-28` `RESERVED_COMPONENTS`;
`codegen/calls.rs:501` `_ => "0"`.
**Resolution:** The checklist now names each function and adds focused tests
for nested closure rejection, `Box<Wrap<i32>>` typedef, generic `Box<T>`
substitution, freestanding Box, and `boxAlloc` emission.
**User Decision:** Resolved — AI delegated.

### PF-012: A user `struct Box<T>` shadows the built-in and breaks `std.mem` 🟠 MAJOR

**Dimension:** 9 — Edge Case / 13 — Codebase Alignment
**Location:** R1.1, AR-20.
**Codebase Evidence:** `resolve_named` checks the global struct table first
(`types/table.rs:798-823`); the challenger demonstrated the same hazard with
`struct String` and `std.text` (E10036/warnings inside the stdlib). The current
design promised supported shadowing.
**Resolution:** The preflight first chose a reserved `Box` name (`E20006`).
Execution rejected that: it broke 957 existing implementation-test sources and
the spec's own `Box<T>` generic example. A user `struct Box` now shadows the
built-in like `String`; the standard-library Box API is unavailable in that
program and the limitation is documented (AR-69 runtime). The shadowing test is
added.
**User Decision:** Resolved — AI delegated.

### PF-013: `Box<Self>` uses a type the language does not have 🟠 MAJOR

**Dimension:** 3 — Contradiction
**Location:** R3.5, AR-14.
**Codebase Evidence:** no `Self` keyword in `compiler/src/token.rs`; `struct
Node { next: raw Self?; }` reports `E10219`.
**Resolution:** R3.5/AR-14 now spell `Box<Node>?` with the concrete struct
name.
**User Decision:** Resolved — AI delegated (wording).

### PF-014: The proposed stdlib API declarations fail `E10302` and are not exported 🟠 MAJOR

**Dimension:** 5 — Dependency
**Location:** R1.3.
**Codebase Evidence:** `export func id<T>` is `E10302`; stdlib generic
functions carry no `export` and are listed in `stdlib::MODULES`
(`stdlib.rs:246-263`). Without table entries, `import { boxNew }` fails.
**Resolution:** R1.3/AR-63 drop `export`, add `boxNew`/`boxSet`/`boxWith`/`Box`
to the table, and add `Box` to `COMPILER_KNOWN_EXPORTS`.
**User Decision:** Resolved — AI delegated.

### PF-015: `boxAlloc` is not wired through the intrinsic layers 🟠 MAJOR

**Dimension:** 5 — Dependency / 6 — Feasibility
**Location:** R1.3, AR-24.
**Codebase Evidence:** `types/table.rs:196,275` ignores `Item::Intrinsic`;
`stdlib.rs:215` is the intrinsic registry pinned by
`the_compiler_intrinsic_set_matches_the_library_declarations`;
`codegen/calls.rs:501` silently emits `0` for an unhandled intrinsic.
**Resolution:** `boxAlloc` joins `COMPILER_INTRINSICS`; sema, lowering, and
emission get explicit branches; an impl pin covers the helper. The
"not callable" claim is removed: a qualified call creates a zeroed box and is
safe, though not part of the documented API.
**User Decision:** Resolved — AI delegated.

### PF-016: The borrowed-Box ABI rationale is wrong and dangerous 🟠 MAJOR

**Dimension:** 2 — Implicit Assumptions
**Location:** `03-01-box.md` §6.
**Codebase Evidence:** `borrow.rs:128-130` `is_handle` has no call sites; the
C parameter spelling is `is_pointer_param` (`codegen/types.rs:100-103`), which
returns true only for nullable or non-duplicable struct borrows. A `borrow
Box<T>` must stay a `void *` value, not `void **`.
**Resolution:** The design now states `is_handle` is for ownership
classification only; `is_pointer_param` keeps returning false for Box; an
impl pin covers the `boxPtr`/`boxSet` prototypes.
**User Decision:** Resolved — AI delegated.

### PF-017: Freestanding `Box` has no `E11200` mechanism 🟠 MAJOR

**Dimension:** 4 — Completeness
**Location:** R2.2.
**Codebase Evidence:** `Profile::Freestanding` supports only `std.io`
(`profile.rs:126`); the type-side profile check is `mentions_heap_string` in
`sema/mod.rs:733-744,1203-1222`, which does not know Box; the freestanding
shim has no `cl_box_alloc`/`cl_box_free`.
**Resolution:** R2.2 requires a `mentions_box` arm next to
`mentions_heap_string`; a freestanding Box test asserts `E11200`.
**User Decision:** Resolved — AI delegated.

### PF-018: Box drop-helper order and mutual recursion are unspecified 🟠 MAJOR

**Dimension:** 9 — Edge Case
**Location:** R4.2.
**Codebase Evidence:** `static inline` helpers call each other; C99 requires a
declaration before a call. `order_struct_units` handles acyclic structs only
(`codegen/program.rs:861-912`). An `A`↔`B` boxed pair would fail the strict C
build.
**Resolution:** R4.2 now requires a `static inline` prototype for every helper
before definitions and transitive collection through struct fields/arrays;
an A↔B test is added.
**User Decision:** Resolved — AI delegated.

### PF-019: `raw T(box)` gives application code the pointee pointer 🟠 MAJOR

**Dimension:** 8 — Security
**Location:** R3.4, R5.1, R6.2.
**Codebase Evidence:** `check_raw_cast` accepts any operand
(`sema/expr.rs:1150-1157`); the Box C value is `void *`, so `raw T(b)` yields
the block pointer and `unsafe { *raw T(b) }` moves the pointee out (tested).
**Resolution:** R1.5/R3.4 now reject a raw cast of a Box operand outside
`std.mem` with `E10201`; `boxPtr` is the library's private spelling. The
remaining audited unsafe escape is documented.
**User Decision:** Resolved — AI delegated.

### PF-020: R10.2 contradicts the generic callable-bound acceptance 🟠 MAJOR

**Dimension:** 3 — Contradiction
**Location:** R5.1, R10.2, R10.3.
**Codebase Evidence:** `check_conversion` rejects Closure→Func
(`sema/expr.rs:603-614`), while `check_callable_bounds` accepts a Closure for
`F: func(...)` (`sema/call.rs:784-798`); `boxWith` depends on the latter.
**Resolution:** R10.2/R10.3 now scope the C-hook rule to concrete `func(...)`
parameters and explicitly keep the generic bound acceptance.
**User Decision:** Resolved — AI delegated (wording).

### PF-021: The `E10105` check has no table, ignores aliases, and is not recursive 🟠 MAJOR

**Dimension:** 6 — Feasibility / 13 — Codebase Alignment
**Location:** R6.1.
**Codebase Evidence:** `ffi::check_items(program, ...)` has no `TypeTable`
(`ffi.rs:21`); `signature_param_type` uses `resolve_ty`, which ignores aliases
(`ffi.rs:106-108`); `check_items` iterates blocks without their module;
`abi::ffi_type_with` recurses into `Func` and maps a generic struct to
`Cl{name}` (`abi.rs:55,78-90`).
**Resolution:** R6.1 requires `&TypeTable`, each block's module,
`resolve_ty_folded`, and recursion through Array/Raw/Func; a nested callback
compile-fail case is added.
**User Decision:** Resolved — AI delegated.

### PF-022: The hook record cannot be keyed as designed; the qualified path loses modes 🟠 MAJOR

**Dimension:** 6 — Feasibility
**Location:** `03-03-hook-trampolines.md` §2.
**Codebase Evidence:** `check_ffi_arguments` receives no call address
(`sema/call.rs:1546-1603`); the address is in `check_expr`
(`sema/expr.rs:57-70`); the qualified ffi path builds `Vec<&Type>` and calls
with `&[Ty]` (`sema/call.rs:100-119`), so the cache change alone is
insufficient.
**Resolution:** The design now threads the call-node address from `check_expr`
and converts both ffi paths to `(Ty, Mode)`; the wrapper record is looked up in
`call_arguments`.
**User Decision:** Resolved — AI delegated.

### PF-023: `sizeof`/`offsetof` of a struct containing a Box is untested 🟠 MAJOR

**Dimension:** 7 — Testability
**Location:** R5.4.
**Codebase Evidence:** `AbiLayout::size_at`/`align_at` and
`TypeTable::size_of_at` have `_ => None`; without Box arms, an enclosing
struct's size/offset is unknown. `sizeof(Box<i32>)` alone is spelled by the
emitter as `sizeof(void *)` and does not test the table.
**Resolution:** R5.4/AR-68 require Box arms in both layout paths and tests for
`sizeof(StructWithBox)`, `offsetof`, and an extent.
**User Decision:** Resolved — AI delegated.

## Minor Findings

### PF-024: `Box<static str>` ambiguity 🟡 MINOR

**Dimension:** 1 — Ambiguity
**Location:** R1.4/R5.2.
**Problem:** A naive `is_view` check would reject `Box<static str>`, although a
static literal cannot dangle and the struct-field rule allows it.
**Resolution:** R1.4/R5.2 now allow `Box<static str>` explicitly and reject
only non-static views.

### PF-025: `Box<Array<str>>` inherited a pre-existing view-storage hole 🟡 MINOR

**Dimension:** 9 — Edge Case
**Location:** R5.2.
**Codebase Evidence:** `Array<str>` is accepted by the current compiler (the
compiler does not reject a view element in `Array<T>`); a Box of it would
smuggle views.
**Resolution:** R5.2 now rejects `Box<Array<str>>` through the recursive
container check. The standalone `Array<str>` hole is pre-existing and outside
this feature; it is recorded as an accepted known limitation rather than
fixed here.
**User Decision:** Accepted risk — pre-existing, outside the Box surface; the
Box path is closed.

## Verdict

All critical and major findings are resolved in the revised requirements and
designs. The two minor findings are either incorporated (PF-024) or accepted
as a documented pre-existing limit (PF-025). The requirements set is ready for
plan creation; the plan preflight will test the execution sequencing and the
remaining implementation surface.

**Verdict:** ✅ PREFLIGHT PASSED WITH NOTES — 21 findings resolved, 1 accepted
(PF-025), 1 incorporated (PF-024).

## Applied Revisions

| Finding | Document changed | Revision |
|---|---|---|
| PF-001 | R1.1, 03-01 §1 | type-only `Box` predeclaration; bare-Box test |
| PF-002 | R13.1-R13.2, AR-5 | two new compile-fail tests + E11215 exclusion |
| PF-003 | R11.6, 03-03 §3 | closure materialized at its argument position |
| PF-004 | R10.2, 03-03 §2 | ABI-identical hook signature set |
| PF-005 | R1.3, R5.1, AR-64 | `match_param` Box recursion |
| PF-006 | R1.5, R3.3, AR-24a | `Box {}`/`Box(x)`/raw-cast rejection |
| PF-007 | R7.3, 03-02 §3 | handle origins + cross-module E11101 |
| PF-008 | R7.4, R9.1-R9.2 | stored deleter identity |
| PF-009 | R7.1-R7.2, 03-02 §2 | module-scoped resolution + resolved Ty |
| PF-010 | R3.2, AR-24b | `emits_drop` Box arm + ASan case |
| PF-011 | 03-01 §9 | silent-fallback checklist + tests |
| PF-012 | R1.1, AR-20, AR-69 | shadowing rule + documented limit |
| PF-013 | R3.5, AR-14 | concrete `Node`, no `Self` |
| PF-014 | R1.3, AR-63 | no `export`; export table |
| PF-015 | R1.3, AR-24, AR-63 | intrinsic wiring + impl pins |
| PF-016 | 03-01 §6 | `is_pointer_param` stays false |
| PF-017 | R2.2, AR-21 | `mentions_box` E11200 |
| PF-018 | R4.2, AR-24c | helper prototypes + mutual recursion |
| PF-019 | R1.5, R3.4, AR-24a | raw-cast gate |
| PF-020 | R10.2-R10.3 | generic-bound carve-out |
| PF-021 | R6.1, AR-66 | table/aliases/recursion |
| PF-022 | 03-03 §2, AR-39 | call-address threading + modes |
| PF-023 | R5.4, AR-68 | layout arms + sizeof/offsetof tests |
| PF-024 | R1.4, R5.2 | allow `Box<static str>` |
| PF-025 | R5.2 | reject `Box<Array<str>>`; pre-existing `Array<str>` documented |
