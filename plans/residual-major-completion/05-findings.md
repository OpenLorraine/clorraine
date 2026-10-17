# Findings Ledger: Residual Major Completion (Batch M)

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)
> **Status**: Scaffold — each phase review appends its verdicts, fix rounds, and filed rows here.
> The batch-L ledger (`plans/residual-major-closeout-ii/05-findings.md`) is the format and method
> reference.

## How to use this ledger

- One section per phase. The phase review appends: the reviewed revisions (the fix tree and the
  red-cases baseline), the dispatched lenses, the verdict table (finding ID, severity, area,
  evidence), the fix-round table, and the confirmations.
- Findings IDs follow the phase's lens initials (`RV`/`P<n>R` correctness, `SA`/`M` memory,
  `EI`/`EM` emission, `S`/`PR` semantics), with the phase-prefixed confirmations (`P<n>C`).
- In-round fixes are recorded as fixed-and-confirmed; anything left open is filed as a new
  `DEF-` row in `plans/00-roadmap.md` with a reproduction and base/head evidence (the batch
  convention).
- The close-out section records the final verification, the row accounting, and the code record.
- The two pinned controls (`DEF-361`, `DEF-374`) record their probe-verified state in their
  phases; a close-out probe that shows a regression turns the row red there (AR-31, AR-32).

## Row Ledger (empty result cells — the execution fills them)

| Row | Severity | Verdict at `86d6f51` | Phase | Post-fix expectation | Phase result |
|-----|----------|----------------------|-------|----------------------|--------------|
| `DEF-360` | 🟠 major | Red | P1 | One `E10200` at the dot-call written argument (call-site inference); the generic caller runs | _(to be filled)_ |
| `DEF-361` | 🟠 major | Control | P1 | Six direct-name fixtures stay green (run 0); module-own removed | _(to be filled)_ |
| `DEF-363` | 🟠 major | Red | P1 | One `E11103` per declaration/import collision (func/struct/const matrix); renamed control runs | _(to be filled)_ |
| `DEF-390` | 🟠 major | Red | P1 | Qualified call uses the module's symbol and modes/params/return; `m.hook()` control; mismatch keeps `E10200` | _(to be filled)_ |
| `DEF-364` | 🟠 major | Red | P2 | `E10200` at the else arm / `E10012` at the null, in direct let/return/field/arg contexts too; controls run | _(to be filled)_ |
| `DEF-365` | 🟠 major | Red | P2 | `cl_id__u8`, no `#`; build/run on host and m68k profiles | _(to be filled)_ |
| `DEF-368` | 🟠 major | Red | P2 | Concrete twin's `cl_string_view`; build/run 0 | _(to be filled)_ |
| `DEF-371` | 🟠 major | Red | P3 | Literal typed by the concrete parameter; build/run 0 | _(to be filled)_ |
| `DEF-384` | 🟠 major | Red | P3 | One `E10011` at each nested type-argument site and the explicit call type argument; five sites in the count case, no duplicates; nullable handle control runs | _(to be filled)_ |
| `DEF-385` | 🟠 major | Red | P3 | Qualified type path emits the unqualified instance; run 0 | _(to be filled)_ |
| `DEF-386` | 🟠 major | Red | P3 | Folded fit: `-5`/`i8 -128` accept; `u8 -5`/`i8 -129` report `E10202`; `f32`/`bool` stay `E10200` | _(to be filled)_ |
| `DEF-367` | 🔴 critical | Red | P4 | Plain/compound/update/chain/nested/multi-index/order stores emit; per-instance owning store LSan-clean; bounds trap kept | _(to be filled)_ |
| `DEF-377` | 🟠 major | Red | P4 | Five conditional-rooted forms and unsafe-wrapped conditional forms each `E10221`; raw `unsafe p.f` prefixes accepted with a store-emitting read-back; block controls accepted; `(*p).f`/`unsafe (*p).f` residual probed | _(to be filled)_ |
| `DEF-372` | 🟠 major | Red | P5 | One `E10030` at the fill; direct/member twins keep theirs | _(to be filled)_ |
| `DEF-373` | 🟠 major | Red | P5 | One `E10030` per affected fill; the mixed-position union reports both; in-scope copies accepted | _(to be filled)_ |
| `DEF-374` | 🟠 major | Control | P5 | Two `E10026` per twin at the same span | _(to be filled)_ |
| `DEF-375` | 🟠 major | Red | P6 | Store/return/field/array/conditional each call `alt` once and move the pointee; ASan/LSan-clean | _(to be filled)_ |
| `DEF-376` | 🟠 major | Red | P6 | Both selections ASan-clean; the false path does not call the impure pointer | _(to be filled)_ |
| `DEF-378` | 🟠 major | Red | P7 | Read/unguarded/nested/guarded positions build and run 0; one call each | _(to be filled)_ |
| `DEF-379` | 🟠 major | Red | P7 | Calls = 2 (single-pass) / 4 (loop); run 0; carrier fill inside the guard path; eager control run 0; ASan-clean | _(to be filled)_ |

## Phase 1 — Call identity and written-argument checks

**Reviewed fix**: _(recorded at review time — the fix commit and the red-cases baseline)_.
**Lenses**: correctness (RV), semantics (S), memory-safety (SA), and emission-integrity (EI),
dispatched in parallel on the phase diff (AR-27; the memory lens covers the symbol/IR change).

### Verdicts

_(to be filled by the review: one row per lens with its result and evidence)_

### Findings and rulings

_(to be filled: finding ID, severity, area, ruling; include the `DEF-361` control evidence and
the `DEF-390` symbol-set blast radius)_

### Filed rows

_(to be filled: new `DEF-` rows, or "none")_

## Phase 2 — Substitution edges I: conditional and element keys

**Reviewed fix**: _(recorded at review time)_. **Lenses**: correctness, semantics, memory-safety,
and emission-integrity.

### Verdicts

_(to be filled; the conditional report counts/spans and the keyed-symbol set are recorded here)_

### Findings and rulings

_(to be filled)_

### Filed rows

_(to be filled)_

## Phase 3 — Substitution edges II: context typing

**Reviewed fix**: _(recorded at review time)_. **Lenses**: correctness, semantics, and
emission-integrity (AR-27).

### Verdicts

_(to be filled; the literal/type-argument spans and the qualified-instance emission are recorded
here)_

### Findings and rulings

_(to be filled)_

### Filed rows

_(to be filled)_

## Phase 4 — Dropped store targets

**Reviewed fix**: _(recorded at review time)_. **Lenses**: correctness, semantics, memory-safety,
and emission-integrity (AR-27; semantics owns the place rule and the store target).

### Verdicts

_(to be filled; the `DEF-367` store shape, per-instance owning store, RHS count, bounds guard,
target order, the five plus unsafe-wrapped `DEF-377` rejection spans, and the accepted raw-prefix read-backs are recorded here)_

### Findings and rulings

_(to be filled)_

### Filed rows

_(to be filled: new `DEF-` rows, or "none"; the dropped stores through `(*p).f` and
`unsafe (*p).f` are known candidates for this phase — probe both and record their reproduction
and evidence before filing)_

## Phase 5 — Element and store loans

**Reviewed fix**: _(recorded at review time)_. **Lenses**: correctness, memory-safety, and
semantics (AR-27; no emission lens for P5).

### Verdicts

_(to be filled; the loan matrix, the `E10030` counts/spans, and the `DEF-374` control parity are
recorded here)_

### Findings and rulings

_(to be filled)_

### Filed rows

_(to be filled)_

## Phase 6 — Deref and take channels

**Reviewed fix**: _(recorded at review time)_. **Lenses**: correctness, semantics, memory-safety,
and emission-integrity.

### Verdicts

_(to be filled; the single-call emission and the guard-path emptying are recorded here)_

### Findings and rulings

_(to be filled)_

### Filed rows

_(to be filled)_

## Phase 7 — Literal destinations and guards

**Reviewed fix**: _(recorded at review time)_. **Lenses**: correctness, semantics, memory-safety,
and emission-integrity.

### Verdicts

_(to be filled; the destination form, the call counts, and the guard-path evaluation are recorded
here)_

### Findings and rulings

_(to be filled)_

### Filed rows

_(to be filled)_

## Close-out

**Verification**: _(to be filled: `make check`, `make verify`, the focused ASan legs, the strict
sweep, the per-phase emission differentials)_. **Row accounting**: _(to be filled: 18 red rows
Done, 2 controls Done, the two parked majors open, the remaining minors counted by audit; every
in-execution filed row checked, including `DEF-380`'s gridel boundary and `DEF-382`'s open
status — recorded incidental vs open, AR-38)_. **Code record**: _(to be filled: the phase commits and the
final tree)_. **Residuals**: _(to be filled: any filed row or recorded limitation, including the
`(*p).f`/`unsafe (*p).f` stores if they remain)_.

## Phase 1 — Call identity and written-argument checks

**Reviewed implementation**: `a0c4f26` (the implementation), `6e281cd` (the pins), `dec7695` (the
corrections). **Lenses**: correctness (L1C), semantics (L1S), memory-safety (M), emission-integrity.

### Verdicts

| Lens | Result | Evidence |
|------|--------|----------|
| correctness | FAIL — 2 blockers | L1C-1 (a self-qualified inferred generic call emits an instance the monomorphizer never declares), L1C-2 (a module-path receiver dot call emits the dotted path as the C callee); L1C-3/L1C-4 minors (the root scratch files, the register escapes) |
| semantics | PASS WITH MINORS — 0 blockers | L1S-1 (the pending-conversion report at the whole argument, not the written value), L1S-2 (the collision test comment claimed `E90001`); L1S-3..L1S-7 filed (`DEF-396`, `DEF-397`, `DEF-398`, `DEF-404`, `DEF-405`) |
| memory-safety | FINDINGS — 2 | M-1 (a qualified `take` ffi argument released twice in a module initializer), M-2 (the qualified ffi call emitted the dotted path); both phase-introduced |
| emission-integrity | CONFIRMED — 0 | 22 deltas in the four intended classes; no head-only strict failure; the qualified symbol/prototype agreement and re-emission determinism clean; the pre-existing generic-return `ClT` filed (`DEF-395`) |

### Fix rounds

| Round | Outcome |
|-------|---------|
| C1 (`dec7695`) | **Fixed**: the ffi declarations keyed by `(module, name)` and the declared symbol spelled; a self-qualified ffi call keeps its modes and take contract (the initializer passes the temp, no release) |
| C2 (`dec7695`) | **Fixed**: the dotted IR callee resolved module-aware in the instance collectors; the inferred generic instance is declared, defined, and called once (body forms; the initializer form stays `DEF-402`) |
| C3 (`dec7695`) | **Fixed**: member-ness decided from the whole receiver path and the initializer receiver resolved module-aware; `DEF-403` incidentally closed |
| C4 (`dec7695`) | **Fixed**: a marked dot-call argument reports at its written value |
| C5/C6 (`dec7695`) | The collision comment corrected; the register em dashes fixed; the root scratch files removed |

### Confirmations

| Lens | Result |
|------|--------|
| memory-safety | **PASS — 0 blockers**: both ffi findings fixed on the unmodified head; the 51-TU coverage set ASan/LSan-clean (the conditional variants included); no new leak/UAF/invalid free |
| correctness | **PASS WITH MINORS — 0 blockers**: L1C-1/L1C-2 closed (body and initializer), the semantics minors closed, the regression matrix clean; the `W10001` on a duplicable borrow receiver is the frozen warning; the pins discriminate (4 spec + 4 impl; the modes control declared) |

### Filed rows

`DEF-395` (the generic-return `ClT`), `DEF-396` (the qualified call's checker bypass), `DEF-397` (the
generic dot-call inference/bounds), `DEF-398` (the qualified function value), `DEF-399` (the
qualified intrinsic/ffi `E10219`), `DEF-400` (the imported-value head), `DEF-401` (the dot-call
explicit type arguments), `DEF-402` (the initializer inferred generic — open), `DEF-403` (the
initializer module-global receiver dot call — **closed incidentally**), `DEF-404` (the `cstring`
literal), `DEF-405` (the explicit-type-argument literal), `DEF-406` (the compound callee parameter
with a same-named own type parameter), `DEF-407` (the qualified generic fixed-array loop iterable).

### Record notes

- AR-39 records the batch-L origin-case input correction (the import renamed to `other`; the
  initializer-keying subject preserved; the collision pinned by the new `E11103` case).
- A `W10001` on a generic dot call whose receiver is a duplicable `borrow` value is the frozen
  needless-lend warning, emitted at both refs.
- The initializer form of the inferred generic call remains `DEF-402` (the initializer AST collector
  returns `Unknown` for a module-path receiver).
- Two same-named ffi declarations with different C signatures cannot share one TU (a C constraint,
  out of scope; the keyed lookup is structurally verified).

## Phase 2 — Substitution edges I: conditional and element keys

**Reviewed implementation**: `f96303f` (the implementation), `38ea969` (the pins), `b50cc2f` and
`731cbc7` (the corrections). **Lenses**: correctness (L2C), semantics (L2S), memory (MS),
emission-integrity (F).

### Verdicts

| Lens | Result | Evidence |
|------|--------|----------|
| correctness | FAIL — 2 blockers | L2C-1 (the substituted nullable member widening lost the guard and dereferenced null; the member-null rule missing), L2C-2 (the null rule once per instance, not per site); L2C-3/5/6 minors (the unpinned literal-arm context typing, the freestanding pin, the duplicated rule) |
| semantics | FAIL — 3 blockers | L2S-3 (= L2C-1), L2S-1 (= L2C-2), L2S-2 (a nested conditional's false null arm at the inner conditional's span); L2S-4..6 pre-existing (the argument/arm widening, the concrete no-destination null arm, the `cstring` index-surface note) |
| memory-safety | FINDINGS — 1 critical | MS-1 (= L2C-1); plus the dotted (`Qualified`) root missing the `E10010` minor and the uncontracted-FFI view-crossing regression |
| emission-integrity | FINDINGS — 1 major | F1 (= L2C-1); the intended delta classes otherwise clean |

### Fix rounds

| Round | Outcome |
|-------|---------|
| C1 (`b50cc2f`) | **Fixed**: the resolved source type drives the widening guard (the nullable member emits the concrete twin's guard); the member-null rule per instance (`E10010`); the null report stops after the site's first offending instance; `unify_numeric` checks the unknown before the parameter (the nested/unannotated false null arm); the literal/array-literal arm context typing pinned (AR-42); the freestanding element pin; the shared conditional-result/null-report helpers |
| C7 (`731cbc7`) | **Fixed**: the dotted (`Qualified`) member root feeds the `E10010`; the uncontracted-FFI view crossing resolves the argument type (`(APTR)(...).ptr` restored); the nullable mixed conditional arm reports the concrete twin's one `E10200`; the argument/both-arm/both-null pins added |

### Confirmations

| Lens | Result |
|------|--------|
| memory-safety | **PASS WITH MINORS — 0 blockers**: MS-1 fixed (the guarded line byte-identical to the concrete twin; the base SEGVs; the full coverage clean); the dotted-root and FFI-crossing items found and fixed |
| correctness | **PASS WITH MINORS — 0 blockers**: all findings closed; the incidental fixes noted (`DEF-408`/`DEF-409`) |
| final (fresh) | **CLEAN — 0 findings**: F1–F4 verified; the 54-probe matrix and the corpus show only the intended deltas; no head-only strict failure |

### Filed rows

`DEF-408` (the argument widening — **closed** by the correction), `DEF-409` (the conditional-arm
widening — **closed** by the correction), `DEF-410`–`DEF-414` open, `DEF-415` filed (the same-arm
nullable conditional zero view).

### Record notes

- AR-40 (the two test-source span corrections; the mechanical evidence recorded, the user veto
  available in the batch summary), AR-41 (the `cstring` index-surface spec-text note), AR-42 (the
  literal/array-literal arm context typing).
- The correction round incidentally fixed `DEF-408` and `DEF-409`; both are closed with the
  confirmation evidence.
- `DEF-414`'s reproducer was not retained; the row stays open with the note.

## Phase 3 — Substitution edges II: context typing

**Reviewed implementation**: `ffae71f` (the implementation), `c6f5dc4` (the pins), `fef7cbe` (the
corrections). **Lenses**: correctness (L3C), semantics (L3S), emission-integrity.

### Verdicts

| Lens | Result | Evidence |
|------|--------|----------|
| correctness | PASS WITH MINORS — 0 blockers | L3C-001 (the register's trailing blank line); P-1..P-3 pre-existing (the dot-call explicit type arguments, the literal-context siblings, the TY-4 declaration positions) |
| semantics | FAIL — 1 blocker | L3S-001 (the literal-context rule was integer-only: a float at an `f32` concrete parameter and a `static str` at a `cstring` concrete parameter reported `E10200` while the concrete twin accepted); L3S-002 (a value-headed qualified type path resolved by its last segment); L3S-003..L3S-007 pre-existing filed |
| emission-integrity | CONFIRMED — 0 | The four intended delta classes; no head-only strict failure (two base failures fixed); the qualified type byte-identical to the unqualified control; determinism clean; the qualified generic callee's type arguments noted (extends `DEF-396`) |

### Fix rounds

| Round | Outcome |
|-------|---------|
| C1 (`fef7cbe`) | **Fixed**: every literal at a concrete generic parameter goes through the non-generic check — a float takes the `f32` context, a static-str literal takes the `cstring` allowance, an integer at `f32`/`bool` stays `E10200`; a `judged` flag stops the later conversion from repeating; the qualified type name requires a real module prefix (a value/function-headed path reports `E10219`); the register's trailing blank line removed |

### Confirmations

| Lens | Result |
|------|--------|
| semantics | **PASS — 0 blockers**: L3S-001/L3S-002 closed; the regression matrix (the literal contexts, the TY-4 five-site count and the nested/legal instances, the qualified type byte identity, the folded matrix and boundaries, the controls) shows only the intended deltas; the pre-existing rows confirmed open |

### Filed rows

`DEF-416` (the cross-module simple-name type collision), `DEF-417` (the TY-4 declaration
positions), `DEF-418` (the explicit type arguments on a non-generic callee), `DEF-419` (the
import-renamed type names), `DEF-420` (the negated float at `f32`); the extensions to `DEF-396`
(the qualified generic callee's type arguments), `DEF-401` (the dot-call explicit type arguments),
and `DEF-405` (the explicit/inferred type-argument literal context).

### Record notes

- AR-43 corrects the nullable annotation spec span to the argument's own start (the AR-40
  mechanical-span precedent).
- The emission lens's P-1 (the qualified generic callee's type arguments) extends `DEF-396`; P-2
  (the `m378_*` hidden out-parameter probes) is `DEF-378`'s phase; P-3 is the parked `embed`.
- A negated float literal at `f32` reports `E10200` on both the generic and concrete spellings
  (pre-existing parity, `DEF-420`).

## Phase 4 — Dropped store targets

**Reviewed implementation**: `43a3a35` (the implementation), `4371374` (the pins), and the correction
rounds (`26f27d6`, `af04306`, `2d2e17a`). **Lenses**: correctness (L4C), semantics (L4S),
memory-safety (MS), emission-integrity (P4).

### Verdicts

| Lens | Result | Evidence |
|------|--------|----------|
| correctness | PASS WITH MINORS — 0 blockers | L4C-1 (the fixed-array row-store pin); the P-1..P-6 pre-existing notes; the distinct-arm double free (L4C2-1) was the R1 correction's finding |
| semantics | FAIL — 3 blockers | L4S-1 (the generic RHS indexed-place fold: the stored value read element 0), L4S-2 (the raw/view conditional base rejected although the implied deref is admitted), L4S-3 (the generic compound/update signed arithmetic); L4S-4/5 minors; L4S-6 (the owned-move RHS pin) in the re-confirmation |
| memory-safety | FINDINGS — 2 criticals | MS-1 (the new member store freed the overwritten owner before a view/alias RHS read it), MS-2 (the conditional self-arm over a runtime index nulled the target); the R1 distinct-arm double free in the re-confirmation |
| emission-integrity | CONFIRMED — 0 head-introduced | The three intended delta classes; the re-audit then found the R1 distinct-arm (fixed) and the self-slot crash (F3) |

### Fix rounds

| Round | Outcome |
|-------|---------|
| C1 (`26f27d6`) | **Fixed**: the overwritten-owner save until the RHS view/alias has read it (including the ArrayLit fill and the raw-deref path); the pinned runtime-index source place; the conditional self-arm storage-address compare; the generic compound/update wrap casts; the `Unsafe` capture root (`E10402`); the raw-deref member out-slot addressing; the conditional-value-base rejection control; the bare-`T` row pin |
| R1 (`af04306`) | **Fixed**: the distinct fixed-array arm's source move-out; the duplicable RHS-index pins; the slice-view control and the deref read-back; the self-slot address guard (the member and whole-element twins) |
| F3/P4F-2 (`2d2e17a`) | **Fixed**: the identical whole-element self-copy is skipped entirely (no tautological compare, no save/copy/empty) |

### Confirmations

| Lens | Result |
|------|--------|
| memory-safety | PASS WITH MINORS (the two criticals + the five shared-root rows fixed), then FAIL (the R1 distinct-arm) → fixed |
| correctness | FAIL (the R1 distinct-arm) → fixed; the final confirmation confirmed 7/7 fixed shapes |
| semantics | FAIL (L4S-6) → the duplicable pins and the F2 controls |
| emission-integrity | FINDINGS (the R1 distinct-arm + the self-slot) → fixed |
| final (`P4F-2`) | **CLEAN — 0 findings** |

### Filed rows

`DEF-421` (the deref-rooted member store dropped), `DEF-422` (the standalone raw-prefix store
`E10600`), `DEF-423` (the whole-element store order; **extended** with `at1`/`at2`), `DEF-424` (the
ArrayLit save/clear before the RHS; extended with the constant-index self-copy), `DEF-425` (the
generic owned element move through a borrow parameter), `DEF-426` (the generic `/=`/`%=` guard),
`DEF-427` (the `len` of a runtime-indexed element folds to zero).

### Record notes

- AR-44 records the conditional-value base decision: the syntactic closed-list reading rejects
  `(c?p:q).v.x` / `(c?a:b)[i].y` (one `E10221`), the explicit `*(c?p:q)` deref spelling is accepted
  with a read-back, and no per-arm selecting route is added; the final confirmation closed the
  deferred-route question.
- The conditional self-arm now compares storage addresses (ignoring the pinned-index spelling); the
  distinct fixed-array arm empties its source; the identical whole-element self-copy is skipped.
- The `(*p).f` / `unsafe (*p).f` dropped-store residual remains `DEF-421`; the constant-index
  whole-element self-copy is added to `DEF-424`'s evidence.

## Phase 5 — Element and store loans

**Reviewed implementation**: `97db81d` (the implementation), `9f58b20` (the pins), `3907b3b` and
`9f74e60` (the corrections). **Lenses**: correctness (L5C), semantics (L5S), memory-safety.

### Verdicts

| Lens | Result | Evidence |
|------|--------|----------|
| correctness | FAIL — 2 blockers | L5C-1 (the fill recorded at the aggregate path: the unused and overwritten fills were newly rejected), L5C-2 (a fill-covered element copy lost the loan — an accepted UAF); L5C-3 (the write-conflict reading unpinned) |
| semantics | PASS WITH MINORS — 0 blockers | L5S-1 (the post-conflict subsumption unpinned), L5S-2 (the runtime-index copy unpinned), L5S-3 filed; the write-conflict drop is oracle-conformant (the two batch-L controls pin one `E10031`) |
| memory-safety | CONFIRMED — 0 in-diff | All ten red shapes reject with no TU; every accepted control ASan/LSan-clean; 18 newly covered escapes now reject; no base-reject→head-accept flip; three pre-existing UAFs filed (`DEF-428`, `DEF-429`, `DEF-432`) |

### Fix rounds

| Round | Outcome |
|-------|---------|
| C1 (`3907b3b`) | **Fixed**: the fill expands over the destination's known fixed extent into per-element records (the unused/overwritten fills restored to base acceptance; the fill-covered read transfers its loan); the write-conflict readings pinned (one `E10031` direct; `E10031`+`E10033` global); the runtime-index copy pinned |
| C2 (`9f74e60`) | **Fixed**: the nested fill expands at its own level (`position.iterated_element()`); the nested unused/overwritten fills restored (base-byte-identical); the nested fill-covered copies report one `E10030` |

### Confirmations

| Lens | Result |
|------|--------|
| correctness | FAIL (the nested fills) → fixed → **PASS** (the six nested flips only; base byte identity) |
| semantics | FAIL (B1, the nested fills) → the in-flight fix confirmed → **PASS** (the committed fix) |
| memory-safety | **CONFIRMED CLEAN** (0 in-diff; 3 pre-existing UAFs filed) |

### Filed rows

`DEF-428` (the exact whole-binding self-assignment erases the element loans), `DEF-429` (a composite
copy source into an element/projected destination), `DEF-430` (a runtime store followed by a
constant-read copy), `DEF-431` (a deeper projection after a shorter-path recording), `DEF-432` (a
temporary-derived element in a view-array literal), `DEF-433` (a call-mediated element-view result),
`DEF-434` (the direct-owner literal overwrite through the provenance fallback), `DEF-435` (an
element-store nested fill with a fresh member value); `DEF-309` extended (the call-fill evaluation).

### Record notes

- AR-45 records the write-conflict reading: the old element-loan positions are dead, the write's
  `E10031` stands, the binding/provenance loans stay; the direct, global, and member forms are pinned.
- `DEF-374` remains a green control: the generic and concrete conditional takes each report two
  `E10026` at the same span.
- The open-extent fill keeps one aggregate record (a conservative fallback; an unresolved extent
  cannot name its elements).

## Phase 6 — Deref and take channels

**Reviewed implementation**: `194a942` (the implementation), `7671684` (the pins), `98aedeb` (the
correction). **Lenses**: correctness (L6C), semantics (L6S), memory-safety (P), emission-integrity.

### Verdicts

| Lens | Result | Evidence |
|------|--------|----------|
| correctness | PASS WITH MINORS — 0 blockers | L6C-1 (the deferred-fill comment), L6C-2 (a projection through a raw deref — filed `DEF-437`), L6C-3 (the phase-7 carrier, verified with the composites in P7), L6C-4 (a loop-head place take — filed `DEF-444`) |
| semantics | FAIL — 1 blocker | L6S-1 (the literal sibling source order reversed by the eager field/element fill); L6S-2..L6S-5 pre-existing filed (`DEF-436`, `DEF-437`, `DEF-438`, `DEF-439`) |
| memory-safety | CONFIRMED — 0 in-diff | The five positions and both guard forms memory-correct (place and impure pointers, concrete struct/view/duplicable substitutions, both selections); five pre-existing findings filed (`DEF-436`, `DEF-437`, `DEF-440`, `DEF-441`, `DEF-442`) |
| emission-integrity | CONFIRMED — 0 in-diff | 48 deltas in the intended classes; no head-only strict failure; the profiles/determinism clean; the pre-existing residuals filed (`DEF-436`, `DEF-443`) |

### Fix rounds

| Round | Outcome |
|-------|---------|
| C1 (`98aedeb`) | **Fixed**: the struct-literal field and array-literal element defer their owning-deref fill, so the earlier siblings evaluate first (the source order); the deferred-fill comment reworded; the two-field/two-element order cases added |

### Confirmations

| Lens | Result |
|------|--------|
| semantics | **PASS — 0 blockers**: L6S-1 closed (the two-field/two-element/nested/multi-field runs all match the source order); the deferred-fill mechanics conformant; the regression matrix unchanged |

### Filed rows

`DEF-436` (the whole-element and raw-target stores bypass the materializer), `DEF-437` (the member
projection of a raw pointee), `DEF-438` (the eager take-argument order), `DEF-439` (the
`value_move_outs` gap in `arm_consumes_target`), `DEF-440` (the guarded intrinsic raw-deref pointee),
`DEF-441` (the guarded take of a member over a raw pointer), `DEF-442` (the guarded take of a
conditional of literals), `DEF-443` (the guarded struct-literal field with a conditional deref arm),
`DEF-444` (the loop-head place-deref take).

### Record notes

- The literal-position fill now travels in a `Materialize` and is emitted by the field/element store
  at its own turn; the guarded literal remains the phase-7 carrier concern (`DEF-379`).
- `b375_arr` (and an existing-array `xs = [*alt(p,q)]`) gains the pre-existing fixed-array
  `reads_target` save prologue because the deferred fill exposes the call's named-binding arguments;
  it is non-semantic (a fresh declaration's save is a no-op; the existing-array run values are
  identical and ASan/LSan-clean).
- The `value_move_outs` channel carries the consumed non-place source in both guard forms; the
  `move_outs` place channel is unchanged.

## Phase 7 — Literal destinations and guards

**Reviewed implementation**: `581e8c7` (the implementation), `8c10710` (the pins), `103cd78` and
`78a848a` (the corrections). **Lenses**: correctness (L7C), semantics (L7S), memory-safety,
emission-integrity (F1/R1).

### Verdicts

| Lens | Result | Evidence |
|------|--------|----------|
| correctness | FAIL — 2 blockers | L7C-1 (a guarded literal's owned place/deref/fixed-array field was copied without emptying — a double free), L7C-4 (the conditional-field instance, found in the re-confirmation); L7C-2 (the read-base statement temporary — a false `E10030`), L7C-3 (the literal-only grid store form) |
| semantics | FAIL — 1 blocker | L7S-1 (= L7C-1); L7S-2 (the conditional-of-literals read base, filed `DEF-445`), L7S-3 (the fill re-evaluation, `DEF-309` extended), L7S-4 (the carrier argument order, `DEF-438` extended), L7S-5 (the docs, closed) |
| memory-safety | FINDINGS — 1 | The guarded fixed-array field of caller-owned places (= L7C-1's family); five pre-existing shapes confirmed |
| emission-integrity | FINDINGS — 1 | F1 (a ternary-arm deferred carrier emitted a bare hidden-destination call); the re-audit then found R1 (the body-scope read-base element move double-freed) |

### Fix rounds

| Round | Outcome |
|-------|---------|
| C1 (`103cd78`) | **Fixed**: the guarded literal place fields move through the store form (the struct arm, the fixed-array field, the direct literal); a ternary arm's struct literal is materialized before the deferred carrier is wrapped; an owning read-base literal binds at body scope; the all-literal grid keeps the brace list; the docs describe the body-scope lifetime |
| C2 (`78a848a`) | **Fixed**: the conditional literal field moves per arm (the L7C-4 fix); an owning projection of a compiler-generated owner re-roots its move-out, so a call-filled element moved into a binding empties the source (R1) |

### Confirmations

| Lens | Result |
|------|--------|
| correctness | **PASS**: L7C-4 closed (the place, deref, mixed, loop, `\|\|`, and ternary arms); the full phase-7 matrix regressed only by the intended deltas |
| semantics | **PASS WITH MINORS**: L7S-1 closed (the nine carrier shapes and the direct/loop/take place forms clean); the rows filed |
| emission-integrity | **CONFIRMED** (F1 closed; zero head-only strict failures), then **FINDINGS — 1** (R1) → fixed → **CONFIRMED — 0** |

### Filed rows

`DEF-445` (the conditional-of-literals read base), `DEF-446` (a take conditional literal in an eager
ternary arm), `DEF-447` (the inline read-base owning literal — **annotated not reproduced**),
`DEF-448` (a take of a conditional array literal of places); `DEF-309` and `DEF-438` extended.

### Record notes

- `DEF-442` and `DEF-443` are **closed incidentally** by the phase-7 corrections (the guarded take
  of a conditional of literals; the conditional literal field).
- `DEF-447`'s filed repro does not reproduce on any ref where the program's C builds (the base and
  the phase-7 heads are all LSan-clean); the row is annotated and stays open pending a fresh repro.
- The `Materialize` extension (`temp: Option<TempId>`, `destination: Option<Place>`) is consistent
  across both guard forms; the named destination has body-scope lifetime.

## Close-out

**Verification (8.1.1):** `make check` exit 0 (inside `make verify`); **`make verify` — all legs
passed**: the hosted-amiga cross-toolchain legs, the portable corpus under `vamos`, the vbcc
build-only leg, the FS-UAE interrupt-counter and graphics outputs, and the six runtime-fault
programs each matching. The focused ASan legs: checker 194/194 and 114/114, the residue 190/190 and
171/171, the callable-release 68/68 and 36/36, and the module-initializer 65/65 and 43/43. The
strict-build sweep evidence is the batch's per-phase emission audits (no head-only strict failure in
any phase). Environment note: the FS-UAE leg required refreshing the local (untracked)
`docker/.env` XAUTHORITY cookie after the display session rotated; all other legs ran unchanged.

**Roadmap (8.1.2):** the 18 red rows and the two controls move to Done with per-row evidence;
`DEF-380` is closed incidentally (the multi-extent grid read; the `m378_gridel` probe builds strictly
and runs 0); the two parked majors (`DEF-167`, `DEF-171`) stay open; every in-execution filed row is
checked — the incidentals `DEF-391`, `DEF-394`, `DEF-403`, `DEF-408`, `DEF-409`, `DEF-442`, and
`DEF-443` are Done, and the remaining filed rows stay open. The roadmap now shows 437 rows: 310 Done
and 127 open (the next-batch backlog: 2 parked majors + 125 minors/residuals); `DEF-382` stays open
(the `DEF-371` concrete-parameter fix does not substitute explicit type arguments), and `DEF-447` is
annotated not reproduced.

**Plan (8.1.3):** the plan is marked Complete (60/60).

**Specialist (8.1.4):** the `analyze-agents` check refreshed — outcome **None**: the four
committed project agents (correctness-reviewer, semantics-reviewer, memory-safety-auditor,
emission-integrity-auditor) cover the recurring compiler areas the batch's reviews exercised, and no
new role has a unique evidence source or capability; `codeops/specialist-check.json` records the
refreshed state.
