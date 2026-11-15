# Task: Audit real-workload generated-C hot shapes

> **Type**: Task (lightweight) · **CodeOps Artifact Schema**: 1
> **Progress**: 8/8 tasks (100%) — full `make verify` green
> **Reasoning**: medium — measurement-only audit over two committed real
> workloads, with one harness test and no compiler change. One gate-unblocking
> host-shim prototype fix (DEF-559) is included. The follow-up optimization is
> scoped out and receives its own plan.

## Objective

Deliver issue #14: add one or two real portable workloads with fixed expected
output, measure their m68k instruction count and `.text`, attribute the top
costs to concrete emitted-C shapes, and record a clear decision — a no-op with
evidence, or a targeted shape that becomes its own requirements/plan.

**Smallest viable design:** add two corpus-derived workloads under
`benchmarks/` where the existing harness discovers them, add one
fixture-completeness test, measure with `make bench`, and attribute with the
existing m68k instruction trace plus disassembly. No harness behavior change
and no compiler change. The array-growth finding becomes a separate follow-up
issue and task plan. The only runtime-shim edit is the pre-existing
`UnlockPubScreen` prototype correction recorded as DEF-559, needed to unblock
the full gate.

## Tasks

- [x] T-01 Write the fixture-completeness test in
  `tools/perf/tests/test_bench.py` and record the red run before the fixtures
  exist.
- [x] T-02 Add `benchmarks/text-scan/` (fixed command line, 64 passes, `str`
  scanning, enum dispatch, struct return) and `benchmarks/container/` (1,024
  `Point` structs in an `Array<Point>`, four scans through `get(i)[0]`), each
  with `program.clo`, `README.md`, and `expected-output.txt`.
- [x] T-03 Confirm byte-exact output on host and hosted-amiga and record the
  harness measurements (net instructions, `.text`, C size).
- [x] T-04 Attribute the top costs: shipped symbol trace, a forced-noinline
  analysis build for function shares, and loop disassembly for the bounds-check
  share.
- [x] T-05 Run the geometric-growth probe on the emitted container shim and
  record the measured delta.
- [x] T-06 Record the audit table and the decision; file the growable-array
  follow-up issue and its task plan.
- [x] T-07 Run the independent performance and correctness reviews, and fix
  every critical/major finding.
- [x] T-08 Run `make check`, `make bench`, `make bench-base`,
  `make bench-compare`, and the full `make verify` gate; update the roadmap and
  close #14. The A500 smoke capture was made deterministic (pinned 640x256
  window, fixed crop, 640x256 size assertion); `make verify` now reports
  `verify: all legs passed` (exit 0). A500 smoke exit 0 with the final script.

**Verify**: `make check`; `make bench`; `make bench-base COMMIT=d44c62fd`;
`make bench-compare`; `make verify`.

## Key decisions

| Decision | Outcome |
|---|---|
| Workload origin | The two existing portable corpus programs, scaled with fixed inputs: `string-parser` and `buffer-container` |
| Workload count | Two, so the audit covers both a text scanner and a container/generic workload |
| Harness change | None; the directory auto-discovery and expected-output contract already fit |
| Attribution method | Shipped `-O2` m68k trace with symbol ranges, a forced-noinline analysis build for function shares, and loop disassembly for the per-check cost |
| Bounds-check probe | The emitted C is patched only in `build/tmp`; the patch is evidence for a follow-up, not a committed change |
| Compiler decision | No compiler change: the text scanner's cost is inherent, and the container's top cost is the runtime shim's exact-size growth |
| Gate-unblocking fix | DEF-559: the pre-existing hosted-amiga `UnlockPubScreen` prototype disagreed with the NDK and failed the vbcc `bench-shape` leg; corrected to the NDK signature, with an Amiga platform review |
| Follow-up | A separate issue and task plan for amortized `cl_buffer_reserve` growth in both shims |
| Specialist check | None new — the audit is covered by the performance and correctness reviews; the follow-up shim change is covered by the existing correctness and Amiga platform reviewers |

## Audit findings

### Measured workloads

| Workload | Net m68k instructions | m68k `.text` | Hosted-amiga output |
|---|---:|---:|---|
| `container` | 223,504 | 24,092 B | `container: 10475520 1024` |
| `text-scan` | 375,678 | 24,152 B | `text-scan: tokens=3648 numbers=2432 words=1216 sum=146944` |

The net count subtracts the 2,928-instruction calibration count. The
calibration program prints one line and exits, so the subtraction also removes
one console path; it is a fixed cross-program offset, not pure startup.

**Attribution method.** `make bench` emits the hosted-amiga C with
`clc --profile hosted-amiga`, builds it with the shipped `-O2` flags, and runs
`vamos --instr-trace`. For the symbol table, the same hosted-amiga C was
rebuilt with the shipped flags minus `-s` so the linked hunk keeps symbols;
the hunk format carries no DWARF line table. The load base is **0x260c**: the
first traced PC is 0x260c and maps to `__stext` at offset 0, so subtracting
0x260c maps each PC to the nearest preceding `m68k-amigaos-nm` text symbol.
PCs outside `__stext..__etext` are ROM/library calls; the container trace has
23,692 such instructions and the text-scan trace 397. The shipped traces are
226,432 instructions for `container` and 378,606 for `text-scan`, matching the
harness raw counts.

The function shares come from a temporary copy of the **hosted-amiga** C
where the workload functions are tagged `__attribute__((noinline, used))`;
that build changes the total (372,902 and 479,470 instructions), so its shares
are labelled as an analysis build, not as the shipped count. The bounds-check
detail is read from the shipped `_main` disassembly: the `classify` loop at
`_main+0x94` is a `dbf` pointer loop with one pointer-bound compare and no
separate view-length guard; the digit loop at `_main+0x20a` still compares the
input bound and the character count each digit.

### Top costs and candidate shapes

| Candidate shape | Where measured | Finding | Decision |
|---|---|---|---|
| Dynamic-length bounds checks | `text-scan` `classify`/`parseNumber` | In the forced-noinline analysis build, `classify` is 35.7% and `parseNumber` 21.1% of instructions. In the shipped `-O2` build, the `classify` loop at `_main+0x94` is a `dbf` pointer loop with one pointer-bound compare per character and no separate view-length guard; the digit loop at `_main+0x20a` still compares the input bound and the character count each digit. The digit residual is about 4,160 digits × 2 compares ≈ 8,300 instructions (~2.2%); counting the classify loop bound gives a ~4–6% upper bound for the scanner. | No compiler plan: the residual is small and a range-analysis change would be much larger than the measured win |
| Generic-call overhead | `container` `Array<Point>.get`/`push` | The monomorphized instances are inlined into `main` at `-O2`; the shipped scan loop at `_main+0x74` reads `x` and `y` directly (`addl d1,d1; addl a1@(4),d1`) with no call or bounds guard. The hosted-amiga analysis build puts `get` at 24.2%, `sumPoints` at 16.5%, `push` at 6.9%, and `cl_buffer_reserve` at 6.6% of its trace, which is the one-element-view construction and the reserve call, not dispatch. | No compiler change |
| Function-local constant tables | Existing `local-table` benchmark | Net 2,927 instructions, `.text` 24,568 B; the block-scope `static const` template is already the compact shape measured by PLAN-36. | No change |
| Struct copies in non-inlined real code | `container` `Point` push/get; `text-scan` `Stats` return | `Point` is 8 bytes and `Stats` is 16 bytes; the `-O2` build inlines the boundaries, and the shipped container scan loop reads the two fields directly with no `memcpy` in the hot path. The copies are not a top cost. | No compiler change |
| Growable-array growth churn | `container` | The allocator is 58.0% of all traced instructions (`malloc` 15.5%, `memset` 15.4%, `free` 13.9%, `realloc` 13.1%); `Array.push` requests `(count + 1) * sizeof(T)` and the shim reallocs exactly. The inlined workload `main` is 30.3% and the rest is startup/console. | Follow-up issue #33 and plan `growable-array-growth` |

### Geometric-growth probe

The probe patches only the emitted `cl_buffer_reserve` in `build/tmp` so the
capacity doubles from a floor of eight bytes:

| Metric | Shipped | Probe | Delta |
|---|---:|---:|---:|
| Net m68k instructions | 223,504 | 56,814 | **-74.6%** |
| m68k `.text` | 24,092 B | 24,108 B | **+16 B (+0.07%)** |
| Output | `container: 10475520 1024` | same | exact |

The `.text` comparison is hosted-amiga against hosted-amiga, with the same
shipped flags. The +16 B is far inside the 1% `.text` guard (about 241 B); the
earlier `-4.2%` figure compared a hosted-amiga baseline with a host-shim
analysis binary and is discarded.

The probe is not committed. Its emitted-C diff is one 29-line hunk in
`cl_buffer_reserve`; no other function changed. The follow-up plan owns the
shim change, its tests, and the full gate.

### Decision

- **Compiler-emitted shapes: no-op with evidence.** The text scanner's cost is
  the parsing work itself. The shipped `-O2` build folds the classification
  loop into a `dbf` pointer loop; the digit loop retains two bound compares per
  digit, an estimated 8,300 instructions or about 2.2% of the net text-scan
  count. Counting the classification loop bound gives a ~4–6% upper bound;
  that residual is still too small to justify a range-analysis compiler plan.
  The container's generic calls and struct copies disappear under inlining. No
  emitted-C change is justified by this audit.
- **Runtime shim: follow-up.** The dominant real-workload cost is the
  exact-size growable-array reserve. Issue #33 and
  `plans/growable-array-growth/99-execution-plan.md` carry the measured probe
  and the acceptance criteria for the change.

### Gate-unblocking defect DEF-559

The first full `make verify` run failed in `bench-shape`, not in the audit
items: vbcc warning 68 reported that the emitted C redeclared
`UnlockPubScreen` with a different type. The hosted-amiga shim declared
`(struct Screen *screen, struct Screen *pub)` while the NDK declares
`(CONST_STRPTR name, struct Screen *screen)`. The calls already passed
`(NULL, public_screen)`, so only the prototype was wrong.

The defect predates the audit (`6a503fbb`) and reproduces with the base
compiler. It is fixed by matching the NDK signature and correcting the
comment; the calls and runtime behavior do not change. DEF-559 records the
defect in the roadmap, and the one-line shim edit receives the Amiga platform
review.

### A500 capture determinism

The original `sdk-a500-smoke` cropped a fixed `430x22+160+88` region from a
host-scaled FS-UAE window, so the gate depended on the desktop scale and
missed the hello line here (AE=3994). The gate now pins the FS-UAE window to
640x256 with no border, crops the hello line at `215x8+169+44`, and upscales it
2x to the 430x16 golden. FS-UAE's default aspect correction centers the hires
screen in that window, so the offsets are valid for the pinned window and
aspect default, not native framebuffer coordinates. `make sdk-a500-smoke`
passes on the private Xvfb, and the full `make verify` is green. The Amiga platform review's SR-001 noted the
aspect-corrected rendering; the comment is corrected and the gate now checks
that the capture is exactly 640x256 before comparing. The sibling unpinned
A500 gates are follow-up #34.

## Evidence log

| Evidence | Result |
|---|---|
| Baseline `make check` before the fixtures | exit 0 |
| Fixture commit | `d44c62fd` — `test(bench): add real text-scan and container workloads`; both the head and base measurements use this compiler source |
| Follow-up issue | #33 — `[performance] Amortize growable-array growth` |
| Fixture test red | `AssertionError: 'text-scan' not found in ['local-table', 'lookup', 'matrix', 'moves', 'sieve', 'strings']` |
| Fixture test green | `Ran 1 test ... OK` |
| Fixture `make check` | exit 0; `make bench-test` ran 103 tests (46 in `test_bench.py`, including the fixture test; one new flag-audit set test); the setup installer's separate 29-test suite is not the bench count |
| Host strict build and run | both programs build with `-Wall -Wextra -Werror` and print the expected output |
| Hosted-amiga debug build and run | both programs build without warnings and print the expected output under `vamos` |
| Shipped harness measurement | `make bench` exit 0; `build/perf/head.json` records the table above with calibration 2,928 instructions |
| Golden hashes | `head.json` `output_sha256` equals the committed `expected-output.txt` hashes for both workloads |
| Base/head guard | `make bench-base COMMIT=d44c62fd` and `make bench-compare` exit 0; all eight scored rows report +0.0% net instructions and +0.0% `.text`; the only C-byte change is the prototype declaration (-0.0%, advisory) |
| Attribution | Hosted-amiga traces (226,432 and 378,606 instructions) mapped through `m68k-amigaos-nm` at load base 0x260c; forced-noinline hosted-amiga analysis builds for the function shares; loop disassembly for the guard counts |
| Shipped bounds-check check | `text-scan` `_main` disassembly: token classification at `_main+0x94` is a `dbf` pointer loop with one pointer-bound compare; digit parsing at `_main+0x20a` retains two bound compares per digit, estimated 8,300 instructions (~2.2%; ~4–6% with the classify loop bound) |
| Shipped container check | `container` `_main` disassembly: the scan loop at `_main+0x74` reads `x`/`y` directly; the fill loop at `_main+0x1c` contains the inlined exact-size reserve with `realloc` and `memset` |
| Correctness review | Not clean initially: RV-001 (`.text` profile mix), RV-002 (premature roadmap completion), RV-003 (mislabeled text trace), RV-004 (flag-audit scored set), RV-005 (fixture test tautology), RV-006 (test count), RV-007 (follow-up test duplication), RV-008 (README inlining), RV-009 (issue #33 allocator table). All fixed; re-review clean, no critical/major remains |
| Performance review | Not clean initially: PE-001 (probe `.text` profile), PE-002 (0x260c load base and 58.0% allocator share), PE-003 (analysis builds were host-shim), PE-004 (digit-loop residual), PE-005 (flag-audit subset), PE-006 (premature roadmap), PE-007..PE-013 (calibration label, test count, fixture test, README inlining, guard counts, baseline list, reasoning). All fixed; re-review clean, no critical/major remains |
| Amiga platform review | PASS, no findings; the reviewer confirmed the NDK signature, register order (a0 name, a1 screen), the 1.3/2.0 guard, and identical codegen. It noted the V37 public-screen floor is stricter than the V36 header introduction; the project baseline already states V37, so no change |
| Full gate, first run | failed at `bench-shape` on pre-existing DEF-559; reproduced with the base compiler |
| Full gate | `make check`, `make bench-shape`, and `make ubsan` (12 suites) green; after the deterministic A500 capture fix, `make verify` reports `verify: all legs passed` (exit 0). `make sdk-a500-smoke` with the final 640x256 size assertion also exits 0 |
| A500 capture commit | `084d91fc` — `test(amiga): make the A500 smoke capture deterministic`; sibling unpinned A500 gates are follow-up #34 |
| Flag audit | `SCORED` now covers all eight benchmarks; `make bench-flags` passes the new `container`, `text-scan`, and `local-table` rows but fails the pre-existing `host/lookup` binary floor (`30,912 > 34,880 × 85%`); the floor drift predates the audit and is not a scored-benchmark regression |

## Review findings

| Review | Verdict | Findings |
|---|---|---|
| Performance (independent) | Not clean initially; re-review clean | Re-review: no critical/major remains; only the documented FS-UAE environment blocker. PE-001 fixed: the `.text` probe was re-measured hosted-amiga against hosted-amiga at 24,092 → 24,108 B (+16 B, +0.07%), not -4.2%. PE-002 fixed: load base 0x260c and allocator share 58.0%. PE-003 fixed: traces and noinline builds regenerated from hosted-amiga C (226,432 and 378,606; tagged 372,902 and 479,470). PE-004 fixed: the digit loop retains two bound compares; the ~2.2% residual is recorded and the no-op decision stands. PE-005 fixed: `flag_audit.SCORED` is derived from `bench.benchmark_names()` with a test; the new rows pass and the pre-existing `host/lookup` floor is recorded. PE-006 fixed: the roadmap is back to Executing with no premature Done/closed/green claims. PE-007..PE-013 fixed in the plan, tests, and READMEs |
| Correctness | Not clean initially; re-review clean | Re-review: no critical/major remains; only the documented FS-UAE blocker. RV-001 (the same `.text` mix as PE-001) fixed to +16 B. RV-002 fixed: PLAN-39/DEF-559 are Executing and the header/pending claims match disk. RV-003 fixed: the analysis is now hosted-amiga; text trace 378,606. RV-004 fixed: `SCORED` covers all eight and the test guards it. RV-005 fixed: the fixture test iterates the directory tree and catches a deleted older source (red run with `sieve/program.clo` removed). RV-006 fixed: 103 `make bench-test` tests / 46 in `test_bench.py`. RV-007 fixed: the follow-up reuses `stdlib_impl_test.rs` and the existing growth loop. RV-008 fixed: the READMEs say the source copies by value and the shipped `-O2` build inlines it. RV-009 fixed: the live issue #33 allocator table now reads 15.5/15.4/13.9/13.1, total 58.0% |
| Amiga platform (DEF-559) | PASS | No findings. The NDK signature, m68k register order, 1.3/2.0 guard, and pre-fix/post-fix codegen were verified; the V37 public-screen floor is the existing project baseline |
| Amiga platform (A500 capture) | PASS after one minor | SR-001: the pinned window holds an aspect-corrected 328x256 rendering, not native framebuffer pixels; the comment is corrected and the gate now asserts the capture is 640x256. The geometry is deterministic on the pinned FS-UAE build. The sibling unpinned A500 gates are follow-up #34 |
