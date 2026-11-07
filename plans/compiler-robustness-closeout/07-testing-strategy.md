# Testing Strategy: Compiler Robustness Closeout

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Test Split

| Layer | File | Derives from |
|---|---|---|
| Behavior oracle | `compiler/tests/compiler_robustness_spec_test.rs` | R1–R5, R7.7 |
| Emission and C-shape pins | `compiler/tests/compiler_robustness_impl_test.rs` | R1–R5 |
| UBSan corpus | `compiler/tests/ubsan_corpus_spec_test.rs` | R6.4 |
| Fuzzer unit tests | `tools/fuzz/tests/test_fuzz.py` | R7 |
| Existing PLAN-14 pins | `generated_c_optimization_{spec,impl}_test.rs` | updated only per R8.3 |

The spec suite states observable behavior: stdout, exit, fault line, and
sanitizer cleanliness. The impl suite states emitted C shapes: the presence and
count of casts, guards, macros, and folder-derived values. A shape pin that
changes for an unintended reason is a failure.

## Defect Evidence Matrix

| Defect | Red-first case | Red evidence | Green evidence |
|---|---|---|---|
| D1 bounds | `should_guard_a_runtime_index_in_a_pure_fill` | ASan stack-buffer-overflow, `a: 0` | `error E11211`, ASan clean |
| D1 division | `should_trap_on_a_runtime_zero_divisor_in_a_pure_fill` | `SIGFPE` | `error E11210` |
| D1 float | `should_trap_on_an_out_of_range_float_fill` | `f: 44` | `error E11212` |
| D1 struct | `should_guard_a_struct_literal_array_field` | ASan overflow, `s: 0` | `error E11211`, ASan clean |
| D1 conditional | `should_keep_the_scalar_conditional_fill_guard` | control: the arm is already guarded inline; the condition guard runs once |
| D1 conditional struct | `should_guard_a_conditional_struct_array_field` | unguarded arm | guard on the arm |
| D1 module conditional | `should_guard_a_module_conditional_struct_array_field` | unguarded arm | guard on the arm |
| D1 module count | `should_emit_one_fill_guard_for_a_module_array` | five guards for four elements | one guard plus prefix guards |
| D1 prefix count | `should_emit_one_guard_per_prefix_element_and_fill` | duplicated prefix guard | one per prefix + one fill |
| D1 flag restore | `should_restore_the_fill_guard_flag_after_a_module_fill` | a later statement fill skips its guard | both guards present |
| D2 values | `should_wrap_narrow_arithmetic_in_inline_and_const_positions` | `300 400 -1 256 65537` | `44 144 255 0 1` |
| D2 16-bit | `should_wrap_a_16_bit_multiply_and_shift` | `u16*u16` build fails; `u16<<15` is 2147450880 | `1`, `32768`; UBSan clean |
| D2 operators | `should_agree_with_the_folder_for_the_other_operators` | inline/folder difference if any | agreement for `>> / % & | ^ ~` |
| D2 32-bit control | `should_keep_the_32_bit_controls` | control passes now | unchanged |
| D4 const | `should_build_a_narrow_const_under_the_strict_flags` | `-Werror=overflow` | strict build, reads 44 |
| D4 local/fill/field | `should_build_a_narrow_local_a_fill_and_a_field` | `-Werror=overflow` | strict build, reads 44 |
| D3 shift divisor | `should_keep_an_inline_shift_divisor_out_of_the_zero_trap` | base `0ac87d3e` traps | prints 0 |
| D5 cast | `should_round_an_f32_cast_in_a_constant_context` | `117 116` | `116 116` |
| D5 declared const | `should_round_a_declared_f32_const_reference` | `117 116` | `116 116` |
| D5 per-op | `should_round_each_f32_operation` (unit) | `1` at f64 | `0` at f32 |
| D5 NaN | `should_reject_a_constant_nan_float_conversion` | `#define E_A 0` | `E10202` registered message |
| D5 range | `should_reject_a_constant_out_of_range_float_conversion` | wrong extent | `E10202` registered message |
| D6 explicit | `should_fold_an_explicit_enum_member_expression` | `E_A 0` | `E_A 5` |
| D6 overflow | `should_reject_an_enum_auto_increment_that_does_not_fit` | accepted with `E_B 256` | `E10202` |
| D6 in-range | `should_keep_an_in_range_enum_auto_increment` | control | `A=254, B=255` |
| D6 store | `should_store_an_enum_member_in_its_enum` | `-Werror=overflow` with 256 | compiles, reads 255 |
| D6 cast | `should_emit_an_enum_conversion_as_a_cast` | C build error | `(ClDirection)(2)` |
| D6 runtime cast | `should_convert_a_runtime_value_to_an_enum` | C build error | cast from a runtime value |
| D6 handle cast | `should_emit_a_handle_conversion_as_a_cast` | C build error | `(ClBuffer)(0)` |
| D6 constant cast | `should_fold_an_enum_conversion_in_a_constant_context` | `E10224` | folds |
| D6 shadow | `should_keep_a_shadowing_binding_a_call` | `E(2)` steals a parameter | call path unchanged |
| D6 cross-module | `should_agree_on_a_cross_module_enum_member` | folder/emitter split | equal values |

## Test-Integrity Procedure

For each implementation task, before committing:

1. Run the focused spec and impl cases and record the red output in
   `05-findings.md`.
2. Apply the fix.
3. Re-run the same cases and the affected existing suites.
4. Revert only the production change in a scratch copy (or use the recorded
   base compiler) and confirm the same assertion fails again.
5. Restore the fix and run `make check`.

D3 has no production change. Its red evidence is the recorded
`0ac87d3e` trap and its green evidence is the head value. The test is still
required to fail if the evaluator regresses. The spec helper accepts
`RDC_CLC` so the case can be run against the base binary.

## Sanitizer Evidence

| Leg | What it proves |
|---|---|
| `RDC_ASAN` on the robustness suite | D1 never reads outside the source; no leak or double free in the new cases |
| `RDC_UBSAN` on the robustness suite | D2's 16-bit arithmetic is defined; no shift or conversion UB |
| `ubsan_corpus_spec_test` | the portable corpus has no UB on its normal paths |
| `make ubsan` activation self-test | the sanitizer is actually linked in the runner |
| suite-level ASan/UBSan probes | the suite's child builds are actually sanitized |
| fuzzer ASan/UBSan | generated programs stay clean across the matrix |

The robustness suite's build helper:

```text
flags = base host flags
if RDC_ASAN:   flags += -fsanitize=address -no-pie
if RDC_UBSAN:  flags += -fsanitize=undefined -fno-sanitize-recover=all -no-pie
```

A case that expects the compiler's own fault line asserts that line and, when
a sanitizer is active, that no sanitizer report appears.

## Fuzzer Evidence

| Level | Work |
|---|---|
| Unit | determinism across processes, template acceptance, seed/golden completeness, seed safety scan, normalization, classifier, argument-list commands, private cwd/core |
| Smoke | all 12 seeds on host + sanitizer + m68k, generated 0–15 host, 0–3 sanitizer and m68k |
| Smoke-host | same host and sanitizer work, no m68k |
| Full | 256 generated seeds on host and sanitizer, every eighth on m68k |

The fuzzer's failures are deterministic: the same seed and mode produce the
same case list and the same retained artifacts. A failure is not retried.

## Existing Suites

| Suite | Effect |
|---|---|
| `generated_c_optimization_spec_test` | the narrow value becomes 44; the enum value becomes 255 with an in-range enum; a new compile-fail case covers E10202 |
| `generated_c_optimization_impl_test` | the narrow-const place pin stays; the enum place pin uses the in-range enum; the `fill-shift` pin gains the wrap cast |
| `codegen_run_spec_test` | unchanged; it must stay green because the wrap changes emitted arithmetic |
| `stage_spec_test` | unchanged; the enum cast now lowers as a cast |
| `run_fault_spec_test` | unchanged; the frozen `spec/tests/run` set is not edited |
| `corpus_spec_test` | unchanged; emitted C must still match the recorded output |

## Gate Matrix

| Command | Required result |
|---|---|
| `make check` | green; includes `fuzz-test` and `fuzz-smoke` |
| `make asan` | 27 suites (the robustness spec suite joins the 26) |
| `make ubsan` | 2 suites plus the activation and suite-level controls |
| `make corpus-amiga` | all portable programs match |
| `make differential` | 3/3 match |
| `make bench-shape` | GCC `nix20`, GCC `nix13`, vbcc, freestanding all clean; the host strict build is covered by the spec suites |
| `make verify` | prints `verify: all legs passed`, or names every blocked licensed-media leg and does not claim full verify |

## Determinism

- The compiler emits byte-identical C for the same source, profile, and flags;
  the emission audit checks the base/head diff is attributable.
- The fuzzer's generator uses SHA-256 decisions; a unit test compares two
  subprocesses with different `PYTHONHASHSEED` values byte for byte.
- The seed set is sorted by filename.
- No pass/fail decision uses wall-clock time. Timeouts only bound a hang.

## Requirement Coverage

| Requirement | Named evidence |
|---|---|
| R1.1–R1.6 | D1 matrix rows; impl counts in the function/init body |
| R2.1–R2.6 | D2/D4 rows; operator agreement; 32-bit controls; freestanding shape |
| R3.1–R3.3 | four positions + named const; `RDC_CLC` base run |
| R4.1–R4.7 | cast, declared const, per-op, NaN, range, finite, `-0.0`/infinity controls |
| R5.1–R5.7 | explicit, overflow reject, in-range control, store, enum/handle cast, constant cast, shadow, cross-module |
| R6.1–R6.7 | runner self-test, suite-level probes, corpus sweep, verify placement |
| R7.1–R7.13 | unit tests, seeds, smoke/smoke-host/full, classifier, safety, budget |
| R8.1–R8.6 | new suites, pin updates, shape gate, docs |
| R9.1–R9.6 | gate matrix, emission audit, fault parity, clean push, re-gate |

## Residual-Risk Checks

| Risk | Check |
|---|---|
| Guard duplication remains in a path | impl count per path (R1.6) |
| Narrow wrapping changes an unrelated expression | full `make check` + `make differential` + emission audit |
| `f32` rounding changes a finite constant's legal range | `E10202` negative cases plus the finite control |
| Enum overflow rejection changes an accepted program | compile-fail case, in-range controls, and the user ruling |
| Enum cast lowering misclassifies a function call | shadowing spec case |
| UBSan is silently disabled | runner self-test + suite-level control |
| Fuzzer writes into the repository | private-cwd unit test + ignored build path |
| Smoke exceeds the fast-gate budget | measured runtime in `05-findings.md`; `fuzz-smoke-host` fallback |
