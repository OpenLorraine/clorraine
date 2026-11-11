# Ambiguity Register: Template Placeholder Residuals

> **Status**: ✅ GATE PASSED — all 24 items resolved
> **Last Updated**: 2026-11-11 21:03
> **Authority mode**: auto-design (root invocation
> `codeops-20261111-2103-template-placeholder-residuals`, policy version 1)

## Register

| # | Category | Ambiguity / Gap | Options Presented | Decision | Status |
|---|----------|-----------------|-------------------|----------|--------|
| 1 | Behavioral | A bare `$"{null}"` placeholder emits `cl_text_unknown(NULL)` and fails the C build | reject E10500 / reject E10012 / type it as an empty `str` | Reject with E10500 at the placeholder: a null-shaped value has no formattable type. A null arm with a view sibling stays an empty view; a null arm with a handle sibling makes the result nullable and reports E10010 (AR-23) | ✅ Resolved |
| 2 | Behavioral | A conditional of `String` and `String?` is typed non-null and crashes when the null arm is chosen | preserve nullable / template-only rejection / reject all mixed conditionals | Preserve nullable when either arm is nullable; the existing E10010 rule then applies | ✅ Resolved |
| 3 | Behavioral | A zero-state `String` handle from `H {}` reaches `cl_string_copy(NULL)` and crashes | null-safe runtime accessors / reject the zero value / reject the placeholder | Treat the all-zero handle as the empty String: null-safe `cl_string_ptr`, `cl_string_len`, `cl_string_copy` | ✅ Resolved |
| 4 | Behavioral | A template expression used as a discarded statement or in a `for` step leaks its result | bind the owned template result and drain per step / template-only emitter free / leave | Bind an owned template result in `lower_discarded`; drain pending releases after every for-step expression | ✅ Resolved |
| 5 | Behavioral | An owning aggregate literal field before a later template placeholder evaluates after the copy (`seq=21`) | adopt the aggregate with a no-pre-drop fieldwise store / restructure the whole literal / leave | Extend `Statement::Adopt` to a no-pre-drop structured store; pin owning non-fixed `StructLit` fields | ✅ Resolved |
| 6 | Technical | How a `Statement::Adopt` handling a `StructLit` keeps valid C when a field needs stores | plain assignment / fieldwise store without a pre-drop / new statement variant | Reuse `emit_struct_literal_fields` into the fresh temporary; no pre-drop because the temporary is fresh | ✅ Resolved |
| 7 | Technical | How the conditional result keeps nullability without breaking narrowed arms | OR the arm flags in `conditional_result` / a template-only check / per-arm duty only | OR the arm `nullable` flags into the settled result; narrowed arms stay accepted by the flow check | ✅ Resolved |
| 8 | Technical | Which String runtime helpers must tolerate the zero state | copy only / copy + len / ptr + len + copy | `cl_string_ptr` returns NULL for NULL, `cl_string_len` returns 0, `cl_string_copy` copies the empty view; mirror on Amiga | ✅ Resolved |
| 9 | Technical | Where the for-step cleanup belongs | inside each step expression / after the whole step block / in the emitter | Record a release floor before each step expression and drain after it, mirroring `check_stmt` | ✅ Resolved |
| 10 | Scope | Whether the explicit `text(null)` ICE is part of this plan | include / follow-up issue | Exclude: it is the explicit conversion path, not a template placeholder; record it as a follow-up at closeout | ✅ Resolved |
| 11 | Compatibility | Does preserving conditional nullability reject valid programs | no valid narrowing rejected / reject all mixed conditionals | Only unnarrowed nullable-arm programs are rejected; narrowed arms (concrete and generic) stay accepted | ✅ Resolved |
| 12 | Compatibility | Does the zero-state runtime change the ABI | signatures unchanged / new helpers | No ABI change; the same three helper signatures become null-safe | ✅ Resolved |
| 13 | Specification | Is a spec edit required | edit spec / no edit | No spec edit: O7 (zero = empty), STR-7 (`Point {}`), TY-6, and Ch 04 §3 already require the behavior | ✅ Resolved |
| 14 | Scope | Whether #24 or other deferred issues are bundled | bundle / separate | Separate; this plan delivers #27 only | ✅ Resolved |
| 15 | Naming | Plan folder and suite names | — | `template-placeholder-residuals`; `template_placeholder_residuals_{spec,impl}_test.rs` | ✅ Resolved |
| 16 | Non-functional | Verification and regression budget | full gate + 1% guard / correctness only | Full gate (`check`, `asan`, `ubsan`, `verify`, `bench-shape`, `bench` + `bench-compare`) against `b6f12d8` | ✅ Resolved |
| 17 | Edge cases | A null literal in a conditional with a view (`flag ? null : "x"`) currently prints empty in a placeholder | keep / reject | Keep for a placeholder with no expected type: the result is a `str` empty view. A written or resolved view destination keeps its existing E10012 rule | ✅ Resolved |
| 18 | Edge cases | A `for` step reached by `continue` | cleanup on both edges / fall-through only | The release statement lives in the step block, so both edges run it | ✅ Resolved |
| 19 | Edge cases | An owning aggregate field in a loop | adopt without pre-drop / pre-drop | Adopt: the literal field owns the handle, and a loop re-entry must not free it again | ✅ Resolved |
| 20 | Security | New input, parser, FFI, or unsafe surface | — | None: a diagnostic, a type rule, two internal cleanup paths, and null checks inside existing runtime helpers | ✅ Resolved |
| 21 | UX & presentation | Diagnostic code and message for the null placeholder | E10500 / E10012 | E10500 with the frozen `type '{type}' is not formattable` message; the displayed type is `null` | ✅ Resolved |
| 22 | Non-functional | The String accessor null checks may add benchmark instructions | accept under 1% / avoid with a narrower fix | Measure; if a scored metric exceeds 1%, first inline one NULL test in the generated `cl_string_view` while keeping all three accessors safe; if still over, seek a ruling (no safety trade-off) | ✅ Resolved |
| 23 | Behavioral | A null arm with a handle sibling (`flag ? null : stringOf("x")`) is typed `String` and crashes when the null arm is selected | type the result nullable / reject E10012 at the null arm / leave | Type the result `String?`; a placeholder reports E10010 at the null arm. A view/slice null arm stays the empty state; an annotated non-null handle destination keeps E10012 | ✅ Resolved |
| 24 | Technical | The adopted aggregate's fieldwise store pre-drops fields on loop re-entry | zero-fill the fresh temporary first / pre-drop / new statement | Emit `cl_zero(&temp)` before the fieldwise fill; the temporary is a fresh alias, so zeroing loses nothing the literal already adopted | ✅ Resolved |

## Delegated Resolution Records

All eligible decisions below are recorded under `_shared/auto-design.md` policy
version 1, root invocation
`codeops-20261111-2103-template-placeholder-residuals`. The shared objective is
issue #27: close the five template-placeholder residuals without weakening any
frozen rule. Reopen triggers are listed once at the end; each applies to every
row unless a row adds its own.

**AR-1, AR-21 — null literal placeholder**

- Authority: AI — delegated by `--auto-design`.
- Eligibility: behavioral consequence of the frozen placeholder formattability
  rule (`spec/04-expressions.md` §12, E10500) and the null-shape typing.
- Decision: report E10500 at a placeholder whose recorded type is null-shaped
  (`TyKind::Unknown` with `nullable`). The message is the frozen
  `type 'null' is not formattable`; no translation unit is written.
- Evidence: `$"{null}"` emits `cl_text_unknown(NULL)` and the driver reports
  `internal compiler error: the emitted C failed to compile` on `b6f12d8`.
  `compiler/src/sema/expr.rs::check_template` excludes null-shaped types from
  the opaque rejection, so nothing judges them. `compiler/src/own/expr.rs`
  types the null literal as `Unknown` + nullable.
- Rejected: typing the bare null as an empty `str` (a context-free conversion
  the language does not define); E10012 (the placeholder rule owns
  formattability, and the frozen registry maps a non-formattable placeholder to
  E10500).
- Strongest counterargument: `flag ? null : "x"` is accepted and prints empty,
  so users may expect `$"{null}"` to do the same. That conditional has a `str`
  context; a bare null has no expected type. Rejecting is the conservative,
  spec-grounded choice.
- Confidence: High. What would change it: a spec clause giving the bare null
  literal a default `str` type in a formatting context.

**AR-2, AR-7, AR-11 — mixed-nullability conditional**

- Authority: AI — delegated.
- Eligibility: direct consequence of the frozen nullable-use rule
  (`spec/02-types.md` TY-6) applied to the existing conditional result rule.
- Decision: `types::conditional_result` keeps the nullable flag from any
  **non-null-shaped** nullable arm. A conditional of `String` and `String?` is
  `String?`; an unnarrowed placeholder over it reports E10010 at the nullable
  arm, and a narrowed arm stays accepted. A null-shaped arm sets the flag only
  when the settled result is a handle/`Box` (`String?`); with a view/`raw`/
  `cstring`/slice sibling it is the empty view and the flag stays clear
  (AR-23).
- Evidence: `flag ? a : b` with `a: String`, `b: String?` compiles and
  segfaults when the null arm is selected. `compiler/src/types/mod.rs:1055`
  returns `then_ty.clone()` when the kinds match, dropping `else_ty.nullable`.
  `let s: String = b;` already reports E10010
  (`compiler/src/own/expr.rs:1441`). `flag ? null : "x"` runs as an empty view
  today; a blanket OR would make it `str?` and reject it, so null-shaped arms
  are excluded from the flag.
- Rejected: a template-only arm check (leaves the same unsafe store accepted);
  rejecting every mixed conditional (valid narrowed programs must stay legal);
  a blanket OR (rejects the accepted empty-view conditional).
- Strongest counterargument: a nullable static type can be over-conservative
  for a narrowed arm. The flow check accepts the narrowed arm, so only the
  static type remains nullable; it does not reject a valid program.
- Confidence: High.

**AR-3, AR-8, AR-12 — zero-state String handle**

- Authority: AI — delegated.
- Eligibility: runtime behavior of the frozen zero-state rule (`spec/03` O7)
  and the explicit zero value (`spec/07` STR-7); no language surface.
- Decision: the three String helpers treat NULL as the empty String:
  `cl_string_ptr(NULL)` returns NULL, `cl_string_len(NULL)` returns 0, and
  `cl_string_copy(NULL)` returns `cl_string_new(0)`. Mirror the Amiga shim.
- Evidence: `H {}` is a valid zero value; `$"{h.name}"` emits
  `cl_string_copy(h.name)` and `println(h.name)` widens through
  `cl_string_ptr`/`cl_string_len`; all three dereference NULL and segfault on
  `b6f12d8`. `FreeString` is already null-safe (`compiler/runtime/shim_host.c`
  and `shim_hosted_amiga.c`), so the zero state is already a defined drop
  state; the read accessors are the missing half. Issue #27 authorizes the
  empty-state reading ("apply the nullable/empty-state rule to the zero-state
  handle"). O7 defines the zero bit pattern as the valid empty state and STR-7
  makes `H {}` a valid zero value; the read rule is the only coherent reading
  of the two clauses. No spec text is edited.
- Rejected: rejecting `H {}` owning fields (breaks STR-7); rejecting the
  placeholder at compile time (the zero state is a valid empty String; its
  origin is not always visible to the checker); a sentinel allocation for every
  zero-state field (wider emission change, and the C zero state is the
  language's empty state).
- Strongest counterargument: adding null checks to `cl_string_ptr`/`len`
  touches every String use and may move benchmark instructions. The 1% guard
  measures it; the safe fallback is one inlined NULL test in the generated
  `cl_string_view`, never removing the accessor checks.
- Confidence: High.

**AR-4, AR-9, AR-18 — discarded template and for-step cleanup**

- Authority: AI — delegated.
- Eligibility: internal ownership cleanup for an already-valid expression
  statement; no language change.
- Decision: bind an owned `Template` result in `lower_discarded` exactly as a
  bare call result is bound, and drain pending releases after each `for` step
  expression. The same drain covers a discarded owning call in a step.
- Evidence: a bare `$"{1}";` statement leaks 20 bytes under ASan on `b6f12d8`;
  `for (let i = 0; i < 3; $"{i}", i = i + 1)` leaks 60 bytes. `lower_discarded`
  binds only `Call`/`ClosureCall` (`compiler/src/own/lower.rs:1984`), and
  `check_for_c` calls `check_expr_statement` without the release-floor drain
  every other statement gets (`compiler/src/own/loops.rs:400`).
- Rejected: a template-specific emitter free (the emitter does not own the
  value and would miss a discarded owning call); leaving the step flat but
  releasing after the loop (one release for many iterations).
- Confidence: High.

**AR-5, AR-6, AR-19 — owning aggregate literal order**

- Authority: AI — delegated.
- Eligibility: internal lowering/emission for the frozen source-order rule
  (`spec/04-expressions.md` §3, struct literal fields evaluate in source order).
- Decision: `pin_literal_operand` adopts an owning non-fixed-array `StructLit`
  through `materialize_adopted_operand`; `Statement::Adopt` gains a no-pre-drop
  structured store for a `StructLit` whose fields need stores, and keeps the
  plain assignment otherwise. The outer literal then reads the adopted
  temporary at its field.
- Evidence: `P { inner: Inner { a: mkString() }, b: $"{mark(2)}" }` prints
  `seq=21`: the template copy statement is emitted while lowering field `b`,
  before the C initializer that evaluates `mkString()` in field `inner`. The
  current pin leaves owning aggregates inline (`compiler/src/own/lower.rs`,
  `pin_literal_operand`). `emit_struct_literal_store` pre-drops the target, so
  a plain `Assign` to a reused temporary double-frees on loop re-entry; the
  fresh `Adopt` temporary needs the fieldwise fill without that drop.
- Rejected: a whole-literal structured store (the copy statement still
  precedes the outer `Assign`); a new statement variant (Adopt already has the
  no-pre-drop semantics); an aggregate `Assign` (invalid C for a fixed-array
  field and a double free on loop re-entry).
- Strongest counterargument: a second aggregate carrier path adds emission
  surface. The fieldwise core already exists and is reused; only the no-drop
  caller is new.
- Confidence: High.

**AR-10 — `text(null)` scope**

- Authority: AI — delegated (strict scope).
- Eligibility: scope boundary; a different conversion surface.
- Decision: the explicit `text(null)` ICE stays out of this plan and is filed
  as a follow-up issue at closeout.
- Evidence: `let t: String = text(null);` emits `cl_text_unknown(NULL)` and
  fails the C build, but it is an explicit converter, not a template
  placeholder. The template fix does not change it.
- Rejected: bundling a new formattability rule for every explicit converter
  (scope expansion beyond #27).
- Confidence: High.

**AR-13, AR-14, AR-17 — no spec/ABI change and scope**

- Authority: AI — delegated.
- Eligibility: compatibility assessment; no frozen artifact edit.
- Decision: no `spec/`, ABI, or public syntax change. The runtime helpers keep
  their signatures; the conditional rule follows TY-6; the zero-state read
  follows O7/STR-7. #24 and the other deferred issues stay separate.
- Confidence: High.

**AR-15 — naming**

- Authority: AI — delegated (cosmetic).
- Decision: `plans/template-placeholder-residuals/`,
  `compiler/tests/template_placeholder_residuals_spec_test.rs`, and
  `..._impl_test.rs`.
- Confidence: High.

**AR-16, AR-22 — verification and performance**

- Authority: AI — delegated.
- Eligibility: testing strategy using the project's commands.
- Decision: `make check`, `make asan`, `make ubsan`, `make verify`,
  `make bench-shape`, and `make bench` + `make bench-compare` against
  `b6f12d8`; every scored output equal and no scored metric above 1%.
  The String null checks are measured; if above 1%, first inline one NULL test
  in the generated `cl_string_view` while keeping all three accessors safe. If
  that still exceeds 1%, stop and seek a ruling; removing an accessor check is
  not an acceptable trade-off because `len` and `clone` call the accessors
  directly.
- Confidence: High.

**AR-20 — security**

- Authority: AI — delegated.
- Decision: no new input, parser, FFI, or unsafe surface. The runtime checks
  are internal; no secrets or user data are involved.
- Confidence: High.

**AR-23 — null arm with a non-nullable handle sibling**

- Authority: AI — delegated.
- Eligibility: behavioral consequence of the frozen null rule (E10012) and the
  conditional result rule; no new surface.
- Decision: when a conditional settles to a `String`/`Box` handle and an arm
  is null-shaped, the result is that concrete nullable type (`String?`). A
  placeholder then reports E10010 at the null arm, exactly as the issue's
  "concrete nullable type and E10010 applies" direction. A view/`raw`/
  `cstring`/slice result accepts the null as its empty state and stays
  non-nullable. An annotated non-nullable handle destination keeps its existing
  `report_null_conditional_arm` E10012. A result that still names a parameter is
  left to the per-instance duty.
- Evidence: `flag ? null : stringOf("x")` in a placeholder compiles and
  segfaults when the null arm is selected on `b6f12d8`; the type stage has no
  expected destination to report against. `flag ? null : "x"` runs as an empty
  view and must stay accepted. `flag ? x : null` with a narrowed `String?` `x`
  already reports E10010 at the null arm on `b6f12d8`, and #26 ST-37 expects
  the same for a generic instance. `let y = c ? h : null;` with a non-null
  `String` `h` must stay accepted and narrowable (`c ? h : null` is `String?`);
  the parity suite pins it. `null_arm_needs_report`
  (`compiler/src/types/mod.rs:1077`) applies only to a written expected type and
  is unchanged.
- Rejected: rejecting the null arm with E10012 when no expected type was
  written (the parity suite requires the nullable inferred binding); leaving the
  handle case to crash.
- Strongest counterargument: the settled type becomes nullable in a context
  that did not write `?`. The language already widens `String` into `String?`
  at an inferred binding, and the E10010 rule makes the use explicit.
- Concrete/generic parity: a null arm at a nullable result keeps reporting
  E10010 in both the concrete ownership check and the generic per-arm duty (#26
  ST-37); the plan does not skip null-shaped arms. ST-28 pins the concrete twin.
  A result that still names a parameter is left to the per-instance duty, so the
  type-stage rule does not preempt ST-37.
- Confidence: High.

**AR-24 — adopted aggregate zero-fill**

- Authority: AI — delegated.
- Eligibility: internal emission for the adopted temporary; no language rule.
- Decision: the `Statement::Adopt` structured-store arm emits
  `cl_zero(&temp)` before the fieldwise fill. The temporary is a fresh alias
  whose values the outer literal already adopted; zeroing loses nothing and
  makes every fieldwise pre-drop a no-op on loop re-entry.
- Evidence: `emit_struct_literal_fields` uses the fixed-array store core and
  the owned-conditional move-assign, both of which release the previous field
  value. Without the zero-fill, a second iteration double-frees the previous
  iteration's handles (reproduced by the grounding auditor). The temporary is
  overwritten on each iteration; the outer literal owns the previous iteration's
  values and drops them at its own scope end.
- Rejected: removing the pre-drops from the fieldwise core (it is shared with
  ordinary stores); a new no-drop statement variant (the zero-fill is smaller).
- Confidence: High.

## Reopen Triggers

- `make bench-compare` shows a scored regression above 1% (AR-16, AR-22).
- Preserving conditional nullability rejects a valid narrowed program or breaks
  a generic instance audit (AR-2, AR-11, AR-23).
- A zero-state String still crashes through a non-placeholder path after the
  three helpers are null-safe (AR-3).
- The aggregate Adopt path emits invalid C or double-frees on loop re-entry
  (AR-5, AR-6, AR-24).
- The user rules to bundle `text(null)` or another deferred issue (AR-10).
