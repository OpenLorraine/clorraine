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
