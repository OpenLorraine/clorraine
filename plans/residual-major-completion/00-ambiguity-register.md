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
| 1 | Scope | Which rows batch M closes | A) the 20 remaining un-parked rows (1 critical + 19 majors), severity-first; B) the two parked majors; C) the minors | **A** — pre-decided by the dispatch and recorded, not re-litigated. The scope is **severity-first** (the critical is planned first); execution is dependency-ordered (AR-4/AR-36). B stays parked (`DEF-167`, `DEF-171`); C is the next batch. The dispatch's "79 minors" is the remaining backlog count after this batch: 2 parked majors + 77 minor rows (observed: 99 open rows = 1 critical + 21 majors + 77 minors; 20 close here, 2 majors stay parked) | ✅ Resolved (pre-decided, count corrected) |
| 2 | Process | The specification-first and zero-ambiguity discipline | — | **Spec cases first, then red, then implement, then green, then impl pins.** Every row's expected outcome is fixed below before any code: a frozen diagnostic (code + site), a strict-build-clean run with its exit value, or an ASan-clean run. No implementer question may remain; a discovered gap re-enters this register before code | ✅ Resolved (pre-decided) |
| 3 | Verification | The current status of the 20 rows at the head | probe the actual compiler (`86d6f51`) against reconstructed minimal reproductions | **The AR-5 table** — 18 red; `DEF-361` and `DEF-374` green at the head and carried as pinned controls. No row is parked: every fix reuses a frozen code | ✅ Resolved |
| 4 | Design | Phase grouping and sequencing | A) seven phases by machinery, executed 1 → 7; B) one row per phase; C) the origin batches' phases | **A** — P1 call identity/written arguments; P2 substitution edges I (conditional/element keys); P3 substitution edges II (context typing); P4 dropped store targets; P5 element/store loans; P6 deref/take channels; P7 literal destinations/guards. Each phase's surface is coherent for its lens set; the scope is severity-first, the **execution is dependency-ordered** (P1 → P2, P1 → P3, P2 → P3 and P2 → P5, P6 → P7; P4 is independent and runs fourth — AR-36) | ✅ Resolved (delegated) |
| 5 | Behavior | Per-row handling mode | red → green, or a control where the probe shows the row green | **Red → green for 18 rows**; `DEF-361` and `DEF-374` are green controls (their probed shapes stay accepted/rejecting exactly as observed). Every red case must fail without the fix; every control must fail if a fix regresses it | ✅ Resolved (delegated) |

## Per-Row Behavior Rulings

**AR-6 — `DEF-367`: the runtime-indexed element/member store emits in every target form.** The
frozen place list (Ch 04 §4, `spec/04-expressions.md:87`: "The target must be a place: a local, a
parameter, a struct field, an array element, or a dereferenced raw pointer, otherwise E10221";
the unary `*` row at `:112` makes a dereferenced raw pointer a place) includes both a struct field
and an array element; `a[i].y` is a struct field of an array element, so it **is** a place. The
row's "or reject `E10221` until supported" fallback is **non-conformant** and declined: it would
reject a spelling the frozen list accepts. Fix: (a) resolve the element/member type per instance —
the pointer's `ty` is unknown at the declaration (`own/expr.rs:2367` `element_ty`), so
`StorePointer`'s owning/fixed-array branch never runs. The generic-borrow member spelling
(`w.items[i].name = ...`) is currently dropped (run 255, no leak), and the whole-element spelling
(`w.items[i] = Elem { ... }`) emits but leaks the overwritten owner (LSan `m367_gen_elemstore`);
both are covered by resolving the substituted place type. The store's pointer is a `Value`
(`AddressOf`), so `resolved_place_ty` (which takes a `Place`) is **not** its path: extend
`resolved_value_ty` (`codegen/expr.rs:366`) to the `AddressOf` operand, or record a target
`Place`/type duty at lowering. (b) Generalize the target builder so the plain store, compound
assignment (`+=`), update (`++`/`--`), deep member chains, **nested generic members**
(`w.mids[i].inner.name`), and **multi-index** targets (`w.grid[i][j].name`) all emit —
`indexed_target` (`own/expr.rs:2354`, the Index-only gate at `:2364-2366`) is shared by
`emit_indexed_store` (`:2417`) and `emit_indexed_update` (`:2491`); the no-silent-drop rule
applies to `emit_indexed_update` too. (c) Always materialize the runtime base/index for the new
route so the target location is evaluated before the RHS (Ch 04 §3, `:53`) — the `contains_update`
pin is insufficient. (d) The `None` fallback must not silently drop and must **not** add an
`E10221` false-rejection path for accepted targets: make it a debug assertion or pin its
unreachability over the supported place roots. The bounds guard stays (Ch 04 §9,
`:98-111`; IX-1 `E11211`). The slice spelling is pinned compile-only (AR-33); the bounds-fault
case lives in the residue suite with the inline `assert_traps` helper (precedent
`borrow_store_integrity_spec_test.rs:363`), not in `run_fault_spec_test`.
Expected: local array, borrow parameter, generic borrow, constant index, compound, update, deep
chain, nested generic member, multi-index, and target-order shapes all emit; the values read back; the RHS count is exact (4 in the
basic probe); the generic owning store is LSan-clean; an out-of-range generic-borrow index traps
`E11211`; ASan/LSan-clean. There is no park fallback: if the route cannot be emitted on a profile,
the phase stops for a user ruling (AR-34(a)).

**AR-7 — `DEF-377`: the conditional-base member store reports `E10221`; the place test recurses.**
The frozen place list is closed; a conditional selection is not a local, parameter, field, element,
or deref, so the **five** forms `(c ? a : b).f.g = x`, `(c ? a : b).f[i].x = x`,
`(c ? a : b).b.ys[i] = x`, `(c ? a1 : a2)[i].x = x`, and the deep chain
`(c ? h1 : h2).b.ys[i] = x` are not places. The batch-L `DEF-322` ruling is the precedent: the
scalar member twin already reports `E10221`, and the read spelling stays accepted because reads
need no place. Fix: three changes to `is_place` (`sema/ops.rs:402`): (a) add
`ExprKind::Member { base, .. } => self.is_place(base)` — the current shallow `matches!` on the
base kind is what lets all five red shapes through; (b) add
`ExprKind::Unsafe(inner) => self.is_place(inner)` — the wrapper is transparent to the inner place,
so `unsafe p.f`/`unsafe p.f.x` are accepted (their inner member roots at a local/raw pointer)
while `unsafe (c ? a : b).v` and `(unsafe (c ? a : b)).v.x` are **not** places and report one
`E10221`; (c) remove the Index arm's shallow `|| matches!(base.kind, ExprKind::Unsafe(_))` bypass
and keep its `self.is_place(base)` recursion. The five red forms and the unsafe-wrapped
conditional forms then each report one `E10221`. The accepted prefix raw forms must **read back
the stored value** (the read-back is the oracle that the store is emitted); the block forms
(`s.f.g`, `s.arr[i]`, `p.f.g` inside `unsafe { }`) stay green. The dropped store through `(*p).f` and `unsafe (*p).f` is filed as a residual row during
execution (the `Unsafe` transparency newly accepts the unsafe spelling, which still drops the
store; probe both when filed).
Expected: one `E10221` per conditional-rooted store (the five forms and the unsafe-wrapped
conditional forms), no TU; the `unsafe p.f`/`p.f.x` prefix forms are accepted and read back; the
block controls are accepted and run 0.

**AR-8 — `DEF-360`: the dot-call written argument reports `E10200`, inferred at the call site.**
Ch 04 §8 (`spec/04-expressions.md:189-192`): a dot-call candidate "must accept the receiver as its
first parameter and match the argument list under the usual implicit widenings (section 5, Ch 02
section 9); no other conversions apply." Ch 04 §14 (`:327`): "An implicit conversion the language
does not allow is E10200." The receiver fixes the first parameter (`T = Box`), so the written `7`
can only match `w: T = Box` by an illegal conversion. The direct inferred and explicit twins each
report one `E10200` (they do **not** run); the matching dot calls run 0. The dot-call route must
run the same written-argument conversion check after the receiver. It must **infer the callee's
type arguments at the call site** from the receiver and written arguments, not defer a
parameter-typed record against the enclosing function: the deferred record
(`sema/expr.rs:491` `record_deferred_conversion`) stores `self.function_name`, so the valid
generic caller `outer<i32>(b, c)` with `b: Box`, `c: Box`, and `b.pick(c)` is falsely rejected
with **two** `E10200` (`m360_genericcaller_box.clo` 8:12 and 8:19); the earlier
`m360_genericcaller`/`2` probes pass `c: i32` and are invalid rejection cases, not controls.
Both spellings apply: a `take` receiver (`b.pick(7)`) and a `borrow` receiver (`b.put(7)`).
Expected: one `E10200` at the written argument (`m360_dotarg.clo` 11:25;
`m360_borrowdot.clo`'s written `7` at 11:11) and for each `c: i32` generic-caller shape, no TU;
the matching dot calls and the `c: Box` generic caller run 0.

**AR-9 — `DEF-363`: a declaration/import collision reports `E11103` for every declaration kind.**
Ch 10 §4 MOD-12 (`spec/10-modules.md:111`): "Binding one name twice in a module is an error;
rename with `as` | E11103". The frozen message is "duplicate import of '{name}'", so the
diagnostic is reported at the **import item's name span** (`ImportItem::name_span`, the existing
convention for the duplicate-import report; the message names the local binding), once per
colliding import, regardless of declaration order (the declaration stays a valid single
declaration; no `E11101`). The import pass (`load/mod.rs:386` `resolve_imports`) extends its
prior-import comparison to the module's own declarations (`declarations_of` at `:293`, which
covers `func`, `comptime`, `struct`, `enum`, `handle`, `ffi` block names and functions,
`intrinsic`, module `let`, and module `const`). The pin matrix covers a function, a struct, and a
`const` at minimum, both orders, one `E11103` each. Controls: `import { b as g }` beside an
unrelated `let h` runs; two imports colliding keep their current report. Expected: both orders and
the direct spelling report one `E11103`; the renamed control runs 0.

**AR-10 — `DEF-364`: the conditional is judged per instance, with the expected type in every
direct context.** (a) `id(c ? a : b)` with `a: T`, `b: U` and `T = i32`, `U = str` is accepted
today and emits a C conditional with mismatched arms; the instance must judge the substituted arms
and report one `E10200` at the else arm (the arm that cannot convert to the selected type, the
concrete twin's site). Ch 04 §5 (`:108`): `?:` requires "same-typed branches" with the implicit
widenings; Ch 04 §14 (`:327`) owns the code. (b) `id(c ? x : null)` with `x: i32` is accepted today
and emits a pointer/integer conditional; the null arm at the non-nullable substituted context
reports one `E10012` at the `null` (Ch 02 §5 TY-7, `spec/02-types.md:127`: "`null` used where a
non-nullable type is required is an error"). The judgment is per instantiation (Ch 06 G-6, `:272`;
Ch 17 §7, `:242`), once per site; the un-instantiated generic body keeps its current acceptance.
(c) The **direct** conditional must take the expected type into `check_conditional`
(`sema/ops.rs:318`) from `check_value`, and `check_conditional` passes it into **both** arm
checks (`check_value(then_value, Some(expected))` and `check_value(else_value, Some(expected))`);
today `sema/expr.rs:67-71` drops `expected`, so `let y: i32 = c ? x : null;` is accepted and the
C fails. The four context pins are the let
initializer, the return, the struct field, and the call argument; the concrete disagreeing call
and the null-arm case are also pinned. Controls: identical arms and a non-null sibling arm run 0.
Fix site: the conditional check (`sema/ops.rs:318`) plus a per-instance judgment for a
conditional whose arms still mention type parameters (`own/instance_check.rs`). The P2 phase owns
this duty; it does not share P1's or P3's hook (AR-36).

**AR-11 — `DEF-365`: the nested element placeholder resolves from the substituted element.** The
instance bindings (`types/mod.rs:712` `instance_bindings`) mint one placeholder per nested loop
level but only push the next level when the element is an `Array` (`:730-732`). With `T = str[2]`,
the first element is `str` and the nested `#element of #element of T` is never bound, so the
emitted symbol is `cl_id__#element of #element of T` (observed). Ch 06 G-6 and Ch 17 §7 (`:242`,
`:253`) require one keyed copy per concrete instantiation; Ch 08 §4 (`:100-111`, IX-1) makes a
`str` index yield its byte. Fix: extend the level chain whenever the element has an index surface
(`Array`, `Str`, `StaticStr`, `CString`, `Raw`), so the nested placeholder binds
`index_element(str) = u8`. Expected: `cl_id__u8`; build-clean, run 0; the host and both m68k
profiles emit no `#`.

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
04 §5's literal rule (`:120-121`: "A literal operand takes the other operand's type when its value
fits, otherwise E10202") is the same contract. `set<S[1]>(borrow a, 0)` has the concrete parameter
`n: usize`; the literal must be typed `usize`, not `i32`, so no `E10200`. Fix: propagate the
concrete resolved parameter type into a literal argument at a generic callee (a parameter that
does not mention a type parameter). This is the P3 **plain-call** route; it does not share the P1
dot-call hook or the P2 conditional duty (AR-36). Expected: accepted, build-clean, run 0; the
concrete twin is the control and stays green; the out-of-range input `1000000000000` reports
`E10202` at the literal.

**AR-14 — `DEF-384`: the invalid nullable scalar reports `E10011` at nested type-argument
positions.** Ch 02 §5 TY-4 (`spec/02-types.md:124`): "`T?` is allowed only on handles, `raw`,
`cstring`, and views | E10011". The new check is scoped to **nested type-argument positions** and
the **explicit call type argument**: (a) a nested annotation walk alongside the existing
declaration checks (`own/place.rs:1931` `check_nullable_decl`, `own/program.rs:279`
`report_illegal_nullable`, both unchanged) and (b) a call-site walk or position flag on an
explicit call type argument (`arrayNew<i32?>()`), so both the annotation and the call argument
report. The probes split into two single-occurrence programs: the field annotation
`struct H { a: Array<i32?>; }` (`m384_annot`) and the explicit call type argument
`arrayNew<i32?>()` (`m384_callarg`), each expected to report one `E10011`. Nested cases
`Array<Array<i32?>>` and `Array<Wrap<i32?>>` (`m384_nested`) each report one `E10011` at the inner
`i32?`; the duplicate-count case (`m384_count`: nullable parameter, nullable return,
`let x: i32?`, the `Array<i32?>` annotation, and the `arrayNew<i32?>()` call argument) reports
**five** sites, one each, no duplicates. `Array<String?>` already keys (`ClArray__StringQ`),
builds, and runs 0, so the row's "key the nullable spelling in the ABI" half **needs no change**;
it is the control, and the ABI-key residual of the note is not reproducible as a defect.
Expected: one `E10011` at each invalid nested/call site; the nullable-capable instances
(`Array<String?>`, the verified control) run 0.
If the extension rejects a legal nullable type, the phase narrows to the invalid non-capable kinds
(AR-34(b)).

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

**AR-16 — `DEF-386`: a negated integer literal is folded and fit-checked once.** Ch 01 LEX-14 and
Ch 04 §13 (`:291`) say a literal is typed by its context; Ch 04 §5 (`:120-121`) carries the fit
rule and the unary `-` row (`:110`) gives the same type. The spec's own example is
`let delta: i16 = -5;` (`spec/02-types.md:49`). Fix: pass the expected type to `check_unary`;
for a **negated integer literal**, fold it first with `int_literal_value` (which already handles the unary minus), apply exactly one
`int_fits` check on the **folded value** gated by `is_integer_scalar`, and **do not** context-type
the operand. A non-integer context (`f32`, `bool`) stays on the ordinary conversion path and
reports `E10200`. Expected: `i16 = -5`, `i8 = -5`, and `i8 = -128` are accepted; `u8 = -5` and
`i8 = -129` each report one `E10202` at the literal; `f32 = -5` and `bool = -5` still report
`E10200`; the positive-literal control stays green. The position matrix covers **eight**
positions: let initializer, module value, call argument, return, struct field, array element,
conditional arm, and assignment. There is no wrapping-value expectation.

**AR-17 — `DEF-372`: the element-read store carries the element's loan.** Ch 16 §2
(`spec/16-borrow-rules.md:51-58`): "Turning a `String` into a `str` implicitly — for example ...
binding a local `str` view — creates a shared loan of that `String`. The loan ends at the last use
of the derived view or any alias of it." The element store `xs[0] = a.v` records the loan (the
direct element argument reports `E10030`); `let s: str = xs[0]` must carry that loan to `s` and
its aliases, so `fill(s, borrow slot)` reports one `E10030` at the fill (Ch 16 §6, `:233`: a use
reachable only through the owner's release is rejected `E10030`; BR-1 at `:187`). Fix: the
view-like store/let transfer adopts the source element's loan instead of dropping it (the `let`
transfer is `own/statements.rs:391-399`). Expected: one `E10030` at the `fill` site, matching the
direct and member twins; no TU.

**AR-18 — `DEF-373`: conditional and array-literal view copies transfer the source's element
loans as a union.** Same frozen rule as AR-17, plus Ch 03 §2 (`spec/03-ownership.md:70-71`: a view
lend copies the view, and the storage it points into is shared) and Ch 16 BR-4
(`spec/16-borrow-rules.md:190`: a move of the owner requires no live loan). The legal copy
(`let ys: str[2] = (c ? xs : zs);`, `[xs[0], xs[1]]`, `c ? xs[0] : zs[0]`, and a conditional
element inside an array literal `[c ? xs[0] : zs[0], xs[1]]`) must record each element/arm loan so
the later out-slot store reports `E10030` at the fill, exactly as the direct conditional twin
does. The destination record is a **union**: `move_element_loans` (`own/place.rs:1184`) snapshots
the source and replaces the destination record at `:1202` through `replace_element_loans`
(`:1103`), so a per-arm/per-element call would lose the first arm's positions. Fix: snapshot both
arms and the array-literal elements and write the destination's loan **union** once; recurse into
a conditional element inside an array literal, unioning both arms; extend the DEF-315 machinery to
the conditional, array-literal (**including the fill form**), single-view, and
conditional-with-an-array-literal-arm sources. Not a rejection of the copy (the copy itself is
legal; the escape is the report). Expected: the mixed-position case (arm A loans `ys[0]`, arm B
loans `ys[1]`) reports at both fill sites; the different-arm-count case (`b373_diff`) and the
fill-form case (`b373_fill`) report at every affected fill; one `E10030` per fill; no TU.

**AR-19 — `DEF-374`: the borrow-element conditional take stays a control.** At the head the
generic and the concrete twins both report two `E10026` (one per arm) at the same span
(`m374_condelem.clo` 12:24 = `m374_concrete.clo` 12:24). Ch 07 STR-11/12
(`spec/07-structs.md:106-107`) limits partial moves to top-level fields and reports an element
move as `E10026`; the two reports are the two arm move-outs. The phase pins the count parity and
makes no code change. A regression turns the row red in the phase and re-enters this register.

**AR-20 — `DEF-375`: the owning raw-deref source is materialized and moved in every consuming
position.** Ch 04 §3 (`:53`): "Evaluation is strictly left-to-right and deterministic." Ch 04 §7
makes `*p` a place read; Ch 17 §5 (`:182-192`) makes a move a copy plus a zero of the source, with
the zero as the drop flag, so one release can never run twice. The store-only gate `owns_drop(ty)`
(`own/statements.rs:490`, in `prepare_store`) does not cover `return *alt(p,q)` (`m375_ret`), the
struct/aggregate field `Box { s: *alt(p,q) }` (`m375_field` and `b375_field`), an array-literal
element `[*alt(p,q)]` (`b375_arr`), or a conditional arm `flag ? *alt(p,q) : x` (`b375_cond`);
all abort with ASan double-free. The take-argument path (`own/lower.rs:4866-4888`) already
materializes under `owns_drop || maybe_owns || unknown`. Fix: one shared "materialize an owning
deref source" helper with that take-path predicate, used by the **store**, the **return**, the
**aggregate/struct-literal field**, the **array-literal element**, and the **conditional arm**
paths. A field/aggregate source must be **moved** — the pointee is zeroed — not merely pinned: the
current field C either calls `alt` twice (impure pointer) or once without emptying (place
pointer), so the caller and the field can both own the pointee. Under a guard the fill/materialization is deferred to the evaluated path. Maybe-owning
values materialize; a concrete duplicable pointee is the control (`m375_dup`, `T = i32`: one call,
no emptiness, run 0). A generic duplicable instance may emit one extra temporary versus its
concrete twin; that emission delta is acceptable and named. The discarded read (`b375_discard`)
and the non-taking argument (`b375_arg`) stay clean (no move, no empty). Expected: `alt` called
once in all five consuming shapes; the store probe runs 12, the return/field read-backs run 12,
the array/conditional probes run 12; the discarded/argument controls run 0; ASan/LSan-clean.

**AR-21 — `DEF-376`: the guarded take carries the consumed non-place source, including its
pointer fill.** Ch 04 §3 (`:53` and the `&&`/`||` row at `:63`): "Left operand first; the right
operand only when the result is not yet fixed"; Ch 04 §5 (`:108`): "only the chosen branch
evaluates." Ch 03 §3/O6 makes a value moved on any path count as moved at a join; the zeroed
source is the drop flag (Ch 17 §5). The deferred move-out channel carries places only, so
`g && (eat(take (unsafe *q)) == 1)` keeps the inline call and the pointee is not emptied on the
taken path. The take-deref materialization at `own/lower.rs:4866-4888` runs at statement entry
even under `defer_move_outs`, so `g = false` still calls an impure pointer
(`m376_impure_false`: calls = 1, run 10; expected calls = 0, run 0). Fix: gate the eager take
materialization on `defer_move_outs == 0`, and carry the pointer fill on the deferred record. The
carrier shape must change: `Materialize.move_outs` is `Vec<Place>` and cannot carry a raw-deref
pointee (a `Value`), so add `value_move_outs: Vec<Value>` to `Materialize` (`ir.rs:298`) while
`move_outs` stays the place channel; the pointer-temp fill is `Materialize.value`. The existing
pointee channel is `take_pointee_move_outs -> Vec<Value>` (`own/call.rs:768`) emitted as
`MoveOutValue` (`ir.rs:377`); emit `MoveOutValue` in both guard forms in `codegen/guards.rs`.
The exhaustive-match sites to audit: `ir.rs` (`Materialize`), `codegen/guards.rs`,
`codegen/function.rs`, `codegen/cleanup.rs`, `own/generics.rs`, `own/instance_check.rs`,
`own/loops.rs`. Expected: accepted; `g = true` run 0, `g = false` run 0
with the impure pointer not called; ASan/LSan-clean for both selections.

**AR-22 — `DEF-378`: the unguarded codegen value/read path must consult the hidden-destination
predicate.** Ch 04 §3 (`:53`) fixes the function-call and array-literal evaluation order; Ch 04
§15 makes the literal elements typed by the expected type; Ch 17 §5 fixes the destination move
shape. A call returning a fixed view array (`str[2]`) writes through a hidden destination
parameter (`static void cl_n1(ClStr __cl_ret[2])`), so the literal must bind that destination at
**every** position. The guarded positions use `materialize_guard_literal`; the **unguarded
codegen value/read path** is decided by `struct_literal_needs_stores`
(`codegen/function.rs:1890`), which is not consulted for the literal's value/projection-base
position, so `Inner { row: n1() }.row[0][0]` (`m378_readpos`) keeps the brace-list/call form and
is red. The lowerer predicate `literal_field_needs_stores` (`own/lower.rs:4563`) already returns
true for a call-valued fixed-array field (the field is not a brace list), so the element-wise
store machinery exists; the missing link is the read-path call site. Fix: consult
`struct_literal_needs_stores` (or route the literal through the lowerer's store form) at the
unguarded value/read position, so the destination is bound and the call is emitted into it.
Expected: `readpos`, `gridel`, and `guardrowcond` build strictly and run 0; each call runs once;
the concrete brace-list control is unchanged.

**AR-23 — `DEF-379`: the owning nested conditional's argument carrier and fill stay in its guard
path.** Ch 04 §3 (`:63`, the `&&`/`||` row): the right operand evaluates only when the left does
not fix the result; Ch 04 §5 (`:108`): "only the chosen branch evaluates." The eager carrier is
`materialize_owned_argument`'s by-value binding (`own/lower.rs:5109-5131`), which emits an
`Assign` into `__cl_owned0` at statement entry even when `defer_move_outs > 0`; the fill position
is emitted by `codegen/guards.rs` (`:106-117` for the statement form, `:1016` for the guarded
expression form), and the drop/re-fill hoist is `guards.rs:273-307`. The observed C fills the arm
carrier before `if (g && consume(...))`, so a call runs on the skipped `g == false` path (calls =
3, expected 2). Fix: defer the final by-value argument binding under `defer_move_outs` into the
`Materialize` channel instead of the eager `Assign`, so the guarded codegen fills it on the
evaluated path; the phase's memory lens also analyzes the guard drop/re-fill interaction
(`guards.rs:273-307`) for a re-executed guard. A loop fixture (`b379_loop`) re-executes the
guard twice per use (observed calls = 6, run 2; expected calls = 4, run 0); `b379_eager` is the
unguarded eager-order control (run 0). Expected: calls = 2 and run 0 in the single-pass fixture,
calls = 4 and run 0 in the loop fixture; ASan/LSan-clean; the impl pin asserts `__cl_owned0`'s
fill is inside the guarded evaluated path and the re-fill releases the previous value once.

**AR-24 — `DEF-390`: a qualified call carries its module identity through every lookup.** Ch 10 §5
MOD-13 (`:112`): "Qualified access `A.B.f()` is always available, with or without an import"; Ch
04 §8 (`:180`): `A.B.f` is "module-qualified name or function". The checker already types the call
with the module's signature (the mismatch control reports `E10200`); the emitter computes the
callee name in `lower_call` (`own/call.rs:308`) via `callee_name` (`own/call_check.rs:291-298`),
which reduces the qualified path to its last segment, and `resolve_function` (`codegen.rs:701`)
resolves own-first, so the caller's same-named function is emitted. Note the site distinction:
`own/call.rs:255` is the qualified **func-field** arm, not the call-name path. Fix: thread the
resolved `(module, name)` through the lowered callee and use it in the mode, parameter, and return
lookups as well as the symbol — `codegen/calls.rs`, `codegen/expr.rs`, `codegen/function.rs`; the
module path wins over a same-named current-module function, and a lexical local/parameter still
shadows the head (batch-L AR-25). Expected: `probe.m390.lib.f()` runs the module's `f` and
returns 1 (run 0); the signature shape builds and runs 0; the qualified func-typed global control
`m.hook()` stays 0 (verified green at the head); the mismatch keeps its `E10200`.

**AR-25 — Interaction with batch L's standing rulings.** (a) Batch-L AR-25 (module path wins for
reads and calls when no lexical local shadows the head) governs `DEF-390` and `DEF-385`. (b)
Batch-L AR-24 (host/m68k ABI layout; no new layout numbers) governs the cross-profile emission
checks for `DEF-365`, `368`, `378`, `384`, `385`. (c) Batch-L AR-22 (a `raw E` member read is a
value form) does **not** apply to `DEF-375`/`376`: a raw pointee deref `*p` is a place (EX-14
lists a dereferenced raw pointer), not a value-form enum member read. (d) Batch-L AR-21 (the
`m09` oracle re-pin) is untouched: no oracle contradiction arises here. (e) Batch-L AR-9 (origin
suites, additions only) is reused as AR-26. (f) The frozen citations used by this register are
Ch 16 BR-4 `spec/16-borrow-rules.md:190`, Ch 03 §2 `spec/03-ownership.md:70-71`, Ch 08 §4
`spec/08-arrays-strings.md:100-111`, Ch 04 §13 `:291` and §5 `:120-121`, `A.B.f` `:180`, the
`&&`/`||` rows `:63` and `:107`, the place rule §4 `:87`, and unary `*` `:112`.

## Process and Verification Rulings

**AR-26 — Test placement and the re-pin rule.** Additions only to the origin suites:
- `checker_conformance_{spec,impl}_test.rs`: P1 (`DEF-360`, `361` control, `DEF-363`), all of P2
  (`DEF-364`, `365`, `368`), and all of P3 (`DEF-371`, `384`, `385`, `386`).
- `callable_release_integrity_{spec,impl}_test.rs`: P1 `DEF-390` (the callee-identity origin
  suite).
- `conditional_projection_residue_{spec,impl}_test.rs`: all of P4 (`DEF-367`, `377`), P5
  (`DEF-372`, `373`, `374` control), P6 (`DEF-375`, `376`), and P7 (`DEF-378`, `379`). The
  `DEF-367` bounds-fault case also lives here, using an inline `assert_traps` helper (precedent
  `borrow_store_integrity_spec_test.rs:363`, which asserts the exact first stderr line on both the
  plain and sanitizer builds).
- `module_initializer_integrity_{spec,impl}_test.rs`, the frozen spec suites (`*_spec_test.rs`
  under `compiler/tests/`, including `run_fault_spec_test.rs`), and the corpus suites are
  regression oracles only. The Amiga fault list is untouched.
- **No new suite pair is created.** A case that cannot be hosted naturally requires a register
  entry and explicit user approval before a pair is added (AR-35).
- **Re-pin rule:** only `_impl_test.rs` byte-identity pins may be re-pinned when a legitimate
  emission delta is proven by the differential. **Spec cases are never edited**; a spec case that
  fails means the implementation is wrong.

**AR-27 — Lenses and the dispatch matrix.** P1: **all four** — the dot-call route is a
symbol/IR-adjacent change and carries the memory-safety auditor (correctness + semantics + memory
+ emission). P2: all four. P3: correctness + semantics + emission-integrity (the nullable-handle
control runs ASan in the verification; no ownership/drop change). P4: **all four** — including the
semantics reviewer (the place rule). P5: correctness + memory-safety + semantics (no emission).
P6: all four. P7: all four. The read-only auditors stay read-only. This supersedes the earlier
"emission every phase" and "semantics not P4" wording.

**AR-28 — Verification command set and per-phase legs.**
- Build: `make -C docker exec CMD="cargo build --workspace"`.
- Compile: `make -C docker exec CMD="target/debug/clc --profile host -o out.c prog.clo && cc
  -std=c99 -O0 -g -Wall -Wextra -Werror -o out.bin out.c"` (the `&& cc` spelling; the pinned
  container).
- ASan: `make -C docker exec CMD="cc -std=c99 -O0 -g -fsanitize=address -no-pie -o
  build/batchM-plan/out/<name>.asan.bin <out>.c"`, then
  `ASAN_OPTIONS=detect_leaks=1:symbolize=0 timeout 30 <name>.asan.bin`.
- Suites: `make check` per phase; focused suites via
  `make -C docker exec CMD="RDC_ASAN=1 cargo test --test <suite>"` for the memory legs and
  `make -C docker exec CMD="cargo test --test <suite>"` for the plain legs.
- Profiles: `clc --profile hosted-amiga` plus the pinned `m68k-amigaos-gcc` leg for `DEF-365`,
  `368`, `378`, `384`, `385`; `clc --profile freestanding` for `DEF-365` and the no-import
  `m364_control_fs` (the `m364_control` fixture imports `std.text` and is **not**
  freestanding-eligible, `E11200`). The freestanding exclusions are recorded: `DEF-368`/`384`/`385`
  import `std.text`/`std.mem`, which the freestanding profile rejects with `E11200` (observed,
  not a defect).
- Per-phase legs: P2 adds the `RDC_ASAN=1` suite leg and the `DEF-368` hosted-amiga emission; P3
  adds the nullable-handle ASan run; P4 compiles the accepted `DEF-367` store forms on
  hosted-amiga (`clc --profile hosted-amiga` plus the pinned `m68k-amigaos-gcc`); the no-import
  `m364_control_fs` is compiled on hosted-amiga and freestanding (the std-importing
  `m364_control` is host-only).
- **Emission differential:** every emission-changing phase (P1, P2, P4, P6, P7 at minimum) runs a
  base/head differential over its named probe set and the corpus, with the command
  `make -C docker exec CMD="target/debug/clc --profile host -o out.c <file>"` plus `cc`; every
  delta is classified (intended fix / byte-identical control / regression) and the spec cases are
  never edited (AR-26).
- `make verify` at close-out.

**AR-29 — No spec, registry, or golden edit; park protocol.** Every fix reuses a frozen code:
`E10011`, `E10012`, `E10030`, `E10026` (control only), `E10200`, `E10202`, `E10221`, `E11103`,
and the runtime trap `E11211`. No new diagnostic, registry row, chapter, or golden. A row whose
fix turns out to need one is parked mid-phase with its probe evidence and recorded in the findings
ledger; the phase continues with its remaining rows. Exception: `DEF-367` has no park fallback —
it becomes a phase blocker and a user ruling (AR-34(a)).

**AR-30 — Complexity and runtime deviation.** Every fix rides an existing hook (the dot-call
check, the import pass, the callee identity, the per-instance duty/audit pattern, the type
resolver, the store dispatch, the loan transfer, the owning-deref materializer, the guard
lowering). A phase's fix surface may touch the module that owns the judgment beyond the task's
file list; the deviation is recorded in this register and covered by the phase's lenses (the
batch-I/J/K/L precedent).

**AR-31 — `DEF-361` control evidence (six fixtures).** No red reproduction could be constructed
at the head. The green fixture set is exactly: `m361_local.clo` + holder (local shadow),
`m361_param.clo` + holder (parameter shadow), `matrix/d_param_generic.clo` (generic-body local),
`matrix/d_lambda_expr.clo` (expression-bodied lambda parameter), `matrix/d_lambda_block.clo`
(block-bodied lambda parameter), and `probes/m361_control.clo` (the no-import baseline); all
compile strictly and run 0. The **module-own/module-initializer
shape is removed from the control set**: it is the `DEF-363` import/`let` collision, and P1's fix
turns it red (it must report `E11103`, not run 0). `matrix/d_comp.clo` (`E90001`, comptime
unsupported) and `matrix/d_capture.clo` (`E10405`, missing capture list) are unrelated dialect
errors and are annotated, not controls. The class is structurally closed by the batch-L phase-2
C2/R1 corrections: function bodies and both lambda body forms walk with an empty scope map, so a
body/parameter name can no longer resolve to an import (`own/generics.rs:34-52`, `:400-410`). The
phase pins the six fixtures; no code change. Frozen rule: Ch 10 §5 MOD-15/16
(`spec/10-modules.md:169-170`).

**AR-32 — `DEF-374` control evidence.** The generic and concrete twins both report two `E10026`
at the same span at the head (`m374_condelem.clo` = `m374_concrete.clo`, 12:24). The phase pins
the count and span parity; no code change. Frozen rule: Ch 07 STR-11/12 (`spec/07-structs.md:106-107`).

**AR-33 — `DEF-367` slice spelling.** A slice value cannot be produced from a fixed array inside
the probe: Ch 08 §1 AR-2 (`spec/08-arrays-strings.md:23`: "A fixed array does not decay to a
pointer") and O9 ("There are no local view expressions") forbid the local slice binding, and
`rawSlice` is not exported by `std.mem`. The slice spelling is therefore pinned compile-only: an
uncalled `func viaslice(borrow ps: Elem[], i: i32)` must emit the element store and its bounds
guard in the translation unit; the runnable spellings are the local array, the borrow parameter,
the generic borrow, the constant index, compound, update, deep chain, and target-order forms. The
implementation pin asserts the emitted store; the runtime case covers the callable forms; the
bounds-fault case is in the residue suite (AR-26).

**AR-34 — Stop conditions and blockers.** (a) `DEF-367`: if the indexed-element/member store
cannot be emitted on a profile without a new C declarator form, the phase **stops** (blocker) and
asks the user for a ruling with the emitted-C evidence; parking is not allowed (the frozen place
rule forbids the rejection fallback). (b) `DEF-384`: if extending the nullable check to nested
type arguments rejects the accepted `Array<String?>` control, the phase records the false
positive and **narrows** the check to the invalid non-capable kinds (never parks); the control
is the proof. (c) `DEF-385`: a
cross-module last-segment type-name collision is out of scope; the phase records it as a residual
if a probe shows it. (d) `DEF-390`: if a lexical local shadows the module head, the value-headed
reading wins (batch-L AR-25) and the case is not a module call; the phase pins that shadow
control.

**AR-35 — Test re-pin rule.** Spec cases (`*_spec_test.rs`) are immutable oracles and are never
edited to match an implementation; a failing spec case means the implementation is wrong. Only
`_impl_test.rs` byte-identity pins may be re-pinned, and only when the per-phase emission
differential proves the delta is intended and behavior-preserving; the re-pin and its evidence
are recorded in the findings ledger. A new suite pair needs a register entry and user approval
(AR-26).

**AR-36 — Hook ownership and execution dependencies.** P1 owns the dot-call written-argument route
and its call-site type-argument inference; P2 owns the per-instance conditional duty; P3 owns the
plain-call literal context. There is **no shared hook** between them. The execution dependencies
are: P1 → P2 and P1 → P3 (the written-argument route settles the call-site inference the other
two respect), P2 → P3 (the conditional's expected-type arm check is one of the context-typing
positions P3 extends), P2 → P5 (the element substitution feeds the loan positions), P6 → P7 (the
deferred move-out/Materialize channel feeds the guard fill), and P4 is independent. The scope is
severity-first scope (the critical is planned first); execution is dependency-ordered, so P4
runs fourth.

**AR-37 — Verification legs and the emission differential.** Every emission-changing phase runs a
base/head differential over its named probe set (P1: `m360_*`, `m361_*`, `m363_*`, `m390_*`; P2:
`m364_*`, `m365_*`, `m368_*`; P4: `m367_*`, `m377_*`; P6: `m375_*`, `m376_*`; P7: `m378_*`,
`m379_*`) plus the corpus. The differential command and the classification are in AR-28. P3 is
included when the qualified type path changes an emitted symbol. The P2/P3 memory legs and the
profile legs are the ones in AR-28.

**AR-38 — Close-out accounting.** At close-out, every row filed during execution is added or
checked in the roadmap with its reproduction and evidence. The `DEF-378` grid-element fixture
(`gridel`) incidentally closes **`DEF-380`** (the multi-extent view-array field): record the
boundary (which grid shapes are covered and which remain) and mark `DEF-380` incidental or open
accordingly. `DEF-382` is **not** claimed incidental via the `DEF-371` fix: that fix is scoped to
a concrete parameter at a generic callee, while `DEF-382`'s `identity<u8>(5)` is the separate
generic-argument route; record `DEF-382` as open unless the fix also substitutes explicit type
arguments. The next-batch count is recomputed from the roadmap; the two control rows (`DEF-361`,
`DEF-374`) move to `Done` with their control-pin evidence; the accounting states the totals
explicitly (18 red rows Done, 2 controls Done, 2 parked majors open, the remaining minors counted
by audit — no "minors untouched" claim without the count).

## Open Questions (collected for the user — none blocks execution)

- The two parked majors' rulings (`DEF-167`, `DEF-171`) are still due; they stay out of this
  batch (AR-1).
- `DEF-384`'s ABI-key half is not reproducible as a defect (the nullable handle already keys and
  runs); the row is closed on the rejection half with the ABI control.

## Verified Current Status (AR-5 probes, 2026-10-17, head `86d6f51`)

Probe method: `make -C docker exec CMD="cargo build --workspace"`, then
`target/debug/clc --profile host -o out.c prog.clo` against the reconstructed minimal programs
(`build/batchM-plan/probes/`, `preflight/`, `matrix/`, `extra/`); the emitted C compiled with
`cc -std=c99 -O0 -g -Wall -Wextra -Werror` and run with `timeout 10`; the ownership rows also
built with `-fsanitize=address -no-pie` and run with
`ASAN_OPTIONS=detect_leaks=1:symbolize=0 timeout 30`. Multi-file probes pass every `.clo` file on
the command line (the file list is the program). The probes are gitignored scratch.

| Row | Probe | Verified current behavior |
|-----|-------|---------------------------|
| `DEF-360` | `m360_dotarg.clo`, `m360_borrowdot.clo`, `m360_genericcaller.clo` / `m360_genericcaller2.clo` (invalid `c: i32`), `m360_genericcaller_box.clo` (`c: Box`) | ❌ Red: accepted; strict build fails (`incompatible type for argument 2 of 'cl_pick__Box'` at 11:25; `cl_put__Box` at 11:20). The direct inferred and explicit twins each report one `E10200`; `m360_control.clo` runs 0. The valid `c: Box` generic caller is falsely rejected with **two** `E10200` (8:12 and 8:19); the `c: i32` shapes are rejection cases (one `E10200` at 8:12) |
| `DEF-361` | six fixtures: `m361_local.clo` + holder, `m361_param.clo` + holder, `matrix/d_param_generic.clo`, `matrix/d_lambda_expr.clo`, `matrix/d_lambda_block.clo`, `m361_control.clo` (no-import baseline) | ✅ **Control**: all six compile strictly and run 0; no red reproduction at the head (AR-31). The module-own/module-initializer shape is **removed** (it is the `DEF-363` collision and P1 turns it red); `matrix/d_comp.clo` is `E90001` (comptime unsupported) and `matrix/d_capture.clo` is `E10405` (missing capture list) — unrelated |
| `DEF-363` | `m363_importfirst.clo`, `m363_declfirst.clo`, `m363_direct.clo`; `m363_kinds_main.clo` + `m363_kinds_{func,struct,const}.clo` | ❌ Red: all accepted; compile strictly and run 0 (the collision is unreported); `m363_control.clo` also runs 0. The function/struct/const kind matrix is accepted with only unused-import warnings |
| `DEF-390` | `m390_main.clo` + `m390_lib.clo`; `m390_sigmain.clo` + `m390_siglib.clo`; `m390_hook_main.clo` + `m390_hook_lib.clo`, `m390_shadow_main.clo` + `m390_shadow_lib.clo` | ❌ Red: `probe.m390.lib.f()` is accepted and runs the caller's `f` (run 1, expected 0); the signature shape is accepted and the strict build fails (`incompatible type for argument 1 of 'cl_probe_m390_sigmain_f'`); the mismatch control reports one `E10200` at the argument; `m390_control.clo` runs 0. The qualified func-typed global `m.hook()` and the qualified-head local-shadow `m390_shadow` are green controls (run 0) |
| `DEF-364` | `m364_condopaque.clo`, `m364_nullarm.clo`, `m364_direct.clo`, `m364_ctx.clo` | ❌ Red: (a) accepted, strict build fails (`type mismatch in conditional expression`); (b) accepted, strict build fails (`pointer/integer type mismatch` and `void *` to `i32`). The direct contexts (let, return, struct field, call argument) are accepted and fail the strict build (`m364_direct`/`m364_ctx`). The concrete disagreeing call already reports one `E10200` at 7:22 (`m364_concrete`). Controls `m364_control.clo` (host) and the no-import `m364_control_fs` (host and freestanding) run 0 |
| `DEF-365` | `m365_elemkey.clo` | ❌ Red: accepted; strict build fails (`stray '#' in program`, `cl_id__` undeclared); the emitted key is `cl_id__#element of #element of T` on host, hosted-amiga, and freestanding |
| `DEF-368` | `m368_elemview.clo` (generic), `m368_concrete.clo` | ❌ Red: accepted; strict build fails (`incompatible types ... assigning to 'ClStr' from 'ClString'` at 8:9); the concrete twin runs 0; hosted-amiga emits the same assignment |
| `DEF-371` | `m371_literal.clo`, `m371_control.clo` | ❌ Red: one `E10200` (`cannot convert 'i32' to 'usize'`) at 15:25; the concrete twin compiles and runs 0 |
| `DEF-384` | `m384_annot.clo`, `m384_callarg.clo`, `m384_nested.clo`, `m384_count.clo`; `m384_nullabletypearg.clo`, `m384_nullablehandle.clo` | ❌ Red: the field annotation, the explicit call type argument, and both nested cases (`Array<Array<i32?>>`, `Array<Wrap<i32?>>`) are accepted and run 0. The duplicate-count case reports the three existing `E10011` sites (nullable parameter, nullable return, `let x: i32?`) but neither the `Array<i32?>` annotation nor the `arrayNew<i32?>()` argument; after the fix the **five** sites report once each, no duplicates. The combined probe is accepted and fails the strict build (`void *` to `i32` in `cl_push__i32Q`); `Array<String?>` keys (`ClArray__StringQ`) and runs 0 |
| `DEF-385` | `m385_qualifiedtype.clo` | ❌ Red: accepted; strict build fails (`incompatible types ... to 'i32' from 'ClArray__i32'` at 4:9; `cl_push__i32` argument type); hosted-amiga emits `i32 g`. The unqualified control runs 0 |
| `DEF-386` | `m386_matrix.clo`, `m386_i8_neg5.clo`, `m386_i8_neg128.clo`, `m386_i8_neg129.clo`, `m386_u8_neg5.clo`, `m386_f32_neg5.clo`, `m386_bool_neg5.clo`, `m386_control.clo` | ❌ Red: the eight-position matrix (let, module value, argument, return, field, array element, conditional arm, assignment) reports eight `E10200` (`i32`→`i16`); `i8 = -5` and `i8 = -128` report `E10200`; `i8 = -129` and `u8 = -5` report `E10200` (expected `E10202`); `f32 = -5` and `bool = -5` report `E10200` and must stay `E10200`; the positive control runs 0 |
| `DEF-367` | `m367_indexed.clo`, `m367_compound.clo`, `m367_update.clo`, `m367_chain.clo`, `m367_order.clo`, `m367_gen_elemset.clo`, `m367_gen_elemstore.clo`, `m367_bounds.clo`, `b367_nested.clo`, `b367_two.clo`, `b367_whole.clo` | ❌ Red (critical): accepted; strict build ok; `m367_indexed` run 191 (calls = 1, expected 4); `m367_compound` 253; `m367_update` 255; `m367_chain` 249; `m367_order` 247; the generic member spelling is dropped (255) and the whole-element store runs 0 with a LeakSanitizer leak (`m367_gen_elemstore`); `b367_nested`/`b367_two` run 255; `b367_whole` runs 0 with a leak; `m367_bounds` runs 0 (no `E11211`); the constant index emits; ASan clean where no memory error is expected |
| `DEF-377` | `m377_member.clo`, `m377_index.clo`, `m377_array.clo`, `m377_indexbase.clo`, `m377_deep.clo`; `m377_unsafe_cond_member.clo`, `m377_unsafe_cond_index.clo`, `m377_unsafe_prefix.clo`, `m377_unsafe_prefix2.clo`, `m377_unsafe_bare.clo`; `m377_controls.clo`, `m377_unsafe_ptr.clo` | ❌ Red: the five forms are accepted; runs 255/249/249/249/255 — the stores and RHS side effects are dropped; ASan/LSan-clean (the store is absent). The `unsafe`-wrapped conditional member/index stores are accepted and run 0. Green controls: `s.f.g`, `s.arr[i]`, and the raw member chain `p.f.g` inside `unsafe { }` (run 0). The prefix raw forms `unsafe p.f`/`p.f.x` report `E10221` at the head and become accepted (with a read-back) after the place-test change; the prefix conditional `unsafe (c?a:b).v` stays `E10221` |
| `DEF-372` | `m372_elementread.clo`; `m372_direct.clo`, `m372_member.clo` | ❌ Red: accepted; strict build ok; run varies; ASan `heap-use-after-free` (the sanitizer is the oracle). The direct element argument and the member twin report one `E10030` |
| `DEF-373` | `m373_conditional.clo`, `m373_arrayliteral.clo`, `m373_singleview.clo`, `m373_union.clo`, `b373_mix.clo`, `b373_diff.clo`, `b373_fill.clo`; `m373_direct.clo` | ❌ Red: all accepted; strict build ok; run varies; ASan `heap-use-after-free` (the sanitizer report is the oracle); the `b373_mix`/`b373_diff`/`b373_fill` runs also vary; ASan UAF; the direct conditional twin reports one `E10030` |
| `DEF-374` | `m374_condelem.clo`, `m374_concrete.clo` | ✅ **Control**: both reject with two `E10026` at 12:24 (the per-arm parity, AR-32) |
| `DEF-375` | `m375_derefimpure.clo`, `m375_ret.clo`, `m375_field.clo`, `b375_arr.clo`, `b375_cond.clo`; controls `b375_discard.clo`, `b375_arg.clo`, `m375_dup.clo` | ❌ Red: the five consuming shapes are accepted; strict build ok; abort 134 + ASan `attempting double-free`; the store C calls `cl_alt__String` twice (`m375_derefimpure.c:702-703`); the return/field/array/conditional shapes double-free identically; the discarded read, the non-taking argument, and the concrete duplicable `T = i32` are clean (run 0) |
| `DEF-376` | `m376_guardedtake.clo`, `m376_impure_false.clo`, `m376_impure_true.clo` | ❌ Red: accepted; `m376_guardedtake`/`m376_impure_true` abort 134 + ASan `attempting double-free`; `m376_impure_false` runs 10 (the statement-entry materialization calls the impure pointer once even though `g = false`; expected calls = 0, run 0) |
| `DEF-378` | `m378_readpos.clo`, `m378_gridel.clo`, `m378_guardrowcond.clo` | ❌ Red: all accepted; strict build fails (`too few arguments ... expected 1, have 0` for `cl_probe_m378_*_n1/r1`); the read and unguarded positions are red because the builder does not consult the hidden-destination predicate |
| `DEF-379` | `m379_guardnest.clo`, `b379_loop.clo`, `b379_eager.clo` | ❌ Red: accepted; strict build ok; single-pass run 1 (expected 0), calls = 3 (expected 2); the loop fixture (two re-executions) runs 2 (expected 0), calls = 6 (expected 4); the by-value carrier fill precedes `if (g && consume(...))`; the eager (unguarded) control runs 0 |

**The two reclassifications**: `DEF-361` and `DEF-374` are verified green at the head and are
carried as pinned controls in their phases; neither has a residual in another row's scope. The
other 18 rows are red and planned.
