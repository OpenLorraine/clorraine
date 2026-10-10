# Ambiguity Register: Evaluation and Emission Integrity

> **Status**: ✅ GATE PASSED — all 16 items resolved (6 user-decided; 4 conformance to the frozen
> spec; 6 auto-design-delegated)

| # | Category | Ambiguity / Gap | Options Presented | Decision | Status |
|---|----------|-----------------|-------------------|----------|--------|
| AR-B1 | Scope | Which defects the batch covers | the agreed 18 (majors + rulings + minor sweep) / also the five fresh rows / trim | **User:** the 25 roadmap rows — majors DEF-95, DEF-96, DEF-107, DEF-111, DEF-117, DEF-120, DEF-121; rulings DEF-119, DEF-124; minors DEF-76, DEF-80, DEF-90, DEF-97, DEF-98, DEF-99, DEF-103, DEF-108, DEF-112, DEF-118, DEF-122; fresh rows DEF-125, DEF-126, DEF-127, DEF-128, DEF-129. DEF-103 closes as the documented limitation (no code change). Fixes conform to the frozen rules where those decide the behavior; the only spec edits are the E10043 row, the E10042 wording, the E10212 trigger, and their goldens (AR-B6). | ✅ Resolved |
| AR-B2 | Behavioral | DEF-95: a view bound from an owned projection temporary leaks per iteration | scope-owned owner / reject the shape | **User:** the binding takes ownership of the backing temporary and releases it at the binding's scope end; the shape stays accepted and becomes memory-correct. | ✅ Resolved |
| AR-B3 | Behavioral | DEF-103: the `Q` key marker collides with a user `StringQ` in nested positions | keep as documented / clean checker diagnostic / escaping scheme (ABI) | **User:** keep the documented limitation — the collision fails closed under the strict build (a struct redefinition, never a runnable wrong program); the row closes with the evidence recorded and no code change. An escaping scheme remains an ABI decision with its own trigger. | ✅ Resolved |
| AR-B4 | Behavioral | DEF-108: `let x = null;` emits `i32 x = NULL;` | keep nullable inference / reject | **User:** keep the nullable inference — the binding keeps the literal's nullable type and the declaration/null spelling are made to agree. | ✅ Resolved |
| AR-B5 | Behavioral | DEF-122: a slice-typed binding from an array literal emits an array assignment | reject at the checker / static backing storage | **User:** reject at the checker — the literal has no backing to view; the sanctioned spelling is `slice(array, lo, hi)`. | ✅ Resolved |
| AR-B6 | Diagnostics | The DEF-118 and DEF-129 landing set | E10043 new + E10212 amended / E10043 + E10212 unchanged / reuse E10209 | **User:** approve both — new `\| E10043 \| Element loop over a non-iterable type \| \`type '{type}' cannot be iterated\` \| iterate a fixed array or a slice \|` with its golden and driver registration; E10212's trigger becomes `Unknown member of a struct, enum, module, or type parameter` (the message and code are reused). | ✅ Resolved |
| AR-B7 | Behavioral | DEF-107: `str` pair equality is accepted and emits a C struct comparison | support / reject | **Conformance:** the frozen operator table (spec/04-expressions.md:104) covers scalars, enums, `raw`, and handles — views are outside it, so `str`/`static str` pairs report `E10222`; the null-pair exemption stays and `cstring` keeps its pointer comparison. | ✅ Resolved |
| AR-B8 | Behavioral | DEF-118: a `str` element loop is accepted and emits broken C | support typed `u8` views / reject | **Conformance:** the frozen element-loop rule (spec/08-arrays-strings.md §5) walks a fixed array or a slice; a `str` loop is outside the surface and reports the approved `E10043` (the shape never built, so nothing working regresses). | ✅ Resolved |
| AR-B9 | Behavioral | DEF-129: a member call on a bare parameter receiver emits `foo(x, 5)` | reject / support | **Conformance:** a bare `T` has no resolvable members, so the member callee reports the approved `E10212` at the callee, one report per site; concrete receivers and callable-bound members are unaffected. | ✅ Resolved |
| AR-B10 | Technical | DEF-119: value-position updates bypass the write checks | (user ruling recorded) | **User ruling (2026-10-10):** route value-position updates through the statement-form checks — `read_place` then `write_place` for locals, the const and immutable-view checks for globals — keeping exactly one report per site; the statement path is untouched. | ✅ Resolved |
| AR-B11 | Behavioral | DEF-124: `cstring` element writes are accepted although immutable | (user ruling recorded) | **User ruling (2026-10-10):** extend `E10042` to `cstring` — the predicate gains `TyKind::CString` and the registry row becomes `\| E10042 \| Write to an element of an immutable view (\`str\`, \`static str\`, or \`cstring\`) \| \`cannot write through immutable view '{name}'\` \| copy the bytes into an owned \`String\` or a \`borrow\`-derived \`u8[]\` view \|` (whole-view rebinds and writable slice writes stay accepted); the existing golden satisfies the coverage rule. | ✅ Resolved |
| AR-B12 | Behavioral | DEF-126: a block-bodied lambda return of a by-value generic capture reports `E10034` | (ruled consequence) | **Conformance to the AR-5 ruling:** the not-lend duty is suppressed for capture bindings (`base = Some(EnvField)`), the capture-move duty records at the block return and the take sites, and the report is the ruled `E10401`; the closure batch's RV-004 pin (block take → `E10034`) is updated to the ruled code as its documented consequence. | ✅ Resolved |
| AR-B13 | Technical | Implementation mechanisms for the 23 code fixes | per-defect levers (the recon fact sheets) | **AI — delegated by --auto-design (eligible: compiler mechanisms inside user-confirmed scope and conformance outcomes).** Each fix takes the smallest recorded lever, all evidenced: DEF-117 a module-init materialization pass mirroring the function path's hidden-destination contract across the six positions (`codegen/program.rs:640`, `codegen/expr.rs:508`); DEF-121 the guarded-sub-operand materialization in the scalar single-assignment path (`program.rs:854`); DEF-122 stop adopting a `None`-bearing expected extent in `check_array_literal` (`sema/expr.rs:755-761`), letting `check_conversion` report `E10200` for both module and body spellings; DEF-76/97 skip the owned-argument binding for `len` of a fixed array (`own/lower.rs:1491`), matching the concrete path's existing drop; DEF-80 a `cl_zero_<ClType>` struct zero-helper family returned by `empty_return_value` (`codegen/cleanup.rs:922`); DEF-90 `typedef unsigned char bool;` in the freestanding preamble (`program.rs:88-99`), matching the 1-byte bool; DEF-98 the slice-to-pointer coercion for uncontracted extern parameters at the call site (`codegen/calls.rs:185`), prototype stays `APTR`; DEF-99 reproduce first, then fix the fold fallback (`codegen/calls.rs:239`) in the DEF-111 class; DEF-111 resolve the projected field through the substituted base at the value path (`codegen/expr.rs` field resolver vs `own/lower.rs:645`); DEF-112 seed the generated spellings (`__cl_ret`, `__env`, `__cl_tmpN`, `__cl_ownedN`, `__cl_itmpN`) into `used_names` (`codegen/function.rs:83,185`) — no new rejections; DEF-120 split slice-typedef emission — pointer-element before the struct definitions, complete-row-element after (`program.rs:145`, `codegen.rs:368-369`) — and revise the store-emission AR-8 order record; DEF-125 the func-value call takes its modes from the binding's own func type (`own/call.rs:211`, `call_check.rs:55`, `call.rs:386`); DEF-127 conditional arms recurse in the rejection and the recorder with per-site dedupe (`sema/expr.rs:274-335`, mirroring `own/expr.rs:109`); DEF-128 resolve the bounded member call through the instance field/closure call path in the emission — the smaller of `own/call.rs:111-137` (`func_field_receiver` accepting a callable-bound `Param` field) or a codegen-time re-resolution of the receiver-as-first-argument shape (`codegen/expr.rs:520-589` via `instance_field`) — chosen by probe; DEF-96 arm the superseded-result gate for an ordinary store when a defer in scope has an out-edge (`codegen/cleanup.rs:455`, reusing `defer_region_has_out_edge:931`) under the byte-identity constraint; DEF-95 the binding-scoped owner through the existing owner machinery (`own/statements.rs:302-384`, `own/lower.rs:1689-1719,1357`) with `StructLit` descent in the temp-owner walk (`:1130`). | ✅ Resolved (delegated) |
| AR-B14 | Process | Verification commands and gates | detected project commands | **AI — delegated by --auto-design (eligible: testing strategy; commands detected from `AGENTS.md`/the `Makefile`).** Per phase: `make check` and the batch suite in the pinned container; host ASan probes are the memory oracle for the DEF-95/96/121/125/127 family; byte-identity checks for the emission-order and name-seeding fixes; `make verify` at close-out. The two new goldens join `./spec/check.sh` and the compile-fail driver. | ✅ Resolved (delegated) |
| AR-B15 | Naming | Suite and module names | follow the batch convention | **AI — delegated by --auto-design (eligible: internal naming).** Folder `plans/eval-emission-integrity/`; suites `compiler/tests/eval_emission_integrity_spec_test.rs` and `_impl_test.rs`; modules `spec.eei.*`; no new language syntax names. | ✅ Resolved (delegated) |
| AR-B16 | Sequencing | Phase structure and review dispatch | one phase per layer (5) / per defect | **AI — delegated by --auto-design (eligible: implementation sequencing).** Five phases by layer: P1 module-initializer integrity (DEF-117, 121, 122); P2 strict-build emission shapes (DEF-76, 97, 80, 90, 98, 112, 120, 128); P3 value typing and checker conformance (DEF-99, 111, 108, 107, 118, 129); P4 write-mutability rules (DEF-119, 124); P5 ownership lifetimes and releases (DEF-95, 96, 125, 126, 127); DEF-103 closes in the plan's close (no phase). Review dispatch per the `AGENTS.md` matrix: P1/P2 correctness + emission-integrity (plus the memory-safety lens where a fix changes guards or releases); P3/P4 correctness + semantics; P5 correctness + memory-safety + semantics. | ✅ Resolved (delegated) |

## Resolution Notes

**AR-B1 (user):** the scope is the agreed batch-B membership plus the five fresh rows from batch A's
reviews; DEF-103 closes with its evidence instead of a change. The conformance framing carries over
from batch A: where a frozen rule already decides the behavior, the implementation conforms.

**AR-B6 (user):** the E10043 row is drafted in the table above; its golden
(`spec/tests/compile-fail/E10043_str_loop.clo`) and the compile-fail driver registration land with
it, and the E10212 trigger amendment changes no code or message. Both edits are the batch's only
registry changes besides the E10042 wording.

**AR-B7 / AR-B8 / AR-B9 (conformance):** the operator table
(`spec/04-expressions.md:104`), the element-loop rule (`spec/08-arrays-strings.md:124-137`), and the
member rules decide the outcomes; the register records them so no later step re-litigates.

**AR-B12 (conformance to the AR-5 ruling):** the change completes the batch-A ruling for the block
forms; the review-observed `E10034`/`E10401` duplicate is removed by suppressing the not-lend duty
for captures rather than by leaving one shape on the wrong code.

**AR-B13 (delegated):**

```text
Authority: AI — delegated by --auto-design
Eligibility: compiler mechanisms inside user-confirmed scope, rulings, and conformance outcomes; no new surface beyond the approved diagnostics
Objective: each defect fixed with the smallest evidenced lever from the recon fact sheets
Decision: the per-defect levers listed in the table
Evidence: each lever cites the recon fact sheet's file:line mapping (three read-only fact sheets, 25 rows, current HEAD 6548954)
Rejected alternatives: the larger workstream variants (static backing storage for slice literals, an escaping ABI scheme, environment-owned captures) were already user-declined; within-lever alternatives are recorded per component doc
Strongest counterargument: the batch reuses older gates whose precision assumptions (for example byte-identity for ordinary returns) could conflict with the new arms; the plan keeps the identity constraints explicit per fix
Confidence: High
Hardening: in-context layers only (established mechanisms; per-phase reviews are the outer layer)
Policy version: 1
Root invocation ID: plans-eval-emission-integrity-20261010
Reopen triggers: a fix whose probe shows the recorded lever cannot reach the defect without a surface change
```

**AR-B14 / AR-B15 / AR-B16 (delegated):** recorded in the table with their eligibility and
evidence; each follows an established repository mechanism (the detected verify commands, the
`*_integrity` batch naming, and the `AGENTS.md` review matrix), so no further note is needed.

**Scan completeness (12 categories):** Feature gaps — the per-defect requirements (01) and
components (03-01 – 03-05). Behavioral — AR-B2 – AR-B9, AR-B11, AR-B12. Scope — AR-B1. Technical
unknowns — AR-B13 (with the DEF-99/K6 reproduce-first caveat), AR-B14. Edge cases — the per-defect
edge lists in 07 (both spellings of each shape, the multi-position init forms, the
module/function-body parity, the profile legs). Integration points — sema (`ops.rs`, `stmt.rs`,
`call.rs`, `expr.rs`), own (`lower.rs`, `call.rs`, `place.rs`, `statements.rs`, `loops.rs`), codegen
(`program.rs`, `calls.rs`, `function.rs`, `cleanup.rs`, `types.rs`, `expr.rs`), the diagnostics
registry and the compile-fail driver, and the freestanding shim; owned by 03-01 – 03-05. Data &
state — no new data model; the existing IR/mono/duty machinery. Security & compliance — memory
safety is the surface; ASan probes cover the leaks and double-frees (DEF-95, 96, 121, 125, 127).
Non-functional — determinism (byte-identical re-emission) and strict-build correctness follow the
suite conventions. UX & presentation — the two approved diagnostic edits (E10043, E10042, E10212)
are the only user-facing text. Stakeholder conflicts — none. Naming — AR-B15.
