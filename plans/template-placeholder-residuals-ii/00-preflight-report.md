# Preflight Report: Template Placeholder Residuals II and `text` Arity

> **Audit targets**: `plans/template-placeholder-residuals-ii/` (8 documents) and
> `plans/text-arity/99-execution-plan.md`
> **Artifact hash**:
> `21e8a647f0b8d02212b13cade70064f7abc0bf0891dd9e8b15f1fdc0b7dfb3c8`
> (SHA-256 of the nine plan documents after the third resolution round)
> **Reviewed**: 2026-11-12
> **Mode**: auto-design; three independent read-only auditor rounds plus
> codebase probes on `bb3956c`
> **Verdict**: ✅ PREFLIGHT PASSED — all 1 critical and 13 major findings
> resolved; all minor findings resolved; no accepted risk

> ⚠️ SAME-SESSION REVIEW: both plans were authored in this session.
> Independence was provided by separate read-only auditor dispatches
> (document soundness, codebase grounding, and two verification rounds); the
> reconciliation re-verified every finding against `bb3956c` with direct
> probes.

## Codebase Context

The plans target:

- Static diagnoses: `compiler/src/sema/call.rs:276` (explicit `text`),
  `compiler/src/sema/ops.rs:115` (binary operands), `:449`
  (`check_conditional`), `:616` (`judge_conditional_argument`),
  `compiler/src/types/mod.rs:1055` (`conditional_result`), `:1094`
  (`null_arm_needs_report`), `compiler/src/own/instance_check.rs:2510`
  (deferred placeholder duty), `:2819` (`null_conditional_report`).
- Lowering/runtime: `compiler/src/own/loops.rs:273` (`check_for_c`), `:999`
  (`for_step_is_structured`), `compiler/src/codegen/control.rs:549`
  (`for_step_text`), `compiler/src/codegen/function.rs:1721` (`emit_assign`),
  `compiler/src/codegen/function.rs:3003` and
  `compiler/src/codegen/guards.rs:1413` (initializer order), and
  `compiler/runtime/shim_{host,hosted_amiga}.c` (`cl_panic`).
- `text` arity: the same `sema/call.rs:276` branch and the three
  `codegen/expr.rs` `text_call` sites.

The frozen rules are E10218 (non-bool condition), E10308 (arity), E10500
(formattability), E10010 (possible-null use), E10012 (null at a concrete
position), O7 (zero = empty), and Ch 04 §3 (source order).

## Grounded Evidence Gathered

| Check | Result on `bb3956c` |
|-------|----------------------|
| `text(null)` | accepted; `cl_text_unknown(NULL)`; C build fails |
| `text(Box { n: 1 })` | accepted; `cl_text_Box` undeclared |
| generic `show<Box>` with `text(x)` | accepted; `cl_text_Box` undeclared |
| `text(s)` with `s: String?` | accepted |
| `text(x.n)` over a parameter | accepted; `cl_text_unknown` |
| `$"{null + 1}"`, `null + null` | accepted; invalid pointer arithmetic C |
| `null && flag` | E10218 (frozen; must stay) |
| generic `flag ? null : x` at `str`/`String` | E10012+E10010 |
| `let x = flag ? null : 5;` | accepted; `(flag ? NULL : 5)` |
| `let q: str = c ? null : v;` | E10012 (pre-existing annotated rule) |
| for-step owning assignment | ASan 49 bytes leaked in 3 allocations |
| for-init template/call/increment | effect does not run; `$"{tick()}"` prints `0` |
| same/differing aggregate order | `12` / `21` on the host |
| `panic(null view)` | UBSan null-pointer `fwrite` report |
| `text()` / `text(1, side())` | accepted; `text()` fails the C build; `side()` dropped |

## Findings and Resolutions

| PF | Severity | Finding | Resolution |
|----|----------|---------|------------|
| PF-01 | 🔴 CRITICAL | The new null rule would turn `null && flag` into E10012, contradicting the frozen E10218 condition rule | `LogicalAnd`/`LogicalOr` are excluded from the null rule; ST-30 pins E10218 (AR-4) |
| PF-02 | 🟠 MAJOR | `null + null` still passed because the report guards require the other operand non-null | Add a both-null branch for non-`==`/`!=`, non-logical operators; ST-33/IT-15 pin E10012 once |
| PF-03 | 🟠 MAJOR | `accepts_bare_null` did not exist; `is_nullable_capable` includes handles/`Box` and would misclassify the `String` arm | Define `accepts_bare_null` (views/`raw`/`cstring`/slice only) with unit test IT-13; own `null_capable_destination` delegates |
| PF-04 | 🟠 MAJOR | The report would not double-report `(c ? 1 : null) + 1` without a guard | `judge_conditional_argument` skips a site whose `null_reported` is set; IT-14 pins one report |
| PF-05 | 🟠 MAJOR | A generic `String` null arm would get E10012+E10010 from two duties | The instance conversion duty owns E10012; the placeholder duty owns E10010; the split matches the call order |
| PF-06 | 🟠 MAJOR | A generic scalar/struct null arm would keep a spurious E10010 | Settlement classification leaves only the E10012; ST-13 pins the scalar case |
| PF-07 | 🟠 MAJOR | `text(x.n)` over a parameter still emitted `cl_text_unknown` | Reuse the placeholder opaque-projection gate in the `text` branch; ST-7 pins E10500 |
| PF-08 | 🟠 MAJOR | The frozen E10500 help was changed to a text-only variant | Use the exact frozen help; R10/03-01/IT-1 align |
| PF-09 | 🟠 MAJOR | The own-stage `text` detection was unspecified; `resolved_callee("text")` is `Callee::Stdlib`, so an "unresolved" test never fires | Specify plain name + no local + `Callee::Stdlib { name == "text" }`, mirroring sema shadowing |
| PF-10 | 🟠 MAJOR | The for-init change also makes increments run, an unclaimed behavior change | R11/AR-11/03-02 state every effectful init now runs; ST-32 pins the increment |
| PF-11 | 🟠 MAJOR | The Amiga panic stream/coverage and vbcc execution were overclaimed | Each shim keeps its stream; host runtime + Amiga parity pin/build; vbcc is build-only and contract-only |
| PF-12 | 🟠 MAJOR | ST-11 selected the null arm but expected `x`; ST-21 was vacuous | ST-11 uses `false`; ST-21 increments an observable counter and expects `1` |
| PF-13 | 🟠 MAJOR | A written view destination is E10012, contradicting an unqualified R4 acceptance claim | R4/03-01 are qualified to no-expected-type contexts; ST-31 pins the annotated guard; the PLAN-31 distinction is unchanged |
| PF-14 | 🟠 MAJOR | A placeholder over a non-formattable `raw`/`cstring`/slice/`Box` arm is E10500, not accepted | R4/03-01 state the whole-value E10500 (and possible E10010); ST-35 pins the cstring guard |
| PF-15 | 🟡 MINOR | Test case ranges were off by one (ST-17–ST-24 vs ST-18–ST-25) | Corrected in 99 §2.1 and the task list |
| PF-16 | 🟡 MINOR | Generic nullable `text` E10010 had no oracle | ST-28 (red) and ST-29 (guard) added |
| PF-17 | 🟡 MINOR | #24 help diverged from the frozen E10308 help | Use `pass one argument per parameter` |
| PF-18 | 🟡 MINOR | The mini-plan had no independent review and omitted bench evidence | T-06 adds correctness + emission-integrity review; T-08 runs the full gate including `bench-compare` |
| PF-19 | 🟡 MINOR | Stale citations/claims (`emit_assign :1611`, "eval never lowers", `known-limitations`, present-tense vbcc/Amiga pins) | Corrected in 02/03-02/99 |
| PF-20 | 🟡 MINOR | IT-13 was untraced; 1.3 cited AR-10; the #28 baseline did not state PLAN-33 ordering | Trace and refs fixed; Phase 1 baseline and Dependencies state PLAN-33 executes first |
| PF-21 | 🟡 MINOR | `text_call` has three call sites, not one | T-04 names all three |
| PF-22 | 🟡 MINOR | The index omitted `null + 1`, listed a removed IR flag, and said `Implements R1–R9` | Index corrected to R1–R11 and the final file map |

## Verdict

- No 🔴 CRITICAL or 🟠 MAJOR finding remains unresolved.
- All 🟡 MINOR findings are resolved; no finding is accepted as risk.
- The plans have all required documents, a passed 21-row ambiguity register
  (PLAN-32) and 8-task mini-plan (PLAN-33), 38 tasks (16/14/8) in spec-first
  order for PLAN-32, and consistent AR/ST/IT references.
- The implementation may begin.

## Codebase References

- `compiler/src/sema/call.rs:276`; `compiler/src/sema/ops.rs:115`, `:449`, `:616`
- `compiler/src/types/mod.rs:711`, `:1001`, `:1055`, `:1094`
- `compiler/src/own/expr.rs:1141`, `:1555`, `:1631`
- `compiler/src/own/instance_check.rs:2510`, `:2819`
- `compiler/src/own/loops.rs:273`, `:999`
- `compiler/src/codegen/control.rs:549`
- `compiler/src/codegen/function.rs:1721`, `:3003`; `compiler/src/codegen/guards.rs:1413`
- `compiler/src/codegen/expr.rs:1171`, `:1280`, `:1561`, `:1685`
- `compiler/runtime/shim_host.c:56`; `compiler/runtime/shim_hosted_amiga.c:560`;
  `compiler/runtime/shim_freestanding.c:240`
- `spec/04-expressions.md:132-139`; `spec/14-diagnostics.md:127`, `:146`
