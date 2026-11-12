# Preflight Report: Template Placeholder Residuals III

> **Audit target**: `plans/template-placeholder-residuals-iii/` (7 documents)
> **Artifact hash**:
> `12a4f9fb76fe9aa8b82248bdc54dd3d7f9371ab0f37a5a2d4b4df9c5c29ea648`
> (SHA-256 of the seven plan documents after the final resolution round)
> **Reviewed**: 2026-11-12
> **Mode**: auto-design; three independent read-only auditor rounds plus
> codebase probes on `541ba27`
> **Verdict**: ✅ PREFLIGHT PASSED — all 2 critical-adjacent design defects and
> 10 major findings resolved; no accepted risk

> ⚠️ SAME-SESSION REVIEW: the plan was authored in this session. Independence
> was provided by separate read-only auditor dispatches (document soundness,
> codebase grounding, and a final verification round); the reconciliation
> re-verified every finding against `541ba27` with direct probes.

## First Round (blocked)

| PF | Severity | Finding | Resolution |
|----|----------|---------|------------|
| S-1 | MAJOR | Task counts said 36 = 16/12/8; the checklist had 35 = 16/11/8 | Corrected to 35 |
| S-2 | MAJOR | ST-5/ST-13 were guards but the alias breaks the strict C build on base | Reclassified red |
| S-3 | MAJOR | ST-11 (module `let t` + import alias) is E11103 duplicate | Replaced with a lexical shadow |
| S-4 | MAJOR | The planned `NullValue` settlement variant is inert: a no-destination both-null site is not opaque | Dropped the variant; decision is documented + guard-tested |
| S-5 | MAJOR | Red-evidence task claimed both-null red; no such case exists | Removed; both-null cases are guards |
| S-6 | MAJOR | ST-16 had no type parameter, so it never tested the deferred condition flag | Replaced with a generic bool settlement; added `while`/`assert` |
| S-7 | MAJOR | The qualified path could hijack a user `std.fmt.text` declaration | Added real-declaration gates in sema and own |
| G-1 | 🔴 CRITICAL | Canonicalizing to the bare `text` name is captured by user-symbol resolution | Use the existing `$text` sentinel, rendered before user resolution |
| G-2 | MAJOR | The qualified gate missed a real global/ffi/function | Added `qualified_decl_kind` / `table.global` / target-decl gates |
| G-3 | MAJOR | `len(cstring null)` is already E10010, so the shim guard is not observable there | Dropped the `len` claim; scoped to the panic path |
| G-4 | MAJOR | The new duty drain was placed in `check_module`, which has no `SemaOutput` | Moved to the per-module loop at `sema/mod.rs:478-486` |
| G-5 | MAJOR | ST-5/ST-13/ST-11 corrections as S-2/S-3 | Applied |
| G-6 | MINOR | The `NullValue` variant would be dead code | Dropped |

## Final Round (passed after corrections)

| PF | Severity | Finding | Resolution |
|----|----------|---------|------------|
| F-1 | MAJOR | An aliased user `std.fmt` `ffi text` was still classified as the intrinsic | The alias gate checks `qualified_decl_kind` and `qualified_ffi_signature`; own rejects `Callee::Ffi` |
| F-2 | MAJOR | A bare user `std.fmt` `ffi text` fell through to the intrinsic | The bare gate checks the imported and target `ffi` declaration; IT-11 covers it |
| F-3 | MINOR | Freestanding shim has no `NULL` and uses `unsigned long` | Its guard is `text == 0`; IT-7 says "null-test" |
| F-4 | MINOR | `is_builtin_text_call` is private to `own::expr` | Made `pub(super)` |
| F-5 | MINOR | Canonicalizing the bare spelling would make the `match "text"` arm unreachable | Canonicalize every intrinsic form and remove the name-based arm; a user `ffi text` keeps its own symbol |
| F-6 | MINOR | R9 still traced red ST-5/ST-13 | Corrected |
| F-7 | MINOR | Both-null wording overclaimed "null arms never make a site opaque" | Qualified to the no-destination shape |
| F-8 | MINOR | ST-17 is red by acceptance, not an extra E10012 | Corrected |
| F-9 | MINOR | The null-comparison audit would accept an `Unknown` substitution | Added the unknown/param skip |
| F-10 | MINOR | The index claimed one shared predicate | Corrected to matching stage-local predicates |

## Grounded Evidence

| Check | Result on `541ba27` |
|-------|----------------------|
| `import { text as t }` / `std.fmt.text(7)` | accepted; emit undeclared `t`/`std.fmt.text` |
| user `std.fmt.text` function | accepted; calls the user symbol |
| user `std.fmt` `ffi text`, bare or aliased | the intrinsic wins on base (the gap fixed by F-2/F-1) |
| `if`/`while`/`assert`/`&&` generic condition at `T = i32` | E10012 (the concrete twin is E10218) |
| both-null inferred/nullable/typed/placeholder/condition | accepted / accepted / E10012 per arm / E10500 / E10218 |
| `null == x`, `x == null`, `x != null` at `T = i32` | accepted; pointer/integer C |
| `T = String`/`str`/`Box<i32>?` | accepted; valid C |
| `panic(cstring null)` | UBSan null `strlen`; freestanding panic is length-guarded |
| `len(cstring null)` | E10010 (not a runtime defect) |
| fixed-array for-step call/member | compact header; strict C rejects it |
| `$text` render order | handled at `codegen/expr.rs:1170` before user resolution |
| E10218 concrete span | conditional start (matches `conditional.span`) |

## Verdict

- No critical or major finding remains unresolved.
- The plan has 7 documents, a passed 15-row ambiguity register, 35 tasks
  (16/11/8) in spec-first order, 45 specification cases, and 12 implementation
  cases with consistent AR/ST/IT/R references.
- The implementation may begin.
