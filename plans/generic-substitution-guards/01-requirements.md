# Requirements: Generic Substitution Guards

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

The compiler checks two rules at the declaration site, where a type parameter is
still abstract, so the concrete kind supplied by a type argument is invisible.
This feature adds an instantiation-time audit over the monomorphization set the
ownership stage already collects. It introduces no language surface: the frozen
rules (`spec/02-types.md:184` TY-10, `spec/08-arrays-strings.md:50` AL-1)
already forbid both programs; the implementation misses them.

## Functional Requirements

### Must Have

- [ ] **R1 — View-field guard at the instance (AR-2, AR-4):** for every recorded
  generic struct instance, a field whose declared type mentions a type parameter
  and whose substituted type is `str` or a slice reports `E10205` with the
  frozen message and help. A nullable substituted view is rejected the same way.
  `static str`, `cstring`, `raw`, `func`, handles, and fixed arrays stay
  accepted, exactly as the declaration-site check decides them.
- [ ] **R2 — Fill guard at the instance (AR-2):** for every recorded generic
  function instance, a fill literal whose declared type mentions a type
  parameter and whose substituted base element is concrete and not duplicable
  reports `E10215` with the frozen message and help. The duplicable
  instantiation stays accepted, and an inferred type argument is audited exactly
  like an explicit one.
- [ ] **R3 — Scoped to substituted types (AR-7, AR-17, PF-004):** the audit
  examines only a declared type that mentions a type parameter, so it never
  re-reports a declaration-site case; the declaration-site guards themselves are
  unchanged. A concrete illegal field or fill is owned by the type checker,
  which stops the ownership stage before the audit runs, so the two can never
  report the same site.
- [ ] **R4 — Deterministic coverage of the recorded set (AR-8, AR-9, AR-19):**
  every concrete instance the monomorphization records is audited — explicit and
  inferred type arguments, locals, parameters, returns, globals, and
  temporaries. One error is reported per illegal instance-field or instance, in
  monomorphization order. Instances the collector does not record (a discarded
  call's result type, a nested instance reached only from an IR-discovered outer
  instance, a lifted lambda body) are a separate pre-existing
  instance-discovery gap, kept out of scope (AR-19).
- [ ] **R5 — Audit boundary (AR-11, AR-19):** the audit runs over the recorded
  `Item::Func` instances. `comptime` generics are `E90001`; an uninstantiated
  generic is not checked and emits nothing. A fill inside a lifted lambda body
  reachable from an instance is a separate pre-existing monomorphization gap,
  kept out of scope (AR-19).
- [ ] **R6 — No regressions (AR-12):** every existing `spec/` test, Rust suite,
  corpus program, and profile stays green; a valid program emits byte-identical
  C.
- [ ] **R7 — No frozen-artifact change (AR-12):** no `spec/` chapter, registry
  row, golden, or standard-library source changes; no CLI, ABI, or profile
  contract changes.

### Should Have

- [ ] **R8 — Diagnostic parity (AR-3, PF-005):** each new diagnostic reuses the
  exact text and help of its declaration-site twin, so no `spec/14` text edit is
  needed. For `E10215` that is the fill help
  `a fill cannot repeat a move-only element; construct each element`
  (`compiler/src/sema/expr.rs:713`), not the registry row's combined help.

### Won't Have (Out of Scope)

- Transitive function instantiation: a generic function that calls another
  generic function with its own type parameter emits an undefined C symbol
  today; it is a separate pre-existing defect and is not fixed here (AR-5).
- Intrinsic-only instance discovery: a struct named only in `sizeof(Wrap<str>)`
  is not emitted today; a separate pre-existing defect (AR-5).
- Instance-discovery completeness beyond the recorded set (AR-19): a discarded
  statement-level call's result type, a nested generic struct reached only from
  an IR-discovered outer instance, and a fill inside a lifted lambda body are
  not audited; each already fails at the C compile today.
- A deep view ban inside fixed arrays (`str[2]`, `Wrap<str[2]>`): the audit
  mirrors the existing top-level predicate; widening the frozen rule is a
  separate change (AR-4).
- A generalized per-instance replay of the checking rules; a trait-bound-like
  "duplicable" constraint (AR-10).

## Technical Requirements

### Performance

The audit is linear in the recorded instances times the size of the body they
name, and runs once per compile; no performance target changes (AR-13).

### Compatibility

Internal compiler change only. No CLI, ABI, profile, or emitted-artifact
contract change; valid programs are unaffected (AR-12). Diagnostics remain
deterministic because the instance order is deterministic (AR-13).

### Security

Not applicable as an input surface. The change improves memory safety by
rejecting a double-free program and a view-field escape before code generation
(AR-14).

## Scope Decisions

| Decision | Options Considered | Chosen | Rationale | AR Ref |
| -------- | ------------------ | ------ | --------- | ------ |
| Plan routing | full plan / task mini-plan | full plan | mirror of the local defect-fix convention | AR-1 |
| Fill semantics | instance audit / declaration rejection | instance audit | the spec requires a duplicable fill value, not a declaration-time proof | AR-2 |
| View predicate | mirror / deep ban | mirror | the defect is the substitution blind spot, not the rule's depth | AR-4 |
| Check placement | new module / extend `generics.rs` | new module | `generics.rs` is at 674 lines; the audit has one responsibility | AR-6 |
| Audit selection | parameter-mentioning only / all instances | parameter-mentioning | keeps the audit scoped to substituted types; a declaration-site error already stops the stage before the audit (PF-004) | AR-7 |
| Instance discovery | recorded set only / extend discovery | recorded set only | the unrecorded gaps already fail at C compile and are a separate change | AR-19 |
| `spec/` text | reuse registry text / edit Ch 14 | reuse | the frozen rules already forbid both programs | AR-3, AR-12 |
| Complexity | focused audit / generalized replay | focused audit | no material support surface beyond the existing data | AR-10 |

> **Traceability:** every scope decision above cites the Ambiguity Register entry
> that resolved it — see `00-ambiguity-register.md`.

## Acceptance Criteria

1. [ ] R1–R8 implemented and each traced to an `ST-` case in `07-testing-strategy.md`.
2. [ ] `make check` is green after every task; `make verify` is green before completion (AR-16).
3. [ ] No dead code: the module has no unused helper, and the declaration-site guards are untouched.
4. [ ] No `spec/`, `stdlib/`, or corpus source changes (AR-12).
5. [ ] Documentation updated: the plan state and the `DEF-12` roadmap status.
