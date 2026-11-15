# Task: Amortize growable-array growth

> **Type**: Task (lightweight) · **CodeOps Artifact Schema**: 1
> **Progress**: 6/9 tasks (67%)
> **Reasoning**: medium — one growth-policy change in two runtime shims, but it
> crosses host, hosted-amiga, and every container-using program, so it keeps
> the full gate and the Amiga platform review.

## Objective

Deliver issue #33: stop `Array.push` from reallocating and zeroing on every
element. The real-workload audit (#14) measured `benchmarks/container` at
223,504 net m68k instructions; 58.0% of the trace is allocator work because
`cl_buffer_reserve` grows to the exact requested size each time. A probe that
doubles the capacity measured 56,814 net instructions (-74.6%); `.text` moves
from 24,092 B to 24,108 B (+16 B, +0.07%), inside the 1% guard.

**Smallest viable design:** change only the growth policy in
`cl_buffer_reserve` in `compiler/runtime/shim_host.c` and
`compiler/runtime/shim_hosted_amiga.c`: start at eight bytes, double until the
request fits, clamp at the type maximum, and zero exactly the newly grown
region. Keep the private `cap` field at the new capacity. No language, ABI,
stdlib-source, emitted-C, or public-surface change.

## Tasks

- [x] T-01 Write the specification tests: the `container` benchmark must stay
  byte-exact and fall by at least 70% net instructions, and every scored
  benchmark must keep its expected output. Record the red baseline: the audit
  measurement is 223,504 net instructions, so
  `make bench-compare REQUIRE=container:instructions:70` fails before the
  change and must pass after it. The red run reports `+0.0% ... need <= -70%
  FAIL` with every base/head row at `+0.0%`.
- [x] T-02 Extend the existing `compiler/tests/stdlib_impl_test.rs` case
  `buffer_from_and_array_growth_keep_their_contents` (20 pushes, reserve(64),
  first/last/count checks) to read every value past the first geometric step
  back against a control array. Reuse that test instead of adding a parallel
  one; it passes on both growth policies and is the behavioral regression
  oracle. The failing specification test is the `container` instruction
  threshold in T-01.
- [x] T-03 Implement the geometric growth in `shim_host.c` with
  `UINT_MAX` overflow clamping, reusing the growth loop already in
  `cl_buffer_append` (or a shared `grown_cap` helper) rather than duplicating
  it.
- [x] T-04 Implement the same policy in `shim_hosted_amiga.c` with
  `ULONG_MAX` overflow clamping, reusing that shim's existing append loop.
- [x] T-05 Pin the emitted-C and benchmark evidence: `Array.push`/`Array.reserve`
  keep their emitted call shape (base/head `clc --profile hosted-amiga` emit
  diff); `benchmarks/container` is at least 70% lower net instructions and its
  `.text` stays inside the 1% guard; no scored benchmark regresses more than
  1%.
- [x] T-06 Run the independent correctness and Amiga platform reviews; fix
  every critical/major finding. Both reviews are clean; the minor evidence
  gaps are recorded in the Findings section.
- [ ] T-07 Run `make check`, `make asan`, `make ubsan`, `make bench`,
  `make bench-base COMMIT=d44c62fd`, `make bench-compare`,
  `make bench-shape`, `make corpus-amiga`, `make differential`, and
  `make verify`.
- [ ] T-08 Record the base/head table, allocation evidence, review verdicts,
  and gate results in the task findings.
- [ ] T-09 Update the roadmap and close #33.

**Verify**: `make check`; `make asan`; `make ubsan`; `make bench`;
`make bench-base COMMIT=d44c62fd`; `make bench-compare`;
`make bench-shape`; `make corpus-amiga`; `make differential`; `make verify`.

## Design constraints

| Constraint | Reason |
|---|---|
| Growth starts at 8 bytes and doubles | Matches `cl_buffer_append`'s existing policy and removes the quadratic copy |
| Clamp at `UINT_MAX`/`ULONG_MAX` | A doubling step must not wrap before `realloc` |
| Zero only `grown_cap - old_cap` | The old region is already initialized; the new region must stay canonical |
| `cap` remains private | No public surface or ABI change |
| `Array.push`/`Array.reserve` sources unchanged | The fix is the shim policy, not a stdlib API change |
| `cl_buffer_append`/`cl_buffer_write` keep their requests | Both already pass a capacity at least as large as the payload |

## Known risks

| Risk | Mitigation |
|---|---|
| A caller relies on `cap == requested` | `cap` is private to the shim; all readers use `len`, `ptr`, or bounds derived from `len` |
| Extra capacity raises peak heap | Peak capacity is less than twice the live bytes; a request that fit at the exact size can now trap `E1` near the allocator limit. No scored metric measures heap, so this is an accepted tradeoff for the instruction win and is recorded here |
| Host and Amiga shims drift | The same growth loop and clamp are applied to both, with the type-specific maximum |
| Existing tests pin exact allocation counts | Search for `cl_buffer_reserve`/capacity assertions first; the plan's T-01 records any such test before the change |

## Evidence from #14

| Metric | Exact-size shim | Geometric probe |
|---|---:|---:|
| `container` net instructions | 223,504 | 56,814 (-74.6%) |
| `container` m68k `.text` | 24,092 B | 24,108 B (+16 B, +0.07%) |
| `container` output | `container: 10475520 1024` | same |

The probe patched the emitted C only; it is not committed. See
`plans/real-workload-audit/99-execution-plan.md` for the attribution table.

## Findings

### Red baseline

`make bench-compare REQUIRE=container:instructions:70` on the unchanged shim:

```
container           223504      223504    +0.0%       24092       24092    +0.0%
...
  container   instructions     +0.0%  need <= -70%  FAIL
compare: FAILED
```

Every base/head row was `+0.0%`, so the base compiler and the unchanged head
are the same behavior.

### Green result

`make bench-compare REQUIRE=container:instructions:70` after the change:

| Benchmark | Base instructions | Head instructions | Delta | Base `.text` | Head `.text` | Delta |
|---|---:|---:|---:|---:|---:|---:|
| `container` | 223,504 | **56,814** | **−74.6%** | 24,092 B | 24,108 B | +0.1% |
| `local-table` | 2,927 | 2,927 | +0.0% | 24,568 B | 24,568 B | +0.0% |
| `lookup` | 8,779 | 8,779 | +0.0% | 39,884 B | 39,884 B | +0.0% |
| `matrix` | 9,361 | 9,361 | +0.0% | 23,592 B | 23,592 B | +0.0% |
| `moves` | 44,630 | 44,630 | +0.0% | 23,684 B | 23,684 B | +0.0% |
| `sieve` | 1,040,081 | 1,040,081 | +0.0% | 23,816 B | 23,816 B | +0.0% |
| `strings` | 1,221,232 | 1,221,232 | +0.0% | 23,972 B | 23,972 B | +0.0% |
| `text-scan` | 375,678 | 375,678 | +0.0% | 24,152 B | 24,152 B | +0.0% |

```
require:
  container   instructions    -74.6%  need <= -70%  PASS
compare: passed
```

The `+0.1%` display is the 16-byte `.text` increase rounded to one decimal
place; the exact delta is +0.066%, inside the 1% guard. Every scored output
hash is unchanged.

### Allocation evidence

A host probe counted the shim's `realloc` calls on the 1,024-element fill. The
probe is a scratch copy under `build/tmp`, not a committed change:

| Compiler | Realloc calls | Output |
|---|---:|---|
| Base `d44c62fd` | 1,024 | `container: 10475520 1024` |
| Head | 11 | `container: 10475520 1024` |

The capacity sequence is 8, 16, 32, ..., 8192 bytes (1, 2, 4, ..., 1,024
`Point` elements), so eleven reallocations replace one per push.

### Emitted shape

The base and head `clc --profile hosted-amiga` emits of
`benchmarks/container/program.clo` differ in four hunks, all inside the
embedded runtime shim:

1. the new `cl_buffer_grown_cap` helper;
2. `cl_buffer_reserve` requesting and zeroing the grown capacity;
3. `cl_buffer_append` delegating the growth to `cl_buffer_reserve`;
4. the already-committed `UnlockPubScreen` NDK prototype correction, present
   because base `d44c62fd` predates that commit.

No `Array.push`/`Array.reserve` call site, workload statement, or
monomorphized instance changed.

### Behavioral oracle

`buffer_from_and_array_growth_keep_their_contents` now reads every pushed value
back against an independent fixed-array control and reports
`total=190 mismatches=0`. The `buffer-container` corpus case and all six corpus
cases pass.

### Target coverage

The shims are shared by `hosted-amiga` (nix20) and `hosted-amiga-13` (nix13).
The benchmark trace and `make corpus-amiga` exercise the nix20 path;
`bench-shape` compiles the 1.3 shape but does not link or run it. A manual 1.3
probe of `benchmarks/container` was emitted with
`--profile hosted-amiga-13`, linked with `m68k-amigaos-gcc -std=c99
-mcrt=nix13 -m68000 -fwrapv -O0 -g -Wall -Wextra -Werror`, and run under
`vamos`: warning-free, output `container: 10475520 1024`. A committed nix13
run gate for buffer growth is a possible follow-up, not part of this plan.

`cl_buffer_append` is declared FFI surface but has no stdlib caller today
(`push` uses `cl_buffer_reserve`, `bufferFrom` uses `cl_buffer_write`); its
delegation preserves the previous growth policy for any caller and is not
exercised by the current corpus.

### Review findings

The Amiga platform review found no code defect and recorded two minor
evidence gaps:

| ID | Finding | Resolution |
|---|---|---|
| AP-001 | No committed gate runs buffer growth on the 1.3 baseline | Recorded the manual nix13 emit/link/run probe above; the 1.3 path is compile-covered by `bench-shape` and run-verified by that probe |
| AP-002 | The plan's memory mitigation named code metrics, not heap | Replaced with the real bound (peak < 2× live bytes) and the accepted near-limit `E1` consequence |

The correctness review reported no findings. It independently reproduced the
−74.6% / +16 B result, the 1,024 → 11 realloc probe, the shim-only emit diff,
the base/head behavioral test, all six corpus cases, clean ASan/UBSan probes,
and warning-free nix13, nix20, and vbcc builds. Its non-blocking note is
recorded here: the extended stdlib test is a behavioral oracle and passes on
both policies; the `REQUIRE` benchmark threshold is the pinning oracle, so a
future revert to exact-size growth would be caught by a base/head comparison
rather than by a standing instruction threshold. The harness's 1% and output
guard remains the permanent comparison gate.

Reviewer confirmations: the 32-bit `UINT_MAX`/`ULONG_MAX` clamp cannot wrap
(boundary probe `8, 8, 16, 2000, 8192, needed`), the `size_t` conversions
cannot truncate, `cap` is read and written only inside the shim, `FreeBuffer`
and `cl_buffer_ptr` use `data`/`len` only, the allocator is `malloc`/`free`
rather than `AllocMem`, and both shims build warning-free for nix13, nix20,
and vbcc with no unused-static warning when a program omits `std.mem`.
