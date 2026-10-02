# Ambiguity Register: Residual Defect Completion

> **Status**: ✅ GATE PASSED — all 12 items resolved
> **Last Updated**: 2026-10-02 22:53
> **Execution additions**: AR-11 and AR-12 (runtime), recorded during Phase 2 execution.

## Ambiguity Register

| # | Category | Ambiguity / Gap | Options Presented | User Decision | Status |
|---|----------|-----------------|-------------------|---------------|--------|
| 1 | Scope | One plan for all six filed defects (DEF-50 – DEF-55) or split them | One plan, two clusters / two plans by cluster / one per defect | One plan, two clusters | ✅ Resolved |
| 2 | Technical | DEF-52: specialize a lifted lambda per enclosing instance or reject the generic-plus-lambda combination | Specialize per instance / reject with a new diagnostic | Specialize per instance | ✅ Resolved |
| 3 | Naming | DEF-50: how the C symbol key encodes array extents | `arr2_i32` / `arr2x3_i32` extents form / suffixed `i32_2` / struct-key-only fold | `arr2_i32` / `arr2x3_i32` (full extents in `type_key`) | ✅ Resolved |
| 4 | Technical | DEF-53 and DEF-55: bind the ownerless call result to a temporary and move the projection, or record per-instance duties | Bind temp and move projection / per-instance duty | Bind temp and move projection | ✅ Resolved |
| 5 | Scope | Compatibility baseline for the six fixes | Fix all six as accepted, leak-free programs with no spec/diagnostic/dependency change (strict scope) / allow a spec change if needed | Fix all six; no `spec/`, diagnostics, or dependency change; DEF-45-class neighbors stay out | ✅ Resolved |
| 6 | Naming | Plan folder and test-suite names | `residual-defect-completion` / `value-crossing-residuals` / `generic-ownership-residuals` | `residual-defect-completion`; suites `residual_defect_completion_{spec,impl}_test.rs` | ✅ Resolved |
| 7 | Technical (verification) | Verify command per phase and at close-out | `make check` per phase + `make verify` at close-out / `make verify` every phase | Detected from `AGENTS.md`; `make check` per phase, `make verify` at close-out | ✅ Resolved |
| 8 | Technical (complexity escalation) | DEF-51: on-demand per-type drop helper versus a targeted statement hoist in `emit_guards` | Helper facility / generic runtime helper with thunks / guard restructure / targeted statement hoist | Smaller solution: targeted statement hoist (larger machinery explicitly not approved) | ✅ Resolved |
| 9 | Behavioral | Phase ordering of the six fixes | Substitution cluster first (DEF-54, DEF-50, DEF-52), release cluster second (DEF-55, DEF-53, DEF-51), then close-out / other orders | The recommended order | ✅ Resolved |
| 10 | Behavioral | The implementation test that pins the DEF-51 skip is rewritten when the limit is fixed | Rewrite the implementation pin (impl tests track the code; the spec suites never pinned the skip) / keep it | Rewrite it; no specification case pinned the skip | ✅ Resolved |
| 11 | Technical (runtime) | ST-8's bound `String` result hits a pre-existing bound-lend double free, outside this plan's scope | Inline the result and file the pre-existing defect / inline and note only / expand scope to fix | Inline the result in ST-8; file the pre-existing defect as DEF-56 (roadmap Backlog) | ✅ Resolved |
| 12 | Technical (runtime) | Phase 2 review: unsubstituted lifted return type (critical), specialization predicate gaps (major), closure-argument specialization hole (major), pre-existing trailing-comma closure call | Fix F1+F2 now and file F3 (+ pre-existing) / fix all three now / fix F1 only | Fix F1 (`return_ty` substitution) and F2 (capture/nested-value predicate) now; file F3 as DEF-57 and the trailing-comma call as DEF-58 (Backlog) | ✅ Resolved |

### Resolution Notes

**AR-1:** The six defects share two machinery threads: C symbol/type substitution (DEF-50, DEF-52,
DEF-54) and call-result ownership release (DEF-51, DEF-53, DEF-55). One plan keeps the interlocking
duty semantics (DEF-53 and DEF-55 both bind an ownerless call result) in one reviewable change.

**AR-2:** Acceptance is decided at the policy level: an instance of the enclosing generic function
carries its lifted lambdas through the same substitution, so the nested generic call is recorded
and emitted. No diagnostic is added, so the frozen specification is untouched.

**AR-3:** Encodings are deterministic and collision-resistant across the six defect classes; the
extents render literally (`arr{0}x{1}..._{element}`), preserving byte-identical re-emission.

**AR-4:** The chosen mechanism binds the ownerless call result to an owned temporary, moves the
element or field out of it, and releases the temporary for the rest. This reuses the existing move
and drop machinery instead of adding a per-instance analysis duty.

**AR-5:** Strict scope: only DEF-50 – DEF-55 are planned. DEF-45-class transfer temporaries, the
deferred element-move limits, and any other reviewed residual stay out of this plan.

**AR-6:** Naming follows the existing feature-plan and suite convention.

**AR-7:** `make check` runs `./spec/check.sh` plus `cargo test --workspace` in the pinned container;
`make verify` adds every hosted-amiga cross-toolchain leg.

**AR-8 — complexity escalation record:**
```text
Original goal: A guarded operand of a drop-needing growable element type must release its
previous value before re-store (DEF-51; currently leaks under LeakSanitizer).
Extra system or support code: An on-demand per-type drop helper facility (helper registry,
name mangling, forward declarations or pre-scan, deterministic ordering).
Why it may be needed: Guards are C comma-expression operands and C99 has no statement
expressions, so the growable element loop seemed impossible to express.
Evidence: drop_expr_text returns None for a growable element with a drop-needing element
(compiler/src/codegen/drop.rs:160-164); guards push a drop only when it returns Some
(compiler/src/codegen/guards.rs:680-684). The statement-form path already exists:
emit_reassigned_temp_drop (guards.rs:215-226) runs per-iteration from emit_guards for
always-evaluated materializes, and every guarded loop has that per-iteration slot
(compiler/src/ir/guards.rs:71-74, control.rs:346-352). Temps are zero-initialized
(function.rs:251-258), so an early hoisted drop is null-safe.
Smallest solution that still works: extend emit_guards to walk the conditionally evaluated
operand (&&/|| right side, ?: arms) and call the existing emit_reassigned_temp_drop for each
Materialize whose drop_expr_text is None; keep expression drops for types that have them.
Extra cost: None beyond the walk; the helper facility would add symbols, registration, and a
name-mangling surface, and would still miss the "struct containing a growable array" half
unless generalized.
Independent verdict: Simplify — the targeted statement hoist is the right mechanism; the
helper facility is roughly an order of magnitude more surface.
Direct user decision: smaller (adopt the statement hoist; larger machinery not approved)
```

**AR-9:** The substitution cluster lands first because DEF-54's unsubstituted argument type is
adjacent to DEF-52's unsubstituted lifted body, and DEF-50's key encoding is independent but sits
in the same codegen type surface. The release cluster then binds call results once for DEF-53 and
DEF-55 before DEF-51's guard hoist.

**AR-10:** Fixing DEF-51 changes the emitted shape that
`should_keep_the_drop_skip_when_a_growable_guard_element_needs_a_drop`
(`compiler/tests/call_value_integrity_impl_test.rs:1609-1641`) pinned as a recorded limit. No
specification suite pinned the skip, so the implementation test is rewritten to pin the fixed
shape, and the defect's behavior is pinned by new specification cases in this plan's suite.

**AR-11 (runtime):** While pinning the two-instance case, binding the `outer<String>` result to an
owning local (`let s: String = outer<String>(stringOf("abc"))`) double-freed: the callee returns an
alias of the caller's owned temporary while the binding takes ownership, so both release it. The
shape reproduces on `efbf55f` with `outer<T>(x: T): T { return apply<T>(x); }` and no lambdas, so it
is a pre-existing defect outside this plan's strict scope. The user ruled: keep the ST-8 behavior
(both instances build, exit `3`) with the String result measured in place, and file the
pre-existing defect as roadmap DEF-56 (Backlog). No scope expansion.

**AR-12 (runtime):** The Phase 2 review found three issues in the new specialization machinery: a
critical unsubstituted `return_ty` in `emit_lifted_function` (F1), a specialization predicate that
missed lambda captures and nested value positions (F2, major), and a callable-bound closure
argument whose instance parameter type did not carry the closure's specialization (F3, major). A
pre-existing zero-argument closure call with a trailing comma was also reproduced. The user ruled:
fix F1 and F2 now, with compile-and-run pins; file F3 as roadmap DEF-57 and the trailing-comma
call as roadmap DEF-58 (Backlog). No scope expansion; one scoped re-review of the fix diff.
