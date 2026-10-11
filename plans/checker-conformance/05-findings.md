# Findings: Checker Conformance

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)
> **Status**: in progress

## Review Protocol

Each phase receives one review round from the lenses listed in its execution-plan header,
dispatched per the project policy (AGENTS.md). A finding is recorded as
`<lens>-NNN | severity | area | what | reproduction | expected vs observed | ruling`; a
critical or major finding gets an authorized ruling in the active mode and a fix round, then
one scoped re-review; direct verification closes further fixes in the same phase. The phase's
verdict line records `no findings` or the final state of every finding.

## Phase 1: Initializer guard operands and typedef collectors

**Baseline**: `477f27dd36dcbde5f40e6dc267ce1864388dbe11` (the `8170199` tree).
**Lenses**: correctness, memory-safety, emission-integrity

### Deliverables

| # | Shape | Result |
| - | ----- | ------ |
| DEF-136 | The initializer materialization stores an impure dividend (`Div`/`Rem`) and an impure slice index base into an init local before their guard runs; the fixed-array base keeps its spelling (C cannot assign an array to a temporary) and the existing single-call shape | Fixed; spec `initdividend`/`initbase`; impl pins the one-call stores |
| DEF-142 | The body collector records every value's own substituted type, so an inline `slice(...)` builder's view typedef exists | Fixed; spec `inlinearg`; impl pins the typedef and the compound literal |
| DEF-215 | The collector walks the lifted-lambda specializations with their substitutions (mirroring the emission's specialization guard) | Fixed; spec `lambdaspec`; impl pins the typedef and the lifted body's `sizeof` use |
| DEF-216 | The monomorphization records a written type argument's concrete types, so a struct instance referenced only as a slice element is declared — and, in the emitter, before the slice typedef that names it | Fixed; spec `structelement`; impl pins the declaration order |

### Red evidence

At the phase baseline the five row cases failed exactly as filed: `initdividend` and
`initbase` exited 22 (the guard and the operation each ran the impure call), `inlinearg`
failed the strict build with `'ClSlice_String' undeclared`, `lambdaspec` with
`'ClSlice_i32' undeclared` (the specialization body), and `structelement` with the
undeclared `ClWrap__i32` its slice typedef names. The `initownbase` reference passed at
the baseline (the fixed-array base was already single-call and AddressSanitizer-clean),
as intended.

### Implementation notes

- **Shape gating.** The initializer additions fire only for an impure dividend and an
  impure non-fixed-array base; the needs-materialization predicate mirrors both so a
  lazily evaluated position (a conditional arm or a short-circuit right side) structures
  its guard exactly when the store happens. The collector additions only grow the typedef
  set; the mono recording only grows the instance list.
- **The fixed-array guard.** `store_impure_intrinsic_arg` already documents the reason a
  fixed array never stores into a temporary; the index base carries the same guard so the
  verified owning-index-base shape re-emits byte-identically.
- **The specialization walk** mirrors `lifted_emissions`: same enclosing lookup, same
  `lifted_needs_specialization` gate, same `instance_subst` — so collected types exist
  exactly where a specialization is emitted.
- **The struct-instance recording** sits in `walk_value` beside the value-type recording,
  so it applies to plain bodies, instances, and lifted specializations alike.

### Verification

Spec suite 6/6 plain and under `RDC_ASAN=1`; impl 6/6; `make check` 62 suites green (the
two new suites included). The phase review (`correctness` + `memory-safety` +
`emission-integrity`) is dispatched after the phase commit.

### Review and fix round

**Three lenses reported; no critical or major finding on the change.**

| Lens | Verdict | Evidence highlights |
| ---- | ------- | ------------------- |
| correctness | no findings above minor | the five row cases red exactly as filed and green at head; ~40 base/head probes (laziness kept in arms and short-circuit sides, the guard/effect order now matching the body twins, fixed-array and pure operands byte-identical, the specialization gate, three-profile typedefs); the reference green at both refs |
| memory-safety | **no findings** | 24 ASan probes clean on head (view bases release nothing, the dividend store adds no release, the lazy stores run only on their path, the owning fixed-array base releases once, the trap path); no leak, double free, or use-after-free attributable to the change |
| emission-integrity | **no findings** | 4476 base/head emission pairs: the only differing units are the six new cases plus one pre-existing inline `slice(...)`-base unit (an intended DEF-136-class fix); 2989 strict builds across host/freestanding/hosted-amiga with zero new failures; deterministic re-emission; declaration agreement (the instance typedef before the slice typedef) |

**Fix round (two minor items, fixed directly):**

| Finding | Severity | Area | Resolution |
| ------- | -------- | ---- | ---------- |
| RV-001 | 🟡 minor | `checker_conformance_impl_test.rs` carried two unused helpers (`dead_code` warnings) | Fixed: the unused proximity helper deleted, and `emit_twice` wired into the new `reemit` determinism pin (the changed initializer store re-emits byte-identically) |
| RV-002 | 🟡 minor | the initializer materialization's doc lists still said "an impure index, divisor, or cast operand" | Fixed: both lists now name the index base and the dividend |

**Filed from the reviews' pre-existing observations** (base-identical; outside the phase):

- **DEF-219** 🟠 major — an owned bind in a short-circuit right side is declared inside its `if` block but released outside it, so an accepted initializer emits `__cl_itmp1 undeclared` (the batch G Phase 1 memory audit, PRE-1).
- **DEF-220** 🟠 major — a generic function called only from a module initializer with an inferred type argument emits no instance (undefined symbol); the memory audit's PRE-2.
- **DEF-221** 🟡 minor — a fixed-array literal base with a run-time index in an initializer emits `{ ... }[pos]` (invalid C99); the emission audit's P-1 and the correctness review's P2.
- **DEF-222** 🟡 minor — a generic instance recorded from a type argument is not field-expanded, so a nested instance field stays undeclared (`sizeOf<Box<i32>>()` names `ClPair__i32`); the correctness review's P1.
- **DEF-223** 🟡 minor — the global-initializer `sizeof(Wrap<i32>[])` spelling still names an undeclared instance (the initializer twin of DEF-216); the emission audit's P-2.

### Verification (updated)

Spec suite 6/6 plain and under `RDC_ASAN=1`; impl 7/7 (the determinism pin added); `make check` 62 suites green.

## Phase 2: Generic instance conformance

**Baseline**: `5a378fb` (the Phase 1 close-out tree).
**Lenses**: correctness, semantics, emission-integrity

### Deliverables

| # | Shape | Result |
| - | ----- | ------ |
| DEF-145 | `check_for_in` rejects a null-shaped iterable with `E10012` and records an opaque-iterable duty; the per-instance audit (`check_iterable_requirements`) substitutes each instance and reports `E10043` when the type is not an array or its element is not a scalar or an enum | Fixed; spec `iterscalar`/`iterenum`/`iterstruct`/`iternull`; impl pins the scalar and borrow loop shapes |
| DEF-146 | A value-form member read on a bare type parameter records an opaque projection; the audit (`check_opaque_projections`) resolves it through the substituted type and reports `E10212` with the substituted display when the field is missing, while a resolving instance keeps the read | Fixed; spec `memval` (rejected) and `memvalcall` (unchanged); impl pins `return x.foo` for the resolving instance |
| DEF-150 | `resolved_value_ty` resolves a conditional value when both arms name the same substituted type, so the structural `len` fold sees the real extent and still evaluates nothing | Fixed; spec `condfold`, `condfolddrop`; impl pins `return 7;`; the concrete twin `condfoldconcrete` is unchanged |

### Red evidence

At the phase baseline the seven row cases failed exactly as filed: `iterscalar`, `iterenum`,
`iterstruct`, and `iternull` were accepted (the first three emitted invalid C that failed the
strict build; the null one emitted an unsigned `< 0` comparison), and `condfold`,
`condfolddrop`, and `memval` were accepted with the wrong result (`0` for the folds; a member
access on an untyped value for `memval`, which after the amendment reports the instance's
member error). The five references (`iterok` 4, `iterenumelem` 2, `iterborrow` 4, `memvalcall`
one `E10212`, `condfoldconcrete` 7) passed at the baseline, as intended.

### Design notes

- **The DEF-146/DEF-150 conflict (the AR-5 amendment).** A declaration-site `E10212` for a
  value-form member read on a parameter rejects the designed late resolution: the probe
  `get(W { foo: 7 })` builds and runs at the baseline, and DEF-150's arms (`b.s`) reach the
  fold through the same path. The report therefore lands per instance, mirroring DEF-145's
  audit, with the substituted type in the message. The call form's declaration-site report is
  unchanged (a call needs a signature the declaration must know).
- **The pinned element rule (DEF-145).** The probe matrix: an array whose element is a scalar
  or an enum builds and runs (scalar, `usize`, enum-element, and borrow-scalar instances
  verified); non-array iterables (scalar, enum) and arrays with struct, handle, view, or array
  elements fail the strict build. The audit accepts exactly the measuring set and reports
  `E10043` once at the first offending instance; the per-instance `borrow` requirement
  (`E10040`) is not added because the element rule already rejects those elements.
- **The null shape** lands as a shared `types::is_null_shaped` predicate with a unit test; the
  codegen method delegates to it. Phase 4 reuses the predicate for the remaining positions.

### Verification

Spec suite 18/18 plain and under `RDC_ASAN=1`; impl 11/11; `make check` 62 suites green.

### Review and fix round

**Semantics and correctness reported; one critical and five major items; all fixed in-phase.**

| Finding | Severity | Resolution |
| ------- | -------- | ---------- |
| SR-001 | 🔴 critical | The element predicate accepted non-i32 scalar elements the emitter mis-binds (`sumf<f32[2]>` computed 3.0; float/enum borrow instances failed the strict build) | **Fixed:** the element binding is typed per instance through the element placeholder — `float v`/`ClS v` bind correctly and the float sum is 4.0; spec `iterf32` (exit 40), impl pins the `float v`/`ClS v` shapes |
| SR-002 = RV-002 | 🟠 major | The audit measured the innermost element, not the loop's one-step element, so multi-extent arrays passed and emitted invalid C | **Fixed:** the binding carries the substituted one-step element; spec `iterrow` (exit 2) |
| SR-003 = RV-004 | 🟠 major | Legal iterables (struct/handle/view elements, the borrow form) were rejected with `E10043`; `E10040` was skipped | **Fixed:** the typed binding accepts them (spec `iterstruct`, `iterborrowstruct`); a non-borrow non-duplicable element reports `E10040` (spec `iterstring`); only a non-array substitution reports `E10043` |
| RV-001 | 🟠 major | The projection audit falsely rejected resolving `raw` and enum instances (a working-program regression for `raw`) | **Fixed:** the audit mirrors the value-form dispatch (struct field, enum member, `raw`→struct, the structural `.len`); spec `memraw` (exit 7) and `memenum` (one `E10212`) |
| SR-004 | 🟠 major | `.len` bypassed field-wins, falsely rejecting a struct field named `len` (regression) | **Fixed:** `instance_field` is tried for every struct base first |
| SR-005 | 🟠 major | `.len` on `cstring`/`String` instances was accepted but not emittable | **Fixed:** the `.len` set is narrowed to the emittable kinds (fixed array, slice, `str`, `static str`); those instances report `E10212`; the concrete value-form gap is filed as DEF-224 |
| SR-006 | 🟡 minor | An undeclared enum member's report contradicted the registry | **Fixed:** a declared member resolves (mirroring the concrete acceptance), an undeclared one reports the truthful `E10212`. The declared-member acceptance emits invalid C because the value-form enum read is broken concretely too (base-identical); the class is filed as DEF-233 |
| RV-003 | 🟡 minor | Stale plan/ledger counts (the denominator is 38, not 34) | Fixed: the header, the counts, and the 03-02 §2 supersede notes |
| PE-001 | 🟠 pre-existing | The mixed-arm generic conditional `len` returns `0` (the concrete spelling is `E10200`) | Filed as DEF-225 |
| PE-002 + the correctness pre-existing items | 🟡 pre-existing | The concrete value-form `.len` reads, the nested opaque chain, the lifted-lambda array capture, and the generic `raw` unsafe bypass | Filed as DEF-224 (`.len` only), DEF-226–DEF-228; the concrete enum read is DEF-233 (corrected — the earlier note overclaimed DEF-224's scope) |

### Scoped re-reviews of the fix round (`8f0e5b8..d498a67`)

**Both re-reviews confirmed every filed finding fixed** (the raw and enum projections, the
multi-extent rows, the float sums, the struct/borrow instances, `E10040`, field-wins, the `.len`
narrowing, the borrow aliasing, the placeholder hygiene — no `#element` in any C, symbol, or
diagnostic) **and found one shared limit plus one acceptance question, both filed:**

| Finding | Severity | Outcome |
| ------- | -------- | ------- |
| SR-007 / RV-005 (the member half) | 🟠 major | Filed as **DEF-229**: the placeholder reaches the binding, not the element's uses; the fix boundary is the type-stage hoist |
| RV-005 (the capture half) | 🟠 major | Filed as **DEF-230** (float/row variants pre-existing; struct newly reachable) |
| SR-008 | 🟠 major | Filed as **DEF-231**, with the emission audit's null-argument `unknownQ` case |
| SR-009 | 🟠 major | Filed as **DEF-232** (no bounds guard on an element index) |
| SR-010 / RV-006 | 🟠 major | Filed as **DEF-233** (the value-form enum read; the concrete twin is identically broken at the phase baseline) |
| RV-007 | 🟡 minor | The round's cases: six are red at the fix base, and `memenum` is a control that reported truthfully before the round; the declared-member acceptance carries no runnable pin because its emission is broken (DEF-233) |
| RV-008 | 🟡 minor | The plan header's stamp is refreshed with this reconciliation |

The net interaction: the fix round widened the accepted set for struct/row elements, which
exposes the pre-existing element-use gaps for those types (the scalar variants are
byte-identical base↔head). Counting and binding-only bodies — the common case — build and run
correctly; the filed rows carry the hoist boundary for the use sites.

The emission audit of `5a378fb..8f0e5b8` reported **no findings**: 4617 emissions per compiler
over three profiles, the eight differing units being exactly the intended shapes, zero new
strict-build failures, and every new rejection removing base-invalid C. Its pre-existing table
was reconciled: P-1/P-4/P-5 are closed by the fix round, P-2/P-6 are DEF-226/DEF-225, P-3 is
DEF-231, P-6b is DEF-234, and P-7 belongs to DEF-224's class.

### Verification (after the fix round and the re-reviews)

Spec suite 25/25 plain and under `RDC_ASAN=1`; impl 13/13; `make check` 62 suites green.

## Phase 3: Call, member, and `cstring` surfaces

**Baseline**: `68d6428` (the Phase 2 close-out tree).
**Lenses**: correctness, semantics, emission-integrity

### Deliverables

| # | Shape | Result |
| - | ----- | ------ |
| DEF-144 | A view, slice, `String`, or `raw` argument at a `cstring` parameter reports the conversion: the parameter is contracted and the conversion table exhaustive, so the argument reports `E10200` with the explicit `toCString` route (the phase's crossing experiment was reverted by the review) | Fixed; spec `cstringview`/`cstringslicearg`/`cstringowned`/`cstringraw` (one `E10200` each); the literal stays unwrapped |
| DEF-147/DEF-213 | The written count is judged against the declared parameters: the dot-call candidate path against the remaining list (the receiver is not written) and the direct-call path against the whole list, under the ruled `E10308`; the generic and `ffi` direct-call twins join the same rule; a zero-parameter free function is not a dot-call candidate (Ch 04 §8's "must accept the receiver as its first parameter") | Fixed; spec `dotarity`, `directarity`, `genericarity`, `ffiarity`, `memberzero`; the registry row, its compile-fail golden, and the harness inventory line land under the AR-2 ruling |
| DEF-148 | Every receiver whose surface resolves no field and no matching candidate reports `E10212` once (the frozen "zero matches is an ordinary member error"), instead of emitting an implicit call; a non-callable field of the name keeps its own path | Fixed; spec `memberunknown`, `memberraw`, `memberzero`; `memberfield` pins the field-wins rule |
| DEF-154 | `toCString(s: str)` returns the view's pointer (`s.ptr`), and the store conversion spells the `char *` cast the declaration expects; the `String`→`str` widening keeps owned callers working | Fixed; spec `cstringviewconv`, `cstringownedconv`; impl pins the body (`return (char *)s.ptr;`), the widened parameter (`char * cl_std_text_toCString(ClStr s)`), and the call-site widening (`cl_string_view(owned)`) |

### Red evidence

At the phase baseline the ten row cases failed exactly as filed: the two view crossings
(`cstringview`, `cstringslicearg`) passed the view struct where `char *` is declared and failed
the strict build (`cstringviewconv` was rejected at its `toCString` call); the four arity cases were accepted and
failed the strict build on the prototype; the three member cases emitted implicit calls; and
`toCString(view)` reported the old signature's `E10200`. The five references (`cstringliteral`,
`cstringowned`, `arityok`, `memberfield`, `cstringownedconv`) were green, as intended.

### Design notes

- **The DEF-144 judgment (restored in the fix round).** The phase first crossed a view at a
  `cstring` parameter as its pointer, reading Ch 12 CN-11/§10 as accepting views there. The
  semantics review showed the reading is wrong: a `cstring` parameter is contracted (an
  uncontracted parameter is a mode-less handle or aggregate), CN-9 checks the declared types,
  and the conversion table is exhaustive — only the `static str` literal converts. The design's
  original judgment therefore stands: the view reports `E10200` with the explicit `toCString`
  route, and the emitter crossing was reverted with it. The `String` handle and a `raw` or
  fixed-array argument report the same conversion (the handle is DEF-235, closed by the fix).
- **The `toCString` body and the untyped `.ptr` member.** The view's `.ptr` member carries no
  checked type (the type stage leaves it unknown), so the store conversion recognizes the read
  by shape — a member value or a place's last field projection named `ptr` — and spells the
  `char *` cast. The untyped members themselves (a `.ptr` flowing into a scalar is accepted and
  emits invalid C) are filed as DEF-238.
- **The arity rule's scope.** The named direct call, the generic direct call, and the `ffi`
  direct call report `E10308`; a call through a func-typed value passes no name and keeps its
  path (filed as DEF-236). The dot-call paths count the receiver as the first declared
  parameter.
- **The zero-parameter candidate rule.** `dot_call` requires a non-empty parameter list from a
  free-function candidate, so a zero-parameter function falls through to the intrinsic lookup
  and otherwise reports the unknown member instead of passing the receiver to a no-argument C
  function.
- **The uncontracted-extern rules (CN-10/CN-11) are implemented** (`own/call_check.rs`); a
  `cstring`/`i32` declaration has no uncontracted parameter, so a plain `Open` call needs no
  `unsafe` (the stdlib calls it without one). The earlier DEF-237 filing was withdrawn with
  this correction.

### Verification

Spec suite 40/40 plain and under `RDC_ASAN=1`; impl 18/18; `make check` 62 suites green
(including the `E10308` golden and the whole-corpus sweep after the stdlib widening).

### Review and fix round

**All three lenses reported.** The emission audit: **no findings** (1634 units × 3 profiles; the
deltas exactly the intended `toCString` shape, the widened call sites, and the new cases; zero
new strict-build failures; determinism and declaration agreement hold). The correctness lens:
five minors. The semantics lens: one major, one minor. All fixed in-phase:

| Finding | Severity | Resolution |
| ------- | -------- | ---------- |
| SR-001 | 🟠 major | The `char *` crossing contradicted the conversion table: a `cstring` parameter is contracted, so CN-9 and the exhaustive table govern. **Reverted to the `E10200` rejection**: the emitter crossing is removed; the ffi argument path reports for a view, slice, handle, or `raw` argument (the literal and a `cstring` value stay accepted); the spec cases pin the rejection, with `cstringraw` added |
| RV-001 | 🟡 minor | The ffi parameter types resolve once per declaration (cached in the checker), so a rejected parameter type reports once instead of once per call site |
| SR-002 = RV-002 | 🟡 minor | The inferred-generic inference reports (`E10301`/`E10305`) are suppressed when the written count mismatches: the arity report is the report |
| RV-003 | 🟡 minor | The design's acceptance restated: the references' own emission is unchanged apart from the intended `toCString` delta |
| RV-004 | 🟡 minor | The red evidence corrected (two view crossings, not three) |
| RV-005 | 🟡 minor | The design note cites DEF-235 |
| DEF-235 | correction | A `String` at a contracted `cstring` parameter is `E10200` (TY-13), not `E10101`; the SR-001 fix closes it |
| DEF-237 | withdrawal | The CN-10/CN-11 checks exist (`own/call_check.rs`); `Open(path: cstring, mode: i32)` has no uncontracted parameter, so no `unsafe` is due (the stdlib calls it without one). The real gap was the conversion at contracted ffi parameters — SR-001's family, fixed |

**New filings from the reviews:** DEF-239 (the `std.mem` `Array` dot-call arm bypasses the
zero-parameter candidate rule), DEF-240 (a module-qualified direct call bypasses the arity
rule), DEF-241 (a raw receiver's callable field emits `p.foo(1)` where C wants `p->foo(1)`) —
all pre-existing, base-identical.

**Emission-audit pre-existing classes:** P1/P2 belong to DEF-238 (the untyped `.ptr`, including
conditional arms); P3 is a hosted-amiga fixture-name conflict of the `Open` test programs (the
suites are host-profile, and the conflict reproduces identically on the base compiler); P4 is
the suites' appended C stubs.

### Scoped re-reviews of the fix round (`7e32f39..88a818e`)

**Both re-reviews confirmed the three fixes behaviorally** (the four rejections, the
one-report cache, the suppressed cascade, the reverted emitter, the byte-identical references)
**and added five minors, all fixed in-phase, plus three pre-existing classes filed:**

| Finding | Severity | Resolution |
| ------- | -------- | ---------- |
| RV-006 = SR-003 | 🟡 minor | Five stale record locations and the design's §1 sentence updated to the reverted state (this reconciliation) |
| RV-007 = SR-005 | 🟡 minor | Two pins added: the inferred-generic arity (one `E10308`, no `E10301`) and the ffi declaration's one-report cache (one `E10226` for two call sites) |
| SR-004 | 🟡 minor | The qualified spelling (`probe.Open(view, 0)`) bypasses the ffi checks like the arity: DEF-240's fix direction extended to cover it |
| PE-1 | 🟠 pre-existing | An ffi result's declared type is discarded (the sema types it unknown), so a mismatched result passes conversions: filed as DEF-243 |
| PE-2 | 🟠 pre-existing | A parameter-typed argument's conversion is deferred to an instance audit that does not exist (`call<str>` and the ordinary twin emit invalid C): filed as DEF-242 |
| PE-3 | 🟡 pre-existing | A never-called ffi declaration's types are never validated: filed as DEF-244 |
| PE-4/PE-5 | note | The untyped `.ptr` is DEF-238; the null-shaped local's `E10010`-to-`E10012` move is Phase 4's DEF-151 |

### Verification (after the fix round and the re-reviews)

Spec suite 43/43 plain; impl 17/17; `make check` 62 suites green.

## Phase 4: Null-shaped values

**Baseline**: `5a3727c` (the Phase 3 close-out tree).
**Lenses**: correctness, semantics

### Deliverables

| # | Shape | Result |
| - | ----- | ------ |
| DEF-149 | A store of a concrete value into the local inferred from `null` reports `E10012`: the null shape accepts only the null shape (or `null`) | Fixed; spec `nullassignview`; the copy between null-shaped locals stays accepted (`nullcopy`) |
| DEF-151 | A null-shaped value used where a concrete type is required (a return, an argument) reports `E10012`: the narrowing that silences the possible-null rule does not give the value a type | Fixed; spec `nullreturn`, `nullarg`; the comparisons (`nullcompare`) and the nullable declarations (`nullnullable`) stay accepted |
| DEF-145 (the iterable sub-case) | The null-shaped iterable already reports `E10012` at the declaration (Phase 2's `iternull`) | Covered; no new code |

### Design notes

- **The sites live in the own stage's shared gate.** The design named `compiler/src/sema/`,
  but the nullability gate is `own/expr.rs::check_value_into`, which every position already
  funnels through (the store, the `let`, the return, the arguments). The extension is two
  branches there: a null-SHAPED value at a concrete expected reports `E10012` (ahead of the
  possible-null rule, so a narrowing cannot answer for a value with no type), and a concrete
  value stored into a null-SHAPED expected reports `E10012` (the copy between two null-shaped
  locals returns before it). The registry message and help are the shared text, reported
  through one small helper.
- **The predicate** (`types::is_null_shaped`) landed in Phase 2 with its unit tests; Phase 4
  reuses it unchanged.
- **The accepted positions are unchanged and pinned by emission** (the null comparisons, the
  null-shaped copy, the nullable declarations).

### Red evidence

At the phase baseline the store and the return were accepted and failed the strict build
(`void * = ClStr`; `void *` returned where `i32` is declared); the argument case was already
rejected at the baseline with `E10010` (its change is the code: `E10010` → `E10012`), and the
three references were green. The first implementation run left `nullarg` failing on its own
program (`take` is a reserved keyword) — the case was renamed to `consume`, not an
implementation change.

### Verification

Spec suite 49/49 plain and under `RDC_ASAN=1`; impl 20/20; `make check` 62 suites green (the
whole corpus and stdlib are unaffected by the new rejections).

### Review and fix round

**Both lenses reported; no phase findings.** The correctness lens: no critical or major; one
minor records item (the red evidence described the argument case as accepted — it was already
rejected at the baseline with `E10010`). The semantics lens: **no findings** — the four sites
enforce the recorded reading, the diagnostics match the registry, one report per site, and the
accepted positions are byte-identical. It confirmed the `E10012`-over-`E10010` precedence and
the DEF-149 store reading as conformant under the register's delegation, with no user ruling
needed (the frozen text does not define the null shape itself; a future reserved spec edit could
pin it).

| Item | Resolution |
| ---- | ---------- |
| RV-001 | The red evidence corrected (the store and the return were accepted-invalid; the argument row is an `E10010` → `E10012` code change); the plan's `Last Updated` refreshed |
| SR-001…SR-006 (pre-existing) | Filed: DEF-245 (the shape at a nullable non-pointer destination emits `void *` — major; every profile for views, hosted-amiga for handles), DEF-246 (the literal `null` at a non-nullable aggregate position), DEF-247 (a member read/store through the null-shaped head), DEF-248 (the shape at arithmetic/comparison operands), DEF-249 (the shape at a condition — the missing `E10218`); SR-006 folds into DEF-242's note (the parameter-typed return and declaration positions) |

The gate's comment about an "instance audit" was corrected to state what the check does (a
parameter-typed expected cannot be judged at the declaration), closing SR-006's comment half.

## Phase 5: Diagnostic hygiene

**Baseline**: _(recorded at phase start)_
**Lenses**: correctness, semantics

_(findings recorded after the review round)_

## Phase 6: Close-out

**Baseline**: _(recorded at phase start)_
**Lenses**: none new

_(the full-verification evidence recorded at close-out)_
