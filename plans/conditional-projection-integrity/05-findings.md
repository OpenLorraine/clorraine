# Findings Ledger: Conditional and Projection Integrity

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)
> **Status**: Scaffold — the phase reviews append their verdicts, fix rounds, and filed rows here
> during execution.

## Phase 1 — Conditional arm ownership

**Baseline**: `0ae53eabf119f957d4b7b38083d94ce1781cbc88`. **Lenses**: correctness (RV), memory-safety (SA), both
dispatched on the phase diff in parallel.

### Verdicts

| Lens | Result | Evidence |
|------|--------|----------|
| correctness | 1 major, 1 minor | RV-001 (the nested projection-in-arm spelling missed the checker's move marking), RV-002 (a formatting artifact); the reviewer re-ran the suites and the red phase, reproduced the red base (plain 7/19, ASan 12/19, impl 6/7 pins) and traced all five hooks |
| memory-safety | 1 critical | SA-001 (a re-executed `take` of a conditional projection kept the transferred pointer in the caller's arm temporary, so the second execution double-freed it); every other probed shape was clean, including the store/return/defer/guarded arms and the two-field sibling releases |

### Findings and rulings

| Finding | Severity | Area | Ruling |
|---------|----------|------|--------|
| RV-001 | 🟠 major | `own/expr.rs` (`projection_move_arm_sources`) | **Fixed**: the arm walk now descends a `Member`/`Index` arm onto its conditional root with the segment appended, so a nested spelling (`d ? (c ? b : mk()).v : z`) records every arm's projected place; the new spec case `armnestedread` pins the `E10020` read-after-store |
| RV-002 | 🟡 minor | `own/lower.rs` | **Fixed** directly: the collapsed brace on `is_pure_value` restored to the file's formatting |
| SA-001 | 🔴 critical | `own/lower.rs` (`materialize_owned_argument`) | **Fixed**: a `take` projection of a conditional now fills one shared carrier temporary per arm (`materialize_take_conditional_projection`), and the ordinary post-call move-out empties it, so a re-executed statement never frees the transferred value twice; the new spec case `armtakeloop` runs both taken arms across two iterations under ASan |

**Verification after the fix round**: spec 21/21 plain and under `RDC_ASAN=1`; impl 7/7; `make check` exit 0
(64 test binaries); the two filed reproducers (`take` in a loop, `defer` + `take` in a loop) are ASan-clean; the
store-path emission is byte-identical to the pre-fix compiler.

### Pre-existing findings (filed, not folded into the phase's verdict)

| Row | Severity | Shape |
|-----|----------|-------|
| DEF-257 | 🟠 major | a generic destination (`let s: T = (c ? a : mk<T>()).v;` instantiated owning) keeps copy semantics and is accepted-unsound at base and head (the generic gate is deliberate) |
| DEF-258 | 🟠 major | a conditional-of-projections spellings' fresh root leaks its sibling (`let s: String = c ? a.v : mkBox().v;`) |
| DEF-259 | 🟡 minor | a struct-literal fresh arm's sibling leaks (`(c ? a : Box { ... }).v`) — the literal root is never bound |
| DEF-260 | 🟠 major | a guarded `take` of a conditional projection double-frees (`g && consume((c ? a : mkBox()).v)`, the `\|\|` and fresh-arm variants alike; byte-identical at base and head) |

### Scoped re-review of the fix round

Both lenses re-ran on the fix diff (`a447db4..33070ad`). The correctness lens closed RV-001 and RV-002 and
filed three follow-ups; the memory lens closed SA-001 for every eager `take` shape and confirmed the store
and return emission is byte-identical.

| Finding | Severity | Area | Ruling |
|---------|----------|------|--------|
| RV-101 = SA-102 | 🟠 major | the new `armtakeloop` spec case (test integrity) | **Fixed**: the case alternated arms, so neither per-arm temporary was re-entered and the base compiler passed it; it now takes the place arm twice and the fresh arm twice, which the base compiler double-frees |
| RV-102 | 🟠 major | `own/expr.rs` (`conditional_move_sources`) | **Fixed**: an arm that is a projection over a conditional and names no single place now contributes its own arms' projected places, so the place-sibling spelling marks the moved member; the new spec case `armnestedplace` pins the `E10020` read |
| RV-103 | 🟡 minor | `own/lower.rs` (a dead per-arm temporary when the carrier exists) | **Fixed** directly: the carrier is passed into the consumption so no unused temporary is allocated |
| SA-101 | 🔴 critical | a guarded `take` of a conditional projection (pre-existing, base == head) | **Filed** as DEF-260 (outside the phase's declared scope; the guarded intrinsic read is DEF-178) |
| SA-001 | 🔴 critical | the eager `take` carrier | **Closed**: every base-failing eager shape (loop, defer copy, nested, func-typed callee, discarded call, two takes, early return/panic) is ASan-clean at head |

**Targeted confirmation**: dispatched once on the follow-up fix diff (`33070ad..3af940b`) and returned
**PASS** (2026-10-12 02:03): the rewritten loop case double-frees on the pre-carrier base under both ASan
and a plain build; the place-sibling program is accepted at the base and rejected `E10020` at head; the
dead per-arm temporary is gone and the carrier is filled by both arms with the post-call move-out intact;
spec 22/22 plain and under `RDC_ASAN=1`, impl 7/7, `make check` exit 0.

## Phase 2 — Guarded receivers and loans

**Baseline**: `e6f8848c8561c161dcb39c317b3a4bef804ec973` (phase 1's final commit tree at `576b424`). **Lenses**:
correctness (RV), memory-safety (SA), semantics (PR), all dispatched on the phase diff in parallel.

### Verdicts

| Lens | Result | Evidence |
|------|--------|----------|
| correctness | no blockers; 1 minor in scope, 1 minor pre-existing | RV-201 (the widened gate also fired for a bare all-ownerless conditional), RV-202 (a fixed-array `.len` folds and never evaluates its receiver); all 39 spec and 11 impl cases verified, the red phase reproduced at the baseline (plain 37/2, ASan 32/7) |
| memory-safety | 1 critical regression; 2 major pre-existing | SA-201 (the guarded struct-literal receiver double-freed in a **loop condition**: the loop-condition walk gave the literal's field call a second owner), SA-202 (a call-wrapped conditional view escapes the loan), SA-203 (a conditional-of-literals receiver in a loop condition double-frees at base and head); every other probed shape clean |
| semantics | no findings | The shared-loan reading verified against Ch 03 §7 and Ch 16 §3 across 18 probes: named/conditional parity for same-member and sibling writes, whole-owner writes, moves, take/borrow parameters, nested chains, element arms, and the arm-less spelling; the corrected `armplace` control judged forced by the frozen rule, not a weakening |

### Findings and rulings

| Finding | Severity | Area | Ruling |
|---------|----------|------|--------|
| SA-201 | 🔴 critical | `own/loops.rs` (`materialize_owned_calls`) | **Fixed**: the materialize skip now covers a struct-literal inner, so the literal's temporary is the single owner of its fields and the loop releases them once per iteration; the new spec case `guardliteralloop` pins the shape (clean at the phase baseline, double-freeing at the phase head before this fix under both plain and ASan) |
| RV-201 | 🟡 minor | `own/lower.rs` (`conditional_root_needs_arm_fill`) | **Fixed** directly: the predicate now routes a projection chain per arm whenever its root's arms need a fill, but a *bare* conditional only when an arm borrows a place — restoring the whole-value route for the bare all-ownerless spelling |
| RV-202 | 🟡 minor | `own/call.rs`, `own/lower.rs` (pre-existing) | **Filed** as DEF-263 (spec-silent; needs a ruling on the folded receiver's evaluation) |
| SA-202 | 🟠 major | `own/place.rs` (`view_provenance`; pre-existing) | **Filed** as DEF-261 |
| SA-203 | 🟠 major | `own/loops.rs` (pre-existing) | **Filed** as DEF-262 |
| SA-301 | 🟠 major | `own/lower.rs` (the literal binder's field paths; pre-existing) | **Filed** as DEF-264 — the loop manifestation is a leak where the base double-freed, and the `if` position leaks identically at base and head, so the root cause (the literal binder never binds a field's projection root) is the pre-existing DEF-259 class; the proper fix binds the root in the literal lowering and covers every position |

**Verification after the fix round**: spec 40/40 plain and under `RDC_ASAN=1` (the new loop
regression included); impl 11/11; `make check` exit 0; the critical reproducers (the exact loop
condition, a `for`/`do-while` pair, and the `||` twin) are ASan-clean.

### Re-review of the fix round

One scoped round on the fix diff (`a00ea4d..d3ae995`).

- **Correctness: PASS** (no findings). RV-201 is closed: a bare all-ownerless conditional takes the
  whole-value route again (one `__cl_ownedN` local; the base emitted per-arm temporaries), a bare
  place-borrowing conditional keeps the per-arm route byte-identically, the projection chains
  (DEF-178/DEF-182) still fire, and the `len(cond.v)` control is byte-identical base→head. The new
  loop regression case is present and red at the phase head before the fix under both plain and ASan.
- **Memory: SA-201 closed.** The exact reproducer and the whole failing family (`for`/`do-while`,
  `||`, the chosen `?:` arm, two fields, nested literals, nested loops, a `defer` body, `break`/
  `return` exits, and a generic instantiation) are ASan-clean; the base double-freed every one.
  The audit filed **SA-301** instead: a loop-condition literal whose field projects an ownerless
  root (`s: mkBox().v`) now leaks the root's un-moved sibling (33 B over three iterations) where
  the base double-freed it; the identical field in an `if` guarded position leaks at base *and*
  head with byte-identical emission, so the root cause — the literal binder never binds a field's
  projection root — is pre-existing (the DEF-259 class) and filed as **DEF-264**. The audit's
  array-literal variant of the second-owner class is recorded in DEF-262's note.

**Phase verdict**: SA-201 fixed and verified; RV-201 fixed and verified; no open critical/major
finding in the phase's own change; the pre-existing findings are filed as DEF-261 through DEF-264.

## Phase 3 — View provenance and elision

**Baseline**: `8de8e082855c075053a70cd82dab4af59f2d08e7` (Phase 2's final commit tree at `01c2481`). **Lenses**:
correctness (RV-30x), memory-safety (SA-30x), semantics (SR-30x), all dispatched on the phase diff in
parallel.

### Verdicts

| Lens | Result | Evidence |
|------|--------|----------|
| correctness | 1 critical | RV-301 (a mixed view-array element list shared only the first element's loan, so the second owner stayed movable and its view dangled while base rejected E10025); all 45 spec and 15 impl cases verified, the red base reproduced (plain 42/45, ASan 41/45, impl 12/15), the pins checked against the base compiler |
| memory-safety | 1 critical in scope, plus pre-existing classes | SA-302 (the widened view-array store adopted a function-scoped backing for destinations that outlive the frame — a global store, a `borrow` slot, a returned struct field — turning base leaks into use-after-frees); every direct local store spelling (single, loop, defer, break/continue, multi-element, mixed, fill, conditional, nested, assignment, `const`) was leak-free and double-free-free |
| semantics | no phase findings; 3 pre-existing | The adoption and derivation readings judged forced by the pinned oracle (`deferview`, `condviewstruct`, `viewarrayretain`) and the frozen stdlib (`bytes`' intrinsic accessor chain); PRE-1/2/3 filed (see DEF-267 – DEF-269) |

### Findings and rulings

| Finding | Severity | Area | Ruling |
|---------|----------|------|--------|
| RV-301 | 🔴 critical | `own/statements.rs` (`apply_binding_effect`), `own/place.rs` (`view_loans`) | **Fixed**: a view-array element list records one loan per element owner (the shared-source shortcut dropped every owner after the first); `viewarraymixed` pins the mixed list, and the second fix round restored the union for the reborrowing call path (`viewarraycall`) |
| SA-302 | 🔴 critical | `own/expr.rs`, `own/statements.rs` | **Fixed**: the view-array adoption runs only for a whole-slot store into a genuine frame local (no globals, no borrowed slots, no place-overridden element aliases); a struct-literal field value that is a loan reports `E10033`, so the returned-aggregate spelling rejects instead of dangling |
| RV-302 = SR-002 | 🟠 major | `own/expr.rs` (the destination guard) | **Fixed** in the second round: a projected destination under a frame local (`h.names = [mk().v]`) adopts again (the containment case was dropped and leaked), while a place-overridden alias keeps its storage in place |
| SA-303 | 🔴 critical | `own/expr.rs` (the destination guard) | **Fixed** in the second round: a `for (borrow e in xs)` element alias is not a frame-local destination, so it never adopts a per-iteration owner whose storage the iterable outlives; `viewarrayalias` pins it |
| SR-301 | 🔴 critical | `own/place.rs` (`view_loans`), `own/statements.rs` (`prepare_binding_effect`) | **Fixed** in the second round: the array union stays for the reborrowing call path while the binding effect takes the per-owner path, so a call-wrapped mixed list carries every element owner's loan; `viewarraycall` pins it |
| RV-303 | 🟡 minor | the second fix round (test integrity) | **Fixed** directly: the three behaviors of the second round are pinned (`viewarraycall`, `viewarrayproj`, `viewarrayalias`) |
| RV-304 | 🟡 minor | `own/expr.rs` (a rejected struct field reports `E10033` plus the pre-existing `E10025`) | **Accepted** (report-only): the program is rejected either way; the duplicated diagnostic is cosmetic and a diagnostic-hygiene pass can dedupe it |

### Fix rounds and confirmation

The first fix round (`954687a`) closed RV-301 and SA-302. The scoped re-review on that fix diff found
the guard's over-reach (RV-302 = SR-002), the alias adoption (SA-303), and the call-path loan loss
(SR-301); the second round (`50c5ebd`) fixed all three, and the pins commit (`2425d02`) added the three
regression cases. The targeted confirmation on the second fix diff returned **PASS** on all five items
(the mixed call reborrow, the projected adoption clean, the alias base-equivalence, the first-round
regression sweep, the suites) and filed RV-303, fixed directly.

**Verification after the fix rounds**: spec 50/50 plain and under `RDC_ASAN=1`; impl 15/15;
`make check` exit 0 (64 binaries).

### Pre-existing findings (filed as DEF-265 – DEF-269)

| Row | Severity | Shape |
|-----|----------|-------|
| DEF-265 | 🟠 major | A view stored into a destination that outlives the storing frame (a global, a borrowed slot, a projected target) keeps the derived element in place: the single-view spelling reads freed storage (base == head), the array spelling leaks (base == head after the phase's destination restriction) |
| DEF-266 | 🟠 major | View-array literals in nested positions (a call argument, a struct-literal field) leak their temporary elements (base == head) |
| DEF-267 | 🟡 minor | The `str[N]` return spelling treats a place/handle-derived element as a move (a false `E10034` plus a new `E10036`) where the single-view twin stays accepted (base == head) |
| DEF-268 | 🟠 major | A conditional view return whose arms disagree is accepted and emits C that fails a strict build (base == head, byte-identical) |
| DEF-269 | 🟡 minor | A view-array store to a global struct field (and a capturing lambda over a `str[1]`) emits invalid C at base and head |

## Phase 4 — Statement-path discards

**Baseline**: `66026bfdbe2ddc72b95135d261927e3a4c56a5ca` (Phase 3's final commit tree at `a51c508`). **Lenses**:
correctness, memory-safety, emission-integrity, all dispatched on the phase diff in parallel; the one
re-review round and the targeted confirmations were scoped to the fix diffs.

### Verdicts

| Lens | Result | Evidence |
|------|--------|----------|
| correctness | 1 critical (introduced by the fix round), 2 minor | RV-401 (the first gate missed aliasing struct-literal fields), RV-404 (the intrinsic route is wider than the named dot-call), RV-405 (a duplicated purity doc line); every spec and impl case verified, the red base reproduced (plain 53/54, ASan 51/54, impl 17/21) |
| memory-safety | 1 critical (the same RV-401 independently), no other in-scope | the four direct reproducers and the clean-shape matrix (closure, conditional, dot-call, path edges) verified; the aliasing struct arm double-freed at head where base was clean |
| emission-integrity | 1 critical (the same class, reached through a nested aggregate), 1 minor, 2 pre-existing | EA-401 = the nested aliasing spelling; EA-402 (M1: nested value-position conditionals also materialize — benign, accepted); the corpus is byte-identical base vs head; every unit builds strictly |

### Findings and rulings

| Finding | Severity | Area | Ruling |
|---------|----------|------|--------|
| RV-401 = SA-401 | 🔴 critical | `own/lower.rs` (`defer_discarded_arm`) | **Fixed** (`cea27ac`): a struct-literal arm materializes only when every owning field is a fresh result; an aliasing field keeps the literal inline |
| RV-402 = EA-401 | 🔴 critical | `own/lower.rs` (`field_aliases_owned_storage`) | **Fixed** (`5481108`): the aliasing predicate descends nested struct/array-literal fields; the regression case gains the nested and array-field programs |
| RV-403 | 🔴 critical | `own/lower.rs` (the conditional arm) | **Fixed** (`deb6cef`, pinned by `04f6ca4`): a conditional field counts when either arm aliases, because the chosen arm supplies its value |
| RV-404 | 🟡 minor | `own/statements.rs` (the intrinsic route) | **Accepted** (report-only): the route covers every compiler-known intrinsic, not only `len`; the wider reading fixes the same "intrinsic the C build cannot resolve" class and the emission audit classified the `slice` spelling as intended |
| RV-405 | 🟡 minor | `own/lower.rs` (the purity doc) | **Fixed** directly: the duplicate summary folded into the existing block |
| SA coverage gap | 🟡 minor | the struct-literal arm had no sanitizer probe | **Fixed**: the `conddiscardstruct` case covers the fresh, place, nested, array-field, and conditional-field spellings |
| EA-402 (M1) | 🟡 minor | nested value-position conditionals inside a discarded expression | **Accepted**: narrowing the depth flag would reintroduce leaks for conditionals nested in a discarded expression; the extra materialization is memory-clean and the escape spellings are rejected by the checker |
| RV-406 | 🟡 minor | the `field_aliases_owned_storage` doc did not name conditional fields | **Fixed** in the close-out (doc touch) |

### Fix rounds and confirmation

The first fix round (`b8aa766`) closed the three filed defects; the review round found RV-401/SA-401
(the aliasing struct arm), fixed by the gate (`cea27ac`). The scoped re-review found RV-402/EA-401 (the
nested spelling), fixed by the recursive predicate (`5481108`). The first confirmation found RV-403
(the conditional spelling), fixed by the conditional arm (`deb6cef`) and pinned (`04f6ca4`). The final
confirmation returned **PASS** on all five items (the conditional reproducer, the owning-capable
field-kind sweep, the full reproducer set, the test integrity, and the no-re-regression checks) with
no findings.

**Verification after the fix rounds**: spec 55/55 plain and under `RDC_ASAN=1`; impl 21/21;
`make check` exit 0 (64 binaries, 1874 tests).

### Pre-existing findings (filed as DEF-270 – DEF-272)

| Row | Severity | Shape |
|-----|----------|-------|
| DEF-270 | 🟡 minor | A discarded struct-literal arm mixing a fresh and a place-derived owning field keeps the literal inline, so the fresh field leaks (base == head, equal size) |
| DEF-271 | 🟡 minor | A discarded conditional whose fresh arm is a projection over an ownerless fresh receiver (`c ? mk().v : t;`) reads the member and leaks the receiver's storage (base == head) |
| DEF-272 | 🟠 major | A conditional of fixed-array call arms (and a literal with a fixed-array owning field) is accepted and emits C that fails the strict build (base == head, byte-identical) |

## Phase 5 — Emission spellings

**Baseline**: `4cfe13ebf04c44ee28a7eeffa78a729b18d41ac3` (Phase 4's final commit tree at `926b3ad`). **Lenses**:
correctness, emission-integrity, semantics, all dispatched on the phase diff in parallel; the scoped
re-review covered the fix diff.

### Verdicts

| Lens | Result | Evidence |
|------|--------|----------|
| correctness | 1 critical (the pin order), 1 pre-existing minor | RV-001: the pin ran after both operands were lowered, so a nested update in the right operand reordered the left read (`i + h(i++)` returned 1 where the strict left-to-right rule gives 0); RV-002: a fixed-array projection of a conditional root still fails the strict build |
| memory-safety | 1 critical (a newly reachable dangling view), 3 pre-existing | the widened owned arm made a local-backed conditional return build and run — heap-use-after-free on `return (c ? b.v : a)` with a local `b`; the view-arm variant, the out-slot store, and the conditional fixed-array root are pre-existing |
| emission-integrity | 1 critical (the same order reversal on a base-green program), 1 minor, pre-existing rows | EIA-5.1: `readg() + h(gv++)` returned 101 where the spec gives 100 (base green — a phase regression); EIA-5.2: the call-argument form; EIA-5.3: the discarded `j++ + 1;` changes spelling (intended; the plan wording was narrower); the corpus is otherwise byte-identical and every unit builds |

### Findings and rulings

| Finding | Severity | Area | Ruling |
|---------|----------|------|--------|
| RV-001 = EIA-5.1 = EIA-5.2 | 🔴 critical | `own/lower.rs` (the Binary arm), `own/call.rs` (the argument loop) | **Fixed**: the pin now runs at each operand's and each argument's own lowering position (source order), so a nested update in a later operand cannot reorder an earlier read; the call loop resolves the parameter modes before lowering and pins each non-`take` argument in place |
| The dangling conditional return (memory) | 🔴 critical | `own/place.rs` (`view_provenance`) | **Fixed**: a conditional whose arms disagree keeps a local-backed arm's provenance, so the local-backed conditional return reports `E10035` instead of widening and dangling; the same rule now rejects the pre-existing view-arm spelling |
| EIA-5.3 | 🟡 minor | the update rule's breadth | **Accepted** (record-only): any non-short-circuit binary whose operands contain an update pins both, and any call whose arguments contain one pins each non-`take` argument, because the left-to-right rule orders the whole operation; the design wording in `03-05` was corrected |
| RV-002, the `for`-iterable projection, the assignment-target update spellings, the literal field order, the out-slot store view | pre-existing | filed as `DEF-273` – `DEF-276` | rows with base == head evidence |

### Fix round and verification

The fix round corrected the pin position in both constructs and added the conditional provenance rule;
two regression spec cases pin the nested-update order (`postfixnested`) and the two local-escape
rejects (`condreturnlocal`, `condreturnview`). **Verification after the fix**: spec 61/61 plain and
under `RDC_ASAN=1`; impl 27/27; `make check` exit 0 (64 binaries, 1886 tests).

### Re-review (scoped to the fix diff)

| Lens | Result | Evidence |
|------|--------|----------|
| correctness | 1 minor (report-only), 2 pre-existing edges | RV-001 verified fixed: the reproducers and composites match the strict left-to-right values, and the emitted pins are in source order; the two escape rejects verified; the provenance blast radius checked (mixed conditional arguments and `let` spellings stay accepted, the suites' parameter cases stay accepted); the element-store over-rejection observed and judged consistent with the pre-existing named-spelling rule; the dot-receiver and loop-condition pin edges re-stated |
| memory-safety | no new findings; the critical verified fixed | the four dangling-return programs now report `E10035`; the safe `let` store stays accepted and ASan-clean; the update-pin and fixed-array matrices are clean, including loops, early exits, `take` lists, and generic instances; the out-slot store remains a pre-existing row with corrected evidence |
| emission-integrity | no critical or major; 1 minor | the order probes and the full probe family now emit source-order pins and the spec values; the corpus is byte-identical or an intended shape; the two new `E10035` rejections are the intended escape class; one already-rejected program now reports `E10035` beside its `E10030` |

| Finding | Severity | Ruling |
|---------|----------|--------|
| The element-store over-rejection (`xs[1] = c ? b.v : a;` reports `E10033`; the named spelling already reports it) | 🟡 minor | **Accepted** (report-only): the pre-existing element-store rule over-rejects a local-backed view, and the phase's provenance rule reaches one more spelling consistently; the reviewer judged it no lifetime-safety regression. Filed as `DEF-280` |
| The duplicate escape report (`E10035` beside `E10030` at one returned mixed conditional) | 🟡 minor | **Accepted** (report-only): both diagnostics are true, the program stays rejected, and the oracle test passes; one-report-per-site needs a liveness-pass suppression, deferred. Filed as `DEF-279` |
| The update-pin edges (the dot receiver and the loop condition) | pre-existing | Filed as `DEF-277` (strict-build failures, base == head) |
| The non-duplicable argument order (`f(g(), i++)` builds and evaluates in the wrong order) | pre-existing | Filed as `DEF-278` (base == head) |

## Close-out

**Final verification** (2026-10-13):

- `make check` — exit 0; 64 test binaries, 1886 tests; the spec check OK (27 files, 110 compile-fail tests).
- `make verify` — exit 0, **all legs passed**: the hosted-amiga cross-toolchain corpus (every portable program matches; interrupt-counter and graphics built), the FS-UAE-only boots (interrupt-counter and graphics output match), and the six runtime-fault programs under `vamos`.
- The focused suites at the final tree: spec 61/61 plain and under `RDC_ASAN=1`; impl 27/27.
- The strict-build sweep (the emission re-audit): every head-accepted suite and corpus unit compiles under `cc -std=c99 -O0 -g -Wall -Wextra -Werror`, plus the hosted-amiga and freestanding profile builds of the changed shapes.

**Roadmap sync**: the 14 batch-H rows (`DEF-176` – `DEF-188` and `DEF-256`) are Done with per-row
evidence; six earlier-phase rows (`DEF-176`, `DEF-178` – `DEF-182`) still read `Executing` and are
corrected here. The review-filed rows `DEF-257` – `DEF-280` remain in the backlog with their
base == head evidence.

**Plan close**: `00-index.md` marks the plan Complete; the execution plan ends at 33/33 tasks.
