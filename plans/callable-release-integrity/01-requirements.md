# Requirements: Callable Release Integrity

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

Batch E of the 37-defect mandate closes the 13 callable/identity/release defects (DEF-138 –
DEF-141, DEF-155 – DEF-159, DEF-163 – DEF-166) filed with base-identical evidence by the
evaluation/emission-integrity reviews. Every requirement below was reproduced against the committed
compiler before this plan was written; each must be re-derived red at HEAD before its fix (AR-14).
No specification text changes and no new diagnostic codes (AR-11); one user-decided behavior
(non-null-narrowed widening reports `E10010`) follows the frozen narrowing rule (AR-9).

## Functional Requirements

### Must Have

- [ ] R1 (DEF-138) — A source local spelled like a generated owner (`__cl_owned0`) is an ordinary
  local: the accepted program builds, runs under ASan clean, and every binding keeps its own
  identity (AR-2, AR-3).
- [ ] R2 (DEF-139) — A call through a func-typed field of a temporary receiver (`H { … }.call(5)`,
  `makeH().call(5)`) releases the receiver's owned sibling fields (ASan-clean) (AR-7).
- [ ] R3 (DEF-140) — Two `func`-signature instances of one generic struct (`Holder<func(i32):
  i32>` and `Holder<func(u32): u32>`) emit two distinct struct definitions and build under the
  strict flags (AR-4).
- [ ] R4 (DEF-141) — A possibly-null `String` used through the implicit `str` widening without a
  narrowing check reports `E10010`; the narrowed and non-nullable spellings build and run
  (AR-9).
- [ ] R5 (DEF-155) — A func-typed field call behaves exactly like its module twin: `take` moves
  (caller emptied), `borrow` records the lend the twin records, and an owned temporary argument
  materializes and releases (AR-6).
- [ ] R6 (DEF-156) — A module-level func-typed global applies its own signature's modes (move-out
  and marker check for `take`; temporary release for owned arguments) (AR-6).
- [ ] R7 (DEF-157) — A `Param`-typed callee applies the instantiated signature's modes at each
  owning instance; a binding that shadows a same-named `take` module function reports no false
  `E10020` (AR-6).
- [ ] R8 (DEF-158) — A `defer` body materializes owned temporaries: the view backing and the
  func-value temporary spellings release exactly once per execution (ASan-clean) (AR-7).
- [ ] R9 (DEF-159) — A conditional view projection whose taken arm needs an owner releases that
  arm's backing (the two filed spellings ASan-clean; the both-call spelling unchanged) (AR-7).
- [ ] R10 (DEF-163) — A fixed array of view elements returned from a temporary reports the same
  diagnostics as the equivalent single-view return spelling (`E10036` family) (AR-8).
- [ ] R11 (DEF-164) — A discarded owned call statement releases its result (ASan-clean) (AR-7).
- [ ] R12 (DEF-165) — An inline capturing lambda emits one environment struct definition per
  unique (site, instance) and builds (AR-5).
- [ ] R13 (DEF-166) — A lambda's parameters are unmarked lends: the filed expression-bodied shape
  reports the same single `E10034` its block-bodied spelling and its module twin report (at the
  owning instance through the per-instance duty), and a closure call's owned temporary argument
  releases on the caller's side (ASan-clean) (AR-6, AR-17).

### Should Have

- [ ] R14 — Programs outside the defect shapes keep byte-identical emission where pinned, and no
  accepted non-defect program changes behavior (the corpus and strict-build sweeps prove it).
- [ ] R15 — Every row's reproduction is re-derived red at HEAD before its fix lands (AR-14).

### Won't Have (Out of Scope)

- Specification text edits or new diagnostic codes (AR-11 — such a need pauses to the user).
- New passes, layers, dependencies, or harnesses (AR-15).
- The remaining 24 defects (DEF-130 – DEF-137, DEF-142 – DEF-154, DEF-160 – DEF-162): batches F
  and G per the confirmed split (AR-1).
- Batch D (`clorraine-dx`) execution; it stays parked until no defect row remains open (AR-1).

## Technical Requirements

### Compatibility

- Accepted programs that already behave correctly stay accepted and byte-identical where pinned
  (R14); the emitted C keeps the strict-build flags green (`-std=c99 -O0 -Wall -Wextra -Werror`).
- The one new rejection surface (R10) reuses the existing `E10036` family and is the same report
  the single-view spelling already produces; the one behavior change (R4) reports the existing
  `E10010` per the frozen narrowing rule.

### Security

- No input surface changes; the fixes concern compiler-internal identity, ownership, and emission.
  Memory safety is the security property under test: ASan probes (host) and the leak/double-free
  reproductions cover every new release shape (R2, R8, R9, R11, R13).

### Performance

- No measurable effect: the fixes add lookups already on the hot path (identity keys, mode
  lists) and emit the same or fewer statements per defect shape.

## Scope Decisions

| Decision | Options Considered | Chosen | Rationale | AR Ref |
| -------- | ------------------ | ------ | --------- | ------ |
| Batch composition | the 13-row callable/release cluster / re-partition | E as confirmed | Memory-safety first; critical included | AR-1 |
| DEF-138 direction | identity-keyed / collision-avoiding naming | identity-keyed | Structural; kills the class | AR-2 |
| DEF-138 mechanism | keep generated bindings out of the name map + rename C collisions / reserve prefix | delegated | Reuses the `__cl_tmp*` precedent | AR-3 |
| Func type key | include the signature / separate mangle pass | delegated | One key owner; no new pass | AR-4 |
| Env structs | one per unique (site, instance) / rename duplicates | delegated | Redefinition is the defect | AR-5 |
| Callee modes | thread every callee's own signature, per instance / reject the shapes | delegated | Conformance to the move/lend rules | AR-6 |
| Release holes | extend the existing machinery / reject accepted-and-leaking shapes | delegated | The release model says release | AR-7 |
| View-array return | same diagnostics as the single-view spelling / new code | delegated | No new surface | AR-8 |
| Null widening | `E10010` / empty view | `E10010` | Frozen narrowing rule | AR-9 |

> **Traceability:** every scope decision references its Ambiguity Register entry — see
> `00-ambiguity-register.md` (including the PD-1 – PD-4 provenance blocks).

## Acceptance Criteria

1. [ ] All specification and implementation tests pass; `make check` and `make verify` are green.
2. [ ] Each of the 13 defects' reproductions is red at the pre-fix head and green after; the ASan
   runs (host) are clean for every accepted defect shape.
3. [ ] The strict builds pass for every accepted unit; no head-only C build failure.
4. [ ] No spec text or diagnostic-registry change in the diff.
5. [ ] Phase reviews recorded in the findings ledger; no blocking finding left open.
6. [ ] Roadmap rows DEF-138 – DEF-141, DEF-155 – DEF-159, DEF-163 – DEF-166 set Done with evidence;
   the loop continues to batch F.
