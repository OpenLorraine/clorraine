# Findings: Clorraine v1.0 Production Readiness

> **Phase**: 1 — Release-blocking residual hardening
> **Review date**: 2026-11-02; remediation 2026-11-03
> **Reviewers**: correctness/semantics, memory-safety (ASan), emission-integrity (base/head C diff)
> **Gate**: final `make check` green; ASan new suites 43/43; implementation 6/6; affected existing ASan suites 769/769
> **Status**: ✅ Phase 1 review closed — RR-1 – RR-9 fixed; pre-existing residuals accepted with documented magnitude

The independent review found new defects in the Phase 1 change. The intended
fixtures were green, but the implementation was not safe for the wider shapes.
The user approved fixing all findings; the first-pass fixes and permanent
regressions are in place. The independent re-review verified several fixes but
found new and remaining critical/major issues. Phase 1 remains blocked.

## Findings

| ID | Severity | Finding | Location | Status |
|---|---|---|---|---|
| F-1 | 🔴 CRITICAL | A global fixed-array fill re-evaluates a run-time index in the post-store move-out. The second evaluation is unguarded and can select a different slot, producing a heap-use-after-free; an out-of-range second value writes out of bounds. | `compiler/src/codegen/program.rs:1696-1704` (retention before preparation), `:1355-1362`, `:1425-1431` | ✅ Fixed — prepare before retention; shared index local |
| F-2 | 🔴 CRITICAL | A conditional initializer arm's move-out is emitted without the arm guard. When the arm is untaken and its runtime index is out of range, the move-out writes past the zeroed root array. | `compiler/src/codegen/program.rs:1513-1566` (`retain_guarded_projection_root`) | ✅ Fixed — move-out is guarded |
| F-3 | 🔴 CRITICAL | Nested view-array element roots of a guarded literal are bound inside the arm block but released at function scope. The emitted C names undeclared locals and does not compile, on host and m68k. | `compiler/src/codegen/program.rs:2200-2232`, `:1475`, `:1085-1090` | ✅ Fixed — pre-retain fields outside the guard; carry per-arm selection |
| F-4 | 🟠 MAJOR | Fill-root binding is hoisted before earlier explicit element evaluations, changing source evaluation order. | `compiler/src/codegen/program.rs:2227-2232`, `bind_fill_root` `:2256-2271` | ✅ Fixed — explicit calls bind in the element loop before the fill |
| F-5 | 🟠 MAJOR | The new lend-argument root binding uses a function-body local. In a loop the root is overwritten each iteration without release, so the leak grows without bound. | `compiler/src/own/lower.rs:6581-6593`, `:6683` | ✅ Fixed — scoped root owner per iteration |
| F-6 | 🟡 MINOR | New tests read only the view's cached length; ST-5's `pick()` is pure and the impl pin does not assert one call. A wrong-slot free could pass the spec oracle. | `compiler/tests/v1_production_readiness_spec_test.rs:336-338,359-361,427-429`; `..._impl_test.rs:280-321` | ✅ Fixed — byte reads and side-effecting `pick()` count |
| F-7 | 🟡 MINOR | Multi-element retention does not cover bare call elements, a runtime index into a view-array field, or an unselected conditional element. These shapes still leak (some pre-existing). | `compiler/src/codegen/program.rs:2176-2235`, `:1279-1290` vs `:2212` | ✅ Fixed — bare calls bind in order; runtime field and unselected conditional regressions |

## Independent Re-review (2026-11-03)

The re-review ran the new suites, the named probes, and the base/head corpus
diff. It confirmed the F-1 UAF/OOB, the F-2 untaken move-out, the F-3 invalid C,
the F-4 call-element order, and the F-5 loop-body release. It found the
following new or remaining blockers.

| ID | Severity | Finding | Location | Status |
|---|---|---|---|---|
| RR-1 | 🔴 CRITICAL | A derived view that escapes a loop body through a view-returning call is freed with the loop-scoped F-5 root. Head ASan reports heap-use-after-free; base leaks but does not free. | `compiler/src/own/lower.rs:6586-6597`, `:4868`, `:6737-6761` | ❌ Open |
| RR-2 | 🔴 CRITICAL | Conditional-arm root binding uses the raw condition while the arm call can mutate it. The store then selects an arm whose root was never bound; a run-time-index arm dereferences the zero root (SEGV). | `compiler/src/codegen/program.rs:1510-1568`, `:2908-2918` | ❌ Open |
| RR-3 | 🟠 MAJOR | Guarded literals pre-retain only fixed-array fields, so a later array's calls hoist above an earlier non-retained field (`tagf` runs after `a`/`f`). | `compiler/src/codegen/program.rs:2195-2208`, `:2244-2288`, `:2210-2222` | ❌ Open |
| RR-4 | 🟠 MAJOR | A lend projection in a `while`/`do-while` condition is scoped to the enclosing block and released only after the loop; the leak grows per evaluation (22,000 B at N=1000). | `compiler/src/own/lower.rs:6586-6597`, `:6737` | ❌ Open |
| RR-5 | 🟠 MAJOR | Reordering F-1 preparation before `retain_view_array_fill_root` hides the prepared call root, so a whole-array projection no longer releases residual owning fields (`mkH().names` leaks 11 B). | `compiler/src/codegen/program.rs:1741`, `:1760`, `:1777-1780` | ❌ Open |
| RR-6 | 🟠 MAJOR | Only call/closure explicit elements bind before a fill. A place/member/conditional element remains inline after the fill, changing source order and reading mutated storage. | `compiler/src/codegen/program.rs:2335-2350`, `:2397` | ❌ Open |
| RR-7 | 🟠 MAJOR | Field selection is propagated to nested struct fields but not to a conditional field value, so every arm element is marked selected; an unselected arm element is nulled and leaks. | `compiler/src/codegen/program.rs:2974-2977`, `:2931-2938` | ❌ Open |
| RR-8 | 🟡 MINOR | Nested conditional arms are not descended (11 B); a runtime index on a named view-array field (`names[pick()]`) still leaks; the F-2/F-3 tests read only `len(g)` and the F-2 test is untaken-only. | `compiler/src/codegen/program.rs:1510-1527`, `:1583-1585`; `v1_production_readiness_regression_spec_test.rs:274,299,325` | Report / fold into RR fixes |

**Verified clean in the re-review**

| Area | Evidence |
|---|---|
| F-1 | `g33`/`g37` exit 0, ASan clean, one shared index local. |
| F-2 untaken arm | `f2` exit 0, ASan clean; guarded move-out present. |
| F-3 invalid C | `g39`, `g49`, `g43` strict host and m68k builds pass; ASan clean. |
| F-4 call elements | `g17`/`g19`/`g20`/`g25` source order restored. |
| F-5 loop body | `f5a`–`f5e` (body/break/continue/return, N=1000) clean. |
| Corpus | 0 acceptance changes, 0 run differences, 0 new strict-build failures; 23 intended emitted-C diffs. |
| ASan | new spec 9/9, regression 14/14, implementation 6/6, existing affected 769/769. |

**Ruling:** the user chose to continue with the best path; RR-1 – RR-7 are
fixed and RR-8 is recorded below.

## Second-pass Resolution

| ID | Change | Permanent regression |
|---|---|---|
| RR-1 | A view of a loop-body generated owner cannot escape to an outer binding; a direct call-returning view with a fresh argument and a call-returned local view are rejected with `E10030`. | `should_reject_a_view_escaping_a_loop_body`, `should_reject_a_loop_local_view_escaping_through_a_binding` |
| RR-2 | Conditional retention always stores the condition once (reusing an already-stored read) before binding arms; guards and the store read the same local. | `should_evaluate_a_mutable_conditional_condition_once` |
| RR-3 | Guarded literal field calls bind in source order before a later fixed-array field. | `should_preserve_order_before_a_guarded_array_field` |
| RR-4 | A lend projection in a loop condition binds through a per-evaluation materialization with a pending release. | `should_release_a_loop_condition_lend_root` |
| RR-5 | A prepared whole-array call root is registered for residual-field release. | `should_release_a_whole_array_projection_root` |
| RR-6 | Every non-constant view element binds at its source position before a following fill. | `should_read_a_place_element_before_a_following_fill`, `should_read_a_conditional_element_before_a_following_fill` |
| RR-7 | Conditional field values carry the outer field selection into their arms. | `should_release_an_unselected_conditional_struct_field` |
| RR-8a | Nested conditional arms recurse under the composed guard; fill member projections retain their fresh root. | `should_release_a_nested_conditional_arm`, `should_release_a_fill_member_projection` |
| RR-8b | **Accepted residual:** a run-time index on a named view-array field (`Box.names[pick()]`) still leaks one unselected backing (head 11 B, base 21 B), and a view-like struct element array (`Row[2]`) retains its pre-existing residual. These shapes are outside the v1.0 residual fixtures and are recorded for post-v1.0 work. | Not added as a passing ASan case; evidence in the re-review. |

## Third-pass Resolution (final review follow-up)

| ID | Change | Permanent regression |
|---|---|---|
| RR-9 | A fresh projection element (including a conditional arm and a struct-literal projection) goes through root retention instead of a plain view copy. | `should_release_a_fresh_projection_element`, `should_release_a_conditional_fresh_projection_element`, `should_release_a_struct_literal_projection_element` |
| RR-1 capture | A directly invoked write-capture lambda rejects a fresh argument or a copied call-returned local view; nested inline-lambda calls are left to their own call site so legal nested out-slots keep one report. | `should_reject_a_view_stored_through_a_capturing_lambda`, `should_reject_a_view_nested_in_an_array_literal_escaping_a_loop` |
| RR-1 false positive | The loop-escape predicate now follows the callee's single lend return source, not any fresh argument; a capture store of `staticView(next(), "zz")` and an unrelated owned argument are accepted. | `should_accept_a_static_view_with_a_fresh_unrelated_argument`, `should_accept_a_capture_store_of_a_static_view` |
| RR-3 conditional field | A conditional field before a guarded array binds in source order. | `should_preserve_order_for_a_conditional_field_before_a_guarded_array` |
| Test integrity | `assert_rejected` requires exactly one diagnostic; the mutable-condition, F-2, and F-3 cases read the selected backing byte rather than only `len`. | Regression suites |

## Accepted Residuals (pre-existing, no UAF)

These shapes predate the Phase 1 fixtures, are leak- or order-only, and are explicitly
outside this phase's closure. They are recorded for post-v1.0 work rather than
silently treated as clean.

| Residual | Evidence |
|---|---|
| RR-8b: named run-time view-array field index (`Box.names[pick()]`) leaks 11 B (base 21 B); `Row[2]` view-like struct elements leak 33 B at both revisions. | LeakSanitizer only; no UAF/double-free/OOB. |
| A directly invoked conditional lend (`sink(flag ? mk().xs[0] : mk().xs[1])`) still leaks per loop condition; extending the direct-projection binder is a separate ownership task. | Leak-only; head 33 B / base 44 B at the three-iteration probe; no UAF. |
| Aggregate view backing in a loop (`S { arr: [head(mk().xs[0]), "z"] }`) still grows; the direct projection control grows at base and head. | Leak-only; head 21,978 B / 1,998 allocations vs base 11,000 B / 1,000 — the documented 2× head magnitude is accepted; no UAF. |
| A fixed-array call field after an earlier field call and a direct global `[s; f()]` element still order by the call, not the earlier read. | Order-only; base and head both show the pre-existing order; no memory error. |
| `r08` whole-array projection of a struct literal emits invalid C at base and head; `e03` direct global place order is wrong at base and head. | Pre-existing emission/order defects; no new acceptance change. |

## Final Verification

| Gate | Result |
|---|---|
| `make check` | ✅ pass |
| `RDC_ASAN=1` `v1_production_readiness_spec_test` | ✅ 9/9 |
| `RDC_ASAN=1` `v1_production_readiness_regression_spec_test` | ✅ 15/15 |
| `RDC_ASAN=1` `v1_production_readiness_review2_spec_test` | ✅ 19/19 |
| `v1_production_readiness_impl_test` | ✅ 6/6 |
| Affected existing ASan suites (8) | ✅ 769/769 |
| Reviewer probe families | ✅ F-1, F-2, F-3, F-4, F-5 body/condition, RR-1 direct/capture/array escapes, RR-2 – RR-9 clean or correctly rejected; only accepted residuals above remain |
| Final independent re-review | ✅ No new critical/major; one aggregate leak accepted with its 2× magnitude documented |

## First-pass Resolution

| Finding | Change | Permanent regression |
|---|---|---|
| F-1 | `emit_global_array_fill` runs `prepare_fill_value` before retention, so a runtime index becomes a shared init local before the move-out is built. | `should_evaluate_a_global_fill_index_once`, `should_not_write_out_of_bounds_for_a_second_fill_index` |
| F-2 | `retain_guarded_projection_root` wraps the move-out in the arm guard. | `should_guard_an_untaken_conditional_move_out` |
| F-3 | Guarded literals pre-retain fixed-array fields before the store block opens; conditional arm walks carry the arm's own selection. | `should_build_a_guarded_nested_view_initializer`, `should_build_a_guarded_literal_as_a_call_argument` |
| F-4 | A bare call element binds in the element walk before a following fill; all call elements bind in source order. | `should_preserve_source_order_for_a_flat_fill`, `..._nested_fill_rows`, `..._field_before_a_fill` |
| F-5 | A non-take lend projection uses `bind_view_projection_root`, whose owner lives in the current scope and releases per loop iteration. | `should_release_each_loop_iteration_lend_root` |
| F-6 | Byte reads on the selected nested, multi-element, and indexed backings; `pick()` increments a counter so a second call changes the exit code. | `should_read_the_selected_nested_fill_backing`, `..._multi_element_backing`, `..._indexed_view_and_evaluate_pick_once` |
| F-7 | Runtime-indexed field elements retain the shared index; unselected conditional elements keep their roots whole. | `should_release_a_runtime_indexed_element_in_a_view_array_field`, `should_release_an_unselected_conditional_element` |

## Verification

| Gate | Result |
|---|---|
| `make check` | ✅ pass |
| `RDC_ASAN=1 cargo test --test v1_production_readiness_spec_test -- --test-threads=1` | ✅ 9/9 |
| `RDC_ASAN=1 cargo test --test v1_production_readiness_regression_spec_test -- --test-threads=1` | ✅ 14/14 |
| `cargo test --test v1_production_readiness_impl_test` | ✅ 6/6 |
| Affected existing ASan suites (`def_backlog_closeout`, `module_initializer_integrity`, `eval_emission_integrity`, `conditional_projection_residue`) | ✅ 769/769 |
| Reviewer probes `g17`–`g49`, `f2`, `f5` | ✅ expected results; `g32` exits 246; no sanitizer reports |

The temporary `build/phase1-probes` and reviewer worktrees are removed after the
re-review; the permanent tests are the two `v1_production_readiness_*` suites.

## Evidence

> Historical pre-fix evidence from the independent review. The resolution and
> permanent regressions are recorded above; the temporary reviewer probes were
> removed after verification.

### F-1 — runtime index re-evaluated in a global fill

Probe (`build/memory-audit/u7.clo`; the emission probe `g33` is equivalent):

```clorraine
let calls: i32 = 0;
func next(): usize { let r: usize = usize(calls); calls = calls + 1; return r; }
func mk(): String[2] { return [stringOf("aa"), stringOf("bb")]; }
let g: str[2] = [mk()[next()]];
```

Head emitted init:

```c
usize __cl_itmp1 = cl_..._next();
...
cl_..._g[0] = cl_string_view(__cl_itmp0[__cl_itmp1]);
__cl_itmp0[cl_..._next()] = NULL;      /* second evaluation */
if (__cl_itmp0[0]) { FreeString(__cl_itmp0[0]); }  /* frees the viewed slot */
```

ASan: heap-use-after-free reading `g[0]`. A second call returning 1000000
writes out of bounds and the head binary SIGSEGVs. Base leaks 11 B but has no
UAF/OOB. The scalar ST-5 path is safe because its walk precedes retention.

**Fix direction:** prepare the fill value before retention and build the
move-out from the prepared value (shared index local), or materialize every
effectful index operand before retention. Add a side-effecting-index regression.

### F-2 — unguarded conditional move-out

Probe: `let g: str = flag ? mk()[idx] : stringOf("zz");` with `flag=false`,
`idx=5`. Head emits:

```c
if (__cl_itmp0) { cl_..._mk(__cl_itmp1); }
...
__cl_itmp2[cl_..._idx] = NULL;   /* outside the arm guard */
```

ASan: stack-buffer-overflow writing past `ClString[2]`. Base emitted no
move-out and ran clean.

**Fix direction:** emit the move-out under the arm guard, e.g.
`if (guard) { local[accessor] = empty; }`. Add taken/untaken and valid/out-of-
range cases.

### F-3 — guarded nested roots released at function scope

Probe `g39` (conditional initializer arm with a nested view-array literal) emits
locals `__cl_itmp2/3` inside the `if (guard)` block, then:

```c
if (__cl_itmp3.v) { FreeString(__cl_itmp3.v); }  /* function scope */
```

Strict host and m68k builds fail with undeclared identifiers. Base emitted the
inert inline stores and built/runs.

**Fix direction:** thread the guard into the nested retention/root binding so
the local is declared outside the guard, or emit the release inside the same
arm block. The pre-existing flat guarded case (`g43`) has the same shape and
should be considered separately.

### F-4 — fill binding hoists earlier element evaluations

Probe `g20`:

```clorraine
let g: str = Box { rows: [a(); f()] }.rows[1];
```

Head emits `f()` before `a()`, while base emits `a()` then `f()`. `g17`, `g19`,
and `g25` show the same reordering with nested fills, a preceding struct field,
and two arrays. The new `bind_fill_root` comment claims source order; the
emission contradicts it.

**Fix direction:** bind fill roots during the `materialize_init_value` array
walk, or bind every explicit element in order before binding the fill, so the
fill call never hoists above an inline earlier element.

### F-5 — loop lend-argument leak

The ST-4 shape inside a three-iteration loop:

| Revision | Leak |
|---|---|
| Base | 33 B (three unselected siblings) |
| Head | 44 B (one function-body root overwritten each iteration) |

`materialize_owned_argument` moves the root to function-body scope
(`move_binding_to_body_scope`), so the per-iteration release is lost.

**Fix direction:** bind the lend root to a statement/scope temporary released
per iteration (for example a temp with `register_pending_release`, or a
current-scope local without body-scope promotion). Add a loop ASan regression.

## Minor coverage gaps

- ST-2/ST-3/ST-5 read only `len(view)`, which reads the cached view length and
  never touches the selected backing. A byte read (`view[0]`) on at least one
  fixture per retained shape would make a wrong-slot free visible to ASan.
- ST-5's `pick()` is pure and always returns 1; the impl pin checks the emitted
  declaration/use but not the call count. Use a side-effecting index or assert
  exactly one call.
- No loop or conditional runtime-index cases exist; those are where F-2 and F-5
  live.
- Bare call elements, runtime index into a view-array field, and unselected
  conditional elements still leak (some pre-existing). Decide whether to extend
  the fix or record them as explicit accepted residuals/DEF rows under AR-4.

## Pre-existing, not counted as new

The emission audit found pre-existing failures (FFI fragment link failures,
four null-to-`i32` `-Wint-conversion` fixtures, and the literal-index array
`g03`). They reproduce at base and head; they are not part of this change. The
flat guarded array `g43` was pre-existing at base; the F-3 guarded-literal
retention now also makes it build and run clean. The Phase 1 review does not
expand its scope to the remaining pre-existing items.

## User decision

The strict quality profile paused Phase 1 for the user's ruling on F-1 – F-5.
The user chose **fix all**. F-1 – F-7 are fixed and covered by the regression
suite above. The Phase 1 task stays open until the independent re-review is
clean.

**User decision:** Continue with the best path (2026-11-03). RR-1 – RR-9 fixed;
the pre-existing residuals above are explicitly accepted for this phase,
including the aggregate 2× magnitude. **Final re-review:** complete; no new
critical/major findings. Phase 1 review closed.

## Phase 3 Review (verification infrastructure)

Independent review of `make asan`, `make differential`, and the host-only CI
workflow found one critical and one major defect. Both are fixed, and the
re-review is clean.

| Severity | Finding | Resolution |
|---|---|---|
| Critical | The host-only CI could not pass on a bare runner: the driver required the SDK marker for every command, so host-profile driver tests exited 2. | The marker gate now applies only to Amiga and freestanding commands; `emit` and host-profile `build`/`run` use the host compiler. Unit coverage and bare-host probes confirm `emit`/host `run` succeed and Amiga commands still report the SDK guidance. |
| Major | `OUTDIR` and `DIFF_ONLY` were interpolated into the container command, so whitespace broke the command and a quote could inject shell text. | Values are exported and forwarded with `docker compose run -e NAME` through a fixed `PASSTHROUGH` list; they never enter `CMD`. A hostile value is absent from the expanded command, and a path with spaces works. |
| Minor | `ASAN_SUITE_TIMEOUT` was not forwarded; the suite timeout lacked a kill-after; combined sanitizer flags missed the wrapper. | The timeout is forwarded, `--kill-after=30` bounds a stuck suite, and the wrapper matches any flag containing `-fsanitize=address`. |

**Evidence:** `make check` green; `make asan` 24/24 suites; `make differential`
matches all three programs and the `DIFF_ONLY=1` corruption control exits
nonzero without regenerating the output; a marker-masked, tools-stripped
`cargo test --workspace` exits 0. The GitHub Actions run URL is pending an
authorized push; local evidence is recorded here and in the execution plan.

## Phase 4 Review (DX slice execution)

The DX plan was refreshed against the current tree before execution; the ten
factual corrections are recorded in `plans/clorraine-dx/05-findings.md`
(D1-D10: 114 registry rows, five `clc` render sites, fourteen `clorraine`
error prints, the two R5 cases, and the fixture/oracle corrections). The plan
then ran specification-first through all 28 tasks.

| Phase | Independent review | Resolution |
|---|---|---|
| DX Phase 1 (explain + help texts) | Correctness + semantics; no critical/major; six minor | Sanitized the unknown-code echo, used the fixture, recorded the red run, added the two Help headers, corrected `deleter` to `freedBy` |
| DX Phase 2 (JSON diagnostics) | Correctness; one major (`run.rs` bypassed the format), four minor | Threaded the format through `run.rs`, made explain errors honour JSON, escaped DEL/C1, removed a plan identifier, recorded the red run |
| DX Phase 3 (grammar) | Correctness; no critical/major; one minor scanner false-negative | Normalized `\b` in `word_bounded`; removed the inherited `i64`/`u64` highlight |
| `spec/14-diagnostics.md` | Semantics re-review; **no findings** | Twelve help cells and both headers verified against their owning chapters; status remains `stable (v1.0)` |

**Environment preflight:** `DISPLAY=:0` is exported, `docker/.env` carries
`DISPLAY`, `XAUTHORITY`, and `KICKSTART`, and `make verify` reached and passed
the FS-UAE corpus leg.

**Evidence:** DX plan 28/28; `make check` exit 0 (91 test-result groups,
`./spec/check.sh` OK with 27 files and 111 compile-fail tests); `make verify`
exit 0 with `verify: all legs passed` (differential, corpus-fs-uae, and the six
runtime faults); `make demo-check` exit 0 with 78 passed and 1 skipped;
spec suite 24/24; impl suite 12/12; registry/diag unit tests 19/19. The host
lacks `node`/`npm`, so the exact `make demo-check` target ran with a temporary
Node 22.23.4 toolchain on `PATH`; the target's commands were unchanged.

**Accepted residual:** the E90003 help says the `stores`/`transfers` hand-off is
modelled with explicit moves and `freedBy` deleters; the `stores` half is
guidance pending a user wording ruling, recorded as SR-001. PLAN-05 is marked
Done in the roadmap; the installer, LSP, and VSCode packaging remain backlog.

## Phase 6 Review (release readiness)

The independent correctness/security review found no critical or major findings
and two minor findings. Both are fixed:

| # | Severity | Finding | Resolution |
|---|----------|---------|------------|
| RV-001 | Minor | The known-limitations introduction claimed that everything outside the three-item list works as specified, which overstates the pre-existing leak/order residuals recorded in this ledger | The blanket sentence is removed; the document keeps exactly the three AR-14 items and no catch-all |
| RV-002 | Minor | Tasks 6.1.1-6.1.4 and the plan/roadmap progress were stale | The tasks, the 34/45 progress, the index, and the roadmap now record the phase |

**ST-13:** `docs/known-limitations.md` lists exactly the ASan `-no-pie`
requirement, the top-level-`while`-only loop split, and the host-only CI
limitation; both links resolve and `./spec/check.sh` passes.

**ST-14:** the MIT `LICENSE`, `compiler/Cargo.toml` (`1.0.0`, MIT, description,
repository), `Cargo.lock` (`clc 1.0.0`), both demo manifests, and the README
licence section are consistent; `clc --version` and `clorraine --version` both
report `1.0.0`.

**Evidence after the version bump:** `make verify` exit 0 with
`verify: all legs passed`; `make asan` exit 0 with 24 suites; `./spec/check.sh`
OK (27 files, 111 compile-fail tests). `RELEASE.md` records the AR-12 decision,
the four gates, the licence audit, and the conditional publish steps; nothing
was tagged or published.

## Phase 7 Review (final production gate)

The independent final-gate audit found **no blocking findings** and no false
closure claim. It consumed the prior phase reviews and checked the gate state
directly.

| Gate | Result |
|------|--------|
| `make check` | exit 0; 92 `test result: ok` groups, 3,153 passed, 0 failed; `./spec/check.sh` OK (27 files, 111 compile-fail tests) |
| `make verify` | exit 0; `verify: all legs passed` (differential 3/3, FS-UAE corpus, six runtime faults) |
| `make asan` | exit 0; 24 suites, serial, with the `-no-pie` wrapper |
| `make differential` | exit 0; 3/3 programs match |
| DX plan | 28/28; `spec/14-diagnostics.md` re-reviewed after the help edit |
| PLAN-13 | 16/16; `exec-self-test`, `message-drop`, and Hamurabi match their transcripts under `vamos` |
| Specification | chapters 00-17 and appendices A/B `stable (v1.0)`; appendices re-reviewed against `std.amiga.exec` |
| Release | MIT LICENSE, 1.0.0 metadata, lockfile, demo manifests, `THIRD_PARTY_NOTICES.md`, three-item known limitations linked from README/spec, `RELEASE.md` with AR-12 |
| CI | no push and no run URL claimed; local host-only workflow validation |
| Security | no secrets, NDK, ROM, or new network/credential surface |

**Conditional publish:** 0/2, explicitly blocked. No tag, binary artifact,
checksum file, or release note was created; publication requires explicit user
authorization. The main 45-task plan closes production-ready, as the success
criteria allow.

**Close-out notes applied:** the roadmap PLAN-15 row and header are current;
`RELEASE.md` records 92 passing groups; the parent index records all 15 AR
entries resolved. The specialist-coverage count in `AGENTS.md` remains the
2026-10-31 snapshot (51 plan folders; the tree now has 53); it is a CodeOps
bookkeeping refresh, not a v1.0 gate.
