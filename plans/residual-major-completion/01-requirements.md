# Requirements: Residual Major Completion (Batch M)

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

Complete the last 20 un-parked defect rows of the major backlog named by AR-1, severity-first
(the critical `DEF-367` first, then the 19 majors): seven phases grouped by machinery, 18 red
rows implemented with red → green cases and two rows (`DEF-361`, `DEF-374`) verified green at the
planning head and pinned as controls. Every row was re-probed against the compiler built from head
`86d6f51` (AR-5). Each requirement below is conformance to the frozen specification: the fix
makes an accepted program build and run with the language's semantics, stops an invalid-C
acceptance, or makes a runtime ownership failure impossible. No new language surface, no new
diagnostic code, no specification or registry edit (AR-29). The behavior choices the rows left
open are resolved inside the frozen rules in the register: `DEF-367` emits the store (the frozen
place list includes a field of an element; AR-6), `DEF-377` reports `E10221` (AR-7), `DEF-384`
rejects the invalid nullable scalar at nested type-argument positions and keeps the nullable
handle (AR-14), and `DEF-386` judges the folded negated value (AR-16).

## Functional Requirements

### Must Have

- [ ] **R1 — The dot-call written argument is checked** (`DEF-360`): a dot call whose receiver
  fixes the callee's first type parameter judges every written argument against the parameter
  resolved **at the call site** (the callee's type arguments are inferred there, not deferred
  against the enclosing function), so `b.pick(7)` on `pick<T>(take v: T, take w: T)` and the
  `borrow`-receiver twin report one `E10200` at the written argument, while a valid generic caller
  `outer<i32>(b, c)` with `b.pick(c)` stays accepted and runs.
- [ ] **R2 — The direct-name shadow stays green** (`DEF-361`, control): a local or parameter
  named like a direct imported binding keeps its own type at a dot receiver (the generic body
  iterates the local, not the import) in a body, a parameter, a generic body, and an expression-
  or block-bodied lambda; the six green fixtures are pinned as controls with no code change. The
  module-own/module-initializer shape is **not** a control (it is the `DEF-363` collision).
- [ ] **R3 — A declaration/import collision is reported** (`DEF-363`): binding one name twice in a
  module — an import local name and a same-module declaration of any kind (`func`, `struct`,
  `enum`, `handle`, `let`, `const`, `comptime`, `ffi` name/function, `intrinsic`), in either order
  — reports one `E11103` at the import item (Ch 10 MOD-12); the renamed-import control stays
  accepted.
- [ ] **R4 — A qualified call carries its module identity** (`DEF-390`): `m.f(...)` calls the
  loaded module `m`'s `f` and uses its modes, parameters, and return in both checking and
  emission, even when the caller declares a same-named function; the checker's module-signature
  typing and the emission agree, a signature mismatch still reports `E10200`, a lexical
  local/parameter still shadows the head (batch-L AR-25), and the qualified func-typed global
  `m.hook()` stays a green control.
- [ ] **R5 — The conditional is judged per instance and per context** (`DEF-364`): a conditional
  whose two arms are opaque at the declaration is judged with the substituted arm types at each
  instance — disagreeing concrete arms report one `E10200` at the offending arm, and a null arm at
  a non-nullable substituted context reports one `E10012` at the `null` (TY-7); the **direct**
  conditional takes the expected type in the let, return, field, and call-argument contexts, so
  `let y: i32 = c ? x : null;` reports one `E10012` instead of emitting invalid C; agreeing arms
  and non-null siblings stay accepted.
- [ ] **R6 — The element-of-element key resolves** (`DEF-365`): an indexing step over an opaque
  element binds the nested element placeholder from the substituted element type (a `str` row's
  element is `u8`), so the instance is keyed `cl_id__u8`, the translation unit has no `#`, and
  the program builds strictly on host and both m68k profiles and runs.
- [ ] **R7 — A substituted element member at a view destination widens** (`DEF-368`): a
  `String` member read from a substituted element assigned to a `str` destination emits the
  concrete twin's `cl_string_view` widening per instance; the generic and concrete spellings
  build strictly and run the same value.
- [ ] **R8 — A literal at a generic concrete parameter takes its type** (`DEF-371`): a literal
  written as an argument to a concrete parameter of a generic callee (for example `usize`) is
  typed by that parameter (LEX-14, Ch 04 §13), so `set<S[1]>(borrow a, 0)` is accepted, builds,
  and runs; the concrete twin stays green; the out-of-range input `1000000000000` reports
  `E10202`.
- [ ] **R9 — The nullable scalar type argument is rejected at nested positions** (`DEF-384`): `T?`
  on a type that cannot be empty reports one `E10011` at the type-argument span (TY-4) when it
  appears in a **nested type-argument position** (the field annotation, the explicit call type
  argument, `Array<Array<i32?>>`, `Array<Wrap<i32?>>`); the existing declaration checks stay
  unchanged; each site reports once (no duplicates); the valid nullable handle (`Array<String?>`)
  stays accepted, keys its own instance, builds, and runs.
- [ ] **R10 — The qualified type path resolves to the unqualified instance** (`DEF-385`):
  `std.mem.Array<i32>` resolves to the same generic struct instance as `Array<i32>` (MOD-13), so
  the declaration, method call, and emitted C agree; the unqualified control stays byte-identical.
- [ ] **R11 — A negated literal is judged on its folded value** (`DEF-386`): `let delta: i16 =
  -5;`, `let a: i8 = -5;`, and `let a: i8 = -128;` are accepted per the context-typing rule
  (LEX-14, Ch 04 §13) and unary minus's same-type result (Ch 04 §5), build strictly, and read
  back the value; `u8 = -5` and `i8 = -129` report one `E10202` each; the position matrix covers
  a let, a call argument, a return, a struct field, an array element, a conditional arm, and an
  assignment; the positive-literal control stays green.
- [ ] **R12 — The runtime-indexed element/member store emits in every form** (`DEF-367`,
  critical): the element/member type is resolved per instance; the plain store, compound
  assignment (`+=`), update (`++`/`--`), and deep member chains at a runtime index (local array,
  slice, borrow parameter, generic borrow) emit through the indexed element address; the runtime
  base/index is always materialized so the target is evaluated before the RHS; the bounds guard
  (`E11211`) is kept; the overwritten owner drops per O3; and the `None` fallback never silently
  drops. The constant-index twin and a runnable read-back prove the value; the generic owning
  element store is LSan-clean.
- [ ] **R13 — The conditional-base member store is rejected, including through `unsafe`**
  (`DEF-377`): a store whose target is a member/Index chain rooted in a conditional selection
  reports one `E10221` at the store target (the frozen place list, Ch 04 §4 / EX-14), including
  `(unsafe (c ? a : b)).v.x = 7` and `(unsafe (c ? a : b))[i].x = 7`; the place test recurses
  through `Unsafe` and the Index bypass is removed; the read spelling and the green controls
  (`s.f.g`, `s.arr[i]`, the raw chain `p.f.g` inside an `unsafe { }` block) stay accepted.
- [ ] **R14 — The element-read store carries its loan** (`DEF-372`): `let s: str = xs[0];` over an
  element holding a loaned `String` view carries the loan to `s`, so a later out-slot store
  reports one `E10030` at the fill (Ch 16 BR-1/§6), matching the direct element and member twins.
- [ ] **R15 — Conditional and array-literal view copies transfer their element loans as a union**
  (`DEF-373`): `let ys: str[2] = (c ? xs : zs);`, the array-literal copy (including the fill
  form), the single-view conditional, and a conditional whose arm is an array literal record each
  source element's loan as a **union** (both arms, not the last arm's record), so a later out-slot
  store reports one `E10030` at every affected fill; the mixed-position case reports at both
  fills; the copies themselves stay accepted.
- [ ] **R16 — The borrow-element conditional take stays a control** (`DEF-374`, control): the
  generic `eat(take (c ? v.name : w.name))` and its concrete twin both report two `E10026` (one
  per arm) at the same span; the count and span parity are pinned with no code change.
- [ ] **R17 — The owning raw-deref source is materialized once in every consuming position**
  (`DEF-375`): one shared materializer (the take-path predicate: `owns_drop || maybe_owns ||
  unknown`) is used by the store, the return, and the aggregate/struct-literal-field paths, so an
  impure generic pointer expression is evaluated once and exactly the pointee it read is emptied
  (Ch 04 §3, Ch 17 §5); the store, return, and field probes are ASan/LSan-clean; a concrete
  duplicable pointee is the control.
- [ ] **R18 — The guarded take carries its consumed source and its pointer fill** (`DEF-376`): a
  `take` of a non-place raw-deref source inside a `&&` right operand empties the pointee on the
  evaluated path, and the pointer fill is deferred on the record rather than emitted at statement
  entry (Ch 04 §3/§5, Ch 03 O6, Ch 17 §5), so the accepted program is ASan/LSan-clean for both
  guard selections and an impure pointer is not called on the false path.
- [ ] **R19 — Every literal position materializes the hidden view-array destination** (`DEF-378`):
  a call-valued view-array field materializes its hidden destination at the read, unguarded,
  nested, and guarded literal positions; the **unguarded** builder consults the hidden-destination
  predicate (Ch 04 §3/§15), so the accepted programs build strictly and run the literal's values.
- [ ] **R20 — The owning nested conditional's argument carrier and fill stay in its guard path**
  (`DEF-379`): the by-value argument carrier is deferred into the `Materialize` channel under
  `defer_move_outs`, and its fill runs only when the guard selects it (Ch 04 §3/§5), so the
  skipped path runs no call and the run matches the expected values; the guard drop/re-fill
  analysis covers a re-executed guard.

### Won't Have (Out of Scope)

- `DEF-167` (a call-shaped comparison emits a call) and `DEF-171` (an accepted `embed` outside a
  constant context) — the two older rows parked for user rulings (AR-1).
- The minors: the remaining backlog after this batch is 77 minor rows (the dispatch's "79" is
  2 parked majors + 77 minors); they form the batch after this one (AR-1).
- Any new diagnostic code, registry row, specification chapter, or golden (AR-29); a row that
  turns out to need one is parked mid-phase with the evidence — except `DEF-367`, whose only
  fallback is a phase blocker and user ruling (AR-34).
- A store route for a conditional selection (`DEF-377`'s class) — the frozen place list is closed
  (AR-7); the dropped `(*p).f` member store is a residual row filed during execution; a new ABI
  key scheme for nullable type arguments (`DEF-384`'s ABI half is already green, AR-14); a
  cross-module last-segment type-name identity change (`DEF-385`'s residual limitation, AR-15).
- A new test-suite pair: every case has an origin home; a case with no home needs a register
  entry and user approval (AR-26, AR-35).

## Technical Requirements

### Compatibility

- The host, hosted-amiga, and freestanding profiles stay conformant; every accepted program
  keeps building under `-std=c99 -Wall -Wextra -Werror` on the pinned container.
- Byte identity wherever a phase does not change the shape (the emission differential classifies
  every delta); the pinned origin suites and the batch-I/J/K/L oracles stay green.

### Security

- Not applicable beyond the compiler's own input validation (unchanged).

## Scope Decisions

| Decision | Options Considered | Chosen | Rationale | AR Ref |
| -------- | ------------------ | ------ | --------- | ------ |
| Batch scope | 20 remaining rows / parked majors / minors | the 20 rows, critical first | Scope is severity-first (the critical planned first); execution is dependency-ordered (AR-36) | AR-1 |
| Phase grouping | seven machinery phases / one per row / origin-batch phases | seven machinery phases | Coherent review surfaces; dependencies P1 → P2, P1 → P3, P2 → P5, P6 → P7; P4 independent and fourth | AR-4, AR-36 |
| `DEF-361` / `DEF-374` | red cases / controls | pinned controls | The probe shows the filed failure fixed; no residual in another row's scope | AR-3, AR-31, AR-32 |
| `DEF-367` | emit the indexed element/member store / reject `E10221` | emit every target form | The frozen place list (Ch 04 §4 / EX-14) includes a struct field and an array element; rejection is non-conformant; no park fallback | AR-6, AR-34 |
| `DEF-377` | emit the per-arm store / reject `E10221` | reject | The frozen place list is closed; a conditional selection is not a place (the batch-L `DEF-322` precedent); `Unsafe` recurses | AR-7 |
| `DEF-384` | reject the invalid scalar + key nullable / key only | reject at nested type-argument positions; keep the nullable-handle control | TY-4 forbids `i32?`; the existing declaration checks stay; `Array<String?>` already keys and runs | AR-14 |
| `DEF-386` | context-type the operand + folding fit / keep the rejection | folded negated value | LEX-14 and Ch 04 §13/§5; `i8 = -128` fits, `u8 = -5`/`i8 = -129` do not | AR-16 |
| Test placement | origin suites / new pair | origin suites, additions only | The machinery is the origin batches'; no new pair without approval; spec cases are never re-pinned | AR-26, AR-35 |

> **Traceability:** Every scope decision references the Ambiguity Register entry that resolved
> it. See `00-ambiguity-register.md`.

## Acceptance Criteria

1. [ ] Every one of the 18 red rows is closed with a reproduction that flips (red before, green
   after) and a suite pin that fails without the fix; `DEF-361` and `DEF-374` stay green as
   pinned controls (AR-3, AR-5).
2. [ ] Every ownership/handle fix is ASan/LSan-clean on the host (plain and `RDC_ASAN=1`), with
   both guard selections and loop/early-exit variants where the shape is re-executed; every
   store-dropping fix reads the stored value back and counts the side effect (`DEF-367`, `DEF-379`);
   the `DEF-367` generic owning store is LSan-clean and its out-of-range index traps `E11211`.
3. [ ] The strict build passes for every accepted shape in scope on the pinned container; the
   hosted-amiga and freestanding legs stay green for their rows (AR-28); the freestanding
   exclusions (`DEF-368`, `384`, `385`) are recorded with the `std`-module rationale.
4. [ ] No behavior change outside the in-scope rows; the origin suites, the frozen spec suites,
   and the batch-H/I/J/K/L oracles stay green.
5. [ ] `make check` green on every phase; `make verify` green at close-out.
6. [ ] The roadmap's 20 rows are accounted for: 18 red rows Done with per-row evidence, the 2
   controls Done with their control-pin evidence; the two parked majors stay open; the remaining
   minors are counted by audit (AR-38); every row filed during execution is recorded with its
   reproduction/evidence (including the likely incidentals `DEF-380` and `DEF-382`).
7. [ ] No spec, registry, or golden changes; the new cases append to the origin suites only
   (AR-26); only `_impl_test.rs` byte-identity pins are ever re-pinned and only with differential
   evidence (AR-35); the `AGENTS.md` test map updates at close-out only if its wording needs the
   batch mention.
