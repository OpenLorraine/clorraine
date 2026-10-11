# Findings: Checker Conformance

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)
> **Status**: in progress

## Review Protocol

Each phase receives one review round from the lenses listed in its execution-plan header,
dispatched per the project policy (AGENTS.md). A finding is recorded as
`<lens>-NNN | severity | area | what | reproduction | expected vs observed | ruling`; a
critical or major finding gets an authorized ruling in the active mode and a fix round, then
one scoped re-review; direct verification closes further fixes in the same phase. The phase's
verdict line records `no findings` or the final state of every finding.

## Phase 1: Initializer guard operands and typedef collectors

**Baseline**: `477f27dd36dcbde5f40e6dc267ce1864388dbe11` (the `8170199` tree).
**Lenses**: correctness, memory-safety, emission-integrity

### Deliverables

| # | Shape | Result |
| - | ----- | ------ |
| DEF-136 | The initializer materialization stores an impure dividend (`Div`/`Rem`) and an impure slice index base into an init local before their guard runs; the fixed-array base keeps its spelling (C cannot assign an array to a temporary) and the existing single-call shape | Fixed; spec `initdividend`/`initbase`; impl pins the one-call stores |
| DEF-142 | The body collector records every value's own substituted type, so an inline `slice(...)` builder's view typedef exists | Fixed; spec `inlinearg`; impl pins the typedef and the compound literal |
| DEF-215 | The collector walks the lifted-lambda specializations with their substitutions (mirroring the emission's specialization guard) | Fixed; spec `lambdaspec`; impl pins the typedef and the lifted body's `sizeof` use |
| DEF-216 | The monomorphization records a written type argument's concrete types, so a struct instance referenced only as a slice element is declared — and, in the emitter, before the slice typedef that names it | Fixed; spec `structelement`; impl pins the declaration order |

### Red evidence

At the phase baseline the five row cases failed exactly as filed: `initdividend` and
`initbase` exited 22 (the guard and the operation each ran the impure call), `inlinearg`
failed the strict build with `'ClSlice_String' undeclared`, `lambdaspec` with
`'ClSlice_i32' undeclared` (the specialization body), and `structelement` with the
undeclared `ClWrap__i32` its slice typedef names. The `initownbase` reference passed at
the baseline (the fixed-array base was already single-call and AddressSanitizer-clean),
as intended.

### Implementation notes

- **Shape gating.** The initializer additions fire only for an impure dividend and an
  impure non-fixed-array base; the needs-materialization predicate mirrors both so a
  lazily evaluated position (a conditional arm or a short-circuit right side) structures
  its guard exactly when the store happens. The collector additions only grow the typedef
  set; the mono recording only grows the instance list.
- **The fixed-array guard.** `store_impure_intrinsic_arg` already documents the reason a
  fixed array never stores into a temporary; the index base carries the same guard so the
  verified owning-index-base shape re-emits byte-identically.
- **The specialization walk** mirrors `lifted_emissions`: same enclosing lookup, same
  `lifted_needs_specialization` gate, same `instance_subst` — so collected types exist
  exactly where a specialization is emitted.
- **The struct-instance recording** sits in `walk_value` beside the value-type recording,
  so it applies to plain bodies, instances, and lifted specializations alike.

### Verification

Spec suite 6/6 plain and under `RDC_ASAN=1`; impl 6/6; `make check` 62 suites green (the
two new suites included). The phase review (`correctness` + `memory-safety` +
`emission-integrity`) is dispatched after the phase commit.

### Review and fix round

**Three lenses reported; no critical or major finding on the change.**

| Lens | Verdict | Evidence highlights |
| ---- | ------- | ------------------- |
| correctness | no findings above minor | the five row cases red exactly as filed and green at head; ~40 base/head probes (laziness kept in arms and short-circuit sides, the guard/effect order now matching the body twins, fixed-array and pure operands byte-identical, the specialization gate, three-profile typedefs); the reference green at both refs |
| memory-safety | **no findings** | 24 ASan probes clean on head (view bases release nothing, the dividend store adds no release, the lazy stores run only on their path, the owning fixed-array base releases once, the trap path); no leak, double free, or use-after-free attributable to the change |
| emission-integrity | **no findings** | 4476 base/head emission pairs: the only differing units are the six new cases plus one pre-existing inline `slice(...)`-base unit (an intended DEF-136-class fix); 2989 strict builds across host/freestanding/hosted-amiga with zero new failures; deterministic re-emission; declaration agreement (the instance typedef before the slice typedef) |

**Fix round (two minor items, fixed directly):**

| Finding | Severity | Area | Resolution |
| ------- | -------- | ---- | ---------- |
| RV-001 | 🟡 minor | `checker_conformance_impl_test.rs` carried two unused helpers (`dead_code` warnings) | Fixed: the unused proximity helper deleted, and `emit_twice` wired into the new `reemit` determinism pin (the changed initializer store re-emits byte-identically) |
| RV-002 | 🟡 minor | the initializer materialization's doc lists still said "an impure index, divisor, or cast operand" | Fixed: both lists now name the index base and the dividend |

**Filed from the reviews' pre-existing observations** (base-identical; outside the phase):

- **DEF-219** 🟠 major — an owned bind in a short-circuit right side is declared inside its `if` block but released outside it, so an accepted initializer emits `__cl_itmp1 undeclared` (the batch G Phase 1 memory audit, PRE-1).
- **DEF-220** 🟠 major — a generic function called only from a module initializer with an inferred type argument emits no instance (undefined symbol); the memory audit's PRE-2.
- **DEF-221** 🟡 minor — a fixed-array literal base with a run-time index in an initializer emits `{ ... }[pos]` (invalid C99); the emission audit's P-1 and the correctness review's P2.
- **DEF-222** 🟡 minor — a generic instance recorded from a type argument is not field-expanded, so a nested instance field stays undeclared (`sizeOf<Box<i32>>()` names `ClPair__i32`); the correctness review's P1.
- **DEF-223** 🟡 minor — the global-initializer `sizeof(Wrap<i32>[])` spelling still names an undeclared instance (the initializer twin of DEF-216); the emission audit's P-2.

### Verification (updated)

Spec suite 6/6 plain and under `RDC_ASAN=1`; impl 7/7 (the determinism pin added); `make check` 62 suites green.

## Phase 2: Generic instance conformance

**Baseline**: `5a378fb` (the Phase 1 close-out tree).
**Lenses**: correctness, semantics, emission-integrity

### Deliverables

| # | Shape | Result |
| - | ----- | ------ |
| DEF-145 | `check_for_in` rejects a null-shaped iterable with `E10012` and records an opaque-iterable duty; the per-instance audit (`check_iterable_requirements`) substitutes each instance and reports `E10043` when the type is not an array or its element is not a scalar or an enum | Fixed; spec `iterscalar`/`iterenum`/`iterstruct`/`iternull`; impl pins the scalar and borrow loop shapes |
| DEF-146 | A value-form member read on a bare type parameter records an opaque projection; the audit (`check_opaque_projections`) resolves it through the substituted type and reports `E10212` with the substituted display when the field is missing, while a resolving instance keeps the read | Fixed; spec `memval` (rejected) and `memvalcall` (unchanged); impl pins `return x.foo` for the resolving instance |
| DEF-150 | `resolved_value_ty` resolves a conditional value when both arms name the same substituted type, so the structural `len` fold sees the real extent and still evaluates nothing | Fixed; spec `condfold`, `condfolddrop`; impl pins `return 7;`; the concrete twin `condfoldconcrete` is unchanged |

### Red evidence

At the phase baseline the seven row cases failed exactly as filed: `iterscalar`, `iterenum`,
`iterstruct`, and `iternull` were accepted (the first three emitted invalid C that failed the
strict build; the null one emitted an unsigned `< 0` comparison), and `condfold`,
`condfolddrop`, and `memval` were accepted with the wrong result (`0` for the folds; a member
access on an untyped value for `memval`, which after the amendment reports the instance's
member error). The five references (`iterok` 4, `iterenumelem` 2, `iterborrow` 4, `memvalcall`
one `E10212`, `condfoldconcrete` 7) passed at the baseline, as intended.

### Design notes

- **The DEF-146/DEF-150 conflict (the AR-5 amendment).** A declaration-site `E10212` for a
  value-form member read on a parameter rejects the designed late resolution: the probe
  `get(W { foo: 7 })` builds and runs at the baseline, and DEF-150's arms (`b.s`) reach the
  fold through the same path. The report therefore lands per instance, mirroring DEF-145's
  audit, with the substituted type in the message. The call form's declaration-site report is
  unchanged (a call needs a signature the declaration must know).
- **The pinned element rule (DEF-145).** The probe matrix: an array whose element is a scalar
  or an enum builds and runs (scalar, `usize`, enum-element, and borrow-scalar instances
  verified); non-array iterables (scalar, enum) and arrays with struct, handle, view, or array
  elements fail the strict build. The audit accepts exactly the measuring set and reports
  `E10043` once at the first offending instance; the per-instance `borrow` requirement
  (`E10040`) is not added because the element rule already rejects those elements.
- **The null shape** lands as a shared `types::is_null_shaped` predicate with a unit test; the
  codegen method delegates to it. Phase 4 reuses the predicate for the remaining positions.

### Verification

Spec suite 18/18 plain and under `RDC_ASAN=1`; impl 11/11; `make check` 62 suites green.

### Review and fix round

_(findings recorded after the review round)_

## Phase 3: Call, member, and `cstring` surfaces

**Baseline**: _(recorded at phase start)_
**Lenses**: correctness, semantics, emission-integrity
**Gate**: AR-2 (the arity diagnostic code) must be resolved before Step 3.2.

_(findings recorded after the review round)_

## Phase 4: Null-shaped values

**Baseline**: _(recorded at phase start)_
**Lenses**: correctness, semantics

_(findings recorded after the review round)_

## Phase 5: Diagnostic hygiene

**Baseline**: _(recorded at phase start)_
**Lenses**: correctness, semantics

_(findings recorded after the review round)_

## Phase 6: Close-out

**Baseline**: _(recorded at phase start)_
**Lenses**: none new

_(the full-verification evidence recorded at close-out)_
