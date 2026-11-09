# Testing Strategy: Nested Fill Compaction (PLAN-22)

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Test Split

| Suite | Kind | Purpose |
|---|---|---|
| `compiler/tests/nested_fill_compaction_spec_test.rs` | Specification | Behaviour oracle: every case prints the required result. It passes on the base compiler and must keep passing. |
| `compiler/tests/nested_fill_compaction_impl_test.rs` | Implementation | Emitted-shape pins: one `memset`, no `__cl_fillrow`, the size expression, the threshold, every exclusion. Positive pins are red on the base. |
| `tools/perf/tests/test_bench.py` | Tooling unit | The parameterized comparison gate. |

The specification suite derives from requirements R3 and R4. It must never be
edited to match the implementation. The implementation suite pins the shape
and moves only when the intended emission changes.

## Specification Cases

Each case builds one generated program, runs it on the host, and compares the
standard output with the expected text recorded from a run on the unmodified
compiler at `b68dae5`. The suite spawns only `CARGO_BIN_EXE_clc`; it does not
read the gitignored `build/perf/base-clc`.

| ID | Case | Covered slots | Expected |
|---|---|---:|---|
| ST-01 | `let a: i32[8][8] = [; [; 0]];` read every slot | 64 | all zero |
| ST-02 | `let a: i32[4][4][4] = [; [; [; 0]]];` read every slot | 64 | all zero |
| ST-03 | `let a: i32[16][8] = [8 prefix rows; [; 0]];` read every slot | 64 | prefix rows set, tail zero |
| ST-04 | `let a: i32[8][9] = [; [; 7]];` read every slot | 72 | all 7 (excluded path) |
| ST-05 | `let a: u8[16][16] = [; [; 0]];` sum + one write/read | 256 | zero, then the written byte |
| ST-06 | `let a: bool[8][8] = [; [; false]];` count true | 64 | zero |
| ST-08 | An `i32[8][9]` parameter filled with a nested zero | 72 | all zero |
| ST-09 | A function returning `i32[8][9]` with a nested zero literal | 72 | all zero in the caller |
| ST-10 | An address-taken `i32[8][9]` local with a nested zero fill | 72 | all zero through a raw pointer |
| ST-11 | A struct row (`struct R { v: i32[4]; }`) `R[8][9]` with a nested fill | 72 outer / 288 leaf | excluded path, correct values; the threshold does not apply to struct rows |
| ST-12 | `i32[7][9]` (63 slots) nested zero fill | 63 | all zero; below-threshold control |
| ST-13 | `i32[8][8]` (64 slots) nested zero fill | 64 | all zero |
| ST-14 | A full prefix (`i32[8][8]` with 8 explicit rows and a zero fill) | 0 covered | prefix values; no temporary, no marker |
| ST-15 | `i32[4][4]` (16 slots) nested zero fill | 16 | all zero; below-threshold control |
| ST-16 | `i32[8][8]` with an `i32(0)` nested fill | 64 | all zero; integer-cast zero |
| ST-17 | A conditional return arm with a zero-fill `i32[8][8]` literal | 64 | all zero; hidden-return arm |

The small cases (ST-12, ST-15) are controls. They prove the old spelling still
produces correct values; they are not compaction expectations. ST-07 (volatile
target) is not a behaviour case: the base compiler's per-row `memcpy` spelling
discards the volatile qualifier and is rejected by a strict host build. That is
a pre-existing defect outside this change, so IP-11 pins the emission only and
the ID is reserved, not reused.

## Implementation Pins

Positive pins are red on the base compiler. Exclusion guards are green on both
and fail if the implementation wrongly compacts the excluded shape. Every
exclusion guard uses at least 72 slots and records the same-size positive
control IP-21, so a threshold move cannot explain the guard. The boundary
itself is pinned by IP-05/IP-06.

| ID | Kind | Shape | Pin |
|---|---|---|---|
| IP-01 | Positive | `i32[8][8] = [; [; 0]]` | exactly one `memset`, no `__cl_fillrow` |
| IP-02 | Positive | same | `memset` target starts at `&(a[0])`, count is `8 * sizeof(a[0])` or the folded equivalent |
| IP-03 | Positive | `i32[4][4][4] = [; [; [; 0]]]` | exactly one `memset`, no row temporary |
| IP-04 | Positive | `i32[16][8]` with 8 prefix rows and a zero fill | one `memset` starting at `&(a[8])`, count `8 * sizeof(a[8])` |
| IP-05 | Boundary | `i32[7][9]` (63 slots) | no whole-array `memset`; old row spelling |
| IP-06 | Boundary | `i32[8][8]` (64 slots) | one `memset` |
| IP-07 | Exclusion | non-zero `i32[8][9] = [; [; 7]]` | no whole-array `memset`; row spelling |
| IP-08 | Exclusion | `f32[8][9] = [; [; 0.0]]` | no whole-array `memset`; row spelling |
| IP-09 | Exclusion | `R[8][9]` struct row | no whole-array `memset`; row spelling |
| IP-10 | Exclusion | `i32[16][8] = [; [1,2,3,4,5,6,7,8]]` (row literal with explicit elements) | no whole-array `memset`; row spelling |
| IP-11 | Exclusion (emission only) | function-body assignment to a volatile global `i32[8][9]` | no whole-array `memset`; row spelling pinned without a build because the old spelling is invalid C |
| IP-12 | Exclusion | struct field `i32[8][9]` | no whole-array `memset`; row spelling |
| IP-13 | Regression | one-extent `u8[8192] = [; 1]` | unchanged single `memset` |
| IP-14 | Regression | one-extent `i32[100] = [; 0]` | unchanged loop compaction |
| IP-15 | Positive | parameter `i32[8][9]` | `memset` target `&(a[0])`, count from `element(0).bytes`, not `target.bytes` |
| IP-16 | Positive | hidden return slot `i32[8][9]` | `memset` targets the slot; no `__cl_fillrow` |
| IP-17 | Boundary | full prefix, 0 covered rows | no `__cl_fillrow`, no `if (sizeof(...))` marker |
| IP-18 | Unit (`function.rs` `#[cfg(test)]`) | synthetic `Ty` with a missing extent; `usize` overflow; `covered_rows == 0` | helper rejects missing/overflow; accepts zero covered |
| IP-19 | Positive | conditional return arm with a zero-fill `i32[8][8]` | one `memset` in the selected arm |
| IP-20 | Positive | `i32[8][8]` with an `i32(0)` fill | one `memset` |
| IP-21 | Control | zero-fill `i32[8][9]` (72 slots) | one `memset`; the same-size control for IP-07 |
| IP-22 | Build | nested `bool[8][9]` zero fill in `shapes.clo` | m68k, vbcc, and freestanding build without a warning |
| IP-23 | Exclusion (emission only) | `i32[8][9] = [; [; i32(f32(0.0))]]` | no whole-array `memset`; row spelling; the float-to-integer cast is not a constant zero |
| IP-24 | Boundary/positive | field-target full prefix `i32[8][8]` | no `__cl_fillrow`, no marker, no memset; the zero-covered return is outside the `compact` flag |

A pin that says "one `memset`" counts the whole-array fill statement, not the
runtime shim's own `memset` calls in the same translation unit.

The `compact` flag is inherited from the existing gate. `EnvField` and `Temp`
targets are allowed by that gate and name real storage; no separate fixture
exists for a nested fill in those positions. The address-taken case (ST-10)
and the parameter/return cases cover the real-storage forms. A hidden-return
conditional arm passes `compact=true`; that arm is a real return slot and is
pinned by IP-19.

## Red/Green Record

1. Write both suites; do not change the compiler.
2. Run `cargo test --test nested_fill_compaction_impl_test` in the clean
   clone. Record every failing positive pin and boundary pin, and the
   observed old spelling. The exclusion guards must pass.
3. Run `cargo test --test nested_fill_compaction_spec_test`. It must pass on
   the base; a failure means the test or the expected output is wrong.
4. Implement the change. Both suites pass.
5. Test integrity: disable the compaction condition and confirm every positive
   pin fails; restore. Remove each exclusion in turn (non-zero, float, struct,
   explicit row, volatile, field, threshold) and confirm its guard fails;
   restore. Confirm the `covered_rows == 0` unit case still selects the early
   return.
6. Record both runs in `05-findings.md`.

The red run is recorded before the implementation. The tests and the
implementation land in one commit so every commit stays green.

## Tooling Tests

`tools/perf/tests/test_bench.py` gains the cases listed in
[Comparison Gate](03-01-comparison-gate.md). The retired
`test_compare_fails_on_c_byte_target` is replaced by an invariant case. Run
with `make bench-test`.

## Sanitizers

The behaviour suite builds generated programs. It carries the `RDC_ASAN` and
`RDC_UBSAN` discovery markers. Its builder adds `-fsanitize=address` when
`RDC_ASAN` is set and
`-fsanitize=undefined -fno-sanitize-recover=all` when `RDC_UBSAN` is set,
following `compiler_robustness_spec_test.rs`. `make asan` and `make ubsan`
discover and run it. The shape owns and releases no memory, but the suite
proves the compaction does not corrupt adjacent storage.

## Strict-Shape Fixture

`tools/perf/fixtures/shapes.clo` gains a nested zero fill above 64 slots, a
nested `bool` zero fill, a prefix case, an array-parameter case, and a
hidden-return case.
`make bench-shape` builds it with m68k GCC, vbcc, and the freestanding profile.
It is a build-only leg. The freestanding runtime corpus is covered by
`freestanding_spec_test.rs` inside `make check`.

## Emission Audit

The emission-integrity auditor emits with `build/perf/base-clc` and the head
`clc` for:

- all seven programs under `spec/tests/corpus/` (four portable and three
  hosted-only),
- `benchmarks/*/program.clo` (four scored plus calibration),
- `examples/amiga/*.clo`.

Every diff must be a nested zero fill replaced by one `memset`. Any diff
outside that class blocks the closeout. The audit runs on the codegen commit,
not the tooling commit.

## Reproducible Audit Commands

These commands run from the repository root. Outputs stay under
`build/perf/audit/`, which is gitignored and is removed or retained only as a
scratch artifact.

### Emit and trace one benchmark

```bash
mkdir -p build/perf/audit
./build/perf/base-clc --profile hosted-amiga -o build/perf/audit/sieve.c benchmarks/sieve/program.clo
make -C docker exec CMD="m68k-amigaos-gcc -std=c99 -mcrt=nix20 -m68000 -fwrapv -O2 -o build/perf/audit/sieve build/perf/audit/sieve.c -lamiga -lm"
make -C docker exec CMD="vamos --instr-trace build/perf/audit/sieve 2>&1" > build/perf/audit/sieve.trace
grep -c "instr:" build/perf/audit/sieve.trace
```

### Bucket trace PCs

A temporary host script under `build/perf/audit/bucket.py` reads
`build/perf/audit/<name>.trace`, extracts the PC column with
`instr:.*?\s([0-9a-fA-F]{6,8})\s`, and counts the most frequent PCs. The Phase
1 record includes the top-PC table and the total. The script is scratch, not a
committed tool.

### Matrix probe

```bash
./build/perf/base-clc --profile hosted-amiga -o build/perf/audit/matrix-base.c benchmarks/matrix/program.clo
python3 - <<'PY'
src = open('build/perf/audit/matrix-base.c').read()
start = src.index('    i32 __cl_fillrow_1[32] = {0};')
last = src.rindex('&(__cl_fillrow_1)')
end = src.index('\n', last) + 1
open('build/perf/audit/matrix-probe.c', 'w').write(
    src[:start] + '    memset(&(a[0]), 0, sizeof(a));\n' + src[end:]
)
PY
make -C docker exec CMD="m68k-amigaos-gcc -std=c99 -mcrt=nix20 -m68000 -fwrapv -O2 -o build/perf/audit/matrix-probe build/perf/audit/matrix-probe.c -lamiga -lm"
make -C docker exec CMD="vamos --instr-trace build/perf/audit/matrix-probe 2>&1" > build/perf/audit/matrix-probe.trace
grep -c "instr:" build/perf/audit/matrix-probe.trace
make -C docker exec CMD="m68k-amigaos-size build/perf/audit/matrix-probe"
```

The base matrix binary and its trace are produced the same way without the
patch. Record raw count, net count (raw minus calibration), `.text`, C bytes,
and output for both.

### Emission audit

```bash
mkdir -p build/perf/audit/base build/perf/audit/head
for f in spec/tests/corpus/*/program.clo benchmarks/*/program.clo examples/amiga/*.clo; do
  name=$(echo "$f" | tr '/' '_')
  ./build/perf/base-clc --profile hosted-amiga -o "build/perf/audit/base/$name.c" "$f"
  ./target/debug/clc --profile hosted-amiga -o "build/perf/audit/head/$name.c" "$f"
done
diff -ru build/perf/audit/base build/perf/audit/head > build/perf/audit/emission.diff || true
```

A source that needs a different profile (for example a host-only program) uses
that profile on both sides. Every hunk is classified in `05-findings.md`; an
unclassified hunk blocks closeout.

## Gate Commands

| Gate | Command |
|---|---|
| Focused | `cargo test --test nested_fill_compaction_spec_test --test nested_fill_compaction_impl_test` |
| Fast gate | `make check` |
| Sanitizers | `make asan`, `make ubsan` |
| Corpus | `make corpus-amiga`, `make differential` |
| Strict C shapes | `make bench-shape` |
| Benchmark | `make bench`, `make bench-compare REQUIRE="<final requirements>"` |
| Full acceptance | `make verify` |

A gate result is recorded with the exact command and its final line. A failed
gate blocks the commit and the push.
