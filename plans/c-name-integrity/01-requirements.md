# Requirements: C Name Integrity

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

Close theme issue #42. Every accepted program builds on every compile-matrix leg whatever its
source names are, generated symbols never meet user symbols, every line of emitted C is attributed
to a real source or runtime line, and a driver build of a project keeps the user's absolute paths
out of the C. Scope per AR-1..AR-4, AR-10, AR-15.

## Functional Requirements

### Must Have
- [ ] R1 — A source name in any of the six positions (locals; parameters of functions, lambdas,
  prototypes, and function types; struct fields; closure-capture fields; enum constants) that is
  in an escape class is emitted with the AR-18 spelling; every other name keeps its source
  spelling (AR-2, AR-5, AR-15..AR-18).
- [ ] R2 — Probe 340 builds and prints 121, probe 341 prints 120, on host, hosted-amiga, and
  hosted-amiga-13 (#61 acceptance).
- [ ] R3 — The class-3 names are one committed list covering every leg and toolchain of AR-19, and
  a test proves it complete for every leg available (AR-7, AR-19).
- [ ] R4 — Generated names (`__cl_*`, `__env`, `__cl_ret`, helpers) keep their spelling (AR-17).
- [ ] R5 — A user function or global whose symbol equals or enters a generated family is renamed
  with the existing `__user` suffix; an instance of a user generic that meets a helper family is
  renamed the same way; non-colliding symbols are unchanged (AR-8, AR-20).
- [ ] R6 — Probe 342 builds and prints `1 102` on the three profiles (#64 acceptance).
- [ ] R7 — Every top-level C definition after the first `#line`, and every appended runtime file,
  is preceded by its own `#line` (AR-9, AR-21).
- [ ] R8 — `clorraine build|run|emit` passes each source inside its working folder by its relative
  path with `/` separators; `clc` keeps the command-line spelling (AR-10, AR-24).
- [ ] R9 — Docs and notices updated (AR-14, AR-22, AR-23).

### Won't Have (Out of Scope)
- The AR-4 items: locals spelled like mangled or generated function/object symbols; E11101 for
  same-named private generics in two modules; the driver's first-error line; the `Cl<Type>`
  spelling of user aggregates.
- A new instance prefix or any `spec/` or spec-test change (AR-8, AR-10).
- `-fdebug-prefix-map` (AR-22).

## Technical Requirements

### Performance
- `make bench-compile` within its existing guard; list lookup by binary search (AR-26).

### Compatibility
- Emission stays byte-deterministic and independent of the build machine and OS (spec/17 §11).
- The existing `#line` spec pins (`stage_spec_test`, `windows_cli_spec_test`) keep passing.

### Security
- No user or repository text is interpolated into a shell; the driver rewrite reads only paths
  (AR-24). Absolute home paths stay out of driver-built C (AR-10); the debug-info residual is
  documented (AR-22).

## Scope Decisions

| Decision | Options Considered | Chosen | Rationale | AR Ref |
| -------- | ------------------ | ------ | --------- | ------ |
| Positions | locals+params / all six | all six | the goal says "every" | AR-2, AR-15 |
| Generated families | instances / plus helpers | plus helpers | same collision | AR-3, AR-20 |
| Path fix | driver / clc + spec tests / docs / defer | driver | user choice | AR-10 |

> **Traceability:** Every scope decision references the Ambiguity Register entry that resolved it.

## Acceptance Criteria

1. [ ] ST-1..ST-24 pass (07).
2. [ ] The completeness test passes in the SDK image and with `CLORRAINE_RC24_ROOT` set (AR-19).
3. [ ] The final gate of AR-12 passes.
4. [ ] `docs/known-limitations.md` and `THIRD_PARTY_NOTICES.md` updated (AR-14, AR-23).
5. [ ] No spec chapter or spec test changed.
