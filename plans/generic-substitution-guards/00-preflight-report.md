# Preflight Report: Generic Substitution Guards

> **Status**: ✅ PASSED — all 7 findings resolved (2 critical, 2 major, 3 minor)
> **Iteration**: 2 (re-scan after fixes)
> **Artifact**: Implementation plan at `plans/generic-substitution-guards/` (8 documents)
> **Artifact hash at scan start**: `sha256:57d2fa56b1f307e8b16ec598f0d8159bb6445c95f9d77952082b77c0dd316af8`
> **Codebase Grounded**: 14 source files + 2 test files examined; 18 references verified
> **Last Updated**: 2026-09-26 22:25
> **Scope**: strict (no `--auto-design`, no `--explore-scope`)

> ⚠️ **SAME-SESSION REVIEW**: this plan was created in the current session. Same-agent
> bias risk is elevated. A new-session or human review is recommended before execution.
> The critical findings below were independently re-verified by a challenger subagent.

### Codebase Context Summary

**Tech Stack:** Rust (std-only workspace, `#![forbid(unsafe_code)]`), C99 runtime
shims, Clorraine sources. Pipeline: `driver` → `load` → `resolve` → `sema` →
`own` → `codegen`.
**Architecture:** `own::check` lowers every `Item::Func` once, then collects
`Monomorphization` via `generics::collect` (AST side) and
`generics::collect_ir_instances` (IR side). Codegen consumes `functions` plus the
instance set. The plan adds an audit module after instance collection.

**Key Files Examined:**
`compiler/src/driver.rs`, `compiler/src/own.rs`, `compiler/src/own/generics.rs`,
`compiler/src/own/program.rs`, `compiler/src/own/lower.rs`,
`compiler/src/ir/mono.rs`, `compiler/src/ir.rs`, `compiler/src/types/mod.rs`,
`compiler/src/types/table.rs`, `compiler/src/sema/items.rs`,
`compiler/src/sema/expr.rs`, `compiler/tests/ownership_drop_spec_test.rs`,
`compiler/tests/ownership_drop_impl_test.rs`, `spec/14-diagnostics.md`.

**Reproductions run (host `cc`, `clc` in the SDK container):**
`Mixed<str>` → 1×E10205; `Wrap<str>`/`Wrap<u8[]>`/`Wrap<str?>` → accepted;
`fillIt<String>` → accepted + ASan double-free; `rep<String>` → accepted;
`Pair<str,u8[]>` → accepted (2 expected post-fix); `make<str>("hi");` discarded →
invalid C, `Wrap<str>` not recorded; bound form → recorded; `build<str>` nested →
invalid C, `Wrap<str>` not recorded; lambda fill in a generic → invalid C.

### Summary by Dimension

| # | Dimension | Findings | Highest |
|---|-----------|----------|---------|
| 1 | Ambiguities | 1 | 🟡 |
| 2 | Implicit Assumptions | 1 | 🟠 |
| 3 | Logical Contradictions | 1 | 🔴 |
| 4 | Completeness Gaps | 3 | 🔴 |
| 5 | Dependency Issues | 0 | — |
| 6 | Feasibility Concerns | 1 | 🟠 |
| 7 | Testability | 3 | 🔴 |
| 8 | Security Blind Spots | 0 | — |
| 9 | Edge Cases | 0 | — |
| 10 | Scope Creep Indicators | 0 | — |
| 11 | Ordering & Sequencing | 0 | — |
| 12 | Consistency | 1 | 🟡 |
| 13 | Codebase Alignment | 2 | 🔴 |

### Summary by Severity

| Severity | Count | Status |
|----------|-------|--------|
| CRITICAL | 2 | 2 pending |
| MAJOR | 2 | 2 pending |
| MINOR | 3 | 3 pending |
| OBSERVATION | 0 | — |

---

### PF-001: ST-7 expects two `E10205` lines that the pipeline can never produce 🔴 CRITICAL

**Dimension:** Testability (7), Logical Contradiction (3), Codebase Alignment (13)
**Location:** `07-testing-strategy.md` ST-7 (line 44) and the count claim at lines 82-84;
contradicts `03-01-instantiation-checks.md` Error Handling row "The program already
failed the type stage → the ownership stage does not run".
**Codebase Evidence:** `compiler/src/driver.rs:292-302` runs `own::check` only when
`diagnostics.error_count() == 0`. ST-7's program has `part: u8[]`, a concrete view
field that sema rejects at `compiler/src/sema/items.rs:39`. Compiling
`target/def12/pf/mixed.clo` emits exactly one `E10205` (for `part`); the audit never
runs for this program.
**The Problem:** The mandatory oracle ST-7 can never reach its expected state, so the
plan's defined green phase is unreachable and execution blocks. The duplicate-guard
scenario it was designed to pin cannot occur through a compile test at all.

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Replace ST-7 with a sema-clean two-error program (`Pair<str, u8[]>`), keep a separate single-line concrete control | Achievable; exercises the audit's per-field reporting | Loses the "concrete + substituted" pairing narrative |
| B | Keep `Mixed<str>` but expect one line; add a separate two-error test | Minimal edit | The duplicate-guard rationale is still not exercised |
| C | Drop the two-line requirement entirely | Smallest | R3 gets no positive coverage here (worsened by PF-004) |

**Recommendation:** Option A — `Pair<str, u8[]>` is verified sema-clean and produces
one audit diagnostic per parameterized view field; it is the only shape that yields two
`E10205` lines.

**User Decision:** Resolved — User accepted recommendation: Option A

---

### PF-002: ST-5's discarded call does not record `Wrap<str>`, so the audit cannot report it 🔴 CRITICAL

**Dimension:** Completeness Gaps (4), Testability (7), Codebase Alignment (13)
**Location:** `07-testing-strategy.md` ST-5 (line 42); `01-requirements.md` R4
("discarded results"); `02-current-state.md` instance-collection claim.
**Codebase Evidence:** `generics::collect_ir_instances`
(`compiler/src/own/generics.rs:464-469`) records a `Statement::Call`'s instance but
never its result type; only `walk_value` records `value.ty` (`:523-524`). Compiling
`target/def12/pf/st5.clo` (`make<str>("hi");`) yields C with
`static ClWrap__str cl_make__str(...)` but no struct definition, and
`cc -fsyntax-only` fails with `unknown type name 'ClWrap__str'` — `Wrap<str>` is absent
from `mono.struct_instances()`. The bound form
`let w: Wrap<str> = make<str>("hi");` does record it (`target/def12/pf/st5_let.c`).
**The Problem:** ST-5's expected `E10205` is unreachable; the plan's R4 claim that
discarded results are covered is false.

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Narrow ST-5/R4: use a form that is recorded for the audit test, and record discarded-return discovery as an out-of-scope gap (like AR-5) | Smallest; keeps the fix bounded | Drops the discarded-result coverage claim |
| B | Derive struct instances from each function instance's substituted return/parameter types and record them (signature-derived discovery) | Actually covers discarded returns and nested cases | Extends instance discovery beyond the two guards; needs an out-of-scope decision |
| C | Record `Statement::Call.result.ty` when present | Small, localized | The discarded call's result is `None`, so this does not cover ST-5 |

**Recommendation:** Option A for the strict-scope fix, with Option B named as the
separate instance-discovery extension if the user wants the discarded form covered.

**User Decision:** Resolved — User accepted recommendation: Option A

---

### PF-003: R4/R5 overstate completeness — IR-only nested instances and lifted-lambda fills are not audited 🟠 MAJOR

**Dimension:** Implicit Assumptions (2), Completeness Gaps (4), Codebase Alignment (13)
**Location:** `01-requirements.md` R4/R5; `03-01-instantiation-checks.md` lines 98-100;
`00-ambiguity-register.md` AR-11.
**Codebase Evidence:** the field fixpoint runs inside `generics::collect`
(`compiler/src/own/generics.rs:97-109`, called from `compiler/src/own.rs:117`),
strictly before `collect_ir_instances` (`compiler/src/own.rs:118`), so IR-discovered
struct instances are never field-expanded. `target/def12/pf/nested_generic.clo`
(`build<str>` returns `Outer<str>` containing `Wrap<T>`) emits
`struct ClOuter__str { ClWrap__str w; }` with no `ClWrap__str` definition, and
`Wrap<str>` is not in the audit input. A capturing lambda inside a generic function
building `[; a]` (`target/def12/pf/lam_inline_string.clo`) is lifted to a separate
`ir::Function` (`compiler/src/own.rs:113`) and emits unsubstituted `ClT xs[2]`.
**The Problem:** both cases already emit invalid C, so this is the same
instance-discovery family as the AR-5 gaps — not a silent double-free. But R4's "nested
... through the field expansion" and R5/AR-11's "no other body can carry a parameter
into a fill" are false, and ST-4's rationale ("recorded through the field expansion")
is misleading (ST-4 passes only because `Wrap<str>` is spelled).

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Narrow R4/R5/ST-4 and record the IR-only nested and lifted-lambda cases as out-of-scope instance-discovery gaps | Smallest; honest scope | Nested `Outer<str>` stays rejected-by-invalid-C only |
| B1 | Re-run the struct field fixpoint after IR collection (own.rs ordering) | Small; fixes the nested case | Changes instance discovery beyond the two guards |
| B2 | Monomorphize lifted lambdas | Covers the lambda case | Large; a separate defect, not an audit-walk fix |

**Recommendation:** Option A (strict scope). Name B1 as the smallest real nested fix
and record B2 separately as a monomorphization defect.

**User Decision:** Resolved — User accepted recommendation: Option A

---

### PF-004: ST-8 and ST-14 are vacuous; R3/AR-7 has no valid spec coverage 🟠 MAJOR

**Dimension:** Testability (7), Completeness Gaps (4)
**Location:** `07-testing-strategy.md` ST-8 (line 45), ST-14 (line 56), lines 82-84;
`01-requirements.md` R3.
**Codebase Evidence:** both programs contain a concrete illegal field/fill that sema
rejects, so `own::check` is skipped (`compiler/src/driver.rs:292-302`) and the expected
single diagnostic is produced entirely by sema. Verified: `Mixed<String>` → one
`E10205` (`part`), no `v` line; a concrete `String[2]` fill in `f<T>` → one `E10215`.
Both pass **before** implementation, violating the red-first gate (`07` line 126).
**The Problem:** these tests pass for the wrong reason and cannot catch a broken
audit; the plan's only claimed coverage of the no-duplicate rule is invalid. The
stated R3 rationale ("avoids a duplicate diagnostic") is moot because the stage gate
already prevents an audit error from coexisting with a declaration-site error.

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Reclassify as declaration-site regression controls; test R3 with a sema-clean multi-field/multi-instance program; move the concrete-gate assertion to unit tests | Honest; keeps regression value | R3 wording must change |
| B | Keep them but remove the "pins the no-duplicate rule" claim | Minimal | R3 still has no positive spec coverage |

**Recommendation:** Option A. The parameter-mention gate remains a sound, harmless
defensive measure; its justification should be restated as "keep the audit scoped to
substituted types", not "avoid a duplicate".

**User Decision:** Resolved — User accepted recommendation: Option A

---

### PF-005: AR-3/R8 "reuse the registry text and help" is imprecise for `E10215` 🟡 MINOR

**Dimension:** Ambiguities (1)
**Location:** `00-ambiguity-register.md` AR-3; `01-requirements.md` R8.
**Codebase Evidence:** `spec/14-diagnostics.md:99` gives one combined help
("annotate the array type; construct each element for a move-only fill"), while the
declaration-site fill guard emits the move-only help
`"a fill cannot repeat a move-only element; construct each element"`
(`compiler/src/sema/expr.rs:708-714`) — a different string from both the registry and
the extent error at `:672-678`. ST-9 asserts only the message line, so the mismatch is
invisible.
**The Problem:** "the exact registry text and help" is ambiguous for `E10215`; the
audit could pick the extent help.
**Options:** A) pin the audit's help to the sema fill help
(`expr.rs:713`); B) pin it to the registry row's combined help.
**Recommendation:** Option A — match the existing declaration-site fill diagnostic so
the message/help pair is consistent.

**User Decision:** Resolved — User accepted recommendation

---

### PF-006: Minor codebase reference drift and a malformed register row 🟡 MINOR

**Dimension:** Consistency (12), Codebase Alignment (13)
**Location:** `00-index.md` and `02-current-state.md` cite
`compiler/src/ir/mono.rs:105`; `Monomorphization` is at `:106`. `00-ambiguity-register.md`
AR-16 has a User Decision cell reading only "Resolved" and was not part of the bulk
acceptance.
**Codebase Evidence:** `compiler/src/ir/mono.rs:105` is the derive attribute; the struct
is line 106.
**The Problem:** a drifted citation and a register formatting/consistency nit.
**Options:** A) fix the citation to `:106` and restate the AR-16 decision cell.
**Recommendation:** Option A.

**User Decision:** Resolved — User accepted recommendation

---

### PF-007: The `compiler and language` domain lens is not selected for the phase review 🟡 MINOR

**Dimension:** Feasibility (6), scope fit
**Location:** `99-execution-plan.md` phase "Lenses" lines (`correctness`;
`regression, documentation`).
**Codebase Evidence:** the change is a type-checker/instance-semantics change
(substitution and duplicability), which the CodeOps domain selection maps to the
**compiler and language** lens.
**The Problem:** the phase review would not run the compiler-semantics domain lens, so
substitution/duplicability edge cases are less likely to be caught downstream.
**Options:** A) add the compiler/language domain lens to Phase 1's review.
**Recommendation:** Option A.

**User Decision:** Resolved — User accepted recommendation

---

## Iteration 2 — Re-scan After Fixes

All accepted fixes were applied to the plan documents and re-checked:

| PF | Fix applied | Verification |
|----|-------------|--------------|
| PF-001 | ST-7 replaced with sema-clean `Pair<str, u8[]>`; two-line requirement kept; count-helper note added | `pair_view.clo` re-confirmed sema-clean; the audit has two parameterized view fields to report |
| PF-002 | ST-5 changed to a bound `let w: Wrap<str> = make<str>("hi");`; R4 narrowed; discarded-return gap recorded | `st5_let.c` defines `struct ClWrap__str`; `st5.clo` recorded as the out-of-scope gap |
| PF-003 | R4/R5/ST-4/03-01 narrowed; IR-only nested and lifted-lambda gaps recorded as AR-19 | `nested_generic.c` and `lam_inline_string.c` re-confirmed invalid-C, not silent |
| PF-004 | ST-8/ST-14 reclassified as declaration-site controls; R3 rationale restated; unit tests own the gate | Both controls verified to trip sema before `own::check` |
| PF-005 | E10215 help pinned to `sema/expr.rs:713` in R8, 03-01 step 4, and the error table | `spec/14:99` vs `expr.rs:713` re-read |
| PF-006 | `mono.rs:106` fixed in 00-index, 02, 03-01, and the register; AR-16 row restated | Grep clean |
| PF-007 | Domain lens added to Phase 1 | `99` line 64 |

**Regression check:** no stale claims remain in the target (grep for
"discarded results", "field expansion", "no other body", "mono.rs:105" is clean
outside this report); all AR references resolve to AR-1..AR-19; the red/control
sets in `99` match the reclassified ST cases; no new finding was introduced.

## Verdict

✅ **PREFLIGHT PASSED — all 7 findings resolved** (2 critical, 2 major, 3 minor;
zero remaining). The roadmap row advances to **Plan Preflighted**.
