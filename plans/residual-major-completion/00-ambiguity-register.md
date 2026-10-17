# Ambiguity Register: Residual Major Completion (Batch M)

> **Status**: ✅ GATE PASSED — the planning decisions are recorded (auto-design delegated); no
> batch row is parked; the two parked majors and the remaining minors stay out (AR-1); two rows
> are reclassified as already green and carried as pinned controls (`DEF-361`, `DEF-374`;
> AR-3, AR-31, AR-32); the behavior choices inside the frozen rules are resolved in AR-6 – AR-24
> **Created**: 2026-10-17 (planning)
> **Batch**: M of the defect mandate (the last 20 un-parked rows: 1 🔴 critical + 19 🟠 majors;
> 18 red, 2 verified green)
> **Auto-design**: active — the standing user mandate: all defects first, batches continue
> without further go-aheads; severity-first across the backlog (the critical first, then the
> majors). Reserved decisions (spec text, new user-visible behavior, acceptance criteria) are
> **not** touched: every fix below is conformance to the frozen specification. A row whose fix
> needs a spec or registry ruling is parked and collected in AR-29.

## Why this batch

Batch L closed the 23 un-parked majors of its round. The remaining open backlog is led by exactly
these 20 rows: the last 19 majors and the single remaining critical. Each row carries a
reproduction, a base/head observation, and a fix direction, so one batch grouped into seven
machinery-coherent phases (AR-4) closes them with the origin suites as the regression oracle. All
20 were re-probed against the compiler built from head `86d6f51` (AR-5); two turned out to be
fixed by earlier corrections and become controls.

| # | Category | Ambiguity / Gap | Options Presented | Resolution | Status |
|---|----------|-----------------|-------------------|------------|--------|
| 1 | Scope | Which rows batch M closes | A) the 20 remaining un-parked rows (1 critical + 19 majors), severity-first; B) the two parked majors; C) the minors | **A** — pre-decided by the dispatch and recorded, not re-litigated. B stays parked (`DEF-167`, `DEF-171`); C is the next batch. The dispatch's "79 minors" is the remaining backlog count after this batch: 2 parked majors + 77 minor rows (observed: 99 open rows = 1 critical + 21 majors + 77 minors; 20 close here, 2 majors stay parked) | ✅ Resolved (pre-decided, count corrected) |
| 2 | Process | The specification-first and zero-ambiguity discipline | — | **Spec cases first, then red, then implement, then green, then impl pins.** Every row's expected outcome is fixed below before any code: a frozen diagnostic (code + site), a strict-build-clean run with its exit value, or an ASan-clean run. No implementer question may remain; a discovered gap re-enters this register before code | ✅ Resolved (pre-decided) |
| 3 | Verification | The current status of the 20 rows at the head | probe the actual compiler (`86d6f51`) against reconstructed minimal reproductions | **The AR-5 table** — 18 red; `DEF-361` and `DEF-374` green at the head and carried as pinned controls. No row is parked: every fix reuses a frozen code | ✅ Resolved |
| 4 | Design | Phase grouping and sequencing | A) seven phases by machinery, executed 1 → 7; B) one row per phase; C) the origin batches' phases | **A** — P1 call identity/written arguments; P2 substitution edges I (conditional/element keys); P3 substitution edges II (context typing); P4 dropped store targets; P5 element/store loans; P6 deref/take channels; P7 literal destinations/guards. Each phase's surface is coherent for its lens set; the order follows dependency direction (AR-4) | ✅ Resolved (delegated) |
| 5 | Behavior | Per-row handling mode | red → green, or a control where the probe shows the row green | **Red → green for 18 rows**; `DEF-361` and `DEF-374` are green controls (their probed shapes stay accepted/rejecting exactly as observed). Every red case must fail without the fix; every control must fail if a fix regresses it | ✅ Resolved (delegated) |

## Per-Row Behavior Rulings

**AR-6 — `DEF-367`: the runtime-indexed element field store emits.** The frozen place list (Ch 04
§4, `spec/04-expressions.md:87`: "The target must be a place: a local, a parameter, a struct
field, an array element, or a dereferenced raw pointer, otherwise E10221") includes both a struct
field and an array element; `a[i].y` is a struct field of an array element, so it **is** a place.
The row's "or reject `E10221` until supported" fallback is **non-conformant** and declined: it
would reject a spelling the frozen list accepts. Fix: lower the indexed target to the element
address and apply the member projection (the constant-index twin's shape), with the target
evaluated before the RHS (Ch 04 §3, `:53`) and the bounds guard kept (Ch 04 §9, "Indexing is
always bounds-checked"). The RHS side effect runs exactly once. The slice spelling is pinned
compile-only (AR-33). Expected: local array, borrow parameter, generic borrow, and constant-index
spellings all emit; the three runnable runtime-index shapes read back `7`, the side-effect count
is 4, run exit 0; ASan/LSan-clean.

**AR-7 — `DEF-377`: the conditional-base member store reports `E10221`.** The same frozen place
list is closed; a conditional selection is not a local, parameter, field, element, or deref, so
`(c ? a : b).f.g = x`, `(c ? a : b).f[i].x = x`, `(c ? a : b).b.ys[i] = x`, and
`(c ? a1 : a2)[i].x = x` are not places. The batch-L `DEF-322` ruling is the precedent: the scalar
member twin already reports `E10221`, and the read spelling stays accepted because reads need no
place. Fix: recurse the `Member` arm's base test (`sema/ops.rs:402` `is_place`) so a member/Index
chain rooted in a conditional is not a place. Expected: one `E10221` per store site, no TU; the
read and ordinary member/element store controls stay accepted.

**AR-8 — `DEF-360`: the dot-call written argument reports `E10200`.** Ch 04 §8
(`spec/04-expressions.md:189-192`): a dot-call candidate "must accept the receiver as its first
parameter and match the argument list under the usual implicit widenings (section 5, Ch 02
section 9); no other conversions apply." Ch 04 §14 (`:327`): "An implicit conversion the language
does not allow is E10200." The receiver fixes the first parameter (`T = Box`), so the written `7`
can only match `w: T = Box` by an illegal conversion. The direct inferred and explicit twins
already report one `E10200` at the argument; the dot-call route must run the same written-argument
conversion check after the receiver. Both spellings apply: a `take` receiver (`b.pick(7)`) and a
`borrow` receiver (`b.put(7)`). Expected: one `E10200` at the written argument (`m360_dotarg.clo`
11:25; `m360_borrowdot.clo`'s written `7` at 11:11), no TU; the matching dot calls stay accepted
and run.

**AR-9 — `DEF-363`: a declaration/import collision reports `E11103`.** Ch 10 §4 MOD-12
(`spec/10-modules.md:111`): "Binding one name twice in a module is an error; rename with `as` |
E11103". The frozen message is "duplicate import of '{name}'", so the diagnostic is reported at
the **import item's name span** (`ImportItem::name_span`, the existing convention for the
duplicate-import report; the message names the local binding), once per colliding import,
regardless of declaration order (the declaration stays a valid single declaration; no `E11101`).
The import pass (`load/mod.rs:386` `resolve_imports`) extends its prior-import comparison to the
module's own declarations (`declarations_of`, all declaration kinds). Controls: `import { b as g
}` beside an unrelated `let h` runs; two imports colliding keep their current report. Expected:
both orders and the direct spelling report one `E11103`; the renamed control runs 0.

**AR-10 — `DEF-364`: the opaque conditional is judged per instance.** (a) `id(c ? a : b)` with
`a: T`, `b: U` and `T = i32`, `U = str` is accepted today and emits a C conditional with
mismatched arms; the instance must judge the substituted arms and report one `E10200` at the
else arm (the arm that cannot convert to the selected type, the concrete twin's site). Ch 04 §5
(`:108`): `?:` requires "same-typed branches" with the implicit widenings; Ch 04 §14 (`:327`)
owns the code. (b) `id(c ? x : null)` with `x: i32` is accepted today and emits a pointer/integer
conditional; the null arm at the non-nullable substituted context reports one `E10012` at the
`null` (Ch 02 §5 TY-7, `spec/02-types.md:127`: "`null` used where a non-nullable type is required
is an error"). The judgment is per instantiation (Ch 06 G-6, `:272`; Ch 17 §7, `:242`), once per
site; the un-instantiated generic body keeps its current acceptance. Controls: identical arms and
a non-null sibling arm run 0. Fix site: the conditional check (`sema/ops.rs:318`) plus a
per-instance judgment for a conditional whose arms still mention type parameters
(`own/instance_check.rs`).

**AR-11 — `DEF-365`: the nested element placeholder resolves from the substituted element.** The
instance bindings (`types/mod.rs:712` `instance_bindings`) mint one placeholder per nested loop
level but only push the next level when the element is an `Array`. With `T = str[2]`, the first
element is `str` and the nested `#element of #element of T` is never bound, so the emitted symbol
is `cl_id__#element of #element of T` (observed). Ch 06 G-6 and Ch 17 §7 (`:242`, `:253`) require
one keyed copy per concrete instantiation; Ch 08 §3 (a `str` index yields its byte) makes the
nested element `u8`. Fix: extend the level chain whenever the element has an index surface
(`Array`, `Str`, `StaticStr`, `CString`, `Raw`), so the nested placeholder binds `index_element(str)
= u8`. Expected: `cl_id__u8`; build-clean, run 0; the host and both m68k profiles emit no `#`.

**AR-12 — `DEF-368`: the substituted element member at a view destination widens per instance.**
`let s: str = xs[0].name;` at `T = S` (a `String` member) is accepted and emits `s =
xs[0].name` (`ClString` to `ClStr`), while the concrete twin emits `s = cl_string_view(xs[0].name)`.
Ch 16 §2 (`spec/16-borrow-rules.md:51`) makes the implicit `String`→`str` widening a loan; Ch 06
G-6/Ch 17 §7 make the instance behave as its concrete twin. The accepted direction is a
conversion, not a rejection (the concrete twin accepts). Fix: the substituted member read at a
view destination emits the concrete twin's `cl_string_view` widening (per-instance substitution
of the destination and source types). Expected: build-clean, run 0; byte-identical to the concrete
twin; the host and hosted-amiga legs agree.

**AR-13 — `DEF-371`: a literal at a generic callee's concrete parameter takes that type.** Ch 01
LEX-14 (`spec/01-lexical.md:181`: "A literal is typed by its context; a decimal literal defaults
to `i32`") and Ch 04 §13 (`spec/04-expressions.md:291`: "A literal is typed by its context"); Ch
04 §5's literal rule ("A literal operand takes the other operand's type when its value fits,
otherwise E10202") is the same contract. `set<S[1]>(borrow a, 0)` has the concrete parameter
`n: usize`; the literal must be typed `usize`, not `i32`, so no `E10200`. Fix: propagate the
concrete resolved parameter type into a literal argument at a generic callee (a parameter that
does not mention a type parameter). Expected: accepted, build-clean, run 0; the concrete twin is
the control and stays green.

**AR-14 — `DEF-384`: the invalid nullable scalar type argument reports `E10011`; the nullable
handle stays accepted.** Ch 02 §5 TY-4 (`spec/02-types.md:124`): "`T?` is allowed only on handles,
`raw`, `cstring`, and views | E10011". `Array<i32?>` is an illegal type argument and reports one
`E10011` at the `i32?` span; the existing helper (`own/program.rs:279` `report_illegal_nullable`)
extends from declared fields to every resolved type node, including type arguments. The probe
shows `Array<String?>` already keys (`ClArray__StringQ`), builds, and runs 0, so the row's "key
the nullable spelling in the ABI" half **needs no change**; it is the control, and the ABI-key
residual of the note is not reproducible as a defect. Expected: `Array<i32?>` one `E10011` at
the `i32?` type argument (line 6; the test captures the exact span); `Array<String?>` runs 0. No
new ABI scheme.

**AR-15 — `DEF-385`: a qualified type path resolves to the unqualified instance.** Ch 10 §5
MOD-13 (`spec/10-modules.md:112`): "Qualified access `A.B.f()` is always available, with or
without an import"; MOD-17 makes module-level declarations and imports resolve globally. The
qualified type path is the type-position spelling of the same rule. The resolver
(`types/table.rs:659` `resolve_named`) resolves only single-segment names, so
`std.mem.Array<i32>` becomes unknown and emits `i32`; the fix resolves a multi-segment named type
through its declaration's own name (the last segment) in the same table, yielding the same
`Array<i32>` instance. The type table is keyed by simple name; a cross-module last-segment
collision is a pre-existing table limitation and is out of this row's scope (it would need a
table-identity change, not this fix). Expected: build-clean, run 0; the emitted symbol is the
unqualified `ClArray__i32`; the unqualified control stays byte-identical.

**AR-16 — `DEF-386`: a negated literal takes the concrete context type.** Ch 01 LEX-14 and Ch 04
§13 (`:291`) say a literal is typed by its context; Ch 04 §5 (`:110`) says unary `-` yields the
operand's "same type" with wrapping for the minimum value. The spec's own example is
`let delta: i16 = -5;` (`spec/02-types.md:49`). Fix: the unary check (`sema/ops.rs:18`
`check_unary`) propagates the expected context type into the operand, so the literal `5` is typed
`i16`/`i8` and negated. Expected: `let delta: i16 = -5; let a: i8 = -5;` build and run 0; the
positive-literal control stays green. A literal that does not fit the context is `E10202` (Ch 04
§13): `let a: i8 = -1000;` reports one `E10202` at the literal. The `u8` spelling is accepted with
wrapping (`-5` = 251; an implementation pin, not a rejection). The plan does not introduce a new
fit rule.

**AR-17 — `DEF-372`: the element-read store carries the element's loan.** Ch 16 §2
(`spec/16-borrow-rules.md:51-58`): "Turning a `String` into a `str` implicitly — for example ...
binding a local `str` view — creates a shared loan of that `String`. The loan ends at the last use
of the derived view or any alias of it." The element store `xs[0] = a.v` records the loan (the
direct element argument reports `E10030`); `let s: str = xs[0]` must carry that loan to `s` and
its aliases, so `fill(s, borrow slot)` reports one `E10030` at the fill (Ch 16 §6, `:233`: a use
reachable only through the owner's release is rejected `E10030`). Fix: the view-like store/let
transfer adopts the source element's loan instead of dropping it. Expected: one `E10030` at the
`fill` site, matching the direct and member twins; no TU.

**AR-18 — `DEF-373`: conditional and array-literal view copies transfer the source's element
loans.** Same frozen rule as AR-17, plus Ch 03 §3 (a view binding copies the view; the storage it
names is shared) and Ch 16 BR-4 (a move of the owner requires no live loan). The legal copy
(`let ys: str[2] = (c ? xs : zs);`, `[xs[0], xs[1]]`, `c ? xs[0] : zs[0]`) must record each
element/arm loan so the later out-slot store reports `E10030` at the fill, exactly as the direct
conditional twin does. Fix: extend the DEF-315 loan-transfer machinery to the conditional,
array-literal, and single-view copy sources. Not a rejection of the copy (the copy itself is
legal; the escape is the report). Expected: one `E10030` at each fill; no TU.

**AR-19 — `DEF-374`: the borrow-element conditional take stays a control.** At the head the
generic and the concrete twins both report two `E10026` (one per arm) at the same span
(`m374_condelem.clo` 12:24 = `m374_concrete.clo` 12:24). Ch 07 STR-11/12
(`spec/07-structs.md:106-107`) limits partial moves to top-level fields and reports an element
move as `E10026`; the two reports are the two arm move-outs. The phase pins the count parity and
makes no code change. A regression turns the row red in the phase and re-enters this register.

**AR-20 — `DEF-375`: the raw-deref read materializes its pointer expression once.** Ch 04 §3
(`:53`): "Evaluation is strictly left-to-right and deterministic." Ch 04 §7 makes `*p` a place
read; Ch 17 §5 (`:182-192`) makes a move a copy plus a zero of the source, with the zero as the
drop flag, so one release can never run twice. The emitted C currently calls `cl_alt(p,q)` twice
(`build/batchM-plan/out/m375_derefimpure.c:702-703`), so the second call selects the other
pointee, which is emptied while the first pointee's value is bound and later released twice. Fix:
materialize the deref operand whenever the lowered value is a `Unary::Deref` read that will be
moved/emptied, under the substituted instance type — not the declaration-time `owns_drop(ty)`
gate (`own/statements.rs:479`). Expected: `alt` called once, `r` owns the read pointee, run exit
12 (`len("aa") + 1 * 10`), ASan/LSan-clean.

**AR-21 — `DEF-376`: the guarded take carries the consumed non-place source.** Ch 04 §3 (`:53`
and the `&&`/`||` row): "Left operand first; the right operand only when the result is not yet
fixed"; Ch 04 §5: "only the chosen branch evaluates." Ch 03 §3/O6 makes a value moved on any path
count as moved at a join; the zeroed source is the drop flag (Ch 17 §5). The deferred move-out
channel currently carries places only, so `g && (eat(take (unsafe *q)) == 1)` keeps the inline
call and the pointee is not emptied on the taken path. Fix: carry the consumed deref source's
emptying statement in the guarded operand so the evaluated path empties the pointee. Expected:
accepted; `g=true` run 0, `g=false` run 1; ASan/LSan-clean.

**AR-22 — `DEF-378`: every literal position materializes the hidden view-array destination.**
Ch 04 §3 (`:53`) fixes the function-call and array-literal evaluation order; Ch 04 §15 makes the
literal elements typed by the expected type; Ch 17 §5 fixes the destination move shape. A call
returning a fixed view array (`str[2]`) writes through a hidden destination parameter, so the
literal builder must bind that destination at **every** position, not only the guarded one. Fix:
a call-valued view-array field (a call whose return needs the hidden destination) routes to the
element-wise store/binding form at the read, unguarded, nested, and guarded positions
(`own/lower.rs:4536` `literal_fields_need_stores`, `:4592` `view_element_needs_binding`, and the
literal builder). Expected: `readpos`, `gridel`, and `guardrowcond` build strictly and run 0.

**AR-23 — `DEF-379`: the owning nested conditional's arm store stays in its guard path.** Ch 04
§3 (`:53`, `&&`/`||` row): the right operand evaluates only when the left does not fix the result;
Ch 04 §5 (`:108`): "only the chosen branch evaluates." The emitted C currently fills the arm
carrier (`__cl_owned0`) before `if (g && consume(...))`, so a call runs on the skipped `g == false`
path (observed calls = 3 where 2 are expected). Fix: the guarded literal's fill is emitted at the
guard's evaluated position (inside the `&&` right operand), not at statement entry. Expected:
calls = 2, run 0; ASan-clean.

**AR-24 — `DEF-390`: a qualified call carries its module identity.** Ch 10 §5 MOD-13
(`:112`): "Qualified access `A.B.f()` is always available, with or without an import"; Ch 04 §8
(`:178`): `A.B.f` is "module-qualified name or function". The checker already types the call with
the module's signature (the mismatch control reports `E10200`); the emitter's `callee_name`
reduces the qualified path to its last segment and `resolve_function` (`codegen.rs:701`) resolves
own-first, so the caller's same-named function is emitted. Fix: thread the resolved
`(module, name)` through the lowered callee so the emitter calls the qualified module's function;
the module path wins over a same-named current-module function, and a lexical local/parameter
still shadows the head (batch-L AR-25). Expected: `probe.m390.lib.f()` runs the module's `f` and
returns 1 (run 0); the signature shape builds and runs 0; the mismatch keeps its `E10200`.

**AR-25 — Interaction with batch L's standing rulings.** (a) Batch-L AR-25 (module path wins for
reads and calls when no lexical local shadows the head) governs `DEF-390` and `DEF-385`. (b)
Batch-L AR-24 (host/m68k ABI layout; no new layout numbers) governs the cross-profile emission
checks for `DEF-365`, `368`, `378`, `384`, `385`. (c) Batch-L AR-22 (a `raw E` member read is a
value form) does **not** apply to `DEF-375`/`376`: a raw pointee deref `*p` is a place (EX-14
lists a dereferenced raw pointer), not a value-form enum member read. (d) Batch-L AR-21 (the
`m09` oracle re-pin) is untouched: no oracle contradiction arises here. (e) Batch-L AR-9 (origin
suites, additions only) is reused as AR-26.

## Process and Verification Rulings

**AR-26 — Test placement.** Additions only to the origin suites:
- `checker_conformance_{spec,impl}_test.rs`: P1 (`DEF-360`, `361` control, `DEF-363`), all of P2
  (`DEF-364`, `365`, `368`), and all of P3 (`DEF-371`, `384`, `385`, `386`).
- `callable_release_integrity_{spec,impl}_test.rs`: P1 `DEF-390` (the callee-identity origin
  suite).
- `conditional_projection_residue_{spec,impl}_test.rs`: all of P4 (`DEF-367`, `377`), P5
  (`DEF-372`, `373`, `374` control), P6 (`DEF-375`, `376`), and P7 (`DEF-378`, `379`).
- `run_fault_spec_test.rs`: the `DEF-367` bounds-fault case (a runtime index outside the element
  count traps `E11211` after the new store route).
- `module_initializer_integrity_{spec,impl}_test.rs`, the frozen spec suites (`*_spec_test.rs`
  under `compiler/tests/`), and the corpus suites are regression oracles only.
- No new suite pair is created; if a case has no natural home, the phase creates
  `residual_major_completion_{spec,impl}_test.rs` with only that case and records it here. The
  `AGENTS.md` test map gains the pair only at close-out if it is created.

**AR-27 — Lenses.** P1: correctness + semantics + emission-integrity (call identity, module
binding, and written-argument typing; no ownership change). P2: all four (the conditional values
and element substitutions touch ownership and emission). P3: correctness + semantics +
emission-integrity (type and conversion rules; no ownership change; the nullable-handle control
runs ASan in the verification). P4: all four (the critical store, the place rule, the side
effects, and emission). P5: correctness + memory-safety + semantics (loan judgments and escaped
views). P6: all four (deref materialization, move-out channels, emitted C). P7: all four (literal
destinations, guard evaluation, stored values, and leaks). The read-only auditors stay read-only.

**AR-28 — Verification command set.**
- Build: `make -C docker exec CMD="cargo build --workspace"`.
- Compile: `make -C docker exec CMD="target/debug/clc --profile host -o out.c prog.clo"`; strict C:
  `cc -std=c99 -O0 -g -Wall -Wextra -Werror` (the pinned container).
- ASan: `cc -std=c99 -O0 -g -fsanitize=address -no-pie`; run
  `ASAN_OPTIONS=detect_leaks=1:symbolize=0 timeout 30`.
- Suites: `make check` per phase; focused suites via
  `make -C docker exec CMD="cargo test --test <suite>"`; the suites' `RDC_ASAN=1` switch for the
  memory runs.
- Profiles: `clc --profile hosted-amiga` plus the pinned `m68k-amigaos-gcc` leg for `DEF-365`,
  `368`, `378`, `384`, `385`; `clc --profile freestanding` for `DEF-365` only (the other rows
  import `std.text`/`std.mem`, which the freestanding profile rejects with `E11200` — observed,
  not a defect). `make verify` at close-out.

**AR-29 — No spec, registry, or golden edit; park protocol.** Every fix reuses a frozen code:
`E10011`, `E10012`, `E10030`, `E10026` (control only), `E10200`, `E10202`, `E10221`, `E11103`,
and the runtime trap `E11211`. No new diagnostic, registry row, chapter, or golden. A row whose
fix turns out to need one is parked mid-phase with its probe evidence and recorded in the findings
ledger; the phase continues with its remaining rows.

**AR-30 — Complexity and runtime deviation.** Every fix rides an existing hook (the dot-call check,
the import pass, the callee identity, the per-instance duty/audit pattern, the type resolver, the
store dispatch, the loan transfer, the deref materialization, the guard lowering). A phase's fix
surface may touch the module that owns the judgment beyond the task's file list; the deviation is
recorded in this register and covered by the phase's lenses (the batch-I/J/K/L precedent).

**AR-31 — `DEF-361` control evidence.** No red reproduction could be constructed at the head: the
direct analog matrix (local shadow of a direct import, parameter shadow, expression-bodied lambda
parameter, block-bodied lambda parameter, module-own binding, generic-body local) all compile
strictly and run 0 (`build/batchM-plan/matrix/d_*.clo`, `m361_local.clo`, `m361_param.clo`). The
class is structurally closed by the batch-L phase-2 C2/R1 corrections: function bodies and both
lambda body forms walk with an empty scope map, so a body/parameter name can no longer resolve to
an import (`own/generics.rs:34-52`, `:400-410`). The phase pins the direct-name shapes; it plans
no code change and records the evidence. Frozen rule: Ch 10 §5 MOD-15/16 (`spec/10-modules.md:169-170`).

**AR-32 — `DEF-374` control evidence.** The generic and concrete twins both report two `E10026`
at the same span at the head (`m374_condelem.clo` = `m374_concrete.clo`, 12:24). The phase pins
the count and span parity; no code change. Frozen rule: Ch 07 STR-11/12 (`spec/07-structs.md:106-107`).

**AR-33 — `DEF-367` slice spelling.** A slice value cannot be produced from a fixed array inside
the probe: Ch 08 §1 AR-2 (`spec/08-arrays-strings.md:23`: "A fixed array does not decay to a
pointer") and O9 ("There are no local view expressions") forbid the local slice binding, and
`rawSlice` is not exported by `std.mem`. The slice spelling is therefore pinned compile-only: an
uncalled `func viaslice(borrow ps: Elem[], i: i32)` must emit the element store and its bounds
guard in the translation unit; the runnable spellings are the local array, the borrow parameter,
the generic borrow, and the constant index. The implementation pin asserts the emitted store; the
runtime case covers the three callable forms.

**AR-34 — Stop conditions.** (a) `DEF-367`: if the indexed-element member store cannot be emitted
on a profile without a new C declarator form, the phase parks the row with the emitted-C evidence
(the frozen place rule forbids the rejection fallback). (b) `DEF-384`: if extending the nullable
check to type arguments rejects an accepted legal spelling (`Array<String?>` and the nullable
view/handle controls), the phase records the false positive and narrows the check to the invalid
scalar case; the control set is the proof. (c) `DEF-385`: a cross-module last-segment type-name
collision is out of scope; the phase records it as a residual if a probe shows it. (d) `DEF-390`:
if a lexical local shadows the module head, the value-headed reading wins (batch-L AR-25) and the
case is not a module call; the phase pins that shadow control.

## Open Questions (collected for the user — none blocks execution)

- The two parked majors' rulings (`DEF-167`, `DEF-171`) are still due; they stay out of this
  batch (AR-1).
- `DEF-386`'s `u8` wrapping spelling (`-5` = 251) is accepted per Ch 04 §5's wrapping rule; it is
  recorded as an implementation pin, not a spec acceptance. If the semantics review reads the
  fit rule as applying to the negated value, the phase records the ruling and re-pins; this is a
  reading of frozen text, not a new surface.
- `DEF-384`'s ABI-key half is not reproducible as a defect (the nullable handle already keys and
  runs); the row is closed on the rejection half with the ABI control.

## Verified Current Status (AR-5 probes, 2026-10-17, head `86d6f51`)

Probe method: `make -C docker exec CMD="cargo build --workspace"`, then
`target/debug/clc --profile host -o out.c prog.clo` against the reconstructed minimal programs
(`build/batchM-plan/probes/`, `matrix/`, `extra/`); the emitted C compiled with
`cc -std=c99 -O0 -g -Wall -Wextra -Werror` and run with `timeout 10`; the ownership rows also
built with `-fsanitize=address -no-pie` and run with
`ASAN_OPTIONS=detect_leaks=1:symbolize=0 timeout 30`. Multi-file probes pass every `.clo` file on
the command line (the file list is the program). The probes are gitignored scratch.

| Row | Probe | Verified current behavior |
|-----|-------|---------------------------|
| `DEF-360` | `m360_dotarg.clo`, `m360_borrowdot.clo` | ❌ Red: accepted; strict build fails (`incompatible type for argument 2 of 'cl_pick__Box'` at 11:25; `cl_put__Box` at 11:20). The direct inferred and explicit twins report one `E10200` at the argument; `m360_control.clo` runs 0 |
| `DEF-361` | `m361_local.clo`, `m361_param.clo`, `matrix/d_*.clo` | ✅ **Control**: the local, parameter, lambda-expression, lambda-block, module-own, and generic-body direct-name shadow shapes all compile strictly and run 0; no red reproduction at the head (AR-31) |
| `DEF-363` | `m363_importfirst.clo`, `m363_declfirst.clo`, `m363_direct.clo` | ❌ Red: all three accepted; compile strictly and run 0 (the collision is unreported); `m363_control.clo` also runs 0 |
| `DEF-390` | `m390_main.clo` + `m390_lib.clo`; `m390_sigmain.clo` + `m390_siglib.clo` | ❌ Red: `probe.m390.lib.f()` is accepted and runs the caller's `f` (run 1, expected 0); the signature shape is accepted and the strict build fails (`incompatible type for argument 1 of 'cl_probe_m390_sigmain_f'`); the mismatch control reports one `E10200` at the argument; `m390_control.clo` runs 0 |
| `DEF-364` | `m364_condopaque.clo`, `m364_nullarm.clo` | ❌ Red: (a) accepted, strict build fails (`type mismatch in conditional expression`); (b) accepted, strict build fails (`pointer/integer type mismatch` and `void *` to `i32`). Controls `m364_control.clo` run 0 |
| `DEF-365` | `m365_elemkey.clo` | ❌ Red: accepted; strict build fails (`stray '#' in program`, `cl_id__` undeclared); the emitted key is `cl_id__#element of #element of T` on host, hosted-amiga, and freestanding |
| `DEF-368` | `m368_elemview.clo` (generic), `m368_concrete.clo` | ❌ Red: accepted; strict build fails (`incompatible types ... assigning to 'ClStr' from 'ClString'` at 8:9); the concrete twin runs 0; hosted-amiga emits the same assignment |
| `DEF-371` | `m371_literal.clo`, `m371_control.clo` | ❌ Red: one `E10200` (`cannot convert 'i32' to 'usize'`) at 15:25; the concrete twin compiles and runs 0 |
| `DEF-384` | `m384_nullabletypearg.clo`, `m384_nullablehandle.clo` | ❌ Red: accepted; strict build fails (`passing argument 2 of 'cl_push__i32Q' makes integer from pointer` at 8:25); hosted-amiga emits `ClArray__i32Q` with an `i32` parameter and a `void *` argument. `Array<String?>` keys and runs 0 |
| `DEF-385` | `m385_qualifiedtype.clo` | ❌ Red: accepted; strict build fails (`incompatible types ... to 'i32' from 'ClArray__i32'` at 4:9; `cl_push__i32` argument type); hosted-amiga emits `i32 g`. The unqualified control runs 0 |
| `DEF-386` | `m386_negated.clo`, `m386_control.clo` | ❌ Red: one `E10200` (`i32`→`i16`) at 4:22 and one (`i32`→`i8`) at 5:17; the positive control runs 0 |
| `DEF-367` | `m367_indexed.clo` | ❌ Red (critical): accepted; strict build ok; run 191 — the local, borrow-parameter, and generic-borrow stores and their `bump()` side effects are dropped (calls = 1, expected 4); the constant index emits. ASan clean (no read) |
| `DEF-377` | `m377_member.clo`, `m377_index.clo`, `m377_array.clo`, `m377_indexbase.clo`, `m377_deep.clo` | ❌ Red: all accepted; strict build ok; runs 249/255/249/249/255 — the stores and their RHS side effects are dropped; ASan/LSan-clean (no memory error; the store is simply absent); the read control runs 0 |
| `DEF-372` | `m372_elementread.clo`; `m372_direct.clo`, `m372_member.clo` | ❌ Red: accepted; strict build ok; run varies (14 observed) + ASan `heap-use-after-free`. The direct element argument and the member twin report one `E10030` |
| `DEF-373` | `m373_conditional.clo`, `m373_arrayliteral.clo`, `m373_singleview.clo`; `m373_direct.clo` | ❌ Red: all three accepted; strict build ok; runs vary (56/193/145) + ASan `heap-use-after-free`. The direct conditional twin reports one `E10030` |
| `DEF-374` | `m374_condelem.clo`, `m374_concrete.clo` | ✅ **Control**: both reject with two `E10026` at 12:24 (the per-arm parity, AR-32) |
| `DEF-375` | `m375_derefimpure.clo` | ❌ Red: accepted; strict build ok; abort 134 + ASan `attempting double-free`; the emitted C calls `cl_alt__String` twice (`m375_derefimpure.c:702-703`), so the second call empties the other pointee |
| `DEF-376` | `m376_guardedtake.clo` | ❌ Red: accepted; strict build ok; abort 134 + ASan `attempting double-free` (the guarded take keeps the inline call) |
| `DEF-378` | `m378_readpos.clo`, `m378_gridel.clo`, `m378_guardrowcond.clo` | ❌ Red: all accepted; strict build fails (`too few arguments ... expected 1, have 0` for `cl_probe_m378_*_n1/r1`) |
| `DEF-379` | `m379_guardnest.clo` | ❌ Red: accepted; strict build ok; run 1 (expected 0) — the arm fill runs before `if (g && consume(...))`, so calls = 3 where 2 are expected |

**The two reclassifications**: `DEF-361` and `DEF-374` are verified green at the head and are
carried as pinned controls in their phases; neither has a residual in another row's scope. The
other 18 rows are red and planned.
