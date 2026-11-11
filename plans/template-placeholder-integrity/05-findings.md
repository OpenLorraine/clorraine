# Findings and Evidence: Template Placeholder Integrity

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)
> **Created**: 2026-11-11 15:21
> **Status**: Phase 1 implemented; phase reviews and the full gate follow

## 1. Phase 1 — Nullable and Generic Placeholder Duties

### 1.1 Red Evidence (phase-start `8b4bff2`)

The new spec suite (`compiler/tests/template_placeholder_integrity_spec_test.rs`)
ran against the phase-start compiler. `cargo test -p clc --test
template_placeholder_integrity_spec_test`:

- 7 red failures: `st1_nullable_string_placeholder_is_rejected`,
  `st2_nullable_str_placeholder_is_rejected`,
  `st4_nullable_call_placeholder_is_rejected`,
  `st23_nullable_field_placeholder_is_rejected`,
  `st26_nullable_generic_instance_is_rejected`,
  `st32_non_formattable_generic_instance_is_rejected`,
  `st33_opaque_generic_member_is_rejected`.
- 7 behavior guards passed: `st3`, `st5`, `st22`, `st24`, `st27`, `st34`,
  `st36`.
- The base accepted the red programs (exit 0) or crashed when run; the tests
  failed on the missing diagnostic, exactly as the plan expected. ST-33 was
  accepted because no instance is emitted; an instantiated member would fail
  the C build.

### 1.2 Implementation

- `compiler/src/own/expr.rs`: `eval`'s `Template` arm now applies
  `check_value_into(inner, &str, inner.span)` and records a
  `RequiresTemplatePlaceholder` duty for a parameter-typed placeholder,
  including per-arm records for a conditional.
- `compiler/src/sema/expr.rs::check_template`: an opaque (`is_unknown` and not
  null-shaped) placeholder is rejected with E10500; the null literal keeps its
  separate path.
- `compiler/src/ir/mono.rs`: `RequiresTemplatePlaceholder` /
  `TemplatePlaceholderArm`; `compiler/src/own.rs` transfers the duty;
  `compiler/src/own/instance_check.rs` reports E10500/E10010 per instance;
  `compiler/src/sema/mod.rs` exposes `formattable`.

### 1.3 Green Evidence

`cargo test -p clc --test template_placeholder_integrity_spec_test`:

```
running 14 tests
... 14 passed; 0 failed
```

`cargo test -p clc --test template_placeholder_integrity_impl_test`:

```
running 4 tests
... 4 passed; 0 failed
```

The impl suite pins IT-11 (one E10010 at the placeholder line, no C) and
IT-12 (generic nullable E10010 at the placeholder, non-formattable E10500, and
a narrowed instance emitting the copy).

### 1.4 Phase Review

| Review | Verdict | Findings |
|--------|---------|----------|
| Correctness | first round: 🟠 RV-001, 🟡 RV-002–RV-006 | RV-001/RV-002/RV-003/RV-004/RV-006 fixed; RV-005 accepted (see below) |
| Semantics | first round: 🟠 SR-001, 🟡 SR-002–SR-004 | SR-003/SR-004 fixed; SR-001 resolved with base evidence; SR-002 accepted as a display nit |
| Memory safety | no defect in the phase diff | all accepted probes ASan/UBSan-clean, C byte-identical to base; two rejected probes write no C |

**RV-001/SR-001 — null-shaped conditional arm.** A `String?` conditional arm
whose other arm is `String` was accepted by the instance duty and could reach
`cl_string_copy(NULL)`. The arm loop now treats a null-shaped (`Unknown` +
nullable) arm as an unnarrowed possible-null use and reports E10010 at the arm
(ST-37). Fixed.

**SR-001 — opaque member rejection.** The semantics reviewer reported that
`show<T>` with `$"{x.name}"` runs on base and should be supported per instance.
The direct base probe on `8b4bff2` (`build/perf/base-clc`) fails the C build
with `cl_text_unknown`; it is not a working program. The opaque member is
therefore rejected with E10500 at the declaration (no new working program is
lost), and the `clone(x.name)` wrapper still works because its placeholder type
is `String`. Resolved with evidence; no regression.

**RV-002 — duplicate E10010.** The read walk and the placeholder check could
report the same site twice. `Diagnostics::has_error_at` records one kept
diagnostic per code/span; a discarded speculative report does not suppress the
reporting pass. Fixed.

**RV-003 — E10500 noise after an earlier error.** The opaque rejection is now
gated on a recorded `opaque_projection` at the placeholder, so `n.foo` on an
`i32` keeps only its E10212. Fixed.

**RV-004/SR-002 — documentation and display.** The `check_template` doc now
states the opaque rejection; the opaque diagnostic uses the frozen
`type '{type}'` template, which renders `?` for the unresolved type. The `?`
display is a cosmetic registry nit, accepted (no new code).

**RV-005 — null-view test coverage.** The old #25 case used an unnarrowed
`str?` null view, which is now E10010. The test is replaced with a valid empty
view; the `ptr == NULL, len == 0` branch is no longer reachable from safe code,
and the scalar kind-code invariant remains pinned by the #25 impl tests and the
runtime contract. Accepted as a coverage note.

**RV-006 — exact spans.** The impl test now pins the placeholder columns
(`5:16`, `4:40`); the ST-33 rejection asserts the frozen message text.

**SR-003/SR-004 — dedupe and E10500 count.** The instance dedupe key now
includes the file; a conditional reports one E10500 for the whole placeholder
and keeps E10010 per arm. Fixed.

**Pre-existing residuals (outside this phase, AR-21).** `$"{null}"` and a
mixed-nullability `c ? a : b` (`String` / `String?`) still fail at the type or
C-build boundary. They are recorded for the closeout follow-up issue; the
generic conditional null arm is now rejected (ST-37).

### 1.5 Green after fixes

`cargo test -p clc --test template_placeholder_integrity_spec_test` 15/15;
`..._impl_test` 4/4; `def_zero_null_spec_test` 44/44; `make check` green
(including `fuzz-smoke` 28/0).

## 2. Phase 2 — Ordered Placeholder Evaluation

_Pending._

## 3. Acceptance

_Pending._
