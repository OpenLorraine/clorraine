# Findings Ledger: Module Initializer Integrity

> **Batch**: F — the 9 module-initializer defects (DEF-130 – DEF-135, DEF-137, DEF-143, DEF-152)
> **Plan**: [99-execution-plan.md](99-execution-plan.md) · **Register**: [00-ambiguity-register.md](00-ambiguity-register.md)

## Phase 1: Initializer call resolution (DEF-131, DEF-132, DEF-133)

**Reviewers**: correctness reviewer, semantics reviewer, emission-integrity auditor, over the
diff `a97725b..6e084c8`.

### Merge table

| Finding | Severity | Area | Ruling |
| ------- | -------- | ---- | ------ |
| EIA-001 | 🔴 critical | The new field-receiver branch fired for a qualified function path: `a.b.c.func()` rendered `(a.b.c.func)(...)` in an initializer where the base built the named symbol — a regression. | Fixed: the callable branch requires a value projection (`Place`/`Member`/`Index`), so a declared function path keeps the named-call lowering; spec case `qualifiedcall`. |
| RV-001 | 🟠 major | A non-func-field dot call (`a.add1()` calling a module function) dropped its receiver; the design promised the body's receiver-as-first-argument mirror. | Fixed: `init_dot_receiver` mirrors `lower_dot_receiver`; spec case `dotmethod`. |
| RV-002 = SR-001 + SR-002 | 🟠 major | The resolved callable lost its signature: func-typed globals substituted the mangled symbol and field receivers carried an unknown type, so `String`→`str` widenings, `borrow` pointers, and null typing were skipped and body-accepted initializers failed the strict build. | Fixed: the callable is passed as the receiver value with its resolved signature (`init_callable_value`); spec cases `globalwiden`, `fieldwiden`. |
| RV-003 | 🟠 major | A callable field named like an intrinsic (`b.len()`) was intercepted by the intrinsic gate and emitted a silent `0`. | Fixed: the gate excludes callable values (`init_callable_value`); spec case `fieldlen`. |
| RV-004 | 🟡 minor | The spec suite's doc header was copied from the callable suite (the wrong feature and nonexistent groups). | Fixed: the header names this batch's groups; the staged helpers serve their phases. |
| RV-005 | 🟡 minor | DEF-172 (a lambda-valued func-typed global emitted NULL) is fixed by the lambda-value path but unrecorded. | Recorded here; the roadmap row syncs at close-out; spec case `lambdavalue` and its impl pin. |

### Filed (pre-existing classes; new roadmap rows)

| # | Severity | Finding |
| - | -------- | ------- |
| DEF-196 | 🟠 major | An initializer call whose callee is an expression (a call result `g()(5)`, a conditional `(c ? f : g)()`, or a lifted lambda whose result is called) emits invalid or silently wrong C; the body spelling handles the call-result callee. |
| DEF-197 | 🟠 major | An inferred generic call at an initializer's top level (`pick(1, 2)`) emits the plain template symbol with no instance; only explicit type arguments instantiate. |
| DEF-198 | 🟡 minor | A call-site marker (`take`) in an initializer call is accepted where the body twin reports one `E10039`. |
| DEF-199 | 🟡 minor | A closure-call argument needing the `str` widening (`((s: str) => …)(stringOf(…))`) emits the raw handle in both body and initializer spellings. |
| DEF-200 | 🟡 minor | A qualified function value reference (`let f: func(): i32 = A.B.helper;`) emits `A.B.helper` as a C member path. |
| DEF-201 | 🟡 minor | A constant zero divisor inside a lambda body emits a literal `/ 0` that fails the strict build under `-Werror=div-by-zero`. |

### Fix-round verification

Spec suite 10/10 (the four filed shapes plus the six review counterexamples); impl 5/5 (the
func-global call re-pinned to the callable-style call; the lambda-value identity pinned);
`make check` 60 suites green. Every reviewer counterexample re-run at head: the qualified call,
the module-function dot call, both signature widenings, and the intrinsic-named field build and
run; every original Phase 1 shape stays green.

### Scoped re-review (`6e084c8..91c5847`)

One scoped round (correctness + emission-integrity).

| Finding | Severity | Area | Ruling |
| ------- | -------- | ---- | ------ |
| RV-001 (re-review) | 🟠 major | `init_dot_receiver`'s qualified branch resolved the whole receiver path through `resolve_global`, whose module-path branch binds a module qualifier as a global: a call like `acme.utils.last()` prepended the exported global `utils` (`too many arguments`, or a silently shifted argument where the arity coincidentally matched). Base built it. | Fixed: the head-only resolution mirrors the body (`resolve_name_global(head)` plus field projections); the `guardedpath` pair spec case pins the collision (red at the fix base). |
| EIA-001 (verification) | — | Verified fixed: every qualified-call profile (`a.b.c.func`, `Demo.MinMax.clamp`, `std.text.stringOf`, cross-module, nested) is byte-identical to the batch base at head. | Closed. |
| RV-001..RV-005 (re-verification) | — | All verified closed: the dot receiver, both widenings, the intrinsic-named field, the pins, and the doc header. | Closed. |
| EIA-002 | 🟠 major | Owned argument temporaries of initializer calls are never materialized or released (host-ASan leak; root cause pre-existing; the body binds and releases the same argument). | Filed DEF-204; the batch's Phase 3 release work extends to argument materialization. The `lambdavalue` spec case now uses a non-owning spelling so the suite's ASan path stays green. |
| Re-review P1/P2 | — | The initializer qualified fixed-array field `.len` fold (DEF-143's filed class) and the body's unmangled global field receiver (DEF-170's filed class). | Already filed; noted. |
| Re-review P3 / N1 | 🟡 | The func-typed fixed-array global declarator and the initializer global-borrow acceptance. | Filed DEF-202 and DEF-203; the body-side view-builder shadow filed DEF-205. |

### Fix-round 2 verification

Spec suite 11/11 plain and under `RDC_ASAN=1`; impl 5/5; `make check` 60 suites green.
Direct probes: the two-file collision builds and runs (the receiver is not prepended); every
other receiver and widening shape stays green; the qualified-call family is byte-identical to
the batch base.

## Phase 2: Initializer aggregate emission (DEF-134, DEF-135, DEF-143)

**Baseline**: `f81235752c2d5fabe88b20b54083c82853b17251` (the `77c268d` tree). Implementation committed before the review.

### Deliverables

| # | Shape | Result |
| - | ----- | ------ |
| DEF-134 | A struct literal whose fields need statements (a fixed-array field, a deferred fill, a nested literal) binds into an init local and stores per field with `memcpy` for array fields — the body's literal-store predicate and shape, in every position: whole-init, nested field, array element, conditional arm under its branch guard | Fixed; spec cases `structstore`, `structstorecall`, `nestedstore`, `guardstore` |
| DEF-135 | A `str`/`static str` destination takes an owned `String` value through the view constructor (`templates` and String-valued calls) | Fixed; spec cases `templateview`, `strviewcall` |
| DEF-143 | A qualified dot-`len` receiver walks the field projections, so a fixed-array field folds to its extent; the receiver value now also resolves only the head segment (a module path that spells a segment like an exported global stays a named call) | Fixed; spec case `foldlen`; the shared helper now serves both the call receiver and the fold |

### Red evidence

- Store shapes: `missing braces around initializer` / `initialization of 'int' from 'i32 *'` on the emitted compound literals.
- Str values: `incompatible types when assigning to type 'ClStr' from type 'void *'` (template) and `... from type 'ClString'` (the call).
- Fold: `n = 0`, run exit 254 where 2 is expected.
- The nested and conditional-arm positions were probe-verified red before the fix (`C BUILD FAILED` on the same errors) and are green after.

### Implementation notes

- **Shape convergence (intended).** A struct literal with a fixed-array field now takes the statement store in every case, including array-literal fields (`BoxB { arr: [1, 2] }`) whose compound-literal spelling built before. This follows the body's predicate exactly (`struct_literal_needs_stores`), so the initializer and body emissions agree; `make check` (60 suites) shows no pinned shape changed.
- **Guard placement.** The initializer walk emits each operand's guard where the operand is evaluated, so the literal store does not re-emit them; a guarded (conditional-arm) literal stores under its branch guard and single-emits the operand guards — verified with a run-time division probe (the trap fires, one check). One corner remains: an unguarded fixed-array field built from an array literal re-emits an element's guard at the store core (correct but redundant) — filed **DEF-206** with the analysis (the store core's guard emission is what makes the guarded path single, so the fix is a deliberate flag, not a removal).
- **The collapse bonus.** `global_len_receiver` and the dot receiver now share one helper; the module-path collision class fixed in Phase 1's re-review cannot reappear through the fold.

### Review and fix round (`77c268d..6fc0bbb`)

**Correctness reviewer** found one phase regression:

| Finding | Severity | Area | Ruling |
| ------- | -------- | ---- | ------ |
| RV-001 | 🟠 major | A needs-stores struct literal as the receiver of a folded `.len()` (`BoxA { arr: [1, 2], tag: 3 }.arr.len()`) was materialized by the walk while the renderer folded the intrinsic without reading it: `-Werror=unused-but-set-variable` at the phase head, where the batch base folded it cleanly. | Fixed: the fixed-array fold now happens at lowering time (`lower_global_intrinsic`), returning the extent as a value before any materialization — the body's structural-fold semantics. The discarded receiver's operands are never evaluated (a division inside it no longer traps, matching the body twin), and slice/`Array` receivers still keep their runtime value. Spec cases `literalfold` and `folddiscard`; impl pin asserts the folded constant and zero init locals. |

The same reviewer verified every other deliverable, the guarded-path guard placement (single, lazy), the unchanged emissions of pure conditional/logical shapes, and the test discriminability.

**Cross-batch pin updated**: `store_emission_integrity_impl_test.rs`'s `init-intrinsic-len` pin expected the renderer's bare `3`; the shared fold now spells the honest `usize` constant `3u` (consistent with a written literal). Re-pinned with that note — the store-emission suite is otherwise untouched.

### Scoped re-review (`6fc0bbb..39adc5d`) and its fix round

| Finding | Severity | Area | Ruling |
| ------- | -------- | ---- | ------ |
| RV-002 | 🟠 major | The sibling spelling `len(BoxA { arr: [1, 2], tag: 3 }.arr)` still materialized the receiver while the renderer folded it away (`-Werror=unused-but-set-variable`), and a discarded receiver's division trapped where the body folds without evaluating. | **Ruling (auto-design, delegated):** fix in-phase — the same class as the already-fixed RV-001 and a mechanism decision mirroring the body's structural fold. Fixed: both spellings fold through one helper (`fixed_array_len_extent`); the free spelling folds at its lowering arm. Spec cases `freelenfold`, `freelendiscard`; impl pin asserts the constant and zero init locals. |
| RV-003 | 🟡 minor | A stray escape in a new doc comment (`body\'s`). | Fixed. |

The same reviewer verified RV-001 closed for the dot spelling (three reproductions strict-built and run; the new cases red at the fix base; the division case also red at the batch base). The emission-integrity audit of `77c268d..6fc0bbb` reported **no findings**: all 13 corpus deltas are the intended DEF-134/135/143 shapes, every strict-build failure is base-identical, and every variant is deterministic.

**Adjacent class closed by the fold work**: the audit's P4 (an initializer `xs[k].arr.len()` with a run-time `k` evaluated the discarded receiver and trapped where the body folds without evaluating) is fixed by the lowering-time fold — verified by probe (exit 0, no bounds guard).

**Filed from the audit's pre-existing classes**: DEF-207 (a nullable-view conditional with a null arm dereferences NULL — the body spelling segfaults at base too) and DEF-208 (a mixed `str`/`String` conditional emits an invalid ternary in both spellings).

### Verification

Spec suite 22/22 plain and under `RDC_ASAN=1`; impl 12/12; `make check` 60 suites green.
