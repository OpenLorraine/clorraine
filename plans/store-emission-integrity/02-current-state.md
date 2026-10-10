# Current State: Store-Emission Integrity

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Existing Implementation

### What Exists

The four defects sit on three existing subsystems, all verified at the plan baseline
(`39f0e5a`, `target/debug/clc` built from it) with emitted-C and host AddressSanitizer probes:

* **The lend-store judgment** (`compiler/src/own/expr.rs`) — `check_lend_target_store` judges a
  whole-value store into a lend parameter or out-slot; `lend_moves_judged` suppresses that
  judgment when a source of the stored value is a lend (because the source-side path reports the
  site instead). The binding-effect marking path already exempts "an arm that names the target
  path itself" for projected stores; the suppression path does not apply the same exemption.
* **The write-mutability checks** (`compiler/src/own/place.rs`) — `write_place` reports `E10023`
  for `const` bindings and `E10024` for read-only parameter modes. No check covers storage that
  is immutable by view type (`str`, `static str`).
* **The emitter** (`compiler/src/codegen.rs`, `compiler/src/codegen/program.rs`) — emission runs
  typdefs (slice typedefs, then handle typedefs), aggregate declarations (struct forward
  typedefs, struct definitions, then enums), closure declarations, prototypes, globals, init
  functions, and bodies. Module initializers are lowered by `lower_global` and emitted as
  assignment statements in `cl_init_<module>`; compiler intrinsics are recognized only by the
  function-body path (`own/call.rs::lower_intrinsic_call`).

### Relevant Files

| File | Purpose | Changes Needed |
| ---- | ------- | -------------- |
| `compiler/src/own/expr.rs` | `check_lend_target_store`, `lend_moves_judged` | DEF-113: target-aware suppression exclusion |
| `compiler/src/own/place.rs` | `write_place` (plain/compound/update local writes) | DEF-114: the E10042 helper call |
| `compiler/src/codegen.rs` | `run()` emission order | DEF-115: reorder typedef phases |
| `compiler/src/codegen/program.rs` | `emit_slice_typedefs`, `emit_aggregate_declarations`, `lower_global`, `emit_global_init` | DEF-115 split; DEF-116 fill + intrinsic lowering |
| `spec/14-diagnostics.md` | the diagnostics registry | DEF-114: the authorized E10042 row |
| `spec/tests/compile-fail/` | compile-fail goldens | DEF-114: the E10042 golden |
| `compiler/tests/store_emission_integrity_spec_test.rs` | new specification suite | all phases |
| `compiler/tests/store_emission_integrity_impl_test.rs` | new implementation suite | all phases |

### Baseline Probes (verified)

Each probe was compiled with `target/debug/clc --profile host -o out.c prog.clo`; accepted units
were built with `cc -std=c99 -O0 -Wall -Wextra -Werror` (and `-fsanitize=address -no-pie` where
memory matters). The program shapes are the durable record; scratch files are not kept.

| Probe | Shape | Baseline result |
| ----- | ----- | --------------- |
| d113 | `func st(borrow slot: String, pick: bool) { let other = stringOf("bbbb"); slot = pick ? slot : other; }` called with `pick = false` | accepted; emitted `ClString __cl_save_1 = slot; if (pick) { slot = __cl_save_1; __cl_save_1 = NULL; } else { slot = other; other = NULL; } if (__cl_save_1) { FreeString(__cl_save_1); }`; ASan heap-use-after-free at the caller's later `len(s)` (exit 1) |
| d113e | the generic `st<T>(borrow slot: T, pick: bool, take other: T)` at `T = String` | accepted; same emission; ASan heap-use-after-free (exit 1) |
| d113d | nullable out-slot: `g(borrow slot: String?, pick: bool) { let other: String? = …; slot = pick ? slot : other; }` | accepted; the self arm nulls the save instead of freeing it; run exit 4 = the program's length, ASan clean (safe — pinned) |
| d113c | generic with a read-only lend arm (`other: T` unmarked) at `T = String` | rejected `E10034` naming `other` (the read-only arm move; unchanged) |
| p1 / p8 | direct store from a read-only lend / direct self-store | rejected `E10034` naming the source `other` / the target `slot` (pinned regressions) |
| p3 / p4 | conditional of two read-only lends / two `take` arms | `E10034` per read-only arm / one `E10034` on `slot` (pinned) |
| d114 | `func f(borrow s: str) { s[0] = 65; }` | accepted; `cc` rejects `*s.ptr = 65` (assignment of read-only location) |
| d114c / d114d | local `str` write / `static str` write | accepted; the same `cc` rejection |
| d114b | `const s: str` parameter write | already rejected `E10024` |
| d114f | `cstring` parameter write (read-only mode) | already rejected `E10024` (different type; out of scope) |
| p2a / p2b / p2c / p2f | compound `s[0] += 1` / update `s[0]++` / `static str` field write / module-level `str` write | accepted; the same `cc` rejection |
| p2d / p2e | whole-view rebind / writable slice element store | accepted; build and run clean (pinned controls) |
| d115 | `struct P` + `slice(arr, 0, 2)` + `ps[0].x` | accepted; `cc` fails: `unknown type name 'ClP'` at `typedef struct { ClP *ptr; … } ClSlice_P;` |
| d115b | the same with an enum element | accepted; `cc` fails: `unknown type name 'ClE'` at `ClSlice_E` |
| d115d | `struct S { e: E; n: i32; }` (no slice) | accepted; `cc` fails: `unknown type name 'ClE'` inside the struct definition |
| p3a | closure capturing a slice (`[s0] () => i32(s0[1]))()`) | accepted; builds clean (pinned regression control) |
| d116 | `let backing: i32[2] = [1, 2]; let view: i32[] = slice(backing, 0, 2);` | accepted; `cl_init` emits `backing = { 1, 2 };` (invalid C) and `view = slice(backing, 0, 2);` (unlowered bare name) |
| p4a / p4b / p4d | nested extents `[[1, 2], [3, 4]]` / prefix+fill `[1, 2; 7]` / place copy `= backing` | accepted; all fail `cc` at `= { … }` |
| p4e | `let v: i32[] = slice(backing, bump(), 3);` with a side-effecting offset | accepted; the array line fails first; after the fix the offset must run once |
| p4f | `let a: i32[2] = pick() ? x : y;` (module level) | accepted; emits `a = (cl_..._pick() ? cl_..._x : cl_..._y);` — an array ternary, invalid C (covered per the user ruling, AR-18) |
| p4c | `const c: i32[2] = [5, 6];` | rejected `E10224` (a rejection gap; out of scope) |
| g1 | `let g: i32 = make();` with a user function | accepted; the call is resolved and mangled (`cl_repros_g1_make()`), builds clean — only intrinsic calls are unlowered |

## Gaps Identified

### Gap 1: the conditional store evades the lend-store rule (DEF-113)

**Current Behavior:** `lend_moves_judged` counts the target's own arm as a lending source for a
conditional value, so the target-side store rule never fires (`compiler/src/own/expr.rs:785-808`).
The emitted conditional store frees the caller's saved handle on the non-slot arm.
**Required Behavior:** the store is judged like the direct store: exactly one `E10034` naming the
target, per the confirmed concrete spellings (AR-4, AR-5).
**Fix Required:** exclude conditional leaves equal to the exact target path from the suppression
test (03-01).

### Gap 2: immutable views accept element writes (DEF-114)

**Current Behavior:** `write_place` (and the global write branches) check only `const` bindings
and read-only parameter modes (`compiler/src/own/place.rs:99-121`); a `let`, `borrow`, or global
`str`/`static str` base accepts an element write that no conforming C compiler compiles.
**Required Behavior:** one `E10042` per write through an immutable view (AR-2, AR-6, AR-7).
**Fix Required:** a shared ownership-layer check over the written place's element base (03-02),
plus the authorized registry row and golden (AR-12).

### Gap 3: typedef emission violates its own dependencies (DEF-115)

**Current Behavior:** `emit_slice_typedefs` runs before every struct, enum, and handle typedef
(`compiler/src/codegen.rs:352`, `compiler/src/codegen/program.rs:135,146`), and enum typedefs are
emitted after struct definitions (`emit_aggregate_declarations` order).
**Required Behavior:** every typedef precedes its uses for slice-of-struct, slice-of-enum, and
struct-with-enum-field (AR-3, AR-8).
**Fix Required:** reorder the emission phases (03-03).

### Gap 4: module initializers cannot express arrays or intrinsics (DEF-116)

**Current Behavior:** `emit_global_init` renders `target = value;` for every type, so an
array-typed value becomes an array assignment (`compiler/src/codegen/program.rs:620-654`), and
`lower_global`'s call branch builds a plain call whose intrinsic callee has no symbol
(`compiler/src/codegen/program.rs:772-784`).
**Required Behavior:** element-wise fills for literal, place-copy, and conditional values, and
intrinsic view construction, with operand-once evaluation (AR-9, AR-10, AR-11, AR-18).
**Fix Required:** the fill walk, intrinsic detection, and init-local materialization (03-04).

## Dependencies

### Internal Dependencies

- The duty audit (`compiler/src/own/instance_check.rs`) already provides the per-instance E10034
  judgment DEF-113 reuses (`check_storable_slot_requirements`).
- The emitter's intrinsic renderer (`compiler/src/codegen/calls.rs::intrinsic`) and guard arm
  (`compiler/src/codegen/guards.rs:98-104`) already exist for DEF-116's slice value and its
  bounds trap.
- The spec-suite helpers and the golden runner (`spec/check.sh`) are established; no new
  harness is built (AR-13).

### External Dependencies

- None. The workspace stays std-only; the emitted C relies on the existing shim/profile matrix.

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| DEF-113's exclusion reopens a case the source side reports (double report) | Low | Med | The direct-place branch is unchanged; probes p1/p3/p6/p8 pin the unchanged reports; impl tests pin counts |
| DEF-114 rejects a shape that should stay legal (writable slice / rebind) | Low | Med | ST-16/ST-17 pin the accepted controls; the helper fires only for `str`/`static str` element bases |
| The typedef reorder changes emission for unrelated units | Med | Low | Deterministic reorder; the emission-integrity auditor diffs base/head over the corpus; byte-identical re-emission pinned |
| DEF-116's fill diverges from the function-path fill semantics | Low | Med | The walk mirrors the established element-store order (`fill1` reference); ST-25 pins the fill value |
| The E10042 spec edit drifts wider than authorized | Low | Med | AR-12 fixes the modification set: one registry row + one golden |
