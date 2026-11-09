# Requirements: Generic Field Integrity

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

A generic struct declaration writes its fields in terms of its type parameters
(`struct Pair<T> { first: T; second: T; }`). When the struct is used as an
instance (`Pair<i32>`), every stage that inspects a field must replace `T` with
the instance's argument before using the field type. Today the substitution
happens in some paths (the field read fixed for DEF-05) but not in the type
table, the overlay walk, the ownership check, or the emitter. This plan makes the
substitution uniform through one shared helper.

This is a defect-remediation plan: it changes no language syntax, adds no
diagnostic, and edits no `spec/` file. It makes the implementation match the
frozen specification and the already-recorded expectations for DEF-06…DEF-10.

## Functional Requirements

### Must Have

- [x] R1 — A field read of an instantiated generic struct yields the field type with the instance's arguments substituted, in the checker, the ownership stage, and the emitter (DEF-05 follow-up; DEF-10). `AR-13`.
- [x] R2 — An overlay (volatile) type reached through a generic struct field is rejected by value with `E10207`, matching the non-generic case (DEF-06). `AR-9`.
- [x] R3 — A generic struct whose substituted field list contains a handle is move-only; a second use reports `E10020` (DEF-07). `AR-10`.
- [x] R4 — The emitter drops a generic struct's handle field on scope exit (DEF-08). `AR-11`.
- [x] R5 — A generic zero value (`Point {}`, `Pair<Pair<i32>> {}`) emits valid C and compiles (DEF-09). `AR-2`, `AR-12`.
- [x] R6 — `sizeof` and `offsetof` of a generic instance fold to the concrete size/offset (DEF-10). `AR-13`.
- [x] R7 — The substitution logic lives in exactly one shared place and every field-read site uses it (AR-3, AR-6). `AR-3`, `AR-6`.

### Should Have

- [x] R8 — The ownership stage's raw-pointer member access and the checker's field reads share the same substitution so a raw record's field is typed, not `Unknown` (DEF-10). `AR-13`.

### Won't Have (Out of Scope)

- No new language surface, syntax, or diagnostic.
- No edit to any file under `spec/` (goldens, chapters, grammar, registry) (AR-4).
- No new test harness, dependency, or generated artifact (AR-22).
- No change to the `freestanding`/`hosted-amiga` ABI or runtime shims.

## Technical Requirements

### Compatibility

- The workspace stays Rust **std-only**; the new helper uses the existing `BTreeMap` and `substitute` already present in `compiler/src/types/`.
- Emitted C for non-generic programs must be byte-for-byte unchanged.

### Correctness Invariants

- Instance arguments are zipped against the declaration's **own** parameter names (AR-14).
- An unbound parameter (empty/short argument list) is left unchanged rather than corrupted; this path is unreachable after `E10300`/`E10301` (AR-15).
- Existing recursion depth limits (32) are preserved.

### Diagnostics

- `E10020`, `E10207`, and `E10224` keep their `spec/14-diagnostics.md` wording; no message changes (AR-19).

### Security

- N/A: compiler type-checking and emission, with no external input, network, auth, or persisted-data surface (AR-17).

## Scope Decisions

| Decision | Options Considered | Chosen | Rationale | AR Ref |
| -------- | ------------------ | ------ | --------- | ------ |
| Defects covered | all five / majors / soundness+leak | all five DEF-06–DEF-10 | One root cause; fixing a subset leaves the same drift to recur | AR-1 |
| Empty literal | codegen-only / forbid / defer | codegen-only; keep `Point {}` valid | STR-7 defines it as the zero value; only the emitter was wrong | AR-2 |
| Duplication | shared helper / per-site | shared helper | The drift is the root cause; one owner prevents recurrence | AR-3 |
| Test placement | impl-only / also spec goldens / + runtime | regressions in existing suites, red-first | Honors the user's no-`spec/`-edit ruling while keeping tests-before-fix | AR-4 |
| `offset_of` input | `&Ty` / new method | `&Ty` | Removes the name-only variant that cannot see instance args | AR-7 |

> **Traceability:** Every scope decision references the Ambiguity Register entry (AR #) that resolved it. See `00-ambiguity-register.md`.

## Acceptance Criteria

1. [x] R1–R8 satisfied; DEF-06…DEF-10 no longer reproduce.
2. [x] Every field-read stage calls the shared helper; no duplicated zip-substitute block remains in the touched sites.
3. [x] `make check` passes on every task; final `make verify` passes.
4. [x] A non-generic program's emitted C is unchanged (no regression in the existing suites).
5. [x] No `spec/` file and no `*_spec_test.rs` file is modified.
6. [x] The roadmap records DEF-06…DEF-10 as fixed and the plan is marked Done.
