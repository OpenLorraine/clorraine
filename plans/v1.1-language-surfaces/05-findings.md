# Findings: v1.1 Language Surfaces

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-11-08

This ledger records the starting evidence, the red/green evidence for every
phase, the gate logs, the review findings, and the residual limits. It is
updated as the execution plan progresses; a claim without a recorded command
or output is not evidence.

## 1. Baseline

| Check | Result |
|---|---|
| Starting tree | `a7dbdd5` — `docs(tests): describe compiler robustness cases in plain language` |
| Working tree | clean before the plan work |
| `git rev-list --count origin/main..HEAD` | `0` |
| `make check` | exit 0; fuzzer smoke `cases=28 host=28 sanitizer=16 target=16 failures=0` |
| Only release tag | `v1.0.0` |
| Baseline log | `build/v11/make-check-baseline.log` (ignored) |

## 2. Requirements and Plan Gates

| Gate | Result |
|---|---|
| Requirements preflight | ✅ PASSED WITH NOTES — 23 findings (7 critical, 14 major, 2 minor); 21 resolved, 1 accepted (pre-existing `Array<str>` hole), 1 incorporated (`Box<static str>`) |
| Plan preflight | ✅ PASSED — 19 findings (3 critical, 14 major, 2 minor); all resolved |
| Requirements commit | `8a91bc5` — `docs(plans): specify the v1.1 language surfaces` |
| Revised docs commit | recorded by 0.1.4 |

## 3. Red Evidence

Each phase records the exact failing output before the fix. The expected
starting behavior is in [Current State](02-current-state.md).

### 3.1 Box type model

| Case | Starting behavior |
|---|---|
| bare `Box<i32>` | `E10219: 'Box' is not declared` |
| user `struct Box<T>` | accepted |
| `Box {}` / `Box(x)` / `Box<str>` | `E10219` before the name resolves; after 1.2.5 they are the accepted cases the new checks close |
| `Box<i32>` in `ffi` | `E10219` before the name resolves; the generic-aggregate `Wrap<i32>` case emits `ClWrap` with no typedef |

**Red run (2026-11-08)**: `cargo test --test
v11_language_surfaces_spec_test` — 22 failed, 0 passed. The
`box-arity-zero` and `box-arity-two` cases reported `["E10219"]` instead of
`E10300`; the user-`Box` case was accepted; every other case stopped at
`E10219`. Log: `build/v11/spec-red-phase1.log` (ignored).

### 3.2 Box allocation and drop

Records the first implementation attempt's output (the API/type are absent on
the base).

### 3.3 X1 ordinary deleters

| Case | Starting behavior |
|---|---|
| valid `take T -> void` ordinary deleter | `E10102` |
| two modules with the same handle and different deleters | accepted; validator and emitter disagree |

### 3.4 L5 hooks

| Case | Starting behavior |
|---|---|
| capturing lambda at an unmarked `ffi` callback | accepted; emits `ClClosure_N` where a function pointer is expected; strict C build fails |
| capturing lambda at a `borrow` callback | same invalid C (the marker is not special yet) |
| view/borrow/fixed-array callback | accepted; the lifted lambda's C spelling differs from the ABI |

**Red run (2026-11-08)**: the new hook suite compiled the base with
`cargo test --test v11_language_surfaces_spec_test` — 9 of the new cases failed
and 63 older cases passed. The compiler accepted a capturing lambda at an
unmarked, `const`, `take`, and `borrow` callback and emitted
`((ClClosure_0){ cl_lambda_0, ... })` into a `LONG (*)(LONG)` parameter; it also
accepted `str`, `i32[]`, `i32[2]`, and `func(): i32[2]` callbacks, a
`borrow Owned` (move-only struct) callback, and a non-callable `1` argument.
The mismatch case reported `E10219` only because the first probe used a
non-existent `i64` spelling; the committed case uses `u8` and now passes.

## 4. Gate Log

| Gate | Command | Result | Log |
|---|---|---|---|
| Baseline | `make check` | exit 0 | `build/v11/make-check-baseline.log` |
| Requirements preflight | document audit | passed with notes | [report](00-preflight-report-requirements.md) |
| Plan preflight | document audit | passed | [report](00-preflight-report.md) |
| Phase 4 focused suite | `cargo test --test v11_language_surfaces_spec_test` | 74 passed, 0 failed | `build/v11/spec-green-phase4.log` (ignored) |

The phase gate logs are appended as each phase completes.

## 5. Review Findings

The per-phase reviewers are dispatched by the execution plan:

| Phase | Reviewers | Result |
|---|---|---|
| 1 | correctness + emission-integrity | 2 reviews; 12 findings, all resolved or explicitly deferred |
| 2 | correctness + memory-safety | 10 findings; 9 fixed, 1 pre-existing capture UAF recorded |
| 3 | correctness + memory-safety | 9 findings; 8 fixed, 1 minor cleanup |
| 5 | correctness + memory-safety + emission-integrity + semantics | pending |
| 7 | final cross-phase | pending |

### Phase 1 review (Box type model)

| Finding | Severity | Resolution |
|---|---|---|
| A bare built-in `Box` was accepted as a value and emitted `Box` | major | `check_name` now reports `E10219` for a bare `Box` type name; a spec case pins it |
| A Box parameter/local was not read, so the strict C build could warn | major | `emit_drop_expr`/`drop_expr_text`/`emits_drop` now reference the place; the real recursive release lands with the allocator in Phase 2 |
| `Box<T>? == null` was `E10222` | major | `comparable` now includes `Box`; a spec case pins the comparison and narrowing |
| `Box<u8[]>` and `Box<Holder<str>>` were accepted | major | the shared `box_rule_violation` walk checks slices and follows struct fields after substitution; two spec cases pin them |
| Shadowing was only checked for a struct named `Box` | major | `builtin_box_active` covers type parameters, structural/enum/handle declarations, and imports; enum/handle shadow cases pin it |
| A raw cast of a Box outside `std.mem` could be reached through a generic parameter | major | direct casts are rejected; the substituted-instance audit is task 2.2.5 (the remaining path is inside `unsafe`) |
| A user module named `std.mem` could claim the raw-cast exemption | major | the gate now requires the embedded std.mem module |
| `Box<i32?>` reported `E10011` twice | low | the duplicate Box arm was removed; the argument walk reports once |
| `Box<i32, u8>(p)` missed `E10300` | low | the call-shaped branch checks the type-argument count |
| `E10105`/`E10227` were not registered and had no goldens | low | registry rows and three compile-fail files added; the compile-fail and DX counts were updated |
| FFI struct fields are not walked for a nested Box | minor | accepted: the emitted C is valid and the boxed field is opaque; the direct Box rule is enforced |
| `Box<Wrap<i32>>` emits an unreferenced inner instance | minor | accepted: the instance is recorded for the future drop helper and builds strictly |

The generic raw-cast audit (task 2.2.5) covers the one deferred rule; the
exploit is inside `unsafe` and requires an explicit or inferred generic
instantiation with a Box argument.

### Phase 2 review (Box allocation, drop, and library surface)

The correctness and memory-safety reviewers reproduced the happy paths and
found the following; all but the pre-existing capture case were fixed in this
phase.

| Finding | Severity | Resolution |
|---|---|---|
| `boxNew<u8[]>` / `boxNew<str>` bypassed the element rule through generic inference and produced a safe-code use-after-free | critical | a function-instance audit substitutes each generic parameter/return and runs `box_rule_violation`; spec cases pin `boxNew<u8[]>` and `boxNew<str>` |
| A nullable `Box<T>?` passed to `boxWith`/`boxSet` inferred `T` and dereferenced null | major | the ownership call check now reports `E10010` for a concrete non-nullable parameter when the flow state is not narrowed; a spec case pins it |
| A local or imported `boxAlloc` was hijacked by the intrinsic lowering | major | `lower_box_alloc` applies the same shadow guards as `lower_intrinsic_call`; a local-shadow run case pins it |
| `import { Box }` / `import { Box as B }` disabled the Box diagnostics | major | `builtin_box_active` recognizes the built-in import and `is_box_name` follows aliases; view and alias cases pin it |
| A user `struct Box<T>` broke `boxWith` inference | major | inferred parameter types resolve in the declaring module; a coexist run case pins it |
| Box helper definitions preceded a user deleter's prototype | major | the helper block is emitted after the prototypes; `Box<Buffer>` builds strictly |
| `Box<T[4]>` / `Box<func(...)>` emitted an invalid `raw T` return declarator | major | `raw_pointer_type` and the function declarator spell pointer-to-row and pointer-to-function-pointer returns; both run cases pin it |
| `box_rule_violation` stops at depth 32 | minor | accepted for now; the depth guards a recursive type that cannot contain a deeper concrete view without a written `Box` chain |
| Host `cl_box_alloc` took `unsigned int` | minor | changed to `size_t`, matching `cl_alloc` |
| A capturing closure that captures a local also lent to the same call is a pre-existing use-after-free | critical, pre-existing | reproduced on base `d0addac` without Box; outside this feature, routed to the borrow checker |

The ASan proof lives in `v11_language_surfaces_memory_spec_test`, a dedicated
suite with one comprehensive program; the behavior suite stays fast and the
`make asan` runner discovers the memory suite by its `RDC_ASAN` gate.

### Phase 3 review (ordinary `freedBy` deleters)

The correctness and memory-safety reviewers found the following; all are
fixed in this phase.

| Finding | Severity | Resolution |
|---|---|---|
| A non-consuming deleter dropped its own `take` parameter and called itself recursively | critical | the deleter's own parameter is excluded from the implicit scope drop, and the AST check rejects a body that never passes it to a call; the forwarding/structural tests and the combined memory case cover the runtime |
| Two ordinary candidates plus one `ffi` candidate resolved to the `ffi` | major | the program-wide fallback counts ordinary and `ffi` candidates together and requires exactly one; an ambiguous name is E10102 |
| A private cross-module ordinary function was accepted as a deleter | major | a cross-module ordinary candidate requires `export`; a private one is E10102 |
| A non-function declaration in the handle's own or imported scope did not shadow the fallback | major | the scope lookup stops at any declaration of the written name and accepts only `func`/`ffi` |
| A user `handle String` was accepted and its deleter ignored | major | a synthetic predeclared origin makes the duplicate E11101 |
| X1 had no sanitizer coverage and the acceptance test did not run the deleter | major | the same-module acceptance test runs through a C fixture, and the dedicated memory suite contains a structural/forwarding deleter case under ASan |
| The impl pin could match the deleter body's own self-drop | minor | the test sources consume the handle through an ffi release, so only the caller's drop names the ordinary symbol |
| The emitter fell back to the bare handle name when the identity was missing | minor | the emitter now panics on a missing internal identity; a valid program always records one |
| A vacuous resolver test and an unused `type_name_matches` remained | minor | both removed; the stale `resolve_dotted_name` doc is corrected |

## 6. Residual Limits

The v1.1 limits to document in `docs/known-limitations.md`:

- `Box<T>` allocation failure traps; `Box<T>?` is an optional slot, not an
  allocation result.
- A boxed value cannot be moved out; `boxSet` replaces it and `boxWith` lends
  it read-only. A raw cast of a Box outside `std.mem` is rejected.
- Direct Box FFI crossing is rejected (`E10105`); use `boxWith`.
- Freestanding has no Box (`std.mem` is not supported).
- The C hook environment is caller-stack only; the callback is synchronous.
- A capturing hook is not supported in a generic function body.
- Hook callback signatures are restricted to ABI-identical types; views,
  borrow move-only structs, fixed arrays, and fixed-array returns are
  rejected.
- A pre-existing `Array<str>` view-storage hole is outside this feature; the
  Box path (`Box<Array<str>>`) is closed by `E10227`.
