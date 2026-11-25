# WS1 — Semantic core

> **Baseline**: e24a8752b08827c3a5d7c77b6e2827dac708b629 (see [WS0-baseline.md](WS0-baseline.md))
> **Checklist**: 03-02-compiler-workstreams.md §WS1 (WS1-01..WS1-10)

## Scope and limits

Components: `lex`, `parse`, `ast`, `load`, `resolve`, `sema`, `types`, `consts`,
`analysis.rs`, `driver.rs`, `abi.rs`, `ffi.rs`, `profile.rs`. Lens: wrong acceptance and wrong
rejection, including a wrong diagnostic code; diagnostic wording is not assessed.

## Dispatch record

| Role | Effort | Date |
|------|--------|------|
| Executing session (inline) | high | 2026-11-25 |

## Coverage table

| Group | Files | Status |
|-------|-------|--------|
| Probes written (1.1.1) | `probes/WS1/001`–`065` | done |
| Red check (1.1.2) | all probes with a contrast | done |

## Checklist outcomes

_(filled in Step 1.2)_

## Findings

_(filled in Step 1.2)_

## Spec questions

_(filled in Step 1.2)_

## Falsifiability log

Red check run 2026-11-25 after the probes were committed (`4ab6b39a`; amended FFI
contrasts `c70f47c7`): `probe.py … --contrast` (log `build/tmp/assessment/WS1/contrast.log`,
`contrast-ffi.log`; results `build/tmp/assessment/WS1/probes/contrast.json`,
`probes-ffi/contrast.json`). Every contrast ran on every listed profile, debug and release
builds where applicable.

| Probe | Form | Red-check result |
|-------|------|------------------|
| 001, 002, 004–007, 009–011, 013, 014, 016–018, 020, 023–026, 029–031, 033–038, 044–046 | contrasting input: the offending token is replaced by a valid one; the reject probe's contrast must be accepted and print the stated line | all contrasts accepted and printed their expected output on host, hosted-amiga, hosted-amiga-13 (debug and release) |
| 003, 008, 012, 015, 019, 022, 027, 028, 032, 039–041 | contrasting input: a changed operand or literal whose expected output differs from the probe's | all contrasts printed their (different) expected output |
| 042, 043, 047 | contrasting input: an in-range operand; the fault probe's contrast must print a value instead of faulting | all printed the value |
| 048 | seeded defect: a field added to the Clorraine `struct C` only; the probe must then differ from its C companion | differed on host, hosted-amiga, hosted-amiga-13 (debug and release) |
| 050, 051, 052 | seeded defect: a user `ffi` declaration of libc `strlen` returning `u16`; the C build must fail | failed with `conflicting types for 'strlen'` on each profile. The original seed (a user `ffi` redeclaring a shim function) was rejected by `clc` with E11101 before C emission, so it could not reach the C compiler; amended in `c70f47c7` with the original kept |
| 053, 054, 055 | contrasting input: the same program shape with an available module or a plain string must be accepted on the same profile | all accepted |
| 057 | contrasting input inside the matrix: the same generated one-import program is expected `A` on some (module, profile) cells and `R` on others, so a gate that accepted or rejected everything would mismatch | form recorded; observed in task 1.2.3 |
| 065 | seeded defect: `--seeded` sets a 1 ms limit on the first case; the checker must report a failure | reported `FAIL parens-10k`, exit 1. The same run showed `clc` ending with signal 6 (`PANIC`) on that input; recorded for WS1-10 in task 1.2.3 |
| 021 | contrasting input: `pair(next() * 10, next())` must print `10 2` | **did not match**: printed `20 1` on every profile and build. The contrast does discriminate (its output differs from the probe's own expectation), and it is itself evidence that call arguments are not evaluated left to right; confirmed or refuted by the main run in task 1.2.1 |
| 058–064 | contrasting input: a valid `embed` form | **could not run**: every `embed` form, including the spec's own examples, is rejected (`E10224` for a module `const`, `E10213` for a local `const` or a module `let`). The path-validation detectors (059, 060, 062) can never be reached, so their checklist parts are a `limit`; the rejection itself is evidence for WS1-08, recorded in task 1.2.3 |

## Escalations

None.

## Probe index

Runner: `probes/WS1/probe.py` (expectation format in its docstring). Scripts:
`057-gating-matrix.py` (WS1-09), `065-robustness.py` (WS1-10).

| Item | Probes |
|------|--------|
| WS1-01 lexer | 001–018 |
| WS1-02 parser | 019–021 |
| WS1-03 resolution | 022–029 |
| WS1-04 types | 030–038 |
| WS1-05 constant evaluation | 039–047 |
| WS1-06 layout | 048 (+ `048-layout-sizeof.c` companion) |
| WS1-07 FFI | 050–052 |
| WS1-08 data inclusion | 058–064 |
| WS1-09 profile gating | 051–055, 057 |
| WS1-10 robustness | 065 |
