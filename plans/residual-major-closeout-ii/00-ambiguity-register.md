# Ambiguity Register: Residual Major Closeout II (Batch L)

> **Status**: ✅ GATE PASSED — the planning decisions are recorded (auto-design delegated); no
> batch row is parked; the six older rows and the minors stay out (AR-2); two rows are
> reclassified as incidentally fixed and carried as pinned controls (`DEF-341`, `DEF-350`; AR-3,
> AR-8); the behavior choices inside the frozen rules are resolved in AR-6, AR-7, and AR-13
> **Created**: 2026-10-16 (planning)
> **Batch**: L of the defect mandate (the 23 remaining un-parked 🟠 majors; 21 red, 2 verified
> incidentally fixed)
> **Auto-design**: active — the standing user mandate: all defects first, batches continue
> without further go-aheads; severity-first across the backlog (all remaining un-parked majors).
> Reserved decisions (spec text, new user-visible behavior, acceptance criteria) are **not**
> touched: every fix below is conformance to the frozen specification. A row whose fix needs a
> spec or registry ruling is parked and collected in AR-2.

## Why this batch

Batches I/J/K fixed the criticals and the earlier majors; the remaining open backlog is led by
exactly these 23 🟠 majors — the freshly filed follow-ons of the batch-J and batch-K phase
reviews. Each row carries a reproduction, a base/head observation, and a fix direction, so one
batch grouped into eight machinery-coherent phases (AR-4) closes them with the origin suites as
the regression oracle. All 23 were re-probed against the compiler built from head `8829be6`
(AR-3); two turned out to be fixed by the batch-K corrections and become controls.

| # | Category | Ambiguity / Gap | Options Presented | Resolution | Status |
|---|----------|-----------------|-------------------|------------|--------|
| 1 | Scope | Which rows batch L closes | A) the 23 remaining un-parked 🟠 majors, severity-first; B) the six older ruling-parked rows; C) the minors | **A** — pre-decided by the dispatch and recorded, not re-litigated: all remaining majors, filed with reproductions, grouped by machinery. B stays parked (rulings due); C forms the batch after this one | ✅ Resolved (pre-decided) |
| 2 | Scope | Rows excluded on purpose | — | **The six older ruling-parked rows** (`DEF-167`, `DEF-169`, `DEF-171`, `DEF-190`, `DEF-191`, `DEF-263` — including the majors `DEF-167`/`DEF-171`) **and the minors** (`DEF-311`–`DEF-359` minus the batch's 23), which are the next batch. None of the 23 needs a spec or registry edit | ✅ Resolved (pre-decided) |
| 3 | Verification | The current status of the 23 rows at the head | probe the actual compiler (`8829be6`) against the recorded probe sources | **The AR-3 probe table below** — 21 red; `DEF-341` fixed incidentally by the batch-K phase-2 C3 correction; `DEF-350` fixed incidentally by the batch-K phase-4 F1 correction; several rows are partial with probe-verified green controls (recorded per row) | ✅ Resolved |
| 4 | Design | Phase grouping and sequencing | A) eight phases by machinery, executed 1 → 8; B) one row per phase; C) the origin batches' phases | **A** — P1 nullable-view arms; P2 walk resolver; P3 instance substitution/element surfaces; P4 ownership transfer judgments; P5 store emission; P6 call conversions/`ffi`; P7 member typing; P8 module/global identity and the const folder. Option C re-splits rows that were filed across four reviews; one row per phase multiplies the review overhead. The order follows the dependency direction (AR-4) | ✅ Resolved (delegated) |
| 5 | Design | Per-row handling mode | red → green spec case, or a control where the probe shows the row fixed | **Red → green for 21 rows**; `DEF-341` and `DEF-350` are green controls (their probed shapes stay rejecting/building); the partial rows carry their probe-verified green twins as controls (e.g. `DEF-337`'s body spelling, `DEF-340`'s `id(v[i].y)`, `DEF-357`'s value folds). Every red case must fail without the fix | ✅ Resolved (delegated) |
| 6 | Behavior | `DEF-322`'s element store through a conditional/projected base | A) route the per-arm base (the read spelling already routes through the conditional); B) reject with the frozen `E10221` place diagnostic | **B (amended by the preflight PF-001 ruling)** — the frozen place rule (Ch 04 §4 / EX-14: a local, a parameter, a struct field, an array element, or a dereferenced raw pointer, otherwise `E10221`) does not list a conditional base, and the scalar member twin already reports `E10221`; the read spelling needs no place, so its acceptance does not sanction a store. Route A is parked: it would add an acceptance the frozen place list does not define (AR-12); it would also accept what the scalar twin rejects | ✅ Resolved (preflight-delegated) |
| 7 | Behavior | `DEF-347`'s view `.ptr` at a mutable raw destination | A) emit the sanctioned member cast at the destination (the `cstring` member-conversion shape); B) reject with the conversion duty | **A** — the sanctioned spelling `raw u8(s.ptr)` inside `unsafe` builds (probe `ctl347`), so the destination cast is the same conversion the store slots already spell for `cstring`; no new conversion is added. B remains the fallback if the semantics review rules the drop-qualifier conversion illegal | ✅ Resolved (delegated, stop-gated) |
| 8 | Verification | The two incidentally fixed rows | — | `DEF-341`: generic and concrete both report one `E10401` (the batch-K phase-2 C3 capture-move correction replaced the invalid C with the frozen rule; the concrete parity is the proof). `DEF-350`: the view/slice destinations reject `E10200`, `len(Make())`/`Make()[0]`/the cstring return build (the batch-K phase-4 F1 correction typed the result as the ABI pointer); the residual `.ptr`/`.len` spellings are the separately filed minor `DEF-352`. Both are pinned as controls in their phases | ✅ Resolved |
| 9 | Naming | New suite pair vs origin suites | — | **Origin suites, additions only**: P1/P8(`DEF-357`) → `module_initializer_integrity_{spec,impl}_test.rs`; P2/P3/P6/P7 → `checker_conformance_{spec,impl}_test.rs` (the `DEF-341` capture-pair control may ride `conditional_projection_residue_{spec,impl}_test.rs` with the P4/P5 cases); P4/P5 → `conditional_projection_residue_{spec,impl}_test.rs`; P8(`DEF-356`/`DEF-359`) → `callable_release_integrity_{spec,impl}_test.rs`. A new `residual_major_closeout_ii_{spec,impl}_test.rs` pair is created only if a phase cannot host a case naturally (AR-9) | ✅ Resolved |
| 10 | Lenses | Per-phase reviewer dispatch | — | P1: correctness + memory-safety + emission-integrity; P2: correctness + semantics + memory-safety + emission-integrity; P3: all four; P4: correctness + memory-safety + semantics; P5: correctness + memory-safety + emission-integrity; P6: correctness + semantics + memory-safety + emission-integrity; P7: correctness + semantics + emission-integrity; P8: correctness + semantics + emission-integrity. The P2/P6 sets follow their batch-K sibling phases K2/K4, which carried the memory-safety auditor (preflight PF-005); P7/P8 follow K3/K6. The read-only auditors stay read-only | ✅ Resolved (PF-005 amended) |
| 11 | Verification | The command set | — | `make check` per phase; the focused origin suites; `RDC_ASAN=1` memory runs on P1/P3/P4/P5; strict builds on the pinned container; `make verify` at close-out. The `ffi` probes that call an unimplemented extern compile-only (the filed evidence is the strict *build*) | ✅ Resolved |
| 12 | Design | No spec, registry, or golden edit | — | Every in-scope fix is conformance: no new diagnostic code, no registry row, no spec chapter edit, no golden change; the rejections reuse `E10012`, `E10200`, `E10212`, `E10221`, `E10401`, and the runtime traps. A row that turns out to need a ruling is parked mid-phase (AR-14) | ✅ Resolved |
| 13 | Design | `DEF-357`'s ABI-layout-faithful `sizeof` | A) defer the extent-position fold to the C type size (`u8[sizeof(ClS)]`, as the value position already emits); B) make the folder compute the padded layout | **A** — the value position already emits `sizeof(ClS)` (probe `def357a`/`def357b` run 0), so deferring the *extent* position to the same spelling is the smaller fix and keeps one fold contract. B would add a layout engine for an operand the C `sizeof` already answers; it is rejected unless A proves impossible (stop-gated) | ✅ Resolved (delegated, stop-gated) |
| 14 | Complexity | Escalation triggers and the park protocol | — | None — every fix extends existing machinery (the view widening, the walk resolver, the instance substitution/audits, the ownership judgments, the store/literal emitters, the conversion duties, the member dispatch, the identity/const sites). A row whose fix needs a spec/registry edit or a new surface is parked with the evidence and recorded in the findings ledger | ✅ Resolved |
| 15 | Deviation (runtime) | A phase's fix surface proves wider than the row | A) stop and re-plan; B) extend the phase's task files to the adjacent module with the evidence and record the deviation here | **B** — the batch I/J/K precedent: a conformance fix may touch the module that owns the judgment; the deviation is recorded and the phase review covers the extra files | ✅ Resolved (runtime, delegated) |
| 16 | Specialist gap | Missing specialist types for this surface | — | None — `correctness-reviewer`, `semantics-reviewer`, the read-only `memory-safety-auditor`, and the read-only `emission-integrity-auditor` cover the batch; recorded in the index's Specialist Agents section | ✅ Resolved |
| 17 | Behavior | `DEF-359`'s checker typing of a bare func-global call | — | The checker types the callee through the binding the *scope* resolves (the caller's module or its imports), not the program-wide simple-name map, so `f("x")` against an imported `func(i32)` global reports the frozen argument conversion instead of emitting invalid C while a same-named `func(str)` exists elsewhere. Conformance to M10/MOD-13; no spec edit | ✅ Resolved (delegated) |

## Verified Current Status (AR-3 probes, 2026-10-16, head `8829be6`)

Probe method: `make -C docker exec CMD="cargo build --workspace"`, then the compiler
(`target/debug/clc --profile host`, `--profile freestanding`/`hosted-amiga` where a row requires)
against the recorded probe sources; the emitted C compiled with
`cc -std=c99 -O0 -g -Wall -Wextra -Werror` (GCC 12.2, the pinned container) and run; the memory
rows additionally built with `-fsanitize=address -no-pie` and run with `ASAN_OPTIONS=detect_leaks=1`.
Probe sources: the recorded `build/` tree (paths below) plus a small scratch set under
`build/batch-l-probes/` for the rows whose filed probe was not preserved (removed at planning
close). Multi-file probes pass every `.clo` file on the command line (the file list is the
program).

| Row | Probe (recorded source) | Verified current behavior |
|-----|-------------------------|---------------------------|
| `DEF-315` | `memory-audit/final-audit/hp_alias_copy.clo` | ❌ Red: accepted, strict build ok; run 215; ASan `heap-use-after-free` (the `str[2]` copy transfers no element loans; `fill(ys[0], borrow slot)` reads freed storage) |
| `DEF-322` | `memory-audit/j4/def322_probe.clo` | ❌ Red: accepted, strict build ok; run 251 — `(c ? b1 : b2).ys[i] = 7;` emits no store and drops the RHS; the scalar member twin reports `E10221` (probe `ctl322`, the fallback/control) |
| `DEF-323` | crafted `def323a`/`def323b` per the row | ❌ Red: the `take` spelling is accepted and the strict build fails (`too few arguments to function 'cl_..._n1'`); the plain spelling builds and runs 10 (the skipped arm's call ran: calls=3 where 2 expected); the direct-literal take control (`ctl323`) builds and runs 0 |
| `DEF-329` | `memory-audit/batchJ-p4-final/probes/rawreadown.clo` | ❌ Red: accepted, strict build ok; run abort 134; ASan `attempting double-free` (`let v: String = unsafe *q;` creates a second owner) |
| `DEF-335` | `memory-audit/k1/k1d10_nullarm_call_taken.clo` (call arm); crafted `def335f`/`def335g` (local/parameter place arm) | ❌ Red: the call arm runs 139 and ASan reports a null-deref crash (`(c ? cl_string_view(maybe()) : cl_string_view(...))`); the local and parameter place arms run 139. Controls: `k1d9` (call arm not taken) run 2, `k1m1` (non-null body place) run 0, `k1d17` (body call) run 0 — all clean |
| `DEF-336` | `memory-audit/k1/k1d15_nullarm_arg.clo` | ❌ Red: accepted, strict build ok; run 139 + ASan (`cl_string_view(__cl_owned0)` as an argument, unguarded) |
| `DEF-337` | `memory-audit/k1/k1e6_member_call_guard.clo` (module) | ❌ Red: accepted, strict build ok; run 139 + ASan (`cl_string_view(__cl_itmp1.name)` on a null handle); the body twin `k1j1` runs 0 (control); the body-shaped member probe runs 0 |
| `DEF-338` | `emission-audit/k1/variants/d03_init_method_wrongkey2.clo` | ❌ Red: accepted, strict build fails (`incompatible type for argument 2 of 'cl_pick__Box'` — the local-headed qualified receiver keys no instance) |
| `DEF-339` | `k2-audit/src/s2_fieldstore_loop.clo` | ❌ Red: accepted, strict build ok; run 26; LeakSanitizer reports the overwritten owner of each `v.name = ...` store (11 B per store) |
| `DEF-340` | `k2-audit/src/b8_owningfieldarg.clo` (`keep(v.name)`) | ❌ Red: accepted, strict build fails (`implicit declaration of 'cl_keep__String'` — the walk records no instance for a temp-rooted projection; the emitter keys it). Control: `K2S-review/src/p2f_elem_member.clo` (`id(v[i].y)`) builds and runs 7 |
| `DEF-341` | `k2-audit/src/b10_capturereturn.clo` + crafted concrete twin | ✅ **Incidentally fixed**: both the generic and the concrete spelling are rejected with one `E10401` (`cannot move captured 'v' in a lambda`) — the batch-K phase-2 C3 correction substituted the capture-move requirements, so the invalid C is unreachable and the concrete parity holds. Pinned as the phase-3 control |
| `DEF-342` | `K2S-review/src/p2b_condarg.clo` (`id(c ? a.y : b.y)`) | ❌ Red: accepted, strict build fails (`implicit declaration of 'cl_id__i32'` — the conditional base is declined). Controls: `p2d2_borrow_local.clo` (`id(s.y)` through a local) runs 3; crafted `def342c` (`id(o.s.y)` non-local place) runs 1 |
| `DEF-344` | `memory-audit/k4p4/dn1_def344_pair.clo` (`pair(1, x)`) | ❌ Red: accepted, strict build fails (`passing argument 2 of 'cl_pair__i32' makes integer from pointer`). Controls: `dn2` (`pair(x, 1)`) rejects one `E10012`; `dn3` (`pair(x, x)`) rejects two `E10012` (the cross-lens count question recorded in the row) |
| `DEF-345` | `emission-audit/k2/probes/e04…e12` | ❌ Red: all seven spellings (place return, conditional return, direct element return, element let, element argument, loop element return/let) are accepted and fail the strict build |
| `DEF-346` | crafted `def346a`/`def346b` per the row | ❌ Red: `let p = a.ptr;` on a `u8[4]` and `let x = s.foo;` on a `str` are accepted and fail the strict build (`request for member 'ptr'/'foo' …`) |
| `DEF-347` | crafted `def347` per the row | ❌ Red: `let p: raw u8 = s.ptr;`, the `raw u8` return, and the struct field are accepted and fail the strict build (`discarded-qualifiers`); the sanctioned cast (`ctl347`, `raw u8(s.ptr)`) builds and runs 0 (control) |
| `DEF-348` | crafted `def348` per the row | ❌ Red: `i32(p.A)` on a `raw E` is accepted and fails the strict build (`request for member 'A' …`) |
| `DEF-350` | `memory-audit/k4p4/h8`/`h9`; `emission-audit/k4/variants/fr17*` | ✅ **Incidentally fixed in core** (batch-K phase-4 F1): the view/slice destinations now report one `E10200` (`h8`/`h9`/`fr17c`/`fr17f`), the template reports `E10500` (`fr17l`), `len(Make())` and the slice length build and run 0 (`fr17b`/`fr17e`), the cstring return and the index compile (`fr17d`/`fr17h`). Residuals belong to other rows: `Make().ptr`/`Make().len` fail the strict build = the filed minor `DEF-352`; the inferred local leaves `s` unused after the `len` fold (the `len`-surface family). Pinned as the phase-6 control |
| `DEF-351` | `K4S-sem/reverse1.clo`, `gencallee.clo`, `genmixed.clo`; `emission-audit/k4/variants/bs01_explicit_param_arg.clo` | ❌ Red: all four are accepted and fail the strict build (`returning 'int' … 'char *'`, `incompatible type for argument 1 of 'cl_consume2__cstring'`, `pointer from integer`, `pointer-sign`). Control: `K4S-sem/genlit.clo` rejects one `E10200` |
| `DEF-353` | `memory-audit/p5/p5_capture_dup_instance.clo` | ❌ Red: accepted, strict build ok; run 249 (`out` stays 0 — the `borrow out: i32` instance's store is dropped; no memory error) |
| `DEF-356` | `k18/probes/def356/alias_fn.clo` + holder (aliased import); crafted `def356d` (cross-module qualified value head) and `def356b4` (self-qualified, control) | ❌ Red (partial): the aliased import emits `cl_def356holder_bg` (undeclared, strict build fails); the cross-module qualified value head `q.holder.g(5)` is rejected `E10219`. Controls: the self-qualified `b.holder.g(5)` with a current-module `let g` builds and runs 0 (fixed by the batch-K K6C-1 correction); the plain import and the qualified func-global build and run 0 |
| `DEF-357` | `k18/probes/def357/{extent,extent_view,structpad,view}.clo` | ❌ Red (partial): the extent-position folds are wrong — `u8 pad[5]` for `sizeof(S)` (host 8) runs 253, `u8 buf[8]` for `sizeof(str)` (host 16) runs 248; the value positions are green controls (`structpad`/`view` run 0 — they emit `sizeof(ClS)`/`sizeof(ClStr)`) |
| `DEF-359` | `k18/sig/{main2,holder,other}.clo` | ❌ Red: accepted, strict build fails (`incompatible type for argument 1 of 'cl_sigcholder_f'` — the checker types the bare call by simple name while a same-named `func(str)` exists elsewhere) |

**The two reclassifications**: `DEF-341` and `DEF-350` are verified fixed at the head (the
batch-K corrections closed their filed failures) and are carried as pinned controls in their
phases; their residual spellings are recorded as other rows' scope (`DEF-352`, the `len`-surface
minors). Rows not probed beyond the table: none — all 23 were probed.

## Resolution Notes

**AR-1:** The dispatch fixed the scope as the 23 remaining un-parked 🟠 majors; the register
records it without re-litigating. The batch includes `DEF-341` and `DEF-350` as probe-verified
controls so all 23 roadmap rows are accounted for by the close-out.

**AR-2:** Parked rows and the decision each needs:
`DEF-167` — the diagnostic for a call of a non-callable callee (shared with `DEF-214`);
`DEF-169` — whether to add an escaping scheme for generated type keys (an ABI decision);
`DEF-171` — whether a staged `embed` outside a constant context stays rejected;
`DEF-190` — the diagnostic for a nameless nullable source;
`DEF-191` — whether the `unsafe` exemption for the null-use rule matches the conversion rule;
`DEF-263` — whether a folded fixed-array `.len` receiver evaluates its operand.
The minors (`DEF-311`–`DEF-321`, `DEF-324`–`DEF-334`, `DEF-349`, `DEF-352`, `DEF-354`,
`DEF-355`, `DEF-358` minus the closed ones) are the next batch's scope. None of the 23 in-scope
rows needs a spec or registry edit: every fix reuses the frozen rules and codes (`E10012`,
`E10200`, `E10212`, `E10221`, `E10401`, `E11211`, and the runtime traps).

**AR-3:** Probe evidence is the table above; the recorded probe sources live in the gitignored
`build/` tree and the scratch set was removed at planning close. The reclassified rows keep
their roadmap "Backlog" text until close-out, when they move to Done with the incidental-fix
evidence (the batch-K `DEF-209` precedent).

**AR-4:** The phase grouping and dependency order:
P1 nullable-view arms (the guard widening feeds every later view store) → P2 the walk's
projected-argument resolver (the resolution P3's instance work extends) → P3 instance
substitution/element surfaces (the audits P6's duties mirror) → P4 ownership transfer judgments
(independent) → P5 store emission (independent) → P6 call conversions and `ffi` typing (the duty
records ride P3's audits) → P7 member typing (independent) → P8 module/global identity and the
const folder (independent). Executed strictly 1 → 8; `make check` gates every phase.

**AR-5:** Handling modes: 21 red → green rows. `DEF-341` and `DEF-350` are green controls (the
exact probed shapes stay rejecting/building). The partial rows pin their green twins beside the
red case (`DEF-337`'s body spelling, `DEF-340`'s `id(v[i].y)`, `DEF-356`'s self-qualified and
plain spellings, `DEF-357`'s value folds, `DEF-344`'s `dn2`/`dn3`). Every red case must be
reproduced at the phase's red ref before implementation and flip green after it.

**AR-6 (amended by the preflight PF-001 ruling):** `DEF-322`'s fix is the frozen `E10221`
rejection the scalar member twin reports — the assignment-place judgment (`sema/ops.rs`'s
`is_place`/`check_assign`; the executor re-resolves) rejects the conditional/projected base
exactly as it rejects the scalar member twin, so the store is never silently dropped and the RHS
side effects never run. The per-arm route is parked: it is not grounded in the frozen place list
(Ch 04 §4 / EX-14: a local, a parameter, a struct field, an array element, or a dereferenced raw
pointer, otherwise `E10221`) and would accept `(c ? b1 : b2).ys[i] = 7` while the same base stays
rejected for a scalar member. The read spelling `(c ? b1 : b2).ys[i]` stays accepted (reads need
no place) and is the control.

**AR-7:** `DEF-347`'s conversion: the destination cast (`(u8 *)(s.ptr)`) at the mutable raw
destination, mirroring the sanctioned `cstring` member conversion the store slots already spell;
the `unsafe` gate stays required. If the semantics review rules the const-drop illegal, the
phase reports the frozen conversion code instead (stop condition, recorded).

**AR-8:** The incidental fixes: `DEF-341` — the batch-K phase-2 correction C3
(`check_capture_move_requirements` with `instance_bindings`) makes both the generic and the
concrete spelling reject one `E10401`; pin the pair and the parity. `DEF-350` — the batch-K
phase-4 correction C4/F1 types a view/slice `ffi` result as the ABI pointer, so the view
destinations report `E10200` and the length/index/cstring uses build; pin the fixed spellings and
record the residual ownership (`DEF-352`, the `len` fold).

**AR-9:** Test placement: additions only to the origin suites (AR-9 table in the index). If a
phase finds a case with no natural home in an origin suite, it creates
`compiler/tests/residual_major_closeout_ii_{spec,impl}_test.rs` with only that case (recorded at
the phase); the `AGENTS.md` test map gains the pair only at close-out if it is created.

**AR-10:** Lens justification: P1 changes view arm shapes with null dereferences (correctness +
memory + emission); P2 changes instance key resolution and makes previously invalid-C programs
run (correctness + semantics + memory + emission — the batch-K K2 sibling set); P3 changes
substitution, drop decisions, and a store (all four); P4 changes loan/move judgments with double
frees and UAFs (correctness + memory + semantics); P5 changes store emission with side-effect and
leak effects (correctness + memory + emission); P6 changes conversion duties and `ffi` typing and
makes previously invalid-C programs run (correctness + semantics + memory + emission — the
batch-K K4 sibling set); P7 changes the member surface and conversions (correctness + semantics +
emission — the K3 sibling set); P8 changes callee identity, the checker typing, and a fold
(correctness + semantics + emission — the K6 sibling set). The P2/P6 memory-safety additions are
the preflight PF-005 ruling.

**AR-11:** Verification: `make check` per phase; focused origin suites; `RDC_ASAN=1` on
P1/P3/P4/P5; strict builds (compile-only where an `ffi` extern is unimplemented, with the
recorded stubs where they exist); `make verify` at close-out.

**AR-12:** Every in-scope fix is conformance: no new diagnostic code, no registry row, no spec
chapter edit, no golden change. The rejections reuse `E10012`, `E10200`, `E10212`, `E10221`,
`E10401`, `E11211`, and the runtime traps.

**AR-13:** `DEF-357`'s fix: the extent-position fold emits the C type size
(`u8 pad[sizeof(ClS)]`), matching the value position's existing `sizeof(ClS)` emission; the
folder's other folds (`offsetof`, scalar/struct values) stay behavior-pinned. Stop condition: if
a C extent cannot take the deferred spelling on every profile, the phase parks the row with the
probe evidence.

**AR-14:** Complexity: every fix rides an existing hook — the view widening
(`codegen/expr.rs`, `calls.rs`, `function.rs`, `program.rs`), the walk resolver
(`own/generics.rs`), the instance substitution and audits (`own/instance_check.rs`,
`codegen/drop.rs`), the ownership judgments (`own/expr.rs`), the store/literal emitters
(`own/lower.rs`), the conversion duties (`own/call_check.rs`, `sema/call.rs`), the member
dispatch (`sema/expr.rs`), and the identity/fold sites (`codegen/expr.rs`, `own/call.rs`,
`consts/mod.rs`).

**AR-15:** The runtime deviation protocol follows batch I/J/K: a phase's fix may touch the module
that owns the judgment beyond the task's file list; the deviation is recorded in this register
and reviewed by the phase's lenses.

## Open Questions (collected for the user — none blocks execution)

- The six parked rows' rulings (AR-2) are still due; they stay out of this batch.
- `DEF-347`'s and `DEF-357`'s stop conditions may return a parked row with fresh evidence;
  `DEF-322`'s per-arm route is parked (preflight PF-001) and there is no remaining stop
  condition — the frozen `E10221` rejection is the phase's fix. The findings ledger records
  either outcome for the remaining stop-gated rows.
- The `DEF-344` count question (`pair(x, 1)` one `E10012` for two uses of the null `x` while
  `pair(x, x)` reports two) is settled inside the phase with the frozen per-site rules; the
  findings ledger records the ruling.

**AR-18 (ownership scope, auto-design):** The phase-2 memory audit found a double free on shapes the walk
resolver newly keys (`build/memory-audit/L2-audit/progs/b2_cond_owning_take.clo`; the Local-rooted twin
`b16`/`b17` reproduces byte-identically at the base, so the root pre-exists while the reachability is
introduced). Filed as `DEF-362` and **folded into phase 4** (the ownership-transfer phase): the `take`
move-out of an opaque member type must be recorded as a resolved per-instance duty, so the instance
emits the concrete twin's source emptying. Phase 4's scope is DEF-315, DEF-329, and DEF-362.

**AR-19 (scope ruling, auto-design):** The phase-3 spec authoring found that `DEF-353`'s fix direction
(a caller write-through for a duplicable out-slot) contradicts the frozen rules: a lend of a
duplicable value binds a copy and writes through it affect the copy only (Ch 03 3, Ch 06, Ch 16,
decision D9/V5), with the pre-existing `W10001` warning and a pinned deviation in
`def_backlog_closeout_impl_test.rs`. The row is **reclassified as a false positive** and closed; the
phase-3 ST-9 slot carries the conforming control instead (the copy read-back + the frozen probe
exit + the `String?` in-out slot control). The phase's implementation rows are `DEF-339` and
`DEF-345`; the `DEF-341` and `DEF-350` controls stand.

