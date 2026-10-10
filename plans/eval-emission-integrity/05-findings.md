# Findings: Evaluation and Emission Integrity

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)
> **Protocol**: per-phase reviews follow the `AGENTS.md` dispatch matrix; each finding gets a
> ruling (fixed in-batch, filed as a roadmap row, or dismissed with evidence) before its phase
> closes.

## Phase 1: Module-initializer integrity (DEF-117, DEF-121, DEF-122)

**Reviewers**: correctness reviewer (RV), emission-integrity auditor (EIA), memory-safety auditor
(MS). Diff reviewed: `135cfc5..655d7bd`.

### Fixed in-batch (the review's blocking findings)

| # | Sev | Finding | Ruling |
| - | --- | ------- | ------ |
| RV-001 / EIA (O1) | 🔴 | The new `\|\|` restructure initialized its result local to `false`, so a true left operand yielded `false`. | **Fixed**: the result local starts at the operator's left-decided value (`false` for `&&`, `true` for `\|\|`); both spellings are pinned. |
| RV-002 / EIA-1 / MS (O1) | 🔴 | `materialize_guarded_sub_operands` hoisted lazy-position operand stores (`?:` arms, `&&`/`\|\|` right sides) before the construct, so a skipped operand's call ran — a behavior regression for previously working programs. | **Fixed**: the merged walk stores operands under branch guards (`if (c) { … }`) and never hoists out of a lazily evaluated position; side-effect counts are pinned. |
| RV-003 / MS (O4) | 🔴 | Two materialization passes ran before the render, so a right-side bind ran before an earlier left-side operand store (left-to-right order violated for newly accepted shapes). | **Fixed**: one left-to-right walk (`materialize_init_value`) binds and stores at each sub-expression's source position; an order-counting pin covers the mixed case. |
| RV-004a / EIA-2 | 🟠 | The short-circuit restructure dropped its enclosing guard, so a skipped arm's left operand ran. | **Fixed**: the decision combines the enclosing guard with the left operand (`if ((guard && left))` / `(guard && !left)`) and the left's own binds/stores stay under the guard. |
| RV-004b / EIA-3 | 🟠 | The short-circuit left operand was not single-evaluated (its guards re-read impure operands), and the right side's operands ran unstored. | **Fixed**: the left renders through embedded guards (`guarded_value`) over stored operands; the right side is fully walked inside the decision block. |
| EIA-4 / RV-P2 (fill and scalar literal positions) | 🟠 | Fixed-array calls inside array-literal elements, struct-literal fields, and template placeholders still emitted destination-less calls (base-identical, so pre-existing, but inside the changed lane). | **Partially fixed**: the walk descends `ArrayLit`, `StructLit`, and `Template`, and the probes build and run (literal element, row literal, template argument). A struct-literal fixed-array *field* still renders an invalid compound literal (`(ClBox){ __cl_itmp0 }`; the same spelling fails for a placed field with no call at all, and the function body emits the statement-form store) — that remaining renderer gap is filed as **DEF-134**. |
| EIA-5 | 🟠 | A fill conditional's condition was stored without binding a fixed-array call inside it. | **Fixed**: the condition is walked before its store, in both the fill and scalar paths. |
| RV-P3 | 🟡 | `collect_init_slice_types` skipped written intrinsic type arguments (`sizeof(i32[])`). | **Fixed**: the `IntrinsicArg::Type` arm resolves and collects. |
| RV-P4 | 🟡 | The literal-slice fix added a follow-on `E10200` next to the fill's `E10215`, and a view-return spelling now reports `E10200` instead of the signature-level `E10036`. | **Partially accepted**: the extent replacement is scoped to fill-free literals, so a fill target keeps its single `E10215`; the view-return spelling's `E10200` is kept — the literal is not a view, the conversion report is the accurate one, and both spellings reject. Recorded as the fix's documented consequence. |
| MS (F3) | 🟡 | The `run_memory`/`run_asan` helpers were unused, so the module-initializer shapes never ran under the sanitizer even with `RDC_ASAN=1`. | **Fixed**: the Phase-1 specification cases run through `run_memory` (ordinary strict host build by default, ASan with `RDC_ASAN`). |

### Fixed as a consequence of the redesign

| # | Sev | Finding | Ruling |
| - | --- | ------- | ------ |
| RV-P1 / MS (F2) / EIA (P-3) | 🟠 | Impure operands under call arguments and inside array-literal elements evaluated multiple times (bounds/division guard read plus the rendered read), pre-existing at base. | **Fixed by the merged walk**: call arguments and literal elements are walked at their positions, so their impure operands store once. Probes cover the argument, element, and divisor spellings. |

### Filed (pre-existing classes outside the phase's rows; new rows on the roadmap)

| # | Sev | Finding | Filed as |
| - | --- | ------- | -------- |
| MS (F1) | 🟠 | Module initializers never release owned temporaries; the phase's new bindings extend the pre-existing leak class to newly runnable positions (member projection with an owned member, `String` element projection, an owned conditional arm). | DEF-130 |
| EIA-6 | 🟠 | A closure or func-typed-global callee in a module initializer emits an unresolved or empty callable (`(__cl_itmp0);`, or an undeclared plain name for a func-typed global). | DEF-131 |
| EIA (P-1) | 🟠 | A generic function call in a module initializer emits the plain module symbol; no instance is declared. | DEF-132 |
| EIA (P-2) | 🟠 | A callable-field call in a module initializer drops the receiver. | DEF-133 |
| EIA-4 (struct-literal remainder) | 🟡 | A module-initializer struct literal with a fixed-array field from a place emits an invalid compound literal; the function body emits the statement-form store. | DEF-134 |

### Verification evidence (fix round)

All reviewer reproductions re-run after the redesign: `\|\|` left-true result (exit 1), arm and `&&`/`\|\|` skip laziness (side-effect counts 0), left-to-right order (log 12), enclosing-guard skip (`f` not called), left single evaluation (one call), literal element (3), row literal (3), fill condition (4), template argument (1), call-argument operand (one call, value 7), `\|\|` skip/take (10/20 pair), plus the full specification and implementation suites and `make check`.

### Dismissed with evidence

| # | Finding | Evidence |
| - | ------- | -------- |
| EIA (P-4) | Freestanding `bool` missing. | Already the batch's DEF-90 (Phase 2); the auditor's base-identical finding confirms the row's evidence. |

**Review budget**: one scoped re-review after the fix round (correctness + emission lenses on the
redesigned walk).
