# Requirements: Module Initializer Integrity

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

Batch F closes the 9 filed module-initializer defects. A module-level `let`/`const` initializer
must behave, for the expression surface it uses, exactly as the same expression in a function
body: calls resolve (lambdas lift, func-typed globals spell their symbol, dot calls pass their
receiver, generic calls declare their instance), aggregates emit valid C (statement-form stores
for fixed-array fields, the view wrap at `str` destinations, structural `len` folds), owning
intermediates release once, and write-mutability rules report. One body-side gap (a written
slice type argument missing its typedef) belongs to the same batch because the initializer
spelling was fixed in the preceding batch and the body spelling was noted then.

## Functional Requirements

### Must Have

- [ ] **R1 (DEF-131).** A lambda-literal callee in an initializer builds under the strict C
  flags and runs; its lifted function and callable identity are emitted like a body lambda's.
- [ ] **R2 (DEF-131).** A call through a func-typed module global in an initializer spells the
  global's mangled C symbol.
- [ ] **R3 (DEF-132).** A generic function or callable call in an initializer declares and uses
  its instance (no implicit declaration at the strict build).
- [ ] **R4 (DEF-133).** A callable-field (dot) call in an initializer passes its receiver like
  the body spelling.
- [ ] **R5 (DEF-134).** A struct literal with a fixed-array field from a place or call result in
  an initializer emits a valid store (statement form with `memcpy` where the body does).
- [ ] **R6 (DEF-135).** A `str`-typed module binding initialized from a template or owned
  `String` value receives the view wrapper (`cl_string_view`).
- [ ] **R7 (DEF-143).** A qualified dot-`len` on a fixed-array field in an initializer folds to
  the declared extent (never a silent `0`).
- [ ] **R8 (DEF-130).** Every owning temporary a module initializer binds releases exactly once
  at the init function's end; the three filed leak shapes are host-ASan-clean.
- [ ] **R9 (DEF-152).** Initializer expressions run the write-mutability checks: a write through
  a `const` global reports `E10023`, an immutable-view write reports its code, matching the
  function-body reports.
- [ ] **R10 (DEF-137).** A written slice type argument (`sizeof(i32[])`) in a function body
  defines its slice typedef, like the initializer spelling.

### Should Have

- [ ] The fixes are shape-gated: unaffected initializers re-emit byte-identically (R11).
- [ ] Each mirrored contract keeps one definition where practical; where the init path must
  duplicate a body predicate, the duplication is named in the design doc.

### Won't Have (Out of Scope)

- The Design A unification (own-stage synthetic init functions) — excluded machinery (AR-2).
- Any new diagnostic code or specification edit — reserved (AR-12); a fix that needs one pauses
  to the user.
- The review-filed residue rows (DEF-167 – DEF-195) — later batches.
- New language surface (initializer closures with captures, move semantics in initializers).

## Technical Requirements

### Performance

- No measurable compile-time impact beyond the added walks (initializer expressions are a small
  fraction of the AST).

### Compatibility

- The emitted C for unaffected initializers stays byte-identical (R11); the existing
  `eval_emission_integrity_*` and `store_emission_integrity_*` init pins stay green untouched.
- Both profiles compile the changed shapes where the feature surface exists (`std.text` shapes
  are hosted/host profiles; slice typedefs are profile-independent).

### Security

- Not applicable beyond memory safety: the release and check fixes remove leak and
  invalid-write classes. No input handling changes.

## Scope Decisions

| Decision | Options Considered | Chosen | Rationale | AR Ref |
| -------- | ------------------ | ------ | --------- | ------ |
| Pipeline strategy | A) own-stage synthetic init functions; B) extend the init path | B | Smallest viable; matches precedent; A changes every global program | AR-2 |
| Lambda callees | Lift / reject with a new code | Lift | The frozen surface allows closures; rejection needs a reserved code | AR-3 |
| Initializer writes | Run the checks / leave accepted | Run | Conformance to frozen mutability rules; no pinned shape flips | AR-4 |
| Release point | Init-function end, reverse order, null-guarded / reject | End | The init function is one scope (MOD-20); no accepted-surface narrowing | AR-10 |
| Diagnostics | Existing codes only | Existing | A new code is a reserved registry change | AR-12 |
| Test suites | New `module_initializer_integrity_{spec,impl}_test.rs` / extend the eval-emission suites | New suites | Per-batch suite convention; the eval-emission suites stay untouched oracles | AR-15 |

> **Traceability:** Every scope decision references the Ambiguity Register entry (AR #) that
> resolved it. See `00-ambiguity-register.md`.

## Acceptance Criteria

1. [ ] Every R1 – R10 acceptance item holds with spec tests written first (red at the batch
   base, green at the head).
2. [ ] `make check` green (all suites) and `make verify` all legs passed at close-out.
3. [ ] The existing initializer pins are unchanged (byte-identity for unaffected shapes).
4. [ ] No new diagnostic code, no `spec/` change.
5. [ ] `AGENTS.md` test map and `plans/00-roadmap.md` synced; the plan closed with its review
   evidence.
