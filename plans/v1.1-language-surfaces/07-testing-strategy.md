# Testing Strategy: v1.1 Language Surfaces

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Test Split

| Layer | File | Derives from |
|---|---|---|
| Behavior oracle | `compiler/tests/v11_language_surfaces_spec_test.rs` | R1-R12 |
| Emission and ABI pins | `compiler/tests/v11_language_surfaces_impl_test.rs` | R1-R12 |
| Host hook fixture | `compiler/tests/fixtures/hook_fixture_host.c` | R11-R12 |
| m68k hook fixture | `compiler/tests/fixtures/hook_fixture_m68k.c` | R11-R12 |
| Existing closure pins | `closure_integrity_{spec,impl}_test.rs` | E10400 regression only |
| Existing raw-store pin | `conditional_projection_residue_spec_test.rs` | `boxSet` raw-store basis |

The spec suite states observable behavior: stdout, exit status, diagnostic
first line, fault line, and sanitizer cleanliness. The impl suite states emitted
C shapes: the presence and count of helper calls, trampoline/wrapper/env
symbols, the plain-pointer form, the absence of a heap environment, and the
ordinary-deleter symbol. A shape pin that changes for an unintended reason is a
failure.

## Red-First Evidence

Every new behavior gets a case that fails on the starting tree `a7dbdd5` or on
the first implementation attempt:

| Case | Red evidence on the starting tree |
|---|---|
| ordinary `freedBy` deleter | `E10102` on a valid `take T -> void` function |
| `Box<T>` exists | type resolves to `Unknown`/E10219; no API |
| capturing lambda at an `ffi` callback | accepted and emits `ClClosure_N` where `LONG (*)(LONG)` is expected; strict C build fails |
| generic aggregate at the FFI boundary | accepted and emits `ClWrap` with no typedef; strict C build fails |
| capture-less lambda at a hook | already a plain pointer; control |
| Box nested drop | no Box; after the first implementation, a missing recursive arm leaks or double-frees under ASan |

The first two are recorded in [Current State](02-current-state.md). The hook
and generic-aggregate red cases are recorded there too. The implementation
task that fixes a case commits the red-first test with the fix, so no commit is
red.

## Box Cases

| ID | Case | Expectation |
|---|---|---|
| BX-1 | `boxNew<i32>(7)`, `boxWith` prints 7 | run, exit 0 |
| BX-2 | `Box<String>`, `boxWith(bytes)` reads the bytes | run; ASan clean |
| BX-3 | `boxSet` replaces a `String` twice | old values freed once; ASan clean |
| BX-4 | `Box<StructWithString>` | field drops once; ASan clean |
| BX-5 | `Box<Box<String>>` | inner then outer free once; ASan clean |
| BX-6 | `Box<Array<String>>` | elements then buffer; ASan clean |
| BX-7 | `Node { next: Box<Node>? }` three deep | recursive drop; ASan clean |
| BX-8 | `Box<i32>? = null` drop | no-op |
| BX-9 | `Box<i32>` copied | `E10020` |
| BX-10 | `Box<i32>?` used without narrowing | `E10010` |
| BX-11 | `null` into a non-nullable box | `E10012` |
| BX-12 | `Box<str>` / `Box<u8[]>` | `E10227` |
| BX-13 | `Box<Overlay>` | `E10207` |
| BX-14 | `Box<closure>` | `E10306` |
| BX-15 | `Box` with two arguments | `E10300` |
| BX-16 | `Box<i32>` in `ffi` | `E10105` |
| BX-17 | freestanding `std.mem` import/Box | `E11200` |
| BX-18 | `sizeof(Box<i32>)` | pointer size |
| BX-19 | `boxNew` in a loop and a `defer` | one drop per box; ASan clean |
| BX-20 | `boxWith` capturing lambda | runs synchronously; environment live |
| BX-21 | `boxWith` view does not escape | `E10033` at the escape |
| BX-22 | `b.member` / `b[i]` / `b()` | E10212 / E10209 / E10309 |
| BX-23 | `Box<i32>` in a conditional and a return | exactly-once drop |

## X1 Cases

| ID | Case | Expectation |
|---|---|---|
| X1-1 | same-module ordinary `take T -> void` deleter | accepted; run; ASan clean |
| X1-2 | exported cross-module deleter `A.B.freeX` | accepted; mangled symbol |
| X1-3 | `ffi` deleter (regression) | accepted |
| X1-4 | wrong mode (`const`/`borrow`/none) | `E10102` |
| X1-5 | wrong parameter type | `E10102` |
| X1-6 | two parameters | `E10102` |
| X1-7 | non-void return | `E10102` |
| X1-8 | nullable return | `E10102` |
| X1-9 | unresolved name | `E10102` |
| X1-10 | `intrinsic` name | `E10102` |
| X1-11 | generic function | `E10102` |
| X1-12 | second `handle` declaration | `E11101` |
| X1-13 | `freedBy` on a struct | `E10104` |
| X1-14 | deleter body drops a `String` field | ASan clean, one release |
| X1-15 | handle copied after declaration | `E10020` |
| X1-16 | deleter calls an `ffi` release | run; ASan clean |

## Hook Cases

| ID | Case | Expectation |
|---|---|---|
| HK-1 | capture-less lambda at `borrow func` | plain pointer; no env/trampoline |
| HK-2 | capturing lambda at `borrow func` | correct result; host run |
| HK-3 | capturing lambda at unmarked `func` | `E10400` |
| HK-4 | capturing lambda at user `func` parameter | `E10400` |
| HK-5 | capturing lambda at `newHook`/`createTask`/`addHandler` | `E10400` |
| HK-6 | `[borrow x]` capture writes through the callback | updated value |
| HK-7 | two capturing callbacks in one call | each finds its env |
| HK-8 | nested hook calls | each finds its env |
| HK-9 | reentrant/recursive hook | each finds its env |
| HK-10 | repeat invocation of one callback | same env; correct |
| HK-11 | callback invoked after return | `error E11215`, nonzero exit |
| HK-12 | hook signature mismatch | `E10210` |
| HK-13 | non-callable argument | `E10200` |
| HK-14 | hook in a short-circuit operand | runs only on the evaluated path |
| HK-15 | capture-less hook in a generic body | accepted, plain pointer |
| HK-16 | capturing hook in a generic body | `E10400` |
| HK-17 | m68k cross-build + `vamos` run | correct result, `E11215` case excluded from the value path |

## Sanitizer Evidence

| Leg | What it proves |
|---|---|
| `RDC_ASAN` on the spec suite | every Box drop path and the hook environment are leak/double-free/use-after-free clean |
| `make asan` | the suite is discovered and run in its own sanitized process |
| `RDC_UBSAN` on the spec suite | the new pointer and arithmetic paths are defined |
| `make ubsan` | the suite is discovered and the sanitizer is active |
| host + m68k hook fixture | the caller-stack environment is valid on both ABIs |

## Test-Integrity Procedure

For each implementation task:

1. Run the focused spec and impl cases and record the red output in
   `05-findings.md`.
2. Apply the fix.
3. Re-run the same cases and the affected existing suites.
4. Revert only the production change in a scratch copy (or use the recorded
   `a7dbdd5` compiler) and confirm the same assertion fails again.
5. Restore the fix and run `make check`.

For a drop-path fix, the red check is an ASan leak or double-free report on
the starting tree or on the first implementation without the fix. For a
diagnostic fix, the red check is the accepted program or the wrong code.

## Gate Matrix

| Gate | When |
|---|---|
| focused spec + impl | after every implementation task |
| `make check` | after every task or coherent feature |
| `make asan` | after every new drop path, at phase boundaries, and at the end |
| `make ubsan` | at the phase boundary and at the end |
| `make corpus-amiga`, `make differential` | after the Box and X1 phases |
| `make bench-shape` | after the Box type and hook emission phases |
| `make verify` | before the final push |
| `docs/known-limitations.md`, roadmap, findings | before the final push |

## Commit Hygiene

- Docs commits separate from implementation commits.
- One verified task or coherent feature per implementation commit.
- Stage explicit paths; review `git diff --cached` before each commit.
- Never commit `build/`, `target/`, emitted C, captures, licensed media, or
  scratch files.
- Do not amend or force-push.
- Push only when the full gate is green; after pushing verify
  `git status -sb` is clean and `git rev-list --count origin/main..HEAD` is 0.
