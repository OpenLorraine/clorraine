# Testing Strategy: Direct Template Formatting

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

The oracle is the existing template behavior plus the allocation contract:
one result allocation for non-float templates, two for a float template, zero
for a literal-only template. The allocation count is observable by wrapping
the host allocator and reading a counter around the template expression.

## Specification Test Cases

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|---|---|---|
| ST-1 | `$"{a}-{b}"` with two integer placeholders; allocation delta around the `let` | delta 1 on head, 3 on the base | R1 / AR-01 |
| ST-2 | `let src = stringOf("abc"); let t = $"[{src}]"`; delta | delta 2 on head and base (the copy plus the result); `src` still usable after | R1, R3 |
| ST-3 | `let v: str = "abc"; let t = $"[{v}]"`; delta | delta 2 on head and base (the copy plus the result) | R1, R3 |
| ST-4 | `$"{true}"`; delta | delta 1 on head, 2 on base | R1 |
| ST-5 | `$"{1.5}"`; delta | delta 2 on both (the `%g` temp plus the result) | R1, AR-04 |
| ST-6 | `let v: str = $"plain"`; delta | 0 (a literal-only template is a view) | R2 |
| ST-7 | `let t: String = text(7)`; delta | 1, unchanged from the base | R2 |
| ST-8 | Signed/unsigned boundaries, narrow integers, `usize` max, adjacent placeholders, literal text, empty `str`, and a null view `str?` | byte-identical to the current output; the null view prints `[]` | R2 |
| ST-9 | `let s = stringOf("a"); let t = $"{s}"; s = stringOf("b"); println(t)` | `a` (the result is independent of the later source) | R3 / AR-03 |
| ST-10 | Two placeholder calls with a stateful counter, `$"{touch()}-{touch()}"` | `7-7 2` (each call evaluates exactly once; left-to-right order is not a language guarantee) | R2 |
| ST-11 | The 2,000-iteration `strings` loop | `strings: 0x3F9A6` | R2 |
| ST-12 | The ST-1/ST-8 programs under ASan with leak detection | exit 0, no report | R3 |
| ST-13 | `INT_MIN` and `UINT_MAX` under UBSan with recovery disabled | exit 0, no report | R3 |
| ST-14 | A loop with an owned `String` placeholder reused after the template | no leak/double free under ASan | R3 |
| ST-15 | `toDecimal(-42)` and `toHex(0x3F9A6)` | `-42`, `0x3F9A6` (the explicit converters are unchanged) | R2, R4 |

## Test Categories

| File | Cases | Notes |
|---|---|---|
| `compiler/tests/template_direct_formatting_spec_test.rs` | ST-1 – ST-15 | Host; `--wrap=malloc` for the deltas; ASan/UBSan markers |
| `compiler/tests/template_direct_formatting_impl_test.rs` | Emission | Host and Amiga: scalar parts, no `cl_text_i32/u32(` inside a template call, borrowed `String`/`str`, `cl_str_part_bool`/`f64`, literal-only `cl_str`, determinism |

The existing `string_integer_formatting_spec_test` and `_impl_test` remain the
converter oracle and must stay green.

## Red/Green

| Phase | Evidence |
|---|---|
| Red | ST-1 reports delta 3 on `00eb6bc`; ST-4 reports 2; the impl pins fail (no scalar parts). ST-2/ST-3 are no-change at 2. |
| No-change oracle | ST-2, ST-3, ST-5 – ST-8, ST-10, ST-11, ST-13, ST-15 pass on both |
| Green | ST-1 and ST-4 report delta 1; the impl pins pass; the null-view case passes |

## Verification Checklist

- [ ] Every ST case has a concrete input and expected output.
- [ ] ST-1 is red on the base.
- [ ] All new and existing suites pass.
- [ ] ASan and UBSan legs pass.
- [ ] `make bench-compare` passes with `strings` allocations near 2,002.
- [ ] `make check`, `make bench-shape`, `make verify` pass.
- [ ] The emission-integrity audit finds no program-body change.
