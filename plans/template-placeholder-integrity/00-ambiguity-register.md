# Ambiguity Register: Template Placeholder Integrity

> **Status**: ✅ GATE PASSED — all 22 items resolved
> **Last Updated**: 2026-11-11 14:08
> **Authority mode**: auto-design (root invocation
> `codeops-20261111-1410-template-placeholder-integrity`, policy version 1)

## Register

| # | Category | Ambiguity / Gap | Options Presented | Decision | Status |
|---|----------|-----------------|-------------------|----------|--------|
| 1 | Behavioral | A nullable `String?` placeholder in a function body lowers to `cl_string_copy(NULL)` and crashes | reject with E10010 / format an empty string / keep crashing | Reject with the existing E10010 nullable-use rule, matching the module-initializer path | ✅ Resolved |
| 2 | Behavioral | A nullable `str?` placeholder has the same empty-view risk | reject with E10010 / treat as empty / leave | Reject with E10010, the same rule as AR-1 | ✅ Resolved |
| 3 | Behavioral | A `str`-returning call placeholder is emitted as `(call).ptr` and `(call).len`, evaluating the call twice | evaluate once / accept twice | Evaluate once, at the placeholder's source position | ✅ Resolved |
| 4 | Behavioral | A `String` placeholder is copied at part construction, after a hoisted `take` sibling has already released its source | copy before the sibling / reject the program / leave | Copy at the placeholder's source position, before any later sibling effect | ✅ Resolved |
| 5 | Behavioral | A `str` view placeholder is read after a later `take` sibling released its owner (`$"{v}{consume(s)}"`) | order the read before the move / reject with E10022 / leave | Order the read before the move; the program is well-defined under left-to-right evaluation | ✅ Resolved |
| 6 | Behavioral | The reverse order (`$"{consume(s)}{v}"`) must stay rejected | reject E10022 / accept | Keep the existing E10022 move-while-borrowed rejection and pin it | ✅ Resolved |
| 7 | Scope | Generic type-parameter placeholders (`show<T>(x)`) hit the same crash when instantiated with `String` | include / defer to a new issue | Include: the same source rule applies before instantiation | ✅ Resolved |
| 8 | Technical | How to materialize a string-like placeholder copy in source order | new IR value kind / hidden internal text-copy call / new shim function / reject | Hidden internal copy call rendered by the existing `text_call` converter, bound to a temporary | ✅ Resolved |
| 9 | Technical | Whether pure scalar placeholders also get a temporary | always / only when a later placeholder can reorder them / never | Only when the placeholder has effects or precedes a later effectful placeholder | ✅ Resolved |
| 10 | Scope | Module initializers (`let s: str = $"x{get()}";`) evaluate a view call twice | include / defer | Include the once-only rule; the shared-owner ordering cases are unreachable in a module initializer | ✅ Resolved |
| 11 | Behavioral | Guarded positions (`&&`/`||`, `?:`, loop condition) have the same reordering and double evaluation | include / defer | Include, using the existing deferred-materialization/fill machinery | ✅ Resolved |
| 12 | Technical | Ownership of a module-initializer copy temporary | release at init end / retain for the builder | Retain it: the template part adopts the copy, so the init release pass must not free it again | ✅ Resolved |
| 13 | Compatibility | Whether the language specification or ABI changes | change spec / no spec change | No spec or ABI change: Ch 04 §3 (left-to-right), Ch 02 TY-6, and Ch 16 §2 already require the behavior | ✅ Resolved |
| 14 | Scope | Whether #24 (`text` arity) or the other deferred issues are bundled | bundle / separate | Separate; this plan delivers #26 only | ✅ Resolved |
| 15 | Naming | Plan folder and test-suite names | — | `template-placeholder-integrity`; `template_placeholder_integrity_{spec,impl}_test.rs` | ✅ Resolved |
| 16 | Non-functional | Verification and regression budget | full gate + 1% scored guard / correctness suites only | Full gate (`check`, `asan`, `ubsan`, `verify`, `bench-shape`, `bench-compare`) with the 1% scored guard against `8b4bff2` | ✅ Resolved |
| 17 | Edge cases | Literal-only templates and zero-length strings | special-case / unchanged | Unchanged: a placeholder-free template stays a `str` literal; the #25 zero-length append guard stays | ✅ Resolved |
| 18 | Non-functional | Generic scalar placeholders pay one extra `String` conversion through the copy path | accept / add an instance-specialized fast path | Accept for now; only generic templates pay it. Reopen on measured impact | ✅ Resolved |
| 19 | Security | New input, parser, or unsafe surface | — | None: the copy is an internal lowering of an already-checked expression | ✅ Resolved |
| 20 | UX & presentation | Diagnostic code, message, and help for a nullable placeholder | new code / reuse E10010 | Reuse E10010 with the frozen registry text (`value '{name}' may be null here`) | ✅ Resolved |
| 21 | Scope | `$"{null}"` currently reaches the driver as an internal C-compile error | include / exclude | Exclude from this plan and file a separate follow-up issue at closeout | ✅ Resolved |
| 22 | Technical (runtime) | An eager template copy needs a store that does not pre-drop the previous temporary, while still running at the placeholder's source position | plain Assign / adopting Materialize / new `Statement::Adopt` | Add `Statement::Adopt`: a store with no previous-value release and no guard loss | ✅ Resolved |

## Delegated Resolution Records

All eligible decisions below are recorded under `_shared/auto-design.md` policy
version 1, root invocation
`codeops-20261111-1410-template-placeholder-integrity`. The shared
objective is issue #26: make every template placeholder well-defined or
rejected, with no crash and no use-after-free. Reopen triggers are listed once
at the end; each applies to every row unless a row adds its own.

**AR-1, AR-2 — nullable placeholders**

- Authority: AI — delegated by `--auto-design`.
- Eligibility: behavioral consequence of the already-frozen nullable-use rule
  (`spec/02-types.md` TY-6, `spec/14-diagnostics.md` E10010); no new language
  surface.
- Decision: report E10010 for a nullable `String?`/`str?` placeholder that is
  not narrowed. A narrowed placeholder formats normally. For `str?` this
  tightens a currently accepted program: `let v: str? = null; println($"{v}")`
  prints an empty line on `8b4bff2`; after the fix it is E10010, which is what
  TY-6 already requires.
- Evidence: `compiler/src/own/expr.rs:1123` walks placeholders without the
  check; `compiler/src/own/lower.rs:1365` already applies
  `check_value_into(inner, str)` in the module-initializer path. The issue
  reproduces exit 139 for the function-body case, and
  `def_zero_null_spec_test::should_judge_module_initializer_call_and_template_arguments`
  already pins the module case.
- Rejected: formatting a null as an empty string (invents behavior the spec
  does not define); keeping the crash (unsound).
- Strongest counterargument: users may expect a nullable placeholder to print
  an empty value. The spec's nullable rule is explicit and the module path
  already rejects it, so consistency wins.
- Confidence: High. What would change it: an explicit spec clause granting a
  null placeholder an empty-value meaning.

**AR-3 — `str` call evaluated once**

- Authority: AI — delegated.
- Eligibility: direct consequence of the frozen left-to-right evaluation order
  (`spec/04-expressions.md` §3, EX-3).
- Decision: evaluate the placeholder expression once at its source position.
- Evidence: `compiler/src/codegen/expr.rs:1646` emits `(value).ptr` and
  `(value).len`; the reproduction prints `x calls=2`.
- Rejected: accept double evaluation (observable side effects and a contract
  violation).
- Strongest counterargument: a pure `str` place is harmless to read twice, so
  a narrower fix could special-case calls. The temporary is needed for order
  anyway, so the general fix is smaller.
- Confidence: High.

**AR-4, AR-5 — copy/read before a later move**

- Authority: AI — delegated.
- Eligibility: implementation of the frozen evaluation order and last-use
  loan rule (`spec/16-borrow-rules.md` §1–2); no new rule.
- Decision: materialize a `String`/`str` placeholder's bytes at the
  placeholder's position, before a later sibling can move or release the
  source. `$"{v}{consume(s)}"` stays legal and prints the pre-move bytes.
- Evidence: `compiler/src/own/lower.rs:754` embeds placeholder values in the
  concat expression; a later `take` argument's statement is emitted first.
  The reproduction reports ASan heap-use-after-free; the reverse order is
  already E10022 (`compiler/src/own/expr.rs` move checks).
- Rejected: reject use-then-move templates (contradicts EX-3); defer the
  `take` call (the callee frees the storage during the call).
- Strongest counterargument: rejecting would be simpler, but it forbids a
  well-defined program that the frozen evaluation order permits.
- Confidence: High.

**AR-6 — reverse order stays rejected**

- Authority: AI — delegated.
- Eligibility: preserves an existing frozen diagnostic (E10022).
- Decision: keep and pin the existing rejection.
- Evidence: `u1` (`$"{consume(s)}{v}"`) already reports E10022 on `8b4bff2`.
- Rejected: accepting it (would read freed storage).
- Confidence: High.

**AR-7 — generic placeholders**

- Authority: AI — delegated.
- Eligibility: the same guarantee applied to an already-supported language
  surface; implementation mechanism.
- Decision: include type-parameter placeholders; at instantiation the hidden
  copy uses the concrete type's converter. A per-instance
  `RequiresTemplatePlaceholder` duty judges the substituted type: a
  non-formattable type reports E10500, a nullable type reports E10010 unless
  the declaration narrowed it, and a conditional placeholder judges each arm
  separately so two narrowed nullable arms stay accepted. A bare parameter is
  per-instance; an **opaque** placeholder (`x.name`) whose type cannot be
  named at the declaration is rejected with E10500 rather than emitting a
  nonexistent converter.
- Evidence: `gm1` (`show<String>(s, s)` with a sibling take) segfaults on
  `8b4bff2`; `show<String?>(null)` segfaults because the declaration sees only
  `T`; `show<H>(h)` and `show<T>(x){$"{x.name}"}` fail the C build with
  nonexistent `cl_text_*` calls; the narrowed-arm conditional runs on the base
  and must stay accepted. The duty follows the existing `RequiresLenOperand` /
  `RequiresConditionalArmNarrowing` patterns and the sema formattability
  predicate (`compiler/src/sema/expr.rs:1864`).
- Rejected: deferring leaves the same crash reachable through a generic
  wrapper.
- Strongest counterargument: the generic scalar path now allocates an extra
  `String` (AR-18). Correctness first; the cost is measured and reopenable.
- Confidence: Medium-High.

**AR-8 — materialization mechanism**

- Authority: AI — delegated.
- Eligibility: internal compiler mechanism; no new dependency or layer.
- Decision: lower a string-like placeholder copy to an internal
  `ValueKind::Call` with the reserved callee `$text`; `render_call` maps it to
  the existing `text_call` converter. Bind the copy to a compiler temporary:
  a plain statement temporary in an eager position, an adopting deferred
  `Materialize` in a guarded position. Mark the resulting piece with
  `ValuePiece::Placeholder { owned: true }` so the emitter adopts exactly the
  pre-copied value; any other String piece keeps the defensive copy.
- Evidence: `compiler/src/codegen/expr.rs:1160` renders an unresolved callee
  as written; `text_call` already dispatches on the concrete type;
  `compiler/src/own/lower.rs:3438` (`materialize_value`) and
  `compiler/src/own/lower.rs:2646` (`materialize_guard_operand`) already bind
  values in both positions. The `owned` flag is the provenance mark a `Temp`
  alone cannot carry; without it, a bypassing producer could hand the builder
  an unowned String and double-free it.
- Rejected: a new `ValueKind` variant (many exhaustive walk sites, higher
  regression surface); trusting the type alone (no provenance); a new shim
  helper (runtime/ABI surface, two profiles); rejecting the programs (spec
  violation).
- Strongest counterargument: the `ValuePiece` flag touches 29 match sites.
  The compiler forces every site to be reviewed (struct pattern), and each
  update only carries the flag through.
- Confidence: High.

**AR-9 — scalar materialization policy**

- Authority: AI — delegated.
- Eligibility: performance implementation detail within the accepted behavior.
- Decision: materialize a scalar placeholder when its expression has effects
  or when a later placeholder has effects; otherwise leave it inline.
- Evidence: the `strings` benchmark's loop has two pure scalar placeholders
  (`benchmarks/strings/program.clo:13`); always materializing them adds stores
  in a 2000-iteration loop for no observable difference. The order problem
  only arises from a later hoisted effect.
- Rejected: always materialize (measurable risk on a scored benchmark);
  never materialize (a later hoisted call would still reorder an earlier
  pure read).
- Confidence: High, subject to AR-16's measurement.

**AR-10 — module initializers**

- Authority: AI — delegated.
- Eligibility: same rule in the second lowering path; no new behavior beyond
  once-evaluation.
- Decision: apply once-only materialization to module-initializer templates
  in `compiler/src/codegen/program.rs`.
- Evidence: `let s: str = $"v={mk()}";` prints `2` on `8b4bff2` because
  `compiler/src/codegen/program.rs:3381` maps placeholders directly.
  A module initializer cannot hold a shared `String` owner (module `let`s must
  be duplicable), so the copy-before-move cases are unreachable there.
- Rejected: leave module initializers (the issue's "each placeholder once"
  applies to every evaluation context).
- Confidence: High.

**AR-11 — guarded positions**

- Authority: AI — delegated.
- Eligibility: implementation mechanism using existing guard machinery.
- Decision: cover `?:` arms, `&&`/`||` right operands, and loop conditions.
  Every `?:` arm and short-circuit operand uses the deferred `Materialize`
  fill, even when the enclosing conditional is eager: an eager statement
  emitted while lowering the arm lands above the `?:` and runs on the skipped
  path. `eager_conditional_arm` is not a statement position.
- Evidence: `q2` reproduces the UAF in a `?:` arm; `r1` prints `2` for a
  `str` call in a `&&` right operand; `p1` calls twice per loop test.
  `compiler/src/codegen/guards.rs` emits `Materialize` fills as comma
  expressions in source order and skips the re-fill drop for an adopting
  carrier. `compiler/src/own/lower.rs:2787` states that an eager statement in
  a guarded operand would run on a skipped path.
- Rejected: statement-position-only scope (the same defect remains one
  expression away).
- Strongest counterargument: the guard emitter is intricate; a nested fill
  could be misplaced. The plan requires dedicated guarded-position spec cases
  and an emission-integrity review.
- Confidence: Medium-High. Reopen if a guarded case cannot be ordered without
  a disproportionate guard-emitter change.

**AR-12 — module copy ownership**

- Authority: AI — delegated.
- Eligibility: internal ownership mechanism.
- Decision: bind the init-time copy with `init_retain_owners` set, so the
  release pass does not free a value the template part adopts.
- Evidence: `compiler/src/codegen/program.rs:2376` registers a local in
  `init_owners` unless the destination retains; the template part contract is
  `cl_str_part_owned`.
- Rejected: normal owned local (double free at init end).
- Confidence: High.

**AR-13 — no spec/ABI change**

- Authority: AI — delegated.
- Eligibility: compatibility assessment; no change to the frozen oracle.
- Decision: no `spec/` edit. The plan adds regression tests only. The
  nullable rejection also tightens the currently accepted `str?` placeholder
  (it prints an empty line today) to the E10010 rule the spec already states;
  this is a diagnostic correction, not a new language rule.
- Evidence: `spec/04-expressions.md:51` (strict left-to-right),
  `spec/02-types.md:139` (TY-6), `spec/16-borrow-rules.md:38` (last-use loan).
- Rejected: adding a template-specific evaluation rule (redundant).
- Confidence: High.

**AR-14 — scope excludes #24**

- Authority: AI — delegated (strict scope).
- Eligibility: scope boundary already established by the prior closeout and
  the current objective.
- Decision: #24 (`text` arity) and all other deferred issues stay out.
- Evidence: the issue bodies are independent; #24 has its own reproduction
  and one-argument fix.
- Rejected: bundling (needs a user ruling; not required by #26).
- Confidence: High.

**AR-15 — naming**

- Authority: AI — delegated.
- Eligibility: cosmetic/low-impact naming.
- Decision: plan folder `plans/template-placeholder-integrity/`; suites
  `compiler/tests/template_placeholder_integrity_spec_test.rs` and
  `..._impl_test.rs`.
- Evidence: follows the `<feature>_spec_test.rs` /
  `<feature>_impl_test.rs` convention in `AGENTS.md`.
- Confidence: High.

**AR-16 — verification**

- Authority: AI — delegated.
- Eligibility: testing strategy using project commands.
- Decision: `make check`, `make asan`, `make ubsan`, `make verify`,
  `make bench-shape`, and `make bench` + `make bench-compare` against
  `8b4bff2`; every scored metric within 1% and every scored output equal.
- Evidence: `AGENTS.md` command table; the #25 plan used the same gate.
- Rejected: correctness suites only (the change touches emission and memory
  safety).
- Confidence: High.

**AR-17 — literal/empty edge cases**

- Authority: AI — delegated.
- Eligibility: preserves existing behavior.
- Decision: a placeholder-free template remains a `str` literal; the #25
  zero-length append guard remains.
- Evidence: `compiler/src/codegen/expr.rs:1620` returns `cl_str(...)` when no
  placeholder exists; `compiler/runtime/shim_host.c` skips a zero-length copy.
- Confidence: High.

**AR-18 — generic scalar conversion cost**

- Authority: AI — delegated.
- Eligibility: performance trade-off inside the correctness fix.
- Decision: accept one extra `String` conversion for a generic scalar
  placeholder; do not add an instance-specialized fast path now.
- Evidence: the hidden copy path must be type-agnostic before instantiation;
  the concrete scalar path stays allocation-free.
- Rejected: an instance-specialized codegen path (a material support surface
  and a larger emission change for a rare case).
- Confidence: Medium. Reopen if a benchmark or a user reports the cost.

**AR-19 — security surface**

- Authority: AI — delegated.
- Eligibility: security assessment.
- Decision: no new input, parser, FFI, or unsafe path; the copy is internal.
- Evidence: only already-checked placeholder expressions reach the lowering.
- Confidence: High.

**AR-20 — diagnostic reuse**

- Authority: AI — delegated.
- Eligibility: UX consistency with the frozen registry.
- Decision: reuse E10010 and its registry message/help unchanged.
- Evidence: `compiler/src/diag/registry.rs:153`.
- Confidence: High.

**AR-21 — null literal placeholder**

- Authority: AI — delegated (strict scope).
- Eligibility: scope boundary; a different root cause from the four #26
  defects.
- Decision: `$"{null}"` and a null-shaped placeholder value (for example a
  `String?` conditional arm whose other arm is `String`) are excluded. Their
  current paths reach the driver as an internal "emitted C failed to compile"
  error or a crash; they are recorded as a follow-up issue at closeout, not
  bundled. A **conditional null arm in a generic template** is still judged by
  the per-instance duty (ST-37), because the duty sees the arm and can report
  E10010. An owning **aggregate** literal field before a template copy keeps
  its inline evaluation order (the pin covers duplicable operands, owning
  calls, and fixed arrays); that ordering residual is part of the same
  follow-up.
- Evidence: a standalone `$"{null}"` program on `8b4bff2` reports the ICE;
  `c ? a : b` with `a: String` and `b: String?` is typed non-null and crashes
  on the base and head. The fix belongs in the formattability/null-literal and
  conditional-result rules, not in the ownership ordering this plan changes.
- Rejected: bundling the sema change (scope expansion, separate root cause).
- Confidence: High.

**AR-22 (runtime) — adopting template store**

- Authority: AI — delegated by `--auto-design`.
- Eligibility: internal compiler mechanism within the accepted ordered-lowering
  behavior; no language or ABI surface.
- Decision: add `Statement::Adopt`, a compiler-temporary store that does not
  release the previous value. The part builder adopts the copy; the store runs
  at the placeholder's source position.
- Evidence: a plain `Statement::Assign` pre-drops the previous copy on a loop
  re-entry and double-frees a value the builder already freed
  (`owned_placeholders_in_a_loop_are_leak_free`); an adopting `Materialize`
  fill runs after a later sibling's hoisted mutation and copies the wrong bytes
  (`an_alias_sibling_does_not_free_a_placeholder_source` printed `z1`). The
  `Adopt` store preserves both. `emit_statement` calls `emit_guards` before the
  store, so an index/slice/division guard in the copied expression is kept
  (ST-38).
- Rejected: a plain Assign (double free); an adopting Materialize (source-order
  regression); a new `ValueKind` (heavier walk surface).
- Strongest counterargument: a new statement variant must be handled by every
  walker. The compiler forces the exhaustive matches; the walkers that inspect
  assigned values now treat `Adopt` identically, and the for-step path
  deliberately does not consider it structured.
- Confidence: High. Reopen if a missed walker changes a type, cleanup, or
  guard for an adopted value.

## Reopen Triggers

- The frozen spec gains an explicit rule for null placeholders or a different
  evaluation order.
- `make bench-compare` shows a scored regression above 1% (AR-9, AR-16).
- A guarded-position case cannot be ordered through the existing fill
  machinery (AR-11).
- A generic placeholder cannot be made correct without an instance-level
  mechanism (AR-7, AR-18).
- The user rules to bundle #24 or another deferred issue (AR-14).
