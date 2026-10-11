# Requirements: Checker Conformance

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Overview

Batch G closes 18 defect rows: the 15 filed checker-conformance rows, one plan-created companion
of DEF-147 (DEF-213, gated by AR-2), and two plan-created typedef rows (DEF-215, DEF-216) from
the batch F emission audit. Every requirement below is a conformance statement against the
frozen specification; none adds language surface.

## Functional Requirements

### Emission conformance

| # | Defect | Requirement | Frozen reference |
|---|--------|-------------|------------------|
| FR-1 | DEF-136 | An impure dividend and an impure index base are evaluated **once** in a module initializer, exactly as the divisor and the index already are: the guard and the operation read the same stored value. The body twin (already single) stays byte-identical. | Ch 04 §4 (single evaluation of guarded operands) |
| FR-2 | DEF-142 | A slice built only as an inline argument (`len(slice(mkS(), 0, 2))`) defines its `ClSlice_*` typedef and builds strictly. | Ch 17 (typedemission is total over emitted types) |
| FR-3 | DEF-215 | A slice type argument written inside a lifted-lambda specialization defines its typedef; the specialization's instance joins the collection. | Ch 17, Ch 06 §6 (specialization) |
| FR-4 | DEF-216 | A slice typedef whose element is a generic struct instance names a declared instance type; the struct instance is emitted before the typedef that references it. | Ch 17 |
| FR-5 | DEF-150 | `len(flag ? b.s : b.s)` in a generic body folds to the field's extent, and the fold evaluates nothing (the condition and the arms' stores never emit). | Ch 04 §5 (`len` is structural for fixed arrays), the batch F structural-fold contract |

### Call and member surfaces

| # | Defect | Requirement | Frozen reference |
|---|--------|-------------|------------------|
| FR-6 | DEF-147 (+DEF-213) | A call — dot-call or direct — whose written argument count does not match the callee's parameter list reports one arity diagnostic per site, using the code ruled under AR-2. A matching call is unchanged. | Ch 04 §8 (a candidate function must match the argument list), Ch 04 §7 |
| FR-7 | DEF-148 | A member callee that resolves to no field, function, or intrinsic reports `E10212` for every receiver kind (raw, plain struct, module, bare type parameter). The field-wins rule (PF-016) and accepted calls are unchanged. | Ch 04 §8, the E10212 registry row |
| FR-8 | DEF-146 | A value-form member read on a bare type parameter reports `E10212`; the call form's report is unchanged. | Ch 04 §8 |
| FR-9 | DEF-145 | A `T` iterable is judged at each instance: a substituted type that cannot be iterated reports `E10043` once (the first offending instance), and a struct-element array instance either builds correctly or is rejected by the pinned element rule. The concrete-array body twin defines the expected emission. | Ch 05 (`for (v in xs)` walks a fixed array or a slice), the E10043 registry row |
| FR-10 | DEF-144 | A `str` or slice argument at a `cstring` parameter reports `E10200`; literals and `String` handles keep their accepted, building behavior. | Ch 04 §8/§5 (the implicit conversions), Ch 12 (the FFI crossing model) |
| FR-11 | DEF-154 | The embedded `toCString` accepts a `str` argument, per the frozen appendix; `String` callers stay accepted and emit byte-identically. | Ch 08 §`toCString`, appendix-stdlib:249 |

### Null-shaped values

| # | Defect | Requirement | Frozen reference |
|---|--------|-------------|------------------|
| FR-12 | DEF-151 | A null-shaped local (inferred from `null`) used where a concrete value is required — returned, passed as an argument, or iterated — reports `E10012` instead of emitting `void *` misuse. | Ch 03 §6, the E10012 registry row |
| FR-13 | DEF-149 | A store into a null-shaped local of a non-null-shaped value reports `E10012`; `null` stores, comparisons, copies between null-shaped locals, and nullable-typed targets stay accepted. | Ch 03 §6 |

### Diagnostic hygiene

| # | Defect | Requirement | Frozen reference |
|---|--------|-------------|------------------|
| FR-14 | DEF-153 | One site reports one state diagnostic: a moved or uninitialized place's update reports `E10020`/`E10021` exactly once, keeping the first. | the one-report-per-site convention |
| FR-15 | DEF-160 | A by-value capture moved into an owned local reports `E10401` (not `E10034`) at an owning instance; a duplicable instance stays accepted. | Ch 06 L-3, the E10401 registry row |
| FR-16 | DEF-161 | A block-bodied composite capture return reports exactly one `E10401`; the expression-bodied twin is unchanged. | Ch 06 L-3 |
| FR-17 | DEF-162 | An escaped-view `E10030` naming a compiler-generated owner renders a display phrase instead of `__cl_ownedN`; the registry template is unchanged. | the E10030 registry row |

## Non-Functional Requirements

| # | Requirement |
|---|-------------|
| NFR-1 | **Shape gating**: units whose shapes do not hit a fix emit byte-identically (the per-phase corpus differential classifies every delta). |
| NFR-2 | **Strict builds**: every accepted unit builds under `-Wall -Wextra -Werror` in host, freestanding, and hosted-amiga emission. |
| NFR-3 | **Determinism**: changed shapes re-emit byte-identically on a second run. |
| NFR-4 | **One report per site**: a rejected program keeps a single diagnostic at the offending site (no cascades). |
| NFR-5 | **Verification**: `make check` and `make verify` pass; the scoped suites pass plain and, where ownership is touched, under `RDC_ASAN=1`. |

## Acceptance Criteria

1. Every row in the index reaches **Done** with a specification test that failed at its phase
   baseline and passes at the phase head.
2. The full corpus differential per phase shows only intended, classified deltas.
3. `make verify` passes at the close-out head.
4. The roadmap, the findings ledger, and `AGENTS.md` are synced; the plan closes at 33/33.
