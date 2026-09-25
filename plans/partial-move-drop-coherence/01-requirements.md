# Requirements: Partial-Move / Drop Coherence

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

An owned value (a handle, or a struct containing one) appears once in memory and
is dropped once. When a top-level field is moved out of an owned value, the
source field must become empty as part of the move, so every later drop of the
source is a no-op for that field. The compiler already does this for a whole
place and for a `take` argument, but not for an owned place nested inside an
aggregate literal, and its assignment save-copy can free a moved-out value. This
plan generalizes the existing move lowering to those sites.

This is a defect-remediation plan: it changes no language syntax, adds no
diagnostic, and edits no `spec/` file. It makes the implementation match
`spec/17-codegen.md` P3 and the ownership chapters.

## Functional Requirements

### Must Have

- [ ] R1 — A value-consuming read of an owned (non-duplicable) place is lowered as a move that zeroes the source, wherever it occurs: an aggregate-literal field or element, a `take` argument, and a whole place (already correct). `AR-1`, `AR-3`.
- [ ] R2 — A struct-literal reassignment whose RHS moves a field out of the assignment target (`b = Box<String> { value: b.value };`) frees the value exactly once. `AR-2`.
- [ ] R3 — The temporary-binding form (`let t = b.value; b = Box<String> { value: t };`) frees the value exactly once. `AR-2`.
- [ ] R4 — The assignment lowering releases the old target after the RHS is evaluated and never drops a stale copy that aliases a moved-out value. `AR-3`.
- [ ] R5 — Generic and non-generic owning structs behave identically (`Box<String>` and `Label`/`String`). `AR-11`.
- [ ] R6 — A borrow RHS keeps its current behavior: `f = Dup(borrow f)` releases the old value after the RHS and frees each value once. `AR-12`.

### Should Have

- [ ] R7 — Self-assignment of an owned value (`s = s`) frees once. `AR-13`.
- [ ] R8 — A multi-field move (`s = S { a: s.a, b: s.b };`) moves each field once; reading a field twice is the existing use-after-move `E10020`. `AR-14`.

### Won't Have (Out of Scope)

- No new language surface, syntax, or diagnostic.
- No edit to any file under `spec/` (AR-7).
- No new test harness or dependency beyond an ASan compile-and-run helper (AR-4).
- Deeper-than-top-level partial moves stay `E10026`; arrays of owners, owner elements, and global places are out of scope (AR-15).
- No change to the `freestanding`/`hosted-amiga` ABI or runtime shims.

## Technical Requirements

### Compatibility

- The workspace stays Rust **std-only**; the change reuses the existing IR
  (`Statement::Move`, `Statement::MoveOut`) and `emit_move_zero`.
- A program with no value-consuming move-out keeps byte-identical emitted C
  (pinned by the determinism and corpus suites) (AR-8).

### Correctness Invariants

- A move is a copy plus a zero of the source; the zeroed source is the drop flag
  (`spec/17-codegen.md:182-199`).
- Every owned value is freed exactly once on every path (normal exit, early
  exit, loop iteration, and reassignment).
- Partial moves remain limited to a top-level field of an owned local or a
  `take` parameter (v1) (AR-15).

### Diagnostics

- No message changes; the existing move diagnostics (`E10020`, `E10025`,
  `E10026`) are reused (AR-2, AR-15).

### Security

- Memory safety: the fix closes a double-free (use-after-free) hole. There is no
  external input surface (AR-9).

## Scope Decisions

| Decision | Options Considered | Chosen | Rationale | AR Ref |
| -------- | ------------------ | ------ | --------- | ------ |
| Breadth | general invariant / struct-literal only | general invariant | The temporary form is also broken and shares the root | AR-1 |
| Fix site | lowering + assignment / codegen-only | lowering + assignment | Only the lowering knows which place is consumed and moves it; codegen cannot infer moves | AR-3 |
| Tests | ASan + emitted-C / emitted-C only | ASan + emitted-C | The defect is a runtime double free; ASan proves free-once | AR-4 |

> **Traceability:** Every scope decision references the Ambiguity Register entry
> (AR #) that resolved it. See `00-ambiguity-register.md`.

## Acceptance Criteria

1. [ ] R1–R8 satisfied; DEF-11 no longer reproduces (ASan clean).
2. [ ] The moved-out source is zeroed in the emitted C for the struct-literal and temporary forms.
3. [ ] `make check` passes on every task; final `make verify` passes.
4. [ ] A program with no partial move keeps byte-identical emitted C.
5. [ ] No `spec/` file is modified.
6. [ ] The roadmap records DEF-11 fixed and the plan is marked Done; PLAN-10 Phase 4 resumes.
