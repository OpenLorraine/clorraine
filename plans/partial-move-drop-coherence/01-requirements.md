# Requirements: Partial-Move / Drop Coherence

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

An owned value appears once in memory and is dropped once. When a top-level field
is moved out of an owned value, the source field must become empty as part of the
move, so every later drop of the source is a no-op for that field. The compiler
already does this for a whole place and for a `take` argument, but not for an
owned place nested inside an aggregate literal; the literal copies the field and
the source keeps owning it. This plan consumes such a nested owned field as a
move.

This is a defect-remediation plan: it changes no language syntax, adds no
diagnostic, and edits no `spec/` file. It makes the implementation match
`spec/17-codegen.md` P3 and the ownership chapters.

## Functional Requirements

### Must Have

- [ ] R1 — A value-consuming read of an owned (non-duplicable) place nested in an aggregate literal is lowered as a move that zeroes the source, so the source is emptied before the literal is built. `AR-1`, `AR-3`.
- [ ] R2 — A struct-literal reassignment whose RHS moves a field out of the assignment target (`b = Box<String> { value: b.value };`) frees the value exactly once. `AR-2`.
- [ ] R3 — The temporary-binding form (`let t = b.value; b = Box<String> { value: t };`) frees the value exactly once. `AR-2`.
- [ ] R4 — A consumed non-duplicable literal field is recorded as moved in the ownership flow, so a second read of the same field reports `E10020`, a deeper-than-top-level path reports `E10026`, and a field of a borrowed parameter reports `E10034`. A field whose type mentions a type parameter is skipped by both the mark and the lowering. `AR-14`, `AR-15`.
- [ ] R5 — Generic and non-generic owning structs behave identically (`Box<String>` and `Label`/`String`). `AR-11`.
- [ ] R6 — The assignment lowering is unchanged: the existing drop-then-store fast path and the borrow behavior (`f = Dup(borrow f)`) are preserved. `AR-3`, `AR-12`.

### Should Have

- [ ] R7 — A multi-field move (`s = S { a: s.a, b: s.b };`) moves each field once. `AR-14`.

### Won't Have (Out of Scope)

- No new language surface, syntax, or diagnostic.
- No edit to any file under `spec/` (AR-7).
- No change to the assignment emitter; the `value_reads_base` save/restore branch is left as is (it is not reached once the literal field is consumed) (AR-3).
- No fix for a `take`-argument dotted field: `consume(b.value)` already zeroes; the postfix `(b).value` shape is a separate defect, not DEF-11 (AR-15).
- No self-assignment special case: `s = s` already frees once through `Statement::Move` (AR-13).
- Deeper-than-top-level partial moves stay `E10026`; guarded/short-circuit fields, arrays of owners, owner elements, and global places are out of scope (AR-15).
- No new test harness or dependency beyond an ASan compile-and-run helper in an implementation suite (AR-4).

## Technical Requirements

### Compatibility

- The workspace stays Rust **std-only**; the change reuses the existing IR
  (`Statement::Move`) and `emit_move_zero`.
- A program with no value-consuming move-out keeps the existing emitted C: the
  assignment fast path is untouched (AR-8).

### Correctness Invariants

- A move is a copy plus a zero of the source; the zeroed source is the drop flag
  (`spec/17-codegen.md:182-199`).
- Every owned value is freed exactly once on every path (normal exit, early exit,
  loop iteration, and reassignment).
- Partial moves remain limited to a top-level field of an owned local or a
  `take` parameter (v1) (AR-15).

### Diagnostics

- No message changes; the existing move diagnostics (`E10020`, `E10025`,
  `E10026`, `E10034`) are reused (AR-2, AR-14, AR-15).

### Security

- Memory safety: the fix closes a double-free (use-after-free) hole. There is no
  external input surface (AR-9).

## Scope Decisions

| Decision | Options Considered | Chosen | Rationale | AR Ref |
| -------- | ------------------ | ------ | --------- | ------ |
| Fix site | lowering + ownership mark / assignment reorder / codegen-only | ownership mark + lowering | The literal field is the root; once consumed the assignment path is already correct | AR-3 |
| Assignment phase | keep / drop | drop | Unneeded after the literal fix and would break an immutable spec oracle | AR-3 |
| `take`-field | fix/retarget / remove | remove | The common form already works; the remaining shape is a separate defect | AR-15 |
| Self-assignment | keep / drop | drop | Already frees once; not a DEF-11 case | AR-13 |
| Tests | ASan + emitted-C | ASan in an impl suite + emitted-C in the spec suite | Keeps the spec oracle sanitizer-independent | AR-4 |

> **Traceability:** Every scope decision references the Ambiguity Register entry
> (AR #) that resolved it. See `00-ambiguity-register.md`.

## Acceptance Criteria

1. [ ] R1–R7 satisfied; DEF-11 no longer reproduces (ASan clean).
2. [ ] The moved-out source is zeroed in the emitted C for the struct-literal and temporary forms.
3. [ ] `make check` passes on every task; final `make verify` passes.
4. [ ] The assignment fast path is preserved (no emitted-C change for a non-reading RHS).
5. [ ] No `spec/` file is modified.
6. [ ] The roadmap records DEF-11 fixed and the plan is marked Done; PLAN-10 Phase 4 resumes.
