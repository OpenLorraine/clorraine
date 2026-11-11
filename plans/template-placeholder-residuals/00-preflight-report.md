# Preflight Report: Template Placeholder Residuals

> **Audit target**: `plans/template-placeholder-residuals/` (all plan
> documents except this report)
> **Artifact hash**: `0f302e0c4cbf71f2106d5eb716810131797ee872ee64e3a61b6d41e872472dfa`
> (SHA-256 of the eight plan documents after the resolution round; this report
> is excluded)
> **Reviewed**: 2026-11-11
> **Mode**: auto-design; two independent read-only auditor dispatches plus a
> same-session reconciliation against the real code
> **Verdict**: ✅ PREFLIGHT PASSED — all 3 critical and 12 major findings
> resolved; all 8 minor findings resolved; no accepted risk

> ⚠️ SAME-SESSION REVIEW: the plan was authored in this session. Independence
> was provided by two separate read-only auditor dispatches (document soundness
> and codebase grounding); the reconciliation below re-verified every finding
> against `b6f12d8` with direct probes.

## Codebase Context

The plan targets five independent paths:

- Type diagnostics: `compiler/src/types/mod.rs:1055`
  (`conditional_result`), `compiler/src/sema/expr.rs:1770` (`check_template`),
  `compiler/src/sema/ops.rs:449` (`check_conditional`), and
  `compiler/src/own/expr.rs:1441` (`check_value_into`).
- Zero-state String: `compiler/runtime/shim_host.c:276,282,305` and
  `compiler/runtime/shim_hosted_amiga.c:893,899,922`; the generated
  `cl_string_view` at `compiler/src/codegen/program.rs:221`.
- Discarded template / for step: `compiler/src/own/lower.rs:1984`
  (`lower_discarded`), `compiler/src/own/statements.rs:159` (release floor),
  `compiler/src/own/loops.rs:332,394` (step expressions).
- Aggregate order: `compiler/src/own/lower.rs:2830`
  (`pin_literal_operand`), `compiler/src/codegen/function.rs:701`
  (`Statement::Adopt`), `:3041` (`emit_struct_literal_store`), `:3078`
  (`emit_struct_literal_fields`).

The frozen rules are O7 (zero = empty), STR-7 (`Point {}`), TY-6 (nullable
use), E10500 (formattability), E10012 (null at a non-nullable destination), and
Ch 04 §3 (left-to-right, fields in source order).

## Grounded Evidence Gathered

| Check | Result on `b6f12d8` |
|-------|----------------------|
| `$"{null}"` | internal "emitted C failed to compile"; `cl_text_unknown(NULL)` |
| `flag ? a : b` (`String` / `String?`, null arm) | exit 139 |
| `let s: String = flag ? a : b;` | accepted; unsound |
| `flag ? null : stringOf("x")` (null arm) | exit 139 |
| `flag ? null : "x"` | empty line; must stay accepted |
| `H {}` placeholder / direct print | exit 139; `cl_string_copy(NULL)` / `cl_string_view(NULL)` |
| bare `$"{1}";` | ASan 20 bytes leaked |
| for-step template | ASan 60 bytes leaked in 3 allocations |
| `P { inner: Inner { a: mkString() }, b: $"{mark(2)}" }` | `seq=21` |
| uninstantiated generic `$"{null}"` body | accepted (no check) |
| `let v: str? = null; $"[{v}]"` | E10010 (#26 rule; not a guard) |

## Findings and Resolutions

| PF | Severity | Finding | Resolution |
|----|----------|---------|------------|
| PF-01 | 🔴 CRITICAL | A blanket nullable OR would type `flag ? null : "x"` as `str?` and reject a program that runs as an empty view | Exclude null-shaped arms from the OR; view/`raw`/`cstring` keep the empty state; ST-4 pins it (AR-2, AR-17) |
| PF-02 | 🔴 CRITICAL | The adopted aggregate's fieldwise store pre-drops fixed-array/owned-conditional fields on loop re-entry, double-freeing the previous iteration | Emit `cl_zero(&temp)` before the fieldwise fill; ST-23/ST-27 pin the fixed-array and owning-conditional loops (AR-24) |
| PF-03 | 🟠 MAJOR | ST-3 used an unnarrowed `str?`, which is E10010 on the base, not a guard | ST-3 uses `let v: str = null;` and prints `[]` |
| PF-04 | 🟠 MAJOR | The 1% fallback removed `cl_string_ptr`/`cl_string_len`, but `len` and `clone` call them directly; R3 would fail | Fallback is now one inlined NULL test in `cl_string_view` with all accessors safe; no accessor removal (AR-22) |
| PF-05 | 🟠 MAJOR | The checklist is 38 tasks (12/17/9), while the header said 39/18 | Header and phase table corrected to 38 (12/17/9) |
| PF-06 | 🟠 MAJOR | ST-5 used an uninitialized `a` (E10021) and ST-7/8 initialized `b` to null | ST-5 initializes both arms; ST-7/8 use `b = stringOf("beta")` and exact stdout |
| PF-07 | 🟠 MAJOR | ST-20–ST-23 had no scaffold and ST-22/23 had no exact output | Shared scaffold, exact `seq=12`, and loop length sums `6` |
| PF-08 | 🟠 MAJOR | ST-25 claimed rejection at the declaration, but an uninstantiated generic body was accepted on the base | ST-25 is the uninstantiated body and expects the new declaration-time E10500; the text is corrected |
| PF-09 | 🟠 MAJOR | A null arm with a handle sibling (`flag ? null : stringOf("x")`) still crashed | The conditional settles to `String?`; the placeholder reports E10010 at the null arm; ST-26 pins it (AR-23) |
| PF-10 | 🟠 MAJOR | The zero-state read rule is language-visible and the plan claimed "no language change" | AR-3 cites issue #27's empty-state direction plus O7/STR-7; R6 now says "no language surface or ABI change" |
| PF-11 | 🟡 MINOR | The Amiga zero-state runtime was claimed but no Amiga runtime test exists | R3 is host behavior; IT-4 pins the Amiga shim parity and `make verify` builds the Amiga leg |
| PF-12 | 🟡 MINOR | Stale "Implements R1–R7", "AR-1–AR-19", "ST-12" verification range, "add PLAN-31" | Corrected to R1–R9, AR-1–AR-24, the full ST range, and "update the existing row" |
| PF-13 | 🟡 MINOR | The new E10500 help diverged from the frozen registry help | Use the registry help verbatim (AR-21) |
| PF-14 | 🟡 MINOR | IT-6 claimed the step loop "uses the flat form" as a fix, but a template/call step is already flat | IT-6 pins only the release; the "degrades" claim is removed |
| PF-15 | 🟡 MINOR | `toCString(h.name)` was claimed as an empty cstring but a NULL cstring is not printable | Removed from R3/03-02 claims; the tested paths are placeholder, print, len, clone |
| PF-16 | 🟡 MINOR | `text(h.name)` and an owning call in a step were outside the stated R3/R4 surface | `text` is tested as the zero-state copy path; the owning call step is the general drain case (ST-18) |
| PF-17 | 🔴 CRITICAL | The own-stage skip of null-shaped arms would accept the concrete twin of #26 ST-37 while the generic duty still rejects it | Removed the skip; a null arm at a nullable result keeps reporting E10010 (ST-28 pins the concrete twin; ST-37 stays green) |
| PF-18 | 🟠 MAJOR | ST-21–ST-23/ST-27 lacked executable bodies and exact sums; `flag` and `mkPair`/`mkOther` were undefined | Shared scaffold defines every helper and `flag`; ST-21's `mkPair` calls `mark(1)`; each loop states the exact summed expression and output `6` |
| PF-19 | 🟠 MAJOR | The view-empty claim was not applied to the written-expected path, and the rule's type scope was unclear | Scope the sibling rule to a no-expected-type placeholder and to `String`/`Box` handles; views/`raw`/`cstring`/slice keep the empty state; the written-expected and generic duties are unchanged |
| PF-20 | 🟠 MAJOR | The removed own-stage skip had no test, and the concrete null arm had no guard | ST-28 pins the concrete `flag ? x : null` E10010 at the null arm, matching #26 ST-37 |
| PF-21 | 🟠 MAJOR | A generic null arm at a resolved view destination keeps the pre-existing E10012/E10010 behavior, while R1/AR-17 claimed view-empty acceptance without a scope | R1/AR-17/03-01 are scoped to a placeholder with no expected type; the type-stage flag targets handle/`Box` results, and the generic/written destination rule is explicitly unchanged and #26 ST-37 stays protected |
| PF-22 | 🟡 MINOR | `00-index.md:130` still listed the removed own-stage skip | The line is removed; the modified-file list matches 03-01/99 |
| PF-23 | 🟡 MINOR | ST-27 lacked `flag`; the loop sums needed a `usize` accumulator; ST-15 needed the `text` import; ST-18 lacked exact stdout; ST-24 pointed at #26 instead of being self-contained | Every row now declares `flag`, the loop scaffold declares `sum: usize`, ST-15 imports `text`, ST-18 prints `done`, and ST-24 spells the generic function |

## Verdict

- No 🔴 CRITICAL or 🟠 MAJOR finding remains unresolved.
- All 🟡 MINOR findings are resolved; no finding is accepted as risk.
- The plan has the required documents, a passed 24-row ambiguity register,
  38 tasks (12/17/9) in spec-first order, and consistent AR/ST/IT references.
- The implementation may begin.

## Codebase References

- `compiler/src/types/mod.rs:1055`, `:711`, `:1077`
- `compiler/src/sema/expr.rs:1770`; `compiler/src/sema/ops.rs:449`
- `compiler/src/own/expr.rs:1441`, `:1493`
- `compiler/src/own/lower.rs:1984`, `:2572`, `:2830`, `:2875`
- `compiler/src/own/loops.rs:332`, `:394`, `:995`
- `compiler/src/own/statements.rs:159`
- `compiler/src/codegen/function.rs:701`, `:3041`, `:3078`
- `compiler/src/codegen/program.rs:221`
- `compiler/runtime/shim_host.c:276`, `:282`, `:305`
- `compiler/runtime/shim_hosted_amiga.c:893`, `:899`, `:922`
