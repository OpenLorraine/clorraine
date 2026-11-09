# Testing Strategy: Volatile Nested Fill Fix (PLAN-23)

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Test Split

| Suite | Kind | Purpose |
|---|---|---|
| `compiler/tests/nested_fill_compaction_spec_test.rs` | Specification | Behaviour oracle: emit, strict host build, run, compare output |
| `compiler/tests/nested_fill_compaction_impl_test.rs` | Implementation | Emitted-shape pins for the volatile and non-volatile paths |
| `tools/perf/fixtures/shapes.clo` | Build fixture | m68k GCC, GCC nix13, vbcc, and freestanding strict builds |

The existing specification cases pass on the base. The new volatile cases are
deliberately red on the base because the base emits invalid C or re-evaluates
an impure prefix. The implementation suite pins the intended spelling and the
paths that must not change.

## Behaviour Cases

Each running case emits host C, builds with
`cc -std=c99 -O0 -g -Wall -Wextra -Werror` (plus `-fsanitize=address` when
`RDC_ASAN` is set and `-fsanitize=undefined -fno-sanitize-recover=all` when
`RDC_UBSAN` is set), runs the program, and compares stdout.

| ID | Shape | Expected |
|---|---|---|
| ST-V1 | `volatile let g: i32[8][9] = [; [; 0]];` then `g = [; [; 0]];` read every slot | `sum: 0` |
| ST-V2 | `volatile let g: i32[2][3][4] = [; [; [; 0]]];` then `g = [; [; [; 7]]];` | `sum: 168` |
| ST-V3 | `struct R { v: i32; }`; `volatile let g: R[2][3] = [; [; R { v: 0 }]];` then `g = [; [; R { v: 9 }]];` | `sum: 54` |
| ST-V4 | `volatile let g: i32[4][4] = [; [; 0]];` then `g = [[1,2,3,4],[5,6,7,8]; [; 0]];` | `sum: 36` |
| ST-V5 | `volatile let g: i32[4][4]`; `let row: i32[4] = [; 1];` then `g = [row, row; [; 0]];` | `sum: 8` |
| ST-V6 | Overlay field `struct S { a: volatile i32[2][3]; }` written through `raw S` | builds under strict host C; not run |
| ST-V7 | `let i: i32 = 1; g[i] = [; [; 9]];` on a volatile `i32[2][3][4]` | `sum: 108` |
| ST-V8 | An impure prefix row `g = [makeRow(p); [; 0]];` where `makeRow` increments a counter | `calls: 1 sum: 28` |
| ST-V9 | Run-time indexed volatile overlay field `regs.a[i] = [; [; 9]];` | builds under strict host C; not run |
| ST-V10 | `struct S { p: volatile raw Inner; }`; `regs.p.v = [; [; 0]];` | builds under strict host C; not run |
| ST-V11 | Volatile destination filled from `h[0]`, `h[1]` where `h` is volatile | `sum: 36` |
| ST-V12 | Volatile destination filled from `h[k - 1]` where `h` is volatile | `sum: 26` |

ST-V1 is the reported defect. ST-V2 exercises the recursive copy. ST-V3
exercises a struct leaf. ST-V4 exercises a literal prefix. ST-V5 exercises the
place-valued prefix row. ST-V6, ST-V9, and ST-V10 are build-only because the
probe addresses are not real host objects. ST-V7 exercises the run-time
indexed element target. ST-V8 proves the impure prefix value is evaluated once.

The module-level initializer of each global is already element-wise and is not
the subject of the fix; the function-body assignment is.

## Emission Pins

The emitted C includes the runtime shim, which contains its own `memcpy` and
`memset` calls. Each pin matches the destination spelling, not a bare function
name. Global symbols are mangled `cl_<module>_<name>`; the pins use the full
symbol.

| ID | Kind | Shape | Pin |
|---|---|---|---|
| IP-V1 | Positive | volatile global `i32[8][9]` zero fill | no `memcpy(&(cl_impl_NestedVolatile_g[`; `cl_impl_NestedVolatile_g[0][0] = __cl_fillrow_1[0];` and `...[7][8] = __cl_fillrow_1[8];` present |
| IP-V2 | Positive | volatile global `i32[2][3][4]` non-zero fill | no `memcpy(&(cl_impl_NestedVolatile3_g[`; `cl_impl_NestedVolatile3_g[0][0][0] = __cl_fillrow_` and `...[1][2][3] = __cl_fillrow_` present |
| IP-V3 | Positive | volatile overlay field `regs->a` (`raw S`) | no `memcpy(&(regs->a[`; `regs->a[0][0] = __cl_fillrow_1[0];` present |
| IP-V4 | Positive | volatile global struct array `S[2][3]` | no `memcpy(&(cl_impl_NestedVolatileStruct_g[`; `cl_impl_NestedVolatileStruct_g[0][0] = __cl_fillrow_1[0];` present |
| IP-V5 | Positive | place-valued prefix row `g = [row, row; [; 0]]` | no `memcpy(&(cl_impl_NestedPrefixPlace_g[`; `cl_impl_NestedPrefixPlace_g[0][0] = row[0];` and `...[2][0] = __cl_fillrow_1[0];` present |
| IP-V6 | Positive | volatile field named `len` through `raw S` | no `memcpy(&(regs->len[`; `regs->len[0][0] = __cl_fillrow_1[0];` present |
| IP-V7 | Guard | non-volatile field `s.a: i32[8][9]` | `memcpy(&(s.a[0]), &(__cl_fillrow_` present; no leaf store |
| IP-V8 | Guard | non-volatile local `i32[8][8]` zero fill | one `memset(&(a[0]), 0, 8 * sizeof(a[0]));`, no `__cl_fillrow` |
| IP-V9 | Guard | volatile one-extent `i32[8]` fill | `cl_impl_NestedVolatile1_g[0] = 0;` stores; no target `memset` |
| IP-V10 | Guard | volatile full-prefix `i32[8][8]` zero fill | no `__cl_fillrow`, no marker |
| IP-V11 | Guard | volatile explicit rows `[[...], [...]]` | direct stores, no `__cl_fillrow` |
| IP-V12 | Guard | whole-array place copy `g = other` into a volatile global | exact `memcpy(&(cl_impl_NestedWholeCopy_g), &(other), sizeof(cl_impl_NestedWholeCopy_g));` present; no leaf store |
| IP-V13 | Positive | run-time indexed volatile global element `g[i]` | no destination `memcpy`; `(*(&(cl_impl_NestedVolatileIndex_g[` and `)))[0][0] = __cl_fillrow_` present |
| IP-V14 | Positive | impure prefix row `makeRow(p)` | no destination `memcpy`; exactly one `_makeRow(p,` call; leaf stores from the row temporary |
| IP-V15 | Positive | run-time indexed volatile overlay field `regs.a[i]` | no destination `memcpy`; `(*(&(regs->a[` and `)))[0][0] = __cl_fillrow_` present |
| IP-V16 | Positive | `volatile raw` pointer field `regs.p.v` | no `memcpy(&(regs->p->v[`; `regs->p->v[0][0] = __cl_fillrow_1[0];` present |
| IP-V17 | Positive | constant-indexed volatile source row `h[0]` | no destination `memcpy`; `g[0][0] = h[0][0];` present |
| IP-V18 | Positive | arithmetic-indexed volatile source row `h[k - 1]` | no destination `memcpy`; the `h[(i32)((u32)k - (u32)1)][0]` leaf store present |

IP-V7 and IP-V8 protect the non-volatile emission from the new flag. IP-V9
through IP-V12 protect the other volatile spellings and the separate
whole-array class. IP-V13 through IP-V16 cover the review-driven shapes.

## Red/Green Record

1. Add the behaviour cases and pins; do not change the compiler.
2. Run both focused suites on the base. Record:
   - ST-V1..V10 fail (strict C for the `memcpy` shapes; ST-V8 also shows the
     prefix call re-evaluated four times);
   - IP-V1..V6 and IP-V13..V16 fail (destination `memcpy` present, leaf
     stores absent);
   - IP-V7..V12 pass.
3. Implement the fix. Both suites pass.
4. Test integrity, each applied and restored:
   - restore the `memcpy` fill-row copy: ST-V1..V7 fail and IP-V1..V4,
     IP-V13, IP-V15 fail;
   - remove the prefix wrapper: ST-V5 and IP-V5 fail;
   - make `fill_target_volatile` return false: the volatile place pins fail;
   - make it return true: IP-V7 and the non-volatile compaction pins fail;
   - make the `.len` guard unconditional: IP-V6 fails;
   - remove the copy recursion: IP-V2 fails;
   - treat an impure prefix value as stable: ST-V8 and IP-V14 fail;
   - disable the run-time index volatility: ST-V7/ST-V9 and IP-V13/IP-V15
     fail;
   - reject a constant or arithmetic index as stable: ST-V11/ST-V12 and
     IP-V17/IP-V18 fail.
5. Record every run in [Findings](05-findings.md).

## Strict-Shape Fixture

`tools/perf/fixtures/shapes.clo` gains:

- `volatile let vg: i32[8][8] = [; [; 0]];` and a function-body prefix fill
  `vg = [row, row; [; 0]];` with a read that keeps the global live;
- an overlay struct with a `volatile i32[2][3]` field and a helper that writes
  it through a `raw` pointer inside `unsafe`. The helper is called from `main`
  so the strict C build does not report an unused static function.

`make bench-shape` builds the fixture with m68k GCC, GCC nix13, vbcc, and the
freestanding profile. All four C legs must be warning-free.

## Sanitizers

The behaviour suite carries the `RDC_ASAN` and `RDC_UBSAN` discovery markers
and the same builder flags as the existing cases. `make asan` and `make ubsan`
discover it and include ST-V1..V5 and ST-V7..V8.

## Emission Audit

The base `clc` is built from `7cdbb71` with `make bench-base COMMIT=7cdbb71`.
The base and head emit:

- all seven `spec/tests/corpus/*/program.clo` sources;
- `benchmarks/*/program.clo`;
- `examples/amiga/*.clo`.

No committed corpus, benchmark, or example source contains a nested fill into
volatile storage, so the audit is expected to be byte-identical. The auditor
also emits the probe shapes from the test suites and confirms the only diffs
are the per-leaf copies.

## Benchmark Check

The fix must not change a scored benchmark. Build a fresh base at the fix's
parent and compare with the default 1% guard:

```text
make bench-base COMMIT=7cdbb71
make bench
make bench-compare
```

No `REQUIRE` is passed: this plan claims no improvement, only no regression.
Record both compiler SHA-256 values and `dirty=false`. The matrix win from
PLAN-22 is already in both sides and cancels out. Any codegen change after the
measurement reruns this block on the new clean commit.

## Gate Commands

| Gate | Command |
|---|---|
| Focused | `make -C docker exec CMD="cargo test --test nested_fill_compaction_spec_test --test nested_fill_compaction_impl_test"` |
| Fast | `make check` |
| Sanitizers | `make asan`, `make ubsan` |
| Target | `make corpus-amiga`, `make differential` |
| Strict shapes | `make bench-shape` |
| Benchmark | `make bench-base COMMIT=7cdbb71`, `make bench`, `make bench-compare` |
| Full acceptance | `make verify` |

A failed gate blocks the push and requires a follow-up fix commit; a commit
already made is corrected by the follow-up, never force-pushed.
