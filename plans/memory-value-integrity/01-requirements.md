# Requirements: Memory and Value Integrity (Batch N)

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

Close the 23 rows named by the dispatch (9 🔴 criticals and 14 🟠 majors) in seven machinery
phases. Twenty-one rows are red at the planning head `f6da78f` and are implemented red → green;
two (`DEF-427`, `DEF-447`) are verified not reproducible and are closed as false positives with
green controls and probe evidence (AR-27, AR-28). Every requirement below is conformance to the
frozen specification: the fix makes an accepted program build strictly and run with the language's
semantics, stops an invalid-C acceptance, makes a runtime ownership failure impossible, or — for
the two false positives — pins the behavior the filed row misread. No new language surface, no new
diagnostic code, no specification or registry edit (AR-33).

The behavior choices the rows left open are resolved in the register: the `Deref`-rooted member
store is **emitted**, never rejected (AR-17); the generic conditional/borrow moves report
`E10034` per owning instance, matching their concrete twins (AR-21, AR-22); the two non-reproducing
rows close as controls (AR-27, AR-28); and the division guard resolves its operand type per
instance (AR-23). The two ruling-parked majors (`DEF-167`, `DEF-171`) and every other row stay out
(AR-1).

> **Numbering note:** the R-numbers follow the row list's grouping (the loan rows, then the deref,
> store, per-instance, and literal rows, then the controls); the execution phase of each
> requirement is given in the traceability table at the end. R3 (`DEF-430`) is a P2 row while
> R4 (`DEF-431`) is P1; this is a numbering convenience, not an execution order.

## Functional Requirements

### Must Have

- [ ] **R1 — An exact whole-binding self-assignment keeps the element loans** (`DEF-428`,
  critical): `s = s` for a `str` binding and `ys = ys` for a `str[2]` binding preserve the
  binding's element-loan record, so a later `fill(s, borrow slot)` reports one `E10030` at the
  fill (Ch 16 §2, BR-1) and no TU is produced; an in-scope read-back stays accepted.
- [ ] **R2 — A composite copy source into an element or projected destination transfers its
  positions** (`DEF-429`, critical): `grid[0] = c ? xs : zs`, `out2[0] = c ? xs[0] : zs[0]`, and
  `g[0] = [xs[0], xs[1]]` record the source positions as a union on the destination, so each later
  fill reports one `E10030`; the copies themselves stay accepted.
- [ ] **R3 — A runtime-index store's union loans reach a constant-read copy** (`DEF-430`,
  critical): after `xs[k] = a.v` with a runtime `k`, `let s: str = xs[1]; fill(s, ...)` reports
  one `E10030` at the fill; the constant-index store control keeps its report; the copy is
  otherwise accepted.
- [ ] **R4 — The copy transfer keeps the collector's prefix fallback** (`DEF-431`, critical):
  after `rows[0] = xs`, `let s = rows[0][0]; fill(s, ...)` reports one `E10030` at the fill,
  matching the direct `fill(rows[0][0], ...)` control; the direct control keeps its report.
- [ ] **R5 — A non-place element records its provenance owners, including a temporary**
  (`DEF-432`, critical): `let ys: str[2] = [mk().v; "EE"]; fill(ys[0], ...)` reports one `E10030`
  at the fill; the direct `fill(mk().v, ...)` twin keeps its report; an in-scope use stays
  accepted.
- [ ] **R6 — The cross-frame fallback consults binding-loan owners** (`DEF-433`, critical): both
  `let s = pick(xs)` and `fill(pick(xs), slot)` report one `E10030` at the fill when `pick`
  returns a view of loaned element storage; the direct element fill keeps its report; an in-scope
  result stays accepted.
- [ ] **R7 — An element-store value goes through the composite-aware collector** (`DEF-435`,
  major): `g[1] = [; a.v]` followed by `let s = g[1][0]; fill(s, ...)` reports one `E10030`,
  matching the direct `fill(g[1][0], ...)` read; the direct read keeps its report; the store
  itself stays accepted.
- [ ] **R8 — A `Deref` anywhere in a projection chain is pinned before the move** (`DEF-437`,
  critical): `let v: String = (*p).s`, `return (*p).s`, `Holder { t: (*p).s }`, and
  `eat(take (*p).s)` each move the pointee once; the read-backs are correct and the runs are
  ASan/LSan-clean; the conditional-arm form stays a control.
- [ ] **R9 — The conditional target's consumed save consults the value channel** (`DEF-439`,
  major): `v = (g && eat(take (unsafe *p))) ? x : y` with `p = &v` does not free the consumed
  save; `g = true` and `g = false` run 0 and are ASan/LSan-clean; the false path leaves v intact.
- [ ] **R10 — A borrowing intrinsic leaves a raw-deref read unchanged** (`DEF-440`, critical):
  `while (i < 2 && len(*alt(p,q)) == 2)` binds and releases nothing, runs twice, calls `alt` once
  per condition evaluation, and is ASan/LSan-clean.
- [ ] **R11 — A guarded take of a member over a raw pointer records the pointee move-out**
  (`DEF-441`, critical): `g && (eat(take (*alt(p,q)).name) == 1)` empties the selected member on
  the evaluated path; both selections are ASan/LSan-clean; the false path does not call.
- [ ] **R12 — The `Deref`-rooted member store is emitted, never rejected** (`DEF-421`, major):
  `(*p).f`, `(*p).f.x`, `(*p).arr[i].x`, and `unsafe (*p).f` are accepted as places (Ch 04 §4),
  emit the store, and read back the stored value; the `None` fallback is a debug assertion; the
  conditional-base rejection (`E10221`) and the block controls stay green.
- [ ] **R13 — The whole-element target is pinned before the RHS** (`DEF-423`, major):
  `a[idx] = mkrow()` stores into the pinned in-bounds element (no stack overflow), and
  `a[idx] = set_idx() ? a[idx] : ...` is a legal move with a correct destination read (no double
  free; the source read is a separate use-after-move and the spec case reads the destination); the
  bounds trap `E11211` stays.
- [ ] **R14 — The RHS self-read runs before the destination is cleared** (`DEF-424`, major):
  `a[i].names = [stringOf(a[i].names[0]), ...]` and `a[i].inner = *(&a[i].inner)` run 0 with their
  values, and the constant-index self-copy `w.items[1] = w.items[1]` in a generic body is skipped
  and runs 0 with the value intact; the concrete constant-index `E10026` is unchanged.
- [ ] **R15 — The element and raw-target stores materialize their owning-deref RHS** (`DEF-436`,
  major): `xs[i] = *alt(p,q)` and the raw-target form call `alt` once, read and empty the same
  pointee, store the correct value, and are ASan/LSan-clean.
- [ ] **R16 — A conditional store of a by-value generic parameter reports `E10034` per owning
  instance** (`DEF-410`, major): `pick<T>(c, h) { let r: String? = c ? h : null; }` with
  `T = String` reports one `E10034` at the store and no TU, matching the concrete twin; a
  duplicable instance stays accepted.
- [ ] **R17 — An element move-out through a borrowed parameter reports `E10034` per owning
  instance** (`DEF-425`, major): the generic `w.items[i].name = w.items[j].name` with an owning
  `T` reports one `E10034` at the RHS element read, matching the concrete `borrow` twin; the
  duplicable instance stays accepted; the `take` twin keeps its `E10026`.
- [ ] **R18 — The generic compound division guard resolves its operand type per instance**
  (`DEF-426`, major): the generic and concrete `/=`, `%=` twins emit the same guards; `/0`,
  `INT_MIN / -1`, and `INT_MIN % -1` trap `E11210` (no SIGFPE).
- [ ] **R19 — The literal read/store predicates recurse into conditional arms** (`DEF-445`,
  major): `(ok ? Inner { row: n1() } : Inner { row: n2() }).row[0][0]` and its nested
  field/element forms build strictly and run 0; each call runs once; the brace-list control is
  byte-identical.
- [ ] **R20 — A take conditional literal in an eager ternary arm uses the guarded literal store**
  (`DEF-446`, major): the ternary-arm form builds strictly and runs 0; each call runs once; the
  `if`-condition control stays green.
- [ ] **R21 — A take of a conditional array literal of places uses the element-wise store form**
  (`DEF-448`, major): `consume(take (ok ? [a, b] : [c, d]))` with `str[2]` view places builds
  strictly and reads the chosen arm's values; the unconditional controls stay green.
- [ ] **R22 — The `len` view-length controls pin the `DEF-427` false-positive closure**
  (`DEF-427`, major, control): `len(w.items[i])` and `len(w.items[i].name)` are 2 in the
  concrete, generic, borrow, and loop-alias spellings; the filed `len(w.items[i][0])` double index
  reads a `u8` byte; the row is closed with the probe evidence and no code change.
- [ ] **R23 — The inline read-base owning literal control pins the `DEF-447` false-positive
  closure** (`DEF-447`, major, control): the filed `G { rows: [n1(), n1()], tag: stringOf("x")
  }.rows[0][0][0]` and its call-row and view-copy variants are LSan-clean; the row is closed with
  the probe evidence and no code change.

### Should Have

- [ ] **S1 — Origin-suite regression:** every existing case in the origin suites and the frozen
  spec suites stays green; only `_impl_test.rs` byte-identity pins may be re-pinned, with the
  differential evidence (AR-30).
- [ ] **S2 — Emission determinism:** re-emitting any accepted probe is byte-identical.
- [ ] **S3 — Profile sanity:** the accepted store forms compile on hosted-amiga; the
  literal-destination forms compile there too; the `String`-importing probes are recorded as
  freestanding-ineligible (`E11200`), not a defect.

### Won't Have (this batch)

- A new diagnostic code, registry row, chapter, or golden (AR-33).
- A conditional-value base store (`E10221` stands; batch-M AR-44).
- A new place kind, a per-arm selecting store route, or a new IR move-out channel (AR-17, AR-14).
- `DEF-167`, `DEF-171`, and every other major/minor (AR-1).

## Acceptance Criteria

| # | Criterion | Evidence |
|---|-----------|----------|
| A1 | Every red row's spec case fails without the fix and passes after it | the phase red/green records in `05-findings.md` |
| A2 | The two control resolutions have green controls and the probe evidence | the P7 record and the phase-7 ledger |
| A3 | Every accepted shape builds strictly (`-Wall -Wextra -Werror`) | the per-phase strict sweeps |
| A4 | Every ownership shape is ASan/LSan-clean after the fix | the `RDC_ASAN=1` suite legs and host ASan runs |
| A5 | The traps are `E11210`/`E11211`, not a signal | the inline `assert_traps` cases |
| A6 | No existing origin/spec/frozen suite regresses | `make check` per phase; `make verify` at close-out |
| A7 | The emission differential classifies every delta | the per-phase differential (AR-32) |
| A8 | The roadmap and the ledger account for all 23 rows | the close-out (AR-36) |

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
| R17 | `DEF-425` | major | P5 | [03-05](03-05-per-instance-generic.md) |
| R18 | `DEF-426` | major | P5 | [03-05](03-05-per-instance-generic.md) |
| R19 | `DEF-445` | major | P6 | [03-06](03-06-literal-destinations.md) |
| R20 | `DEF-446` | major | P6 | [03-06](03-06-literal-destinations.md) |
| R21 | `DEF-448` | major | P6 | [03-06](03-06-literal-destinations.md) |
| R22 | `DEF-427` | major (control) | P7 | [03-07](03-07-control-resolution.md) |
| R23 | `DEF-447` | major (control) | P7 | [03-07](03-07-control-resolution.md) |
