# Requirements: Memory and Value Integrity (Batch N)

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

Close the 23 rows named by the dispatch (9 🔴 criticals and 14 🟠 majors) in seven machinery
phases; `DEF-425` is parked pending the user's ruling on a frozen-spec conflict, leaving 22 active
rows (9 criticals + 13 majors). Twenty active rows are red at the planning head `f6da78f` and are
implemented red → green; two (`DEF-427`, `DEF-447`) are verified not reproducible and are closed
as false positives with green controls and probe evidence (AR-27, AR-28); `DEF-425` is recorded as
parked with its conflict evidence and a ruling request (AR-22). Every requirement below is
conformance to the frozen specification: the fix makes an accepted program build strictly and run
with the language's semantics, stops an invalid-C acceptance, makes a runtime ownership failure
impossible, or — for the two false positives — pins the behavior the filed row misread. No new
language surface, no new diagnostic code, no specification or registry edit (AR-33).

The behavior choices the rows left open are resolved in the register: the `Deref`-rooted member
store is **emitted**, never rejected (AR-17); the generic conditional move reports `E10034` per
owning instance, matching its concrete twin (AR-21); the two non-reproducing rows close as
controls (AR-27, AR-28); the division guard resolves its operand type through the already-
substituted value chain (AR-23); and `DEF-425` is parked rather than silently fixed or closed
(AR-22). The two ruling-parked majors (`DEF-167`, `DEF-171`), `DEF-425`, and every other row stay
out (AR-1).

> **Numbering note:** the R-numbers follow the row list's grouping (the loan rows, then the deref,
> store, per-instance, and literal rows, then the controls); the execution phase of each
> requirement is given in the traceability table at the end. R3 (`DEF-430`) is a P2 row while
> R4 (`DEF-431`) is P1; this is a numbering convenience, not an execution order. R17 onward were
> renumbered after `DEF-425` was parked (ST-17 and the later cases follow the same shift).

## Functional Requirements

### Must Have

- [x] **R1 — An exact whole-binding self-assignment keeps the element loans** (`DEF-428`,
  critical): `s = s` for a `str` binding and `ys = ys` for a `str[2]` binding preserve the
  binding's element-loan record, so a later `fill(s, borrow slot)` reports one `E10030` at the
  fill (Ch 16 §2, BR-1) and no TU is produced; an in-scope read-back stays accepted.
- [x] **R2 — A composite copy source into an element, projected, or runtime destination transfers
  its positions** (`DEF-429`, critical): `grid[0] = c ? xs : zs`, `out2[0] = c ? xs[0] : zs[0]`,
  `g[0] = [xs[0], xs[1]]`, and the runtime-destination forms `grid[k] = c ? xs : zs` followed by
  `fill(grid[1][0])` or `let s = grid[1][0]` all record the source positions as a union on the
  destination, so each later fill reports one `E10030`; the copies themselves stay accepted. The
  runtime destination records a wildcard `Element(None)` that matches any `Element(_)` at the same
  position prefix in `collect_place_copy_loans`, `move_element_loans`, and
  `place_recorded_position_owners` (`recorded_position_frame_owners` shares the match); the
  non-constant branch of `replace_element_loans` records the wildcard path with the union it writes
  to the binding and keeps that binding union for `E10031`. The constant-store sibling control
  `ys[0] = a.v; let s = ys[1]; fill(s)` stays accepted and clean.
- [x] **R3 — A runtime-index store's wildcard loans reach a constant-read copy** (`DEF-430`,
  critical): after `xs[k] = a.v` with a runtime `k`, `let s: str = xs[1]; fill(s, ...)` reports
  one `E10030` at the fill; the `k = 0` variant is recorded as the conservative expected outcome;
  the constant-store sibling read `ys[0] = a.v; let s = ys[1]` stays accepted and clean; the
  constant-index store control keeps its report; the copy is otherwise accepted. The wildcard
  `Element(None)` matches any `Element(_)` at the same position prefix in the three position
  readers; the non-constant branch records it in `element_loans` with the union it writes to the
  binding and keeps that union for `E10031`.
- [x] **R4 — The deeper-projection copy keeps the collector's prefix fallback** (`DEF-431`,
  critical): after `rows[0] = xs`, `let s = rows[0][0]; fill(s, ...)` reports one `E10030` at the
  fill, matching the direct `fill(rows[0][0], ...)` control. Phase 1 verifies this after the
  `DEF-429` gate: if the gate alone greens it, the row closes incidental with the evidence;
  otherwise the collector's prefix fallback (`own/place.rs:1212`) is added. The direct control
  keeps its report.
- [x] **R5 — A non-place element records its provenance owners, including a lowered temporary**
  (`DEF-432`, critical): `let ys: str[2] = [mk().v; "EE"]; fill(ys[0], ...)` reports one `E10030`
  at the fill; the direct `fill(mk().v, ...)` twin keeps its report; the lowered owner (a
  `__cl_owned0`/statement temporary) appears in the record; an in-scope use stays accepted.
- [x] **R6 — The cross-frame fallback consults binding-loan owners for both copy and direct-call
  forms** (`DEF-433`, critical): both `let s = pick(xs)` and `fill(pick(xs), slot)` report one
  `E10030` at the fill when `pick` returns a view of loaned element storage; the direct element
  fill keeps its report; a caller-parameter view stays accepted; an in-scope result stays accepted.
- [x] **R7 — An element-store value goes through the composite-aware collector** (`DEF-435`,
  major): the constant case `g[1] = [; a.v]` followed by `let s = g[1][0]; fill(s, ...)` and the
  runtime-element case `g[k] = [; a.v]` followed by a later constant read each report one
  `E10030`, matching the constant direct `fill(g[1][0], ...)` read (`d435b`, which reports
  `E10030`); the store itself stays accepted. Phase 1 verifies the constant case after the
  `DEF-429` gate (it is the prefix-fallback path); the runtime-element case needs the collector in
  `emit_indexed_store`. The runtime direct read (`q435runtime_direct`) is accepted and ASan-red at
  the head; it is the AR-35(e) blast radius: the intended delta is one `E10030`, and if it stays
  accepted the phase files a residual row with the probe. Both store cases close as incidental if
  the gate alone greens them.
- [x] **R8 — A `Deref` anywhere in a projection chain is pinned before the move** (`DEF-437`,
  critical): `let v: String = (*p).s`, `return (*p).s`, `Holder { t: (*p).s }`, and
  `eat(take (*p).s)` each move the pointee once; the read-backs are correct and the runs are
  ASan/LSan-clean; the three emission gates (`moves_pointee`, `return_source_needs_emptying`,
  `struct_literal_needs_stores`/the field loop) each cover the chain; the conditional-arm form
  stays a control.
- [x] **R9 — The conditional target's consumed save applies a runtime alias check** (`DEF-439`,
  major): `v = (g && eat(take (unsafe *p))) ? x : y` with `p = &v` does not free the consumed
  save; the then-branch zeroes the save only when the taken pointer aliases the target, the
  else-branch drops it normally and stores `y`; `g = true`, `g = false` (aliasing), and the
  non-aliasing control run 0 and are ASan/LSan-clean; the case uses distinct lengths
  (`v = "vv"`, `y = "yyyy"`) so the false arm's stored `y` is visible (the run is 2, not 0).
- [x] **R10 — A borrowing intrinsic or borrow argument leaves a raw-deref read unchanged**
  (`DEF-440`, critical): `while (i < 2 && len(*alt(p,q)) == 2)` and `read((*alt(p,q)).s)` bind and
  release nothing, run their expected paths, call `alt` once per evaluation, and are ASan/LSan-
  clean; the non-take rule covers a `Deref`-rooted projection chain.
- [x] **R11 — A guarded take of a member over a raw pointer records the projected member move-out**
  (`DEF-441`, critical): `g && (eat(take (*alt(p,q)).name) == 1)` empties only the selected member
  on the evaluated path; the sibling member stays owned and readable; both selections are
  ASan/LSan-clean; the false path does not call; the impure pointer runs once.
- [x] **R12 — The `Deref`-rooted member store is emitted, never rejected** (`DEF-421`, major):
  `(*p).f`, `(*p).f.x`, `(*p).arr[i].x`, and `unsafe (*p).f` are accepted as places (Ch 04 §4),
  emit the store, and read back the stored value; the base/index of a `Deref`-rooted target is
  pinned before the RHS (`(*p).arr[idx] = bump()` stores at the old index and calls `bump` once);
  the `None` fallback is a debug assertion; the structural `.len` `E10221` branch and the
  conditional-base control stay green.
- [x] **R13 — The whole-element target is pinned before the RHS** (`DEF-423`, major):
  `a[idx] = mkrow()` stores into the pinned in-bounds element (no stack overflow), and
  `a[idx] = set_idx() ? a[idx] : ...` is a legal move with a correct destination read (no double
  free); the post-store source read (`at1`) is a separate accepted use-after-move and is filed as a
  residual row at the phase-4 record; the bounds trap `E11211` stays.
- [x] **R14 — The RHS self-read runs before the destination is cleared** (`DEF-424`, major):
  `a[i].names = [stringOf(a[i].names[0]), ...]`, the two-element self-read, and the self-alias
  `a[i].inner = *(&a[i].inner)` / `x = *(&x)` run 0 with their values; the constant-index self-copy
  `w.items[1] = w.items[1]` in a generic body is skipped and runs 0 with the value intact; the
  concrete constant-index `E10026` is unchanged; the RHS temporaries have one statement lifetime,
  are zeroed, and are moved out once. The raw self-alias is recognized by `is_raw_self_alias`
  (unwrap `Deref(AddressOf(place))` and compare the inner place; `place_target` stays
  `Deref`-free); ST-14 pins the two self-alias shapes, the distinct-binding/place controls
  (`x = *(&y)`, `a[0].inner = *(&a[1].inner)`) with distinct contents, and a following
  distinct-value store through each self-alias place, so an over-broad no-op that loses a real
  store is visible.
- [x] **R15 — The element and raw-target stores materialize their owning-deref RHS** (`DEF-436`,
  major): `xs[i] = *alt(p,q)` and `unsafe *alt(rp,rq) = *alt(p,q)` each call its pointer
  expression once (one for the element form, two total for the raw-target form), read and empty
  the same pointee, store the correct value, and are ASan/LSan-clean.
- [x] **R16 — A conditional store of a by-value generic parameter reports `E10034` per owning
  instance** (`DEF-410`, major): `pick<T>(c, h) { let r: String? = c ? h : null; }` with
  `T = String` reports one `E10034` at the store and no TU, matching the concrete twin; the effect
  is selected in `prepare_binding_effect` and no report is emitted at the definition; a separate
  duplicable control (a generic whole move `let r: T = h;` at `T = P`, the duplicable struct used
  by `q410dup`) stays accepted.
- [x] **R17 — The generic compound division guard resolves its operand type per instance**
  (`DEF-426`, major): the generic and concrete `/=`, `%=` twins emit the same guards; `/0`,
  `INT_MIN / -1`, and `INT_MIN % -1` trap `E11210` (no SIGFPE); the guard already substitutes
  `left.ty`, so the fix resolves the unknown projection type through `resolved_place_ty`/
  `resolved_value_ty`.
- [x] **R18 — The literal read/store predicates recurse into conditional arms with the OR rule**
  (`DEF-445`, major): `(ok ? Inner { row: n1() } : Inner { row: n2() }).row[0][0]`, the mixed
  `ok ? place : literal` read base, and its nested field/element forms build strictly and run 0;
  each call runs once; the brace-list control is byte-identical.
- [x] **R19 — A take conditional literal in an eager ternary arm uses the guarded literal store**
  (`DEF-446`, major): the ternary-arm form builds strictly and runs 0; each call runs once; the
  `if`-condition control stays green.
- [x] **R20 — A take of a conditional array literal of places uses the element-wise store form**
  (`DEF-448`, major): `consume(take (ok ? [a, b] : [c, d]))` with `str[2]` view places builds
  strictly and reads the chosen arm's values; the unconditional controls stay green.
- [x] **R21 — The `len` view-length controls pin the `DEF-427` false-positive closure**
  (`DEF-427`, major, control): `len(w.items[i])` and `len(w.items[i].name)` are 2 in the concrete,
  generic, borrow, and loop-alias spellings; the filed `len(w.items[i][0])` double index is
  accepted, builds strictly, and is ASan/LSan-clean (its exit value is not asserted in the spec
  case); the exact `254`/`0` observations are implementation pins that document the current output,
  and a residual row is filed for the scalar-`len` fold; the row is closed with the probe evidence
  and no code change. A future scalar-`len` fix needs a user ruling because the residual row
  records the current fallback.
- [x] **R22 — The inline read-base owning literal control pins the `DEF-447` false-positive
  closure** (`DEF-447`, major, control): the filed `G { rows: [n1(), n1()], tag: stringOf("x")
  }.rows[0][0][0]` and its call-row and view-copy variants are LSan-clean; the row is closed with
  the probe evidence and no code change.

### Should Have

- [x] **S1 — Origin-suite regression:** every existing case in the origin suites and the frozen
  spec suites stays green; only `_impl_test.rs` byte-identity pins may be re-pinned, with the
  differential evidence (AR-30).
- [x] **S2 — Emission determinism:** re-emitting any accepted probe is byte-identical.
- [x] **S3 — Profile sanity:** the accepted store forms compile on hosted-amiga; the
  literal-destination forms compile there too; the `String`-importing probes are recorded as
  freestanding-ineligible (`E11200`), not a defect. At the P4/P6 records the executor runs the
  freestanding compile-only leg for every eligible scalar form and records the per-case
  ineligibility reason for each `E11200` case (including in task 8.1.1).

### Won't Have (this batch)

- A new diagnostic code, registry row, chapter, or golden (AR-33).
- A conditional-value base store (`E10221` stands; batch-M AR-44).
- A new place kind, a per-arm selecting store route, or a new IR move-out channel (AR-17, AR-14).
- A fix for `DEF-425`: it is parked pending the user's ruling (AR-22).
- `DEF-167`, `DEF-171`, and every other major/minor (AR-1).

## Acceptance Criteria

| # | Criterion | Evidence |
|---|-----------|----------|
| A1 | Every active red row's spec case fails without the phase fix and passes after it (an incidental `DEF-431`/`DEF-435` case may pass once the `DEF-429` gate lands) | the phase red/green records in `05-findings.md` |
| A2 | The two control resolutions have green controls and the probe evidence, and `DEF-425` is recorded as parked with its conflict evidence and ruling request | the P7 record, the P5 park record, and the phase ledgers |
| A3 | Every accepted shape builds strictly (`-Wall -Wextra -Werror`) | the per-phase strict sweeps |
| A4 | Every ownership shape is ASan/LSan-clean after the fix | the `RDC_ASAN=1` suite legs and host ASan runs |
| A5 | The traps are `E11210`/`E11211`, not a signal | the inline `assert_traps` cases |
| A6 | No existing origin/spec/frozen suite regresses | `make check` per phase; `make verify` at close-out |
| A7 | The emission differential classifies every delta | the per-phase differential (AR-32) |
| A8 | The roadmap and the ledger account for all 22 active rows plus the parked `DEF-425` | the close-out (AR-36) |

## Traceability

| Requirement | Row | Severity | Phase | Design |
|-------------|-----|----------|-------|--------|
| R1 | `DEF-428` | critical | P1 | [03-01](03-01-loan-store-transfers.md) |
| R2 | `DEF-429` | critical | P1 | [03-01](03-01-loan-store-transfers.md) |
| R3 | `DEF-430` | critical | P2 | [03-02](03-02-loan-sources.md) |
| R4 | `DEF-431` | critical | P1 | [03-01](03-01-loan-store-transfers.md) |
| R5 | `DEF-432` | critical | P2 | [03-02](03-02-loan-sources.md) |
| R6 | `DEF-433` | critical | P2 | [03-02](03-02-loan-sources.md) |
| R7 | `DEF-435` | major | P1 | [03-01](03-01-loan-store-transfers.md) |
| R8 | `DEF-437` | critical | P3 | [03-03](03-03-deref-move-outs.md) |
| R9 | `DEF-439` | major | P3 | [03-03](03-03-deref-move-outs.md) |
| R10 | `DEF-440` | critical | P3 | [03-03](03-03-deref-move-outs.md) |
| R11 | `DEF-441` | critical | P3 | [03-03](03-03-deref-move-outs.md) |
| R12 | `DEF-421` | major | P4 | [03-04](03-04-store-lowering-order.md) |
| R13 | `DEF-423` | major | P4 | [03-04](03-04-store-lowering-order.md) |
| R14 | `DEF-424` | major | P4 | [03-04](03-04-store-lowering-order.md) |
| R15 | `DEF-436` | major | P4 | [03-04](03-04-store-lowering-order.md) |
| R16 | `DEF-410` | major | P5 | [03-05](03-05-per-instance-generic.md) |
| R17 | `DEF-426` | major | P5 | [03-05](03-05-per-instance-generic.md) |
| R18 | `DEF-445` | major | P6 | [03-06](03-06-literal-destinations.md) |
| R19 | `DEF-446` | major | P6 | [03-06](03-06-literal-destinations.md) |
| R20 | `DEF-448` | major | P6 | [03-06](03-06-literal-destinations.md) |
| R21 | `DEF-427` | major (control) | P7 | [03-07](03-07-control-resolution.md) |
| R22 | `DEF-447` | major (control) | P7 | [03-07](03-07-control-resolution.md) |
| — | `DEF-425` | major (parked) | P5 record | [03-05](03-05-per-instance-generic.md) |
