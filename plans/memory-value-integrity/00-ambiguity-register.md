# Ambiguity Register: Memory and Value Integrity (Batch N)

> **Status**: ✅ GATE PASSED — every row's behavior direction is fixed below before any code.
> Twenty-one rows are red and planned; `DEF-427` and `DEF-447` are verified not reproducible and
> are planned as false-positive control resolutions (AR-27, AR-28). No row is parked.
> **Created**: 2026-10-18 (planning)
> **Batch**: N of the defect mandate (23 rows: 9 🔴 criticals + 14 🟠 majors)
> **Baseline**: head `f6da78f`; the probe evidence is in `02-current-state.md`
> **Auto-design**: active — the standing mandate: all defects first, batches continue without
> further go-aheads; severity-first across the backlog. Reserved decisions (spec text, new
> user-visible behavior, acceptance criteria) are not touched: every fix is conformance to the
> frozen specification. A row whose fix would need a spec or registry ruling is parked and
> recorded (AR-33).

## Why this batch

Batch M closed the last 20 un-parked rows of its round. Its phase reviews filed the 23 rows this
batch closes: the memory-safe value-transfer, store-lowering, per-instance, and literal-destination
residues that remained after the batch-M corrections. Nine are the **last un-parked criticals**;
they lead the plan and every one of them carries the memory-safety lens. All 23 were rebuilt and
probed at head `f6da78f` (AR-3); 21 reproduce, and 2 do not. The two non-reproducing rows are
closed as false positives with green controls and probe evidence — they are not parked and no
compiler change is planned for them (AR-27, AR-28).

| # | Category | Ambiguity / Gap | Options Presented | Resolution | Status |
|---|----------|-----------------|-------------------|------------|--------|
| 1 | Scope | Which rows batch N closes | A) the 23 rows named by the dispatch (9 criticals + 14 majors); B) the parked majors; C) the rest of the majors/minors | **A** — pre-decided by the dispatch and recorded, not re-litigated. The scope is **severity-first** (all nine criticals are in P1–P3); execution is dependency-ordered (AR-4). B (`DEF-167`, `DEF-171`) stays parked; C is out of scope. | ✅ Resolved (pre-decided) |
| 2 | Process | The specification-first and zero-ambiguity discipline | — | **Spec cases first, then red, then implement, then green, then impl pins.** Every row's expected outcome is fixed below before code: a frozen diagnostic (code + site), a strict-build-clean run with its exit value, an ASan/LSan-clean run, or a green control. No implementer question may remain; a discovered gap re-enters this register before code | ✅ Resolved (pre-decided) |
| 3 | Verification | The current status of the 23 rows at the head | rebuild and probe the compiler at `f6da78f` (container build; host `clc`; strict `cc`; ASan/LSan) | **The AR-3 table and `02-current-state.md`**: 21 red; `DEF-427` and `DEF-447` not reproducible. The sanitizer report or the strict C compiler is the oracle where the plain run is nondeterministic | ✅ Resolved |
| 4 | Design | Phase grouping and sequencing | A) seven machinery phases, severity-first; B) one row per phase; C) the batch-M phases | **A** — P1 loan transfer on stored copies (4), P2 loan sources (3), P3 raw-deref move-outs (4), P4 store lowering/order (4), P5 per-instance generic judgments (3), P6 literal destinations (3), P7 control resolution (2). All criticals land in P1–P3; each phase is coherent for its lenses (AR-31). P1/P2 extend the same `own/place.rs` bookkeeping; P3 extends the deref/guard channels; P4 the store paths; P5 the instance audits; P6 the literal lowering; P7 is fix-free | ✅ Resolved (delegated) |
| 5 | Behavior | Per-row handling mode | red → green, control, or park | **Red → green for 21 rows**; **control resolution for `DEF-427` and `DEF-447`** (false positives closed with green controls and probe evidence; no code change). A red case must fail without the fix; a control must fail if a fix regresses it | ✅ Resolved (delegated) |

## Probe Method (AR-3)

- Build: `make -C docker exec CMD="cargo build --workspace"`; the compiler is
  `target/debug/clc` at `f6da78f`.
- Emit: `target/debug/clc --profile host -o out/<name>.c <name>.clo`.
- Strict build: `cc -std=c99 -O0 -g -Wall -Wextra -Werror -o out/<name>.bin out/<name>.c`.
- Run: `timeout 30 out/<name>.bin`.
- Memory: `cc -std=c99 -O0 -g -fsanitize=address -no-pie -o out/<name>.asan out/<name>.c`;
  `ASAN_OPTIONS=detect_leaks=1:symbolize=0 timeout 30 out/<name>.asan`.
- A plain run whose exit varies because it reads freed storage is recorded as "varies"; the
  AddressSanitizer report is the oracle. A strict C failure is recorded with the first diagnostic.
- Every claim in `02-current-state.md` comes from these probes at `f6da78f`; the probe sources are
  reproduced in that document and in the gitignored `build/batchN-plan/probes/` tree.

## Per-Row Behavior Rulings

**AR-6 — `DEF-428`: an exact whole-binding self-assignment keeps the element loans.** The frozen
rule is Ch 16 §2 (`spec/16-borrow-rules.md:51-58`: a view binding loans the owner's storage) and
BR-1 (`:187`: a loan never outlives its owner). `s = s` for `s: str` and `ys = ys` for
`ys: str[2]` are legal copies, but the binding effect replaces the destination's element-loan
record before the copy transfer snapshots the source (`own/statements.rs:951` calls
`set_binding_loans`; `own/place.rs:1096` drops `element_loans`; `own/expr.rs:1296` snapshots after
the effect). **Chosen fix:** detect the exact self-assignment (`place_target(value)` equals the
target binding with the same projections) and leave the destination's element-loan record
unchanged; the store is a no-op. The snapshot-before-effect alternative is not chosen: it touches
every binding effect for the one self-assignment shape. Expected: `fill(s, borrow slot)` after
the self-assignment reports one `E10030` at the fill; no TU; an in-scope read-back (no escape)
stays accepted; the array form reports at its fill. The self-assignment itself is not rejected.

**AR-7 — `DEF-429`: a composite copy source into an element or projected destination transfers
its positions.** Frozen: Ch 16 §2 and BR-4 (`spec/16-borrow-rules.md:190`: a move of the owner
requires no live loan). `grid[0] = c ? xs : zs`, `out2[0] = c ? xs[0] : zs[0]`, and
`g[0] = [xs[0], xs[1]]` are copies of views; the destination's positions must be the **union** of
the source positions, exactly as the batch-M `DEF-373` copy transfer. The copy route is gated by
`projections.is_empty()` at `own/expr.rs:1296-1307`, so a projected/element destination records
nothing. Fix: pass the destination `base` (the projections) into the composite-aware collector and
write the union once. Expected: one `E10030` at each later fill; the copies themselves stay
accepted; the direct conditional twin keeps its report.

**AR-8 — `DEF-430`: a runtime-index store's binding-union loans reach a constant-read copy.**
Frozen: Ch 16 §2 and §6 (`:233`: a use reachable only through the owner's release is `E10030`).
`xs[k] = a.v` with a runtime `k` records the loan conservatively (the store cannot name the
element); `let s: str = xs[1]` reads a constant element, and the copy transfer consults only the
position records, so it loses the runtime union (`own/place.rs:1192` `move_element_loans`,
`:1369` `collect_place_copy_loans`). Fix: include the binding-union loans in the constant-read
transfer when a runtime record exists. Expected: `fill(s, borrow slot)` reports one `E10030`; the
copy is otherwise accepted. **Blast radius:** the direct runtime-index fill (`fill(xs[1])` after
`xs[k] = a.v`, probe `d430c`) is accepted at the head; the conformant outcome is also one
`E10030`. If the fix reaches it, record the intended delta and pin it; if not, the phase files a
residual row with the probe (it is outside the row's copy wording but the same frozen rule).

**AR-9 — `DEF-431`: the copy transfer keeps the collector's prefix fallback.** Frozen: Ch 16 §2;
the direct `fill(rows[0][0], ...)` already reports `E10030`. `move_element_loans`
(`own/place.rs:1192`) strips only positions **below** the read (`path.strip_prefix(source)`),
while the collector (`own/place.rs:1369-1398`) also keeps a position that **covers** the read
(`read.starts_with(path)`), because a fill record covers the element it is read through. Add the
same prefix fallback to `move_element_loans`. Expected: `rows[0] = xs; let s = rows[0][0]`
reports one `E10030` at the fill; the direct fill keeps its report.

**AR-10 — `DEF-432`: a non-place element records its provenance owners, including a temporary.**
Frozen: Ch 16 §2. `let ys: str[2] = [mk().v; "EE"]` stores a view derived from the temporary
`mk()`; the non-place branch of `collect_place_copy_loans` (`own/place.rs:1424-1432`) records only
`view_loans`, not `provenance_loan_owners` (`:1435`), so the temporary owner is lost. Fix: record
the provenance owners of a non-place element/fill value under the destination path, exactly as the
place branch does. Expected: one `E10030` at the later fill; the direct `fill(mk().v, ...)` twin
keeps its `E10030`; an in-scope use stays accepted.

**AR-11 — `DEF-433`: the cross-frame fallback consults binding-loan owners.** Frozen: Ch 16 §2
and §6. `pick(xs): str` returns a view of `xs[0]`'s loaned storage; the loan sits in the
**binding loans** of `s`, but `frame_owned_view_owner` (`own/call_check.rs:585`) first asks
`recorded_position_frame_owners` (`own/place.rs:975`), which only reads `element_loans`, then falls
back to provenance, which names no owner for a fresh call result. Fix: make the fallback consult
the binding-loan owners (`binding_loan_owners` / `view_loan_destination_owners`) before the
provenance walk. Expected: both `let s = pick(xs)` and `fill(pick(xs), slot)` report one `E10030`
at the fill; the direct element fill keeps its report; an in-scope result stays accepted.

**AR-12 — `DEF-435`: an element-store value goes through the composite-aware collector.** Frozen:
Ch 16 §2. `g[1] = [; a.v]` stores a fresh member view into every element of the row; the direct
read `fill(g[1][0], ...)` reports `E10030`, but the copy `let s = g[1][0]` is accepted because the
whole-element `StorePointer` path (`own/expr.rs:2557-2621`) never calls the copy collector. Fix:
route the element-store value through the same composite-aware collector (the `base` parameter is
ready), with the element's destination path. Expected: one `E10030` at the copy's later fill,
matching the direct read; the direct read keeps its report; the store itself stays accepted.

**AR-13 — `DEF-437`: a `Deref` anywhere in a projection chain is pinned before the move.**
Frozen: Ch 04 §5 (`:112`: unary `*` yields a place of type `T`) and §7 (the raw dereference
operation), Ch 17 §5 (`spec/17-codegen.md:184-193`: a move is
a copy plus a zero of the source; one release per owner). `let v: String = (*p).s`, `return
(*p).s`, `Holder { t: (*p).s }`, and `eat(take (*p).s)` are accepted and double-free because
`materialize_owning_deref_at` (`own/lower.rs:3699-3721`) matches only a **top-level** `Deref`; a
projection through it is not materialized, so the pointee is copied without being emptied (or the
pointer is evaluated twice). **Chosen fix:** extend `materialize_owning_deref_at` to walk a
`Member`/`Index` projection chain to its `Deref` root and pin that pointer once. A separate
projected-read materializer is not added. Expected: each form moves the pointee once, the
read-backs are correct, and ASan/LSan are clean. The conditional-arm form is already fixed by
batch M and stays a control.

**AR-14 — `DEF-439`: the conditional target's consumed save consults the value channel.**
Frozen: Ch 04 §3 (`spec/04-expressions.md:53`, left-to-right), §5 (`:108`, only the chosen branch
evaluates), and Ch 17 §5. `arm_consumes_target` (`codegen/function.rs:2400-2418`) checks
`Materialize.move_outs` but not `value_move_outs`, so `v = (g && eat(take (unsafe *p))) ? x : y`
with `p = &v` saves v's old value and frees it after the take already freed it. Fix: consult
`value_move_outs` too, with the runtime address comparison the place channel uses. Expected: the
taken path does not free the consumed save; both `g = true` and `g = false` run 0 and are
ASan/LSan-clean; the false path leaves v intact.

**AR-15 — `DEF-440`: a borrowing intrinsic leaves a raw-deref read unchanged.** Frozen: Ch 16 §2
(a view/read lend does not move) and Ch 17 §5. `while (i < 2 && len(*alt(p,q)) == 2)` passes a
raw-deref read to the borrowing `len` intrinsic; `bind_intrinsic_argument`
(`own/lower.rs:4171-4242`) binds it into a released temporary, freeing the caller's pointee on the
second iteration. Fix: return the `Deref` unchanged for a borrowing intrinsic, exactly as the
non-take rule in `materialize_owned_argument` (`own/lower.rs:5390-5401`) already does. Expected:
no bind/release, the loop runs twice, `alt` is called once per condition evaluation, and the run
is ASan/LSan-clean.

**AR-16 — `DEF-441`: a guarded take of a member over a raw pointer records the pointee move-out.**
Frozen: Ch 17 §5 (one release per owner) and Ch 04 §3/§5 (the guarded path evaluates only the
chosen branch). `take_pointee_move_outs` (`own/call.rs:804-821`) matches only a top-level `Deref`,
so `(*alt(p,q)).name` records no pointee move-out and the caller's member stays owned after the
callee freed it. Fix: record the pointee move-out for a projection rooted at a `Deref` (walk to
the root). Expected: `g && (eat(take (*alt(p,q)).name) == 1)` empties the selected member on the
evaluated path; `g = true` and `g = false` are ASan/LSan-clean; the false path does not call.

**AR-17 — `DEF-421`: the `Deref`-rooted member store is emitted, never rejected.** The frozen
place list (Ch 04 §4, `spec/04-expressions.md:87`) includes a struct field and a dereferenced raw
pointer; the unary `*` row (`:112`) makes `*p` a place, so `(*p).f`, `(*p).f.x`, and
`(*p).arr[i].x` are places. `place_target` (`own/place.rs:1931-1976`) has no `Deref` arm, so the
store lowers through the raw-deref target path (`own/expr.rs:1003-1073` only matches a bare `*p`)
or is dropped; `unsafe (*p).f` is newly accepted by the batch-M `Unsafe` arm and also dropped.
Fix: route the `Deref`-rooted member/index target through the raw-deref store path in the
assignment lowering (the store-specific route), and make the `None` fallback a debug assertion —
**no** `E10221` fallback. A `Deref` arm is **not** added to `place_target` (it would broaden every
place consumer). A conditional-rooted base stays rejected (batch-M AR-44, `E10221`). Expected: the
four forms emit the store and read back the stored value; the block controls and the
conditional-base rejection stay green.

**AR-18 — `DEF-423`: the whole-element target is pinned before the RHS.** Frozen: Ch 04 §3
(left-to-right) and §9/Ch 08 IX-1 (`spec/08-arrays-strings.md:105`, bounds-checked). `a[idx] =
mkrow()` and `a[idx] = set_idx() ? a[idx] : ...` leave the base/index inline; C does not sequence
the assignment operands, so a RHS call can change `idx` before the target is read. The member
route already pins (`own/expr.rs:2361-2377`); the whole-element route pins only when the RHS
contains an update (`:2564`). Fix: always materialize the whole-element base and index before the
RHS. Expected: `a[idx] = mkrow()` stores into the pinned in-bounds element (no stack overflow);
the conditional self-store is a **legal move** (target `a[0]` pinned, chosen arm `a[1]` moved into
it) with no double free; the spec case reads the destination (`a[0].name` = "bb") because reading
the moved-out source `a[1]` is a separate use-after-move, not the row's subject (`at2`, which does
not read the source, runs 0). The filed `at1`'s source read is replaced by a destination read — a
mechanical probe correction of the same kind as batch-M AR-40, not a behavior change. The bounds
trap `E11211` stays.

**AR-19 — `DEF-424`: the RHS self-read runs before the destination is cleared.** Frozen: Ch 04 §3
(left-to-right); Ch 17 §5 (drop-then-store). The fixed-array literal store saves and clears the
destination before the literal's element expressions run (`codegen/function.rs:1556-1605`), so
`a[i].names = [stringOf(a[i].names[0]), ...]` and `a[i].inner = *(&a[i].inner)` read cleared
storage and SEGV. **Chosen fix:** evaluate the literal's self-reading element expressions into
statement temporaries **before** the save/clear (the frozen left-to-right order), then perform the
save/clear and store the temporaries; the saved bytes still release the old owners once. The
"read the saved copy" alternative is not chosen (it would redirect the RHS reads through the save
and complicate the existing store form). The constant-index sibling `w.items[1] = w.items[1]` in
a generic body (`d424f`) emits drop + self-copy + zero and SEGVs because the `Statement::Move`
emission (`codegen/function.rs:659-690`) skips an identical source/target only for a runtime-index
source; fix: skip the identical constant-index move too (batch-M F3's self-copy rule). Expected:
the `au1`/`ak1`/`ak2` shapes run 0 with their values; the constant-index self-copy runs 0 with the
value intact; the concrete constant-index spelling's `E10026` is unchanged (the red shape is the
generic owned-move acceptance).

**AR-20 — `DEF-436`: the element and raw-target stores materialize their owning-deref RHS.**
Frozen: Ch 17 §5 and Ch 04 §3. `xs[i] = *alt(p,q)` lowers the RHS through `consume_place`
(`own/expr.rs:2599-2613`); the raw-deref target does the same (`own/expr.rs:1060-1070`), so an
impure pointer is evaluated twice (read and empty) and the wrong pointee is emptied. The named
store already runs `materialize_owning_deref` (`own/statements.rs:495`). Fix: run the shared
materializer on the RHS in both lowerings. Expected: `alt` is called once, the pointee is read and
emptied through the same address, `xs[i] = *alt(p,q)` is ASan/LSan-clean and reads back the value;
the raw-target form stores the correct value and is clean. (The filed raw-target double-free is
the same defect; the head's pinned LHS leaves the RHS re-evaluation, observed as a wrong value in
`d436b`.)

**AR-21 — `DEF-410`: the conditional store of a by-value generic parameter reports `E10034` per
owning instance.** Frozen: BR-5 (`spec/16-borrow-rules.md:191`: a lend parameter may not be
released or stored) and the concrete twin (`d410b`: one `E10034` at 6:5). The by-value parameter
`h: T` is a shared lend; the conditional store `let r: String? = c ? h : null` moves it. The
generic body is accepted because `conditional_move_sources` (`own/expr.rs:175-205`) skips an arm
whose type mentions a type parameter (`:191`), so no lend duty is recorded. **Chosen fix:** record
the conditional arm as a maybe-owning source through the existing `MoveIfOwning` effect (or the
equivalent `mark_moved_if_owning` duty), so the per-instance audit
(`own/instance_check.rs:210-260`) reports one `E10034` when the substitute owns. No new effect
kind is added. The note's "caller does not empty the moved local" is the runtime symptom of the
accepted program; the frozen rule and the concrete twin make rejection the conformant outcome.
Expected: `T = String` reports one `E10034` at the store, no TU; a duplicable `T` (for example a
scalar struct) stays accepted; the concrete twin is unchanged.

**AR-22 — `DEF-425`: the element move-out through a borrowed parameter reports `E10034` per
owning instance.** Frozen: BR-5 and STR-11/12 (`spec/07-structs.md:106-107`). The generic body
`copy<T>(borrow w: T, i, j) { w.items[i].name = w.items[j].name; }` moves an owning element out of
a borrow parameter; the concrete `borrow` twin reports one `E10034` at the RHS element read, and
the `take` twin reports `E10026` (a different rule: the top-level element-move limit). The generic
body is accepted because the element move through a **parameter** is not recorded:
`note_borrowed_element_move` (`own/place.rs:629-666`) only accepts a `BindingKind::Local` borrow,
and `mark_moved_if_owning_at` (`:541-590`) has no projection path for the substituted element.
**Chosen fix:** extend `RequiresNotLend` with an optional projection path and record the element
move through a `BindingKind::Param`/`Mode::Borrow`; the audit substitutes the parameter type,
projects the path (`project_place_path`), and reports one `E10034` when the selected element owns.
The duplicable element stays accepted. Do **not** reuse the `E10026` element-move audit (that is
the take/local rule) and do not add a second element audit. Expected: `T = W` (owning `String`
members) reports one `E10034`; `T =` a duplicable payload is accepted; the concrete and take twins
keep their reports.

**AR-23 — `DEF-426`: the generic compound division guard uses the resolved per-instance operand
type.** Frozen: EX-5 (`spec/04-expressions.md:378`) and the operator table (`:100`): a zero
divisor, or MIN/-1, faults `E11210`. `division_guard_condition`
(`compiler/src/codegen/guards.rs:1444-1460`) reads `left.ty`; a generic compound operand records
the parameter type, so the guard returns `None` (the concrete twin emits three guards, `d426d`;
the generic emits none, `d426e`, and `/0` SIGFPEs, `d426c`). Fix: feed the resolved per-instance
operand type into the guard (the store's `element_ty`/the compound operand's resolved type). The
guard is emitted per instance, not at the declaration. Expected: the generic and concrete twins
emit the same guards; `INT_MIN / -1`, `INT_MIN % -1`, and `/0` trap `E11210` (no SIGFPE); the
in-scope value results are unchanged.

**AR-24 — `DEF-445`: the literal read/store predicates recurse into conditional arms.** Frozen:
Ch 04 §3 (evaluation order), §15 (literal elements typed by the expected type), and Ch 17 §5. A
call-valued fixed-array field has no C initializer form, so the literal must bind the hidden
destination. `literal_read_needs_stores` / `literal_field_read_needs_stores` /
`array_element_read_needs_stores` (`own/lower.rs:4839/4849/4864`) return false for a
`Conditional`, so `(ok ? Inner { row: n1() } : Inner { row: n2() }).row[0][0]` keeps the brace
list and the strict build fails (`too few arguments`), also nested in a field or element. Fix:
recurse into the conditional arms in the three predicates (the existing store form already
handles the arms). Expected: the read base and the nested field/element forms build strictly and
run 0; each call runs once; the concrete brace-list control is byte-identical.

**AR-25 — `DEF-446`: a take conditional literal in an eager ternary arm uses the guarded literal
store.** Frozen: Ch 04 §3 and §5 (only the chosen branch evaluates) and Ch 17 §5. The
`if`-condition form builds and runs 0 (`d446d`); the ternary-arm form
`g ? consumeO(ok ? Outer { ... } : Outer { ... }) : 0` keeps the literal as a brace list and the
strict build fails (`too few arguments`, `d446c`). Fix: route the ternary-arm literal through the
same guarded literal store the phase-7 carrier uses (the `if`-condition/guarded path already
does). Expected: the ternary form builds strictly and runs 0; each call runs once; the
`if`-condition control stays green.

**AR-26 — `DEF-448`: a take of a conditional array literal of places uses the element-wise store
form.** Frozen: Ch 04 §3/§5, Ch 17 §5, and the aggregate-literal positional rule (Ch 04 §15).
`consume(take (ok ? [a, b] : [c, d]))` with `str[2]` view places emits the bare C
`(ok ? { a, b } : { c, d })` (`d448c`: `expected expression before '{'`). This is distinct from
`DEF-446`: the conditional array literal of view places has no complete C value, so it must be
materialized into a destination and the chosen arm stored element-wise (the existing conditional
fixed-array assign machinery, `codegen/function.rs:1723`). Expected: the strict build succeeds and
the run reads the chosen arm's values; the unconditional array-literal controls stay green; the
`String`/`Box` array forms that already build are controls.

**AR-27 — `DEF-427`: false positive — the double index selects a `u8`.** Frozen: Ch 08 SL-2
(`spec/08-arrays-strings.md:72`: `len` is the element count of an array/slice/`str`) and §4
(`:100-111`: indexing a `str` yields a `u8` byte). The filed form `len(w.items[i][0])` with
`items: str[2]` first selects the `str` element (`w.items[i]`), then its first **byte**
(`[0]`); `len` of a scalar is not the element's length, so the emitted `0` is not the filed
"expected 2". The expected 2 belongs to `len(w.items[i])`, which is green in every probed shape
(concrete, generic, borrow, loop alias, `str`/`String` elements), as does the single-index member
form `len(w.items[i].name)`. **Ruling: close `DEF-427` as a false positive with the probe
evidence; no code change.** The control case pins the view-length readings and the byte-index
variant (a `u8` read, not a length). A future checker rule for `len` of a scalar is a separate
scope; no frozen code is added here.

**AR-28 — `DEF-447`: false positive — no reproduction.** The filed inline read-base owning
literal `G { rows: [n1(), n1()], tag: stringOf("x") }.rows[0][0][0]` and its nested-field,
`String`-field, and view-copy variants are LSan-clean at the head (`d447a`, `d447c`, `d447d`,
`d447f`, `d447g`); no ref that builds leaks. The batch-M phase-7 C1 correction bound an owning
read-base literal at body scope, which is the mechanism the row describes, and the phase-7
emission re-audit had already annotated the row "not reproduced". **Ruling: close `DEF-447` as a
false positive with the probe evidence; no code change.** A green LSan control pins the filed shape
and the body-scope binding; if the execution-head probe ever shows a leak, the control task
reopens the row (AR-35(d)).

## Process and Verification Rulings

**AR-29 — Interaction with the batch-M rulings.**
- Batch-M AR-44 (the syntactic closed place list) governs `DEF-421`: a `Deref`-rooted field is a
  place and is **emitted**; a conditional-value base stays rejected (`E10221`), and no per-arm
  selecting route is added.
- Batch-M AR-45 (the write-conflict reading) governs the loan phases: a write's `E10031` stands,
  the binding/provenance loans stay, and the batch-L single-view controls keep their counts. The
  P1/P2 fixes must not disturb them.
- Batch-M F3 (the identical whole-element self-copy skip) is extended by `DEF-424` to the
  constant-index generic `Statement::Move`; the whole-element `StorePointer` form already skips.
- Batch-M phase-7 C1 (the body-scope owning read-base literal) is the mechanism behind `DEF-447`;
  the control pins it.
- Batch-M AR-20/AR-21 (the shared owning-deref materializer and the `value_move_outs` channel)
  are the hooks P3/P4 extend. No new IR channel is added: `DEF-439`/`441` use `value_move_outs`,
  `DEF-437`/`436` use the materializer.
- The batch-M `DEF-309`/`DEF-438` extensions and the frozen fault data stay untouched.

**AR-30 — Test placement and the re-pin rule.** Additions only to the origin suites; **no new
suite pair**:
- `compiler/tests/conditional_projection_residue_{spec,impl}_test.rs`: P1, P2, P3, P4, P6,
  `DEF-426`'s guard, and `DEF-447`'s control (the loan/store/deref/literal origins from batch M).
- `compiler/tests/borrow_generic_integrity_{spec,impl}_test.rs`: P5's `DEF-410` and `DEF-425`
  (the per-instance lend judgments are that suite's subject).
- `compiler/tests/array_value_integrity_{spec,impl}_test.rs`: P7's `DEF-427` `len` control (the
  fixed-array `len` origin).
- `compiler/tests/borrow_store_integrity_{spec,impl}_test.rs` is the origin of the E10031
  write-conflict controls and is a regression oracle only.
- **Re-pin rule:** only `_impl_test.rs` byte-identity pins may be re-pinned when the per-phase
  differential proves the delta intended. **Spec cases are never edited**; a failing spec case
  means the implementation is wrong.

**AR-31 — Lenses and the dispatch matrix.** P1: correctness + semantics + memory-safety (loan
bookkeeping reaches the ownership rules; no emission change). P2: same. P3: **all four** (the
`value_move_outs`/materializer channel is IR and emission). P4: **all four** (store shape and
ordering). P5: **all four** (the rejections are memory/ownership; `DEF-426` changes emitted C).
P6: **all four** (the literal store form is emission and ordering). P7: correctness + memory-safety
(the controls and the LSan evidence; no code). The read-only auditors stay read-only.

**AR-32 — Verification command set and per-phase legs.**
- Build: `make -C docker exec CMD="cargo build --workspace"`.
- Focused suites: the residue pair for P1–P4/P6/P7-447; the borrow-generic pair for P5 and the
  array-value pair for P7-427; plain and with `CMD="RDC_ASAN=1 cargo test --test <suite>"` for the
  memory legs (P1, P2, P3, P4, P5, P6, P7-447).
- Strict builds: `clc --profile host` + `cc -std=c99 -O0 -g -Wall -Wextra -Werror` for every
  accepted shape.
- ASan/LSan: the host `-fsanitize=address -no-pie` runs, `detect_leaks=1:symbolize=0`,
  `timeout 30`.
- `E11210`/`E11211` traps: the inline `assert_traps` helper in the residue spec suite (precedent
  `borrow_store_integrity_spec_test.rs:363`); the frozen `run_fault_spec_test.rs` data and the
  Amiga fault list stay untouched.
- Profiles: `DEF-421`/`423`/`424`/`436` compile the accepted store forms on hosted-amiga;
  `DEF-445`/`446`/`448` compile-only on hosted-amiga. The `String`-importing probes are **not**
  freestanding-eligible (`E11200`), and this is recorded, not a defect.
- Emission differential: every emission-changing phase (P3, P4, P5, P6) runs a base/head
  differential over its named probe set and the corpus; every delta is classified (intended fix /
  byte-identical control / regression).
- `make verify` at close-out.

**AR-33 — No spec, registry, or golden edit; park protocol.** Every fix reuses a frozen code:
`E10030`, `E10031`, `E10034`, `E10026` (controls only), `E10221` (controls only), `E11210`,
`E11211`. No new diagnostic, registry row, chapter, or golden. A row whose fix turns out to need
one is parked mid-phase with its probe evidence and recorded in the findings ledger; the phase
continues with its remaining rows. The two false positives are **closed**, not parked (AR-27,
AR-28).

**AR-34 — Complexity and hook ownership.** Every fix rides an existing hook: the loan
transfer/replace/collector helpers, the shared materializer, the `value_move_outs` channel, the
intrinsic borrow rule, the store target builder and pin, the fixed-array assign form, the
per-instance lend/element duties, the division guard, and the literal store predicates. No new
pass, abstraction, or dependency. A fix may touch the module that owns the judgment beyond the
task's file list; the deviation is recorded and covered by the phase's lenses.

**AR-35 — Stop conditions and blockers.**
- (a) `DEF-421`: if the `Deref`-rooted store cannot be emitted on a profile without a new C
  declarator form, the phase **stops** and asks the user; rejecting a place the frozen list
  accepts is non-conformant.
- (b) `DEF-437`/`441`: if the fix needs an IR channel beyond `value_move_outs`/the materializer,
  stop and record the need; a new channel needs a ruling.
- (c) `DEF-423`/`424`: if pinning or RHS-first changes an accepted store shape that the corpus or
  the residue controls exercise, the controls are the oracle; narrow the fix and record the delta.
- (d) `DEF-427`/`447`: if the execution-head probe shows a red shape (the false-positive evidence
  no longer holds), the control-resolution task **reclassifies the row to a fix** and re-enters
  this register before code; a silent fix is forbidden.
- (e) `DEF-430`: if the direct runtime-index fill stays accepted after the copy fix, the phase
  files a residual row with `d430c`/`d430e`; if it starts rejecting, the intended delta is pinned.
- (f) `DEF-425`: if the per-instance element path cannot carry the projection without a new duty
  shape, record the shape and reuse the `RequiresElementMove` path type; do not add a second
  element audit.

**AR-36 — Close-out accounting.** At close-out: 21 red rows Done (9 criticals + 12 majors) and
the 2 false-positive controls Done with their probe/control evidence; the two parked majors
(`DEF-167`, `DEF-171`) stay open; every row filed during execution is added or checked with its
reproduction and base/head evidence (incidental vs open); the next-batch count is recomputed from
the roadmap.

**AR-37 — The incidental-control rule.** A row that is found green at the execution head becomes
a pinned control with its evidence (the batch-M `DEF-361`/`DEF-374` precedent); the row is not
silently dropped. `DEF-427` and `DEF-447` are not "incidental fixes": they were never
reproducible at `f6da78f`, so they are false-positive closures with a green control, and the
closure evidence is recorded in phase 7.

**AR-38 — Frozen citations used by this register.**

| Citation | Text used | Rows |
|----------|-----------|------|
| Ch 04 §3 (`spec/04-expressions.md:53`) | Evaluation is strictly left-to-right and deterministic | 428–436, 439, 441, 445–448 |
| Ch 04 §4 (`:87`) | The target must be a place: local, parameter, struct field, array element, or dereferenced raw pointer, otherwise `E10221` | 421 |
| Ch 04 §5 (`:108`) | Only the chosen branch of `?:` evaluates | 439, 441, 446, 448 |
| Ch 04 §5 (`:112`) | Unary `*` on `raw T` yields a place of type `T` | 421, 437 |
| Ch 04 EX-5 (`:378`) | Integer `/`/`%` by zero or MIN/-1 faults `E11210` | 426 |
| Ch 04 §13 (`:291`) | A literal is typed by its context | 445, 446, 448 |
| Ch 07 STR-11/12 (`spec/07-structs.md:106-107`) | Partial moves are top-level fields of owned locals/`take` params; element moves are `E10026` | 425 |
| Ch 08 SL-2 (`spec/08-arrays-strings.md:72`) | `len` is the element count of an array/slice/`str` | 427 |
| Ch 08 IX-1 (`:105`) | Indexing is bounds-checked; out of range faults `E11211` | 423 |
| Ch 16 §2 (`spec/16-borrow-rules.md:51-58`) | A view binding loans the owner's storage; the loan ends at the last use | 428–435 |
| Ch 16 BR-1 (`:187`) | A loan never outlives its owner (`E10030`) | 428–435 |
| Ch 16 BR-4 (`:190`) | A move of the owner requires no live loan | 429 |
| Ch 16 BR-5 (`:191`) | A lend parameter may not be released or stored (`E10034`) | 410, 425 |
| Ch 16 §6 (`:233`) | A use reachable only through the owner's release is `E10030` | 430, 433 |
| Ch 17 §5 (`spec/17-codegen.md:184-193`) | A move is a copy plus a zero of the source; the zeroed source is the drop flag; one release per owner | 436–441 |
