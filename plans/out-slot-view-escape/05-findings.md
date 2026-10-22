# Findings Ledger — out-slot-view-escape

| Finding | Phase | Severity | Area | Ruling |
|---|---|---|---|---|
| RV-001 | 0 | MAJOR | spec/16-borrow-rules | fixed |
| RV-002 | 0 | MINOR | spec/14-diagnostics | fixed |
| RV-003 | 0 | MINOR | plans/out-slot-view-escape | fixed |
| RV-004 | 5 | MAJOR | own/expr — duplicate E10034 on read-only nullable owned stores | fixed |
| RV-005 | 5 | MAJOR | own/call_check — syntactic withholding bit | fixed |
| RV-006 | 5 | MAJOR | own/instance_check — generic take-view parity | fixed |
| RV-007 | 5 | MAJOR | own/expr — raw-pointer classification | fixed |
| RV-008 | 5 | MAJOR | eval impl — lost unused-owned carrier acceptance | fixed |
| RV-009 | 5 | MINOR | own/expr — capture report names the capture | report-only |
| RV-010 | 5 | MINOR | own/expr — conditional foreign arm precedence | fixed |
| RV-011 | 5 | MINOR | compiler/tests — names and docs after retargeting | fixed |
| RV-012 | 5 | CRITICAL | own/call_check — withheld caller report accepted a dangling view | fixed |
| RV-013 | 5 | CRITICAL | own/statements — generic local copy lost the foreign root | fixed |

## Phase 5 review notes

- **RV-012 / RV-013 (critical, UAF).** The withholding bit now resolves the
  stored value's source through local initializers and conditional arms
  instead of matching any name anywhere in the value. A static field or a
  conditional condition no longer sets the bit. The generic local-copy form
  records its source separately (`Binding::copy_source`), so
  `let t: T = v; slot = t` reports one `E10034` at the instance instead of
  reaching the caller-side guard. The reviewer's `leak<T>` reproduction now
  fails at the store.
- **RV-004.** A read-only nullable owned store is left to the existing
  lend-move duty; the concrete whole-slot branch reports only view-like and
  raw sources. The new exact-one test covers a `String?` source.
- **RV-005.** The bit resolves `Qualified` roots (the AST spelling of
  `h.name`), follows local copies, skips static-typed fields and conditional
  conditions, and understands an immediately-called lambda. New cross-frame
  tests cover a local copy and a static-field decoy.
- **RV-006 / RV-007.** A generic `take` view reports `E10030` after
  substitution; raw pointer sources use the whole-slot view path in both the
  concrete and generic judgments. Raw owning `take` remains accepted.
- **RV-008.** `pqunusedowned` stores a static value again and keeps its
  original emission/run acceptance; the unused owned argument remains the
  test's subject.
- **RV-009 (minor, report-only).** A captured local copy names the capture
  (`s`) rather than the original parameter (`p`). The one-report and
  provenance behavior is correct.
- **RV-010.** A conditional with a foreign arm and a frame-local arm now
  reports the foreign parameter's `E10034` first; a new mixed-conditional
  case pins it.
- **RV-011.** The reversed cross-frame test name, the cross-frame/store docs,
  and the inline-carrier control doc now state the `E10034` behavior.
