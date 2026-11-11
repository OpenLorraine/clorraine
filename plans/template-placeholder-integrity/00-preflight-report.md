# Preflight Report: Template Placeholder Integrity

> **Audit target**: `plans/template-placeholder-integrity/` (all plan documents)
> **Artifact hash**: recorded at scan start from the working tree
> **Reviewed**: 2026-11-11
> **Mode**: auto-design; independent auditor dispatches plus a same-session
> reconciliation
> **Verdict**: ✅ PREFLIGHT PASSED — all critical/major and minor findings
> resolved; no accepted risk

> ⚠️ SAME-SESSION REVIEW: the plan was authored in this session. Independence
> was provided by four separate read-only auditor dispatches; the final
> reconciliation was done against the real code (file:line evidence below).

## Codebase Context

The plan targets the template lowering in two paths:

- Function bodies: `compiler/src/sema/expr.rs::check_template` types the
  template; `compiler/src/own/expr.rs:1123` walks placeholders without a
  nullable check; `compiler/src/own/lower.rs:754` embeds placeholder values;
  `compiler/src/codegen/expr.rs:1620` renders the builder call.
- Module initializers: `compiler/src/codegen/program.rs:3619`
  (`lower_global`), `:2927` (`materialize_init_value`), `:3381` (Template
  arm), `:2293` (`materialize_init_local`).

The relevant frozen rules are `spec/04-expressions.md` §3 (strict
left-to-right), `spec/02-types.md` TY-6 (nullable use), and
`spec/16-borrow-rules.md` §1–2 (views/loans). The runtime builder contract is
in `compiler/runtime/shim_host.c` (`cl_str_part_owned`, `cl_string_concat`)
and mirrors in `shim_hosted_amiga.c`; no shim change is planned.

## Grounded Evidence Gathered

| Check | Result |
|-------|--------|
| Nullable `String?` body placeholder | exit 139 on `8b4bff2` |
| Nullable `str?` body placeholder | empty line, exit 0 (still an unnarrowed use) |
| Module nullable template | already E10010 (`def_zero_null_spec_test:1510`) |
| `str` call placeholder | `calls=2` |
| Take sibling / view over move | exit 139 / ASan heap-use-after-free |
| Reverse view order | already E10022 |
| Generic String placeholder + take | exit 139 |
| Generic nullable `show<String?>(null)` | exit 139 |
| Generic non-formattable `show<H>(h)` | internal "emitted C failed to compile" |
| Generic opaque member `x.name` | internal "emitted C failed to compile" |
| View conditional / static-str projection | `calls=2` |
| String conditional / String projection | `calls=1` (no residual) |
| Module `str` call placeholder | `calls=2` |

## Findings and Resolutions

| PF | Severity | Finding | Resolution |
|----|----------|---------|------------|
| PF-01 | 🔴 CRITICAL | `eager_conditional_arm` is not a statement position; an eager `?:` arm write would run on the skipped path | Every `?:`/short-circuit uses deferred fills; the plan's §3.3 and §6 agree |
| PF-02 | 🟠 MAJOR | Re-wrapping an already-bound `Materialize` source double-registers a String release | Positive source predicate; owning carriers/projections/derefs are never re-wrapped |
| PF-03 | 🟠 MAJOR | `expression_has_effects` is AST-only but the module walk holds IR values | Module pre-scan uses `init_operand_is_pure` over the lowered pieces |
| PF-04 | 🟡 MINOR | `str?` is accepted today, not a crash | Red evidence and AR-2/AR-13 corrected; the tightening is explicit |
| PF-05 | 🟡 MINOR | `$"{null}"` claims were false | AR-21 records it as out of scope; `03-01 §5` corrected; follow-up task added |
| PF-06 | 🟡 MINOR | The `toHex` benchmark placeholder is already once | Benchmark expectation restated as neutral |
| PF-07 | 🟡 MINOR | The `str` copy arm is a defensive fallback | Kept and documented; the raw String arm copies, the `owned` arm adopts |
| PF-08 | 🟡 MINOR | `05-findings.md` was missing at review time | Created in task 1.3.2 before the first review |
| PF-09 | 🟡 MINOR | PLAN-30 row already exists | Task 3.2.2 updates it to Done |
| PF-10 | 🔵 OBS | Copy carrier used `mark_destination_temp` | Removed; it matches `consume_place_into` (no mark, no release) |
| PF-11 | 🔵 OBS | `init_call_locals` insertion unstated | Stated; IT-7 pins no `FreeString` of the copy local |
| PF-12 | 🔵 OBS | Loop-condition helper / R7 duplicate | Documented; R7 merged |
| PF-13 | 🟠 MAJOR | §6 contradicted §3.3 on eager `?:` arms | §6 now uses deferred fills only |
| PF-14 | 🟠 MAJOR | Generic nullable instantiations crash | Per-instance `RequiresTemplatePlaceholder` duty; ST-26/27 |
| PF-15 | 🟡 MINOR | Missing String-source/guarded cases | ST-28/29/30 added |
| PF-16 | 🟡 MINOR | `pub(crate)`/re-export text | Changed to `pub(super)` |
| PF-17 | 🟡 MINOR | Wrong ST citation | ST-13 |
| PF-18 | 🟠 MAJOR | A projection over an already-bound `Materialize` could double-release | The positive view predicate replaces the `rooted_in_materialize` walk; projections are bound by `bind_member_receiver` |
| PF-19 | 🟠 MAJOR | Deferred projections/conditionals/derefs could be re-wrapped | The positive predicate wraps only direct calls/intrinsics and impure **view** values; owning shapes are never wrapped |
| PF-20 | 🟠 MAJOR | Generic non-formattable/opaque placeholders ICE | The duty judges E10500; sema rejects an opaque placeholder (excluding the null shape) |
| PF-21 | 🟡 MINOR | ST classification | Red/guard lists corrected |
| PF-22 | 🟡 MINOR | Duty plumbing/IT/task references | `ir/mono.rs`, `ir.rs`, `own.rs`, `instance_check.rs`, `sema/mod.rs`, name/dedupe/arm records; IT-12; task ranges fixed |
| PF-23 | 🟡 MINOR | "Before lowering" wording and missing ST refs | Changed to "after lowering the generic template but before emission"; ST lists completed |
| PF-M1 | 🟠 MAJOR | ST-23 expected the field name, but a place-rooted member names the root | ST-23 expects `h`; 03-01 §2 corrected |
| PF-M2 | 🟠 MAJOR | The opaque rejection would also reject `$"{null}"` | The rejection excludes the null-shaped (`Unknown` + nullable) type; AR-21 keeps the follow-up |
| PF-N1 | 🟠 MAJOR | Opaque member `x.name` is not `is_unknown`? It is; `x[0]` is an element param | `x.name` is rejected; `x[0]` is per-instance and covered by ST-36 |
| PF-N2 | 🟠 MAJOR | A generic conditional with narrowed arms was falsely rejected | Per-arm records judge each arm; ST-34 |
| PF-N3 | 🟠 MAJOR | A view projection over a fresh call evaluated the root twice | The view-impure predicate materializes it once; ST-35 |
| PF-N4 | 🟡 MINOR | Red/guard labels | Corrected |
| PF-N5 | 🟡 MINOR | `sema/mod.rs` exposure, task references, malformed ST row | Fixed |

## Verdict

- No 🔴 CRITICAL or 🟠 MAJOR finding remains unresolved.
- No 🟡 MINOR finding remains unresolved; the remaining observations are
  design notes, not defects.
- The plan has the required documents, a passed ambiguity register, 37 tasks
  (12/16/9) in spec-first order, and consistent AR/ST/IT references.
- The implementation may begin.

## Codebase References

- `compiler/src/own/expr.rs:1123`, `:1427`
- `compiler/src/own/lower.rs:754`, `:1365`, `:2646`, `:3438`, `:5530`, `:7129`
- `compiler/src/own/call.rs:1463`
- `compiler/src/ir/mono.rs:228`, `:644`
- `compiler/src/own/instance_check.rs:61`, `:2442`
- `compiler/src/codegen/expr.rs:1160`, `:1556`, `:1620`
- `compiler/src/codegen/program.rs:2293`, `:2927`, `:3381`, `:3410`, `:3619`
- `compiler/src/sema/expr.rs:1766`, `:1864`; `compiler/src/sema/mod.rs:31`
- `compiler/runtime/shim_host.c:335`, `:432`; `shim_hosted_amiga.c:922`
