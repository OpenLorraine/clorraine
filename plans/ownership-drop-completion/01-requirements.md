# Requirements: Ownership and Drop Model Completion

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

The compiler must implement the frozen ownership and drop rules for arrays and
must enforce the view-field ban. Today a fixed array is treated as a view, so its
elements are copied instead of moved and the array is never dropped; a slice
field is accepted; and owning-element drop glue for the growable container is
staged. This feature makes the implementation match `spec/03-ownership.md`,
`spec/08-arrays-strings.md`, `spec/11-memory.md`, `spec/16-borrow-rules.md`, and
`spec/appendix-stdlib.md`. It introduces no new language surface.

## Functional Requirements

### Must Have

- [ ] **R1 — Shape rule (AR-5):** a fixed array (`TyKind::Array` with every
  extent present) is an *owned aggregate*; a slice (at least one extent omitted)
  is a *view*. One predicate expresses the slice case, and the fixed array is its
  complement.
- [ ] **R2 — Classification (AR-5, AR-23):** `borrow::is_view`,
  `borrow::is_elided_view`, and `borrow::is_nullable_capable` treat a fixed array
  as owned (not a view), and a slice as a view. `borrow::is_lend_candidate` is
  **unchanged**: a fixed array is borrowed storage a returned view may borrow, so
  it stays a candidate (PF-006). `TypeTable::duplicable` treats a slice as
  duplicable regardless of its element (AR-22).
- [ ] **R3 — Fixed-array return (AR-9):** `func pair(): String[N] { ... return
  [a, b]; }` compiles and returns by value; it is not a view return and needs no
  elision candidate (`spec/08-arrays-strings.md` section 6). Because C cannot
  return an array, this requires the by-value lowering in
  [03-04](03-04-fixed-array-return.md); classification alone is not enough.
- [ ] **R4 — Element moves (AR-6):** in both the ownership walk and the lowering,
  an array-literal element is consumed exactly like a struct-literal field: an
  owned (non-duplicable) place moves and its source is emptied; a duplicable place
  copies; a type that still mentions a generic parameter is left to the
  instantiation.
- [ ] **R5 — Fixed-array drop (AR-7):** at scope exit a fixed array drops its
  elements first-to-last. A fixed array of duplicable elements emits no drop.
- [ ] **R6 — Growable drop and `E90000` (AR-2, AR-3, AR-7):** a growable `Array<T>`
  drops its elements first-to-last and then frees its buffer. The
  `Array<move-only>` trigger for `E90000` is removed, so `Array<Buffer>` and
  `Array<String>` are accepted. The stale staged golden is removed and `E90000`
  stays a dormant fallback (AR-3).
- [ ] **R7 — Fill guard (AR-10):** a fill literal (`[x; fill]` or `[; fill]`)
  whose element type is not duplicable is rejected under the existing `E10215`
  code; the fill form cannot repeat a move-only value.
- [ ] **R8 — View fields (AR-4):** `E10205` rejects a field whose type is `str`
  or a slice (`T[]`), at any nullability. It keeps accepting `cstring`,
  `static str`, `raw`, `func`, handles, structs, and fixed arrays.
- [ ] **R10 — `set` drops the replaced element (AR-25):** once
  `Array<move-only>` is accepted (R6), `xs.set(i, take v)` must drop the old
  element before storing `v`, as `spec/appendix-stdlib.md:62` requires.

### Should Have

- [ ] **R9 — No regressions:** every existing `spec/` test, Rust suite, and the
  corpus stays green.

### Won't Have (Out of Scope)

- Nested partial moves (ledger O4), per-field loan precision (O8), aggregate
  view returns (ledger F2), `Box<T>` (ledger X1), and non-lexical lifetimes —
  each stays deferred (AR-1). This plan does not change the borrow checker's
  lexical model.
- Whole fixed-array assignment (`b = a;`, `b = id(a)`) is a pre-existing codegen
  gap and stays out of scope (PF-009); no requirement depends on it.

## Technical Requirements

### Performance

No target. The element-drop loop is bounded by the array length; type-checking
cost is unchanged in order (AR-17).

### Compatibility

Internal compiler change only. No CLI, ABI, profile, or emitted-artifact contract
changes (AR-15). The two spec text edits (AR-10, AR-20) are the only `spec/`
chapter changes; the plan adds no `spec/` golden (AR-12).

### Security

Not applicable as an input surface; the change improves memory safety by removing
a use-after-free and a double-free (AR-16).

## Scope Decisions

| Decision | Options Considered | Chosen | Rationale | AR Ref |
| -------- | ------------------ | ------ | --------- | ------ |
| Plan boundary | fix confirmed defects / pull deferred items forward | fix confirmed defects | the ledger freezes O4/O8/F2/X1 as deferred | AR-1 |
| Array shapes fixed | fixed+growable / fixed only | fixed+growable; un-stage `E90000` | `spec/appendix-stdlib.md` section 3 defines growable element drop | AR-2 |
| Array shape predicate | one predicate / per-site | one predicate | the defects come from one fact duplicated across stages | AR-5 |
| Element consumption | move like struct fields / copy | move like struct fields | a move is copy-plus-empty (O1/O3) | AR-6 |
| Growable drop lowering | inline loop / new helper | inline loop over `cl_buffer_ptr` | ordinary code in the existing drop ladder | AR-8 |
| `E90000` disposition | delete code / dormant fallback | dormant fallback | minimal change to the authorized un-stage | AR-3 |
| View fields | `str`+slices / `str` only | `str`+slices | `spec/08-arrays-strings.md:289` and T4/T9 | AR-4 |
| Fill of move-only | reject / accept risk | reject under `E10215` | reproduced double-free | AR-10 |
| Test placement | Rust suites / new goldens | Rust suites + existing oracle | matches prior plans; no new `spec/` golden | AR-12 |

> **Traceability:** every scope decision above cites the Ambiguity Register entry
> that resolved it — see `00-ambiguity-register.md`.

## Acceptance Criteria

1. [ ] R1–R10 implemented and each traced to an `ST-` case in `07-testing-strategy.md`.
2. [ ] `make check` is green after every task; `make verify` is green before completion (AR-13).
3. [ ] No dead code: the removed `E90000` trigger leaves no unused function, and no stale helper remains.
4. [ ] The two `spec/` text edits (AR-10, AR-20) are present and consistent with the code.
5. [ ] Documentation updated: the plan state and the `E90000` un-staging note.
