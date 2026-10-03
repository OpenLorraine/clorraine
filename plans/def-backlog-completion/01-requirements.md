# Requirements: Def Backlog Completion

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

Seven defects filed during the residual-defect-completion cycle are open on the roadmap. This plan
closes all seven in one cycle. Each defect is a pre-existing compiler behavior where an accepted
program either emits invalid C, leaks, double-frees, or reads freed memory — or where a wrong slot
is moved. The plan fixes each reproduction, leaves the concrete spellings and accepted programs
unchanged where they are correct, and adds one new backlog row for a residual discovered while
re-verifying a filed remedy (AR-1, AR-6).

The plan modifies compiler internals only. The frozen specification under `spec/` is the oracle and
does not change; the only user-visible acceptance change is the generic lend rejection, which makes
the generic spelling match the concrete spelling's existing `E10034` (AR-2, AR-9).

## Functional Requirements

### Must Have

- [ ] **R1 — Generic lend escaping as owned (DEF-56).** A generic instantiation must not accept a
  lend returned, stored, or passed as an owned value when the instantiated type owns; it reports the
  existing `E10034` at the move site, matching the concrete spelling (a moved lend that is read
  again additionally keeps its existing `E10020`, PF-024; the rule also covers a concrete-typed
  lend in a maybe-owning position, PF-021). Duplicable instantiations stay accepted
  (AR-2, [03-01](03-01-generic-lend-duties.md)).
- [ ] **R2 — Generic lend passed to a take position (DEF-61).** The same duty covers a default-lend
  parameter passed to a `take` position inside a generic body; the instantiated `String` form
  reports `E10034`, mirroring `spec/tests/compile-fail/E10034_borrow_released.clo` (AR-2, 03-01).
- [ ] **R3 — Runtime-index element move-out (DEF-60).** An ownerless element take with a runtime
  index (`sink<T>(mk()[i])`, guarded or unguarded) must zero the slot the callee moved, not slot
  zero; the program runs without a double free and with the moved value correct
  (AR-1, [03-02](03-02-move-out-integrity.md)).
- [ ] **R4 — Ownerless element read on store and return (DEF-59).** `let s: T = mk()[0]` and
  `return mk()[0]` under a maybe-owning substitution must move the element out of the
  statement-owned temporary before its release, so no read touches freed memory
  (AR-5, 03-02).
- [ ] **R5 — Do-while early return release (DEF-62).** A `return` inside a `do ... while (guarded)`
  body must release the condition's materialized temporaries; the reproduction runs leak-free under
  AddressSanitizer (AR-4, [03-03](03-03-do-while-condition-release.md)).
- [ ] **R6 — Specialized closure argument spelling (DEF-57).** A specialized capturing closure
  passed to a callable-bound generic parameter must spell the same callable type at the call site
  and in the callee; two enclosing specializations must produce distinct C symbols and both must run
  (AR-3, [03-04](03-04-closure-c-spelling.md)).
- [ ] **R7 — Zero-argument closure call (DEF-58).** A zero-argument call through a closure-typed
  value must emit valid C without a trailing separator (AR-3 family, 03-04).

### Should Have

- [ ] **R8 — No new acceptance changes beyond R1/R2.** Concrete spellings, accepted programs, and
  emitted shapes not covered by the seven defects stay unchanged; pinned re-emission stays
  byte-identical (AR-9, AR-10).

### Won't Have (Out of Scope)

- The guarded member-root sibling leak (`flag && sink(mkv().a)`) is filed as a new backlog defect,
  not fixed here (AR-1, AR-6).
- The aggregate-literal consume form (`Box<T> { v: mk()[i] }`) is a distinct defect filed as DEF-64,
  not fixed here; this plan documents the limitation (PF-009).
- Guard-deferred take-root binding, the `ClosureTy` enclosing-argument representation, and the
  lower-layer return move-out (AR-1, AR-3, AR-5).
- Any `spec/` text change, new diagnostic code, dependency, or layout change (AR-9).
- DEF-45-class transfer temporaries, deferred element-move limits, and any other reviewed residual.

## Technical Requirements

### Compatibility

- No `spec/` file changes; `git diff --name-only <plan start>..HEAD -- spec/` stays empty.
- The `host`, `hosted-amiga`, and `freestanding` profiles must stay green (`make check`,
  `make verify`).
- The `own` stage must not import the `codegen` stage; the shared key renderer moves to the
  stage-neutral `types` module (AR-3).
- The new `E10034` duty must not fire for `take` parameters, locals, or duplicable substitutions
  (AR-2).

### Memory Safety

- The five runnable reproductions (DEF-57, DEF-58, DEF-59, DEF-60, DEF-62) run with host
  AddressSanitizer and leak detection enabled and report neither a double free nor a leak; the two
  rejection fixes (DEF-56, DEF-61) prove `E10034` and no emitted C. The close-out probe runner
  covers all seven in those two shapes, with a timeout and `handle_segv=0` for determinism
  (AR-7, AR-12, PF-010, PF-011, PF-025).

### Performance

- No runtime cost beyond the existing reference: the fixes are compile-time decisions plus, for
  R3/R4, zeroing a slot that the existing move paths already zero.

### Security

- N/A for input validation, authentication, injection, and rate limiting: the compiler has no
  service or user-input runtime surface. The security-relevant property this plan hardens is memory
  safety — every reproduction must run without a double free, leak, or use-after-free
  (AR-7, AR-12).

## Scope Decisions

| Decision | Options Considered | Chosen | Rationale | AR Ref |
| -------- | ------------------ | ------ | --------- | ------ |
| DEF-60 root cause and scope | Fix index move-out and file the member leak / include the guard-deferred binding | Fix index move-out; file the member leak | The filed double free is the index loss; the guard binding fixes a different, non-critical leak | AR-1 |
| Generic lend acceptance | Reject via `E10034` duty / move-on-return semantics / infer `take` | Reject via `E10034` duty | Matches the concrete spellings and the frozen spec; smallest | AR-2 |
| Closure instance reach | Stamp key at recording / `ClosureTy` representation / reject | Stamp key, relocate renderer to `types` | Reuses `ClosureTy.instance`; preserves stage boundaries | AR-3 |
| Do-while release layer | Checker-local append / emitter tracking / reorder lowering | Checker-local append | Keeps the IR authoritative; reuses the return pipeline | AR-4 |
| Element read move layer | Lower store + emitter return / lower both | Lower store + emitter return | Renders the lowered value, so a runtime index survives | AR-5 |
| Filing | File member leak / include it | File it | The plan charter is the seven filed defects | AR-6 |
| Verify | `make check` per task; `make verify` + probes at close-out / `make verify` every phase | Confirm the first | Matches the project's commands and the previous cycle | AR-7 |

> **Traceability:** Every scope decision references the Ambiguity Register entry (AR #) that
> resolved it. See `00-ambiguity-register.md`.

## Acceptance Criteria

1. [ ] R1 – R7 each pass their specification cases (ST-1 – ST-27) and the host sanitizer probes.
2. [ ] No `spec/` file changes and no new diagnostic codes.
3. [ ] `make check` green for every phase; `make verify` green at close-out.
4. [ ] The close-out probe runner runs five AddressSanitizer run probes (DEF-57, DEF-58, DEF-59,
   DEF-60, DEF-62) clean and two rejection probes (DEF-56, DEF-61) that emit `E10034` with no C
   written (PF-011).
5. [ ] Documentation synced: `AGENTS.md` test map, the roadmap rows (DEF-56 – DEF-62 Done; DEF-63
   and DEF-64 verified as filed), and the corrected DEF-60 remedy (AR-6, AR-10, PF-009, PF-016).
