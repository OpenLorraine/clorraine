# WS8 — Test integrity and defect history

> **Baseline**: e24a8752b08827c3a5d7c77b6e2827dac708b629 (see [WS0-baseline.md](WS0-baseline.md))
> **Checklist**: 03-03-runtime-performance-workstreams.md §WS8 (WS8-01..WS8-08)

## Scope and limits

Static analysis only: no test is run or edited (03-03 WS8-03). Inputs: the 157 Rust test
files (3,888 `#[test]` functions) under `compiler/tests/`, the fuzzer templates
(`tools/fuzz/`), the sanitizer runners (`make asan`, `make ubsan`), the FS-UAE gates,
the 582 DEF rows of `plans/00-roadmap.md`, and the WS1–WS7 findings. Classification is
by the committed schemes (probes 800, 810); every class used in a finding is read by
hand and the scheme's error rate on the read rows is reported.

## Dispatch record

| Role | Effort | Date |
|------|--------|------|
| Executing session (inline) | high | 2026-11-26 |

## Coverage table

| Group | Files | Status |
|-------|-------|--------|
| Schemes written (8.1.1) | `probes/WS8/800`, `810` | done |

## Checklist outcomes

_(filled in Step 8.2)_

## Findings

_(filled in Step 8.2)_

## Spec questions

_(filled in Step 8.2)_

## Falsifiability log

Red-check targets, chosen by reading before the classifiers run (task 8.1.1):

- 800 `value`: `amiga_ilbm_spec_test.rs::an_uncompressed_ilbm_decodes_every_pixel`
  (compares the decoded pixel report through `assert_stdout`).
- 800 `compiles`: `stage_spec_test.rs::lexical_literals_and_templates_are_accepted`
  (its only check is `assert_front_end_accepts`; the program returns 0).
- 810: DEF-13 (memory-safety) and DEF-22 (invalid-c).

_(results filled in task 8.1.2)_

## Escalations

None yet.

## Probe index

| Item | Probes |
|------|--------|
| WS8-03, WS8-06 assertion strength, pinned text | 800 |
| WS8-07 defect history | 810 |
