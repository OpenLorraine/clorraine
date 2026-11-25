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

## Checklist outcomes

_(filled in Step 1.2)_

## Findings

_(filled in Step 1.2)_

## Spec questions

_(filled in Step 1.2)_

## Falsifiability log

_(filled in task 1.1.2)_

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
