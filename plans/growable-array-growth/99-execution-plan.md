# Task: Amortize growable-array growth

> **Type**: Task (lightweight) · **CodeOps Artifact Schema**: 1
> **Progress**: 0/9 tasks (0%)
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

- [ ] T-01 Write the specification tests: the `container` benchmark must stay
  byte-exact and fall by at least 70% net instructions, and every scored
  benchmark must keep its expected output. Record the red baseline: the audit
  measurement is 223,504 net instructions, so
  `make bench-compare REQUIRE=container:instructions:70` fails before the
  change and must pass after it.
- [ ] T-02 Extend the existing `compiler/tests/stdlib_impl_test.rs` case
  `buffer_from_and_array_growth_keep_their_contents` (20 pushes, reserve(64),
  first/last/count checks) to read every value past the first geometric step
  back against a control array. Reuse that test instead of adding a parallel
  one; it passes on both growth policies and is the behavioral regression
  oracle. The failing specification test is the `container` instruction
  threshold in T-01.
- [ ] T-03 Implement the geometric growth in `shim_host.c` with
  `UINT_MAX` overflow clamping, reusing the growth loop already in
  `cl_buffer_append` (or a shared `grown_cap` helper) rather than duplicating
  it.
- [ ] T-04 Implement the same policy in `shim_hosted_amiga.c` with
  `ULONG_MAX` overflow clamping, reusing that shim's existing append loop.
- [ ] T-05 Pin the emitted-C and benchmark evidence: `Array.push`/`Array.reserve`
  keep their emitted call shape (base/head `clc --profile hosted-amiga` emit
  diff); `benchmarks/container` is at least 70% lower net instructions and its
  `.text` stays inside the 1% guard; no scored benchmark regresses more than
  1%.
- [ ] T-06 Run the independent correctness and Amiga platform reviews; fix
  every critical/major finding.
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
| Extra capacity changes a memory measurement | The scored metric is instructions, `.text`, and linked size; the audit baseline records all three |
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
